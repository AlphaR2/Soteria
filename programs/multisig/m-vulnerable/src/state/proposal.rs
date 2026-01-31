use anchor_lang::prelude::*;
use crate::constants::MAX_OWNERS;
use super::member::*;

// Proposal status enum
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ProposalStatus {
    Active,
    Executed,
    Cancelled,
}

impl Default for ProposalStatus {
    fn default() -> Self {
        ProposalStatus::Active
    }
}

// Proposal Types
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ProposalType {
    // VULNERABILITY [CRITICAL]: No validation on new_member in AddMember
    //
    // The secure version validates:
    // - new_member != proposer (can't add yourself)
    // - new_member not already a member (no duplicates)
    // - new_member != Pubkey::default()
    // - owner_count < MAX_OWNERS
    //
    // This vulnerable version accepts any pubkey without checks.
    //
    // Example Attack:
    //   1. Attacker creates AddMember proposal with Pubkey::default()
    //   2. Default pubkey gets added as a member
    //   3. Anyone who signs with default key can now approve/execute
    AddMember { new_member: Pubkey, role: MemberRole },

    // VULNERABILITY [CRITICAL]: Creator can be removed
    //
    // The secure version prevents removing the creator for accountability.
    // The secure version also checks:
    // - member_to_remove is actually a member
    // - owner_count > 1 (must have at least 1 member)
    // - threshold <= new_owner_count after removal
    //
    // This vulnerable version allows removing the creator, potentially
    // locking all funds if admin-only operations are required later.
    RemoveMember { member_to_remove: Pubkey },

    // VULNERABILITY [HIGH]: No bounds checking on threshold
    //
    // The secure version ensures:
    // - new_threshold >= 1
    // - new_threshold <= owner_count
    //
    // This vulnerable version allows:
    // - threshold = 0 (any proposal auto-executes!)
    // - threshold > owner_count (proposals can never execute, DoS)
    ChangeThreshold { new_threshold: u8 },

    // VULNERABILITY [MEDIUM]: No max timelock validation
    //
    // The secure version limits timelock to 2 days.
    // This vulnerable version allows any timelock, including:
    // - timelock = u64::MAX (proposals can never execute, DoS)
    // - timelock = 0 (no delay, immediate execution of malicious proposals)
    ChangeTimelock { new_timelock: u64 },
}

// Proposal account
#[account]
#[derive(InitSpace)]
pub struct Proposal {
    pub multisig: Pubkey,
    pub proposal_id: u64,
    pub proposer: Pubkey,
    pub proposal_type: ProposalType,
    pub status: ProposalStatus,

    // VULNERABILITY [CRITICAL]: Approval bitmap not properly used
    //
    // The bitmap exists to track which members approved (one bit per member).
    // The secure version:
    // - Sets bit: bitmap |= 1u64 << owner_index
    // - Checks bit: (bitmap & (1u64 << owner_index)) != 0
    // - Prevents double approval by checking before setting
    //
    // This vulnerable version may:
    // - Not check the bitmap before approving (double approvals)
    // - Incorrectly manipulate bits (wrong member credited)
    // - Allow approval_count to exceed actual approvals
    //
    // Example Attack:
    //   1. Alice approves (approval_count = 1)
    //   2. Alice calls approve again
    //   3. Without bitmap check, approval_count = 2
    //   4. Threshold of 2 is met with only 1 actual approver!
    pub approval_bitmap: u64,

    pub approval_count: u8,
    pub created_at: i64,
    pub expires_at: i64,
    pub executed_at: i64,
    pub bump: u8,
}

impl Proposal {
    // VULNERABILITY [CRITICAL]: has_approved might not be used
    //
    // The method exists but vulnerable instructions may not call it,
    // allowing double approvals.
    pub fn has_approved(&self, owner_index: usize) -> bool {
        if owner_index >= MAX_OWNERS {
            return false;
        }
        (self.approval_bitmap & (1u64 << owner_index)) != 0
    }

    // VULNERABILITY [CRITICAL]: approve() might not check has_approved first
    //
    // The secure version returns false if already approved.
    // If callers don't check the return value, double approvals slip through.
    pub fn approve(&mut self, owner_index: usize) -> bool {
        if owner_index >= MAX_OWNERS || self.has_approved(owner_index) {
            return false;
        }
        self.approval_bitmap |= 1u64 << owner_index;
        self.approval_count += 1;
        true
    }

    // VULNERABILITY [HIGH]: is_ready_to_execute not comprehensive
    //
    // The secure version checks:
    // - approval_count >= threshold
    // - status == Active
    // - timelock_passed()
    // - !is_expired()
    //
    // This vulnerable version only checks approval count, missing
    // timelock, expiry, and status checks.
    pub fn is_ready_to_execute(&self, threshold: u8) -> bool {
        self.approval_count >= threshold
        // MISSING: && self.status == ProposalStatus::Active
        // MISSING: && timelock check
        // MISSING: && expiry check
    }

    pub fn is_active(&self) -> bool {
        self.status == ProposalStatus::Active
    }

    // VULNERABILITY [HIGH]: Expiry check exists but may not be called
    //
    // Even if implemented correctly, the execute handler might not
    // call this method, allowing expired proposals to execute.
    pub fn is_expired(&self, current_timestamp: i64) -> bool {
        current_timestamp > self.expires_at
    }

    // VULNERABILITY [HIGH]: Timelock check exists but may not be called
    //
    // Same issue - the method exists but execute handler might skip it,
    // allowing immediate execution of malicious proposals.
    pub fn timelock_passed(&self, current_timestamp: i64, timelock_seconds: u64) -> bool {
        let timelock_end = self.created_at + timelock_seconds as i64;
        current_timestamp >= timelock_end
    }
}
