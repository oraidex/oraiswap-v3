use crate::state::{Config, CONFIG};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use oraiswap_v3_common::{
    asset::Asset,
    error::ContractError,
    incentives_fund_manager::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:incentives-fund-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            owner: msg.owner.unwrap_or(info.sender),
            oraiswap_v3: msg.oraiswap_v3,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { owner, oraiswap_v3 } => {
            execute_update_config(deps, info, owner, oraiswap_v3)
        }
        ExecuteMsg::SendFund { asset, receiver } => execute_send_fund(deps, info, asset, receiver),
    }
}

/// Allows owner can adjust config
///
/// # Parameters
/// - `owner`: new owner
/// - `oraiswap_v3`: new oraiswapV3 contract
///
/// # Errors
/// - Reverts the call when the caller is an unauthorized user
///
fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<Addr>,
    oraiswap_v3: Option<Addr>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        config.owner = owner;
    }
    if let Some(oraiswap_v3) = oraiswap_v3 {
        config.oraiswap_v3 = oraiswap_v3;
    }
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Allows oraiswap_v3_contract can send fund
///
/// # Parameters
/// - `asset`: asset to send.
/// - `receiver`: receiver address
///
/// # Errors
/// - Reverts the call when the caller is an unauthorized user or contract not enough fund
///
fn execute_send_fund(
    deps: DepsMut,
    info: MessageInfo,
    asset: Asset,
    receiver: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.oraiswap_v3 {
        return Err(ContractError::Unauthorized {});
    }

    let mut msgs: Vec<CosmosMsg> = vec![];
    asset.transfer(
        &mut msgs,
        &MessageInfo {
            sender: receiver,
            funds: vec![],
        },
    )?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "send_fund"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
    }
}
