//! 工作節拍：求職撮合、Traveler 註冊、傳聞與視野刷新。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::{Server, sim};
use crate::db::{
    get_npc_title_from_assignments, get_schedule_for_entity, upsert_npc_rumor, with_room_graph,
};
use crate::gametext;
use crate::npc::{
    JobMatchParams, MovementDef, MovementType, SEEK_JOB_MG_THRESHOLD_DEFAULT, TravelerManager,
    movement_speed_for_title, run_job_matching_tick,
};
use crate::store::NpcRumor;

use super::super::SessionStore;
use super::super::broadcast::broadcast_room_views;
use super::MainLoopTickState;
use super::helpers::{periodic_fire, sprintf_d};

/// 求職撮合 tick（＋傳聞）；新入職且有排班者註冊排班型 Traveler。
pub(super) fn run_job_matching_section(
    sessions: &SessionStore,
    cfg: &Server,
    now_unix: i64,
    state: &Arc<Mutex<MainLoopTickState>>,
    traveler_mgr: &Arc<Mutex<TravelerManager>>,
) {
    let mut jm_sec = sim().job_matching_interval_sec;
    if jm_sec <= 0 {
        jm_sec = 30;
    }
    let fire = {
        let Ok(mut st) = state.lock() else {
            return;
        };
        periodic_fire(
            &mut st.last_job_matching,
            Duration::from_secs(jm_sec as u64),
        )
    };
    if !fire {
        return;
    }
    let mut mg_th = cfg.seek_job_mg_threshold;
    if mg_th <= 0 {
        mg_th = SEEK_JOB_MG_THRESHOLD_DEFAULT;
    }
    let added = with_room_graph(|g| {
        run_job_matching_tick(
            Some(g),
            JobMatchParams {
                max_per_venue: sim().max_assignments_per_venue,
                mg_threshold: mg_th,
                job_match_when_stable: cfg.job_match_when_stable,
            },
        )
    });
    if added.is_empty() {
        return;
    }
    for eid in &added {
        if get_schedule_for_entity(eid).is_some() {
            let mut title = get_npc_title_from_assignments(eid);
            if title.is_empty() {
                title = gametext::default_manager_title();
            }
            let speed = movement_speed_for_title(&title);
            if let Ok(mut g) = traveler_mgr.lock() {
                g.register(
                    eid.clone(),
                    MovementDef {
                        kind: MovementType::Schedule,
                        speed,
                    },
                );
            }
        }
    }
    broadcast_room_views(sessions, cfg);
    let mut job_ttl = sim().job_match_rumor_ttl_sec;
    if job_ttl <= 0 {
        job_ttl = 7200;
    }
    let fmt = gametext::npc_social().job_match_rumor_fmt;
    let text = sprintf_d(&fmt, added.len() as i32);
    let rumor = NpcRumor {
        id: format!("job|{}", now_unix / 1800),
        text,
        room_id: String::new(),
        zone: String::new(),
        source: "job".into(),
        source_score: 4,
        weight: 2,
        mention_count: 0,
        last_used_at: 0,
        blocked_until: 0,
        penalty_count: 0,
        last_penalty_at: 0,
        last_penalty_reason: String::new(),
        updated_at: now_unix,
        expires_at: now_unix + job_ttl,
    };
    if let Err(e) = upsert_npc_rumor(rumor) {
        tracing::warn!("job match upsert_npc_rumor: {e}");
    }
}
