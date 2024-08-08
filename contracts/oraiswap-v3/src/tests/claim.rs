use cosmwasm_std::Timestamp;
use decimal::{Decimal, Factories};
use oraiswap_v3_common::asset::AssetInfo;

use crate::{
    liquidity::Liquidity,
    percentage::Percentage,
    sqrt_price::{self, calculate_sqrt_price, SqrtPrice},
    tests::helper::{macros::*, MockApp},
    token_amount::TokenAmount,
    FeeTier, PoolKey,
};

#[test]
fn test_claim() {
    let mut app = MockApp::new(&[]);
    let (dex, token_x, token_y) = init_dex_and_tokens!(app);
    init_basic_pool!(app, dex, token_x, token_y);
    init_basic_position!(app, dex, token_x, token_y);
    init_basic_swap!(app, dex, token_x, token_y);

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let user_amount_before_claim = balance_of!(app, token_x, "alice");
    let dex_amount_before_claim = balance_of!(app, token_x, dex);

    claim_fee!(app, dex, 0, "alice").unwrap();

    let user_amount_after_claim = balance_of!(app, token_x, "alice");
    let dex_amount_after_claim = balance_of!(app, token_x, dex);
    let position = get_position!(app, dex, 0, "alice").unwrap();
    let expected_tokens_claimed = 5;

    assert_eq!(
        user_amount_after_claim - expected_tokens_claimed,
        user_amount_before_claim
    );
    assert_eq!(
        dex_amount_after_claim + expected_tokens_claimed,
        dex_amount_before_claim
    );
    assert_eq!(position.fee_growth_inside_x, pool.fee_growth_global_x);
    assert_eq!(position.tokens_owed_x, TokenAmount(0));
}

#[test]
fn test_claim_not_owner() {
    let mut app = MockApp::new(&[]);
    let (dex, token_x, token_y) = init_dex_and_tokens!(app);
    init_basic_pool!(app, dex, token_x, token_y);
    init_basic_position!(app, dex, token_x, token_y);
    init_basic_swap!(app, dex, token_x, token_y);

    let error = claim_fee!(app, dex, 0, "bob").unwrap_err();
    assert!(error.root_cause().to_string().contains("not found"));
}

#[test]
fn claim_both_fee_and_incentives() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::new(0));
    let dex_raw = &dex.to_string();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount);
    mint!(app, token_z, dex_raw, initial_amount, "alice").unwrap();
    let (token_a, token_b) = create_tokens!(app, initial_amount, initial_amount);
    mint!(app, token_a, dex_raw, initial_amount, "alice").unwrap();
    mint!(app, token_b, dex_raw, initial_amount, "alice").unwrap();

    let incentives_fund_manager = app.get_incentives_fund_manager(dex_raw).unwrap();
    let incentives_fund_manager_raw = &incentives_fund_manager.to_string();

    // mint token for incentive contract
    mint!(
        app,
        token_a,
        incentives_fund_manager_raw,
        initial_amount,
        "alice"
    )
    .unwrap();
    mint!(
        app,
        token_b,
        incentives_fund_manager_raw,
        initial_amount,
        "alice"
    )
    .unwrap();
    mint!(
        app,
        token_z,
        incentives_fund_manager_raw,
        initial_amount,
        "alice"
    )
    .unwrap();

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

    let reward_token_1 = AssetInfo::Token {
        contract_addr: token_z.clone(),
    };
    let reward_token_2 = AssetInfo::Token {
        contract_addr: token_a.clone(),
    };
    let reward_token_3 = AssetInfo::Token {
        contract_addr: token_b.clone(),
    };
    let total_reward = Some(TokenAmount::from_integer(1000000000));
    let reward_per_sec = TokenAmount(100);
    let start_timestamp: Option<u64> = None;
    let liquidity = Liquidity::from_integer(1000000);

    // create position in range -20 - 20
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
        reward_token_1.clone(),
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
        reward_token_3.clone(),
        total_reward,
        reward_per_sec,
        start_timestamp,
        "alice"
    )
    .unwrap();

    let before_dex_balance_token_x = balance_of!(app, token_x, dex);
    let before_incentive_balance_token_z = balance_of!(app, token_z, incentives_fund_manager);
    let before_user_balance_token_x = balance_of!(app, token_x, "alice");
    let before_user_balance_token_z = balance_of!(app, token_z, "alice");

    // swap to increase fee growth
    mint!(app, token_x, "bob", initial_amount, "alice").unwrap();
    approve!(app, token_x, dex, initial_amount, "bob").unwrap();
    swap!(
        app,
        dex,
        pool_key,
        true,
        TokenAmount(1000),
        true,
        sqrt_price::get_min_sqrt_price(fee_tier.tick_spacing),
        "bob"
    )
    .unwrap();

    // increase time to have incentives
    let mut block_info = app.app.block_info();
    let current_timestamp = block_info.time.seconds();
    block_info.time = Timestamp::from_seconds(current_timestamp + 1000);
    app.app.set_block(block_info.clone());

    // claim both
    claim_fee!(app, dex, 0, "alice").unwrap();

    let timestamp_after = app.app.block_info().time.seconds();
    let total_emit = (timestamp_after - timestamp_init) as u128 * reward_per_sec.0;

    let after_dex_balance_token_x = balance_of!(app, token_x, dex);
    let after_incentive_balance_token_z = balance_of!(app, token_z, incentives_fund_manager);
    let after_user_balance_token_x = balance_of!(app, token_x, "alice");
    let after_user_balance_token_z = balance_of!(app, token_z, "alice");

    // incentive assert
    assert!(before_incentive_balance_token_z.gt(&after_incentive_balance_token_z));
    assert!(before_user_balance_token_z.lt(&after_user_balance_token_z));
    assert!(
        (before_user_balance_token_z + before_incentive_balance_token_z)
            .eq(&(after_user_balance_token_z + after_incentive_balance_token_z))
    );
    assert!((after_user_balance_token_z - before_user_balance_token_z).le(&total_emit));

    // fee claimed assert
    let position = get_position!(app, dex, 0, "alice").unwrap();
    let fee_tokens_claimed = 6;
    let receive_x_for_dex = 994;
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    assert_eq!(
        before_user_balance_token_x + fee_tokens_claimed,
        after_user_balance_token_x
    );
    assert_eq!(
        before_dex_balance_token_x + receive_x_for_dex,
        after_dex_balance_token_x
    );
    assert_eq!(position.fee_growth_inside_x, pool.fee_growth_global_x);
    assert_eq!(position.tokens_owed_x, TokenAmount(0));
}
