use anchor_lang::prelude::*;
use crate::constants::MAX_OWNERS;
use super::proposal::ProposalStatus;

// Transfer Proposal account
//
// Separate from governance proposals for cleaner architecture.
// Contains SOL transfer-specific data.

#[account]
#[derive(InitSpace)]
pub struct TransferProposal {
    pub multisig: Pubkey,
    pub proposal_id: u64,
    pub proposer: Pubkey,
    pub status: ProposalStatus,
    pub approval_bitmap: u64,
    pub approval_count: u8,
    pub created_at: i64,
    pub expires_at: i64,
    pub executed_at: i64,

    // Transfer-specific fields
    // VULNERABILITY [HIGH]: No amount validation at creation
    //
    // The secure version validates:
    // - amount > 0 (no zero transfers)
    // - At execution: vault.lamports >= amount
    //
    // This vulnerable version accepts any amount, including 0 or
    // amounts exceeding vault balance.
    pub amount: u64,

    // VULNERABILITY [CRITICAL]: Recipient not validated
    //
    // The secure version validates:
    // - recipient != Pubkey::default()
    // - At execution: recipient is system-owned
    // - At execution: recipient matches stored value
    //
    // This vulnerable version allows:
    // - Sending to default pubkey (funds lost)
    // - Substituting different recipient at execution
    // - Sending to PDAs that can't handle lamports
    pub recipient: Pubkey,

    pub bump: u8,
}

impl TransferProposal {
    // Same vulnerabilities as Proposal for approval methods

    pub fn has_approved(&self, owner_index: usize) -> bool {
        if owner_index >= MAX_OWNERS {
            return false;
        }
        (self.approval_bitmap & (1u64 << owner_index)) != 0
    }

    // VULNERABILITY [CRITICAL]: No double-approval check enforcement
    //
    // Returns false if already approved, but callers might not check
    // the return value, allowing approval_count inflation.
    pub fn approve(&mut self, owner_index: usize) -> bool {
        if owner_index >= MAX_OWNERS || self.has_approved(owner_index) {
            return false;
        }
        self.approval_bitmap |= 1u64 << owner_index;
        self.approval_count += 1;
        true
    }

    pub fn is_active(&self) -> bool {
        self.status == ProposalStatus::Active
    }

    pub fn is_expired(&self, current_timestamp: i64) -> bool {
        current_timestamp > self.expires_at
    }

    pub fn timelock_passed(&self, current_timestamp: i64, timelock_seconds: u64) -> bool {
        let timelock_end = self.created_at + timelock_seconds as i64;
        current_timestamp >= timelock_end
    }
}
