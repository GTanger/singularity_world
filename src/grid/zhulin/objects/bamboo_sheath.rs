//! 竹林系列採集物件 5：【枯黃竹籜】（`zl_bamboo_sheath`）
//!
//! 生態定位：成竹脫落老筍殼、天然防潮包裹與編材（Natural Waterproof Wrapping & Fiber）
//! - 初始存量：4 ～ 7 片厚實竹籜
//! - 自然再生：`regrowth_per_tick = 1`

use std::collections::HashMap;

use crate::grid::cell::{GridObject, GridObjectKind};
use crate::model::RoomObject;

pub const ID: &str = "zl_bamboo_sheath";
pub const NAME: &str = "枯黃竹籜";
pub const YIELD_ITEM_ID: &str = "zl_item_bamboo_sheath";
pub const YIELD_NOUN: &str = "韌竹籜";
pub const KIND: GridObjectKind = GridObjectKind::Flora;
pub const CELL_TAGS: &[&str] = &["bamboo", "sheath", "fiber", "flora"];

/// 場景採集物件三層標籤（類別標籤、屬性標籤、專屬標籤）
pub const OBJECT_TAGS: super::ZhulinTagTriad = super::ZhulinTagTriad {
    category_tags: &["flora", "shed_sheath", "gather_node"],
    attribute_tags: &["broad_husk", "water_repellent", "tough_fiber", "ground_shed"],
    exclusive_tag: "zl_obj_bamboo_sheath",
};

/// 採集產出物品物件定義與三層標籤（獨立專屬 ID，絕不共用舊世 harvest_vine）
pub const YIELD_ITEM_DEF: super::ZhulinItemDef = super::ZhulinItemDef {
    id: YIELD_ITEM_ID,
    name: YIELD_NOUN,
    item_type: "material",
    weight: 0.15,
    origin: "刺竹林",
    description: "成竹脫落的寬厚老筍殼，外覆斑駁褐紋、內裡光滑防潮，可包裹米食、襯墊斗笠或搓製籜繩。",
    tags: super::ZhulinTagTriad {
        category_tags: &["wrapping_sheet", "weaving_fiber", "bamboo_sheath"],
        attribute_tags: &["waterproof", "broad_sheet", "pliable_when_wet", "tear_resistant"],
        exclusive_tag: YIELD_ITEM_ID,
    },
};

pub fn scene_sentence(qty: u32) -> &'static str {
    match qty {
        0 => "",
        1..=2 => "竹叢根部散落著一兩片邊緣捲曲的枯黃竹籜。",
        _ => "竹叢根部斜倚著數片寬大厚實、帶斑駁紋理的枯黃竹籜。",
    }
}

pub fn look_description(qty: u32) -> String {
    let base = "成竹脫落的寬大老筍殼，外覆褐斑、內裡光滑防潮。";
    let state = match qty {
        0 => "完好的寬大竹籜已拾取一空，僅留碎裂籜屑。",
        1..=2 => "竹根旁僅剩一兩片完好竹籜尚可拾取。",
        _ => "竹根旁散落數片完整竹籜，可作包裹編材。",
    };
    format!("{base}{state}")
}

pub fn gather_narrative(harvested_count: u32, remaining_qty: u32) -> String {
    let action_part = if harvested_count > 1 {
        format!("你揀選完好筍殼，拾起 {harvested_count} 片防潮的「{YIELD_NOUN}」收入行囊。")
    } else {
        format!("你自竹根旁拾起一片寬厚防潮的「{YIELD_NOUN}」收入行囊。")
    };
    let remain_part = match remaining_qty {
        0 => "此處完好的竹籜已悉數拾盡。",
        1..=2 => "竹根旁僅餘一兩片竹籜散落。",
        _ => "竹根旁尚有數片完好竹籜可拾。",
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
