// Integration tests for AMM program using LiteSVM
// These tests verify core AMM functionality: pool init, first deposit, deposit/withdraw, swap, lock/unlock

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
    // Test: Initialize a new AMM pool with given tokens and fee
    println!("\n[TEST START] test_initialize_pool - Initializing new AMM pool");

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Authority account funded with SOL");

    // Create token mints for the pool
    println!("[Setup] Creating token mints for the pool...");
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

    println!("[Setup] Mint A created: {}", mint_a);
    println!("[Setup] Mint B created: {}", mint_b);

    // Build and send initialize pool instruction
    println!("[Action] Building initialize pool instruction (30 bp fee)");
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

    println!("[Action] Sending pool initialization transaction...");
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Pool initialization failed: {:?}", result.err());

    println!("[Success] Pool successfully initialized with 30bp fee");
    println!("[TEST END] test_initialize_pool");
}

#[test]
fn test_deposit_liquidity_first_deposit() {
    // Test: Perform the very first liquidity deposit into a new pool
    println!("\n[TEST START] test_deposit_liquidity_first_deposit - First liquidity deposit");

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let depositor = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Authority and depositor accounts funded");

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
    println!("[Setup] Token mints created");

    // Initialize pool
    println!("[Action] Initializing new pool");
    let init_ix = build_initialize_pool_ix(&authority.pubkey(), &mint_a, &mint_b, 30);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[Success] Pool initialized");

    // Create depositor's token accounts
    let depositor_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &depositor, &mint_a)
        .owner(&depositor.pubkey())
        .send()
        .expect("Failed to create ATA A");

    let depositor_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &depositor, &mint_b)
        .owner(&depositor.pubkey())
        .send()
        .expect("Failed to create ATA B");
    println!("[Setup] Depositor associated token accounts created");

    // Mint initial tokens to depositor
    let amount_a = 1_000_000_000; // 1 token (adjusted for decimals)
    let amount_b = 2_000_000_000; // 2 tokens

    MintTo::new(&mut svm, &authority, &mint_a, &depositor_ata_a, amount_a)
        .owner(&authority)
        .send()
        .expect("Failed to mint token A");

    MintTo::new(&mut svm, &authority, &mint_b, &depositor_ata_b, amount_b)
        .owner(&authority)
        .send()
        .expect("Failed to mint token B");
    println!("[Setup] Depositor minted {} token A and {} token B", amount_a, amount_b);

    // Prepare expiration time
    let clock = svm.get_sysvar::<Clock>();
    let expiration = clock.unix_timestamp + 60;

    println!("[Action] Building first deposit liquidity instruction");
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

    println!("[Action] Sending first deposit transaction...");
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "First deposit failed: {:?}", result.err());

    println!("[Success] First liquidity deposit completed successfully");
    println!("[TEST END] test_deposit_liquidity_first_deposit");
}

#[test]
fn test_deposit_and_withdraw() {
    // Test: Deposit liquidity then withdraw part of it
    println!("\n[TEST START] test_deposit_and_withdraw - Deposit followed by withdraw");

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let depositor = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Authority and depositor funded");

    // Create mints
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
    println!("[Setup] Token mints created");

    // Initialize pool
    println!("[Action] Initializing pool with 30bp fee");
    let init_ix = build_initialize_pool_ix(&authority.pubkey(), &mint_a, &mint_b, 30);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[Success] Pool initialized");

    // Setup depositor token accounts and mint tokens
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
    println!("[Setup] Depositor funded with {} token A and {} token B", amount_a, amount_b);

    // Deposit liquidity
    let clock = svm.get_sysvar::<Clock>();
    let expiration = clock.unix_timestamp + 60;

    println!("[Action] Depositing liquidity into pool");
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
    println!("[Step 1] Liquidity successfully deposited");

    // Check LP balance after deposit
    let (pool_config, _) = derive_pool_config_pda(&mint_a, &mint_b);
    let (lp_mint, _) = derive_lp_mint_pda(&pool_config);
    let depositor_lp_ata = spl_associated_token_account::get_associated_token_address(
        &depositor.pubkey(),
        &lp_mint,
    );

    let lp_account: spl_token::state::Account = get_spl_account(&svm, &depositor_lp_ata)
        .expect("LP token account should exist after deposit");

    let lp_balance = lp_account.amount;
    println!("[Info] Depositor LP balance after deposit: {}", lp_balance);

    // Withdraw half of the LP tokens
    let lp_to_burn = lp_balance / 2;
    println!("[Action] Preparing to withdraw {} LP tokens (half of balance)", lp_to_burn);

    let withdraw_ix = build_withdraw_liquidity_ix(
        &depositor.pubkey(),
        &mint_a,
        &mint_b,
        lp_to_burn,
        1, // min amount A (accept any)
        1, // min amount B (accept any)
        expiration,
    );

    let tx = Transaction::new_signed_with_payer(
        &[withdraw_ix],
        Some(&depositor.pubkey()),
        &[&depositor],
        svm.latest_blockhash(),
    );

    println!("[Action] Sending withdraw transaction...");
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Withdraw failed: {:?}", result.err());

    println!("[Success] Successfully withdrew {} LP tokens", lp_to_burn);
    println!("[TEST END] test_deposit_and_withdraw");
}

#[test]
fn test_swap_a_for_b() {
    // Test: Perform a token swap (A → B) after adding liquidity
    println!("\n[TEST START] test_swap_a_for_b - Token swap A for B");

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let lp = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let swapper = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Authority, LP provider, and swapper funded");

    // Create mints
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

    // Initialize pool
    println!("[Action] Initializing AMM pool");
    let init_ix = build_initialize_pool_ix(&authority.pubkey(), &mint_a, &mint_b, 30);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[Success] Pool initialized");

    // Add initial liquidity
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

    println!("[Action] Adding initial liquidity to pool");
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
    println!("[Success] Pool now has liquidity");

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
    println!("[Setup] Swapper funded with {} token A", swap_amount);

    // Perform swap A → B
    println!("[Action] Building swap instruction (A → B)");
    let swap_ix = build_swap_tokens_ix(
        &swapper.pubkey(),
        &mint_a,
        &mint_b,
        true, // A for B
        swap_amount,
        1, // min output amount (accept any)
        expiration,
    );

    let tx = Transaction::new_signed_with_payer(
        &[swap_ix],
        Some(&swapper.pubkey()),
        &[&swapper],
        svm.latest_blockhash(),
    );

    println!("[Action] Sending swap transaction...");
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Swap failed: {:?}", result.err());

    println!("[Success] Swapped {} token A for token B", swap_amount);
    println!("[TEST END] test_swap_a_for_b");
}

#[test]
fn test_lock_unlock_pool() {
    // Test: Lock the pool (disable operations) then unlock it
    println!("\n[TEST START] test_lock_unlock_pool - Pool lock and unlock flow");

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Authority funded");

    // Create mints
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

    // Initialize pool
    println!("[Action] Initializing pool");
    let init_ix = build_initialize_pool_ix(&authority.pubkey(), &mint_a, &mint_b, 30);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[Step 1] Pool initialized");

    // Lock pool
    println!("[Action] Locking pool (operations should be blocked after this)");
    let lock_ix = build_lock_pool_ix(&authority.pubkey(), &mint_a, &mint_b);
    let tx = Transaction::new_signed_with_payer(
        &[lock_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Lock failed: {:?}", result.err());
    println!("[Step 2] Pool successfully locked");

    // Unlock pool
    println!("[Action] Unlocking pool (operations should resume)");
    let unlock_ix = build_unlock_pool_ix(&authority.pubkey(), &mint_a, &mint_b);
    let tx = Transaction::new_signed_with_payer(
        &[unlock_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Unlock failed: {:?}", result.err());

    println!("[Success] Pool successfully unlocked");
    println!("[TEST END] test_lock_unlock_pool");
}