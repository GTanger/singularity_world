//! 心境值與 Thought Log（對齊既有 `db/disposition`）。
//!
//! 設計：NPC 情緒以帶來源、強度、衰減期的 Thought 列表表示。
//! 當前心境值 = 尚未衰減完的 thought 線性衰減後加總，clamp [-100, 100]。
//! `Character.disposition: i32` 保留作 cache，每次 `add_thought` 後重算寫入。

use std::time::{SystemTime, UNIX_EPOCH};

use crate::{entity::Thought, store};

use super::ErrNoStore;

// ─── ThoughtSource 列舉 ───────────────────────────────────────────────────

/// NPC 內心活動的來源類型，帶預設 delta、衰減期（分鐘）與散文描述。
#[derive(Debug, Clone, Copy)]
pub enum ThoughtSource {
    /// 乞討成功
    BegSuccess,
    /// 採集成功
    Gather,
    /// 獲指派工作
    Hired,
    /// 街頭兜售成功
    Trade,
    /// 與人交談
    Talked,
    /// 鎂歸零
    Broke,
    /// 每日支出
    Daily,
    /// 被制伏
    Subdued,
    /// 工作完成（delta 動態指定，走 add_thought_with_flavor）
    WorkDone,
}

impl ThoughtSource {
    /// 回傳 (delta, decay_minutes, default_flavor)。
    /// WorkDone 的 delta 預設 0，應走 add_thought_with_flavor 傳入實際 delta。
    pub fn defaults(self) -> (i32, i64, &'static str) {
        match self {
            ThoughtSource::BegSuccess => (3, 240, "乞了一頓飯，肚裡踏實了些"),
            ThoughtSource::Gather => (5, 360, "採到一些野貨，心裡踏實"),
            ThoughtSource::Hired => (20, 1440, "得了差事，心裡寬慰"),
            ThoughtSource::Trade => (6, 240, "今日生意做成幾筆"),
            ThoughtSource::Talked => (5, 180, "跟人說了會兒話"),
            ThoughtSource::Broke => (-20, 1440, "鎂袋空了，心裡發慌"),
            ThoughtSource::Daily => (-2, 720, "今日花用又見底，發愁"),
            ThoughtSource::Subdued => (-15, 2880, "被人留下制伏，心裡有氣"),
            ThoughtSource::WorkDone => (0, 360, "今日活幹完了"),
        }
    }

    /// source key 字串（用於 Thought.source 欄位）。
    pub fn key(self) -> &'static str {
        match self {
            ThoughtSource::BegSuccess => "BegSuccess",
            ThoughtSource::Gather => "Gather",
            ThoughtSource::Hired => "Hired",
            ThoughtSource::Trade => "Trade",
            ThoughtSource::Talked => "Talked",
            ThoughtSource::Broke => "Broke",
            ThoughtSource::Daily => "Daily",
            ThoughtSource::Subdued => "Subdued",
            ThoughtSource::WorkDone => "WorkDone",
        }
    }
}

// ─── 時間工具 ─────────────────────────────────────────────────────────────

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ─── 線性衰減計算 ─────────────────────────────────────────────────────────

/// 計算單條 thought 在當前時刻的有效 delta（線性衰減到 0）。
/// 返回 f64 以保持衰減精度，調用方彙總後再 round。
fn effective_delta(t: &Thought, now: i64) -> f64 {
    let decay_secs = t.decay_minutes * 60;
    if decay_secs <= 0 {
        return 0.0;
    }
    let elapsed = (now - t.created_at).max(0);
    if elapsed >= decay_secs {
        return 0.0;
    }
    let ratio = (decay_secs - elapsed) as f64 / decay_secs as f64;
    f64::from(t.delta) * ratio
}

/// 是否完全衰減完（即不計入）。
fn is_expired(t: &Thought, now: i64) -> bool {
    let decay_secs = t.decay_minutes * 60;
    (now - t.created_at) >= decay_secs
}

// ─── 重算 cache ───────────────────────────────────────────────────────────

/// 由 thoughts 列表重算 disposition cache（clamp [-100, 100]）。
fn recalc_disposition(thoughts: &[Thought], now: i64) -> i32 {
    let sum: f64 = thoughts.iter().map(|t| effective_delta(t, now)).sum();
    (sum.round() as i32).clamp(-100, 100)
}

// ─── 清理過期與超限 ───────────────────────────────────────────────────────

/// 清掉完全衰減的條目；若仍超 100 條，砍最舊。
fn cleanup_thoughts(thoughts: &mut Vec<Thought>, now: i64) {
    thoughts.retain(|t| !is_expired(t, now));
    if thoughts.len() > 100 {
        // 依 created_at 升序，砍最舊的
        thoughts.sort_unstable_by_key(|t| t.created_at);
        thoughts.drain(..thoughts.len() - 100);
    }
}

// ─── 公開 API ─────────────────────────────────────────────────────────────

/// 加入一條 thought（by ThoughtSource，用預設 delta/decay/flavor）。
pub fn add_thought(entity_id: &str, source: ThoughtSource) -> anyhow::Result<()> {
    let (delta, decay_minutes, flavor) = source.defaults();
    add_thought_with_flavor(
        entity_id,
        source.key(),
        delta,
        decay_minutes,
        flavor.to_string(),
    )
}

/// 加入一條 thought（自訂 source、delta、decay、flavor）。
/// WorkDone 等動態 delta 場景走此路徑。
pub fn add_thought_with_flavor(
    entity_id: &str,
    source: &str,
    delta: i32,
    decay_minutes: i64,
    flavor: String,
) -> anyhow::Result<()> {
    let now = now_unix();
    let thought = Thought {
        source: source.to_string(),
        delta,
        created_at: now,
        decay_minutes,
        flavor: Some(flavor),
    };
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.update_entity(entity_id, |e| {
        e.thoughts.push(thought);
        cleanup_thoughts(&mut e.thoughts, now);
        e.disposition = recalc_disposition(&e.thoughts, now);
    })
}

/// 取最近 N 條尚未完全衰減的 thoughts，給 LLM prompt 用。
/// 依 created_at 降序（最新的在前）。
#[must_use]
pub fn get_active_thoughts(entity_id: &str, limit: usize) -> Vec<Thought> {
    let now = now_unix();
    let Some(arc) = store::get_store() else {
        return vec![];
    };
    let s = arc.read();
    let Some(e) = s.get_entity(entity_id) else {
        return vec![];
    };
    let mut active: Vec<Thought> = e
        .thoughts
        .iter()
        .filter(|t| !is_expired(t, now))
        .cloned()
        .collect();
    active.sort_unstable_by(|a, b| b.created_at.cmp(&a.created_at));
    active.truncate(limit);
    active
}

/// 取得心境投影值；直接讀 cache（無實體時 0）。
/// 簽名與舊版完全相同，讀取端零改動。
#[must_use]
pub fn get_disposition(entity_id: &str) -> i32 {
    let Some(arc) = store::get_store() else {
        return 0;
    };
    let s = arc.read();
    s.get_entity(entity_id).map(|e| e.disposition).unwrap_or(0)
}

/// 調整心境並 clamp 於 [-100, 100]。
///
/// # Deprecated
/// 保留作舊呼叫兜底。內部轉為 `add_thought_with_flavor`（"Legacy" source，衰減 6h）。
/// 逐步以 `add_thought(entity_id, ThoughtSource::X)` 取代。
pub fn adjust_disposition(entity_id: &str, delta: i32) -> anyhow::Result<()> {
    add_thought_with_flavor(
        entity_id,
        "Legacy",
        delta,
        360, // 6 小時
        String::new(),
    )
}
