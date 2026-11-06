use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::combat::wordplate::{
    CarrierAffinity, CraftDims4D, OrbitFluency, StarInlay, StarSentence, TacticalLoadout,
    TacticalSlot, TargetPart, WordCasket, WordElement, WordQuality, WordRegistry,
};
use crate::db::{
    self, add_to_inventory, clear_equipment_slot, compute_resource_maxes,
    expand_soul_seed_to_origin_sentence, expand_soul_seed_to_topology_costs, get_inventory,
    get_item_def, inventory_has_item, inventory_weight, remove_from_inventory,
    update_equipment_slot,
};
use crate::entity::Character;
use crate::gametext;
use crate::grid::{SquareCoord, grid_room_id_from_coord};

use super::super::protocol::{
    ActionResultMsg, CasketWordEntry, ClientMsg, EntityStatusMsg, InventoryItemView, InventoryMsg,
    NaturalSkillView, TacticalLoadoutView, TacticalSentenceView, TacticalStateMsg,
    TopologyDebugAckMsg,
};
use super::{WsConnection, parse_activated_nodes, player_id};

include!("inventory/equipment.rs");
include!("inventory/actions.rs");
include!("inventory/use_manual.rs");
include!("inventory/use_corpse.rs");
include!("inventory/use_item.rs");
include!("inventory/casket_state.rs");
include!("inventory/skills.rs");
include!("inventory/tests.rs");
