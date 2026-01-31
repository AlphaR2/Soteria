// Common test utilities for NFT staking tests
// TODO: we might have a stack overflow issue. working on it 

use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::program::ID as system_program;
use borsh::BorshSerialize;

// Program ID matching declare_id!
pub const PROGRAM_ID: Pubkey = solana_sdk::pubkey!("xbwEtBJ9eoyGCAkvr4P2JmMH8wSnrb6amh2po57oGGJ");

pub const MPL_CORE_ID: Pubkey = solana_sdk::pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

// Seed constants (must match constants.rs)
pub const COLLECTION_STATE: &[u8] = b"collection_state";
pub const STAKED_KEY: &str = "staked";
pub const STAKED_TIME_KEY: &str = "staked_time";
pub const MIN_STAKE_DURATION: i64 = 30 * 24 * 60 * 60; // 30 days in seconds

// ======================== HELPERS ========================

/// Build Anchor instruction discriminator (8 bytes from sighash of "global:method_name")
pub fn anchor_discriminator(method: &str) -> [u8; 8] {
    let preimage = format!("global:{}", method);
    let hash = solana_sdk::hash::hash(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.to_bytes()[..8]);
    discriminator
}

/// Load the compiled program binary into LiteSVM
pub fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../target/deploy/nft_staking_secure.so");
    svm.add_program(PROGRAM_ID, program_bytes).expect("Failed to add staking program");

    // IMPORTANT: Load mpl-core program binary for LiteSVM to execute its instructions.  
    // i did a build of it using solana program dump -u d CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d mpl_core.so to get a .so file and keep. 
 

    let mpl_core_bytes = include_bytes!("../../utils/mpl-core-sample-so/mpl_core.so");

    svm.add_program(MPL_CORE_ID, mpl_core_bytes).expect("Failed to add mpl-core program");

    svm
}

/// Create a new keypair and fund it with SOL via airdrop
pub fn create_funded_account(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports)
        .expect("Airdrop should succeed");
    keypair
}

/// Derive the collection_state PDA using seeds: ["collection_state", collection]
pub fn derive_collection_state_pda(collection: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[COLLECTION_STATE, collection.as_ref()],
        &PROGRAM_ID,
    )
}

/// Advance the SVM clock by the specified number of seconds
pub fn advance_time(svm: &mut LiteSVM, seconds: u64) {
    let mut clock: solana_sdk::clock::Clock = svm.get_sysvar();
    clock.unix_timestamp += seconds as i64;
    svm.set_sysvar(&clock);
}

// ======================== MPL-CORE HELPERS ========================

/// Create a Metaplex Core collection (manual instruction)
pub fn create_mpl_collection(
    svm: &mut LiteSVM,
    payer: &Keypair,
    collection: &Keypair,
    name: String,
    uri: String,
) {
    let discriminator = anchor_discriminator("create_collection_v2");

    let mut data = discriminator.to_vec();

    // Manually serialize name (String = u32 length + bytes)
    name.serialize(&mut data).expect("Failed to serialize name");

    // Manually serialize uri (String = u32 length + bytes)
    uri.serialize(&mut data).expect("Failed to serialize uri");

    // Serialize empty vectors for plugins (Vec = u32 length)
    // No plugins needed for basic collection creation
    let empty_vec_len: u32 = 0;
    data.extend_from_slice(&empty_vec_len.to_le_bytes()); // plugins
    data.extend_from_slice(&empty_vec_len.to_le_bytes()); // authorities
    data.extend_from_slice(&empty_vec_len.to_le_bytes()); // ext_adapters
    data.extend_from_slice(&empty_vec_len.to_le_bytes()); // ext_adapter_auths

    let accounts = vec![
        AccountMeta::new(collection.pubkey(), true),               // collection
        AccountMeta::new_readonly(payer.pubkey(), false),          // update_authority = payer
        AccountMeta::new(payer.pubkey(), true),                    // payer
        AccountMeta::new_readonly(system_program, false),    // system_program
    ];

    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts,
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer, collection],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).expect("Failed to create MPL Core collection");
}

/// Mint a Metaplex Core NFT asset into a collection (manual instruction)
pub fn mint_mpl_asset(
    svm: &mut LiteSVM,
    payer: &Keypair,
    collection: &Pubkey,
    asset: &Keypair,
    owner: &Pubkey,
    name: String,
    uri: String,
) {
    let discriminator = anchor_discriminator("create_v2");

    let mut data = discriminator.to_vec();

    // Manually serialize name (String = u32 length + bytes)
    name.serialize(&mut data).expect("Failed to serialize name");

    // Manually serialize uri (String = u32 length + bytes)
    uri.serialize(&mut data).expect("Failed to serialize uri");

    // Serialize empty vectors for plugins (Vec = u32 length)
    // No plugins needed for basic asset creation
    let empty_vec_len: u32 = 0;
    data.extend_from_slice(&empty_vec_len.to_le_bytes()); // plugins
    data.extend_from_slice(&empty_vec_len.to_le_bytes()); // authorities
    data.extend_from_slice(&empty_vec_len.to_le_bytes()); // ext_adapters
    data.extend_from_slice(&empty_vec_len.to_le_bytes()); // ext_adapter_auths

    let accounts = vec![
        AccountMeta::new(asset.pubkey(), true),                    // asset
        AccountMeta::new(*collection, false),                      // collection (writable for size update)
        AccountMeta::new_readonly(payer.pubkey(), false),          // authority = payer
        AccountMeta::new(payer.pubkey(), true),                    // payer
        AccountMeta::new_readonly(*owner, false),                  // owner
        AccountMeta::new_readonly(payer.pubkey(), false),          // update_authority = payer
        AccountMeta::new_readonly(system_program, false),    // system_program
    ];

    let ix = Instruction {
        program_id: MPL_CORE_ID,
        accounts,
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer, asset],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).expect("Failed to mint MPL Core asset");
}

// ======================== INSTRUCTION BUILDERS ========================

/// Build create_collection instruction
pub fn build_create_collection_ix(
    authority: &Pubkey,
    collection: &Pubkey,
    collection_state: &Pubkey,
    payer: &Pubkey,
    mpl_core_program: &Pubkey,
    name: String,
    uri: String,
) -> Instruction {
    let discriminator = anchor_discriminator("create_collection");

    let mut data = discriminator.to_vec();

    // Serialize name
    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    // Serialize uri
    data.extend_from_slice(&(uri.len() as u32).to_le_bytes());
    data.extend_from_slice(uri.as_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(*collection, true),
            AccountMeta::new(*collection_state, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*mpl_core_program, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

/// Build mint_nft instruction
pub fn build_mint_nft_ix(
    authority: &Pubkey,
    asset: &Pubkey,
    collection: &Pubkey,
    collection_state: &Pubkey,
    update_authority: &Pubkey,
    owner: &Pubkey,
    payer: &Pubkey,
    mpl_core_program: &Pubkey,
    name: String,
    uri: String,
) -> Instruction {
    let discriminator = anchor_discriminator("mint_nft");

    let mut data = discriminator.to_vec();

    // Serialize name
    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());

    // Serialize uri
    data.extend_from_slice(&(uri.len() as u32).to_le_bytes());
    data.extend_from_slice(uri.as_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*authority, true),           // authority (signer)
            AccountMeta::new(*asset, true),               // asset (signer)
            AccountMeta::new(*collection, false),         // collection
            AccountMeta::new(*collection_state, false),   // collection_state
            AccountMeta::new_readonly(*update_authority, false), // update_authority
            AccountMeta::new_readonly(*owner, false),     // owner
            AccountMeta::new(*payer, true),               // payer (signer)
            AccountMeta::new_readonly(*mpl_core_program, false), // mpl_core_program
            AccountMeta::new_readonly(system_program, false),    // system_program
        ],
        data,
    }
}

/// Build stake instruction (no args, just discriminator)
pub fn build_stake_ix(
    owner: &Pubkey,
    update_authority: &Pubkey,
    payer: &Pubkey,
    asset: &Pubkey,
    collection: &Pubkey,
    collection_state: &Pubkey,
    mpl_core_program: &Pubkey,

) -> Instruction {
    let discriminator = anchor_discriminator("stake");

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new_readonly(*update_authority, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new(*asset, false),
            AccountMeta::new(*collection, false),
            AccountMeta::new(*collection_state, false),
            AccountMeta::new_readonly(*mpl_core_program, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data: discriminator.to_vec(),
    }
}

/// Build unstake instruction (no args, just discriminator)
pub fn build_unstake_ix(
    owner: &Pubkey,
    update_authority: &Pubkey,
    payer: &Pubkey,
    asset: &Pubkey,
    collection: &Pubkey,
    collection_state: &Pubkey,
    mpl_core_program: &Pubkey,
) -> Instruction {
    let discriminator = anchor_discriminator("unstake");

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new_readonly(*update_authority, true),
            AccountMeta::new(*payer, true),
            AccountMeta::new(*asset, false),
            AccountMeta::new(*collection, false),
            AccountMeta::new(*collection_state, false),
            AccountMeta::new_readonly(*mpl_core_program, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data: discriminator.to_vec(),
    }
}

// ======================== TRANSACTION HELPERS ========================

/// Send a transaction and expect success
pub fn send_tx_expect_success(
    svm: &mut LiteSVM,
    ix: Instruction,
    payer: &Keypair,
    signers: &[&Keypair],
) {
    let blockhash = svm.latest_blockhash();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        signers,
        blockhash,
    );

    svm.send_transaction(tx)
        .expect("Transaction should succeed");
}

/// Send a transaction and expect failure
pub fn send_tx_expect_failure(
    svm: &mut LiteSVM,
    ix: Instruction,
    payer: &Keypair,
    signers: &[&Keypair],
) -> String {
    let blockhash = svm.latest_blockhash();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        signers,
        blockhash,
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Transaction should have failed");
    format!("{:?}", result.err().unwrap())
}