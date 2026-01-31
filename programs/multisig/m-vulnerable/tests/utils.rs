// Common test utilities for exploit tests to be shared across multiple test files

use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;

// Program ID matching declare_id!
pub const PROGRAM_ID: Pubkey = solana_sdk::pubkey!("2Skteich3Jdz4W41oek3wrwdFSFRJcgvaAT7H1bxGvck");

// PDA seed constants (must match constants.rs)
pub const MULTISIG_SEED: &[u8] = b"multisig";
pub const PROPOSAL_SEED: &[u8] = b"proposal";
pub const TRANSFER_PROPOSAL_SEED: &[u8] = b"transfer";
pub const VAULT_SEED: &[u8] = b"vault";

// Member roles (must match MemberRole enum)
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum MemberRole {
    Admin = 0,
    Proposer = 1,
    Executor = 2,
}

// Proposal types (must match ProposalType enum - 0-indexed!)
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ProposalTypeDiscriminator {
    AddMember = 0,
    RemoveMember = 1,
    ChangeThreshold = 2,
    ChangeTimelock = 3,
}

// ======================== HELPERS ========================

/// Build Anchor instruction discriminator (8 bytes from sighash of "global:method_name")
pub fn anchor_discriminator(method: &str) -> [u8; 8] {
    let preimage = format!("global:{}", method);
    let hash = solana_sdk::hash::hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.to_bytes()[..8]);
    discriminator
}

/// Load the compiled program binary into LiteSVM
pub fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../target/deploy/multisig_vulnerable.so");
    svm.add_program(PROGRAM_ID, program_bytes);
    svm
}

/// Create a new keypair and fund it with SOL via airdrop
pub fn create_funded_account(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports)
        .expect("Airdrop should succeed");
    keypair
}

/// Derive the multisig PDA using seeds: ["multisig", creator_pubkey, multisig_id]
pub fn derive_multisig_pda(creator: &Pubkey, multisig_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MULTISIG_SEED, creator.as_ref(), &multisig_id.to_le_bytes()],
        &PROGRAM_ID,
    )
}

/// Derive the vault PDA using seeds: ["vault", multisig_pubkey]
pub fn derive_vault_pda(multisig: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_SEED, multisig.as_ref()], &PROGRAM_ID)
}

/// Derive the proposal PDA using seeds: ["proposal", multisig_pubkey, proposal_id]
pub fn derive_proposal_pda(multisig: &Pubkey, proposal_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[PROPOSAL_SEED, multisig.as_ref(), &proposal_id.to_le_bytes()],
        &PROGRAM_ID,
    )
}

/// Derive the transfer proposal PDA using seeds: ["transfer", multisig_pubkey, proposal_count]
/// NOTE: Use get_multisig_proposal_count() to get the current count before creating proposal
pub fn derive_transfer_proposal_pda(multisig: &Pubkey, proposal_count: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            TRANSFER_PROPOSAL_SEED,
            multisig.as_ref(),
            &proposal_count.to_le_bytes(),
        ],
        &PROGRAM_ID,
    )
}

/// Get the current proposal_count from a multisig account
pub fn get_multisig_proposal_count(svm: &LiteSVM, multisig: &Pubkey) -> u64 {
    let account = svm.get_account(multisig).expect("Multisig account should exist");

    // Multisig account layout:
    // 8 bytes: discriminator
    // 8 bytes: multisig_id
    // 32 bytes: creator
    // 1 byte: threshold
    // 1 byte: owner_count
    // (32 + 1) * 10 bytes: members array
    // 8 bytes: proposal_count <- we want this

    let offset = 8 + 8 + 32 + 1 + 1 + (33 * 10);
    let mut proposal_count_bytes = [0u8; 8];
    proposal_count_bytes.copy_from_slice(&account.data[offset..offset + 8]);
    u64::from_le_bytes(proposal_count_bytes)
}

/// Advance the SVM clock by the specified number of seconds
pub fn advance_time(svm: &mut LiteSVM, seconds: u64) {
    let mut clock: solana_sdk::clock::Clock = svm.get_sysvar();
    clock.unix_timestamp += seconds as i64;
    svm.set_sysvar(&clock);
}

// ======================== INSTRUCTION BUILDERS ========================

/// Build create_multisig instruction
pub fn build_create_multisig_ix(
    creator: &Pubkey,
    multisig: &Pubkey,
    vault: &Pubkey,
    multisig_id: u64,
    threshold: u8,
    timelock_seconds: u64,
) -> Instruction {
    let discriminator = anchor_discriminator("create_multisig");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&multisig_id.to_le_bytes());
    data.extend_from_slice(&[threshold]);
    data.extend_from_slice(&timelock_seconds.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*creator, true),
            AccountMeta::new(*multisig, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build create_transfer_proposal instruction
pub fn build_create_transfer_proposal_ix(
    proposer: &Pubkey,
    multisig: &Pubkey,
    transfer_proposal: &Pubkey,
    amount: u64,
    recipient: &Pubkey,
) -> Instruction {
    let discriminator = anchor_discriminator("create_transfer_proposal");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(recipient.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*proposer, true),
            AccountMeta::new(*multisig, false),
            AccountMeta::new(*transfer_proposal, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

/// Build approve_transfer_proposal instruction
pub fn build_approve_transfer_proposal_ix(
    owner: &Pubkey,
    multisig: &Pubkey,
    transfer_proposal: &Pubkey,
) -> Instruction {
    let discriminator = anchor_discriminator("approve_transfer_proposal");

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new_readonly(*multisig, false),
            AccountMeta::new(*transfer_proposal, false),
        ],
        data: discriminator.to_vec(),
    }
}

/// Build execute_transfer_proposal instruction
/// Note: m-vulnerable does NOT have proposer account (rent goes to executor instead)
pub fn build_execute_transfer_proposal_ix(
    executor: &Pubkey,
    multisig: &Pubkey,
    transfer_proposal: &Pubkey,
    vault: &Pubkey,
    recipient: &Pubkey,
) -> Instruction {
    let discriminator = anchor_discriminator("execute_transfer_proposal");

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*executor, true),
            AccountMeta::new(*multisig, false),
            AccountMeta::new(*transfer_proposal, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new(*recipient, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: discriminator.to_vec(),
    }
}

/// Build toggle_pause instruction
pub fn build_toggle_pause_ix(admin: &Pubkey, multisig: &Pubkey) -> Instruction {
    let discriminator = anchor_discriminator("toggle_pause");

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(*multisig, false),
        ],
        data: discriminator.to_vec(),
    }
}

// ======================== TRANSACTION HELPERS ========================

/// Send a transaction and expect success
pub fn send_tx_expect_success(
    svm: &mut LiteSVM,
    ix: Instruction,
    payer: &Keypair,
    signers: &[&Keypair],
) {
    let blockhash = svm.latest_blockhash();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        signers,
        blockhash,
    );

    svm.send_transaction(tx)
        .expect("Transaction should succeed");
}

/// Send a transaction and expect failure
pub fn send_tx_expect_failure(
    svm: &mut LiteSVM,
    ix: Instruction,
    payer: &Keypair,
    signers: &[&Keypair],
) -> String {
    let blockhash = svm.latest_blockhash();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        signers,
        blockhash,
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Transaction should have failed");
    format!("{:?}", result.err().unwrap())
}

// ======================== SETUP HELPERS ========================

/// Multisig scenario setup result
pub struct MultisigScenario {
    pub svm: LiteSVM,
    pub creator: Keypair,
    pub member1: Keypair,
    pub member2: Keypair,
    pub member3: Keypair,
    pub attacker: Keypair,
    pub multisig_id: u64,
    pub multisig_pda: Pubkey,
    pub vault_pda: Pubkey,
}

/// Helper to set up a complete multisig scenario with funded accounts
pub fn setup_multisig_scenario(_threshold: u8, _timelock: u64) -> MultisigScenario {
    let mut svm = setup_svm();

    // Create funded accounts
    let creator = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let member1 = create_funded_account(&mut svm, 5 * LAMPORTS_PER_SOL);
    let member2 = create_funded_account(&mut svm, 5 * LAMPORTS_PER_SOL);
    let member3 = create_funded_account(&mut svm, 5 * LAMPORTS_PER_SOL);
    let attacker = create_funded_account(&mut svm, 5 * LAMPORTS_PER_SOL);

    // Derive PDAs
    let multisig_id = 1u64;
    let (multisig_pda, _) = derive_multisig_pda(&creator.pubkey(), multisig_id);
    let (vault_pda, _) = derive_vault_pda(&multisig_pda);

    MultisigScenario {
        svm,
        creator,
        member1,
        member2,
        member3,
        attacker,
        multisig_id,
        multisig_pda,
        vault_pda,
    }
}


pub fn create_basic_multisig(
    svm: &mut LiteSVM,
    creator: &Keypair,
    multisig_id: u64,
    timelock_seconds: u64,
) -> (Pubkey, Pubkey) {
    let (multisig, _) = derive_multisig_pda(&creator.pubkey(), multisig_id);
    let (vault, _) = derive_vault_pda(&multisig);

    let create_ix = build_create_multisig_ix(
        &creator.pubkey(),
        &multisig,
        &vault,
        multisig_id,
        1, // threshold must be 1 at creation (only 1 member)
        timelock_seconds,
    );

    send_tx_expect_success(svm, create_ix, creator, &[creator]);

    (multisig, vault)
}

/// Helper to create a multisig with custom owners
pub fn create_multisig_with_owners(
    svm: &mut LiteSVM,
    creator: &Keypair,
    multisig_pda: &Pubkey,
    vault_pda: &Pubkey,
    multisig_id: u64,
    threshold: u8,
    timelock_seconds: u64,
    owners: &[(Pubkey, MemberRole)],
) {
    let discriminator = anchor_discriminator("create_multisig");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&multisig_id.to_le_bytes());
    data.push(threshold);
    data.extend_from_slice(&timelock_seconds.to_le_bytes());

    // Vec<(Pubkey, MemberRole)> serialization: length (4 bytes) + items
    data.extend_from_slice(&(owners.len() as u32).to_le_bytes());
    for (pubkey, role) in owners {
        data.extend_from_slice(&pubkey.to_bytes());
        data.push(*role as u8);
    }

    let create_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(creator.pubkey(), true),
            AccountMeta::new(*multisig_pda, false),
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    };

    send_tx_expect_success(svm, create_ix, creator, &[creator]);
}

/// Helper to fund the vault using airdrop
pub fn fund_vault(svm: &mut LiteSVM, vault_pda: &Pubkey, amount: u64) {
    svm.airdrop(vault_pda, amount)
        .expect("Vault funding should succeed");
}

//DATA-ONLY BUILDERS (for manual instruction construction) 

/// Build approve_transfer_proposal instruction data (discriminator only)
pub fn build_approve_transfer_proposal_data() -> Vec<u8> {
    let discriminator = anchor_discriminator("approve_transfer_proposal");
    discriminator.to_vec()
}

/// Build execute_transfer_proposal instruction data (discriminator only)
pub fn build_execute_transfer_proposal_data() -> Vec<u8> {
    let discriminator = anchor_discriminator("execute_transfer_proposal");
    discriminator.to_vec()
}

/// Build toggle_pause instruction data (discriminator only)
pub fn build_toggle_pause_data() -> Vec<u8> {
    let discriminator = anchor_discriminator("toggle_pause");
    discriminator.to_vec()
}
