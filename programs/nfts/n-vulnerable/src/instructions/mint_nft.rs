use anchor_lang::prelude::*;
use mpl_core::{
    ID as MPL_CORE_ID,
    accounts::BaseCollectionV1,
    instructions::CreateV2CpiBuilder,
};

use crate::{constants::*, errors::NftError, state::CollectionState};

// Mint NFT Instruction - VULNERABLE VERSION
//
// Mints NFTs with critical security vulnerabilities.

#[derive(Accounts)]
pub struct MintNft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub asset: Signer<'info>,

    #[account(mut)]
    pub collection: Account<'info, BaseCollectionV1>,

    #[account(
        mut,
        seeds = [
            COLLECTION_STATE,
            collection.key().as_ref(),
        ],
        bump = collection_state.bump,
    )]
    pub collection_state: Account<'info, CollectionState>,

    /// CHECK: Can be any account
    pub owner: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(address = MPL_CORE_ID @ NftError::InvalidMplCoreProgram)]
    /// CHECK: Validated by address constraint
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> MintNft<'info> {
    pub fn mint_nft(
        &mut self,
        name: String,
        uri: String,
    ) -> Result<()> {
        // VULNERABILITY [HIGH]: Missing name validation
        //
        // Fix: require!(!name.is_empty(), NftError::EmptyName);
        // Fix: require!(name.len() <= MAX_NAME_LENGTH, NftError::NameTooLong);

        // VULNERABILITY [HIGH]: Missing URI validation
        //
        // Fix: require!(!uri.is_empty(), NftError::EmptyUri);
        // Fix: require!(uri.len() <= MAX_URI_LENGTH, NftError::UriTooLong);

        // VULNERABILITY [MEDIUM]: Missing collection authority validation
        //
        // Fix: require!(self.authority.key() == self.collection_state.authority, ...);

        // VULNERABILITY [MEDIUM]: Missing collection validation
        //
        // Fix: require!(self.collection.key() == self.collection_state.collection, ...);

        // VULNERABILITY [CRITICAL]: No immutability plugins
        //
        // The secure version adds Plugin::ImmutableMetadata and Plugin::AddBlocker.
        // Without these, authority can change metadata or add malicious plugins.
        //
        // Example Attack:
        //   1. Mint NFT with attractive metadata
        //   2. User buys NFT
        //   3. Authority changes metadata to worthless image
        //   4. OR Authority adds 100% royalty plugin
        //
        // Fix: Add .plugins(vec![Plugin::ImmutableMetadata, Plugin::AddBlocker])

        CreateV2CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .authority(Some(&self.authority.to_account_info()))
            .payer(&self.payer.to_account_info())
            .owner(Some(&self.owner.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .name(name)   // VULNERABLE: Unvalidated
            .uri(uri)     // VULNERABLE: Unvalidated
            // VULNERABLE: No .plugins(...) - missing immutability
            .invoke()?;

        // VULNERABILITY [MEDIUM]: Integer overflow on minted counter
        //
        // Fix: self.collection_state.increment_minted()?;
        self.collection_state.total_minted += 1; // VULNERABLE: Unchecked add

        Ok(())
    }
}

