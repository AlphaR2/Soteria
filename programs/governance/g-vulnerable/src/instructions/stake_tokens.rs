use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

use crate::{constants::*, errors::*, state::*};

// Stake Tokens Instruction
//
// VULNERABILITY SUMMARY:
// - No minimum stake requirement enforcement
// - Missing owner validation on user_profile
// - No token mint validation
// - Unchecked arithmetic operations (overflow risk)
// - No system pause check
// - Missing balance verification before transfer

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: Used for config and treasury PDA derivation
    pub admin: UncheckedAccount<'info>,

    #[account(
        seeds = [CONFIG, admin.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [TREASURY, admin.key().as_ref()],
        bump = treasury.state_bump
    )]
    pub treasury: Account<'info, Treasury>,

    // VULNERABILITY: No owner constraint
    // User can stake to someone else's profile
    #[account(
        mut,
        seeds = [USERPROFILE, user.key().as_ref()],
        bump,
    )]
    pub user_profile: Account<'info, UserProfile>,

    // VULNERABILITY: No mint validation
    // Could accept wrong token mint
    pub token_mint_account: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = token_mint_account,
        associated_token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,

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
        // VULNERABILITY 1: No minimum stake check
        // The config.minimum_stake exists but is not enforced
        // Users can stake tiny amounts (1 lamport) to gain voting rights
        // Enables sybil attacks with minimal capital

        // VULNERABILITY 2: No system pause check
        // Missing: require!(!self.config.is_paused, ...)
        // System can't be halted during emergencies

        // VULNERABILITY 3: No balance verification
        // Transfer might fail but no explicit check beforehand
        // Poor user experience and wasted compute

        let transfer_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_token_account.to_account_info(),
                to: self.treasury_token_account.to_account_info(),
                authority: self.user.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        let user_profile = &mut self.user_profile;
        let was_new_staker = user_profile.stake_amount == 0;

        // VULNERABILITY 4: Unchecked arithmetic
        // Using direct addition instead of checked_add
        // If stake_amount + amount > u64::MAX, program panics
        user_profile.stake_amount = user_profile.stake_amount + amount;

        user_profile.role_level = MemberRanks::from_reputation(user_profile.reputation_points);

        // VULNERABILITY 5: Unchecked arithmetic on treasury
        let treasury = &mut self.treasury;
        treasury.total_staked = treasury.total_staked + amount;

        if was_new_staker {
            treasury.stakers_count = treasury.stakers_count + 1;
        }

        Ok(())
    }
}
