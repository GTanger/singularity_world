pub(crate) fn inv_count_item(inv_json: &str, target_id: &str) -> i32 {
    serde_json::from_str::<Vec<serde_json::Value>>(inv_json)
        .ok()
        .into_iter()
        .flatten()
        .filter(|v| v.get("item_id").and_then(|s| s.as_str()) == Some(target_id))
        .map(|v| v.get("qty").and_then(|q| q.as_i64()).unwrap_or(0) as i32)
        .sum()
}

fn inv_mutate_item(inv_json: &mut String, target_id: &str, delta: i32, max_cap: i32) -> i32 {
    let mut items = serde_json::from_str::<Vec<serde_json::Value>>(inv_json).unwrap_or_default();
    let mut new_qty = 0i32;
    let mut found = false;
    for entry in items.iter_mut() {
        if entry.get("item_id").and_then(|s| s.as_str()) == Some(target_id) {
            let cur = entry.get("qty").and_then(|q| q.as_i64()).unwrap_or(0) as i32;
            let next = (cur + delta).clamp(0, max_cap);
            entry["qty"] = serde_json::json!(next);
            new_qty = next;
            found = true;
            break;
        }
    }
    if !found && delta > 0 {
        new_qty = delta.clamp(0, max_cap);
        items.push(serde_json::json!({ "item_id": target_id, "qty": new_qty }));
    }
    items.retain(|v| v.get("qty").and_then(|q| q.as_i64()).unwrap_or(0) > 0);
    if let Ok(s) = serde_json::to_string(&items) {
        *inv_json = s;
    }
    new_qty
}

/// 玩家同構「四問決策閉環」（`Observe -> Can Continue? -> Choose -> Mutate World`）：
/// 驅動 `秦水生` 真實消耗白米與體力、累積並推磨手鏨磨損、跨區向陳萬福交割米漿結算鎂幣、斷料向周萬生提穀，並寫回資料庫。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentLifeActionChoice {
    RepairTool,
    DeliverContract,
    RestockGrain,
    FloodgateCheck,
    DineParley,
    NightRest,
    MillGrain,
}

/// 由角色「先天心性單純形 × CAPS 處境觸發相 × 後天所學・所熟・所歷」（`PluralPersonalityWeights`）
/// 對當下處境候選行動進行效用權衡，取代死板之單一 `if / else` 機器人梯子。
/// 相同處境下（例如手鏨微鈍 `wear_points == 3` 恰逢早市交貨，或體力微乏 `vit == 42`），
/// 不同 `soul_seed` 與心相權重的人物會做出截然不同但符合其為人的選擇。
pub fn choose_resident_life_action(
    role_title: &str,
    settlement: &str,
    soul_seed: u64,
    vit: i32,
    magnesium: i32,
    grain_qty: i32,
    ricemilk_qty: i32,
    wear_points: i32,
    ricemilk_ob_open: bool,
    game_hour: i32,
    weather: crate::game::ZoneWeatherPhase,
    tide: crate::game::ZoneTidePhase,
    repetitions: u32,
    hooks: &[crate::npc::causal_core::RelationalHook],
) -> ResidentLifeActionChoice {
    let stress = ((100 - vit.clamp(0, 100)) as u8).saturating_div(2);
    let pw = crate::npc::causal_core::compute_plural_personality_weights_with_experience(
        role_title,
        settlement,
        soul_seed,
        stress,
        magnesium,
        repetitions,
        hooks,
        None,
    );

    let net_trust: i32 = hooks
        .iter()
        .map(|h| {
            let is_grudge = ["遭拒", "追討", "結怨", "嫌隙", "展期"]
                .iter()
                .any(|k| h.kind_label.contains(k) || h.summary.contains(k));
            let is_favor = ["恩", "守諾", "解囊", "馳援", "完修", "交割", "供料"]
                .iter()
                .any(|k| h.kind_label.contains(k) || h.summary.contains(k));
            if is_grudge {
                -(h.intensity as i32)
            } else if is_favor {
                h.intensity as i32
            } else {
                0
            }
        })
        .sum::<i32>()
        .clamp(-60, 80);

    // 1. 修磨器物 vs 強行趕工／先送早市：
    //    - wear_points == 2 時，極重匠心與守序的完美主義者已會考慮提前修磨；
    //    - wear_points == 3 時，匠心／守序型選擇先修磨（哪怕延誤早市交貨），
    //      而重義／嗜利型在有早市契約時會選擇「先履約送貨、回來再磨刀」，
    //      極端嗜利／傲骨趕工型甚至會在無契約時硬撐鈍器再趕一輪工（直至 wear_points >= 4）。
    let score_repair = if wear_points >= 4 {
        1250 + pw.craft * 5 + pw.order * 3
    } else if wear_points == 3 {
        680 + pw.craft * 5 + pw.order * 3 - pw.profit
    } else if wear_points == 2 && pw.craft >= 148 && pw.order >= 130 && grain_qty >= 3 {
        560 + pw.craft * 3 + pw.order * 2
    } else {
        0
    };

    let score_deliver = if ricemilk_ob_open && ricemilk_qty >= 2 && (5..=8).contains(&game_hour) {
        580 + pw.honor * 4 + pw.profit * 3 + net_trust * 6
    } else {
        0
    };

    let score_restock = if grain_qty < 2 && (6..=18).contains(&game_hour) {
        740 + (2 - grain_qty.min(2)) * 180 + pw.order * 2 + pw.profit * 2 + pw.craft
    } else {
        0
    };

    let score_floodgate = if weather == crate::game::ZoneWeatherPhase::RainMist
        && tide == crate::game::ZoneTidePhase::FloodTide
        && (7..=17).contains(&game_hour)
    {
        720 + pw.care * 3 + pw.order * 2 + pw.honor
    } else {
        0
    };

    let score_dine = if (18..=20).contains(&game_hour) || vit < 32 {
        680 + (50 - vit.min(50)) * 8 + pw.care * 2
    } else if vit < 48 && (7..=17).contains(&game_hour) {
        430 + (48 - vit) * 14 + pw.care * 3 - pw.craft - pw.profit
    } else {
        0
    };

    let score_rest = if !(7..=17).contains(&game_hour) {
        600 + pw.order * 2 + pw.care * 2
    } else {
        0
    };

    let score_mill = if (7..=17).contains(&game_hour) && grain_qty >= 1 {
        490 + pw.craft * 2 + pw.profit * 2 + pw.pride + pw.order
    } else {
        100
    };

    let candidates = [
        (ResidentLifeActionChoice::RepairTool, score_repair),
        (ResidentLifeActionChoice::DeliverContract, score_deliver),
        (ResidentLifeActionChoice::RestockGrain, score_restock),
        (ResidentLifeActionChoice::FloodgateCheck, score_floodgate),
        (ResidentLifeActionChoice::DineParley, score_dine),
        (ResidentLifeActionChoice::NightRest, score_rest),
        (ResidentLifeActionChoice::MillGrain, score_mill),
    ];

    candidates
        .into_iter()
        .max_by_key(|&(_, s)| s)
        .map(|(c, _)| c)
        .unwrap_or(ResidentLifeActionChoice::MillGrain)
}

pub fn step_flagship_specimen_qin_shuisheng_life_loop(
    store: &mut crate::store::Store,
    game_hour: i32,
    now_unix: i64,
) -> (i32, i32, String) {
    step_resident_human_life_loop(
        store,
        "秦水生",
        -3,
        125,
        "老堰頭・水輪石磨工坊",
        "引流老堰頭急湍活水驅動雙疊石磨，傾聽磨盤節奏精細碾磨新收稻穀",
        game_hour,
        now_unix,
    )
}

fn inspect_raw_stock_and_tool(
    entity_id: &str,
    display_title: &str,
    equipment_slots: &str,
    inventory: &str,
) -> (String, String, bool, VesselStateRecord, i32, i32) {
    let mut tool_id = String::new();
    let mut tool_name = String::new();
    let mut is_bladed = false;
    let mut equipped_ids = std::collections::HashSet::new();

    if let Ok(slots) = serde_json::from_str::<std::collections::HashMap<String, String>>(equipment_slots) {
        for (_, val) in &slots {
            if !val.trim().is_empty() {
                equipped_ids.insert(val.clone());
            }
        }
        for slot in &["hold_r", "hold_l"] {
            if let Some(tid) = slots.get(*slot).filter(|id| !id.trim().is_empty()) {
                let name = crate::game::peninsula_catalog::find_catalog_entry(tid)
                    .map(|c| c.name.to_string())
                    .or_else(|| crate::game::object_ecology::lookup_ecology_item_def(tid).map(|d| d.name.to_string()))
                    .or_else(|| crate::db::inventory::fallback_item_name(tid).map(|s| s.to_string()))
                    .unwrap_or_else(|| crate::npc::causal_core::resolve_vocational_vessel(display_title).display_name.to_string());
                let is_rejected_item = name.contains("傘")
                    || name.contains("扇")
                    || name.contains("膏")
                    || name.contains("散")
                    || name.contains("丸")
                    || name.contains("丹")
                    || name.contains("藥")
                    || name.contains("米")
                    || name.contains("漿")
                    || name.contains("湯")
                    || name.contains("餅")
                    || name.contains("茶")
                    || name.contains("麵")
                    || name.contains("粥")
                    || name.contains("果")
                    || name.contains("肉")
                    || name.contains("魚")
                    || name.contains("衫")
                    || name.contains("袍")
                    || name.contains("襖")
                    || name.contains("衣")
                    || name.contains("褲")
                    || name.contains("靴")
                    || name.contains("鞋");
                if is_rejected_item {
                    continue;
                }
                let bladed = !tid.contains("0973") && !tid.contains("abacus") && !tid.contains("basket");
                tool_id = tid.clone();
                tool_name = name;
                is_bladed = bladed;
                break;
            }
        }
    }
    let mismatched_default_knife = tool_id == "craft_steel_hunting_knife"
        && !display_title.trim().is_empty()
        && !display_title.contains("獵")
        && !display_title.contains("巡")
        && !display_title.contains("衛")
        && !display_title.contains("哨")
        && !display_title.contains("武");
    if tool_id.is_empty() || tool_id == "craft_bamboo_basket" || mismatched_default_knife {
        let spec = crate::npc::causal_core::resolve_vocational_vessel(display_title);
        if tool_id.is_empty() || spec.item_id != "craft_bamboo_basket" {
            tool_id = if !spec.catalog_id.is_empty() {
                spec.catalog_id.to_string()
            } else {
                spec.item_id.to_string()
            };
            tool_name = spec.display_name.to_string();
            is_bladed = spec.is_bladed;
        }
    }

    let vstate = get_vessel_state(entity_id, &tool_id);

    let mut raw_stock_qty = 0i32;
    let mut finished_stock_qty = 0i32;
    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(inventory) {
        for v in items {
            let id = v.get("item_id").or_else(|| v.get("id")).and_then(|s| s.as_str()).unwrap_or("");
            let qty = v.get("qty").or_else(|| v.get("count")).and_then(|q| q.as_i64()).unwrap_or(0) as i32;
            if id.starts_with("wt-good-") || id.starts_with("mat_") || id.starts_with("harvest_") {
                raw_stock_qty += qty;
            } else if id.starts_with("craft_") || (id.starts_with("wt_cat_") && !equipped_ids.contains(id)) {
                finished_stock_qty += qty;
            }
        }
    }

    (tool_id, tool_name, is_bladed, vstate, raw_stock_qty, finished_stock_qty)
}

/// 依實體角色資料檢視其當前手持／職務器物與庫存原料、成品狀態
pub fn inspect_entity_live_stock_and_tool(
    ent: &crate::entity::Character,
) -> (String, String, bool, VesselStateRecord, i32, i32) {
    inspect_raw_stock_and_tool(&ent.id, &ent.display_title, &ent.equipment_slots, &ent.inventory)
}

/// 依 store 實體資料檢視其當前手持／職務器物與庫存原料、成品狀態
pub fn inspect_store_entity_live_stock_and_tool(
    ent: &crate::store::Entity,
) -> (String, String, bool, VesselStateRecord, i32, i32) {
    inspect_raw_stock_and_tool(&ent.id, &ent.display_title, &ent.equipment_slots, &ent.inventory)
}

fn record_resident_continuity_row(
    entity_id: &str,
    intent: &str,
    reason: &str,
    dest_place: &str,
    now_unix: i64,
) {
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let next_wake = now_unix + 60;
        let _ = client.execute(
            "INSERT INTO resident_continuity (
                entity_id, control_mode, life_status, current_intent,
                intent_reason, destination_place_id, intent_started_at,
                next_wake_at, updated_at, source_kind, source_ref
            ) VALUES (
                $1, 'autonomous', 'alive', $2,
                $3, $4, $5,
                $6, $5, 'four_question_loop', 'resident_life_loop'
            )
            ON CONFLICT (entity_id) DO UPDATE SET
                control_mode = 'autonomous',
                life_status = 'alive',
                current_intent = EXCLUDED.current_intent,
                intent_reason = EXCLUDED.intent_reason,
                destination_place_id = EXCLUDED.destination_place_id,
                blocked_by = NULL,
                source_kind = EXCLUDED.source_kind,
                source_ref = EXCLUDED.source_ref,
                updated_at = EXCLUDED.updated_at",
            &[&entity_id, &intent, &reason, &dest_place, &now_unix, &next_wake],
        );
    }
}

#[derive(Debug, Clone)]
pub struct ResidentIntrinsicFactors {
    pub tool_id: String,
    pub tool_name: String,
    pub is_bladed: bool,
    pub secondary_tool_id: Option<String>,
    pub secondary_tool_name: Option<String>,
    pub maintenance_station: String,
    pub raw_item_id: String,
    pub raw_item_name: String,
    pub raw_item_unit: &'static str,
    pub raw_consume_qty: i32,
    pub outputs: Vec<(String, String, &'static str, i32)>,
    pub delivery_items: Vec<(String, String, &'static str, i32)>,
    pub delivery_payout_mg: i32,
    pub delivery_vessel_desc: String,
    pub need_supply_tag: &'static str,
    pub need_buyer_tag: &'static str,
    pub supplier_id: String,
    pub supplier_place_label: String,
    pub supplier_coord: Option<(i32, i32)>,
    pub restock_qty: i32,
    pub restock_cost_mg: i32,
    pub counterparty_id: String,
    pub work_coord: (i32, i32),
    pub deliver_coord: (i32, i32),
    pub floodgate_coord: (i32, i32),
    pub canteen_coord: (i32, i32),
    pub home_coord: (i32, i32),
    pub work_bldg: String,
    pub deliver_bldg: String,
    pub floodgate_bldg: String,
    pub floodgate_facility: String,
    pub canteen_bldg: String,
    pub home_bldg: String,
    pub settlement: String,
    pub work_activity_desc: String,
    pub anatomy_familiarity_subject: String,
}

impl ResidentIntrinsicFactors {
    pub fn resolve(
        ent: &crate::store::Entity,
        sched_x: i32,
        sched_y: i32,
        sched_bldg: &str,
        sched_act: &str,
    ) -> Self {
        let (mut tool_id, mut tool_name, mut is_bladed, _, _, _) =
            inspect_store_entity_live_stock_and_tool(ent);
        let mut hold_r = String::new();
        let mut hold_l = String::new();
        let mut waist_id = String::new();
        if let Ok(slots) =
            serde_json::from_str::<std::collections::HashMap<String, String>>(&ent.equipment_slots)
        {
            if let Some(r) = slots.get("hold_r") {
                hold_r = r.clone();
            }
            if let Some(l) = slots.get("hold_l") {
                hold_l = l.clone();
            }
            if let Some(w) = slots.get("waist") {
                waist_id = w.clone();
            }
        }

        let imprints = get_entity_lineage_imprints(&ent.id);
        let has_mill_factor = hold_r == "wt_cat_0984"
            || hold_l == "wt_cat_0973"
            || ent.display_title.contains("水輪")
            || ent.display_title.contains("石磨")
            || sched_bldg.contains("石磨")
            || sched_bldg.contains("水輪")
            || sched_act.contains("石磨")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("石磨") || imp.imprint_title.contains("水輪"));
        let smith_primary = tool_id.contains("hammer")
            || ent.display_title.contains("鐵")
            || ent.display_title.contains("鍛")
            || sched_bldg.contains("鐵")
            || sched_bldg.contains("鍛")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("鍛") || imp.imprint_title.contains("落錘"));
        let carpenter_primary = tool_id.contains("saw")
            || tool_id.contains("chisel")
            || ent.display_title.contains("木")
            || ent.display_title.contains("鋸")
            || ent.display_title.contains("船")
            || sched_bldg.contains("木")
            || sched_bldg.contains("鋸")
            || sched_bldg.contains("船")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("榫卯") || imp.imprint_title.contains("木"));
        let herbal_primary = tool_id.contains("mortar")
            || ent.display_title.contains("藥")
            || ent.display_title.contains("醫")
            || sched_bldg.contains("藥")
            || sched_bldg.contains("診")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("藥") || imp.imprint_title.contains("百草"));
        let hunt_leather_primary = tool_id.contains("bow")
            || tool_id.contains("knife")
            || tool_id == "wt_cat_1021"
            || ent.display_title.contains("獵")
            || ent.display_title.contains("皮")
            || ent.display_title.contains("弓")
            || ent.display_title.contains("靴")
            || ent.display_title.contains("鞋")
            || ent.display_title.contains("傘")
            || sched_bldg.contains("獵")
            || sched_bldg.contains("皮")
            || sched_bldg.contains("弓")
            || sched_act.contains("獵")
            || sched_act.contains("皮")
            || sched_act.contains("弓")
            || sched_act.contains("楦頭")
            || sched_act.contains("補靴")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("獵") || imp.imprint_title.contains("皮") || imp.imprint_title.contains("弓"));
        let fish_salt_primary = tool_id.contains("rod")
            || tool_id == "wt_cat_1006"
            || ent.display_title.contains("漁")
            || ent.display_title.contains("鹽")
            || ent.display_title.contains("海")
            || sched_bldg.contains("漁")
            || sched_bldg.contains("鹽")
            || sched_bldg.contains("海")
            || sched_act.contains("漁")
            || sched_act.contains("鹽")
            || sched_act.contains("海")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("漁") || imp.imprint_title.contains("鹽") || imp.imprint_title.contains("海"));
        let weave_tailor_primary = tool_id.contains("needle")
            || tool_id.contains("shears")
            || tool_id == "wt_cat_0890"
            || ent.display_title.contains("織")
            || ent.display_title.contains("布")
            || ent.display_title.contains("染")
            || ent.display_title.contains("針")
            || sched_bldg.contains("織")
            || sched_bldg.contains("布")
            || sched_bldg.contains("染")
            || sched_bldg.contains("針")
            || sched_act.contains("織")
            || sched_act.contains("布")
            || sched_act.contains("染")
            || sched_act.contains("針")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("織") || imp.imprint_title.contains("布") || imp.imprint_title.contains("染"));
        let farming_primary = tool_id.contains("plow")
            || tool_id.contains("hoe")
            || tool_id == "wt_cat_0950"
            || tool_id == "wt_cat_0981"
            || ent.display_title.contains("農")
            || ent.display_title.contains("田")
            || ent.display_title.contains("耕")
            || sched_bldg.contains("農")
            || sched_bldg.contains("田")
            || sched_bldg.contains("耕")
            || sched_act.contains("農")
            || sched_act.contains("田")
            || sched_act.contains("耕")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("農") || imp.imprint_title.contains("田") || imp.imprint_title.contains("耕"));

        let any_primary_factor = has_mill_factor
            || smith_primary
            || carpenter_primary
            || herbal_primary
            || hunt_leather_primary
            || fish_salt_primary
            || weave_tailor_primary
            || farming_primary;
        let allow_inventory_fallback = !any_primary_factor
            && ent.display_title.trim().is_empty()
            && sched_bldg.trim().is_empty()
            && !ent.id.starts_with("wt_npc_")
            && !ent.id.starts_with("wt_vendor_");

        let has_smith_factor = smith_primary
            || (allow_inventory_fallback
                && (ent.inventory.contains("wt_cat_0387") || ent.inventory.contains("wt-good-019")));
        let has_carpenter_factor = carpenter_primary
            || (allow_inventory_fallback
                && (ent.inventory.contains("craft_wooden_tool_handle")
                    || ent.inventory.contains("wt-good-012")));
        let has_herbal_factor = herbal_primary
            || (allow_inventory_fallback
                && (ent.inventory.contains("wt_cat_0293") || ent.inventory.contains("wt-good-025")));
        let has_hunt_leather_factor = hunt_leather_primary
            || (allow_inventory_fallback
                && (ent.inventory.contains("wt_cat_0203")
                    || ent.inventory.contains("wt_cat_0877")
                    || ent.inventory.contains("wt_cat_1021")
                    || ent.inventory.contains("鹿皮")));
        let has_fish_salt_factor = fish_salt_primary
            || (allow_inventory_fallback
                && (ent.inventory.contains("wt_cat_0229")
                    || ent.inventory.contains("wt_cat_0254")
                    || ent.inventory.contains("wt_cat_1006")
                    || ent.inventory.contains("海鹽")));
        let has_weave_tailor_factor = weave_tailor_primary
            || (allow_inventory_fallback
                && (ent.inventory.contains("wt_cat_0869")
                    || ent.inventory.contains("wt_cat_0835")
                    || ent.inventory.contains("wt_cat_0890")));
        let has_farming_factor = farming_primary
            || (allow_inventory_fallback
                && (ent.inventory.contains("wt_cat_0950")
                    || ent.inventory.contains("wt_cat_0981")
                    || ent.inventory.contains("wt_cat_0277")));
        let settlement = if has_mill_factor {
            "老堰頭".to_string()
        } else {
            let slot = crate::store::wenyao_household_slot_for(&ent.id);
            if ent.id == slot.name() || ent.display_title.contains(slot.name()) {
                "文曜".to_string()
            } else {
                let s = crate::world::materials::infer_origin_settlement(&format!("grid:{}:{}", ent.x, ent.y));
                if !s.is_empty() {
                    s.to_string()
                } else if !sched_bldg.is_empty() && sched_bldg.contains('・') {
                    sched_bldg.split('・').next().unwrap_or("文曜").to_string()
                } else {
                    "文曜".to_string()
                }
            }
        };

        let secondary_tool_id = if !hold_l.is_empty() && hold_l != tool_id {
            Some(hold_l.clone())
        } else if has_mill_factor {
            Some("wt_cat_0973".to_string())
        } else {
            None
        };
        let secondary_tool_name = secondary_tool_id.as_deref().map(|tid| {
            crate::game::peninsula_catalog::find_catalog_entry(tid)
                .map(|c| c.name.to_string())
                .or_else(|| {
                    crate::game::object_ecology::lookup_ecology_item_def(tid)
                        .map(|d| d.name.to_string())
                })
                .unwrap_or_else(|| crate::db::get_item_name(tid))
        });

        let needs_craft_tool = tool_id.is_empty() || tool_id == "craft_bamboo_basket";
        if has_mill_factor && (needs_craft_tool || tool_id == "wt_cat_0973") {
            tool_id = "wt_cat_0984".to_string();
            tool_name = "羊角鐵錘打石手鏨".to_string();
            is_bladed = true;
        } else if has_smith_factor && needs_craft_tool {
            tool_id = "wt_blacksmith_hammer".to_string();
            tool_name = "岑氏省力導能鍛錘".to_string();
            is_bladed = false;
        } else if has_carpenter_factor && needs_craft_tool {
            tool_id = "wt_carpenter_chisel".to_string();
            tool_name = "夾鋼手作木工鑿刀".to_string();
            is_bladed = true;
        } else if has_herbal_factor && needs_craft_tool {
            tool_id = "wt_cat_0968".to_string();
            tool_name = "打磨青石雙槽研磨缽".to_string();
            is_bladed = false;
        } else if has_hunt_leather_factor && needs_craft_tool {
            tool_id = "wt_cat_1021".to_string();
            tool_name = if ent.display_title.contains("靴") || ent.display_title.contains("傘") || ent.display_title.contains("鞋") {
                "彎頭修靴裁皮刀".to_string()
            } else {
                "岩羊角手柄折疊獵刀".to_string()
            };
            is_bladed = true;
        } else if has_fish_salt_factor && needs_craft_tool {
            tool_id = "wt_cat_1006".to_string();
            tool_name = "老青竹黃銅輪捲線釣竿".to_string();
            is_bladed = false;
        } else if has_weave_tailor_factor && needs_craft_tool {
            tool_id = "wt_cat_0890".to_string();
            tool_name = "熟鐵裁布大剪刀".to_string();
            is_bladed = true;
        } else if has_farming_factor && needs_craft_tool {
            tool_id = "wt_cat_0981".to_string();
            tool_name = "長把平頭挖泥鐵鍬".to_string();
            is_bladed = true;
        }

        let (
            maintenance_station,
            raw_item_id,
            raw_item_name,
            raw_item_unit,
            outputs,
            delivery_items,
            need_supply_tag,
            need_buyer_tag,
            supplier_place_label,
            work_activity_desc,
            anatomy_familiarity_subject,
        ) = if has_mill_factor {
            (
                "工坊引水渠旁的活水磨石台".to_string(),
                "wt-good-009".to_string(),
                "精碾白米".to_string(),
                "袋",
                vec![
                    ("wt_cat_0388".to_string(), "溫米漿".to_string(), "桶", 1),
                    ("wt_cat_0279".to_string(), "在來米粉".to_string(), "袋", 1),
                ],
                vec![
                    ("wt_cat_0388".to_string(), "現磨溫米漿".to_string(), "桶", 2),
                    ("wt_cat_0279".to_string(), "特級在來米粉".to_string(), "袋", 1),
                ],
                "grain",
                "canteen",
                "大圳頭".to_string(),
                "巡看老堰頭湍急溪水推動的重礦包邊大石磨，將運來的稻麥與北山藥材碾得細膩均勻"
                    .to_string(),
                "在來米與北山藥材水磨澄漿".to_string(),
            )
        } else if has_smith_factor {
            (
                format!("{}淬火石槽旁的活水磨石台", sched_bldg),
                "wt-good-019".to_string(),
                "赤鐵粗礦".to_string(),
                "簍",
                vec![("wt_cat_0387".to_string(), "百鍛農具鐵件".to_string(), "件", 1)],
                vec![("wt_cat_0387".to_string(), "百鍛農具鐵件".to_string(), "件", 2)],
                "ore",
                "market",
                "西嶺礦谷".to_string(),
                if sched_act.is_empty() {
                    format!("在{}揮動{}鍛打赤鐵粗礦，校準器物火候與分寸", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "百鍛鐵胚紋理與落錘火候".to_string(),
            )
        } else if has_carpenter_factor {
            (
                format!("{}木作案旁的活水磨石台", sched_bldg),
                "wt-good-012".to_string(),
                "北山老杉木料".to_string(),
                "根",
                vec![("craft_wooden_tool_handle".to_string(), "堅韌硬木工具長柄".to_string(), "件", 1)],
                vec![("craft_wooden_tool_handle".to_string(), "堅韌硬木工具長柄".to_string(), "件", 2)],
                "timber",
                "workshop",
                "霜林口".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}刨削北山老杉木料，校合榫卯分寸", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "北山杉木紋理與榫卯合度".to_string(),
            )
        } else if has_herbal_factor {
            (
                format!("{}碾槽旁的活水磨石台", sched_bldg),
                "wt-good-025".to_string(),
                "北山靈草".to_string(),
                "束",
                vec![("wt_cat_0293".to_string(), "清心草藥散".to_string(), "包", 1)],
                vec![("wt_cat_0293".to_string(), "清心草藥散".to_string(), "包", 2)],
                "medicine",
                "shop",
                "北山藥圃".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}分揀炮製北山靈草", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "北山百草藥性與炮製分寸".to_string(),
            )
        } else if has_hunt_leather_factor {
            (
                format!("{}皮作案旁的活水磨石台", sched_bldg),
                "wt_cat_0203".to_string(),
                "高地水鹿厚鹿皮".to_string(),
                "張",
                vec![("wt_cat_0877".to_string(), "鞣製黃牛皮寬版腰帶".to_string(), "條", 1)],
                vec![("wt_cat_0877".to_string(), "鞣製黃牛皮寬版腰帶".to_string(), "條", 2)],
                "leather",
                "shop",
                "長風草甸".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}熟化硝製厚鹿皮，修裁筋骨分寸", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "獸皮鞣製紋理與筋膜分寸".to_string(),
            )
        } else if has_fish_salt_factor {
            (
                format!("{}防浪棧道旁的收繩台", sched_bldg),
                "wt_cat_0229".to_string(),
                "南灣粗晶海鹽".to_string(),
                "袋",
                vec![("wt_cat_0254".to_string(), "海風日曬飛魚乾".to_string(), "條", 1)],
                vec![("wt_cat_0254".to_string(), "海風日曬飛魚乾".to_string(), "條", 2)],
                "sea_salt",
                "canteen",
                "南灣漁港".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}分揀醃製鮮魚，調和海鹽鹹度", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "海風潮候與魚鱗鹽漬分寸".to_string(),
            )
        } else if has_weave_tailor_factor {
            (
                format!("{}裁案旁的活水磨石台", sched_bldg),
                "wt_cat_0869".to_string(),
                "粗麻雙帶布料".to_string(),
                "匹",
                vec![("wt_cat_0835".to_string(), "細棉麻透氣常服".to_string(), "件", 1)],
                vec![("wt_cat_0835".to_string(), "細棉麻透氣常服".to_string(), "件", 2)],
                "fabric",
                "shop",
                format!("{settlement}織坊"),
                if sched_act.is_empty() {
                    format!("在{}運使{}裁切粗麻布匹，走線縫綴衣物", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "粗麻經緯經線與裁剪分寸".to_string(),
            )
        } else if has_farming_factor {
            (
                format!("{}田埂邊的活水磨石台", sched_bldg),
                "wt-good-009".to_string(),
                "精選稻種麥穀".to_string(),
                "袋",
                vec![("wt_cat_0277".to_string(), "秋收蓬萊新米".to_string(), "袋", 1)],
                vec![("wt_cat_0277".to_string(), "秋收蓬萊新米".to_string(), "袋", 2)],
                "grain",
                "workshop",
                "大圳頭".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}翻整田土，順應節氣引水灌溉", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "水脈土壤墒情與稻穀生息分寸".to_string(),
            )
        } else if ent.display_title.contains("鑿石")
            || ent.display_title.contains("切石")
            || ent.display_title.contains("灰窯")
            || ent.display_title.contains("磚窯")
            || sched_bldg.contains("鑿石")
            || sched_bldg.contains("切石")
            || sched_bldg.contains("磚窯")
        {
            (
                format!("{}的鑿石工作台", sched_bldg),
                "wt_cat_0001".to_string(),
                "玄武岩原石".to_string(),
                "塊",
                vec![("wt_cat_0023".to_string(), "精鑿玄武岩基石".to_string(), "方", 1)],
                vec![("wt_cat_0023".to_string(), "精鑿玄武岩基石".to_string(), "方", 2)],
                "ore",
                "workshop",
                "三號坑口".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}將堅硬玄武岩鑿成防風屋基石與船錨", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "石材紋理與灰窯火候".to_string(),
            )
        } else if ent.display_title.contains("大廚")
            || ent.display_title.contains("食堂")
            || ent.display_title.contains("攤")
            || ent.display_title.contains("肉")
            || ent.id.starts_with("wt_vendor_")
            || sched_bldg.contains("食堂")
        {
            (
                format!("{}的料理灶台", sched_bldg),
                "wt-good-009".to_string(),
                "精碾白米".to_string(),
                "袋",
                vec![("wt_cat_0437".to_string(), "老麵蔥油大餅".to_string(), "份", 2)],
                vec![("wt_cat_0437".to_string(), "老麵蔥油大餅".to_string(), "份", 2)],
                "grain",
                "canteen",
                "大圳頭".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}照看爐灶火候、備妥新鮮熱食點心", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "半島四時食材與灶火成色".to_string(),
            )
        } else {
            (
                format!("{}的當值案台", sched_bldg),
                "wt_cat_0781".to_string(),
                "竹綿紙記事手札".to_string(),
                "冊",
                vec![("wt_cat_0786".to_string(), "核訖當值清冊".to_string(), "卷", 1)],
                vec![("wt_cat_0786".to_string(), "核訖當值清冊".to_string(), "卷", 1)],
                "shop",
                "shop",
                "文曜".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}踏實料理手頭差事", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "半島百工器物與材料分寸".to_string(),
            )
        };

        let delivery_vessel_desc = if waist_id == "wt_cat_0897" || has_mill_factor {
            "薄層重礦烏木膽溫存桶".to_string()
        } else {
            "隨身背簍與貨囊".to_string()
        };

        let cur_pos = ent.grid_x.zip(ent.grid_y).unwrap_or((sched_x, sched_y));
        let work_coord = if has_mill_factor { (-3, 125) } else { (sched_x, sched_y) };
        let work_bldg = if has_mill_factor {
            "老堰頭・水輪石磨工坊".to_string()
        } else {
            sched_bldg.to_string()
        };

        let fb_deliver_coord = if has_mill_factor || settlement == "文曜" || settlement.is_empty() {
            (-4, 4)
        } else {
            cur_pos
        };
        let fb_deliver_bldg = if has_mill_factor || settlement == "文曜" || settlement.is_empty() {
            "文曜・三水大食堂".to_string()
        } else {
            format!("{settlement}・市集鋪面")
        };
        let (deliver_coord, deliver_bldg) = resolve_nearest_room_by_tag(
            cur_pos,
            &[need_buyer_tag, "canteen", "shop", "market"],
            None,
            fb_deliver_coord,
            &fb_deliver_bldg,
        );

        let fb_floodgate_coord = if settlement == "老堰頭" { (-1, 124) } else { (sched_x, sched_y) };
        let fb_floodgate_bldg = if settlement == "老堰頭" {
            "老堰頭・清泉水脈"
        } else {
            sched_bldg
        };
        let (floodgate_coord, floodgate_bldg) = resolve_nearest_room_by_tag(
            cur_pos,
            &["sluice", "water", "waterfront"],
            None,
            fb_floodgate_coord,
            fb_floodgate_bldg,
        );

        let floodgate_facility = if has_mill_factor {
            "一號分洪木閘，調緩入渠水勢以護住重礦大石磨與水青岡木齒".to_string()
        } else {
            format!("{}周邊溝渠分洪閘板與排水道", sched_bldg)
        };
        let fb_canteen_coord = if settlement == "老堰頭" {
            (1, 124)
        } else if settlement != "文曜" && !settlement.is_empty() {
            cur_pos
        } else {
            (-4, 4)
        };
        let fb_canteen_bldg = if settlement == "老堰頭" {
            "老堰頭・堰渠水畔食堂".to_string()
        } else if settlement != "文曜" && !settlement.is_empty() {
            format!("{settlement}・食堂")
        } else {
            "文曜・三水大食堂".to_string()
        };
        let (canteen_coord, canteen_bldg) = resolve_nearest_room_by_tag(
            cur_pos,
            &["canteen", "hot_food", "fresh_drink"],
            None,
            fb_canteen_coord,
            &fb_canteen_bldg,
        );
        let (home_coord, home_bldg) = if has_mill_factor {
            ((-3, 124), "老堰頭・秦氏水輪石磨傳承老宅".to_string())
        } else {
            let slot = crate::store::wenyao_household_slot_for(&ent.id);
            if ent.id == slot.name() || ent.display_title.contains(slot.name()) {
                ((slot.home_x, slot.home_y), slot.home_name.to_string())
            } else if settlement != "文曜" && !settlement.is_empty() {
                (cur_pos, format!("{settlement}・居民居所"))
            } else {
                ((slot.home_x, slot.home_y), slot.home_name.to_string())
            }
        };

        Self {
            tool_id,
            tool_name,
            is_bladed,
            secondary_tool_id,
            secondary_tool_name,
            maintenance_station,
            raw_item_id,
            raw_item_name,
            raw_item_unit,
            raw_consume_qty: 1,
            outputs,
            delivery_items,
            delivery_payout_mg: 16,
            delivery_vessel_desc,
            need_supply_tag,
            need_buyer_tag,
            supplier_id: String::new(),
            supplier_place_label,
            supplier_coord: None,
            restock_qty: 6,
            restock_cost_mg: 12,
            counterparty_id: String::new(),
            work_coord,
            deliver_coord,
            floodgate_coord,
            canteen_coord,
            home_coord,
            work_bldg,
            deliver_bldg,
            floodgate_bldg,
            floodgate_facility,
            canteen_bldg,
            home_bldg,
            settlement,
            work_activity_desc,
            anatomy_familiarity_subject,
        }
    }
}

fn is_grudge_hook(h: &crate::npc::causal_core::RelationalHook) -> bool {
    ["遭拒", "追討", "結怨", "嫌隙", "展期"]
        .iter()
        .any(|k| h.kind_label.contains(k) || h.summary.contains(k))
}

fn is_favor_hook(h: &crate::npc::causal_core::RelationalHook) -> bool {
    ["恩", "守諾", "解囊", "馳援", "完修", "交割", "供料"]
        .iter()
        .any(|k| h.kind_label.contains(k) || h.summary.contains(k))
}

