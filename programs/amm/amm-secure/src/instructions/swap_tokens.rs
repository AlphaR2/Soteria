// Swap Tokens Instruction
//
// Swaps tokens using constant product formula (x * y = k).
// Fee is deducted from input before calculating output.

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount, Transfer, transfer},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

use crate::{constants::*, errors::*, state::*};

#[derive(Accounts)]
pub struct SwapTokens<'info> {
    #[account(mut)]
    pub swapper: Signer<'info>,

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

    #[account(address = pool_config.token_a_mint)]
    pub token_a_mint: Box<Account<'info, Mint>>,

    #[account(address = pool_config.token_b_mint)]
    pub token_b_mint: Box<Account<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = swapper,
        associated_token::mint = token_a_mint,
        associated_token::authority = swapper,
    )]
    pub swapper_token_a: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = swapper,
        associated_token::mint = token_b_mint,
        associated_token::authority = swapper,
    )]
    pub swapper_token_b: Box<Account<'info, TokenAccount>>,

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

impl<'info> SwapTokens<'info> {
    pub fn swap_tokens(
        &mut self,
        swap_token_a_for_b: bool,
        input_amount: u64,
        min_output_amount: u64,
        expiration: i64,
    ) -> Result<()> {
        // Check pool not locked
        self.pool_config.assert_not_locked()?;

        // Validate expiration
        self.validate_expiration(expiration)?;

        // Check non-zero amounts
        require!(input_amount > 0, AmmError::ZeroSwapAmount);
        require!(min_output_amount > 0, AmmError::SlippageExceeded);

        let vault_a_balance = self.token_a_vault.amount;
        let vault_b_balance = self.token_b_vault.amount;

        // Check pool has liquidity
        require!(vault_a_balance > 0, AmmError::InsufficientPoolLiquidity);
        require!(vault_b_balance > 0, AmmError::InsufficientPoolLiquidity);

        // Initialize constant product curve
        let mut curve = ConstantProduct::init(
            vault_a_balance,
            vault_b_balance,
            vault_a_balance,
            self.pool_config.fee_basis_points,
            None,
        )
        .map_err(|_| AmmError::CurveCalculationFailed)?;

        // Determine swap direction
        let swap_direction = if swap_token_a_for_b {
            LiquidityPair::X
        } else {
            LiquidityPair::Y
        };

        // Calculate swap
        let swap_result = curve
            .swap(swap_direction, input_amount, min_output_amount)
            .map_err(|_| AmmError::CurveCalculationFailed)?;

        // Validate swap result
        require!(swap_result.deposit > 0, AmmError::InvalidCurveParams);
        require!(swap_result.withdraw > 0, AmmError::InvalidCurveParams);
        require!(swap_result.withdraw >= min_output_amount, AmmError::SlippageExceeded);

        // Check pool has enough output tokens
        let output_vault_balance = if swap_token_a_for_b {
            vault_b_balance
        } else {
            vault_a_balance
        };
        require!(
            swap_result.withdraw <= output_vault_balance,
            AmmError::InsufficientPoolLiquidity
        );

        // Perform swap transfers
        if swap_token_a_for_b {
            self.deposit_token_a(swap_result.deposit)?;
            self.withdraw_token_b(swap_result.withdraw)?;
            msg!("Swapped {} A -> {} B", swap_result.deposit, swap_result.withdraw);
        } else {
            self.deposit_token_b(swap_result.deposit)?;
            self.withdraw_token_a(swap_result.withdraw)?;
            msg!("Swapped {} B -> {} A", swap_result.deposit, swap_result.withdraw);
        }

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

    fn deposit_token_a(&self, amount: u64) -> Result<()> {
        transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.swapper_token_a.to_account_info(),
                    to: self.token_a_vault.to_account_info(),
                    authority: self.swapper.to_account_info(),
                },
            ),
            amount,
        )
    }

    fn deposit_token_b(&self, amount: u64) -> Result<()> {
        transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from: self.swapper_token_b.to_account_info(),
                    to: self.token_b_vault.to_account_info(),
                    authority: self.swapper.to_account_info(),
                },
            ),
            amount,
        )
    }

    fn withdraw_token_a(&self, amount: u64) -> Result<()> {
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
                    to: self.swapper_token_a.to_account_info(),
                    authority: self.pool_authority.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )
    }

    fn withdraw_token_b(&self, amount: u64) -> Result<()> {
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
                    to: self.swapper_token_b.to_account_info(),
                    authority: self.pool_authority.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )
    }
}