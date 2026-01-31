#![no_std]

use pinocchio::{
    AccountView,
    Address,
    ProgramResult,
    error::ProgramError,
    address,
    no_allocator,
    program_entrypoint
};

pub mod state;
pub mod instructions;

use instructions::{
    ProposeOfferInstruction,
    TakeOfferInstruction,
    Instruction
};

address::declare_id!("97G55caS2vz4RKqa34TMZN2s6ZmEG2FZeg9jkQBAnUtu");

program_entrypoint!(process_instruction);
no_allocator!();


pub fn process_instruction(
    _program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {

    // VULNERABILITY [CRITICAL]: Missing program ID verification
    //
    // The program does not verify that _program_id matches crate::ID.
    // An attacker could invoke this program through a CPI with a spoofed program ID,
    // potentially causing the program to operate under a different identity.
    //
    // Example:
    //   Attacker deploys a malicious program that CPIs into this one with a fake program_id.
    //   The escrow thinks it is a different program, and PDA derivations may produce
    //   unexpected addresses, leading to fund misdirection.
    //
    // Fix: if _program_id != &crate::ID { return Err(ProgramError::IncorrectProgramId); }

    // Split discriminator from instruction data
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    // Route to appropriate instruction handler
    match Instruction::try_from(discriminator)? {
        Instruction::ProposeOffer => {
            let ix = ProposeOfferInstruction::try_from((accounts, data))?;
            ix.handler()
        }
        Instruction::TakeOffer => {
            let ix = TakeOfferInstruction::try_from((accounts, data))?;
            ix.handler()
        }
    }
}
