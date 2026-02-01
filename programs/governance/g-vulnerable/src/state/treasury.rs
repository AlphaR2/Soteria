use anchor_lang::prelude::*;

// DAO Configuration
//
// SECURITY: Global configuration for the governance system
// Controls key parameters that affect all users and operations
// Only admin can modify these settings (via separate update instructions)
#[account]
#[derive(InitSpace)]
pub struct Config {
    // Admin authority
    // Only this pubkey can perform admin-only operations
    pub admin: Pubkey,

    // Minimum stake requirement for voting
    // SECURITY: Prevents sybil attacks by requiring economic commitment
    pub minimum_stake: u64,

    // Token mint for staking
    // SECURITY: Ensures only approved tokens can be staked
    pub token_mint: Pubkey,

    // Vote power multiplier
    // Multiplied with role weight to calculate final vote impact
    pub vote_power: u8,

    // System pause flag
    // SECURITY: Emergency stop for maintenance or security incidents
    pub is_paused: bool,

    // PDA bump
    pub config_bump: u8,
}

// Treasury State
//
// SECURITY: Tracks staking pool state and statistics
// All staked tokens are held in the treasury token account
// Treasury authority PDA signs all withdrawals
#[account]
#[derive(InitSpace)]
pub struct Treasury {
    // Admin authority
    pub admin: Pubkey,

    // Total tokens currently staked
    // SECURITY: Must match treasury token account balance
    pub total_staked: u64,

    // Number of unique stakers
    // Tracks users with non-zero stake
    pub stakers_count: u64,

    // Treasury token account address
    // Holds all staked tokens
    pub treasury_token_account: Pubkey,

    // PDA bumps
    pub state_bump: u8,
    pub vault_bump: u8,
}