use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub mixed_router: Addr,
    pub dex_v3: Addr,
}

#[cw_serde]
pub struct ProtocolFee {
    pub percent: Decimal,
    pub fee_receiver: Addr,
}
