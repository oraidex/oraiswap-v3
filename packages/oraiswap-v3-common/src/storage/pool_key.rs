use core::fmt;
use cw_storage_plus::KeyDeserialize;

use cosmwasm_schema::cw_serde;
use cosmwasm_storage::to_length_prefixed_nested;
use crate::error::ContractError;
use crate::storage::FeeTier;

#[cw_serde]
#[derive(Default, Eq)]
pub struct PoolKey {
    pub token_x: String,
    pub token_y: String,
    pub fee_tier: FeeTier,
}

impl PoolKey {
    pub fn key(&self) -> Vec<u8> {
        // sort by asc then append fee_tier at the end to create unique key
        to_length_prefixed_nested(&[
            self.token_x.as_bytes(),
            self.token_y.as_bytes(),
            &self.fee_tier.key(),
        ])
    }

    pub fn from_bytes(raw_key: &[u8]) -> Result<Self, ContractError> {
        // first 2 bytes is length, then next is bytes
        let mut index = 0;
        let mut namespaces = vec![];
        while index < raw_key.len() {
            let item_len = u16::from_be_bytes([raw_key[index], raw_key[index + 1]]) as usize;
            index += 2;
            namespaces.push(&raw_key[index..index + item_len]);
            index += item_len;
        }

        // size must be 3
        if namespaces.len() != 3 {
            return Err(ContractError::InvalidSize);
        }

        Ok(Self {
            token_x: String::from_slice(namespaces[0])?,
            token_y: String::from_slice(namespaces[1])?,
            fee_tier: FeeTier::from_bytes(namespaces[2])?,
        })
    }

    pub fn new(token_0: String, token_1: String, fee_tier: FeeTier) -> Result<Self, ContractError> {
        if token_0 == token_1 {
            return Err(ContractError::TokensAreSame);
        }

        if token_0 < token_1 {
            Ok(Self {
                token_x: token_0.to_string(),
                token_y: token_1.to_string(),
                fee_tier,
            })
        } else {
            Ok(Self {
                token_x: token_1.to_string(),
                token_y: token_0.to_string(),
                fee_tier,
            })
        }
    }
}

impl fmt::Display for PoolKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // tokenx-tokeny-fee-tick_spacing
        write!(
            f,
            "{}-{}-{}-{}",
            self.token_x, self.token_y, self.fee_tier.fee.0, self.fee_tier.tick_spacing
        )
    }
}
#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;
    use cosmwasm_storage::to_length_prefixed_nested;
    use decimal::Decimal;

    use crate::{math::percentage::Percentage, storage::{FeeTier, PoolKey}};

    #[test]
    fn test_key() {
        let token_x = Addr::unchecked("token_0").to_string().to_string();
        let token_y = Addr::unchecked("token_1").to_string().to_string();
        let fee_tier = FeeTier {
            fee: Percentage::new(10),
            tick_spacing: 1,
        };

        let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

        assert_eq!(
            pool_key.key(),
            to_length_prefixed_nested(&[b"token_0", b"token_1", fee_tier.key().as_slice()])
        );

        println!("{:?}", pool_key.to_string());
    }
}
