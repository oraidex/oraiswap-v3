use cosmwasm_std::Deps;
use oraiswap_v3_common::error::ContractError;

use crate::{
    state::{CONFIG, PROTOCOL_FEE},
    Config, ProtocolFee,
};

pub fn get_config(deps: Deps) -> Result<Config, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

pub fn get_protocol_fee(deps: Deps) -> Result<ProtocolFee, ContractError> {
    let protocol_fee = PROTOCOL_FEE.load(deps.storage)?;
    Ok(protocol_fee)
}
