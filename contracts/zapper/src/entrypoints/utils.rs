use cosmwasm_std::Api;
use oraiswap_v3_common::{asset::AssetInfo, storage::PoolKey};

pub fn get_pool_v3_asset_info(api: &dyn Api, pool_key: &PoolKey) -> (AssetInfo, AssetInfo) {
    (
        AssetInfo::from_denom(api, &pool_key.token_x),
        AssetInfo::from_denom(api, &pool_key.token_y),
    )
}
