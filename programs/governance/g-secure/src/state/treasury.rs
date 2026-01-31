
use anchor_lang::prelude::*;

// game config state
#[account]
#[derive(InitSpace)]
pub struct Config {
pub admin: Pubkey,
pub minimum_stake: u64,
pub token_mint: Pubkey,
pub vote_power: u8,
pub is_paused: bool,
pub config_bump: u8,
}

// vault for staking
#[account]
#[derive(InitSpace)]
pub struct Treasury {
	pub admin: Pubkey,
	pub total_staked: u64,
	pub stakers_count: u64,
	pub treasury_token_account: Pubkey,
	pub state_bump: u8,
	pub vault_bump: u8
}