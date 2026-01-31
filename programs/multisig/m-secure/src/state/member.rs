use anchor_lang::prelude::*;

// Member role determines permissions within the multisig
//
// Admin: Full control (creator only)
//   - Can add/remove members
//   - Can propose and approve
//   - Can change threshold and settings
//
// Proposer: Can create proposals and approve
//   - Can propose all proposal types
//   - Can approve proposals

// Executor: Can only approve proposals
//   - Can approve existing proposals
//   - Cannot create proposals
//   - Read-only access to member management

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum MemberRole {
    Admin,
    Proposer,
    Executor,
}

impl Default for MemberRole {
    fn default() -> Self {
        MemberRole::Executor
    }
}

// Member information
// Tracks both the pubkey and role for each multisig member
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub struct Member {
    pub pubkey: Pubkey,
    pub role: MemberRole,
}

impl Default for Member {
    fn default() -> Self {
        Member {
            pubkey: Pubkey::default(),
            role: MemberRole::Executor,
        }
    }
}
