//! 全地圖底板唯讀端點：`/api/substrate/{meta,basemap.png,window,cell}`。

use axum::extract::Query;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::grid::substrate::facade::CellFacade;
use crate::grid::substrate::l0_cell::{SurfaceWaterKind, landmass};
use crate::grid::substrate::render_synth::{synthesize_cell, zone_name_of};
use crate::grid::substrate::runtime::{self, AtlasLayerMode, SubstrateRuntime, WORLD_SEED};
use crate::grid::substrate::slots::{FieldState, SlotStack, derive_pristine_slots};
use crate::pg;

/// 視窗寬高上限（格）。
const WINDOW_MAX_SIDE: i64 = 96;

struct LandmarkDef {
    gx: i32,
    gy: i32,
    name: &'static str,
    badge: &'static str,
    desc: &'static str,
    zoom: u32,
}

const LANDMARKS: [LandmarkDef; 8] = [
    LandmarkDef {
        gx: 0,
        gy: 0,
        name: "西麓湧泉原點",
        badge: "(0, 0) · 原點",
        desc: "紅土台地西麓世界原點，北鄰斷層湧泉眼",
        zoom: 50,
    },
    LandmarkDef {
        gx: 0,
        gy: 1,
        name: "北側斷層湧泉眼",
        badge: "(0, 1) · 湧泉",
        desc: "斷層裂隙常年湧出清冽泉水之源頭",
        zoom: 50,
    },
    LandmarkDef {
        gx: 410,
        gy: 0,
        name: "紅土礫石台地高稜",
        badge: "(410, 0) · 高稜",
        desc: "紅土礫石台地脊線高處，視野開闊",
        zoom: 24,
    },
    LandmarkDef {
        gx: -320,
        gy: 0,
        name: "西濱沖積菅芒草海",
        badge: "(-320, 0) · 平原",
        desc: "西側沖積砂壤低地與連綿菅芒草海",
        zoom: 24,
    },
    LandmarkDef {
        gx: -602,
        gy: 0,
        name: "潮間淤泥灘與西海岸",
        badge: "(-602, 0) · 海岸",
        desc: "潮汐漲退交會之泥質潮間淺灘",
        zoom: 24,
    },
    LandmarkDef {
        gx: 895,
        gy: 0,
        name: "東界淺溪峽口",
        badge: "(895, 0) · 東界河",
        desc: "東側界溪與對岸內陸盆地分界水岸",
        zoom: 24,
    },
    LandmarkDef {
        gx: 100,
        gy: 1045,
        name: "北界深切大溪峽谷",
        badge: "(100, 1045) · 北界河",
        desc: "北側湍急深溪構成之水密自然邊界",
        zoom: 18,
    },
    LandmarkDef {
        gx: 100,
        gy: -1406,
        name: "南界寬闊大河漫灘",
        badge: "(100, -1406) · 南界河",
        desc: "南側寬闊大河構成之水密自然邊界",
        zoom: 18,
    },
];

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "available": false, "error": "底板尚未載入" })),
    )
        .into_response()
}

fn landmass_label(id: u8) -> &'static str {
    match id {
        landmass::SEA => "外海",
        landmass::DAJIA_WU_TABLELAND => "首發開放陸塊",
        landmass::TAICHUNG_BASIN => "東岸內陸盆地",
        landmass::HOULI_DAJIA_FAN => "北岸扇原台地",
        landmass::BAGUA_CHANGHUA_PLAIN => "南岸長嶺平原",
        _ => "未闢霧境",
    }
}

fn surface_water_label(w: SurfaceWaterKind) -> &'static str {
    match w {
        SurfaceWaterKind::None => "無常態地表水",
        SurfaceWaterKind::SeasonalGully => "季節乾溝",
        SurfaceWaterKind::PerennialStream => "常流山澗",
        SurfaceWaterKind::FaultSpring => "斷層湧泉眼",
        SurfaceWaterKind::DeepRapidRiver => "湍急深溪",
        SurfaceWaterKind::IntertidalShallow => "潮間淺海",
        SurfaceWaterKind::OpenSea => "外海深水",
        SurfaceWaterKind::NoData => "無資料",
    }
}

fn groundwater_label(bucket: u8) -> &'static str {
    match bucket {
        0..=3 => "地下水極淺，掘土即滲",
        4..=9 => "地下水淺藏，深掘可及",
        _ => "地下水深遠",
    }
}

fn floors_label(max_z: u8) -> &'static str {
    match max_z {
        0 => "地基不穩，無法承重起樓",
        1 => "僅宜平房",
        2 => "可起兩層樓",
        _ => "可起三層樓",
    }
}

#[derive(Deserialize)]
pub struct BasemapQuery {
    mode: Option<String>,
}

#[derive(Deserialize)]
pub struct WindowQuery {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

#[derive(Deserialize)]
pub struct CellQuery {
    x: i32,
    y: i32,
}

pub async fn substrate_meta() -> Response {
    let Some(rt) = runtime::global() else { return unavailable() };
    let r = rt.region();
    let dirty = pg::pool().map_or(0, |p| pg::cell_overlay::count_dirty_cells(&p));
    let landmarks: Vec<Value> = LANDMARKS
        .iter()
        .map(|l| {
            json!({
                "gx": l.gx,
                "gy": l.gy,
                "x": l.gx,
                "y": l.gy,
                "name": l.name,
                "badge": l.badge,
                "desc": l.desc,
                "zoom": l.zoom,
            })
        })
        .collect();
    let area_km2 = (rt.landmass_01_cells() as f64 * 0.004).round() / 10.0;
    Json(json!({
        "available": true,
        "min_gx": r.min_gx, "max_gx": r.max_gx(),
        "min_gy": r.min_gy, "max_gy": r.max_gy(),
        "width": r.width, "height": r.height,
        "cell_size_m": 20, "expansion_k": 2,
        "landmass_01_cells": rt.landmass_01_cells(),
        "landmass_01_area_km2": area_km2,
        "spring_count": rt.spring_count(),
        "overlay_dirty_count": dirty,
        "landmarks": landmarks,
    }))
    .into_response()
}

pub async fn substrate_basemap(Query(q): Query<BasemapQuery>) -> Response {
    let Some(rt) = runtime::global() else { return unavailable() };
    let mode = q
        .mode
        .as_deref()
        .map_or(Some(AtlasLayerMode::Terrain), AtlasLayerMode::parse);
    let Some(mode) = mode else {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "未知圖層模式" }))).into_response();
    };
    let png = tokio::task::spawn_blocking(move || rt.basemap_png(mode)).await;
    match png {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "image/png"),
                (header::CACHE_CONTROL, "public, max-age=300"),
            ],
            bytes.as_ref().clone(),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

fn clamp_span(lo: i32, hi: i32) -> (i32, i32) {
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    let hi = (i64::from(lo) + WINDOW_MAX_SIDE - 1).min(i64::from(hi)) as i32;
    (lo, hi)
}

fn cue_text(rt: &SubstrateRuntime, gx: i32, gy: i32) -> Option<String> {
    rt.edge_cue(gx, gy).map(|c| c.compact_badge())
}

pub async fn substrate_window(Query(q): Query<WindowQuery>) -> Response {
    let Some(rt) = runtime::global() else { return unavailable() };
    let (min_x, max_x) = clamp_span(q.min_x, q.max_x);
    let (min_y, max_y) = clamp_span(q.min_y, q.max_y);
    let overlays: std::collections::HashMap<(i32, i32), SlotStack> = pg::pool()
        .map(|p| pg::cell_overlay::load_window_overlays(&p, min_x, max_x, min_y, max_y))
        .unwrap_or_default()
        .into_iter()
        .collect();
    let mut cells = Vec::new();
    for y in (min_y..=max_y).rev() {
        for x in min_x..=max_x {
            let Some(l0) = rt.l0(x, y) else {
                cells.push(json!({ "x": x, "y": y, "sym": "≋", "name": "外海深水",
                    "elev_m": null, "landmass_id": 0, "walkable": false, "cue": null }));
                continue;
            };
            let slots = overlays
                .get(&(x, y))
                .cloned()
                .unwrap_or_else(|| derive_pristine_slots(x, y, l0, WORLD_SEED));
            let slope = rt.slope(x, y);
            let cue = rt.edge_cue(x, y);
            let cue_s = cue.as_ref().map(|c| c.compact_badge());
            let v = synthesize_cell(x, y, l0, slope, cue, &slots, FieldState::default());
            cells.push(json!({
                "x": x, "y": y, "sym": v.symbol, "name": v.micro_name,
                "elev_m": l0.elevation_m(), "landmass_id": l0.landmass_id,
                "walkable": v.walkable_on_landmass_01, "cue": cue_s,
            }));
        }
    }
    Json(json!({ "min_x": min_x, "max_x": max_x, "min_y": min_y, "max_y": max_y, "cells": cells }))
        .into_response()
}

pub async fn substrate_cell(Query(q): Query<CellQuery>) -> Response {
    let Some(rt) = runtime::global() else { return unavailable() };
    let (x, y) = (q.x, q.y);
    if rt.l0(x, y).is_none() {
        return Json(json!({
            "found": true, "x": x, "y": y,
            "landmass_id": landmass::SEA,
            "landmass_label": landmass_label(landmass::SEA),
            "landmass_open": false,
            "is_open_landmass": false,
            "elev_m": 0.0,
            "slope_pct": 0.0,
            "zone_name": "外海深水",
            "micro_name": "外海深水",
            "symbol": "≋",
            "walkable": false,
            "edge_cue": Value::Null,
            "material_code": 0,
            "material_label": "外海水域",
            "max_floors_label": "外海水域，無法築造",
            "surface_water_label": "外海深水",
            "groundwater_label": "海水浸沒",
            "base_energy": 80,
            "rain_cover": false,
            "rain_cover_label": "露天無遮",
            "max_z": 0,
            "step_ms": 280,
            "overlay": Value::Null,
            "slots": {
                "u2": "無深樁",
                "u1": "外海床",
                "s0_pave": "深海波濤",
                "s0_companions": Vec::<&str>::new(),
                "v1": "海天無遮",
            },
            "terrain_line": "蒼茫外海波濤起伏，深水不可涉足。",
            "canvas_cards": Vec::<Value>::new(),
        }))
        .into_response();
    }
    let pool = pg::pool();
    let snap = CellFacade::snapshot_live(pool.as_deref(), x, y, FieldState::default());
    let l0 = snap.l0;
    let slots = &snap.slots;
    let slope = snap.slope;
    let v = &snap.synth;
    let walk = v.walkable_on_landmass_01;
    let overlay_meta = snap.overlay.as_ref().map(|o| {
        json!({
            "building_id": o.building_id,
            "roll_day": o.roll_day,
            "weather_epoch": o.weather_epoch,
            "charge": o.charge,
            "stamp": o.stamp,
            "ver": o.ver,
        })
    });
    let cards: Vec<Value> = v
        .canvas_cards
        .iter()
        .map(|c| json!({ "title": c.title, "hint": c.hint }))
        .collect();
    let comps: Vec<&str> = slots.s0_companions.iter().map(|c| c.label_zh()).collect();
    let pile = match slots.u2 {
        crate::grid::substrate::slots::U2Plugin::None => "無深樁",
        crate::grid::substrate::slots::U2Plugin::DeepPile { .. } => "深樁",
    };
    let is_open = l0.landmass_id == landmass::DAJIA_WU_TABLELAND;
    let rain_cover = slots.v1.provides_rain_cover();
    let rain_cover_label = if rain_cover {
        "上空有遮蔽（可避雨）"
    } else {
        "露天無遮"
    };
    Json(json!({
        "found": true, "x": x, "y": y,
        "landmass_id": l0.landmass_id,
        "landmass_label": landmass_label(l0.landmass_id),
        "landmass_open": is_open,
        "is_open_landmass": is_open,
        "elev_m": l0.elevation_m(),
        "slope_pct": slope.map(|s| (s.grade_ratio * 100.0).round()),
        "zone_name": zone_name_of(x, l0),
        "micro_name": v.micro_name,
        "symbol": v.symbol,
        "walkable": walk,
        "edge_cue": cue_text(rt, x, y),
        "material_code": l0.substrate_material,
        "material_label": l0.material().name_zh(),
        "max_floors_label": floors_label(SlotStack::max_supported_z(l0)),
        "surface_water_label": surface_water_label(l0.surface_water()),
        "groundwater_label": groundwater_label(l0.groundwater_bucket()),
        "base_energy": l0.base_energy,
        "rain_cover": rain_cover,
        "rain_cover_label": rain_cover_label,
        "max_z": snap.max_z,
        "step_ms": snap.step_ms,
        "overlay": overlay_meta,
        "slots": {
            "u2": pile,
            "u1": slots.u1.label_zh(),
            "s0_pave": slots.s0_pave.label_zh(),
            "s0_companions": comps,
            "v1": slots.v1.label_zh(),
        },
        "terrain_line": v.terrain_line,
        "canvas_cards": cards,
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_span_is_clamped() {
        assert_eq!(clamp_span(0, 1000), (0, 95));
        assert_eq!(clamp_span(10, 5), (5, 10));
        assert_eq!(clamp_span(-3, 3), (-3, 3));
    }

    #[test]
    fn labels_avoid_real_world_names() {
        let banned = ["台灣", "臺灣", "台中", "沙鹿", "烏溪", "大甲", "筏子", "彰化", "八卦", "晶片"];
        for id in 0..=6_u8 {
            let s = landmass_label(id);
            assert!(!banned.iter().any(|b| s.contains(b)));
        }
        for l in &LANDMARKS {
            for field in [l.name, l.badge, l.desc] {
                assert!(!banned.iter().any(|b| field.contains(b)));
            }
        }
    }
}
