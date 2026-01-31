use anchor_lang::prelude::*;

#[constant]
pub const SEED: &str = "anchor";

pub const CONFIG: &[u8] = b"config";
pub const TREASURY: &[u8] = b"treasury";
pub const TREASURYAUTH: &[u8] = b"treasury_auth";
pub const TREASURYMINT: &[u8] = b"treasury_mint";
pub const USERPROFILE: &[u8] = b"user_profile";
pub const USER_REGISTRY: &[u8] = b"user_registry";
pub const VOTE_COOLDOWN : &[u8]  = b"cooldown";
pub const VOTE_RECORD : &[u8]  = b"vote_record";

pub const ANCHOR_DISCRIMINATOR: usize = 8;