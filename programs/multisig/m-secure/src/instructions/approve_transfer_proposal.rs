use anchor_lang::prelude::*;
use crate::{state::*, errors::*, constants::*};

// Approve Transfer Proposal Instruction
//
// Allows members to approve a TransferSol proposal
// Each member can only approve once (tracked via bitmap)
// When approval_count reaches threshold, proposal can be executed

#[derive(Accounts)]
pub struct ApproveTransferProposal<'info> {
    // Member approving the proposal
    #[account(mut)]
    pub owner: Signer<'info>,

    // Multisig account (for member validation)
    #[account(
        seeds = [
            MULTISIG,
            multisig_account.creator.as_ref(),
            &multisig_account.multisig_id.to_le_bytes(),
        ],
        bump = multisig_account.bump,
    )]
    pub multisig_account: Account<'info, Multisig>,

    // Transfer Proposal being approved
    #[account(
        mut,
        seeds = [
            TRANSFER_PROPOSAL,
            multisig_account.key().as_ref(),
            &transfer_proposal.proposal_id.to_le_bytes(),
        ],
        bump = transfer_proposal.bump,
        constraint = transfer_proposal.multisig == multisig_account.key() @ MultisigError::InvalidProposal,
    )]
    pub transfer_proposal: Account<'info, TransferProposal>,
}

impl<'info> ApproveTransferProposal<'info> {
    pub fn approve_transfer_proposal(&mut self) -> Result<()> {
        // SECURITY CHECKS

        // 1. Pause Check
        require!(
            !self.multisig_account.paused,
            MultisigError::MultisigPaused
        );

        // 2. Proposal-Multisig Relationship Validation
        // Already checked by constraint, but defensive programming
        require!(
            self.transfer_proposal.multisig == self.multisig_account.key(),
            MultisigError::InvalidProposal
        );

        // 3. Member Validation
        // Only existing members can approve proposals
        require!(
            self.multisig_account.is_member(&self.owner.key()),
            MultisigError::NotAMember
        );

        // 4. Get Member Index for Bitmap
        let owner_index = self.multisig_account
            .member_index(&self.owner.key())
            .ok_or(MultisigError::NotAMember)?;

        // 5. Proposal Status Check
        // Only active proposals can be approved
        require!(
            self.transfer_proposal.is_active(),
            MultisigError::ProposalNotActive
        );

        // 6. Double Approval Prevention
        // Each member can only approve once
        require!(
            !self.transfer_proposal.has_approved(owner_index),
            MultisigError::AlreadyApproved
        );

        // 7. Member Index Bounds Check
        require!(
            owner_index < MAX_OWNERS,
            MultisigError::Overflow
        );

        // 8. Record Approval
        // Updates bitmap and increments approval_count atomically
        self.transfer_proposal.approve(owner_index);

        // 9. Approval Count Sanity Check
        // approval_count should never exceed owner_count
        require!(
            self.transfer_proposal.approval_count <= self.multisig_account.owner_count,
            MultisigError::Overflow
        );

        Ok(())
    }
}
