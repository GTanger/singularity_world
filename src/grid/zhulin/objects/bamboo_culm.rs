//! 竹林系列採集物件 1：【合抱刺竹叢】（`zl_bamboo_culm`）
//!
//! 生態定位：主體結構型林木（Bulk Timber Stand）
//! - 初始存量：6 ～ 10 根成熟刺竹稈（絕非砍兩三下即憑空消失的小草）
//! - 自然再生：`regrowth_per_tick = 1`
//! - 場景短句（L3）、端詳觀察（Look）與採集回饋（Gather）皆以自然量詞「根／株／竹樁」呈現，不暴露後台數字分數。

use std::collections::HashMap;

use crate::grid::cell::{GridObject, GridObjectKind};
use crate::model::RoomObject;

pub const ID: &str = "zl_bamboo_culm";
pub const NAME: &str = "合抱刺竹叢";
pub const YIELD_ITEM_ID: &str = "zl_item_bamboo_culm";
pub const YIELD_NOUN: &str = "刺竹稈";
pub const KIND: GridObjectKind = GridObjectKind::Flora;
pub const CELL_TAGS: &[&str] = &["bamboo", "timber", "flora", "wind_shelter"];

/// 場景採集物件三層標籤（類別標籤、屬性標籤、專屬標籤）
pub const OBJECT_TAGS: super::ZhulinTagTriad = super::ZhulinTagTriad {
    category_tags: &["flora", "bamboo_stand", "gather_node"],
    attribute_tags: &["thorny", "tall_culm", "wind_shelter", "regenerative"],
    exclusive_tag: "zl_obj_bamboo_culm",
};

/// 採集產出物品物件定義與三層標籤（獨立專屬 ID，絕不共用舊世 harvest_wood）
pub const YIELD_ITEM_DEF: super::ZhulinItemDef = super::ZhulinItemDef {
    id: YIELD_ITEM_ID,
    name: YIELD_NOUN,
    item_type: "material",
    weight: 1.2,
    origin: "刺竹林",
    description: "自合抱刺竹叢伐下的成熟刺竹稈，中空有節、竹壁堅厚，可鋸作竹筒或劈削竹篾。",
    tags: super::ZhulinTagTriad {
        category_tags: &["timber", "bamboo_material", "structural_pole"],
        attribute_tags: &["hollow_jointed", "splittable", "tough_wall", "flammable_pop"],
        exclusive_tag: YIELD_ITEM_ID,
    },
};

/// 上方 L3 場景描述短句（隨剩餘可伐成竹根數分三階段演進；歸零時變為 `[空白]` 消失）
pub fn scene_sentence(qty: u32) -> &'static str {
    match qty {
        0 => "",
        1 => "竹樁間僅餘一株待採的合抱刺竹叢，竹節密布彎刺。",
        2..=4 => "林間尚立幾株合抱刺竹叢，根部留有新伐竹樁。",
        _ => "數叢粗壯的合抱刺竹叢拔地而起，竹節密布彎刺。",
    }
}

/// 點擊物件端詳（Look）時的自然觀察描述（依剩餘可伐根數呈現豐枯狀態，取代 `（物產存量：X/Y）`）
pub fn look_description(qty: u32) -> String {
    let base = "竹稈粗如合抱的原生刺竹叢，竹節密布堅硬彎刺。";
    let state = match qty {
        0 => "成熟竹稈已悉數伐去，僅留青翠竹樁待抽新。",
        1..=2 => "大半成竹已被伐去，竹樁間僅剩一兩根成竹可伐。",
        3..=5 => "外圍留有幾處新伐竹樁，叢中尚餘數根筆直成竹。",
        _ => "叢中筆直堪用的成熟刺竹稈尚有十來根，枝幹繁茂。",
    };
    format!("{base}{state}")
}

/// 執行 `[採集]` 時的專屬動作敘事（明確區分「場景物件：合抱刺竹叢」與「採得產物：刺竹稈」）
pub fn gather_narrative(harvested_count: u32, remaining_qty: u32) -> String {
    let action_part = if harvested_count > 1 {
        format!("你避開竹節硬刺，俐落伐下 {harvested_count} 根堅韌的「{YIELD_NOUN}」收入行囊。")
    } else {
        format!("你避開竹節硬刺，自刺竹叢伐下一根堅韌的「{YIELD_NOUN}」收入行囊。")
    };
    let remain_part = match remaining_qty {
        0 => "最後一根成竹應聲倒下，此處已無成材可伐。",
        1..=2 => "叢中僅餘最後一兩根成竹尚可取用。",
        3..=5 => "叢中尚餘數根筆直成竹可供採伐。",
        _ => "整叢刺竹依舊枝幹繁茂、成材充裕。",
    };
    format!("{action_part}{remain_part}")
}

/// 未持 `heavy_chop`（砍劈利器）徒手採伐時的物理受阻敘事（扣 0 存量、得 0 產物）
pub fn blocked_gather_narrative() -> String {
    "刺竹基部密布堅硬彎刺，竹壁厚實如鐵，徒手攀折只會被銳刺扎傷手掌；需持有石斧等砍劈利器方可伐取竹稈。".to_string()
}

pub fn build_grid_object(qty: u32) -> GridObject {
    let q = qty.max(1);
    GridObject {
        id: ID.to_string(),
        kind: KIND,
        name: NAME.to_string(),
        quantity: q,
        max_quantity: q,
        regrowth_per_tick: 1,
    }
}

pub fn to_room_object(qty: u32) -> RoomObject {
    let sockets = if qty > 0 {
        vec!["Look".to_string(), "Gather".to_string()]
    } else {
        vec!["Look".to_string()]
    };
    RoomObject {
        id: ID.to_string(),
        name: NAME.to_string(),
        owner: String::new(),
        sockets,
        responses: HashMap::from([("Look".to_string(), look_description(qty))]),
        move_to_room_id: String::new(),
    }
}
