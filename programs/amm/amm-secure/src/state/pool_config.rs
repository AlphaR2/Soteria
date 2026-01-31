// Pool Configuration State

use anchor_lang::prelude::*;
use crate::errors::*;

#[account]
#[derive(InitSpace)]
pub struct PoolConfig {
    pub authority: Pubkey,        // Can lock/unlock pool
    pub token_a_mint: Pubkey,      // First token in pair
    pub token_b_mint: Pubkey,      // Second token in pair
    pub lp_token_mint: Pubkey,     // LP token mint
    pub fee_basis_points: u16,     // Swap fee (e.g., 30 = 0.30%)
    pub locked: bool,              // Emergency pause state
    pub config_bump: u8,           // PDA bump for config
    pub authority_bump: u8,        // PDA bump for authority
    pub lp_mint_bump: u8,          // PDA bump for LP mint
}

impl PoolConfig {
    pub fn lock(&mut self) -> Result<()> {
        require!(!self.locked, AmmError::PoolAlreadyLocked);
        self.locked = true;
        Ok(())
    }

    pub fn unlock(&mut self) -> Result<()> {
        require!(self.locked, AmmError::PoolAlreadyUnlocked);
        self.locked = false;
        Ok(())
    }

    pub fn assert_not_locked(&self) -> Result<()> {
        require!(!self.locked, AmmError::PoolLocked);
        Ok(())
    }

    pub fn assert_is_authority(&self, caller: &Pubkey) -> Result<()> {
        require!(self.authority == *caller, AmmError::UnauthorizedAccess);
        Ok(())
    }
}
