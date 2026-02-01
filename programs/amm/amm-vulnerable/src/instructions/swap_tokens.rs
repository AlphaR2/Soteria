// Swap Tokens Instruction - VULNERABLE VERSION
//
// WARNING: This version contains intentional vulnerabilities for educational purposes.
//
// VULNERABILITIES:
// V009: No swap slippage protection enforcement - swappers can be front-run
// V003: No expiration validation - stale transactions can execute
// V007: No pool lock enforcement - swaps work even when pool is locked
// V010: No zero amount checks - wastes gas
// V011: No liquidity checks before operations - may fail ungracefully

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount, Transfer, transfer},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

use crate::{constants::*, errors::*, state::*, helpers::*};

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
        // VULNERABILITY V007: No pool lock enforcement
        // Secure version: self.pool_config.assert_not_locked()?;
        // Attack: Pool has critical bug, admin locks it, but swaps continue anyway

        // VULNERABILITY V003: No expiration validation
        // Secure version: self.validate_expiration(expiration)?;
        // Using helper instead (which does nothing in vulnerable version)
        validate_expiration(expiration)?;
        // Attack: User submits swap at 1.0 exchange rate, transaction pending for hours,
        // executes at 0.5 exchange rate after massive market movement

        // VULNERABILITY V010: No zero amount checks
        // Secure version: require!(input_amount > 0, AmmError::ZeroSwapAmount);
        // Secure version: require!(min_output_amount > 0, AmmError::SlippageExceeded);
        // Impact: Wastes gas processing 0-amount swaps

        let vault_a_balance = self.token_a_vault.amount;
        let vault_b_balance = self.token_b_vault.amount;

        // VULNERABILITY V011: No liquidity checks before operations
        // Secure version: require!(vault_a_balance > 0, AmmError::InsufficientPoolLiquidity);
        // Secure version: require!(vault_b_balance > 0, AmmError::InsufficientPoolLiquidity);
        // Impact: Operations may fail ungracefully if pool is empty

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

        // VULNERABILITY V009: No minimum output enforcement
        // Secure version validates: require!(swap_result.withdraw >= min_output_amount, ...)
        // The curve.swap() receives min_output_amount parameter but vulnerable version
        // doesn't verify the result meets minimum requirements
        // Attack scenario (front-running):
        // 1. Victim submits swap: 1000 A -> min 950 B
        // 2. Front-runner sees pending transaction
        // 3. Front-runner swaps large amount A -> B, moving price against victim
        // 4. Victim's swap executes at terrible price, gets only 500 B (should fail but doesn't)
        // 5. Back-runner swaps B -> A, profiting from price movement

        // Validate swap result (partial checks only)
        require!(swap_result.deposit > 0, AmmError::InvalidCurveParams);
        require!(swap_result.withdraw > 0, AmmError::InvalidCurveParams);
        // Missing: slippage protection check

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

    // VULNERABILITY V003: Expiration validation removed
    // This function existed in secure version but is replaced by helper that does nothing
    // fn validate_expiration(&self, expiration: i64) -> Result<()> {
    //     let current_time = Clock::get()?.unix_timestamp;
    //     require!(expiration > current_time, AmmError::TransactionExpired);
    //     let time_until_expiration = expiration.checked_sub(current_time)?;
    //     require!(time_until_expiration <= MAX_EXPIRATION_SECONDS, ...);
    //     Ok(())
    // }

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