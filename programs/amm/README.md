# AMM: Secure vs Vulnerable

A side-by-side comparison of secure and vulnerable Solana Automated Market Maker (AMM) implementations using Anchor.

---

## What It Does

Constant product AMM (x * y = k) for decentralized token swaps:
1. **Pool Creators** initialize trading pools with custom fees and token pairs
2. **Liquidity Providers** deposit token pairs and receive LP tokens representing their share
3. **Swappers** exchange tokens using the constant product formula with configurable fees
4. **Pool Authority** can lock/unlock pools for emergency pause
5. Slippage protection, expiration timestamps, and checked arithmetic protect users

---

## Project Structure

```
amm/
  amm-secure/       # Proper security validations
    src/
      lib.rs                                  # Entry point with 6 instructions
      constants.rs                            # Fees, liquidity, expiration limits
      errors.rs                               # Custom error definitions
      helpers.rs                              # Reusable calculation and CPI helpers
      state/
        mod.rs                                # State module exports
        pool_config.rs                        # Pool configuration and lock state
      instructions/
        mod.rs                                # Instruction routing
        initialize_pool.rs                    # 2 security checks
        deposit_liquidity.rs                  # 6+ security checks
        withdraw_liquidity.rs                 # 8+ security checks
        swap_tokens.rs                        # 9+ security checks
        lock_pool.rs                          # Authorization check
        unlock_pool.rs                        # Authorization check
    tests/
      integration.rs                          # 5 comprehensive tests (LiteSVM)
      utils.rs                                # Test helpers and builders

  amm-vulnerable/   # Intentionally insecure (educational)
    src/
      lib.rs                                  # Missing critical validations
      constants.rs                            # Weak constraints (MINIMUM_LIQUIDITY=1)
      errors.rs                               # Same error definitions
      helpers.rs                              # Unchecked arithmetic, no expiration
      state/
        (same structure)                      # Same state definitions
      instructions/
        (same structure)                      # Security checks omitted
    tests/
      integration.rs                          # 7 exploit demonstrations
      utils.rs                                # Test helpers
    VULNERABILITIES.md                        # 14 documented vulnerabilities
    TESTING.md                                # Comprehensive testing guide
```

---

## Security Checks: Secure vs Vulnerable

### InitializePool

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Fee validation | `require!(fee <= MAX_FEE_BASIS_POINTS)` | **Missing** (allows 655%) |
| Identical mint check | `require!(mint_a != mint_b)` | **Missing** (SOL/SOL pools) |
| PDA derivation | Secure seeds | Same |
| Authority setup | Pool creator becomes authority | Same |

### DepositLiquidity

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Pool lock check | `pool_config.assert_not_locked()?` | **Missing** |
| Expiration validation | `validate_expiration(expiration)?` | **No-op** (accepts expired) |
| Zero amount checks | `require!(desired_amount_a > 0)` | **Missing** |
| Slippage protection | `require!(amount_a <= max_amount_a)` | **Missing** (front-running) |
| LP token check | `require!(lp_tokens > 0)` | **Missing** (0 LP minted) |
| MINIMUM_LIQUIDITY | 1000 tokens locked | **1 token** (inflation attack) |
| Checked arithmetic | `checked_mul()`, `checked_div()` | **Unchecked** (overflow) |

### WithdrawLiquidity

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Pool lock check | `pool_config.assert_not_locked()?` | **Missing** |
| Expiration validation | `validate_expiration(expiration)?` | **No-op** |
| Zero amount checks | `require!(lp_to_burn > 0)` | **Missing** |
| Liquidity checks | `require!(lp_supply > 0)` | **Missing** |
| Slippage protection | `require!(amount_a >= min_amount_a)` | **Missing** (sandwich attack) |
| Vault balance validation | `require!(vault_a >= amount_a)` | **Missing** |
| Checked arithmetic | `checked_mul()`, `checked_div()` | **Unchecked** |

### SwapTokens

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Pool lock check | `pool_config.assert_not_locked()?` | **Missing** |
| Expiration validation | `validate_expiration(expiration)?` | **No-op** |
| Zero amount checks | `require!(input_amount > 0)` | **Missing** |
| Liquidity checks | `require!(vault_a > 0, vault_b > 0)` | **Missing** |
| Min output enforcement | `require!(output >= min_output)` | **Missing** (front-running) |
| Checked arithmetic | Via constant_product_curve | Same |

### LockPool / UnlockPool

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Authorization check | `pool_config.assert_is_authority()` | **Missing** (DoS attack) |
| Pool config validation | PDA constraints | Same |

---

## Documented Vulnerabilities

The vulnerable version contains **14 intentional vulnerabilities** documented in source comments and VULNERABILITIES.md:

### Critical (9 vulnerabilities)
- **V001**: No fee validation - allows up to 655.35% fees (u16::MAX basis points)
- **V002**: No deposit slippage protection - front-runners manipulate pool ratio
- **V003**: No expiration validation - stale transactions execute at terrible prices
- **V004**: Unchecked arithmetic - overflow/underflow in calculations
- **V005**: MINIMUM_LIQUIDITY = 1 - enables inflation attacks (should be 1000)
- **V006**: No authorization on lock/unlock - anyone can DoS any pool
- **V007**: No pool lock enforcement - operations work even when pool is locked
- **V008**: No withdrawal slippage protection - sandwich attacks steal from withdrawers
- **V009**: No swap slippage protection - front-running steals from swappers

### High (2 vulnerabilities)
- **V012**: Liquidity checks missing - division by zero and underflow risks
- **V013**: No vault balance validation - may fail ungracefully

### Medium (3 vulnerabilities)
- **V010**: No zero amount checks - wastes gas, unexpected behavior
- **V011**: Liquidity checks before operations - operations may fail ungracefully
- **V014**: No identical mint check - allows nonsense SOL/SOL pools

---

## Running Tests

Build the programs first:

```bash
# Build secure version
cd programs/amm/amm-secure && cargo build-sbf

# Build vulnerable version
cd programs/amm/amm-vulnerable && cargo build-sbf
```

### Secure Tests

```bash
cd programs/amm/amm-secure

# Run all tests with output
cargo test-sbf -- --nocapture

# Run specific tests
cargo test-sbf test_initialize_pool -- --nocapture
cargo test-sbf test_deposit_liquidity_first_deposit -- --nocapture
cargo test-sbf test_deposit_and_withdraw -- --nocapture
cargo test-sbf test_swap_a_for_b -- --nocapture
cargo test-sbf test_lock_unlock_pool -- --nocapture
```

**Expected Results (Secure):**
- Fee capped at 10% (1000 basis points)
- MINIMUM_LIQUIDITY of 1000 locked on first deposit
- Slippage protection enforced on deposits, withdrawals, and swaps
- Expiration validation prevents stale transactions
- Pool lock prevents operations when paused
- Authorization required for lock/unlock

### Vulnerable Tests (Exploit Demonstrations)

```bash
cd programs/amm/amm-vulnerable

# Run all exploit tests with detailed output
cargo test-sbf -- --nocapture

# Run specific exploit tests
cargo test-sbf test_exploit_excessive_fees -- --nocapture
cargo test-sbf test_exploit_identical_mints -- --nocapture
cargo test-sbf test_exploit_deposit_front_running -- --nocapture
cargo test-sbf test_exploit_inflation_attack -- --nocapture
cargo test-sbf test_exploit_unauthorized_lock -- --nocapture
cargo test-sbf test_exploit_stale_transaction -- --nocapture
cargo test-sbf test_all_basic_operations_work -- --nocapture
```

**Expected Results (Vulnerable):**
- 500% fee pool creation succeeds (should fail at 10%)
- SOL/SOL pool creation succeeds (should fail)
- Front-runner manipulates deposit ratio (should fail slippage check)
- Inflation attack: victim deposits 100M tokens, receives 0 LP (should prevent)
- Unauthorized pool lock succeeds (should require authority)
- Hour-old transaction executes (should fail expiration)

All tests use **LiteSVM** for fast, Rust-based testing without requiring a validator.

See [TESTING.md](amm-vulnerable/TESTING.md) for detailed testing guide with example outputs.

---

## Key Features

### Constant Product Formula (x * y = k)

AMM maintains product constant for price discovery:

```rust
// Initial state: 100 A * 100 B = 10,000
// User swaps 10 A for B
// New state: 110 A * 90.9 B = 10,000 (minus fees)
```

**First Deposit (Geometric Mean):**
```rust
// LP tokens = sqrt(amount_a * amount_b) - MINIMUM_LIQUIDITY
// Example: sqrt(100 * 100) - 1000 = 10,000 - 1000 = 9,000 LP
// MINIMUM_LIQUIDITY (1000) is permanently locked
```

**Subsequent Deposits (Proportional):**
```rust
// LP tokens = min(amount_a/vault_a, amount_b/vault_b) * lp_supply
// Example: Pool has 100A + 200B with 10,000 LP supply
// User deposits 10A + 20B
// LP minted = min(10/100, 20/200) * 10,000 = 0.1 * 10,000 = 1,000 LP
```

**Withdrawals (Proportional):**
```rust
// amount_a = (lp_burned / lp_supply) * vault_a
// amount_b = (lp_burned / lp_supply) * vault_b
// Example: Burn 1,000 LP from 10,000 supply (10%)
// Receive 10% of vault_a and 10% of vault_b
```

### Slippage Protection

Protects users from price manipulation:

**Deposit Slippage:**
```rust
// User sets max amounts willing to deposit
require!(amount_a <= max_amount_a, AmmError::ExcessiveDepositAmount);
require!(amount_b <= max_amount_b, AmmError::ExcessiveDepositAmount);

// Prevents front-running:
// 1. User submits: deposit with max_amount_a=1000
// 2. Attacker front-runs: massive swap manipulates ratio
// 3. Secure: transaction fails slippage check
// 4. Vulnerable: transaction succeeds, user loses value
```

**Withdrawal Slippage:**
```rust
// User sets minimum amounts expected
require!(amount_a >= min_amount_a, AmmError::InsufficientWithdrawAmount);
require!(amount_b >= min_amount_b, AmmError::InsufficientWithdrawAmount);

// Prevents sandwich attacks:
// 1. User submits: withdraw expecting min_amount_a=1000
// 2. Front-runner: swap manipulates ratio
// 3. Secure: transaction fails slippage check
// 4. Vulnerable: transaction succeeds, user receives less
```

**Swap Slippage:**
```rust
// User sets minimum output expected
require!(output >= min_output_amount, AmmError::SlippageExceeded);

// Prevents front-running:
// 1. User submits: swap 1000 A for min 950 B
// 2. Front-runner: large swap moves price
// 3. Secure: transaction fails, user protected
// 4. Vulnerable: transaction succeeds at terrible price
```

### Expiration Timestamps

Prevents stale transactions from executing:

```rust
// Secure version
pub fn validate_expiration(expiration: i64) -> Result<()> {
    let current_time = Clock::get()?.unix_timestamp;

    // Must be in the future
    require!(expiration > current_time, AmmError::TransactionExpired);

    // Cannot be too far in the future (prevents griefing)
    let time_until = expiration.checked_sub(current_time)?;
    require!(time_until <= MAX_EXPIRATION_SECONDS, AmmError::ExpirationTooFar);

    Ok(())
}

// Vulnerable version - accepts ANY timestamp
pub fn validate_expiration(_expiration: i64) -> Result<()> {
    Ok(()) // No validation!
}
```

**Attack Scenario (Vulnerable):**
- User submits swap when price is 1.0
- Transaction pending for hours
- Market moves to price 0.5
- Transaction executes at terrible price (should have expired)

### MINIMUM_LIQUIDITY Protection

Prevents inflation attacks on first deposit:

**Secure Version (MINIMUM_LIQUIDITY = 1000):**
```rust
// First depositor adds 100,000 A + 100,000 B
let liquidity = sqrt(100000 * 100000) = 100,000
let lp_tokens = 100,000 - 1,000 = 99,000 LP minted
// 1,000 LP permanently locked

// Attacker tries inflation attack:
// 1. Attacker deposits 1 A + 1 B -> gets 0 LP (liquidity < MINIMUM_LIQUIDITY)
// Attack prevented before it starts!
```

**Vulnerable Version (MINIMUM_LIQUIDITY = 1):**
```rust
// Attacker deposits 1,000 A + 1,000 B
let liquidity = sqrt(1000 * 1000) = 1,000
let lp_tokens = 1,000 - 1 = 999 LP minted

// Attacker donates 1,000,000,000 directly to vaults
// Vault now has ~1B A + ~1B B, but only 999 LP tokens exist
// Each LP token represents ~1,000,000 of underlying

// Victim deposits 100,000,000 A + 100,000,000 B
// LP calculation: min(100M/1B, 100M/1B) * 999 = 0.1 * 999 = 99 LP
// Due to rounding in integer math, victim may get 0 LP!
// Victim loses entire deposit, attacker steals via withdrawal
```

### Fee System

Configurable swap fees with validation:

```rust
// Secure: Maximum 10% fee (1000 basis points)
pub const MAX_FEE_BASIS_POINTS: u16 = 1000;

// Initialization check
require!(
    fee_basis_points <= MAX_FEE_BASIS_POINTS,
    AmmError::ExcessiveSwapFee
);

// Vulnerable: Allows up to u16::MAX (65535 = 655.35%)
pub const MAX_FEE_BASIS_POINTS: u16 = u16::MAX;
// No validation check!
```

**Attack Scenario (Vulnerable):**
- Malicious pool creator sets 50,000 basis points (500% fee)
- Unsuspecting users swap on this pool
- Pool creator extracts massive fees as profit

### Pool Lock Mechanism

Emergency pause for critical bugs:

```rust
// Secure version
impl PoolConfig {
    pub fn assert_not_locked(&self) -> Result<()> {
        require!(!self.is_locked, AmmError::PoolLocked);
        Ok(())
    }

    pub fn assert_is_authority(&self, authority: &Pubkey) -> Result<()> {
        require!(self.authority == *authority, AmmError::Unauthorized);
        Ok(())
    }
}

// Every operation checks lock status
deposit_liquidity: pool_config.assert_not_locked()?;
withdraw_liquidity: pool_config.assert_not_locked()?;
swap_tokens: pool_config.assert_not_locked()?;

// Lock/unlock requires authorization
lock_pool: pool_config.assert_is_authority(&authority.key())?;
unlock_pool: pool_config.assert_is_authority(&authority.key())?;
```

**Vulnerable Version:**
- No lock checks in operations (V007)
- No authorization checks on lock/unlock (V006)
- Anyone can lock any pool (DoS attack)
- Anyone can unlock pools that authority locked for safety

### Checked Arithmetic

Prevents overflow and underflow:

**Secure Version:**
```rust
// First deposit calculation
let product = (amount_a as u128)
    .checked_mul(amount_b as u128)
    .ok_or(AmmError::Overflow)?;

let lp_tokens = liquidity
    .checked_sub(MINIMUM_LIQUIDITY)
    .ok_or(AmmError::Underflow)?;

// Subsequent deposit calculation
let lp_from_a = (desired_a as u128)
    .checked_mul(lp_supply as u128)
    .ok_or(AmmError::Overflow)?
    .checked_div(vault_a as u128)
    .ok_or(AmmError::DivisionByZero)?;
```

**Vulnerable Version:**
```rust
// Direct operations - can overflow/underflow
let product = (amount_a as u128) * (amount_b as u128);
let lp_tokens = liquidity - MINIMUM_LIQUIDITY;
let lp_from_a = (desired_a as u128) * (lp_supply as u128) / (vault_a as u128);
```

### Helper Functions

Reusable functions reduce code duplication:

**Validation Helpers:**
- `validate_expiration()` - Checks transaction not expired

**Calculation Helpers:**
- `calculate_first_deposit()` - Geometric mean for initial LP
- `calculate_subsequent_deposit()` - Proportional deposit calculation
- `calculate_withdrawal()` - Proportional withdrawal calculation

**CPI Helpers:**
- `transfer_tokens()` - Generic token transfer
- `transfer_from_vault()` - PDA-signed vault withdrawal
- `mint_lp_tokens()` - Mint LP tokens with PDA authority
- `burn_lp_tokens()` - Burn LP tokens during withdrawal

---

## Attack Scenarios Demonstrated

### Excessive Fees (test_exploit_excessive_fees)
**Vulnerable behavior**: Pool creator sets 50,000 basis points (500% fee). Swappers lose massive amounts to fees.

**Secure prevention**: Fee validation requires `fee_basis_points <= 1000` (10% maximum).

### Identical Mints (test_exploit_identical_mints)
**Vulnerable behavior**: Create SOL/SOL or USDC/USDC pool (nonsense trading pair).

**Secure prevention**: Requires `token_a_mint != token_b_mint`.

### Deposit Front-Running (test_exploit_deposit_front_running)
**Vulnerable behavior**:
1. Victim submits deposit at 1:1 ratio with max_amount_a=10, max_amount_b=10
2. Attacker front-runs with massive swap, manipulating ratio to 3:1
3. Victim's deposit executes at 3:1, depositing way more than expected
4. Victim receives far fewer LP tokens than anticipated

**Secure prevention**: Slippage protection enforces `amount_a <= max_amount_a` and `amount_b <= max_amount_b`.

### Inflation Attack (test_exploit_inflation_attack)
**Vulnerable behavior**:
1. Attacker deposits 1,000 A + 1,000 B (minimal amounts)
2. Receives sqrt(1000*1000) - 1 = 999 LP tokens
3. Attacker donates 1,000,000,000 A + 1,000,000,000 B directly to vaults
4. LP token value now inflated: each LP represents ~1,000,000 tokens
5. Victim deposits 100,000,000 A + 100,000,000 B
6. Victim receives 0 LP tokens due to rounding in proportional calculation
7. Attacker withdraws all liquidity, stealing victim's deposit

**Secure prevention**: MINIMUM_LIQUIDITY of 1,000 makes attack economically infeasible. Attacker would need to burn 1,000 LP tokens worth significant value.

### Unauthorized Lock (test_exploit_unauthorized_lock)
**Vulnerable behavior**: Random attacker (not pool authority) locks legitimate pool, preventing all operations (DoS).

**Secure prevention**: Authorization check requires `signer == pool_authority` for lock/unlock.

### Stale Transaction (test_exploit_stale_transaction)
**Vulnerable behavior**: User creates swap transaction hours ago. Market moves significantly. Hours-old transaction executes at current terrible price instead of failing.

**Secure prevention**: Expiration validation rejects transactions with `expiration < current_time`.

---

## Educational Purpose

This codebase is designed for **security education**:

- **amm-secure**: Demonstrates best practices for Solana AMM development
- **amm-vulnerable**: Shows common DeFi vulnerability patterns and attack vectors
- **Side-by-side comparison**: Helps developers identify security gaps
- **Exploit tests**: Practical demonstrations of real-world attacks
- **Detailed comments**: Every vulnerability explained with attack scenarios

**WARNING**: The vulnerable version is intentionally insecure. Never deploy similar code to production.

---

## Key Takeaways

### For Secure Implementation
1. Always validate fees against reasonable maximums (10% or less)
2. Implement slippage protection on deposits, withdrawals, and swaps
3. Validate expiration timestamps to prevent stale transactions
4. Use sufficient MINIMUM_LIQUIDITY (1000+) to prevent inflation attacks
5. Use checked arithmetic to prevent overflow/underflow
6. Enforce pool lock mechanism for emergency response
7. Require authorization for admin operations (lock/unlock)
8. Validate token mints are distinct (no SOL/SOL pools)
9. Check for zero amounts and sufficient liquidity
10. Use Box<Account> for large account structs to prevent stack overflow

### Common Pitfalls (Vulnerable Version)
1. No fee validation → predatory pools with 500%+ fees
2. No slippage protection → front-running and sandwich attacks
3. No expiration validation → stale transactions at terrible prices
4. MINIMUM_LIQUIDITY too low → inflation attacks steal deposits
5. Unchecked arithmetic → overflow/underflow vulnerabilities
6. No pool lock enforcement → cannot pause during emergencies
7. No authorization on lock/unlock → DoS attacks on any pool
8. No identical mint check → nonsense SOL/SOL pools
9. Missing zero/liquidity checks → unexpected failures
10. Not using Box<Account> → potential stack overflow
