//! 外海大島與全大陸通用封包級迷霧、$O(1)$ 在場視線裁剪與純世界內語氣 JSON 視圖組裝。

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::clock_and_lang::{SolarClockSnapshot, language_name_zh};
use super::engine::ordered_actors_at_node;
use super::persona::derive_innate_persona;
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

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
                format!("陌生的{}", other.role)
            };

            let innate = derive_innate_persona(other.soul_seed.max(1));
            let live_cue = super::living_presence::build_living_npc_cue(
                doc, delta, clock, viewer, other, &innate,
            );

            let peer_meta = build_peer_interaction_meta_json(delta, viewer, other);
            let deeds = super::scene_sockets::build_actor_deeds_for_viewer(
                doc, delta, viewer, other, node, clock,
            );

            json!({
                "id": other.public_actor_token(),
                "name": shown_name,
                "race": other.race,
                "role": other.role,
                "is_player": other.controller_id.is_some(),
                "known": known,
                "pinned": at_public_post || known || !deeds.is_empty(),
                "cue": live_cue,
                "deeds": deeds,
                "peer": peer_meta,
            })
        })
        .collect();

    let (scene_objects, body_cues, scene_actions) =
        super::scene_sockets::build_scene_presence_sockets(
            doc, delta, viewer, node, clock, &all_npcs,
        );

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
            let ambient_cue =
                super::mesh_expedition::exit_gate_ambient_cue(ex, clock).or_else(|| {
                    dst_opt.and_then(|dst| {
                        if dst.kind == "wild" {
                            Some("遠處傳來風吹林葉聲")
                        } else if dst.kind == "plaza" {
                            Some("傳來市集人聲")
                        } else if dst.kind == "campfire_nook" {
                            Some("石窩乾爽避風・宜歇腳回氣")
                        } else if dst.kind == "summit_outlook" {
                            Some("高處視野開闊・可眺望山海")
                        } else {
                            None
                        }
                    })
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
                "milestone_zh": ex.milestone_zh,
                "ambient_cue": ambient_cue,
                "to_pos": dst_opt.map(|d| d.pos),
                "to_cell_id": dst_opt.map(|d| d.cell_id),
                "to_indoor": dst_opt.is_some_and(|d| d.indoor),
                "to_kind": dst_opt.map(|d| d.kind.as_str()),
            })
        })
        .collect();

    if let Some(hid) = doc.hidden_entrances.get(&node.id)
        && viewer.unlocked_secrets.contains(&node.id)
    {
        let h_dst = doc.nodes.get(&hid.target);
        exits_out.push(json!({
            "dir": hid.dir,
            "to": hid.target,
            "to_name": hid.to_name,
            "kind": "secret",
            "ambient_cue": "石縫深處湧出微涼氣流",
            "to_pos": h_dst.map(|d| d.pos),
            "to_cell_id": h_dst.map(|d| d.cell_id),
            "to_indoor": h_dst.is_some_and(|d| d.indoor),
            "to_kind": h_dst.map(|d| d.kind.as_str()),
        }));
    }

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
            json!({
                "id": "skirmish_auto_clash",
                "param": "yield_then_strike",
                "label": "後發制人（下馬兌上馬）",
                "hint": "守勢卸銳→中盤拆招→破綻成相，自動連續演繹至換氣空檔"
            }),
            json!({
                "id": "skirmish_auto_clash",
                "param": "press_burst",
                "label": "搶勢壓制（一力降十會）",
                "hint": "起手搶攻→重勢壓架→連攜收勢，正面轟散對手架勢"
            }),
            json!({
                "id": "skirmish_auto_clash",
                "param": "seam_harvest",
                "label": "入微尋縫（庖丁解牛）",
                "hint": "游鬥誘隙→卸力貼近→切入甲縫，最宜對付重甲巨獸"
            }),
        ];
        if sk.foe_slug == "grey_ear_goblin" {
            moves.push(json!({
                "id": "skirmish_parley",
                "param": "",
                "label": "拋出乾糧喝止",
                "hint": if sk.can_speak_goblin { "以部族短詞換路免戰" } else { "尚未聽懂部族語" }
            }));
        }
        moves.push(json!({
            "id": "skirmish_retreat",
            "param": "",
            "label": "虛晃抽身退回",
            "hint": "保住隨身行囊脫離戰圈"
        }));
        json!({
            "foe_name": sk.foe_name,
            "guarding": sk.player_guard,
            "staggered": sk.foe_stagger > 0,
            "moves": moves,
        })
    });

    let transit_json = viewer.active_transit.as_ref().map(|tr| {
        let next_stop = super::transit_and_cargo::resolve_next_stop_name_zh(doc, delta, tr);
        json!({
            "line_name": tr.line_name,
            "next_stop_name": next_stop,
            "alight_next_stop": tr.alight_next_stop,
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

    let mut scene_desc = if !delta.sold_houses.contains_key(&node.id)
        && let Some(trace) = super::harbor_history::vacant_house_trace_zh(&node.id)
    {
        format!("{} {}", node.desc, trace)
    } else {
        node.desc.clone()
    };
    let homestead_labels: Vec<&str> = delta
        .built_houses
        .values()
        .filter(|bh| bh.anchor_node_id == node.id)
        .map(|bh| bh.label_zh.as_str())
        .take(4)
        .collect();
    if !homestead_labels.is_empty() {
        scene_desc.push_str(&format!(
            " 街巷旁新立石木宅院：【{}】。",
            homestead_labels.join("】、【")
        ));
    }
    scene_desc.push_str(&super::live_land::live_land_scene_prose_suffix(
        doc, delta, node,
    ));
    scene_desc.push_str(&super::live_wilds::live_wilds_scene_prose_suffix(
        doc, delta, node, clock,
    ));
    scene_desc
        .push_str(&super::live_commodities::live_commodities_scene_prose_suffix(doc, delta, node));

    let live_desc = super::live_tide::compose_live_scene_desc(doc, delta, node, clock, &scene_desc);
    let td = super::live_tide::derive_tide_clock_details(clock);

    let major_town_slug: Option<&str> = match node.cell_id {
        4616 => Some("white_gull_harbor"),
        4613 => Some("gull_tail_port"),
        4321 => Some("west_strait_ferry"),
        4525 => Some("greypine_post"),
        4073 => Some("north_cape_beacon"),
        _ => None,
    };
    let cell_opt = doc.cells.get(&node.cell_id.to_string());
    let cell_map_json = cell_illustrated_map_spec(node.cell_id).map(|sp| {
        json!({
            "url": sp.url,
            "bg_day": sp.bg_day,
            "bg_night": sp.bg_night,
            "glows": sp.glows,
        })
    });
    let dungeon_room_json = if node.dungeon {
        dungeon_room_tile_spec(&node.id).map(|sp| {
            json!({
                "room_url": sp.room_url,
                "tile_url": sp.tile_url,
                "anchor": sp.anchor,
                "label_pos": sp.label_pos,
            })
        })
    } else {
        None
    };

    let backpack_json: Vec<Value> = viewer
        .backpack
        .iter()
        .map(|it| {
            json!({
                "slug": it.slug,
                "name": it.name,
                "count": it.count,
                "fair_copper": super::phase_and_vessels::appraise_item_fair_copper(it),
                "is_voucher": it.bound_owner.is_some(),
                "is_unclaimed": it.stolen_from.is_some(),
                "is_vessel": it.vessel_meta.is_some(),
                "is_gear": it.gear_meta.is_some(),
                "is_manual": it.slug.starts_with("manual_"),
                "gear_slot": it.gear_meta.as_ref().map(|g| g.slot.as_str()),
            })
        })
        .collect();
    let locker_json: Vec<Value> = viewer
        .inn_lockers
        .get(&node.id)
        .unwrap_or(&viewer.inn_locker)
        .iter()
        .map(|it| {
            json!({
                "slug": it.slug,
                "name": it.name,
                "count": it.count,
            })
        })
        .collect();
    let player_json = json!({
        "id": public_id,
        "name": viewer.display_name,
        "race": viewer.race,
        "appearance": viewer.appearance,
        "copper": viewer.copper,
        "vibe": vibe_text,
        "body_cues": body_cues,
        "temper_cue": v_motive_cue,
        "self_bio": super::world_weave_bio::compose_player_biography_zh(doc, viewer, clock.game_days),
        "memorable_notes": memorable_notes,
        "wound": viewer.wound_part,
        "severe_warning": viewer.severe_wrist_warning,
        "inn_nights_left": viewer.inn_nights_left,
        "assigned_inn_room": viewer.assigned_inn_room,
        "ongoing_activity": viewer.ongoing_activity,
        "rescue_pact": viewer.rescue_pact_active,
        "owned_house_node": viewer.owned_house_node,
        "offline": super::offline_autonomy::build_offline_policy_view_json(viewer),
        "mastery": super::phase_and_vessels::format_mastery_status_zh(viewer),
        "weapon": {
            "name": viewer.bound_weapon.name,
            "provenance": viewer.bound_weapon.provenance,
            "wear": wear_text,
            "grip": grip_text,
            "rune": viewer.bound_weapon.rune_mark,
        },
        "body_gear": super::body_gear_craft::build_body_gear_view_json(viewer),
        "eight_phases": super::martial_art_and_manuals::build_eight_phases_view_json(viewer),
        "lifestyle": super::living_sim_and_farm::build_lifestyle_and_farm_view_json(delta, viewer, clock),
        "hunt_clue": super::wild_hunt_and_events::build_hunt_clue_view_json(doc, delta, viewer),
        "backpack": backpack_json,
        "inn_locker": locker_json,
        "languages": lang_summary,
    });

    json!({
        "ok": true,
        "setup_done": viewer.setup_done,
        "sync": {
            "boot_epoch": super::live_tide::boot_epoch_ms(),
            "persist_seq": delta.persist_seq,
        },
        "clock": {
            "calendar": clock.calendar_label,
            "bell": clock.bell_label,
            "bell_index": clock.bell_index,
            "is_night": clock.is_night,
            "hour": td.hour,
            "minute": td.minute,
            "sec_since_midnight": td.sec_since_midnight,
            "server_unix": clock.real_unix,
            "server_unix_ms": now_ms,
            "epoch_unix": td.epoch_unix,
            "scale": td.scale,
            "tide_slot": td.tide_slot,
            "tide_name": td.tide_name,
            "weather": td.weather_zh,
            "exposure": super::live_tide::node_exposure_kind(node),
        },
        "scene": {
            "node_id": node.id,
            "cell_id": node.cell_id,
            "name": node.name,
            "kind": node.kind,
            "indoor": node.indoor,
            "dungeon": node.dungeon,
            "pos": node.pos,
            "desc": live_desc,
            "happenings": super::living_presence::build_live_scene_happenings(doc, delta, viewer, node, clock),
            "npcs": visible_npcs,
            "crowd_overflow": crowd_overflow,
            "objects": scene_objects,
            "exits": exits_out,
            "actions": scene_actions,
            "transit_signboards": super::transit_and_cargo::build_node_transit_signboards(doc, delta, &node.id),
            "breaths": super::surface_cues::filter_clean_scene_breaths(&viewer.recent_breaths, &clock.bell_label),
            "static_map": {
                "cell_id": node.cell_id,
                "cell_title": cell_opt.map(|c| c.title.as_str()).unwrap_or(""),
                "cell_center": cell_opt.map(|c| c.center),
                "town_slug": major_town_slug,
                "cell_map": cell_map_json,
                "dungeon_room": dungeon_room_json,
                "island_master_url": "/maps/seam_lab/master_island_2k.webp?v=20261127r1",
            },
        },
        "player": player_json,
        "skirmish": skirmish_json,
        "transit": transit_json,
    })
}

pub fn build_peer_interaction_meta_json(
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    other: &UnifiedMindBody,
) -> Value {
    let other_tok = other.public_actor_token();
    let other_is_npc = other.controller_id.is_none() && !other.actor_id.starts_with("pl:");
    let tradable_goods: Vec<Value> = if other_is_npc {
        let inn = derive_innate_persona(other.soul_seed.max(1));
        let warmth = other
            .mind
            .ltm
            .social
            .get(&viewer.actor_id)
            .map_or(0, |s| i64::from(s.warmth));
        other
            .backpack
            .iter()
            .filter(|i| i.count > 0 && i.bound_owner.is_none() && i.stolen_from.is_none())
            .take(4)
            .map(|i| {
                let fair = super::phase_and_vessels::appraise_item_fair_copper(i);
                let ask = if warmth >= 35 {
                    (fair - 1).max(2)
                } else if inn.concession_slope < -10 {
                    fair + 2
                } else {
                    fair + 1
                };
                json!({
                    "slug": i.slug,
                    "name": i.name,
                    "ask_copper": ask,
                    "is_vessel": i.vessel_meta.is_some(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let open_pacts: Vec<Value> = delta
        .contracts
        .values()
        .filter(|c| {
            super::pacts_and_peers::is_escrow_pact_kind(&c.kind)
                && ((c.creditor_id == other.actor_id && c.status == "escrowed")
                    || (c.debtor_id == viewer.actor_id
                        && c.status == "in_progress"
                        && (c.creditor_id == other.actor_id
                            || super::pacts_and_peers::parse_pact_note(&c.note_zh).0
                                == other.actor_id)))
        })
        .take(3)
        .map(|c| {
            let (_, desc) = super::pacts_and_peers::parse_pact_note(&c.note_zh);
            json!({
                "contract_id": c.contract_id,
                "kind": c.kind,
                "status": c.status,
                "reward_copper": c.amount_copper,
                "desc_zh": desc,
                "can_accept": c.status == "escrowed",
                "can_fulfill": c.status == "in_progress" && c.debtor_id == viewer.actor_id,
            })
        })
        .collect();

    let incoming_offer = viewer
        .mind
        .stm
        .pending_trade_offer
        .as_ref()
        .filter(|o| o.seller_id == other.actor_id && o.node_id == viewer.node_id)
        .map(|o| {
            json!({
                "item_name": o.item_name,
                "price_copper": o.price_copper,
            })
        });
    let incoming_spar = viewer
        .mind
        .stm
        .pending_spar_from
        .as_ref()
        .is_some_and(|(from_id, nid, _)| from_id == &other.actor_id && nid == &viewer.node_id);

    json!({
        "target_token": other_tok,
        "in_skirmish": other.active_skirmish.as_ref().map(|s| s.foe_name.clone()),
        "phase_badge": other.mind.ltm.mastery.witnessed_martial_phase.clone()
            .or_else(|| other.mind.ltm.mastery.witnessed_craft_phase.clone()),
        "tradable_goods": tradable_goods,
        "open_pacts": open_pacts,
        "incoming_offer": incoming_offer,
        "incoming_spar": incoming_spar,
    })
}

pub fn build_filtered_map_json(
    doc: &WorldStaticDoc,
    delta: &super::types::IslandWorldDelta,
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
            let core_access = super::map_memory::actor_has_core_cell_access(doc, viewer, cid);
            expand_bounds(&cell.polygon);
            if !core_access {
                outline_ids.insert(cid);
            }
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
                "title": if core_access { cell.title.as_str() } else { "路廊外緣" },
                "state_known": core_access && viewer.known_state_cells.contains(&cid),
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
            && super::map_memory::actor_has_core_cell_access(doc, viewer, cid as u32)
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
            "stage_size": [900, 900],
            "bg_color": "#11100e",
            "rooms": rooms,
        }))
    } else {
        None
    };

    if !min_x.is_finite() {
        min_x = 1460.0;
        min_y = 690.0;
        max_x = 1485.0;
        max_y = 712.0;
    }

    let mut full_min_x = f64::INFINITY;
    let mut full_min_y = f64::INFINITY;
    let mut full_max_x = f64::NEG_INFINITY;
    let mut full_max_y = f64::NEG_INFINITY;
    for cell in doc.cells.values() {
        for &[x, y] in &cell.polygon {
            full_min_x = full_min_x.min(x);
            full_min_y = full_min_y.min(y);
            full_max_x = full_max_x.max(x);
            full_max_y = full_max_y.max(y);
        }
    }
    if !full_min_x.is_finite() {
        full_min_x = min_x;
        full_min_y = min_y;
        full_max_x = max_x;
        full_max_y = max_y;
    }

    let substrate = super::island_substrate::build_viewer_substrate_json(doc, delta, viewer, clock);

    let cur_cell_map_json = cell_illustrated_map_spec(cur_node.cell_id).map(|sp| {
        json!({
            "url": sp.url,
            "bg_day": sp.bg_day,
            "bg_night": sp.bg_night,
            "glows": sp.glows,
        })
    });

    json!({
        "ok": true,
        "world_name": doc.world_name,
        "state_name": if viewer.known_state_cells.is_empty() { Value::Null } else { json!(doc.state_name) },
        "is_night": clock.is_night,
        "current_cell_id": cur_node.cell_id,
        "current_node_id": cur_node.id,
        "current_node_name": cur_node.name,
        "in_dungeon": cur_node.dungeon,
        "camera_bounds": [full_min_x - 3.0, full_min_y - 3.0, full_max_x + 3.0, full_max_y + 3.0],
        "visited_cells": visited_cells_out,
        "outline_cells": outline_polygons,
        "highways": visible_highways,
        "sea_corridors": visible_sea,
        "town_micro_maps": visited_towns,
        "current_cell_map": cur_cell_map_json,
        "island_master_url": "/maps/seam_lab/master_island_2k.webp?v=20261127r1",
        "dungeon_local": dungeon_local,
        "substrate": substrate,
    })
}

pub(crate) use super::cell_maps::{cell_illustrated_map_spec, dungeon_room_tile_spec};
