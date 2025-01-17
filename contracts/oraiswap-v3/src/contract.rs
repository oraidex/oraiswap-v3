#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::entrypoints::*;
use crate::state::CONFIG;
use oraiswap_v3_common::{
    error::ContractError,
    oraiswap_v3_msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    storage::Config,
};

use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oraiswap_v3";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        fee_tiers: vec![],
        admin: info.sender,
        protocol_fee: msg.protocol_fee,
        incentives_fund_manager: msg.incentives_fund_manager,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ChangeAdmin { new_admin } => change_admin(deps, info, new_admin),
        ExecuteMsg::WithdrawProtocolFee { pool_key } => withdraw_protocol_fee(deps, info, pool_key),
        ExecuteMsg::WithdrawAllProtocolFee { receiver } => {
            withdraw_all_protocol_fee(deps, info, receiver)
        }
        ExecuteMsg::ChangeProtocolFee { protocol_fee } => {
            change_protocol_fee(deps, info, protocol_fee)
        }
        ExecuteMsg::ChangeFeeReceiver {
            pool_key,
            fee_receiver,
        } => change_fee_receiver(deps, info, pool_key, fee_receiver),
        ExecuteMsg::CreatePosition {
            pool_key,
            lower_tick,
            upper_tick,
            liquidity_delta,
            slippage_limit_lower,
            slippage_limit_upper,
        } => create_position(
            deps,
            env,
            info,
            pool_key,
            lower_tick,
            upper_tick,
            liquidity_delta,
            slippage_limit_lower,
            slippage_limit_upper,
        ),
        ExecuteMsg::Swap {
            pool_key,
            x_to_y,
            amount,
            by_amount_in,
            sqrt_price_limit,
        } => swap(
            deps,
            env,
            info,
            pool_key,
            x_to_y,
            amount,
            by_amount_in,
            sqrt_price_limit,
        ),
        ExecuteMsg::SwapRoute {
            amount_in,
            expected_amount_out,
            slippage,
            swaps,
        } => swap_route(
            deps,
            env,
            info,
            amount_in,
            expected_amount_out,
            slippage,
            swaps,
        ),
        ExecuteMsg::TransferPosition { index, receiver } => {
            transfer_position(deps, env, info, index, receiver)
        }
        ExecuteMsg::ClaimFee { index } => claim_fee(deps, env, info, index),
        ExecuteMsg::RemovePosition { index } => remove_position(deps, env, info, index),
        ExecuteMsg::CreatePool {
            token_0,
            token_1,
            fee_tier,
            init_sqrt_price,
            init_tick,
        } => create_pool(
            deps,
            info,
            env,
            token_0,
            token_1,
            fee_tier,
            init_sqrt_price,
            init_tick,
        ),
        ExecuteMsg::AddFeeTier { fee_tier } => add_fee_tier(deps, env, info, fee_tier),
        ExecuteMsg::RemoveFeeTier { fee_tier } => remove_fee_tier(deps, env, info, fee_tier),
        ExecuteMsg::Approve {
            spender,
            token_id,
            expires,
        } => handle_approve(deps, env, info, spender, token_id, expires),
        ExecuteMsg::Revoke { spender, token_id } => {
            handle_revoke(deps, env, info, spender, token_id)
        }
        ExecuteMsg::ApproveAll { operator, expires } => {
            handle_approve_all(deps, env, info, operator, expires)
        }
        ExecuteMsg::RevokeAll { operator } => handle_revoke_all(deps, env, info, operator),
        ExecuteMsg::TransferNft {
            recipient,
            token_id,
        } => handle_transfer_nft(deps, env, info, recipient, token_id),
        ExecuteMsg::Burn { token_id } => handle_burn(deps, env, info, token_id),
        ExecuteMsg::SendNft {
            contract,
            token_id,
            msg,
        } => handle_send_nft(deps, env, info, contract, token_id, msg),
        ExecuteMsg::Mint { extension } => handle_mint(
            deps,
            env,
            info,
            extension.pool_key,
            extension.lower_tick,
            extension.upper_tick,
            extension.liquidity_delta,
            extension.slippage_limit_lower,
            extension.slippage_limit_upper,
        ),
        ExecuteMsg::CreateIncentive {
            pool_key,
            reward_token,
            total_reward,
            reward_per_sec,
            start_timestamp,
        } => create_incentive(
            deps,
            env,
            info,
            pool_key,
            reward_token,
            total_reward,
            reward_per_sec,
            start_timestamp,
        ),
        ExecuteMsg::ClaimIncentive { index } => claim_incentives(deps, env, info, index),
        ExecuteMsg::UpdateIncentive {
            pool_key,
            incentive_id,
            remaining_reward,
            start_timestamp,
            reward_per_sec,
        } => update_incentive(
            deps,
            env,
            info,
            pool_key,
            incentive_id,
            remaining_reward,
            start_timestamp,
            reward_per_sec,
        ),
        ExecuteMsg::UpdatePoolStatus { pool_key, status } => {
            update_pool_status(deps, info, pool_key, status)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Admin {} => to_json_binary(&query_admin(deps)?),
        QueryMsg::ProtocolFee {} => to_json_binary(&get_protocol_fee(deps)?),
        QueryMsg::IncentivesFundManager {} => to_json_binary(&get_incentives_fund_manager(deps)?),
        QueryMsg::Position { owner_id, index } => {
            to_json_binary(&get_position(deps, owner_id, index)?)
        }
        QueryMsg::Positions {
            owner_id,
            limit,
            offset,
        } => to_json_binary(&get_positions(deps, owner_id, limit, offset)?),
        QueryMsg::FeeTierExist { fee_tier } => to_json_binary(&fee_tier_exist(deps, fee_tier)?),
        QueryMsg::Pool {
            token_0,
            token_1,
            fee_tier,
        } => to_json_binary(&get_pool(deps, token_0, token_1, fee_tier)?),
        QueryMsg::Pools { limit, start_after } => {
            to_json_binary(&get_pools(deps, limit, start_after)?)
        }
        QueryMsg::Tick { key, index } => to_json_binary(&get_tick(deps, key, index)?),
        QueryMsg::IsTickInitialized { key, index } => {
            to_json_binary(&is_tick_initialized(deps, key, index)?)
        }
        QueryMsg::FeeTiers {} => to_json_binary(&get_fee_tiers(deps)?),
        QueryMsg::PositionTicks { owner, offset } => {
            to_json_binary(&get_position_ticks(deps, owner, offset)?)
        }
        QueryMsg::UserPositionAmount { owner } => {
            to_json_binary(&get_user_position_amount(deps, owner)?)
        }
        QueryMsg::TickMap {
            pool_key,
            lower_tick_index,
            upper_tick_index,
            x_to_y,
        } => to_json_binary(&get_tickmap(
            deps,
            pool_key,
            lower_tick_index,
            upper_tick_index,
            x_to_y,
        )?),
        QueryMsg::LiquidityTicks {
            pool_key,
            tick_indexes,
        } => to_json_binary(&get_liquidity_ticks(deps, pool_key, tick_indexes)?),
        QueryMsg::LiquidityTicksAmount {
            pool_key,
            lower_tick,
            upper_tick,
        } => to_json_binary(&get_liquidity_ticks_amount(
            deps, pool_key, lower_tick, upper_tick,
        )?),
        QueryMsg::PoolsForPair { token_0, token_1 } => {
            to_json_binary(&get_all_pools_for_pair(deps, token_0, token_1)?)
        }
        QueryMsg::Quote {
            pool_key,
            x_to_y,
            amount,
            by_amount_in,
            sqrt_price_limit,
        } => to_json_binary(&quote(
            deps,
            env,
            pool_key,
            x_to_y,
            amount,
            by_amount_in,
            sqrt_price_limit,
        )?),
        QueryMsg::QuoteRoute { amount_in, swaps } => {
            to_json_binary(&quote_route(deps, env, amount_in, swaps)?)
        }
        QueryMsg::OwnerOf {
            token_id,
            include_expired,
        } => to_json_binary(&query_owner_of(
            deps,
            env,
            token_id,
            include_expired.unwrap_or(false),
        )?),
        QueryMsg::ApprovedForAll {
            owner,
            include_expired,
            start_after,
            limit,
        } => to_json_binary(&query_all_approvals(
            deps,
            env,
            owner,
            include_expired.unwrap_or(false),
            start_after,
            limit,
        )?),
        QueryMsg::NftInfo { token_id } => to_json_binary(&query_nft_info(deps, token_id)?),
        QueryMsg::AllNftInfo {
            token_id,
            include_expired,
        } => to_json_binary(&query_all_nft_info(
            deps,
            env,
            token_id,
            include_expired.unwrap_or(false),
        )?),
        QueryMsg::Tokens {
            owner,
            start_after,
            limit,
        } => to_json_binary(&query_tokens(deps, owner, start_after, limit)?),
        QueryMsg::AllTokens { start_after, limit } => {
            to_json_binary(&query_all_tokens(deps, start_after, limit)?)
        }
        QueryMsg::NumTokens {} => to_json_binary(&query_num_tokens(deps)?),
        QueryMsg::PositionIncentives { owner_id, index } => {
            to_json_binary(&query_position_incentives(deps, env, owner_id, index)?)
        }
        QueryMsg::PoolsByPoolKeys { pool_keys } => {
            to_json_binary(&get_pools_with_pool_keys(deps, pool_keys)?)
        }
        QueryMsg::AllPosition { limit, start_after } => {
            to_json_binary(&query_all_positions(deps, limit, start_after)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let original_version =
        cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // // query all position, then update token id
    // let positions: Vec<_> = crate::state::POSITIONS
    //     .range_raw(deps.storage, None, None, cosmwasm_std::Order::Ascending)
    //     .collect();
    // let mut token_id = 0;
    // for item in positions {
    //     if let Ok((key, mut position)) = item {
    //         token_id += 1;
    //         position.token_id = token_id;
    //         let account_id = &key[..key.len() - 4];
    //         let index = u32::from_be_bytes(key[key.len() - 4..].try_into().unwrap());
    //         // update position and its index
    //         crate::state::POSITIONS.save(deps.storage, &key, &position)?;
    //         crate::state::POSITION_KEYS_BY_TOKEN_ID.save(
    //             deps.storage,
    //             position.token_id,
    //             &(account_id.to_vec(), index),
    //         )?;
    //     }
    // }

    // // update total token id, first time token count is total token ids
    // crate::state::TOKEN_COUNT.save(deps.storage, &token_id)?;
    // crate::state::TOKEN_ID.save(deps.storage, &token_id)?;

    Ok(Response::new().add_attribute("new_version", original_version.to_string()))
}
