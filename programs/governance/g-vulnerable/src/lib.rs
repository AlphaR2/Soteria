pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use errors::*;
pub use state::*;

declare_id!("Bx6GKFsyW1YbJdQu9f5mW5Z4T6rNpkQZPnYDVr7pump");


#[program]
pub mod vulnerable {
    use crate::errors::GovernanceError;

    use super::*;
    // / Initialize the DAO and set configuration parameters
    pub fn init_dao(
        ctx: Context<InitializeDaoProgram>,
        admin: Pubkey,
        minimum_stake: u64,
        token_mint: Pubkey,
        vote_power: u8,
    ) -> Result<()> {
        ctx.accounts.initialize(
            minimum_stake,
            admin,
            token_mint,
            vote_power,
            ctx.bumps
        )
    }

    /// Initialize the treasury for token staking
    pub fn initialize_treasury(
        ctx: Context<InitializeTreasury>,
    ) -> Result<()> {
        ctx.accounts.initialize_treasury(ctx.bumps)
    }

    /// Create a new user profile with a unique username
    pub fn create_profile(
        ctx: Context<CreateProfile>,
        username: String,
    ) -> Result<()> {
        let bumps = ctx.bumps;
        ctx.accounts.create_profile(username, bumps)
    }

    /// Stake tokens to gain voting rights
    pub fn stake_tokens(
        ctx: Context<Stake>,
        amount: u64,
    ) -> Result<()> {
        // Validate amount
        require!(amount > 0, GovernanceError::InvalidStakeAmount);
		ctx.accounts.stake_tokens(amount)
    }

    /// Unstake tokens and reduce voting power
    pub fn unstake_tokens(
        ctx: Context<Unstake>,
        amount: u64,
    ) -> Result<()> {
        ctx.accounts.unstake_tokens(amount)
    }

    /// Cast an upvote for another user
    pub fn upvote(
        ctx: Context<Vote>,
        target_username: String,
    ) -> Result<()> {
        let bumps = ctx.bumps;
        ctx.accounts.upvote_user(target_username, bumps)
    }

    /// Cast a downvote for another user
    pub fn downvote(
        ctx: Context<Vote>,
        target_username: String,
    ) -> Result<()> {
        let bumps = ctx.bumps;
        ctx.accounts.downvote_user(target_username, bumps)
    }

    /// Reset a user's reputation (admin only)
    pub fn reset_user_reputation(
        ctx: Context<ResetUserReputation>,
        user: Pubkey
    ) -> Result<()> {
        ctx.accounts.reset_user_reputation()
    }

}