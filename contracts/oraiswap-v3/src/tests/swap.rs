use cosmwasm_std::coins;
use decimal::{Decimal, Factories};

use crate::tests::helper::{macros::*, FEE_DENOM};
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
    storage::{FeeTier, PoolKey, PoolStatus},
};

use super::helper::MockApp;

#[test]
fn test_swap_with_pool_status() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let protocol_fee = Percentage::from_scale(6, 3);

    let dex = create_dex!(app, protocol_fee, alice);

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();
    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

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

    let lower_tick_index = -20;
    let middle_tick_index = -10;
    let upper_tick_index = 10;

    let liquidity_delta = Liquidity::from_integer(1000000000);

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index - 20,
        middle_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.liquidity, liquidity_delta);

    let amount = 1000;
    let mint_amount = 1000000;
    let swap_amount = TokenAmount(amount);

    mint!(app, token_x, bob, mint_amount, alice).unwrap();
    approve!(app, token_x, dex, mint_amount, bob).unwrap();

    let slippage = SqrtPrice::new(MIN_SQRT_PRICE);
    let target_sqrt_price = quote!(app, dex, pool_key, true, swap_amount, true, slippage)
        .unwrap()
        .target_sqrt_price;

    // case 1: pool status = None => can swap
    swap!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        target_sqrt_price,
        bob
    )
    .unwrap();

    // case 2: poolStatus = None => can swap
    let target_sqrt_price = quote!(app, dex, pool_key, true, swap_amount, true, slippage)
        .unwrap()
        .target_sqrt_price;
    app.update_pool_status(&alice, dex.as_str(), &pool_key, Some(PoolStatus::Opening))
        .unwrap();
    swap!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        target_sqrt_price,
        bob
    )
    .unwrap();

    // case 3: poolStatus = Paused => cannot swap
    let target_sqrt_price = quote!(app, dex, pool_key, true, swap_amount, true, slippage)
        .unwrap()
        .target_sqrt_price;
    app.update_pool_status(&alice, dex.as_str(), &pool_key, Some(PoolStatus::Paused))
        .unwrap();
    let error = swap!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        target_sqrt_price,
        bob
    )
    .unwrap_err();
    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::PoolPaused {}.to_string()));

    // case 4: poolStatus = swapOnly => can swap
    let target_sqrt_price = quote!(app, dex, pool_key, true, swap_amount, true, slippage)
        .unwrap()
        .target_sqrt_price;
    app.update_pool_status(&alice, dex.as_str(), &pool_key, Some(PoolStatus::SwapOnly))
        .unwrap();
    swap!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        target_sqrt_price,
        bob
    )
    .unwrap();

    // case 5: poolStatus = lpOnly => cannot swap
    let target_sqrt_price = quote!(app, dex, pool_key, true, swap_amount, true, slippage)
        .unwrap()
        .target_sqrt_price;
    app.update_pool_status(&alice, dex.as_str(), &pool_key, Some(PoolStatus::LpOnly))
        .unwrap();
    let error = swap!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        target_sqrt_price,
        bob
    )
    .unwrap_err();
    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::PoolPaused {}.to_string()));
}

#[test]
fn test_swap_x_to_y() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let protocol_fee = Percentage::from_scale(6, 3);

    let dex = create_dex!(app, protocol_fee, alice);

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();
    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

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

    let lower_tick_index = -20;
    let middle_tick_index = -10;
    let upper_tick_index = 10;

    let liquidity_delta = Liquidity::from_integer(1000000);

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index - 20,
        middle_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.liquidity, liquidity_delta);

    let amount = 1000;
    let swap_amount = TokenAmount(amount);

    mint!(app, token_x, bob, amount, alice).unwrap();
    approve!(app, token_x, dex, amount, bob).unwrap();

    let slippage = SqrtPrice::new(MIN_SQRT_PRICE);
    let target_sqrt_price = quote!(app, dex, pool_key, true, swap_amount, true, slippage)
        .unwrap()
        .target_sqrt_price;

    let before_dex_x = balance_of!(app, token_x, dex);
    let before_dex_y = balance_of!(app, token_y, dex);

    swap!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        target_sqrt_price,
        bob
    )
    .unwrap();

    // Load states
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let lower_tick = get_tick!(app, dex, pool_key, lower_tick_index).unwrap();
    let middle_tick = get_tick!(app, dex, pool_key, middle_tick_index).unwrap();
    let upper_tick = get_tick!(app, dex, pool_key, upper_tick_index).unwrap();
    let lower_tick_bit = is_tick_initialized!(app, dex, pool_key, lower_tick_index);
    let middle_tick_bit = is_tick_initialized!(app, dex, pool_key, middle_tick_index);
    let upper_tick_bit = is_tick_initialized!(app, dex, pool_key, upper_tick_index);
    let bob_x = balance_of!(app, token_x, bob);
    let bob_y = balance_of!(app, token_y, bob);
    let dex_x = balance_of!(app, token_x, dex);
    let dex_y = balance_of!(app, token_y, dex);
    let delta_dex_y = before_dex_y - dex_y;
    let delta_dex_x = dex_x - before_dex_x;
    let expected_y = amount - 10;
    let expected_x = 0u128;

    // Check balances
    assert_eq!(bob_x, expected_x);
    assert_eq!(bob_y, expected_y);
    assert_eq!(delta_dex_x, amount);
    assert_eq!(delta_dex_y, expected_y);

    // Check Pool
    assert_eq!(pool.fee_growth_global_y, FeeGrowth::new(0));
    assert_eq!(
        pool.fee_growth_global_x,
        FeeGrowth::new(40000000000000000000000)
    );
    assert_eq!(pool.fee_protocol_token_y, TokenAmount(0));
    assert_eq!(pool.fee_protocol_token_x, TokenAmount(2));

    // Check Ticks
    assert_eq!(lower_tick.liquidity_change, liquidity_delta);
    assert_eq!(middle_tick.liquidity_change, liquidity_delta);
    assert_eq!(upper_tick.liquidity_change, liquidity_delta);
    assert_eq!(upper_tick.fee_growth_outside_x, FeeGrowth::new(0));
    assert_eq!(
        middle_tick.fee_growth_outside_x,
        FeeGrowth::new(30000000000000000000000)
    );
    assert_eq!(lower_tick.fee_growth_outside_x, FeeGrowth::new(0));
    assert!(lower_tick_bit);
    assert!(middle_tick_bit);
    assert!(upper_tick_bit);
}

#[test]
fn test_swap_y_to_x() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let protocol_fee = Percentage::from_scale(6, 3);

    let dex = create_dex!(app, protocol_fee, alice);
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();
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

    let lower_tick_index = -10;
    let middle_tick_index = 10;
    let upper_tick_index = 20;

    let liquidity_delta = Liquidity::from_integer(1000000);

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        middle_tick_index,
        upper_tick_index + 20,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.liquidity, liquidity_delta);

    let amount = 1000;
    let swap_amount = TokenAmount(amount);

    mint!(app, token_y, bob, amount, alice).unwrap();
    approve!(app, token_y, dex, amount, bob).unwrap();

    let target_sqrt_price = SqrtPrice::new(MAX_SQRT_PRICE);

    let target_sqrt_price = quote!(
        app,
        dex,
        pool_key,
        false,
        swap_amount,
        true,
        target_sqrt_price
    )
    .unwrap()
    .target_sqrt_price;

    let before_dex_x = balance_of!(app, token_x, dex);
    let before_dex_y = balance_of!(app, token_y, dex);

    swap!(
        app,
        dex,
        pool_key,
        false,
        swap_amount,
        true,
        target_sqrt_price,
        bob
    )
    .unwrap();

    // Load states
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let lower_tick = get_tick!(app, dex, pool_key, lower_tick_index).unwrap();
    let middle_tick = get_tick!(app, dex, pool_key, middle_tick_index).unwrap();
    let upper_tick = get_tick!(app, dex, pool_key, upper_tick_index).unwrap();
    let lower_tick_bit = is_tick_initialized!(app, dex, pool_key, lower_tick_index);
    let middle_tick_bit = is_tick_initialized!(app, dex, pool_key, middle_tick_index);
    let upper_tick_bit = is_tick_initialized!(app, dex, pool_key, upper_tick_index);
    let bob_x = balance_of!(app, token_x, bob);
    let bob_y = balance_of!(app, token_y, bob);
    let dex_x = balance_of!(app, token_x, dex);
    let dex_y = balance_of!(app, token_y, dex);
    let delta_dex_x = before_dex_x - dex_x;
    let delta_dex_y = dex_y - before_dex_y;
    let expected_x = amount - 10;
    let expected_y = 0;

    // Check balances
    assert_eq!(bob_x, expected_x);
    assert_eq!(bob_y, expected_y);
    assert_eq!(delta_dex_x, expected_x);
    assert_eq!(delta_dex_y, amount);

    // Check Pool
    assert_eq!(pool.fee_growth_global_x, FeeGrowth::new(0));
    assert_eq!(
        pool.fee_growth_global_y,
        FeeGrowth::new(40000000000000000000000)
    );
    assert_eq!(pool.fee_protocol_token_x, TokenAmount(0));
    assert_eq!(pool.fee_protocol_token_y, TokenAmount(2));

    // Check Ticks
    assert_eq!(lower_tick.liquidity_change, liquidity_delta);
    assert_eq!(middle_tick.liquidity_change, liquidity_delta);
    assert_eq!(upper_tick.liquidity_change, liquidity_delta);
    assert_eq!(upper_tick.fee_growth_outside_y, FeeGrowth::new(0));
    assert_eq!(
        middle_tick.fee_growth_outside_y,
        FeeGrowth::new(30000000000000000000000)
    );
    assert_eq!(lower_tick.fee_growth_outside_y, FeeGrowth::new(0));
    assert!(lower_tick_bit);
    assert!(middle_tick_bit);
    assert!(upper_tick_bit);
}

#[test]
fn test_swap_not_enough_liquidity_token_x() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let protocol_fee: Percentage = Percentage::from_scale(6, 3);

    let dex = create_dex!(app, protocol_fee, alice);

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();

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

    let lower_tick_index = -10;
    let middle_tick_index = 10;
    let upper_tick_index = 20;

    let liquidity_delta = Liquidity::from_integer(1000000);

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        middle_tick_index,
        upper_tick_index + 20,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.liquidity, liquidity_delta);

    let amount = 1000;
    let swap_amount = TokenAmount(amount);
    mint!(app, token_x, bob, amount, alice).unwrap();
    approve!(app, token_x, dex, amount, bob).unwrap();

    let target_sqrt_price = SqrtPrice::new(MIN_SQRT_PRICE);

    let error = swap!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        target_sqrt_price,
        bob
    )
    .unwrap_err();

    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::TickLimitReached {}.to_string()));
}

#[test]
fn test_swap_not_enough_liquidity_token_y() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let protocol_fee = Percentage::from_scale(6, 3);

    let dex = create_dex!(app, protocol_fee, alice);
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();

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

    let lower_tick_index = -20;
    let middle_tick_index = -10;
    let upper_tick_index = 10;

    let liquidity_delta = Liquidity::from_integer(1000000);

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index - 20,
        middle_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        alice
    )
    .unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.liquidity, liquidity_delta);

    let amount = 1000;
    let swap_amount = TokenAmount(amount);

    mint!(app, token_y, bob, amount, alice).unwrap();
    approve!(app, token_y, dex, amount, bob).unwrap();

    let slippage = SqrtPrice::new(MAX_SQRT_PRICE);

    let error = swap!(app, dex, pool_key, false, swap_amount, true, slippage, bob).unwrap_err();
    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::TickLimitReached {}.to_string()));
}
