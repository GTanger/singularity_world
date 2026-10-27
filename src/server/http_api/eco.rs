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

    // 補入 19 處獵場的代表性《台灣山海經》生靈駐點（供上帝全地圖觀測與檢視）
    const HUNTING_ANCHORS: &[(&str, i32, i32)] = &[
        ("長風草甸", -180, -195),
        ("古河巨木林", -80, -60),
        ("赤石坡", -85, -180),
        ("落日岬", -340, -140),
        ("驚濤岩岸", -360, 180),
        ("巨藻淺灣", -300, 260),
        ("黑石灣", -260, -20),
        ("西麓林道", -130, 80),
        ("清泉溪谷", 60, 160),
        ("翠冠巨木林", 0, 240),
        ("北坡雲杉林", 80, 300),
        ("淺澤泥口", 0, -340),
        ("白茫洲", -120, -320),
        ("貼山深潭", 120, -320),
        ("青籠關", 0, -450),
        ("西嶺礦谷", 180, 40),
        ("東海峭壁", 320, 200),
        ("雲腰雪林", 180, 320),
        ("白塔雪巔", 0, 420),
    ];
    let time_tick = (crate::game::now_unix() / 180) as i32;
    let roam_dx = (time_tick % 3) - 1;
    let roam_dy = ((time_tick / 3) % 3) - 1;

    for (z_idx, &(zone_name, cx, cy)) in HUNTING_ANCHORS.iter().enumerate() {
        let apex_x = cx + 1;
        let apex_y = cy - 1;

        // 1. Tier 1 鎮場霸主（預設隱匿・見兆尋跡顯形）：每座獵場 1 頭潛伏於核心地脈（全島 19 頭，佔山海生靈 1.25%）
        if let Some(apex) = crate::game::ensure_ambient_npc_at_grid(apex_x, apex_y, "__apex__") {
            let held_vessel = if let Some((_lid, loot_name, shanhai_note)) =
                crate::game::ambient_creature_loot(&apex.display_title)
            {
                format!("尋跡顯形可剝取【{loot_name}】｜{shanhai_note}")
            } else {
                String::new()
            };
            entities.push(EcoEntity {
                id: format!("apex_{z_idx}_{apex_x}_{apex_y}"),
                name: apex.display_title,
                title: format!("{zone_name}・隱匿鎮場霸主（尋跡顯形）"),
                x: apex_x,
                y: apex_y,
                kind: "shanhai_apex".to_string(),
                activity: format!(
                    "【隱匿潛伏・百鳥屏息】{}（平時藏跡於林莽深處，須查探【新鮮霸主獸跡】方會破霧顯形）",
                    apex.current_activity
                ),
                breath_habit: "山林長息".to_string(),
                held_vessel,
                afflictions: Vec::new(),
                lineage: Vec::new(),
            });
        }

        // 依據艾爾頓個體數量金字塔（Eltonian Pyramid of Numbers）與台灣山林生態調查比例：
        // 每座獵場配置 79 隻顯形游走生靈（全島 19 獵場共 1,501 隻顯形生靈 + 19 頭隱匿霸主 = 1,520 隻山海生靈）：
        // - Tier 2 次級掠食／珍稀靈獸（5 隻 / 獵場，全島 95 隻，6.25%，霸主:掠食 = 1:5）
        // - Tier 3 尋常中型獸群（18 隻 / 獵場，分 6 群結伴游走，全島 342 隻，22.50%，霸主:中型獸群 = 1:18）
        // - Tier 4 尋常小怪與靈禽（56 隻 / 獵場，含 28 隻小型奇獸/甲殼 + 28 隻山海靈禽/樹蛙，全島 1,064 隻，70.00%，霸主:小怪 = 1:56）
        let mut push_pyramid_fauna =
            |slot_idx: usize, raw_dx: i32, raw_dy: i32, tier_tag: &str, kind_str: &str, tier_label: &str, act_prefix: &str| {
                let mut dx = raw_dx + roam_dx;
                let mut dy = raw_dy + roam_dy;
                // 避開鎮場霸主周圍 2 格的「百鳥屏息圈」（小怪走避，形成視覺可見的威壓空白圈）
                if (cx + dx - apex_x).abs() <= 2 && (cy + dy - apex_y).abs() <= 2 {
                    dx += if dx >= 0 { 3 } else { -3 };
                    dy += if dy >= 0 { 3 } else { -3 };
                }
                let mut chosen = None;
                // 若海岬或峭壁地塊落入深水/岩壁，則向獵場中心收斂尋找合法棲地格
                for scale in [100, 75, 50, 25, 0] {
                    let tx = (cx + (dx * scale) / 100).clamp(-395, 395);
                    let ty = (cy + (dy * scale) / 100).clamp(-495, 495);
                    if (tx - apex_x).abs() <= 1 && (ty - apex_y).abs() <= 1 {
                        continue;
                    }
                    if let Some(amb) = crate::game::ensure_ambient_npc_at_grid(tx, ty, tier_tag) {
                        chosen = Some((tx, ty, amb));
                        break;
                    }
                }
                if chosen.is_none() {
                    for &(ox, oy) in &[(-3, 2), (4, -3), (-4, -2), (3, 4), (-2, 4)] {
                        let tx = (cx + ox).clamp(-395, 395);
                        let ty = (cy + oy).clamp(-495, 495);
                        if let Some(amb) = crate::game::ensure_ambient_npc_at_grid(tx, ty, tier_tag) {
                            chosen = Some((tx, ty, amb));
                            break;
                        }
                    }
                }
                if let Some((gx, gy, amb)) = chosen {
                    let held_vessel = if let Some((_lid, loot_name, shanhai_note)) =
                        crate::game::ambient_creature_loot(&amb.display_title)
                    {
                        format!("可剝取【{loot_name}】｜{shanhai_note}")
                    } else {
                        String::new()
                    };
                    entities.push(EcoEntity {
                        id: format!("shanhai_{z_idx}_{slot_idx}_{gx}_{gy}"),
                        name: amb.display_title,
                        title: format!("{zone_name}・{tier_label}"),
                        x: gx,
                        y: gy,
                        kind: kind_str.to_string(),
                        activity: format!("{act_prefix}{}", amb.current_activity),
                        breath_habit: "山林長息".to_string(),
                        held_vessel,
                        afflictions: Vec::new(),
                        lineage: Vec::new(),
                    });
                }
            };

        // 2. Tier 2 次級掠食猛獸／珍獸（6 隻 / 獵場，全島 114 隻，巡弋於中圈）
        const TIER2_OFFSETS: &[(i32, i32)] = &[(-6, 5), (7, 4), (-8, -6), (6, -7), (0, 9), (-9, 1)];
        for (i, &(dx, dy)) in TIER2_OFFSETS.iter().enumerate() {
            push_pyramid_fauna(
                i,
                dx,
                dy,
                "__tier2__",
                "shanhai_predator",
                "顯形次級掠食／珍獸",
                "【格內顯形・伏行巡獵】",
            );
        }

        // 3. Tier 3 尋常中型獸群（20 隻 / 獵場，全島 380 隻，結伴遷徙）
        const HERD_OFFSETS: &[(i32, i32)] = &[
            (-11, 4), (-10, 4), (-11, 5),
            (10, 8), (11, 8), (10, 9),
            (-9, -11), (-8, -11), (-9, -10),
            (12, -9), (13, -9), (12, -8),
            (-4, 14), (-3, 14), (-4, 15),
            (4, -14), (5, -14), (4, -13),
            (-14, -2), (-14, -1),
        ];
        for (h_idx, &(hx, hy)) in HERD_OFFSETS.iter().enumerate() {
            push_pyramid_fauna(
                6 + h_idx,
                hx,
                hy,
                "__tier3__",
                "shanhai_herd",
                "顯形尋常中型獸群",
                "【格內顯形・結群游走】",
            );
        }

        // 4. Tier 4 尋常小怪與山海靈禽（68 隻 / 獵場，全島 1,292 隻，以黃金角螺旋散佈於半徑 4..32 格）
        for k in 0..68usize {
            let angle = (k as f64) * 2.399963229728653 + (z_idx as f64) * 0.47;
            let radius = 4.0 + ((k + 1) as f64).sqrt() * 3.4;
            let dx = (angle.cos() * radius).round() as i32;
            let dy = (angle.sin() * radius).round() as i32;
            let (tier_tag, tier_label) = if k % 2 == 0 {
                ("__tier4a__", "顯形尋常小怪（奇獸／甲殼）")
            } else {
                ("__tier4b__", "顯形尋常小怪（靈禽／兩棲）")
            };
            push_pyramid_fauna(
                26 + k,
                dx,
                dy,
                tier_tag,
                "shanhai_beast",
                tier_label,
                "【格內顯形・漫山游走】",
            );
        }

        // 5. 山海獵場與古道驛路三大古典編制（每座獵場／聯外古道 18 人，全島 19 處共 342 人 + 1 = 343 人，佔全島人類 19.4%）：
        // - 典故一：《東番記》與《諸羅縣志》「合社共獵、帶犬持鏢」——結伴獵團 6 人
        // - 典故二：《裨海紀遊》採磺煉樟與清代「腦寮／木馬道伐木／百草採藥」——入山採集工班 7 人
        // - 典故三：《淡蘭古道》與《噶瑪蘭廳志》「郊商挑夫、牛車商隊、隘勇巡哨」——古道行旅與巡隘 5 人
        const WILDERNESS_EXPEDITIONS: &[((i32, i32), &str, &str, &str)] = &[
            // (一) 合社結伴獵團（6 人）
            ((-5, -4), "合社獵團・領隊老獵頭", "手持百年山桑獵弓與山豬牙號角", "【合社共獵】正蹲身觀察泥地獸跡深淺，以手勢指揮獵團包抄下風處"),
            ((-4, -5), "合社獵團・持鏢主獵手", "手握倒鉤熟鐵長鏢與鹿皮護腕", "【合社共獵】正屏息伏於芒草與灌木掩體後，凝神鎖定前方獸群動向"),
            ((-6, -3), "合社獵團・尋跡引犬人", "牽引精悍山地獵犬、腰別山刀", "【合社共獵】正循著獵犬低鳴指引，辨識腐葉間殘留的山獸氣息"),
            ((-3, -6), "合社獵團・開山刀手", "手執加厚精鍛台灣山刀與藤盾", "【合社共獵】正揮刀劈開帶刺咬人貓與糾結葛藤，為獵團開闢林間小徑"),
            ((-7, -5), "合社獵團・割鹿焙脯匠", "攜帶刮皮骨刀、粗鹽包與麻布袋", "【合社共獵】正於獵寮石灶旁張開竹架剝整獸皮，以相思木慢火焙製鹿脯"),
            ((-5, -7), "合社獵團・負皮擔夫", "肩挑成捆水鹿皮與山羌皮扁擔", "【合社共獵】正將硝整妥當的獸皮與山產捆紮上肩，準備運往市集交割"),
            // (二) 入山腦寮、伐木、採磺與百草採集工班（7 人）
            ((6, 3), "入山百工・劈樟腦丁頭", "手持彎刃樟腦刨刀與油布護裙", "【山海採集】正於巨木根部熟練削取富含腦油的樟木薄片，香氣四溢"),
            ((7, 4), "入山百工・顧灶煉腦師", "手持長柄木杓與測溫銅管", "【山海採集】正看守山澗石砌腦灶火候，引清泉冷卻水氣凝結雪白樟腦霜"),
            ((5, 5), "入山百工・木馬道集材夫", "肩扛雙人鋼齒大鋸與塗油木馬架", "【山海採集】正於林間木馬軌塗抹烏桕滑油，合力拖曳百斤良材下山"),
            ((8, 2), "入山百工・懸崖採藥客", "背負竹編藥簍、熟鐵藥鋤與攀岩麻索", "【山海採集】正腰繫粗麻安全索，於背陰岩壁與腐木間採挖百年靈草"),
            ((4, 6), "入山百工・探脈淘砂匠", "手握選礦木盆、開山鐵鎬與麻袋", "【山海採集】正於溪谷岩脈間敲取礦石，引急流淘洗高純度重鐵砂與石粉"),
            ((7, 6), "入山百工・抽藤編簍匠", "手持破藤篾刀與防刺厚皮手套", "【山海採集】正砍取深山韌實黃藤、刮節煙燻，編結耐重山海背架"),
            ((6, 7), "入山百工・山寮伙夫", "身繫麻布圍裙、手執大鐵杓與驅蟲艾草束", "【山海採集】正於山寮石灶旁生起樟木營火，熬煮熱騰騰的薑湯與芋頭鹹粥"),
            // (三) 淡蘭古道挑夫幫、郊商牛車隊與隘勇巡哨（5 人）
            ((-3, 7), "古道行旅・古道挑夫頭", "肩挑百斤山海貨擔、手持鐵箍竹杖", "【古道行旅】正領著挑夫隊伍踏上青苔石階，高聲吆喝提醒前後路況"),
            ((-2, 8), "古道行旅・負重擔夫", "背負雙層竹製背架與粗鹽麻袋", "【古道行旅】正趁著古道樹蔭歇腳擦汗，暢飲竹筒內的清涼山泉"),
            ((-4, 9), "古道行旅・郊商押貨管事", "腋夾油布貨單帳冊、手撥銅算盤", "【古道行旅】正隨行清點運往城鎮市集的山海乾貨、藥材與皮料件數"),
            ((-1, 9), "古道行旅・護商刀牌鏢師", "身披浸油藤牌、腰佩熟鐵護手刀", "【古道行旅】正按刀走在商隊兩翼高處，雙目如電戒備林莽動靜"),
            ((-3, 10), "古道行旅・巡隘銃手", "身穿鑲皮號褂、肩扛火繩鳥銃與銅哨", "【古道行旅】正駐守古道隘口寮站，巡查往來旅人並標記沿途路況"),
        ];
        for (w_idx, &((wdx, wdy), role_label, vessel_str, act_str)) in WILDERNESS_EXPEDITIONS.iter().enumerate() {
            let wx = (cx + wdx + roam_dx).clamp(-395, 395);
            let wy = (cy + wdy + roam_dy).clamp(-495, 495);
            let wid = format!("wild_human_{z_idx}_{w_idx}");
            let wname = store::resolve_npc_person_name(&wid, "", None);
            entities.push(EcoEntity {
                id: wid,
                name: wname,
                title: format!("{zone_name}・{role_label}"),
                x: wx,
                y: wy,
                kind: "npc".to_string(),
                activity: act_str.to_string(),
                breath_habit: "山海行氣".to_string(),
                held_vessel: vessel_str.to_string(),
                afflictions: Vec::new(),
                lineage: Vec::new(),
            });
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
