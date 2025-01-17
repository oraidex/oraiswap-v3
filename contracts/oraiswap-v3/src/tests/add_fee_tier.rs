use crate::tests::helper::{macros::*, MockApp, FEE_DENOM};
use cosmwasm_std::coins;
use decimal::Decimal;
use oraiswap_v3_common::{
    error::ContractError, math::percentage::Percentage, oraiswap_v3_msg::QueryMsg, storage::FeeTier,
};

#[test]
fn test_add_multiple_fee_tiers() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, Percentage::new(0), alice);

    let first_fee_tier = FeeTier::new(Percentage::new(1), 1).unwrap();
    let second_fee_tier = FeeTier::new(Percentage::new(1), 2).unwrap();
    let third_fee_tier = FeeTier::new(Percentage::new(1), 4).unwrap();

    add_fee_tier!(app, dex, first_fee_tier, alice).unwrap();
    add_fee_tier!(app, dex, second_fee_tier, alice).unwrap();
    add_fee_tier!(app, dex, third_fee_tier, alice).unwrap();

    let fee_tiers: Vec<FeeTier> = app.query(dex.clone(), &QueryMsg::FeeTiers {}).unwrap();
    assert!(fee_tiers.contains(&first_fee_tier));
    assert!(fee_tiers.contains(&second_fee_tier));
    assert!(fee_tiers.contains(&third_fee_tier));
    assert_eq!(fee_tiers.len(), 3);
}

#[test]
fn test_add_fee_tier_not_admin() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let dex = create_dex!(app, Percentage::new(0), alice);

    let fee_tier = FeeTier::new(Percentage::new(1), 1).unwrap();
    let error = add_fee_tier!(app, dex, fee_tier, bob).unwrap_err();

    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::Unauthorized {}.to_string()));
}

#[test]
fn test_add_fee_tier_zero_fee() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, Percentage::new(0), alice);

    let fee_tier = FeeTier::new(Percentage::new(0), 10).unwrap();
    let result = add_fee_tier!(app, dex, fee_tier, alice);
    assert!(result.is_ok());
}

#[test]
fn test_add_fee_tier_tick_spacing_zero() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, Percentage::new(0), alice);

    let fee_tier = FeeTier {
        fee: Percentage::new(1),
        tick_spacing: 0,
    };
    let error = add_fee_tier!(app, dex, fee_tier, alice).unwrap_err();

    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::InvalidTickSpacing {}.to_string()));
}

#[test]
fn test_add_fee_tier_over_upper_bound_tick_spacing() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, Percentage::new(0), alice);

    let fee_tier = FeeTier {
        fee: Percentage::new(1),
        tick_spacing: 101,
    };
    let error = add_fee_tier!(app, dex, fee_tier, alice).unwrap_err();

    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::InvalidTickSpacing {}.to_string()));
}

#[test]
fn test_add_fee_tier_fee_above_limit() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, Percentage::new(0), alice);

    let fee_tier = FeeTier {
        fee: Percentage::new(1000000000000),
        tick_spacing: 10,
    };
    let error = add_fee_tier!(app, dex, fee_tier, alice).unwrap_err();

    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::InvalidFee {}.to_string()));
}
