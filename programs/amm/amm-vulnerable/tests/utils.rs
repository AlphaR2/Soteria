// Test utilities for AMM program

use litesvm::LiteSVM;
use solana_sdk::{
    hash::hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::get_associated_token_address;

// Program ID matching declare_id! (amm_vulnerable)
pub const AMM_PROGRAM_ID: Pubkey = Pubkey::new_from_array(amm_vulnerable::ID.to_bytes());

// Build Anchor instruction discriminator
// Formula: first 8 bytes of sha256("global:method_name")
pub fn anchor_discriminator(method: &str) -> [u8; 8] {
    let preimage = format!("global:{}", method);
    let hash_result = hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash_result.to_bytes()[..8]);
    discriminator
}

// Standard program IDs
pub const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = spl_associated_token_account::ID;
use solana_system_interface::program::ID as system_program;

// PDA Seeds
pub const AMM_CONFIG_SEED: &[u8] = b"amm_config";
pub const AMM_AUTHORITY_SEED: &[u8] = b"amm_authority";
pub const LP_MINT_SEED: &[u8] = b"lp_mint";

// Token decimals
pub const DECIMALS: u8 = 9;

// Setup LiteSVM with AMM program
pub fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../target/deploy/amm_vulnerable.so");
    let _ = svm.add_program(AMM_PROGRAM_ID, program_bytes);
    svm
}

// Create and fund account
pub fn create_funded_account(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports)
        .expect("Airdrop should succeed");
    keypair
}

// Derive pool config PDA
pub fn derive_pool_config_pda(token_a_mint: &Pubkey, token_b_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            AMM_CONFIG_SEED,
            token_a_mint.as_ref(),
            token_b_mint.as_ref(),
        ],
        &AMM_PROGRAM_ID,
    )
}

// Derive pool authority PDA
pub fn derive_pool_authority_pda(pool_config: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[AMM_AUTHORITY_SEED, pool_config.as_ref()],
        &AMM_PROGRAM_ID,
    )
}

// Derive LP mint PDA
pub fn derive_lp_mint_pda(pool_config: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[LP_MINT_SEED, pool_config.as_ref()],
        &AMM_PROGRAM_ID,
    )
}

// Build initialize_pool instruction
pub fn build_initialize_pool_ix(
    authority: &Pubkey,
    token_a_mint: &Pubkey,
    token_b_mint: &Pubkey,
    fee_basis_points: u16,
) -> Instruction {
    let (pool_config, _) = derive_pool_config_pda(token_a_mint, token_b_mint);
    let (pool_authority, _) = derive_pool_authority_pda(&pool_config);
    let (lp_token_mint, _) = derive_lp_mint_pda(&pool_config);
    let token_a_vault = get_associated_token_address(&pool_authority, token_a_mint);
    let token_b_vault = get_associated_token_address(&pool_authority, token_b_mint);

    // Discriminator for initialize_pool
    let discriminator = anchor_discriminator("initialize_pool");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&fee_basis_points.to_le_bytes());

    Instruction {
        program_id: AMM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new_readonly(*token_a_mint, false),
            AccountMeta::new_readonly(*token_b_mint, false),
            AccountMeta::new(pool_config, false),
            AccountMeta::new_readonly(pool_authority, false),
            AccountMeta::new(lp_token_mint, false),
            AccountMeta::new(token_a_vault, false),
            AccountMeta::new(token_b_vault, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

// Build deposit_liquidity instruction
pub fn build_deposit_liquidity_ix(
    depositor: &Pubkey,
    token_a_mint: &Pubkey,
    token_b_mint: &Pubkey,
    desired_amount_a: u64,
    desired_amount_b: u64,
    max_amount_a: u64,
    max_amount_b: u64,
    expiration: i64,
) -> Instruction {
    let (pool_config, _) = derive_pool_config_pda(token_a_mint, token_b_mint);
    let (pool_authority, _) = derive_pool_authority_pda(&pool_config);
    let (lp_token_mint, _) = derive_lp_mint_pda(&pool_config);

    let depositor_token_a = get_associated_token_address(depositor, token_a_mint);
    let depositor_token_b = get_associated_token_address(depositor, token_b_mint);
    let depositor_lp_token = get_associated_token_address(depositor, &lp_token_mint);
    let token_a_vault = get_associated_token_address(&pool_authority, token_a_mint);
    let token_b_vault = get_associated_token_address(&pool_authority, token_b_mint);

    // Discriminator for deposit_liquidity
    let discriminator = anchor_discriminator("deposit_liquidity");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&desired_amount_a.to_le_bytes());
    data.extend_from_slice(&desired_amount_b.to_le_bytes());
    data.extend_from_slice(&max_amount_a.to_le_bytes());
    data.extend_from_slice(&max_amount_b.to_le_bytes());
    data.extend_from_slice(&expiration.to_le_bytes());

    Instruction {
        program_id: AMM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*depositor, true),
            AccountMeta::new_readonly(pool_config, false),
            AccountMeta::new_readonly(pool_authority, false),
            AccountMeta::new(lp_token_mint, false),
            AccountMeta::new_readonly(*token_a_mint, false),
            AccountMeta::new_readonly(*token_b_mint, false),
            AccountMeta::new(depositor_token_a, false),
            AccountMeta::new(depositor_token_b, false),
            AccountMeta::new(depositor_lp_token, false),
            AccountMeta::new(token_a_vault, false),
            AccountMeta::new(token_b_vault, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

// Build withdraw_liquidity instruction
pub fn build_withdraw_liquidity_ix(
    withdrawer: &Pubkey,
    token_a_mint: &Pubkey,
    token_b_mint: &Pubkey,
    lp_tokens_to_burn: u64,
    min_amount_a: u64,
    min_amount_b: u64,
    expiration: i64,
) -> Instruction {
    let (pool_config, _) = derive_pool_config_pda(token_a_mint, token_b_mint);
    let (pool_authority, _) = derive_pool_authority_pda(&pool_config);
    let (lp_token_mint, _) = derive_lp_mint_pda(&pool_config);

    let withdrawer_token_a = get_associated_token_address(withdrawer, token_a_mint);
    let withdrawer_token_b = get_associated_token_address(withdrawer, token_b_mint);
    let withdrawer_lp_token = get_associated_token_address(withdrawer, &lp_token_mint);
    let token_a_vault = get_associated_token_address(&pool_authority, token_a_mint);
    let token_b_vault = get_associated_token_address(&pool_authority, token_b_mint);

    // Discriminator for withdraw_liquidity
    let discriminator = anchor_discriminator("withdraw_liquidity");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&lp_tokens_to_burn.to_le_bytes());
    data.extend_from_slice(&min_amount_a.to_le_bytes());
    data.extend_from_slice(&min_amount_b.to_le_bytes());
    data.extend_from_slice(&expiration.to_le_bytes());

    Instruction {
        program_id: AMM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*withdrawer, true),
            AccountMeta::new_readonly(pool_config, false),
            AccountMeta::new_readonly(pool_authority, false),
            AccountMeta::new(lp_token_mint, false),
            AccountMeta::new_readonly(*token_a_mint, false),
            AccountMeta::new_readonly(*token_b_mint, false),
            AccountMeta::new(withdrawer_token_a, false),
            AccountMeta::new(withdrawer_token_b, false),
            AccountMeta::new(withdrawer_lp_token, false),
            AccountMeta::new(token_a_vault, false),
            AccountMeta::new(token_b_vault, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

// Build swap_tokens instruction
pub fn build_swap_tokens_ix(
    swapper: &Pubkey,
    token_a_mint: &Pubkey,
    token_b_mint: &Pubkey,
    swap_token_a_for_b: bool,
    input_amount: u64,
    min_output_amount: u64,
    expiration: i64,
) -> Instruction {
    let (pool_config, _) = derive_pool_config_pda(token_a_mint, token_b_mint);
    let (pool_authority, _) = derive_pool_authority_pda(&pool_config);

    let swapper_token_a = get_associated_token_address(swapper, token_a_mint);
    let swapper_token_b = get_associated_token_address(swapper, token_b_mint);
    let token_a_vault = get_associated_token_address(&pool_authority, token_a_mint);
    let token_b_vault = get_associated_token_address(&pool_authority, token_b_mint);

    // Discriminator for swap_tokens
    let discriminator = anchor_discriminator("swap_tokens");

    let mut data = discriminator.to_vec();
    data.push(if swap_token_a_for_b { 1 } else { 0 });
    data.extend_from_slice(&input_amount.to_le_bytes());
    data.extend_from_slice(&min_output_amount.to_le_bytes());
    data.extend_from_slice(&expiration.to_le_bytes());

    Instruction {
        program_id: AMM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*swapper, true),
            AccountMeta::new_readonly(pool_config, false),
            AccountMeta::new_readonly(pool_authority, false),
            AccountMeta::new_readonly(*token_a_mint, false),
            AccountMeta::new_readonly(*token_b_mint, false),
            AccountMeta::new(swapper_token_a, false),
            AccountMeta::new(swapper_token_b, false),
            AccountMeta::new(token_a_vault, false),
            AccountMeta::new(token_b_vault, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

// Build lock_pool instruction
pub fn build_lock_pool_ix(
    authority: &Pubkey,
    token_a_mint: &Pubkey,
    token_b_mint: &Pubkey,
) -> Instruction {
    let (pool_config, _) = derive_pool_config_pda(token_a_mint, token_b_mint);

    // Discriminator for lock_pool
    let discriminator = anchor_discriminator("lock_pool");

    Instruction {
        program_id: AMM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(pool_config, false),
        ],
        data: discriminator.to_vec(),
    }
}

// Build unlock_pool instruction
pub fn build_unlock_pool_ix(
    authority: &Pubkey,
    token_a_mint: &Pubkey,
    token_b_mint: &Pubkey,
) -> Instruction {
    let (pool_config, _) = derive_pool_config_pda(token_a_mint, token_b_mint);

    // Discriminator for unlock_pool
    let discriminator = anchor_discriminator("unlock_pool");

    Instruction {
        program_id: AMM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(pool_config, false),
        ],
        data: discriminator.to_vec(),
    }
}
