use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};
use crate::{state::*, constants::*};

// Create Multisig Instruction - VULNERABLE VERSION
//
// Creates a new multisig wallet with multiple security vulnerabilities.
// Compare with secure version to see proper implementation.

#[derive(Accounts)]
#[instruction(multisig_id: u64)]
pub struct CreateMultisig<'info> {
    // VULNERABILITY [MEDIUM]: No explicit signer check documentation
    //
    // While Anchor's Signer type enforces this, the code doesn't
    // clearly document what happens if signature validation fails.
    // In a native program, this would be a critical vulnerability.
    #[account(mut)]
    pub creator: Signer<'info>,

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

        // VULNERABILITY [CRITICAL]: Missing threshold lower bound check
        //
        // The secure version validates: threshold >= 1
        // Without this check, threshold = 0 means NO approvals needed!
        // Any proposal would be immediately executable.
        //
        // Example Attack:
        //   1. Attacker creates multisig with threshold = 0
        //   2. Attacker creates proposal to transfer all funds
        //   3. Proposal is immediately executable (0 approvals needed)
        //   4. Attacker drains all funds in single transaction
        //
        // Fix: require!(threshold >= 1, MultisigError::InvalidThreshold);


        // VULNERABILITY [HIGH]: Missing threshold upper bound check
        //
        // The secure version validates: threshold <= owner_count (which is 1 at creation)
        // Without this check, threshold can exceed owner count.
        //
        // Example Attack (DoS):
        //   1. Attacker creates multisig with threshold = 255, owner_count = 1
        //   2. Only 1 member exists, needs 255 approvals
        //   3. No proposal can ever reach threshold
        //   4. Funds are permanently locked
        //
        // Example Attack (Griefing):
        //   1. Service creates multisig for user with threshold = 5
        //   2. Only 2 members ever added
        //   3. Multisig is unusable, funds locked
        //
        // Fix: require!(threshold <= 1, MultisigError::ThresholdExceedsOwners);


        // VULNERABILITY [MEDIUM]: No timelock validation
        //
        // The secure version limits timelock to reasonable values (e.g., max 2 days).
        // Without validation, attacker can set:
        // - timelock = 0: Immediate execution, no delay for security review
        // - timelock = u64::MAX: Proposals can never execute, permanent DoS
        //
        // Fix: Validate reasonable timelock bounds.


        // Initialize members array
        // This part is secure - using fixed-size array avoids realloc attacks
        let mut members = [Member::default(); MAX_OWNERS];
        members[0] = Member {
            pubkey: self.creator.key(),
            role: MemberRole::Admin,
        };

        // Set multisig state
        self.multisig_account.set_inner(Multisig {
            multisig_id,
            creator: self.creator.key(),
            threshold, // VULNERABLE: Unvalidated threshold stored
            owner_count: 1,
            members,
            proposal_count: 0,
            last_executed_proposal: 0,
            paused: false,
            timelock_seconds, // VULNERABLE: Unvalidated timelock stored
            vault: self.vault.key(),
            bump: bumps.multisig_account,
            vault_bump: bumps.vault,
        });

        // Initialize vault account
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
            ).with_signer(signer_seeds),
            min_rent,
            0,
            &self.system_program.key(),
        )?;

        Ok(())
    }
}
