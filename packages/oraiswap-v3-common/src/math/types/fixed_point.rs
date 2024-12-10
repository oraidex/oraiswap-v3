use cosmwasm_schema::cw_serde;
use decimal::*;

#[decimal(24)]
#[cw_serde]
#[derive(Default, Eq, Copy)]
pub struct FixedPoint(#[schemars(with = "String")] pub U256);
