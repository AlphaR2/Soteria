pub const ANCHOR_DISCRIMINATOR: usize = 8;

// Seeds for PDA derivation
pub const MULTISIG: &[u8] = b"multisig";
pub const PROPOSAL: &[u8] = b"proposal";
pub const VAULT: &[u8] = b"vault";
pub const TRANSFER_PROPOSAL: &[u8] = b"transfer";

// VULNERABILITY [HIGH]: No maximum owners limit enforced
//
// While MAX_OWNERS is defined, the vulnerable instructions don't
// check against it. An attacker could potentially add more members
// than the fixed array can hold, causing undefined behavior or panics.
//
// Fix: Always check owner_count < MAX_OWNERS before adding members.
pub const MAX_OWNERS: usize = 10;

// VULNERABILITY [MEDIUM]: Expiry period too long or not enforced
//
// A 7-day expiry period means malicious proposals can linger for a week.
// If expiry checks are missing, proposals never expire at all.
//
// Fix: Implement strict expiry checks and consider shorter periods.
pub const DEFAULT_EXPIRY_PERIOD: u64 = 7 * 24 * 60 * 60;
