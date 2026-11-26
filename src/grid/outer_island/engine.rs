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
    add_language_exposure, apply_setup_origin, new_uninitialized_actor, push_breath,
};
use super::persist_pg::{
    NormalizedLoadOutcome, ack_commit_batch, commit_delta_atomic, execute_commit_batch_pg,
    load_or_migrate_from_pg, prepare_commit_batch,
};
use super::persona::derive_innate_persona;
use super::sim_tick::{seed_genesis_actors_once, spawn_dynamic_actors, tick_outer_island_world};
use super::slow_think::{
    coerce_feasible_strategy, execute_slow_think_off_lock, mask_player_names_in_text,
    pop_top_slow_think_candidate,
};
use super::types::{IslandWorldDelta, NewActorSpawnSpec, WorldStaticDoc};
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
    working.last_stagger_minute = working.last_stagger_minute.max(live.last_stagger_minute);
    super::living_chronicler::preserve_polished_projections(
        &live.living_chronicle,
        &mut working.living_chronicle,
    );
    for (aid, live_a) in &live.actors {
        if (live_a.conversing_until_ms > 0
            || live_a.last_talk_unix_ms > 0
            || live_a.last_active_unix > 0)
            && let Some(wa_ro) = working.actors.get(aid)
            && (live_a.conversing_until_ms > wa_ro.conversing_until_ms
                || live_a.last_talk_unix_ms > wa_ro.last_talk_unix_ms
                || live_a.last_active_unix > wa_ro.last_active_unix)
            && let Some(wa) = working.actors.get_mut(aid)
        {
            wa.conversing_until_ms = wa.conversing_until_ms.max(live_a.conversing_until_ms);
            wa.last_talk_unix_ms = wa.last_talk_unix_ms.max(live_a.last_talk_unix_ms);
            if live_a.last_active_unix > wa.last_active_unix {
                wa.last_active_unix = live_a.last_active_unix;
                if live_a.controller_id.is_some() && live_a.route_plan.is_empty() {
                    wa.route_plan.clear();
                }
            }
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
                    match commit_delta_atomic(&doc, &pool, &mut delta, true) {
                        Ok(()) => has_pg = true,
                        Err(err) => tracing::error!(
                            "[outer_island] initial genesis PG commit failed, deferring: {err}"
                        ),
                    }
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
                    commit_delta_atomic(&self.doc, &pool, &mut g, true).map_err(|err| {
                        tracing::error!(
                            "[outer_island] ensure_pg_ready genesis commit failed: {err}"
                        );
                        "港務木冊暫時忙碌，請稍候片刻再試。".to_string()
                    })?;
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
        self.mutate_commit_opt(false, mutator)
    }

    fn mutate_commit_opt<T, F>(&self, skip_if_clean: bool, mutator: F) -> Result<T, String>
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
            let prev_bell = working.last_settled_bell;
            let prev_stagger = working.last_stagger_minute;
            let out = mutator(&self.doc, &mut working)?;
            if skip_if_clean && !super::live_tide::delta_has_pending_commit(&working, prev_bell) {
                if working.last_stagger_minute > prev_stagger
                    && let Ok(mut guard) = self.delta.write()
                {
                    guard.last_stagger_minute =
                        guard.last_stagger_minute.max(working.last_stagger_minute);
                }
                return Ok(out);
            }
            let batch = prepare_commit_batch(&self.doc, &working, false)?;
            execute_commit_batch_pg(&pool, &batch).map_err(|err| {
                tracing::error!("[outer_island] commit_before_publish PG failed: {err}");
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

    /// 背景模擬迴圈每 2.5 秒（1 遊戲分鐘）呼叫：全服全體 `UnifiedMindBody` 共用同一連續時間流、錯峰作息與慢思考佇列
    pub fn tick_live(&self, now_ms: u64) {
        let clock = current_solar_clock();
        let slow_cand = self
            .mutate_commit_opt(true, |doc, working| {
                tick_outer_island_world(doc, working, &clock, now_ms);
                Ok(pop_top_slow_think_candidate(doc, working, clock.game_days))
            })
            .ok()
            .flatten();

        if let Some(cand) = slow_cand {
            let llm_out = execute_slow_think_off_lock(&cand);
            let _ = self.mutate_commit_before_publish(|doc, working| {
                let owes_debt = working.contracts.values().any(|c| {
                    c.debtor_id == cand.actor_id && (c.status == "open" || c.status == "defaulted")
                });
                let safe_llm_out =
                    llm_out.map(|(l_zh, sk)| (mask_player_names_in_text(working, &l_zh), sk));
                let feasible_llm = safe_llm_out.map(|(l_zh, sk)| {
                    (
                        l_zh,
                        coerce_feasible_strategy(doc, working, &cand.actor_id, sk, owes_debt),
                    )
                });
                let feasible_fb = coerce_feasible_strategy(
                    doc,
                    working,
                    &cand.actor_id,
                    cand.fallback_strategy,
                    owes_debt,
                );
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
                        let wear_now = a.bound_weapon.wear_stage;
                        if let Some((lesson_zh, strat_opt)) = feasible_llm {
                            let anchor =
                                format!("slow:{}:{}:{}", cand.actor_id, cand.game_day, a.rev);
                            let slug = strat_opt.map_or("steady_work", |k| k.tag_slug());
                            a.mind
                                .push_turning_point(&anchor, cand.game_day, &lesson_zh, slug);
                            if let Some(strat_kind) = strat_opt {
                                a.mind.apply_goal_strategy_with_snapshot(
                                    strat_kind,
                                    cand.focus_target_id.clone(),
                                    cand.game_day,
                                    4,
                                    wear_now,
                                    owes_debt,
                                );
                            }
                        } else if let Some(fb_kind) = feasible_fb {
                            a.mind.apply_goal_strategy_with_snapshot(
                                fb_kind,
                                cand.focus_target_id.clone(),
                                cand.game_day,
                                3,
                                wear_now,
                                owes_debt,
                            );
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
            super::live_tide::wake_player_on_explicit_input(working, pid, clock.real_unix);
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
            super::live_tide::wake_player_on_explicit_input(working, pid, clock.real_unix);
            let aid = working
                .resolve_controller(pid)
                .ok_or("請先向港務官核對通行身分。")?;
            super::map_memory::apply_memory_map_travel(
                doc,
                working,
                &clock,
                &aid,
                target_node,
                now_ms,
            )
        })
    }

    pub fn explore_here(&self, pid: &str) -> Result<String, String> {
        let clock = current_solar_clock();
        self.mutate_commit_before_publish(|doc, working| {
            super::live_tide::wake_player_on_explicit_input(working, pid, clock.real_unix);
            let aid = working
                .resolve_controller(pid)
                .ok_or("請先向港務官核對通行身分。")?;
            apply_explore(doc, working, &clock, &aid)
        })
    }

    pub fn do_action(&self, pid: &str, action: &str, param: &str) -> Result<String, String> {
        let clock = current_solar_clock();
        self.mutate_commit_before_publish(|doc, working| {
            super::live_tide::wake_player_on_explicit_input(working, pid, clock.real_unix);
            let aid = working
                .resolve_controller(pid)
                .ok_or("請先向港務官核對通行身分。")?;
            apply_world_action(doc, working, &clock, &aid, action, param)
        })
    }

    /// 若當面交談對象為另一名真人玩家（`controller_id.is_some()`），直接走源頭語言遮罩＋在場旁聽＋拆穿吹牛（零 AI 代言冒充！）
    pub fn try_peer_direct_talk(
        &self,
        pid: &str,
        target_token: &str,
        message: &str,
        now_ms: u64,
    ) -> Option<Result<Value, String>> {
        self.ensure_pg_ready().ok()?;
        let precheck = {
            let guard = self.delta.read().ok()?;
            let tid = guard.resolve_public_target(target_token)?;
            let t = guard.actors.get(&tid)?;
            if t.controller_id.is_none() && !tid.starts_with("pl:") {
                return None;
            }
            let Some(sp_aid) = guard.resolve_controller(pid) else {
                return Some(Err("請先向港務官核對通行身分。".to_string()));
            };
            if sp_aid == tid {
                return Some(Err("無須對著自己的影子自言自語。".to_string()));
            }
            let Some(sp) = guard.actors.get(&sp_aid) else {
                return Some(Err("請先向港務官核對通行身分。".to_string()));
            };
            if let Err(e) = super::live_tide::check_speaker_ready_to_talk(sp) {
                return Some(Err(e));
            }
            let co_present = t.alive
                && t.setup_done
                && t.node_id == sp.node_id
                && guard
                    .node_occupants
                    .get(&sp.node_id)
                    .is_some_and(|occ| occ.contains(&tid));
            if !co_present {
                return Some(Err("對方此刻已走遠，不在你的視線範圍內。".to_string()));
            }
            let shown_name = if sp.is_acquainted_with(&tid) || t.is_acquainted_with(&sp_aid) {
                t.display_name.clone()
            } else {
                format!("陌生的{}", t.role)
            };
            (shown_name, t.role.clone(), t.race.clone())
        };
        if message.trim().is_empty() {
            let (shown_name, t_role, t_race) = precheck;
            let v = self.build_player_view(pid, pid);
            return Some(Ok(json!({
                "ok": true,
                "is_peer_player": true,
                "npc_id": target_token,
                "npc_name": shown_name,
                "npc_role": t_role,
                "npc_race": t_race,
                "reply": "（對方是同在現場的旅人，可在下方直接輸入話語當面開口，或進行亮物談價、託事與點到為止切磋。）",
                "view": v,
            })));
        }
        let clock = current_solar_clock();
        let res = self.mutate_commit_before_publish(|doc, working| {
            let sp_aid = working
                .resolve_controller(pid)
                .ok_or("請先向港務官核對通行身分。")?;
            {
                let sp = working
                    .actors
                    .get_mut(&sp_aid)
                    .ok_or("請先向港務官核對通行身分。")?;
                super::live_tide::check_speaker_ready_to_talk(sp)?;
                if now_ms > 0
                    && sp.last_talk_unix_ms > 0
                    && now_ms.saturating_sub(sp.last_talk_unix_ms) < 1200
                {
                    return Err("對方正在思索你的話意，請稍候片刻再開口。".to_string());
                }
                sp.last_talk_unix_ms = now_ms;
                sp.last_active_unix = clock.real_unix;
                sp.mark_mutated();
            }
            let tid = working
                .resolve_public_target(target_token)
                .ok_or("對方已不在場。")?;
            let (_masked, exposed) = super::pacts_and_peers::apply_peer_direct_speech(
                doc,
                working,
                &clock,
                &sp_aid,
                target_token,
                message,
            )?;
            let (t_name, t_role, t_race) = {
                let t = working.actors.get(&tid).ok_or("對方已不在場。")?;
                (t.display_name.clone(), t.role.clone(), t.race.clone())
            };
            Ok((t_name, t_role, t_race, exposed))
        });
        Some(res.map(|(t_name, t_role, t_race, exposed)| {
            json!({
                "ok": true,
                "is_peer_player": true,
                "npc_id": target_token,
                "npc_name": t_name,
                "npc_role": t_role,
                "npc_race": t_race,
                "reply": "（話音已當面傳入對方耳中，待對方親自開口回應。）",
                "side_overheard": exposed,
                "view": self.build_player_view(pid, pid),
            })
        }))
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
            super::live_tide::check_speaker_ready_to_talk(sp)?;
            let _ = super::live_tide::touch_client_lease(pid, clock.real_unix, true);
            sp.route_plan.clear();
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
            if t_mut.controller_id.is_some() || target_aid.starts_with("pl:") {
                return Err(format!(
                    "旅人{}正專注於自身行止，未曾搭腔。",
                    t_mut.display_name
                ));
            }
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
        let fam = soc_opt.map_or(if acquainted { 40 } else { 0 }, |s| s.familiarity);
        let warmth = soc_opt.map_or(0, |s| s.warmth);
        let is_kin_or_hh = (!sp.household_id.is_empty() && sp.household_id == target.household_id)
            || soc_opt.is_some_and(|s| s.is_kin());
        let mut stance_verdict = innate.arbitrate_dialogue_stance_with_memory(
            &target.drift,
            &target.role,
            acquainted,
            fam,
            warmth,
            is_kin_or_hh,
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

        let mut relevant_memories_zh = target.mind.recall_relevant_for_utterance(
            &speaker_aid,
            &trimmed_msg,
            topic.trim(),
            clock.game_days,
        );
        super::surface_cues::enrich_dialogue_with_graduated_disclosure(
            &self.doc,
            &guard,
            sp,
            target,
            &innate,
            topic.trim(),
            &trimmed_msg,
            &mut stance_verdict,
            &mut relevant_memories_zh,
        );
        if !target.ancestor_note_zh.is_empty() && relevant_memories_zh.len() < 5 {
            relevant_memories_zh.push(target.ancestor_note_zh.clone());
        }
        if !target.bound_weapon.provenance.is_empty()
            && (trimmed_msg.contains("錘")
                || trimmed_msg.contains("刀")
                || trimmed_msg.contains("傢伙")
                || trimmed_msg.contains("手藝"))
        {
            relevant_memories_zh.insert(
                0,
                format!(
                    "手頭這把{}是{}",
                    target.bound_weapon.name, target.bound_weapon.provenance
                ),
            );
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
    ) -> Result<(Option<String>, Option<String>), String> {
        let clock = current_solar_clock();
        self.mutate_commit_before_publish(|doc, working| {
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
                    let t_inn = derive_innate_persona(target.soul_seed.max(1));
                    let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
                    let _ = target.mind.appraise_event(
                        &t_inn,
                        super::appraisal::ConcernCause::PactBetrayed,
                        Some(&speaker_aid),
                        Some(super::persona::ValueAxis::Autonomy),
                        46,
                        abs_bell,
                    );
                }
                target.mark_mutated();
            }

            let boast_callout = super::pacts_and_peers::check_and_expose_unwitnessed_boast(
                doc,
                working,
                &clock,
                &speaker_aid,
                &p_node_id,
                "harbor_trade",
                &out.player_said,
            );

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
            let breath_line = if out.npc_reply.starts_with('「')
                && out.npc_reply.contains('」')
                && out.npc_reply.contains(npc_name)
            {
                out.npc_reply.clone()
            } else {
                format!(
                    "「{}」{}應道。",
                    out.npc_reply.trim_matches(|c| c == '「' || c == '」'),
                    npc_name
                )
            };
            push_breath(sp, breath_line);
            sp.mark_mutated();
            if let Some(ref rkey) = out.unlocked_rumor {
                super::map_memory::ingest_hearsay_rumor_pin(
                    doc,
                    working,
                    &speaker_aid,
                    rkey,
                    &out.npc_reply,
                    clock.game_days,
                );
            }
            Ok((lang_msg, boast_callout))
        })
    }

    pub fn pulse_meta(&self, pid: &str) -> Value {
        let clock = current_solar_clock();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(0));
        let _ = super::live_tide::touch_client_lease(pid, clock.real_unix, false);
        let seq = self.delta.read().map_or(0, |g| g.persist_seq);
        json!({ "ok": true, "boot_epoch": super::live_tide::boot_epoch_ms(), "persist_seq": seq, "server_unix": clock.real_unix, "server_unix_ms": now_ms })
    }

    pub fn build_player_view(&self, pid: &str, fallback_name: &str) -> Value {
        self.ensure_player(pid, fallback_name);
        let clock = current_solar_clock();
        let _ = super::live_tide::touch_client_lease(pid, clock.real_unix, false);
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
        let _ = super::live_tide::touch_client_lease(pid, clock.real_unix, false);
        let Ok(guard) = self.delta.read() else {
            return json!({ "ok": false, "msg": "海霧迷濛，請稍候再展卷。" });
        };
        let Some(viewer) = guard.actor_by_controller(pid) else {
            return json!({ "ok": false, "msg": "尚未在港務冊查見你的名號。" });
        };
        build_filtered_map_json(&self.doc, &guard, viewer, &clock)
    }

    pub fn cleanup_qa_actor(&self, pid: &str) -> Result<bool, String> {
        super::live_tide::revoke_client_lease(pid);
        self.ensure_pg_ready()?;
        self.mutate_commit_before_publish(|_doc, working| {
            Ok(super::persist_pg::purge_test_actor_conserving_copper(
                working, pid,
            ))
        })
    }
}

pub use super::live_shifts::ordered_actors_at_node;
