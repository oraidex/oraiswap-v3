use cosmwasm_std::Deps;
use oraiswap_v3_common::error::ContractError;

use crate::{state::CONFIG, Config};

pub fn get_config(deps: Deps) -> Result<Config, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}
