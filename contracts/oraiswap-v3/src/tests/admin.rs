use crate::tests::helper::MockApp;
use crate::tests::helper::{macros::*, FEE_DENOM};
use cosmwasm_std::{coins, Addr};
use decimal::Decimal;
use oraiswap_v3_common::error::ContractError;
use oraiswap_v3_common::math::percentage::Percentage;
use oraiswap_v3_common::oraiswap_v3_msg::{ExecuteMsg, QueryMsg};

#[test]
fn test_change_admin() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let dex = create_dex!(app, Percentage::new(0), alice);

    let query_msg = QueryMsg::Admin {};
    let admin: Addr = app.query(dex.clone(), &query_msg).unwrap();
    assert_eq!(admin.as_str(), alice);

    let execute_msg = ExecuteMsg::ChangeAdmin {
        new_admin: Addr::unchecked(bob),
    };

    let result = app.execute(
        Addr::unchecked(alice),
        Addr::unchecked(dex.clone()),
        &execute_msg,
        &[],
    );
    assert!(result.is_ok());

    let admin: Addr = app.query(dex.clone(), &query_msg).unwrap();
    assert_eq!(admin.as_str(), bob);
}

#[test]
fn test_change_admin_not_admin() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let dex = create_dex!(app, Percentage::new(0), alice);

    let execute_msg = ExecuteMsg::ChangeAdmin {
        new_admin: Addr::unchecked(bob),
    };
    let error = app
        .execute(
            Addr::unchecked(bob),
            Addr::unchecked(dex.clone()),
            &execute_msg,
            &[],
        )
        .unwrap_err();

    assert!(error
        .root_cause()
        .to_string()
        .contains(&ContractError::Unauthorized {}.to_string()));
}
