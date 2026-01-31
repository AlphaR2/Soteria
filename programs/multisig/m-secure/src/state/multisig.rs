use anchor_lang::prelude::*;
use crate::{
    state::{
    Member, MemberRole
    }, 
    constants::*
};

// Multisig wallet account
// Stores configuration and owner list
#[account]
#[derive(InitSpace)]
pub struct Multisig {
    // Unique identifier for this multisig
    pub multisig_id: u64,

    // Creator of the multisig (cannot be removed)
    pub creator: Pubkey,

    // Number of approvals required to execute a proposal
    // Must be: 1 <= threshold <= owners.len()
    pub threshold: u8,

    // Current number of active members
    pub owner_count: u8,

    // List of members with their roles
    // Fixed-size array avoids realloc vulnerabilities
    // Index 0 is always the creator with Admin role
    pub members: [Member; MAX_OWNERS],

    // Total proposals ever created (used for proposal numbering)
    pub proposal_count: u64,

    // Last executed proposal ID
    // Used for tracking execution history
    pub last_executed_proposal: u64,

    // Pause state - when true, all operations except unpause are blocked
    // Only admin can pause/unpause
    pub paused: bool,

    // Timelock duration in seconds
    // Proposals must wait this duration after creation before execution
    // Prevents immediate execution of malicious proposals
    pub timelock_seconds: u64,

    // Vault PDA address
    // Stored for easy reference and validation
    pub vault: Pubkey,

    // PDA bump seed for multisig account
    pub bump: u8,

    // PDA bump seed for vault account
    // Used for vault PDA signing when executing proposals
    pub vault_bump: u8,
}

impl Multisig {
    // Check if a pubkey is a member
    pub fn is_member(&self, key: &Pubkey) -> bool {
        self.members
            .iter()
            .take(self.owner_count as usize)
            .any(|member| &member.pubkey == key)
    }

    // Get the member info for a pubkey
    pub fn get_member(&self, key: &Pubkey) -> Option<&Member> {
        self.members
            .iter()
            .take(self.owner_count as usize)
            .find(|member| &member.pubkey == key)
    }

    // Get the index of a member, returns None if not found
    pub fn member_index(&self, key: &Pubkey) -> Option<usize> {
        self.members
            .iter()
            .take(self.owner_count as usize)
            .position(|member| &member.pubkey == key)
    }

    // Check if a member has a specific role
    pub fn has_role(&self, key: &Pubkey, role: MemberRole) -> bool {
        self.get_member(key)
            .map(|member| member.role == role)
            .unwrap_or(false)
    }

    // Check if a member is admin (creator only)
    pub fn is_admin(&self, key: &Pubkey) -> bool {
        key == &self.creator
    }

    // Check if a member can propose (Admin or Proposer)
    pub fn can_propose(&self, key: &Pubkey) -> bool {
        self.get_member(key)
            .map(|member| matches!(member.role, MemberRole::Admin | MemberRole::Proposer))
            .unwrap_or(false)
    }

    // Check if a member can approve (all roles can approve)
    pub fn can_approve(&self, key: &Pubkey) -> bool {
        self.is_member(key)
    }

    // Check if a member can execute (Admin or Executor)
    pub fn can_execute(&self, key: &Pubkey) -> bool {
        self.get_member(key)
            .map(|member| matches!(member.role, MemberRole::Admin | MemberRole::Executor))
            .unwrap_or(false)
    }

    // Check if threshold is valid for current owner count
    pub fn is_valid_threshold(&self) -> bool {
        self.threshold >= 1 && self.threshold <= self.owner_count
    }
}
