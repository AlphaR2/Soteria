// Common test utilities for exploit test to be shared across multiple test files

use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;

// Program ID matching declare_id!("97G55caS2vz4RKqa34TMZN2s6ZmEG2FZeg9jkQBAnUtu")
pub const PROGRAM_ID: Pubkey = Pubkey::new_from_array(p_vulnerable::ID.to_bytes());

// Standard program IDs
pub const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = spl_associated_token_account::ID;

// Seed prefix must match MakeState::SEED_PREFIX
pub const OFFER_SEED_PREFIX: &[u8] = b"offer";

// Token configuration
pub const DECIMALS: u8 = 9;

// Test amounts
pub const INITIAL_MINT_AMOUNT: u64 = 1_000_000_000_000; // 1000 tokens
pub const TOKEN_A_OFFER_AMOUNT: u64 = 100_000_000_000;  // 100 tokens
pub const TOKEN_B_WANTED_AMOUNT: u64 = 50_000_000_000;  // 50 tokens

// Instruction discriminators
pub const PROPOSE_OFFER_DISCRIMINATOR: u8 = 0;
pub const TAKE_OFFER_DISCRIMINATOR: u8 = 1;


pub fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../target/deploy/vulnerable.so");
    svm.add_program(PROGRAM_ID, program_bytes);
    svm
}

pub fn create_funded_account(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports)
        .expect("Airdrop should succeed");
    keypair
}

pub fn derive_offer_pda(maker: &Pubkey, id: &[u8; 8]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[OFFER_SEED_PREFIX, maker.as_ref(), id],
        &PROGRAM_ID,
    )
}

// Build ProposeOffer instruction data
// Layout: discriminator(1) + id(8) + token_b_wanted(8) + token_a_offered(8) + bump(1) + padding(7)
pub fn build_propose_offer_data(
    id: [u8; 8],
    token_b_wanted_amount: u64,
    token_a_offered_amount: u64,
    bump: u8,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(33);
    data.push(PROPOSE_OFFER_DISCRIMINATOR);
    data.extend_from_slice(&id);
    data.extend_from_slice(&token_b_wanted_amount.to_le_bytes());
    data.extend_from_slice(&token_a_offered_amount.to_le_bytes());
    data.push(bump);
    data.extend_from_slice(&[0u8; 7]); // padding for repr(C) alignment
    data
}

pub fn build_take_offer_data() -> Vec<u8> {
    vec![TAKE_OFFER_DISCRIMINATOR]
}

// Escrow scenario setup result
pub struct EscrowScenario {
    pub svm: LiteSVM,
    pub payer: Keypair,
    pub proposer: Keypair,
    pub taker: Keypair,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub proposer_ata_a: Pubkey,
    pub taker_ata_a: Pubkey,
    pub taker_ata_b: Pubkey,
    pub offer_id: [u8; 8],
    pub offer_pda: Pubkey,
    pub bump: u8,
    pub vault_ata: Pubkey,
}

// Helper to set up a complete escrow scenario with funded accounts and mints
pub fn setup_escrow_scenario() -> EscrowScenario {
    // Initialize LiteSVM
    let mut svm = setup_svm();

    // fund accounts 
    let payer = create_funded_account(&mut svm, 10 * LAMPORTS_PER_SOL);
    let proposer = create_funded_account(&mut svm, 5 * LAMPORTS_PER_SOL);
    let taker = create_funded_account(&mut svm, 5 * LAMPORTS_PER_SOL);

    // Create mints
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

    // Create proposer's ATA A and fund it
    let proposer_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_a)
        .owner(&proposer.pubkey())
        .send()
        .expect("Failed to create proposer ATA A");

    MintTo::new(&mut svm, &payer, &mint_a, &proposer_ata_a, INITIAL_MINT_AMOUNT)
        .owner(&payer)
        .send()
        .expect("Failed to mint to proposer ATA A");

    // Create taker's ATAs
    let taker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_a)
        .owner(&taker.pubkey())
        .send()
        .expect("Failed to create taker ATA A");

    let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_b)
        .owner(&taker.pubkey())
        .send()
        .expect("Failed to create taker ATA B");

    MintTo::new(&mut svm, &payer, &mint_b, &taker_ata_b, INITIAL_MINT_AMOUNT)
        .owner(&payer)
        .send()
        .expect("Failed to mint to taker ATA B");

    // Derive offer PDA
    let offer_id: [u8; 8] = 1u64.to_le_bytes();
    let (offer_pda, bump) = derive_offer_pda(&proposer.pubkey(), &offer_id);
    let vault_ata = get_associated_token_address(&offer_pda, &mint_a);

    EscrowScenario {
        svm,
        payer,
        proposer,
        taker,
        mint_a,
        mint_b,
        proposer_ata_a,
        taker_ata_a,
        taker_ata_b,
        offer_id,
        offer_pda,
        bump,
        vault_ata,
    }
}

// Helper to create a valid offer (used as setup for TakeOffer exploits)
pub fn create_offer(
    svm: &mut LiteSVM,
    proposer: &Keypair,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    proposer_ata_a: &Pubkey,
    offer_pda: &Pubkey,
    vault_ata: &Pubkey,
    offer_id: [u8; 8],
    bump: u8,
) {
    let ix_data = build_propose_offer_data(
        offer_id,
        TOKEN_B_WANTED_AMOUNT,
        TOKEN_A_OFFER_AMOUNT,
        bump,
    );

    let propose_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(proposer.pubkey(), true),
            AccountMeta::new_readonly(*mint_a, false),
            AccountMeta::new_readonly(*mint_b, false),
            AccountMeta::new(*proposer_ata_a, false),
            AccountMeta::new(*offer_pda, false),
            AccountMeta::new(*vault_ata, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
        ],
        data: ix_data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[propose_ix],
        Some(&proposer.pubkey()),
        &[proposer],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).expect("ProposeOffer should succeed");
}
