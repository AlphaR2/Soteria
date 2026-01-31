use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};
use crate::{state::*, errors::*, constants::*};

// Create Multisig Instruction
//
// Initializes a new multisig wallet with:
// - Creator as first owner
// - Configurable approval threshold
// - Associated vault PDA for holding SOL
//
// The creator becomes owner[0] and cannot be removed.
// Additional owners can be added via proposals.

#[derive(Accounts)]
#[instruction(multisig_id: u64)]
pub struct CreateMultisig<'info> {
    // Creator and first owner of the multisig
    // Must sign and pay for account creation
    #[account(mut)]
    pub creator: Signer<'info>,

    // Multisig account PDA
    // Seeds: ["multisig", creator, multisig_id]
    // Stores configuration and owner list
    #[account(
        init,
        payer = creator,
        space = ANCHOR_DISCRIMINATOR + Multisig::INIT_SPACE,
        seeds = [
            MULTISIG,
            creator.key().as_ref(),
            &multisig_id.to_le_bytes(),
        ],
        bump,
    )]
    pub multisig_account: Account<'info, Multisig>,

    // Vault PDA - holds SOL for the multisig
    // Seeds: ["vault", multisig_account]
    // Created as a system-owned account
    #[account(
        mut,
        seeds = [
            VAULT,
            multisig_account.key().as_ref(),
        ],
        bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreateMultisig<'info> {
    pub fn create_multisig(
        &mut self,
        multisig_id: u64,
        threshold: u8,
        timelock_seconds: u64,
        bumps: &CreateMultisigBumps,
    ) -> Result<()> {
        let multisig = self.multisig_account.key();

        // SECURITY CHECKS

        // 1. Threshold Validation - Lower Bound
        // Ensures at least one approval is required
        // Prevents threshold=0 which would allow immediate execution
        require!(threshold >= 1, MultisigError::InvalidThreshold);

        // 2. Threshold Validation - Upper Bound
        // Threshold cannot exceed number of owners
        // At creation, owner_count=1, so threshold must be 1
        // This will be validated again when adding owners
        require!(
            threshold <= 1,
            MultisigError::ThresholdExceedsOwners
        );

        // 3. Initialize Members Array
        // Fixed-size array avoids realloc vulnerabilities
        // Creator is automatically Admin (index 0)
        let mut members = [Member::default(); MAX_OWNERS];
        members[0] = Member {
            pubkey: self.creator.key(),
            role: MemberRole::Admin,
        };

        // 4. Set Multisig State
        // Store all configuration and PDAs
        // Use vault.key() directly instead of re-deriving
        self.multisig_account.set_inner(Multisig {
            multisig_id,
            creator: self.creator.key(),
            threshold,
            owner_count: 1,
            members,
            proposal_count: 0,
            last_executed_proposal: 0,
            paused: false,
            timelock_seconds,
            vault: self.vault.key(),
            bump: bumps.multisig_account,
            vault_bump: bumps.vault,
        });

        // 5. Initialize Vault Account
        // Transfer minimum rent to create the vault account

       let signer_seeds: &[&[&[u8]]] = &[&[
        VAULT,
        multisig.as_ref(),
        &[bumps.vault],
        ]];

    
        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(0);
        create_account(
             CpiContext::new(
                self.system_program.to_account_info(),

                CreateAccount {
                    from: self.creator.to_account_info(),
                    to: self.vault.to_account_info(),
                },
            ).with_signer(signer_seeds)
            , 
            min_rent, 
            0, 
            &self.system_program.key(),
        )?;
 
        Ok(())
    }
}
