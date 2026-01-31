use anchor_lang::prelude::*;
use crate::{state::*, errors::*, constants::*};

// Approve Proposal Instruction
//
// Allows an owner to approve a pending proposal.
// Uses bitmap to efficiently track which owners have approved.
// Each owner can only approve once per proposal.
//
// When approval_count reaches threshold, proposal can be executed.

#[derive(Accounts)]
pub struct ApproveProposal<'info> {
    // Owner approving the proposal
    // Must be an existing owner of the multisig
    #[account(mut)]
    pub owner: Signer<'info>,

    // Multisig account - needed for owner validation
    #[account(
        seeds = [
            MULTISIG,
            multisig_account.creator.as_ref(),
            &multisig_account.multisig_id.to_le_bytes(),
        ],
        bump = multisig_account.bump,
    )]
    pub multisig_account: Account<'info, Multisig>,

    // Proposal being approved
    // Must be active and owned by this program
    #[account(
        mut,
        seeds = [
            PROPOSAL,
            multisig_account.key().as_ref(),
            &proposal.proposal_id.to_le_bytes(),
        ],
        bump = proposal.bump,
    )]
    pub proposal: Account<'info, Proposal>,
}

impl<'info> ApproveProposal<'info> {
    pub fn approve_proposal(&mut self) -> Result<()> {
        // SECURITY CHECKS

        // 1. Pause Check
        // Multisig must not be paused
        require!(
            !self.multisig_account.paused,
            MultisigError::MultisigPaused
        );

        // 2. Proposal-Multisig Relationship Validation
        // Ensures proposal belongs to the provided multisig
        // Prevents approving proposals from different multisig wallets
        require!(
            self.proposal.multisig == self.multisig_account.key(),
            MultisigError::NotAMember
        );

        // 3. Member Validation
        // Only existing members can approve proposals
        // Prevents external actors from manipulating approval count
        require!(
            self.multisig_account.is_member(&self.owner.key()),
            MultisigError::NotAMember
        );

        // Get member's index for bitmap manipulation
        let owner_index = self
            .multisig_account
            .member_index(&self.owner.key())
            .ok_or(MultisigError::NotAMember)?;

        // 4. Proposal Status Check
        // Only active proposals can receive approvals
        // Prevents re-approving executed or cancelled proposals
        require!(
            self.proposal.is_active(),
            MultisigError::ProposalNotActive
        );

        // 5. Double Approval Check
        // Each member can only approve once using bitmap
        // Prevents approval count manipulation
        require!(
            !self.proposal.has_approved(owner_index),
            MultisigError::AlreadyApproved
        );

        // 6. Member Index Bounds Check
        // Redundant safety check (has_approved also checks)
        // Prevents out-of-bounds bitmap access
        require!(
            owner_index < MAX_OWNERS,
            MultisigError::NotAMember
        );

        // 7. Record Approval Using Bitmap
        // Set the bit at owner_index position
        // This is atomic and prevents double-approval
        let success = self.proposal.approve(owner_index);
        require!(success, MultisigError::AlreadyApproved);

        // 8. Approval Count Overflow Check
        // The approve() method increments approval_count
        // Verify it hasn't overflowed (should never happen with proper owner_count)
        require!(
            self.proposal.approval_count <= self.multisig_account.owner_count,
            MultisigError::Overflow
        );

        Ok(())
    }
}
