use anchor_lang::prelude::*;

// Vote Cooldown Tracking
//
// SECURITY: Prevents spam voting attacks
// Tracks last vote timestamp per user to enforce role-based cooldowns
// Different roles have different cooldown periods (0-24 hours)
#[account]
#[derive(InitSpace)]
pub struct VoteCooldown {
    pub voter: Pubkey,
    pub last_vote_timestamp: i64,
    pub bump: u8,
}

// Vote Record
//
// SECURITY: Provides vote auditability and prevents vote spam
// Stores historical voting data for transparency
// Allows users to change their votes (upvote to downvote or vice versa)
//
// NOTE: vote_weight stored as i64 to preserve full voting power calculation
// This prevents truncation when role_weight * vote_power exceeds u8::MAX
#[account]
#[derive(InitSpace)]
pub struct VoteRecord {
    pub voter: Pubkey,
    #[max_len(32)]
    pub target_username: String,
    pub target_owner: Pubkey,
    pub vote_type: VoteType,
    pub vote_weight: i64,
    pub timestamp: i64,
    pub bump: u8,
}

// Vote Type Enum
//
// Represents the direction of a reputation vote
// Upvote increases reputation, Downvote decreases it
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Copy, PartialEq, Eq)]
pub enum VoteType {
    Upvote,
    Downvote,
}

impl Space for VoteType {
    const INIT_SPACE: usize = 1;
}