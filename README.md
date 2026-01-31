# **Soteria - A Solana program Security implementation**

Repository demonstrating critical security vulnerabilities in Solana programs through 6 production-grade examples.

---

## **Overview**

Each program contains **vulnerable** and **secure** implementations with real exploit demonstrations.

**Programs:**

1. **Escrow** (Pinocchio)
2. **DAO Governance** (Anchor)
3. **Multisig** (Anchor) 
4. **AMM** (Anchor) 
5. **NFT Minting** (Anchor) - On-chain minting with Metaplex Core


**Coming Soon:**

1. **DAO Governance** (Pinocchio) - Voting & proposals
2. **Lending** (Anchor) - Collateralized loans
3. ..more


---

## **Security Severity Classification**

All vulnerabilities are classified using this standard:

### **CRITICAL**

Bugs that cause **direct loss of funds** with minimal setup. Attacker can trigger with little preparation or even accidentally. Effects are difficult to undo after detection.

---

### **HIGH**

Bugs that enable **loss of funds with preparation**, or render the contract unusable/DOS(Or in a locked-out state).

---

### **MEDIUM**

Bugs that don't cause direct fund loss but **lead to exploitable mechanisms**.

---

### **LOW**

Bugs with **no significant immediate impact**, easily fixed after detection. This can also include wrong decisions in code, not harmful, just best practices

---

## **Repository Structure**

Tests are properly implemented in each program with molecular(fuzz) testing enabled. You can `cd` into any of the folders and run a test, see test cmd towards end of readme

```
programs/
├── pino-escrow/              # Pinocchio implementation
│   ├── vulnerable/           # Shows security flaws
│   ├── secure/               # Shows fixes
│   └── README.md             # Vulnerabilities explained
│
├── governance/                 # Pinocchio implementation
│   ├── vulnerable/
│   ├── secure/
│   └── README.md
│
├── vault/                    # Anchor implementation
│   ├── vulnerable/
│   ├── secure/
│   └── README.md
│
├── amm/                      # Anchor implementation
│   ├── vulnerable/
│   ├── secure/
│   └── README.md
│
├── lending/                  # Anchor implementation
│   ├── vulnerable/
│   ├── secure/
│   └── README.md
│
└── nft-minting/              # Anchor implementation
    ├── vulnerable/
    ├── secure/
    └── README.md
```

---

## **Quick Start**

### **Prerequisites**

```bash

#quick installation
curl --proto '=https' --tlsv1.2 -sSfL https://solana-install.solana.workers.dev | bash

```

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Install Solana CLI (v2+)
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"

# Install Anchor CLI (v0.30.1+)
cargo install --git https://github.com/coral-xyz/anchor avm --force
avm install latest
avm use latest

# Install Node.js dependencies
npm install
# or
yarn install
```

---

### **Build Programs**

```bash
#build individually -
cd programs/vault/vulnerable && cargo build-sbf
cd programs/vault/secure && cargo build-sbf
```

---

### **Test a Specific Program**

#### **Pinocchio Programs (Escrow, DAO)**

```bash
# Show exploits work on vulnerable version
cd programs/pino-escrow/vulnerable
cargo

# Show fixes prevent exploits
cd ../secure
cargo
```

#### **Anchor Programs (Vault, AMM, Lending, NFT Minting)**

```bash
# Test vulnerable version - exploits should succeed
cd programs/vault
anchor build
anchor test ./tests/exploit.ts

# Test secure version - exploits should fail
anchor build
anchor test ./tests/secure.ts
```

---

---

## **Vulnerability Coverage**

see GRADING.md for full coverage of all issues

## **Each Program README Contains:**

- What the program does
- 4 security vulnerabilities with:
  - Severity classification
  - Vulnerable code location
  - Exploit scenario
  - Fix explanation
  - Real-world impact
- Anchor vs Pinocchio comparison (where applicable)
- Test instructions
- Learning resources

---

## **Disclaimer**

These programs are **intentionally vulnerable** for example purposes. The "vulnerable" versions should **NEVER** be deployed for production.



# **Next to cover**
- Pinocchio Governance 
- Lending protocol in Anchor 

---
