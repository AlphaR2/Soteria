// AMM (Automated Market Maker) Program - VULNERABLE VERSION
//
// WARNING: This program contains intentional security vulnerabilities for educational purposes.
// NEVER deploy this to production. Use amm-secure instead.
//
// This vulnerable AMM demonstrates common security flaws in DeFi protocols:
// - Missing slippage protection
// - No expiration validation
// - Unchecked arithmetic (overflow/underflow)
// - Missing authorization checks
// - No pool lock enforcement
// - Improper fee validation
//
// VULNERABILITIES DOCUMENTED:
// See individual instruction files for detailed vulnerability explanations.
// Each vulnerability is marked with VULNERABILITY comments.

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod helpers;

use instructions::*;

declare_id!("AMM1VuLNerAbLe11111111111111111111111111111");

#[program]
pub mod amm_vulnerable {
    use super::*;

    // VULNERABILITY: No fee validation
    pub fn initialize_pool(ctx: Context<InitializePool>, fee_basis_points: u16) -> Result<()> {
        ctx.accounts.initialize_pool(fee_basis_points, &ctx.bumps)
    }

    // VULNERABILITY: Missing slippage and expiration checks
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

    // VULNERABILITY: Missing slippage and expiration checks
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

    // VULNERABILITY: Missing slippage and expiration checks
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

    // VULNERABILITY: No authorization check
    pub fn lock_pool(ctx: Context<LockPool>) -> Result<()> {
        ctx.accounts.lock_pool()
    }

    // VULNERABILITY: No authorization check
    pub fn unlock_pool(ctx: Context<UnlockPool>) -> Result<()> {
        ctx.accounts.unlock_pool()
    }
}
