use cosmwasm_std::coins;
use decimal::*;
use oraiswap_v3_common::{
    math::{fee_growth::FeeGrowth, liquidity::Liquidity, percentage::Percentage},
    storage::{FeeTier, PoolKey},
};

use crate::tests::helper::{macros::*, MockApp, FEE_DENOM};

#[test]
fn test_cross() {
    let initial_mint = 10u128.pow(10);
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];

    let dex = create_dex!(app, Percentage::from_scale(1, 2), alice);
    let (token_x, token_y) = create_tokens!(app, initial_mint, initial_mint, alice);

    init_basic_pool!(app, dex, token_x, token_y, alice);
    init_basic_position!(app, dex, token_x, token_y, alice);
    init_cross_position!(app, dex, token_x, token_y, alice);
    init_cross_swap!(
        app,
        dex,
        token_x.to_string(),
        token_y.to_string(),
        alice,
        bob
    );

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let upper_tick_index = 10;
    let middle_tick_index = -10;
    let lower_tick_index = -20;

    let upper_tick = get_tick!(app, dex, pool_key, upper_tick_index).unwrap();
    let middle_tick = get_tick!(app, dex, pool_key, middle_tick_index).unwrap();
    let lower_tick = get_tick!(app, dex, pool_key, lower_tick_index).unwrap();

    assert_eq!(
        upper_tick.liquidity_change,
        Liquidity::from_integer(1000000)
    );
    assert_eq!(
        middle_tick.liquidity_change,
        Liquidity::from_integer(1000000)
    );
    assert_eq!(
        lower_tick.liquidity_change,
        Liquidity::from_integer(1000000)
    );

    assert_eq!(
        upper_tick.fee_growth_outside_x,
        FeeGrowth::new(U256::from(0))
    );
    assert_eq!(
        middle_tick.fee_growth_outside_x,
        FeeGrowth::new(30000000000000000000000_u128.into())
    );
    assert_eq!(
        lower_tick.fee_growth_outside_x,
        FeeGrowth::new(U256::from(0))
    );
}
