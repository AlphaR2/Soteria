// AMM Helper Functions
//
// Reusable calculation and CPI helpers for the AMM program.
// These functions reduce code duplication across instructions.

use anchor_lang::prelude::*;
use anchor_spl::token::{Burn, MintTo, Transfer, burn, mint_to, transfer};

use crate::{constants::*, errors::*};

// VALIDATION HELPERS

// Validate transaction expiration timestamp
// Ensures transaction is not expired and not too far in the future
// Used in deposit, withdraw, and swap instructions
pub fn validate_expiration(expiration: i64) -> Result<()> {
    let current_time = Clock::get()?.unix_timestamp;

    // Transaction must not be expired
    require!(expiration > current_time, AmmError::TransactionExpired);

    // Calculate time until expiration with overflow protection
    let time_until_expiration = expiration
        .checked_sub(current_time)
        .ok_or(AmmError::Underflow)?;

    // Expiration cannot be more than MAX_EXPIRATION_SECONDS in the future
    require!(
        time_until_expiration <= MAX_EXPIRATION_SECONDS,
        AmmError::ExpirationTooFar
    );

    Ok(())
}

// LIQUIDITY CALCULATION HELPERS

// Calculate LP tokens for first deposit (pool initialization)
// Uses geometric mean formula: LP = sqrt(a * b) - MINIMUM_LIQUIDITY
// The MINIMUM_LIQUIDITY is permanently locked to prevent inflation attacks
//
// Why lock minimum liquidity?
// Without locking, an attacker could:
// 1. Create pool with 1 wei of each token
// 2. Receive sqrt(1*1) = 1 LP token
// 3. Donate large amounts to inflate LP token value
// 4. Small depositors get rounded to 0 LP tokens
pub fn calculate_first_deposit(amount_a: u64, amount_b: u64) -> Result<(u64, u64, u64)> {
    // Calculate product with overflow protection
    let product = (amount_a as u128)
        .checked_mul(amount_b as u128)
        .ok_or(AmmError::Overflow)?;

    // Geometric mean provides initial liquidity valuation
    // sqrt(a * b) ensures equal weighting of both tokens
    // Using f64 for simplicity (production may use integer sqrt for precision)
    let liquidity = (product as f64).sqrt() as u64;

    // Ensure sufficient liquidity for minimum lock
    require!(liquidity > MINIMUM_LIQUIDITY, AmmError::InsufficientLiquidity);

    // Lock MINIMUM_LIQUIDITY permanently by not minting those LP tokens
    // This protects against inflation attacks
    let lp_tokens = liquidity
        .checked_sub(MINIMUM_LIQUIDITY)
        .ok_or(AmmError::Underflow)?;

    Ok((amount_a, amount_b, lp_tokens))
}

// Calculate proportional deposit for existing pool
// Maintains current pool ratio to prevent price manipulation
// Formula: LP_minted = min(desired_a/vault_a, desired_b/vault_b) * lp_supply
//
// Why use min()?
// Using minimum ensures depositor cannot manipulate pool price.
// Excess tokens are not deposited, maintaining the pool ratio.
pub fn calculate_subsequent_deposit(
    desired_a: u64,
    desired_b: u64,
    vault_a: u64,
    vault_b: u64,
    lp_supply: u64,
) -> Result<(u64, u64, u64)> {
    // Calculate LP tokens if only depositing token A
    // Formula: LP = (desired_a / vault_a) * lp_supply
    let lp_from_a = (desired_a as u128)
        .checked_mul(lp_supply as u128)
        .ok_or(AmmError::Overflow)?
        .checked_div(vault_a as u128)
        .ok_or(AmmError::DivisionByZero)?;

    // Calculate LP tokens if only depositing token B
    let lp_from_b = (desired_b as u128)
        .checked_mul(lp_supply as u128)
        .ok_or(AmmError::Overflow)?
        .checked_div(vault_b as u128)
        .ok_or(AmmError::DivisionByZero)?;

    // Use minimum to maintain pool ratio
    // This prevents price manipulation
    let lp_to_mint = std::cmp::min(lp_from_a, lp_from_b);

    // Calculate actual token amounts needed based on LP to mint
    // These amounts maintain the pool's current ratio
    let amount_a = (lp_to_mint)
        .checked_mul(vault_a as u128)
        .ok_or(AmmError::Overflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::DivisionByZero)? as u64;

    let amount_b = (lp_to_mint)
        .checked_mul(vault_b as u128)
        .ok_or(AmmError::Overflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::DivisionByZero)? as u64;

    Ok((amount_a, amount_b, lp_to_mint as u64))
}

// Calculate withdrawal amounts when burning LP tokens
// Returns proportional share of both tokens
// Formula: amount = (lp_burned / lp_supply) * vault_balance
pub fn calculate_withdrawal(
    lp_to_burn: u64,
    vault_a: u64,
    vault_b: u64,
    lp_supply: u64,
) -> Result<(u64, u64)> {
    // Calculate token A to withdraw
    let amount_a = (lp_to_burn as u128)
        .checked_mul(vault_a as u128)
        .ok_or(AmmError::Overflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::DivisionByZero)? as u64;

    // Calculate token B to withdraw
    let amount_b = (lp_to_burn as u128)
        .checked_mul(vault_b as u128)
        .ok_or(AmmError::Overflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::DivisionByZero)? as u64;

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