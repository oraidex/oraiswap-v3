use cosmwasm_std::coins;
use decimal::*;

use crate::{
    liquidity::Liquidity,
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    tests::helper::{macros::*, MockApp, FEE_DENOM},
    token_amount::TokenAmount,
    FeeTier, PoolKey, MAX_SQRT_PRICE,
};
use oraiswap_v3_common::error::ContractError;

#[test]
fn test_basic_slippage() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = create_dex!(app, Percentage::from_scale(1, 2), alice);

    let mint_amount = 10u128.pow(23);
    let (token_x, token_y) = create_tokens!(app, mint_amount, mint_amount, alice);

    let pool_key = init_slippage_pool_with_liquidity!(app, dex, token_x, token_y, alice);
    let amount = 10u128.pow(8);
    let swap_amount = TokenAmount::new(amount);
    approve!(app, token_x, dex, amount, alice).unwrap();

    let target_sqrt_price = SqrtPrice::new(1009940000000000000000001);
    swap!(
        app,
        dex,
        pool_key,
        false,
        swap_amount,
        true,
        target_sqrt_price,
        alice
    )
    .unwrap();
    let expected_sqrt_price = SqrtPrice::new(1009940000000000000000000);
    let pool = get_pool!(app, dex, token_x, token_y, pool_key.fee_tier).unwrap();

    assert_eq!(expected_sqrt_price, pool.sqrt_price);
}

#[test]
fn test_swap_close_to_limit() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, Percentage::from_scale(1, 2), alice);
    let mint_amount = 10u128.pow(23);
    let (token_x, token_y) = create_tokens!(app, mint_amount, mint_amount, alice);
    let pool_key = init_slippage_pool_with_liquidity!(app, dex, token_x, token_y, alice);
    let amount = 10u128.pow(8);
    let swap_amount = TokenAmount::new(amount);
    approve!(app, token_x, dex, amount, alice).unwrap();

    let target_sqrt_price = SqrtPrice::new(MAX_SQRT_PRICE);
    let quoted_target_sqrt_price = quote!(
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

    let target_sqrt_price = quoted_target_sqrt_price - SqrtPrice::new(1);

    let error = swap!(
        app,
        dex,
        pool_key,
        false,
        swap_amount,
        true,
        target_sqrt_price,
        alice
    )
    .unwrap_err();

    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::PriceLimitReached {}.to_string()));
}

#[test]
fn test_swap_exact_limit() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let dex = create_dex!(app, Percentage::from_scale(1, 2), alice);
    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount, alice);
    init_basic_pool!(app, dex, token_x, token_y, alice);
    init_basic_position!(app, dex, token_x, token_y, alice);

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let amount = 1000;

    mint!(app, token_x, bob, amount, alice).unwrap();
    let amount_x = balance_of!(app, token_x, bob);
    assert_eq!(amount_x, amount);
    approve!(app, token_x, dex, amount, bob).unwrap();

    let swap_amount = TokenAmount::new(amount);
    swap_exact_limit!(app, dex, pool_key, true, swap_amount, true, bob);
}
