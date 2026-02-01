use anchor_lang::prelude::*;

use crate::{constants::*, errors::*, state::*};

// Create Profile Instruction
//
// VULNERABILITY SUMMARY:
// - No username uniqueness enforcement (registry not checked)
// - Weak username validation (allows 1-char names)
// - Missing owner validation on registry account
// - No check for existing profile before init

#[derive(Accounts)]
#[instruction(username: String)]
pub struct CreateProfile<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    // VULNERABILITY: init_if_needed without claimed check
    // The registry exists but we never verify if it's claimed
    // Multiple users can register the same username
    #[account(
        init_if_needed,
        payer = user,
        space = ANCHOR_DISCRIMINATOR + UsernameRegistry::INIT_SPACE,
        seeds = [USER_REGISTRY, username.as_bytes()],
        bump
    )]
    pub user_registry: Account<'info, UsernameRegistry>,

    #[account(
        init,
        payer = user,
        space = ANCHOR_DISCRIMINATOR + UserProfile::INIT_SPACE,
        seeds = [USERPROFILE, user.key().as_ref()],
        bump
    )]
    pub user_profile: Account<'info, UserProfile>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreateProfile<'info> {
    pub fn create_profile(
        &mut self,
        username: String,
        bumps: CreateProfileBumps,
    ) -> Result<()> {
        // VULNERABILITY 1: Weak username validation
        // Allows single-character usernames (MIN_USERNAME_LENGTH = 1)
        // No check for special characters or reserved names
        require!(
            username.len() >= MIN_USERNAME_LENGTH && username.len() <= MAX_USERNAME_LENGTH,
            GovernanceError::InvalidUsername
        );

        // VULNERABILITY 2: No uniqueness enforcement
        // Registry account exists but claimed flag is never checked
        // Users can squat on the same username by racing to create profiles
        // The following code SHOULD check if user_registry.claimed is true
        // but this check is commented out or missing

        let user_registry = &mut self.user_registry;
        user_registry.claimed = true;
        user_registry.owner = self.user.key();
        user_registry.bump = bumps.user_registry;

        // VULNERABILITY 3: No owner validation
        // If registry exists with different owner, we overwrite it
        // Original owner loses their username registration

        self.user_profile.set_inner(UserProfile {
            username,
            owner: self.user.key(),
            reputation_points: 0,
            stake_amount: 0,
            role_level: MemberRanks::Member,
            upvotes_received: 0,
            downvotes_received: 0,
            total_votes_cast: 0,
            last_vote_timestamp: 0,
            created_at: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }
}
