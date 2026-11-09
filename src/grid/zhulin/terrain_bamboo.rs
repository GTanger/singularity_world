//! 竹林地形格模版：【刺竹林】（`𓐂`）× 格內 3 槽位拉霸 × 生態分層存量 × 動態描述拼接
//!
//! 核心架構：
//! 1. 整片竹海由同一個【刺竹林】地形物件模版複製貼上而成。
//! 2. 每格內含 3 個採集物件槽位（Slot 1..=3），在生成時從「合抱刺竹叢、乾竹葉層、破土竹筍、`[空白]`」拉霸抽出並持久化至資料庫。
//! 3. 三種採集物件依真實生態體量分層設定初始存量：
//!    - 【乾竹葉層】（地表鋪積耗材）：8 ～ 14 把
//!    - 【合抱刺竹叢】（主體結構建材）：6 ～ 10 根
//!    - 【破土竹筍】（點狀稀有食材）：1 ～ 3 株
//! 4. 格子的場景描述由「刺竹林基底短句 ＋ 各非空白槽位物件隨存量變化的自帶短句」動態拼接；
//!    當某槽位被採集至存量 0 時，該槽位即變為 `[空白]`，對應句子立即從畫面描述中消失。

use crate::grid::cell::{GridObject, Terrain};
use crate::grid::coord::SquareCoord;
use crate::grid::grid::GridCell;
use crate::grid::zhulin::objects::{self, BambooSlotKind, bamboo_culm};

pub const BAMBOO_TERRAIN_NAME: &str = "刺竹林";
pub const BAMBOO_TERRAIN_TAG: &str = "bamboo_slot_terrain";

/// 【刺竹林】地形物件三層標籤（類別標籤、屬性標籤、專屬標籤）
pub const TERRAIN_TAGS: crate::grid::zhulin::objects::ZhulinTagTriad =
    crate::grid::zhulin::objects::ZhulinTagTriad {
        category_tags: &["terrain", "forest", "bamboo_grove"],
        attribute_tags: &["thorny_canopy", "dappled_shade", "wind_filtering", "three_slot_ecology"],
        exclusive_tag: "zl_terrain_thorny_bamboo_grove",
    };
pub const BASE_DESC_DRY: &str = "枝刺交錯的原生刺竹林，竹蔭間光影斑駁。";
pub const BASE_DESC_WATERSIDE: &str =
    "枝刺交錯的原生刺竹林，受鄰近泉溪浸潤，竹蔭下泥土濕涼。";

/// 確定性雜湊輔助：將座標與槽位序號映射為 `u64` 亂數種子
fn slot_hash(x: i32, y: i32, slot_idx: u64, seed: u64) -> u64 {
    let mut h = seed
        ^ ((x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        ^ ((y as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        ^ slot_idx.wrapping_mul(0x94D0_49BB_1331_11EB);
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

/// 為單一【刺竹林】地形格執行 3 槽位拉霸抽籤（含 `[空白]` 與分層生態存量）
pub fn roll_bamboo_three_slots(
    coord: SquareCoord,
    is_waterside: bool,
    world_seed: u64,
) -> [BambooSlotKind; 3] {
    // 原點周圍示範格：確保玩家從 (0,0) 往外走一步即可體驗 1～3 槽位、分層存量與 [空白] 的差異
    match (coord.x, coord.y) {
        // (-1, 0)：抽中【合抱刺竹叢 (8根)】＋【乾竹葉層 (11把)】＋【盤錯竹鞭 (3段)】（3 個物件）
        (-1, 0) => {
            return [
                BambooSlotKind::BambooCulm(8),
                BambooSlotKind::DryLeaves(11),
                BambooSlotKind::BambooRhizome(3),
            ];
        }
        // (1, 0)：抽中【枯黃竹籜 (5片)】＋【竹根木耳 (2朵)】＋【破土竹筍 (1株孤筍)】（3 個物件）
        (1, 0) => {
            return [
                BambooSlotKind::BambooSheath(5),
                BambooSlotKind::BambooFungus(2),
                BambooSlotKind::BambooShoot(1),
            ];
        }
        // (-1, 1)：抽中【乾竹葉層 (10把)】＋ [空白] ＋【竹根木耳 (1朵)】（2 個物件）
        (-1, 1) => {
            return [
                BambooSlotKind::DryLeaves(10),
                BambooSlotKind::Empty,
                BambooSlotKind::BambooFungus(1),
            ];
        }
        // (1, 1) 鄰近冷泉：抽中【合抱刺竹叢 (9根)】＋【盤錯竹鞭 (2段)】＋【破土竹筍 (2株)】（3 槽全中！）
        (1, 1) => {
            return [
                BambooSlotKind::BambooCulm(9),
                BambooSlotKind::BambooRhizome(2),
                BambooSlotKind::BambooShoot(2),
            ];
        }
        // (-1, -1)：抽中【合抱刺竹叢 (6根)】＋ [空白] ＋ [空白]（1 個物件）
        (-1, -1) => {
            return [
                BambooSlotKind::BambooCulm(6),
                BambooSlotKind::Empty,
                BambooSlotKind::Empty,
            ];
        }
        _ => {}
    }

    let h0 = slot_hash(coord.x, coord.y, 0, world_seed);
    let h1 = slot_hash(coord.x, coord.y, 1, world_seed);
    let h2 = slot_hash(coord.x, coord.y, 2, world_seed);

    // 槽位 1（主體／地表保底槽）：55% 合抱刺竹叢（6..=10 根）、30% 乾竹葉層（8..=14 把）、15% 枯黃竹籜（4..=7 片）
    let roll0 = h0 % 100;
    let slot1 = if roll0 < 55 {
        let culm_qty = 6 + ((h0 >> 8) % 5) as u32;
        BambooSlotKind::BambooCulm(culm_qty)
    } else if roll0 < 85 {
        let leaf_qty = 8 + ((h0 >> 8) % 7) as u32;
        BambooSlotKind::DryLeaves(leaf_qty)
    } else {
        let sheath_qty = 4 + ((h0 >> 8) % 4) as u32;
        BambooSlotKind::BambooSheath(sheath_qty)
    };

    // 槽位 2（中層伴生拉霸槽）：35% [空白]、25% 乾竹葉層（8..=13 把）、20% 枯黃竹籜（4..=7 片）、20% 盤錯竹鞭（2..=4 段）
    let roll1 = h1 % 100;
    let candidate2 = if roll1 < 35 {
        BambooSlotKind::Empty
    } else if roll1 < 60 {
        let leaf_qty = 8 + ((h1 >> 8) % 6) as u32;
        BambooSlotKind::DryLeaves(leaf_qty)
    } else if roll1 < 80 {
        let sheath_qty = 4 + ((h1 >> 8) % 4) as u32;
        BambooSlotKind::BambooSheath(sheath_qty)
    } else {
        let rhizome_qty = 2 + ((h1 >> 8) % 3) as u32;
        BambooSlotKind::BambooRhizome(rhizome_qty)
    };
    let slot2 = match (&slot1, &candidate2) {
        (BambooSlotKind::BambooCulm(_), BambooSlotKind::BambooCulm(_))
        | (BambooSlotKind::DryLeaves(_), BambooSlotKind::DryLeaves(_))
        | (BambooSlotKind::BambooSheath(_), BambooSlotKind::BambooSheath(_)) => {
            BambooSlotKind::Empty
        }
        _ => candidate2,
    };

    // 槽位 3（底層尋寶拉霸槽）：抽 [空白]、【破土竹筍】（1..=3 株）或【竹根木耳】（1..=2 朵）
    let roll2 = h2 % 100;
    let sub_roll = (h2 >> 8) % 100;
    let slot3 = if is_waterside {
        if roll2 < 32 {
            let shoot_qty = if sub_roll < 50 { 1 } else if sub_roll < 85 { 2 } else { 3 };
            BambooSlotKind::BambooShoot(shoot_qty)
        } else if roll2 < 58 {
            let fungus_qty = if sub_roll < 60 { 1 } else { 2 };
            BambooSlotKind::BambooFungus(fungus_qty)
        } else {
            BambooSlotKind::Empty
        }
    } else if roll2 < 24 {
        let shoot_qty = if sub_roll < 55 { 1 } else if sub_roll < 88 { 2 } else { 3 };
        BambooSlotKind::BambooShoot(shoot_qty)
    } else if roll2 < 34 {
        BambooSlotKind::BambooFungus(1)
    } else {
        BambooSlotKind::Empty
    };

    [slot1, slot2, slot3]
}

/// 根據格內當前剩餘存量 > 0 的槽位物件，動態拼出【刺竹林】地形格的場景描述
pub fn compose_bamboo_description_from_objects(
    is_waterside: bool,
    objects: &[GridObject],
) -> String {
    let mut desc = String::from(if is_waterside {
        BASE_DESC_WATERSIDE
    } else {
        BASE_DESC_DRY
    });

    for obj in objects {
        if let Some(sentence) = objects::scene_sentence_for_object(obj) {
            desc.push_str(sentence);
        }
    }
    desc
}

/// 若該格為【刺竹林】槽位地形格，依其當前非空白槽位（`quantity > 0`）動態重組描述
pub fn compose_bamboo_description(cell: &GridCell) -> Option<String> {
    if !is_bamboo_slot_cell(cell) {
        return None;
    }
    let is_waterside = cell.has_room_tag("waterside");
    Some(compose_bamboo_description_from_objects(
        is_waterside,
        &cell.objects,
    ))
}

/// 檢查格子是否為採用 3 槽位拉霸機制的【刺竹林】地形格
pub fn is_bamboo_slot_cell(cell: &GridCell) -> bool {
    cell.name == BAMBOO_TERRAIN_NAME || cell.has_room_tag(BAMBOO_TERRAIN_TAG)
}

/// 建立單一【刺竹林】（`𓐂`）地形格實例：套用統一地形模版、拉霸抽出 3 槽位並動態拼接初始描述
pub fn build_bamboo_terrain_cell(
    coord: SquareCoord,
    zone: &str,
    is_waterside: bool,
    world_seed: u64,
) -> GridCell {
    let slots = roll_bamboo_three_slots(coord, is_waterside, world_seed);
    let cell_objects: Vec<GridObject> = slots
        .into_iter()
        .filter_map(BambooSlotKind::into_grid_object)
        .collect();

    let has_culm = cell_objects.iter().any(|o| o.id == bamboo_culm::ID);
    let terrain = if has_culm {
        Terrain::ForestHeavy
    } else {
        Terrain::Forest
    };

    let mut cell = GridCell::new(coord, terrain, BAMBOO_TERRAIN_NAME);
    cell.zone = zone.to_string();
    cell.explored = true;

    let mut tags = vec![
        "bamboo".to_string(),
        "flora".to_string(),
        BAMBOO_TERRAIN_TAG.to_string(),
    ];
    if is_waterside {
        tags.push("waterside".to_string());
    }
    for obj in &cell_objects {
        if obj.quantity > 0 {
            for &t in objects::cell_tags_for_object_id(&obj.id) {
                if !tags.iter().any(|existing| existing == t) {
                    tags.push(t.to_string());
                }
            }
        }
    }
    for derived in crate::grid::derive_room_tags(terrain, BAMBOO_TERRAIN_NAME, zone) {
        if !tags.iter().any(|t| t == &derived) {
            tags.push(derived);
        }
    }

    cell.description = compose_bamboo_description_from_objects(is_waterside, &cell_objects);
    cell.tags = tags;
    cell.objects = cell_objects;
    cell
}

/// 當竹林格內的槽位被採集或自然復生後，同步更新該格的 `description` 與 `tags`
pub fn refresh_bamboo_cell_after_gather(cell: &mut GridCell) {
    if !is_bamboo_slot_cell(cell) {
        return;
    }
    let is_waterside = cell.has_room_tag("waterside");
    cell.description = compose_bamboo_description_from_objects(is_waterside, &cell.objects);

    let mut tags = vec![
        "bamboo".to_string(),
        "flora".to_string(),
        BAMBOO_TERRAIN_TAG.to_string(),
    ];
    if is_waterside {
        tags.push("waterside".to_string());
    }
    for obj in &cell.objects {
        if obj.quantity > 0 {
            for &t in objects::cell_tags_for_object_id(&obj.id) {
                if !tags.iter().any(|existing| existing == t) {
                    tags.push(t.to_string());
                }
            }
        }
    }
    for derived in crate::grid::derive_room_tags(cell.terrain, &cell.name, &cell.zone) {
        if !tags.iter().any(|t| t == &derived) {
            tags.push(derived);
        }
    }
    cell.tags = tags;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::zhulin::objects::{
        bamboo_culm, bamboo_fungus, bamboo_rhizome, bamboo_sheath, bamboo_shoot, dry_leaves,
        resolve_bamboo_slot_object_with_qty,
    };

    #[test]
    fn bamboo_cell_stratifies_ecological_stock_and_uses_diegetic_descriptions() {
        let cell_west = build_bamboo_terrain_cell(
            SquareCoord::new(-1, 0),
            "紅土西麓",
            true,
            2026_11_09,
        );
        let culm = cell_west
            .objects
            .iter()
            .find(|o| o.id == bamboo_culm::ID)
            .unwrap();
        let leaves = cell_west
            .objects
            .iter()
            .find(|o| o.id == dry_leaves::ID)
            .unwrap();
        let rhizome = cell_west
            .objects
            .iter()
            .find(|o| o.id == bamboo_rhizome::ID)
            .unwrap();
        // 生態分層：乾竹葉層 (8..=14) > 合抱刺竹叢 (6..=10) > 盤錯竹鞭 (2..=4)
        assert!((6..=10).contains(&culm.quantity));
        assert!((8..=14).contains(&leaves.quantity));
        assert!((2..=4).contains(&rhizome.quantity));

        // 端詳描述絕不含「物產存量」括號分數，而是使用自然量詞描寫
        let culm_room_obj =
            resolve_bamboo_slot_object_with_qty(bamboo_culm::ID, culm.quantity).unwrap();
        let look_text = &culm_room_obj.responses["Look"];
        assert!(!look_text.contains("物產存量"));
        assert!(look_text.contains("成熟刺竹稈尚有十來根"));

        let rhizome_look =
            resolve_bamboo_slot_object_with_qty(bamboo_rhizome::ID, rhizome.quantity).unwrap();
        assert!(rhizome_look.responses["Look"].contains("老竹鞭盤結"));
        assert!(!rhizome_look.responses["Look"].contains("物產存量"));

        let mut cell_east = build_bamboo_terrain_cell(
            SquareCoord::new(1, 0),
            "紅土西麓",
            false,
            2026_11_09,
        );
        let sheath = cell_east
            .objects
            .iter()
            .find(|o| o.id == bamboo_sheath::ID)
            .expect("sheath slot present");
        assert!((4..=7).contains(&sheath.quantity));
        let fungus = cell_east
            .objects
            .iter()
            .find(|o| o.id == bamboo_fungus::ID)
            .expect("fungus slot present");
        assert!((1..=2).contains(&fungus.quantity));

        let shoot = cell_east
            .objects
            .iter_mut()
            .find(|o| o.id == bamboo_shoot::ID)
            .expect("shoot slot present");
        assert_eq!(shoot.quantity, 1);
        let shoot_look =
            resolve_bamboo_slot_object_with_qty(bamboo_shoot::ID, shoot.quantity).unwrap();
        assert!(shoot_look.responses["Look"].contains("僅見這一株孤筍"));
        assert!(!shoot_look.responses["Look"].contains("物產存量"));

        // 採空歸零後句子消失
        shoot.quantity = 0;
        refresh_bamboo_cell_after_gather(&mut cell_east);
        assert!(!cell_east.description.contains("破土竹筍"));
        assert!(cell_east.description.contains("枯黃竹籜"));
        assert!(cell_east.description.contains("竹根木耳"));
    }

    #[test]
    fn bamboo_objects_and_yield_items_have_unique_ids_and_three_tier_tags() {
        use std::collections::HashSet;
        use crate::grid::zhulin::{
            ALL_ZHULIN_ITEMS, TERRAIN_TAGS, lookup_zhulin_item_def, lookup_zhulin_object_tags,
            yield_item_id_for_object,
        };

        assert!(!TERRAIN_TAGS.category_tags.is_empty());
        assert!(!TERRAIN_TAGS.attribute_tags.is_empty());
        assert_eq!(TERRAIN_TAGS.exclusive_tag, "zl_terrain_thorny_bamboo_grove");

        let object_ids = [
            bamboo_culm::ID,
            dry_leaves::ID,
            bamboo_shoot::ID,
            bamboo_rhizome::ID,
            bamboo_sheath::ID,
            bamboo_fungus::ID,
        ];
        let mut seen_obj_exclusive = HashSet::new();
        let mut seen_item_ids = HashSet::new();

        for obj_id in object_ids {
            let obj_tags = lookup_zhulin_object_tags(obj_id).expect("object tags must exist");
            assert!(!obj_tags.category_tags.is_empty());
            assert!(!obj_tags.attribute_tags.is_empty());
            assert!(seen_obj_exclusive.insert(obj_tags.exclusive_tag));

            let item_id = yield_item_id_for_object(obj_id).expect("yield item id must exist");
            assert!(
                !item_id.starts_with("harvest_")
                    && !item_id.starts_with("wt_")
                    && !item_id.starts_with("mat_"),
                "must not reuse legacy world item id: {item_id}"
            );
            assert!(
                seen_item_ids.insert(item_id),
                "each bamboo object must yield a distinct item id: {item_id}"
            );

            let item_def = lookup_zhulin_item_def(item_id).expect("item def must exist");
            assert_eq!(item_def.origin, "刺竹林");
            assert!(!item_def.tags.category_tags.is_empty());
            assert!(!item_def.tags.attribute_tags.is_empty());
            assert_eq!(item_def.tags.exclusive_tag, item_id);

            let brief = crate::db::get_item_def(item_id).expect("db::get_item_def must resolve");
            assert_eq!(brief.name, item_def.name);
            assert_eq!(brief.origin, "刺竹林");
        }
        assert_eq!(ALL_ZHULIN_ITEMS.len(), 16);
    }
}
