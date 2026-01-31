use pinocchio::{error::ProgramError, Address};
use core::mem::transmute;


// This represents the escrow PDA that holds information about a pending token swap.
// Sarah (initializer) deposits tokens and specifies what she wants in return.


// NOTE: We use manual unsafe transmute instead of bytemuck here because:
// - Pinocchio's Address type is NOT Pod-compatible (has internal structure - a padding)

// Use #[repr(C)] to ensure consistent memory layout across different architectures.
// Without this, Rust might rearrange fields

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MakeState {
    // Unique identifier for this escrow
    pub id: [u8; 8],
    pub proposer: Address,
    // What Sarah is offering
    pub token_mint_a: Address,
    // What Sarah wants in return
    pub token_mint_b: Address,
    // Amount of token B Sarah wants
    pub token_b_wanted_amount: u64,
    // Amount of token A Sarah is offering
    pub token_a_offered_amount: u64,
    pub bump: u8,
    // Whether this escrow is active - 1 byte
    // 0 = inactive/closed, 1 = active
    pub is_initialized: u8,
}



impl MakeState {
    // Seed prefix for PDA derivation
    pub const SEED_PREFIX: &'static [u8] = b"offer";
    pub const LEN: usize = core::mem::size_of::<MakeState>();

    // Load mutable reference from account data
    //
    // Safety:
    // This uses unsafe code to transmute raw bytes into a struct reference.
    // It's safe because:
    // 1. We verify the length matches exactly
    // 2. #[repr(C)] ensures predictable, sequential memory layout
    // 3. All fields are plain old data (POD) types
    #[inline(always)]
    pub fn load_mut(bytes: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if bytes.len() != MakeState::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(unsafe { &mut *transmute::<*mut u8, *mut Self>(bytes.as_mut_ptr()) })
    }


    // Load immutable reference from account data
    #[inline(always)]
    pub fn load(bytes: &[u8]) -> Result<&Self, ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(unsafe { &*(bytes.as_ptr() as *const Self) })
    }

    // Initialize all fields at once
    #[inline(always)]
    pub fn set_inner(
        &mut self,
        id: [u8; 8],
        proposer: Address,
        token_mint_a: Address,
        token_mint_b: Address,
        token_b_wanted_amount: u64,
        token_a_offered_amount: u64,
        bump: u8,
    ) {
        self.id = id;
        self.proposer = proposer;
        self.token_mint_a = token_mint_a;
        self.token_mint_b = token_mint_b;
        self.token_b_wanted_amount = token_b_wanted_amount;
        self.token_a_offered_amount = token_a_offered_amount;
        self.bump = bump;
        self.is_initialized = 1; // Mark as active
    }

    // Helper: Check if escrow is initialized
    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.is_initialized == 1
    }

    // Helper: Close/deactivate the escrow
    #[inline(always)]
    pub fn close(&mut self) {
        self.is_initialized = 0;
    }
}
