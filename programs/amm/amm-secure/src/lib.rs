// AMM (Automated Market Maker) Program - SECURE VERSION
//
// Implementation of a constant product market maker (x * y = k) for token swaps.
// This AMM allows users to:
// 1. Create liquidity pools for any SPL token pair
// 2. Provide liquidity and earn fees by receiving LP tokens
// 3. Swap tokens at prices determined by the constant product formula
// 4. Remove liquidity by burning LP tokens
//
// SECURITY FEATURES:
// - Pool lock/unlock for emergency pause
// - Slippage protection via min/max amounts
// - Expiration timestamps to prevent stale transactions
// - Fee validation (max 10%)
// - Checked arithmetic to prevent overflow/underflow
// - Box<Account> to reduce stack usage and prevent stack overflow
//
// CONSTANT PRODUCT FORMULA:
// The pool maintains: token_a_reserve * token_b_reserve = k (constant)
// When swapping, the product k must remain constant after accounting for fees.
// Price is determined by the ratio of reserves.

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod helpers;

use instructions::*;

declare_id!("FeLNaVGuQZWizZMd2hfy4MaXoko3kLC5q675q5EE5KaC");

#[program]
pub mod amm_secure {
    use super::*;

    // Create a new liquidity pool for a token pair
    // Only needs to be called once per token pair
    pub fn initialize_pool(ctx: Context<InitializePool>, fee_basis_points: u16) -> Result<()> {
        ctx.accounts.initialize_pool(fee_basis_points, &ctx.bumps)
    }

    // Add liquidity to the pool and receive LP tokens
    // First deposit uses sqrt(a * b) formula, subsequent deposits are proportional
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

    // Remove liquidity by burning LP tokens
    // Returns proportional share of both tokens from the pool
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

    // Swap one token for another using constant product formula
    // Fee is deducted from input before calculating output
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

    // Emergency pause - only pool authority can lock
    pub fn lock_pool(ctx: Context<LockPool>) -> Result<()> {
        ctx.accounts.lock_pool()
    }

    // Resume operations - only pool authority can unlock
    pub fn unlock_pool(ctx: Context<UnlockPool>) -> Result<()> {
        ctx.accounts.unlock_pool()
    }
}
