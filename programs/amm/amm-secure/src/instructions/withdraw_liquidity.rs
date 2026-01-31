// Withdraw Liquidity Instruction
//
// Burns LP tokens and returns proportional share of pool tokens.

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount, Burn, Transfer, burn, transfer},
};

use crate::{constants::*, errors::*, state::*};

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
    )]
    pub lp_token_mint: Account<'info, Mint>,

    #[account(address = pool_config.token_a_mint)]
    pub token_a_mint: Account<'info, Mint>,

    #[account(address = pool_config.token_b_mint)]
    pub token_b_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = withdrawer,
        associated_token::mint = token_a_mint,
        associated_token::authority = withdrawer,
    )]
    pub withdrawer_token_a: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = withdrawer,
        associated_token::mint = token_b_mint,
        associated_token::authority = withdrawer,
    )]
    pub withdrawer_token_b: Account<'info, TokenAccount>,

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

impl<'info> WithdrawLiquidity<'info> {
    pub fn withdraw_liquidity(
        &mut self,
        lp_tokens_to_burn: u64,
        min_amount_a: u64,
        min_amount_b: u64,
        expiration: i64,
    ) -> Result<()> {
        // Check pool not locked
        self.pool_config.assert_not_locked()?;

        // Validate expiration
        self.validate_expiration(expiration)?;

        // Check non-zero LP amount
        require!(lp_tokens_to_burn > 0, AmmError::ZeroWithdrawAmount);

        let vault_a_balance = self.token_a_vault.amount;
        let vault_b_balance = self.token_b_vault.amount;
        let lp_supply = self.lp_token_mint.supply;

        // Check pool has liquidity
        require!(lp_supply > 0, AmmError::InsufficientLiquidity);
        require!(vault_a_balance > 0, AmmError::InsufficientLiquidity);
        require!(vault_b_balance > 0, AmmError::InsufficientLiquidity);

        // Calculate withdrawal amounts
        let amount_a = (lp_tokens_to_burn as u128)
            .checked_mul(vault_a_balance as u128)
            .ok_or(AmmError::Overflow)?
            .checked_div(lp_supply as u128)
            .ok_or(AmmError::DivisionByZero)? as u64;

        let amount_b = (lp_tokens_to_burn as u128)
            .checked_mul(vault_b_balance as u128)
            .ok_or(AmmError::Overflow)?
            .checked_div(lp_supply as u128)
            .ok_or(AmmError::DivisionByZero)? as u64;

        // Slippage protection
        require!(amount_a >= min_amount_a, AmmError::InsufficientWithdrawAmount);
        require!(amount_b >= min_amount_b, AmmError::InsufficientWithdrawAmount);

        // Check non-zero withdrawals
        require!(amount_a > 0, AmmError::InsufficientLiquidity);
        require!(amount_b > 0, AmmError::InsufficientLiquidity);

        // Check vault balances
        require!(vault_a_balance >= amount_a, AmmError::InsufficientPoolLiquidity);
        require!(vault_b_balance >= amount_b, AmmError::InsufficientPoolLiquidity);

        // Burn LP tokens
        self.burn_lp_tokens(lp_tokens_to_burn)?;

        // Transfer tokens from vaults
        self.transfer_token_a_from_vault(amount_a)?;
        self.transfer_token_b_from_vault(amount_b)?;

        msg!("Withdrawn: {} LP -> {} A, {} B", lp_tokens_to_burn, amount_a, amount_b);

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

    fn burn_lp_tokens(&self, amount: u64) -> Result<()> {
        burn(
            CpiContext::new(
                self.token_program.to_account_info(),
                Burn {
                    mint: self.lp_token_mint.to_account_info(),
                    from: self.withdrawer_lp_token.to_account_info(),
                    authority: self.withdrawer.to_account_info(),
                },
            ),
            amount,
        )
    }

    fn transfer_token_a_from_vault(&self, amount: u64) -> Result<()> {
        let pool_config_key = self.pool_config.key();
        let authority_seeds = &[
            AMM_AUTHORITY_SEED,
            pool_config_key.as_ref(),
            &[self.pool_config.authority_bump],
        ];
        let signer_seeds = &[&authority_seeds[..]];

        transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.token_a_vault.to_account_info(),
                    to: self.withdrawer_token_a.to_account_info(),
                    authority: self.pool_authority.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )
    }

    fn transfer_token_b_from_vault(&self, amount: u64) -> Result<()> {
        let pool_config_key = self.pool_config.key();
        let authority_seeds = &[
            AMM_AUTHORITY_SEED,
            pool_config_key.as_ref(),
            &[self.pool_config.authority_bump],
        ];
        let signer_seeds = &[&authority_seeds[..]];

        transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.token_b_vault.to_account_info(),
                    to: self.withdrawer_token_b.to_account_info(),
                    authority: self.pool_authority.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )
    }
}