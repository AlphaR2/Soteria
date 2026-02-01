use litesvm::LiteSVM;
use solana_sdk::{
    hash::hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::get_associated_token_address;

pub const GOVERNANCE_PROGRAM_ID: Pubkey = Pubkey::new_from_array(g_vulnerable::ID.to_bytes());

// Build Anchor instruction discriminator (first 8 bytes of sha256("global:method_name"))
pub fn anchor_discriminator(method: &str) -> [u8; 8] {
    let preimage = format!("global:{}", method);
    let hash = hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.to_bytes()[..8]);
    discriminator
}

pub const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = spl_associated_token_account::ID;
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;

// PDA Seeds
pub const CONFIG: &[u8] = b"config";
pub const TREASURY: &[u8] = b"treasury";
pub const TREASURYAUTH: &[u8] = b"treasury_auth";
pub const USERPROFILE: &[u8] = b"user_profile";
pub const USER_REGISTRY: &[u8] = b"user_registry";
pub const VOTE_COOLDOWN: &[u8] = b"cooldown";
pub const VOTE_RECORD: &[u8] = b"vote_record";

pub const DECIMALS: u8 = 6;

pub fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
   
    let program_bytes = include_bytes!("../target/deploy/g_vulnerable.so");
    svm.add_program(GOVERNANCE_PROGRAM_ID, program_bytes);
    svm
}

pub fn create_funded_account(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports)
        .expect("Airdrop should succeed");
    keypair
}

pub fn derive_config_pda(admin: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG, admin.as_ref()], &GOVERNANCE_PROGRAM_ID)
}

pub fn derive_treasury_pda(admin: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TREASURY, admin.as_ref()], &GOVERNANCE_PROGRAM_ID)
}

pub fn derive_treasury_authority_pda(config: &Pubkey, admin: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[TREASURYAUTH, config.as_ref(), admin.as_ref()],
        &GOVERNANCE_PROGRAM_ID,
    )
}

pub fn derive_user_profile_pda(user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[USERPROFILE, user.as_ref()], &GOVERNANCE_PROGRAM_ID)
}

pub fn derive_user_registry_pda(username: &str) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[USER_REGISTRY, username.as_bytes()], &GOVERNANCE_PROGRAM_ID)
}

pub fn derive_vote_cooldown_pda(voter: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VOTE_COOLDOWN, voter.as_ref()], &GOVERNANCE_PROGRAM_ID)
}

pub fn derive_vote_record_pda(voter: &Pubkey, target_username: &str) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[VOTE_RECORD, voter.as_ref(), target_username.as_bytes()],
        &GOVERNANCE_PROGRAM_ID,
    )
}

pub fn init_dao_instruction(
    signer: &Pubkey,
    admin: &Pubkey,
    config: &Pubkey,
    minimum_stake: u64,
    token_mint: &Pubkey,
    vote_power: u8,
) -> Instruction {
    let discriminator = anchor_discriminator("init_dao");
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&admin.to_bytes());
    data.extend_from_slice(&minimum_stake.to_le_bytes());
    data.extend_from_slice(&token_mint.to_bytes());
    data.extend_from_slice(&vote_power.to_le_bytes());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new(*config, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn initialize_treasury_instruction(
    signer: &Pubkey,
    admin: &Pubkey,
    config: &Pubkey,
    treasury: &Pubkey,
    treasury_authority: &Pubkey,
    treasury_token_account: &Pubkey,
    token_mint: &Pubkey,
) -> Instruction {
    let discriminator = anchor_discriminator("initialize_treasury");
    let data = discriminator.to_vec();

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new_readonly(*admin, false),
            AccountMeta::new_readonly(*config, false),
            AccountMeta::new(*treasury, false),
            AccountMeta::new_readonly(*treasury_authority, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(*treasury_token_account, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn create_profile_instruction(
    user: &Pubkey,
    user_registry: &Pubkey,
    user_profile: &Pubkey,
    username: &str,
) -> Instruction {
    let discriminator = anchor_discriminator("create_profile");
    let mut data = discriminator.to_vec();
    let username_len = username.len() as u32;
    data.extend_from_slice(&username_len.to_le_bytes());
    data.extend_from_slice(username.as_bytes());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*user_registry, false),
            AccountMeta::new(*user_profile, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn stake_tokens_instruction(
    user: &Pubkey,
    admin: &Pubkey,
    config: &Pubkey,
    treasury: &Pubkey,
    user_profile: &Pubkey,
    token_mint: &Pubkey,
    user_token_account: &Pubkey,
    treasury_token_account: &Pubkey,
    amount: u64,
) -> Instruction {
    let discriminator = anchor_discriminator("stake_tokens");
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new_readonly(*admin, false),
            AccountMeta::new_readonly(*config, false),
            AccountMeta::new(*treasury, false),
            AccountMeta::new(*user_profile, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(*user_token_account, false),
            AccountMeta::new(*treasury_token_account, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn upvote_instruction(
    voter: &Pubkey,
    admin: &Pubkey,
    config: &Pubkey,
    voter_profile: &Pubkey,
    target_user_registry: &Pubkey,
    target_user_profile: &Pubkey,
    vote_cooldown: &Pubkey,
    vote_record: &Pubkey,
    target_username: &str,
) -> Instruction {
    let discriminator = anchor_discriminator("upvote");
    let mut data = discriminator.to_vec();
    let username_len = target_username.len() as u32;
    data.extend_from_slice(&username_len.to_le_bytes());
    data.extend_from_slice(target_username.as_bytes());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*voter, true),
            AccountMeta::new_readonly(*admin, false),
            AccountMeta::new_readonly(*config, false),
            AccountMeta::new(*voter_profile, false),
            AccountMeta::new_readonly(*target_user_registry, false),
            AccountMeta::new(*target_user_profile, false),
            AccountMeta::new(*vote_cooldown, false),
            AccountMeta::new(*vote_record, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn downvote_instruction(
    voter: &Pubkey,
    admin: &Pubkey,
    config: &Pubkey,
    voter_profile: &Pubkey,
    target_user_registry: &Pubkey,
    target_user_profile: &Pubkey,
    vote_cooldown: &Pubkey,
    vote_record: &Pubkey,
    target_username: &str,
) -> Instruction {
    let discriminator = anchor_discriminator("downvote");
    let mut data = discriminator.to_vec();
    let username_len = target_username.len() as u32;
    data.extend_from_slice(&username_len.to_le_bytes());
    data.extend_from_slice(target_username.as_bytes());

    Instruction {
        program_id: GOVERNANCE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*voter, true),
            AccountMeta::new_readonly(*admin, false),
            AccountMeta::new_readonly(*config, false),
            AccountMeta::new(*voter_profile, false),
            AccountMeta::new_readonly(*target_user_registry, false),
            AccountMeta::new(*target_user_profile, false),
            AccountMeta::new(*vote_cooldown, false),
            AccountMeta::new(*vote_record, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
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
