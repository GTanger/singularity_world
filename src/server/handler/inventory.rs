//! 背包／裝備／狀態查詢 + 拓撲除錯。

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use parking_lot::RwLock;

use serde::{Deserialize, Serialize};

use crate::combat::wordplate::{
    CarrierAffinity, CraftDims4D, OrbitFluency, StarInlay, StarSentence, TacticalLoadout,
    TacticalSlot, TargetPart, WordCasket, WordElement, WordQuality,
    WordRegistry,
};
use crate::db::{
    self, add_to_inventory, clear_equipment_slot, compute_resource_maxes,
    expand_soul_seed_to_origin_sentence, expand_soul_seed_to_topology_costs, get_inventory,
    get_item_def, inventory_has_item, inventory_weight, remove_from_inventory,
    update_equipment_slot,
};
use crate::entity::Character;
use crate::gametext;
use crate::grid::{SquareCoord, grid_room_id_from_coord};

use super::super::protocol::{
    ActionResultMsg, CasketWordEntry, ClientMsg, EntityStatusMsg, InventoryItemView, InventoryMsg,
    NaturalSkillView, TacticalLoadoutView, TacticalSentenceView, TacticalStateMsg,
    TopologyDebugAckMsg,
};
use super::{WsConnection, parse_activated_nodes, player_id};

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
    ensure_player_initialized(npc_id);
    let ent_opt = db::get_entity(npc_id).ok().flatten();
    let (vocation_hint, activity_hint) = if let Some(e) = &ent_opt {
        (e.display_title.clone(), e.current_activity.clone())
    } else {
        (String::new(), String::new())
    };
    let seed = ent_opt.as_ref().and_then(|e| e.soul_seed).unwrap_or(0);

    if let Some(ent) = &ent_opt {
        if !inventory_has_item(&ent.inventory, "blank_manual")
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
                }
            })
            .collect(),
        current_weight: inv.current_weight,
        max_weight: inv.max_weight,
    }
}

/// 裝備變更後推播背包與自身狀態（對齊既有 `pushRefresh`）。
pub(crate) fn push_refresh(conn: &WsConnection, player_id: &str) {
    let Ok(Some(ent)) = db::get_entity(player_id) else {
        return;
    };
    conn.send_json(&inventory_msg_for_character(&ent));
    conn.send_json(&entity_status_from_character(&ent, true));
}

pub(super) fn handle_get_entity_status(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let mut entity_id = msg.entity_id.clone();
    if entity_id.is_empty() {
        entity_id.clone_from(&pid);
    }
    let Ok(Some(ent)) = db::get_entity(&entity_id) else {
        conn.send_error(gametext::client("status_entity_not_found"));
        return;
    };
    let is_self = entity_id == pid;
    conn.send_json(&entity_status_from_character(&ent, is_self));
}

pub(super) fn handle_get_inventory(conn: &WsConnection) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    conn.send_json(&inventory_msg_for_character(&ent));
}

pub(super) fn handle_equip_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.item_id.is_empty() {
        conn.send_error(gametext::client("inv_no_item"));
        return;
    }
    let Some(def) = get_item_def(&msg.item_id) else {
        conn.send_error(gametext::client("inv_item_not_found"));
        return;
    };
    if def.slot.is_empty()
        || msg.item_id.contains("storage_box")
        || msg.item_id.contains("pine_soot_ink")
        || msg.item_id.contains("tool_handle")
        || def.name.contains("隨身匣")
        || def.name.contains("收納匣")
        || def.name.contains("墨錠")
        || def.name.contains("工具長柄")
    {
        conn.send_error(gametext::client("inv_cannot_equip"));
        return;
    }
    let mut target_slot = match def.slot.as_str() {
        "hand" | "weapon" => "hold_r".to_string(),
        "shield" | "tool" => "hold_l".to_string(),
        "accessory" => "trinket".to_string(),
        "helmet" => "head".to_string(),
        "boots" => "feet".to_string(),
        "armor" => "body".to_string(),
        "hold" => {
            if msg.target_slot == "hold_l" || msg.target_slot == "hold_r" {
                msg.target_slot.clone()
            } else {
                "hold_r".to_string()
            }
        }
        other => other.to_string(),
    };
    if (target_slot == "hold_r" || target_slot == "hold_l")
        && (msg.target_slot == "hold_l" || msg.target_slot == "hold_r")
        && (msg.target_slot == "hold_l"
            || crate::pg::living_world::is_weapon_or_tool_item(&msg.item_id, &def.name))
    {
        target_slot.clone_from(&msg.target_slot);
    }
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    if !inventory_has_item(&ent.inventory, &msg.item_id) {
        conn.send_error(gametext::client("inv_bag_missing"));
        return;
    }
    let current_slots: HashMap<String, String> = if ent.equipment_slots.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_str(&ent.equipment_slots).unwrap_or_default()
    };
    let old_item_id = current_slots.get(&target_slot).cloned().unwrap_or_default();
    if remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    if update_equipment_slot(&pid, &target_slot, &msg.item_id).is_err() {
        conn.send_error(gametext::client("inv_equip_fail"));
        return;
    }
    if !old_item_id.is_empty() && add_to_inventory(&pid, &old_item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_unequip_old_fail"));
        return;
    }
    if (target_slot == "hold_r" || target_slot == "hold_l")
        && crate::pg::living_world::is_weapon_or_tool_item(&msg.item_id, &def.name)
    {
        let vstate = crate::pg::living_world::get_vessel_state(&pid, &msg.item_id);
        if vstate.use_count == 0 {
            let _ = crate::pg::living_world::record_vessel_use(
                &pid,
                &msg.item_id,
                0,
                1,
                crate::game::now_unix(),
            );
        }
    }
    push_refresh(conn, &pid);
    let item_name = format_entity_item_with_inscription(&pid, &msg.item_id, &def.name);
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "equip_item".into(),
        target_id: msg.item_id.clone(),
        target_name: item_name.clone(),
        narrative: format!("你穿戴並佩掛起【{item_name}】。"),
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}

pub(super) fn handle_unequip_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.slot.is_empty() {
        conn.send_error(gametext::client("unequip_no_slot"));
        return;
    }
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    let current_slots: HashMap<String, String> = if ent.equipment_slots.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_str(&ent.equipment_slots).unwrap_or_default()
    };
    if current_slots.is_empty() {
        conn.send_error(gametext::client("unequip_nothing"));
        return;
    }
    let item_id = current_slots.get(&msg.slot).cloned().unwrap_or_default();
    if item_id.is_empty() {
        conn.send_error(gametext::client("unequip_slot_empty"));
        return;
    }
    let weight = get_item_def(&item_id).map(|d| d.weight).unwrap_or(0.0);
    let cur_w = inventory_weight(&ent.inventory);
    let max_w = ent.vit as f64 * 10.0;
    if cur_w + weight > max_w {
        conn.send_error(gametext::client("unequip_bag_full"));
        return;
    }
    if clear_equipment_slot(&pid, &msg.slot).is_err() {
        conn.send_error(gametext::client("unequip_slot_fail"));
        return;
    }
    if add_to_inventory(&pid, &item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    push_refresh(conn, &pid);
}

pub(super) fn handle_drop_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.item_id.is_empty() {
        conn.send_error(gametext::client("inv_no_item"));
        return;
    }
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    if !inventory_has_item(&ent.inventory, &msg.item_id) {
        conn.send_error(gametext::client("inv_bag_missing"));
        return;
    }
    let inv = get_inventory(&ent.inventory, ent.vit);
    let owned_qty = inv
        .items
        .iter()
        .find(|it| it.item_id == msg.item_id)
        .map(|it| it.qty)
        .unwrap_or(1)
        .max(1);
    let drop_qty = if msg.action == "all" {
        owned_qty
    } else {
        1
    };
    let item_name = resolve_inv_item_name(&msg.item_id);
    if remove_from_inventory(&pid, &msg.item_id, drop_qty).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    push_refresh(conn, &pid);
    let narrative = if drop_qty > 1 {
        format!("你從行囊中取出 {drop_qty} 份【{item_name}】，隨手將其捨棄在路旁。")
    } else {
        format!("你從行囊中取出一份【{item_name}】，隨手將其捨棄在路旁。")
    };
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "drop_item".into(),
        target_id: msg.item_id.clone(),
        target_name: item_name,
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}

pub(super) fn handle_use_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.item_id.is_empty() {
        conn.send_error(gametext::client("inv_no_item"));
        return;
    }

    // ─── 著譜立說（TranscribeManual）───────────────────────────────────────
    if msg.action == "TranscribeManual" {
        ensure_player_initialized(&pid);
        let Ok(Some(ent)) = db::get_entity(&pid) else {
            conn.send_error(gametext::client("inv_entity_not_found"));
            return;
        };

        let blank_id = if inventory_has_item(&ent.inventory, "blank_manual") {
            "blank_manual"
        } else if inventory_has_item(&ent.inventory, "wt_blank_manual") {
            "wt_blank_manual"
        } else {
            conn.send_error("行囊中尚無「空白手札」，無法著譜立說。");
            return;
        };

        let slot_str = msg.item_id.trim();
        let (slot_enum, slot_label) = match slot_str {
            "opening" | "起手" => (TacticalSlot::Opening, "起手"),
            "midgame" | "連攜" => (TacticalSlot::Midgame, "連攜"),
            "finisher" | "破綻" => (TacticalSlot::Finisher, "破綻"),
            "defense" | "守勢" => (TacticalSlot::Defense, "守勢"),
            _ => (TacticalSlot::Opening, "起手"),
        };

        let registry = WordRegistry::global();
        ensure_player_initialized(&pid);
        let casket = get_player_word_casket(&pid);
        let loadout = {
            let guard = PLAYER_LOADOUTS.read();
            guard.get(&pid).cloned().unwrap_or_else(|| {
                let seed = crate::combat::human_phase::resolve_entity_soul_seed(&ent.id, ent.soul_seed);
                crate::combat::wordplate::build_loadout_from_soul_seed(seed, Some(&casket), registry)
            })
        };

        let sent_opt = match slot_enum {
            TacticalSlot::Opening => loadout.opening.clone(),
            TacticalSlot::Midgame => loadout.midgame.clone(),
            TacticalSlot::Finisher => loadout.finisher.clone(),
            TacticalSlot::Defense => loadout.defense.clone(),
        };

        let Some(sent) = sent_opt else {
            conn.send_error("此路數尚未純熟，無法著譜立說。");
            return;
        };

        let (equip_slots, equip_names, _) = parse_equipment_triple(&ent.equipment_slots);
        let held_keys = ["hold_r", "hold_l", "weapon", "hand", "tool"];
        let mut held_item_id = String::new();
        let mut held_item_display = String::new();
        for k in &held_keys {
            if let Some(id) = equip_slots.get(*k).filter(|s| !s.is_empty()) {
                held_item_id = id.clone();
                let base = equip_names.get(*k).cloned().unwrap_or_else(|| resolve_inv_item_name(id));
                held_item_display = format_entity_item_with_inscription(&ent.id, id, &base);
                break;
            }
        }
        let held_carrier = if !held_item_id.is_empty() {
            crate::combat::wordplate::infer_item_carrier_affinity(&held_item_id, &held_item_display)
        } else {
            None
        };
        let martial_carrier = held_carrier.unwrap_or(CarrierAffinity::Unarmed);
        let vessel_name = if held_item_display.is_empty() {
            "隨身套路".to_string()
        } else {
            held_item_display.clone()
        };
        let studied_manuals = crate::pg::living_world::get_entity_studied_manuals(&ent.id);
        let learned_bonus = (studied_manuals.len() as f64 * 0.25).min(1.0);
        let vessel_familiarity_count = if !held_item_id.is_empty() {
            crate::pg::living_world::get_vessel_state(&ent.id, &held_item_id).use_count as u32
        } else {
            0
        };

        let seed = crate::combat::human_phase::resolve_entity_soul_seed(&ent.id, ent.soul_seed);
        let bone = crate::combat::human_phase::InnateBonePhase::from_seed(seed);
        let personal_routine = crate::combat::human_phase::derive_personal_martial_routine(
            &bone,
            slot_enum,
            martial_carrier,
            &vessel_name,
            learned_bonus,
            vessel_familiarity_count,
            registry,
        );

        let (formal_name, origin_basis, carrier, word_ids, word_chars) = if let Some(custom_name) = &sent.custom_name {
            let ids: Vec<String> = sent.inlays.iter().map(|i| i.word_id.clone()).collect();
            let chars: Vec<String> = sent
                .inlays
                .iter()
                .map(|inlay| {
                    registry
                        .get_by_id(&inlay.word_id)
                        .map(|e| e.ch.clone())
                        .unwrap_or_else(|| inlay.word_id.clone())
                })
                .collect();
            (
                custom_name.clone(),
                format!("體察自身發力習慣與呼吸節奏，將隨身演練純熟之《{}》著錄於冊", custom_name),
                martial_carrier,
                ids,
                chars,
            )
        } else {
            let mut ids = vec![personal_routine.omp_word_ids.0.clone(), personal_routine.omp_word_ids.1.clone()];
            for inlay in &sent.inlays {
                if !ids.contains(&inlay.word_id) {
                    ids.push(inlay.word_id.clone());
                }
            }
            while ids.len() < 3 {
                ids.push(personal_routine.omp_word_ids.1.clone());
            }
            let chars: Vec<String> = ids
                .iter()
                .map(|wid| {
                    registry
                        .get_by_id(wid)
                        .map(|e| e.ch.clone())
                        .unwrap_or_else(|| wid.clone())
                })
                .collect();
            (
                personal_routine.formal_name.clone(),
                personal_routine.origin_basis_zh.clone(),
                personal_routine.carrier,
                ids,
                chars,
            )
        };

        let mut hasher: u64 = 0xcbf29ce484222325;
        for b in pid.bytes() {
            hasher ^= b as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
        }
        for wid in &word_ids {
            for b in wid.bytes() {
                hasher ^= b as u64;
                hasher = hasher.wrapping_mul(0x100000001b3);
            }
        }
        let manual_hash = format!("{:016x}", hasher);

        if remove_from_inventory(&pid, blank_id, 1).is_err() {
            conn.send_error("扣除空白手札失敗");
            return;
        }

        let author_name = if !ent.display_title.trim().is_empty() {
            ent.display_title.trim().to_string()
        } else {
            "山海行旅者".to_string()
        };

        let now = crate::game::now_unix();
        let record = crate::pg::living_world::TranscribedManualRecord {
            manual_hash: manual_hash.clone(),
            author_id: pid.clone(),
            author_name,
            formal_name: formal_name.clone(),
            slot_label: slot_label.to_string(),
            carrier_code: carrier.code().to_string(),
            word_ids,
            word_chars: word_chars.clone(),
            origin_basis: origin_basis.clone(),
            created_at: now,
        };
        crate::pg::living_world::save_transcribed_manual(&record);

        let manual_item_id = format!("manual_{manual_hash}");
        let _ = db::add_to_inventory(&pid, &manual_item_id, 1);
        push_refresh(conn, &pid);

        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "TranscribeManual".into(),
            target_id: manual_item_id,
            target_name: format!("《{}》手札", formal_name),
            narrative: format!(
                "你鋪開空白手札，體察自身發力習慣與招式流轉，將隨身演練純熟之招法著錄於冊：以「{}」之勢與「{}」諸字真意逐筆錄下，裝訂成一冊《{}》手札收入行囊。",
                origin_basis,
                word_chars.join("・"),
                formal_name
            ),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }

    // ─── 研讀手札（StudyManual / Use manual_*）─────────────────────────────
    if msg.action == "StudyManual"
        || (msg.item_id.starts_with("manual_") && (msg.action.is_empty() || msg.action == "Use"))
    {
        ensure_player_initialized(&pid);
        let Ok(Some(ent)) = db::get_entity(&pid) else {
            conn.send_error(gametext::client("inv_entity_not_found"));
            return;
        };

        let Some(manual) = crate::pg::living_world::get_transcribed_manual(&msg.item_id) else {
            conn.send_error("此卷手札字跡殘損模糊，其中要訣已無從辨識。");
            return;
        };
        if manual.word_ids.len() < 3 {
            conn.send_error("此冊手札篇章殘缺，難以連貫參悟。");
            return;
        }
        let registry = WordRegistry::global();
        let learner_seed = crate::combat::human_phase::resolve_entity_soul_seed(&ent.id, ent.soul_seed);
        let bone = crate::combat::human_phase::InnateBonePhase::from_seed(learner_seed);

        let is_craft_manual = manual.carrier_code == "tool"
            || manual.slot_label.contains("百工")
            || manual.slot_label.contains("庖廚")
            || manual.slot_label.contains("採獵")
            || manual.slot_label.contains("行旅")
            || manual.slot_label == "craft"
            || manual.slot_label == "production"
            || manual.formal_name.contains("卯合")
            || manual.formal_name.contains("火候")
            || manual.formal_name.contains("收汁")
            || manual.formal_name.contains("衡準");
        let (slot_enum, slot_label_zh) = match manual.slot_label.as_str() {
            "起手" | "opening" => (TacticalSlot::Opening, "起手"),
            "連攜" | "midgame" => (TacticalSlot::Midgame, "連攜"),
            "破綻" | "finisher" | "重手" => (TacticalSlot::Finisher, "破綻"),
            "守勢" | "defense" => (TacticalSlot::Defense, "守勢"),
            _ => (TacticalSlot::Opening, "起手"),
        };

        let (equip_slots, equip_names, _) = parse_equipment_triple(&ent.equipment_slots);
        let held_keys = ["hold_r", "hold_l", "weapon", "hand", "tool"];
        let mut held_item_id = String::new();
        let mut held_item_display = String::new();
        for k in &held_keys {
            if let Some(id) = equip_slots.get(*k).filter(|s| !s.is_empty()) {
                let base = equip_names.get(*k).cloned().unwrap_or_else(|| resolve_inv_item_name(id));
                if crate::pg::living_world::is_weapon_or_tool_item(id, &base) {
                    held_item_id = id.clone();
                    held_item_display = format_entity_item_with_inscription(&ent.id, id, &base);
                    break;
                }
            }
        }
        let carrier = if !held_item_id.is_empty() {
            crate::combat::wordplate::infer_item_carrier_affinity(&held_item_id, &held_item_display)
                .unwrap_or(CarrierAffinity::Blade)
        } else {
            crate::combat::wordplate::CarrierAffinity::from_code(&manual.carrier_code)
                .unwrap_or(CarrierAffinity::Unarmed)
        };

        let (routine, adaptation_prose) = crate::combat::human_phase::adapt_manual_to_bone_phase(
            &bone,
            slot_enum,
            carrier,
            &manual.formal_name,
            &manual.word_chars,
            registry,
        );

        crate::pg::living_world::record_entity_studied_manual(&pid, &manual.formal_name);

        let word_count = if manual.word_ids.len() >= 7 {
            7
        } else if manual.word_ids.len() >= 5 {
            5
        } else {
            3
        };
        let inlays: Vec<StarInlay> = (0..word_count)
            .map(|i| {
                let node_id = format!("N{:03}", i + 1);
                let wid = if i == 0 {
                    routine.omp_word_ids.0.clone()
                } else if i == 1 {
                    routine.omp_word_ids.1.clone()
                } else {
                    manual.word_ids.get(i).cloned().unwrap_or_else(|| routine.omp_word_ids.1.clone())
                };
                let fluency = if routine.continuity_c >= 1.25 {
                    OrbitFluency::ChangLiu
                } else if routine.continuity_c >= 0.95 {
                    OrbitFluency::ShunTong
                } else if routine.continuity_c >= 0.70 {
                    OrbitFluency::PingWen
                } else {
                    OrbitFluency::ZhiSe
                };
                StarInlay::with_fluency(
                    node_id,
                    wid,
                    fluency,
                )
            })
            .collect();
        let mut star_sentence = match StarSentence::new(inlays) {
            Ok(s) => s.with_custom_name(&routine.formal_name),
            Err(e) => {
                conn.send_error(format!("招式推演受阻：{e}"));
                return;
            }
        };
        star_sentence.set_soul_seed(learner_seed);

        if !is_craft_manual {
            let mut loadouts = PLAYER_LOADOUTS.write();
            if let Some(loadout) = loadouts.get_mut(&pid) {
                loadout.set_slot(slot_enum, star_sentence);
            } else {
                let mut loadout = create_default_loadout(registry);
                loadout.set_slot(slot_enum, star_sentence);
                loadouts.insert(pid.clone(), loadout);
            }
            save_player_wordplates();
        }
        push_refresh(conn, &pid);

        let motions_str = routine.main_motions.join(" → ");
        let narrative = if is_craft_manual {
            format!(
                "你靜心參讀《{}》手札，細細揣摩其中百工造物與行氣法度，隨自身發力習慣心領神會！\n{}\n手札所載之「{}」要訣已融會貫通並納入【{}】與百工手藝之中！",
                manual.formal_name,
                adaptation_prose,
                motions_str,
                manual.slot_label
            )
        } else {
            format!(
                "你靜心參讀《{}》手札，細細揣摩其中發力法度，隨自身體魄節奏自然化為「{}」！\n{}\n招式循序運化為「{}」，已自然融入{}身手！",
                manual.formal_name,
                routine.formal_name,
                adaptation_prose,
                motions_str,
                slot_label_zh
            )
        };
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "StudyManual".into(),
            target_id: msg.item_id.clone(),
            target_name: format!("《{}》手札", manual.formal_name),
            narrative,
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }

    // ─── 舊法攔截（InscribeItem, StackWord, MintWord, StripName）───────────
    if matches!(
        msg.action.as_str(),
        "InscribeItem" | "StackWord" | "MintWord" | "ExtractWord"
    ) {
        conn.send_error("此項舊法已不再通行，請直接運用器物或執行百工工序。");
        return;
    }
    if msg.action == "StripName" {
        conn.send_error("此項舊法已不再通行，請直接運用器物或執行百工工序。");
        return;
    }
    if msg.action == "StripNameWhole" && !msg.item_id.starts_with("corpse_") {
        conn.send_error("此項舊法已不再通行，請直接運用器物或執行百工工序。");
        return;
    }

    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    if !inventory_has_item(&ent.inventory, &msg.item_id) {
        conn.send_error(gametext::client("inv_bag_missing"));
        return;
    }
    // ─── 舊版晶石使用清理 ───────────────────────────────────────────────────
    if crate::combat::wordplate::parse_crystal_item_id(&msg.item_id).is_some() {
        let _ = remove_from_inventory(&pid, &msg.item_id, 1);
        push_refresh(conn, &pid);
        return;
    }

    // ─── 肢體分解（ButcherCorpse 或殘軀之 StripNameWhole）────────────────────
    if msg.action == "ButcherCorpse" || msg.action == "StripNameWhole" {
        if !msg.item_id.starts_with("corpse_") {
            conn.send_error("此物已非完整殘軀，不可再次分解。");
            return;
        }

        let Ok(Some(ent)) = db::get_entity(&pid) else {
            conn.send_error(gametext::client("inv_entity_not_found"));
            return;
        };

        if remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
            conn.send_error(gametext::client("inv_bag_fail"));
            return;
        }

        let corpse_rec = crate::pg::living_world::get_beast_corpse(&msg.item_id);
        crate::pg::living_world::consume_beast_corpse(&msg.item_id);

        let primary_loot_id = corpse_rec
            .as_ref()
            .map(|r| r.primary_loot_id.as_str())
            .unwrap_or("mat_leather_strip");
        let primary_loot_name = corpse_rec
            .as_ref()
            .map(|r| r.primary_loot_name.as_str())
            .unwrap_or("異獸毛皮");
        let creature_title = corpse_rec
            .as_ref()
            .map(|r| r.creature_title.as_str())
            .unwrap_or("異獸");

        let _ = db::add_to_inventory(&pid, primary_loot_id, 1);
        let _ = db::add_to_inventory(&pid, "wt_beast_bone", 1);
        push_refresh(conn, &pid);
        crate::pg::living_world::record_entity_anatomy_familiarity(&pid, creature_title, 1);

        let now = crate::game::now_unix();
        let habit_rec = crate::pg::living_world::record_breath_activity(
            &pid,
            crate::pg::living_world::BreathHabitKind::ForestLong,
            2,
            now,
        );
        let equipment_map: HashMap<String, String> = if ent.equipment_slots.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&ent.equipment_slots).unwrap_or_default()
        };
        let held_keys = ["hold_r", "hold_l", "weapon", "hand", "tool"];
        let mut bladed_tool: Option<(String, String)> = None;
        for key in &held_keys {
            if let Some(held_id) = equipment_map.get(*key).filter(|id| !id.is_empty()) {
                let held_name = resolve_inv_item_name(held_id);
                if crate::pg::living_world::is_bladed_vessel_item(held_id, &held_name) {
                    bladed_tool = Some((held_id.clone(), held_name));
                    break;
                }
            }
        }
        if bladed_tool.is_none() {
            let inv_check = get_inventory(&ent.inventory, ent.vit);
            for it in inv_check.items {
                if crate::pg::living_world::is_bladed_vessel_item(&it.item_id, &it.name) {
                    bladed_tool = Some((it.item_id, it.name));
                    break;
                }
            }
        }
        let vstate = bladed_tool.as_ref().map(|(kid, _)| {
            crate::pg::living_world::record_vessel_use(&pid, kid, 1, 2, now)
        });

        let mut narrative = if let (Some((_, kname)), Some(vs)) = (&bladed_tool, &vstate) {
            format!(
                "你操持【{}】（{}），順著肌理筋絡細細剖解【{}殘軀】。{}。手中刃鋒俐落剔下品相完好的「{}」，並收納下一批堅實的獸骨。",
                kname,
                vs.status_badge(),
                creature_title,
                vs.familiarity().feel_prose(),
                primary_loot_name
            )
        } else {
            format!(
                "你徒手順著肌理筋絡細細分理【{}殘軀】。氣勁沉穩，俐落剔下品相完好的「{}」，並收納下一批堅實的獸骨。",
                creature_title,
                primary_loot_name
            )
        };
        let seed = crate::combat::human_phase::resolve_entity_soul_seed(&ent.id, ent.soul_seed);
        let bone = crate::combat::human_phase::InnateBonePhase::from_seed(seed);
        let anatomy_fam = crate::pg::living_world::get_entity_anatomy_familiarity_total(&pid);
        let seam = crate::combat::human_phase::evaluate_seam_threading(
            &bone,
            CarrierAffinity::Blade,
            ent.dex as f64,
            45.0,
            anatomy_fam,
        );
        if seam.xi_index >= 0.40 {
            narrative.push_str("你順著生靈骨節肌理之天然空隙游刃穿入，刀鋒避開堅硬外皮，毫不費力便將其完整剖解，器物鋒刃幾無損耗。");
        }

        let familiar_enough = vstate.as_ref().map(|vs| matches!(
            vs.familiarity(),
            crate::pg::living_world::VesselFamiliarity::WellBalanced
                | crate::pg::living_world::VesselFamiliarity::LikeOwnArm
        )).unwrap_or(false);

        if familiar_enough {
            let actor_name = if !ent.display_title.trim().is_empty() {
                ent.display_title.trim().to_string()
            } else {
                "獵手".to_string()
            };
            let tool_name = bladed_tool.as_ref().map(|(_, n)| n.clone()).unwrap_or_else(|| "徒手".to_string());
            let tool_familiarity = vstate.as_ref().map(|vs| vs.familiarity().label().to_string()).unwrap_or_else(|| "得心應手".to_string());
            let packet = crate::ai::CraftActionFactPacket {
                actor_name,
                tool_name,
                tool_familiarity,
                breath_habit: habit_rec.dominant.label().to_string(),
                material_names: vec![format!("{}殘軀", creature_title)],
                output_name: primary_loot_name.to_string(),
                highlight_kind: "殘軀分解".to_string(),
                environment_hint: "荒澤獵場".to_string(),
            };
            let highlight = crate::ai::compose_craft_highlight_from_data(&packet);
            if !highlight.is_empty() {
                narrative.push_str("\n\n");
                narrative.push_str(&highlight);
            }
        }

        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "ButcherCorpse".into(),
            target_id: msg.item_id.clone(),
            target_name: format!("{}殘軀", creature_title),
            narrative,
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }


    if matches!(
        msg.action.as_str(),
        "MakeFire" | "SplitWood" | "RefinePlank" | "BurnCharcoal" | "CraftTorch" | "CraftHandle" | "AddFuel" | "RepairStructure"
    ) || (msg.action.is_empty()
        && matches!(
            msg.item_id.as_str(),
            "mat_kindling_bundle" | "mat_timber_log" | "harvest_wood" | "mat_charcoal_sack"
        ))
    {
        let (room_id, room_name) = entity_location_label(&ent);
        let ent_room = if !room_name.is_empty() { room_name } else { room_id };
        let now = crate::game::now_unix();

        let res = match msg.action.as_str() {
            "SplitWood" | "RefinePlank" | "BurnCharcoal" | "CraftTorch" | "CraftHandle" => {
                crate::game::object_ecology::refine_wood_multi_socket(&pid, &msg.action, &ent_room, now)
                    .map(|(_id, _name, narr)| narr)
            }
            "AddFuel" => {
                crate::game::object_ecology::stoke_workshop_furnace(&pid, &ent_room, now)
            }
            "RepairStructure" => {
                let target_name = resolve_inv_item_name(&msg.item_id);
                crate::game::object_ecology::repair_room_structure(&pid, &ent_room, &target_name, now)
            }
            _ => {
                if crate::game::object_ecology::get_active_campfire(&ent_room).is_none() {
                    let result = crate::game::object_ecology::ignite_or_stoke_campfire(
                        &pid,
                        &ent_room,
                        Some(&msg.item_id),
                        now,
                    );
                    if result.is_ok() && msg.item_id == "mat_timber_log" {
                        let _ = db::add_to_inventory(&pid, "mat_kindling_bundle", 1);
                    }
                    result.map(|(narr, _is_new)| narr)
                } else {
                    let mode = if msg.item_id == "mat_timber_log" {
                        "RefinePlank"
                    } else {
                        "CraftTorch"
                    };
                    crate::game::object_ecology::refine_wood_multi_socket(&pid, mode, &ent_room, now)
                        .map(|(_id, _name, narr)| narr)
                }
            }
        };

        match res {
            Ok(narrative) => {
                push_refresh(conn, &pid);
                crate::server::broadcast::refresh_room_views_for_room(
                    &conn.sessions,
                    &conn.cfg,
                    &ent_room,
                );
                let item_name = resolve_inv_item_name(&msg.item_id);

                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: if msg.action.is_empty() { "use_item".into() } else { msg.action.clone() },
                    target_id: msg.item_id.clone(),
                    target_name: item_name,
                    narrative,
                    success: true,
                    actions: vec![
                        "MakeFire".into(),
                        "SplitWood".into(),
                        "RefinePlank".into(),
                        "BurnCharcoal".into(),
                        "CraftTorch".into(),
                        "AddFuel".into(),
                        "RepairStructure".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    } else if matches!(
        msg.action.as_str(),
        "WashOre" | "SmeltBloom" | "ForgeRepairStrip" | "SharpenBlade" | "BuildFieldAnvil"
    ) {
        let (room_id, room_name) = entity_location_label(&ent);
        let ent_room = if !room_name.is_empty() { room_name } else { room_id };
        let now = crate::game::now_unix();
        let act = if msg.action.is_empty() { "WashOre" } else { msg.action.as_str() };
        let res = crate::game::object_ecology::refine_mineral_multi_socket(&pid, act, &ent_room, now);
        match res {
            Ok((_id, _name, narrative)) => {
                push_refresh(conn, &pid);
                if act == "BuildFieldAnvil" {
                    crate::server::broadcast::refresh_room_views_for_room(
                        &conn.sessions,
                        &conn.cfg,
                        &ent_room,
                    );
                }
                let item_name = resolve_inv_item_name(&msg.item_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: if msg.action.is_empty() { act.into() } else { msg.action.clone() },
                    target_id: msg.item_id.clone(),
                    target_name: item_name,
                    narrative,
                    success: true,
                    actions: vec![
                        "WashOre".into(),
                        "SmeltBloom".into(),
                        "ForgeRepairStrip".into(),
                        "SharpenBlade".into(),
                        "BuildFieldAnvil".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    } else if matches!(
        msg.action.as_str(),
        "WashHerb" | "DryHerb" | "CrushHerb" | "BoilDecoction" | "SetupHerbStove"
    ) {
        let (room_id, room_name) = entity_location_label(&ent);
        let ent_room = if !room_name.is_empty() { room_name } else { room_id };
        let now = crate::game::now_unix();
        let act = if msg.action.is_empty() { "WashHerb" } else { msg.action.as_str() };
        let res = crate::game::object_ecology::refine_herb_multi_socket(&pid, act, &ent_room, now);
        match res {
            Ok((_id, _name, narrative)) => {
                push_refresh(conn, &pid);
                if act == "SetupHerbStove" {
                    crate::server::broadcast::refresh_room_views_for_room(
                        &conn.sessions,
                        &conn.cfg,
                        &ent_room,
                    );
                }
                let item_name = resolve_inv_item_name(&msg.item_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: if msg.action.is_empty() { act.into() } else { msg.action.clone() },
                    target_id: msg.item_id.clone(),
                    target_name: item_name,
                    narrative,
                    success: true,
                    actions: vec![
                        "WashHerb".into(),
                        "DryHerb".into(),
                        "CrushHerb".into(),
                        "BoilDecoction".into(),
                        "SetupHerbStove".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    } else if matches!(
        msg.action.as_str(),
        "TwistRope" | "WeaveSackcloth" | "OilCanvas" | "PressPaper"
    ) {
        let (room_id, room_name) = entity_location_label(&ent);
        let ent_room = if !room_name.is_empty() { room_name } else { room_id };
        let now = crate::game::now_unix();
        let act = if msg.action.is_empty() { "TwistRope" } else { msg.action.as_str() };
        let res = crate::game::object_ecology::refine_fiber_multi_socket(&pid, act, &ent_room, now);
        match res {
            Ok((_id, _name, narrative)) => {
                push_refresh(conn, &pid);
                let item_name = resolve_inv_item_name(&msg.item_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: if msg.action.is_empty() { act.into() } else { msg.action.clone() },
                    target_id: msg.item_id.clone(),
                    target_name: item_name,
                    narrative,
                    success: true,
                    actions: vec![
                        "TwistRope".into(),
                        "WeaveSackcloth".into(),
                        "OilCanvas".into(),
                        "PressPaper".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    } else if matches!(
        msg.action.as_str(),
        "ScrapeHide" | "BoilFishGlue" | "CalcifyShell" | "CobbleBoots"
    ) {
        let (room_id, room_name) = entity_location_label(&ent);
        let ent_room = if !room_name.is_empty() { room_name } else { room_id };
        let now = crate::game::now_unix();
        let act = if msg.action.is_empty() { "ScrapeHide" } else { msg.action.as_str() };
        let res = crate::game::object_ecology::refine_hide_fish_multi_socket(&pid, act, &ent_room, now);
        match res {
            Ok((_id, _name, narrative)) => {
                push_refresh(conn, &pid);
                let item_name = resolve_inv_item_name(&msg.item_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: if msg.action.is_empty() { act.into() } else { msg.action.clone() },
                    target_id: msg.item_id.clone(),
                    target_name: item_name,
                    narrative,
                    success: true,
                    actions: vec![
                        "ScrapeHide".into(),
                        "BoilFishGlue".into(),
                        "CalcifyShell".into(),
                        "CobbleBoots".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    }
    if let Some((tag, vit_gain, qi_gain, consume, effect_narr)) = crate::game::shanhai_item_use_effect(&msg.item_id) {
        if consume && remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
            conn.send_error(gametext::client("inv_bag_fail"));
            return;
        }
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(&pid, |e| {
                e.vit = (e.vit + vit_gain).min(100);
                e.qi = (e.qi + qi_gain).min(100);
            });
        }
        crate::pg::living_world::clear_entity_afflictions(&pid, true, crate::game::now_unix());
        push_refresh(conn, &pid);
        let item_name = resolve_inv_item_name(&msg.item_id);
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: if msg.action.is_empty() { "Use".into() } else { msg.action.clone() },
            target_id: msg.item_id.clone(),
            target_name: item_name,
            narrative: format!("【{tag}】{effect_narr}（山海靈韻化入肺腑，氣血隨之充盈）"),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }
    let item_def = get_item_def(&msg.item_id);
    let item_name = format_entity_item_with_inscription(&pid, &msg.item_id, &resolve_inv_item_name(&msg.item_id));
    let is_non_edible_gear_or_mat = item_def
        .as_ref()
        .is_some_and(|d| !d.slot.is_empty() || d.item_type == "equipment" || d.item_type == "misc" || d.item_type == "tool")
        || matches!(
            msg.item_id.as_str(),
            "mat_fishing_line_bundle"
                | "mat_fish_scale_sack"
                | "mat_fish_bone_sack"
                | "mat_sealing_pitch"
                | "item_ebony_mortise_tea_tray"
        )
        || [
            "魚骨", "魚鱗", "魚皮", "釣魚線", "魚線", "漁線", "魚網", "漁網", "魚簍", "魚刀", "魚鰾",
            "螺殼", "貝殼", "蟹殼", "封縫膏", "灰膏", "茶盤", "茶壺", "茶盅", "茶篩", "茶托", "茶籽",
            "湯鍋", "湯碗", "果盤", "螺鈿",
        ]
        .iter()
        .any(|kw| item_name.contains(kw));
    let is_edible = !is_non_edible_gear_or_mat
        && (item_def.as_ref().is_some_and(|d| d.item_type == "consumable" || d.item_type == "food")
            || matches!(
                msg.item_id.as_str(),
                "mat_fresh_shrimp"
                    | "shanhai_fresh_fish"
                    | "shanhai_beast_meat"
                    | "mat_salted_fish"
                    | "harvest_fish"
                    | "harvest_berries"
                    | "craft_roasted_shrimp_skewer"
                    | "craft_smoked_fish_fillet"
                    | "craft_spiced_beast_jerky"
                    | "craft_ration_grain_cake"
                    | "craft_meridian_salve"
                    | "craft_calming_tea_pack"
            )
            || ["蝦", "魚", "肉", "蟹", "鮑", "蛤", "蜆", "螺", "果", "茶", "餅", "脯", "串", "湯", "粥", "飲", "露", "膏", "散", "丹", "丸", "水芹", "芝"]
                .iter()
                .any(|kw| item_name.contains(kw)));
    if !is_edible {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: if msg.action.is_empty() { "Use".into() } else { msg.action.clone() },
            target_id: msg.item_id.clone(),
            target_name: item_name.clone(),
            narrative: format!("【{item_name}】屬於百工原材或隨身器物，無法直接食用；請於行囊點擊「加工煉製」或「穿戴」。"),
            success: false,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }
    if remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    let is_meridian_salve = msg.item_id == "craft_meridian_salve" || msg.item_id.contains("meridian_salve");
    let is_calming_tea = msg.item_id == "craft_calming_tea_pack" || msg.item_id.contains("calming_tea");
    let is_culinary = matches!(
        msg.item_id.as_str(),
        "craft_roasted_shrimp_skewer"
            | "craft_smoked_fish_fillet"
            | "craft_spiced_beast_jerky"
            | "craft_ration_grain_cake"
            | "mat_salted_fish"
    );
    let is_mutated = msg.item_id.starts_with("mut_") || crate::game::object_ecology::classify_item_stage(&msg.item_id) == "mutated";
    if let Some(arc) = crate::store::get_store() {
        let _ = arc.write().update_entity(&pid, |e| {
            if is_meridian_salve {
                e.vit = (e.vit + 25).min(100);
                e.qi = (e.qi + 20).min(100);
            } else if is_calming_tea {
                e.qi = (e.qi + 30).min(100);
            } else if is_culinary {
                e.vit = (e.vit + 20).min(100);
                e.qi = (e.qi + 18).min(100);
            } else if is_mutated {
                e.vit = (e.vit + 15).min(100);
                e.qi = (e.qi + 20).min(100);
            } else {
                e.vit = (e.vit + 8).min(100);
                e.qi = (e.qi + 10).min(100);
            }
        });
    }
    if is_meridian_salve || is_calming_tea || is_mutated {
        crate::pg::living_world::clear_entity_afflictions(&pid, true, crate::game::now_unix());
    }
    push_refresh(conn, &pid);

    // item_name already bound above
    let bracketed_name = if item_name.starts_with('【') {
        item_name.clone()
    } else {
        format!("【{item_name}】")
    };

    let is_food = ["餅", "麵", "飯", "肉", "薯", "凍", "花", "菜", "蝦", "蛤", "魚", "丸", "糖", "湯", "粥", "粿"]
        .iter()
        .any(|k| item_name.contains(k));
    let is_drink = ["茶", "漿", "飲", "水", "酒", "汁", "露"]
        .iter()
        .any(|k| item_name.contains(k));
    let is_herb = ["草", "葉", "根", "果", "藥", "丹", "膏", "散"]
        .iter()
        .any(|k| item_name.contains(k));

    let narrative = if is_meridian_salve {
        format!("你取出一份{bracketed_name}塗抹於周身經絡要穴。溫潤的藥力順著經脈化開，氣血大暢，肌肉酸痛盡消（周身氣血與心神皆獲豐沛滋養）！")
    } else if is_calming_tea {
        format!("你沏開這份{bracketed_name}細細品飲。茶湯清洌甘甜，清氣透入眉宇，心神澄澈通達（心神頓時澄澈充盈）！")
    } else if is_mutated {
        format!("你凝神運用手中神異的{bracketed_name}。微芒順著指尖湧入經脈，天地共鳴之氣充盈四肢百骸，周身氣脈頓時大振（體魄與元氣漸次平順舒暢）！")
    } else if is_food {
        format!("你取出一份熱騰騰的{bracketed_name}，大口趁熱品嚐。外皮酥香、內餡鮮美多汁，溫潤的香氣自舌尖散開，頓時腹中一暖，整個人精神大振！")
    } else if is_drink {
        format!("你仰頭啜飲這份{bracketed_name}。甘潤清甜的湯水順喉而下，齒頰留香，周身燥熱頓消，氣息隨之舒暢通達。")
    } else if is_herb {
        format!("你細細研服這份{bracketed_name}。草木清香在口中化開，微苦回甘，藥力順著經脈緩緩化開，氣血漸漸充盈。")
    } else {
        format!("你取出手中的{bracketed_name}仔細檢視並加以使用。觸感紮實細膩，工藝純熟，片刻調理後整個人氣定神閒。")
    };

    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: if msg.action.is_empty() { "Use".into() } else { msg.action.clone() },
        target_id: msg.item_id.clone(),
        target_name: item_name,
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}

pub(super) fn handle_sell_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.item_id.is_empty() {
        conn.send_error(gametext::client("inv_no_item"));
        return;
    }
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    if !inventory_has_item(&ent.inventory, &msg.item_id) {
        conn.send_error(gametext::client("inv_bag_missing"));
        return;
    }
    let inv = get_inventory(&ent.inventory, ent.vit);
    let owned_qty = inv
        .items
        .iter()
        .find(|it| it.item_id == msg.item_id)
        .map(|it| it.qty)
        .unwrap_or(1)
        .max(1);
    let sell_qty = if msg.action == "all" {
        owned_qty
    } else {
        1
    };

    let item_name = resolve_inv_item_name(&msg.item_id);

    let base_unit_price = crate::combat::wordplate::parse_crystal_item_id(&msg.item_id)
        .map(|(_, q)| (q.magnesium_value() + 5).max(3))
        .or_else(|| crate::world::materials::trade_price_anchor_mg(&msg.item_id))
        .or_else(|| {
            crate::game::peninsula_catalog::find_catalog_entry(&msg.item_id)
                .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
        })
        .or_else(|| {
            for fg in &crate::game::object_ecology::STANDARD_FINISHED_GOODS {
                if fg.id == msg.item_id {
                    return Some(fg.base_price);
                }
            }
            get_item_def(&msg.item_id).map(|d| (d.weight * 15.0).round() as i32)
        })
        .unwrap_or(3)
        .max(3);

    let player_room = db::get_entity_room(&pid)
        .ok()
        .and_then(|r| db::get_room_name(&r).ok())
        .unwrap_or_else(|| "文曜".to_string());
    let initial_quote = crate::game::object_ecology::dynamic_settlement_market_quote(
        &player_room,
        &msg.item_id,
        base_unit_price,
    );
    if remove_from_inventory(&pid, &msg.item_id, sell_qty).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    let mut payout = 0i32;
    for _ in 0..sell_qty {
        let cur = crate::game::object_ecology::dynamic_settlement_market_quote(
            &player_room,
            &msg.item_id,
            base_unit_price,
        );
        payout = payout.saturating_add(cur.unit_price);
        let _ = crate::game::object_ecology::record_settlement_market_trade(
            &player_room,
            &msg.item_id,
            base_unit_price,
            1,
            "玩家售出",
        );
    }
    if db::add_magnesium(&pid, payout).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }

    push_refresh(conn, &pid);

    let narrative = format!(
        "你將 {sell_qty} 份【{item_name}】交予本地市集商鋪驗收（{}，實得 {payout} 鎂，款貨兩訖）。",
        initial_quote.market_narrative,
    );
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "sell_item".into(),
        target_id: msg.item_id.clone(),
        target_name: item_name,
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}

pub(super) fn handle_craft_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    let (_, room_name) = entity_location_label(&ent);
    let now = crate::game::now_unix();

    match crate::game::object_ecology::craft_from_raw_material(&pid, &msg.item_id, &room_name, now) {
        Ok((item_id, item_name, _is_mutated, mut narrative)) => {
            let nat_skills = synthesize_entity_natural_skills(&ent);
            if let Some(forge_sk) = nat_skills.iter().find(|s| s.domain == "production" && s.category_label.contains("百工")) {
                let is_culinary = item_id.contains("shrimp")
                    || item_id.contains("fish")
                    || item_id.contains("jerky")
                    || item_id.contains("grain")
                    || item_id.contains("tea");
                if is_culinary {
                    narrative.push_str(&format!(
                        " 你順著平日熟稔的《{}》手法掌控火候收汁，滋味愈發醇厚鮮美。",
                        forge_sk.name
                    ));
                } else {
                    narrative.push_str(&format!(
                        " 你順著平日熟稔的《{}》手法落手收勢，器物肌理更顯勻稱。",
                        forge_sk.name
                    ));
                }
            }
            push_refresh(conn, &pid);
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "craft_item".into(),
                target_id: item_id,
                target_name: item_name,
                narrative,
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
        }
        Err(err) => {
            conn.send_error(err);
        }
    }
}

pub(super) fn handle_mutate_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    let (_, room_name) = entity_location_label(&ent);
    let now = crate::game::now_unix();

    match crate::game::object_ecology::mutate_finished_item(&pid, &msg.item_id, &room_name, now) {
        Ok((mut_id, mut_name, mut narrative)) => {
            let nat_skills = synthesize_entity_natural_skills(&ent);
            if let Some(forge_sk) = nat_skills.iter().find(|s| s.domain == "production" && s.category_label.contains("百工")) {
                narrative.push_str(&format!(
                    " 你循著《{}》的呼吸節奏導引火候，器表靈韻隱隱流轉。",
                    forge_sk.name
                ));
            }
            push_refresh(conn, &pid);
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "mutate_item".into(),
                target_id: mut_id,
                target_name: mut_name,
                narrative,
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
        }
        Err(err) => {
            conn.send_error(err);
        }
    }
}

pub(super) fn handle_print_topology_debug(conn: &WsConnection) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    let seed = match ent.soul_seed {
        Some(s) if s != 0 => s,
        _ => {
            tracing::info!(target: "topology_debug", "角色無 soul_seed（可能為舊資料）");
            conn.send_error(gametext::client("seed_no_soul"));
            return;
        }
    };
    let costs = expand_soul_seed_to_topology_costs(seed);
    let sum: f64 = costs.iter().sum();
    tracing::info!(target: "topology_debug", "========== 361 拓撲除錯（當前角色） ==========");
    tracing::info!(target: "topology_debug", soul_seed = seed, "SoulSeed (int64)");
    tracing::info!(target: "topology_debug", "N000（生之本源）→ 前三條主星軌阻力：");
    if costs.len() >= 3 {
        tracing::info!(target: "topology_debug", c0 = costs[0], "N000 → N001");
        tracing::info!(target: "topology_debug", c1 = costs[1], "N000 → N002");
        tracing::info!(target: "topology_debug", c2 = costs[2], "N000 → N003");
    }
    tracing::info!(target: "topology_debug", sum, "全 760 條連線 Cost 總和（規格常數應為 10000）");
    tracing::info!(target: "topology_debug", "=============================================");
    conn.send_json(&TopologyDebugAckMsg {
        msg_type: "topology_debug".into(),
        message: "已於伺服器終端印出".into(),
    });
}

// ─── 詞匣與四戰術序位系統 ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedWordPlateState {
    #[serde(default)]
    caskets: HashMap<String, WordCasket>,
    #[serde(default)]
    loadouts: HashMap<String, TacticalLoadout>,
}

const PLAYER_WORDPLATES_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/runtime/player_wordplates.json"
);

fn player_wordplates_path() -> std::path::PathBuf {
    let p = std::path::Path::new("data/runtime/player_wordplates.json");
    if p.exists() || std::path::Path::new("data/runtime").exists() {
        p.to_path_buf()
    } else {
        std::path::PathBuf::from(PLAYER_WORDPLATES_FILE)
    }
}

fn is_persisted_wordplate_owner(pid: &str) -> bool {
    if pid.starts_with("npc_")
        || pid.starts_with("ambient_")
        || pid.starts_with("shanhai_")
        || pid.starts_with("wt_npc_")
        || pid.starts_with("wt_vendor_")
    {
        false
    } else {
        true
    }
}
fn load_persisted_wordplates() -> PersistedWordPlateState {
    let path = player_wordplates_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(mut state) = serde_json::from_str::<PersistedWordPlateState>(&data) {
            let registry = WordRegistry::global();
            let mut repaired = false;
            let caskets_before = state.caskets.len();
            state.caskets.retain(|k, _| is_persisted_wordplate_owner(k));
            if state.caskets.len() != caskets_before {
                repaired = true;
            }
            let loadouts_before = state.loadouts.len();
            state.loadouts.retain(|k, _| is_persisted_wordplate_owner(k));
            if state.loadouts.len() != loadouts_before {
                repaired = true;
            }
            for (pid, loadout) in state.loadouts.iter_mut() {
                if loadout.opening.is_none()
                    || loadout.midgame.is_none()
                    || loadout.finisher.is_none()
                    || loadout.defense.is_none()
                {
                    let seed = crate::combat::human_phase::resolve_entity_soul_seed(pid, None);
                    let casket_opt = state.caskets.get(pid);
                    let default_ld = crate::combat::wordplate::build_loadout_from_soul_seed(
                        seed, casket_opt, registry,
                    );
                    if loadout.opening.is_none() {
                        loadout.opening = default_ld.opening;
                        repaired = true;
                    }
                    if loadout.midgame.is_none() {
                        loadout.midgame = default_ld.midgame;
                        repaired = true;
                    }
                    if loadout.finisher.is_none() {
                        loadout.finisher = default_ld.finisher;
                        repaired = true;
                    }
                    if loadout.defense.is_none() {
                        loadout.defense = default_ld.defense;
                        repaired = true;
                    }
                }
            }
            if repaired {
                if let Ok(json) = serde_json::to_string_pretty(&state) {
                    let _ = std::fs::write(&path, json);
                }
            }
            return state;
        }
    }
    PersistedWordPlateState::default()
}

pub(crate) fn save_player_wordplates() {
    let path = player_wordplates_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let state = PersistedWordPlateState {
        caskets: PLAYER_CASKETS
            .read()
            .clone()
            .into_iter()
            .filter(|(k, _)| is_persisted_wordplate_owner(k))
            .collect(),
        loadouts: PLAYER_LOADOUTS
            .read()
            .clone()
            .into_iter()
            .filter(|(k, _)| is_persisted_wordplate_owner(k))
            .collect(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let tmp_path = path.with_extension("tmp");
        if std::fs::write(&tmp_path, json).is_ok() {
            let _ = std::fs::rename(tmp_path, path);
        }
    }
}

#[allow(dead_code)]
pub(crate) fn grant_crystal_or_word(
    pid: &str,
    word_id: &str,
    count: u32,
    quality: WordQuality,
    ratio: u8,
) {
    ensure_player_initialized(pid);
    {
        let mut caskets = PLAYER_CASKETS.write();
        if let Some(casket) = caskets.get_mut(pid) {
            casket.add_extracted(word_id, count, quality, ratio);
        }
    }
    save_player_wordplates();
}

/// 將多個單字字印一次性納入玩家詞匣，供手札參悟即刻就緒
pub fn grant_words_to_player_casket(pid: &str, word_ids: &[&str]) {
    ensure_player_initialized(pid);
    {
        let mut caskets = PLAYER_CASKETS.write();
        if let Some(casket) = caskets.get_mut(pid) {
            for &wid in word_ids {
                casket.add(wid, 1);
            }
        }
    }
    save_player_wordplates();
}

static PLAYER_CASKETS: LazyLock<RwLock<HashMap<String, WordCasket>>> = LazyLock::new(|| {
    let persisted = load_persisted_wordplates();
    RwLock::new(persisted.caskets)
});

static PLAYER_LOADOUTS: LazyLock<RwLock<HashMap<String, TacticalLoadout>>> = LazyLock::new(|| {
    let persisted = load_persisted_wordplates();
    RwLock::new(persisted.loadouts)
});

static STARTER_MANUAL_GRANTED: LazyLock<RwLock<HashSet<String>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

const DEFAULT_BASE_CHARS: &[&str] = &[
    "吾", "疾", "刺", "斬", "震", "卸", "盾", "步", "凝", "流", "破", "守",
];

fn resolve_word_id(registry: &WordRegistry, ch_or_id: &str) -> Option<String> {
    if registry.get_by_id(ch_or_id).is_some() {
        return Some(ch_or_id.to_string());
    }
    if let Some(ids) = registry.get_by_char(ch_or_id) {
        if let Some(first) = ids.first() {
            return Some(first.clone());
        }
    }
    // 若特定字不存在則選取最接近的基礎詞元
    let fallback = match ch_or_id {
        "疾" => "速",
        "卸" => "化",
        "步" => "避",
        _ => return registry.ordered_elements().next().map(|w| w.id.clone()),
    };
    if let Some(ids) = registry.get_by_char(fallback) {
        if let Some(first) = ids.first() {
            return Some(first.clone());
        }
    }
    registry.ordered_elements().next().map(|w| w.id.clone())
}

fn create_default_casket(registry: &WordRegistry) -> WordCasket {
    let mut casket = WordCasket::new();
    for &ch in DEFAULT_BASE_CHARS {
        if let Some(id) = resolve_word_id(registry, ch) {
            casket.add(id, 5);
        }
    }
    casket
}

fn create_default_loadout(registry: &WordRegistry) -> TacticalLoadout {
    crate::combat::wordplate::build_loadout_from_soul_seed(0, None, registry)
}

pub fn populate_resident_vocational_casket(pid: &str, casket: &mut WordCasket, registry: &WordRegistry) -> bool {
    let mut matched_clue: Option<String> = None;

    if pid == "秦水生" {
        matched_clue = Some("水輪石磨匠|老堰頭・水輪石磨工坊|秦水生".to_string());
    } else {
        for slot in crate::store::WENYAO_HOUSEHOLD_SLOTS.iter().chain(crate::store::OUTER_HOUSEHOLD_SLOTS.iter()) {
            let s_name = slot.name();
            if s_name == pid || pid.ends_with(&format!("|{}", s_name)) {
                matched_clue = Some(format!("{}|{}|{}", slot.brief_tag, slot.work_name, s_name));
                break;
            }
        }
        if matched_clue.is_none() {
            let display_title = crate::db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.display_title)
                .filter(|t| !t.trim().is_empty());
            if let Some(dt) = display_title {
                matched_clue = Some(dt);
            } else {
                // 嘗試從全島 59 位名宿作息排程中比對其職銜與姓名
                let mut h: u64 = 0xcbf2_9ce4_8422_2325;
                for b in pid.as_bytes() {
                    h ^= u64::from(*b);
                    h = h.wrapping_mul(0x0100_0000_01b3);
                }
                let target_settlement = crate::store::canonical_npc_macro_settlement(pid, "").unwrap_or("文曜");
                let matching: Vec<&'static crate::store::WenyaoHouseholdSlot> = if target_settlement == "文曜" {
                    crate::store::WENYAO_HOUSEHOLD_SLOTS.iter().collect()
                } else {
                    crate::store::OUTER_HOUSEHOLD_SLOTS
                        .iter()
                        .filter(|s| s.home_name.starts_with(target_settlement))
                        .collect()
                };
                if !matching.is_empty() {
                    let slot = matching[(h >> 8) as usize % matching.len()];
                    matched_clue = Some(format!("{}|{}|{}", slot.brief_tag, slot.work_name, pid));
                }
            }
        }
    }
    let clue = match matched_clue {
        Some(c) => c,
        None => return false,
    };

    let mut mutated = false;

    // 1. 核心百工/工匠詞元（每人至少 3 個字印）
    for ch in ["磨", "流", "純", "巧", "堅", "定", "沉", "水"] {
        if let Some(wid) = resolve_word_id(registry, ch)
            && casket.count(&wid) < 3
        {
            casket.add(&wid, 3);
            mutated = true;
        }
    }

    // 2. 領域專屬詞元（依線索加成，嚴守字庫限制）
    let domain_chars: &[&str] = if clue.contains("鍛") || clue.contains("鐵") || clue.contains("礦") || clue.contains("窯") {
        &["火", "破", "堅", "定"]
    } else if clue.contains("食堂") || clue.contains("廚") || clue.contains("麵") || clue.contains("烤") || clue.contains("茶") {
        &["火", "溫", "和", "純"]
    } else if clue.contains("醫") || clue.contains("藥") || clue.contains("診") || clue.contains("百草") || clue.contains("浴") {
        &["清", "純", "生", "和"]
    } else if clue.contains("木") || clue.contains("竹") || clue.contains("筏") || clue.contains("造船") {
        &["巧", "固", "正", "合"]
    } else if clue.contains("織") || clue.contains("染") || clue.contains("服") || clue.contains("繡") {
        &["柔", "密", "巧", "純"]
    } else if clue.contains("港") || clue.contains("船") || clue.contains("海") || clue.contains("漁") || clue.contains("渡") {
        &["風", "潮", "流", "定"]
    } else {
        &[]
    };

    for &ch in domain_chars {
        if let Some(wid) = resolve_word_id(registry, ch)
            && casket.count(&wid) < 3
        {
            casket.add(&wid, 3);
            mutated = true;
        }
    }

    // 3. 解析本職隨身器物並刻銘
    let vessel = crate::npc::causal_core::resolve_vocational_vessel(&clue);
    if casket.inscribed_words_of(vessel.item_id).is_empty() {
        let inscribe_pair: &[&str] = if vessel.is_bladed
            || clue.contains("鍛")
            || clue.contains("鐵")
            || clue.contains("木")
            || clue.contains("石")
        {
            &["巧", "堅"]
        } else {
            &["純", "流"]
        };
        for &ch in inscribe_pair {
            if let Some(wid) = resolve_word_id(registry, ch) {
                casket.add(&wid, 1);
                let _ = casket.inscribe_item(vessel.item_id, &wid);
                mutated = true;
            }
        }
    }

    // 石磨器具固有刻印相容性
    if casket.inscribed_words_of("wt_cat_0984").is_empty() {
        for ch in ["巧", "堅"] {
            if let Some(wid) = resolve_word_id(registry, ch) {
                casket.add(&wid, 1);
                let _ = casket.inscribe_item("wt_cat_0984", &wid);
                mutated = true;
            }
        }
    }
    if casket.inscribed_words_of("wt_cat_0973").is_empty() {
        for ch in ["純", "流"] {
            if let Some(wid) = resolve_word_id(registry, ch) {
                casket.add(&wid, 1);
                let _ = casket.inscribe_item("wt_cat_0973", &wid);
                mutated = true;
            }
        }
    }

    mutated
}

pub fn ensure_player_initialized(pid: &str) {
    let registry = WordRegistry::global();
    let mut mutated = false;
    let casket_opt = {
        let mut caskets = PLAYER_CASKETS.write();
        let is_new = !caskets.contains_key(pid);
        let casket = caskets
            .entry(pid.to_string())
            .or_insert_with(|| create_default_casket(registry));
        if is_new {
            mutated = true;
        }
        let resident_mutated = populate_resident_vocational_casket(pid, casket, registry);
        if resident_mutated {
            mutated = true;
        }
        Some(casket.clone())
    };
    {
        let mut loadouts = PLAYER_LOADOUTS.write();
        let soul_seed = db::get_entity(pid).ok().flatten().and_then(|e| e.soul_seed);
        let seed = crate::combat::human_phase::resolve_entity_soul_seed(pid, soul_seed);
        let needs_reset = match loadouts.get(pid) {
            None => true,
            Some(loadout) => {
                let has_legacy_name = |slot: &Option<StarSentence>| {
                    slot.as_ref()
                        .and_then(|s| s.custom_name.as_deref())
                        .map(|name| matches!(name, "疾刺斬" | "流破斬" | "吾震破" | "凝流步") || name.contains('・'))
                        .unwrap_or(false)
                };
                has_legacy_name(&loadout.opening)
                    || has_legacy_name(&loadout.midgame)
                    || has_legacy_name(&loadout.finisher)
                    || has_legacy_name(&loadout.defense)
            }
        };
        if needs_reset {
            let loadout = crate::combat::wordplate::build_loadout_from_soul_seed(seed, casket_opt.as_ref(), registry);
            loadouts.insert(pid.to_string(), loadout);
            mutated = true;
        } else if let Some(loadout) = loadouts.get_mut(pid)
            && (loadout.opening.is_none()
                || loadout.midgame.is_none()
                || loadout.finisher.is_none()
                || loadout.defense.is_none())
        {
            let default_loadout =
                crate::combat::wordplate::build_loadout_from_soul_seed(seed, casket_opt.as_ref(), registry);
            if loadout.opening.is_none() {
                loadout.opening = default_loadout.opening;
                mutated = true;
            }
            if loadout.midgame.is_none() {
                loadout.midgame = default_loadout.midgame;
                mutated = true;
            }
            if loadout.finisher.is_none() {
                loadout.finisher = default_loadout.finisher;
                mutated = true;
            }
            if loadout.defense.is_none() {
                loadout.defense = default_loadout.defense;
                mutated = true;
            }
        }
    }
    if mutated && is_persisted_wordplate_owner(pid) {
        save_player_wordplates();
    }
    if !(pid.starts_with("ambient_") || pid.starts_with("shanhai_") || pid.starts_with("npc_")) {
        if let Ok(Some(ent)) = db::get_entity(pid) {
            if !inventory_has_item(&ent.inventory, "blank_manual")
                && !inventory_has_item(&ent.inventory, "wt_blank_manual")
                && STARTER_MANUAL_GRANTED.write().insert(pid.to_string())
            {
                let grant_qty = 2;
                let _ = db::add_to_inventory(pid, "blank_manual", grant_qty);
            }
        }
    }
}

/// 一次性建世種子：為全島工坊與住戶匠人初始化器物磨損與吐納基底（所有人皆為自己的主角）
pub fn ensure_flagship_specimen_qin_shuisheng_wordplate_initialized() {
    let pid = "秦水生";
    ensure_player_initialized(pid);
    let registry = WordRegistry::global();
    let mut mutated = false;
    let casket_snapshot = {
        let mut caskets = PLAYER_CASKETS.write();
        let casket = caskets
            .entry(pid.to_string())
            .or_insert_with(|| create_default_casket(registry));
        if populate_resident_vocational_casket(pid, casket, registry) {
            mutated = true;
        }
        casket.clone()
    };
    if mutated {
        let mut loadouts = PLAYER_LOADOUTS.write();
        let soul_seed = db::get_entity(pid).ok().flatten().and_then(|e| e.soul_seed);
        let seed = crate::combat::human_phase::resolve_entity_soul_seed(pid, soul_seed);
        let loadout = crate::combat::wordplate::build_loadout_from_soul_seed(
            seed,
            Some(&casket_snapshot),
            registry,
        );
        loadouts.insert(pid.to_string(), loadout);
        drop(loadouts);
        save_player_wordplates();
    }
}

/// 供戰鬥模組（combat.rs）呼叫以獲取玩家之四戰術序位配置
pub fn get_player_tactical_loadout(pid: &str) -> TacticalLoadout {
    ensure_player_initialized(pid);
    let guard = PLAYER_LOADOUTS.read();
    guard.get(pid).cloned().unwrap_or_default()
}

/// 供戰鬥模組（combat.rs）呼叫以獲取玩家之隨身詞匣
pub fn get_player_word_casket(pid: &str) -> WordCasket {
    ensure_player_initialized(pid);
    let guard = PLAYER_CASKETS.read();
    guard.get(pid).cloned().unwrap_or_default()
}

/// 供測試或系統設定特定角色之隨身詞匣
#[cfg(test)]
pub fn set_player_word_casket(pid: &str, casket: WordCasket) {
    ensure_player_initialized(pid);
    let mut guard = PLAYER_CASKETS.write();
    guard.insert(pid.to_string(), casket);
}

/// 取得實體（玩家或匠人）當前裝配的百工四維總和（含槽位星句字印與手持/裝備器物銘文加成）
pub fn entity_total_craft_dims(pid: &str) -> CraftDims4D {
    ensure_player_initialized(pid);
    let registry = WordRegistry::global();
    let loadouts = PLAYER_LOADOUTS.read();
    let mut craft_totals = loadouts
        .get(pid)
        .map(|l| l.total_craft_dims(registry))
        .unwrap_or_default();
    drop(loadouts);

    let casket = get_player_word_casket(pid);
    if let Ok(Some(ent)) = crate::db::get_entity(pid)
        && let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&ent.equipment_slots)
    {
        for key in ["weapon", "hold_r", "hand", "tool", "hold_l"] {
            if let Some(weapon_id) = map.get(key).and_then(|v| v.as_str())
                && !weapon_id.is_empty()
            {
                let bonus = casket.inscribed_craft_bonus(weapon_id, registry);
                craft_totals.forge += bonus.forge;
                craft_totals.culinary += bonus.culinary;
                craft_totals.harvest += bonus.harvest;
                craft_totals.wayfare += bonus.wayfare;
            }
        }
    }
    craft_totals
}

fn sentence_to_view(
    slot_name: &str,
    sentence: &StarSentence,
    registry: &WordRegistry,
) -> TacticalSentenceView {
    let name = sentence
        .custom_name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| match sentence.synthesize(registry) {
            Ok(syn) => syn.effective_name,
            Err(_) => "無名招式".to_string(),
        });

    let words: Vec<String> = sentence
        .inlays
        .iter()
        .map(|inlay| {
            registry
                .get_by_id(&inlay.word_id)
                .map(|elem| elem.ch.clone())
                .unwrap_or_else(|| inlay.word_id.clone())
        })
        .collect();

    let (fluency_label, primary_carrier, primary_target) = match sentence.synthesize(registry) {
        Ok(syn) => (
            syn.fluency_summary.label().to_string(),
            syn.primary_carrier.label().to_string(),
            syn.primary_target.label().to_string(),
        ),
        Err(_) => (
            OrbitFluency::PingWen.label().to_string(),
            CarrierAffinity::Unarmed.label().to_string(),
            TargetPart::Torso.label().to_string(),
        ),
    };

    TacticalSentenceView {
        slot: slot_name.to_string(),
        name,
        words,
        nodes: sentence.inlays.iter().map(|inlay| inlay.node_id.clone()).collect(),
        fluency_label,
        primary_carrier,
        primary_target,
    }
}

fn loadout_to_view(loadout: &TacticalLoadout, registry: &WordRegistry) -> TacticalLoadoutView {
    TacticalLoadoutView {
        opening: loadout
            .opening
            .as_ref()
            .map(|s| sentence_to_view("opening", s, registry)),
        midgame: loadout
            .midgame
            .as_ref()
            .map(|s| sentence_to_view("midgame", s, registry)),
        finisher: loadout
            .finisher
            .as_ref()
            .map(|s| sentence_to_view("finisher", s, registry)),
        defense: loadout
            .defense
            .as_ref()
            .map(|s| sentence_to_view("defense", s, registry)),
    }
}

fn casket_to_entries(casket: &WordCasket, registry: &WordRegistry) -> Vec<CasketWordEntry> {
    let mut entries = Vec::new();
    for (word_id, count) in casket.list() {
        if let Some(elem) = registry.get_by_id(word_id) {
            entries.push(CasketWordEntry {
                id: word_id.to_string(),
                ch: elem.ch.clone(),
                count,
                carrier_affinity: elem.carrier_affinity.label().to_string(),
                target_part: elem.target_part.label().to_string(),
                quality_label: casket.quality_of(word_id).label_zh().to_string(),
                charge_pct: casket.charge_of(word_id),
                extraction_ratio: casket.ratio_of(word_id),
                semantic: elem.semantic.clone(),
                craft_label: elem.craft_dims.primary_aspect().label().to_string(),
            });
        }
    }
    entries
}

fn build_tactical_state_msg(pid: &str) -> TacticalStateMsg {
    ensure_player_initialized(pid);
    let registry = WordRegistry::global();
    let casket_guard = PLAYER_CASKETS.read();
    let loadout_guard = PLAYER_LOADOUTS.read();

    let casket = casket_guard
        .get(pid)
        .map(|c| casket_to_entries(c, registry))
        .unwrap_or_default();

    let loadout = loadout_guard
        .get(pid)
        .map(|l| loadout_to_view(l, registry))
        .unwrap_or_default();

    TacticalStateMsg {
        msg_type: "tactical_state".to_string(),
        casket,
        loadout,
    }
}

pub(super) fn handle_get_tactical_state(conn: &WsConnection) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let msg = build_tactical_state_msg(&pid);
    conn.send_json(&msg);
}

pub(super) fn handle_update_tactics(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };

    let slot = match msg.slot.to_lowercase().as_str() {
        "opening" => TacticalSlot::Opening,
        "midgame" => TacticalSlot::Midgame,
        "finisher" => TacticalSlot::Finisher,
        "defense" => TacticalSlot::Defense,
        _ => {
            conn.send_error("未知的演練路數槽位");
            return;
        }
    };

    let raw_inlays: Vec<(String, String)> = if !msg.inlays.is_empty() {
        msg.inlays.clone()
    } else if !msg.words.is_empty() {
        let fallback_node = msg.nodes.first().cloned().unwrap_or_else(|| "N001".to_string());
        let nodes: Vec<String> = if msg.nodes.len() == msg.words.len() {
            msg.nodes.clone()
        } else {
            (0..msg.words.len()).map(|_| fallback_node.clone()).collect()
        };
        nodes.into_iter().zip(msg.words.iter().cloned()).collect()
    } else {
        conn.send_error("未提供星盤節點與鑲嵌字韻");
        return;
    };

    let len = raw_inlays.len();
    if len != 3 && len != 5 && len != 7 {
        conn.send_error(format!(
            "造句字數不符規則：長度 {}（僅允許三言、五言、七言）",
            len
        ));
        return;
    }

    let registry = WordRegistry::global();
    let mut resolved_inlays = Vec::with_capacity(len);
    for (node_id, word_spec) in &raw_inlays {
        if let Err(e) = crate::combat::wordplate::parse_star_node(node_id) {
            conn.send_error(format!("星盤節點校驗失敗：{e}"));
            return;
        }
        let resolved_id = if registry.get_by_id(word_spec).is_some() {
            word_spec.clone()
        } else if let Some(id) = resolve_word_id(registry, word_spec) {
            id
        } else {
            conn.send_error(format!("字韻不存在：{word_spec}"));
            return;
        };
        resolved_inlays.push(StarInlay::new(node_id.clone(), resolved_id));
    }

    let sentence = match StarSentence::new(resolved_inlays) {
        Ok(s) => s,
        Err(e) => {
            conn.send_error(format!("造句校驗失敗：{e}"));
            return;
        }
    };

    ensure_player_initialized(&pid);

    let mut word_counts: HashMap<String, u32> = HashMap::new();
    for inlay in &sentence.inlays {
        *word_counts.entry(inlay.word_id.clone()).or_insert(0) += 1;
    }

    {
        let caskets = PLAYER_CASKETS.read();
        if let Some(casket) = caskets.get(&pid) {
            for (word_id, needed) in &word_counts {
                let owned = casket.count(word_id);
                if owned < *needed {
                    let ch = registry
                        .get_by_id(word_id)
                        .map(|w| w.ch.as_str())
                        .unwrap_or(word_id.as_str());
                    conn.send_error(format!(
                        "字匣中字韻數量不足：{}（現有 {}，需求 {}）",
                        ch, owned, needed
                    ));
                    return;
                }
            }
        }
    }

    {
        let mut loadouts = PLAYER_LOADOUTS.write();
        if let Some(loadout) = loadouts.get_mut(&pid) {
            loadout.set_slot(slot, sentence);
        } else {
            let mut loadout = create_default_loadout(registry);
            loadout.set_slot(slot, sentence);
            loadouts.insert(pid.clone(), loadout);
        }
    }
    save_player_wordplates();
    push_refresh(conn, &pid);
    let state_msg = build_tactical_state_msg(&pid);
    conn.send_json(&state_msg);

    let effective_name = state_msg
        .loadout
        .opening
        .as_ref()
        .filter(|_| matches!(slot, TacticalSlot::Opening))
        .or(state_msg.loadout.midgame.as_ref().filter(|_| matches!(slot, TacticalSlot::Midgame)))
        .or(state_msg.loadout.finisher.as_ref().filter(|_| matches!(slot, TacticalSlot::Finisher)))
        .or(state_msg.loadout.defense.as_ref().filter(|_| matches!(slot, TacticalSlot::Defense)))
        .map(|v| v.name.clone())
        .unwrap_or_else(|| msg.words.join(""));
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "UpdateTactics".into(),
        target_id: msg.slot.clone(),
        target_name: effective_name.clone(),
        narrative: format!("你凝神引字入樞，氣隨星位流轉，凝成一式「{}」。", effective_name),
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}


/// 後台根據人物先天骨相、內息底色、所學手札、所熟悉器物與山海歷練自然生成技能列表與經歷紀錄。
/// 這些技能並非玩家手動自訂，而是在戰鬥交鋒與百工生產中由後台自動施展。
pub fn synthesize_entity_natural_skills(ent: &Character) -> Vec<NaturalSkillView> {
    ensure_player_initialized(&ent.id);
    let registry = WordRegistry::global();

    let (equip_slots, equip_names, _) = parse_equipment_triple(&ent.equipment_slots);
    let held_keys = ["hold_r", "hold_l", "weapon", "hand", "tool"];
    let mut held_item_id = String::new();
    let mut held_item_display = String::new();
    let mut martial_item_id = String::new();
    let mut martial_item_display = String::new();
    for k in &held_keys {
        if let Some(id) = equip_slots.get(*k).filter(|s| !s.is_empty()) {
            let base = equip_names
                .get(*k)
                .cloned()
                .unwrap_or_else(|| resolve_inv_item_name(id));
            let display = format_entity_item_with_inscription(&ent.id, id, &base);
            if held_item_id.is_empty() {
                held_item_id = id.clone();
                held_item_display = display.clone();
            }
            if martial_item_id.is_empty()
                && crate::pg::living_world::is_weapon_or_tool_item(id, &base)
            {
                martial_item_id = id.clone();
                martial_item_display = display;
            }
        }
    }

    let held_carrier = if !martial_item_id.is_empty() {
        crate::combat::wordplate::infer_item_carrier_affinity(&martial_item_id, &martial_item_display)
    } else {
        None
    };

    let raw_vocation = {
        let t = ent.display_title.trim();
        if t.is_empty() || t == "山海行旅者" {
            ""
        } else {
            t
        }
    };
    let seed = crate::combat::human_phase::resolve_entity_soul_seed(&ent.id, ent.soul_seed);
    let bone = crate::combat::human_phase::InnateBonePhase::from_seed(seed);
    let habit = crate::pg::living_world::get_or_derive_breath_habit(&ent.id, raw_vocation, seed);
    let imprints = crate::pg::living_world::get_entity_lineage_imprints(&ent.id);
    let studied_manuals = crate::pg::living_world::get_entity_studied_manuals(&ent.id);
    let anatomy_total = crate::pg::living_world::get_entity_anatomy_familiarity_total(&ent.id);
    let active_afflictions = crate::pg::living_world::get_entity_afflictions(&ent.id);

    let mut breath_origin = habit.dominant.somatic_description().to_string();
    if !active_afflictions.is_empty() {
        let aff_labels: Vec<_> = active_afflictions.iter().map(|a| a.label()).collect();
        breath_origin.push_str(&format!("（身帶創傷：{}）", aff_labels.join("、")));
    }

    let vessel_name = if martial_item_display.is_empty() {
        "徒手".to_string()
    } else {
        martial_item_display.clone()
    };
    let vessel_familiarity_count = if !martial_item_id.is_empty() {
        crate::pg::living_world::get_vessel_state(&ent.id, &martial_item_id).use_count as u32
    } else {
        0
    };
    let learned_bonus = (studied_manuals.len() as f64 * 0.25).min(1.0);

    let mut skills = Vec::with_capacity(14);

    // ─── 領域一：骨相與氣息底色（bone_phase）─────────────────────────────
    skills.push(NaturalSkillView {
        domain: "bone_phase".to_string(),
        category: "innate_bone".to_string(),
        category_label: "天生體魄".to_string(),
        name: bone.bone_archetype_title(),
        origin_basis: bone.describe_bone_phase_zh(),
        learned_and_familiar: bone.auto_trigger_summary_zh(),
        auto_trigger_desc: String::new(),
    });

    skills.push(NaturalSkillView {
        domain: "bone_phase".to_string(),
        category: "inner_breath".to_string(),
        category_label: "內息底色".to_string(),
        name: habit.dominant.label().to_string(),
        origin_basis: breath_origin,
        learned_and_familiar: "周天吐納（隨日常打鐵、涉山、涉水、靜修與交鋒自然演進，行氣節奏漸次沉入筋骨）".to_string(),
        auto_trigger_desc: format!("{}。{}", habit.dominant.combat_rhythm_phrase(), habit.dominant.effect_summary()),
    });

    // ─── 領域二：臨陣武相（martial，4 項常駐）─────────────────────────────
    let martial_slots = [
        ("opening", TacticalSlot::Opening),
        ("midgame", TacticalSlot::Midgame),
        ("finisher", TacticalSlot::Finisher),
        ("defense", TacticalSlot::Defense),
    ];

    let martial_carrier = held_carrier.unwrap_or(CarrierAffinity::Unarmed);
    let player_loadout = PLAYER_LOADOUTS.read().get(&ent.id).cloned();

    for (cat, slot) in martial_slots {
        let studied_sent_opt = player_loadout.as_ref().and_then(|ld| match slot {
            TacticalSlot::Opening => ld.opening.as_ref(),
            TacticalSlot::Midgame => ld.midgame.as_ref(),
            TacticalSlot::Finisher => ld.finisher.as_ref(),
            TacticalSlot::Defense => ld.defense.as_ref(),
        });

        let routine = if let Some(sent) = studied_sent_opt.filter(|s| {
            !studied_manuals.is_empty()
                && s.custom_name
                    .as_deref()
                    .is_some_and(|n| {
                        !n.contains("順紋卯合")
                            && !n.contains("火候收汁")
                            && studied_manuals.iter().any(|m| n.contains(m) || m.contains(n))
                    })
        }) {
            let manual_name = sent.custom_name.as_deref().unwrap_or("承襲手札");
            let manual_word_chars: Vec<String> = sent
                .inlays
                .iter()
                .map(|inlay| {
                    registry
                        .get_by_id(&inlay.word_id)
                        .map(|e| e.ch.clone())
                        .unwrap_or_else(|| inlay.word_id.clone())
                })
                .collect();
            let (adapted, _) = crate::combat::human_phase::adapt_manual_to_bone_phase(
                &bone,
                slot,
                martial_carrier,
                manual_name,
                &manual_word_chars,
                registry,
            );
            adapted
        } else {
            crate::combat::human_phase::derive_personal_martial_routine(
                &bone,
                slot,
                martial_carrier,
                &vessel_name,
                learned_bonus,
                vessel_familiarity_count,
                registry,
            )
        };

        let mut affliction_tags = Vec::new();
        for aff in &active_afflictions {
            match aff {
                crate::pg::living_world::AfflictionKind::ArmSprain => {
                    if martial_carrier != CarrierAffinity::Footwork {
                        affliction_tags.push("【帶傷：腕肘挫傷】");
                    }
                }
                crate::pg::living_world::AfflictionKind::LegSprain => {
                    if slot == TacticalSlot::Defense || martial_carrier == CarrierAffinity::Footwork {
                        affliction_tags.push("【帶傷：下盤扭傷】");
                    }
                }
                crate::pg::living_world::AfflictionKind::TorsoInternal => {
                    affliction_tags.push("【帶傷：胸脅內傷】");
                }
                crate::pg::living_world::AfflictionKind::HeadConcussion => {
                    affliction_tags.push("【帶傷：頭面震盪】");
                }
                crate::pg::living_world::AfflictionKind::Miasma => {
                    affliction_tags.push("【帶傷：瘴氣入體】");
                }
            }
        }
        let affliction_suffix = if !affliction_tags.is_empty() {
            format!("；{}", affliction_tags.join("、"))
        } else {
            String::new()
        };

        skills.push(NaturalSkillView {
            domain: "martial".to_string(),
            category: cat.to_string(),
            category_label: format!("{}・{}", routine.slot_label_zh, routine.phase_specialty_zh),
            name: routine.formal_name,
            origin_basis: routine.origin_basis_zh,
            learned_and_familiar: format!("{}{}", routine.learned_and_familiar_zh, affliction_suffix),
            auto_trigger_desc: routine.auto_trigger_desc_zh,
        });
    }

    // ─── 領域三：百工藝相（production，4 項常駐）─────────────────────────
    let craft_specs = [
        ("harvest", held_carrier.unwrap_or(CarrierAffinity::Blade)),
        ("forge", CarrierAffinity::Blunt),
        ("culinary", CarrierAffinity::Blade),
        ("wayfare", CarrierAffinity::Footwork),
    ];

    for (craft_code, carrier) in craft_specs {
        let craft_tool_name = match craft_code {
            "harvest" => {
                if !martial_item_display.is_empty() {
                    martial_item_display.as_str()
                } else {
                    "採獵刀索與隨身器具"
                }
            }
            "forge" => "鍛錘、刻刀與百工器具",
            "culinary" => "庖廚刀勺與灶火器皿",
            _ => "行囊步履與隨身行具",
        };
        let routine = crate::combat::human_phase::derive_personal_craft_routine(
            &bone,
            craft_code,
            carrier,
            craft_tool_name,
            vessel_familiarity_count,
            anatomy_total,
            registry,
        );

        let mut craft_afflictions = Vec::new();
        for aff in &active_afflictions {
            match aff {
                crate::pg::living_world::AfflictionKind::ArmSprain => {
                    if carrier != CarrierAffinity::Footwork {
                        craft_afflictions.push("【帶傷：腕肘挫傷】");
                    }
                }
                crate::pg::living_world::AfflictionKind::LegSprain => {
                    if craft_code == "wayfare" {
                        craft_afflictions.push("【帶傷：下盤扭傷】");
                    }
                }
                crate::pg::living_world::AfflictionKind::TorsoInternal => {
                    if craft_code == "forge" || craft_code == "wayfare" {
                        craft_afflictions.push("【帶傷：胸脅內傷】");
                    }
                }
                crate::pg::living_world::AfflictionKind::HeadConcussion => {
                    if craft_code == "harvest" {
                        craft_afflictions.push("【帶傷：頭面震盪】");
                    }
                }
                crate::pg::living_world::AfflictionKind::Miasma => {
                    craft_afflictions.push("【帶傷：瘴氣入體】");
                }
            }
        }
        let craft_aff_suffix = if !craft_afflictions.is_empty() {
            format!("；{}", craft_afflictions.join("、"))
        } else {
            String::new()
        };

        skills.push(NaturalSkillView {
            domain: "production".to_string(),
            category: craft_code.to_string(),
            category_label: format!("{}・{}", routine.slot_label_zh, routine.phase_specialty_zh),
            name: routine.formal_name,
            origin_basis: routine.origin_basis_zh,
            learned_and_familiar: format!("{}{}", routine.learned_and_familiar_zh, craft_aff_suffix),
            auto_trigger_desc: routine.auto_trigger_desc_zh,
        });
    }

    // ─── 領域四：所學手札與器物歷練（mastery_ledger，3 項抽屜卡片）─────────
    skills.push(NaturalSkillView {
        domain: "mastery_ledger".to_string(),
        category: "learned_manuals".to_string(),
        category_label: "所學・師承與手札".to_string(),
        name: "師承與手札印記".to_string(),
        origin_basis: if studied_manuals.is_empty() {
            "尚未展卷研讀各家手札，全憑日常交手與勞作摸索門道。".to_string()
        } else {
            let titles = studied_manuals
                .iter()
                .map(|m| format!("《{}》", m))
                .collect::<Vec<_>>()
                .join("、");
            format!("平日展卷參詳{}等手札經卷，揣摩前人行氣與百工造物之法度。", titles)
        },
        learned_and_familiar: if imprints.is_empty() {
            "平日多憑自身揣摩習練，尚未得前輩名宿親口指點。".to_string()
        } else {
            let s = imprints
                .iter()
                .map(|imp| format!("曾蒙{}親授「{}」心法", imp.mentor_name, imp.imprint_title))
                .collect::<Vec<_>>()
                .join("、");
            format!("{}。", s)
        },
        auto_trigger_desc: "所習手札與前輩指點已融入日常吐納與出手習慣，臨陣及勞作時自然應變。".to_string(),
    });

    let is_bladed_held = !held_item_id.is_empty()
        && crate::pg::living_world::is_bladed_vessel_item(&held_item_id, &held_item_display);
    let is_weapon_held = !held_item_id.is_empty()
        && crate::pg::living_world::is_weapon_or_tool_item(&held_item_id, &held_item_display);

    skills.push(NaturalSkillView {
        domain: "mastery_ledger".to_string(),
        category: "familiar_vessels".to_string(),
        category_label: "所熟・隨身器物磨合".to_string(),
        name: if held_item_display.is_empty() {
            "隨身持物：徒手".to_string()
        } else {
            format!("隨身持物：{}", held_item_display)
        },
        origin_basis: if held_item_display.is_empty() {
            "以血肉筋骨為器，長久修持鍛打。".to_string()
        } else if is_bladed_held {
            let vstate = crate::pg::living_world::get_vessel_state(&ent.id, &held_item_id);
            format!(
                "器物現況：{}（磨合度：{}）",
                vstate.status_badge(),
                vstate.familiarity().label()
            )
        } else if is_weapon_held {
            let vstate = crate::pg::living_world::get_vessel_state(&ent.id, &held_item_id);
            format!(
                "器物現況：器身完好（磨合度：{}）",
                vstate.familiarity().label()
            )
        } else {
            "器物現況：隨身攜行（日常器物）".to_string()
        },
        learned_and_familiar: if held_item_display.is_empty() {
            "掌拳合一，收發自如。".to_string()
        } else if is_weapon_held {
            let vstate = crate::pg::living_world::get_vessel_state(&ent.id, &held_item_id);
            format!("{}。", vstate.familiarity().feel_prose())
        } else {
            "日常隨身攜行之物，取用順手。".to_string()
        },
        auto_trigger_desc: if is_bladed_held {
            "手持兵刃經反覆磨合，發力順應刀刃重心與硬度，減免鋒口磨損並提升破綻穿透。".to_string()
        } else if is_weapon_held {
            "手持器物經反覆磨合，發力順應器身重心與分量，運使愈發沉穩順手。".to_string()
        } else {
            "供平日行旅或起居取用；臨陣交鋒時則憑拳腳身法或隨身兵刃應敵。".to_string()
        },
    });

    skills.push(NaturalSkillView {
        domain: "mastery_ledger".to_string(),
        category: "material_anatomy".to_string(),
        category_label: "所歷・山海肌理與百工".to_string(),
        name: "山海肌理與百工體察".to_string(),
        origin_basis: crate::pg::living_world::get_entity_anatomy_familiarity_summary(&ent.id),
        learned_and_familiar: if anatomy_total >= 10 {
            "長年於山海之間採獵、庖解與鍛冶，熟知各類走獸異木之關節空隙與紋理走向。".to_string()
        } else if anatomy_total > 0 {
            "於日常採集與處理獵物時留心觀察皮肉筋節，漸能順著自然紋理落手。".to_string()
        } else {
            "平日多依常理下刀運錘，尚在摸索各類物材之肌理走向。".to_string()
        },
        auto_trigger_desc: "面對堅硬外皮或礦石結構時，能順著紋理縫隙落刃下錘，避開硬處、省力耐用。".to_string(),
    });

    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_residents_receive_vocational_word_casket_and_inscribed_vessel() {
        let targets = [
            "秦水生",
            "陳萬福",
            "岑守岳",
            "顧海平",
            crate::store::OUTER_HOUSEHOLD_SLOTS[0].name(),
        ];

        for pid in targets {
            let casket = get_player_word_casket(pid);
            assert!(
                casket.total_count() >= 24,
                "住民 {pid} 之隨身詞匣應被初始化並具備基礎百工詞元"
            );

            // 解析住民對應之本職器物
            let clue = if pid == "秦水生" {
                "水輪石磨匠|老堰頭・水輪石磨工坊|秦水生".to_string()
            } else if let Some(slot) = crate::store::WENYAO_HOUSEHOLD_SLOTS
                .iter()
                .chain(crate::store::OUTER_HOUSEHOLD_SLOTS.iter())
                .find(|s| s.name() == pid)
            {
                format!("{}|{}|{}", slot.brief_tag, slot.work_name, slot.name())
            } else {
                let mut h: u64 = 0xcbf2_9ce4_8422_2325;
                for b in pid.as_bytes() {
                    h ^= u64::from(*b);
                    h = h.wrapping_mul(0x0100_0000_01b3);
                }
                let target_settlement = crate::store::canonical_npc_macro_settlement(pid, "").unwrap_or("文曜");
                let matching: Vec<&'static crate::store::WenyaoHouseholdSlot> = if target_settlement == "文曜" {
                    crate::store::WENYAO_HOUSEHOLD_SLOTS.iter().collect()
                } else {
                    crate::store::OUTER_HOUSEHOLD_SLOTS
                        .iter()
                        .filter(|s| s.home_name.starts_with(target_settlement))
                        .collect()
                };
                let slot = matching[(h >> 8) as usize % matching.len()];
                format!("{}|{}|{}", slot.brief_tag, slot.work_name, pid)
            };
            let vessel = crate::npc::causal_core::resolve_vocational_vessel(&clue);
            assert!(
                !casket.inscribed_words_of(vessel.item_id).is_empty(),
                "住民 {pid} 之本職器物 {} 應被銘刻專屬字印",
                vessel.item_id
            );
        }
    }
}
