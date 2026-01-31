use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("xbwEtBJ9eoyGCAkvr4P2JmMH8wSnrb6amh2po57oGGJ");

#[program]
pub mod nft_staking_secure {
    use super::*;

    pub fn create_collection(
        ctx: Context<CreateCollection>,
        name: String,
        uri: String,
    ) -> Result<()> {
        ctx.accounts.create_collection(name, uri, &ctx.bumps)
    }

    pub fn mint_nft(
        ctx: Context<MintNft>,
        name: String,
        uri: String,
    ) -> Result<()> {
        ctx.accounts.mint_nft(name, uri)
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        ctx.accounts.stake()
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        ctx.accounts.unstake()
    }
}
