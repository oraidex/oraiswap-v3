use std::vec;

use cosmwasm_std::{to_json_binary, Coin, CosmosMsg, DepsMut, Env, Response, SubMsg, WasmMsg};
use oraiswap_v3_common::{
    error::ContractError,
    logic::{get_liquidity_by_x, get_liquidity_by_y},
    math::{sqrt_price::get_min_sqrt_price, token_amount::TokenAmount},
    oraiswap_v3_msg::{ExecuteMsg, QueryMsg},
    storage::Pool,
};

use crate::{
    contract::ADD_LIQUIDITY_REPLY_ID,
    state::{CONFIG, PENDING_POSITION, RECEIVER, SNAP_BALANCE, SNAP_INCENTIVE, ZAP_OUT_ROUTES},
};

use super::build_swap_msg;

pub fn zap_in_liquidity(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut sub_msgs: Vec<SubMsg> = vec![];

    // 5. Recheck the balance of tokenX and tokenY in this contract
    let snap_balance = SNAP_BALANCE.load(deps.storage)?;
    let token_x = snap_balance.token_x;
    let token_y = snap_balance.token_y;

    // 6. Minus with the previous balance of tokenX and tokenY snap in state
    let x_amount_after = token_x
        .info
        .balance(&deps.querier, env.contract.address.to_string())?;
    let y_amount_after = token_y
        .info
        .balance(&deps.querier, env.contract.address.to_string())?;
    let x_amount = x_amount_after - token_x.amount;
    let y_amount = y_amount_after - token_y.amount;

    // 7. Process create new position with amountX and amountY
    let config = CONFIG.load(deps.storage)?;
    let pending_position = PENDING_POSITION.load(deps.storage)?;
    let pool_info: Pool = deps.querier.query_wasm_smart(
        config.dex_v3.to_string(),
        &QueryMsg::Pool {
            token_0: token_x.info.denom(),
            token_1: token_y.info.denom(),
            fee_tier: pending_position.pool_key.fee_tier,
        },
    )?;
    let mut res = get_liquidity_by_x(
        TokenAmount(x_amount.u128()),
        pending_position.lower_tick,
        pending_position.upper_tick,
        pool_info.sqrt_price,
        false,
    )
    .unwrap();

    if res.amount > TokenAmount(y_amount.u128()) {
        res = get_liquidity_by_y(
            TokenAmount(y_amount.u128()),
            pending_position.lower_tick,
            pending_position.upper_tick,
            pool_info.sqrt_price,
            false,
        )
        .unwrap();
    }

    // approve tokenX and tokenY to dex_v3
    let mut coins: Vec<Coin> = vec![];
    token_x
        .info
        .increase_allowance(&mut coins, &mut msgs, config.dex_v3.to_string(), x_amount)
        .unwrap();
    token_y
        .info
        .increase_allowance(&mut coins, &mut msgs, config.dex_v3.to_string(), y_amount)
        .unwrap();

    sub_msgs.push(SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: config.dex_v3.to_string(),
            msg: to_json_binary(&ExecuteMsg::CreatePosition {
                pool_key: pending_position.pool_key.clone(),
                lower_tick: pending_position.lower_tick,
                upper_tick: pending_position.upper_tick,
                liquidity_delta: res.l,
                slippage_limit_lower: pending_position.slippage_limit_lower.unwrap_or(
                    get_min_sqrt_price(pending_position.pool_key.fee_tier.tick_spacing),
                ),
                slippage_limit_upper: pending_position.slippage_limit_upper.unwrap_or(
                    get_min_sqrt_price(pending_position.pool_key.fee_tier.tick_spacing),
                ),
            })
            .unwrap(),
            funds: coins,
        },
        ADD_LIQUIDITY_REPLY_ID,
    ));

    Ok(Response::new().add_messages(msgs).add_submessages(sub_msgs))
}

pub fn add_liquidity(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut msgs: Vec<CosmosMsg> = vec![];

    // 8. Refund unused tokenX and tokenY to user
    let snap_balance = SNAP_BALANCE.load(deps.storage)?;
    let token_x = snap_balance.token_x;
    let token_y = snap_balance.token_y;

    let x_amount_after = token_x
        .info
        .balance(&deps.querier, env.contract.address.to_string())?;
    let y_amount_after = token_y
        .info
        .balance(&deps.querier, env.contract.address.to_string())?;
    // amount to refund
    let x_amount = x_amount_after - token_x.amount;
    let y_amount = y_amount_after - token_y.amount;

    // 9. Transfer position to user
    let config = CONFIG.load(deps.storage)?;
    let pending_position = PENDING_POSITION.load(deps.storage)?;
    let receiver = RECEIVER.load(deps.storage)?;
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.dex_v3.to_string(),
        msg: to_json_binary(&ExecuteMsg::TransferPosition {
            index: pending_position.index,
            receiver: receiver.to_string(),
        })
        .unwrap(),
        funds: vec![],
    }));

    // 10. Refund unused tokenX and tokenY to user
    if !x_amount.is_zero() {
        token_x
            .info
            .transfer(&mut msgs, receiver.to_string(), x_amount)
            .unwrap();
    }
    if !y_amount.is_zero() {
        token_y
            .info
            .transfer(&mut msgs, receiver.to_string(), y_amount)
            .unwrap();
    }

    // remove pending position
    PENDING_POSITION.remove(deps.storage);
    SNAP_BALANCE.remove(deps.storage);
    RECEIVER.remove(deps.storage);

    Ok(Response::new().add_messages(msgs))
}

pub fn zap_out_liquidity(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut msgs: Vec<CosmosMsg> = vec![];
    let receiver = RECEIVER.load(deps.storage)?;
    let zap_out_routes = ZAP_OUT_ROUTES.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    // 4. Recheck the balance of tokenX and tokenY in this contract
    let snap_balance = SNAP_BALANCE.load(deps.storage)?;
    let token_x = snap_balance.token_x;
    let token_y = snap_balance.token_y;

    let x_amount_after = token_x
        .info
        .balance(&deps.querier, env.contract.address.to_string())?;
    let y_amount_after = token_y
        .info
        .balance(&deps.querier, env.contract.address.to_string())?;
    // amount to refund
    let x_amount = x_amount_after - token_x.amount;
    let y_amount = y_amount_after - token_y.amount;

    let incentive_balance = SNAP_INCENTIVE.load(deps.storage)?;
    for incentive in incentive_balance.incentives.iter() {
        let after_balance = incentive
            .info
            .balance(&deps.querier, env.contract.address.to_string())?;
        let amount = after_balance - incentive.amount;
        if !amount.is_zero() {
            incentive
                .info
                .transfer(&mut msgs, receiver.to_string(), amount)
                .unwrap();
        }
    }

    if !x_amount.is_zero() {
        if let Some(operation_from_x) = zap_out_routes.operation_from_x {
            let swap_msg = build_swap_msg(
                &token_x.info,
                config.mixed_router.clone(),
                x_amount,
                operation_from_x,
                zap_out_routes.minimum_receive_x,
                Some(receiver.clone()),
                None,
            )?;
            msgs.push(CosmosMsg::Wasm(swap_msg));
        } else {
            // transfer to receiver
            token_x
                .info
                .transfer(&mut msgs, receiver.to_string(), x_amount)?;
        }
    }

    if !y_amount.is_zero() {
        if let Some(operation_from_y) = zap_out_routes.operation_from_y {
            let swap_msg = build_swap_msg(
                &token_y.info,
                config.mixed_router.clone(),
                y_amount,
                operation_from_y,
                zap_out_routes.minimum_receive_y,
                Some(receiver.clone()),
                None,
            )?;
            msgs.push(CosmosMsg::Wasm(swap_msg));
        } else {
            // transfer to receiver
            token_y
                .info
                .transfer(&mut msgs, receiver.to_string(), y_amount)?;
        }
    }

    Ok(Response::new().add_messages(msgs))
}
