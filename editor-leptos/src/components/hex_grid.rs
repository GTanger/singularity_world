#![allow(clippy::redundant_locals)]

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::CanvasWindingRule;
use std::collections::{HashMap, HashSet};
use crate::types::{HexCell, HexCoord, Terrain, ToolMode};

const HEX_R: f64 = 28.0;
/// 林區邊界：頂點往鄰接林格格心內縮後，每邊以**二十四次貝茲**（25 控制點）相連；Canvas 無原生高次 API，以 De Casteljau 取樣折線逼近。
const FOREST_BOUNDARY_VERTEX_INSET: f64 = 9.0;
/// 自格心指向該邊中點方向之基準鼓出強度（再乘隨機係數）
const FOREST_ARC_BULGE_ALONG_EDGE: f64 = 10.5;
/// 鼓出量隨機係數（略收斂 → 曲線較連續、少突兀轉折）
const FOREST_BULGE_JITTER_MIN: f64 = 0.58;
const FOREST_BULGE_JITTER_MAX: f64 = 1.22;
/// 沿格心→邊中再加減之額外距離（世界座標 px）
const FOREST_RADIAL_JITTER_PX: f64 = 4.0;
/// 法線方向擺動（垂直於弦 p0→p1）；會再乘包絡並做鄰點平滑
const FOREST_NORMAL_WOBBLE_PX: f64 = 6.0;
/// 二十四次貝茲每邊取樣段數（愈密愈順）
const FOREST_BEZIER24_STEPS: usize = 160;
/// 頂點內縮距離之係數範圍
const FOREST_INSET_SCALE_MIN: f64 = 0.78;
const FOREST_INSET_SCALE_MAX: f64 = 1.14;

/// 水域邊界：頂點內縮須**小於約半徑**，否則細水道／單格水會在格間「斷段」
const WATER_BOUNDARY_VERTEX_INSET: f64 = 7.8;
/// 僅一格連通區：內縮較小，藍色填滿較多，相鄰水格視覺才接得上
const WATER_SINGLE_HEX_VERTEX_INSET: f64 = 5.2;
/// 2～5 格：略收
const WATER_SMALL_COMPONENT_VERTEX_INSET: f64 = 6.6;
const WATER_ARC_BULGE_ALONG_EDGE: f64 = 5.2;
const WATER_BULGE_JITTER_MIN: f64 = 0.72;
const WATER_BULGE_JITTER_MAX: f64 = 1.06;
const WATER_RADIAL_JITTER_PX: f64 = 1.6;
const WATER_NORMAL_WOBBLE_PX: f64 = 3.2;
const WATER_BEZIER24_STEPS: usize = 200;
const WATER_INSET_SCALE_MIN: f64 = 0.96;
const WATER_INSET_SCALE_MAX: f64 = 1.02;
const SQRT3: f64 = 1.7320508075688772;
const HEX_VERT_OFFSETS: [(f64, f64); 6] = [
    (24.24871130596428, 14.0),
    (0.0, 28.0),
    (-24.24871130596428, 14.0),
    (-24.24871130596428, -14.0),
    (0.0, -28.0),
    (24.24871130596428, -14.0),
];

/// 與 `HEX_VERT_OFFSETS` **邊索引 0..5** 對齊的鄰格轴向增量 `(dq, dr)`（邊 `i`＝頂點 `i → (i+1)%6`）。
const AXIAL_DIRS_FOR_EDGES: [(i32, i32); 6] = [
    (0, 1),   // 邊 0：外側為 Se
    (-1, 1),  // 邊 1：Sw
    (-1, 0),  // 邊 2：W
    (0, -1),  // 邊 3：Nw
    (1, -1),  // 邊 4：Ne
    (1, 0),   // 邊 5：E
];

fn is_forest_terrain(t: Terrain) -> bool {
    matches!(t, Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle)
}

fn is_water_terrain(t: Terrain) -> bool {
    matches!(t, Terrain::Water | Terrain::WaterDeep)
}

/// 畫面上顯示用的地名／地形字（與下方文字繪製一致）
fn cell_display_label(cell: &HexCell) -> String {
    if cell.name.trim().is_empty() {
        cell.terrain.label().to_string()
    } else {
        cell.name.clone()
    }
}

/// 標籤合併：**六邊相鄰**（與遊戲可走鄰居一致）且 **同屬性**＝同 `Terrain`；
/// 有自訂 `name` 時須地名也相同才算同一區。
#[derive(Clone, PartialEq, Eq, Hash)]
enum LabelMergeKey {
    /// 無自訂名：僅依地形合併
    TerrainOnly(Terrain),
    /// 有自訂名：同地形且同名才合併
    TerrainAndName { terrain: Terrain, name: String },
}

fn label_merge_key(cell: &HexCell) -> LabelMergeKey {
    let n = cell.name.trim();
    if n.is_empty() {
        LabelMergeKey::TerrainOnly(cell.terrain)
    } else {
        LabelMergeKey::TerrainAndName {
            terrain: cell.terrain,
            name: n.to_string(),
        }
    }
}

/// 與後端 `HexDir::delta` 一致之六鄰（axial）— **僅共用邊** 之相鄰
const AXIAL_NEIGHBOR_DR: [(i32, i32); 6] = [
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, 0),
    (-1, 1),
    (0, 1),
];

/// 連通區內僅**一格**顯示名稱；代表格取與區域幾何中心（螢幕座標平均）最近之六角
fn label_representative_coords(cs: &[HexCell]) -> HashSet<(i32, i32)> {
    let mut at: HashMap<(i32, i32), &HexCell> = HashMap::with_capacity(cs.len());
    for c in cs {
        at.insert((c.coord.q, c.coord.r), c);
    }
    let mut visited: HashSet<(i32, i32)> = HashSet::with_capacity(cs.len());
    let mut reps: HashSet<(i32, i32)> = HashSet::new();

    for c in cs {
        let start = (c.coord.q, c.coord.r);
        if visited.contains(&start) {
            continue;
        }
        let seed = at.get(&start).expect("cell in list");
        let key = label_merge_key(seed);
        let mut stack = vec![start];
        let mut comp: Vec<(i32, i32)> = Vec::new();
        visited.insert(start);

        while let Some(p) = stack.pop() {
            comp.push(p);
            for &(dq, dr) in &AXIAL_NEIGHBOR_DR {
                let nq = p.0 + dq;
                let nr = p.1 + dr;
                let nid = (nq, nr);
                if visited.contains(&nid) {
                    continue;
                }
                let Some(nc) = at.get(&nid) else {
                    continue;
                };
                if label_merge_key(nc) != key {
                    continue;
                }
                visited.insert(nid);
                stack.push(nid);
            }
        }
        let rep = label_rep_coord_nearest_centroid(&comp);
        reps.insert(rep);
    }
    reps
}

/// 林緣外側邊描邊色（略深於對應地形填色）
fn forest_boundary_stroke_color(t: Terrain) -> &'static str {
    match t {
        Terrain::ForestHeavy => "#1a3d12",
        Terrain::Forest => "#3d6530",
        Terrain::ForestLight => "#5f8a4a",
        Terrain::Jungle => "#1f4d1a",
        _ => "#3d6530",
    }
}

/// 弧線封閉區域內填色（深綠，略深於格面以區分林緣內側）
fn forest_interior_fill_color(t: Terrain) -> &'static str {
    match t {
        Terrain::ForestHeavy => "#14280e",
        Terrain::Forest => "#1f3d18",
        Terrain::ForestLight => "#2d4f24",
        Terrain::Jungle => "#183814",
        _ => "#1f3d18",
    }
}

/// 水岸描邊（略亮於填色，河流感）
fn water_boundary_stroke_color(t: Terrain) -> &'static str {
    match t {
        Terrain::WaterDeep => "#7ec8ff",
        Terrain::Water => "#a8dcff",
        _ => "#7ec8ff",
    }
}

/// 水域弧線內側填色（藍系，與格面草原色區隔）
fn water_interior_fill_color(t: Terrain) -> &'static str {
    match t {
        Terrain::WaterDeep => "#1e5080",
        Terrain::Water => "#3d7cb8",
        _ => "#3d7cb8",
    }
}

/// 林緣邊界有向邊：林格 `(q,r)` 上邊 `ei`（鄰格不在**同一連通林區**）
type ForestBoundaryEdge = (i32, i32, usize);
/// 單一邊界迴線：地形 + 邊序列 + 內縮頂點表
type ForestLoopPiece = (
    Terrain,
    Vec<ForestBoundaryEdge>,
    HashMap<(i64, i64), (f64, f64)>,
);
/// 迴線頂點聚合：量化鍵 →（世界座標、鄰接林格集合）
type ForestVertexAgg = HashMap<(i64, i64), ((f64, f64), HashSet<(i32, i32)>)>;

fn forest_vertex_world(q: i32, r: i32, vi: usize) -> (f64, f64) {
    let (px, py) = coord_to_pixel(q, r);
    let (vx, vy) = HEX_VERT_OFFSETS[vi];
    (px + vx, py + vy)
}

fn forest_vertex_key_xy(x: f64, y: f64) -> (i64, i64) {
    ((x * 10_000.0).round() as i64, (y * 10_000.0).round() as i64)
}

fn forest_edge_vertex_keys(q: i32, r: i32, ei: usize) -> ((i64, i64), (i64, i64)) {
    let (wa, wb) = (forest_vertex_world(q, r, ei), forest_vertex_world(q, r, (ei + 1) % 6));
    (forest_vertex_key_xy(wa.0, wa.1), forest_vertex_key_xy(wb.0, wb.1))
}

fn forest_inset_toward(px: f64, py: f64, gx: f64, gy: f64, dist: f64) -> (f64, f64) {
    let dx = gx - px;
    let dy = gy - py;
    let len = (dx * dx + dy * dy).sqrt().max(1e-9);
    (px + (dx / len) * dist, py + (dy / len) * dist)
}

/// [-1, 1]，與 `forest_stable_rand01` 同鍵空間
fn forest_stable_rand_signed(q: i32, r: i32, ei: usize, salt: u32) -> f64 {
    forest_stable_rand01(q, r, ei, salt).mul_add(2.0, -1.0)
}

/// [0, 1) 穩定偽隨機，同一 `(q,r,ei,salt)` 永遠相同（平移／縮放畫面不重算種子）
fn forest_stable_rand01(q: i32, r: i32, ei: usize, salt: u32) -> f64 {
    let mut x = (q as u32)
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add((r as u32).rotate_left(7))
        .wrapping_add((ei as u32).wrapping_mul(0x85EB_CA6B));
    x ^= salt.wrapping_mul(0xC2B2_AE3D);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    ((x >> 8) as f64) / (1u64 << 24) as f64
}

/// 量化頂點鍵 → [0,1)（用於內縮距離微調）
fn forest_vertex_rand01(k: (i64, i64), salt: u32) -> f64 {
    let a = k.0 as u64;
    let b = k.1 as u64;
    let mut x = a
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ (salt as u64).wrapping_mul(0xA5A5_A5A5_A5A5_A5A5);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    let hi = (x >> 32) as u32;
    let lo = x as u32;
    let y = hi ^ lo.rotate_left(13);
    ((y >> 8) as f64) / (1u64 << 24) as f64
}

#[inline]
fn lerp_f64(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// 二十四次貝茲（25 控制點）在參數 `t∈[0,1]` 之值（De Casteljau）
fn bezier24_eval(p: [(f64, f64); 25], t: f64) -> (f64, f64) {
    let mut v = p;
    for r in 1..25 {
        for i in 0..(25 - r) {
            v[i] = (
                v[i].0 * (1.0 - t) + v[i + 1].0 * t,
                v[i].1 * (1.0 - t) + v[i + 1].1 * t,
            );
        }
    }
    v[0]
}

/// 一維 23 點（內部控制點索引）平滑：多輪柔核，壓掉微尖角／控制多邊形急折
fn smooth_forest_offset_23(a: &mut [f64; 23]) {
    for _ in 0..4 {
        let prev = *a;
        for i in 1..22 {
            a[i] = 0.22 * prev[i - 1] + 0.56 * prev[i] + 0.22 * prev[i + 1];
        }
        a[0] = 0.5 * prev[0] + 0.5 * prev[1];
        a[22] = 0.5 * prev[21] + 0.5 * prev[22];
    }
}

/// 單邊林緣：二十四次貝茲控制點（端點＝內縮頂點）
fn forest_edge_bezier24_controls(
    p0: (f64, f64),
    p1: (f64, f64),
    q: i32,
    r: i32,
    ei: usize,
    ux: f64,
    uy: f64,
) -> [(f64, f64); 25] {
    let sx = p1.0 - p0.0;
    let sy = p1.1 - p0.1;
    let slen = (sx * sx + sy * sy).sqrt().max(1e-9);
    let tx = sx / slen;
    let ty = sy / slen;
    let nx = -ty;
    let ny = tx;

    let bulge = FOREST_ARC_BULGE_ALONG_EDGE
        * lerp_f64(
            FOREST_BULGE_JITTER_MIN,
            FOREST_BULGE_JITTER_MAX,
            forest_stable_rand01(q, r, ei, 0xB0D9),
        );
    let radial_extra = FOREST_RADIAL_JITTER_PX * forest_stable_rand_signed(q, r, ei, 0xE11A);
    let outward = bulge + radial_extra;
    let radial_blend = 0.55_f64;

    let mut wn = [0.0_f64; 23];
    let mut wt = [0.0_f64; 23];
    for (idx, k) in (1..24).enumerate() {
        let s = k as f64 / 24.0;
        let bell = (std::f64::consts::PI * s).sin();
        // sin⁴：邊緣極柔，主波動留在中段，整體較滑
        let b2 = bell * bell;
        let bell_soft = b2 * b2;
        wn[idx] = FOREST_NORMAL_WOBBLE_PX * forest_stable_rand_signed(q, r, ei, 0xA100 + k as u32) * bell_soft;
        wt[idx] = FOREST_NORMAL_WOBBLE_PX
            * 0.32
            * forest_stable_rand_signed(q, r, ei, 0xC200 + k as u32)
            * bell_soft;
    }
    smooth_forest_offset_23(&mut wn);
    smooth_forest_offset_23(&mut wt);

    let mut pts = [(0.0_f64, 0.0_f64); 25];
    pts[0] = p0;
    pts[24] = p1;
    for (idx, k) in (1..24).enumerate() {
        let s = k as f64 / 24.0;
        let bx = p0.0 + tx * slen * s;
        let by = p0.1 + ty * slen * s;
        let bell = (std::f64::consts::PI * s).sin();
        let b2 = bell * bell;
        let bell_soft = b2 * b2;
        let wni = wn[idx];
        let wti = wt[idx];
        pts[idx + 1] = (
            bx + nx * wni + tx * wti + ux * outward * radial_blend * bell_soft,
            by + ny * wni + ty * wti + uy * outward * radial_blend * bell_soft,
        );
    }
    pts
}

/// 水域專用：多一輪平滑，河流線更連續
fn smooth_water_offset_23(a: &mut [f64; 23]) {
    for _ in 0..5 {
        let prev = *a;
        for i in 1..22 {
            a[i] = 0.24 * prev[i - 1] + 0.52 * prev[i] + 0.24 * prev[i + 1];
        }
        a[0] = 0.5 * prev[0] + 0.5 * prev[1];
        a[22] = 0.5 * prev[21] + 0.5 * prev[22];
    }
}

/// 單邊水岸：二十四次貝茲；鼓出較小、波動柔，貼近格心、流暢感
fn water_edge_bezier24_controls(
    p0: (f64, f64),
    p1: (f64, f64),
    q: i32,
    r: i32,
    ei: usize,
    ux: f64,
    uy: f64,
) -> [(f64, f64); 25] {
    let sx = p1.0 - p0.0;
    let sy = p1.1 - p0.1;
    let slen = (sx * sx + sy * sy).sqrt().max(1e-9);
    let tx = sx / slen;
    let ty = sy / slen;
    let nx = -ty;
    let ny = tx;

    let bulge = WATER_ARC_BULGE_ALONG_EDGE
        * lerp_f64(
            WATER_BULGE_JITTER_MIN,
            WATER_BULGE_JITTER_MAX,
            forest_stable_rand01(q, r, ei, 0xD1CE),
        );
    let radial_extra = WATER_RADIAL_JITTER_PX * forest_stable_rand_signed(q, r, ei, 0xE11B);
    let outward = bulge + radial_extra;
    let radial_blend = 0.38_f64;

    let mut wn = [0.0_f64; 23];
    let mut wt = [0.0_f64; 23];
    for (idx, k) in (1..24).enumerate() {
        let s = k as f64 / 24.0;
        let bell = (std::f64::consts::PI * s).sin();
        let b2 = bell * bell;
        // sin⁴：與林緣同級柔度，避免過度內縮造成細水道斷裂
        let bell_soft = b2 * b2;
        wn[idx] = WATER_NORMAL_WOBBLE_PX * forest_stable_rand_signed(q, r, ei, 0xB100 + k as u32) * bell_soft;
        wt[idx] = WATER_NORMAL_WOBBLE_PX
            * 0.22
            * forest_stable_rand_signed(q, r, ei, 0xD200 + k as u32)
            * bell_soft;
    }
    smooth_water_offset_23(&mut wn);
    smooth_water_offset_23(&mut wt);

    let mut pts = [(0.0_f64, 0.0_f64); 25];
    pts[0] = p0;
    pts[24] = p1;
    for (idx, k) in (1..24).enumerate() {
        let s = k as f64 / 24.0;
        let bx = p0.0 + tx * slen * s;
        let by = p0.1 + ty * slen * s;
        let bell = (std::f64::consts::PI * s).sin();
        let b2 = bell * bell;
        let bell_soft = b2 * b2;
        let wni = wn[idx];
        let wti = wt[idx];
        pts[idx + 1] = (
            bx + nx * wni + tx * wti + ux * outward * radial_blend * bell_soft,
            by + ny * wni + ty * wti + uy * outward * radial_blend * bell_soft,
        );
    }
    pts
}

/// 連通林區（六鄰、同屬 `forest_set`）
fn forest_connected_components(forest_set: &HashSet<(i32, i32)>) -> Vec<Vec<(i32, i32)>> {
    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    let mut out: Vec<Vec<(i32, i32)>> = Vec::new();
    for &seed in forest_set {
        if visited.contains(&seed) {
            continue;
        }
        let mut stack = vec![seed];
        visited.insert(seed);
        let mut comp = Vec::new();
        while let Some((cq, cr)) = stack.pop() {
            comp.push((cq, cr));
            for &(dq, dr) in &AXIAL_NEIGHBOR_DR {
                let nq = cq + dq;
                let nr = cr + dr;
                let nid = (nq, nr);
                if !forest_set.contains(&nid) || visited.contains(&nid) {
                    continue;
                }
                visited.insert(nid);
                stack.push(nid);
            }
        }
        out.push(comp);
    }
    out
}

/// 與連通林區相鄰但**不在該區**之邊（含外輪廓與包圍破孔非林格之內輪廓）
fn component_boundary_edges(comp: &[(i32, i32)]) -> Vec<ForestBoundaryEdge> {
    let cset: HashSet<(i32, i32)> = comp.iter().copied().collect();
    let mut edges = Vec::new();
    for &(q, r) in comp {
        for (ei, &(dq, dr)) in AXIAL_DIRS_FOR_EDGES.iter().enumerate() {
            let nq = q + dq;
            let nr = r + dr;
            if !cset.contains(&(nq, nr)) {
                edges.push((q, r, ei));
            }
        }
    }
    edges
}

fn forest_other_vertex_key(e: ForestBoundaryEdge, v: (i64, i64)) -> (i64, i64) {
    let (a, b) = forest_edge_vertex_keys(e.0, e.1, e.2);
    if a == v {
        b
    } else {
        a
    }
}

/// 邊界邊集拆成封閉迴線（每迴線一筆封閉路徑）
fn forest_trace_boundary_cycles(edges: &[ForestBoundaryEdge]) -> Vec<Vec<ForestBoundaryEdge>> {
    if edges.is_empty() {
        return vec![];
    }
    let mut unused: HashSet<ForestBoundaryEdge> = edges.iter().copied().collect();
    let mut incident: HashMap<(i64, i64), Vec<ForestBoundaryEdge>> = HashMap::new();
    for &e in edges {
        let (a, b) = forest_edge_vertex_keys(e.0, e.1, e.2);
        incident.entry(a).or_default().push(e);
        incident.entry(b).or_default().push(e);
    }
    let mut cycles: Vec<Vec<ForestBoundaryEdge>> = Vec::new();
    while let Some(&start_e) = unused.iter().next() {
        let (va, _) = forest_edge_vertex_keys(start_e.0, start_e.1, start_e.2);
        let mut cur_e = start_e;
        let mut cur_v = va;
        let start_v = va;
        let mut cycle: Vec<ForestBoundaryEdge> = Vec::new();
        loop {
            if !unused.contains(&cur_e) {
                for &e in &cycle {
                    unused.remove(&e);
                }
                break;
            }
            cycle.push(cur_e);
            let other = forest_other_vertex_key(cur_e, cur_v);
            if other == start_v && cycle.len() > 1 {
                for &e in &cycle {
                    unused.remove(&e);
                }
                cycles.push(cycle);
                break;
            }
            let next_e = incident
                .get(&other)
                .and_then(|lst| lst.iter().copied().find(|&e| e != cur_e && unused.contains(&e)));
            match next_e {
                Some(ne) => {
                    cur_v = other;
                    cur_e = ne;
                }
                None => {
                    for &e in &cycle {
                        unused.remove(&e);
                    }
                    break;
                }
            }
        }
    }
    cycles
}

/// 單一迴線：頂點內縮後之量化鍵 → 座標
fn boundary_loop_inset_map(
    cycle: &[ForestBoundaryEdge],
    vertex_inset: f64,
    inset_scale_min: f64,
    inset_scale_max: f64,
    rand_salt: u32,
) -> HashMap<(i64, i64), (f64, f64)> {
    let mut vertex_agg: ForestVertexAgg = HashMap::new();
    for &e in cycle {
        let (q, r, ei) = e;
        for vi in [ei, (ei + 1) % 6] {
            let (wx, wy) = forest_vertex_world(q, r, vi);
            let k = forest_vertex_key_xy(wx, wy);
            let ent = vertex_agg.entry(k).or_insert_with(|| ((wx, wy), HashSet::new()));
            ent.1.insert((q, r));
        }
    }
    let mut inset_map: HashMap<(i64, i64), (f64, f64)> =
        HashMap::with_capacity(vertex_agg.len());
    for (k, ((wx, wy), cells)) in vertex_agg {
        let mut sx = 0.0_f64;
        let mut sy = 0.0_f64;
        let n = cells.len() as f64;
        for (q, r) in cells {
            let (px, py) = coord_to_pixel(q, r);
            sx += px;
            sy += py;
        }
        let gx = sx / n.max(1.0);
        let gy = sy / n.max(1.0);
        let inset_scale = lerp_f64(
            inset_scale_min,
            inset_scale_max,
            forest_vertex_rand01(k, rand_salt),
        );
        inset_map.insert(
            k,
            forest_inset_toward(wx, wy, gx, gy, vertex_inset * inset_scale),
        );
    }
    inset_map
}

fn forest_loop_inset_map(cycle: &[ForestBoundaryEdge]) -> HashMap<(i64, i64), (f64, f64)> {
    boundary_loop_inset_map(
        cycle,
        FOREST_BOUNDARY_VERTEX_INSET,
        FOREST_INSET_SCALE_MIN,
        FOREST_INSET_SCALE_MAX,
        0x51EE,
    )
}

fn water_vertex_inset_for_component(comp_len: usize) -> f64 {
    if comp_len <= 1 {
        WATER_SINGLE_HEX_VERTEX_INSET
    } else if comp_len <= 5 {
        WATER_SMALL_COMPONENT_VERTEX_INSET
    } else {
        WATER_BOUNDARY_VERTEX_INSET
    }
}

fn water_loop_inset_map(
    cycle: &[ForestBoundaryEdge],
    comp_len: usize,
) -> HashMap<(i64, i64), (f64, f64)> {
    boundary_loop_inset_map(
        cycle,
        water_vertex_inset_for_component(comp_len),
        WATER_INSET_SCALE_MIN,
        WATER_INSET_SCALE_MAX,
        0xA7E1,
    )
}

/// 將**單一**迴線弧段接到目前 path（不重複 `begin_path`）
fn append_forest_loop_path(
    ctx: &web_sys::CanvasRenderingContext2d,
    cycle: &[ForestBoundaryEdge],
    inset_map: &HashMap<(i64, i64), (f64, f64)>,
) {
    if cycle.is_empty() {
        return;
    }
    let (va, _) = forest_edge_vertex_keys(cycle[0].0, cycle[0].1, cycle[0].2);
    let mut cur_v = va;
    for (i, &e) in cycle.iter().enumerate() {
        let (q, r, ei) = e;
        let other = forest_other_vertex_key(e, cur_v);
        let p0 = inset_map[&cur_v];
        let p1 = inset_map[&other];
        let (cx, cy) = coord_to_pixel(q, r);
        let (ax, ay) = HEX_VERT_OFFSETS[ei];
        let (bx, by) = HEX_VERT_OFFSETS[(ei + 1) % 6];
        let lmx = (ax + bx) * 0.5;
        let lmy = (ay + by) * 0.5;
        let ex = cx + lmx;
        let ey = cy + lmy;
        let rdx = ex - cx;
        let rdy = ey - cy;
        let rlen = (rdx * rdx + rdy * rdy).sqrt().max(1e-9);
        let ux = rdx / rlen;
        let uy = rdy / rlen;

        let cps = forest_edge_bezier24_controls(p0, p1, q, r, ei, ux, uy);
        let steps = FOREST_BEZIER24_STEPS.max(32);

        if i == 0 {
            ctx.move_to(p0.0, p0.1);
        }
        for j in 1..=steps {
            let t = j as f64 / steps as f64;
            let pt = bezier24_eval(cps, t);
            ctx.line_to(pt.0, pt.1);
        }
        cur_v = other;
    }
    ctx.close_path();
}

/// 單一水岸迴線（與林緣同幾何，參數較靠格心、較柔）
fn append_water_loop_path(
    ctx: &web_sys::CanvasRenderingContext2d,
    cycle: &[ForestBoundaryEdge],
    inset_map: &HashMap<(i64, i64), (f64, f64)>,
) {
    if cycle.is_empty() {
        return;
    }
    let (va, _) = forest_edge_vertex_keys(cycle[0].0, cycle[0].1, cycle[0].2);
    let mut cur_v = va;
    for (i, &e) in cycle.iter().enumerate() {
        let (q, r, ei) = e;
        let other = forest_other_vertex_key(e, cur_v);
        let p0 = inset_map[&cur_v];
        let p1 = inset_map[&other];
        let (cx, cy) = coord_to_pixel(q, r);
        let (ax, ay) = HEX_VERT_OFFSETS[ei];
        let (bx, by) = HEX_VERT_OFFSETS[(ei + 1) % 6];
        let lmx = (ax + bx) * 0.5;
        let lmy = (ay + by) * 0.5;
        let ex = cx + lmx;
        let ey = cy + lmy;
        let rdx = ex - cx;
        let rdy = ey - cy;
        let rlen = (rdx * rdx + rdy * rdy).sqrt().max(1e-9);
        let ux = rdx / rlen;
        let uy = rdy / rlen;

        let cps = water_edge_bezier24_controls(p0, p1, q, r, ei, ux, uy);
        let steps = WATER_BEZIER24_STEPS.max(48);

        if i == 0 {
            ctx.move_to(p0.0, p0.1);
        }
        for j in 1..=steps {
            let t = j as f64 / steps as f64;
            let pt = bezier24_eval(cps, t);
            ctx.line_to(pt.0, pt.1);
        }
        cur_v = other;
    }
    ctx.close_path();
}

/// 一個連通林區：弧線內側填深綠，再沿同路徑描林緣線
fn fill_and_stroke_forest_component_arcs(
    ctx: &web_sys::CanvasRenderingContext2d,
    comp: &[(i32, i32)],
    terrain_of: &HashMap<(i32, i32), Terrain>,
    line_width: f64,
) {
    let edges = component_boundary_edges(comp);
    if edges.is_empty() {
        return;
    }
    let cycles = forest_trace_boundary_cycles(&edges);
    if cycles.is_empty() {
        return;
    }

    let mut pieces: Vec<ForestLoopPiece> = Vec::new();
    for cycle in cycles {
        let q0 = cycle[0].0;
        let r0 = cycle[0].1;
        let Some(&t) = terrain_of.get(&(q0, r0)) else {
            continue;
        };
        let inset_map = forest_loop_inset_map(&cycle);
        pieces.push((t, cycle, inset_map));
    }
    if pieces.is_empty() {
        return;
    }

    // 填色：多迴線（外輪廓 + 破孔）用 even-odd 保留孔洞
    let fill_t = pieces[0].0;
    ctx.begin_path();
    for (_, cycle, inset_map) in &pieces {
        append_forest_loop_path(ctx, cycle, inset_map);
    }
    ctx.set_fill_style_str(forest_interior_fill_color(fill_t));
    let fill_rule = if pieces.len() > 1 {
        CanvasWindingRule::Evenodd
    } else {
        CanvasWindingRule::Nonzero
    };
    ctx.fill_with_canvas_winding_rule(fill_rule);

    for (t, cycle, inset_map) in &pieces {
        ctx.begin_path();
        append_forest_loop_path(ctx, cycle, inset_map);
        ctx.set_stroke_style_str(forest_boundary_stroke_color(*t));
        ctx.set_line_width(line_width);
        ctx.set_line_cap("round");
        ctx.set_line_join("round");
        ctx.stroke();
    }
}

/// 一個連通水域：水岸弧線內填藍、再描邊（格面已與草原同色）
fn fill_and_stroke_water_component_arcs(
    ctx: &web_sys::CanvasRenderingContext2d,
    comp: &[(i32, i32)],
    terrain_of: &HashMap<(i32, i32), Terrain>,
    line_width: f64,
) {
    let edges = component_boundary_edges(comp);
    if edges.is_empty() {
        return;
    }
    let cycles = forest_trace_boundary_cycles(&edges);
    if cycles.is_empty() {
        return;
    }

    let mut pieces: Vec<ForestLoopPiece> = Vec::new();
    for cycle in cycles {
        let q0 = cycle[0].0;
        let r0 = cycle[0].1;
        let Some(&t) = terrain_of.get(&(q0, r0)) else {
            continue;
        };
        let inset_map = water_loop_inset_map(&cycle, comp.len());
        pieces.push((t, cycle, inset_map));
    }
    if pieces.is_empty() {
        return;
    }

    let fill_t = pieces[0].0;
    ctx.begin_path();
    for (_, cycle, inset_map) in &pieces {
        append_water_loop_path(ctx, cycle, inset_map);
    }
    ctx.set_fill_style_str(water_interior_fill_color(fill_t));
    let fill_rule = if pieces.len() > 1 {
        CanvasWindingRule::Evenodd
    } else {
        CanvasWindingRule::Nonzero
    };
    ctx.fill_with_canvas_winding_rule(fill_rule);

    for (t, cycle, inset_map) in &pieces {
        ctx.begin_path();
        append_water_loop_path(ctx, cycle, inset_map);
        ctx.set_stroke_style_str(water_boundary_stroke_color(*t));
        ctx.set_line_width(line_width);
        ctx.set_line_cap("round");
        ctx.set_line_join("round");
        ctx.stroke();
    }
}

fn canvas_view_rect(canvas: &web_sys::HtmlCanvasElement) -> web_sys::DomRect {
    if let Some(parent) = canvas.parent_element() {
        parent.get_bounding_client_rect()
    } else {
        canvas.get_bounding_client_rect()
    }
}

fn coord_to_pixel(q: i32, r: i32) -> (f64, f64) {
    let x = HEX_R * SQRT3 * (q as f64 + r as f64 / 2.0);
    let y = HEX_R * 1.5 * r as f64;
    (x, y)
}

/// 將「視窗中心」對齊裝置像素格（僅用於 Canvas transform，不改互動用 camera）。
/// 否則平移時弧線描邊與格線之間會有次像素抖動。
fn snap_camera_to_device_pixels(cam_x: f64, cam_y: f64, zoom: f64, dpr: f64) -> (f64, f64) {
    let k = zoom * dpr;
    if !k.is_finite() || k <= 1e-12 {
        return (cam_x, cam_y);
    }
    let cx = (cam_x * k).round() / k;
    let cy = (cam_y * k).round() / k;
    (cx, cy)
}

/// 在連通區 `comp` 中選與 **像素座標重心** 最近之一格（同距離則 `(q,r)` 字典序）
fn label_rep_coord_nearest_centroid(comp: &[(i32, i32)]) -> (i32, i32) {
    let n = comp.len() as f64;
    debug_assert!(n > 0.0);
    let mut sx = 0.0_f64;
    let mut sy = 0.0_f64;
    for &(q, r) in comp {
        let (px, py) = coord_to_pixel(q, r);
        sx += px;
        sy += py;
    }
    let cx = sx / n;
    let cy = sy / n;
    comp
        .iter()
        .copied()
        .min_by(|&(q1, r1), &(q2, r2)| {
            let d1 = {
                let (px, py) = coord_to_pixel(q1, r1);
                (px - cx).powi(2) + (py - cy).powi(2)
            };
            let d2 = {
                let (px, py) = coord_to_pixel(q2, r2);
                (px - cx).powi(2) + (py - cy).powi(2)
            };
            d1.partial_cmp(&d2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| q1.cmp(&q2))
                .then_with(|| r1.cmp(&r2))
        })
        .expect("non-empty component")
}

fn pixel_to_axial(wx: f64, wy: f64) -> (f64, f64) {
    let q = (SQRT3 / 3.0 * wx - (1.0 / 3.0) * wy) / HEX_R;
    let r = ((2.0 / 3.0) * wy) / HEX_R;
    (q, r)
}

fn axial_round(q: f64, r: f64) -> HexCoord {
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

    HexCoord {
        q: x as i32,
        r: z as i32,
    }
}

fn axial_distance(a: HexCoord, b: HexCoord) -> i32 {
    let dq = (a.q - b.q).abs();
    let dr = (a.r - b.r).abs();
    let ds = ((-a.q - a.r) - (-b.q - b.r)).abs();
    dq.max(dr).max(ds)
}

fn axial_line(a: HexCoord, b: HexCoord) -> Vec<HexCoord> {
    let n = axial_distance(a, b);
    if n == 0 {
        return vec![a];
    }
    let mut out = Vec::with_capacity((n + 1) as usize);
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let q = a.q as f64 + (b.q - a.q) as f64 * t;
        let r = a.r as f64 + (b.r - a.r) as f64 * t;
        out.push(axial_round(q, r));
    }
    out
}

fn brush_disk(center: HexCoord, radius: i32) -> Vec<HexCoord> {
    if radius <= 0 {
        return vec![center];
    }
    let mut out = Vec::new();
    for dq in -radius..=radius {
        let r_min = (-radius).max(-dq - radius);
        let r_max = radius.min(-dq + radius);
        for dr in r_min..=r_max {
            out.push(HexCoord {
                q: center.q + dq,
                r: center.r + dr,
            });
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, zoom: 1.0 }
    }
}

#[component]
pub fn HexGrid(
    cells: Signal<Vec<HexCell>>,
    selected: ReadSignal<Option<HexCoord>>,
    selected_many: ReadSignal<Vec<HexCoord>>,
    set_selected: WriteSignal<Option<HexCoord>>,
    brush_size: ReadSignal<u8>,
    tool_mode: ReadSignal<ToolMode>,
    #[prop(into)] on_paint: Callback<HexCoord>,
    #[prop(into)] on_erase: Callback<HexCoord>,
    #[prop(into)] on_select_toggle: Callback<HexCoord>,
    #[prop(into)] on_select_add: Callback<HexCoord>,
    #[prop(into)] on_move_selection: Callback<(HexCoord, HexCoord)>,
    #[prop(into)] on_load_json: Callback<HexCoord>,
) -> impl IntoView {
    let (camera, set_camera) = signal(Camera::default());
    let (panning, set_panning) = signal(false);
    let (painting, set_painting) = signal(false);
    let (last_painted, set_last_painted) = signal::<Option<HexCoord>>(None);
    let (last_stroke_center, set_last_stroke_center) = signal::<Option<HexCoord>>(None);
    let (drag_start, set_drag_start) = signal((0.0_f64, 0.0_f64));
    let (cam_start, set_cam_start) = signal((0.0_f64, 0.0_f64));
    let (container_size, set_container_size) = signal((800.0_f64, 600.0_f64));
    let (touch_dist, set_touch_dist) = signal(0.0_f64);
    let (ctx_menu_pos, set_ctx_menu_pos) = signal((0.0_f64, 0.0_f64));
    let (ctx_menu_coord, set_ctx_menu_coord) = signal::<Option<HexCoord>>(None);
    let (move_anchor, set_move_anchor) = signal::<Option<HexCoord>>(None);
    let (select_start, set_select_start) = signal::<Option<HexCoord>>(None);
    let (select_dragged, set_select_dragged) = signal(false);
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    let update_size = move || {
        if let Some(el) = canvas_ref.get() {
            let rect = canvas_view_rect(&el);
            let w = rect.width();
            let h = rect.height();
            if w > 0.0 && h > 0.0 {
                let dpr = web_sys::window()
                    .map(|win| win.device_pixel_ratio())
                    .unwrap_or(1.0)
                    .max(1.0);
                el.set_width((w * dpr).round() as u32);
                el.set_height((h * dpr).round() as u32);
                set_container_size.set((w, h));
            }
        }
    };

    let paint_screen_pos = {
        let on_paint = on_paint;
        let on_erase = on_erase;
        let on_select_add = on_select_add;
        let set_selected = set_selected;
        move |sx: f64, sy: f64, cw: f64, ch: f64| {
            let cam = camera.get_untracked();
            let wx = (sx - cw * 0.5) / cam.zoom + cam.x;
            let wy = (sy - ch * 0.5) / cam.zoom + cam.y;
            let (aq, ar) = pixel_to_axial(wx, wy);
            let center = axial_round(aq, ar);
            if last_painted.get_untracked() == Some(center) {
                return;
            }
            set_last_painted.set(Some(center));
            match tool_mode.get_untracked() {
                ToolMode::Paint | ToolMode::Erase => set_selected.set(Some(center)),
                ToolMode::View | ToolMode::Select | ToolMode::Move => {}
            }

            let radius = (brush_size.get_untracked() as i32 - 1).max(0);
            let stroke_centers = if let Some(prev) = last_stroke_center.get_untracked() {
                axial_line(prev, center)
            } else {
                vec![center]
            };
            set_last_stroke_center.set(Some(center));

            for c in stroke_centers {
                for b in brush_disk(c, radius) {
                    match tool_mode.get_untracked() {
                        ToolMode::View => {}
                        ToolMode::Erase => on_erase.run(b),
                        ToolMode::Paint => on_paint.run(b),
                        ToolMode::Select => on_select_add.run(b),
                        ToolMode::Move => {}
                    }
                }
            }
        }
    };

    let pick_coord_at = move |sx: f64, sy: f64, cw: f64, ch: f64| -> HexCoord {
        let cam = camera.get_untracked();
        let wx = (sx - cw * 0.5) / cam.zoom + cam.x;
        let wy = (sy - ch * 0.5) / cam.zoom + cam.y;
        let (aq, ar) = pixel_to_axial(wx, wy);
        axial_round(aq, ar)
    };

    Effect::new(move || {
        update_size();

        let Some(canvas) = canvas_ref.get() else { return };
        let event_target_canvas = canvas.clone();
        let target: &web_sys::EventTarget = event_target_canvas.unchecked_ref();

        // window resize
        if let Some(win) = web_sys::window() {
            let resize_cb = Closure::<dyn Fn()>::new({
                let update_size = update_size;
                move || update_size()
            });
            let _ = win.add_event_listener_with_callback("resize", resize_cb.as_ref().unchecked_ref());
            resize_cb.forget();
        }

        // wheel zoom
        let wheel_cb = Closure::<dyn Fn(web_sys::WheelEvent)>::new({
            let set_camera = set_camera;
            move |ev: web_sys::WheelEvent| {
                ev.prevent_default();
                let delta = ev.delta_y();
                let factor = if delta > 0.0 { 0.9 } else { 1.1 };
                set_camera.update(|c| {
                    c.zoom = (c.zoom * factor).clamp(0.03, 8.0);
                });
            }
        });
        let wheel_opts = web_sys::AddEventListenerOptions::new();
        wheel_opts.set_passive(false);
        let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
            "wheel",
            wheel_cb.as_ref().unchecked_ref(),
            &wheel_opts,
        );
        wheel_cb.forget();

        // mouse down
        let mousedown_cb = Closure::<dyn Fn(web_sys::MouseEvent)>::new({
            let set_panning = set_panning;
            let set_painting = set_painting;
            let set_drag_start = set_drag_start;
            let set_cam_start = set_cam_start;
            let paint_screen_pos = paint_screen_pos;
            let pick_coord_at = pick_coord_at;
            let set_selected = set_selected;
            let canvas_for_down = canvas.clone();
            move |ev: web_sys::MouseEvent| {
                // 左鍵：繪製；中鍵：平移；右鍵：載入 JSON
                if ev.button() == 0 {
                    ev.prevent_default();
                    set_ctx_menu_coord.set(None);
                    let rect = canvas_view_rect(&canvas_for_down);
                    let sx = ev.client_x() as f64 - rect.left();
                    let sy = ev.client_y() as f64 - rect.top();
                    let coord = pick_coord_at(sx, sy, rect.width(), rect.height());
                    match tool_mode.get_untracked() {
                        ToolMode::View => {
                            // 左鍵拖曳＝平移（與中鍵相同），縮放用滾輪；不會誤繪格子
                            set_painting.set(false);
                            set_panning.set(true);
                            set_drag_start.set((ev.client_x() as f64, ev.client_y() as f64));
                            let cam = camera.get_untracked();
                            set_cam_start.set((cam.x, cam.y));
                            set_select_start.set(None);
                            set_select_dragged.set(false);
                        }
                        ToolMode::Paint | ToolMode::Erase => {
                            set_painting.set(true);
                            set_panning.set(false);
                            set_last_painted.set(None);
                            set_last_stroke_center.set(None);
                            set_select_start.set(None);
                            set_select_dragged.set(false);
                            paint_screen_pos(sx, sy, rect.width(), rect.height());
                        }
                        ToolMode::Select => {
                            set_painting.set(true);
                            set_panning.set(false);
                            set_last_painted.set(None);
                            set_last_stroke_center.set(None);
                            set_select_start.set(Some(coord));
                            set_select_dragged.set(false);
                        }
                        ToolMode::Move => {
                            let sel = selected_many.get_untracked();
                            if sel.contains(&coord) {
                                set_move_anchor.set(Some(coord));
                            } else {
                                set_move_anchor.set(None);
                            }
                            set_painting.set(false);
                            set_panning.set(false);
                            set_select_start.set(None);
                            set_select_dragged.set(false);
                        }
                    }
                    return;
                }
                if ev.button() == 1 {
                    ev.prevent_default();
                    set_ctx_menu_coord.set(None);
                    set_panning.set(true);
                    set_painting.set(false);
                    set_drag_start.set((ev.client_x() as f64, ev.client_y() as f64));
                    let cam = camera.get_untracked();
                    set_cam_start.set((cam.x, cam.y));
                    return;
                }
                if ev.button() == 2 {
                    ev.prevent_default();
                    set_panning.set(false);
                    set_painting.set(false);
                    set_last_painted.set(None);
                    set_last_stroke_center.set(None);
                    let rect = canvas_view_rect(&canvas_for_down);
                    let sx = ev.client_x() as f64 - rect.left();
                    let sy = ev.client_y() as f64 - rect.top();
                    let coord = pick_coord_at(sx, sy, rect.width(), rect.height());
                    set_selected.set(Some(coord));
                    set_ctx_menu_pos.set((sx, sy));
                    set_ctx_menu_coord.set(Some(coord));
                }
            }
        });
        let _ = target.add_event_listener_with_callback("mousedown", mousedown_cb.as_ref().unchecked_ref());
        mousedown_cb.forget();

        // mouse move
        let mousemove_cb = Closure::<dyn Fn(web_sys::MouseEvent)>::new({
            let set_camera = set_camera;
            let paint_screen_pos = paint_screen_pos;
            let canvas_for_move = canvas.clone();
            move |ev: web_sys::MouseEvent| {
                if painting.get_untracked() {
                    let rect = canvas_view_rect(&canvas_for_move);
                    let sx = ev.client_x() as f64 - rect.left();
                    let sy = ev.client_y() as f64 - rect.top();
                    if tool_mode.get_untracked() == ToolMode::Select {
                        let curr = pick_coord_at(sx, sy, rect.width(), rect.height());
                        if let Some(start) = select_start.get_untracked() {
                            if curr != start && !select_dragged.get_untracked() {
                                on_select_add.run(start);
                                set_select_dragged.set(true);
                            }
                        }
                    }
                    paint_screen_pos(sx, sy, rect.width(), rect.height());
                    return;
                }
                if panning.get_untracked() {
                    let (sx, sy) = drag_start.get_untracked();
                    let dx = ev.client_x() as f64 - sx;
                    let dy = ev.client_y() as f64 - sy;
                    let cam = camera.get_untracked();
                    let (cx, cy) = cam_start.get_untracked();
                    set_camera.set(Camera {
                        x: cx - dx / cam.zoom,
                        y: cy - dy / cam.zoom,
                        zoom: cam.zoom,
                    });
                }
            }
        });
        let _ = target.add_event_listener_with_callback("mousemove", mousemove_cb.as_ref().unchecked_ref());
        mousemove_cb.forget();

        // mouse up
        let mouseup_cb = Closure::<dyn Fn(web_sys::MouseEvent)>::new({
            let set_panning = set_panning;
            let set_painting = set_painting;
            let pick_coord_at = pick_coord_at;
            let on_select_toggle = on_select_toggle;
            let on_move_selection = on_move_selection;
            let canvas_for_up = canvas.clone();
            move |_ev: web_sys::MouseEvent| {
                if let Some(from) = move_anchor.get_untracked() {
                    let rect = canvas_view_rect(&canvas_for_up);
                    let sx = _ev.client_x() as f64 - rect.left();
                    let sy = _ev.client_y() as f64 - rect.top();
                    let to = pick_coord_at(sx, sy, rect.width(), rect.height());
                    on_move_selection.run((from, to));
                }
                if tool_mode.get_untracked() == ToolMode::Select {
                    if let Some(start) = select_start.get_untracked() {
                        if !select_dragged.get_untracked() {
                            on_select_toggle.run(start);
                        }
                    }
                }
                set_move_anchor.set(None);
                set_select_start.set(None);
                set_select_dragged.set(false);
                set_panning.set(false);
                set_painting.set(false);
                set_last_painted.set(None);
                set_last_stroke_center.set(None);
            }
        });
        let _ = target.add_event_listener_with_callback("mouseup", mouseup_cb.as_ref().unchecked_ref());
        let _ = target.add_event_listener_with_callback("mouseleave", mouseup_cb.as_ref().unchecked_ref());
        mouseup_cb.forget();

        let contextmenu_cb = Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
            ev.prevent_default();
        });
        let _ = target.add_event_listener_with_callback("contextmenu", contextmenu_cb.as_ref().unchecked_ref());
        contextmenu_cb.forget();

        // touchstart
        let ts_cb = Closure::<dyn Fn(web_sys::TouchEvent)>::new({
            let set_painting = set_painting;
            let set_panning = set_panning;
            let set_drag_start = set_drag_start;
            let set_cam_start = set_cam_start;
            let set_touch_dist = set_touch_dist;
            let paint_screen_pos = paint_screen_pos;
            let canvas_for_touch = canvas.clone();
            move |ev: web_sys::TouchEvent| {
                let touches = ev.touches();
                set_ctx_menu_coord.set(None);
                if touches.length() == 1 {
                    if let Some(t) = touches.get(0) {
                        let rect = canvas_view_rect(&canvas_for_touch);
                        let sx = t.client_x() as f64 - rect.left();
                        let sy = t.client_y() as f64 - rect.top();
                        let coord = pick_coord_at(sx, sy, rect.width(), rect.height());
                        match tool_mode.get_untracked() {
                            ToolMode::View => {
                                set_painting.set(false);
                                set_panning.set(true);
                                set_drag_start.set((t.client_x() as f64, t.client_y() as f64));
                                let cam = camera.get_untracked();
                                set_cam_start.set((cam.x, cam.y));
                                set_select_start.set(None);
                                set_select_dragged.set(false);
                            }
                            ToolMode::Paint | ToolMode::Erase => {
                                set_painting.set(true);
                                set_panning.set(false);
                                set_last_painted.set(None);
                                set_last_stroke_center.set(None);
                                set_select_start.set(None);
                                set_select_dragged.set(false);
                                paint_screen_pos(sx, sy, rect.width(), rect.height());
                            }
                            ToolMode::Select => {
                                set_painting.set(true);
                                set_panning.set(false);
                                set_last_painted.set(None);
                                set_last_stroke_center.set(None);
                                set_select_start.set(Some(coord));
                                set_select_dragged.set(false);
                            }
                            ToolMode::Move => {
                                let sel = selected_many.get_untracked();
                                if sel.contains(&coord) {
                                    set_move_anchor.set(Some(coord));
                                } else {
                                    set_move_anchor.set(None);
                                }
                                set_painting.set(false);
                                set_panning.set(false);
                                set_select_start.set(None);
                                set_select_dragged.set(false);
                            }
                        }
                    }
                } else if touches.length() == 2 {
                    set_painting.set(false);
                    set_panning.set(true);
                    if let (Some(a), Some(b)) = (touches.get(0), touches.get(1)) {
                        let dx = a.client_x() as f64 - b.client_x() as f64;
                        let dy = a.client_y() as f64 - b.client_y() as f64;
                        set_touch_dist.set((dx * dx + dy * dy).sqrt());
                        set_drag_start.set(((a.client_x() + b.client_x()) as f64 * 0.5, (a.client_y() + b.client_y()) as f64 * 0.5));
                        let cam = camera.get_untracked();
                        set_cam_start.set((cam.x, cam.y));
                    }
                }
            }
        });

        let opts = web_sys::AddEventListenerOptions::new();
        opts.set_passive(false);

        let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
            "touchstart",
            ts_cb.as_ref().unchecked_ref(),
            &opts,
        );
        ts_cb.forget();

        // touchmove（必須 non-passive 才能 preventDefault）
        let tm_cb = Closure::<dyn Fn(web_sys::TouchEvent)>::new({
            let set_touch_dist = set_touch_dist;
            let paint_screen_pos = paint_screen_pos;
            let canvas_for_touch = canvas.clone();
            move |ev: web_sys::TouchEvent| {
                ev.prevent_default();
                let touches = ev.touches();
                if touches.length() == 1 && painting.get_untracked() {
                    if let Some(t) = touches.get(0) {
                        let rect = canvas_view_rect(&canvas_for_touch);
                        let sx = t.client_x() as f64 - rect.left();
                        let sy = t.client_y() as f64 - rect.top();
                        if tool_mode.get_untracked() == ToolMode::Select {
                            let curr = pick_coord_at(sx, sy, rect.width(), rect.height());
                            if let Some(start) = select_start.get_untracked() {
                                if curr != start && !select_dragged.get_untracked() {
                                    on_select_add.run(start);
                                    set_select_dragged.set(true);
                                }
                            }
                        }
                        paint_screen_pos(sx, sy, rect.width(), rect.height());
                    }
                } else if touches.length() == 2 && panning.get_untracked() {
                    if let (Some(a), Some(b)) = (touches.get(0), touches.get(1)) {
                        let center_x = (a.client_x() + b.client_x()) as f64 * 0.5;
                        let center_y = (a.client_y() + b.client_y()) as f64 * 0.5;
                        let (sx, sy) = drag_start.get_untracked();
                        let ddx = center_x - sx;
                        let ddy = center_y - sy;
                        let cam = camera.get_untracked();
                        let (cx, cy) = cam_start.get_untracked();
                        let dx = a.client_x() as f64 - b.client_x() as f64;
                        let dy = a.client_y() as f64 - b.client_y() as f64;
                        let new_dist = (dx * dx + dy * dy).sqrt();
                        let old_dist = touch_dist.get_untracked();
                        if old_dist > 0.0 {
                            let factor = new_dist / old_dist;
                            set_camera.update(|c| {
                                c.zoom = (c.zoom * factor).clamp(0.03, 8.0);
                                c.x = cx - ddx / cam.zoom;
                                c.y = cy - ddy / cam.zoom;
                            });
                        }
                        set_touch_dist.set(new_dist);
                    }
                }
            }
        });

        let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
            "touchmove",
            tm_cb.as_ref().unchecked_ref(),
            &opts,
        );
        tm_cb.forget();

        // touchend
        let te_cb = Closure::<dyn Fn(web_sys::TouchEvent)>::new({
            let set_painting = set_painting;
            let set_panning = set_panning;
            let set_touch_dist = set_touch_dist;
            let pick_coord_at = pick_coord_at;
            let on_select_toggle = on_select_toggle;
            let on_move_selection = on_move_selection;
            let canvas_for_touch = canvas.clone();
            move |_ev: web_sys::TouchEvent| {
                if let Some(from) = move_anchor.get_untracked() {
                    if let Some(t) = _ev.changed_touches().get(0) {
                        let rect = canvas_view_rect(&canvas_for_touch);
                        let sx = t.client_x() as f64 - rect.left();
                        let sy = t.client_y() as f64 - rect.top();
                        let to = pick_coord_at(sx, sy, rect.width(), rect.height());
                        on_move_selection.run((from, to));
                    }
                }
                if tool_mode.get_untracked() == ToolMode::Select {
                    if let Some(start) = select_start.get_untracked() {
                        if !select_dragged.get_untracked() {
                            on_select_toggle.run(start);
                        }
                    }
                }
                set_move_anchor.set(None);
                set_select_start.set(None);
                set_select_dragged.set(false);
                set_painting.set(false);
                set_panning.set(false);
                set_last_painted.set(None);
                set_last_stroke_center.set(None);
                set_touch_dist.set(0.0);
            }
        });

        let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
            "touchend",
            te_cb.as_ref().unchecked_ref(),
            &opts,
        );
        te_cb.forget();
    });

    // 畫面重繪：Canvas + LOD
    Effect::new(move || {
        let cs = cells.get();
        let cam = camera.get();
        let _ = container_size.get();
        let sel = selected.get();
        let Some(canvas) = canvas_ref.get() else { return };

        let rect = canvas_view_rect(&canvas);
        let w_css = rect.width();
        let h_css = rect.height();
        if w_css <= 0.0 || h_css <= 0.0 {
            return;
        }
        let dpr = web_sys::window()
            .map(|win| win.device_pixel_ratio())
            .unwrap_or(1.0)
            .max(1.0);
        let w_u32 = (w_css * dpr).round() as u32;
        let h_u32 = (h_css * dpr).round() as u32;
        if canvas.width() != w_u32 || canvas.height() != h_u32 {
            canvas.set_width(w_u32);
            canvas.set_height(h_u32);
        }

        let Ok(Some(raw_ctx)) = canvas.get_context("2d") else { return };
        let Ok(ctx) = raw_ctx.dyn_into::<web_sys::CanvasRenderingContext2d>() else { return };

        let _ = ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        ctx.set_fill_style_str("#0f1923");
        ctx.fill_rect(0.0, 0.0, w_u32 as f64, h_u32 as f64);
        let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
        ctx.save();
        let (cam_dx, cam_dy) = snap_camera_to_device_pixels(cam.x, cam.y, cam.zoom, dpr);
        let _ = ctx.translate(w_css * 0.5, h_css * 0.5);
        let _ = ctx.scale(cam.zoom, cam.zoom);
        let _ = ctx.translate(-cam_dx, -cam_dy);

        let vw = w_css / cam.zoom;
        let vh = h_css / cam.zoom;
        // 與實際 transform 一致，避免裁切範圍與畫面差半個像素
        let vx = cam_dx - vw / 2.0;
        let vy = cam_dy - vh / 2.0;
        let margin = HEX_R * 2.0;
        let min_x = vx - margin;
        let max_x = vx + vw + margin;
        let min_y = vy - margin;
        let max_y = vy + vh + margin;

        let label_reps = label_representative_coords(&cs);

        // 以視口四角反推 axial 範圍，避免右上/左下被裁切
        let mut qf_min = f64::INFINITY;
        let mut qf_max = f64::NEG_INFINITY;
        let mut rf_min = f64::INFINITY;
        let mut rf_max = f64::NEG_INFINITY;
        for (wx, wy) in [
            (min_x, min_y),
            (max_x, min_y),
            (min_x, max_y),
            (max_x, max_y),
        ] {
            let (aq, ar) = pixel_to_axial(wx, wy);
            qf_min = qf_min.min(aq);
            qf_max = qf_max.max(aq);
            rf_min = rf_min.min(ar);
            rf_max = rf_max.max(ar);
        }
        let q_min = qf_min.floor() as i32 - 3;
        let q_max = qf_max.ceil() as i32 + 3;
        let r_min = rf_min.floor() as i32 - 3;
        let r_max = rf_max.ceil() as i32 + 3;

        // 背景網格：中高倍才畫
        if cam.zoom >= 0.28 {
            ctx.begin_path();
            for r in r_min..=r_max {
                for q in q_min..=q_max {
                    let (px, py) = coord_to_pixel(q, r);
                    if px < min_x || px > max_x || py < min_y || py > max_y {
                        continue;
                    }
                    let (ox0, oy0) = HEX_VERT_OFFSETS[0];
                    ctx.move_to(px + ox0, py + oy0);
                    for (ox, oy) in HEX_VERT_OFFSETS.iter().skip(1) {
                        ctx.line_to(px + *ox, py + *oy);
                    }
                    ctx.close_path();
                }
            }
            ctx.set_stroke_style_str("#1e2d3d");
            ctx.set_line_width((0.6 / cam.zoom).clamp(0.2, 1.0));
            ctx.stroke();
        }

        // 前景格子：LOD
        if cam.zoom < 0.15 {
            // 粒子模式：按顏色批次
            let mut buckets: HashMap<&'static str, Vec<(f64, f64)>> = HashMap::new();
            for cell in cs.iter() {
                let (px, py) = coord_to_pixel(cell.coord.q, cell.coord.r);
                if px < min_x || px > max_x || py < min_y || py > max_y {
                    continue;
                }
                buckets.entry(cell.terrain.color()).or_default().push((px, py));
            }
            for (color, pts) in buckets.iter() {
                ctx.set_fill_style_str(color);
                for (px, py) in pts.iter() {
                    ctx.fill_rect(px - 1.4, py - 1.4, 2.8, 2.8);
                }
            }
        } else {
            for cell in cs.iter() {
                let (px, py) = coord_to_pixel(cell.coord.q, cell.coord.r);
                if px < min_x || px > max_x || py < min_y || py > max_y {
                    continue;
                }

                ctx.begin_path();
                let (ox0, oy0) = HEX_VERT_OFFSETS[0];
                ctx.move_to(px + ox0, py + oy0);
                for (ox, oy) in HEX_VERT_OFFSETS.iter().skip(1) {
                    ctx.line_to(px + *ox, py + *oy);
                }
                ctx.close_path();
                ctx.set_fill_style_str(cell.terrain.color());
                ctx.fill();

                // 彩格預設不畫六角邊線（避免格與格之間出現深灰縫）；僅選取／多選時描邊
                if cam.zoom >= 0.35 {
                    let is_sel = sel == Some(cell.coord);
                    let is_multi = selected_many.get().contains(&cell.coord);
                    if is_multi || is_sel {
                        let stroke = if is_multi { "#22d3ee" } else { "#ffcc00" };
                        ctx.set_stroke_style_str(stroke);
                        ctx.set_line_width(2.0 / cam.zoom);
                        ctx.stroke();
                    }
                }

                // 遊戲釘死彩格（如 player_spawn 標籤）
                if cell.tags.iter().any(|t| t == "player_spawn") && cam.zoom >= 0.3 {
                    ctx.begin_path();
                    ctx.move_to(px + ox0, py + oy0);
                    for (ox, oy) in HEX_VERT_OFFSETS.iter().skip(1) {
                        ctx.line_to(px + *ox, py + *oy);
                    }
                    ctx.close_path();
                    ctx.set_stroke_style_str("#f59e0b");
                    ctx.set_line_width((2.4 / cam.zoom).clamp(1.0, 3.2));
                    ctx.stroke();
                }

                if cam.zoom >= 0.6 {
                    let cid = (cell.coord.q, cell.coord.r);
                    if label_reps.contains(&cid) {
                        ctx.set_fill_style_str("#e0e8f0");
                        ctx.set_font(&format!("{}px sans-serif", (8.0 / cam.zoom).max(7.0)));
                        ctx.set_text_align("center");
                        ctx.set_text_baseline("middle");
                        let label = cell_display_label(cell);
                        let _ = ctx.fill_text(&label, px, py + 2.0);
                    }
                }
            }
        }

        // === 水岸：連通水域（先畫，林緣再疊上）===
        if cam.zoom >= 0.25 {
            let water_set: HashSet<(i32, i32)> = cs
                .iter()
                .filter(|c| is_water_terrain(c.terrain))
                .map(|c| (c.coord.q, c.coord.r))
                .collect();

            if !water_set.is_empty() {
                let wlw = (1.0 / cam.zoom).clamp(0.48, 2.4);
                let water_terrain_of: HashMap<(i32, i32), Terrain> = cs
                    .iter()
                    .filter(|c| is_water_terrain(c.terrain))
                    .map(|c| ((c.coord.q, c.coord.r), c.terrain))
                    .collect();
                let water_comps = forest_connected_components(&water_set);
                for comp in water_comps {
                    let any_visible = comp.iter().any(|&(q, r)| {
                        let (px, py) = coord_to_pixel(q, r);
                        px >= min_x - HEX_R
                            && px <= max_x + HEX_R
                            && py >= min_y - HEX_R
                            && py <= max_y + HEX_R
                    });
                    if any_visible {
                        fill_and_stroke_water_component_arcs(&ctx, &comp, &water_terrain_of, wlw);
                    }
                }
            }
        }

        // === 林緣：連通林區外輪廓與破孔內輪廓，迴線上弧段相連 ===
        if cam.zoom >= 0.25 {
            let forest_set: HashSet<(i32, i32)> = cs
                .iter()
                .filter(|c| is_forest_terrain(c.terrain))
                .map(|c| (c.coord.q, c.coord.r))
                .collect();

            if !forest_set.is_empty() {
                let lw = (1.15 / cam.zoom).clamp(0.55, 2.8);
                let terrain_of: HashMap<(i32, i32), Terrain> = cs
                    .iter()
                    .filter(|c| is_forest_terrain(c.terrain))
                    .map(|c| ((c.coord.q, c.coord.r), c.terrain))
                    .collect();
                let comps = forest_connected_components(&forest_set);
                for comp in comps {
                    let any_visible = comp.iter().any(|&(q, r)| {
                        let (px, py) = coord_to_pixel(q, r);
                        px >= min_x - HEX_R
                            && px <= max_x + HEX_R
                            && py >= min_y - HEX_R
                            && py <= max_y + HEX_R
                    });
                    if any_visible {
                        fill_and_stroke_forest_component_arcs(&ctx, &comp, &terrain_of, lw);
                    }
                }
            }
        }

        ctx.restore();
    });

    let on_ctx_load_json = {
        let on_load_json = on_load_json;
        move |_| {
            if let Some(coord) = ctx_menu_coord.get() {
                on_load_json.run(coord);
            }
            set_ctx_menu_coord.set(None);
        }
    };

    let on_ctx_delete = {
        let on_erase = on_erase;
        move |_| {
            if let Some(coord) = ctx_menu_coord.get() {
                on_erase.run(coord);
            }
            set_ctx_menu_coord.set(None);
        }
    };

    let hide_ctx_menu = move |ev: leptos::ev::MouseEvent| {
        if ev.button() == 0 {
            set_ctx_menu_coord.set(None);
        }
    };

    view! {
        <div class="hex-grid-wrap" on:mousedown=hide_ctx_menu>
            <canvas
                node_ref=canvas_ref
                style="background:#0f1923;touch-action:none;-webkit-user-select:none;user-select:none"
            >
            </canvas>
            <Show when=move || ctx_menu_coord.get().is_some()>
                <div
                    class="hex-ctx-menu"
                    style=move || {
                        let (x, y) = ctx_menu_pos.get();
                        format!("left:{:.1}px;top:{:.1}px;", x, y)
                    }
                    on:mousedown=move |ev| ev.stop_propagation()
                >
                    <button class="hex-ctx-item" on:click=on_ctx_load_json>"載入 JSON（Watabou）"</button>
                    <button class="hex-ctx-item danger" on:click=on_ctx_delete>"刪除當前格"</button>
                </div>
            </Show>
        </div>
    }
}
