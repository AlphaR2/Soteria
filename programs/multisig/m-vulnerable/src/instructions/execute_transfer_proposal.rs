use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use crate::{state::*,  constants::*};

// Execute Transfer Proposal Instruction - VULNERABLE VERSION
//
// Executes SOL transfers from vault with critical security vulnerabilities.

#[derive(Accounts)]
pub struct ExecuteTransferProposal<'info> {
    // VULNERABILITY [CRITICAL]: No executor role validation
    #[account(mut)]
    pub executor: Signer<'info>,

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

    // VULNERABILITY [HIGH]: close = executor steals rent from proposer
    #[account(
        mut,
        seeds = [
            TRANSFER_PROPOSAL,
            multisig_account.key().as_ref(),
            &transfer_proposal.proposal_id.to_le_bytes(),
        ],
        bump = transfer_proposal.bump,
        close = executor, // VULNERABLE: Should be proposer
    )]
    pub transfer_proposal: Account<'info, TransferProposal>,

    #[account(
        mut,
        seeds = [
            VAULT,
            multisig_account.key().as_ref(),
        ],
        bump = multisig_account.vault_bump,
    )]
    pub vault: SystemAccount<'info>,

    // VULNERABILITY [CRITICAL]: Recipient not properly validated
    //
    // The secure version:
    // 1. Uses UncheckedAccount but manually validates
    // 2. Checks recipient is system-owned (not a PDA)
    // 3. Checks recipient matches transfer_proposal.recipient
    //
    // This vulnerable version accepts any account as recipient,
    // allowing fund redirection attacks.
    //
    // Example Attack:
    //   1. Proposal created to send 100 SOL to Alice
    //   2. Attacker calls execute with their own address as recipient
    //   3. Without recipient validation, 100 SOL goes to attacker
    //   4. Alice never receives funds
    /// CHECK: Recipient validation is intentionally missing for educational purposes
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> ExecuteTransferProposal<'info> {
    pub fn execute_transfer_proposal(&mut self) -> Result<()> {
        // VULNERABILITY [CRITICAL]: Missing pause check
        //
        // Fix: require!(!self.multisig_account.paused, MultisigError::MultisigPaused);


        // VULNERABILITY [CRITICAL]: Missing executor permission check
        //
        // Fix: require!(self.multisig_account.can_execute(&self.executor.key()), MultisigError::CannotExecute);


        // VULNERABILITY [CRITICAL]: Missing proposal status check
        //
        // Fix: require!(self.transfer_proposal.status == ProposalStatus::Active, MultisigError::ProposalNotActive);


        // VULNERABILITY [CRITICAL]: Missing threshold check
        //
        // The most critical vulnerability! Without this check,
        // transfers execute with insufficient approvals.
        //
        // Example Attack:
        //   1. Vault has 1000 SOL
        //   2. Threshold is 5
        //   3. Attacker creates transfer proposal for 1000 SOL
        //   4. Only attacker approves (1 approval)
        //   5. Attacker executes (no threshold check)
        //   6. All 1000 SOL stolen with 1/5 approvals
        //
        // Fix: require!(self.transfer_proposal.approval_count >= self.multisig_account.threshold, MultisigError::InsufficientApprovals);


        // VULNERABILITY [CRITICAL]: Missing timelock check
        //
        // Fix: require!(self.transfer_proposal.timelock_passed(clock.unix_timestamp, self.multisig_account.timelock_seconds), MultisigError::TimelockNotPassed);


        // VULNERABILITY [HIGH]: Missing expiry check
        //
        // Fix: require!(!self.transfer_proposal.is_expired(clock.unix_timestamp), MultisigError::ProposalExpired);


        // VULNERABILITY [CRITICAL]: No recipient validation
        //
        // The secure version validates:
        // 1. recipient.owner == system_program::ID (system-owned, not PDA)
        // 2. recipient.key() == transfer_proposal.recipient (matches stored)
        //
        // Without these checks, anyone can redirect funds!
        //
        // Fix: require!(self.recipient.owner == &anchor_lang::system_program::ID, MultisigError::InvalidRecipient);
        // Fix: require!(self.transfer_proposal.recipient == self.recipient.key(), MultisigError::InvalidRecipient);


        // VULNERABILITY [HIGH]: No vault balance check
        //
        // The secure version validates: vault.lamports >= amount
        // Without this, the transfer CPI will fail with insufficient funds,
        // but better to check upfront for clear error messages.
        //
        // Fix: require!(self.vault.lamports() >= self.transfer_proposal.amount, MultisigError::InsufficientFunds);


        // Execute the transfer
        // This part is actually secure (proper PDA signing)
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

        // VULNERABILITY [MEDIUM]: Transfer amount from proposal, not validated
        //
        // While the transfer itself works, the amount was never validated
        // at proposal creation time. Combined with recipient substitution,
        // this allows complete fund drainage.
        transfer(cpi_context, self.transfer_proposal.amount)?;

        // Update proposal state
        self.transfer_proposal.status = ProposalStatus::Executed;
        self.transfer_proposal.executed_at = Clock::get()?.unix_timestamp;

        // Update multisig state
        self.multisig_account.last_executed_proposal = self.transfer_proposal.proposal_id;

        Ok(())
    }
}
