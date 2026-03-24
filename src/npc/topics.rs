// NPC 間對話主題劇本（載入與查詢），對齊 Go npc/topics.go。

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{LazyLock, RwLock};

use rand::Rng;
use serde::{Deserialize, Serialize};

/// 單一主題劇本（對齊 Go `NpcNpcTopic`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcNpcTopic {
    pub id: String,
    pub hint: String,
    #[serde(default)]
    pub lines: Vec<String>,
    #[serde(default)]
    pub requires_work: bool,
    #[serde(default)]
    pub night_only: bool,
    #[serde(default)]
    pub follow_up: bool,
}

#[derive(Debug, Deserialize)]
struct TopicsFile {
    topics: Vec<NpcNpcTopic>,
}

static NPC_TOPICS_LIST: LazyLock<RwLock<Vec<NpcNpcTopic>>> = LazyLock::new(|| RwLock::new(Vec::new()));

/// 從路徑載入主題劇本；檔案不存在則靜默跳過，其餘錯誤寫 log（對齊 Go `LoadNpcNpcTopics`）。
pub fn load_npc_npc_topics(path: impl AsRef<Path>) {
    if let Err(e) = try_load_npc_npc_topics(path.as_ref()) {
        tracing::warn!("[npc_topics] load {}: {}", path.as_ref().display(), e);
    }
}

/// 載入主題；檔案不存在則 `Ok(())` 且不變更列表。
pub fn try_load_npc_npc_topics(path: &Path) -> anyhow::Result<()> {
    let raw = match fs::read_to_string(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
        Ok(s) => s,
    };
    let f: TopicsFile = serde_json::from_str(&raw)?;
    let list = f.topics;
    let mut g = NPC_TOPICS_LIST.write().expect("npc topics lock");
    *g = list;
    Ok(())
}

/// 依 id 回傳主題（對齊 Go `GetNpcNpcTopicByID`）。
#[must_use]
pub fn get_npc_npc_topic_by_id(id: &str) -> Option<NpcNpcTopic> {
    let g = NPC_TOPICS_LIST.read().expect("npc topics lock");
    g.iter().find(|t| t.id == id).cloned()
}

fn normalize_npc_npc_topic_hint(h: &str) -> String {
    let h = h.trim();
    if let Some(i) = h.find("（路人在旁") {
        return h[..i].trim().to_string();
    }
    h.to_string()
}

/// 依 `topic_hint` 反查主題 id（對齊 Go `FindNpcNpcTopicIDByHint`）。
#[must_use]
pub fn find_npc_npc_topic_id_by_hint(topic_hint: &str) -> String {
    let h = normalize_npc_npc_topic_hint(topic_hint);
    if h.is_empty() {
        return String::new();
    }
    let g = NPC_TOPICS_LIST.read().expect("npc topics lock");
    for t in g.iter() {
        let th = t.hint.trim();
        if !th.is_empty() && h == th {
            return t.id.clone();
        }
    }
    let mut best_id = String::new();
    let mut best_len = 0usize;
    for t in g.iter() {
        let th = t.hint.trim();
        if th.is_empty() {
            continue;
        }
        if h.starts_with(th) && th.len() > best_len {
            best_len = th.len();
            best_id = t.id.clone();
        }
    }
    best_id
}

/// 隨機一主題（對齊 Go `PickRandomNpcNpcTopic`）。
#[must_use]
pub fn pick_random_npc_npc_topic() -> Option<NpcNpcTopic> {
    pick_random_npc_npc_topic_exclude("")
}

/// 隨機主題，可排除 `exclude_id`（對齊 Go `PickRandomNpcNpcTopicExclude`）。
#[must_use]
pub fn pick_random_npc_npc_topic_exclude(exclude_id: &str) -> Option<NpcNpcTopic> {
    let g = NPC_TOPICS_LIST.read().expect("npc topics lock");
    let candidates: Vec<usize> = g
        .iter()
        .enumerate()
        .filter(|(_, t)| exclude_id.is_empty() || t.id != exclude_id)
        .map(|(i, _)| i)
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let mut rng = rand::rng();
    let idx = candidates[rng.random_range(0..candidates.len())];
    g.get(idx).cloned()
}

/// 主題篩選條件（對齊 Go `NpcTopicMask`）。
#[derive(Debug, Clone, Default)]
pub struct NpcTopicMask {
    pub is_work_venue: bool,
    pub is_night_time: bool,
    pub has_room_event: bool,
    pub room_tags: Vec<String>,
}

/// 除錯用權重明細（對齊 Go `TopicWeightDebug`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicWeightDebug {
    pub id: String,
    pub hint: String,
    pub weight: i32,
    pub eligible: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

#[must_use]
pub fn room_has_tag(room_tags: &[String], want: &str) -> bool {
    room_tags.iter().any(|v| v == want)
}

/// 依情境與房間 tag 計算基礎權重（對齊 Go `topicMaskBaseWeight`）。
#[must_use]
pub fn topic_mask_base_weight(mask: &NpcTopicMask, t: &NpcNpcTopic) -> i32 {
    let mut w = 1;
    if mask.is_work_venue && t.requires_work {
        w += 4;
    }
    if mask.has_room_event && t.follow_up {
        w += 3;
    }
    if mask.is_night_time && t.night_only {
        w += 2;
    }
    if room_has_tag(&mask.room_tags, "outdoor") {
        if t.id == "城外聞" {
            w += 5;
        }
        if t.id == "天候" {
            w += 3;
        }
    }
    if room_has_tag(&mask.room_tags, "social") && t.id == "閒聊" {
        w += 2;
    }
    w.max(1)
}

/// 雙人關係額外加權（對齊 Go `pairRelationWeightAdj`）。
#[must_use]
pub fn pair_relation_weight_adj(t: &NpcNpcTopic, familiarity: i32, sentiment: i32, dyad_tags: &[String]) -> i32 {
    let has_tag = |s: &str| dyad_tags.iter().any(|v| v == s);
    let mut w = 0;
    if familiarity >= 50 && t.id == "閒聊" {
        w += 2;
    }
    if has_tag("同職場") && t.id == "交班" {
        w += 3;
    }
    if sentiment <= -20 && t.id == "打聽" {
        w += 2;
    }
    w
}

/// 依情境加權抽主題；無符合則回退隨機（對齊 Go `PickRandomNpcNpcTopicByMask`）。
#[must_use]
pub fn pick_random_npc_npc_topic_by_mask(mask: &NpcTopicMask, exclude_id: &str) -> Option<NpcNpcTopic> {
    let list: Vec<NpcNpcTopic> = NPC_TOPICS_LIST.read().expect("npc topics lock").clone();
    if list.is_empty() {
        return None;
    }
    let mut candidates: Vec<(usize, i32)> = Vec::new();
    let mut fallback: Vec<usize> = Vec::new();
    for (i, t) in list.iter().enumerate() {
        if !exclude_id.is_empty() && t.id == exclude_id {
            continue;
        }
        fallback.push(i);
        if t.requires_work && !mask.is_work_venue {
            continue;
        }
        if t.night_only && !mask.is_night_time {
            continue;
        }
        if t.follow_up && !mask.has_room_event {
            continue;
        }
        let w = topic_mask_base_weight(mask, t);
        candidates.push((i, w));
    }
    let mut rng = rand::rng();
    if !candidates.is_empty() {
        let total: i32 = candidates.iter().map(|(_, w)| *w).sum();
        if total > 0 {
            let r = rng.random_range(0..total);
            let mut acc = 0;
            for (idx, w) in &candidates {
                acc += w;
                if r < acc {
                    return list.get(*idx).cloned();
                }
            }
        }
    }
    if !fallback.is_empty() {
        let j = rng.random_range(0..fallback.len());
        return list.get(fallback[j]).cloned();
    }
    None
}

/// 情境 + 雙人關係加權抽主題（對齊 Go `PickRandomNpcNpcTopicForPair`）。
#[must_use]
pub fn pick_random_npc_npc_topic_for_pair(
    mask: &NpcTopicMask,
    exclude_id: &str,
    familiarity: i32,
    sentiment: i32,
    tags: &[String],
) -> Option<NpcNpcTopic> {
    let list: Vec<NpcNpcTopic> = NPC_TOPICS_LIST.read().expect("npc topics lock").clone();
    if list.is_empty() {
        return None;
    }
    let mut candidates: Vec<(usize, i32)> = Vec::new();
    let mut fallback: Vec<usize> = Vec::new();
    for (i, t) in list.iter().enumerate() {
        if !exclude_id.is_empty() && t.id == exclude_id {
            continue;
        }
        fallback.push(i);
        if t.requires_work && !mask.is_work_venue {
            continue;
        }
        if t.night_only && !mask.is_night_time {
            continue;
        }
        if t.follow_up && !mask.has_room_event {
            continue;
        }
        let mut w = topic_mask_base_weight(mask, t) + pair_relation_weight_adj(t, familiarity, sentiment, tags);
        if w < 1 {
            w = 1;
        }
        candidates.push((i, w));
    }
    let mut rng = rand::rng();
    if !candidates.is_empty() {
        let total: i32 = candidates.iter().map(|(_, w)| *w).sum();
        if total > 0 {
            let r = rng.random_range(0..total);
            let mut acc = 0;
            for (idx, w) in &candidates {
                acc += w;
                if r < acc {
                    return list.get(*idx).cloned();
                }
            }
        }
    }
    if !fallback.is_empty() {
        let j = rng.random_range(0..fallback.len());
        return list.get(fallback[j]).cloned();
    }
    None
}

/// 除錯：各主題權重拆解（對齊 Go `DebugTopicWeightsForPair`）。
#[must_use]
pub fn debug_topic_weights_for_pair(
    mask: &NpcTopicMask,
    exclude_id: &str,
    familiarity: i32,
    sentiment: i32,
    tags: &[String],
) -> Vec<TopicWeightDebug> {
    let g = NPC_TOPICS_LIST.read().expect("npc topics lock");
    let mut out = Vec::with_capacity(g.len());
    for t in g.iter() {
        let mut d = TopicWeightDebug {
            id: t.id.clone(),
            hint: t.hint.clone(),
            eligible: true,
            weight: 0,
            reason: String::new(),
        };
        if !exclude_id.is_empty() && t.id == exclude_id {
            d.eligible = false;
            d.reason = "excluded by id".to_string();
            out.push(d);
            continue;
        }
        if t.requires_work && !mask.is_work_venue {
            d.eligible = false;
            d.reason = "requires_work".to_string();
            out.push(d);
            continue;
        }
        if t.night_only && !mask.is_night_time {
            d.eligible = false;
            d.reason = "night_only".to_string();
            out.push(d);
            continue;
        }
        if t.follow_up && !mask.has_room_event {
            d.eligible = false;
            d.reason = "follow_up without room event".to_string();
            out.push(d);
            continue;
        }
        d.weight = topic_mask_base_weight(mask, t) + pair_relation_weight_adj(t, familiarity, sentiment, tags);
        out.push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topics_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/npc_to_npc_topics.json")
    }

    #[test]
    fn find_topic_id_by_hint_matches_go() {
        try_load_npc_npc_topics(&topics_path()).expect("load topics");
        assert_eq!(
            find_npc_npc_topic_id_by_hint("換班、交接時的情境，簡短交代或叮囑"),
            "交班"
        );
        assert_eq!(
            find_npc_npc_topic_id_by_hint("隨口閒聊，天氣、見聞、抱怨（路人在旁，兩句務必極短。）"),
            "閒聊"
        );
        assert_eq!(
            find_npc_npc_topic_id_by_hint(
                "鎮上傳聞城外惡地、霜林、游離輻射或富態化巨獸（非荒蕪末世，而是生機過剩的危險）"
            ),
            "城外聞"
        );
        assert_eq!(find_npc_npc_topic_id_by_hint(""), "");
    }

    #[test]
    fn get_by_id_after_load() {
        try_load_npc_npc_topics(&topics_path()).expect("load");
        assert_eq!(
            get_npc_npc_topic_by_id("閒聊").map(|t| t.hint.contains("隨口")),
            Some(true)
        );
    }
}
