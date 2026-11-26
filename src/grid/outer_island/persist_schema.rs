//! 外海大島與全大陸正規化資料庫 DDL、`WorldExtrasBlob` 與提交快照結構。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::types::{
    ContractRecord, CustomForgeOrder, DockCargoOrder, DroppedLootBag, IslandWorldDelta,
    LedgerEntry, PickupRecord, TransitVehicleState,
};

pub const CREATE_NORMALIZED_ISLAND_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS outer_island_world_state (
    state_id SMALLINT PRIMARY KEY,
    persist_seq BIGINT NOT NULL DEFAULT 0,
    state_json TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS oi_world_globals (
    singleton SMALLINT PRIMARY KEY CHECK (singleton = 1),
    persist_seq BIGINT NOT NULL DEFAULT 1,
    last_settled_bell BIGINT NOT NULL DEFAULT 0,
    municipal_treasury_copper BIGINT NOT NULL,
    silver_bank_reserve_copper BIGINT NOT NULL,
    mainland_clearing_copper BIGINT NOT NULL DEFAULT 0,
    ledger_seq_counter BIGINT NOT NULL DEFAULT 0,
    genesis_initialized BOOLEAN NOT NULL DEFAULT true,
    unified_v2 BOOLEAN NOT NULL DEFAULT false,
    harbor_history_v1 BOOLEAN NOT NULL DEFAULT false,
    world_lineage_v2 BOOLEAN NOT NULL DEFAULT false,
    chronicle_gazetteer_v3 BOOLEAN NOT NULL DEFAULT false,
    living_chronicler_v4 BOOLEAN NOT NULL DEFAULT false,
    rural_v5 BOOLEAN NOT NULL DEFAULT false,
    genesis_total_copper BIGINT NOT NULL DEFAULT 0,
    extras_json TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE oi_world_globals ADD COLUMN IF NOT EXISTS unified_v2 BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE oi_world_globals ADD COLUMN IF NOT EXISTS harbor_history_v1 BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE oi_world_globals ADD COLUMN IF NOT EXISTS world_lineage_v2 BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE oi_world_globals ADD COLUMN IF NOT EXISTS chronicle_gazetteer_v3 BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE oi_world_globals ADD COLUMN IF NOT EXISTS living_chronicler_v4 BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE oi_world_globals ADD COLUMN IF NOT EXISTS rural_v5 BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE oi_world_globals ADD COLUMN IF NOT EXISTS genesis_total_copper BIGINT NOT NULL DEFAULT 0;
CREATE TABLE IF NOT EXISTS oi_actors (
    actor_id TEXT PRIMARY KEY,
    is_player BOOLEAN NOT NULL,
    soul_seed BIGINT UNIQUE NOT NULL,
    rev BIGINT NOT NULL DEFAULT 1,
    node_id TEXT NOT NULL,
    copper_val BIGINT NOT NULL DEFAULT 0,
    state_json TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS oi_contracts (
    contract_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    creditor_id TEXT NOT NULL,
    debtor_id TEXT NOT NULL,
    amount_copper BIGINT NOT NULL,
    due_day BIGINT NOT NULL,
    status TEXT NOT NULL,
    note_zh TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS oi_world_ledger (
    seq BIGSERIAL PRIMARY KEY,
    idempotency_key TEXT UNIQUE NOT NULL,
    game_day BIGINT NOT NULL,
    bell_idx SMALLINT NOT NULL,
    kind TEXT NOT NULL,
    debit_actor TEXT NOT NULL,
    credit_actor TEXT NOT NULL,
    amount_copper BIGINT NOT NULL CHECK (amount_copper >= 0),
    summary_zh TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS oi_turning_points_archive (
    actor_id TEXT NOT NULL,
    anchor_event_id TEXT NOT NULL,
    game_day BIGINT NOT NULL,
    lesson_zh TEXT NOT NULL,
    new_goal_tag TEXT NOT NULL,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (actor_id, anchor_event_id)
);
CREATE TABLE IF NOT EXISTS outer_island_world_state_corrupt_bak (
    id BIGSERIAL PRIMARY KEY,
    raw_json TEXT NOT NULL,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct WorldExtrasBlob {
    pub(crate) dropped_loot_by_node: BTreeMap<String, Vec<DroppedLootBag>>,
    pub(crate) cleared_nests_until_day: BTreeMap<String, i64>,
    pub(crate) harvested_altars_until_day: BTreeMap<String, i64>,
    pub(crate) sold_houses: BTreeMap<String, String>,
    pub(crate) pickup_ledger: Vec<PickupRecord>,
    pub(crate) contract_escrow_copper: i64,
    pub(crate) chaos_rune_seeded: bool,
    pub(crate) chaos_rune_pool: u16,
    pub(crate) first_spider_slain: Option<(String, String, i64)>,
    pub(crate) built_houses: BTreeMap<String, super::phase_and_vessels::BuiltHomestead>,
    pub(crate) cell_stocks: BTreeMap<u32, u16>,
    pub(crate) cell_last_regen_day: i64,
    pub(crate) slain_apex_beasts: BTreeMap<u32, (String, i64)>,
    pub(crate) transit_vehicles: Vec<TransitVehicleState>,
    pub(crate) dock_cargo_piles: BTreeMap<String, Vec<DockCargoOrder>>,
    pub(crate) forge_orders: BTreeMap<String, Vec<CustomForgeOrder>>,
    pub(crate) survivor_letters_delivered: std::collections::BTreeSet<String>,
    pub(crate) live_land: super::live_land::LiveLandState,
    pub(crate) live_commodities: super::live_commodities::LiveCommoditiesState,
    pub(crate) goblin_hoard_copper: i64,
    pub(crate) live_wilds: super::live_wilds::LiveWildsState,
    pub(crate) live_lifecycle: super::live_lifecycle::LiveLifecycleState,
}

pub enum NormalizedLoadOutcome {
    Loaded(IslandWorldDelta),
    Empty,
    Unavailable,
    Corrupt,
}

#[derive(Debug, Clone)]
pub struct ActorCommitRow {
    pub actor_id: String,
    pub is_player: bool,
    pub soul_seed: i64,
    pub rev_u64: u64,
    pub rev_i64: i64,
    pub node_id: String,
    pub copper: i64,
    pub cold_outbox_flushed: usize,
    pub state_json: String,
}

/// 離鎖提交批次快照（全服全體人物共用單一 `actor_rows` 結構，零玩家／NPC 雙表分流）
pub struct CommitBatch {
    pub next_seq: u64,
    pub last_settled_bell: i64,
    pub genesis_seeded: bool,
    pub genesis_total_copper: i64,
    pub municipal_treasury_copper: i64,
    pub silver_bank_reserve_copper: i64,
    pub mainland_clearing_copper: i64,
    pub ledger_seq_counter: u64,
    pub extras_json: String,
    pub actor_rows: Vec<ActorCommitRow>,
    pub clean_dirty_keys: Vec<String>,
    pub obsolete_actor_ids: Vec<String>,
    pub tp_archives: Vec<(String, super::mind::LifeTurningPoint)>,
    pub contracts: Vec<ContractRecord>,
    pub ledger_entries: Vec<LedgerEntry>,
    pub chronicle_rows: Vec<super::living_chronicler::ChronicleProjectionRow>,
}

/// 原子守恆回收自動化驗收／測試遺留帳號（`codex`、`手機驗收員`），並正規化現存角色 `recent_breaths`（絕不觸碰真實玩家角色）
pub fn purge_test_actor_conserving_copper(delta: &mut IslandWorldDelta, pid: &str) -> bool {
    if !matches!(pid, "codex" | "手機驗收員") {
        return false;
    }
    let aid = delta
        .resolve_controller(pid)
        .unwrap_or_else(|| format!("pl:{pid}"));
    let Some(bal) = delta.actors.get(&aid).map(|a| a.copper) else {
        return false;
    };
    if bal > 0 {
        let _ = delta.transfer_copper(
            0,
            0,
            "qa_cleanup_refund",
            &aid,
            "municipal_treasury",
            bal,
            format!("回收驗收測試帳號 {pid} 銅幣歸還公庫"),
        );
    }
    delta.actors.remove(&aid);
    delta.obsolete_actor_ids.push(aid);
    delta.rebuild_indexes();
    true
}

pub(crate) fn purge_test_actors_and_sanitize_breaths(delta: &mut IslandWorldDelta) -> bool {
    let mut changed = false;
    for test_pid in ["codex", "手機驗收員"] {
        if purge_test_actor_conserving_copper(delta, test_pid) {
            changed = true;
        }
    }
    let player_ids: Vec<String> = delta
        .actors
        .iter()
        .filter(|(_, a)| a.controller_id.is_some())
        .map(|(k, _)| k.clone())
        .collect();
    for aid in player_ids {
        if let Some(a) = delta.actors.get_mut(&aid) {
            let cleaned = super::surface_cues::filter_clean_scene_breaths(&a.recent_breaths, "");
            if a.recent_breaths.len() != cleaned.len() || a.recent_breaths.iter().ne(cleaned.iter())
            {
                a.recent_breaths = cleaned.into_iter().collect();
                a.mark_mutated();
                changed = true;
            }
        }
    }
    changed
}
