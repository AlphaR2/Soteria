// Integration tests for the secure governance program using LiteSVM
//
// Test Coverage:
//
// === Happy Path Tests ===
// 1. test_initialize_dao - Setup config and treasury
// 2. test_create_profile - Create user profile with unique username
// 3. test_stake_tokens - Stake tokens to gain voting rights
// 4. test_upvote_user - Vote to increase reputation
//
// === Security Tests ===
// 5. test_duplicate_username_rejected - Username uniqueness enforcement
// 6. test_minimum_stake_enforcement - Cannot vote without minimum stake

mod utils;

use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use utils::*;

#[test]
fn test_initialize_dao() {
    println!("[TEST START] test_initialize_dao");
    let mut svm = setup_svm();

    let admin = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let signer = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Admin and signer funded");

    let token_mint = CreateMint::new(&mut svm, &admin)
        .authority(&admin.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Mint creation should succeed");
    println!("[Setup] Token mint created");

    let ix = build_init_dao_ix(
        &signer.pubkey(),
        &admin.pubkey(),
        10_000_000,
        &token_mint,
        5,
    );
    println!("[Action] Building initialize DAO instruction");

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&signer.pubkey()),
        &[&signer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(
        result.is_ok(),
        "DAO initialization should succeed, got error: {:?}",
        result.err()
    );
    println!("[TEST END] test_initialize_dao - DAO initialized successfully");
}

#[test]
fn test_create_profile() {
    println!("[TEST START] test_create_profile");
    let mut svm = setup_svm();

    let admin = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Admin and user funded");

    let token_mint = CreateMint::new(&mut svm, &admin)
        .authority(&admin.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Mint creation should succeed");
    println!("[Setup] Token mint created");

    let ix = build_init_dao_ix(
        &admin.pubkey(),
        &admin.pubkey(),
        10_000_000,
        &token_mint,
        5,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("DAO init should succeed");
    println!("[Setup] DAO initialized");

    let username = "alice";
    let ix = build_create_profile_ix(&user.pubkey(), username);
    println!("[Action] Building create profile instruction for user: {}", username);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(
        result.is_ok(),
        "Profile creation should succeed, got error: {:?}",
        result.err()
    );
    println!("[TEST END] test_create_profile - Profile created successfully");
}

#[test]
fn test_stake_tokens() {
    println!("[TEST START] test_stake_tokens");
    let mut svm = setup_svm();

    let admin = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Admin and user funded");

    let token_mint = CreateMint::new(&mut svm, &admin)
        .authority(&admin.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Mint creation should succeed");
    println!("[Setup] Token mint created");

    let ix = build_init_dao_ix(
        &admin.pubkey(),
        &admin.pubkey(),
        10_000_000,
        &token_mint,
        5,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("DAO init should succeed");
    println!("[Setup] DAO initialized");

    let ix = build_initialize_treasury_ix(&admin.pubkey(), &admin.pubkey(), &token_mint);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("Treasury init should succeed");
    println!("[Setup] Treasury initialized");

    let username = "bob";
    let ix = build_create_profile_ix(&user.pubkey(), username);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("Profile creation should succeed");
    println!("[Setup] User profile created");

    let user_token_account = CreateAssociatedTokenAccount::new(&mut svm, &admin, &token_mint)
        .owner(&user.pubkey())
        .send()
        .expect("Failed to create user ATA");
    println!("[Setup] User token account created");

    MintTo::new(&mut svm, &admin, &token_mint, &user_token_account, 100_000_000)
        .owner(&admin)
        .send()
        .expect("Minting should succeed");
    println!("[Setup] Tokens minted to user");

    let stake_amount = 20_000_000;
    let ix = build_stake_tokens_ix(
        &user.pubkey(),
        &admin.pubkey(),
        &token_mint,
        stake_amount,
    );
    println!("[Action] Building stake tokens instruction - amount: {}", stake_amount);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(
        result.is_ok(),
        "Staking should succeed, got error: {:?}",
        result.err()
    );
    println!("[TEST END] test_stake_tokens - Staking completed successfully");
}

#[test]
fn test_upvote_user() {
    println!("[TEST START] test_upvote_user");
    let mut svm = setup_svm();

    let admin = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let voter = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let target = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Accounts funded: admin, voter, target");

    let token_mint = CreateMint::new(&mut svm, &admin)
        .authority(&admin.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Mint creation should succeed");
    println!("[Setup] Token mint created");

    let ix = build_init_dao_ix(
        &admin.pubkey(),
        &admin.pubkey(),
        10_000_000,
        &token_mint,
        5,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("DAO init should succeed");
    println!("[Setup] DAO initialized");

    let ix = build_initialize_treasury_ix(&admin.pubkey(), &admin.pubkey(), &token_mint);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("Treasury init should succeed");
    println!("[Setup] Treasury initialized");

    let voter_username = "voter1";
    let target_username = "target1";

    let ix = build_create_profile_ix(&voter.pubkey(), voter_username);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&voter.pubkey()),
        &[&voter],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("Voter profile creation should succeed");
    println!("[Setup] Voter profile created");

    let ix = build_create_profile_ix(&target.pubkey(), target_username);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&target.pubkey()),
        &[&target],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("Target profile creation should succeed");
    println!("[Setup] Target profile created");

    let voter_token_account = CreateAssociatedTokenAccount::new(&mut svm, &admin, &token_mint)
        .owner(&voter.pubkey())
        .send()
        .expect("Failed to create voter ATA");
    println!("[Setup] Voter token account created");

    MintTo::new(&mut svm, &admin, &token_mint, &voter_token_account, 100_000_000)
        .owner(&admin)
        .send()
        .expect("Minting should succeed");
    println!("[Setup] Tokens minted to voter");

    let ix = build_stake_tokens_ix(
        &voter.pubkey(),
        &admin.pubkey(),
        &token_mint,
        20_000_000,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&voter.pubkey()),
        &[&voter],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("Staking should succeed");
    println!("[Test] Voter staked 20 tokens");

    println!("[Test] Advancing time by 25 hours to bypass Member cooldown");
    advance_time(&mut svm, 25 * 3600);

    println!("[Test] Voter upvotes target");
    let ix = build_upvote_ix_with_target(
        &voter.pubkey(),
        &admin.pubkey(),
        &target.pubkey(),
        target_username,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&voter.pubkey()),
        &[&voter],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).expect("Upvote should succeed");
    println!("[Test] Upvote successful - target reputation increased");

    println!("[TEST END] test_upvote_user");
}

#[test]
fn test_duplicate_username_rejected() {
    println!("[TEST START] test_duplicate_username_rejected");
    let mut svm = setup_svm();

    let admin = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user1 = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user2 = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Accounts funded");

    let token_mint = CreateMint::new(&mut svm, &admin)
        .authority(&admin.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Mint creation should succeed");
    println!("[Setup] Token mint created");

    let ix = build_init_dao_ix(
        &admin.pubkey(),
        &admin.pubkey(),
        10_000_000,
        &token_mint,
        5,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("DAO init should succeed");
    println!("[Setup] DAO initialized");

    let username = "alice";
    let ix = build_create_profile_ix(&user1.pubkey(), username);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user1.pubkey()),
        &[&user1],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("First profile creation should succeed");
    println!("[Action] First user created profile with username: {}", username);

    let ix = build_create_profile_ix(&user2.pubkey(), username);
    println!("[Action] Second user attempting to create profile with duplicate username: {}", username);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user2.pubkey()),
        &[&user2],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "Duplicate username should be rejected, but it succeeded"
    );
    println!("[TEST END] test_duplicate_username_rejected - Duplicate rejected as expected");
}

#[test]
fn test_minimum_stake_enforcement() {
    println!("[TEST START] test_minimum_stake_enforcement");
    let mut svm = setup_svm();

    let admin = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let voter = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let target = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Accounts funded");

    let token_mint = CreateMint::new(&mut svm, &admin)
        .authority(&admin.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Mint creation should succeed");
    println!("[Setup] Token mint created");

    let ix = build_init_dao_ix(
        &admin.pubkey(),
        &admin.pubkey(),
        10_000_000,
        &token_mint,
        5,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("DAO init should succeed");
    println!("[Setup] DAO initialized");

    let ix = build_initialize_treasury_ix(&admin.pubkey(), &admin.pubkey(), &token_mint);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("Treasury init should succeed");
    println!("[Setup] Treasury initialized");

    let voter_username = "lowstaker";
    let target_username = "sometarget";

    let ix = build_create_profile_ix(&voter.pubkey(), voter_username);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&voter.pubkey()),
        &[&voter],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("Voter profile creation should succeed");
    println!("[Setup] Voter profile created");

    let ix = build_create_profile_ix(&target.pubkey(), target_username);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&target.pubkey()),
        &[&target],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .expect("Target profile creation should succeed");
    println!("[Setup] Target profile created");

    let voter_token_account = CreateAssociatedTokenAccount::new(&mut svm, &admin, &token_mint)
        .owner(&voter.pubkey())
        .send()
        .expect("Failed to create voter ATA");
    println!("[Setup] Voter token account created");

    MintTo::new(&mut svm, &admin, &token_mint, &voter_token_account, 100_000_000)
        .owner(&admin)
        .send()
        .expect("Minting should succeed");
    println!("[Setup] Tokens minted to voter");

    let ix = build_stake_tokens_ix(
        &voter.pubkey(),
        &admin.pubkey(),
        &token_mint,
        5_000_000,
    );
    println!("[Action] Attempting to stake below minimum (5 tokens)");
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&voter.pubkey()),
        &[&voter],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Staking below minimum should fail in secure version");
    println!("[Verification] Staking below minimum correctly rejected");

    let ix = build_stake_tokens_ix(
        &voter.pubkey(),
        &admin.pubkey(),
        &token_mint,
        10_000_000,
    );
    println!("[Action] Staking exact minimum amount (10 tokens)");
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&voter.pubkey()),
        &[&voter],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("Staking minimum amount should succeed");
    println!("[Test] Staking with minimum amount (10 tokens) succeeded");

    println!("[Test] Advancing time by 25 hours to bypass Member cooldown");
    advance_time(&mut svm, 25 * 3600);

    println!("[Test] Attempting to vote with minimum stake");
    let ix = build_upvote_ix_with_target(
        &voter.pubkey(),
        &admin.pubkey(),
        &target.pubkey(),
        target_username,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&voter.pubkey()),
        &[&voter],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).expect("Vote should succeed with minimum stake");
    println!("[Test] Vote succeeded with minimum stake - secure version enforces minimum");

    println!("[TEST END] test_minimum_stake_enforcement");
}