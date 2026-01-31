use core::mem::{transmute, size_of};

use pinocchio::{
    AccountView, ProgramResult, cpi::Seed, cpi::Signer, error::ProgramError,
    sysvars::{Sysvar, rent::Rent}
};
use pinocchio_token::{instructions::TransferChecked, state::Mint};

use crate::state::MakeState;


// Account context for the Offer instruction
//
// This struct defines all accounts involved in creating an escrow offer.
//
// Remember proposer is Sarah in this context


pub struct OfferAccounts<'a> {
    // The person creating the escrow offer (Sarah)
    pub maker: &'a AccountView,

    // Token mint A - what Sarah is offering
    pub token_mint_a: &'a AccountView,

    // Token mint B - what Sarah wants in return
    pub token_mint_b: &'a AccountView,

    // Sarah's associated token account for Token A (source of funds)
    pub maker_ata_a: &'a AccountView,

    // The escrow offer PDA account (will be created)
    pub offer: &'a AccountView,

    // The vault ATA that will hold the escrowed tokens (will be created) until deal is done
    pub vault: &'a AccountView,
    pub token_program: &'a AccountView,
    pub system_program: &'a AccountView,
}


impl<'a> TryFrom<&'a [AccountView]> for OfferAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {

        let [maker, token_mint_a, token_mint_b, maker_ata_a, offer, vault, token_program, system_program, _] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };
        //Refer to root readme for details on the vulnerabilities and the meaning of each level

        // VULNERABILITY [CRITICAL]: Missing signer check on maker
        //
        // The maker account is not verified as a transaction signer.
        // Anyone can call ProposeOffer using someone else's pubkey as the maker,
        // creating escrow offers on behalf of victims and locking their tokens.
        //
        // Example:
        //   Attacker passes Alice's pubkey as maker without Alice signing.
        //   The program creates an escrow with Alice's ATA as the source.
        //   Alice's tokens get transferred to the vault without her consent.
        //
        // Fix: if !maker.is_signer() { return Err(ProgramError::MissingRequiredSignature); }


        // VULNERABILITY [CRITICAL]: Missing token mint ownership validation
        //
        // Neither token_mint_a nor token_mint_b are verified to be owned by the SPL Token program.
        // An attacker can pass fake mint accounts owned by their own program, with fabricated
        // data (arbitrary supply, decimals, authority).
        //
        // Example:
        //   Attacker creates a fake mint account owned by their malicious program.
        //   Fake mint reports decimals=0, supply=999999999.
        //   The escrow accepts it as a real token mint.
        //
        // Fix: if !token_mint_a.owned_by(token_program.address()) { return Err(...); }


        // VULNERABILITY [CRITICAL]: Missing maker ATA ownership validation
        //
        // The maker's token account (maker_ata_a) is not checked for:
        // - Ownership by SPL Token program (could be a fake account)
        // - Correct data size (could have malformed data)
        //
        // An attacker can pass any account as the maker's ATA, including accounts
        // owned by a malicious program with crafted data showing fake balances.
        //
        // Example:
        //   Attacker creates an account with fake token data showing balance of 1 billion.
        //   Program reads this fake balance and proceeds with the escrow.
        //
        // Fix: if !maker_ata_a.owned_by(token_program.address()) { return Err(...); }
        // Fix: if maker_ata_a.data_len() != TokenAccount::LEN { return Err(...); }


        // VULNERABILITY [HIGH]: Missing maker ATA address derivation check
        //
        // The maker's ATA address is not derived and compared to the passed account.
        // An attacker can pass any token account (not necessarily the maker's ATA)
        // as the source of funds.
        //
        // Example:
        //   Attacker passes a different user's ATA as maker_ata_a.
        //   If combined with the missing signer check, the attacker can drain
        //   anyone's token account into an escrow they control.
        //
        // Fix: Derive expected ATA with find_program_address and compare.


        // VULNERABILITY [MEDIUM]: Missing offer uninitialized check
        //
        // The offer PDA is not checked to be empty before initialization.
        // If the account already contains data, its state will be silently overwritten,
        // destroying the previous escrow and potentially locking funds in the old vault.
        //
        // Example:
        //   Sarah creates offer #1 with 100 USDC in vault.
        //   Attacker re-initializes offer #1 with new data.
        //   Old vault with 100 USDC is now orphaned, funds permanently locked.
        //
        // Fix: if !offer.is_data_empty() { return Err(ProgramError::AccountAlreadyInitialized); }


        // VULNERABILITY [LOW]: Missing offer writable check
        //
        // The offer account is not checked for writability before initialization.
        // While the runtime would catch this during the actual write, checking upfront
        // provides a clearer error and prevents wasted compute units.
        //
        // Fix: if !offer.is_writable() { return Err(ProgramError::InvalidAccountData); }


        // VULNERABILITY [HIGH]: Missing vault ATA address derivation check
        //
        // The vault ATA address is not derived and compared to the passed account.
        // An attacker can pass any writable token account as the vault, redirecting
        // escrowed tokens to an account they control.
        //
        // Example:
        //   Attacker passes their own ATA as the vault account.
        //   Maker's tokens get transferred directly to the attacker's wallet.
        //   The escrow PDA points to a vault the attacker owns.
        //
        // Fix: Derive expected vault ATA with find_program_address and compare.


        // VULNERABILITY [MEDIUM]: Missing vault uninitialized and writable checks
        //
        // The vault account is not verified to be empty or writable.
        // If the vault already exists with tokens from a previous escrow,
        // new tokens get deposited into it, mixing funds from different offers.
        //
        // Fix: if !vault.is_data_empty() { return Err(ProgramError::AccountAlreadyInitialized); }
        // Fix: if !vault.is_writable() { return Err(ProgramError::InvalidAccountData); }


        // No validations - all accounts accepted as-is
        Ok(Self {
            maker,
            token_mint_a,
            token_mint_b,
            maker_ata_a,
            offer,
            vault,
            token_program,
            system_program,
        })
    }
}


// Instruction data for proposing an escrow offer

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProposalOfferData {
    // Unique identifier for this escrow (used in PDA derivation)
    pub id: [u8; 8],

    // Amount of Token B the maker wants to receive
    pub token_b_wanted_amount: u64,

    // Amount of Token A the maker is offering
    pub token_a_offered_amount: u64,

    pub bump: u8,
}

impl ProposalOfferData {
    pub const LEN: usize = core::mem::size_of::<ProposalOfferData>();
}

impl<'a> TryFrom<&'a [u8]> for ProposalOfferData {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(unsafe {
            transmute(
                TryInto::<[u8; size_of::<ProposalOfferData>()]>::try_into(data)
                    .map_err(|_| ProgramError::InvalidInstructionData)?,
            )
        })
    }
}


pub struct ProposeOfferInstruction<'a> {
    pub accounts: OfferAccounts<'a>,
    pub data: ProposalOfferData,
}

impl<'a> TryFrom<(&'a [AccountView], &'a [u8])> for ProposeOfferInstruction<'a> {
    type Error = ProgramError;

    fn try_from(
        (accounts, data): (&'a [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        let accounts = OfferAccounts::try_from(accounts)?;
        let data = ProposalOfferData::try_from(data)?;

        Ok(Self { accounts, data })
    }
}


// INSTRUCTION HANDLER


impl<'a> ProposeOfferInstruction<'a> {

    pub fn handler(&self) -> ProgramResult {

        // VULNERABILITY [HIGH]: Missing offer PDA address verification
        //
        // The handler does not derive the canonical PDA and compare it to the
        // provided offer account. An attacker can pass any account as the offer,
        // meaning the program will write escrow state to an arbitrary account.
        // Combined with missing ownership checks, the attacker controls where state lives.
        //
        // Additionally, the bump used for PDA signing is taken from instruction data
        // without verifying it is the canonical bump from find_program_address.
        // Non-canonical bumps can create multiple valid PDAs for the same seeds,
        // enabling state duplication and confusion attacks.
        //
        // Example:
        //   Attacker provides bump=253 instead of canonical bump=255.
        //   Two different PDAs now exist for the same maker+id combination.
        //   The attacker can create conflicting escrow states.
        //
        // Fix: Derive PDA with find_program_address and compare address + use returned bump.

        // Use bump from instruction data directly (not verified as canonical)
        let bump = self.data.bump;

        // Calculate rent for offer account
        let rent = Rent::get()?;
        let space = MakeState::LEN;
        let lamports = rent.try_minimum_balance(space)?;


        // Create the offer PDA account
        pinocchio_system::instructions::CreateAccount {
            from: self.accounts.maker,
            to: self.accounts.offer,
            space: space as u64,
            lamports,
            owner: &crate::ID,
        }
        .invoke_signed(&[Signer::from(&[
            Seed::from(MakeState::SEED_PREFIX),
            Seed::from(self.accounts.maker.address().as_array()),
            Seed::from(&self.data.id),
            Seed::from(&[bump]),
        ])])?;


        // Initialize the offer state
        {
            let mut offer_data = self.accounts.offer.try_borrow_mut()?;
            let offer_state = MakeState::load_mut(&mut offer_data)?;

            offer_state.set_inner(
                self.data.id,
                *self.accounts.maker.address(),
                *self.accounts.token_mint_a.address(),
                *self.accounts.token_mint_b.address(),
                self.data.token_b_wanted_amount,
                self.data.token_a_offered_amount,
                bump,
            );
        }


        // Create the vault ATA
        pinocchio_associated_token_account::instructions::Create {
            account: self.accounts.vault,
            funding_account: self.accounts.maker,
            mint: self.accounts.token_mint_a,
            token_program: self.accounts.token_program,
            system_program: self.accounts.system_program,
            wallet: self.accounts.offer,
        }
        .invoke()?;


        // Transfer tokens from maker to vault
        TransferChecked {
            from: self.accounts.maker_ata_a,
            to: self.accounts.vault,
            authority: self.accounts.maker,
            mint: self.accounts.token_mint_a,
            amount: self.data.token_a_offered_amount,
            decimals: Mint::from_account_view(self.accounts.token_mint_a)?.decimals(),
        }
        .invoke()?;

        Ok(())
    }
}
