use anchor_lang::prelude::*;
use crate::{state::*, errors::*, constants::*};

// Create Transfer Proposal Instruction
//
// Creates a SOL transfer proposal. 
// This is separate from governance proposals for cleaner architecture
//
// Flow:
// 1. Creates TransferProposal (for transfer-specific data)

#[derive(Accounts)]
pub struct CreateTransferProposal<'info> {
    // Proposer - must be Admin or Proposer role
    #[account(mut)]
    pub proposer: Signer<'info>,

    // Multisig account
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


    // Transfer Proposal PDA - linked to base proposal
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
        // SECURITY CHECKS

        // 1. Pause Check
        require!(
            !self.multisig_account.paused,
            MultisigError::MultisigPaused
        );

        // 2. Member Validation
        require!(
            self.multisig_account.is_member(&self.proposer.key()),
            MultisigError::NotAMember
        );

        // 3. Role Permission Check
        // Only Admin or Proposer can create transfer proposals
        require!(
            self.multisig_account.can_propose(&self.proposer.key()),
            MultisigError::CannotPropose
        );

        // 4. Recipient Validation
        require!(
            recipient != Pubkey::default(),
            MultisigError::InvalidRecipient
        );

        // 5. Amount Validation
        require!(amount > 0, MultisigError::InvalidParameter);

        // Get proposer's index for auto-approval
        let proposer_index = self.multisig_account
            .member_index(&self.proposer.key())
            .ok_or(MultisigError::NotAMember)?;

        // 6. Increment Proposal Count
        self.multisig_account.proposal_count = self
            .multisig_account
            .proposal_count
            .checked_add(1)
            .ok_or(MultisigError::Overflow)?;

        let proposal_id = self.multisig_account.proposal_count - 1;

        // 7. Initialize Base Proposal
        let mut approval_bitmap: u64 = 0;
        approval_bitmap |= 1u64 << proposer_index;

        let clock = Clock::get()?;

        // Calculate expiry
        let expires_at = clock
            .unix_timestamp
            .checked_add(self.multisig_account.timelock_seconds as i64)
            .and_then(|t| t.checked_add(DEFAULT_EXPIRY_PERIOD as i64))
            .ok_or(MultisigError::Overflow)?;

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
            amount, 
            recipient, 
            bump: bumps.transfer_proposal 
        });

        Ok(())
    }
}
