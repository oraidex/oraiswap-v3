use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[allow(unused_imports)]
use crate::{
    fee_growth::FeeGrowth, interface::SwapHop, liquidity::Liquidity, percentage::Percentage,
    sqrt_price::SqrtPrice, token_amount::TokenAmount, FeeTier, Pool, PoolKey, Position, Tick,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub protocol_fee: Percentage,
}

#[cw_serde]
pub enum ExecuteMsg {
    WithdrawProtocolFee {
        pool_key: PoolKey,
    },
    ChangeProtocolFee {
        protocol_fee: Percentage,
    },
    ChangeFeeReceiver {
        pool_key: PoolKey,
        fee_receiver: Addr,
    },
    CreatePosition {
        pool_key: PoolKey,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_delta: Liquidity,
        slippage_limit_lower: SqrtPrice,
        slippage_limit_upper: SqrtPrice,
    },
    Swap {
        pool_key: PoolKey,
        x_to_y: bool,
        amount: TokenAmount,
        by_amount_in: bool,
        sqrt_price_limit: SqrtPrice,
    },
    QuoteRoute {
        amount_in: TokenAmount,
        swaps: Vec<SwapHop>,
    },
    TransferPosition {
        index: u32,
        receiver: String,
    },
    ClaimFee {
        index: u32,
    },
    RemovePosition {
        index: u32,
    },
    CreatePool {
        token_0: Addr,
        token_1: Addr,
        fee_tier: FeeTier,
        init_sqrt_price: SqrtPrice,
        init_tick: i32,
    },
    AddFeeTier {
        fee_tier: FeeTier,
    },
    RemoveFeeTier {
        fee_tier: FeeTier,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Percentage)]
    ProtocolFee {},

    #[returns(Position)]
    Position { owner_id: Addr, index: u32 },

    #[returns(Vec<Position>)]
    Positions {
        owner_id: Addr,
        limit: Option<u32>,
        offset: Option<u32>,
    },

    #[returns(bool)]
    FeeTierExist { fee_tier: FeeTier },

    #[returns(Pool)]
    Pool {
        token_0: Addr,
        token_1: Addr,
        fee_tier: FeeTier,
    },

    #[returns(Vec<PoolKey>)]
    Pools {
        limit: Option<u32>,
        offset: Option<u32>,
    },

    #[returns(Tick)]
    Tick { key: PoolKey, index: i32 },

    #[returns(bool)]
    IsTickInitialized { key: PoolKey, index: i32 },

    #[returns(Vec<FeeTier>)]
    FeeTiers {},

    #[returns(Vec<PositionTick>)]
    PositionTicks { owner: Addr, offset: u32 },

    #[returns(u32)]
    UserPositionAmount { owner: Addr },

    #[returns(Vec<(u16, u64)>)]
    TickMap {
        pool_key: PoolKey,
        lower_tick_index: i32,
        upper_tick_index: i32,
        x_to_y: bool,
    },

    #[returns(Vec<LiquidityTick>)]
    LiquidityTicks {
        pool_key: PoolKey,
        tick_indexes: Vec<i32>,
    },

    #[returns(u32)]
    LiquidityTicksAmount {
        pool_key: PoolKey,
        lower_tick: i32,
        upper_tick: i32,
    },

    #[returns(Vec<Pool>)]
    PoolsForPair { token0: Addr, token1: Addr },

    #[returns(QuoteResult)]
    Quote {
        pool_key: PoolKey,
        x_to_y: bool,
        amount: TokenAmount,
        by_amount_in: bool,
        sqrt_price_limit: SqrtPrice,
    },
}

#[cw_serde]
pub struct PositionTick {
    pub index: i32,
    pub fee_growth_outside_x: FeeGrowth,
    pub fee_growth_outside_y: FeeGrowth,
    pub seconds_outside: u64,
}

#[cw_serde]
pub struct LiquidityTick {
    pub index: i32,
    pub liquidity_change: Liquidity,
    pub sign: bool,
}

#[cw_serde]
pub struct QuoteResult {
    pub amount_in: TokenAmount,
    pub amount_out: TokenAmount,
    pub target_sqrt_price: SqrtPrice,
    pub ticks: Vec<Tick>,
}