// use cosmwasm_std::{
//     testing::{mock_dependencies, mock_env, mock_info},
//     Order,
// };
// use decimal::Decimal;

// use crate::{
//     contract::{instantiate, migrate},
//     entrypoints::{self},
//     liquidity::Liquidity,
//     msg::{self, InstantiateMsg},
//     percentage::Percentage,
//     sqrt_price::{calculate_sqrt_price, SqrtPrice},
//     state::{self},
//     FeeTier, PoolKey,
// };

// #[test]
// fn test_migrate_contract() {
//     // fixture
//     let mut deps = mock_dependencies();

//     let info = mock_info("alice", &[]);

//     instantiate(
//         deps.as_mut(),
//         mock_env(),
//         info.clone(),
//         InstantiateMsg {
//             protocol_fee: Percentage::new(0),
//         },
//     )
//     .unwrap();

//     let fee_tier = FeeTier::new(Percentage::new(0), 1).unwrap();

//     entrypoints::add_fee_tier(deps.as_mut(), mock_env(), info.clone(), fee_tier).unwrap();

//     let init_tick = 10;
//     let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
//     entrypoints::create_pool(
//         deps.as_mut(),
//         mock_env(),
//         "token_x".to_string(),
//         "token_y".to_string(),
//         fee_tier,
//         init_sqrt_price,
//         init_tick,
//     )
//     .unwrap();

//     let pool_key = PoolKey::new("token_x".to_string(), "token_y".to_string(), fee_tier).unwrap();
//     let tick_indexes = [-9780, -42, 0, 9, 276, 32343, 50001];
//     for i in 0..tick_indexes.len() - 1 {
//         entrypoints::create_position(
//             deps.as_mut(),
//             mock_env(),
//             info.clone(),
//             pool_key.clone(),
//             tick_indexes[i],
//             tick_indexes[i + 1],
//             Liquidity::new(10),
//             SqrtPrice::new(0),
//             SqrtPrice::max_instance(),
//         )
//         .unwrap();
//     }

//     // now can query first NFT info
//     let nft_info = entrypoints::query_nft_info(deps.as_ref(), 1).unwrap();
//     assert_eq!(nft_info.extension.token_id, 1);

//     // we will reset to old storage
//     state::POSITION_KEYS_BY_TOKEN_ID.clear(deps.as_mut().storage);
//     state::TOKEN_COUNT.remove(deps.as_mut().storage);
//     state::TOKEN_ID.remove(deps.as_mut().storage);

//     // and also reset all token_id in positions
//     let positions: Vec<_> = state::POSITIONS
//         .range_raw(deps.as_mut().storage, None, None, Order::Ascending)
//         .collect();
//     for item in positions {
//         if let Ok((key, mut position)) = item {
//             position.token_id = 0;
//             // update position and its index
//             crate::state::POSITIONS
//                 .save(deps.as_mut().storage, &key, &position)
//                 .unwrap();
//         }
//     }

//     // must return error
//     entrypoints::query_nft_info(deps.as_ref(), 1).unwrap_err();
//     let num_tokens = entrypoints::query_num_tokens(deps.as_ref()).unwrap();
//     assert_eq!(num_tokens.count, 0);

//     // then migrate contract
//     migrate(deps.as_mut(), mock_env(), msg::MigrateMsg {}).unwrap();

//     // now can query first NFT info again
//     let nft_info = entrypoints::query_nft_info(deps.as_ref(), 1).unwrap();
//     assert_eq!(nft_info.extension.token_id, 1);

//     // total tokens is total ranges in tick_indexes
//     let num_tokens = entrypoints::query_num_tokens(deps.as_ref()).unwrap();
//     assert_eq!(num_tokens.count as usize, tick_indexes.len() - 1);
// }
