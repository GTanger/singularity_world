//! WebSocket `do_action` 主幹：分派到同房實體（talk/look/combat/borrow/trade）
//! 或房間物件（look/use/move）的動作。實際處理見各子模組。

mod borrow;
pub(crate) mod combat;
mod look;
mod object;
mod talk;
pub(crate) mod trade;

use crate::config::Server;
use crate::db;
use crate::entity::{self, Character, EntityKind, Verb};
use crate::game;
use crate::gametext;

use super::handler::{WsConnection, player_id, push_refresh};
use super::protocol::{ActionResultMsg, ClientMsg};

/// 與目標是否在同一可互動位置（place_id 優先，舊資料才使用相容鏡像）。
fn entities_share_playable_cell(player: &Character, target: &Character, player_room: &str) -> bool {
    if let (Some(px), Some(py), Some(tx), Some(ty)) =
        (player.grid_x, player.grid_y, target.grid_x, target.grid_y)
        && (px - tx).abs() + (py - ty).abs() <= 1
    {
        return true;
    }
    if let (Some(px), Some(py)) = (player.grid_x, player.grid_y) {
        if let Some(state) = crate::store::LIVE_NPC_GRID_STATES.read().get(&target.id)
            && (px - state.current.0).abs() + (py - state.current.1).abs() <= 1
        {
            return true;
        }
        if let Some(store) = crate::store::get_store()
            && store
                .read()
                .entity_ids_at_grid(px, py)
                .iter()
                .any(|id| id == &target.id)
        {
            return true;
        }
    }
    let player_place = db::get_entity_place_id(&player.id);
    let target_place = db::get_entity_place_id(&target.id);
    if player_place.is_some() && target_place.is_some() && player_place == target_place {
        return true;
    }
    let Ok(tr) = db::get_entity_room(&target.id) else {
        return false;
    };
    !player_room.is_empty() && db::location_keys_equivalent(player_room, &tr)
}

/// 分派 `do_action`：先試同房實體，再試房間物件。
pub fn handle_do_action(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let target_id = msg.entity_id.clone();
    let mut action = msg.action.clone();
    if action == "Attack" {
        action = "Slay".into();
    }
    if action == "Work" {
        do_player_work(conn, &pid, &target_id);
        return;
    }
    if target_id.is_empty() || action.is_empty() {
        conn.send_error(gametext::client("missing_target_or_action"));
        return;
    }
    if target_id == pid {
        conn.send_error(gametext::client("cannot_self_action"));
        return;
    }
    let Ok(Some(player_ch)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("get_self_failed"));
        return;
    };
    let player_room = match db::get_entity_room(&pid) {
        Ok(room) if !room.is_empty() => room,
        _ => match grid_room_key(&player_ch) {
            Some(room) => room,
            None => {
                conn.send_error(gametext::client("no_current_room"));
                return;
            }
        },
    };

    if try_entity_branch(
        conn,
        msg,
        &pid,
        &player_ch,
        &player_room,
        &target_id,
        &action,
    ) {
        return;
    }
    object::do_object_branch(conn, &pid, &player_room, &target_id, &action);
}

fn do_player_work(conn: &WsConnection, pid: &str, target_id: &str) {
    let now = game::now_unix();
    let Some(pool) = crate::pg::pool() else {
        conn.send_error("世界工作主資料暫時不可用");
        return;
    };
    match crate::pg::world_work::perform_shift(&pool, pid, now, "player_work") {
        Ok(Some(result)) => {
            if result.work_id.is_none() {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Work".into(),
                    target_id: if target_id.is_empty() {
                        result.venue_id.clone()
                    } else {
                        target_id.to_string()
                    },
                    target_name: result.occupation_id.clone(),
                    narrative: "工坊目前暫時無法結算本班次工資，請稍後再試。".into(),
                    success: false,
                    actions: vec!["Look".into()],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            if let Some(store) = crate::store::get_store()
                && let Err(e) = store.write().mirror_economy_from_authority(
                    pid,
                    result.entity_magnesium,
                    &result.inventory,
                    &result.current_activity,
                )
            {
                tracing::warn!("player work cache mirror: {e}");
            }
            let narrative = if let Some(supply) = &result.supply {
                format!(
                    "你接過班牌投入「{}」，順著工序一氣呵成完成整輪勞作；盤點封裝好的 {} 份「{}」隨即由專人起運。管事核對名冊後當面點清發放了 {} 鎂工資，沉甸甸落入你的囊中。",
                    result.occupation_id,
                    supply.quantity,
                    supply.display_name,
                    result.wage_mg
                )
            } else {
                format!(
                    "你挽起衣袖投入「{}」，專注而連貫地完成了整整一班的勞作。隨著當值鐘聲敲響，管事核對工冊無誤，當面點清並如數發放了 {} 鎂工資，整輪差事順利結清。",
                    result.occupation_id,
                    result.wage_mg
                )
            };
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Work".into(),
                target_id: if target_id.is_empty() {
                    result.venue_id.clone()
                } else {
                    target_id.to_string()
                },
                target_name: result.occupation_id,
                narrative,
                success: true,
                actions: vec!["Work".into(), "Look".into()],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
        }
        Ok(None) => {
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Work".into(),
                target_id: target_id.to_string(),
                target_name: "工作差事".into(),
                narrative:
                    "此處暫無你可以參與的排班工序，或是本班次的定額勞作已經結清；請稍候下一班次或確認派工紀錄。"
                        .into(),
                success: false,
                actions: vec!["Look".into()],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
        }
        Err(e) => {
            tracing::warn!("player work perform_shift failed: {e}");
            conn.send_error(format!("執行工作時發生錯誤：{e}"));
        }
    }
}

fn try_entity_branch(
    conn: &WsConnection,
    msg: &ClientMsg,
    pid: &str,
    player: &Character,
    player_room: &str,
    target_id: &str,
    action: &str,
) -> bool {
    let is_entity_id_prefix = target_id.starts_with("ambient_")
        || target_id.starts_with("wt_npc_")
        || target_id.starts_with("wt_vendor_")
        || target_id.starts_with("wt_citizen_")
        || target_id.starts_with("npc_");
    let Ok(target_opt) = db::get_entity(target_id) else {
        return false;
    };
    let Some(target) = target_opt else {
        if is_entity_id_prefix {
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.to_string(),
                target_id: target_id.to_string(),
                target_name: "陌生人".into(),
                narrative: "你欲上前，對方卻已邁步離開此處，朝鄰街走遠了。".into(),
                success: false,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
            return true;
        }
        return false;
    };
    if !entities_share_playable_cell(player, &target, player_room) {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.to_string(),
            target_id: target_id.to_string(),
            target_name: "陌生人".into(),
            narrative: "你欲上前，對方卻已邁步離開此處，朝鄰街走遠了。".into(),
            success: false,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        push_refresh(conn, pid);
        return true;
    }
    crate::store::pause_npc_for_interaction(target_id, 25);
    if !entity_socket_ok(conn, pid, &target, target_id, player_room, action) {
        return true;
    }
    let now = game::now_unix();
    match action {
        "Look" => look::do_entity_look(conn, pid, now, target_id, &target),
        "Talk" => talk::do_entity_talk(conn, msg, pid, player_room, target_id, &target, now),
        "Subdue" | "Slay" => {
            combat::do_entity_combat(conn, pid, player_room, target_id, action, &target, now);
        }
        "Borrow" => borrow::do_entity_borrow(conn, pid, target_id, &target, now),
        "Trade" => {
            let _ = trade::do_entity_trade(conn, msg, pid, target_id, &target, now);
        }
        _ => conn.send_error(client_err_fmt("unknown_action_fmt", action)),
    }
    true
}

fn grid_room_key(ch: &Character) -> Option<String> {
    let (Some(x), Some(y)) = (ch.grid_x, ch.grid_y) else {
        return None;
    };
    Some(format!("grid:{x}:{y}"))
}

pub fn action_label_zh(action: &str) -> &str {
    match action {
        "Gather" | "Harvest" | "Take" => "採集",
        "Look" => "觀察",
        "Talk" => "交談",
        "Trade" => "交易",
        "Borrow" => "商借",
        "Subdue" => "制伏",
        "Slay" | "Attack" => "擊殺",
        "Use" => "使用",
        "Operate" => "操作",
        "Read" => "閱讀",
        "Rest" => "歇息",
        "Sleep" => "入眠",
        "Bath" | "Wash" => "沐浴",
        "Drink" => "飲用",
        "Taste" => "品嚐",
        "Smell" => "聞香",
        "WarmHands" => "烘手",
        "RoastFood" => "炙烤",
        "MakeFire" => "生火",
        "AddFuel" => "添柴",
        "SplitWood" => "劈柴",
        "RefinePlank" => "裁鋸木板",
        "BurnCharcoal" => "悶燒精炭",
        "CraftTorch" => "紮製火把",
        "RepairStructure" => "修繕加固",
        "CollectCharcoal" => "取炭",
        "Work" => "差事",
        "Move" => "移動",
        _ => action,
    }
}

pub(super) fn client_err_fmt(key: &str, arg: &str) -> String {
    let label = action_label_zh(arg);
    gametext::client(key).replacen("%s", label, 1)
}

fn entity_socket_ok(
    conn: &WsConnection,
    pid: &str,
    target: &Character,
    target_id: &str,
    player_room: &str,
    action: &str,
) -> bool {
    let room_name = resolve_action_room_name(pid, target);
    if (is_canonical_named_npc(target) || trade::target_has_shop_offer(target, &room_name))
        && matches!(action, "Look" | "Talk" | "Trade" | "Borrow" | "Subdue" | "Slay")
    {
        return true;
    }
    if target.kind == EntityKind::Npc {
        let sockets = db::get_sockets_for_npc(target_id, player_room);
        if !sockets.iter().any(|s| s == action) {
            conn.send_error(client_err_fmt("cannot_action_fmt", action));
            return false;
        }
        if !db::is_default_socket(action) {
            let in_venue = db::entity_in_venue_at_room(target_id, player_room).unwrap_or(false);
            if !in_venue {
                conn.send_error(client_err_fmt("target_not_at_workplace_fmt", action));
                return false;
            }
        }
        return true;
    }
    let Some(v) = verb_from_action(action) else {
        conn.send_error(client_err_fmt("unknown_action_fmt", action));
        return false;
    };
    if !entity::has_socket(&Character::sockets(), &v) {
        conn.send_error(client_err_fmt("cannot_action_fmt", action));
        return false;
    }
    true
}

pub(super) fn is_canonical_named_npc(target: &Character) -> bool {
    if target.id.starts_with("wt_npc_")
        || target.id.starts_with("wt_vendor_")
        || target.display_title.contains('｜')
        || target.display_title.contains('|')
    {
        return true;
    }
    const CANONICAL_NAMES: &[&str] = &[
        "陸秉衡", "陸知微", "岑守岳", "岑拓", "溫懷瑾", "溫晴", "韓鐵山", "許萬川",
        "許海音", "林正雄", "周萬生", "方石", "岑守淵", "羅烈", "陳萬福", "沈清河",
        "白泉生", "柯成器", "蘇錦心", "顧海平", "洪守明", "鄭烏石", "秦水生", "馬山行",
        "梁千木", "石萬鈞", "袁山茶", "高望雲", "田百稼", "牧長風", "方赤土", "雷千鈞",
        "葉青囊", "阮水生", "陸承禮", "陸長貴", "陸敬軒", "秦明誠", "林時中", "趙觀星",
        "周一刀", "岑雲野", "許阿甘", "馬補靴", "沈硯舟",
    ];
    CANONICAL_NAMES.iter().any(|name| {
        target.id.contains(name) || target.display_title.contains(name)
    })
}

pub(super) fn target_grid_room_name(target: &Character) -> String {
    if let (Some(x), Some(y)) = (target.grid_x, target.grid_y) {
        if let Some(name) = grid_room_name_at(x, y) {
            return name;
        }
    }
    String::new()
}

pub(super) fn resolve_action_room_name(player_id: &str, target: &Character) -> String {
    let t_room = target_grid_room_name(target);
    if !t_room.is_empty() {
        return t_room;
    }
    if let Ok(Some(player)) = db::get_entity(player_id) {
        if let (Some(px), Some(py)) = (player.grid_x, player.grid_y) {
            if let Some(name) = grid_room_name_at(px, py) {
                return name;
            }
        }
    }
    String::new()
}

fn grid_room_name_at(x: i32, y: i32) -> Option<String> {
    crate::server::grid_manager::get_runtime_square_grid().and_then(|g| {
        g.get(crate::grid::SquareCoord::new(x, y)).map(|c| {
            if c.name.trim().is_empty() {
                crate::grid::terrain_name_zh(c.terrain).to_string()
            } else {
                c.name.clone()
            }
        })
    })
}

pub(super) fn player_visible_target_name(player_id: &str, target: &Character) -> String {
    if target.kind == EntityKind::Player {
        return display_target_name(target);
    }
    if let Ok(Some(acq)) = db::get_acquaintance(player_id, &target.id) {
        if acq.named {
            return display_target_name(target);
        }
    }
    "陌生人".to_string()
}

fn verb_from_action(action: &str) -> Option<Verb> {
    match action {
        "Talk" => Some(Verb::Talk),
        "Borrow" => Some(Verb::Borrow),
        "Subdue" => Some(Verb::Subdue),
        "Slay" => Some(Verb::Slay),
        "Look" => Some(Verb::Look),
        "Trade" => Some(Verb::Trade),
        _ => None,
    }
}

pub(super) fn display_target_name(target: &Character) -> String {
    if target.kind == EntityKind::Npc {
        let gender_str = match target.gender {
            Some(crate::entity::Gender::F) => Some("F"),
            Some(crate::entity::Gender::M) => Some("M"),
            None => None,
        };
        return crate::store::resolve_npc_person_name(&target.id, &target.display_title, gender_str);
    }
    if !target.display_title.is_empty() {
        target.display_title.clone()
    } else {
        target.id.clone()
    }
}

pub(super) fn current_game_hour(cfg: &Server) -> i32 {
    if cfg.game_time_epoch_unix == 0 {
        return 12;
    }
    let (_, h, _, _) = game::game_time_now(
        game::now_unix(),
        cfg.game_time_epoch_unix,
        cfg.game_time_scale,
    );
    h
}
