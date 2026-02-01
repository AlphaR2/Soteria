use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

use crate::{constants::*, errors::*, state::*};

// Unstake Tokens Instruction
//
// Allows users to withdraw their staked tokens from the treasury
// Uses PDA authority to sign the transfer from treasury to user
//
// SECURITY FEATURES:
// - Treasury PDA authority signs withdrawals (no private keys)
// - Sufficient balance checks (user profile and treasury)
// - Token mint validation
// - Checked arithmetic prevents underflow
// - System pause check
// - Staker count tracking

#[derive(Accounts)]
pub struct Unstake<'info> {
    // User unstaking tokens
    // Must have sufficient staked balance
    #[account(mut)]
    pub user: Signer<'info>,

    // Admin pubkey for PDA derivation
    /// CHECK: Used for config and treasury PDA derivation
    pub admin: UncheckedAccount<'info>,

    // Config PDA
    // Seeds: ["config", admin]
    // SECURITY: Validates token mint
    #[account(
        seeds = [CONFIG, admin.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    // Treasury state PDA
    // Seeds: ["treasury", admin]
    // SECURITY: Tracks total staked and validates treasury balance
    #[account(
        mut,
        seeds = [TREASURY, admin.key().as_ref()],
        bump = treasury.state_bump
    )]
    pub treasury: Account<'info, Treasury>,

    // Treasury authority PDA
    // Seeds: ["treasury_auth", config, admin]
    // SECURITY: PDA signer for treasury withdrawals
    // Only the program can sign, no private key exists
    #[account(
        seeds = [TREASURYAUTH, config.key().as_ref(), admin.key().as_ref()],
        bump = treasury.vault_bump,
    )]
    /// CHECK: PDA authority for signing treasury transfers
    pub treasury_authority: UncheckedAccount<'info>,

    // User profile PDA
    // Seeds: ["user_profile", user]
    // SECURITY: Validates ownership and tracks stake amount
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

    // User's token account (destination)
    // SECURITY: Validated as user's ATA for correct mint
    #[account(
        mut,
        associated_token::mint = token_mint_account,
        associated_token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    // Treasury token account (source)
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

impl<'info> Unstake<'info> {
    pub fn unstake_tokens(&mut self, amount: u64) -> Result<()> {
        // SECURITY CHECKS

        // 1. Amount Validation
        // Prevents zero-value unstakes
        require!(amount > 0, GovernanceError::InvalidStakeAmount);

        // 2. System Pause Check
        // Prevents unstaking during maintenance
        require!(!self.config.is_paused, GovernanceError::SystemPaused);

        let user_profile = &mut self.user_profile;

        // 3. User Stake Balance Check
        // SECURITY: Ensures user has enough staked tokens
        require!(
            user_profile.stake_amount >= amount,
            GovernanceError::InsufficientStake
        );

        // 4. Treasury Balance Check
        // SECURITY: Ensures treasury has sufficient tokens
        // Prevents withdrawal if treasury is drained
        require!(
            self.treasury_token_account.amount >= amount,
            GovernanceError::InsufficientTreasuryBalance
        );

        let config = self.config.key();
        let admin = self.admin.key();

        // 5. Calculate New Stake Amount
        // SECURITY: Checked subtraction prevents underflow
        let new_stake_amount = user_profile
            .stake_amount
            .checked_sub(amount)
            .ok_or(GovernanceError::MathOverflow)?;

        // 6. Transfer Tokens from Treasury to User
        // SECURITY: Uses PDA authority to sign the transfer
        // Treasury authority PDA has no private key, only program can sign
        let treasury_auth_seeds = &[
            TREASURYAUTH,
            config.as_ref(),
            admin.as_ref(),
            &[self.treasury.vault_bump],
        ];
        let signer_seeds = &[&treasury_auth_seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            Transfer {
                from: self.treasury_token_account.to_account_info(),
                to: self.user_token_account.to_account_info(),
                authority: self.treasury_authority.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, amount)?;

        // 7. Update User Profile
        // Track if user had stake before (for staker count)
        let was_staker = user_profile.stake_amount > 0;
        user_profile.stake_amount = new_stake_amount;

        // 8. Update Role Level
        // Role automatically updates based on reputation
        // Unstaking does not directly affect role
        user_profile.role_level = MemberRanks::from_reputation(user_profile.reputation_points);

        // 9. Update Treasury Totals
        // SECURITY: Checked subtraction prevents underflow
        let treasury = &mut self.treasury;
        treasury.total_staked = treasury
            .total_staked
            .checked_sub(amount)
            .ok_or(GovernanceError::MathOverflow)?;

        // 10. Decrement Stakers Count
        // Only decrement if user unstaked everything
        if was_staker && new_stake_amount == 0 {
            treasury.stakers_count = treasury
                .stakers_count
                .checked_sub(1)
                .ok_or(GovernanceError::MathOverflow)?;
        }

        Ok(())
    }
}