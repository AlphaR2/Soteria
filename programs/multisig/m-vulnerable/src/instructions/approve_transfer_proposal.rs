use anchor_lang::prelude::*;
use crate::{state::*,  constants::*};

// Approve Transfer Proposal Instruction - VULNERABLE VERSION
//
// Allows approving transfer proposals with the same vulnerabilities
// as approve_proposal.

#[derive(Accounts)]
pub struct ApproveTransferProposal<'info> {
    // VULNERABILITY [CRITICAL]: No member validation on owner
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        seeds = [
            MULTISIG,
            multisig_account.creator.as_ref(),
            &multisig_account.multisig_id.to_le_bytes(),
        ],
        bump = multisig_account.bump,
    )]
    pub multisig_account: Account<'info, Multisig>,

    // VULNERABILITY [HIGH]: Missing constraint to validate multisig relationship
    //
    // The secure version uses:
    // constraint = transfer_proposal.multisig == multisig_account.key() @ MultisigError::InvalidProposal
    //
    // Without this, transfer proposals from different multisigs could be approved.
    #[account(
        mut,
        seeds = [
            TRANSFER_PROPOSAL,
            multisig_account.key().as_ref(),
            &transfer_proposal.proposal_id.to_le_bytes(),
        ],
        bump = transfer_proposal.bump,
        // MISSING: constraint = transfer_proposal.multisig == multisig_account.key()
    )]
    pub transfer_proposal: Account<'info, TransferProposal>,
}

impl<'info> ApproveTransferProposal<'info> {
    pub fn approve_transfer_proposal(&mut self) -> Result<()> {
        // VULNERABILITY [CRITICAL]: Missing pause check
        //
        // Fix: require!(!self.multisig_account.paused, MultisigError::MultisigPaused);


        // VULNERABILITY [CRITICAL]: Missing proposal-multisig validation
        //
        // Fix: require!(self.transfer_proposal.multisig == self.multisig_account.key(), MultisigError::InvalidProposal);


        // VULNERABILITY [CRITICAL]: Missing member validation
        //
        // Fix: require!(self.multisig_account.is_member(&self.owner.key()), MultisigError::NotAMember);


        let owner_index = self.multisig_account
            .member_index(&self.owner.key())
            .unwrap_or(0); // VULNERABLE: Non-members get index 0


        // VULNERABILITY [CRITICAL]: Missing proposal status check
        //
        // Fix: require!(self.transfer_proposal.is_active(), MultisigError::ProposalNotActive);


        // VULNERABILITY [CRITICAL]: No double approval prevention enforcement
        //
        // The approve() method checks internally, but we don't validate
        // or error on its return value.
        let _success = self.transfer_proposal.approve(owner_index);
        // VULNERABLE: Not checking if _success is false!


        // VULNERABILITY [MEDIUM]: No bounds check on owner_index
        //
        // Fix: require!(owner_index < MAX_OWNERS, MultisigError::Overflow);


        // VULNERABILITY [LOW]: No approval count sanity check
        //
        // Fix: require!(self.transfer_proposal.approval_count <= self.multisig_account.owner_count, MultisigError::Overflow);


        Ok(())
    }
}
