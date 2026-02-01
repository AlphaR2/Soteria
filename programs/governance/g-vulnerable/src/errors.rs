use anchor_lang::prelude::*;

#[error_code]
pub enum GovernanceError {
    // Stake-related errors
    #[msg("User must stake at least 10 tokens to vote")]
    InsufficientStake,
    
    #[msg("Minimum stake amount is 10 tokens")]
    MinimumStakeRequired,
    
    #[msg("Invalid stake amount")]
    InvalidStakeAmount,
    
    // Vote-related errors
    #[msg("Vote cooldown period is still active")]
    VoteCooldownActive,

    #[msg("Cannot vote for yourself")]
    CannotVoteForSelf,
    
    #[msg("Your role is not high enough to downvote")]
    CannotDownvote,
    
    #[msg("Your role is not high enough for this action")]
    UnauthorizedRole,
    
    // Username-related errors
    #[msg("Username must be between 3 and 32 characters")]
    InvalidUsername,
    
    #[msg("This username is already taken")]
    UsernameAlreadyExists,
    
    #[msg("Username not found")]
    UsernameNotFound,
    
    // Authorization errors
    #[msg("Only the admin can perform this action")]
    UnauthorizedAdmin,
    
    #[msg("Unauthorized user")]
    UnauthorizedUser,
    
    // Account validation errors
    #[msg("Profile mismatch - username doesn't match owner")]
    ProfileMismatch,
    
    #[msg("Invalid token mint")]
    InvalidTokenMint,
    
    #[msg("Invalid treasury account")]
    InvalidTreasuryAccount,
    
    // Treasury errors
    #[msg("Insufficient treasury balance")]
    InsufficientTreasuryBalance,
    
    // System errors
    #[msg("System is currently paused")]
    SystemPaused,
    
    #[msg("Math overflow occurred")]
    MathOverflow,
    
    // Token errors
    #[msg("Token transfer failed")]
    TokenTransferFailed,
    
    #[msg("Failed to create token account")]
    TokenAccountCreationFailed,
    
    // General errors
    #[msg("Invalid instruction data")]
    InvalidInstructionData,
    
    #[msg("Account already initialized")]
    AccountAlreadyInitialized,
}