// store 模組 — 以 JSON 為唯一數據源的記憶體層（無 DB）。
// 啟動時從指定 JSON 檔與目錄載入全部資料，執行期只讀寫記憶體，必要時原子寫回對應 JSON。
// 對齊 Go store/store.go。
#![allow(clippy::collapsible_if)]

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

use crate::model;

pub mod sql;

// ── 實體寫回防抖間隔（秒）——Phase 3+ 實作 tokio timer 時使用 ──
#[allow(dead_code)]
const ENTITIES_WRITE_DEBOUNCE_SECS: u64 = 3;

// ══════════════════════════════════════
//  資料型別
// ══════════════════════════════════════

/// 場所：id、名稱、room_ids、max_staff。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venue {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub room_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub max_staff: i32,
}

/// 指派：誰、什麼職業、哪個場所。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub entity_id: String,
    pub occupation_id: String,
    pub venue_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub assigned_by: String,
}

/// 排班：工作房、休息房、班次起迄。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub entity_id: String,
    pub work_room: String,
    pub rest_room: String,
    #[serde(default)]
    pub shift_start: i32,
    #[serde(default)]
    pub shift_end: i32,
}

/// 實體（玩家/NPC）供 JSON 背板。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub display_char: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub move_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_y: Option<i32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub walk_or_run: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_started_at: Option<i64>,
    #[serde(default)]
    pub vit: i32,
    #[serde(default)]
    pub qi: i32,
    #[serde(default)]
    pub dex: i32,
    #[serde(default)]
    pub magnesium: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_observed_at: Option<i64>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gender: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soul_seed: Option<i64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub display_title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub activated_nodes: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub equipment_slots: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub inventory: String,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub disposition: i32,
}

/// 物品定義。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub item_type: String,
    #[serde(default)]
    pub weight: f64,
    #[serde(default)]
    pub stackable: i32,
    #[serde(default)]
    pub denomination: i32,
    #[serde(default)]
    pub description: String,
}

/// 事件日誌一筆。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    pub at: i64,
    pub entity_id: String,
    pub event_type: String,
    pub payload: String,
}

/// NPC 長期記憶一條。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalEntry {
    pub entity_id: String,
    pub content: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub created_at: i64,
}

/// NPC 短期記憶：對某位玩家的見面次數與好感度。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMemory {
    pub entity_id: String,
    pub subject_id: String,
    #[serde(default)]
    pub meet_count: i32,
    #[serde(default)]
    pub favorability: i32,
}

/// 兩名 NPC 的短期話題線狀態。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcThread {
    pub thread_key: String,
    #[serde(default)]
    pub topic_type: String,
    #[serde(default)]
    pub phase: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<String>,
    #[serde(default)]
    pub turn_count: i32,
    #[serde(default)]
    pub cooldown_until: i64,
    #[serde(default)]
    pub updated_at: i64,
}

/// 兩名 NPC 的關係狀態。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDyad {
    pub a_id: String,
    pub b_id: String,
    #[serde(default)]
    pub familiarity: i32,
    #[serde(default)]
    pub sentiment: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub updated_at: i64,
}

/// 可衰減的社會傳聞。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcRumor {
    pub id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub room_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zone: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub source_score: i32,
    #[serde(default)]
    pub weight: i32,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub mention_count: i32,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub last_used_at: i64,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub blocked_until: i64,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub penalty_count: i32,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub last_penalty_at: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_penalty_reason: String,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub expires_at: i64,
}

/// 傳聞池批次摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcRumorDigest {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub source_count: i32,
    #[serde(default)]
    pub updated_at: i64,
}

// ── serde 輔助 ──

fn is_zero_i32(v: &i32) -> bool { *v == 0 }
fn is_zero_i64(v: &i64) -> bool { *v == 0 }

// ── JSON 載入/寫出輔助結構 ──

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RoomFileOne {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    zone: String,
    #[serde(default)]
    exits: Vec<ExitOut>,
    #[serde(default)]
    objects: Vec<model::RoomObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ExitOut {
    direction: String,
    to: String,
}

#[derive(Deserialize, Default)]
struct RoomsFile {
    #[serde(default)]
    rooms: Vec<RoomDef>,
    #[serde(default)]
    exits: Vec<ExitDef>,
}

#[derive(Deserialize, Default)]
struct RoomDef {
    id: String,
    name: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    zone: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize, Default)]
struct ExitDef {
    from: String,
    direction: String,
    to: String,
}

#[derive(Serialize, Deserialize, Default)]
struct EntityRoomsFile {
    #[serde(default)]
    entries: Vec<EntityRoomEntry>,
}

#[derive(Serialize, Deserialize)]
struct EntityRoomEntry {
    entity_id: String,
    room_id: String,
}

#[derive(Serialize, Deserialize, Default)]
struct VenuesFile {
    #[serde(default)]
    venues: Vec<Venue>,
}

#[derive(Serialize, Deserialize, Default)]
struct AssignmentsFile {
    #[serde(default)]
    entries: Vec<Assignment>,
}

#[derive(Serialize, Deserialize, Default)]
struct SchedulesFile {
    #[serde(default)]
    entries: Vec<Schedule>,
}

#[derive(Serialize, Deserialize, Default)]
struct EntitiesFile {
    #[serde(default)]
    entities: Vec<Entity>,
}

#[derive(Serialize, Deserialize, Default)]
struct ItemsFile {
    #[serde(default)]
    items: Vec<Item>,
}



#[derive(Serialize, Deserialize, Default)]
struct NpcThreadsFile {
    #[serde(default)]
    entries: Vec<NpcThread>,
}

#[derive(Serialize, Deserialize, Default)]
struct NpcDyadsFile {
    #[serde(default)]
    entries: Vec<NpcDyad>,
}

#[derive(Serialize, Deserialize, Default)]
struct NpcRumorsFile {
    #[serde(default)]
    entries: Vec<NpcRumor>,
}

/// 與 Go `npcRumorDigestFile`、現行 `npc_rumor_digest.json` 包一層 `digest` 對齊。
#[derive(Serialize, Deserialize)]
struct NpcRumorDigestFile {
    digest: NpcRumorDigest,
}

// ══════════════════════════════════════
//  Store 主結構
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
    DEFAULT.read().unwrap().clone()
}

pub fn get_db_pool() -> Option<sql::DbPool> {
    if let Some(st) = get_store() {
        st.read().unwrap().db_pool.clone()
    } else {
        None
    }
}

/// 設定全域 store。
fn set_store(store: Store) {
    let mut guard = DEFAULT.write().unwrap();
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

    s.load_rooms(rooms_path)?;
    let _ = s.load_entity_rooms();
    
    // PostgreSQL 連線字串，優先讀取環境變數，否則使用預設值
    let pg_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:singularity@localhost:5432/singularity".to_string());

    s.db_pool = match sql::init_pool(&pg_url) {
        Ok(pool) => {
            tracing::info!("[store] Initialized PostgreSQL Pool at {}", pg_url);
            Some(pool)
        }
        Err(e) => {
            tracing::error!("[store] Failed to initialize PostgreSQL Pool: {}", e);
            None
        }
    };

    if !data_dir.is_empty() {
        let _ = s.load_venues();
        let _ = s.load_assignments();
        let _ = s.load_schedules();
        let _ = s.load_entities();
        let _ = s.load_items();
        let _ = s.load_npc_threads();
        let _ = s.load_npc_dyads();
        let _ = s.load_npc_rumors();
        let _ = s.load_npc_rumor_digest();
    }

    tracing::info!(
        "[store] loaded: {} rooms, {} entity_room, {} venues, {} assignments, {} schedules, {} entities, {} items",
        s.rooms.len(),
        s.entity_rooms.len(),
        s.venues.len(),
        s.assignments_count(),
        s.schedules.len(),
        s.entities.len(),
        s.items.len()
    );

    // 啟動時同步到 PostgreSQL (確保 AI 批量新增或手動編輯的 JSON 能進入 DB)
    if let Err(e) = s.sync_all_to_postgresql() {
        tracing::error!("[store] Startup PostgreSQL sync failed: {}", e);
    }

    set_store(s);
    Ok(())
}

// ══════════════════════════════════════
//  載入方法
// ══════════════════════════════════════

impl Store {
    fn assignments_count(&self) -> usize {
        self.assignments.values().map(|v| v.len()).sum()
    }

    // ── 房間載入 ──

    fn load_rooms(&mut self, path: &str) -> anyhow::Result<()> {
        let p = Path::new(path);
        if p.is_dir() {
            self.load_rooms_from_dir(p)?;
        } else {
            self.load_rooms_from_file(p)?;
        }
        Ok(())
    }

    fn load_rooms_from_file(&mut self, path: &Path) -> anyhow::Result<()> {
        let raw = fs::read_to_string(path)?;
        let f: RoomsFile = serde_json::from_str(&raw)?;
        let mut name_by_id: HashMap<String, String> = HashMap::new();
        for r in &f.rooms {
            self.rooms.insert(r.id.clone(), model::Room {
                id: r.id.clone(),
                name: r.name.clone(),
                tags: r.tags.clone(),
                zone: r.zone.clone(),
                description: r.description.clone(),
                objects: Vec::new(),
            });
            name_by_id.insert(r.id.clone(), r.name.clone());
        }
        for e in &f.exits {
            let to_name = name_by_id.get(&e.to).cloned().unwrap_or_default();
            self.exits.entry(e.from.clone()).or_default().push(model::Exit {
                direction: e.direction.clone(),
                to_room_id: e.to.clone(),
                to_room_name: to_name,
            });
        }
        self.prune_non_street_rooms(None);
        Ok(())
    }

    fn load_rooms_from_dir(&mut self, dir: &Path) -> anyhow::Result<()> {
        struct FileEntry {
            room: RoomFileOne,
            is_editor: bool,
        }
        let mut list: Vec<FileEntry> = Vec::new();
        Self::walk_json_dir(dir, &mut |path, rel_path| {
            let raw = fs::read_to_string(path)?;
            if let Ok(one) = serde_json::from_str::<RoomFileOne>(&raw) {
                let is_editor = rel_path.starts_with("editor/");
                list.push(FileEntry { room: one, is_editor });
            }
            Ok(())
        })?;

        let mut editor_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut name_by_id: HashMap<String, String> = HashMap::new();

        for entry in &list {
            let one = &entry.room;
            self.rooms.insert(one.id.clone(), model::Room {
                id: one.id.clone(),
                name: one.name.clone(),
                tags: one.tags.clone(),
                zone: one.zone.clone(),
                description: one.description.clone(),
                objects: one.objects.clone(),
            });
            name_by_id.insert(one.id.clone(), one.name.clone());
            if entry.is_editor && !one.id.is_empty() {
                editor_ids.insert(one.id.clone());
            }
        }
        for entry in &list {
            let one = &entry.room;
            for ex in &one.exits {
                let to_name = name_by_id.get(&ex.to).cloned().unwrap_or_default();
                self.exits.entry(one.id.clone()).or_default().push(model::Exit {
                    direction: ex.direction.clone(),
                    to_room_id: ex.to.clone(),
                    to_room_name: to_name,
                });
            }
        }
        self.prune_non_street_rooms(Some(&editor_ids));
        Ok(())
    }

    /// 遞迴掃描 dir 下所有 .json（跳過底線開頭），呼叫 f(path, rel_path)。
    fn walk_json_dir(dir: &Path, f: &mut dyn FnMut(&Path, &str) -> anyhow::Result<()>) -> anyhow::Result<()> {
        fn walk_inner(base: &Path, current: &Path, f: &mut dyn FnMut(&Path, &str) -> anyhow::Result<()>) -> anyhow::Result<()> {
            let entries = match fs::read_dir(current) {
                Ok(e) => e,
                Err(_) => return Ok(()),
            };
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    walk_inner(base, &path, f)?;
                } else if path.extension().is_some_and(|e| e == "json") {
                    let name = path.file_stem().unwrap_or_default().to_string_lossy();
                    if name.starts_with('_') {
                        continue;
                    }
                    let rel = path.strip_prefix(base)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    f(&path, &rel)?;
                }
            }
            Ok(())
        }
        walk_inner(dir, dir, f)
    }

    fn is_street_or_alley(room: &model::Room) -> bool {
        if room.name.contains("大街") || room.name.contains("巷") {
            return true;
        }
        room.tags.iter().any(|t| {
            let lt = t.trim().to_lowercase();
            lt == "street" || lt == "alley"
        })
    }

    fn prune_non_street_rooms(&mut self, exempt_ids: Option<&std::collections::HashSet<String>>) {
        let keep: std::collections::HashSet<String> = self.rooms.iter()
            .filter(|(id, r)| {
                exempt_ids.is_some_and(|e| e.contains(id.as_str()))
                    || Self::is_street_or_alley(r)
            })
            .map(|(id, _)| id.clone())
            .collect();

        self.rooms.retain(|id, _| keep.contains(id));
        self.exits.retain(|id, _| keep.contains(id));
        for exits in self.exits.values_mut() {
            exits.retain(|ex| keep.contains(&ex.to_room_id));
        }
    }

    fn first_room_id_sorted(&self) -> String {
        let mut ids: Vec<&String> = self.rooms.keys().collect();
        ids.sort();
        ids.first().map(|s| s.to_string()).unwrap_or_default()
    }

    // ── entity_rooms ──

    fn load_entity_rooms(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.entity_rooms_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: EntityRoomsFile = serde_json::from_str(&raw)?;
        let fallback = self.first_room_id_sorted();
        for e in f.entries {
            if self.rooms.contains_key(&e.room_id) {
                self.entity_rooms.insert(e.entity_id, e.room_id);
            } else if !fallback.is_empty() {
                self.entity_rooms.insert(e.entity_id, fallback.clone());
            }
        }
        Ok(())
    }

    // ── venues / assignments / schedules ──

    fn load_venues(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.venues_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: VenuesFile = serde_json::from_str(&raw)?;
        for v in f.venues {
            self.venues.insert(v.id.clone(), v);
        }
        Ok(())
    }

    fn load_assignments(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.assignments_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: AssignmentsFile = serde_json::from_str(&raw)?;
        for a in f.entries {
            self.assignments.entry(a.entity_id.clone()).or_default().push(a);
        }
        Ok(())
    }

    fn load_schedules(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.schedules_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: SchedulesFile = serde_json::from_str(&raw)?;
        for s in f.entries {
            self.schedules.insert(s.entity_id.clone(), s);
        }
        Ok(())
    }

    // ── entities / items / event_log / auth ──

    fn load_entities(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.entities_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: EntitiesFile = serde_json::from_str(&raw)?;
        for e in f.entities {
            self.entities.insert(e.id.clone(), e);
        }
        Ok(())
    }

    fn load_items(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.items_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: ItemsFile = serde_json::from_str(&raw)?;
        for it in f.items {
            self.items.insert(it.id.clone(), it);
        }
        Ok(())
    }



    // ── archival / npc_memory / summaries ──
    // SQLite Migrated

    // ── npc threads / dyads / rumors ──

    fn load_npc_threads(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.npc_thread_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: NpcThreadsFile = serde_json::from_str(&raw)?;
        for t in f.entries {
            self.npc_threads.insert(t.thread_key.clone(), t);
        }
        Ok(())
    }

    fn load_npc_dyads(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.npc_dyad_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: NpcDyadsFile = serde_json::from_str(&raw)?;
        for d in f.entries {
            let key = dyad_key(&d.a_id, &d.b_id);
            self.npc_dyads.insert(key, d);
        }
        Ok(())
    }

    fn load_npc_rumors(&mut self) -> anyhow::Result<()> {
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                if let Ok(rows) = conn.query(
                    "SELECT id, text, room_id, zone, source, source_score, weight, mention_count, 
                            last_used_at, blocked_until, penalty_count, last_penalty_at, 
                            last_penalty_reason, updated_at, expires_at FROM npc_rumors",
                    &[]
                ) {
                    for row in rows {
                        let r = NpcRumor {
                            id: row.get(0),
                            text: row.get(1),
                            room_id: row.get(2),
                            zone: row.get(3),
                            source: row.get(4),
                            source_score: row.get(5),
                            weight: row.get(6),
                            mention_count: row.get(7),
                            last_used_at: row.get(8),
                            blocked_until: row.get(9),
                            penalty_count: row.get(10),
                            last_penalty_at: row.get(11),
                            last_penalty_reason: row.get(12),
                            updated_at: row.get(13),
                            expires_at: row.get(14),
                        };
                        self.npc_rumors.insert(r.id.clone(), r);
                    }
                    return Ok(());
                }
            }
        
        // Fallback to JSON
        let raw = match fs::read_to_string(&self.npc_rumor_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: NpcRumorsFile = serde_json::from_str(&raw)?;
        for r in f.entries {
            self.npc_rumors.insert(r.id.clone(), r);
        }
        Ok(())
    }

    fn load_npc_rumor_digest(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.npc_rumor_digest_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        if let Ok(w) = serde_json::from_str::<NpcRumorDigestFile>(&raw) {
            self.npc_rumor_digest = Some(w.digest);
            return Ok(());
        }
        if let Ok(d) = serde_json::from_str::<NpcRumorDigest>(&raw) {
            self.npc_rumor_digest = Some(d);
        }
        Ok(())
    }

    // ══════════════════════════════════════
    //  CRUD 方法 — 房間
    // ══════════════════════════════════════

    pub fn room_ids(&self) -> Vec<String> {
        self.rooms.keys().cloned().collect()
    }

    pub fn get_room(&self, id: &str) -> Option<model::Room> {
        self.rooms.get(id).cloned()
    }

    pub fn get_room_name(&self, id: &str) -> String {
        self.rooms.get(id).map(|r| r.name.clone()).unwrap_or_default()
    }

    pub fn get_room_id_by_name(&self, name: &str) -> String {
        self.rooms.values()
            .find(|r| r.name == name)
            .map(|r| r.id.clone())
            .unwrap_or_default()
    }

    pub fn get_rooms_by_tag(&self, tag: &str) -> Vec<String> {
        let lt = tag.trim().to_lowercase();
        self.rooms.iter()
            .filter(|(_, r)| r.tags.iter().any(|t| t.trim().to_lowercase() == lt))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn get_rooms_by_zone(&self, zone: &str) -> Vec<String> {
        let lz = zone.trim().to_lowercase();
        self.rooms.iter()
            .filter(|(_, r)| r.zone.trim().to_lowercase() == lz)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn get_exits_for_room(&self, from_room_id: &str) -> Vec<model::Exit> {
        self.exits.get(from_room_id).cloned().unwrap_or_default()
    }

    /// 更新房間資料並寫回 JSON 與 PostgreSQL。
    pub fn upsert_room_data(&mut self, room: model::Room, exits: Option<Vec<model::Exit>>) {
        self.upsert_room_data_internal(room, exits, true);
    }

    fn upsert_room_data_internal(&mut self, room: model::Room, exits: Option<Vec<model::Exit>>, sync_graph: bool) {
        if room.id.is_empty() {
            return;
        }
        let id = room.id.clone();
        
        // 1. 更新內存
        self.rooms.insert(id.clone(), room.clone());
        if let Some(exits_val) = exits {
            let enriched: Vec<model::Exit> = exits_val.into_iter()
                .filter(|ex| !ex.direction.is_empty() && !ex.to_room_id.is_empty())
                .map(|mut ex| {
                    if ex.to_room_name.is_empty()
                        && let Some(r) = self.rooms.get(&ex.to_room_id)
                    {
                        ex.to_room_name = r.name.clone();
                    }
                    ex
                })
                .collect();
            self.exits.insert(id.clone(), enriched);
        }

        // 2. 存回 JSON (維持 Editor 相容性)
        let room_json_path = PathBuf::from(&self.rooms_path).join("editor").join(format!("{}.json", id));
        let exits_for_json = self.exits.get(&id).cloned().unwrap_or_default();
        let file_data = RoomFileOne {
            id: id.clone(),
            name: room.name.clone(),
            description: room.description.clone(),
            tags: room.tags.clone(),
            zone: room.zone.clone(),
            exits: exits_for_json.iter().map(|e| ExitOut {
                direction: e.direction.clone(),
                to: e.to_room_id.clone(),
            }).collect(),
            objects: room.objects.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&file_data) {
            if let Err(e) = std::fs::write(&room_json_path, json) {
                tracing::error!("[store] Failed to write room JSON to {:?}: {}", room_json_path, e);
            }
        }

        // 3. 存入 PostgreSQL (統一地基)
        if let Some(pool) = &self.db_pool {
            let mut conn = match pool.get() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("[store] Failed to get DB connection: {}", e);
                    return;
                }
            };
            
            // Upsert Room
            let tags = &room.tags;
            let objects_json = serde_json::to_string(&room.objects).unwrap_or_else(|_| "[]".to_string());
            let res = conn.execute(
                "INSERT INTO rooms (id, name, description, zone, tags, objects)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name,
                    description = EXCLUDED.description,
                    zone = EXCLUDED.zone,
                    tags = EXCLUDED.tags,
                    objects = EXCLUDED.objects",
                &[&room.id, &room.name, &room.description, &room.zone, tags, &objects_json],
            );
            if let Err(e) = res {
                tracing::error!("[store] Failed to upsert room {} to DB: {}", room.id, e);
            }

            // Sync Exits
            let _ = conn.execute("DELETE FROM exits WHERE from_room_id = $1", &[&room.id]);
            for ex in self.exits.get(&id).cloned().unwrap_or_default() {
                let _ = conn.execute(
                    "INSERT INTO exits (from_room_id, direction, to_room_id) VALUES ($1, $2, $3)
                     ON CONFLICT DO NOTHING",
                    &[&room.id, &ex.direction, &ex.to_room_id],
                );
            }
        }

        if sync_graph { crate::db::sync_room_graph_with_store(self); }
    }

    pub fn delete_room_data(&mut self, room_id: &str) {
        if room_id.is_empty() {
            return;
        }
        let id = room_id.to_string();
        self.rooms.remove(&id);
        self.exits.remove(&id);
        for exits in self.exits.values_mut() {
            exits.retain(|ex| ex.to_room_id != id);
        }

        // 刪除 JSON
        let room_json_path = PathBuf::from(&self.rooms_path).join("editor").join(format!("{}.json", id));
        if room_json_path.exists() {
            let _ = std::fs::remove_file(room_json_path);
        }

        // 刪除 PostgreSQL
        if let Some(pool) = &self.db_pool {
            if let Ok(mut conn) = pool.get() {
                let _ = conn.execute("DELETE FROM rooms WHERE id = $1", &[&id]);
                let _ = conn.execute("DELETE FROM exits WHERE from_room_id = $1 OR to_room_id = $1", &[&id]);
            }
        }

        crate::db::sync_room_graph_with_store(self);
    }

    /// 重新命名房間 ID，並級聯更新所有引用（出口、物件）。
    pub fn rename_room(&mut self, old_id: &str, new_id: &str) -> anyhow::Result<()> {
        if self.rooms.contains_key(new_id) {
            anyhow::bail!("new id already exists");
        }
        let Some(room) = self.rooms.get(old_id).cloned() else {
            anyhow::bail!("room not found");
        };
        let exits = self.get_exits_for_room(old_id);

        // 1. 重命名實體 JSON 檔案
        let old_path = PathBuf::from(&self.rooms_path).join("editor").join(format!("{}.json", old_id));
        let new_path = PathBuf::from(&self.rooms_path).join("editor").join(format!("{}.json", new_id));
        if old_path.exists() {
            fs::rename(&old_path, &new_path)?;
        }

        // 2. 移除內存舊資料
        self.rooms.remove(old_id);
        self.exits.remove(old_id);

        // 3. 建立新資料 (這會寫入新 JSON 並 UPSERT DB)
        let mut new_room = room;
        new_room.id = new_id.to_string();
        self.upsert_room_data_internal(new_room, Some(exits), false);

        // 4. 全局級聯更新 (物件引用 & 出口引用)
        let mut to_update = Vec::new();
        
        // 掃描物件
        for (rid, r) in self.rooms.iter_mut() {
            let mut changed = false;
            for obj in &mut r.objects {
                if obj.move_to_room_id == old_id {
                    obj.move_to_room_id = new_id.to_string();
                    changed = true;
                }
                if obj.id == old_id {
                    obj.id = new_id.to_string();
                    changed = true;
                }
            }
            if changed {
                to_update.push(rid.clone());
            }
        }

        // 掃描出口
        for (rid, ex_list) in self.exits.iter_mut() {
            let mut changed = false;
            for ex in ex_list.iter_mut() {
                if ex.to_room_id == old_id {
                    ex.to_room_id = new_id.to_string();
                    changed = true;
                }
            }
            if changed && !to_update.contains(rid) {
                to_update.push(rid.clone());
            }
        }

        // 寫回所有受影響的房間
        for rid in to_update {
            if let Some(r) = self.rooms.get(&rid).cloned() {
                let exs = self.exits.get(&rid).cloned();
                self.upsert_room_data_internal(r, exs, false);
            }
        }

        // 5. 更新 NpcRumors 與 EntityRooms
        for room_id in self.entity_rooms.values_mut() {
            if room_id == old_id {
                *room_id = new_id.to_string();
            }
        }

        // 6. 清理資料庫舊紀錄
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            conn.execute("UPDATE npc_rumors SET room_id = $1 WHERE room_id = $2", &[&new_id, &old_id])?;
            conn.execute("DELETE FROM rooms WHERE id = $1", &[&old_id])?;
            conn.execute("DELETE FROM exits WHERE from_room_id = $1", &[&old_id])?;
        }

        crate::db::sync_room_graph_with_store(self);
        Ok(())
    }

    pub fn reload_rooms(&mut self) -> anyhow::Result<()> {
        self.rooms.clear();
        self.exits.clear();
        let path = self.rooms_path.clone();
        self.load_rooms(&path)?;
        
        // 強制同步到 PostgreSQL
        if let Err(e) = self.sync_all_to_postgresql() {
            tracing::error!("[store] Failed to sync rooms to PostgreSQL during reload: {}", e);
        }

        crate::db::sync_room_graph_with_store(self);
        Ok(())
    }

    /// 將目前內存中所有房間與出口同步到 PostgreSQL (全量更新)
    pub fn sync_all_to_postgresql(&self) -> anyhow::Result<()> {
        let Some(pool) = &self.db_pool else { return Ok(()) };
        let mut conn = pool.get()?;
        let mut trans = conn.transaction()?;
        
        tracing::info!("[store] Syncing all rooms ({}) to PostgreSQL...", self.rooms.len());

        for room in self.rooms.values() {
            let tags = &room.tags;
            let objects_json = serde_json::to_string(&room.objects).unwrap_or_else(|_| "[]".to_string());
            trans.execute(
                "INSERT INTO rooms (id, name, description, zone, tags, objects)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name,
                    description = EXCLUDED.description,
                    zone = EXCLUDED.zone,
                    tags = EXCLUDED.tags,
                    objects = EXCLUDED.objects",
                &[&room.id, &room.name, &room.description, &room.zone, tags, &objects_json],
            )?;

            // Sync Exits
            trans.execute("DELETE FROM exits WHERE from_room_id = $1", &[&room.id])?;
            if let Some(exs) = self.exits.get(&room.id) {
                for ex in exs {
                    trans.execute(
                        "INSERT INTO exits (from_room_id, direction, to_room_id) VALUES ($1, $2, $3)
                         ON CONFLICT DO NOTHING",
                        &[&room.id, &ex.direction, &ex.to_room_id],
                    )?;
                }
            }
        }
        
        trans.commit()?;
        
        tracing::info!("[store] PostgreSQL room sync completed.");
        Ok(())
    }

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

    // ══════════════════════════════════════
    //  CRUD 方法 — entity_rooms
    // ══════════════════════════════════════

    pub fn get_entity_room(&self, entity_id: &str) -> String {
        self.entity_rooms.get(entity_id).cloned().unwrap_or_default()
    }

    pub fn set_entity_room(&mut self, entity_id: &str, room_id: &str) -> anyhow::Result<()> {
        self.entity_rooms.insert(entity_id.to_string(), room_id.to_string());
        self.persist_entity_rooms()
    }

    pub fn entity_ids_in_room(&self, room_id: &str) -> Vec<String> {
        self.entity_rooms.iter()
            .filter(|(_, rid)| rid.as_str() == room_id)
            .map(|(eid, _)| eid.clone())
            .collect()
    }

    pub fn all_entity_ids(&self) -> Vec<String> {
        self.entities.keys().cloned().collect()
    }

    pub fn get_npc_ids_with_room(&self) -> Vec<String> {
        self.entity_rooms.keys()
            .filter(|eid| {
                self.entities.get(eid.as_str())
                    .is_some_and(|e| e.kind == "npc" && e.vit > 0)
            })
            .cloned()
            .collect()
    }

    pub fn get_player_ids_with_room(&self) -> Vec<String> {
        self.entity_rooms.keys()
            .filter(|eid| {
                self.entities.get(eid.as_str())
                    .is_some_and(|e| e.kind == "player")
            })
            .cloned()
            .collect()
    }

    // ══════════════════════════════════════
    //  CRUD 方法 — 實體
    // ══════════════════════════════════════

    pub fn get_entity(&self, id: &str) -> Option<Entity> {
        self.entities.get(id).cloned()
    }

    pub fn put_entity(&mut self, e: Entity) -> anyhow::Result<()> {
        let mut e = e;
        if e.activated_nodes.is_empty() {
            e.activated_nodes = r#"["N000"]"#.to_string();
        }
        if e.inventory.is_empty() {
            e.inventory = "[]".to_string();
        }
        self.entities.insert(e.id.clone(), e);
        self.persist_entities()
    }

    pub fn update_entity(&mut self, id: &str, f: impl FnOnce(&mut Entity)) -> anyhow::Result<()> {
        if let Some(e) = self.entities.get_mut(id) {
            f(e);
            return self.persist_entities();
        }
        Ok(())
    }

    pub fn transfer_magnesium(&mut self, from_id: &str, to_id: &str, amount: i32) -> anyhow::Result<()> {
        if amount <= 0 {
            anyhow::bail!("轉帳鎂數須為正");
        }
        let from_mg = self.entities.get(from_id)
            .ok_or_else(|| anyhow::anyhow!("找不到轉出實體"))?.magnesium;
        let _to = self.entities.get(to_id)
            .ok_or_else(|| anyhow::anyhow!("找不到轉入實體"))?;
        if from_mg < amount {
            anyhow::bail!("鎂不足");
        }
        self.entities.get_mut(from_id).unwrap().magnesium = from_mg - amount;
        let to_mg = self.entities.get(to_id).unwrap().magnesium;
        self.entities.get_mut(to_id).unwrap().magnesium = to_mg + amount;
        self.persist_entities()
    }

    pub fn get_entities_in_box(&self, x_min: i32, x_max: i32, y_min: i32, y_max: i32, kind: &str) -> Vec<Entity> {
        self.entities.values()
            .filter(|e| {
                (kind.is_empty() || e.kind == kind)
                    && e.x >= x_min && e.x <= x_max
                    && e.y >= y_min && e.y <= y_max
            })
            .cloned()
            .collect()
    }

    pub fn get_moving_entity_ids(&self) -> Vec<String> {
        self.entities.iter()
            .filter(|(_, e)| e.move_state == "moving")
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn npc_ids_with_missing_soul_seed(&self) -> Vec<String> {
        self.entities.iter()
            .filter(|(_, e)| e.kind == "npc" && e.soul_seed.is_none())
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn clear_all_entities(&mut self) -> anyhow::Result<()> {
        self.entities.clear();
        self.entity_rooms.clear();
        self.persist_entities()?;
        self.persist_entity_rooms()
    }

    // ══════════════════════════════════════
    //  CRUD 方法 — Venues / Assignments / Schedules
    // ══════════════════════════════════════

    pub fn get_venue(&self, id: &str) -> Option<&Venue> {
        self.venues.get(id)
    }

    pub fn is_room_in_venue(&self, room_id: &str, venue_id: &str) -> bool {
        self.venues.get(venue_id)
            .is_some_and(|v| v.room_ids.iter().any(|r| r == room_id))
    }

    pub fn get_venue_ids_for_room(&self, room_id: &str) -> Vec<String> {
        self.venues.iter()
            .filter(|(_, v)| v.room_ids.iter().any(|r| r == room_id))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn get_venue_max_staff(&self, venue_id: &str, default_max: i32) -> i32 {
        self.venues.get(venue_id)
            .map(|v| if v.max_staff > 0 { v.max_staff } else { default_max })
            .unwrap_or(default_max)
    }

    pub fn get_all_venue_ids(&self) -> Vec<String> {
        self.venues.keys().cloned().collect()
    }

    pub fn get_all_venue_room_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.venues.values()
            .flat_map(|v| v.room_ids.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    pub fn get_assignments_for_entity(&self, entity_id: &str) -> Vec<Assignment> {
        self.assignments.get(entity_id).cloned().unwrap_or_default()
    }

    pub fn get_assignment_count_by_venue(&self, venue_id: &str) -> usize {
        self.assignments.values()
            .flat_map(|v| v.iter())
            .filter(|a| a.venue_id == venue_id)
            .count()
    }

    /// 該場所既有指派中的第一個職業 ID（對齊 Go `GetFirstOccupationIDForVenue`）。
    pub fn get_first_occupation_id_for_venue(&self, venue_id: &str) -> String {
        for list in self.assignments.values() {
            for a in list {
                if a.venue_id == venue_id {
                    return a.occupation_id.clone();
                }
            }
        }
        String::new()
    }

    /// 新增指派；同 entity+occupation+venue 已存在則忽略（對齊 Go `InsertAssignment`）。
    pub fn insert_assignment(&mut self, entity_id: &str, occupation_id: &str, venue_id: &str, assigned_by: &str) -> anyhow::Result<()> {
        let list = self.assignments.entry(entity_id.to_string()).or_default();
        for a in list.iter() {
            if a.occupation_id == occupation_id && a.venue_id == venue_id {
                return Ok(());
            }
        }
        list.push(Assignment {
            entity_id: entity_id.to_string(),
            occupation_id: occupation_id.to_string(),
            venue_id: venue_id.to_string(),
            assigned_by: assigned_by.to_string(),
        });
        self.persist_assignments()
    }

    pub fn remove_assignments_for_entity(&mut self, entity_id: &str) -> anyhow::Result<()> {
        self.assignments.remove(entity_id);
        self.persist_assignments()
    }

    pub fn get_all_schedules(&self) -> Vec<Schedule> {
        self.schedules.values().cloned().collect()
    }

    pub fn get_schedule(&self, entity_id: &str) -> Option<&Schedule> {
        self.schedules.get(entity_id)
    }

    pub fn insert_schedule(&mut self, entity_id: &str, work_room: &str, rest_room: &str, shift_start: i32, shift_end: i32) -> anyhow::Result<()> {
        self.schedules.insert(entity_id.to_string(), Schedule {
            entity_id: entity_id.to_string(),
            work_room: work_room.to_string(),
            rest_room: rest_room.to_string(),
            shift_start,
            shift_end,
        });
        self.persist_schedules()
    }

    pub fn remove_schedule_for_entity(&mut self, entity_id: &str) -> anyhow::Result<()> {
        self.schedules.remove(entity_id);
        self.persist_schedules()
    }

    // ══════════════════════════════════════
    //  CRUD 方法 — Items
    // ══════════════════════════════════════

    pub fn get_item(&self, id: &str) -> Option<&Item> {
        self.items.get(id)
    }

    pub fn put_item(&mut self, it: Item) -> anyhow::Result<()> {
        self.items.insert(it.id.clone(), it);
        self.persist_items()
    }

    // ══════════════════════════════════════
    //  CRUD 方法 — Event Log
    // ══════════════════════════════════════

    pub fn append_event(&mut self, at: i64, entity_id: &str, event_type: &str, payload: &str) -> anyhow::Result<()> {
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            conn.execute(
                "INSERT INTO event_log (entity_id, event_type, payload, created_at) VALUES ($1, $2, $3, $4)",
                &[&entity_id, &event_type, &payload, &at],
            )?;
        }
        Ok(())
    }

    pub fn last_by_entity(&self, entity_id: &str, event_type: &str, at: i64) -> String {
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                let row = conn.query_opt(
                    "SELECT payload FROM event_log 
                     WHERE entity_id = $1 AND event_type = $2 AND created_at <= $3 
                     ORDER BY created_at DESC, id DESC LIMIT 1",
                    &[&entity_id, &event_type, &at]
                );
                if let Ok(Some(row)) = row {
                    return row.get::<_, String>(0);
                }
            }
        String::new()
    }

    pub fn events_in_range(&self, entity_id: &str, from_at: i64, to_at: i64) -> Vec<EventEntry> {
        let mut results = Vec::new();
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                if let Ok(rows) = conn.query(
                    "SELECT created_at, entity_id, event_type, payload FROM event_log 
                     WHERE entity_id = $1 AND created_at >= $2 AND created_at <= $3 
                     ORDER BY created_at ASC, id ASC",
                    &[&entity_id, &from_at, &to_at]
                ) {
                    for row in rows {
                        results.push(EventEntry {
                            at: row.get(0),
                            entity_id: row.get(1),
                            event_type: row.get(2),
                            payload: row.get(3),
                        });
                    }
                }
            }
        results
    }

    /// 由新到舊（對齊 Go `RecentByEntity`）。
    pub fn recent_by_entity(&self, entity_id: &str, n: usize) -> Vec<EventEntry> {
        let mut results = Vec::new();
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                let n_i64 = n as i64;
                if let Ok(rows) = conn.query(
                    "SELECT created_at, entity_id, event_type, payload FROM event_log 
                     WHERE entity_id = $1 
                     ORDER BY created_at DESC, id DESC LIMIT $2",
                    &[&entity_id, &n_i64]
                ) {
                    for row in rows {
                        results.push(EventEntry {
                            at: row.get(0),
                            entity_id: row.get(1),
                            event_type: row.get(2),
                            payload: row.get(3),
                        });
                    }
                }
            }
        results
    }


    // ══════════════════════════════════════
    //  CRUD 方法 — Auth
    // ══════════════════════════════════════

    pub fn set_auth(&mut self, entity_id: &str, password_hash: &str) -> anyhow::Result<()> {
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            conn.execute(
                "INSERT INTO auth (entity_id, password_hash) VALUES ($1, $2) 
                 ON CONFLICT(entity_id) DO UPDATE SET password_hash=EXCLUDED.password_hash",
                &[&entity_id, &password_hash],
            )?;
        }
        Ok(())
    }

    pub fn get_auth(&self, entity_id: &str) -> String {
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                if let Ok(Some(row)) = conn.query_opt("SELECT password_hash FROM auth WHERE entity_id = $1", &[&entity_id]) {
                    return row.get::<_, String>(0);
                }
            }
        String::new()
    }

    // ══════════════════════════════════════
    //  CRUD 方法 — Archival
    // ══════════════════════════════════════

    pub fn append_archival(&mut self, entry: ArchivalEntry) -> anyhow::Result<()> {
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            conn.execute(
                "INSERT INTO archival (entity_id, content, tag, created_at) VALUES ($1, $2, $3, $4)",
                &[&entry.entity_id, &entry.content, &entry.tag, &entry.created_at],
            )?;
        }
        Ok(())
    }

    pub fn get_archival_by_entity(&self, entity_id: &str) -> Vec<ArchivalEntry> {
        let mut results = Vec::new();
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                if let Ok(rows) = conn.query(
                    "SELECT entity_id, content, tag, created_at FROM archival WHERE entity_id = $1 ORDER BY created_at ASC, id ASC",
                    &[&entity_id]
                ) {
                    for row in rows {
                        results.push(ArchivalEntry {
                            entity_id: row.get(0),
                            content: row.get(1),
                            tag: row.get(2),
                            created_at: row.get(3),
                        });
                    }
                }
            }
        results
    }

    pub fn trim_archival_per_entity(&mut self, max: usize) {
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                let max_i64 = max as i64;
                let _ = conn.execute(
                    "DELETE FROM archival WHERE id NOT IN (
                        SELECT id FROM (
                            SELECT id, row_number() OVER (PARTITION BY entity_id ORDER BY created_at DESC, id DESC) as rn
                            FROM archival
                        ) t WHERE rn <= $1
                    )",
                    &[&max_i64],
                );
            }
    }

    // ══════════════════════════════════════
    //  CRUD 方法 — NPC Memory
    // ══════════════════════════════════════

    pub fn get_npc_memory(&self, entity_id: &str, subject_id: &str) -> Option<NpcMemory> {
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                if let Ok(Some(row)) = conn.query_opt(
                    "SELECT meet_count, favorability FROM npc_memories WHERE entity_id = $1 AND subject_id = $2",
                    &[&entity_id, &subject_id]
                ) {
                    return Some(NpcMemory {
                        entity_id: entity_id.to_string(),
                        subject_id: subject_id.to_string(),
                        meet_count: row.get(0),
                        favorability: row.get(1),
                    });
                }
            }
        None
    }

    pub fn record_meet(&mut self, entity_id: &str, subject_id: &str) -> anyhow::Result<()> {
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            conn.execute(
                "INSERT INTO npc_memories (entity_id, subject_id, meet_count) VALUES ($1, $2, 1) 
                 ON CONFLICT(entity_id, subject_id) DO UPDATE SET meet_count = npc_memories.meet_count + 1",
                &[&entity_id, &subject_id],
            )?;
        }
        Ok(())
    }

    pub fn adjust_favorability(&mut self, entity_id: &str, subject_id: &str, delta: i32) -> anyhow::Result<()> {
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            let old_mem = self.get_npc_memory(entity_id, subject_id);
            let old_fav = old_mem.map_or(0, |m| m.favorability);
            let new_fav = (old_fav + delta).clamp(-100, 100);
            
            conn.execute(
                "INSERT INTO npc_memories (entity_id, subject_id, favorability) VALUES ($1, $2, $3)
                 ON CONFLICT(entity_id, subject_id) DO UPDATE SET favorability = $3",
                &[&entity_id, &subject_id, &new_fav],
            )?;
        }
        Ok(())
    }

    // ══════════════════════════════════════
    //  CRUD 方法 — NPC Summaries
    // ══════════════════════════════════════

    pub fn get_npc_summary(&self, entity_id: &str) -> String {
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                if let Ok(Some(row)) = conn.query_opt("SELECT summary FROM npc_summaries WHERE entity_id = $1", &[&entity_id]) {
                    return row.get(0);
                }
            }
        String::new()
    }

    pub fn set_npc_summary(&mut self, entity_id: &str, summary: &str) -> anyhow::Result<()> {
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            conn.execute(
                "INSERT INTO npc_summaries (entity_id, summary) VALUES ($1, $2) 
                 ON CONFLICT(entity_id) DO UPDATE SET summary=EXCLUDED.summary",
                &[&entity_id, &summary],
            )?;
        }
        Ok(())
    }

    pub fn get_npc_npc_summary(&self, id_a: &str, id_b: &str) -> String {
        let key = dyad_key(id_a, id_b);
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                if let Ok(Some(row)) = conn.query_opt("SELECT summary FROM npc_npc_summaries WHERE dyad_key = $1", &[&key]) {
                    return row.get(0);
                }
            }
        String::new()
    }

    pub fn set_npc_npc_summary(&mut self, id_a: &str, id_b: &str, summary: &str) -> anyhow::Result<()> {
        let key = dyad_key(id_a, id_b);
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            conn.execute(
                "INSERT INTO npc_npc_summaries (dyad_key, summary) VALUES ($1, $2) 
                 ON CONFLICT(dyad_key) DO UPDATE SET summary=EXCLUDED.summary",
                &[&key, &summary],
            )?;
        }
        Ok(())
    }

    // ══════════════════════════════════════
    //  CRUD 方法 — NPC Threads
    // ══════════════════════════════════════

    pub fn get_npc_thread(&self, id_a: &str, id_b: &str) -> Option<NpcThread> {
        let key = dyad_key(id_a, id_b);
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                if let Ok(Some(row)) = conn.query_opt(
                    "SELECT topic_type, phase, anchors, turn_count, cooldown_until, updated_at 
                     FROM npc_threads WHERE thread_key = $1",
                    &[&key]
                ) {
                    let anchors_raw: String = row.get(2);
                    let anchors: Vec<String> = serde_json::from_str(&anchors_raw).unwrap_or_default();
                    return Some(NpcThread {
                        thread_key: key,
                        topic_type: row.get(0),
                        phase: row.get(1),
                        anchors,
                        turn_count: row.get(3),
                        cooldown_until: row.get(4),
                        updated_at: row.get(5),
                    });
                }
            }
        self.npc_threads.get(&key).cloned()
    }

    pub fn set_npc_thread(&mut self, id_a: &str, id_b: &str, t: NpcThread) -> anyhow::Result<()> {
        let key = dyad_key(id_a, id_b);
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            let anchors_raw = serde_json::to_string(&t.anchors).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "INSERT INTO npc_threads (thread_key, topic_type, phase, anchors, turn_count, cooldown_until, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT(thread_key) DO UPDATE SET 
                    topic_type=EXCLUDED.topic_type, 
                    phase=EXCLUDED.phase, 
                    anchors=EXCLUDED.anchors, 
                    turn_count=EXCLUDED.turn_count, 
                    cooldown_until=EXCLUDED.cooldown_until, 
                    updated_at=EXCLUDED.updated_at",
                &[&key, &t.topic_type, &t.phase, &anchors_raw, &t.turn_count, &t.cooldown_until, &t.updated_at],
            )?;
        }
        self.npc_threads.insert(key, t);
        self.persist_npc_threads() // Keep JSON as fallback for now
    }

    pub fn delete_npc_thread(&mut self, id_a: &str, id_b: &str) -> anyhow::Result<()> {
        let key = dyad_key(id_a, id_b);
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            conn.execute("DELETE FROM npc_threads WHERE thread_key = $1", &[&key])?;
        }
        self.npc_threads.remove(&key);
        self.persist_npc_threads()
    }

    // ══════════════════════════════════════
    //  CRUD 方法 — NPC Dyads
    // ══════════════════════════════════════

    pub fn get_npc_dyad(&self, id_a: &str, id_b: &str) -> Option<NpcDyad> {
        let key = dyad_key(id_a, id_b);
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                if let Ok(Some(row)) = conn.query_opt(
                    "SELECT a_id, b_id, familiarity, sentiment, tags, updated_at 
                     FROM npc_dyads WHERE dyad_key = $1",
                    &[&key]
                ) {
                    let tags_raw: String = row.get(4);
                    let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
                    return Some(NpcDyad {
                        a_id: row.get(0),
                        b_id: row.get(1),
                        familiarity: row.get(2),
                        sentiment: row.get(3),
                        tags,
                        updated_at: row.get(5),
                    });
                }
            }
        self.npc_dyads.get(&key).cloned()
    }

    pub fn set_npc_dyad(&mut self, id_a: &str, id_b: &str, d: NpcDyad) -> anyhow::Result<()> {
        let key = dyad_key(id_a, id_b);
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            let tags_raw = serde_json::to_string(&d.tags).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "INSERT INTO npc_dyads (dyad_key, a_id, b_id, familiarity, sentiment, tags, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT(dyad_key) DO UPDATE SET 
                    familiarity=EXCLUDED.familiarity, 
                    sentiment=EXCLUDED.sentiment, 
                    tags=EXCLUDED.tags, 
                    updated_at=EXCLUDED.updated_at",
                &[&key, &d.a_id, &d.b_id, &d.familiarity, &d.sentiment, &tags_raw, &d.updated_at],
            )?;
        }
        self.npc_dyads.insert(key, d);
        self.persist_npc_dyads()
    }

    // ══════════════════════════════════════
    //  CRUD 方法 — NPC Rumors
    // ══════════════════════════════════════

    pub fn upsert_npc_rumor(&mut self, r: NpcRumor) -> anyhow::Result<()> {
        let id = r.id.trim().to_string();
        if id.is_empty() {
            return Ok(());
        }
        if let Some(old) = self.npc_rumors.get_mut(&id) {
            if !r.text.trim().is_empty() { old.text.clone_from(&r.text); }
            if !r.room_id.is_empty() { old.room_id.clone_from(&r.room_id); }
            if !r.zone.is_empty() { old.zone.clone_from(&r.zone); }
            if !r.source.is_empty() { old.source.clone_from(&r.source); }
            if r.source_score > 0 { old.source_score = r.source_score; }
            old.weight += r.weight;
            if old.weight < 1 { old.weight = 1; }
            if r.updated_at > 0 { old.updated_at = r.updated_at; }
            if r.expires_at > old.expires_at { old.expires_at = r.expires_at; }
        } else {
            let mut cp = r;
            cp.id.clone_from(&id);
            if cp.weight <= 0 { cp.weight = 1; }
            if cp.source_score <= 0 { cp.source_score = 1; }
            self.npc_rumors.insert(id, cp);
        }
        self.persist_npc_rumors()
    }

    pub fn get_npc_rumor_digest(&self) -> Option<&NpcRumorDigest> {
        self.npc_rumor_digest.as_ref()
    }

    /// 依時間移除過期傳聞並衰減權重／可信度（對齊 Go `DecayNpcRumors`）。
    pub fn decay_npc_rumors(&mut self, now_unix: i64) -> anyhow::Result<()> {
        let mut changed = false;
        let mut to_remove: Vec<String> = self
            .npc_rumors
            .iter()
            .filter(|(_, r)| r.expires_at > 0 && now_unix > r.expires_at)
            .map(|(id, _)| id.clone())
            .collect();
        for id in to_remove.drain(..) {
            self.npc_rumors.remove(&id);
            changed = true;
        }
        for r in self.npc_rumors.values_mut() {
            if r.updated_at > 0 && now_unix - r.updated_at >= 900 && r.weight > 1 {
                r.weight -= 1;
                changed = true;
            }
            if r.last_used_at > 0 && now_unix - r.last_used_at >= 3600 {
                if r.source_score > 1 {
                    r.source_score -= 1;
                    changed = true;
                } else if r.weight > 1 {
                    r.weight -= 1;
                    changed = true;
                }
            }
        }
        if changed {
            self.persist_npc_rumors()?;
        }
        Ok(())
    }

    /// 由目前傳聞池 top 項組一句摘要並寫入 digest 檔（對齊 Go `BuildNpcRumorDigest`）。
    pub fn build_npc_rumor_digest(&mut self, now_unix: i64) -> anyhow::Result<()> {
        let top = self.top_npc_rumors("", "", now_unix, 5);
        if top.is_empty() {
            return Ok(());
        }
        let mut parts: Vec<String> = Vec::new();
        for t in top.iter().take(3) {
            let x = t.text.trim();
            if x.is_empty() {
                continue;
            }
            parts.push(Self::trunc_to_runes(x, 16));
        }
        if parts.is_empty() {
            return Ok(());
        }
        let text = format!("近日鎮上：{}", parts.join("；"));
        self.npc_rumor_digest = Some(NpcRumorDigest {
            text,
            source_count: top.len() as i32,
            updated_at: now_unix,
        });
        self.persist_npc_rumor_digest()
    }

    /// 正規化傳聞文本鍵（對齊 Go `canonicalRumorText`：小寫、去空白）。
    fn canonical_rumor_text(text: &str) -> String {
        text.trim().to_lowercase()
    }

    fn trunc_to_runes(s: &str, max_runes: usize) -> String {
        let ch: Vec<char> = s.chars().collect();
        if ch.len() <= max_runes {
            return s.to_string();
        }
        let head: String = ch[..max_runes].iter().collect();
        head + "…"
    }

    /// 取指定 room/zone 最相關 topK 傳聞（對齊 Go `TopNpcRumors`）。
    #[must_use]
    pub fn top_npc_rumors(&self, room_id: &str, zone: &str, now_unix: i64, top_k: i32) -> Vec<NpcRumor> {
        if top_k <= 0 {
            return Vec::new();
        }
        let top_k = top_k as usize;
        let mut list: Vec<NpcRumor> = self
            .npc_rumors
            .values()
            .filter(|r| !r.text.trim().is_empty())
            .filter(|r| r.expires_at == 0 || now_unix <= r.expires_at)
            .filter(|r| r.blocked_until == 0 || now_unix > r.blocked_until)
            .cloned()
            .map(|mut r| {
                let mut score = r.weight + r.source_score;
                match r.source.as_str() {
                    "job" => score += 3,
                    "room_event" => score += 2,
                    "spawn" => score += 1,
                    "economy" => {}
                    _ => {}
                }
                if !room_id.is_empty() && r.room_id == room_id {
                    score += 5;
                }
                if !zone.is_empty() && r.zone == zone {
                    score += 2;
                }
                r.weight = score;
                r
            })
            .collect();
        list.sort_by(|a, b| match b.weight.cmp(&a.weight) {
            Ordering::Equal => b.updated_at.cmp(&a.updated_at),
            o => o,
        });
        let mut selected: Vec<NpcRumor> = Vec::new();
        let mut used_source: HashMap<String, i32> = HashMap::new();
        let mut used_text: HashMap<String, bool> = HashMap::new();
        let mut rest: Vec<NpcRumor> = Vec::new();
        for it in list {
            let key = Self::canonical_rumor_text(&it.text);
            if key.is_empty() || used_text.get(&key).copied().unwrap_or(false) {
                continue;
            }
            let src = it.source.trim().to_string();
            if !src.is_empty() && used_source.get(&src).copied().unwrap_or(0) >= 1 {
                rest.push(it);
                continue;
            }
            used_text.insert(key, true);
            if !src.is_empty() {
                *used_source.entry(src).or_default() += 1;
            }
            selected.push(it);
            if selected.len() >= top_k {
                return selected;
            }
        }
        for it in rest {
            let key = Self::canonical_rumor_text(&it.text);
            if key.is_empty() || used_text.get(&key).copied().unwrap_or(false) {
                continue;
            }
            used_text.insert(key, true);
            selected.push(it);
            if selected.len() >= top_k {
                break;
            }
        }
        selected
    }

    /// 標記傳聞被引用（對齊 Go `MarkRumorUsedByText`）。
    pub fn mark_rumor_used_by_text(&mut self, text: &str, now_unix: i64) -> anyhow::Result<()> {
        let key = Self::canonical_rumor_text(text);
        if key.is_empty() {
            return Ok(());
        }
        let mut changed = false;
        for r in self.npc_rumors.values_mut() {
            if Self::canonical_rumor_text(&r.text) != key {
                continue;
            }
            r.mention_count += 1;
            r.last_used_at = now_unix;
            if r.mention_count % 3 == 0 && r.weight < 20 {
                r.weight += 1;
            }
            if r.mention_count % 5 == 0 && r.source_score < 8 {
                r.source_score += 1;
            }
            changed = true;
            break;
        }
        if changed {
            self.persist_npc_rumors()?;
        }
        Ok(())
    }

    /// 衝突降權（對齊 Go `PenalizeRumorByText`）。
    pub fn penalize_rumor_by_text(
        &mut self,
        text: &str,
        now_unix: i64,
        reason: &str,
    ) -> anyhow::Result<()> {
        let key = Self::canonical_rumor_text(text);
        if key.is_empty() {
            return Ok(());
        }
        let mut changed = false;
        for r in self.npc_rumors.values_mut() {
            if Self::canonical_rumor_text(&r.text) != key {
                continue;
            }
            if r.weight > 1 {
                r.weight -= 2;
                if r.weight < 1 {
                    r.weight = 1;
                }
            }
            if r.source_score > 1 {
                r.source_score -= 1;
            }
            r.blocked_until = now_unix + 900;
            r.updated_at = now_unix;
            r.penalty_count += 1;
            r.last_penalty_at = now_unix;
            r.last_penalty_reason = reason.trim().to_string();
            changed = true;
            break;
        }
        if changed {
            self.persist_npc_rumors()?;
        }
        Ok(())
    }

    // ══════════════════════════════════════
    //  持久化方法（原子寫回 JSON）
    // ══════════════════════════════════════

    fn atomic_write(path: &Path, data: &[u8]) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, data)?;
        if fs::rename(&tmp, path).is_err() {
            let _ = fs::remove_file(&tmp);
        }
        Ok(())
    }

    fn persist_entity_rooms(&self) -> anyhow::Result<()> {
        let entries: Vec<EntityRoomEntry> = self.entity_rooms.iter()
            .map(|(eid, rid)| EntityRoomEntry { entity_id: eid.clone(), room_id: rid.clone() })
            .collect();
        let raw = serde_json::to_string_pretty(&EntityRoomsFile { entries })?;
        Self::atomic_write(&self.entity_rooms_path, raw.as_bytes())
    }

    fn persist_entities(&self) -> anyhow::Result<()> {
        if self.entities_path.as_os_str().is_empty() {
            return Ok(());
        }
        let list: Vec<Entity> = self.entities.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&EntitiesFile { entities: list })?;
        Self::atomic_write(&self.entities_path, raw.as_bytes())
    }

    fn persist_assignments(&self) -> anyhow::Result<()> {
        let entries: Vec<Assignment> = self.assignments.values().flatten().cloned().collect();
        let raw = serde_json::to_string_pretty(&AssignmentsFile { entries })?;
        Self::atomic_write(&self.assignments_path, raw.as_bytes())
    }

    fn persist_schedules(&self) -> anyhow::Result<()> {
        let entries: Vec<Schedule> = self.schedules.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&SchedulesFile { entries })?;
        Self::atomic_write(&self.schedules_path, raw.as_bytes())
    }

    fn persist_items(&self) -> anyhow::Result<()> {
        let items: Vec<Item> = self.items.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&ItemsFile { items })?;
        Self::atomic_write(&self.items_path, raw.as_bytes())
    }

    fn persist_npc_threads(&self) -> anyhow::Result<()> {
        let entries: Vec<NpcThread> = self.npc_threads.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&NpcThreadsFile { entries })?;
        Self::atomic_write(&self.npc_thread_path, raw.as_bytes())
    }

    fn persist_npc_dyads(&self) -> anyhow::Result<()> {
        let entries: Vec<NpcDyad> = self.npc_dyads.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&NpcDyadsFile { entries })?;
        Self::atomic_write(&self.npc_dyad_path, raw.as_bytes())
    }

    fn persist_npc_rumors(&self) -> anyhow::Result<()> {
        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            for r in self.npc_rumors.values() {
                conn.execute(
                    "INSERT INTO npc_rumors (id, text, room_id, zone, source, source_score, weight, mention_count, 
                                            last_used_at, blocked_until, penalty_count, last_penalty_at, 
                                            last_penalty_reason, updated_at, expires_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                     ON CONFLICT(id) DO UPDATE SET 
                        text=EXCLUDED.text, room_id=EXCLUDED.room_id, zone=EXCLUDED.zone, source=EXCLUDED.source, 
                        source_score=EXCLUDED.source_score, weight=EXCLUDED.weight, mention_count=EXCLUDED.mention_count, 
                        last_used_at=EXCLUDED.last_used_at, blocked_until=EXCLUDED.blocked_until, 
                        penalty_count=EXCLUDED.penalty_count, last_penalty_at=EXCLUDED.last_penalty_at, 
                        last_penalty_reason=EXCLUDED.last_penalty_reason, updated_at=EXCLUDED.updated_at, 
                        expires_at=EXCLUDED.expires_at",
                    &[&r.id, &r.text, &r.room_id, &r.zone, &r.source, &r.source_score, &r.weight, &r.mention_count, 
                      &r.last_used_at, &r.blocked_until, &r.penalty_count, &r.last_penalty_at, 
                      &r.last_penalty_reason, &r.updated_at, &r.expires_at],
                )?;
            }
        }
        
        let entries: Vec<NpcRumor> = self.npc_rumors.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&NpcRumorsFile { entries })?;
        Self::atomic_write(&self.npc_rumor_path, raw.as_bytes())
    }

    fn persist_npc_rumor_digest(&self) -> anyhow::Result<()> {
        if self.npc_rumor_digest_path.as_os_str().is_empty() {
            return Ok(());
        }
        let Some(ref d) = self.npc_rumor_digest else {
            return Ok(());
        };
        let raw = serde_json::to_string_pretty(&NpcRumorDigestFile {
            digest: d.clone(),
        })?;
        Self::atomic_write(&self.npc_rumor_digest_path, raw.as_bytes())
    }
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
