
#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, Env, Symbol,
};
 
/// Storage layout for the contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,          // the librarian / NGO account that operates the contract
    TokenAddress,   // the deposit token (e.g. a USDC Stellar Asset Contract)
    DepositAmount,  // fixed refundable deposit required per checkout
    LoanPeriod,     // loan duration in seconds before a book counts as overdue
    Treasury,       // where forfeited (overdue) deposits are sent
    Loan(Symbol),   // book_id -> Loan record
}
 
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum LoanStatus {
    Active,
    Returned,
    Forfeited,
}
 
#[contracttype]
#[derive(Clone)]
pub struct Loan {
    pub borrower: Address,
    pub checkout_time: u64,
    pub due_time: u64,
    pub deposit_amount: i128,
    pub status: LoanStatus,
}
 
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyCheckedOut = 2,
    NoActiveLoan = 3,
    NotOverdue = 4,
    AlreadyClosed = 5,
}
 
#[contract]
pub struct ShelfTrust;
 
#[contractimpl]
impl ShelfTrust {
    /// One-time setup: who administers the library contract, which token
    /// deposits are held in, how large the refundable deposit is, how long a
    /// loan period lasts, and where forfeited deposits (overdue penalties) go.
    pub fn initialize(
        env: Env,
        admin: Address,
        token_address: Address,
        deposit_amount: i128,
        loan_period_secs: u64,
        treasury: Address,
    ) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::TokenAddress, &token_address);
        env.storage()
            .instance()
            .set(&DataKey::DepositAmount, &deposit_amount);
        env.storage()
            .instance()
            .set(&DataKey::LoanPeriod, &loan_period_secs);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
    }
 
    /// Borrower checks out a book: their refundable deposit is pulled from
    /// their wallet into the contract's escrow balance, and a loan record is
    /// created with a due date. This on-chain lock replaces the paper ledger
    /// Ana used to rely on.
    pub fn checkout_book(env: Env, borrower: Address, book_id: Symbol) -> Result<(), Error> {
        borrower.require_auth();
 
        let key = DataKey::Loan(book_id.clone());
        if let Some(existing) = env.storage().persistent().get::<DataKey, Loan>(&key) {
            if existing.status == LoanStatus::Active {
                return Err(Error::AlreadyCheckedOut);
            }
        }
 
        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .ok_or(Error::NotInitialized)?;
        let deposit_amount: i128 = env
            .storage()
            .instance()
            .get(&DataKey::DepositAmount)
            .ok_or(Error::NotInitialized)?;
        let loan_period: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LoanPeriod)
            .ok_or(Error::NotInitialized)?;
 
        // Pull the deposit from the borrower into this contract's balance.
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&borrower, &env.current_contract_address(), &deposit_amount);
 
        let now = env.ledger().timestamp();
        let loan = Loan {
            borrower: borrower.clone(),
            checkout_time: now,
            due_time: now + loan_period,
            deposit_amount,
            status: LoanStatus::Active,
        };
        env.storage().persistent().set(&key, &loan);
        Ok(())
    }
 
    /// Librarian confirms the physical book has been returned. If it's on or
    /// before the due date, the full deposit is released back to the
    /// borrower — the moment that makes a deposit model usable for patrons
    /// who can't afford to have funds locked up indefinitely.
    pub fn return_book(env: Env, book_id: Symbol) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
 
        let key = DataKey::Loan(book_id.clone());
        let mut loan: Loan = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NoActiveLoan)?;
        if loan.status != LoanStatus::Active {
            return Err(Error::AlreadyClosed);
        }
 
        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .ok_or(Error::NotInitialized)?;
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(
            &env.current_contract_address(),
            &loan.borrower,
            &loan.deposit_amount,
        );
 
        loan.status = LoanStatus::Returned;
        env.storage().persistent().set(&key, &loan);
        Ok(())
    }
 
    /// If a book is overdue and never returned, the librarian can claim the
    /// forfeited deposit into the library's treasury to cover the cost of
    /// replacing the lost book.
    pub fn claim_overdue(env: Env, book_id: Symbol) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
 
        let key = DataKey::Loan(book_id.clone());
        let mut loan: Loan = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NoActiveLoan)?;
        if loan.status != LoanStatus::Active {
            return Err(Error::AlreadyClosed);
        }
 
        let now = env.ledger().timestamp();
        if now <= loan.due_time {
            return Err(Error::NotOverdue);
        }
 
        let treasury: Address = env
            .storage()
            .instance()
            .get(&DataKey::Treasury)
            .ok_or(Error::NotInitialized)?;
        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .ok_or(Error::NotInitialized)?;
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(
            &env.current_contract_address(),
            &treasury,
            &loan.deposit_amount,
        );
 
        loan.status = LoanStatus::Forfeited;
        env.storage().persistent().set(&key, &loan);
        Ok(())
    }
 
    /// Read-only lookup so the app UI can show a book's current loan state.
    pub fn get_loan(env: Env, book_id: Symbol) -> Option<Loan> {
        env.storage().persistent().get(&DataKey::Loan(book_id))
    }
}
 
#[cfg(test)]
mod test;
 