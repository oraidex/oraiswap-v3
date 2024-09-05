use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use oraiswap_v3_common::asset::AssetInfo;

use crate::msgs::SwapOperation;

#[cw_serde]
pub struct ZapOutRoutes {
    pub operation_from_x: Option<Vec<SwapOperation>>,
    pub operation_from_y: Option<Vec<SwapOperation>>,
    pub minimum_receive_x: Option<Uint128>,
    pub minimum_receive_y: Option<Uint128>,
    pub token_out: AssetInfo,
}