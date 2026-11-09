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

/// 依場上採集物件 ID、當前存量與玩家行囊 JSON，執行三階標籤對撞採集檢定
#[must_use]
pub fn evaluate_zhulin_gather(
    obj_id: &str,
    available_qty: u32,
    inventory_json: &str,
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
        // 徒手硬掰時若存量 >= 2 則折損筍身（耗2得1），若僅剩 1 株孤筍則擋回避免折斷
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
        if let Some(pop_bamboo_id) = find_item_id_by_tag(&entries, "flammable_pop", 1) {
            let _ = db::remove_from_inventory(pid, item_id, 1);
            let _ = db::remove_from_inventory(pid, &pop_bamboo_id, 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 18).min(100);
                    e.qi = (e.qi + 22).min(100);
                });
            }
            return Some(Ok((
                item_id.to_string(),
                "竹爆營火".to_string(),
                "你攏起乾竹葉引燃火苗，將整節未剖的「刺竹稈」投入火中。竹節內空氣受熱急遽膨脹，猛地炸開一聲震耳欲聾的「啪！轟！」竹爆巨響，火光四濺、寒濕盡散，周遭窺伺的野獸頓時驚逃遠遁！".to_string(),
            )));
        }
        let Some(tinder_id) = find_item_id_by_tag(&entries, "tinder", 2) else {
            return Some(Err(
                "引火需至少 2 把「乾竹葉」，或 1 把乾竹葉搭配 1 根「刺竹稈」以觸發竹爆營火。".to_string(),
            ));
        };
        let _ = db::remove_from_inventory(pid, &tinder_id, 2);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 10).min(100);
                e.qi = (e.qi + 12).min(100);
            });
        }
        return Some(Ok((
            item_id.to_string(),
            "乾葉野火".to_string(),
            "你將兩把乾燥蓬鬆的乾竹葉交疊引燃，火苗迅速竄起，溫暖火光驅散了林間濕涼寒意。".to_string(),
        )));
    }

    // 5. 煨烤熟食（`roastable` / `soup_ingredient` 竹筍或木耳 + `tinder` 乾竹葉 -> 籜裹煨竹筍）
    if def.tags.has_tag("roastable") || def.tags.has_tag("soup_ingredient") {
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
        return Some(Ok((
            ITEM_ROASTED_BAMBOO_SHOOT.id.to_string(),
            ITEM_ROASTED_BAMBOO_SHOOT.name.to_string(),
            "你以乾竹葉升起文火，將食材裹入竹籜埋進熱灰慢煨。焦香混著清甜筍香四溢，烤出一份熱騰騰的「籜裹煨竹筍」收入行囊。".to_string(),
        )));
    }

    None
}
