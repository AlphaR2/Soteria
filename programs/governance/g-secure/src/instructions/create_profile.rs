use crate::errors::GovernanceError;
use crate::*;
use crate::errors::*;
use anchor_lang::prelude::*;

//create account
#[derive(Accounts)]
#[instruction(username: String)]
pub struct CreateProfile <'info> {
#[account(mut)]
pub user : Signer<'info>, // user
#[account(
init_if_needed,
payer = user,
space = ANCHOR_DISCRIMINATOR + UsernameRegistry::INIT_SPACE,
seeds = [USER_REGISTRY, username.as_bytes()], // will fail if you want to use an already choosen name 
bump
)]
pub user_registry : Account<'info, UsernameRegistry>, // we will use this as registry to make sure we have unique usernames 

#[account(
init,
payer = user,
space = ANCHOR_DISCRIMINATOR + UserProfile::INIT_SPACE,
seeds = [USERPROFILE, user.key().as_ref()],
bump

)]
pub user_profile : Account<'info, UserProfile>,
pub system_program : Program<'info, System>
}

impl <'info> CreateProfile <'info> {
	pub fn create_profile(
		&mut self,
		username: String,
		bumps: CreateProfileBumps
	) -> Result<()>{
		require!(username.len() <= 32, GovernanceError::InvalidUsername);
		require!(username.len() > 0, GovernanceError::InvalidUsername);

		// Handle user registry
        let user_registry = &mut self.user_registry;
        if user_registry.claimed { // if it has been claimed we return error.
            return err!(GovernanceError::UsernameAlreadyExists);
        } else {
            user_registry.claimed = true;
            user_registry.owner = self.user.key();
            user_registry.bump = bumps.user_registry;
        }

        // Initialize user profile
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