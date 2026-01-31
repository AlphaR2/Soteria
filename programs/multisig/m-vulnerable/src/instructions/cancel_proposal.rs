use anchor_lang::prelude::*;
use crate::{state::*, constants::*};

// Cancel Proposal Instruction - VULNERABLE VERSION
//
// Allows cancelling proposals with security vulnerabilities.

#[derive(Accounts)]
pub struct CancelProposal<'info> {
    // VULNERABILITY [HIGH]: No authorization check on canceller
    //
    // The secure version validates that canceller is either:
    // - The original proposer (has right to retract their proposal)
    // - The admin/creator (has emergency override)
    //
    // Without this check, ANY signer can cancel ANY proposal,
    // enabling griefing attacks.
    //
    // Example Attack:
    //   1. Alice creates legitimate proposal to transfer funds
    //   2. 4/5 members approve, threshold almost met
    //   3. Attacker calls cancel_proposal
    //   4. Without authorization check, proposal is cancelled
    //   5. All approval work is lost, must start over
    //   6. Attacker repeats indefinitely (griefing DoS)
    pub canceller: Signer<'info>,

    #[account(
        seeds = [
            MULTISIG,
            multisig_account.creator.as_ref(),
            &multisig_account.multisig_id.to_le_bytes(),
        ],
        bump = multisig_account.bump,
    )]
    pub multisig_account: Account<'info, Multisig>,

    // VULNERABILITY [HIGH]: close = proposer without validating proposer exists
    //
    // The secure version has a separate proposer account that must match
    // proposal.proposer for rent refund. This vulnerable version closes
    // to a potentially wrong account.
    #[account(
        mut,
        seeds = [
            PROPOSAL,
            multisig_account.key().as_ref(),
            &proposal.proposal_id.to_le_bytes(),
        ],
        bump = proposal.bump,
        close = canceller, // VULNERABLE: Should validate and close to proposer
    )]
    pub proposal: Account<'info, Proposal>,
}

impl<'info> CancelProposal<'info> {
    pub fn cancel_proposal(&mut self) -> Result<()> {
        // VULNERABILITY [MEDIUM]: Missing proposal-multisig validation
        //
        // The secure version validates: proposal.multisig == multisig_account.key()
        // Without this, proposals from different multisigs could be cancelled.
        //
        // Fix: require!(self.proposal.multisig == self.multisig_account.key(), MultisigError::NotAMember);


        // VULNERABILITY [CRITICAL]: Missing proposal status check
        //
        // The secure version validates: proposal.is_active()
        // Without this:
        // - Executed proposals could be "cancelled" (state corruption)
        // - Already cancelled proposals cancelled again (double close attack?)
        //
        // Fix: require!(self.proposal.is_active(), MultisigError::ProposalNotActive);


        // VULNERABILITY [CRITICAL]: Missing authorization check
        //
        // The secure version validates:
        // let is_proposer = canceller.key() == proposal.proposer;
        // let is_creator = canceller.key() == multisig_account.creator;
        // require!(is_proposer || is_creator, MultisigError::NotProposer);
        //
        // Without this, anyone can cancel any proposal, completely
        // undermining the multisig governance process.
        //
        // Example Attack Scenario:
        //   - Critical security update proposal created
        //   - Attacker (competitor or disgruntled employee) cancels it
        //   - Security vulnerability remains unpatched
        //   - Attacker exploits the unpatched vulnerability
        //
        // Fix: require!(is_proposer || is_creator, MultisigError::NotProposer);


        // Mark as cancelled
        // The secure version does this BEFORE closing to prevent race conditions
        self.proposal.status = ProposalStatus::Cancelled;

        // Account closed by Anchor (close = canceller)
        // VULNERABILITY: Rent goes to canceller, not original proposer

        Ok(())
    }
}
