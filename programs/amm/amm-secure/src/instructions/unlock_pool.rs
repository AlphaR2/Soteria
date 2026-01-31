// Unlock Pool Instruction
//
// Re-enables pool operations. Only pool authority can unlock.

use anchor_lang::prelude::*;
use crate::{constants::*, state::*};

#[derive(Accounts)]
pub struct UnlockPool<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [
            AMM_CONFIG_SEED,
            pool_config.token_a_mint.as_ref(),
            pool_config.token_b_mint.as_ref(),
        ],
        bump = pool_config.config_bump,
    )]
    pub pool_config: Account<'info, PoolConfig>,
}

impl<'info> UnlockPool<'info> {
    pub fn unlock_pool(&mut self) -> Result<()> {
        // Validate authority
        self.pool_config.assert_is_authority(&self.authority.key())?;

        // Unlock pool
        self.pool_config.unlock()?;

        msg!("Pool unlocked by {}", self.authority.key());

        Ok(())
    }
}
