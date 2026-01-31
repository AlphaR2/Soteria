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

// Unstake NFT Instruction - VULNERABLE VERSION
//
// Unstakes NFTs with critical security vulnerabilities.

#[derive(Accounts)]
pub struct Unstake<'info> {
    pub owner: Signer<'info>,

    pub update_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub asset: Account<'info, BaseAssetV1>,

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

    #[account(address = MPL_CORE_ID @ NftError::InvalidMplCoreProgram)]
    /// CHECK: Validated by address constraint
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Unstake<'info> {
    pub fn unstake(&mut self) -> Result<()> {
        // VULNERABILITY [CRITICAL]: Missing owner validation
        //
        // The secure version validates:
        // require!(self.asset.owner == self.owner.key(), NftError::AssetOwnerMismatch);
        //
        // Without this check, ANYONE can unstake ANYONE ELSE'S NFT!
        //
        // Example Attack:
        //   1. Victim stakes NFT (gets frozen)
        //   2. Attacker calls unstake with victim's NFT
        //   3. NFT gets unfrozen, staked time reset
        //   4. Victim loses accumulated staking time rewards
        //
        // Fix: Validate asset.owner == owner.key()

        // VULNERABILITY [CRITICAL]: Missing collection validation
        //
        // The secure version validates:
        // require!(self.asset.update_authority == UpdateAuthority::Collection(self.collection.key()), ...);
        //
        // Without this check, can unstake NFTs from ANY collection!
        //
        // Example Attack:
        //   1. Create fake collection with same structure
        //   2. Stake fake NFT in this program
        //   3. Unstake it, decrementing legitimate collection's total_staked
        //   4. Counter underflows or shows incorrect data
        //
        // Fix: Validate asset belongs to the correct collection

        // VULNERABILITY [MEDIUM]: Missing authority validation
        //
        // Fix: require!(self.update_authority.key() == self.collection_state.authority, ...);

        let current_time = Clock::get()?.unix_timestamp;

        // VULNERABILITY [HIGH]: Missing staking state validation
        //
        // The secure version checks:
        // require!(attribute.value != "0", NftError::NotStaked);
        //
        // Without this check, can unstake non-staked NFTs, causing:
        //   1. Counter underflow (total_staked decrements when it shouldn't)
        //   2. Incorrect staking statistics
        //   3. Potential for griefing attacks
        //
        // Fix: Check if staked attribute value is != "0" before unstaking

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
                        // VULNERABLE: No check if already unstaked!

                        // Parse staked timestamp - VULNERABLE: No error handling
                        let staked_timestamp = attribute
                            .value
                            .parse::<i64>()
                            .unwrap_or(0);

                        // VULNERABILITY [MEDIUM]: Unchecked arithmetic
                        //
                        // The secure version uses:
                        // current_time.checked_sub(staked_timestamp).ok_or(NftError::Underflow)?;
                        //
                        // Without checked arithmetic, can cause integer underflow/overflow
                        //
                        // Fix: Use checked_sub and checked_add
                        let time_staked = current_time - staked_timestamp; // VULNERABLE: Unchecked sub

                        staked_time = staked_time + time_staked; // VULNERABLE: Unchecked add

                        attribute_list.push(Attribute {
                            key: STAKED_KEY.to_string(),
                            value: 0.to_string(),
                        });
                        is_initialized = true;
                    } else if attribute.key == STAKED_TIME_KEY {
                        let existing_time = attribute
                            .value
                            .parse::<i64>()
                            .unwrap_or(0);

                        staked_time = staked_time + existing_time; // VULNERABLE: Unchecked add
                    } else {
                        attribute_list.push(attribute.clone());
                    }
                }

                // VULNERABLE: No check if staking was initialized!

                attribute_list.push(Attribute {
                    key: STAKED_TIME_KEY.to_string(),
                    value: staked_time.to_string(),
                });

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
                // VULNERABLE: No proper error handling
                return Err(NftError::AttributesNotInitialized.into());
            }
        }

        UpdatePluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.payer.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .invoke()?;

        RemovePluginV1CpiBuilder::new(&self.mpl_core_program)
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.payer)
            .authority(Some(&self.owner))
            .system_program(&self.system_program)
            .plugin_type(PluginType::FreezeDelegate)
            .invoke()?;

        // VULNERABILITY [MEDIUM]: Unchecked decrement
        //
        // Fix: self.collection_state.decrement_staked()?;
        self.collection_state.total_staked -= 1; // VULNERABLE: Can underflow

        Ok(())
    }
}
