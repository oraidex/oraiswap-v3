use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::{Config, PairBalance, PendingPosition};

pub const CONFIG: Item<Config> = Item::new("config");

pub const SNAP_BALANCE: Item<PairBalance> = Item::new("snap_balance");

pub const PENDING_POSITION: Item<PendingPosition> = Item::new("pending_position");

pub const RECEIVER: Item<Addr> = Item::new("receiver");
