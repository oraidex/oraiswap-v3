use decimal::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[decimal(6)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, scale::Decode, scale::Encode, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct Liquidity(pub u128);