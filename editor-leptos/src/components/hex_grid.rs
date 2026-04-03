use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::collections::HashMap;
use crate::types::{HexCell, HexCoord, ToolMode};

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
                ToolMode::Select | ToolMode::Move => {}
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

                if cam.zoom >= 0.35 {
                    let is_sel = sel == Some(cell.coord);
                    let is_multi = selected_many.get().contains(&cell.coord);
                    let stroke = if is_multi { "#22d3ee" } else if is_sel { "#ffcc00" } else { "#1a2634" };
                    let sw = if is_multi || is_sel { 2.0 / cam.zoom } else { 0.6 / cam.zoom };
                    ctx.set_stroke_style_str(stroke);
                    ctx.set_line_width(sw);
                    ctx.stroke();
                }

                if cam.zoom >= 0.6 {
                    ctx.set_fill_style_str("#e0e8f0");
                    ctx.set_font(&format!("{}px sans-serif", (8.0 / cam.zoom).max(7.0)));
                    ctx.set_text_align("center");
                    ctx.set_text_baseline("middle");
                    let label = if cell.name.trim().is_empty() {
                        cell.terrain.label().to_string()
                    } else {
                        cell.name.clone()
                    };
                    let _ = ctx.fill_text(&label, px, py + 2.0);
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
