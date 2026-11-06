// 自動由《白塔半島千物總典》編譯生成之 1,120 件世界物件常數庫。
// 每一件物件皆具備明確的產地原料、百工工坊、日常用途、場所分佈與輕操作互動插座。

use std::collections::HashMap;
use crate::model::RoomObject;

#[derive(Debug, Clone, Copy)]
pub struct PeninsulaCatalogEntry {
    pub id: &'static str,
    pub object_tag: &'static str,
    pub category_num: &'static str,
    pub category_tag: &'static str,
    pub attribute_tags: &'static [&'static str],
    pub category_name: &'static str,
    pub name: &'static str,
    pub origin_material: &'static str,
    pub workshop: &'static str,
    pub daily_function: &'static str,
    pub room_tag: &'static str,
    pub socket: &'static str,
    pub look_text: &'static str,
    pub action_text: &'static str,
}

pub type WorldObjectEntry = PeninsulaCatalogEntry;


include!("peninsula_catalog/part_01.rs");
include!("peninsula_catalog/part_02.rs");
include!("peninsula_catalog/part_03.rs");
include!("peninsula_catalog/part_04.rs");
include!("peninsula_catalog/part_05.rs");
include!("peninsula_catalog/part_06.rs");
include!("peninsula_catalog/part_07.rs");
include!("peninsula_catalog/part_08.rs");
include!("peninsula_catalog/part_09.rs");
include!("peninsula_catalog/part_10.rs");
include!("peninsula_catalog/part_11.rs");
include!("peninsula_catalog/part_12.rs");
include!("peninsula_catalog/part_13.rs");
include!("peninsula_catalog/part_14.rs");
include!("peninsula_catalog/part_15.rs");
include!("peninsula_catalog/part_16.rs");
include!("peninsula_catalog/part_17.rs");
include!("peninsula_catalog/part_18.rs");
include!("peninsula_catalog/part_19.rs");
include!("peninsula_catalog/part_20.rs");

const TOTAL_COUNT: usize = 1109;

const fn concat_catalog() -> [PeninsulaCatalogEntry; TOTAL_COUNT] {
    let mut result = [PART_01[0]; TOTAL_COUNT];
    let parts: [&[PeninsulaCatalogEntry]; 20] = [
        PART_01, PART_02, PART_03, PART_04, PART_05,
        PART_06, PART_07, PART_08, PART_09, PART_10,
        PART_11, PART_12, PART_13, PART_14, PART_15,
        PART_16, PART_17, PART_18, PART_19, PART_20,
    ];
    let mut out_idx = 0;
    let mut part_idx = 0;
    while part_idx < parts.len() {
        let part = parts[part_idx];
        let mut in_idx = 0;
        while in_idx < part.len() {
            result[out_idx] = part[in_idx];
            out_idx += 1;
            in_idx += 1;
        }
        part_idx += 1;
    }
    result
}

static ALL_PENINSULA_WORLD_OBJECTS: [PeninsulaCatalogEntry; TOTAL_COUNT] = concat_catalog();
pub const PENINSULA_WORLD_OBJECTS: &[PeninsulaCatalogEntry] = &ALL_PENINSULA_WORLD_OBJECTS;

include!("peninsula_catalog/methods.rs");
include!("peninsula_catalog/tests.rs");
