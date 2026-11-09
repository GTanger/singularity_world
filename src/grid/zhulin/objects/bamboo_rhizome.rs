//! 竹林系列採集物件 4：【盤錯竹鞭】（`zl_bamboo_rhizome`）
//!
//! 生態定位：地下堅韌竹根鞭，可作器柄韌材
//! - 初始存量：2 ～ 4 段
//! - 自然再生：`regrowth_per_tick = 1`

use std::collections::HashMap;

use crate::grid::cell::{GridObject, GridObjectKind};
use crate::model::RoomObject;

pub const ID: &str = "zl_bamboo_rhizome";
pub const NAME: &str = "盤錯竹鞭";
pub const YIELD_ITEM_ID: &str = "zl_item_bamboo_rhizome";
pub const YIELD_NOUN: &str = "老竹根鞭";
pub const KIND: GridObjectKind = GridObjectKind::Flora;
pub const CELL_TAGS: &[&str] = &["bamboo", "rhizome", "timber", "flora"];

/// 場景採集物件三層標籤（類別標籤、屬性標籤、專屬標籤）
pub const OBJECT_TAGS: super::ZhulinTagTriad = super::ZhulinTagTriad {
    category_tags: &["flora", "root_rhizome", "gather_node"],
    attribute_tags: &["interlaced", "dense_node", "semi_buried", "tough"],
    exclusive_tag: "zl_obj_bamboo_rhizome",
};

/// 採集產出物品物件定義與三層標籤（獨立專屬 ID，絕不共用舊世 harvest_rare_wood）
pub const YIELD_ITEM_DEF: super::ZhulinItemDef = super::ZhulinItemDef {
    id: YIELD_ITEM_ID,
    name: YIELD_NOUN,
    item_type: "material",
    weight: 0.8,
    origin: "刺竹林",
    description: "自竹林土表掘出的老竹地下根鞭，節短壁厚近乎實心，質地極韌耐劈，堪作器柄或敲擊硬材。",
    tags: super::ZhulinTagTriad {
        category_tags: &["haft_material", "carving_stock", "bamboo_rhizome"],
        attribute_tags: &["solid_core", "dense_joint", "shock_resistant", "curved_grain"],
        exclusive_tag: YIELD_ITEM_ID,
    },
};

/// 上方 L3 場景描述短句（歸零時變為 `""` 消失）
pub fn scene_sentence(qty: u32) -> &'static str {
    match qty {
        0 => "",
        1 => "泥土表層裸露著最後一段曲折堅韌的盤錯竹鞭。",
        _ => "土表半露著幾段節理緊密、如鐵線交纏的盤錯竹鞭。",
    }
}

/// 點擊物件端詳（Look）時的自然觀察描述（<= 44 字，不含物產存量括號）
pub fn look_description(qty: u32) -> String {
    let base = "竹林地底橫走破土的老竹根鞭，節短壁厚、質地極韌。";
    let state = match qty {
        0 => "裸露土表的老竹鞭已悉數掘去，僅留深陷泥槽。",
        1 => "泥縫間僅剩最後一段老竹鞭尚可掘取。",
        _ => "土表尚有數段粗實老竹鞭盤結，堪作器柄韌材。",
    };
    format!("{base}{state}")
}

/// 執行 `[採集]` 時的專屬動作敘事（<= 44 字）
pub fn gather_narrative(harvested_count: u32, remaining_qty: u32) -> String {
    let action_part = if harvested_count > 1 {
        format!("你沿泥縫斬斷鬚根，掘出 {harvested_count} 段堅韌的「{YIELD_NOUN}」收入行囊。")
    } else {
        format!("你沿泥縫斬斷鬚根，掘出一段堅韌的「{YIELD_NOUN}」收入行囊。")
    };
    let remain_part = match remaining_qty {
        0 => "裸露的竹鞭已盡數掘完，只留泥溝。",
        1 => "土表僅餘最後一段老竹鞭半露。",
        _ => "土表尚餘兩三段老竹鞭盤結。",
    };
    format!("{action_part}{remain_part}")
}

/// 未持切割／掘土／砍劈工具（`sharp_edge` / `earth_dig` / `heavy_chop`）徒手拉扯時的物理受阻敘事
pub fn blocked_gather_narrative() -> String {
    "土表半露的老竹根鞭節短壁厚、鬚根深鎖紅土，徒手拉扯紋絲不動，反倒磨得掌心發紅；需藉助銳邊石片或掘土器物斬斷鬚根方可掘出。".to_string()
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
