// AMM (Automated Market Maker) Program - SECURE VERSION
//
// Constant product AMM (x * y = k) with comprehensive security features.
//
// Instructions:
// - initialize_pool: Create new token pair pool
// - deposit_liquidity: Add tokens, receive LP tokens
// - withdraw_liquidity: Burn LP tokens, receive tokens
// - swap_tokens: Exchange tokens using constant product formula
// - lock_pool / unlock_pool: Emergency pause mechanism

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("FeLNaVGuQZWizZMd2hfy4MaXoko3kLC5q675q5EE5KaC");

#[program]
pub mod amm_secure {
    use super::*;

    pub fn initialize_pool(ctx: Context<InitializePool>, fee_basis_points: u16) -> Result<()> {
        ctx.accounts.initialize_pool(fee_basis_points, &ctx.bumps)
    }

    pub fn deposit_liquidity(
        ctx: Context<DepositLiquidity>,
        desired_amount_a: u64,
        desired_amount_b: u64,
        max_amount_a: u64,
        max_amount_b: u64,
        expiration: i64,
    ) -> Result<()> {
        ctx.accounts.deposit_liquidity(
            desired_amount_a,
            desired_amount_b,
            max_amount_a,
            max_amount_b,
            expiration,
        )
    }

    pub fn withdraw_liquidity(
        ctx: Context<WithdrawLiquidity>,
        lp_tokens_to_burn: u64,
        min_amount_a: u64,
        min_amount_b: u64,
        expiration: i64,
    ) -> Result<()> {
        ctx.accounts.withdraw_liquidity(
            lp_tokens_to_burn,
            min_amount_a,
            min_amount_b,
            expiration,
        )
    }

    pub fn swap_tokens(
        ctx: Context<SwapTokens>,
        swap_token_a_for_b: bool,
        input_amount: u64,
        min_output_amount: u64,
        expiration: i64,
    ) -> Result<()> {
        ctx.accounts.swap_tokens(
            swap_token_a_for_b,
            input_amount,
            min_output_amount,
            expiration,
        )
    }

    pub fn lock_pool(ctx: Context<LockPool>) -> Result<()> {
        ctx.accounts.lock_pool()
    }

    pub fn unlock_pool(ctx: Context<UnlockPool>) -> Result<()> {
        ctx.accounts.unlock_pool()
    }
}
