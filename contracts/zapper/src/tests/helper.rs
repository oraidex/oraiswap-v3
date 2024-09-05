use cosmwasm_std::{Addr, Coin};
use cosmwasm_testing_util::MockResult;

use derive_more::{Deref, DerefMut};

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
            zapper_id = app.upload(include_bytes!("./testdata/zapper.wasm"));
        }

        (Self { app, zapper_id }, accounts)
    }

    pub fn create_zapper(
        &mut self,
        admin: &str,
        mixed_router: &str,
        dex_v3: &str,
    ) -> MockResult<Addr> {
        let code_id = self.zapper_id;
        self.instantiate(
            code_id,
            Addr::unchecked(admin),
            &msg::InstantiateMsg {
                admin: Addr::unchecked(admin),
                mixed_router: Addr::unchecked(mixed_router),
                dex_v3: Addr::unchecked(dex_v3),
            },
            &[],
            "zapper",
        )
    }
}

pub mod macros {
    macro_rules! create_zapper {
        ($app:ident, $admin:expr, $mixed_router:expr, $dex_v3:expr) => {{
                $app.create_zapper($admin, $mixed_router, $dex_v3).unwrap()
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
