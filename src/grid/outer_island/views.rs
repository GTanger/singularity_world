//! 外海大島與全大陸通用封包級迷霧、$O(1)$ 在場視線裁剪與純世界內語氣 JSON 視圖組裝。

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::clock_and_lang::{SolarClockSnapshot, language_name_zh};
use super::engine::ordered_actors_at_node;
use super::persona::derive_innate_persona;
use super::types::{IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};

const PRIMARY_SCENE_NPC_SLOTS: usize = 6;

pub fn build_view_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    clock: &SolarClockSnapshot,
) -> Value {
    let node = doc
        .nodes
        .get(&viewer.node_id)
        .or_else(|| doc.nodes.get(&doc.spawn_node))
        .expect("spawn node exists");

    let now_ms = u64::try_from(clock.real_unix.max(0)).unwrap_or(0) * 1000;
    let all_npcs: Vec<Value> = ordered_actors_at_node(delta, viewer, node, now_ms)
        .into_iter()
        .map(|other| {
            let known = viewer.is_acquainted_with(&other.actor_id)
                || other.is_acquainted_with(&viewer.actor_id);
            let at_public_post = other.controller_id.is_none()
                && (other.work_node == node.id
                    || (node.kind == "shop" && other.home_node == node.id))
                && matches!(node.kind.as_str(), "shop" | "dock" | "gate");
            let shown_name = if known || at_public_post {
                other.display_name.clone()
            } else {
                format!("陌生的{}{}", other.race, other.role)
            };

            let wealth_cue = if other.copper >= 150 {
                "錢袋鼓實"
            } else if other.copper >= 40 {
                "衣著整潔"
            } else {
                "手頭略緊"
            };
            let innate = derive_innate_persona(other.soul_seed.max(1));
            let moving_to = other
                .route_plan
                .front()
                .and_then(|nx| doc.nodes.get(nx))
                .map(|nd| nd.name.split('・').next_back().unwrap_or(nd.name.as_str()));
            let posture = innate.body_posture_cue(
                other.mind.stm.affect.hunger,
                other.mind.stm.affect.fatigue.max(other.fatigue),
                other.mind.stm.affect.anger,
                moving_to,
            );
            let live_cue = if moving_to.is_some() {
                posture
            } else {
                format!("{}・{}", posture, wealth_cue)
            };

            json!({
                "id": other.public_actor_token(),
                "name": shown_name,
                "race": other.race,
                "role": other.role,
                "known": known,
                "pinned": at_public_post || known,
                "cue": live_cue,
            })
        })
        .collect();

    let (visible_npcs, crowd_overflow): (Vec<Value>, Vec<Value>) =
        if all_npcs.len() > PRIMARY_SCENE_NPC_SLOTS {
            (
                all_npcs[..PRIMARY_SCENE_NPC_SLOTS].to_vec(),
                all_npcs[PRIMARY_SCENE_NPC_SLOTS..].to_vec(),
            )
        } else {
            (all_npcs, Vec::new())
        };

    let known_nodes = &viewer.mind.ltm.known_nodes;
    let mut exits_out: Vec<Value> = node
        .exits
        .iter()
        .map(|ex| {
            let dst_opt = doc.nodes.get(&ex.to);
            let ambient_cue = dst_opt.and_then(|dst| {
                if dst.kind == "wild" {
                    Some("遠處傳來風吹林葉聲")
                } else if dst.kind == "plaza" {
                    Some("傳來市集人聲")
                } else {
                    None
                }
            });
            let safe_to_name =
                if dst_opt.is_some_and(|dst| dst.dungeon) && !known_nodes.contains(&ex.to) {
                    "幽暗石道深處".to_string()
                } else {
                    ex.to_name.clone()
                };
            json!({
                "dir": ex.dir,
                "to": ex.to,
                "to_name": safe_to_name,
                "kind": ex.kind,
                "ambient_cue": ambient_cue,
            })
        })
        .collect();

    if let Some(hid) = doc.hidden_entrances.get(&node.id)
        && viewer.unlocked_secrets.contains(&node.id)
    {
        exits_out.push(json!({
            "dir": hid.dir,
            "to": hid.target,
            "to_name": hid.to_name,
            "kind": "secret",
            "ambient_cue": "石縫深處湧出微涼氣流",
        }));
    }

    let scene_actions = collect_scene_actions(doc, delta, viewer, node);

    let eff_fatigue = viewer.fatigue.max(viewer.mind.stm.affect.fatigue);
    let vibe_text = if eff_fatigue >= 80 {
        "氣喘如牛・步履沉重"
    } else if eff_fatigue >= 50 {
        "雙腿微痠・額角見汗"
    } else if eff_fatigue >= 25 {
        "呼吸微促・精神尚健"
    } else {
        "氣息勻暢・腳步輕快"
    };

    let wear_text = match viewer.bound_weapon.wear_stage {
        0 => "完好厚實",
        1 => "刃口微鈍",
        2 => "多處崩痕",
        _ => "急待修補",
    };
    let grip_text = if viewer.bound_weapon.grip_familiarity >= 22 {
        "揮灑自如"
    } else if viewer.bound_weapon.grip_familiarity >= 14 {
        "漸趨順手"
    } else {
        "初握尚生"
    };

    let lang_summary: Vec<Value> = viewer
        .languages
        .iter()
        .map(|(k, &v)| {
            let tier = if v >= 80 {
                "通曉流利"
            } else if v >= 25 {
                "略辨字詞"
            } else {
                "初聞音節"
            };
            json!({
                "code": k,
                "name": language_name_zh(k),
                "tier": tier,
            })
        })
        .collect();

    let skirmish_json = viewer.active_skirmish.as_ref().map(|sk| {
        let mut moves = vec![
            json!({ "id": "skirmish_wrist", "label": "專攻持刃手腕", "hint": "穩健削減對方架勢" }),
            json!({ "id": "skirmish_guard", "label": "沉肩守勢護腕", "hint": "格擋猛撲並穩住陣腳" }),
            json!({ "id": "skirmish_heavy", "label": "踏步重錘猛擊", "hint": "傾力破甲，帶傷力竭時慎用" }),
        ];
        if sk.foe_slug == "grey_ear_goblin" {
            moves.push(json!({
                "id": "skirmish_parley",
                "label": "拋出乾糧喝止",
                "hint": if sk.can_speak_goblin { "以部族短詞換路免戰" } else { "尚未聽懂部族語" }
            }));
        }
        moves.push(
            json!({ "id": "skirmish_retreat", "label": "虛晃抽身退回", "hint": "保住隨身行囊脫離戰圈" }),
        );
        json!({
            "foe_name": sk.foe_name,
            "round": sk.round_idx,
            "guarding": sk.player_guard,
            "staggered": sk.foe_stagger > 0,
            "moves": moves,
        })
    });

    let transit_json = viewer.active_transit.as_ref().map(|tr| {
        json!({
            "line_name": tr.line_name,
            "step_idx": tr.step_idx,
            "total_steps": tr.route_nodes.len().saturating_sub(1),
        })
    });

    let v_innate = derive_innate_persona(viewer.soul_seed.max(1));
    let v_motive_cue = format!(
        "心頭看重「{}」",
        v_innate.top_value_axis(&viewer.drift).in_world_motive_zh()
    );
    let memorable_notes: Vec<String> = viewer
        .mind
        .ltm
        .turning_points
        .iter()
        .map(|tp| tp.lesson_zh.clone())
        .chain(
            viewer
                .mind
                .ltm
                .episodic
                .iter()
                .rev()
                .take(6)
                .map(|e| e.subjective_zh.clone()),
        )
        .collect();

    let public_id = viewer.controller_id.as_deref().unwrap_or(&viewer.actor_id);

    json!({
        "ok": true,
        "setup_done": viewer.setup_done,
        "clock": {
            "calendar": clock.calendar_label,
            "bell": clock.bell_label,
            "is_night": clock.is_night,
            "hour": clock.hour,
        },
        "scene": {
            "node_id": node.id,
            "cell_id": node.cell_id,
            "name": node.name,
            "kind": node.kind,
            "indoor": node.indoor,
            "dungeon": node.dungeon,
            "desc": node.desc,
            "npcs": visible_npcs,
            "crowd_overflow": crowd_overflow,
            "exits": exits_out,
            "actions": scene_actions,
            "breaths": viewer.recent_breaths,
        },
        "player": {
            "id": public_id,
            "name": viewer.display_name,
            "race": viewer.race,
            "appearance": viewer.appearance,
            "copper": viewer.copper,
            "vibe": vibe_text,
            "temper_cue": v_motive_cue,
            "memorable_notes": memorable_notes,
            "wound": viewer.wound_part,
            "severe_warning": viewer.severe_wrist_warning,
            "inn_nights_left": viewer.inn_nights_left,
            "rescue_pact": viewer.rescue_pact_active,
            "owned_house_node": viewer.owned_house_node,
            "weapon": {
                "name": viewer.bound_weapon.name,
                "provenance": viewer.bound_weapon.provenance,
                "wear": wear_text,
                "grip": grip_text,
                "rune": viewer.bound_weapon.rune_mark,
            },
            "backpack": viewer.backpack.iter().map(|it| json!({
                "slug": it.slug,
                "name": it.name,
                "count": it.count,
                "is_voucher": it.bound_owner.is_some(),
                "is_unclaimed": it.stolen_from.is_some(),
            })).collect::<Vec<_>>(),
            "inn_locker": viewer.inn_lockers.get(&node.id).unwrap_or(&viewer.inn_locker).iter().map(|it| json!({
                "slug": it.slug,
                "name": it.name,
                "count": it.count,
            })).collect::<Vec<_>>(),
            "languages": lang_summary,
        },
        "skirmish": skirmish_json,
        "transit": transit_json,
    })
}

fn collect_scene_actions(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    node: &SpatialNodeDef,
) -> Vec<Value> {
    let mut acts = Vec::new();
    if viewer.active_skirmish.is_some() {
        return acts;
    }
    if viewer.active_transit.is_some() {
        acts.push(json!({ "id": "carriage_next", "label": "繼續乘車前行", "sub": "順大路推進" }));
        acts.push(json!({ "id": "carriage_alight", "label": "拍板中途下車", "sub": "改為步行" }));
        return acts;
    }

    if delta
        .dropped_loot_by_node
        .get(&node.id)
        .is_some_and(|v| !v.is_empty())
    {
        acts.push(
            json!({ "id": "pickup_dropped_bag", "label": "拾起遺落行囊", "sub": "查看散落物" }),
        );
    }
    if node.kind == "dock" {
        acts.push(json!({ "id": "work_dock", "label": "幫忙扛貨上棧橋", "sub": "領 4 枚銅幣" }));
    }
    if node.id.starts_with("oi:west_strait_ferry") {
        acts.push(
            json!({ "id": "inquire_ferry", "label": "查看跨海雙桅渡輪", "sub": "船票 25 銅幣" }),
        );
    }
    if node.name.contains("鍛鐵") || node.name.contains("鐵匠") {
        acts.push(json!({ "id": "forge_help", "label": "拉風箱協助打鐵", "sub": "領 5 枚銅幣" }));
        if viewer.bound_weapon.wear_stage > 0 {
            acts.push(
                json!({ "id": "repair_weapon", "label": "請師傅修補兵刃", "sub": "4 枚銅幣" }),
            );
        }
        if viewer
            .backpack
            .iter()
            .any(|i| i.slug == "rune_stone" && i.count > 0)
        {
            acts.push(
                json!({ "id": "engrave_rune", "label": "將原石鍛入兵刃", "sub": "6 枚銅幣" }),
            );
        }
    }
    if node.name.contains("酒館") {
        if viewer
            .backpack
            .iter()
            .any(|i| i.slug == "lodging_voucher" && i.count > 0)
        {
            acts.push(
                json!({ "id": "redeem_voucher", "label": "出示船東抵宿券", "sub": "換二樓住兩宿" }),
            );
        }
        acts.push(json!({ "id": "rent_inn", "label": "租下二樓閣樓一宿", "sub": "3 枚銅幣" }));
        acts.push(json!({ "id": "buy_ration", "label": "購買黑麥乾糧", "sub": "2 枚銅幣" }));
        if !viewer.rescue_pact_active {
            acts.push(
                json!({ "id": "buy_rescue_pact", "label": "登記逾時搜救契", "sub": "5 枚銅幣" }),
            );
        }
    }
    let in_owned = viewer.owned_house_node.as_deref() == Some(node.id.as_str());
    if node.kind == "inn_room" || in_owned {
        acts.push(json!({ "id": "rest_bed", "label": "上床寬衣安睡", "sub": "消除奔波疲乏" }));
    }
    if node.name.contains("草藥") && viewer.wound_part.is_some() {
        acts.push(json!({ "id": "bandage_wound", "label": "敷苦艾膏包紮傷處", "sub": "3 枚銅幣" }));
    }
    if (node.kind == "shop" || node.kind == "dock" || node.kind == "plaza")
        && viewer.backpack.iter().any(|i| {
            matches!(
                i.slug.as_str(),
                "rune_stone" | "spider_carapace" | "iron_ore" | "driftwood" | "herb_bundle"
            ) && i.count > 0
        })
    {
        acts.push(
            json!({ "id": "sell_specialty", "label": "出售採集山海貨品", "sub": "當面過秤點銅" }),
        );
    }
    if node.name.contains("驛馬") || node.name.contains("車馬") {
        if !node.id.starts_with("oi:west_strait_ferry") {
            acts.push(json!({ "id": "board_carriage_west", "label": "搭篷車往西峽渡口", "sub": "車資 3 銅幣" }));
        }
        if !node.id.starts_with("oi:white_gull_harbor") {
            acts.push(json!({ "id": "board_carriage_east", "label": "搭篷車回白鷗港", "sub": "車資 3 銅幣" }));
        }
    }
    if (node.kind == "gate" || node.kind == "plaza")
        && viewer.backpack.iter().any(|i| i.stolen_from.is_some())
    {
        acts.push(
            json!({ "id": "declare_found_bag", "label": "向衛兵申報拾遺物", "sub": "蓋章轉正當留用" }),
        );
    }
    if node.kind == "house_vacant"
        && viewer.owned_house_node.is_none()
        && !delta.sold_houses.contains_key(&node.id)
    {
        let price = doc
            .households
            .iter()
            .find(|h| h.node_id == node.id)
            .map_or(360, |h| h.price_copper);
        acts.push(json!({
            "id": "buy_vacant_house",
            "label": "購置此棟石木屋舍",
            "sub": format!("{price} 枚銅幣")
        }));
    }
    if viewer.fatigue >= 20
        && viewer
            .backpack
            .iter()
            .any(|i| i.slug == "rye_bread" && i.count > 0)
    {
        acts.push(json!({ "id": "eat_ration", "label": "嚼塊黑麥乾糧", "sub": "恢復腳力" }));
    }
    acts
}

pub fn build_filtered_map_json(
    doc: &WorldStaticDoc,
    viewer: &UnifiedMindBody,
    clock: &SolarClockSnapshot,
) -> Value {
    let cur_node = doc
        .nodes
        .get(&viewer.node_id)
        .or_else(|| doc.nodes.get(&doc.spawn_node))
        .expect("spawn node exists");

    let visited_cells = &viewer.mind.ltm.known_cells;
    let visited_nodes = &viewer.mind.ltm.known_nodes;

    let mut visited_cells_out = Vec::new();
    let mut outline_ids: BTreeSet<u32> = BTreeSet::new();
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    let mut expand_bounds = |poly: &[[f64; 2]]| {
        for &[x, y] in poly {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    };

    for &cid in visited_cells {
        if let Some(cell) = doc.cells.get(&cid.to_string()) {
            expand_bounds(&cell.polygon);
            for &nb in &cell.neighbors_f6 {
                if !visited_cells.contains(&nb) {
                    outline_ids.insert(nb);
                }
            }
            visited_cells_out.push(json!({
                "cell_id": cell.cell_id,
                "center": cell.center,
                "polygon": cell.polygon,
                "height": cell.height,
                "biome": cell.biome,
                "flux": cell.flux,
                "is_coast": cell.is_coast,
                "kind": cell.kind,
                "title": cell.title,
                "state_known": viewer.known_state_cells.contains(&cid),
            }));
        }
    }

    let mut outline_polygons = Vec::new();
    for oid in outline_ids {
        if let Some(c) = doc.cells.get(&oid.to_string()) {
            expand_bounds(&c.polygon);
            outline_polygons.push(json!({
                "cell_id": oid,
                "polygon": c.polygon,
            }));
        }
    }

    for rp in &doc.ring_polygons {
        let near_visited_coast = visited_cells.iter().any(|vcid| {
            doc.cells.get(&vcid.to_string()).is_some_and(|vc| {
                vc.is_coast
                    && rp.polygon.iter().any(|pt| {
                        let dx = pt[0] - vc.center[0];
                        let dy = pt[1] - vc.center[1];
                        (dx * dx + dy * dy) <= 196.0
                    })
            })
        });
        if near_visited_coast {
            expand_bounds(&rp.polygon);
            outline_polygons.push(json!({
                "cell_id": rp.cell_id,
                "polygon": rp.polygon,
            }));
        }
    }

    let near_any_visited = |pt: &[f64; 2], radius_sq: f64| -> bool {
        visited_cells.iter().any(|vcid| {
            doc.cells.get(&vcid.to_string()).is_some_and(|vc| {
                let dx = pt[0] - vc.center[0];
                let dy = pt[1] - vc.center[1];
                (dx * dx + dy * dy) <= radius_sq
            })
        })
    };

    let visible_highways: Vec<Value> = doc
        .highways
        .iter()
        .filter(|h| h.cells.iter().any(|c| visited_cells.contains(c)))
        .filter_map(|h| {
            let clipped_pts: Vec<[f64; 2]> = h
                .points
                .iter()
                .copied()
                .filter(|pt| near_any_visited(pt, 225.0))
                .collect();
            if clipped_pts.len() >= 2 {
                Some(json!({
                    "route_id": h.route_id,
                    "group": h.group,
                    "points": clipped_pts,
                }))
            } else {
                None
            }
        })
        .collect();

    let visible_sea: Vec<Value> = doc
        .sea_corridors
        .iter()
        .filter(|s| !s.crosses_strait || visited_cells.contains(&4321))
        .filter_map(|s| {
            let clipped: Vec<[f64; 2]> = s
                .points
                .iter()
                .copied()
                .filter(|pt| near_any_visited(pt, 324.0))
                .collect();
            if clipped.len() >= 2 {
                Some(json!({
                    "route_id": s.route_id,
                    "crosses_strait": s.crosses_strait,
                    "points": clipped,
                }))
            } else {
                None
            }
        })
        .collect();

    let mut visited_towns = serde_json::Map::new();
    for (slug, tval) in &doc.town_micro_maps {
        if let Some(cid) = tval.get("cell_id").and_then(|v| v.as_u64())
            && visited_cells.contains(&(cid as u32))
        {
            visited_towns.insert(slug.clone(), tval.clone());
        }
    }

    let dungeon_local = if cur_node.dungeon {
        let mut rooms = Vec::new();
        for (nid, nd) in &doc.nodes {
            if nd.dungeon && visited_nodes.contains(nid) {
                let safe_exits: Vec<Value> = nd
                    .exits
                    .iter()
                    .map(|ex| {
                        if visited_nodes.contains(&ex.to) {
                            json!({ "dir": ex.dir, "to": ex.to })
                        } else {
                            json!({ "dir": ex.dir, "to": Value::Null })
                        }
                    })
                    .collect();
                rooms.push(json!({
                    "id": nd.id,
                    "name": nd.name,
                    "pos": nd.pos,
                    "current": nd.id == cur_node.id,
                    "exits": safe_exits,
                }));
            }
        }
        Some(json!({
            "current_room": cur_node.id,
            "rooms": rooms,
        }))
    } else {
        None
    };

    if !min_x.is_finite() {
        min_x = 690.0;
        min_y = 510.0;
        max_x = 740.0;
        max_y = 560.0;
    }

    json!({
        "ok": true,
        "world_name": doc.world_name,
        "state_name": if viewer.known_state_cells.is_empty() { Value::Null } else { json!(doc.state_name) },
        "is_night": clock.is_night,
        "current_cell_id": cur_node.cell_id,
        "current_node_id": cur_node.id,
        "current_node_name": cur_node.name,
        "in_dungeon": cur_node.dungeon,
        "camera_bounds": [min_x - 6.0, min_y - 6.0, max_x + 6.0, max_y + 6.0],
        "visited_cells": visited_cells_out,
        "outline_cells": outline_polygons,
        "highways": visible_highways,
        "sea_corridors": visible_sea,
        "town_micro_maps": visited_towns,
        "dungeon_local": dungeon_local,
    })
}
