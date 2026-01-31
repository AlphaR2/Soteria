use anchor_lang::prelude::*;


#[account]
#[derive(InitSpace)]
pub struct VoteCooldown {
    pub voter: Pubkey,
    pub last_vote_timestamp: i64,
    pub bump: u8,
}

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