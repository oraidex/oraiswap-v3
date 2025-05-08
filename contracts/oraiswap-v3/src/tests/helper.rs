use cosmwasm_std::{Addr, Binary, Coin, Event, StdResult, Uint64};
use cosmwasm_testing_util::{ExecuteResponse, MockResult};

use cosmwasm_testing_util::ContractWrapper;
use derive_more::{Deref, DerefMut};
use oraiswap_v3_common::asset::{Asset, AssetInfo};
use oraiswap_v3_common::interface::{PoolWithPoolKey, QuoteResult, SwapHop};
use oraiswap_v3_common::math::liquidity::Liquidity;
use oraiswap_v3_common::math::percentage::Percentage;
use oraiswap_v3_common::math::sqrt_price::SqrtPrice;
use oraiswap_v3_common::math::token_amount::TokenAmount;
use oraiswap_v3_common::oraiswap_v3_msg;
use oraiswap_v3_common::storage::{
    FeeTier, LiquidityTick, Pool, PoolKey, PoolStatus, Position, Tick,
};

use crate::state::MAX_LIMIT;

pub const FEE_DENOM: &str = "orai";

#[cfg(not(feature = "test-tube"))]
pub type TestMockApp = cosmwasm_testing_util::MultiTestMockApp;
#[cfg(feature = "test-tube")]
pub type TestMockApp = cosmwasm_testing_util::TestTubeMockApp;

#[derive(Deref, DerefMut)]
pub struct MockApp {
    #[deref]
    #[deref_mut]
    app: TestMockApp,
    dex_id: u64,
    incentives_id: u64,
}

#[allow(dead_code)]
impl MockApp {
    pub fn new(init_balances: &[(&str, &[Coin])]) -> (Self, Vec<String>) {
        let (mut app, accounts) = TestMockApp::new(init_balances);

        let dex_id;
        let incentives_id;
        #[cfg(not(feature = "test-tube"))]
        {
            dex_id = app.upload(Box::new(
                cosmwasm_testing_util::ContractWrapper::new_with_empty(
                    crate::contract::execute,
                    crate::contract::instantiate,
                    crate::contract::query,
                ),
            ));
            incentives_id = app.upload(Box::new(ContractWrapper::new_with_empty(
                incentives_fund_manager::contract::execute,
                incentives_fund_manager::contract::instantiate,
                incentives_fund_manager::contract::query,
            )));
        }
        #[cfg(feature = "test-tube")]
        {
            dex_id = app.upload(include_bytes!("./testdata/oraiswap-v3.wasm"));
            incentives_id = app.upload(include_bytes!("./testdata/incentives-fund-manager.wasm"));
        }

        (
            Self {
                app,
                dex_id,
                incentives_id,
            },
            accounts,
        )
    }

    pub fn create_dex(&mut self, owner: &str, protocol_fee: Percentage) -> MockResult<Addr> {
        // create incentive_contract
        let incentive_id = self.incentives_id;

        let incentive_addr = self.instantiate(
            incentive_id,
            Addr::unchecked(owner),
            &oraiswap_v3_common::incentives_fund_manager::InstantiateMsg {
                owner: None,
                oraiswap_v3: Addr::unchecked("oraiswap_v3"),
            },
            &[],
            "incentives_fund_mamnager",
        )?;

        let code_id = self.dex_id;
        let dex_addr = self.instantiate(
            code_id,
            Addr::unchecked(owner),
            &oraiswap_v3_msg::InstantiateMsg {
                protocol_fee,
                incentives_fund_manager: incentive_addr.clone(),
            },
            &[],
            "oraiswap_v3",
        )?;

        // update config for incentive_contract
        self.execute(
            Addr::unchecked(owner),
            incentive_addr.clone(),
            &oraiswap_v3_common::incentives_fund_manager::ExecuteMsg::UpdateConfig {
                owner: None,
                oraiswap_v3: Some(dex_addr.clone()),
            },
            &[],
        )?;

        Ok(dex_addr)
    }

    pub fn get_incentives_fund_manager(&mut self, dex: &str) -> StdResult<Addr> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::IncentivesFundManager {},
        )
    }

    pub fn add_fee_tier(
        &mut self,
        sender: &str,
        dex: &str,
        fee_tier: FeeTier,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::AddFeeTier { fee_tier },
            &[],
        )
    }

    pub fn remove_fee_tier(
        &mut self,
        sender: &str,
        dex: &str,
        fee_tier: FeeTier,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::RemoveFeeTier { fee_tier },
            &[],
        )
    }

    pub fn create_pool(
        &mut self,
        sender: &str,
        dex: &str,
        token_x: &str,
        token_y: &str,
        fee_tier: FeeTier,
        init_sqrt_price: SqrtPrice,
        init_tick: i32,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::CreatePool {
                token_0: token_x.to_string(),
                token_1: token_y.to_string(),
                fee_tier,
                init_sqrt_price,
                init_tick,
            },
            &[],
        )
    }

    pub fn withdraw_protocol_fee(
        &mut self,
        sender: &str,
        dex: &str,
        pool_key: &PoolKey,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::WithdrawProtocolFee {
                pool_key: pool_key.clone(),
            },
            &[],
        )
    }

    pub fn withdraw_all_protocol_fee(
        &mut self,
        sender: &str,
        dex: &str,
        receiver: Option<Addr>,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::WithdrawAllProtocolFee { receiver },
            &[],
        )
    }

    pub fn change_fee_receiver(
        &mut self,
        sender: &str,
        dex: &str,
        pool_key: &PoolKey,
        fee_recevier: &str,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::ChangeFeeReceiver {
                pool_key: pool_key.clone(),
                fee_receiver: Addr::unchecked(fee_recevier),
            },
            &[],
        )
    }

    pub fn create_position(
        &mut self,
        sender: &str,
        dex: &str,
        pool_key: &PoolKey,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_delta: Liquidity,
        slippage_limit_lower: SqrtPrice,
        slippage_limit_upper: SqrtPrice,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::CreatePosition {
                pool_key: pool_key.clone(),
                lower_tick,
                upper_tick,
                liquidity_delta,
                slippage_limit_lower,
                slippage_limit_upper,
            },
            &[],
        )
    }

    pub fn transfer_position(
        &mut self,
        sender: &str,
        dex: &str,
        index: u32,
        receiver: &str,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::TransferPosition {
                index,
                receiver: receiver.to_string(),
            },
            &[],
        )
    }

    pub fn remove_position(
        &mut self,
        sender: &str,
        dex: &str,
        index: u32,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::RemovePosition { index },
            &[],
        )
    }

    pub fn swap_route(
        &mut self,
        sender: &str,
        dex: &str,
        amount_in: TokenAmount,
        expected_amount_out: TokenAmount,
        slippage: Percentage,
        swaps: Vec<SwapHop>,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::SwapRoute {
                amount_in,
                expected_amount_out,
                slippage,
                swaps,
            },
            &[],
        )
    }

    pub fn swap(
        &mut self,
        sender: &str,
        dex: &str,
        pool_key: &PoolKey,
        x_to_y: bool,
        amount: TokenAmount,
        by_amount_in: bool,
        sqrt_price_limit: SqrtPrice,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::Swap {
                pool_key: pool_key.clone(),
                x_to_y,
                amount,
                by_amount_in,
                sqrt_price_limit,
            },
            &[],
        )
    }

    pub fn claim_fee(
        &mut self,
        sender: &str,
        dex: &str,
        index: u32,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::ClaimFee { index },
            &[],
        )
    }

    pub fn claim_incentives(
        &mut self,
        sender: &str,
        dex: &str,
        index: u32,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::ClaimIncentive { index },
            &[],
        )
    }

    pub fn quote_route(
        &mut self,
        dex: &str,
        amount_in: TokenAmount,
        swaps: Vec<SwapHop>,
    ) -> StdResult<TokenAmount> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::QuoteRoute { amount_in, swaps },
        )
    }

    pub fn quote(
        &mut self,
        dex: &str,
        pool_key: &PoolKey,
        x_to_y: bool,
        amount: TokenAmount,
        by_amount_in: bool,
        sqrt_price_limit: SqrtPrice,
    ) -> StdResult<QuoteResult> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::Quote {
                pool_key: pool_key.clone(),
                x_to_y,
                amount,
                by_amount_in,
                sqrt_price_limit,
            },
        )
    }

    pub fn get_pool(
        &self,
        dex: &str,
        token_x: &str,
        token_y: &str,
        fee_tier: FeeTier,
    ) -> StdResult<Pool> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::Pool {
                token_0: Addr::unchecked(token_x).to_string(),
                token_1: Addr::unchecked(token_y).to_string(),
                fee_tier,
            },
        )
    }

    pub fn get_liquidity_ticks(
        &self,
        dex: &str,
        pool_key: &PoolKey,
        tick_indexes: Vec<i32>,
    ) -> StdResult<Vec<LiquidityTick>> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::LiquidityTicks {
                pool_key: pool_key.clone(),
                tick_indexes,
            },
        )
    }

    pub fn get_pools(
        &self,
        dex: &str,
        limit: Option<u32>,
        start_after: Option<PoolKey>,
    ) -> StdResult<Vec<PoolWithPoolKey>> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::Pools { limit, start_after },
        )
    }

    pub fn get_position(&self, dex: &str, owner_id: &str, index: u32) -> StdResult<Position> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::Position {
                owner_id: Addr::unchecked(owner_id),
                index,
            },
        )
    }

    pub fn get_position_incentives(
        &self,
        dex: &str,
        owner_id: &str,
        index: u32,
    ) -> StdResult<Vec<Asset>> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::PositionIncentives {
                owner_id: Addr::unchecked(owner_id),
                index,
            },
        )
    }

    pub fn get_all_positions(&self, dex: &str, owner_id: &str) -> StdResult<Vec<Position>> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::Positions {
                owner_id: Addr::unchecked(owner_id),
                limit: Some(MAX_LIMIT),
                offset: Some(0),
            },
        )
    }

    pub fn fee_tier_exist(&self, dex: &str, fee_tier: FeeTier) -> StdResult<bool> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::FeeTierExist { fee_tier },
        )
    }

    pub fn get_tick(&self, dex: &str, pool_key: &PoolKey, index: i32) -> StdResult<Tick> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::Tick {
                key: pool_key.clone(),
                index,
            },
        )
    }

    pub fn is_tick_initialized(
        &self,
        dex: &str,
        pool_key: &PoolKey,
        index: i32,
    ) -> StdResult<bool> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::IsTickInitialized {
                key: pool_key.clone(),
                index,
            },
        )
    }

    pub fn create_incentive(
        &mut self,
        sender: &str,
        dex: &str,
        pool_key: &PoolKey,
        reward_token: AssetInfo,
        total_reward: Option<TokenAmount>,
        reward_per_sec: TokenAmount,
        start_timestamp: Option<u64>,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::CreateIncentive {
                pool_key: pool_key.clone(),
                reward_token,
                total_reward,
                reward_per_sec,
                start_timestamp,
            },
            &[],
        )
    }
    pub fn query_all_positions(
        &self,
        dex: &str,
        limit: Option<u32>,
        start_after: Option<Binary>,
    ) -> StdResult<Vec<Position>> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::AllPosition { limit, start_after },
        )
    }

    pub fn query_tickmaps(
        &self,
        dex: &str,
        pool_key: &PoolKey,
        lower_tick: i32,
        upper_tick: i32,
        x_to_y: bool,
    ) -> StdResult<Vec<(u16, u64)>> {
        let tickmaps: Vec<(u16, Uint64)> = self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::TickMap {
                pool_key: pool_key.clone(),
                lower_tick_index: lower_tick,
                upper_tick_index: upper_tick,
                x_to_y,
            },
        )?;

        Ok(tickmaps.into_iter().map(|(k, v)| (k, v.u64())).collect())
    }

    pub fn update_pool_status(
        &mut self,
        sender: &str,
        dex: &str,
        pool_key: &PoolKey,
        status: Option<PoolStatus>,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::UpdatePoolStatus {
                pool_key: pool_key.clone(),
                status,
            },
            &[],
        )
    }

    pub fn pause(
        &mut self,
        sender: &str,
        dex: &str,
        pause_status: bool,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(dex),
            &oraiswap_v3_msg::ExecuteMsg::Pause { pause_status },
            &[],
        )
    }
}

pub fn extract_amount(events: &[Event], key: &str) -> Option<TokenAmount> {
    for event in events {
        if event.ty == "wasm" {
            for attr in &event.attributes {
                if attr.key == key {
                    return attr.value.parse::<u128>().map(TokenAmount).ok();
                }
            }
        }
    }
    None
}

pub fn subtract_assets(old_assets: &[Asset], new_assets: &[Asset]) -> Vec<Asset> {
    let mut assets = vec![];
    for asset in new_assets {
        let amount = asset.amount
            - old_assets
                .iter()
                .find(|a| a.info.eq(&asset.info))
                .map(|a| a.amount)
                .unwrap_or_default();
        assets.push(Asset {
            info: asset.info.clone(),
            amount,
        })
    }
    assets
}

pub mod macros {

    macro_rules! create_dex {
        ($app:ident, $protocol_fee:expr,$owner: tt) => {{
            $app.create_dex($owner, $protocol_fee).unwrap()
        }};
    }
    pub(crate) use create_dex;

    macro_rules! create_tokens {
        ($app:ident, $token_x_supply:expr, $token_y_supply:expr, $owner: tt) => {{
            let token_x = $app.create_token($owner, "tokenx", $token_x_supply);
            let token_y = $app.create_token($owner, "tokeny", $token_y_supply);
            if token_x < token_y {
                (token_x, token_y)
            } else {
                (token_y, token_x)
            }
        }};
        ($app:ident, $token_x_supply:expr, $token_y_supply:expr,$owner:tt) => {{
            create_tokens!($app, $token_x_supply, $token_y_supply, $owner)
        }};
        ($app:ident, $token_supply:expr,$owner:tt) => {{
            create_tokens!($app, $token_supply, $token_supply, $owner)
        }};
    }

    pub(crate) use create_tokens;

    macro_rules! create_3_tokens {
        ($app:ident, $token_x_supply:expr, $token_y_supply:expr,$token_z_supply:expr, $owner: tt) => {{
            let mut tokens = [
                $app.create_token($owner, "tokenx", $token_x_supply),
                $app.create_token($owner, "tokeny", $token_y_supply),
                $app.create_token($owner, "tokenz", $token_y_supply),
            ];
            tokens.sort();
            (tokens[0].clone(), tokens[1].clone(), tokens[2].clone())
        }};
        ($app:ident, $token_x_supply:expr, $token_y_supply:expr,$token_z_supply:expr,$owner:tt) => {{
            create_3_tokens!(
                $app,
                $token_x_supply,
                $token_y_supply,
                $token_z_supply,
                $owner
            )
        }};
    }
    pub(crate) use create_3_tokens;

    macro_rules! create_pool {
        ($app:ident, $dex_address:expr, $token_0:expr, $token_1:expr, $fee_tier:expr, $init_sqrt_price:expr, $init_tick:expr, $caller:tt) => {{
            $app.create_pool(
                $caller,
                $dex_address.as_str(),
                $token_0.as_str(),
                $token_1.as_str(),
                $fee_tier,
                $init_sqrt_price,
                $init_tick,
            )
        }};
    }
    pub(crate) use create_pool;

    macro_rules! add_fee_tier {
        ($app:ident, $dex_address:expr, $fee_tier:expr, $caller:tt) => {{
            $app.add_fee_tier($caller, $dex_address.as_str(), $fee_tier)
        }};
    }
    pub(crate) use add_fee_tier;

    macro_rules! remove_fee_tier {
        ($app:ident, $dex_address:expr, $fee_tier:expr, $caller:tt) => {{
            $app.remove_fee_tier($caller, $dex_address.as_str(), $fee_tier)
        }};
    }
    pub(crate) use remove_fee_tier;

    macro_rules! approve {
        ($app:ident, $token_address:expr, $spender:expr, $value:expr, $caller:tt) => {{
            $app.approve_token($token_address.as_str(), $caller, $spender.as_str(), $value)
        }};
    }
    pub(crate) use approve;

    macro_rules! fee_tier_exist {
        ($app:ident, $dex_address:expr, $fee_tier:expr) => {{
            $app.fee_tier_exist($dex_address.as_str(), $fee_tier)
                .unwrap()
        }};
    }
    pub(crate) use fee_tier_exist;

    macro_rules! create_incentive {
        ($app:ident, $dex_address:expr, $pool_key:expr, $reward_token:expr, $total_reward:expr, $reward_per_sec:expr, $start_timestamp:expr, $caller:tt) => {{
            $app.create_incentive(
                $caller,
                $dex_address.as_str(),
                &$pool_key,
                $reward_token,
                $total_reward,
                $reward_per_sec,
                $start_timestamp,
            )
        }};
    }
    pub(crate) use create_incentive;

    macro_rules! create_position {
        ($app:ident, $dex_address:expr, $pool_key:expr, $lower_tick:expr, $upper_tick:expr, $liquidity_delta:expr, $slippage_limit_lower:expr, $slippage_limit_upper:expr, $caller:tt) => {{
            $app.create_position(
                $caller,
                $dex_address.as_str(),
                &$pool_key,
                $lower_tick,
                $upper_tick,
                $liquidity_delta,
                $slippage_limit_lower,
                $slippage_limit_upper,
            )
        }};
    }
    pub(crate) use create_position;

    macro_rules! remove_position {
        ($app:ident,  $dex_address:expr, $index:expr, $caller:tt) => {{
            $app.remove_position($caller, $dex_address.as_str(), $index)
        }};
    }
    pub(crate) use remove_position;

    macro_rules! get_pool {
        ($app:ident, $dex_address:expr, $token_0:expr, $token_1:expr, $fee_tier:expr) => {{
            $app.get_pool(
                $dex_address.as_str(),
                $token_0.as_str(),
                $token_1.as_str(),
                $fee_tier,
            )
        }};
    }
    pub(crate) use get_pool;

    macro_rules! get_position {
        ($app:ident, $dex_address:expr, $index:expr, $owner:tt) => {{
            $app.get_position($dex_address.as_str(), $owner, $index)
        }};
    }
    pub(crate) use get_position;

    macro_rules! get_position_incentives {
        ($app:ident, $dex_address:expr, $index:expr, $owner:tt) => {{
            $app.get_position_incentives($dex_address.as_str(), $owner, $index)
        }};
    }
    pub(crate) use get_position_incentives;

    macro_rules! get_tick {
        ($app:ident, $dex_address:expr, $key:expr, $index:expr) => {{
            $app.get_tick($dex_address.as_str(), &$key, $index)
        }};
    }
    pub(crate) use get_tick;

    macro_rules! is_tick_initialized {
        ($app:ident, $dex_address:expr, $key:expr, $index:expr) => {{
            $app.is_tick_initialized($dex_address.as_str(), &$key, $index)
                .unwrap()
        }};
    }
    pub(crate) use is_tick_initialized;

    macro_rules! mint {
        ($app:ident, $token_address:expr, $to:tt, $value:expr, $caller:tt) => {{
            $app.mint_token($caller, $to, $token_address.as_str(), $value)
        }};
    }
    pub(crate) use mint;

    macro_rules! quote {
        ($app:ident,  $dex_address:expr, $pool_key:expr, $x_to_y:expr, $amount:expr, $by_amount_in:expr, $sqrt_price_limit:expr) => {{
            $app.quote(
                $dex_address.as_str(),
                &$pool_key,
                $x_to_y,
                $amount,
                $by_amount_in,
                $sqrt_price_limit,
            )
        }};
    }
    pub(crate) use quote;

    macro_rules! balance_of {
        // any type that can converted to string
        ($app:ident, $token_address:expr, $owner:expr) => {{
            $app.query_token_balance($token_address.as_str(), &$owner.to_string())
                .unwrap()
                .u128()
        }};
        ($app:ident, $token_address:expr, $owner:tt) => {{
            $app.query_token_balance($token_address.as_str(), $owner)
                .unwrap()
                .u128()
        }};
    }
    pub(crate) use balance_of;

    macro_rules! swap {
        ($app:ident, $dex_address:expr, $pool_key:expr, $x_to_y:expr, $amount:expr, $by_amount_in:expr, $sqrt_price_limit:expr, $caller:tt) => {{
            $app.swap(
                $caller,
                $dex_address.as_str(),
                &$pool_key,
                $x_to_y,
                $amount,
                $by_amount_in,
                $sqrt_price_limit,
            )
        }};
    }
    pub(crate) use swap;

    macro_rules! quote_route {
        ($app:ident, $dex_address:expr, $amount_in:expr, $swaps:expr) => {{
            $app.quote_route($dex_address.as_str(), $amount_in, $swaps)
        }};
    }
    pub(crate) use quote_route;

    macro_rules! swap_route {
        ($app:ident, $dex_address:expr, $amount_in:expr, $expected_amount_out:expr, $slippage:expr, $swaps:expr, $caller:tt) => {{
            $app.swap_route(
                $caller,
                $dex_address.as_str(),
                $amount_in,
                $expected_amount_out,
                $slippage,
                $swaps,
            )
        }};
    }
    pub(crate) use swap_route;

    macro_rules! claim_fee {
        ($app:ident, $dex_address:expr, $index:expr, $caller:tt) => {{
            $app.claim_fee($caller, $dex_address.as_str(), $index)
        }};
    }
    pub(crate) use claim_fee;

    macro_rules! claim_incentives {
        ($app:ident, $dex_address:expr, $index:expr, $caller:tt) => {{
            $app.claim_incentives($caller, $dex_address.as_str(), $index)
        }};
    }
    pub(crate) use claim_incentives;

    macro_rules! init_slippage_pool_with_liquidity {
        ($app:ident, $dex_address:ident, $token_x_address:ident, $token_y_address:ident,$owner:tt) => {{
            let fee_tier = FeeTier {
                fee: Percentage::from_scale(6, 3),
                tick_spacing: 10,
            };
            add_fee_tier!($app, $dex_address, fee_tier, $owner).unwrap();

            let init_tick = 0;
            let init_sqrt_price =
                oraiswap_v3_common::math::sqrt_price::calculate_sqrt_price(init_tick).unwrap();
            create_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier,
                init_sqrt_price,
                init_tick,
                $owner
            )
            .unwrap();

            let mint_amount = 10u128.pow(10);
            approve!($app, $token_x_address, $dex_address, mint_amount, $owner).unwrap();
            approve!($app, $token_y_address, $dex_address, mint_amount, $owner).unwrap();

            let pool_key = PoolKey::new(
                $token_x_address.to_string(),
                $token_y_address.to_string(),
                fee_tier,
            )
            .unwrap();
            let lower_tick = -1000;
            let upper_tick = 1000;
            let liquidity =
                oraiswap_v3_common::math::liquidity::Liquidity::from_integer(10_000_000_000u128);

            let pool_before = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();
            let slippage_limit_lower = pool_before.sqrt_price;
            let slippage_limit_upper = pool_before.sqrt_price;
            create_position!(
                $app,
                $dex_address,
                pool_key,
                lower_tick,
                upper_tick,
                liquidity,
                slippage_limit_lower,
                slippage_limit_upper,
                $owner
            )
            .unwrap();

            let pool_after = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();

            assert_eq!(pool_after.liquidity, liquidity);

            pool_key
        }};
    }
    pub(crate) use init_slippage_pool_with_liquidity;

    macro_rules! init_basic_pool {
        ($app:ident, $dex_address:ident, $token_x_address:ident, $token_y_address:ident,$owner:tt) => {{
            let fee_tier = FeeTier {
                fee: Percentage::from_scale(6, 3),
                tick_spacing: 10,
            };

            add_fee_tier!($app, $dex_address, fee_tier, $owner).unwrap();

            let init_tick = 0;
            let init_sqrt_price =
                oraiswap_v3_common::math::sqrt_price::calculate_sqrt_price(init_tick).unwrap();
            create_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier,
                init_sqrt_price,
                init_tick,
                $owner
            )
            .unwrap();
        }};
    }
    pub(crate) use init_basic_pool;

    macro_rules! init_basic_position {
        ($app:ident, $dex_address:ident, $token_x_address:ident, $token_y_address:ident,$owner:tt) => {{
            let fee_tier = FeeTier {
                fee: Percentage::from_scale(6, 3),
                tick_spacing: 10,
            };

            let mint_amount = 10u128.pow(10);
            approve!($app, $token_x_address, $dex_address, mint_amount, $owner).unwrap();
            approve!($app, $token_y_address, $dex_address, mint_amount, $owner).unwrap();

            let pool_key = oraiswap_v3_common::storage::PoolKey::new(
                $token_x_address.to_string(),
                $token_y_address.to_string(),
                fee_tier,
            )
            .unwrap();
            let lower_tick = -20;
            let upper_tick = 10;
            let liquidity = oraiswap_v3_common::math::liquidity::Liquidity::from_integer(1000000);

            let pool_before = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();
            let slippage_limit_lower = pool_before.sqrt_price;
            let slippage_limit_upper = pool_before.sqrt_price;
            create_position!(
                $app,
                $dex_address,
                pool_key,
                lower_tick,
                upper_tick,
                liquidity,
                slippage_limit_lower,
                slippage_limit_upper,
                $owner
            )
            .unwrap();

            let pool_after = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();

            assert_eq!(pool_after.liquidity, liquidity);
        }};
    }
    pub(crate) use init_basic_position;

    macro_rules! init_cross_position {
        ($app:ident, $dex_address:ident, $token_x_address:ident, $token_y_address:ident,$owner:tt) => {{
            let fee_tier = FeeTier {
                fee: Percentage::from_scale(6, 3),
                tick_spacing: 10,
            };

            let mint_amount = 10u128.pow(10);
            approve!($app, $token_x_address, $dex_address, mint_amount, $owner).unwrap();
            approve!($app, $token_y_address, $dex_address, mint_amount, $owner).unwrap();

            let pool_key = PoolKey::new(
                $token_x_address.to_string(),
                $token_y_address.to_string(),
                fee_tier,
            )
            .unwrap();
            let lower_tick = -40;
            let upper_tick = -10;
            let liquidity = Liquidity::from_integer(1000000);

            let pool_before = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();
            let slippage_limit_lower = pool_before.sqrt_price;
            let slippage_limit_upper = pool_before.sqrt_price;
            create_position!(
                $app,
                $dex_address,
                pool_key,
                lower_tick,
                upper_tick,
                liquidity,
                slippage_limit_lower,
                slippage_limit_upper,
                $owner
            )
            .unwrap();

            let pool_after = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();

            assert_eq!(pool_after.liquidity, liquidity);
        }};
    }
    pub(crate) use init_cross_position;

    macro_rules! swap_exact_limit {
        ($app:ident, $dex_address:ident, $pool_key:expr, $x_to_y:expr, $amount:expr, $by_amount_in:expr, $caller:tt) => {{
            let sqrt_price_limit = if $x_to_y {
                oraiswap_v3_common::math::sqrt_price::SqrtPrice::new(
                    oraiswap_v3_common::math::MIN_SQRT_PRICE,
                )
            } else {
                oraiswap_v3_common::math::sqrt_price::SqrtPrice::new(
                    oraiswap_v3_common::math::MAX_SQRT_PRICE,
                )
            };

            let quote_result = quote!(
                $app,
                $dex_address,
                $pool_key,
                $x_to_y,
                $amount,
                $by_amount_in,
                sqrt_price_limit
            )
            .unwrap();
            swap!(
                $app,
                $dex_address,
                $pool_key,
                $x_to_y,
                $amount,
                $by_amount_in,
                quote_result.target_sqrt_price,
                $caller
            )
            .unwrap();
        }};
    }
    pub(crate) use swap_exact_limit;

    macro_rules! init_dex_and_tokens {
        ($app:ident, $mint_amount:expr,$protocol_fee:expr,$owner:tt) => {{
            let (token_x, token_y) = create_tokens!($app, $mint_amount, $mint_amount, $owner);
            let dex = $app.create_dex($owner, $protocol_fee).unwrap();
            (dex, token_x, token_y)
        }};
        ($app:ident, $owner:tt) => {{
            init_dex_and_tokens!(
                $app,
                10u128.pow(10),
                oraiswap_v3_common::math::percentage::Percentage::from_scale(1, 2),
                $owner
            )
        }};
    }
    pub(crate) use init_dex_and_tokens;

    macro_rules! init_basic_swap {
        ($app:ident, $dex_address:ident, $token_x_address:ident, $token_y_address:ident,$owner:tt, $bob: tt) => {{
            let fee = Percentage::from_scale(6, 3);
            let tick_spacing = 10;
            let fee_tier = FeeTier { fee, tick_spacing };
            let pool_key = oraiswap_v3_common::storage::PoolKey::new(
                $token_x_address.to_string(),
                $token_y_address.to_string(),
                fee_tier,
            )
            .unwrap();
            let lower_tick = -20;

            let amount = 1000;

            mint!($app, $token_x_address, $bob, amount, $owner).unwrap();
            let amount_x = balance_of!($app, $token_x_address, $bob);
            assert_eq!(amount_x, amount);
            approve!($app, $token_x_address, $dex_address, amount, $bob).unwrap();

            let amount_x = balance_of!($app, $token_x_address, $dex_address);
            let amount_y = balance_of!($app, $token_y_address, $dex_address);
            assert_eq!(amount_x, 500);
            assert_eq!(amount_y, 1000);

            let pool_before = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                pool_key.fee_tier
            )
            .unwrap();

            let swap_amount = TokenAmount::new(amount);
            let slippage = oraiswap_v3_common::math::sqrt_price::SqrtPrice::new(
                oraiswap_v3_common::math::MIN_SQRT_PRICE,
            );
            swap!(
                $app,
                $dex_address,
                pool_key,
                true,
                swap_amount,
                true,
                slippage,
                $bob
            )
            .unwrap();

            let pool_after = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();
            assert_eq!(pool_after.liquidity, pool_before.liquidity);
            assert_eq!(pool_after.current_tick_index, lower_tick);
            assert_ne!(pool_after.sqrt_price, pool_before.sqrt_price);

            let amount_x = balance_of!($app, $token_x_address, $bob);
            let amount_y = balance_of!($app, $token_y_address, $bob);
            assert_eq!(amount_x, 0);
            assert_eq!(amount_y, 993);

            let amount_x = balance_of!($app, $token_x_address, $dex_address);
            let amount_y = balance_of!($app, $token_y_address, $dex_address);
            assert_eq!(amount_x, 1500);
            assert_eq!(amount_y, 7);

            assert_eq!(
                pool_after.fee_growth_global_x,
                oraiswap_v3_common::math::fee_growth::FeeGrowth::new(50000000000000000000000)
            );
            assert_eq!(
                pool_after.fee_growth_global_y,
                oraiswap_v3_common::math::fee_growth::FeeGrowth::new(0)
            );

            assert_eq!(pool_after.fee_protocol_token_x, TokenAmount::new(1));
            assert_eq!(pool_after.fee_protocol_token_y, TokenAmount::new(0));
        }};
    }
    pub(crate) use init_basic_swap;

    macro_rules! withdraw_protocol_fee {
        ($app:ident, $dex_address:expr, $pool_key:expr, $caller:tt) => {{
            $app.withdraw_protocol_fee($caller, $dex_address.as_str(), &$pool_key)
        }};
    }
    pub(crate) use withdraw_protocol_fee;

    macro_rules! withdraw_all_protocol_fee {
        ($app:ident, $dex_address:expr,$receiver:expr, $caller:tt) => {{
            $app.withdraw_all_protocol_fee($caller, $dex_address.as_str(), $receiver)
        }};
    }
    pub(crate) use withdraw_all_protocol_fee;

    macro_rules! change_fee_receiver {
        ($app:ident,  $dex_address:expr, $pool_key:expr, $fee_receiver:tt, $caller:tt) => {{
            $app.change_fee_receiver($caller, $dex_address.as_str(), &$pool_key, $fee_receiver)
        }};
    }
    pub(crate) use change_fee_receiver;

    macro_rules! init_cross_swap {
        ($app:ident, $dex_address:ident, $token_x_address:expr, $token_y_address:expr,$owner:tt,$bob:tt) => {{
            let fee = Percentage::from_scale(6, 3);
            let tick_spacing = 10;
            let fee_tier = FeeTier { fee, tick_spacing };
            let pool_key = PoolKey::new($token_x_address, $token_y_address, fee_tier).unwrap();
            let lower_tick = -20;

            let amount = 1000;

            mint!($app, $token_x_address, $bob, amount, $owner).unwrap();
            let amount_x = balance_of!($app, $token_x_address, $bob);
            assert_eq!(amount_x, amount);
            approve!($app, $token_x_address, $dex_address, amount, $bob).unwrap();

            let amount_x = balance_of!($app, $token_x_address, $dex_address);
            let amount_y = balance_of!($app, $token_y_address, $dex_address);
            assert_eq!(amount_x, 500);
            assert_eq!(amount_y, 2499);

            let pool_before = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();

            let swap_amount = oraiswap_v3_common::math::token_amount::TokenAmount::new(amount);
            let slippage = oraiswap_v3_common::math::sqrt_price::SqrtPrice::new(
                oraiswap_v3_common::math::MIN_SQRT_PRICE,
            );
            swap!(
                $app,
                $dex_address,
                pool_key,
                true,
                swap_amount,
                true,
                slippage,
                $bob
            )
            .unwrap();

            let pool_after = get_pool!(
                $app,
                $dex_address,
                $token_x_address,
                $token_y_address,
                fee_tier
            )
            .unwrap();
            let position_liquidity = Liquidity::from_integer(1000000);
            assert_eq!(
                pool_after.liquidity - position_liquidity,
                pool_before.liquidity
            );
            assert_eq!(pool_after.current_tick_index, lower_tick);
            assert_ne!(pool_after.sqrt_price, pool_before.sqrt_price);

            let amount_x = balance_of!($app, $token_x_address, $bob);
            let amount_y = balance_of!($app, $token_y_address, $bob);
            assert_eq!(amount_x, 0);
            assert_eq!(amount_y, 990);

            let amount_x = balance_of!($app, $token_x_address, $dex_address);
            let amount_y = balance_of!($app, $token_y_address, $dex_address);
            assert_eq!(amount_x, 1500);
            assert_eq!(amount_y, 1509);

            assert_eq!(
                pool_after.fee_growth_global_x,
                FeeGrowth::new(40000000000000000000000)
            );
            assert_eq!(pool_after.fee_growth_global_y, FeeGrowth::new(0));

            assert_eq!(
                pool_after.fee_protocol_token_x,
                oraiswap_v3_common::math::token_amount::TokenAmount::new(2)
            );
            assert_eq!(
                pool_after.fee_protocol_token_y,
                oraiswap_v3_common::math::token_amount::TokenAmount::new(0)
            );
        }};
    }
    pub(crate) use init_cross_swap;

    macro_rules! get_liquidity_ticks_amount {
        ($app:ident, $dex_address:expr, $pool_key:expr, $lower_tick:expr, $upper_tick:expr) => {{
            $app.query(
                Addr::unchecked($dex_address.as_str()),
                &oraiswap_v3_common::oraiswap_v3_msg::QueryMsg::LiquidityTicksAmount {
                    pool_key: $pool_key.clone(),
                    lower_tick: $lower_tick,
                    upper_tick: $upper_tick,
                },
            )
        }};
    }
    pub(crate) use get_liquidity_ticks_amount;

    macro_rules! get_tickmap {
        ($app:ident, $dex_address:expr, $pool_key:expr, $lower_tick_index:expr, $upper_tick_index:expr, $x_to_y:expr) => {{
            $app.query_tickmaps(
                $dex_address.as_str(),
                &$pool_key,
                $lower_tick_index,
                $upper_tick_index,
                $x_to_y,
            )
        }};
    }
    pub(crate) use get_tickmap;

    macro_rules! get_liquidity_ticks {
        ($app:ident, $dex_address:expr, $pool_key:expr, $tick_indexes:expr) => {{
            $app.get_liquidity_ticks($dex_address.as_str(), $pool_key, $tick_indexes)
        }};
    }
    pub(crate) use get_liquidity_ticks;

    macro_rules! liquidity_tick_equals {
        ($a:expr, $b:expr) => {{
            assert_eq!($a.index, $b.index);
            assert_eq!($a.liquidity_change, $b.liquidity_change);
            assert_eq!($a.sign, $b.sign);
        }};
    }
    pub(crate) use liquidity_tick_equals;

    macro_rules! get_position_ticks {
        ($app:ident, $dex_address:expr, $owner:expr, $offset:expr) => {{
            $app.query(
                Addr::unchecked($dex_address.as_str()),
                &oraiswap_v3_common::oraiswap_v3_msg::QueryMsg::PositionTicks {
                    owner: $owner,
                    offset: $offset,
                },
            )
        }};
    }
    pub(crate) use get_position_ticks;

    macro_rules! position_tick_equals {
        ($a:expr, $b:expr) => {{
            assert_eq!($a.index, $b.index);
            assert_eq!($a.fee_growth_outside_x, $b.fee_growth_outside_x);
            assert_eq!($a.fee_growth_outside_y, $b.fee_growth_outside_y);
            assert_eq!($a.seconds_outside, $b.seconds_outside);
        }};
    }
    pub(crate) use position_tick_equals;

    macro_rules! get_pools {
        ($app:ident, $dex_address:expr, $size:expr, $offset:expr) => {{
            $app.get_pools($dex_address.as_str(), $size, $offset)
                .unwrap()
        }};
    }
    pub(crate) use get_pools;

    macro_rules! get_all_positions {
        ($app:ident, $dex_address:expr, $caller:tt) => {{
            $app.get_all_positions($dex_address.as_str(), $caller)
                .unwrap()
        }};
    }
    pub(crate) use get_all_positions;

    macro_rules! transfer_position {
        ($app:ident, $dex_address:expr, $index:expr, $receiver:expr, $caller:tt) => {{
            $app.transfer_position(
                $caller,
                $dex_address.as_str(),
                $index,
                &$receiver.to_string(),
            )
        }};
        ($app:ident, $dex_address:expr, $index:expr, $receiver:tt, $caller:tt) => {{
            $app.transfer_position($caller, $dex_address.as_str(), $index, $receiver)
        }};
    }
    pub(crate) use transfer_position;

    macro_rules! multiple_swap {
        ($app:ident, $x_to_y:expr,$owner:tt,$bob:tt) => {{
            use decimal::*;
            let (dex, token_x, token_y) = init_dex_and_tokens!($app, $owner);

            let fee_tier = oraiswap_v3_common::storage::FeeTier {
                fee: oraiswap_v3_common::math::percentage::Percentage::from_scale(1, 3),
                tick_spacing: 1,
            };

            add_fee_tier!($app, dex, fee_tier, $owner).unwrap();

            let init_tick = 0;
            let init_sqrt_price =
                oraiswap_v3_common::math::sqrt_price::calculate_sqrt_price(init_tick).unwrap();
            create_pool!(
                $app,
                dex,
                token_x,
                token_y,
                fee_tier,
                init_sqrt_price,
                init_tick,
                $owner
            )
            .unwrap();

            let mint_amount = 10u128.pow(10);
            approve!($app, token_x, dex, mint_amount, $owner).unwrap();
            approve!($app, token_y, dex, mint_amount, $owner).unwrap();

            let pool_key = oraiswap_v3_common::storage::PoolKey::new(
                token_x.to_string(),
                token_y.to_string(),
                fee_tier,
            )
            .unwrap();
            let upper_tick = 953;
            let lower_tick = -upper_tick;

            let amount = 100;
            let pool_data = get_pool!($app, dex, token_x, token_y, fee_tier).unwrap();
            let result = oraiswap_v3_common::logic::math::get_liquidity(
                oraiswap_v3_common::math::token_amount::TokenAmount(amount),
                oraiswap_v3_common::math::token_amount::TokenAmount(amount),
                lower_tick,
                upper_tick,
                pool_data.sqrt_price,
                true,
            )
            .unwrap();
            let _amount_x = result.x;
            let _amount_y = result.y;
            let liquidity_delta = result.l;
            let slippage_limit_lower = pool_data.sqrt_price;
            let slippage_limit_upper = pool_data.sqrt_price;

            create_position!(
                $app,
                dex,
                pool_key,
                lower_tick,
                upper_tick,
                liquidity_delta,
                slippage_limit_lower,
                slippage_limit_upper,
                $owner
            )
            .unwrap();

            if $x_to_y {
                mint!($app, token_x, $bob, amount, $owner).unwrap();
                let amount_x = balance_of!($app, token_x, $bob);
                assert_eq!(amount_x, amount);
                approve!($app, token_x, dex, amount, $bob).unwrap();
            } else {
                mint!($app, token_y, $bob, amount, $owner).unwrap();
                let amount_y = balance_of!($app, token_y, $bob);
                assert_eq!(amount_y, amount);
                approve!($app, token_y, dex, amount, $bob).unwrap();
            }

            let swap_amount = oraiswap_v3_common::math::token_amount::TokenAmount(10);
            for _ in 1..=10 {
                swap_exact_limit!($app, dex, pool_key, $x_to_y, swap_amount, true, $bob);
            }

            let pool = get_pool!($app, dex, token_x, token_y, fee_tier).unwrap();
            if $x_to_y {
                assert_eq!(pool.current_tick_index, -821);
            } else {
                assert_eq!(pool.current_tick_index, 820);
            }
            assert_eq!(
                pool.fee_growth_global_x,
                oraiswap_v3_common::math::fee_growth::FeeGrowth::new(0)
            );
            assert_eq!(
                pool.fee_growth_global_y,
                oraiswap_v3_common::math::fee_growth::FeeGrowth::new(0)
            );
            if $x_to_y {
                assert_eq!(
                    pool.fee_protocol_token_x,
                    oraiswap_v3_common::math::token_amount::TokenAmount(10)
                );
                assert_eq!(
                    pool.fee_protocol_token_y,
                    oraiswap_v3_common::math::token_amount::TokenAmount(0)
                );
            } else {
                assert_eq!(
                    pool.fee_protocol_token_x,
                    oraiswap_v3_common::math::token_amount::TokenAmount(0)
                );
                assert_eq!(
                    pool.fee_protocol_token_y,
                    oraiswap_v3_common::math::token_amount::TokenAmount(10)
                );
            }
            assert_eq!(pool.liquidity, liquidity_delta);
            if $x_to_y {
                assert_eq!(
                    pool.sqrt_price,
                    oraiswap_v3_common::math::sqrt_price::SqrtPrice::new(959805958620596146276151)
                );
            } else {
                assert_eq!(
                    pool.sqrt_price,
                    oraiswap_v3_common::math::sqrt_price::SqrtPrice::new(1041877257604411525269920)
                );
            }

            let dex_amount_x = balance_of!($app, token_x, dex);
            let dex_amount_y = balance_of!($app, token_y, dex);
            if $x_to_y {
                assert_eq!(dex_amount_x, 200);
                assert_eq!(dex_amount_y, 20);
            } else {
                assert_eq!(dex_amount_x, 20);
                assert_eq!(dex_amount_y, 200);
            }

            let user_amount_x = balance_of!($app, token_x, $bob);
            let user_amount_y = balance_of!($app, token_y, $bob);
            if $x_to_y {
                assert_eq!(user_amount_x, 0);
                assert_eq!(user_amount_y, 80);
            } else {
                assert_eq!(user_amount_x, 80);
                assert_eq!(user_amount_y, 0);
            }
        }};
    }
    pub(crate) use multiple_swap;

    macro_rules! big_deposit_and_swap {
        ($app:ident, $x_to_y:expr,$owner:tt) => {{
            let (dex, token_x, token_y) =
                init_dex_and_tokens!($app, u128::MAX, Percentage::from_scale(1, 2), $owner);

            let mint_amount = 2u128.pow(75) - 1;

            approve!($app, token_x, dex, u128::MAX, $owner).unwrap();
            approve!($app, token_y, dex, u128::MAX, $owner).unwrap();

            let fee_tier = FeeTier {
                fee: Percentage::from_scale(6, 3),
                tick_spacing: 1,
            };
            add_fee_tier!($app, dex, fee_tier, $owner).unwrap();

            let init_tick = 0;
            let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
            create_pool!(
                $app,
                dex,
                token_x,
                token_y,
                fee_tier,
                init_sqrt_price,
                init_tick,
                $owner
            )
            .unwrap();

            let lower_tick = if $x_to_y {
                -(fee_tier.tick_spacing as i32)
            } else {
                0
            };
            let upper_tick = if $x_to_y {
                0
            } else {
                fee_tier.tick_spacing as i32
            };
            let pool = get_pool!($app, dex, token_x, token_y, fee_tier).unwrap();

            let liquidity_delta = if $x_to_y {
                get_liquidity_by_y(
                    TokenAmount(mint_amount),
                    lower_tick,
                    upper_tick,
                    pool.sqrt_price,
                    true,
                )
                .unwrap()
                .l
            } else {
                get_liquidity_by_x(
                    TokenAmount(mint_amount),
                    lower_tick,
                    upper_tick,
                    pool.sqrt_price,
                    true,
                )
                .unwrap()
                .l
            };

            let pool_key =
                PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
            let slippage_limit_lower = pool.sqrt_price;
            let slippage_limit_upper = pool.sqrt_price;
            create_position!(
                $app,
                dex,
                pool_key,
                lower_tick,
                upper_tick,
                liquidity_delta,
                slippage_limit_lower,
                slippage_limit_upper,
                $owner
            )
            .unwrap();

            let amount_x = balance_of!($app, token_x, $owner);
            let amount_y = balance_of!($app, token_y, $owner);
            if $x_to_y {
                assert_eq!(amount_x, 340282366920938463463374607431768211455);
                assert_eq!(amount_y, 340282366920938425684442744474606501888);
            } else {
                assert_eq!(amount_x, 340282366920938425684442744474606501888);
                assert_eq!(amount_y, 340282366920938463463374607431768211455);
            }

            let sqrt_price_limit = if $x_to_y {
                SqrtPrice::new(MIN_SQRT_PRICE)
            } else {
                SqrtPrice::new(MAX_SQRT_PRICE)
            };

            swap!(
                $app,
                dex,
                pool_key,
                $x_to_y,
                TokenAmount(mint_amount),
                true,
                sqrt_price_limit,
                $owner
            )
            .unwrap();

            let amount_x = balance_of!($app, token_x, $owner);
            let amount_y = balance_of!($app, token_y, $owner);
            if $x_to_y {
                assert_eq!(amount_x, 340282366920938425684442744474606501888);
                assert_ne!(amount_y, 0);
            } else {
                assert_ne!(amount_x, 0);
                assert_eq!(amount_y, 340282366920938425684442744474606501888);
            }
        }};
    }
    pub(crate) use big_deposit_and_swap;

    macro_rules! positions_equals {
        ($a:expr, $b:expr) => {{
            assert_eq!($a.fee_growth_inside_x, $b.fee_growth_inside_x);
            assert_eq!($a.fee_growth_inside_y, $b.fee_growth_inside_y);
            assert_eq!($a.liquidity, $b.liquidity);
            assert_eq!($a.lower_tick_index, $b.lower_tick_index);
            assert_eq!($a.upper_tick_index, $b.upper_tick_index);
            assert_eq!($a.pool_key, $b.pool_key);
            assert_eq!($a.tokens_owed_x, $b.tokens_owed_x);
            assert_eq!($a.tokens_owed_y, $b.tokens_owed_y);
        }};
    }
    pub(crate) use positions_equals;
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{coins, Addr, Coin, Uint128};

    use crate::tests::helper::FEE_DENOM;

    use super::MockApp;

    #[test]
    fn token_balance_querier() {
        let (mut app, accounts) = MockApp::new(&[
            ("owner", &coins(100_000_000_000, FEE_DENOM)),
            ("receiver", &[]),
        ]);
        let owner = &accounts[0];
        let receiver = &accounts[1];

        app.set_token_balances(owner, &[(&"AIRI".to_string(), &[(receiver, 123u128)])])
            .unwrap();

        assert_eq!(
            Uint128::from(123u128),
            app.query_token_balance(app.get_token_addr("AIRI").unwrap().as_str(), receiver,)
                .unwrap()
        );
    }

    #[test]
    fn balance_querier() {
        let (app, accounts) = MockApp::new(&[(
            "account",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            }],
        )]);

        assert_eq!(
            app.query_balance(Addr::unchecked(&accounts[0]), "uusd".to_string())
                .unwrap(),
            Uint128::from(200u128)
        );
    }

    #[test]
    fn all_balances_querier() {
        let (app, accounts) = MockApp::new(&[(
            "account",
            &[
                Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(200u128),
                },
                Coin {
                    denom: "ukrw".to_string(),
                    amount: Uint128::from(300u128),
                },
            ],
        )]);

        let mut balance1 = app
            .query_all_balances(Addr::unchecked(&accounts[0]))
            .unwrap();
        balance1.sort_by(|a, b| a.denom.cmp(&b.denom));
        let mut balance2 = vec![
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            },
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128::from(300u128),
            },
        ];
        balance2.sort_by(|a, b| a.denom.cmp(&b.denom));
        assert_eq!(balance1, balance2);
    }

    #[test]
    fn supply_querier() {
        let (mut app, accounts) = MockApp::new(&[
            ("owner", &coins(100_000_000_000, FEE_DENOM)),
            ("addr00000", &[]),
            ("addr00001", &[]),
            ("addr00002", &[]),
            ("addr00003", &[]),
        ]);
        let owner = &accounts[0];
        app.set_token_balances(
            owner,
            &[(
                &"LPA".to_string(),
                &[
                    (&accounts[1], 123u128),
                    (&accounts[2], 123u128),
                    (&accounts[3], 123u128),
                    (&accounts[4], 123u128),
                ],
            )],
        )
        .unwrap();

        assert_eq!(
            app.query_token_info(app.get_token_addr("LPA").unwrap())
                .unwrap()
                .total_supply,
            Uint128::from(492u128)
        )
    }
}
