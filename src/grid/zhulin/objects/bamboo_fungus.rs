//! 竹林系列採集物件 6：【竹根木耳】（`zl_bamboo_fungus`）
//!
//! 生態定位：陰濕老竹樁根部木耳（Shaded Damp Bamboo-Stump Wood Ear Fungus）
//! - 初始存量：1 ～ 2 朵
//! - 自然再生：`regrowth_per_tick = 1`

use std::collections::HashMap;

use crate::grid::cell::{GridObject, GridObjectKind};
use crate::model::RoomObject;

pub const ID: &str = "zl_bamboo_fungus";
pub const NAME: &str = "竹根木耳";
pub const YIELD_ITEM_ID: &str = "zl_item_bamboo_fungus";
pub const YIELD_NOUN: &str = "野生竹耳";
pub const KIND: GridObjectKind = GridObjectKind::Flora;
pub const CELL_TAGS: &[&str] = &["bamboo", "fungus", "edible", "flora"];

/// 場景採集物件三層標籤（類別標籤、屬性標籤、專屬標籤）
pub const OBJECT_TAGS: super::ZhulinTagTriad = super::ZhulinTagTriad {
    category_tags: &["fungi", "stump_fungus", "gather_node"],
    attribute_tags: &["shade_loving", "damp_stump", "gelatinous", "scarce"],
    exclusive_tag: "zl_obj_bamboo_fungus",
};

/// 採集產出物品物件定義與三層標籤（獨立專屬 ID，絕不共用舊世 harvest_berries）
pub const YIELD_ITEM_DEF: super::ZhulinItemDef = super::ZhulinItemDef {
    id: YIELD_ITEM_ID,
    name: YIELD_NOUN,
    item_type: "consumable",
    weight: 0.1,
    origin: "刺竹林",
    description: "貼生於陰濕老竹樁根部的野生木耳，耳片褐亮肥厚、富膠質，可鮮食燉煮或日曬乾存。",
    tags: super::ZhulinTagTriad {
        category_tags: &["food", "edible_fungi", "bamboo_fungus"],
        attribute_tags: &["gelatinous_crisp", "sun_dryable", "soup_ingredient", "mild_sweet"],
        exclusive_tag: YIELD_ITEM_ID,
    },
};

pub fn scene_sentence(qty: u32) -> &'static str {
    match qty {
        0 => "",
        1 => "潮濕的老竹樁根部貼生著一朵褐亮肥厚的竹根木耳。",
        _ => "陰涼腐殖的老竹樁側簇生著幾朵晶瑩肥厚的竹根木耳。",
    }
}

pub fn look_description(qty: u32) -> String {
    let base = "生於陰濕老竹樁根部的野生木耳，耳片肥厚富膠質。";
    let state = match qty {
        0 => "竹樁上的木耳已被採摘一空，僅留淡淡菌痕。",
        1 => "竹樁縫隙僅見這一朵肥厚竹耳，採後便無餘朵。",
        _ => "竹樁陰涼處簇生著兩朵飽滿竹耳，正值鮮嫩可採。",
    };
    format!("{base}{state}")
}

pub fn gather_narrative(harvested_count: u32, remaining_qty: u32) -> String {
    let action_part = if harvested_count > 1 {
        format!("你貼著濕潤竹樁，輕手摘下 {harvested_count} 朵肥厚的「{YIELD_NOUN}」收入行囊。")
    } else {
        format!("你貼著濕潤竹樁，輕手摘下一朵肥厚的「{YIELD_NOUN}」收入行囊。")
    };
    let remain_part = match remaining_qty {
        0 => "竹樁上的木耳已盡數摘完，僅留微濕菌痕。",
        _ => "竹樁側尚留最後一朵竹耳貼生。",
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
