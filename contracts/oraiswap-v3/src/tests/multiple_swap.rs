use cosmwasm_std::coins;

use crate::tests::helper::{macros::*, MockApp, FEE_DENOM};

#[test]
fn test_multiple_swap_x_to_y() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    multiple_swap!(app, true, alice, bob);
}

#[test]
fn test_multiple_swap_y_to_x() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    multiple_swap!(app, false, alice, bob);
}
