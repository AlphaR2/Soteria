
use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Fee basis points cannot exceed maximum allowed (1000 = 10%)")]
    FeeTooHigh, 

    #[msg("Token mints must be different - cannot create pool with same token")]
    IdenticalTokenMints,


    #[msg("Deposit amount cannot be zero")]
    ZeroDepositAmount, 

    #[msg("Withdrawal amount cannot be zero")]
    ZeroWithdrawAmount,

    #[msg("Insufficient liquidity in pool for this withdrawal")]
    InsufficientLiquidity,

    #[msg("Deposited amount exceeds maximum allowed (slippage protection)")]
    ExcessiveDepositAmount,

    #[msg("Withdrawn amount below minimum required (slippage protection)")]
    InsufficientWithdrawAmount, 

    #[msg("Swap amount cannot be zero")]
    ZeroSwapAmount,

    #[msg("Swap output is below minimum required (slippage protection)")]
    SlippageExceeded, 

    #[msg("Pool does not have enough liquidity for this swap")]
    InsufficientPoolLiquidity, 


    #[msg("Arithmetic overflow occurred")]
    Overflow, 

    #[msg("Arithmetic underflow occurred")]
    Underflow,

    #[msg("Division by zero attempted")]
    DivisionByZero, 

    #[msg("Pool is currently locked - operations are disabled")]
    PoolLocked, 

    #[msg("Pool is already locked")]
    PoolAlreadyLocked, 

    #[msg("Pool is already unlocked")]
    PoolAlreadyUnlocked, 

    #[msg("Only the pool authority can perform this action")]
    UnauthorizedAccess, 

    #[msg("Transaction deadline has expired")]
    TransactionExpired, 

    #[msg("Expiration timestamp is too far in the future")]
    ExpirationTooFar, 

    #[msg("Expiration timestamp must be in the future")]
    ExpirationInPast, 


    #[msg("Constant product curve calculation failed")]
    CurveCalculationFailed, 

    #[msg("Invalid curve parameters provided")]
    InvalidCurveParams,
}
