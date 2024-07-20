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
    FeeTier, PoolKey, MAX_SQRT_PRICE, MIN_SQRT_PRICE,
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
    let total_reward = Some(TokenAmount(1000000000));
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
            remaining: total_reward.unwrap(),
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
                remaining: total_reward.unwrap(),
                start_timestamp: current_time,
                incentive_growth_global: FeeGrowth(0),
                last_updated: new_timestamp_time
            },
            IncentiveRecord {
                id: 1,
                reward_per_sec,
                reward_token: reward_token.clone(),
                remaining: total_reward.unwrap(),
                start_timestamp: new_timestamp_time,
                incentive_growth_global: FeeGrowth(0),
                last_updated: new_timestamp_time
            }
        ]
    );

    // create incentive with no total reward -> fallback to max:u128
    let latest_timestamp_time = app.app.block_info().time.seconds();
    create_incentive!(
        app,
        dex,
        pool_key,
        reward_token.clone(),
        None,
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
                remaining: total_reward.unwrap(),
                start_timestamp: current_time,
                incentive_growth_global: FeeGrowth(0),
                last_updated: latest_timestamp_time
            },
            IncentiveRecord {
                id: 1,
                reward_per_sec,
                reward_token: reward_token.clone(),
                remaining: total_reward.unwrap(),
                start_timestamp: new_timestamp_time,
                incentive_growth_global: FeeGrowth(0),
                last_updated: latest_timestamp_time
            },
            IncentiveRecord {
                id: 2,
                reward_per_sec,
                reward_token: reward_token.clone(),
                remaining: TokenAmount(u128::MAX),
                start_timestamp: latest_timestamp_time,
                incentive_growth_global: FeeGrowth(0),
                last_updated: latest_timestamp_time
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
    let total_reward = Some(TokenAmount(1000000));
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
    let total_reward = Some(TokenAmount(1000000));
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
        Some(TokenAmount(1000000000)),
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
    let total_reward = Some(TokenAmount(1000000));
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
        Some(TokenAmount(1000000000)),
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

    // try create other position, with the same range, but double liquidity
    create_position!(
        app,
        dex,
        pool_key,
        -10,
        10,
        Liquidity::new(2000),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    // try increase 1000s
    block_info = app.app.block_info();
    let current_timestamp = block_info.time.seconds();
    block_info.time = Timestamp::from_seconds(current_timestamp + 1000);
    app.app.set_block(block_info.clone());

    let incentives = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(
        incentives,
        vec![
            Asset {
                info: reward_token.clone(),
                amount: Uint128::from(133500u128)
            },
            Asset {
                info: reward_token_2.clone(),
                amount: Uint128::from(267000u128)
            }
        ]
    );
    let incentives_2 = get_position_incentives!(app, dex, 1, "alice").unwrap();
    assert_eq!(
        incentives_2,
        vec![
            Asset {
                info: reward_token.clone(),
                amount: Uint128::from(67000u128)
            },
            Asset {
                info: reward_token_2.clone(),
                amount: Uint128::from(134000u128)
            }
        ]
    );
}
#[test]
pub fn test_incentive_with_position_cross_out_of_range() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let dex_raw = &dex.to_string();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount);
    mint!(app, token_z, dex_raw, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();

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

    let reward_token = AssetInfo::Token {
        contract_addr: token_z.clone(),
    };
    let total_reward = Some(TokenAmount(1000000000));
    let reward_per_sec = TokenAmount(100);
    let start_timestamp: Option<u64> = None;
    let liquidity = Liquidity::from_integer(1000000);
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

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    // create 2 position
    // first_pos: range (-20, 20)
    // second_pos: range (10, 50)
    create_position!(
        app,
        dex,
        pool_key,
        -20,
        20,
        liquidity,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();
    create_position!(
        app,
        dex,
        pool_key,
        10,
        50,
        liquidity,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    // increase 1000s, the second position does not have  incentive
    let mut block_info = app.app.block_info();
    let current_timestamp = block_info.time.seconds();
    block_info.time = Timestamp::from_seconds(current_timestamp + 1000);
    app.app.set_block(block_info.clone());

    let incentives = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(incentives.len(), 1);
    println!("incentives: {:?}", incentives);
    let incentives_2 = get_position_incentives!(app, dex, 1, "alice").unwrap();
    assert_eq!(incentives_2, vec![]);

    // try swap to cross tick

    let amount = 1000;
    let swap_amount = TokenAmount(amount);

    mint!(app, token_y, "bob", amount, "alice").unwrap();
    approve!(app, token_y, dex, amount, "bob").unwrap();

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

    swap!(
        app,
        dex,
        pool_key,
        false,
        swap_amount,
        true,
        target_sqrt_price,
        "bob"
    )
    .unwrap();
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.current_tick_index, 14);

    // currently, the both position in range
    let incentive_1_before = get_position_incentives!(app, dex, 0, "alice").unwrap()[0].amount;
    let incentive_2_before = get_position_incentives!(app, dex, 1, "alice").unwrap()[0].amount;

    // try increase 1000s
    let mut block_info = app.app.block_info();
    let current_timestamp = block_info.time.seconds();
    block_info.time = Timestamp::from_seconds(current_timestamp + 1000);
    app.app.set_block(block_info.clone());

    let incentive_1_after = get_position_incentives!(app, dex, 0, "alice").unwrap()[0].amount;
    let incentive_2_after = get_position_incentives!(app, dex, 1, "alice").unwrap()[0].amount;

    assert!(incentive_1_before.lt(&incentive_1_after));
    assert!(incentive_2_before.lt(&incentive_2_after));
    let emit = Uint128::new(100 * 1000);
    assert!(
        (incentive_1_after - incentive_1_before + incentive_2_after - incentive_2_before).le(&emit)
    );

    //try swap to out of range of fist position
    let amount = 1000;
    let swap_amount = TokenAmount(amount);

    mint!(app, token_y, "bob", amount, "alice").unwrap();
    approve!(app, token_y, dex, amount, "bob").unwrap();

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

    swap!(
        app,
        dex,
        pool_key,
        false,
        swap_amount,
        true,
        target_sqrt_price,
        "bob"
    )
    .unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.current_tick_index, 29);

    // currently, the first position is out_of_range, but the second position still in range
    let incentive_1_before = get_position_incentives!(app, dex, 0, "alice").unwrap()[0].amount;
    let incentive_2_before = get_position_incentives!(app, dex, 1, "alice").unwrap()[0].amount;

    // try increase 1000s
    let mut block_info = app.app.block_info();
    let current_timestamp = block_info.time.seconds();
    block_info.time = Timestamp::from_seconds(current_timestamp + 1000);
    app.app.set_block(block_info.clone());

    let incentive_1_after = get_position_incentives!(app, dex, 0, "alice").unwrap()[0].amount;
    let incentive_2_after = get_position_incentives!(app, dex, 1, "alice").unwrap()[0].amount;

    assert!(incentive_1_before.eq(&incentive_1_after));
    assert!(incentive_2_before.lt(&incentive_2_after));
    let emit = Uint128::new(100 * 1000);
    assert!(
        (incentive_1_after - incentive_1_before + incentive_2_after - incentive_2_before).le(&emit)
    );

    // try claim incentives
    let before_dex_balance = balance_of!(app, token_z, dex);
    let before_user_balance = balance_of!(app, token_z, "alice");

    claim_incentives!(app, dex, 0, "alice").unwrap();
    claim_incentives!(app, dex, 1, "alice").unwrap();

    let after_dex_balance = balance_of!(app, token_z, dex);
    let after_user_balance = balance_of!(app, token_z, "alice");
    assert!(before_dex_balance.gt(&after_dex_balance));
    assert!(before_user_balance.lt(&after_user_balance));
    assert!(
        (before_user_balance + before_dex_balance).eq(&(after_user_balance + after_dex_balance))
    );
}

#[test]
pub fn test_remove_position() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let dex_raw = &dex.to_string();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount);
    mint!(app, token_z, dex_raw, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();

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

    let reward_token = AssetInfo::Token {
        contract_addr: token_z.clone(),
    };
    let total_reward = Some(TokenAmount(1000000000));
    let reward_per_sec = TokenAmount(100);
    let start_timestamp: Option<u64> = None;
    let liquidity = Liquidity::from_integer(1000000);
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

    // create position in range
    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        -20,
        20,
        liquidity,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    // increase block time
    let mut block_info = app.app.block_info();
    let current_timestamp = block_info.time.seconds();
    block_info.time = Timestamp::from_seconds(current_timestamp + 1000);
    app.app.set_block(block_info.clone());

    let before_dex_balance = balance_of!(app, token_z, dex);
    let before_user_balance = balance_of!(app, token_z, "alice");

    // try remove position
    remove_position!(app, dex, 0, "alice").unwrap();

    let after_dex_balance = balance_of!(app, token_z, dex);
    let after_user_balance = balance_of!(app, token_z, "alice");

    assert!(before_dex_balance.gt(&after_dex_balance));
    assert!(before_user_balance.lt(&after_user_balance));
    assert!(
        (before_user_balance + before_dex_balance).eq(&(after_user_balance + after_dex_balance))
    );
}

#[test]
pub fn incentive_stress_test() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let dex_raw = &dex.to_string();

    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount);
    mint!(app, token_z, dex_raw, initial_amount, "alice").unwrap();
    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();

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

    let reward_token = AssetInfo::Token {
        contract_addr: token_z.clone(),
    };
    let total_reward = Some(TokenAmount(1000000000));
    let start_timestamp: Option<u64> = None;

    let rps: Vec<TokenAmount> = vec![
        TokenAmount(1),
        TokenAmount(5),
        TokenAmount(10),
        TokenAmount(49),
        TokenAmount(99),
        TokenAmount(10000),
    ];

    for i in 0..rps.len() {
        create_incentive!(
            app,
            dex,
            pool_key,
            reward_token.clone(),
            total_reward,
            rps[i],
            start_timestamp,
            "alice"
        )
        .unwrap();
    }

    // create multi position
    let liq = vec![3233322, 54343223, 3223135, 2431323, 1322339, 53283, 123293];
    let ranges = vec![
        -10000, -1000, -500, -90, -40, -20, -5, 4, 12, 23, 35, 42, 63, 120, 1000, 10000,
    ];
    for i in 0..1000 {
        let liquidity = Liquidity::from_integer(liq[i % liq.len()]);
        let tick_index = i % (ranges.len() - 1);
        let lower_tick = ranges[tick_index];
        let upper_tick = ranges[tick_index + 1];
        create_position!(
            app,
            dex,
            pool_key,
            lower_tick,
            upper_tick,
            liquidity,
            SqrtPrice::new(0),
            SqrtPrice::max_instance(),
            "alice"
        )
        .unwrap();
    }

    // try swap
    mint!(app, token_y, "bob", initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "bob").unwrap();
    mint!(app, token_x, "bob", initial_amount, "alice").unwrap();
    approve!(app, token_x, dex, initial_amount, "bob").unwrap();

    let swap_amounts: Vec<u128> = vec![2323, 233, 321, 5353, 12, 932, 42, 3123, 5438];
    let x_to_y_list = vec![true, false, false, true, true, false, false, true];

    for i in 0..1000 {
        let x_to_y = x_to_y_list[i % x_to_y_list.len()];
        let swap_amount = TokenAmount(swap_amounts[i % swap_amounts.len()]);
        let target_sqrt_price = if x_to_y {
            SqrtPrice::new(MIN_SQRT_PRICE)
        } else {
            SqrtPrice::new(MAX_SQRT_PRICE)
        };

        swap!(
            app,
            dex,
            pool_key,
            x_to_y,
            swap_amount,
            true,
            target_sqrt_price,
            "bob"
        )
        .unwrap();
    }

    let before_dex_balance = balance_of!(app, token_z, dex);
    let before_user_balance = balance_of!(app, token_z, "alice");

    // claim all incentives
    for _ in 0..1000 {
        // try remove position
        remove_position!(app, dex, 0, "alice").unwrap();
    }

    let after_dex_balance = balance_of!(app, token_z, dex);
    let after_user_balance = balance_of!(app, token_z, "alice");

    assert!(before_dex_balance.gt(&after_dex_balance));
    assert!(before_user_balance.lt(&after_user_balance));
    assert!(
        (before_user_balance + before_dex_balance).eq(&(after_user_balance + after_dex_balance))
    );
}

#[test]
pub fn test_claim_incentive_with_single_position() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let dex_raw = &dex.to_string();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount);
    mint!(app, token_z, dex_raw, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();

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

    let reward_token = AssetInfo::Token {
        contract_addr: token_z.clone(),
    };
    let total_reward = Some(TokenAmount::from_integer(1000000000));
    let reward_per_sec = TokenAmount(100);
    let start_timestamp: Option<u64> = None;
    let liquidity = Liquidity::from_integer(1000000);

    // create position in range
    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();
    create_position!(
        app,
        dex,
        pool_key,
        -20,
        20,
        liquidity,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    let timestamp_init = app.app.block_info().time.seconds();
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

    let before_dex_balance = balance_of!(app, token_z, dex);
    let before_user_balance = balance_of!(app, token_z, "alice");

    // increase block time
    for _ in 0..100 {
        let mut block_info = app.app.block_info();
        let current_timestamp = block_info.time.seconds();
        block_info.time = Timestamp::from_seconds(current_timestamp + 1000);
        app.app.set_block(block_info.clone());

        // claim incentives
        claim_incentives!(app, dex, 0, "alice").unwrap();
        let position_state = get_position!(app, dex, 0, "alice").unwrap();
        assert_eq!(position_state.incentives[0].pending_rewards, TokenAmount(0));
    }
    let timestamp_after = app.app.block_info().time.seconds();
    let total_emit = (timestamp_after - timestamp_init) as u128 * reward_per_sec.0;

    let after_dex_balance = balance_of!(app, token_z, dex);
    let after_user_balance = balance_of!(app, token_z, "alice");

    assert!(before_dex_balance.gt(&after_dex_balance));
    assert!(before_user_balance.lt(&after_user_balance));
    assert!(
        (before_user_balance + before_dex_balance).eq(&(after_user_balance + after_dex_balance))
    );
    // total claimed of user must be less than or equal total emit
    assert!((after_user_balance - before_user_balance).le(&total_emit));
}

#[test]
pub fn test_claim_incentive_with_multi_position() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let dex_raw = &dex.to_string();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount);
    mint!(app, token_z, dex_raw, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();

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

    let reward_token = AssetInfo::Token {
        contract_addr: token_z.clone(),
    };
    let total_reward = Some(TokenAmount::from_integer(1000000000));
    let reward_per_sec = TokenAmount(100);
    let start_timestamp: Option<u64> = None;

    // create position in range
    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();
    let timestamp_init = app.app.block_info().time.seconds();
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

    // create multi position
    let liq = vec![3233322, 54343223, 3223135, 2431323, 1322339, 53283, 123293];
    let ranges = vec![
        -10000, -1000, -500, -90, -40, -20, -5, 4, 12, 23, 35, 42, 63, 120, 1000, 10000,
    ];
    for i in 0..100 {
        let liquidity = Liquidity::from_integer(liq[i % liq.len()]);
        let tick_index = i % (ranges.len() - 1);
        let lower_tick = ranges[tick_index];
        let upper_tick = ranges[tick_index + 1];
        create_position!(
            app,
            dex,
            pool_key,
            lower_tick,
            upper_tick,
            liquidity,
            SqrtPrice::new(0),
            SqrtPrice::max_instance(),
            "alice"
        )
        .unwrap();
    }

    let before_dex_balance = balance_of!(app, token_z, dex);
    let before_user_balance = balance_of!(app, token_z, "alice");

    // increase block time
    for _ in 0..100 {
        let mut block_info = app.app.block_info();
        let current_timestamp = block_info.time.seconds();
        block_info.time = Timestamp::from_seconds(current_timestamp + 1000);
        app.app.set_block(block_info.clone());
        for i in 0..100 {
            // claim incentives
            claim_incentives!(app, dex, i, "alice").unwrap();
            let position_state = get_position!(app, dex, i, "alice").unwrap();
            assert_eq!(position_state.incentives[0].pending_rewards, TokenAmount(0));
        }
    }

    let timestamp_after = app.app.block_info().time.seconds();
    let total_emit = (timestamp_after - timestamp_init) as u128 * reward_per_sec.0;

    let after_dex_balance = balance_of!(app, token_z, dex);
    let after_user_balance = balance_of!(app, token_z, "alice");

    assert!(before_dex_balance.gt(&after_dex_balance));
    assert!(before_user_balance.lt(&after_user_balance));
    assert!(
        (before_user_balance + before_dex_balance).eq(&(after_user_balance + after_dex_balance))
    );
    // total claimed of user must be less than or equal total emit
    assert!((after_user_balance - before_user_balance).le(&total_emit));
}

#[test]
pub fn test_update_incentive_with_tick_move_left_to_right() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let dex_raw = &dex.to_string();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount);
    mint!(app, token_z, dex_raw, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();

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

    let reward_token = AssetInfo::Token {
        contract_addr: token_z.clone(),
    };
    let total_reward = Some(TokenAmount(1000000000));
    let reward_per_sec = TokenAmount(100);
    let start_timestamp: Option<u64> = None;
    let liquidity = Liquidity::from_integer(1000000);
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

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    // create 2 position
    // first_pos: range (10, 20)
    // second_pos: range (30, 40)
    create_position!(
        app,
        dex,
        pool_key,
        10,
        20,
        liquidity,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();
    create_position!(
        app,
        dex,
        pool_key,
        30,
        40,
        liquidity,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    // Both positions do not have any incentives due to being out of range
    app.increase_block_time_by_seconds(1000);
    let incentive = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(incentive, vec![]);
    let incentive = get_position_incentives!(app, dex, 1, "alice").unwrap();
    assert_eq!(incentive, vec![]);

    // swap y to x, tick move left -> right
    let amount = 100;
    let swap_amount = TokenAmount(amount);
    mint!(app, token_y, "bob", amount, "alice").unwrap();
    approve!(app, token_y, dex, amount, "bob").unwrap();
    swap!(
        app,
        dex,
        pool_key,
        false,
        swap_amount,
        true,
        SqrtPrice::new(MAX_SQRT_PRICE),
        "bob"
    )
    .unwrap();
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.current_tick_index, 11);
    // The first position has an incentive, but the second one does not have any.
    app.increase_block_time_by_seconds(1000);
    let incentive = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(
        incentive,
        vec![Asset {
            info: reward_token.clone(),
            amount: Uint128::new(100500u128)
        }]
    );
    let incentive = get_position_incentives!(app, dex, 1, "alice").unwrap();
    assert_eq!(incentive, vec![]);

    // swap again
    let amount = 700;
    let swap_amount = TokenAmount(amount);
    mint!(app, token_y, "bob", amount, "alice").unwrap();
    approve!(app, token_y, dex, amount, "bob").unwrap();
    swap!(
        app,
        dex,
        pool_key,
        false,
        swap_amount,
        true,
        SqrtPrice::new(MAX_SQRT_PRICE),
        "bob"
    )
    .unwrap();
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.current_tick_index, 35);
    // the second position have incentive,
    claim_incentives!(app, dex, 0, "alice").unwrap();
    app.increase_block_time_by_seconds(1000);
    let incentive = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(incentive, vec![]);
    let incentive = get_position_incentives!(app, dex, 1, "alice").unwrap();
    assert_eq!(
        incentive,
        vec![Asset {
            info: reward_token.clone(),
            amount: Uint128::new(101000u128)
        }]
    );
}

#[test]
pub fn test_update_incentive_with_tick_move_right_to_left() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let dex_raw = &dex.to_string();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount);
    mint!(app, token_z, dex_raw, initial_amount, "alice").unwrap();

    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();

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

    let reward_token = AssetInfo::Token {
        contract_addr: token_z.clone(),
    };
    let total_reward = Some(TokenAmount(1000000000));
    let reward_per_sec = TokenAmount(100);
    let start_timestamp: Option<u64> = None;
    let liquidity = Liquidity::from_integer(1000000);
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

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    // create 2 position
    // first_pos: range (-20, -10)
    // second_pos: range (-40, -30)
    create_position!(
        app,
        dex,
        pool_key,
        -20,
        -10,
        liquidity,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();
    create_position!(
        app,
        dex,
        pool_key,
        -40,
        -30,
        liquidity,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    // Both positions do not have any incentives due to being out of range
    app.increase_block_time_by_seconds(1000);
    let incentive = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(incentive, vec![]);
    let incentive = get_position_incentives!(app, dex, 1, "alice").unwrap();
    assert_eq!(incentive, vec![]);

    // swap x to y, tick move right -> left
    let amount = 100;
    let swap_amount = TokenAmount(amount);
    mint!(app, token_x, "bob", amount, "alice").unwrap();
    approve!(app, token_x, dex, amount, "bob").unwrap();
    swap!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        SqrtPrice::new(MIN_SQRT_PRICE),
        "bob"
    )
    .unwrap();
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.current_tick_index, -12);
    // The first position has an incentive, but the second one does not have any.
    app.increase_block_time_by_seconds(1000);
    let incentive = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(
        incentive,
        vec![Asset {
            info: reward_token.clone(),
            amount: Uint128::new(100500u128)
        }]
    );
    let incentive = get_position_incentives!(app, dex, 1, "alice").unwrap();
    assert_eq!(incentive, vec![]);

    // swap again
    let amount = 700;
    let swap_amount = TokenAmount(amount);
    mint!(app, token_x, "bob", amount, "alice").unwrap();
    approve!(app, token_x, dex, amount, "bob").unwrap();
    swap!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        SqrtPrice::new(MIN_SQRT_PRICE),
        "bob"
    )
    .unwrap();
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.current_tick_index, -36);
    // the second position have incentive,
    claim_incentives!(app, dex, 0, "alice").unwrap();
    app.increase_block_time_by_seconds(1000);
    let incentive = get_position_incentives!(app, dex, 0, "alice").unwrap();
    assert_eq!(incentive, vec![]);
    let incentive = get_position_incentives!(app, dex, 1, "alice").unwrap();
    assert_eq!(
        incentive,
        vec![Asset {
            info: reward_token.clone(),
            amount: Uint128::new(101000u128)
        }]
    );
}
