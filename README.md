# **Soteria - Solana Program Security Education**

Learn Solana security through **vulnerable** and **secure** implementations with real exploit demonstrations.

---

## **Overview**

5 production-grade programs demonstrating critical security vulnerabilities:

1. **Multisig** (Anchor) - Multi-signature wallet (4 Critical vulnerabilities)
2. **Governance** (Anchor) - Reputation-based DAO (6 Critical, 3 High, 2 Medium)
3. **AMM** (Anchor) - Automated Market Maker (9 Critical, 2 High, 3 Medium)
4. **Escrow** (Pinocchio) - Atomic token swap escrow
5. **NFT Minting** (Anchor) - On-chain NFT minting with Metaplex Core

Each program includes side-by-side secure/vulnerable implementations with comprehensive tests.

---

## **Security Severity Classification**

All vulnerabilities are classified using this standard:

### **CRITICAL**

Bugs that cause **direct loss of funds** with minimal setup. Attacker can trigger with little preparation or even accidentally. Effects are difficult to undo after detection.

Examples: Missing signer checks, arbitrary CPI, uninitialized account exploits

---

### **HIGH**

Bugs that enable **loss of funds with preparation**, or render the contract unusable/DOS (or in a locked-out state).

Examples: Missing authorization checks, reentrancy, integer overflow

---

### **MEDIUM**

Bugs that don't cause direct fund loss but **lead to exploitable mechanisms**.

Examples: Weak validation, precision loss, missing slippage protection

---

### **LOW**

Bugs with **no significant immediate impact**, easily fixed after detection. This can also include wrong decisions in code, not harmful, just best practices.

Examples: Suboptimal gas usage, missing events, incomplete error messages

---

## **Repository Structure**

Tests are properly implemented in each program with LiteSVM for fast testing. You can `cd` into any of the folders and run tests.

```
Soteria/
├── programs/
│   ├── multisig/                 # Multi-signature wallet (Anchor)
│   │   ├── m-secure/             # Secure implementation
│   │   │   ├── src/              # 4 instructions, 40+ security checks
│   │   │   └── tests/            # 4 comprehensive tests
│   │   ├── m-vulnerable/         # Vulnerable implementation
│   │   │   ├── src/              # Missing critical checks
│   │   │   ├── tests/            # 4 exploit demonstrations
│   │   │   └── VULNERABILITIES.md
│   │   └── README.md             # Side-by-side comparison
│   │
│   ├── governance/               # Reputation-based DAO (Anchor)
│   │   ├── g-secure/             # Secure implementation
│   │   │   ├── src/              # 8 instructions, 50+ security checks
│   │   │   └── tests/            # 5 comprehensive tests
│   │   ├── g-vulnerable/         # Vulnerable implementation
│   │   │   ├── src/              # 10+ intentional vulnerabilities
│   │   │   ├── tests/            # 6 exploit demonstrations
│   │   │   └── VULNERABILITIES.md
│   │   └── README.md             # Side-by-side comparison
│   │
│   ├── amm/                      # Automated Market Maker (Anchor)
│   │   ├── amm-secure/           # Secure implementation
│   │   │   ├── src/              # 6 instructions, 30+ security checks
│   │   │   └── tests/            # 5 comprehensive tests
│   │   ├── amm-vulnerable/       # Vulnerable implementation
│   │   │   ├── src/              # 14 intentional vulnerabilities
│   │   │   ├── tests/            # 7 exploit demonstrations
│   │   │   ├── VULNERABILITIES.md
│   │   │   └── TESTING.md
│   │   └── README.md             # Side-by-side comparison
│   │
│   ├── pino-escrow/              # Atomic swap escrow (Pinocchio)
│   │   ├── p-vulnerable/           # Shows security flaws
│   │   ├── p-secure/               # Shows fixes
│   │   └── README.md             # Vulnerabilities explained
│   │
│   └── nfts/                     # NFT minting (Anchor + Metaplex Core)
│       ├── n-secure/           # Secure implementation
│       ├── n-vulnerable/       # Vulnerable implementation
│       └── README.md
│
├── test-runner.sh             # Run all tests across all programs
│
└── README.md                     # This file
```

---

## **Quick Start**

### **1. Clone the Repository**

```bash
git clone https://github.com/AlphaR2/Soteria.git
cd Soteria
```

### **2. Install Prerequisites**

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Install Solana CLI (v2.0+)
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"

# Install Anchor CLI (v0.32.1+) - for Anchor programs
cargo install --git https://github.com/coral-xyz/anchor avm --force
avm install latest
avm use latest
```

### **3. Run the Interactive Test Runner**

```bash
# Make executable (first time only)
chmod +x test-runner.sh

# Launch interactive menu
./test-runner.sh
```

**The test runner provides:**
- Build all programs or individual versions
- Run secure tests (exploits prevented)
- Run exploit tests (attacks succeed)
- Execute all test suites sequentially
- Color-coded output with progress indicators

### **4. Explore Programs**

Each program directory contains:
- `README.md` - Architecture, vulnerabilities, attack scenarios
- `VULNERABILITIES.md` - Detailed security analysis
- `TESTING.md` - Testing guide and examples
- Side-by-side secure/vulnerable implementations

---

## **Programs Overview**

| Program | Framework | Vulnerabilities | Test Count |
|---------|-----------|-----------------|------------|
| **Multisig** | Anchor | 4 Critical | 4 secure + 4 exploit |
| **Governance** | Anchor | 6 Critical, 3 High, 2 Medium | 5 secure + 6 exploit |
| **AMM** | Anchor | 9 Critical, 2 High, 3 Medium | 5 secure + 7 exploit |
| **Escrow** | Pinocchio | TBD | TBD |
| **NFT Minting** | Anchor + Metaplex | TBD | TBD |

**See individual program READMEs for:**
- Detailed vulnerability documentation
- Attack scenario walkthroughs
- Side-by-side security comparisons
- Complete test documentation

---


## **Testing Approach**

All programs use **LiteSVM** for fast Rust-based testing:

- No validator required
- Tests run instantly in Rust
- Comprehensive exploit demonstrations
- Side-by-side secure vs vulnerable comparisons

Each test follows: **Setup → Exploit → Result → Impact → Lesson**

Run tests via the interactive test runner (recommended) or manually from program directories.

---


## **Recommended Path**

1. **Multisig** → Basic security (signer checks, threshold validation)
2. **Governance** → State management, cooldowns, reputation systems
3. **AMM** → DeFi mechanics, slippage, economic attacks
4. **Escrow** → Pinocchio framework patterns
5. **NFT Minting** → Metaplex integration

---

## **Common Vulnerability Patterns**

| Category | Examples |
|----------|----------|
| **Authorization** | Missing signer checks, owner validation, RBAC |
| **Validation** | Input boundaries, state transitions, account ownership |
| **Economic** | Slippage manipulation, inflation attacks, front-running |
| **Arithmetic** | Integer overflow/underflow, precision loss |
| **State** | Uninitialized accounts, replay attacks, missing cooldowns |
| **Time-based** | Expiration validation, cooldown enforcement, stale transactions |

---

## **Core Security Principles**

1. **Defense in Depth** - Validate explicitly even when lower layers enforce constraints
2. **Least Privilege** - Verify every signer, check every authority, validate every account
3. **Fail Securely** - Reject by default, use allow-lists, handle all errors
4. **Validate Everything** - Inputs, account data, time parameters, arithmetic, state

---


## **Additional Resources**

- [Solana Documentation](https://docs.solana.com/)
- [Anchor Book](https://book.anchor-lang.com/)
- [Neodyme Security Blog](https://blog.neodyme.io/)
- [Solana Security Best Practices](https://github.com/slowmist/solana-smart-contract-security-best-practices)
- [LiteSVM Testing](https://github.com/LiteSVM/litesvm)

---


## **Disclaimer**

**WARNING:** These programs contain **intentionally vulnerable** code for educational purposes.

- Vulnerable versions should **NEVER** be deployed to production
- Demonstrates real-world attack vectors found in production systems
- Use secure versions as reference implementations only
- Always perform thorough audits before mainnet deployment

**This repository is for learning and security education only.**

---

## **License**

Educational use only. Not intended for production deployment.

---

**Coming Next:** Lending Protocol • Staking • Oracle • More governance patterns
