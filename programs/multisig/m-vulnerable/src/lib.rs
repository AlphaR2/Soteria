use anchor_lang::prelude::*;

pub mod instructions;
pub mod errors;
pub mod state;
pub mod constants;

pub use instructions::*;
pub use errors::*;
pub use state::*;

declare_id!("2Skteich3Jdz4W41oek3wrwdFSFRJcgvaAT7H1bxGvck");

// VULNERABILITY [CRITICAL]: No program ID verification in entry point
//
// While Anchor handles this automatically, the declare_id! macro
// could be bypassed if the program were invoked via CPI with a
// different program ID. In a native program without Anchor's protections,
// this would allow identity spoofing.
//
// The secure version uses Anchor's built-in program ID verification.

#[program]
pub mod vulnerable {
    use super::*;

    // VULNERABILITY [MEDIUM]: No input sanitization on multisig_id
    //
    // The multisig_id can be any u64 value without validation.
    // An attacker could use specially crafted IDs to create predictable PDAs
    // or cause integer overflow in PDA derivation.
    //
    // Fix: Validate multisig_id is within reasonable bounds.
    pub fn create_multisig(
        ctx: Context<CreateMultisig>,
        multisig_id: u64,
        threshold: u8,
        timelock_seconds: u64,
    ) -> Result<()> {
        ctx.accounts.create_multisig(multisig_id, threshold, timelock_seconds, &ctx.bumps)
    }

    // VULNERABILITY [CRITICAL]: Missing role-based access control
    //
    // The secure version validates that only Admin or Proposer roles
    // can create proposals. This vulnerable version allows any member
    // or potentially non-members to create proposals.
    //
    // Fix: Implement proper role checking before allowing proposal creation.
    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        proposal_type: ProposalType,
    ) -> Result<()> {
        ctx.accounts.create_proposal(proposal_type, &ctx.bumps)
    }

    // VULNERABILITY [CRITICAL]: Missing validation on transfer amounts
    //
    // No checks for:
    // - Zero amount transfers
    // - Amount exceeding vault balance
    // - Recipient validation
    //
    // Fix: Validate amount > 0, recipient != default, vault has funds.
    pub fn create_transfer_proposal(
        ctx: Context<CreateTransferProposal>,
        amount: u64,
        recipient: Pubkey,
    ) -> Result<()> {
        ctx.accounts.create_transfer_proposal(amount, recipient, &ctx.bumps)
    }

    // VULNERABILITY [CRITICAL]: Double approval possible
    //
    // The secure version uses a bitmap to track which members have approved
    // and prevents double approvals. This vulnerable version may allow
    // the same member to approve multiple times, inflating approval_count.
    //
    // Fix: Check bitmap before approving, use atomic bitmap operations.
    pub fn approve_proposal(ctx: Context<ApproveProposal>) -> Result<()> {
        ctx.accounts.approve_proposal()
    }

    pub fn approve_transfer_proposal(ctx: Context<ApproveTransferProposal>) -> Result<()> {
        ctx.accounts.approve_transfer_proposal()
    }

    // VULNERABILITY [CRITICAL]: Missing threshold and timelock checks
    //
    // The secure version validates:
    // - approval_count >= threshold
    // - current_time >= created_at + timelock_seconds
    // - proposal not expired
    //
    // This vulnerable version may allow premature execution.
    //
    // Fix: Implement all three checks before executing.
    pub fn execute_proposal(ctx: Context<ExecuteProposal>) -> Result<()> {
        ctx.accounts.execute_proposal()
    }

    // VULNERABILITY [CRITICAL]: No recipient validation
    //
    // The secure version validates:
    // - Recipient is system-owned (not a PDA)
    // - Recipient matches stored proposal recipient
    // - Vault has sufficient balance
    //
    // Fix: Implement full recipient and balance validation.
    pub fn execute_transfer_proposal(ctx: Context<ExecuteTransferProposal>) -> Result<()> {
        ctx.accounts.execute_transfer_proposal()
    }

    // VULNERABILITY [HIGH]: Anyone can cancel any proposal
    //
    // The secure version only allows proposer or admin to cancel.
    // This vulnerable version may allow any member to grief by
    // cancelling legitimate proposals.
    //
    // Fix: Check that canceller is proposer or admin.
    pub fn cancel_proposal(ctx: Context<CancelProposal>) -> Result<()> {
        ctx.accounts.cancel_proposal()
    }

    // VULNERABILITY [CRITICAL]: No admin check on pause
    //
    // The secure version only allows the creator (admin) to pause.
    // This vulnerable version may allow any member to pause the
    // entire multisig, causing a denial of service.
    //
    // Fix: Verify signer is the creator/admin before toggling pause.
    pub fn toggle_pause(ctx: Context<TogglePause>) -> Result<()> {
        ctx.accounts.toggle_pause()
    }
}
