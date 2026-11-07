fn normalize_room_key(room_id: &str) -> String {
    if let Some(coord) = crate::grid::parse_grid_room_id(room_id) {
        if let Some(cell) = crate::server::grid_manager::get_runtime_square_grid()
            .and_then(|g| g.get(coord).cloned())
        {
            let name = cell.name.trim();
            if !name.is_empty() {
                return name.replace("文曜・文曜", "文曜・").replace("文曜文曜", "文曜");
            }
        }
    }
    room_id.replace("文曜・文曜", "文曜・").replace("文曜文曜", "文曜")
}

/// 取得指定房間當前活躍的野地營火狀態。
pub fn get_active_campfire(room_id: &str) -> Option<CampfireState> {
    let norm = normalize_room_key(room_id);
    let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
    ledger
        .active_campfires
        .get(&norm)
        .or_else(|| ledger.active_campfires.get(room_id))
        .cloned()
}
/// 點燃或加添野地營火。
pub fn ignite_or_stoke_campfire(
    pid: &str,
    room_id: &str,
    preferred_item_id: Option<&str>,
    now_unix: i64,
) -> Result<(String, bool), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    let fuel_candidates = [
        "mat_kindling_bundle",
        "mat_timber_log",
        "wt_timber_cedar",
        "mat_charcoal_sack",
        "mat_wood_offcut_bundle",
        "mat_reed_bundle",
        "wt_cat_0089",
        "wt_cat_0090",
        "wt_cat_0091",
    ];

    let mut consumed_item = None;
    if let Some(pref) = preferred_item_id {
        if db::inventory_has_item(&ent.inventory, pref) {
            consumed_item = Some(pref.to_string());
        }
    }

    if consumed_item.is_none() {
        for candidate in fuel_candidates {
            if db::inventory_has_item(&ent.inventory, candidate) {
                consumed_item = Some(candidate.to_string());
                break;
            }
        }
    }
    if consumed_item.is_none() {
        let parsed_inv = db::get_inventory(&ent.inventory, ent.vit);
        for it in &parsed_inv.items {
            if it.qty > 0
                && let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(&it.item_id)
                && matches!(cat.category_num.trim(), "02" | "2")
            {
                consumed_item = Some(it.item_id.clone());
                break;
            }
        }
    }

    let is_charcoal = consumed_item.as_deref() == Some("mat_charcoal_sack");
    let fuel_name = if let Some(it) = &consumed_item {
        let _ = db::remove_from_inventory(pid, it, 1);
        get_clean_item_name(it)
    } else {
        "拾撿枯枝".to_string()
    };

    let norm_room = normalize_room_key(room_id);
    let is_new_fire = {
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        let was_new = if let Some(cf) = ledger.active_campfires.get_mut(&norm_room) {
            cf.fuel_level = (cf.fuel_level + 2).min(5);
            cf.is_charcoal = cf.is_charcoal || is_charcoal;
            cf.updated_at = now_unix;
            false
        } else {
            let state = CampfireState {
                fuel_level: 3,
                is_charcoal,
                updated_at: now_unix,
            };
            ledger.active_campfires.insert(norm_room.clone(), state.clone());
            if norm_room != room_id {
                ledger.active_campfires.insert(room_id.to_string(), state);
            }
            true
        };

        let ev = if was_new {
            format!("荒野旅人在【{norm_room}】引燃了野地營火堆，暖光照耀四方。")
        } else {
            format!("【{norm_room}】的野地營火添柴增旺，柴煙裊裊，熱力充沛。")
        };
        ledger.recent_events.push_back(ev);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
        was_new
    };

    if let Some(arc) = store::get_store() {
        let _ = arc.write().update_entity(pid, |e| {
            e.vit = (e.vit + 12).min(100);
            e.qi = (e.qi + 12).min(100);
        });
    }
    db::log_npc_event(now_unix, pid, "ignite_campfire", room_id);

    let narrative = if is_new_fire {
        format!(
            "你取用【{fuel_name}】引燃火星，在空地中央架起通風柴堆。隨著青煙升騰，金紅烈焰劈啪燃起，生起了一座溫暖明亮的〔野地營火堆〕！周遭寒濕之氣為之一空，融融暖意順著掌心湧入經脈，體魄與元氣大受滋養。"
        )
    } else {
        format!(
            "你將【{fuel_name}】添入營火核心，原本微暗的炭火騰躍起金紅火舌，熱浪撲面而來，火勢更為旺盛，體魄與心神元氣大受溫養。"
        )
    };

    Ok((narrative, is_new_fire))
}

/// 嘗試烹調《白塔山海誌》獵獲與特產名物之合味珍餚：
/// 檢測玩家行囊是否持有山海食材與半島特色佐料，若有則進行料理並恢復體魄元氣。
/// 回傳 `Some((narrative, total_vit, total_qi))`。
pub fn try_camp_shanhai_cooking(
    actor_id: &str,
    venue_label: &str,
    now_unix: i64,
) -> Option<(String, i32, i32)> {
    let ent = db::get_entity(actor_id).ok().flatten()?;

    // (ingredient_id, dish_name, condiment_id, condiment_name, base_vit, base_qi)
    const SHANHAI_RECIPES: &[(&str, &str, &str, &str, i32, i32)] = &[
        ("wt_cat_0205", "老薑黑糖麻油炙月熊掌", "wt_cat_0364", "熬製古法黑糖塊", 30, 24),
        ("wt_cat_0224", "壺底蔭油紅燒銀鱗土魠", "wt_cat_0341", "陶缸日曬黑豆壺底蔭油", 26, 20),
        ("wt_cat_0223", "馬告清燉野生九孔鮑湯", "wt_cat_0118", "山胡椒馬告粒", 26, 22),
        ("wt_cat_0236", "甘醇蔭油膏爆炒墨玉花枝", "wt_cat_0342", "手工甘醇蔭油膏", 24, 20),
        ("wt_cat_0203", "馬告炭炙高地水鹿腱", "wt_cat_0118", "山胡椒馬告粒", 26, 18),
        ("wt_cat_0209", "陳年老菜脯慢燉山羌盅", "wt_cat_0370", "甕藏陳年老菜脯", 25, 20),
        ("wt_cat_0269", "清溪活水黃金蜆", "wt_cat_0341", "陶缸日曬黑豆壺底蔭油", 22, 18),
        ("wt_cat_0270", "老薑麻油烏魚米粉鍋", "wt_cat_0364", "熬製古法黑糖塊", 24, 18),
    ];

    for &(ing_id, dish_name, cond_id, cond_name, base_vit, base_qi) in SHANHAI_RECIPES {
        if db::inventory_has_item(&ent.inventory, ing_id) {
            let ing_name = get_clean_item_name(ing_id);
            let _ = db::remove_from_inventory(actor_id, ing_id, 1);

            let has_cond = db::inventory_has_item(&ent.inventory, cond_id);
            let (extra_bonus, cond_narrative, mat_names) = if has_cond {
                let _ = db::remove_from_inventory(actor_id, cond_id, 1);
                (
                    4,
                    format!("你更取出行囊中珍藏的【{}】下鍋調味，特產甘醇芳香四溢", cond_name),
                    vec![ing_name.clone(), cond_name.to_string()],
                )
            } else {
                (
                    0,
                    format!("你順手採集「{}」周遭備妥的在地佐料調和入味", venue_label),
                    vec![ing_name.clone(), "在地佐料".to_string()],
                )
            };

            crate::pg::living_world::record_breath_activity(
                actor_id,
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                2,
                now_unix,
            );

            let craft_dims = crate::server::handler::entity_total_craft_dims(actor_id);
            let culinary_bonus = if craft_dims.culinary >= 0.6 { 5 } else { 0 };

            let total_vit = base_vit + extra_bonus + culinary_bonus;
            let total_qi = base_qi + extra_bonus + culinary_bonus;

            if let Some(arc) = store::get_store() {
                let _ = arc.write().update_entity(actor_id, |e| {
                    e.vit = (e.vit + total_vit).min(100);
                    e.qi = (e.qi + total_qi).min(100);
                });
            }

            let actor_name = if !ent.display_title.trim().is_empty() {
                ent.display_title.trim().to_string()
            } else {
                "荒野行旅".to_string()
            };

            let tool_familiarity = if craft_dims.culinary >= 0.6 {
                "沉穩趁手".to_string()
            } else {
                "得心應手".to_string()
            };

            let packet = crate::ai::CraftActionFactPacket {
                actor_name,
                tool_name: "行旅銅釜".to_string(),
                tool_familiarity,
                breath_habit: "潮汐連息".to_string(),
                material_names: mat_names,
                output_name: dish_name.to_string(),
                highlight_kind: "山海營火合味現煮".to_string(),
                environment_hint: venue_label.to_string(),
            };
            let highlight_prose = crate::ai::compose_craft_highlight_from_data(&packet);

            let resonance_note = if craft_dims.culinary >= 0.6 {
                " 【庖廚火候】你刀工與火候拿捏得宜，收汁與調味恰到好處。"
            } else {
                ""
            };

            let narrative = if highlight_prose.is_empty() {
                format!(
                    "【山海合味・{}】你在「{}」旁支起鍋鑊，取出獵獲名物【{}】細心炙烹，{}，精心料理成一道香氣撲鼻的盛宴名饌「{}」！熱氣騰騰中大快朵頤，濃郁滋味透徹肺腑，渾身百骸如沐暖陽，體魄大受滋養、心神元氣充盈。{}",
                    dish_name, venue_label, ing_name, cond_narrative, dish_name, resonance_note
                )
            } else {
                format!(
                    "【山海合味・{}】你在「{}」旁支起鍋鑊，取出獵獲名物【{}】細心炙烹，{}，精心料理成一道香氣撲鼻的盛宴名饌「{}」！熱氣騰騰中大快朵頤，濃郁滋味透徹肺腑，渾身百骸如沐暖陽，體魄大受滋養、心神元氣充盈。{}\n\n{}",
                    dish_name, venue_label, ing_name, cond_narrative, dish_name, resonance_note, highlight_prose
                )
            };
            return Some((narrative, total_vit, total_qi));
        }
    }

    None
}

/// 保持相容：嘗試烹調《白塔山海誌》獵獲與特產名物之合味珍餚。
pub fn try_cook_shanhai_delicacy(pid: &str, venue_label: &str) -> Option<(String, i32, i32)> {
    try_camp_shanhai_cooking(pid, venue_label, crate::game::now_unix())
}

/// 與野地營火堆的多重插座互動。
pub fn interact_with_campfire(
    pid: &str,
    room_id: &str,
    action: &str,
    now_unix: i64,
) -> Result<String, String> {
    match action {
        "WarmHands" => {
            if let Some(arc) = store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 10).min(100);
                    e.qi = (e.qi + 12).min(100);
                });
            }
            Ok("你湊近營火伸出雙手，融融暖流自掌心順著經絡徐徐流轉周身，荒野長途奔波的疲憊寒濕隨之消散大半，整個人神清氣爽。".to_string())
        }
        "RoastFood" => {
            if let Some((shanhai_narr, _v, _q)) = try_camp_shanhai_cooking(pid, "野地營火堆", now_unix) {
                return Ok(shanhai_narr);
            }
            if let Some(arc) = store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 20).min(100);
                    e.qi = (e.qi + 10).min(100);
                });
            }
            Ok("你取出行囊中的乾糧肉脯架在營火紅炭上方細心炙烤，外焦裡嫩、油脂滋滋作響，趁熱食用後腹中溫熱充實，精氣神大振。".to_string())
        }
        "CraftTorch" => {
            let Ok(Some(ent)) = db::get_entity(pid) else {
                return Err("角色不存在".to_string());
            };
            if db::inventory_has_item(&ent.inventory, "mat_kindling_bundle") {
                let _ = db::remove_from_inventory(pid, "mat_kindling_bundle", 1);
            } else if db::inventory_has_item(&ent.inventory, "mat_timber_log") {
                let _ = db::remove_from_inventory(pid, "mat_timber_log", 1);
            }
            let _ = db::add_to_inventory(pid, "craft_pine_torch", 1);
            Ok("你挑選了一根乾燥堅實的硬柴浸塗松脂，伸入營火中引燃並纏裹麻絲封固，製成一柄劈啪燃燒、可用於荒野夜行照明驅獸的【浸脂松明火把】收入行囊。".to_string())
        }
        "AddFuel" => {
            let (narrative, _) = ignite_or_stoke_campfire(pid, room_id, None, now_unix)?;
            Ok(narrative)
        }
        "CollectCharcoal" => {
            let norm_room = normalize_room_key(room_id);
            let had_fire = {
                let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
                let r1 = ledger.active_campfires.remove(&norm_room).is_some();
                let r2 = ledger.active_campfires.remove(room_id).is_some();
                r1 || r2
            };
            if !had_fire {
                return Err("此處營火已無餘熱，無法封收木炭。".to_string());
            }
            let _ = db::add_to_inventory(pid, "mat_charcoal_sack", 2);
            Ok("你用長枝撥開營火外層浮灰，將深處熾熱通紅的硬木火炭妥善封土覆泥隔氧熄滅，待其冷卻凝結後，收獲了兩袋熱值極高、敲擊有金屬聲的【硬木白炭袋】收入行囊。".to_string())
        }
        _ => Err(format!("營火無法執行此項操作：{action}")),
    }
}

/// 木作五向精煉多插座派發。
pub fn refine_wood_multi_socket(
    pid: &str,
    mode: &str,
    _location_hint: &str,
    now_unix: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    let inv = db::get_inventory(&ent.inventory, ent.vit);
    let wood_candidates = [
        "mat_timber_log",
        "wt_timber_cedar",
        "mat_kindling_bundle",
        "mat_plank_board",
        "mat_wood_offcut_bundle",
        "harvest_wood",
        "wt_cat_0089",
        "wt_cat_0090",
        "wt_cat_0091",
    ];
    let mut consumed = false;
    for w in wood_candidates {
        if db::inventory_has_item(&ent.inventory, w) {
            let _ = db::remove_from_inventory(pid, w, 1);
            consumed = true;
            break;
        }
    }
    if !consumed {
        if let Some(it) = inv.items.iter().find(|it| is_wood_or_firewood_item(&it.item_id)) {
            let _ = db::remove_from_inventory(pid, &it.item_id, 1);
            consumed = true;
        }
    }
    if !consumed {
        return Err("行囊中尚無可供加工的木竹原材。".to_string());
    }

    let (prod_id, prod_name, qty, narrative) = match mode {
        "SplitWood" => (
            "mat_kindling_bundle".to_string(),
            "乾燥柴枝束".to_string(),
            2,
            "你手起斧落順著木紋順暢剖劈，將木料劈裂分開，紮成兩束易於引燃的【乾燥柴枝束】收入行囊。".to_string(),
        ),
        "RefinePlank" => (
            "mat_plank_board".to_string(),
            "樟松木板料".to_string(),
            2,
            "你拉動雙手長鋸順紋裁鋸刨切，將原木加工為兩塊平整堅實、不易變形的【樟松木板料】收入行囊。".to_string(),
        ),
        "BurnCharcoal" => (
            "mat_charcoal_sack".to_string(),
            "硬木白炭袋".to_string(),
            1,
            "你將硬木截段架入土窯封閉泥封，慢火悶燒凝煉，成功製成一袋成色純淨、熱值極高的【硬木白炭袋】收入行囊。".to_string(),
        ),
        "CraftTorch" => (
            "craft_pine_torch".to_string(),
            "浸脂松明火把".to_string(),
            1,
            "你挑選油脂豐富的松木枝條塗抹脂膏緊密紮裹，製成一柄照明驅寒的【浸脂松明火把】收入行囊。".to_string(),
        ),
        "CraftHandle" => (
            "craft_wooden_tool_handle".to_string(),
            "堅韌硬木工具長柄".to_string(),
            1,
            "你用木工推刀細細車削刨光，打磨出一根手感沉穩堅韌的【堅韌硬木工具長柄】收入行囊。".to_string(),
        ),
        _ => return Err(format!("未知的木作精煉模式：{mode}")),
    };

    let _ = db::add_to_inventory(pid, &prod_id, qty);
    db::log_npc_event(now_unix, pid, "refine_wood", &prod_name);
    Ok((prod_id, prod_name, narrative))
}

/// 工坊高爐添柴加溫。
pub fn stoke_workshop_furnace(
    pid: &str,
    room_id: &str,
    now_unix: i64,
) -> Result<String, String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    let (fuel_name, heat_level) = if db::inventory_has_item(&ent.inventory, "mat_charcoal_sack") {
        let _ = db::remove_from_inventory(pid, "mat_charcoal_sack", 1);
        ("硬木白炭", 3)
    } else if db::inventory_has_item(&ent.inventory, "mat_kindling_bundle") {
        let _ = db::remove_from_inventory(pid, "mat_kindling_bundle", 1);
        ("乾燥柴枝束", 2)
    } else if db::inventory_has_item(&ent.inventory, "mat_timber_log") {
        let _ = db::remove_from_inventory(pid, "mat_timber_log", 1);
        ("伐採原木", 2)
    } else {
        ("周遭柴薪", 2)
    };

    {
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        ledger.room_furnace_heat.insert(room_id.to_string(), heat_level);
        let rname = db::get_room_name(room_id)
            .ok()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "當地工坊".to_string());
        let ev = format!("【{rname}】添柴入爐，爐溫飆升至白熾狀態，百工造物異變機率大幅提升。");
        if !ledger.recent_events.iter().any(|e| e == &ev) {
            ledger.recent_events.push_back(ev);
            if ledger.recent_events.len() > 30 {
                ledger.recent_events.pop_front();
            }
        }
    }
    db::log_npc_event(now_unix, pid, "stoke_furnace", room_id);

    Ok(format!(
        "你將【{fuel_name}】添入高溫工爐核心，狂猛的熱浪夾雜著金紅烈芒瞬間騰躍而起，爐膛劇烈咆哮、火溫轉為純白熾熱，工坊已蓄勢待發，鍛造與淬火異變之氣達至鼎盛！"
    ))
}

/// 修補房間內的損耗設施與建築構件。
pub fn repair_room_structure(
    pid: &str,
    room_id: &str,
    target_name: &str,
    now_unix: i64,
) -> Result<String, String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    let repair_materials = [
        "mat_plank_board",
        "craft_wooden_tool_handle",
        "mat_timber_log",
        "mat_repair_strip",
        "mat_hemp_rope_roll",
    ];

    let mut used_item = None;
    for mat in repair_materials {
        if db::inventory_has_item(&ent.inventory, mat) {
            let _ = db::remove_from_inventory(pid, mat, 1);
            used_item = Some(get_clean_item_name(mat));
            break;
        }
    }

    let mat_desc = used_item.unwrap_or_else(|| "就地取材之構件".to_string());

    {
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        ledger.room_structure_repairs.insert(room_id.to_string(), now_unix);
        let ev = format!("白塔半島設施【{target_name}】經巧手工匠修繕穩固。");
        ledger.recent_events.push_back(ev);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
    }

    let inferred = crate::world::materials::infer_origin_settlement(room_id);
    let settlement = if inferred.is_empty() { "文曜" } else { inferred };
    let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, &settlement, 8);
    db::log_npc_event(now_unix, pid, "repair_structure", target_name);

    Ok(format!(
        "你取出【{mat_desc}】細心楔合修補「{target_name}」的破損榫眼與鬆動踏板，敲釘抹膠固定妥當。破損處重歸平整穩固，市場營造司為此核發當班津貼入行囊（設施修復穩妥，資財獲得結算補貼）。"
    ))
}

/// 判斷物品是否屬於礦石、金屬、修件或鍛造料件。
pub fn is_mineral_item(item_id: &str) -> bool {
    if matches!(
        item_id,
        "mat_iron_scrap"
            | "mat_repair_strip"
            | "mat_rough_ore"
            | "mat_washed_ore"
            | "mat_stone_powder_sack"
            | "mat_metal_rim_strip"
            | "mat_iron_scrap_box"
            | "mat_flint_stone"
            | "craft_steel_hunting_knife"
            | "craft_iron_pickaxe"
    ) {
        return true;
    }
    if item_id.starts_with("craft_") || item_id.starts_with("mut_") {
        return false;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            return (1..=55).contains(&num);
        }
    }
    item_id.contains("ore")
        || item_id.contains("iron")
        || item_id.contains("stone")
        || item_id.contains("metal")
        || item_id.contains("flint")
        || item_id.contains("steel")
        || item_id.contains("pickaxe")
}

/// 判斷物品是否屬於黏土、紅磚或瓦陶料件。
pub fn is_clay_brick_item(item_id: &str) -> bool {
    if matches!(
        item_id,
        "mat_stone_powder_sack" | "wt_cat_0006" | "wt_cat_0023" | "wt_cat_0677"
    ) {
        return true;
    }
    item_id.contains("clay")
        || item_id.contains("brick")
        || item_id.contains("pottery")
        || item_id.contains("ceramic")
}

/// 將物品分類為五源字牌產出來源與素材品階 (WordSource, tier 2..=4)。
pub fn classify_material_word_source(
    item_id: &str,
    item_name: &str,
) -> Option<(crate::combat::wordplate::WordSource, u8)> {
    use crate::combat::wordplate::WordSource;

    if item_id.starts_with("crystal_")
        || item_id.starts_with("starter_")
        || item_id.starts_with("key_")
    {
        return None;
    }
    if item_id.starts_with("corpse_") {
        let tier = crate::pg::living_world::get_beast_corpse(item_id).map(|r| r.tier).unwrap_or(4);
        return Some((WordSource::Beast, tier));
    }

    // 1. 獸源 (Beast)
    let is_beast_cat = if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            (166..=220).contains(&num)
        } else {
            false
        }
    } else {
        false
    };

    const BEAST_KEYWORDS: &[&str] = &[
        "皮", "毛", "羽", "翎", "掌", "鱗", "麟", "蛻", "角", "牙", "肉", "骨", "脂",
        "鹿", "豹", "熊", "虎", "豬", "牛", "羊", "羌", "兔", "鼠", "鷹", "雉", "鷴", "鱷", "蛇", "鯪鯉",
    ];

    let is_aquatic_override = item_name.contains("魚")
        || item_name.contains("鯊")
        || item_name.contains("鯨")
        || item_name.contains("烏賊")
        || item_name.contains("花枝")
        || item_name.contains("鮑")
        || item_name.contains("蝦")
        || item_name.contains("蟹");
    let is_plant_override = item_name.contains("牛樟")
        || item_name.contains("靈芝")
        || item_name.contains("草藥")
        || is_herb_item(item_id)
        || is_wood_or_firewood_item(item_id);
    let has_beast_kw = !is_aquatic_override
        && !is_plant_override
        && BEAST_KEYWORDS
            .iter()
            .any(|&kw| item_name.contains(kw) || item_id.contains(kw));

    if is_beast_cat || has_beast_kw {
        const APEX_BEAST_KW: &[&str] = &[
            "雲豹", "月熊", "穿山甲", "鯪鯉", "百步蛇", "巨鱷", "帝雉", "藍腹鷴", "熊鷹", "大角原牛", "水鹿",
        ];
        const MED_BEAST_KW: &[&str] = &["石虎", "山羊", "野豬", "白鼻心"];

        let tier = if APEX_BEAST_KW.iter().any(|&kw| item_name.contains(kw) || item_id.contains(kw)) {
            4
        } else if MED_BEAST_KW.iter().any(|&kw| item_name.contains(kw) || item_id.contains(kw)) {
            3
        } else {
            2
        };
        return Some((WordSource::Beast, tier));
    }

    // 2. 水源 (Water)
    let is_water_cat = if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            (221..=276).contains(&num)
        } else {
            false
        }
    } else {
        false
    };

    const WATER_KEYWORDS: &[&str] = &[
        "魚", "鮑", "九孔", "烏賊", "花枝", "墨囊", "鯊", "鯨", "海", "蝦", "蛤", "貝",
        "蟹", "螺", "藻", "水", "泉", "露", "冰", "膠", "潮",
    ];

    let has_water_kw = WATER_KEYWORDS
        .iter()
        .any(|&kw| item_name.contains(kw) || item_id.contains(kw));

    if is_water_cat || has_water_kw {
        const APEX_WATER_KW: &[&str] = &["大花枝", "豆腐鯊", "土魠", "九孔鮑", "龍魚"];
        let tier = if APEX_WATER_KW.iter().any(|&kw| item_name.contains(kw) || item_id.contains(kw)) {
            4
        } else {
            2
        };
        return Some((WordSource::Water, tier));
    }

    // 3. 礦源 (Mineral)
    const MINERAL_KEYWORDS: &[&str] = &[
        "礦", "鐵", "石", "岩", "鹽", "煤", "硫", "砂", "泥", "陶", "磚", "灰", "金", "銅", "晶", "鍛", "錠",
    ];

    let has_mineral_kw = MINERAL_KEYWORDS
        .iter()
        .any(|&kw| item_name.contains(kw) || item_id.contains(kw));

    if is_mineral_item(item_id) || is_clay_brick_item(item_id) || has_mineral_kw {
        const RARE_MINERAL_KW: &[&str] = &["玄武", "硫磺", "精鐵", "黑曜", "鹽晶"];
        let tier = if RARE_MINERAL_KW.iter().any(|&kw| item_name.contains(kw) || item_id.contains(kw)) {
            3
        } else {
            2
        };
        return Some((WordSource::Mineral, tier));
    }

    // 4. 植源 (Plant)
    const PLANT_KEYWORDS: &[&str] = &[
        "樟", "芝", "愛玉", "馬告", "菜脯", "蔭油", "黑糖", "草", "葉", "木", "板", "柴",
        "炭", "竹", "蘆", "麻", "繩", "布", "紙", "茶", "果", "稻", "穀", "菇", "藤", "花",
        "根", "藥", "薯", "豆", "薑",
    ];

    let has_plant_kw = PLANT_KEYWORDS
        .iter()
        .any(|&kw| item_name.contains(kw) || item_id.contains(kw));

    if is_wood_or_firewood_item(item_id)
        || is_herb_item(item_id)
        || is_fiber_item(item_id)
        || has_plant_kw
    {
        let tier = if item_name.contains("牛樟芝") || item_id.contains("牛樟芝") {
            4
        } else if ["愛玉", "馬告", "老菜脯", "蔭油", "老薑"]
            .iter()
            .any(|&kw| item_name.contains(kw) || item_id.contains(kw))
        {
            3
        } else {
            2
        };
        return Some((WordSource::Plant, tier));
    }

    // 5. 氣源 (Qi) 或 Fallback
    Some((WordSource::Qi, 2))
}


/// 判斷物品是否屬於草藥、煎膏或茶包。
pub fn is_herb_item(item_id: &str) -> bool {
    if item_id.starts_with("craft_") || item_id.starts_with("mut_") {
        return false;
    }
    if matches!(
        item_id,
        "mat_foraged_herb"
            | "harvest_herb"
            | "mat_wet_grass_bundle"
            | "mat_water_moss_sheet"
            | "mat_sealing_pitch"
    ) {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            return (111..=165).contains(&num);
        }
    }
    item_id.contains("herb")
        || item_id.contains("moss")
        || item_id.contains("cress")
        || item_id.contains("root")
}

/// 判斷物品是否屬於纖維、麻繩、紙張或布料。
pub fn is_fiber_item(item_id: &str) -> bool {
    if item_id.starts_with("craft_") || item_id.starts_with("mut_") {
        return false;
    }
    if matches!(
        item_id,
        "mat_reed_bundle"
            | "mat_hemp_rope_roll"
            | "mat_sackcloth_bolt"
            | "mat_coarse_paper_ream"
            | "mat_canvas_roll"
            | "mat_oilcloth_roll"
            | "mat_wrapping_paper_pack"
            | "mat_fishing_line_bundle"
            | "mat_rice_straw_bundle"
    ) {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            if (56..=110).contains(&num) {
                let name = crate::game::peninsula_catalog::find_catalog_entry(item_id)
                    .map(|e| e.name)
                    .unwrap_or("");
                return name.contains("麻")
                    || name.contains("藤")
                    || name.contains("竹")
                    || name.contains("蘆")
                    || name.contains("草")
                    || name.contains("蒲")
                    || name.contains("棕")
                    || name.contains("樹皮")
                    || matches!(num, 58 | 59 | 64 | 66 | 68 | 69 | 74 | 75 | 78 | 81 | 83 | 85 | 87 | 88 | 91 | 94 | 96 | 99 | 104);
            }
        }
    }
    item_id.contains("reed")
        || item_id.contains("rope")
        || item_id.contains("sackcloth")
        || item_id.contains("paper")
        || item_id.contains("canvas")
        || item_id.contains("oilcloth")
        || item_id.contains("flax")
        || item_id.contains("fiber")
}

/// 判斷物品是否屬於生皮、水產、魚骨或貝類。
pub fn is_hide_fish_item(item_id: &str) -> bool {
    if item_id.starts_with("craft_") || item_id.starts_with("mut_") {
        return false;
    }
    if matches!(
        item_id,
        "mat_leather_strip"
            | "mat_fish_bone_sack"
            | "mat_fish_scale_sack"
            | "mat_shell_sack"
            | "mat_mane_hair_bundle"
    ) {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            return (166..=276).contains(&num);
        }
    }
    item_id.contains("leather")
        || item_id.contains("fish")
        || item_id.contains("shell")
        || item_id.contains("pelt")
        || item_id.contains("hide")
        || item_id.contains("bone")
}

/// 礦鐵金石多插座精煉派發：支援 WashOre, SmeltBloom, ForgeRepairStrip, SharpenBlade, BuildFieldAnvil
pub fn refine_mineral_multi_socket(
    pid: &str,
    mode: &str,
    room: &str,
    now: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    let inv = db::get_inventory(&ent.inventory, ent.vit);
    let consume_mineral = |candidates: &[&str]| -> bool {
        for &c in candidates {
            if db::inventory_has_item(&ent.inventory, c) {
                let _ = db::remove_from_inventory(pid, c, 1);
                return true;
            }
        }
        if let Some(it) = inv.items.iter().find(|it| is_mineral_item(&it.item_id)) {
            let _ = db::remove_from_inventory(pid, &it.item_id, 1);
            return true;
        }
        false
    };

    match mode {
        "WashOre" => {
            if !consume_mineral(&["mat_rough_ore", "mat_iron_scrap", "mat_stone_powder_sack", "mat_washed_ore"]) {
                return Err("行囊中尚無可供淘洗的礦石料件。".to_string());
            }
            let prod_id = "mat_washed_ore".to_string();
            let prod_name = "洗選精礦石".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_mineral", &prod_name);
            Ok((prod_id, prod_name, "你將礦料置入澄淨流水竹籃中反覆淘洗，洗去表層頑泥雜質與黏土，篩濾出一籃晶芒隱現的【洗選精礦石】收入行囊。".to_string()))
        }
        "SmeltBloom" => {
            if !consume_mineral(&["mat_washed_ore", "mat_rough_ore", "mat_iron_scrap", "mat_repair_strip"]) {
                return Err("行囊中尚無可供悶燒冶煉的礦鐵料件。".to_string());
            }
            let prod_id = "mat_iron_scrap".to_string();
            let prod_name = "回收熟鐵料塊".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 2);
            db::log_npc_event(now, pid, "refine_mineral", &prod_name);
            Ok((prod_id, prod_name, "你引燃爐炭鼓風增溫，將選洗後的礦石高溫悶燒還原為海綿鐵團，趁熱重錘鍛打擠出爐渣，製成兩塊成色均勻的【回收熟鐵料塊】收入行囊。".to_string()))
        }
        "ForgeRepairStrip" => {
            if !consume_mineral(&["mat_iron_scrap", "mat_metal_rim_strip", "mat_rough_ore"]) {
                return Err("行囊中尚無可供延展鍛打的熟鐵料件。".to_string());
            }
            let prod_id = "mat_repair_strip".to_string();
            let prod_name = "修補熟鐵薄片".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 2);
            db::log_npc_event(now, pid, "refine_mineral", &prod_name);
            Ok((prod_id, prod_name, "你將熟鐵料段置於砧石之上反覆鍛打延展，裁截整飭為兩片平整勻稱、韌性十足的【修補熟鐵薄片】收入行囊。".to_string()))
        }
        "SharpenBlade" => {
            if db::inventory_has_item(&ent.inventory, "mat_iron_scrap") && !db::inventory_has_item(&ent.inventory, "craft_steel_hunting_knife") {
                let _ = db::remove_from_inventory(pid, "mat_iron_scrap", 1);
                let _ = db::add_to_inventory(pid, "craft_steel_hunting_knife", 1);
                db::log_npc_event(now, pid, "refine_mineral", "淬火開鋒精鍛防身獵刀");
                return Ok(("craft_steel_hunting_knife".to_string(), "精鍛防身獵刀".to_string(), "你將熟鐵料塊反覆鍛打成型、蘸水在青石上細心推磨開鋒，寒芒畢露，鍛出一柄【精鍛防身獵刀】收進行囊！".to_string()));
            }
            let candidates = ["craft_steel_hunting_knife", "craft_iron_pickaxe", "mat_repair_strip", "mat_flint_stone"];
            let mut sharpened_id = String::new();
            let mut sharpened = "隨身刃具".to_string();
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    sharpened_id = c.to_string();
                    sharpened = get_clean_item_name(c);
                    break;
                }
            }
            if sharpened_id.is_empty() {
                let parsed_inv = db::get_inventory(&ent.inventory, ent.vit);
                for it in &parsed_inv.items {
                    let cname = get_clean_item_name(&it.item_id);
                    if it.qty > 0 && crate::pg::living_world::is_bladed_vessel_item(&it.item_id, &cname) {
                        sharpened_id = it.item_id.clone();
                        sharpened = cname;
                        break;
                    }
                }
            }
            if !sharpened_id.is_empty() && crate::pg::living_world::is_bladed_vessel_item(&sharpened_id, &sharpened) {
                let _ = crate::pg::living_world::sharpen_vessel_at_grindstone(pid, &sharpened_id, now);
            }
            db::log_npc_event(now, pid, "refine_mineral", "磨礪刃具");
            Ok(("mat_repair_strip".to_string(), "刃面鋒芒".to_string(), format!("你蘸水在油石上細心推磨【{sharpened}】，砂痕層層推勻，刃線重煥清冽寒芒，握持手感愈發沉穩順手。")))
        }
        "BuildFieldAnvil" => {
            let norm_room = normalize_room_key(room);
            let state = FieldFacilityState {
                facility_id: "dyn_field_anvil".to_string(),
                name: "野地修具石砧".to_string(),
                created_at: now,
                expires_at: now + 86400,
            };
            {
                let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
                ledger.active_field_facilities.entry(norm_room.clone()).or_default().push(state.clone());
                if norm_room != room {
                    ledger.active_field_facilities.entry(room.to_string()).or_default().push(state);
                }
                let rname = db::get_room_name(room)
                    .ok()
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| "郊野地界".to_string());
                let ev = format!("【{rname}】有人就地架設了一座平整沉重的【野地修具石砧】，可供往來行者鍛打修治鐵件。");
                if !ledger.recent_events.iter().any(|e| e == &ev) {
                    ledger.recent_events.push_back(ev);
                    if ledger.recent_events.len() > 30 {
                        ledger.recent_events.pop_front();
                    }
                }
            }
            db::log_npc_event(now, pid, "build_field_anvil", room);
            Ok(("dyn_field_anvil".to_string(), "野地修具石砧".to_string(), "你尋來一塊質地緊密堅實的平整玄武巨石，穩穩楔入地面夯實，架設起一座可供敲打鍛修器具的【野地修具石砧】！".to_string()))
        }
        _ => Err(format!("未知的礦鐵精煉模式：{mode}")),
    }
}

