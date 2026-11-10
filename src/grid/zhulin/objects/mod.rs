//! 竹林系列採集物件註冊表（純竹最小閉環：合抱刺竹叢、乾竹葉層、破土竹筍）
//!
//! 每個採集物件獨立一個檔案，提供：
//! - 槽位拉霸枚舉 `BambooSlotKind`（含 `Empty` 空白槽）
//! - 物件自帶場景短句分派 `scene_sentence_for_object`
//! - 物件自帶存量感知端詳描述 `resolve_bamboo_slot_object_with_qty`
//! - 物件自帶採集敘事 `gather_narrative_for_object`

pub mod affordance;
pub mod bamboo_culm;
pub mod bamboo_fungus;
pub mod bamboo_rhizome;
pub mod bamboo_sheath;
pub mod bamboo_shoot;
pub mod dry_leaves;

pub use affordance::{
    GatherCheckResult, ITEM_BAMBOO_CANTEEN, ITEM_BAMBOO_SPADE, ITEM_BAMBOO_STONE_AXE,
    ITEM_ROASTED_BAMBOO_SHOOT, ITEM_SHARP_STONE_FLAKE, classify_zhulin_item_capabilities,
    evaluate_fire_weather_shelter, evaluate_zhulin_gather, evaluate_zhulin_gather_with_weather,
    inventory_has_capability_tag, try_craft_zhulin_item,
};
use crate::grid::cell::GridObject;
use crate::model::RoomObject;


/// 萬物物件化之三層標籤結構（類別標籤、屬性標籤、專屬標籤）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZhulinTagTriad {
    /// 類別標籤（例：木竹建材、火絨耗材、生鮮食材）
    pub category_tags: &'static [&'static str],
    /// 屬性標籤（例：中空有節、蓬鬆易燃、天然防潮）
    pub attribute_tags: &'static [&'static str],
    /// 專屬標籤（唯一識別該物件本體，絕不與他物或舊世 ID 共用）
    pub exclusive_tag: &'static str,
}

impl ZhulinTagTriad {
    /// 檢查此三層標籤組是否包含指定標籤（比對 `category_tags`、`attribute_tags` 或 `exclusive_tag`）
    #[must_use]
    pub fn has_tag(&self, target: &str) -> bool {
        let t = target.trim();
        self.exclusive_tag == t
            || self.category_tags.contains(&t)
            || self.attribute_tags.contains(&t)
    }
}

/// 竹林採集產出之獨立物品物件定義（與舊世白塔半島 `harvest_*` 完全切割）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZhulinItemDef {
    pub id: &'static str,
    pub name: &'static str,
    pub item_type: &'static str,
    pub weight: f64,
    pub origin: &'static str,
    pub description: &'static str,
    pub tags: ZhulinTagTriad,
}

pub const ITEM_QUARTZ_COBBLE: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_quartz_cobble",
    name: "石英圓礫",
    item_type: "material",
    weight: 1.0,
    origin: "紅土西麓",
    description: "自西麓地層與溪灘採得的石英砂岩圓礫，質地堅硬密實，經水流沖磨渾圓，可作敲擊石核或砌築基石。",
    tags: ZhulinTagTriad {
        category_tags: &["stone", "mineral", "cobble"],
        attribute_tags: &["hard_dense", "water_worn", "knappable"],
        exclusive_tag: "zl_item_quartz_cobble",
    },
};

pub const ITEM_RED_CLAY: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_red_clay",
    name: "赭紅黏土",
    item_type: "material",
    weight: 0.8,
    origin: "紅土西麓",
    description: "自西麓台地深層掘取的富鐵紅土黏土，質地細膩黏韌、富可塑性，經淘洗揉捏後可耐高溫燒製陶器。",
    tags: ZhulinTagTriad {
        category_tags: &["clay", "mineral", "earth"],
        attribute_tags: &["iron_rich", "plastic_malleable", "kiln_firable"],
        exclusive_tag: "zl_item_red_clay",
    },
};

pub const ITEM_CAMPHOR_TIMBER: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_camphor_timber",
    name: "芬芳樟木料",
    item_type: "material",
    weight: 1.4,
    origin: "紅土山腰",
    description: "自山腰與坡頭原生古樟修伐的上等闊葉木料，木質緊密細緻、氣味辛烈芳香，天然驅避蟲蛀。",
    tags: ZhulinTagTriad {
        category_tags: &["timber", "hardwood", "camphor"],
        attribute_tags: &["aromatic", "insect_repellent", "fine_grain"],
        exclusive_tag: "zl_item_camphor_timber",
    },
};

pub const ITEM_MELIA_TIMBER: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_melia_timber",
    name: "苦楝韌木料",
    item_type: "material",
    weight: 1.3,
    origin: "紅土山腰",
    description: "自迎風山坡與海岸防風林採伐的苦楝老木，木理交錯強韌、耐濕防裂，利於加工削製農具柄柄。",
    tags: ZhulinTagTriad {
        category_tags: &["timber", "hardwood", "melia"],
        attribute_tags: &["tough_grain", "moisture_resistant", "split_resistant"],
        exclusive_tag: "zl_item_melia_timber",
    },
};

pub const ITEM_DRIFTWOOD: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_driftwood",
    name: "風乾漂流木",
    item_type: "material",
    weight: 0.9,
    origin: "潮間泥灘",
    description: "經海浪長年沖刷浸蝕並於高潮線烈日曬乾的漂流木材，鹽分結晶固化木纖維，乾硬易燃且適作支架。",
    tags: ZhulinTagTriad {
        category_tags: &["timber", "driftwood", "fuel_wood"],
        attribute_tags: &["salt_cured", "dry_flammable", "weathered"],
        exclusive_tag: "zl_item_driftwood",
    },
};

pub const ITEM_TIDAL_REED: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_tidal_reed",
    name: "濱海蘆葦稈",
    item_type: "material",
    weight: 0.3,
    origin: "潮間泥灘",
    description: "自潮間帶鹹淡水交界泥灘割取的修長蘆葦稈，莖稈中空柔韌，去葉曬乾後為編織草蓆簾幕之良材。",
    tags: ZhulinTagTriad {
        category_tags: &["fiber", "reed_stalk", "weaving_material"],
        attribute_tags: &["pliable", "hollow_stem", "mat_weavable"],
        exclusive_tag: "zl_item_tidal_reed",
    },
};

pub const ITEM_MISCANTHUS_CULM: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_miscanthus_culm",
    name: "堅韌菅芒莖",
    item_type: "material",
    weight: 0.25,
    origin: "西側海岸平原",
    description: "自西側海岸平原與稜頂耐風草叢採割的野生菅芒草莖，纖維極其堅韌密實，為紮縛屋頂草苫之首選。",
    tags: ZhulinTagTriad {
        category_tags: &["fiber", "thatch_material", "grass_stalk"],
        attribute_tags: &["wind_tough", "thatchable", "dry_ignitable"],
        exclusive_tag: "zl_item_miscanthus_culm",
    },
};

pub const ITEM_WILD_MELASTOMA: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_wild_melastoma",
    name: "野牡丹藥草",
    item_type: "consumable",
    weight: 0.15,
    origin: "紅土山腰",
    description: "採自紅土山坡酸性土質的野生常綠野牡丹枝葉，氣味微苦澀，搗碎敷抹可止血收斂傷口。",
    tags: ZhulinTagTriad {
        category_tags: &["herb", "medicinal_flora", "shrub_leaf"],
        attribute_tags: &["astringent", "hemostatic", "acid_soil_grown"],
        exclusive_tag: "zl_item_wild_melastoma",
    },
};

pub const ITEM_MOUNTAIN_FERN: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_mountain_fern",
    name: "鮮嫩山蘇葉",
    item_type: "consumable",
    weight: 0.2,
    origin: "台地東坡",
    description: "採擷自台地東坡陰濕闊葉林下與筆筒樹旁的野生鳥巢蕨嫩葉，翠綠爽脆、富含黏滑膠質，生熟皆可食。",
    tags: ZhulinTagTriad {
        category_tags: &["food", "fresh_greens", "edible_fern"],
        attribute_tags: &["crisp_frond", "shade_grown", "edible_raw_or_cooked"],
        exclusive_tag: "zl_item_mountain_fern",
    },
};

pub const ITEM_SPRING_WATER: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_spring_water",
    name: "清冽山泉水",
    item_type: "consumable",
    weight: 0.5,
    origin: "紅土西麓",
    description: "自台地深層礫石岩隙湧出的天然活水，水質冰涼澄澈、甘美無垢，飽含地下礦物精華，可生飲解渴。",
    tags: ZhulinTagTriad {
        category_tags: &["water", "fresh_drink", "spring_water"],
        attribute_tags: &["cold_clear", "potable", "mineral_filtered"],
        exclusive_tag: "zl_item_spring_water",
    },
};

pub const ALL_ZHULIN_ITEMS: &[ZhulinItemDef] = &[
    bamboo_culm::YIELD_ITEM_DEF,
    dry_leaves::YIELD_ITEM_DEF,
    bamboo_shoot::YIELD_ITEM_DEF,
    bamboo_rhizome::YIELD_ITEM_DEF,
    bamboo_sheath::YIELD_ITEM_DEF,
    bamboo_fungus::YIELD_ITEM_DEF,
    ITEM_QUARTZ_COBBLE,
    ITEM_RED_CLAY,
    ITEM_CAMPHOR_TIMBER,
    ITEM_MELIA_TIMBER,
    ITEM_DRIFTWOOD,
    ITEM_TIDAL_REED,
    ITEM_MISCANTHUS_CULM,
    ITEM_WILD_MELASTOMA,
    ITEM_MOUNTAIN_FERN,
    ITEM_SPRING_WATER,
    ITEM_SHARP_STONE_FLAKE,
    ITEM_BAMBOO_STONE_AXE,
    ITEM_BAMBOO_SPADE,
    ITEM_BAMBOO_CANTEEN,
    ITEM_ROASTED_BAMBOO_SHOOT,
];

/// 依物品專屬 ID 查詢竹林產出物品物件定義
pub fn lookup_zhulin_item_def(item_id: &str) -> Option<&'static ZhulinItemDef> {
    let id = item_id.trim();
    match id {
        bamboo_culm::YIELD_ITEM_ID => Some(&bamboo_culm::YIELD_ITEM_DEF),
        dry_leaves::YIELD_ITEM_ID => Some(&dry_leaves::YIELD_ITEM_DEF),
        bamboo_shoot::YIELD_ITEM_ID => Some(&bamboo_shoot::YIELD_ITEM_DEF),
        bamboo_rhizome::YIELD_ITEM_ID => Some(&bamboo_rhizome::YIELD_ITEM_DEF),
        bamboo_sheath::YIELD_ITEM_ID => Some(&bamboo_sheath::YIELD_ITEM_DEF),
        bamboo_fungus::YIELD_ITEM_ID => Some(&bamboo_fungus::YIELD_ITEM_DEF),
        "zl_item_quartz_cobble" => Some(&ITEM_QUARTZ_COBBLE),
        "zl_item_red_clay" => Some(&ITEM_RED_CLAY),
        "zl_item_camphor_timber" => Some(&ITEM_CAMPHOR_TIMBER),
        "zl_item_melia_timber" => Some(&ITEM_MELIA_TIMBER),
        "zl_item_driftwood" => Some(&ITEM_DRIFTWOOD),
        "zl_item_tidal_reed" => Some(&ITEM_TIDAL_REED),
        "zl_item_miscanthus_culm" => Some(&ITEM_MISCANTHUS_CULM),
        "zl_item_wild_melastoma" => Some(&ITEM_WILD_MELASTOMA),
        "zl_item_mountain_fern" => Some(&ITEM_MOUNTAIN_FERN),
        "zl_item_spring_water" => Some(&ITEM_SPRING_WATER),
        "zl_item_sharp_stone_flake" => Some(&ITEM_SHARP_STONE_FLAKE),
        "zl_item_bamboo_stone_axe" => Some(&ITEM_BAMBOO_STONE_AXE),
        "zl_item_bamboo_spade" => Some(&ITEM_BAMBOO_SPADE),
        "zl_item_bamboo_canteen" => Some(&ITEM_BAMBOO_CANTEEN),
        "zl_item_roasted_bamboo_shoot" => Some(&ITEM_ROASTED_BAMBOO_SHOOT),
        _ => None,
    }
}

/// 依場上採集物件 ID 查詢其三層標籤（類別標籤、屬性標籤、專屬標籤）
pub fn lookup_zhulin_object_tags(obj_id: &str) -> Option<&'static ZhulinTagTriad> {
    match obj_id.trim() {
        bamboo_culm::ID | "zl_obj_bamboo_culm" => Some(&bamboo_culm::OBJECT_TAGS),
        dry_leaves::ID | "zl_obj_dry_leaves" => Some(&dry_leaves::OBJECT_TAGS),
        bamboo_shoot::ID | "zl_obj_bamboo_shoot" => Some(&bamboo_shoot::OBJECT_TAGS),
        bamboo_rhizome::ID | "zl_obj_bamboo_rhizome" => Some(&bamboo_rhizome::OBJECT_TAGS),
        bamboo_sheath::ID | "zl_obj_bamboo_sheath" => Some(&bamboo_sheath::OBJECT_TAGS),
        bamboo_fungus::ID | "zl_obj_bamboo_fungus" => Some(&bamboo_fungus::OBJECT_TAGS),
        _ => None,
    }
}
/// 竹林格內槽位拉霸結果（含 `[空白]`）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BambooSlotKind {
    /// `[空白]`：此槽位未抽中物件（或已被採集殆盡歸零）
    Empty,
    /// 【合抱刺竹叢】（主體結構型：6 ～ 10 根）
    BambooCulm(u32),
    /// 【乾竹葉層】（地表鋪積型：8 ～ 14 把）
    DryLeaves(u32),
    /// 【破土竹筍】（點狀尋寶型：1 ～ 3 株）
    BambooShoot(u32),
    /// 【盤錯竹鞭】（地下韌材型：2 ～ 4 段）
    BambooRhizome(u32),
    /// 【枯黃竹籜】（包裹編材型：4 ～ 7 片）
    BambooSheath(u32),
    /// 【竹根木耳】（陰濕菌菇型：1 ～ 2 朵）
    BambooFungus(u32),
}

impl BambooSlotKind {
    pub fn into_grid_object(self) -> Option<GridObject> {
        match self {
            BambooSlotKind::Empty => None,
            BambooSlotKind::BambooCulm(q) => Some(bamboo_culm::build_grid_object(q)),
            BambooSlotKind::DryLeaves(q) => Some(dry_leaves::build_grid_object(q)),
            BambooSlotKind::BambooShoot(q) => Some(bamboo_shoot::build_grid_object(q)),
            BambooSlotKind::BambooRhizome(q) => Some(bamboo_rhizome::build_grid_object(q)),
            BambooSlotKind::BambooSheath(q) => Some(bamboo_sheath::build_grid_object(q)),
            BambooSlotKind::BambooFungus(q) => Some(bamboo_fungus::build_grid_object(q)),
        }
    }
}

/// 依格內物件 ID 與當前存量回傳該物件自帶的場景描述短句；
/// 若存量為 0（即槽位已變為 `[空白]`）則回傳 `None`。
pub fn scene_sentence_for_object(obj: &GridObject) -> Option<&'static str> {
    if obj.quantity == 0 {
        return None;
    }
    let s = match obj.id.as_str() {
        bamboo_culm::ID => bamboo_culm::scene_sentence(obj.quantity),
        dry_leaves::ID => dry_leaves::scene_sentence(obj.quantity),
        bamboo_shoot::ID => bamboo_shoot::scene_sentence(obj.quantity),
        bamboo_rhizome::ID => bamboo_rhizome::scene_sentence(obj.quantity),
        bamboo_sheath::ID => bamboo_sheath::scene_sentence(obj.quantity),
        bamboo_fungus::ID => bamboo_fungus::scene_sentence(obj.quantity),
        _ => return None,
    };
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// 取得竹林採集物件對應的地塊搜尋標籤
pub fn cell_tags_for_object_id(id: &str) -> &'static [&'static str] {
    match id {
        bamboo_culm::ID => bamboo_culm::CELL_TAGS,
        dry_leaves::ID => dry_leaves::CELL_TAGS,
        bamboo_shoot::ID => bamboo_shoot::CELL_TAGS,
        bamboo_rhizome::ID => bamboo_rhizome::CELL_TAGS,
        bamboo_sheath::ID => bamboo_sheath::CELL_TAGS,
        bamboo_fungus::ID => bamboo_fungus::CELL_TAGS,
        _ => &["bamboo", "flora"],
    }
}

/// 取得竹林採集物件對應的產出物品 ID（`YIELD_ITEM_ID`）
pub fn yield_item_id_for_object(obj_id: &str) -> Option<&'static str> {
    // 1. 純竹物件
    match obj_id {
        bamboo_culm::ID => return Some(bamboo_culm::YIELD_ITEM_ID),
        dry_leaves::ID => return Some(dry_leaves::YIELD_ITEM_ID),
        bamboo_shoot::ID => return Some(bamboo_shoot::YIELD_ITEM_ID),
        bamboo_rhizome::ID => return Some(bamboo_rhizome::YIELD_ITEM_ID),
        bamboo_sheath::ID => return Some(bamboo_sheath::YIELD_ITEM_ID),
        bamboo_fungus::ID => return Some(bamboo_fungus::YIELD_ITEM_ID),
        _ => {}
    }

    // 2. 0-stock 斷崖絕壁不可採集
    if obj_id == "zl_quartz_steep_cliff" || obj_id == "zl_quartz_ridge_precipice" {
        return None;
    }

    // 3. 非竹自然採集地景物件分派
    if obj_id.starts_with("zl_quartz_") || obj_id == "zl_niche_rock_shelter" {
        Some("zl_item_quartz_cobble")
    } else if obj_id.starts_with("zl_clay_") {
        Some("zl_item_red_clay")
    } else if obj_id.starts_with("zl_camphor_") {
        Some("zl_item_camphor_timber")
    } else if obj_id.starts_with("zl_melia_") {
        Some("zl_item_melia_timber")
    } else if obj_id == "zl_bamboo_driftwood" {
        Some("zl_item_driftwood")
    } else if obj_id == "zl_flora_tidal_reed" {
        Some("zl_item_tidal_reed")
    } else if obj_id == "zl_flora_plains_grass" || obj_id == "zl_flora_ridge_grass" {
        Some("zl_item_miscanthus_culm")
    } else if obj_id == "zl_flora_melastoma" || obj_id == "zl_flora_east_shrub" {
        Some("zl_item_wild_melastoma")
    } else if obj_id == "zl_flora_east_fern" || obj_id == "zl_flora_tree_ferns" {
        Some("zl_item_mountain_fern")
    } else if obj_id.starts_with("zl_spring_") || obj_id.starts_with("zl_creek_") {
        Some("zl_item_spring_water")
    } else {
        None
    }
}

/// 依執行期實際存量解析竹林採集物件之 `Look` 與 `Gather` 互動插座（預設靜態查詢給滿存量代表值）
pub fn resolve_bamboo_slot_object(target: &str) -> Option<RoomObject> {
    let t = target.trim();
    if t == bamboo_culm::ID || t == bamboo_culm::NAME {
        Some(bamboo_culm::to_room_object(8))
    } else if t == dry_leaves::ID || t == dry_leaves::NAME {
        Some(dry_leaves::to_room_object(10))
    } else if t == bamboo_shoot::ID || t == bamboo_shoot::NAME {
        Some(bamboo_shoot::to_room_object(2))
    } else if t == bamboo_rhizome::ID || t == bamboo_rhizome::NAME {
        Some(bamboo_rhizome::to_room_object(3))
    } else if t == bamboo_sheath::ID || t == bamboo_sheath::NAME {
        Some(bamboo_sheath::to_room_object(5))
    } else if t == bamboo_fungus::ID || t == bamboo_fungus::NAME {
        Some(bamboo_fungus::to_room_object(2))
    } else {
        None
    }
}

/// 依格內實際剩餘存量 `qty` 解析竹林採集物件之自然端詳描述（不輸出 `（物產存量：X/Y）`）
pub fn resolve_bamboo_slot_object_with_qty(target: &str, qty: u32) -> Option<RoomObject> {
    let t = target.trim();
    if t == bamboo_culm::ID || t == bamboo_culm::NAME {
        Some(bamboo_culm::to_room_object(qty))
    } else if t == dry_leaves::ID || t == dry_leaves::NAME {
        Some(dry_leaves::to_room_object(qty))
    } else if t == bamboo_shoot::ID || t == bamboo_shoot::NAME {
        Some(bamboo_shoot::to_room_object(qty))
    } else if t == bamboo_rhizome::ID || t == bamboo_rhizome::NAME {
        Some(bamboo_rhizome::to_room_object(qty))
    } else if t == bamboo_sheath::ID || t == bamboo_sheath::NAME {
        Some(bamboo_sheath::to_room_object(qty))
    } else if t == bamboo_fungus::ID || t == bamboo_fungus::NAME {
        Some(bamboo_fungus::to_room_object(qty))
    } else {
        None
    }
}

/// 依採集物件 ID、本次實際採得份數與採後剩餘存量，回傳專屬自然採集敘事
pub fn gather_narrative_for_object(
    obj_id: &str,
    harvested_count: u32,
    remaining_qty: u32,
) -> Option<String> {
    match obj_id {
        bamboo_culm::ID => Some(bamboo_culm::gather_narrative(harvested_count, remaining_qty)),
        dry_leaves::ID => Some(dry_leaves::gather_narrative(harvested_count, remaining_qty)),
        bamboo_shoot::ID => Some(bamboo_shoot::gather_narrative(harvested_count, remaining_qty)),
        bamboo_rhizome::ID => Some(bamboo_rhizome::gather_narrative(
            harvested_count,
            remaining_qty,
        )),
        bamboo_sheath::ID => Some(bamboo_sheath::gather_narrative(
            harvested_count,
            remaining_qty,
        )),
        bamboo_fungus::ID => Some(bamboo_fungus::gather_narrative(
            harvested_count,
            remaining_qty,
        )),
        _ => None,
    }
}

/// 為非竹林之一般自然地景物件生成沉浸式豐枯端詳尾句（全面取代生硬的 `（物產存量：X/Y）`）
pub fn append_diegetic_stock_clause(
    obj_id: &str,
    obj_name: &str,
    look_desc: &mut String,
    qty: u32,
    max_qty: u32,
) {
    // 竹林六物件已由各自的 `look_description(qty)` 完整包含自然豐枯描寫，無須再附加尾句
    if matches!(
        obj_id,
        bamboo_culm::ID
            | dry_leaves::ID
            | bamboo_shoot::ID
            | bamboo_rhizome::ID
            | bamboo_sheath::ID
            | bamboo_fungus::ID
    ) || matches!(
        obj_name,
        bamboo_culm::NAME
            | dry_leaves::NAME
            | bamboo_shoot::NAME
            | bamboo_rhizome::NAME
            | bamboo_sheath::NAME
            | bamboo_fungus::NAME
    ) {
        return;
    }
    if max_qty == 0 {
        return;
    }

    let is_water = obj_id.contains("spring")
        || obj_id.contains("creek")
        || obj_id.contains("pool")
        || obj_name.contains("泉")
        || obj_name.contains("溪")
        || obj_name.contains("潭");
    let is_mineral = obj_id.contains("pebble")
        || obj_id.contains("clay")
        || obj_id.contains("boulder")
        || obj_id.contains("gravel")
        || obj_id.contains("cave")
        || obj_id.contains("peak")
        || obj_name.contains("石")
        || obj_name.contains("土")
        || obj_name.contains("礫")
        || obj_name.contains("岩");

    let clause = if qty == 0 {
        if is_water {
            "眼下泉脈暫歇，需待地下伏流重新滲聚。"
        } else if is_mineral {
            "表層易於撿拾的石土已悉數取盡，需待風雨沖刷再度裸露。"
        } else {
            "此處可供採擷的部分已暫時取盡，尚待時序流轉自然復生。"
        }
    } else if is_water {
        "活水源源不絕、清冽充盈，隨時可俯身掬飲或汲取。"
    } else if is_mineral {
        if qty * 3 >= max_qty * 2 {
            "表層裸露的石料與土層極為豐厚，隨手便可揀選取用。"
        } else if qty * 4 >= max_qty {
            "表層可見些許採掘痕跡，周遭尚有數處完好石土可供取用。"
        } else {
            "表層易取之處已近採盡，僅餘零星幾處尚可拾取。"
        }
    } else if qty * 3 >= max_qty * 2 {
        "林木枝葉繁茂、生機盎然，可供採擷之材極為充裕。"
    } else if qty * 4 >= max_qty {
        "枝幹間留有些許採伐痕跡，尚餘數份完好物材可取。"
    } else {
        "可用之材已被採去大半，僅餘一兩處尚可取用。"
    };

    look_desc.push_str(clause);
}
