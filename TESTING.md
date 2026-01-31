# Testing Guide

This guide explains how to run tests for the Soteria security demonstration programs.

## Quick Start

### Using the Interactive Test Runner (Recommended)

We provide interactive test runners that make it easy to run tests without remembering commands.

#### Option 1: Bash Script (Linux/WSL/Mac)

```bash
# Make the script executable
chmod +x test-runner.sh

# Run the script
./test-runner.sh
```

#### Option 2: Python Script (Cross-platform)

```bash
# Run with Python 3
python3 test-runner.py
```

Both scripts provide an interactive menu where you can:
- Select which program to test (Escrow, Multisig, NFTs)
- Choose secure or vulnerable version
- Run all tests or specific tests
- Build all programs at once
- Run all tests across all programs

---

## Manual Testing

If you prefer to run tests manually, use the following commands:

### Pino Escrow

#### Secure Version
```bash
cd programs/pino-escrow/p-secure

# Build
cargo build-sbf

# Run all tests
cargo test -- --nocapture

# Run specific test
cargo test test_complete_escrow -- --nocapture
```

Available secure tests:
- `test_complete_escrow` - Full escrow flow
- `test_missing_recipient` - Validates recipient presence
- `test_wrong_token` - Validates token types
- `test_deposit_amounts` - Validates deposit amounts

#### Vulnerable Version
```bash
cd programs/pino-escrow/p-vulnerable

# Build
cargo build-sbf

# Run all exploit tests
cargo test -- --nocapture

# Run specific exploit
cargo test exploit_double_take -- --nocapture
```

Available exploit tests:
- `exploit_double_take` - Missing active state check
- `exploit_fake_offer` - Anyone can create offer
- `exploit_missing_signer` - Missing signer validation
- `exploit_wrong_proposer` - Proposer validation bypass

---

### Multisig

#### Secure Version
```bash
cd programs/multisig/m-secure

# Build
cargo build-sbf

# Run all tests
cargo test -- --nocapture

# Run specific test
cargo test test_full_governance_flow -- --nocapture
```

Available secure tests (14 total):
- `test_create_multisig` - Create multisig
- `test_full_governance_flow` - Complete governance
- `test_transfer_proposal_flow` - SOL transfers
- `test_remove_member` - Member removal
- `test_change_timelock` - Timelock changes
- `test_toggle_pause` - Pause mechanism
- `test_non_admin_cannot_pause` - Admin-only pause
- `test_timelock_enforcement` - Timelock validation
- `test_threshold_enforcement` - Threshold validation
- `test_double_approval_prevention` - Bitmap checks
- `test_role_based_access_control` - RBAC
- `test_non_member_cannot_approve` - Member validation
- `test_cannot_remove_creator` - Creator protection
- `test_cancel_proposal` - Proposal cancellation

#### Vulnerable Version
```bash
cd programs/multisig/m-vulnerable

# Build
cargo build-sbf

# Run all exploit tests
cargo test -- --nocapture

# Run specific exploit
cargo test exploit_threshold_bypass -- --nocapture
```

Available exploit tests (5 total):
- `exploit_threshold_bypass` - Execute with insufficient approvals (V002)
- `exploit_recipient_substitution` - Redirect funds to attacker (V004)
- `exploit_double_approval` - Same member approves multiple times (V005)
- `exploit_unauthorized_pause` - Non-admin pauses multisig (V013)
- `exploit_timelock_bypass` - Execute before timelock expires (V003)

---

### NFT Staking

#### Secure Version
```bash
cd programs/nfts/n-secure

# Build
cargo build-sbf

# Run all tests
cargo test -- --nocapture
```

#### Vulnerable Version
```bash
cd programs/nfts/n-vulnerable

# Build
cargo build-sbf

# Run all exploit tests
cargo test -- --nocapture
```

---

## Test Runner Features

### Main Menu
```
Soteria Security Test Runner
========================================

Select a program to test:
1. Pino Escrow
2. Multisig
3. NFTs (Staking)
4. Run all programs
5. Build all programs
0. Exit
```

### Version Selection
```
Select version:
1. Secure version
2. Vulnerable version
3. Both versions
0. Back to main menu
```

### Test Type Selection
```
How would you like to run the tests?
1. Run all tests
2. Run specific test
0. Back
```

When selecting "Run specific test", you'll see a numbered list of available tests to choose from.

---

## Understanding Test Output

### Secure Tests
Secure tests validate that security checks work correctly:
- **PASS** - Security check prevented unauthorized action
- **FAIL** - Security check failed or missing

### Vulnerable/Exploit Tests
Exploit tests demonstrate vulnerabilities:
- **VULNERABLE** - Exploit succeeded (demonstrates the vulnerability)
- **PROTECTED** - Exploit failed (vulnerability was fixed)

---

## Continuous Integration

To run all tests in CI:

```bash
# Build all programs
./test-runner.sh # Select option 5

# Run all tests
./test-runner.sh # Select option 4
```

Or manually:
```bash
# Run all tests for all programs
cd programs/pino-escrow/p-secure && cargo test -- --nocapture && cd -
cd programs/pino-escrow/p-vulnerable && cargo test -- --nocapture && cd -
cd programs/multisig/m-secure && cargo test -- --nocapture && cd -
cd programs/multisig/m-vulnerable && cargo test -- --nocapture && cd -
cd programs/nfts/n-secure && cargo test -- --nocapture && cd -
cd programs/nfts/n-vulnerable && cargo test -- --nocapture && cd -
```

---

## Troubleshooting

### "cargo: command not found"
Install Rust and Cargo:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### "cargo build-sbf: command not found"
Install Solana CLI tools:
```bash
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
```

### Tests fail with "program not found"
Build the program first:
```bash
cargo build-sbf
```

### Permission denied on test-runner.sh
Make it executable:
```bash
chmod +x test-runner.sh
```

---

## Next Steps

- Read `VULNERABILITIES.md` in each vulnerable program directory for detailed exploit documentation
- Read `README.md` in each program directory for architecture details
- Check individual test files for inline security annotations

---

## Quick Reference

| Command | Description |
|---------|-------------|
| `./test-runner.sh` | Interactive test runner (Bash) |
| `python3 test-runner.py` | Interactive test runner (Python) |
| `cargo test` | Run all tests in current directory |
| `cargo test <name>` | Run specific test |
| `cargo build-sbf` | Build Solana program |
| `cargo test -- --nocapture` | Run tests with output |
| `cargo test -- --list` | List available tests |

---

Happy testing!
