use anchor_lang::prelude::*;
use crate::{state::*, errors::*, constants::*};

// Execute Proposal Instruction
//
// Executes an approved governance proposal once threshold is reached.
// Handles governance proposal types only:
// - AddMember: Add new member with role to multisig
// - RemoveMember: Remove existing member from multisig
// - ChangeThreshold: Update approval threshold
// - ChangeTimelock: Update timelock duration
//
// TransferSol proposals use execute_transfer_proposal instead.
//
// Closes the proposal account after successful execution (rent returned to proposer).

#[derive(Accounts)]
pub struct ExecuteProposal<'info> {
    // Executor - any account can execute if threshold is met
    // Receives rent from closed proposal account
    #[account(mut)]
    pub executor: Signer<'info>,

    // Multisig account - will be modified for governance proposals
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

    // Proposal being executed
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

    // Proposer - who created and paid for the proposal
    
    /// CHECK: Validated by has_one constraint on transfer_proposal
    #[account(mut)]
    pub proposer: UncheckedAccount<'info>,
}

impl<'info> ExecuteProposal<'info> {
    pub fn execute_proposal(&mut self) -> Result<()> {
        // SECURITY CHECKS

        // 1. Pause Check
        // Multisig must not be paused (unless admin unpause)
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

        // 3. Proposal-Multisig Relationship Validation
        // Ensures proposal belongs to this multisig
        // Prevents executing proposals from other multisig wallets
        require!(
            self.proposal.multisig == self.multisig_account.key(),
            MultisigError::NotAMember
        );

        // 4. Proposal Status Check
        // Only active proposals can be executed
        // Prevents double-execution of already-executed proposals
        require!(
            self.proposal.is_active(),
            MultisigError::ProposalNotActive
        );

        // 5. Threshold Check
        // Proposal must have required number of approvals
        // Prevents premature execution
        require!(
            self.proposal.approval_count >= self.multisig_account.threshold,
            MultisigError::InsufficientApprovals
        );

        // 6. Approval Count Sanity Check
        // approval_count should never exceed owner_count
        // Defense against bitmap manipulation bugs
        require!(
            self.proposal.approval_count <= self.multisig_account.owner_count,
            MultisigError::Overflow
        );

        // 7. Timelock Check
        // Proposal must wait timelock duration before execution
        // Prevents immediate execution of potentially malicious proposals
        let clock = Clock::get()?;
        require!(
            self.proposal.timelock_passed(clock.unix_timestamp, self.multisig_account.timelock_seconds),
            MultisigError::TimelockNotPassed
        );

        // 8. Expiry Check
        // Proposal must not be expired
        // Prevents execution of stale proposals
        require!(
            !self.proposal.is_expired(clock.unix_timestamp),
            MultisigError::ProposalExpired
        );

      

        // Execute based on proposal type
        match self.proposal.proposal_type {
         
            ProposalType::AddMember { new_member, role } => {
                // 10. Already Member Check
                // Prevents duplicate member entries
                require!(
                    !self.multisig_account.is_member(&new_member),
                    MultisigError::AlreadyMember
                );

                // 11. Max Members Check
                // Ensure we haven't reached the fixed array limit
                require!(
                    self.multisig_account.owner_count < MAX_OWNERS as u8,
                    MultisigError::MaxMembersReached
                );

                // 12. New Member Validation
                // Prevent adding default pubkey as member
                require!(
                    new_member != Pubkey::default(),
                    MultisigError::InvalidParameter
                );

                // Add member to the next available slot with role
                let new_index = self.multisig_account.owner_count as usize;
                self.multisig_account.members[new_index] = Member {
                    pubkey: new_member,
                    role,
                };

                // Increment owner count with overflow check
                self.multisig_account.owner_count = self
                    .multisig_account
                    .owner_count
                    .checked_add(1)
                    .ok_or(MultisigError::Overflow)?;
            }

            ProposalType::RemoveMember { member_to_remove } => {
                // 13. Member Exists Check
                require!(
                    self.multisig_account.is_member(&member_to_remove),
                    MultisigError::NotAMember
                );

                // 14. Creator Protection
                // Creator cannot be removed for accountability
                require!(
                    member_to_remove != self.multisig_account.creator,
                    MultisigError::CannotRemoveCreator
                );

                // 15. Minimum Members Check
                // Must have at least 1 owner remaining
                require!(
                    self.multisig_account.owner_count > 1,
                    MultisigError::MinimumOneMember
                );

                // Find and remove owner
                let owner_index = self
                    .multisig_account
                    .member_index(&member_to_remove)
                    .ok_or(MultisigError::NotAMember)?;

                // Shift array left to fill the gap
                // This maintains compact owner list without holes
                let owner_count = self.multisig_account.owner_count as usize;
                for i in owner_index..owner_count - 1 {
                    self.multisig_account.members[i] = self.multisig_account.members[i + 1];
                }

                // Clear the last slot
                self.multisig_account.members[owner_count - 1] = Member::default();

                // Decrement owner count
                self.multisig_account.owner_count = self
                    .multisig_account
                    .owner_count
                    .checked_sub(1)
                    .ok_or(MultisigError::Overflow)?;

                // 16. Threshold Validation After Removal
                // Ensure threshold is still valid with new owner count
                require!(
                    self.multisig_account.is_valid_threshold(),
                    MultisigError::InvalidThreshold
                );
            }

            ProposalType::ChangeThreshold { new_threshold } => {
                // 17. Threshold Bounds Check
                require!(
                    new_threshold >= 1,
                    MultisigError::InvalidThreshold
                );

                // 18. Threshold vs Owner Count Check
                // New threshold cannot exceed current owner count
                require!(
                    new_threshold <= self.multisig_account.owner_count,
                    MultisigError::ThresholdExceedsOwners
                );

                // Update threshold
                self.multisig_account.threshold = new_threshold;
            }

            ProposalType::ChangeTimelock { new_timelock } => {
                // 19. Timelock Validation
                // Ensure reasonable timelock duration
                const MAX_TIMELOCK: u64 = 2 * 24 * 60 * 60; // 2 days
                require!(
                    new_timelock <= MAX_TIMELOCK,
                    MultisigError::InvalidParameter
                );

                // Update timelock
                self.multisig_account.timelock_seconds = new_timelock;
            }
        }

        // 20. Update last executed proposal
        // Track execution history
        self.multisig_account.last_executed_proposal = self.proposal.proposal_id;

        // 21. Mark Proposal as Executed
        // Prevents double-execution before account closure
        self.proposal.status = ProposalStatus::Executed;

        // Record execution timestamp (reuse clock from earlier)
        self.proposal.executed_at = clock.unix_timestamp;

        // Proposal account automatically closed by Anchor (close = executor)
        // Rent returned to executor as compensation for gas costs

        Ok(())
    }
}
