// Test utilities for governance program

use litesvm::LiteSVM;
use solana_sdk::{
    hash::hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::get_associated_token_address;

// Program ID matching declare_id!
pub const GOVERNANCE_PROGRAM_ID: Pubkey = Pubkey::new_from_array(governance_secure::ID.to_bytes());

// Standard program IDs
pub const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = spl_associated_token_account::ID;
use solana_system_interface::program::ID as system_program;

// PDA Seeds
pub const CONFIG: &[u8] = b"config";
pub const TREASURY: &[u8] = b"treasury";
pub const TREASURYAUTH: &[u8] = b"treasury_auth";
pub const USERPROFILE: &[u8] = b"user_profile";
pub const USER_REGISTRY: &[u8] = b"user_registry";
pub const VOTE_COOLDOWN: &[u8] = b"cooldown";
pub const VOTE_RECORD: &[u8] = b"vote_record";

// Token decimals
pub const DECIMALS: u8 = 6;

// ======================== HELPERS ========================

/// Build Anchor instruction discriminator (first 8 bytes of sha256("global:method_name"))
pub fn anchor_discriminator(method: &str) -> [u8; 8] {
    let preimage = format!("global:{}", method);
    let hash = hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.to_bytes()[..8]);
    discriminator
}

// Setup LiteSVM with governance program
pub fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../target/deploy/governance_secure.so");
    svm.add_program(GOVERNANCE_PROGRAM_ID, program_bytes);
    svm
}

// Create and fund account
pub fn create_funded_account(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports)
        .expect("Airdrop should succeed");
    keypair
}

// Derive config PDA
pub fn derive_config_pda(admin: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG, admin.as_ref()], &GOVERNANCE_PROGRAM_ID)
}

// Derive treasury state PDA
pub fn derive_treasury_pda(admin: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TREASURY, admin.as_ref()], &GOVERNANCE_PROGRAM_ID)
}

// Derive treasury authority PDA
pub fn derive_treasury_authority_pda(config: &Pubkey, admin: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[TREASURYAUTH, config.as_ref(), admin.as_ref()],
        &GOVERNANCE_PROGRAM_ID,
    )
}

// Derive user profile PDA
pub fn derive_user_profile_pda(user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[USERPROFILE, user.as_ref()], &GOVERNANCE_PROGRAM_ID)
}

// Derive username registry PDA
pub fn derive_username_registry_pda(username: &str) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[USER_REGISTRY, username.as_bytes()],
        &GOVERNANCE_PROGRAM_ID,
    )
}

// Derive vote cooldown PDA
pub fn derive_vote_cooldown_pda(voter: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VOTE_COOLDOWN, voter.as_ref()], &GOVERNANCE_PROGRAM_ID)
}

// Derive vote record PDA
pub fn derive_vote_record_pda(voter: &Pubkey, target_username: &str) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[VOTE_RECORD, voter.as_ref(), target_username.as_bytes()],
        &GOVERNANCE_PROGRAM_ID,
    )
}

// Build init_dao instruction
pub fn build_init_dao_ix(
    signer: &Pubkey,
    admin: &Pubkey,
    minimum_stake: u64,
    token_mint: &Pubkey,
    vote_power: u8,
) -> Instruction {
    let (config, _) = derive_config_pda(admin);

    let discriminator = anchor_discriminator("init_dao");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(admin.as_ref());
    data.extend_from_slice(&minimum_stake.to_le_bytes());
    data.extend_from_slice(token_mint.as_ref());
    data.push(vote_power);

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

// Build initialize_treasury instruction
pub fn build_initialize_treasury_ix(
    signer: &Pubkey,
    admin: &Pubkey,
    token_mint: &Pubkey,
) -> Instruction {
    let (config, _) = derive_config_pda(admin);
    let (treasury, _) = derive_treasury_pda(admin);
    let (treasury_authority, _) = derive_treasury_authority_pda(&config, admin);
    let treasury_token_account = get_associated_token_address(&treasury_authority, token_mint);

    let discriminator = anchor_discriminator("initialize_treasury");

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new_readonly(*admin, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(treasury, false),
            AccountMeta::new_readonly(treasury_authority, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(treasury_token_account, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data: discriminator.to_vec(),
    }
}

// Build create_profile instruction
pub fn build_create_profile_ix(user: &Pubkey, username: &str) -> Instruction {
    let (user_registry, _) = derive_username_registry_pda(username);
    let (user_profile, _) = derive_user_profile_pda(user);

    let discriminator = anchor_discriminator("create_profile");

    let mut data = discriminator.to_vec();
    // Borsh serialization: len (4 bytes) + string bytes
    data.extend_from_slice(&(username.len() as u32).to_le_bytes());
    data.extend_from_slice(username.as_bytes());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(user_registry, false),
            AccountMeta::new(user_profile, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

// Build stake_tokens instruction
pub fn build_stake_tokens_ix(
    user: &Pubkey,
    admin: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let (config, _) = derive_config_pda(admin);
    let (treasury, _) = derive_treasury_pda(admin);
    let (user_profile, _) = derive_user_profile_pda(user);
    let (treasury_authority, _) = derive_treasury_authority_pda(&config, admin);

    let user_token_account = get_associated_token_address(user, token_mint);
    let treasury_token_account = get_associated_token_address(&treasury_authority, token_mint);

    let discriminator = anchor_discriminator("stake_tokens");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(*admin, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(treasury, false),
            AccountMeta::new(user_profile, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new(treasury_token_account, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

// Build unstake_tokens instruction
pub fn build_unstake_tokens_ix(
    user: &Pubkey,
    admin: &Pubkey,
    token_mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let (config, _) = derive_config_pda(admin);
    let (treasury, _) = derive_treasury_pda(admin);
    let (treasury_authority, _) = derive_treasury_authority_pda(&config, admin);
    let (user_profile, _) = derive_user_profile_pda(user);

    let user_token_account = get_associated_token_address(user, token_mint);
    let treasury_token_account = get_associated_token_address(&treasury_authority, token_mint);

    let discriminator = anchor_discriminator("unstake_tokens");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(*admin, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(treasury, false),
            AccountMeta::new_readonly(treasury_authority, false),
            AccountMeta::new(user_profile, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new(treasury_token_account, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

// Build upvote instruction (without target profile – legacy / partial version)
pub fn build_upvote_ix(
    voter: &Pubkey,
    admin: &Pubkey,
    target_username: &str,
) -> Instruction {
    let (config, _) = derive_config_pda(admin);
    let (voter_profile, _) = derive_user_profile_pda(voter);
    let (target_user_registry, _) = derive_username_registry_pda(target_username);
    let (vote_cooldown, _) = derive_vote_cooldown_pda(voter);
    let (vote_record, _) = derive_vote_record_pda(voter, target_username);

    let discriminator = anchor_discriminator("upvote");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&(target_username.len() as u32).to_le_bytes());
    data.extend_from_slice(target_username.as_bytes());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*voter, true),
            AccountMeta::new_readonly(*admin, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(voter_profile, false),
            AccountMeta::new_readonly(target_user_registry, false),
            // Note: target_user_profile is missing here – this version might be incomplete
            AccountMeta::new(vote_cooldown, false),
            AccountMeta::new(vote_record, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

// Build upvote instruction with target profile (recommended version)
pub fn build_upvote_ix_with_target(
    voter: &Pubkey,
    admin: &Pubkey,
    target_user: &Pubkey,
    target_username: &str,
) -> Instruction {
    let (config, _) = derive_config_pda(admin);
    let (voter_profile, _) = derive_user_profile_pda(voter);
    let (target_user_registry, _) = derive_username_registry_pda(target_username);
    let (target_user_profile, _) = derive_user_profile_pda(target_user);
    let (vote_cooldown, _) = derive_vote_cooldown_pda(voter);
    let (vote_record, _) = derive_vote_record_pda(voter, target_username);

    let discriminator = anchor_discriminator("upvote");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&(target_username.len() as u32).to_le_bytes());
    data.extend_from_slice(target_username.as_bytes());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*voter, true),
            AccountMeta::new_readonly(*admin, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(voter_profile, false),
            AccountMeta::new_readonly(target_user_registry, false),
            AccountMeta::new(target_user_profile, false),
            AccountMeta::new(vote_cooldown, false),
            AccountMeta::new(vote_record, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

// Build downvote instruction
pub fn build_downvote_ix_with_target(
    voter: &Pubkey,
    admin: &Pubkey,
    target_user: &Pubkey,
    target_username: &str,
) -> Instruction {
    let (config, _) = derive_config_pda(admin);
    let (voter_profile, _) = derive_user_profile_pda(voter);
    let (target_user_registry, _) = derive_username_registry_pda(target_username);
    let (target_user_profile, _) = derive_user_profile_pda(target_user);
    let (vote_cooldown, _) = derive_vote_cooldown_pda(voter);
    let (vote_record, _) = derive_vote_record_pda(voter, target_username);

    let discriminator = anchor_discriminator("downvote");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&(target_username.len() as u32).to_le_bytes());
    data.extend_from_slice(target_username.as_bytes());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*voter, true),
            AccountMeta::new_readonly(*admin, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(voter_profile, false),
            AccountMeta::new_readonly(target_user_registry, false),
            AccountMeta::new(target_user_profile, false),
            AccountMeta::new(vote_cooldown, false),
            AccountMeta::new(vote_record, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

// Build reset_user_reputation instruction
pub fn build_reset_user_reputation_ix(
    admin: &Pubkey,
    user: &Pubkey,
) -> Instruction {
    let (config, _) = derive_config_pda(admin);
    let (user_profile, _) = derive_user_profile_pda(user);

    let discriminator = anchor_discriminator("reset_user_reputation");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(user.as_ref());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(user_profile, false),
        ],
        data,
    }
}

// Advance the SVM clock by the specified number of seconds
pub fn advance_time(svm: &mut LiteSVM, seconds: u64) {
    let mut clock: solana_sdk::clock::Clock = svm.get_sysvar();
    clock.unix_timestamp += seconds as i64;
    svm.set_sysvar(&clock);

    let current_slot = clock.slot;
    svm.warp_to_slot(current_slot + (seconds * 2) + 5);
}