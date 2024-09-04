use crate::fee_growth::FeeGrowth;
use crate::incentive::IncentiveRecord;
use crate::interface::{CalculateSwapResult, Cw721ReceiveMsg, SwapHop};
use crate::liquidity::Liquidity;
use crate::percentage::Percentage;
use crate::sqrt_price::SqrtPrice;
use crate::state::{self, CONFIG, POOLS};
use crate::token_amount::TokenAmount;
use crate::{calculate_min_amount_out, check_tick, FeeTier, Pool, PoolKey, Position};
use oraiswap_v3_common::asset::{Asset, AssetInfo};
use oraiswap_v3_common::error::ContractError;
use oraiswap_v3_common::incentives_fund_manager;

use super::{
    check_can_send, create_tick, remove_tick_and_flip_bitmap, swap_internal, swap_route_internal,
    transfer_nft, update_approvals, TimeStampExt,
};
use cosmwasm_std::{
    attr, wasm_execute, Addr, Attribute, Binary, DepsMut, Env, MessageInfo, Response,
};
use cw20::Expiration;
use decimal::Decimal;

/// Allows an admin to adjust admin.
///
/// # Parameters
/// - `new_admin`: new admin address.
///
/// # Errors
/// - Reverts the call when the caller is an unauthorized user.
///

pub fn change_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: Addr,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    config.admin = new_admin.clone();
    CONFIG.save(deps.storage, &config)?;

    let event_attributes = vec![
        attr("action", "change_admin"),
        attr("old_admin", info.sender.as_str()),
        attr("new_admin", new_admin.as_str()),
    ];

    Ok(Response::new().add_attributes(event_attributes))
}

/// Allows an fee receiver to withdraw collected fees.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
///
/// # Errors
/// - Reverts the call when the caller is an unauthorized receiver.
pub fn withdraw_protocol_fee(
    deps: DepsMut,
    info: MessageInfo,
    pool_key: PoolKey,
) -> Result<Response, ContractError> {
    let pool_key_db = pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;

    if pool.fee_receiver != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let (fee_protocol_token_x, fee_protocol_token_y) = pool.withdraw_protocol_fee();
    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    let asset_0 = Asset {
        info: AssetInfo::from_denom(deps.api, pool_key.token_x.as_str()),
        amount: fee_protocol_token_x.into(),
    };

    let asset_1 = Asset {
        info: AssetInfo::from_denom(deps.api, pool_key.token_y.as_str()),
        amount: fee_protocol_token_y.into(),
    };

    let mut msgs = vec![];
    asset_0.transfer(&mut msgs, &info)?;
    asset_1.transfer(&mut msgs, &info)?;

    let event_attributes = vec![
        attr("action", "withdraw_protocol_fee"),
        attr("pool_key", pool_key.to_string()),
        attr("token_x", fee_protocol_token_x.to_string()),
        attr("token_y", fee_protocol_token_y.to_string()),
        attr("receiver", pool.fee_receiver.as_str()),
    ];

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(event_attributes))
}

/// Allows an admin to adjust the protocol fee.
///
/// # Parameters
/// - `protocol_fee`: The expected fee represented as a percentage.
///
/// # Errors
/// - Reverts the call when the caller is an unauthorized user.
pub fn change_protocol_fee(
    deps: DepsMut,
    info: MessageInfo,
    protocol_fee: Percentage,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let old_protocol_fee = config.protocol_fee;

    config.protocol_fee = protocol_fee;
    CONFIG.save(deps.storage, &config)?;

    let event_attributes = vec![
        attr("action", "change_protocol_fee"),
        attr("admin", info.sender.as_str()),
        attr("old_protocol_fee", old_protocol_fee.get().to_string()),
        attr("new_protocol_fee", protocol_fee.get().to_string()),
    ];

    Ok(Response::new().add_attributes(event_attributes))
}

/// Allows admin to change current fee receiver.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `fee_receiver`: An `Addr` identifying the user authorized to claim fees.
///
/// # Errors
/// - Reverts the call when the caller is an unauthorized user.
pub fn change_fee_receiver(
    deps: DepsMut,
    info: MessageInfo,
    pool_key: PoolKey,
    fee_receiver: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let pool_key_db = pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;
    let old_fee_receiver = pool.fee_receiver.clone();
    pool.fee_receiver = fee_receiver.to_string();
    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    let event_attributes = vec![
        attr("action", "change_fee_receiver"),
        attr("pool_key", pool_key.to_string()),
        attr("admin", info.sender.as_str()),
        attr("old_fee_receiver", old_fee_receiver.as_str()),
        attr("new_fee_receiver", fee_receiver.as_str()),
    ];

    Ok(Response::new().add_attributes(event_attributes))
}

/// Opens a position.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `lower_tick`: The index of the lower tick for opening the position.
/// - `upper_tick`: The index of the upper tick for opening the position.
/// - `liquidity_delta`: The desired liquidity provided by the user in the specified range.
/// - `slippage_limit_lower`: The price limit for downward movement to execute the position creation.
/// - `slippage_limit_upper`: The price limit for upward movement to execute the position creation.
///
/// # Events
/// - On successful transfer, emits a `Create Position` event for the newly opened position.
///
/// # Errors
/// - Fails if the user attempts to open a position with zero liquidity.
/// - Fails if the user attempts to create a position with invalid tick indexes or tick spacing.
/// - Fails if the price has reached the slippage limit.
/// - Fails if the allowance is insufficient or the user balance transfer fails.
/// - Fails if pool does not exist
#[allow(clippy::too_many_arguments)]
pub fn create_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_key: PoolKey,
    lower_tick: i32,
    upper_tick: i32,
    liquidity_delta: Liquidity,
    slippage_limit_lower: SqrtPrice,
    slippage_limit_upper: SqrtPrice,
) -> Result<Response, ContractError> {
    let current_timestamp = env.block.time.millis();
    let current_block_number = env.block.height;

    // liquidity delta = 0 => return
    if liquidity_delta.is_zero() {
        return Err(ContractError::InsufficientLiquidity {});
    }

    if lower_tick == upper_tick {
        return Err(ContractError::InvalidTickIndex {});
    }
    let pool_key_db = pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;

    // update global incentives
    pool.update_global_incentives(env.block.time.seconds())?;

    let mut lower_tick = match state::get_tick(deps.storage, &pool_key, lower_tick) {
        Ok(tick) => tick,
        _ => create_tick(deps.storage, current_timestamp, &pool_key, lower_tick)?,
    };

    let mut upper_tick = match state::get_tick(deps.storage, &pool_key, upper_tick) {
        Ok(tick) => tick,
        _ => create_tick(deps.storage, current_timestamp, &pool_key, upper_tick)?,
    };

    let (mut position, x, y) = Position::create(
        &mut pool,
        pool_key.clone(),
        &mut lower_tick,
        &mut upper_tick,
        current_timestamp,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        current_block_number,
        pool_key.fee_tier.tick_spacing,
    )?;

    position.token_id = state::next_token_id(deps.storage)?;

    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    state::add_position(deps.storage, &info.sender, &position)?;

    state::update_tick(deps.storage, &pool_key, lower_tick.index, &lower_tick)?;
    state::update_tick(deps.storage, &pool_key, upper_tick.index, &upper_tick)?;

    let asset_0 = Asset {
        info: AssetInfo::from_denom(deps.api, pool_key.token_x.as_str()),
        amount: x.into(),
    };

    let asset_1 = Asset {
        info: AssetInfo::from_denom(deps.api, pool_key.token_y.as_str()),
        amount: y.into(),
    };

    let mut msgs = vec![];
    asset_0.transfer_from(&mut msgs, &info, env.contract.address.to_string())?;
    asset_1.transfer_from(&mut msgs, &info, env.contract.address.to_string())?;

    let event_attributes = vec![
        attr("action", "create_position"),
        attr("pool_key", pool_key.to_string()),
        attr("token_id", position.token_id.to_string()),
        attr("owner", info.sender.as_str()),
        attr("position_liquidity", liquidity_delta.get().to_string()),
        attr("lower_tick", lower_tick.index.to_string()),
        attr("upper_tick", upper_tick.index.to_string()),
        attr("current_sqrt_price", pool.sqrt_price.get().to_string()),
        attr("liquidity_x", x.to_string()),
        attr("liquidity_y", y.to_string()),
        attr("after_liquidity ", pool.liquidity.get().to_string()),
        attr("ater_tick_index", pool.current_tick_index.to_string()),
    ];

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(event_attributes))
}

/// Performs a single swap based on the provided parameters.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `x_to_y`: A boolean specifying the swap direction.
/// - `amount`: TokenAmount that the user wants to swap.
/// - `by_amount_in`: A boolean specifying whether the user provides the amount to swap or expects the amount out.
/// - `sqrt_price_limit`: A square root of price limit allowing the price to move for the swap to occur.
///
/// # Events
/// - On a successful swap, emits a `Swap` event for the freshly made swap.
/// - On a successful swap, emits a `Cross Tick` event for every single tick crossed.
///
/// # Errors
/// - Fails if the user attempts to perform a swap with zero amounts.
/// - Fails if the price has reached the specified price limit (or price associated with specified square root of price).
/// - Fails if the user would receive zero tokens.
/// - Fails if the allowance is insufficient or the user balance transfer fails.
/// - Fails if there is insufficient liquidity in pool
/// - Fails if pool does not
#[allow(clippy::too_many_arguments)]
pub fn swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_key: PoolKey,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> Result<Response, ContractError> {
    // update incentives first
    let mut pool = state::get_pool(deps.storage, &pool_key)?;
    pool.update_global_incentives(env.block.time.seconds())?;
    POOLS.save(deps.storage, &pool_key.key(), &pool)?;

    let mut msgs = vec![];

    let CalculateSwapResult {
        amount_in,
        amount_out,
        fee,
        pool: after_pool,
        ..
    } = swap_internal(
        deps.storage,
        deps.api,
        &info,
        &mut msgs,
        &env.contract.address,
        env.block.time.millis(),
        &pool_key,
        x_to_y,
        amount,
        by_amount_in,
        sqrt_price_limit,
    )?;

    let event_attributes = vec![
        attr("action", "swap"),
        attr("pool_key", pool_key.to_string()),
        attr("sender", info.sender.as_str()),
        attr("amount_in", amount_in.to_string()),
        attr("amount_out", amount_out.to_string()),
        attr("current_tick", after_pool.current_tick_index.to_string()),
        attr(
            "current_sqrt_price",
            after_pool.sqrt_price.get().to_string(),
        ),
        attr("liquidity", after_pool.liquidity.get().to_string()),
        attr("x_to_y", x_to_y.to_string()),
        attr("fee", fee.to_string()),
    ];

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(event_attributes))
}

/// Performs atomic swap involving several pools based on the provided parameters.
///
/// # Parameters
/// - `amount_in`: The amount of tokens that the user wants to swap.
/// - `expected_amount_out`: The amount of tokens that the user wants to receive as a result of the swaps.
/// - `slippage`: The max acceptable percentage difference between the expected and actual amount of output tokens in a trade, not considering square root of target price as in the case of a swap.
/// - `swaps`: A vector containing all parameters needed to identify separate swap steps.
///
/// # Events
/// - On every successful swap, emits a `Swap` event for the freshly made swap.
/// - On every successful swap, emits a `Cross Tick` event for every single tick crossed.
///
/// # Errors
/// - Fails if the user attempts to perform a swap with zero amounts.
/// - Fails if the user would receive zero tokens.
/// - Fails if the allowance is insufficient or the user balance transfer fails.
/// - Fails if the minimum amount out after a single swap is insufficient to perform the next swap to achieve the expected amount out.
/// - Fails if pool does not exist
///
/// # External contracts
pub fn swap_route(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount_in: TokenAmount,
    expected_amount_out: TokenAmount,
    slippage: Percentage,
    swaps: Vec<SwapHop>,
) -> Result<Response, ContractError> {
    // update incentives first
    for hop in &swaps {
        let mut pool = state::get_pool(deps.storage, &hop.pool_key)?;
        pool.update_global_incentives(env.block.time.seconds())?;
        POOLS.save(deps.storage, &hop.pool_key.key(), &pool)?;
    }

    let mut msgs = vec![];
    let amount_out = swap_route_internal(
        deps.storage,
        deps.api,
        env,
        &info,
        &mut msgs,
        amount_in,
        swaps.clone(),
    )?;

    let min_amount_out = calculate_min_amount_out(expected_amount_out, slippage);

    if amount_out < min_amount_out {
        return Err(ContractError::AmountUnderMinimumAmountOut);
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "swap_route")
        .add_attribute("amount_out", amount_out.to_string())
        .add_attribute("swap_hop", format!("{:?}", swaps)))
}

/// Transfers a position between users.
///
/// # Parameters
/// - `index`: The index of the user position to transfer.
/// - `receiver`: An `AccountId` identifying the user who will own the position.
pub fn transfer_position(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    index: u32,
    receiver: String,
) -> Result<Response, ContractError> {
    let mut position = state::get_position(deps.storage, &info.sender, index)?;

    state::remove_position(deps.storage, &info.sender, index)?;

    let receiver_addr = deps.api.addr_validate(&receiver)?;
    // reset approvals
    position.approvals = vec![];
    state::add_position(deps.storage, &receiver_addr, &position)?;

    let event_attributes = vec![
        attr("action", "transfer_position"),
        attr("sender", info.sender.as_str()),
        attr("receiver", receiver),
        attr("position_token_id", position.token_id.to_string()),
    ];

    Ok(Response::new().add_attributes(event_attributes))
}

/// Allows an authorized user (owner of the position) to claim collected fees.
///
/// # Parameters
/// - `index`: The index of the user position from which fees will be claimed.
///
/// # Errors
/// - Fails if the position cannot be found.
pub fn claim_fee(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    index: u32,
) -> Result<Response, ContractError> {
    let current_timestamp = env.block.time.millis();

    let mut position = state::get_position(deps.storage, &info.sender, index)?;

    let mut lower_tick =
        state::get_tick(deps.storage, &position.pool_key, position.lower_tick_index)?;
    let mut upper_tick =
        state::get_tick(deps.storage, &position.pool_key, position.upper_tick_index)?;
    let pool_key_db = position.pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;

    let (x, y) = position.claim_fee(
        &mut pool,
        &mut upper_tick,
        &mut lower_tick,
        current_timestamp,
    )?;

    state::update_position(deps.storage, &position)?;
    POOLS.save(deps.storage, &pool_key_db, &pool)?;
    state::update_tick(
        deps.storage,
        &position.pool_key,
        upper_tick.index,
        &upper_tick,
    )?;
    state::update_tick(
        deps.storage,
        &position.pool_key,
        lower_tick.index,
        &lower_tick,
    )?;

    let asset_0 = Asset {
        info: AssetInfo::from_denom(deps.api, position.pool_key.token_x.as_str()),
        amount: x.into(),
    };

    let asset_1 = Asset {
        info: AssetInfo::from_denom(deps.api, position.pool_key.token_y.as_str()),
        amount: y.into(),
    };

    let mut msgs = vec![];
    asset_0.transfer(&mut msgs, &info)?;
    asset_1.transfer(&mut msgs, &info)?;

    let event_attributes = vec![
        attr("action", "claim_fee"),
        attr("owner", info.sender.as_str()),
        attr("pool_key", position.pool_key.to_string()),
        attr("position_token_id", position.token_id.to_string()),
        attr("amount_x", x.to_string()),
        attr("amount_y", y.to_string()),
    ];

    let resp: Response = claim_incentives(deps, env, info, index)?;

    Ok(resp.add_messages(msgs).add_attributes(event_attributes))
}

/// Allows an authorized user (owner of the position) to claim incentives.
///
/// # Parameters
/// - `index`: The index of the user position from which fees will be claimed.
///
/// # Errors
/// - Fails if the position cannot be found.
pub fn claim_incentives(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    index: u32,
) -> Result<Response, ContractError> {
    let mut position = state::get_position(deps.storage, &info.sender, index)?;
    let config = CONFIG.load(deps.storage)?;

    let lower_tick = state::get_tick(deps.storage, &position.pool_key, position.lower_tick_index)?;
    let upper_tick = state::get_tick(deps.storage, &position.pool_key, position.upper_tick_index)?;
    let pool_key_db = position.pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;

    // update global incentive
    pool.update_global_incentives(env.block.time.seconds())?;

    let incentives = position
        .claim_incentives(&pool, &upper_tick, &lower_tick)
        .unwrap_or(vec![]);

    state::update_position(deps.storage, &position)?;
    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    let mut msgs = vec![];
    for asset in incentives.clone() {
        msgs.push(wasm_execute(
            config.incentives_fund_manager.clone(),
            &incentives_fund_manager::ExecuteMsg::SendFund {
                asset,
                receiver: info.sender.clone(),
            },
            vec![],
        )?);
        // asset.transfer(&mut msgs, &info)?;
    }

    let mut event_attributes: Vec<Attribute> = vec![];

    if !incentives.is_empty() {
        event_attributes.append(&mut vec![
            attr(
                "incentives_token_address",
                incentives
                    .iter()
                    .map(|x| x.info.denom())
                    .collect::<Vec<String>>()
                    .join(","),
            ),
            attr(
                "incentives_amount",
                incentives
                    .iter()
                    .map(|x| x.amount.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            ),
        ]);
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(event_attributes))
}

/// Removes a position. Sends tokens associated with specified position to the owner.
///
/// # Parameters
/// - `index`: The index of the user position to be removed.
///
/// # Events
/// - Emits a `Remove Position` event upon success.
///
/// # Errors
/// - Fails if Position cannot be found
pub fn remove_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    index: u32,
) -> Result<Response, ContractError> {
    let current_timestamp = env.block.time.millis();
    let mut position = state::get_position(deps.storage, &info.sender, index)?;
    let withdrawed_liquidity = position.liquidity;

    let mut lower_tick =
        state::get_tick(deps.storage, &position.pool_key, position.lower_tick_index)?;
    let mut upper_tick =
        state::get_tick(deps.storage, &position.pool_key, position.upper_tick_index)?;

    let pool_key_db = position.pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;

    // update global incentives first
    pool.update_global_incentives(env.block.time.seconds())?;

    // calculate pending incentives
    let incentives = position.claim_incentives(&pool, &upper_tick, &lower_tick)?;

    let mut event_attributes: Vec<Attribute> = vec![attr("action", "remove_position")];

    if !incentives.is_empty() {
        event_attributes.append(&mut vec![
            // attr("_contract_address", env.contract.address.to_string()),
            attr(
                "incentives_token_address",
                incentives
                    .iter()
                    .map(|x| x.info.denom())
                    .collect::<Vec<String>>()
                    .join(","),
            ),
            attr(
                "incentives_amount",
                incentives
                    .iter()
                    .map(|x| x.amount.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            ),
        ]);
    }

    let (
        amount_x,
        amount_y,
        liquidity_x,
        liquidity_y,
        fee_x,
        fee_y,
        after_liquidity,
        ater_tick_index,
        deinitialize_lower_tick,
        deinitialize_upper_tick,
    ) = position.remove(
        &mut pool,
        current_timestamp,
        &mut lower_tick,
        &mut upper_tick,
        position.pool_key.fee_tier.tick_spacing,
    )?;

    event_attributes.append(&mut vec![
        attr("fee_x", fee_x.to_string()),
        attr("fee_y", fee_y.to_string()),
    ]);

    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    if deinitialize_lower_tick {
        remove_tick_and_flip_bitmap(deps.storage, &position.pool_key, &lower_tick)?;
    } else {
        state::update_tick(
            deps.storage,
            &position.pool_key,
            position.lower_tick_index,
            &lower_tick,
        )?;
    }

    if deinitialize_upper_tick {
        remove_tick_and_flip_bitmap(deps.storage, &position.pool_key, &upper_tick)?;
    } else {
        state::update_tick(
            deps.storage,
            &position.pool_key,
            position.upper_tick_index,
            &upper_tick,
        )?;
    }

    let position = state::remove_position(deps.storage, &info.sender, index)?;

    let asset_0 = Asset {
        info: AssetInfo::from_denom(deps.api, position.pool_key.token_x.as_str()),
        amount: amount_x.into(),
    };

    let asset_1 = Asset {
        info: AssetInfo::from_denom(deps.api, position.pool_key.token_y.as_str()),
        amount: amount_y.into(),
    };

    let mut msgs = vec![];
    asset_0.transfer(&mut msgs, &info)?;
    asset_1.transfer(&mut msgs, &info)?;
    // claim incentives
    for asset in incentives.clone() {
        let config = CONFIG.load(deps.storage)?;
        msgs.push(
            wasm_execute(
                config.incentives_fund_manager.clone(),
                &incentives_fund_manager::ExecuteMsg::SendFund {
                    asset,
                    receiver: info.sender.clone(),
                },
                vec![],
            )?
            .into(),
        );
        // asset.transfer(&mut msgs, &info)?;
    }

    event_attributes.append(&mut vec![
        attr("pool_key", position.pool_key.to_string()),
        attr("token_id", position.token_id.to_string()),
        attr("owner", info.sender.as_str()),
        attr("position_liquidity", withdrawed_liquidity.get().to_string()),
        attr("lower_tick", lower_tick.index.to_string()),
        attr("upper_tick", upper_tick.index.to_string()),
        attr("current_sqrt_price", pool.sqrt_price.get().to_string()),
        attr("liquidity_x", liquidity_x.to_string()),
        attr("liquidity_y", liquidity_y.to_string()),
        attr("after_liquidity ", after_liquidity.get().to_string()),
        attr("after_tick_index", ater_tick_index.to_string()),
    ]);

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(event_attributes))
}

/// Allows a user to create a custom pool on a specified token pair and fee tier.
/// The contract specifies the order of tokens as x and y, the lower token address assigned as token x.
/// The choice is deterministic.
///
/// # Parameters
/// - `token_0`: The address of the first token.
/// - `token_1`: The address of the second token.
/// - `fee_tier`: A struct identifying the pool fee and tick spacing.
/// - `init_sqrt_price`: The square root of the price for the initial pool related to `init_tick`.
/// - `init_tick`: The initial tick at which the pool will be created.
///
/// # Errors
/// - Fails if the specified fee tier cannot be found.
/// - Fails if the user attempts to create a pool for the same tokens.
/// - Fails if Pool with same tokens and fee tier already exist.
/// - Fails if the init tick is not divisible by the tick spacing.
/// - Fails if the init sqrt price is not related to the init tick.
#[allow(clippy::too_many_arguments)]
pub fn create_pool(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    token_0: String,
    token_1: String,
    fee_tier: FeeTier,
    init_sqrt_price: SqrtPrice,
    init_tick: i32,
) -> Result<Response, ContractError> {
    let current_timestamp = env.block.time.millis();

    let config = CONFIG.load(deps.storage)?;

    if !config.fee_tiers.contains(&fee_tier) {
        return Err(ContractError::FeeTierNotFound);
    }

    check_tick(init_tick, fee_tier.tick_spacing)?;

    let pool_key = PoolKey::new(token_0, token_1, fee_tier)?;
    let db_key = pool_key.key();

    if POOLS.has(deps.storage, &db_key) {
        return Err(ContractError::PoolAlreadyExist);
    }

    let config = CONFIG.load(deps.storage)?;

    let pool = Pool::create(
        init_sqrt_price,
        init_tick,
        current_timestamp,
        fee_tier.tick_spacing,
        config.admin,
    )?;

    POOLS.save(deps.storage, &db_key, &pool)?;

    let event_attributes = vec![
        attr("action", "create_pool"),
        attr("pool_creator", info.sender.as_str()),
        attr("block", env.block.height.to_string()),
        attr("pool_key", pool_key.to_string()),
        attr("init_sqrt_price", init_sqrt_price.get().to_string()),
        attr("init_tick", init_tick.to_string()),
    ];

    Ok(Response::new().add_attributes(event_attributes))
}

/// Allows admin to add a custom fee tier.
///
/// # Parameters
/// - `fee_tier`: A struct identifying the pool fee and tick spacing.
///
/// # Errors
/// - Fails if an unauthorized user attempts to create a fee tier.
/// - Fails if the tick spacing is invalid.
/// - Fails if the fee tier already exists.
/// - Fails if fee is invalid
pub fn add_fee_tier(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    fee_tier: FeeTier,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if fee_tier.tick_spacing == 0 || fee_tier.tick_spacing > 100 {
        return Err(ContractError::InvalidTickSpacing);
    }

    if fee_tier.fee >= Percentage::new(1000000000000) {
        // 100% -> fee invalid
        return Err(ContractError::InvalidFee);
    }

    config.fee_tiers.push(fee_tier);

    CONFIG.save(deps.storage, &config)?;

    let event_attributes = vec![
        attr("action", "add_fee_tier"),
        attr("admin", info.sender.as_str()),
        attr("fee", fee_tier.fee.get().to_string()),
        attr("tick_spacing", fee_tier.tick_spacing.to_string()),
    ];

    Ok(Response::new().add_attributes(event_attributes))
}

/// Removes an existing fee tier.
///
/// # Parameters
/// - `fee_tier`: A struct identifying the pool fee and tick spacing.
///
/// # Errors
/// - Fails if an unauthorized user attempts to remove a fee tier.
/// - Fails if fee tier does not exist
pub fn remove_fee_tier(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    fee_tier: FeeTier,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(pos) = config.fee_tiers.iter().position(|x| *x == fee_tier) {
        config.fee_tiers.remove(pos);
    } else {
        return Err(ContractError::FeeTierNotFound);
    }

    CONFIG.save(deps.storage, &config)?;

    let event_attributes = vec![
        attr("action", "remove_fee_tier"),
        attr("admin", info.sender.as_str()),
        attr("fee", fee_tier.fee.get().to_string()),
        attr("tick_spacing", fee_tier.tick_spacing.to_string()),
    ];

    Ok(Response::new().add_attributes(event_attributes))
}

pub fn handle_approve(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    spender: Addr,
    token_id: u64,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    update_approvals(deps, &env, &info, &spender, token_id, true, expires)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "approve"),
        attr("token_id", token_id.to_string()),
        attr("sender", info.sender),
        attr("spender", spender),
    ]))
}

pub fn handle_revoke(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    spender: Addr,
    token_id: u64,
) -> Result<Response, ContractError> {
    update_approvals(deps, &env, &info, &spender, token_id, false, None)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "revoke"),
        attr("token_id", token_id.to_string()),
        attr("sender", info.sender),
        attr("spender", spender),
    ]))
}

pub fn handle_approve_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operator: Addr,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    // reject expired data as invalid
    let expires = expires.unwrap_or_default();
    if expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // set the operator for us
    let sender_raw = info.sender.as_bytes();
    let operator_raw = operator.as_bytes();
    state::OPERATORS.save(deps.storage, (sender_raw, operator_raw), &expires)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "approve_all"),
        attr("sender", info.sender),
        attr("operator", operator),
    ]))
}

pub fn handle_revoke_all(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    operator: Addr,
) -> Result<Response, ContractError> {
    let sender_raw = info.sender.as_bytes();
    let operator_raw = operator.as_bytes();
    state::OPERATORS.remove(deps.storage, (sender_raw, operator_raw));

    Ok(Response::new().add_attributes(vec![
        attr("action", "revoke_all"),
        attr("sender", info.sender),
        attr("operator", operator),
    ]))
}

/// this is trigger when there is buy_nft action
pub fn handle_transfer_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Addr,
    token_id: u64,
) -> Result<Response, ContractError> {
    transfer_nft(deps, &env, &info, &recipient, token_id)?;

    // need transfer_payout as well

    Ok(Response::new().add_attributes(vec![
        attr("action", "transfer_nft"),
        attr("token_id", token_id.to_string()),
        attr("sender", info.sender),
        attr("recipient", recipient),
    ]))
}

pub fn handle_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: u64,
) -> Result<Response, ContractError> {
    let (owner_raw, index) = state::POSITION_KEYS_BY_TOKEN_ID.load(deps.storage, token_id)?;
    let mut position_key = owner_raw.to_vec();
    position_key.extend_from_slice(&index.to_be_bytes());
    let pos = state::get_position_by_key(deps.storage, &position_key)?;

    check_can_send(deps.as_ref(), &env, &info, &owner_raw, &pos)?;

    // remain action to help sync backend
    remove_position(deps, env, info.clone(), index)
}

pub fn handle_send_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: Addr,
    token_id: u64,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    // Transfer token
    transfer_nft(deps, &env, &info, &contract, token_id)?;

    let send = Cw721ReceiveMsg {
        sender: info.sender.clone(),
        token_id,
        msg,
    };

    // Send message
    Ok(Response::new()
        .add_message(send.into_cosmos_msg(contract.to_string())?)
        .add_attributes(vec![
            attr("action", "send_nft"),
            attr("token_id", token_id.to_string()),
            attr("sender", info.sender),
            attr("recipient", contract),
        ]))
}

#[allow(clippy::too_many_arguments)]
pub fn handle_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_key: PoolKey,
    lower_tick: i32,
    upper_tick: i32,
    liquidity_delta: Liquidity,
    slippage_limit_lower: SqrtPrice,
    slippage_limit_upper: SqrtPrice,
) -> Result<Response, ContractError> {
    // remain action to help sync backend
    create_position(
        deps,
        env,
        info,
        pool_key,
        lower_tick,
        upper_tick,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
    )
}

// only owner can execute
#[allow(clippy::too_many_arguments)]
pub fn create_incentive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_key: PoolKey,
    reward_token: AssetInfo,
    total_reward: Option<TokenAmount>,
    reward_per_sec: TokenAmount,
    start_timestamp: Option<u64>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let pool_key_db = pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;
    pool.update_global_incentives(env.block.time.seconds())?;

    let id = pool.incentives.len() as u64;
    let remaining = total_reward.unwrap_or(TokenAmount(u128::MAX));
    let incentive = IncentiveRecord {
        id,
        reward_per_sec,
        reward_token: reward_token.clone(),
        remaining,
        start_timestamp: start_timestamp.unwrap_or(env.block.time.seconds()),
        incentive_growth_global: FeeGrowth(0),
        last_updated: env.block.time.seconds(),
    };
    pool.incentives.push(incentive);

    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "create_incentive"),
        ("pool", &pool_key.to_string()),
        ("record_id", &id.to_string()),
        ("reward_token", &reward_token.denom()),
        ("total_reward", &remaining.to_string()),
        ("reward_per_sec", &reward_per_sec.to_string()),
        (
            "start_timestamp",
            &start_timestamp
                .unwrap_or(env.block.time.seconds())
                .to_string(),
        ),
    ]))
}

// only owner can execute
#[allow(clippy::too_many_arguments)]
pub fn update_incentive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_key: PoolKey,
    record_id: u64,
    remaining_reward: Option<TokenAmount>,
    start_timestamp: Option<u64>,
    reward_per_sec: Option<TokenAmount>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let pool_key_db = pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;
    pool.update_global_incentives(env.block.time.seconds())?;

    if let Some(record) = pool.incentives.iter_mut().find(|i| i.id == record_id) {
        if let Some(remaining_reward) = remaining_reward {
            record.remaining = remaining_reward;
        }
        if let Some(start_timestamp) = start_timestamp {
            record.start_timestamp = start_timestamp;
        }
        if let Some(reward_per_sec) = reward_per_sec {
            record.reward_per_sec = reward_per_sec;
        }
    }

    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "update_incentive"),
        ("pool", &pool_key.to_string()),
        ("record_id", &record_id.to_string()),
        (
            "remaining_reward",
            &remaining_reward.unwrap_or_default().to_string(),
        ),
        (
            "start_timestamp",
            &start_timestamp.unwrap_or_default().to_string(),
        ),
        (
            "reward_per_sec",
            &reward_per_sec.unwrap_or_default().to_string(),
        ),
    ]))
}
