use anchor_lang::prelude::*;

declare_id!("BDKRVtanadszCG81PbcHoA4KsS7vwxeGScGi4p3iJoGS");

#[program]
pub mod g_vulnerable {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
