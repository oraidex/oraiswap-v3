use std::ops::Add;

use cosmwasm_std::{coins, Decimal as StdDecimal, Uint128};
use decimal::*;
use oraiswap::mixed_router::SwapOperation;
use oraiswap_v3_common::error::ContractError;
use oraiswap_v3_common::math::liquidity::Liquidity;
use oraiswap_v3_common::math::percentage::Percentage;

use oraiswap_v3_common::math::sqrt_price::SqrtPrice;
use oraiswap_v3_common::math::token_amount::TokenAmount;
use oraiswap_v3_common::math::MIN_TICK;
use oraiswap_v3_common::storage::{FeeTier, PoolKey};

use crate::msg::Route;
use crate::tests::common::init_basic_v3_pool;
use crate::tests::helper::MockApp;
use crate::tests::helper::{macros::*, FEE_DENOM};
#[test]
fn zap_out_position_not_exist() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let tick_lower_index = 0;
    let tick_upper_index = 10;
    let liquidity_delta = Liquidity::new(2u128.pow(60) - 1);

    create_position!(
        app,
        config.dex_v3,
        pool_key,
        tick_lower_index,
        tick_upper_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        bob
    )
    .unwrap();

    // zap-out position with index 1 => fail
    let err = app
        .zap_out_liquidity(&bob, zapper.as_str(), 1, vec![])
        .unwrap_err();
    println!("error {:?}", err.root_cause().to_string());

    #[cfg(not(feature="test-tube"))]
    assert!(err.root_cause().to_string().contains("not found"));

    #[cfg(feature="test-tube")]
    assert!(err.root_cause().to_string().contains("Querier contract error"));
}

#[test]
fn zap_out_position_not_approval_first() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let tick_lower_index = 0;
    let tick_upper_index = 10;
    let liquidity_delta = Liquidity::new(2u128.pow(60) - 1);

    create_position!(
        app,
        config.dex_v3,
        pool_key,
        tick_lower_index,
        tick_upper_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        bob
    )
    .unwrap();

    let err = app
        .zap_out_liquidity(&bob, zapper.as_str(), 0, vec![])
        .unwrap_err();
    assert!(err
        .root_cause()
        .to_string()
        .contains(&ContractError::Unauthorized {}.to_string()));
}

#[test]
fn success_zap_out_with_no_routes() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let tick_lower_index = -10;
    let tick_upper_index = 10;
    let liquidity_delta = Liquidity::new(2u128.pow(60) - 1);

    let balance_x_before = balance_of!(app, token_x, bob);
    let balance_y_before = balance_of!(app, token_y, bob);
    let balance_incentive_before = balance_of!(app, token_z, bob);

    create_position!(
        app,
        config.dex_v3,
        pool_key,
        tick_lower_index,
        tick_upper_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        bob
    )
    .unwrap();

    // try increase 1000s
    app.increase_time(1000);

    let all_positions = get_all_positions!(app, config.dex_v3, bob);
    assert_eq!(all_positions.len(), 1);
    app.approve_position(
        &bob,
        config.dex_v3.as_str(),
        zapper.as_str(),
        all_positions[0].token_id,
    )
    .unwrap();

    app.zap_out_liquidity(&bob, zapper.as_str(), 0, vec![])
        .unwrap();

    let balance_x_after = balance_of!(app, token_x, bob);
    let balance_y_after = balance_of!(app, token_y, bob);
    let balance_incentive_after = balance_of!(app, token_z, bob);
    assert!(balance_x_before.abs_diff(balance_x_after).lt(&10u128));
    assert!(balance_y_before.abs_diff(balance_y_after).lt(&10u128));
    assert!(balance_incentive_after.gt(&balance_incentive_before));
}

#[test]
fn zap_out_position_not_enough_balance_to_swap() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let tick_lower_index = -10;
    let tick_upper_index = 10;
    let liquidity_delta = Liquidity::new(2u128.pow(60) - 1);

    let balance_x_before = balance_of!(app, token_x, bob);
    create_position!(
        app,
        config.dex_v3,
        pool_key,
        tick_lower_index,
        tick_upper_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        bob
    )
    .unwrap();
    let balance_x_after = balance_of!(app, token_x, bob);

    let amount_x_to_swap = balance_x_before - balance_x_after;

    let all_positions = get_all_positions!(app, config.dex_v3, bob);
    assert_eq!(all_positions.len(), 1);
    app.approve_position(
        &bob,
        config.dex_v3.as_str(),
        zapper.as_str(),
        all_positions[0].token_id,
    )
    .unwrap();
    // we will swap x_to_y
    let err = app
        .zap_out_liquidity(
            &bob,
            zapper.as_str(),
            0,
            vec![Route {
                token_in: token_x.to_string(),
                offer_amount: Uint128::new(amount_x_to_swap + 1),
                operations: vec![SwapOperation::SwapV3 {
                    pool_key: pool_key.clone(),
                    x_to_y: true,
                }],
                minimum_receive: None,
            }],
        )
        .unwrap_err();
    assert!(err
        .root_cause()
        .to_string()
        .contains(&ContractError::ZapOutNotEnoughBalanceToSwap {}.to_string()));
}

#[test]
fn zap_out_position_with_slippage() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let tick_lower_index = -10;
    let tick_upper_index = 10;
    let liquidity_delta = Liquidity::new(2u128.pow(60) - 1);

    let balance_x_before = balance_of!(app, token_x, bob);
    create_position!(
        app,
        config.dex_v3,
        pool_key,
        tick_lower_index,
        tick_upper_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        bob
    )
    .unwrap();
    let balance_x_after = balance_of!(app, token_x, bob);

    let amount_x_to_swap = balance_x_before - balance_x_after;

    let all_positions = get_all_positions!(app, config.dex_v3, bob);
    assert_eq!(all_positions.len(), 1);
    app.approve_position(
        &bob,
        config.dex_v3.as_str(),
        zapper.as_str(),
        all_positions[0].token_id,
    )
    .unwrap();

    let quote = quote!(
        app,
        config.dex_v3,
        pool_key,
        true,
        TokenAmount::new(amount_x_to_swap - 10),
        true,
        SqrtPrice::from_tick(MIN_TICK).unwrap()
    )
    .unwrap();

    // we will swap x_to_y
    let err = app
        .zap_out_liquidity(
            &bob,
            zapper.as_str(),
            0,
            vec![Route {
                token_in: token_x.to_string(),
                offer_amount: Uint128::new(amount_x_to_swap - 10),
                operations: vec![SwapOperation::SwapV3 {
                    pool_key: pool_key.clone(),
                    x_to_y: true,
                }],
                minimum_receive: Some(Uint128::new(quote.amount_out.0 + 10)),
            }],
        )
        .unwrap_err();

    assert!(err
        .root_cause()
        .to_string()
        .contains("Assertion failed; minimum receive amount:"));
}

#[test]
fn zap_out_position_with_routes_success() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let tick_lower_index = -10;
    let tick_upper_index = 10;
    let liquidity_delta = Liquidity::new(2u128.pow(60) - 1);

    let balance_x_before = balance_of!(app, token_x, bob);
    let balance_y_before = balance_of!(app, token_y, bob);
    create_position!(
        app,
        config.dex_v3,
        pool_key,
        tick_lower_index,
        tick_upper_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        bob
    )
    .unwrap();
    let balance_x_after = balance_of!(app, token_x, bob);
    let amount_x_to_swap = balance_x_before - balance_x_after;

    let all_positions = get_all_positions!(app, config.dex_v3, bob);
    assert_eq!(all_positions.len(), 1);
    app.approve_position(
        &bob,
        config.dex_v3.as_str(),
        zapper.as_str(),
        all_positions[0].token_id,
    )
    .unwrap();

    let balance_x_before = balance_of!(app, token_x, bob);
    // we will swap x_to_y
    app.zap_out_liquidity(
        &bob,
        zapper.as_str(),
        0,
        vec![Route {
            token_in: token_x.to_string(),
            offer_amount: Uint128::new(amount_x_to_swap - 10),
            operations: vec![SwapOperation::SwapV3 {
                pool_key: pool_key.clone(),
                x_to_y: true,
            }],
            minimum_receive: None,
        }],
    )
    .unwrap();
    let balance_x_after = balance_of!(app, token_x, bob);
    let balance_y_after = balance_of!(app, token_y, bob);

    assert!(balance_x_before.abs_diff(balance_x_after).lt(&10u128));
    assert!(balance_y_before.lt(&balance_y_after));
}

#[test]
fn zap_out_position_with_fee() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
        ("charlie", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    // register protocol fee: 0.1%
    app.register_protocol_fee(
        &alice,
        zapper.as_str(),
        StdDecimal::from_ratio(1u128, 10u128),
        &charlie,
    )
    .unwrap();

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let tick_lower_index = -10;
    let tick_upper_index = 10;
    let liquidity_delta = Liquidity::new(2u128.pow(60) - 1);

    let balance_x_before = balance_of!(app, token_x, bob);
    create_position!(
        app,
        config.dex_v3,
        pool_key,
        tick_lower_index,
        tick_upper_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        bob
    )
    .unwrap();
    let balance_x_after = balance_of!(app, token_x, bob);
    let amount_x_to_swap = balance_x_before - balance_x_after - 1;

    let all_positions = get_all_positions!(app, config.dex_v3, bob);
    assert_eq!(all_positions.len(), 1);
    app.approve_position(
        &bob,
        config.dex_v3.as_str(),
        zapper.as_str(),
        all_positions[0].token_id,
    )
    .unwrap();

    let balance_fee_receiver_before = balance_of!(app, token_x, charlie);
    // we will swap x_to_y
    app.zap_out_liquidity(
        &bob,
        zapper.as_str(),
        0,
        vec![Route {
            token_in: token_x.to_string(),
            offer_amount: Uint128::new(amount_x_to_swap),
            operations: vec![SwapOperation::SwapV3 {
                pool_key: pool_key.clone(),
                x_to_y: true,
            }],
            minimum_receive: None,
        }],
    )
    .unwrap();
    let balance_fee_receiver_after = balance_of!(app, token_x, charlie);

    assert_eq!(
        balance_fee_receiver_after,
        balance_fee_receiver_before
            .add((Uint128::new(amount_x_to_swap) * StdDecimal::from_ratio(1u128, 10u128)).u128())
    );
}
