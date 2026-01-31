use crate::*;
use crate::errors::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};
use anchor_spl::associated_token::AssociatedToken;

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: The admin pubkey will be passed here
    pub admin: UncheckedAccount<'info>,

    // Config account
	#[account(
        seeds = [CONFIG, admin.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    // Treasury state
    #[account(
        mut,
        seeds = [TREASURY, admin.key().as_ref()],
         bump = treasury.state_bump
    )]
    pub treasury: Account<'info, Treasury>,

    // Treasury authority PDA for signing transfers
    #[account(
        seeds = [TREASURYAUTH, config.key().as_ref(), admin.key().as_ref()],
        bump = treasury.vault_bump,
    )]
    /// CHECK: This is for signing auth
    pub treasury_authority: UncheckedAccount<'info>,

    // User's profile to update stake amount
    #[account(
        mut,
        seeds = [USERPROFILE, user.key().as_ref()],
        bump,
        constraint = user_profile.owner == user.key() @ GovernanceError::UnauthorizedUser
    )]
    pub user_profile: Account<'info, UserProfile>,

    // Token mint for validation
    #[account(
        address = config.token_mint @ GovernanceError::InvalidTokenMint
    )]
    pub token_mint_account: Account<'info, Mint>,

    // User token account (destination)
    #[account(
        mut,
        associated_token::mint = token_mint_account,
        associated_token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    // Treasury token account (source)
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
        // Validation
        require!(amount > 0, GovernanceError::InvalidStakeAmount);
        require!(!self.config.is_paused, GovernanceError::SystemPaused);
        
        let user_profile = &mut self.user_profile;
        
        // Check user has enough staked
        require!(
            user_profile.stake_amount >= amount,
            GovernanceError::InsufficientStake
        );

        // Check treasury has enough tokens
        require!(
            self.treasury_token_account.amount >= amount,
            GovernanceError::InsufficientTreasuryBalance
        );

        let config = self.config.key();
        let admin = self.admin.key();

        // Calculate new stake amount
        let new_stake_amount = user_profile.stake_amount
            .checked_sub(amount)
            .ok_or(GovernanceError::MathOverflow)?;

        // Transfer tokens from treasury to user using PDA authority
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

        // Update user profile
        let was_staker = user_profile.stake_amount > 0;
        user_profile.stake_amount = new_stake_amount;

        // Update role based on new reputation (stake doesn't affect role directly)
        user_profile.role_level = MemberRanks::from_reputation(user_profile.reputation_points);

        // Update treasury
        let treasury = &mut self.treasury;
        treasury.total_staked = treasury.total_staked
            .checked_sub(amount)
            .ok_or(GovernanceError::MathOverflow)?;

        // Decrement stakers count if user unstaked everything
        if was_staker && new_stake_amount == 0 {
            treasury.stakers_count = treasury.stakers_count
                .checked_sub(1)
                .ok_or(GovernanceError::MathOverflow)?;
        }

        Ok(())
    }
}