use cosmwasm_std::{coins, Decimal as StdDecimal, Uint128};
use decimal::*;
use oraiswap::mixed_router::SwapOperation;
use oraiswap_v3_common::asset::{Asset, AssetInfo};
use oraiswap_v3_common::error::ContractError;
use oraiswap_v3_common::math::liquidity::Liquidity;
use oraiswap_v3_common::math::percentage::Percentage;

use oraiswap_v3_common::math::sqrt_price::SqrtPrice;
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
    let liquidity_delta = Liquidity::new(10u128.pow(10) - 1);

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
    assert!(err.root_cause().to_string().contains("not found"));
}

#[test]
fn zap_out_position_not_approval_first() {}

#[test]
fn success_zap_out_with_no_routes() {}

#[test]
fn zap_out_position_not_enough_balance_to_swap() {}

#[test]
fn zap_out_position_with_slippage() {}

#[test]
fn zap_out_position_with_routes_success() {}

#[test]
fn zap_out_position_with_fee() {}
