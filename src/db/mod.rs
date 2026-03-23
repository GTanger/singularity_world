// db 模組 — 資料存取門面（讀寫 store），對齊 Go db/ 層。
// store 為唯一資料源；db 提供業務語義的包裝（實體查詢、密碼驗證、soul_seed 展開等）。

use crate::entity::Character;
use crate::store::{self, Entity};

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
    // 簡易 LCG（與 Go math/rand NewSource 相容度足夠，非加密用途）
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
    let s = arc.read().unwrap();
    let Some(se) = s.get_entity(id) else { return Ok(None) };
    let title = ""; // Phase 3+ 補 NPC display name 邏輯
    Ok(Some(store_entity_to_character(&se, title)))
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
pub const SPAWN_ROOM_NAME: &str = "界壁";

/// 取得創生預設房間 id。
pub fn get_spawn_room_id() -> String {
    let Some(arc) = store::get_store() else { return "lobby".to_string() };
    let s = arc.read().unwrap();
    let id = s.get_room_id_by_name(SPAWN_ROOM_NAME);
    if id.is_empty() { "lobby".to_string() } else { id }
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

// ══════════════════════════════════════
//  簡易 LCG 偽隨機（對齊 Go math/rand）
// ══════════════════════════════════════

struct SimpleLcg {
    state: i64,
}

impl SimpleLcg {
    fn new(seed: i64) -> Self {
        Self { state: seed }
    }

    fn next_i64(&mut self) -> i64 {
        // Go math/rand rngSource 使用的是更複雜的演算法，
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
