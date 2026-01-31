use anchor_lang::prelude::*;
use crate::{state::*, errors::*, constants::*};

// Toggle Pause Instruction
//
// Allows the admin (creator) to pause/unpause the multisig
// When paused, all operations are blocked except unpause
// Emergency brake for security incidents

#[derive(Accounts)]
pub struct TogglePause<'info> {
    // Admin - must be the creator
    #[account(mut)]
    pub admin: Signer<'info>,

    // Multisig account to pause/unpause
    #[account(
        mut,
        seeds = [
            MULTISIG,
            multisig_account.creator.as_ref(),
            &multisig_account.multisig_id.to_le_bytes(),
        ],
        bump = multisig_account.bump,
    )]
    pub multisig_account: Account<'info, Multisig>,
}

impl<'info> TogglePause<'info> {
    pub fn toggle_pause(&mut self) -> Result<()> {
        // SECURITY CHECKS

        // 1. Admin Check
        // Only the creator (admin) can pause/unpause
        require!(
            self.multisig_account.is_admin(&self.admin.key()),
            MultisigError::OnlyAdmin
        );

        // Toggle pause state
        self.multisig_account.paused = !self.multisig_account.paused;

        Ok(())
    }
}
