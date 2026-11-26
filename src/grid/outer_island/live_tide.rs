//! 「1 真實小時 ＝ 1 遊戲日（`scale = 24`）」連續時間流、48 階晨昏天光錨點、錯峰個人作息、
//! 120 秒存活租約（防搶在線身體與偷花抵宿券）、雙軌見聞分流與可追問目擊記憶橋接。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{LazyLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::combat::wordplate::splitmix64;
use crate::game;

use super::clock_and_lang::SolarClockSnapshot;
use super::desire_sim::{BellDecisionIndex, DesireDecision, evaluate_personalized_desire};
use super::household_economy::find_town_tavern_and_keeper;
pub use super::live_shifts::{
    assign_offline_shelter_route_if_needed, check_dock_shift_eligible, check_forge_shift_eligible,
    check_speaker_ready_to_talk, compose_arrival_breaths, compose_live_scene_desc,
    delta_has_pending_commit, ordered_actors_at_node, prune_stale_day_window_keys,
    record_dock_shift_claimed, record_forge_shift_claimed, settle_work_dock_action,
    tavern_has_ration_stock, wake_player_on_explicit_input,
};
use super::mind::{ActiveIntention, EventPublicity, EvidenceTier};
use super::movement::{push_breath, shortest_walk_path};
use super::persona::{ValueAxis, derive_innate_persona};
use super::types::{IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};

/// 線上視窗存活租約秒數（相容行動端背景分頁 60 秒計時器節流，嚴禁 150s 短閒置搶在線玩家身體）
pub const ONLINE_LEASE_SECS: i64 = 120;

/// 每日市政港務潮汐卸貨公共預算上限（班次數），杜絕無上限連點掏空公庫
pub const MAX_DAILY_DOCK_CARGO_SHIFTS: usize = 8;

#[derive(Debug, Clone, Copy)]
struct ClientLeaseEntry {
    lease_until_unix: i64,
    updated_unix: i64,
    control_gen: u64,
}

static BOOT_EPOCH_MS: LazyLock<u64> = LazyLock::new(|| {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |d| u64::try_from(d.as_millis()).unwrap_or(1))
});

static CLIENT_LEASES: LazyLock<RwLock<BTreeMap<String, ClientLeaseEntry>>> =
    LazyLock::new(|| RwLock::new(BTreeMap::new()));

pub fn boot_epoch_ms() -> u64 {
    *BOOT_EPOCH_MS
}

/// 續租在線視窗存活租約；`explicit_action = true` 時遞增控制代次以即刻撤銷在途自主步進。
pub fn touch_client_lease(pid: &str, now_unix: i64, explicit_action: bool) -> u64 {
    if pid.is_empty() || now_unix <= 0 {
        return 0;
    }
    let Ok(mut map) = CLIENT_LEASES.write() else {
        return 0;
    };
    if map.len() > 256 {
        map.retain(|_, v| now_unix.saturating_sub(v.updated_unix) <= 7_200);
    }
    let entry = map.entry(pid.to_string()).or_insert(ClientLeaseEntry {
        lease_until_unix: now_unix + ONLINE_LEASE_SECS,
        updated_unix: now_unix,
        control_gen: 1,
    });
    let was_expired = entry.lease_until_unix < now_unix;
    entry.lease_until_unix = now_unix + ONLINE_LEASE_SECS;
    entry.updated_unix = now_unix;
    if explicit_action || was_expired {
        entry.control_gen = entry.control_gen.saturating_add(1);
    }
    entry.control_gen
}

/// 玩家主動登出或關閉頁面時寫入過期墓碑（`lease_until_unix = 0`），避免回退觸發 1800s 假在線鎖死！
pub fn revoke_client_lease(pid: &str) {
    if pid.is_empty() {
        return;
    }
    if let Ok(mut map) = CLIENT_LEASES.write() {
        let now_u = game::now_unix();
        let entry = map.entry(pid.to_string()).or_insert(ClientLeaseEntry {
            lease_until_unix: 0,
            updated_unix: now_u,
            control_gen: 1,
        });
        entry.lease_until_unix = 0;
        entry.updated_unix = now_u;
    }
}

/// 判定真人控制角色目前是否受在線視窗保護（保護期間 100% 禁止自主接管移動或代用任何資產）。
pub fn is_actor_online_protected(a: &UnifiedMindBody, real_unix: i64) -> bool {
    let Some(ref pid) = a.controller_id else {
        return false;
    };
    if real_unix <= 0 {
        return false;
    }
    if let Ok(map) = CLIENT_LEASES.read()
        && let Some(entry) = map.get(pid)
    {
        return entry.lease_until_unix >= real_unix;
    }
    #[cfg(test)]
    {
        a.last_active_unix > 0 && (real_unix - a.last_active_unix) < 1800
    }
    #[cfg(not(test))]
    {
        false
    }
}

pub const TIDE_48_SHORT_NAMES: [&str; 48] = [
    "夜央", "深宵", "寒更", "沉夜", "更深", "蒼暝", "夜闌", "曉星", "殘夜", "將曉", "拂曉", "破曉",
    "晨微", "朝露", "晨光", "朝日", "早潮", "朝晴", "晝前", "晴嵐", "隅中", "向午", "日高", "照融",
    "正晝", "日盛", "日中", "日斜", "午後", "日昳", "晚晴", "斜陽", "落照", "夕照", "夕景", "黃昏",
    "日暮", "暮色", "夕霧", "薄暮", "初夜", "夜幕", "星垂", "沉冥", "夜靜", "寒夜", "夜半", "深更",
];

#[derive(Debug, Clone)]
pub struct TideClockDetails {
    pub epoch_unix: i64,
    pub scale: f64,
    pub sec_since_midnight: u32,
    pub hour: u32,
    pub minute: u32,
    pub tide_slot: u8,
    pub tide_name: &'static str,
    pub weather_zh: &'static str,
}

/// 由伺服器單一時間源（`scale = 24`，`1 真實小時 = 1 遊戲日`）計算精確分秒、48 候名與海島天候。
pub fn derive_tide_clock_details(clock: &SolarClockSnapshot) -> TideClockDetails {
    let (epoch_unix, raw_scale) = game::game_time_cfg();
    let scale = if raw_scale <= 0.0 { 24.0 } else { raw_scale };
    let (sec_day, hour, minute) = if clock.real_unix >= epoch_unix && clock.real_unix > 0 {
        let (sec, h, m, _d) = game::game_time_now(clock.real_unix, epoch_unix, scale);
        (
            u32::try_from(sec.clamp(0, 86_399)).unwrap_or(0),
            u32::try_from(h.clamp(0, 23)).unwrap_or(clock.hour),
            u32::try_from(m.clamp(0, 59)).unwrap_or(0),
        )
    } else {
        let h = clock.hour.min(23);
        (h * 3_600, h, 0)
    };
    let tide_slot = ((sec_day / 1_800) % 48) as u8;
    let tide_name = TIDE_48_SHORT_NAMES[usize::from(tide_slot)];
    let weather_zh = coastal_weather_zh(clock.game_days, clock.bell_index);
    TideClockDetails {
        epoch_unix,
        scale,
        sec_since_midnight: sec_day,
        hour,
        minute,
        tide_slot,
        tide_name,
        weather_zh,
    }
}

pub fn coastal_weather_zh(game_days: i64, bell_index: u8) -> &'static str {
    let band = (game_days.unsigned_abs().wrapping_mul(4)).wrapping_add(u64::from(bell_index / 3));
    match splitmix64(band ^ 0x51A7_C0A5) % 10 {
        0..=4 => "晴朗",
        5..=6 => "海風",
        7..=8 => "薄霧",
        _ => "微雨",
    }
}

/// 空間採光三態：`outdoor`（戶外吃 48 階天光）、`windowed`（室內窗光）、`sealed`（密閉地城鎖定常暗）。
pub fn node_exposure_kind(node: &SpatialNodeDef) -> &'static str {
    if node.dungeon {
        "sealed"
    } else if node.indoor {
        "windowed"
    } else {
        "outdoor"
    }
}

/// 白晝早晚兩班潮汐商船靠港卸貨視窗：早潮（晨鐘二響～日高四響 `3..=5`）、午潮（午後初鐘～申時二鐘 `7..=8`）。
pub fn is_dock_cargo_tide_open(clock: &SolarClockSnapshot) -> bool {
    !clock.is_night && (matches!(clock.bell_index, 3..=5) || matches!(clock.bell_index, 7..=8))
}

pub fn dock_shift_window_key(game_days: i64, bell_index: u8) -> String {
    format!("shift:dock:{game_days}:{bell_index}")
}

pub fn forge_shift_window_key(game_days: i64, bell_index: u8) -> String {
    format!("shift:forge:{game_days}:{bell_index}")
}

/// 將 4 座港鎮原本誤設 `home_node == dock`（露天）之碼頭苦力冪等配回同鎮支巷民居（保留全部待售空屋）。
pub fn ensure_porter_indoor_homes(doc: &WorldStaticDoc, delta: &mut IslandWorldDelta) {
    let mapping = [
        (
            "res:white_gull_harbor:dock_porter",
            "oi:white_gull_harbor:sail_ln:h6",
        ),
        (
            "res:west_strait_ferry:dock_porter",
            "oi:west_strait_ferry:ferry_row:h3",
        ),
        (
            "res:gull_tail_port:dock_porter",
            "oi:gull_tail_port:oyster_ln:h1",
        ),
        (
            "res:north_cape_beacon:dock_porter",
            "oi:north_cape_beacon:beacon_foot_ln:h1",
        ),
    ];
    for (aid, indoor_home) in mapping {
        let Some(home_nd) = doc.nodes.get(indoor_home) else {
            continue;
        };
        let needs_fix = delta
            .actors
            .get(aid)
            .is_some_and(|a| a.home_node != indoor_home);
        if !needs_fix {
            continue;
        }
        let local_nodes: Vec<String> = doc
            .nodes
            .iter()
            .filter(|(_, nd)| nd.cell_id == home_nd.cell_id && !nd.dungeon && !nd.indoor)
            .map(|(nid, _)| nid.clone())
            .collect();
        if let Some(a) = delta.actors.get_mut(aid) {
            a.home_node = indoor_home.to_string();
            a.mind.ltm.known_nodes.insert(indoor_home.to_string());
            a.mind.ltm.known_nodes.insert(a.work_node.clone());
            for ln in local_nodes {
                a.mind.ltm.known_nodes.insert(ln);
            }
            a.mark_mutated();
        }
    }
}

/// 嚴格床權判定：唯有身處「室內自有居所」或「已付宿／已換券之二樓宿房」方算真正上床安睡（露天碼頭絕不算床！）。
pub fn has_real_indoor_bed(doc: &WorldStaticDoc, a: &UnifiedMindBody, sleep_home: &str) -> bool {
    let at_indoor_home =
        a.node_id == sleep_home && doc.nodes.get(sleep_home).is_some_and(|n| n.indoor);
    let at_paid_inn_room = doc
        .nodes
        .get(&a.node_id)
        .is_some_and(|n| n.kind == "inn_room")
        && a.inn_nights_left > 0;
    at_indoor_home || at_paid_inn_room
}

/// 離線真人夜間零耗費同鎮避風安置目標：嚴限當前聚落 `cur_cell`（絕不拖穿外鎮荒野偷開迷霧），絕不消耗抵宿券或銅幣。
pub fn resolve_offline_player_night_target(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
) -> Option<(&'static str, String, String)> {
    let cur_nd = doc.nodes.get(&a.node_id)?;
    if cur_nd.dungeon || cur_nd.kind == "wild" {
        return None;
    }
    let cur_cell = cur_nd.cell_id;
    if let Some(ref owned) = a.owned_house_node
        && let Some(ond) = doc.nodes.get(owned)
        && ond.cell_id == cur_cell
    {
        if a.node_id != *owned {
            return Some((
                "go_home_sleep",
                owned.clone(),
                "返自家宅院吹燈安睡".to_string(),
            ));
        }
        return None;
    }
    if a.inn_nights_left > 0 {
        if let Some((inn_nid, _)) = doc
            .nodes
            .iter()
            .find(|(_, n)| n.cell_id == cur_cell && n.kind == "inn_room")
        {
            if a.node_id != *inn_nid {
                return Some((
                    "go_home_sleep",
                    inn_nid.clone(),
                    "回二樓已訂宿房安寢".to_string(),
                ));
            }
            return None;
        }
    }
    let (tav_opt, _) = find_town_tavern_and_keeper(doc, delta, cur_cell);
    if let Some(tav_nid) = tav_opt
        && a.node_id != tav_nid
    {
        return Some((
            "shelter_hearth",
            tav_nid,
            "往酒館大堂壁爐邊避寒歇腳".to_string(),
        ));
    }
    None
}

/// 為離線避風角色規劃同鎮安全路徑（嚴禁跨聚落或踏入荒野，且避開 `warmth <= -22` 嫌隙對象節點）。
pub fn plan_safe_offline_shelter_path(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
    goal: &str,
) -> Option<Vec<String>> {
    let cur_cell = doc.nodes.get(&a.node_id)?.cell_id;
    if doc.nodes.get(goal).is_none_or(|g| g.cell_id != cur_cell)
        || node_has_hostile_rival(delta, a, goal)
    {
        return None;
    }
    let mut allowed: BTreeSet<String> = doc
        .nodes
        .iter()
        .filter(|(nid, nd)| {
            nd.cell_id == cur_cell
                && !nd.dungeon
                && nd.kind != "wild"
                && (!nd.indoor || *nid == goal || nd.kind == "shop")
                && !node_has_hostile_rival(delta, a, nid)
        })
        .map(|(nid, _)| nid.clone())
        .collect();
    allowed.insert(a.node_id.clone());
    allowed.insert(goal.to_string());
    shortest_walk_path(doc, &a.node_id, goal, &allowed)
}

fn node_has_hostile_rival(delta: &IslandWorldDelta, a: &UnifiedMindBody, nid: &str) -> bool {
    delta.node_occupants.get(nid).is_some_and(|occ| {
        occ.iter().any(|oid| {
            oid != &a.actor_id
                && a.mind
                    .ltm
                    .social
                    .get(oid)
                    .is_some_and(|s| !s.cohabiting && s.warmth <= -22)
        })
    })
}

/// 依毫秒級時間與單調水位 `last_stagger_minute` 將全島 120 個 `(soul_seed % 120)` 作息槽逐分無漏觸發（杜絕整數秒奇偶跳槽與重複觸發）。
pub fn step_staggered_minute_routines(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    now_unix_ms: u64,
) {
    let td = derive_tide_clock_details(clock);
    let epoch_ms = u64::try_from(td.epoch_unix.max(0)).unwrap_or(0) * 1_000;
    let abs_game_min = if now_unix_ms >= epoch_ms && epoch_ms > 0 {
        let real_sec_f = (now_unix_ms as f64) / 1_000.0;
        (((real_sec_f - (td.epoch_unix as f64)) * td.scale) / 60.0).floor() as i64
    } else {
        clock.game_days * 1_440 + i64::from(td.hour) * 60 + i64::from(td.minute)
    };
    let start_min = if delta.last_stagger_minute <= 0 {
        abs_game_min - 1
    } else {
        delta.last_stagger_minute.max(abs_game_min - 120)
    };
    if abs_game_min <= start_min {
        return;
    }
    delta.last_stagger_minute = abs_game_min;
    let due_slots: BTreeSet<u64> = ((start_min + 1)..=abs_game_min)
        .map(|m| m.rem_euclid(120) as u64)
        .collect();
    let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);

    let mut cell_tavern_cache: BTreeMap<u32, (Option<String>, Option<String>)> = BTreeMap::new();
    for c in doc.cells.values() {
        if c.kind == "settlement" {
            cell_tavern_cache.insert(
                c.cell_id,
                find_town_tavern_and_keeper(doc, delta, c.cell_id),
            );
        }
    }

    let due_ids: Vec<String> = delta
        .actors
        .iter()
        .filter(|(aid, a)| {
            a.alive
                && a.setup_done
                && a.route_plan.is_empty()
                && a.walk_waypoints.is_empty()
                && a.active_skirmish.is_none()
                && a.active_transit.is_none()
                && !doc.nodes.get(&a.node_id).is_some_and(|n| n.dungeon)
                && !a.known_rumors.contains("pit_rumor")
                && !a
                    .backpack
                    .iter()
                    .any(|i| i.slug == "sealed_errand_pkg" && i.count > 0)
                && !delta.contracts.values().any(|c| {
                    c.status == "in_progress" && (c.creditor_id == **aid || c.debtor_id == **aid)
                })
                && !is_actor_online_protected(a, clock.real_unix)
                && due_slots.contains(&(a.soul_seed % 120))
        })
        .map(|(id, _)| id.clone())
        .collect();

    if due_ids.is_empty() {
        return;
    }

    let bell_idx_cache = BellDecisionIndex::build(delta);
    for aid in due_ids {
        let decision_opt: Option<(&'static str, String, String, bool)> = {
            let Some(a) = delta.actors.get(&aid) else {
                continue;
            };
            // 若正在執行護衛或探窟契約，絕不覆寫
            if a.mind.stm.intentions.back().is_some_and(|i| {
                matches!(
                    i.kind.as_str(),
                    "escort_watch" | "pit_expedition" | "courier_deliver"
                )
            }) {
                None
            } else if a.controller_id.is_some() {
                if clock.is_night {
                    resolve_offline_player_night_target(doc, delta, a)
                        .map(|(k, dst, r)| (k, dst, r, true))
                } else {
                    None
                }
            } else if clock.is_night {
                let sleep_home = a
                    .owned_house_node
                    .clone()
                    .unwrap_or_else(|| a.home_node.clone());
                if !has_real_indoor_bed(doc, a, &sleep_home) && a.node_id != sleep_home {
                    Some((
                        "go_home_sleep",
                        sleep_home,
                        "收工沿街返家歇息".to_string(),
                        false,
                    ))
                } else {
                    None
                }
            } else {
                let sleep_home = a
                    .owned_house_node
                    .as_deref()
                    .unwrap_or(a.home_node.as_str());
                let home_cell = doc.nodes.get(sleep_home).map_or(4616, |n| n.cell_id);
                let (tav_opt, ik_opt) = cell_tavern_cache
                    .get(&home_cell)
                    .map_or((None, None), |(t, k)| (t.as_deref(), k.as_deref()));
                let innate = derive_innate_persona(a.soul_seed.max(1));
                let DesireDecision {
                    kind,
                    target_node,
                    reason_zh,
                    ..
                } = evaluate_personalized_desire(
                    doc,
                    delta,
                    &bell_idx_cache,
                    a,
                    &innate,
                    tav_opt,
                    ik_opt,
                    clock.bell_index,
                    abs_bell,
                );
                (a.node_id != target_node).then_some((kind, target_node, reason_zh, false))
            }
        };

        let Some((kind, dst_nid, reason, is_offline_pl)) = decision_opt else {
            continue;
        };
        let path_opt = delta.actors.get(&aid).and_then(|a| {
            if is_offline_pl {
                plan_safe_offline_shelter_path(doc, delta, a, &dst_nid)
            } else if a.mind.ltm.known_nodes.contains(&dst_nid) {
                shortest_walk_path(doc, &a.node_id, &dst_nid, &a.mind.ltm.known_nodes)
            } else {
                None
            }
        });
        if let Some(full_path) = path_opt
            && full_path.len() > 1
            && let Some(a) = delta.actors.get_mut(&aid)
        {
            if is_offline_pl {
                a.mind.ltm.known_nodes.insert(dst_nid.clone());
            }
            a.route_plan = full_path.into_iter().skip(1).collect();
            if a.mind.stm.intentions.len() >= 4 {
                a.mind.stm.intentions.pop_front();
            }
            a.mind.stm.intentions.push_back(ActiveIntention {
                kind: kind.to_string(),
                target_node: dst_nid,
                reason_zh: reason,
                created_bell: abs_bell,
            });
            a.mark_mutated();
        }
    }
}

/// 邁步遷移並寫入「局部方向目擊記憶（僅知哪道門離開、不知遠端現址）」＋「雙軌防洗版在場呼吸流」。
pub fn relocate_actor_with_local_witness(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_id: &str,
    from_nid: &str,
    to_nid: &str,
    to_cell: u32,
    knows_state: bool,
) {
    relocate_actor_at_xy_with_local_witness(
        doc,
        delta,
        clock,
        actor_id,
        from_nid,
        to_nid,
        to_cell,
        knows_state,
        None,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "relocation with explicit tile coordinate preservation"
)]
pub fn relocate_actor_at_xy_with_local_witness(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_id: &str,
    from_nid: &str,
    to_nid: &str,
    to_cell: u32,
    knows_state: bool,
    xy_override: Option<[f32; 2]>,
) {
    if from_nid == to_nid {
        return;
    }
    let Some((mover_name, mover_role, mover_work, mover_home, mover_ctrl)) =
        delta.actors.get(actor_id).map(|a| {
            (
                a.display_name.clone(),
                a.role.clone(),
                a.work_node.clone(),
                a.home_node.clone(),
                a.controller_id.is_some(),
            )
        })
    else {
        return;
    };

    let from_short = doc.nodes.get(from_nid).map_or("街口", |n| {
        n.name.split('・').next_back().unwrap_or(&n.name)
    });
    let to_short = doc.nodes.get(to_nid).map_or("街口", |n| {
        n.name.split('・').next_back().unwrap_or(&n.name)
    });

    let td = derive_tide_clock_details(clock);
    let stamp = format!("{:02}:{:02}", td.hour, td.minute);

    // 1. 起點在場目擊者：先唯讀過濾避免無謂 CoW 深拷貝，寫入目擊記憶後確實標記 `mark_mutated()` 落盤！
    if let Some(leavers) = delta.node_occupants.get(from_nid).cloned() {
        let crowd_len = leavers.len();
        for oid in leavers {
            if oid == actor_id
                || !delta
                    .actors
                    .get(&oid)
                    .is_some_and(|o| o.alive && o.setup_done)
            {
                continue;
            }
            let Some(obs) = delta.actors.get_mut(&oid) else {
                continue;
            };
            let known = obs.is_acquainted_with(actor_id)
                || (!mover_ctrl && (mover_work == from_nid || mover_home == from_nid));
            let shown = if known {
                mover_name.clone()
            } else {
                format!("一名{}", mover_role)
            };
            let sal = if obs.work_node == from_nid || known {
                32
            } else {
                26
            };
            let ev_id = format!(
                "step_out:{}:{}:{}:{}:{}",
                clock.game_days, td.hour, td.minute, actor_id, from_nid
            );
            let langs = obs.languages.clone();
            let line_for_mem = format!("見{}往{}方向離開", shown, to_short);
            let mut touched = obs.mind.ingest_sensory_event(
                &langs,
                EventPublicity::PublicLoud,
                None,
                false,
                &ev_id,
                clock.game_days,
                clock.bell_index,
                from_nid,
                actor_id,
                &shown,
                EvidenceTier::DirectWitness,
                sal,
                6,
                ValueAxis::SurvivalSafety,
                &line_for_mem,
            );
            if obs.controller_id.is_some() && (known || crowd_len <= 3) {
                prune_stale_day_window_keys(&mut obs.explored_windows, clock.game_days);
                let throttle_key = format!(
                    "foot_echo:{}:{}:{}",
                    clock.game_days,
                    td.hour,
                    td.minute / 2
                );
                if known || obs.explored_windows.insert(throttle_key) {
                    push_breath(obs, format!("〔{stamp}〕{shown}往{to_short}方向離開了。"));
                    touched = true;
                }
            }
            if touched {
                obs.mark_mutated();
            }
        }
    }

    delta.relocate_actor(actor_id, to_nid, to_cell, knows_state);
    if let Some([gx, gy]) = xy_override
        && let Some(a) = delta.actors.get_mut(actor_id)
    {
        a.map_gx = gx;
        a.map_gy = gy;
        a.map_pos_seeded = true;
    }
    super::map_memory::record_local_witness_at_cur_node(doc, delta, actor_id, clock.game_days);

    // 2. 終點在場觀察者：唯讀篩出真人玩家後方呼叫 `get_mut`，杜絕同場 NPC 被無故觸發 CoW 深拷貝！
    if let Some(arrivers) = delta.node_occupants.get(to_nid).cloned() {
        let crowd_len = arrivers.len();
        for oid in arrivers {
            if oid == actor_id
                || !delta
                    .actors
                    .get(&oid)
                    .is_some_and(|o| o.alive && o.setup_done && o.controller_id.is_some())
            {
                continue;
            }
            let Some(obs) = delta.actors.get_mut(&oid) else {
                continue;
            };
            let known = obs.is_acquainted_with(actor_id)
                || (!mover_ctrl && (mover_work == to_nid || mover_home == to_nid));
            let shown = if known {
                mover_name.clone()
            } else {
                format!("一名{}", mover_role)
            };
            prune_stale_day_window_keys(&mut obs.explored_windows, clock.game_days);
            let throttle_key = format!("foot_in:{}:{}:{}", clock.game_days, td.hour, td.minute / 2);
            if known || (crowd_len <= 3 && obs.explored_windows.insert(throttle_key)) {
                push_breath(
                    obs,
                    format!("〔{stamp}〕{shown}從{from_short}方向走了過來。"),
                );
                obs.mark_mutated();
            }
        }
    }
}
