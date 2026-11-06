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
    thought: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    magnesium: Option<i32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    hunting_party: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    prey_target: Option<[i32; 2]>,
    #[serde(skip_serializing_if = "String::is_empty")]
    breath_habit: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    held_vessel: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    afflictions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lineage: Vec<String>,
}

/// 住民因果思緒與決策選項條目（對應 /api/eco/npc-thoughts）
#[derive(Serialize)]
pub struct NpcThoughtEntry {
    pub id: String,
    pub name: String,
    pub title: String,
    pub settlement: String,
    pub activity: String,
    pub thought: String,
    pub magnesium: i32,
    pub decision_image_code: String,
    pub eligible_option_count: u8,
    pub chosen_action_code: String,
    pub active_need: String,
    pub image_decision: String,
    pub situated_action: String,
    pub carryover_note: String,
    pub stress: u8,
    pub time_scale_mark: String,
    pub belief_summary: String,
    pub latest_ripple: Option<crate::npc::causal_core::WorldRippleFact>,
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
    pub active_ripples_count: usize,
    pub active_ripples: Vec<crate::npc::causal_core::WorldRippleFact>,
    pub stranded_cargo_orders: Vec<crate::npc::causal_core::StrandedCargoOrder>,
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
    let normalized = act
        .replace("買米菜回屋自炊熱粥", "採買米菜自炊熱粥")
        .replace("切滷肉包熱點心帶回家", "切份滷肉包熱點心解饞")
        .replace("舀大水缸冷開水喝", "取用清冽涼水一飲解渴")
        .replace("掬山泉涼水飲用解渴", "取用清冽涼水一飲解渴")
        .replace("空手去近郊拾柴採野草換鎂幣", "空手拾柴採野草換取鎂幣")
        .replace("靠著案頭木架閉目喘口氣", "尋處陰涼角落閉目歇腳喘口氣");
    let mut result = String::with_capacity(normalized.len());
    let mut chars = normalized.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch == '（' || ch == '(' {
            let closer = if ch == '（' { '）' } else { ')' };
            let rest = &normalized[idx..];
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
    if role_hint.trim().is_empty() {
        return None;
    }
    let spec = crate::npc::causal_core::resolve_vocational_vessel(role_hint);
    if let Some(cat) = crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS
        .iter()
        .find(|o| o.id == spec.catalog_id)
    {
        Some((cat.id, cat.name))
    } else {
        Some((spec.catalog_id, spec.display_name))
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

fn derive_eco_inner_thought(
    e: &store::Entity,
    title: &str,
    activity: &str,
    hunting_party: &str,
    effective_mg: i32,
) -> String {
    if let Some(last) = e.thoughts.last()
        && let Some(flav) = last.flavor.as_deref()
    {
        let cleaned = sanitize_eco_activity(flav.trim());
        if !cleaned.is_empty() {
            return cleaned;
        }
    }
    let seed = e.soul_seed.unwrap_or(0).unsigned_abs();
    let role = format!("{} {}", title, e.display_title);
    let settlement = title.split('・').next().unwrap_or("文曜");
    crate::npc::causal_core::render_somatic_10layer_thought(
        &e.id,
        &role,
        settlement,
        seed,
        activity,
        "",
        "山海長息",
        &[],
        hunting_party,
        effective_mg,
    )
}

/// 取得全島住民因果思緒與決策選項明細（/api/eco/npc-thoughts）
pub async fn handle_eco_npc_thoughts() -> impl IntoResponse {
    let Some(st_arc) = store::get_store() else {
        return Json(Vec::<NpcThoughtEntry>::new()).into_response();
    };
    crate::npc::causal_core::ensure_circular_economy_normalized_if_needed();
    let raw_rows: Vec<(store::Entity, Option<String>, Option<String>)> = {
        let st = st_arc.read();
        st.entities
            .values()
            .filter_map(|e| {
                if e.kind != "npc" || e.vit <= 0 {
                    return None;
                }
                let primary_id = extract_primary_slot_id(&e.equipment_slots);
                let store_item_name = primary_id
                    .as_deref()
                    .and_then(|iid| st.get_item(iid).map(|it| it.name.clone()));
                Some((e.clone(), primary_id, store_item_name))
            })
            .collect()
    };

    let live_acts: std::collections::HashMap<String, (String, String)> = {
        let live_states = store::LIVE_NPC_GRID_STATES.read();
        live_states
            .iter()
            .map(|(k, v)| (k.clone(), (v.current_activity(), v.dest_name.clone())))
            .collect()
    };

    let mut thoughts: Vec<NpcThoughtEntry> = Vec::with_capacity(raw_rows.len());

    for (e, _primary_id, _store_item_name) in raw_rows {
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
        let person = store::resolve_npc_person_name(&e.id, non_empty_title, Some(non_empty_gender));
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

        let slot = store::wenyao_household_slot_for(&e.id);
        let role = if !e.id.starts_with("npc_") && !e.display_title.trim().is_empty() {
            e.display_title.clone()
        } else {
            format!("{} {}", slot.brief_tag, e.display_title)
        };
        let settlement = store::resident_macro_settlement(&e.id, &e.display_title)
            .unwrap_or_else(|| store::normalize_household_home_settlement(slot.home_name))
            .to_string();

        let (raw_act, live_dest_name) = if let Some((act, dname)) = live_acts.get(&e.id) {
            (act.clone(), dname.clone())
        } else {
            (e.current_activity.clone(), slot.work_name.to_string())
        };
        let res_seed = crate::npc::causal_core::deterministic_resident_seed(&e.id, e.soul_seed);
        let eff_mg = e.magnesium;
        let mut cstate = crate::npc::causal_core::get_or_init_causal_state(
            &e.id,
            &role,
            &settlement,
            res_seed,
        );
        crate::npc::causal_core::sync_causal_state_with_magnesium(&mut cstate, eff_mg, res_seed);
        let cleaned_raw_owned = raw_act
            .trim()
            .replace("買米菜回屋自炊熱粥", "採買米菜自炊熱粥")
            .replace("切滷肉包熱點心帶回家", "切份滷肉包熱點心解饞")
            .replace("舀大水缸冷開水喝", "取用清冽涼水一飲解渴")
            .replace("掬山泉涼水飲用解渴", "取用清冽涼水一飲解渴")
            .replace("空手去近郊拾柴採野草換鎂幣", "空手拾柴採野草換取鎂幣")
            .replace("靠著案頭木架閉目喘口氣", "尋處陰涼角落閉目歇腳喘口氣");
        let cleaned_raw = cleaned_raw_owned.as_str();
        let is_moving = cleaned_raw.starts_with("正動身前往")
            || cleaned_raw.starts_with("正踏著夜色返回")
            || cleaned_raw.starts_with("正返回【")
            || cleaned_raw.starts_with("收工之餘，正動身前往")
            || cleaned_raw.contains("（推進目標：【");
        let chosen_opt = crate::npc::causal_core::lookup_action_option(&cstate.chosen_action_code);
        let matches_chosen = chosen_opt.is_some_and(|opt| {
            if is_moving {
                let is_sleep_rest = opt.action_title.contains("安歇")
                    || opt.action_title.contains("睡")
                    || opt.action_title.contains("歇息")
                    || opt.action_title.contains("歇宿")
                    || opt.action_title.contains("返家")
                    || opt.action_title.contains("回屋舍")
                    || opt.action_title.contains("回居所");
                if is_sleep_rest
                    && (cleaned_raw.contains("準備安歇") || cleaned_raw.contains("稍作歇息"))
                {
                    return true;
                }
                let opt_title_norm = opt
                    .action_title
                    .replace("買米菜回屋自炊熱粥", "採買米菜自炊熱粥")
                    .replace("切滷肉包熱點心帶回家", "切份滷肉包熱點心解饞")
                    .replace("舀大水缸冷開水喝", "取用清冽涼水一飲解渴")
                    .replace("掬山泉涼水飲用解渴", "取用清冽涼水一飲解渴")
                    .replace("空手去近郊拾柴採野草換鎂幣", "空手拾柴採野草換取鎂幣")
                    .replace("靠著案頭木架閉目喘口氣", "尋處陰涼角落閉目歇腳喘口氣");
                if cleaned_raw.contains(&opt_title_norm) {
                    return true;
                }
                let prefixes = [
                    "回屋舍", "回居所", "在攤前", "在旅社", "在茶屋", "在貨站", "在案頭", "去", "到", "在",
                ];
                let mut trimmed_title = opt_title_norm.as_str();
                let mut changed = true;
                while changed {
                    changed = false;
                    for p in prefixes {
                        if let Some(rest) = trimmed_title.strip_prefix(p) {
                            trimmed_title = rest.trim_start();
                            changed = true;
                            break;
                        }
                    }
                }
                !trimmed_title.is_empty() && cleaned_raw.contains(trimmed_title)
            } else {
                cleaned_raw.contains(opt.action_behavior)
            }
        });
        let is_legacy_or_mismatched = cleaned_raw.is_empty()
            || !matches_chosen
            || cleaned_raw.contains("（【")
            || cleaned_raw.contains("——")
            || cleaned_raw.contains("項符合條件選項中擇");
        let activity = if is_legacy_or_mismatched {
            if is_moving && let Some(opt) = chosen_opt {
                let place = crate::npc::causal_core::coherent_place_for_venue_req(
                    opt.venue_req,
                    &live_dest_name,
                );
                let dest_label = format!("{settlement}・{place}");
                let title_norm = opt
                    .action_title
                    .replace("買米菜回屋自炊熱粥", "採買米菜自炊熱粥")
                    .replace("切滷肉包熱點心帶回家", "切份滷肉包熱點心解饞")
                    .replace("舀大水缸冷開水喝", "取用清冽涼水一飲解渴")
                    .replace("掬山泉涼水飲用解渴", "取用清冽涼水一飲解渴")
                    .replace("空手去近郊拾柴採野草換鎂幣", "空手拾柴採野草換取鎂幣")
                    .replace("靠著案頭木架閉目喘口氣", "尋處陰涼角落閉目歇腳喘口氣");
                let title = title_norm.as_str();
                let is_sleep_rest = title.contains("安歇")
                    || title.contains("睡")
                    || title.contains("歇息")
                    || title.contains("歇宿")
                    || title.contains("返家")
                    || title.contains("回屋舍")
                    || title.contains("回居所");
                if is_sleep_rest {
                    let hour = crate::store::current_game_hour();
                    if hour >= 20 || hour < 5 {
                        format!("正踏著夜色返回【{dest_label}】準備安歇")
                    } else {
                        format!("正返回【{dest_label}】稍作歇息")
                    }
                } else {
                    let prefixes = [
                        "回屋舍", "回居所", "在攤前", "在旅社", "在茶屋", "在貨站", "在案頭", "去", "到", "在",
                    ];
                    let mut trimmed = title;
                    let mut changed = true;
                    while changed {
                        changed = false;
                        for p in prefixes {
                            if let Some(rest) = trimmed.strip_prefix(p) {
                                trimmed = rest.trim_start();
                                changed = true;
                                break;
                            }
                        }
                    }
                    if trimmed.is_empty() {
                        format!("正動身前往【{dest_label}】")
                    } else {
                        format!("正動身前往【{dest_label}】，準備{trimmed}")
                    }
                }
            } else {
                crate::npc::causal_core::format_predefined_stay_activity(
                    &live_dest_name,
                    &cstate,
                    eff_mg,
                )
            }
        } else {
            sanitize_eco_activity(cleaned_raw)
        };
        let thought = derive_eco_inner_thought(
            &e,
            &final_title,
            &activity,
            "",
            eff_mg,
        );
        let now_unix = crate::game::now_unix();
        let mark = crate::npc::causal_core::format_time_scale_mark(&cstate, now_unix);
        let latest_ripple = crate::npc::causal_core::top_active_ripple_for_npc(&e.id, now_unix);
        thoughts.push(NpcThoughtEntry {
            id: e.id,
            name: person,
            title: final_title,
            settlement,
            activity,
            thought,
            magnesium: eff_mg,
            decision_image_code: cstate.decision_image_code.clone(),
            eligible_option_count: cstate.eligible_option_count,
            chosen_action_code: cstate.chosen_action_code.clone(),
            active_need: cstate.active_need,
            image_decision: cstate.image_decision,
            situated_action: cstate.situated_action,
            carryover_note: cstate.carryover_from_yesterday,
            stress: cstate.stress,
            time_scale_mark: mark,
            belief_summary: cstate.belief_summary,
            latest_ripple,
        });
    }

    Json(thoughts).into_response()
}

/// 取得全島住民因果思緒（別名相容 /api/eco/npc-thoughts）
pub async fn eco_npc_thoughts() -> impl IntoResponse {
    handle_eco_npc_thoughts().await
}

/// 取得所有實體的即時位置與活體狀態（別名相容 handle_eco_entities）
pub async fn handle_eco_entities() -> impl IntoResponse {
    eco_entities().await
}

/// 取得全城地圖資料（柵格化）
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
    crate::npc::causal_core::ensure_circular_economy_normalized_if_needed();
    if store::LIVE_HUNTING_PARTIES.read().is_empty()
        && let Some(grid) = crate::server::grid_manager::get_runtime_square_grid()
    {
        st_arc.read().sync_and_advance_hunting_parties(&grid);
    }

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

    let party_lookup: std::collections::HashMap<String, (String, Option<[i32; 2]>)> = {
        let parties = store::LIVE_HUNTING_PARTIES.read();
        let mut map = std::collections::HashMap::new();
        for p in parties.values() {
            let (badge, target) = match p.phase {
                store::HuntingPartyPhase::Rallying => (
                    format!(
                        "{}合社獵團｜領隊 {}・吹角集結（{}人）",
                        p.zone_name,
                        p.leader_name,
                        p.member_ids.len()
                    ),
                    None,
                ),
                store::HuntingPartyPhase::SurroundingPrey => {
                    let (px, py) = p.prey_coord;
                    let prey_place = crate::game::CANONICAL_HUNTING_ZONES
                        .get(crate::game::nearest_hunting_zone_idx(px, py))
                        .map(|z| z.0.to_string())
                        .unwrap_or_else(|| {
                            let (z, s, _) = crate::grid::r#gen::classify_wilderness_cell(px, py);
                            format!("{z}・{s}")
                        });
                    (
                        format!(
                            "{}合社獵團｜四面合圍【{}】於【{prey_place}】（{}人）",
                            p.zone_name,
                            p.prey_name,
                            p.member_ids.len()
                        ),
                        Some([p.prey_coord.0, p.prey_coord.1]),
                    )
                }
                store::HuntingPartyPhase::TakedownAndSplit => (
                    format!(
                        "{}合社獵團｜合擊【{}】・見者有份割取【{}】",
                        p.zone_name, p.prey_name, p.prey_loot_name
                    ),
                    Some([p.prey_coord.0, p.prey_coord.1]),
                ),
                store::HuntingPartyPhase::ReturningToMarket => (
                    format!(
                        "{}合社獵團｜滿載【{}】赴【{}】交割分成",
                        p.zone_name, p.prey_loot_name, p.market_name
                    ),
                    None,
                ),
            };
            for mid in &p.member_ids {
                map.insert(mid.clone(), (badge.clone(), target));
            }
        }
        map
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
        let (hunting_party, prey_target) = party_lookup
            .get(&e.id)
            .cloned()
            .unwrap_or_else(|| (String::new(), None));
        let eff_mg = e.magnesium;
        let thought = derive_eco_inner_thought(&e, &title, &activity, &hunting_party, eff_mg);
        let magnesium = Some(eff_mg);

        entities.push(EcoEntity {
            id: e.id,
            name,
            title,
            x,
            y,
            kind: e.kind,
            activity,
            thought,
            magnesium,
            hunting_party,
            prey_target,
            breath_habit,
            held_vessel,
            afflictions,
            lineage,
        });
    }

    // 補入 19 處獵場的 1,805 隻《白塔山海誌》全島自然棲地生靈（消除同心圓，按真實疆域進行藍噪聲 Poisson-disc 與獸群聚落分布）
    let time_tick = (crate::game::now_unix() / 180) as i64;
    for (z_idx, &(zone_name, _, _)) in crate::game::CANONICAL_HUNTING_ZONES.iter().enumerate() {
        let slots = crate::game::live_zone_shanhai_slots(z_idx, time_tick);
        for sl in slots {
            if let Some(amb) = crate::game::ensure_ambient_npc_at_grid(sl.x, sl.y, sl.tier_tag) {
                let held_vessel = if let Some((_lid, loot_name, shanhai_note)) =
                    crate::game::ambient_creature_loot(&amb.display_title)
                {
                    format!("可剝取【{loot_name}】｜{shanhai_note}")
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
                        "{}{}（常隱於{}深處，唯順著林間深深獸跡才能尋見蹤影）",
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
                    thought: String::new(),
                    magnesium: None,
                    hunting_party: String::new(),
                    prey_target: None,
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

/// 取得全島 16 聚落即時供需行情與 19 獵場山海異兆總覽
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
        "下營",
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
            let apex_omen = crate::game::dynamic_hunting_zone_ecology_omen(name)
                .unwrap_or_default();
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

    let now_unix = crate::game::now_unix();
    let stranded_cargo_orders =
        crate::npc::causal_core::list_active_stranded_cargo_orders(now_unix);
    let mut active_ripples = Vec::new();
    if let Ok(guard) = crate::npc::causal_core::LIVE_NPC_CAUSAL_STATES.read() {
        for state in guard.values() {
            for rip in &state.active_ripples {
                if rip.origin_tick + 5400 >= now_unix
                    && !active_ripples.iter().any(|r: &crate::npc::causal_core::WorldRippleFact| r.fact_id == rip.fact_id)
                {
                    active_ripples.push(rip.clone());
                }
            }
        }
    }
    let active_ripples_count = active_ripples.len();

    Json(EcoPulseResponse {
        explicit_cells,
        settlements,
        hunting_zones,
        active_ripples_count,
        active_ripples,
        stranded_cargo_orders,
    })
    .into_response()
}
