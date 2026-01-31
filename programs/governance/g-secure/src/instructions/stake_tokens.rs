use crate::*;
use crate::errors::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};
use anchor_spl::associated_token::AssociatedToken;

#[derive(Accounts)]
pub struct Stake<'info> {
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

    // User token account (source of tokens)
    #[account(
        mut,
        associated_token::mint = token_mint_account,
        associated_token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    // Treasury token account (destination)
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
        // Validation
        require!(amount > 0, GovernanceError::InvalidStakeAmount);
        require!(amount >= self.config.minimum_stake, GovernanceError::MinimumStakeRequired);
        require!(!self.config.is_paused, GovernanceError::SystemPaused);
        require!(
            self.user_token_account.amount >= amount,
            GovernanceError::InsufficientStake
        );

        // Transfer tokens from user to treasury
        let transfer_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_account.to_account_info(),
                to: self.treasury_token_account.to_account_info(),
                authority: self.user.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        // Update user profile
        let user_profile = &mut self.user_profile;
        let was_new_staker = user_profile.stake_amount == 0;
        
        user_profile.stake_amount = user_profile.stake_amount
            .checked_add(amount)
            .ok_or(GovernanceError::MathOverflow)?;

        // Update role based on new reputation + stake
        user_profile.role_level = MemberRanks::from_reputation(user_profile.reputation_points);

        // Update treasury
        let treasury = &mut self.treasury;
        treasury.total_staked = treasury.total_staked
            .checked_add(amount)
            .ok_or(GovernanceError::MathOverflow)?;

        // Increment stakers count if new staker
        if was_new_staker {
            treasury.stakers_count = treasury.stakers_count
                .checked_add(1)
                .ok_or(GovernanceError::MathOverflow)?;
        }

        Ok(())
    }
}