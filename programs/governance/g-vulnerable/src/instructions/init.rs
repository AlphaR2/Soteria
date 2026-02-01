use anchor_lang::prelude::*;

use crate::{constants::*, state::*};

// Initialize DAO Instruction
//
// Creates the governance configuration account with core parameters
// This is a one-time initialization that sets up the DAO rules
//
// SECURITY FEATURES:
// - Admin passed as parameter (not derived from signer) for flexibility
// - Config PDA prevents unauthorized modification
// - Minimum stake requirement set at initialization
// - Vote power multiplier configurable
// - System starts unpaused by default

#[derive(Accounts)]
#[instruction(admin: Pubkey)]
pub struct InitializeDaoProgram<'info> {
    // Signer paying for account creation
    // Does not need to be the admin
    #[account(mut)]
    pub signer: Signer<'info>,

    // Config PDA
    // Seeds: ["config", admin]
    // SECURITY: PDA derivation ties config to specific admin
    // Only this admin can perform admin-only operations
    #[account(
        init,
        payer = signer,
        space = ANCHOR_DISCRIMINATOR + Config::INIT_SPACE,
        seeds = [CONFIG, admin.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    pub system_program: Program<'info, System>,
}

impl<'info> InitializeDaoProgram<'info> {
    pub fn initialize(
        &mut self,
        minimum_stake: u64,
        admin: Pubkey,
        token_mint: Pubkey,
        vote_power: u8,
        bumps: InitializeDaoProgramBumps,
    ) -> Result<()> {
        // SECURITY: Admin passed as parameter instead of using signer
        // This allows flexibility in who initializes vs who controls the DAO
        // The admin derives the config PDA and has special privileges

        self.config.set_inner(Config {
            admin: admin.key(),
            minimum_stake,
            token_mint,
            vote_power,
            is_paused: false,
            config_bump: bumps.config,
        });

        Ok(())
    }
}