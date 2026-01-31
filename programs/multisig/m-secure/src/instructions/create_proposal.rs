use anchor_lang::prelude::*;
use crate::{state::*, errors::*, constants::*};

// Create Proposal Instruction
//
// Allows any owner to propose an action requiring multi-signature approval.
// Proposal types: AddOwner, RemoveOwner, ChangeThreshold
//
// The proposer automatically approves their own proposal (approval_count starts at 1).
// Proposal remains active until executed or cancelled.

#[derive(Accounts)]
pub struct CreateProposal<'info> {
    // Proposer - must be an existing owner of the multisig
    // Must sign and pay for proposal account creation
    #[account(mut)]
    pub proposer: Signer<'info>,

    // Multisig account - contains owner list and configuration
    // Must be owned by this program to prevent fake multisig accounts
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

    // Proposal PDA - stores the pending action
    // Seeds: ["proposal", multisig_account, proposal_id]
    // proposal_id comes from multisig_account.proposal_count
    #[account(
        init,
        payer = proposer,
        space = ANCHOR_DISCRIMINATOR + Proposal::INIT_SPACE,
        seeds = [
            PROPOSAL,
            multisig_account.key().as_ref(),
            &multisig_account.proposal_count.to_le_bytes(),
        ],
        bump,
    )]
    pub proposal: Account<'info, Proposal>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreateProposal<'info> {
    pub fn create_proposal(
        &mut self,
        proposal_type: ProposalType,
        bumps: &CreateProposalBumps,
    ) -> Result<()> {
        // SECURITY CHECKS

        // 1. Pause Check
        // Multisig must not be paused
        // Only unpause instruction allowed when paused
        require!(
            !self.multisig_account.paused,
            MultisigError::MultisigPaused
        );

        // 2. Member Validation
        // Only existing members can create proposals
        // Prevents external actors from spamming proposals
        require!(
            self.multisig_account.is_member(&self.proposer.key()),
            MultisigError::NotAMember
        );

        // 3. Role-Based Permission Check
        // Only Admin or Proposer can create proposals
        // Executor role can only approve, not propose
        require!(
            self.multisig_account.can_propose(&self.proposer.key()),
            MultisigError::CannotPropose
        );

        // Get proposer's index for auto-approval
        let proposer_index = self.multisig_account
            .member_index(&self.proposer.key())
            .ok_or(MultisigError::NotAMember)?;

       

        // 5. Proposal Type Specific Validation
        match proposal_type {
         
            ProposalType::AddMember { new_member, role: _ } => {
                // Only admin can add members
                // Prevents non-admins from adding members
                require!(
                    self.multisig_account.is_admin(&self.proposer.key()),
                    MultisigError::OnlyAdmin
                );

                // Cannot add yourself
                // Prevents self-invitation attacks
                require!(
                    new_member != self.proposer.key(),
                    MultisigError::CannotAddSelf
                );

                // Validate not already a member
                // Prevents duplicate member entries
                require!(
                    !self.multisig_account.is_member(&new_member),
                    MultisigError::AlreadyMember
                );

                // Validate new member is not default pubkey
                require!(
                    new_member != Pubkey::default(),
                    MultisigError::InvalidParameter
                );

                // Validate max members not reached
                // Fixed array has MAX_OWNERS limit
                require!(
                    self.multisig_account.owner_count < MAX_OWNERS as u8,
                    MultisigError::MaxMembersReached
                );
            }

            ProposalType::RemoveMember { member_to_remove } => {
                // Only admin can remove members
                require!(
                    self.multisig_account.is_admin(&self.proposer.key()),
                    MultisigError::OnlyAdmin
                );

                // Validate member exists
                require!(
                    self.multisig_account.is_member(&member_to_remove),
                    MultisigError::NotAMember
                );

                // Validate not removing creator
                // Creator is immutable for accountability
                require!(
                    member_to_remove != self.multisig_account.creator,
                    MultisigError::CannotRemoveCreator
                );

                // Validate won't go below minimum members
                // Must have at least 1 member (the creator)
                require!(
                    self.multisig_account.owner_count > 1,
                    MultisigError::MinimumOneMember
                );

                // After removal, threshold must still be valid
                // New owner_count will be current - 1
                let new_owner_count = self.multisig_account.owner_count - 1;
                require!(
                    self.multisig_account.threshold <= new_owner_count,
                    MultisigError::ThresholdExceedsOwners
                );
            }

            ProposalType::ChangeThreshold { new_threshold } => {
                // Validate threshold bounds
                require!(new_threshold >= 1, MultisigError::InvalidThreshold);

                // Threshold cannot exceed current owner count
                require!(
                    new_threshold <= self.multisig_account.owner_count,
                    MultisigError::ThresholdExceedsOwners
                );
            }

            ProposalType::ChangeTimelock { new_timelock } => {
                // Only admin can change timelock
                require!(
                    self.multisig_account.is_admin(&self.proposer.key()),
                    MultisigError::OnlyAdmin
                );

                // Validate reasonable timelock (not more than 2 days)
                const MAX_TIMELOCK: u64 = 2 * 24 * 60 * 60;
                require!(
                    new_timelock <= MAX_TIMELOCK,
                    MultisigError::InvalidParameter
                );
            }
        }

        // 6. Increment Proposal Count
        // Use checked_add to prevent overflow 
        // If proposal_count overflows, entire protocol is compromised
        self.multisig_account.proposal_count = self
            .multisig_account
            .proposal_count
            .checked_add(1)
            .ok_or(MultisigError::Overflow)?;

        let proposal_id = self.multisig_account.proposal_count - 1;

        // 7. Initialize Proposal State
        // Proposer auto-approves their own proposal
        let mut approval_bitmap: u64 = 0;
        approval_bitmap |= 1u64 << proposer_index;

        let clock = Clock::get()?;

        // Calculate expiry: created_at + timelock + grace period
        let expires_at = clock
            .unix_timestamp
            .checked_add(self.multisig_account.timelock_seconds as i64)
            .and_then(|t| t.checked_add(DEFAULT_EXPIRY_PERIOD as i64))
            .ok_or(MultisigError::Overflow)?;

        self.proposal.set_inner(Proposal {
            multisig: self.multisig_account.key(),
            proposal_id,
            proposer: self.proposer.key(),
            proposal_type,
            status: ProposalStatus::Active,
            approval_bitmap,
            approval_count: 1, // Proposer auto-approves
            created_at: clock.unix_timestamp,
            expires_at,
            executed_at: 0,
            bump: bumps.proposal,
        });

        Ok(())
    }
}
