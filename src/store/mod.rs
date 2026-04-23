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
mod persist;
mod event;
mod auth;
mod archival;
mod venue;
mod work;
mod item;
mod npc_social;
mod rumor;
mod entity;
mod room;
mod init_load;
mod pg_sync;

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
    if let Some(st) = get_store() {
        st.read().db_pool.clone()
    } else {
        None
    }
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
    let pg_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:singularity@localhost:5432/singularity".to_string());

    s.db_pool = match sql::init_pool(&pg_url) {
        Ok(pool) => {
            tracing::info!("[store] PostgreSQL 連線池已初始化: {}", pg_url);
            // 同步灌給全域 pg:: 模組——繞過 store lock 的直接 IO 路徑用
            crate::pg::set_pool(pool.clone());
            Some(pool)
        }
        Err(e) => {
            tracing::error!("[store] PostgreSQL 連線池初始化失敗: {}", e);
            None
        }
    };

    // 3. 優先從 PostgreSQL 載入；若 DB 為空則從 JSON 種子灌入
    let pg_loaded = s.load_entities_from_pg();

    if pg_loaded {
        tracing::info!("[store] 資料從 PostgreSQL 載入（主資料源）");
        // entity_rooms 需驗證房間存在性（房間可能被編輯器刪除）
        let fallback = s.get_room_id_by_name(crate::db::SPAWN_ROOM_NAME);
        let fallback = if fallback.is_empty() { s.first_room_id_sorted() } else { fallback };
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
        // 種子資料灌入 DB
        if let Err(e) = s.sync_all_to_postgresql() {
            tracing::error!("[store] 啟動時 PostgreSQL 同步失敗: {}", e);
        }
    }

    // 4. 傳聞：已有 PG-first + JSON fallback 邏輯
    let _ = s.load_npc_rumors();
    let _ = s.load_npc_rumor_digest();

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
    Ok(())
}

// ══════════════════════════════════════
//  輕量存取器（較肥的 impl Store 區塊：init_load / pg_sync / entity / room / ...）
// ══════════════════════════════════════

impl Store {
    pub fn adjacency(&self) -> HashMap<String, Vec<String>> {
        self.exits.iter()
            .map(|(from, exs)| (from.clone(), exs.iter().map(|e| e.to_room_id.clone()).collect()))
            .collect()
    }

    pub fn name_map(&self) -> HashMap<String, String> {
        self.rooms.iter().map(|(id, r)| (id.clone(), r.name.clone())).collect()
    }

    pub fn zone_map(&self) -> HashMap<String, String> {
        self.rooms.iter().map(|(id, r)| (id.clone(), r.zone.clone())).collect()
    }

    pub fn room_tags_map(&self) -> HashMap<String, Vec<String>> {
        self.rooms.iter().map(|(id, r)| (id.clone(), r.tags.clone())).collect()
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
