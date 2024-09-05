use cosmwasm_std::{
    coins, to_json_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, SubMsg, Uint128,
    WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use oraiswap_v3_common::{
    asset::{Asset, AssetInfo},
    oraiswap_v3_msg::{ExecuteMsg, QueryMsg},
    storage::{PoolKey, Position},
};

use crate::{
    contract::{ZAP_IN_LIQUIDITY_REPLY_ID, ZAP_OUT_LIQUIDITY_REPLY_ID},
    entrypoints::utils::get_pool_v3_asset_info,
    msgs::{mixed_router, SwapOperation},
    state::{
        CONFIG, PENDING_POSITION, RECEIVER, SNAP_BALANCE, SNAP_INCENTIVE, ZAP_OUT_POSITION,
        ZAP_OUT_ROUTES,
    },
    Config, ContractError, IncentiveBalance, PairBalance, PendingPosition, ZapOutRoutes,
};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<Addr>,
    mixed_router: Option<Addr>,
    dex_v3: Option<Addr>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let new_config = Config {
        admin: admin.unwrap_or(config.admin),
        mixed_router: mixed_router.unwrap_or(config.mixed_router),
        dex_v3: dex_v3.unwrap_or(config.dex_v3),
    };
    CONFIG.save(deps.storage, &new_config)?;

    let event_attributes = vec![("action", "update_config")];
    Ok(Response::new().add_attributes(event_attributes))
}

pub fn zap_in_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_key: PoolKey,
    tick_lower_index: i32,
    tick_upper_index: i32,
    asset_in: Asset,
    amount_to_x: Uint128,
    amount_to_y: Uint128,
    operation_to_x: Option<Vec<SwapOperation>>,
    operation_to_y: Option<Vec<SwapOperation>>,
    minimum_receive_x: Option<Uint128>,
    minimum_receive_y: Option<Uint128>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut sub_msgs: Vec<SubMsg> = vec![];

    let position_length: u32 = deps.querier.query_wasm_smart(
        config.dex_v3.to_string(),
        &QueryMsg::UserPositionAmount {
            owner: env.contract.address.clone(),
        },
    )?;
    let position = PendingPosition {
        index: position_length,
        pool_key: pool_key.clone(),
        lower_tick: tick_lower_index,
        upper_tick: tick_upper_index,
        liquidity_delta: None,
        slippage_limit_lower: None,
        slippage_limit_upper: None,
    };
    PENDING_POSITION.save(deps.storage, &position)?;
    RECEIVER.save(deps.storage, &info.sender)?;

    match asset_in.info.clone() {
        AssetInfo::Token { contract_addr: _ } => {
            // 1. First transfer from tokenIn amount to this contract
            asset_in.transfer(&mut msgs, &info).unwrap(); // handle error
        }
        _ => {}
    }

    // Snap the balance of tokenX and tokenY in this contract
    let (token_x, token_y) = get_pool_v3_asset_info(deps.api, &pool_key);
    let mut balance_x = token_x.balance(&deps.querier, &env.contract.address)?;
    let mut balance_y = token_y.balance(&deps.querier, &env.contract.address)?;

    if asset_in.info.denom() == token_x.denom() {
        balance_x = balance_x - asset_in.amount;
    }

    if asset_in.info.denom() == token_y.denom() {
        balance_y = balance_y - asset_in.amount;
    }

    SNAP_BALANCE.save(
        deps.storage,
        &PairBalance {
            token_x: Asset {
                info: token_x.clone(),
                amount: balance_x,
            },
            token_y: Asset {
                info: token_y.clone(),
                amount: balance_y,
            },
        },
    )?;

    // 3. Create SubMsg to process swap operations in mixedRouter contract
    // 4. Reply on success, if error occurs, revert the state
    if asset_in.info.denom() == token_x.denom() {
        // just need to swap x to y
        match token_x {
            AssetInfo::Token { contract_addr } => {
                let swap_msg = Cw20ExecuteMsg::Send {
                    contract: config.mixed_router.into_string(),
                    amount: amount_to_y,
                    msg: to_json_binary(&mixed_router::ExecuteMsg::ExecuteSwapOperations {
                        operations: operation_to_y.unwrap(),
                        minimum_receive: minimum_receive_y,
                        to: None,
                        affiliates: None,
                    })?,
                };
                let swap_msg = WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_json_binary(&swap_msg)?,
                    funds: vec![],
                };
                sub_msgs.push(SubMsg::reply_on_success(
                    swap_msg,
                    ZAP_IN_LIQUIDITY_REPLY_ID,
                ))
            }
            AssetInfo::NativeToken { denom } => {
                let swap_msg = mixed_router::ExecuteMsg::ExecuteSwapOperations {
                    operations: operation_to_y.unwrap(),
                    minimum_receive: minimum_receive_y,
                    to: None,
                    affiliates: None,
                };
                let swap_msg = WasmMsg::Execute {
                    contract_addr: config.mixed_router.into_string(),
                    msg: to_json_binary(&swap_msg)?,
                    funds: coins(amount_to_y.u128(), denom),
                };
                sub_msgs.push(SubMsg::reply_on_success(
                    swap_msg,
                    ZAP_IN_LIQUIDITY_REPLY_ID,
                ))
            }
        }
    } else if asset_in.info.denom() == token_y.denom() {
        // just need to swap y to x
        match token_y {
            AssetInfo::Token { contract_addr } => {
                let swap_msg = Cw20ExecuteMsg::Send {
                    contract: config.mixed_router.into_string(),
                    amount: amount_to_x,
                    msg: to_json_binary(&mixed_router::ExecuteMsg::ExecuteSwapOperations {
                        operations: operation_to_x.unwrap(),
                        minimum_receive: minimum_receive_x,
                        to: None,
                        affiliates: None,
                    })?,
                };
                let swap_msg = WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_json_binary(&swap_msg)?,
                    funds: vec![],
                };
                sub_msgs.push(SubMsg::reply_on_success(
                    swap_msg,
                    ZAP_IN_LIQUIDITY_REPLY_ID,
                ))
            }
            AssetInfo::NativeToken { denom } => {
                let swap_msg = mixed_router::ExecuteMsg::ExecuteSwapOperations {
                    operations: operation_to_x.unwrap(),
                    minimum_receive: minimum_receive_x,
                    to: None,
                    affiliates: None,
                };
                let swap_msg = WasmMsg::Execute {
                    contract_addr: config.mixed_router.into_string(),
                    msg: to_json_binary(&swap_msg)?,
                    funds: coins(amount_to_x.u128(), denom),
                };
                sub_msgs.push(SubMsg::reply_on_success(
                    swap_msg,
                    ZAP_IN_LIQUIDITY_REPLY_ID,
                ))
            }
        }
    } else {
        // need to swap two times, asset_in to x, asset_in to y
        match asset_in.info {
            AssetInfo::Token { contract_addr } => {
                // swap operation 1
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: config.mixed_router.clone().into_string(),
                        amount: amount_to_x,
                        msg: to_json_binary(&mixed_router::ExecuteMsg::ExecuteSwapOperations {
                            operations: operation_to_x.unwrap(),
                            minimum_receive: minimum_receive_x,
                            to: None,
                            affiliates: None,
                        })?,
                    })
                    .unwrap(),
                    funds: vec![],
                }));
                // swap operation 2 is subMsg
                let swap_msg = Cw20ExecuteMsg::Send {
                    contract: config.mixed_router.into_string(),
                    amount: amount_to_y,
                    msg: to_json_binary(&mixed_router::ExecuteMsg::ExecuteSwapOperations {
                        operations: operation_to_y.unwrap(),
                        minimum_receive: minimum_receive_y,
                        to: None,
                        affiliates: None,
                    })?,
                };
                let swap_msg = WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_json_binary(&swap_msg)?,
                    funds: vec![],
                };
                sub_msgs.push(SubMsg::reply_on_success(
                    swap_msg,
                    ZAP_IN_LIQUIDITY_REPLY_ID,
                ))
            }
            AssetInfo::NativeToken { denom } => {
                // swap operation 1
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: config.mixed_router.clone().into_string(),
                    msg: to_json_binary(&mixed_router::ExecuteMsg::ExecuteSwapOperations {
                        operations: operation_to_x.unwrap(),
                        minimum_receive: minimum_receive_x,
                        to: None,
                        affiliates: None,
                    })
                    .unwrap(),
                    funds: coins(amount_to_x.u128(), denom.clone()),
                }));
                // swap operation 2 is subMsg
                let swap_msg = mixed_router::ExecuteMsg::ExecuteSwapOperations {
                    operations: operation_to_y.unwrap(),
                    minimum_receive: minimum_receive_y,
                    to: None,
                    affiliates: None,
                };
                let swap_msg = WasmMsg::Execute {
                    contract_addr: config.mixed_router.into_string(),
                    msg: to_json_binary(&swap_msg)?,
                    funds: coins(amount_to_y.u128(), denom),
                };
                sub_msgs.push(SubMsg::reply_on_success(
                    swap_msg,
                    ZAP_IN_LIQUIDITY_REPLY_ID,
                ))
            }
        }
    }

    Ok(Response::new().add_messages(msgs).add_submessages(sub_msgs))
}

pub fn zap_out_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    position_index: u32,
    operation_from_x: Option<Vec<SwapOperation>>,
    operation_from_y: Option<Vec<SwapOperation>>,
    minimum_receive_x: Option<Uint128>,
    minimum_receive_y: Option<Uint128>,
    token_out: AssetInfo,
) -> Result<Response, ContractError> {
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut sub_msgs: Vec<SubMsg> = vec![];
    let config = CONFIG.load(deps.storage)?;
    let position: Position = deps.querier.query_wasm_smart(
        config.dex_v3.to_string(),
        &QueryMsg::Position {
            owner_id: info.sender.clone(),
            index: position_index,
        },
    )?;
    let position_incentives: Vec<Asset> = deps.querier.query_wasm_smart(
        config.dex_v3.to_string(),
        &QueryMsg::PositionIncentives {
            owner_id: info.sender.clone(),
            index: position_index,
        },
    )?;
    let snap_incentives = position_incentives
        .iter()
        .map(|asset| {
            let balance = asset
                .info
                .balance(&deps.querier, &env.contract.address)
                .unwrap();
            Asset {
                info: asset.info.clone(),
                amount: balance,
            }
        })
        .collect::<Vec<Asset>>();
    SNAP_INCENTIVE.save(
        deps.storage,
        &IncentiveBalance {
            incentives: snap_incentives,
        },
    )?;
    ZAP_OUT_ROUTES.save(
        deps.storage,
        &ZapOutRoutes {
            operation_from_x,
            operation_from_y,
            minimum_receive_x,
            minimum_receive_y,
            token_out,
        },
    )?;
    ZAP_OUT_POSITION.save(deps.storage, &position)?;

    // 1. Transfer position to this contract
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.dex_v3.to_string(),
        msg: to_json_binary(&ExecuteMsg::TransferPosition {
            index: position_index,
            receiver: env.contract.address.to_string(),
        })?,
        funds: vec![],
    }));

    // snap balance
    let (token_x, token_y) = get_pool_v3_asset_info(deps.api, &position.pool_key);
    let balance_x = token_x.balance(&deps.querier, &env.contract.address)?;
    let balance_y = token_y.balance(&deps.querier, &env.contract.address)?;

    SNAP_BALANCE.save(
        deps.storage,
        &PairBalance {
            token_x: Asset {
                info: token_x.clone(),
                amount: balance_x,
            },
            token_y: Asset {
                info: token_y.clone(),
                amount: balance_y,
            },
        },
    )?;
    RECEIVER.save(deps.storage, &info.sender)?;

    // 2. Create SubMsg to process remove liquidity in dex_v3 contract
    sub_msgs.push(SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: config.dex_v3.to_string(),
            msg: to_json_binary(&ExecuteMsg::RemovePosition {
                index: position_index,
            })
            .unwrap(),
            funds: vec![],
        },
        ZAP_OUT_LIQUIDITY_REPLY_ID,
    ));

    Ok(Response::new().add_messages(msgs).add_submessages(sub_msgs))
}
