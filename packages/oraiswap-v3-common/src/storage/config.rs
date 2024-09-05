use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::{math::types::percentage::Percentage, storage::FeeTier};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub fee_tiers: Vec<FeeTier>,
    pub protocol_fee: Percentage,
    pub incentives_fund_manager: Addr,
}
