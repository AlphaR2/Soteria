use anchor_lang::prelude::*;

// Vote Cooldown Tracking
//
// VULNERABILITY: Cooldown tracking exists but not enforced in vote instruction
// The account is created but the timestamp check is missing
// Allows users to vote repeatedly without waiting for cooldown period
#[account]
#[derive(InitSpace)]
pub struct VoteCooldown {
    pub voter: Pubkey,
    pub last_vote_timestamp: i64,
    pub bump: u8,
}

// Vote Record
//
// VULNERABILITY: Uses 'init' instead of 'init_if_needed'
// Users cannot change their votes after initial vote is cast
// Once a vote_record PDA exists, subsequent votes fail with "already initialized" error
// This makes the voting system inflexible and punishes users who vote early
//
// VULNERABILITY: vote_weight stored as u8 despite calculations using i64
// When role_weight * vote_power exceeds 255, vote power is truncated
// High-level users with strong vote multipliers lose voting influence
#[account]
#[derive(InitSpace)]
pub struct VoteRecord {
    pub voter: Pubkey,
    #[max_len(32)]
    pub target_username: String,
    pub target_owner: Pubkey,
    pub vote_type: VoteType,
    pub vote_weight: u8,
    pub timestamp: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Copy, PartialEq, Eq)]
pub enum VoteType {
    Upvote,
    Downvote,
}

impl Space for VoteType {
    const INIT_SPACE: usize = 1;
}
