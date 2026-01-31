use anchor_lang::prelude::*;

use crate::{constants::MAX_OWNERS, state::*};

// Transfer Proposal Account
//
// Separate account for TransferSol proposals
// Linked to a base Proposal account
// Contains transfer-specific data (amount, recipient)
//
// This separation:
// - Eliminates wasted space on governance proposals
// - Makes transfer data always present (no Option)
// - Enables type-safe execution with UncheckedAccount
// - Cleaner separation of concerns

#[account]
#[derive(InitSpace)]
pub struct TransferProposal {
    
    // The multisig this proposal belongs to
    pub multisig: Pubkey,

    // Unique proposal number within this multisig
    pub proposal_id: u64,

    // Who created this proposal (must be an owner)
    pub proposer: Pubkey,

    // Current status
    pub status: ProposalStatus,

    // ..check the bitwise-note.md for explanation..
    // Bitmap of approvals from owners
    pub approval_bitmap: u64,

    // Current approval count
    pub approval_count: u8,

    // Timestamp when proposal was created
    pub created_at: i64,

    // Timestamp when proposal expires (created_at + timelock + grace_period)
    // Expired proposals cannot be executed
    pub expires_at: i64,

    // Timestamp when proposal was executed (0 if not executed)
    pub executed_at: i64,


    // Amount of SOL to transfer from vault
    // Always present, no Option needed
    pub amount: u64,

    // Recipient address for the transfer
    // Always present, no Option needed
    pub recipient: Pubkey,

    // PDA bump seed
    pub bump: u8,
}



impl TransferProposal {
    
    // Check if a specific owner index has approved
    pub fn has_approved(&self, owner_index: usize) -> bool {
        if owner_index >= MAX_OWNERS {
            return false;
        }
        // Check if the bit at owner_index is set in approval_bitmap
        (self.approval_bitmap & (1u64 << owner_index)) != 0
    

    }

    // Record an approval from owner at given index
    pub fn approve(&mut self, owner_index: usize) -> bool {
        if owner_index >= MAX_OWNERS || self.has_approved(owner_index) {
            return false;
        }

        // for owner at index i, set the ith bit in approval_bitmap
        self.approval_bitmap |= 1u64 << owner_index;
        self.approval_count += 1;
        true
    }

    // Check if proposal has reached threshold
    pub fn is_ready_to_execute(&self, threshold: u8) -> bool {
        self.approval_count >= threshold && self.status == ProposalStatus::Active
    }

    // Check if proposal is still active
    pub fn is_active(&self) -> bool {
        self.status == ProposalStatus::Active
    }

    // Check if proposal has expired
    pub fn is_expired(&self, current_timestamp: i64) -> bool {
        current_timestamp > self.expires_at
    }

    // Check if timelock has passed
    pub fn timelock_passed(&self, current_timestamp: i64, timelock_seconds: u64) -> bool {
        let timelock_end = self.created_at + timelock_seconds as i64;
        current_timestamp >= timelock_end
    }
}

