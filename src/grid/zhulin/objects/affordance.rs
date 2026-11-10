//! 竹林三層標籤對撞檢定與荒野啟動鏈（`src/grid/zhulin/objects/affordance.rs`）
//!
//! 100% 以物件之三層正交標籤（`category_tags`、`attribute_tags`、`exclusive_tag`）驅動：
//! - 場上採集門檻梯度（第 0 階徒手直採、第 1 階淺土帶刺高損耗、第 1.5/2 階需切割／掘土／重砍劈工具標籤）
//! - 出生點 `(0,0)` 九宮格零死鎖手作啟動鏈（敲擊打片 $\to$ 紮製石斧 $\to$ 裁製竹器／引火竹爆／煨烤熟食）

use crate::db::{self, inventory::InventoryEntry};

use super::{
    ZhulinItemDef, ZhulinTagTriad, bamboo_culm, bamboo_rhizome, bamboo_shoot,
    lookup_zhulin_item_def,
};

pub const ITEM_SHARP_STONE_FLAKE: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_sharp_stone_flake",
    name: "銳邊石片",
    item_type: "tool",
    weight: 0.25,
    origin: "手作打製",
    description: "兩塊石英圓礫互擊剝離出的鋒利石片，刃口呈貝殼狀斷口，可割斷老竹鬚根、削竹或無損撬取竹筍。",
    tags: ZhulinTagTriad {
        category_tags: &["tool", "stone_implement", "cutting_flake"],
        attribute_tags: &["sharp_edge", "pry_edge", "knapped_quartz"],
        exclusive_tag: "zl_item_sharp_stone_flake",
    },
};

pub const ITEM_BAMBOO_STONE_AXE: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_bamboo_stone_axe",
    name: "籜繩竹鞭石斧",
    item_type: "tool",
    weight: 1.1,
    origin: "手作紮製",
    description: "以實心老竹根鞭為抗震斧柄，嵌夾銳邊石英刃，外用韌竹籜搓繩緊緊紮縛的重型石斧，足可砍伐堅厚刺竹。",
    tags: ZhulinTagTriad {
        category_tags: &["tool", "stone_implement", "chopping_axe"],
        attribute_tags: &["heavy_chop", "earth_dig", "sharp_edge", "shock_resistant"],
        exclusive_tag: "zl_item_bamboo_stone_axe",
    },
};

pub const ITEM_BAMBOO_SPADE: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_bamboo_spade",
    name: "掘土竹鏟",
    item_type: "tool",
    weight: 0.45,
    origin: "手作裁削",
    description: "取厚壁刺竹稈留節斜劈、刃口經微烤硬化的竹製窄鏟，專司連根掘筍、起竹鞭與翻挖紅土。",
    tags: ZhulinTagTriad {
        category_tags: &["tool", "bamboo_implement", "digging_spade"],
        attribute_tags: &["earth_dig", "pry_edge", "beveled_culm"],
        exclusive_tag: "zl_item_bamboo_spade",
    },
};

pub const ITEM_BAMBOO_CANTEEN: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_bamboo_canteen",
    name: "青竹水筒",
    item_type: "consumable",
    weight: 0.6,
    origin: "手作裁削",
    description: "截取一節中空刺竹稈，留底節盛水、上塞乾竹葉栓，筒內山泉清冽甘甜並透著淡淡竹青清香。",
    tags: ZhulinTagTriad {
        category_tags: &["container", "bamboo_vessel", "water_flask"],
        attribute_tags: &["water_container", "node_sealed", "potable"],
        exclusive_tag: "zl_item_bamboo_canteen",
    },
};

pub const ITEM_ROASTED_BAMBOO_SHOOT: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_roasted_bamboo_shoot",
    name: "籜裹煨竹筍",
    item_type: "consumable",
    weight: 0.35,
    origin: "野炊煨烤",
    description: "以防潮韌竹籜包裹鮮刺竹筍與野竹耳，埋入乾竹葉火堆慢煨至熟，盡去生筍苦澀，入口鮮甜溫熱。",
    tags: ZhulinTagTriad {
        category_tags: &["food", "cooked_dish", "bamboo_cuisine"],
        attribute_tags: &["cooked_savory", "sheath_steamed", "stamina_restoring"],
        exclusive_tag: "zl_item_roasted_bamboo_shoot",
    },
};

pub const ITEM_AGED_BAMBOO_SLAT: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_aged_bamboo_slat",
    name: "風折老竹片",
    item_type: "material",
    weight: 0.2,
    origin: "刺竹林",
    description: "刺竹林下經風吹折斷、自然陰乾的老竹裂片，外層竹青富含天然矽質，經文火烘烤硬化後可削出鋒利竹刃。",
    tags: ZhulinTagTriad {
        category_tags: &["material", "flora", "bamboo"],
        attribute_tags: &["splittable", "silica_rind", "heat_hardenable"],
        exclusive_tag: "zl_item_aged_bamboo_slat",
    },
};

pub const ITEM_FIRE_HARDENED_BAMBOO_KNIFE: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_fire_hardened_bamboo_knife",
    name: "烤刃竹片刀",
    item_type: "tool",
    weight: 0.18,
    origin: "刺竹林",
    description: "取風折老竹片將富含矽質的竹青外緣置於乾竹火苗上反覆烘烤去水，刮削出的斜刃堅硬銳利，足以斬斷竹鞭鬚根、撬挖春筍與刻竹留號。",
    tags: ZhulinTagTriad {
        category_tags: &["tool", "flora", "bamboo", "blade"],
        attribute_tags: &["sharp_edge", "pry_edge", "cutting_flake"],
        exclusive_tag: "zl_item_fire_hardened_bamboo_knife",
    },
};

pub const ITEM_SHEATH_CORD: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_sheath_cord",
    name: "竹籜繩",
    item_type: "material",
    weight: 0.1,
    origin: "刺竹林",
    description: "將兩片枯黃韌竹籜順著縱向纖維撕成細條、掌心對搓絞合而成的雙股韌繩，抗拉耐潮，可紮縛槌斧或編結背簍。",
    tags: ZhulinTagTriad {
        category_tags: &["material", "flora", "bamboo", "cordage"],
        attribute_tags: &["cordage", "binding_fiber", "weaving_fiber"],
        exclusive_tag: "zl_item_sheath_cord",
    },
};

pub const ITEM_RHIZOME_CLUB: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_rhizome_club",
    name: "竹鞭根槌",
    item_type: "tool",
    weight: 0.95,
    origin: "刺竹林",
    description: "以實心密節的老竹根鞭為主體，利用堅硬如鐵的膨大根瘤為槌頭、握柄纏滿竹籜繩防震，揮砸劈砍力道沉猛，可伐取刺竹稈。",
    tags: ZhulinTagTriad {
        category_tags: &["tool", "flora", "bamboo", "chopper"],
        attribute_tags: &["heavy_chop", "earth_dig", "impact_resistant"],
        exclusive_tag: "zl_item_rhizome_club",
    },
};

pub const ITEM_BAMBOO_BASKET: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_bamboo_basket",
    name: "刺竹背簍",
    item_type: "tool",
    weight: 0.6,
    origin: "刺竹林",
    description: "將刺竹稈順節劈為勻稱青竹篾，以十字打底、六角孔編起簍身並穿入竹籜雙肩背帶，可大幅分擔山林採集負重（負重上限提升 15 公斤）。",
    tags: ZhulinTagTriad {
        category_tags: &["equipment", "flora", "bamboo", "container"],
        attribute_tags: &["carry_pack", "breathable_weave"],
        exclusive_tag: "zl_item_bamboo_basket",
    },
};

pub const ITEM_BAMBOO_CANTEEN_FULL: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_bamboo_canteen_full",
    name: "盛水青竹筒",
    item_type: "food",
    weight: 0.85,
    origin: "刺竹林",
    description: "盛滿清冽山雨與竹葉甘露的青竹水筒，帶著淡淡竹青芳香，既可仰頭暢飲解渴，亦可架於炭火上作為煲煮山珍湯的竹筒鍋具。",
    tags: ZhulinTagTriad {
        category_tags: &["food", "flora", "bamboo", "container"],
        attribute_tags: &["water_filled", "hydrating", "soup_base", "potable"],
        exclusive_tag: "zl_item_bamboo_canteen_full",
    },
};

pub const ITEM_BAMBOO_TUBE_SOUP: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_bamboo_tube_soup",
    name: "竹筒山珍湯",
    item_type: "food",
    weight: 0.75,
    origin: "刺竹林",
    description: "以盛水青竹筒投入鮮筍、竹耳或雞肉絲菇於炭火旁慢煨而成的熱湯，竹膜清香盡入湯中，溫潤滋補。",
    tags: ZhulinTagTriad {
        category_tags: &["food", "flora", "bamboo", "cooked"],
        attribute_tags: &["cooked_food", "hydrating", "nourishing"],
        exclusive_tag: "zl_item_bamboo_tube_soup",
    },
};

pub const ITEM_BAMBOO_WEEVIL: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_bamboo_weevil",
    name: "筍龜",
    item_type: "material",
    weight: 0.06,
    origin: "刺竹林",
    description: "棲息於破土春筍間吸食筍汁長大的紅褐大竹象鼻蟲，體態圓潤肥美，穿於竹枝上火烤後脂香四溢。",
    tags: ZhulinTagTriad {
        category_tags: &["food", "fauna", "bamboo", "insect"],
        attribute_tags: &["roastable", "protein_rich"],
        exclusive_tag: "zl_item_bamboo_weevil",
    },
};

pub const ITEM_ROASTED_WEEVIL: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_roasted_weevil",
    name: "焦香烤筍龜",
    item_type: "food",
    weight: 0.05,
    origin: "刺竹林",
    description: "以乾竹葉炭火翻烤至甲殼金紅酥脆的筍龜，咬開後滿口濃郁甘甜脂香，能迅速補充山林跋涉消耗的體力。",
    tags: ZhulinTagTriad {
        category_tags: &["food", "fauna", "bamboo", "cooked"],
        attribute_tags: &["cooked_food", "edible", "protein_rich"],
        exclusive_tag: "zl_item_roasted_weevil",
    },
};

pub const ITEM_TERMITE_MUSHROOM: ZhulinItemDef = ZhulinItemDef {
    id: "zl_item_termite_mushroom",
    name: "雞肉絲菇",
    item_type: "food",
    weight: 0.08,
    origin: "刺竹林",
    description: "雨後自刺竹林腐葉與白蟻巢上方破土而出的傘菌，菌蓋尖凸、菌柄肥厚潔白，撕開如雞肉細絲般鮮甜。",
    tags: ZhulinTagTriad {
        category_tags: &["food", "fungi", "bamboo", "mushroom"],
        attribute_tags: &["edible", "roastable", "soup_ingredient", "sweet_umami"],
        exclusive_tag: "zl_item_termite_mushroom",
    },
};

/// 採集檢定結果（由場上物件阻力標籤與玩家行囊工具能力標籤對撞決定）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatherCheckResult {
    Allowed {
        stock_cost: u32,
        yield_qty: u32,
        custom_narrative: Option<String>,
    },
    Blocked {
        narrative: String,
    },
}

fn parse_bag_entries(inventory_json: &str) -> Vec<InventoryEntry> {
    serde_json::from_str::<Vec<InventoryEntry>>(inventory_json).unwrap_or_default()
}

/// 檢查玩家行囊中是否持有具備指定 `category_tags` 或 `attribute_tags` 的新世界物品
#[must_use]
pub fn inventory_has_capability_tag(inventory_json: &str, required_tag: &str) -> bool {
    let entries = parse_bag_entries(inventory_json);
    entries.iter().any(|e| {
        e.qty > 0
            && lookup_zhulin_item_def(&e.item_id)
                .is_some_and(|def| def.tags.has_tag(required_tag))
    })
}

fn find_item_id_by_tag(entries: &[InventoryEntry], tag: &str, min_qty: i32) -> Option<String> {
    entries
        .iter()
        .find(|e| {
            e.qty >= min_qty
                && lookup_zhulin_item_def(&e.item_id).is_some_and(|def| def.tags.has_tag(tag))
        })
        .map(|e| e.item_id.clone())
}

fn find_item_with_tag(entries: &[InventoryEntry], tag: &str, min_qty: i32) -> Option<String> {
    find_item_id_by_tag(entries, tag, min_qty)
}

/// 依場上採集物件 ID、當前存量與玩家行囊 JSON，執行三階標籤對撞採集檢定（委派至天氣連動檢定）
#[must_use]
pub fn evaluate_zhulin_gather(
    obj_id: &str,
    available_qty: u32,
    inventory_json: &str,
) -> GatherCheckResult {
    let weather_def = crate::grid::zhulin::weather::current_world_zhulin_weather().0;
    evaluate_zhulin_gather_with_weather(
        obj_id,
        available_qty,
        inventory_json,
        weather_def.tags.attribute_tags,
    )
}

/// 依場上採集物件 ID、當前存量、玩家行囊 JSON 與當前環境氣候屬性標籤，執行環境連動三階標籤對撞採集檢定
#[must_use]
pub fn evaluate_zhulin_gather_with_weather(
    obj_id: &str,
    available_qty: u32,
    inventory_json: &str,
    weather_tags: &[&str],
) -> GatherCheckResult {
    match obj_id.trim() {
        // 第 2 階：【合抱刺竹叢】竹壁堅厚帶刺，必須持有具備 `heavy_chop` 屬性標籤之砍劈工具
        bamboo_culm::ID => {
            if inventory_has_capability_tag(inventory_json, "heavy_chop") {
                GatherCheckResult::Allowed {
                    stock_cost: 1,
                    yield_qty: 1,
                    custom_narrative: None,
                }
            } else {
                GatherCheckResult::Blocked {
                    narrative: bamboo_culm::blocked_gather_narrative(),
                }
            }
        }
        // 第 1.5 階：【盤錯竹鞭】鬚根深鎖紅土，必須持有 `sharp_edge`、`earth_dig` 或 `heavy_chop` 工具
        bamboo_rhizome::ID => {
            if inventory_has_capability_tag(inventory_json, "sharp_edge")
                || inventory_has_capability_tag(inventory_json, "earth_dig")
                || inventory_has_capability_tag(inventory_json, "heavy_chop")
            {
                GatherCheckResult::Allowed {
                    stock_cost: 1,
                    yield_qty: 1,
                    custom_narrative: None,
                }
            } else {
                GatherCheckResult::Blocked {
                    narrative: bamboo_rhizome::blocked_gather_narrative(),
                }
            }
        }
        // 第 1 階：【破土竹筍】淺土帶刺，持 `pry_edge` / `earth_dig` / `sharp_edge` 無損連根掘取（耗1得1）；
        // 若環境處於「雨後初霽」（具備 `moist_soil` 屬性標籤），紅土鬆軟，徒手亦可無損拔出（耗1得1）；
        // 否則徒手硬掰時若存量 >= 2 則折損筍身（耗2得1），若僅剩 1 株孤筍則擋回避免折斷
        bamboo_shoot::ID => {
            if inventory_has_capability_tag(inventory_json, "pry_edge")
                || inventory_has_capability_tag(inventory_json, "earth_dig")
                || inventory_has_capability_tag(inventory_json, "sharp_edge")
            {
                GatherCheckResult::Allowed {
                    stock_cost: 1,
                    yield_qty: 1,
                    custom_narrative: None,
                }
            } else if weather_tags.contains(&"moist_soil") {
                GatherCheckResult::Allowed {
                    stock_cost: 1,
                    yield_qty: 1,
                    custom_narrative: Some(
                        "雨後紅土濕軟鬆動，你順著筍尖輕輕一拔，便將整株鮮嫩的「刺竹筍」連根完整拔出，毫無折損！"
                            .to_string(),
                    ),
                }
            } else if available_qty >= 2 {
                let rem = available_qty - 2;
                GatherCheckResult::Allowed {
                    stock_cost: 2,
                    yield_qty: 1,
                    custom_narrative: Some(bamboo_shoot::barehand_lossy_gather_narrative(rem)),
                }
            } else {
                GatherCheckResult::Blocked {
                    narrative: bamboo_shoot::blocked_lone_shoot_narrative(),
                }
            }
        }
        // 第 0 階：乾竹葉、枯黃竹籜、竹根木耳與溪石泉水等皆可徒手直接拾取（耗1得1）
        _ => GatherCheckResult::Allowed {
            stock_cost: 1,
            yield_qty: 1,
            custom_narrative: None,
        },
    }
}

/// 檢定生火野炊時當前氣候與地形掩蔽標籤的阻力與庇護效果
pub fn evaluate_fire_weather_shelter(
    weather_tags: &[&str],
    cell_tags: &[String],
    has_pop_bamboo: bool,
    tinder_qty: i32,
) -> Result<Option<&'static str>, String> {
    if weather_tags.contains(&"wetting")
        && !cell_tags
            .iter()
            .any(|t| t == "canopy_shelter" || t == "wind_shelter")
    {
        return Err(
            "【山雨霏微】細雨綿綿，露天潮濕無法引火；請步入尚存合抱刺竹叢的【刺竹林】（竹冠遮雨）或避風岩龕下方可生火。"
                .to_string(),
        );
    }
    if weather_tags.contains(&"gale_force")
        && !cell_tags
            .iter()
            .any(|t| t == "windbreak" || t == "wind_shelter")
    {
        return Err(
            "【九降疾風】強風呼嘯吹散火苗，空曠處無法引火；請步入尚存合抱刺竹叢的【刺竹林】（防風林障）或避風處方可生火。"
                .to_string(),
        );
    }
    if !has_pop_bamboo && weather_tags.contains(&"dew_condensing") && tinder_qty < 3 {
        return Err(
            "【晨霧凝露】林間濃霧凝露使表層乾葉微潮，單靠乾竹葉引火需至少 3 把（或搭配 1 根刺竹稈觸發竹爆營火）。"
                .to_string(),
        );
    }
    if weather_tags.contains(&"wetting") {
        return Ok(Some("濃密刺竹林冠替你擋去了漫天細雨，"));
    }
    if weather_tags.contains(&"gale_force") {
        return Ok(Some("合抱刺竹叢替你擋下了呼嘯九降風，"));
    }
    Ok(None)
}

/// 依新世界物品三層標籤判定行囊顯示階段與手作按鈕名稱（完全由標籤驅動）
#[must_use]
pub fn classify_zhulin_item_capabilities(item_id: &str) -> Option<(String, bool, String)> {
    let def = lookup_zhulin_item_def(item_id)?;
    match def.id {
        "zl_item_aged_bamboo_slat" => Some(("raw".to_string(), true, "烘烤竹刃".to_string())),
        "zl_item_bamboo_sheath" => Some(("raw".to_string(), true, "搓製籜繩".to_string())),
        "zl_item_bamboo_rhizome" => Some(("raw".to_string(), true, "紮製槌斧".to_string())),
        "zl_item_bamboo_culm" => Some(("raw".to_string(), true, "裁器編簍".to_string())),
        _ if def.tags.has_tag("tinder") => Some(("raw".to_string(), true, "引火竹爆".to_string())),
        "zl_item_bamboo_shoot"
        | "zl_item_bamboo_fungus"
        | "zl_item_termite_mushroom"
        | "zl_item_bamboo_weevil" => Some(("raw".to_string(), true, "竹火野炊".to_string())),
        "zl_item_bamboo_canteen" => Some(("finished".to_string(), true, "接雨盛水".to_string())),
        _ if def.tags.has_tag("knappable") => {
            Some(("raw".to_string(), true, "敲擊打片".to_string()))
        }
        "zl_item_sharp_stone_flake" => Some(("raw".to_string(), true, "紮製石斧".to_string())),
        _ => Some(("finished".to_string(), false, String::new())),
    }
}
fn ignite_cell_campfire(gx: i32, gy: i32) {
    crate::server::grid_manager::with_square_grid_mut(|grid| {
        if let Some(cell) = grid.get_mut(crate::grid::SquareCoord::new(gx, gy)) {
            if !cell.has_room_tag("zl_campfire:active") {
                cell.tags.push("zl_campfire:active".to_string());
            }
            if let Some(cf) = cell.objects.iter_mut().find(|o| o.id == super::campfire::ID) {
                cf.quantity = cf.quantity.max(3);
            } else {
                cell.objects.push(super::campfire::build_grid_object(3));
            }
            crate::grid::zhulin::refresh_bamboo_cell_after_gather(cell);
            let _ = crate::db::update_cell_state(gx, gy, cell.explored, cell.freshness, &cell.objects);
        }
    });
}

/// 執行新世界標籤驅動手作／野炊／引火動作（支援 `item_id:recipe` 顯式自選配方後綴）
pub fn try_craft_zhulin_item(
    pid: &str,
    raw_item_id: &str,
) -> Option<Result<(String, String, String), String>> {
    let (item_id, recipe_opt) = match raw_item_id.split_once(':') {
        Some((base, r)) => (base.trim(), Some(r.trim())),
        None => (raw_item_id.trim(), None),
    };
    let def = lookup_zhulin_item_def(item_id)?;
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Some(Err("查無角色狀態，無法執行工序。".to_string()));
    };
    let entries = parse_bag_entries(&ent.inventory);
    let (w_def, _) = crate::grid::zhulin::weather::current_world_zhulin_weather();
    let weather_tags = w_def.tags.attribute_tags;
    let (gx, gy) = ent.grid_x.zip(ent.grid_y).unwrap_or((ent.x, ent.y));
    let cell_tags: Vec<String> = crate::server::grid_manager::get_runtime_grid_cell(
        crate::grid::SquareCoord::new(gx, gy),
    )
    .map(|c| c.tags)
    .unwrap_or_default();
    let has_cell_campfire = cell_tags.iter().any(|t| t == "zl_campfire:active");

    // 0a. `heat_hardenable`（風折老竹片）× `tinder`（乾竹葉或格內營火）-> 烘烤竹青硬化為【烤刃竹片刀】
    if def.id == "zl_item_aged_bamboo_slat" || def.tags.has_tag("heat_hardenable") {
        let tinder_id_opt = find_item_with_tag(&entries, "tinder", 1);
        if !has_cell_campfire && tinder_id_opt.is_none() {
            return Some(Err(
                "烘烤硬化老竹片需生起文火，行囊中尚缺「乾竹葉」（火絨耗材）且格內尚無營火。".to_string(),
            ));
        }
        let total_tinder_qty = if has_cell_campfire {
            3
        } else {
            entries
                .iter()
                .filter(|e| lookup_zhulin_item_def(&e.item_id).is_some_and(|d| d.tags.has_tag("tinder")))
                .map(|e| e.qty)
                .sum::<i32>()
        };
        let shelter_prefix = match evaluate_fire_weather_shelter(
            weather_tags,
            &cell_tags,
            true,
            total_tinder_qty,
        ) {
            Ok(prefix) => prefix.unwrap_or(""),
            Err(msg) => return Some(Err(msg)),
        };
        if db::remove_from_inventory(pid, item_id, 1).is_err() {
            return Some(Err("行囊中的「風折老竹片」數量不足。".to_string()));
        }
        if !has_cell_campfire {
            if let Some(tid) = tinder_id_opt {
                let _ = db::remove_from_inventory(pid, &tid, 1);
            }
            ignite_cell_campfire(gx, gy);
        }
        let _ = db::add_to_inventory(pid, ITEM_FIRE_HARDENED_BAMBOO_KNIFE.id, 1);
        return Some(Ok((
            ITEM_FIRE_HARDENED_BAMBOO_KNIFE.id.to_string(),
            ITEM_FIRE_HARDENED_BAMBOO_KNIFE.name.to_string(),
            format!(
                "{shelter_prefix}你借竹葉文火將「風折老竹片」富含矽質的竹青外緣反覆烘烤去水，待纖維收縮硬化後刮出俐落斜刃，製成一把鋒利的「{}」！（解鎖割取竹鞭與無損撬筍能力）",
                ITEM_FIRE_HARDENED_BAMBOO_KNIFE.name
            ),
        )));
    }

    // 0b. `zl_item_bamboo_sheath`（韌竹籜 ×2）-> 徒手對搓為【竹籜繩】
    if def.id == "zl_item_bamboo_sheath" {
        let qty = entries
            .iter()
            .find(|e| e.item_id == "zl_item_bamboo_sheath")
            .map_or(0, |e| e.qty);
        if qty < 2 {
            return Some(Err(
                "搓製「竹籜繩」需至少 2 片「韌竹籜」對搓成股（可於刺竹林採集枯黃竹籜）。"
                    .to_string(),
            ));
        }
        if db::remove_from_inventory(pid, "zl_item_bamboo_sheath", 2).is_err() {
            return Some(Err("行囊中的「韌竹籜」數量不足 2 片。".to_string()));
        }
        let _ = db::add_to_inventory(pid, ITEM_SHEATH_CORD.id, 1);
        return Some(Ok((
            ITEM_SHEATH_CORD.id.to_string(),
            ITEM_SHEATH_CORD.name.to_string(),
            format!(
                "你將兩片堅韌的「韌竹籜」順著纖維撕成細條，置於掌心反覆對搓成股，絞製出一條結實耐拉的「{}」！",
                ITEM_SHEATH_CORD.name
            ),
        )));
    }

    // 1. `knappable`（如石英圓礫 ×2）-> 敲擊打片產出【銳邊石片】
    if def.tags.has_tag("knappable") {
        let qty = entries
            .iter()
            .find(|e| e.item_id == item_id)
            .map_or(0, |e| e.qty);
        if qty < 2 {
            return Some(Err(format!(
                "敲擊打片需兩塊堅硬石材互擊，行囊中的「{}」尚不足 2 份。",
                def.name
            )));
        }
        if db::remove_from_inventory(pid, item_id, 2).is_err() {
            return Some(Err("扣除石材失敗，請稍後再試。".to_string()));
        }
        let _ = db::add_to_inventory(pid, ITEM_SHARP_STONE_FLAKE.id, 1);
        return Some(Ok((
            ITEM_SHARP_STONE_FLAKE.id.to_string(),
            ITEM_SHARP_STONE_FLAKE.name.to_string(),
            format!(
                "你雙手各執一塊「{}」找準紋理猛力互擊，啪地剝離出一片刃口鋒利的「{}」收入行囊！",
                def.name, ITEM_SHARP_STONE_FLAKE.name
            ),
        )));
    }

    // 2. `zl_item_sharp_stone_flake` 或 `haft_material`（老竹根鞭）-> 紮製【籜繩竹鞭石斧】或純竹【竹鞭根槌】
    if def.id == "zl_item_sharp_stone_flake" || def.tags.has_tag("haft_material") {
        let has_stone_flake = entries
            .iter()
            .any(|e| e.item_id == "zl_item_sharp_stone_flake" && e.qty >= 1);
        let Some(haft_id) = find_item_with_tag(&entries, "haft_material", 1) else {
            return Some(Err(
                "紮製重型工具尚缺「老竹根鞭」（堅韌柄材，可持烤刃竹片刀或石片於刺竹林掘取盤錯竹鞭）。"
                    .to_string(),
            ));
        };
        let want_stone_axe = match recipe_opt {
            Some("stone_axe") => true,
            Some("club") => false,
            _ => has_stone_flake || def.id == "zl_item_sharp_stone_flake",
        };
        if want_stone_axe {
            if !has_stone_flake {
                return Some(Err(
                    "紮製「籜繩竹鞭石斧」尚缺「銳邊石片 ×1」（可由兩塊石英圓礫互擊打片，或先選「紮製竹鞭根槌」走純竹路線）。"
                        .to_string(),
                ));
            }
            let cord_or_sheath = if entries
                .iter()
                .any(|e| e.item_id == ITEM_SHEATH_CORD.id && e.qty >= 1)
            {
                Some((ITEM_SHEATH_CORD.id.to_string(), 1))
            } else {
                find_item_with_tag(&entries, "weaving_fiber", 1).map(|id| (id, 1))
            };
            let Some((fiber_id, fiber_qty)) = cord_or_sheath else {
                return Some(Err(
                    "紮製石斧尚缺「竹籜繩」或「韌竹籜」（編織纖維）用以綁縛柄刃。".to_string(),
                ));
            };
            if db::remove_from_inventory(pid, "zl_item_sharp_stone_flake", 1).is_err()
                || db::remove_from_inventory(pid, &haft_id, 1).is_err()
                || db::remove_from_inventory(pid, &fiber_id, fiber_qty).is_err()
            {
                return Some(Err("紮製石斧所需材料扣除失敗。".to_string()));
            }
            let _ = db::add_to_inventory(pid, ITEM_BAMBOO_STONE_AXE.id, 1);
            return Some(Ok((
                ITEM_BAMBOO_STONE_AXE.id.to_string(),
                ITEM_BAMBOO_STONE_AXE.name.to_string(),
                format!(
                    "你將「老竹根鞭」劈開夾口，嵌入「銳邊石片」，再以韌籜緊緊纏繞固定，紮成一把沉甸甸的「{}」！",
                    ITEM_BAMBOO_STONE_AXE.name
                ),
            )));
        }

        // 純竹路線：老竹根鞭 + 竹籜繩 ×1（或韌竹籜 ×2）-> 紮製【竹鞭根槌】（具備 heavy_chop）
        let has_cord = entries
            .iter()
            .any(|e| e.item_id == ITEM_SHEATH_CORD.id && e.qty >= 1);
        let has_two_sheaths = entries
            .iter()
            .any(|e| e.item_id == "zl_item_bamboo_sheath" && e.qty >= 2);
        if !has_cord && !has_two_sheaths {
            return Some(Err(
                "紮製「竹鞭根槌」需「老竹根鞭 ×1」搭配「竹籜繩 ×1」（或「韌竹籜 ×2」）纏柄防震。"
                    .to_string(),
            ));
        }
        if db::remove_from_inventory(pid, &haft_id, 1).is_err() {
            return Some(Err("行囊中的「老竹根鞭」數量不足。".to_string()));
        }
        if has_cord {
            let _ = db::remove_from_inventory(pid, ITEM_SHEATH_CORD.id, 1);
        } else {
            let _ = db::remove_from_inventory(pid, "zl_item_bamboo_sheath", 2);
        }
        let _ = db::add_to_inventory(pid, ITEM_RHIZOME_CLUB.id, 1);
        return Some(Ok((
            ITEM_RHIZOME_CLUB.id.to_string(),
            ITEM_RHIZOME_CLUB.name.to_string(),
            format!(
                "你挑選實心密節的「老竹根鞭」，將粗壯如鐵的根瘤端打磨成型，並以竹籜繩死死纏緊握柄防滑，紮成一把沉重耐砸的「{}」！（具備重擊劈砍能力，可伐合抱刺竹叢）",
                ITEM_RHIZOME_CLUB.name
            ),
        )));
    }

    // 3. `hollow_jointed` + `splittable`（刺竹稈）-> 支援 `:canteen_spade`（水筒與竹鏟）、`:basket`（刺竹背簍）、`:fire_pop`（投火竹爆）
    if def.tags.has_tag("hollow_jointed") && def.tags.has_tag("splittable") {
        if recipe_opt == Some("fire_pop") {
            let tinder_opt = find_item_with_tag(&entries, "tinder", 1);
            if !has_cell_campfire && tinder_opt.is_none() {
                return Some(Err(
                    "執行「投火竹爆」需格內已有營火，或行囊備有至少 1 把「乾竹葉」引燃。".to_string(),
                ));
            }
            let shelter_prefix = match evaluate_fire_weather_shelter(
                weather_tags,
                &cell_tags,
                true,
                3,
            ) {
                Ok(prefix) => prefix.unwrap_or(""),
                Err(msg) => return Some(Err(msg)),
            };
            if db::remove_from_inventory(pid, item_id, 1).is_err() {
                return Some(Err("行囊中的「刺竹稈」數量不足。".to_string()));
            }
            if !has_cell_campfire {
                if let Some(tid) = tinder_opt {
                    let _ = db::remove_from_inventory(pid, &tid, 1);
                }
            }
            ignite_cell_campfire(gx, gy);
            let new_vit = (ent.vit + 12).min(100);
            if new_vit != ent.vit {
                let _ = db::update_vit(pid, new_vit);
            }
            return Some(Ok((
                item_id.to_string(),
                "投火竹爆".to_string(),
                format!(
                    "{shelter_prefix}你將一節中空「{}」投入竹葉營火中——受熱膨脹的竹節猛然炸出「砰！」地一聲巨響，林間寒濕瘴氣頓時消散，周遭窺伺的野獸亦驚惶遠遁！（氣力略微恢復）",
                    def.name
                ),
            )));
        }

        let has_canteen = entries.iter().any(|e| {
            (e.item_id == ITEM_BAMBOO_CANTEEN.id || e.item_id == ITEM_BAMBOO_CANTEEN_FULL.id)
                && e.qty > 0
        });
        let has_spade = entries
            .iter()
            .any(|e| e.item_id == ITEM_BAMBOO_SPADE.id && e.qty > 0);
        let has_basket = entries
            .iter()
            .any(|e| e.item_id == ITEM_BAMBOO_BASKET.id && e.qty > 0);

        let want_basket = match recipe_opt {
            Some("basket") => true,
            Some("canteen_spade") => false,
            _ => has_canteen && has_spade && !has_basket,
        };

        if want_basket {
            let has_cord = entries
                .iter()
                .any(|e| e.item_id == ITEM_SHEATH_CORD.id && e.qty >= 1);
            let has_two_sheaths = entries
                .iter()
                .any(|e| e.item_id == "zl_item_bamboo_sheath" && e.qty >= 2);
            if !has_cord && !has_two_sheaths {
                return Some(Err(
                    "劈篾編織「刺竹背簍」需「刺竹稈 ×1」搭配「竹籜繩 ×1」（或「韌竹籜 ×2」）結成雙肩背帶。"
                        .to_string(),
                ));
            }
            if db::remove_from_inventory(pid, item_id, 1).is_err() {
                return Some(Err("行囊中的「刺竹稈」數量不足。".to_string()));
            }
            if has_cord {
                let _ = db::remove_from_inventory(pid, ITEM_SHEATH_CORD.id, 1);
            } else {
                let _ = db::remove_from_inventory(pid, "zl_item_bamboo_sheath", 2);
            }
            let _ = db::add_to_inventory(pid, ITEM_BAMBOO_BASKET.id, 1);
            return Some(Ok((
                ITEM_BAMBOO_BASKET.id.to_string(),
                ITEM_BAMBOO_BASKET.name.to_string(),
                format!(
                    "你將「{}」順節劈開為勻稱青竹篾，以十字編法打底、六角孔編起簍身，再穿入竹籜繩結成雙肩背帶，編出一只輕巧堅固的「{}」！（行囊負重上限提升 15 公斤）",
                    def.name, ITEM_BAMBOO_BASKET.name
                ),
            )));
        }

        if db::remove_from_inventory(pid, item_id, 1).is_err() {
            return Some(Err("行囊中的竹稈數量不足。".to_string()));
        }
        let _ = db::add_to_inventory(pid, ITEM_BAMBOO_CANTEEN.id, 1);
        let _ = db::add_to_inventory(pid, ITEM_BAMBOO_SPADE.id, 1);
        return Some(Ok((
            ITEM_BAMBOO_SPADE.id.to_string(),
            format!("{}、{}", ITEM_BAMBOO_CANTEEN.name, ITEM_BAMBOO_SPADE.name),
            format!(
                "你依循「{}」中空留節與縱向易劈的物性，上半截留底節裁成一具「{}」，下半截厚壁斜削為一把「{}」收入行囊！",
                def.name, ITEM_BAMBOO_CANTEEN.name, ITEM_BAMBOO_SPADE.name
            ),
        )));
    }

    // 3b. `zl_item_bamboo_canteen`（空青竹水筒）-> 接雨／集露／取泉盛滿為【盛水青竹筒】
    if def.id == ITEM_BAMBOO_CANTEEN.id {
        let can_collect_water = weather_tags.contains(&"rain_harvestable")
            || weather_tags.contains(&"dew_condensing")
            || weather_tags.contains(&"moist_soil")
            || cell_tags
                .iter()
                .any(|t| t == "waterside" || t == "water_source" || t == "moist_soil");
        if !can_collect_water {
            return Some(Err(
                "目前天晴乾燥且非近水竹蔭，無雨水或泉露可接；待山雨、晨霧、雨後或步入近水竹林即可盛水。"
                    .to_string(),
            ));
        }
        if db::remove_from_inventory(pid, ITEM_BAMBOO_CANTEEN.id, 1).is_err() {
            return Some(Err("行囊中無可用的「青竹水筒」。".to_string()));
        }
        let _ = db::add_to_inventory(pid, ITEM_BAMBOO_CANTEEN_FULL.id, 1);
        return Some(Ok((
            ITEM_BAMBOO_CANTEEN_FULL.id.to_string(),
            ITEM_BAMBOO_CANTEEN_FULL.name.to_string(),
            format!(
                "你將「青竹水筒」斜倚於竹節滴水匯聚處，清冽甘甜的水珠順著竹青凹槽注入筒內，盛滿了一筒「{}」！（可直接飲用，或留作竹筒山珍湯底水）",
                ITEM_BAMBOO_CANTEEN_FULL.name
            ),
        )));
    }

    // 4. `kindling`（乾竹葉）＋ 選配 `flammable_pop`（刺竹稈）-> 引火落地為 `zl_campfire:active` 格態
    if def.tags.has_tag("tinder") {
        let total_tinder_qty = entries
            .iter()
            .filter(|e| lookup_zhulin_item_def(&e.item_id).is_some_and(|d| d.tags.has_tag("tinder")))
            .map(|e| e.qty)
            .sum::<i32>();
        let pop_culm_id = if recipe_opt == Some("camp_fire") {
            None
        } else {
            find_item_with_tag(&entries, "flammable_pop", 1)
        };

        if total_tinder_qty < 2 && pop_culm_id.is_none() {
            return Some(Err(format!(
                "引火需至少 2 把「{}」（或搭配 1 根刺竹稈），目前行囊火絨不足。",
                def.name
            )));
        }

        let shelter_prefix = match evaluate_fire_weather_shelter(
            weather_tags,
            &cell_tags,
            pop_culm_id.is_some(),
            total_tinder_qty,
        ) {
            Ok(prefix) => prefix.unwrap_or(""),
            Err(msg) => return Some(Err(msg)),
        };

        let is_dew_damp = weather_tags.contains(&"dew_condensing") && pop_culm_id.is_none();
        let consume_tinder_qty = if pop_culm_id.is_some() {
            1
        } else if is_dew_damp {
            3
        } else {
            2
        };
        if db::remove_from_inventory(pid, item_id, consume_tinder_qty).is_err() {
            return Some(Err("扣除引火材料失敗。".to_string()));
        }
        ignite_cell_campfire(gx, gy);
        let mut narr = if let Some(culm_id) = pop_culm_id {
            let _ = db::remove_from_inventory(pid, &culm_id, 1);
            format!(
                "{shelter_prefix}你引燃「{}」生起營火，並將一節中空刺竹稈投入火中——受熱膨脹的竹節猛然炸出「砰！」地一聲巨響，林間寒濕瘴氣頓時消散，周遭窺伺的野獸亦驚惶遠遁！",
                def.name
            )
        } else if weather_tags.contains(&"rain_harvestable")
            && find_item_with_tag(&entries, "water_container", 1).is_some()
        {
            format!(
                "{shelter_prefix}你將乾燥蓬鬆的「{}」聚攏引燃，於林下生起一簇溫暖營火；你同時將「青竹水筒」斜置於竹葉滴水處，接滿了清冽雨水，氣力恢復不少。",
                def.name
            )
        } else {
            format!(
                "{shelter_prefix}你將乾燥蓬鬆的「{}」聚攏引燃，於林下生起一簇劈啪作響的竹葉營火（同格後續野炊與烤刃可直接借火，免耗乾竹葉）。",
                def.name
            )
        };
        let new_vit = (ent.vit + 12).min(100);
        if new_vit != ent.vit {
            let _ = db::update_vit(pid, new_vit);
            narr.push_str("（氣力略微恢復）");
        }
        return Some(Ok((
            item_id.to_string(),
            "竹林營火".to_string(),
            narr,
        )));
    }

    // 5. `roastable` / `soup_ingredient`（鮮刺竹筍／野生竹耳／雞肉絲菇／筍龜）-> 支援 `:roast`（炙烤／煨筍，不耗水筒）與 `:soup`（竹筒煲湯）
    if def.tags.has_tag("roastable") || def.tags.has_tag("soup_ingredient") {
        let tinder_id_opt = find_item_with_tag(&entries, "tinder", 1);
        if !has_cell_campfire && tinder_id_opt.is_none() {
            return Some(Err(format!(
                "野炊「{}」需生火慢煨，行囊中尚缺「乾竹葉」（火絨耗材）且格內尚無營火。",
                def.name
            )));
        }
        let total_tinder_qty = if has_cell_campfire {
            3
        } else {
            entries
                .iter()
                .filter(|e| lookup_zhulin_item_def(&e.item_id).is_some_and(|d| d.tags.has_tag("tinder")))
                .map(|e| e.qty)
                .sum::<i32>()
        };
        let has_culm = has_cell_campfire || find_item_with_tag(&entries, "flammable_pop", 1).is_some();

        let shelter_prefix = match evaluate_fire_weather_shelter(
            weather_tags,
            &cell_tags,
            has_culm,
            total_tinder_qty,
        ) {
            Ok(prefix) => prefix.unwrap_or(""),
            Err(msg) => return Some(Err(msg)),
        };

        // 5a. 筍龜 -> 焦香烤筍龜
        if def.id == ITEM_BAMBOO_WEEVIL.id {
            if db::remove_from_inventory(pid, item_id, 1).is_err() {
                return Some(Err("行囊中的「筍龜」數量不足。".to_string()));
            }
            if !has_cell_campfire {
                if let Some(tid) = tinder_id_opt {
                    let _ = db::remove_from_inventory(pid, &tid, 1);
                }
                ignite_cell_campfire(gx, gy);
            }
            let _ = db::add_to_inventory(pid, ITEM_ROASTED_WEEVIL.id, 1);
            return Some(Ok((
                ITEM_ROASTED_WEEVIL.id.to_string(),
                ITEM_ROASTED_WEEVIL.name.to_string(),
                format!(
                    "{shelter_prefix}你將肥美的「筍龜」穿在細竹枝上，架於竹葉炭火上方翻烤，甲殼漸轉金紅、油脂滋滋作響，烤出一串香氣撲鼻的「{}」！",
                    ITEM_ROASTED_WEEVIL.name
                ),
            )));
        }

        let has_full_canteen = entries
            .iter()
            .any(|e| e.item_id == ITEM_BAMBOO_CANTEEN_FULL.id && e.qty >= 1);
        let want_soup = match recipe_opt {
            Some("soup") => true,
            Some("roast") => false,
            _ => has_full_canteen,
        };

        // 5b. 竹筒煲湯（消耗【盛水青竹筒】-> 產出【竹筒山珍湯】並歸還空【青竹水筒】）
        if want_soup {
            if !has_full_canteen {
                return Some(Err(
                    "煲煮「竹筒山珍湯」需行囊備有「盛水青竹筒 ×1」（可先持空青竹水筒於近水竹蔭或雨露時盛水，或改選炙烤／煨筍）。"
                        .to_string(),
                ));
            }
            if db::remove_from_inventory(pid, item_id, 1).is_err()
                || db::remove_from_inventory(pid, ITEM_BAMBOO_CANTEEN_FULL.id, 1).is_err()
            {
                return Some(Err("煲湯所需食材或盛水竹筒扣除失敗。".to_string()));
            }
            if !has_cell_campfire {
                if let Some(tid) = tinder_id_opt {
                    let _ = db::remove_from_inventory(pid, &tid, 1);
                }
                ignite_cell_campfire(gx, gy);
            }
            let _ = db::add_to_inventory(pid, ITEM_BAMBOO_TUBE_SOUP.id, 1);
            let _ = db::add_to_inventory(pid, ITEM_BAMBOO_CANTEEN.id, 1);
            return Some(Ok((
                ITEM_BAMBOO_TUBE_SOUP.id.to_string(),
                ITEM_BAMBOO_TUBE_SOUP.name.to_string(),
                format!(
                    "{shelter_prefix}你將「{}」投入「盛水青竹筒」中，以乾竹葉塞口斜架於炭火旁慢煨。竹壁內的水分與竹膜清香漸次沸騰交融，煲出一筒熱氣騰騰的「{}」（空青竹水筒亦完好留存）！",
                    def.name, ITEM_BAMBOO_TUBE_SOUP.name
                ),
            )));
        }

        // 5c. 野生竹耳／雞肉絲菇 `:roast` 炭火炙烤（不消耗盛水竹筒，直接產出籜裹煨山珍／籜裹煨竹筍）
        let wrap_id = find_item_with_tag(&entries, "wrapping_sheet", 1);
        if db::remove_from_inventory(pid, item_id, 1).is_err() {
            return Some(Err("扣除食材失敗。".to_string()));
        }
        if !has_cell_campfire {
            if let Some(tid) = tinder_id_opt {
                let _ = db::remove_from_inventory(pid, &tid, 1);
            }
            ignite_cell_campfire(gx, gy);
        }
        let narr = if let Some(w_id) = wrap_id {
            let _ = db::remove_from_inventory(pid, &w_id, 1);
            format!(
                "{shelter_prefix}你以防潮「韌竹籜」將「{}」層層裹緊，埋入竹葉營火餘燼中慢煨至熟（未消耗竹筒飲水），剝開時香氣四溢，製成一份「{}」！",
                def.name, ITEM_ROASTED_BAMBOO_SHOOT.name
            )
        } else {
            format!(
                "{shelter_prefix}你將「{}」置於竹葉營火旁慢炙至熟（未消耗竹筒飲水），去除了生澀濕氣，製成一份香氣溫潤的「{}」收入行囊。",
                def.name, ITEM_ROASTED_BAMBOO_SHOOT.name
            )
        };
        let _ = db::add_to_inventory(pid, ITEM_ROASTED_BAMBOO_SHOOT.id, 1);
        return Some(Ok((
            ITEM_ROASTED_BAMBOO_SHOOT.id.to_string(),
            ITEM_ROASTED_BAMBOO_SHOOT.name.to_string(),
            narr,
        )));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_zhulin_gather_with_weather_moist_soil() {
        let empty_bag = "[]";
        // 一般天氣徒手採孤筍：擋回
        let normal_weather: &[&str] = &["clear_visibility"];
        let res_normal = evaluate_zhulin_gather_with_weather(bamboo_shoot::ID, 1, empty_bag, normal_weather);
        assert!(matches!(res_normal, GatherCheckResult::Blocked { .. }));

        // 雨後初霽（moist_soil）：徒手採孤筍無損完整拔出
        let rain_mist_weather: &[&str] = &["moist_soil", "fungus_blooming"];
        let res_rain = evaluate_zhulin_gather_with_weather(bamboo_shoot::ID, 1, empty_bag, rain_mist_weather);
        match res_rain {
            GatherCheckResult::Allowed {
                stock_cost,
                yield_qty,
                custom_narrative,
            } => {
                assert_eq!(stock_cost, 1);
                assert_eq!(yield_qty, 1);
                assert!(custom_narrative.unwrap().contains("雨後紅土濕軟鬆動"));
            }
            GatherCheckResult::Blocked { .. } => panic!("雨後初霽應允許徒手掘筍"),
        }
    }

    #[test]
    fn test_evaluate_fire_weather_shelter() {
        let rain_weather: &[&str] = &["wetting", "rain_harvestable"];
        let gale_weather: &[&str] = &["gale_force", "air_drying"];
        let fog_weather: &[&str] = &["dew_condensing", "cool_damp"];
        let clear_weather: &[&str] = &["solar_warming", "air_drying"];

        let open_cell_tags: Vec<String> = vec!["open_ground".to_string()];
        let shelter_cell_tags: Vec<String> = vec!["canopy_shelter".to_string(), "windbreak".to_string()];

        // 雨天露天生火阻斷
        assert!(evaluate_fire_weather_shelter(rain_weather, &open_cell_tags, false, 2).is_err());
        // 雨天林冠庇護生火成功
        let rain_sheltered = evaluate_fire_weather_shelter(rain_weather, &shelter_cell_tags, false, 2);
        assert!(rain_sheltered.is_ok());
        assert_eq!(rain_sheltered.unwrap(), Some("濃密刺竹林冠替你擋去了漫天細雨，"));

        // 強風空曠生火阻斷
        assert!(evaluate_fire_weather_shelter(gale_weather, &open_cell_tags, false, 2).is_err());
        // 強風防風障壁庇護生火成功
        let gale_sheltered = evaluate_fire_weather_shelter(gale_weather, &shelter_cell_tags, false, 2);
        assert!(gale_sheltered.is_ok());
        assert_eq!(gale_sheltered.unwrap(), Some("合抱刺竹叢替你擋下了呼嘯九降風，"));

        // 晨霧凝露乾葉不足3把且無竹稈
        assert!(evaluate_fire_weather_shelter(fog_weather, &open_cell_tags, false, 2).is_err());
        // 晨霧凝露有3把乾葉
        assert!(evaluate_fire_weather_shelter(fog_weather, &open_cell_tags, false, 3).is_ok());
        // 晨霧凝露有竹稈
        assert!(evaluate_fire_weather_shelter(fog_weather, &open_cell_tags, true, 1).is_ok());

        // 晴朗天氣無阻礙
        let clear_res = evaluate_fire_weather_shelter(clear_weather, &open_cell_tags, false, 2);
        assert_eq!(clear_res.unwrap(), None);
    }

    #[test]
    fn test_campfire_plugin_and_socket_contract() {
        use crate::grid::zhulin::{
            campfire, lookup_zhulin_object_tags, resolve_bamboo_slot_object_with_qty,
            scene_sentence_for_object,
        };
        let cf_obj = campfire::build_grid_object(3);
        assert_eq!(cf_obj.id, "zl_bamboo_campfire");
        assert_eq!(cf_obj.name, "竹葉營火");
        let tags = lookup_zhulin_object_tags(campfire::ID).expect("campfire tags");
        assert!(tags.has_tag("heat_source"));
        assert!(tags.has_tag("socket_host"));
        assert!(tags.has_tag("ember_bed"));
        assert!(tags.has_tag("beast_deterrent"));
        let sentence = scene_sentence_for_object(&cf_obj).expect("scene sentence");
        assert!(sentence.contains("竹葉營火"));
        let room_obj = resolve_bamboo_slot_object_with_qty(campfire::ID, 3).expect("room obj");
        assert!(room_obj.sockets.contains(&"WarmHands".to_string()));
        assert!(room_obj.sockets.contains(&"CampfireRoastKnife".to_string()));
        assert!(room_obj.sockets.contains(&"CampfireRoastShoot".to_string()));
        assert!(room_obj.sockets.contains(&"CampfireBoilSoup".to_string()));
    }
}
