// Deposit Liquidity Instruction
//
// Adds liquidity to pool and receives LP tokens.
// First deposit: LP = sqrt(a * b) - MINIMUM_LIQUIDITY
// Subsequent: LP proportional to pool share

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, MintTo, Token, TokenAccount, Transfer, mint_to, transfer},
};

use crate::{constants::*, errors::*, state::*};

#[derive(Accounts)]
pub struct DepositLiquidity<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,

    #[account(
        seeds = [
            AMM_CONFIG_SEED,
            pool_config.token_a_mint.as_ref(),
            pool_config.token_b_mint.as_ref(),
        ],
        bump = pool_config.config_bump,
    )]
    pub pool_config: Account<'info, PoolConfig>,

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
        mint::authority = pool_authority,
    )]
    pub lp_token_mint: Account<'info, Mint>,

    #[account(address = pool_config.token_a_mint)]
    pub token_a_mint: Account<'info, Mint>,

    #[account(address = pool_config.token_b_mint)]
    pub token_b_mint: Account<'info, Mint>,

    #[account(
        mut,
        token::mint = token_a_mint,
        token::authority = depositor,
    )]
    pub depositor_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_b_mint,
        token::authority = depositor,
    )]
    pub depositor_token_b: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = depositor,
        associated_token::mint = lp_token_mint,
        associated_token::authority = depositor,
    )]
    pub depositor_lp_token: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_a_mint,
        token::authority = pool_authority,
    )]
    pub token_a_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_b_mint,
        token::authority = pool_authority,
    )]
    pub token_b_vault: Account<'info, TokenAccount>,

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
        // Check pool not locked
        self.pool_config.assert_not_locked()?;

        // Validate expiration
        self.validate_expiration(expiration)?;

        // Check non-zero amounts
        require!(desired_amount_a > 0, AmmError::ZeroDepositAmount);
        require!(desired_amount_b > 0, AmmError::ZeroDepositAmount);

        let vault_a_balance = self.token_a_vault.amount;
        let vault_b_balance = self.token_b_vault.amount;
        let lp_supply = self.lp_token_mint.supply;

        // Calculate deposit amounts and LP tokens
        let (amount_a, amount_b, lp_tokens) = if lp_supply == 0 {
            self.calculate_first_deposit(desired_amount_a, desired_amount_b)?
        } else {
            self.calculate_subsequent_deposit(
                desired_amount_a,
                desired_amount_b,
                vault_a_balance,
                vault_b_balance,
                lp_supply,
            )?
        };

        // Slippage protection
        require!(amount_a <= max_amount_a, AmmError::ExcessiveDepositAmount);
        require!(amount_b <= max_amount_b, AmmError::ExcessiveDepositAmount);
        require!(lp_tokens > 0, AmmError::InsufficientLiquidity);

        // Transfer tokens to vaults
        self.transfer_token_a_to_vault(amount_a)?;
        self.transfer_token_b_to_vault(amount_b)?;

        // Mint LP tokens
        self.mint_lp_tokens(lp_tokens)?;

        msg!("Deposited: {} A, {} B -> {} LP", amount_a, amount_b, lp_tokens);

        Ok(())
    }

    fn validate_expiration(&self, expiration: i64) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;
        require!(expiration > current_time, AmmError::TransactionExpired);

        let time_until_expiration = expiration
            .checked_sub(current_time)
            .ok_or(AmmError::Underflow)?;

        require!(
            time_until_expiration <= MAX_EXPIRATION_SECONDS,
            AmmError::ExpirationTooFar
        );

        Ok(())
    }

    fn calculate_first_deposit(&self, amount_a: u64, amount_b: u64) -> Result<(u64, u64, u64)> {
        let product = (amount_a as u128)
            .checked_mul(amount_b as u128)
            .ok_or(AmmError::Overflow)?;

        // Better to use integer square root instead of f64
        // But keeping your original f64 version for now
        let liquidity = (product as f64).sqrt() as u64;

        require!(liquidity > MINIMUM_LIQUIDITY, AmmError::InsufficientLiquidity);

        let lp_tokens = liquidity
            .checked_sub(MINIMUM_LIQUIDITY)
            .ok_or(AmmError::Underflow)?;

        Ok((amount_a, amount_b, lp_tokens))
    }

    fn calculate_subsequent_deposit(
        &self,
        desired_a: u64,
        desired_b: u64,
        vault_a: u64,
        vault_b: u64,
        lp_supply: u64,
    ) -> Result<(u64, u64, u64)> {
        let lp_from_a = (desired_a as u128)
            .checked_mul(lp_supply as u128)
            .ok_or(AmmError::Overflow)?
            .checked_div(vault_a as u128)
            .ok_or(AmmError::DivisionByZero)?;

        let lp_from_b = (desired_b as u128)
            .checked_mul(lp_supply as u128)
            .ok_or(AmmError::Overflow)?
            .checked_div(vault_b as u128)
            .ok_or(AmmError::DivisionByZero)?;

        let lp_to_mint = std::cmp::min(lp_from_a, lp_from_b);

        let amount_a = (lp_to_mint)
            .checked_mul(vault_a as u128)
            .ok_or(AmmError::Overflow)?
            .checked_div(lp_supply as u128)
            .ok_or(AmmError::DivisionByZero)? as u64;

        let amount_b = (lp_to_mint)
            .checked_mul(vault_b as u128)
            .ok_or(AmmError::Overflow)?
            .checked_div(lp_supply as u128)
            .ok_or(AmmError::DivisionByZero)? as u64;

        Ok((amount_a, amount_b, lp_to_mint as u64))
    }

    fn transfer_token_a_to_vault(&self, amount: u64) -> Result<()> {
        transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.depositor_token_a.to_account_info(),
                    to: self.token_a_vault.to_account_info(),
                    authority: self.depositor.to_account_info(),
                },
            ),
            amount,
        )
    }

    fn transfer_token_b_to_vault(&self, amount: u64) -> Result<()> {
        transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.depositor_token_b.to_account_info(),
                    to: self.token_b_vault.to_account_info(),
                    authority: self.depositor.to_account_info(),
                },
            ),
            amount,
        )
    }

    fn mint_lp_tokens(&self, amount: u64) -> Result<()> {
        let pool_config_key = self.pool_config.key();
        let authority_seeds = &[
            AMM_AUTHORITY_SEED,
            pool_config_key.as_ref(),
            &[self.pool_config.authority_bump],
        ];
        let signer_seeds = &[&authority_seeds[..]];

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.lp_token_mint.to_account_info(),
                    to: self.depositor_lp_token.to_account_info(),
                    authority: self.pool_authority.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )
    }
}