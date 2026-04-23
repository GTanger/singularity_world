// db 模組 — 資料存取門面（讀寫 store），對齊既有 db/ 層。
// store 為唯一資料源；db 提供業務語義的包裝（實體查詢、密碼驗證、soul_seed 展開等）。

mod archival;
mod assignment;
mod dialogue;
mod disposition;
mod favorability;
mod equip;
mod identity;
mod inventory;
mod npc_display;
mod npc_events;
mod npc_expense;
mod npc_names;
mod npc_trade;
mod npc_social;
mod npc_spawn;
mod occupation;
mod room_graph;
mod room_object;
mod sched;
mod trade_pending;
mod text;
mod lexicon;
mod grid_world;
mod hex_reveal;
mod hex_world;
mod room_hex;

use crate::entity::Character;
use crate::hex::HexGrid;
use crate::store::{self, Entity};

// 六角世界格網門面（實作於 `hex_world`）；由本模組直接呼叫，避免 `pub use` 子函式時 dead_code 誤報。
/// 自 PostgreSQL 載入 Hex 世界格網。
pub fn load_hex_grid() -> Option<HexGrid> {
    hex_world::load_hex_grid()
}

/// 將 Hex 世界格網寫入 PostgreSQL。
pub fn save_hex_grid_to_pg(grid: &HexGrid) -> anyhow::Result<()> {
    hex_world::save_hex_grid_to_pg(grid)
}

// 正方格世界格網門面（實作於 `grid_world`）
/// 自 PostgreSQL 載入正方格世界格網。
pub fn load_square_grid() -> Option<crate::grid::SquareGrid> {
    grid_world::load_square_grid()
}

/// 將正方格世界格網寫入 PostgreSQL。
pub fn save_square_grid_to_pg(grid: &crate::grid::SquareGrid) -> anyhow::Result<()> {
    grid_world::save_square_grid_to_pg(grid)
}

pub use assignment::{
    entity_in_venue_at_room, get_all_venue_ids, get_all_venue_room_ids, get_assignment_count_by_venue,
    get_assignments_for_entity, get_first_occupation_id_for_venue, get_npc_title_from_assignments,
    get_room_ids_for_venue, get_venue_ids_for_room, get_venue_max_staff, insert_assignment,
    is_room_in_venue, remove_assignments_for_entity, seed_venues, Assignment, Venue,
};
pub use equip::{get_item_descs, get_item_names, is_naked, seed_items, starter_equipment};
pub use npc_display::{
    get_npc_display_label_at_hour, get_npc_person_display_name, get_npc_title, get_npc_title_in_room,
    get_npc_title_in_room_at_hour, get_schedule_for_entity, person_name_from_npc_list_label,
    split_npc_list_display_label, NpcSchedule,
};
pub use npc_names::{first_rune, generate_npc_name};
pub use npc_spawn::{
    ensure_all_npcs_have_soul_seed, ensure_grid_coords, get_npc_gender_counts, get_room_count,
    insert_npc, seed_npcs, seed_npcs_for_store, spawn_one_npc_from_pool, DEFAULT_NPCS, NpcDef,
};
pub use archival::{
    insert_archival, insert_npc_npc_dialogue_archival, pick_style_examples,
    recent_npc_npc_archival_lines_for_entity, search_archival, search_archival_for_player_talk,
    set_npc_summary,
};
pub use disposition::{
    adjust_disposition, get_disposition, DISP_BEG_SUCCESS, DISP_BROKE, DISP_DAILY, DISP_GATHER, DISP_HIRED,
    DISP_SUBDUED, DISP_TALKED, DISP_TRADE,
};
pub use favorability::{
    adjust_favorability, format_npc_memory_for_backstory,
    FAV_BORROW_CAUGHT, FAV_BORROW_SUCCESS, FAV_SLAY, FAV_SUBDUE, FAV_TALK,
};
pub use identity::build_identity;
pub use inventory::{
    add_to_inventory, clear_equipment_slot, default_trade_ask_mg, get_inventory, get_item_def,
    inventory_has_item, inventory_total_qty, inventory_weight, pick_npc_trade_offer, remove_from_inventory,
    trade_floor_from_ask, update_equipment_slot,
};
pub use npc_events::{
    get_recent_events, log_npc_event, EVT_BEG, EVT_DEATH, EVT_GATHER, EVT_HIRED, EVT_TALK, EVT_TRADE,
};
pub use npc_trade::npc_street_sell_one;
pub use npc_expense::{deduct_daily_expense, DAILY_EXPENSE_BASE};
pub use npc_social::{
    build_npc_rumor_digest, decay_npc_rumors, delete_npc_npc_thread,
    get_npc_npc_conversation_summary, get_npc_npc_dyad, get_npc_npc_thread,
    set_npc_npc_conversation_summary, set_npc_npc_dyad, set_npc_npc_thread, upsert_npc_rumor,
};
pub use occupation::{get_sockets_for_npc, is_default_socket};
pub use hex_reveal::{
    count_player_hex_revealed, is_player_hex_revealed, list_player_hex_revealed,
    mark_player_hex_revealed,
};
pub use room_hex::{
    canonical_location_key, location_keys_equivalent, resolve_room_to_hex, room_hex_for_world_room,
};
pub use room_graph::{rebuild_room_graph, sync_room_graph_with_store, with_room_graph, RoomGraph};
pub use room_object::{get_object_and_room, get_object_by_id_in_room, get_object_by_name_in_room, object_response};
pub use trade_pending::{trade_offer_clear, trade_offer_get, trade_offer_set, TradePending};
pub use sched::{
    apply_schedules, get_all_schedules, get_schedule_target, get_schedule_target_room, insert_schedule,
    remove_schedule_for_entity, ScheduleMove, ScheduleTarget,
};
pub use text::rune_lcs_similarity;
pub use dialogue::{
    fill_placeholders, apply_micro_variants, load_dialogue, load_dialogue_keywords,
    load_dialogue_slots, pick_from_dialogue, pick_from_public_talk, pick_line_weighted,
    try_match_keyword, DialogueFile, DialogueSlots, PlaceholderCtx,
};
pub use lexicon::{upsert_lexicon_term, promote_lexicon_candidates, decay_lexicon, list_notable_terms};

mod soul_seed;
pub use soul_seed::{
    compute_resource_maxes, expand_soul_seed_to_base_stats, expand_soul_seed_to_combat_axes,
    expand_soul_seed_to_origin_sentence, expand_soul_seed_to_personality,
    expand_soul_seed_to_topology_costs, generate_origin_sentence, generate_soul_seed, Personality,
    ResourceMaxes, NUM_TOPOLOGY_EDGES, TOTAL_TOPOLOGY_COST_NORM,
};

mod entity;
pub use entity::{
    add_magnesium, clear_last_observed, get_entities_at_grid, get_entities_at_hex,
    get_entities_in_box, get_entities_in_room, get_entity, get_entity_room, get_moving_entities,
    get_personality_for_entity, set_move_target, transfer_magnesium, update_last_observed,
    update_position, update_vit,
};

mod auth;
pub use auth::{create_auth, has_password_for_entity, verify_password};

mod room;
pub use room::{
    get_exits_for_room, get_npc_ids_with_grid, get_npc_ids_with_room, get_objects_in_room,
    get_player_ids_with_room, get_room, get_room_name, get_spawn_room_id, object_has_socket,
    set_entity_activity, set_entity_grid, set_entity_hex, set_entity_room, terrain_from_room,
    SPAWN_ROOM_NAME,
};

// ══════════════════════════════════════
//  錯誤型別
// ══════════════════════════════════════

/// store.Default 未初始化。
#[derive(Debug, thiserror::Error)]
#[error("store not initialized")]
pub struct ErrNoStore;

/// items 定義中無此 id。
#[derive(Debug, thiserror::Error)]
#[error("item not found")]
pub struct ErrItemNotFound;

/// 找不到要更新的房間。
#[derive(Debug, thiserror::Error)]
#[error("room not found")]
pub struct ErrRoomNotFound;


/// 計算角色穿戴裝備後的有效屬性 (e_vit, e_qi, e_dex, atk_gear)。
pub fn effective_stats(ch: &crate::entity::Character) -> (i32, i32, i32, i32) {
    let mut e_vit = ch.vit;
    let mut e_dex = ch.dex;
    let mut atk_gear: i32 = 0;
    if !ch.equipment_slots.is_empty()
        && let Ok(slots) = serde_json::from_str::<std::collections::HashMap<String, String>>(&ch.equipment_slots)
    {
        for item_id in slots.values() {
                if item_id.is_empty() {
                    continue;
                }
                if let Some(item_def) = get_item_def(item_id) {
                    e_vit += item_def.vit_bonus;
                    e_dex += item_def.dex_bonus;
                    atk_gear += item_def.atk_bonus;
                }
            }
        }
    (e_vit, ch.qi, e_dex, atk_gear)
}

// ══════════════════════════════════════
//  store.Entity ↔ entity.Character 轉換
// ══════════════════════════════════════

/// 將 store::Entity 轉成 entity::Character。
pub fn store_entity_to_character(e: &Entity, npc_display_title: &str) -> Character {
    let mut c = Character {
        id: e.id.clone(),
        kind: match e.kind.as_str() {
            "npc" => crate::entity::EntityKind::Npc,
            _ => crate::entity::EntityKind::Player,
        },
        display_char: e.display_char.clone(),
        x: e.x,
        y: e.y,
        move_state: match e.move_state.as_str() {
            "moving" => crate::entity::MoveState::Moving,
            _ => crate::entity::MoveState::Idle,
        },
        target_x: e.target_x,
        target_y: e.target_y,
        walk_or_run: match e.walk_or_run.as_str() {
            "run" => crate::entity::WalkOrRun::Run,
            _ => crate::entity::WalkOrRun::Walk,
        },
        move_started_at: e.move_started_at,
        vit: e.vit,
        qi: e.qi,
        dex: e.dex,
        magnesium: e.magnesium,
        last_observed_at: e.last_observed_at,
        created_at: e.created_at,
        gender: match e.gender.as_str() {
            "F" => Some(crate::entity::Gender::F),
            "M" => Some(crate::entity::Gender::M),
            _ => None,
        },
        soul_seed: e.soul_seed,
        display_title: e.display_title.clone(),
        activated_nodes: e.activated_nodes.clone(),
        equipment_slots: e.equipment_slots.clone(),
        inventory: e.inventory.clone(),
        disposition: e.disposition,
        current_activity: e.current_activity.clone(),
        hex_q: e.hex_q,
        hex_r: e.hex_r,
        grid_x: e.grid_x,
        grid_y: e.grid_y,
    };
    if e.kind == "npc" && !npc_display_title.is_empty() {
        c.display_title = npc_display_title.to_string();
    }
    c
}


/// 新增玩家實體（對齊 `InsertEntity`）。
pub fn insert_entity(id: &str, display_char: &str, gender: &str) -> anyhow::Result<()> {
    let mut display_char = display_char.trim().to_string();
    if display_char.is_empty() {
        display_char = "我".into();
    }
    let gender = if gender == "F" || gender == "女" {
        "F".to_string()
    } else {
        "M".to_string()
    };
    let seed = generate_soul_seed();
    let (vit, qi, dex) = expand_soul_seed_to_base_stats(seed);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let equip = equip::starter_equipment(&gender);
    let e = Entity {
        id: id.to_string(),
        kind: "player".into(),
        display_char,
        x: 0,
        y: 0,
        move_state: "idle".into(),
        target_x: None,
        target_y: None,
        walk_or_run: String::new(),
        move_started_at: None,
        vit,
        qi,
        dex,
        magnesium: 0,
        last_observed_at: None,
        created_at: now,
        gender,
        soul_seed: Some(seed),
        display_title: String::new(),
        activated_nodes: r#"["N000"]"#.into(),
        equipment_slots: equip,
        inventory: "[]".into(),
        disposition: 0,
        current_activity: String::new(),
        hex_q: None,
        hex_r: None,
        grid_x: None,
        grid_y: None,
    };
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.put_entity(e)
}

/// 記錄 NPC 與玩家見面（對齊 `RecordMeet`）。
pub fn record_meet(npc_id: &str, player_id: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.record_meet(npc_id, player_id)
}

/// 傳聞池 top K（對齊 `TopNpcRumors`）。
#[must_use]
pub fn top_npc_rumors(room_id: &str, zone: &str, now_unix: i64, top_k: i32) -> Vec<store::NpcRumor> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read();
    s.top_npc_rumors(room_id, zone, now_unix, top_k)
}

/// 標記傳聞被引用。
pub fn mark_rumor_used_by_text(text: &str, now_unix: i64) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.mark_rumor_used_by_text(text, now_unix)
}

/// 衝突降權傳聞。
pub fn penalize_rumor_by_text(text: &str, now_unix: i64, reason: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.penalize_rumor_by_text(text, now_unix, reason)
}

