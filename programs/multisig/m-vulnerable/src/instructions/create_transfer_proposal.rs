use anchor_lang::prelude::*;
use crate::{state::*,  constants::*};

// Create Transfer Proposal Instruction - VULNERABLE VERSION
//
// Creates SOL transfer proposals with critical security vulnerabilities.

#[derive(Accounts)]
pub struct CreateTransferProposal<'info> {
    // VULNERABILITY [CRITICAL]: No member/role validation
    //
    // Same as create_proposal - anyone can create transfer proposals.
    #[account(mut)]
    pub proposer: Signer<'info>,

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
        space = ANCHOR_DISCRIMINATOR + TransferProposal::INIT_SPACE,
        seeds = [
            TRANSFER_PROPOSAL,
            multisig_account.key().as_ref(),
            &multisig_account.proposal_count.to_le_bytes(),
        ],
        bump,
    )]
    pub transfer_proposal: Account<'info, TransferProposal>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreateTransferProposal<'info> {
    pub fn create_transfer_proposal(
        &mut self,
        amount: u64,
        recipient: Pubkey,
        bumps: &CreateTransferProposalBumps,
    ) -> Result<()> {
        // VULNERABILITY [CRITICAL]: Missing pause check
        //
        // Fix: require!(!self.multisig_account.paused, MultisigError::MultisigPaused);


        // VULNERABILITY [CRITICAL]: Missing member validation
        //
        // Fix: require!(self.multisig_account.is_member(&self.proposer.key()), MultisigError::NotAMember);


        // VULNERABILITY [CRITICAL]: Missing role permission check
        //
        // Fix: require!(self.multisig_account.can_propose(&self.proposer.key()), MultisigError::CannotPropose);


        // VULNERABILITY [HIGH]: No recipient validation
        //
        // The secure version validates: recipient != Pubkey::default()
        // Without this, funds could be sent to the default pubkey (all zeros),
        // which is effectively burning them.
        //
        // Example Attack:
        //   1. Attacker creates transfer proposal with recipient = default
        //   2. Proposal is approved (maybe by social engineering)
        //   3. Execution sends SOL to 11111...11111 (default pubkey)
        //   4. Funds are lost forever
        //
        // Fix: require!(recipient != Pubkey::default(), MultisigError::InvalidRecipient);


        // VULNERABILITY [HIGH]: No amount validation
        //
        // The secure version validates: amount > 0
        // While 0-amount transfers are harmless, they waste gas
        // and could be used for griefing/spamming.
        //
        // More importantly, there's no check that vault has enough funds.
        // This is checked at execution time in secure version.
        //
        // Fix: require!(amount > 0, MultisigError::InvalidParameter);


        // Get proposer index (same vulnerability as create_proposal)
        let proposer_index = self.multisig_account
            .member_index(&self.proposer.key())
            .unwrap_or(0); // VULNERABLE: Non-members get index 0


        // VULNERABILITY [MEDIUM]: Integer overflow on proposal_count
        self.multisig_account.proposal_count += 1; // VULNERABLE: Unchecked add

        let proposal_id = self.multisig_account.proposal_count - 1;

        let mut approval_bitmap: u64 = 0;
        approval_bitmap |= 1u64 << proposer_index;

        let clock = Clock::get()?;

        // VULNERABILITY [MEDIUM]: Overflow in expiry calculation
        let expires_at = clock.unix_timestamp
            + self.multisig_account.timelock_seconds as i64
            + DEFAULT_EXPIRY_PERIOD as i64; // VULNERABLE: Unchecked add

        self.transfer_proposal.set_inner(TransferProposal {
            multisig: self.multisig_account.key(),
            proposal_id,
            proposer: self.proposer.key(),
            status: ProposalStatus::Active,
            approval_bitmap,
            approval_count: 1,
            created_at: clock.unix_timestamp,
            expires_at,
            executed_at: 0,
            amount, // VULNERABLE: Unvalidated amount
            recipient, // VULNERABLE: Unvalidated recipient
            bump: bumps.transfer_proposal,
        });

        Ok(())
    }
}
