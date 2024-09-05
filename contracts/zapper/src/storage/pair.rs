use cosmwasm_schema::cw_serde;
use oraiswap_v3_common::asset::Asset;

#[cw_serde]
pub struct PairBalance {
    pub tokenX: Asset,
    pub tokenY: Asset,
}
