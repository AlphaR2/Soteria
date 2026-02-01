use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{constants::*, errors::*, state::*};

// Initialize Treasury Instruction
//
// Creates the treasury state and associated token account for staking
// The treasury holds all staked tokens from users
//
// SECURITY FEATURES:
// - Treasury authority PDA signs all withdrawals (no private keys)
// - Token mint validated against config to prevent wrong token
// - Admin authorization required
// - System pause check prevents setup during maintenance

#[derive(Accounts)]
pub struct InitializeTreasury<'info> {
    // Signer paying for account creation
    #[account(mut)]
    pub signer: Signer<'info>,

    // Admin account
    /// CHECK: Used for PDA derivation and validation
    pub admin: UncheckedAccount<'info>,

    // Config PDA
    // Seeds: ["config", admin]
    // SECURITY: Validates admin authority and token mint
    #[account(
        seeds = [CONFIG, admin.key().as_ref()],
        bump,
        constraint = config.admin == admin.key() @ GovernanceError::UnauthorizedAdmin
    )]
    pub config: Account<'info, Config>,

    // Treasury state PDA
    // Seeds: ["treasury", admin]
    // Tracks total staked and staker count
    #[account(
        init,
        payer = signer,
        space = ANCHOR_DISCRIMINATOR + Treasury::INIT_SPACE,
        seeds = [TREASURY, admin.key().as_ref()],
        bump,
    )]
    pub treasury: Account<'info, Treasury>,

    // Treasury authority PDA
    // Seeds: ["treasury_auth", config, admin]
    // SECURITY: PDA signer for all treasury token transfers
    // No private key exists, only program can sign
    #[account(
        seeds = [TREASURYAUTH, config.key().as_ref(), admin.key().as_ref()],
        bump
    )]
    /// CHECK: PDA authority for treasury token account
    pub treasury_authority: UncheckedAccount<'info>,

    // Token mint account
    // SECURITY: Validates this matches config.token_mint
    // Prevents using wrong token for staking
    #[account(
        address = config.token_mint @ GovernanceError::InvalidTokenMint
    )]
    pub token_mint_account: Account<'info, Mint>,

    // Treasury token account (ATA)
    // Holds all staked tokens
    // Authority is treasury_authority PDA for security
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
        // SECURITY CHECKS

        // 1. System Pause Check
        // Prevents treasury initialization during system maintenance
        require!(!self.config.is_paused, GovernanceError::SystemPaused);

        // 2. Initialize Treasury State
        // Start with zero stakes and stakers
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