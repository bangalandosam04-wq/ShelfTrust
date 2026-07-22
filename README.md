# ShelfTrust

A deposit-escrow book checkout system for community libraries, built on Stellar with Soroban smart contracts.

## Problem

Ana, a volunteer librarian at a rural community library in Cebu, Philippines, has no way to enforce book returns beyond a paper sign-out sheet. Roughly 30% of borrowed books never come back, forcing the library to spend its small budget replacing lost titles instead of buying new ones.

## Solution

A borrower scans a book's QR code to check it out, which locks a small refundable USDC deposit into a Soroban smart contract as on-chain collateral. When the librarian confirms the physical return before the due date, the contract instantly releases the deposit back to the borrower. If the book is never returned, the librarian can claim the forfeited deposit to help replace it. Stellar's near-zero fees and fast settlement make it practical to escrow even $1–2 deposits — something that isn't viable on higher-fee chains or through a traditional bank.

## Timeline

- **Day 1:** Contract design, storage schema, initialize/checkout/return functions
- **Day 2:** Overdue claim logic, error handling, test suite
- **Day 3:** QR-based mobile front end, testnet deployment, demo polish

## Stellar Features Used

- USDC transfers (deposit escrow)
- Soroban smart contracts (checkout/return/overdue logic)
- Trustlines (borrower wallets holding USDC without a bank account)

## Vision and Purpose

Community libraries in low-connectivity regions lose a meaningful share of their collection every year to books that simply never come back, with no practical way to enforce accountability. ShelfTrust replaces the paper sign-out sheet with a lightweight financial commitment that borrowers get back automatically when they do the right thing — turning "trust" into something the library doesn't have to hope for.

## Prerequisites

- Rust (stable, edition 2021)
- Soroban CLI v21.x or later
- A funded Stellar testnet account (for deployment)

## Build

```bash
soroban contract build
```

## Test

```bash
cargo test
```

## Deploy to Testnet

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/shelf_trust.wasm \
  --source <YOUR_SECRET_KEY> \
  --network testnet
```

## Sample CLI Invocation

Initialize the contract (run once by the librarian/admin):

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <ADMIN_SECRET_KEY> \
  --network testnet \
  -- \
  initialize \
  --admin <ADMIN_ADDRESS> \
  --token_address <USDC_SAC_ADDRESS> \
  --deposit_amount 500 \
  --loan_period_secs 1209600 \
  --treasury <TREASURY_ADDRESS>
```

Check out a book (called by the borrower's wallet):

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <BORROWER_SECRET_KEY> \
  --network testnet \
  -- \
  checkout_book \
  --borrower <BORROWER_ADDRESS> \
  --book_id book1
```
Link

https://stellar.expert/explorer/testnet/tx/0d7d05ad77d750faf0deebf7d0feaf69a69cf52b7c5263c4bd4509656a883475
https://lab.stellar.org/r/testnet/contract/CCRZGYLN4JDRM57DSRWETTHMSCKOLFL2UUJBQ7GZAJXES2WKRWDN7YHM

MIT
