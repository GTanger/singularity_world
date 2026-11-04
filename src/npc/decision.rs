//! NPC 決策引擎（對齊既有 `npc/decision`）：`Decide` 產生意圖，`ResolveBrainPath` 轉成尋路序列。
// 結構刻意保留多層判斷，合併 if 反而難與原版對照。
#![allow(clippy::collapsible_if)]

use std::time::{SystemTime, UNIX_EPOCH};

use rand::Rng;

use crate::db::{
    EVT_TALK, Personality, RoomGraph, active_trade_inventory_total_qty,
    expand_soul_seed_to_personality, get_all_venue_room_ids, get_assignments_for_entity,
    get_entity, get_recent_events,
};

/// 生存層鎂閾值（對齊既有 `SurvivalLineMg`）。
pub const SURVIVAL_LINE_MG: i32 = 50;
/// 社交衰減週期秒（對齊既有 `socialDecaySec`）。
const SOCIAL_DECAY_SEC: i64 = 7200;
/// 社交或進出門戶語意標籤常數（涵蓋既有英文與半島中文房間標籤）。
pub const SOCIAL_OR_GATE_TAGS: &[&str] = &[
    "social", "gate", "廣場", "石橋", "市集", "茶屋", "食堂", "公議",
];
/// 野外戶外語意標籤常數（涵蓋既有英文與半島中文房間標籤）。
pub const WILDERNESS_OUTDOOR_TAGS: &[&str] = &[
    "wilderness", "outdoor", "林場", "礦場", "漁場", "獵場", "農田", "水井",
];
/// 茶屋客棧或休憩語意標籤常數（涵蓋既有英文與半島中文房間標籤）。
pub const TAVERN_OR_REST_TAGS: &[&str] = &["tavern", "inn", "茶屋", "旅社", "浴場", "食堂"];
/// 漫遊目標語意標籤常數（涵蓋既有英文與半島中文房間標籤）。
pub const WANDER_TARGET_TAGS: &[&str] = &[
    "social", "gate", "outdoor", "廣場", "石橋", "市集", "茶屋", "水井",
];


/// 意圖類型（對齊既有 `IntentType`）。
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

/// 決策輸出：單一意圖與可選目標房（對齊既有 `Intent`）。
#[derive(Debug, Clone, Default)]
pub struct Intent {
    pub ty: IntentType,
    pub target_room_id: String,
}

/// 決策輸入：NPC 自身狀態（對齊既有 `DecisionState`）。
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

/// 世界情境（對齊既有 `DecisionContext`）。
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

/// 由 store 組裝決策狀態（對齊既有 `BuildDecisionState`）。
pub fn build_decision_state(entity_id: &str) -> anyhow::Result<DecisionState> {
    let Some(ch) = get_entity(entity_id)? else {
        anyhow::bail!("entity not found");
    };
    let assignments = get_assignments_for_entity(entity_id)?;
    let has_assignment = !assignments.is_empty();
    let (personality, has_personality) = if let Some(seed) = ch.soul_seed {
        (expand_soul_seed_to_personality(seed), true)
    } else {
        (
            Personality {
                boldness: 0.5,
                sensitivity: 0.5,
                orderliness: 0.5,
            },
            false,
        )
    };
    let inv_qty = active_trade_inventory_total_qty(&ch.inventory);
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

/// 組裝周邊情境（對齊既有 `BuildDecisionContext`）。
pub fn build_decision_context(
    g: &RoomGraph,
    current_room_id: &str,
    max_dist: i32,
) -> DecisionContext {
    let mut md = max_dist;
    if md <= 0 {
        md = 20;
    }
    let venue_rooms = get_all_venue_room_ids().unwrap_or_default();
    DecisionContext {
        room_tags: g.room_tags(current_room_id),
        venue_room_ids_in_range: filter_rooms_within_dist(g, current_room_id, &venue_rooms, md),
        social_or_gate_in_range: g.find_rooms_within_dist(current_room_id, SOCIAL_OR_GATE_TAGS, md),
        wilderness_outdoor_in_range: g.find_rooms_within_dist(
            current_room_id,
            WILDERNESS_OUTDOOR_TAGS,
            md,
        ),
        tavern_in_range: g.find_rooms_within_dist(current_room_id, TAVERN_OR_REST_TAGS, md),
    }
}

fn filter_rooms_within_dist(
    g: &RoomGraph,
    origin: &str,
    candidates: &[String],
    max_dist: i32,
) -> Vec<String> {
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
    if has_assignment { 0.0 } else { 0.8 }
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

/// 意圖慣性加權倍率：正在執行的意圖享有此倍率優勢，避免每 tick 失憶跳來跳去。
const INTENT_INERTIA_MULTIPLIER: f64 = 1.8;

/// 評分引擎：加權隨機選意圖。
/// `prev_intent` 為上一次意圖類型，若非 None 則給予慣性加權，使行為具連貫性。
/// 無慣性需求時傳 `None` 即可。
pub fn decide_with_inertia(
    state: DecisionState,
    ctx: &DecisionContext,
    prev_intent: Option<IntentType>,
) -> Intent {
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

    if has_tag(&ctx.room_tags, SOCIAL_OR_GATE_TAGS) || !ctx.social_or_gate_in_range.is_empty() {
        let mut score = survival;
        if has_pers {
            score *= (1.0 - p.boldness * 0.5) * (0.5 + p.sensitivity * 0.5);
        }
        if score > 0.05 {
            let target = if !has_tag(&ctx.room_tags, SOCIAL_OR_GATE_TAGS) {
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

    if state.inventory_total_qty > 0
        && survival > 0.05
        && (!ctx.social_or_gate_in_range.is_empty() || has_tag(&ctx.room_tags, SOCIAL_OR_GATE_TAGS))
    {
        let mut score = survival * 0.45;
        if has_pers {
            score *= 0.5 + p.boldness * 0.5;
        }
        if score > 0.05 {
            let target = if !has_tag(&ctx.room_tags, SOCIAL_OR_GATE_TAGS) {
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
            let target = if !has_tag(&ctx.room_tags, TAVERN_OR_REST_TAGS) {
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

    // 意圖慣性：如果上一次有意圖，同類型候選享有加權優勢
    if let Some(prev) = prev_intent {
        for c in &mut cands {
            if c.intent.ty == prev {
                c.weight *= INTENT_INERTIA_MULTIPLIER;
            }
        }
    }

    weighted_random_select(&cands)
}

/// 依意圖解析尋路目標序列（對齊既有 `ResolveBrainPath`）。
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
            if let Some((room, _)) = SOCIAL_OR_GATE_TAGS
                .iter()
                .find_map(|tag| g.find_nearest_by_tag(current_room_id, tag, md))
            {
                if room != current_room_id {
                    if let Some(p) = g.find_path(current_room_id, &room) {
                        return p;
                    }
                }
            }
        }
        IntentType::Beg => {
            let room = SOCIAL_OR_GATE_TAGS.iter().find_map(|tag| {
                g.find_nearest_by_tag(current_room_id, tag, md)
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
            let room = SOCIAL_OR_GATE_TAGS.iter().find_map(|tag| {
                g.find_nearest_by_tag(current_room_id, tag, md)
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
            let room = WILDERNESS_OUTDOOR_TAGS.iter().find_map(|tag| {
                g.find_nearest_by_tag(current_room_id, tag, md)
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
            for tag in TAVERN_OR_REST_TAGS {
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
            let candidates =
                g.find_rooms_within_dist(current_room_id, WANDER_TARGET_TAGS, md);
            let filtered: Vec<String> = candidates
                .into_iter()
                .filter(|c| c != current_room_id)
                .collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Personality;

    #[test]
    fn test_decide_with_chinese_semantic_room_tags() {
        // 1. 測試在「廣場」房間且處於低鎂生存危機時，會生成乞討意圖 (IntentType::Beg)
        let state_beg = DecisionState {
            magnesium: 0,
            has_assignment: true,
            personality: Personality {
                boldness: 0.0,
                sensitivity: 1.0,
                orderliness: 1.5,
            },
            has_personality: true,
            last_talk_at: 0,
            disposition: 0,
            inventory_total_qty: 0,
        };
        let ctx_square = DecisionContext {
            room_tags: vec!["廣場".to_string()],
            venue_room_ids_in_range: vec![],
            social_or_gate_in_range: vec![],
            wilderness_outdoor_in_range: vec![],
            tavern_in_range: vec![],
        };
        let intent_beg = decide_with_inertia(state_beg, &ctx_square, Some(IntentType::Beg));
        assert_eq!(intent_beg.ty, IntentType::Beg);
        assert!(intent_beg.target_room_id.is_empty());

        // 2. 測試有庫存物資且在「市集」周邊時，生成交易意圖 (IntentType::Trade)
        let state_trade = DecisionState {
            magnesium: 0,
            has_assignment: true,
            personality: Personality {
                boldness: 1.0,
                sensitivity: 0.0,
                orderliness: 1.5,
            },
            has_personality: true,
            last_talk_at: 0,
            disposition: 0,
            inventory_total_qty: 5,
        };
        let ctx_market = DecisionContext {
            room_tags: vec![],
            venue_room_ids_in_range: vec![],
            social_or_gate_in_range: vec!["水尾寮渡頭市集".to_string()],
            wilderness_outdoor_in_range: vec![],
            tavern_in_range: vec![],
        };
        let saw_trade = (0..32).any(|_| {
            let it = decide_with_inertia(state_trade.clone(), &ctx_market, Some(IntentType::Trade));
            it.ty == IntentType::Trade && it.target_room_id == "水尾寮渡頭市集"
        });
        assert!(saw_trade);

        // 3. 測試長時間未交談且周邊有「茶屋」/「浴場」等休憩場所時，生成社交意圖 (IntentType::Socialize)
        let state_social = DecisionState {
            magnesium: 100,
            has_assignment: true,
            personality: Personality {
                boldness: 0.5,
                sensitivity: 1.0,
                orderliness: 1.5,
            },
            has_personality: true,
            last_talk_at: 1,
            disposition: 100,
            inventory_total_qty: 0,
        };
        let ctx_tea_bath = DecisionContext {
            room_tags: vec![],
            venue_room_ids_in_range: vec![],
            social_or_gate_in_range: vec![],
            wilderness_outdoor_in_range: vec![],
            tavern_in_range: vec!["港邊茶屋".to_string(), "溫泉浴場".to_string()],
        };
        let intent_social =
            decide_with_inertia(state_social, &ctx_tea_bath, Some(IntentType::Socialize));
        assert_eq!(intent_social.ty, IntentType::Socialize);
        assert!(
            intent_social.target_room_id == "港邊茶屋"
                || intent_social.target_room_id == "溫泉浴場"
        );
    }
}
