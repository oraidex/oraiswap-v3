use cosmwasm_std::{
    Addr, Api, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Storage, Timestamp, Uint64,
};

use cw20::Expiration;
use cw_storage_plus::Bound;
use decimal::{CheckedOps, Decimal};

use oraiswap_v3_common::{
    asset::{Asset, AssetInfo},
    error::ContractError,
    interface::{Approval, CalculateSwapResult, SwapHop},
    math::{
        check_tick, compute_swap_step,
        sqrt_price::{get_max_tick, get_min_tick, SqrtPrice},
        token_amount::TokenAmount,
        MAX_SQRT_PRICE, MIN_SQRT_PRICE,
    },
    storage::{PoolKey, Position, Tick, UpdatePoolTick},
};

use crate::state;

pub trait TimeStampExt {
    fn millis(&self) -> u64;
}

impl TimeStampExt for Timestamp {
    fn millis(&self) -> u64 {
        self.nanos() / 1_000_000
    }
}

pub fn create_tick(
    store: &mut dyn Storage,
    current_timestamp: u64,
    pool_key: &PoolKey,
    index: i32,
) -> Result<Tick, ContractError> {
    check_tick(index, pool_key.fee_tier.tick_spacing)?;
    let pool = state::get_pool(store, pool_key)?;

    let tick = Tick::create(index, &pool, current_timestamp);
    state::add_tick(store, pool_key, index, &tick)?;
    state::flip_bitmap(store, true, index, pool_key.fee_tier.tick_spacing, pool_key)?;

    Ok(tick)
}

pub fn calculate_swap(
    store: &dyn Storage,
    current_timestamp: u64,
    pool_key: &PoolKey,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> Result<CalculateSwapResult, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::AmountIsZero {});
    }

    let mut ticks: Vec<Tick> = vec![];
    let mut pool = state::get_pool(store, pool_key)?;

    if x_to_y {
        if pool.sqrt_price <= sqrt_price_limit || sqrt_price_limit > SqrtPrice::new(MAX_SQRT_PRICE)
        {
            return Err(ContractError::WrongLimit {});
        }
    } else if pool.sqrt_price >= sqrt_price_limit
        || sqrt_price_limit < SqrtPrice::new(MIN_SQRT_PRICE)
    {
        return Err(ContractError::WrongLimit {});
    }

    let tick_limit = if x_to_y {
        get_min_tick(pool_key.fee_tier.tick_spacing)
    } else {
        get_max_tick(pool_key.fee_tier.tick_spacing)
    };

    let mut remaining_amount = amount;

    let mut total_amount_in = TokenAmount::new(0);
    let mut total_amount_out = TokenAmount::new(0);

    let event_start_sqrt_price = pool.sqrt_price;
    let mut event_fee_amount = TokenAmount::new(0);

    while !remaining_amount.is_zero() {
        let (swap_limit, limiting_tick) = state::get_closer_limit(
            store,
            sqrt_price_limit,
            x_to_y,
            pool.current_tick_index,
            pool_key.fee_tier.tick_spacing,
            pool_key,
        )?;

        let result = compute_swap_step(
            pool.sqrt_price,
            swap_limit,
            pool.liquidity,
            remaining_amount,
            by_amount_in,
            pool_key.fee_tier.fee,
        )?;

        // make remaining amount smaller
        if by_amount_in {
            remaining_amount =
                remaining_amount.checked_sub(result.amount_in.checked_add(result.fee_amount)?)?;
        } else {
            remaining_amount = remaining_amount.checked_sub(result.amount_out)?;
        }

        pool.add_fee(
            result.fee_amount,
            x_to_y,
            state::CONFIG.load(store)?.protocol_fee,
        )?;
        event_fee_amount = event_fee_amount.checked_add(result.fee_amount)?;

        pool.sqrt_price = result.next_sqrt_price;

        total_amount_in = total_amount_in
            .checked_add(result.amount_in)?
            .checked_add(result.fee_amount)?;
        total_amount_out = total_amount_out.checked_add(result.amount_out)?;

        // Fail if price would go over swap limit
        if pool.sqrt_price == sqrt_price_limit && !remaining_amount.is_zero() {
            return Err(ContractError::PriceLimitReached {});
        }

        let mut tick_update = {
            if let Some((tick_index, is_initialized)) = limiting_tick {
                if is_initialized {
                    let tick = state::get_tick(store, pool_key, tick_index)?;
                    UpdatePoolTick::TickInitialized(tick)
                } else {
                    UpdatePoolTick::TickUninitialized(tick_index)
                }
            } else {
                UpdatePoolTick::NoTick
            }
        };

        let (amount_to_add, amount_after_tick_update, has_crossed) = pool.update_tick(
            result,
            swap_limit,
            &mut tick_update,
            remaining_amount,
            by_amount_in,
            x_to_y,
            current_timestamp,
            state::CONFIG.load(store)?.protocol_fee,
            pool_key.fee_tier,
        )?;

        remaining_amount = amount_after_tick_update;
        total_amount_in = total_amount_in.checked_add(amount_to_add)?;

        if let UpdatePoolTick::TickInitialized(tick) = tick_update {
            if has_crossed {
                ticks.push(tick)
            }
        }

        let reached_tick_limit = match x_to_y {
            true => pool.current_tick_index <= tick_limit,
            false => pool.current_tick_index >= tick_limit,
        };

        if reached_tick_limit {
            return Err(ContractError::TickLimitReached {});
        }
    }
    if total_amount_out.is_zero() {
        return Err(ContractError::NoGainSwap {});
    }

    Ok(CalculateSwapResult {
        amount_in: total_amount_in,
        amount_out: total_amount_out,
        start_sqrt_price: event_start_sqrt_price,
        target_sqrt_price: pool.sqrt_price,
        fee: event_fee_amount,
        pool,
        ticks,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn swap_internal(
    store: &mut dyn Storage,
    api: &dyn Api,
    info: &MessageInfo,
    msgs: &mut Vec<CosmosMsg>,
    contract_address: &Addr,
    current_timestamp: u64,
    pool_key: &PoolKey,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> Result<CalculateSwapResult, ContractError> {
    let calculate_swap_result = calculate_swap(
        store,
        current_timestamp,
        pool_key,
        x_to_y,
        amount,
        by_amount_in,
        sqrt_price_limit,
    )?;

    for tick in &calculate_swap_result.ticks {
        state::update_tick(store, pool_key, tick.index, tick)?;
    }

    state::POOLS.save(store, &pool_key.key(), &calculate_swap_result.pool)?;

    let (token_0, token_1) = if x_to_y {
        (&pool_key.token_x, &pool_key.token_y)
    } else {
        (&pool_key.token_y, &pool_key.token_x)
    };

    let asset_0 = Asset {
        info: AssetInfo::from_denom(api, token_0.as_str()),
        amount: calculate_swap_result.amount_in.into(),
    };

    let asset_1 = Asset {
        info: AssetInfo::from_denom(api, token_1.as_str()),
        amount: calculate_swap_result.amount_out.into(),
    };

    asset_0.transfer_from(msgs, info, contract_address.to_string())?;
    asset_1.transfer(msgs, info)?;

    Ok(calculate_swap_result)
}

pub fn swap_route_internal(
    store: &mut dyn Storage,
    api: &dyn Api,
    env: Env,
    info: &MessageInfo,
    msgs: &mut Vec<CosmosMsg>,
    amount_in: TokenAmount,
    swaps: Vec<SwapHop>,
) -> Result<TokenAmount, ContractError> {
    let mut next_swap_amount = amount_in;

    let current_timestamp = env.block.time.millis();

    for swap_hop in &swaps {
        let sqrt_price_limit = if swap_hop.x_to_y {
            SqrtPrice::new(MIN_SQRT_PRICE)
        } else {
            SqrtPrice::new(MAX_SQRT_PRICE)
        };

        next_swap_amount = swap_internal(
            store,
            api,
            info,
            msgs,
            &env.contract.address,
            current_timestamp,
            &swap_hop.pool_key,
            swap_hop.x_to_y,
            next_swap_amount,
            true,
            sqrt_price_limit,
        )?
        .amount_out;
    }

    Ok(next_swap_amount)
}

pub fn route(
    store: &dyn Storage,
    env: Env,
    amount_in: TokenAmount,
    swaps: Vec<SwapHop>,
) -> Result<TokenAmount, ContractError> {
    let mut next_swap_amount = amount_in;

    let current_timestamp = env.block.time.millis();

    for swap_hop in &swaps {
        let sqrt_price_limit = if swap_hop.x_to_y {
            SqrtPrice::new(MIN_SQRT_PRICE)
        } else {
            SqrtPrice::new(MAX_SQRT_PRICE)
        };

        next_swap_amount = calculate_swap(
            store,
            current_timestamp,
            &swap_hop.pool_key,
            swap_hop.x_to_y,
            next_swap_amount,
            true,
            sqrt_price_limit,
        )?
        .amount_out;
    }

    Ok(next_swap_amount)
}

pub fn tickmap_slice(
    store: &dyn Storage,
    min_chunk: u16,
    max_chunk: u16,
    pool_key: &PoolKey,
    limit: usize,
) -> Vec<(u16, Uint64)> {
    let pool_key = pool_key.key();
    let mut min_key = min_chunk.to_be_bytes().to_vec();
    let mut max_key = max_chunk.to_be_bytes().to_vec();
    min_key.extend_from_slice(&pool_key);
    max_key.extend_from_slice(&pool_key);
    let min = Some(Bound::InclusiveRaw(min_key));
    let max = Some(Bound::InclusiveRaw(max_key));
    let tickmap_slice = state::BITMAP
        .range_raw(store, min, max, Order::Ascending)
        .filter_map(|item| {
            if let Ok((k, v)) = item {
                if pool_key.eq(&k[2..]) {
                    return Some((u16::from_be_bytes([k[0], k[1]]), v.into()));
                }
            }
            None
        })
        .take(limit)
        .collect();

    tickmap_slice
}

pub fn remove_tick_and_flip_bitmap(
    storage: &mut dyn Storage,
    key: &PoolKey,
    tick: &Tick,
) -> Result<(), ContractError> {
    if !tick.liquidity_gross.is_zero() {
        return Err(ContractError::NotEmptyTickDeinitialization);
    }

    state::flip_bitmap(storage, false, tick.index, key.fee_tier.tick_spacing, key)?;

    state::remove_tick(storage, key, tick.index)?;

    Ok(())
}

/// returns true iff the sender can execute approve or reject on the contract
pub fn check_can_approve(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    owner_raw: &[u8],
) -> Result<(), ContractError> {
    // owner can approve
    let sender_raw = info.sender.as_bytes();
    if sender_raw.eq(owner_raw) {
        return Ok(());
    }

    // operator can approve
    let op = state::OPERATORS.may_load(deps.storage, (owner_raw, sender_raw))?;
    match op {
        Some(ex) => {
            if ex.is_expired(&env.block) {
                Err(ContractError::Unauthorized {})
            } else {
                Ok(())
            }
        }
        None => Err(ContractError::Unauthorized {}),
    }
}

/// returns true if the sender can transfer ownership of the token
pub fn check_can_send(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    owner_raw: &[u8],
    pos: &Position,
) -> Result<(), ContractError> {
    // owner can send
    let sender_raw = info.sender.as_bytes();

    if sender_raw.eq(owner_raw) {
        return Ok(());
    }

    // any non-expired token approval can send
    if pos
        .approvals
        .iter()
        .any(|apr| apr.spender == info.sender && !apr.expires.is_expired(&env.block))
    {
        return Ok(());
    }

    // operator can send
    let op = state::OPERATORS.may_load(deps.storage, (owner_raw, sender_raw))?;
    match op {
        Some(ex) => {
            if ex.is_expired(&env.block) {
                Err(ContractError::Unauthorized {})
            } else {
                Ok(())
            }
        }
        None => Err(ContractError::Unauthorized {}),
    }
}

pub fn update_approvals(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    spender: &Addr,
    token_id: u64,
    // if add == false, remove. if add == true, remove then set with this expiration
    add: bool,
    expires: Option<Expiration>,
) -> Result<Position, ContractError> {
    let (owner_raw, index) = state::POSITION_KEYS_BY_TOKEN_ID.load(deps.storage, token_id)?;
    let mut position_key = owner_raw.to_vec();
    position_key.extend_from_slice(&index.to_be_bytes());
    let mut pos = state::get_position_by_key(deps.storage, &position_key)?;
    // ensure we have permissions
    check_can_approve(deps.as_ref(), env, info, &owner_raw)?;

    // update the approval list (remove any for the same spender before adding)
    pos.approvals.retain(|apr| apr.spender != spender);

    // only difference between approve and revoke
    if add {
        // reject expired data as invalid
        let expires = expires.unwrap_or_default();
        if expires.is_expired(&env.block) {
            return Err(ContractError::Expired {});
        }
        let approval = Approval {
            spender: spender.clone(),
            expires,
        };
        pos.approvals.push(approval);
    }

    state::POSITIONS.save(deps.storage, &position_key, &pos)?;

    Ok(pos)
}

pub fn transfer_nft(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    recipient: &Addr,
    token_id: u64,
) -> Result<Position, ContractError> {
    let (owner_raw, index) = state::POSITION_KEYS_BY_TOKEN_ID.load(deps.storage, token_id)?;
    let account_id = Addr::unchecked(String::from_utf8(owner_raw.to_vec())?);
    let mut pos = state::get_position(deps.storage, &account_id, index)?;
    // ensure we have permissions
    check_can_send(deps.as_ref(), env, info, &owner_raw, &pos)?;
    // set owner and remove existing approvals
    state::remove_position(deps.storage, &account_id, index)?;
    // reset approvals when transfer
    pos.approvals = vec![];
    state::add_position(deps.storage, recipient, &pos)?;

    Ok(pos)
}
