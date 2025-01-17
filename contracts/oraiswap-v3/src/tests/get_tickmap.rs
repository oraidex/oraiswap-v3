use cosmwasm_std::coins;
use decimal::{Decimal, Factories};
use oraiswap_v3_common::math::liquidity::Liquidity;
use oraiswap_v3_common::math::percentage::Percentage;
use oraiswap_v3_common::math::sqrt_price::{
    calculate_sqrt_price, get_max_tick, get_min_tick, SqrtPrice,
};
use oraiswap_v3_common::storage::{get_max_chunk, FeeTier, PoolKey};

use crate::tests::helper::FEE_DENOM;
use crate::tests::helper::{macros::*, MockApp};

#[test]
fn test_get_tickmap() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = create_dex!(app, Percentage::new(0), alice);
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    approve!(app, token_x, dex, initial_amount, alice).unwrap();
    approve!(app, token_y, dex, initial_amount, alice).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
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

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let liquidity_delta = Liquidity::new(1000);

    create_position!(
        app,
        dex,
        pool_key,
        -58,
        5,
        liquidity_delta,
        pool.sqrt_price,
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();

    assert_eq!(
        tickmap[0],
        (
            3465,
            0b1000000000000000000000000000000000000000000000000000000000000001
        )
    );
    assert_eq!(tickmap.len(), 1);
}

#[test]
fn test_get_tickmap_tick_spacing_over_one() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, Percentage::new(0), alice);
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    approve!(app, token_x, dex, initial_amount, alice).unwrap();
    approve!(app, token_y, dex, initial_amount, alice).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 10).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
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

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(1000);

    create_position!(
        app,
        dex,
        pool_key,
        10,
        20,
        liquidity_delta,
        pool.sqrt_price,
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        liquidity_delta,
        pool.sqrt_price,
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();

    assert_eq!(tickmap[0], (0, 0b1));
    assert_eq!(
        tickmap[1],
        (346, 0b1100000000000000000000000000000000000000)
    );
    assert_eq!(
        tickmap[2],
        (get_max_chunk(fee_tier.tick_spacing), 0b10000000000)
    );
    assert_eq!(tickmap.len(), 3);
}

#[test]
fn test_get_tickmap_edge_ticks_intialized() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, Percentage::new(0), alice);
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    approve!(app, token_x, dex, initial_amount, alice).unwrap();
    approve!(app, token_y, dex, initial_amount, alice).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
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

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(1000);

    create_position!(
        app,
        dex,
        pool_key,
        -221818,
        -221817,
        liquidity_delta,
        pool.sqrt_price,
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        221817,
        221818,
        liquidity_delta,
        pool.sqrt_price,
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();

    assert_eq!(tickmap[0], (0, 0b11));
    assert_eq!(
        tickmap[1],
        (
            get_max_chunk(fee_tier.tick_spacing),
            0b11000000000000000000000000000000000000000000000000000
        )
    );
    assert_eq!(tickmap.len(), 2);
    {
        let tickmap: Vec<(u16, u64)> = get_tickmap!(
            app,
            dex,
            pool_key,
            get_min_tick(fee_tier.tick_spacing),
            get_max_tick(fee_tier.tick_spacing),
            false
        )
        .unwrap();
        assert_eq!(tickmap[0], (0, 0b11));
        assert_eq!(
            tickmap[1],
            (
                get_max_chunk(fee_tier.tick_spacing),
                0b11000000000000000000000000000000000000000000000000000
            )
        );
        assert_eq!(tickmap.len(), 2);

        let tickmap: Vec<(u16, u64)> = get_tickmap!(
            app,
            dex,
            pool_key,
            get_min_tick(fee_tier.tick_spacing),
            get_max_tick(fee_tier.tick_spacing),
            false
        )
        .unwrap();
        assert_eq!(tickmap[0], (0, 0b11));
        assert_eq!(
            tickmap[1],
            (
                get_max_chunk(fee_tier.tick_spacing),
                0b11000000000000000000000000000000000000000000000000000
            )
        );
        assert_eq!(tickmap.len(), 2);

        let tickmap: Vec<(u16, u64)> = get_tickmap!(
            app,
            dex,
            pool_key,
            get_min_tick(fee_tier.tick_spacing),
            get_max_tick(fee_tier.tick_spacing),
            true
        )
        .unwrap();
        assert_eq!(tickmap[1], (0, 0b11));
        assert_eq!(
            tickmap[0],
            (
                get_max_chunk(fee_tier.tick_spacing),
                0b11000000000000000000000000000000000000000000000000000
            )
        );
        assert_eq!(tickmap.len(), 2);
    }
}

#[test]
fn test_get_tickmap_more_chunks_above() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, Percentage::new(0), alice);
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    approve!(app, token_x, dex, initial_amount, alice).unwrap();
    approve!(app, token_y, dex, initial_amount, alice).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
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

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(1000);

    for i in (6..52500).step_by(64) {
        create_position!(
            app,
            dex,
            pool_key,
            i,
            i + 1,
            liquidity_delta,
            pool.sqrt_price,
            SqrtPrice::max_instance(),
            alice
        )
        .unwrap();
    }

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();

    for (i, _) in (0..tickmap.len()).enumerate() {
        let current = 3466 + i as u16;
        assert_eq!(tickmap[i], (current, 0b11));
    }
}

#[test]
fn test_get_tickmap_more_chunks_below() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, Percentage::new(0), alice);
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    approve!(app, token_x, dex, initial_amount, alice).unwrap();
    approve!(app, token_y, dex, initial_amount, alice).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
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

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(1000);

    for i in (-52544..6).step_by(64) {
        create_position!(
            app,
            dex,
            pool_key,
            i,
            i + 1,
            liquidity_delta,
            pool.sqrt_price,
            SqrtPrice::max_instance(),
            alice
        )
        .unwrap();
    }

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();
    for (i, _) in (0..tickmap.len()).enumerate() {
        let current = 2644 + i as u16;
        assert_eq!(
            tickmap[i],
            (
                current,
                0b110000000000000000000000000000000000000000000000000000000000
            )
        );
    }
}

#[test]
fn test_get_tickmap_max_chunks_returned() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, Percentage::new(0), alice);
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    approve!(app, token_x, dex, initial_amount, alice).unwrap();
    approve!(app, token_y, dex, initial_amount, alice).unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 1).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let init_tick = -200_000;
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

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(1000);

    for i in (0..104832).step_by(64) {
        create_position!(
            app,
            dex,
            pool_key,
            i,
            i + 1,
            liquidity_delta,
            pool.sqrt_price,
            SqrtPrice::max_instance(),
            alice
        )
        .unwrap();
    }

    let tickmap: Vec<(u16, u64)> = get_tickmap!(
        app,
        dex,
        pool_key,
        get_min_tick(fee_tier.tick_spacing),
        get_max_tick(fee_tier.tick_spacing),
        false
    )
    .unwrap();

    assert_eq!(tickmap.len(), 1638);
}
