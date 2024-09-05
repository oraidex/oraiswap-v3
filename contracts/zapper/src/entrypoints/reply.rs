use std::vec;

use cosmwasm_std::{
    to_json_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, Response, SubMsg, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use oraiswap_v3_common::{
    asset::{Asset, AssetInfo},
    logic::{get_liquidity_by_x, get_liquidity_by_y},
    math::{sqrt_price::get_min_sqrt_price, token_amount::TokenAmount},
    oraiswap_v3_msg::{ExecuteMsg, QueryMsg},
    storage::{Pool, Position},
};

use crate::{
    contract::ADD_LIQUIDITY_REPLY_ID,
    state::{CONFIG, PENDING_POSITION, RECEIVER, SNAP_BALANCE},
    ContractError, PairBalance,
};

pub fn zap_in_liquidity(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut sub_msgs: Vec<SubMsg> = vec![];

    // 5. Recheck the balance of tokenX and tokenY in this contract
    let snap_balance = SNAP_BALANCE.load(deps.storage)?;
    let token_x = snap_balance.tokenX;
    let token_y = snap_balance.tokenY;

    // 6. Minus with the previous balance of tokenX and tokenY snap in state
    let x_amount_after = token_x.info.balance(&deps.querier, &env.contract.address)?;
    let y_amount_after = token_y.info.balance(&deps.querier, &env.contract.address)?;
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
    match token_x.info.clone() {
        AssetInfo::NativeToken { denom } => {
            coins.push(Coin {
                denom,
                amount: x_amount.into(),
            });
        }
        AssetInfo::Token { contract_addr } => {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: config.dex_v3.to_string(),
                    amount: x_amount.into(),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            }));
        }
    }

    match token_y.info.clone() {
        AssetInfo::NativeToken { denom } => {
            coins.push(Coin {
                denom,
                amount: y_amount.into(),
            });
        }
        AssetInfo::Token { contract_addr } => {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: config.dex_v3.to_string(),
                    amount: y_amount.into(),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            }));
        }
    }

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

    SNAP_BALANCE.save(
        deps.storage,
        &PairBalance {
            tokenX: Asset {
                info: token_x.info,
                amount: x_amount_after - x_amount,
            },
            tokenY: Asset {
                info: token_y.info,
                amount: y_amount_after - y_amount,
            },
        },
    )?;

    Ok(Response::new().add_messages(msgs).add_submessages(sub_msgs))
}

pub fn add_liquidity(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut msgs: Vec<CosmosMsg> = vec![];

    // 8. Refund unused tokenX and tokenY to user
    let snap_balance = SNAP_BALANCE.load(deps.storage)?;
    let token_x = snap_balance.tokenX;
    let token_y = snap_balance.tokenY;

    let x_amount_after = token_x.info.balance(&deps.querier, &env.contract.address)?;
    let y_amount_after = token_y.info.balance(&deps.querier, &env.contract.address)?;
    // amount to refund
    let x_amount = x_amount_after - token_x.amount;
    let y_amount = y_amount_after - token_y.amount;

    // 9. Transfer position to user
    let config = CONFIG.load(deps.storage)?;
    let pending_position = PENDING_POSITION.load(deps.storage)?;
    let receiver = RECEIVER.load(deps.storage)?;
    let _position_info: Position = deps.querier.query_wasm_smart(
        config.dex_v3.to_string(),
        &QueryMsg::Position {
            index: pending_position.index,
            owner_id: env.contract.address,
        },
    )?;
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
    if x_amount > 0u128.into() {
        match token_x.info.clone() {
            AssetInfo::NativeToken { denom } => {
                msgs.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: receiver.to_string(),
                    amount: vec![Coin {
                        denom,
                        amount: x_amount.into(),
                    }],
                }));
            }
            AssetInfo::Token { contract_addr } => {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: receiver.to_string(),
                        amount: x_amount.into(),
                    })
                    .unwrap(),
                    funds: vec![],
                }));
            }
        }
    }

    if y_amount > 0u128.into() {
        match token_y.info.clone() {
            AssetInfo::NativeToken { denom } => {
                msgs.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: receiver.to_string(),
                    amount: vec![Coin {
                        denom,
                        amount: y_amount.into(),
                    }],
                }));
            }
            AssetInfo::Token { contract_addr } => {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: receiver.to_string(),
                        amount: y_amount.into(),
                    })
                    .unwrap(),
                    funds: vec![],
                }));
            }
        }
    }

    Ok(Response::new().add_messages(msgs))
}

pub fn zap_out_liquidity(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    // 4. Recheck the balance of tokenX and tokenY in this contract

    // 5. Minus with the previous balance of tokenX and tokenY snap in state

    // 6. Send the amounts of tokenX and tokenY to user

    Ok(Response::new())
}
