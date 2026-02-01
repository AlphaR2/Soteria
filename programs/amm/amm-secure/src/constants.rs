
// Seed for pool configuration PDA
// Derived with: [AMM_CONFIG_SEED, token_a_mint, token_b_mint]
pub const AMM_CONFIG_SEED: &[u8] = b"amm_config";

// Seed for pool authority PDA (signer for vault operations)
// Derived with: [AMM_AUTHORITY_SEED, pool_config_pubkey]
pub const AMM_AUTHORITY_SEED: &[u8] = b"amm_authority";

// Seed for LP token mint PDA
// Derived with: [LP_MINT_SEED, pool_config_pubkey]
pub const LP_MINT_SEED: &[u8] = b"lp_mint";

// LIMITS AND THRESHOLDS

// Maximum swap fee (1000 basis points = 10%)
// Prevents excessive fees that would harm users
pub const MAX_FEE_BASIS_POINTS: u16 = 1000;

// Minimum liquidity locked on first deposit
// Prevents division by zero and protects against inflation attacks
// These tokens are permanently locked by being sent to the zero address
pub const MINIMUM_LIQUIDITY: u64 = 1000;

// Maximum transaction expiration (1 year in seconds)
// Prevents unreasonably far-future expirations
pub const MAX_EXPIRATION_SECONDS: i64 = 31_536_000;

pub const ANCHOR_DISCRIMINATOR: usize = 8;
