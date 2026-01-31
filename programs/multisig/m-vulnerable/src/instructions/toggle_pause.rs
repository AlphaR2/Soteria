use anchor_lang::prelude::*;
use crate::{state::*, constants::*};

// Toggle Pause Instruction - VULNERABLE VERSION
//
// Allows pausing/unpausing the multisig with critical vulnerabilities.

#[derive(Accounts)]
pub struct TogglePause<'info> {
    // VULNERABILITY [CRITICAL]: No admin check on caller
    //
    // The secure version validates: multisig.is_admin(&admin.key())
    // Only the creator (admin) should be able to pause/unpause.
    //
    // Without this check, ANY signer can toggle the pause state,
    // enabling denial of service attacks.
    //
    // Example Attack (DoS by Pausing):
    //   1. Multisig is operating normally
    //   2. Attacker calls toggle_pause (no admin check)
    //   3. Multisig is now paused
    //   4. IF pause checks are implemented: all operations blocked
    //   5. Admin tries to unpause, but attacker immediately re-pauses
    //   6. Multisig is permanently DoS'd
    //
    // Example Attack (Unpause During Emergency):
    //   1. Admin detects security breach, pauses multisig
    //   2. Attacker (the security threat) calls toggle_pause
    //   3. Multisig is unpaused
    //   4. Attacker continues malicious activity
    //   5. Emergency pause mechanism is useless
    #[account(mut)]
    pub admin: Signer<'info>,

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
        // VULNERABILITY [CRITICAL]: Missing admin check
        //
        // The secure version validates:
        // require!(self.multisig_account.is_admin(&self.admin.key()), MultisigError::OnlyAdmin);
        //
        // Without this, the "emergency brake" can be toggled by anyone,
        // making it either:
        // - A DoS vector (anyone can pause)
        // - Useless (attacker can unpause during emergencies)
        //
        // This is one of the most critical vulnerabilities because it
        // undermines the entire emergency response mechanism.
        //
        // Fix: require!(self.multisig_account.is_admin(&self.admin.key()), MultisigError::OnlyAdmin);


        // VULNERABILITY [MEDIUM]: No event emission for audit trail
        //
        // The secure version should emit an event when pause state changes.
        // Without events, there's no audit trail of who paused/unpaused
        // and when, making incident investigation difficult.
        //
        // Fix: Emit PauseToggled event with admin pubkey and new state.


        // Toggle the pause state
        self.multisig_account.paused = !self.multisig_account.paused;

        Ok(())
    }
}
