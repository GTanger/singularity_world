//! NPC 決策引擎（對齊 Go `npc/decision.go`）：`Decide` 產生意圖，`ResolveBrainPath` 轉成尋路序列。
// 結構刻意鏡像 Go 之多層判斷，合併 if 反而難與原版對照。
#![allow(clippy::collapsible_if)]

use std::time::{SystemTime, UNIX_EPOCH};

use rand::Rng;

use crate::db::{
    expand_soul_seed_to_personality, get_all_venue_room_ids, get_assignments_for_entity, get_entity,
    get_recent_events, inventory_total_qty, Personality, RoomGraph, EVT_TALK,
};

/// 生存層鎂閾值（對齊 Go `SurvivalLineMg`）。
pub const SURVIVAL_LINE_MG: i32 = 50;
/// 社交衰減週期秒（對齊 Go `socialDecaySec`）。
const SOCIAL_DECAY_SEC: i64 = 7200;

/// 意圖類型（對齊 Go `IntentType`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IntentType {
    SeekJob,
    Beg,
    Trade,
    Gather,
    #[default]
    Wander,
    Work,
    Idle,
    Socialize,
}

impl IntentType {
    /// 對應 `gametext.json` brain_arrival 鍵與事件語意。
    #[must_use]
    pub fn as_key(&self) -> &'static str {
        match self {
            Self::SeekJob => "seek_job",
            Self::Beg => "beg",
            Self::Trade => "trade",
            Self::Gather => "gather",
            Self::Wander => "wander",
            Self::Work => "work",
            Self::Idle => "idle",
            Self::Socialize => "socialize",
        }
    }
}

/// 決策輸出：單一意圖與可選目標房（對齊 Go `Intent`）。
#[derive(Debug, Clone, Default)]
pub struct Intent {
    pub ty: IntentType,
    pub target_room_id: String,
}

/// 決策輸入：NPC 自身狀態（對齊 Go `DecisionState`）。
#[derive(Debug, Clone)]
pub struct DecisionState {
    pub magnesium: i32,
    pub has_assignment: bool,
    pub personality: Personality,
    pub has_personality: bool,
    pub last_talk_at: i64,
    pub disposition: i32,
    pub inventory_total_qty: i32,
}

/// 世界情境（對齊 Go `DecisionContext`）。
#[derive(Debug, Clone, Default)]
pub struct DecisionContext {
    pub room_tags: Vec<String>,
    pub venue_room_ids_in_range: Vec<String>,
    pub social_or_gate_in_range: Vec<String>,
    pub wilderness_outdoor_in_range: Vec<String>,
    pub tavern_in_range: Vec<String>,
}

struct Candidate {
    intent: Intent,
    weight: f64,
}

/// 由 store 組裝決策狀態（對齊 Go `BuildDecisionState`）。
pub fn build_decision_state(entity_id: &str) -> anyhow::Result<DecisionState> {
    let Some(ch) = get_entity(entity_id)? else {
        anyhow::bail!("entity not found");
    };
    let assignments = get_assignments_for_entity(entity_id)?;
    let has_assignment = !assignments.is_empty();
    let (personality, has_personality) = if let Some(seed) = ch.soul_seed {
        (expand_soul_seed_to_personality(seed), true)
    } else {
        (Personality {
            boldness: 0.5,
            sensitivity: 0.5,
            orderliness: 0.5,
        }, false)
    };
    let inv_qty = inventory_total_qty(&ch.inventory);
    let mut last_talk_at: i64 = 0;
    for e in get_recent_events(entity_id, 10) {
        if e.event_type == EVT_TALK {
            last_talk_at = e.at;
            break;
        }
    }
    Ok(DecisionState {
        magnesium: ch.magnesium,
        has_assignment,
        personality,
        has_personality,
        last_talk_at,
        disposition: ch.disposition,
        inventory_total_qty: inv_qty,
    })
}

/// 組裝周邊情境（對齊 Go `BuildDecisionContext`）。
pub fn build_decision_context(g: &RoomGraph, current_room_id: &str, max_dist: i32) -> DecisionContext {
    let mut md = max_dist;
    if md <= 0 {
        md = 20;
    }
    let venue_rooms = get_all_venue_room_ids().unwrap_or_default();
    DecisionContext {
        room_tags: g.room_tags(current_room_id),
        venue_room_ids_in_range: filter_rooms_within_dist(g, current_room_id, &venue_rooms, md),
        social_or_gate_in_range: g.find_rooms_within_dist(current_room_id, &["social", "gate"], md),
        wilderness_outdoor_in_range: g.find_rooms_within_dist(
            current_room_id,
            &["wilderness", "outdoor"],
            md,
        ),
        tavern_in_range: g.find_rooms_within_dist(current_room_id, &["tavern", "inn"], md),
    }
}

fn filter_rooms_within_dist(g: &RoomGraph, origin: &str, candidates: &[String], max_dist: i32) -> Vec<String> {
    if candidates.is_empty() || max_dist <= 0 {
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::new();
    for c in candidates {
        if c == origin {
            out.push(c.clone());
            continue;
        }
        if let Some(p) = g.find_path(origin, c) {
            if !p.is_empty() && p.len() <= max_dist as usize {
                out.push(c.clone());
            }
        }
    }
    out
}

fn has_tag(tags: &[String], want: &[&str]) -> bool {
    for t in tags {
        for w in want {
            if t == w {
                return true;
            }
        }
    }
    false
}

fn pick_random<T: Clone>(xs: &[T]) -> T {
    let mut rng = rand::rng();
    xs[rng.random_range(0..xs.len())].clone()
}

fn urgency_survival(mg: i32) -> f64 {
    if mg >= SURVIVAL_LINE_MG {
        return 0.0;
    }
    let v = 1.0 - (f64::from(mg) / f64::from(SURVIVAL_LINE_MG));
    v.clamp(0.0, 1.0)
}

fn urgency_stability(has_assignment: bool) -> f64 {
    if has_assignment {
        0.0
    } else {
        0.8
    }
}

fn urgency_social(last_talk_at: i64, disposition: i32) -> f64 {
    if last_talk_at <= 0 {
        return 0.0;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let elapsed = now.saturating_sub(last_talk_at).max(0);
    let u = (elapsed as f64 / SOCIAL_DECAY_SEC as f64).min(1.0);
    let disp_factor = (f64::from(disposition) + 100.0) / 200.0;
    u * disp_factor
}

fn weighted_random_select(cands: &[Candidate]) -> Intent {
    if cands.is_empty() {
        return Intent {
            ty: IntentType::Wander,
            target_room_id: String::new(),
        };
    }
    let total: f64 = cands.iter().map(|c| c.weight).sum();
    if total <= 0.0 {
        return cands[0].intent.clone();
    }
    let mut rng = rand::rng();
    let mut r = rng.random_range(0.0..total);
    for c in cands {
        r -= c.weight;
        if r <= 0.0 {
            return c.intent.clone();
        }
    }
    cands.last().expect("cands non-empty").intent.clone()
}

/// 評分引擎：加權隨機選意圖（對齊 Go `Decide`）。
pub fn decide(state: DecisionState, ctx: &DecisionContext) -> Intent {
    let survival = urgency_survival(state.magnesium);
    let stability = urgency_stability(state.has_assignment);
    let social = urgency_social(state.last_talk_at, state.disposition);
    let p = state.personality;
    let has_pers = state.has_personality;
    let mut cands: Vec<Candidate> = Vec::new();

    if !ctx.venue_room_ids_in_range.is_empty() {
        let mut score = stability + survival * 0.5;
        if has_pers {
            score *= 0.5 + p.orderliness * 0.5;
        }
        if score > 0.05 {
            cands.push(Candidate {
                intent: Intent {
                    ty: IntentType::SeekJob,
                    target_room_id: pick_random(&ctx.venue_room_ids_in_range),
                },
                weight: score,
            });
        }
    }

    if has_tag(&ctx.room_tags, &["gate", "social"]) || !ctx.social_or_gate_in_range.is_empty() {
        let mut score = survival;
        if has_pers {
            score *= (1.0 - p.boldness * 0.5) * (0.5 + p.sensitivity * 0.5);
        }
        if score > 0.05 {
            let target = if !has_tag(&ctx.room_tags, &["gate", "social"]) {
                pick_random(&ctx.social_or_gate_in_range)
            } else {
                String::new()
            };
            cands.push(Candidate {
                intent: Intent {
                    ty: IntentType::Beg,
                    target_room_id: target,
                },
                weight: score,
            });
        }
    }

    if !ctx.wilderness_outdoor_in_range.is_empty() {
        let mut score = survival;
        if has_pers {
            score *= (0.5 + p.boldness * 0.5) * (1.0 - p.orderliness * 0.5);
        }
        if score > 0.05 {
            cands.push(Candidate {
                intent: Intent {
                    ty: IntentType::Gather,
                    target_room_id: pick_random(&ctx.wilderness_outdoor_in_range),
                },
                weight: score,
            });
        }
    }

    if state.inventory_total_qty > 0 && survival > 0.05
        && (!ctx.social_or_gate_in_range.is_empty() || has_tag(&ctx.room_tags, &["gate", "social"]))
    {
        let mut score = survival * 0.45;
        if has_pers {
            score *= 0.5 + p.boldness * 0.5;
        }
        if score > 0.05 {
            let target = if !has_tag(&ctx.room_tags, &["gate", "social"]) {
                pick_random(&ctx.social_or_gate_in_range)
            } else {
                String::new()
            };
            cands.push(Candidate {
                intent: Intent {
                    ty: IntentType::Trade,
                    target_room_id: target,
                },
                weight: score,
            });
        }
    }

    if !ctx.tavern_in_range.is_empty() && social > 0.0 {
        let mut score = social;
        if has_pers {
            score *= 0.5 + p.sensitivity * 0.5;
        }
        if score > 0.05 {
            let target = if !has_tag(&ctx.room_tags, &["tavern", "inn", "social"]) {
                pick_random(&ctx.tavern_in_range)
            } else {
                String::new()
            };
            cands.push(Candidate {
                intent: Intent {
                    ty: IntentType::Socialize,
                    target_room_id: target,
                },
                weight: score,
            });
        }
    }

    if state.has_assignment && survival == 0.0 {
        if has_pers && p.boldness > 0.6 && !ctx.wilderness_outdoor_in_range.is_empty() {
            cands.push(Candidate {
                intent: Intent {
                    ty: IntentType::Gather,
                    target_room_id: pick_random(&ctx.wilderness_outdoor_in_range),
                },
                weight: 0.3 * p.boldness,
            });
        }
        if has_pers && p.orderliness > 0.6 && !ctx.venue_room_ids_in_range.is_empty() {
            cands.push(Candidate {
                intent: Intent {
                    ty: IntentType::Work,
                    target_room_id: ctx.venue_room_ids_in_range[0].clone(),
                },
                weight: 0.3 * p.orderliness,
            });
        }
    }

    let mut wander_score = 0.4;
    if has_pers {
        wander_score *= 1.5 - p.orderliness;
    }
    cands.push(Candidate {
        intent: Intent {
            ty: IntentType::Wander,
            target_room_id: String::new(),
        },
        weight: wander_score,
    });

    weighted_random_select(&cands)
}

/// 依意圖解析尋路目標序列（對齊 Go `ResolveBrainPath`）。
pub fn resolve_brain_path(
    g: &RoomGraph,
    intent: &Intent,
    current_room_id: &str,
    ctx: &DecisionContext,
    max_dist: i32,
) -> Vec<String> {
    let mut md = max_dist;
    if md <= 0 {
        md = 25;
    }
    if !intent.target_room_id.is_empty() && intent.target_room_id != current_room_id {
        if let Some(path) = g.find_path(current_room_id, &intent.target_room_id) {
            if !path.is_empty() {
                return path;
            }
        }
    }
    match intent.ty {
        IntentType::SeekJob => {
            if !ctx.venue_room_ids_in_range.is_empty() {
                let target = pick_random(&ctx.venue_room_ids_in_range);
                if target != current_room_id {
                    if let Some(p) = g.find_path(current_room_id, &target) {
                        if !p.is_empty() {
                            return p;
                        }
                    }
                }
            }
            if let Some((room, _)) = g.find_nearest_by_tag(current_room_id, "social", md) {
                if room != current_room_id {
                    if let Some(p) = g.find_path(current_room_id, &room) {
                        return p;
                    }
                }
            }
        }
        IntentType::Beg => {
            let room = g
                .find_nearest_by_tag(current_room_id, "gate", md)
                .map(|(r, _)| r)
                .filter(|r| r != current_room_id)
                .or_else(|| {
                    g.find_nearest_by_tag(current_room_id, "social", md)
                        .map(|(r, _)| r)
                        .filter(|r| r != current_room_id)
                });
            if let Some(r) = room {
                if let Some(p) = g.find_path(current_room_id, &r) {
                    return p;
                }
            }
        }
        IntentType::Trade => {
            let room = g
                .find_nearest_by_tag(current_room_id, "social", md)
                .map(|(r, _)| r)
                .filter(|r| r != current_room_id)
                .or_else(|| {
                    g.find_nearest_by_tag(current_room_id, "gate", md)
                        .map(|(r, _)| r)
                        .filter(|r| r != current_room_id)
                });
            if let Some(r) = room {
                if let Some(p) = g.find_path(current_room_id, &r) {
                    return p;
                }
            }
        }
        IntentType::Gather => {
            let room = g
                .find_nearest_by_tag(current_room_id, "wilderness", md)
                .map(|(r, _)| r)
                .filter(|r| r != current_room_id)
                .or_else(|| {
                    g.find_nearest_by_tag(current_room_id, "outdoor", md)
                        .map(|(r, _)| r)
                        .filter(|r| r != current_room_id)
                });
            if let Some(r) = room {
                if let Some(p) = g.find_path(current_room_id, &r) {
                    return p;
                }
            }
        }
        IntentType::Work => {
            if !ctx.venue_room_ids_in_range.is_empty() {
                let target = ctx.venue_room_ids_in_range[0].clone();
                if target != current_room_id {
                    if let Some(p) = g.find_path(current_room_id, &target) {
                        if !p.is_empty() {
                            return p;
                        }
                    }
                }
            }
        }
        IntentType::Socialize => {
            if !ctx.tavern_in_range.is_empty() {
                let target = pick_random(&ctx.tavern_in_range);
                if target != current_room_id {
                    if let Some(p) = g.find_path(current_room_id, &target) {
                        if !p.is_empty() {
                            return p;
                        }
                    }
                }
            }
            for tag in ["tavern", "inn", "social"] {
                if let Some((room, _)) = g.find_nearest_by_tag(current_room_id, tag, md) {
                    if room != current_room_id {
                        if let Some(p) = g.find_path(current_room_id, &room) {
                            return p;
                        }
                    }
                }
            }
        }
        IntentType::Wander => {
            let candidates = g.find_rooms_within_dist(current_room_id, &["social", "gate", "outdoor"], md);
            let filtered: Vec<String> = candidates.into_iter().filter(|c| c != current_room_id).collect();
            if !filtered.is_empty() {
                let target = pick_random(&filtered);
                if let Some(p) = g.find_path(current_room_id, &target) {
                    return p;
                }
            }
        }
        IntentType::Idle => {}
    }
    Vec::new()
}
