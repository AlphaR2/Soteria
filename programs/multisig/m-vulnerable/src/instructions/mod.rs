// Instructions module - vulnerable implementations
//
// Each instruction is intentionally missing critical security checks
// to demonstrate common vulnerabilities in multisig programs.
//
// Compare with the secure version to understand what checks are needed.

pub mod create_multisig;
pub mod create_proposal;
pub mod create_transfer_proposal;
pub mod approve_proposal;
pub mod approve_transfer_proposal;
pub mod execute_proposal;
pub mod execute_transfer_proposal;
pub mod cancel_proposal;
pub mod toggle_pause;

pub use create_multisig::*;
pub use create_proposal::*;
pub use create_transfer_proposal::*;
pub use approve_proposal::*;
pub use approve_transfer_proposal::*;
pub use execute_proposal::*;
pub use execute_transfer_proposal::*;
pub use cancel_proposal::*;
pub use toggle_pause::*;
