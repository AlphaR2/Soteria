use anchor_lang::prelude::*;
use crate::{state::*, constants::*};

// Create Proposal Instruction - VULNERABLE VERSION
//
// Allows creation of governance proposals (AddMember, RemoveMember, etc.)
// with multiple critical security vulnerabilities.

#[derive(Accounts)]
pub struct CreateProposal<'info> {
    // VULNERABILITY [CRITICAL]: No member/role validation on proposer
    //
    // The secure version validates:
    // - proposer is a member of the multisig
    // - proposer has Admin or Proposer role
    //
    // This vulnerable version allows ANYONE to create proposals,
    // including complete strangers to the multisig.
    //
    // Example Attack:
    //   1. Attacker sees Alice's multisig on-chain
    //   2. Attacker calls create_proposal with their own keypair
    //   3. Attacker proposes to add themselves as Admin
    //   4. If threshold is low or other vulnerabilities exist,
    //      attacker becomes Admin and takes over
    #[account(mut)]
    pub proposer: Signer<'info>,

    // VULNERABILITY [MEDIUM]: No ownership verification on multisig
    //
    // While Anchor's Account<'info, Multisig> verifies discriminator,
    // there's no explicit check that this multisig belongs to
    // the expected program. A fake multisig account could be passed.
    #[account(
        mut,
        seeds = [
            MULTISIG,
            multisig_account.creator.as_ref(),
            &multisig_account.multisig_id.to_le_bytes(),
        ],
        bump = multisig_account.bump,
    )]
    pub multisig_account: Account<'info, Multisig>,

    #[account(
        init,
        payer = proposer,
        space = ANCHOR_DISCRIMINATOR + Proposal::INIT_SPACE,
        seeds = [
            PROPOSAL,
            multisig_account.key().as_ref(),
            &multisig_account.proposal_count.to_le_bytes(),
        ],
        bump,
    )]
    pub proposal: Account<'info, Proposal>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreateProposal<'info> {
    pub fn create_proposal(
        &mut self,
        proposal_type: ProposalType,
        bumps: &CreateProposalBumps,
    ) -> Result<()> {
        // VULNERABILITY [CRITICAL]: Missing pause check
        //
        // The secure version validates: !multisig.paused
        // Without this check, proposals can be created even when
        // the multisig is supposed to be frozen during emergencies.
        //
        // Fix: require!(!self.multisig_account.paused, MultisigError::MultisigPaused);


        // VULNERABILITY [CRITICAL]: Missing member validation
        //
        // The secure version validates: multisig.is_member(&proposer.key())
        // Without this, anyone can create proposals for any multisig.
        //
        // Fix: require!(self.multisig_account.is_member(&self.proposer.key()), MultisigError::NotAMember);


        // VULNERABILITY [CRITICAL]: Missing role permission check
        //
        // The secure version validates: multisig.can_propose(&proposer.key())
        // Only Admin or Proposer roles should create proposals.
        // Without this, Executor role members (who should only execute) can propose.
        //
        // Fix: require!(self.multisig_account.can_propose(&self.proposer.key()), MultisigError::CannotPropose);


        // VULNERABILITY [CRITICAL]: No proposal type specific validation
        //
        // The secure version validates each proposal type:
        //
        // AddMember:
        // - Only admin can add members
        // - Cannot add yourself (prevents self-invitation)
        // - Cannot add existing member (no duplicates)
        // - Cannot add default pubkey
        // - owner_count < MAX_OWNERS
        //
        // RemoveMember:
        // - Only admin can remove members
        // - Cannot remove creator
        // - Member must exist
        // - owner_count > 1 (at least 1 member)
        // - threshold <= new_owner_count after removal
        //
        // ChangeThreshold:
        // - new_threshold >= 1
        // - new_threshold <= owner_count
        //
        // ChangeTimelock:
        // - Only admin can change
        // - Reasonable bounds (e.g., <= 2 days)
        //
        // THIS VULNERABLE VERSION SKIPS ALL THESE CHECKS!
        //
        // Example Attack (Add Duplicate Member):
        //   1. Multisig has Alice and Bob
        //   2. Attacker proposes AddMember(Alice)
        //   3. Without duplicate check, Alice is added twice
        //   4. Alice now has 2 slots in the array
        //   5. When Alice approves, she might set 2 bits, inflating count
        //
        // Example Attack (Remove Creator):
        //   1. Attacker proposes RemoveMember(creator)
        //   2. Without creator protection, creator is removed
        //   3. No one left with Admin role for critical operations
        //   4. Multisig becomes ungovernable


        // Get proposer's index for auto-approval
        // VULNERABILITY [HIGH]: This fails if proposer is not a member
        //
        // The secure version checks membership first, then gets index.
        // Here, if proposer is not a member, this returns None and
        // the next line panics or uses wrong index.
        let proposer_index = self.multisig_account
            .member_index(&self.proposer.key())
            .unwrap_or(0); // VULNERABLE: Default to 0 if not found!


        // VULNERABILITY [MEDIUM]: Integer overflow on proposal_count
        //
        // The secure version uses checked_add to prevent overflow.
        // Without checked arithmetic, proposal_count could overflow
        // and wrap around to 0, overwriting old proposals.
        //
        // Fix: Use checked_add and return Overflow error.
        self.multisig_account.proposal_count += 1; // VULNERABLE: Unchecked add

        let proposal_id = self.multisig_account.proposal_count - 1;

        // Initialize proposal with auto-approval from proposer
        let mut approval_bitmap: u64 = 0;
        approval_bitmap |= 1u64 << proposer_index;

        let clock = Clock::get()?;

        // VULNERABILITY [MEDIUM]: Overflow in expiry calculation
        //
        // The secure version uses checked arithmetic for:
        // expires_at = created_at + timelock + grace_period
        //
        // Without checks, this could overflow and produce a very small
        // expires_at, causing proposal to immediately expire.
        let expires_at = clock.unix_timestamp
            + self.multisig_account.timelock_seconds as i64
            + DEFAULT_EXPIRY_PERIOD as i64; // VULNERABLE: Unchecked add

        self.proposal.set_inner(Proposal {
            multisig: self.multisig_account.key(),
            proposal_id,
            proposer: self.proposer.key(),
            proposal_type,
            status: ProposalStatus::Active,
            approval_bitmap,
            approval_count: 1, // Proposer auto-approves
            created_at: clock.unix_timestamp,
            expires_at,
            executed_at: 0,
            bump: bumps.proposal,
        });

        Ok(())
    }
}
