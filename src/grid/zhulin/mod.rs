//! 沙鹿竹林模組（`src/grid/zhulin/`）
//!
//! 竹林 6 物件生態閉環：
//! - 1 個地形物件模版：`terrain_bamboo`（【刺竹林】`𓐂`）
//! - 6 個獨立採集物件檔案：
//!   1. `objects::bamboo_culm`（【合抱刺竹叢】`6..=10` 根）
//!   2. `objects::dry_leaves`（【乾竹葉層】`8..=14` 把）
//!   3. `objects::bamboo_shoot`（【破土竹筍】`1..=3` 株）
//!   4. `objects::bamboo_rhizome`（【盤錯竹鞭】`2..=4` 段）
//!   5. `objects::bamboo_sheath`（【枯黃竹籜】`4..=7` 片）
//!   6. `objects::bamboo_fungus`（【竹根木耳】`1..=2` 朵）
//! - 格內 3 槽位拉霸（含 `[空白]`）＋ 生態分層存量 ＋ 自然豐枯端詳與採集敘事

pub mod objects;
pub mod terrain_bamboo;
pub mod weather;

pub use objects::{
    ALL_ZHULIN_ITEMS, BambooSlotKind, GatherCheckResult, ITEM_AGED_BAMBOO_SLAT,
    ITEM_BAMBOO_BASKET, ITEM_BAMBOO_CANTEEN, ITEM_BAMBOO_CANTEEN_FULL, ITEM_BAMBOO_SPADE,
    ITEM_BAMBOO_STONE_AXE, ITEM_BAMBOO_TUBE_SOUP, ITEM_BAMBOO_WEEVIL,
    ITEM_FIRE_HARDENED_BAMBOO_KNIFE, ITEM_RHIZOME_CLUB, ITEM_ROASTED_BAMBOO_SHOOT,
    ITEM_ROASTED_WEEVIL, ITEM_SHARP_STONE_FLAKE, ITEM_SHEATH_CORD, ITEM_TERMITE_MUSHROOM,
    ZhulinItemDef, ZhulinTagTriad, append_diegetic_stock_clause, bamboo_culm, bamboo_fungus,
    bamboo_rhizome, bamboo_sheath, bamboo_shoot, classify_zhulin_item_capabilities, dry_leaves,
    evaluate_fire_weather_shelter, evaluate_zhulin_gather, evaluate_zhulin_gather_with_weather,
    gather_narrative_for_object, inventory_has_capability_tag, lookup_zhulin_item_def,
    lookup_zhulin_object_tags, resolve_bamboo_slot_object, resolve_bamboo_slot_object_with_qty,
    scene_sentence_for_object, try_craft_zhulin_item, yield_item_id_for_object,
};
pub use terrain_bamboo::{
    BAMBOO_TERRAIN_NAME, BAMBOO_TERRAIN_TAG, TERRAIN_TAGS, build_bamboo_terrain_cell,
    carve_bamboo_trail_mark, compose_bamboo_description, explore_bamboo_understory,
    get_bamboo_cell_pressure, has_bamboo_trail_mark, is_bamboo_slot_cell, is_bamboo_trail_cleared,
    lazy_refresh_bamboo_cell, record_bamboo_gather_pressure, refresh_bamboo_cell_after_gather,
    roll_bamboo_three_slots, set_bamboo_cell_meta_tag,
};
pub use weather::{
    ALL_ZHULIN_WEATHERS, ZhulinWeatherDef, active_zhulin_weather_for_hour,
    current_world_zhulin_weather, lookup_zhulin_weather, next_zhulin_weather_in_cycle,
};
