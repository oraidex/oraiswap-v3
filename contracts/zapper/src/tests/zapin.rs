use cosmwasm_std::{coins, Addr, Uint128};
use decimal::*;
use oraiswap::mixed_router::SwapOperation;
use oraiswap_v3_common::asset::{Asset, AssetInfo};
use oraiswap_v3_common::math::liquidity::Liquidity;
use oraiswap_v3_common::math::percentage::Percentage;
use oraiswap_v3_common::math::sqrt_price::{calculate_sqrt_price, SqrtPrice};
use oraiswap_v3_common::storage::{FeeTier, PoolKey};

use crate::tests::helper::MockApp;
use crate::tests::helper::{macros::*, FEE_DENOM};

fn init_basic_v3_pool(
    app: &mut MockApp,
    zapper: &Addr,
    token_x: &Addr,
    token_y: &Addr,
    token_z: &Addr,
    admin: &str,
    user: &str,
) {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(20);

    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    approve!(app, token_x, config.dex_v3, initial_amount, admin).unwrap();
    approve!(app, token_y, config.dex_v3, initial_amount, admin).unwrap();
    approve!(app, token_z, config.dex_v3, initial_amount, admin).unwrap();

    mint!(app, token_x, user, initial_amount, admin).unwrap();
    mint!(app, token_y, user, initial_amount, admin).unwrap();
    mint!(app, token_z, user, initial_amount, admin).unwrap();
    approve!(app, token_x, config.dex_v3, initial_amount, user).unwrap();
    approve!(app, token_y, config.dex_v3, initial_amount, user).unwrap();
    approve!(app, token_z, zapper, initial_amount, user).unwrap();
    approve!(app, token_x, zapper, initial_amount, user).unwrap();
    approve!(app, token_y, zapper, initial_amount, user).unwrap();
    approve!(app, token_z, zapper, initial_amount, user).unwrap();

    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();

    add_fee_tier!(app, config.dex_v3, fee_tier, admin).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        config.dex_v3,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        admin
    )
    .unwrap();

    create_pool!(
        app,
        config.dex_v3,
        token_y,
        token_z,
        fee_tier,
        init_sqrt_price,
        init_tick,
        admin
    )
    .unwrap();

    let pool_key_1 = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let pool_key_2 = PoolKey::new(token_y.to_string(), token_z.to_string(), fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(2u128.pow(63) - 1);

    let lower_tick_index = -20;
    let middle_tick_index = -10;
    let upper_tick_index = 10;

    create_position!(
        app,
        config.dex_v3,
        pool_key_1,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        admin
    )
    .unwrap();

    create_position!(
        app,
        config.dex_v3,
        pool_key_2,
        lower_tick_index - 20,
        middle_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        admin
    )
    .unwrap();
}
#[test]
fn test_zap_in() {
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

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key_x_y = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let pool_key_y_z = PoolKey::new(token_y.to_string(), token_z.to_string(), fee_tier).unwrap();

    let tick_lower_index = 0;
    let tick_upper_index = 10;
    let asset_in = Asset {
        info: AssetInfo::Token {
            contract_addr: token_x,
        },
        amount: Uint128::new(1000000),
    };

    app.zap_in_liquidity(
        &bob,
        zapper.as_str(),
        pool_key_x_y.clone(),
        tick_lower_index,
        tick_upper_index,
        &asset_in,
        Uint128::new(500000),
        Uint128::new(500000),
        None,
        Some(vec![SwapOperation::SwapV3 {
            pool_key: pool_key_x_y,
            x_to_y: true,
        }]),
        None,
        None,
    )
    .unwrap();
}
