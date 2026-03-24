//! 玩家與 NPC 議價暫存（記憶體、TTL），對齊 Go `db/trade_pending.go`。

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const TRADE_PENDING_TTL_SEC: u64 = 300;

/// 一筆待成交報價。
#[derive(Debug, Clone)]
pub struct TradePending {
    pub npc_id: String,
    pub item_id: String,
    pub item_qty: i32,
    pub ask_mg: i32,
    pub floor_mg: i32,
    pub expires_unix: i64,
}

static TRADE_MAP: LazyLock<Mutex<HashMap<String, TradePending>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn session_key(player_id: &str, npc_id: &str) -> String {
    format!("{player_id}|{npc_id}")
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 建立或覆寫待報價並重設逾時。
pub fn trade_offer_set(player_id: &str, mut p: TradePending) {
    p.expires_unix = now_unix() + TRADE_PENDING_TTL_SEC as i64;
    let mut g = TRADE_MAP.lock().expect("trade pending poisoned");
    g.insert(session_key(player_id, &p.npc_id), p);
}

/// 取得未逾時報價；逾時則刪除並回 `None`。
#[must_use]
pub fn trade_offer_get(player_id: &str, npc_id: &str) -> Option<TradePending> {
    let mut g = TRADE_MAP.lock().expect("trade pending poisoned");
    let k = session_key(player_id, npc_id);
    let p = g.get(&k).cloned()?;
    if now_unix() > p.expires_unix {
        g.remove(&k);
        return None;
    }
    Some(p)
}

/// 結束議價（成交、拒絕、取消）。
pub fn trade_offer_clear(player_id: &str, npc_id: &str) {
    let mut g = TRADE_MAP.lock().expect("trade pending poisoned");
    g.remove(&session_key(player_id, npc_id));
}
