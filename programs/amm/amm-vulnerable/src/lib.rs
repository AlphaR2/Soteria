use anchor_lang::prelude::*;

declare_id!("AMMVULN1111111111111111111111111111111111111");

#[program]
pub mod amm_vulnerable {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("AMM Vulnerable - Placeholder");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
