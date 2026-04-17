// 長期記憶寫入與 NPC↔NPC 去重（對齊既有 `db/archival` 子集）。

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::store::{self, ArchivalEntry};

use super::{get_assignments_for_entity, text, ErrNoStore};

const ARCHIVAL_THROTTLE_WINDOW_SEC: i64 = 600;
const ARCHIVAL_THROTTLE_MAX: usize = 3;

fn count_archival_since(entity_id: &str, since_unix: i64) -> usize {
    let Some(arc) = store::get_store() else {
        return 0;
    };
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    s.get_archival_by_entity(entity_id)
        .into_iter()
        .filter(|e| e.created_at >= since_unix)
        .count()
}

fn recent_npc_npc_archival_contents(entity_id: &str, n: usize) -> Vec<String> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    let mut v: Vec<_> = s
        .get_archival_by_entity(entity_id)
        .into_iter()
        .filter(|e| e.tag == "npc_npc")
        .collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.created_at));
    v.into_iter()
        .take(n)
        .map(|e| e.content)
        .collect()
}

fn should_skip_npc_npc_archival(entity_id: &str, content: &str) -> bool {
    let content = content.trim();
    if content.is_empty() {
        return true;
    }
    for old in recent_npc_npc_archival_contents(entity_id, 3) {
        if old == content {
            return true;
        }
        if text::rune_lcs_similarity(&old, content) >= 0.8 {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone)]
struct ArchivalRank {
    content: String,
    score: i32,
    at: i64,
}

fn split_query_terms(q: &str) -> Vec<String> {
    let q = q.trim();
    if q.is_empty() {
        return Vec::new();
    }
    let terms: Vec<String> = q
        .split_whitespace()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect();
    if terms.is_empty() {
        vec![q.to_string()]
    } else {
        terms
    }
}

fn rank_archival_for_query(entries: &[ArchivalEntry], query: &str) -> Vec<ArchivalRank> {
    let q = query.trim();
    if q.is_empty() {
        return Vec::new();
    }
    let terms = split_query_terms(q);
    let mut ranked: Vec<ArchivalRank> = entries
        .iter()
        .map(|e| {
            let mut s = 0_i32;
            for t in &terms {
                if !t.is_empty() && e.content.contains(t) {
                    s += 1;
                }
            }
            ArchivalRank {
                content: e.content.clone(),
                score: s,
                at: e.created_at,
            }
        })
        .collect();
    ranked.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| b.at.cmp(&a.at)));
    ranked
}

fn normalize_player_talk_query_for_greeting(q: &str) -> String {
    let mut s = q.trim().to_lowercase();
    for ch in ["。", "！", "？", ".", "!", "?", "~", "～", "…"] {
        s = s.trim_matches(|c| ch.contains(c)).to_string();
    }
    loop {
        let before = s.clone();
        for suf in ["啊", "呀", "喔", "哦", "吧", "呢", "嘛", "啦", "唄", "哈"] {
            if s.chars().count() > suf.chars().count() && s.ends_with(suf) {
                s = s.trim_end_matches(suf).trim().to_string();
            }
        }
        if s == before {
            break;
        }
    }
    s.trim().to_string()
}

fn should_omit_archival_fallback_for_talk_query(query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() || q.contains("搭話") {
        return true;
    }
    let norm = normalize_player_talk_query_for_greeting(q);
    if norm.is_empty() {
        return true;
    }
    let exact: HashSet<&str> = HashSet::from([
        "你好", "您好", "嗨", "哈囉", "哈喽", "hello", "hi", "早", "早安", "午安", "晚安", "在嗎", "在不在",
        "喂", "欸", "嘿", "謝謝", "谢谢", "感恩", "再見", "再见", "拜拜", "掰掰", "掰", "嗯", "嗯嗯", "喔",
        "哦", "好", "好啊", "好喔", "好的", "收到", "了解", "恩", "噢",
    ]);
    exact.contains(norm.as_str())
}

/// 寫入 tag=npc_npc 的長期記憶；去重／節流對齊既有 `InsertNpcNpcDialogueArchival`。
pub fn insert_npc_npc_dialogue_archival(entity_id: &str, content: &str) -> anyhow::Result<(bool, bool)> {
    let content = content.trim();
    if content.is_empty() {
        return Ok((false, false));
    }
    if should_skip_npc_npc_archival(entity_id, content) {
        return Ok((false, true));
    }
    let now = crate::game::now_unix();
    if count_archival_since(entity_id, now - ARCHIVAL_THROTTLE_WINDOW_SEC) >= ARCHIVAL_THROTTLE_MAX {
        return Ok((false, false));
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    s.append_archival(ArchivalEntry {
        entity_id: entity_id.to_string(),
        content: content.to_string(),
        tag: "npc_npc".to_string(),
        created_at: now,
    })?;
    Ok((true, false))
}

/// 最近 n 條 NPC↔NPC 長期記憶行（對齊 `RecentNpcNpcArchivalLinesForEntity`）。
#[must_use]
pub fn recent_npc_npc_archival_lines_for_entity(entity_id: &str, n: usize) -> Vec<String> {
    recent_npc_npc_archival_contents(entity_id, n)
}

/// 依 query 關鍵詞評分檢索長期記憶；若零命中則 fallback 最新幾條。
#[must_use]
pub fn search_archival(entity_id: &str, query: &str, top_k: usize) -> Vec<String> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    let mut entries = s.get_archival_by_entity(entity_id);
    if entries.is_empty() || top_k == 0 {
        return Vec::new();
    }
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let query = query.trim();
    if query.is_empty() {
        return entries.into_iter().take(top_k).map(|e| e.content).collect();
    }
    let ranked = rank_archival_for_query(&entries, query);
    let mut out: Vec<String> = ranked
        .into_iter()
        .filter(|r| r.score > 0)
        .take(top_k)
        .map(|r| r.content)
        .collect();
    if out.is_empty() {
        out = entries.into_iter().take(top_k).map(|e| e.content).collect();
    }
    out
}

/// 玩家↔NPC Talk 專用記憶檢索；寒暄零命中時不做 fallback，避免塞入無關記憶。
#[must_use]
pub fn search_archival_for_player_talk(entity_id: &str, query: &str, top_k: usize) -> Vec<String> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    let mut entries = s.get_archival_by_entity(entity_id);
    if entries.is_empty() || top_k == 0 {
        return Vec::new();
    }
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let query = query.trim();
    if query.is_empty() {
        return entries.into_iter().take(top_k).map(|e| e.content).collect();
    }
    let ranked = rank_archival_for_query(&entries, query);
    let mut out: Vec<String> = ranked
        .into_iter()
        .filter(|r| r.score > 0)
        .take(top_k)
        .map(|r| r.content)
        .collect();
    if out.is_empty() && should_omit_archival_fallback_for_talk_query(query) {
        return Vec::new();
    }
    if out.is_empty() {
        out = entries.into_iter().take(top_k).map(|e| e.content).collect();
    }
    out
}

#[derive(Debug, Deserialize)]
struct OccupationsFile {
    #[serde(default)]
    occupations: std::collections::HashMap<String, OccupationDef>,
}

#[derive(Debug, Deserialize)]
struct OccupationDef {
    #[serde(default)]
    dialogue_file: String,
}

#[derive(Debug, Deserialize)]
struct DialoguePool {
    #[serde(default)]
    talk: DialogueLines,
    #[serde(default)]
    greet: DialogueLines,
}

#[derive(Debug, Default, Deserialize)]
struct DialogueLines {
    #[serde(default)]
    lines: Vec<String>,
}

fn load_occupations_file() -> Option<OccupationsFile> {
    let path = Path::new("data/templates/occupations.json");
    let raw = fs::read_to_string(path)
        .or_else(|_| fs::read_to_string(Path::new("..").join(path)))
        .ok()?;
    serde_json::from_str(&raw).ok()
}

fn load_dialogue_file(dialogue_file: &str) -> Option<DialoguePool> {
    if dialogue_file.trim().is_empty() {
        return None;
    }
    let path = Path::new("data/templates").join(dialogue_file);
    let raw = fs::read_to_string(&path)
        .or_else(|_| fs::read_to_string(Path::new("..").join(&path)))
        .ok()?;
    serde_json::from_str(&raw).ok()
}

/// 從該 NPC 的對話池抽 n 句作為口吻範例（不填佔位符），供 Talk prompt style examples。
#[must_use]
pub fn pick_style_examples(entity_id: &str, n: usize) -> Vec<String> {
    if n == 0 {
        return Vec::new();
    }
    let Ok(assignments) = get_assignments_for_entity(entity_id) else {
        return Vec::new();
    };
    let Some(a) = assignments.first() else {
        return Vec::new();
    };
    let Some(occs) = load_occupations_file() else {
        return Vec::new();
    };
    let Some(occ) = occs.occupations.get(&a.occupation_id) else {
        return Vec::new();
    };
    let Some(pool) = load_dialogue_file(&occ.dialogue_file) else {
        return Vec::new();
    };
    let mut lines: Vec<String> = Vec::new();
    lines.extend(pool.talk.lines);
    lines.extend(pool.greet.lines);
    if lines.is_empty() {
        return Vec::new();
    }
    let mut seed: i64 = 0;
    for r in entity_id.chars() {
        seed = seed.saturating_mul(31).saturating_add(r as i64);
    }
    let mut out = Vec::new();
    for i in 0..n {
        let idx = (seed.saturating_add((i as i64) * 7)).unsigned_abs() as usize % lines.len();
        let s = lines[idx].trim();
        if !s.is_empty() {
            out.push(s.to_string());
        }
    }
    out
}

/// 寫入一筆通用長期記憶（對齊既有 `InsertArchival`）。
pub fn insert_archival(entity_id: &str, content: &str, tag: &str) -> anyhow::Result<()> {
    let content = content.trim();
    if content.is_empty() {
        return Ok(());
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    s.append_archival(ArchivalEntry {
        entity_id: entity_id.to_string(),
        content: content.to_string(),
        tag: tag.to_string(),
        created_at: crate::game::now_unix(),
    })
}

/// 寫入 NPC 對玩家互動摘要（對齊既有 `SetNpcSummary`）。
pub fn set_npc_summary(entity_id: &str, summary: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    s.set_npc_summary(entity_id, summary)
}
