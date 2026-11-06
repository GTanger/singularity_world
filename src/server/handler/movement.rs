//! 移動／探索／離場觀測清除。六角與正方格雙路徑。

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use parking_lot::RwLock;
use rand::Rng;

use crate::db;
use crate::event::{self, types::BLOCKED};
use crate::game;
use crate::game::TimePeriod;
use crate::gametext;
use crate::grid::{Freshness, SquareCoord, Terrain};

use super::super::protocol::{ActionResultMsg, BlockedMsg, ClientMsg, ExploreLogMsg, LootItem, MovedMsg, NpcAddressMsg, NpcDialogueMsg, StatDelta};
use super::{WsConnection, current_game_hour};
use super::super::protocol::GridCellView;

include!("movement/step_move.rs");
include!("movement/room_move.rs");
include!("movement/presence_and_climate.rs");
include!("movement/grid_view_1.rs");
include!("movement/grid_view_2.rs");
include!("movement/scene_desc.rs");
include!("movement/route_reentry.rs");

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;

    include!("movement/tests_1.rs");
    include!("movement/tests_2.rs");
}
