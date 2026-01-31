use anchor_lang::prelude::*;

// Member role determines permissions within the multisig
//
// VULNERABILITY [CRITICAL]: Roles are defined but NOT enforced
//
// The secure version enforces role-based access control:
// - Admin: Full control (creator only) - can propose, approve, execute, manage members
// - Proposer: Can create proposals and approve
// - Executor: Can only approve and execute
//
// This vulnerable version defines roles but the instruction handlers
// don't actually check them, allowing any member to perform any action.
//
// Example Attack:
//   1. Alice (Admin) creates multisig and adds Bob as Executor
//   2. Bob should only be able to approve and execute
//   3. But without role enforcement, Bob can also:
//      - Add/remove members (Admin only)
//      - Create proposals (Admin/Proposer only)
//      - Pause the multisig (Admin only)
//
// Fix: Check member.role before each privileged operation.

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
