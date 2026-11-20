//! 外海大島與全大陸通用正規化雙速複式帳本、`UnifiedMindBody` `UNNEST` 批次持久化與無損升級引擎。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::store::sql::DbPool;

use super::persist_v1_migrate::{migrate_v1_player_json, migrate_v1_resident_json};
use super::sim_tick::seed_genesis_actors_once;
use super::types::{
    ContractRecord, DroppedLootBag, IslandWorldDelta, LedgerEntry, PickupRecord, ResidentSeedDef,
    UnifiedMindBody, WorldStaticDoc,
};

static NORMALIZED_SCHEMA_READY: AtomicBool = AtomicBool::new(false);
const UNNEST_CHUNK_ROWS: usize = 4_096;

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
    genesis_total_copper BIGINT NOT NULL DEFAULT 0,
    extras_json TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE oi_world_globals ADD COLUMN IF NOT EXISTS unified_v2 BOOLEAN NOT NULL DEFAULT false;
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
struct WorldExtrasBlob {
    dropped_loot_by_node: BTreeMap<String, Vec<DroppedLootBag>>,
    cleared_nests_until_day: BTreeMap<String, i64>,
    harvested_altars_until_day: BTreeMap<String, i64>,
    sold_houses: BTreeMap<String, String>,
    pickup_ledger: Vec<PickupRecord>,
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
}

/// 啟動時自正規化表載入：
/// - `unified_v2 == true`：直接反序列化 `UnifiedMindBody`（零 `doc.residents` 依賴，支援任意動態增長人口）；
/// - `unified_v2 == false`：自動將既有 v1 玩家與居民列無損合流為 `UnifiedMindBody` 並原子回寫升格為 `unified_v2 = true`！
pub fn load_or_migrate_from_pg(doc: &WorldStaticDoc, pool: &DbPool) -> NormalizedLoadOutcome {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(err) => {
            tracing::error!("[outer_island] PG pool unavailable: {err}");
            return NormalizedLoadOutcome::Unavailable;
        }
    };
    if let Err(err) = conn.batch_execute(CREATE_NORMALIZED_ISLAND_SCHEMA) {
        tracing::error!("[outer_island] PG normalized schema init failed: {err}");
        return NormalizedLoadOutcome::Unavailable;
    }
    NORMALIZED_SCHEMA_READY.store(true, Ordering::Release);

    let g_row_opt = match conn.query_opt(
        "SELECT persist_seq, last_settled_bell, municipal_treasury_copper, silver_bank_reserve_copper,
                mainland_clearing_copper, ledger_seq_counter, genesis_initialized, unified_v2,
                genesis_total_copper, extras_json
         FROM oi_world_globals WHERE singleton = 1",
        &[],
    ) {
        Ok(r) => r,
        Err(err) => {
            tracing::error!("[outer_island] PG query oi_world_globals failed: {err}");
            return NormalizedLoadOutcome::Unavailable;
        }
    };

    let Some(g_row) = g_row_opt else {
        return NormalizedLoadOutcome::Empty;
    };

    let persist_seq: i64 = g_row.get(0);
    let last_settled_bell: i64 = g_row.get(1);
    let municipal_treasury_copper: i64 = g_row.get(2);
    let silver_bank_reserve_copper: i64 = g_row.get(3);
    let mainland_clearing_copper: i64 = g_row.get(4);
    let ledger_seq_counter: i64 = g_row.get(5);
    let genesis_init: bool = g_row.get(6);
    let unified_v2: bool = g_row.get(7);
    let genesis_total_col: i64 = g_row.get(8);
    let extras_raw: String = g_row.get(9);

    let extras: WorldExtrasBlob = match serde_json::from_str(&extras_raw) {
        Ok(ex) => ex,
        Err(err) => {
            tracing::error!("[outer_island] corrupt extras_json: {err}");
            return NormalizedLoadOutcome::Corrupt;
        }
    };

    let mut delta = IslandWorldDelta {
        persist_seq: u64::try_from(persist_seq.max(0)).unwrap_or(0),
        last_settled_bell,
        genesis_seeded: genesis_init,
        genesis_total_copper: if genesis_total_col > 0 {
            genesis_total_col
        } else {
            doc.ledger_genesis.total_conserved_copper
        },
        municipal_treasury_copper,
        silver_bank_reserve_copper,
        mainland_clearing_copper,
        ledger_seq_counter: u64::try_from(ledger_seq_counter.max(0)).unwrap_or(0),
        dropped_loot_by_node: extras.dropped_loot_by_node,
        cleared_nests_until_day: extras.cleared_nests_until_day,
        harvested_altars_until_day: extras.harvested_altars_until_day,
        sold_houses: extras.sold_houses,
        pickup_ledger: extras.pickup_ledger,
        ..IslandWorldDelta::default()
    };

    let actor_rows = match conn.query(
        "SELECT actor_id, is_player, soul_seed, rev, copper_val, state_json FROM oi_actors",
        &[],
    ) {
        Ok(rows) => rows,
        Err(err) => {
            tracing::error!("[outer_island] query oi_actors failed: {err}");
            return NormalizedLoadOutcome::Unavailable;
        }
    };

    if unified_v2 {
        for row in actor_rows {
            let aid: String = row.get(0);
            let _is_pl: bool = row.get(1);
            let seed_col: i64 = row.get(2);
            let rev_col: i64 = row.get(3);
            let cval: i64 = row.get(4);
            let sjson: String = row.get(5);
            match serde_json::from_str::<UnifiedMindBody>(&sjson) {
                Ok(mut actor) => {
                    if actor.actor_id.is_empty() {
                        actor.actor_id = aid;
                    }
                    actor.soul_seed = u64::try_from(seed_col.max(1)).unwrap_or(1);
                    actor.rev = actor.rev.max(u64::try_from(rev_col.max(1)).unwrap_or(1));
                    actor.copper = cval;
                    actor.dirty = false;
                    actor.flushed_rev = actor.rev;
                    actor.flushed_copper = cval;
                    delta.upsert_actor(actor);
                }
                Err(err) => {
                    tracing::error!("[outer_island] corrupt v2 actor {aid}: {err}");
                    return NormalizedLoadOutcome::Corrupt;
                }
            }
        }
    } else {
        let seed_by_id: BTreeMap<&str, &ResidentSeedDef> =
            doc.residents.iter().map(|r| (r.id.as_str(), r)).collect();
        for row in actor_rows {
            let aid: String = row.get(0);
            let is_pl: bool = row.get(1);
            let seed_col: i64 = row.get(2);
            let rev_col: i64 = row.get(3);
            let cval: i64 = row.get(4);
            let sjson: String = row.get(5);
            let migrated = if is_pl {
                migrate_v1_player_json(doc, &seed_by_id, &aid, seed_col, rev_col, cval, &sjson)
            } else {
                migrate_v1_resident_json(doc, &seed_by_id, &aid, seed_col, rev_col, cval, &sjson)
            };
            match migrated {
                Ok(actor) => {
                    if delta.actors.contains_key(&actor.actor_id) {
                        tracing::error!(
                            "[outer_island] duplicate actor_id '{}' during v1->v2 migration",
                            actor.actor_id
                        );
                        return NormalizedLoadOutcome::Corrupt;
                    }
                    if aid != actor.actor_id {
                        delta.obsolete_actor_ids.push(aid);
                    }
                    delta.upsert_actor(actor);
                }
                Err(err) => {
                    tracing::error!("[outer_island] v1->v2 migrate failed for {aid}: {err}");
                    return NormalizedLoadOutcome::Corrupt;
                }
            }
        }
        // C3 防呆：即使 v1 舊庫採稀疏落盤（`c_delta == 0` 居民未入表且 `genesis_initialized = true`），
        // 亦強制補齊缺列創世居民，確保 `conservation_diff == 0` 100% 成立！
        delta.genesis_seeded = false;
        seed_genesis_actors_once(doc, &mut delta, false);
        for (_, owner) in &mut delta.sold_houses {
            if !owner.contains(':') && !owner.is_empty() {
                *owner = format!("pl:{owner}");
            }
        }
    }

    let crows = match conn.query(
        "SELECT contract_id, kind, creditor_id, debtor_id, amount_copper, due_day, status, note_zh
         FROM oi_contracts WHERE status <> 'settled'",
        &[],
    ) {
        Ok(rows) => rows,
        Err(err) => {
            tracing::error!("[outer_island] query oi_contracts failed: {err}");
            return NormalizedLoadOutcome::Unavailable;
        }
    };
    for r in crows {
        let cid: String = r.get(0);
        let mut cred: String = r.get(2);
        let mut debt: String = r.get(3);
        if !unified_v2 {
            if !cred.contains(':')
                && !matches!(
                    cred.as_str(),
                    "municipal_treasury" | "silver_bank" | "mainland_clearing"
                )
            {
                cred = format!("pl:{cred}");
            }
            if !debt.contains(':')
                && !matches!(
                    debt.as_str(),
                    "municipal_treasury" | "silver_bank" | "mainland_clearing"
                )
            {
                debt = format!("pl:{debt}");
            }
        }
        delta.contracts.insert(
            cid.clone(),
            ContractRecord {
                contract_id: cid,
                kind: r.get(1),
                creditor_id: cred,
                debtor_id: debt,
                amount_copper: r.get(4),
                due_day: r.get(5),
                status: r.get(6),
                note_zh: r.get(7),
            },
        );
    }

    delta.rebuild_indexes();
    drop(conn);

    // M2 防呆：載入後立即強制驗證全域銅幣守恆，若帳目失衡立即凍結拒絕靜默掉檔！
    let load_diff = delta.conservation_diff(doc);
    if load_diff != 0 {
        tracing::error!(
            "[outer_island] conservation_diff={load_diff} after PG load, freezing as Corrupt"
        );
        return NormalizedLoadOutcome::Corrupt;
    }

    if !unified_v2 {
        tracing::info!(
            "[outer_island] upgrading {} v1 actors into unified_v2 UnifiedMindBody",
            delta.actors.len()
        );
        if let Err(err) = commit_delta_atomic(doc, pool, &mut delta, true) {
            tracing::error!("[outer_island] v1->v2 atomic upgrade commit failed: {err}");
            return NormalizedLoadOutcome::Unavailable;
        }
    }

    NormalizedLoadOutcome::Loaded(delta)
}

/// 在讀鎖內組裝提交快照並先行驗算 `conservation_diff == 0`（全服全體人物單一迴圈，零雙軌分流）
pub fn prepare_commit_batch(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    flush_all_actors: bool,
) -> Result<CommitBatch, String> {
    let diff = delta.conservation_diff(doc);
    if diff != 0 {
        tracing::error!("[outer_island] copper conservation failed before commit: diff={diff}");
        return Err("港務銀冊帳目核對未過，請稍候片刻再試。".to_string());
    }

    let extras = WorldExtrasBlob {
        dropped_loot_by_node: delta.dropped_loot_by_node.clone(),
        cleared_nests_until_day: delta.cleared_nests_until_day.clone(),
        harvested_altars_until_day: delta.harvested_altars_until_day.clone(),
        sold_houses: delta.sold_houses.clone(),
        pickup_ledger: delta.pickup_ledger.clone(),
    };
    let extras_json = serde_json::to_string(&extras).map_err(|e| e.to_string())?;
    let next_seq = delta.persist_seq.saturating_add(1);
    let genesis_total = if delta.genesis_total_copper > 0 {
        delta.genesis_total_copper
    } else {
        doc.ledger_genesis.total_conserved_copper
    };

    let mut actor_rows = Vec::new();
    let mut clean_dirty_keys = Vec::new();
    let mut tp_archives = Vec::new();
    if flush_all_actors {
        for a in delta.actors.values() {
            for tp in &a.mind.ltm.cold_outbox {
                tp_archives.push((a.actor_id.clone(), tp.clone()));
            }
            let seed_i64 = ((a.soul_seed & ((1u64 << 63) - 1)).max(1)) as i64;
            let rev_i64 = i64::try_from(a.rev.max(1)).unwrap_or(1);
            let state_json = serde_json::to_string(a).map_err(|e| e.to_string())?;
            actor_rows.push(ActorCommitRow {
                actor_id: a.actor_id.clone(),
                is_player: a.controller_id.is_some(),
                soul_seed: seed_i64,
                rev_u64: a.rev,
                rev_i64,
                node_id: a.node_id.clone(),
                copper: a.copper,
                cold_outbox_flushed: a.mind.ltm.cold_outbox.len(),
                state_json,
            });
        }
    } else {
        for aid in delta.actors.dirty_keys() {
            let Some(a) = delta.actors.get(aid) else {
                clean_dirty_keys.push(aid.clone());
                continue;
            };
            for tp in &a.mind.ltm.cold_outbox {
                tp_archives.push((a.actor_id.clone(), tp.clone()));
            }
            if !a.dirty
                && a.rev == a.flushed_rev
                && a.copper == a.flushed_copper
                && a.mind.ltm.cold_outbox.is_empty()
            {
                clean_dirty_keys.push(aid.clone());
                continue;
            }
            let seed_i64 = ((a.soul_seed & ((1u64 << 63) - 1)).max(1)) as i64;
            let rev_i64 = i64::try_from(a.rev.max(1)).unwrap_or(1);
            let state_json = serde_json::to_string(a).map_err(|e| e.to_string())?;
            actor_rows.push(ActorCommitRow {
                actor_id: a.actor_id.clone(),
                is_player: a.controller_id.is_some(),
                soul_seed: seed_i64,
                rev_u64: a.rev,
                rev_i64,
                node_id: a.node_id.clone(),
                copper: a.copper,
                cold_outbox_flushed: a.mind.ltm.cold_outbox.len(),
                state_json,
            });
        }
    }

    let contracts: Vec<ContractRecord> = if flush_all_actors {
        delta.contracts.values().cloned().collect()
    } else {
        delta
            .contracts
            .iter()
            .filter(|(cid, c)| delta.dirty_contracts.contains(*cid) || c.status == "settled")
            .map(|(_, c)| c.clone())
            .collect()
    };

    Ok(CommitBatch {
        next_seq,
        last_settled_bell: delta.last_settled_bell,
        genesis_seeded: delta.genesis_seeded,
        genesis_total_copper: genesis_total,
        municipal_treasury_copper: delta.municipal_treasury_copper,
        silver_bank_reserve_copper: delta.silver_bank_reserve_copper,
        mainland_clearing_copper: delta.mainland_clearing_copper,
        ledger_seq_counter: delta.ledger_seq_counter,
        extras_json,
        actor_rows,
        clean_dirty_keys,
        obsolete_actor_ids: delta.obsolete_actor_ids.clone(),
        tp_archives,
        contracts,
        ledger_entries: delta.pending_ledger.clone(),
    })
}

/// 離鎖執行單一 PostgreSQL 原子交易提交（全表採用 `UNNEST` / `ANY` 集合批量寫入，零逐列 `for` 往返）
pub fn execute_commit_batch_pg(pool: &DbPool, batch: &CommitBatch) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    if !NORMALIZED_SCHEMA_READY.load(Ordering::Acquire) {
        conn.batch_execute(CREATE_NORMALIZED_ISLAND_SCHEMA)
            .map_err(|e| e.to_string())?;
        NORMALIZED_SCHEMA_READY.store(true, Ordering::Release);
    }

    let seq_i64 = i64::try_from(batch.next_seq).unwrap_or(i64::MAX);
    let lseq_i64 = i64::try_from(batch.ledger_seq_counter).unwrap_or(i64::MAX);

    let mut tx = conn.transaction().map_err(|e| e.to_string())?;

    let globals_affected = tx
        .execute(
            "INSERT INTO oi_world_globals (
                singleton, persist_seq, last_settled_bell, municipal_treasury_copper,
                silver_bank_reserve_copper, mainland_clearing_copper, ledger_seq_counter,
                genesis_initialized, unified_v2, genesis_total_copper, extras_json, updated_at
            ) VALUES (1, $1, $2, $3, $4, $5, $6, $7, true, $8, $9, now())
            ON CONFLICT (singleton) DO UPDATE SET
                persist_seq = EXCLUDED.persist_seq,
                last_settled_bell = EXCLUDED.last_settled_bell,
                municipal_treasury_copper = EXCLUDED.municipal_treasury_copper,
                silver_bank_reserve_copper = EXCLUDED.silver_bank_reserve_copper,
                mainland_clearing_copper = EXCLUDED.mainland_clearing_copper,
                ledger_seq_counter = EXCLUDED.ledger_seq_counter,
                genesis_initialized = EXCLUDED.genesis_initialized,
                unified_v2 = true,
                genesis_total_copper = EXCLUDED.genesis_total_copper,
                extras_json = EXCLUDED.extras_json,
                updated_at = now()
            WHERE oi_world_globals.persist_seq <= EXCLUDED.persist_seq",
            &[
                &seq_i64,
                &batch.last_settled_bell,
                &batch.municipal_treasury_copper,
                &batch.silver_bank_reserve_copper,
                &batch.mainland_clearing_copper,
                &lseq_i64,
                &batch.genesis_seeded,
                &batch.genesis_total_copper,
                &batch.extras_json,
            ],
        )
        .map_err(|e| e.to_string())?;
    if globals_affected == 0 {
        return Err("stale persist_seq rejected by oi_world_globals".to_string());
    }

    if !batch.obsolete_actor_ids.is_empty() {
        tx.execute(
            "DELETE FROM oi_actors WHERE actor_id = ANY($1::text[])",
            &[&batch.obsolete_actor_ids],
        )
        .map_err(|e| e.to_string())?;
    }

    for chunk in batch.actor_rows.chunks(UNNEST_CHUNK_ROWS) {
        let mut aids = Vec::with_capacity(chunk.len());
        let mut is_pls = Vec::with_capacity(chunk.len());
        let mut seeds = Vec::with_capacity(chunk.len());
        let mut revs = Vec::with_capacity(chunk.len());
        let mut nids = Vec::with_capacity(chunk.len());
        let mut coppers = Vec::with_capacity(chunk.len());
        let mut sjsons = Vec::with_capacity(chunk.len());
        for r in chunk {
            aids.push(r.actor_id.clone());
            is_pls.push(r.is_player);
            seeds.push(r.soul_seed);
            revs.push(r.rev_i64);
            nids.push(r.node_id.clone());
            coppers.push(r.copper);
            sjsons.push(r.state_json.clone());
        }
        let affected = tx
            .execute(
                "INSERT INTO oi_actors (
                    actor_id, is_player, soul_seed, rev, node_id, copper_val, state_json, updated_at
                 )
                 SELECT u.aid, u.isp, u.seed, u.rv, u.nid, u.cop, u.sj, now()
                 FROM UNNEST(
                    $1::text[],
                    $2::bool[],
                    $3::bigint[],
                    $4::bigint[],
                    $5::text[],
                    $6::bigint[],
                    $7::text[]
                 ) AS u(aid, isp, seed, rv, nid, cop, sj)
                 ON CONFLICT (actor_id) DO UPDATE SET
                    is_player = EXCLUDED.is_player,
                    soul_seed = EXCLUDED.soul_seed,
                    rev = EXCLUDED.rev,
                    node_id = EXCLUDED.node_id,
                    copper_val = EXCLUDED.copper_val,
                    state_json = EXCLUDED.state_json,
                    updated_at = now()
                 WHERE oi_actors.rev <= EXCLUDED.rev",
                &[&aids, &is_pls, &seeds, &revs, &nids, &coppers, &sjsons],
            )
            .map_err(|e| e.to_string())?;
        if affected != chunk.len() as u64 {
            return Err(format!(
                "oi_actors rev guard rejected {} of {} rows (atomic rollback)",
                chunk.len() as u64 - affected,
                chunk.len()
            ));
        }
    }

    for chunk in batch.tp_archives.chunks(UNNEST_CHUNK_ROWS) {
        let mut aids = Vec::with_capacity(chunk.len());
        let mut anchors = Vec::with_capacity(chunk.len());
        let mut gdays = Vec::with_capacity(chunk.len());
        let mut lessons = Vec::with_capacity(chunk.len());
        let mut goals = Vec::with_capacity(chunk.len());
        for (aid, tp) in chunk {
            aids.push(aid.clone());
            anchors.push(tp.anchor_event_id.clone());
            gdays.push(tp.game_day);
            lessons.push(tp.lesson_zh.clone());
            goals.push(tp.new_goal_tag.clone());
        }
        tx.execute(
            "INSERT INTO oi_turning_points_archive (
                actor_id, anchor_event_id, game_day, lesson_zh, new_goal_tag, archived_at
             )
             SELECT u.aid, u.anc, u.gd, u.ls, u.gl, now()
             FROM UNNEST(
                $1::text[],
                $2::text[],
                $3::bigint[],
                $4::text[],
                $5::text[]
             ) AS u(aid, anc, gd, ls, gl)
             ON CONFLICT (actor_id, anchor_event_id) DO NOTHING",
            &[&aids, &anchors, &gdays, &lessons, &goals],
        )
        .map_err(|e| e.to_string())?;
    }

    let settled_cids: Vec<String> = batch
        .contracts
        .iter()
        .filter(|c| c.status == "settled")
        .map(|c| c.contract_id.clone())
        .collect();
    if !settled_cids.is_empty() {
        tx.execute(
            "DELETE FROM oi_contracts WHERE contract_id = ANY($1::text[])",
            &[&settled_cids],
        )
        .map_err(|e| e.to_string())?;
    }

    let active_contracts: Vec<&ContractRecord> = batch
        .contracts
        .iter()
        .filter(|c| c.status != "settled")
        .collect();
    for chunk in active_contracts.chunks(UNNEST_CHUNK_ROWS) {
        let mut cids = Vec::with_capacity(chunk.len());
        let mut kinds = Vec::with_capacity(chunk.len());
        let mut creds = Vec::with_capacity(chunk.len());
        let mut debts = Vec::with_capacity(chunk.len());
        let mut amts = Vec::with_capacity(chunk.len());
        let mut dues = Vec::with_capacity(chunk.len());
        let mut stats = Vec::with_capacity(chunk.len());
        let mut notes = Vec::with_capacity(chunk.len());
        for c in chunk {
            cids.push(c.contract_id.clone());
            kinds.push(c.kind.clone());
            creds.push(c.creditor_id.clone());
            debts.push(c.debtor_id.clone());
            amts.push(c.amount_copper);
            dues.push(c.due_day);
            stats.push(c.status.clone());
            notes.push(c.note_zh.clone());
        }
        tx.execute(
            "INSERT INTO oi_contracts (
                contract_id, kind, creditor_id, debtor_id, amount_copper, due_day, status, note_zh, updated_at
             )
             SELECT u.cid, u.knd, u.cr, u.db, u.amt, u.due, u.st, u.nt, now()
             FROM UNNEST(
                $1::text[],
                $2::text[],
                $3::text[],
                $4::text[],
                $5::bigint[],
                $6::bigint[],
                $7::text[],
                $8::text[]
             ) AS u(cid, knd, cr, db, amt, due, st, nt)
             ON CONFLICT (contract_id) DO UPDATE SET
                kind = EXCLUDED.kind,
                creditor_id = EXCLUDED.creditor_id,
                debtor_id = EXCLUDED.debtor_id,
                amount_copper = EXCLUDED.amount_copper,
                due_day = EXCLUDED.due_day,
                status = EXCLUDED.status,
                note_zh = EXCLUDED.note_zh,
                updated_at = now()",
            &[&cids, &kinds, &creds, &debts, &amts, &dues, &stats, &notes],
        )
        .map_err(|e| e.to_string())?;
    }

    for chunk in batch.ledger_entries.chunks(UNNEST_CHUNK_ROWS) {
        let mut ikeys = Vec::with_capacity(chunk.len());
        let mut gdays = Vec::with_capacity(chunk.len());
        let mut bells = Vec::with_capacity(chunk.len());
        let mut kinds = Vec::with_capacity(chunk.len());
        let mut debs = Vec::with_capacity(chunk.len());
        let mut creds = Vec::with_capacity(chunk.len());
        let mut amts = Vec::with_capacity(chunk.len());
        let mut sums = Vec::with_capacity(chunk.len());
        for le in chunk {
            ikeys.push(le.idempotency_key.clone());
            gdays.push(le.game_day);
            bells.push(i16::from(le.bell_idx));
            kinds.push(le.kind.clone());
            debs.push(le.debit_actor.clone());
            creds.push(le.credit_actor.clone());
            amts.push(le.amount_copper);
            sums.push(le.summary_zh.clone());
        }
        tx.execute(
            "INSERT INTO oi_world_ledger (
                idempotency_key, game_day, bell_idx, kind, debit_actor, credit_actor, amount_copper, summary_zh
             )
             SELECT u.ik, u.gd, u.bl, u.kn, u.db, u.cr, u.am, u.sm
             FROM UNNEST(
                $1::text[],
                $2::bigint[],
                $3::smallint[],
                $4::text[],
                $5::text[],
                $6::text[],
                $7::bigint[],
                $8::text[]
             ) AS u(ik, gd, bl, kn, db, cr, am, sm)
             ON CONFLICT (idempotency_key) DO NOTHING",
            &[&ikeys, &gdays, &bells, &kinds, &debs, &creds, &amts, &sums],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// `tx.commit()` 成功後回寫記憶體確認狀態（Commit-Before-Publish，僅清除未被併發更新覆寫之 `dirty`）
pub fn ack_commit_batch(delta: &mut IslandWorldDelta, batch: &CommitBatch) {
    if batch.next_seq > delta.persist_seq {
        delta.persist_seq = batch.next_seq;
    }
    let committed_keys: BTreeSet<&str> = batch
        .ledger_entries
        .iter()
        .map(|e| e.idempotency_key.as_str())
        .collect();
    delta
        .pending_ledger
        .retain(|e| !committed_keys.contains(e.idempotency_key.as_str()));

    let deleted_cids: BTreeSet<&str> = batch
        .contracts
        .iter()
        .filter(|c| c.status == "settled")
        .map(|c| c.contract_id.as_str())
        .collect();
    delta
        .contracts
        .retain(|cid, _| !deleted_cids.contains(cid.as_str()));
    for c in &batch.contracts {
        delta.dirty_contracts.remove(&c.contract_id);
    }
    delta.obsolete_actor_ids.clear();

    for ck in &batch.clean_dirty_keys {
        if delta.actors.get(ck).is_none_or(|a| {
            !a.dirty
                && a.rev == a.flushed_rev
                && a.copper == a.flushed_copper
                && a.mind.ltm.cold_outbox.is_empty()
        }) {
            delta.actors.unmark_dirty_key(ck);
        }
    }

    for row in &batch.actor_rows {
        let mut can_unmark = false;
        if let Some(a) = delta.actors.get_mut(&row.actor_id)
            && a.rev >= row.rev_u64
        {
            if row.cold_outbox_flushed > 0 {
                let n = row.cold_outbox_flushed.min(a.mind.ltm.cold_outbox.len());
                a.mind.ltm.cold_outbox.drain(..n);
            }
            a.flushed_rev = row.rev_u64;
            a.flushed_copper = row.copper;
            if a.rev == row.rev_u64 && a.copper == row.copper && a.mind.ltm.cold_outbox.is_empty() {
                a.dirty = false;
                can_unmark = true;
            }
        }
        if can_unmark {
            delta.actors.unmark_dirty_key(&row.actor_id);
        }
    }
}

/// 單步同步提交封裝（供啟動遷移與測試直接呼叫）
pub fn commit_delta_atomic(
    doc: &WorldStaticDoc,
    pool: &DbPool,
    delta: &mut IslandWorldDelta,
    flush_all_actors: bool,
) -> Result<(), String> {
    let batch = prepare_commit_batch(doc, delta, flush_all_actors)?;
    execute_commit_batch_pg(pool, &batch)?;
    ack_commit_batch(delta, &batch);
    Ok(())
}
