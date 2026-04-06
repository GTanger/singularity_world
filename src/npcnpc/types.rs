// NPC↔NPC 除錯 API 型別，對齊既有 npcnpc/types。

use serde::Serialize;

use crate::ai::DialogueScoreDetail;
use crate::npc::TopicWeightDebug;
use crate::store::{NpcDyad, NpcThread};

/// 最近一次 NPC↔NPC 社交決策（對齊既有 `LastSocialChoice`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LastSocialChoice {
    pub at: i64,
    pub room_id: String,
    pub a_id: String,
    pub b_id: String,
    pub score_total: i32,
    pub topic_hint: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub topic_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub topic_reason: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub anchors_used: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub anchors_written: Vec<String>,
    pub anchor_conflict: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub anchor_conflict_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialogue_score: Option<DialogueScoreDetail>,
}

/// 一對 NPC 配對除錯細項（對齊既有 `PairDebug`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PairDebug {
    pub a_id: String,
    pub b_id: String,
    pub last_talk_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread: Option<NpcThread>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dyad: Option<NpcDyad>,
    pub score_total: i32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub score_detail: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub topic_weights: Vec<TopicWeightDebug>,
}

/// `GET /api/debug/npc-social` 回應骨架（對齊既有 `DebugResp`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DebugResp {
    pub room_id: String,
    pub room_name: String,
    pub events: Vec<String>,
    pub pairs: Vec<PairDebug>,
    pub npc_ids: Vec<String>,
    pub server_unix: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_choice: Option<LastSocialChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<std::collections::HashMap<String, i64>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rumors: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rumor_digest: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rumor_details: Vec<RumorDebugItem>,
}

/// 傳聞池除錯一筆（對齊既有 `RumorDebugItem`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RumorDebugItem {
    pub text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub source: String,
    pub weight: i32,
    pub source_score: i32,
    pub mention_count: i32,
    #[serde(skip_serializing_if = "is_zero_i64")]
    pub last_used_at: i64,
    #[serde(skip_serializing_if = "is_zero_i64")]
    pub blocked_until: i64,
    #[serde(skip_serializing_if = "is_zero_i32")]
    pub penalty_count: i32,
    #[serde(skip_serializing_if = "is_zero_i64")]
    pub last_penalty_at: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub last_penalty_reason: String,
    pub status: String,
}

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

fn is_zero_i32(v: &i32) -> bool {
    *v == 0
}
