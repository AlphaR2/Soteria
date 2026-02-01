// AMM Program Constants - VULNERABLE VERSION
//
// VULNERABILITIES:
// - MAX_FEE_BASIS_POINTS set to u16::MAX (allows 655.35% fees!)
// - MINIMUM_LIQUIDITY reduced to 1 (enables inflation attacks)

// PDA SEEDS (same as secure version)

pub const AMM_CONFIG_SEED: &[u8] = b"amm_config";
pub const AMM_AUTHORITY_SEED: &[u8] = b"amm_authority";
pub const LP_MINT_SEED: &[u8] = b"lp_mint";

// VULNERABLE LIMITS

// VULNERABILITY 1: No maximum fee enforcement
// Allows pool creators to set exorbitant fees (up to 655.35%)
// Secure version limits this to 1000 (10%)
pub const MAX_FEE_BASIS_POINTS: u16 = u16::MAX;

// VULNERABILITY 2: Minimum liquidity too low
// With only 1 token locked, inflation attacks become feasible
// Attacker can manipulate LP token value by donating to pool
// Secure version uses 1000 to make attacks economically infeasible
pub const MINIMUM_LIQUIDITY: u64 = 1;

// VULNERABILITY 3: Expiration validation not enforced
// This constant exists but is never checked in vulnerable version
// Allows stale transactions to execute at unfavorable prices
pub const MAX_EXPIRATION_SECONDS: i64 = 31_536_000;

pub const ANCHOR_DISCRIMINATOR: usize = 8;
