use anchor_lang::prelude::*;

use crate::{constants::*, errors::*, state::*};

// Reset User Reputation Instruction
//
// Admin-only operation to reset a user's reputation and voting stats
// Useful for moderation or seasonal resets
//
// SECURITY FEATURES:
// - Admin-only access (validated via config PDA)
// - Does not affect user's stake amount
// - Resets role to Member
// - System pause check
// - Logs the reset action for auditability

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct ResetUserReputation<'info> {
    // Admin account
    // Must be the configured admin
    #[account(mut)]
    pub admin: Signer<'info>,

    // Config PDA
    // Seeds: ["config", admin]
    // SECURITY: Validates admin authority via has_one constraint
    // Only the designated admin can perform this action
    #[account(
        seeds = [CONFIG, admin.key().as_ref()],
        bump,
        has_one = admin @ GovernanceError::UnauthorizedAdmin
    )]
    pub config: Account<'info, Config>,

    // User profile to reset
    // Seeds: ["user_profile", user]
    // SECURITY: Admin can reset any user's reputation
    #[account(
        mut,
        seeds = [USERPROFILE, user.as_ref()],
        bump
    )]
    pub user_profile: Account<'info, UserProfile>,
}

impl<'info> ResetUserReputation<'info> {
    pub fn reset_user_reputation(&mut self) -> Result<()> {
        // SECURITY CHECKS

        // 1. System Pause Check
        // Prevents reputation resets during maintenance
        require!(!self.config.is_paused, GovernanceError::SystemPaused);

        let user_profile = &mut self.user_profile;

        // 2. Reset Reputation Fields
        // SECURITY: Only resets reputation-related data
        // Does NOT reset:
        // - Username (permanent identity)
        // - Owner (cannot change ownership)
        // - Stake amount (user keeps their staked tokens)
        // - Total votes cast (historical data preserved)
        // - Last vote timestamp (for cooldown tracking)
        // - Created at (account age preserved)
        user_profile.reputation_points = 0;
        user_profile.upvotes_received = 0;
        user_profile.downvotes_received = 0;
        user_profile.role_level = MemberRanks::Member;

        // 3. Log Reset Action
        // Provides audit trail for admin actions
        msg!(
            "Reset reputation for user: {} ({})",
            user_profile.username,
            user_profile.owner
        );

        Ok(())
    }
}