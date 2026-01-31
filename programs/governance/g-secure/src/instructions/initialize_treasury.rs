use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use anchor_spl::associated_token::AssociatedToken;
use crate::state::*;
use crate::constants::*;
use crate::errors::*;

#[derive(Accounts)]
pub struct InitializeTreasury<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK: Admin account, zero panic
    pub admin: UncheckedAccount<'info>,
    /// Verify config exists and admin matches
    #[account(
        seeds = [CONFIG, admin.key().as_ref()],
        bump,
        constraint = config.admin == admin.key() @ GovernanceError::UnauthorizedAdmin
    )]
    pub config: Account<'info, Config>,

    /// Initialize treasury state
    #[account(
        init,
        payer = signer,
        space = ANCHOR_DISCRIMINATOR + Treasury::INIT_SPACE,
        seeds = [TREASURY, admin.key().as_ref()],
        bump,
    )]
    pub treasury: Account<'info, Treasury>,

    /// Treasury authority PDA for signing token transfers from treasury
    #[account(
        seeds = [TREASURYAUTH, config.key().as_ref(), admin.key().as_ref()],
        bump
    )]
    /// CHECK: This is for signing auth
    pub treasury_authority: UncheckedAccount<'info>,

	// token mint
	#[account(
        address = config.token_mint @ GovernanceError::InvalidTokenMint
    )]
    pub token_mint_account: Account<'info, Mint>, 

    /// Treasury token account to hold staked tokens
    #[account(
        init,
        payer = signer,
        associated_token::mint = token_mint_account,
        associated_token::authority = treasury_authority,
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeTreasury<'info> {
    pub fn initialize_treasury(
        &mut self,
        bumps: InitializeTreasuryBumps,
    ) -> Result<()> {
        // Validation
        require!(
            !self.config.is_paused,
            GovernanceError::SystemPaused
        );

        // Initialize treasury
        self.treasury.set_inner(Treasury {
            admin: self.admin.key(),
            total_staked: 0,
            stakers_count: 0,
            treasury_token_account: self.treasury_token_account.key(),
			state_bump: bumps.treasury,
            vault_bump: bumps.treasury_authority,
        });

        Ok(())
    }
}