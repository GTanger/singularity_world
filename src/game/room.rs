use crate::db::{self, object_has_socket};
use crate::entity::{Character, EntityKind};
use crate::grid::{Direction, SquareCoord, grid_room_id_from_coord, terrain_name_zh};
use crate::model::{Exit, Room, RoomObject};
use crate::store;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

include!("room/trophic.rs");
include!("room/view_and_nav.rs");
include!("room/habitat.rs");
include!("room/waypoints_and_talismans.rs");
include!("room/tests.rs");
