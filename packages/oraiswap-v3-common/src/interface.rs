use cosmwasm_schema::cw_serde;
use cosmwasm_std::{wasm_execute, Addr, Binary, CosmosMsg, StdResult};
use cw20::Expiration;

use crate::{
    math::{fee_growth::FeeGrowth, sqrt_price::SqrtPrice, token_amount::TokenAmount},
    storage::{Pool, PoolKey, Position, Tick},
};

#[cw_serde]
pub struct CalculateSwapResult {
    pub amount_in: TokenAmount,
    pub amount_out: TokenAmount,
    pub start_sqrt_price: SqrtPrice,
    pub target_sqrt_price: SqrtPrice,
    pub fee: TokenAmount,
    pub pool: Pool,
    pub ticks: Vec<Tick>,
}

#[cw_serde]
pub struct SwapHop {
    pub pool_key: PoolKey,
    pub x_to_y: bool,
}

#[cw_serde]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: Addr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

#[cw_serde]
pub struct PositionTick {
    pub index: i32,
    pub fee_growth_outside_x: FeeGrowth,
    pub fee_growth_outside_y: FeeGrowth,
    pub seconds_outside: u64,
}

#[cw_serde]
pub struct PoolWithPoolKey {
    pub pool: Pool,
    pub pool_key: PoolKey,
}

#[cw_serde]
pub struct QuoteResult {
    pub amount_in: TokenAmount,
    pub amount_out: TokenAmount,
    pub target_sqrt_price: SqrtPrice,
    pub ticks: Vec<Tick>,
}

#[cw_serde]
pub struct TokensResponse {
    /// Contains all token_ids in lexicographical ordering
    /// If there are more than `limit`, use `start_from` in future queries
    /// to achieve pagination.
    pub tokens: Vec<u64>,
}

#[cw_serde]
pub struct OwnerOfResponse {
    /// Owner of the token
    pub owner: Addr,
    /// If set this address is approved to transfer/send the token as well
    pub approvals: Vec<Approval>,
}

#[cw_serde]
pub struct ApprovedForAllResponse {
    pub operators: Vec<Approval>,
}

#[cw_serde]
pub struct AllNftInfoResponse {
    /// Who can transfer the token
    pub access: OwnerOfResponse,
    /// Data on the token itself,
    pub info: NftInfoResponse,
}

/// Cw721ReceiveMsg should be de/serialized under `Receive()` variant in a HandleMsg
#[cw_serde]
pub struct Cw721ReceiveMsg {
    pub sender: Addr,
    pub token_id: u64,
    pub msg: Option<Binary>,
}

impl Cw721ReceiveMsg {
    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg(self, contract_addr: String) -> StdResult<CosmosMsg> {
        let execute_msg =
            wasm_execute(contract_addr, &ReceiverHandleMsg::ReceiveNft(self), vec![])?;
        Ok(execute_msg.into())
    }
}

/// This is just a helper to properly serialize the above message.
/// The actual receiver should include this variant in the larger HandleMsg enum
#[cw_serde]
enum ReceiverHandleMsg {
    ReceiveNft(Cw721ReceiveMsg),
}

#[cw_serde]
pub struct NftInfoResponse {
    pub extension: Position,
}

#[cw_serde]
pub struct NumTokensResponse {
    pub count: u64,
}
