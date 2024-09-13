use cosmwasm_std::{Addr, Coin, Decimal as StdDecimal, StdResult};
use cosmwasm_testing_util::{ContractWrapper, ExecuteResponse, MockResult};

use decimal::Decimal;
use derive_more::{Deref, DerefMut};

use oraiswap_v3::state::MAX_LIMIT;
use oraiswap_v3_common::{
    asset::{Asset, AssetInfo},
    math::{
        liquidity::Liquidity, percentage::Percentage, sqrt_price::SqrtPrice,
        token_amount::TokenAmount,
    },
    oraiswap_v3_msg,
    storage::{FeeTier, Pool, PoolKey, Position},
};

use crate::{
    msg::{self, Route},
    Config, ProtocolFee,
};

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
    zapper_id: u64,
}

#[allow(dead_code)]
impl MockApp {
    pub fn new(init_balances: &[(&str, &[Coin])]) -> (Self, Vec<String>) {
        let (mut app, accounts) = TestMockApp::new(init_balances);

        let zapper_id;

        #[cfg(not(feature = "test-tube"))]
        {
            zapper_id = app.upload(Box::new(
                cosmwasm_testing_util::ContractWrapper::new_with_empty(
                    crate::contract::execute,
                    crate::contract::instantiate,
                    crate::contract::query,
                )
                .with_reply_empty(crate::contract::reply),
            ));
        }
        #[cfg(feature = "test-tube")]
        {
            zapper_id = app.upload(include_bytes!("testdata/zapper.wasm"));
        }

        (Self { app, zapper_id }, accounts)
    }

    fn init_dex_and_router(&mut self, owner: &str) -> MockResult<(Addr, Addr)> {
        let dex_v3_id;
        let incentive_id;
        let mixed_router_id;
        let v2_factory_id;
        let oracle_id;
        #[cfg(not(feature = "test-tube"))]
        {
            dex_v3_id = self.app.upload(Box::new(ContractWrapper::new_with_empty(
                oraiswap_v3::contract::execute,
                oraiswap_v3::contract::instantiate,
                oraiswap_v3::contract::query,
            )));
            incentive_id = self.app.upload(Box::new(ContractWrapper::new_with_empty(
                incentives_fund_manager::contract::execute,
                incentives_fund_manager::contract::instantiate,
                incentives_fund_manager::contract::query,
            )));
            mixed_router_id = self.app.upload(Box::new(ContractWrapper::new_with_empty(
                oraiswap_mixed_router::contract::execute,
                oraiswap_mixed_router::contract::instantiate,
                oraiswap_mixed_router::contract::query,
            )));
            v2_factory_id = self.app.upload(Box::new(ContractWrapper::new_with_empty(
                oraiswap_factory::contract::execute,
                oraiswap_factory::contract::instantiate,
                oraiswap_factory::contract::query,
            )));
            oracle_id = self.app.upload(Box::new(ContractWrapper::new_with_empty(
                oraiswap_oracle::contract::execute,
                oraiswap_oracle::contract::instantiate,
                oraiswap_oracle::contract::query,
            )));
        }
        #[cfg(feature = "test-tube")]
        {
            dex_v3_id = self.app.upload(include_bytes!(
                "../../../oraiswap-v3/src/tests/testdata/oraiswap-v3.wasm"
            ));
            incentive_id = self.app.upload(include_bytes!(
                "../../../oraiswap-v3/src/tests/testdata/incentives-fund-manager.wasm"
            ));
            mixed_router_id = self
                .app
                .upload(include_bytes!("testdata/oraiswap-mixed-router.wasm"));
            v2_factory_id = self
                .app
                .upload(include_bytes!("testdata/oraiswap-factory.wasm"));
            oracle_id = self
                .app
                .upload(include_bytes!("testdata/oraiswap-oracle.wasm"));
        }

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

        let dex_v3_addr = self.instantiate(
            dex_v3_id,
            Addr::unchecked(owner),
            &oraiswap_v3_msg::InstantiateMsg {
                protocol_fee: Percentage::new(0),
                incentives_fund_manager: incentive_addr.clone(),
            },
            &[],
            "oraiswap_v3",
        )?;

        let oracle_addr = self.instantiate(
            oracle_id,
            Addr::unchecked(owner),
            &oraiswap::oracle::InstantiateMsg {
                name: None,
                version: None,
                admin: None,
                min_rate: None,
                max_rate: None,
            },
            &[],
            "oraiswap_v3",
        )?;

        let v2_factory_addr = self.instantiate(
            v2_factory_id,
            Addr::unchecked(owner),
            &oraiswap::factory::InstantiateMsg {
                pair_code_id: 1,
                token_code_id: 1,
                oracle_addr,
                commission_rate: None,
            },
            &[],
            "oraiswap_v3",
        )?;

        let mixed_router_addr = self.instantiate(
            mixed_router_id,
            Addr::unchecked(owner),
            &oraiswap::mixed_router::InstantiateMsg {
                factory_addr: v2_factory_addr.clone(),
                factory_addr_v2: v2_factory_addr,
                oraiswap_v3: dex_v3_addr.clone(),
            },
            &[],
            "oraiswap_v3",
        )?;

        Ok((dex_v3_addr, mixed_router_addr))
        // upload incentive
    }

    pub fn create_zapper(&mut self, admin: &str) -> MockResult<Addr> {
        let code_id = self.zapper_id;
        let (dex_v3_addr, mixed_router_addr) = self.init_dex_and_router(admin)?;

        self.instantiate(
            code_id,
            Addr::unchecked(admin),
            &msg::InstantiateMsg {
                admin: Addr::unchecked(admin),
                mixed_router: mixed_router_addr,
                dex_v3: dex_v3_addr,
            },
            &[],
            "zapper",
        )
    }

    pub fn register_protocol_fee(
        &mut self,
        sender: &str,
        zapper: &str,
        percent: StdDecimal,
        fee_receiver: &str,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(zapper),
            &msg::ExecuteMsg::RegisterProtocolFee {
                percent,
                fee_receiver: Addr::unchecked(fee_receiver.to_string()),
            },
            &[],
        )
    }

    pub fn get_zapper_config(&mut self, zapper: &str) -> StdResult<Config> {
        self.query(Addr::unchecked(zapper), &msg::QueryMsg::Config {})
    }

    pub fn zap_in_liquidity(
        &mut self,
        sender: &str,
        zapper: &str,
        pool_key: PoolKey,
        tick_lower_index: i32,
        tick_upper_index: i32,
        asset_in: &Asset,
        routes: Vec<Route>,
        minimum_liquidity: Option<Liquidity>,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(zapper),
            &msg::ExecuteMsg::ZapInLiquidity {
                pool_key,
                tick_lower_index,
                tick_upper_index,
                asset_in: asset_in.to_owned(),
                routes,
                minimum_liquidity,
            },
            &[],
        )
    }

    pub fn zap_out_liquidity(
        &mut self,
        sender: &str,
        zapper: &str,
        position_index: u32,
        routes: Vec<Route>,
    ) -> MockResult<ExecuteResponse> {
        self.execute(
            Addr::unchecked(sender),
            Addr::unchecked(zapper),
            &msg::ExecuteMsg::ZapOutLiquidity {
                position_index,
                routes,
            },
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

    pub fn get_protocol_fee(&self, zapper: &str) -> StdResult<ProtocolFee> {
        self.query(Addr::unchecked(zapper), &msg::QueryMsg::ProtocolFee {})
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

    pub fn get_incentives_fund_manager(&mut self, dex: &str) -> StdResult<Addr> {
        self.query(
            Addr::unchecked(dex),
            &oraiswap_v3_msg::QueryMsg::IncentivesFundManager {},
        )
    }
}

pub mod macros {
    macro_rules! create_zapper {
        ($app:ident, $admin:expr) => {{
                $app.create_zapper($admin).unwrap()
            }};
    }
    pub(crate) use create_zapper;

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

    macro_rules! approve {
        ($app:ident, $token_address:expr, $spender:expr, $value:expr, $caller:tt) => {{
            $app.approve_token($token_address.as_str(), $caller, $spender.as_str(), $value)
        }};
    }
    pub(crate) use approve;

    macro_rules! mint {
        ($app:ident, $token_address:expr, $to:tt, $value:expr, $caller:tt) => {{
            $app.mint_token($caller, $to, $token_address.as_str(), $value)
        }};
    }

    pub(crate) use mint;

    macro_rules! add_fee_tier {
        ($app:ident, $dex_address:expr, $fee_tier:expr, $caller:tt) => {{
            $app.add_fee_tier($caller, $dex_address.as_str(), $fee_tier)
        }};
    }
    pub(crate) use add_fee_tier;

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

    macro_rules! get_all_positions {
        ($app:ident, $dex_address:expr, $caller:tt) => {{
            $app.get_all_positions($dex_address.as_str(), $caller)
                .unwrap()
        }};
    }
    pub(crate) use get_all_positions;

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
