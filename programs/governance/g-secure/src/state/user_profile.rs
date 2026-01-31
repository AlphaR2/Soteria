
use anchor_lang::prelude::*;

//user profile where the user profile 
#[account]
#[derive(InitSpace)]

pub struct UserProfile {
    #[max_len(32)]
    pub username: String,
    pub owner: Pubkey,               
    pub reputation_points: i64,       
    pub stake_amount: u64,             
    pub role_level: MemberRanks,              
    pub upvotes_received: u64, 
    pub downvotes_received: u64,
    pub total_votes_cast: u64, 
    pub last_vote_timestamp: i64,     
    pub created_at: i64,            
}


#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Copy, PartialEq, Eq)]
pub enum MemberRanks {
    /// Entry level - Can upvote only, 24h cooldown
    /// Reputation: 0-50 points
    /// Perks: Basic profile, stake tokens, upvote others
    Member,
    /// First upgrade - Unlocks downvoting ability
    /// Reputation: 50-100 points  
    /// Perks: Can downvote, bronze badge, all Member perks
    Bronze,
    /// Active community member - Enhanced voting power
    /// Reputation: 100-200 points
    /// Perks:  18h cooldown, can nominate others
    Contributor,
    /// Trusted moderator - Significant influence
    /// Reputation: 200-400 points
    /// Perks: 12h cooldown, can vote twice daily, moderator privileges
    Guardian,
    /// Elite status - Maximum privileges and recognition
    /// Reputation: 400+ points
    /// Perks: no cooldown, unlimited voting, governance proposals
    Leader,
 }

 impl MemberRanks {
    pub fn can_downvote(&self) -> bool {
        !matches!(self, MemberRanks::Member)
    }

    pub fn cooldown_hours(&self) -> u64 {
        match self {
            MemberRanks::Member | MemberRanks::Bronze => 24,  // Full day cooldown
            MemberRanks::Contributor => 18,                    // 25% reduction
            MemberRanks::Guardian => 12,                       // 50% reduction  
            MemberRanks::Leader => 0,                          // No restrictions
        }
    }

    pub fn from_reputation(points: i64) -> Self {
        match points {
            i64::MIN..=50 => MemberRanks::Member,      
            51..=100 => MemberRanks::Bronze,          
            101..=200 => MemberRanks::Contributor,      
            201..=400 => MemberRanks::Guardian,     
            401..=i64::MAX => MemberRanks::Leader, 
        }
    }

    pub fn vote_weight(&self) -> u8 {
    match self {
        MemberRanks::Member => 1,        // +1 or -1 reputation
        MemberRanks::Bronze => 1,        // +1 or -1 reputation  
        MemberRanks::Contributor => 2,   // +2 or -2 reputation
        MemberRanks::Guardian => 2,      // +2 or -2 reputation
        MemberRanks::Leader => 3,        // +3 or -3 reputation
    }
}
 }

impl Space for MemberRanks {
	const INIT_SPACE : usize = 1;
}


#[account]
#[derive(InitSpace)]
pub struct UsernameRegistry {
    pub claimed: bool,
    pub owner: Pubkey,
    pub bump: u8
}