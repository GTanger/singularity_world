//! 玩家↔NPC Talk 對話緩衝與結束整併（對齊既有 `server/conversation_buffer`）。

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use crate::db;

const CONVERSATION_GAP_SEC: i64 = 120;
const MAX_TURNS_PER_BUFFER: usize = 20;
const MAX_CONSOLIDATE: usize = 3;

#[derive(Debug, Clone)]
struct ConversationTurn {
    player_input: String,
    npc_reply: String,
    at: i64,
}

static BUFFERS: LazyLock<Mutex<HashMap<String, Vec<ConversationTurn>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn buffer_key(player_id: &str, npc_id: &str) -> String {
    format!("{player_id}\u{0000}{npc_id}")
}

fn sanitize_talk_text(s: &str) -> String {
    let mut out = s.replace(['\r', '\n'], " ");
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    crate::ai::sanitize::sanitize_npc_dialogue_line(out.trim())
}

fn clean_player_input_text(s: &str) -> String {
    let mut out = s.replace(['\r', '\n'], " ");
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out.trim().to_string()
}

fn truncate_snippet(s: &str, max_chars: usize) -> String {
    let sanitized = sanitize_talk_text(s);
    if sanitized.chars().count() > max_chars {
        sanitized.chars().take(max_chars).collect::<String>() + "…"
    } else {
        sanitized
    }
}

fn truncate_player_snippet(s: &str, max_chars: usize) -> String {
    let cleaned = clean_player_input_text(s);
    if cleaned.chars().count() > max_chars {
        cleaned.chars().take(max_chars).collect::<String>() + "…"
    } else {
        cleaned
    }
}

fn consolidate_turns(turns: &[ConversationTurn]) -> Vec<String> {
    if turns.is_empty() {
        return Vec::new();
    }
    let picked: Vec<&ConversationTurn> = if turns.len() <= MAX_CONSOLIDATE {
        turns.iter().collect()
    } else {
        turns[turns.len() - MAX_CONSOLIDATE..].iter().collect()
    };
    picked
        .into_iter()
        .map(|t| {
            let clean_p = clean_player_input_text(&t.player_input);
            let clean_r = sanitize_talk_text(&t.npc_reply);
            format!(
                "【{}】對方說：「{}」；回應：「{}」",
                crate::store::format_game_calendar_stamp(t.at),
                clean_p,
                clean_r
            )
        })
        .collect()
}

fn summary_from_turns(turns: &[ConversationTurn]) -> String {
    if turns.is_empty() {
        return String::new();
    }
    let window: Vec<&ConversationTurn> = if turns.len() <= 2 {
        turns.iter().collect()
    } else {
        turns[turns.len() - 2..].iter().collect()
    };

    let p_snippets: Vec<String> = window
        .iter()
        .map(|t| truncate_player_snippet(&t.player_input, 80))
        .filter(|s| !s.is_empty() && s != "（搭話）")
        .collect();

    let r_snippets: Vec<String> = window
        .iter()
        .map(|t| truncate_snippet(&t.npc_reply, 80))
        .filter(|s| !s.is_empty())
        .collect();

    let p_snip = if p_snippets.is_empty() {
        "寒暄問候".to_string()
    } else {
        p_snippets.join("／")
    };

    let r_snip = if r_snippets.is_empty() {
        "略作回應".to_string()
    } else {
        r_snippets.join("／")
    };
    let last_at = turns.last().map(|t| t.at).unwrap_or(0);
    let stamp = if last_at > 0 {
        format!("（{}）", crate::store::format_game_calendar_stamp(last_at))
    } else {
        String::new()
    };

    format!(
        "曾與來客交談{stamp}（共 {} 輪）：來客提及「{}」，回應「{}」。",
        turns.len(),
        p_snip,
        r_snip
    )
}

/// 讀取 (player_id, npc_id) 最近 `limit` 輪對話紀錄 (player_input, npc_reply)。
pub fn get_recent_turns(player_id: &str, npc_id: &str, limit: usize) -> Vec<(String, String)> {
    if limit == 0 {
        return Vec::new();
    }
    let key = buffer_key(player_id, npc_id);
    let g = BUFFERS.lock().expect("conversation buffers poisoned");
    if let Some(buf) = g.get(&key) {
        let start = buf.len().saturating_sub(limit);
        buf[start..]
            .iter()
            .map(|t| (t.player_input.clone(), t.npc_reply.clone()))
            .collect()
    } else {
        Vec::new()
    }
}
/// 讀取 (player_id, npc_id) 最近 `limit` 輪對話紀錄 (player_input, npc_reply, at_unix)。
pub fn get_recent_turns_with_time(
    player_id: &str,
    npc_id: &str,
    limit: usize,
) -> Vec<(String, String, i64)> {
    if limit == 0 {
        return Vec::new();
    }
    let key = buffer_key(player_id, npc_id);
    let g = BUFFERS.lock().expect("conversation buffers poisoned");
    if let Some(buf) = g.get(&key) {
        let start = buf.len().saturating_sub(limit);
        buf[start..]
            .iter()
            .map(|t| (t.player_input.clone(), t.npc_reply.clone(), t.at))
            .collect()
    } else {
        Vec::new()
    }
}

/// 取得當前對話 sitting 的跨度 (first_turn_at, last_turn_at, turn_count)。
/// 若緩衝非空且距離最後一輪在 `CONVERSATION_GAP_SEC` 內，回傳 `Some`，否則回傳 `None`。
pub fn get_current_session_span(
    player_id: &str,
    npc_id: &str,
    now_unix: i64,
) -> Option<(i64, i64, usize)> {
    let key = buffer_key(player_id, npc_id);
    let g = BUFFERS.lock().expect("conversation buffers poisoned");
    if let Some(buf) = g.get(&key) {
        if let Some(last) = buf.last() {
            if now_unix - last.at <= CONVERSATION_GAP_SEC {
                let first_at = buf.first().unwrap().at;
                let last_at = last.at;
                let count = buf.len();
                return Some((first_at, last_at, count));
            }
        }
    }
    None
}

/// 若上一場已逾時：先 consolidation 寫 archival(tag=event)+summary；再 append 本輪。
/// 同時立即持久化至 archival(tag=player_talk) 與更新 npc_summary，確保記憶永續不忘。
pub fn flush_conversation_and_append(
    player_id: &str,
    npc_id: &str,
    player_input: &str,
    npc_reply: &str,
    now: i64,
) {
    let key = buffer_key(player_id, npc_id);
    let mut g = BUFFERS.lock().expect("conversation buffers poisoned");
    let mut buf = g.remove(&key).unwrap_or_default();
    if let Some(last) = buf.last()
        && now - last.at > CONVERSATION_GAP_SEC
    {
        for c in consolidate_turns(&buf) {
            let _ = db::insert_archival(npc_id, &c, "event");
        }
        let sum = summary_from_turns(&buf);
        if !sum.is_empty() {
            let _ = db::set_npc_summary(npc_id, &sum);
        }
        buf.clear();
    }
    let clean_reply_for_store = sanitize_talk_text(npc_reply);
    buf.push(ConversationTurn {
        player_input: player_input.to_string(),
        npc_reply: clean_reply_for_store.clone(),
        at: now,
    });
    if buf.len() > MAX_TURNS_PER_BUFFER {
        let keep_from = buf.len() - MAX_TURNS_PER_BUFFER;
        buf = buf.split_off(keep_from);
    }

    let trimmed_input = player_input.trim();
    let trimmed_reply = clean_reply_for_store.trim();
    if !trimmed_input.is_empty() && !trimmed_reply.is_empty() {
        let clean_p = clean_player_input_text(trimmed_input);
        let archival_line = format!(
            "與來客({})交談：對方說「{}」，我回應「{}」",
            player_id, clean_p, trimmed_reply
        );
        let _ = db::insert_archival(npc_id, &archival_line, "player_talk");
        let sum = summary_from_turns(&buf);
        if !sum.is_empty() {
            let _ = db::set_npc_summary(npc_id, &sum);
        }
    }

    g.insert(key, buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_from_turns_and_sanitization() {
        let turns = vec![
            ConversationTurn {
                player_input: "請問最近渡口還有船班嗎？".to_string(),
                npc_reply: concat!("\u{525b}", "\u{525b}", "聽說午後有一班渡船靠岸。").to_string(),
                at: 100,
            },
            ConversationTurn {
                player_input: "船票怎麼算？".to_string(),
                npc_reply: "一人兩枚鎂幣，到碼頭直接找船家就行。".to_string(),
                at: 110,
            },
        ];

        let sum = summary_from_turns(&turns);
        assert!(sum.contains("共 2 輪"));
        assert!(sum.contains("請問最近渡口還有船班嗎？"));
        assert!(sum.contains("聽說午後有一班渡船靠岸"));
        assert!(!sum.contains(concat!("\u{525b}", "\u{624d}")));
        assert!(!sum.contains(concat!("\u{525b}", "\u{525b}")));

        let consolidated = consolidate_turns(&turns);
        assert_eq!(consolidated.len(), 2);
        assert!(!consolidated[0].contains(concat!("\u{525b}", "\u{525b}")));
    }
}
