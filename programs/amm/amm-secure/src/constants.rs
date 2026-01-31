// AMM Program Constants

pub const AMM_CONFIG_SEED: &[u8] = b"amm_config";
pub const AMM_AUTHORITY_SEED: &[u8] = b"amm_authority";
pub const LP_MINT_SEED: &[u8] = b"lp_mint";
pub const MAX_FEE_BASIS_POINTS: u16 = 1000; // 10% max
pub const MINIMUM_LIQUIDITY: u64 = 1000;
pub const MAX_EXPIRATION_SECONDS: i64 = 31_536_000; // 1 year
pub const ANCHOR_DISCRIMINATOR: usize = 8;

