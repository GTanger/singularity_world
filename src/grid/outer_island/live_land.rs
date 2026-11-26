//! 銀鷗島第一刀：四季水文漲退、橋樑沖毀與公庫雇工修復、林木採伐再生、墾田休耕退化、空屋失修傾頹與動態疫氣管理。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::combat::wordplate::splitmix64;

use super::clock_and_lang::SolarClockSnapshot;
use super::island_substrate::compiled_island_substrate;
use super::live_land_tiles::{
    carve_felled_tree_tile_at_actor, prune_tile_diffs_for_cell_day, reclaim_arable_tiles_at_actor,
    resolve_actor_standing_xy, sync_river_crossings_with_substrate,
};
use super::movement::{add_carry_item, push_breath, take_clean_carry_items};
use super::types::{CarryItem, IslandWorldDelta, SpatialNodeDef, WorldStaticDoc};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CellLiveLand {
    pub flood_level: u8,
    pub has_river_crossing: bool,
    pub bridge_integrity: u8,
    pub bridge_down_days: u16,
    pub forest_density: u8,
    pub reclaimed_plots: u8,
    pub fallow_days: u16,
    pub plague_miasmas: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveLandState {
    pub seeded: bool,
    pub last_ecology_day: i64,
    pub water_stage_pct: u8,
    pub cells: BTreeMap<u32, CellLiveLand>,
    pub house_integrity: BTreeMap<String, u8>,
    #[serde(skip)]
    pub runtime_crossings_synced: bool,
}

/// 依太陽曆遊戲日計算全島季節性水位基準（`20..=98`）：
/// - 寒冬（`273..364`，含開服基準寒冬 69 日）水淺冰寒（約 `27..35`）
/// - 初春雪融與仲夏梅汛（`0..145`）水漲湧高（峰期達 `86..94`）
/// - 晚夏金秋（`145..273`）漸趨平穩（約 `40..56`）
pub fn compute_seasonal_water_stage(game_day: i64) -> u8 {
    let doy = game_day.rem_euclid(364) as i32;
    let base = if doy < 91 {
        let peak_wave = 38 - ((doy - 42).abs() * 38) / 48;
        52 + peak_wave.max(0)
    } else if doy < 145 {
        let summer_storm = 15 - ((doy - 115).abs() * 15) / 28;
        74 + summer_storm.max(0)
    } else if doy < 273 {
        42 + ((273 - doy) * 14) / 128
    } else {
        30
    };
    let wobble = ((splitmix64((game_day as u64) ^ 0x5EA5_0A11) % 7) as i32) - 3;
    (base + wobble).clamp(20, 98) as u8
}

fn is_cell_forest_habitat(c: &super::types::VoronoiCellDef) -> bool {
    c.biome == 6
        || c.biome == 9
        || matches!(
            c.resource_slug.as_str(),
            "timber_log" | "charcoal_sack" | "herb_bundle" | "beast_tallow"
        )
        || c.cell_id == 4525
}

fn cell_forest_cap(c: &super::types::VoronoiCellDef, reclaimed_plots: u8) -> u8 {
    let base = if is_cell_forest_habitat(c) {
        85u8
    } else if c.biome == 4 || c.biome == 5 || c.kind == "wilderness" {
        45u8
    } else {
        20u8
    };
    base.saturating_sub(reclaimed_plots.saturating_mul(5))
}

/// 冪等初始化全島 93 地塊之水文、橋樑、林相、疫氣與待售空屋物理狀態。
pub fn ensure_live_land_seeded(doc: &WorldStaticDoc, delta: &mut IslandWorldDelta, game_day: i64) {
    if delta.live_land.seeded {
        if !delta.live_land.runtime_crossings_synced {
            let sub = compiled_island_substrate(doc);
            sync_river_crossings_with_substrate(doc, sub, delta);
            delta.live_land.runtime_crossings_synced = true;
        }
        return;
    }
    let stage = compute_seasonal_water_stage(game_day);
    let mut cells = BTreeMap::new();
    for c in doc.cells.values() {
        let is_crossing = c.kind != "settlement"
            && c.kind != "ruin_dungeon"
            && (c.flux >= 21 || c.title.contains('橋') || c.title.contains('渡'));
        let init_bridge = if is_crossing {
            (88 + (u32::from(c.cell_id) % 13)) as u8
        } else {
            100
        };
        let init_forest = if is_cell_forest_habitat(c) {
            85
        } else if c.biome == 4 || c.biome == 5 || c.kind == "wilderness" {
            45
        } else {
            15
        };
        let init_miasma = if c.plague_sealed {
            if c.kind == "plague_ruin" { 85 } else { 55 }
        } else {
            0
        };
        let init_flood = if stage >= 84 && c.flux >= 26 && c.height <= 22 {
            2
        } else if stage >= 72 && c.flux >= 21 && c.height <= 23 {
            1
        } else {
            0
        };
        cells.insert(
            c.cell_id,
            CellLiveLand {
                flood_level: init_flood,
                has_river_crossing: is_crossing,
                bridge_integrity: init_bridge,
                bridge_down_days: 0,
                forest_density: init_forest,
                reclaimed_plots: 0,
                fallow_days: 0,
                plague_miasmas: init_miasma,
            },
        );
    }

    let mut house_integrity = BTreeMap::new();
    for hh in &doc.households {
        if hh.vacant_for_sale && !delta.sold_houses.contains_key(&hh.node_id) {
            house_integrity.insert(hh.node_id.clone(), 76);
        }
    }

    delta.live_land = LiveLandState {
        seeded: true,
        last_ecology_day: game_day,
        water_stage_pct: stage,
        cells,
        house_integrity,
        runtime_crossings_synced: true,
    };
    let sub = compiled_island_substrate(doc);
    sync_river_crossings_with_substrate(doc, sub, delta);
    delta.extras_dirty = true;
}

fn find_repair_timber_source(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    cid: u32,
) -> Option<u32> {
    let cur = doc.cells.get(&cid.to_string())?;
    let mut cands: Vec<u32> = Vec::with_capacity(cur.neighbors_f6.len() + 1);
    cands.push(cid);
    for &nb in &cur.neighbors_f6 {
        cands.push(nb);
    }
    for &c_id in &cands {
        if let Some(c) = doc.cells.get(&c_id.to_string())
            && c.resource_slug == "timber_log"
            && delta.cell_stocks.get(&c_id).copied().unwrap_or(0) >= 2
        {
            return Some(c_id);
        }
    }
    for &c_id in &cands {
        if delta.cell_stocks.get(&c_id).copied().unwrap_or(0) >= 2 {
            return Some(c_id);
        }
    }
    None
}

fn find_available_bridge_artisan(delta: &IslandWorldDelta) -> Option<String> {
    delta
        .actors
        .iter()
        .find(|(_, a)| {
            a.alive
                && a.controller_id.is_none()
                && a.active_skirmish.is_none()
                && (super::shop_leases::has_forestry_or_bridge_tool(a)
                    || super::shop_leases::has_mining_or_forging_tool(a))
        })
        .map(|(id, _)| id.clone())
}

/// 按日推進全島水位、洪水侵蝕橋樑、林木生長、休耕退化、空屋衰敗與真實付薪買木修橋。
pub fn tick_daily_live_land(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    ensure_live_land_seeded(doc, delta, clock.game_days);
    if clock.game_days <= delta.live_land.last_ecology_day {
        return;
    }

    let start_day = delta.live_land.last_ecology_day + 1;
    let end_day = clock.game_days.min(delta.live_land.last_ecology_day + 30);

    for d in start_day..=end_day {
        super::cell_resources::ensure_and_regen_cell_resources(doc, delta, d);
        let stage = compute_seasonal_water_stage(d);
        delta.live_land.water_stage_pct = stage;
        let doy = d.rem_euclid(364);
        let is_winter = doy >= 273;
        let forest_grow: u8 = if doy < 182 {
            2
        } else if doy < 273 {
            1
        } else {
            0
        };

        let cids: Vec<u32> = delta.live_land.cells.keys().copied().collect();
        let sub = compiled_island_substrate(doc);
        let mut repair_queue: Vec<u32> = Vec::new();
        let mut tile_prune_events: Vec<(u32, bool, bool)> = Vec::new();
        for cid in cids {
            let Some(c) = doc.cells.get(&cid.to_string()) else {
                continue;
            };
            let Some(cl) = delta.live_land.cells.get_mut(&cid) else {
                continue;
            };
            cl.flood_level = if stage >= 84 && c.flux >= 26 && c.height <= 22 {
                2
            } else if stage >= 72 && c.flux >= 21 && c.height <= 23 {
                1
            } else {
                0
            };
            if cl.has_river_crossing && cl.bridge_integrity > 0 {
                let wear = if cl.flood_level == 2 {
                    36u8
                } else if cl.flood_level == 1 {
                    10u8
                } else {
                    0u8
                };
                cl.bridge_integrity = cl.bridge_integrity.saturating_sub(wear);
            }
            if cl.has_river_crossing && cl.bridge_integrity == 0 {
                cl.bridge_down_days = cl.bridge_down_days.saturating_add(1);
                if cl.bridge_down_days >= 2 {
                    repair_queue.push(cid);
                }
            } else {
                cl.bridge_down_days = 0;
            }

            let cap = cell_forest_cap(c, cl.reclaimed_plots);
            let mut grew_forest = false;
            if cl.forest_density < cap && forest_grow > 0 {
                cl.forest_density = cl.forest_density.saturating_add(forest_grow).min(cap);
                grew_forest = true;
            }

            let mut decayed_farm = false;
            let is_tended_farm_village =
                !c.plague_sealed && c.kind == "rural_settlement" && cl.reclaimed_plots == 0;
            if (cl.reclaimed_plots > 0
                || matches!(c.resource_slug.as_str(), "wheat_grain" | "dairy_cheese"))
                && !is_tended_farm_village
            {
                cl.fallow_days = cl.fallow_days.saturating_add(1);
                if cl.fallow_days >= 18 {
                    if cl.reclaimed_plots > 0 {
                        cl.reclaimed_plots -= 1;
                        decayed_farm = true;
                    }
                    cl.fallow_days = 10;
                    cl.forest_density = cl.forest_density.saturating_add(4).min(85);
                }
            }

            if is_winter && d.rem_euclid(14) == 0 && cl.plague_miasmas > 0 {
                cl.plague_miasmas = cl.plague_miasmas.saturating_sub(1);
            }
            if grew_forest || decayed_farm {
                tile_prune_events.push((cid, grew_forest, decayed_farm));
            }
        }
        for (cid, restore_forest, decay_farm) in tile_prune_events {
            prune_tile_diffs_for_cell_day(sub, delta, cid, d, restore_forest, decay_farm);
        }

        // 未售出空屋自然失修：暴洪季節加速損耗，低於 30 則屋瓦塌落為斷牆
        let sold_keys: Vec<String> = delta
            .live_land
            .house_integrity
            .keys()
            .filter(|hnid| delta.sold_houses.contains_key(*hnid))
            .cloned()
            .collect();
        for sold_nid in sold_keys {
            delta.live_land.house_integrity.remove(&sold_nid);
        }
        if d.rem_euclid(3) == 0 {
            let decay = if stage >= 80 { 4u8 } else { 2u8 };
            for integrity in delta.live_land.house_integrity.values_mut() {
                *integrity = integrity.saturating_sub(decay);
            }
        }

        // 公庫確實有錢、周邊有木料且有在世匠師時，方依工資發包修通斷橋（絕無免錢憑空復原）
        for cid in repair_queue {
            if delta.municipal_treasury_copper < 8 {
                break;
            }
            let Some(timber_cid) = find_repair_timber_source(doc, delta, cid) else {
                continue;
            };
            let Some(artisan_id) = find_available_bridge_artisan(delta) else {
                break;
            };
            let cur_stk = delta.cell_stocks.get(&timber_cid).copied().unwrap_or(0);
            if cur_stk < 2 {
                continue;
            }
            let c_title = doc
                .cells
                .get(&cid.to_string())
                .map_or("河谷木橋", |c| c.title.as_str())
                .to_string();
            if delta.transfer_copper(
                d,
                4,
                "bridge_repair_wage",
                "municipal_treasury",
                &artisan_id,
                8,
                format!("發放{}木橋搶修工資", c_title),
            ) {
                delta.cell_stocks.insert(timber_cid, cur_stk - 2);
                if let Some(cl) = delta.live_land.cells.get_mut(&cid) {
                    cl.bridge_integrity = 100;
                    cl.bridge_down_days = 0;
                }
            }
        }
    }

    delta.live_land.last_ecology_day = end_day;
    delta.extras_dirty = true;
}

pub fn is_quarantine_restricted(doc: &WorldStaticDoc, delta: &IslandWorldDelta, cid: u32) -> bool {
    if delta.live_land.seeded {
        delta
            .live_land
            .cells
            .get(&cid)
            .is_some_and(|c| c.plague_miasmas >= 30)
    } else {
        doc.cells
            .get(&cid.to_string())
            .is_some_and(|c| c.plague_sealed)
    }
}

pub fn is_bridge_washed_out(delta: &IslandWorldDelta, cid: u32) -> bool {
    delta
        .live_land
        .cells
        .get(&cid)
        .is_some_and(|c| c.has_river_crossing && c.bridge_integrity == 0)
}

/// 判斷車馬重貨能否通過地塊：建有高架木橋者以橋體完好（`bridge_integrity > 0`）為準，無橋低地遇大水漫流（`flood_level >= 2`）亦阻隔重載。
pub fn is_river_crossing_impassable_for_wheels(delta: &IslandWorldDelta, cid: u32) -> bool {
    delta.live_land.cells.get(&cid).is_some_and(|cl| {
        if cl.has_river_crossing {
            cl.bridge_integrity == 0
        } else {
            cl.flood_level >= 2
        }
    })
}

pub fn cell_flood_level(delta: &IslandWorldDelta, cid: u32) -> u8 {
    delta.live_land.cells.get(&cid).map_or(0, |c| c.flood_level)
}

pub fn house_is_ruined(delta: &IslandWorldDelta, house_nid: &str) -> bool {
    !delta.sold_houses.contains_key(house_nid)
        && delta
            .live_land
            .house_integrity
            .get(house_nid)
            .is_some_and(|&v| v < 30)
}

/// 依地塊即時活態水文與林田橋屋實況，補寫當場體感描述。
pub fn live_land_scene_prose_suffix(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    node: &SpatialNodeDef,
) -> String {
    if node.dungeon {
        return String::new();
    }
    let mut notes: Vec<&str> = Vec::new();
    if let Some(cl) = delta.live_land.cells.get(&node.cell_id) {
        if cl.has_river_crossing && cl.bridge_integrity == 0 {
            notes.push("河谷木橋已被洪流沖斷，僅餘兩岸殘樁");
        } else if cl.flood_level >= 2 {
            notes.push("春水暴漲漫過低岸，河灘泥濘湍急");
        } else if cl.flood_level == 1 {
            notes.push("上游來水充沛，溪水漫濕路緣礫石");
        }
        if cl.reclaimed_plots > 0 {
            notes.push("路側新闢出數畦翻鬆黑麥田壟");
        }
        if cl.forest_density <= 25
            && doc
                .cells
                .get(&node.cell_id.to_string())
                .is_some_and(is_cell_forest_habitat)
        {
            notes.push("周遭林木已被砍伐一空，露出大片樹樁");
        }
    }
    if node.kind == "house_vacant" && house_is_ruined(delta, &node.id) {
        notes.push("屋頂青瓦塌落半邊，露出朽裂斷牆");
    }
    if notes.is_empty() {
        String::new()
    } else {
        format!(" {}。", notes.join("；"))
    }
}

fn find_nearby_bridge_repair_cell(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    node: &SpatialNodeDef,
) -> Option<u32> {
    let mut cands = vec![node.cell_id];
    for ex in &node.exits {
        if let Some(dst) = doc.nodes.get(&ex.to)
            && !cands.contains(&dst.cell_id)
        {
            cands.push(dst.cell_id);
        }
    }
    cands.into_iter().find(|cid| {
        delta
            .live_land
            .cells
            .get(cid)
            .is_some_and(|cl| cl.has_river_crossing && cl.bridge_integrity < 80)
    })
}

fn find_nearby_vacant_house_to_repair(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    node: &SpatialNodeDef,
) -> Option<String> {
    if node.kind == "house_vacant"
        && delta
            .live_land
            .house_integrity
            .get(&node.id)
            .is_some_and(|&v| v < 65)
        && !delta.sold_houses.contains_key(&node.id)
    {
        return Some(node.id.clone());
    }
    for ex in &node.exits {
        if let Some(dst) = doc.nodes.get(&ex.to)
            && dst.kind == "house_vacant"
            && !delta.sold_houses.contains_key(&dst.id)
            && delta
                .live_land
                .house_integrity
                .get(&dst.id)
                .is_some_and(|&v| v < 65)
        {
            return Some(dst.id.clone());
        }
    }
    None
}

/// 掛載現場活態土地實作物件（修橋、伐林、拓墾、修屋、煙燻除疫）於玩家與居民共用之場景插座。
pub fn append_live_land_scene_objects<F>(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &super::types::UnifiedMindBody,
    node: &SpatialNodeDef,
    objects: &mut Vec<Value>,
    mut push_audit: F,
) where
    F: FnMut(&str, &str, &str, &str, &str, &str),
{
    if node.dungeon {
        return;
    }
    let outdoor_work = !node.indoor
        && matches!(
            node.kind.as_str(),
            "wild" | "road" | "gate" | "village_gate" | "quarantine_post"
        );
    let cl_opt = delta.live_land.cells.get(&node.cell_id);

    if outdoor_work && let Some(br_cid) = find_nearby_bridge_repair_cell(doc, delta, node) {
        let param = br_cid.to_string();
        objects.push(json!({
            "object_id": "obj:river_bridge_ruin",
            "name": "受損溪河木橋",
            "cue": "橋面木樑受洪流沖刷損毀，急需備木搶修",
            "desc": "兩岸石墩旁的橋木歪斜斷裂，若備妥原木、漂流木或石料前來接榫加固，鎮庫會依工資撥付酬勞。",
            "direct_action": {
                "id": "repair_river_bridge",
                "param": param,
                "label": "架木搶修渡橋",
            },
            "options": [],
        }));
        push_audit(
            "repair_river_bridge",
            &br_cid.to_string(),
            "架木搶修渡橋",
            "備木方石料加固橋樑",
            "object",
            "obj:river_bridge_ruin",
        );
    }

    if outdoor_work && let Some(cl) = cl_opt {
        let mut land_opts: Vec<Value> = Vec::new();
        if cl.forest_density >= 20 {
            land_opts.push(json!({
                "id": "fell_timber_clear",
                "param": "",
                "label": "揮斧伐木開林",
                "sub": "取下灰松原木並林相透光",
            }));
            push_audit(
                "fell_timber_clear",
                "",
                "揮斧伐木開林",
                "採得原木入囊",
                "object",
                "obj:living_terrain",
            );
        }
        let not_plague = cl.plague_miasmas < 30;
        let height_ok = doc
            .cells
            .get(&node.cell_id.to_string())
            .is_some_and(|c| c.height <= 24);
        if not_plague && height_ok && cl.reclaimed_plots < 8 {
            land_opts.push(json!({
                "id": "reclaim_farmland",
                "param": "",
                "label": "翻土拓墾新壟",
                "sub": "清出平坦田地供長遠生計",
            }));
            push_audit(
                "reclaim_farmland",
                "",
                "翻土拓墾新壟",
                "闢出新畦耕地",
                "object",
                "obj:living_terrain",
            );
        }
        if !land_opts.is_empty() {
            objects.push(json!({
                "object_id": "obj:living_terrain",
                "name": "林緣坡地與荒原",
                "cue": "林木枝葉茂盛，坡腳土層鬆厚可伐木拓墾",
                "desc": "這片林野隨四季雨水消長，一斧一鋤留下的空地與田壟都會真實刻在土地上。",
                "direct_action": Value::Null,
                "options": land_opts,
            }));
        }
    }

    if let Some(hnid) = find_nearby_vacant_house_to_repair(doc, delta, node) {
        objects.push(json!({
            "object_id": "obj:house_repair_spot",
            "name": "失修破敗空屋",
            "cue": "屋簷破漏、牆瓦斑駁，需添木石修繕",
            "desc": "無人長住的老屋漸見損毀，添置原木或石料就能替它補齊屋脊與門牆。",
            "direct_action": {
                "id": "repair_vacant_house",
                "param": hnid,
                "label": "添料修繕屋瓦",
            },
            "options": [],
        }));
        push_audit(
            "repair_vacant_house",
            &hnid,
            "添料修繕屋瓦",
            "補強空屋屋頂壁面",
            "object",
            "obj:house_repair_spot",
        );
    }

    let has_wormwood = viewer
        .backpack
        .iter()
        .any(|i| i.slug == "quarantine_wormwood" && i.count > 0);
    if !node.indoor && has_wormwood && cl_opt.is_some_and(|cl| cl.plague_miasmas > 0) {
        objects.push(json!({
            "object_id": "obj:wormwood_brazier",
            "name": "路側檢疫燻爐",
            "cue": "殘留濁氣尚在，可燃燒苦艾淨化",
            "desc": "石砌低矮火盆專供行人與守衛焚燒南岸苦艾，逐日驅散地面疫氣。",
            "direct_action": {
                "id": "burn_wormwood_miasma",
                "param": "",
                "label": "燃艾驅散疫氣",
            },
            "options": [],
        }));
        push_audit(
            "burn_wormwood_miasma",
            "",
            "燃艾驅散疫氣",
            "焚化苦艾減弱濁氣",
            "object",
            "obj:wormwood_brazier",
        );
    }
}

/// 玩家與居民 100% 共用之活態土地動詞分派；非此模組動詞回傳 `None`。
pub fn try_apply_live_land_action(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    action: &str,
    param: &str,
) -> Option<Result<String, String>> {
    match action {
        "repair_river_bridge" => {
            ensure_live_land_seeded(doc, delta, clock.game_days);
            let target_cid = param
                .trim()
                .parse::<u32>()
                .ok()
                .or_else(|| find_nearby_bridge_repair_cell(doc, delta, node));
            let Some(cid) = target_cid else {
                return Some(Err("附近並無受損待修的溪河木橋。".to_string()));
            };
            let needs_repair = delta
                .live_land
                .cells
                .get(&cid)
                .is_some_and(|cl| cl.has_river_crossing && cl.bridge_integrity < 100);
            if !needs_repair {
                return Some(Err("這座木橋橋樑目前穩固完好，無需動工搶修。".to_string()));
            }
            let mat_taken = {
                let Some(a) = delta.actors.get_mut(aid) else {
                    return Some(Err("找不到旅人行蹤。".to_string()));
                };
                if a.fatigue >= 88 {
                    return Some(Err("氣力已近枯竭，先歇息再扛木修橋。".to_string()));
                }
                let took = take_clean_carry_items(&mut a.backpack, "timber_log", 1)
                    .or_else(|| take_clean_carry_items(&mut a.backpack, "driftwood", 1))
                    .or_else(|| take_clean_carry_items(&mut a.backpack, "quarry_stone", 1));
                let Some(item) = took else {
                    return Some(Err(
                        "行囊內尚無原木、漂流木或石料可充作橋樑架料。".to_string()
                    ));
                };
                a.fatigue = a.fatigue.saturating_add(8).min(95);
                a.mark_mutated();
                item.name
            };
            if let Some(cl) = delta.live_land.cells.get_mut(&cid) {
                cl.bridge_integrity = cl.bridge_integrity.saturating_add(60).min(100);
                if cl.bridge_integrity >= 80 {
                    cl.bridge_integrity = 100;
                    cl.bridge_down_days = 0;
                }
            }
            delta.extras_dirty = true;
            let got_wage = delta.municipal_treasury_copper >= 6
                && delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "bridge_repair_reward",
                    "municipal_treasury",
                    aid,
                    6,
                    format!("搶修第 {cid} 區木橋撥付工費"),
                );
            let msg = if got_wage {
                format!(
                    "你搬出【{mat_taken}】重新架牢斷橋木樑並打緊鐵楔，渡口恢復車馬通行，鎮庫依例撥付 6 枚銅幣工資。"
                )
            } else {
                format!(
                    "你搬出【{mat_taken}】架牢斷橋木樑使渡口恢復通行；鎮庫暫時吃緊未能結付工資。"
                )
            };
            if let Some(a) = delta.actors.get_mut(aid) {
                push_breath(a, msg.clone());
            }
            Some(Ok(msg))
        }
        "fell_timber_clear" => {
            ensure_live_land_seeded(doc, delta, clock.game_days);
            if node.indoor
                || node.dungeon
                || doc
                    .cells
                    .get(&node.cell_id.to_string())
                    .is_some_and(|c| c.kind == "settlement")
            {
                return Some(Err("城鎮石板街坊或屋簷坑穴內並無野林成木可伐。".to_string()));
            }
            if delta.actors.get(aid).is_some_and(|a| a.fatigue >= 85) {
                return Some(Err("雙臂酸沉乏力，先歇一口氣再揮斧伐木。".to_string()));
            }
            let sub = compiled_island_substrate(doc);
            super::map_walk::ensure_single_actor_grid_seeded(doc, sub, delta, aid);
            let Some((actor_gx, actor_gy)) = resolve_actor_standing_xy(delta, aid) else {
                return Some(Err("找不到旅人當下立足方位。".to_string()));
            };
            let carved =
                carve_felled_tree_tile_at_actor(sub, delta, actor_gx, actor_gy, clock.game_days);
            if carved == 0 {
                return Some(Err(
                    "腳下四周並無可揮斧伐倒的成材林木，請先走近林緣樹叢再動手。".to_string(),
                ));
            }
            let Some(a) = delta.actors.get_mut(aid) else {
                return Some(Err("找不到旅人行蹤。".to_string()));
            };
            a.fatigue = a.fatigue.saturating_add(8).min(95);
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: "timber_log".to_string(),
                    name: "灰松厚原木".to_string(),
                    count: 1,
                    weight_units: 4,
                    ..CarryItem::default()
                },
            );
            let msg =
                "你揮斧放倒一株壯實灰松並去枝截段，將 1 根【灰松厚原木】扛上行囊。".to_string();
            push_breath(a, msg.clone());
            a.mark_mutated();
            super::map_memory::record_local_witness_at_tile(
                doc,
                delta,
                aid,
                actor_gx as f32 + 0.5,
                actor_gy as f32 + 0.5,
                clock.game_days,
            );
            if let Some(cl) = delta.live_land.cells.get_mut(&node.cell_id) {
                cl.forest_density = cl.forest_density.saturating_sub((carved as u8) * 6);
            }
            if let Some(stk) = delta.cell_stocks.get_mut(&node.cell_id) {
                *stk = stk.saturating_sub(1);
            }
            delta.extras_dirty = true;
            Some(Ok(msg))
        }
        "reclaim_farmland" => {
            ensure_live_land_seeded(doc, delta, clock.game_days);
            if node.indoor
                || node.dungeon
                || doc
                    .cells
                    .get(&node.cell_id.to_string())
                    .is_some_and(|c| c.kind == "settlement")
            {
                return Some(Err("城鎮石板街區與屋簷內並無農坡空地可開墾。".to_string()));
            }
            let plague_m = delta
                .live_land
                .cells
                .get(&node.cell_id)
                .map_or(100, |cl| cl.plague_miasmas);
            if plague_m >= 30 {
                return Some(Err("南岸疫氣未消，土壤混濁不宜直接開墾播穀。".to_string()));
            }
            if doc
                .cells
                .get(&node.cell_id.to_string())
                .is_none_or(|c| c.height > 24)
            {
                return Some(Err("山勢陡峭多礫石，難以起出整片耕作田壟。".to_string()));
            }
            if delta.actors.get(aid).is_some_and(|a| a.fatigue >= 84) {
                return Some(Err("腰背痠麻氣喘，先歇息後再翻土開壟。".to_string()));
            }
            let sub = compiled_island_substrate(doc);
            super::map_walk::ensure_single_actor_grid_seeded(doc, sub, delta, aid);
            let Some((actor_gx, actor_gy)) = resolve_actor_standing_xy(delta, aid) else {
                return Some(Err("找不到旅人當下立足方位。".to_string()));
            };
            let reclaimed =
                reclaim_arable_tiles_at_actor(sub, delta, actor_gx, actor_gy, clock.game_days);
            if reclaimed == 0 {
                return Some(Err(
                    "腳下四周並無可翻土起壟的平曠草坡，請走近水畔空地再揮鋤。".to_string(),
                ));
            }
            let Some(a) = delta.actors.get_mut(aid) else {
                return Some(Err("找不到旅人行蹤。".to_string()));
            };
            a.fatigue = a.fatigue.saturating_add(10).min(95);
            let msg = "你揮鋤清出亂草碎石並翻鬆表土，地坡上實實在在多出一片新墾田壟。".to_string();
            push_breath(a, msg.clone());
            a.mark_mutated();
            super::map_memory::record_local_witness_at_tile(
                doc,
                delta,
                aid,
                actor_gx as f32 + 0.5,
                actor_gy as f32 + 0.5,
                clock.game_days,
            );
            if let Some(cl) = delta.live_land.cells.get_mut(&node.cell_id) {
                cl.reclaimed_plots = cl.reclaimed_plots.saturating_add(1).min(12);
                cl.fallow_days = 0;
                cl.forest_density = cl.forest_density.saturating_sub(4);
            }
            delta.extras_dirty = true;
            Some(Ok(msg))
        }
        "repair_vacant_house" => {
            ensure_live_land_seeded(doc, delta, clock.game_days);
            let target_nid = if !param.trim().is_empty() {
                param.trim().to_string()
            } else if let Some(hn) = find_nearby_vacant_house_to_repair(doc, delta, node) {
                hn
            } else {
                return Some(Err("眼前並無破損待修的空屋。".to_string()));
            };
            if !delta.live_land.house_integrity.contains_key(&target_nid) {
                return Some(Err(
                    "這棟屋舍尚未達到破損修繕程度或已有住戶承買。".to_string()
                ));
            }
            if let Some(sq) = super::live_wilds::has_squatter_at_node(delta, &target_nid) {
                return Some(Err(format!(
                    "這棟破屋裡正蹲踞著【{}】，得先持械驅逐清場才沒法被咬傷動工！",
                    sq.foe_name_zh
                )));
            }
            let mat = {
                let Some(a) = delta.actors.get_mut(aid) else {
                    return Some(Err("找不到旅人行蹤。".to_string()));
                };
                let took = take_clean_carry_items(&mut a.backpack, "timber_log", 1)
                    .or_else(|| take_clean_carry_items(&mut a.backpack, "quarry_stone", 1));
                let Some(item) = took else {
                    return Some(Err("修補老屋需自備 1 份原木或青灰方石料。".to_string()));
                };
                a.fatigue = a.fatigue.saturating_add(6).min(95);
                a.mark_mutated();
                item.name
            };
            delta.live_land.house_integrity.insert(target_nid, 95);
            delta.extras_dirty = true;
            let msg = format!("你搬進【{mat}】重新換柱補瓦，這棟空屋的屋脊壁面恢復齊整。");
            if let Some(a) = delta.actors.get_mut(aid) {
                push_breath(a, msg.clone());
            }
            Some(Ok(msg))
        }
        "burn_wormwood_miasma" => {
            ensure_live_land_seeded(doc, delta, clock.game_days);
            let cur_m = delta
                .live_land
                .cells
                .get(&node.cell_id)
                .map_or(0, |cl| cl.plague_miasmas);
            if cur_m == 0 {
                return Some(Err("此處空氣清朗，並無殘留疫氣需燻散。".to_string()));
            }
            let Some(a) = delta.actors.get_mut(aid) else {
                return Some(Err("找不到旅人行蹤。".to_string()));
            };
            if take_clean_carry_items(&mut a.backpack, "quarantine_wormwood", 1).is_none() {
                return Some(Err("隨身行囊內並無可焚燻的【南岸清冽苦艾束】。".to_string()));
            }
            let next_m = cur_m.saturating_sub(20);
            let msg = if next_m < 30 && cur_m >= 30 {
                "你點燃整束【南岸清冽苦艾束】，濃烈清香漫過地界，這處路段的疫氣已散去大半，不再受檢疫封鎖限制！".to_string()
            } else {
                "苦艾青煙隨海風騰起，地面滯留的腐濁疫氣淡薄了許多。".to_string()
            };
            push_breath(a, msg.clone());
            a.mark_mutated();
            if let Some(cl) = delta.live_land.cells.get_mut(&node.cell_id) {
                cl.plague_miasmas = next_m;
            }
            delta.extras_dirty = true;
            Some(Ok(msg))
        }
        _ => None,
    }
}
