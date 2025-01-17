use cosmwasm_std::{Addr, Binary, Deps, Env, Order, StdResult, Uint64};
use cw_storage_plus::Bound;
use oraiswap_v3_common::{
    asset::Asset,
    error::ContractError,
    interface::{
        AllNftInfoResponse, Approval, ApprovedForAllResponse, NftInfoResponse, NumTokensResponse,
        OwnerOfResponse, PoolWithPoolKey, PositionTick, QuoteResult, SwapHop, TokensResponse,
    },
    math::{
        percentage::Percentage,
        sqrt_price::{get_max_tick, get_min_tick, SqrtPrice},
        token_amount::TokenAmount,
    },
    storage::{
        get_max_chunk, get_min_chunk, tick_to_position, FeeTier, LiquidityTick, Pool, PoolKey,
        Position, Tick, CHUNK_SIZE, LIQUIDITY_TICK_LIMIT, MAX_TICKMAP_QUERY_SIZE,
        POSITION_TICK_LIMIT,
    },
};

use crate::state::{self, CONFIG, MAX_LIMIT, POSITIONS};

use super::{calculate_swap, route, tickmap_slice, TimeStampExt};

/// Retrieves the admin of contract.
pub fn query_admin(deps: Deps) -> Result<Addr, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.admin)
}

/// Retrieves the protocol fee represented as a percentage.
pub fn get_protocol_fee(deps: Deps) -> Result<Percentage, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.protocol_fee)
}

/// Retrieves the incentives_fund_manager contract address.
pub fn get_incentives_fund_manager(deps: Deps) -> Result<Addr, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.incentives_fund_manager)
}

/// Retrieves information about a single position.
///
/// # Parameters
/// - `owner_id`: An `Addr` identifying the user who owns the position.
/// - `index`: The index of the user position.
///
/// # Errors
/// - Fails if position cannot be found    
pub fn get_position(deps: Deps, owner_id: Addr, index: u32) -> Result<Position, ContractError> {
    state::get_position(deps.storage, &owner_id, index)
}

// /// Retrieves a vector containing all positions held by the user.
// ///
// /// # Parameters
// /// - `owner_id`: An `Addr` identifying the user who owns the positions.
pub fn get_positions(
    deps: Deps,
    owner_id: Addr,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Position>, ContractError> {
    state::get_all_positions(deps.storage, &owner_id, limit, offset)
}

/// Query of whether the fee tier exists.
///
/// # Parameters
/// - `fee_tier`: A struct identifying the pool fee and tick spacing.
pub fn fee_tier_exist(deps: Deps, fee_tier: FeeTier) -> Result<bool, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.fee_tiers.contains(&fee_tier))
}

/// Retrieves information about a pool created on a specified token pair with an associated fee tier.
///
/// # Parameters
/// - `token_0`: The address of the first token.
/// - `token_1`: The address of the second token.
/// - `fee_tier`: A struct identifying the pool fee and tick spacing.
///
/// # Errors
/// - Fails if there is no pool associated with created key

pub fn get_pool(
    deps: Deps,
    token_0: String,
    token_1: String,
    fee_tier: FeeTier,
) -> Result<Pool, ContractError> {
    let pool_key = &PoolKey::new(token_0, token_1, fee_tier)?;
    state::get_pool(deps.storage, pool_key)
}

/// Retrieves information about a tick at a specified index.
///
/// # Parameters
/// - `key`: A unique key that identifies the specified pool.
/// - `index`: The tick index in the tickmap.
///
/// # Errors
/// - Fails if tick cannot be found    
pub fn get_tick(deps: Deps, key: PoolKey, index: i32) -> Result<Tick, ContractError> {
    state::get_tick(deps.storage, &key, index)
}

/// Checks if the tick at a specified index is initialized.
///
/// # Parameters
/// - `key`: A unique key that identifies the specified pool.
/// - `index`: The tick index in the tickmap.
pub fn is_tick_initialized(deps: Deps, key: PoolKey, index: i32) -> Result<bool, ContractError> {
    Ok(state::get_bitmap(
        deps.storage,
        index,
        key.fee_tier.tick_spacing,
        &key,
    ))
}

/// Retrieves listed pools
/// - `size`: Amount of pool keys to retrive
/// - `offset`: The offset from which retrive pools.
pub fn get_pools(
    deps: Deps,
    limit: Option<u32>,
    start_after: Option<PoolKey>,
) -> Result<Vec<PoolWithPoolKey>, ContractError> {
    state::get_pools(deps.storage, limit, start_after)
}

pub fn get_pools_with_pool_keys(
    deps: Deps,
    pool_keys: Vec<PoolKey>,
) -> Result<Vec<PoolWithPoolKey>, ContractError> {
    let mut pools = vec![];
    for pool_key in pool_keys {
        if let Ok(pool) = state::get_pool(deps.storage, &pool_key) {
            pools.push(PoolWithPoolKey { pool, pool_key });
        }
    }
    Ok(pools)
}

/// Retrieves listed pools for provided token pair
/// - `token_0`: Address of first token
/// - `token_1`: Address of second token
pub fn get_all_pools_for_pair(
    deps: Deps,
    token_0: String,
    token_1: String,
) -> Result<Vec<PoolWithPoolKey>, ContractError> {
    let fee_tiers = get_fee_tiers(deps)?;
    let mut pool_key = PoolKey::new(token_0, token_1, FeeTier::default())?;
    let mut pools = vec![];
    for fee_tier in fee_tiers {
        pool_key.fee_tier = fee_tier;
        if let Ok(pool) = state::get_pool(deps.storage, &pool_key) {
            pools.push(PoolWithPoolKey {
                pool,
                pool_key: pool_key.clone(),
            });
        }
    }
    Ok(pools)
}

/// Retrieves available fee tiers
pub fn get_fee_tiers(deps: Deps) -> Result<Vec<FeeTier>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.fee_tiers)
}

/// Retrieves list of lower and upper ticks of user positions.
///
/// # Parameters
/// - `owner`: An `Addr` identifying the user who owns the position.
/// - `offset`: The offset from the current position index.
pub fn get_position_ticks(
    deps: Deps,
    owner: Addr,
    offset: u32,
) -> Result<Vec<PositionTick>, ContractError> {
    let positions_length = state::get_position_length(deps.storage, &owner);
    let mut ticks = vec![];

    if offset > positions_length {
        return Err(ContractError::InvalidOffset);
    }

    for i in offset..positions_length {
        if let Ok(position) = state::get_position(deps.storage, &owner, i) {
            if let Ok(tick) =
                state::get_tick(deps.storage, &position.pool_key, position.lower_tick_index)
            {
                ticks.push(PositionTick {
                    index: tick.index,
                    fee_growth_outside_x: tick.fee_growth_outside_x,
                    fee_growth_outside_y: tick.fee_growth_outside_y,
                    seconds_outside: tick.seconds_outside,
                });
            }

            if let Ok(tick) =
                state::get_tick(deps.storage, &position.pool_key, position.upper_tick_index)
            {
                ticks.push(PositionTick {
                    index: tick.index,
                    fee_growth_outside_x: tick.fee_growth_outside_x,
                    fee_growth_outside_y: tick.fee_growth_outside_y,
                    seconds_outside: tick.seconds_outside,
                });
            }
        }

        if ticks.len() >= POSITION_TICK_LIMIT {
            break;
        }
    }

    Ok(ticks)
}

/// Retrieves the amount of positions held by the user.
///
/// # Parameters
/// - `owner`: An `Addr` identifying the user who owns the position.
pub fn get_user_position_amount(deps: Deps, owner: Addr) -> Result<u32, ContractError> {
    Ok(state::get_position_length(deps.storage, &owner))
}

/// Retrieves tickmap chunks
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `lower_tick_index`: offset tick index.
/// - `upper_tick_index`: limiting tick index.
/// - `x_to_y`: direction of the query.
pub fn get_tickmap(
    deps: Deps,
    pool_key: PoolKey,
    lower_tick_index: i32,
    upper_tick_index: i32,
    x_to_y: bool,
) -> Result<Vec<(u16, Uint64)>, ContractError> {
    let tick_spacing = pool_key.fee_tier.tick_spacing;
    let (start_chunk, _) = tick_to_position(lower_tick_index, tick_spacing);
    let (end_chunk, _) = tick_to_position(upper_tick_index, tick_spacing);

    let min_chunk_index = get_min_chunk(tick_spacing).max(start_chunk);
    let max_chunk_index = get_max_chunk(tick_spacing).min(end_chunk);

    let mut tickmaps = tickmap_slice(
        deps.storage,
        min_chunk_index,
        max_chunk_index,
        &pool_key,
        MAX_TICKMAP_QUERY_SIZE,
    );

    if x_to_y {
        tickmaps.reverse();
    };

    Ok(tickmaps)
}

/// Retrieves ticks of a specified pool.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `tick_indexes`: Indexes of the tick to be retrieved.
///
/// # Errors
/// - Fails if tick_indexes are too large
/// - Fails if tick is not found
///
pub fn get_liquidity_ticks(
    deps: Deps,
    pool_key: PoolKey,
    tick_indexes: Vec<i32>,
) -> Result<Vec<LiquidityTick>, ContractError> {
    let mut liqudity_ticks: Vec<LiquidityTick> = vec![];

    if tick_indexes.len() > LIQUIDITY_TICK_LIMIT {
        return Err(ContractError::TickLimitReached);
    }

    for index in tick_indexes {
        let tick = LiquidityTick::from(state::get_tick(deps.storage, &pool_key, index)?);

        liqudity_ticks.push(tick);
    }

    Ok(liqudity_ticks)
}

/// Retrieves the amount of liquidity ticks of a specified pool.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool. For poolkeys with tick_spacing equal to 1 the query has to be split into 2 smaller queries
/// - `lower_tick`: index to start counting from(inclusive)
/// - `upper_tick`: index to stop counting after(inclusive)
///
/// # Errors
/// - Fails if lower_tick or upper_tick are invalid
/// - Fails if tick_spacing is invalid
pub fn get_liquidity_ticks_amount(
    deps: Deps,
    pool_key: PoolKey,
    lower_tick: i32,
    upper_tick: i32,
) -> Result<u32, ContractError> {
    let tick_spacing = pool_key.fee_tier.tick_spacing;
    if tick_spacing == 0 {
        return Err(ContractError::InvalidTickSpacing);
    };

    if lower_tick % (tick_spacing as i32) != 0 || upper_tick % (tick_spacing as i32) != 0 {
        return Err(ContractError::InvalidTickIndex);
    }

    let max_tick = get_max_tick(tick_spacing);
    let min_tick = get_min_tick(tick_spacing);

    if lower_tick < min_tick || upper_tick > max_tick {
        return Err(ContractError::InvalidTickIndex);
    };

    let (min_chunk_index, min_bit) = tick_to_position(lower_tick, tick_spacing);
    let (max_chunk_index, max_bit) = tick_to_position(upper_tick, tick_spacing);

    let active_bits_in_range = |chunk, min_bit, max_bit| {
        let range: u64 = (chunk >> min_bit) & ((1u64 << (max_bit - min_bit + 1)) - 1);
        range.count_ones()
    };

    let min_chunk = state::get_bitmap_item(deps.storage, min_chunk_index, &pool_key).unwrap_or(0);

    if max_chunk_index == min_chunk_index {
        return Ok(active_bits_in_range(min_chunk, min_bit, max_bit));
    }

    let max_chunk = state::get_bitmap_item(deps.storage, max_chunk_index, &pool_key).unwrap_or(0);

    let mut amount: u32 = 0;
    amount = amount
        .checked_add(active_bits_in_range(
            min_chunk,
            min_bit,
            (CHUNK_SIZE - 1) as u8,
        ))
        .ok_or(ContractError::Add)?;
    amount = amount
        .checked_add(active_bits_in_range(max_chunk, 0, max_bit))
        .ok_or(ContractError::Add)?;

    for i in (min_chunk_index + 1)..max_chunk_index {
        let chunk = state::get_bitmap_item(deps.storage, i, &pool_key).unwrap_or(0);

        amount = amount
            .checked_add(chunk.count_ones())
            .ok_or(ContractError::Add)?;
    }

    Ok(amount)
}

/// Simulates the swap without its execution.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `x_to_y`: A boolean specifying the swap direction.
/// - `amount`: The amount of tokens that the user wants to swap.
/// - `by_amount_in`: A boolean specifying whether the user provides the amount to swap or expects the amount out.
/// - `sqrt_price_limit`: A square root of price limit allowing the price to move for the swap to occur.
///
/// # Errors
/// - Fails if the user attempts to perform a swap with zero amounts.
/// - Fails if the price has reached the specified limit.
/// - Fails if the user would receive zero tokens.
/// - Fails if pool does not exist
pub fn quote(
    deps: Deps,
    env: Env,
    pool_key: PoolKey,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> Result<QuoteResult, ContractError> {
    let calculate_swap_result = calculate_swap(
        deps.storage,
        env.block.time.millis(),
        &pool_key,
        x_to_y,
        amount,
        by_amount_in,
        sqrt_price_limit,
    )?;

    Ok(QuoteResult {
        amount_in: calculate_swap_result.amount_in,
        amount_out: calculate_swap_result.amount_out,
        target_sqrt_price: calculate_swap_result.pool.sqrt_price,
        ticks: calculate_swap_result.ticks,
    })
}

/// Simulates multiple swaps without its execution.
///
/// # Parameters
/// - `amount_in`: The amount of tokens that the user wants to swap.
/// - `swaps`: A vector containing all parameters needed to identify separate swap steps.
///
/// # Errors
/// - Fails if the user attempts to perform a swap with zero amounts.
/// - Fails if the user would receive zero tokens.
/// - Fails if pool does not exist
pub fn quote_route(
    deps: Deps,
    env: Env,
    amount_in: TokenAmount,
    swaps: Vec<SwapHop>,
) -> Result<TokenAmount, ContractError> {
    let amount_out = route(deps.storage, env, amount_in, swaps)?;
    Ok(amount_out)
}

pub fn query_owner_of(
    deps: Deps,
    env: Env,
    token_id: u64,
    include_expired: bool,
) -> Result<OwnerOfResponse, ContractError> {
    let (owner_raw, index) = state::POSITION_KEYS_BY_TOKEN_ID.load(deps.storage, token_id)?;
    let owner = Addr::unchecked(String::from_utf8(owner_raw.to_vec())?);
    let mut pos = state::get_position(deps.storage, &owner, index)?;
    pos.approvals
        .retain(|apr| include_expired || !apr.expires.is_expired(&env.block));
    Ok(OwnerOfResponse {
        owner,
        approvals: pos.approvals,
    })
}

pub fn query_all_approvals(
    deps: Deps,
    env: Env,
    owner: Addr,
    include_expired: bool,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> Result<ApprovedForAllResponse, ContractError> {
    let limit = limit.unwrap_or(MAX_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|x| x.as_bytes().to_vec())
        .map(Bound::ExclusiveRaw);

    let owner_raw = owner.as_bytes();
    let res: StdResult<Vec<_>> = state::OPERATORS
        .prefix(owner_raw)
        .range_raw(deps.storage, start, None, Order::Ascending)
        .filter(|r| include_expired || r.is_err() || !r.as_ref().unwrap().1.is_expired(&env.block))
        .take(limit)
        .map(|item| {
            let (spender_raw, expires) = item?;
            let spender = Addr::unchecked(String::from_utf8(spender_raw)?);
            Ok(Approval { spender, expires })
        })
        .collect();
    Ok(ApprovedForAllResponse { operators: res? })
}

pub fn query_nft_info(deps: Deps, token_id: u64) -> Result<NftInfoResponse, ContractError> {
    let (owner_raw, index) = state::POSITION_KEYS_BY_TOKEN_ID.load(deps.storage, token_id)?;
    let mut position_key = owner_raw.to_vec();
    position_key.extend_from_slice(&index.to_be_bytes());
    let pos = state::get_position_by_key(deps.storage, &position_key)?;
    Ok(NftInfoResponse { extension: pos })
}

pub fn query_all_nft_info(
    deps: Deps,
    env: Env,
    token_id: u64,
    include_expired: bool,
) -> Result<AllNftInfoResponse, ContractError> {
    let (owner_raw, index) = state::POSITION_KEYS_BY_TOKEN_ID.load(deps.storage, token_id)?;
    let owner = Addr::unchecked(String::from_utf8(owner_raw.to_vec())?);
    let mut pos = state::get_position(deps.storage, &owner, index)?;
    pos.approvals
        .retain(|apr| include_expired || !apr.expires.is_expired(&env.block));
    Ok(AllNftInfoResponse {
        access: OwnerOfResponse {
            owner,
            approvals: pos.approvals.clone(),
        },
        info: NftInfoResponse { extension: pos },
    })
}

pub fn query_tokens(
    deps: Deps,
    owner: Addr,
    start_after: Option<u32>,
    limit: Option<u32>,
) -> Result<TokensResponse, ContractError> {
    let tokens = state::get_all_position_keys(deps.storage, &owner, limit, start_after)
        .into_iter()
        .map(|key| {
            let pos = state::get_position_by_key(deps.storage, &key)?;
            Ok(pos.token_id)
        })
        .collect::<StdResult<_>>()?;

    Ok(TokensResponse { tokens })
}

pub fn query_all_tokens(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> Result<TokensResponse, ContractError> {
    let limit = limit.unwrap_or(MAX_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|x| x.to_be_bytes().to_vec())
        .map(Bound::ExclusiveRaw);

    let tokens = state::POSITION_KEYS_BY_TOKEN_ID
        .keys_raw(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|key| u64::from_be_bytes(key.try_into().unwrap()))
        .collect();

    Ok(TokensResponse { tokens })
}

pub fn query_num_tokens(deps: Deps) -> Result<NumTokensResponse, ContractError> {
    let count = state::num_tokens(deps.storage)?;
    Ok(NumTokensResponse { count })
}

/// Retrieves incentives information of a single position.
///
/// # Parameters
/// - `owner_id`: An `Addr` identifying the user who owns the position.
/// - `index`: The index of the user position.
///
/// # Errors
/// - Fails if position cannot be found    
pub fn query_position_incentives(
    deps: Deps,
    env: Env,
    owner_id: Addr,
    index: u32,
) -> Result<Vec<Asset>, ContractError> {
    let mut position = state::get_position(deps.storage, &owner_id, index)?;
    let mut pool = state::get_pool(deps.storage, &position.pool_key)?;
    let lower_tick = state::get_tick(deps.storage, &position.pool_key, position.lower_tick_index)?;
    let upper_tick = state::get_tick(deps.storage, &position.pool_key, position.upper_tick_index)?;
    // update global incentive
    pool.update_global_incentives(env.block.time.seconds())?;
    position.update_incentives(&pool, &upper_tick, &lower_tick)?;

    let incentives = position.claim_incentives(&pool, &upper_tick, &lower_tick)?;

    Ok(incentives)
}

pub fn query_all_positions(
    deps: Deps,
    limit: Option<u32>,
    start_after: Option<Binary>,
) -> Result<Vec<Position>, ContractError> {
    let limit = limit.unwrap_or(MAX_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|x| x.to_vec()).map(Bound::ExclusiveRaw);
    Ok(POSITIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .filter_map(Result::ok)
        .map(|(_, position)| position)
        .collect::<Vec<Position>>())
}
