//! 玩家 Session 與 `SessionStore`（對齊既有 `server/session`）。

use crate::db;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

/// 單一玩家連線會話；`outbound` 為 WebSocket 寫入端。
#[derive(Clone)]
pub struct Session {
    /// 本次連線 id（重連時與舊 session 區分）。
    pub connection_id: Uuid,
    pub player_id: String,
    pub outbound: Option<Sender<Vec<u8>>>,
    pub last_talk_at: Option<Instant>,
    pub last_talk_room: String,
    pub last_talk_snippet: String,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("connection_id", &self.connection_id)
            .field("player_id", &self.player_id)
            .field("has_outbound", &self.outbound.is_some())
            .finish_non_exhaustive()
    }
}

impl Session {
    pub fn new_disconnected(player_id: impl Into<String>) -> Self {
        Self {
            connection_id: Uuid::nil(),
            player_id: player_id.into(),
            outbound: None,
            last_talk_at: None,
            last_talk_room: String::new(),
            last_talk_snippet: String::new(),
        }
    }

    pub fn with_outbound(player_id: impl Into<String>, connection_id: Uuid, tx: Sender<Vec<u8>>) -> Self {
        Self {
            connection_id,
            player_id: player_id.into(),
            outbound: Some(tx),
            last_talk_at: None,
            last_talk_room: String::new(),
            last_talk_snippet: String::new(),
        }
    }

    /// 傳送原始 JSON bytes；失敗時回傳 false。
    pub fn try_send_bytes(&self, msg: Vec<u8>) -> bool {
        let Some(tx) = &self.outbound else {
            return false;
        };
        tx.blocking_send(msg).is_ok()
    }
}

/// 以 `PlayerID` 為 key 的線上 session（對齊既有 `SessionStore`）。
pub struct SessionStore {
    inner: RwLock<HashMap<String, Session>>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn set(&self, player_id: &str, session: Session) {
        let mut g = self.inner.write().expect("session store poisoned");
        g.insert(player_id.to_string(), session);
    }

    pub fn get(&self, player_id: &str) -> Option<Session> {
        let g = self.inner.read().expect("session store poisoned");
        g.get(player_id).cloned()
    }

    pub fn remove(&self, player_id: &str) {
        let mut g = self.inner.write().expect("session store poisoned");
        g.remove(player_id);
    }

    /// 若目前綁定的連線 id 相符才移除（對齊既有 斷線比對 `Client`）。
    pub fn remove_if_connection(&self, player_id: &str, connection_id: Uuid) {
        let mut g = self.inner.write().expect("session store poisoned");
        if let Some(s) = g.get(player_id)
            && s.connection_id == connection_id
        {
            g.remove(player_id);
        }
    }

    pub fn all_player_ids(&self) -> Vec<String> {
        let g = self.inner.read().expect("session store poisoned");
        g.keys().cloned().collect()
    }

    pub fn all_sessions(&self) -> Vec<Session> {
        let g = self.inner.read().expect("session store poisoned");
        g.values().cloned().collect()
    }

    pub fn update_last_talk_at(&self, player_id: &str) {
        let mut g = self.inner.write().expect("session store poisoned");
        if let Some(s) = g.get_mut(player_id) {
            s.last_talk_at = Some(Instant::now());
        }
    }

    pub fn record_player_talk_for_room_echo(
        &self,
        player_id: &str,
        room_id: &str,
        player_input: &str,
    ) {
        let mut g = self.inner.write().expect("session store poisoned");
        let Some(s) = g.get_mut(player_id) else {
            return;
        };
        s.last_talk_at = Some(Instant::now());
        s.last_talk_room = room_id.to_string();
        s.last_talk_snippet = sanitize_talk_snippet(player_input);
    }

    pub fn player_talk_echo_for_npc_social(&self, room_id: &str) -> String {
        if room_id.is_empty() {
            return String::new();
        }
        let g = self.inner.read().expect("session store poisoned");
        let mut best_snippet = String::new();
        let mut best_at: Option<Instant> = None;
        let want = db::canonical_location_key(room_id);
        for s in g.values() {
            if s.last_talk_snippet.is_empty() || db::canonical_location_key(&s.last_talk_room) != want {
                continue;
            }
            let Ok(rid) = db::get_entity_room(&s.player_id) else {
                continue;
            };
            if db::canonical_location_key(&rid) != want {
                continue;
            }
            let Some(at) = s.last_talk_at else {
                continue;
            };
            let age = at.elapsed();
            if age < Duration::from_secs(60) || age > Duration::from_secs(240) {
                continue;
            }
            let replace = match best_at {
                None => true,
                Some(prev) => at > prev,
            };
            if replace {
                best_snippet.clone_from(&s.last_talk_snippet);
                best_at = Some(at);
            }
        }
        best_snippet
    }

    pub fn room_has_player(&self, room_id: &str) -> bool {
        let want = db::canonical_location_key(room_id);
        let g = self.inner.read().expect("session store poisoned");
        for s in g.values() {
            let rid = db::get_entity_room(&s.player_id).unwrap_or_default();
            if db::canonical_location_key(&rid) == want {
                return true;
            }
        }
        false
    }

    pub fn room_has_player_with_recent_talk(&self, room_id: &str, within: Duration) -> bool {
        let want = db::canonical_location_key(room_id);
        let g = self.inner.read().expect("session store poisoned");
        for s in g.values() {
            let rid = db::get_entity_room(&s.player_id).unwrap_or_default();
            if db::canonical_location_key(&rid) != want {
                continue;
            }
            if let Some(at) = s.last_talk_at
                && at.elapsed() < within
            {
                return true;
            }
        }
        false
    }

    /// 目前有玩家在線且已落在某房間的 room id 集合（對齊既有 `GetPlayerRoomMap`）。
    /// 條目為 [`db::canonical_location_key`]，使世界房間 id 與 `hex:…` 在模擬迴圈比對時一致。
    #[must_use]
    pub fn player_room_ids(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        let g = self.inner.read().expect("session store poisoned");
        for s in g.values() {
            let rid = db::get_entity_room(&s.player_id).unwrap_or_default();
            if rid.is_empty() {
                continue;
            }
            set.insert(db::canonical_location_key(&rid));
        }
        set
    }
}

pub fn sanitize_talk_snippet(s: &str) -> String {
    let mut s = s.trim().to_string();
    if s.is_empty() || s == "（搭話）" {
        return String::new();
    }
    s = s
        .chars()
        .map(|c| if matches!(c, '\r' | '\n') { ' ' } else { c })
        .collect();
    while s.contains("  ") {
        s = s.replace("  ", " ");
    }
    if s.len() > 256 {
        let mut end = 256.min(s.len());
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > 48 {
        let head: String = chars[..48].iter().collect();
        return head + "…";
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_empty_and_dahua() {
        assert_eq!(sanitize_talk_snippet("   "), "");
        assert_eq!(sanitize_talk_snippet("（搭話）"), "");
    }

    #[test]
    fn sanitize_newlines_and_spaces() {
        assert_eq!(sanitize_talk_snippet("a\nb\r\nc"), "a b c".to_string());
        assert_eq!(sanitize_talk_snippet("x    y"), "x y");
    }

    #[test]
    fn store_set_get_remove() {
        let st = SessionStore::new();
        st.set("p1", Session::new_disconnected("p1"));
        assert!(st.get("p1").is_some());
        st.remove("p1");
        assert!(st.get("p1").is_none());
    }
}
