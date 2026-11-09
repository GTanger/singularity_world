//! Grid 格上物件與地形型別。
//!
//! 原先定義於 `src/hex/cell.rs`，隨觀景窗 hex 概念下線，搬回方格模組根處。

use serde::{Deserialize, Serialize};

// ─── 探索新鮮度 ──────────────────────────────────────────────────────

/// 格子探索新鮮度布林。
///
/// 同一玩家同一格連續 explore：`Fresh → Half → Empty`。
/// 玩家離格後下一個 simulation tick 回 `Fresh`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// 正常（預設）
    #[default]
    Fresh,
    /// 採集量 ×0.5
    Half,
    /// 強制 outcome: "empty"
    Empty,
}

impl Freshness {
    /// 連續探索時依序推進（Empty → Empty，不再往下）。
    pub fn advance(self) -> Self {
        match self {
            Freshness::Fresh => Freshness::Half,
            Freshness::Half => Freshness::Empty,
            Freshness::Empty => Freshness::Empty,
        }
    }

    /// 序列化用字串標籤（前端 freshness_after 欄位）。
    pub fn as_str(self) -> &'static str {
        match self {
            Freshness::Fresh => "fresh",
            Freshness::Half => "half",
            Freshness::Empty => "empty",
        }
    }
}

// ─── 格上物件型別 ────────────────────────────────────────────────────

/// 格子上可採集物件的種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridObjectKind {
    /// 植物性資源（野果、草藥、木材等）
    Flora,
    /// 礦物性資源（礦石、寶石等）
    Mineral,
    /// 水源（淡水、溫泉等）
    WaterSource,
    /// 其他（遺跡、碎片等）
    Other,
}

impl GridObjectKind {
    /// 格上物件種類的原型資源標籤（resource category tag）。
    pub fn category_tag(self) -> &'static str {
        match self {
            GridObjectKind::Flora => "flora_resource",
            GridObjectKind::Mineral => "mineral_resource",
            GridObjectKind::WaterSource => "water_source",
            GridObjectKind::Other => "wilderness_feature",
        }
    }
}

/// 格子上的可採集物件。
///
/// 與 `model::RoomObject` 不同：此結構帶資源量與回復速率，
/// 用於 NPC 採集與資源生態模擬。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridObject {
    /// 物件唯一識別碼（通常為 `"{種類}_{座標}"`）
    pub id: String,
    pub kind: GridObjectKind,
    pub name: String,
    /// 目前剩餘量
    pub quantity: u32,
    /// 上限量（採滿後不再增長）
    pub max_quantity: u32,
    /// 每 tick 自然回復量（0 = 不回復）
    pub regrowth_per_tick: u32,
}

impl GridObject {
    /// 格上物件的專屬英文規範標籤（dedicated English snake_case tag）。
    pub fn tag(&self) -> &'static str {
        self.tag_and_archetype().0
    }

    /// 格上物件的分類標籤（category tag）。
    pub fn category_tag(&self) -> &'static str {
        if let Some(spec) = find_grid_object_spec(&self.id, &self.name) {
            return spec.category_tag;
        }
        match self.kind {
            GridObjectKind::Mineral => "mineral_jade",
            GridObjectKind::WaterSource => "plumbing_bath_hygiene",
            GridObjectKind::Flora => {
                let arch = self.archetype_tag();
                let tag = self.tag();
                if arch == "firewood_pile"
                    || tag.contains("wood")
                    || tag.contains("pine")
                    || tag.contains("bamboo")
                {
                    "timber_bamboo_fungi"
                } else {
                    "herbs_flora_spices"
                }
            }
            GridObjectKind::Other => match self.archetype_tag() {
                "bench" | "bollard" | "stele" | "railing" | "notice_board" | "way_stele" => "public_fixture",
                _ => "maritime_transport_outdoor",
            },
        }
    }

    /// 格上物件的功能原型標籤（functional archetype tag）。
    pub fn archetype_tag(&self) -> &'static str {
        self.tag_and_archetype().1
    }

    /// 格上物件的屬性標籤清單（attribute tags）。
    pub fn attribute_tags(&self) -> &'static [&'static str] {
        if let Some(spec) = find_grid_object_spec(&self.id, &self.name) {
            spec.attribute_tags
        } else {
            match self.kind {
                GridObjectKind::Mineral => &["raw_material", "mineral", "ore_and_stone", "gatherable"],
                GridObjectKind::WaterSource => &["facility", "water_source", "water_supply", "fresh_drink"],
                GridObjectKind::Flora => &["raw_material", "flora", "gatherable"],
                GridObjectKind::Other => &["facility", "outdoor_and_maritime", "wilderness_feature"],
            }
        }
    }

    /// 取得格上物件的所有語意標籤清單（包含專屬 tag、category tag、kind category tag、archetype tag、attribute tags，去重）。
    pub fn tags(&self) -> Vec<&'static str> {
        let mut result = Vec::with_capacity(8);
        let base_candidates = [
            self.tag(),
            self.category_tag(),
            self.kind.category_tag(),
            self.archetype_tag(),
        ];
        for tag in base_candidates {
            if !tag.is_empty() && !result.contains(&tag) {
                result.push(tag);
            }
        }
        for &attr in self.attribute_tags() {
            if !attr.is_empty() && !result.contains(&attr) {
                result.push(attr);
            }
        }
        result
    }

    /// 檢查格上物件是否具備指定標籤（比對 tags 清單）。
    pub fn has_tag(&self, wanted: &str) -> bool {
        self.tags().contains(&wanted)
    }

    fn tag_and_archetype(&self) -> (&'static str, &'static str) {
        grid_object_tag_and_archetype(&self.id, &self.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridObjectTagSpec {
    pub pattern: &'static str,
    pub name_alias: &'static str,
    pub object_tag: &'static str,
    pub category_tag: &'static str,
    pub archetype_tag: &'static str,
    pub attribute_tags: &'static [&'static str],
}

pub const GRID_OBJECT_TAG_SPECS: &[GridObjectTagSpec] = &[
    GridObjectTagSpec {
        pattern: "tracks",
        name_alias: "獸徑足印",
        object_tag: "beast_tracks",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "beast_tracks",
        attribute_tags: &["fixed", "outdoor_gear", "fauna", "hunting", "wild"],
    },
    GridObjectTagSpec {
        pattern: "niche",
        name_alias: "避風岩龕",
        object_tag: "rock_niche",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "rock_niche",
        attribute_tags: &["fixed", "facility", "stone", "rest_spot", "wind_shelter", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "cairn_niche",
        name_alias: "石壘岩龕",
        object_tag: "rock_niche",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "rock_niche",
        attribute_tags: &["fixed", "facility", "stone", "rest_spot", "wind_shelter", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "cape_cairn",
        name_alias: "海角石壘",
        object_tag: "rock_niche",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "rock_niche",
        attribute_tags: &["fixed", "facility", "stone", "rest_spot", "sailing", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "snow_cairn",
        name_alias: "雪線石壘",
        object_tag: "rock_niche",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "rock_niche",
        attribute_tags: &["fixed", "facility", "stone", "rest_spot", "wind_shelter", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "forest_sign",
        name_alias: "林間避風石",
        object_tag: "rock_niche",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "rock_niche",
        attribute_tags: &["fixed", "facility", "stone", "rest_spot", "wind_shelter", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "spring",
        name_alias: "山澗清泉眼",
        object_tag: "mountain_spring",
        category_tag: "plumbing_bath_hygiene",
        archetype_tag: "mountain_spring",
        attribute_tags: &["fixed", "facility", "water_source", "water_supply", "cooling", "healing"],
    },
    GridObjectTagSpec {
        pattern: "stepping_logs",
        name_alias: "墊步木墩",
        object_tag: "stepping_logs",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "stepping_logs",
        attribute_tags: &["fixed", "facility", "timber", "footing", "marsh", "outdoor_fixture"],
    },
    GridObjectTagSpec {
        pattern: "marsh_post",
        name_alias: "沼澤踏木",
        object_tag: "stepping_logs",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "stepping_logs",
        attribute_tags: &["fixed", "facility", "timber", "footing", "marsh", "outdoor_fixture"],
    },
    GridObjectTagSpec {
        pattern: "tide_step",
        name_alias: "潮間墊步",
        object_tag: "stepping_logs",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "stepping_logs",
        attribute_tags: &["fixed", "facility", "timber", "footing", "marine", "outdoor_fixture"],
    },
    GridObjectTagSpec {
        pattern: "waymark",
        name_alias: "指路石碑",
        object_tag: "way_stele",
        category_tag: "public_fixture",
        archetype_tag: "stele",
        attribute_tags: &["fixed", "outdoor_fixture", "stone", "wayfinding", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "signpost",
        name_alias: "界標石碑",
        object_tag: "way_stele",
        category_tag: "public_fixture",
        archetype_tag: "stele",
        attribute_tags: &["fixed", "outdoor_fixture", "stone", "wayfinding", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "stele",
        name_alias: "道路里程碑",
        object_tag: "way_stele",
        category_tag: "public_fixture",
        archetype_tag: "stele",
        attribute_tags: &["fixed", "outdoor_fixture", "stone", "wayfinding", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "snow_cairn_mark",
        name_alias: "雪路程碑",
        object_tag: "way_stele",
        category_tag: "public_fixture",
        archetype_tag: "stele",
        attribute_tags: &["fixed", "outdoor_fixture", "stone", "wayfinding", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "pass_sign",
        name_alias: "隘口指路碑",
        object_tag: "way_stele",
        category_tag: "public_fixture",
        archetype_tag: "stele",
        attribute_tags: &["fixed", "outdoor_fixture", "stone", "wayfinding", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "boundary_stone",
        name_alias: "林界石碑",
        object_tag: "forest_boundary_stone",
        category_tag: "public_fixture",
        archetype_tag: "stele",
        attribute_tags: &["fixed", "outdoor_fixture", "stone", "boundary", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "campfire",
        name_alias: "野地營火",
        object_tag: "wilderness_campfire",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "campfire",
        attribute_tags: &["fixed", "facility", "heating", "illuminating", "camping", "outdoor_gear"],
    },
    GridObjectTagSpec {
        pattern: "hunter_post",
        name_alias: "巡山木樁",
        object_tag: "hunter_post",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "hunter_post",
        attribute_tags: &["fixed", "facility", "timber", "wayfinding", "patrol", "outdoor_gear"],
    },
    GridObjectTagSpec {
        pattern: "stone",
        name_alias: "路旁護路樁",
        object_tag: "road_guard_stake",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "hunter_post",
        attribute_tags: &["fixed", "facility", "stone", "wayfinding", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "slope_post",
        name_alias: "陡坡防滑木樁",
        object_tag: "anti_skid_road_stake",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "hunter_post",
        attribute_tags: &["fixed", "facility", "timber", "outdoor_fixture", "slope"],
    },
    GridObjectTagSpec {
        pattern: "fog_post",
        name_alias: "霧林哨樁",
        object_tag: "hunter_post",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "hunter_post",
        attribute_tags: &["fixed", "facility", "timber", "wayfinding", "patrol", "outdoor_gear"],
    },
    GridObjectTagSpec {
        pattern: "peak_post",
        name_alias: "山頂哨樁",
        object_tag: "hunter_post",
        category_tag: "maritime_transport_outdoor",
        archetype_tag: "hunter_post",
        attribute_tags: &["fixed", "facility", "timber", "wayfinding", "patrol", "outdoor_gear"],
    },
    GridObjectTagSpec {
        pattern: "horse_trough",
        name_alias: "飲馬石槽",
        object_tag: "horse_drinking_stone_trough",
        category_tag: "plumbing_bath_hygiene",
        archetype_tag: "scene_fixture",
        attribute_tags: &["fixed", "utensil", "stone", "water_supply", "trough", "fauna", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "bridge_trough",
        name_alias: "石橋繫馬環",
        object_tag: "horse_mooring_ring",
        category_tag: "public_fixture",
        archetype_tag: "bollard",
        attribute_tags: &["fixed", "outdoor_fixture", "iron", "stone", "horse_ring", "bridge_fixture"],
    },
    GridObjectTagSpec {
        pattern: "bridge_plank",
        name_alias: "備換橋木板",
        object_tag: "spare_bridge_plank",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "repair_plank",
        attribute_tags: &["portable", "timber", "wood", "repair", "heavy", "bridge_material"],
    },
    GridObjectTagSpec {
        pattern: "rest_stone",
        name_alias: "路側歇腳石",
        object_tag: "roadside_rest_stone",
        category_tag: "public_fixture",
        archetype_tag: "bench",
        attribute_tags: &["fixed", "outdoor_fixture", "stone", "seating", "rest_spot", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "fruit",
        name_alias: "野果",
        object_tag: "wild_forest_fruit",
        category_tag: "herbs_flora_spices",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "fruit", "flora", "gatherable", "food_ingredient", "sweet"],
    },
    GridObjectTagSpec {
        pattern: "herb",
        name_alias: "草藥",
        object_tag: "medicinal_herb",
        category_tag: "herbs_flora_spices",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "herb", "flora", "gatherable", "medical", "healing"],
    },
    GridObjectTagSpec {
        pattern: "creek_herb",
        name_alias: "溪邊水芹",
        object_tag: "creek_water_celery",
        category_tag: "herbs_flora_spices",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "herb", "flora", "gatherable", "food_ingredient", "waterfront"],
    },
    GridObjectTagSpec {
        pattern: "pine_wood",
        name_alias: "乾燥松木柴",
        object_tag: "dried_pine_firewood",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "firewood_pile",
        attribute_tags: &["portable", "raw_material", "flora", "timber", "heating", "gatherable", "fuel"],
    },
    GridObjectTagSpec {
        pattern: "fallen_wood",
        name_alias: "林下落枯枝",
        object_tag: "fallen_dry_wood",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "firewood_pile",
        attribute_tags: &["portable", "raw_material", "flora", "timber", "heating", "gatherable", "wood"],
    },
    GridObjectTagSpec {
        pattern: "timber",
        name_alias: "原木",
        object_tag: "wild_timber_log",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "firewood_pile",
        attribute_tags: &["portable", "raw_material", "flora", "timber", "woodworking", "gatherable", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "resin",
        name_alias: "天然松脂塊",
        object_tag: "natural_pine_resin_block",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "flora", "resin", "gatherable", "fuel", "adhesive"],
    },
    GridObjectTagSpec {
        pattern: "jade_ore",
        name_alias: "溪畔水磨青卵石",
        object_tag: "creek_blue_pebble",
        category_tag: "mineral_jade",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "mineral", "jade", "gatherable", "stone", "quartz"],
    },
    GridObjectTagSpec {
        pattern: "road_ore",
        name_alias: "道旁碎紅石",
        object_tag: "roadside_red_gravel",
        category_tag: "mineral_jade",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "mineral", "gravel", "gatherable", "stone", "masonry"],
    },
    GridObjectTagSpec {
        pattern: "copper_sand",
        name_alias: "伴生銅砂",
        object_tag: "associated_copper_sand",
        category_tag: "mineral_jade",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "mineral", "copper", "gatherable", "metal", "sand"],
    },
    GridObjectTagSpec {
        pattern: "bamboo",
        name_alias: "防風青竹枝",
        object_tag: "windbreak_bamboo_branch",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "flora", "bamboo", "gatherable", "green", "crafting"],
    },
    GridObjectTagSpec {
        pattern: "bay_post",
        name_alias: "灣口繫船木樁",
        object_tag: "mooring_post",
        category_tag: "public_fixture",
        archetype_tag: "bollard",
        attribute_tags: &["fixed", "outdoor_fixture", "timber", "sailing", "marine"],
    },
    GridObjectTagSpec {
        pattern: "coast_post",
        name_alias: "海灘繫纜樁",
        object_tag: "mooring_post",
        category_tag: "public_fixture",
        archetype_tag: "bollard",
        attribute_tags: &["fixed", "outdoor_fixture", "timber", "sailing", "marine"],
    },
    GridObjectTagSpec {
        pattern: "water_post",
        name_alias: "水岸繫船柱",
        object_tag: "mooring_post",
        category_tag: "public_fixture",
        archetype_tag: "bollard",
        attribute_tags: &["fixed", "outdoor_fixture", "timber", "sailing", "marine"],
    },
    GridObjectTagSpec {
        pattern: "cliff_railing",
        name_alias: "斷崖防護木欄",
        object_tag: "cliff_guard_railing",
        category_tag: "public_fixture",
        archetype_tag: "railing",
        attribute_tags: &["fixed", "outdoor_fixture", "timber", "safety", "heavy"],
    },
    GridObjectTagSpec {
        pattern: "meadow_grass",
        name_alias: "長風牧草",
        object_tag: "long_wind_pasture_grass",
        category_tag: "herbs_flora_spices",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "pasture_grass", "flora", "gatherable", "fauna", "feed"],
    },
    GridObjectTagSpec {
        pattern: "grain_wood",
        name_alias: "向陽木枝",
        object_tag: "sun_grain_wood_branch",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "flora", "timber", "gatherable", "wood", "fuel"],
    },
    GridObjectTagSpec {
        pattern: "ore",
        name_alias: "野生礦石",
        object_tag: "wild_mineral_ore",
        category_tag: "mineral_jade",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "mineral", "gatherable", "iron", "stone", "metal"],
    },
    GridObjectTagSpec {
        pattern: "zl_bamboo_culm",
        name_alias: "合抱刺竹叢",
        object_tag: "thorny_bamboo_culm",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "resource_node",
        attribute_tags: &["fixed", "raw_material", "bamboo", "flora", "timber", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_dry_leaves",
        name_alias: "乾竹葉層",
        object_tag: "dry_bamboo_leaves",
        category_tag: "herbs_flora_spices",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "bamboo", "organic", "humus", "flora", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_bamboo_shoot",
        name_alias: "破土竹筍",
        object_tag: "emergent_bamboo_shoot",
        category_tag: "herbs_flora_spices",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "bamboo", "bamboo_shoot", "food_raw", "flora", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_bamboo_rhizome",
        name_alias: "盤錯竹鞭",
        object_tag: "interlaced_bamboo_rhizome",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "resource_node",
        attribute_tags: &["fixed", "raw_material", "bamboo", "bamboo_rhizome", "timber", "flora", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_bamboo_sheath",
        name_alias: "枯黃竹籜",
        object_tag: "shed_bamboo_sheath",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "bamboo", "bamboo_sheath", "fiber", "flora", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_bamboo_fungus",
        name_alias: "竹根木耳",
        object_tag: "bamboo_stump_wood_ear",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "bamboo", "fungus", "food_raw", "flora", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_bamboo",
        name_alias: "合抱刺竹林",
        object_tag: "thorny_bamboo_grove",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "resource_node",
        attribute_tags: &["fixed", "raw_material", "bamboo", "flora", "timber", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_shoot",
        name_alias: "破土巨筍",
        object_tag: "wild_bamboo_shoot",
        category_tag: "herbs_flora_spices",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "bamboo", "bamboo_shoot", "food_raw", "flora", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_rhizome",
        name_alias: "紅土竹鞭",
        object_tag: "laterite_bamboo_rhizome",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "bamboo", "bamboo_rhizome", "timber", "flora", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_spring",
        name_alias: "伏流冷泉",
        object_tag: "subterranean_cold_spring",
        category_tag: "plumbing_bath_hygiene",
        archetype_tag: "water_source",
        attribute_tags: &["fixed", "facility", "fresh_water", "fresh_drink", "liquid", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_creek",
        name_alias: "清淺山澗流",
        object_tag: "mountain_stream_water",
        category_tag: "plumbing_bath_hygiene",
        archetype_tag: "water_source",
        attribute_tags: &["fixed", "facility", "fresh_water", "fresh_drink", "liquid", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_quartz",
        name_alias: "石英卵石灘",
        object_tag: "quartz_pebble_bed",
        category_tag: "mineral_jade",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "mineral", "stone", "quartz_pebble", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_clay",
        name_alias: "赭紅礫石層",
        object_tag: "laterite_red_gravel",
        category_tag: "mineral_jade",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "mineral", "stone", "red_clay", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_humus",
        name_alias: "積葉腐殖坑",
        object_tag: "bamboo_leaf_humus",
        category_tag: "herbs_flora_spices",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "organic", "humus", "flora", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_camphor",
        name_alias: "百年老樟樹",
        object_tag: "ancient_camphor_tree",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "resource_node",
        attribute_tags: &["fixed", "raw_material", "camphor", "timber", "wood", "flora", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_melia",
        name_alias: "蒼勁苦楝林",
        object_tag: "native_melia_woods",
        category_tag: "timber_bamboo_fungi",
        archetype_tag: "resource_node",
        attribute_tags: &["fixed", "raw_material", "melia", "timber", "wood", "flora", "gatherable"],
    },
    GridObjectTagSpec {
        pattern: "zl_flora",
        name_alias: "叢生姑婆芋",
        object_tag: "native_wild_flora",
        category_tag: "herbs_flora_spices",
        archetype_tag: "resource_node",
        attribute_tags: &["portable", "raw_material", "organic", "flora", "herb", "gatherable"],
    },
];


pub fn find_grid_object_spec(id: &str, name: &str) -> Option<&'static GridObjectTagSpec> {
    let clean_id = id.strip_prefix("grid:").unwrap_or(id).trim();
    let clean_name = name.trim();

    // 1. 完全相符名稱比對
    if !clean_name.is_empty() {
        for spec in GRID_OBJECT_TAG_SPECS {
            if spec.name_alias == clean_name {
                return Some(spec);
            }
        }
    }

    // 2. ID 前綴比對
    if !clean_id.is_empty() {
        for spec in GRID_OBJECT_TAG_SPECS {
            if clean_id.starts_with(spec.pattern) {
                return Some(spec);
            }
        }
    }

    // 3. 名稱包含比對
    if !clean_name.is_empty() {
        for spec in GRID_OBJECT_TAG_SPECS {
            if clean_name.contains(spec.name_alias) || spec.name_alias.contains(clean_name) {
                return Some(spec);
            }
        }
    }

    None
}


/// 映射 GridObject 的 ID 與名稱至專屬英文標籤與語意原型標籤。
pub fn grid_object_tag_and_archetype(id: &str, name: &str) -> (&'static str, &'static str) {
    if let Some(spec) = find_grid_object_spec(id, name) {
        return (spec.object_tag, spec.archetype_tag);
    }
    ("grid_resource_node", "resource_node")
}

include!("cell/terrain.rs");
