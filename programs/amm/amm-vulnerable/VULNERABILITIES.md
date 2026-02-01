# AMM Vulnerable - Documented Vulnerabilities

This document catalogs all intentional security vulnerabilities in the amm-vulnerable program for educational purposes.

## Critical Vulnerabilities

### V001: No Fee Validation (initialize_pool.rs)
**Severity**: Critical
**Location**: `initialize_pool.rs` - missing fee validation
**Description**: Pool creator can set arbitrary swap fees up to 655.35% (u16::MAX basis points)
**Secure Version**: Validates `fee_basis_points <= MAX_FEE_BASIS_POINTS` (1000 = 10%)
**Vulnerable Code**:
```rust
// VULNERABILITY: No fee validation
// Missing: require!(fee_basis_points <= MAX_FEE_BASIS_POINTS, AmmError::FeeTooHigh);
```
**Attack Scenario**: Malicious pool creator sets 50000 basis points (500%) fee, stealing from swappers

### V002: No Slippage Protection (deposit_liquidity.rs)
**Severity**: Critical
**Location**: `deposit_liquidity.rs` - missing slippage checks
**Description**: Depositors can receive far fewer LP tokens than expected due to front-running
**Secure Version**: Validates `amount_a <= max_amount_a` and `amount_b <= max_amount_b`
**Vulnerable Code**:
```rust
// VULNERABILITY: No slippage protection
// Missing: require!(amount_a <= max_amount_a, AmmError::ExcessiveDepositAmount);
// Missing: require!(amount_b <= max_amount_b, AmmError::ExcessiveDepositAmount);
```
**Attack Scenario**: Front-runner sees pending deposit, manipulates pool ratio, victim deposits at terrible price

### V003: No Expiration Validation (All operations)
**Severity**: Critical
**Location**: `helpers.rs::validate_expiration()` - function exists but does nothing
**Description**: Transactions can execute hours/days after submission at stale prices
**Secure Version**: Checks `expiration > current_time` and `expiration <= current_time + MAX_EXPIRATION_SECONDS`
**Vulnerable Code**:
```rust
pub fn validate_expiration(expiration: i64) -> Result<()> {
    // VULNERABILITY: No validation
    Ok(())
}
```
**Attack Scenario**: User submits swap at good price, transaction pending for hours, executes at terrible price

### V004: Unchecked Arithmetic (helpers.rs)
**Severity**: Critical
**Location**: `calculate_first_deposit`, `calculate_subsequent_deposit`, `calculate_withdrawal`
**Description**: Integer overflow/underflow possible in calculations
**Secure Version**: Uses `.checked_mul()`, `.checked_div()`, `.checked_sub()`
**Vulnerable Code**:
```rust
// VULNERABILITY: Unchecked arithmetic
let amount_a = (lp_to_mint * vault_a) / lp_supply;  // Can overflow
```
**Attack Scenario**: Large numbers cause overflow, user receives incorrect token amounts

### V005: Minimum Liquidity Too Low (constants.rs)
**Severity**: Critical
**Location**: `constants.rs::MINIMUM_LIQUIDITY = 1`
**Description**: Enables inflation attacks on LP token value
**Secure Version**: `MINIMUM_LIQUIDITY = 1000`
**Vulnerable Code**:
```rust
pub const MINIMUM_LIQUIDITY: u64 = 1;  // Too low!
```
**Attack Scenario**:
1. Attacker deposits 1 token A + 1 token B, gets 0 LP tokens (1 locked)
2. Attacker donates 1,000,000 tokens directly to vaults
3. Next depositor gets rounded to 0 LP tokens, loses their deposit

### V006: No Authorization on Lock/Unlock (lock_pool.rs, unlock_pool.rs)
**Severity**: High
**Location**: `lock_pool.rs`, `unlock_pool.rs` - missing authority check
**Description**: Anyone can lock/unlock any pool (DoS attack)
**Secure Version**: Calls `self.pool_config.assert_is_authority(&self.authority.key())`
**Vulnerable Code**:
```rust
// VULNERABILITY: No authority check
// Missing: self.pool_config.assert_is_authority(&self.authority.key())?;
self.pool_config.lock()?;
```
**Attack Scenario**: Attacker locks popular pool, preventing all operations (DoS)

### V007: No Pool Lock Enforcement (deposit/withdraw/swap)
**Severity**: High
**Location**: All operation instructions - missing lock check
**Description**: Operations continue even when pool is locked
**Secure Version**: Calls `self.pool_config.assert_not_locked()?` at start of operations
**Vulnerable Code**:
```rust
// VULNERABILITY: No lock enforcement
// Missing: self.pool_config.assert_not_locked()?;
```
**Attack Scenario**: Pool has critical bug, admin locks it, but operations continue anyway

### V008: No Withdrawal Slippage Protection (withdraw_liquidity.rs)
**Severity**: Critical
**Location**: `withdraw_liquidity.rs` - missing minimum amount checks
**Description**: Withdrawers can receive far less than expected due to sandwich attacks
**Secure Version**: Validates `amount_a >= min_amount_a` and `amount_b >= min_amount_b`
**Vulnerable Code**:
```rust
// VULNERABILITY: No slippage protection
// Missing: require!(amount_a >= min_amount_a, AmmError::InsufficientWithdrawAmount);
// Missing: require!(amount_b >= min_amount_b, AmmError::InsufficientWithdrawAmount);
```
**Attack Scenario**: Sandwicher front-runs withdrawal, manipulates ratio, victim gets less tokens

### V009: No Swap Slippage Protection (swap_tokens.rs)
**Severity**: Critical
**Location**: `swap_tokens.rs` - missing minimum output check
**Description**: Swappers can receive far less output than expected
**Secure Version**: Validates `output_amount >= min_output_amount` via curve library
**Vulnerable Code**:
```rust
// VULNERABILITY: Minimum output not enforced
// Secure version passes min_output_amount to curve.swap()
// Vulnerable version may ignore it
```
**Attack Scenario**: Front-runner manipulates pool before swap, victim gets 50% less tokens

## Medium Vulnerabilities

### V010: No Zero Amount Checks
**Severity**: Medium
**Location**: Multiple instructions
**Description**: Operations with 0 amounts waste gas and can cause unexpected behavior
**Secure Version**: `require!(amount > 0, AmmError::ZeroAmount)`
**Vulnerable Code**: Missing zero checks in deposit, withdraw, swap

### V011: No Liquidity Checks Before Operations
**Severity**: Medium
**Location**: `withdraw_liquidity.rs`, `swap_tokens.rs`
**Description**: Operations may fail ungracefully if pool is empty
**Secure Version**: Checks `vault_a_balance > 0`, `vault_b_balance > 0`, `lp_supply > 0`
**Vulnerable Code**: Missing pre-flight liquidity checks

### V012: Identical Mint Check Missing
**Severity**: Medium
**Location**: `initialize_pool.rs`
**Description**: Pool can be created with same token for both sides
**Secure Version**: `require!(token_a_mint != token_b_mint, AmmError::IdenticalTokenMints)`
**Vulnerable Code**:
```rust
// VULNERABILITY: Can create pool with identical tokens
// Missing: require!(self.token_a_mint.key() != self.token_b_mint.key())
```
**Attack Scenario**: Attacker creates SOL/SOL pool to confuse users

## Summary by Severity

**Critical (9 vulnerabilities)**:
- V001: No fee validation
- V002: No deposit slippage protection
- V003: No expiration validation
- V004: Unchecked arithmetic
- V005: Minimum liquidity too low
- V008: No withdrawal slippage protection
- V009: No swap slippage protection

**High (2 vulnerabilities)**:
- V006: No authorization on lock/unlock
- V007: No pool lock enforcement

**Medium (3 vulnerabilities)**:
- V010: No zero amount checks
- V011: No liquidity checks
- V012: Identical mint check missing

## Total: 14 Documented Vulnerabilities

## Testing

Each vulnerability can be demonstrated with exploit tests showing:
1. Attack setup
2. Exploit execution
3. Victim impact
4. Profit extraction

See `tests/exploits.rs` for practical demonstrations.

## Comparison with Secure Version

| Feature | Secure | Vulnerable |
|---------|--------|------------|
| Fee validation | Max 10% | Unlimited |
| Slippage protection | Yes | No |
| Expiration checks | Yes | No |
| Checked arithmetic | Yes | No (overflow risk) |
| Min liquidity lock | 1000 tokens | 1 token |
| Lock/unlock auth | Owner only | Anyone |
| Lock enforcement | Yes | No |
| Zero amount checks | Yes | No |
| Identical mint check | Yes | No |

## Educational Use Only

These vulnerabilities are intentional for teaching purposes. Never deploy code with these patterns to production.
