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

/// NPC 內心活動，帶來源、強度、衰減期。
/// 加總尚未衰減完的 thoughts.delta = 當前 disposition 投影值。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    /// 來源 key（對應 ThoughtSource 列舉的字串值，便於 i18n／散文化）
    pub source: String,
    /// 強度（正負，未限幅；最終 disposition 投影時才 clamp）
    pub delta: i32,
    /// 創建時的真實 Unix 秒（SystemTime）
    pub created_at: i64,
    /// 衰減期（分鐘，超過後不計入；比較時換算為秒 * 60）
    pub decay_minutes: i64,
    /// 給 LLM 用的散文，由 source + 詞盤 / 簽名後續變化（可空，預設由 source 對應表抽）
    pub flavor: Option<String>,
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
    /// 心境 -100~+100，0=中性。作為 thought log 加總的 cache，每次 add_thought 後重算。
    #[serde(default)]
    pub disposition: i32,
    /// NPC 內心活動列表（最近 100 條，超過砍最舊）
    #[serde(default)]
    pub thoughts: Vec<Thought>,
    /// 表面可觀測行為，玩家 Look 時看到的「在做什麼」。
    #[serde(default)]
    pub current_activity: String,
    /// 正方格座標 x（東增）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_x: Option<i32>,
    /// 正方格座標 y（北增）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_y: Option<i32>,
}

impl Character {
    /// 回傳此角色對外開放的動詞清單（預設插座）。
    pub fn sockets() -> Vec<Verb> {
        vec![
            Verb::Talk,
            Verb::Borrow,
            Verb::Subdue,
            Verb::Slay,
            Verb::Look,
            Verb::Trade,
        ]
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
