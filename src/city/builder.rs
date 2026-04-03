// 由 CityGeo 建「語意層」城市：城門 + 各區坊市節點、往×× 命名出口、可進建築掛在區上；幾何細節不逐路口展開。

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::model;

use super::ambience::{load_city_ambience, mix_seed_index, CityAmbienceConfig};
use super::geom::{
    dist_point_polyline, dist_point_polygon_boundary, dist_xy, point_in_polygon, polygon_area_abs,
};
use super::CityGeo;

/// 一批城市房間與出口（僅注入記憶體，不寫檔）。
pub struct CityInjectBatch {
    pub rooms: HashMap<String, model::Room>,
    pub exits: HashMap<String, Vec<model::Exit>>,
}

/// builder 輸出：供 load_and_inject 使用。
pub struct BuiltCity {
    pub batch: CityInjectBatch,
    pub gate_room_id: String,
    pub gate_room_name: String,
    /// 語意節點數：1 城門 + 各區坊（非幾何路口數）。
    pub junction_count: usize,
    pub enterable_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuildingType {
    Shop,
    Tavern,
    Temple,
    Warehouse,
    Residence,
}

struct BuildingDraft {
    centroid_x: f64,
    centroid_y: f64,
    area: f64,
    district_idx: Option<usize>,
    dist_river: f64,
    dist_square: f64,
}

struct HubInfo {
    cx: f64,
    cy: f64,
    label: String,
    near_wall: bool,
    near_river: bool,
    near_square: bool,
    building_count: usize,
}

/// 依 GeoJSON 與世界房出口建出城市批次（語意圖 + 環境描述）。
pub fn build_city_rooms(
    geo: &CityGeo,
    world_tags: &[String],
    world_room_id: &str,
    world_room_name: &str,
    world_exits: &[model::Exit],
    ambience_path: Option<&Path>,
) -> BuiltCity {
    let zone = format!("city_{}", geo.burg_id);
    let prefix = format!("city_{}_", geo.burg_id);
    let gate_id = format!("{}gate", prefix);
    let ambience = load_city_ambience(ambience_path);
    let is_port_city = world_tags.iter().any(|t| {
        let x = t.to_lowercase();
        x == "naval" || x == "port" || t.contains("港")
    });
    let seed_base = (geo.burg_id as u64) << 8;

    let bdrafts: Vec<BuildingDraft> = geo
        .buildings
        .iter()
        .map(|b| {
            let area = polygon_area_abs(&b.ring);
            let (sx, sy): (f64, f64) = b
                .ring
                .iter()
                .fold((0.0, 0.0), |acc, p| (acc.0 + p.0, acc.1 + p.1));
            let n = b.ring.len().max(1) as f64;
            let centroid_x = sx / n;
            let centroid_y = sy / n;
            let district_idx = geo
                .districts
                .iter()
                .position(|d| point_in_polygon(centroid_x, centroid_y, &d.ring));
            let dist_river = geo
                .river_lines
                .iter()
                .map(|ln| dist_point_polyline(centroid_x, centroid_y, ln))
                .fold(f64::MAX, f64::min);
            let dist_square = geo
                .squares
                .iter()
                .map(|sq| {
                    if point_in_polygon(centroid_x, centroid_y, sq) {
                        0.0
                    } else {
                        dist_point_polygon_boundary(centroid_x, centroid_y, sq)
                    }
                })
                .fold(f64::MAX, f64::min);
            BuildingDraft {
                centroid_x,
                centroid_y,
                area,
                district_idx,
                dist_river,
                dist_square,
            }
        })
        .collect();

    let n_d = if geo.districts.is_empty() {
        1usize
    } else {
        geo.districts.len()
    };

    let mut hubs: Vec<HubInfo> = Vec::with_capacity(n_d);
    for di in 0..n_d {
        let (cx, cy) = if geo.districts.is_empty() {
            centroid_all_buildings(&bdrafts)
        } else {
            ring_centroid(&geo.districts[di].ring)
        };
        let near_wall = geo.wall_rings.iter().any(|w| {
            dist_point_polygon_boundary(cx, cy, w) < 50.0
        });
        let near_river = geo
            .river_lines
            .iter()
            .any(|ln| dist_point_polyline(cx, cy, ln) < 40.0);
        let near_square = geo.squares.iter().any(|sq| {
            point_in_polygon(cx, cy, sq) || dist_point_polygon_boundary(cx, cy, sq) < 25.0
        });
        let building_count = if geo.districts.is_empty() {
            bdrafts.len()
        } else {
            bdrafts
                .iter()
                .filter(|b| b.district_idx == Some(di))
                .count()
        };
        let label = district_label_semantic(geo, di, cx, cy, &bdrafts, &ambience);
        hubs.push(HubInfo {
            cx,
            cy,
            label,
            near_wall,
            near_river,
            near_square,
            building_count,
        });
    }

    let gate_di = pick_gate_district(&hubs);
    let gate_room_name = if world_room_name.trim().is_empty() {
        format!("burg {} 城門", geo.burg_id)
    } else {
        format!("{}城門", world_room_name.trim())
    };

    let enterable = select_enterable_buildings(geo, &bdrafts);
    let has_temple_tag = world_tags.iter().any(|t| {
        let x = t.to_lowercase();
        x.contains("temple") || t.contains("廟") || t.contains("寺")
    });

    let mut rooms: HashMap<String, model::Room> = HashMap::new();
    let mut exits: HashMap<String, Vec<model::Exit>> = HashMap::new();

    // 城門房
    let gate_desc = ambience.pick_gate(seed_base);
    rooms.insert(
        gate_id.clone(),
        model::Room {
            id: gate_id.clone(),
            name: gate_room_name.clone(),
            tags: merge_tags(world_tags, &["city_gate", "city_junction", "street"]),
            zone: zone.clone(),
            description: gate_desc,
            objects: Vec::new(),
        },
    );

    // 各區坊
    for (di, h) in hubs.iter().enumerate() {
        let hid = format!("{}d{:03}", prefix, di);
        let room_name = ambience.format_district_room_display(
            world_room_name.trim(),
            &h.label,
            di,
            geo.burg_id,
        );
        let d_seed = seed_base
            .wrapping_add((di as u64).wrapping_mul(0xBF58_476D))
            .wrapping_add(h.cx.to_bits() ^ h.cy.to_bits().rotate_left(17));
        let desc = ambience.pick_district(
            d_seed,
            h.near_river,
            h.near_wall,
            h.near_square,
            is_port_city,
        );
        rooms.insert(
            hid.clone(),
            model::Room {
                id: hid.clone(),
                name: room_name,
                tags: merge_tags(world_tags, &["city_district", "city_junction", "street"]),
                zone: zone.clone(),
                description: desc,
                objects: Vec::new(),
            },
        );
    }

    // 坊數過多時拆「外街」扇區：城門只連外街（甲乙丙…），外街再連轄下各坊。
    const SECTOR_BRANCH: [&str; 6] = ["甲", "乙", "丙", "丁", "戊", "己"];
    let flat_max = ambience.gate_flat_max();
    let use_sectors = n_d > flat_max && n_d > 1;
    let dps = ambience.districts_per_sector();
    let cap = ambience.sector_cap().min(SECTOR_BRANCH.len());
    let n_sectors = if use_sectors {
        let raw = n_d.div_ceil(dps);
        raw.min(cap).max(2).min(n_d)
    } else {
        0usize
    };
    let sector_chunks: Option<Vec<Vec<usize>>> = if use_sectors && n_sectors > 0 {
        Some(partition_districts_by_angle(&hubs, n_sectors))
    } else {
        None
    };

    let mut di_to_sector: Vec<usize> = vec![0; n_d];
    let mut sector_ids: Vec<String> = Vec::new();
    if let Some(ref chunks) = sector_chunks {
        for (si, chunk) in chunks.iter().enumerate() {
            for &di in chunk {
                if di < n_d {
                    di_to_sector[di] = si;
                }
            }
            let sid = format!("{}s{:02}", prefix, si);
            sector_ids.push(sid.clone());
            let short = ambience.sector_short_name(si, geo.burg_id);
            let sroom_name = ambience.format_sector_room_display(
                world_room_name.trim(),
                si,
                geo.burg_id,
                &short,
            );
            let sdesc = ambience.pick_sector_street(
                seed_base
                    .wrapping_add(0x200)
                    .wrapping_add(si as u64 * 0xD1CE),
            );
            rooms.insert(
                sid.clone(),
                model::Room {
                    id: sid.clone(),
                    name: sroom_name,
                    tags: merge_tags(world_tags, &["city_sector", "city_junction", "street"]),
                    zone: zone.clone(),
                    description: sdesc,
                    objects: Vec::new(),
                },
            );
        }
    }

    // 城門出口：直連各坊，或只連外街扇區 + 世界地圖
    let mut gate_exits: Vec<model::Exit> = Vec::new();
    if let Some(ref chunks) = sector_chunks {
        for (si, sid) in sector_ids.iter().enumerate() {
            if si >= chunks.len() {
                break;
            }
            let sn = ambience.sector_short_name(si, geo.burg_id);
            let to_name = rooms.get(sid).map(|r| r.name.clone()).unwrap_or_default();
            gate_exits.push(model::Exit {
                direction: format!("往{}", sn),
                to_room_id: sid.clone(),
                to_room_name: to_name,
            });
        }
    } else {
        for (di, h) in hubs.iter().enumerate() {
            let hid = format!("{}d{:03}", prefix, di);
            let to_name = rooms.get(&hid).map(|r| r.name.clone()).unwrap_or_default();
            gate_exits.push(model::Exit {
                direction: format!("往{}", h.label),
                to_room_id: hid,
                to_room_name: to_name,
            });
        }
    }
    for we in world_exits {
        if we.to_room_id == world_room_id {
            continue;
        }
        gate_exits.push(model::Exit {
            direction: we.direction.clone(),
            to_room_id: we.to_room_id.clone(),
            to_room_name: we.to_room_name.clone(),
        });
    }
    dedup_exits(&mut gate_exits);
    exits.insert(gate_id.clone(), gate_exits);

    // 外街扇區：進城門 + 往轄下各坊
    if let Some(ref chunks) = sector_chunks {
        for (si, chunk) in chunks.iter().enumerate() {
            let Some(sid) = sector_ids.get(si) else {
                continue;
            };
            let mut sexs: Vec<model::Exit> = vec![model::Exit {
                direction: "進城門".to_string(),
                to_room_id: gate_id.clone(),
                to_room_name: gate_room_name.clone(),
            }];
            for &di in chunk {
                let h = &hubs[di];
                let hid = format!("{}d{:03}", prefix, di);
                let to_name = rooms.get(&hid).map(|r| r.name.clone()).unwrap_or_default();
                sexs.push(model::Exit {
                    direction: format!("往{}", h.label),
                    to_room_id: hid,
                    to_room_name: to_name,
                });
            }
            dedup_exits(&mut sexs);
            exits.insert(sid.clone(), sexs);
        }
    }

    // 區與區互連：每區接最近 3 個其他區 + 回城門／退回外街
    let neighbor_count = 3usize;
    for (di, h) in hubs.iter().enumerate() {
        let hid = format!("{}d{:03}", prefix, di);
        let back_exit = if sector_chunks.is_some() && !sector_ids.is_empty() {
            let si = di_to_sector[di];
            let sid = sector_ids
                .get(si)
                .cloned()
                .unwrap_or_else(|| format!("{}s00", prefix));
            let sn = ambience.sector_short_name(si, geo.burg_id);
            let sname = rooms.get(&sid).map(|r| r.name.clone()).unwrap_or_default();
            model::Exit {
                direction: format!("退回{}", sn),
                to_room_id: sid,
                to_room_name: sname,
            }
        } else {
            model::Exit {
                direction: "回城門".to_string(),
                to_room_id: gate_id.clone(),
                to_room_name: gate_room_name.clone(),
            }
        };
        let mut exs: Vec<model::Exit> = vec![back_exit];
        let mut others: Vec<(usize, f64)> = (0..n_d)
            .filter(|&j| j != di)
            .map(|j| {
                let d = dist_xy(h.cx, h.cy, hubs[j].cx, hubs[j].cy);
                (j, d)
            })
            .collect();
        others.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (j, _) in others.iter().take(neighbor_count) {
            let hj = &hubs[*j];
            let jid = format!("{}d{:03}", prefix, j);
            let jname = rooms.get(&jid).map(|r| r.name.clone()).unwrap_or_default();
            exs.push(model::Exit {
                direction: format!("往{}", hj.label),
                to_room_id: jid,
                to_room_name: jname,
            });
        }
        dedup_exits(&mut exs);
        exits.insert(hid, exs);
    }

    // 可進建築：每區最多 3 棟（面積大者優先），出口用「進入××」
    let mut per_district: HashMap<usize, Vec<(usize, f64)>> = HashMap::new();
    for &bi in &enterable {
        let di = if geo.districts.is_empty() {
            0usize
        } else {
            bdrafts[bi].district_idx.unwrap_or(gate_di)
        };
        let area = bdrafts[bi].area;
        per_district.entry(di).or_default().push((bi, area));
    }
    for v in per_district.values_mut() {
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(3);
    }

    let mut building_count = 0usize;
    for (di, list) in &per_district {
        let hid = format!("{}d{:03}", prefix, *di);
        let hub = &hubs[*di];
        for &(bi, _) in list {
            let b = &bdrafts[bi];
            let near_gate = *di == gate_di;
            let btype = infer_building_type(
                b,
                near_gate,
                has_temple_tag,
                hub.near_square,
            );
            let b_seed = seed_base
                .wrapping_add(bi as u64 * 0xC13F_A9A9)
                .wrapping_add(b.centroid_x.to_bits() ^ b.centroid_y.to_bits().rotate_left(11));
            let kind_str = building_type_kind_str(btype);
            let bname = ambience.building_place_name(kind_str, b_seed);
            let bid = format!("{}b{:04}", prefix, bi);
            let mut tags = merge_tags(world_tags, &["city_building"]);
            match btype {
                BuildingType::Shop => tags.push("shop".to_string()),
                BuildingType::Tavern => tags.push("tavern".to_string()),
                BuildingType::Temple => tags.push("temple".to_string()),
                BuildingType::Warehouse => tags.push("warehouse".to_string()),
                BuildingType::Residence => tags.push("residence".to_string()),
            }
            let b_desc = ambience.pick_building_interior(seed_base + 0x1000 + bi as u64);
            rooms.insert(
                bid.clone(),
                model::Room {
                    id: bid.clone(),
                    name: bname.clone(),
                    tags,
                    zone: zone.clone(),
                    description: b_desc,
                    objects: Vec::new(),
                },
            );
            let jname = rooms.get(&hid).map(|r| r.name.clone()).unwrap_or_default();
            exits.insert(
                bid.clone(),
                vec![model::Exit {
                    direction: "離開".to_string(),
                    to_room_id: hid.clone(),
                    to_room_name: jname,
                }],
            );
            let enter_dir = format!("進入{}", bname);
            exits.entry(hid.clone()).or_default().push(model::Exit {
                direction: enter_dir,
                to_room_id: bid,
                to_room_name: bname.clone(),
            });
            building_count += 1;
        }
    }

    for ex_list in exits.values_mut() {
        dedup_exits(ex_list);
        for ex in ex_list.iter_mut() {
            if ex.to_room_name.is_empty() {
                ex.to_room_name = rooms
                    .get(&ex.to_room_id)
                    .map(|x| x.name.clone())
                    .unwrap_or_default();
            }
        }
    }

    let extra_sectors = sector_ids.len();
    let semantic_nodes = 1 + n_d + extra_sectors;
    BuiltCity {
        batch: CityInjectBatch { rooms, exits },
        gate_room_id: gate_id,
        gate_room_name,
        junction_count: semantic_nodes,
        enterable_count: building_count,
    }
}

/// 依各坊中心相對全城中心的方位角排序，再切成 `n_sectors` 段（每段一條外街）。
fn partition_districts_by_angle(hubs: &[HubInfo], n_sectors: usize) -> Vec<Vec<usize>> {
    let n = hubs.len();
    if n == 0 || n_sectors == 0 {
        return Vec::new();
    }
    let k = n_sectors.min(n).max(1);
    let mx: f64 = hubs.iter().map(|h| h.cx).sum::<f64>() / n as f64;
    let my: f64 = hubs.iter().map(|h| h.cy).sum::<f64>() / n as f64;
    let mut items: Vec<(usize, f64)> = hubs
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let ang = (h.cy - my).atan2(h.cx - mx);
            (i, ang)
        })
        .collect();
    items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
    let base = n / k;
    let rem = n % k;
    let mut out: Vec<Vec<usize>> = Vec::with_capacity(k);
    let mut idx = 0usize;
    for s in 0..k {
        let size = base + if s < rem { 1 } else { 0 };
        let end = (idx + size).min(n);
        let chunk: Vec<usize> = items[idx..end].iter().map(|(i, _)| *i).collect();
        idx = end;
        out.push(chunk);
    }
    out
}

fn pick_gate_district(hubs: &[HubInfo]) -> usize {
    let wall_candidates: Vec<usize> = hubs
        .iter()
        .enumerate()
        .filter(|(_, h)| h.near_wall)
        .map(|(i, _)| i)
        .collect();
    let iter: Vec<usize> = if wall_candidates.is_empty() {
        (0..hubs.len()).collect()
    } else {
        wall_candidates
    };
    let mut best = *iter.first().unwrap_or(&0);
    let mut best_bc = usize::MAX;
    for &i in &iter {
        let bc = hubs[i].building_count;
        if bc < best_bc || (bc == best_bc && i < best) {
            best_bc = bc;
            best = i;
        }
    }
    best
}

fn ring_centroid(ring: &[(f64, f64)]) -> (f64, f64) {
    if ring.is_empty() {
        return (0.0, 0.0);
    }
    let (sx, sy) = ring.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
    let n = ring.len() as f64;
    (sx / n, sy / n)
}

fn centroid_all_buildings(bdrafts: &[BuildingDraft]) -> (f64, f64) {
    if bdrafts.is_empty() {
        return (0.0, 0.0);
    }
    let sx: f64 = bdrafts.iter().map(|b| b.centroid_x).sum();
    let sy: f64 = bdrafts.iter().map(|b| b.centroid_y).sum();
    let n = bdrafts.len() as f64;
    (sx / n, sy / n)
}

/// 產生區坊語意短標（用於房名與「往××」）；吃 GeoJSON 區名或幾何推斷，避免固定「東隅××區」。
fn district_label_semantic(
    geo: &CityGeo,
    di: usize,
    hub_cx: f64,
    hub_cy: f64,
    bdrafts: &[BuildingDraft],
    ambience: &CityAmbienceConfig,
) -> String {
    let seed = hub_cx
        .to_bits()
        .wrapping_mul(0x9E37)
        .wrapping_add(hub_cy.to_bits())
        .wrapping_add((di as u64).wrapping_mul(0x1F));
    if geo.districts.is_empty() {
        return "城內".to_string();
    }
    let d = &geo.districts[di];
    let raw_name = d.name.as_deref().unwrap_or("").trim();
    if !raw_name.is_empty() {
        let has_cjk = raw_name
            .chars()
            .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
        if has_cjk {
            return ambience.format_chinese_district_raw(raw_name, seed);
        }
        return ambience.foreign_district_label(raw_name, seed);
    }
    let bc = bdrafts
        .iter()
        .filter(|b| b.district_idx == Some(di))
        .count();
    let near_river = geo
        .river_lines
        .iter()
        .any(|ln| dist_point_polyline(hub_cx, hub_cy, ln) < 40.0);
    let near_wall = geo.wall_rings.iter().any(|w| {
        dist_point_polygon_boundary(hub_cx, hub_cy, w) < 50.0
    });
    let near_square = geo.squares.iter().any(|sq| {
        point_in_polygon(hub_cx, hub_cy, sq)
            || dist_point_polygon_boundary(hub_cx, hub_cy, sq) < 25.0
    });
    let cores_square = ["市集旁", "攤影裡", "鐘樓下"];
    let cores_river = ["水埠邊", "漕聲處", "繫船石旁"];
    let cores_wall = ["牆腳巷", "垛口影下", "護城內側"];
    let cores_dense = ["連簷下", "煙突旁", "馬頭牆間"];
    let cores_quiet = ["深巷底", "僻角", "少人踏處"];
    let (pool, salt): (&[&str], u32) = if near_square && bc > 5 {
        (&cores_square, 41)
    } else if near_river && bc > 8 {
        (&cores_river, 43)
    } else if near_wall && bc < 6 {
        (&cores_wall, 47)
    } else if bc > 25 {
        (&cores_dense, 49)
    } else {
        (&cores_quiet, 51)
    };
    let core = pool[mix_seed_index(seed, salt, pool.len())];
    let epi = ambience.pick_district_epithet(seed.wrapping_add((di as u64) << 32));
    if epi.is_empty() {
        core.to_string()
    } else {
        format!("{}{}", epi, core)
    }
}

fn building_type_kind_str(t: BuildingType) -> &'static str {
    match t {
        BuildingType::Shop => "shop",
        BuildingType::Tavern => "tavern",
        BuildingType::Temple => "temple",
        BuildingType::Warehouse => "warehouse",
        BuildingType::Residence => "residence",
    }
}

fn select_enterable_buildings(geo: &CityGeo, bdrafts: &[BuildingDraft]) -> HashSet<usize> {
    let mut areas_sorted: Vec<f64> = bdrafts.iter().map(|b| b.area).collect();
    areas_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let threshold_idx = ((areas_sorted.len() as f64) * 0.99).floor() as usize;
    let area_threshold = areas_sorted.get(threshold_idx).copied().unwrap_or(f64::MAX);

    let mut enterable: HashSet<usize> = HashSet::new();
    for (bi, b) in bdrafts.iter().enumerate() {
        if b.area >= area_threshold {
            enterable.insert(bi);
        }
    }
    if let Some(bi) = bdrafts
        .iter()
        .enumerate()
        .filter(|(_, b)| b.dist_square < 30.0)
        .max_by(|(_, a), (_, b)| a.area.partial_cmp(&b.area).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
    {
        enterable.insert(bi);
    }
    if let Some(bi) = bdrafts
        .iter()
        .enumerate()
        .filter(|(_, b)| b.dist_river < 30.0)
        .max_by(|(_, a), (_, b)| a.area.partial_cmp(&b.area).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
    {
        enterable.insert(bi);
    }
    for di in 0..geo.districts.len() {
        let mut best: Option<(usize, f64)> = None;
        for (bi, b) in bdrafts.iter().enumerate() {
            if b.district_idx != Some(di) {
                continue;
            }
            let cur = best.map(|(_, a)| a).unwrap_or(-1.0);
            if b.area > cur {
                best = Some((bi, b.area));
            }
        }
        if let Some((bi, _)) = best {
            enterable.insert(bi);
        }
    }
    if enterable.len() > 50 {
        let mut sorted_entries: Vec<(usize, f64)> = enterable
            .iter()
            .map(|&bi| (bi, bdrafts[bi].area))
            .collect();
        sorted_entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        enterable = sorted_entries.into_iter().take(50).map(|(bi, _)| bi).collect();
    }
    enterable
}

fn merge_tags(world: &[String], extra: &[&str]) -> Vec<String> {
    let mut v = world.to_vec();
    for t in extra {
        let s = (*t).to_string();
        if !v.iter().any(|x| x == &s) {
            v.push(s);
        }
    }
    v
}

fn dedup_exits(exs: &mut Vec<model::Exit>) {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    exs.retain(|e| seen.insert((e.direction.clone(), e.to_room_id.clone())));
}

fn infer_building_type(
    b: &BuildingDraft,
    near_gate: bool,
    has_temple_tag: bool,
    hub_near_square: bool,
) -> BuildingType {
    if near_gate {
        return BuildingType::Tavern;
    }
    if hub_near_square && b.dist_square < 40.0 {
        return BuildingType::Shop;
    }
    if b.dist_river < 35.0 && b.area > 200.0 {
        return BuildingType::Warehouse;
    }
    if has_temple_tag && b.area > 300.0 {
        return BuildingType::Temple;
    }
    BuildingType::Residence
}

