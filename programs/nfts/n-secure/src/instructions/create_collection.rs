use anchor_lang::prelude::*;
use mpl_core::{
    ID as MPL_CORE_ID,
    instructions::CreateCollectionV2CpiBuilder,
};

use crate::{constants::*, errors::NftError, state::CollectionState};

// Create Collection Instruction
//
// Initializes a new Metaplex Core collection and creates tracking state
// for the NFT staking program.
//
// The authority becomes the collection update authority and can mint NFTs.
// Collection state tracks total minted and staked NFTs.

#[derive(Accounts)]
pub struct CreateCollection<'info> {
    // Collection authority
    // Must sign and pay for state account creation
    #[account(mut)]
    pub authority: Signer<'info>,

    // Collection account (Metaplex Core)
    // Must be a new keypair, must sign
    #[account(mut)]
    pub collection: Signer<'info>,

    // Collection state PDA
    // Seeds: ["collection_state", collection]
    // Stores minting and staking counters
    #[account(
        init,
        payer = payer,
        space = ANCHOR_DISCRIMINATOR + CollectionState::INIT_SPACE,
        seeds = [
            COLLECTION_STATE,
            collection.key().as_ref(),
        ],
        bump,
    )]
    pub collection_state: Account<'info, CollectionState>,

    // Payer for account creation
    #[account(mut)]
    pub payer: Signer<'info>,

    // Metaplex Core program
    // Validated to prevent fake program CPI
    #[account(address = MPL_CORE_ID @ NftError::InvalidMplCoreProgram)]
    /// CHECK: Validated by address constraint
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreateCollection<'info> {
    pub fn create_collection(
        &mut self,
        name: String,
        uri: String,
        bumps: &CreateCollectionBumps,
    ) -> Result<()> {
        // SECURITY CHECKS

        // 1. Name Validation
        // Ensures name is not empty and within allowed length
        require!(!name.is_empty(), NftError::EmptyName);
        require!(name.len() <= MAX_NAME_LENGTH, NftError::NameTooLong);

        // 2. URI Validation
        // Ensures URI is not empty and within allowed length
        require!(!uri.is_empty(), NftError::EmptyUri);
        require!(uri.len() <= MAX_URI_LENGTH, NftError::UriTooLong);

        // 3. Create Metaplex Core Collection via CPI
        // Authority becomes the update authority for the collection
        CreateCollectionV2CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .collection(&self.collection.to_account_info())
            .payer(&self.payer.to_account_info())
            .update_authority(Some(&self.authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .name(name)
            .uri(uri)
            .invoke()?;

        // 4. Initialize Collection State
        // Set authority and collection pubkey
        // Initialize counters to zero
        self.collection_state.set_inner(CollectionState {
            authority: self.authority.key(),
            collection: self.collection.key(),
            total_minted: 0,
            total_staked: 0,
            bump: bumps.collection_state,
        });

        Ok(())
    }
}
