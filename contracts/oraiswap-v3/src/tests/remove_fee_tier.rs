use cosmwasm_std::coins;
use decimal::*;

use crate::{
    percentage::Percentage,
    tests::helper::{macros::*, MockApp, FEE_DENOM},
    ContractError, FeeTier,
};

#[test]
fn test_remove_fee_tier() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = create_dex!(app, Percentage::new(0), alice);

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap();
    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    remove_fee_tier!(app, dex, fee_tier, alice).unwrap();
    let exist = fee_tier_exist!(
        app,
        dex,
        FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap()
    );
    assert!(!exist);
}

#[test]
fn test_remove_not_existing_fee_tier() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = create_dex!(app, Percentage::new(0), alice);

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap();
    let error = remove_fee_tier!(app, dex, fee_tier, alice).unwrap_err();

    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::FeeTierNotFound {}.to_string()));
}

#[test]
fn test_remove_fee_tier_not_admin() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let dex = create_dex!(app, Percentage::new(0), alice);

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap();
    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let error = remove_fee_tier!(app, dex, fee_tier, bob).unwrap_err();
    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::Unauthorized {}.to_string()));
}
