use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, SubMsg, Uint128, WasmMsg,
};
use oraiswap::mixed_router::SwapOperation;
use oraiswap_v3_common::{
    asset::Asset,
    error::ContractError,
    oraiswap_v3_msg::{ExecuteMsg as V3ExecuteMsg, QueryMsg as V3QueryMsg},
    storage::{PoolKey, Position},
};

use crate::{
    contract::{ZAP_IN_LIQUIDITY_REPLY_ID, ZAP_OUT_LIQUIDITY_REPLY_ID},
    entrypoints::common::get_pool_v3_asset_info,
    state::{CONFIG, PENDING_POSITION, RECEIVER, SNAP_INCENTIVE, ZAP_OUT_POSITION, ZAP_OUT_ROUTES},
    Config, IncentiveBalance, PairBalance, PendingPosition, ZapOutRoutes,
};

use super::{build_swap_msg, validate_fund};

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

#[allow(clippy::too_many_arguments)]
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
    // init messages and submessages
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut sub_msgs: Vec<SubMsg> = vec![];

    // transfer the amount or check the fund is sent with request
    validate_fund(
        &deps.querier,
        &info,
        env.contract.address.to_string(),
        asset_in.clone(),
        &mut msgs,
    )?;
    // validate amount_to_x + amount_to_y = asset_in
    if asset_in.amount != amount_to_x + amount_to_y {
        return Err(ContractError::InvalidFund {});
    }

    // load config to get address
    let config = CONFIG.load(deps.storage)?;

    // snap pending position
    let position_length = deps.querier.query_wasm_smart(
        config.dex_v3.to_string(),
        &V3QueryMsg::UserPositionAmount {
            owner: env.contract.address.clone(),
        },
    )?;
    let position = PendingPosition::new(
        position_length,
        pool_key.clone(),
        tick_lower_index,
        tick_upper_index,
        None,
        None,
        None,
    );
    PENDING_POSITION.save(deps.storage, &position)?;

    // snap receiver
    RECEIVER.save(deps.storage, &info.sender)?;

    // Snap the balance of tokenX and tokenY in this contract
    let (token_x, token_y) = get_pool_v3_asset_info(deps.api, &pool_key);
    let mut balance_x = token_x.balance(&deps.querier, env.contract.address.to_string())?;
    let mut balance_y = token_y.balance(&deps.querier, env.contract.address.to_string())?;

    if asset_in.info.eq(&token_x) {
        balance_x -= asset_in.amount;
    }
    if asset_in.info.eq(&token_y) {
        balance_y -= asset_in.amount;
    }
    PairBalance::save(deps.storage, &token_x, balance_x, &token_y, balance_y)?;

    // 3. Create SubMsg to process swap operations in mixedRouter contract
    // 4. Reply on success, if error occurs, revert the state
    if asset_in.info.eq(&token_x) {
        // just need to swap x to y
        let swap_msg = build_swap_msg(
            &asset_in.info,
            config.mixed_router,
            amount_to_y,
            operation_to_y.unwrap(),
            minimum_receive_y,
            None,
            None,
        )?;

        sub_msgs.push(SubMsg::reply_on_success(
            swap_msg,
            ZAP_IN_LIQUIDITY_REPLY_ID,
        ));
    } else if asset_in.info.eq(&token_y) {
        // just need to swap y to x
        let swap_msg = build_swap_msg(
            &asset_in.info,
            config.mixed_router,
            amount_to_x,
            operation_to_x.unwrap(),
            minimum_receive_x,
            None,
            None,
        )?;
        sub_msgs.push(SubMsg::reply_on_success(
            swap_msg,
            ZAP_IN_LIQUIDITY_REPLY_ID,
        ));
    } else {
        let swap_to_x_msg = build_swap_msg(
            &asset_in.info,
            config.mixed_router.clone(),
            amount_to_x,
            operation_to_x.unwrap(),
            minimum_receive_x,
            None,
            None,
        )?;
        let swap_to_y_msg = build_swap_msg(
            &asset_in.info,
            config.mixed_router,
            amount_to_y,
            operation_to_y.unwrap(),
            minimum_receive_y,
            None,
            None,
        )?;

        sub_msgs.push(SubMsg::new(swap_to_x_msg));
        sub_msgs.push(SubMsg::reply_on_success(
            swap_to_y_msg,
            ZAP_IN_LIQUIDITY_REPLY_ID,
        ));
    }

    Ok(Response::new().add_messages(msgs).add_submessages(sub_msgs))
}

#[allow(clippy::too_many_arguments)]
pub fn zap_out_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    position_index: u32,
    operation_from_x: Option<Vec<SwapOperation>>,
    operation_from_y: Option<Vec<SwapOperation>>,
    minimum_receive_x: Option<Uint128>,
    minimum_receive_y: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut sub_msgs: Vec<SubMsg> = vec![];
    let config = CONFIG.load(deps.storage)?;
    let position: Position = deps.querier.query_wasm_smart(
        config.dex_v3.to_string(),
        &V3QueryMsg::Position {
            owner_id: info.sender.clone(),
            index: position_index,
        },
    )?;
    let position_incentives: Vec<Asset> = deps.querier.query_wasm_smart(
        config.dex_v3.to_string(),
        &V3QueryMsg::PositionIncentives {
            owner_id: info.sender.clone(),
            index: position_index,
        },
    )?;
    let snap_incentives = position_incentives
        .iter()
        .map(|asset| {
            let balance = asset
                .info
                .balance(&deps.querier, env.contract.address.to_string())
                .unwrap();
            Asset {
                info: asset.info.clone(),
                amount: balance,
            }
        })
        .collect();
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
        },
    )?;
    ZAP_OUT_POSITION.save(deps.storage, &position)?;

    // 1. Transfer position to this contract
    // sender must be approve for contract first
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.dex_v3.to_string(),
        msg: to_json_binary(&V3ExecuteMsg::TransferNft {
            token_id: position.token_id,
            recipient: env.contract.address.clone(),
        })?,
        funds: vec![],
    }));

    // snap balance
    let (token_x, token_y) = get_pool_v3_asset_info(deps.api, &position.pool_key);
    let balance_x = token_x.balance(&deps.querier, env.contract.address.to_string())?;
    let balance_y = token_y.balance(&deps.querier, env.contract.address.to_string())?;

    PairBalance::save(deps.storage, &token_x, balance_x, &token_y, balance_y)?;
    RECEIVER.save(deps.storage, &info.sender)?;

    // 2. Create SubMsg to process remove liquidity in dex_v3 contract
    sub_msgs.push(SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: config.dex_v3.to_string(),
            msg: to_json_binary(&V3ExecuteMsg::Burn {
                token_id: position.token_id,
            })?,
            funds: vec![],
        },
        ZAP_OUT_LIQUIDITY_REPLY_ID,
    ));

    Ok(Response::new().add_messages(msgs).add_submessages(sub_msgs))
}
