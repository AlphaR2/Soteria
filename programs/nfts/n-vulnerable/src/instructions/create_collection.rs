use anchor_lang::prelude::*;
use mpl_core::{
    ID as MPL_CORE_ID,
    instructions::CreateCollectionV2CpiBuilder,
};

use crate::{constants::*, errors::NftError, state::CollectionState};

// Create Collection Instruction - VULNERABLE VERSION
//
// Creates a Metaplex Core collection with missing validation checks.

#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub collection: Signer<'info>,

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

    #[account(mut)]
    pub payer: Signer<'info>,

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
        // VULNERABILITY [HIGH]: Missing name validation
        //
        // The secure version validates:
        // - require!(!name.is_empty(), NftError::EmptyName);
        // - require!(name.len() <= MAX_NAME_LENGTH, NftError::NameTooLong);
        //
        // Without validation:
        // - Empty names allowed (confusing/invalid collections)
        // - Excessively long names allowed (storage bloat, potential overflow)
        //
        // Fix: Add name validation checks

        // VULNERABILITY [HIGH]: Missing URI validation
        //
        // The secure version validates:
        // - require!(!uri.is_empty(), NftError::EmptyUri);
        // - require!(uri.len() <= MAX_URI_LENGTH, NftError::UriTooLong);
        //
        // Without validation:
        // - Empty URIs allowed (no metadata)
        // - Excessively long URIs allowed (storage bloat)
        //
        // Fix: Add URI validation checks

        CreateCollectionV2CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .collection(&self.collection.to_account_info())
            .payer(&self.payer.to_account_info())
            .update_authority(Some(&self.authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .name(name) // VULNERABLE: Unvalidated name
            .uri(uri)   // VULNERABLE: Unvalidated URI
            .invoke()?;

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
