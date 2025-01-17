#![allow(unused_imports)]
use crate::{
    asset::{Asset, AssetInfo},
    storage::PoolStatus,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint64};
use cw20::Expiration;

use crate::{
    interface::{
        AllNftInfoResponse, ApprovedForAllResponse, NftInfoResponse, NumTokensResponse,
        OwnerOfResponse, PoolWithPoolKey, PositionTick, QuoteResult, SwapHop, TokensResponse,
    },
    math::{
        liquidity::Liquidity, percentage::Percentage, sqrt_price::SqrtPrice,
        token_amount::TokenAmount,
    },
    storage::{FeeTier, LiquidityTick, Pool, PoolKey, Position, Tick},
};
#[allow(unused_imports)]
#[cw_serde]
pub struct InstantiateMsg {
    pub protocol_fee: Percentage,
    pub incentives_fund_manager: Addr,
}

#[cw_serde]
pub struct NftExtensionMsg {
    pub pool_key: PoolKey,
    pub lower_tick: i32,
    pub upper_tick: i32,
    pub liquidity_delta: Liquidity,
    pub slippage_limit_lower: SqrtPrice,
    pub slippage_limit_upper: SqrtPrice,
}

#[cw_serde]
pub enum ExecuteMsg {
    ChangeAdmin {
        new_admin: Addr,
    },
    WithdrawProtocolFee {
        pool_key: PoolKey,
    },
    WithdrawAllProtocolFee {
        receiver: Option<Addr>,
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
    SwapRoute {
        amount_in: TokenAmount,
        expected_amount_out: TokenAmount,
        slippage: Percentage,
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
        token_0: String,
        token_1: String,
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
    /// Transfer is a base message to move a token to another account without triggering actions
    TransferNft {
        recipient: Addr,
        token_id: u64,
    },
    /// Mint a new NFT, can only be called by the contract minter
    Mint {
        /// Any custom extension used by this contract
        extension: NftExtensionMsg,
    },
    Burn {
        token_id: u64,
    },
    /// Send is a base message to transfer a token to a contract and trigger an action
    /// on the receiving contract.
    SendNft {
        contract: Addr,
        token_id: u64,
        msg: Option<Binary>,
    },
    /// Allows operator to transfer / send the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve {
        spender: Addr,
        token_id: u64,
        expires: Option<Expiration>,
    },
    /// Remove previously granted Approval
    Revoke {
        spender: Addr,
        token_id: u64,
    },
    /// Allows operator to transfer / send any token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    ApproveAll {
        operator: Addr,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll {
        operator: Addr,
    },
    // create incentives for specific pool
    CreateIncentive {
        pool_key: PoolKey,
        reward_token: AssetInfo,
        total_reward: Option<TokenAmount>,
        reward_per_sec: TokenAmount,
        start_timestamp: Option<u64>,
    },
    // update  for specific pool
    UpdateIncentive {
        pool_key: PoolKey,
        incentive_id: u64,
        remaining_reward: Option<TokenAmount>,
        start_timestamp: Option<u64>,
        reward_per_sec: Option<TokenAmount>,
    },
    // Claim Incentives
    ClaimIncentive {
        index: u32,
    },
    // update pool status
    UpdatePoolStatus {
        pool_key: PoolKey,
        status: Option<PoolStatus>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    Admin {},

    #[returns(Percentage)]
    ProtocolFee {},

    #[returns(Addr)]
    IncentivesFundManager {},

    #[returns(Position)]
    Position { owner_id: Addr, index: u32 },

    #[returns(Vec<Position>)]
    Positions {
        owner_id: Addr,
        limit: Option<u32>,
        offset: Option<u32>,
    },

    #[returns(Vec<Position>)]
    AllPosition {
        limit: Option<u32>,
        start_after: Option<Binary>,
    },

    #[returns(bool)]
    FeeTierExist { fee_tier: FeeTier },

    #[returns(Pool)]
    Pool {
        token_0: String,
        token_1: String,
        fee_tier: FeeTier,
    },

    #[returns(Vec<PoolWithPoolKey>)]
    Pools {
        limit: Option<u32>,
        start_after: Option<PoolKey>,
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

    #[returns(Vec<(u16, Uint64)>)]
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

    #[returns(Vec<PoolWithPoolKey>)]
    PoolsForPair { token_0: String, token_1: String },

    #[returns(QuoteResult)]
    Quote {
        pool_key: PoolKey,
        x_to_y: bool,
        amount: TokenAmount,
        by_amount_in: bool,
        sqrt_price_limit: SqrtPrice,
    },

    #[returns(TokenAmount)]
    QuoteRoute {
        amount_in: TokenAmount,
        swaps: Vec<SwapHop>,
    },

    ///
    ///
    ///  NFT methods
    ///
    ///
    ///
    /// Total number of tokens issued
    #[returns(NumTokensResponse)]
    NumTokens {},

    /// Return the owner of the given token, error if token does not exist
    /// Return type: OwnerOfResponse
    #[returns(OwnerOfResponse)]
    OwnerOf {
        token_id: u64,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },
    /// List all operators that can access all of the owner's tokens.
    #[returns(ApprovedForAllResponse)]
    ApprovedForAll {
        owner: Addr,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    /// With MetaData Extension.
    /// Returns metadata about one particular token, based on *ERC721 Metadata JSON Schema*
    /// but directly from the contract: `NftInfoResponse`
    #[returns(NftInfoResponse)]
    NftInfo { token_id: u64 },
    /// With MetaData Extension.
    /// Returns the result of both `NftInfo` and `OwnerOf` as one query as an optimization
    #[returns(AllNftInfoResponse)]
    AllNftInfo {
        token_id: u64,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },
    /// With Enumerable extension.
    /// Returns all tokens owned by the given address, [] if unset.
    /// Return type: TokensResponse.
    #[returns(TokensResponse)]
    Tokens {
        owner: Addr,
        start_after: Option<u32>,
        limit: Option<u32>,
    },
    /// With Enumerable extension.
    /// Requires pagination. Lists all token_ids controlled by the contract.
    /// Return type: TokensResponse.
    #[returns(TokensResponse)]
    AllTokens {
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(Vec<Asset>)]
    PositionIncentives { owner_id: Addr, index: u32 },

    #[returns(Vec<PoolWithPoolKey>)]
    PoolsByPoolKeys { pool_keys: Vec<PoolKey> },
}
