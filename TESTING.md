# Testing Guide

Complete guide for running tests in the Soteria security education repository.

---

## **Quick Start (Recommended)**

### **Interactive Test Runner**

The easiest way to build and test all programs:

```bash
# Make executable (first time only)
chmod +x test-runner.sh

# Launch interactive menu
./test-runner.sh
```

**Features:**
- Build individual programs or all at once
- Run secure tests (exploits prevented)
- Run vulnerable tests (attacks succeed)
- Execute all test suites sequentially
- Color-coded output with progress bars
- Test counts and timing information

---

## **Test Runner Menu Options**

### **Main Menu**

```
Main Menu
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  1 │ Multisig                 Multi-signature wallet tests
  2 │ Governance               Reputation-based DAO tests
  3 │ AMM                      Automated Market Maker tests
  4 │ Escrow (Pinocchio)       Atomic swap escrow tests
  5 │ NFT Minting              NFT minting & Metaplex tests
  ──┼────────────────────────────────────────────────────────
  6 │ Run All Tests            Execute all test suites
  7 │ Build Programs           Compile Solana programs
  ──┼────────────────────────────────────────────────────────
  0 │ Exit
```

### **Program Test Menu (Example: AMM)**

```
Select Test Type
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  1 │ Run Secure Tests         (5 tests)
  2 │ Run Exploit Tests        (7 tests)
  3 │ Run Both                 (12 tests total)
  ──┼────────────────────────────────────────────────────────
  0 │ Back to Main Menu
```

### **Build Menu**

```
Select Program to Build
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  1 │ Multisig     Secure      programs/multisig/m-secure
  2 │ Multisig     Vulnerable  programs/multisig/m-vulnerable
  3 │ Governance   Secure      programs/governance/g-secure
  4 │ Governance   Vulnerable  programs/governance/g-vulnerable
  5 │ AMM          Secure      programs/amm/amm-secure
  6 │ AMM          Vulnerable  programs/amm/amm-vulnerable
  7 │ Escrow       Secure      programs/pino-escrow/p-secure
  8 │ Escrow       Vulnerable  programs/pino-escrow/p-vulnerable
  9 │ NFT Minting  Secure      programs/nfts/n-secure
  10│ NFT Minting  Vulnerable  programs/nfts/n-vulnerable
  ──┼────────────────────────────────────────────────────────
  A │ Build All Programs      Sequential build (10 programs)
```

---

## **Manual Testing**

If you prefer manual testing, use the commands below.

### **Prerequisites**

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Solana CLI
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"

# Install Anchor CLI (for Anchor programs)
cargo install --git https://github.com/coral-xyz/anchor avm --force
avm install latest
avm use latest
```

---

### **Multisig (Anchor)**

```bash
# Build
cd programs/multisig/m-secure && cargo build-sbf
cd ../m-vulnerable && cargo build-sbf

# Test secure version (exploits prevented)
cd programs/multisig/m-secure
cargo test-sbf -- --nocapture

# Test vulnerable version (attacks succeed)
cd programs/multisig/m-vulnerable
cargo test-sbf -- --nocapture
```

**Test Count:** 4 secure + 4 exploit tests

---

### **Governance (Anchor)**

```bash
# Build
cd programs/governance/g-secure && cargo build-sbf
cd ../g-vulnerable && cargo build-sbf

# Test secure version (exploits prevented)
cd programs/governance/g-secure
cargo test-sbf -- --nocapture

# Test vulnerable version (attacks succeed)
cd programs/governance/g-vulnerable
cargo test-sbf -- --nocapture
```

**Test Count:** 5 secure + 6 exploit tests

**Key Exploits Demonstrated:**
- No minimum stake enforcement (sybil attacks)
- Self-voting (reputation inflation)
- No cooldown enforcement (spam voting)
- Member downvote capability (grief attacks)
- Vote weight truncation (precision loss)
- Unlimited negative reputation

---

### **AMM (Anchor)**

```bash
# Build
cd programs/amm/amm-secure && cargo build-sbf
cd ../amm-vulnerable && cargo build-sbf

# Test secure version (exploits prevented)
cd programs/amm/amm-secure
cargo test-sbf -- --nocapture

# Test vulnerable version (attacks succeed)
cd programs/amm/amm-vulnerable
cargo test-sbf -- --nocapture
```

**Test Count:** 5 secure + 7 exploit tests

**Key Exploits Demonstrated:**
- Excessive fee extraction
- Deposit front-running (sandwich attacks)
- Inflation attacks (low MINIMUM_LIQUIDITY)
- Unauthorized pool locking (DoS)
- Stale transaction execution
- No slippage protection

---

### **Escrow (Pinocchio)**

```bash
# Build
cd programs/pino-escrow/p-secure && cargo build
cd ../p-vulnerable && cargo build

# Test secure version
cd programs/pino-escrow/p-secure
cargo test -- --nocapture

# Test vulnerable version
cd programs/pino-escrow/p-vulnerable
cargo test -- --nocapture
```

**Test Count:** TBD

---

### **NFT Minting (Anchor + Metaplex Core)**

```bash
# Build
cd programs/nfts/n-secure && cargo build-sbf
cd ../n-vulnerable && cargo build-sbf

# Test secure version
cd programs/nfts/n-secure
cargo test-sbf -- --nocapture

# Test vulnerable version
cd programs/nfts/n-vulnerable
cargo test-sbf -- --nocapture
```

**Test Count:** TBD

---

## **Understanding Test Output**

### **Secure Tests (Exploits Prevented)**

```
✓ test_stake_tokens
  [SETUP] Creates DAO, initializes treasury, stakes tokens
  [VERIFY] Minimum stake requirement enforced (100,000 tokens)
  [VERIFY] Stake amount correctly updated in user profile
  [RESULT] ✓ PASS - Security checks prevented unauthorized action
```

### **Vulnerable Tests (Attacks Succeed)**

```
✓ test_exploit_no_minimum_stake
  [SETUP] Creates DAO and user profile
  [EXPLOIT STEP 1] Attacker stakes only 1 token (should require 100,000)
  [EXPLOIT STEP 2] Transaction succeeds - no minimum stake check
  [RESULT] ✓ VULNERABLE - Attack succeeded
  [IMPACT] Attacker gains voting rights with minimal economic stake
  [IMPACT] Enables sybil attacks (1000 accounts = only 1000 tokens)
  [LESSON] Always enforce minimum stake requirements
```

---

## **Test Structure**

All tests follow this pattern:

```rust
#[test]
fn test_exploit_name() {
    // SETUP: Create test environment
    let (svm, accounts) = setup_test();

    // EXPLOIT: Execute attack
    let result = execute_attack(&mut svm, &accounts);

    // VERIFY: Confirm outcome
    assert!(result.is_ok()); // Vulnerable version
    // assert!(result.is_err()); // Secure version

    // IMPACT: Document consequences
    println!("[IMPACT] Attacker can...");

    // LESSON: Explain prevention
    println!("[LESSON] Always validate...");
}
```

---

## **Running All Tests**

### **Via Test Runner (Recommended)**

```bash
./test-runner.sh
# Select option 6: "Run All Tests"
```

### **Via Script**

```bash
# If you have a run_all_tests.sh script
./scripts/run_tests.sh
```

### **Manually**

```bash
# From repository root
cd programs/multisig/m-secure && cargo test-sbf -- --nocapture && cd -
cd programs/multisig/m-vulnerable && cargo test-sbf -- --nocapture && cd -
cd programs/governance/g-secure && cargo test-sbf -- --nocapture && cd -
cd programs/governance/g-vulnerable && cargo test-sbf -- --nocapture && cd -
cd programs/amm/amm-secure && cargo test-sbf -- --nocapture && cd -
cd programs/amm/amm-vulnerable && cargo test-sbf -- --nocapture && cd -
cd programs/pino-escrow/p-secure && cargo test -- --nocapture && cd -
cd programs/pino-escrow/p-vulnerable && cargo test -- --nocapture && cd -
cd programs/nfts/n-secure && cargo test-sbf -- --nocapture && cd -
cd programs/nfts/n-vulnerable && cargo test-sbf -- --nocapture && cd -
```

---

## **Troubleshooting**

### **"cargo: command not found"**

Install Rust and Cargo:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### **"cargo build-sbf: command not found"**

Install Solana CLI:
```bash
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

### **"anchor: command not found"**

Install Anchor CLI:
```bash
cargo install --git https://github.com/coral-xyz/anchor avm --force
avm install latest
avm use latest
```

### **Tests fail with "program not found"**

Build the program first:
```bash
cargo build-sbf  # For Anchor programs
cargo build      # For Pinocchio programs
```

### **Permission denied: ./test-runner.sh**

Make it executable:
```bash
chmod +x test-runner.sh
```

### **Tests timeout or hang**

LiteSVM tests should run quickly. If tests hang:
- Ensure program is built with `cargo build-sbf`
- Check for infinite loops in test code
- Try running with `RUST_LOG=debug` for more output

---

## **Continuous Integration**

For CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Build All Programs
  run: |
    cd programs/multisig/m-secure && cargo build-sbf
    cd ../m-vulnerable && cargo build-sbf
    cd ../../governance/g-secure && cargo build-sbf
    cd ../g-vulnerable && cargo build-sbf
    cd ../../amm/amm-secure && cargo build-sbf
    cd ../amm-vulnerable && cargo build-sbf

- name: Run All Tests
  run: |
    cd programs/multisig/m-secure && cargo test-sbf
    cd ../m-vulnerable && cargo test-sbf
    cd ../../governance/g-secure && cargo test-sbf
    cd ../g-vulnerable && cargo test-sbf
    cd ../../amm/amm-secure && cargo test-sbf
    cd ../amm-vulnerable && cargo test-sbf
```

---

## **Test Coverage Summary**

| Program | Secure Tests | Exploit Tests | Total |
|---------|--------------|---------------|-------|
| Multisig | 4 | 4 | 8 |
| Governance | 5 | 6 | 11 |
| AMM | 5 | 7 | 12 |
| Escrow | TBD | TBD | TBD |
| NFT Minting | TBD | TBD | TBD |
| **Total** | **14+** | **17+** | **31+** |

---

## **Next Steps**

After running tests:

1. **Read VULNERABILITIES.md** in each vulnerable program directory
2. **Compare implementations** side-by-side (secure vs vulnerable)
3. **Study exploit tests** to understand attack vectors
4. **Review program READMEs** for architecture details
5. **Experiment** by modifying code and observing test outcomes

---

## **Quick Reference**

| Command | Description |
|---------|-------------|
| `./test-runner.sh` | Launch interactive test runner |
| `cargo test-sbf` | Run Anchor program tests |
| `cargo test` | Run Pinocchio program tests |
| `cargo test -- --nocapture` | Run tests with output |
| `cargo test <name>` | Run specific test |
| `cargo build-sbf` | Build Anchor program |
| `cargo build` | Build Pinocchio program |

---

**Need Help?** Check individual program README.md files or examine test source code for detailed examples.
