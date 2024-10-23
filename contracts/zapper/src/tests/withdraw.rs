use cosmwasm_std::{coins, Uint128};
use oraiswap_v3_common::asset::{Asset, AssetInfo};

use crate::tests::helper::MockApp;
use crate::tests::helper::{macros::*, FEE_DENOM};

#[test]
fn test_withdraw() {
    let (mut app, accounts) = MockApp::new(&[
        ("alice", &coins(100_000_000_000, FEE_DENOM)),
        ("bob", &coins(100_000_000_000, FEE_DENOM)),
    ]);
    let alice = &accounts[0];
    let bob = &accounts[1];
    let initial_amount = 10u128.pow(20);
    let (token_x, _, _) =
        create_3_tokens!(app, initial_amount, initial_amount, initial_amount, alice);

    let zapper = create_zapper!(app, alice);

    app.mint_token(alice, zapper.as_str(), token_x.as_str(), 1)
        .unwrap();
    let assets: Vec<Asset> = vec![Asset {
        info: AssetInfo::Token {
            contract_addr: token_x.clone(),
        },
        amount: Uint128::new(1),
    }];
    let err = app
        .withdraw(bob, zapper.as_str(), assets.clone(), Some(bob))
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Unauthorized"));

    app.withdraw(alice, zapper.as_str(), assets, Some(bob))
        .unwrap();
    let balance = balance_of!(app, token_x, bob);
    assert_eq!(balance, 1);
}
