use anchor_lang::prelude::*;
use mpl_core::{
    ID as MPL_CORE_ID,
    accounts::BaseCollectionV1,
    instructions::CreateV2CpiBuilder,
};

use mpl_core::types::{
    Plugin,
    PluginAuthorityPair,
    ImmutableMetadata,
    AddBlocker, 
};

use crate::{constants::*, errors::NftError, state::CollectionState};

// Mint NFT Instruction
//
// Mints a new NFT asset into the collection via Metaplex Core.
// Only the collection authority can mint.
//
// The asset account must be a new keypair to ensure uniqueness.
// Updates the collection state's total_minted counter.

#[derive(Accounts)]
pub struct MintNft<'info> {
    // Collection authority
    // Must match collection_state.authority
    #[account(mut)]
    pub authority: Signer<'info>,

    // Asset account (new NFT being minted)
    // Must be a new keypair, must sign
    #[account(mut)]
    pub asset: Signer<'info>,

    // Metaplex Core collection
    // Validates authority controls the collection
    #[account(
        mut,
        has_one = update_authority @ NftError::CollectionAuthorityMismatch,
    )]
    pub collection: Account<'info, BaseCollectionV1>,

    // Collection state PDA
    // Seeds: ["collection_state", collection]
    // Tracks total minted and staked
    #[account(
        mut,
        seeds = [
            COLLECTION_STATE,
            collection.key().as_ref(),
        ],
        bump = collection_state.bump,
    )]
    pub collection_state: Account<'info, CollectionState>,

    // Collection update authority
    // Validated by BaseCollectionV1 has_one constraint
    /// CHECK: Validated by has_one constraint
    pub update_authority: UncheckedAccount<'info>,

    // NFT owner (recipient)
    /// CHECK: Can be any account, passed to MPL Core
    pub owner: UncheckedAccount<'info>,

    // Payer for account creation
    #[account(mut)]
    pub payer: Signer<'info>,

    // Metaplex Core program
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
        // SECURITY CHECKS

        // 1. Name Validation
        require!(!name.is_empty(), NftError::EmptyName);
        require!(name.len() <= MAX_NAME_LENGTH, NftError::NameTooLong);

        // 2. URI Validation
        require!(!uri.is_empty(), NftError::EmptyUri);
        require!(uri.len() <= MAX_URI_LENGTH, NftError::UriTooLong);

        // 3. Collection Authority Validation
        // Ensure signer is the collection authority
        require!(
            self.authority.key() == self.collection_state.authority,
            NftError::CollectionAuthorityMismatch
        );

        // 4. Collection Validation
        // Ensure collection matches collection_state
        require!(
            self.collection.key() == self.collection_state.collection,
            NftError::InvalidCollection
        );

        // 5. Mint NFT via CPI to Metaplex Core
        // Add ImmutableMetadata plugin for security
        // - ImmutableMetadata: Prevents name/URI tampering after minting
        //
        // NOTE: We do NOT add AddBlocker here because our staking program
        // needs to add Attributes and FreezeDelegate plugins during stake/unstake.
        // The collection authority controls which plugins can be added through
        // program logic, so AddBlocker is not necessary at the asset level.

        let mut plugins: Vec<PluginAuthorityPair> = vec![];

        plugins.push(PluginAuthorityPair {
        plugin: Plugin::ImmutableMetadata(ImmutableMetadata{}),
        authority: None,
        });

        CreateV2CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .authority(Some(&self.authority.to_account_info()))
            .payer(&self.payer.to_account_info())
            .owner(Some(&self.owner.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .name(name)
            .uri(uri)
            .plugins(plugins)
            .invoke()?;

        // NOTE: Update Authority Revocation (Production Best Practice)
        //
        // For maximum immutability in production, the update authority should be
        // revoked after minting via UpdateAsset instruction to set it to None.
        // This prevents ANY future modifications to the asset, even by the authority.
        //
        // Not implemented here to keep the example focused on core staking logic.
        // In production: Call UpdateAsset CPI with update_authority = None

        // 6. Increment Minted Counter
        // Uses checked arithmetic to prevent overflow
        self.collection_state.increment_minted()?;

        Ok(())
    }
}
