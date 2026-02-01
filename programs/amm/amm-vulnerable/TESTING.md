# AMM-Vulnerable Testing Guide

This document explains how to run the vulnerability exploit tests for the AMM-VULNERABLE program.

## Overview

The tests in this program demonstrate real security vulnerabilities and their exploits. Each test shows:
- What vulnerability exists
- How an attacker can exploit it
- What the impact is on users
- What the lesson learned is

## Prerequisites

Before running the tests, ensure you have:
1. Rust and Cargo installed
2. Solana CLI tools installed
3. Anchor framework installed (v0.32.1)

## Building the Program

From the `programs/amm/amm-vulnerable` directory:

```bash
# Build the vulnerable program
cargo build-sbf
```

This compiles the program and creates `target/deploy/amm_vulnerable.so`.

## Running All Tests

To run all exploit tests:

```bash
# Run all tests with output
cargo test-sbf -- --nocapture

# Or without output
cargo test-sbf
```

## Running Individual Tests

To run a specific exploit test:

```bash
# Test excessive fees exploit (V001)
cargo test-sbf test_exploit_excessive_fees -- --nocapture

# Test identical mints exploit (V014)
cargo test-sbf test_exploit_identical_mints -- --nocapture

# Test deposit front-running exploit (V002)
cargo test-sbf test_exploit_deposit_front_running -- --nocapture

# Test inflation attack exploit (V005)
cargo test-sbf test_exploit_inflation_attack -- --nocapture

# Test unauthorized lock exploit (V006)
cargo test-sbf test_exploit_unauthorized_lock -- --nocapture

# Test stale transaction exploit (V003)
cargo test-sbf test_exploit_stale_transaction -- --nocapture

# Test basic operations (sanity check)
cargo test-sbf test_all_basic_operations_work -- --nocapture
```

## Test Output Explanation

Each test prints detailed output showing:

```
================================================================================
EXPLOIT TEST: [Vulnerability Name] ([Vulnerability ID])
================================================================================

[Setup] - Test environment preparation
[SCENARIO] - Description of the attack scenario
[EXPLOIT] - The exploit being executed
[EXPLOIT STEP X] - Multi-step exploit progression
[RESULT] - Whether the exploit succeeded
[IMPACT] - What damage this vulnerability causes
[LESSON] - How to prevent this vulnerability

================================================================================
```

## Example: Running the Excessive Fees Exploit

```bash
$ cd programs/amm/amm-vulnerable
$ cargo test-sbf test_exploit_excessive_fees -- --nocapture
```

Expected output:
```
================================================================================
EXPLOIT TEST: Excessive Fees (V001)
================================================================================
This test demonstrates how missing fee validation allows pool creators
to set exorbitant fees that steal from swappers.

[Setup] Malicious pool creator funded
[Setup] Token mints created

[EXPLOIT] Initializing pool with 50000 basis points (500.00% fee)
[EXPLOIT] In secure version, max fee is 1000 bp (10%)
[EXPLOIT] In vulnerable version, attacker can set up to 65535 bp (655.35%)

[RESULT] Pool initialization: SUCCESS
[IMPACT] In secure version, this would FAIL with fee validation error
[IMPACT] In vulnerable version, this SUCCEEDS, creating a predatory pool
[IMPACT] Users who swap on this pool will lose massive amounts to fees

[LESSON] Always validate fee parameters against reasonable maximums
================================================================================
```

## Vulnerabilities Tested

| Test Name | Vulnerability ID | Description |
|-----------|-----------------|-------------|
| `test_exploit_excessive_fees` | V001 | No fee validation - allows 655% fees |
| `test_exploit_identical_mints` | V014 | No identical mint check - SOL/SOL pools |
| `test_exploit_deposit_front_running` | V002 | No slippage protection on deposits |
| `test_exploit_inflation_attack` | V005 | MINIMUM_LIQUIDITY too low (1 instead of 1000) |
| `test_exploit_unauthorized_lock` | V006 | No authorization check on lock_pool |
| `test_exploit_stale_transaction` | V003 | No expiration validation |
| `test_all_basic_operations_work` | N/A | Sanity test - basic operations still work |

## Understanding the Tests

### Test Structure

Each exploit test follows this pattern:

1. **Setup Phase**: Create test environment (accounts, mints, pools)
2. **Exploit Phase**: Execute the attack
3. **Verification Phase**: Confirm the exploit succeeded
4. **Impact Phase**: Show what damage was done
5. **Lesson Phase**: Explain the fix

### Why Tests Should Pass

All exploit tests are expected to **PASS** (not fail). This is because:
- The vulnerable version ALLOWS the exploits (by design)
- Tests verify that exploits work as expected
- A passing test means the vulnerability exists (which we're demonstrating)

### Reading Test Output

When you see `assert!(result.is_ok(), ...)`:
- This asserts that the exploit succeeded
- In the secure version, the same operation would fail
- The test proves the vulnerability exists

## Comparing with Secure Version

To see how the secure version prevents these exploits:

```bash
# Run secure version tests
cd ../amm-secure
cargo test-sbf -- --nocapture
```

The secure version has additional checks that prevent all these exploits.

## Test Framework

Tests use:
- **LiteSVM**: Lightweight Solana VM for fast testing
- **litesvm_token**: Helper functions for SPL token operations
- **Anchor**: Solana framework for building programs

## Troubleshooting

### Build Errors

If you get build errors:
```bash
# Clean and rebuild
cargo clean
cargo build-sbf
```

### Missing Program File

If tests fail with "No such file or directory" for `.so` file:
```bash
# Ensure program is built
cargo build-sbf
# Check that target/deploy/amm_vulnerable.so exists
ls -la target/deploy/
```

### Test Hangs or Times Out

If tests hang:
- Kill with Ctrl+C
- Check that you're not running multiple tests in parallel
- Try running individual tests

## Additional Resources

- See `VULNERABILITIES.md` for detailed vulnerability documentation
- See `../amm-secure/` for the secure implementation
- See source code comments for inline exploit explanations

## Summary

These tests demonstrate real-world DeFi vulnerabilities:
- Missing validation (fees, mints, expiration)
- Missing authorization checks (lock/unlock)
- Missing slippage protection (deposits, withdrawals, swaps)
- Economic attacks (inflation, front-running)

**IMPORTANT**: Never deploy this vulnerable version to production. It is for educational purposes only.
