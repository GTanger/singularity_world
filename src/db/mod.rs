// db 模組 — 資料存取門面（讀寫 store），對齊既有 db/ 層。
// store 為唯一資料源；db 提供業務語義的包裝（實體查詢、密碼驗證、soul_seed 展開等）。

mod acquaintance;
mod archival;
mod assignment;
mod climate;
mod dialogue;
mod disposition;
mod equip;
mod favorability;
mod grid_world;
mod identity;
pub mod inventory;
pub use inventory::ItemDefBrief;
mod journey;
mod lexicon;
mod npc_display;
mod npc_events;
mod npc_expense;
mod npc_names;
mod npc_social;
mod npc_spawn;
mod npc_trade;
mod occupation;
mod room_graph;
mod room_object;
mod sched;
mod text;
mod trade_pending;

use crate::entity::Character;
use crate::store::{self, Entity};

// 正方格世界格網門面（實作於 `grid_world`）
/// 自 PostgreSQL 載入正方格世界格網。
pub fn load_square_grid() -> Option<crate::grid::SquareGrid> {
    grid_world::load_square_grid()
}

/// 將正方格世界格網寫入 PostgreSQL。
pub fn save_square_grid_to_pg(grid: &crate::grid::SquareGrid) -> anyhow::Result<()> {
    grid_world::save_square_grid_to_pg(grid)
}

/// 更新單一格子的 freshness（explore handler 呼叫）。
pub fn update_cell_freshness(
    x: i32,
    y: i32,
    freshness: crate::grid::Freshness,
) -> anyhow::Result<()> {
    grid_world::update_cell_freshness(x, y, freshness)
}

/// 更新單一格子的 explored、freshness 與 objects（探索或採集後直接單格寫入 PG）。
pub fn update_cell_state(
    x: i32,
    y: i32,
    explored: bool,
    freshness: crate::grid::Freshness,
    objects: &[crate::grid::GridObject],
) -> anyhow::Result<()> {
    grid_world::update_cell_state(x, y, explored, freshness, objects)
}

/// simulation tick：將無玩家在場的 non-fresh 格子回復為 fresh。
pub fn reset_freshness_for_vacant_cells(occupied_coords: &[(i32, i32)]) -> anyhow::Result<()> {
    grid_world::reset_freshness_for_vacant_cells(occupied_coords)
}

pub use archival::{
    insert_archival, insert_npc_npc_dialogue_archival, pick_style_examples,
    recent_npc_npc_archival_lines_for_entity, search_archival, search_archival_for_player_talk,
    set_npc_summary,
};
pub use assignment::{
    Assignment, Venue, entity_in_venue_at_room, get_all_venue_ids, get_all_venue_room_ids,
    get_assignment_count_by_venue, get_assignments_for_entity, get_first_occupation_id_for_venue,
    get_npc_title_from_assignments, get_room_ids_for_venue, get_runtime_occupation_for_venue,
    get_venue_ids_for_room, get_venue_max_staff, insert_assignment, is_room_in_venue,
    remove_assignments_for_entity, seed_venues,
};
pub use dialogue::{
    DialogueFile, DialogueSlots, PlaceholderCtx, apply_micro_variants, fill_placeholders,
    load_dialogue, load_dialogue_keywords, load_dialogue_slots, pick_from_dialogue,
    pick_from_public_talk, pick_line_weighted, try_match_keyword,
};
pub use disposition::{
    ThoughtSource, add_thought, add_thought_to_all_npcs_with_flavor, add_thought_to_entity_with_flavor,
    add_thought_to_entity_with_delta, add_thought_with_flavor,
    adjust_disposition, clean_and_recalc_entity_disposition, get_active_thoughts, get_disposition,
};
pub use equip::{get_item_descs, get_item_names, is_naked, seed_items, starter_equipment};
pub use favorability::{
    FAV_BORROW_CAUGHT, FAV_BORROW_SUCCESS, FAV_SLAY, FAV_SUBDUE, FAV_TALK, adjust_favorability,
    format_npc_memory_for_backstory, get_favorability,
};
pub use identity::build_identity;
pub use inventory::{
    active_trade_inventory_total_qty, add_item, add_to_inventory, clear_equipment_slot, default_trade_ask_mg,
    get_inventory, get_item_def, get_item_name, inventory_has_item, inventory_has_qty, inventory_total_qty,
    inventory_weight, item_is_tradable, mirror_market_trade_settlement, pick_npc_trade_offer,
    remove_from_inventory, settle_inventory_trade, settle_npc_inventory_sale, trade_floor_from_ask,
    update_equipment_slot,
};
pub use journey::{
    get_entity_journey, get_person_world_state, load_pending_journeys, set_entity_journey,
};
pub use lexicon::{
    decay_lexicon, list_notable_terms, promote_lexicon_candidates, upsert_lexicon_term,
};
pub use npc_display::{
    NpcSchedule, get_npc_display_label_at_hour, get_npc_person_display_name, get_npc_title,
    get_npc_title_in_room, get_npc_title_in_room_at_hour, get_schedule_for_entity,
    person_name_from_npc_list_label, split_npc_list_display_label,
};
pub use npc_events::{
    EVT_BEG, EVT_DEATH, EVT_GATHER, EVT_HIRED, EVT_TALK, EVT_TRADE, EVT_WORK, get_recent_events,
    log_npc_event,
};
pub use npc_expense::{DAILY_EXPENSE_BASE, deduct_daily_expense};
pub use npc_names::{first_rune, generate_npc_name};
pub use npc_social::{
    build_npc_rumor_digest, decay_npc_rumors, delete_npc_npc_thread,
    get_npc_npc_conversation_summary, get_npc_npc_dyad, get_npc_npc_thread,
    set_npc_npc_conversation_summary, set_npc_npc_dyad, set_npc_npc_thread, upsert_npc_rumor,
};
pub use npc_spawn::{
    DEFAULT_NPCS, NpcDef, ensure_all_npcs_have_soul_seed, ensure_grid_coords,
    get_npc_gender_counts, get_room_count, insert_npc, seed_npcs, seed_npcs_for_store,
    spawn_one_npc_from_pool,
};
pub use npc_trade::npc_street_sell_one;
pub use occupation::{get_sockets_for_npc, is_default_socket};
pub use room_graph::{RoomGraph, rebuild_room_graph, sync_room_graph_with_store, with_room_graph};
pub use room_object::{
    get_object_and_room, get_object_by_id_in_room, get_object_by_name_in_room, object_response,
};
pub use sched::{
    ScheduleMove, ScheduleTarget, apply_schedules, get_all_schedules, get_schedule_target,
    get_schedule_target_room, insert_schedule, remove_schedule_for_entity,
};
pub use text::rune_lcs_similarity;
pub use trade_pending::{TradePending, trade_offer_clear, trade_offer_get, trade_offer_set};

mod soul_seed;
pub use soul_seed::{
    NUM_TOPOLOGY_EDGES, Personality, ResourceMaxes, TOTAL_TOPOLOGY_COST_NORM,
    compute_resource_maxes, expand_soul_seed_to_base_stats, expand_soul_seed_to_combat_axes,
    expand_soul_seed_to_origin_sentence, expand_soul_seed_to_personality,
    expand_soul_seed_to_topology_costs, generate_origin_sentence, generate_soul_seed,
};

mod entity;
pub use entity::{
    add_magnesium, clear_last_observed, get_entities_at_grid, get_entities_in_box,
    get_entities_in_room, get_entity, get_entity_room, get_moving_entities,
    get_personality_for_entity, set_move_target, transfer_magnesium, update_last_observed,
    update_position, update_vit,
};

mod auth;
pub use auth::{create_auth, has_password_for_entity, verify_password};

mod room;
pub use room::{
    SPAWN_ROOM_NAME, get_entity_place_id, get_exits_for_room, get_npc_ids_with_grid,
    get_npc_ids_with_room, get_objects_in_room, get_player_ids_with_room, get_room, get_room_name,
    get_spawn_room_id, object_has_socket, set_entity_activity, set_entity_grid, set_entity_room,
    terrain_from_room,
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
        && let Ok(slots) =
            serde_json::from_str::<std::collections::HashMap<String, String>>(&ch.equipment_slots)
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
        grid_x: e.grid_x,
        grid_y: e.grid_y,
        thoughts: e.thoughts.clone(),
    };
    if e.kind == "npc" {
        if !npc_display_title.is_empty() {
            c.display_title = npc_display_title.to_string();
        }
        if let Some(state) = crate::store::LIVE_NPC_GRID_STATES.read().get(&e.id) {
            if c.grid_x.is_none() || c.grid_y.is_none() {
                c.grid_x = Some(state.current.0);
                c.grid_y = Some(state.current.1);
            }
            let live_act = state.current_activity();
            if !live_act.is_empty() {
                c.current_activity = live_act;
            }
        }
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
        magnesium: 100,
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
        place_id: None,
        grid_x: None,
        grid_y: None,
        thoughts: vec![],
    };
    let pool = crate::pg::pool().ok_or(ErrNoStore)?;
    crate::pg::entity::upsert(&pool, &e)?;
    let arc = store::get_store().ok_or(ErrNoStore)?;
    arc.write().mirror_new_entity_from_authority(e)
}

/// 正規化房間 id 鍵（grid 世界僅為字串本身）。
pub fn canonical_location_key(room_id: &str) -> String {
    room_id.to_string()
}

/// 判斷兩個房間 id 是否等同（grid 世界直接比字串）。
pub fn location_keys_equivalent(a: &str, b: &str) -> bool {
    a == b
}

/// 記錄 NPC 與玩家見面（對齊 `RecordMeet`）。
pub fn record_meet(npc_id: &str, player_id: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.record_meet(npc_id, player_id)
}

/// 傳聞池 top K（對齊 `TopNpcRumors`）。
#[must_use]
pub fn top_npc_rumors(
    room_id: &str,
    zone: &str,
    now_unix: i64,
    top_k: i32,
) -> Vec<store::NpcRumor> {
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

// ─── v5 新增：氣候與觀察態 ───────────────────────────────────────────
pub use acquaintance::{
    NpcAcquaintance, ensure_acquaintance, get_acquaintance, get_acquaintances_for_player_npcs,
    set_named, set_observed,
};
pub use climate::{RegionClimate, get_region_climate, list_region_climates, upsert_region_climate};
