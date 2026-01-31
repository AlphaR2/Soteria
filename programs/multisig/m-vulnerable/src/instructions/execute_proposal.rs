use anchor_lang::prelude::*;
use crate::{state::*,  constants::*};

// Execute Proposal Instruction - VULNERABLE VERSION
//
// Executes governance proposals (AddMember, RemoveMember, ChangeThreshold, ChangeTimelock)
// with critical security vulnerabilities.

#[derive(Accounts)]
pub struct ExecuteProposal<'info> {
    // VULNERABILITY [CRITICAL]: No executor role validation
    //
    // The secure version validates that executor has Admin or Executor role.
    // This vulnerable version allows anyone to execute, including non-members.
    //
    // Example Attack:
    //   1. Threshold is 3, only 2 members have approved
    //   2. Attacker is not a member
    //   3. Attacker calls execute (no role check)
    //   4. Combined with missing threshold check, proposal executes
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

    // VULNERABILITY [HIGH]: close = executor may not be appropriate
    //
    // Rent should go back to the original proposer, not the executor.
    // This creates incentive for frontrunning: executor steals rent refund.
    #[account(
        mut,
        seeds = [
            PROPOSAL,
            multisig_account.key().as_ref(),
            &proposal.proposal_id.to_le_bytes(),
        ],
        bump = proposal.bump,
        close = executor, // VULNERABLE: Should be proposer
    )]
    pub proposal: Account<'info, Proposal>,
}

impl<'info> ExecuteProposal<'info> {
    pub fn execute_proposal(&mut self) -> Result<()> {
        // VULNERABILITY [CRITICAL]: Missing pause check
        //
        // The secure version validates: !multisig.paused
        // Without this, proposals execute during emergencies when
        // the multisig should be frozen.
        //
        // Example Attack:
        //   1. Malicious proposal is created
        //   2. Admin pauses multisig to investigate
        //   3. Attacker executes anyway (no pause check)
        //   4. Malicious action completes despite emergency pause
        //
        // Fix: require!(!self.multisig_account.paused, MultisigError::MultisigPaused);


        // VULNERABILITY [CRITICAL]: Missing executor permission check
        //
        // The secure version validates: multisig.can_execute(&executor.key())
        // Only Admin or Executor roles should execute.
        //
        // Fix: require!(self.multisig_account.can_execute(&self.executor.key()), MultisigError::CannotExecute);


        // VULNERABILITY [CRITICAL]: Missing proposal status check
        //
        // The secure version validates: proposal.status == ProposalStatus::Active
        // Without this, already-executed or cancelled proposals can be re-executed.
        //
        // Example Attack (Double Execution):
        //   1. Proposal to add Alice is executed
        //   2. Alice is added to multisig
        //   3. Attacker calls execute again on same proposal
        //   4. Without status check, proposal re-executes
        //   5. Depending on AddMember logic, could corrupt state
        //
        // Fix: require!(self.proposal.status == ProposalStatus::Active, MultisigError::ProposalNotActive);


        // VULNERABILITY [CRITICAL]: Missing threshold check
        //
        // The secure version validates: approval_count >= threshold
        // Without this, proposals execute with fewer approvals than required!
        //
        // Example Attack:
        //   1. Multisig has threshold of 5
        //   2. Proposal has 1 approval (auto-approve from proposer)
        //   3. Attacker calls execute (no threshold check)
        //   4. Proposal executes with only 1/5 approvals
        //   5. Attacker has bypassed entire multisig security
        //
        // Fix: require!(self.proposal.approval_count >= self.multisig_account.threshold, MultisigError::InsufficientApprovals);


        // VULNERABILITY [CRITICAL]: Missing timelock check
        //
        // The secure version validates: timelock_passed()
        // Without this, proposals execute immediately after creation,
        // defeating the purpose of the timelock security delay.
        //
        // Example Attack:
        //   1. Malicious insider creates proposal to drain funds
        //   2. Insider approves with their own signature
        //   3. Immediately executes (no timelock wait)
        //   4. Other members have no time to notice and stop it
        //
        // Fix: require!(self.proposal.timelock_passed(clock.unix_timestamp, self.multisig_account.timelock_seconds), MultisigError::TimelockNotPassed);


        // VULNERABILITY [HIGH]: Missing expiry check
        //
        // The secure version validates: !proposal.is_expired()
        // Without this, very old proposals can still execute, even
        // if the situation has changed since creation.
        //
        // Example Attack:
        //   1. Proposal created to transfer 100 SOL to vendor
        //   2. Vendor relationship ends, proposal forgotten
        //   3. 2 years later, attacker finds old approved proposal
        //   4. Executes ancient proposal, funds go to old vendor
        //
        // Fix: require!(!self.proposal.is_expired(clock.unix_timestamp), MultisigError::ProposalExpired);


        // Execute the proposal type
        // Note: Even the execution logic has vulnerabilities!
        match self.proposal.proposal_type {
            ProposalType::AddMember { new_member, role } => {
                // VULNERABILITY [HIGH]: No bounds check before adding
                //
                // The secure version validates: owner_count < MAX_OWNERS
                // Without this, array index out of bounds panic.
                //
                // Fix: Check owner_count < MAX_OWNERS

                // VULNERABILITY [MEDIUM]: No duplicate check at execution
                //
                // Even if creation didn't check, execution should verify
                // the member isn't already present.

                let index = self.multisig_account.owner_count as usize;
                // VULNERABLE: No bounds check, will panic if index >= MAX_OWNERS
                self.multisig_account.members[index] = Member {
                    pubkey: new_member,
                    role,
                };
                self.multisig_account.owner_count += 1; // VULNERABLE: Unchecked increment
            }

            ProposalType::RemoveMember { member_to_remove } => {
                // VULNERABILITY [CRITICAL]: Creator can be removed
                //
                // The secure version validates: member_to_remove != creator
                // Without this, the admin can be removed, making the
                // multisig ungovernable for admin-only operations.
                //
                // Fix: require!(member_to_remove != self.multisig_account.creator, MultisigError::CannotRemoveCreator);


                // VULNERABILITY [HIGH]: No threshold validation after removal
                //
                // The secure version ensures: threshold <= new_owner_count
                // Without this, removing members could make threshold
                // unreachable, permanently locking the multisig.
                //
                // Example:
                //   - 5 members, threshold = 5
                //   - Remove 1 member (owner_count = 4)
                //   - Now need 5 approvals from 4 members = impossible
                //   - Multisig permanently locked

                // Find and remove the member
                if let Some(index) = self.multisig_account.member_index(&member_to_remove) {
                    let last_index = (self.multisig_account.owner_count - 1) as usize;
                    self.multisig_account.members[index] = self.multisig_account.members[last_index];
                    self.multisig_account.members[last_index] = Member::default();
                    self.multisig_account.owner_count -= 1; // VULNERABLE: No underflow check
                }
            }

            ProposalType::ChangeThreshold { new_threshold } => {
                // VULNERABILITY [CRITICAL]: No threshold bounds validation
                //
                // The secure version validates:
                // - new_threshold >= 1
                // - new_threshold <= owner_count
                //
                // Without these checks:
                // - threshold = 0: No approvals needed for anything!
                // - threshold > owner_count: Proposals can never execute
                //
                // Fix: Validate threshold bounds before setting.

                self.multisig_account.threshold = new_threshold; // VULNERABLE: Unvalidated value
            }

            ProposalType::ChangeTimelock { new_timelock } => {
                // VULNERABILITY [MEDIUM]: No timelock bounds validation
                //
                // The secure version validates reasonable bounds.
                // Without this:
                // - timelock = 0: No delay, instant malicious execution
                // - timelock = u64::MAX: Proposals can never execute (DoS)

                self.multisig_account.timelock_seconds = new_timelock; // VULNERABLE: Unvalidated value
            }
        }

        // Mark proposal as executed
        self.proposal.status = ProposalStatus::Executed;
        self.proposal.executed_at = Clock::get()?.unix_timestamp;

        // Update last executed
        self.multisig_account.last_executed_proposal = self.proposal.proposal_id;

        Ok(())
    }
}
