//! 心境值與 Thought Log（對齊既有 `db/disposition`）。
//!
//! 設計：NPC 情緒以帶來源、強度、衰減期的 Thought 列表表示。
//! 當前心境值 = 尚未衰減完的 thought 線性衰減後加總，clamp [-100, 100]。
//! `Character.disposition: i32` 保留作 cache，每次 `add_thought` 後重算寫入。

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{entity::Thought, store};

use super::ErrNoStore;

// ─── ThoughtSource 列舉 ───────────────────────────────────────────────────

/// NPC 內心活動的來源類型，帶預設 delta、衰減期（分鐘）與散文描述。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// 用過茶飯飲食
    Ate,
    /// 沐浴溫水泡湯歇息
    Rested,
}

pub const TALKED_FLAVORS: &[&str] = &[
    "同熟識的鄰里寒暄幾句家常，神色舒緩",
    "與街坊聊起近日市集物價與水路貨況，心裡有了底",
    "在街口與相熟的同行交換了幾句活計近況",
    "聽往來行客提起外鎮風聲，暗自記在心上",
    "與鄰座茶客閒話幾句半島晴雨，緊繃的心緒鬆快不少",
    "和熟人應對幾句日常瑣務，轉身繼續手頭差事",
    "順口向路過街坊打聽了幾句市面行情，心緒平穩",
    "與同坊匠人聊了幾句手頭工序與料況，頗有斬獲",
];

pub fn deterministic_talked_flavor(entity_id: &str, now: i64, thoughts_len: usize) -> &'static str {
    let mut hash: u64 = thoughts_len as u64;
    for b in entity_id.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(b as u64);
    }
    hash ^= (now as u64) / 120;
    TALKED_FLAVORS[(hash as usize) % TALKED_FLAVORS.len()]
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
            ThoughtSource::Talked => (5, 180, TALKED_FLAVORS[0]),
            ThoughtSource::Broke => (-20, 1440, "鎂袋空了，心裡發慌"),
            ThoughtSource::Daily => (-2, 720, "今日花用又見底，發愁"),
            ThoughtSource::Subdued => (-15, 2880, "遭人出手攻擊落敗，心裡有氣"),
            ThoughtSource::WorkDone => (0, 360, "今日活幹完了"),
            ThoughtSource::Ate => (8, 360, "用過熱騰騰的茶飯，肚裡暖和舒坦"),
            ThoughtSource::Rested => (10, 480, "泡過熱湯歇息，一身疲乏盡數散去"),
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
            ThoughtSource::Ate => "Ate",
            ThoughtSource::Rested => "Rested",
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
/// 返回 f64 以保持衰減精度，呼叫端彙總後再 round。
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
fn is_misclassified_thought_flavor(f: &str) -> bool {
    if f.contains("公井") && f.contains("用過熱騰騰的茶飯") {
        return true;
    }
    (f.contains("連軸忙完") || f.contains("行囊裡添了新採獲的山海物產"))
        && [
            "乾糧", "果腹", "冷開水", "解渴", "粗茶", "水筒", "麥餅", "自炊", "熱粥", "滷肉",
            "點心", "大菜", "米酒", "小菜", "退火", "清湯粗麵", "陽春粗麵", "藥膳", "吃熱食",
            "上茶寮", "共飲", "品茶", "閉目", "歇宿", "睡一覺", "熟睡", "調息", "擦汗", "泡湯",
            "沐浴", "止血", "膏藥", "包紮", "煎湯", "熱敷", "施針", "靜養", "傷藥",
        ]
        .iter()
        .any(|kw| f.contains(kw))
}

pub fn normalize_thought_flavor(f: &str) -> String {
    let mut s = f
        .replace("大圳頭・西南原", "大圳頭・圳頭")
        .replace("西門外坊・西南原", "西門外坊・西門")
        .replace("西南原農具與百工坊", "大圳頭農具與百工坊")
        .replace("西南原", "大圳頭")
        .replace("防爆防風", "防風")
        .replace("陽春粗麵", "清湯粗麵")
        .replace("陽春麵", "清湯粗麵")
        .replace("冷開水", "山泉涼茶")
        .replace("白開水", "熱茶水")
        .replace("黑沼關隘", "下營")
        .replace("鹽田澳", "烏石鼻")
        .replace("青籠關", "下營")
        .replace("黑沼林澤", "水尾寮")
        .replace("黑沼林", "水尾寮")
        .replace("白塔山麓", "半嶺鎮")
        .replace("北聖山", "北嶺腳")
        .replace("西北礦帶", "紅土崁");
    if ["巨藻淺灣", "驚濤岩岸", "黑石灣", "落日岬", "淺澤泥口", "白茫洲", "貼山深潭"]
        .iter()
        .any(|z| s.contains(z))
    {
        s = s
            .replace("的林木因連日採擷正逢封山休養", "的漁場因連日採擷正逢休漁養息")
            .replace("林木亟待休養，正親手培土護苗、修整山場", "漁場亟待休養，正親手修整礁岸、養護藻床")
            .replace("林木封山", "休漁養息");
    }
    s
}

fn cleanup_thoughts(thoughts: &mut Vec<Thought>, now: i64) {
    for t in thoughts.iter_mut() {
        if let Some(ref mut f) = t.flavor {
            *f = normalize_thought_flavor(f);
        }
    }
    thoughts.retain(|t| {
        !is_expired(t, now) && !t.flavor.as_deref().is_some_and(is_misclassified_thought_flavor)
    });
    // 去除具有相同非空 flavor 的重複念頭，保留 created_at 最大者
    thoughts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let mut seen_flavors = HashSet::new();
    thoughts.retain(|t| match t.flavor.as_deref() {
        Some(f) if !f.is_empty() => seen_flavors.insert(f.to_string()),
        _ => true,
    });
    thoughts.sort_unstable_by_key(|t| t.created_at);
    if thoughts.len() > 100 {
        // 依 created_at 升序，砍最舊的
        thoughts.drain(..thoughts.len() - 100);
    }
}

/// 公開讓實體清理過期或陳舊 thoughts 並重算 disposition。
pub fn clean_and_recalc_entity_disposition(entity: &mut store::Entity, now: i64) {
    cleanup_thoughts(&mut entity.thoughts, now);
    entity.disposition = recalc_disposition(&entity.thoughts, now);
}

// ─── 公開 API ─────────────────────────────────────────────────────────────

/// 加入一條 thought（by ThoughtSource，用預設 delta/decay/flavor）。
pub fn add_thought(entity_id: &str, source: ThoughtSource) -> anyhow::Result<()> {
    let (delta, decay_minutes, default_flavor) = source.defaults();
    let now = now_unix();
    let flavor = if source == ThoughtSource::Talked {
        deterministic_talked_flavor(entity_id, now, 0).to_string()
    } else {
        default_flavor.to_string()
    };
    add_thought_with_flavor(
        entity_id,
        source.key(),
        delta,
        decay_minutes,
        flavor,
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
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let (pool, snapshot) = {
        let mut s = arc.write();
        let pool = s
            .db_pool
            .clone()
            .ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
        let entity = s
            .entities
            .get_mut(entity_id)
            .ok_or_else(|| anyhow::anyhow!("實體不存在：{entity_id}"))?;
        push_thought(entity, source, delta, decay_minutes, flavor, now);
        (pool, entity.clone())
    };
    crate::pg::entity::upsert(&pool, &snapshot)
}

/// 同一時段影響所有居民時，一次更新記憶體並用單一批次交易落庫。
pub fn add_thought_to_all_npcs_with_flavor(
    source: &str,
    delta: i32,
    decay_minutes: i64,
    flavor: String,
) -> anyhow::Result<usize> {
    let now = now_unix();
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let (pool, snapshots) = {
        let mut s = arc.write();
        let pool = s
            .db_pool
            .clone()
            .ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
        let snapshots = s
            .entities
            .values_mut()
            .filter(|entity| entity.kind == "npc" && entity.vit > 0)
            .map(|entity| {
                push_thought(entity, source, delta, decay_minutes, flavor.clone(), now);
                entity.clone()
            })
            .collect::<Vec<_>>();
        (pool, snapshots)
    };
    crate::pg::entity::upsert_batch(&pool, &snapshots)?;
    Ok(snapshots.len())
}

pub(crate) fn add_thought_to_entity(entity: &mut store::Entity, source: ThoughtSource, now: i64) {
    let (delta, decay_minutes, default_flavor) = source.defaults();
    let flavor = if source == ThoughtSource::Talked {
        deterministic_talked_flavor(&entity.id, now, entity.thoughts.len()).to_string()
    } else {
        default_flavor.to_string()
    };
    push_thought(
        entity,
        source.key(),
        delta,
        decay_minutes,
        flavor,
        now,
    );
}

pub fn add_thought_to_entity_with_flavor(
    entity: &mut store::Entity,
    source: ThoughtSource,
    now: i64,
    flavor: impl Into<String>,
) {
    let (delta, decay_minutes, _) = source.defaults();
    push_thought(
        entity,
        source.key(),
        delta,
        decay_minutes,
        flavor.into(),
        now,
    );
}

pub fn add_thought_to_entity_with_delta(
    entity: &mut store::Entity,
    source: ThoughtSource,
    delta: i32,
    now: i64,
    flavor: impl Into<String>,
) {
    let (_, decay_minutes, _) = source.defaults();
    push_thought(
        entity,
        source.key(),
        delta,
        decay_minutes,
        flavor.into(),
        now,
    );
}

fn push_thought(
    entity: &mut store::Entity,
    source: &str,
    delta: i32,
    decay_minutes: i64,
    flavor: String,
    now: i64,
) {
    let flavor = normalize_thought_flavor(&flavor);
    let mut updated_existing = false;
    if !flavor.is_empty() {
        if let Some(existing) = entity
            .thoughts
            .iter_mut()
            .find(|t| t.flavor.as_deref() == Some(flavor.as_str()))
        {
            existing.source = source.to_string();
            existing.delta = delta;
            existing.created_at = now;
            existing.decay_minutes = decay_minutes;
            updated_existing = true;
        }
    }
    if !updated_existing {
        entity.thoughts.push(Thought {
            source: source.to_string(),
            delta,
            created_at: now,
            decay_minutes,
            flavor: Some(flavor),
        });
    }
    cleanup_thoughts(&mut entity.thoughts, now);
    entity.disposition = recalc_disposition(&entity.thoughts, now);
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
    active.sort_unstable_by_key(|b| std::cmp::Reverse(b.created_at));
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
