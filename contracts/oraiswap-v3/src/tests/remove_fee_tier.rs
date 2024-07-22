use decimal::*;

use crate::{
    percentage::Percentage,
    tests::helper::{macros::*, MockApp},
    ContractError, FeeTier,
};

#[test]
fn test_remove_fee_tier() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    remove_fee_tier!(app, dex, fee_tier, "alice").unwrap();
    let exist = fee_tier_exist!(
        app,
        dex,
        FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap()
    );
    assert!(!exist);
}

#[test]
fn test_remove_not_existing_fee_tier() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap();
    let error = remove_fee_tier!(app, dex, fee_tier, "alice").unwrap_err();

    assert_eq!(
        error.root_cause().to_string(),
        ContractError::FeeTierNotFound {}.to_string()
    );
}

#[test]
fn test_remove_fee_tier_not_admin() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 2).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let error = remove_fee_tier!(app, dex, fee_tier, "bob").unwrap_err();
    assert_eq!(
        error.root_cause().to_string(),
        ContractError::Unauthorized {}.to_string()
    );
}
