use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response};

use crate::{state::CONFIG, Config, ContractError};

pub fn update_config(deps: DepsMut, info: MessageInfo, admin: Option<Addr>, mixed_router: Option<Addr>, dex_v3: Option<Addr>) -> Result<Response, ContractError> {
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

    let event_attributes = vec![
        ("action", "update_config")
    ];
    Ok(Response::new().add_attributes(event_attributes))
}