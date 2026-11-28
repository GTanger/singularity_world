//! 四條實體逐拍車船班線（東西幹線雙馬驛車、南北縱貫重載篷車、北岸環島運貨帆駁、南灣淨岸漁鹽貨駁）
//! 與「二階直覺負重（防單人螞蟻搬家跨城步行、站區 1～2 步自由短駁）」統一引擎。

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::cell_resources::{is_bulk_commodity_slug, lookup_resource_spec};
use super::clock_and_lang::SolarClockSnapshot;
use super::live_tide::derive_tide_clock_details;
use super::movement::{add_carry_item, push_breath};
use super::types::{
    CarryItem, IslandWorldDelta, SpatialNodeDef, TransitState, TransitVehicleState,
    UnifiedMindBody, VehicleCargoCrate, WorldStaticDoc,
};

/// 計算背包內大宗商貨總件數（跨物品格加總，拆箱或分堆絕不降低總量！）
pub fn count_backpack_bulk_units(backpack: &[CarryItem]) -> u32 {
    backpack
        .iter()
        .filter(|i| i.count > 0 && is_bulk_commodity_slug(&i.slug))
        .map(|i| i.count)
        .sum()
}

/// 檢驗單步步行是否違反「二階直覺負重（防螞蟻搬家）」與動態疫氣邊界檢疫禁運、春汛斷橋重載阻隔
pub fn check_bulk_overland_move(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    actor: &UnifiedMindBody,
    src_node: &SpatialNodeDef,
    dst_node: &SpatialNodeDef,
) -> Result<(), String> {
    let src_restricted = super::live_land::is_quarantine_restricted(doc, delta, src_node.cell_id);
    let dst_restricted = super::live_land::is_quarantine_restricted(doc, delta, dst_node.cell_id);
    let crossing_quarantine = src_restricted != dst_restricted;
    if crossing_quarantine
        && actor.backpack.iter().any(|i| {
            i.count > 0
                && matches!(
                    i.slug.as_str(),
                    "home_ration"
                        | "rye_bread"
                        | "fresh_fish_basket"
                        | "salt_fish_barrel"
                        | "shellfish_meat"
                        | "dairy_cheese"
                )
        })
    {
        return Err(
            "檢疫衛兵橫戟攔下：「南岸封鎖區嚴禁私帶生鮮口糧與魚乳進出疫線！請先食畢或寄存北岸，僅苦艾藥束與書信可經煙燻放行。」"
                .to_string(),
        );
    }

    if src_node.cell_id == dst_node.cell_id && (src_node.dungeon || !dst_node.dungeon) {
        return Ok(());
    }
    let bulk_total = count_backpack_bulk_units(&actor.backpack);
    if bulk_total > 0
        && super::live_land::is_river_crossing_impassable_for_wheels(delta, dst_node.cell_id)
    {
        return Err("前方河谷木橋沖毀、春水湍急，背負重貨無法涉水強渡！".to_string());
    }
    let has_dock_crate = actor
        .backpack
        .iter()
        .any(|i| i.slug == "dock_cargo_crate" && i.count > 0);
    if has_dock_crate {
        return Err(
            "肩上正扛著港務封蠟貨箱，只能送往本鎮訂貨鋪子或先放回地面，無法扛著出城遠行！"
                .to_string(),
        );
    }
    let bulk_total = count_backpack_bulk_units(&actor.backpack);
    let max_bulk = super::body_gear_craft::effective_overland_bulk_cap(actor);
    if bulk_total > max_bulk {
        return Err(format!(
            "背囊內裝有 {} 件大宗重貨（超過當前隨身負重上限 {} 件），單人無法徒步翻越城間長路；請就近搭乘驛馬篷車、沿岸貨駁隨行託運，或先穿戴採集背架、存入木箱分批攜帶。",
            bulk_total, max_bulk
        ));
    }
    Ok(())
}

const CN_VEHICLE_NUMS: [&str; 6] = ["一", "二", "三", "四", "五", "六"];

/// 初始化全島 4 條班線共 11 具實體車船，均勻錯開於路線節點上以確保班距 `<= 45~60s`
pub fn ensure_transit_vehicles_initialized(doc: &WorldStaticDoc, delta: &mut IslandWorldDelta) {
    if !delta.transit_vehicles.is_empty() || doc.transit_lines.is_empty() {
        return;
    }
    for line in &doc.transit_lines {
        let rlen = line.route_nodes.len();
        if rlen < 2 || line.vehicle_count == 0 {
            continue;
        }
        for v_i in 0..line.vehicle_count {
            let start_idx = if line.vehicle_count <= 1 {
                0
            } else {
                (v_i * (rlen - 1)) / (line.vehicle_count - 1)
            };
            let forward = v_i % 2 == 0;
            let num_zh = CN_VEHICLE_NUMS.get(v_i).copied().unwrap_or("甲");
            let unit_word = if line.mode == "barge" {
                "號駁船"
            } else {
                "號篷車"
            };
            delta.transit_vehicles.push(TransitVehicleState {
                vehicle_id: format!("{}:v{}", line.line_id, v_i + 1),
                line_id: line.line_id.clone(),
                name_zh: format!("{}・{}{}", line.name_zh, num_zh, unit_word),
                mode: line.mode.clone(),
                route_nodes: line.route_nodes.clone(),
                route_idx: start_idx.min(rlen - 1),
                forward,
                fare_copper: line.fare_copper.max(1),
                passengers: BTreeSet::new(),
                cargo_hold: Vec::new(),
            });
        }
        delta.extras_dirty = true;
    }
}

fn nearest_stop_index_for_node(
    route_nodes: &[String],
    lookup_node: &str,
    cur_idx: usize,
) -> Option<usize> {
    route_nodes
        .iter()
        .enumerate()
        .filter(|(_, rn)| rn.as_str() == lookup_node)
        .min_by_key(|(i, _)| cur_idx.abs_diff(*i))
        .map(|(i, _)| i)
}

/// 推進單一指定載具 1 站（供相容測試 `carriage_next` 呼叫，絕不連帶加速全島其餘 10 具載具！）
fn tick_single_vehicle_by_id(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    target_vid: &str,
) {
    let Some(v_idx) = delta
        .transit_vehicles
        .iter()
        .position(|v| v.vehicle_id == target_vid)
    else {
        return;
    };
    let rlen = delta.transit_vehicles[v_idx].route_nodes.len();
    if rlen < 2 {
        return;
    }
    let cur_idx = delta.transit_vehicles[v_idx].route_idx;
    let mut forward = delta.transit_vehicles[v_idx].forward;
    if forward && cur_idx + 1 >= rlen {
        forward = false;
    } else if !forward && cur_idx == 0 {
        forward = true;
    }
    let next_idx = if forward {
        (cur_idx + 1).min(rlen - 1)
    } else {
        cur_idx.saturating_sub(1)
    };
    delta.transit_vehicles[v_idx].forward = forward;
    delta.transit_vehicles[v_idx].route_idx = next_idx;
    delta.extras_dirty = true;
    let cur_node = delta.transit_vehicles[v_idx].route_nodes[next_idx].clone();
    let v_name = delta.transit_vehicles[v_idx].name_zh.clone();
    let cur_cell = doc.nodes.get(&cur_node).map_or(4616, |n| n.cell_id);
    let cur_node_name = doc
        .nodes
        .get(&cur_node)
        .map_or_else(|| cur_node.clone(), |n| n.name.clone());
    let is_stop = is_major_stop_node(doc, &cur_node);
    let reached_end = next_idx == 0 || next_idx + 1 == rlen;
    let td = derive_tide_clock_details(clock);
    let riders: Vec<String> = delta.transit_vehicles[v_idx]
        .passengers
        .iter()
        .cloned()
        .collect();
    let mut alighted_ids = Vec::new();
    for rid in riders {
        let valid_rider = delta
            .actors
            .get(&rid)
            .is_some_and(|a| a.alive && a.active_transit.is_some());
        if !valid_rider {
            alighted_ids.push(rid);
            continue;
        }
        delta.relocate_actor(&rid, &cur_node, cur_cell, is_stop);
        let should_alight = delta.actors.get_mut(&rid).is_some_and(|a| {
            if let Some(ref mut tr) = a.active_transit {
                tr.step_idx = next_idx;
                tr.target_stop_node.as_deref() == Some(cur_node.as_str())
                    || (tr.alight_next_stop && is_stop)
                    || reached_end
            } else {
                true
            }
        });
        if should_alight {
            if let Some(a) = delta.actors.get_mut(&rid) {
                a.active_transit = None;
                push_breath(
                    a,
                    format!(
                        "〔{:02}:{:02}〕【{}】穩穩停靠【{}】，你拎起隨身行囊下船車踏上實地。",
                        td.hour, td.minute, v_name, cur_node_name
                    ),
                );
                a.mark_mutated();
            }
            super::map_memory::record_local_witness_at_cur_node(doc, delta, &rid, clock.game_days);
            alighted_ids.push(rid);
        }
    }
    for rid in alighted_ids {
        delta.transit_vehicles[v_idx].passengers.remove(&rid);
    }
}

fn is_major_stop_node(doc: &WorldStaticDoc, nid: &str) -> bool {
    doc.nodes.get(nid).is_some_and(|n| {
        matches!(
            n.kind.as_str(),
            "plaza" | "dock" | "village_gate" | "quarantine_post"
        )
    })
}

/// 查詢某節點是否有班線經過，並回傳即時站牌資訊（精確顯示最近一班車船目前所在節點與剩餘步數）
pub fn build_node_transit_signboards(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    node_id: &str,
) -> Vec<Value> {
    let mut out = Vec::new();
    for line in &doc.transit_lines {
        if !line.route_nodes.iter().any(|n| n == node_id) {
            continue;
        }
        let mut best_veh: Option<(&TransitVehicleState, usize)> = None;
        for v in &delta.transit_vehicles {
            if v.line_id != line.line_id {
                continue;
            }
            let Some(my_pos) = nearest_stop_index_for_node(&v.route_nodes, node_id, v.route_idx)
            else {
                continue;
            };
            let dist = v.route_idx.abs_diff(my_pos);
            if best_veh.is_none_or(|(_, bd)| dist < bd) {
                best_veh = Some((v, dist));
            }
        }
        if let Some((veh, dist)) = best_veh {
            let cur_nid = veh
                .route_nodes
                .get(veh.route_idx)
                .map_or("", String::as_str);
            let cur_name = doc.nodes.get(cur_nid).map_or("途中", |n| n.name.as_str());
            let dir_zh = if veh.forward {
                line.route_nodes
                    .last()
                    .and_then(|id| doc.nodes.get(id))
                    .map_or("終點", |n| n.name.split('・').next().unwrap_or(&n.name))
            } else {
                line.route_nodes
                    .first()
                    .and_then(|id| doc.nodes.get(id))
                    .map_or("起點", |n| n.name.split('・').next().unwrap_or(&n.name))
            };
            let route_blocked = line.mode == "carriage"
                && line.route_nodes.iter().any(|rn| {
                    doc.nodes
                        .get(rn)
                        .is_some_and(|nd| super::live_land::is_bridge_washed_out(delta, nd.cell_id))
                });
            let blocked_suffix = if route_blocked {
                "（沿途橋斷受阻）"
            } else {
                ""
            };
            let eta_zh = if dist == 0 {
                format!("現正停靠本站（往{}方向）{}", dir_zh, blocked_suffix)
            } else {
                format!(
                    "行至【{}】，距此 {} 步（往{}）{}",
                    cur_name, dist, dir_zh, blocked_suffix
                )
            };
            out.push(json!({
                "line_id": line.line_id,
                "line_name": line.name_zh,
                "mode": line.mode,
                "fare_copper": line.fare_copper,
                "vehicle_id": veh.vehicle_id,
                "vehicle_name": veh.name_zh,
                "distance_steps": dist,
                "at_node_now": dist == 0,
                "status_zh": eta_zh,
                "cargo_crates": veh.cargo_hold.len(),
                "passenger_count": veh.passengers.len(),
            }));
        }
    }
    out
}

/// 每拍（`2.5s`）推進全島 11 具實體車船沿真實節點移動、同步乘客座標、自動處理鄉鎮雙向貨箱裝卸（嚴守風語橋南向檢疫禁運）
pub fn tick_embodied_transit_vehicles(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    ensure_transit_vehicles_initialized(doc, delta);
    let td = derive_tide_clock_details(clock);

    for v_idx in 0..delta.transit_vehicles.len() {
        let rlen = delta.transit_vehicles[v_idx].route_nodes.len();
        if rlen < 2 {
            continue;
        }
        let cur_idx = delta.transit_vehicles[v_idx].route_idx;
        let at_terminal = cur_idx == 0 || cur_idx + 1 == rlen;
        let has_riders = !delta.transit_vehicles[v_idx].passengers.is_empty();
        let is_land = delta.transit_vehicles[v_idx].mode == "carriage";

        // 深夜（00:00..05:00）陸路篷車若已抵終點且車上無客，於總站歇馬待旦；載客途中必送達！
        if is_land && td.hour < 5 && at_terminal && !has_riders {
            continue;
        }

        let mut forward = delta.transit_vehicles[v_idx].forward;
        if forward && cur_idx + 1 >= rlen {
            forward = false;
        } else if !forward && cur_idx == 0 {
            forward = true;
        }
        let next_idx = if forward {
            (cur_idx + 1).min(rlen - 1)
        } else {
            cur_idx.saturating_sub(1)
        };
        let prev_node_id = delta.transit_vehicles[v_idx].route_nodes[cur_idx].clone();
        let prev_cell = doc.nodes.get(&prev_node_id).map_or(4616, |n| n.cell_id);
        let next_node_id = delta.transit_vehicles[v_idx].route_nodes[next_idx].clone();
        let next_cell = doc.nodes.get(&next_node_id).map_or(4616, |n| n.cell_id);
        let next_is_rural_bridge = prev_cell != next_cell
            || doc.nodes.get(&next_node_id).is_some_and(|nd| {
                matches!(
                    nd.kind.as_str(),
                    "road" | "village_gate" | "quarantine_post"
                )
            });
        if is_land
            && next_is_rural_bridge
            && super::live_land::is_river_crossing_impassable_for_wheels(delta, next_cell)
        {
            delta.transit_vehicles[v_idx].forward = !forward;
            delta.extras_dirty = true;
            let blk_name = doc
                .nodes
                .get(&next_node_id)
                .map_or("前方河谷", |n| n.name.as_str())
                .to_string();
            let riders: Vec<String> = delta.transit_vehicles[v_idx]
                .passengers
                .iter()
                .cloned()
                .collect();
            for rid in riders {
                if let Some(a) = delta.actors.get_mut(&rid) {
                    push_breath(
                        a,
                        format!(
                            "〔{:02}:{:02}〕前方【{}】河水暴漲木橋中斷，重載車馬無法通行，車夫勒馬折返前站！",
                            td.hour, td.minute, blk_name
                        ),
                    );
                    a.mark_mutated();
                }
            }
            continue;
        }
        delta.transit_vehicles[v_idx].forward = forward;
        delta.transit_vehicles[v_idx].route_idx = next_idx;
        delta.extras_dirty = true;

        let cur_node = delta.transit_vehicles[v_idx].route_nodes[next_idx].clone();
        let v_name = delta.transit_vehicles[v_idx].name_zh.clone();
        let cur_cell = doc.nodes.get(&cur_node).map_or(4616, |n| n.cell_id);
        let cur_node_name = doc
            .nodes
            .get(&cur_node)
            .map_or_else(|| cur_node.clone(), |n| n.name.clone());
        let is_stop = is_major_stop_node(doc, &cur_node);
        let reached_end = next_idx == 0 || next_idx + 1 == rlen;

        // 1. 同步車船上所有乘客之真實空間節點，並處理到站下車
        let riders: Vec<String> = delta.transit_vehicles[v_idx]
            .passengers
            .iter()
            .cloned()
            .collect();
        let mut alighted_ids = Vec::new();
        for rid in riders {
            let valid_rider = delta
                .actors
                .get(&rid)
                .is_some_and(|a| a.alive && a.active_transit.is_some());
            if !valid_rider {
                alighted_ids.push(rid);
                continue;
            }
            delta.relocate_actor(&rid, &cur_node, cur_cell, is_stop);
            let should_alight = delta.actors.get_mut(&rid).is_some_and(|a| {
                if let Some(ref mut tr) = a.active_transit {
                    tr.step_idx = next_idx;
                    let hit_target = tr.target_stop_node.as_deref() == Some(cur_node.as_str());
                    let hit_next_stop = tr.alight_next_stop && is_stop;
                    hit_target || hit_next_stop || reached_end
                } else {
                    true
                }
            });
            if should_alight {
                if let Some(a) = delta.actors.get_mut(&rid) {
                    a.active_transit = None;
                    push_breath(
                        a,
                        format!(
                            "〔{:02}:{:02}〕【{}】穩穩停靠【{}】，你拎起隨身行囊下船車踏上實地。",
                            td.hour, td.minute, v_name, cur_node_name
                        ),
                    );
                    a.mark_mutated();
                }
                super::map_memory::record_local_witness_at_cur_node(
                    doc,
                    delta,
                    &rid,
                    clock.game_days,
                );
                alighted_ids.push(rid);
            }
        }
        for rid in alighted_ids {
            delta.transit_vehicles[v_idx].passengers.remove(&rid);
        }

        // 2. 城鄉雙向自動實物貨箱循環（煤鐵木魚乳酪入城 ↔ 乾糧鹽布農具下鄉；4695 風語橋哨嚴禁糧食南運入疫區！）
        let cell_opt = doc.cells.get(&cur_cell.to_string());
        let cell_kind = cell_opt.map_or("", |c| c.kind.as_str());
        let cur_is_quarantine_post = doc
            .nodes
            .get(&cur_node)
            .is_some_and(|n| n.kind == "quarantine_post");
        if cell_kind == "rural_settlement"
            && !cur_is_quarantine_post
            && !super::live_land::is_quarantine_restricted(doc, delta, cur_cell)
        {
            // 卸下城鎮運來之日用補給入本莊戶長木箱
            let mut remaining = Vec::new();
            let mut delivered_rural = 0u32;
            for cr in delta.transit_vehicles[v_idx].cargo_hold.drain(..) {
                if cr.dest_node == "rural_any" && delivered_rural == 0 {
                    delivered_rural += cr.count;
                } else {
                    remaining.push(cr);
                }
            }
            delta.transit_vehicles[v_idx].cargo_hold = remaining;
            if delivered_rural > 0
                && let Some(hh) = doc.households.iter().find(|h| h.cell_id == cur_cell)
                && let Some(hid) = hh.members.first()
                && let Some(h_act) = delta.actors.get_mut(hid)
            {
                let h_home = h_act.home_node.clone();
                let larder = h_act.inn_lockers.entry(h_home).or_default();
                add_carry_item(
                    larder,
                    CarryItem {
                        slug: super::household_economy::HOME_RATION_SLUG.to_string(),
                        name: super::household_economy::HOME_RATION_NAME.to_string(),
                        count: delivered_rural.min(4),
                        weight_units: 1,
                        ..CarryItem::default()
                    },
                );
                h_act.mark_mutated();
            }

            // 若本莊今日資源充沛且車船貨艙未滿，裝載 1 箱本莊特產運往主鎮
            let stock = delta.cell_stocks.get(&cur_cell).copied().unwrap_or(0);
            let res_slug = cell_opt.map_or("", |c| c.resource_slug.as_str());
            if stock >= 4
                && delta.transit_vehicles[v_idx].cargo_hold.len() < 4
                && let Some(spec) = lookup_resource_spec(res_slug)
            {
                delta.cell_stocks.insert(cur_cell, stock - 2);
                delta.transit_vehicles[v_idx]
                    .cargo_hold
                    .push(VehicleCargoCrate {
                        slug: spec.slug.to_string(),
                        name_zh: spec.name_zh.to_string(),
                        count: 2,
                        dest_node: "town_any".to_string(),
                        owner_id: format!("cell:{cur_cell}"),
                    });
            }
        } else if cell_kind == "settlement" && is_stop {
            // 抵達五大主鎮：將鄉間運抵之煤鐵木魚卸入本鎮鋪主木箱，並回裝下鄉補給箱
            let mut kept = Vec::new();
            let mut unloaded: Vec<(String, String, u32)> = Vec::new();
            for cr in delta.transit_vehicles[v_idx].cargo_hold.drain(..) {
                if cr.dest_node == "town_any" || cr.dest_node == cur_node {
                    unloaded.push((cr.slug, cr.name_zh, cr.count));
                } else {
                    kept.push(cr);
                }
            }
            delta.transit_vehicles[v_idx].cargo_hold = kept;
            if !unloaded.is_empty() {
                super::live_commodities::route_unloaded_freight_to_town_shops(
                    doc, delta, cur_cell, unloaded,
                );
            }
            // 回程裝載下鄉農具口糧箱（須從本鎮酒館實扣餘糧，且前方若屬封鎖區絕不裝糧南運！）
            let ahead_idx = if forward {
                (next_idx + 1).min(rlen - 1)
            } else {
                next_idx.saturating_sub(1)
            };
            let ahead_cell = delta.transit_vehicles[v_idx]
                .route_nodes
                .get(ahead_idx)
                .and_then(|nid| doc.nodes.get(nid))
                .map_or(cur_cell, |n| n.cell_id);
            let heading_to_quarantine =
                super::live_land::is_quarantine_restricted(doc, delta, ahead_cell);
            if !heading_to_quarantine && delta.transit_vehicles[v_idx].cargo_hold.len() < 2 {
                let loaded_rural = super::live_commodities::try_load_town_surplus_rations_for_rural(
                    doc, delta, cur_cell,
                );
                if loaded_rural > 0 {
                    delta.transit_vehicles[v_idx]
                        .cargo_hold
                        .push(VehicleCargoCrate {
                            slug: super::household_economy::HOME_RATION_SLUG.to_string(),
                            name_zh: "城鎮鹽布魚麥補給箱".to_string(),
                            count: loaded_rural,
                            dest_node: "rural_any".to_string(),
                            owner_id: format!("town:{cur_cell}"),
                        });
                }
            }
        }
        delta.extras_dirty = true;
    }
}

/// 統一處理乘車／搭船、站牌候車、途中招手攔乘、手動推進一步與「下一站下車／立即下車」
pub fn apply_carriage_action(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    node: &SpatialNodeDef,
    coach_id: Option<&str>,
    action: &str,
    param: &str,
) -> Result<String, String> {
    ensure_transit_vehicles_initialized(doc, delta);
    let aid = delta.resolve_actor_id(actor_key).ok_or("找不到旅人行蹤")?;

    match action {
        "board_carriage_west" | "board_carriage_east" | "board_transit" => {
            if delta
                .actors
                .get(&aid)
                .is_some_and(|a| a.active_transit.is_some())
            {
                return Err("你已在車船座艙內，無需重複購票登乘。".to_string());
            }
            if delta.actors.get(&aid).is_some_and(|a| {
                a.backpack
                    .iter()
                    .any(|i| i.slug == "dock_cargo_crate" && i.count > 0)
            }) {
                return Err(
                    "請先將肩上領籌扛送的本鎮港務貨箱送交鋪內或放下，再登乘遠行車船。".to_string(),
                );
            }
            // 尋找當前節點（或同城鎮廣場/碼頭站點）可搭乘之班線與最近車船
            let lookup_node = if node.name.contains("驛馬") || node.name.contains("車馬") {
                doc.nodes
                    .values()
                    .find(|n| n.cell_id == node.cell_id && n.kind == "plaza")
                    .map_or(node.id.as_str(), |n| n.id.as_str())
            } else {
                node.id.as_str()
            };

            let preferred_line =
                if action == "board_carriage_west" || action == "board_carriage_east" {
                    Some("land_ew_express")
                } else if !param.trim().is_empty() {
                    Some(param.trim())
                } else {
                    None
                };

            let mut chosen_v_idx: Option<(usize, usize, usize)> = None;
            let mut nearest_any: Option<(String, String, usize)> = None;
            let is_major = is_major_stop_node(doc, lookup_node);
            for (idx, v) in delta.transit_vehicles.iter().enumerate() {
                if let Some(pref) = preferred_line
                    && v.line_id != pref
                {
                    continue;
                }
                let Some(stop_pos) =
                    nearest_stop_index_for_node(&v.route_nodes, lookup_node, v.route_idx)
                else {
                    continue;
                };
                let dist = v.route_idx.abs_diff(stop_pos);
                let cur_v_nid = v.route_nodes.get(v.route_idx).map_or("", String::as_str);
                let cur_v_name = doc.nodes.get(cur_v_nid).map_or("途中", |n| n.name.as_str());
                if nearest_any.as_ref().is_none_or(|(_, _, bd)| dist < *bd) {
                    nearest_any = Some((v.name_zh.clone(), cur_v_name.to_string(), dist));
                }
                // 步車分軌＋幹道迎頭攔車：若車上有客限 dist <= 2，空車在幹道攔搭限 dist <= 3（主要大站空車限 dist <= 4）
                let max_allow = if !v.passengers.is_empty() {
                    2
                } else if is_major {
                    4
                } else {
                    3
                };
                if dist > max_allow {
                    continue;
                }
                if chosen_v_idx.is_none_or(|(_, _, bd)| dist < bd) {
                    chosen_v_idx = Some((idx, stop_pos, dist));
                }
            }

            let Some((v_idx, stop_pos, dist)) = chosen_v_idx else {
                if let Some((vname, cur_loc, d_steps)) = nearest_any {
                    return Err(format!(
                        "最近一班【{}】現正行至【{}】（距此 {} 站）；無須原地乾等，你可直接沿官道邁步迎上前去半路招手攔車，或即刻改走山海小徑出發！",
                        vname, cur_loc, d_steps
                    ));
                }
                return Err(
                    "此刻本路段無對應班車或貨駁經過；請至市集廣場或港務碼頭站牌候乘。".to_string(),
                );
            };

            if clock.is_night
                && delta.transit_vehicles[v_idx].mode == "carriage"
                && derive_tide_clock_details(clock).hour < 5
            {
                return Err(
                    "深夜山道露重昏暗，雙馬篷車正於總站歇馬，待五更破曉即刻發車。".to_string(),
                );
            }

            let fare = delta.transit_vehicles[v_idx].fare_copper;
            let v_name = delta.transit_vehicles[v_idx].name_zh.clone();
            let v_id = delta.transit_vehicles[v_idx].vehicle_id.clone();
            let r_nodes = delta.transit_vehicles[v_idx].route_nodes.clone();
            let payee = coach_id.unwrap_or("municipal_treasury");
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "transit_fare",
                &aid,
                payee,
                fare,
                format!("搭乘【{}】票資", v_name),
            ) {
                return Err(format!("搭乘【{}】需要 {} 枚銅幣。", v_name, fare));
            }

            // 僅在無其他乘客時才依上車指令調整空車發車方向，絕不逆轉同車其他乘客行程！
            delta.transit_vehicles[v_idx].route_idx = stop_pos;
            if delta.transit_vehicles[v_idx].passengers.is_empty() {
                if action == "board_carriage_west" {
                    delta.transit_vehicles[v_idx].forward = true;
                } else if action == "board_carriage_east" {
                    delta.transit_vehicles[v_idx].forward = false;
                }
            }
            delta.transit_vehicles[v_idx].passengers.insert(aid.clone());
            delta.extras_dirty = true;

            let target_stop = if action == "board_carriage_west" {
                Some("oi:west_strait_ferry:plaza".to_string())
            } else if action == "board_carriage_east" {
                Some("oi:white_gull_harbor:plaza".to_string())
            } else {
                None
            };

            let stop_cell = doc
                .nodes
                .get(lookup_node)
                .map_or(node.cell_id, |n| n.cell_id);
            delta.relocate_actor(&aid, lookup_node, stop_cell, true);
            let co_riders: Vec<String> = delta.transit_vehicles[v_idx]
                .passengers
                .iter()
                .filter(|id| *id != &aid)
                .filter_map(|id| delta.actors.get(id).map(|a| a.display_name.clone()))
                .collect();
            let co_note = if co_riders.is_empty() {
                "艙內尚有寬敞空位，堆著幾隻封蠟鄉鎮貨箱。".to_string()
            } else {
                format!("同車船還坐著{}，彼此點頭致意。", co_riders.join("、"))
            };

            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.route_plan.clear();
            a.active_transit = Some(TransitState {
                line_name: v_name.clone(),
                route_nodes: r_nodes,
                step_idx: stop_pos,
                started_unix: clock.real_unix,
                vehicle_id: v_id,
                alight_next_stop: false,
                target_stop_node: target_stop,
            });
            let hail_word = if dist == 0 {
                "穩穩登乘"
            } else {
                "招手攔下進站的"
            };
            let msg = format!(
                "你付了 {} 枚銅幣{}【{}】（隨背囊重貨一併安頓上艙，若半途想落腳可隨時向駕手示意「下一站下車」）。{}",
                fare, hail_word, v_name, co_note
            );
            push_breath(a, msg.clone());
            a.mark_mutated();
            Ok(msg)
        }
        "transit_alight_next" => {
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let Some(ref mut tr) = a.active_transit else {
                return Err("你目前並未搭乘任何車船。".to_string());
            };
            tr.alight_next_stop = true;
            let msg = format!(
                "你敲了敲木板向【{}】駕手示意：抵達前方最近一處聚落／碼頭站口即停靠下車。",
                tr.line_name
            );
            push_breath(a, msg.clone());
            a.mark_mutated();
            Ok(msg)
        }
        "carriage_next" => {
            // 相容單拍手動推進測試：僅推進自身所乘單一載具 1 站，絕不牽動全島其餘 10 具載具
            let veh_id_opt = delta
                .actors
                .get(&aid)
                .and_then(|a| a.active_transit.as_ref().map(|t| t.vehicle_id.clone()));
            if let Some(vid) = veh_id_opt
                && !vid.is_empty()
            {
                tick_single_vehicle_by_id(doc, delta, clock, &vid);
                let cur_nid = delta
                    .actors
                    .get(&aid)
                    .map_or_else(String::new, |a| a.node_id.clone());
                let cur_name = doc
                    .nodes
                    .get(&cur_nid)
                    .map_or("前方路段", |n| n.name.as_str());
                return Ok(format!("車船順著路網推進至【{}】。", cur_name));
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.active_transit = None;
            a.mark_mutated();
            Ok("你已抵達目的地並下車。".to_string())
        }
        "carriage_alight" => {
            let veh_id_opt = delta
                .actors
                .get(&aid)
                .and_then(|a| a.active_transit.as_ref().map(|t| t.vehicle_id.clone()));
            if let Some(vid) = veh_id_opt {
                for v in &mut delta.transit_vehicles {
                    if v.vehicle_id == vid {
                        v.passengers.remove(&aid);
                    }
                }
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let Some(tr) = a.active_transit.take() else {
                return Err("你目前並未搭乘任何車船。".to_string());
            };
            let cur_name = doc
                .nodes
                .get(&a.node_id)
                .map_or("路旁", |n| n.name.as_str());
            let msg = format!(
                "你示意【{}】靠邊停穩，拎起行囊在【{}】中途落腳。",
                tr.line_name, cur_name
            );
            push_breath(a, msg.clone());
            a.mark_mutated();
            Ok(msg)
        }
        _ => Err("未知的車船指令。".to_string()),
    }
}

pub fn should_board_ew_carriage(
    doc: &WorldStaticDoc,
    node: &SpatialNodeDef,
    copper: i64,
    dst_nid: &str,
) -> bool {
    let dst_cell = doc.nodes.get(dst_nid).map_or(0, |n| n.cell_id);
    node.kind == "plaza"
        && copper >= 6
        && dst_cell != node.cell_id
        && ((node.id == "oi:white_gull_harbor:plaza" && dst_nid.contains("west_strait"))
            || (node.id == "oi:west_strait_ferry:plaza" && dst_nid.contains("white_gull")))
}

pub fn find_local_node_by_kind(doc: &WorldStaticDoc, cell_id: u32, kind: &str) -> Option<String> {
    doc.nodes
        .iter()
        .find(|(_, n)| n.cell_id == cell_id && n.kind == kind)
        .map(|(id, _)| id.clone())
}

/// 推算乘客當前所乘車船之「下一處前方停靠地名」（純世界內方位地名，零量化進度條）
pub fn resolve_next_stop_name_zh(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    tr: &super::types::TransitState,
) -> String {
    if let Some(veh) = delta
        .transit_vehicles
        .iter()
        .find(|v| !tr.vehicle_id.is_empty() && v.vehicle_id == tr.vehicle_id)
    {
        let rlen = veh.route_nodes.len();
        if rlen >= 2 {
            let indices: Vec<usize> = if veh.forward {
                ((veh.route_idx + 1)..rlen).collect()
            } else {
                (0..veh.route_idx).rev().collect()
            };
            for idx in &indices {
                let nid = &veh.route_nodes[*idx];
                if is_major_stop_node(doc, nid)
                    && let Some(nd) = doc.nodes.get(nid)
                {
                    return nd.name.clone();
                }
            }
            if let Some(&first_next) = indices.first()
                && let Some(nd) = doc.nodes.get(&veh.route_nodes[first_next])
            {
                return nd.name.clone();
            }
        }
    }
    if let Some(ref tgt) = tr.target_stop_node
        && let Some(nd) = doc.nodes.get(tgt)
    {
        return nd.name.clone();
    }
    tr.route_nodes
        .get(tr.step_idx + 1)
        .and_then(|nid| doc.nodes.get(nid))
        .map_or_else(|| "前方站口".to_string(), |n| n.name.clone())
}
