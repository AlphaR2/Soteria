// Initialize Pool Instruction
//
// Creates a new AMM liquidity pool for a token pair.

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{constants::*, errors::*, state::*};

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_a_mint: Box<Account<'info, Mint>>,
    pub token_b_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = authority,
        space = ANCHOR_DISCRIMINATOR + PoolConfig::INIT_SPACE,
        seeds = [
            AMM_CONFIG_SEED,
            token_a_mint.key().as_ref(),
            token_b_mint.key().as_ref(),
        ],
        bump
    )]
    pub pool_config: Box<Account<'info, PoolConfig>>,

    /// CHECK: PDA signer for vault operations
    #[account(
        seeds = [AMM_AUTHORITY_SEED, pool_config.key().as_ref()],
        bump
    )]
    pub pool_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = authority,
        seeds = [LP_MINT_SEED, pool_config.key().as_ref()],
        bump,
        mint::decimals = 9,
        mint::authority = pool_authority,
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = token_a_mint,
        associated_token::authority = pool_authority,
    )]
    pub token_a_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = token_b_mint,
        associated_token::authority = pool_authority,
    )]
    pub token_b_vault: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializePool<'info> {
    pub fn initialize_pool(
        &mut self,
        fee_basis_points: u16,
        bumps: &InitializePoolBumps,
    ) -> Result<()> {
        // VULNERABILITY 1: No fee validation
        // Secure version checks: require!(fee_basis_points <= MAX_FEE_BASIS_POINTS)
        // Vulnerable version: Accepts any fee up to u16::MAX (655.35%!)
        // Attack: Pool creator sets 50000 basis points (500% fee) to steal from swappers

        // VULNERABILITY 2: No identical mint check
        // Secure version checks: require!(token_a_mint != token_b_mint)
        // Vulnerable version: Allows creating SOL/SOL or USDC/USDC pools
        // Attack: Confuse users with nonsense pools

        // Initialize pool configuration
        self.pool_config.set_inner(PoolConfig {
            authority: self.authority.key(),
            token_a_mint: self.token_a_mint.key(),
            token_b_mint: self.token_b_mint.key(),
            lp_token_mint: self.lp_token_mint.key(),
            fee_basis_points,
            locked: false,
            config_bump: bumps.pool_config,
            authority_bump: bumps.pool_authority,
            lp_mint_bump: bumps.lp_token_mint,
        });

        msg!(
            "Pool initialized: {} / {}",
            self.token_a_mint.key(),
            self.token_b_mint.key()
        );
        msg!("Fee: {} basis points", fee_basis_points);

        Ok(())
    }
}
