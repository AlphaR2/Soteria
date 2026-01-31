use anchor_lang::prelude::*;
pub mod instructions;
pub mod errors;
pub mod state;
pub mod constants;

pub use instructions::*;
pub use errors::*;
pub use state::*;

declare_id!("HH8rYFiTjMX8FiiRgiFQx1jnXdT9D4TTiC5mSBhe9r7P");

#[program]
pub mod secure {
    use super::*;

    // Initialize a new multisig wallet
    // Creates the multisig account and associated vault PDA
    // Creator becomes admin (only admin role)
    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        multisig_id: u64,
        threshold: u8,
        timelock_seconds: u64,
    ) -> Result<()> {
        ctx.accounts.create_multisig(multisig_id, threshold, timelock_seconds, &ctx.bumps)
    }

    // Create a new governance proposal requiring multi-sig approval
    // Only Admin or Proposer roles can create proposals
    // Proposer automatically approves their own proposal
    // Handles: AddMember, RemoveMember, ChangeThreshold, ChangeTimelock
    // For TransferSol: use create_transfer_proposal instead
    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        proposal_type: ProposalType,
    ) -> Result<()> {
        ctx.accounts.create_proposal(proposal_type, &ctx.bumps)
    }

    // Create a new transfer proposal requiring multi-sig approval
    // Only Admin or Proposer roles can create proposals
    // Proposer automatically approves their own proposal
    // Creates both base Proposal and linked TransferProposal accounts
    pub fn create_transfer_proposal(
        ctx: Context<CreateTransferProposal>,
        amount: u64,
        recipient: Pubkey,
    ) -> Result<()> {
        ctx.accounts.create_transfer_proposal(amount, recipient, &ctx.bumps)
    }

    // Approve an existing governance proposal
    // Each member can only approve once per proposal
    pub fn approve_proposal(ctx: Context<ApproveProposal>) -> Result<()> {
        ctx.accounts.approve_proposal()
    }

    // Approve an existing transfer proposal
    // Each member can only approve once per proposal
    pub fn approve_transfer_proposal(ctx: Context<ApproveTransferProposal>) -> Result<()> {
        ctx.accounts.approve_transfer_proposal()
    }

    // Execute an approved governance proposal once threshold is reached
    // Handles AddMember, RemoveMember, ChangeThreshold, ChangeTimelock
    // For TransferSol: use execute_transfer_proposal instead
    pub fn execute_proposal(ctx: Context<ExecuteProposal>) -> Result<()> {
        ctx.accounts.execute_proposal()
    }

    // Execute an approved transfer proposal once threshold is reached
    // Uses UncheckedAccount for recipient with manual validation
    // Recipient must be writable and system-owned
    pub fn execute_transfer_proposal(ctx: Context<ExecuteTransferProposal>) -> Result<()> {
        ctx.accounts.execute_transfer_proposal()
    }

    // Cancel an active proposal
    // Only proposer or creator can cancel
    pub fn cancel_proposal(ctx: Context<CancelProposal>) -> Result<()> {
        ctx.accounts.cancel_proposal()
    }

    // Toggle pause state on the multisig
    // Only admin (creator) can pause/unpause
    // Emergency brake for security incidents
    pub fn toggle_pause(ctx: Context<TogglePause>) -> Result<()> {
        ctx.accounts.toggle_pause()
    }
}

