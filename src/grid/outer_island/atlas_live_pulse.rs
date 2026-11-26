//! 全開實況大地圖之一～四刀動態世界狀態彙整（水文、供需鋪倉、荒野營寨異變、生死繼承）。

use serde_json::{Value, json};

use super::live_commodities::{
    APOTHECARY_HERB_SLUGS, SMITHY_FUEL_SLUGS, count_shop_stock_by_slugs,
    has_npc_shop_staff_assigned,
};
use super::live_land::{compute_seasonal_water_stage, house_is_ruined};
use super::live_lifecycle::current_solar_year;
use super::slow_think::mask_player_names_in_text;
use super::types::{IslandWorldDelta, SpatialNodeDef, WorldStaticDoc};

fn node_title(doc: &WorldStaticDoc, nid: &str) -> String {
    doc.nodes
        .get(nid)
        .map_or_else(|| nid.to_string(), |n| n.name.clone())
}

/// 建構 `/api/world_atlas/snapshot` 之 `autonomy_pulse.live_world`（第一～四刀即時實況，絕不外洩真人名）。
pub(super) fn build_live_world_pulse_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    game_days: i64,
) -> Value {
    let water_stage_pct = if delta.live_land.seeded {
        delta.live_land.water_stage_pct
    } else {
        compute_seasonal_water_stage(game_days)
    };

    let mut washed_out_bridges = Vec::new();
    let mut flooded_cells = Vec::new();
    let mut reclaimed_cells = Vec::new();
    let mut detoxified_ruins = Vec::new();

    for (&cid, cl) in &delta.live_land.cells {
        let cell_title = doc
            .cells
            .get(&cid.to_string())
            .map_or_else(|| format!("地塊 #{cid}"), |c| c.title.clone());
        if cl.has_river_crossing && cl.bridge_integrity == 0 {
            washed_out_bridges.push(json!({
                "cell_id": cid,
                "title": cell_title,
                "down_days": cl.bridge_down_days,
            }));
        }
        if cl.flood_level > 0 {
            flooded_cells.push(json!({
                "cell_id": cid,
                "title": cell_title,
                "flood_level": cl.flood_level,
            }));
        }
        if cl.reclaimed_plots > 0 {
            reclaimed_cells.push(json!({
                "cell_id": cid,
                "title": cell_title,
                "reclaimed_plots": cl.reclaimed_plots,
                "fallow_days": cl.fallow_days,
            }));
        }
        if delta.live_land.seeded
            && cl.plague_miasmas < 30
            && doc
                .cells
                .get(&cid.to_string())
                .is_some_and(|c| c.kind == "plague_ruin")
        {
            detoxified_ruins.push(json!({
                "cell_id": cid,
                "title": cell_title,
                "plague_miasmas": cl.plague_miasmas,
            }));
        }
    }

    let mut ruined_houses = Vec::new();
    for (hnid, &integrity) in &delta.live_land.house_integrity {
        if house_is_ruined(delta, hnid) {
            ruined_houses.push(json!({
                "node_id": hnid,
                "name": node_title(doc, hnid),
                "integrity": integrity,
            }));
        }
    }

    let mut cold_forges = Vec::new();
    let mut herb_out_apothecaries = Vec::new();
    for (nid, nd) in &doc.nodes {
        if nd.kind != "shop" {
            continue;
        }
        if has_npc_shop_staff_assigned(delta, nid, &["鍛鐵匠"]) {
            let fuel = count_shop_stock_by_slugs(delta, nid, &SMITHY_FUEL_SLUGS);
            if fuel == 0 {
                cold_forges.push(json!({
                    "node_id": nid,
                    "name": nd.name,
                    "cell_id": nd.cell_id,
                    "fuel_count": fuel,
                }));
            }
        }
        if has_npc_shop_staff_assigned(delta, nid, &["草藥師"]) {
            let herbs = count_shop_stock_by_slugs(delta, nid, &APOTHECARY_HERB_SLUGS);
            if herbs == 0 {
                herb_out_apothecaries.push(json!({
                    "node_id": nid,
                    "name": nd.name,
                    "cell_id": nd.cell_id,
                    "herb_count": herbs,
                }));
            }
        }
    }

    let bands: Vec<Value> = delta
        .live_wilds
        .bands
        .values()
        .map(|b| {
            let item_cnt: u32 = b.hoard_items.iter().map(|i| i.count).sum();
            json!({
                "cell_id": b.cell_id,
                "camp_node_id": b.camp_node_id,
                "band_name_zh": b.band_name_zh,
                "headcount": b.headcount,
                "food_stores": b.food_stores,
                "raid_pressure": b.raid_pressure,
                "hoard_copper": b.hoard_copper,
                "hoard_item_count": item_cnt,
                "active": game_days >= b.suppressed_until_day,
                "suppressed_until_day": b.suppressed_until_day,
            })
        })
        .collect();

    let squatters: Vec<Value> = delta
        .live_wilds
        .squatters_by_node
        .iter()
        .map(|(nid, sq)| {
            let cid = doc.nodes.get(nid).map_or(4616, |n| n.cell_id);
            json!({
                "node_id": nid,
                "node_name": node_title(doc, nid),
                "cell_id": cid,
                "foe_name_zh": sq.foe_name_zh,
                "occupied_since_day": sq.occupied_since_day,
            })
        })
        .collect();

    let mutated_bosses: Vec<Value> = delta
        .live_wilds
        .mutated_boss_by_node
        .iter()
        .map(|(nid, mb)| {
            let cid = doc.nodes.get(nid).map_or(4529, |n| n.cell_id);
            json!({
                "node_id": nid,
                "node_name": node_title(doc, nid),
                "cell_id": cid,
                "boss_name_zh": mb.boss_name_zh,
                "absorbed_runes": mb.absorbed_runes,
                "victim_label_zh": mask_player_names_in_text(delta, &mb.victim_label_zh),
                "mutated_day": mb.mutated_day,
            })
        })
        .collect();

    let obituaries: Vec<Value> = delta
        .live_lifecycle
        .obituaries
        .iter()
        .rev()
        .take(20)
        .map(|ob| {
            json!({
                "deceased_name": ob.deceased_name,
                "race": ob.race,
                "role": ob.role,
                "solar_year": ob.solar_year,
                "season_zh": ob.season_zh,
                "age_at_death": ob.age_at_death,
                "cause_zh": mask_player_names_in_text(delta, &ob.cause_zh),
                "heir_name": ob.heir_name,
                "heirloom_zh": ob.heirloom_zh.as_deref().map(|s| mask_player_names_in_text(delta, s)),
                "house_note_zh": ob.house_note_zh.as_deref().map(|s| mask_player_names_in_text(delta, s)),
            })
        })
        .collect();

    json!({
        "hydrology": {
            "water_stage_pct": water_stage_pct,
            "washed_out_bridges": washed_out_bridges,
            "flooded_cells": flooded_cells,
            "reclaimed_cells": reclaimed_cells,
            "ruined_houses": ruined_houses,
            "detoxified_ruins": detoxified_ruins,
        },
        "commodities": {
            "cold_forges": cold_forges,
            "herb_out_apothecaries": herb_out_apothecaries,
        },
        "wilds": {
            "bands": bands,
            "squatters": squatters,
            "mutated_bosses": mutated_bosses,
            "lair_rotation_seq": delta.live_wilds.lair_rotation_seq,
        },
        "lifecycle": {
            "solar_year": current_solar_year(game_days),
            "spawned_youth_seq": delta.live_lifecycle.spawned_youth_seq,
            "obituaries": obituaries,
        },
    })
}

/// 建構單一檢視節點之第一～四刀即時動態細況（供 `/api/world_atlas/node` 附載 `live_status`）。
pub(super) fn build_node_live_status_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    nd: &SpatialNodeDef,
    game_days: i64,
) -> Value {
    let house_integrity = delta.live_land.house_integrity.get(&nd.id).copied();
    let is_ruined_house = house_is_ruined(delta, &nd.id);
    let squatter = delta.live_wilds.squatters_by_node.get(&nd.id).map(|sq| {
        json!({
            "foe_name_zh": sq.foe_name_zh,
            "occupied_since_day": sq.occupied_since_day,
        })
    });
    let mutated_boss = delta.live_wilds.mutated_boss_by_node.get(&nd.id).map(|mb| {
        json!({
            "boss_name_zh": mb.boss_name_zh,
            "absorbed_runes": mb.absorbed_runes,
            "victim_label_zh": mask_player_names_in_text(delta, &mb.victim_label_zh),
            "mutated_day": mb.mutated_day,
        })
    });
    let goblin_band = delta
        .live_wilds
        .bands
        .values()
        .find(|b| b.camp_node_id == nd.id || b.cell_id == nd.cell_id)
        .map(|b| {
            json!({
                "band_name_zh": b.band_name_zh,
                "headcount": b.headcount,
                "food_stores": b.food_stores,
                "raid_pressure": b.raid_pressure,
                "hoard_copper": b.hoard_copper,
                "hoard_item_count": b.hoard_items.iter().map(|i| i.count).sum::<u32>(),
                "active": game_days >= b.suppressed_until_day,
            })
        });
    let cell_land = delta.live_land.cells.get(&nd.cell_id).map(|cl| {
        json!({
            "flood_level": cl.flood_level,
            "bridge_down": cl.has_river_crossing && cl.bridge_integrity == 0,
            "bridge_integrity": cl.bridge_integrity,
            "forest_density": cl.forest_density,
            "reclaimed_plots": cl.reclaimed_plots,
            "fallow_days": cl.fallow_days,
            "plague_miasmas": cl.plague_miasmas,
        })
    });
    let shop_supply = if nd.kind == "shop" {
        let mut out = serde_json::Map::new();
        if has_npc_shop_staff_assigned(delta, &nd.id, &["鍛鐵匠"]) {
            out.insert(
                "smithy_fuel".into(),
                json!(count_shop_stock_by_slugs(delta, &nd.id, &SMITHY_FUEL_SLUGS)),
            );
        }
        if has_npc_shop_staff_assigned(delta, &nd.id, &["草藥師"]) {
            out.insert(
                "apothecary_herbs".into(),
                json!(count_shop_stock_by_slugs(
                    delta,
                    &nd.id,
                    &APOTHECARY_HERB_SLUGS
                )),
            );
        }
        if out.is_empty() {
            Value::Null
        } else {
            Value::Object(out)
        }
    } else {
        Value::Null
    };

    let _ = doc;
    json!({
        "house_integrity": house_integrity,
        "is_ruined_house": is_ruined_house,
        "squatter": squatter,
        "mutated_boss": mutated_boss,
        "goblin_band": goblin_band,
        "cell_land": cell_land,
        "shop_supply": shop_supply,
    })
}
