use anchor_lang::prelude::*;

use crate::{constants::*, errors::*, state::*};

// Vote Instruction
//
// VULNERABILITY SUMMARY:
// - No minimum stake requirement check
// - No cooldown enforcement (created but never checked)
// - Self-voting allowed
// - No downvote role restriction
// - Cannot change votes (init instead of init_if_needed)
// - No reputation floor (unlimited downvoting)
// - Unchecked arithmetic operations
// - vote_weight truncated to u8 (loses precision)

#[derive(Accounts)]
#[instruction(target_username: String)]
pub struct Vote<'info> {
    #[account(mut)]
    pub voter: Signer<'info>,

    /// CHECK: Used only for PDA derivation
    pub admin: UncheckedAccount<'info>,

    #[account(
        seeds = [CONFIG, admin.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    // VULNERABILITY: No owner constraint on voter_profile
    #[account(
        mut,
        seeds = [USERPROFILE, voter.key().as_ref()],
        bump,
    )]
    pub voter_profile: Account<'info, UserProfile>,

    #[account(
        seeds = [USER_REGISTRY, target_username.as_bytes()],
        bump,
        constraint = target_user_registry.claimed @ GovernanceError::UsernameNotFound
    )]
    pub target_user_registry: Account<'info, UsernameRegistry>,

    #[account(
        mut,
        seeds = [USERPROFILE, target_user_registry.owner.as_ref()],
        bump,
        constraint = target_user_profile.owner == target_user_registry.owner @ GovernanceError::ProfileMismatch
    )]
    pub target_user_profile: Account<'info, UserProfile>,

    // VULNERABILITY: Cooldown account created but never checked
    // The account exists but timestamp validation is missing in cast_vote
    #[account(
        init_if_needed,
        payer = voter,
        space = ANCHOR_DISCRIMINATOR + VoteCooldown::INIT_SPACE,
        seeds = [VOTE_COOLDOWN, voter.key().as_ref()],
        bump
    )]
    pub vote_cooldown: Account<'info, VoteCooldown>,

    // VULNERABILITY: Uses 'init' instead of 'init_if_needed'
    // Once a user votes, they cannot change their vote
    // Subsequent votes will fail with "already initialized" error
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
        // VULNERABILITY: No downvote role restriction
        // Member rank can downvote (should require Bronze+)
        // Allows new users to immediately grief others

        self.cast_vote(target_username, VoteType::Downvote, bumps)
    }

    fn cast_vote(
        &mut self,
        target_username: String,
        vote_type: VoteType,
        bumps: VoteBumps,
    ) -> Result<()> {
        // VULNERABILITY 1: No system pause check
        // Missing: require!(!self.config.is_paused, ...)
        // System cannot be halted during emergencies

        // VULNERABILITY 2: No self-vote prevention
        // Missing: require!(self.voter.key() != self.target_user_profile.owner, ...)
        // Users can vote for themselves to inflate reputation

        // VULNERABILITY 3: No minimum stake check
        // Missing: require!(self.voter_profile.stake_amount >= self.config.minimum_stake, ...)
        // Users with zero stake can vote (sybil attack)

        // VULNERABILITY 4: No cooldown enforcement
        // Cooldown account exists and last_vote_timestamp is updated
        // But no validation happens here, allowing spam voting

        let current_time = Clock::get()?.unix_timestamp;

        // VULNERABILITY 5: No vote change handling
        // Since we use 'init' on vote_record, this code path never executes
        // Users are locked into their first vote permanently

        // VULNERABILITY 6: vote_weight truncated to u8
        // Calculate as i64 but store as u8 in VoteRecord
        let initial_vote_weight = self.voter_profile.role_level.vote_weight() as i64;
        let vote_weight = initial_vote_weight * self.config.vote_power as i64;

        // Truncation happens here when casting to u8 for storage
        let vote_weight_u8 = vote_weight as u8;

        let reputation_change = match vote_type {
            VoteType::Upvote => vote_weight,
            VoteType::Downvote => -vote_weight,
        };

        // VULNERABILITY 7: No reputation floor
        // Missing: .max(REPUTATION_FLOOR)
        // Users can be downvoted to i64::MIN
        let target_profile = &mut self.target_user_profile;

        // VULNERABILITY 8: Unchecked arithmetic
        // Using direct addition instead of checked_add
        target_profile.reputation_points = target_profile.reputation_points + reputation_change;

        match vote_type {
            VoteType::Upvote => {
                target_profile.upvotes_received = target_profile.upvotes_received + 1;
            }
            VoteType::Downvote => {
                target_profile.downvotes_received = target_profile.downvotes_received + 1;
            }
        }

        target_profile.role_level = MemberRanks::from_reputation(target_profile.reputation_points);

        // VULNERABILITY 9: Unchecked arithmetic on voter stats
        let voter_profile = &mut self.voter_profile;
        voter_profile.total_votes_cast = voter_profile.total_votes_cast + 1;
        voter_profile.last_vote_timestamp = current_time;

        // Update cooldown tracker
        self.vote_cooldown.last_vote_timestamp = current_time;
        if self.vote_cooldown.voter == Pubkey::default() {
            self.vote_cooldown.voter = self.voter.key();
            self.vote_cooldown.bump = bumps.vote_cooldown;
        }

        // Store truncated vote_weight as u8
        self.vote_record.set_inner(VoteRecord {
            voter: self.voter.key(),
            target_username: target_username.clone(),
            target_owner: self.target_user_profile.owner,
            vote_type,
            vote_weight: vote_weight_u8,
            timestamp: current_time,
            bump: bumps.vote_record,
        });

        Ok(())
    }
}
