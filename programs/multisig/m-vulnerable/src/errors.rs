use anchor_lang::prelude::*;


#[error_code]
pub enum MultisigError {
  
    #[msg("Invalid operation")]
    InvalidOperation,

    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("Invalid state")]
    InvalidState,

    #[msg("Not a member")]
    NotAMember,

    #[msg("Invalid threshold")]
    InvalidThreshold,

    #[msg("Proposal not active")]
    ProposalNotActive,

    #[msg("Insufficient approvals")]
    InsufficientApprovals,

    #[msg("Overflow")]
    Overflow,
}
