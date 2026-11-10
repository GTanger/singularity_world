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
    if def.tags.has_tag("knappable") {
        Some(("raw".to_string(), true, "敲擊打片".to_string()))
    } else if def.tags.has_tag("cutting_flake") || def.tags.has_tag("haft_material") {
        Some(("raw".to_string(), true, "紮製石斧".to_string()))
    } else if def.tags.has_tag("hollow_jointed") && def.tags.has_tag("splittable") {
        Some(("raw".to_string(), true, "裁製竹器".to_string()))
    } else if def.tags.has_tag("tinder") {
        Some(("raw".to_string(), true, "引火竹爆".to_string()))
    } else if def.tags.has_tag("roastable") || def.tags.has_tag("soup_ingredient") {
        Some(("raw".to_string(), true, "煨烤熟食".to_string()))
    } else {
        Some(("finished".to_string(), false, String::new()))
    }
}

/// 執行新世界標籤驅動手作／野炊／引火動作
pub fn try_craft_zhulin_item(
    pid: &str,
    item_id: &str,
) -> Option<Result<(String, String, String), String>> {
    let def = lookup_zhulin_item_def(item_id)?;
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Some(Err("查無角色狀態，無法執行工序。".to_string()));
    };
    let entries = parse_bag_entries(&ent.inventory);

    // 1. 敲擊打片（`knappable` 石材 x2 -> 銳邊石片）
    if def.tags.has_tag("knappable") {
        let Some(stone_id) = find_item_id_by_tag(&entries, "knappable", 2) else {
            return Some(Err(
                "敲擊打片需行囊內至少有 2 塊堅硬圓礫互擊，方可剝離出銳邊石片。".to_string(),
            ));
        };
        if db::remove_from_inventory(pid, &stone_id, 2).is_err() {
            return Some(Err("行囊內石材數量不足。".to_string()));
        }
        let _ = db::add_to_inventory(pid, ITEM_SHARP_STONE_FLAKE.id, 1);
        return Some(Ok((
            ITEM_SHARP_STONE_FLAKE.id.to_string(),
            ITEM_SHARP_STONE_FLAKE.name.to_string(),
            "你雙手各執一塊石英圓礫猛力斜擊，伴隨清脆石響與微星，剝離出一片刃口鋒利的「銳邊石片」收入行囊（可用於撬取竹筍與斬斷老竹鬚根）。".to_string(),
        )));
    }

    // 2. 紮製石斧（`cutting_flake` 刃口 + `haft_material` 抗震柄材 + `weaving_fiber` / `fiber` 韌材 -> 籜繩竹鞭石斧）
    if def.tags.has_tag("cutting_flake") || def.tags.has_tag("haft_material") {
        let Some(blade_id) = find_item_id_by_tag(&entries, "cutting_flake", 1) else {
            return Some(Err(
                "紮製石斧缺少刃口：需先以兩塊石英圓礫「敲擊打片」取得「銳邊石片」。".to_string(),
            ));
        };
        let Some(haft_id) = find_item_id_by_tag(&entries, "haft_material", 1) else {
            return Some(Err(
                "紮製石斧缺少抗震斧柄：請持銳邊石片至竹林掘取一段實心的「老竹根鞭」。".to_string(),
            ));
        };
        let fiber_id = find_item_id_by_tag(&entries, "weaving_fiber", 1)
            .or_else(|| find_item_id_by_tag(&entries, "fiber", 1));
        let Some(cord_id) = fiber_id else {
            return Some(Err(
                "紮製石斧缺少捆綁韌材：請於竹林拾取「韌竹籜」或採集韌草莖以搓繩紮縛。".to_string(),
            ));
        };
        let _ = db::remove_from_inventory(pid, &blade_id, 1);
        let _ = db::remove_from_inventory(pid, &haft_id, 1);
        let _ = db::remove_from_inventory(pid, &cord_id, 1);
        let _ = db::add_to_inventory(pid, ITEM_BAMBOO_STONE_AXE.id, 1);
        return Some(Ok((
            ITEM_BAMBOO_STONE_AXE.id.to_string(),
            ITEM_BAMBOO_STONE_AXE.name.to_string(),
            "你將銳邊石片嵌進老竹根鞭的劈口，再以韌竹籜撕條搓繩反覆纏緊，紮成一把沉實耐劈的「籜繩竹鞭石斧」收入行囊（現可砍伐合抱刺竹叢）！".to_string(),
        )));
    }

    // 3. 裁製竹器（`hollow_jointed` + `splittable` 刺竹稈 -> 青竹水筒 / 掘土竹鏟）
    if def.tags.has_tag("hollow_jointed") && def.tags.has_tag("splittable") {
        let has_cutter = find_item_id_by_tag(&entries, "sharp_edge", 1).is_some()
            || find_item_id_by_tag(&entries, "heavy_chop", 1).is_some();
        if !has_cutter {
            return Some(Err(
                "裁削厚壁刺竹稈需行囊內備有銳邊石片或石斧等利器。".to_string(),
            ));
        }
        if db::remove_from_inventory(pid, item_id, 1).is_err() {
            return Some(Err("行囊內刺竹稈數量不足。".to_string()));
        }
        let already_has_canteen = entries
            .iter()
            .any(|e| e.item_id == ITEM_BAMBOO_CANTEEN.id && e.qty > 0);
        let target_def = if already_has_canteen {
            &ITEM_BAMBOO_SPADE
        } else {
            &ITEM_BAMBOO_CANTEEN
        };
        let _ = db::add_to_inventory(pid, target_def.id, 1);
        let narr = if target_def.id == ITEM_BAMBOO_CANTEEN.id {
            "你持利刃順著刺竹節理截開厚壁竹稈，留底節封密、削平筒口，裁製出一只可盛泉解渴的「青竹水筒」收入行囊。".to_string()
        } else {
            "你將厚壁刺竹稈順紋斜劈削尖，修整握柄，裁削出一把輕便堅韌的「掘土竹鏟」收入行囊。".to_string()
        };
        return Some(Ok((
            target_def.id.to_string(),
            target_def.name.to_string(),
            narr,
        )));
    }

    // 4. 引火竹爆（`tinder` 乾竹葉 + `flammable_pop` 刺竹稈）
    if def.tags.has_tag("tinder") {
        let weather_def = crate::grid::zhulin::weather::current_world_zhulin_weather().0;
        let cell_tags: Vec<String> = ent
            .grid_x
            .zip(ent.grid_y)
            .and_then(|(gx, gy)| {
                crate::server::grid_manager::get_runtime_grid_cell(
                    crate::grid::SquareCoord::new(gx, gy),
                )
            })
            .map(|c| c.tags)
            .unwrap_or_default();

        let has_pop = find_item_id_by_tag(&entries, "flammable_pop", 1).is_some();
        let tinder_qty = entries
            .iter()
            .filter(|e| lookup_zhulin_item_def(&e.item_id).is_some_and(|d| d.tags.has_tag("tinder")))
            .map(|e| e.qty)
            .sum::<i32>();

        let shelter_pref = match evaluate_fire_weather_shelter(
            weather_def.tags.attribute_tags,
            &cell_tags,
            has_pop,
            tinder_qty,
        ) {
            Ok(p) => p.unwrap_or(""),
            Err(e) => return Some(Err(e)),
        };

        if let Some(pop_bamboo_id) = find_item_id_by_tag(&entries, "flammable_pop", 1) {
            let _ = db::remove_from_inventory(pid, item_id, 1);
            let _ = db::remove_from_inventory(pid, &pop_bamboo_id, 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 18).min(100);
                    e.qi = (e.qi + 22).min(100);
                });
            }
            let narr = format!(
                "{shelter_pref}你攏起乾竹葉引燃火苗，將整節未剖的「刺竹稈」投入火中。竹節內空氣受熱急遽膨脹，猛地炸開一聲震耳欲聾的「啪！轟！」竹爆巨響，火光四濺、寒濕盡散，周遭窺伺的野獸頓時驚逃遠遁！"
            );
            return Some(Ok((
                item_id.to_string(),
                "竹爆營火".to_string(),
                narr,
            )));
        }
        let req_tinder = if weather_def.tags.has_tag("dew_condensing") {
            3
        } else {
            2
        };
        let Some(tinder_id) = find_item_id_by_tag(&entries, "tinder", req_tinder) else {
            return Some(Err(
                "引火需至少 2 把「乾竹葉」，或 1 把乾竹葉搭配 1 根「刺竹稈」以觸發竹爆營火。".to_string(),
            ));
        };
        let _ = db::remove_from_inventory(pid, &tinder_id, req_tinder);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 10).min(100);
                e.qi = (e.qi + 12).min(100);
            });
        }
        let narr = format!(
            "{shelter_pref}你將 {req_tinder} 把乾竹葉交疊引燃，火苗迅速竄起，溫暖火光驅散了林間濕涼寒意。"
        );
        return Some(Ok((
            item_id.to_string(),
            "乾葉野火".to_string(),
            narr,
        )));
    }

    // 5. 煨烤熟食（`roastable` / `soup_ingredient` 竹筍或木耳 + `tinder` 乾竹葉 -> 籜裹煨竹筍）
    if def.tags.has_tag("roastable") || def.tags.has_tag("soup_ingredient") {
        let weather_def = crate::grid::zhulin::weather::current_world_zhulin_weather().0;
        let cell_tags: Vec<String> = ent
            .grid_x
            .zip(ent.grid_y)
            .and_then(|(gx, gy)| {
                crate::server::grid_manager::get_runtime_grid_cell(
                    crate::grid::SquareCoord::new(gx, gy),
                )
            })
            .map(|c| c.tags)
            .unwrap_or_default();

        let has_pop = find_item_id_by_tag(&entries, "flammable_pop", 1).is_some();
        let tinder_qty = entries
            .iter()
            .filter(|e| lookup_zhulin_item_def(&e.item_id).is_some_and(|d| d.tags.has_tag("tinder")))
            .map(|e| e.qty)
            .sum::<i32>();

        let shelter_pref = match evaluate_fire_weather_shelter(
            weather_def.tags.attribute_tags,
            &cell_tags,
            has_pop,
            tinder_qty,
        ) {
            Ok(p) => p.unwrap_or(""),
            Err(e) => return Some(Err(e)),
        };

        let Some(tinder_id) = find_item_id_by_tag(&entries, "tinder", 1) else {
            return Some(Err(
                "野炊煨烤需行囊內備有至少 1 把「乾竹葉」作為引火燃料。".to_string(),
            ));
        };
        let _ = db::remove_from_inventory(pid, item_id, 1);
        let _ = db::remove_from_inventory(pid, &tinder_id, 1);
        if let Some(wrap_id) = find_item_id_by_tag(&entries, "wrapping_sheet", 1) {
            let _ = db::remove_from_inventory(pid, &wrap_id, 1);
        }
        let _ = db::add_to_inventory(pid, ITEM_ROASTED_BAMBOO_SHOOT.id, 1);
        let narr = format!(
            "{shelter_pref}你以乾竹葉升起文火，將食材裹入竹籜埋進熱灰慢煨。焦香混著清甜筍香四溢，烤出一份熱騰騰的「籜裹煨竹筍」收入行囊。"
        );
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
}
