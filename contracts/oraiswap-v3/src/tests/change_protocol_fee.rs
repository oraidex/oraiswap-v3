use crate::msg::{ExecuteMsg, QueryMsg};
use crate::percentage::Percentage;
use crate::tests::helper::MockApp;
use crate::tests::helper::{macros::*, FEE_DENOM};
use crate::ContractError;
use cosmwasm_std::{coins, Addr};
use decimal::Decimal;

#[test]
fn test_change_protocol_fee() {
    let (mut app, accounts) = MockApp::new(&[("alice", &coins(100_000_000_000, FEE_DENOM))]);
    let alice = &accounts[0];

    let dex = create_dex!(app, Percentage::new(0), alice);

    let query_msg = QueryMsg::ProtocolFee {};
    let protocol_fee: Percentage = app.query(dex.clone(), &query_msg).unwrap();
    assert_eq!(protocol_fee, Percentage::new(0));

    let execute_msg = ExecuteMsg::ChangeProtocolFee {
        protocol_fee: Percentage::new(1),
    };
    let result = app.execute(
        Addr::unchecked(alice),
        Addr::unchecked(dex.clone()),
        &execute_msg,
        &[],
    );
    assert!(result.is_ok());

    let protocol_fee: Percentage = app.query(dex.clone(), &query_msg).unwrap();
    assert_eq!(protocol_fee, Percentage::new(1));
}

#[test]
fn test_change_protocol_fee_not_admin() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let dex = create_dex!(app, Percentage::new(0), alice);

    let execute_msg = ExecuteMsg::ChangeProtocolFee {
        protocol_fee: Percentage::new(1),
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
