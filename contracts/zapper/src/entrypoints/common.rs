use cosmwasm_std::{
    coins, to_json_binary, wasm_execute, Addr, Api, CosmosMsg, MessageInfo, QuerierWrapper,
    Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use oraiswap_v3_common::{
    asset::{Asset, AssetInfo},
    error::ContractError,
    storage::PoolKey,
};

use oraiswap::mixed_router::{self, Affiliate, SwapOperation};

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
            asset.transfer_from(msgs, info, address)?;
        }
        AssetInfo::NativeToken { denom: _ } => {
            let balance = asset.info.balance(querier, address)?;
            if balance < asset.amount {
                return Err(ContractError::NoFundSent {});
            }
        }
    }
    Ok(())
}

pub fn build_swap_msg(
    asset: &AssetInfo,
    swap_router: Addr,
    amount: Uint128,
    operations: Vec<SwapOperation>,
    minimum_receive: Option<Uint128>,
    to: Option<Addr>,
    affiliates: Option<Vec<Affiliate>>,
) -> Result<WasmMsg, ContractError> {
    match asset {
        AssetInfo::Token { contract_addr } => Ok(wasm_execute(
            contract_addr,
            &Cw20ExecuteMsg::Send {
                contract: swap_router.to_string(),
                amount,
                msg: to_json_binary(&mixed_router::ExecuteMsg::ExecuteSwapOperations {
                    operations,
                    minimum_receive,
                    to,
                    affiliates,
                })?,
            },
            vec![],
        )?),
        AssetInfo::NativeToken { denom } => Ok(wasm_execute(
            swap_router,
            &mixed_router::ExecuteMsg::ExecuteSwapOperations {
                operations,
                minimum_receive,
                to,
                affiliates,
            },
            coins(amount.u128(), denom),
        )?),
    }
}
