use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use oraiswap_v3_common::storage::Position;

use crate::{msg::Route, Config, IncentiveBalance, PairBalance, PendingPosition, ProtocolFee};

pub const CONFIG: Item<Config> = Item::new("config");

pub const SNAP_BALANCE: Item<PairBalance> = Item::new("snap_balance");

pub const SNAP_INCENTIVE: Item<IncentiveBalance> = Item::new("snap_incentive");

pub const PENDING_POSITION: Item<PendingPosition> = Item::new("pending_position");

pub const ZAP_OUT_POSITION: Item<Position> = Item::new("zap_out_position");

pub const ZAP_OUT_ROUTES: Item<Vec<Route>> = Item::new("zap_out_routes");

pub const RECEIVER: Item<Addr> = Item::new("receiver");

pub const PROTOCOL_FEE: Item<ProtocolFee> = Item::new("protocol_fee");

pub const SNAP_BALANCES: Map<String, Uint128> = Map::new("snap_balances");
