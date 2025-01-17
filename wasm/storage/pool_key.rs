use crate::alloc::string::ToString;
use crate::errors::SwapError;
use crate::is_token_x;
use crate::FeeTier;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct PoolKey {
    #[tsify(type = "string")]
    pub token_x: String,
    #[tsify(type = "string")]
    pub token_y: String,
    pub fee_tier: FeeTier,
}

impl PoolKey {
    pub fn new(token_0: String, token_1: String, fee_tier: FeeTier) -> Result<Self, SwapError> {
        if token_0 == token_1 {
            return Err(SwapError::TokensAreSame);
        }

        Ok(if is_token_x(token_0.clone(), token_1.clone()).unwrap() {
            PoolKey {
                token_x: token_0,
                token_y: token_1,
                fee_tier,
            }
        } else {
            PoolKey {
                token_x: token_1,
                token_y: token_0,
                fee_tier,
            }
        })
    }
}

#[wasm_bindgen(js_name = newPoolKey)]
pub fn new_pool_key(
    token_0: String,
    token_1: String,
    fee_tier: FeeTier,
) -> Result<PoolKey, SwapError> {
    PoolKey::new(token_0, token_1, fee_tier)
}
