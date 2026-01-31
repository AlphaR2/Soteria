use anchor_lang::prelude::*;

#[error_code]
pub enum MultisigError {
    // Member validation errors
    #[msg("Signer is not a member of this multisig")]
    NotAMember,

    #[msg("Address is already a member")]
    AlreadyMember,

    #[msg("Cannot remove the creator from the multisig")]
    CannotRemoveCreator,

    #[msg("Maximum number of members reached")]
    MaxMembersReached,

    #[msg("Multisig must have at least one member")]
    MinimumOneMember,

    // Role/permission errors
    #[msg("Member does not have permission to perform this action")]
    InsufficientPermissions,

    #[msg("Only admin can perform this action")]
    OnlyAdmin,

    #[msg("Member cannot propose - must be Admin or Proposer role")]
    CannotPropose,

    #[msg("Member cannot execute - must be Admin or Executor role")]
    CannotExecute,

    #[msg("Cannot add yourself as a member")]
    CannotAddSelf,

    // Threshold errors
    #[msg("Invalid threshold: must be between 1 and owner count")]
    InvalidThreshold,

    #[msg("Threshold cannot exceed number of owners")]
    ThresholdExceedsOwners,

    // Proposal errors
    #[msg("Proposal is not active")]
    ProposalNotActive,

    #[msg("Member has already approved this proposal")]
    AlreadyApproved,

    #[msg("Proposal has not reached required approvals")]
    InsufficientApprovals,

    #[msg("Only the proposer or admin can cancel this proposal")]
    NotProposer,

    #[msg("Proposal has expired and cannot be executed")]
    ProposalExpired,

    #[msg("Timelock period has not passed yet")]
    TimelockNotPassed,

    #[msg("Invalid proposal type for this instruction")]
    InvalidProposalType,

    #[msg("Invalid proposal - mismatch between accounts")]
    InvalidProposal,

    // Execution errors
    #[msg("Insufficient funds in multisig vault")]
    InsufficientFunds,

    #[msg("Invalid recipient address")]
    InvalidRecipient,

    // Arithmetic errors
    #[msg("Arithmetic overflow")]
    Overflow,

    // State errors
    #[msg("Multisig is paused - only admin can unpause")]
    MultisigPaused,

    #[msg("Invalid parameter provided")]
    InvalidParameter,
}
