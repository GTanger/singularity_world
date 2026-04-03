#![allow(clippy::redundant_locals)]

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::collections::{HashMap, HashSet};
use crate::types::{HexCell, HexCoord, Terrain, ToolMode};

const HEX_R: f64 = 28.0;
const SQRT3: f64 = 1.7320508075688772;
const HEX_VERT_OFFSETS: [(f64, f64); 6] = [
    (24.24871130596428, 14.0),
    (0.0, 28.0),
    (-24.24871130596428, 14.0),
    (-24.24871130596428, -14.0),
    (0.0, -28.0),
    (24.24871130596428, -14.0),
];

/// 與 `HEX_VERT_OFFSETS` **邊索引 0..5** 對齊的鄰格轴向增量 `(dq, dr)`。
/// 邊 `i` 為頂點 `i → (i+1)%6`；**必須**與畫布上該邊的外側鄰格一致，否則森林邊緣樹會畫在林區內部共用邊上。
/// （舊版誤用 E/Ne/… 固定順序，與頂點順序不一致，導致截圖中「森林格之間仍出現小樹」。）
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

/// 連通區內僅在字典序最小之一格顯示名稱（例如六格相連森林 → 一個「森林」）
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
        let rep = *comp.iter().min().expect("non-empty component");
        reps.insert(rep);
    }
    reps
}

/// 六角邊的兩個頂點索引（邊 i 連接頂點 i 和 (i+1)%6）
fn hex_edge_verts(edge_idx: usize) -> ((f64, f64), (f64, f64)) {
    let a = HEX_VERT_OFFSETS[edge_idx];
    let b = HEX_VERT_OFFSETS[(edge_idx + 1) % 6];
    (a, b)
}

/// 用確定性 hash 生成偽隨機（同座標同結果）
fn cell_hash(q: i32, r: i32, salt: u32) -> u64 {
    let mut h = (q as u64).wrapping_mul(0x9E3779B97F4A7C15)
        .wrapping_add((r as u64).wrapping_mul(0x517CC1B727220A95))
        .wrapping_add(salt as u64);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58476D1CE4E5B9);
    h ^= h >> 27;
    h
}

/// 在 hex 邊緣畫程序化小樹（bezier 樹冠 + 樹幹）
fn draw_edge_trees(
    ctx: &web_sys::CanvasRenderingContext2d,
    px: f64,
    py: f64,
    edge_idx: usize,
    q: i32,
    r: i32,
    is_heavy: bool,
) {
    let ((ax, ay), (bx, by)) = hex_edge_verts(edge_idx);
    // 邊中點
    let mx = (ax + bx) / 2.0;
    let my = (ay + by) / 2.0;
    // 朝外法線方向（從中心往邊中點）
    let out_x = mx;
    let out_y = my;
    let out_len = (out_x * out_x + out_y * out_y).sqrt().max(0.01);
    let nx = out_x / out_len;
    let ny = out_y / out_len;
    // 邊的切線方向
    let tx = bx - ax;
    let ty = by - ay;
    let tlen = (tx * tx + ty * ty).sqrt().max(0.01);
    let tnx = tx / tlen;
    let tny = ty / tlen;

    let tree_count = if is_heavy { 3 } else { 2 };

    for ti in 0..tree_count {
        let th = cell_hash(q, r, (edge_idx as u32) * 100 + ti);
        // 沿邊分佈位置（0.2 ~ 0.8 避開頂點）
        let frac = 0.2 + 0.6 * (ti as f64 + 0.5) / tree_count as f64;
        // 加微量隨機偏移
        let jitter = ((th % 100) as f64 / 100.0 - 0.5) * 0.15;
        let t = frac + jitter;
        let base_x = px + ax + (bx - ax) * t;
        let base_y = py + ay + (by - ay) * t;

        // 樹高（朝外長）
        let h_base = HEX_R * 0.35;
        let h_var = ((th >> 8) % 60) as f64 / 100.0;
        let tree_h = h_base * (0.7 + h_var);
        // 樹冠寬
        let w_base = HEX_R * 0.22;
        let w_var = ((th >> 16) % 50) as f64 / 100.0;
        let crown_w = w_base * (0.7 + w_var);

        // 樹頂
        let top_x = base_x + nx * tree_h;
        let top_y = base_y + ny * tree_h;

        // 樹幹（短線）
        let trunk_h = tree_h * 0.3;
        let trunk_top_x = base_x + nx * trunk_h;
        let trunk_top_y = base_y + ny * trunk_h;

        // 樹幹色
        ctx.set_stroke_style_str("#4a3728");
        ctx.set_line_width(1.2);
        ctx.begin_path();
        ctx.move_to(base_x, base_y);
        ctx.line_to(trunk_top_x, trunk_top_y);
        ctx.stroke();

        // 樹冠（兩條 bezier 圍成水滴形）
        let left_x = trunk_top_x - tnx * crown_w;
        let left_y = trunk_top_y - tny * crown_w;
        let right_x = trunk_top_x + tnx * crown_w;
        let right_y = trunk_top_y + tny * crown_w;

        // 樹冠色依密度
        let crown_color = if is_heavy { "#2d5a1e" } else { "#3d7a2e" };
        let crown_dark = if is_heavy { "#1e4015" } else { "#2d6020" };

        ctx.set_fill_style_str(crown_color);
        ctx.begin_path();
        ctx.move_to(left_x, left_y);
        // 左弧 → 頂
        ctx.quadratic_curve_to(
            trunk_top_x - tnx * crown_w * 0.3 + nx * tree_h * 0.7,
            trunk_top_y - tny * crown_w * 0.3 + ny * tree_h * 0.7,
            top_x,
            top_y,
        );
        // 頂 → 右弧
        ctx.quadratic_curve_to(
            trunk_top_x + tnx * crown_w * 0.3 + nx * tree_h * 0.7,
            trunk_top_y + tny * crown_w * 0.3 + ny * tree_h * 0.7,
            right_x,
            right_y,
        );
        ctx.close_path();
        ctx.fill();

        // 樹冠暗邊
        ctx.set_stroke_style_str(crown_dark);
        ctx.set_line_width(0.6);
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
        let _ = ctx.translate(w_css * 0.5, h_css * 0.5);
        let _ = ctx.scale(cam.zoom, cam.zoom);
        let _ = ctx.translate(-cam.x, -cam.y);

        let vw = w_css / cam.zoom;
        let vh = h_css / cam.zoom;
        let vx = cam.x - vw / 2.0;
        let vy = cam.y - vh / 2.0;
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

        // === 森林樹海：邊緣樹形 ===
        if cam.zoom >= 0.25 {
            // 建森林格集合
            let forest_set: HashSet<(i32, i32)> = cs.iter()
                .filter(|c| is_forest_terrain(c.terrain))
                .map(|c| (c.coord.q, c.coord.r))
                .collect();

            if !forest_set.is_empty() {
                for cell in cs.iter() {
                    if !is_forest_terrain(cell.terrain) {
                        continue;
                    }
                    let (px, py) = coord_to_pixel(cell.coord.q, cell.coord.r);
                    if px < min_x - HEX_R || px > max_x + HEX_R
                        || py < min_y - HEX_R || py > max_y + HEX_R
                    {
                        continue;
                    }
                    let is_heavy = matches!(cell.terrain, Terrain::ForestHeavy | Terrain::Jungle);

                    // 檢查六個方向，鄰居不是森林的邊畫樹
                    for (di, &(dq, dr)) in AXIAL_DIRS_FOR_EDGES.iter().enumerate() {
                        let nq = cell.coord.q + dq;
                        let nr = cell.coord.r + dr;
                        if !forest_set.contains(&(nq, nr)) {
                            draw_edge_trees(&ctx, px, py, di, cell.coord.q, cell.coord.r, is_heavy);
                        }
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
