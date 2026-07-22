#![cfg(test)]
 
use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
 
/// Spins up a fresh env, a mock USDC-style token, and an initialized
/// ShelfTrust contract with a 500-unit deposit and a 14-day loan period.
fn setup<'a>() -> (
    Env,
    ShelfTrustClient<'a>,
    Address,      // admin
    Address,      // treasury
    token::Client<'a>,
    token::StellarAssetClient<'a>,
) {
    let env = Env::default();
    env.mock_all_auths();
 
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin = Address::generate(&env);
 
    // Register a Stellar Asset Contract to stand in for USDC in tests.
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = sac.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);
    let token_client = token::Client::new(&env, &token_address);
 
    let contract_id = env.register_contract(None, ShelfTrust);
    let client = ShelfTrustClient::new(&env, &contract_id);
 
    let deposit_amount: i128 = 500; // e.g. 5.00 USDC at 2 decimals, illustrative
    let loan_period_secs: u64 = 14 * 24 * 60 * 60; // 14 days
    client.initialize(
        &admin,
        &token_address,
        &deposit_amount,
        &loan_period_secs,
        &treasury,
    );
 
    (env, client, admin, treasury, token_client, token_admin_client)
}
 
#[test]
fn test_happy_path_checkout_and_return() {
    let (env, client, _admin, _treasury, token_client, token_admin_client) = setup();
    let borrower = Address::generate(&env);
    token_admin_client.mint(&borrower, &1000);
 
    let book_id = Symbol::new(&env, "book1");
    client.checkout_book(&borrower, &book_id);
 
    // Deposit should have left the borrower's wallet into escrow.
    assert_eq!(token_client.balance(&borrower), 500);
 
    client.return_book(&book_id);
 
    // On-time return refunds the full deposit.
    assert_eq!(token_client.balance(&borrower), 1000);
}
 
#[test]
fn test_edge_case_double_checkout_fails() {
    let (env, client, _admin, _treasury, _token_client, token_admin_client) = setup();
    let borrower = Address::generate(&env);
    token_admin_client.mint(&borrower, &1000);
 
    let book_id = Symbol::new(&env, "book1");
    client.checkout_book(&borrower, &book_id);
 
    let result = client.try_checkout_book(&borrower, &book_id);
    assert_eq!(result, Err(Ok(Error::AlreadyCheckedOut)));
}
 
#[test]
fn test_state_after_checkout() {
    let (env, client, _admin, _treasury, _token_client, token_admin_client) = setup();
    let borrower = Address::generate(&env);
    token_admin_client.mint(&borrower, &1000);
 
    let book_id = Symbol::new(&env, "book1");
    client.checkout_book(&borrower, &book_id);
 
    let loan = client.get_loan(&book_id).unwrap();
    assert_eq!(loan.borrower, borrower);
    assert_eq!(loan.deposit_amount, 500);
    assert_eq!(loan.status, LoanStatus::Active);
}
 
#[test]
fn test_overdue_claim_moves_deposit_to_treasury() {
    let (env, client, _admin, treasury, token_client, token_admin_client) = setup();
    let borrower = Address::generate(&env);
    token_admin_client.mint(&borrower, &1000);
 
    let book_id = Symbol::new(&env, "book1");
    client.checkout_book(&borrower, &book_id);
 
    // Fast-forward past the 14-day loan period.
    let past_due = env.ledger().timestamp() + 15 * 24 * 60 * 60;
    env.ledger().set_timestamp(past_due);
 
    client.claim_overdue(&book_id);
 
    assert_eq!(token_client.balance(&treasury), 500);
    let loan = client.get_loan(&book_id).unwrap();
    assert_eq!(loan.status, LoanStatus::Forfeited);
}
 
#[test]
fn test_return_after_already_closed_fails() {
    let (env, client, _admin, _treasury, _token_client, token_admin_client) = setup();
    let borrower = Address::generate(&env);
    token_admin_client.mint(&borrower, &1000);
 
    let book_id = Symbol::new(&env, "book1");
    client.checkout_book(&borrower, &book_id);
    client.return_book(&book_id);
 
    let result = client.try_return_book(&book_id);
    assert_eq!(result, Err(Ok(Error::AlreadyClosed)));
}
 