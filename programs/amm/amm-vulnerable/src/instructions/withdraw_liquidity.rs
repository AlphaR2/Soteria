// Withdraw Liquidity Instruction - VULNERABLE VERSION
//
// WARNING: This version contains intentional vulnerabilities for educational purposes.
//
// VULNERABILITIES:
// V008: No withdrawal slippage protection - withdrawers can be sandwiched
// V003: No expiration validation - stale transactions can execute
// V007: No pool lock enforcement - withdrawals work even when pool is locked
// V010: No zero amount checks - wastes gas
// V011: No liquidity checks before operations - may fail ungracefully

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

        // VULNERABILITY V007: No pool lock enforcement
        // Secure version: self.pool_config.assert_not_locked()?;
        // Attack: Pool has critical bug, admin locks it, but withdrawals continue anyway

        // VULNERABILITY V003: No expiration validation
        // Secure version: validate_expiration(expiration)?;
        // Attack: User submits withdrawal at good ratio, transaction pending for hours,
        // executes at terrible ratio after pool has been manipulated

        // VULNERABILITY V010: No zero amount checks
        // Secure version: require!(lp_tokens_to_burn > 0, AmmError::ZeroWithdrawAmount);
        // Impact: Wastes gas processing 0-amount withdrawals

        let vault_a_balance = self.token_a_vault.amount;
        let vault_b_balance = self.token_b_vault.amount;
        let lp_supply = self.lp_token_mint.supply;

        // VULNERABILITY V011: No liquidity checks before operations
        // Secure version: require!(lp_supply > 0, AmmError::InsufficientLiquidity);
        // Secure version: require!(vault_a_balance > 0, AmmError::InsufficientLiquidity);
        // Secure version: require!(vault_b_balance > 0, AmmError::InsufficientLiquidity);
        // Impact: Operations may fail ungracefully if pool is empty

        // Calculate withdrawal amounts using helper
        // VULNERABILITY V004: Unchecked arithmetic in calculate_withdrawal (overflow/underflow risk)
        let (amount_a, amount_b) = calculate_withdrawal(
            lp_tokens_to_burn,
            vault_a_balance,
            vault_b_balance,
            lp_supply,
        )?;

        // VULNERABILITY V008: No withdrawal slippage protection
        // Secure version: require!(amount_a >= min_amount_a, AmmError::InsufficientWithdrawAmount);
        // Secure version: require!(amount_b >= min_amount_b, AmmError::InsufficientWithdrawAmount);
        // Attack scenario (sandwich attack):
        // 1. Victim submits withdrawal expecting min_amount_a=1000, min_amount_b=1000
        // 2. Front-runner sees pending transaction
        // 3. Front-runner manipulates pool ratio with large swap
        // 4. Victim's withdrawal executes at terrible ratio
        // 5. Victim receives far less tokens than expected (but no error because no check)
        // 6. Back-runner restores pool ratio, profiting from victim's loss

        // VULNERABILITY V010: No check that withdrawn amounts > 0
        // Secure version: require!(amount_a > 0, AmmError::InsufficientLiquidity);
        // Secure version: require!(amount_b > 0, AmmError::InsufficientLiquidity);
        // Impact: User could burn LP tokens and receive 0 tokens due to rounding

        // VULNERABILITY V011: No vault balance validation
        // Secure version: require!(vault_a_balance >= amount_a, AmmError::InsufficientPoolLiquidity);
        // Secure version: require!(vault_b_balance >= amount_b, AmmError::InsufficientPoolLiquidity);
        // Impact: May fail ungracefully in transfer if vaults don't have enough balance

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