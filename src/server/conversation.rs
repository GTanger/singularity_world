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
        .map(|t| format!("玩家說：{}；NPC回：{}", t.player_input, t.npc_reply))
        .collect()
}

fn summary_from_turns(turns: &[ConversationTurn]) -> String {
    let Some(last) = turns.last() else {
        return String::new();
    };
    let mut s = last.player_input.trim().to_string();
    if s.chars().count() > 30 {
        s = s.chars().take(30).collect::<String>() + "…";
    }
    if s.is_empty() || s == "（搭話）" {
        return "最近曾與玩家對話。".to_string();
    }
    format!("最近聊過：「{s}」。")
}

/// 若上一場已逾時：先 consolidation 寫 archival(tag=event)+summary；再 append 本輪。
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
    buf.push(ConversationTurn {
        player_input: player_input.to_string(),
        npc_reply: npc_reply.to_string(),
        at: now,
    });
    if buf.len() > MAX_TURNS_PER_BUFFER {
        let keep_from = buf.len() - MAX_TURNS_PER_BUFFER;
        buf = buf.split_off(keep_from);
    }
    g.insert(key, buf);
}
