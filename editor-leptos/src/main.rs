mod api;
mod types;
mod components;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use std::collections::{HashMap, HashSet, VecDeque};
use types::{HexCell, HexCoord, Terrain, ToolMode};
use components::hex_grid::HexGrid;
use components::cell_editor::CellEditor;
use components::toolbar::Toolbar;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

fn parse_qr_from_key(key: &str) -> Option<(i32, i32)> {
    // 例如 q53_r0, q-2_r14
    if !key.starts_with('q') {
        return None;
    }
    let mid = key.find("_r")?;
    let q_str = &key[1..mid];
    let r_str = &key[(mid + 2)..];
    let q = q_str.parse::<i32>().ok()?;
    let r = r_str.parse::<i32>().ok()?;
    Some((q, r))
}

const IMPORT_HEX_R: f64 = 28.0;
const IMPORT_SQRT3: f64 = 1.7320508075688772;

fn axial_round_pair(q: f64, r: f64) -> (i32, i32) {
    let mut x = q;
    let mut z = r;
    let y = -x - z;

    let rx = x.round();
    let ry = y.round();
    let rz = z.round();

    let x_diff = (rx - x).abs();
    let y_diff = (ry - y).abs();
    let z_diff = (rz - z).abs();

    if x_diff > y_diff && x_diff > z_diff {
        x = -ry - rz;
        z = rz;
    } else if y_diff > z_diff {
        x = rx;
        z = rz;
    } else {
        x = rx;
        z = -rx - ry;
    }

    (x as i32, z as i32)
}

fn pixel_to_axial_pair(wx: f64, wy: f64) -> (f64, f64) {
    let q = (IMPORT_SQRT3 / 3.0 * wx - (1.0 / 3.0) * wy) / IMPORT_HEX_R;
    let r = ((2.0 / 3.0) * wy) / IMPORT_HEX_R;
    (q, r)
}

fn coord_to_pixel_pair(q: i32, r: i32) -> (f64, f64) {
    let x = IMPORT_HEX_R * IMPORT_SQRT3 * (q as f64 + r as f64 / 2.0);
    let y = IMPORT_HEX_R * 1.5 * r as f64;
    (x, y)
}

fn parse_point(v: &serde_json::Value) -> Option<(f64, f64)> {
    let arr = v.as_array()?;
    let x = arr.first()?.as_f64()?;
    let y = arr.get(1)?.as_f64()?;
    Some((x, y))
}

fn point_in_polygon(pt: (f64, f64), ring: &[(f64, f64)]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let (px, py) = pt;
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let (xi, yi) = ring[i];
        let (xj, yj) = ring[j];
        let intersects = ((yi > py) != (yj > py))
            && (px < (xj - xi) * (py - yi) / ((yj - yi).abs().max(1e-12)) + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn sample_line_to_cells(points: &[(f64, f64)], map: &mut HashMap<(i32, i32), (Terrain, String)>, terrain: Terrain, tag: &str) {
    if points.len() < 2 {
        return;
    }
    for w in points.windows(2) {
        let (x1, y1) = w[0];
        let (x2, y2) = w[1];
        let dx = x2 - x1;
        let dy = y2 - y1;
        let dist = (dx * dx + dy * dy).sqrt();
        let steps = ((dist / (IMPORT_HEX_R * 0.6)).ceil() as i32).max(1);
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let x = x1 + dx * t;
            let y = y1 + dy * t;
            let (aq, ar) = pixel_to_axial_pair(x, y);
            let (q, r) = axial_round_pair(aq, ar);
            map.insert((q, r), (terrain, tag.to_string()));
        }
    }
}

fn fill_polygon_to_cells(ring: &[(f64, f64)], map: &mut HashMap<(i32, i32), (Terrain, String)>, terrain: Terrain, tag: &str) {
    if ring.len() < 3 {
        return;
    }
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in ring {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }
    let corners = [(min_x, min_y), (max_x, min_y), (min_x, max_y), (max_x, max_y)];
    let mut qf_min = f64::INFINITY;
    let mut qf_max = f64::NEG_INFINITY;
    let mut rf_min = f64::INFINITY;
    let mut rf_max = f64::NEG_INFINITY;
    for (x, y) in corners {
        let (aq, ar) = pixel_to_axial_pair(x, y);
        qf_min = qf_min.min(aq);
        qf_max = qf_max.max(aq);
        rf_min = rf_min.min(ar);
        rf_max = rf_max.max(ar);
    }
    let q_min = qf_min.floor() as i32 - 2;
    let q_max = qf_max.ceil() as i32 + 2;
    let r_min = rf_min.floor() as i32 - 2;
    let r_max = rf_max.ceil() as i32 + 2;
    for q in q_min..=q_max {
        for r in r_min..=r_max {
            let (cx, cy) = coord_to_pixel_pair(q, r);
            if point_in_polygon((cx, cy), ring) {
                map.insert((q, r), (terrain, tag.to_string()));
            }
        }
    }
}

fn extract_mfcg_cells(parsed: &serde_json::Value) -> Vec<(i32, i32, Terrain, String)> {
    if parsed.get("type").and_then(|v| v.as_str()) != Some("FeatureCollection") {
        return Vec::new();
    }
    let Some(features) = parsed.get("features").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    let mut map = HashMap::<(i32, i32), (Terrain, String)>::new();

    // 先鋪底（earth / fields / greens），再覆蓋（buildings / water / roads）
    for feat in features {
        let id = feat.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let ftype = feat.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if (id == "earth"
            || id == "fields"
            || id == "greens"
            || id == "water"
            || id == "buildings"
            || id == "prisms"
            || id == "squares")
            && (ftype == "Polygon" || ftype == "MultiPolygon")
        {
            let terrain = match id {
                "water" => Terrain::Water,
                "buildings" | "prisms" | "squares" => Terrain::Farmhouse,
                "greens" => Terrain::Forest,
                "fields" => Terrain::FarmField,
                _ => Terrain::Grassland,
            };
            if ftype == "Polygon" {
                if let Some(rings) = feat.get("coordinates").and_then(|v| v.as_array()) {
                    if let Some(outer) = rings.first().and_then(|v| v.as_array()) {
                        let ring: Vec<(f64, f64)> = outer.iter().filter_map(parse_point).collect();
                        fill_polygon_to_cells(&ring, &mut map, terrain, id);
                    }
                }
            } else if let Some(polys) = feat.get("coordinates").and_then(|v| v.as_array()) {
                for poly in polys {
                    if let Some(rings) = poly.as_array() {
                        if let Some(outer) = rings.first().and_then(|v| v.as_array()) {
                            let ring: Vec<(f64, f64)> = outer.iter().filter_map(parse_point).collect();
                            fill_polygon_to_cells(&ring, &mut map, terrain, id);
                        }
                    }
                }
            }
        }
    }

    for feat in features {
        let id = feat.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let ftype = feat.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if id == "roads" && ftype == "GeometryCollection" {
            if let Some(gs) = feat.get("geometries").and_then(|v| v.as_array()) {
                let mut candidates: Vec<(f64, Vec<(f64, f64)>)> = Vec::new();
                for g in gs {
                    let gt = g.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if gt == "LineString" {
                        if let Some(coords) = g.get("coordinates").and_then(|v| v.as_array()) {
                            let pts: Vec<(f64, f64)> = coords.iter().filter_map(parse_point).collect();
                            if pts.len() < 2 {
                                continue;
                            }
                            let mut total_len = 0.0_f64;
                            for w in pts.windows(2) {
                                let (x1, y1) = w[0];
                                let (x2, y2) = w[1];
                                let dx = x2 - x1;
                                let dy = y2 - y1;
                                total_len += (dx * dx + dy * dy).sqrt();
                            }
                            candidates.push((total_len, pts));
                        }
                    }
                }

                // MFCG roads 含大量支線：僅保留最長的一小批主幹道
                candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                let keep_n = ((candidates.len() as f64) * 0.12).ceil() as usize;
                let keep_n = keep_n.clamp(6, 28);

                for (idx, (total_len, pts)) in candidates.into_iter().enumerate() {
                    if idx >= keep_n {
                        break;
                    }
                    if total_len < IMPORT_HEX_R * 10.0 {
                        continue;
                    }
                    for w in pts.windows(2) {
                        let (x1, y1) = w[0];
                        let (x2, y2) = w[1];
                        let dx = x2 - x1;
                        let dy = y2 - y1;
                        let dist = (dx * dx + dy * dy).sqrt();
                        let steps = ((dist / (IMPORT_HEX_R * 1.1)).ceil() as i32).max(1);
                        for i in 0..=steps {
                            let t = i as f64 / steps as f64;
                            let x = x1 + dx * t;
                            let y = y1 + dy * t;
                            let (aq, ar) = pixel_to_axial_pair(x, y);
                            let (q, r) = axial_round_pair(aq, ar);
                            let k = (q, r);
                            let blocked = matches!(
                                map.get(&k).map(|v| v.0),
                                Some(Terrain::Wall | Terrain::Water | Terrain::Mountain)
                            );
                            if blocked {
                                continue;
                            }
                            map.insert(k, (Terrain::Road, "roads".to_string()));
                        }
                    }
                }
            }
        }
        if id == "planks" && ftype == "GeometryCollection" {
            if let Some(gs) = feat.get("geometries").and_then(|v| v.as_array()) {
                for g in gs {
                    let gt = g.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if gt == "LineString" {
                        if let Some(coords) = g.get("coordinates").and_then(|v| v.as_array()) {
                            let pts: Vec<(f64, f64)> = coords.iter().filter_map(parse_point).collect();
                            sample_line_to_cells(&pts, &mut map, Terrain::Bridge, "planks");
                        }
                    }
                }
            }
        }
        if id == "walls" && ftype == "GeometryCollection" {
            if let Some(gs) = feat.get("geometries").and_then(|v| v.as_array()) {
                for g in gs {
                    let gt = g.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if gt == "Polygon" {
                        if let Some(rings) = g.get("coordinates").and_then(|v| v.as_array()) {
                            if let Some(outer) = rings.first().and_then(|v| v.as_array()) {
                                let ring: Vec<(f64, f64)> = outer.iter().filter_map(parse_point).collect();
                                // 牆只取邊界，不填滿內部，避免整片區域變成 wall
                                if ring.len() >= 2 {
                                    let mut border = ring.clone();
                                    if let (Some(first), Some(last)) = (border.first().copied(), border.last().copied()) {
                                        if first != last {
                                            border.push(first);
                                        }
                                    }
                                    sample_line_to_cells(&border, &mut map, Terrain::Wall, "walls");
                                }
                            }
                        }
                    } else if gt == "LineString" {
                        if let Some(coords) = g.get("coordinates").and_then(|v| v.as_array()) {
                            let pts: Vec<(f64, f64)> = coords.iter().filter_map(parse_point).collect();
                            sample_line_to_cells(&pts, &mut map, Terrain::Wall, "walls");
                        }
                    }
                }
            }
        }
        if id == "trees" && ftype == "MultiPoint" {
            if let Some(coords) = feat.get("coordinates").and_then(|v| v.as_array()) {
                for p in coords {
                    if let Some((x, y)) = parse_point(p) {
                        let (aq, ar) = pixel_to_axial_pair(x, y);
                        let (q, r) = axial_round_pair(aq, ar);
                        map.insert((q, r), (Terrain::Forest, "trees".to_string()));
                    }
                }
            }
        }
    }

    map.into_iter()
        .map(|((q, r), (terrain, tag))| (q, r, terrain, tag))
        .collect()
}

fn terrain_from_watabou(raw: &str) -> Terrain {
    let k = raw.trim().to_ascii_lowercase();
    match k.as_str() {
        "road" => Terrain::Road,
        "bridge" | "plank" | "planks" => Terrain::Bridge,
        "wall" | "walls" => Terrain::Wall,
        "forest" | "forest-light" | "forest_light" | "forest-heavy" | "forest_heavy" => Terrain::Forest,
        "mountain" => Terrain::Mountain,
        "hills" | "hill" => Terrain::Hills,
        "water" | "water-deep" | "water_deep" => Terrain::Water,
        "desert" => Terrain::Desert,
        "swamp" => Terrain::Swamp,
        "tundra" => Terrain::Tundra,
        "jungle" => Terrain::Jungle,
        "farm_field" | "farmland" | "field" | "fields" | "farm" => Terrain::FarmField,
        "farmhouse" | "farm_house" | "barn" => Terrain::Farmhouse,
        "inn" => Terrain::Inn,
        "tavern" | "pub" => Terrain::Tavern,
        "blacksmith" | "smithy" | "forge" => Terrain::Blacksmith,
        "general_store" | "store" | "shop" => Terrain::GeneralStore,
        "clinic" | "hospital" | "healer" => Terrain::Clinic,
        "workshop" => Terrain::Workshop,
        "market" | "bazaar" => Terrain::Market,
        "guild_hall" | "guildhall" | "guild" => Terrain::GuildHall,
        "temple" | "shrine" => Terrain::Temple,
        "academy" | "school" => Terrain::Academy,
        "library" | "archive" => Terrain::Library,
        "barracks" => Terrain::Barracks,
        "guard_post" | "guardpost" | "guardhouse" | "watch" | "watchtower" => Terrain::GuardPost,
        "warehouse" | "depot" => Terrain::Warehouse,
        "granary" | "silo" => Terrain::Granary,
        "dock" | "port" | "harbor" | "harbour" => Terrain::Dock,
        "bathhouse" | "bath" | "spa" => Terrain::Bathhouse,
        "courthouse" | "court" => Terrain::Courthouse,
        "jail" | "prison" => Terrain::Jail,
        "town_hall" | "townhall" | "city_hall" | "cityhall" => Terrain::TownHall,
        "bank" => Terrain::Bank,
        "mint" => Terrain::Mint,
        "stables" | "stable" => Terrain::Stables,
        "caravanserai" | "caravan" | "inn_caravan" => Terrain::Caravanserai,
        "theater" | "theatre" | "opera" => Terrain::Theater,
        "arena" | "colosseum" => Terrain::Arena,
        "observatory" => Terrain::Observatory,
        "alchemist" | "alchemy" | "apothecary" => Terrain::Alchemist,
        "mage_tower" | "magetower" | "wizard_tower" | "arcane_tower" => Terrain::MageTower,
        "embassy" | "consulate" => Terrain::Embassy,
        "prison_yard" | "prisonyard" | "gaol_yard" => Terrain::PrisonYard,
        "urban" | "city" | "town" | "village" => Terrain::Grassland,
        "grassland" | "plain" | "plains" => Terrain::Grassland,
        _ => Terrain::Grassland,
    }
}

fn extract_watabou_cells(parsed: &serde_json::Value) -> Vec<(i32, i32, Terrain, String)> {
    let mut out = Vec::<(i32, i32, Terrain, String)>::new();

    if let Some(hexes_obj) = parsed.get("hexes").and_then(|v| v.as_object()) {
        for (key, v) in hexes_obj {
            let q = v.get("q").and_then(|n| n.as_i64()).map(|n| n as i32);
            let r = v.get("r").and_then(|n| n.as_i64()).map(|n| n as i32);
            let (q, r) = match (q, r) {
                (Some(q), Some(r)) => (q, r),
                _ => match parse_qr_from_key(key) {
                    Some(p) => p,
                    None => continue,
                },
            };
            let terrain_raw = v.get("terrain").and_then(|t| t.as_str()).unwrap_or("grassland");
            out.push((q, r, terrain_from_watabou(terrain_raw), terrain_raw.to_string()));
        }
        return out;
    }

    if let Some(hexes_arr) = parsed.get("hexes").and_then(|v| v.as_array()) {
        for v in hexes_arr {
            let q = v.get("q").and_then(|n| n.as_i64()).map(|n| n as i32);
            let r = v.get("r").and_then(|n| n.as_i64()).map(|n| n as i32);
            let (Some(q), Some(r)) = (q, r) else { continue };
            let terrain_raw = v.get("terrain").and_then(|t| t.as_str()).unwrap_or("grassland");
            out.push((q, r, terrain_from_watabou(terrain_raw), terrain_raw.to_string()));
        }
        return out;
    }

    if let Some(cells_arr) = parsed.get("cells").and_then(|v| v.as_array()) {
        for v in cells_arr {
            let q = v.get("q").and_then(|n| n.as_i64()).map(|n| n as i32)
                .or_else(|| v.get("col").and_then(|n| n.as_i64()).map(|n| n as i32));
            let r = v.get("r").and_then(|n| n.as_i64()).map(|n| n as i32)
                .or_else(|| v.get("row").and_then(|n| n.as_i64()).map(|n| n as i32));
            let (Some(q), Some(r)) = (q, r) else { continue };
            let terrain_raw = v.get("terrain").and_then(|t| t.as_str()).unwrap_or("grassland");
            out.push((q, r, terrain_from_watabou(terrain_raw), terrain_raw.to_string()));
        }
    }

    out
}

fn watabou_offset_q_to_axial(q: i32, r: i32, layout: &str) -> (i32, i32) {
    match layout {
        // Red Blob Games offset->axial
        "odd-q" => (q, r - (q - (q & 1)) / 2),
        "even-q" => (q, r - (q + (q & 1)) / 2),
        "odd-r" => (q - (r - (r & 1)) / 2, r),
        "even-r" => (q - (r + (r & 1)) / 2, r),
        _ => (q, r),
    }
}

fn component_stats(coords: &[(i32, i32)]) -> (usize, usize) {
    let set: HashSet<(i32, i32)> = coords.iter().copied().collect();
    if set.is_empty() {
        return (0, 0);
    }
    let dirs: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];
    let mut seen = HashSet::<(i32, i32)>::new();
    let mut comp_count = 0usize;
    let mut max_comp = 0usize;
    for &start in &set {
        if seen.contains(&start) {
            continue;
        }
        comp_count += 1;
        let mut q = VecDeque::new();
        q.push_back(start);
        seen.insert(start);
        let mut size = 0usize;
        while let Some((x, y)) = q.pop_front() {
            size += 1;
            for (dx, dy) in dirs {
                let next = (x + dx, y + dy);
                if set.contains(&next) && !seen.contains(&next) {
                    seen.insert(next);
                    q.push_back(next);
                }
            }
        }
        if size > max_comp {
            max_comp = size;
        }
    }
    (comp_count, max_comp)
}

fn normalize_watabou_coords(
    raw: Vec<(i32, i32, Terrain, String)>,
    layout_hint: Option<&str>,
) -> (Vec<(i32, i32, Terrain, String)>, &'static str) {
    let mut candidates = vec!["raw", "even-q", "odd-q", "even-r", "odd-r"];
    if let Some(h) = layout_hint {
        let lower = h.to_ascii_lowercase();
        if let Some(idx) = candidates.iter().position(|c| *c == lower) {
            let c = candidates.remove(idx);
            candidates.insert(0, c);
        }
    }

    let mut best_layout = "raw";
    let mut best_cells: Vec<(i32, i32, Terrain, String)> = Vec::new();
    let mut best_score = (0usize, 0usize, usize::MAX); // unique, max_comp, comp_count(越少越好)

    for layout in candidates {
        let mut dedup = HashMap::<(i32, i32), (Terrain, String)>::new();
        for (q, r, t, s) in &raw {
            let (aq, ar) = if layout == "raw" {
                (*q, *r)
            } else {
                watabou_offset_q_to_axial(*q, *r, layout)
            };
            dedup.insert((aq, ar), (*t, s.clone()));
        }
        let coords: Vec<(i32, i32)> = dedup.keys().copied().collect();
        let (comp_count, max_comp) = component_stats(&coords);
        let score = (dedup.len(), max_comp, usize::MAX - comp_count);
        let best_cmp = (best_score.0, best_score.1, usize::MAX - best_score.2);
        if score > best_cmp {
            best_score = (dedup.len(), max_comp, comp_count);
            best_layout = layout;
            best_cells = dedup
                .into_iter()
                .map(|((q, r), (terrain, terrain_str))| (q, r, terrain, terrain_str))
                .collect();
        }
    }

    (best_cells, best_layout)
}

#[component]
fn App() -> impl IntoView {
    let (cells, set_cells) = signal(Vec::new());
    let (selected, set_selected) = signal::<Option<HexCoord>>(None);
    let (selected_many, set_selected_many) = signal::<Vec<HexCoord>>(Vec::new());
    let (brush_terrain, set_brush_terrain) = signal(Terrain::Grassland);
    let (brush_size, set_brush_size) = signal(1_u8);
    let (tool_mode, set_tool_mode) = signal(ToolMode::Paint);
    let (editor_open, set_editor_open) = signal(false);
    let (mobile_menu_open, set_mobile_menu_open) = signal(false);
    let (status, set_status) = signal("載入中⋯".to_string());
    let (world_seed_str, set_world_seed_str) = signal(String::new());
    let (watabou_anchor, set_watabou_anchor) = signal::<Option<HexCoord>>(None);
    let watabou_input_ref = NodeRef::<leptos::html::Input>::new();

    let cells_signal = Signal::from(cells);
    let cell_count = Signal::derive(move || cells.get().len());

    let reload_grid = {
        let set_cells = set_cells;
        let set_status = set_status;
        move || {
            let set_cells = set_cells;
            let set_status = set_status;
            leptos::task::spawn_local(async move {
                match api::load_grid().await {
                    Ok(grid) => {
                        set_world_seed_str.set(grid.world_seed.to_string());
                        set_status.set(format!(
                            "已載入 {} 格 · world_seed={}",
                            grid.cells.len(),
                            grid.world_seed
                        ));
                        set_cells.set(grid.cells);
                    }
                    Err(e) => set_status.set(format!("載入失敗：{e}")),
                }
            });
        }
    };

    // 首次載入
    {
        let reload = reload_grid;
        Effect::new(move || {
            reload();
        });
    }

    let on_reload = {
        let reload = reload_grid;
        Callback::new(move |()| reload())
    };

    let on_save = {
        Callback::new(move |()| {
            set_status.set("已儲存到磁碟".into());
        })
    };

    let on_cell_saved = {
        let reload = reload_grid;
        Callback::new(move |()| reload())
    };

    let on_cell_deleted = {
        let reload = reload_grid;
        Callback::new(move |_coord: HexCoord| {
            set_selected.set(None);
            reload();
        })
    };

    let on_paint = {
        let set_cells = set_cells;
        let set_status = set_status;
        let set_selected = set_selected;
        Callback::new(move |coord: HexCoord| {
            let terrain = brush_terrain.get_untracked();
            set_selected.set(Some(coord));

            let mut should_put = true;
            let mut req_name = String::new();
            let mut req_zone = String::new();
            let mut req_tags = Vec::new();
            let mut req_desc = String::new();

            set_cells.update(|cs| {
                if let Some(cell) = cs.iter_mut().find(|c| c.coord == coord) {
                    if cell.terrain == terrain {
                        should_put = false;
                        req_name = cell.name.clone();
                        req_zone = cell.zone.clone();
                        req_tags = cell.tags.clone();
                        req_desc = cell.description.clone();
                    } else {
                        cell.terrain = terrain;
                        req_name = cell.name.clone();
                        req_zone = cell.zone.clone();
                        req_tags = cell.tags.clone();
                        req_desc = cell.description.clone();
                    }
                } else {
                    cs.push(HexCell {
                        coord,
                        terrain,
                        name: req_name.clone(),
                        zone: String::new(),
                        tags: Vec::new(),
                        description: String::new(),
                        objects: Vec::new(),
                    });
                }
            });

            if !should_put {
                return;
            }

            let set_status = set_status;
            leptos::task::spawn_local(async move {
                let req = api::CellPutReq {
                    q: coord.q,
                    r: coord.r,
                    terrain,
                    name: req_name,
                    zone: req_zone,
                    tags: req_tags,
                    description: req_desc,
                };
                if let Err(e) = api::put_cell(&req).await {
                    set_status.set(format!("新增失敗：{e}"));
                }
            });
        })
    };

    let on_erase = {
        let set_cells = set_cells;
        let set_status = set_status;
        let set_selected = set_selected;
        Callback::new(move |coord: HexCoord| {
            set_selected.set(Some(coord));
            let mut existed = false;
            set_cells.update(|cs| {
                let before = cs.len();
                cs.retain(|c| c.coord != coord);
                existed = cs.len() != before;
            });
            if !existed {
                return;
            }
            let set_status = set_status;
            leptos::task::spawn_local(async move {
                if let Err(e) = api::delete_cell(coord.q, coord.r).await {
                    set_status.set(format!("刪除失敗：{e}"));
                }
            });
        })
    };

    let on_select_toggle = {
        let set_selected = set_selected;
        let set_selected_many = set_selected_many;
        Callback::new(move |coord: HexCoord| {
            let mut first_after = None;
            set_selected_many.update(|sel| {
                if let Some(i) = sel.iter().position(|c| *c == coord) {
                    sel.remove(i);
                } else {
                    sel.push(coord);
                }
                first_after = sel.first().copied();
            });
            set_selected.set(first_after);
        })
    };

    let on_select_add = {
        let set_selected = set_selected;
        let set_selected_many = set_selected_many;
        Callback::new(move |coord: HexCoord| {
            set_selected_many.update(|sel| {
                if !sel.contains(&coord) {
                    sel.push(coord);
                }
            });
            set_selected.set(Some(coord));
        })
    };

    let on_clear_selection = {
        let set_selected = set_selected;
        let set_selected_many = set_selected_many;
        Callback::new(move |()| {
            set_selected.set(None);
            set_selected_many.set(Vec::new());
        })
    };

    let on_apply_world_seed = {
        let reload_grid = reload_grid;
        let set_status = set_status;
        Callback::new(move |()| {
            let s = world_seed_str.get_untracked();
            let seed: u64 = match s.trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    set_status.set("種子須為非負整數（u64）".into());
                    return;
                }
            };
            let reload_grid = reload_grid;
            let set_status = set_status;
            leptos::task::spawn_local(async move {
                match api::put_world_seed(seed).await {
                    Ok(()) => {
                        reload_grid();
                        set_status.set(format!("已設定 world_seed={seed}"));
                    }
                    Err(e) => set_status.set(format!("套用種子失敗：{e}")),
                }
            });
        })
    };

    let on_move_selection = {
        let selected_many = selected_many;
        let set_selected_many = set_selected_many;
        let set_selected = set_selected;
        let set_cells = set_cells;
        let set_status = set_status;
        Callback::new(move |(from, to): (HexCoord, HexCoord)| {
            let dq = to.q - from.q;
            let dr = to.r - from.r;
            if dq == 0 && dr == 0 {
                return;
            }

            let selected_list = selected_many.get_untracked();
            if selected_list.is_empty() {
                return;
            }

            let mut moved_cells: Vec<api::CellPutReq> = Vec::new();
            let old_coords = selected_list.clone();
            set_cells.update(|cs| {
                let mut moving = Vec::<HexCell>::new();
                cs.retain(|c| {
                    if old_coords.contains(&c.coord) {
                        moving.push(c.clone());
                        false
                    } else {
                        true
                    }
                });
                for mut c in moving {
                    c.coord = HexCoord { q: c.coord.q + dq, r: c.coord.r + dr };
                    moved_cells.push(api::CellPutReq {
                        q: c.coord.q,
                        r: c.coord.r,
                        terrain: c.terrain,
                        name: c.name.clone(),
                        zone: c.zone.clone(),
                        tags: c.tags.clone(),
                        description: c.description.clone(),
                    });
                    cs.push(c);
                }
            });

            let new_selection: Vec<HexCoord> = old_coords
                .iter()
                .map(|c| HexCoord { q: c.q + dq, r: c.r + dr })
                .collect();
            set_selected_many.set(new_selection.clone());
            set_selected.set(new_selection.first().copied());

            let set_status = set_status;
            leptos::task::spawn_local(async move {
                for c in old_coords {
                    let _ = api::delete_cell(c.q, c.r).await;
                }
                if let Err(e) = api::put_cells(&moved_cells).await {
                    set_status.set(format!("移動寫入失敗：{e}"));
                }
            });
        })
    };

    let on_load_json = {
        let watabou_input_ref = watabou_input_ref;
        Callback::new(move |coord: HexCoord| {
            set_watabou_anchor.set(Some(coord));
            if let Some(input) = watabou_input_ref.get() {
                input.set_value("");
                input.click();
            }
        })
    };

    let on_watabou_change = {
        let set_status = set_status;
        let reload_grid = reload_grid;
        let set_selected = set_selected;
        move |ev: leptos::ev::Event| {
            let Some(anchor) = watabou_anchor.get_untracked() else {
                set_status.set("未設定 Watabou 錨點".into());
                return;
            };
            let Some(target) = ev.target() else {
                return;
            };
            let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() else {
                return;
            };
            let Some(files) = input.files() else {
                return;
            };
            let Some(file) = files.get(0) else {
                return;
            };

            set_status.set("Watabou JSON 匯入中⋯".into());
            let set_status = set_status;
            let reload_grid = reload_grid;
            let set_selected = set_selected;
            leptos::task::spawn_local(async move {
                let text_js = wasm_bindgen_futures::JsFuture::from(file.text()).await;
                let Ok(text_js) = text_js else {
                    set_status.set("讀取檔案失敗".into());
                    return;
                };
                let Some(text) = text_js.as_string() else {
                    set_status.set("檔案內容不是文字 JSON".into());
                    return;
                };

                let parsed: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        set_status.set(format!("JSON 解析失敗：{e}"));
                        return;
                    }
                };
                let raw = extract_watabou_cells(&parsed);
                let mfcg = if raw.is_empty() {
                    extract_mfcg_cells(&parsed)
                } else {
                    Vec::new()
                };

                if raw.is_empty() && mfcg.is_empty() {
                    set_status.set("未找到可匯入格子（支援 Watabou hexes/cells 或 MFCG GeoJSON）".into());
                    return;
                }

                let (converted, picked_layout, source_count) = if !raw.is_empty() {
                    let layout_hint = parsed
                        .get("layout")
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty());
                    let source_count = raw.len();
                    let (converted, picked_layout) = normalize_watabou_coords(raw, layout_hint);
                    (converted, picked_layout.to_string(), source_count)
                } else {
                    let source_count = mfcg.len();
                    (mfcg, "mfcg-geojson".to_string(), source_count)
                };

                let min_q = converted.iter().map(|(q, _, _, _)| *q).min().unwrap_or(0);
                let min_r = converted.iter().map(|(_, r, _, _)| *r).min().unwrap_or(0);
                let batch: Vec<api::CellPutReq> = converted
                    .into_iter()
                    .map(|(q, r, terrain, terrain_str)| {
                        let tq = anchor.q + (q - min_q);
                        let tr = anchor.r + (r - min_r);
                        api::CellPutReq {
                            q: tq,
                            r: tr,
                            terrain,
                            name: String::new(),
                            zone: terrain_str.split('-').next().unwrap_or("wild").to_string(),
                            tags: vec!["watabou".into(), terrain_str],
                            description: String::new(),
                        }
                    })
                    .collect();

                // 分批上傳，避免大檔一次請求過大
                let mut total = 0usize;
                const CHUNK: usize = 2000;
                for part in batch.chunks(CHUNK) {
                    match api::put_cells(part).await {
                        Ok(n) => total += n,
                        Err(e) => {
                            set_status.set(format!("Watabou 匯入失敗：{e}"));
                            return;
                        }
                    }
                }
                reload_grid();
                set_selected.set(Some(anchor));
                set_status.set(format!(
                    "Watabou 匯入完成：{} 格（來源 {}，座標 {}）",
                    total, source_count, picked_layout
                ));
            });
        }
    };

    view! {
        <input
            node_ref=watabou_input_ref
            type="file"
            accept=".json,application/json"
            style="display:none"
            on:change=on_watabou_change
        />
        <Toolbar
            on_reload=on_reload
            on_save=on_save
            on_clear_selection=on_clear_selection
            on_apply_world_seed=on_apply_world_seed
            world_seed_str=world_seed_str
            set_world_seed_str=set_world_seed_str
            selected=selected
            cell_count=cell_count
            brush_terrain=brush_terrain
            set_brush_terrain=set_brush_terrain
            brush_size=brush_size
            set_brush_size=set_brush_size
            tool_mode=tool_mode
            set_tool_mode=set_tool_mode
            editor_open=editor_open
            set_editor_open=set_editor_open
            mobile_menu_open=mobile_menu_open
            set_mobile_menu_open=set_mobile_menu_open
        />
        <div class="main-layout">
            <div
                class="map-area"
                on:pointerdown=move |_| {
                    set_mobile_menu_open.set(false);
                }
            >
                <HexGrid
                    cells=cells_signal
                    selected=selected
                    selected_many=selected_many
                    set_selected=set_selected
                    brush_size=brush_size
                    tool_mode=tool_mode
                    on_paint=on_paint
                    on_erase=on_erase
                    on_select_toggle=on_select_toggle
                    on_select_add=on_select_add
                    on_move_selection=on_move_selection
                    on_load_json=on_load_json
                />
            </div>
            <Show when=move || editor_open.get()>
                <CellEditor
                    cells=cells_signal
                    selected=selected
                    on_saved=on_cell_saved
                    on_deleted=on_cell_deleted
                />
            </Show>
        </div>
        <div class="status-bar">
            {move || status.get()}
            {move || {
                selected.get().map(|c| format!(" | 選取：({},{})", c.q, c.r)).unwrap_or_default()
            }}
            {move || {
                let n = selected_many.get().len();
                if n > 1 { format!(" | 多選：{} 格", n) } else { String::new() }
            }}
        </div>
    }
}
