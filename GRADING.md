# **Security Analysis & Vulnerability Assessment**

This document provides a comprehensive analysis of Solana program vulnerabilities, their severity classifications, real-world impact, and mitigation strategies demonstrated across this repository.

---

## **📊 Repository Overview**

**Total Programs:** 6 (2 Pinocchio + 4 Anchor)  
**Total Vulnerabilities Demonstrated:** 28+  
**Frameworks Covered:** Pinocchio (manual validation) + Anchor (framework abstractions)  
**Real-World References:** 5+ major exploits documented ($58M+ in losses)  
**Audit Sources:** Sec3 2025 Report, Zellic, ThreeSigma

---

## **🎯 Security Focus Areas**

This repository demonstrates critical security concepts across:
- ✅ Access control and authentication
- ✅ Account validation and ownership
- ✅ Arithmetic safety and overflow prevention
- ✅ Cross-program invocation (CPI) security
- ✅ Oracle manipulation and data validation
- ✅ Economic attack vectors
- ✅ Emergency response mechanisms
- ✅ Type safety and discriminators

---

## **🔴 Critical Vulnerability Analysis**

### **Understanding Severity Classifications**

We use an **impact-based classification system** derived from industry standards (OWASP, CWE, CVSS) adapted for Solana's unique execution model:

**CRITICAL (🔴):** Direct loss of funds with minimal attacker setup  
**HIGH (🟠):** Loss of funds with preparation, or contract denial-of-service  
**MEDIUM (🟡):** Indirect exploits, no immediate fund loss  
**LOW (🟢):** Minimal security impact, easy to detect and fix  

---

## **💥 Vulnerability Deep Dive**

### **1. Missing Signer Validation (CRITICAL)**
**Programs:** Escrow, Vault, DAO  
**Real-World Impact:** Cashio exploit ($52M, March 2022)

**What Happens:**
When a program doesn't verify that a required account has actually signed the transaction, **any user can impersonate the authority** and perform privileged operations.

**Pinocchio Example (Manual Check Required):**
```rust
// ❌ VULNERABLE - No signer check
pub fn withdraw(accounts: &[AccountInfo]) -> ProgramResult {
    let owner = &accounts[0];
    // Missing: if !owner.is_signer { return Err(...) }
    // Anyone can pass any pubkey as owner
}

// ✅ SECURE
pub fn withdraw(accounts: &[AccountInfo]) -> ProgramResult {
    let owner = &accounts[0];
    if !owner.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
}
```

**Anchor Protection:**
```rust
// ✅ Anchor handles this automatically
#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,  // Type system enforces signature
}
```

**Key Insight:** This is why frameworks exist—Anchor's type system makes this class of bug impossible at compile time.

---

### **2. Account Ownership Not Checked (CRITICAL)**
**Programs:** ALL 6 programs  
**Real-World Impact:** Cashio exploit ($52M), Common in 2024 audits

**What Happens:**
Solana programs can accept **any account** as input. Without ownership validation, attackers can pass fake accounts controlled by malicious programs that return fraudulent data.

**The 4-Point Token Account Validation:**
```rust
// ❌ VULNERABLE - Accepts any account
let token_account: Account<TokenAccount> = // ... deserialized
// What if this isn't actually owned by Token Program?

// ✅ SECURE - The 4 critical checks:
pub fn validate_token_account(account: &AccountInfo) -> ProgramResult {
    // 1. Owner check - Must be owned by Token Program
    if account.owner != &spl_token::ID {
        return Err(ProgramError::IllegalOwner);
    }
    
    // 2. Discriminator check - Has correct data layout
    let data = account.try_borrow_data()?;
    if data.len() != TokenAccount::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // 3. Mint validation - Token is from expected mint
    let token_account = TokenAccount::unpack(&data)?;
    if token_account.mint != expected_mint {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // 4. Authority validation - Owned by expected user
    if token_account.owner != expected_owner {
        return Err(ProgramError::InvalidAccountData);
    }
    
    Ok(())
}
```

**Why This Matters:**
An attacker can create a fake program that returns any data structure, including a "TokenAccount" with inflated balances. Without ownership verification, your program trusts this fake data.

**Anchor Protection:** Partial—`Account<'info, TokenAccount>` validates owner and deserializes, but you must still validate mint and authority in business logic.

---

### **3. PDA Bump Not Validated (HIGH)**
**Programs:** Escrow, DAO  
**Real-World Impact:** Sec3 2025 Report findings

**What Happens:**
PDAs can have multiple valid bumps (0-255). Only the **canonical bump** (the highest valid value) should be used. Accepting non-canonical bumps allows multiple PDA addresses for the same seeds, breaking uniqueness assumptions.

**Attack Scenario:**
```rust
// ❌ VULNERABLE - Accepts any bump
let (pda, _bump) = Pubkey::find_program_address(&[b"vault", user.key()], program_id);
// Attacker passes bump=253 instead of canonical bump=254
// Now two "vault" PDAs exist for same user!

// ✅ SECURE - Validate canonical bump
let (pda, canonical_bump) = Pubkey::find_program_address(&[b"vault", user.key()], program_id);
if provided_bump != canonical_bump {
    return Err(ProgramError::InvalidSeeds);
}
```

**Pinocchio:** Must manually derive and compare  
**Anchor:** `seeds` and `bump` constraints handle this automatically

---

### **4. Integer Overflow (HIGH)**
**Programs:** Vault, AMM, Lending  
**Real-World Impact:** Multiple DeFi exploits across chains

**What Happens:**
Rust's default arithmetic **wraps on overflow** in release builds. `255u8 + 1 = 0`.

**Devastating Example:**
```rust
// ❌ VULNERABLE
let total_rewards = user_stake * reward_rate + existing_rewards;
// If overflow occurs: user_stake=u64::MAX, this wraps to near-zero
// User loses all rewards!

// ✅ SECURE
let rewards_earned = user_stake
    .checked_mul(reward_rate)
    .ok_or(ProgramError::ArithmeticOverflow)?;
let total_rewards = existing_rewards
    .checked_add(rewards_earned)
    .ok_or(ProgramError::ArithmeticOverflow)?;
```

**Framework Protection:**
- **Pinocchio:** No protection, use `checked_*` manually
- **Anchor:** No automatic protection, use `checked_*` manually

**Key Insight:** Even Anchor doesn't protect against this—business logic vulnerabilities require manual care.

---

### **5. No Circuit Breaker (HIGH)**
**Programs:** Vault, AMM, Lending  
**Real-World Impact:** Industry best practice, post-mortem requirement

**What Happens:**
Once an exploit begins, there's no way to **pause the program** and stop the bleeding.

**Production Pattern:**
```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum OperatingMode {
    Normal,
    Paused,
    DepositOnly,
    WithdrawOnly,
}

#[account]
pub struct ProtocolState {
    pub mode: OperatingMode,
    pub authority: Pubkey,
}

// In every instruction:
pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
    require!(
        ctx.accounts.state.mode == OperatingMode::Normal ||
        ctx.accounts.state.mode == OperatingMode::WithdrawOnly,
        ErrorCode::ProtocolPaused
    );
    // ... rest of logic
}
```

**Why This Matters:** Loopscale lost $5.8M in April 2025. A circuit breaker would have stopped the attack after the first few transactions.

---

### **6. Account Reload After CPI (MEDIUM)**
**Programs:** Escrow  
**Real-World Impact:** Sec3 audit findings

**What Happens:**
After a CPI (cross-program invocation), account data in memory is **stale**. The external program may have modified the account, but your borrow reflects the old state.

**Pinocchio Example:**
```rust
// ❌ VULNERABLE
let mut token_account_data = token_account.try_borrow_mut_data()?;
let token_account = TokenAccount::unpack(&token_account_data)?;
let balance_before = token_account.amount;

// CPI transfer happens here
invoke(...)?;

// Still holding old borrow! balance_before is stale!
// token_account.amount hasn't changed in our view

// ✅ SECURE - Drop and re-borrow
drop(token_account_data);  // Release stale borrow

let token_account_data = token_account.try_borrow_data()?;  // Fresh borrow
let token_account = TokenAccount::unpack(&token_account_data)?;
let balance_after = token_account.amount;  // Now accurate
```

**Anchor:** Less common due to reload mechanisms, but still possible with manual borrows.

---

### **7. Oracle Confidence Interval Not Checked (HIGH)**
**Programs:** Lending  
**Real-World Impact:** Loopscale $5.8M (April 2025)

**What Happens:**
Oracles like Pyth provide a **confidence interval** around the price. Ignoring this allows attackers to exploit price feeds during high volatility or manipulation.

**Secure Pattern:**
```rust
// ❌ VULNERABLE
let price = price_feed.get_current_price()?;
// What if confidence is ±$10,000 on a $50,000 BTC price?

// ✅ SECURE
let price_data = price_feed.get_current_price_with_confidence()?;
let max_confidence = price_data.price * CONFIDENCE_THRESHOLD / 10000; // e.g., 1%

if price_data.confidence > max_confidence {
    return Err(ErrorCode::OracleTooVolatile);
}
```

**Why It Matters:** During the Loopscale exploit, attackers leveraged periods when oracle confidence was high to manipulate pricing and drain funds.

---

### **8. remaining_accounts Not Validated (CRITICAL)**
**Programs:** DAO  
**Real-World Impact:** Synthetify DAO $230K (2023)

**What Happens:**
`remaining_accounts` is a dynamic array of accounts passed to instructions. Without validation, attackers can pass **malicious accounts** and trick the program logic.

**Attack Scenario:**
```rust
// ❌ VULNERABLE
pub fn execute_proposal(ctx: Context<Execute>) -> Result<()> {
    // Assumes remaining_accounts[0] is the treasury
    let treasury = &ctx.remaining_accounts[0];
    // What if attacker passed their own account instead?
}

// ✅ SECURE
pub fn execute_proposal(ctx: Context<Execute>) -> Result<()> {
    require!(
        ctx.remaining_accounts.len() >= 1,
        ErrorCode::MissingAccounts
    );
    
    let treasury = &ctx.remaining_accounts[0];
    
    // Validate it's actually the treasury PDA
    let (expected_treasury, _) = Pubkey::find_program_address(
        &[b"treasury", dao.key().as_ref()],
        ctx.program_id
    );
    
    require!(
        treasury.key() == expected_treasury,
        ErrorCode::InvalidTreasuryAccount
    );
}
```

**Framework Protection:**
- **Pinocchio:** Manual validation required
- **Anchor:** Manual validation required (even with Anchor!)

**Key Insight:** Dynamic account arrays are always dangerous—never trust indices without validation.

---

## **📊 Vulnerability Statistics**

### **By Severity**
- **CRITICAL (🔴):** 12 vulnerabilities → $110M+ in referenced exploits
- **HIGH (🟠):** 10 vulnerabilities → Industry standard violations
- **MEDIUM (🟡):** 6 vulnerabilities → Edge cases and data staleness
- **LOW (🟢):** 0 (this repo focuses on impactful vulnerabilities)

### **By Category**
- **Access Control:** 8 vulnerabilities (Signer, Owner, PDA)
- **Arithmetic Safety:** 3 vulnerabilities (Overflow, Rounding)
- **Data Validation:** 7 vulnerabilities (Oracle, Token Account, Discriminator)
- **CPI Security:** 4 vulnerabilities (Reentrancy, Data Staleness, Malicious Programs)
- **Economic Logic:** 4 vulnerabilities (Slippage, Collateral, Liquidation)
- **Emergency Response:** 2 vulnerabilities (Circuit Breaker, Timelock)

### **Framework Protection Analysis**
- **Anchor Auto-Protects:** ~40% of vulnerabilities (Signer, Owner, Discriminator, PDA)
- **Anchor Cannot Protect:** ~60% of vulnerabilities (Overflow, Business Logic, Oracles)
- **Pinocchio Auto-Protects:** 0% (100% manual validation required)

---

## **🎓 Key Security Insights**

### **1. Frameworks Are Not Silver Bullets**
Anchor eliminates entire classes of bugs (missing signer, owner checks), but **60% of critical vulnerabilities** still require manual validation:
- Integer overflow
- Business logic (collateral ratios, slippage)
- Oracle data quality
- Economic attack vectors

### **2. The 80/20 Rule Applies**
- **80% of exploits** come from 3 vulnerability types:
  1. Missing access control (signer/owner)
  2. Account ownership not validated
  3. Integer overflow
- **20% of exploits** are complex economic/oracle attacks

### **3. Manual Validation Is Unavoidable**
Even with Anchor, you must manually validate:
- Business logic invariants
- Economic parameters
- Oracle confidence
- Token mint/authority
- Dynamic account arrays

### **4. Defense in Depth**
Production programs layer multiple protections:
```
Access Control → Account Validation → Arithmetic Safety → 
Business Logic → Circuit Breaker → Emergency Pause
```

### **5. The Pinocchio Trade-off**
**Gains:** 84% compute unit reduction, 40% smaller binaries  
**Costs:** 100% manual security validation  
**Verdict:** Only use for performance-critical programs with expert teams

---

## **🔬 Testing Philosophy**

Each vulnerability includes:

**Exploit Test (exploit.ts):**
- Demonstrates the attack works on vulnerable code
- Shows exact exploit vector
- Quantifies impact (funds drained, DOS achieved, etc.)

**Security Test (secure.ts):**
- Proves fix prevents the exploit
- Shows error handling works correctly
- Validates defense-in-depth layers

**Why Both Matter:**
Security tests alone don't prove anything—you need exploit tests to demonstrate the vulnerability was real and exploitable.

---

## **📈 Real-World Impact**

### **Exploits Referenced**
| Exploit | Date | Amount | Vulnerability |
|---------|------|--------|---------------|
| **Cashio** | March 2022 | $52M | Missing account owner check |
| **Loopscale** | April 2025 | $5.8M | Oracle confidence not validated |
| **Synthetify DAO** | 2023 | $230K | remaining_accounts not validated |
| **Various** | 2024-2025 | ~$58M+ | Integer overflow, PDA issues |

### **Audit Report Insights**
From **Sec3's 2025 Annual Report** (163 audits, 1,669 findings):
- **85.5%** of vulnerabilities: Business logic, permissions, validation
- **14.5%** of vulnerabilities: Advanced (reentrancy, oracle manipulation)
- **Most common:** Missing access control, account validation failures

**Conclusion:** Basic validation bugs still dominate. Master the fundamentals before worrying about advanced attacks.

---

## **🛡️ Production Security Checklist**

Use this before deploying any Solana program:

### **Access Control**
- [ ] All privileged functions check `is_signer`
- [ ] All accounts validate `owner` field
- [ ] PDA bumps use canonical values
- [ ] Authority keys stored securely

### **Account Validation**
- [ ] Token accounts: 4-point validation (owner, discriminator, mint, authority)
- [ ] PDA derivation verified on-chain
- [ ] Account sizes validated
- [ ] Discriminators checked (or using Anchor)

### **Arithmetic Safety**
- [ ] All arithmetic uses `checked_*` operations
- [ ] Division by zero handled
- [ ] Rounding considered (LP shares, rewards)
- [ ] Overflow tests written

### **CPI Security**
- [ ] Target program IDs validated
- [ ] Account reload after CPI
- [ ] Signer seeds properly passed
- [ ] Reentrancy guards if needed

### **Business Logic**
- [ ] Slippage protection (DEX/AMM)
- [ ] Collateral ratios enforced (Lending)
- [ ] Oracle confidence checked
- [ ] Economic parameters validated

### **Emergency Response**
- [ ] Circuit breaker implemented
- [ ] Timelock for critical operations
- [ ] Upgrade authority secured
- [ ] Emergency contacts documented

---

## **🔗 Additional Resources**

**Audit Reports:**
- [Sec3 2025 Annual Security Report](https://sec3.dev/reports/2025)
- [Zellic Solana Security Research](https://zellic.io)
- [ThreeSigma Audit Database](https://threesigma.xyz)

**Solana Security:**
- [Solana Security Best Practices](https://docs.solana.com/security)
- [Neodyme's Solana Security Workshop](https://workshop.neodyme.io)
- [Anchor Security Guide](https://www.anchor-lang.com/docs/security)

**Case Studies:**
- [Cashio Post-Mortem](https://cashio.app/post-mortem)
- [Loopscale Security Analysis](https://loopscale.com/security)

---

## **📞 Contributing**

Found additional vulnerabilities? Have better mitigation patterns? 

**We welcome:**
- Additional exploit scenarios
- More efficient secure implementations
- Links to relevant audit findings
- Educational improvements

Open an issue or PR with your suggestions!

---

## **⚖️ Disclaimer**

The "vulnerable" implementations in this repository are **intentionally insecure** for educational purposes. 

**DO NOT:**
- Deploy vulnerable versions to mainnet
- Use code snippets without understanding them
- Assume Anchor protects against all bugs

**DO:**
- Study both versions to understand the vulnerability
- Run tests to see exploits in action
- Apply patterns to your own programs
- Share knowledge with the community

---

**Last Updated:** January 2026  
**Maintainer:** Steve (AlphaR)  
**License:** MIT

---

## **Appendix: Severity Classification Details**

### **CRITICAL (🔴)**
**Definition:** Bugs that cause direct loss of funds with minimal attacker setup.

**Characteristics:**
- Exploitable immediately upon discovery
- No specialized knowledge required
- Difficult or impossible to undo after detection
- Often goes unnoticed until significant damage

**Examples:**
- Missing signer check → Anyone can withdraw
- Account owner not validated → Fake accounts accepted
- CPI to arbitrary program → Funds drained instantly

**Response:** Immediate hotfix, program pause, post-mortem required

---

### **HIGH (🟠)**
**Definition:** Bugs that enable loss of funds with preparation, or render contract unusable.

**Characteristics:**
- Requires attacker preparation or specific conditions
- May need multiple transactions to exploit
- Can cause denial-of-service
- Moderate difficulty to detect and fix

**Examples:**
- Integer overflow in calculations
- Non-canonical PDA bump accepted
- No circuit breaker during emergency
- Oracle manipulation windows

**Response:** Urgent patch within 24-48 hours, user communication

---

### **MEDIUM (🟡)**
**Definition:** Bugs that don't cause direct fund loss but lead to exploitable mechanisms.

**Characteristics:**
- Indirect exploitation path
- Requires specific state or timing
- Limited economic impact
- Relatively easy to fix post-discovery

**Examples:**
- Account data staleness after CPI
- Missing discriminator checks
- Suboptimal slippage handling
- Poor event emissions

**Response:** Scheduled patch in next release, monitoring increased

---

### **LOW (🟢)**
**Definition:** Bugs with no significant immediate security impact.

**Characteristics:**
- Minimal or theoretical exploitation
- Easy to detect in testing
- Simple to fix
- No fund risk

**Examples:**
- Inefficient compute usage
- Missing documentation
- Code organization issues
- Non-critical event gaps

**Response:** Addressed in regular maintenance cycle