use crate::store;
use axum::{Json, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
struct EcoMapCell {
    x: i32,
    y: i32,
    t: String,
    z: String,
    n: String,
    s: String,
    o: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    d: String,
}

#[derive(Serialize)]
struct EcoEntity {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    title: String,
    x: i32,
    y: i32,
    kind: String,
    activity: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    breath_habit: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    held_vessel: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    afflictions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lineage: Vec<String>,
}

#[derive(Serialize)]
struct EcoSettlementPulse {
    name: String,
    scarcest_item: String,
    scarcest_price: i32,
    summary: String,
}

#[derive(Serialize)]
struct EcoHuntingZonePulse {
    name: String,
    apex_omen: String,
    word_source: String,
}

#[derive(Serialize)]
struct EcoPulseResponse {
    explicit_cells: usize,
    settlements: Vec<EcoSettlementPulse>,
    hunting_zones: Vec<EcoHuntingZonePulse>,
}

trait TerrainExt {
    fn as_str(&self) -> &'static str;
}

impl TerrainExt for crate::grid::Terrain {
    fn as_str(&self) -> &'static str {
        self.kind_str()
    }
}

trait CellExt {
    fn symbol(&self) -> &'static str;
}

impl CellExt for crate::grid::GridCell {
    fn symbol(&self) -> &'static str {
        self.terrain.symbol()
    }
}

fn sanitize_eco_activity(act: &str) -> String {
    let mut result = String::with_capacity(act.len());
    let mut chars = act.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch == '（' || ch == '(' {
            let closer = if ch == '（' { '）' } else { ')' };
            let rest = &act[idx..];
            if let Some(close_offset) = rest.find(closer) {
                let inside = &rest[..close_offset + closer.len_utf8()];
                if inside.contains("側 · ")
                    || inside.contains("側・")
                    || inside.contains("WS-")
                    || inside.contains("日常劇本")
                {
                    let target_pos = idx + inside.len();
                    while let Some(&(next_idx, _)) = chars.peek() {
                        if next_idx < target_pos {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    continue;
                }
            }
        }
        result.push(ch);
    }
    result
}

fn extract_primary_slot_id(equipment_slots: &str) -> Option<String> {
    if equipment_slots.trim().is_empty() {
        return None;
    }
    let Ok(serde_json::Value::Object(slot_map)) =
        serde_json::from_str::<serde_json::Value>(equipment_slots)
    else {
        return None;
    };
    ["hold_r", "weapon", "hand", "tool", "hold_l", "主手", "工具", "武器", "副手"]
        .iter()
        .find_map(|k| slot_map.get(*k).and_then(|v| v.as_str()))
        .filter(|s| !s.is_empty() && *s != "starter_blade")
        .map(|s| s.to_string())
}

fn infer_vocational_vessel(role_hint: &str) -> Option<(&'static str, &'static str)> {
    if role_hint.contains("鐵") || role_hint.contains("鍛") || role_hint.contains("爐") || role_hint.contains("鑄") {
        Some(("craft_iron_pickaxe", "百鍛長柄鐵錘"))
    } else if role_hint.contains("木") || role_hint.contains("竹") || role_hint.contains("鋸") || role_hint.contains("船") {
        Some(("craft_wooden_tool_handle", "魯班平推刨刀"))
    } else if role_hint.contains("藥") || role_hint.contains("醫") || role_hint.contains("草") || role_hint.contains("丹") {
        Some(("craft_meridian_salve", "青銅搗藥杵"))
    } else if role_hint.contains("巡") || role_hint.contains("衛") || role_hint.contains("哨") || role_hint.contains("獵") || role_hint.contains("武") {
        Some(("craft_steel_hunting_knife", "精鍛防身獵刀"))
    } else if role_hint.contains("礦") || role_hint.contains("石") || role_hint.contains("坑") {
        Some(("craft_iron_pickaxe", "雙刃碎石手鎬"))
    } else if role_hint.contains("織") || role_hint.contains("布") || role_hint.contains("服") || role_hint.contains("染") {
        Some(("craft_steel_hunting_knife", "熟鐵裁布大剪刀"))
    } else if role_hint.contains("廚") || role_hint.contains("茶") || role_hint.contains("膳") || role_hint.contains("食") || role_hint.contains("攤") {
        Some(("craft_steel_hunting_knife", "厚背熟鐵廚刀"))
    } else if role_hint.contains("書") || role_hint.contains("院") || role_hint.contains("講") || role_hint.contains("印") || role_hint.contains("記") {
        Some(("craft_peninsula_almanac", "白塔風物考工手札"))
    } else if role_hint.contains("漁") || role_hint.contains("港") || role_hint.contains("渡") || role_hint.contains("海") {
        Some(("craft_pine_torch", "浸油麻編海釣竿"))
    } else {
        None
    }
}

fn format_held_vessel_outside_store(
    entity_id: &str,
    kind: &str,
    soul_seed: Option<i64>,
    role_hint: &str,
    primary_id: Option<&str>,
    store_item_name: Option<String>,
) -> String {
    let (item_id, resolved_override_name) = if let Some(iid) = primary_id {
        (iid, None)
    } else if let Some((vid, vname)) = infer_vocational_vessel(role_hint) {
        (vid, Some(vname))
    } else {
        return String::new();
    };
    let item_name = if let Some(ov) = resolved_override_name {
        ov.to_string()
    } else if let Some(nm) = store_item_name {
        nm
    } else if let Some(mat) = crate::world::materials::material_by_item_id(item_id) {
        mat.name.clone()
    } else if let Some(name) = crate::db::inventory::fallback_item_name(item_id) {
        name.to_string()
    } else if let Some(ecol) = crate::game::object_ecology::lookup_ecology_item_def(item_id) {
        ecol.name
    } else {
        item_id.to_string()
    };
    let mut vstate = crate::pg::living_world::get_vessel_state(entity_id, item_id);
    if vstate.use_count == 0 && kind == "npc" {
        vstate.use_count = 4 + ((soul_seed.unwrap_or(0).unsigned_abs() % 18) as i32);
    }
    format!("{item_name}（{}）", vstate.status_badge())
}

/// 取得全城地圖數據（柵格化）
pub async fn eco_map() -> impl IntoResponse {
    if let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() {
        let all_drops = crate::pg::living_world::snapshot_all_ground_dropped_items();
        let cells: Vec<EcoMapCell> = grid
            .cells()
            .map(|cell| {
                let mut objs: Vec<String> =
                    cell.objects.iter().map(|obj| obj.name.clone()).collect();
                if let Some(drops) = all_drops.get(&(cell.coord.x, cell.coord.y)) {
                    for (_id, drop_name, qty) in drops {
                        objs.push(format!("地面遺落：{drop_name} ×{qty}"));
                    }
                }
                EcoMapCell {
                    x: cell.coord.x,
                    y: cell.coord.y,
                    t: cell.terrain.as_str().to_string(),
                    z: cell.zone.clone(),
                    n: cell.name.clone(),
                    s: cell.symbol().to_string(),
                    o: objs,
                    d: cell.description.clone(),
                }
            })
            .collect();
        return Json(cells).into_response();
    }

    let pool = match store::get_db_pool() {
        Some(p) => p,
        None => return Json(Vec::<EcoMapCell>::new()).into_response(),
    };

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return Json(Vec::<EcoMapCell>::new()).into_response(),
    };

    let cells: Vec<EcoMapCell> = if let Ok(rows) = conn.query(
        "SELECT x, y, terrain, zone, name, symbol FROM grid_cells",
        &[],
    ) {
        rows.iter()
            .map(|r| EcoMapCell {
                x: r.get(0),
                y: r.get(1),
                t: r.get(2),
                z: r.get(3),
                n: r.get(4),
                s: r.get(5),
                o: Vec::new(),
                d: String::new(),
            })
            .collect()
    } else {
        let rows = conn
            .query("SELECT x, y, terrain, zone, name FROM grid_cells", &[])
            .unwrap_or_default();
        rows.iter()
            .map(|r| {
                let x: i32 = r.get(0);
                let y: i32 = r.get(1);
                let t: String = r.get(2);
                let z: String = r.get(3);
                let n: String = r.get(4);
                let s = crate::grid::Terrain::from_kind_str(&t)
                    .map(|tr| tr.symbol().to_string())
                    .unwrap_or_else(|| "·".to_string());
                EcoMapCell {
                    x,
                    y,
                    t,
                    z,
                    n,
                    s,
                    o: Vec::new(),
                    d: String::new(),
                }
            })
            .collect()
    };

    Json(cells).into_response()
}

/// 取得所有實體的即時位置與活體狀態（內息底色、手持器物磨合器況、部位傷勢、師承烙印、山海生靈）
pub async fn eco_entities() -> impl IntoResponse {
    let Some(st_arc) = store::get_store() else {
        return Json(Vec::<EcoEntity>::new()).into_response();
    };

    let raw_rows: Vec<(store::Entity, i32, i32, Option<String>, Option<String>)> = {
        let st = st_arc.read();
        st.entities
            .values()
            .filter_map(|e| {
                if e.vit <= 0 {
                    return None;
                }
                let (x, y) = st.effective_entity_grid(e)?;
                let primary_id = extract_primary_slot_id(&e.equipment_slots);
                let store_item_name = primary_id
                    .as_deref()
                    .and_then(|iid| st.get_item(iid).map(|it| it.name.clone()));
                Some((e.clone(), x, y, primary_id, store_item_name))
            })
            .collect()
    };

    let live_acts: std::collections::HashMap<String, String> = {
        let live_states = store::LIVE_NPC_GRID_STATES.read();
        live_states
            .iter()
            .map(|(k, v)| (k.clone(), v.current_activity()))
            .collect()
    };

    let mut entities: Vec<EcoEntity> = Vec::with_capacity(raw_rows.len() + 60);

    for (e, x, y, primary_id, store_item_name) in raw_rows {
        let (name, title) = if e.kind == "npc" {
            let non_empty_title = if e.display_title.trim().is_empty() {
                "住民"
            } else {
                e.display_title.as_str()
            };
            let non_empty_gender = if e.gender.trim().is_empty() {
                "M"
            } else {
                e.gender.as_str()
            };
            let person =
                store::resolve_npc_person_name(&e.id, non_empty_title, Some(non_empty_gender));
            let role_raw = store::npc_persistent_role_label(&e.id, non_empty_title);
            let role_clean = role_raw
                .split(['|', '｜'])
                .next()
                .unwrap_or(&role_raw)
                .trim()
                .to_string();
            let final_title = if role_clean != person {
                role_clean
            } else {
                String::new()
            };
            (person, final_title)
        } else {
            let nm = if e.display_title.trim().is_empty() {
                e.id.clone()
            } else {
                e.display_title.clone()
            };
            (nm, "旅人".to_string())
        };

        let role_for_breath = format!("{} {}", title, e.display_title);
        let held_vessel = format_held_vessel_outside_store(
            &e.id,
            &e.kind,
            e.soul_seed,
            &role_for_breath,
            primary_id.as_deref(),
            store_item_name,
        );

        let raw_act = if let Some(act) = live_acts.get(&e.id) {
            act.clone()
        } else {
            e.current_activity.clone()
        };
        let activity = sanitize_eco_activity(&raw_act);
        let breath_habit = crate::pg::living_world::get_or_derive_breath_habit(
            &e.id,
            &role_for_breath,
            e.soul_seed.unwrap_or(0),
        )
        .dominant
        .label()
        .to_string();

        let afflictions: Vec<String> = crate::pg::living_world::get_entity_afflictions(&e.id)
            .into_iter()
            .map(|a| a.label().to_string())
            .collect();

        let lineage: Vec<String> = crate::pg::living_world::get_entity_lineage_imprints(&e.id)
            .into_iter()
            .map(|l| l.imprint_title)
            .collect();

        entities.push(EcoEntity {
            id: e.id,
            name,
            title,
            x,
            y,
            kind: e.kind,
            activity,
            breath_habit,
            held_vessel,
            afflictions,
            lineage,
        });
    }

    // 補入 19 處獵場的 1,805 隻《台灣山海經》全島自然棲地生靈（消除同心圓，按真實疆域進行藍噪聲 Poisson-disc 與獸群聚落分布）
    let time_tick = (crate::game::now_unix() / 180) as i64;
    for (z_idx, &(zone_name, _, _)) in crate::game::CANONICAL_HUNTING_ZONES.iter().enumerate() {
        let slots = crate::game::live_zone_shanhai_slots(z_idx, time_tick);
        for sl in slots {
            if let Some(amb) = crate::game::ensure_ambient_npc_at_grid(sl.x, sl.y, sl.tier_tag) {
                let held_vessel = if let Some((_lid, loot_name, shanhai_note)) =
                    crate::game::ambient_creature_loot(&amb.display_title)
                {
                    if sl.tier == 1 {
                        format!("尋跡顯形可剝取【{loot_name}】｜{shanhai_note}")
                    } else {
                        format!("可剝取【{loot_name}】｜{shanhai_note}")
                    }
                } else {
                    String::new()
                };
                let id_str = if sl.tier == 1 {
                    format!("apex_{z_idx}_{}_{}", sl.x, sl.y)
                } else {
                    format!("shanhai_{z_idx}_{}_{}_{}", sl.slot_idx, sl.x, sl.y)
                };
                let act_str = if sl.tier == 1 {
                    format!(
                        "{}{}（平時藏跡於{}深處，須查探【新鮮霸主獸跡】方會破霧顯形）",
                        sl.act_prefix, amb.current_activity, sl.sub_geo
                    )
                } else {
                    format!("{}{}", sl.act_prefix, amb.current_activity)
                };
                entities.push(EcoEntity {
                    id: id_str,
                    name: amb.display_title,
                    title: format!("{zone_name}・{}（{}）", sl.tier_label, sl.sub_geo),
                    x: sl.x,
                    y: sl.y,
                    kind: sl.kind_str.to_string(),
                    activity: act_str,
                    breath_habit: "山林長息".to_string(),
                    held_vessel,
                    afflictions: Vec::new(),
                    lineage: Vec::new(),
                });
            }
        }
    }

    Json(entities).into_response()
}

/// 取得全島 15 聚落即時供需行情與 19 獵場山海異兆總覽
pub async fn eco_pulse() -> impl IntoResponse {
    let explicit_cells = crate::server::grid_manager::get_runtime_square_grid()
        .map(|g| g.len())
        .unwrap_or(14_424);

    const SETTLEMENT_NAMES: &[&str] = &[
        "文曜",
        "風舶角",
        "風舶嶼",
        "大圳頭",
        "半嶺鎮",
        "南哨",
        "老堰頭",
        "北嶺腳",
        "霜林口",
        "風櫃尾",
        "烏石鼻",
        "牛埔",
        "紅土崁",
        "三號坑口",
        "水尾寮",
    ];
    let settlements: Vec<EcoSettlementPulse> = SETTLEMENT_NAMES
        .iter()
        .map(|&name| {
            let (scarcest_item, scarcest_price) =
                crate::game::object_ecology::scarcest_item_for_settlement(name)
                    .map(|(_id, nm, _cur, _max, pr)| (nm, pr))
                    .unwrap_or_else(|| ("回收熟鐵料塊".to_string(), 12));
            let summary =
                crate::game::object_ecology::format_settlement_supply_demand_report(name);
            EcoSettlementPulse {
                name: name.to_string(),
                scarcest_item,
                scarcest_price,
                summary,
            }
        })
        .collect();

    const HUNTING_NAMES: &[&str] = &[
        "長風草甸",
        "古河巨木林",
        "赤石坡",
        "落日岬",
        "驚濤岩岸",
        "巨藻淺灣",
        "黑石灣",
        "西麓林道",
        "清泉溪谷",
        "翠冠巨木林",
        "北坡雲杉林",
        "淺澤泥口",
        "白茫洲",
        "貼山深潭",
        "青籠關",
        "西嶺礦谷",
        "東海峭壁",
        "雲腰雪林",
        "白塔雪巔",
    ];
    let hunting_zones: Vec<EcoHuntingZonePulse> = HUNTING_NAMES
        .iter()
        .map(|&name| {
            let apex_omen = crate::game::hunting_zone_apex_omen(name)
                .unwrap_or_default()
                .to_string();
            let word_source = crate::game::zone_ambient_word_source(name)
                .map(|(_ws, desc)| desc.to_string())
                .unwrap_or_default();
            EcoHuntingZonePulse {
                name: name.to_string(),
                apex_omen,
                word_source,
            }
        })
        .collect();

    Json(EcoPulseResponse {
        explicit_cells,
        settlements,
        hunting_zones,
    })
    .into_response()
}
