use anchor_lang::prelude::*;

/// Tracks the state of our NFT collection for the staking program - VULNERABLE VERSION
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
    // VULNERABILITY NOTE: All helper methods removed in vulnerable version
    //
    // The secure version provides checked arithmetic helpers:
    // - increment_minted() - uses checked_add to prevent overflow
    // - increment_staked() - uses checked_add to prevent overflow
    // - decrement_staked() - uses checked_sub to prevent underflow
    //
    // In the vulnerable version, instructions directly manipulate counters
    // using unchecked arithmetic (+=, -=), which can overflow/underflow.
    //
    // Example Vulnerability in mint_nft.rs:
    //   self.collection_state.total_minted += 1; // Can overflow at u64::MAX
    //
    // Example Vulnerability in stake.rs:
    //   self.collection_state.total_staked += 1; // Can overflow at u64::MAX
    //
    // Example Vulnerability in unstake.rs:
    //   self.collection_state.total_staked -= 1; // Can underflow at 0
    //
    // Fix: Use the checked arithmetic helper methods from the secure version
}
