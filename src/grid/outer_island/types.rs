//! 東南外海大島與全大陸通用靜態地景定義、全服唯一人物結構 `UnifiedMindBody` 與 $O(1)$ 空間索引狀態。

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ops::Index;
use std::sync::{Arc, LazyLock};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::combat::wordplate::splitmix64;

static PROCESS_PUBLIC_TOKEN_SECRET: LazyLock<[u8; 32]> = LazyLock::new(|| {
    let mut hasher = Sha256::new();
    hasher.update(b"singularity_oi_public_token_v2:");
    hasher.update(std::process::id().to_le_bytes());
    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0u128, |d| d.as_nanos());
    hasher.update(now_ns.to_le_bytes());
    hasher.finalize().into()
});

use super::mind::CognitiveMind;
use super::persona::CharacterDriftState;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorldStaticDoc {
    pub schema_version: u32,
    pub seed_hash: String,
    pub world_name: String,
    pub state_name: String,
    pub spawn_node: String,
    pub hidden_entrances: BTreeMap<String, HiddenEntranceDef>,
    pub ledger_genesis: LedgerGenesis,
    pub cells: BTreeMap<String, VoronoiCellDef>,
    pub ring_polygons: Vec<RingPolygonDef>,
    pub highways: Vec<HighwayDef>,
    pub sea_corridors: Vec<SeaCorridorDef>,
    pub town_micro_maps: BTreeMap<String, serde_json::Value>,
    pub nodes: BTreeMap<String, SpatialNodeDef>,
    pub households: Vec<HouseholdDef>,
    pub residents: Vec<ResidentSeedDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LedgerGenesis {
    pub municipal_treasury_copper: i64,
    pub silver_bank_reserve_copper: i64,
    pub resident_copper_sum: i64,
    pub total_conserved_copper: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HiddenEntranceDef {
    pub target: String,
    pub dir: String,
    pub to_name: String,
    pub rumor_key: String,
    pub clue_text: String,
    pub found_text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VoronoiCellDef {
    pub cell_id: u32,
    pub center: [f64; 2],
    pub polygon: Vec<[f64; 2]>,
    pub height: i32,
    pub biome: u32,
    pub flux: i32,
    pub is_coast: bool,
    pub kind: String,
    pub title: String,
    pub desc: String,
    pub neighbors_f6: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RingPolygonDef {
    pub cell_id: u32,
    pub polygon: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HighwayDef {
    pub route_id: u32,
    pub group: String,
    pub points: Vec<[f64; 2]>,
    pub cells: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SeaCorridorDef {
    pub route_id: u32,
    pub crosses_strait: bool,
    pub points: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpatialNodeDef {
    pub id: String,
    pub cell_id: u32,
    pub name: String,
    pub kind: String,
    pub desc: String,
    pub pos: [f64; 2],
    pub indoor: bool,
    pub dungeon: bool,
    pub exits: Vec<NodeExitDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeExitDef {
    pub to: String,
    pub dir: String,
    pub to_name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HouseholdDef {
    pub id: String,
    pub cell_id: u32,
    pub settlement: String,
    pub node_id: String,
    pub label: String,
    pub vacant_for_sale: bool,
    pub price_copper: i64,
    pub members: Vec<String>,
}

/// 僅供創世建檔（Genesis）一次性灌入初始人口之種子定義；執行期徹底與此表脫鉤！
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResidentSeedDef {
    pub id: String,
    pub name: String,
    pub race: String,
    pub role: String,
    pub household_id: String,
    pub home_node: String,
    pub work_node: String,
    pub soul_seed: u64,
    pub copper: i64,
    pub languages: BTreeMap<String, u8>,
    pub persona: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoundTool {
    pub slug: String,
    pub name: String,
    pub provenance: String,
    pub wear_stage: u8,
    pub grip_familiarity: u16,
    pub rune_mark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CarryItem {
    pub slug: String,
    pub name: String,
    pub count: u32,
    pub bound_owner: Option<String>,
    pub stolen_from: Option<String>,
    pub weight_units: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkirmishState {
    pub foe_slug: String,
    pub foe_name: String,
    pub can_speak_goblin: bool,
    pub foe_stagger: u8,
    pub player_guard: bool,
    pub round_idx: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransitState {
    pub line_name: String,
    pub route_nodes: Vec<String>,
    pub step_idx: usize,
    pub started_unix: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DroppedLootBag {
    pub owner_id: String,
    pub copper: i64,
    pub items: Vec<CarryItem>,
    pub promoted_boss_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PickupRecord {
    pub picker_id: String,
    pub original_owner: String,
    pub node_id: String,
    pub summary: String,
    pub declared_to_guard: bool,
}

/// 全世界唯一人物身心與記憶實體（`玩家 ≡ NPC` 100% 同構，僅以 `controller_id` 區分控制插頭！）
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct UnifiedMindBody {
    pub actor_id: String,
    pub controller_id: Option<String>,
    pub soul_seed: u64,
    pub rev: u64,
    pub alive: bool,
    pub setup_done: bool,
    pub display_name: String,
    pub race: String,
    pub role: String,
    pub appearance: String,
    pub origin_pack: String,
    pub household_id: String,
    pub home_node: String,
    pub work_node: String,
    pub node_id: String,
    pub route_plan: VecDeque<String>,
    pub copper: i64,
    pub fatigue: u8,
    pub wound_part: Option<String>,
    pub severe_wrist_warning: bool,
    pub bound_weapon: BoundTool,
    pub backpack: Vec<CarryItem>,
    pub inn_locker: Vec<CarryItem>,
    pub inn_lockers: BTreeMap<String, Vec<CarryItem>>,
    pub inn_nights_left: u32,
    pub owned_house_node: Option<String>,
    pub rescue_pact_active: bool,
    pub conversing_until_ms: u64,
    pub languages: BTreeMap<String, u8>,
    pub lang_daily_gain: BTreeMap<String, u8>,
    pub lang_gain_day: i64,
    pub known_state_cells: BTreeSet<u32>,
    pub unlocked_secrets: BTreeSet<String>,
    pub known_rumors: BTreeSet<String>,
    pub explored_windows: BTreeSet<String>,
    pub recent_breaths: VecDeque<String>,
    pub active_skirmish: Option<SkirmishState>,
    pub active_transit: Option<TransitState>,
    pub last_step_unix_ms: u64,
    pub last_talk_unix_ms: u64,
    pub last_active_unix: i64,
    pub drift: CharacterDriftState,
    pub mind: CognitiveMind,
    pub dirty: bool,
    #[serde(skip)]
    pub flushed_rev: u64,
    #[serde(skip)]
    pub flushed_copper: i64,
}

impl Default for UnifiedMindBody {
    fn default() -> Self {
        Self {
            actor_id: String::new(),
            controller_id: None,
            soul_seed: 0,
            rev: 1,
            alive: true,
            setup_done: false,
            display_name: String::new(),
            race: "人類".to_string(),
            role: "行旅人".to_string(),
            appearance: "披著海風舊斗篷".to_string(),
            origin_pack: "smith_apprentice".to_string(),
            household_id: String::new(),
            home_node: "oi:white_gull_harbor:dock".to_string(),
            work_node: "oi:white_gull_harbor:dock".to_string(),
            node_id: "oi:white_gull_harbor:dock".to_string(),
            route_plan: VecDeque::new(),
            copper: 0,
            fatigue: 0,
            wound_part: None,
            severe_wrist_warning: false,
            bound_weapon: BoundTool {
                slug: "harbor_hammer".to_string(),
                name: "港務舊鍛錘".to_string(),
                provenance: String::new(),
                wear_stage: 0,
                grip_familiarity: 8,
                rune_mark: None,
            },
            backpack: Vec::new(),
            inn_locker: Vec::new(),
            inn_lockers: BTreeMap::new(),
            inn_nights_left: 0,
            owned_house_node: None,
            rescue_pact_active: false,
            conversing_until_ms: 0,
            languages: BTreeMap::new(),
            lang_daily_gain: BTreeMap::new(),
            lang_gain_day: 0,
            known_state_cells: BTreeSet::new(),
            unlocked_secrets: BTreeSet::new(),
            known_rumors: BTreeSet::new(),
            explored_windows: BTreeSet::new(),
            recent_breaths: VecDeque::new(),
            active_skirmish: None,
            active_transit: None,
            last_step_unix_ms: 0,
            last_talk_unix_ms: 0,
            last_active_unix: 0,
            drift: CharacterDriftState::default(),
            mind: CognitiveMind::default(),
            dirty: true,
            flushed_rev: 0,
            flushed_copper: i64::MIN,
        }
    }
}

impl UnifiedMindBody {
    pub fn mark_mutated(&mut self) {
        self.dirty = true;
        self.rev = self.rev.saturating_add(1);
    }

    pub fn is_acquainted_with(&self, target_actor_id: &str) -> bool {
        self.mind
            .ltm
            .social
            .get(target_actor_id)
            .is_some_and(|s| s.acquainted)
    }

    pub fn record_step_visit(&mut self, node_id: &str, cell_id: u32, knows_state: bool) {
        self.mind.record_footstep(node_id, cell_id);
        if knows_state {
            self.known_state_cells.insert(cell_id);
        }
    }

    /// 產生不可逆單向雜湊公開代號（`p:<16hex>`）：同時隱藏底層登入帳號 ID 與個人 `soul_seed`，封死逆推偷讀！
    pub fn public_actor_token(&self) -> String {
        if self.controller_id.is_some() {
            let mut hasher = Sha256::new();
            hasher.update(*PROCESS_PUBLIC_TOKEN_SECRET);
            hasher.update(self.actor_id.as_bytes());
            hasher.update(b":");
            hasher.update(self.soul_seed.to_le_bytes());
            let digest = hasher.finalize();
            let head = u64::from_be_bytes([
                digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6],
                digest[7],
            ]);
            format!("p:{head:016x}")
        } else {
            self.actor_id.clone()
        }
    }
}

/// Copy-on-Write 全服人物表：工作副本 `clone()` 僅遞增 `Arc` 計數（萬人規模 < 0.2ms 零深拷貝），`get_mut` 自動執行 `Arc::make_mut` 並登記 `dirty_keys`！
#[derive(Debug, Clone, Default)]
pub struct ActorMap {
    inner: BTreeMap<String, Arc<UnifiedMindBody>>,
    dirty_keys: BTreeSet<String>,
}

impl Serialize for ActorMap {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.inner.iter().map(|(k, v)| (k, v.as_ref())))
    }
}

impl<'de> Deserialize<'de> for ActorMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = BTreeMap::<String, UnifiedMindBody>::deserialize(deserializer)?;
        let mut inner = BTreeMap::new();
        let mut dirty_keys = BTreeSet::new();
        for (k, v) in raw {
            if v.dirty {
                dirty_keys.insert(k.clone());
            }
            inner.insert(k, Arc::new(v));
        }
        Ok(Self { inner, dirty_keys })
    }
}

impl ActorMap {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn contains_key(&self, k: &str) -> bool {
        self.inner.contains_key(k)
    }

    pub fn get(&self, k: &str) -> Option<&UnifiedMindBody> {
        self.inner.get(k).map(Arc::as_ref)
    }

    pub fn get_mut(&mut self, k: &str) -> Option<&mut UnifiedMindBody> {
        if let Some(arc) = self.inner.get_mut(k) {
            self.dirty_keys.insert(k.to_string());
            Some(Arc::make_mut(arc))
        } else {
            None
        }
    }

    pub fn insert(&mut self, k: String, v: UnifiedMindBody) -> Option<UnifiedMindBody> {
        if v.dirty || v.rev != v.flushed_rev || v.copper != v.flushed_copper {
            self.dirty_keys.insert(k.clone());
        } else {
            self.dirty_keys.remove(&k);
        }
        self.inner.insert(k, Arc::new(v)).map(Arc::unwrap_or_clone)
    }

    pub fn remove(&mut self, k: &str) -> Option<UnifiedMindBody> {
        self.dirty_keys.remove(k);
        self.inner.remove(k).map(Arc::unwrap_or_clone)
    }

    pub fn keys(&self) -> std::collections::btree_map::Keys<'_, String, Arc<UnifiedMindBody>> {
        self.inner.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &UnifiedMindBody> {
        self.inner.values().map(Arc::as_ref)
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut UnifiedMindBody> {
        for k in self.inner.keys() {
            self.dirty_keys.insert(k.clone());
        }
        self.inner.values_mut().map(Arc::make_mut)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &UnifiedMindBody)> {
        self.inner.iter().map(|(k, v)| (k, v.as_ref()))
    }

    pub fn dirty_keys(&self) -> &BTreeSet<String> {
        &self.dirty_keys
    }

    pub fn unmark_dirty_key(&mut self, k: &str) {
        self.dirty_keys.remove(k);
    }
}

impl Index<&str> for ActorMap {
    type Output = UnifiedMindBody;
    fn index(&self, index: &str) -> &Self::Output {
        self.get(index).expect("actor key exists")
    }
}

impl Index<&String> for ActorMap {
    type Output = UnifiedMindBody;
    fn index(&self, index: &String) -> &Self::Output {
        self.get(index.as_str()).expect("actor key exists")
    }
}

impl<'a> IntoIterator for &'a ActorMap {
    type Item = (&'a String, &'a UnifiedMindBody);
    type IntoIter = std::iter::Map<
        std::collections::btree_map::Iter<'a, String, Arc<UnifiedMindBody>>,
        fn((&'a String, &'a Arc<UnifiedMindBody>)) -> (&'a String, &'a UnifiedMindBody),
    >;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter().map(|(k, v)| (k, v.as_ref()))
    }
}

/// 動態誕生或跨海移入新人物之統一規格（與創世種子及玩家創角走同一套 `UnifiedMindBody` 建構流）
#[derive(Debug, Clone)]
pub struct NewActorSpawnSpec {
    pub actor_id: String,
    pub controller_id: Option<String>,
    pub preferred_soul_seed: Option<u64>,
    pub display_name: String,
    pub race: String,
    pub role: String,
    pub appearance: String,
    pub household_id: String,
    pub home_node: String,
    pub work_node: String,
    pub start_node: String,
    pub initial_copper_from_mainland: i64,
    pub bound_weapon: BoundTool,
    pub languages: BTreeMap<String, u8>,
}

/// 外置正式債務與六大關係契約列（獨立於記憶槽外，永不擠爆人際滑窗）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContractRecord {
    pub contract_id: String,
    pub kind: String,
    pub creditor_id: String,
    pub debtor_id: String,
    pub amount_copper: i64,
    pub due_day: i64,
    pub status: String,
    pub note_zh: String,
}

/// 附加式複式簿記分錄（`debit_actor` 扣款 == `credit_actor` 入帳）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LedgerEntry {
    pub idempotency_key: String,
    pub game_day: i64,
    pub bell_idx: u8,
    pub kind: String,
    pub debit_actor: String,
    pub credit_actor: String,
    pub amount_copper: i64,
    pub summary_zh: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct IslandWorldDelta {
    pub persist_seq: u64,
    pub last_settled_bell: i64,
    pub genesis_seeded: bool,
    pub genesis_total_copper: i64,
    pub municipal_treasury_copper: i64,
    pub silver_bank_reserve_copper: i64,
    pub mainland_clearing_copper: i64,
    pub actors: ActorMap,
    pub contracts: BTreeMap<String, ContractRecord>,
    pub pending_ledger: Vec<LedgerEntry>,
    pub ledger_seq_counter: u64,
    pub dropped_loot_by_node: BTreeMap<String, Vec<DroppedLootBag>>,
    pub cleared_nests_until_day: BTreeMap<String, i64>,
    pub harvested_altars_until_day: BTreeMap<String, i64>,
    pub sold_houses: BTreeMap<String, String>,
    pub pickup_ledger: Vec<PickupRecord>,
    #[serde(skip)]
    pub controller_to_actor: BTreeMap<String, String>,
    #[serde(skip)]
    pub public_token_to_actor: BTreeMap<String, String>,
    #[serde(skip)]
    pub node_occupants: BTreeMap<String, BTreeSet<String>>,
    #[serde(skip)]
    pub used_soul_seeds: BTreeSet<i64>,
    #[serde(skip)]
    pub dirty_contracts: BTreeSet<String>,
    #[serde(skip)]
    pub obsolete_actor_ids: Vec<String>,
}

impl IslandWorldDelta {
    /// 全世界單一銅幣守恆驗算：完全不依賴 `doc.residents`，直接加總全體 `UnifiedMindBody.copper`！
    pub fn conservation_diff(&self, doc: &WorldStaticDoc) -> i64 {
        let target_total = if self.genesis_total_copper > 0 {
            self.genesis_total_copper
        } else {
            doc.ledger_genesis.total_conserved_copper
        };
        let actor_total: i64 = self.actors.values().map(|a| a.copper).sum();
        let unseeded_genesis = if self.genesis_seeded {
            0
        } else {
            doc.ledger_genesis.resident_copper_sum
        };
        let loot_total: i64 = self
            .dropped_loot_by_node
            .values()
            .flatten()
            .map(|b| b.copper)
            .sum();
        let now_sum = self.municipal_treasury_copper
            + self.silver_bank_reserve_copper
            + self.mainland_clearing_copper
            + actor_total
            + unseeded_genesis
            + loot_total;
        now_sum - target_total
    }

    /// 重建常駐 $O(1)$ 空間索引、控制插頭索引、公開代號索引與唯一 `soul_seed` 集合
    pub fn rebuild_indexes(&mut self) {
        self.controller_to_actor.clear();
        self.public_token_to_actor.clear();
        self.node_occupants.clear();
        self.used_soul_seeds.clear();
        for (aid, a) in &self.actors {
            if a.soul_seed > 0 {
                let s_i64 = ((a.soul_seed & ((1u64 << 63) - 1)).max(1)) as i64;
                self.used_soul_seeds.insert(s_i64);
            }
            if let Some(ref cid) = a.controller_id {
                self.controller_to_actor
                    .entry(cid.clone())
                    .or_insert_with(|| aid.clone());
                self.public_token_to_actor
                    .insert(a.public_actor_token(), aid.clone());
            }
            if a.alive {
                self.node_occupants
                    .entry(a.node_id.clone())
                    .or_default()
                    .insert(aid.clone());
            }
        }
    }

    /// 唯讀挑選與全宇宙既有實體絕不碰撞之唯一正整數 `soul_seed`（實際佔位統一由 `upsert_actor` 原子寫入，防雙重扣號！）
    pub fn pick_unique_soul_seed(&self, preferred: Option<u64>, tag: &str) -> u64 {
        if let Some(pref) = preferred
            && pref > 0
        {
            let masked = (pref & ((1u64 << 63) - 1)).max(1);
            if !self.used_soul_seeds.contains(&(masked as i64)) {
                return masked;
            }
        }
        let mut h: u64 = 0xCBF2_9CE4_8422_2325;
        for b in tag.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01B3);
        }
        let mut cand = splitmix64(h);
        loop {
            let pos_u64 = (cand & ((1u64 << 63) - 1)).max(10_001);
            if !self.used_soul_seeds.contains(&(pos_u64 as i64)) {
                return pos_u64;
            }
            cand = splitmix64(cand.wrapping_add(1));
        }
    }

    /// 原子插入或更新 `UnifiedMindBody`，單一權威維護三大 $O(1)$ 索引並防止控制權撞名或條碼重抽
    pub fn upsert_actor(&mut self, mut actor: UnifiedMindBody) {
        if let Some(old) = self.actors.remove(&actor.actor_id) {
            if let Some(occ) = self.node_occupants.get_mut(&old.node_id) {
                occ.remove(&actor.actor_id);
                if occ.is_empty() {
                    self.node_occupants.remove(&old.node_id);
                }
            }
            if old.controller_id.is_some() {
                self.public_token_to_actor.remove(&old.public_actor_token());
            }
            if let Some(ref old_c) = old.controller_id
                && self
                    .controller_to_actor
                    .get(old_c)
                    .is_some_and(|v| v == &actor.actor_id)
            {
                self.controller_to_actor.remove(old_c);
            }
            if old.soul_seed > 0 {
                let old_s = ((old.soul_seed & ((1u64 << 63) - 1)).max(1)) as i64;
                self.used_soul_seeds.remove(&old_s);
            }
        }

        let pref = (actor.soul_seed > 0).then_some(actor.soul_seed);
        let chosen_seed = self.pick_unique_soul_seed(pref, &actor.actor_id);
        if actor.soul_seed != 0 && actor.soul_seed != chosen_seed {
            actor.dirty = true;
            actor.rev = actor.rev.saturating_add(1);
        }
        actor.soul_seed = chosen_seed;
        self.used_soul_seeds.insert(chosen_seed as i64);

        if let Some(ref cid) = actor.controller_id {
            if self
                .controller_to_actor
                .get(cid)
                .is_some_and(|existing| existing != &actor.actor_id)
            {
                tracing::warn!(
                    "[outer_island] rejected duplicate controller_id '{}' on actor '{}'",
                    cid,
                    actor.actor_id
                );
                actor.controller_id = None;
            } else {
                self.controller_to_actor
                    .insert(cid.clone(), actor.actor_id.clone());
                self.public_token_to_actor
                    .insert(actor.public_actor_token(), actor.actor_id.clone());
            }
        }
        if actor.alive {
            self.node_occupants
                .entry(actor.node_id.clone())
                .or_default()
                .insert(actor.actor_id.clone());
        }
        self.actors.insert(actor.actor_id.clone(), actor);
    }

    /// 解析前端視圖傳入之公開互動對象代號（`p:<hex>` 或非玩家實體 ID），防止直接猜測帳號名定位他人
    pub fn resolve_public_target(&self, token_or_id: &str) -> Option<String> {
        if let Some(aid) = self.public_token_to_actor.get(token_or_id) {
            return Some(aid.clone());
        }
        if let Some(a) = self.actors.get(token_or_id)
            && a.controller_id.is_none()
        {
            return Some(a.actor_id.clone());
        }
        None
    }

    /// 專供外部操控端（HTTP 玩家請求）以 `controller_id` 進行嚴格 $O(1)$ 解析，絕不回退匹配任意 `actor_id`！
    pub fn resolve_controller(&self, controller_id: &str) -> Option<String> {
        self.controller_to_actor.get(controller_id).cloned()
    }

    pub fn actor_by_controller(&self, controller_id: &str) -> Option<&UnifiedMindBody> {
        let aid = self.controller_to_actor.get(controller_id)?;
        self.actors.get(aid)
    }

    /// 供世界內部規則／測試按 `actor_id` 或 `controller_id` 進行 $O(1)$ 解析
    pub fn resolve_actor_id(&self, id_or_controller: &str) -> Option<String> {
        if self.actors.contains_key(id_or_controller) {
            return Some(id_or_controller.to_string());
        }
        self.controller_to_actor.get(id_or_controller).cloned()
    }

    pub fn actor_by_id_or_controller(&self, key: &str) -> Option<&UnifiedMindBody> {
        if let Some(a) = self.actors.get(key) {
            return Some(a);
        }
        let aid = self.controller_to_actor.get(key)?;
        self.actors.get(aid)
    }

    pub fn actor_mut_by_id_or_controller(&mut self, key: &str) -> Option<&mut UnifiedMindBody> {
        let aid = self.resolve_actor_id(key)?;
        self.actors.get_mut(&aid)
    }

    /// 原子遷移人物所在空間節點並同步更新 `node_occupants` $O(1)$ 索引與個人地圖迷霧
    pub fn relocate_actor(
        &mut self,
        actor_key: &str,
        new_node_id: &str,
        new_cell_id: u32,
        knows_state: bool,
    ) -> bool {
        let Some(aid) = self.resolve_actor_id(actor_key) else {
            return false;
        };
        let (old_node, alive) = {
            let Some(a) = self.actors.get(&aid) else {
                return false;
            };
            (a.node_id.clone(), a.alive)
        };
        if old_node != new_node_id {
            if let Some(occ) = self.node_occupants.get_mut(&old_node) {
                occ.remove(&aid);
                if occ.is_empty() {
                    self.node_occupants.remove(&old_node);
                }
            }
            if alive {
                self.node_occupants
                    .entry(new_node_id.to_string())
                    .or_default()
                    .insert(aid.clone());
            }
        }
        if let Some(a) = self.actors.get_mut(&aid) {
            a.node_id = new_node_id.to_string();
            a.record_step_visit(new_node_id, new_cell_id, knows_state);
            a.mark_mutated();
        }
        true
    }

    fn resolve_ledger_party(&self, raw_key: &str) -> String {
        if matches!(
            raw_key,
            "municipal_treasury" | "silver_bank" | "mainland_clearing"
        ) {
            return raw_key.to_string();
        }
        self.resolve_actor_id(raw_key)
            .unwrap_or_else(|| raw_key.to_string())
    }

    /// 全世界唯一複式銅幣過戶入口（系統三大帳戶字面值最高優先，嚴防註冊同名帳號劫持公庫金流！）
    #[expect(clippy::too_many_arguments, reason = "double-entry transfer context")]
    pub fn transfer_copper(
        &mut self,
        game_day: i64,
        bell_idx: u8,
        kind: &str,
        debit_key: &str,
        credit_key: &str,
        amount: i64,
        summary_zh: String,
    ) -> bool {
        if amount <= 0 {
            return true;
        }
        let debit_resolved = self.resolve_ledger_party(debit_key);
        let credit_resolved = self.resolve_ledger_party(credit_key);
        if debit_resolved == credit_resolved {
            return true;
        }

        // 1. 檢查扣款方餘額（`mainland_clearing` 為跨海清算平衡科目，允許正負對沖）
        match debit_resolved.as_str() {
            "municipal_treasury" => {
                if self.municipal_treasury_copper < amount {
                    return false;
                }
            }
            "silver_bank" => {
                if self.silver_bank_reserve_copper < amount {
                    return false;
                }
            }
            "mainland_clearing" => {}
            other => {
                let Some(src) = self.actors.get(other) else {
                    return false;
                };
                if src.copper < amount {
                    return false;
                }
            }
        }
        // 檢查入帳方存在
        if !matches!(
            credit_resolved.as_str(),
            "municipal_treasury" | "silver_bank" | "mainland_clearing"
        ) && !self.actors.contains_key(&credit_resolved)
        {
            return false;
        }

        // 2. 原子扣款與入帳
        match debit_resolved.as_str() {
            "municipal_treasury" => self.municipal_treasury_copper -= amount,
            "silver_bank" => self.silver_bank_reserve_copper -= amount,
            "mainland_clearing" => self.mainland_clearing_copper -= amount,
            other => {
                if let Some(src) = self.actors.get_mut(other) {
                    src.copper -= amount;
                    src.mark_mutated();
                }
            }
        }
        match credit_resolved.as_str() {
            "municipal_treasury" => self.municipal_treasury_copper += amount,
            "silver_bank" => self.silver_bank_reserve_copper += amount,
            "mainland_clearing" => self.mainland_clearing_copper += amount,
            other => {
                if let Some(dst) = self.actors.get_mut(other) {
                    dst.copper += amount;
                    dst.mark_mutated();
                }
            }
        }

        self.push_ledger_entry(
            game_day,
            bell_idx,
            kind,
            &debit_resolved,
            &credit_resolved,
            amount,
            summary_zh,
        );
        true
    }

    pub fn push_ledger_entry(
        &mut self,
        game_day: i64,
        bell_idx: u8,
        kind: &str,
        debit_actor: &str,
        credit_actor: &str,
        amount_copper: i64,
        summary_zh: String,
    ) {
        if amount_copper <= 0 {
            return;
        }
        self.ledger_seq_counter = self.ledger_seq_counter.saturating_add(1);
        let ikey = format!(
            "oi:{}:{}:{}:{}",
            game_day, bell_idx, kind, self.ledger_seq_counter
        );
        self.pending_ledger.push(LedgerEntry {
            idempotency_key: ikey,
            game_day,
            bell_idx,
            kind: kind.to_string(),
            debit_actor: debit_actor.to_string(),
            credit_actor: credit_actor.to_string(),
            amount_copper,
            summary_zh,
        });
    }
}
