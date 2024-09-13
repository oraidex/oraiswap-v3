use std::vec;

use cosmwasm_std::{
    to_json_binary, Coin, CosmosMsg, Decimal, DepsMut, Env, Order, Response, StdResult, SubMsg,
    WasmMsg,
};

use oraiswap_v3_common::{
    asset::{Asset, AssetInfo},
    error::ContractError,
    logic::{get_liquidity_by_x, get_liquidity_by_y, SingleTokenLiquidity},
    math::{
        sqrt_price::{get_max_sqrt_price, get_min_sqrt_price},
        token_amount::TokenAmount,
    },
    oraiswap_v3_msg::{ExecuteMsg as V3ExecuteMsg, QueryMsg as V3QueryMsg},
    storage::Pool,
};

use crate::{
    contract::ADD_LIQUIDITY_REPLY_ID,
    state::{
        CONFIG, PENDING_POSITION, PROTOCOL_FEE, RECEIVER, SNAP_BALANCE, SNAP_BALANCES,
        ZAP_OUT_ROUTES,
    },
    ProtocolFee,
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
        &V3QueryMsg::Pool {
            token_0: token_x.info.denom(),
            token_1: token_y.info.denom(),
            fee_tier: pending_position.pool_key.fee_tier,
        },
    )?;

    let mut res: SingleTokenLiquidity;

    let is_in_range = pending_position.lower_tick <= pool_info.current_tick_index
        && pending_position.upper_tick > pool_info.current_tick_index;
    if is_in_range {
        res = get_liquidity_by_x(
            TokenAmount(x_amount.u128()),
            pending_position.lower_tick,
            pending_position.upper_tick,
            pool_info.sqrt_price,
            false,
        )?;
        if res.amount > TokenAmount(y_amount.u128()) {
            res = get_liquidity_by_y(
                TokenAmount(y_amount.u128()),
                pending_position.lower_tick,
                pending_position.upper_tick,
                pool_info.sqrt_price,
                false,
            )?;
        }
    } else if pending_position.lower_tick > pool_info.current_tick_index {
        res = get_liquidity_by_x(
            TokenAmount(x_amount.u128()),
            pending_position.lower_tick,
            pending_position.upper_tick,
            pool_info.sqrt_price,
            false,
        )?;
    } else {
        res = get_liquidity_by_y(
            TokenAmount(y_amount.u128()),
            pending_position.lower_tick,
            pending_position.upper_tick,
            pool_info.sqrt_price,
            false,
        )?;
    }

    // validate minimum liquidity
    if let Some(min_liquidity) = pending_position.minimum_liquidity {
        if res.l.lt(&min_liquidity) {
            return Err(ContractError::ZapInAssertionFailure {
                minium_receive: min_liquidity,
                return_amount: res.l,
            });
        }
    }

    // approve tokenX and tokenY to dex_v3
    let mut coins: Vec<Coin> = vec![];
    token_x
        .info
        .increase_allowance(&mut coins, &mut msgs, config.dex_v3.to_string(), x_amount)?;
    token_y
        .info
        .increase_allowance(&mut coins, &mut msgs, config.dex_v3.to_string(), y_amount)?;

    sub_msgs.push(SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: config.dex_v3.to_string(),
            msg: to_json_binary(&V3ExecuteMsg::CreatePosition {
                pool_key: pending_position.pool_key.clone(),
                lower_tick: pending_position.lower_tick,
                upper_tick: pending_position.upper_tick,
                liquidity_delta: res.l,
                slippage_limit_lower: pending_position.slippage_limit_lower.unwrap_or(
                    get_min_sqrt_price(pending_position.pool_key.fee_tier.tick_spacing),
                ),
                slippage_limit_upper: pending_position.slippage_limit_upper.unwrap_or(
                    get_max_sqrt_price(pending_position.pool_key.fee_tier.tick_spacing),
                ),
            })?,
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
        msg: to_json_binary(&V3ExecuteMsg::TransferPosition {
            index: pending_position.index,
            receiver: receiver.to_string(),
        })?,
        funds: vec![],
    }));

    // 10. Refund unused tokenX and tokenY to user
    if !x_amount.is_zero() {
        token_x
            .info
            .transfer(&mut msgs, receiver.to_string(), x_amount)?;
    }
    if !y_amount.is_zero() {
        token_y
            .info
            .transfer(&mut msgs, receiver.to_string(), y_amount)?;
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

    // no need to use hashMap because the number of tokens is very small
    let mut all_balances: Vec<Asset> = SNAP_BALANCES
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, amount) = item?;
            let token_info = AssetInfo::from_denom(deps.api, &denom);
            let current_balance =
                token_info.balance(&deps.querier, env.contract.address.to_string())?;

            Ok(Asset {
                info: token_info,
                amount: current_balance.checked_sub(amount)?,
            })
        })
        .collect::<StdResult<Vec<Asset>>>()?;

    let protocol_fee = PROTOCOL_FEE.may_load(deps.storage)?.unwrap_or(ProtocolFee {
        percent: Decimal::zero(),
        fee_receiver: receiver.clone(),
    });
    // try swaps
    for route in zap_out_routes {
        let token_info = AssetInfo::from_denom(deps.api, &route.token_in);
        if let Some(balance) = all_balances.iter_mut().find(|b| b.info.eq(&token_info)) {
            if balance.amount < route.offer_amount {
                return Err(ContractError::ZapOutNotEnoughBalanceToSwap {});
            }
            balance.amount -= route.offer_amount;

            let mut amount_to_swap = route.offer_amount;
            if !protocol_fee.percent.is_zero() {
                let fee_amount = amount_to_swap * protocol_fee.percent;
                amount_to_swap -= fee_amount;

                // transfer fee to fee_receiver
                token_info.transfer(
                    &mut msgs,
                    protocol_fee.fee_receiver.to_string(),
                    fee_amount,
                )?;
            }

            let swap_msg = build_swap_msg(
                &token_info,
                config.mixed_router.clone(),
                amount_to_swap,
                route.operations,
                route.minimum_receive,
                Some(receiver.clone()),
                None,
            )?;
            msgs.push(swap_msg.into());
        }
    }

    // refund remaining asset to user
    for balance in all_balances.iter().filter(|b| !b.amount.is_zero()) {
        balance
            .info
            .transfer(&mut msgs, receiver.to_string(), balance.amount)?;
    }

    // remove pending states
    SNAP_BALANCES.clear(deps.storage);
    RECEIVER.remove(deps.storage);
    ZAP_OUT_ROUTES.remove(deps.storage);

    Ok(Response::new().add_messages(msgs))
}
