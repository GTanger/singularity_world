// 由 CityGeo 聚類路口、標記、可進建築、生成 model::Room／Exit 與城門邏輯。

use std::collections::{HashMap, HashSet};

use crate::model;

use super::geom::{
    bearing_label, dist_point_polyline, dist_point_polygon_boundary, dist_xy, point_in_polygon,
    polygon_area_abs,
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

struct JunctionDraft {
    cx: f64,
    cy: f64,
    district_idx: Option<usize>,
    near_wall: bool,
    near_river: bool,
    near_square: bool,
    degree: usize,
}

struct BuildingDraft {
    centroid_x: f64,
    centroid_y: f64,
    area: f64,
    district_idx: Option<usize>,
    dist_river: f64,
    dist_square: f64,
}

/// 依 GeoJSON 與世界房出口建出城市批次。
pub fn build_city_rooms(
    geo: &CityGeo,
    world_tags: &[String],
    world_room_id: &str,
    world_room_name: &str,
    world_exits: &[model::Exit],
) -> BuiltCity {
    let zone = format!("city_{}", geo.burg_id);
    let prefix = format!("city_{}_", geo.burg_id);

    let mut eps = 15.0_f64;
    let (_cluster_of_point, centers, edges) = loop {
        let r = cluster_roads(&geo.roads, eps);
        let nj = r.1.len();
        if nj > 500 {
            eps *= 1.5;
            continue;
        }
        if nj < 50 && nj > 0 && eps > 2.0 {
            eps *= 0.75;
            continue;
        }
        break r;
    };

    let n_j = centers.len();
    let mut adj: Vec<HashSet<usize>> = vec![HashSet::new(); n_j];
    for (a, b) in &edges {
        adj[*a].insert(*b);
        adj[*b].insert(*a);
    }

    let junction_meta: Vec<JunctionDraft> = centers
        .iter()
        .enumerate()
        .map(|(ji, (cx, cy))| {
            let district_idx = geo
                .districts
                .iter()
                .position(|d| point_in_polygon(*cx, *cy, &d.ring));
            let near_wall = geo.wall_rings.iter().any(|w| {
                dist_point_polygon_boundary(*cx, *cy, w) < 50.0
            });
            let near_river = geo.river_lines.iter().any(|ln| {
                dist_point_polyline(*cx, *cy, ln) < 30.0
            });
            let near_square = geo.squares.iter().any(|sq| {
                point_in_polygon(*cx, *cy, sq)
                    || dist_point_polygon_boundary(*cx, *cy, sq) < 20.0
            });
            let degree = adj[ji].len();
            JunctionDraft {
                cx: *cx,
                cy: *cy,
                district_idx,
                near_wall,
                near_river,
                near_square,
                degree,
            }
        })
        .collect();

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

    // --- 可進入建築篩選（條件收斂，總量控制在 ≤ 50） ---
    let mut areas_sorted: Vec<f64> = bdrafts.iter().map(|b| b.area).collect();
    areas_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // 取面積前 1%（而非 5%），大城市才不會灌入上千棟
    let threshold_idx = ((areas_sorted.len() as f64) * 0.99).floor() as usize;
    let area_threshold = areas_sorted.get(threshold_idx).copied().unwrap_or(f64::MAX);

    let near_wall_junctions: HashSet<usize> = junction_meta
        .iter()
        .enumerate()
        .filter(|(_, j)| j.near_wall)
        .map(|(i, _)| i)
        .collect();

    let mut enterable: HashSet<usize> = HashSet::new();

    // 條件一：面積前 1% 的巨型建築
    for (bi, b) in bdrafts.iter().enumerate() {
        if b.area >= area_threshold {
            enterable.insert(bi);
        }
    }

    // 條件二：廣場旁面積最大的 1 棟
    if let Some(bi) = bdrafts
        .iter()
        .enumerate()
        .filter(|(_, b)| b.dist_square < 30.0)
        .max_by(|(_, a), (_, b)| a.area.partial_cmp(&b.area).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
    {
        enterable.insert(bi);
    }

    // 條件三：河岸旁面積最大的 1 棟
    if let Some(bi) = bdrafts
        .iter()
        .enumerate()
        .filter(|(_, b)| b.dist_river < 30.0)
        .max_by(|(_, a), (_, b)| a.area.partial_cmp(&b.area).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
    {
        enterable.insert(bi);
    }

    // 條件五：每個 district 至少 1 棟（取最大面積）
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

    // 硬上限：超過 50 棟時只保留面積最大的 50 棟
    if enterable.len() > 50 {
        let mut sorted_entries: Vec<(usize, f64)> = enterable
            .iter()
            .map(|&bi| (bi, bdrafts[bi].area))
            .collect();
        sorted_entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        enterable = sorted_entries.into_iter().take(50).map(|(bi, _)| bi).collect();
    }

    let junc_id: Vec<String> = (0..n_j)
        .map(|i| format!("{}j{:04}", prefix, i))
        .collect();

    let building_to_junction: Vec<usize> = bdrafts
        .iter()
        .map(|b| {
            let mut best_j = 0usize;
            let mut best_d = f64::MAX;
            for (ji, j) in junction_meta.iter().enumerate() {
                let d = dist_xy(b.centroid_x, b.centroid_y, j.cx, j.cy);
                if d < best_d {
                    best_d = d;
                    best_j = ji;
                }
            }
            best_j
        })
        .collect();

    let has_temple_tag = world_tags.iter().any(|t| {
        let x = t.to_lowercase();
        x.contains("temple") || t.contains("廟") || t.contains("寺")
    });

    let mut type_count: HashMap<BuildingType, u32> = HashMap::new();
    let mut enterable_list: Vec<(usize, BuildingType, String)> = Vec::new();

    for &bi in &enterable {
        let b = &bdrafts[bi];
        let ji = building_to_junction[bi];
        let near_gate = near_wall_junctions.contains(&ji);
        let btype = infer_building_type(
            b,
            near_gate,
            has_temple_tag,
            junction_meta[ji].near_square,
        );
        *type_count.entry(btype).or_insert(0) += 1;
        let n = *type_count.get(&btype).unwrap();
        let name = building_display_name(btype, n);
        enterable_list.push((bi, btype, name));
    }

    let district_labels = district_display_names(geo, &junction_meta, &bdrafts);

    let junction_room_names: Vec<String> = junction_meta
        .iter()
        .map(|j| {
            let dname = j
                .district_idx
                .and_then(|d| district_labels.get(d).cloned())
                .unwrap_or_else(|| "城內".to_string());
            format!("{}路口", dname)
        })
        .collect();

    let gate_j = pick_gate_index(&junction_meta, &near_wall_junctions).unwrap_or(0);
    let gate_room_id = junc_id
        .get(gate_j)
        .cloned()
        .unwrap_or_default();
    let gate_room_name = if world_room_name.trim().is_empty() {
        format!("burg {} 城門", geo.burg_id)
    } else {
        format!("{}城門", world_room_name.trim())
    };

    let mut rooms: HashMap<String, model::Room> = HashMap::new();
    let mut exits: HashMap<String, Vec<model::Exit>> = HashMap::new();

    for ji in 0..n_j {
        let id = junc_id[ji].clone();
        let j = &junction_meta[ji];
        let room_name = if ji == gate_j {
            gate_room_name.clone()
        } else {
            junction_room_names[ji].clone()
        };
        let mut tags = merge_tags(world_tags, &["city_junction", "street"]);
        if j.near_wall {
            tags.push("near_wall".to_string());
        }
        if j.near_river {
            tags.push("near_river".to_string());
        }
        if j.near_square {
            tags.push("near_square".to_string());
        }
        rooms.insert(
            id.clone(),
            model::Room {
                id: id.clone(),
                name: room_name,
                tags,
                zone: zone.clone(),
                description: String::new(),
                objects: Vec::new(),
            },
        );
        let mut exs: Vec<model::Exit> = Vec::new();
        for &nj in &adj[ji] {
            let dx = junction_meta[nj].cx - j.cx;
            let dy = junction_meta[nj].cy - j.cy;
            let dir = bearing_label(dx, dy).to_string();
            let to_id = junc_id[nj].clone();
            let to_name = if nj == gate_j {
                gate_room_name.clone()
            } else {
                junction_room_names[nj].clone()
            };
            exs.push(model::Exit {
                direction: dir,
                to_room_id: to_id,
                to_room_name: to_name,
            });
        }
        if ji == gate_j {
            for we in world_exits {
                if we.to_room_id == world_room_id {
                    continue;
                }
                exs.push(model::Exit {
                    direction: we.direction.clone(),
                    to_room_id: we.to_room_id.clone(),
                    to_room_name: we.to_room_name.clone(),
                });
            }
        }
        dedup_exits(&mut exs);
        exits.insert(id, exs);
    }

    for (bi, btype, bname) in enterable_list {
        let bid = format!("{}b{:04}", prefix, bi);
        let ji = building_to_junction[bi];
        let junc_rid = junc_id[ji].clone();
        let mut tags = merge_tags(world_tags, &["city_building"]);
        match btype {
            BuildingType::Shop => tags.push("shop".to_string()),
            BuildingType::Tavern => tags.push("tavern".to_string()),
            BuildingType::Temple => tags.push("temple".to_string()),
            BuildingType::Warehouse => tags.push("warehouse".to_string()),
            BuildingType::Residence => tags.push("residence".to_string()),
        }
        rooms.insert(
            bid.clone(),
            model::Room {
                id: bid.clone(),
                name: bname.clone(),
                tags,
                zone: zone.clone(),
                description: String::new(),
                objects: Vec::new(),
            },
        );
        let jname = rooms
            .get(&junc_rid)
            .map(|r| r.name.clone())
            .unwrap_or_default();
        exits.insert(
            bid.clone(),
            vec![model::Exit {
                direction: "離開".to_string(),
                to_room_id: junc_rid.clone(),
                to_room_name: jname,
            }],
        );
        let enter_dir = format!("進入{}", bname);
        let list = exits.entry(junc_rid).or_default();
        list.push(model::Exit {
            direction: enter_dir,
            to_room_id: bid,
            to_room_name: bname.clone(),
        });
    }

    for ex_list in exits.values_mut() {
        dedup_exits(ex_list);
    }

    for ex_list in exits.values_mut() {
        for ex in ex_list.iter_mut() {
            if ex.to_room_name.is_empty() {
                ex.to_room_name = rooms
                    .get(&ex.to_room_id)
                    .map(|x| x.name.clone())
                    .unwrap_or_default();
            }
        }
    }

    let enterable_count = enterable.len();
    BuiltCity {
        batch: CityInjectBatch { rooms, exits },
        gate_room_id,
        gate_room_name,
        junction_count: n_j,
        enterable_count,
    }
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
    junc_near_square: bool,
) -> BuildingType {
    if near_gate {
        return BuildingType::Tavern;
    }
    if junc_near_square && b.dist_square < 40.0 {
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

fn building_display_name(t: BuildingType, n: u32) -> String {
    match t {
        BuildingType::Shop => format!("街坊舖子·{}", n),
        BuildingType::Tavern => format!("城門酒肆·{}", n),
        BuildingType::Temple => format!("街廟·{}", n),
        BuildingType::Warehouse => format!("河岸貨棧·{}", n),
        BuildingType::Residence => format!("民居·{}", n),
    }
}

fn district_display_names(
    geo: &CityGeo,
    junctions: &[JunctionDraft],
    buildings: &[BuildingDraft],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(geo.districts.len());
    for (di, d) in geo.districts.iter().enumerate() {
        let name = d.name.as_deref().unwrap_or("").trim();
        if !name.is_empty()
            && name
                .chars()
                .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
        {
            out.push(format!("{}片", name));
            continue;
        }
        let jc = junctions
            .iter()
            .filter(|j| j.district_idx == Some(di))
            .count();
        let bc = buildings
            .iter()
            .filter(|b| b.district_idx == Some(di))
            .count();
        let near_river = junctions
            .iter()
            .any(|j| j.district_idx == Some(di) && j.near_river);
        let near_wall = junctions
            .iter()
            .any(|j| j.district_idx == Some(di) && j.near_wall);
        let has_square = junctions
            .iter()
            .any(|j| j.district_idx == Some(di) && j.near_square);
        let label = if has_square && bc > 5 {
            "廣場區"
        } else if near_river && bc > 8 {
            "河畔商區"
        } else if near_wall && bc < 6 {
            "外城片區"
        } else if jc + bc > 25 {
            "民居區"
        } else {
            "靜巷區"
        };
        out.push(format!("{}{}", compass_for_district(di), label));
    }
    out
}

fn compass_for_district(di: usize) -> &'static str {
    match di % 4 {
        0 => "東",
        1 => "南",
        2 => "西",
        _ => "北",
    }
}

fn pick_gate_index(
    junctions: &[JunctionDraft],
    near_wall_set: &HashSet<usize>,
) -> Option<usize> {
    if junctions.is_empty() {
        return None;
    }
    let candidates: Vec<usize> = (0..junctions.len())
        .filter(|i| near_wall_set.contains(i))
        .collect();
    let iter: Vec<usize> = if candidates.is_empty() {
        (0..junctions.len()).collect()
    } else {
        candidates
    };
    let mut best = iter[0];
    let mut best_deg = usize::MAX;
    for &i in &iter {
        let d = junctions[i].degree;
        if d < best_deg || (d == best_deg && i < best) {
            best_deg = d;
            best = i;
        }
    }
    Some(best)
}

type RoadClusterOutcome = (Vec<usize>, Vec<(f64, f64)>, Vec<(usize, usize)>);

/// 回傳 (每個端點的 cluster 編號, 聚類中心, 無向邊集合)。
fn cluster_roads(roads: &[super::Road], eps: f64) -> RoadClusterOutcome {
    let mut points: Vec<(f64, f64)> = Vec::new();
    for r in roads {
        if r.coords.len() < 2 {
            continue;
        }
        let a = *r.coords.first().unwrap();
        let b = *r.coords.last().unwrap();
        points.push(a);
        points.push(b);
    }
    if points.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    let n = points.len();
    let mut uf = UnionFind::new(n);
    for i in 0..n {
        for j in i + 1..n {
            if dist_xy(points[i].0, points[i].1, points[j].0, points[j].1) <= eps {
                uf.union(i, j);
            }
        }
    }
    let mut root_to_idx: HashMap<usize, usize> = HashMap::new();
    let mut centers: Vec<(f64, f64)> = Vec::new();
    let mut cluster_of: Vec<usize> = vec![0; n];
    for (i, slot) in cluster_of.iter_mut().enumerate().take(n) {
        let r = uf.find(i);
        let ci = *root_to_idx.entry(r).or_insert_with(|| {
            let k = centers.len();
            centers.push((0.0, 0.0));
            k
        });
        *slot = ci;
    }
    let mut counts = vec![0usize; centers.len()];
    for i in 0..n {
        let ci = cluster_of[i];
        centers[ci].0 += points[i].0;
        centers[ci].1 += points[i].1;
        counts[ci] += 1;
    }
    for ci in 0..centers.len() {
        let c = counts[ci].max(1) as f64;
        centers[ci].0 /= c;
        centers[ci].1 /= c;
    }

    let mut edges: HashSet<(usize, usize)> = HashSet::new();
    for r in roads {
        if r.coords.len() < 2 {
            continue;
        }
        let a = *r.coords.first().unwrap();
        let b = *r.coords.last().unwrap();
        let ia = nearest_point_index(&points, a.0, a.1);
        let ib = nearest_point_index(&points, b.0, b.1);
        let ca = cluster_of[ia];
        let cb = cluster_of[ib];
        if ca != cb {
            let e = if ca < cb { (ca, cb) } else { (cb, ca) };
            edges.insert(e);
        }
    }
    let edge_vec: Vec<(usize, usize)> = edges.into_iter().collect();
    (cluster_of, centers, edge_vec)
}

fn nearest_point_index(points: &[(f64, f64)], x: f64, y: f64) -> usize {
    let mut best = 0usize;
    let mut best_d = f64::MAX;
    for (i, p) in points.iter().enumerate() {
        let d = dist_xy(x, y, p.0, p.1);
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }
    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }
    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent[rb] = ra;
        }
    }
}
