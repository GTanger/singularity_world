//! 背包／裝備／狀態查詢 + 拓撲除錯。

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use parking_lot::RwLock;

use serde::{Deserialize, Serialize};

use crate::ai::narrative::{SkillDataPacket, synthesize_skill_from_data};
use crate::combat::wordplate::{
    derive_elementary_motions_from_chars, derive_elementary_motions_with_fluency,
    CarrierAffinity, CraftDims4D, OrbitFluency, StarInlay, StarSentence, TacticalLoadout,
    TacticalSlot, TargetPart, WordCasket, WordElement, WordPlateError, WordQuality,
    WordRegistry, WordSource,
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

/// 檢查 entity_id 在 PLAYER_CASKETS 中是否有針對 item_id 之銘文，若有則綴上 ` [字1][字2]`
pub fn format_entity_item_with_inscription(entity_id: &str, item_id: &str, base_name: &str) -> String {
    let caskets = PLAYER_CASKETS.read();
    if let Some(casket) = caskets.get(entity_id) {
        let suffix = casket.format_inscribed_suffix(item_id, WordRegistry::global());
        if !suffix.is_empty() {
            return format!("{base_name}{suffix}");
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
        let npc_seed = ent.soul_seed.unwrap_or(0);
        for corpse_id in corpse_items {
            let corpse_rec = crate::pg::living_world::get_beast_corpse(&corpse_id);
            let _ = remove_from_inventory(npc_id, &corpse_id, 1);
            crate::pg::living_world::consume_beast_corpse(&corpse_id);
            let mut hasher: u64 = 0xcbf29ce484222325;
            for b in ent.id.as_bytes() {
                hasher ^= *b as u64;
                hasher = hasher.wrapping_mul(0x100000001b3);
            }
            for b in corpse_id.as_bytes() {
                hasher ^= *b as u64;
                hasher = hasher.wrapping_mul(0x100000001b3);
            }
            hasher ^= npc_seed.unsigned_abs();
            hasher = hasher.wrapping_mul(0x100000001b3);
            let tier = corpse_rec.as_ref().map(|r| r.tier).unwrap_or(3);
            let strip_quality = crate::combat::wordplate::roll_strip_quality(35.0, tier, true, hasher);
            if matches!(strip_quality, WordQuality::Jia | WordQuality::Yi | WordQuality::Bing) {
                let crystal_id = crate::combat::wordplate::crystal_item_id(
                    WordSource::Beast,
                    strip_quality,
                );
                let _ = db::add_to_inventory(npc_id, &crystal_id, 1);
            } else {
                let primary_loot = corpse_rec
                    .as_ref()
                    .map(|r| r.primary_loot_id.as_str())
                    .unwrap_or("mat_beast_hide");
                let _ = db::add_to_inventory(npc_id, primary_loot, 1);
                let _ = db::add_to_inventory(npc_id, "wt_beast_bone", 1);
            }
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

    Some(format!(
        "{}並以自身氣機將【「{}」】字烙印其上，隨手佩掛【{} [{}]】。",
        npc_name, elem.ch, crafted_item_name, elem.ch
    ))
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

fn entity_status_from_character(ent: &Character, is_self: bool) -> EntityStatusMsg {
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
    };
    if !ent.display_title.is_empty() {
        msg.display_title.clone_from(&ent.display_title);
    }
    if is_self {
        msg.activated_nodes = parse_activated_nodes(&ent.activated_nodes);
        if let Some(seed) = ent.soul_seed {
            msg.origin_sentence = expand_soul_seed_to_origin_sentence(seed);
            msg.topology_costs = expand_soul_seed_to_topology_costs(seed);
        }
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
    if def.slot.is_empty() {
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
        let casket = get_player_word_casket(&pid);
        let loadout = {
            let guard = PLAYER_LOADOUTS.read();
            guard.get(&pid).cloned().unwrap_or_else(|| {
                let seed = ent.soul_seed.unwrap_or(100_003);
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
            conn.send_error("此路數尚未排定字譜星軌，無法著譜立說。");
            return;
        };

        let word_ids: Vec<String> = sent.inlays.iter().map(|i| i.word_id.clone()).collect();
        let word_chars: Vec<String> = sent
            .inlays
            .iter()
            .map(|inlay| {
                registry
                    .get_by_id(&inlay.word_id)
                    .map(|e| e.ch.clone())
                    .unwrap_or_else(|| inlay.word_id.clone())
            })
            .collect();

        let syn_opt = sent.synthesize(registry).ok();
        let carrier = syn_opt.as_ref().map(|s| s.primary_carrier).unwrap_or(CarrierAffinity::Unarmed);
        let motion_seq = syn_opt
            .as_ref()
            .map(|s| s.derive_elementary_motions_for_carrier(Some(carrier)))
            .unwrap_or_else(|| {
                derive_elementary_motions_from_chars(&word_chars, carrier, TargetPart::Torso)
            });

        let role_hint = if !ent.display_title.trim().is_empty() {
            ent.display_title.trim().to_string()
        } else {
            "山海行旅者".to_string()
        };

        let first_hub = sent
            .inlays
            .first()
            .map(|i| hub_label_from_node(&i.node_id))
            .unwrap_or_else(|| "天極主樞".to_string());

        let natural_name = syn_opt
            .as_ref()
            .map(|s| s.effective_name.clone())
            .unwrap_or_else(|| format!("{}連式", word_chars.join("")));

        let packet = SkillDataPacket {
            entity_id: ent.id.clone(),
            role_title: role_hint,
            domain: "martial".to_string(),
            slot_label: slot_label.to_string(),
            hub_or_tier: first_hub,
            word_chars: word_chars.clone(),
            vessel_name: "隨身套路".to_string(),
            carrier,
            motion_sequence: motion_seq,
            fallback_name: natural_name,
        };

        let synthesized = synthesize_skill_from_data(&packet);
        let formal_name = synthesized.name;
        let origin_basis = synthesized.origin_basis;

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
                "你鋪開空白手札，凝神將平生習熟的「{}」行氣路線與「{}」諸字真意逐筆錄下，裝訂成一冊《{}》手札收入行囊。",
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
            conn.send_error("此卷手札殘損模糊，字印氣脈無從辨識。");
            return;
        };
        if manual.word_ids.len() < 3 {
            conn.send_error("此冊手札字印殘缺，無法成句。");
            return;
        }

        let registry = WordRegistry::global();
        let casket = get_player_word_casket(&pid);

        let in_loadout_has_word = |wid: &str| -> bool {
            let guard = PLAYER_LOADOUTS.read();
            guard.get(&pid).map(|l| {
                let check_sent = |s: Option<&StarSentence>| {
                    s.map(|sent| sent.inlays.iter().any(|inlay| inlay.word_id == wid)).unwrap_or(false)
                };
                check_sent(l.opening.as_ref())
                    || check_sent(l.midgame.as_ref())
                    || check_sent(l.finisher.as_ref())
                    || check_sent(l.defense.as_ref())
            }).unwrap_or(false)
        };

        let mut grasped_chars = Vec::new();
        let mut missing_chars = Vec::new();
        for (i, wid) in manual.word_ids.iter().enumerate() {
            let ch = manual.word_chars.get(i).cloned().unwrap_or_else(|| {
                registry.get_by_id(wid).map(|e| e.ch.clone()).unwrap_or_else(|| wid.clone())
            });
            let in_casket = casket.count(wid) > 0;
            let in_loadout = in_loadout_has_word(wid);

            if in_casket || in_loadout {
                grasped_chars.push(ch);
            } else {
                missing_chars.push(ch);
            }
        }

        if !missing_chars.is_empty() {
            let narrative = if grasped_chars.is_empty() {
                format!(
                    "你細細研讀《{}》手札，發覺其中所載之「{}」諸字真意尚未採獵得獲，遂先將心得記為「參悟殘譜（待日後採獵剝名補齊字印方可完整成相）」，未能即刻化入星軌。",
                    manual.formal_name,
                    missing_chars.join("、")
                )
            } else {
                format!(
                    "你細細研讀《{}》手札，發覺其中所載之「{}」諸字真意，你目前僅通曉「{}」，其餘「{}」字印尚未採獵得獲，遂先將心得記為「參悟殘譜（待日後採獵剝名補齊字印方可完整成相）」，未能即刻化入星軌。",
                    manual.formal_name,
                    manual.word_chars.join("・"),
                    grasped_chars.join("・"),
                    missing_chars.join("、")
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

        let learner_seed = ent.soul_seed.unwrap_or(100_003);
        let costs = expand_soul_seed_to_topology_costs(learner_seed);

        let (slot_idx, slot_enum) = match manual.slot_label.as_str() {
            "起手" | "opening" => (0, TacticalSlot::Opening),
            "連攜" | "midgame" => (1, TacticalSlot::Midgame),
            "破綻" | "finisher" | "重手" => (2, TacticalSlot::Finisher),
            "守勢" | "defense" => (3, TacticalSlot::Defense),
            _ => (0, TacticalSlot::Opening),
        };

        let mut hub_scores: Vec<(usize, f64)> = (1..=20)
            .map(|h| {
                let a_cost = costs.get(h - 1).copied().unwrap_or(13.0);
                let b_start = 20 + 5 * (h - 1);
                let b_avg: f64 = (0..5)
                    .map(|j| costs.get(b_start + j).copied().unwrap_or(13.0))
                    .sum::<f64>()
                    / 5.0;
                let total_resistance = a_cost * 0.4 + b_avg * 0.6;
                (h, total_resistance)
            })
            .collect();
        hub_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let selected_hub = hub_scores[slot_idx % hub_scores.len()].0;

        let word_count = if manual.word_ids.len() >= 7 {
            7
        } else if manual.word_ids.len() >= 5 {
            5
        } else {
            3
        };

        let avg_cost = costs.iter().take(word_count.max(1)).sum::<f64>() / (word_count.max(1) as f64);
        let learner_fluency = OrbitFluency::from_cost(avg_cost);
        let inlays: Vec<StarInlay> = (0..word_count)
            .map(|i| {
                let node_id = format!("N{:03}", i + 1);
                let node_cost = costs.get(i).copied().unwrap_or(10.0);
                StarInlay::with_fluency(
                    node_id,
                    manual.word_ids[i].clone(),
                    OrbitFluency::from_cost(node_cost),
                )
            })
            .collect();
        let mut star_sentence = match StarSentence::new(inlays) {
            Ok(s) => s.with_custom_name(&manual.formal_name),
            Err(e) => {
                conn.send_error(format!("星軌成相滯澀：{e}"));
                return;
            }
        };
        star_sentence.set_soul_seed(learner_seed);
        let detour_note = match star_sentence.synthesize(registry) {
            Ok(synth) => synth.meridian_detour_note,
            Err(_) => String::new(),
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
        let carrier = if !held_item_id.is_empty() {
            crate::combat::wordplate::infer_item_carrier_affinity(&held_item_id, &held_item_display)
                .unwrap_or(CarrierAffinity::Blade)
        } else {
            crate::combat::wordplate::CarrierAffinity::from_code(&manual.carrier_code)
                .unwrap_or(CarrierAffinity::Unarmed)
        };

        let motions = derive_elementary_motions_with_fluency(
            &manual.word_chars,
            carrier,
            TargetPart::Torso,
            learner_fluency,
        );

        {
            let mut loadouts = PLAYER_LOADOUTS.write();
            if let Some(loadout) = loadouts.get_mut(&pid) {
                loadout.set_slot(slot_enum, star_sentence);
            } else {
                let mut loadout = create_default_loadout(registry);
                loadout.set_slot(slot_enum, star_sentence);
                loadouts.insert(pid.clone(), loadout);
            }
        }
        save_player_wordplates();
        push_refresh(conn, &pid);

        let motions_str = motions.join(" → ");
        let detour_suffix = if !detour_note.is_empty() {
            format!("（{}）", detour_note)
        } else {
            String::new()
        };
        let narrative = format!(
            "你手捧《{}》研讀，循自身三十六位天星氣脈運轉，經絡相映間「{}」諸字真意自識海流轉，氣機{}貫通；雖習同門心法，卻隨你自身體魄拓撲衍生出獨屬武相{}，碎招化作「{}」，順勢納入{}式！",
            manual.formal_name,
            manual.word_chars.join("・"),
            learner_fluency.label(),
            detour_suffix,
            motions_str,
            manual.slot_label
        );
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

    // ─── 器物刻字／外盤銘文（InscribeItem）───────────────────────────────────
    if msg.action == "InscribeItem" {
        ensure_player_initialized(&pid);
        let registry = WordRegistry::global();
        let (target_item_id, raw_word_id) = if let Some((target, wid)) = msg.item_id.split_once('|') {
            (target.trim(), wid.trim())
        } else {
            (msg.item_id.as_str().trim(), "")
        };

        if target_item_id.is_empty() {
            conn.send_error("未指定欲銘刻外盤字韻之器物");
            return;
        }

        let Ok(Some(ent)) = db::get_entity(&pid) else {
            conn.send_error(gametext::client("inv_entity_not_found"));
            return;
        };

        let is_in_inventory = inventory_has_item(&ent.inventory, target_item_id);
        let is_equipped = if ent.equipment_slots.is_empty() {
            false
        } else {
            let slots: HashMap<String, String> =
                serde_json::from_str(&ent.equipment_slots).unwrap_or_default();
            slots.values().any(|id| id == target_item_id)
        };

        if !is_in_inventory && !is_equipped {
            conn.send_error("行囊或身上並無此器物，無法銘刻外盤字韻");
            return;
        }

        let word_id = if !raw_word_id.is_empty() {
            let Some(wid) = resolve_word_id(registry, raw_word_id) else {
                conn.send_error(format!("未能辨識字印【{raw_word_id}】"));
                return;
            };
            wid
        } else {
            let caskets = PLAYER_CASKETS.read();
            let Some(casket) = caskets.get(&pid) else {
                conn.send_error("隨身詞匣尚未開啟");
                return;
            };
            let first_available = casket.list().into_iter().find(|(_wid, count)| *count > 0).map(|(wid, _)| wid.to_string());
            match first_available {
                Some(wid) => wid,
                None => {
                    conn.send_error("隨身詞匣內無可用之字韻詞元");
                    return;
                }
            }
        };

        let word_char = {
            let mut caskets = PLAYER_CASKETS.write();
            let Some(casket) = caskets.get_mut(&pid) else {
                conn.send_error("隨身詞匣未就緒");
                return;
            };
            if casket.count(&word_id) == 0 {
                let ch = registry.get_by_id(&word_id).map(|e| e.ch.as_str()).unwrap_or(&word_id);
                conn.send_error(format!("隨身詞匣中已無「{ch}」字詞元"));
                return;
            }
            match casket.inscribe_item(target_item_id, &word_id) {
                Ok(_) => {
                    registry.get_by_id(&word_id).map(|e| e.ch.clone()).unwrap_or_else(|| word_id.clone())
                }
                Err(e) => {
                    conn.send_error(format!("器物銘文失敗：{e}"));
                    return;
                }
            }
        };

        save_player_wordplates();
        push_refresh(conn, &pid);
        let state_msg = build_tactical_state_msg(&pid);
        conn.send_json(&state_msg);

        let base_name = resolve_inv_item_name(target_item_id);
        let inscribed_full_name = format_entity_item_with_inscription(&pid, target_item_id, &base_name);

        let narrative = format!(
            "你凝心定氣，自隨身詞匣引出一縷「{word_char}」字玄芒，於【{base_name}】之外盤肌理細細鏤刻！氣機貫注之下，器物泛起五源流轉之微芒，化為【{inscribed_full_name}】——銘文既成，此器於庖丁解牛、採獵百工中平添入微化境，於戰陣交鋒時更能與星盤經脈引動同律共鳴！"
        );

        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "InscribeItem".into(),
            target_id: target_item_id.to_string(),
            target_name: inscribed_full_name,
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

    // ─── 識海試排槽心印操作：同名疊煉（StackWord）與鎔鑄化鎂（MintWord） ───────────
    if msg.action == "StackWord" || msg.action == "MintWord" {
        ensure_player_initialized(&pid);
        let registry = WordRegistry::global();
        let word_id = if let Some(wid) = resolve_word_id(registry, &msg.item_id) {
            wid
        } else {
            conn.send_error("未能辨識欲操作之字印");
            return;
        };
        let word_ch = registry.get_by_id(&word_id).map(|e| e.ch.clone()).unwrap_or_else(|| word_id.clone());

        if msg.action == "StackWord" {
            let (target_quality, target_charge, upgraded, narrative) = {
                let mut caskets = PLAYER_CASKETS.write();
                let casket = match caskets.get_mut(&pid) {
                    Some(c) => c,
                    None => {
                        conn.send_error("詞匣尚未開竅");
                        return;
                    }
                };
                let loadouts = PLAYER_LOADOUTS.read();
                let loadout = loadouts.get(&pid);
                let xuan_bonus = loadout.map(|l| l.xuan_luck_bonus(casket, registry)).unwrap_or(0.0);
                let now = crate::game::now_unix();
                let seed = (now as u64)
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(pid.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64)));
                match casket.stack_refine(&word_id, xuan_bonus, seed) {
                    Ok(res) => res,
                    Err(e) => {
                        conn.send_error(format!("疊煉未成：{e}"));
                        return;
                    }
                }
            };
            save_player_wordplates();
            let state_msg = build_tactical_state_msg(&pid);
            conn.send_json(&state_msg);

            let result_title = if upgraded { "【同名疊煉・品階躍升】" } else { "【同名疊煉・充能蘊養】" };
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "StackWord".into(),
                target_id: word_id,
                target_name: format!("「{word_ch}」字心印"),
                narrative: format!("{result_title}{narrative}（當前品位：{}）", target_quality.label_zh()),
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return;
        }

        if msg.action == "MintWord" {
            let (mg_gain, quality, narrative) = {
                let mut caskets = PLAYER_CASKETS.write();
                let casket = match caskets.get_mut(&pid) {
                    Some(c) => c,
                    None => {
                        conn.send_error("詞匣尚未開竅");
                        return;
                    }
                };
                match casket.mint_to_magnesium(&word_id) {
                    Ok(res) => res,
                    Err(e) => {
                        conn.send_error(format!("化鎂未成：{e}"));
                        return;
                    }
                }
            };
            if let Err(e) = db::add_magnesium(&pid, mg_gain) {
                tracing::warn!("[MintWord] 增加鎂錠失敗：{e}");
            }
            save_player_wordplates();
            push_refresh(conn, &pid);
            let state_msg = build_tactical_state_msg(&pid);
            conn.send_json(&state_msg);

            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "MintWord".into(),
                target_id: word_id,
                target_name: format!("「{word_ch}」字心印"),
                narrative: format!("【鎔鑄化鎂】{narrative}（化解{}心印，凝結入行囊實得 {mg_gain} 鎂錠）", quality.label_zh()),
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return;
        }
    }
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    if !inventory_has_item(&ent.inventory, &msg.item_id) {
        conn.send_error(gametext::client("inv_bag_missing"));
        return;
    }
    // ─── 碎晶提出（ExtractWord / Use on crystal_*）───────────────────────────
    if let Some((source, quality)) = crate::combat::wordplate::parse_crystal_item_id(&msg.item_id) {
        if msg.action.is_empty() || msg.action == "ExtractWord" || msg.action == "Use" {
            if remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
                conn.send_error(gametext::client("inv_bag_fail"));
                return;
            }
            ensure_player_initialized(&pid);
            let registry = WordRegistry::global();
            let loadouts = PLAYER_LOADOUTS.read();
            let forge_score = loadouts
                .get(&pid)
                .map(|l| l.total_craft_dims(registry).forge)
                .unwrap_or(0.0);
            drop(loadouts);

            let (_, room_name) = entity_location_label(&ent);
            let is_official_agency = room_name.contains("文曜")
                || room_name.contains("講堂")
                || room_name.contains("學政")
                || room_name.contains("工坊")
                || room_name.contains("市");

            let ratio = crate::combat::wordplate::compute_extraction_ratio(forge_score as f32, is_official_agency);
            let now = crate::game::now_unix();
            let seed = (now as u64)
                .wrapping_mul(6364136223846793005)
                .wrapping_add(pid.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64)))
                .wrapping_add(source.code().bytes().fold(0u64, |acc, b| acc.wrapping_mul(17).wrapping_add(b as u64)));
            let elem = registry.roll_word_from_source(source, quality, seed);

            {
                let mut caskets = PLAYER_CASKETS.write();
                if let Some(casket) = caskets.get_mut(&pid) {
                    casket.add_extracted(&elem.id, 1, quality, ratio);
                }
            }
            save_player_wordplates();
            push_refresh(conn, &pid);
            let state_msg = build_tactical_state_msg(&pid);
            conn.send_json(&state_msg);

            let crystal_name = resolve_inv_item_name(&msg.item_id);
            let efficiency_note = if is_official_agency {
                "爐溫與陣勢相合，字韻凝結格外飽滿。"
            } else {
                "字韻隨心火緩緩收束成形。"
            };
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "ExtractWord".into(),
                target_id: elem.id.clone(),
                target_name: format!("「{}」字心印", elem.ch),
                narrative: format!(
                    "你運起心火碎解【{crystal_name}】，識海字譜中凝現一枚{}【「{}」】字心印。{efficiency_note}",
                    quality.label_zh(),
                    elem.ch
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
    }

    // ─── 剝名結晶（StripName / StripNameWhole）───────────────────────────────
    if msg.action == "StripName" || msg.action == "StripNameWhole" {
        let is_whole = msg.action == "StripNameWhole" || msg.item_id.starts_with("corpse_");
        let item_name = resolve_inv_item_name(&msg.item_id);
        let Some((source, tier)) = crate::game::object_ecology::classify_material_word_source(&msg.item_id, &item_name) else {
            conn.send_error("此物未具備可剝名凝晶之天地五源質性");
            return;
        };

        let now = crate::game::now_unix();
        let is_corpse = msg.item_id.starts_with("corpse_");
        let corpse_opt = if is_corpse {
            let c = crate::pg::living_world::get_beast_corpse(&msg.item_id);
            if c.is_none() {
                conn.send_error("此具殘軀放置過久，靈韻已散，僅可執行【肢體分解】。");
                return;
            }
            c
        } else {
            None
        };

        if let Some(corpse) = &corpse_opt {
            if now > corpse.fresh_until {
                conn.send_error("此具殘軀放置過久，周身靈韻已隨天地元氣散逸，無法再整軀剝名，僅可執行【肢體分解】取下皮骨。");
                return;
            }
        }

        if remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
            conn.send_error(gametext::client("inv_bag_fail"));
            return;
        }

        if msg.item_id.starts_with("corpse_") {
            crate::pg::living_world::consume_beast_corpse(&msg.item_id);
        }
        ensure_player_initialized(&pid);
        let registry = WordRegistry::global();
        let total_craft = {
            let loadouts = PLAYER_LOADOUTS.read();
            loadouts
                .get(&pid)
                .map(|l| l.total_craft_dims(registry))
                .unwrap_or_default()
        };
        let mut harvest_score = total_craft.harvest + 0.35 * total_craft.culinary;

        let equipment_map: HashMap<String, String> = if ent.equipment_slots.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&ent.equipment_slots).unwrap_or_default()
        };

        let mut tool_resonance = false;
        let mut resonance_tool_name = String::new();
        let held_keys = ["hold_r", "hold_l", "weapon", "hand", "tool"];
        {
            let caskets = PLAYER_CASKETS.read();
            let casket_opt = caskets.get(&pid);

            for key in &held_keys {
                if let Some(held_id) = equipment_map.get(*key).filter(|id| !id.is_empty()) {
                    let held_name = resolve_inv_item_name(held_id);
                    let is_tool = held_name.contains("刀")
                        || held_name.contains("剪")
                        || held_name.contains("鋸")
                        || held_name.contains("刃")
                        || held_name.contains("鎬")
                        || held_name.contains("斧")
                        || held_name.contains("鍬")
                        || held_name.contains("針")
                        || held_name.contains("刮")
                        || held_name.contains("工具")
                        || held_id.contains("blade")
                        || held_id.contains("knife")
                        || held_id.contains("tool")
                        || held_id.contains("axe");

                    if is_tool {
                        harvest_score += 15.0;
                    }

                    if let Some(casket) = casket_opt {
                        let craft_bonus = casket.inscribed_craft_bonus(held_id, registry);
                        harvest_score += craft_bonus.harvest + 0.5 * craft_bonus.culinary;

                        let inscribed = casket.inscribed_words_of(held_id);
                        let has_dissecting = inscribed.iter().any(|wid| {
                            let ch = registry.get_by_id(wid).map(|e| e.ch.as_str()).unwrap_or(wid);
                            matches!(ch, "解" | "剝" | "骨" | "筋" | "巧" | "薄" | "探")
                        });
                        if is_tool && (!inscribed.is_empty() || has_dissecting) {
                            tool_resonance = true;
                            harvest_score += 25.0;
                            if has_dissecting {
                                harvest_score += 20.0;
                            }
                            if resonance_tool_name.is_empty() {
                                resonance_tool_name = format_entity_item_with_inscription(&pid, held_id, &held_name);
                            }
                        }
                    }
                }
            }
        }

        let habit_rec = crate::pg::living_world::record_breath_activity(
            &pid,
            crate::pg::living_world::BreathHabitKind::ForestLong,
            2,
            now,
        );

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

        let seed = (now as u64)
            .wrapping_mul(6364136223846793005)
            .wrapping_add(pid.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64)))
            .wrapping_add(tier as u64);

        if is_whole {
            let success_threshold = (45.0 + (harvest_score as f32) * 0.65).min(92.0);
            if ((seed % 100) as f32) > success_threshold {
                push_refresh(conn, &pid);
                let fail_narrative = if let (Some((_, kname)), Some(vs)) = (&bladed_tool, &vstate) {
                    format!(
                        "你操持【{}】（{}），試圖趁餘溫將【{}】整具靈韻一氣剝離，孰料半途氣機微滯，殘軀晶光驟然潰散於天地之間，未能凝成結晶。{}",
                        kname,
                        vs.status_badge(),
                        item_name,
                        vs.familiarity().feel_prose()
                    )
                } else {
                    format!(
                        "你徒手試圖趁餘溫將【{}】整具靈韻一氣剝離，孰料半途氣機微滯，殘軀晶光驟然潰散於天地之間，未能凝成結晶。",
                        item_name
                    )
                };
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "StripNameWhole".into(),
                    target_id: msg.item_id.clone(),
                    target_name: item_name.clone(),
                    narrative: fail_narrative,
                    success: false,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
        }

        let quality = if is_whole {
            if (harvest_score >= 35.0) || ((seed % 10) < 5) {
                WordQuality::Jia
            } else {
                WordQuality::Yi
            }
        } else {
            crate::combat::wordplate::roll_strip_quality(harvest_score as f32, tier, is_whole, seed)
        };
        let crystal_id = crate::combat::wordplate::crystal_item_id(source, quality);
        let crystal_def = crate::game::object_ecology::lookup_ecology_item_def(&crystal_id);
        let crystal_name = crystal_def.map(|d| d.name).unwrap_or_else(|| "詞元結晶".to_string());

        if let Err(e) = db::add_to_inventory(&pid, &crystal_id, 1) {
            tracing::warn!("[StripName] 增加結晶至背包失敗：{e}");
        }
        push_refresh(conn, &pid);

        let whole_note = if is_whole { "藉整全完軀之靈韻引導，" } else { "" };
        let paoding_note = if tool_resonance {
            format!("，手中【{resonance_tool_name}】順著骨縫肌理輕巧游走，皮肉筋骨應手而解")
        } else {
            String::new()
        };
        let vessel_phrase = if let (Some((_, kname)), Some(vs)) = (&bladed_tool, &vstate) {
            format!("手中【{}】（{}），{}。", kname, vs.status_badge(), vs.familiarity().feel_prose())
        } else {
            "徒手運勁分理，手感沉著。".to_string()
        };
        let mut narrative = format!(
            "你凝神處理【{item_name}】，{whole_note}剝去雜質{paoding_note}，掌心凝結出一枚【{crystal_name}】（{}・{}）。{}",
            source.label_zh(),
            quality.label_zh(),
            vessel_phrase
        );

        let familiar_enough = vstate.as_ref().map(|vs| matches!(
            vs.familiarity(),
            crate::pg::living_world::VesselFamiliarity::WellBalanced
                | crate::pg::living_world::VesselFamiliarity::LikeOwnArm
        )).unwrap_or(false);

        if is_whole || familiar_enough {
            let actor_name = if !ent.display_title.trim().is_empty() {
                ent.display_title.trim().to_string()
            } else {
                "行者".to_string()
            };
            let tool_name = bladed_tool.as_ref().map(|(_, n)| n.clone()).unwrap_or_else(|| "徒手".to_string());
            let tool_familiarity = vstate.as_ref().map(|vs| vs.familiarity().label().to_string()).unwrap_or_else(|| "得心應手".to_string());
            let packet = crate::ai::CraftActionFactPacket {
                actor_name,
                tool_name,
                tool_familiarity,
                breath_habit: habit_rec.dominant.label().to_string(),
                material_names: vec![item_name.clone()],
                output_name: crystal_name.clone(),
                highlight_kind: if is_whole { "整軀剝名" } else { "剝名凝晶" }.to_string(),
                environment_hint: "荒澤曠野".to_string(),
            };
            let highlight = crate::ai::compose_craft_highlight_from_data(&packet);
            if !highlight.is_empty() {
                narrative.push_str("\n\n");
                narrative.push_str(&highlight);
            }
        }

        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "StripName".into(),
            target_id: crystal_id,
            target_name: crystal_name.to_string(),
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
    // ─── 肢體分解（ButcherCorpse）───────────────────────────────────────────
    if msg.action == "ButcherCorpse" {
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
            .unwrap_or("mat_beast_hide");
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


    if crate::game::object_ecology::is_wood_or_firewood_item(&msg.item_id)
        || msg.item_id == "mat_charcoal_sack"
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
    ) || (crate::game::object_ecology::is_mineral_item(&msg.item_id) && msg.action.is_empty()) {
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
    ) || (crate::game::object_ecology::is_herb_item(&msg.item_id) && msg.action.is_empty() && !msg.item_id.starts_with("craft_")) {
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
    ) || (crate::game::object_ecology::is_fiber_item(&msg.item_id) && msg.action.is_empty()) {
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
    ) || (crate::game::object_ecology::is_hide_fish_item(&msg.item_id) && msg.action.is_empty()) {
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
    if remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    let is_meridian_salve = msg.item_id == "craft_meridian_salve" || msg.item_id.contains("meridian_salve");
    let is_calming_tea = msg.item_id == "craft_calming_tea_pack" || msg.item_id.contains("calming_tea");
    let is_mutated = msg.item_id.starts_with("mut_") || crate::game::object_ecology::classify_item_stage(&msg.item_id) == "mutated";

    if let Some(arc) = crate::store::get_store() {
        let _ = arc.write().update_entity(&pid, |e| {
            if is_meridian_salve {
                e.vit = (e.vit + 25).min(100);
                e.qi = (e.qi + 20).min(100);
            } else if is_calming_tea {
                e.qi = (e.qi + 30).min(100);
            } else if is_mutated {
                e.vit = (e.vit + 15).min(100);
                e.qi = (e.qi + 20).min(100);
            } else {
                e.vit = (e.vit + 8).min(100);
                e.qi = (e.qi + 10).min(100);
            }
        });
    }
    push_refresh(conn, &pid);

    let item_name = format_entity_item_with_inscription(&pid, &msg.item_id, &resolve_inv_item_name(&msg.item_id));

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
        format!("你取出一份【{item_name}】塗抹於周身經絡要穴。溫潤的藥力順著經脈化開，氣血大暢，肌肉酸痛盡消（周身氣血與心神皆獲豐沛滋養）！")
    } else if is_calming_tea {
        format!("你沏開這份【{item_name}】細細品飲。茶湯清洌甘甜，靈氣透入靈台，心神澄澈通達（心神頓時澄澈充盈）！")
    } else if is_mutated {
        format!("你凝神運用手中神異的【{item_name}】。微芒順著指尖湧入經脈，天地共鳴之氣充盈四肢百骸，周身氣脈頓時大振（體魄與元氣漸次平順舒暢）！")
    } else if is_food {
        format!("你取出一份熱騰騰的【{item_name}】，大口趁熱品嚐。外皮酥香、內餡鮮美多汁，溫潤的香氣自舌尖散開，頓時腹中一暖，整個人精神大振！")
    } else if is_drink {
        format!("你仰頭啜飲這份【{item_name}】。甘潤清甜的湯水順喉而下，齒頰留香，周身燥熱頓消，氣息隨之舒暢通達。")
    } else if is_herb {
        format!("你細細研服這份【{item_name}】。草木清香在口中化開，微苦回甘，藥力順著經脈緩緩化開，氣血漸漸充盈。")
    } else {
        format!("你取出手中的【{item_name}】仔細檢視並加以使用。觸感扎實細膩，工藝純熟，片刻調理後整個人氣定神閒。")
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
    let mut after_quote = initial_quote.clone();
    for _ in 0..sell_qty {
        let cur = crate::game::object_ecology::dynamic_settlement_market_quote(
            &player_room,
            &msg.item_id,
            base_unit_price,
        );
        payout = payout.saturating_add(cur.unit_price);
        after_quote = crate::game::object_ecology::record_settlement_market_trade(
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
        "你將 {sell_qty} 份【{item_name}】交予本地市集商鋪驗收（{}，實得 {payout} 鎂；成交後本地市集庫存變為 {}/{}，下一件收購行情調整為 {} 鎂）。",
        initial_quote.market_narrative,
        after_quote.local_stock,
        after_quote.target_stock,
        after_quote.unit_price,
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
                narrative.push_str(&format!(
                    " 你順著平日熟稔的《{}》手法落手收勢，器物肌理更顯勻稱。",
                    forge_sk.name
                ));
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
    tracing::info!(target: "topology_debug", "N000（生之奇點）→ 前三條主星軌阻力：");
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

fn load_persisted_wordplates() -> PersistedWordPlateState {
    let path = player_wordplates_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(state) = serde_json::from_str::<PersistedWordPlateState>(&data) {
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
        caskets: PLAYER_CASKETS.read().clone(),
        loadouts: PLAYER_LOADOUTS.read().clone(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let tmp_path = path.with_extension("tmp");
        if std::fs::write(&tmp_path, json).is_ok() {
            let _ = std::fs::rename(tmp_path, path);
        }
    }
}

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
    let mut loadout = TacticalLoadout::new();

    let get_id = |ch: &str| -> String {
        resolve_word_id(registry, ch).unwrap_or_else(|| "wu".to_string())
    };

    // 起手式「疾刺斬」
    let id_ji = get_id("疾");
    let id_ci = get_id("刺");
    let id_zhan = get_id("斬");
    if let Ok(sentence) = StarSentence::new(vec![
        StarInlay::new("N001", id_ji),
        StarInlay::new("N002", id_ci),
        StarInlay::new("N003", id_zhan.clone()),
    ]) {
        loadout.set_slot(TacticalSlot::Opening, sentence.with_custom_name("疾刺斬"));
    }

    // 常態連攜「流破斬」
    let id_liu = get_id("流");
    let id_po = get_id("破");
    if let Ok(sentence) = StarSentence::new(vec![
        StarInlay::new("N004", id_liu.clone()),
        StarInlay::new("N005", id_po.clone()),
        StarInlay::new("N006", id_zhan),
    ]) {
        loadout.set_slot(TacticalSlot::Midgame, sentence.with_custom_name("流破斬"));
    }

    // 破綻殺招「吾震破」
    let id_wu = get_id("吾");
    let id_zhen = get_id("震");
    if let Ok(sentence) = StarSentence::new(vec![
        StarInlay::new("N007", id_wu),
        StarInlay::new("N008", id_zhen),
        StarInlay::new("N009", id_po),
    ]) {
        loadout.set_slot(TacticalSlot::Finisher, sentence.with_custom_name("吾震破"));
    }

    // 守勢身法「凝流步」
    let id_ning = get_id("凝");
    let id_bu = get_id("步");
    if let Ok(sentence) = StarSentence::new(vec![
        StarInlay::new("N010", id_ning),
        StarInlay::new("N011", id_liu),
        StarInlay::new("N012", id_bu),
    ]) {
        loadout.set_slot(TacticalSlot::Defense, sentence.with_custom_name("凝流步"));
    }

    loadout
}

pub fn ensure_player_initialized(pid: &str) {
    let registry = WordRegistry::global();
    let mut mutated = false;
    let casket_opt = {
        let mut caskets = PLAYER_CASKETS.write();
        if !caskets.contains_key(pid) {
            caskets.insert(pid.to_string(), create_default_casket(registry));
            mutated = true;
        }
        caskets.get(pid).cloned()
    };
    {
        let mut loadouts = PLAYER_LOADOUTS.write();
        if !loadouts.contains_key(pid) {
            let soul_seed = db::get_entity(pid).ok().flatten().and_then(|e| e.soul_seed);
            let loadout = if let Some(seed) = soul_seed {
                crate::combat::wordplate::build_loadout_from_soul_seed(seed, casket_opt.as_ref(), registry)
            } else {
                create_default_loadout(registry)
            };
            loadouts.insert(pid.to_string(), loadout);
            mutated = true;
        }
    }
    if mutated {
        save_player_wordplates();
    }
    if let Ok(Some(ent)) = db::get_entity(pid) {
        if !inventory_has_item(&ent.inventory, "blank_manual")
            && !inventory_has_item(&ent.inventory, "wt_blank_manual")
            && STARTER_MANUAL_GRANTED.write().insert(pid.to_string())
        {
            let grant_qty = match ent.kind {
                crate::entity::EntityKind::Player => 2,
                crate::entity::EntityKind::Npc => 1,
            };
            let _ = db::add_to_inventory(pid, "blank_manual", grant_qty);
        }
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
pub fn set_player_word_casket(pid: &str, casket: WordCasket) {
    ensure_player_initialized(pid);
    let mut guard = PLAYER_CASKETS.write();
    guard.insert(pid.to_string(), casket);
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

const HUB_NAMES_ZH: [&str; 20] = [
    "天極", "脈衝", "震淵", "游離", "弦絲", "曜核", "凜晶", "淵流", "萬象", "解離",
    "鎮閾", "衡定", "穹壁", "重塑", "逆熵", "神淵", "識閾", "坍縮", "無相", "越權",
];

fn hub_label_from_node(node_id: &str) -> String {
    if let Some(num_str) = node_id.strip_prefix('N') {
        if let Ok(idx) = num_str.parse::<usize>() {
            if (1..=20).contains(&idx) {
                return format!("{}主樞", HUB_NAMES_ZH[idx - 1]);
            }
        }
    }
    "天極主樞".to_string()
}

/// 後台根據人物所學（字譜與星軌）、所熟悉（手持器物、銘文與日常營生）、以及自身屬性（生命種子與體氣敏三維）自然生成技能列表。
/// 這些技能並非玩家手動自訂，而是在戰鬥交鋒與百工生產中由後台自動施展。
pub fn synthesize_entity_natural_skills(ent: &Character) -> Vec<NaturalSkillView> {
    ensure_player_initialized(&ent.id);
    let registry = WordRegistry::global();
    let casket = get_player_word_casket(&ent.id);
    let loadout = {
        let guard = PLAYER_LOADOUTS.read();
        guard.get(&ent.id).cloned().unwrap_or_else(|| {
            let seed = ent.soul_seed.unwrap_or(100_003);
            crate::combat::wordplate::build_loadout_from_soul_seed(seed, Some(&casket), registry)
        })
    };

    let (equip_slots, equip_names, _) = parse_equipment_triple(&ent.equipment_slots);
    let held_keys = ["hold_r", "hold_l", "weapon", "hand", "tool"];
    let mut held_item_id = String::new();
    let mut held_item_display = String::new();
    for k in &held_keys {
        if let Some(id) = equip_slots.get(*k).filter(|s| !s.is_empty()) {
            held_item_id = id.clone();
            let base = equip_names
                .get(*k)
                .cloned()
                .unwrap_or_else(|| resolve_inv_item_name(id));
            held_item_display = format_entity_item_with_inscription(&ent.id, id, &base);
            break;
        }
    }

    let held_carrier = if !held_item_id.is_empty() {
        crate::combat::wordplate::infer_item_carrier_affinity(&held_item_id, &held_item_display)
    } else {
        None
    };

    let role_hint = if !ent.display_title.trim().is_empty() {
        ent.display_title.trim().to_string()
    } else {
        "山海行旅者".to_string()
    };

    let raw_vocation = {
        let t = ent.display_title.trim();
        if t.is_empty() || t == "山海行旅者" {
            ""
        } else {
            t
        }
    };
    let seed = ent.soul_seed.unwrap_or(100_003);
    let habit = crate::pg::living_world::get_or_derive_breath_habit(&ent.id, raw_vocation, seed);
    let imprints = crate::pg::living_world::get_entity_lineage_imprints(&ent.id);

    let somatic_awareness = crate::combat::wordplate::somatic_meridian_awareness(seed);
    let mut breath_origin = format!("{} {}", habit.dominant.somatic_description(), somatic_awareness);
    if !imprints.is_empty() {
        let imprints_str = imprints
            .iter()
            .map(|imp| format!("{}·{}", imp.mentor_name, imp.imprint_title))
            .collect::<Vec<_>>()
            .join("、");
        breath_origin.push_str(&format!(" ｜ 師承烙印：{}", imprints_str));
    }

    let breath_skill = NaturalSkillView {
        domain: "martial".to_string(),
        category: "inner_breath".to_string(),
        category_label: "【內息底色】".to_string(),
        name: habit.dominant.label().to_string(),
        origin_basis: breath_origin,
        learned_and_familiar: "周天吐納".to_string(),
        auto_trigger_desc: habit.dominant.combat_rhythm_phrase().to_string(),
    };

    let mut skills = Vec::with_capacity(10);
    skills.push(breath_skill);
    let active_afflictions = crate::pg::living_world::get_entity_afflictions(&ent.id);

    // ─── 一、 臨陣武相：由四序位星軌＋所學字印＋手中器物自然生成的 4 項戰鬥技能 ───
    let slot_specs = [
        ("起手", loadout.opening.as_ref()),
        ("連攜", loadout.midgame.as_ref()),
        ("破綻", loadout.finisher.as_ref()),
        ("守勢", loadout.defense.as_ref()),
    ];

    for (cat_label, sent_opt) in slot_specs {
        if let Some(sent) = sent_opt {
            let words_vec: Vec<String> = sent
                .inlays
                .iter()
                .map(|inlay| {
                    registry
                        .get_by_id(&inlay.word_id)
                        .map(|e| e.ch.clone())
                        .unwrap_or_else(|| inlay.word_id.clone())
                })
                .collect();
            let words_badge = words_vec
                .iter()
                .map(|w| format!("【{w}】"))
                .collect::<Vec<_>>()
                .join("");
            let first_hub = sent
                .inlays
                .first()
                .map(|i| hub_label_from_node(&i.node_id))
                .unwrap_or_else(|| "天極主樞".to_string());

            let mut sent_mut = sent.clone();
            sent_mut.set_soul_seed(seed);
            let syn_opt = sent_mut.synthesize(registry).ok();
            let carrier = held_carrier
                .or_else(|| syn_opt.as_ref().map(|s| s.primary_carrier))
                .unwrap_or(CarrierAffinity::Unarmed);

            let natural_name = syn_opt
                .as_ref()
                .map(|s| s.effective_name.clone())
                .unwrap_or_else(|| format!("{}連式", words_vec.join("")));

            let vessel_name = if held_item_display.is_empty() {
                "徒手".to_string()
            } else {
                held_item_display.clone()
            };

            let vessel_str = if !held_item_display.is_empty() {
                format!("隨身器物：{}", held_item_display)
            } else {
                "徒手演練".to_string()
            };

            let motion_seq = syn_opt
                .as_ref()
                .map(|s| s.derive_elementary_motions_for_carrier(Some(carrier)))
                .unwrap_or_else(|| {
                    derive_elementary_motions_from_chars(&words_vec, carrier, TargetPart::Torso)
                });

            let packet = SkillDataPacket {
                entity_id: ent.id.clone(),
                role_title: role_hint.clone(),
                domain: "martial".to_string(),
                slot_label: cat_label.to_string(),
                hub_or_tier: first_hub.clone(),
                word_chars: words_vec.clone(),
                vessel_name,
                carrier,
                motion_sequence: motion_seq.clone(),
                fallback_name: natural_name,
            };

            let synthesized = synthesize_skill_from_data(&packet);

            let mut affliction_tags = Vec::new();
            for aff in &active_afflictions {
                match aff {
                    crate::pg::living_world::AfflictionKind::ArmSprain => {
                        if carrier != CarrierAffinity::Footwork {
                            affliction_tags.push("【帶傷：腕肘挫傷】");
                        }
                    }
                    crate::pg::living_world::AfflictionKind::LegSprain => {
                        if cat_label == "守勢" || carrier == CarrierAffinity::Footwork {
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
                format!(" ｜ {}", affliction_tags.join(" "))
            } else {
                String::new()
            };

            let mut origin_basis = synthesized.origin_basis;
            if let Some(syn) = syn_opt.as_ref() {
                if !syn.meridian_detour_note.is_empty() {
                    origin_basis.push_str(&format!(" {}", syn.meridian_detour_note));
                }
            }

            skills.push(NaturalSkillView {
                domain: "martial".to_string(),
                category: String::new(),
                category_label: format!("{} · {}", cat_label, first_hub),
                name: synthesized.name,
                origin_basis,
                learned_and_familiar: format!("字印：{} ｜ 碎招：{} ｜ {}{}", words_badge, motion_seq.join("→"), vessel_str, affliction_suffix),
                auto_trigger_desc: String::new(),
            });
        }
    }

    let mut craft_totals = loadout.total_craft_dims(registry);
    if !held_item_id.is_empty() {
        let bonus = casket.inscribed_craft_bonus(&held_item_id, registry);
        craft_totals.forge += bonus.forge;
        craft_totals.culinary += bonus.culinary;
        craft_totals.harvest += bonus.harvest;
        craft_totals.wayfare += bonus.wayfare;
    }

    let mastery_tier = |score: f64| -> &'static str {
        if score >= 45.0 {
            "爐火純青"
        } else if score >= 25.0 {
            "純熟"
        } else if score >= 12.0 {
            "熟稔"
        } else {
            "初窺"
        }
    };

    let harvest_score = craft_totals.harvest + ent.dex as f64 * 0.5;
    let harvest_name = if !held_item_display.is_empty()
        && (held_item_display.contains("刀")
            || held_item_display.contains("刃")
            || held_item_display.contains("剪"))
    {
        "庖丁解骨手".to_string()
    } else {
        "山林辨紋手".to_string()
    };
    let harvest_packet = SkillDataPacket {
        entity_id: ent.id.clone(),
        role_title: role_hint.clone(),
        domain: "production".to_string(),
        slot_label: "採獵".to_string(),
        hub_or_tier: mastery_tier(harvest_score).to_string(),
        word_chars: vec![],
        vessel_name: if held_item_display.is_empty() { "徒手".to_string() } else { held_item_display.clone() },
        carrier: held_carrier.unwrap_or(CarrierAffinity::Blade),
        motion_sequence: vec!["探縫尋節".into(), "貼骨順紋".into(), "剝離完軀".into()],
        fallback_name: harvest_name,
    };
    let harvest_syn = synthesize_skill_from_data(&harvest_packet);
    let mut harvest_afflictions = Vec::new();
    for aff in &active_afflictions {
        match aff {
            crate::pg::living_world::AfflictionKind::ArmSprain => harvest_afflictions.push("【帶傷：腕肘挫傷】"),
            crate::pg::living_world::AfflictionKind::HeadConcussion => harvest_afflictions.push("【帶傷：頭面震盪】"),
            crate::pg::living_world::AfflictionKind::Miasma => harvest_afflictions.push("【帶傷：瘴氣入體】"),
            _ => {}
        }
    }
    let harvest_aff_suffix = if !harvest_afflictions.is_empty() {
        format!(" ｜ {}", harvest_afflictions.join(" "))
    } else {
        String::new()
    };
    skills.push(NaturalSkillView {
        domain: "production".to_string(),
        category: String::new(),
        category_label: format!("採獵 · {}", mastery_tier(harvest_score)),
        name: harvest_syn.name,
        origin_basis: harvest_syn.origin_basis,
        learned_and_familiar: if held_item_display.is_empty() {
            format!("歷練：{}{}", role_hint, harvest_aff_suffix)
        } else {
            format!("慣用器物：{} ｜ 歷練：{}{}", held_item_display, role_hint, harvest_aff_suffix)
        },
        auto_trigger_desc: String::new(),
    });

    let forge_score = craft_totals.forge + ent.vit as f64 * 0.5;
    let forge_name = if !held_item_display.is_empty()
        && (held_item_display.contains("錘")
            || held_item_display.contains("鎬")
            || held_item_display.contains("斧")
            || held_item_display.contains("鉗"))
    {
        "震淵百鍛法".to_string()
    } else {
        "觀火成形訣".to_string()
    };
    let forge_packet = SkillDataPacket {
        entity_id: ent.id.clone(),
        role_title: role_hint.clone(),
        domain: "production".to_string(),
        slot_label: "百工".to_string(),
        hub_or_tier: mastery_tier(forge_score).to_string(),
        word_chars: vec![],
        vessel_name: if held_item_display.is_empty() { "徒手".to_string() } else { held_item_display.clone() },
        carrier: held_carrier.unwrap_or(CarrierAffinity::Blunt),
        motion_sequence: vec!["聽聲辨火".into(), "落錘順肌".into(), "收勢成形".into()],
        fallback_name: forge_name,
    };
    let forge_syn = synthesize_skill_from_data(&forge_packet);
    let mut forge_afflictions = Vec::new();
    for aff in &active_afflictions {
        match aff {
            crate::pg::living_world::AfflictionKind::ArmSprain => forge_afflictions.push("【帶傷：腕肘挫傷】"),
            crate::pg::living_world::AfflictionKind::TorsoInternal => forge_afflictions.push("【帶傷：胸脅內傷】"),
            _ => {}
        }
    }
    let forge_aff_suffix = if !forge_afflictions.is_empty() {
        format!(" ｜ {}", forge_afflictions.join(" "))
    } else {
        String::new()
    };
    skills.push(NaturalSkillView {
        domain: "production".to_string(),
        category: String::new(),
        category_label: format!("百工 · {}", mastery_tier(forge_score)),
        name: forge_syn.name,
        origin_basis: forge_syn.origin_basis,
        learned_and_familiar: if held_item_display.is_empty() {
            format!("徒手揉製與控火{}", forge_aff_suffix)
        } else {
            format!("慣用器物：{}{}", held_item_display, forge_aff_suffix)
        },
        auto_trigger_desc: String::new(),
    });

    let culinary_score = craft_totals.culinary + ent.qi as f64 * 0.5;
    let culinary_packet = SkillDataPacket {
        entity_id: ent.id.clone(),
        role_title: role_hint.clone(),
        domain: "production".to_string(),
        slot_label: "庖廚".to_string(),
        hub_or_tier: mastery_tier(culinary_score).to_string(),
        word_chars: vec![],
        vessel_name: if held_item_display.is_empty() { "徒手".to_string() } else { held_item_display.clone() },
        carrier: held_carrier.unwrap_or(CarrierAffinity::Blade),
        motion_sequence: vec!["順紋切配".into(), "文武控火".into(), "收汁凝香".into()],
        fallback_name: "文武合味手".to_string(),
    };
    let culinary_syn = synthesize_skill_from_data(&culinary_packet);
    let mut culinary_afflictions = Vec::new();
    for aff in &active_afflictions {
        match aff {
            crate::pg::living_world::AfflictionKind::ArmSprain => culinary_afflictions.push("【帶傷：腕肘挫傷】"),
            _ => {}
        }
    }
    let culinary_aff_suffix = if !culinary_afflictions.is_empty() {
        format!(" ｜ {}", culinary_afflictions.join(" "))
    } else {
        String::new()
    };
    skills.push(NaturalSkillView {
        domain: "production".to_string(),
        category: String::new(),
        category_label: format!("庖廚 · {}", mastery_tier(culinary_score)),
        name: culinary_syn.name,
        origin_basis: culinary_syn.origin_basis,
        learned_and_familiar: format!("歷練：{}{}", role_hint, culinary_aff_suffix),
        auto_trigger_desc: String::new(),
    });

    let wayfare_score = craft_totals.wayfare + (ent.vit + ent.dex) as f64 * 0.3;
    let wayfare_packet = SkillDataPacket {
        entity_id: ent.id.clone(),
        role_title: role_hint.clone(),
        domain: "production".to_string(),
        slot_label: "行旅".to_string(),
        hub_or_tier: mastery_tier(wayfare_score).to_string(),
        word_chars: vec![],
        vessel_name: if held_item_display.is_empty() { "徒手".to_string() } else { held_item_display.clone() },
        carrier: held_carrier.unwrap_or(CarrierAffinity::Footwork),
        motion_sequence: vec!["調勻吐納".into(), "借風踏浪".into(), "省力疾行".into()],
        fallback_name: "觀風踏浪步".to_string(),
    };
    let wayfare_syn = synthesize_skill_from_data(&wayfare_packet);
    let mut wayfare_afflictions = Vec::new();
    for aff in &active_afflictions {
        match aff {
            crate::pg::living_world::AfflictionKind::LegSprain => wayfare_afflictions.push("【帶傷：下盤扭傷】"),
            crate::pg::living_world::AfflictionKind::TorsoInternal => wayfare_afflictions.push("【帶傷：胸脅內傷】"),
            crate::pg::living_world::AfflictionKind::Miasma => wayfare_afflictions.push("【帶傷：瘴氣入體】"),
            _ => {}
        }
    }
    let wayfare_aff_suffix = if !wayfare_afflictions.is_empty() {
        format!(" ｜ {}", wayfare_afflictions.join(" "))
    } else {
        String::new()
    };
    skills.push(NaturalSkillView {
        domain: "production".to_string(),
        category: String::new(),
        category_label: format!("行旅 · {}", mastery_tier(wayfare_score)),
        name: wayfare_syn.name,
        origin_basis: wayfare_syn.origin_basis,
        learned_and_familiar: format!("歷練：{}{}", role_hint, wayfare_aff_suffix),
        auto_trigger_desc: String::new(),
    });
    skills
}

