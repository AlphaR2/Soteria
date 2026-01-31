# Multisig: Secure vs Vulnerable

A side-by-side comparison of secure and vulnerable Solana multisig wallet implementations using Anchor.

---

## What It Does

Multi-signature wallet with role-based access control:
1. **Admin** creates multisig, manages members, and configures security parameters
2. **Proposers** create governance and transfer proposals
3. **Members** approve proposals using bitmap-based voting
4. **Executors** execute approved proposals after timelock and threshold requirements met
5. Timelock delays and expiry windows protect against malicious actions

---

## Project Structure

```
multisig/
  secure/           # Proper security validations
    src/
      lib.rs                                  # Entry point with 9 instructions
      constants.rs                            # PDA seeds and constants
      errors.rs                               # Custom error definitions
      state/
        mod.rs                                # State module exports
        multisig.rs                           # Multisig account 
        proposal.rs                           # Governance proposal
        transfer_proposal.rs                  # SOL transfer proposal
        member.rs                             # Member role enum
      instructions/
        mod.rs                                # Instruction routing
        create_multisig.rs                    # 8+ security checks
        create_proposal.rs                    # 6+ security checks
        create_transfer_proposal.rs           # 8+ security checks
        approve_proposal.rs                   # 7+ security checks
        approve_transfer_proposal.rs          # 7+ security checks
        execute_proposal.rs                   # 9+ security checks
        execute_transfer_proposal.rs          # 11+ security checks
        cancel_proposal.rs                    # 6+ security checks
        toggle_pause.rs                       # 4+ security checks
    tests/
      test.rs                                 # 14 comprehensive tests (LiteSVM)

  vulnerable/       # Intentionally insecure (example)
    src/
      lib.rs                                  # Missing role-based access control
      constants.rs                            # Same constants
      errors.rs                               # Same error definitions
      state/
        (same structure)                      # No validation in state
      instructions/
        (same structure)                      # Security checks omitted
    VULNERABILITIES.md                        # 18 documented vulnerabilities
```

---

## Security Checks: Secure vs Vulnerable

### CreateMultisig

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Threshold >= 1 | `require!(threshold >= 1)` | Missing |
| Threshold <= owner count | `require!(threshold <= owners.len())` | Missing |
| Timelock < max duration | `require!(timelock_seconds <= MAX_TIMELOCK)` | Missing |
| Owner count >= threshold | `require!(owners.len() >= threshold)` | Missing |
| Creator is owner | `require!(owners.contains(&creator))` | Missing |
| No duplicate owners | Deduplication check | Missing |
| Valid member roles | Role validation | Missing |
| Vault PDA derivation | Secure PDA seeds | Same |

### CreateProposal

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Proposer is member | `multisig.is_member(proposer)` | Missing |
| Proposer has proposer role | `multisig.can_propose(proposer)` | Missing |
| Multisig not paused | `require!(!multisig.paused)` | Missing |
| Valid proposal type | Type validation | Missing |
| PDA derivation | Secure seeds | Same |
| Auto-approval for proposer | Bitmap set correctly | Missing double-approval check |

### CreateTransferProposal

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Proposer is member | `multisig.is_member(proposer)` | Missing |
| Proposer has proposer role | `multisig.can_propose(proposer)` | Missing |
| Multisig not paused | `require!(!multisig.paused)` | Missing |
| Amount > 0 | `require!(amount > 0)` | Missing |
| Recipient is valid pubkey | `require!(recipient != default)` | Missing |
| Vault has sufficient balance | Balance check | Missing |
| PDA derivation | Secure seeds | Same |
| Auto-approval for proposer | Bitmap set correctly | Missing double-approval check |

### ApproveProposal

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Approver is member | `multisig.is_member(approver)` | Missing |
| Multisig not paused | `require!(!multisig.paused)` | Missing |
| Proposal is active | `require!(proposal.status == Active)` | Missing |
| Not already approved | Bitmap check | Missing (allows double approval) |
| Not expired | Expiry check | Missing |
| Proposal matches multisig | PDA validation | Missing |
| Bitmap update atomic | `proposal.approve(index)` | Vulnerable |

### ApproveTransferProposal

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Approver is member | `multisig.is_member(approver)` | Missing |
| Multisig not paused | `require!(!multisig.paused)` | Missing |
| Proposal is active | `require!(proposal.status == Active)` | Missing |
| Not already approved | Bitmap check | Missing (allows double approval) |
| Not expired | Expiry check | Missing |
| Proposal matches multisig | PDA validation | Missing |
| Bitmap update atomic | `proposal.approve(index)` | Vulnerable |

### ExecuteProposal

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Executor is member | `multisig.is_member(executor)` | Missing |
| Executor has executor role | `multisig.can_execute(executor)` | Missing |
| Multisig not paused | `require!(!multisig.paused)` | Missing |
| Proposal is active | `require!(proposal.status == Active)` | Missing |
| Threshold met | `require!(approvals >= threshold)` | Missing |
| Timelock passed | `proposal.timelock_passed()` | Missing |
| Not expired | `require!(!proposal.is_expired())` | Missing |
| Status update atomic | `proposal.status = Executed` | Same |
| Execution timestamp | `proposal.executed_at = now` | Same |

### ExecuteTransferProposal

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Executor is member | `multisig.is_member(executor)` | Missing |
| Executor has executor role | `multisig.can_execute(executor)` | Missing |
| Multisig not paused | `require!(!multisig.paused)` | Missing |
| Proposal is active | `require!(proposal.status == Active)` | Missing |
| Threshold met | `require!(approvals >= threshold)` | Missing |
| Timelock passed | `proposal.timelock_passed()` | Missing |
| Not expired | `require!(!proposal.is_expired())` | Missing |
| Recipient is system-owned | `require!(recipient.owner == System)` | Missing |
| Recipient matches proposal | `require!(recipient.key() == proposal.recipient)` | Missing |
| Vault has sufficient funds | `require!(vault.lamports >= amount)` | Missing |
| PDA signing | Secure vault seeds | Same |

### CancelProposal

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Canceller is proposer or admin | Role validation | Missing (anyone can cancel) |
| Multisig not paused | `require!(!multisig.paused)` | Missing |
| Proposal is active | `require!(proposal.status == Active)` | Missing |
| Proposal matches multisig | PDA validation | Missing |
| Status update | `proposal.status = Cancelled` | Same |
| Rent refund to proposer | `close = proposer` | Vulnerable (steals from proposer) |

### TogglePause

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Caller is creator/admin | `require!(caller == creator)` | Missing (anyone can pause) |
| Pause state toggle | `multisig.paused = !paused` | Same |

---

## Documented Vulnerabilities

The vulnerable version contains **18 intentional vulnerabilities** documented in `VULNERABILITIES.md`:

### Critical (8 vulnerabilities)
- **V001**: Threshold = 0 allows instant execution without approvals
- **V002**: Missing threshold check allows execution with insufficient approvals
- **V003**: Missing timelock enforcement allows immediate execution
- **V004**: Recipient substitution attack - funds redirected to attacker
- **V005**: Double approval attack - same member approves multiple times
- **V006**: Missing role-based access control - unauthorized proposal creation
- **V007**: Missing executor role check - anyone can execute proposals
- **V008**: Missing pause check - operations continue when paused

### High (6 vulnerabilities)
- **V009**: Missing threshold bounds check (threshold > owner count)
- **V010**: Missing proposal status check - execute cancelled proposals
- **V011**: Rent theft via close = executor instead of proposer
- **V012**: Anyone can cancel any proposal (no role check)
- **V013**: Anyone can pause multisig (DoS attack)
- **V014**: Missing vault balance check before transfer

### Medium (4 vulnerabilities)
- **V015**: Missing expiry check - execute stale proposals
- **V016**: Unlimited timelock value (DoS via permanent lock)
- **V017**: Zero amount transfers allowed
- **V018**: Missing input sanitization on multisig_id

---

## Running Tests

Build the programs first:

```bash

cd programs/multisig/secure && cargo build-sbf -- --target-dir ./target
cd programs/multisig/vulnerable && cargo build-sbf -- --target-dir ./target

```

### Secure Tests

```bash
cd programs/multisig/secure

# Run all tests with output
cargo test -- --nocapture

# Run specific tests

# -- for secure tests 
cargo test test_create_multisig -- --nocapture
cargo test test_full_governance_flow -- --nocapture
cargo test test_transfer_proposal_flow -- --nocapture
cargo test test_timelock_enforcement -- --nocapture
cargo test test_threshold_enforcement -- --nocapture
cargo test test_double_approval_prevention -- --nocapture
cargo test test_role_based_access_control -- --nocapture
cargo test test_non_admin_cannot_pause -- --nocapture
cargo test test_cannot_remove_creator -- --nocapture
cargo test test_non_member_cannot_approve -- --nocapture

# -- for vulnerable tests 
```

All tests use **LiteSVM** for fast, Rust-based testing without requiring a validator.

---

### Bitmap Approval System

The secure implementation uses a `u64` bitmap to track approvals efficiently:
- Each bit represents one member (supports up to 64 members)
- Prevents double approvals atomically
- Gas-efficient compared to vector storage

```rust
pub fn approve(&mut self, member_index: u8) -> Result<()> {
    let bit_mask = 1u64 << member_index;
    require!(self.approval_bitmap & bit_mask == 0, MultisigError::AlreadyApproved);
    self.approval_bitmap |= bit_mask;
    self.approval_count += 1;
    Ok(())
}
```

### Timelock Mechanism

Proposals cannot execute until `current_time >= created_at + timelock_seconds`:
- Provides security review window
- Allows time to cancel malicious proposals
- Configurable per multisig (0 to MAX_TIMELOCK)

### Proposal Expiry

Proposals expire after 30 days to prevent stale governance:
- `is_expired()` checks `current_time > created_at + PROPOSAL_EXPIRY_DURATION`
- Prevents execution of outdated proposals
- Cancelled proposals can be closed to reclaim rent

### Role-Based Access Control

Three roles with distinct permissions:
- **Admin**: Full control (add/remove members, pause, configure)
- **Proposer**: Create proposals
- **Executor**: Execute approved proposals

Members can have multiple roles simultaneously via bitflags.

---

