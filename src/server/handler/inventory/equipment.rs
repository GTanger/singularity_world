fn resolve_inv_item_name(item_id: &str) -> String {
    let candidate = get_item_def(item_id)
        .map(|d| d.name)
        .or_else(|| crate::world::materials::material_by_item_id(item_id).map(|m| m.name.to_string()))
        .or_else(|| crate::game::peninsula_catalog::find_catalog_entry(item_id).map(|c| c.name.to_string()))
        .or_else(|| crate::game::object_ecology::lookup_ecology_item_def(item_id).map(|d| d.name))
        .or_else(|| crate::db::fallback_item_name(item_id).map(|s| s.to_string()))
        .filter(|n| !n.trim().is_empty());
    match candidate {
        Some(name) if !name.is_ascii() => name,
        _ => {
            if item_id == "blank_manual" || item_id == "wt_blank_manual" {
                "空白手札".to_string()
            } else if item_id.starts_with("manual_") {
                "傳世手札".to_string()
            } else if item_id.starts_with("corpse_") {
                "異獸殘軀".to_string()
            } else if item_id.starts_with("mut_") {
                "異變奇珍器物".to_string()
            } else {
                "隨身器物".to_string()
            }
        }
    }
}

/// 依據命魂種子（soul_seed）為未通節點或僅有初生核心節點 ["N000"] 之生靈（玩家與真人NPC）
/// 確定性點亮 6–9 個星盤節點（包含 N000，以及 3 個經絡節點 M01..M12、3 個星府節點 G010..G060、2 個星宿節點 S101..S150）
pub fn derive_activated_nodes_from_soul_seed(seed: i64) -> Vec<String> {
    let mut nodes = vec!["N000".to_string()];
    let u = (seed as u64) ^ 0x514E_5353_A5A5_5A5A;

    // 1. 3 個經絡節點 M01..M12
    let m_a = (u % 12) + 1;
    let m_b = ((u.wrapping_shr(7)) % 12) + 1;
    let m_c = ((u.wrapping_shr(14)) % 12) + 1;
    let mut m_set = vec![m_a];
    if !m_set.contains(&m_b) {
        m_set.push(m_b);
    } else {
        let alt = if m_b % 12 + 1 != m_a { m_b % 12 + 1 } else { (m_b + 1) % 12 + 1 };
        m_set.push(alt);
    }
    for cand in [m_c, (m_c % 12) + 1, ((m_c + 1) % 12) + 1] {
        if !m_set.contains(&cand) {
            m_set.push(cand);
            break;
        }
    }
    for m in m_set {
        nodes.push(format!("M{:02}", m));
    }

    // 2. 3 個星府節點 G010..G060（步進 10）
    let g_a = ((u.wrapping_shr(21) % 6) + 1) * 10;
    let g_b = ((u.wrapping_shr(28) % 6) + 1) * 10;
    let g_c = ((u.wrapping_shr(35) % 6) + 1) * 10;
    let mut g_set = vec![g_a];
    if !g_set.contains(&g_b) {
        g_set.push(g_b);
    } else {
        let alt = if (g_b / 10) % 6 + 1 != g_a / 10 { ((g_b / 10) % 6 + 1) * 10 } else { (((g_b / 10) + 1) % 6 + 1) * 10 };
        g_set.push(alt);
    }
    for cand_idx in [g_c / 10, (g_c / 10) % 6 + 1, ((g_c / 10) + 1) % 6 + 1] {
        let cand = cand_idx * 10;
        if !g_set.contains(&cand) {
            g_set.push(cand);
            break;
        }
    }
    for g in g_set {
        nodes.push(format!("G{:03}", g));
    }

    // 3. 2 個星宿節點 S101..S150
    let s_a = 101 + (u.wrapping_shr(42) % 50);
    let s_b = 101 + (u.wrapping_shr(49) % 50);
    let mut s_set = vec![s_a];
    if s_b != s_a {
        s_set.push(s_b);
    } else {
        let alt = if s_b < 150 { s_b + 1 } else { 101 };
        s_set.push(alt);
    }
    for s in s_set {
        nodes.push(format!("S{:03}", s));
    }

    nodes
}

/// 檢查 entity_id 在 PLAYER_CASKETS 中是否有針對 item_id 之銘文，若有則綴上 ` [字1][字2]`
pub fn format_entity_item_with_inscription(entity_id: &str, item_id: &str, base_name: &str) -> String {
    let caskets = PLAYER_CASKETS.read();
    if let Some(casket) = caskets.get(entity_id) {
        let word_ids = casket.inscribed_words_of(item_id);
        if !word_ids.is_empty() && !base_name.contains('[') {
            let registry = WordRegistry::global();
            let mut suffix = String::from(" ");
            for wid in word_ids {
                let ch = registry
                    .get_by_id(wid)
                    .map(|elem| elem.ch.as_str())
                    .unwrap_or_else(|| {
                        if wid.is_ascii() {
                            "銘"
                        } else {
                            wid.as_str()
                        }
                    });
                suffix.push('[');
                suffix.push_str(ch);
                suffix.push(']');
            }
            return format!("{}{}", base_name, suffix);
        }
    }
    if !base_name.contains('[') && entity_id != "player" {
        match item_id {
            "wt_cat_0984" => return format!("{base_name} [巧][堅]"),
            "wt_cat_0973" => return format!("{base_name} [純][流]"),
            "wt_cat_0945" | "wt_blacksmith_hammer" => return format!("{base_name} [鍛][堅]"),
            "wt_cat_0947" | "wt_carpenter_chisel" => return format!("{base_name} [巧][定]"),
            _ => {}
        }
    }
    base_name.to_string()
}

/// NPC 自主字韻淬礪與器物佩掛：
/// 1. 確保 NPC 初始化於 PLAYER_CASKETS 與 PLAYER_LOADOUTS（基於其 soul_seed 衍生）
/// 2. 自 WordRegistry 依其 soul_seed + 時間抽得一枚字韻並納入詞匣
/// 3. 若所造器物可裝備（slot 非空），則將該字韻烙印其上，並配戴入 equipment_slots
pub fn npc_autonomous_word_and_equip_step(
    npc_id: &str,
    npc_name: &str,
    crafted_item_id: &str,
    crafted_item_name: &str,
) -> Option<String> {
    if npc_id.starts_with("npc_")
        || crafted_item_id.contains("umbrella")
        || crafted_item_id.contains("cloak")
    {
        return None;
    }
    ensure_player_initialized(npc_id);
    let ent_opt = db::get_entity(npc_id).ok().flatten();
    let (vocation_hint, activity_hint) = if let Some(e) = &ent_opt {
        (e.display_title.clone(), e.current_activity.clone())
    } else {
        (String::new(), String::new())
    };
    let seed = ent_opt.as_ref().and_then(|e| e.soul_seed).unwrap_or(0);

    if let Some(ent) = &ent_opt {
        if !npc_id.starts_with("npc_")
            && !npc_id.starts_with("wt_")
            && !npc_id.starts_with("shanhai_")
            && !npc_id.starts_with("ambient_")
            && !inventory_has_item(&ent.inventory, "blank_manual")
            && !inventory_has_item(&ent.inventory, "wt_blank_manual")
            && STARTER_MANUAL_GRANTED.write().insert(npc_id.to_string())
        {
            let _ = db::add_to_inventory(npc_id, "blank_manual", 1);
        }

        let inv = get_inventory(&ent.inventory, ent.vit);
        let corpse_items: Vec<String> = inv
            .items
            .iter()
            .filter(|it| it.item_id.starts_with("corpse_") && it.qty > 0)
            .map(|it| it.item_id.clone())
            .collect();
        let _npc_seed = ent.soul_seed.unwrap_or(0);
        for corpse_id in corpse_items {
            let corpse_rec = crate::pg::living_world::get_beast_corpse(&corpse_id);
            let _ = remove_from_inventory(npc_id, &corpse_id, 1);
            crate::pg::living_world::consume_beast_corpse(&corpse_id);
            let primary_loot = corpse_rec
                .as_ref()
                .map(|r| r.primary_loot_id.as_str())
                .unwrap_or("mat_leather_strip");
            let _ = db::add_to_inventory(npc_id, primary_loot, 1);
            let _ = db::add_to_inventory(npc_id, "wt_beast_bone", 1);
            let ctitle = corpse_rec.as_ref().map(|r| r.creature_title.as_str()).unwrap_or("異獸");
            crate::pg::living_world::record_entity_anatomy_familiarity(npc_id, ctitle, 1);
        }
    }

    let registry = WordRegistry::global();
    let now = crate::game::now_unix();

    // 內息底色累積：依據 NPC 職稱、活動或所造器物特性累積吐納氣機
    let habit_kind = {
        let combined = format!("{vocation_hint} {activity_hint} {crafted_item_name} {crafted_item_id}");
        if combined.contains("鍛")
            || combined.contains("鐵")
            || combined.contains("礦")
            || combined.contains("石")
            || combined.contains("爐")
            || combined.contains("錘")
        {
            crate::pg::living_world::BreathHabitKind::ForgeHeavy
        } else if combined.contains("漁")
            || combined.contains("海")
            || combined.contains("港")
            || combined.contains("磨")
            || combined.contains("渡")
            || combined.contains("烹")
            || combined.contains("煮")
            || combined.contains("食")
        {
            crate::pg::living_world::BreathHabitKind::TideContinuous
        } else if combined.contains("醫")
            || combined.contains("院")
            || combined.contains("書")
            || combined.contains("茶")
            || combined.contains("木")
            || combined.contains("雕")
            || combined.contains("藥")
        {
            crate::pg::living_world::BreathHabitKind::ChamberClear
        } else if combined.contains("衛")
            || combined.contains("武")
            || combined.contains("巡")
            || combined.contains("守")
            || combined.contains("陣")
            || combined.contains("戰")
            || combined.contains("甲")
        {
            crate::pg::living_world::BreathHabitKind::BattleFierce
        } else if combined.contains("獵")
            || combined.contains("林")
            || combined.contains("樵")
            || combined.contains("採")
            || combined.contains("草")
            || combined.contains("山")
            || combined.contains("皮")
        {
            crate::pg::living_world::BreathHabitKind::ForestLong
        } else {
            crate::pg::living_world::get_or_derive_breath_habit(npc_id, &vocation_hint, seed).dominant
        }
    };
    crate::pg::living_world::record_breath_activity(npc_id, habit_kind, 2, now);

    // 人器磨合與自主保養修整：武器與工器具累積使用熟稔度與耗損
    if crate::pg::living_world::is_weapon_or_tool_item(crafted_item_id, crafted_item_name) {
        let vstate = crate::pg::living_world::record_vessel_use(npc_id, crafted_item_id, 1, 3, now);
        if vstate.condition == crate::pg::living_world::VesselCondition::SlightlyDull {
            let _ = crate::pg::living_world::sharpen_vessel_at_grindstone(npc_id, crafted_item_id, now);
        } else if vstate.condition == crate::pg::living_world::VesselCondition::RolledEdge {
            let _ = crate::pg::living_world::temper_vessel_at_forge(npc_id, crafted_item_id, now);
        }
    }

    let roll_seed = (seed as u64).wrapping_add(now as u64);

    let elems: Vec<&WordElement> = registry.ordered_elements().collect();
    if elems.is_empty() {
        return None;
    }
    let elem = elems[(roll_seed as usize) % elems.len()];

    // 納入 1 枚字韻至 NPC 詞匣
    {
        let mut caskets = PLAYER_CASKETS.write();
        if let Some(casket) = caskets.get_mut(npc_id) {
            casket.add(&elem.id, 1);
        }
    }

    let is_equipable = db::get_item_def(crafted_item_id)
        .map(|d| !d.slot.is_empty())
        .unwrap_or(false);

    if !is_equipable {
        save_player_wordplates();
        return None;
    }

    // 防護硬閘門：居民/NPC 僅能配戴與自身本職相符之本命器物，絕不將晴雨傘、行旅斗篷或非本職商品自動頂替身上既有常服與器物
    let phys_class = crate::game::object_ecology::ItemPhysicalClass::classify(
        crafted_item_id,
        crafted_item_name,
    );
    let is_npc_actor = npc_id.starts_with("npc_") || npc_id.starts_with("wt_");
    if is_npc_actor {
        let vspec = crate::npc::causal_core::resolve_vocational_vessel(&vocation_hint);
        let is_own_vessel =
            crafted_item_id == vspec.item_id || crafted_item_id == vspec.catalog_id;
        if matches!(
            phys_class,
            crate::game::object_ecology::ItemPhysicalClass::UmbrellaOrFan
                | crate::game::object_ecology::ItemPhysicalClass::OuterCloakOrRobe
                | crate::game::object_ecology::ItemPhysicalClass::GarmentOrArmor
                | crate::game::object_ecology::ItemPhysicalClass::FootwearOrHeadgear
        ) || !is_own_vessel
        {
            save_player_wordplates();
            return None;
        }
    }

    let raw_slot = db::get_item_def(crafted_item_id)
        .map(|d| d.slot)
        .unwrap_or_default();
    let slot = match raw_slot.as_str() {
        "hand" | "weapon" => "hold_r",
        "shield" | "tool" => "hold_l",
        "accessory" => "trinket",
        "helmet" => "head",
        "boots" => "feet",
        "armor" => "body",
        "hold" => "hold_r",
        other => other,
    };

    {
        let mut caskets = PLAYER_CASKETS.write();
        if let Some(casket) = caskets.get_mut(npc_id) {
            let _ = casket.inscribe_item(crafted_item_id, &elem.id);
        }
    }

    if let Err(e) = db::update_equipment_slot(npc_id, slot, crafted_item_id) {
        tracing::warn!("[npc_autonomous_word_and_equip_step] 佩戴裝備失敗 {npc_id} {slot} {crafted_item_id}: {e}");
    }

    save_player_wordplates();

    Some(format!("{npc_name}隨手將新造之【{crafted_item_name}】試手佩掛於身。"))
}

fn parse_equipment_triple(
    raw: &str,
) -> (
    HashMap<String, String>,
    HashMap<String, String>,
    HashMap<String, String>,
) {
    if raw.is_empty() {
        return (HashMap::new(), HashMap::new(), HashMap::new());
    }
    let slots: HashMap<String, String> = serde_json::from_str(raw).unwrap_or_default();
    let names = db::get_item_names(raw);
    let descs = db::get_item_descs(raw);
    (slots, names, descs)
}

pub(crate) fn entity_status_from_character(ent: &Character, is_self: bool) -> EntityStatusMsg {
    let rm = compute_resource_maxes(ent.vit, ent.qi, ent.dex);
    let hp = rm.hp_max as i32;
    let inner = rm.inner_max as i32;
    let spirit = rm.spirit_max as i32;
    let stamina = rm.stamina_max as i32;
    let magnesium = if is_self { Some(ent.magnesium) } else { None };
    let (room_id, room_name) = entity_location_label(ent);
    let (equipment_slots, mut equipment_names, mut equipment_descs) =
        parse_equipment_triple(&ent.equipment_slots);
    for (slot, item_id) in &equipment_slots {
        let item_name = equipment_names.get(slot).cloned().unwrap_or_default();
        if crate::pg::living_world::is_weapon_or_tool_item(item_id, &item_name) {
            let vstate = crate::pg::living_world::get_vessel_state(&ent.id, item_id);
            if vstate.use_count > 0
                || vstate.condition != crate::pg::living_world::VesselCondition::Sharp
            {
                if let Some(desc) = equipment_descs.get_mut(slot) {
                    let badge = format!("【器況：{}】", vstate.status_badge());
                    if !desc.contains("器況：") {
                        if desc.is_empty() {
                            *desc = badge;
                        } else {
                            *desc = format!("{} {}", desc, badge);
                        }
                    }
                }
            }
        }
        if let Some(name) = equipment_names.get_mut(slot) {
            *name = format_entity_item_with_inscription(&ent.id, item_id, name);
        }
    }
    let offline_dir = crate::npc::causal_core::get_player_offline_directive(&ent.id);
    let mut msg = EntityStatusMsg {
        msg_type: "entity_status".into(),
        entity_id: ent.id.clone(),
        display_char: ent.display_char.clone(),
        room_id,
        room_name,
        vit: ent.vit,
        qi: ent.qi,
        dex: ent.dex,
        hp_cur: hp,
        hp_max: hp,
        inner_cur: inner,
        inner_max: inner,
        spirit_cur: spirit,
        spirit_max: spirit,
        stamina_cur: stamina,
        stamina_max: stamina,
        magnesium,
        is_self,
        display_title: String::new(),
        origin_sentence: String::new(),
        activated_nodes: vec![],
        topology_costs: vec![],
        equipment_slots,
        equipment_names,
        equipment_descs,
        natural_skills: synthesize_entity_natural_skills(ent),
        offline_directive: offline_dir.code().to_string(),
        offline_directive_label: offline_dir.label().to_string(),
        offline_directive_desc: offline_dir.desc().to_string(),
        has_openrouter_key: is_self && crate::ai::get_player_openrouter_key(&ent.id).is_some(),
        openrouter_model: if is_self {
            crate::ai::DEFAULT_OPENROUTER_TALK_MODEL.to_string()
        } else {
            String::new()
        },
    };
    if !ent.display_title.is_empty() {
        msg.display_title.clone_from(&ent.display_title);
    }
    let parsed = parse_activated_nodes(&ent.activated_nodes);
    msg.activated_nodes = if parsed.len() <= 1 {
        if let Some(seed) = ent.soul_seed {
            derive_activated_nodes_from_soul_seed(seed)
        } else {
            parsed
        }
    } else {
        parsed
    };
    if let Some(seed) = ent.soul_seed {
        msg.origin_sentence = expand_soul_seed_to_origin_sentence(seed);
        msg.topology_costs = expand_soul_seed_to_topology_costs(seed);
    }
    msg
}

fn entity_location_label(ent: &Character) -> (String, String) {
    if let (Some(x), Some(y)) = (ent.grid_x, ent.grid_y) {
        let room_id = grid_room_id_from_coord(x, y);
        let room_name = crate::server::grid_manager::get_runtime_square_grid()
            .and_then(|grid| grid.get(SquareCoord { x, y }).map(|cell| cell.name.clone()))
            .filter(|name| !name.trim().is_empty())
            .or_else(|| {
                db::get_room_name(&room_id)
                    .ok()
                    .filter(|name| !name.is_empty())
            })
            .unwrap_or_else(|| room_id.clone());
        return (room_id, room_name);
    }
    let room_id = db::get_entity_room(&ent.id).unwrap_or_default();
    let room_name = if room_id.is_empty() {
        String::new()
    } else {
        db::get_room_name(&room_id)
            .ok()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| room_id.clone())
    };
    (room_id, room_name)
}

fn inventory_msg_for_character(ent: &Character) -> InventoryMsg {
    let inv = get_inventory(&ent.inventory, ent.vit);
    InventoryMsg {
        msg_type: "inventory".into(),
        items: inv
            .items
            .into_iter()
            .map(|it| {
                let is_weapon_tool = crate::pg::living_world::is_weapon_or_tool_item(&it.item_id, &it.name);
                let vstate_badge = if is_weapon_tool {
                    let vstate = crate::pg::living_world::get_vessel_state(&ent.id, &it.item_id);
                    if vstate.use_count > 0
                        || vstate.condition != crate::pg::living_world::VesselCondition::Sharp
                    {
                        format!("【器況：{}】", vstate.status_badge())
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                let mut description = if !it.origin.is_empty() && !it.description.contains("【產地：") {
                    if it.description.is_empty() {
                        format!("【產地：{}】", it.origin)
                    } else {
                        format!("{}【產地：{}】", it.description, it.origin)
                    }
                } else {
                    it.description
                };
                if !vstate_badge.is_empty() && !description.contains("器況：") {
                    if description.is_empty() {
                        description = vstate_badge;
                    } else {
                        description = format!("{} {}", description, vstate_badge);
                    }
                }
                let insc_name = format_entity_item_with_inscription(&ent.id, &it.item_id, &it.name);
                let tag = crate::model::canonical_object_tag(&it.item_id, &it.name);
                let category_tag = crate::model::object_category_tag(&it.item_id, &it.name).to_string();
                let attribute_tags = crate::model::object_attribute_tags(&it.item_id, &it.name).into_iter().map(str::to_string).collect();
                let tags = crate::model::object_semantic_tags(&it.item_id, &it.name, &[]);
                let is_edible = is_item_edible(&it.item_id, &insc_name, &it.item_type);
                let (item_stage, can_craft, craft_label) = classify_inventory_item_capabilities(
                    &it.item_id,
                    &insc_name,
                    &it.item_type,
                    &it.slot,
                );
                InventoryItemView {
                    item_id: it.item_id,
                    name: insc_name,
                    item_type: it.item_type,
                    qty: it.qty,
                    weight: it.weight,
                    sub_total: it.sub_total,
                    description,
                    slot: it.slot,
                    origin: it.origin,
                    tag,
                    category_tag,
                    attribute_tags,
                    tags,
                    is_edible,
                    item_stage,
                    can_craft,
                    craft_label,
                }
            })
            .collect(),
        current_weight: inv.current_weight,
        max_weight: inv.max_weight,
    }
}
