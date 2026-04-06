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
    ensure_all_npcs_have_soul_seed, get_npc_gender_counts, get_room_count, insert_npc, seed_npcs, seed_npcs_for_store,
    spawn_one_npc_from_pool, DEFAULT_NPCS, NpcDef,
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

// ══════════════════════════════════════
//  soul_seed 展開 — 三軸與基礎屬性
// ══════════════════════════════════════

// 三軸區間與映射常數（與人物屬性彙整 §二 一致）
const AMP_MIN: f64 = 0.1;
const AMP_MAX: f64 = 3.0;
const FREQ_MIN: f64 = 0.5;
const FREQ_MAX: f64 = 2.0;
const PHASE_MIN: f64 = -1.0;
const PHASE_MAX: f64 = 1.0;
const BASE_STAT: f64 = 10.0;
const K_AMP: f64 = 0.2;
const K_FREQ: f64 = 0.2;
const K_PHASE: f64 = 0.2;
const MIN_STAT: i32 = 1;
const MAX_STAT: i32 = 30;

/// 由 seed 前 3 次偽隨機展開三軸（能階、時脈、相位）。
fn expand_seed_axes(seed: i64) -> (f64, f64, f64) {
    // 簡易 LCG（與舊版 PRNG 種子用法相容度足夠，非加密用途）
    let mut rng = SimpleLcg::new(seed);
    let u1 = rng.next_f64();
    let u2 = rng.next_f64();
    let u3 = rng.next_f64();
    let amp = AMP_MIN + u1 * (AMP_MAX - AMP_MIN);
    let freq = FREQ_MIN + u2 * (FREQ_MAX - FREQ_MIN);
    let phase = PHASE_MIN + u3 * (PHASE_MAX - PHASE_MIN);
    (amp, freq, phase)
}

/// 產生加密安全的 soul_seed（i64）。
pub fn generate_soul_seed() -> i64 {
    let mut buf = [0u8; 8];
    // 使用 getrandom 或簡易 fallback
    if getrandom_fill(&mut buf).is_err() {
        // fallback: 系統時間
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        return t as i64;
    }
    i64::from_be_bytes(buf)
}

fn getrandom_fill(buf: &mut [u8]) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Read;
    let mut f = File::open("/dev/urandom")?;
    f.read_exact(buf)?;
    Ok(())
}

/// 由 soul_seed 展開為基礎體質／氣脈／靈敏。
pub fn expand_soul_seed_to_base_stats(seed: i64) -> (i32, i32, i32) {
    let (amp, freq, phase) = expand_seed_axes(seed);
    let v = (BASE_STAT * (1.0 + K_AMP * (amp - 1.0))).round() as i32;
    let q = (BASE_STAT * (1.0 + K_FREQ * (freq - 1.0))).round() as i32;
    let d = (BASE_STAT * (1.0 + K_PHASE * phase)).round() as i32;
    (v.clamp(MIN_STAT, MAX_STAT), q.clamp(MIN_STAT, MAX_STAT), d.clamp(MIN_STAT, MAX_STAT))
}

/// 四項資源最大值。
pub struct ResourceMaxes {
    pub hp_max: f64,
    pub inner_max: f64,
    pub spirit_max: f64,
    pub stamina_max: f64,
}

/// 由體質、氣脈、靈敏計算四項資源最大值。
pub fn compute_resource_maxes(vit: i32, qi: i32, dex: i32) -> ResourceMaxes {
    let (v, q, d) = (vit as f64, qi as f64, dex as f64);
    ResourceMaxes {
        hp_max: ((0.7 * v + 0.3 * q) * (0.05 * d + 1.0) * v).ceil(),
        inner_max: ((0.7 * q + 0.3 * v) * (0.05 * d + 1.0) * q).ceil(),
        spirit_max: ((0.6 * d + 0.4 * q) * (0.05 * v + 1.0) * d).ceil(),
        stamina_max: ((0.5 * v + 0.4 * q + 0.3 * d) * ((v + q + d) / 3.0)).ceil(),
    }
}

/// 由三軸產出本源語句。
pub fn generate_origin_sentence(amp: f64, freq: f64, phase: f64) -> String {
    let amp_word = if amp < 1.0 { "幽微" } else if amp > 2.0 { "霸道" } else { "綿長" };
    let freq_word = if freq < 1.0 { "渾厚" } else if freq > 1.6 { "敏銳" } else { "洞察" };
    let phase_word = if phase < -0.3 { "混沌" } else if phase > 0.3 { "秩序" } else { "順流" };
    format!("你的神識{amp_word}且{freq_word}，隱隱透著一股{phase_word}的逆流。")
}

/// 由 soul_seed 展開為本源語句。
pub fn expand_soul_seed_to_origin_sentence(seed: i64) -> String {
    let (amp, freq, phase) = expand_seed_axes(seed);
    generate_origin_sentence(amp, freq, phase)
}

/// 性格維度（皆 [0,1]），供決策／對話權重使用。
#[derive(Debug, Clone)]
pub struct Personality {
    pub boldness: f64,
    pub sensitivity: f64,
    pub orderliness: f64,
}

/// 由 soul_seed 展開為性格維度。
pub fn expand_soul_seed_to_personality(seed: i64) -> Personality {
    let (amp, freq, phase) = expand_seed_axes(seed);
    let norm = |v: f64, lo: f64, hi: f64| -> f64 {
        ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
    };
    Personality {
        boldness: norm(amp, AMP_MIN, AMP_MAX),
        sensitivity: norm(freq, FREQ_MIN, FREQ_MAX),
        orderliness: norm(phase, PHASE_MIN, PHASE_MAX),
    }
}

/// 由 soul_seed 展開為戰鬥三係數（能階α、時脈β、相位γ）。
pub fn expand_soul_seed_to_combat_axes(seed: i64) -> (f64, f64, f64) {
    let (amp, freq, phase) = expand_seed_axes(seed);
    let alpha = 0.5 + (amp - AMP_MIN) / (AMP_MAX - AMP_MIN) * 1.5;
    let beta = 0.5 + (freq - FREQ_MIN) / (FREQ_MAX - FREQ_MIN) * 1.0;
    let gamma = (phase - PHASE_MIN) / (PHASE_MAX - PHASE_MIN);
    (alpha, beta, gamma)
}

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
    };
    if e.kind == "npc" && !npc_display_title.is_empty() {
        c.display_title = npc_display_title.to_string();
    }
    c
}

// ══════════════════════════════════════
//  密碼（bcrypt）
// ══════════════════════════════════════

const BCRYPT_COST: u32 = 10;

/// 建立密碼雜湊。
pub fn create_auth(entity_id: &str, password: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let hash = bcrypt::hash(password, BCRYPT_COST)?;
    let mut s = arc.write().unwrap();
    s.set_auth(entity_id, &hash)
}

/// 是否已有密碼。
pub fn has_password_for_entity(entity_id: &str) -> bool {
    let Some(arc) = store::get_store() else { return false };
    let s = arc.read().unwrap();
    !s.get_auth(entity_id).is_empty()
}

/// 驗證密碼。
pub fn verify_password(entity_id: &str, password: &str) -> anyhow::Result<bool> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    let hash = s.get_auth(entity_id);
    if hash.is_empty() {
        return Ok(false);
    }
    Ok(bcrypt::verify(password, &hash).unwrap_or(false))
}

// ══════════════════════════════════════
//  實體查詢
// ══════════════════════════════════════

/// 依 id 查詢實體。
pub fn get_entity(id: &str) -> anyhow::Result<Option<Character>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    let Some(se) = s.get_entity(id) else { return Ok(None) };
    if se.kind == "npc" {
        npc_display::npc_person_display_name_locked(&mut s, id);
    }
    let Some(se) = s.get_entity(id) else { return Ok(None) };
    Ok(Some(store_entity_to_character(&se, "")))
}

/// 回傳實體當前房間 id（對齊既有 `db.GetEntityRoom`）。
pub fn get_entity_room(entity_id: &str) -> anyhow::Result<String> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.get_entity_room(entity_id))
}

/// 查詢座標落在 `[x_min,x_max]×[y_min,y_max]` 內的實體；`kind` 空字串表示不限種類（對齊既有 `GetEntitiesInBox`）。
pub fn get_entities_in_box(
    x_min: i32,
    x_max: i32,
    y_min: i32,
    y_max: i32,
    kind: &str,
) -> anyhow::Result<Vec<Character>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    let sel = s.get_entities_in_box(x_min, x_max, y_min, y_max, kind);
    let mut list = Vec::with_capacity(sel.len());
    for e in sel {
        let mut se = e;
        if se.kind == "npc" {
            npc_display::npc_person_display_name_locked(&mut s, &se.id);
            if let Some(updated) = s.get_entity(&se.id) {
                se = updated;
            }
        }
        list.push(store_entity_to_character(&se, ""));
    }
    Ok(list)
}

/// 回傳所有 `move_state == "moving"` 的實體（對齊既有 `GetMovingEntities`）。
pub fn get_moving_entities() -> anyhow::Result<Vec<Character>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    let ids = s.get_moving_entity_ids();
    let mut list = Vec::with_capacity(ids.len());
    for id in ids {
        let Some(se) = s.get_entity(&id) else {
            continue;
        };
        list.push(store_entity_to_character(&se, ""));
    }
    Ok(list)
}

/// 回傳指定房間內所有存活實體（對齊既有 `GetEntitiesInRoom`）。
/// `game_hour` 0–23 供在職場顯示職稱；`-1` 不套用下班規則（職場內一律「職稱|真名」）。
///
/// 合併 `entity_rooms` 字串與 [`resolve_room_to_hex`] 對應之六角座標上的實體，避免世界房間 id 與 `hex:…` 各寫一套時漏列。
pub fn get_entities_in_room(room_id: &str, game_hour: i32) -> anyhow::Result<Vec<Character>> {
    use std::collections::HashSet;

    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    let mut id_set: HashSet<String> = HashSet::new();
    for id in s.entity_ids_in_room(room_id) {
        id_set.insert(id);
    }
    if let Some((q, r)) = room_hex::resolve_room_to_hex(room_id) {
        for id in s.entity_ids_at_hex(q, r) {
            id_set.insert(id);
        }
    }
    if id_set.is_empty() {
        return Ok(Vec::new());
    }
    let label_room = room_hex::canonical_location_key(room_id);
    let mut list = Vec::new();
    for id in id_set {
        let Some(se) = s.get_entity(&id) else {
            continue;
        };
        if se.vit <= 0 {
            continue;
        }
        let label = if se.kind == "npc" {
            npc_display::npc_title_in_room_locked(&mut s, &id, &label_room, game_hour)
        } else {
            String::new()
        };
        let se = s.get_entity(&id).unwrap_or(se);
        list.push(store_entity_to_character(&se, &label));
    }
    Ok(list)
}

/// 指定六角座標上的存活實體（與 `hex_q`／`hex_r` 對齊；`room_id` 供 NPC 職稱推斷用）。
pub fn get_entities_at_hex(q: i32, r: i32, game_hour: i32) -> anyhow::Result<Vec<Character>> {
    use crate::hex::hex_room_id_from_coord;

    let hex_room = hex_room_id_from_coord(q, r);
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    let ids = s.entity_ids_at_hex(q, r);
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut list = Vec::new();
    for id in ids {
        let Some(se) = s.get_entity(&id) else {
            continue;
        };
        if se.vit <= 0 {
            continue;
        }
        let label = if se.kind == "npc" {
            npc_display::npc_title_in_room_locked(&mut s, &id, &hex_room, game_hour)
        } else {
            String::new()
        };
        let se = s.get_entity(&id).unwrap_or(se);
        list.push(store_entity_to_character(&se, &label));
    }
    Ok(list)
}

/// 依 id 查 soul_seed 並展開為 Personality。
pub fn get_personality_for_entity(entity_id: &str) -> Option<Personality> {
    let arc = store::get_store()?;
    let s = arc.read().unwrap();
    let e = s.get_entity(entity_id)?;
    let seed = e.soul_seed?;
    Some(expand_soul_seed_to_personality(seed))
}

/// 更新 last_observed_at。
pub fn update_last_observed(id: &str, at: i64) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.update_entity(id, |e| { e.last_observed_at = Some(at); })
}

/// 清除 last_observed_at。
pub fn clear_last_observed(id: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.update_entity(id, |e| { e.last_observed_at = None; })
}

/// 更新位置並設為 idle。
pub fn update_position(id: &str, x: i32, y: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.update_entity(id, |e| {
        e.x = x;
        e.y = y;
        e.move_state = "idle".to_string();
        e.target_x = None;
        e.target_y = None;
        e.move_started_at = None;
    })
}

/// 設定移動目標。
pub fn set_move_target(id: &str, target_x: i32, target_y: i32, walk_or_run: &str, started_at: i64) -> anyhow::Result<()> {
    let wor = if walk_or_run.is_empty() { "walk" } else { walk_or_run };
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    let wor = wor.to_string();
    s.update_entity(id, move |e| {
        e.target_x = Some(target_x);
        e.target_y = Some(target_y);
        e.move_state = "moving".to_string();
        e.walk_or_run = wor;
        e.move_started_at = Some(started_at);
    })
}

/// 增減鎂（clamp >= 0）。
pub fn add_magnesium(entity_id: &str, delta: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.update_entity(entity_id, |e| {
        e.magnesium += delta;
        if e.magnesium < 0 { e.magnesium = 0; }
    })
}

/// 將 `amount` 鎂自 `from_id` 轉至 `to_id`（餘額不足或實體不存在則錯）；對齊既有 `db.TransferMagnesium`。
pub fn transfer_magnesium(from_id: &str, to_id: &str, amount: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.transfer_magnesium(from_id, to_id, amount)
}

/// 更新體質（氣血）。
pub fn update_vit(entity_id: &str, new_vit: i32) -> anyhow::Result<()> {
    let v = new_vit.max(0);
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.update_entity(entity_id, move |e| { e.vit = v; })
}

// ══════════════════════════════════════
//  房間查詢
// ══════════════════════════════════════

/// 創生預設房間名稱。
pub const SPAWN_ROOM_NAME: &str = "宜林";

/// 取得創生預設房間 id。
pub fn get_spawn_room_id() -> String {
    let Some(arc) = store::get_store() else { return "lobby".to_string() };
    let s = arc.read().unwrap();
    let id = s.get_room_id_by_name(SPAWN_ROOM_NAME);
    if id.is_empty() { "lobby".to_string() } else { id }
}

/// 有房間座標的 NPC id 列表（對齊既有 `GetNPCIDsWithRoom`）。
#[must_use]
pub fn get_npc_ids_with_room() -> Vec<String> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read().unwrap();
    s.get_npc_ids_with_room()
}

/// 有房間座標的玩家 id 列表（對齊既有 `GetPlayerIDsWithRoom`）。
#[must_use]
pub fn get_player_ids_with_room() -> Vec<String> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read().unwrap();
    s.get_player_ids_with_room()
}

/// 回傳房間的戰鬥地形標籤。
pub fn terrain_from_room(room_id: &str) -> String {
    let Some(arc) = store::get_store() else { return String::new() };
    let s = arc.read().unwrap();
    let Some(room) = s.get_room(room_id) else { return String::new() };
    for t in &room.tags {
        let lt = t.trim().to_lowercase();
        if matches!(lt.as_str(), "lush" | "chaos" | "silent" | "grip") {
            return lt;
        }
    }
    if room.zone.trim().to_lowercase() == "chaos" {
        return "chaos".to_string();
    }
    String::new()
}

/// 依 id 查房間（對齊既有 `GetRoom`）。
pub fn get_room(room_id: &str) -> anyhow::Result<Option<crate::model::Room>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.get_room(room_id))
}

/// 房間顯示名稱（對齊既有 `GetRoomName`）。
pub fn get_room_name(room_id: &str) -> anyhow::Result<String> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.get_room_name(room_id))
}

/// 將實體設為在指定房間（對齊既有 `SetEntityRoom`）。
/// 若 `room_id` 為 `hex:q:r` 或於 `room_hex_overlay.json`／創生房規則可解析為六角，則改寫權威座標為 [`set_entity_hex`]。
pub fn set_entity_room(entity_id: &str, room_id: &str) -> anyhow::Result<()> {
    if let Some((q, r)) = crate::hex::parse_hex_room_id(room_id) {
        return set_entity_hex(entity_id, q, r);
    }
    if let Some((q, r)) = room_hex::room_hex_for_world_room(room_id) {
        return set_entity_hex(entity_id, q, r);
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.set_entity_room(entity_id, room_id)?;
    s.clear_entity_hex(entity_id)
}

/// 權威六角座標；見 [`store::Store::set_entity_hex`]（同步 `entity_rooms` 為 `hex:…`）。
pub fn set_entity_hex(entity_id: &str, q: i32, r: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.set_entity_hex(entity_id, q, r)
}

/// 設定實體的表面可觀測行為（玩家 Look 時看到的「在做什麼」）。
pub fn set_entity_activity(entity_id: &str, activity: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.set_entity_activity(entity_id, activity)
}

/// 房間出口列表（對齊 `GetExitsForRoom`）。
pub fn get_exits_for_room(room_id: &str) -> anyhow::Result<Vec<crate::model::Exit>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.get_exits_for_room(room_id))
}

/// 房間內可互動物件（對齊 `GetObjectsInRoom`）。
pub fn get_objects_in_room(room_id: &str) -> anyhow::Result<Vec<crate::model::RoomObject>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.get_room(room_id).map(|r| r.objects).unwrap_or_default())
}

/// 物件是否具備指定動詞插座（對齊 `ObjectHasSocket`）。
#[must_use]
pub fn object_has_socket(obj: &crate::model::RoomObject, action: &str) -> bool {
    obj.sockets.iter().any(|s| s.eq_ignore_ascii_case(action))
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
    };
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.put_entity(e)
}

/// 記錄 NPC 與玩家見面（對齊 `RecordMeet`）。
pub fn record_meet(npc_id: &str, player_id: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.record_meet(npc_id, player_id)
}

/// 傳聞池 top K（對齊 `TopNpcRumors`）。
#[must_use]
pub fn top_npc_rumors(room_id: &str, zone: &str, now_unix: i64, top_k: i32) -> Vec<store::NpcRumor> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read().unwrap();
    s.top_npc_rumors(room_id, zone, now_unix, top_k)
}

/// 標記傳聞被引用。
pub fn mark_rumor_used_by_text(text: &str, now_unix: i64) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.mark_rumor_used_by_text(text, now_unix)
}

/// 衝突降權傳聞。
pub fn penalize_rumor_by_text(text: &str, now_unix: i64, reason: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.penalize_rumor_by_text(text, now_unix, reason)
}

// ══════════════════════════════════════
//  簡易 LCG 偽隨機（對齊既有 math/rand）
// ══════════════════════════════════════

struct SimpleLcg {
    state: i64,
}

impl SimpleLcg {
    fn new(seed: i64) -> Self {
        Self { state: seed }
    }

    fn next_i64(&mut self) -> i64 {
        // 標準庫 PRNG 的 rngSource 使用的是更複雜的演算法，
        // 但對於 soul_seed 展開只要一致性即可，後續可精確對齊。
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    fn next_f64(&mut self) -> f64 {
        let v = self.next_i64();
        // 取高 53 位元映射到 [0, 1)
        let bits = (v as u64) >> 11;
        bits as f64 / (1u64 << 53) as f64
    }
}

/// 拓撲邊權總量常數（對齊既有 `TotalCostNorm`）。
pub const TOTAL_TOPOLOGY_COST_NORM: f64 = 10_000.0;
/// 拓撲邊數（對齊既有 `NumTopologyEdges`）。
pub const NUM_TOPOLOGY_EDGES: usize = 760;

/// 由 soul_seed 產生 760 條拓撲 Cost（對齊 `ExpandSoulSeedToTopologyCosts` 流程）。
#[must_use]
pub fn expand_soul_seed_to_topology_costs(seed: i64) -> Vec<f64> {
    let mut rng = SimpleLcg::new(seed);
    let _ = (rng.next_f64(), rng.next_f64(), rng.next_f64());
    let mut raw = vec![0f64; NUM_TOPOLOGY_EDGES];
    let mut sum = 0f64;
    for r in &mut raw {
        let u = rng.next_f64();
        *r = 0.1 + u * 0.9;
        sum += *r;
    }
    raw.iter().map(|x| (x / sum) * TOTAL_TOPOLOGY_COST_NORM).collect()
}
