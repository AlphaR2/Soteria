# Governance: Secure vs Vulnerable

A side-by-side comparison of secure and vulnerable Solana reputation-based governance implementations using Anchor.

---

## What It Does

Reputation-based governance system with token staking and voting:
1. **Admin** initializes DAO, configures token requirements, and manages treasury
2. **Users** stake governance tokens to earn voting rights and reputation
3. **Voters** cast upvotes/downvotes to increase/decrease others' reputation
4. **Roles** automatically upgrade based on reputation thresholds (Member → Bronze → Silver → Gold)
5. Vote cooldowns and role restrictions protect against spam and abuse

---

## Project Structure

```
governance/
  g-secure/         # Proper security validations
    src/
      lib.rs                                  # Entry point with 8 instructions
      constants.rs                            # PDA seeds, thresholds, and limits
      errors.rs                               # Custom error definitions
      state/
        mod.rs                                # State module exports
        config.rs                             # DAO configuration
        treasury.rs                           # Token treasury state
        user_profile.rs                       # User reputation and stats
        username_registry.rs                  # Username uniqueness tracker
        vote.rs                               # Vote records and cooldowns
        member_ranks.rs                       # Rank progression system
      instructions/
        mod.rs                                # Instruction routing
        initialize_dao.rs                     # 5+ security checks
        initialize_treasury.rs                # 3+ security checks
        create_profile.rs                     # 5+ security checks
        stake_tokens.rs                       # 7+ security checks
        unstake_tokens.rs                     # 6+ security checks
        vote.rs                               # 11+ security checks
        reset_user_reputation.rs              # 3+ security checks
    tests/
      integration.rs                          # 5 comprehensive tests (LiteSVM)
      utils.rs                                # Test helpers and builders

  g-vulnerable/     # Intentionally insecure (educational)
    src/
      lib.rs                                  # Missing critical validations
      constants.rs                            # Weak constraints (1-char usernames)
      errors.rs                               # Same error definitions
      state/
        (same structure)                      # Truncated vote_weight (u8)
      instructions/
        (same structure)                      # Security checks omitted
    tests/
      integration.rs                          # 6 exploit demonstrations
      utils.rs                                # Test helpers
    VULNERABILITIES.md                        # 10+ documented vulnerabilities
```

---

## Security Checks: Secure vs Vulnerable

### InitializeDao

| Check | Secure | Vulnerable |
|-------|--------|------------|
| System pause check | `require!(!config.is_paused)` | Same |
| Minimum stake > 0 | `require!(minimum_stake > 0)` | Same |
| Vote power in range | `require!(vote_power >= 1 && <= 10)` | Same |
| PDA derivation | Secure seeds | Same |
| Admin authorization | `constraint = admin == signer` | Same |

### InitializeTreasury

| Check | Secure | Vulnerable |
|-------|--------|------------|
| System pause check | `require!(!config.is_paused)` | Same |
| Token mint matches config | `constraint = token_mint == config.token_mint` | Same |
| Treasury authority PDA | Secure PDA signing | Same |

### CreateProfile

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Username length >= 3 chars | `require!(username.len() >= MIN_USERNAME_LENGTH)` | **Missing** (allows 1-char) |
| Username length <= 20 chars | `require!(username.len() <= MAX_USERNAME_LENGTH)` | Same |
| Username alphanumeric only | Regex validation | **Missing** |
| Username not already taken | Registry check | Same |
| One profile per wallet | Account constraint | Same |

### StakeTokens

| Check | Secure | Vulnerable |
|-------|--------|------------|
| System pause check | `require!(!config.is_paused)` | **Missing** |
| Amount > 0 | `require!(amount > 0)` | Same |
| Minimum stake requirement | `require!(new_stake >= config.minimum_stake)` | **Missing** (allows 1 token) |
| Token mint matches config | Constraint validation | Same |
| Checked arithmetic | `checked_add()` for stake updates | **Unchecked** (overflow risk) |
| Token transfer validation | ATA verification | Same |
| Profile stake update | `profile.stake_amount += amount` | Same |

### UnstakeTokens

| Check | Secure | Vulnerable |
|-------|--------|------------|
| System pause check | `require!(!config.is_paused)` | **Missing** |
| Amount > 0 | `require!(amount > 0)` | Same |
| Sufficient stake balance | `require!(profile.stake >= amount)` | Same |
| Maintain minimum stake | `require!(remaining >= minimum_stake)` | **Missing** |
| Checked arithmetic | `checked_sub()` for stake updates | **Unchecked** (underflow risk) |
| Treasury authority PDA signing | Secure seeds | Same |

### Upvote/Downvote

| Check | Secure | Vulnerable |
|-------|--------|------------|
| System pause check | `require!(!config.is_paused)` | **Missing** |
| Self-vote prevention | `require!(voter != target)` | **Missing** (allows self-voting) |
| Minimum stake requirement | `require!(voter_stake >= minimum_stake)` | **Missing** (zero-stake voting) |
| Cooldown enforcement | `require!(elapsed >= cooldown_duration)` | **Missing** (spam voting) |
| Downvote role restriction | `require!(role >= Bronze)` | **Missing** (Members can downvote) |
| Vote record uniqueness | `init` vs `init_if_needed` | `init` (cannot change votes) |
| Reputation floor | `.max(REPUTATION_FLOOR)` | **Missing** (unlimited negative) |
| Vote weight precision | Stored as `i64` | **Truncated to u8** (data loss) |
| Checked arithmetic | `checked_add()` for reputation | **Unchecked** (overflow risk) |
| Target user validation | PDA and registry checks | Same |

### ResetUserReputation

| Check | Secure | Vulnerable |
|-------|--------|------------|
| Admin authorization | `require!(admin == config.admin)` | Same |
| System pause check | `require!(!config.is_paused)` | **Missing** |
| Target user exists | Account constraint | Same |

---

## Documented Vulnerabilities

The vulnerable version contains **10+ intentional vulnerabilities** documented in source comments:

### Critical (6 vulnerabilities)
- **V001**: No minimum stake enforcement - sybil attacks with 1-token stake
- **V002**: Self-voting allowed - users inflate their own reputation
- **V003**: No cooldown enforcement - unlimited spam voting
- **V004**: Members can downvote - new users can grief others
- **V005**: Vote weight truncated to u8 - precision loss (stores 255 max instead of actual)
- **V006**: No reputation floor - unlimited negative reputation (i64::MIN)

### High (3 vulnerabilities)
- **V007**: Cannot change votes - `init` instead of `init_if_needed` locks votes permanently
- **V008**: Unchecked arithmetic - overflow/underflow in stake and reputation
- **V009**: Single-character usernames allowed - namespace pollution

### Medium (2 vulnerabilities)
- **V010**: No system pause check in vote functions - cannot halt during emergencies
- **V011**: No username alphanumeric validation - special characters allowed

---

## Running Tests

Build the programs first:

```bash
# Build secure version
cd programs/governance/g-secure && cargo build-sbf -- --target-dir ./target

# Build vulnerable version
cd programs/governance/g-vulnerable && cargo build-sbf -- --target-dir ./target
```

### Secure Tests

```bash
cd programs/governance/g-secure

# Run all tests with output
cargo test -- --nocapture

# Run specific tests
cargo test test_initialize_dao -- --nocapture
cargo test test_create_profile -- --nocapture
cargo test test_stake_tokens -- --nocapture
cargo test test_upvote_user -- --nocapture
cargo test test_minimum_stake_enforcement -- --nocapture
```

**Expected Results (Secure):**
- Minimum stake enforced (100,000 tokens required)
- Vote cooldown enforced (24 hours for Members)
- Self-voting prevented
- Reputation floor enforced (cannot go below -1000)
- Downvote role restriction (Bronze+ only)

### Vulnerable Tests (Exploit Demonstrations)

```bash
cd programs/governance/g-vulnerable

# Run all exploit tests
cargo test -- --nocapture

# Run specific exploit tests
cargo test test_exploit_no_minimum_stake -- --nocapture
cargo test test_exploit_self_voting -- --nocapture
cargo test test_exploit_no_cooldown_enforcement -- --nocapture
cargo test test_exploit_member_can_downvote -- --nocapture
cargo test test_exploit_vote_weight_truncation -- --nocapture
cargo test test_exploit_unlimited_negative_reputation -- --nocapture
```

**Expected Results (Vulnerable):**
- 1-token stake succeeds (should require 100,000)
- Self-voting succeeds (should be blocked)
- Immediate second vote succeeds (should be blocked for 24 hours)
- Member downvote succeeds (should require Bronze+)
- Vote weight truncated to 255 (should be 1000+)
- Reputation goes to -10,000 (should floor at -1000)

All tests use **LiteSVM** for fast, Rust-based testing without requiring a validator.

---

## Key Features

### Reputation-Based Rank Progression

Automatic role upgrades based on reputation points:

| Rank | Reputation Required | Vote Weight | Cooldown | Can Downvote |
|------|---------------------|-------------|----------|--------------|
| Member | 0 | 1x | 24 hours | No |
| Bronze | 100 | 2x | 18 hours | Yes |
| Silver | 500 | 3x | 12 hours | Yes |
| Gold | 1000 | 5x | 6 hours | Yes |

```rust
pub fn from_reputation(points: i64) -> Self {
    if points >= 1000 { MemberRanks::Gold }
    else if points >= 500 { MemberRanks::Silver }
    else if points >= 100 { MemberRanks::Bronze }
    else { MemberRanks::Member }
}
```

### Vote Cooldown System

Prevents spam voting with role-based cooldowns:
- **Member**: 24 hours between votes
- **Bronze**: 18 hours between votes
- **Silver**: 12 hours between votes
- **Gold**: 6 hours between votes

```rust
pub fn cooldown_duration(&self) -> i64 {
    match self {
        MemberRanks::Member => MEMBER_COOLDOWN_SECONDS,
        MemberRanks::Bronze => BRONZE_COOLDOWN_SECONDS,
        MemberRanks::Silver => SILVER_COOLDOWN_SECONDS,
        MemberRanks::Gold => GOLD_COOLDOWN_SECONDS,
    }
}
```

### Vote Weight Calculation

Higher ranks earn more influence per vote:

```rust
// Secure version: full precision
let initial_vote_weight = voter_profile.role_level.vote_weight() as i64;
let vote_weight = initial_vote_weight * config.vote_power as i64; // e.g., 5 * 200 = 1000

// Vulnerable version: truncated to u8
let vote_weight_u8 = vote_weight as u8; // 1000 becomes 255 (data loss!)
```

### Reputation Floor

Prevents unlimited negative reputation in secure version:

```rust
// Secure version
target_profile.reputation_points = (target_profile.reputation_points + reputation_change)
    .max(REPUTATION_FLOOR); // Cannot go below -1000

// Vulnerable version
target_profile.reputation_points = target_profile.reputation_points + reputation_change;
// Can reach i64::MIN (-9,223,372,036,854,775,808)
```

### Token Staking for Governance

Users must stake governance tokens to participate:
- **Minimum stake**: Configurable (default 100,000 tokens)
- **Purpose**: Prevents sybil attacks and spam
- **Unstaking**: Users can unstake but must maintain minimum stake if voting

### Username Registry

Prevents duplicate usernames using PDA-based registry:
- **Seeds**: `[b"user_registry", username.as_bytes()]`
- **Claimed flag**: Marks username as taken
- **Validation**: 3-20 characters, alphanumeric only (secure version)

### Treasury Management

Secure token custody using PDA authority:
- **Treasury PDA**: Holds all staked tokens
- **Treasury Authority**: PDA signer for withdrawals (no private key)
- **Associated Token Account**: SPL token storage

### System Pause Mechanism

Emergency halt for security incidents:
- Admin can toggle `is_paused` flag
- All operations (except unpause) blocked when paused
- Vulnerable version missing pause checks in vote functions

---

## Attack Scenarios Demonstrated

### Sybil Attack (test_exploit_no_minimum_stake)
**Vulnerable behavior**: Attacker stakes 1 token and gains voting rights, creates 1000 accounts to manipulate votes.

**Secure prevention**: Requires minimum stake (100,000 tokens), making sybil attacks economically infeasible.

### Reputation Inflation (test_exploit_self_voting)
**Vulnerable behavior**: Attacker votes for themselves repeatedly to gain Gold rank artificially.

**Secure prevention**: Self-vote check prevents users from voting for themselves.

### Vote Spam (test_exploit_no_cooldown_enforcement)
**Vulnerable behavior**: Attacker votes for same target 100 times in succession.

**Secure prevention**: Cooldown enforcement requires 24 hours between votes for Members.

### Grief Attack (test_exploit_member_can_downvote)
**Vulnerable behavior**: New Member user immediately downvotes legitimate users to -10,000 reputation.

**Secure prevention**: Downvote restricted to Bronze+ rank, requires earning reputation first.

### Precision Loss (test_exploit_vote_weight_truncation)
**Vulnerable behavior**: Gold user with 5x weight and 200 vote_power should apply 1000 points, but only 255 applied due to u8 truncation.

**Secure prevention**: Stores vote_weight as i64 to preserve full precision.

### Unlimited Negative Reputation (test_exploit_unlimited_negative_reputation)
**Vulnerable behavior**: Target user's reputation reaches -9,223,372,036,854,775,808 (i64::MIN).

**Secure prevention**: Reputation floor at -1000 prevents extreme negative values.

---

## Educational Purpose

This codebase is designed for **security education**:

- **g-secure**: Demonstrates best practices for Solana governance
- **g-vulnerable**: Shows common vulnerability patterns and attack vectors
- **Side-by-side comparison**: Helps developers identify security gaps
- **Exploit tests**: Practical demonstrations of attack scenarios

**WARNING**: The vulnerable version is intentionally insecure. Never deploy similar code to production.

---

## Key Takeaways

### For Secure Implementation
1. Always enforce minimum stake requirements to prevent sybil attacks
2. Implement cooldowns to prevent spam and vote manipulation
3. Validate all user inputs (username length, characters, uniqueness)
4. Use checked arithmetic to prevent overflow/underflow
5. Enforce role-based access control (e.g., downvote restrictions)
6. Implement reputation floors/ceilings to bound values
7. Prevent self-interactions (self-voting, self-delegation)
8. Use appropriate data types (i64 for precision, not u8)
9. Include system pause for emergency response
10. Use `init_if_needed` to allow vote changes

### Common Pitfalls (Vulnerable Version)
1. No minimum stake → sybil attacks
2. No cooldown → spam attacks
3. No self-vote check → reputation inflation
4. No role restriction on downvote → grief attacks
5. Truncated vote weight → precision loss
6. No reputation floor → unlimited negative reputation
7. Using `init` instead of `init_if_needed` → locked votes
8. Unchecked arithmetic → overflow/underflow
9. Weak username validation → namespace pollution
10. Missing pause checks → cannot halt during emergencies

---
