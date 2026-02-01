// AMM Helper Functions - VULNERABLE VERSION
//
// WARNING: This version contains intentional vulnerabilities for educational purposes.
//
// VULNERABILITIES:
// V003: validate_expiration does nothing
// V004: Unchecked arithmetic (overflow/underflow risk)

use anchor_lang::prelude::*;
use anchor_spl::token::{Burn, MintTo, Transfer, burn, mint_to, transfer};

use crate::{constants::*, errors::*};

// VALIDATION HELPERS

// VULNERABILITY V003: Expiration validation does nothing
// Secure version checks expiration > current_time and <= MAX_EXPIRATION_SECONDS
// Vulnerable version accepts any expiration timestamp
// Attack: User submits transaction at good price, transaction pending for hours/days,
// executes at terrible price when market has moved significantly
pub fn validate_expiration(_expiration: i64) -> Result<()> {
    // Secure version implementation (removed):
    // let current_time = Clock::get()?.unix_timestamp;
    // require!(expiration > current_time, AmmError::TransactionExpired);
    // let time_until_expiration = expiration.checked_sub(current_time)?;
    // require!(time_until_expiration <= MAX_EXPIRATION_SECONDS, AmmError::ExpirationTooFar);

    Ok(())
}

// LIQUIDITY CALCULATION HELPERS

// Calculate LP tokens for first deposit (pool initialization)
// VULNERABILITY V004: Unchecked arithmetic (overflow risk)
// VULNERABILITY V005: MINIMUM_LIQUIDITY = 1 (enables inflation attacks)
//
// Inflation attack scenario:
// 1. Attacker creates pool with 1 token A + 1 token B
// 2. Gets sqrt(1*1) - 1 = 0 LP tokens (1 token locked)
// 3. Attacker donates 1,000,000 tokens directly to vaults
// 4. Next depositor deposits 1000 A + 1000 B
// 5. LP calculation: min(1000/1000001, 1000/1000001) * 1 = 0 LP tokens
// 6. Victim receives 0 LP tokens, loses entire deposit
pub fn calculate_first_deposit(amount_a: u64, amount_b: u64) -> Result<(u64, u64, u64)> {
    // VULNERABILITY V004: Unchecked arithmetic
    // Secure version uses: checked_mul()
    // Vulnerable version: direct multiplication can overflow
    let product = (amount_a as u128) * (amount_b as u128);

    let liquidity = (product as f64).sqrt() as u64;

    require!(liquidity > MINIMUM_LIQUIDITY, AmmError::InsufficientLiquidity);

    // VULNERABILITY V004: Unchecked subtraction
    // Secure version uses: checked_sub()
    // Vulnerable version: direct subtraction can underflow
    let lp_tokens = liquidity - MINIMUM_LIQUIDITY;

    Ok((amount_a, amount_b, lp_tokens))
}

// Calculate proportional deposit for existing pool
// VULNERABILITY V004: Unchecked arithmetic (overflow/underflow risk)
pub fn calculate_subsequent_deposit(
    desired_a: u64,
    desired_b: u64,
    vault_a: u64,
    vault_b: u64,
    lp_supply: u64,
) -> Result<(u64, u64, u64)> {
    // VULNERABILITY V004: Unchecked arithmetic
    // Secure version uses: checked_mul() and checked_div()
    // Vulnerable version: direct operations can overflow
    let lp_from_a = (desired_a as u128) * (lp_supply as u128) / (vault_a as u128);
    let lp_from_b = (desired_b as u128) * (lp_supply as u128) / (vault_b as u128);

    let lp_to_mint = std::cmp::min(lp_from_a, lp_from_b);

    // VULNERABILITY V004: Unchecked arithmetic
    let amount_a = (lp_to_mint * (vault_a as u128) / (lp_supply as u128)) as u64;
    let amount_b = (lp_to_mint * (vault_b as u128) / (lp_supply as u128)) as u64;

    Ok((amount_a, amount_b, lp_to_mint as u64))
}

// Calculate withdrawal amounts when burning LP tokens
// VULNERABILITY V004: Unchecked arithmetic (overflow/underflow risk)
pub fn calculate_withdrawal(
    lp_to_burn: u64,
    vault_a: u64,
    vault_b: u64,
    lp_supply: u64,
) -> Result<(u64, u64)> {
    // VULNERABILITY V004: Unchecked arithmetic
    // Secure version uses: checked_mul() and checked_div()
    // Vulnerable version: direct operations can overflow/underflow
    // Attack: Large numbers could overflow, user receives incorrect amounts
    let amount_a = ((lp_to_burn as u128) * (vault_a as u128) / (lp_supply as u128)) as u64;
    let amount_b = ((lp_to_burn as u128) * (vault_b as u128) / (lp_supply as u128)) as u64;

    Ok((amount_a, amount_b))
}

// CPI HELPERS

// Generic token transfer helper
// Used for transferring tokens to/from vaults
pub fn transfer_tokens<'info>(
    amount: u64,
    token_program: &AccountInfo<'info>,
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
) -> Result<()> {
    transfer(
        CpiContext::new(
            token_program.clone(),
            Transfer {
                from: from.clone(),
                to: to.clone(),
                authority: authority.clone(),
            },
        ),
        amount,
    )
}

// Transfer tokens from vault (requires PDA signer)
// Used in withdraw and swap instructions
pub fn transfer_from_vault<'info>(
    amount: u64,
    token_program: &AccountInfo<'info>,
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    authority_seeds: &[&[u8]],
) -> Result<()> {
    let signer_seeds = &[authority_seeds];

    transfer(
        CpiContext::new_with_signer(
            token_program.clone(),
            Transfer {
                from: from.clone(),
                to: to.clone(),
                authority: authority.clone(),
            },
            signer_seeds,
        ),
        amount,
    )
}

// Mint LP tokens (requires PDA authority)
// Used when depositing liquidity
pub fn mint_lp_tokens<'info>(
    amount: u64,
    token_program: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    authority_seeds: &[&[u8]],
) -> Result<()> {
    let signer_seeds = &[authority_seeds];

    mint_to(
        CpiContext::new_with_signer(
            token_program.clone(),
            MintTo {
                mint: mint.clone(),
                to: to.clone(),
                authority: authority.clone(),
            },
            signer_seeds,
        ),
        amount,
    )
}

// Burn LP tokens
// Used when withdrawing liquidity
// User burns LP tokens to receive their proportional share
pub fn burn_lp_tokens<'info>(
    amount: u64,
    token_program: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    from: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
) -> Result<()> {
    burn(
        CpiContext::new(
            token_program.clone(),
            Burn {
                mint: mint.clone(),
                from: from.clone(),
                authority: authority.clone(),
            },
        ),
        amount,
    )
}