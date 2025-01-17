use cosmwasm_std::{coins, Decimal as StdDecimal, Uint128};
use decimal::*;
use oraiswap::mixed_router::SwapOperation;
use oraiswap_v3_common::asset::{Asset, AssetInfo};
use oraiswap_v3_common::error::ContractError;
use oraiswap_v3_common::math::liquidity::Liquidity;
use oraiswap_v3_common::math::percentage::Percentage;

use oraiswap_v3_common::storage::{FeeTier, PoolKey};

use crate::msg::Route;
use crate::tests::common::init_basic_v3_pool;
use crate::tests::helper::MockApp;
use crate::tests::helper::{macros::*, FEE_DENOM};

#[test]
fn test_zap_in_with_same_token() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key_x_y = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let tick_lower_index = 0;
    let tick_upper_index = 10;

    // asset_in = token_x
    let asset_in = Asset {
        info: AssetInfo::Token {
            contract_addr: token_x.clone(),
        },
        amount: Uint128::new(1000000),
    };
    let _res = app
        .zap_in_liquidity(
            &bob,
            zapper.as_str(),
            pool_key_x_y.clone(),
            tick_lower_index,
            tick_upper_index,
            &asset_in,
            vec![Route {
                token_in: token_x.to_string(),
                offer_amount: Uint128::new(500000),
                operations: vec![SwapOperation::SwapV3 {
                    pool_key: pool_key_x_y.clone(),
                    x_to_y: true,
                }],
                minimum_receive: None,
            }],
            None,
        )
        .unwrap();

    // get all positions
    let all_positions = get_all_positions!(app, config.dex_v3, bob);
    assert_eq!(all_positions.len(), 1);

    // asset_in = token_y
    let asset_in = Asset {
        info: AssetInfo::Token {
            contract_addr: token_y.clone(),
        },
        amount: Uint128::new(1000000),
    };

    // success
    app.zap_in_liquidity(
        &bob,
        zapper.as_str(),
        pool_key_x_y.clone(),
        tick_lower_index,
        tick_upper_index,
        &asset_in,
        vec![Route {
            token_in: token_x.to_string(),
            offer_amount: Uint128::new(500000),
            operations: vec![SwapOperation::SwapV3 {
                pool_key: pool_key_x_y.clone(),
                x_to_y: false,
            }],
            minimum_receive: None,
        }],
        None,
    )
    .unwrap();
    // get all positions
    let all_positions = get_all_positions!(app, config.dex_v3, bob);
    assert_eq!(all_positions.len(), 2);
}

#[test]
fn test_zap_in_by_diff_token() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key_x_y = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let pool_key_y_z = PoolKey::new(token_y.to_string(), token_z.to_string(), fee_tier).unwrap();

    let tick_lower_index = 0;
    let tick_upper_index = 10;

    // asset_in = token_z
    let asset_in = Asset {
        info: AssetInfo::Token {
            contract_addr: token_z.clone(),
        },
        amount: Uint128::new(1000),
    };

    // add successful
    app.zap_in_liquidity(
        &bob,
        zapper.as_str(),
        pool_key_x_y.clone(),
        tick_lower_index,
        tick_upper_index,
        &asset_in,
        vec![
            Route {
                token_in: token_z.to_string(),
                offer_amount: Uint128::new(500),
                operations: vec![
                    SwapOperation::SwapV3 {
                        pool_key: pool_key_y_z.clone(),
                        x_to_y: false,
                    },
                    SwapOperation::SwapV3 {
                        pool_key: pool_key_x_y.clone(),
                        x_to_y: false,
                    },
                ],
                minimum_receive: None,
            },
            Route {
                token_in: token_z.to_string(),
                offer_amount: Uint128::new(500),
                operations: vec![SwapOperation::SwapV3 {
                    pool_key: pool_key_y_z.clone(),
                    x_to_y: false,
                }],
                minimum_receive: None,
            },
        ],
        None,
    )
    .unwrap();

    // get all positions
    let all_positions = get_all_positions!(app, config.dex_v3, bob);
    assert_eq!(all_positions.len(), 1);
}

#[test]
fn test_zap_in_with_asset_in_lt_total_swap() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key_x_y = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let pool_key_y_z = PoolKey::new(token_y.to_string(), token_z.to_string(), fee_tier).unwrap();

    let tick_lower_index = 0;
    let tick_upper_index = 10;

    // asset_in = token_z
    let asset_in = Asset {
        info: AssetInfo::Token {
            contract_addr: token_z.clone(),
        },
        amount: Uint128::new(999),
    };

    // add successful
    let err = app
        .zap_in_liquidity(
            &bob,
            zapper.as_str(),
            pool_key_x_y.clone(),
            tick_lower_index,
            tick_upper_index,
            &asset_in,
            vec![
                Route {
                    token_in: token_z.to_string(),
                    offer_amount: Uint128::new(500),
                    operations: vec![
                        SwapOperation::SwapV3 {
                            pool_key: pool_key_y_z.clone(),
                            x_to_y: false,
                        },
                        SwapOperation::SwapV3 {
                            pool_key: pool_key_x_y.clone(),
                            x_to_y: false,
                        },
                    ],
                    minimum_receive: None,
                },
                Route {
                    token_in: token_z.to_string(),
                    offer_amount: Uint128::new(500),
                    operations: vec![SwapOperation::SwapV3 {
                        pool_key: pool_key_y_z.clone(),
                        x_to_y: false,
                    }],
                    minimum_receive: None,
                },
            ],
            None,
        )
        .unwrap_err();

    assert!(err.root_cause().to_string().contains("Invalid fund"));
}

#[test]
fn test_zap_in_out_of_range() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key_x_y = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let pool_key_y_z = PoolKey::new(token_y.to_string(), token_z.to_string(), fee_tier).unwrap();

    // asset_in = token_z
    let asset_in = Asset {
        info: AssetInfo::Token {
            contract_addr: token_z.clone(),
        },
        amount: Uint128::new(1000),
    };

    // query pool
    let pool = get_pool!(app, config.dex_v3, token_x, token_y, fee_tier).unwrap();

    // add position < currentTick
    let tick_lower_index = pool.current_tick_index - 30;
    let tick_upper_index = pool.current_tick_index - 20;

    // add fail. liquidity = 0
    let error = app
        .zap_in_liquidity(
            &bob,
            zapper.as_str(),
            pool_key_x_y.clone(),
            tick_lower_index,
            tick_upper_index,
            &asset_in,
            vec![Route {
                token_in: token_z.to_string(),
                offer_amount: Uint128::new(500),
                operations: vec![
                    SwapOperation::SwapV3 {
                        pool_key: pool_key_y_z.clone(),
                        x_to_y: false,
                    },
                    SwapOperation::SwapV3 {
                        pool_key: pool_key_x_y.clone(),
                        x_to_y: false,
                    },
                ],
                minimum_receive: None,
            }],
            None,
        )
        .unwrap_err();
    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::InsufficientLiquidity {}.to_string()));

    // add successful
    app.zap_in_liquidity(
        &bob,
        zapper.as_str(),
        pool_key_x_y.clone(),
        tick_lower_index,
        tick_upper_index,
        &asset_in,
        vec![Route {
            token_in: token_z.to_string(),
            offer_amount: Uint128::new(500),
            operations: vec![SwapOperation::SwapV3 {
                pool_key: pool_key_y_z.clone(),
                x_to_y: false,
            }],
            minimum_receive: None,
        }],
        None,
    )
    .unwrap();

    // get all positions
    let all_positions = get_all_positions!(app, config.dex_v3, bob);

    assert_eq!(all_positions.len(), 1);

    // add position < currentTick
    let tick_lower_index = pool.current_tick_index + 20;
    let tick_upper_index = pool.current_tick_index + 30;

    // add fail. liquidity = 0
    let error = app
        .zap_in_liquidity(
            &bob,
            zapper.as_str(),
            pool_key_x_y.clone(),
            tick_lower_index,
            tick_upper_index,
            &asset_in,
            vec![Route {
                token_in: token_z.to_string(),
                offer_amount: Uint128::new(500),
                operations: vec![SwapOperation::SwapV3 {
                    pool_key: pool_key_y_z.clone(),
                    x_to_y: false,
                }],
                minimum_receive: None,
            }],
            None,
        )
        .unwrap_err();
    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::InsufficientLiquidity {}.to_string()));

    // add successful
    app.zap_in_liquidity(
        &bob,
        zapper.as_str(),
        pool_key_x_y.clone(),
        tick_lower_index,
        tick_upper_index,
        &asset_in,
        vec![Route {
            token_in: token_z.to_string(),
            offer_amount: Uint128::new(500),
            operations: vec![
                SwapOperation::SwapV3 {
                    pool_key: pool_key_y_z.clone(),
                    x_to_y: false,
                },
                SwapOperation::SwapV3 {
                    pool_key: pool_key_x_y.clone(),
                    x_to_y: false,
                },
            ],
            minimum_receive: None,
        }],
        None,
    )
    .unwrap();

    // get all positions
    let all_positions = get_all_positions!(app, config.dex_v3, bob);

    assert_eq!(all_positions.len(), 2);
}

#[test]
fn test_zap_in_with_minimum_receive() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let _config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key_x_y = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let pool_key_y_z = PoolKey::new(token_y.to_string(), token_z.to_string(), fee_tier).unwrap();

    let tick_lower_index = 0;
    let tick_upper_index = 10;

    // asset_in = token_z
    let asset_in = Asset {
        info: AssetInfo::Token {
            contract_addr: token_z.clone(),
        },
        amount: Uint128::new(1000),
    };

    let minimum_liquidity = Liquidity(10u128.pow(15));
    let err = app
        .zap_in_liquidity(
            &bob,
            zapper.as_str(),
            pool_key_x_y.clone(),
            tick_lower_index,
            tick_upper_index,
            &asset_in,
            vec![
                Route {
                    token_in: token_z.to_string(),
                    offer_amount: Uint128::new(500),
                    operations: vec![
                        SwapOperation::SwapV3 {
                            pool_key: pool_key_y_z.clone(),
                            x_to_y: false,
                        },
                        SwapOperation::SwapV3 {
                            pool_key: pool_key_x_y.clone(),
                            x_to_y: false,
                        },
                    ],
                    minimum_receive: None,
                },
                Route {
                    token_in: token_z.to_string(),
                    offer_amount: Uint128::new(500),
                    operations: vec![SwapOperation::SwapV3 {
                        pool_key: pool_key_y_z.clone(),
                        x_to_y: false,
                    }],
                    minimum_receive: None,
                },
            ],
            Some(minimum_liquidity),
        )
        .unwrap_err();

    assert!(err.root_cause().to_string().contains("Assertion failed"));
}

#[test]
fn test_zap_in_with_fee() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
        ("charlie", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key_x_y = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let pool_key_y_z = PoolKey::new(token_y.to_string(), token_z.to_string(), fee_tier).unwrap();

    let tick_lower_index = 0;
    let tick_upper_index = 10;

    // register protocol fee: 0.1%
    app.register_protocol_fee(
        &alice,
        zapper.as_str(),
        StdDecimal::from_ratio(1u128, 10u128),
        &charlie,
    )
    .unwrap();

    // asset_in = token_z
    let asset_in = Asset {
        info: AssetInfo::Token {
            contract_addr: token_z.clone(),
        },
        amount: Uint128::new(1000),
    };

    // add fail, totalSwap + fee < asset in
    let err = app
        .zap_in_liquidity(
            &bob,
            zapper.as_str(),
            pool_key_x_y.clone(),
            tick_lower_index,
            tick_upper_index,
            &asset_in,
            vec![
                Route {
                    token_in: token_z.to_string(),
                    offer_amount: Uint128::new(500),
                    operations: vec![
                        SwapOperation::SwapV3 {
                            pool_key: pool_key_y_z.clone(),
                            x_to_y: false,
                        },
                        SwapOperation::SwapV3 {
                            pool_key: pool_key_x_y.clone(),
                            x_to_y: false,
                        },
                    ],
                    minimum_receive: None,
                },
                Route {
                    token_in: token_z.to_string(),
                    offer_amount: Uint128::new(500),
                    operations: vec![SwapOperation::SwapV3 {
                        pool_key: pool_key_y_z.clone(),
                        x_to_y: false,
                    }],
                    minimum_receive: None,
                },
            ],
            None,
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Invalid fund"));

    // add successful
    app.zap_in_liquidity(
        &bob,
        zapper.as_str(),
        pool_key_x_y.clone(),
        tick_lower_index,
        tick_upper_index,
        &asset_in,
        vec![
            Route {
                token_in: token_z.to_string(),
                offer_amount: Uint128::new(400),
                operations: vec![
                    SwapOperation::SwapV3 {
                        pool_key: pool_key_y_z.clone(),
                        x_to_y: false,
                    },
                    SwapOperation::SwapV3 {
                        pool_key: pool_key_x_y.clone(),
                        x_to_y: false,
                    },
                ],
                minimum_receive: None,
            },
            Route {
                token_in: token_z.to_string(),
                offer_amount: Uint128::new(500),
                operations: vec![SwapOperation::SwapV3 {
                    pool_key: pool_key_y_z.clone(),
                    x_to_y: false,
                }],
                minimum_receive: None,
            },
        ],
        None,
    )
    .unwrap();
    // get all positions
    let all_positions = get_all_positions!(app, config.dex_v3, bob);
    assert_eq!(all_positions.len(), 1);

    // check balance of fee_receiver
    let fee_receiver_balance = balance_of!(app, token_z, charlie);
    assert_eq!(fee_receiver_balance, 100u128);
}

#[test]
fn test_zap_in_no_routes() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
        ("charlie", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];
    let initial_amount = 10u128.pow(20);
    let (token_x, token_y, token_z) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);
    let config = app.get_zapper_config(zapper.as_str()).unwrap();

    init_basic_v3_pool(
        &mut app, &zapper, &token_x, &token_y, &token_z, &alice, &bob,
    );

    let protocol_fee = Percentage::from_scale(6, 3);
    let fee_tier = FeeTier::new(protocol_fee, 1).unwrap();
    let pool_key_x_y = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let tick_lower_index = 25;
    let tick_upper_index = 30;

    // register protocol fee: 0.1%
    app.register_protocol_fee(
        &alice,
        zapper.as_str(),
        StdDecimal::from_ratio(1u128, 10u128),
        &charlie,
    )
    .unwrap();

    // asset_in = token_x
    let asset_in = Asset {
        info: AssetInfo::Token {
            contract_addr: token_x.clone(),
        },
        amount: Uint128::new(1000),
    };

    // add successful
    app.zap_in_liquidity(
        &bob,
        zapper.as_str(),
        pool_key_x_y.clone(),
        tick_lower_index,
        tick_upper_index,
        &asset_in,
        vec![],
        None,
    )
    .unwrap();
    // get all positions
    let all_positions = get_all_positions!(app, config.dex_v3, bob);
    assert_eq!(all_positions.len(), 1);

    // check balance of fee_receiver
    let fee_receiver_balance = balance_of!(app, token_x, charlie);
    assert_eq!(fee_receiver_balance, 100u128);
}