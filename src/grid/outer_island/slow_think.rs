//! 全服 `UnifiedMindBody` System 2 慢思考高張力優先佇列（廢除 `rust_slow` 假模板洗版、超額保留分數延期、全中文封閉方針與數日期限落地）。

use crate::ai::talk::call_openai_compatible_completions;

use super::appraisal::{ConcernCause, GoalStrategyKind};
use super::desire_sim::{find_confidant_in_known_nodes, find_known_wild_forage_node};
use super::dialogue_llm::validate_clean_in_world_reply;
use super::persona::{ValueAxis, derive_innate_persona};
use super::surface_cues::render_concern_inner_thought_zh;
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

const SLOW_THINK_URL: &str = "http://127.0.0.1:11436/v1";
const SLOW_THINK_MODEL: &str = "/home/tanger/Models/Qwen3-4B-Instruct-2507-Q8_0.gguf";

pub struct SlowThinkCandidate {
    pub actor_id: String,
    pub last_tp_anchor: Option<String>,
    pub saved_score: u32,
    pub game_day: i64,
    pub focus_target_id: Option<String>,
    pub prompt: String,
    pub fallback_strategy: GoalStrategyKind,
}

pub fn effective_slow_priority(a: &UnifiedMindBody) -> u32 {
    if a.mind.pending_slow_think_score == 0 {
        return 0;
    }
    a.mind
        .pending_slow_think_score
        .saturating_add(u32::from(a.mind.slow_think_wait_bells) * 12)
}

fn infer_default_strategy_for_actor(
    a: &UnifiedMindBody,
    owes_debt: bool,
) -> (GoalStrategyKind, Option<String>) {
    if let Some(top) = a.mind.top_concern() {
        match top.cause {
            ConcernCause::DebtDefaulted => {
                return (GoalStrategyKind::ClearDebt, top.target_id.clone());
            }
            ConcernCause::DebtPressured => {
                if owes_debt {
                    return (GoalStrategyKind::ClearDebt, top.target_id.clone());
                }
                return (GoalStrategyKind::AvoidRift, top.target_id.clone());
            }
            ConcernCause::RivalEncounter | ConcernCause::KinshipRift => {
                return (GoalStrategyKind::AvoidRift, top.target_id.clone());
            }
            ConcernCause::LivelihoodShortfall => {
                if a.copper <= 2 && !a.role.contains("掌櫃") {
                    return (GoalStrategyKind::ShiftLivelihood, None);
                }
                if a.copper < 6 {
                    return (GoalStrategyKind::ForageSupplement, None);
                }
                return (GoalStrategyKind::KinMutualAid, None);
            }
            ConcernCause::ToolDamaged | ConcernCause::CraftBreakthrough => {
                if a.bound_weapon.wear_stage > 0 {
                    return (GoalStrategyKind::HoneCraft, None);
                }
            }
            ConcernCause::CrossTownYearning => {
                return (GoalStrategyKind::SeekCrossTownNews, top.target_id.clone());
            }
            ConcernCause::PactBetrayed => {
                let innate = derive_innate_persona(a.soul_seed.max(1));
                if innate.top_value_axis(&a.drift) == ValueAxis::KinshipLoyalty {
                    return (GoalStrategyKind::KinMutualAid, None);
                }
                return (GoalStrategyKind::AvoidRift, top.target_id.clone());
            }
            ConcernCause::KinshipWarmth | ConcernCause::PactHonored => {}
        }
    }
    if a.bound_weapon.wear_stage > 0 {
        (GoalStrategyKind::HoneCraft, None)
    } else if a.copper < 8 {
        (GoalStrategyKind::ForageSupplement, None)
    } else if a
        .mind
        .ltm
        .social
        .values()
        .any(|s| s.warmth >= 25 && !s.target_region_zh.is_empty())
    {
        (GoalStrategyKind::SeekCrossTownNews, None)
    } else {
        (GoalStrategyKind::KinMutualAid, None)
    }
}

/// R4-M3 修正：掛載中程方針前驗證該角色物理上確有對應路徑／知己／欠債／器物磨損，若不可行則退回可執行方針（或 None 免空窗卡死）
pub fn coerce_feasible_strategy(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    actor_id: &str,
    preferred: GoalStrategyKind,
    owes_debt: bool,
) -> Option<GoalStrategyKind> {
    let a = delta.actors.get(actor_id)?;
    let can_forage = find_known_wild_forage_node(doc, a).is_some();
    let can_confide = find_confidant_in_known_nodes(delta, a).is_some();
    let can_tavern = a.copper >= 8
        && a.mind.ltm.known_nodes.iter().any(|nid| {
            doc.nodes
                .get(nid)
                .is_some_and(|n| n.kind == "shop" && n.name.contains("酒館"))
        });
    let can_hone = a.bound_weapon.wear_stage > 0;
    let has_rift = a.mind.ltm.social.values().any(|s| s.warmth <= -22);
    let has_broken_bridge = a.mind.ltm.known_nodes.iter().any(|nid| {
        doc.nodes.get(nid).is_some_and(|n| {
            delta
                .live_land
                .cells
                .get(&n.cell_id)
                .is_some_and(|cl| cl.has_river_crossing && cl.bridge_integrity < 100)
        })
    });
    let has_reclaimable = a.mind.ltm.known_nodes.iter().any(|nid| {
        doc.nodes.get(nid).is_some_and(|n| {
            doc.cells
                .get(&n.cell_id.to_string())
                .is_some_and(|c| c.kind != "settlement" && !n.dungeon)
                && delta
                    .live_land
                    .cells
                    .get(&n.cell_id)
                    .is_some_and(|cl| cl.reclaimed_plots < 4 && cl.flood_level == 0)
        })
    });
    let can_shift = !a.role.contains("掌櫃") && a.copper <= 6;

    let is_ok = |k: GoalStrategyKind| -> bool {
        match k {
            GoalStrategyKind::ClearDebt => owes_debt,
            GoalStrategyKind::HoneCraft => can_hone,
            GoalStrategyKind::KinMutualAid => can_confide,
            GoalStrategyKind::ForageSupplement => can_forage,
            GoalStrategyKind::SeekCrossTownNews => can_tavern,
            GoalStrategyKind::AvoidRift => has_rift,
            GoalStrategyKind::ReclaimLand => has_reclaimable,
            GoalStrategyKind::RepairCivicWorks => has_broken_bridge,
            GoalStrategyKind::ShiftLivelihood => can_shift,
        }
    };
    if is_ok(preferred) {
        return Some(preferred);
    }
    [
        GoalStrategyKind::ClearDebt,
        GoalStrategyKind::ShiftLivelihood,
        GoalStrategyKind::RepairCivicWorks,
        GoalStrategyKind::ReclaimLand,
        GoalStrategyKind::HoneCraft,
        GoalStrategyKind::ForageSupplement,
        GoalStrategyKind::KinMutualAid,
        GoalStrategyKind::SeekCrossTownNews,
        GoalStrategyKind::AvoidRift,
    ]
    .into_iter()
    .find(|&k| is_ok(k))
}

/// 遮罩真實玩家名號、曾用名與帳號代號為「外來旅人」，依字元長度降序替換，杜絕前綴改名殘留尾字或外洩玩家隱私
pub fn mask_player_names_in_text(delta: &IslandWorldDelta, raw: &str) -> String {
    let mut tokens: Vec<&str> = Vec::new();
    for (aid, act) in &delta.actors {
        if act.controller_id.is_some() || aid.starts_with("pl:") {
            if !act.display_name.is_empty() {
                tokens.push(act.display_name.as_str());
            }
            for old in &act.mind.ltm.mastery.past_aliases {
                if !old.is_empty() {
                    tokens.push(old.as_str());
                }
            }
            if let Some(ref cid) = act.controller_id
                && cid.len() >= 2
            {
                tokens.push(cid.as_str());
            }
        }
    }
    tokens.sort_by(|a, b| b.chars().count().cmp(&a.chars().count()).then(a.cmp(b)));
    tokens.dedup();
    let mut out = raw.to_string();
    for tok in tokens {
        if out.contains(tok) {
            out = out.replace(tok, "外來旅人");
        }
    }
    out
}

/// 解析 `11436` 全中文單行輸出（格式：`定見：<短句>｜方針：<六選一>`），避開英文字母過濾衝突
pub fn parse_slow_think_chinese_output(
    raw: &str,
    fallback_kind: GoalStrategyKind,
) -> Option<(String, GoalStrategyKind)> {
    let normalized = raw.trim().replace('\n', " ").replace('|', "｜");
    let (lhs_raw, rhs_opt) = match normalized.split_once('｜') {
        Some((l, r)) => (l.trim(), Some(r.trim())),
        None => (normalized.as_str(), None),
    };

    let lesson_body = lhs_raw
        .trim_start_matches("定見：")
        .trim_start_matches("定見:")
        .trim()
        .trim_matches('"')
        .trim_matches('「')
        .trim_matches('」')
        .trim();

    let clean_lesson = validate_clean_in_world_reply(lesson_body)?;
    if clean_lesson.chars().count() > 60
        || clean_lesson.contains('<')
        || clean_lesson.contains('>')
        || clean_lesson.contains('＜')
        || clean_lesson.contains('＞')
        || clean_lesson.contains("字內")
        || clean_lesson.contains("第一人稱")
        || clean_lesson.contains("六選一")
    {
        return None;
    }

    let kind = rhs_opt
        .and_then(|r| {
            let tag_part = r
                .trim_start_matches("方針：")
                .trim_start_matches("方針:")
                .trim();
            GoalStrategyKind::from_zh_text(tag_part)
        })
        .or_else(|| GoalStrategyKind::from_zh_text(&clean_lesson))
        .unwrap_or(fallback_kind);

    Some((clean_lesson, kind))
}

/// 零偏置挑出最高張力之 1 名角色送交 `11436` 慢思考；其餘排隊者保留分數延期等待（零 `rust_slow` 假結案！）
pub fn pop_top_slow_think_candidate(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    game_day: i64,
) -> Option<SlowThinkCandidate> {
    let best_id = delta
        .actors
        .iter()
        .filter(|(id, a)| {
            a.alive
                && !id.starts_with("pl:")
                && a.setup_done
                && a.controller_id.is_none()
                && a.mind.pending_slow_think_score > 0
                && (game_day - a.mind.stm.last_slow_think_day >= 2
                    || a.mind.pending_slow_think_score >= 360)
        })
        .max_by_key(|(_, a)| effective_slow_priority(a))
        .map(|(id, _)| id.clone())?;

    let owes_debt = delta
        .contracts
        .values()
        .any(|c| c.debtor_id == best_id && (c.status == "open" || c.status == "defaulted"));
    let concern_line = {
        let a_ro = delta.actors.get(&best_id)?;
        a_ro.mind
            .top_concern()
            .map(|c| render_concern_inner_thought_zh(delta, c, true))
            .unwrap_or_else(|| "連日生計緊俏亟待轉機".to_string())
    };
    let masked_recent_mem = {
        let raw_mem = delta
            .actors
            .get(&best_id)
            .and_then(|a_ro| a_ro.mind.ltm.episodic.last())
            .map_or("連日奔波操持營生", |e| e.subjective_zh.as_str());
        mask_player_names_in_text(delta, raw_mem)
    };

    let a = delta.actors.get_mut(&best_id)?;
    let saved_score = a.mind.pending_slow_think_score;
    let last_tp_anchor = a
        .mind
        .ltm
        .turning_points
        .last()
        .map(|t| t.anchor_event_id.clone());
    a.mind.pending_slow_think_score = 0;
    a.mind.slow_think_wait_bells = 0;
    a.mind.stm.last_slow_think_day = game_day;
    a.mark_mutated();

    let innate = derive_innate_persona(a.soul_seed.max(1));
    let top_val = innate.top_value_axis(&a.drift).in_world_motive_zh();
    let taboo_val = innate.taboo_axis.in_world_motive_zh();
    let home_label = doc
        .nodes
        .get(&a.home_node)
        .map_or(a.home_node.as_str(), |n| n.name.as_str());
    let (fallback_strategy, focus_target_id) = infer_default_strategy_for_actor(a, owes_debt);

    let prompt = format!(
        "你是銀鷗島{}的{}{}（手頭{}枚銅幣，平生最重「{}」，最忌諱「{}」受損）。\
         當下心事：{}。近期經歷：{}。\
         請嚴格以單行輸出台灣正體中文（絕對禁止任何英文字母或現代系統詞彙），\
         格式固定為『定見：一句簡短內心打算｜方針：清償舊欠、親故互助、採集貼補、避開嫌隙、精研手藝、打聽音訊、墾荒種糧、修橋繕屋、改換營生』：",
        home_label, a.race, a.role, a.copper, top_val, taboo_val, concern_line, masked_recent_mem
    );

    Some(SlowThinkCandidate {
        actor_id: best_id,
        last_tp_anchor,
        saved_score,
        game_day,
        focus_target_id,
        prompt,
        fallback_strategy,
    })
}

pub fn execute_slow_think_off_lock(
    cand: &SlowThinkCandidate,
) -> Option<(String, GoalStrategyKind)> {
    let raw = call_openai_compatible_completions(
        SLOW_THINK_URL,
        "",
        SLOW_THINK_MODEL,
        "只輸出單行台灣正體中文『定見：...｜方針：...』，嚴禁出現英文字母或設計術語。",
        &cand.prompt,
        4,
        64,
    )
    .ok()?;
    parse_slow_think_chinese_output(&raw, cand.fallback_strategy)
}
