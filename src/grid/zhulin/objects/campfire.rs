//! 竹林系列動態插件兼插座：【竹葉營火】（`zl_bamboo_campfire`）
//!
//! 架構定位：「萬事萬物皆插件兼插座」示範實體
//! - 作為插件（Plugin）：引燃乾竹葉後動態掛載入當前格 `cell.objects` 槽位，為所在格注入火源、溫養與驅獸屬性；露天遇雨（`wetting` 且無立竹遮蔭）時自動澆熄卸載。
//! - 作為主畫面插座（Socket）：直接顯示於主畫面中段（`#sw-l4-objects`），玩家點擊即可原地執行烘火溫養、烘烤竹刃、籜裹煨筍、炭火炙烤、竹筒煲湯與投火竹爆，免開行囊彈窗。

use std::collections::HashMap;

use crate::grid::cell::{GridObject, GridObjectKind};
use crate::model::RoomObject;

pub const ID: &str = "zl_bamboo_campfire";
pub const NAME: &str = "竹葉營火";
pub const KIND: GridObjectKind = GridObjectKind::Other;
pub const CELL_TAGS: &[&str] = &["campfire", "fire_source", "warmth"];

/// 場景插件三層正交標籤（類別標籤、屬性標籤、專屬主標籤）
pub const OBJECT_TAGS: super::ZhulinTagTriad = super::ZhulinTagTriad {
    category_tags: &["camp_structure", "heat_source", "socket_host"],
    attribute_tags: &["open_flame", "ember_bed", "warmth_aura", "beast_deterrent"],
    exclusive_tag: "zl_obj_bamboo_campfire",
};

pub fn scene_sentence(qty: u32) -> &'static str {
    match qty {
        0 => "",
        1 => "林下留著一簇微紅溫熱的竹葉炭火餘燼。",
        _ => "林下燃著一簇劈啪作響的竹葉營火。",
    }
}

pub fn look_description(qty: u32) -> String {
    match qty {
        0 => "地上的竹葉營火已然熄滅，只餘幾撮微溫的白灰。".to_string(),
        1 => "乾燥竹葉與枯枝聚攏而成的營火已化作暗紅炭床，熱力勻稱內斂，正適合慢煨竹筍、烘烤竹刃或湊近烘手回氣。".to_string(),
        _ => "乾燥刺竹葉與枯枝交錯燃起的竹葉營火正劈啪作響，金紅火苗躍動間散發著竹青清香與融融暖意，可直接在此烘烤竹片刀、煨筍、煲湯或投竹爆響。".to_string(),
    }
}

pub fn build_grid_object(qty: u32) -> GridObject {
    let q = qty.max(1);
    GridObject {
        id: ID.to_string(),
        kind: KIND,
        name: NAME.to_string(),
        quantity: q,
        max_quantity: 5,
        regrowth_per_tick: 0,
    }
}

pub fn to_room_object(qty: u32) -> RoomObject {
    let sockets = if qty > 0 {
        vec![
            "Look".to_string(),
            "WarmHands".to_string(),
            "CampfireRoastKnife".to_string(),
            "CampfireRoastShoot".to_string(),
            "CampfireRoastFungus".to_string(),
            "CampfireBoilSoup".to_string(),
            "CampfirePopCulm".to_string(),
        ]
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
