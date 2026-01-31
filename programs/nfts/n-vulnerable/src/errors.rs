use anchor_lang::prelude::*;

#[error_code]
pub enum NftError {
    #[msg("Collection has already been initialized")]
    CollectionAlreadyInitialized,

    #[msg("Collection authority mismatch")]
    CollectionAuthorityMismatch,

    #[msg("Invalid collection account")]
    InvalidCollection,

    #[msg("Asset owner mismatch")]
    AssetOwnerMismatch,

    #[msg("Asset is not part of the specified collection")]
    AssetNotInCollection,

    #[msg("Asset is already staked")]
    AlreadyStaked,

    #[msg("Asset is not currently staked")]
    NotStaked,

    #[msg("Staking attributes are not initialized")]
    StakingNotInitialized,

    #[msg("Attributes plugin not found")]
    AttributesNotInitialized,

    #[msg("Invalid timestamp format")]
    InvalidTimestamp,

    #[msg("Arithmetic overflow occurred")]
    Overflow,

    #[msg("Arithmetic underflow occurred")]
    Underflow,

    #[msg("NFT name exceeds maximum length")]
    NameTooLong,

    #[msg("NFT URI exceeds maximum length")]
    UriTooLong,

    #[msg("NFT name cannot be empty")]
    EmptyName,

    #[msg("NFT URI cannot be empty")]
    EmptyUri,

    #[msg("Invalid MPL Core program ID")]
    InvalidMplCoreProgram,

    #[msg("Unauthorized: Only collection authority can perform this action")]
    UnauthorizedAuthority,

    #[msg("Unauthorized: Only asset owner can perform this action")]
    UnauthorizedOwner,

    #[msg("FreezeDelegate plugin not found")]
    FreezeDelegateNotFound,

    #[msg("Cannot stake: minimum staking duration not met")]
    MinimumStakeDurationNotMet,

    #[msg("Invalid payer account")]
    InvalidPayer,
}
