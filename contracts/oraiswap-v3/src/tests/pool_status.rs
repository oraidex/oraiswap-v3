use cosmwasm_std::coins;
use decimal::*;

use crate::tests::helper::{macros::*, MockApp};
use oraiswap_v3_common::{
    error::ContractError,
    math::{percentage::Percentage, sqrt_price::calculate_sqrt_price},
    storage::{FeeTier, PoolKey, PoolStatus},
};

use super::helper::FEE_DENOM;

#[test]
fn test_update_pool_status() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];

    let dex = create_dex!(app, Percentage::new(0), alice);
    let (token_x, token_y) = create_tokens!(app, 500, 500, alice);

    let fee_tier = FeeTier::new(Percentage::new(0), 1).unwrap();

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
    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    // after create, poolStatus = none
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.status, None);

    // only admin can update pool status
    let error = app
        .update_pool_status(&bob, dex.as_str(), &pool_key, None)
        .unwrap_err();
    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::Unauthorized {}.to_string()));

    // update pool status to opening
    app.update_pool_status(&alice, dex.as_str(), &pool_key, Some(PoolStatus::Opening))
        .unwrap();
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.status, Some(PoolStatus::Opening));

    // update pool status to paused
    app.update_pool_status(&alice, dex.as_str(), &pool_key, Some(PoolStatus::Paused))
        .unwrap();
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.status, Some(PoolStatus::Paused));

    // update pool status to LpOnly
    app.update_pool_status(&alice, dex.as_str(), &pool_key, Some(PoolStatus::LpOnly))
        .unwrap();
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.status, Some(PoolStatus::LpOnly));

    // update pool status to SwapOnly
    app.update_pool_status(&alice, dex.as_str(), &pool_key, Some(PoolStatus::SwapOnly))
        .unwrap();
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.status, Some(PoolStatus::SwapOnly));
}
