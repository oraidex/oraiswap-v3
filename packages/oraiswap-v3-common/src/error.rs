use std::string::FromUtf8Error;

use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

use crate::math::liquidity::Liquidity;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    FromUtf8(#[from] FromUtf8Error),

    #[error("{0}")]
    CheckMathOverUnderFlowError(String),

    #[error("invalid tick spacing")]
    InvalidTickSpacing,

    #[error("invalid fee")]
    InvalidFee,

    #[error("invalid tick index")]
    InvalidTickIndex,

    #[error("invalid tick")]
    InvalidTick,

    #[error("tokens are the same")]
    TokensAreSame,

    #[error("invalid tick")]
    InvalidInitTick,

    #[error("invalid sqrt price")]
    InvalidInitSqrtPrice,

    #[error("invalid offset")]
    InvalidOffset,

    #[error("Assertion failed; require minimum amount: {transfer_amount}")]
    InvalidFunds { transfer_amount: Uint128 },

    #[error("multiplication overflow")]
    Mul,

    #[error("division overflow or division by zero")]
    Div,

    #[error("type failed")]
    Cast,

    #[error("addition overflow")]
    Add,

    #[error("subtraction underflow")]
    Sub,

    #[error("empty position pokes")]
    EmptyPositionPokes,

    #[error("price limit reached")]
    PriceLimitReached,

    #[error("insufficient liquidity")]
    InsufficientLiquidity,

    #[error("current_timestamp - pool.start_timestamp underflow")]
    TimestampSubOverflow,

    #[error("tick limit reached")]
    TickLimitReached,

    #[error("tick not found")]
    TickNotFound,

    #[error("tick already exist")]
    TickAlreadyExist,

    #[error("invalid tick liquidity")]
    InvalidTickLiquidity,

    #[error("invalid size")]
    InvalidSize,

    #[error("sqrt_price out of range")]
    SqrtPriceOutOfRange,

    #[error("current_timestamp > last_timestamp failed")]
    TimestampCheckFailed,

    #[error("can not parse from u320 to u256")]
    U320ToU256,

    #[error("tick over bounds")]
    TickOverBounds,

    #[error("calculate_sqrt_price: parsing from scale failed")]
    ParseFromScale,

    #[error("calcaule_sqrt_price::checked_div division failed")]
    CheckedDiv,

    #[error("big_liquidity -/+ sqrt_price * x")]
    BigLiquidityOverflow,

    #[error("upper_tick is not greater than lower_tick")]
    UpperTickNotGreater,

    #[error("tick_lower > tick_upper")]
    TickLowerGreater,

    #[error("tick initialize tick again")]
    TickReInitialize,

    #[error("Upper Sqrt Price < Current Sqrt Price")]
    UpperSqrtPriceLess,

    #[error("Current Sqrt Price < Lower Sqrt Price")]
    CurrentSqrtPriceLess,

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Cannot set approval that is already expired")]
    Expired {},

    #[error("amount is zero")]
    AmountIsZero,

    #[error("wrong limit")]
    WrongLimit,

    #[error("no gain swap")]
    NoGainSwap,

    #[error("amount under minimum amount out")]
    AmountUnderMinimumAmountOut,

    #[error("pool already exist")]
    PoolAlreadyExist,

    #[error("FeeTierNotFound")]
    FeeTierNotFound,

    #[error("NotEmptyTickDeinitialization")]
    NotEmptyTickDeinitialization,

    #[error("Invalid Reply ID")]
    UnrecognizedReplyId { id: u64 },

    #[error("No fund is sent")]
    NoFundSent {},

    #[error("Invalid fund")]
    InvalidFund {},

    #[error("Missing route swap")]
    MissingRouteSwap {},

    #[error("Assertion failed; expect: {minium_receive}, got: {return_amount}")]
    ZapInAssertionFailure {
        minium_receive: Liquidity,
        return_amount: Liquidity,
    },

    #[error("Error on zap out: not enough balance to swap")]
    ZapOutNotEnoughBalanceToSwap {},

    #[error("Pool paused")]
    PoolPaused {},
}

impl From<ContractError> for StdError {
    fn from(source: ContractError) -> Self {
        Self::generic_err(source.to_string())
    }
}

// Implementing From<String> for ContractError
impl From<String> for ContractError {
    fn from(error: String) -> Self {
        ContractError::CheckMathOverUnderFlowError(error)
    }
}

// Implementing From<&str> for ContractError
impl From<&str> for ContractError {
    fn from(error: &str) -> Self {
        ContractError::CheckMathOverUnderFlowError(error.to_string())
    }
}
