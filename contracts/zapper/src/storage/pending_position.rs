use cosmwasm_schema::cw_serde;
use oraiswap_v3_common::{
    math::{liquidity::Liquidity, sqrt_price::SqrtPrice},
    storage::PoolKey,
};

#[cw_serde]
pub struct PendingPosition {
    pub index: u32,
    pub pool_key: PoolKey,
    pub lower_tick: i32,
    pub upper_tick: i32,
    pub liquidity_delta: Option<Liquidity>,
    pub slippage_limit_lower: Option<SqrtPrice>,
    pub slippage_limit_upper: Option<SqrtPrice>,
}
