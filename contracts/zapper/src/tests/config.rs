use cosmwasm_std::{coins, Addr};

use crate::msg::{ExecuteMsg, QueryMsg};
use crate::tests::helper::MockApp;
use crate::tests::helper::{macros::*, FEE_DENOM};
use crate::Config;

#[test]
fn test_change_config_admin() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];

    let zapper = create_zapper!(app, alice, "mixed_router", "dex_v3");

    let query_msg = QueryMsg::Config {};
    let config: Config = app.query(zapper.clone(), &query_msg).unwrap();
    assert_eq!(config.admin.as_str(), alice);

    let new_admin = Addr::unchecked(bob);
    let execute_msg = ExecuteMsg::UpdateConfig {
        admin: Some(new_admin.clone()),
        mixed_router: None,
        dex_v3: None,
    };

    let result = app.execute(
        Addr::unchecked(alice),
        Addr::unchecked(zapper.clone()),
        &execute_msg,
        &[],
    );
    assert!(result.is_ok());

    let config: Config = app.query(zapper.clone(), &query_msg).unwrap();
    assert_eq!(config.admin.as_str(), bob);
}
