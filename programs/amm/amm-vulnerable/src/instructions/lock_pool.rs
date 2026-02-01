// Lock Pool Instruction - VULNERABLE VERSION
//
// WARNING: This version contains intentional vulnerabilities for educational purposes.
//
// VULNERABILITY:
// V006: No authorization check - anyone can lock any pool (DoS attack)

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
    pub pool_config: Box<Account<'info, PoolConfig>>,
}

impl<'info> LockPool<'info> {
    pub fn lock_pool(&mut self) -> Result<()> {
        // VULNERABILITY V006: No authorization check
        // Secure version: self.pool_config.assert_is_authority(&self.authority.key())?;
        // Attack scenario (DoS):
        // 1. Attacker identifies popular high-volume pool
        // 2. Attacker calls lock_pool (no auth check, so anyone can do it)
        // 3. Pool becomes locked, preventing all deposits, withdrawals, and swaps
        // 4. Legitimate users cannot interact with pool
        // 5. Protocol loses trading volume and user trust
        // 6. Attacker can do this to all pools for minimal cost

        // Lock pool (no authorization required in vulnerable version)
        self.pool_config.lock()?;

        msg!("Pool locked by {}", self.authority.key());

        Ok(())
    }
}
