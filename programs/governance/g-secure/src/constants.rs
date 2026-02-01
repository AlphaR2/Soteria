use anchor_lang::prelude::*;

// PDA Seeds
//
// These seeds are used to derive Program Derived Addresses (PDAs)
// for secure account management. PDAs ensure only the program can
// sign transactions for these accounts, preventing unauthorized access.

pub const CONFIG: &[u8] = b"config";
pub const TREASURY: &[u8] = b"treasury";
pub const TREASURYAUTH: &[u8] = b"treasury_auth";
pub const TREASURYMINT: &[u8] = b"treasury_mint";
pub const USERPROFILE: &[u8] = b"user_profile";
pub const USER_REGISTRY: &[u8] = b"user_registry";
pub const VOTE_COOLDOWN: &[u8] = b"cooldown";
pub const VOTE_RECORD: &[u8] = b"vote_record";

// Account Space Constants
pub const ANCHOR_DISCRIMINATOR: usize = 8;

// Username Constraints
//
// SECURITY: Enforces username length to prevent:
// - Empty or single-character usernames (confusion attacks)
// - Excessively long usernames (storage abuse)
// - Username squatting on common short names
pub const MIN_USERNAME_LENGTH: usize = 3;
pub const MAX_USERNAME_LENGTH: usize = 32;

// Reputation System Limits
//
// SECURITY: Prevents reputation manipulation attacks
// Floor prevents users from being downvoted into impossibly low scores
// Makes the system more forgiving and allows recovery from negative reputation
pub const REPUTATION_FLOOR: i64 = -1000;
pub const REPUTATION_MEMBER_CAP: i64 = 50;
pub const REPUTATION_BRONZE_CAP: i64 = 100;
pub const REPUTATION_CONTRIBUTOR_CAP: i64 = 200;
pub const REPUTATION_GUARDIAN_CAP: i64 = 400;