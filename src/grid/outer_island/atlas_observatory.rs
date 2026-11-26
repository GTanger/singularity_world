//! 設計者實況全開大地圖與誠實落實度審閱台（`/world_atlas`、`/api/world_atlas/*`）。
//!
//! 設計原則：
//! 1. 100% 真實反映後端 `WorldStaticDoc` 與執行期 `IslandWorldDelta` 現況：無迷霧攤開全島 93 地塊、466 空間節點、5 鎮街廓與地底試煉坑；
//! 2. 誠實對照正典：明確標示「已落實運作中之 5 鎮與地塊」vs「正典已定名但尚未落實為聚落之 35 處基層農莊／溪村／漁澳、海陸實體車船與對岸主陸」；
//! 3. 即時呈現全島常住居民（「活人」）當下位置、自主意圖、心緒掛念、隨身銅幣與器物、家戶／店鋪木箱實存物資與契約往來，並嚴格排除／遮罩真人玩家（`pl:`）個資。

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use super::clock_and_lang::current_solar_clock;
use super::harbor_lineage::WHITE_GULL_VACANT_HOUSES;
use super::household_economy::{count_household_larder_rations, effective_home_node};
use super::island_substrate::{compiled_island_substrate, world_to_grid_f32};
use super::live_lifecycle::actor_lived_years;
use super::live_shifts::compose_live_scene_desc;
use super::live_tide::{derive_tide_clock_details, node_exposure_kind};
use super::phase_and_vessels::count_world_active_runes;
use super::slow_think::mask_player_names_in_text;
use super::surface_cues::{build_chronicle_mind_json, render_concern_inner_thought_zh};
use super::types::{CarryItem, IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};
use super::world_weave_bio::{settlement_name_zh, settlement_slug_of};

/// 正典《全大陸中文地名政權物產戰史總冊 §2》銀鷗島 40 聚落現已全數落實（5 主鎮＋29 活態鄉鎮村莊哨點＋5 南岸封村遺址＋1 黑煤鎮塌陷坑舊址），本島未建聚落歸零。
pub const UNIMPLEMENTED_CANON_BURGS: &[(u32, &str, &str)] = &[];

fn lookup_unbuilt_burg(cell_id: u32) -> Option<(&'static str, &'static str)> {
    UNIMPLEMENTED_CANON_BURGS
        .iter()
        .find(|(cid, _, _)| *cid == cell_id)
        .map(|(_, name, kind)| (*name, *kind))
}

fn is_non_player_resident(a: &UnifiedMindBody) -> bool {
    a.alive && a.setup_done && a.controller_id.is_none() && !a.actor_id.starts_with("pl:")
}

/// 將個人主觀第二人稱呼吸／見聞（「你點出…」「…，你只勉強辨出…」）轉為設計者審閱台第三人稱敘述，並消除重複尾註。
pub(crate) fn observer_third_person_zh(actor_name: &str, raw: &str) -> String {
    let trimmed = raw.trim();
    let step1 = if let Some(rest) = trimmed.strip_prefix('你') {
        format!("{actor_name}{rest}")
    } else {
        trimmed.to_string()
    };
    let mut out = if let Some((lhs, rhs)) = step1.split_once("，你只") {
        format!("{lhs}，{actor_name}只{rhs}")
    } else {
        step1
    };
    while let Some(idx) = out.rfind('（') {
        let tail = &out[idx..];
        if tail.ends_with('）') && idx >= tail.len() && out[..idx].ends_with(tail) {
            out.truncate(idx);
        } else {
            break;
        }
    }
    out
}

fn format_items_brief(delta: &IslandWorldDelta, items: &[CarryItem]) -> Vec<Value> {
    items
        .iter()
        .filter(|i| i.count > 0)
        .map(|i| {
            json!({
                "slug": i.slug,
                "name": mask_player_names_in_text(delta, &i.name),
                "count": i.count,
                "has_vessel": i.vessel_meta.is_some(),
            })
        })
        .collect()
}

fn classify_resident_activity(doc: &WorldStaticDoc, a: &UnifiedMindBody) -> &'static str {
    let in_dng = doc.nodes.get(&a.node_id).is_some_and(|n| n.dungeon);
    if in_dng || a.active_skirmish.is_some() {
        "pit_or_combat"
    } else if !a.route_plan.is_empty() || !a.walk_waypoints.is_empty() || a.active_transit.is_some()
    {
        "moving"
    } else if doc
        .nodes
        .get(&a.node_id)
        .and_then(|n| doc.cells.get(&n.cell_id.to_string()))
        .is_some_and(|c| c.kind != "settlement")
    {
        "wilderness"
    } else if a.node_id == effective_home_node(a)
        || doc
            .nodes
            .get(&a.node_id)
            .is_some_and(|n| n.kind == "inn_room")
    {
        "home_rest"
    } else if a.node_id == a.work_node {
        "at_work"
    } else {
        "town_social"
    }
}

/// 建構 `/api/world_atlas/snapshot` 全島無迷霧實況總覽 JSON
pub fn build_atlas_snapshot_json(doc: &WorldStaticDoc, delta: &IslandWorldDelta) -> Value {
    let clock = current_solar_clock();
    let td = derive_tide_clock_details(&clock);
    let sub = compiled_island_substrate(doc);

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    let mut cell_nodes_map: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for (nid, nd) in &doc.nodes {
        if !nd.dungeon {
            cell_nodes_map
                .entry(nd.cell_id)
                .or_default()
                .push(nid.clone());
        }
    }
    for list in cell_nodes_map.values_mut() {
        list.sort();
    }

    // 統計每節點與每地塊之常住居民在場數及木箱物資數
    let mut node_occ_names: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut node_occ_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut cell_occ_counts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut node_box_counts: BTreeMap<String, u32> = BTreeMap::new();

    let mut npc_copper_sum = 0_i64;
    let mut player_copper_sum = 0_i64;
    let mut stat_at_work = 0_usize;
    let mut stat_moving = 0_usize;
    let mut stat_wild = 0_usize;
    let mut stat_pit = 0_usize;
    let mut stat_home = 0_usize;
    let mut stat_social = 0_usize;
    let mut stat_concerned = 0_usize;

    let mut residents_out = Vec::new();
    let mut notable_vessels = Vec::new();
    let mut feed_candidates: Vec<(i64, String, String)> = Vec::new();

    for a in delta.actors.values() {
        if !a.alive {
            continue;
        }
        if a.controller_id.is_some() || a.actor_id.starts_with("pl:") {
            player_copper_sum += a.copper;
            continue;
        }
        if !a.setup_done {
            continue;
        }

        npc_copper_sum += a.copper;
        for (bk, items) in &a.inn_lockers {
            let cnt: u32 = items.iter().map(|i| i.count).sum();
            if cnt > 0 {
                *node_box_counts.entry(bk.clone()).or_insert(0) += cnt;
            }
        }

        *node_occ_counts.entry(a.node_id.clone()).or_insert(0) += 1;
        let names_entry = node_occ_names.entry(a.node_id.clone()).or_default();
        if names_entry.len() < 4 {
            names_entry.push(a.display_name.clone());
        }
        if let Some(nd) = doc.nodes.get(&a.node_id) {
            *cell_occ_counts.entry(nd.cell_id).or_insert(0) += 1;
        }

        let act_bucket = classify_resident_activity(doc, a);
        match act_bucket {
            "at_work" => stat_at_work += 1,
            "moving" => stat_moving += 1,
            "wilderness" => stat_wild += 1,
            "pit_or_combat" => stat_pit += 1,
            "home_rest" => stat_home += 1,
            _ => stat_social += 1,
        }
        if a.mind.top_concern().is_some() {
            stat_concerned += 1;
        }

        if a.bound_weapon.rune_mark.is_some() || a.bound_weapon.grip_familiarity >= 18 {
            notable_vessels.push(json!({
                "owner_id": a.actor_id,
                "owner_name": a.display_name,
                "role": a.role,
                "weapon_name": mask_player_names_in_text(delta, &a.bound_weapon.name),
                "rune_mark": a.bound_weapon.rune_mark,
                "grip_familiarity": a.bound_weapon.grip_familiarity,
                "provenance": mask_player_names_in_text(delta, &a.bound_weapon.provenance),
            }));
        }

        if let Some(ep) = a.mind.stm.day_episodes.back() {
            let key = format!("ep:{}:{}", a.actor_id, ep.event_id);
            let place = doc
                .nodes
                .get(&ep.node_id)
                .map_or(ep.node_id.as_str(), |n| n.name.as_str());
            let txt = format!(
                "【{}・{}】{}",
                a.display_name,
                place,
                observer_third_person_zh(
                    &a.display_name,
                    &mask_player_names_in_text(delta, &ep.summary_zh)
                )
            );
            feed_candidates.push((ep.game_day * 100 + i64::from(ep.bell_idx), key, txt));
        }

        let sanitize_nid = |raw: &str| -> String {
            if let Some(n) = doc.nodes.get(raw) {
                n.name.clone()
            } else if let Some(bh) = delta.built_houses.get(raw) {
                mask_player_names_in_text(delta, &bh.label_zh)
            } else if raw.contains("pl:") {
                "旅人自建新宅".to_string()
            } else {
                mask_player_names_in_text(delta, raw)
            }
        };
        let cur_node_def = doc.nodes.get(&a.node_id);
        let cur_place_zh = sanitize_nid(&a.node_id);
        let cur_cell_id = cur_node_def.map_or(4616, |n| n.cell_id);
        let eff_home = effective_home_node(a);
        let home_name = sanitize_nid(eff_home);
        let work_name = sanitize_nid(&a.work_node);
        let intent_zh = a.mind.stm.intentions.back().map_or_else(
            || "操持日常起居".to_string(),
            |it| {
                let dst = sanitize_nid(&it.target_node);
                mask_player_names_in_text(delta, &format!("{} → {}", it.reason_zh, dst))
            },
        );
        let concern_zh = a
            .mind
            .top_concern()
            .map(|c| {
                format!(
                    "〔{}〕{}",
                    c.emotion.label_zh(),
                    render_concern_inner_thought_zh(delta, c, true)
                )
            })
            .unwrap_or_default();

        residents_out.push(json!({
            "actor_id": a.actor_id,
            "name": a.display_name,
            "race": a.race,
            "role": a.role,
            "age": actor_lived_years(a, clock.game_days),
            "bond": a.household_bond,
            "town_slug": settlement_slug_of(&a.household_id),
            "town_zh": settlement_name_zh(doc, a),
            "home_node": eff_home,
            "home_name": home_name,
            "work_node": a.work_node,
            "work_name": work_name,
            "current_node_id": a.node_id,
            "current_place_zh": cur_place_zh,
            "cell_id": cur_cell_id,
            "activity_bucket": act_bucket,
            "route_steps": a.route_plan.len(),
            "copper": a.copper,
            "fatigue": a.fatigue,
            "hunger": a.mind.stm.affect.hunger,
            "wound_part": a.wound_part,
            "larder_rations": count_household_larder_rations(delta, a),
            "tool_name": mask_player_names_in_text(delta, &a.bound_weapon.name),
            "intention_zh": intent_zh,
            "top_concern_zh": concern_zh,
        }));
    }

    residents_out.sort_by(|a, b| {
        a["town_slug"]
            .as_str()
            .cmp(&b["town_slug"].as_str())
            .then_with(|| a["actor_id"].as_str().cmp(&b["actor_id"].as_str()))
    });
    notable_vessels.truncate(24);

    // 近期真實帳本與生活動態流（去重並遮罩真人玩家）
    for le in delta.pending_ledger.iter().rev().take(24) {
        if le.debit_actor.starts_with("pl:") || le.credit_actor.starts_with("pl:") {
            continue;
        }
        let score = le.game_day * 100 + i64::from(le.bell_idx);
        feed_candidates.push((
            score,
            le.idempotency_key.clone(),
            format!(
                "【銅幣流轉・{}銅】{}",
                le.amount_copper,
                mask_player_names_in_text(delta, &le.summary_zh)
            ),
        ));
    }
    feed_candidates.sort_by(|a, b| b.0.cmp(&a.0));
    let mut seen_feed = BTreeSet::new();
    let mut recent_feed = Vec::new();
    for (_, key, txt) in feed_candidates {
        if seen_feed.insert(key) {
            recent_feed.push(txt);
            if recent_feed.len() >= 32 {
                break;
            }
        }
    }

    // 有效契約與賒帳紀錄
    let mut contracts_out = Vec::new();
    for c in delta.contracts.values() {
        if c.creditor_id.starts_with("pl:") || c.debtor_id.starts_with("pl:") {
            continue;
        }
        let cred_name = delta
            .actors
            .get(&c.creditor_id)
            .map_or(c.creditor_id.as_str(), |x| x.display_name.as_str());
        let debt_name = delta
            .actors
            .get(&c.debtor_id)
            .map_or(c.debtor_id.as_str(), |x| x.display_name.as_str());
        contracts_out.push(json!({
            "contract_id": c.contract_id,
            "kind": c.kind,
            "status": c.status,
            "creditor_name": cred_name,
            "debtor_name": debt_name,
            "amount_copper": c.amount_copper,
            "due_day": c.due_day,
            "note_zh": mask_player_names_in_text(delta, &c.note_zh),
        }));
    }
    contracts_out.truncate(40);

    // 無迷霧全島 93 地塊
    let mut cells_out = Vec::with_capacity(doc.cells.len());
    let mut sorted_cells: Vec<_> = doc.cells.values().collect();
    sorted_cells.sort_by_key(|c| c.cell_id);
    for c in sorted_cells {
        for &[x, y] in &c.polygon {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        let unbuilt = lookup_unbuilt_burg(c.cell_id).map(|(n, k)| {
            json!({
                "name_zh": n,
                "kind_zh": k,
            })
        });
        let c_nodes = cell_nodes_map.get(&c.cell_id).cloned().unwrap_or_default();
        let has_hidden = c_nodes
            .iter()
            .any(|nid| doc.hidden_entrances.contains_key(nid));
        let live_land_val = delta.live_land.cells.get(&c.cell_id).map(|cl| {
            json!({
                "flood_level": cl.flood_level,
                "bridge_down": cl.has_river_crossing && cl.bridge_integrity == 0,
                "forest_density": cl.forest_density,
                "reclaimed_plots": cl.reclaimed_plots,
                "plague_miasmas": cl.plague_miasmas,
            })
        });
        cells_out.push(json!({
            "cell_id": c.cell_id,
            "center": c.center,
            "polygon": c.polygon,
            "height": c.height,
            "biome": c.biome,
            "flux": c.flux,
            "is_coast": c.is_coast,
            "kind": c.kind,
            "title": c.title,
            "desc": c.desc,
            "resource_slug": c.resource_slug,
            "stock_now": delta.cell_stocks.get(&c.cell_id).copied().unwrap_or(c.stock_cap),
            "stock_cap": c.stock_cap,
            "plague_sealed": c.plague_sealed,
            "apex_beast": c.apex_beast.as_ref().map(|ab| json!({
                "name_zh": ab.name_zh,
                "slain": delta.slain_apex_beasts.contains_key(&c.cell_id),
            })),
            "resident_count": cell_occ_counts.get(&c.cell_id).copied().unwrap_or(0),
            "node_ids": c_nodes,
            "has_hidden_entrance": has_hidden,
            "canon_unbuilt_burg": unbuilt,
            "live_land": live_land_val,
            "cell_map": super::views::cell_illustrated_map_spec(c.cell_id).map(|sp| json!({
                "url": sp.url,
                "bg_day": sp.bg_day,
                "bg_night": sp.bg_night,
                "glows": sp.glows,
            })),
        }));
    }

    let ring_out: Vec<Value> = doc
        .ring_polygons
        .iter()
        .map(|rp| {
            for &[x, y] in &rp.polygon {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            json!({ "cell_id": rp.cell_id, "polygon": rp.polygon })
        })
        .collect();

    // 地底試煉坑 6 室（直接共用 views::dungeon_room_tile_spec 手繪石窟切片單一真相來源）
    let mut dungeon_rooms = Vec::new();
    for (nid, nd) in &doc.nodes {
        if nd.dungeon {
            let exits: Vec<Value> = nd
                .exits
                .iter()
                .map(|ex| json!({ "dir": ex.dir, "to": ex.to, "to_name": ex.to_name }))
                .collect();
            let tile_val = super::views::dungeon_room_tile_spec(nid).map(|sp| {
                json!({
                    "url": sp.tile_url,
                    "room_url": sp.room_url,
                    "box": sp.box_xywh,
                    "anchor": sp.anchor,
                    "label_pos": sp.label_pos,
                })
            });
            dungeon_rooms.push(json!({
                "id": nid,
                "name": nd.name,
                "pos": nd.pos,
                "desc": nd.desc,
                "occupant_count": node_occ_counts.get(nid).copied().unwrap_or(0),
                "exits": exits,
                "tile": tile_val,
            }));
        }
    }

    // 全部空間節點輕量摘要（供五鎮街廓魚骨圖即時顯示屋內人數與木箱存貨）
    let mut node_summary = serde_json::Map::new();
    let vacant_set: BTreeSet<&str> = doc
        .households
        .iter()
        .filter(|h| h.vacant_for_sale)
        .map(|h| h.node_id.as_str())
        .collect();
    for (nid, nd) in &doc.nodes {
        let occ_n = node_occ_counts.get(nid).copied().unwrap_or(0);
        let box_n = node_box_counts.get(nid).copied().unwrap_or(0);
        let loot_n = delta.dropped_loot_by_node.get(nid).map_or(0, Vec::len);
        let is_vacant = vacant_set.contains(nid.as_str()) && !delta.sold_houses.contains_key(nid);
        let is_sold = delta.sold_houses.contains_key(nid);
        node_summary.insert(
            nid.clone(),
            json!({
                "name": nd.name,
                "kind": nd.kind,
                "cell_id": nd.cell_id,
                "indoor": nd.indoor,
                "occupant_count": occ_n,
                "occupant_names": node_occ_names.get(nid).cloned().unwrap_or_default(),
                "box_item_count": box_n,
                "loot_bag_count": loot_n,
                "vacant_for_sale": is_vacant,
                "sold_house": is_sold,
                "ruined_house": super::live_land::house_is_ruined(delta, nid),
                "has_squatter": delta.live_wilds.squatters_by_node.contains_key(nid),
                "has_mutated_boss": delta.live_wilds.mutated_boss_by_node.contains_key(nid),
            }),
        );
    }
    for (nid, bh) in &delta.built_houses {
        let occ_n = node_occ_counts.get(nid).copied().unwrap_or(0);
        let bh_cell_id = doc
            .nodes
            .get(&bh.anchor_node_id)
            .map_or(4616, |n| n.cell_id);
        let pub_nid = if nid.contains("pl:") {
            format!("homestead:traveler:{}", bh.built_day)
        } else {
            nid.clone()
        };
        node_summary.insert(
            pub_nid,
            json!({
                "name": mask_player_names_in_text(delta, &bh.label_zh),
                "kind": "homestead",
                "cell_id": bh_cell_id,
                "indoor": true,
                "occupant_count": occ_n,
                "occupant_names": node_occ_names.get(nid).cloned().unwrap_or_default(),
                "box_item_count": node_box_counts.get(nid).copied().unwrap_or(0),
                "loot_bag_count": 0,
                "vacant_for_sale": false,
                "built_homestead": true,
                "ruined_house": super::live_land::house_is_ruined(delta, nid),
                "has_squatter": delta.live_wilds.squatters_by_node.contains_key(nid),
                "has_mutated_boss": false,
            }),
        );
    }

    let active_runes_held = count_world_active_runes(delta);
    let loot_copper_sum: i64 = delta
        .dropped_loot_by_node
        .values()
        .flatten()
        .map(|b| b.copper)
        .sum();
    let target_copper = if delta.genesis_total_copper > 0 {
        delta.genesis_total_copper
    } else {
        doc.ledger_genesis.total_conserved_copper
    };

    let spider_status = match &delta.first_spider_slain {
        Some((_, slayer_name, day)) => format!(
            "已於第 {} 遊戲日由【{}】完成全島唯一首殺（永不重刷，8 日後空巢易主岩蜥）",
            day,
            mask_player_names_in_text(delta, slayer_name)
        ),
        None => "元祖吞石巨蛛尚盤踞於【試煉坑・蛛網深窟】（尚未首殺）".to_string(),
    };

    json!({
        "ok": true,
        "persist_seq": delta.persist_seq,
        "clock": {
            "calendar": clock.calendar_label,
            "game_days": clock.game_days,
            "bell": clock.bell_label,
            "bell_index": clock.bell_index,
            "hour": td.hour,
            "minute": td.minute,
            "tide_slot": td.tide_slot,
            "tide_name": td.tide_name,
            "weather": td.weather_zh,
            "is_night": clock.is_night,
            "scale": td.scale,
        },
        "ledger": {
            "genesis_total_copper": target_copper,
            "resident_copper_sum": npc_copper_sum,
            "player_copper_sum": player_copper_sum,
            "municipal_treasury_copper": delta.municipal_treasury_copper,
            "silver_bank_reserve_copper": delta.silver_bank_reserve_copper,
            "contract_escrow_copper": delta.contract_escrow_copper,
            "mainland_clearing_copper": delta.mainland_clearing_copper,
            "dropped_loot_copper": loot_copper_sum,
            "goblin_hoard_copper": delta.goblin_hoard_copper,
            "conservation_diff": delta.conservation_diff(doc),
        },
        "audit_honesty": {
            "implemented_summary": [
                format!("銀鷗島 93 地塊 100% 無縫水彩古卷活輿圖（860×930 連續底盤）與 40 聚落舊址全面上線：5 座核心城鎮＋29 座鄉野聚落＋5 處南岸封村遺址＋黑煤鎮塌陷坑舊址＋53 處山海資源走廊，共 {} 個空間節點", doc.nodes.len() + delta.built_houses.len()),
                format!("讓世界自己運轉第一～三刀全面運作：四季水文漲退／沖橋攔車／公庫購木修橋／墾田退化與解封疫區、鋪倉煤鐵草藥口糧真存量／冷爐停診與動態供需報價，以及居民自主斷料自採／救災與轉行（共 {} 名具名居民）", residents_out.len()),
                format!("第四刀荒野風浪與生死更替已上線：5 座內陸灰耳部族營寨劫車屯贓（goblin_hoard {} 銅）、傾頹空屋流寇佔據、試煉坑異變頭目、太陽曆歲數老化與往生遺產／鋪位扶正／新丁成年（試煉坑：{}｜符文池餘 {} 枚・場上持有 {} 枚）", delta.goblin_hoard_copper, spider_status, delta.chaos_rune_pool, active_runes_held),
                format!("海陸 4 條實體逐拍車船班線共 {} 具載具即時運行並雙向運載貨箱；碼頭記籌手實物扛箱入鋪、酒館客房、百工精鍛全面運作", delta.transit_vehicles.len()),
            ],
            "unimplemented_gaps": [
                {
                    "title": "海峽對岸【蒼淵主陸・羅泉港等 100 國】尚未製作",
                    "detail": "銀鷗島本島 93 格、40 處正典聚落舊址與環島車船已全數落實；西海峽對岸蒼淵主陸目前僅保留靜態世界觀名冊與 mainland_clearing_copper 清算科目，尚未開放跨海登陸。",
                },
            ],
        },
        "autonomy_pulse": {
            "counts": {
                "total_residents": residents_out.len(),
                "at_work": stat_at_work,
                "moving": stat_moving,
                "wilderness": stat_wild,
                "pit_or_combat": stat_pit,
                "home_rest": stat_home,
                "town_social": stat_social,
                "with_concern": stat_concerned,
                "active_contracts": contracts_out.len(),
            },
            "contracts": contracts_out,
            "notable_vessels": notable_vessels,
            "recent_feed": recent_feed,
            "live_world": super::atlas_live_pulse::build_live_world_pulse_json(doc, delta, clock.game_days),
        },
        "map": {
            "world_name": doc.world_name,
            "state_name": doc.state_name,
            "bounds": [min_x - 8.0, min_y - 8.0, max_x + 8.0, max_y + 8.0],
            "camera_bounds": [1360.0, 628.0, 1506.0, 792.0],
            "substrate": super::island_substrate_pack::build_full_revealed_substrate_json(doc, delta, &clock),
            "cells": cells_out,
            "ring_polygons": ring_out,
            "highways": doc.highways,
            "sea_corridors": doc.sea_corridors,
            "transit_vehicles": delta.transit_vehicles.iter().map(|v| {
                let cur_nid = v.route_nodes.get(v.route_idx).map_or("", String::as_str);
                let nd_opt = doc.nodes.get(cur_nid);
                let cid = nd_opt.map_or(4616, |n| n.cell_id);
                let pos = doc.cells.get(&cid.to_string()).map_or([1420.0, 700.0], |c| c.center);
                let [gx, gy] = sub
                    .node_anchors
                    .get(cur_nid)
                    .copied()
                    .unwrap_or_else(|| world_to_grid_f32(pos[0], pos[1]));
                json!({
                    "vehicle_id": v.vehicle_id,
                    "name_zh": v.name_zh,
                    "mode": v.mode,
                    "node_id": cur_nid,
                    "node_name": nd_opt.map_or("", |n| n.name.as_str()),
                    "cell_id": cid,
                    "pos": pos,
                    "gx": gx,
                    "gy": gy,
                    "passenger_count": v.passengers.len(),
                    "cargo_crates": v.cargo_hold.len(),
                })
            }).collect::<Vec<_>>(),
            "town_micro_maps": doc.town_micro_maps,
            "dungeon_rooms": dungeon_rooms,
        },
        "node_summary": node_summary,
        "residents": residents_out,
    })
}

/// 建構單一場所節點或地塊（`node?id=oi:...` 或 `node?id=cell:4616`）之完整後台實況透視 JSON
pub fn build_atlas_node_detail_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    raw_query: &str,
) -> Option<Value> {
    let clock = current_solar_clock();
    let trimmed = raw_query.trim();

    let resolved_nid = if let Some(cid_str) = trimmed.strip_prefix("cell:") {
        let cid: u32 = cid_str.trim().parse().ok()?;
        doc.nodes
            .values()
            .find(|n| n.cell_id == cid && n.kind == "plaza")
            .or_else(|| doc.nodes.values().find(|n| n.cell_id == cid && !n.dungeon))
            .map(|n| n.id.clone())?
    } else if let Some(slug) = trimmed.strip_prefix("town:") {
        let plaza = format!("oi:{slug}:plaza");
        let yard = format!("oi:{slug}:yard");
        if doc.nodes.contains_key(&plaza) {
            plaza
        } else if doc.nodes.contains_key(&yard) {
            yard
        } else {
            let cid = doc
                .town_micro_maps
                .get(slug)
                .and_then(|v| v.get("cell_id"))
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)?;
            doc.nodes
                .values()
                .find(|n| n.cell_id == cid && !n.dungeon)
                .map(|n| n.id.clone())?
        }
    } else if let Some(day_str) = trimmed.strip_prefix("homestead:traveler:") {
        let day: i64 = day_str.trim().parse().ok()?;
        delta
            .built_houses
            .values()
            .find(|bh| bh.house_id.contains("pl:") && bh.built_day == day)
            .map(|bh| bh.house_id.clone())?
    } else {
        trimmed.to_string()
    };

    // 支援查詢動態自立起屋 `built_houses`
    let synthetic_homestead: Option<SpatialNodeDef> =
        delta.built_houses.get(&resolved_nid).map(|bh| {
            let bh_cell_id = doc
                .nodes
                .get(&bh.anchor_node_id)
                .map_or(4616, |n| n.cell_id);
            let owner_display = delta
                .actors
                .get(&bh.owner_id)
                .filter(|a| is_non_player_resident(a))
                .map_or_else(|| "某外來屋主".to_string(), |a| a.display_name.clone());
            SpatialNodeDef {
                id: bh.house_id.clone(),
                cell_id: bh_cell_id,
                name: mask_player_names_in_text(delta, &bh.label_zh),
                kind: "homestead".to_string(),
                desc: format!(
                    "於第 {} 遊戲日自立立柱起屋落成之新宅（屋主：{}）。",
                    bh.built_day, owner_display
                ),
                pos: [0.0, 0.0],
                indoor: true,
                dungeon: false,
                map_xy: None,
                exits: Vec::new(),
            }
        });

    let nd: &SpatialNodeDef = doc
        .nodes
        .get(&resolved_nid)
        .or(synthetic_homestead.as_ref())?;

    let cell_opt = doc.cells.get(&nd.cell_id.to_string());
    let live_desc = compose_live_scene_desc(doc, delta, nd, &clock, &nd.desc);
    let unbuilt =
        lookup_unbuilt_burg(nd.cell_id).map(|(n, k)| json!({ "name_zh": n, "kind_zh": k }));

    let vacant_info = WHITE_GULL_VACANT_HOUSES
        .iter()
        .find(|v| v.house_node_id == nd.id)
        .map(|v| {
            let sold_to = delta
                .sold_houses
                .get(&nd.id)
                .and_then(|oid| delta.actors.get(oid))
                .filter(|a| is_non_player_resident(a))
                .map(|a| a.display_name.clone());
            json!({
                "house_label": v.house_label,
                "former_head_zh": v.former_head_zh,
                "former_race": v.former_race,
                "former_trade": v.former_trade,
                "solar_year": v.solar_year,
                "room_trace_zh": v.room_trace_zh,
                "sold_to_resident": sold_to,
            })
        });

    let hh_info = doc.households.iter().find(|h| h.node_id == nd.id).map(|h| {
        let members: Vec<String> = delta
            .actors
            .values()
            .filter(|a| is_non_player_resident(a) && effective_home_node(a) == nd.id)
            .map(|a| format!("{}（{}）", a.display_name, a.role))
            .collect();
        json!({
            "household_id": h.id,
            "label": h.label,
            "vacant_for_sale": h.vacant_for_sale && !delta.sold_houses.contains_key(&nd.id),
            "price_copper": h.price_copper,
            "members": members,
        })
    });

    // 此場所內所有常住居民存放之真實木箱／鋪倉物資
    let mut stored_boxes = Vec::new();
    for a in delta.actors.values() {
        if !is_non_player_resident(a) {
            continue;
        }
        if let Some(items) = a.inn_lockers.get(&nd.id) {
            let clean = format_items_brief(delta, items);
            if !clean.is_empty() {
                stored_boxes.push(json!({
                    "owner_id": a.actor_id,
                    "owner_name": a.display_name,
                    "owner_role": a.role,
                    "items": clean,
                }));
            }
        }
    }

    let dropped_loot: Vec<Value> = delta
        .dropped_loot_by_node
        .get(&nd.id)
        .map(|bags| {
            bags.iter()
                .map(|b| {
                    json!({
                        "copper": b.copper,
                        "items": format_items_brief(delta, &b.items),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // 此刻實際站在該節點內之全體常住居民完整細節
    let mut occupants = Vec::new();
    if let Some(occ_set) = delta.node_occupants.get(&nd.id) {
        for aid in occ_set {
            let Some(a) = delta.actors.get(aid) else {
                continue;
            };
            if !is_non_player_resident(a) {
                continue;
            }
            let mind_json = build_chronicle_mind_json(doc, delta, a, clock.game_days);
            let eff_home = effective_home_node(a);
            let home_larder = a
                .inn_lockers
                .get(eff_home)
                .map(|v| format_items_brief(delta, v))
                .unwrap_or_default();
            let recent_breaths: Vec<String> = a
                .recent_breaths
                .iter()
                .rev()
                .take(6)
                .map(|b| {
                    observer_third_person_zh(&a.display_name, &mask_player_names_in_text(delta, b))
                })
                .collect();

            occupants.push(json!({
                "actor_id": a.actor_id,
                "name": a.display_name,
                "race": a.race,
                "role": a.role,
                "age": actor_lived_years(a, clock.game_days),
                "bond": a.household_bond,
                "appearance": a.appearance,
                "copper": a.copper,
                "fatigue": a.fatigue,
                "hunger": a.mind.stm.affect.hunger,
                "wound_part": a.wound_part,
                "route_steps": a.route_plan.len(),
                "home_node": if eff_home.contains("pl:") { "oi:white_gull_harbor:plaza" } else { eff_home },
                "home_name": doc.nodes.get(eff_home).map_or_else(|| if eff_home.contains("pl:") { "旅人自建新宅".to_string() } else { mask_player_names_in_text(delta, eff_home) }, |n| n.name.clone()),
                "work_node": if a.work_node.contains("pl:") { "oi:white_gull_harbor:plaza" } else { a.work_node.as_str() },
                "work_name": doc.nodes.get(&a.work_node).map_or_else(|| if a.work_node.contains("pl:") { "旅人自建新宅".to_string() } else { mask_player_names_in_text(delta, &a.work_node) }, |n| n.name.clone()),
                "tool_name": mask_player_names_in_text(delta, &a.bound_weapon.name),
                "tool_provenance": mask_player_names_in_text(delta, &a.bound_weapon.provenance),
                "backpack": format_items_brief(delta, &a.backpack),
                "home_larder": home_larder,
                "mind": mind_json,
                "recent_breaths": recent_breaths,
            }));
        }
    }

    let sibling_nodes: Vec<Value> = doc
        .nodes
        .values()
        .filter(|n| n.cell_id == nd.cell_id && !n.dungeon)
        .take(30)
        .map(|n| {
            let occ_c = delta.node_occupants.get(&n.id).map_or(0, |s| {
                s.iter()
                    .filter(|id| delta.actors.get(*id).is_some_and(is_non_player_resident))
                    .count()
            });
            json!({
                "id": n.id,
                "name": n.name,
                "kind": n.kind,
                "occupant_count": occ_c,
            })
        })
        .collect();

    Some(json!({
        "ok": true,
        "node_id": nd.id,
        "name": nd.name,
        "kind": nd.kind,
        "cell_id": nd.cell_id,
        "cell_title": cell_opt.map_or("", |c| c.title.as_str()),
        "cell_kind": cell_opt.map_or("wilderness", |c| c.kind.as_str()),
        "indoor": nd.indoor,
        "dungeon": nd.dungeon,
        "exposure": node_exposure_kind(nd),
        "live_desc": live_desc,
        "static_desc": nd.desc,
        "canon_unbuilt_burg": unbuilt,
        "exits": nd.exits,
        "vacant_backstory": vacant_info,
        "household": hh_info,
        "stored_boxes": stored_boxes,
        "dropped_loot": dropped_loot,
        "occupants": occupants,
        "cell_sibling_nodes": sibling_nodes,
        "live_status": super::atlas_live_pulse::build_node_live_status_json(doc, delta, nd, clock.game_days),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::outer_island::engine::load_static_world_doc;
    use crate::grid::outer_island::movement::apply_setup_origin;
    use crate::grid::outer_island::sim_tick::seed_genesis_actors_once;

    #[test]
    fn atlas_snapshot_and_node_detail_expose_full_map_honestly_without_player_leak() {
        let doc = load_static_world_doc();
        let mut delta = IslandWorldDelta {
            genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
            municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
            silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
            ..IslandWorldDelta::default()
        };
        seed_genesis_actors_once(&doc, &mut delta, false);
        let clock = current_solar_clock();
        apply_setup_origin(
            &doc,
            &mut delta,
            &clock,
            "secret_player_acc",
            "測試旅人甲",
            "人類",
            "smith_apprentice",
            "背著舊布囊",
        )
        .expect("player setup ok");

        let snap = build_atlas_snapshot_json(&doc, &delta);
        assert_eq!(snap["ok"], true);
        assert_eq!(snap["ledger"]["conservation_diff"], 0);
        assert_eq!(snap["ledger"]["goblin_hoard_copper"], 0);
        assert_eq!(snap["map"]["substrate"]["grid_w"], 860);
        assert_eq!(snap["map"]["substrate"]["grid_h"], 930);
        assert!(snap["autonomy_pulse"]["live_world"].is_object());
        let cells = snap["map"]["cells"].as_array().expect("cells array");
        assert_eq!(cells.len(), 93);
        let rural_cnt = cells
            .iter()
            .filter(|c| c["kind"] == "rural_settlement")
            .count();
        assert_eq!(rural_cnt, 29);
        assert_eq!(
            snap["map"]["transit_vehicles"]
                .as_array()
                .map_or(0, Vec::len),
            11
        );

        let snap_str = snap.to_string();
        assert!(!snap_str.contains("測試旅人甲"));
        assert!(!snap_str.contains("secret_player_acc"));

        let dock_detail = build_atlas_node_detail_json(&doc, &delta, "oi:white_gull_harbor:dock")
            .expect("dock exists");
        assert_eq!(dock_detail["ok"], true);
        assert!(!dock_detail.to_string().contains("測試旅人甲"));

        let cell_4614 =
            build_atlas_node_detail_json(&doc, &delta, "cell:4614").expect("cell 4614 exists");
        assert_eq!(cell_4614["cell_title"], "黑煤溪寮");
        assert_eq!(cell_4614["cell_kind"], "rural_settlement");

        assert_eq!(
            observer_third_person_zh(
                "止血黛安",
                "你點出 15 枚銅幣交予諾格（暗記此處凶險）（暗記此處凶險）"
            ),
            "止血黛安點出 15 枚銅幣交予諾格（暗記此處凶險）"
        );
    }
}
