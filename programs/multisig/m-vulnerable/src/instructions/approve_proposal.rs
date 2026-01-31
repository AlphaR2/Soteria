use anchor_lang::prelude::*;
use crate::{state::*,  constants::*};

// Approve Proposal Instruction - VULNERABLE VERSION
//
// Allows approving governance proposals with critical security vulnerabilities.

#[derive(Accounts)]
pub struct ApproveProposal<'info> {
    // VULNERABILITY [CRITICAL]: No member validation on owner
    //
    // The secure version validates that owner is a member of the multisig.
    // This vulnerable version allows anyone to call approve, even non-members.
    //
    // Example Attack:
    //   1. Multisig has threshold of 3, with 3 members
    //   2. Only 1 member approves legitimately
    //   3. Attacker (non-member) calls approve twice with their key
    //   4. Without member check, approval_count reaches 3
    //   5. Malicious proposal executes with only 1 real approval
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

    // VULNERABILITY [HIGH]: Proposal-multisig relationship not validated
    //
    // The secure version validates: proposal.multisig == multisig_account.key()
    // Without this, an attacker could:
    // 1. Create multisig A with threshold 1
    // 2. Create multisig B with threshold 5
    // 3. Create proposal in multisig B (needs 5 approvals)
    // 4. Approve it via multisig A context (only needs 1)
    // 5. Execute with insufficient real approvals
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
        // VULNERABILITY [CRITICAL]: Missing pause check
        //
        // The secure version validates: !multisig.paused
        // Without this, approvals continue during emergencies.
        //
        // Fix: require!(!self.multisig_account.paused, MultisigError::MultisigPaused);


        // VULNERABILITY [CRITICAL]: Missing proposal-multisig validation
        //
        // The secure version validates: proposal.multisig == multisig_account.key()
        // Without this, proposals from different multisigs could be approved.
        //
        // Fix: require!(self.proposal.multisig == self.multisig_account.key(), MultisigError::NotAMember);


        // VULNERABILITY [CRITICAL]: Missing member validation
        //
        // The secure version validates: multisig.is_member(&owner.key())
        // Without this, non-members can approve and inflate approval count.
        //
        // Fix: require!(self.multisig_account.is_member(&self.owner.key()), MultisigError::NotAMember);


        // VULNERABILITY [HIGH]: Using wrong index for non-members
        //
        // If owner is not a member, member_index returns None.
        // Using unwrap_or(0) means the non-member's approval goes to index 0,
        // potentially overwriting or duplicating the first member's approval.
        let owner_index = self.multisig_account
            .member_index(&self.owner.key())
            .unwrap_or(0); // VULNERABLE: Non-members get index 0


        // VULNERABILITY [CRITICAL]: Missing proposal status check
        //
        // The secure version validates: proposal.is_active()
        // Without this, cancelled or executed proposals can be re-approved,
        // potentially enabling double-execution or resurrection attacks.
        //
        // Example Attack:
        //   1. Proposal A is executed successfully
        //   2. Attacker calls approve on executed Proposal A
        //   3. Without status check, approval_count increases
        //   4. If combined with missing execution status check,
        //      proposal could execute again
        //
        // Fix: require!(self.proposal.is_active(), MultisigError::ProposalNotActive);


        // VULNERABILITY [CRITICAL]: No double approval prevention
        //
        // The secure version:
        // 1. Checks has_approved(owner_index) BEFORE approving
        // 2. Returns error if already approved
        //
        // This vulnerable version calls approve() which has the check,
        // but DOESN'T CHECK THE RETURN VALUE!
        //
        // Example Attack:
        //   1. Alice approves (returns true, approval_count = 1)
        //   2. Alice approves again (returns false, but we don't check)
        //   3. Alice approves again...
        //   4. Wait, the approve() method DOES increment, but only if not already approved
        //   5. Actually the real vulnerability is if approve() is poorly implemented
        //
        // The REAL vulnerability here is: we don't error on failed approval.
        // The secure version does: require!(success, MultisigError::AlreadyApproved);
        let _success = self.proposal.approve(owner_index);
        // VULNERABLE: Not checking if _success is false!


        // VULNERABILITY [MEDIUM]: No bounds check on owner_index
        //
        // The secure version validates: owner_index < MAX_OWNERS
        // While approve() has internal bounds check, defense in depth
        // requires checking at multiple layers.
        //
        // Fix: require!(owner_index < MAX_OWNERS, MultisigError::NotAMember);


        // VULNERABILITY [LOW]: No approval count sanity check
        //
        // The secure version validates: approval_count <= owner_count
        // This catches bugs where approval_count somehow exceeds members.
        //
        // Fix: require!(self.proposal.approval_count <= self.multisig_account.owner_count, MultisigError::Overflow);


        Ok(())
    }
}
