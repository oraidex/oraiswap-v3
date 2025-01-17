use cosmwasm_std::{coins, Addr};
use decimal::{Decimal, Factories};
use oraiswap_v3_common::{
    error::ContractError,
    math::{
        percentage::Percentage,
        sqrt_price::{calculate_sqrt_price, SqrtPrice},
    },
    oraiswap_v3_msg::{ExecuteMsg, QueryMsg},
    storage::{FeeTier, Pool},
};

use crate::tests::helper::{macros::*, MockApp, FEE_DENOM};

#[test]
fn test_create_pool() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();
    let dex = create_dex!(app, protocol_fee, alice);

    let _res = add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let token_x = app.create_token(alice, "tokenx", initial_amount);
    let token_y = app.create_token(alice, "tokeny", initial_amount);

    let result = app.create_pool(
        alice,
        dex.as_str(),
        token_x.as_str(),
        token_y.as_str(),
        fee_tier,
        init_sqrt_price,
        init_tick,
    );
    assert!(result.is_ok());

    let query_pool_msg = QueryMsg::Pool {
        token_0: token_x.to_string(),
        token_1: token_y.to_string(),
        fee_tier: fee_tier.clone(),
    };
    let pool: Pool = app.query(dex.clone(), &query_pool_msg).unwrap();
    assert_eq!(pool.current_tick_index, init_tick);
}

#[test]
fn test_create_pool_x_to_y_and_y_to_x() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();
    let dex = app.create_dex(alice, protocol_fee).unwrap();

    let _res = app.add_fee_tier(alice, dex.as_str(), fee_tier).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let token_x = app.create_token(alice, "tokenx", initial_amount);
    let token_y = app.create_token(alice, "tokeny", initial_amount);

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

    let result = app
        .create_pool(
            alice,
            dex.as_str(),
            token_y.as_str(),
            token_x.as_str(),
            fee_tier,
            init_sqrt_price,
            init_tick,
        )
        .unwrap_err();

    assert!(result
        .root_cause()
        .to_string()
        .contains(&ContractError::PoolAlreadyExist {}.to_string()));
}

#[test]
fn test_create_pool_with_same_tokens() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();
    let dex = app.create_dex(alice, protocol_fee).unwrap();

    let _res = app.add_fee_tier(alice, dex.as_str(), fee_tier).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let token_x = app.create_token(alice, "tokenx", initial_amount);

    let result = app
        .create_pool(
            alice,
            dex.as_str(),
            token_x.as_str(),
            token_x.as_str(),
            fee_tier,
            init_sqrt_price,
            init_tick,
        )
        .unwrap_err();
    assert!(result
        .root_cause()
        .to_string()
        .contains(&ContractError::TokensAreSame.to_string()));
}

#[test]
fn test_create_pool_fee_tier_not_added() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = create_dex!(app, protocol_fee, alice);

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let token_x = app.create_token(alice, "tokenx", initial_amount);
    let token_y = app.create_token(alice, "tokeny", initial_amount);

    let result = app
        .create_pool(
            alice,
            dex.as_str(),
            token_x.as_str(),
            token_y.as_str(),
            fee_tier,
            init_sqrt_price,
            init_tick,
        )
        .unwrap_err();
    assert!(result
        .root_cause()
        .to_string()
        .contains(&ContractError::FeeTierNotFound.to_string()));
}

#[test]
fn test_create_pool_init_tick_not_divided_by_tick_spacing() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, protocol_fee, alice);

    let fee_tier = FeeTier::new(protocol_fee, 3).unwrap();

    let _res = app.add_fee_tier(alice, dex.as_str(), fee_tier).unwrap();

    let init_tick = 2;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let token_x = app.create_token(alice, "tokenx", initial_amount);
    let token_y = app.create_token(alice, "tokeny", initial_amount);

    let result = app
        .create_pool(
            alice,
            dex.as_str(),
            token_x.as_str(),
            token_y.as_str(),
            fee_tier,
            init_sqrt_price,
            init_tick,
        )
        .unwrap_err();
    assert!(result
        .root_cause()
        .to_string()
        .contains(&ContractError::InvalidInitTick.to_string()));
}

#[test]
fn test_create_pool_init_sqrt_price_minimal_difference_from_tick() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];
    let dex = create_dex!(app, protocol_fee, alice);

    let fee_tier = FeeTier::new(protocol_fee, 3).unwrap();

    let _res = app.add_fee_tier(alice, dex.as_str(), fee_tier).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap() + SqrtPrice::new(1);
    let token_x = app.create_token(alice, "tokenx", initial_amount);
    let token_y = app.create_token(alice, "tokeny", initial_amount);

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

    let query_pool_msg = QueryMsg::Pool {
        token_0: token_x.to_string(),
        token_1: token_y.to_string(),
        fee_tier: fee_tier.clone(),
    };
    let pool: Pool = app.query(dex.clone(), &query_pool_msg).unwrap();
    assert_eq!(pool.current_tick_index, init_tick);
}

#[test]
fn test_create_pool_init_sqrt_price_has_closer_init_tick() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 1).unwrap();
    let dex = app.create_dex(alice, protocol_fee).unwrap();

    let _res = app.add_fee_tier(alice, dex.as_str(), fee_tier).unwrap();

    let init_tick = 2;
    let init_sqrt_price = SqrtPrice::new(1000175003749000000000000);
    let token_x = app.create_token(alice, "tokenx", initial_amount);
    let token_y = app.create_token(alice, "tokeny", initial_amount);

    let create_pool_msg = ExecuteMsg::CreatePool {
        token_0: token_x.to_string(),
        token_1: token_y.to_string(),
        fee_tier: fee_tier.clone(),
        init_sqrt_price: init_sqrt_price.clone(),
        init_tick,
    };
    let result = app
        .execute(Addr::unchecked(alice), dex.clone(), &create_pool_msg, &[])
        .unwrap_err();
    assert!(result
        .root_cause()
        .to_string()
        .contains(&ContractError::InvalidInitSqrtPrice.to_string()));

    let correct_init_tick = 3;
    let create_pool_msg = ExecuteMsg::CreatePool {
        token_0: token_x.to_string(),
        token_1: token_y.to_string(),
        fee_tier: fee_tier.clone(),
        init_sqrt_price: init_sqrt_price.clone(),
        init_tick: correct_init_tick,
    };
    app.execute(Addr::unchecked(alice), dex.clone(), &create_pool_msg, &[])
        .unwrap();

    let query_pool_msg = QueryMsg::Pool {
        token_0: token_x.to_string(),
        token_1: token_y.to_string(),
        fee_tier: fee_tier.clone(),
    };
    let pool: Pool = app.query(dex.clone(), &query_pool_msg).unwrap();
    assert_eq!(pool.current_tick_index, correct_init_tick);
}

#[test]
fn test_create_pool_init_sqrt_price_has_closer_init_tick_spacing_over_one() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = create_dex!(app, Percentage::from_scale(6, 3), alice);
    let initial_amount = 10u128.pow(10);

    let fee_tier = FeeTier::new(Percentage::from_scale(5, 1), 3).unwrap();

    add_fee_tier!(app, dex, fee_tier, alice).unwrap();

    let init_tick = 0;
    let init_sqrt_price = SqrtPrice::new(1000225003749000000000000);
    let (token_x, token_y) = create_tokens!(app, initial_amount, alice);

    let create_pool_msg = ExecuteMsg::CreatePool {
        token_0: token_x.to_string(),
        token_1: token_y.to_string(),
        fee_tier: fee_tier.clone(),
        init_sqrt_price: init_sqrt_price.clone(),
        init_tick,
    };
    let result = app
        .execute(Addr::unchecked(alice), dex.clone(), &create_pool_msg, &[])
        .unwrap_err();
    assert!(result
        .root_cause()
        .to_string()
        .contains(&ContractError::InvalidInitSqrtPrice.to_string()));

    let correct_init_tick = 3;
    let create_pool_msg = ExecuteMsg::CreatePool {
        token_0: token_x.to_string(),
        token_1: token_y.to_string(),
        fee_tier: fee_tier.clone(),
        init_sqrt_price: init_sqrt_price.clone(),
        init_tick: correct_init_tick,
    };
    app.execute(Addr::unchecked(alice), dex.clone(), &create_pool_msg, &[])
        .unwrap();

    let query_pool_msg = QueryMsg::Pool {
        token_0: token_x.to_string(),
        token_1: token_y.to_string(),
        fee_tier: fee_tier.clone(),
    };
    let pool: Pool = app.query(dex.clone(), &query_pool_msg).unwrap();
    assert_eq!(pool.current_tick_index, correct_init_tick);
}

#[test]
fn test_create_many_pools_success() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = create_dex!(app, protocol_fee, alice);

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();

    let _res = app.add_fee_tier(alice, dex.as_str(), fee_tier).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    let amount_of_pools_to_create = 1000;
    let alphabet = "abcdefghijklmnopqrstuvwxyz";

    for i in 0..amount_of_pools_to_create {
        let token_x = app.create_token(
            alice,
            &format!("tokenx{}", alphabet.chars().nth(i % 26).unwrap()),
            initial_amount,
        );
        let token_y = app.create_token(
            alice,
            &format!("tokeny{}", alphabet.chars().nth((i / 26) % 26).unwrap()),
            initial_amount,
        );

        let create_pool_msg = ExecuteMsg::CreatePool {
            token_0: token_x.to_string(),
            token_1: token_y.to_string(),
            fee_tier: fee_tier.clone(),
            init_sqrt_price: init_sqrt_price.clone(),
            init_tick,
        };

        let result = app.execute(Addr::unchecked(alice), dex.clone(), &create_pool_msg, &[]);

        assert!(result.is_ok());
    }
}
