use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Storage, Uint128};
use oraiswap_v3_common::{
    asset::{Asset, AssetInfo},
    error::ContractError,
};

use crate::state::SNAP_BALANCE;

#[cw_serde]
pub struct PairBalance {
    pub token_x: Asset,
    pub token_y: Asset,
}

impl PairBalance {
    pub fn new(token_x: Asset, token_y: Asset) -> Self {
        Self { token_x, token_y }
    }

    pub fn save(
        storage: &mut dyn Storage,
        token_x: &AssetInfo,
        balance_x: Uint128,
        token_y: &AssetInfo,
        balance_y: Uint128,
    ) -> Result<(), ContractError> {
        let pair_balance = Self::new(
            Asset::new(token_x.clone(), balance_x),
            Asset::new(token_y.clone(), balance_y),
        );
        Ok(SNAP_BALANCE.save(storage, &pair_balance)?)
    }
}
