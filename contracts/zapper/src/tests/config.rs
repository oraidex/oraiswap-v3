use cosmwasm_std::{coins, Addr, Decimal};
use oraiswap_v3_common::error::ContractError;

use crate::msg::{ExecuteMsg, QueryMsg};
use crate::tests::helper::MockApp;
use crate::tests::helper::{macros::*, FEE_DENOM};
use crate::{Config, ProtocolFee};

#[test]
fn test_change_config_admin() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];

    let zapper = create_zapper!(app, alice);

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

#[test]
fn test_update_protocol_fee() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];

    let zapper = create_zapper!(app, alice);

    let query_msg = QueryMsg::Config {};
    let config: Config = app.query(zapper.clone(), &query_msg).unwrap();
    assert_eq!(config.admin.as_str(), alice);

    // register failed, unauthorized
    let err = app
        .register_protocol_fee(
            &bob,
            zapper.as_str(),
            Decimal::from_ratio(1u128, 10u128),
            &bob,
        )
        .unwrap_err();
    assert!(err
        .root_cause()
        .to_string()
        .contains(&ContractError::Unauthorized {}.to_string()));

    // register failed, fee > 1%
    let err = app
        .register_protocol_fee(
            &alice,
            zapper.as_str(),
            Decimal::from_ratio(11u128, 10u128),
            &bob,
        )
        .unwrap_err();
    assert!(err
        .root_cause()
        .to_string()
        .contains(&ContractError::InvalidFee {}.to_string()));

    // register success
    app.register_protocol_fee(
        &alice,
        zapper.as_str(),
        Decimal::from_ratio(1u128, 10u128),
        &bob,
    )
    .unwrap();
    // query protocol fee
    let protocol_fee = app.get_protocol_fee(zapper.as_str()).unwrap();
    assert_eq!(
        protocol_fee,
        ProtocolFee {
            percent: Decimal::from_ratio(1u128, 10u128),
            fee_receiver: Addr::unchecked(bob)
        }
    )
}
