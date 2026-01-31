use crate::*;
use crate::errors::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(target_username: String)]
pub struct Vote<'info> {
    #[account(mut)]
    pub voter: Signer<'info>,

    /// CHECK: The admin pubkey will be passed here
    pub admin: UncheckedAccount<'info>,

    // Config account for validation
    #[account(
        seeds = [CONFIG, admin.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    // Voter's profile (to check stake amount and update vote count)
    #[account(
        mut,
        seeds = [USERPROFILE, voter.key().as_ref()],
        bump,
        constraint = voter_profile.owner == voter.key() @ GovernanceError::UnauthorizedUser
    )]
    pub voter_profile: Account<'info, UserProfile>,

    // Target user registry (to verify username exists)
    #[account(
        seeds = [USER_REGISTRY, target_username.as_bytes()],
        bump,
        constraint = target_user_registry.claimed @ GovernanceError::UsernameNotFound
    )]
    pub target_user_registry: Account<'info, UsernameRegistry>,

    // Target user profile (to update reputation)
    #[account(
        mut,
        seeds = [USERPROFILE, target_user_registry.owner.as_ref()],
        bump,
        constraint = target_user_profile.owner == target_user_registry.owner @ GovernanceError::ProfileMismatch
    )]
    pub target_user_profile: Account<'info, UserProfile>,

    // Vote cooldown tracking
    #[account(
        init_if_needed,
        payer = voter,
        space = ANCHOR_DISCRIMINATOR + VoteCooldown::INIT_SPACE,
        seeds = [VOTE_COOLDOWN, voter.key().as_ref()],
        bump
    )]
    pub vote_cooldown: Account<'info, VoteCooldown>,

    // Vote record for tracking
    #[account(
        init,
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
        // Check if voter can downvote
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
        // Basic validations
        require!(!self.config.is_paused, GovernanceError::SystemPaused);
        require!(
            self.voter.key() != self.target_user_profile.owner,
            GovernanceError::CannotVoteForSelf
        );

        // Check minimum stake requirement
        require!(
            self.voter_profile.stake_amount >= self.config.minimum_stake,
            GovernanceError::InsufficientStake
        );

        // Check cooldown
        let current_time = Clock::get()?.unix_timestamp;
        let cooldown_hours = self.voter_profile.role_level.cooldown_hours();
        
        if cooldown_hours > 0 {
            let cooldown_seconds = cooldown_hours * 3600;
            require!(
                current_time >= self.vote_cooldown.last_vote_timestamp + cooldown_seconds as i64,
                GovernanceError::VoteCooldownActive
            );
        }

        // Calculate vote weight based on role
        let initial_vote_weight = self.voter_profile.role_level.vote_weight() as i64;
        let vote_weight = initial_vote_weight * self.config.vote_power as i64;
        let reputation_change = match vote_type {
            VoteType::Upvote => vote_weight,
            VoteType::Downvote => -vote_weight,
        };

        // Update target user reputation and stats
        let target_profile = &mut self.target_user_profile;
        target_profile.reputation_points = target_profile.reputation_points
            .checked_add(reputation_change)
            .ok_or(GovernanceError::MathOverflow)?;

        match vote_type {
            VoteType::Upvote => {
                target_profile.upvotes_received = target_profile.upvotes_received
                    .checked_add(1)
                    .ok_or(GovernanceError::MathOverflow)?;
            }
            VoteType::Downvote => {
                target_profile.downvotes_received = target_profile.downvotes_received
                    .checked_add(1)
                    .ok_or(GovernanceError::MathOverflow)?;
            }
        }

        // Update target user role based on new reputation
        target_profile.role_level = MemberRanks::from_reputation(target_profile.reputation_points);

        // Update voter stats
        let voter_profile = &mut self.voter_profile;
        voter_profile.total_votes_cast = voter_profile.total_votes_cast
            .checked_add(1)
            .ok_or(GovernanceError::MathOverflow)?;
        voter_profile.last_vote_timestamp = current_time;

        // Update cooldown
        self.vote_cooldown.last_vote_timestamp = current_time;
        if self.vote_cooldown.voter == Pubkey::default() {
            self.vote_cooldown.voter = self.voter.key();
            self.vote_cooldown.bump = bumps.vote_cooldown;
        }

        // Create vote record
        self.vote_record.set_inner(VoteRecord {
            voter: self.voter.key(),
            target_username: target_username.clone(),
            target_owner: self.target_user_profile.owner,
            vote_type,
            vote_weight: vote_weight as u8,
            timestamp: current_time,
            bump: bumps.vote_record,
        });

        Ok(())
    }
}