use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use oraiswap::mixed_router::SwapOperation;
use oraiswap_v3_common::{asset::Asset, math::liquidity::Liquidity, storage::PoolKey};

use crate::{Config, ProtocolFee};

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
    ZapInAfterSwapOperation {},
    RefundAfterZapInLiquidity {},
    ZapOutLiquidity {
        position_index: u32,
        routes: Vec<Route>,
    },
    ZapOutAfterSwapOperation {},
    RegisterProtocolFee {
        percent: Decimal,
        fee_receiver: Addr,
    },
    Withdraw {
        assets: Vec<Asset>,
        recipient: Option<Addr>,
    }
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},

    #[returns(ProtocolFee)]
    ProtocolFee {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct Route {
    pub token_in: String,
    pub offer_amount: Uint128,
    pub operations: Vec<SwapOperation>,
    pub minimum_receive: Option<Uint128>,
}
