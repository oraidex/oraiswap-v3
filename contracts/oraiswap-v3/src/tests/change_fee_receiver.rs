use crate::math::types::percentage::Percentage;
use crate::math::types::sqrt_price::calculate_sqrt_price;
use crate::tests::helper::macros::*;
use crate::tests::helper::MockApp;
use crate::tests::helper::FEE_DENOM;
use crate::{FeeTier, PoolKey};
use cosmwasm_std::coins;
use decimal::Decimal;
use oraiswap_v3_common::error::ContractError;

#[test]
fn test_change_fee_reciever() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = create_dex!(app, Percentage::new(0), alice);
    let (token_x, token_y) = create_tokens!(app, 500, alice);

    let fee_tier = FeeTier::new(Percentage::new(1), 1).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let result = create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        alice
    );
    assert!(result.is_ok());

    let pool_key =
        PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier.clone()).unwrap();
    let result = change_fee_receiver!(app, dex, pool_key, alice, alice);
    assert!(result.is_ok());

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.fee_receiver.as_str(), alice);
}

#[test]
fn test_not_admin_change_fee_reciever() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];

    let dex = create_dex!(app, Percentage::new(0), alice);
    let (token_x, token_y) = create_tokens!(app, 500, alice);

    let fee_tier = FeeTier::new(Percentage::new(1), 100).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    let result = add_fee_tier!(app, dex, fee_tier, alice);
    assert!(result.is_ok());

    let result = create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        alice
    );
    assert!(result.is_ok());

    let pool_key =
        PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier.clone()).unwrap();
    let error = change_fee_receiver!(app, dex, pool_key, bob, bob).unwrap_err();

    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::Unauthorized {}.to_string()));
}
