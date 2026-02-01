use anchor_lang::prelude::*;

use crate::{constants::*, errors::*, state::*};

// Create Profile Instruction
//
// Creates a new user profile with a unique username
// Uses a two-account pattern for username uniqueness enforcement
//
// SECURITY FEATURES:
// - Username registry PDA prevents duplicate usernames
// - Username length validation (3-32 chars)
// - User can only have one profile (PDA derived from user pubkey)
// - All users start with zero reputation as Member role

#[derive(Accounts)]
#[instruction(username: String)]
pub struct CreateProfile<'info> {
    // User creating the profile
    // Must sign and pay for account creation
    #[account(mut)]
    pub user: Signer<'info>,

    // Username registry PDA
    // Seeds: ["user_registry", username]
    // SECURITY: init_if_needed allows checking if username is claimed
    // PDA derivation ensures one registry entry per unique username
    #[account(
        init_if_needed,
        payer = user,
        space = ANCHOR_DISCRIMINATOR + UsernameRegistry::INIT_SPACE,
        seeds = [USER_REGISTRY, username.as_bytes()],
        bump
    )]
    pub user_registry: Account<'info, UsernameRegistry>,

    // User profile PDA
    // Seeds: ["user_profile", user_pubkey]
    // SECURITY: One profile per user pubkey
    // Stores reputation, role, voting stats
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
        // SECURITY CHECKS

        // 1. Username Length Validation
        // Ensures username is between 3 and 32 characters
        // Prevents confusion attacks from single-char names
        // Prevents storage abuse from excessively long names
        require!(
            username.len() >= MIN_USERNAME_LENGTH && username.len() <= MAX_USERNAME_LENGTH,
            GovernanceError::InvalidUsername
        );

        // 2. Username Uniqueness Check
        // Verify the username hasn't been claimed already
        // If registry exists and is claimed, reject the request
        let user_registry = &mut self.user_registry;
        if user_registry.claimed {
            return err!(GovernanceError::UsernameAlreadyExists);
        } else {
            user_registry.claimed = true;
            user_registry.owner = self.user.key();
            user_registry.bump = bumps.user_registry;
        }

        // 3. Initialize User Profile
        // Start all users with zero reputation and Member role
        // This ensures fair starting conditions for all participants
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