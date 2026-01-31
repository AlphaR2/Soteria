// Integration tests for AMM program

mod utils;

use utils::*;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo, get_spl_account};
use solana_sdk::{
    clock::Clock,
    native_token::LAMPORTS_PER_SOL,
    signature::Signer,
    transaction::Transaction,
};


#[test]
fn test_initialize_pool() {
    println!("\n=== TEST: Initialize Pool ===\n");

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Create token mints
    println!("[Setup] Creating token mints...");
    let mint_a = CreateMint::new(&mut svm, &authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Failed to create mint A");

    let mint_b = CreateMint::new(&mut svm, &authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Failed to create mint B");

    println!("[Setup] Mint A: {}", mint_a);
    println!("[Setup] Mint B: {}", mint_b);

    // Initialize pool
    let init_ix = build_initialize_pool_ix(
        &authority.pubkey(),
        &mint_a,
        &mint_b,
        30, // 0.30% fee
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Pool initialization failed: {:?}", result.err());

    println!("[Success] Pool initialized with 30bp fee");
}

#[test]
fn test_deposit_liquidity_first_deposit() {
    println!("\n=== TEST: First Deposit ===\n");

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let depositor = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Create mints
    let mint_a = CreateMint::new(&mut svm, &authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Failed to create mint A");

    let mint_b = CreateMint::new(&mut svm, &authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Failed to create mint B");

    // Initialize pool
    let init_ix = build_initialize_pool_ix(&authority.pubkey(), &mint_a, &mint_b, 30);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    println!("[Setup] Pool initialized");

    // Create depositor token accounts
    let depositor_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &depositor, &mint_a)
        .owner(&depositor.pubkey())
        .send()
        .expect("Failed to create ATA A");

    let depositor_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &depositor, &mint_b)
        .owner(&depositor.pubkey())
        .send()
        .expect("Failed to create ATA B");

    // Mint tokens to depositor
    let amount_a = 1_000_000_000; // 1 token
    let amount_b = 2_000_000_000; // 2 tokens

    MintTo::new(&mut svm, &authority, &mint_a, &depositor_ata_a, amount_a)
        .owner(&authority)
        .send()
        .expect("Failed to mint token A");

    MintTo::new(&mut svm, &authority, &mint_b, &depositor_ata_b, amount_b)
        .owner(&authority)
        .send()
        .expect("Failed to mint token B");

    println!("[Setup] Depositor has {} A and {} B", amount_a, amount_b);

    // Deposit liquidity
    let clock = svm.get_sysvar::<Clock>();
    let expiration = clock.unix_timestamp + 60;

    let deposit_ix = build_deposit_liquidity_ix(
        &depositor.pubkey(),
        &mint_a,
        &mint_b,
        amount_a,
        amount_b,
        amount_a,
        amount_b,
        expiration,
    );

    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&depositor.pubkey()),
        &[&depositor],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "First deposit failed: {:?}", result.err());

    println!("[Success] First deposit completed");
}

#[test]
fn test_deposit_and_withdraw() {
    println!("\n=== TEST: Deposit and Withdraw ===\n");

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let depositor = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Setup mints and pool
    let mint_a = CreateMint::new(&mut svm, &authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();

    let mint_b = CreateMint::new(&mut svm, &authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();

    let init_ix = build_initialize_pool_ix(&authority.pubkey(), &mint_a, &mint_b, 30);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Setup depositor
    let depositor_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &depositor, &mint_a)
        .owner(&depositor.pubkey())
        .send()
        .unwrap();

    let depositor_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &depositor, &mint_b)
        .owner(&depositor.pubkey())
        .send()
        .unwrap();

    let amount_a = 10_000_000_000; // 10 tokens
    let amount_b = 10_000_000_000;

    MintTo::new(&mut svm, &authority, &mint_a, &depositor_ata_a, amount_a)
        .owner(&authority)
        .send()
        .unwrap();

    MintTo::new(&mut svm, &authority, &mint_b, &depositor_ata_b, amount_b)
        .owner(&authority)
        .send()
        .unwrap();

    // Deposit
    let clock = svm.get_sysvar::<Clock>();
    let expiration = clock.unix_timestamp + 60;

    let deposit_ix = build_deposit_liquidity_ix(
        &depositor.pubkey(),
        &mint_a,
        &mint_b,
        amount_a,
        amount_b,
        amount_a,
        amount_b,
        expiration,
    );

    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&depositor.pubkey()),
        &[&depositor],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    println!("[Step 1] Liquidity deposited");

    // Get LP balance
    let (pool_config, _) = derive_pool_config_pda(&mint_a, &mint_b);
    let (lp_mint, _) = derive_lp_mint_pda(&pool_config);
    let depositor_lp_ata = spl_associated_token_account::get_associated_token_address(
        &depositor.pubkey(),
        &lp_mint,
    );

    let lp_account: spl_token::state::Account = get_spl_account(&svm, &depositor_lp_ata)
    .expect("LP token account should exist after deposit");

    let lp_balance = lp_account.amount;

    // Withdraw half
    let lp_to_burn = lp_balance / 2;

    let withdraw_ix = build_withdraw_liquidity_ix(
        &depositor.pubkey(),
        &mint_a,
        &mint_b,
        lp_to_burn,
        1, // Accept any amount
        1,
        expiration,
    );

    let tx = Transaction::new_signed_with_payer(
        &[withdraw_ix],
        Some(&depositor.pubkey()),
        &[&depositor],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Withdraw failed: {:?}", result.err());

    println!("[Success] Withdrew {} LP tokens", lp_to_burn);
}

#[test]
fn test_swap_a_for_b() {
    println!("\n=== TEST: Swap A for B ===\n");

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let lp = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let swapper = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Setup mints and pool
    let mint_a = CreateMint::new(&mut svm, &authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();

    let mint_b = CreateMint::new(&mut svm, &authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();

    let init_ix = build_initialize_pool_ix(&authority.pubkey(), &mint_a, &mint_b, 30);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Add liquidity
    let lp_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &lp, &mint_a)
        .owner(&lp.pubkey())
        .send()
        .unwrap();

    let lp_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &lp, &mint_b)
        .owner(&lp.pubkey())
        .send()
        .unwrap();

    let lp_amount = 100_000_000_000; // 100 tokens each

    MintTo::new(&mut svm, &authority, &mint_a, &lp_ata_a, lp_amount)
        .owner(&authority)
        .send()
        .unwrap();

    MintTo::new(&mut svm, &authority, &mint_b, &lp_ata_b, lp_amount)
        .owner(&authority)
        .send()
        .unwrap();

    let clock = svm.get_sysvar::<Clock>();
    let expiration = clock.unix_timestamp + 60;

    let deposit_ix = build_deposit_liquidity_ix(
        &lp.pubkey(),
        &mint_a,
        &mint_b,
        lp_amount,
        lp_amount,
        lp_amount,
        lp_amount,
        expiration,
    );

    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&lp.pubkey()),
        &[&lp],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    println!("[Setup] Pool has liquidity");

    // Setup swapper
    let swapper_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &swapper, &mint_a)
        .owner(&swapper.pubkey())
        .send()
        .unwrap();

    let swap_amount = 1_000_000_000; // 1 token

    MintTo::new(&mut svm, &authority, &mint_a, &swapper_ata_a, swap_amount)
        .owner(&authority)
        .send()
        .unwrap();

    println!("[Setup] Swapper has {} token A", swap_amount);

    // Swap
    let swap_ix = build_swap_tokens_ix(
        &swapper.pubkey(),
        &mint_a,
        &mint_b,
        true, // A for B
        swap_amount,
        1, // Accept any output
        expiration,
    );

    let tx = Transaction::new_signed_with_payer(
        &[swap_ix],
        Some(&swapper.pubkey()),
        &[&swapper],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Swap failed: {:?}", result.err());

    println!("[Success] Swapped {} A for B", swap_amount);
}

#[test]
fn test_lock_unlock_pool() {
    println!("\n=== TEST: Lock and Unlock ===\n");

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Setup pool
    let mint_a = CreateMint::new(&mut svm, &authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();

    let mint_b = CreateMint::new(&mut svm, &authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();

    let init_ix = build_initialize_pool_ix(&authority.pubkey(), &mint_a, &mint_b, 30);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    println!("[Step 1] Pool initialized");

    // Lock
    let lock_ix = build_lock_pool_ix(&authority.pubkey(), &mint_a, &mint_b);
    let tx = Transaction::new_signed_with_payer(
        &[lock_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Lock failed: {:?}", result.err());

    println!("[Step 2] Pool locked");

    // Unlock
    let unlock_ix = build_unlock_pool_ix(&authority.pubkey(), &mint_a, &mint_b);
    let tx = Transaction::new_signed_with_payer(
        &[unlock_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Unlock failed: {:?}", result.err());

    println!("[Success] Pool unlocked");
}
