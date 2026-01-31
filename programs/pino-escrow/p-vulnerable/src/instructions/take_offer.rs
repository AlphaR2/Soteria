use pinocchio::{
    AccountView,
    ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_token::{
    instructions::{TransferChecked, CloseAccount},
    state::{Mint, TokenAccount},
};

use crate::state::MakeState;

// Account context for the Take Offer instruction
//
// The taker (Steve) provides Token B and receives Token A from the vault.
//
// Flow:
// 1. Taker sends Token B -> Proposer's ATA B
// 2. Vault sends Token A -> Taker's ATA A
// 3. Vault is closed (rent returned to proposer)
// 4. Offer PDA is closed (rent returned to taker as compensation)
pub struct TakeOfferAccounts<'a> {
    pub taker: &'a AccountView,
    pub proposer: &'a AccountView,        // Original proposer (Sarah)
    pub proposer_ata_b: &'a AccountView,  // Sarah's Token B account
    pub token_mint_b: &'a AccountView,
    pub token_mint_a: &'a AccountView,
    pub taker_ata_a: &'a AccountView,
    pub taker_ata_b: &'a AccountView,
    pub offer: &'a AccountView,
    pub vault: &'a AccountView,           // Vault holding Token A
    pub token_program: &'a AccountView,
    pub system_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for TakeOfferAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [taker, proposer, proposer_ata_b, token_mint_b, token_mint_a, taker_ata_a, taker_ata_b, offer, vault, token_program, system_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // VULNERABILITY [CRITICAL]: Missing signer check on taker
        //
        // The taker account is not verified as a transaction signer.
        // Anyone can call TakeOffer and impersonate the taker, executing the swap
        // without the taker's authorization. Combined with a fake taker_ata_b,
        // an attacker can drain the vault without paying Token B.
        //
        // Example:
        //   Attacker calls TakeOffer with Bob's pubkey as taker.
        //   Attacker passes their own ATA as taker_ata_a to receive Token A.
        //   Vault tokens go to the attacker, Bob never authorized the trade.
        //
        // Fix: if !taker.is_signer() { return Err(ProgramError::MissingRequiredSignature); }


        // VULNERABILITY [CRITICAL]: Missing token mint ownership validation
        //
        // Neither token_mint_a nor token_mint_b are verified to be owned by SPL Token program.
        // Attacker can pass fake mint accounts with fabricated decimals or supply data,
        // causing TransferChecked to operate with wrong decimals.
        //
        // Example:
        //   Attacker passes a fake mint_b with decimals=0 instead of decimals=9.
        //   TransferChecked sends 50 raw units instead of 50_000_000_000.
        //   Proposer receives almost nothing, taker gets full Token A from vault.
        //
        // Fix: if !token_mint_a.owned_by(token_program.address()) { return Err(...); }


        // VULNERABILITY [CRITICAL]: Missing offer account ownership check
        //
        // The offer PDA is not checked to be owned by this program.
        // An attacker can create a fake offer account owned by their malicious program,
        // with crafted state data (proposer, mints, amounts) to manipulate the swap.
        //
        // Example:
        //   Attacker creates a fake offer account with token_b_wanted_amount = 1.
        //   The real offer wanted 50 billion units of Token B.
        //   Taker only pays 1 unit and drains the entire vault.
        //
        // Fix: if !offer.owned_by(&crate::ID) { return Err(ProgramError::InvalidAccountOwner); }


        // VULNERABILITY [MEDIUM]: Missing offer account size check
        //
        // The offer account data length is not verified to match MakeState::LEN.
        // A malformed account with wrong size could cause buffer overread during
        // the unsafe transmute in MakeState::load(), reading garbage memory.
        //
        // Fix: if offer.data_len() != MakeState::LEN { return Err(...); }


        // VULNERABILITY [LOW]: Missing offer writable check
        //
        // The offer account is not checked for writability. The handler needs to
        // modify and close this account. Without this check, the runtime will fail
        // later with a less informative error.
        //
        // Fix: if !offer.is_writable() { return Err(ProgramError::InvalidAccountData); }


        // VULNERABILITY [HIGH]: Missing offer state validation
        //
        // The offer state is not loaded and validated against the provided accounts.
        // The following checks are all missing:
        //
        // - Active check: Offer could be already closed/taken, enabling double-spend.
        //   A taker could execute the same offer twice, draining the vault both times.
        //
        // - Proposer check: The proposer account is not verified against the stored proposer.
        //   An attacker can pass any account as proposer, redirecting Token B to themselves.
        //
        // - Mint checks: The token mints are not verified against the stored mints.
        //   An attacker can substitute a worthless token mint for the real Token B,
        //   paying with worthless tokens while receiving real Token A.
        //
        // Example (double-spend):
        //   Offer for 100 USDC <-> 50 SOL is created.
        //   Taker takes the offer, receiving 100 USDC.
        //   Taker calls TakeOffer again on the same offer (not marked closed).
        //   Without active check, program tries to transfer again.
        //
        // Example (token substitution):
        //   Offer wants 50 SOL for 100 USDC.
        //   Attacker passes mint address of worthless "SCAM" token as token_mint_b.
        //   Pays 50 SCAM tokens instead of 50 SOL, receives 100 USDC.
        //
        // Fix: Load offer state and verify is_active, proposer, token_mint_a, token_mint_b.


        // VULNERABILITY [HIGH]: Missing proposer ATA B address derivation
        //
        // The proposer's Token B ATA address is not derived and compared.
        // An attacker can pass any account as proposer_ata_b, causing Token B
        // to be sent to the wrong address (or the attacker's address).
        //
        // Example:
        //   Attacker passes their own ATA as proposer_ata_b.
        //   Taker's Token B goes to the attacker instead of Sarah.
        //   Sarah never receives payment, attacker gets free Token B.
        //
        // Fix: Derive expected ATA with find_program_address and compare.


        // VULNERABILITY [CRITICAL]: Missing taker ATA A validation
        //
        // The taker's Token A ATA is not validated for:
        // - Ownership by SPL Token program
        // - Correct data size
        // - Writability
        // - Address derivation
        //
        // An attacker can pass any writable account as taker_ata_a, receiving
        // Token A into an account they fully control.
        //
        // Fix: Full 4-point validation (owner, size, writable, derivation).


        // VULNERABILITY [HIGH]: Missing taker ATA B validation
        //
        // The taker's Token B ATA is not validated for ownership, size, or
        // address derivation. Also no balance check to verify the taker has
        // enough Token B to fulfill the offer.
        //
        // Example:
        //   Taker has 0 Token B but passes a valid-looking ATA.
        //   The TransferChecked CPI will fail at the Token program level,
        //   but compute is wasted. With a fake ATA (no ownership check),
        //   the taker could bypass the balance requirement entirely.
        //
        // Fix: Validate owner, size, derivation, and check balance >= token_b_wanted_amount.


        // VULNERABILITY [CRITICAL]: Missing vault validation
        //
        // The vault account is not checked for:
        // - Ownership by SPL Token program
        // - Correct data size
        // - Writability
        // - Address derivation from offer PDA
        // - Sufficient Token A balance
        //
        // An attacker can pass any token account as the vault. If the fake vault
        // has 0 tokens, the transfer will fail. But combined with other missing
        // checks, an attacker could craft a vault with specific amounts to
        // manipulate the swap ratio.
        //
        // Fix: Full validation (owner, size, writable, derivation, balance check).


        // No validations - all accounts accepted as-is
        Ok(Self {
            taker,
            proposer,
            proposer_ata_b,
            token_mint_b,
            token_mint_a,
            taker_ata_a,
            taker_ata_b,
            offer,
            vault,
            token_program,
            system_program,
        })
    }
}

// Take Offer Instruction
pub struct TakeOfferInstruction<'a> {
    pub accounts: TakeOfferAccounts<'a>,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for TakeOfferInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, _data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = TakeOfferAccounts::try_from(accounts)?;

        Ok(Self { accounts })
    }
}


// INSTRUCTION HANDLER

impl<'a> TakeOfferInstruction<'a> {
    pub fn handler(&self) -> ProgramResult {

        // 1: Load Offer State
        let offer_data = self.accounts.offer.try_borrow()?;
        let offer_state = MakeState::load(&offer_data)?;

        // VULNERABILITY [HIGH]: Missing active state re-check in handler
        //
        // The handler does not verify the offer is still active before proceeding.
        // Even if the validation layer checked it (which it doesn't in this version),
        // a second check here guards against race conditions where the offer was
        // closed between validation and execution.
        //
        // Example:
        //   Two takers call TakeOffer on the same offer simultaneously.
        //   Both pass validation, both enter the handler.
        //   Without this check, both transfers execute, draining the vault twice.
        //
        // Fix: if !offer_state.is_active() { return Err(ProgramError::InvalidAccountData); }

        let token_b_amount = offer_state.token_b_wanted_amount;
        let token_a_amount = offer_state.token_a_offered_amount;
        let bump = offer_state.bump;
        let offer_id = offer_state.id;

        // VULNERABILITY [MEDIUM]: Missing explicit borrow drop before CPI
        //
        // The offer_data borrow is not explicitly dropped before CPIs.
        // In Pinocchio, if the runtime modifies account data during a CPI while
        // we still hold a borrow, it causes a runtime panic (borrow conflict).
        // This is a correctness issue that can DOS the instruction.
        //
        // The code below happens to work because offer_data goes out of scope
        // before the CPIs, but relying on implicit drop is fragile.
        // A future code change could accidentally hold the borrow across a CPI.
        //
        // Fix: drop(offer_data); // Explicitly drop before any CPI
        drop(offer_data);


        // 2: Create Proposer's ATA B if Needed
        if self.accounts.proposer_ata_b.is_data_empty() {
            pinocchio_associated_token_account::instructions::Create {
                account: self.accounts.proposer_ata_b,
                funding_account: self.accounts.taker,
                mint: self.accounts.token_mint_b,
                token_program: self.accounts.token_program,
                system_program: self.accounts.system_program,
                wallet: self.accounts.proposer,
            }
            .invoke()?;
        }

        // 3: Transfer Token B from Taker to Proposer
        TransferChecked {
            from: self.accounts.taker_ata_b,
            to: self.accounts.proposer_ata_b,
            authority: self.accounts.taker,
            mint: self.accounts.token_mint_b,
            amount: token_b_amount,
            decimals: Mint::from_account_view(self.accounts.token_mint_b)?.decimals(),
        }
        .invoke()?;


        // 4: Prepare PDA Signer
        let bump_binding = [bump];
        let seeds = [
            Seed::from(MakeState::SEED_PREFIX),
            Seed::from(self.accounts.proposer.address().as_array()),
            Seed::from(&offer_id),
            Seed::from(&bump_binding),
        ];
        let signer = Signer::from(&seeds);


        // 5: Transfer Token A from Vault to Taker
        let vault_amount = TokenAccount::from_account_view(self.accounts.vault)?.amount();
        let transfer_amount = vault_amount.min(token_a_amount);

        TransferChecked {
            from: self.accounts.vault,
            to: self.accounts.taker_ata_a,
            authority: self.accounts.offer,
            mint: self.accounts.token_mint_a,
            amount: transfer_amount,
            decimals: Mint::from_account_view(self.accounts.token_mint_a)?.decimals(),
        }
        .invoke_signed(&[signer.clone()])?;


        // 6: Close Vault Account
        CloseAccount {
            account: self.accounts.vault,
            destination: self.accounts.proposer,
            authority: self.accounts.offer,
        }
        .invoke_signed(&[signer])?;

        // 7: Close Offer Account
        {
            let mut offer_data = self.accounts.offer.try_borrow_mut()?;
            offer_data[0] = 0xff;
        }

        let lamports = self.accounts.offer.lamports();
        self.accounts.taker.set_lamports(
            self.accounts.taker.lamports().saturating_add(lamports)
        );

        // Zero out offer lamports
        self.accounts.offer.set_lamports(0);

        // Resize account to 0 bytes
        self.accounts.offer.resize(0)?;

        // Close the account
        self.accounts.offer.close()?;

        Ok(())
    }
}
