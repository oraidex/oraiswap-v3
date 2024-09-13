use cosmwasm_std::Addr;
use decimal::*;

use oraiswap_v3_common::math::liquidity::Liquidity;
use oraiswap_v3_common::math::percentage::Percentage;
use oraiswap_v3_common::math::sqrt_price::{calculate_sqrt_price, SqrtPrice};
use oraiswap_v3_common::storage::{FeeTier, PoolKey};

use crate::tests::helper::macros::*;
use crate::tests::helper::MockApp;

pub fn init_basic_v3_pool(
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
    approve!(app, token_z, config.dex_v3, initial_amount, user).unwrap();
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
    let upper_tick_index = 20;

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
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        admin
    )
    .unwrap();
}
