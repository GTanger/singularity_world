// 長期記憶寫入與 NPC↔NPC 去重（對齊 Go `db/archival.go` 子集）。

use crate::store::{self, ArchivalEntry};

use super::{text, ErrNoStore};

const ARCHIVAL_THROTTLE_WINDOW_SEC: i64 = 600;
const ARCHIVAL_THROTTLE_MAX: usize = 3;

fn count_archival_since(entity_id: &str, since_unix: i64) -> usize {
    let Some(arc) = store::get_store() else {
        return 0;
    };
    let s = arc.read().unwrap();
    s.get_archival_by_entity(entity_id)
        .into_iter()
        .filter(|e| e.created_at >= since_unix)
        .count()
}

fn recent_npc_npc_archival_contents(entity_id: &str, n: usize) -> Vec<String> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read().unwrap();
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

/// 寫入 tag=npc_npc 的長期記憶；去重／節流對齊 Go `InsertNpcNpcDialogueArchival`。
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
    let mut s = arc.write().unwrap();
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
