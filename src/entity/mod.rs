// entity 模組 — 玩家／NPC／建築等實體結構與插頭插座邏輯。

use serde::{Deserialize, Serialize};

/// 實體類型：玩家或 NPC。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Player,
    Npc,
}

/// 移動狀態。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoveState {
    #[default]
    Idle,
    Moving,
}

/// 步行或跑步。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WalkOrRun {
    #[default]
    Walk,
    Run,
}

/// 性別。舊版使用 "M"/"F"，serde 對齊。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    M,
    F,
}

/// 動詞（插頭插座系統）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Verb {
    Talk,
    Borrow,
    Subdue,
    Slay,
    Look,
    Trade,
    Read,
    Smell,
    Move,
    Use,
    Open,
    Take,
}

/// 角色（玩家/NPC）共用結構，對齊人物角色模板第一版最小集。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub kind: EntityKind,
    #[serde(default)]
    pub display_char: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub move_state: MoveState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_y: Option<i32>,
    #[serde(default)]
    pub walk_or_run: WalkOrRun,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_started_at: Option<i64>,
    #[serde(default)]
    pub vit: i32,
    #[serde(default)]
    pub qi: i32,
    #[serde(default)]
    pub dex: i32,
    #[serde(default)]
    pub magnesium: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_observed_at: Option<i64>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<Gender>,
    /// 創角時寫入，唯一決定三軸與 760 邊權；None 表示舊資料。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soul_seed: Option<i64>,
    /// 命途稱謂；空則前端顯示「無名之輩」。
    #[serde(default)]
    pub display_title: String,
    /// 星盤已貫通節點 ID 之 JSON 陣列，預設 ["N000"]。
    #[serde(default)]
    pub activated_nodes: String,
    /// 裝備槽位 JSON，key=槽位代碼 value=item_id。
    #[serde(default)]
    pub equipment_slots: String,
    /// 背包物品 JSON 陣列，每元素 {"item_id":"xxx","qty":1}。
    #[serde(default)]
    pub inventory: String,
    /// 心境 -100~+100，0=中性。
    #[serde(default)]
    pub disposition: i32,
    /// 表面可觀測行為，玩家 Look 時看到的「在做什麼」。
    #[serde(default)]
    pub current_activity: String,
    /// Hex even-q 座標；與 store::Entity 對齊。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hex_q: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hex_r: Option<i32>,
}

impl Character {
    /// 回傳此角色對外開放的動詞清單（預設插座）。
    pub fn sockets() -> Vec<Verb> {
        vec![Verb::Talk, Verb::Borrow, Verb::Subdue, Verb::Slay, Verb::Look, Verb::Trade]
    }
}

/// 建築或靜態物，佔格、可阻擋或可接觸。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub id: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    /// true 表示不可進入。
    #[serde(default)]
    pub blocking: bool,
    /// 可執行動作，空則僅擋路。
    #[serde(default)]
    pub socket_list: Vec<String>,
    #[serde(default)]
    pub display_char: String,
}

/// 檢查插座清單是否包含指定動詞。
pub fn has_socket(sockets: &[Verb], verb: &Verb) -> bool {
    sockets.contains(verb)
}
