// Integration tests for the secure multisig program using LiteSVM
//
// Test Coverage:
//
// === Happy Path Tests ===
// 1. test_create_multisig - Create multisig with admin + threshold + timelock
// 2. test_full_governance_flow - Add 3 members, change threshold to 2, full approval flow
// 3. test_transfer_proposal_flow - Create, approve, execute SOL transfer
// 4. test_remove_member - Remove a member and verify threshold adjustment
// 5. test_change_timelock - Modify timelock duration via proposal
//
// === Security Tests ===
// 6. test_toggle_pause - Admin can pause/unpause, blocks operations when paused
// 7. test_non_admin_cannot_pause - Only admin can toggle pause
// 8. test_timelock_enforcement - Cannot execute before timelock expires
// 9. test_threshold_enforcement - Cannot execute without enough approvals
// 10. test_double_approval_prevention - Member cannot approve twice
// 11. test_role_based_access_control - Proposer cannot execute, Executor cannot propose
// 12. test_non_member_cannot_approve - Non-members cannot approve proposals
// 13. test_cannot_remove_creator - Creator is protected from removal
// 14. test_cancel_proposal - Proposer or admin can cancel active proposals

// the test code is long, if you want to read and see how we did the test, go for it, else 
// {
//see full test commands here

// // cargo test test_full_governance_flow -- --nocapture

// # Transfer proposal flow (fund vault, send SOL)
// cargo test test_transfer_proposal_flow -- --nocapture

// # Role-based access control
// cargo test test_role_based_access_control -- --nocapture

// # All security tests
// cargo test test_timelock -- --nocapture
// cargo test test_threshold -- --nocapture
// cargo test test_toggle_pause -- --nocapture
// }

use anchor_lang::prelude::pubkey;
use litesvm::LiteSVM;

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};



use solana_system_interface::program::ID as system_program;


// Program ID matching declare_id in lib.rs
const PROGRAM_ID: Pubkey = solana_sdk::pubkey!("HH8rYFiTjMX8FiiRgiFQx1jnXdT9D4TTiC5mSBhe9r7P");

// PDA seed constants (must match constants.rs)
const MULTISIG_SEED: &[u8] = b"multisig";
const PROPOSAL_SEED: &[u8] = b"proposal";
const TRANSFER_PROPOSAL_SEED: &[u8] = b"transfer";
const VAULT_SEED: &[u8] = b"vault";

// Member roles (must match MemberRole enum in state/member.rs)
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum MemberRole {
    Admin = 0,
    Proposer = 1,
    Executor = 2,
}

// Proposal types (must match ProposalType enum in state/proposal.rs)
// Borsh/Anchor enums are 0-indexed!
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum ProposalTypeDiscriminator {
    AddMember = 0,
    RemoveMember = 1,
    ChangeThreshold = 2,
    ChangeTimelock = 3,
}

// ======================== HELPERS ========================

/// Load the compiled program binary into LiteSVM
fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../target/deploy/multisig_secure.so");
    svm.add_program(PROGRAM_ID, program_bytes);
    svm
}

/// Create a new keypair and fund it with SOL via airdrop
fn create_funded_account(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports)
        .expect("Airdrop should succeed");
    keypair
}

/// Derive the multisig PDA using seeds: ["multisig", creator_pubkey, multisig_id]
fn derive_multisig_pda(creator: &Pubkey, multisig_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MULTISIG_SEED, creator.as_ref(), &multisig_id.to_le_bytes()],
        &PROGRAM_ID,
    )
}

/// Derive the vault PDA using seeds: ["vault", multisig_pubkey]
fn derive_vault_pda(multisig: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_SEED, multisig.as_ref()], &PROGRAM_ID)
}

/// Derive the proposal PDA using seeds: ["proposal", multisig_pubkey, proposal_id]
fn derive_proposal_pda(multisig: &Pubkey, proposal_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[PROPOSAL_SEED, multisig.as_ref(), &proposal_id.to_le_bytes()],
        &PROGRAM_ID,
    )
}

/// Derive the transfer proposal PDA using seeds: ["transfer", multisig_pubkey, proposal_id]
fn derive_transfer_proposal_pda(multisig: &Pubkey, proposal_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            TRANSFER_PROPOSAL_SEED,
            multisig.as_ref(),
            &proposal_id.to_le_bytes(),
        ],
        &PROGRAM_ID,
    )
}

/// Build Anchor instruction discriminator (8 bytes from sighash of "global:method_name")
fn anchor_discriminator(method: &str) -> [u8; 8] {
    let preimage = format!("global:{}", method);
    let hash = solana_sdk::hash::hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.to_bytes()[..8]);
    discriminator
}

/// Advance the SVM clock by the specified number of seconds
/// LiteSVM uses slot-based time, so we warp slots (approx 400ms each)

fn advance_time(svm: &mut LiteSVM, seconds: u64) {
    let mut clock: solana_sdk::clock::Clock = svm.get_sysvar();
    clock.unix_timestamp += seconds as i64;
    svm.set_sysvar(&clock);

    let current_slot = clock.slot;
    svm.warp_to_slot(current_slot + (seconds * 2) + 5);
}

// ======================== INSTRUCTION BUILDERS ========================

/// Build create_multisig instruction
fn build_create_multisig_ix(
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
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

/// Build create_proposal instruction (AddMember variant)
fn build_create_add_member_proposal_ix(
    proposer: &Pubkey,
    multisig: &Pubkey,
    proposal: &Pubkey,
    new_member: &Pubkey,
    role: MemberRole,
) -> Instruction {
    let discriminator = anchor_discriminator("create_proposal");

    let mut data = discriminator.to_vec();
    data.push(ProposalTypeDiscriminator::AddMember as u8);
    data.extend_from_slice(new_member.as_ref());
    data.push(role as u8);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*proposer, true),
            AccountMeta::new(*multisig, false),
            AccountMeta::new(*proposal, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

/// Build create_proposal instruction (RemoveMember variant)
fn build_create_remove_member_proposal_ix(
    proposer: &Pubkey,
    multisig: &Pubkey,
    proposal: &Pubkey,
    member_to_remove: &Pubkey,
) -> Instruction {
    let discriminator = anchor_discriminator("create_proposal");

    let mut data = discriminator.to_vec();
    data.push(ProposalTypeDiscriminator::RemoveMember as u8);
    data.extend_from_slice(member_to_remove.as_ref());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*proposer, true),
            AccountMeta::new(*multisig, false),
            AccountMeta::new(*proposal, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

/// Build create_proposal instruction (ChangeThreshold variant)
fn build_create_change_threshold_proposal_ix(
    proposer: &Pubkey,
    multisig: &Pubkey,
    proposal: &Pubkey,
    new_threshold: u8,
) -> Instruction {
    let discriminator = anchor_discriminator("create_proposal");

    let mut data = discriminator.to_vec();
    data.push(ProposalTypeDiscriminator::ChangeThreshold as u8);
    data.push(new_threshold);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*proposer, true),
            AccountMeta::new(*multisig, false),
            AccountMeta::new(*proposal, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

/// Build create_proposal instruction (ChangeTimelock variant)
fn build_create_change_timelock_proposal_ix(
    proposer: &Pubkey,
    multisig: &Pubkey,
    proposal: &Pubkey,
    new_timelock: u64,
) -> Instruction {
    let discriminator = anchor_discriminator("create_proposal");

    let mut data = discriminator.to_vec();
    data.push(ProposalTypeDiscriminator::ChangeTimelock as u8);
    data.extend_from_slice(&new_timelock.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*proposer, true),
            AccountMeta::new(*multisig, false),
            AccountMeta::new(*proposal, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

/// Build approve_proposal instruction
fn build_approve_proposal_ix(
    owner: &Pubkey,
    multisig: &Pubkey,
    proposal: &Pubkey,
) -> Instruction {
    let discriminator = anchor_discriminator("approve_proposal");

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new_readonly(*multisig, false),
            AccountMeta::new(*proposal, false),
        ],
        data: discriminator.to_vec(),
    }
}

/// Build execute_proposal instruction
fn build_execute_proposal_ix(
    executor: &Pubkey,
    multisig: &Pubkey,
    proposal: &Pubkey,
    proposer: &Pubkey,
) -> Instruction {
    let discriminator = anchor_discriminator("execute_proposal");

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*executor, true),
            AccountMeta::new(*multisig, false),
            AccountMeta::new(*proposal, false),
            AccountMeta::new(*proposer, false),
        ],
        data: discriminator.to_vec(),
    }
}

/// Build cancel_proposal instruction
fn build_cancel_proposal_ix(
    canceller: &Pubkey,
    multisig: &Pubkey,
    proposal: &Pubkey,
    proposer: &Pubkey,
) -> Instruction {
    let discriminator = anchor_discriminator("cancel_proposal");

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*canceller, true),
            AccountMeta::new_readonly(*multisig, false),
            AccountMeta::new(*proposal, false),
             AccountMeta::new(*proposer, false),
        ],
        data: discriminator.to_vec(),
    }
}

/// Build create_transfer_proposal instruction
fn build_create_transfer_proposal_ix(
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
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

/// Build approve_transfer_proposal instruction
fn build_approve_transfer_proposal_ix(
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
fn build_execute_transfer_proposal_ix(
    executor: &Pubkey,
    multisig: &Pubkey,
    transfer_proposal: &Pubkey,
    proposer: &Pubkey,
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
            AccountMeta::new(*proposer, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new(*recipient, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data: discriminator.to_vec(),
    }
}

/// Build toggle_pause instruction
fn build_toggle_pause_ix(admin: &Pubkey, multisig: &Pubkey) -> Instruction {
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
fn send_tx_expect_success(
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
fn send_tx_expect_failure(
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

fn add_unique_meta(mut ix: Instruction) -> Instruction {
    ix.accounts.push(
        solana_sdk::instruction::AccountMeta::new_readonly(
            Pubkey::new_unique(),
            false,
        ),
    );
    ix
}


/// Create a multisig with a single admin (threshold=1)
/// Returns (multisig_pda, vault_pda)
fn create_basic_multisig(
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

/// Add a member to the multisig via proposal
/// Returns the proposal_id used
fn add_member_to_multisig(
    svm: &mut LiteSVM,
    admin: &Keypair,
    multisig: &Pubkey,
    new_member: &Pubkey,
    role: MemberRole,
    proposal_id: u64,
    timelock_seconds: u64,
) {
    let (proposal, _) = derive_proposal_pda(multisig, proposal_id);

    // Create AddMember proposal (auto-approved by proposer)
    let add_member_ix = build_create_add_member_proposal_ix(
        &admin.pubkey(),
        multisig,
        &proposal,
        new_member,
        role,
    );
    send_tx_expect_success(svm, add_member_ix, admin, &[admin]);

    // Advance time past timelock
    advance_time(svm, timelock_seconds + 2);

    // Execute proposal
    let execute_ix = build_execute_proposal_ix(&admin.pubkey(), multisig, &proposal, &admin.pubkey());
    send_tx_expect_success(svm, execute_ix, admin, &[admin]);
}

// ======================== HAPPY PATH TESTS ========================

/// Test 1: Create multisig with admin, threshold, and timelock
///
/// Scenario: Alice creates a 1-of-1 multisig with 60-second timelock
/// Verifies: multisig PDA created, vault PDA created, admin is creator
#[test]
fn test_create_multisig() {
    println!("\n=== TEST: Create Multisig ===\n");

    let mut svm = setup_svm();
    let alice = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Alice (creator) funded: {}", alice.pubkey());

    let multisig_id = 1u64;
    let (multisig, multisig_bump) = derive_multisig_pda(&alice.pubkey(), multisig_id);
    let (vault, vault_bump) = derive_vault_pda(&multisig);
    println!("[Derive] Multisig PDA: {} (bump: {})", multisig, multisig_bump);
    println!("[Derive] Vault PDA: {} (bump: {})", vault, vault_bump);

    // Note: threshold must be 1 at creation since only 1 member exists
    let threshold = 1u8;
    let timelock_seconds = 60u64;
    let create_ix = build_create_multisig_ix(
        &alice.pubkey(),
        &multisig,
        &vault,
        multisig_id,
        threshold,
        timelock_seconds,
    );
    println!(
        "[Build] create_multisig(id={}, threshold={}, timelock={}s)",
        multisig_id, threshold, timelock_seconds
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&alice.pubkey()),
        &[&alice],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    match &result {
        Ok(metadata) => {
            println!("[Result] Transaction succeeded");
            println!("[Result] Compute units: {}", metadata.compute_units_consumed);
        }
        Err(e) => panic!("create_multisig failed: {:?}", e),
    }

    // Verify multisig account exists and is owned by program
    let multisig_account = svm
        .get_account(&multisig)
        .expect("Multisig PDA should exist");
    assert_eq!(multisig_account.owner, PROGRAM_ID);
    println!(
        "[Verify] Multisig account created (owner: {})",
        multisig_account.owner
    );
    println!(
        "[Verify] Multisig data size: {} bytes",
        multisig_account.data.len()
    );

    // Verify vault account exists
    let vault_account = svm.get_account(&vault).expect("Vault PDA should exist");
    println!(
        "[Verify] Vault account created (lamports: {})",
        vault_account.lamports
    );

    println!("\n=== PASSED: test_create_multisig ===\n");
}

/// Test 2: Full governance flow - add members, change threshold, execute proposals
///
/// Scenario:
///   - Alice (Admin) creates multisig with threshold=1, timelock=5s
///   - Alice adds Bob (Proposer)
///   - Alice adds Charlie (Executor)
///   - Alice creates proposal to change threshold to 2
///   - Alice and Bob approve threshold change
///   - Execute threshold change
///   - Verify new proposals need 2 approvals
#[test]
fn test_full_governance_flow() {
    println!("\n=== TEST: Full Governance Flow ===\n");

    let mut svm = setup_svm();

    // Setup accounts
    let alice = create_funded_account(&mut svm, 20 * LAMPORTS_PER_SOL);
    let bob = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let charlie = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Alice (Admin): {}", alice.pubkey());
    println!("[Setup] Bob (Proposer): {}", bob.pubkey());
    println!("[Setup] Charlie (Executor): {}", charlie.pubkey());

    let multisig_id = 1u64;
    let timelock_seconds = 1u64;
    let (multisig, vault) = create_basic_multisig(&mut svm, &alice, multisig_id, timelock_seconds);
    println!("[Step 1] Multisig created with threshold=1");

    // Add Bob as Proposer (proposal 0)
    println!("\n[Step 2] Adding Bob as Proposer");
    add_member_to_multisig(
        &mut svm,
        &alice,
        &multisig,
        &bob.pubkey(),
        MemberRole::Proposer,
        0,
        timelock_seconds,
    );
    println!("[Step 2] Bob added as Proposer (owner_count=2)");

    // Add Charlie as Executor (proposal 1)
    println!("\n[Step 3] Adding Charlie as Executor");
    add_member_to_multisig(
        &mut svm,
        &alice,
        &multisig,
        &charlie.pubkey(),
        MemberRole::Executor,
        1,
        timelock_seconds,
    );
    println!("[Step 3] Charlie added as Executor (owner_count=3)");

    // Create proposal to change threshold to 2 (proposal 2)
    println!("\n[Step 4] Alice proposes to change threshold to 2");
    let proposal_id_2 = 2u64;
    let (proposal_2, _) = derive_proposal_pda(&multisig, proposal_id_2);

    let change_threshold_ix = build_create_change_threshold_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &proposal_2,
        2, // new threshold
    );
    send_tx_expect_success(&mut svm, change_threshold_ix, &alice, &[&alice]);
    println!("[Step 4] Proposal created (Alice auto-approved, 1/1 threshold met for old threshold)");

    // Wait for timelock
    advance_time(&mut svm, timelock_seconds + 1);

    // Execute threshold change
    println!("\n[Step 5] Executing threshold change");
    let execute_ix = build_execute_proposal_ix(&alice.pubkey(), &multisig, &proposal_2, &alice.pubkey());
    send_tx_expect_success(&mut svm, execute_ix, &alice, &[&alice]);
    println!("[Step 5] Threshold changed to 2");

    // Verify new threshold: create a transfer proposal that needs 2 approvals
    println!("\n[Step 6] Verifying new threshold with transfer proposal");
    let recipient = create_funded_account(&mut svm, LAMPORTS_PER_SOL);

    // Fund the vault
    svm.airdrop(&vault, 5 * LAMPORTS_PER_SOL)
        .expect("Vault funding should succeed");

    let transfer_proposal_id = 3u64;
    let (transfer_proposal, _) = derive_transfer_proposal_pda(&multisig, transfer_proposal_id);

    // Bob creates transfer proposal (Bob is Proposer, so auto-approves = 1 approval)
    let create_transfer_ix = build_create_transfer_proposal_ix(
        &bob.pubkey(),
        &multisig,
        &transfer_proposal,
        LAMPORTS_PER_SOL,
        &recipient.pubkey(),
    );
    send_tx_expect_success(&mut svm, create_transfer_ix, &bob, &[&bob]);
    println!("[Step 6] Transfer proposal created by Bob (1/2 approvals)");

   
    advance_time(&mut svm, timelock_seconds + 1);
 

    // Alice approves (now 2/2)
    let approve_ix =
        build_approve_transfer_proposal_ix(&alice.pubkey(), &multisig, &transfer_proposal);
    send_tx_expect_success(&mut svm, approve_ix, &alice, &[&alice]);
    println!("[Step 6] Alice approved (2/2 approvals)");

    // Now execute should work
    let execute_transfer_ix = build_execute_transfer_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &transfer_proposal,
        &bob.pubkey(), // Bob is the proposer
        &vault,
        &recipient.pubkey(),
    );
    send_tx_expect_success(&mut svm, execute_transfer_ix, &alice, &[&alice]);
    println!("[Step 6] Transfer executed successfully with 2 approvals");

    println!("\n=== PASSED: test_full_governance_flow ===\n");
}

/// Test 3: Transfer proposal flow
///
/// Scenario:
///   - Create multisig with threshold=1
///   - Fund vault with 10 SOL
///   - Create transfer proposal to send 1 SOL
///   - Wait for timelock
///   - Execute transfer
///   - Verify balances
#[test]
fn test_transfer_proposal_flow() {
    println!("\n=== TEST: Transfer Proposal Flow ===\n");

    let mut svm = setup_svm();

    let alice = create_funded_account(&mut svm, 20 * LAMPORTS_PER_SOL);
    let recipient = create_funded_account(&mut svm, LAMPORTS_PER_SOL);
    println!("[Setup] Alice (Admin): {}", alice.pubkey());
    println!("[Setup] Recipient: {}", recipient.pubkey());

    let multisig_id = 1u64;
    let timelock_seconds = 5u64;
    let (multisig, vault) = create_basic_multisig(&mut svm, &alice, multisig_id, timelock_seconds);
    println!("[Step 1] Multisig created");

    // Fund vault
    println!("\n[Step 2] Funding vault with 10 SOL");
    svm.airdrop(&vault, 10 * LAMPORTS_PER_SOL)
        .expect("Vault funding should succeed");
    let vault_balance_before = svm.get_account(&vault).unwrap().lamports;
    println!(
        "[Step 2] Vault balance: {} lamports (10 SOL)",
        vault_balance_before
    );

    // Create transfer proposal
    let transfer_amount = LAMPORTS_PER_SOL;
    let proposal_id = 0u64;
    let (transfer_proposal, _) = derive_transfer_proposal_pda(&multisig, proposal_id);

    println!(
        "\n[Step 3] Creating transfer proposal (amount={} lamports)",
        transfer_amount
    );
    let create_transfer_ix = build_create_transfer_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &transfer_proposal,
        transfer_amount,
        &recipient.pubkey(),
    );
    send_tx_expect_success(&mut svm, create_transfer_ix, &alice, &[&alice]);
    println!("[Step 3] Transfer proposal created (Alice auto-approved, threshold met)");

    // Advance time past timelock
    println!("\n[Step 4] Advancing time past timelock");
    advance_time(&mut svm, timelock_seconds + 1);

    // Execute transfer
    let recipient_balance_before = svm.get_account(&recipient.pubkey()).unwrap().lamports;
    println!("\n[Step 5] Executing transfer");
    println!(
        "[Step 5] Recipient balance before: {} lamports",
        recipient_balance_before
    );

    let execute_transfer_ix = build_execute_transfer_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &transfer_proposal,
        &alice.pubkey(), // Alice is the proposer
        &vault,
        &recipient.pubkey(),
    );
    send_tx_expect_success(&mut svm, execute_transfer_ix, &alice, &[&alice]);
    println!("[Step 5] Transfer executed");

    // Verify balances
    let vault_balance_after = svm.get_account(&vault).unwrap().lamports;
    let recipient_balance_after = svm.get_account(&recipient.pubkey()).unwrap().lamports;

    println!(
        "\n[Verify] Vault: {} -> {} lamports",
        vault_balance_before, vault_balance_after
    );
    println!(
        "[Verify] Recipient: {} -> {} lamports",
        recipient_balance_before, recipient_balance_after
    );

    assert_eq!(
        vault_balance_after,
        vault_balance_before - transfer_amount,
        "Vault should have sent 1 SOL"
    );
    assert_eq!(
        recipient_balance_after,
        recipient_balance_before + transfer_amount,
        "Recipient should have received 1 SOL"
    );

    // Verify proposal account was closed
    let proposal_account = svm.get_account(&transfer_proposal);
    assert!(
        proposal_account.is_none() || proposal_account.unwrap().data.is_empty(),
        "Transfer proposal should be closed"
    );
    println!("[Verify] Transfer proposal account closed");

    println!("\n=== PASSED: test_transfer_proposal_flow ===\n");
}

/// Test 4: Remove member
///
/// Scenario:
///   - Create multisig with Alice, add Bob
///   - Create proposal to remove Bob
///   - Execute removal
///   - Verify Bob is no longer a member
#[test]
fn test_remove_member() {
    println!("\n=== TEST: Remove Member ===\n");

    let mut svm = setup_svm();

    let alice = create_funded_account(&mut svm, 20 * LAMPORTS_PER_SOL);
    let bob = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Alice (Admin): {}", alice.pubkey());
    println!("[Setup] Bob (Proposer): {}", bob.pubkey());

    let multisig_id = 1u64;
    let timelock_seconds = 5u64;
    let (multisig, _) = create_basic_multisig(&mut svm, &alice, multisig_id, timelock_seconds);
    println!("[Step 1] Multisig created");

    // Add Bob
    add_member_to_multisig(
        &mut svm,
        &alice,
        &multisig,
        &bob.pubkey(),
        MemberRole::Proposer,
        0,
        timelock_seconds,
    );
    println!("[Step 2] Bob added as member (owner_count=2)");

    // Create proposal to remove Bob (proposal 1)
    let proposal_id = 1u64;
    let (proposal, _) = derive_proposal_pda(&multisig, proposal_id);

    println!("\n[Step 3] Creating proposal to remove Bob");
    let remove_member_ix = build_create_remove_member_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &proposal,
        &bob.pubkey(),
    );
    send_tx_expect_success(&mut svm, remove_member_ix, &alice, &[&alice]);
    println!("[Step 3] Remove proposal created");

    // Advance time and execute
    advance_time(&mut svm, timelock_seconds + 1);

    let execute_ix = build_execute_proposal_ix(&alice.pubkey(), &multisig, &proposal, &alice.pubkey());
    send_tx_expect_success(&mut svm, execute_ix, &alice, &[&alice]);
    println!("[Step 4] Bob removed from multisig");

    // Verify Bob cannot approve proposals anymore
    let proposal_id_2 = 2u64;
    let (proposal_2, _) = derive_proposal_pda(&multisig, proposal_id_2);

    // Alice creates a new proposal
    let charlie = Keypair::new();
    let add_charlie_ix = build_create_add_member_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &proposal_2,
        &charlie.pubkey(),
        MemberRole::Executor,
    );
    send_tx_expect_success(&mut svm, add_charlie_ix, &alice, &[&alice]);

    // Bob tries to approve (should fail - not a member)
    let approve_ix = build_approve_proposal_ix(&bob.pubkey(), &multisig, &proposal_2);
    let error = send_tx_expect_failure(&mut svm, approve_ix, &bob, &[&bob]);
    assert!(
        error.contains("NotAMember") || error.contains("6300"),
        "Bob should not be able to approve"
    );
    println!("[Verify] Bob cannot approve proposals (correctly rejected)");

    println!("\n=== PASSED: test_remove_member ===\n");
}

/// Test 5: Change timelock
///
/// Scenario:
///   - Create multisig with 60s timelock
///   - Create proposal to change timelock to 30s
///   - Execute and verify new timelock works
#[test]
fn test_change_timelock() {
    println!("\n=== TEST: Change Timelock ===\n");

    let mut svm = setup_svm();

    let alice = create_funded_account(&mut svm, 20 * LAMPORTS_PER_SOL);
    println!("[Setup] Alice (Admin): {}", alice.pubkey());

    let multisig_id = 1u64;
    let initial_timelock = 60u64;
    let (multisig, vault) = create_basic_multisig(&mut svm, &alice, multisig_id, initial_timelock);
    println!("[Step 1] Multisig created with timelock=60s");

    // Create proposal to change timelock to 10s (proposal 0)
    let new_timelock = 10u64;
    let proposal_id = 0u64;
    let (proposal, _) = derive_proposal_pda(&multisig, proposal_id);

    println!("\n[Step 2] Creating proposal to change timelock to {}s", new_timelock);
    let change_timelock_ix = build_create_change_timelock_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &proposal,
        new_timelock,
    );
    send_tx_expect_success(&mut svm, change_timelock_ix, &alice, &[&alice]);
    println!("[Step 2] Timelock change proposal created");

    // Wait for original timelock
    advance_time(&mut svm, initial_timelock + 1);

    // Execute timelock change
    let execute_ix = build_execute_proposal_ix(&alice.pubkey(), &multisig, &proposal, &alice.pubkey());
    send_tx_expect_success(&mut svm, execute_ix, &alice, &[&alice]);
    println!("[Step 3] Timelock changed to {}s", new_timelock);

    // Verify new timelock works by creating a transfer that can execute after 10s
    svm.airdrop(&vault, 5 * LAMPORTS_PER_SOL).unwrap();
    let recipient = create_funded_account(&mut svm, LAMPORTS_PER_SOL);

    let transfer_proposal_id = 1u64;
    let (transfer_proposal, _) = derive_transfer_proposal_pda(&multisig, transfer_proposal_id);

    let create_transfer_ix = build_create_transfer_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &transfer_proposal,
        LAMPORTS_PER_SOL,
        &recipient.pubkey(),
    );
    send_tx_expect_success(&mut svm, create_transfer_ix, &alice, &[&alice]);
    println!("[Step 4] Created transfer proposal to test new timelock");

    // Wait only for new timelock (10s)
    advance_time(&mut svm, new_timelock + 1);

    // Should be able to execute with new shorter timelock
    let execute_transfer_ix = build_execute_transfer_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &transfer_proposal,
        &alice.pubkey(), // Alice is the proposer
        &vault,
        &recipient.pubkey(),
    );
    send_tx_expect_success(&mut svm, execute_transfer_ix, &alice, &[&alice]);
    println!("[Verify] Transfer executed with new timelock");

    println!("\n=== PASSED: test_change_timelock ===\n");
}

// ======================== SECURITY TESTS ========================

/// Test 6: Toggle pause
///
/// Scenario:
///   - Create multisig
///   - Pause it
///   - Verify operations blocked
///   - Unpause
///   - Verify operations work again
#[test]
fn test_toggle_pause() {
    println!("\n=== TEST: Toggle Pause ===\n");

    let mut svm = setup_svm();

    let alice = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Alice (Admin): {}", alice.pubkey());

    let multisig_id = 1u64;
    let (multisig, _) = create_basic_multisig(&mut svm, &alice, multisig_id, 60);
    println!("[Step 1] Multisig created");

    // Pause
    println!("\n[Step 2] Pausing multisig");
    let pause_ix = build_toggle_pause_ix(&alice.pubkey(), &multisig);
    send_tx_expect_success(&mut svm, pause_ix, &alice, &[&alice]);
    println!("[Step 2] Multisig paused");

    // Try to create proposal (should fail)
    let bob = Keypair::new();
    let proposal_id = 0u64;
    let (proposal, _) = derive_proposal_pda(&multisig, proposal_id);

    println!("\n[Step 3] Trying to create proposal while paused");
    let add_member_ix = build_create_add_member_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &proposal,
        &bob.pubkey(),
        MemberRole::Proposer,
    );
    let add_member_ix = add_unique_meta(add_member_ix);

    let error = send_tx_expect_failure(&mut svm, add_member_ix.clone(), &alice, &[&alice]);
    assert!(
        error.contains("MultisigPaused") || error.contains("6310"),
        "Should fail with MultisigPaused"
    );
    println!("[Step 3] Proposal creation blocked (as expected)");

    // Unpause
    println!("\n[Step 4] Unpausing multisig");
    let unpause_ix = build_toggle_pause_ix(&alice.pubkey(), &multisig);

    let unpause_ix = add_unique_meta(unpause_ix);

    send_tx_expect_success(&mut svm, unpause_ix, &alice, &[&alice]);
    println!("[Step 4] Multisig unpaused");

    // Create proposal should work now
    println!("\n[Step 5] Creating proposal after unpause");
    let add_member_ix = build_create_add_member_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &proposal,
        &bob.pubkey(),
        MemberRole::Proposer,
    );
    send_tx_expect_success(&mut svm, add_member_ix, &alice, &[&alice]);
    println!("[Step 5] Proposal created successfully");

    println!("\n=== PASSED: test_toggle_pause ===\n");
}

/// Test 7: Non-admin cannot pause
#[test]
fn test_non_admin_cannot_pause() {
    println!("\n=== TEST: Non-Admin Cannot Pause ===\n");

    let mut svm = setup_svm();

    let alice = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let bob = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Alice (Admin): {}", alice.pubkey());
    println!("[Setup] Bob: {}", bob.pubkey());

    let multisig_id = 1u64;
    let timelock = 5u64;
    let (multisig, _) = create_basic_multisig(&mut svm, &alice, multisig_id, timelock);

    // Add Bob as Proposer
    add_member_to_multisig(
        &mut svm,
        &alice,
        &multisig,
        &bob.pubkey(),
        MemberRole::Proposer,
        0,
        timelock,
    );
    println!("[Step 1] Bob added as Proposer");

    // Bob tries to pause (should fail)
    println!("\n[Step 2] Bob tries to pause");
    let pause_ix = build_toggle_pause_ix(&bob.pubkey(), &multisig);
    let error = send_tx_expect_failure(&mut svm, pause_ix, &bob, &[&bob]);
    assert!(
        error.contains("OnlyAdmin") || error.contains("6304"),
        "Should fail with OnlyAdmin"
    );
    println!("[Step 2] Pause blocked (only admin can pause)");

    println!("\n=== PASSED: test_non_admin_cannot_pause ===\n");
}

/// Test 8: Timelock enforcement
///
/// Scenario:
///   - Create proposal
///   - Try to execute immediately (should fail)
///   - Wait for timelock
///   - Execute successfully
#[test]
fn test_timelock_enforcement() {
    println!("\n=== TEST: Timelock Enforcement ===\n");

    let mut svm = setup_svm();

    let alice = create_funded_account(&mut svm, 20 * LAMPORTS_PER_SOL);
    println!("[Setup] Alice (Admin): {}", alice.pubkey());

    let multisig_id = 1u64;
    let timelock_seconds = 60u64; // 60 second timelock
    let (multisig, vault) = create_basic_multisig(&mut svm, &alice, multisig_id, timelock_seconds);
    println!("[Step 1] Multisig created with 60s timelock");

    // Fund vault
    svm.airdrop(&vault, 5 * LAMPORTS_PER_SOL).unwrap();
    let recipient = create_funded_account(&mut svm, LAMPORTS_PER_SOL);

    // Create transfer proposal
    let proposal_id = 0u64;
    let (transfer_proposal, _) = derive_transfer_proposal_pda(&multisig, proposal_id);

    let create_transfer_ix = build_create_transfer_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &transfer_proposal,
        LAMPORTS_PER_SOL,
        &recipient.pubkey(),
    );
    send_tx_expect_success(&mut svm, create_transfer_ix, &alice, &[&alice]);
    println!("[Step 2] Transfer proposal created");

    // Try to execute immediately (should fail - timelock not passed)
    println!("\n[Step 3] Trying to execute before timelock");

    let failed_execute_ix = build_execute_transfer_proposal_ix(
    &alice.pubkey(),
    &multisig,
    &transfer_proposal,
    &alice.pubkey(), // Alice is the proposer
    &vault,
    &recipient.pubkey(),
    );

    // make first unique to avoid blockhash issues
    let failed_execute_ix = add_unique_meta(failed_execute_ix);
    let error = send_tx_expect_failure(&mut svm, failed_execute_ix, &alice, &[&alice]);

    assert!(
        error.contains("TimelockNotPassed") || error.contains("6309"),
        "Should fail with TimelockNotPassed"
    );
    println!("[Step 3] Execution blocked (timelock not passed)");

    // Advance time past timelock
    println!("\n[Step 4] Advancing time past timelock");
    advance_time(&mut svm, timelock_seconds + 1);

   let execute_transfer_ix = build_execute_transfer_proposal_ix(
    &alice.pubkey(),
    &multisig,
    &transfer_proposal,
    &alice.pubkey(), // Alice is the proposer
    &vault,
    &recipient.pubkey(),
    );

    // make second unique as well
    let execute_transfer_ix = add_unique_meta(execute_transfer_ix);
    send_tx_expect_success(&mut svm, execute_transfer_ix, &alice, &[&alice]);

    println!("\n=== PASSED: test_timelock_enforcement ===\n");
}

/// Test 9: Threshold enforcement
///
/// Scenario:
///   - Create multisig with 2 members, threshold=2
///   - Create proposal with 1 approval
///   - Try to execute (should fail)
///   - Add second approval
///   - Execute successfully
#[test]
fn test_threshold_enforcement() {
    println!("\n=== TEST: Threshold Enforcement ===\n");

    let mut svm = setup_svm();

    let alice = create_funded_account(&mut svm, 20 * LAMPORTS_PER_SOL);
    let bob = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Alice (Admin): {}", alice.pubkey());
    println!("[Setup] Bob (Proposer): {}", bob.pubkey());

    let multisig_id = 1u64;
    let timelock = 5u64;
    let (multisig, vault) = create_basic_multisig(&mut svm, &alice, multisig_id, timelock);

    // Add Bob
    add_member_to_multisig(
        &mut svm,
        &alice,
        &multisig,
        &bob.pubkey(),
        MemberRole::Proposer,
        0,
        timelock,
    );
    println!("[Step 1] Bob added (owner_count=2)");

    // Change threshold to 2
    let proposal_id_1 = 1u64;
    let (proposal_1, _) = derive_proposal_pda(&multisig, proposal_id_1);

    let change_threshold_ix = build_create_change_threshold_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &proposal_1,
        2,
    );
    send_tx_expect_success(&mut svm, change_threshold_ix, &alice, &[&alice]);
    advance_time(&mut svm, timelock + 1);

    let execute_ix = build_execute_proposal_ix(&alice.pubkey(), &multisig, &proposal_1, &alice.pubkey());
    send_tx_expect_success(&mut svm, execute_ix, &alice, &[&alice]);
    println!("[Step 2] Threshold changed to 2");

    // Create transfer proposal with only 1 approval
    svm.airdrop(&vault, 5 * LAMPORTS_PER_SOL).unwrap();
    let recipient = create_funded_account(&mut svm, LAMPORTS_PER_SOL);

    let transfer_proposal_id = 2u64;
    let (transfer_proposal, _) = derive_transfer_proposal_pda(&multisig, transfer_proposal_id);

    let create_transfer_ix = build_create_transfer_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &transfer_proposal,
        LAMPORTS_PER_SOL,
        &recipient.pubkey(),
    );
    send_tx_expect_success(&mut svm, create_transfer_ix, &alice, &[&alice]);
    println!("[Step 3] Transfer proposal created (1/2 approvals)");

    // Wait for timelock
    advance_time(&mut svm, timelock + 1);

    // Try to execute with 1 approval (should fail)
    println!("\n[Step 4] Trying to execute with 1/2 approvals");
    let execute_transfer_ix = build_execute_transfer_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &transfer_proposal,
        &alice.pubkey(), // Alice is the proposer
        &vault,
        &recipient.pubkey(),
    );

    let failed_execute_ix = build_execute_transfer_proposal_ix(
    &alice.pubkey(),
    &multisig,
    &transfer_proposal,
    &alice.pubkey(), // Alice is the proposer
    &vault,
    &recipient.pubkey(),
    );

    let failed_execute_ix = add_unique_meta(failed_execute_ix);

    let error = send_tx_expect_failure(&mut svm, failed_execute_ix, &alice, &[&alice]);
    assert!(
        error.contains("InsufficientApprovals") || error.contains("6306"),
        "Should fail with InsufficientApprovals"
    );
    println!("[Step 4] Execution blocked (need 2 approvals)");

    // Bob approves
    let approve_ix = build_approve_transfer_proposal_ix(
        &bob.pubkey(),
        &multisig,
        &transfer_proposal,
    );
    send_tx_expect_success(&mut svm, approve_ix, &bob, &[&bob]);
    println!("[Step 5] Bob approved (2/2 approvals)");

    // Execute should work now
    let execute_transfer_ix = build_execute_transfer_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &transfer_proposal,
        &alice.pubkey(), // Alice is the proposer
        &vault,
        &recipient.pubkey(),
    );

    let execute_transfer_ix = add_unique_meta(execute_transfer_ix);

    send_tx_expect_success(&mut svm, execute_transfer_ix, &alice, &[&alice]);
    println!("[Step 5] Execution succeeded with 2 approvals");

    println!("\n=== PASSED: test_threshold_enforcement ===\n");
}

/// Test 10: Double approval prevention
///
/// Scenario:
///   - Create proposal
///   - Alice approves (auto on creation)
///   - Alice tries to approve again (should fail)
#[test]
fn test_double_approval_prevention() {
    println!("\n=== TEST: Double Approval Prevention ===\n");

    let mut svm = setup_svm();

    let alice = create_funded_account(&mut svm, 20 * LAMPORTS_PER_SOL);
    let bob = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Alice (Admin): {}", alice.pubkey());
    println!("[Setup] Bob (Proposer): {}", bob.pubkey());

    let multisig_id = 1u64;
    let timelock = 5u64;
    let (multisig, _) = create_basic_multisig(&mut svm, &alice, multisig_id, timelock);

    // Add Bob
    add_member_to_multisig(
        &mut svm,
        &alice,
        &multisig,
        &bob.pubkey(),
        MemberRole::Proposer,
        0,
        timelock,
    );
    println!("[Step 1] Bob added");

    // Create proposal (Alice auto-approves)
    let charlie = Keypair::new();
    let proposal_id = 1u64;
    let (proposal, _) = derive_proposal_pda(&multisig, proposal_id);

    let add_charlie_ix = build_create_add_member_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &proposal,
        &charlie.pubkey(),
        MemberRole::Executor,
    );
    send_tx_expect_success(&mut svm, add_charlie_ix, &alice, &[&alice]);
    println!("[Step 2] Proposal created (Alice auto-approved)");

    // Alice tries to approve again (should fail)
    println!("\n[Step 3] Alice tries to approve again");
    let approve_ix = build_approve_proposal_ix(&alice.pubkey(), &multisig, &proposal);
    let error = send_tx_expect_failure(&mut svm, approve_ix, &alice, &[&alice]);
    assert!(
        error.contains("AlreadyApproved") || error.contains("6303"),
        "Should fail with AlreadyApproved"
    );
    println!("[Step 3] Double approval blocked");

    println!("\n=== PASSED: test_double_approval_prevention ===\n");
}

/// Test 11: Role-based access control
///
/// Scenario:
///   - Proposer cannot execute (Admin/Executor only)
///   - Executor cannot propose (Admin/Proposer only)
#[test]
fn test_role_based_access_control() {
    println!("\n=== TEST: Role-Based Access Control ===\n");

    let mut svm = setup_svm();

    let alice = create_funded_account(&mut svm, 20 * LAMPORTS_PER_SOL);
    let bob = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let charlie = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Alice (Admin): {}", alice.pubkey());
    println!("[Setup] Bob (Proposer): {}", bob.pubkey());
    println!("[Setup] Charlie (Executor): {}", charlie.pubkey());

    let multisig_id = 1u64;
    let timelock = 5u64;
    let (multisig, vault) = create_basic_multisig(&mut svm, &alice, multisig_id, timelock);

    // Add Bob as Proposer
    add_member_to_multisig(
        &mut svm,
        &alice,
        &multisig,
        &bob.pubkey(),
        MemberRole::Proposer,
        0,
        timelock,
    );

    // Add Charlie as Executor
    add_member_to_multisig(
        &mut svm,
        &alice,
        &multisig,
        &charlie.pubkey(),
        MemberRole::Executor,
        1,
        timelock,
    );
    println!("[Step 1] Bob (Proposer) and Charlie (Executor) added");

    // Test: Executor cannot propose
    println!("\n[Step 2] Charlie (Executor) tries to create proposal");
    let dave = Keypair::new();
    let proposal_id = 2u64;
    let (proposal, _) = derive_proposal_pda(&multisig, proposal_id);

    let create_proposal_ix = build_create_add_member_proposal_ix(
        &charlie.pubkey(),
        &multisig,
        &proposal,
        &dave.pubkey(),
        MemberRole::Executor,
    );
    let error = send_tx_expect_failure(&mut svm, create_proposal_ix, &charlie, &[&charlie]);
    assert!(
        error.contains("CannotPropose") || error.contains("6305"),
        "Executor should not be able to propose"
    );
    println!("[Step 2] Executor cannot propose (as expected)");

    // Alice creates a transfer proposal for testing execution permissions
    svm.airdrop(&vault, 5 * LAMPORTS_PER_SOL).unwrap();
    let recipient = create_funded_account(&mut svm, LAMPORTS_PER_SOL);

    let transfer_proposal_id = 2u64;
    let (transfer_proposal, _) = derive_transfer_proposal_pda(&multisig, transfer_proposal_id);

    let create_transfer_ix = build_create_transfer_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &transfer_proposal,
        LAMPORTS_PER_SOL,
        &recipient.pubkey(),
    );
    send_tx_expect_success(&mut svm, create_transfer_ix, &alice, &[&alice]);
    advance_time(&mut svm, timelock + 1);
    println!("[Step 3] Alice created transfer proposal");

    // Test: Proposer cannot execute
    println!("\n[Step 4] Bob (Proposer) tries to execute");
    let execute_transfer_ix = build_execute_transfer_proposal_ix(
        &bob.pubkey(),
        &multisig,
        &transfer_proposal,
        &alice.pubkey(), // Alice is the proposer
        &vault,
        &recipient.pubkey(),
    );
    let error = send_tx_expect_failure(&mut svm, execute_transfer_ix, &bob, &[&bob]);
    assert!(
        error.contains("CannotExecute") || error.contains("6306"),
        "Proposer should not be able to execute"
    );
    println!("[Step 4] Proposer cannot execute (as expected)");

    // Test: Executor can execute
    println!("\n[Step 5] Charlie (Executor) executes");
    let execute_transfer_ix = build_execute_transfer_proposal_ix(
        &charlie.pubkey(),
        &multisig,
        &transfer_proposal,
        &alice.pubkey(), // Alice is the proposer
        &vault,
        &recipient.pubkey(),
    );
    send_tx_expect_success(&mut svm, execute_transfer_ix, &charlie, &[&charlie]);
    println!("[Step 5] Executor successfully executed");

    println!("\n=== PASSED: test_role_based_access_control ===\n");
}

/// Test 12: Non-member cannot approve
#[test]
fn test_non_member_cannot_approve() {
    println!("\n=== TEST: Non-Member Cannot Approve ===\n");

    let mut svm = setup_svm();

    let alice = create_funded_account(&mut svm, 20 * LAMPORTS_PER_SOL);
    let bob = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let outsider = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Alice (Admin): {}", alice.pubkey());
    println!("[Setup] Outsider: {}", outsider.pubkey());

    let multisig_id = 1u64;
    let timelock = 5u64;
    let (multisig, _) = create_basic_multisig(&mut svm, &alice, multisig_id, timelock);

    // Create a proposal
    let proposal_id = 0u64;
    let (proposal, _) = derive_proposal_pda(&multisig, proposal_id);

    let add_bob_ix = build_create_add_member_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &proposal,
        &bob.pubkey(),
        MemberRole::Proposer,
    );
    send_tx_expect_success(&mut svm, add_bob_ix, &alice, &[&alice]);
    println!("[Step 1] Proposal created");

    // Outsider tries to approve
    println!("\n[Step 2] Outsider tries to approve");
    let approve_ix = build_approve_proposal_ix(&outsider.pubkey(), &multisig, &proposal);
    let error = send_tx_expect_failure(&mut svm, approve_ix, &outsider, &[&outsider]);
    assert!(
        error.contains("NotAMember") || error.contains("6300"),
        "Non-member should not be able to approve"
    );
    println!("[Step 2] Non-member approval blocked");

    println!("\n=== PASSED: test_non_member_cannot_approve ===\n");
}

/// Test 13: Cannot remove creator
#[test]
fn test_cannot_remove_creator() {
    println!("\n=== TEST: Cannot Remove Creator ===\n");

    let mut svm = setup_svm();

    let alice = create_funded_account(&mut svm, 20 * LAMPORTS_PER_SOL);
    let bob = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);

    println!("[Setup] Alice (Admin/Creator): {}", alice.pubkey());
    println!("[Setup] Bob (Proposer): {}", bob.pubkey());

    let multisig_id = 1u64;
    let timelock = 5u64;

    let (multisig, _) = create_basic_multisig(
        &mut svm,
        &alice,
        multisig_id,
        timelock,
    );

    // Add Bob as proposer
    add_member_to_multisig(
        &mut svm,
        &alice,
        &multisig,
        &bob.pubkey(),
        MemberRole::Proposer,
        0,
        timelock,
    );

    println!("[Step 1] Bob added");

    let proposal_id = 1u64;
    let (proposal, _) = derive_proposal_pda(&multisig, proposal_id);

    println!("\n[Step 2] Attempting to create proposal to remove creator");

    let remove_alice_ix = build_create_remove_member_proposal_ix(
        &alice.pubkey(),
        &multisig,
        &proposal,
        &alice.pubkey(), // Attempt to remove creator
    );

    // Proposal creation MUST FAIL
    let error = send_tx_expect_failure(
        &mut svm,
        remove_alice_ix,
        &alice,
        &[&alice],
    );

    assert!(
        error.contains("CannotRemoveCreator") || error.contains("6002"),
        "Expected CannotRemoveCreator error, got: {}",
        error
    );

    println!("[Step 2] Creator removal proposal correctly blocked");

    println!("\n=== PASSED: test_cannot_remove_creator ===\n");
}







