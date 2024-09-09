use cosmwasm_std::{Addr, Coin};
use cosmwasm_testing_util::{ContractWrapper, MockResult};

use decimal::Decimal;
use derive_more::{Deref, DerefMut};

use oraiswap_v3_common::{math::percentage::Percentage, oraiswap_v3_msg};

use crate::msg;

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
                ),
            ));
        }
        #[cfg(feature = "test-tube")]
        {
            zapper_id = app.upload(include_bytes!("../../artifacts/zapper.wasm"));
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
                "../../../oraiswap-v3/artifacts/oraiswap-v3.wasm"
            ));
            incentive_id = self.app.upload(include_bytes!(
                "../../../incentives-fund-manager/artifacts/incentives-fund-manager.wasm"
            ));
            mixed_router_id = self
                .app
                .upload(include_bytes!("./testdata/oraiswap-mixed-router.wasm"));
            v2_factory_id = self
                .app
                .upload(include_bytes!("./testdata/oraiswap-factory.wasm"));
            oracle_id = self
                .app
                .upload(include_bytes!("./testdata/oraiswap-oracle.wasm"));
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
}

pub mod macros {
    macro_rules! create_zapper {
        ($app:ident, $admin:expr) => {{
                $app.create_zapper($admin).unwrap()
            }};
    }
    pub(crate) use create_zapper;
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
