//! WebSocket／HTTP JSON 訊息型別（對齊既有 `server/protocol`）。
//! 欄位名與 `omitempty` 行為需與前端一致。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 客戶端送出的 JSON；`type` 決定行為。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default)]
    pub player_id: String,
    #[serde(default)]
    pub password: String,
    #[serde(default, rename = "display_char")]
    pub display_char: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub direction: String,
    /// 同一方向有多條路時，玩家按下的確切相鄰場所。
    #[serde(default, rename = "target_place_id")]
    pub target_place_id: String,
    #[serde(default, rename = "entity_id")]
    pub entity_id: String,
    #[serde(default, rename = "item_id")]
    pub item_id: String,
    #[serde(default, rename = "target_slot")]
    pub target_slot: String,
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub action: String,
    #[serde(default, rename = "player_input")]
    pub player_input: String,
    #[serde(default)]
    pub nodes: Vec<String>,
    #[serde(default)]
    pub words: Vec<String>,
    #[serde(default)]
    pub inlays: Vec<(String, String)>,
    #[serde(default, rename = "custom_name")]
    pub custom_name: Option<String>,
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    #[serde(default, rename = "target_terrain")]
    pub target_terrain: String,
    #[serde(default)]
    pub choice: String,
}

/// 房間內單一實體（對齊 `ViewEntity`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewEntity {
    pub id: String,
    pub kind: String,
    #[serde(rename = "display_char")]
    pub display_char: String,
    #[serde(rename = "display_name")]
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    /// 此刻必須保留在可接觸人物卡中的場所角色；只表示互動入口，不公開私名。
    #[serde(default, skip_serializing_if = "is_false")]
    pub interaction_priority: bool,
    /// 人物在世界中實際進行的活動；玩家層只顯示可感知的簡短文字。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub current_activity: String,
    /// NPC 行為敘事句（可選；非 NPC 或無行為時省略）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub behavior_text: String,
}

/// `do_action` 敘事結果（對齊 `ActionResultMsg`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionResultMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub action: String,
    #[serde(rename = "target_id")]
    pub target_id: String,
    #[serde(rename = "target_name")]
    pub target_name: String,
    pub narrative: String,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "move_target_id"
    )]
    pub move_target_id: String,
    #[serde(default, skip_serializing_if = "is_false", rename = "open_trade")]
    pub open_trade: bool,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "suggested_action"
    )]
    pub suggested_action: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tactical_choices: Vec<String>,
}

fn is_false(v: &bool) -> bool {
    !*v
}

/// 單一出口（對齊 `ExitView`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitView {
    pub direction: String,
    #[serde(rename = "to_room_id")]
    pub to_room_id: String,
    #[serde(rename = "to_room_name")]
    pub to_room_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "route_id")]
    pub route_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mode: String,
    #[serde(default, skip_serializing_if = "is_zero_i32", rename = "travel_cost")]
    pub travel_cost: i32,
}

fn is_zero_i32(value: &i32) -> bool {
    *value == 0
}

/// 房間內可互動物件（對齊 `ViewObject`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewObject {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "type")]
    pub object_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
}

/// 當前房間視野（對齊 `RoomViewMsg`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomViewMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "room_id")]
    pub room_id: String,
    #[serde(rename = "room_name")]
    pub room_name: String,
    pub description: String,
    pub exits: Vec<ExitView>,
    pub entities: Vec<ViewEntity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<ViewObject>,
    #[serde(rename = "server_unix")]
    pub server_unix: i64,
    #[serde(rename = "game_time_sec_since_midnight")]
    pub game_time_sec_since_midnight: i32,
    #[serde(rename = "game_days_since_epoch")]
    pub game_days_since_epoch: i32,
}

/// 登入成功後玩家狀態（對齊 `MeMsg`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "player_id")]
    pub player_id: String,
    #[serde(rename = "room_id")]
    pub room_id: String,
    #[serde(rename = "room_name")]
    pub room_name: String,
    pub vit: i32,
    pub qi: i32,
    pub dex: i32,
    #[serde(rename = "hp_cur")]
    pub hp_cur: i32,
    #[serde(rename = "hp_max")]
    pub hp_max: i32,
    #[serde(rename = "inner_cur")]
    pub inner_cur: i32,
    #[serde(rename = "inner_max")]
    pub inner_max: i32,
    #[serde(rename = "spirit_cur")]
    pub spirit_cur: i32,
    #[serde(rename = "spirit_max")]
    pub spirit_max: i32,
    #[serde(rename = "stamina_cur")]
    pub stamina_cur: i32,
    #[serde(rename = "stamina_max")]
    pub stamina_max: i32,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "display_title"
    )]
    pub display_title: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "origin_sentence"
    )]
    pub origin_sentence: String,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "activated_nodes"
    )]
    pub activated_nodes: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "topology_costs"
    )]
    pub topology_costs: Vec<f64>,
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        rename = "equipment_slots"
    )]
    pub equipment_slots: HashMap<String, String>,
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        rename = "equipment_names"
    )]
    pub equipment_names: HashMap<String, String>,
}

/// 移動廣播（對齊 `MovedMsg`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovedMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "player_id")]
    pub player_id: String,
    #[serde(rename = "room_id")]
    pub room_id: String,
    #[serde(rename = "room_name")]
    pub room_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NpcDialogueMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub npc_a: String,
    pub npc_b: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub direction: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PongMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrateMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyDebugAckMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryItemView {
    #[serde(rename = "item_id")]
    pub item_id: String,
    pub name: String,
    #[serde(rename = "item_type")]
    pub item_type: String,
    pub qty: i32,
    pub weight: f64,
    #[serde(rename = "sub_total")]
    pub sub_total: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub slot: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub origin: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub items: Vec<InventoryItemView>,
    #[serde(rename = "current_weight")]
    pub current_weight: f64,
    #[serde(rename = "max_weight")]
    pub max_weight: f64,
}

/// 角色背板「技能」欄位之單一自然生成技能（由後台依人物所學字印、所熟悉器物與星盤屬性自然生成，非玩家手動自訂）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalSkillView {
    /// "martial"（臨陣武相・戰鬥自動使用）或 "production"（百工藝相・生產自動使用）
    pub domain: String,
    /// 類別標籤（如 "inner_breath"）
    #[serde(default)]
    pub category: String,
    /// 序位或領域標籤
    #[serde(rename = "category_label")]
    pub category_label: String,
    /// 後台自然生成之技能名稱
    pub name: String,
    /// 碎招與套路根源說明
    #[serde(rename = "origin_basis")]
    pub origin_basis: String,
    /// 所學字韻與熟悉器物依據
    #[serde(rename = "learned_and_familiar")]
    pub learned_and_familiar: String,
    /// 戰鬥或生產中自動施展之效果說明
    #[serde(rename = "auto_trigger_desc")]
    pub auto_trigger_desc: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityStatusMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "entity_id")]
    pub entity_id: String,
    #[serde(rename = "display_char")]
    pub display_char: String,
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "room_id")]
    pub room_id: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "room_name"
    )]
    pub room_name: String,
    pub vit: i32,
    pub qi: i32,
    pub dex: i32,
    #[serde(rename = "hp_cur")]
    pub hp_cur: i32,
    #[serde(rename = "hp_max")]
    pub hp_max: i32,
    #[serde(rename = "inner_cur")]
    pub inner_cur: i32,
    #[serde(rename = "inner_max")]
    pub inner_max: i32,
    #[serde(rename = "spirit_cur")]
    pub spirit_cur: i32,
    #[serde(rename = "spirit_max")]
    pub spirit_max: i32,
    #[serde(rename = "stamina_cur")]
    pub stamina_cur: i32,
    #[serde(rename = "stamina_max")]
    pub stamina_max: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magnesium: Option<i32>,
    #[serde(rename = "is_self")]
    pub is_self: bool,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "display_title"
    )]
    pub display_title: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "origin_sentence"
    )]
    pub origin_sentence: String,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "activated_nodes"
    )]
    pub activated_nodes: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "topology_costs"
    )]
    pub topology_costs: Vec<f64>,
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        rename = "equipment_slots"
    )]
    pub equipment_slots: HashMap<String, String>,
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        rename = "equipment_names"
    )]
    pub equipment_names: HashMap<String, String>,
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        rename = "equipment_descs"
    )]
    pub equipment_descs: HashMap<String, String>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "natural_skills"
    )]
    pub natural_skills: Vec<NaturalSkillView>,
}

/// 正方格地圖視野（傳送玩家附近的多格資訊）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridViewMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "player_x")]
    pub player_x: i32,
    #[serde(rename = "player_y")]
    pub player_y: i32,
    pub cells: Vec<GridCellView>,
    #[serde(rename = "room_name")]
    pub room_name: String,
    pub description: String,
    pub exits: Vec<ExitView>,
    pub entities: Vec<ViewEntity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<ViewObject>,
    #[serde(rename = "server_unix")]
    pub server_unix: i64,
    #[serde(rename = "game_time_sec_since_midnight")]
    pub game_time_sec_since_midnight: i32,
    #[serde(rename = "game_days_since_epoch")]
    pub game_days_since_epoch: i32,
    #[serde(default)]
    pub in_private_room: bool,
    #[serde(default)]
    pub season_date_label: String,
}

/// 正方格單格視野資訊（地圖渲染用）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridCellView {
    pub x: i32,
    pub y: i32,
    /// Terrain enum 變體名（snake_case），前端切色/icon 用
    pub kind: String,
    pub terrain: String,
    pub name: String,
    /// "terrain"（自然）/ "landmark"（人造）/ "infra"（通行建設）
    pub category: String,
    pub explored: bool,
    pub walkable: bool,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub npc_count: u16,
}

// ─── v5 新增：探索 log 與世界狀態推播 ─────────────────────────────────

/// 探索結果中的單筆戰利品。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LootItem {
    #[serde(rename = "item_id")]
    pub item_id: String,
    pub name: String,
    /// 正值表示獲得；負值表示昏倒時失去。
    pub qty: i32,
}

/// 屬性變化量（歸零表示無變化）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct StatDelta {
    pub hp_cur: i32,
    pub inner_cur: i32,
}

/// 玩家探索動作回應（WS 推播）。
///
/// `outcome` 可為：`harvest` / `combat` / `empty` / `warning` / `faint`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ExploreLogMsg {
    #[serde(rename = "type")]
    pub msg_type: String, // 固定值 "explore_log"
    pub outcome: String,
    pub log: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loot: Vec<LootItem>,
    pub stat_delta: StatDelta,
    pub time_period_after: String,
    pub freshness_after: String,
    /// 預兆散文；空字串代表無預兆。
    pub warning_prose: String,
}

/// 世界自走事件推播（不入玩家 log buffer）。
///
/// `trigger` 可為：`npc_move` / `time_advance` / `climate_change`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct StateUpdateMsg {
    #[serde(rename = "type")]
    pub msg_type: String, // 固定值 "state_update"
    pub trigger: String,
    pub payload: serde_json::Value,
}

/// 詞匣單一詞元條目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CasketWordEntry {
    pub id: String,
    pub ch: String,
    pub count: u32,
    pub carrier_affinity: String,
    pub target_part: String,
    #[serde(default)]
    pub quality_label: String,
    #[serde(default)]
    pub charge_pct: u8,
    #[serde(default)]
    pub extraction_ratio: u8,
    #[serde(default)]
    pub semantic: String,
    #[serde(default)]
    pub craft_label: String,
}

/// 招式槽位之視覺快照（完全去數值化）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticalSentenceView {
    pub slot: String,
    pub name: String,
    pub words: Vec<String>,
    #[serde(default)]
    pub nodes: Vec<String>,
    pub fluency_label: String,
    pub primary_carrier: String,
    pub primary_target: String,
}

/// 戰術配置整合視圖。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticalLoadoutView {
    pub opening: Option<TacticalSentenceView>,
    pub midgame: Option<TacticalSentenceView>,
    pub finisher: Option<TacticalSentenceView>,
    pub defense: Option<TacticalSentenceView>,
}

/// 推送給客戶端的完整戰術狀態。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticalStateMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub casket: Vec<CasketWordEntry>,
    pub loadout: TacticalLoadoutView,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_msg_minimal_json() {
        let j = r#"{"type":"ping"}"#;
        let m: ClientMsg = serde_json::from_str(j).unwrap();
        assert_eq!(m.msg_type, "ping");
        assert!(m.player_id.is_empty());
    }

    #[test]
    fn room_view_roundtrip() {
        let v = RoomViewMsg {
            msg_type: "view".into(),
            room_id: "r1".into(),
            room_name: "大廳".into(),
            description: "…".into(),
            exits: vec![ExitView {
                direction: "東".into(),
                to_room_id: "r2".into(),
                to_room_name: "走廊".into(),
                route_id: String::new(),
                mode: String::new(),
                travel_cost: 0,
            }],
            entities: vec![ViewEntity {
                id: "p1".into(),
                kind: "player".into(),
                display_char: "我".into(),
                display_name: "p1".into(),
                actions: vec![],
                interaction_priority: false,
                current_activity: String::new(),
                behavior_text: String::new(),
            }],
            objects: vec![],
            server_unix: 1700000000,
            game_time_sec_since_midnight: 3600,
            game_days_since_epoch: 1,
        };
        let s = serde_json::to_string(&v).unwrap();
        let v2: RoomViewMsg = serde_json::from_str(&s).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn entity_status_magnesium_omit() {
        let e = EntityStatusMsg {
            msg_type: "entity_status".into(),
            entity_id: "n1".into(),
            display_char: "店".into(),
            room_id: String::new(),
            room_name: String::new(),
            vit: 1,
            qi: 2,
            dex: 3,
            hp_cur: 10,
            hp_max: 10,
            inner_cur: 0,
            inner_max: 0,
            spirit_cur: 0,
            spirit_max: 0,
            stamina_cur: 0,
            stamina_max: 0,
            magnesium: None,
            is_self: false,
            display_title: String::new(),
            origin_sentence: String::new(),
            activated_nodes: vec![],
            topology_costs: vec![],
            equipment_slots: HashMap::new(),
            equipment_names: HashMap::new(),
            equipment_descs: HashMap::new(),
            natural_skills: vec![],
        };
        let s = serde_json::to_string(&e).unwrap();
        assert!(!s.contains("magnesium"));
        let e2: EntityStatusMsg = serde_json::from_str(&s).unwrap();
        assert_eq!(e2.magnesium, None);
    }

    #[test]
    fn action_result_with_move_target_id() {
        let j = r#"{"type":"action_result","action":"Look","target_id":"o1","target_name":"門","narrative":"…","success":true,"move_target_id":"door_1"}"#;
        let a: ActionResultMsg = serde_json::from_str(j).unwrap();
        assert_eq!(a.move_target_id, "door_1");
        assert!(!a.open_trade);
        assert!(a.suggested_action.is_empty());
        let out = serde_json::to_string(&a).unwrap();
        assert!(out.contains("move_target_id"));
    }

    #[test]
    fn action_result_open_trade_roundtrip() {
        let a = ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Talk".into(),
            target_id: "npc_1".into(),
            target_name: "行商".into(),
            narrative: "你問起交易。".into(),
            success: true,
            actions: vec!["Trade".into()],
            move_target_id: String::new(),
            open_trade: true,
            suggested_action: "Trade".into(),
            tactical_choices: vec![],
        };
        let out = serde_json::to_string(&a).unwrap();
        assert!(out.contains(r#""open_trade":true"#));
        assert!(out.contains(r#""suggested_action":"Trade""#));
        let decoded: ActionResultMsg = serde_json::from_str(&out).unwrap();
        assert!(decoded.open_trade);
        assert_eq!(decoded.suggested_action, "Trade");
    }
}
