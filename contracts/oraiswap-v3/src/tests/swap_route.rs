use cosmwasm_std::coins;
use decimal::*;
use oraiswap_v3_common::{
    error::ContractError,
    interface::SwapHop,
    math::{
        liquidity::Liquidity, percentage::Percentage, sqrt_price::calculate_sqrt_price,
        token_amount::TokenAmount,
    },
    storage::{FeeTier, PoolKey, PoolStatus},
};

use crate::tests::helper::{macros::*, MockApp, FEE_DENOM};

#[test]
fn swap_route_with_pool_status() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(20);

    let dex = create_dex!(app, protocol_fee, alice);

    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    approve!(app, token_x, dex, initial_amount, alice).unwrap();
    approve!(app, token_y, dex, initial_amount, alice).unwrap();
    approve!(app, token_z, dex, initial_amount, alice).unwrap();

    let amount = 1000000000;
    mint!(app, token_x, bob, amount, alice).unwrap();
    approve!(app, token_x, dex, amount, bob).unwrap();
    approve!(app, token_y, dex, initial_amount, bob).unwrap();

    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();

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

    create_pool!(
        app,
        dex,
        token_y,
        token_z,
        fee_tier,
        init_sqrt_price,
        init_tick,
        alice
    )
    .unwrap();

    let pool_key_1 = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let pool_key_2 = PoolKey::new(token_y.to_string(), token_z.to_string(), fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(2u128.pow(63) - 1);

    let pool_1 = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let slippage_limit_lower = pool_1.sqrt_price;
    let slippage_limit_upper = pool_1.sqrt_price;
    create_position!(
        app,
        dex,
        pool_key_1,
        -1,
        1,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        alice
    )
    .unwrap();

    let pool_2 = get_pool!(app, dex, token_y, token_z, fee_tier).unwrap();
    let slippage_limit_lower = pool_2.sqrt_price;
    let slippage_limit_upper = pool_2.sqrt_price;
    create_position!(
        app,
        dex,
        pool_key_2,
        -1,
        1,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        alice
    )
    .unwrap();

    let amount_in = TokenAmount(1000);
    let slippage = Percentage::new(0);
    let swaps = vec![
        SwapHop {
            pool_key: pool_key_1.clone(),
            x_to_y: true,
        },
        SwapHop {
            pool_key: pool_key_2,
            x_to_y: true,
        },
    ];

    let expected_token_amount = quote_route!(app, dex, amount_in, swaps.clone()).unwrap();

    // case 1: pool status = None => can swap
    swap_route!(
        app,
        dex,
        amount_in,
        expected_token_amount,
        slippage,
        swaps.clone(),
        bob
    )
    .unwrap();

    // case 2: poolStatus = None => can swap
    app.update_pool_status(&alice, dex.as_str(), &pool_key_1, Some(PoolStatus::Opening))
        .unwrap();
    swap_route!(
        app,
        dex,
        amount_in,
        expected_token_amount,
        slippage,
        swaps.clone(),
        bob
    )
    .unwrap();

    // case 3: poolStatus = Paused => cannot swap
    app.update_pool_status(&alice, dex.as_str(), &pool_key_1, Some(PoolStatus::Paused))
        .unwrap();
    let error = swap_route!(
        app,
        dex,
        amount_in,
        expected_token_amount,
        slippage,
        swaps.clone(),
        bob
    )
    .unwrap_err();
    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::PoolPaused {}.to_string()));

    // case 4: poolStatus = swapOnly => can swap
    app.update_pool_status(
        &alice,
        dex.as_str(),
        &pool_key_1,
        Some(PoolStatus::SwapOnly),
    )
    .unwrap();
    swap_route!(
        app,
        dex,
        amount_in,
        expected_token_amount,
        slippage,
        swaps.clone(),
        bob
    )
    .unwrap();

    // case 5: poolStatus = lpOnly => cannot swap
    app.update_pool_status(&alice, dex.as_str(), &pool_key_1, Some(PoolStatus::LpOnly))
        .unwrap();
    let error = swap_route!(
        app,
        dex,
        amount_in,
        expected_token_amount,
        slippage,
        swaps.clone(),
        bob
    )
    .unwrap_err();
    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::PoolPaused {}.to_string()));
}

#[test]
fn swap_route() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);

    let dex = create_dex!(app, protocol_fee, alice);

    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    approve!(app, token_x, dex, initial_amount, alice).unwrap();
    approve!(app, token_y, dex, initial_amount, alice).unwrap();
    approve!(app, token_z, dex, initial_amount, alice).unwrap();

    let amount = 1000;
    mint!(app, token_x, bob, amount, alice).unwrap();
    approve!(app, token_x, dex, amount, bob).unwrap();
    approve!(app, token_y, dex, initial_amount, bob).unwrap();

    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();

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

    create_pool!(
        app,
        dex,
        token_y,
        token_z,
        fee_tier,
        init_sqrt_price,
        init_tick,
        alice
    )
    .unwrap();

    let pool_key_1 = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let pool_key_2 = PoolKey::new(token_y.to_string(), token_z.to_string(), fee_tier).unwrap();

    let liquidity_delta = Liquidity::new(2u128.pow(63) - 1);

    let pool_1 = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let slippage_limit_lower = pool_1.sqrt_price;
    let slippage_limit_upper = pool_1.sqrt_price;
    create_position!(
        app,
        dex,
        pool_key_1,
        -1,
        1,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        alice
    )
    .unwrap();

    let pool_2 = get_pool!(app, dex, token_y, token_z, fee_tier).unwrap();
    let slippage_limit_lower = pool_2.sqrt_price;
    let slippage_limit_upper = pool_2.sqrt_price;
    create_position!(
        app,
        dex,
        pool_key_2,
        -1,
        1,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        alice
    )
    .unwrap();

    let amount_in = TokenAmount(1000);
    let slippage = Percentage::new(0);
    let swaps = vec![
        SwapHop {
            pool_key: pool_key_1,
            x_to_y: true,
        },
        SwapHop {
            pool_key: pool_key_2,
            x_to_y: true,
        },
    ];

    let expected_token_amount = quote_route!(app, dex, amount_in, swaps.clone()).unwrap();

    swap_route!(
        app,
        dex,
        amount_in,
        expected_token_amount,
        slippage,
        swaps.clone(),
        bob
    )
    .unwrap();

    let bob_amount_x = balance_of!(app, token_x, bob);
    let bob_amount_y = balance_of!(app, token_y, bob);
    let bob_amount_z = balance_of!(app, token_z, bob);

    assert_eq!(bob_amount_x, 0);
    assert_eq!(bob_amount_y, 0);
    assert_eq!(bob_amount_z, 986);

    let pool_1_after = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool_1_after.fee_protocol_token_x, TokenAmount(1));
    assert_eq!(pool_1_after.fee_protocol_token_y, TokenAmount(0));

    let pool_2_after = get_pool!(app, dex, token_y, token_z, fee_tier).unwrap();
    assert_eq!(pool_2_after.fee_protocol_token_x, TokenAmount(1));
    assert_eq!(pool_2_after.fee_protocol_token_y, TokenAmount(0));

    let alice_amount_x_before = balance_of!(app, token_x, alice);
    let alice_amount_y_before = balance_of!(app, token_y, alice);
    let alice_amount_z_before = balance_of!(app, token_z, alice);

    claim_fee!(app, dex, 0, alice).unwrap();
    claim_fee!(app, dex, 1, alice).unwrap();

    let alice_amount_x_after = balance_of!(app, token_x, alice);
    let alice_amount_y_after = balance_of!(app, token_y, alice);
    let alice_amount_z_after = balance_of!(app, token_z, alice);

    assert_eq!(alice_amount_x_after - alice_amount_x_before, 4);
    assert_eq!(alice_amount_y_after - alice_amount_y_before, 4);
    assert_eq!(alice_amount_z_after - alice_amount_z_before, 0);
}
