//! 全服 `UnifiedMindBody` System 2 慢思考有界優先佇列（老化補償、超額純 Rust 保底、型別化意圖與已知地圖校驗）。

use crate::ai::talk::call_openai_compatible_completions;

use super::dialogue_llm::validate_clean_in_world_reply;
use super::mind::ActiveIntention;
use super::persona::derive_innate_persona;
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

const SLOW_THINK_URL: &str = "http://127.0.0.1:11436/v1";
const SLOW_THINK_MODEL: &str = "/home/tanger/Models/Qwen3-4B-Instruct-2507-Q8_0.gguf";
const MAX_SLOW_QUEUE_BEFORE_RUST_SETTLE: usize = 16;

pub struct SlowThinkCandidate {
    pub actor_id: String,
    pub last_tp_anchor: Option<String>,
    pub saved_score: u32,
    pub game_day: i64,
    pub abs_bell: i64,
    pub validated_goal_node: String,
    pub prompt: String,
    pub fallback_lesson: String,
    pub fallback_goal: String,
}

pub fn effective_slow_priority(a: &UnifiedMindBody) -> u32 {
    if a.mind.pending_slow_think_score == 0 {
        return 0;
    }
    a.mind
        .pending_slow_think_score
        .saturating_add(u32::from(a.mind.slow_think_wait_bells) * 12)
}

/// 若同時排隊人數超過上限，對久候者以純 Rust 規則模板保底寫入人生定見與行動意圖，絕不阻塞世界！
pub fn settle_overflow_slow_think_with_rust(
    delta: &mut IslandWorldDelta,
    game_day: i64,
    abs_bell: i64,
) {
    let mut waiting: Vec<(u32, String)> = delta
        .actors
        .iter()
        .filter(|(_, a)| a.alive && a.setup_done && a.mind.pending_slow_think_score > 0)
        .map(|(id, a)| (effective_slow_priority(a), id.clone()))
        .collect();
    if waiting.len() <= MAX_SLOW_QUEUE_BEFORE_RUST_SETTLE {
        return;
    }
    waiting.sort_by_key(|(prio, _)| *prio);
    let excess = waiting.len() - MAX_SLOW_QUEUE_BEFORE_RUST_SETTLE;
    for (_, aid) in waiting.into_iter().take(excess) {
        if let Some(a) = delta.actors.get_mut(&aid) {
            let innate = derive_innate_persona(a.soul_seed);
            let top_val = innate.top_value_axis(&a.drift).in_world_motive_zh();
            let lesson = format!(
                "生計連日受阻（手頭剩 {} 枚銅幣），先穩住「{}」再圖轉機。",
                a.copper, top_val
            );
            let goal_tag = if a.copper < 4 {
                "frugal_clear_debt"
            } else {
                "work_diligently"
            };
            let anchor = format!("rust_slow:{}:{}:{}", aid, game_day, a.rev);
            a.mind
                .push_turning_point(&anchor, game_day, &lesson, goal_tag);
            a.mind.pending_slow_think_score = 0;
            a.mind.slow_think_wait_bells = 0;
            push_validated_goal_intention(a, goal_tag, abs_bell);
            a.mark_mutated();
        }
    }
}

/// 將型別化目標轉換為經驗證物理可達（必在本人 `known_nodes` 內）之 `ActiveIntention`
pub fn push_validated_goal_intention(a: &mut UnifiedMindBody, goal_tag: &str, abs_bell: i64) {
    let (kind, candidate_node, reason_zh) = match goal_tag {
        "frugal_clear_debt" => (
            "walk_work",
            a.work_node.clone(),
            "節衣縮食勤趕工活攢夠銅幣清帳",
        ),
        "seek_kin_alliance" => (
            "go_home_sleep",
            a.home_node.clone(),
            "回屋與同戶家眷商議互助",
        ),
        _ => ("walk_work", a.work_node.clone(), "守緊本行手藝穩住生計"),
    };
    if !a.mind.ltm.known_nodes.contains(&candidate_node) {
        return;
    }
    if a.mind.stm.intentions.len() >= 4 {
        a.mind.stm.intentions.pop_front();
    }
    a.mind.stm.intentions.push_back(ActiveIntention {
        kind: kind.to_string(),
        target_node: candidate_node,
        reason_zh: reason_zh.to_string(),
        created_bell: abs_bell,
    });
}

/// 零偏置挑出含等待老化補償後優先分最高之 1 名 `UnifiedMindBody`
pub fn pop_top_slow_think_candidate(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    game_day: i64,
    abs_bell: i64,
) -> Option<SlowThinkCandidate> {
    settle_overflow_slow_think_with_rust(delta, game_day, abs_bell);

    let best_id = delta
        .actors
        .iter()
        .filter(|(_, a)| a.alive && a.setup_done && a.mind.pending_slow_think_score > 0)
        .max_by_key(|(_, a)| effective_slow_priority(a))
        .map(|(id, _)| id.clone())?;

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
    a.mark_mutated();

    let innate = derive_innate_persona(a.soul_seed);
    let top_val = innate.top_value_axis(&a.drift).in_world_motive_zh();
    let recent_mem = a
        .mind
        .ltm
        .episodic
        .last()
        .map_or("連日生計受阻", |e| e.subjective_zh.as_str());
    let home_label = doc
        .nodes
        .get(&a.home_node)
        .map_or(a.home_node.as_str(), |n| n.name.as_str());
    let fallback_goal = if a.copper < 4 {
        "frugal_clear_debt".to_string()
    } else {
        "work_diligently".to_string()
    };

    let prompt = format!(
        "你是銀鷗島{}的{}{}（手頭{}枚銅幣，心裡最看重「{}」）。近期經歷：{}。請以第一人稱寫一句不超過32字的台灣正體中文內心定見（絕不出現現代或設定詞彙），只輸出那一句話：",
        home_label, a.race, a.role, a.copper, top_val, recent_mem
    );
    let fallback_lesson = format!(
        "連日生計緊俏（手頭剩 {} 枚銅幣），往後行事得更守緊「{}」的本分。",
        a.copper, top_val
    );

    Some(SlowThinkCandidate {
        actor_id: best_id,
        last_tp_anchor,
        saved_score,
        game_day,
        abs_bell,
        validated_goal_node: a.work_node.clone(),
        prompt,
        fallback_lesson,
        fallback_goal,
    })
}

pub fn execute_slow_think_off_lock(cand: &SlowThinkCandidate) -> (String, String) {
    if let Ok(raw) = call_openai_compatible_completions(
        SLOW_THINK_URL,
        "",
        SLOW_THINK_MODEL,
        "只輸出台灣正體中文一句內心定見，絕不出現現代詞彙或設計詞彙。",
        &cand.prompt,
        4,
        48,
    ) {
        let stripped = raw
            .trim()
            .trim_matches('"')
            .trim_matches('「')
            .trim_matches('」')
            .replace('\n', "");
        if let Some(clean) = validate_clean_in_world_reply(&stripped)
            && clean.chars().count() <= 60
        {
            let inferred_goal = if clean.contains("家") || clean.contains("親") {
                "seek_kin_alliance".to_string()
            } else if clean.contains("債") || clean.contains("省") || clean.contains("攢") {
                "frugal_clear_debt".to_string()
            } else {
                cand.fallback_goal.clone()
            };
            return (clean, inferred_goal);
        }
    }
    (cand.fallback_lesson.clone(), cand.fallback_goal.clone())
}
