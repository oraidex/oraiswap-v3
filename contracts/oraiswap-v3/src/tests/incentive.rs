use decimal::*;

use crate::{
    fee_growth::FeeGrowth,
    incentive::IncentiveRecord,
    interface::AssetInfo,
    liquidity::Liquidity,
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    tests::helper::{macros::*, MockApp},
    token_amount::TokenAmount,
    FeeTier, PoolKey, MIN_SQRT_PRICE,
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
            reward_token,
            remaining: total_reward,
            start_timestamp: current_time,
            incentive_growth_global: FeeGrowth(0),
            last_updated: current_time
        }]
    );
}
