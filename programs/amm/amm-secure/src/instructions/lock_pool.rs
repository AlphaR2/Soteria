// Lock Pool Instruction
//
// Emergency pause mechanism. Only pool authority can lock.

use anchor_lang::prelude::*;
use crate::{constants::*, state::*};

#[derive(Accounts)]
pub struct LockPool<'info> {
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

impl<'info> LockPool<'info> {
    pub fn lock_pool(&mut self) -> Result<()> {
        // Validate authority
        self.pool_config.assert_is_authority(&self.authority.key())?;

        // Lock pool
        self.pool_config.lock()?;

        msg!("Pool locked by {}", self.authority.key());

        Ok(())
    }
}
