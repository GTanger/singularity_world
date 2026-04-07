#![allow(clippy::redundant_locals)]

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use std::collections::HashMap;
use wasm_bindgen::JsCast;
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

/// 地圖外緣、或**不同屬性**兩格之分界（深霧灰）
const CELL_HEX_OUTLINE_COLOR: &str = "#5c6570";
/// **同屬性連群**內側邊：每邊只畫一次，偏淡
const CELL_HEX_INNER_EDGE_COLOR: &str = "rgba(92,101,112,0.20)";

/// 與標籤／合併規則一致：**同地形**且（無自訂名 **或** 自訂名相同）才算同一連群
#[derive(Clone, PartialEq, Eq, Hash)]
enum LabelMergeKey {
    TerrainOnly(Terrain),
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

/// 邊 `i`（頂點 `i → (i+1)%6`）對側鄰格之 axial 增量（與 `HEX_VERT_OFFSETS` 一致）
const EDGE_NEIGHBOR_DR: [(i32, i32); 6] = [
    (0, 1),
    (-1, 1),
    (-1, 0),
    (0, -1),
    (1, -1),
    (1, 0),
];

/// 畫面上顯示用的地名／地形字（與下方文字繪製一致）
fn cell_display_label(cell: &HexCell) -> String {
    if cell.name.trim().is_empty() {
        cell.terrain.label().to_string()
    } else {
        cell.name.clone()
    }
}

/// 繪製用：字元間加薄空格（U+2009）作字距，單字不加
fn label_with_thin_space_gaps(s: &str) -> String {
    let t = s.trim();
    let mut it = t.chars();
    let Some(first) = it.next() else {
        return String::new();
    };
    let mut out = String::with_capacity(t.len() + t.chars().count().max(1));
    out.push(first);
    for ch in it {
        out.push('\u{2009}');
        out.push(ch);
    }
    out
}

/// 每格六角邊：**同屬性連群**內側用淡線；**外緣**或**異屬性鄰格**分界用深線（每邊只畫一次）
fn stroke_each_cell_hex_outline(
    ctx: &web_sys::CanvasRenderingContext2d,
    cells: &[HexCell],
    cam_zoom: f64,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
) {
    let line_w = (1.0 / cam_zoom).clamp(0.55, 2.0);
    ctx.set_line_width(line_w);
    ctx.set_line_cap("round");
    ctx.set_line_join("round");

    let mut at: HashMap<(i32, i32), &HexCell> = HashMap::with_capacity(cells.len());
    for c in cells {
        at.insert((c.coord.q, c.coord.r), c);
    }

    for c in cells {
        let q = c.coord.q;
        let r = c.coord.r;
        let (px, py) = coord_to_pixel(q, r);
        if px < min_x || px > max_x || py < min_y || py > max_y {
            continue;
        }
        let my_key = label_merge_key(c);
        for ei in 0..6 {
            let (dq, dr) = EDGE_NEIGHBOR_DR[ei];
            let nq = q + dq;
            let nr = r + dr;
            let Some(nc) = at.get(&(nq, nr)) else {
                ctx.set_stroke_style_str(CELL_HEX_OUTLINE_COLOR);
                let (ax, ay) = HEX_VERT_OFFSETS[ei];
                let (bx, by) = HEX_VERT_OFFSETS[(ei + 1) % 6];
                ctx.begin_path();
                ctx.move_to(px + ax, py + ay);
                ctx.line_to(px + bx, py + by);
                ctx.stroke();
                continue;
            };
            // 有鄰格：只由 (q,r) 較小者畫一次
            if (q, r) > (nq, nr) {
                continue;
            }
            let same_group = label_merge_key(nc) == my_key;
            if same_group {
                ctx.set_stroke_style_str(CELL_HEX_INNER_EDGE_COLOR);
            } else {
                ctx.set_stroke_style_str(CELL_HEX_OUTLINE_COLOR);
            }
            let (ax, ay) = HEX_VERT_OFFSETS[ei];
            let (bx, by) = HEX_VERT_OFFSETS[(ei + 1) % 6];
            ctx.begin_path();
            ctx.move_to(px + ax, py + ay);
            ctx.line_to(px + bx, py + by);
            ctx.stroke();
        }
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
/// 否則平移時格線與互動之間會有次像素抖動。
fn snap_camera_to_device_pixels(cam_x: f64, cam_y: f64, zoom: f64, dpr: f64) -> (f64, f64) {
    let k = zoom * dpr;
    if !k.is_finite() || k <= 1e-12 {
        return (cam_x, cam_y);
    }
    let cx = (cam_x * k).round() / k;
    let cy = (cam_y * k).round() / k;
    (cx, cy)
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
                // 單指平移（檢視模式）：與滑鼠 mousemove 平移一致
                if touches.length() == 1 && panning.get_untracked() && !painting.get_untracked() {
                    if let Some(t) = touches.get(0) {
                        let (sx0, sy0) = drag_start.get_untracked();
                        let dx = t.client_x() as f64 - sx0;
                        let dy = t.client_y() as f64 - sy0;
                        let cam = camera.get_untracked();
                        let (cx, cy) = cam_start.get_untracked();
                        set_camera.set(Camera {
                            x: cx - dx / cam.zoom,
                            y: cy - dy / cam.zoom,
                            zoom: cam.zoom,
                        });
                    }
                } else if touches.length() == 1 && painting.get_untracked() {
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
        // 每格即房間：同屬性連群內淡線，外緣／異屬性分界深線（每邊只畫一次）
        if cam.zoom >= 0.22 {
            stroke_each_cell_hex_outline(&ctx, &cs, cam.zoom, min_x, max_x, min_y, max_y);
        }

        // 選取／標記／每格名稱
        for cell in cs.iter() {
            let (px, py) = coord_to_pixel(cell.coord.q, cell.coord.r);
            if px < min_x || px > max_x || py < min_y || py > max_y {
                continue;
            }
            let (ox0, oy0) = HEX_VERT_OFFSETS[0];

            if cam.zoom >= 0.35 {
                let is_sel = sel == Some(cell.coord);
                let is_multi = selected_many.get().contains(&cell.coord);
                if is_multi || is_sel {
                    ctx.begin_path();
                    ctx.move_to(px + ox0, py + oy0);
                    for (ox, oy) in HEX_VERT_OFFSETS.iter().skip(1) {
                        ctx.line_to(px + *ox, py + *oy);
                    }
                    ctx.close_path();
                    let stroke = if is_multi { "#22d3ee" } else { "#ffcc00" };
                    ctx.set_stroke_style_str(stroke);
                    ctx.set_line_width(2.0 / cam.zoom);
                    ctx.stroke();
                }
            }

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

            if cam.zoom >= 0.42 {
                ctx.set_fill_style_str("#e8eef5");
                ctx.set_font(&format!(
                    "{}px sans-serif",
                    (9.0 / cam.zoom).max(7.5)
                ));
                ctx.set_text_align("center");
                ctx.set_text_baseline("middle");
                let label = label_with_thin_space_gaps(&cell_display_label(cell));
                let _ = ctx.fill_text(&label, px, py + 2.0);
            }
        }

        ctx.restore();
    });

    view! {
        <div class="hex-grid-wrap">
            <canvas
                node_ref=canvas_ref
                style="background:#0f1923;touch-action:none;-webkit-user-select:none;user-select:none"
            >
            </canvas>
        </div>
    }
}
