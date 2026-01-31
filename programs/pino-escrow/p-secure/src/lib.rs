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

address::declare_id!("J8Ru6Zti7EwTwVt35BGN2irvD1ELEjv2MkCYGAbCqaok");

program_entrypoint!(process_instruction);
no_allocator!();
// nostd_panic_handler!();

pub fn process_instruction(
    _program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    
    // Verify correct program ID
    if _program_id != &crate::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

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