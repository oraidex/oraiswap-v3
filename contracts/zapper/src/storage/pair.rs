use cosmwasm_schema::cw_serde;
use oraiswap_v3_common::asset::Asset;

#[cw_serde]
pub struct PairBalance {
    pub token_x: Asset,
    pub token_y: Asset,
}
