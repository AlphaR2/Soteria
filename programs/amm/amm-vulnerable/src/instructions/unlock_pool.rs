// Unlock Pool Instruction - VULNERABLE VERSION
//
// WARNING: This version contains intentional vulnerabilities for educational purposes.
//
// VULNERABILITY:
// V006: No authorization check - anyone can unlock any pool

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
    pub pool_config: Box<Account<'info, PoolConfig>>,
}

impl<'info> UnlockPool<'info> {
    pub fn unlock_pool(&mut self) -> Result<()> {
        // VULNERABILITY V006: No authorization check
        // Secure version: self.pool_config.assert_is_authority(&self.authority.key())?;
        // Attack scenario:
        // 1. Pool authority discovers critical bug and locks pool to protect users
        // 2. Attacker immediately calls unlock_pool (no auth check)
        // 3. Pool becomes unlocked, vulnerable operations resume
        // 4. Users lose funds due to the bug that authority tried to prevent
        // Combined with V007 (no lock enforcement), this makes pool locks completely useless

        // Unlock pool (no authorization required in vulnerable version)
        self.pool_config.unlock()?;

        msg!("Pool unlocked by {}", self.authority.key());

        Ok(())
    }
}
