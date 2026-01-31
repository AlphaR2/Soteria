use anchor_lang::prelude::*;
use mpl_core::{
    ID as MPL_CORE_ID,
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{RemovePluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{
        Attribute, Attributes, FreezeDelegate, Plugin, PluginType, UpdateAuthority,
    },
};

use crate::{constants::*, errors::NftError, state::CollectionState};

// Unstake NFT Instruction
//
// Unstakes an NFT by thawing it and updating total staked time.
// Only the asset owner can unstake their NFT.
//
// Removes FreezeDelegate plugin to allow transfers.
// Updates Attributes plugin to accumulate staked time and reset timestamp.

#[derive(Accounts)]
pub struct Unstake<'info> {
    // Asset owner
    // Must match asset.owner
    pub owner: Signer<'info>,

    // Collection update authority
    // Must match collection.update_authority
    pub update_authority: Signer<'info>,

    // Payer for plugin operations
    #[account(mut)]
    pub payer: Signer<'info>,

    // Asset being unstaked
    // Validates ownership
    #[account(
        mut,
        has_one = owner @ NftError::AssetOwnerMismatch,
    )]
    pub asset: Account<'info, BaseAssetV1>,

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

    // Metaplex Core program
    #[account(address = MPL_CORE_ID @ NftError::InvalidMplCoreProgram)]
    /// CHECK: Validated by address constraint
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Unstake<'info> {
    pub fn unstake(&mut self) -> Result<()> {
        // SECURITY CHECKS

        // 1. Asset Owner Validation
        require!(
            self.asset.owner == self.owner.key(),
            NftError::AssetOwnerMismatch
        );

        // 2. Asset Collection Validation
        require!(
            self.asset.update_authority == UpdateAuthority::Collection(self.collection.key()),
            NftError::AssetNotInCollection
        );

        // 3. Collection Authority Validation
        require!(
            self.update_authority.key() == self.collection_state.authority,
            NftError::CollectionAuthorityMismatch
        );

        // 4. Get Current Timestamp - should be past our staking time 
        
        let current_time = Clock::get()?.unix_timestamp;

        // 5. Update Attributes Plugin
        match fetch_plugin::<BaseAssetV1, Attributes>(
            &self.asset.to_account_info(),
            mpl_core::types::PluginType::Attributes,
        ) {
            Ok((_, fetched_attribute_list, _)) => {
                let mut attribute_list: Vec<Attribute> = Vec::new();
                let mut is_initialized: bool = false;
                let mut staked_time: i64 = 0;

                for attribute in fetched_attribute_list.attribute_list.iter() {
                    if attribute.key == STAKED_KEY {
                        // Ensure asset is currently staked
                        require!(attribute.value != "0", NftError::NotStaked);

                        // Parse staked timestamp
                        let staked_timestamp = attribute
                            .value
                            .parse::<i64>()
                            .map_err(|_| NftError::InvalidTimestamp)?;

                        // Calculate time staked using checked arithmetic
                        let time_staked = current_time
                            .checked_sub(staked_timestamp)
                            .ok_or(NftError::Underflow)?;

                        // Add to accumulated staked_time
                        staked_time = staked_time
                            .checked_add(time_staked)
                            .ok_or(NftError::Overflow)?;

                        // Reset staked key to 0
                        attribute_list.push(Attribute {
                            key: STAKED_KEY.to_string(),
                            value: 0.to_string(),
                        });
                        is_initialized = true;
                    } else if attribute.key == STAKED_TIME_KEY {
                        // Parse existing staked_time
                        let existing_time = attribute
                            .value
                            .parse::<i64>()
                            .map_err(|_| NftError::InvalidTimestamp)?;

                        // Add to total using checked arithmetic
                        staked_time = staked_time
                            .checked_add(existing_time)
                            .ok_or(NftError::Overflow)?;
                    } else {
                        attribute_list.push(attribute.clone());
                    }
                }

                // Ensure staking was initialized
                require!(is_initialized, NftError::StakingNotInitialized);

                // Add updated staked_time to attribute list
                attribute_list.push(Attribute {
                    key: STAKED_TIME_KEY.to_string(),
                    value: staked_time.to_string(),
                });

                // Update the Attributes plugin
                UpdatePluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
                    .asset(&self.asset.to_account_info())
                    .collection(Some(&self.collection.to_account_info()))
                    .payer(&self.payer.to_account_info())
                    .authority(Some(&self.update_authority.to_account_info()))
                    .system_program(&self.system_program.to_account_info())
                    .plugin(Plugin::Attributes(Attributes { attribute_list }))
                    .invoke()?;
            }
            Err(_) => {
                // Attributes plugin must exist for staking
                return Err(NftError::AttributesNotInitialized.into());
            }
        }

        // 6. Thaw Asset by Updating FreezeDelegate
        UpdatePluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.payer.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .invoke()?;

        // 7. Remove FreezeDelegate Plugin
        RemovePluginV1CpiBuilder::new(&self.mpl_core_program)
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.payer)
            .authority(Some(&self.owner))
            .system_program(&self.system_program)
            .plugin_type(PluginType::FreezeDelegate)
            .invoke()?;

        // 8. Decrement Staked Counter
        self.collection_state.decrement_staked()?;

        Ok(())
    }
}
