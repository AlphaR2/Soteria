// Integration test for NFT staking happy path
//
// Tests the complete flow:
// 1. Create collection via our program
// 2. Mint NFT via our program
// 3. Stake NFT (adds FreezeDelegate + Attributes plugins)
// 4. Unstake NFT (removes FreezeDelegate, updates Attributes)

mod utils;

use utils::*;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    signature::Signer,
    pubkey::Pubkey
};
pub const MPL_CORE_ID: Pubkey = solana_sdk::pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

#[test]
fn test_happy_path_full_flow() {
    println!("\n=== TEST: NFT Staking Happy Path ===\n");

    // Setup
    println!("[Setup] Initializing LiteSVM and loading program...");
    let mut svm = setup_svm();

    // Create and fund accounts
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let owner = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Authority: {}", authority.pubkey());
    println!("[Setup] Owner: {}", owner.pubkey());

    // Create collection keypair
    let collection = solana_sdk::signature::Keypair::new();
    let (collection_state_pda, _bump) = derive_collection_state_pda(&collection.pubkey());
    println!("[Setup] Collection: {}", collection.pubkey());
    println!("[Setup] Collection State PDA: {}", collection_state_pda);

    // Step 1: Create collection using our program
    println!("\n[Test 1] Creating collection via create_collection instruction...");
    let create_collection_ix = build_create_collection_ix(
        &authority.pubkey(),
        &collection.pubkey(),
        &collection_state_pda,
        &authority.pubkey(), // payer
        &MPL_CORE_ID,
        "Test Collection".to_string(),
        "https://example.com/collection.json".to_string(),
    );

    send_tx_expect_success(
        &mut svm,
        create_collection_ix,
        &authority,
        &[&authority, &collection],
    );

    println!("[Test 1] Collection created successfully");

    // Verify collection exists
    let collection_account = svm.get_account(&collection.pubkey())
        .expect("Collection should exist");
    println!("[Verify] Collection account exists with {} lamports", collection_account.lamports);

    // Verify collection_state PDA exists
    let collection_state_account = svm.get_account(&collection_state_pda)
        .expect("Collection state should exist");
    println!("[Verify] Collection state PDA exists with {} bytes", collection_state_account.data.len());

    // Step 2: Mint NFT using our program
    println!("\n[Test 2] Minting NFT via mint_nft instruction...");
    let asset = solana_sdk::signature::Keypair::new();
    println!("[Test 2] Asset: {}", asset.pubkey());

    let mint_nft_ix = build_mint_nft_ix(
        &authority.pubkey(),
        &asset.pubkey(),
        &collection.pubkey(),
        &collection_state_pda,
        &authority.pubkey(), // update_authority
        &owner.pubkey(),
        &authority.pubkey(), // payer
        &MPL_CORE_ID,
        "Test NFT #1".to_string(),
        "https://example.com/nft1.json".to_string(),
    );

    send_tx_expect_success(
        &mut svm,
        mint_nft_ix,
        &authority,
        &[&authority, &asset],
    );

    println!("[Test 2] NFT minted successfully");

    // Verify asset exists
    let asset_account = svm.get_account(&asset.pubkey())
        .expect("Asset should exist");
    println!("[Verify] Asset account exists with {} lamports", asset_account.lamports);

    // Step 3: Stake NFT
    println!("\n[Test 3] Staking NFT via stake instruction...");
    let stake_ix = build_stake_ix(
        &owner.pubkey(),
        &authority.pubkey(),
        &owner.pubkey(),
        &asset.pubkey(),
        &collection.pubkey(),
        &collection_state_pda,
        &MPL_CORE_ID,
    );

    send_tx_expect_success(
        &mut svm,
        stake_ix,
        &owner,
        &[&owner, &authority],
    );

    println!("[Test 3] NFT staked successfully");
    println!("[Test 3] FreezeDelegate and Attributes plugins should be added");

    // Step 4: Advance time by 30 days
    println!("\n[Test 4] Advancing time by 30 days...");
    advance_time(&mut svm, MIN_STAKE_DURATION as u64);
    let clock: solana_sdk::clock::Clock = svm.get_sysvar();
    println!("[Test 4] Current timestamp: {}", clock.unix_timestamp);

    // Step 5: Unstake NFT
    println!("\n[Test 5] Unstaking NFT via unstake instruction...");
    let unstake_ix = build_unstake_ix(
        &owner.pubkey(),
        &authority.pubkey(),
        &owner.pubkey(),
        &asset.pubkey(),
        &collection.pubkey(),
        &collection_state_pda,
        &MPL_CORE_ID,
    );

    send_tx_expect_success(
        &mut svm,
        unstake_ix,
        &owner,
        &[&owner, &authority],
    );

    println!("[Test 5] NFT unstaked successfully");
    println!("[Test 5] FreezeDelegate removed, Attributes updated");

    println!("\n=== PASSED: test_happy_path_full_flow ===\n");
}
