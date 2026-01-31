use anchor_lang::prelude::*;
use crate::{
    state::Member,
    constants::*
};

// Multisig wallet account
//
// VULNERABILITY [HIGH]: Helper methods don't enforce security
//
// The is_member() and member_index() methods exist, but the critical
// role-checking methods (can_propose, can_execute, is_admin) are missing
// or not properly used in instructions.
//
// The secure version has comprehensive helper methods:
// - is_member(), get_member(), member_index()
// - has_role(), is_admin()
// - can_propose() - checks Admin or Proposer role
// - can_execute() - checks Admin or Executor role
// - can_approve() - checks membership
// - is_valid_threshold()
//
// Fix: Implement and USE all role-checking helper methods.

#[account]
#[derive(InitSpace)]
pub struct Multisig {
    pub multisig_id: u64,
    pub creator: Pubkey,
    pub threshold: u8,
    pub owner_count: u8,

    // VULNERABILITY [MEDIUM]: Fixed array but no bounds checking
    //
    // The array is fixed size to avoid realloc vulnerabilities (good),
    // but instructions don't check owner_count < MAX_OWNERS before
    // adding members, which could cause array index out of bounds.
    //
    // Fix: Always validate owner_count < MAX_OWNERS before adding.
    pub members: [Member; MAX_OWNERS],

    pub proposal_count: u64,
    pub last_executed_proposal: u64,

    // VULNERABILITY [CRITICAL]: Pause state exists but not checked
    //
    // The paused flag exists, but instruction handlers don't check it.
    // Even when paused, all operations still work normally.
    //
    // Fix: Add `require!(!multisig.paused, MultisigError::MultisigPaused)`
    // at the start of every instruction except toggle_pause.
    pub paused: bool,

    pub timelock_seconds: u64,
    pub vault: Pubkey,
    pub bump: u8,
    pub vault_bump: u8,
}

impl Multisig {
    // Basic membership check - this is implemented
    pub fn is_member(&self, key: &Pubkey) -> bool {
        self.members
            .iter()
            .take(self.owner_count as usize)
            .any(|member| &member.pubkey == key)
    }

    // Get member index
    pub fn member_index(&self, key: &Pubkey) -> Option<usize> {
        self.members
            .iter()
            .take(self.owner_count as usize)
            .position(|member| &member.pubkey == key)
    }

    // VULNERABILITY [CRITICAL]: Role checking methods are missing or unused
    //
    // The secure version has:
    // - get_member() -> Option<&Member>
    // - has_role() -> bool
    // - is_admin() -> bool (checks key == creator)
    // - can_propose() -> bool (Admin or Proposer)
    // - can_approve() -> bool (any member)
    // - can_execute() -> bool (Admin or Executor)
    //
    // These methods are MISSING here, and even if added, the vulnerable
    // instructions don't call them.
    //
    // Fix: Implement all role methods AND use them in instructions.

    // VULNERABILITY [MEDIUM]: No threshold validation method
    //
    // The secure version has is_valid_threshold() which ensures:
    // threshold >= 1 && threshold <= owner_count
    //
    // Without this, invalid thresholds can be set, either:
    // - threshold = 0 (no approvals needed!)
    // - threshold > owner_count (proposals can never execute)
    //
    // Fix: Implement and use is_valid_threshold() in create_multisig
    // and execute_proposal (for ChangeThreshold).
}
