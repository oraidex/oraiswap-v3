use cosmwasm_std::Deps;

use crate::{state::CONFIG, Config, ContractError};

pub fn get_config(deps: Deps) -> Result<Config, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}
