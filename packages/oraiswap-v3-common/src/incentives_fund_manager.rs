use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::asset::Asset;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<Addr>,
    pub oraiswap_v3: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        owner: Option<Addr>,
        oraiswap_v3: Option<Addr>,
    },
    SendFund {
        asset: Asset,
        receiver: Addr,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub oraiswap_v3: Addr,
}
