//! 竹林系列採集物件 2：【乾竹葉層】（`zl_dry_leaves`）
//!
//! 生態定位：地表鋪積型豐沛耗材（Abundant Surface Litter）
//! - 初始存量：8 ～ 14 把乾竹葉（滿地落葉隨手抓幾把不會見底，唯有反覆大量摟取才會露出紅土）
//! - 自然再生：`regrowth_per_tick = 2`（竹林隨風落葉，回復最快）
//! - 場景短句（L3）、端詳觀察（Look）與採集回饋（Gather）皆以「積葉厚薄／幾把／幾撮」呈現。

use std::collections::HashMap;

use crate::grid::cell::{GridObject, GridObjectKind};
use crate::model::RoomObject;

pub const ID: &str = "zl_dry_leaves";
pub const NAME: &str = "乾竹葉層";
pub const YIELD_ITEM_ID: &str = "zl_item_dry_leaves";
pub const YIELD_NOUN: &str = "乾竹葉";
pub const KIND: GridObjectKind = GridObjectKind::Flora;
pub const CELL_TAGS: &[&str] = &["bamboo", "humus", "flora"];

/// 場景採集物件三層標籤（類別標籤、屬性標籤、專屬標籤）
pub const OBJECT_TAGS: super::ZhulinTagTriad = super::ZhulinTagTriad {
    category_tags: &["flora", "surface_litter", "gather_node"],
    attribute_tags: &["dry", "fluffy", "rustling", "fast_regenerative"],
    exclusive_tag: "zl_obj_dry_leaves",
};

/// 採集產出物品物件定義與三層標籤（獨立專屬 ID，絕不共用舊世 harvest_herb）
pub const YIELD_ITEM_DEF: super::ZhulinItemDef = super::ZhulinItemDef {
    id: YIELD_ITEM_ID,
    name: YIELD_NOUN,
    item_type: "material",
    weight: 0.2,
    origin: "刺竹林",
    description: "自林下摟起的乾燥刺竹落葉，質地蓬鬆易燃，極適合作為引火火絨或鋪墊乾草。",
    tags: super::ZhulinTagTriad {
        category_tags: &["tinder", "bedding_material", "bamboo_litter"],
        attribute_tags: &["highly_flammable", "lightweight", "insulating", "dry_crisp"],
        exclusive_tag: YIELD_ITEM_ID,
    },
};

/// 上方 L3 場景描述短句（隨地表落葉厚薄分三階段演進；歸零時變為 `[空白]` 消失）
pub fn scene_sentence(qty: u32) -> &'static str {
    match qty {
        0 => "",
        1..=2 => "泥地上僅剩零星散落的乾竹葉層。",
        3..=5 => "地面散落著半層乾竹葉層，幾處露出紅土。",
        _ => "地面鋪著乾燥蓬鬆的乾竹葉層與筍殼。",
    }
}

/// 點擊物件端詳（Look）時的自然觀察描述（依落葉厚薄呈現豐枯狀態，取代 `（物產存量：X/Y）`）
pub fn look_description(qty: u32) -> String {
    let base = "林下長年堆積的乾燥竹葉與筍籜，質地蓬鬆易燃。";
    let state = match qty {
        0 => "地面落葉已被摟取一空，露出底下微濕紅土。",
        1..=2 => "表層乾葉已拾取大半，泥間僅餘零星幾撮可收。",
        3..=6 => "地面尚鋪著薄層乾葉與筍殼，俯身可摟起數把。",
        _ => "林下積葉厚實蓬鬆、踩踏作響，隨手可摟起大把。",
    };
    format!("{base}{state}")
}

/// 執行 `[採集]` 時的專屬動作敘事（明確區分「場景物件：乾竹葉層」與「採得產物：乾竹葉」）
pub fn gather_narrative(harvested_count: u32, remaining_qty: u32) -> String {
    let action_part = if harvested_count > 1 {
        format!("你俯身自乾竹葉層一連摟起 {harvested_count} 把蓬鬆的「{YIELD_NOUN}」收入行囊。")
    } else {
        format!("你俯身自乾竹葉層抓起一大把蓬鬆的「{YIELD_NOUN}」收入行囊。")
    };
    let remain_part = match remaining_qty {
        0 => "地面最後一撮乾葉已收攏乾淨，露出紅土。",
        1..=2 => "泥間僅剩零星幾撮乾葉散落。",
        3..=6 => "地面尚餘半層乾葉可供拾取。",
        _ => "林下積葉依舊厚實蓬鬆。",
    };
    format!("{action_part}{remain_part}")
}

pub fn build_grid_object(qty: u32) -> GridObject {
    let q = qty.max(1);
    GridObject {
        id: ID.to_string(),
        kind: KIND,
        name: NAME.to_string(),
        quantity: q,
        max_quantity: q,
        regrowth_per_tick: 2,
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
