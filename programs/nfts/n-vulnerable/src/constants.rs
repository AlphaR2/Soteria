// Constants for PDA derivation and program constraints

// PDA seed prefixes
pub const COLLECTION_STATE: &[u8] = b"collection_state";

// Attribute keys for staking data
pub const STAKED_KEY: &str = "staked";
pub const STAKED_TIME_KEY: &str = "staked_time";

// Staking constraints
pub const MIN_STAKE_DURATION: i64 = 30 * 24 * 60 * 60; // 30 days in seconds

// NFT metadata constraints
pub const MAX_NAME_LENGTH: usize = 32;
pub const MAX_URI_LENGTH: usize = 200;

pub const ANCHOR_DISCRIMINATOR: usize = 8;
