use anchor_lang::prelude::*;
use mpl_core::{
    ID as MPL_CORE_ID,
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{AddPluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{
        Attribute, Attributes, FreezeDelegate, Plugin, PluginAuthority, UpdateAuthority,
    },
};

use crate::{constants::*, errors::NftError, state::CollectionState};

// Stake NFT Instruction
//
// Stakes an NFT by freezing it and tracking the staking timestamp.
// Only the asset owner can stake their NFT.
//
// Adds FreezeDelegate plugin to prevent transfers during staking.
// Adds or updates Attributes plugin to track staking timestamp and accumulated time.

#[derive(Accounts)]
pub struct Stake<'info> {
    // Asset owner
    // Must match asset.owner
    pub owner: Signer<'info>,

    // Collection update authority
    // Must match collection.update_authority
    pub update_authority: Signer<'info>,

    // Payer for plugin additions
    #[account(mut)]
    pub payer: Signer<'info>,

    // Asset being staked
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

impl<'info> Stake<'info> {
    pub fn stake(&mut self) -> Result<()> {
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

        // 4. Get Current Timestamp
        let current_time = Clock::get()?.unix_timestamp;

        // 5. Add or Update Attributes Plugin 
        // The Attribute Plugin is a Authority Managed plugin that can store key value pairs of data within the asset.The Attribute Plugin will work in areas such as: Storing on chain attributes/traits of the Asset which can be read by on chain programs.Storing health and other statistical data that can be modified by a game/program.
        
        match fetch_plugin::<BaseAssetV1, Attributes>(
            &self.asset.to_account_info(),
            mpl_core::types::PluginType::Attributes,
        ) {
            Ok((_, fetched_attribute_list, _)) => {
                // Asset has Attributes plugin - validate and update
                let mut attribute_list: Vec<Attribute> = Vec::new();
                let mut is_initialized: bool = false;

                for attribute in fetched_attribute_list.attribute_list {
                    // we use the stake key for timelocking while storing the timestamp so that we can perform staking checks eg: locking for 30 days etc 

                    if attribute.key == STAKED_KEY {
                        // Ensure asset is not already staked
                        require!(attribute.value == "0", NftError::AlreadyStaked);

                        // Update staked key with current timestamp
                        attribute_list.push(Attribute {
                            key: STAKED_KEY.to_string(),
                            value: current_time.to_string(),
                        });
                        is_initialized = true;
                    } else {
                        attribute_list.push(attribute);
                    }
                }

                // If staking attributes don't exist, add them
                if !is_initialized {
                    attribute_list.push(Attribute {
                        key: STAKED_KEY.to_string(),
                        value: current_time.to_string(),
                    });
                    attribute_list.push(Attribute {
                        key: STAKED_TIME_KEY.to_string(),
                        value: 0.to_string(),
                    });
                }

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
                // Asset doesn't have Attributes plugin - add it
                AddPluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
                    .asset(&self.asset.to_account_info())
                    .collection(Some(&self.collection.to_account_info()))
                    .payer(&self.payer.to_account_info())
                    .authority(Some(&self.update_authority.to_account_info()))
                    .system_program(&self.system_program.to_account_info())
                    .plugin(Plugin::Attributes(Attributes {
                        attribute_list: vec![
                            Attribute {
                                key: STAKED_KEY.to_string(),
                                value: current_time.to_string(),
                            },
                            Attribute {
                                key: STAKED_TIME_KEY.to_string(),
                                value: 0.to_string(),
                            },
                        ],
                    }))
                    .init_authority(PluginAuthority::UpdateAuthority)
                    .invoke()?;
            }
        }

        // 6. Add FreezeDelegate Plugin
        // CRITICAL SECURITY: Use PluginAuthority::UpdateAuthority, NOT Owner
        //
        // PluginAuthority::UpdateAuthority means only the collection authority can
        // remove the freeze, preventing the owner from unstaking without going
        // through our program's unstake instruction.
        //
        // If we used PluginAuthority::Owner, the owner could remove the FreezeDelegate
        // directly via MPL Core, bypassing our staking logic and time tracking.

        AddPluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.payer.to_account_info())
            .authority(Some(&self.owner.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
            .init_authority(PluginAuthority::UpdateAuthority)
            .invoke()?;

        // 7. Increment Staked Counter
        self.collection_state.increment_staked()?;

        Ok(())
    }
}
