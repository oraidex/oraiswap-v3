use cosmwasm_std::{
    coins, to_json_binary, Addr, Api, CosmosMsg, MessageInfo, QuerierWrapper, SubMsg, Uint128,
    WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use oraiswap_v3_common::{
    asset::{Asset, AssetInfo},
    storage::PoolKey,
};

use crate::{
    contract::ZAP_IN_LIQUIDITY_REPLY_ID,
    msgs::{mixed_router, Affiliate, SwapOperation},
    ContractError,
};

pub fn get_pool_v3_asset_info(api: &dyn Api, pool_key: &PoolKey) -> (AssetInfo, AssetInfo) {
    (
        AssetInfo::from_denom(api, &pool_key.token_x),
        AssetInfo::from_denom(api, &pool_key.token_y),
    )
}

pub fn validate_fund(
    querier: &QuerierWrapper,
    info: &MessageInfo,
    address: String,
    asset: Asset,
    msgs: &mut Vec<CosmosMsg>,
) -> Result<(), ContractError> {
    match asset.info.clone() {
        AssetInfo::Token { contract_addr: _ } => {
            asset.transfer_from(msgs, info, address).unwrap();
        }
        AssetInfo::NativeToken { denom: _ } => {
            let balance = asset.info.balance(querier, address).unwrap();
            if balance < asset.amount {
                return Err(ContractError::NoFundSent {});
            }
        }
    }
    Ok(())
}

pub fn process_single_swap_operation(
    // sub_msgs: &mut Vec<SubMsg>,
    asset: AssetInfo,
    contract: String,
    amount: Uint128,
    operations: Vec<SwapOperation>,
    minimum_receive: Option<Uint128>,
    to: Option<Addr>,
    affiliates: Option<Vec<Affiliate>>,
) -> Result<WasmMsg, ContractError> {
    match asset {
        AssetInfo::Token { contract_addr } => {
            let swap_msg = Cw20ExecuteMsg::Send {
                contract,
                amount,
                msg: to_json_binary(&mixed_router::ExecuteMsg::ExecuteSwapOperations {
                    operations,
                    minimum_receive,
                    to,
                    affiliates,
                })?,
            };
            let swap_msg = WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_json_binary(&swap_msg)?,
                funds: vec![],
            };
            Ok(swap_msg)
            // sub_msgs.push(SubMsg::reply_on_success(
            //     swap_msg,
            //     ZAP_IN_LIQUIDITY_REPLY_ID,
            // ))
        }
        AssetInfo::NativeToken { denom } => {
            let swap_msg = mixed_router::ExecuteMsg::ExecuteSwapOperations {
                operations,
                minimum_receive,
                to,
                affiliates,
            };
            let swap_msg = WasmMsg::Execute {
                contract_addr: contract,
                msg: to_json_binary(&swap_msg)?,
                funds: coins(amount.u128(), &denom),
            };
            Ok(swap_msg)
            // sub_msgs.push(SubMsg::reply_on_success(
            //     swap_msg,
            //     ZAP_IN_LIQUIDITY_REPLY_ID,
            // ))
        }
    }
}

pub fn process_double_swap_operation(
    msgs: &mut Vec<CosmosMsg>,
    sub_msgs: &mut Vec<SubMsg>,
    asset: AssetInfo,
    contract: String,
    amount_to_x: Uint128,
    amount_to_y: Uint128,
    operations_to_x: Vec<SwapOperation>,
    operations_to_y: Vec<SwapOperation>,
    minimum_receive_x: Option<Uint128>,
    minimum_receive_y: Option<Uint128>,
    to: Option<Addr>,
    affiliates: Option<Vec<Affiliate>>,
) -> Result<(), ContractError> {
    match asset {
        AssetInfo::Token { contract_addr } => {
            // swap operation 1
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: contract.clone(),
                    amount: amount_to_x,
                    msg: to_json_binary(&mixed_router::ExecuteMsg::ExecuteSwapOperations {
                        operations: operations_to_x,
                        minimum_receive: minimum_receive_x,
                        to: to.clone(),
                        affiliates: affiliates.clone(),
                    })?,
                })
                .unwrap(),
                funds: vec![],
            }));
            // swap operation 2 is subMsg
            let swap_msg = Cw20ExecuteMsg::Send {
                contract,
                amount: amount_to_y,
                msg: to_json_binary(&mixed_router::ExecuteMsg::ExecuteSwapOperations {
                    operations: operations_to_y,
                    minimum_receive: minimum_receive_y,
                    to,
                    affiliates,
                })?,
            };
            let swap_msg = WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_json_binary(&swap_msg)?,
                funds: vec![],
            };
            sub_msgs.push(SubMsg::reply_on_success(
                swap_msg,
                ZAP_IN_LIQUIDITY_REPLY_ID,
            ))
        }
        AssetInfo::NativeToken { denom } => {
            // swap operation 1
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract.clone(),
                msg: to_json_binary(&mixed_router::ExecuteMsg::ExecuteSwapOperations {
                    operations: operations_to_x,
                    minimum_receive: minimum_receive_x,
                    to: to.clone(),
                    affiliates: affiliates.clone(),
                })
                .unwrap(),
                funds: coins(amount_to_x.u128(), &denom),
            }));
            // swap operation 2 is subMsg
            let swap_msg = mixed_router::ExecuteMsg::ExecuteSwapOperations {
                operations: operations_to_y,
                minimum_receive: minimum_receive_y,
                to,
                affiliates,
            };
            let swap_msg = WasmMsg::Execute {
                contract_addr: contract,
                msg: to_json_binary(&swap_msg)?,
                funds: coins(amount_to_y.u128(), &denom),
            };
            sub_msgs.push(SubMsg::reply_on_success(
                swap_msg,
                ZAP_IN_LIQUIDITY_REPLY_ID,
            ))
        }
    }
    Ok(())
}
