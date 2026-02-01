// Pool Configuration State
//
// Stores the configuration and metadata for a single liquidity pool.
// One PoolConfig exists per token pair (e.g., SOL/USDC pool).
//


use anchor_lang::prelude::*;
use crate::errors::*;

#[account]
#[derive(InitSpace)]
pub struct PoolConfig {
    // Pool creator and administrator
    // Only this address can lock/unlock the pool
    pub authority: Pubkey,

    // Token A mint address (first token in the pair)
    pub token_a_mint: Pubkey,

    // Token B mint address (second token in the pair)
    pub token_b_mint: Pubkey,

    // LP (Liquidity Provider) token mint
    // Users receive LP tokens when depositing liquidity
    pub lp_token_mint: Pubkey,

    // Swap fee in basis points (1 basis point = 0.01%)
    // Example: 30 = 0.30% fee per swap
    pub fee_basis_points: u16,

    // Emergency pause flag
    // When true, all operations except unlock are disabled
    pub locked: bool,

    // PDA bump seeds (stored to avoid recomputation)
    pub config_bump: u8,       // Bump for this config PDA
    pub authority_bump: u8,    // Bump for pool authority PDA
    pub lp_mint_bump: u8,      // Bump for LP mint PDA
}

impl PoolConfig {
    // Lock the pool (emergency pause)
    // Prevents deposits, withdrawals, and swaps
    pub fn lock(&mut self) -> Result<()> {
        require!(!self.locked, AmmError::PoolAlreadyLocked);
        self.locked = true;
        Ok(())
    }

    // Unlock the pool (resume operations)
    // Allows normal pool operations to continue
    pub fn unlock(&mut self) -> Result<()> {
        require!(self.locked, AmmError::PoolAlreadyUnlocked);
        self.locked = false;
        Ok(())
    }

    // Assert pool is not locked
    // Called at the start of deposit, withdraw, and swap operations
    pub fn assert_not_locked(&self) -> Result<()> {
        require!(!self.locked, AmmError::PoolLocked);
        Ok(())
    }

    // Assert caller is the pool authority
    // Used to restrict lock/unlock to pool creator
    pub fn assert_is_authority(&self, caller: &Pubkey) -> Result<()> {
        require!(self.authority == *caller, AmmError::UnauthorizedAccess);
        Ok(())
    }
}
