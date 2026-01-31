use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use crate::{state::*, errors::*, constants::*};

// Execute Transfer Proposal Instruction
//
// Executes a TransferSol proposal that has reached threshold and passed timelock
// Uses UncheckedAccount for recipient to avoid remaining_accounts validation issues
//
// Security checks:
// 1. Pause check
// 2. Proposal exists and is Active
// 3. Threshold reached
// 4. Timelock passed
// 5. Not expired
// 6. TransferProposal matches Proposal
// 7. Recipient validation (writable, system-owned)
// 8. Vault has sufficient balance

#[derive(Accounts)]
pub struct ExecuteTransferProposal<'info> {
    // Executor - must be Admin or Executor role
    #[account(mut)]
    pub executor: Signer<'info>,

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
    // Security: Rent refunded to proposer (who paid to create it)
    #[account(
        mut,
        seeds = [
            TRANSFER_PROPOSAL,
            multisig_account.key().as_ref(),
            &transfer_proposal.proposal_id.to_le_bytes(),
        ],
        bump = transfer_proposal.bump,
        has_one = proposer @ MultisigError::NotProposer,
        close = proposer,
    )]
    pub transfer_proposal: Account<'info, TransferProposal>,

    // Proposer - who created and paid for the proposal
    // Security: Receives rent refund when proposal is closed
    /// CHECK: Validated by has_one constraint on transfer_proposal
    #[account(mut)]
    pub proposer: UncheckedAccount<'info>,

    // Vault PDA (holds the SOL)
    #[account(
        mut,
        seeds = [
            VAULT,
            multisig_account.key().as_ref(),
        ],
        bump = multisig_account.vault_bump,
    )]
    pub vault: SystemAccount<'info>,

    // Recipient of funds - UncheckedAccount with manual validation
    /// CHECK: This account is validated manually in the instruction logic
    /// We verify it is writable and system-owned before transfer
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> ExecuteTransferProposal<'info> {
    pub fn execute_transfer_proposal(&mut self) -> Result<()> {
        // SECURITY CHECKS

        // 1. Pause Check
        require!(
            !self.multisig_account.paused,
            MultisigError::MultisigPaused
        );

        // 2. Executor Permission Check
        // Only Admin or Executor can execute proposals
        require!(
            self.multisig_account.can_execute(&self.executor.key()),
            MultisigError::CannotExecute
        );

        // 3. Proposal Status Check
        require!(
            self.transfer_proposal.status == ProposalStatus::Active,
            MultisigError::ProposalNotActive
        );

       

        // 5. Threshold Check
        require!(
            self.transfer_proposal.approval_count >= self.multisig_account.threshold,
            MultisigError::InsufficientApprovals
        );

        // 6. Timelock Check
        let clock = Clock::get()?;
        require!(
            self.transfer_proposal.timelock_passed(clock.unix_timestamp, self.multisig_account.timelock_seconds),
            MultisigError::TimelockNotPassed
        );

        // 7. Expiry Check
        require!(
            !self.transfer_proposal.is_expired(clock.unix_timestamp),
            MultisigError::ProposalExpired
        );

        // 8. Recipient Validation
        // Ensure recipient is writable (already checked by #[account(mut)])
        // Ensure recipient is system-owned to prevent sending to PDAs without proper handling
        require!(
            self.recipient.owner == &anchor_lang::system_program::ID,
            MultisigError::InvalidRecipient
        );

        // 9. Recipient Not Default
        require!(
            self.transfer_proposal.recipient == self.recipient.key(),
            MultisigError::InvalidRecipient
        );

        // 10. Vault Balance Check
        let vault_balance = self.vault.lamports();
        require!(
            vault_balance >= self.transfer_proposal.amount,
            MultisigError::InsufficientFunds
        );

        // Execute the transfer
        let multisig_key = self.multisig_account.key();
        let vault_seeds = &[
            VAULT,
            multisig_key.as_ref(),
            &[self.multisig_account.vault_bump],
        ];
        let signer_seeds = &[&vault_seeds[..]];

        let cpi_context = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            Transfer {
                from: self.vault.to_account_info(),
                to: self.recipient.to_account_info(),
            },
            signer_seeds,
        );

        transfer(cpi_context, self.transfer_proposal.amount)?;

        // Update proposal state
        self.transfer_proposal.status = ProposalStatus::Executed;
        self.transfer_proposal.executed_at = clock.unix_timestamp;

        // Update multisig state
        self.multisig_account.last_executed_proposal = self.transfer_proposal.proposal_id;

        Ok(())
    }
}
