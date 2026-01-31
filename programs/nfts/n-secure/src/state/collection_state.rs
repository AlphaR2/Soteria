use anchor_lang::prelude::*;

/// Tracks the state of our NFT collection for the staking program
/// This PDA stores metadata about the collection used for validation
#[account]
#[derive(InitSpace)]

pub struct CollectionState {
    /// The authority that created and controls this collection
    pub authority: Pubkey,

    /// The Metaplex Core collection pubkey
    pub collection: Pubkey,

    /// Total number of NFTs minted in this collection
    pub total_minted: u64,

    /// Total number of NFTs currently staked
    pub total_staked: u64,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl CollectionState {
  
    /// Increment the total minted counter
    pub fn increment_minted(&mut self) -> Result<()> {
        self.total_minted = self.total_minted
            .checked_add(1)
            .ok_or(crate::errors::NftError::Overflow)?;
        Ok(())
    }

    /// Increment the total staked counter
    pub fn increment_staked(&mut self) -> Result<()> {
        self.total_staked = self.total_staked
            .checked_add(1)
            .ok_or(crate::errors::NftError::Overflow)?;
        Ok(())
    }

    /// Decrement the total staked counter
    pub fn decrement_staked(&mut self) -> Result<()> {
        self.total_staked = self.total_staked
            .checked_sub(1)
            .ok_or(crate::errors::NftError::Underflow)?;
        Ok(())
    }
}
