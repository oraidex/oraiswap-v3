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
    pub minimum_liquidity: Option<Liquidity>,
}

impl PendingPosition {
    pub fn new(
        index: u32,
        pool_key: PoolKey,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_delta: Option<Liquidity>,
        slippage_limit_lower: Option<SqrtPrice>,
        slippage_limit_upper: Option<SqrtPrice>,
        minimum_liquidity: Option<Liquidity>,
    ) -> Self {
        Self {
            index,
            pool_key,
            lower_tick,
            upper_tick,
            liquidity_delta,
            slippage_limit_lower,
            slippage_limit_upper,
            minimum_liquidity,
        }
    }
}
