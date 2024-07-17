use cosmwasm_std::{Addr, Timestamp, Uint128};
use decimal::*;

use crate::{
    fee_growth::FeeGrowth,
    incentive::{IncentiveRecord, PositionIncentives},
    interface::{Asset, AssetInfo},
    liquidity::Liquidity,
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    tests::helper::{macros::*, MockApp},
    token_amount::TokenAmount,
    FeeTier, PoolKey,
};

#[test]
pub fn test_create_incentive() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let (token_x, token_y) = create_tokens!(app, 500, 500);

    let fee_tier = FeeTier::new(Percentage::new(0), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = 10;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let reward_token = AssetInfo::NativeToken {
        denom: "orai".to_string(),
    };
    let total_reward = TokenAmount(1000000000);
    let reward_per_sec = TokenAmount(100);
    let start_timestamp: Option<u64> = None;
    let current_time = app.app.block_info().time.seconds();
    create_incentive!(
        app,
        dex,
        pool_key,
        reward_token.clone(),
        total_reward,
        reward_per_sec,
        start_timestamp,
        "alice"
    )
    .unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(
        pool.incentives,
        vec![IncentiveRecord {
            id: 0,
            reward_per_sec,
            reward_token: reward_token.clone(),
            remaining: total_reward,
            start_timestamp: current_time,
            incentive_growth_global: FeeGrowth(0),
            last_updated: current_time
        }]
    );

    // create other incentives
    let new_timestamp_time = app.app.block_info().time.seconds();
    create_incentive!(
        app,
        dex,
        pool_key,
        reward_token.clone(),
        total_reward,
        reward_per_sec,
        start_timestamp,
        "alice"
    )
    .unwrap();
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(
        pool.incentives,
        vec![
            IncentiveRecord {
                id: 0,
                reward_per_sec,
                reward_token: reward_token.clone(),
                remaining: total_reward,
                start_timestamp: current_time,
                incentive_growth_global: FeeGrowth(0),
                last_updated: new_timestamp_time
            },
            IncentiveRecord {
                id: 1,
                reward_per_sec,
                reward_token: reward_token.clone(),
                remaining: total_reward,
                start_timestamp: new_timestamp_time,
                incentive_growth_global: FeeGrowth(0),
                last_updated: new_timestamp_time
            }
        ]
    );

    // create fail, unauthorized
    let res = create_incentive!(
        app,
        dex,
        pool_key,
        reward_token.clone(),
        total_reward,
        reward_per_sec,
        start_timestamp,
        "bob"
    );
    assert!(res.is_err());
}

#[test]
pub fn test_single_incentive_with_single_position() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let (token_x, token_y) = create_tokens!(app, 500, 500);

    let fee_tier = FeeTier::new(Percentage::new(0), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

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
        "alice"
    )
    .unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let reward_token = AssetInfo::NativeToken {
        denom: "orai".to_string(),
    };
    let total_reward = TokenAmount(1000000);
    let reward_per_sec = TokenAmount(100);
    let start_timestamp: Option<u64> = None;
    create_incentive!(
        app,
        dex,
        pool_key,
        reward_token.clone(),
        total_reward,
        reward_per_sec,
        start_timestamp,
        "alice"
    )
    .unwrap();

    // create position
    approve!(app, token_x, dex, 5000, "alice").unwrap();
    approve!(app, token_y, dex, 5000, "alice").unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let block_info = app.app.block_info();
    create_position!(
        app,
        dex,
        pool_key,
        -10,
        10,
        Liquidity::new(1000),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    // No incentive available after creating the position
    let position_state = get_position!(app, dex, 0, "alice").unwrap();

    assert_eq!(
        position_state.incentives,
        vec![PositionIncentives {
            incentive_id: 0,
            pending_rewards: TokenAmount(0),
            incentive_growth_inside: FeeGrowth(0)
        }]
    );
    // set block_info to ensure after create position, block time not change
    app.app.set_block(block_info);
    let incentives = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(incentives, vec![]);

    // try increase block time to 1000s
    // => totalReward for position = 100 * 1000 = 100000;
    let mut block_info = app.app.block_info();
    block_info.time = Timestamp::from_seconds(block_info.time.seconds() + 1000);
    app.app.set_block(block_info);

    // get position
    let incentives = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(
        incentives,
        vec![Asset {
            info: reward_token.clone(),
            amount: Uint128::from(100000u128)
        }]
    );

    // reach limit of total reward
    block_info = app.app.block_info();
    block_info.time = Timestamp::from_seconds(block_info.time.seconds() + 1000000);
    app.app.set_block(block_info);
    let incentives = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(
        incentives,
        vec![Asset {
            info: reward_token.clone(),
            amount: Uint128::from(1000000u128)
        }]
    );
}

#[test]
pub fn test_multi_incentives_with_single_position() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let (token_x, token_y) = create_tokens!(app, 500, 500);

    let fee_tier = FeeTier::new(Percentage::new(0), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

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
        "alice"
    )
    .unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let reward_token = AssetInfo::NativeToken {
        denom: "orai".to_string(),
    };
    let reward_token_2 = AssetInfo::Token {
        contract_addr: Addr::unchecked("usdt"),
    };
    let total_reward = TokenAmount(1000000);
    let reward_per_sec = TokenAmount(100);
    let start_timestamp: Option<u64> = None;
    create_incentive!(
        app,
        dex,
        pool_key,
        reward_token.clone(),
        total_reward,
        reward_per_sec,
        start_timestamp,
        "alice"
    )
    .unwrap();
    create_incentive!(
        app,
        dex,
        pool_key,
        reward_token_2.clone(),
        TokenAmount(1000000000),
        TokenAmount(200),
        start_timestamp,
        "alice"
    )
    .unwrap();

    // create position
    approve!(app, token_x, dex, 5000, "alice").unwrap();
    approve!(app, token_y, dex, 5000, "alice").unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let block_info = app.app.block_info();
    create_position!(
        app,
        dex,
        pool_key,
        -10,
        10,
        Liquidity::new(1000),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    // No incentive available after creating the position
    let position_state = get_position!(app, dex, 0, "alice").unwrap();

    assert_eq!(
        position_state.incentives,
        vec![
            PositionIncentives {
                incentive_id: 0,
                pending_rewards: TokenAmount(0),
                incentive_growth_inside: FeeGrowth(0)
            },
            PositionIncentives {
                incentive_id: 1,
                pending_rewards: TokenAmount(0),
                incentive_growth_inside: FeeGrowth(0)
            }
        ]
    );
    // set block_info to ensure after create position, block time not change
    app.app.set_block(block_info);
    let incentives = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(incentives, vec![]);

    // try increase block time to 1000s
    // => totalReward for position = 100 * 1000 = 100000;
    let mut block_info = app.app.block_info();
    block_info.time = Timestamp::from_seconds(block_info.time.seconds() + 1000);
    app.app.set_block(block_info);

    // get position
    let incentives = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(
        incentives,
        vec![
            Asset {
                info: reward_token.clone(),
                amount: Uint128::from(100000u128)
            },
            Asset {
                info: reward_token_2.clone(),
                amount: Uint128::from(200000u128)
            }
        ]
    );

    // Reached the limit of the total reward for the first incentive,
    // and the calculation for the second incentive is impacted by overflow.
    block_info = app.app.block_info();
    let current_timestamp = block_info.time.seconds();
    block_info.time = Timestamp::from_seconds(current_timestamp + 1000000);
    app.app.set_block(block_info.clone());
    let incentives = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(
        incentives,
        vec![Asset {
            info: reward_token.clone(),
            amount: Uint128::from(1000000u128)
        }]
    );

    // success
    block_info.time = Timestamp::from_seconds(current_timestamp + 20000);
    app.app.set_block(block_info);
    let incentives = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(
        incentives,
        vec![
            Asset {
                info: reward_token.clone(),
                amount: Uint128::from(1000000u128)
            },
            Asset {
                info: reward_token_2.clone(),
                amount: Uint128::from(4200000u128)
            }
        ]
    );
}

#[test]
pub fn test_multi_incentives_with_multi_positions() {
    //TODO
}
#[test]
pub fn test_incentive_with_position_cross_out_of_range() {
    // TODO
}

#[test]
pub fn test_incentive_with_position_cross_in_range() {
    // TODO
}

#[test]
pub fn test_remove_position() {
    // TODO
}
