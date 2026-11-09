//! 竹林系列採集物件 3：【破土竹筍】（`zl_bamboo_shoot`）
//!
//! 生態定位：點狀尋寶型稀有食材（Discrete Scarce Delicacy）
//! - 初始存量：1 ～ 3 株（真正稀少、離散的單株食材，挖一株少一株，挖完即空）
//! - 自然再生：`regrowth_per_tick = 1`
//! - 場景短句（L3）、端詳觀察（Look）與採集回饋（Gather）皆以精準的「幾株／孤筍」呈現。

use std::collections::HashMap;

use crate::grid::cell::{GridObject, GridObjectKind};
use crate::model::RoomObject;

pub const ID: &str = "zl_bamboo_shoot";
pub const NAME: &str = "破土竹筍";
pub const YIELD_ITEM_ID: &str = "zl_item_bamboo_shoot";
pub const YIELD_NOUN: &str = "鮮刺竹筍";
pub const KIND: GridObjectKind = GridObjectKind::Flora;
pub const CELL_TAGS: &[&str] = &["bamboo", "bamboo_shoot", "flora"];

/// 場景採集物件三層標籤（類別標籤、屬性標籤、專屬標籤）
pub const OBJECT_TAGS: super::ZhulinTagTriad = super::ZhulinTagTriad {
    category_tags: &["flora", "edible_shoot", "gather_node"],
    attribute_tags: &["subterranean_bud", "moisture_loving", "scarce", "seasonal"],
    exclusive_tag: "zl_obj_bamboo_shoot",
};

/// 採集產出物品物件定義與三層標籤（獨立專屬 ID，絕不共用舊世 harvest_berries）
pub const YIELD_ITEM_DEF: super::ZhulinItemDef = super::ZhulinItemDef {
    id: YIELD_ITEM_ID,
    name: YIELD_NOUN,
    item_type: "consumable",
    weight: 0.4,
    origin: "刺竹林",
    description: "剛從竹根濕土連根掘出的鮮嫩刺竹筍，褐籜覆絨、筍肉厚實，宜帶殼火煨或切片入饌。",
    tags: super::ZhulinTagTriad {
        category_tags: &["food", "fresh_ingredient", "bamboo_shoot"],
        attribute_tags: &["crisp_tender", "raw_astringent", "roastable", "perishable"],
        exclusive_tag: YIELD_ITEM_ID,
    },
};

/// 上方 L3 場景描述短句（區分多株與單株孤筍；歸零時變為 `[空白]` 消失）
pub fn scene_sentence(qty: u32) -> &'static str {
    match qty {
        0 => "",
        1 => "竹根旁泥土裂開，冒出一株尖頭帶絨毛的破土竹筍。",
        _ => "泥土間冒出幾株籜殼堅實的破土竹筍。",
    }
}

/// 點擊物件端詳（Look）時的自然觀察描述（依剩餘株數呈現，取代 `（物產存量：X/Y）`）
pub fn look_description(qty: u32) -> String {
    let base = "自竹根濕土鑽出的刺竹筍，褐籜覆絨、筍肉鮮嫩。";
    let state = match qty {
        0 => "竹根旁僅留翻開的濕泥坑，竹筍已被掘走。",
        1 => "泥縫間僅見這一株孤筍冒尖，掘後便無新筍。",
        2 => "泥縫間共有兩株飽滿筍尖破土，正值鮮嫩可採。",
        _ => "竹根泥土微裂，共有三株壯碩筍尖破土而出。",
    };
    format!("{base}{state}")
}

/// 執行 `[採集]` 時的專屬動作敘事（明確區分「場景物件：破土竹筍」與「採得產物：鮮刺竹筍」）
pub fn gather_narrative(harvested_count: u32, remaining_qty: u32) -> String {
    let action_part = if harvested_count > 1 {
        format!("你撥開竹根鬆土，連根掘出 {harvested_count} 株鮮嫩的「{YIELD_NOUN}」收入行囊。")
    } else {
        format!("你撥開竹根鬆土，連根掘出一株鮮嫩的「{YIELD_NOUN}」收入行囊。")
    };
    let remain_part = match remaining_qty {
        0 => "此處泥縫間的竹筍已盡數採完，僅留新土凹痕。",
        1 => "竹根旁尚留最後一株孤筍冒尖。",
        _ => "泥縫間尚有兩株飽滿竹筍可掘。",
    };
    format!("{action_part}{remain_part}")
}

/// 未持撬挖／掘土工具（`pry_edge` / `earth_dig`）且存量 >= 2 時，徒手硬掰造成折損（耗 2 株存量得 1 株）
pub fn barehand_lossy_gather_narrative(remaining_qty: u32) -> String {
    let remain_part = match remaining_qty {
        0 => "此處泥縫間的竹筍已盡數扳盡，僅留碎筍殘根。",
        1 => "竹根旁尚留最後一株孤筍冒尖。",
        _ => "泥縫間尚有飽滿竹筍可掘。",
    };
    format!(
        "你徒手硬扳濕土中的筍尖，手指被堅硬籜毛扎得生疼，筍身自中段折損，耗費兩株筍芽才勉強掰出一株「{YIELD_NOUN}」收入行囊。{remain_part}"
    )
}

/// 未持撬挖／掘土工具且僅剩最後 1 株孤筍時，擋回徒手硬掰以免掰碎斷根
pub fn blocked_lone_shoot_narrative() -> String {
    "泥縫間僅剩這一株孤筍，徒手硬掰極易將嫩筍折斷於硬土中；需藉助銳邊石片或竹鏟等撬挖利器方可完整連根掘出。".to_string()
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
