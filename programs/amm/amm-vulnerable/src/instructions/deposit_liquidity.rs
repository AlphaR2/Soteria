// Deposit Liquidity Instruction - VULNERABLE VERSION
//
// WARNING: This version contains intentional vulnerabilities for educational purposes.
//
// VULNERABILITIES:
// V002: No slippage protection - depositors can be front-run
// V003: No expiration validation - stale transactions can execute
// V005: MINIMUM_LIQUIDITY = 1 - enables inflation attacks
// V007: No pool lock enforcement - deposits work even when pool is locked
// V010: No zero amount checks - wastes gas and can cause unexpected behavior

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{constants::*, errors::*, state::*, helpers::*};

#[derive(Accounts)]
pub struct DepositLiquidity<'info> {
    // User adding liquidity (pays for ATA creation if needed)
    #[account(mut)]
    pub depositor: Signer<'info>,

    // Pool configuration PDA
    #[account(
        seeds = [
            AMM_CONFIG_SEED,
            pool_config.token_a_mint.as_ref(),
            pool_config.token_b_mint.as_ref(),
        ],
        bump = pool_config.config_bump,
    )]
    pub pool_config: Box<Account<'info, PoolConfig>>,

    // Pool authority PDA (signs token transfers from vaults)
    /// CHECK: PDA signer, validated by seeds
    #[account(
        seeds = [AMM_AUTHORITY_SEED, pool_config.key().as_ref()],
        bump = pool_config.authority_bump,
    )]
    pub pool_authority: UncheckedAccount<'info>,

    // LP token mint (pool authority is mint authority)
    #[account(
        mut,
        seeds = [LP_MINT_SEED, pool_config.key().as_ref()],
        bump = pool_config.lp_mint_bump,
        mint::authority = pool_authority,
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    // Token A mint (verified against pool_config)
    #[account(address = pool_config.token_a_mint)]
    pub token_a_mint: Box<Account<'info, Mint>>,

    // Token B mint (verified against pool_config)
    #[account(address = pool_config.token_b_mint)]
    pub token_b_mint: Box<Account<'info, Mint>>,

    // Depositor's token A account (source of token A)
    // Anchor validates mint and authority via constraints
    #[account(
        mut,
        token::mint = token_a_mint,
        token::authority = depositor,
    )]
    pub depositor_token_a: Account<'info, TokenAccount>,

    // Depositor's token B account (source of token B)
    #[account(
        mut,
        token::mint = token_b_mint,
        token::authority = depositor,
    )]
    pub depositor_token_b: Account<'info, TokenAccount>,

    // Depositor's LP token account (created if doesn't exist)
    #[account(
        init_if_needed,
        payer = depositor,
        associated_token::mint = lp_token_mint,
        associated_token::authority = depositor,
    )]
    pub depositor_lp_token: Box<Account<'info, TokenAccount>>,

    // Pool's token A vault (holds all token A in the pool)
    #[account(
        mut,
        token::mint = token_a_mint,
        token::authority = pool_authority,
    )]
    pub token_a_vault: Box<Account<'info, TokenAccount>>,

    // Pool's token B vault (holds all token B in the pool)
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

impl<'info> DepositLiquidity<'info> {
    pub fn deposit_liquidity(
        &mut self,
        desired_amount_a: u64,
        desired_amount_b: u64,
        max_amount_a: u64,
        max_amount_b: u64,
        expiration: i64,
    ) -> Result<()> {
        // VULNERABILITY V007: No pool lock enforcement
        // Secure version: self.pool_config.assert_not_locked()?;
        // Attack: Pool has critical bug, admin locks it, but deposits continue anyway

        // VULNERABILITY V003: No expiration validation
        // Secure version: validate_expiration(expiration)?;
        // Attack: User submits deposit at good ratio, transaction pending for hours,
        // executes at terrible ratio after pool has been heavily traded

        // VULNERABILITY V010: No zero amount checks
        // Secure version: require!(desired_amount_a > 0, AmmError::ZeroDepositAmount);
        // Secure version: require!(desired_amount_b > 0, AmmError::ZeroDepositAmount);
        // Impact: Wastes gas, can cause unexpected behavior in calculations

        let vault_a_balance = self.token_a_vault.amount;
        let vault_b_balance = self.token_b_vault.amount;
        let lp_supply = self.lp_token_mint.supply;

        // Calculate deposit amounts and LP tokens using helpers
        // VULNERABILITY V004: Unchecked arithmetic in helpers (overflow/underflow risk)
        // VULNERABILITY V005: MINIMUM_LIQUIDITY = 1 (enables inflation attacks)
        let (amount_a, amount_b, lp_tokens) = if lp_supply == 0 {
            calculate_first_deposit(desired_amount_a, desired_amount_b)?
        } else {
            calculate_subsequent_deposit(
                desired_amount_a,
                desired_amount_b,
                vault_a_balance,
                vault_b_balance,
                lp_supply,
            )?
        };

        // VULNERABILITY V002: No slippage protection
        // Secure version: require!(amount_a <= max_amount_a, AmmError::ExcessiveDepositAmount);
        // Secure version: require!(amount_b <= max_amount_b, AmmError::ExcessiveDepositAmount);
        // Attack scenario:
        // 1. Victim submits deposit with max_amount_a=1000, max_amount_b=1000
        // 2. Front-runner sees pending transaction
        // 3. Front-runner manipulates pool ratio by doing massive swap
        // 4. Victim's deposit executes at terrible ratio, depositing way more than expected
        // 5. Victim receives far fewer LP tokens than expected

        // VULNERABILITY V010: No check that LP tokens > 0
        // Secure version: require!(lp_tokens > 0, AmmError::InsufficientLiquidity);
        // Impact: User could deposit tokens and receive 0 LP tokens due to rounding

        // Transfer tokens to vaults using helper
        transfer_tokens(
            amount_a,
            &self.token_program.to_account_info(),
            &self.depositor_token_a.to_account_info(),
            &self.token_a_vault.to_account_info(),
            &self.depositor.to_account_info(),
        )?;

        transfer_tokens(
            amount_b,
            &self.token_program.to_account_info(),
            &self.depositor_token_b.to_account_info(),
            &self.token_b_vault.to_account_info(),
            &self.depositor.to_account_info(),
        )?;

        // Mint LP tokens using helper
        let pool_config_key = self.pool_config.key();
        let authority_seeds = &[
            AMM_AUTHORITY_SEED,
            pool_config_key.as_ref(),
            &[self.pool_config.authority_bump],
        ];

        mint_lp_tokens(
            lp_tokens,
            &self.token_program.to_account_info(),
            &self.lp_token_mint.to_account_info(),
            &self.depositor_lp_token.to_account_info(),
            &self.pool_authority.to_account_info(),
            authority_seeds,
        )?;

        msg!("Deposited: {} A, {} B -> {} LP", amount_a, amount_b, lp_tokens);

        Ok(())
    }
}