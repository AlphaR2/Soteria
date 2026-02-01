// Withdraw Liquidity Instruction
//
// Allows users to burn LP tokens and receive their proportional share of pool tokens.
//
// HOW IT WORKS:
// 1. User specifies amount of LP tokens to burn
// 2. Calculate proportional withdrawal: amount = (lp_burned / lp_supply) * vault_balance
// 3. Burn LP tokens from user's account
// 4. Transfer proportional amounts of both tokens from vaults to user
//
// SECURITY:
// - Slippage protection: User sets minimum amounts they expect to receive
// - Balance verification: Ensures vaults have sufficient tokens before transfer
// - Expiration check: Prevents stale transactions
// - Pool lock check: Withdrawal disabled when pool is paused

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{constants::*, errors::*, helpers::*, state::*};

#[derive(Accounts)]
pub struct WithdrawLiquidity<'info> {
    #[account(mut)]
    pub withdrawer: Signer<'info>,

    #[account(
        seeds = [
            AMM_CONFIG_SEED,
            pool_config.token_a_mint.as_ref(),
            pool_config.token_b_mint.as_ref(),
        ],
        bump = pool_config.config_bump,
    )]
    pub pool_config: Box<Account<'info, PoolConfig>>,

    /// CHECK: PDA signer
    #[account(
        seeds = [AMM_AUTHORITY_SEED, pool_config.key().as_ref()],
        bump = pool_config.authority_bump,
    )]
    pub pool_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [LP_MINT_SEED, pool_config.key().as_ref()],
        bump = pool_config.lp_mint_bump,
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    #[account(address = pool_config.token_a_mint)]
    pub token_a_mint: Box<Account<'info, Mint>>,

    #[account(address = pool_config.token_b_mint)]
    pub token_b_mint: Box<Account<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = withdrawer,
        associated_token::mint = token_a_mint,
        associated_token::authority = withdrawer,
    )]
    pub withdrawer_token_a: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = withdrawer,
        associated_token::mint = token_b_mint,
        associated_token::authority = withdrawer,
    )]
    pub withdrawer_token_b: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = lp_token_mint,           
        token::authority = withdrawer,         
    )]
    pub withdrawer_lp_token: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_a_mint,
        token::authority = pool_authority,
    )]
    pub token_a_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = token_b_mint,
        token::authority = pool_authority,
    )]
    pub token_b_vault: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> WithdrawLiquidity<'info> {
    pub fn withdraw_liquidity(
        &mut self,
        lp_tokens_to_burn: u64,
        min_amount_a: u64,
        min_amount_b: u64,
        expiration: i64,
    ) -> Result<()> {
        // Validate withdrawer LP token account (Anchor constraints already check mint and authority)
        require!(self.withdrawer_lp_token.amount >= lp_tokens_to_burn, AmmError::InsufficientBalance);

        // Check pool not locked
        self.pool_config.assert_not_locked()?;

        // Validate expiration using helper
        validate_expiration(expiration)?;

        // Check non-zero LP amount
        require!(lp_tokens_to_burn > 0, AmmError::ZeroWithdrawAmount);

        let vault_a_balance = self.token_a_vault.amount;
        let vault_b_balance = self.token_b_vault.amount;
        let lp_supply = self.lp_token_mint.supply;

        // Check pool has liquidity
        require!(lp_supply > 0, AmmError::InsufficientLiquidity);
        require!(vault_a_balance > 0, AmmError::InsufficientLiquidity);
        require!(vault_b_balance > 0, AmmError::InsufficientLiquidity);

        // Calculate withdrawal amounts using helper
        let (amount_a, amount_b) = calculate_withdrawal(
            lp_tokens_to_burn,
            vault_a_balance,
            vault_b_balance,
            lp_supply,
        )?;

        // Slippage protection
        require!(amount_a >= min_amount_a, AmmError::InsufficientWithdrawAmount);
        require!(amount_b >= min_amount_b, AmmError::InsufficientWithdrawAmount);

        // Check non-zero withdrawals
        require!(amount_a > 0, AmmError::InsufficientLiquidity);
        require!(amount_b > 0, AmmError::InsufficientLiquidity);

        // Check vault balances
        require!(vault_a_balance >= amount_a, AmmError::InsufficientPoolLiquidity);
        require!(vault_b_balance >= amount_b, AmmError::InsufficientPoolLiquidity);

        // Burn LP tokens using helper
        burn_lp_tokens(
            lp_tokens_to_burn,
            &self.token_program.to_account_info(),
            &self.lp_token_mint.to_account_info(),
            &self.withdrawer_lp_token.to_account_info(),
            &self.withdrawer.to_account_info(),
        )?;

        // Transfer tokens from vaults using helper
        let pool_config_key = self.pool_config.key();
        let authority_seeds = &[
            AMM_AUTHORITY_SEED,
            pool_config_key.as_ref(),
            &[self.pool_config.authority_bump],
        ];

        transfer_from_vault(
            amount_a,
            &self.token_program.to_account_info(),
            &self.token_a_vault.to_account_info(),
            &self.withdrawer_token_a.to_account_info(),
            &self.pool_authority.to_account_info(),
            authority_seeds,
        )?;

        transfer_from_vault(
            amount_b,
            &self.token_program.to_account_info(),
            &self.token_b_vault.to_account_info(),
            &self.withdrawer_token_b.to_account_info(),
            &self.pool_authority.to_account_info(),
            authority_seeds,
        )?;

        msg!("Withdrawn: {} LP -> {} A, {} B", lp_tokens_to_burn, amount_a, amount_b);

        Ok(())
    }
}