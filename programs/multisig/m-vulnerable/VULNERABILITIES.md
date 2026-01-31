# Multisig Vulnerabilities Documentation

This document catalogs all intentional vulnerabilities in the vulnerable multisig implementation for example purposes. Each vulnerability includes severity, description, attack scenario, and remediation.

## Table of Contents
1. [Critical Vulnerabilities](#critical-vulnerabilities)
2. [High Severity Vulnerabilities](#high-severity-vulnerabilities)
3. [Medium Severity Vulnerabilities](#medium-severity-vulnerabilities)
4. [Vulnerability Summary Table](#vulnerability-summary-table)

---

## Critical Vulnerabilities

### V001: Threshold = 0 Allows Instant Execution
**Location**: `create_multisig.rs:70`
**Severity**: CRITICAL
**CVSS Score**: 10.0

**Description**:
Missing lower bound validation on threshold parameter allows creation of multisig wallets with threshold = 0, meaning zero approvals are required to execute any proposal.

**Attack Scenario**:
```rust
// Attacker creates multisig with threshold = 0
create_multisig(multisig_id: 1, threshold: 0, timelock: 0);

// Attacker creates proposal to drain all funds
create_transfer_proposal(amount: vault_balance, recipient: attacker);

// Proposal is immediately executable without any approvals
execute_transfer_proposal(); // Success with 0 approvals!
```

**Impact**:
- Complete bypass of multi-signature security
- Single party can drain all funds
- No governance oversight

**Fix**:
```rust
require!(threshold >= 1, MultisigError::InvalidThreshold);
require!(threshold <= owner_count, MultisigError::ThresholdExceedsOwners);
```

---

### V002: Missing Threshold Check in Execution
**Location**: `execute_transfer_proposal.rs:88-101`
**Severity**: CRITICAL
**CVSS Score**: 10.0

**Description**:
The execute_transfer_proposal instruction does not validate that approval_count >= threshold before executing transfers. This allows execution of proposals with insufficient approvals.

**Attack Scenario**:
```rust
// Multisig with threshold = 5
// Vault has 1000 SOL

// Attacker is member #1, creates transfer proposal
create_transfer_proposal(amount: 1000 SOL, recipient: attacker);
// Proposer auto-approves: approval_count = 1

// Attacker immediately executes without waiting for 4 more approvals
execute_transfer_proposal(); // Success with 1/5 approvals!
// All 1000 SOL stolen
```

**Impact**:
- Complete circumvention of threshold requirement
- Single member can drain entire vault
- Multi-signature security is nullified

**Fix**:
```rust
require!(
    self.transfer_proposal.approval_count >= self.multisig_account.threshold,
    MultisigError::InsufficientApprovals
);
```

---

### V003: Missing Timelock Enforcement
**Location**: `execute_transfer_proposal.rs:104-106`
**Severity**: CRITICAL
**CVSS Score**: 9.0

**Description**:
No validation that the proposal has waited the required timelock duration before execution. Allows immediate execution, removing the safety window for review and cancellation.

**Attack Scenario**:
```rust
// Multisig with 48-hour timelock for security review

// Malicious admin creates transfer proposal at 12:00 PM
create_transfer_proposal(amount: 1000 SOL, recipient: attacker);

// Malicious admin executes 1 second later at 12:00:01 PM
execute_transfer_proposal(); // Success immediately!
// No time for other members to review and cancel
```

**Impact**:
- Removes security review period
- Enables rapid fund drainage
- Defeats purpose of timelock mechanism

**Fix**:
```rust
let clock = Clock::get()?;
require!(
    self.transfer_proposal.timelock_passed(
        clock.unix_timestamp,
        self.multisig_account.timelock_seconds
    ),
    MultisigError::TimelockNotPassed
);
```

---

### V004: Recipient Substitution Attack
**Location**: `execute_transfer_proposal.rs:114-123`
**Severity**: CRITICAL
**CVSS Score**: 10.0

**Description**:
The recipient account in execute_transfer_proposal is not validated against the recipient stored in the transfer_proposal. An attacker can pass any address as the recipient parameter, redirecting approved funds to their own address.

**Attack Scenario**:
```rust
// Alice creates proposal: Send 100 SOL to Bob
create_transfer_proposal(amount: 100 SOL, recipient: Bob);

// Alice, Charlie, Dave approve (3/3 threshold met)
approve_transfer_proposal(); // Alice
approve_transfer_proposal(); // Charlie
approve_transfer_proposal(); // Dave

// Attacker Eve executes but substitutes herself as recipient
execute_transfer_proposal(
    recipient: Eve // <- Not validated!
);
// Success! 100 SOL goes to Eve instead of Bob
// Bob never receives funds despite valid approvals
```

**Impact**:
- Complete fund redirection
- Approved recipient never receives funds
- Attacker receives funds approved for someone else
- Breaks fundamental trust in proposal system

**Fix**:
```rust
require!(
    self.transfer_proposal.recipient == self.recipient.key(),
    MultisigError::InvalidRecipient
);
require!(
    self.recipient.owner == &anchor_lang::system_program::ID,
    MultisigError::InvalidRecipient
);
```

---

### V005: Double Approval Attack
**Location**: `approve_proposal.rs` (entire implementation)
**Severity**: CRITICAL
**CVSS Score**: 9.5

**Description**:
Missing bitmap validation allows the same member to approve a proposal multiple times, artificially inflating approval_count to meet threshold requirements.

**Attack Scenario**:
```rust
// Multisig with 5 members, threshold = 4
// Attacker is member #3

// Attacker creates malicious proposal
create_proposal(proposal_type: TransferAllFunds);
// approval_count = 1 (auto-approve)

// Attacker approves their own proposal 3 more times
approve_proposal(); // approval_count = 2
approve_proposal(); // approval_count = 3
approve_proposal(); // approval_count = 4 (threshold met!)

// Execute with only 1 real member approval instead of 4
execute_proposal(); // Success!
```

**Impact**:
- Single member can meet any threshold
- Complete bypass of multi-signature requirement
- Attacker needs only one vote to control entire multisig

**Fix**:
```rust
// Check if already approved using bitmap
require!(
    !self.proposal.has_approved(owner_index),
    MultisigError::AlreadyApproved
);

// Atomic bitmap + counter update
self.proposal.approve(owner_index); // Sets bit and increments count
```

---

### V006: Missing Role-Based Access Control
**Location**: `create_proposal.rs:43-49`, `execute_transfer_proposal.rs:78-80`
**Severity**: CRITICAL
**CVSS Score**: 8.5

**Description**:
No validation of member roles (Admin, Proposer, Executor) before allowing proposal creation or execution. The secure version restricts:
- Proposal creation: Admin or Proposer only
- Execution: Admin or Executor only

The vulnerable version allows any member to perform any action.

**Attack Scenario**:
```rust
// Alice is Executor (should only approve, not propose)
// Bob is Proposer (should only propose, not execute)

// Alice creates proposal despite being Executor
create_proposal(proposal_type: DrainFunds); // No role check!

// Bob executes proposal despite being Proposer
execute_proposal(); // No role check!

// Role separation completely bypassed
```

**Impact**:
- Role-based security model is ineffective
- Separation of duties defeated
- Any member has full control

**Fix**:
```rust
// In create_proposal
require!(
    self.multisig_account.can_propose(&self.proposer.key()),
    MultisigError::CannotPropose
);

// In execute_transfer_proposal
require!(
    self.multisig_account.can_execute(&self.executor.key()),
    MultisigError::CannotExecute
);
```

---

### V007: Missing Pause State Checks
**Location**: `execute_transfer_proposal.rs:73-75`, all other instructions
**Severity**: CRITICAL
**CVSS Score**: 7.5

**Description**:
Instructions do not check if the multisig is paused before executing operations. When paused, all operations should be blocked except unpause.

**Attack Scenario**:
```rust
// Security breach detected, admin pauses multisig
toggle_pause(); // multisig.paused = true

// Attacker ignores pause state and executes malicious proposal
execute_transfer_proposal(); // No pause check, succeeds!

// Funds drained despite emergency pause
```

**Impact**:
- Emergency pause mechanism is ineffective
- Cannot stop ongoing attacks
- Defeats purpose of circuit breaker pattern

**Fix**:
```rust
require!(
    !self.multisig_account.paused,
    MultisigError::MultisigPaused
);
```

---

### V008: No Proposal Status Validation
**Location**: `execute_transfer_proposal.rs:83-85`
**Severity**: CRITICAL
**CVSS Score**: 9.0

**Description**:
Missing check that proposal status is Active before execution. Allows re-execution of already-executed proposals or execution of cancelled proposals.

**Attack Scenario**:
```rust
// Execute proposal first time
execute_transfer_proposal(); // Succeeds, status = Executed

// Execute same proposal again
execute_transfer_proposal(); // Succeeds again! (no status check)
// Funds transferred twice for same proposal

// Or execute cancelled proposal
cancel_proposal(); // status = Cancelled
execute_transfer_proposal(); // Succeeds! (no status check)
```

**Impact**:
- Double-spend vulnerability
- Cancelled proposals can still execute
- Fund drainage through repeated execution

**Fix**:
```rust
require!(
    self.transfer_proposal.status == ProposalStatus::Active,
    MultisigError::ProposalNotActive
);

// After execution
self.transfer_proposal.status = ProposalStatus::Executed;
```

---

## High Severity Vulnerabilities

### V009: Threshold Exceeds Owner Count (DoS)
**Location**: `create_multisig.rs:73-89`
**Severity**: HIGH
**CVSS Score**: 7.5

**Description**:
No validation that threshold <= owner_count at multisig creation. Allows creation of multisigs where proposals can never reach threshold, permanently locking all funds.

**Attack Scenario**:
```rust
// Attacker creates multisig with threshold = 10, owner_count = 1
create_multisig(threshold: 10, ...);

// Victim deposits 1000 SOL to vault
// ...

// Try to create withdrawal proposal
// Only 1 member exists, need 10 approvals
// Proposal can NEVER reach threshold
// 1000 SOL permanently locked!
```

**Impact**:
- Permanent fund lockup
- Denial of Service
- Unusable multisig wallet

**Fix**:
```rust
// At creation, owner_count = 1 (creator only)
require!(threshold <= 1, MultisigError::ThresholdExceedsOwners);

// When adding members, validate threshold is still valid
require!(
    self.multisig_account.is_valid_threshold(),
    MultisigError::InvalidThreshold
);
```

---

### V010: Rent Theft via close = executor
**Location**: `execute_transfer_proposal.rs:26-36`
**Severity**: HIGH
**CVSS Score**: 6.5

**Description**:
Transfer proposal accounts are closed with `close = executor`, meaning the executor receives the rent refund. This should be `close = proposer` to return rent to the proposal creator.

**Attack Scenario**:
```rust
// Alice creates proposal (pays ~0.002 SOL rent)
create_transfer_proposal(...); // Alice pays rent

// Bob (executor) waits for proposal approval
// ...

// Bob executes and receives Alice's rent refund
execute_transfer_proposal(); // Bob receives ~0.002 SOL

// Repeated across many proposals
// Bob accumulates rent from all proposers
```

**Impact**:
- Rent theft from proposers
- Economic incentive misalignment
- Griefing through rent collection

**Fix**:
```rust
#[account(
    mut,
    close = proposer, // Return rent to who paid it
)]
pub transfer_proposal: Account<'info, TransferProposal>,

// Add proposer field to account struct
pub proposer: AccountInfo<'info>,
```

---

### V011: Unrestricted Cancel Permission
**Location**: `cancel_proposal.rs` (entire implementation)
**Severity**: HIGH
**CVSS Score**: 7.0

**Description**:
Any member can cancel any proposal, not just the proposer or admin. Enables griefing attacks where malicious members repeatedly cancel legitimate proposals.

**Attack Scenario**:
```rust
// Alice creates legitimate proposal
create_proposal(proposal_type: AddNewMember);

// Bob (malicious member) immediately cancels it
cancel_proposal(); // No proposer check!

// Alice creates another proposal
create_proposal(proposal_type: AddNewMember);

// Bob cancels again
cancel_proposal();

// Bob can indefinitely prevent governance
```

**Impact**:
- Governance deadlock
- Denial of Service on proposal system
- Malicious minority can block legitimate actions

**Fix**:
```rust
require!(
    self.proposal.proposer == self.canceller.key() ||
    self.multisig_account.is_admin(&self.canceller.key()),
    MultisigError::NotProposerOrAdmin
);
```

---

### V012: Unrestricted Pause Permission
**Location**: `toggle_pause.rs` (entire implementation)
**Severity**: HIGH
**CVSS Score**: 7.5

**Description**:
Any member can toggle pause state, not just the admin. Malicious member can pause entire multisig causing denial of service.

**Attack Scenario**:
```rust
// Bob (Proposer role) is malicious

// Bob pauses the multisig
toggle_pause(); // No admin check!
// multisig.paused = true

// All operations now blocked for everyone
create_proposal(); // Fails - paused
approve_proposal(); // Fails - paused
execute_proposal(); // Fails - paused

// Only Bob can unpause (or other malicious members)
// Legitimate operations indefinitely blocked
```

**Impact**:
- Complete denial of service
- Single malicious member can lock entire multisig
- Emergency pause loses meaning

**Fix**:
```rust
require!(
    self.multisig_account.is_admin(&self.admin.key()),
    MultisigError::OnlyAdmin
);
```

---

### V013: Missing Vault Balance Check
**Location**: `execute_transfer_proposal.rs:126-132`
**Severity**: HIGH
**CVSS Score**: 6.0

**Description**:
No pre-validation that vault has sufficient balance before attempting transfer. While the CPI will fail with insufficient funds, early validation provides better error messages and prevents wasted computation.

**Attack Scenario**:
```rust
// Vault has 50 SOL

// Create proposal for 100 SOL
create_transfer_proposal(amount: 100 SOL, ...);

// Proposal gets approved (waste of effort)
approve_proposal(); // 3 members approve
approve_proposal();
approve_proposal();

// Wait for timelock (waste of time)
// ...

// Execute fails at CPI with generic error
execute_transfer_proposal();
// Error: "insufficient lamports"
// Everyone's time and compute wasted
```

**Impact**:
- Wasted computation and rent
- Poor user experience (unclear errors)
- Failed proposals after costly approval process

**Fix**:
```rust
require!(
    self.vault.lamports() >= self.transfer_proposal.amount,
    MultisigError::InsufficientFunds
);
```

---

### V014: No Proposal Expiry Check
**Location**: `execute_transfer_proposal.rs:109-111`
**Severity**: HIGH
**CVSS Score**: 6.5

**Description**:
Missing validation that proposal has not expired (created_at + timelock + grace_period). Allows execution of stale proposals that may no longer be relevant or safe.

**Attack Scenario**:
```rust
// 2020: Create proposal to send 100 SOL to Alice
create_transfer_proposal(amount: 100 SOL, recipient: Alice);

// 2025: Alice's key is compromised (5 years later)
// Attacker finds old approved-but-not-executed proposal

// Attacker executes 5-year-old proposal
execute_transfer_proposal();
// Funds sent to compromised address!
```

**Impact**:
- Execution of outdated proposals
- Context changed since approval
- Funds sent to compromised/incorrect addresses

**Fix**:
```rust
let clock = Clock::get()?;
require!(
    !self.transfer_proposal.is_expired(clock.unix_timestamp),
    MultisigError::ProposalExpired
);
```

---

## Medium Severity Vulnerabilities

### V015: No Input Sanitization on multisig_id
**Location**: `lib.rs:27-33`
**Severity**: MEDIUM
**CVSS Score**: 5.5

**Description**:
The multisig_id can be any u64 value without bounds checking. Specially crafted IDs could cause integer overflow in PDA derivation or enable PDA collision attacks.

**Attack Scenario**:
```rust
// Attacker uses u64::MAX as multisig_id
create_multisig(multisig_id: u64::MAX, ...);

// PDA derivation with u64::MAX bytes may cause:
// 1. Integer overflow in address computation
// 2. Predictable PDA addresses
// 3. Potential collision with other PDAs
```

**Impact**:
- Potential PDA collision
- Predictable addresses
- Integer overflow edge cases

**Fix**:
```rust
const MAX_MULTISIG_ID: u64 = 1_000_000_000; // Reasonable upper bound
require!(
    multisig_id > 0 && multisig_id < MAX_MULTISIG_ID,
    MultisigError::InvalidMultisigId
);
```

---

### V016: Unlimited Timelock Duration
**Location**: `create_multisig.rs:92-99`
**Severity**: MEDIUM
**CVSS Score**: 5.0

**Description**:
No upper or lower bounds on timelock_seconds parameter. Allows:
- timelock = 0: Instant execution (no safety window)
- timelock = u64::MAX: Proposals never executable (DoS)

**Attack Scenario**:
```rust
// Attack 1: Zero timelock for rapid exploitation
create_multisig(timelock_seconds: 0, ...);
create_proposal(...);
execute_proposal(); // Executes immediately, no review time

// Attack 2: Infinite timelock (DoS)
create_multisig(timelock_seconds: u64::MAX, ...);
create_proposal(...);
// Proposal can never execute (u64::MAX seconds = 584 billion years)
```

**Impact**:
- Removal of safety review period (timelock = 0)
- Permanent proposal lockup (timelock = u64::MAX)
- Denial of service

**Fix**:
```rust
const MAX_TIMELOCK: u64 = 7 * 24 * 60 * 60; // 7 days
const MIN_TIMELOCK: u64 = 60; // 1 minute
require!(
    timelock_seconds >= MIN_TIMELOCK && timelock_seconds <= MAX_TIMELOCK,
    MultisigError::InvalidTimelock
);
```

---

### V017: No Zero Amount Validation
**Location**: `create_transfer_proposal.rs` (entire implementation)
**Severity**: MEDIUM
**CVSS Score**: 4.5

**Description**:
Transfer proposals can be created with amount = 0, wasting resources and cluttering governance with meaningless proposals.

**Attack Scenario**:
```rust
// Spam multisig with 1000 zero-amount proposals
for i in 0..1000 {
    create_transfer_proposal(amount: 0, recipient: attacker);
    // Proposer pays rent for useless proposal
    // Clutters governance with noise
}
```

**Impact**:
- Wasted rent and storage
- Governance spam
- Proposal ID exhaustion

**Fix**:
```rust
require!(
    amount > 0,
    MultisigError::InvalidAmount
);
```

---

### V018: No Default Recipient Validation
**Location**: `create_transfer_proposal.rs` (entire implementation)
**Severity**: MEDIUM
**CVSS Score**: 5.0

**Description**:
Transfer proposals can be created with recipient = Pubkey::default() (all zeros), effectively burning funds if executed.

**Attack Scenario**:
```rust
// Malicious or mistaken proposal
create_transfer_proposal(
    amount: 1000 SOL,
    recipient: Pubkey::default() // 11111...1111
);

// Gets approved by mistake
// ...

// Executed - funds burned forever
execute_transfer_proposal();
// 1000 SOL sent to default address (unrecoverable)
```

**Impact**:
- Accidental fund burning
- Permanent loss of assets
- No recovery mechanism

**Fix**:
```rust
require!(
    recipient != Pubkey::default(),
    MultisigError::InvalidRecipient
);
```

---

## Vulnerability Summary Table

| ID | Name | Severity | Location | Impact |
|----|------|----------|----------|--------|
| V001 | Threshold = 0 | CRITICAL | create_multisig.rs:70 | Instant execution bypass |
| V002 | No Threshold Check | CRITICAL | execute_transfer_proposal.rs:88 | Insufficient approvals accepted |
| V003 | No Timelock Check | CRITICAL | execute_transfer_proposal.rs:104 | Immediate execution allowed |
| V004 | Recipient Substitution | CRITICAL | execute_transfer_proposal.rs:114 | Fund redirection |
| V005 | Double Approval | CRITICAL | approve_proposal.rs | Threshold bypass |
| V006 | No RBAC | CRITICAL | Multiple files | Role restrictions ineffective |
| V007 | No Pause Check | CRITICAL | All instructions | Emergency brake ineffective |
| V008 | No Status Check | CRITICAL | execute_transfer_proposal.rs:83 | Double execution |
| V009 | Threshold > Owners | HIGH | create_multisig.rs:73 | Fund lockup |
| V010 | Rent Theft | HIGH | execute_transfer_proposal.rs:26 | Economic griefing |
| V011 | Unrestricted Cancel | HIGH | cancel_proposal.rs | Governance DoS |
| V012 | Unrestricted Pause | HIGH | toggle_pause.rs | Complete DoS |
| V013 | No Balance Check | HIGH | execute_transfer_proposal.rs:126 | Wasted execution |
| V014 | No Expiry Check | HIGH | execute_transfer_proposal.rs:109 | Stale proposals execute |
| V015 | No ID Validation | MEDIUM | lib.rs:27 | PDA collision risk |
| V016 | Unlimited Timelock | MEDIUM | create_multisig.rs:92 | Instant or infinite delay |
| V017 | Zero Amount | MEDIUM | create_transfer_proposal.rs | Governance spam |
| V018 | Default Recipient | MEDIUM | create_transfer_proposal.rs | Fund burning |

**Total Vulnerabilities**: 18
**Critical**: 8
**High**: 6
**Medium**: 4

---

## Additional Notes

### Missing Validation vs Secure Version

The secure version implements the following protections that are intentionally absent here:

1. **Comprehensive Input Validation**: All numeric inputs validated (threshold, timelock, amounts)
2. **State Machine Enforcement**: Proposal status (Active/Executed/Cancelled) strictly validated
3. **Time-Based Security**: Timelock and expiry properly enforced
4. **Role-Based Access Control**: Admin/Proposer/Executor roles strictly enforced
5. **Bitmap-Based Approval**: Prevents double voting, efficient approval tracking
6. **Pause Mechanism**: Admin-only emergency brake that blocks all operations
7. **Recipient Validation**: Ensures funds go to intended system-owned addresses
8. **Economic Protections**: Rent returned to proposer, not executor

### Example Purpose

These vulnerabilities are intentionally implemented for example purposes to:
- Demonstrate common Solana security pitfalls
- Teach proper validation techniques
- Illustrate attack scenarios and their impact
- Provide a reference for security audits

**DO NOT use this vulnerable version in production.**

### Testing Exploits

The vulnerable version can be used to write exploit tests demonstrating each vulnerability in action. See the `tests/exploits/` directory for examples.

---

**Last Updated**: 2026-01-30
**Version**: 1.0.0
**Status**: Example Reference - DO NOT DEPLOY
