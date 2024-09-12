#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use oraiswap_v3_common::error::ContractError;

use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::CONFIG;
use crate::{entrypoints::*, Config};

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::set_contract_version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:zapper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// id for each reply
pub const ZAP_IN_LIQUIDITY_REPLY_ID: u64 = 1;
pub const ZAP_OUT_LIQUIDITY_REPLY_ID: u64 = 2;
pub const ADD_LIQUIDITY_REPLY_ID: u64 = 3;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        admin: info.sender,
        dex_v3: msg.dex_v3,
        mixed_router: msg.mixed_router,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            admin,
            mixed_router,
            dex_v3,
        } => update_config(deps, info, admin, mixed_router, dex_v3),
        ExecuteMsg::ZapInLiquidity {
            pool_key,
            tick_lower_index,
            tick_upper_index,
            asset_in,
            routes,
            minimum_liquidity,
        } => zap_in_liquidity(
            deps,
            env,
            info,
            pool_key,
            tick_lower_index,
            tick_upper_index,
            asset_in,
            routes,
            minimum_liquidity,
        ),
        ExecuteMsg::ZapOutLiquidity {
            position_index,
            routes,
        } => zap_out_liquidity(deps, env, info, position_index, routes),
        ExecuteMsg::RegisterProtocolFee {
            percent,
            fee_receiver,
        } => execute_register_protocol_fee(deps, info, percent, fee_receiver),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&get_config(deps)?),
        QueryMsg::ProtocolFee {} => to_json_binary(&get_protocol_fee(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let original_version =
        cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("new_version", original_version.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        ZAP_IN_LIQUIDITY_REPLY_ID => reply::zap_in_liquidity(deps, env),
        ZAP_OUT_LIQUIDITY_REPLY_ID => reply::zap_out_liquidity(deps, env),
        ADD_LIQUIDITY_REPLY_ID => reply::add_liquidity(deps, env),
        _ => Err(ContractError::UnrecognizedReplyId { id: reply.id }),
    }
}
