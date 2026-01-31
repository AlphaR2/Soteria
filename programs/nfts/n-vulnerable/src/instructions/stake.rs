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

// Stake NFT Instruction - VULNERABLE VERSION
//
// Stakes NFTs with critical security vulnerabilities.

#[derive(Accounts)]
pub struct Stake<'info> {
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

impl<'info> Stake<'info> {
    pub fn stake(&mut self) -> Result<()> {
        // VULNERABILITY [CRITICAL]: Missing owner validation
        //
        // The secure version validates:
        // require!(self.asset.owner == self.owner.key(), NftError::AssetOwnerMismatch);
        //
        // Without this check, ANYONE can stake ANYONE ELSE'S NFT!
        //
        // Example Attack:
        //   1. Victim owns valuable NFT
        //   2. Attacker calls stake with victim's NFT
        //   3. NFT gets frozen, victim can't transfer it
        //   4. Attacker can grief victim or manipulate staking stats
        //
        // Fix: Validate asset.owner == owner.key()

        // VULNERABILITY [CRITICAL]: Missing collection validation
        //
        // The secure version validates:
        // require!(self.asset.update_authority == UpdateAuthority::Collection(self.collection.key()), ...);
        //
        // Without this check, can stake NFTs from ANY collection!
        //
        // Example Attack:
        //   1. Create fake collection with same structure
        //   2. Mint worthless NFTs in fake collection
        //   3. Stake them in this program
        //   4. Inflate total_staked counter for legitimate collection
        //
        // Fix: Validate asset belongs to the correct collection

        // VULNERABILITY [MEDIUM]: Missing authority validation
        //
        // Fix: require!(self.update_authority.key() == self.collection_state.authority, ...);

        let current_time = Clock::get()?.unix_timestamp;

        // VULNERABILITY [HIGH]: Missing double-staking check
        //
        // The secure version checks if already staked:
        // require!(attribute.value == "0", NftError::AlreadyStaked);
        //
        // Without this, can stake an already-staked NFT multiple times,
        // inflating the total_staked counter.
        //
        // Fix: Check if staked attribute value is "0" before staking

        match fetch_plugin::<BaseAssetV1, Attributes>(
            &self.asset.to_account_info(),
            mpl_core::types::PluginType::Attributes,
        ) {
            Ok((_, fetched_attribute_list, _)) => {
                let mut attribute_list: Vec<Attribute> = Vec::new();
                let mut is_initialized: bool = false;

                for attribute in fetched_attribute_list.attribute_list {
                    if attribute.key == STAKED_KEY {
                        // VULNERABLE: No check if already staked!
                        attribute_list.push(Attribute {
                            key: STAKED_KEY.to_string(),
                            value: current_time.to_string(),
                        });
                        is_initialized = true;
                    } else {
                        attribute_list.push(attribute);
                    }
                }

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

        // VULNERABILITY [CRITICAL]: Wrong plugin authority
        //
        // Using PluginAuthority::Owner instead of UpdateAuthority!
        //
        // This means the OWNER can remove the FreezeDelegate themselves
        // via MPL Core, bypassing our unstake logic and time tracking.
        //
        // The secure version uses PluginAuthority::UpdateAuthority so only
        // the collection authority (via our unstake instruction) can remove it.
        //
        // Example Attack:
        //   1. Stake NFT (adds FreezeDelegate with Owner authority)
        //   2. Owner directly calls MPL Core RemovePlugin
        //   3. FreezeDelegate removed, NFT unfrozen
        //   4. Owner transfers NFT, bypassing our unstake tracking
        //   5. total_staked counter never decrements
        //
        // Fix: Use PluginAuthority::UpdateAuthority

        AddPluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.payer.to_account_info())
            .authority(Some(&self.owner.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
            .init_authority(PluginAuthority::Owner) // VULNERABLE: Should be UpdateAuthority
            .invoke()?;

        // VULNERABILITY [MEDIUM]: Unchecked increment
        //
        // Fix: self.collection_state.increment_staked()?;
        self.collection_state.total_staked += 1; // VULNERABLE: Can overflow

        Ok(())
    }
}
