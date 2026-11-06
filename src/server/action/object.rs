//! `do_action` — 物件分支：房內非實體目標（RoomObject）的互動與 Move 穿梭。

use crate::db::{
    self, get_object_and_room, get_object_by_id_in_room, get_object_by_name_in_room,
    object_has_socket, object_response, set_entity_grid,
};
use crate::event;

mod types {
    pub use crate::event::types::*;
    pub const PLAYER_BODY: &str = "player_body";
}
use crate::game;
use crate::gametext;
use crate::grid::parse_grid_room_id;
use crate::server::m3_029;
use crate::world::materials;
use std::collections::HashMap;

use super::super::broadcast::{refresh_room_views_for_room, send_room_view_to_session};
use super::super::handler::{WsConnection, push_refresh};
use super::super::protocol::{ActionResultMsg, MovedMsg};
use super::{action_label_zh, client_err_fmt, current_game_hour};
#[cfg(test)]
use super::normalize_action_key;

const M3_020_MINT_QUEUE_STATE: &str = "m3_020_mint_queue|queued_after_waiting_line";
const M3_020_SHORT_WORK_STATE: &str = "m3_020_short_work|accepted_loading_shift";
const M3_020_PASS_CHECK_STATE: &str = "m3_020_pass_check|joined_gate_review";
const M3_020_COUNCIL_COMPLAINT_STATE: &str = "m3_020_council_complaint|registered_notice_name";
const M3_027_REED_ITEM_ID: &str = "mat_reed_bundle";
const M3_027_SACKCLOTH_ITEM_ID: &str = "mat_sackcloth_bolt";
const M3_027_ROPE_ITEM_ID: &str = "mat_hemp_rope_roll";
const M3_027_REED_MARKET_DEMAND_PER_ROUND: usize = 2;
const M3_027_SOUP_COST: i32 = 2;
const M3_027_CARGO_PAY: i32 = 3;
const M3_027_REED_SELL_PAY: i32 = 4;
const M3_027_SACKCLOTH_SELL_PAY: i32 = 5;
const M3_027_ROPE_SELL_PAY: i32 = 6;
const M3_027_CARGO_TALLY_PAY: i32 = 1;
const M3_027_REED_GATHERED_STATE: &str = "m3_027_material_flow|gathered_north_bank_reed";
const M3_027_SACKCLOTH_GATHERED_STATE: &str =
    "m3_027_material_flow|gathered_short_market_sackcloth";
const M3_027_ROPE_GATHERED_STATE: &str = "m3_027_material_flow|gathered_ferry_plank_rope";
const M3_027_BENCH_REST_STATE: &str = "m3_027_player_body|rested_on_wet_bench";
const M3_027_SOUP_WARM_STATE: &str = "m3_027_player_body|warmed_by_hot_soup";
const M3_027_FREE_BROTH_STATE: &str = "m3_027_player_body|waited_for_shared_broth";
const M3_027_CARGO_HELP_STATE: &str = "m3_027_ferry_credit|carried_cargo_to_plank";
const M3_027_ROPE_HELP_STATE: &str = "m3_027_ferry_credit|helped_boat_rope";
const M3_027_REED_SOLD_STATE: &str = "m3_027_short_market|sold_reed_bundle";
const M3_027_REED_DEMAND_FILLED_STATE: &str = "m3_027_short_market|reed_demand_filled";
const M3_027_SACKCLOTH_DEMAND_FILLED_STATE: &str = "m3_027_short_market|sackcloth_demand_filled";
const M3_027_ROPE_DEMAND_FILLED_STATE: &str = "m3_027_short_market|rope_demand_filled";
const M3_027_BOARD_FERRY_STATE: &str = "m3_027_ferry_access|boarded_from_plank";
const M3_027_LANDED_SOUTH_STATE: &str = "m3_027_ferry_access|stepped_ashore_south";
const M3_027_WAREHOUSE_TALLY_STATE: &str = "m3_027_cargo_loop|checked_cargo_at_warehouse";
const M3_027_AWNING_DRY_STATE: &str = "m3_027_player_body|dried_under_river_awning";
const M3_027_BRAZIER_WARM_STATE: &str = "m3_027_player_body|warmed_hands_at_brazier";
const M3_027_QUEUE_TALK_STATE: &str = "m3_027_player_social|talked_in_waiting_line";
const M3_027_QUEUE_FERRY_STATE: &str = "m3_027_ferry_access|queued_for_ferry";


include!("object/resolve_1.rs");
include!("object/resolve_2.rs");
include!("object/resolve_3.rs");
include!("object/handler_1.rs");
include!("object/handler_2.rs");
include!("object/effects_1.rs");
include!("object/effects_2.rs");
include!("object/effects_3.rs");
include!("object/effects_4.rs");
include!("object/effects_5.rs");
include!("object/resource.rs");
include!("object/m3_020.rs");
include!("object/m3_027_1.rs");
include!("object/m3_027_2.rs");
include!("object/followup.rs");
include!("object/tests_1.rs");
include!("object/tests_2.rs");
