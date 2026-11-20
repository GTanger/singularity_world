//! 外海大島與全大陸通用執行期單例、嚴格控制權隔離、$O(1)$ 對等視線檢索與真正 `Commit-Before-Publish` 引擎。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};

use serde_json::{Value, json};

use crate::{db, pg};

use super::actions::apply_world_action;
use super::clock_and_lang::current_solar_clock;
use super::dialogue_llm::{DialoguePartnerProfile, NpcDialogueInput, NpcDialogueOutcome};
use super::explore_and_combat::apply_explore;
use super::movement::{
    add_language_exposure, apply_move_step, apply_setup_origin, new_uninitialized_actor,
    push_breath,
};
use super::persist_pg::{
    NormalizedLoadOutcome, ack_commit_batch, commit_delta_atomic, execute_commit_batch_pg,
    load_or_migrate_from_pg, prepare_commit_batch,
};
use super::persona::derive_innate_persona;
use super::sim_tick::{seed_genesis_actors_once, spawn_dynamic_actors, tick_outer_island_world};
use super::slow_think::{
    execute_slow_think_off_lock, pop_top_slow_think_candidate, push_validated_goal_intention,
};
use super::types::{
    IslandWorldDelta, NewActorSpawnSpec, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc,
};
use super::views::{build_filtered_map_json, build_view_json};

pub struct IslandEngine {
    pub doc: WorldStaticDoc,
    pub delta: RwLock<IslandWorldDelta>,
    persist_lock: Mutex<()>,
    loaded_from_pg: AtomicBool,
    pg_corrupt_freeze: AtomicBool,
}

static GLOBAL_ISLAND: OnceLock<IslandEngine> = OnceLock::new();

pub fn global_island() -> &'static IslandEngine {
    GLOBAL_ISLAND.get_or_init(IslandEngine::load_or_init)
}

pub fn load_static_world_doc() -> WorldStaticDoc {
    let candidates = [
        Path::new("data/world/tanger_outer_island_v1.json").to_path_buf(),
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/data/world/tanger_outer_island_v1.json"
        ))
        .to_path_buf(),
    ];
    for path in &candidates {
        if let Ok(raw) = std::fs::read_to_string(path)
            && let Ok(doc) = serde_json::from_str::<WorldStaticDoc>(&raw)
        {
            return doc;
        }
    }
    panic!("missing compiled world data/world/tanger_outer_island_v1.json");
}

fn preserve_transient_leases(live: &IslandWorldDelta, working: &mut IslandWorldDelta) {
    for (aid, live_a) in &live.actors {
        if (live_a.conversing_until_ms > 0 || live_a.last_talk_unix_ms > 0)
            && let Some(wa_ro) = working.actors.get(aid)
            && (live_a.conversing_until_ms > wa_ro.conversing_until_ms
                || live_a.last_talk_unix_ms > wa_ro.last_talk_unix_ms)
            && let Some(wa) = working.actors.get_mut(aid)
        {
            wa.conversing_until_ms = wa.conversing_until_ms.max(live_a.conversing_until_ms);
            wa.last_talk_unix_ms = wa.last_talk_unix_ms.max(live_a.last_talk_unix_ms);
        }
    }
}

impl IslandEngine {
    fn load_or_init() -> Self {
        let doc = load_static_world_doc();
        let clock = current_solar_clock();
        let mut delta = IslandWorldDelta {
            genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
            municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
            silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
            ..IslandWorldDelta::default()
        };
        seed_genesis_actors_once(&doc, &mut delta, clock.is_night);

        let mut has_pg = false;
        let mut corrupt = false;
        if let Some(pool) = pg::pool() {
            match load_or_migrate_from_pg(&doc, &pool) {
                NormalizedLoadOutcome::Loaded(loaded) => {
                    delta = loaded;
                    has_pg = true;
                }
                NormalizedLoadOutcome::Empty => {
                    for a in delta.actors.values_mut() {
                        a.dirty = true;
                    }
                    let _ = commit_delta_atomic(&doc, &pool, &mut delta, true);
                    has_pg = true;
                }
                NormalizedLoadOutcome::Corrupt => {
                    has_pg = true;
                    corrupt = true;
                }
                NormalizedLoadOutcome::Unavailable => {}
            }
        }
        Self {
            doc,
            delta: RwLock::new(delta),
            persist_lock: Mutex::new(()),
            loaded_from_pg: AtomicBool::new(has_pg),
            pg_corrupt_freeze: AtomicBool::new(corrupt),
        }
    }

    fn ensure_pg_ready(&self) -> Result<(), String> {
        if self.pg_corrupt_freeze.load(Ordering::Acquire) {
            return Err("港務舊冊正在修繕核對，請稍候再試。".to_string());
        }
        if self.loaded_from_pg.load(Ordering::Acquire) {
            return Ok(());
        }
        let Some(pool) = pg::pool() else {
            return Ok(());
        };
        let Ok(_serial) = self.persist_lock.lock() else {
            return Err("港務木冊暫時忙碌，請稍候再試。".to_string());
        };
        if self.loaded_from_pg.load(Ordering::Acquire) {
            return if self.pg_corrupt_freeze.load(Ordering::Acquire) {
                Err("港務舊冊正在修繕核對，請稍候再試。".to_string())
            } else {
                Ok(())
            };
        }
        match load_or_migrate_from_pg(&self.doc, &pool) {
            NormalizedLoadOutcome::Loaded(loaded) => {
                if let Ok(mut g) = self.delta.write() {
                    *g = loaded;
                }
                self.loaded_from_pg.store(true, Ordering::Release);
                Ok(())
            }
            NormalizedLoadOutcome::Empty => {
                if let Ok(mut g) = self.delta.write() {
                    let clk = current_solar_clock();
                    seed_genesis_actors_once(&self.doc, &mut g, clk.is_night);
                    let _ = commit_delta_atomic(&self.doc, &pool, &mut g, true);
                }
                self.loaded_from_pg.store(true, Ordering::Release);
                Ok(())
            }
            NormalizedLoadOutcome::Corrupt => {
                self.pg_corrupt_freeze.store(true, Ordering::Release);
                self.loaded_from_pg.store(true, Ordering::Release);
                Err("港務舊冊正在修繕核對，請稍候再試。".to_string())
            }
            NormalizedLoadOutcome::Unavailable => {
                Err("港務木冊暫時忙碌，請稍候片刻再試。".to_string())
            }
        }
    }

    /// 嚴格 `Commit-Before-Publish`：先在離鎖工作副本完成狀態推演與 PostgreSQL `COMMIT`（含 `fsync` 與 `rows_affected` 全中驗證），成功後方原子置換記憶體狀態！
    fn mutate_commit_before_publish<T, F>(&self, mutator: F) -> Result<T, String>
    where
        F: FnOnce(&WorldStaticDoc, &mut IslandWorldDelta) -> Result<T, String>,
    {
        self.ensure_pg_ready()?;
        let _serial = self.persist_lock.lock().unwrap_or_else(|e| e.into_inner());

        if let Some(pool) = pg::pool() {
            let mut working = {
                let guard = self.delta.read().unwrap_or_else(|e| e.into_inner());
                guard.clone()
            };
            let out = mutator(&self.doc, &mut working)?;
            let batch = prepare_commit_batch(&self.doc, &working, false)?;
            execute_commit_batch_pg(&pool, &batch).map_err(|err| {
                tracing::error!("[outer_island] commit_before_publish PG failed: {err}");
                // M3 防護：若交易在網路邊界處於未決狀態，強制下次請求自 PG 權威重載以對齊 rev
                self.loaded_from_pg.store(false, Ordering::Release);
                "港務木冊暫時無法落筆，剛才的舉動尚未生效，請稍候再試。".to_string()
            })?;
            ack_commit_batch(&mut working, &batch);
            let mut guard = self.delta.write().unwrap_or_else(|e| e.into_inner());
            preserve_transient_leases(&guard, &mut working);
            *guard = working;
            Ok(out)
        } else {
            let mut guard = self.delta.write().unwrap_or_else(|e| e.into_inner());
            mutator(&self.doc, &mut guard)
        }
    }

    /// 背景模擬迴圈每 10 秒呼叫：全服全體 `UnifiedMindBody` 共用同一時間流、效用決策、睡眠整合與慢思考佇列
    pub fn tick_live(&self, now_ms: u64) {
        let clock = current_solar_clock();
        let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
        let slow_cand = self
            .mutate_commit_before_publish(|doc, working| {
                tick_outer_island_world(doc, working, &clock, now_ms);
                Ok(pop_top_slow_think_candidate(
                    doc,
                    working,
                    clock.game_days,
                    abs_bell,
                ))
            })
            .ok()
            .flatten();

        if let Some(cand) = slow_cand {
            let (lesson_zh, goal_tag) = execute_slow_think_off_lock(&cand);
            let _ = self.mutate_commit_before_publish(|_doc, working| {
                if let Some(a) = working.actors.get_mut(&cand.actor_id)
                    && a.alive
                {
                    let cur_last = a
                        .mind
                        .ltm
                        .turning_points
                        .last()
                        .map(|t| t.anchor_event_id.as_str());
                    if cur_last == cand.last_tp_anchor.as_deref() {
                        let anchor = format!("slow:{}:{}:{}", cand.actor_id, cand.game_day, a.rev);
                        a.mind
                            .push_turning_point(&anchor, cand.game_day, &lesson_zh, &goal_tag);
                        if a.mind.ltm.known_nodes.contains(&cand.validated_goal_node) {
                            push_validated_goal_intention(a, &goal_tag, cand.abs_bell);
                        }
                        a.mark_mutated();
                    } else {
                        a.mind.pending_slow_think_score =
                            a.mind.pending_slow_think_score.max(cand.saved_score);
                        a.mark_mutated();
                    }
                }
                Ok(())
            });
        }
    }

    /// 執行期免重啟批量接納新誕生／跨海移入之 `UnifiedMindBody`（經 `UNNEST` 單次 `Commit-Before-Publish` 提交）
    pub fn spawn_actors(&self, specs: Vec<NewActorSpawnSpec>) -> Result<Vec<String>, String> {
        let clock = current_solar_clock();
        self.mutate_commit_before_publish(|doc, working| {
            Ok(spawn_dynamic_actors(
                doc,
                working,
                specs,
                clock.game_days,
                clock.bell_index,
            ))
        })
    }

    pub fn ensure_player(&self, pid: &str, fallback_name: &str) {
        if self.ensure_pg_ready().is_err() {
            return;
        }
        let already_exists = self
            .delta
            .read()
            .ok()
            .is_some_and(|g| g.resolve_controller(pid).is_some());
        if already_exists {
            return;
        }
        let preferred_seed = db::get_entity(pid)
            .ok()
            .flatten()
            .and_then(|ent| ent.soul_seed)
            .and_then(|s| u64::try_from(s).ok())
            .unwrap_or(0);
        let clk = current_solar_clock();
        let _ = self.mutate_commit_before_publish(|doc, working| {
            if !working.genesis_seeded && working.actors.is_empty() {
                seed_genesis_actors_once(doc, working, clk.is_night);
            }
            if working.resolve_controller(pid).is_none() {
                let actor = new_uninitialized_actor(doc, pid, fallback_name, preferred_seed);
                working.upsert_actor(actor);
            }
            Ok(())
        });
    }

    pub fn setup_player(
        &self,
        pid: &str,
        display_name: &str,
        race: &str,
        appearance: &str,
        origin_pack: &str,
    ) -> Result<String, String> {
        let clock = current_solar_clock();
        self.mutate_commit_before_publish(|doc, working| {
            apply_setup_origin(
                doc,
                working,
                &clock,
                pid,
                display_name,
                race,
                appearance,
                origin_pack,
            )
        })
    }

    pub fn move_player(&self, pid: &str, target_node: &str, now_ms: u64) -> Result<String, String> {
        let clock = current_solar_clock();
        self.mutate_commit_before_publish(|doc, working| {
            let aid = working
                .resolve_controller(pid)
                .ok_or("請先向港務官核對通行身分。")?;
            apply_move_step(doc, working, &clock, &aid, target_node, now_ms)
        })
    }

    pub fn explore_here(&self, pid: &str) -> Result<String, String> {
        let clock = current_solar_clock();
        self.mutate_commit_before_publish(|doc, working| {
            let aid = working
                .resolve_controller(pid)
                .ok_or("請先向港務官核對通行身分。")?;
            apply_explore(doc, working, &clock, &aid)
        })
    }

    pub fn do_action(&self, pid: &str, action: &str, param: &str) -> Result<String, String> {
        let clock = current_solar_clock();
        self.mutate_commit_before_publish(|doc, working| {
            let aid = working
                .resolve_controller(pid)
                .ok_or("請先向港務官核對通行身分。")?;
            apply_world_action(doc, working, &clock, &aid, action, param)
        })
    }

    pub fn prepare_talk_input(
        &self,
        pid: &str,
        npc_id: &str,
        topic: &str,
        message: &str,
        now_ms: u64,
    ) -> Result<NpcDialogueInput, String> {
        self.ensure_pg_ready()?;
        let clock = current_solar_clock();
        let mut guard = self
            .delta
            .write()
            .map_err(|_| "街市喧嚷，請稍候片刻再開口。".to_string())?;

        // C1 + M2 防護：發言方嚴格只走 `resolve_controller`，對話對象走不透明公開代號 `resolve_public_target`！
        let speaker_aid = guard
            .resolve_controller(pid)
            .ok_or("請先向港務官核對通行身分。")?;
        let target_aid = guard
            .resolve_public_target(npc_id)
            .ok_or("眼前找不到這位街坊。")?;
        if speaker_aid == target_aid {
            return Err("無須對著自己的影子自言自語。".to_string());
        }

        let p_node_id = {
            let sp = guard
                .actors
                .get_mut(&speaker_aid)
                .ok_or("請先向港務官核對通行身分。")?;
            if !sp.setup_done {
                return Err("請先在港務登岸木冊留下你的名號與來歷。".to_string());
            }
            if sp.active_skirmish.is_some() {
                return Err("眼前正與敵手短兵相接，無暇分心攀談！".to_string());
            }
            if sp.active_transit.is_some() {
                return Err("雙馬篷車正在大路上奔馳，需下車後才能與街坊交談。".to_string());
            }
            if now_ms > 0
                && sp.last_talk_unix_ms > 0
                && now_ms.saturating_sub(sp.last_talk_unix_ms) < 1200
            {
                return Err("對方正在思索你的話意，請稍候片刻再開口。".to_string());
            }
            sp.last_talk_unix_ms = now_ms;
            sp.last_active_unix = clock.real_unix;
            sp.node_id.clone()
        };

        let node = self
            .doc
            .nodes
            .get(&p_node_id)
            .ok_or("四周霧氣迷濛，暫時辨不清方位。")?;

        let co_present = guard
            .node_occupants
            .get(&p_node_id)
            .is_some_and(|occ| occ.contains(&target_aid))
            && guard
                .actors
                .get(&target_aid)
                .is_some_and(|t| t.alive && t.setup_done && t.node_id == p_node_id);
        if !co_present {
            return Err("對方此刻已走遠，不在你的視線範圍內。".to_string());
        }

        let (target_household, target_race) = {
            let t_mut = guard
                .actors
                .get_mut(&target_aid)
                .ok_or("眼前找不到這位街坊。")?;
            if clock.is_night && t_mut.node_id != t_mut.home_node {
                if t_mut.conversing_until_ms <= now_ms {
                    t_mut.conversing_until_ms = now_ms.saturating_add(8_000);
                }
            } else {
                t_mut.conversing_until_ms = now_ms.saturating_add(25_000);
            }
            (t_mut.household_id.clone(), t_mut.race.clone())
        };

        let apprentice_name = guard.node_occupants.get(&p_node_id).and_then(|occ| {
            occ.iter()
                .filter(|&oid| oid != &target_aid && oid != &speaker_aid)
                .filter_map(|oid| guard.actors.get(oid))
                .find(|other| {
                    other.alive
                        && !target_household.is_empty()
                        && other.household_id == target_household
                        && other.race == target_race
                })
                .map(|other| other.display_name.clone())
        });

        let sp = guard
            .actors
            .get(&speaker_aid)
            .ok_or("請先向港務官核對通行身分。")?;
        let target = guard
            .actors
            .get(&target_aid)
            .ok_or("眼前找不到這位街坊。")?;

        let trimmed_msg: String = message.trim().chars().take(80).collect();
        let used_pressure_tactic = trimmed_msg.contains("便宜")
            || trimmed_msg.contains("算少")
            || trimmed_msg.contains("賒")
            || trimmed_msg.contains("拜託")
            || trimmed_msg.contains("送我")
            || trimmed_msg.contains("滾")
            || trimmed_msg.contains("殺")
            || trimmed_msg.contains("廢話");

        let innate = derive_innate_persona(target.soul_seed.max(1));
        let soc_opt = target.mind.ltm.social.get(&speaker_aid);
        let acquainted =
            sp.is_acquainted_with(&target_aid) || soc_opt.is_some_and(|s| s.acquainted);
        let kept = soc_opt.map_or(0, |s| s.kept_pacts);
        let broken = soc_opt.map_or(0, |s| s.broken_pacts);
        let rep_press = soc_opt.map_or(0, |s| s.pressure_tactic_count);
        let stance_verdict = innate.arbitrate_dialogue_stance(
            &target.drift,
            &target.role,
            acquainted,
            kept,
            broken,
            rep_press,
            target.mind.stm.affect.anger,
            &trimmed_msg,
        );

        let recent_talks_with_player: Vec<(String, String)> = target
            .mind
            .stm
            .recent_talks
            .iter()
            .filter(|t| t.partner_id == speaker_aid)
            .map(|t| (t.partner_said.clone(), t.self_replied.clone()))
            .collect();

        let mut relevant_memories_zh = Vec::new();
        if let Some(soc) = soc_opt
            && !soc.notable_note_zh.is_empty()
        {
            relevant_memories_zh.push(format!("與此人往來紀錄：{}", soc.notable_note_zh));
        }
        for tp in target.mind.ltm.turning_points.iter().rev().take(1) {
            relevant_memories_zh.push(tp.lesson_zh.clone());
        }
        let mut ep_sorted: Vec<_> = target.mind.ltm.episodic.iter().collect();
        ep_sorted.sort_by_key(|e| std::cmp::Reverse(e.retention_score(clock.game_days)));
        for ep in ep_sorted.into_iter().take(3) {
            relevant_memories_zh.push(ep.subjective_zh.clone());
        }

        let known_local_landmarks_zh: Vec<String> = target
            .mind
            .ltm
            .known_nodes
            .iter()
            .filter_map(|nid| self.doc.nodes.get(nid))
            .map(|nd| nd.name.clone())
            .take(10)
            .collect();

        let public_pid = sp
            .controller_id
            .clone()
            .unwrap_or_else(|| sp.actor_id.clone());

        Ok(NpcDialogueInput {
            player_id: public_pid,
            player_name: sp.display_name.clone(),
            player_race: sp.race.clone(),
            player_copper: sp.copper,
            player_langs: sp.languages.clone(),
            has_stele_rubbing: sp
                .backpack
                .iter()
                .any(|i| i.slug == "stele_rubbing" && i.count > 0),
            has_lodging_voucher: sp
                .backpack
                .iter()
                .any(|i| i.slug == "lodging_voucher" && i.count > 0),
            acquainted,
            node_name: node.name.clone(),
            calendar_label: clock.calendar_label,
            bell_label: clock.bell_label,
            npc: DialoguePartnerProfile {
                id: target.public_actor_token(),
                name: target.display_name.clone(),
                race: target.race.clone(),
                role: target.role.clone(),
                languages: target.languages.clone(),
            },
            npc_effective_copper: target.copper,
            npc_tool_name: target.bound_weapon.name.clone(),
            apprentice_name,
            stance_verdict,
            recent_talks_with_player,
            relevant_memories_zh,
            known_local_landmarks_zh,
            used_pressure_tactic,
            topic: topic.trim().to_string(),
            player_utterance: trimmed_msg,
        })
    }

    pub fn apply_dialogue_effects(
        &self,
        pid: &str,
        npc_id: &str,
        npc_name: &str,
        out: &NpcDialogueOutcome,
    ) -> Result<Option<String>, String> {
        let clock = current_solar_clock();
        self.mutate_commit_before_publish(|_doc, working| {
            let speaker_aid = working.resolve_controller(pid).ok_or("找不到旅人行蹤")?;
            let target_aid = working
                .resolve_public_target(npc_id)
                .ok_or("對方已不在場")?;

            let (p_node_id, p_display_name) = {
                let sp = working.actors.get(&speaker_aid).ok_or("找不到旅人行蹤")?;
                (sp.node_id.clone(), sp.display_name.clone())
            };

            let co_present = working
                .actors
                .get(&target_aid)
                .is_some_and(|t| t.alive && t.node_id == p_node_id);
            if !co_present {
                return Err("話音未落，對方已轉過街角走遠了。".to_string());
            }

            if let Some(target) = working.actors.get_mut(&target_aid) {
                target.mind.record_talk_exchange(
                    &speaker_aid,
                    &p_display_name,
                    clock.game_days,
                    &clock.bell_label,
                    &out.player_said,
                    &out.npc_reply,
                    out.used_pressure_tactic,
                );
                if out.used_pressure_tactic {
                    target.mind.stm.affect.anger =
                        (target.mind.stm.affect.anger.saturating_add(14)).min(95);
                }
                target.mark_mutated();
            }

            let mut lang_msg = None;
            let sp = working
                .actors
                .get_mut(&speaker_aid)
                .ok_or("找不到旅人行蹤")?;
            sp.last_active_unix = clock.real_unix;
            sp.mind.record_talk_exchange(
                &target_aid,
                npc_name,
                clock.game_days,
                &clock.bell_label,
                &out.npc_reply,
                &out.player_said,
                false,
            );
            if let Some(ref rkey) = out.unlocked_rumor {
                sp.known_rumors.insert(rkey.clone());
            }
            if let Some((ref lcode, gain)) = out.learned_lang {
                lang_msg = add_language_exposure(sp, clock.game_days, lcode, gain);
            }
            push_breath(sp, format!("{}：{}", npc_name, out.npc_reply));
            sp.mark_mutated();
            Ok(lang_msg)
        })
    }

    pub fn build_player_view(&self, pid: &str, fallback_name: &str) -> Value {
        self.ensure_player(pid, fallback_name);
        let clock = current_solar_clock();
        let Ok(guard) = self.delta.read() else {
            return json!({ "ok": false, "msg": "海霧迷濛，請稍候再看。" });
        };
        let Some(viewer) = guard.actor_by_controller(pid) else {
            return json!({ "ok": false, "msg": "尚未在港務冊查見你的名號。" });
        };
        build_view_json(&self.doc, &guard, viewer, &clock)
    }

    pub fn build_map_view(&self, pid: &str, fallback_name: &str) -> Value {
        self.ensure_player(pid, fallback_name);
        let clock = current_solar_clock();
        let Ok(guard) = self.delta.read() else {
            return json!({ "ok": false, "msg": "海霧迷濛，請稍候再展卷。" });
        };
        let Some(viewer) = guard.actor_by_controller(pid) else {
            return json!({ "ok": false, "msg": "尚未在港務冊查見你的名號。" });
        };
        build_filtered_map_json(&self.doc, viewer, &clock)
    }
}

/// 依 $O(1)$ `node_occupants` 空間索引與釘選優先序回傳同節點全體在場人物（零 `doc.residents` 依賴、零 `.take(8)` 截斷！）
pub fn ordered_actors_at_node<'a>(
    delta: &'a IslandWorldDelta,
    viewer: &UnifiedMindBody,
    node: &SpatialNodeDef,
    now_ms: u64,
) -> Vec<&'a UnifiedMindBody> {
    let Some(occ) = delta.node_occupants.get(&node.id) else {
        return Vec::new();
    };

    let mut list: Vec<(u8, &'a UnifiedMindBody)> = occ
        .iter()
        .filter(|&aid| aid != &viewer.actor_id)
        .filter_map(|aid| delta.actors.get(aid))
        .filter(|other| other.alive && other.setup_done && other.node_id == node.id)
        .map(|other| {
            let conversing = now_ms > 0 && other.conversing_until_ms > now_ms;
            let at_post = other.controller_id.is_none()
                && (other.work_node == node.id
                    || (node.kind == "shop" && other.home_node == node.id))
                && matches!(node.kind.as_str(), "shop" | "dock" | "gate");
            let acquainted = viewer.is_acquainted_with(&other.actor_id)
                || other.is_acquainted_with(&viewer.actor_id);
            let rank = if conversing {
                0
            } else if at_post {
                1
            } else if acquainted {
                2
            } else {
                3
            };
            (rank, other)
        })
        .collect();

    list.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.actor_id.cmp(&b.1.actor_id)));
    list.into_iter().map(|(_, other)| other).collect()
}
