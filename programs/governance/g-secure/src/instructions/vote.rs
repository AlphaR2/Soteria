use anchor_lang::prelude::*;

use crate::{constants::*, errors::*, state::*};

// Vote Instruction
//
// Allows users to vote on other users' reputation
// Supports both upvotes and downvotes (role-dependent)
//
// SECURITY FEATURES:
// - Minimum stake requirement prevents sybil attacks
// - Role-based cooldowns prevent spam voting
// - Self-voting prevention
// - Downvote restriction (Bronze+ only)
// - Vote changing allowed (users can reverse their vote)
// - Reputation floor prevents grief attacks
// - Checked arithmetic prevents overflow/underflow

#[derive(Accounts)]
#[instruction(target_username: String)]
pub struct Vote<'info> {
    // Voter account
    // Must have sufficient stake and respect cooldown
    #[account(mut)]
    pub voter: Signer<'info>,

    // Admin pubkey for config derivation
    /// CHECK: Used only for PDA derivation
    pub admin: UncheckedAccount<'info>,

    // Config PDA
    // Seeds: ["config", admin]
    // SECURITY: Validates system state (pause status, minimum stake)
    #[account(
        seeds = [CONFIG, admin.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    // Voter's profile
    // Seeds: ["user_profile", voter]
    // SECURITY: Validates stake amount and updates vote count
    #[account(
        mut,
        seeds = [USERPROFILE, voter.key().as_ref()],
        bump,
        constraint = voter_profile.owner == voter.key() @ GovernanceError::UnauthorizedUser
    )]
    pub voter_profile: Account<'info, UserProfile>,

    // Target username registry
    // Seeds: ["user_registry", target_username]
    // SECURITY: Ensures username exists before allowing vote
    #[account(
        seeds = [USER_REGISTRY, target_username.as_bytes()],
        bump,
        constraint = target_user_registry.claimed @ GovernanceError::UsernameNotFound
    )]
    pub target_user_registry: Account<'info, UsernameRegistry>,

    // Target user's profile
    // Seeds: ["user_profile", target_owner]
    // SECURITY: Validates profile matches username registry
    // Prevents voting on mismatched accounts
    #[account(
        mut,
        seeds = [USERPROFILE, target_user_registry.owner.as_ref()],
        bump,
        constraint = target_user_profile.owner == target_user_registry.owner @ GovernanceError::ProfileMismatch
    )]
    pub target_user_profile: Account<'info, UserProfile>,

    // Vote cooldown tracker
    // Seeds: ["cooldown", voter]
    // SECURITY: Enforces time-based voting limits
    #[account(
        init_if_needed,
        payer = voter,
        space = ANCHOR_DISCRIMINATOR + VoteCooldown::INIT_SPACE,
        seeds = [VOTE_COOLDOWN, voter.key().as_ref()],
        bump
    )]
    pub vote_cooldown: Account<'info, VoteCooldown>,

    // Vote record
    // Seeds: ["vote_record", voter, target_username]
    // SECURITY: Tracks vote history and allows vote changes
    // Uses init_if_needed to allow users to change their votes
    #[account(
        init_if_needed,
        payer = voter,
        space = ANCHOR_DISCRIMINATOR + VoteRecord::INIT_SPACE,
        seeds = [VOTE_RECORD, voter.key().as_ref(), target_username.as_bytes()],
        bump
    )]
    pub vote_record: Account<'info, VoteRecord>,

    pub system_program: Program<'info, System>,
}

impl<'info> Vote<'info> {
    pub fn upvote_user(
        &mut self,
        target_username: String,
        bumps: VoteBumps,
    ) -> Result<()> {
        self.cast_vote(target_username, VoteType::Upvote, bumps)
    }

    pub fn downvote_user(
        &mut self,
        target_username: String,
        bumps: VoteBumps,
    ) -> Result<()> {
        // SECURITY: Downvote Restriction
        // Only Bronze rank and above can downvote
        // Prevents new users from immediate negative voting
        require!(
            self.voter_profile.role_level.can_downvote(),
            GovernanceError::CannotDownvote
        );

        self.cast_vote(target_username, VoteType::Downvote, bumps)
    }

    fn cast_vote(
        &mut self,
        target_username: String,
        vote_type: VoteType,
        bumps: VoteBumps,
    ) -> Result<()> {
        // SECURITY CHECKS

        // 1. System Pause Check
        // Prevents all voting when system is paused for maintenance or security
        require!(!self.config.is_paused, GovernanceError::SystemPaused);

        // 2. Self-Vote Prevention
        // Users cannot vote for themselves to prevent reputation inflation
        require!(
            self.voter.key() != self.target_user_profile.owner,
            GovernanceError::CannotVoteForSelf
        );

        // 3. Minimum Stake Requirement
        // SECURITY: Prevents sybil attacks by requiring economic commitment
        // Users must stake tokens before gaining voting rights
        require!(
            self.voter_profile.stake_amount >= self.config.minimum_stake,
            GovernanceError::InsufficientStake
        );

        // 4. Cooldown Check
        // SECURITY: Rate limiting to prevent spam voting
        // Different roles have different cooldown periods (0-24 hours)
        let current_time = Clock::get()?.unix_timestamp;
        let cooldown_hours = self.voter_profile.role_level.cooldown_hours();

        if cooldown_hours > 0 {
            let cooldown_seconds = cooldown_hours * 3600;
            require!(
                current_time >= self.vote_cooldown.last_vote_timestamp + cooldown_seconds as i64,
                GovernanceError::VoteCooldownActive
            );
        }

        // 5. Handle Vote Changes
        // SECURITY: If user previously voted, reverse the old vote first
        // This prevents double-counting reputation changes
        let vote_record = &self.vote_record;
        let is_vote_change = vote_record.voter != Pubkey::default();

        if is_vote_change {
            // Reverse previous vote's reputation impact
            let previous_reputation_change = match vote_record.vote_type {
                VoteType::Upvote => -vote_record.vote_weight,
                VoteType::Downvote => vote_record.vote_weight,
            };

            let target_profile = &mut self.target_user_profile;
            target_profile.reputation_points = target_profile
                .reputation_points
                .checked_add(previous_reputation_change)
                .ok_or(GovernanceError::MathOverflow)?;

            // Decrement previous vote stat
            match vote_record.vote_type {
                VoteType::Upvote => {
                    target_profile.upvotes_received = target_profile
                        .upvotes_received
                        .saturating_sub(1);
                }
                VoteType::Downvote => {
                    target_profile.downvotes_received = target_profile
                        .downvotes_received
                        .saturating_sub(1);
                }
            }
        }

        // 6. Calculate New Vote Weight
        // Vote weight = role_weight * vote_power
        // Example: Leader (3) * vote_power (5) = 15 reputation impact
        let initial_vote_weight = self.voter_profile.role_level.vote_weight() as i64;
        let vote_weight = initial_vote_weight * self.config.vote_power as i64;
        let reputation_change = match vote_type {
            VoteType::Upvote => vote_weight,
            VoteType::Downvote => -vote_weight,
        };

        // 7. Update Target User Reputation
        // SECURITY: Apply reputation floor to prevent grief attacks
        // Users cannot be downvoted below REPUTATION_FLOOR (-1000)
        let target_profile = &mut self.target_user_profile;
        let new_reputation = target_profile
            .reputation_points
            .checked_add(reputation_change)
            .ok_or(GovernanceError::MathOverflow)?
            .max(REPUTATION_FLOOR);

        target_profile.reputation_points = new_reputation;

        // 8. Update Vote Statistics
        // Increment upvote or downvote counter
        match vote_type {
            VoteType::Upvote => {
                target_profile.upvotes_received = target_profile
                    .upvotes_received
                    .checked_add(1)
                    .ok_or(GovernanceError::MathOverflow)?;
            }
            VoteType::Downvote => {
                target_profile.downvotes_received = target_profile
                    .downvotes_received
                    .checked_add(1)
                    .ok_or(GovernanceError::MathOverflow)?;
            }
        }

        // 9. Auto-Update Role Level
        // SECURITY: Role derived from reputation prevents manual manipulation
        target_profile.role_level = MemberRanks::from_reputation(target_profile.reputation_points);

        // 10. Update Voter Statistics
        // Track total votes cast only if this is a new vote (not a vote change)
        let voter_profile = &mut self.voter_profile;
        if !is_vote_change {
            voter_profile.total_votes_cast = voter_profile
                .total_votes_cast
                .checked_add(1)
                .ok_or(GovernanceError::MathOverflow)?;
        }
        voter_profile.last_vote_timestamp = current_time;

        // 11. Update Cooldown Tracker
        // Reset cooldown timer after successful vote
        self.vote_cooldown.last_vote_timestamp = current_time;
        if self.vote_cooldown.voter == Pubkey::default() {
            self.vote_cooldown.voter = self.voter.key();
            self.vote_cooldown.bump = bumps.vote_cooldown;
        }

        // 12. Record Vote
        // Store vote details for auditability and vote change tracking
        self.vote_record.set_inner(VoteRecord {
            voter: self.voter.key(),
            target_username: target_username.clone(),
            target_owner: self.target_user_profile.owner,
            vote_type,
            vote_weight,
            timestamp: current_time,
            bump: bumps.vote_record,
        });

        Ok(())
    }
}