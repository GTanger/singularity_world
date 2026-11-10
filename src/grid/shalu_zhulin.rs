//! 滅世後的新生：紅土西麓與台地稜線正方格地圖生成器（一格一物件架構）。
//!
//! 核心契約：
//! 1. 地圖是一個物件（`SquareGrid`），腳下的每一格（`GridCell`）都是一個具體物件（`一格一物件`），
//!    格子上的可採集／可互動實體（`GridObject`）與後端標籤（`tags`）、存量（`quantity` / `max_quantity`）直接綁定。
//! 2. 絕不在單一格子內塞入多個互斥地貌或僅靠散文無中生有；各異地物（如卵石灘、伏流冷泉、合抱刺竹、破土巨筍、百年老樟）
//!    各自佔據獨立座標格，實現真正的空間與物件精準定位。

use std::collections::HashMap;

use crate::grid::cell::{GridObject, GridObjectKind, Terrain};
use crate::grid::coord::SquareCoord;
use crate::grid::grid::{GridCell, SquareGrid};
use crate::model::RoomObject;

/// 新生世界種子（區別於舊世界種子 2026_07_03）
pub const SHALU_ZHULIN_WORLD_SEED: u64 = 2026_11_09;

#[derive(Clone, Copy)]
struct CellObjectSpec {
    id: &'static str,
    name: &'static str,
    kind: GridObjectKind,
    quantity: u32,
    max_quantity: u32,
    regrowth_per_tick: u32,
    cell_tags: &'static [&'static str],
    look_desc: &'static str,
}

struct RoomSpec {
    x: i32,
    y: i32,
    terrain: Terrain,
    zone: &'static str,
    name: &'static str,
    description: &'static str,
    object: CellObjectSpec,
}

const ANCHOR_CELLS: &[RoomSpec] = &[
    // ── 一、紅土西麓核心九宮格（X = -1..=1, Y = -1..=1，嚴格一格一物件） ──
    RoomSpec {
        x: 0,
        y: 0,
        terrain: Terrain::Plain,
        zone: "紅土西麓",
        name: "伏流卵石灘",
        description: "紅土台地西麓中央的平坦淺灘，地面鋪滿經伏流泉水長年淘洗的石英卵石灘，北接湧泉、西臨巨竹、東連筍坡。",
        object: CellObjectSpec {
            id: "zl_quartz_cobbles",
            name: "石英卵石灘",
            kind: GridObjectKind::Mineral,
            quantity: 24,
            max_quantity: 24,
            regrowth_per_tick: 1,
            cell_tags: &["pebble_flat", "stone", "mineral"],
            look_desc: "鋪展於西麓高台中央的渾圓石英砂岩卵石灘，表面被北側滲出的伏流泉水沖刷得潔淨光滑，石質堅硬密實。",
        },
    },
    RoomSpec {
        x: 0,
        y: 1,
        terrain: Terrain::Water,
        zone: "紅土西麓",
        name: "伏流冷泉眼",
        description: "卵石灘北側的天然湧泉窪地，清冽冰涼的伏流冷泉自紅土礫石縫隙源源湧出，水面映著四周翠竹。",
        object: CellObjectSpec {
            id: "zl_spring_pool",
            name: "伏流冷泉",
            kind: GridObjectKind::WaterSource,
            quantity: 100,
            max_quantity: 100,
            regrowth_per_tick: 5,
            cell_tags: &["spring", "water_source", "fresh_drink", "water"],
            look_desc: "自台地深層紅土礫石縫隙湧出的天然冷泉池，水質清冽甘甜，水底細沙隨湧泉微微翻動，可直接汲取飲用。",
        },
    },
    RoomSpec {
        x: 0,
        y: -1,
        terrain: Terrain::Plain,
        zone: "紅土西麓",
        name: "南坡紅土坪",
        description: "卵石灘南側向陽開闊的乾爽台面，地面裸露著緊實緻密的赭紅礫石層，空氣中帶著陽光曝曬紅土的氣息。",
        object: CellObjectSpec {
            id: "zl_clay_gravel",
            name: "赭紅礫石層",
            kind: GridObjectKind::Mineral,
            quantity: 30,
            max_quantity: 30,
            regrowth_per_tick: 1,
            cell_tags: &["red_clay", "stone", "mineral"],
            look_desc: "富含鐵鋁氧化物的赭紅色黏土與圓礫石混合地層，質地緊實緻密，可掘取紅土與礫石。",
        },
    },
    RoomSpec {
        x: 1,
        y: -1,
        terrain: Terrain::Forest,
        zone: "紅土西麓",
        name: "東南百年樟林",
        description: "高台東南側與山腰銜接的闊葉林地，一株樹冠如巨傘的百年老樟樹屹立坡頭，枝葉間散發著清爽樟木香氣。",
        object: CellObjectSpec {
            id: "zl_camphor_old",
            name: "百年老樟樹",
            kind: GridObjectKind::Flora,
            quantity: 20,
            max_quantity: 20,
            regrowth_per_tick: 1,
            cell_tags: &["camphor", "timber", "flora"],
            look_desc: "樹幹粗壯、樹皮深邃縱裂的原生百年老樟樹，木質細密芬芳，枝葉與落材皆蘊含天然樟腦清香。",
        },
    },
    RoomSpec {
        x: -8,
        y: 0,
        terrain: Terrain::Hills,
        zone: "紅土西麓",
        name: "西崖礫石台",
        description: "紅土台地西側坡腳的地形折線邊緣，矗立於坡坎邊的崖畔風化巨礫直面海岸平原與遠方外海。",
        object: CellObjectSpec {
            id: "zl_quartz_bluff_boulder",
            name: "崖畔風化巨礫",
            kind: GridObjectKind::Mineral,
            quantity: 18,
            max_quantity: 18,
            regrowth_per_tick: 1,
            cell_tags: &["stone", "mineral", "observatory"],
            look_desc: "矗立在西崖坡坎邊的巨大石英砂岩礫石，表面布滿海風吹蝕的淺紋，登臨其側可遠眺整片海岸平原。",
        },
    },
    // ── 二、紅土山腰地標（X = 12..=22，一格一物件） ─────────────────────
    RoomSpec {
        x: 12,
        y: 0,
        terrain: Terrain::Hills,
        zone: "紅土山腰",
        name: "紅土礫石長坡",
        description: "順著台地西麓往上攀升的寬廣斜坡，雨水沖刷過的坡面嵌滿累累裸露圓礫石，山風帶著深山特有的濕潤氣息。",
        object: CellObjectSpec {
            id: "zl_quartz_exposed_cobbles",
            name: "裸露圓礫石",
            kind: GridObjectKind::Mineral,
            quantity: 25,
            max_quantity: 25,
            regrowth_per_tick: 1,
            cell_tags: &["red_clay", "stone", "mineral"],
            look_desc: "突出於紅土長坡表面的古老沖積圓礫，質地堅硬渾圓，見證了台地隆起前的古河床地貌。",
        },
    },
    RoomSpec {
        x: 14,
        y: 13,
        terrain: Terrain::Water,
        zone: "紅土山腰",
        name: "北澗卵石淺灘",
        description: "山坡北側的下切溪澗，冰涼清澈的山澗急流在卵石溪床間奔流激盪，水霧在林間陽光折射下泛著微光。",
        object: CellObjectSpec {
            id: "zl_creek_rapid",
            name: "山澗急流",
            kind: GridObjectKind::WaterSource,
            quantity: 80,
            max_quantity: 80,
            regrowth_per_tick: 4,
            cell_tags: &["water", "water_source", "fresh_drink"],
            look_desc: "自台地上游奔流而下的清澈山溪，水流湍急、水聲潺潺，水質冰涼甘冽。",
        },
    },
    RoomSpec {
        x: 14,
        y: -8,
        terrain: Terrain::ForestLight,
        zone: "紅土山腰",
        name: "向陽樟木緩坡",
        description: "山腰南側日照充足的緩坡林地，數株巨傘樟樹群連成一片濃蔭，空氣中瀰漫著溫潤的樟木清香。",
        object: CellObjectSpec {
            id: "zl_camphor_canopy",
            name: "巨傘樟樹群",
            kind: GridObjectKind::Flora,
            quantity: 22,
            max_quantity: 22,
            regrowth_per_tick: 1,
            cell_tags: &["camphor", "timber", "flora"],
            look_desc: "樹冠彼此交疊的原生樟樹林，陽光穿過枝葉在地上投下斑駁光影，枝幹木質堅實芬芳。",
        },
    },
    RoomSpec {
        x: 20,
        y: 0,
        terrain: Terrain::Hills,
        zone: "紅土山腰",
        name: "赤土礫石峭壁",
        description: "山腰中段陡然拔起的險峻坡面，峭壁底部因雨水侵蝕形成內凹避風岩龕，可回望西側整片蒼翠竹海。",
        object: CellObjectSpec {
            id: "zl_niche_rock_shelter",
            name: "內凹避風岩龕",
            kind: GridObjectKind::Mineral,
            quantity: 10,
            max_quantity: 10,
            regrowth_per_tick: 1,
            cell_tags: &["stone", "wind_shelter", "rest_spot"],
            look_desc: "礫石峭壁下方天然內凹的半圓形岩穴，地面乾燥平整，可遮蔽山腰強風並採集剝落岩礫。",
        },
    },
    RoomSpec {
        x: 22,
        y: 14,
        terrain: Terrain::ForestHeavy,
        zone: "紅土山腰",
        name: "北溪紅土峽谷",
        description: "北溪中上游切穿紅土礫石層形成的深邃峽谷，陰濕谷底長滿傘葉寬大的高聳筆筒樹，四周水氣氤氳。",
        object: CellObjectSpec {
            id: "zl_flora_tree_ferns",
            name: "高聳筆筒樹",
            kind: GridObjectKind::Flora,
            quantity: 14,
            max_quantity: 14,
            regrowth_per_tick: 1,
            cell_tags: &["flora", "timber", "waterfront"],
            look_desc: "生長在峽谷陰濕處的大型木本蕨類，巨大的羽狀葉片向四方舒展，樹幹布滿氣生根。",
        },
    },
    RoomSpec {
        x: 20,
        y: -10,
        terrain: Terrain::Forest,
        zone: "紅土山腰",
        name: "苦楝硬木陡坡",
        description: "山腰南側乾燥向陽的紅土陡坡，生長著成片耐旱的蒼勁苦楝林，枝葉在山風中沙沙作響。",
        object: CellObjectSpec {
            id: "zl_melia_woods",
            name: "蒼勁苦楝林",
            kind: GridObjectKind::Flora,
            quantity: 18,
            max_quantity: 18,
            regrowth_per_tick: 1,
            cell_tags: &["melia", "timber", "flora"],
            look_desc: "扎根於乾燥紅土陡坡的原生苦楝樹群，枝幹曲折蒼勁、木質紋理鮮明，耐潮抗蛀。",
        },
    },
    // ── 三、台地稜線地標（X = 35..=36，一格一物件） ─────────────────────
    RoomSpec {
        x: 35,
        y: 0,
        terrain: Terrain::ForestHeavy,
        zone: "台地稜線",
        name: "雲霧巨木稜線",
        description: "台地西麓的最高脊線，終年不散的山嵐白霧間屹立著數人合抱的參天古樟樹，山風穿過林梢發出低沉呼嘯。",
        object: CellObjectSpec {
            id: "zl_camphor_giant",
            name: "參天古樟樹",
            kind: GridObjectKind::Flora,
            quantity: 25,
            max_quantity: 25,
            regrowth_per_tick: 1,
            cell_tags: &["camphor", "timber", "flora"],
            look_desc: "屹立在稜線雲霧帶的數百年古樟，粗大枝幹上披滿松蘿與苔蘚，散發深沉醇厚的樟木香。",
        },
    },
    RoomSpec {
        x: 36,
        y: 15,
        terrain: Terrain::Water,
        zone: "台地稜線",
        name: "稜北湧泉深潭",
        description: "稜線北側凹地匯聚而成的幽深山潭，也是北側山澗的最上游源頭，深谷冷泉潭底不斷冒出細密水泡。",
        object: CellObjectSpec {
            id: "zl_spring_cold_pond",
            name: "深谷冷泉潭",
            kind: GridObjectKind::WaterSource,
            quantity: 120,
            max_quantity: 120,
            regrowth_per_tick: 5,
            cell_tags: &["spring", "water_source", "fresh_drink", "water"],
            look_desc: "匯聚稜線地下湧泉而成的深邃山潭，潭水澄澈見底、寒氣沁人，為北澗溪水之源。",
        },
    },
    RoomSpec {
        x: 35,
        y: -14,
        terrain: Terrain::Hills,
        zone: "台地稜線",
        name: "南稜望海岩峰",
        description: "台地稜線南端的突出制高點，強勁的海風與山風在此交會，立於膠結礫石岩峰旁可將西麓竹海與遠方外海盡收眼底。",
        object: CellObjectSpec {
            id: "zl_quartz_gravel_peak",
            name: "膠結礫石岩峰",
            kind: GridObjectKind::Mineral,
            quantity: 20,
            max_quantity: 20,
            regrowth_per_tick: 1,
            cell_tags: &["stone", "mineral", "observatory"],
            look_desc: "屹立於台地南稜的堅硬礫石岩峰，歷經長年風化依然穩固如山，視野極為遼闊。",
        },
    },
];

// ── 區域地塊單一物件定義（嚴格一格一物件） ─────────────────────────────

const OBJ_STREAM_WATER: CellObjectSpec = CellObjectSpec {
    id: "zl_creek_water",
    name: "清淺山澗流",
    kind: GridObjectKind::WaterSource,
    quantity: 60,
    max_quantity: 60,
    regrowth_per_tick: 3,
    cell_tags: &["water", "water_source", "fresh_drink"],
    look_desc: "自紅土台地滲出匯聚的清澈溪水，水質冰涼甘冽，在陽光下泛著粼粼波光。",
};

const OBJ_DEEP_SEA: CellObjectSpec = CellObjectSpec {
    id: "zl_sea_deep_wave",
    name: "外海深水浪湧",
    kind: GridObjectKind::WaterSource,
    quantity: 0,
    max_quantity: 0,
    regrowth_per_tick: 0,
    cell_tags: &["water", "sea_salt"],
    look_desc: "波濤洶湧的深水外海，灰藍海面翻湧白浪，深處暗流湍急，無法徒步涉入。",
};

const OBJ_TIDAL_REED: CellObjectSpec = CellObjectSpec {
    id: "zl_flora_tidal_reed",
    name: "濱海蘆葦叢",
    kind: GridObjectKind::Flora,
    quantity: 14,
    max_quantity: 14,
    regrowth_per_tick: 1,
    cell_tags: &["flora", "herb", "waterfront"],
    look_desc: "扎根於半鹹水泥灘的茂密蘆葦叢，細長葉片在海風中沙沙作響，可採割編織。",
};

const OBJ_TIDAL_DRIFTWOOD: CellObjectSpec = CellObjectSpec {
    id: "zl_bamboo_driftwood",
    name: "潮間漂流木",
    kind: GridObjectKind::Flora,
    quantity: 10,
    max_quantity: 10,
    regrowth_per_tick: 1,
    cell_tags: &["timber", "beach", "waterfront"],
    look_desc: "被海浪推上沙礫灘的風乾枯木，表面留有海水侵蝕的細密紋路，木質乾爽易燃。",
};

const OBJ_PLAINS_MISCANTHUS: CellObjectSpec = CellObjectSpec {
    id: "zl_flora_plains_grass",
    name: "平原菅芒草",
    kind: GridObjectKind::Flora,
    quantity: 18,
    max_quantity: 18,
    regrowth_per_tick: 2,
    cell_tags: &["flora", "herb"],
    look_desc: "遍布海岸平原的野生菅芒草叢，白色花穗在風中起伏，莖葉堅韌。",
};

const OBJ_COAST_SAND_GRAVEL: CellObjectSpec = CellObjectSpec {
    id: "zl_quartz_sand_gravel",
    name: "風化砂礫石",
    kind: GridObjectKind::Mineral,
    quantity: 20,
    max_quantity: 20,
    regrowth_per_tick: 1,
    cell_tags: &["stone", "mineral"],
    look_desc: "混雜在海岸平原泥沙中的細小圓礫石，表面被海風吹磨得十分粗糙。",
};

const OBJ_WINDBREAK_TREES: CellObjectSpec = CellObjectSpec {
    id: "zl_melia_wind_trees",
    name: "防風林木",
    kind: GridObjectKind::Flora,
    quantity: 14,
    max_quantity: 14,
    regrowth_per_tick: 1,
    cell_tags: &["timber", "flora", "wind_shelter"],
    look_desc: "根系深扎於砂質土的耐鹽喬木，枝葉濃密堅韌，為海岸平原遮擋強風。",
};

const OBJ_FOOTHILL_CLAY: CellObjectSpec = CellObjectSpec {
    id: "zl_clay_ochre",
    name: "赭紅黏土層",
    kind: GridObjectKind::Mineral,
    quantity: 24,
    max_quantity: 24,
    regrowth_per_tick: 1,
    cell_tags: &["red_clay", "stone", "mineral"],
    look_desc: "紅土台地西麓特有的深紅色黏土層，色澤溫潤厚重、黏性極佳。",
};

const OBJ_FOOTHILL_COBBLE: CellObjectSpec = CellObjectSpec {
    id: "zl_quartz_terrace_cobble",
    name: "台地圓礫石",
    kind: GridObjectKind::Mineral,
    quantity: 22,
    max_quantity: 22,
    regrowth_per_tick: 1,
    cell_tags: &["stone", "mineral"],
    look_desc: "嵌在赭紅旱坪中的古老河床圓礫石，質地堅硬。",
};

const OBJ_MIDSLOPE_CLIFF: CellObjectSpec = CellObjectSpec {
    id: "zl_quartz_steep_cliff",
    name: "崩塌礫石陡壁",
    kind: GridObjectKind::Mineral,
    quantity: 0,
    max_quantity: 0,
    regrowth_per_tick: 0,
    cell_tags: &["stone", "mineral"],
    look_desc: "因長年風雨侵蝕而垂直崩落的紅土礫石陡崖，崖壁高聳險峻，大於人形之固體岩壁無法直接穿越。",
};

const OBJ_MIDSLOPE_CAMPHOR: CellObjectSpec = CellObjectSpec {
    id: "zl_camphor_mid",
    name: "山腰老樟木",
    kind: GridObjectKind::Flora,
    quantity: 18,
    max_quantity: 18,
    regrowth_per_tick: 1,
    cell_tags: &["camphor", "bamboo", "timber", "flora"],
    look_desc: "生長在紅土山腰樟竹混林間的粗壯樟樹，樹皮深裂並散發淡淡清香。",
};

const OBJ_MIDSLOPE_MELASTOMA: CellObjectSpec = CellObjectSpec {
    id: "zl_flora_melastoma",
    name: "山坡野牡丹",
    kind: GridObjectKind::Flora,
    quantity: 12,
    max_quantity: 12,
    regrowth_per_tick: 1,
    cell_tags: &["flora", "herb", "red_clay"],
    look_desc: "喜愛酸性紅土礫坡的原生灌木，葉脈深刻清晰，根葉皆可入藥。",
};

const OBJ_MIDSLOPE_MELIA: CellObjectSpec = CellObjectSpec {
    id: "zl_melia_mid",
    name: "山坡苦楝樹",
    kind: GridObjectKind::Flora,
    quantity: 16,
    max_quantity: 16,
    regrowth_per_tick: 1,
    cell_tags: &["melia", "timber", "flora"],
    look_desc: "適應乾燥紅土礫石層的原生苦楝樹，樹冠舒展、木質堅韌。",
};

const OBJ_RIDGE_PRECIPICE: CellObjectSpec = CellObjectSpec {
    id: "zl_quartz_ridge_precipice",
    name: "千仞紅土斷崖",
    kind: GridObjectKind::Mineral,
    quantity: 0,
    max_quantity: 0,
    regrowth_per_tick: 0,
    cell_tags: &["stone", "mineral"],
    look_desc: "近乎垂直崩落的紅土礫石斷崖，雲霧繚繞、深不見底，為不可穿越之天然屏障。",
};

const OBJ_RIDGE_ANCIENT_TREE: CellObjectSpec = CellObjectSpec {
    id: "zl_camphor_ridge_ancient",
    name: "盤根古樟木",
    kind: GridObjectKind::Flora,
    quantity: 20,
    max_quantity: 20,
    regrowth_per_tick: 1,
    cell_tags: &["camphor", "timber", "flora"],
    look_desc: "屹立於稜線雲霧帶的古老闊葉巨木，盤根錯節、枝幹蒼勁。",
};

const OBJ_RIDGE_ROCK: CellObjectSpec = CellObjectSpec {
    id: "zl_quartz_ridge_rock",
    name: "稜線膠結岩塊",
    kind: GridObjectKind::Mineral,
    quantity: 18,
    max_quantity: 18,
    regrowth_per_tick: 1,
    cell_tags: &["stone", "mineral"],
    look_desc: "由圓礫石與紅土緊密膠結而成的稜線岩塊，歷經強風吹蝕依然穩固。",
};

const OBJ_RIDGE_DEW_GRASS: CellObjectSpec = CellObjectSpec {
    id: "zl_flora_ridge_grass",
    name: "稜頂耐風草叢",
    kind: GridObjectKind::Flora,
    quantity: 15,
    max_quantity: 15,
    regrowth_per_tick: 2,
    cell_tags: &["flora", "herb"],
    look_desc: "生長於台地稜頂強風帶的低矮草叢，根系盤結緊密，草尖常凝結清澈露珠。",
};

const OBJ_EAST_STREAM: CellObjectSpec = CellObjectSpec {
    id: "zl_creek_east_stream",
    name: "清緩溪流水",
    kind: GridObjectKind::WaterSource,
    quantity: 70,
    max_quantity: 70,
    regrowth_per_tick: 4,
    cell_tags: &["water", "water_source", "fresh_drink"],
    look_desc: "順著台地東坡緩緩流向盆地的清澈溪流，水勢平穩、清涼甘甜。",
};

const OBJ_EAST_FERN: CellObjectSpec = CellObjectSpec {
    id: "zl_flora_east_fern",
    name: "林下山蘇叢",
    kind: GridObjectKind::Flora,
    quantity: 14,
    max_quantity: 14,
    regrowth_per_tick: 1,
    cell_tags: &["flora", "herb", "timber"],
    look_desc: "附生於東坡背風闊葉林樹幹與濕石上的翠綠山蘇蕨，嫩葉鮮脆。",
};

const OBJ_EAST_SHRUB: CellObjectSpec = CellObjectSpec {
    id: "zl_flora_east_shrub",
    name: "階地野灌叢",
    kind: GridObjectKind::Flora,
    quantity: 16,
    max_quantity: 16,
    regrowth_per_tick: 1,
    cell_tags: &["flora", "herb", "red_clay"],
    look_desc: "散生在東坡紅土階地上的原生常綠灌木叢，枝葉繁茂。",
};

const REGIONAL_OBJECTS: &[CellObjectSpec] = &[
    OBJ_STREAM_WATER,
    OBJ_DEEP_SEA,
    OBJ_TIDAL_REED,
    OBJ_TIDAL_DRIFTWOOD,
    OBJ_PLAINS_MISCANTHUS,
    OBJ_COAST_SAND_GRAVEL,
    OBJ_WINDBREAK_TREES,
    OBJ_FOOTHILL_CLAY,
    OBJ_FOOTHILL_COBBLE,
    OBJ_MIDSLOPE_CLIFF,
    OBJ_MIDSLOPE_CAMPHOR,
    OBJ_MIDSLOPE_MELASTOMA,
    OBJ_MIDSLOPE_MELIA,
    OBJ_RIDGE_PRECIPICE,
    OBJ_RIDGE_ANCIENT_TREE,
    OBJ_RIDGE_ROCK,
    OBJ_RIDGE_DEW_GRASS,
    OBJ_EAST_STREAM,
    OBJ_EAST_FERN,
    OBJ_EAST_SHRUB,
];

fn build_single_object_cell(
    coord: SquareCoord,
    terrain: Terrain,
    zone: &str,
    name: &str,
    desc: &str,
    spec: CellObjectSpec,
) -> GridCell {
    let mut cell = GridCell::new(coord, terrain, name);
    cell.zone = zone.to_string();
    cell.description = desc.to_string();
    cell.explored = true;
    let mut tags: Vec<String> = spec.cell_tags.iter().map(|s| (*s).to_string()).collect();
    for derived in crate::grid::derive_room_tags(terrain, name, zone) {
        if !tags.iter().any(|t| t == &derived) {
            tags.push(derived);
        }
    }
    cell.tags = tags;
    cell.objects = vec![GridObject {
        id: spec.id.to_string(),
        kind: spec.kind,
        name: spec.name.to_string(),
        quantity: spec.quantity,
        max_quantity: spec.max_quantity,
        regrowth_per_tick: spec.regrowth_per_tick,
    }];
    cell
}

/// 生成紅土西麓與台地稜線 121 × 81 = 9,801 格完整純自然地形世界地圖（X = -60..=60, Y = -40..=40）。
/// 嚴格落實「一格一物件」：每一格皆為具備唯一主體 `GridObject`、結構化 `tags` 與真實存量之物件格。
pub fn generate_shalu_zhulin_grid() -> SquareGrid {
    let mut grid = SquareGrid::new();
    grid.set_world_seed(SHALU_ZHULIN_WORLD_SEED);

    // 1. 插入核心與區域定位地塊（每格嚴格對應 1 個實體自然物件）
    for spec in ANCHOR_CELLS {
        let coord = SquareCoord::new(spec.x, spec.y);
        let cell = build_single_object_cell(
            coord,
            spec.terrain,
            spec.zone,
            spec.name,
            spec.description,
            spec.object,
        );
        grid.insert(cell);
    }

    // 2. 確定性遍歷填充 X = -60..=60, Y = -40..=40 之 9,801 格純自然地形（每格嚴格 1 個自然物件）
    for x in -60..=60 {
        for y in -40..=40 {
            let coord = SquareCoord::new(x, y);
            if grid.contains(coord) {
                continue;
            }

            let fx = x as f64;
            let fy = y as f64;
            let w_f = 3.5 * (fy * 0.11).sin() + 2.0 * (fy * 0.23 + 1.1).cos();
            let w = w_f.round() as i32;
            let n1 = (fx * 0.22 + (fy * 0.17).sin() * 1.8).sin()
                * (fy * 0.21 + (fx * 0.15).cos() * 1.6).cos();
            let n2 = ((fx + fy) * 0.14 + 0.7).sin() * ((fx - fy) * 0.16 - 0.4).cos();
            let n3 = (fx * 0.37 - fy * 0.31).sin() * 0.22;
            let h = (((n1 * 0.55 + n2 * 0.35 + n3 * 0.10) + 1.0) * 50.0).clamp(0.0, 99.9) as i32;

            // ── 天然水系（山澗伏流溪道）：連續蜿蜒自東向西奔流入海 ──
            let coast_x = -42 + (w / 2);
            let is_stream = if x >= coast_x && x <= 38 {
                let north_y_f = 11.5
                    + (fx + 42.0) * 0.06
                    + 2.6 * (fx * 0.13).sin()
                    + 1.1 * (fx * 0.06 + 0.5).cos();
                if (fy - north_y_f).abs() <= 0.85 && !(x >= -2 && x <= 2 && y >= -2 && y <= 2) {
                    let name = if x < -10 {
                        "平原伏流河道"
                    } else if x < 15 {
                        "北澗卵石溪床"
                    } else {
                        "北溪上游山澗"
                    };
                    Some((
                        Terrain::Water,
                        "北澗溪谷",
                        name,
                        "自台地紅土層匯聚而下的清淺山澗流，冰涼溪水順著河道蜿蜒奔流。",
                        OBJ_STREAM_WATER,
                    ))
                } else {
                    None
                }
            } else {
                None
            };

            let is_stream = is_stream.or_else(|| {
                if x >= coast_x && x <= 32 {
                    let south_y_f = -13.0
                        - (fx + 42.0) * 0.06
                        + 2.4 * (fx * 0.11 + 0.8).cos()
                        + 1.0 * (fx * 0.05).sin();
                    if (fy - south_y_f).abs() <= 0.85 {
                        let name = if x < -10 {
                            "南側平原溪流"
                        } else {
                            "南坑礫石山澗"
                        };
                        Some((
                            Terrain::Water,
                            "北澗溪谷",
                            name,
                            "沿著台地南側凹谷流淌的清淺山澗流，清澈水面映照著兩岸林影。",
                            OBJ_STREAM_WATER,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

            let (terrain, zone, name, desc, obj_spec): (
                Terrain,
                &str,
                &str,
                &str,
                CellObjectSpec,
            ) = if let Some(s) = is_stream {
                s
            } else {
                let band_x = fx
                    - 0.55 * w_f
                    - 1.7 * (fy * 0.19).sin()
                    - 0.9 * (fy * 0.33 + 1.1).cos();
                let theta = fy.atan2(fx);
                let r_bamboo = ((fx * 0.82).powi(2) + (fy * 0.65).powi(2)).sqrt()
                    + 1.8 * (theta * 3.0).sin()
                    + 1.1 * (theta * 5.0 + 0.7).cos();
                if r_bamboo <= 8.4 || (band_x >= -8.0 && band_x <= 6.5 && h < 45) {
                    let is_waterside = (x.abs() <= 1 && (y - 1).abs() <= 1) || h < 30;
                    let cell = crate::grid::zhulin::build_bamboo_terrain_cell(
                        coord,
                        "紅土西麓",
                        is_waterside,
                        SHALU_ZHULIN_WORLD_SEED,
                    );
                    grid.insert(cell);
                    continue;
                } else if band_x < -42.0 {
                    (
                        Terrain::WaterDeep,
                        "西側外海",
                        "西側深水外海",
                        "波濤起伏的外海深水浪湧，灰藍海面隨海風翻湧白浪，深處水勢湍急無法徒步涉入。",
                        OBJ_DEEP_SEA,
                    )
                } else if band_x < -30.0 {
                    if h < 40 {
                        (
                            Terrain::Swamp,
                            "潮間泥灘",
                            "濱海蘆葦泥澤",
                            "海水與山澗伏流交會的潮間泥澤，鬆軟泥灘上長滿隨風搖擺的濱海蘆葦叢。",
                            OBJ_TIDAL_REED,
                        )
                    } else {
                        (
                            Terrain::Beach,
                            "潮間泥灘",
                            "潮間沙礫海灘",
                            "綿延於外海邊緣的開闊沙礫灘，海浪反覆拍打著散落岸邊的潮間漂流木。",
                            OBJ_TIDAL_DRIFTWOOD,
                        )
                    }
                } else if band_x < -8.0 {
                    if h < 50 {
                        (
                            Terrain::Grassland,
                            "西側海岸平原",
                            "平原菅芒草海",
                            "西側海岸平原的開闊草地，成片高過人頭的平原菅芒草隨海風翻湧如浪。",
                            OBJ_PLAINS_MISCANTHUS,
                        )
                    } else if h < 80 {
                        (
                            Terrain::Plain,
                            "西側海岸平原",
                            "海濱砂質平地",
                            "地勢低平的海岸砂質原野，地面散落著粗糙的風化砂礫石，視野極為遼闊。",
                            OBJ_COAST_SAND_GRAVEL,
                        )
                    } else {
                        (
                            Terrain::ForestLight,
                            "西側海岸平原",
                            "海濱防風雜木林",
                            "生長於海岸平原上的耐風樹林，枝幹向內陸傾斜的防風林木擋去大半海風。",
                            OBJ_WINDBREAK_TREES,
                        )
                    }
                } else if band_x <= 6.5 {
                    if h < 80 {
                        (
                            Terrain::Hills,
                            "紅土西麓",
                            "西麓紅土緩坡",
                            "由海岸平原抬升至台地西麓的斜坡，地表裸露著厚實溫潤的赭紅黏土層。",
                            OBJ_FOOTHILL_CLAY,
                        )
                    } else {
                        (
                            Terrain::Plain,
                            "紅土西麓",
                            "坡腳紅土旱坪",
                            "台地西麓坡腳處開闊的乾硬台面，地面散布著堅硬渾圓的台地圓礫石。",
                            OBJ_FOOTHILL_COBBLE,
                        )
                    }
                } else if band_x <= 26.5 {
                    if h < 15 {
                        (
                            Terrain::Mountain,
                            "紅土山腰",
                            "山腰崩塌陡崖",
                            "因長年風雨侵蝕而垂直崩落的崩塌礫石陡壁，崖壁險峻陡峭，無法直接徒步攀越。",
                            OBJ_MIDSLOPE_CLIFF,
                        )
                    } else if h < 50 {
                        (
                            Terrain::Forest,
                            "紅土山腰",
                            "山腰樟竹混林",
                            "台地山腰處的茂密混生山林，粗壯的山腰老樟木與翠竹交織成蔭。",
                            OBJ_MIDSLOPE_CAMPHOR,
                        )
                    } else if h < 80 {
                        (
                            Terrain::Hills,
                            "紅土山腰",
                            "山腰紅土礫坡",
                            "往台地高處爬升的赭紅礫石斜坡，坡旁盛開著耐旱的山坡野牡丹。",
                            OBJ_MIDSLOPE_MELASTOMA,
                        )
                    } else {
                        (
                            Terrain::ForestLight,
                            "紅土山腰",
                            "山腰向陽苦楝林",
                            "山腰向陽處的疏朗樹林，耐旱的山坡苦楝樹枝幹蒼勁舒展。",
                            OBJ_MIDSLOPE_MELIA,
                        )
                    }
                } else if band_x <= 44.5 {
                    if h < 20 {
                        (
                            Terrain::Mountain,
                            "台地稜線",
                            "險峻紅土斷崖",
                            "近乎垂直崩落的千仞紅土斷崖，地勢險峻且雲霧繚繞，無法直接攀越。",
                            OBJ_RIDGE_PRECIPICE,
                        )
                    } else if h < 60 {
                        (
                            Terrain::ForestHeavy,
                            "台地稜線",
                            "稜線雲霧巨木林",
                            "台地最高處的濃密原始巨木林，終年雲霧在蒼勁的盤根古樟木間流轉。",
                            OBJ_RIDGE_ANCIENT_TREE,
                        )
                    } else if h < 85 {
                        (
                            Terrain::Hills,
                            "台地稜線",
                            "稜線風化礫石嶺",
                            "台地脊線上的開闊山嶺，強勁山風掠過堅硬穩固的稜線膠結岩塊。",
                            OBJ_RIDGE_ROCK,
                        )
                    } else {
                        (
                            Terrain::Grassland,
                            "台地稜線",
                            "稜頂迎風草坡",
                            "台地稜頂受強風吹拂的高地草坡，地面覆滿低矮堅韌的稜頂耐風草叢。",
                            OBJ_RIDGE_DEW_GRASS,
                        )
                    }
                } else {
                    let east_river_x =
                        55.0 + 2.2 * (fy * 0.14).sin() + 1.0 * (fy * 0.07).cos();
                    if (fx - east_river_x).abs() <= 0.85 {
                        (
                            Terrain::Water,
                            "台地東坡",
                            "東麓盆地溪流",
                            "台地東側山腳下匯聚的清緩溪流水，順著鵝卵石河床蜿蜒流淌。",
                            OBJ_EAST_STREAM,
                        )
                    } else if h < 50 {
                        (
                            Terrain::Forest,
                            "台地東坡",
                            "東坡闊葉雜木林",
                            "台地東側背海風的溫暖坡林，茂密樹蔭下長滿翠綠的林下山蘇叢。",
                            OBJ_EAST_FERN,
                        )
                    } else {
                        (
                            Terrain::Hills,
                            "台地東坡",
                            "東坡紅土階地",
                            "台地向東側盆地層層下降的紅土階地，坡面分布著茂密的階地野灌叢。",
                            OBJ_EAST_SHRUB,
                        )
                    }
                }
            };

            let cell = build_single_object_cell(coord, terrain, zone, name, desc, obj_spec);
            grid.insert(cell);
        }
    }

    grid.clear_dirty();
    grid
}

/// 解析場景單一格物件之 `Look` 與採集插座回應。
pub fn resolve_shalu_zhulin_object(target: &str) -> Option<RoomObject> {
    let t = target.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(obj) = crate::grid::zhulin::resolve_bamboo_slot_object(t) {
        return Some(obj);
    }
    for room in ANCHOR_CELLS {
        let f = &room.object;
        if f.id == t || f.name == t || room.name == t {
            let sockets = if f.max_quantity > 0 {
                vec!["Look".to_string(), "Gather".to_string()]
            } else {
                vec!["Look".to_string()]
            };
            return Some(RoomObject {
                id: f.id.to_string(),
                name: f.name.to_string(),
                owner: String::new(),
                sockets,
                responses: HashMap::from([("Look".to_string(), f.look_desc.to_string())]),
                move_to_room_id: String::new(),
            });
        }
    }
    for f in REGIONAL_OBJECTS {
        if f.id == t || f.name == t {
            let sockets = if f.max_quantity > 0 {
                vec!["Look".to_string(), "Gather".to_string()]
            } else {
                vec!["Look".to_string()]
            };
            return Some(RoomObject {
                id: f.id.to_string(),
                name: f.name.to_string(),
                owner: String::new(),
                sockets,
                responses: HashMap::from([("Look".to_string(), f.look_desc.to_string())]),
                move_to_room_id: String::new(),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cell_enforces_one_cell_one_object_and_tag_positioning() {
        let grid = generate_shalu_zhulin_grid();
        assert_eq!(grid.len(), 9801);
        let mut walkable_count = 0usize;
        for cell in grid.cells() {
            assert_eq!(
                cell.terrain.category(),
                "terrain",
                "No settlement building allowed: {:?}",
                cell.coord
            );
            if crate::grid::zhulin::is_bamboo_slot_cell(cell) {
                assert_eq!(cell.name, "刺竹林");
                assert_eq!(cell.display_symbol(), "𓐂");
                assert!(
                    (1..=3).contains(&cell.objects.len()),
                    "Bamboo cell ({}, {}) must roll 1..=3 non-empty slot objects, got {}",
                    cell.coord.x,
                    cell.coord.y,
                    cell.objects.len()
                );
            } else {
                assert_eq!(
                    cell.objects.len(),
                    1,
                    "Non-bamboo cell ({}, {}) '{}' must have 1 object, got {}",
                    cell.coord.x,
                    cell.coord.y,
                    cell.name,
                    cell.objects.len()
                );
            }
            assert!(
                !cell.tags.is_empty(),
                "Every cell ({}, {}) '{}' must have structured tags",
                cell.coord.x,
                cell.coord.y,
                cell.name
            );
            for obj in &cell.objects {
                assert!(
                    cell.description.contains(&obj.name),
                    "Cell ({}, {}) '{}' description '{}' does not contain slot object name '{}'",
                    cell.coord.x,
                    cell.coord.y,
                    cell.name,
                    cell.description,
                    obj.name
                );
                let resolved = resolve_shalu_zhulin_object(&obj.id)
                    .unwrap_or_else(|| panic!("Object '{}' ({}) not resolvable", obj.name, obj.id));
                assert!(!resolved.responses["Look"].is_empty());
                if cell.terrain.walkable() {
                    assert!(
                        obj.max_quantity > 0 && obj.quantity > 0,
                        "Walkable cell ({}, {}) '{}' object '{}' must have positive quantity",
                        cell.coord.x,
                        cell.coord.y,
                        cell.name,
                        obj.name
                    );
                    assert!(resolved.sockets.contains(&"Gather".to_string()));
                }
            }

            if cell.terrain.walkable() {
                walkable_count += 1;
            }
        }
        assert!(
            walkable_count > 6000,
            "Expected > 6000 walkable cells, got {walkable_count}"
        );

        // 驗證原點九宮格「一格一物件」精準空間定位
        let origin = SquareCoord::new(0, 0);
        let c00 = grid.get(origin).unwrap();
        assert_eq!(c00.name, "伏流卵石灘");
        assert_eq!(c00.objects[0].name, "石英卵石灘");
        assert!(c00.has_room_tag("pebble_flat"));

        let ((sx, sy), _) = grid
            .find_nearest_room_by_tags((0, 0), 5, &["spring"], 0)
            .expect("Must locate spring cell by tag");
        assert_eq!((sx, sy), (0, 1));
        assert_eq!(grid.get(SquareCoord::new(sx, sy)).unwrap().objects[0].name, "伏流冷泉");

        let ((bx, by), _) = grid
            .find_nearest_room_by_tags((0, 0), 5, &["bamboo_shoot"], 0)
            .expect("Must locate bamboo shoot cell by tag");
        assert_eq!((bx, by), (1, 0));
        let east_bamboo = grid.get(SquareCoord::new(bx, by)).unwrap();
        assert_eq!(east_bamboo.name, "刺竹林");
        assert!(east_bamboo.objects.iter().any(|o| o.name == "破土竹筍"));
        let ((cx, cy), _) = grid
            .find_nearest_room_by_tags((0, 0), 5, &["camphor"], 0)
            .expect("Must locate camphor cell by tag");
        assert_eq!((cx, cy), (1, -1));
        assert_eq!(grid.get(SquareCoord::new(cx, cy)).unwrap().objects[0].name, "百年老樟樹");

        // 驗證西側外海與海岸線防線：x <= -30 絕不生成海外竹林島，x <= -46 全數為深水外海
        for cell in grid.cells() {
            if cell.coord.x <= -30 {
                assert!(
                    !crate::grid::zhulin::is_bamboo_slot_cell(cell),
                    "Cell at ({}, {}) should not be a bamboo cell in western ocean",
                    cell.coord.x,
                    cell.coord.y
                );
            }
            if cell.coord.x <= -48 {
                assert_eq!(
                    cell.terrain,
                    Terrain::WaterDeep,
                    "Cell at ({}, {}) in far western deep ocean must be WaterDeep",
                    cell.coord.x,
                    cell.coord.y
                );
            }
        }
    }
}
