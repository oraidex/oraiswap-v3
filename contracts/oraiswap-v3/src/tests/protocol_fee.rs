use cosmwasm_std::{coins, Addr};
use decimal::*;

use crate::tests::helper::{macros::*, MockApp, FEE_DENOM};
use oraiswap_v3_common::{
    error::ContractError,
    math::{percentage::Percentage, token_amount::TokenAmount},
    storage::{FeeTier, PoolKey},
};

#[test]
fn test_protocol_fee() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];

    let (dex, token_x, token_y) = init_dex_and_tokens!(app, alice);
    init_basic_pool!(app, dex, token_x, token_y, alice);
    init_basic_position!(app, dex, token_x, token_y, alice);
    init_basic_swap!(app, dex, token_x, token_y, alice, bob);

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    withdraw_protocol_fee!(app, dex, pool_key, alice).unwrap();

    let amount_x = balance_of!(app, token_x, alice);
    let amount_y = balance_of!(app, token_y, alice);
    assert_eq!(amount_x, 9999999501);
    assert_eq!(amount_y, 9999999000);

    let amount_x = balance_of!(app, token_x, dex);
    let amount_y = balance_of!(app, token_y, dex);
    assert_eq!(amount_x, 1499);
    assert_eq!(amount_y, 7);

    let pool_after_withdraw = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_x,
        TokenAmount::new(0)
    );
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_y,
        TokenAmount::new(0)
    );
}

#[test]
fn test_withdraw_all_protocol_fee() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];

    let (dex, token_x, token_y) = init_dex_and_tokens!(app, alice);
    init_basic_pool!(app, dex, token_x, token_y, alice);
    init_basic_position!(app, dex, token_x, token_y, alice);
    init_basic_swap!(app, dex, token_x, token_y, alice, bob);

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();

    withdraw_all_protocol_fee!(app, dex, None, alice).unwrap();

    let amount_x = balance_of!(app, token_x, alice);
    let amount_y = balance_of!(app, token_y, alice);
    assert_eq!(amount_x, 9999999501);
    assert_eq!(amount_y, 9999999000);

    let amount_x = balance_of!(app, token_x, dex);
    let amount_y = balance_of!(app, token_y, dex);
    assert_eq!(amount_x, 1499);
    assert_eq!(amount_y, 7);

    let pool_after_withdraw = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_x,
        TokenAmount::new(0)
    );
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_y,
        TokenAmount::new(0)
    );
}

#[test]
fn test_withdraw_all_protocol_fee_with_receiver() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
        ("charlie", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    let (dex, token_x, token_y) = init_dex_and_tokens!(app, alice);
    init_basic_pool!(app, dex, token_x, token_y, alice);
    init_basic_position!(app, dex, token_x, token_y, alice);
    init_basic_swap!(app, dex, token_x, token_y, alice, bob);

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();

    withdraw_all_protocol_fee!(app, dex, Some(Addr::unchecked(charlie)), alice).unwrap();

    let amount_x = balance_of!(app, token_x, charlie);
    let amount_y = balance_of!(app, token_y, charlie);
    assert_eq!(amount_x, 1);
    assert_eq!(amount_y, 0);

    let amount_x = balance_of!(app, token_x, dex);
    let amount_y = balance_of!(app, token_y, dex);
    assert_eq!(amount_x, 1499);
    assert_eq!(amount_y, 7);

    let pool_after_withdraw = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_x,
        TokenAmount::new(0)
    );
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_y,
        TokenAmount::new(0)
    );
}

#[test]
fn test_protocol_fee_not_admin() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let (dex, token_x, token_y) = init_dex_and_tokens!(app, alice);
    init_basic_pool!(app, dex, token_x, token_y, alice);
    init_basic_position!(app, dex, token_x, token_y, alice);
    init_basic_swap!(app, dex, token_x, token_y, alice, bob);

    let pool_key = PoolKey::new(
        token_x.to_string(),
        token_y.to_string(),
        FeeTier {
            fee: Percentage::from_scale(6, 3),
            tick_spacing: 10,
        },
    )
    .unwrap();

    let error = withdraw_protocol_fee!(app, dex, pool_key, bob).unwrap_err();
    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::Unauthorized {}.to_string()));
}

#[test]
fn test_withdraw_all_protocol_fee_not_admin() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let (dex, token_x, token_y) = init_dex_and_tokens!(app, alice);
    init_basic_pool!(app, dex, token_x, token_y, alice);
    init_basic_position!(app, dex, token_x, token_y, alice);
    init_basic_swap!(app, dex, token_x, token_y, alice, bob);
    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();

    withdraw_all_protocol_fee!(app, dex, Some(Addr::unchecked(alice)), bob).unwrap();

    let amount_x = balance_of!(app, token_x, alice);
    let amount_y = balance_of!(app, token_y, alice);
    assert_eq!(amount_x, 9999999500);
    assert_eq!(amount_y, 9999999000);

    let pool_after_withdraw = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_x,
        TokenAmount::new(1)
    );
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_y,
        TokenAmount::new(0)
    );
}

#[test]
fn test_withdraw_fee_not_deployer() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let (dex, token_x, token_y) = init_dex_and_tokens!(app, alice);
    init_basic_pool!(app, dex, token_x, token_y, alice);
    init_basic_position!(app, dex, token_x, token_y, alice);
    init_basic_swap!(app, dex, token_x, token_y, alice, bob);

    let user_address = bob;

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    change_fee_receiver!(app, dex, pool_key, user_address, alice).unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.fee_receiver.as_str(), user_address);

    withdraw_protocol_fee!(app, dex, pool_key, bob).unwrap();

    let amount_x = balance_of!(app, token_x, user_address);
    let amount_y = balance_of!(app, token_y, user_address);
    assert_eq!(amount_x, 1);
    assert_eq!(amount_y, 993);

    let amount_x = balance_of!(app, token_x, dex);
    let amount_y = balance_of!(app, token_y, dex);
    assert_eq!(amount_x, 1499);
    assert_eq!(amount_y, 7);

    let pool_after_withdraw = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_x,
        TokenAmount::new(0)
    );
    assert_eq!(
        pool_after_withdraw.fee_protocol_token_y,
        TokenAmount::new(0)
    );
}
