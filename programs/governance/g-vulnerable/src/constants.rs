use anchor_lang::prelude::*;

// PDA Seeds
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
// VULNERABILITY: Overly permissive username validation
// Allows usernames as short as 1 character, enabling:
// - Username squatting on single-letter names
// - Confusion attacks using similar single characters
// - Reduced uniqueness in the namespace
pub const MIN_USERNAME_LENGTH: usize = 1;
pub const MAX_USERNAME_LENGTH: usize = 32;

// Reputation System
//
// VULNERABILITY: No reputation floor
// Missing REPUTATION_FLOOR constant allows unlimited downvoting
// Users can be griefed to i64::MIN reputation with no recovery path
