pub const ANCHOR_DISCRIMINATOR: usize = 8;

// Seeds for PDA derivation: ["multisig", creator, multisig_id]
pub const MULTISIG: &[u8] = b"multisig";

// Seeds for PDA derivation: ["proposal", multisig, proposal_id]
pub const PROPOSAL: &[u8] = b"proposal";

// Seeds for PDA derivation: ["vault", multisig]
pub const VAULT: &[u8] = b"vault";

// Seeds for PDA derivation: ["transfer", proposal]
pub const TRANSFER_PROPOSAL: &[u8] = b"transfer";

// Maximum number of members allowed in the multisig
pub const MAX_OWNERS: usize = 10;

// Default expiry grace period (7 days in seconds)
// Proposals expire after: created_at + timelock + grace_period
pub const DEFAULT_EXPIRY_PERIOD: u64 = 7 * 24 * 60 * 60;

