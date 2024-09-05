use cosmwasm_std::coins;
use cosmwasm_std::Addr;
use decimal::{Decimal, Factories};
use oraiswap_v3_common::interface::PositionTick;
use oraiswap_v3_common::math::fee_growth::FeeGrowth;
use oraiswap_v3_common::math::liquidity::Liquidity;
use oraiswap_v3_common::math::percentage::Percentage;
use oraiswap_v3_common::math::sqrt_price::calculate_sqrt_price;
use oraiswap_v3_common::math::sqrt_price::SqrtPrice;
use oraiswap_v3_common::math::token_amount::TokenAmount;
use oraiswap_v3_common::storage::FeeTier;
use oraiswap_v3_common::storage::PoolKey;
use oraiswap_v3_common::storage::Position;
use oraiswap_v3_common::storage::POSITION_TICK_LIMIT;

use crate::tests::helper::FEE_DENOM;
use crate::tests::helper::{macros::*, MockApp};

#[test]
fn test_get_position_ticks() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = app.create_dex(alice, Percentage::from_scale(1, 2)).unwrap();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    let fee_tier = FeeTier::new(Percentage::from_scale(1, 2), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        alice
    )
    .unwrap();

    approve!(app, token_x, dex, 500, alice).unwrap();
    approve!(app, token_y, dex, 500, alice).unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    create_position!(
        app,
        dex,
        pool_key,
        -10,
        10,
        Liquidity::new(10),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let result: Vec<PositionTick> =
        get_position_ticks!(app, dex, Addr::unchecked(alice), 0).unwrap();
    assert_eq!(result.len(), 2);

    let lower_tick = get_tick!(app, dex, pool_key, -10).unwrap();
    let upper_tick = get_tick!(app, dex, pool_key, 10).unwrap();

    position_tick_equals!(result[0], lower_tick);
    position_tick_equals!(result[1], upper_tick);
}

#[test]
fn test_get_position_ticks_limit() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = app.create_dex(alice, Percentage::from_scale(1, 2)).unwrap();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    let fee_tier = FeeTier::new(Percentage::from_scale(1, 2), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        alice
    )
    .unwrap();

    approve!(app, token_x, dex, initial_amount, alice).unwrap();
    approve!(app, token_y, dex, initial_amount, alice).unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    for i in 1..=POSITION_TICK_LIMIT / 2 {
        create_position!(
            app,
            dex,
            pool_key,
            -(i as i32),
            i as i32,
            Liquidity::new(10),
            SqrtPrice::new(0),
            SqrtPrice::max_instance(),
            alice
        )
        .unwrap();
    }

    let result: Vec<PositionTick> =
        get_position_ticks!(app, dex, Addr::unchecked(alice), 0).unwrap();
    assert_eq!(result.len(), POSITION_TICK_LIMIT);

    for i in 1..=POSITION_TICK_LIMIT / 2 {
        let lower_tick = get_tick!(app, dex, pool_key, -(i as i32)).unwrap();
        let upper_tick = get_tick!(app, dex, pool_key, i as i32).unwrap();

        position_tick_equals!(result[i * 2 - 2], lower_tick);
        position_tick_equals!(result[i * 2 - 1], upper_tick);
    }
}

#[test]
fn test_get_position_ticks_with_offset() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = app.create_dex(alice, Percentage::from_scale(1, 2)).unwrap();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    let fee_tier_1 = FeeTier::new(Percentage::from_scale(1, 2), 2).unwrap();
    let fee_tier_2 = FeeTier::new(Percentage::from_scale(1, 2), 10).unwrap();

    add_fee_tier!(app, dex, fee_tier_1, alice).unwrap();
    add_fee_tier!(app, dex, fee_tier_2, alice).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier_1,
        init_sqrt_price,
        init_tick,
        alice
    )
    .unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier_2,
        init_sqrt_price,
        init_tick,
        alice
    )
    .unwrap();

    approve!(app, token_x, dex, initial_amount, alice).unwrap();
    approve!(app, token_y, dex, initial_amount, alice).unwrap();

    let pool_key_1 = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier_1).unwrap();
    create_position!(
        app,
        dex,
        pool_key_1,
        -10,
        30,
        Liquidity::new(10),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let pool_key_2 = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier_2).unwrap();
    create_position!(
        app,
        dex,
        pool_key_2,
        -20,
        40,
        Liquidity::new(10),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let result_1: Vec<PositionTick> =
        get_position_ticks!(app, dex, Addr::unchecked(alice), 0).unwrap();
    assert_eq!(result_1.len(), 4);

    let result_2: Vec<PositionTick> =
        get_position_ticks!(app, dex, Addr::unchecked(alice), 1).unwrap();
    assert_eq!(result_2.len(), 2);

    assert_eq!(result_1[2], result_2[0]);
    assert_eq!(result_1[3], result_2[1]);
}

#[test]
fn test_query_all_positions() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = app.create_dex(alice, Percentage::from_scale(1, 2)).unwrap();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    let fee_tier = FeeTier::new(Percentage::from_scale(1, 2), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        alice
    )
    .unwrap();

    approve!(app, token_x, dex, 500, alice).unwrap();
    approve!(app, token_y, dex, 500, alice).unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    create_position!(
        app,
        dex,
        pool_key,
        -10,
        10,
        Liquidity::new(10),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();
    create_position!(
        app,
        dex,
        pool_key,
        -100,
        100,
        Liquidity::new(10),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let positions = app.query_all_positions(dex.as_str(), None, None).unwrap();
    assert_eq!(positions.len(), 2);
    assert_eq!(
        positions,
        [
            Position {
                pool_key: PoolKey {
                    token_x: token_x.to_string(),
                    token_y: token_y.to_string(),
                    fee_tier: FeeTier {
                        fee: Percentage(10000000000),
                        tick_spacing: 1
                    }
                },
                liquidity: Liquidity(10),
                lower_tick_index: -10,
                upper_tick_index: 10,
                fee_growth_inside_x: FeeGrowth(0),
                fee_growth_inside_y: FeeGrowth(0),
                last_block_number: positions[0].last_block_number,
                tokens_owed_x: TokenAmount(0),
                tokens_owed_y: TokenAmount(0),
                approvals: vec![],
                token_id: 1,
                incentives: vec![]
            },
            Position {
                pool_key: PoolKey {
                    token_x: token_x.to_string(),
                    token_y: token_y.to_string(),
                    fee_tier: FeeTier {
                        fee: Percentage(10000000000),
                        tick_spacing: 1
                    }
                },
                liquidity: Liquidity(10),
                lower_tick_index: -100,
                upper_tick_index: 100,
                fee_growth_inside_x: FeeGrowth(0),
                fee_growth_inside_y: FeeGrowth(0),
                last_block_number: positions[1].last_block_number,
                tokens_owed_x: TokenAmount(0),
                tokens_owed_y: TokenAmount(0),
                token_id: 2,
                approvals: vec![],
                incentives: vec![]
            }
        ]
    )
}
