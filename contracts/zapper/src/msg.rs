use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use oraiswap::mixed_router::SwapOperation;
use oraiswap_v3_common::{asset::Asset, math::liquidity::Liquidity, storage::PoolKey};

use crate::Config;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Addr,
    pub mixed_router: Addr,
    pub dex_v3: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        admin: Option<Addr>,
        mixed_router: Option<Addr>,
        dex_v3: Option<Addr>,
    },
    ZapInLiquidity {
        pool_key: PoolKey,
        tick_lower_index: i32,
        tick_upper_index: i32,
        asset_in: Asset,
        routes: Vec<Route>,
        minimum_liquidity: Option<Liquidity>,
    },
    ZapOutLiquidity {
        position_index: u32,
        operation_from_x: Option<Vec<SwapOperation>>,
        operation_from_y: Option<Vec<SwapOperation>>,
        minimum_receive_x: Option<Uint128>,
        minimum_receive_y: Option<Uint128>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct Route {
    pub offer_amount: Uint128,
    pub operations: Vec<SwapOperation>,
}
