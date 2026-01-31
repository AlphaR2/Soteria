// Integration tests for the secure pino-escrow program using LiteSVM
//
// Tests cover happy path for both instructions:
// 1. ProposeOffer - Proposer creates an escrow offer and deposits Token A into vault
// 2. TakeOffer - Taker accepts the offer, completing the atomic token swap
//
// Uses litesvm-token helpers for SPL token setup (mints, ATAs, minting)

use litesvm::LiteSVM;
use litesvm_token::{
    CreateAssociatedTokenAccount, CreateMint, MintTo,
    spl_token::state::Account as TokenAccount,
    get_spl_account,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;

// Program ID matching declare_id!("J8Ru6Zti7EwTwVt35BGN2irvD1ELEjv2MkCYGAbCqaok")
const PROGRAM_ID: Pubkey = Pubkey::new_from_array(p_secure::ID.to_bytes());

// Standard program IDs
const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = spl_associated_token_account::ID;

// Seed prefix must match MakeState::SEED_PREFIX in state/make.rs
const OFFER_SEED_PREFIX: &[u8] = b"offer";

// Token configuration
const DECIMALS: u8 = 9;

// Test amounts (with 9 decimals)
const INITIAL_MINT_AMOUNT: u64 = 1_000_000_000_000; // 1000 tokens
const TOKEN_A_OFFER_AMOUNT: u64 = 100_000_000_000;  // 100 tokens
const TOKEN_B_WANTED_AMOUNT: u64 = 50_000_000_000;  // 50 tokens

// Instruction discriminators (must match Instruction enum in instructions/mod.rs)
const PROPOSE_OFFER_DISCRIMINATOR: u8 = 0;
const TAKE_OFFER_DISCRIMINATOR: u8 = 1;


// ======================== HELPERS ========================

// Load the compiled program binary into LiteSVM
fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../target/deploy/secure.so");
    svm.add_program(PROGRAM_ID, program_bytes);
    svm
}

// Create a new keypair and fund it with SOL via airdrop
fn create_funded_account(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports)
        .expect("Airdrop should succeed");
    keypair
}

// Derive the offer PDA address using seeds: ["offer", maker_pubkey, offer_id]
fn derive_offer_pda(maker: &Pubkey, id: &[u8; 8]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[OFFER_SEED_PREFIX, maker.as_ref(), id],
        &PROGRAM_ID,
    )
}

// Build ProposeOffer instruction data
//
// Layout matches ProposalOfferData (#[repr(C)]) in propose_offer.rs:
//   [discriminator: u8][id: 8][token_b_wanted_amount: u64][token_a_offered_amount: u64][bump: u8][padding: 7]
//
// repr(C) adds 7 bytes padding after bump to align the struct to 8 bytes.
// size_of::<ProposalOfferData>() = 32 bytes. The discriminator is stripped before parsing,
// so the data after the discriminator must be exactly 32 bytes.
fn build_propose_offer_data(
    id: [u8; 8],
    token_b_wanted_amount: u64,
    token_a_offered_amount: u64,
    bump: u8,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(33); // 1 discriminator + 32 struct
    data.push(PROPOSE_OFFER_DISCRIMINATOR);
    data.extend_from_slice(&id);                                    // 8 bytes
    data.extend_from_slice(&token_b_wanted_amount.to_le_bytes());   // 8 bytes
    data.extend_from_slice(&token_a_offered_amount.to_le_bytes());  // 8 bytes
    data.push(bump);                                                // 1 byte
    data.extend_from_slice(&[0u8; 7]);                              // 7 bytes padding
    data
}

// Build TakeOffer instruction data
// TakeOffer has no extra data, just the discriminator byte
fn build_take_offer_data() -> Vec<u8> {
    vec![TAKE_OFFER_DISCRIMINATOR]
}


// ======================== TESTS ========================


// Test 1: ProposeOffer happy path
//
// Scenario: Maker creates an escrow offering 100 Token A in exchange for 50 Token B.
// Verifies: offer PDA created, vault funded, maker balance decreased.
#[test]
fn test_propose_offer() {
    println!("\n=== TEST: ProposeOffer ===\n");

    // Step 1: Setup LiteSVM and load the escrow program
    println!("[Setup] Initializing LiteSVM and loading program...");
    let mut svm = setup_svm();

    // Step 2: Create and fund the maker account with 10 SOL
    let payer = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Maker funded: {}", payer.pubkey());

    // Step 3: Create Token Mint A (what maker will offer)
    let mint_a = CreateMint::new(&mut svm, &payer)
        .authority(&payer.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Failed to create mint A");
    println!("[Setup] Mint A created: {}", mint_a);

    // Step 4: Create Token Mint B (what maker wants in return)
    let mint_b = CreateMint::new(&mut svm, &payer)
        .authority(&payer.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Failed to create mint B");
    println!("[Setup] Mint B created: {}", mint_b);

    // Step 5: Create maker's ATA for Token A
    let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_a)
        .owner(&payer.pubkey())
        .send()
        .expect("Failed to create maker ATA A");
    println!("[Setup] Maker ATA A created: {}", maker_ata_a);

    // Step 6: Mint 1000 Token A to maker's ATA
    MintTo::new(&mut svm, &payer, &mint_a, &maker_ata_a, INITIAL_MINT_AMOUNT)
        .owner(&payer)
        .send()
        .expect("Failed to mint to maker ATA A");

    // Verify maker starts with 1000 Token A
    let maker_balance_before: TokenAccount = get_spl_account(&svm, &maker_ata_a)
        .expect("Failed to read maker token account");
    assert_eq!(maker_balance_before.amount, INITIAL_MINT_AMOUNT);
    println!("[Setup] Maker Token A balance: {} (1000 tokens)", maker_balance_before.amount);

    // Step 7: Derive offer PDA and vault ATA addresses
    let offer_id: [u8; 8] = 1u64.to_le_bytes();
    let (offer_pda, bump) = derive_offer_pda(&payer.pubkey(), &offer_id);
    let vault_ata = get_associated_token_address(&offer_pda, &mint_a);
    println!("[Derive] Offer PDA: {} (bump: {})", offer_pda, bump);
    println!("[Derive] Vault ATA: {}", vault_ata);

    // Step 8: Build instruction data
    // Data layout: discriminator(1) + id(8) + token_b_wanted(8) + token_a_offered(8) + bump(1) + padding(7) = 33 bytes
    let ix_data = build_propose_offer_data(
        offer_id,
        TOKEN_B_WANTED_AMOUNT,
        TOKEN_A_OFFER_AMOUNT,
        bump,
    );
    println!("[Build] Instruction data: {} bytes", ix_data.len());

    // Step 9: Build instruction with account metas
    // Account order matches OfferAccounts struct in propose_offer.rs
    let propose_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),             // maker (signer, writable)
            AccountMeta::new_readonly(mint_a, false),           // token_mint_a
            AccountMeta::new_readonly(mint_b, false),           // token_mint_b
            AccountMeta::new(maker_ata_a, false),               // maker_ata_a (writable)
            AccountMeta::new(offer_pda, false),                 // offer PDA (writable)
            AccountMeta::new(vault_ata, false),                 // vault ATA (writable)
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_program
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false), // system_program
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false), // ata_program
        ],
        data: ix_data,
    };
    println!("[Build] Instruction built with {} accounts", propose_ix.accounts.len());

    // Step 10: Sign and send the transaction
    let tx = Transaction::new_signed_with_payer(
        &[propose_ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    println!("[Send] Sending ProposeOffer transaction...");
    let result = svm.send_transaction(tx);
    match &result {
        Ok(metadata) => {
            println!("[Result] ProposeOffer succeeded");
            println!("[Result] Compute units: {}", metadata.compute_units_consumed);
            println!("[Result] Logs:");
            for log in &metadata.logs {
                println!("         {}", log);
            }
        }
        Err(e) => panic!("ProposeOffer failed: {:?}", e),
    }

    // Step 11: Verify offer PDA was created and owned by our program
    let offer_account = svm.get_account(&offer_pda)
        .expect("Offer PDA should exist");
    assert_eq!(offer_account.owner, PROGRAM_ID);
    println!("[Verify] Offer PDA owner: {} (correct)", offer_account.owner);
    println!("[Verify] Offer PDA data size: {} bytes", offer_account.data.len());

    // Step 12: Verify vault holds the deposited 100 Token A
    let vault_account: TokenAccount = get_spl_account(&svm, &vault_ata)
        .expect("Failed to read vault");
    assert_eq!(vault_account.amount, TOKEN_A_OFFER_AMOUNT);
    println!("[Verify] Vault Token A balance: {} (100 tokens)", vault_account.amount);

    // Step 13: Verify maker's balance decreased from 1000 to 900 Token A
    let maker_balance_after: TokenAccount = get_spl_account(&svm, &maker_ata_a)
        .expect("Failed to read maker token account");
    assert_eq!(maker_balance_after.amount, INITIAL_MINT_AMOUNT - TOKEN_A_OFFER_AMOUNT);
    println!(
        "[Verify] Maker Token A balance: {} -> {} (transferred {} to vault)",
        maker_balance_before.amount,
        maker_balance_after.amount,
        maker_balance_before.amount - maker_balance_after.amount
    );

    println!("\n=== PASSED: test_propose_offer ===\n");
}


// Test 2: Full escrow flow (ProposeOffer + TakeOffer)
//
// Scenario:
//   - Proposer offers 100 Token A, wants 50 Token B
//   - Taker accepts, sends 50 Token B, receives 100 Token A
//   - Vault and offer PDA are closed after completion
//
// Verifies: all balances correct, vault closed, offer closed.
#[test]
fn test_full_escrow_flow() {
    println!("\n=== TEST: Full Escrow Flow ===\n");

    // ---------- SETUP ----------

    println!("[Setup] Initializing LiteSVM and loading program...");
    let mut svm = setup_svm();

    // Create three accounts: payer (mint authority), proposer (Sarah), taker (Steve)
    let payer = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let proposer = create_funded_account(&mut svm, 5 * LAMPORTS_PER_SOL);
    let taker = create_funded_account(&mut svm, 5 * LAMPORTS_PER_SOL);
    println!("[Setup] Payer:    {}", payer.pubkey());
    println!("[Setup] Proposer: {} (Sarah - offers Token A)", proposer.pubkey());
    println!("[Setup] Taker:    {} (Steve - offers Token B)", taker.pubkey());

    // Create Token Mint A (what proposer offers) and Token Mint B (what proposer wants)
    let mint_a = CreateMint::new(&mut svm, &payer)
        .authority(&payer.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Failed to create mint A");

    let mint_b = CreateMint::new(&mut svm, &payer)
        .authority(&payer.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Failed to create mint B");
    println!("[Setup] Mint A: {} (Token A - offered by proposer)", mint_a);
    println!("[Setup] Mint B: {} (Token B - wanted by proposer)", mint_b);

    // Create proposer's ATA for Token A and fund with 1000 tokens
    let proposer_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_a)
        .owner(&proposer.pubkey())
        .send()
        .expect("Failed to create proposer ATA A");

    MintTo::new(&mut svm, &payer, &mint_a, &proposer_ata_a, INITIAL_MINT_AMOUNT)
        .owner(&payer)
        .send()
        .expect("Failed to mint to proposer ATA A");
    println!("[Setup] Proposer ATA A: {} (funded with 1000 Token A)", proposer_ata_a);

    // Create taker's ATA for Token A (empty, will receive from vault)
    let taker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_a)
        .owner(&taker.pubkey())
        .send()
        .expect("Failed to create taker ATA A");
    println!("[Setup] Taker ATA A: {} (empty, will receive Token A)", taker_ata_a);

    // Create taker's ATA for Token B and fund with 1000 tokens
    let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_b)
        .owner(&taker.pubkey())
        .send()
        .expect("Failed to create taker ATA B");

    MintTo::new(&mut svm, &payer, &mint_b, &taker_ata_b, INITIAL_MINT_AMOUNT)
        .owner(&payer)
        .send()
        .expect("Failed to mint to taker ATA B");
    println!("[Setup] Taker ATA B: {} (funded with 1000 Token B)", taker_ata_b);

    // Derive offer PDA and vault ATA
    let offer_id: [u8; 8] = 1u64.to_le_bytes();
    let (offer_pda, bump) = derive_offer_pda(&proposer.pubkey(), &offer_id);
    let vault_ata = get_associated_token_address(&offer_pda, &mint_a);
    println!("[Derive] Offer PDA: {} (bump: {})", offer_pda, bump);
    println!("[Derive] Vault ATA: {} (will hold escrowed Token A)", vault_ata);

    // Proposer's ATA B does not exist yet - TakeOffer will create it
    let proposer_ata_b = get_associated_token_address(&proposer.pubkey(), &mint_b);
    println!("[Derive] Proposer ATA B: {} (does not exist, will be created during TakeOffer)", proposer_ata_b);


    // ---------- STEP 1: PROPOSE OFFER ----------

    println!("\n--- Step 1: ProposeOffer ---");
    println!("[ProposeOffer] Proposer offers 100 Token A, wants 50 Token B");

    let propose_ix_data = build_propose_offer_data(
        offer_id,
        TOKEN_B_WANTED_AMOUNT,
        TOKEN_A_OFFER_AMOUNT,
        bump,
    );

    // Account order matches OfferAccounts struct
    let propose_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(proposer.pubkey(), true),            // maker (signer, writable)
            AccountMeta::new_readonly(mint_a, false),             // token_mint_a
            AccountMeta::new_readonly(mint_b, false),             // token_mint_b
            AccountMeta::new(proposer_ata_a, false),              // maker_ata_a (writable)
            AccountMeta::new(offer_pda, false),                   // offer PDA (writable)
            AccountMeta::new(vault_ata, false),                   // vault ATA (writable)
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),   // token_program
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),  // system_program
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false), // ata_program
        ],
        data: propose_ix_data,
    };

    let propose_tx = Transaction::new_signed_with_payer(
        &[propose_ix],
        Some(&proposer.pubkey()),
        &[&proposer],
        svm.latest_blockhash(),
    );

    println!("[ProposeOffer] Sending transaction...");
    let propose_result = svm.send_transaction(propose_tx);
    match &propose_result {
        Ok(metadata) => {
            println!("[ProposeOffer] Transaction succeeded");
            println!("[ProposeOffer] Compute units: {}", metadata.compute_units_consumed);
            println!("[ProposeOffer] Logs:");
            for log in &metadata.logs {
                println!("               {}", log);
            }
        }
        Err(e) => panic!("ProposeOffer failed: {:?}", e),
    }

    // Verify vault received 100 Token A
    let vault_balance: TokenAccount = get_spl_account(&svm, &vault_ata)
        .expect("Vault should exist");
    assert_eq!(vault_balance.amount, TOKEN_A_OFFER_AMOUNT);
    println!("[ProposeOffer] Vault Token A balance: {} (100 tokens deposited)", vault_balance.amount);

    // Verify proposer's Token A decreased from 1000 to 900
    let proposer_a_after_propose: TokenAccount = get_spl_account(&svm, &proposer_ata_a)
        .expect("Proposer ATA A should exist");
    println!(
        "[ProposeOffer] Proposer Token A: {} -> {} (sent {} to vault)",
        INITIAL_MINT_AMOUNT,
        proposer_a_after_propose.amount,
        INITIAL_MINT_AMOUNT - proposer_a_after_propose.amount
    );


    // ---------- STEP 2: TAKE OFFER ----------

    println!("\n--- Step 2: TakeOffer ---");
    println!("[TakeOffer] Taker sends 50 Token B, receives 100 Token A");

    // Record pre-swap balances for comparison
    let taker_b_before: TokenAccount = get_spl_account(&svm, &taker_ata_b)
        .expect("Taker ATA B should exist");
    let taker_a_before: TokenAccount = get_spl_account(&svm, &taker_ata_a)
        .expect("Taker ATA A should exist");
    println!("[TakeOffer] Taker Token A before: {}", taker_a_before.amount);
    println!("[TakeOffer] Taker Token B before: {}", taker_b_before.amount);

    let take_ix_data = build_take_offer_data();

    // Account order matches TakeOfferAccounts struct in take_offer.rs
    let take_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(taker.pubkey(), true),             // taker (signer, writable)
            AccountMeta::new(proposer.pubkey(), false),         // proposer (writable, receives vault rent)
            AccountMeta::new(proposer_ata_b, false),            // proposer_ata_b (writable, created if needed)
            AccountMeta::new_readonly(mint_b, false),           // token_mint_b
            AccountMeta::new_readonly(mint_a, false),           // token_mint_a
            AccountMeta::new(taker_ata_a, false),               // taker_ata_a (writable, receives Token A)
            AccountMeta::new(taker_ata_b, false),               // taker_ata_b (writable, sends Token B)
            AccountMeta::new(offer_pda, false),                 // offer PDA (writable, will be closed)
            AccountMeta::new(vault_ata, false),                 // vault (writable, will be closed)
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_program
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false), // system_program
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false), // ata_program
        ],
        data: take_ix_data,
    };

    let take_tx = Transaction::new_signed_with_payer(
        &[take_ix],
        Some(&taker.pubkey()),
        &[&taker],
        svm.latest_blockhash(),
    );

    println!("[TakeOffer] Sending transaction...");
    let take_result = svm.send_transaction(take_tx);
    match &take_result {
        Ok(metadata) => {
            println!("[TakeOffer] Transaction succeeded");
            println!("[TakeOffer] Compute units: {}", metadata.compute_units_consumed);
            println!("[TakeOffer] Logs:");
            for log in &metadata.logs {
                println!("             {}", log);
            }
        }
        Err(e) => panic!("TakeOffer failed: {:?}", e),
    }


    // ---------- VERIFY FINAL STATE ----------

    println!("\n--- Verifying Final State ---");

    // Taker should now have 100 Token A (received from vault)
    let taker_a_after: TokenAccount = get_spl_account(&svm, &taker_ata_a)
        .expect("Taker ATA A should exist");
    assert_eq!(taker_a_after.amount, TOKEN_A_OFFER_AMOUNT);
    println!(
        "[Verify] Taker Token A: {} -> {} (received {} from vault)",
        taker_a_before.amount,
        taker_a_after.amount,
        taker_a_after.amount - taker_a_before.amount
    );

    // Taker should now have 950 Token B (sent 50 to proposer)
    let taker_b_after: TokenAccount = get_spl_account(&svm, &taker_ata_b)
        .expect("Taker ATA B should exist");
    assert_eq!(taker_b_after.amount, INITIAL_MINT_AMOUNT - TOKEN_B_WANTED_AMOUNT);
    println!(
        "[Verify] Taker Token B: {} -> {} (sent {} to proposer)",
        taker_b_before.amount,
        taker_b_after.amount,
        taker_b_before.amount - taker_b_after.amount
    );

    // Proposer should have received 50 Token B (ATA created during TakeOffer)
    let proposer_b_after: TokenAccount = get_spl_account(&svm, &proposer_ata_b)
        .expect("Proposer ATA B should exist after TakeOffer");
    assert_eq!(proposer_b_after.amount, TOKEN_B_WANTED_AMOUNT);
    println!(
        "[Verify] Proposer Token B: 0 -> {} (received from taker, ATA was created during TakeOffer)",
        proposer_b_after.amount
    );

    // Vault should be closed after TakeOffer drained it
    let vault_account = svm.get_account(&vault_ata);
    match vault_account {
        Some(acc) => {
            if !acc.data.is_empty() {
                let vault_state: TokenAccount = get_spl_account(&svm, &vault_ata)
                    .expect("Should parse vault");
                assert_eq!(vault_state.amount, 0, "Vault should be empty");
                println!("[Verify] Vault: exists but empty (amount = 0)");
            } else {
                println!("[Verify] Vault: closed (empty data)");
            }
        }
        None => println!("[Verify] Vault: closed (account removed)"),
    }

    // Offer PDA should be closed after TakeOffer completed the swap
    let offer_account = svm.get_account(&offer_pda);
    assert!(
        offer_account.is_none() || offer_account.unwrap().data.is_empty(),
        "Offer PDA should be closed"
    );
    println!("[Verify] Offer PDA: closed");

    println!("\n=== PASSED: test_full_escrow_flow ===\n");
}
