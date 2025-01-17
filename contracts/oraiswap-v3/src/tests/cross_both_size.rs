use cosmwasm_std::coins;
use decimal::{Decimal, Factories};
use oraiswap_v3_common::{
    error::ContractError,
    math::{
        fee_growth::FeeGrowth,
        liquidity::Liquidity,
        percentage::Percentage,
        sqrt_price::{calculate_sqrt_price, SqrtPrice},
        token_amount::TokenAmount,
        MAX_SQRT_PRICE, MIN_SQRT_PRICE,
    },
    storage::{FeeTier, PoolKey},
};

use crate::tests::helper::{macros::*, MockApp, FEE_DENOM};

#[test]
fn test_cross_both_side() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let mint_token = 10u128.pow(5);

    let (dex, token_x, token_y) =
        init_dex_and_tokens!(app, mint_token, Percentage::from_scale(1, 2), alice);

    let pool_key =
        PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier.clone()).unwrap();

    app.add_fee_tier(alice, dex.as_str(), fee_tier.clone())
        .unwrap();

    app.create_pool(
        alice,
        dex.as_str(),
        token_x.as_str(),
        token_y.as_str(),
        fee_tier,
        init_sqrt_price,
        init_tick,
    )
    .unwrap();

    let lower_tick_index = -10;
    let upper_tick_index = 10;

    let mint_amount = 10u128.pow(5);

    app.mint_token(alice, bob, token_x.as_str(), mint_amount)
        .unwrap();
    app.mint_token(alice, alice, token_y.as_str(), mint_amount)
        .unwrap();

    app.approve_token("tokenx", alice, dex.as_str(), mint_amount)
        .unwrap();
    app.approve_token("tokeny", alice, dex.as_str(), mint_amount)
        .unwrap();

    let liquidity_delta = Liquidity::from_integer(20006000);

    let pool_state = app
        .get_pool(dex.as_str(), token_x.as_str(), token_y.as_str(), fee_tier)
        .unwrap();

    app.create_position(
        alice,
        dex.as_str(),
        &pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        pool_state.sqrt_price,
        pool_state.sqrt_price,
    )
    .unwrap();

    app.create_position(
        alice,
        dex.as_str(),
        &pool_key,
        -20,
        lower_tick_index,
        liquidity_delta,
        pool_state.sqrt_price,
        pool_state.sqrt_price,
    )
    .unwrap();

    let pool = app
        .get_pool(dex.as_str(), token_x.as_str(), token_y.as_str(), fee_tier)
        .unwrap();

    assert_eq!(pool.liquidity, liquidity_delta);

    let limit_without_cross_tick_amount = TokenAmount(10_068);
    let not_cross_amount = TokenAmount(1);
    let min_amount_to_cross_from_tick_price = TokenAmount(3);
    let crossing_amount_by_amount_out = TokenAmount(20136101434);

    let mint_amount = limit_without_cross_tick_amount.get()
        + not_cross_amount.get()
        + min_amount_to_cross_from_tick_price.get()
        + crossing_amount_by_amount_out.get();

    app.mint_token(alice, alice, token_x.as_str(), mint_amount)
        .unwrap();
    app.mint_token(alice, alice, token_y.as_str(), mint_amount)
        .unwrap();

    app.approve_token("tokenx", alice, dex.as_str(), mint_amount)
        .unwrap();
    app.approve_token("tokeny", alice, dex.as_str(), mint_amount)
        .unwrap();

    let pool_before = app
        .get_pool(dex.as_str(), token_x.as_str(), token_y.as_str(), fee_tier)
        .unwrap();

    let limit_sqrt_price = SqrtPrice::new(MIN_SQRT_PRICE);

    app.swap(
        alice,
        dex.as_str(),
        &pool_key,
        true,
        limit_without_cross_tick_amount,
        true,
        limit_sqrt_price,
    )
    .unwrap();

    let pool = app
        .get_pool(dex.as_str(), token_x.as_str(), token_y.as_str(), fee_tier)
        .unwrap();

    let expected_tick = -10;
    let expected_price = calculate_sqrt_price(expected_tick).unwrap();

    assert_eq!(pool.current_tick_index, expected_tick);
    assert_eq!(pool.liquidity, pool_before.liquidity);
    assert_eq!(pool.sqrt_price, expected_price);

    app.swap(
        alice,
        dex.as_str(),
        &pool_key,
        true,
        min_amount_to_cross_from_tick_price,
        true,
        limit_sqrt_price,
    )
    .unwrap();

    app.swap(
        alice,
        dex.as_str(),
        &pool_key,
        false,
        min_amount_to_cross_from_tick_price,
        true,
        SqrtPrice::new(MAX_SQRT_PRICE),
    )
    .unwrap();

    let massive_x = 10u128.pow(19);
    let massive_y = 10u128.pow(19);

    app.mint_token(alice, alice, token_x.as_str(), massive_x)
        .unwrap();
    app.mint_token(alice, alice, token_y.as_str(), massive_y)
        .unwrap();

    app.approve_token("tokenx", alice, dex.as_str(), massive_x)
        .unwrap();
    app.approve_token("tokeny", alice, dex.as_str(), massive_y)
        .unwrap();

    let massive_liquidity_delta = Liquidity::from_integer(19996000399699881985603u128);

    app.create_position(
        alice,
        dex.as_str(),
        &pool_key,
        -20,
        0,
        massive_liquidity_delta,
        SqrtPrice::new(MIN_SQRT_PRICE),
        SqrtPrice::new(MAX_SQRT_PRICE),
    )
    .unwrap();

    app.swap(
        alice,
        dex.as_str(),
        &pool_key,
        true,
        TokenAmount(1),
        false,
        limit_sqrt_price,
    )
    .unwrap();

    app.swap(
        alice,
        dex.as_str(),
        &pool_key,
        false,
        TokenAmount(2),
        true,
        SqrtPrice::new(MAX_SQRT_PRICE),
    )
    .unwrap();

    let pool = app
        .get_pool(dex.as_str(), token_x.as_str(), token_y.as_str(), fee_tier)
        .unwrap();

    let expected_liquidity = Liquidity::from_integer(19996000399699901991603u128);
    let expected_liquidity_change_on_last_tick =
        Liquidity::from_integer(19996000399699901991603u128);
    let expected_liquidity_change_on_upper_tick = Liquidity::from_integer(20006000);

    assert_eq!(pool.current_tick_index, -20);
    assert_eq!(
        pool.fee_growth_global_x,
        FeeGrowth::new(29991002699190242927121)
    );
    assert_eq!(pool.fee_growth_global_y, FeeGrowth::new(0));
    assert_eq!(pool.fee_protocol_token_x, TokenAmount(4));
    assert_eq!(pool.fee_protocol_token_y, TokenAmount(2));
    assert_eq!(pool.liquidity, expected_liquidity);
    assert_eq!(pool.sqrt_price, SqrtPrice::new(999500149964999999999999));

    let final_last_tick = app.get_tick(dex.as_str(), &pool_key, -20).unwrap();
    assert_eq!(final_last_tick.fee_growth_outside_x, FeeGrowth::new(0));
    assert_eq!(final_last_tick.fee_growth_outside_y, FeeGrowth::new(0));
    assert_eq!(
        final_last_tick.liquidity_change,
        expected_liquidity_change_on_last_tick
    );

    let final_lower_tick = app.get_tick(dex.as_str(), &pool_key, -10).unwrap();
    assert_eq!(
        final_lower_tick.fee_growth_outside_x,
        FeeGrowth::new(29991002699190242927121)
    );
    assert_eq!(final_lower_tick.fee_growth_outside_y, FeeGrowth::new(0));
    assert_eq!(final_lower_tick.liquidity_change, Liquidity::new(0));

    let final_upper_tick = app.get_tick(dex.as_str(), &pool_key, 10).unwrap();
    assert_eq!(final_upper_tick.fee_growth_outside_x, FeeGrowth::new(0));
    assert_eq!(final_upper_tick.fee_growth_outside_y, FeeGrowth::new(0));
    assert_eq!(
        final_upper_tick.liquidity_change,
        expected_liquidity_change_on_upper_tick
    );
}

#[test]
fn test_cross_both_side_not_cross_case() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let initial_mint = 10u128.pow(10);

    let dex = create_dex!(app, Percentage::from_scale(1, 2), alice);
    let (token_x, token_y) = create_tokens!(app, initial_mint, initial_mint, alice);

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

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

    let lower_tick_index = -10;
    let upper_tick_index = 10;

    let mint_amount = 10u128.pow(5);
    mint!(app, token_x, alice, mint_amount, alice).unwrap();
    mint!(app, token_y, alice, mint_amount, alice).unwrap();

    approve!(app, token_x, dex, mint_amount, alice).unwrap();
    approve!(app, token_y, dex, mint_amount, alice).unwrap();

    let liquidity_delta = Liquidity::new(20006000000000);

    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        pool_state.sqrt_price,
        pool_state.sqrt_price,
        alice
    )
    .unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        -20,
        lower_tick_index,
        liquidity_delta,
        pool_state.sqrt_price,
        pool_state.sqrt_price,
        alice
    )
    .unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    assert_eq!(pool.liquidity, liquidity_delta);

    let limit_without_cross_tick_amount = TokenAmount(10_068);
    let not_cross_amount = TokenAmount(1);
    let min_amount_to_cross_from_tick_price = TokenAmount(3);
    let crossing_amount_by_amount_out = TokenAmount(20136101434);

    let mint_amount = limit_without_cross_tick_amount.get()
        + not_cross_amount.get()
        + min_amount_to_cross_from_tick_price.get()
        + crossing_amount_by_amount_out.get();

    mint!(app, token_x, alice, mint_amount, alice).unwrap();
    mint!(app, token_y, alice, mint_amount, alice).unwrap();

    approve!(app, token_x, dex, mint_amount, alice).unwrap();
    approve!(app, token_y, dex, mint_amount, alice).unwrap();

    let pool_before = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let limit_sqrt_price = SqrtPrice::new(MIN_SQRT_PRICE);

    swap!(
        app,
        dex,
        pool_key,
        true,
        limit_without_cross_tick_amount,
        true,
        limit_sqrt_price,
        alice
    )
    .unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let expected_tick = -10;
    let expected_price = calculate_sqrt_price(expected_tick).unwrap();

    assert_eq!(pool.current_tick_index, expected_tick);
    assert_eq!(pool.liquidity, pool_before.liquidity);
    assert_eq!(pool.sqrt_price, expected_price);

    let slippage = SqrtPrice::new(MIN_SQRT_PRICE);

    let result = swap!(
        app,
        dex,
        pool_key,
        true,
        not_cross_amount,
        true,
        slippage,
        alice
    )
    .unwrap_err();

    assert!(result
        .root_cause()
        .to_string()
        .contains(&ContractError::NoGainSwap.to_string()));
}
