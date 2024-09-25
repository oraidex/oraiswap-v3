use cosmwasm_std::{
    wasm_execute, Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, SubMsg, Uint128,
};

use oraiswap_v3_common::{
    asset::{Asset, AssetInfo},
    error::ContractError,
    math::liquidity::Liquidity,
    oraiswap_v3_msg::{ExecuteMsg as V3ExecuteMsg, QueryMsg as V3QueryMsg},
    storage::{PoolKey, Position},
};

use crate::{
    contract::{ZAP_IN_LIQUIDITY_REPLY_ID, ZAP_OUT_LIQUIDITY_REPLY_ID},
    entrypoints::common::get_pool_v3_asset_info,
    msg::Route,
    state::{CONFIG, PENDING_POSITION, PROTOCOL_FEE, RECEIVER, SNAP_BALANCES, ZAP_OUT_ROUTES},
    Config, PairBalance, PendingPosition,
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

pub fn execute_register_protocol_fee(
    deps: DepsMut,
    info: MessageInfo,
    percent: Decimal,
    fee_receiver: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // validate percent must be < 1
    if percent.gt(&Decimal::one()) {
        return Err(ContractError::InvalidFee {});
    }

    PROTOCOL_FEE.save(
        deps.storage,
        &crate::ProtocolFee {
            percent,
            fee_receiver: fee_receiver.clone(),
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        ("action", "register_protocol_fee"),
        ("percent", &percent.to_string()),
        ("fee_receiver", fee_receiver.as_str()),
    ]))
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
    routes: Vec<Route>,
    minimum_liquidity: Option<Liquidity>,
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

    let mut amount_after_fee = asset_in.amount;
    // handle deduct zap in fee
    if let Some(protocol_fee) = PROTOCOL_FEE.may_load(deps.storage)? {
        let fee_amount = asset_in.amount * protocol_fee.percent;
        amount_after_fee -= fee_amount;
        // transfer fee to fee_receiver
        asset_in
            .info
            .transfer(&mut msgs, protocol_fee.fee_receiver.to_string(), fee_amount)?;
    }

    // validate asset_in and routes
    let total_swap_amount: Uint128 = routes.iter().map(|route| route.offer_amount).sum();
    if total_swap_amount.gt(&amount_after_fee) {
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
        minimum_liquidity,
    );
    PENDING_POSITION.save(deps.storage, &position)?;

    // snap receiver
    RECEIVER.save(deps.storage, &info.sender)?;

    // Snap the balance of tokenX and tokenY in this contract
    let (token_x, token_y) = get_pool_v3_asset_info(deps.api, &pool_key);
    let mut balance_x = token_x.balance(&deps.querier, env.contract.address.to_string())?;
    let mut balance_y = token_y.balance(&deps.querier, env.contract.address.to_string())?;

    if let AssetInfo::NativeToken { denom: _ } = &asset_in.info {
        if asset_in.info.eq(&token_x) {
            balance_x -= asset_in.amount;
        } else if asset_in.info.eq(&token_y) {
            balance_y -= asset_in.amount;
        }
    }

    PairBalance::save(deps.storage, &token_x, balance_x, &token_y, balance_y)?;

    // 3. Create SubMsg to process swap operations in mixedRouter contract
    // 4. Reply on success, if error occurs, revert the state
    for i in 0..routes.len() {
        let swap_msg = build_swap_msg(
            &asset_in.info,
            config.mixed_router.clone(),
            routes[i].offer_amount,
            routes[i].operations.clone(),
            None,
            None,
            None,
        )?;
        if i == routes.len() - 1 {
            sub_msgs.push(SubMsg::reply_on_success(
                swap_msg,
                ZAP_IN_LIQUIDITY_REPLY_ID,
            ));
        } else {
            sub_msgs.push(SubMsg::new(swap_msg));
        }
    }

    Ok(Response::new().add_messages(msgs).add_submessages(sub_msgs))
}

pub fn zap_out_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    position_index: u32,
    routes: Vec<Route>,
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
    for incentive in position_incentives {
        let balance = incentive
            .info
            .balance(&deps.querier, env.contract.address.to_string())?;
        SNAP_BALANCES.save(deps.storage, incentive.info.denom(), &balance)?;
    }

    // 1. Transfer position to this contract
    // sender must be approve for contract first
    msgs.push(
        wasm_execute(
            config.dex_v3.as_str(),
            &V3ExecuteMsg::TransferNft {
                token_id: position.token_id,
                recipient: env.contract.address.clone(),
            },
            vec![],
        )?
        .into(),
    );

    // snap balance
    let (token_x, token_y) = get_pool_v3_asset_info(deps.api, &position.pool_key);
    let balance_x = token_x.balance(&deps.querier, env.contract.address.to_string())?;
    let balance_y = token_y.balance(&deps.querier, env.contract.address.to_string())?;
    SNAP_BALANCES.save(deps.storage, token_x.denom(), &balance_x)?;
    SNAP_BALANCES.save(deps.storage, token_y.denom(), &balance_y)?;

    RECEIVER.save(deps.storage, &info.sender)?;
    ZAP_OUT_ROUTES.save(deps.storage, &routes)?;

    // 2. Create SubMsg to process remove liquidity in dex_v3 contract
    sub_msgs.push(SubMsg::reply_on_success(
        wasm_execute(
            config.dex_v3.as_str(),
            &V3ExecuteMsg::Burn {
                token_id: position.token_id,
            },
            vec![],
        )?,
        ZAP_OUT_LIQUIDITY_REPLY_ID,
    ));

    Ok(Response::new().add_messages(msgs).add_submessages(sub_msgs))
}
