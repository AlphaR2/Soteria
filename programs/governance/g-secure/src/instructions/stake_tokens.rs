use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

use crate::{constants::*, errors::*, state::*};

// Stake Tokens Instruction
//
// Allows users to stake tokens to gain voting rights
// Staked tokens are held in the treasury until unstaked
//
// SECURITY FEATURES:
// - Minimum stake requirement prevents dust staking
// - Token mint validation prevents wrong token
// - User profile ownership validation
// - Checked arithmetic prevents overflow
// - System pause check
// - First-time staker tracking

#[derive(Accounts)]
pub struct Stake<'info> {
    // User staking tokens
    // Must own the profile and have sufficient balance
    #[account(mut)]
    pub user: Signer<'info>,

    // Admin pubkey for PDA derivation
    /// CHECK: Used for config and treasury PDA derivation
    pub admin: UncheckedAccount<'info>,

    // Config PDA
    // Seeds: ["config", admin]
    // SECURITY: Validates token mint and minimum stake
    #[account(
        seeds = [CONFIG, admin.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    // Treasury state PDA
    // Seeds: ["treasury", admin]
    // SECURITY: Tracks total staked and staker count
    #[account(
        mut,
        seeds = [TREASURY, admin.key().as_ref()],
        bump = treasury.state_bump
    )]
    pub treasury: Account<'info, Treasury>,

    // User profile PDA
    // Seeds: ["user_profile", user]
    // SECURITY: Validates ownership and updates stake amount
    #[account(
        mut,
        seeds = [USERPROFILE, user.key().as_ref()],
        bump,
        constraint = user_profile.owner == user.key() @ GovernanceError::UnauthorizedUser
    )]
    pub user_profile: Account<'info, UserProfile>,

    // Token mint account
    // SECURITY: Must match config.token_mint
    #[account(
        address = config.token_mint @ GovernanceError::InvalidTokenMint
    )]
    pub token_mint_account: Account<'info, Mint>,

    // User's token account (source)
    // SECURITY: Validated as user's ATA for correct mint
    #[account(
        mut,
        associated_token::mint = token_mint_account,
        associated_token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    // Treasury token account (destination)
    // SECURITY: Validated against treasury state
    #[account(
        mut,
        address = treasury.treasury_token_account @ GovernanceError::InvalidTreasuryAccount
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Stake<'info> {
    pub fn stake_tokens(&mut self, amount: u64) -> Result<()> {
        // SECURITY CHECKS

        // 1. Amount Validation
        // Prevents zero-value stakes
        require!(amount > 0, GovernanceError::InvalidStakeAmount);

        // 2. Minimum Stake Enforcement
        // SECURITY: Prevents sybil attacks with dust stakes
        require!(
            amount >= self.config.minimum_stake,
            GovernanceError::MinimumStakeRequired
        );

        // 3. System Pause Check
        // Prevents staking during maintenance
        require!(!self.config.is_paused, GovernanceError::SystemPaused);

        // 4. User Balance Check
        // Ensures user has sufficient tokens
        require!(
            self.user_token_account.amount >= amount,
            GovernanceError::InsufficientStake
        );

        // 5. Transfer Tokens to Treasury
        // User signs the transfer from their account to treasury
        let transfer_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_account.to_account_info(),
                to: self.treasury_token_account.to_account_info(),
                authority: self.user.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        // 6. Update User Profile
        // Track if this is the user's first stake
        let user_profile = &mut self.user_profile;
        let was_new_staker = user_profile.stake_amount == 0;

        // SECURITY: Checked addition prevents overflow
        user_profile.stake_amount = user_profile
            .stake_amount
            .checked_add(amount)
            .ok_or(GovernanceError::MathOverflow)?;

        // 7. Update Role Level
        // Role automatically updates based on reputation
        user_profile.role_level = MemberRanks::from_reputation(user_profile.reputation_points);

        // 8. Update Treasury Totals
        // SECURITY: Checked addition prevents overflow
        let treasury = &mut self.treasury;
        treasury.total_staked = treasury
            .total_staked
            .checked_add(amount)
            .ok_or(GovernanceError::MathOverflow)?;

        // 9. Increment Stakers Count
        // Only increment for first-time stakers
        if was_new_staker {
            treasury.stakers_count = treasury
                .stakers_count
                .checked_add(1)
                .ok_or(GovernanceError::MathOverflow)?;
        }

        Ok(())
    }
}