// store 模組 — PostgreSQL 為主資料源，HashMap 為讀取快取。
// 啟動時優先從 PostgreSQL 載入；若 DB 為空則從 JSON 種子灌入後同步至 PG。
// 執行期：寫入 → PG（單筆 upsert） → 記憶體快取 → JSON 備份。
// 讀取 → 記憶體快取（零延遲）。
#![allow(clippy::collapsible_if)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
// parking_lot::RwLock：eventual fairness、不 poison、比 std 快
// 解 std::sync::RwLock 在 writer-heavy 時 reader starve 的 OS 預設不確定性
use parking_lot::RwLock;

use crate::model;

pub mod sql;
pub mod types;
pub use types::*;
mod archival;
mod auth;
pub mod households;
pub mod outer_households;
pub use outer_households::OUTER_HOUSEHOLD_SLOTS;
mod entity;
pub use entity::{
    canonical_npc_macro_settlement, clean_wilderness_place_label,
    clear_active_npc_obligation, current_game_hour, find_bfs_next_step,
    get_active_npc_dialogue, get_active_npc_obligation, is_npc_paused_for_interaction,
    is_role_or_placeholder_label, mark_player_knows_npc, mark_player_observed_npc,
    normalize_household_home_settlement,
    npc_brief_appearance_tag, npc_effective_is_female, npc_persistent_role_label,
    npc_sensory_appearance_line, open_active_npc_obligation, pause_npc_for_interaction,
    player_knows_npc, player_observed_npc, player_pledged_hunting_zone,
    pledge_player_to_hunting_party, record_active_npc_dialogue, resolve_npc_full_label,
    resolve_npc_person_name, resolve_wenyao_npc_schedule, unpause_npc_for_interaction,
    wenyao_household_slot_for,
    ActiveNpcDialogue, HuntingPartyPhase, HuntingPartyState, NpcGridState, WenyaoHouseholdSlot,
    KNOWN_NPC_NAMES, LIVE_HUNTING_PARTIES, LIVE_NPC_GRID_STATES, LIVE_NPC_OBLIGATIONS,
    OBSERVED_NPCS, PLAYER_HUNTING_PARTY_PLEDGES, WENYAO_HOUSEHOLD_SLOTS,
};
mod event;
mod init_load;
mod item;
mod npc_social;
mod persist;
mod place;
pub use place::PlaceMove;
mod pg_sync;
mod room;
mod rumor;
mod venue;
mod work;

// ── 實體寫回防抖間隔（秒）——Phase 3+ 實作 tokio timer 時使用 ──
#[allow(dead_code)]
const ENTITIES_WRITE_DEBOUNCE_SECS: u64 = 3;

// ══════════════════════════════════════
//  Store 主結構（資料型別見 store::types）
// ══════════════════════════════════════

/// 全域 store：記憶體中的房間、出口、實體、場所等全部資料。
pub struct Store {
    pub rooms: HashMap<String, model::Room>,
    pub exits: HashMap<String, Vec<model::Exit>>,
    pub entity_rooms: HashMap<String, String>,
    pub venues: HashMap<String, Venue>,
    pub assignments: HashMap<String, Vec<Assignment>>,
    pub schedules: HashMap<String, Schedule>,
    pub entities: HashMap<String, Entity>,
    pub place_graph: crate::world::place::PlaceGraph,
    pub items: HashMap<String, Item>,
    pub npc_threads: HashMap<String, NpcThread>,
    pub npc_dyads: HashMap<String, NpcDyad>,
    pub npc_rumors: HashMap<String, NpcRumor>,
    pub npc_rumor_digest: Option<NpcRumorDigest>,
    pub db_pool: Option<sql::DbPool>,
    // 路徑
    rooms_path: String,
    #[allow(dead_code)]
    runtime_dir: String,
    entity_rooms_path: PathBuf,
    venues_path: PathBuf,
    assignments_path: PathBuf,
    schedules_path: PathBuf,
    entities_path: PathBuf,
    items_path: PathBuf,
    npc_thread_path: PathBuf,
    npc_dyad_path: PathBuf,
    npc_rumor_path: PathBuf,
    npc_rumor_digest_path: PathBuf,
}

/// 全域 store 實例（RwLock 保護）。
pub static DEFAULT: RwLock<Option<Arc<RwLock<Store>>>> = RwLock::new(None);

/// 取得全域 store 的 Arc 參照。
pub fn get_store() -> Option<Arc<RwLock<Store>>> {
    DEFAULT.read().clone()
}

pub fn get_db_pool() -> Option<sql::DbPool> {
    if let Some(p) = crate::pg::pool() {
        return Some((*p).clone());
    }
    if let Some(st) = get_store() {
        st.read().db_pool.clone()
    } else {
        None
    }
}

/// 把已驗證的白塔半島 runtime seed 採用為唯一可行走世界，並重載快取。
pub fn activate_white_tower_runtime()
-> anyhow::Result<crate::pg::white_tower_runtime::RuntimeMigrationReport> {
    let store = get_store().ok_or_else(|| anyhow::anyhow!("store 尚未初始化"))?;
    let pool = store
        .read()
        .db_pool
        .clone()
        .ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
    anyhow::ensure!(
        crate::pg::writer::drain(std::time::Duration::from_secs(10)),
        "啟用白塔半島前 PostgreSQL writer 未能清空"
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let report = crate::pg::white_tower_runtime::ensure_and_migrate(&pool, now)?;
    let mut guard = store.write();
    anyhow::ensure!(guard.load_entities_from_pg(), "白塔半島遷移後無法重載人物");
    guard.reload_place_graph()?;
    Ok(report)
}

/// 新人物若還沒有正式位置，出生於白塔半島的正式起點。
pub fn ensure_entity_in_white_tower(entity_id: &str) -> anyhow::Result<String> {
    let world = crate::world::runtime_world::builtin_white_tower_runtime_world()?;
    let store = get_store().ok_or_else(|| anyhow::anyhow!("store 尚未初始化"))?;
    {
        let guard = store.read();
        if let Some(place_id) = guard.get_entity_place_id(entity_id)
            && guard
                .get_place(&place_id)
                .is_some_and(|place| place.source_kind == "azgaar_runtime" && place.traversable)
        {
            return Ok(place_id);
        }
    }
    let pool = store
        .read()
        .db_pool
        .clone()
        .ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    crate::pg::white_tower_runtime::capture_entity_backup_if_missing(&pool, entity_id, now)?;
    let mut guard = store.write();
    guard.set_entity_place(entity_id, &world.spawn_place_id)?;
    Ok(world.spawn_place_id)
}

/// 設定全域 store。
fn set_store(store: Store) {
    let mut guard = DEFAULT.write();
    *guard = Some(Arc::new(RwLock::new(store)));
}

// ══════════════════════════════════════
//  Init — 從 JSON 載入所有資料
// ══════════════════════════════════════

/// 從 rooms_path 載入房間與出口，從 runtime_dir 載入 entity_rooms；
/// 若 data_dir 非空則再載入 venues/assignments/schedules/entities/items/event_log/auth 等。
/// 完成後設定全域 DEFAULT。
pub fn init(rooms_path: &str, runtime_dir: &str, data_dir: &str) -> anyhow::Result<()> {
    let runtime = PathBuf::from(runtime_dir);
    let data = PathBuf::from(data_dir);

    let mut s = Store {
        rooms: HashMap::new(),
        exits: HashMap::new(),
        entity_rooms: HashMap::new(),
        venues: HashMap::new(),
        assignments: HashMap::new(),
        schedules: HashMap::new(),
        entities: HashMap::new(),
        place_graph: crate::world::place::PlaceGraph::default(),
        items: HashMap::new(),
        npc_threads: HashMap::new(),
        npc_dyads: HashMap::new(),
        npc_rumors: HashMap::new(),
        npc_rumor_digest: None,
        db_pool: None,
        rooms_path: rooms_path.to_string(),
        runtime_dir: runtime_dir.to_string(),
        entity_rooms_path: runtime.join("entity_rooms.json"),
        venues_path: data.join("venues.json"),
        assignments_path: data.join("assignments.json"),
        schedules_path: data.join("schedules.json"),
        entities_path: data.join("entities.json"),
        items_path: data.join("items.json"),
        npc_thread_path: runtime.join("npc_thread.json"),
        npc_dyad_path: runtime.join("npc_dyad.json"),
        npc_rumor_path: runtime.join("npc_rumors.json"),
        npc_rumor_digest_path: runtime.join("npc_rumor_digest.json"),
    };

    // 1. 房間始終從檔案系統載入（編輯器工作流）
    s.load_rooms(rooms_path)?;

    // 2. 初始化 PostgreSQL 連線池
    let pg_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:singularity@localhost:5432/singularity".to_string()
    });

    s.db_pool = match sql::init_pool(&pg_url) {
        Ok(pool) => {
            tracing::info!("[store] PostgreSQL 連線池已初始化: {}", pg_url);
            let topology = crate::pg::world_topology::ensure_builtin(&pool)?;
            tracing::info!(
                "[world] 已載入 {} v{}：{} nodes, {} links（PostgreSQL 主資料）",
                topology.world_id,
                topology.version,
                topology.node_count,
                topology.link_count
            );
            let places = crate::pg::world_place::ensure_topology_places(&pool)?;
            crate::pg::world_journey::ensure_schema(&pool)?;
            crate::pg::economy::ensure_schema(&pool)?;
            crate::pg::living_world::ensure_schema(&pool)?;
            tracing::info!(
                "[world] 已把 {} 個白塔拓撲節點登記為不可行走的正典場所容器",
                places.topology_places
            );
            // 同步灌給全域 pg:: 模組——繞過 store lock 的直接 IO 路徑用
            crate::pg::set_pool(pool.clone());
            Some(pool)
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "PostgreSQL 主資料不可用，拒絕啟動平行 JSON 世界：{e}"
            ));
        }
    };

    // 3. 優先從 PostgreSQL 載入；若 DB 為空則從 JSON 種子灌入
    let pg_loaded = s.load_entities_from_pg();

    if pg_loaded {
        tracing::info!("[store] 資料從 PostgreSQL 載入（主資料源）");
        // entity_rooms 需驗證房間存在性（房間可能被編輯器刪除）
        let fallback = s.get_room_id_by_name(crate::db::SPAWN_ROOM_NAME);
        let fallback = if fallback.is_empty() {
            s.first_room_id_sorted()
        } else {
            fallback
        };
        let rooms_ref = &s.rooms;
        s.entity_rooms.retain(|_, rid| {
            if rooms_ref.contains_key(rid.as_str()) {
                true
            } else if !fallback.is_empty() {
                *rid = fallback.clone();
                true
            } else {
                false
            }
        });
        let n = s.ensure_p0_material_items();
        if n > 0 {
            tracing::info!("[store] P0 世界物質補入 items 快取: {n}");
        }
        let n = s.normalize_legacy_material_inventories();
        if n > 0 {
            tracing::info!("[store] 舊物品背包升級為 P0 世界物質: {n} entities");
        }
    } else {
        tracing::info!("[store] PostgreSQL 為空或不可用，從 JSON 種子載入...");
        let _ = s.load_entity_rooms();
        if !data_dir.is_empty() {
            let _ = s.load_venues();
            let _ = s.load_assignments();
            let _ = s.load_schedules();
            let _ = s.load_entities();
            let _ = s.load_items();
            let _ = s.load_npc_threads();
            let _ = s.load_npc_dyads();
        }
        let n = s.ensure_p0_material_items();
        if n > 0 {
            tracing::info!("[store] P0 世界物質補入 items 種子: {n}");
        }
        let n = s.normalize_legacy_material_inventories();
        if n > 0 {
            tracing::info!("[store] 舊物品背包升級為 P0 世界物質: {n} entities");
        }
        // 種子資料灌入 DB
        if let Err(e) = s.sync_all_to_postgresql() {
            tracing::error!("[store] 啟動時 PostgreSQL 同步失敗: {}", e);
        }
    }

    // 4. 傳聞：已有 PG-first + JSON fallback 邏輯
    let _ = s.load_npc_rumors();
    let _ = s.load_npc_rumor_digest();
    s.reload_place_graph()?;

    tracing::info!(
        "[store] 載入完成: {} rooms, {} entity_room, {} venues, {} assignments, {} schedules, {} entities, {} items, {} threads, {} dyads, {} rumors",
        s.rooms.len(),
        s.entity_rooms.len(),
        s.venues.len(),
        s.assignments_count(),
        s.schedules.len(),
        s.entities.len(),
        s.items.len(),
        s.npc_threads.len(),
        s.npc_dyads.len(),
        s.npc_rumors.len()
    );

    // 5. 每次啟動時同步房間到 PG（房間從檔案系統載入，需確保 PG 一致）
    if s.db_pool.is_some() {
        if let Err(e) = s.sync_rooms_to_postgresql() {
            tracing::error!("[store] 房間同步至 PostgreSQL 失敗: {}", e);
        }
    }

    set_store(s);
    crate::game::object_ecology::ensure_ecology_items_in_store();
    Ok(())
}

// ══════════════════════════════════════
//  輕量存取器（較肥的 impl Store 區塊：init_load / pg_sync / entity / room / ...）
// ══════════════════════════════════════

impl Store {
    pub fn new_in_memory() -> Self {
        let tmp = std::env::temp_dir().join("singularity_test_store");
        Self {
            rooms: HashMap::new(),
            exits: HashMap::new(),
            entity_rooms: HashMap::new(),
            venues: HashMap::new(),
            assignments: HashMap::new(),
            schedules: HashMap::new(),
            entities: HashMap::new(),
            place_graph: crate::world::place::PlaceGraph::default(),
            items: HashMap::new(),
            npc_threads: HashMap::new(),
            npc_dyads: HashMap::new(),
            npc_rumors: HashMap::new(),
            npc_rumor_digest: None,
            db_pool: None,
            rooms_path: String::new(),
            runtime_dir: String::new(),
            entity_rooms_path: tmp.join("entity_rooms.json"),
            venues_path: tmp.join("venues.json"),
            assignments_path: tmp.join("assignments.json"),
            schedules_path: tmp.join("schedules.json"),
            entities_path: tmp.join("entities.json"),
            items_path: tmp.join("items.json"),
            npc_thread_path: tmp.join("npc_thread.json"),
            npc_dyad_path: tmp.join("npc_dyad.json"),
            npc_rumor_path: tmp.join("npc_rumors.json"),
            npc_rumor_digest_path: tmp.join("npc_rumor_digest.json"),
        }
    }

    pub fn adjacency(&self) -> HashMap<String, Vec<String>> {
        self.exits
            .iter()
            .map(|(from, exs)| {
                (
                    from.clone(),
                    exs.iter().map(|e| e.to_room_id.clone()).collect(),
                )
            })
            .collect()
    }

    pub fn name_map(&self) -> HashMap<String, String> {
        self.rooms
            .iter()
            .map(|(id, r)| (id.clone(), r.name.clone()))
            .collect()
    }

    pub fn zone_map(&self) -> HashMap<String, String> {
        self.rooms
            .iter()
            .map(|(id, r)| (id.clone(), r.zone.clone()))
            .collect()
    }

    pub fn room_tags_map(&self) -> HashMap<String, Vec<String>> {
        self.rooms
            .iter()
            .map(|(id, r)| (id.clone(), r.tags.clone()))
            .collect()
    }

    // ── 子模組速查 ──
    // Entity / entity_rooms 見 store::entity
    // Venue / Work / Item 見 store::venue / store::work / store::item
    // Event Log 見 store::event
    // Auth / Archival 見 store::auth / store::archival
    // NPC Memory / Summaries / Threads / Dyads 見 store::npc_social
    // NPC Rumors 見 store::rumor
    // 持久化備份（atomic_write + persist_*）見 store::persist
}

// ══════════════════════════════════════
//  輔助函式
// ══════════════════════════════════════

/// 雙向 dyad key：idA < idB 排序後以 "|" 連接。
pub fn dyad_key(id_a: &str, id_b: &str) -> String {
    if id_a <= id_b {
        format!("{id_a}|{id_b}")
    } else {
        format!("{id_b}|{id_a}")
    }
}
