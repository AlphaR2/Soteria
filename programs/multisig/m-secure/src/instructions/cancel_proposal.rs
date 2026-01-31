use anchor_lang::prelude::*;
use crate::{state::*, errors::*, constants::*};

// Cancel Proposal Instruction
//
// Allows the proposer or creator to cancel an active proposal.
// Only active proposals can be cancelled.
// Proposal account is closed and rent returned to the proposer.
//
// Security: Only proposer or creator can cancel to prevent griefing attacks.

#[derive(Accounts)]
pub struct CancelProposal<'info> {
    // Canceller - must be proposer or creator
    pub canceller: Signer<'info>,

    // Multisig account - needed for creator validation
    #[account(
        seeds = [
            MULTISIG,
            multisig_account.creator.as_ref(),
            &multisig_account.multisig_id.to_le_bytes(),
        ],
        bump = multisig_account.bump,
    )]
    pub multisig_account: Account<'info, Multisig>,

    // Proposal being cancelled
    // Rent returned to proposer (who created and paid for it)
    #[account(
        mut,
        seeds = [
            PROPOSAL,
            multisig_account.key().as_ref(),
            &proposal.proposal_id.to_le_bytes(),
        ],
        bump = proposal.bump,
        close = proposer,
    )]
    pub proposal: Account<'info, Proposal>,

    // Proposer account - receives rent refund
    // Must be mutable to receive lamports
    #[account(mut)]
    pub proposer: SystemAccount<'info>,
}

impl<'info> CancelProposal<'info> {
    pub fn cancel_proposal(&mut self) -> Result<()> {
        // SECURITY CHECKS

        // 1. Proposal-Multisig Relationship Validation
        // Ensures proposal belongs to this multisig
        require!(
            self.proposal.multisig == self.multisig_account.key(),
            MultisigError::NotAMember
        );

        // 2. Proposal Status Check
        // Only active proposals can be cancelled
        // Prevents cancelling already-executed or already-cancelled proposals
        require!(
            self.proposal.is_active(),
            MultisigError::ProposalNotActive
        );

        // 3. Authorization Check
        // Only proposer or creator can cancel
        // Proposer: owns the proposal, has right to retract
        // Creator: has emergency override for governance
        let is_proposer = self.canceller.key() == self.proposal.proposer;
        let is_creator = self.canceller.key() == self.multisig_account.creator;

        require!(
            is_proposer || is_creator,
            MultisigError::NotProposer
        );

        // 4. Mark Proposal as Cancelled
        // Prevents race conditions where proposal gets executed during cancellation
        self.proposal.status = ProposalStatus::Cancelled;

        // Proposal account automatically closed by Anchor (close = proposer)
        // Rent returned to original proposer who paid for creation

        Ok(())
    }
}
