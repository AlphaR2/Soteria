pub mod propose_offer;
pub mod take_offer;

pub use propose_offer::*;
pub use take_offer::*;

use pinocchio::error::ProgramError;

#[repr(u8)]
pub enum Instruction {
    ProposeOffer = 0, 
    TakeOffer = 1,    
}

impl TryFrom<&u8> for Instruction {
    type Error = ProgramError;
    
    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(Instruction::ProposeOffer),  
            1 => Ok(Instruction::TakeOffer),     
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

