# Pino-Escrow: Secure vs Vulnerable

A side-by-side comparison of secure and vulnerable Solana escrow implementations using Pinocchio.

---

## What It Does

P2P token swap:
1. **Proposer** deposits Token A, requests specific amount of Token B
2. **Taker** sends Token B to proposer, receives Token A from vault
3. Atomic swap - either complete exchange or full revert

---

## Project Structure

```
pino-escrow/
  secure/           # Proper security validations
    src/
      lib.rs                      # Entry point with program ID check
      state/make.rs               # MakeState struct (122 bytes)
      instructions/
        mod.rs                    # Discriminators and routing
        propose_offer.rs          # 10+ security checks
        take_offer.rs             # 14+ security checks
    tests/
      integration.rs              # Happy path tests

  vulnerable/       # Intentionally insecure (educational)
    src/
      lib.rs                      # Missing program ID verification
      state/make.rs               # Same struct, no validation
      instructions/
        mod.rs                    # No routing checks
        propose_offer.rs          # Security checks omitted
        take_offer.rs             # Security checks omitted
    tests/
      utils.rs                    # Shared test helpers
      exploit_missing_signer.rs   # Signer validation exploit
      exploit_double_take.rs      # Active state check exploit
      exploit_wrong_proposer.rs   # Proposer validation exploit
      exploit_fake_offer.rs       # Ownership validation exploit
```

---

## Security Checks: Secure vs Vulnerable

### ProposeOffer

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Maker signed transaction | `maker.is_signer()` | Missing |
| Mint A owned by Token Program | `owned_by(token_program)` | Missing |
| Mint B owned by Token Program | `owned_by(token_program)` | Missing |
| Maker ATA ownership | `owned_by(token_program)` | Missing |
| Maker ATA correct size | `data_len() == TokenAccount::LEN` | Missing |
| Maker ATA address derived correctly | `find_program_address` check | Missing |
| Offer account uninitialized | `is_data_empty()` | Missing |
| Offer account writable | `is_writable()` | Missing |
| Vault address derived correctly | `find_program_address` check | Missing |
| Vault uninitialized | `is_data_empty()` | Missing |
| Vault writable | `is_writable()` | Missing |

### TakeOffer

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Taker signed transaction | `taker.is_signer()` | Missing |
| Mint ownership | `owned_by(token_program)` | Missing |
| Offer owned by escrow program | `owned_by(&crate::ID)` | Missing |
| Offer correct size | `data_len() == MakeState::LEN` | Missing |
| Offer writable | `is_writable()` | Missing |
| Offer is active | `is_active()` | Missing |
| Proposer matches offer state | `offer_state.proposer == proposer` | Missing |
| Mints match offer state | Field comparison | Missing |
| Proposer ATA B derived correctly | `find_program_address` | Missing |
| Taker ATA A validated | Owner, size, writable, derivation | Missing |
| Taker ATA B validated | Owner, size, derivation | Missing |
| Taker has enough Token B | Balance check | Missing |
| Vault validated | Owner, size, writable, derivation | Missing |
| Vault has enough Token A | Balance check | Missing |

---

## Exploit Test Findings

Each exploit test demonstrates a missing security check. The vulnerable program fails to validate, but Solana's runtime catches some attacks at the CPI level.

### 1. Missing Signer Check

**Test:** `cargo test test_exploit_missing_signer -- --nocapture`

**Attack:** Attacker calls ProposeOffer with victim's pubkey as maker, without victim signing.

**What happens:**
- Escrow program accepts the instruction (no `is_signer()` check)
- Program attempts CPI to Token program for transfer
- Solana runtime catches privilege escalation: `"victim's signer privilege escalated"`
- Transaction fails with `Cross-program invocation with unauthorized signer`

**Finding:** The escrow did NOT enforce signer check. It failed at CPI level because Token program requires authority signature. A secure program would reject immediately with "maker must sign".

---

### 2. Double Take (Missing Active State Check)

**Test:** `cargo test test_exploit_double_take -- --nocapture`

**Attack:** Taker calls TakeOffer twice on the same offer.

**What happens:**
- First TakeOffer succeeds, taker receives 100 Token A
- Offer state and vault are closed after first take
- Second TakeOffer fails with `AlreadyProcessed` or `InvalidAccountData`

**Finding:** The program has no `is_active()` check. It relies on account closure to prevent double-take. A secure program would check `offer_state.is_active()` before processing and reject with "offer not active".

---

### 3. Wrong Proposer (Missing Proposer Validation)

**Test:** `cargo test test_exploit_wrong_proposer -- --nocapture`

**Attack:** Attacker passes their own address as proposer when calling TakeOffer.

**What happens:**
- Escrow accepts wrong proposer (no validation against offer state)
- First CPI succeeds: Token B transfers from taker to attacker's ATA
- Second CPI fails: PDA signature uses real proposer from state, causing mismatch
- Transaction fails with `Cross-program invocation with unauthorized signer`

**Finding:** The escrow did NOT validate proposer matches offer state. The first transfer (taker -> attacker) actually went through before failure. A secure program would reject immediately with "proposer doesn't match offer state".

---

### 4. Fake Offer State (Missing Ownership Check)

**Test:** `cargo test test_exploit_fake_offer_state -- --nocapture`

**Attack:** Use a fake account (not owned by escrow program) as the offer.

**What happens:**
- Program attempts to read offer data from wrong account
- Fails with `InvalidAccountData` (wrong size/format)
- Does NOT fail with `InvalidAccountOwner`

**Finding:** The program did NOT check `offer.owner == escrow_program_id`. It failed on data parsing instead of ownership. A secure program would reject with "invalid account owner" before attempting to deserialize.

---

### Summary

| Exploit | Expected Rejection | Actual Failure Point |
|---------|-------------------|---------------------|
| Missing Signer | "maker must sign" | CPI privilege escalation |
| Double Take | "offer not active" | Account already processed/closed |
| Wrong Proposer | "proposer mismatch" | CPI unauthorized signer |
| Fake Offer | "invalid owner" | Invalid account data |

All vulnerabilities exist in the code. Some attacks are caught by Solana runtime protections, but the escrow program should reject them earlier with proper error messages.

---

## Running Tests

Build the programs first:

```bash
cd programs/pino-escrow/secure && cargo build-sbf
cd programs/pino-escrow/vulnerable && cargo build-sbf
```

### Secure Tests

```bash
cd programs/pino-escrow/secure

# Run all tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_full_escrow_flow -- --nocapture
cargo test test_propose_offer -- --nocapture
```

### Vulnerable Exploit Tests

```bash
cd programs/pino-escrow/vulnerable

# Run all exploit tests with output
cargo test -- --nocapture

# Run individual exploits
cargo test test_exploit_missing_signer -- --nocapture
cargo test test_exploit_double_take -- --nocapture
cargo test test_exploit_wrong_proposer -- --nocapture
cargo test test_exploit_fake_offer_state -- --nocapture
```

---

## Key Concepts

### Why Pinocchio?

Anchor abstracts away many security checks. Pinocchio forces you to write them manually, making vulnerabilities explicit when omitted.

| What Anchor Does | What Pinocchio Requires |
|------------------|-------------------------|
| `Signer<'info>` auto-checks | Manual `is_signer()` |
| `Account<'info, T>` validates owner | Manual `owned_by()` |
| Auto 8-byte discriminator | Manual discriminator check |
| `seeds`, `bump` constraints | Manual `find_program_address` |
| Auto deserialization | Manual `from_bytes` |

### PDA and Vault

- Offer PDA: `["offer", maker_pubkey, offer_id]`
- Vault: ATA of the offer PDA for Token A
- PDA signs transfers using `invoke_signed` with seeds

---

## Resources

- [Pinocchio GitHub](https://github.com/anza-xyz/pinocchio)
- [Solana Program Security Guide](https://www.helius.dev/blog/a-hitchhikers-guide-to-solana-program-security)
- [Sec3 2025 Security Report](https://solanasec25.sec3.dev/)

---

## License

MIT
