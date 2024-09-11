use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use oraiswap_v3_common::storage::Position;

use crate::{Config, IncentiveBalance, PairBalance, PendingPosition, ProtocolFee, ZapOutRoutes};

pub const CONFIG: Item<Config> = Item::new("config");

pub const SNAP_BALANCE: Item<PairBalance> = Item::new("snap_balance");

pub const SNAP_INCENTIVE: Item<IncentiveBalance> = Item::new("snap_incentive");

pub const PENDING_POSITION: Item<PendingPosition> = Item::new("pending_position");

pub const ZAP_OUT_POSITION: Item<Position> = Item::new("zap_out_position");

pub const ZAP_OUT_ROUTES: Item<ZapOutRoutes> = Item::new("zap_out_routes");

pub const RECEIVER: Item<Addr> = Item::new("receiver");

pub const PROTOCOL_FEE: Item<ProtocolFee> = Item::new("protocol_fee");
