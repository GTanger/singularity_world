//! `do_action` — 物件分支：房內非實體目標（RoomObject）的互動與 Move 穿梭。

use crate::db::{
    self, get_object_and_room, get_object_by_id_in_room, get_object_by_name_in_room,
    object_has_socket, object_response, set_entity_grid,
};
use crate::event::{self, types};
use crate::game;
use crate::gametext;
use crate::grid::parse_grid_room_id;
use crate::server::m3_029;
use crate::world::materials;

use super::super::broadcast::{refresh_room_views_for_room, send_room_view_to_session};
use super::super::handler::{WsConnection, push_refresh};
use super::super::protocol::{ActionResultMsg, MovedMsg};
use super::{client_err_fmt, current_game_hour};

const M3_020_MINT_QUEUE_STATE: &str = "m3_020_mint_queue|queued_after_waiting_line";
const M3_020_SHORT_WORK_STATE: &str = "m3_020_short_work|accepted_loading_shift";
const M3_020_PASS_CHECK_STATE: &str = "m3_020_pass_check|joined_gate_review";
const M3_020_COUNCIL_COMPLAINT_STATE: &str = "m3_020_council_complaint|registered_notice_name";
const M3_027_REED_ITEM_ID: &str = "mat_reed_bundle";
const M3_027_SACKCLOTH_ITEM_ID: &str = "mat_sackcloth_bolt";
const M3_027_ROPE_ITEM_ID: &str = "mat_hemp_rope_roll";
const M3_027_REED_MARKET_DEMAND_PER_ROUND: usize = 2;
const M3_027_SOUP_COST: i32 = 2;
const M3_027_CARGO_PAY: i32 = 3;
const M3_027_REED_SELL_PAY: i32 = 4;
const M3_027_SACKCLOTH_SELL_PAY: i32 = 5;
const M3_027_ROPE_SELL_PAY: i32 = 6;
const M3_027_CARGO_TALLY_PAY: i32 = 1;
const M3_027_REED_GATHERED_STATE: &str = "m3_027_material_flow|gathered_north_bank_reed";
const M3_027_SACKCLOTH_GATHERED_STATE: &str =
    "m3_027_material_flow|gathered_short_market_sackcloth";
const M3_027_ROPE_GATHERED_STATE: &str = "m3_027_material_flow|gathered_ferry_plank_rope";
const M3_027_BENCH_REST_STATE: &str = "m3_027_player_body|rested_on_wet_bench";
const M3_027_SOUP_WARM_STATE: &str = "m3_027_player_body|warmed_by_hot_soup";
const M3_027_FREE_BROTH_STATE: &str = "m3_027_player_body|waited_for_shared_broth";
const M3_027_CARGO_HELP_STATE: &str = "m3_027_ferry_credit|carried_cargo_to_plank";
const M3_027_ROPE_HELP_STATE: &str = "m3_027_ferry_credit|helped_boat_rope";
const M3_027_REED_SOLD_STATE: &str = "m3_027_short_market|sold_reed_bundle";
const M3_027_REED_DEMAND_FILLED_STATE: &str = "m3_027_short_market|reed_demand_filled";
const M3_027_SACKCLOTH_DEMAND_FILLED_STATE: &str = "m3_027_short_market|sackcloth_demand_filled";
const M3_027_ROPE_DEMAND_FILLED_STATE: &str = "m3_027_short_market|rope_demand_filled";
const M3_027_BOARD_FERRY_STATE: &str = "m3_027_ferry_access|boarded_from_plank";
const M3_027_LANDED_SOUTH_STATE: &str = "m3_027_ferry_access|stepped_ashore_south";
const M3_027_WAREHOUSE_TALLY_STATE: &str = "m3_027_cargo_loop|checked_cargo_at_warehouse";
const M3_027_AWNING_DRY_STATE: &str = "m3_027_player_body|dried_under_river_awning";
const M3_027_BRAZIER_WARM_STATE: &str = "m3_027_player_body|warmed_hands_at_brazier";
const M3_027_QUEUE_TALK_STATE: &str = "m3_027_player_social|talked_in_waiting_line";
const M3_027_QUEUE_FERRY_STATE: &str = "m3_027_ferry_access|queued_for_ferry";

fn resolve_room_object(room_id: &str, target_id: &str) -> Option<crate::model::RoomObject> {
    get_object_by_id_in_room(room_id, target_id)
        .or_else(|| get_object_by_name_in_room(room_id, target_id))
        .or_else(|| get_object_and_room(target_id).and_then(|(o, r)| (r == room_id).then_some(o)))
        .or_else(|| resolve_grid_scene_object(room_id, target_id))
}

fn resolve_grid_scene_object(room_id: &str, target_id: &str) -> Option<crate::model::RoomObject> {
    let target = target_id.trim();
    if target.is_empty() {
        return None;
    }
    let coord = parse_grid_room_id(room_id)?;
    let grid = crate::server::grid_manager::get_runtime_square_grid()?;
    let cell = grid.get(coord)?;
    game::player_layer_scene_objects_for_room(&cell.name)
        .into_iter()
        .find(|obj| obj.id == target || obj.name == target)
}

pub(super) fn do_object_branch(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    target_id: &str,
    action: &str,
) {
    let Some(obj) = resolve_room_object(player_room, target_id) else {
        conn.send_error(gametext::client("target_not_found"));
        return;
    };
    if !object_has_socket(&obj, action) {
        conn.send_error(client_err_fmt("cannot_action_fmt", action));
        return;
    }
    let now = game::now_unix();
    if try_m3_027_material_gather(conn, pid, player_room, &obj, action, now) {
        return;
    }
    let needs_scene_history = m3_020_scene_object_uses_history(&obj.id, action)
        || m3_027_scene_object_uses_history(&obj.id, action);
    let mut prior_scene_actions = if needs_scene_history {
        recent_scene_object_action_payloads(pid, now)
    } else {
        Vec::new()
    };
    let mut prior_scene_states = if needs_scene_history {
        recent_scene_object_state_payloads(pid, now)
    } else {
        Vec::new()
    };
    if needs_scene_history {
        m3_029::merge_shared_market_history(&mut prior_scene_actions, &mut prior_scene_states, now);
    }
    if let Some(denial) = m3_027_scene_object_action_denial(
        pid,
        &obj.id,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    ) {
        let (others, move_target_id) = object_followup_buttons(
            &obj,
            player_room,
            action,
            &prior_scene_actions,
            &prior_scene_states,
        );
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative: denial.to_string(),
            success: false,
            actions: others,
            move_target_id,
            open_trade: false,
            suggested_action: String::new(),
        });
        return;
    }
    let mut narrative = m3_027_scene_object_state_action_response(
        &obj.id,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    )
    .or_else(|| {
        m3_020_scene_object_state_action_response(
            &obj.id,
            action,
            &prior_scene_actions,
            &prior_scene_states,
        )
    })
    .map(str::to_string)
    .unwrap_or_else(|| object_response(&obj, action));
    if narrative.is_empty() {
        narrative = format!(
            "你對【{}】執行了「{action}」，但似乎沒有什麼特別的反應。",
            obj.name
        );
    }
    if action == "Look"
        && let Some(echo) = m3_027_scene_object_look_echo(
            player_room,
            &obj.id,
            now,
            &prior_scene_actions,
            &prior_scene_states,
        )
        .or_else(|| {
            m3_020_scene_object_look_echo(&obj.id, &prior_scene_actions, &prior_scene_states)
                .map(str::to_string)
        })
    {
        narrative.push('\n');
        narrative.push_str(&echo);
    }
    let _ = event::append(now, pid, types::OBSERVED, &obj.id);
    if !apply_object_move(conn, pid, player_room, &obj, action) {
        return;
    }
    if let Some(payload) = m3_020_scene_object_followup_event_payload(&obj.id, action)
        .or_else(|| m3_027_scene_object_followup_event_payload(&obj.id, action))
    {
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &payload);
    }
    let state_payload = m3_020_scene_object_state_event_payload(
        &obj.id,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    )
    .or_else(|| {
        m3_027_scene_object_state_event_payload(
            &obj.id,
            action,
            &prior_scene_actions,
            &prior_scene_states,
        )
    });
    let state_written = if let Some(payload) = state_payload {
        event::append(now, pid, types::SCENE_OBJECT_STATE, payload).is_ok()
    } else {
        false
    };
    if state_written && m3_027_scene_object_apply_side_effects(pid, &obj.id, action, state_payload)
    {
        push_refresh(conn, pid);
    }
    let (others, move_target_id) = object_followup_buttons(
        &obj,
        player_room,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    );
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: action.into(),
        target_id: obj.id.clone(),
        target_name: obj.name.clone(),
        narrative,
        success: true,
        actions: others,
        move_target_id,
        open_trade: false,
        suggested_action: String::new(),
    });
}

fn m3_020_scene_object_uses_history(object_id: &str, action: &str) -> bool {
    object_id.starts_with("m3_020_")
        && matches!(
            action,
            "Look" | "Queue" | "AcceptJob" | "JoinCheck" | "RegisterComplaint"
        )
}

fn m3_020_scene_object_followup_event_payload(object_id: &str, action: &str) -> Option<String> {
    match (object_id, action) {
        ("m3_020_stone_gate_pass_tags", "Read")
        | ("m3_020_flagstone_waiting_lines", "Use")
        | ("m3_020_mint_half_open_door", "Open")
        | ("m3_020_hire_short_work_boards", "Read")
        | ("m3_020_council_notice_strip", "Read") => Some(format!("{object_id}|{action}")),
        _ => None,
    }
}

fn m3_020_scene_object_state_event_payload(
    object_id: &str,
    action: &str,
    prior_payloads: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    match (object_id, action) {
        ("m3_020_mint_half_open_door", "Queue")
            if m3_020_waiting_line_has_been_used(prior_payloads)
                && !m3_020_mint_queue_state_active(prior_states) =>
        {
            Some(M3_020_MINT_QUEUE_STATE)
        }
        ("m3_020_hire_short_work_boards", "AcceptJob")
            if m3_020_short_work_board_has_been_read(prior_payloads)
                && !m3_020_short_work_state_active(prior_states) =>
        {
            Some(M3_020_SHORT_WORK_STATE)
        }
        ("m3_020_stone_gate_pass_tags", "JoinCheck")
            if m3_020_pass_tags_have_been_read(prior_payloads)
                && !m3_020_pass_check_state_active(prior_states) =>
        {
            Some(M3_020_PASS_CHECK_STATE)
        }
        ("m3_020_council_notice_strip", "RegisterComplaint")
            if m3_020_council_notice_has_been_read(prior_payloads)
                && !m3_020_council_complaint_state_active(prior_states) =>
        {
            Some(M3_020_COUNCIL_COMPLAINT_STATE)
        }
        _ => None,
    }
}

fn m3_020_scene_object_state_action_response(
    object_id: &str,
    action: &str,
    prior_payloads: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    match (object_id, action) {
        ("m3_020_mint_half_open_door", "Queue") if m3_020_mint_queue_state_active(prior_states) => {
            Some("你已在剛被接住的側線裡，兌牌人抬手要你留在門檻外，不讓後面的人重新擠上來。")
        }
        ("m3_020_mint_half_open_door", "Queue")
            if m3_020_waiting_line_has_been_used(prior_payloads) =>
        {
            Some(
                "你按剛分出的側線站到門檻外，兌牌人沒有放你進門，只把下一輪要核的牌號往石案邊推近；身後的人跟著把腳步收成一線。",
            )
        }
        ("m3_020_mint_half_open_door", "Queue") => {
            Some("廣場側線還沒分清，兌牌人抬手要你先退回人群後面，等旗手把去向喊明。")
        }
        ("m3_020_hire_short_work_boards", "AcceptJob")
            if m3_020_short_work_state_active(prior_states) =>
        {
            Some("棚頭人看見你已經應下活計，只把你壓回棚柱旁那條短線，等下一個小隊齊人。")
        }
        ("m3_020_hire_short_work_boards", "AcceptJob")
            if m3_020_short_work_board_has_been_read(prior_payloads) =>
        {
            Some(
                "你指向最外側那片短工牌，棚頭人把半日繩結往你手邊一拋，讓你站到商隊卸貨的小隊末尾。",
            )
        }
        ("m3_020_hire_short_work_boards", "AcceptJob") => {
            Some("棚頭人把短工牌往內側一翻，示意你先看清活計方向和時段，別擠在棚柱前亂應。")
        }
        ("m3_020_stone_gate_pass_tags", "JoinCheck")
            if m3_020_pass_check_state_active(prior_states) =>
        {
            Some("門衛看見你已經候檢，只把你壓回石案側邊那條短線，讓後面的人不要越過木牌。")
        }
        ("m3_020_stone_gate_pass_tags", "JoinCheck")
            if m3_020_pass_tags_have_been_read(prior_payloads) =>
        {
            Some(
                "你照著剛看清的牌序站到石案側邊，門衛把你的去向壓進短冊，讓商隊和工坊送件的人各退半步。",
            )
        }
        ("m3_020_stone_gate_pass_tags", "JoinCheck") => {
            Some("門衛把通行牌往石案內側一收，示意你先看清牌序，別直接擠進候檢線。")
        }
        ("m3_020_council_notice_strip", "RegisterComplaint")
            if m3_020_council_complaint_state_active(prior_states) =>
        {
            Some("記牌人看見你已經押名，只把短冊往階邊再壓一寸，讓你等棚內叫到再上前。")
        }
        ("m3_020_council_notice_strip", "RegisterComplaint")
            if m3_020_council_notice_has_been_read(prior_payloads) =>
        {
            Some(
                "你照告示把名字押在階側短冊上，記牌人用指節敲了敲通行牌申訴那一列，讓後面的人沿坡街邊等候。",
            )
        }
        ("m3_020_council_notice_strip", "RegisterComplaint") => {
            Some("記牌人按住告示邊角，要你先看清欠糧、傭工和通行牌申訴分列，再決定押到哪一行。")
        }
        _ => None,
    }
}

fn m3_020_scene_object_look_echo(
    object_id: &str,
    prior_payloads: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    m3_020_scene_object_state_echo(object_id, prior_states)
        .or_else(|| m3_020_scene_object_followup_echo(object_id, prior_payloads))
}

fn m3_020_scene_object_state_echo(
    object_id: &str,
    prior_states: &[String],
) -> Option<&'static str> {
    for state in prior_states.iter().rev() {
        let echo = match state.as_str() {
            M3_020_MINT_QUEUE_STATE => m3_020_mint_queue_state_echo(object_id),
            M3_020_SHORT_WORK_STATE => m3_020_short_work_state_echo(object_id),
            M3_020_PASS_CHECK_STATE => m3_020_pass_check_state_echo(object_id),
            M3_020_COUNCIL_COMPLAINT_STATE => m3_020_council_complaint_state_echo(object_id),
            _ => None,
        };
        if echo.is_some() {
            return echo;
        }
    }
    None
}

fn m3_020_mint_queue_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_020_mint_half_open_door" => Some(
            "你剛排進去的側線已被兌牌人接住，門檻外留下一條窄窄空位；他偶爾把下一張牌號往石案邊推近。",
        ),
        "m3_020_flagstone_waiting_lines" => {
            Some("鑄幣巷口那條側線被接住後，廣場中間少了一股橫穿的人潮，旗手的手勢比方才短。")
        }
        "m3_020_stone_gate_pass_tags" => {
            Some("門衛看見鑄幣巷口的側線已接住人，商隊那疊通行牌被往石案邊推近一點。")
        }
        _ => None,
    }
}

fn m3_020_short_work_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_020_hire_short_work_boards" => {
            Some("你剛應下的那片短工牌被棚頭人翻到繩結下，商隊卸貨的小隊在棚柱旁收成一列。")
        }
        "m3_020_flagstone_waiting_lines" => {
            Some("傭工棚邊收走一個半日活計後，旗石廣場通往棚口的那股人流短短薄了一層。")
        }
        _ => None,
    }
}

fn m3_020_pass_check_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_020_stone_gate_pass_tags" => {
            Some("你剛候檢的那道牌序被門衛壓在石案側邊，商隊和工坊送件的人各讓出半步。")
        }
        "m3_020_flagstone_waiting_lines" => {
            Some("石壘門那邊的候檢線被收進石案側邊後，旗石廣場入口少了一段反覆回頭的人聲。")
        }
        _ => None,
    }
}

fn m3_020_council_complaint_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_020_council_notice_strip" => {
            Some("你剛押名的短冊被記牌人壓在告示下方，通行牌申訴那一列多了一道折角。")
        }
        "m3_020_flagstone_waiting_lines" => {
            Some("議棚前階收走一列押名的人後，旗石廣場往坡街那側少了幾次逆向推擠。")
        }
        _ => None,
    }
}

fn m3_020_scene_object_followup_echo(
    object_id: &str,
    prior_payloads: &[String],
) -> Option<&'static str> {
    for payload in prior_payloads.iter().rev() {
        let Some((prior_object, prior_action)) = payload.split_once('|') else {
            continue;
        };
        if prior_object == object_id
            && let Some(echo) = m3_020_same_object_echo(object_id, prior_action)
        {
            return Some(echo);
        }
        if let Some(echo) = m3_020_cross_object_echo(object_id, prior_object, prior_action) {
            return Some(echo);
        }
    }
    None
}

fn m3_020_same_object_echo(object_id: &str, prior_action: &str) -> Option<&'static str> {
    match (object_id, prior_action) {
        ("m3_020_stone_gate_pass_tags", "Read") => {
            Some("剛讀過的幾張通行牌仍壓在石案邊，你再看時能更快分清商隊和工坊送件的隊列。")
        }
        ("m3_020_flagstone_waiting_lines", "Use") => {
            Some("你剛讓出的那一步還留在隊列邊，旗手順手把後面的人壓到同一側。")
        }
        ("m3_020_mint_half_open_door", "Open") => Some("門縫比方才更窄，兌牌人的手還壓在門板上。"),
        ("m3_020_hire_short_work_boards", "Read") => {
            Some("最急的那片短工牌仍翻在外側，棚頭人正按那幾個活計點人。")
        }
        ("m3_020_council_notice_strip", "Read") => {
            Some("你剛看過的幾行告示還在階邊，記牌人按著短冊不讓申訴人擠到坡街中間。")
        }
        _ => None,
    }
}

fn m3_020_cross_object_echo(
    object_id: &str,
    prior_object: &str,
    prior_action: &str,
) -> Option<&'static str> {
    match (object_id, prior_object, prior_action) {
        ("m3_020_mint_half_open_door", "m3_020_flagstone_waiting_lines", "Use") => {
            Some("廣場那邊剛理出的側線延伸到巷口，等換票的人比先前收得更窄。")
        }
        ("m3_020_hire_short_work_boards", "m3_020_flagstone_waiting_lines", "Use") => {
            Some("旗石廣場的隊列分清後，等工的人也被棚頭人壓到棚柱旁。")
        }
        _ => None,
    }
}

fn m3_020_waiting_line_has_been_used(prior_payloads: &[String]) -> bool {
    prior_payloads
        .iter()
        .any(|payload| payload == "m3_020_flagstone_waiting_lines|Use")
}

fn m3_020_mint_queue_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_020_MINT_QUEUE_STATE)
}

fn m3_020_short_work_board_has_been_read(prior_payloads: &[String]) -> bool {
    prior_payloads
        .iter()
        .any(|payload| payload == "m3_020_hire_short_work_boards|Read")
}

fn m3_020_short_work_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_020_SHORT_WORK_STATE)
}

fn m3_020_pass_tags_have_been_read(prior_payloads: &[String]) -> bool {
    prior_payloads
        .iter()
        .any(|payload| payload == "m3_020_stone_gate_pass_tags|Read")
}

fn m3_020_pass_check_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_020_PASS_CHECK_STATE)
}

fn m3_020_council_notice_has_been_read(prior_payloads: &[String]) -> bool {
    prior_payloads
        .iter()
        .any(|payload| payload == "m3_020_council_notice_strip|Read")
}

fn m3_020_council_complaint_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_020_COUNCIL_COMPLAINT_STATE)
}

fn try_m3_027_material_gather(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now: i64,
) -> bool {
    if action != "Gather" {
        return false;
    }
    let Some(expected_item_id) = m3_027_material_source_item_for_object(&obj.id) else {
        return false;
    };
    let source_matches_object = materials::material_for_room_id(player_room)
        .is_some_and(|material| material.id == expected_item_id);
    if !source_matches_object {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative:
                "你伸手去理那堆料，卻只摸到不成束的濕碎邊；這裡暫時沒有能收進背包的可用材料。"
                    .to_string(),
            success: false,
            actions: Vec::new(),
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
        });
        return true;
    }
    let (narrative, success) =
        if let Some(gathered) = materials::try_gather_material_for_room(player_room, now) {
            let Some(state_payload) =
                m3_027_material_gather_state_for_object(&obj.id, &gathered.item_id)
            else {
                return false;
            };
            let _ = db::add_to_inventory(pid, &gathered.item_id, 1);
            let _ = event::append(
                now,
                pid,
                types::SCENE_OBJECT_ACTION,
                &format!("{}|Gather|{}", obj.id, gathered.item_id),
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, state_payload);
            push_refresh(conn, pid);
            (
                m3_027_material_gather_success_text(&obj.id, &gathered),
                true,
            )
        } else {
            (
                m3_027_material_gather_empty_text(&obj.id).to_string(),
                false,
            )
        };
    let (actions, move_target_id) = object_followup_buttons(obj, player_room, action, &[], &[]);
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: action.into(),
        target_id: obj.id.clone(),
        target_name: obj.name.clone(),
        narrative,
        success,
        actions,
        move_target_id,
        open_trade: false,
        suggested_action: String::new(),
    });
    true
}

fn m3_027_material_source_item_for_object(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_north_bank_reeds" => Some(M3_027_REED_ITEM_ID),
        "m3_027_market_sackcloth_pile" => Some(M3_027_SACKCLOTH_ITEM_ID),
        "m3_027_plank_wet_rope_coil" => Some(M3_027_ROPE_ITEM_ID),
        _ => None,
    }
}

fn m3_027_material_gather_state_for_object(object_id: &str, item_id: &str) -> Option<&'static str> {
    match (object_id, item_id) {
        ("m3_027_north_bank_reeds", M3_027_REED_ITEM_ID) => Some(M3_027_REED_GATHERED_STATE),
        ("m3_027_market_sackcloth_pile", M3_027_SACKCLOTH_ITEM_ID) => {
            Some(M3_027_SACKCLOTH_GATHERED_STATE)
        }
        ("m3_027_plank_wet_rope_coil", M3_027_ROPE_ITEM_ID) => Some(M3_027_ROPE_GATHERED_STATE),
        _ => None,
    }
}

fn m3_027_material_gather_success_text(
    object_id: &str,
    gathered: &materials::GatheredMaterial,
) -> String {
    match object_id {
        "m3_027_north_bank_reeds" => {
            let trace = if gathered.stock_remaining <= 0 {
                "這一側的蘆叢被割矮了，水線旁暫時只剩碎葉和濕莖。"
            } else {
                "你身後的蘆叢矮下去一小片，霧水順著新斷口往泥裡滴。"
            };
            format!(
                "你撥開貼岸的河霧，從濕蘆裡割下一束「{}」收進背包；{trace}",
                gathered.name
            )
        }
        "m3_027_market_sackcloth_pile" => {
            let trace = if gathered.stock_remaining <= 0 {
                "棚腳那堆舊麻袋被剪到只剩爛邊，搬貨人把空出的木箱往內側推。"
            } else {
                "舊麻袋堆薄了一層，旁邊搬貨人也照同一堆料挑能再包貨的布片。"
            };
            format!("你從短市棚腳剪下一卷「{}」收進背包；{trace}", gathered.name)
        }
        "m3_027_plank_wet_rope_coil" => {
            let trace = if gathered.stock_remaining <= 0 {
                "木樁內側只剩被水泡軟的斷繩，船戶把空出的繩釘重新壓緊。"
            } else {
                "濕繩圈少了一道，船戶把剩下能用的繩頭往樁後收。"
            };
            format!(
                "你從踏板木樁內側理出一卷「{}」收進背包；{trace}",
                gathered.name
            )
        }
        _ => format!(
            "你從【{}】理出一份可用材料收進背包。",
            gathered.origin_label
        ),
    }
}

fn m3_027_material_gather_empty_text(object_id: &str) -> &'static str {
    match object_id {
        "m3_027_north_bank_reeds" => {
            "你撥開蘆叢，只摸到被割過後倒伏的濕草；這一帶能束起來的蘆草暫時被人取走，只剩貼著水線的碎葉。"
        }
        "m3_027_market_sackcloth_pile" => {
            "你翻開棚腳舊麻袋，只剩沾泥爛邊和散線；能剪成包貨布的部分已被搬貨人和看貨人取走。"
        }
        "m3_027_plank_wet_rope_coil" => {
            "你摸到木樁內側的繩圈，只剩泡軟斷股；能重新捆箱的麻繩暫時被船戶和搬貨人取走。"
        }
        _ => "你伸手去理那堆料，卻只摸到不成束的濕碎邊。",
    }
}

fn m3_027_scene_object_uses_history(object_id: &str, action: &str) -> bool {
    object_id.starts_with("m3_027_")
        && matches!(
            action,
            "Look"
                | "Gather"
                | "Rest"
                | "BuySoup"
                | "QueueBroth"
                | "CarryCargo"
                | "HelpRope"
                | "DryOff"
                | "WarmHands"
                | "TalkQueue"
                | "QueueFerry"
                | "SellReeds"
                | "SellSackcloth"
                | "SellRope"
                | "BoardFerry"
                | "StepAshore"
                | "CheckCargo"
        )
}

fn m3_027_scene_object_action_denial(
    pid: &str,
    object_id: &str,
    action: &str,
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    match (object_id, action) {
        ("m3_027_hot_soup_stall", "BuySoup") if !m3_027_soup_taken_state_active(prior_states) => {
            let magnesium = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|ch| ch.magnesium)
                .unwrap_or(0);
            if magnesium < M3_027_SOUP_COST {
                Some(
                    "你摸到錢角時停了一下，手邊鎂數不夠買滿碗熱湯；攤主把勺子往桶側一偏，示意想省錢就去等分湯那條短線。",
                )
            } else {
                None
            }
        }
        ("m3_027_market_reed_buyer", "SellReeds")
            if m3_027_reed_market_demand_filled(prior_actions, prior_states) =>
        {
            Some("收牌人把價牌翻到背面，秤邊的貨籃已墊滿；這一輪短市不再收新的蘆草束。")
        }
        ("m3_027_market_reed_buyer", "SellReeds") => {
            let has_reeds = db::get_entity(pid)
                .ok()
                .flatten()
                .is_some_and(|ch| db::inventory_has_item(&ch.inventory, M3_027_REED_ITEM_ID));
            if has_reeds {
                None
            } else {
                Some("收牌人看了看你空著的手，沒有伸秤；短市這一輪只收能立刻墊箱、束貨的蘆草束。")
            }
        }
        ("m3_027_market_reed_buyer", "SellSackcloth")
            if m3_027_sackcloth_demand_filled_state_active(prior_states) =>
        {
            Some("看貨人把包貨單上麻袋布那行按下濕指印；這一輪要包的箱已裹好，不再收新的麻袋布。")
        }
        ("m3_027_market_reed_buyer", "SellSackcloth") => {
            let has_sackcloth = db::get_entity(pid)
                .ok()
                .flatten()
                .is_some_and(|ch| db::inventory_has_item(&ch.inventory, M3_027_SACKCLOTH_ITEM_ID));
            if has_sackcloth {
                None
            } else {
                Some("看貨人掀了掀包貨單，沒有看見能裹箱的麻袋布；空手站在秤邊，只會擋住搬貨人。")
            }
        }
        ("m3_027_market_reed_buyer", "SellRope")
            if m3_027_rope_demand_filled_state_active(prior_states) =>
        {
            Some("收牌人把麻繩那行劃過，踏板口那批箱包已經重新捆緊；這一輪不再收新的麻繩卷。")
        }
        ("m3_027_market_reed_buyer", "SellRope") => {
            let has_rope = db::get_entity(pid)
                .ok()
                .flatten()
                .is_some_and(|ch| db::inventory_has_item(&ch.inventory, M3_027_ROPE_ITEM_ID));
            if has_rope {
                None
            } else {
                Some("收牌人看了看你空著的手，沒有接繩頭；短市這一輪只收能立刻捆箱的麻繩卷。")
            }
        }
        _ => None,
    }
}

fn m3_027_scene_object_followup_event_payload(object_id: &str, action: &str) -> Option<String> {
    match (object_id, action) {
        ("m3_027_wet_bench", "Rest")
        | ("m3_027_hot_soup_stall", "BuySoup")
        | ("m3_027_hot_soup_stall", "QueueBroth")
        | ("m3_027_cargo_pole", "CarryCargo")
        | ("m3_027_boat_rope", "HelpRope")
        | ("m3_027_river_awning", "DryOff")
        | ("m3_027_charcoal_brazier", "WarmHands")
        | ("m3_027_waiting_line_chat", "TalkQueue")
        | ("m3_027_waiting_line_chat", "QueueFerry")
        | ("m3_027_market_reed_buyer", "SellReeds")
        | ("m3_027_market_reed_buyer", "SellSackcloth")
        | ("m3_027_market_reed_buyer", "SellRope")
        | ("m3_027_plank_boarding_marker", "BoardFerry")
        | ("m3_027_south_landing_rope", "StepAshore")
        | ("m3_027_warehouse_tally_board", "CheckCargo") => Some(format!("{object_id}|{action}")),
        _ => None,
    }
}

fn m3_027_scene_object_state_event_payload(
    object_id: &str,
    action: &str,
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    match (object_id, action) {
        ("m3_027_wet_bench", "Rest") if !m3_027_bench_rest_state_active(prior_states) => {
            Some(M3_027_BENCH_REST_STATE)
        }
        ("m3_027_hot_soup_stall", "BuySoup") if !m3_027_soup_taken_state_active(prior_states) => {
            Some(M3_027_SOUP_WARM_STATE)
        }
        ("m3_027_hot_soup_stall", "QueueBroth")
            if !m3_027_soup_taken_state_active(prior_states) =>
        {
            Some(M3_027_FREE_BROTH_STATE)
        }
        ("m3_027_cargo_pole", "CarryCargo") if !m3_027_cargo_help_state_active(prior_states) => {
            Some(M3_027_CARGO_HELP_STATE)
        }
        ("m3_027_boat_rope", "HelpRope")
            if m3_027_cargo_help_state_active(prior_states)
                && !m3_027_rope_help_state_active(prior_states) =>
        {
            Some(M3_027_ROPE_HELP_STATE)
        }
        ("m3_027_river_awning", "DryOff") if !m3_027_awning_dry_state_active(prior_states) => {
            Some(M3_027_AWNING_DRY_STATE)
        }
        ("m3_027_charcoal_brazier", "WarmHands")
            if !m3_027_brazier_warm_state_active(prior_states) =>
        {
            Some(M3_027_BRAZIER_WARM_STATE)
        }
        ("m3_027_waiting_line_chat", "TalkQueue")
            if !m3_027_queue_talk_state_active(prior_states) =>
        {
            Some(M3_027_QUEUE_TALK_STATE)
        }
        ("m3_027_waiting_line_chat", "QueueFerry")
            if !m3_027_queue_ferry_state_active(prior_states) =>
        {
            Some(M3_027_QUEUE_FERRY_STATE)
        }
        ("m3_027_market_reed_buyer", "SellReeds")
            if m3_027_reed_sales_count(prior_actions) + 1
                >= M3_027_REED_MARKET_DEMAND_PER_ROUND
                && !m3_027_reed_demand_filled_state_active(prior_states) =>
        {
            Some(M3_027_REED_DEMAND_FILLED_STATE)
        }
        ("m3_027_market_reed_buyer", "SellReeds")
            if !m3_027_reed_sold_state_active(prior_states) =>
        {
            Some(M3_027_REED_SOLD_STATE)
        }
        ("m3_027_market_reed_buyer", "SellSackcloth")
            if !m3_027_sackcloth_demand_filled_state_active(prior_states) =>
        {
            Some(M3_027_SACKCLOTH_DEMAND_FILLED_STATE)
        }
        ("m3_027_market_reed_buyer", "SellRope")
            if !m3_027_rope_demand_filled_state_active(prior_states) =>
        {
            Some(M3_027_ROPE_DEMAND_FILLED_STATE)
        }
        ("m3_027_plank_boarding_marker", "BoardFerry")
            if m3_027_queue_ferry_state_active(prior_states)
                && !m3_027_board_ferry_state_active(prior_states) =>
        {
            Some(M3_027_BOARD_FERRY_STATE)
        }
        ("m3_027_south_landing_rope", "StepAshore")
            if m3_027_board_ferry_state_active(prior_states)
                && !m3_027_landed_south_state_active(prior_states) =>
        {
            Some(M3_027_LANDED_SOUTH_STATE)
        }
        ("m3_027_warehouse_tally_board", "CheckCargo")
            if m3_027_cargo_help_state_active(prior_states)
                && !m3_027_warehouse_tally_state_active(prior_states) =>
        {
            Some(M3_027_WAREHOUSE_TALLY_STATE)
        }
        _ => None,
    }
}

fn m3_027_scene_object_apply_side_effects(
    pid: &str,
    object_id: &str,
    action: &str,
    state_payload: Option<&str>,
) -> bool {
    match (object_id, action, state_payload) {
        ("m3_027_hot_soup_stall", "BuySoup", Some(M3_027_SOUP_WARM_STATE)) => {
            let _ = db::add_magnesium(pid, -M3_027_SOUP_COST);
            true
        }
        ("m3_027_cargo_pole", "CarryCargo", Some(M3_027_CARGO_HELP_STATE)) => {
            let _ = db::add_magnesium(pid, M3_027_CARGO_PAY);
            true
        }
        ("m3_027_market_reed_buyer", "SellReeds", Some(state))
            if state == M3_027_REED_SOLD_STATE || state == M3_027_REED_DEMAND_FILLED_STATE =>
        {
            let _ = db::remove_from_inventory(pid, M3_027_REED_ITEM_ID, 1);
            let _ = db::add_magnesium(pid, M3_027_REED_SELL_PAY);
            true
        }
        (
            "m3_027_market_reed_buyer",
            "SellSackcloth",
            Some(M3_027_SACKCLOTH_DEMAND_FILLED_STATE),
        ) => {
            let _ = db::remove_from_inventory(pid, M3_027_SACKCLOTH_ITEM_ID, 1);
            let _ = db::add_magnesium(pid, M3_027_SACKCLOTH_SELL_PAY);
            true
        }
        ("m3_027_market_reed_buyer", "SellRope", Some(M3_027_ROPE_DEMAND_FILLED_STATE)) => {
            let _ = db::remove_from_inventory(pid, M3_027_ROPE_ITEM_ID, 1);
            let _ = db::add_magnesium(pid, M3_027_ROPE_SELL_PAY);
            true
        }
        ("m3_027_warehouse_tally_board", "CheckCargo", Some(M3_027_WAREHOUSE_TALLY_STATE)) => {
            let _ = db::add_magnesium(pid, M3_027_CARGO_TALLY_PAY);
            true
        }
        _ => false,
    }
}

fn m3_027_scene_object_state_action_response(
    object_id: &str,
    action: &str,
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    match (object_id, action) {
        ("m3_027_wet_bench", "Rest") if m3_027_bench_rest_state_active(prior_states) => {
            Some("剛讓出的乾位已被別人的包袱佔回去；你再靠近時，長凳只剩濕冷那一段。")
        }
        ("m3_027_hot_soup_stall", "BuySoup" | "QueueBroth")
            if m3_027_soup_taken_state_active(prior_states) =>
        {
            Some("攤主把勺子靠回鍋沿，說這一輪已經輪過你；鍋底的熱氣留給下一個被河風吹透的人。")
        }
        ("m3_027_cargo_pole", "CarryCargo") if m3_027_cargo_help_state_active(prior_states) => {
            Some("你剛扛過的那副貨擔已經停在踏板邊，短市的人把下一擔交給熟手，沒有再把肩位讓出來。")
        }
        ("m3_027_boat_rope", "HelpRope") if m3_027_rope_help_state_active(prior_states) => {
            Some("船戶看見你已經搭過一手，只讓你留在踏板旁的窄位等船靠穩；繩尾這回交給後面的人。")
        }
        ("m3_027_boat_rope", "HelpRope") if !m3_027_cargo_help_state_active(prior_states) => {
            Some("船戶把繩尾往掌心一收，先指向踏板邊的貨擔；船還沒靠穩，空手擠上去只會擋路。")
        }
        ("m3_027_river_awning", "DryOff") if m3_027_awning_dry_state_active(prior_states) => {
            Some("棚下剛被你晾過的位置已經換成別人的濕包袱；再擠進去，只會把衣角重新蹭濕。")
        }
        ("m3_027_charcoal_brazier", "WarmHands")
            if m3_027_brazier_warm_state_active(prior_states) =>
        {
            Some("炭火盆邊的人看見你剛烘過手，便把下一個冷得發抖的候渡者推近火口。")
        }
        ("m3_027_waiting_line_chat", "TalkQueue")
            if m3_027_queue_talk_state_active(prior_states) =>
        {
            Some("隊列裡能問的船期和湯價你已聽過一輪；再追著問，只會讓前後的人避開你的眼神。")
        }
        ("m3_027_waiting_line_chat", "QueueFerry")
            if m3_027_queue_ferry_state_active(prior_states) =>
        {
            Some("你已在隊列折角裡，前後的人都按這條線往踏板等；再擠一次，只會把剛認住的位置弄亂。")
        }
        ("m3_027_market_reed_buyer", "SellReeds")
            if m3_027_reed_demand_filled_state_active(prior_states) =>
        {
            Some("收牌人把價牌翻到背面，秤邊的貨籃已墊滿；這一輪短市不再收新的蘆草束。")
        }
        ("m3_027_market_reed_buyer", "SellReeds")
            if m3_027_reed_sales_count(prior_actions) + 1
                >= M3_027_REED_MARKET_DEMAND_PER_ROUND =>
        {
            Some(
                "你把蘆草交到秤邊，收牌人掂過濕重後數出幾枚鎂；這一束被抽去墊住最後一只貨籃，價牌隨即翻到背面。",
            )
        }
        ("m3_027_market_reed_buyer", "SellSackcloth")
            if m3_027_sackcloth_demand_filled_state_active(prior_states) =>
        {
            Some("看貨人把包貨單上麻袋布那行按下濕指印；這一輪要包的箱已裹好，不再收新的麻袋布。")
        }
        ("m3_027_market_reed_buyer", "SellSackcloth") => Some(
            "你把麻袋布交到包貨單旁，看貨人沿布邊摸過一圈後數出幾枚鎂；搬貨人立刻拿它裹住一只磨角木箱，麻袋布那行被按下濕指印。",
        ),
        ("m3_027_market_reed_buyer", "SellRope")
            if m3_027_rope_demand_filled_state_active(prior_states) =>
        {
            Some("收牌人把麻繩那行劃過，踏板口那批箱包已經重新捆緊；這一輪不再收新的麻繩卷。")
        }
        ("m3_027_market_reed_buyer", "SellRope") => Some(
            "你把麻繩卷放到濕木板上，收牌人拉開繩頭試了試韌度後數出幾枚鎂；踏板口那排箱包很快被重新捆緊，麻繩那行被劃掉。",
        ),
        ("m3_027_plank_boarding_marker", "BoardFerry")
            if m3_027_board_ferry_state_active(prior_states) =>
        {
            Some("船戶已把你放進踏板內側，再回頭擠木牌只會擋住下一個被點到的人。")
        }
        ("m3_027_plank_boarding_marker", "BoardFerry")
            if !m3_027_queue_ferry_state_active(prior_states) =>
        {
            Some("船戶把手掌壓在木牌上，示意你先回北岸隊列；沒被隊列認住的人不能直接擠上踏板。")
        }
        ("m3_027_south_landing_rope", "StepAshore")
            if m3_027_landed_south_state_active(prior_states) =>
        {
            Some("你已踩過那道濕泥繩位，後面的人沿同一條痕上坡；再退回去只會把上岸線堵住。")
        }
        ("m3_027_south_landing_rope", "StepAshore")
            if !m3_027_board_ferry_state_active(prior_states) =>
        {
            Some("上岸繩那頭還沒輪到你，南岸的人只讓剛從踏板內側下來的人先踩穩。")
        }
        ("m3_027_warehouse_tally_board", "CheckCargo")
            if m3_027_warehouse_tally_state_active(prior_states) =>
        {
            Some("記牌人已把那副貨擔劃進倉棚短冊，尾款也結過；牌邊只剩下一輪貨號。")
        }
        ("m3_027_warehouse_tally_board", "CheckCargo")
            if !m3_027_cargo_help_state_active(prior_states) =>
        {
            Some("記牌人翻了翻短冊，沒有找到你剛送到踏板邊的貨擔痕；沒清過貨的人領不到這行尾款。")
        }
        _ => None,
    }
}

fn m3_027_scene_object_look_echo(
    player_room: &str,
    object_id: &str,
    now: i64,
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<String> {
    if let Some(echo) =
        m3_027_material_object_look_echo(player_room, object_id, now, prior_actions, prior_states)
    {
        return Some(echo);
    }
    if let Some(echo) = m3_027_scene_object_priority_echo(object_id, prior_states) {
        return Some(echo);
    }
    for state in prior_states.iter().rev() {
        let echo = match state.as_str() {
            M3_027_REED_GATHERED_STATE => m3_027_reed_gathered_state_echo(object_id),
            M3_027_SACKCLOTH_GATHERED_STATE => m3_027_sackcloth_gathered_state_echo(object_id),
            M3_027_ROPE_GATHERED_STATE => m3_027_rope_gathered_state_echo(object_id),
            M3_027_BENCH_REST_STATE => m3_027_bench_rest_state_echo(object_id),
            M3_027_SOUP_WARM_STATE => m3_027_soup_warm_state_echo(object_id),
            M3_027_FREE_BROTH_STATE => m3_027_free_broth_state_echo(object_id),
            M3_027_CARGO_HELP_STATE => m3_027_cargo_help_state_echo(object_id),
            M3_027_ROPE_HELP_STATE => m3_027_rope_help_state_echo(object_id),
            M3_027_REED_SOLD_STATE => m3_027_reed_sold_state_echo(object_id),
            M3_027_REED_DEMAND_FILLED_STATE => m3_027_reed_demand_filled_state_echo(object_id),
            M3_027_SACKCLOTH_DEMAND_FILLED_STATE => {
                m3_027_sackcloth_demand_filled_state_echo(object_id)
            }
            M3_027_ROPE_DEMAND_FILLED_STATE => m3_027_rope_demand_filled_state_echo(object_id),
            M3_027_BOARD_FERRY_STATE => m3_027_board_ferry_state_echo(object_id),
            M3_027_LANDED_SOUTH_STATE => m3_027_landed_south_state_echo(object_id),
            M3_027_WAREHOUSE_TALLY_STATE => m3_027_warehouse_tally_state_echo(object_id),
            M3_027_AWNING_DRY_STATE => m3_027_awning_dry_state_echo(object_id),
            M3_027_BRAZIER_WARM_STATE => m3_027_brazier_warm_state_echo(object_id),
            M3_027_QUEUE_TALK_STATE => m3_027_queue_talk_state_echo(object_id),
            M3_027_QUEUE_FERRY_STATE => m3_027_queue_ferry_state_echo(object_id),
            _ => None,
        };
        if echo.is_some() {
            return echo.map(str::to_string);
        }
    }
    None
}

fn m3_027_material_object_look_echo(
    player_room: &str,
    object_id: &str,
    now: i64,
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<String> {
    match object_id {
        "m3_027_north_bank_reeds" => {
            let source = materials::material_source_stock_for_room(player_room, now)?;
            if source.stock <= 0 {
                Some(format!(
                    "{}被割到只剩貼水線的短葉，能束成「{}」的長莖要等下一輪水氣和人手重新補上。",
                    source.origin_label, source.material_name
                ))
            } else if m3_027_reed_gathered_state_active(prior_states)
                || source.stock * 3 <= source.stock_max
            {
                Some(format!(
                    "{}被割矮了一片，還看得到能束成「{}」的長莖；前面採過的人也只沿著同一條水線取草。",
                    source.origin_label, source.material_name
                ))
            } else {
                None
            }
        }
        "m3_027_market_sackcloth_pile" => {
            let source = materials::material_source_stock_for_room(player_room, now)?;
            if source.stock <= 0 {
                Some(format!(
                    "{}只剩沾泥爛邊，能剪成「{}」的布片暫時被搬貨人取走。",
                    source.origin_label, source.material_name
                ))
            } else if m3_027_sackcloth_gathered_state_active(prior_states)
                || source.stock * 3 <= source.stock_max
            {
                Some(format!(
                    "{}薄了一層，仍能翻出幾片「{}」；前面的搬貨人也只挑能再包貨的布。",
                    source.origin_label, source.material_name
                ))
            } else {
                None
            }
        }
        "m3_027_plank_wet_rope_coil" => {
            let source = materials::material_source_stock_for_room(player_room, now)?;
            if source.stock <= 0 {
                Some(format!(
                    "{}只剩泡軟斷股，能捲成「{}」的繩材暫時被船戶和搬貨人取走。",
                    source.origin_label, source.material_name
                ))
            } else if m3_027_rope_gathered_state_active(prior_states)
                || source.stock * 3 <= source.stock_max
            {
                Some(format!(
                    "{}少了一圈，仍看得到能捲成「{}」的繩材；船戶也照同一堆料補踏板邊的捆貨繩。",
                    source.origin_label, source.material_name
                ))
            } else {
                None
            }
        }
        "m3_027_market_reed_buyer" => m3_027_pack_order_look_echo(prior_actions, prior_states),
        _ => None,
    }
}

fn m3_027_pack_order_look_echo(
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<String> {
    let reed_filled = m3_027_reed_market_demand_filled(prior_actions, prior_states);
    let sackcloth_filled = m3_027_sackcloth_demand_filled_state_active(prior_states);
    let rope_filled = m3_027_rope_demand_filled_state_active(prior_states);
    let npc_delivered_reeds = m3_029::pack_runner_delivered_reeds(prior_states);
    if reed_filled && sackcloth_filled && rope_filled {
        return Some("短市包貨單三行都被按下濕指印：蘆草已墊箱，麻袋布已包貨，麻繩也把踏板口那批箱包重新捆緊；看貨人把秤邊讓給下一輪短貨。".to_string());
    }
    let mut filled = Vec::new();
    let mut waiting = Vec::new();
    if reed_filled {
        filled.push("蘆草墊箱");
    } else if m3_027_reed_sales_count(prior_actions) > 0
        || m3_027_reed_sold_state_active(prior_states)
    {
        if npc_delivered_reeds {
            filled.push("搬貨人先交的一束蘆草已墊箱");
        } else {
            filled.push("一部分蘆草已墊箱");
        }
        waiting.push("蘆草還未收滿");
    } else {
        waiting.push("蘆草墊箱");
    }
    if sackcloth_filled {
        filled.push("麻袋布包貨");
    } else {
        waiting.push("麻袋布包貨");
    }
    if rope_filled {
        filled.push("麻繩捆箱");
    } else {
        waiting.push("麻繩捆箱");
    }
    if filled.is_empty() {
        return None;
    }
    if npc_delivered_reeds {
        Some(format!(
            "短市包貨單上{}已被收進秤邊，{}仍壓在單子外側；搬貨人和玩家都照同一張單交料，誰先補上就會改掉這行需求。",
            filled.join("、"),
            waiting.join("、")
        ))
    } else {
        Some(format!(
            "短市包貨單上{}已被收進秤邊，{}仍壓在單子外側；看貨人和搬貨人都照這張單抽料。",
            filled.join("、"),
            waiting.join("、")
        ))
    }
}

fn m3_027_scene_object_priority_echo(object_id: &str, prior_states: &[String]) -> Option<String> {
    match object_id {
        "m3_027_plank_boarding_marker" => {
            if m3_027_board_ferry_state_active(prior_states) {
                m3_027_board_ferry_state_echo(object_id).map(str::to_string)
            } else if m3_027_queue_ferry_state_active(prior_states) {
                m3_027_queue_ferry_state_echo(object_id).map(str::to_string)
            } else if m3_027_cargo_help_state_active(prior_states) {
                m3_027_cargo_help_state_echo(object_id).map(str::to_string)
            } else {
                None
            }
        }
        "m3_027_south_landing_rope" => {
            if m3_027_landed_south_state_active(prior_states) {
                m3_027_landed_south_state_echo(object_id).map(str::to_string)
            } else if m3_027_board_ferry_state_active(prior_states) {
                m3_027_board_ferry_state_echo(object_id).map(str::to_string)
            } else {
                None
            }
        }
        "m3_027_warehouse_tally_board" => {
            if let Some(echo) = m3_027_pack_order_state_echo(object_id, prior_states) {
                Some(echo)
            } else if let Some(echo) = m3_029::shared_economy_echo(object_id, prior_states) {
                Some(echo)
            } else if m3_027_warehouse_tally_state_active(prior_states) {
                m3_027_warehouse_tally_state_echo(object_id).map(str::to_string)
            } else if m3_027_cargo_help_state_active(prior_states) {
                m3_027_cargo_help_state_echo(object_id).map(str::to_string)
            } else if m3_027_landed_south_state_active(prior_states) {
                m3_027_landed_south_state_echo(object_id).map(str::to_string)
            } else if m3_027_reed_demand_filled_state_active(prior_states) {
                m3_027_reed_demand_filled_state_echo(object_id).map(str::to_string)
            } else if m3_027_reed_sold_state_active(prior_states) {
                m3_027_reed_sold_state_echo(object_id).map(str::to_string)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn m3_027_pack_order_state_echo(object_id: &str, prior_states: &[String]) -> Option<String> {
    if !matches!(
        object_id,
        "m3_027_market_reed_buyer" | "m3_027_warehouse_tally_board"
    ) {
        return None;
    }
    let reed_filled = m3_027_reed_demand_filled_state_active(prior_states);
    let sackcloth_filled = m3_027_sackcloth_demand_filled_state_active(prior_states);
    let rope_filled = m3_027_rope_demand_filled_state_active(prior_states);
    if !(reed_filled || sackcloth_filled || rope_filled) {
        return None;
    }
    if object_id == "m3_027_warehouse_tally_board" {
        if reed_filled && sackcloth_filled && rope_filled {
            return Some("短市送來的箱包已在倉棚門邊分好：濕蘆草墊住箱角，麻袋布裹住磨邊，麻繩把同一批短貨重新捆成一排。".to_string());
        }
        let mut parts = Vec::new();
        if reed_filled {
            parts.push("蘆草墊箱");
        }
        if sackcloth_filled {
            parts.push("麻袋布包貨");
        }
        if rope_filled {
            parts.push("麻繩捆箱");
        }
        return Some(format!(
            "短市那邊已把{}送到倉棚門前，記牌人把這批短貨壓進同一行，不再像散貨那樣堆在泥邊。",
            parts.join("、")
        ));
    }
    if reed_filled && sackcloth_filled && rope_filled {
        Some("短市包貨單三行都被按下濕指印，秤邊只剩下一輪短貨要等。".to_string())
    } else {
        None
    }
}

fn m3_027_reed_gathered_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_north_bank_reeds" => {
            Some("你剛割過的蘆叢矮下一片，斷口旁還有水珠順著葉脈往泥裡落。")
        }
        "m3_027_market_reed_buyer" => {
            Some("收牌人看見你手上的濕蘆草，先把秤邊那塊木板空出來，等能墊箱的草束上秤。")
        }
        _ => None,
    }
}

fn m3_027_sackcloth_gathered_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_sackcloth_pile" => {
            Some("你剛剪過的舊麻袋堆薄了一層，棚腳露出被踩濕的爛邊和散線。")
        }
        "m3_027_market_reed_buyer" => {
            Some("看貨人看見你背包裡露出的麻袋布邊，先把包貨單那行壓到秤邊。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("倉棚前場有人指向短市棚腳，說那邊剛剪出的麻袋布能先裹磨角木箱。")
        }
        _ => None,
    }
}

fn m3_027_rope_gathered_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_plank_wet_rope_coil" => {
            Some("你剛理過的濕繩圈少了一道，木樁內側只剩船戶重新壓緊的繩頭。")
        }
        "m3_027_market_reed_buyer" => {
            Some("收牌人看見你手邊的麻繩卷，先把捆箱那行從包貨單下抽出來。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("倉棚門邊有人等踏板口補來麻繩，幾只箱包暫時只靠舊結頂著。")
        }
        _ => None,
    }
}

fn m3_027_bench_rest_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_wet_bench" => {
            Some("你剛坐過的乾位已被包袱佔回去，凳腳旁還留著一點被鞋底蹭開的濕泥。")
        }
        _ => None,
    }
}

fn m3_027_soup_warm_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_hot_soup_stall" => {
            Some("你的空碗還倒扣在攤邊，鍋裡熱氣薄了一層，攤主把收過的錢角壓進濕布底下。")
        }
        "m3_027_wet_bench" => Some("掌心的熱湯氣還沒散，你再看長凳時，河風不像方才那麼割手。"),
        "m3_027_boat_rope" => Some("湯碗留下的熱意還在指節上，你抓繩時不至於立刻被濕繩冰住。"),
        _ => None,
    }
}

fn m3_027_free_broth_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_hot_soup_stall" => {
            Some("你剛等過的桶側短線又接上一小截人，攤主把半碗淡湯留給沒錢買滿碗的候渡者。")
        }
        "m3_027_waiting_line_chat" => Some("隊列裡有人看見你拿過分湯，說話時把湯價壓低了一點。"),
        _ => None,
    }
}

fn m3_027_cargo_help_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_cargo_pole" => {
            Some("你剛扛走的一副貨擔讓泥地空出一道深痕，剩下的麻袋被短市的人往棚腳收。")
        }
        "m3_027_plank_boarding_marker" => {
            Some("你剛清出的貨擔痕停在踏板木牌旁，船戶不用再把上船線和搬貨線分開喊。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("踏板那頭清出的貨擔已被記牌人看見，短冊空出一行等人來對貨。")
        }
        "m3_027_boat_rope" => {
            Some("船戶看見你清過踏板邊的貨擔，手掌在繩上停了一下，像是在等你靠近搭手。")
        }
        _ => None,
    }
}

fn m3_027_rope_help_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_boat_rope" => {
            Some("你剛收過的繩圈被壓在木樁內側，船戶把那段窄位留著，等下一次船腹貼岸。")
        }
        "m3_027_cargo_pole" => {
            Some("貨擔旁的人看見你幫過船繩，說話時把踏板邊那條窄路給你留出半步。")
        }
        _ => None,
    }
}

fn m3_027_awning_dry_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_river_awning" => {
            Some("你剛站過的棚下位置被濕包袱填上，舊帆布邊還滴著慢慢變細的水線。")
        }
        "m3_027_wet_bench" => Some("衣角乾了一些，你再看長凳時，濕痕不再像方才那樣貼著膝蓋。"),
        _ => None,
    }
}

fn m3_027_brazier_warm_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_charcoal_brazier" => {
            Some("炭火盆旁你剛讓出的半掌空位被下一個人補上，煙氣貼著他的袖口往外散。")
        }
        "m3_027_boat_rope" => Some("指節被炭火烘過後，再碰濕繩時不至於立刻僵住。"),
        _ => None,
    }
}

fn m3_027_queue_talk_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_waiting_line_chat" => {
            Some("你剛問過的那幾個候渡者換成低聲閒談，船期和湯價已在隊列裡傳開。")
        }
        "m3_027_boat_rope" => {
            Some("隊列裡聽來的船期讓你看船戶手勢時更快分清哪一次靠岸能上前搭手。")
        }
        _ => None,
    }
}

fn m3_027_queue_ferry_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_waiting_line_chat" => {
            Some("你排進去的那段隊列被後面的人補緊，候渡者也按同一條木牌線往踏板口慢慢挪。")
        }
        "m3_027_boat_rope" => {
            Some("船戶看見你已排進隊列，手掌在繩上停了一下，只等下一次踏板前清出縫。")
        }
        "m3_027_hot_soup_stall" => {
            Some("你在隊列裡的位置被人認住後，湯攤那側的人不再把你擠回桶邊短線。")
        }
        "m3_027_plank_boarding_marker" => {
            Some("船戶已在木牌內側認住你的隊列位置，踏板口留出一道等你上前的窄縫。")
        }
        _ => None,
    }
}

fn m3_027_reed_sold_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_reed_buyer" => {
            Some("你交出的蘆草已被抽去墊在貨籃角落，收牌人把剛數過的鎂收進木牌下，價牌還沒有翻面。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("短市那邊新墊的蘆草跟著貨籃進到棚門旁，木箱角不再直接磨在泥地上。")
        }
        _ => None,
    }
}

fn m3_027_reed_demand_filled_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_reed_buyer" => {
            Some("價牌已被翻到背面，秤邊貨籃都墊上濕蘆草；收牌人把秤砣壓住，不再接新的草束。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("短市收滿的濕蘆草已墊進貨籃和木箱角，倉棚前場少了一點箱底磨泥的聲音。")
        }
        _ => None,
    }
}

fn m3_027_sackcloth_demand_filled_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_reed_buyer" => {
            Some("包貨單上麻袋布那行已按下濕指印，磨角木箱被粗布裹住，搬貨人不再催新的布卷。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("短市送來的麻袋布已裹住磨角木箱，倉棚門邊少了木角刮袋的聲音。")
        }
        "m3_027_market_sackcloth_pile" => {
            Some("包貨那批布已被收走，舊麻袋堆旁只剩等下一輪挑布的人。")
        }
        _ => None,
    }
}

fn m3_027_rope_demand_filled_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_reed_buyer" => {
            Some("包貨單上麻繩那行已被劃過，踏板口那批箱包重新捆緊，收牌人不再接新的繩卷。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("踏板口補來的麻繩把短貨重新捆成一排，倉棚門邊的箱包不再散開。")
        }
        "m3_027_plank_wet_rope_coil" => {
            Some("剛被收走的麻繩已用在踏板口那批箱包上，木樁內側只留給船戶的繩尾。")
        }
        _ => None,
    }
}

fn m3_027_board_ferry_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_plank_boarding_marker" => {
            Some("你剛被放進踏板內側，木牌前的人群收回半步，等下一個被船戶點到。")
        }
        "m3_027_south_landing_rope" => {
            Some("對岸繩位已有人替你讓出上岸腳點，泥坡邊的包袱先往內側挪開。")
        }
        _ => None,
    }
}

fn m3_027_landed_south_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_south_landing_rope" => {
            Some("你剛踩出的泥痕被後面的人沿著走，南岸上坡那條線沒有再散開。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("南岸剛上來的人流分出一股往倉棚前場，記牌人把短冊往棚門邊推近。")
        }
        _ => None,
    }
}

fn m3_027_warehouse_tally_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_warehouse_tally_board" => {
            Some("你對過的那行貨擔已被劃進短冊，牌角留下剛按過的濕指印。")
        }
        "m3_027_market_reed_buyer" => {
            Some("倉棚那邊收走一行貨後，短市價牌旁少了一次催貨聲，收牌人把秤砣壓得更穩。")
        }
        _ => None,
    }
}

fn m3_027_bench_rest_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_BENCH_REST_STATE)
}

fn m3_027_reed_gathered_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_REED_GATHERED_STATE)
}

fn m3_027_sackcloth_gathered_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_SACKCLOTH_GATHERED_STATE)
}

fn m3_027_rope_gathered_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_ROPE_GATHERED_STATE)
}

fn m3_027_soup_warm_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_SOUP_WARM_STATE)
}

fn m3_027_free_broth_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_FREE_BROTH_STATE)
}

fn m3_027_soup_taken_state_active(prior_states: &[String]) -> bool {
    m3_027_soup_warm_state_active(prior_states) || m3_027_free_broth_state_active(prior_states)
}

fn m3_027_cargo_help_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_CARGO_HELP_STATE)
}

fn m3_027_rope_help_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_ROPE_HELP_STATE)
}

fn m3_027_reed_sold_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_REED_SOLD_STATE)
}

fn m3_027_reed_demand_filled_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_REED_DEMAND_FILLED_STATE)
}

fn m3_027_sackcloth_demand_filled_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_SACKCLOTH_DEMAND_FILLED_STATE)
}

fn m3_027_rope_demand_filled_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_ROPE_DEMAND_FILLED_STATE)
}

fn m3_027_reed_sales_count(prior_actions: &[String]) -> usize {
    prior_actions
        .iter()
        .filter(|payload| payload.as_str() == "m3_027_market_reed_buyer|SellReeds")
        .count()
}

fn m3_027_reed_market_demand_filled(prior_actions: &[String], prior_states: &[String]) -> bool {
    m3_027_reed_demand_filled_state_active(prior_states)
        || m3_027_reed_sales_count(prior_actions) >= M3_027_REED_MARKET_DEMAND_PER_ROUND
}

fn m3_027_board_ferry_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_BOARD_FERRY_STATE)
}

fn m3_027_landed_south_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_LANDED_SOUTH_STATE)
}

fn m3_027_warehouse_tally_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_WAREHOUSE_TALLY_STATE)
}

fn m3_027_awning_dry_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_AWNING_DRY_STATE)
}

fn m3_027_brazier_warm_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_BRAZIER_WARM_STATE)
}

fn m3_027_queue_talk_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_QUEUE_TALK_STATE)
}

fn m3_027_queue_ferry_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_QUEUE_FERRY_STATE)
}

fn recent_scene_object_action_payloads(pid: &str, now: i64) -> Vec<String> {
    event::events_in_range(pid, now.saturating_sub(6 * 60 * 60), now)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.event_type == types::SCENE_OBJECT_ACTION)
        .map(|row| row.payload)
        .collect()
}

fn recent_scene_object_state_payloads(pid: &str, now: i64) -> Vec<String> {
    event::events_in_range(pid, now.saturating_sub(6 * 60 * 60), now)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.event_type == types::SCENE_OBJECT_STATE)
        .map(|row| row.payload)
        .collect()
}

fn object_followup_buttons(
    obj: &crate::model::RoomObject,
    player_room: &str,
    action: &str,
    prior_scene_actions: &[String],
    prior_scene_states: &[String],
) -> (Vec<String>, String) {
    let mut others: Vec<String> = obj
        .sockets
        .iter()
        .filter(|s| *s != action)
        .cloned()
        .collect();
    if action == "Look" {
        m3_020_filter_scene_object_buttons(
            &obj.id,
            &mut others,
            prior_scene_actions,
            prior_scene_states,
        );
        m3_027_filter_scene_object_buttons(&obj.id, &mut others, prior_scene_states);
    }
    let mut move_target_id = String::new();
    if action == "Look" && others.is_empty() && object_response(obj, "Look").is_empty() {
        let Ok(objs) = db::get_objects_in_room(player_room) else {
            return (others, move_target_id);
        };
        let idx = objs.iter().position(|o| o.id == obj.id);
        if let Some(i) = idx {
            for range in [(i + 1..objs.len()), (0..i)] {
                for j in range {
                    let o = &objs[j];
                    if !o.move_to_room_id.is_empty() && object_has_socket(o, "Move") {
                        move_target_id.clone_from(&o.id);
                        break;
                    }
                }
                if !move_target_id.is_empty() {
                    break;
                }
            }
        } else {
            for o in &objs {
                if !o.move_to_room_id.is_empty() && object_has_socket(o, "Move") {
                    move_target_id.clone_from(&o.id);
                    break;
                }
            }
        }
        if !move_target_id.is_empty() {
            others = vec!["Move".into()];
        }
    }
    (others, move_target_id)
}

fn m3_020_filter_scene_object_buttons(
    object_id: &str,
    actions: &mut Vec<String>,
    prior_scene_actions: &[String],
    prior_scene_states: &[String],
) {
    match object_id {
        "m3_020_mint_half_open_door" => {
            let can_queue = m3_020_waiting_line_has_been_used(prior_scene_actions)
                && !m3_020_mint_queue_state_active(prior_scene_states);
            if !can_queue {
                actions.retain(|action| action != "Queue");
            }
        }
        "m3_020_hire_short_work_boards" => {
            let can_accept_job = m3_020_short_work_board_has_been_read(prior_scene_actions)
                && !m3_020_short_work_state_active(prior_scene_states);
            if !can_accept_job {
                actions.retain(|action| action != "AcceptJob");
            }
        }
        "m3_020_stone_gate_pass_tags" => {
            let can_join_check = m3_020_pass_tags_have_been_read(prior_scene_actions)
                && !m3_020_pass_check_state_active(prior_scene_states);
            if !can_join_check {
                actions.retain(|action| action != "JoinCheck");
            }
        }
        "m3_020_council_notice_strip" => {
            let can_register_complaint = m3_020_council_notice_has_been_read(prior_scene_actions)
                && !m3_020_council_complaint_state_active(prior_scene_states);
            if !can_register_complaint {
                actions.retain(|action| action != "RegisterComplaint");
            }
        }
        _ => {}
    }
}

fn m3_027_filter_scene_object_buttons(
    object_id: &str,
    actions: &mut Vec<String>,
    prior_scene_states: &[String],
) {
    match object_id {
        "m3_027_wet_bench" if m3_027_bench_rest_state_active(prior_scene_states) => {
            actions.retain(|action| action != "Rest");
        }
        "m3_027_hot_soup_stall" if m3_027_soup_taken_state_active(prior_scene_states) => {
            actions.retain(|action| action != "BuySoup" && action != "QueueBroth");
        }
        "m3_027_cargo_pole" if m3_027_cargo_help_state_active(prior_scene_states) => {
            actions.retain(|action| action != "CarryCargo");
        }
        "m3_027_boat_rope" if m3_027_rope_help_state_active(prior_scene_states) => {
            actions.retain(|action| action != "HelpRope");
        }
        "m3_027_river_awning" if m3_027_awning_dry_state_active(prior_scene_states) => {
            actions.retain(|action| action != "DryOff");
        }
        "m3_027_charcoal_brazier" if m3_027_brazier_warm_state_active(prior_scene_states) => {
            actions.retain(|action| action != "WarmHands");
        }
        "m3_027_waiting_line_chat" => {
            if m3_027_queue_talk_state_active(prior_scene_states) {
                actions.retain(|action| action != "TalkQueue");
            }
            if m3_027_queue_ferry_state_active(prior_scene_states) {
                actions.retain(|action| action != "QueueFerry");
            }
        }
        "m3_027_market_reed_buyer" => {
            if m3_027_reed_demand_filled_state_active(prior_scene_states) {
                actions.retain(|action| action != "SellReeds");
            }
            if m3_027_sackcloth_demand_filled_state_active(prior_scene_states) {
                actions.retain(|action| action != "SellSackcloth");
            }
            if m3_027_rope_demand_filled_state_active(prior_scene_states) {
                actions.retain(|action| action != "SellRope");
            }
        }
        "m3_027_plank_boarding_marker" => {
            let can_board = m3_027_queue_ferry_state_active(prior_scene_states)
                && !m3_027_board_ferry_state_active(prior_scene_states);
            if !can_board {
                actions.retain(|action| action != "BoardFerry");
            }
        }
        "m3_027_south_landing_rope" => {
            let can_step_ashore = m3_027_board_ferry_state_active(prior_scene_states)
                && !m3_027_landed_south_state_active(prior_scene_states);
            if !can_step_ashore {
                actions.retain(|action| action != "StepAshore");
            }
        }
        "m3_027_warehouse_tally_board" => {
            let can_check_cargo = m3_027_cargo_help_state_active(prior_scene_states)
                && !m3_027_warehouse_tally_state_active(prior_scene_states);
            if !can_check_cargo {
                actions.retain(|action| action != "CheckCargo");
            }
        }
        _ => {}
    }
}

fn apply_object_move(
    conn: &WsConnection,
    pid: &str,
    _player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
) -> bool {
    if action != "Move" || obj.move_to_room_id.is_empty() {
        return true;
    }
    let Some(coord) = parse_grid_room_id(&obj.move_to_room_id) else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    let gh = current_game_hour(&conn.cfg);
    let Ok(view_opt) = game::get_grid_room_view(pid, coord.x, coord.y, gh) else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    let Some(view) = view_opt else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    if set_entity_grid(pid, coord.x, coord.y).is_err() {
        conn.send_error(gametext::client("move_failed"));
        return false;
    }
    let Some(session) = conn.sessions.get(pid) else {
        conn.send_error(gametext::client("move_failed"));
        return false;
    };
    send_room_view_to_session(&session, &view, pid, &conn.cfg);
    let rid = crate::grid::grid_room_id_from_coord(coord.x, coord.y);
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: pid.to_string(),
        room_id: rid.clone(),
        room_name: view.room.name.clone(),
    };
    if let Ok(bytes) = serde_json::to_vec(&moved) {
        conn.hub.broadcast(bytes);
    }
    refresh_room_views_for_room(&conn.sessions, &conn.cfg, &rid);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m3_020_followup_event_payloads_are_specific() {
        assert_eq!(
            m3_020_scene_object_followup_event_payload("m3_020_flagstone_waiting_lines", "Use"),
            Some("m3_020_flagstone_waiting_lines|Use".to_string())
        );
        assert_eq!(
            m3_020_scene_object_followup_event_payload("m3_020_flagstone_waiting_lines", "Look"),
            None
        );
        assert_eq!(
            m3_020_scene_object_followup_event_payload("ordinary_door", "Open"),
            None
        );
    }

    #[test]
    fn m3_020_queue_state_requires_waiting_line() {
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_mint_half_open_door",
                "Queue",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_mint_half_open_door",
                "Queue",
                &["m3_020_flagstone_waiting_lines|Use".to_string()],
                &[]
            ),
            Some(M3_020_MINT_QUEUE_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_mint_half_open_door",
                "Queue",
                &["m3_020_flagstone_waiting_lines|Use".to_string()],
                &[M3_020_MINT_QUEUE_STATE.to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_020_short_work_state_requires_board_read() {
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_hire_short_work_boards",
                "AcceptJob",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_hire_short_work_boards",
                "AcceptJob",
                &["m3_020_hire_short_work_boards|Read".to_string()],
                &[]
            ),
            Some(M3_020_SHORT_WORK_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_hire_short_work_boards",
                "AcceptJob",
                &["m3_020_hire_short_work_boards|Read".to_string()],
                &[M3_020_SHORT_WORK_STATE.to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_020_pass_and_council_states_require_prior_read() {
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_stone_gate_pass_tags",
                "JoinCheck",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_stone_gate_pass_tags",
                "JoinCheck",
                &["m3_020_stone_gate_pass_tags|Read".to_string()],
                &[]
            ),
            Some(M3_020_PASS_CHECK_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_council_notice_strip",
                "RegisterComplaint",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_council_notice_strip",
                "RegisterComplaint",
                &["m3_020_council_notice_strip|Read".to_string()],
                &[]
            ),
            Some(M3_020_COUNCIL_COMPLAINT_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_council_notice_strip",
                "RegisterComplaint",
                &["m3_020_council_notice_strip|Read".to_string()],
                &[M3_020_COUNCIL_COMPLAINT_STATE.to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_027_object_states_remain_concrete_scene_states() {
        assert_eq!(
            m3_027_scene_object_state_event_payload("m3_027_wet_bench", "Rest", &[], &[]),
            Some(M3_027_BENCH_REST_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_wet_bench",
                "Rest",
                &[],
                &[M3_027_BENCH_REST_STATE.to_string()]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_wet_bench",
                "Rest",
                &[],
                &[M3_027_BENCH_REST_STATE.to_string()]
            )
            .unwrap()
            .contains("包袱佔回去")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_hot_soup_stall",
                0,
                &[],
                &[M3_027_SOUP_WARM_STATE.to_string()]
            )
            .unwrap()
            .contains("空碗")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_hot_soup_stall",
                "BuySoup",
                &[],
                &[M3_027_FREE_BROTH_STATE.to_string()]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_hot_soup_stall",
                "QueueBroth",
                &[],
                &[M3_027_SOUP_WARM_STATE.to_string()]
            )
            .unwrap()
            .contains("輪過你")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload("m3_027_river_awning", "DryOff", &[], &[]),
            Some(M3_027_AWNING_DRY_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_charcoal_brazier",
                "WarmHands",
                &[],
                &[]
            ),
            Some(M3_027_BRAZIER_WARM_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_waiting_line_chat",
                "TalkQueue",
                &[],
                &[]
            ),
            Some(M3_027_QUEUE_TALK_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_waiting_line_chat",
                "QueueFerry",
                &[],
                &[]
            ),
            Some(M3_027_QUEUE_FERRY_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_waiting_line_chat",
                "QueueFerry",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_waiting_line_chat",
                "QueueFerry",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            )
            .unwrap()
            .contains("你已在隊列")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_waiting_line_chat",
                0,
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            )
            .unwrap()
            .contains("同一條木牌線")
        );
    }

    #[test]
    fn m3_027_rope_help_requires_prior_cargo_help() {
        assert_eq!(
            m3_027_scene_object_state_event_payload("m3_027_boat_rope", "HelpRope", &[], &[]),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response("m3_027_boat_rope", "HelpRope", &[], &[])
                .unwrap()
                .contains("貨擔")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_boat_rope",
                "HelpRope",
                &[],
                &[M3_027_CARGO_HELP_STATE.to_string()]
            ),
            Some(M3_027_ROPE_HELP_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_boat_rope",
                "HelpRope",
                &[],
                &[
                    M3_027_CARGO_HELP_STATE.to_string(),
                    M3_027_ROPE_HELP_STATE.to_string()
                ]
            ),
            None
        );
    }

    #[test]
    fn m3_027_cross_scene_loop_requires_prior_scene_states() {
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_plank_boarding_marker",
                "BoardFerry",
                &[],
                &[]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_plank_boarding_marker",
                "BoardFerry",
                &[],
                &[]
            )
            .unwrap()
            .contains("先回北岸隊列")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_plank_boarding_marker",
                "BoardFerry",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            ),
            Some(M3_027_BOARD_FERRY_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_south_landing_rope",
                "StepAshore",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            ),
            None
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_south_landing_rope",
                "StepAshore",
                &[],
                &[M3_027_BOARD_FERRY_STATE.to_string()]
            ),
            Some(M3_027_LANDED_SOUTH_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_warehouse_tally_board",
                "CheckCargo",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_warehouse_tally_board",
                "CheckCargo",
                &[],
                &[M3_027_CARGO_HELP_STATE.to_string()]
            ),
            Some(M3_027_WAREHOUSE_TALLY_STATE)
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_plank_boarding_marker",
                0,
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            )
            .unwrap()
            .contains("隊列位置")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_warehouse_tally_board",
                0,
                &[],
                &[M3_027_CARGO_HELP_STATE.to_string()]
            )
            .unwrap()
            .contains("短冊")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_warehouse_tally_board",
                0,
                &[],
                &[
                    M3_027_CARGO_HELP_STATE.to_string(),
                    M3_027_BOARD_FERRY_STATE.to_string(),
                    M3_027_LANDED_SOUTH_STATE.to_string(),
                ]
            )
            .unwrap()
            .contains("短冊空出一行")
        );
    }

    #[test]
    fn m3_027_reed_market_uses_recent_sales_as_round_demand() {
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &[],
                &[]
            ),
            Some(M3_027_REED_SOLD_STATE)
        );

        let one_sale = vec!["m3_027_market_reed_buyer|SellReeds".to_string()];
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &one_sale,
                &[M3_027_REED_SOLD_STATE.to_string()]
            ),
            Some(M3_027_REED_DEMAND_FILLED_STATE)
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &one_sale,
                &[M3_027_REED_SOLD_STATE.to_string()]
            )
            .unwrap()
            .contains("價牌")
        );
        assert!(m3_027_reed_market_demand_filled(
            &[
                "m3_027_market_reed_buyer|SellReeds".to_string(),
                "m3_027_market_reed_buyer|SellReeds".to_string(),
            ],
            &[]
        ));

        let buyer_obj = crate::model::RoomObject {
            id: "m3_027_market_reed_buyer".to_string(),
            name: "蘆草收牌".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "SellReeds".to_string()],
            responses: [("Look".to_string(), "價牌壓在秤邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (after_one_sale, _) = object_followup_buttons(
            &buyer_obj,
            "",
            "Look",
            &one_sale,
            &[M3_027_REED_SOLD_STATE.to_string()],
        );
        assert_eq!(after_one_sale, vec!["SellReeds".to_string()]);
        let (after_demand_filled, _) = object_followup_buttons(
            &buyer_obj,
            "",
            "Look",
            &one_sale,
            &[
                M3_027_REED_SOLD_STATE.to_string(),
                M3_027_REED_DEMAND_FILLED_STATE.to_string(),
            ],
        );
        assert!(after_demand_filled.is_empty());
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_market_reed_buyer",
                0,
                &one_sale,
                &[M3_027_REED_SOLD_STATE.to_string()]
            )
            .unwrap()
            .contains("蘆草還未收滿")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_market_reed_buyer",
                0,
                &one_sale,
                &[M3_027_REED_DEMAND_FILLED_STATE.to_string()]
            )
            .unwrap()
            .contains("蘆草墊箱")
        );
    }

    #[test]
    fn m3_027_pack_order_multi_material_demands_are_independent() {
        assert_eq!(
            m3_027_material_source_item_for_object("m3_027_market_sackcloth_pile"),
            Some(M3_027_SACKCLOTH_ITEM_ID)
        );
        assert_eq!(
            m3_027_material_source_item_for_object("m3_027_plank_wet_rope_coil"),
            Some(M3_027_ROPE_ITEM_ID)
        );
        assert_eq!(
            m3_027_material_gather_state_for_object(
                "m3_027_market_sackcloth_pile",
                M3_027_SACKCLOTH_ITEM_ID
            ),
            Some(M3_027_SACKCLOTH_GATHERED_STATE)
        );
        assert_eq!(
            m3_027_material_gather_state_for_object(
                "m3_027_plank_wet_rope_coil",
                M3_027_ROPE_ITEM_ID
            ),
            Some(M3_027_ROPE_GATHERED_STATE)
        );

        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellSackcloth",
                &[],
                &[]
            ),
            Some(M3_027_SACKCLOTH_DEMAND_FILLED_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellRope",
                &[],
                &[]
            ),
            Some(M3_027_ROPE_DEMAND_FILLED_STATE)
        );

        let pack_order = crate::model::RoomObject {
            id: "m3_027_market_reed_buyer".to_string(),
            name: "短市包貨單".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "SellReeds".to_string(),
                "SellSackcloth".to_string(),
                "SellRope".to_string(),
            ],
            responses: [("Look".to_string(), "包貨單壓在秤邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (after_sackcloth, _) = object_followup_buttons(
            &pack_order,
            "",
            "Look",
            &[],
            &[M3_027_SACKCLOTH_DEMAND_FILLED_STATE.to_string()],
        );
        assert_eq!(
            after_sackcloth,
            vec!["SellReeds".to_string(), "SellRope".to_string()]
        );
        let all_filled_states = vec![
            M3_027_REED_DEMAND_FILLED_STATE.to_string(),
            M3_027_SACKCLOTH_DEMAND_FILLED_STATE.to_string(),
            M3_027_ROPE_DEMAND_FILLED_STATE.to_string(),
        ];
        let (after_all_filled, _) =
            object_followup_buttons(&pack_order, "", "Look", &[], &all_filled_states);
        assert!(after_all_filled.is_empty());
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_market_reed_buyer",
                0,
                &[],
                &all_filled_states
            )
            .unwrap()
            .contains("三行")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_warehouse_tally_board",
                0,
                &[],
                &all_filled_states
            )
            .unwrap()
            .contains("麻繩")
        );
    }

    #[test]
    fn m3_029_pack_runner_delivery_counts_toward_shared_reed_demand() {
        let npc_reed_action = vec!["m3_027_market_reed_buyer|SellReeds".to_string()];
        let npc_reed_states = vec![
            "m3_029_pack_runner|reed_delivered_to_short_market".to_string(),
            M3_027_REED_SOLD_STATE.to_string(),
        ];
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &npc_reed_action,
                &npc_reed_states
            ),
            Some(M3_027_REED_DEMAND_FILLED_STATE)
        );
        let market_echo = m3_027_scene_object_look_echo(
            "",
            "m3_027_market_reed_buyer",
            0,
            &npc_reed_action,
            &npc_reed_states,
        )
        .unwrap();
        assert!(market_echo.contains("搬貨人"));
        assert!(market_echo.contains("同一張單"));

        let warehouse_echo = m3_027_scene_object_look_echo(
            "",
            "m3_027_warehouse_tally_board",
            0,
            &npc_reed_action,
            &npc_reed_states,
        )
        .unwrap();
        assert!(warehouse_echo.contains("短市"));
        assert!(warehouse_echo.contains("倉棚"));
    }

    #[test]
    fn m3_027_object_buttons_reflect_consumed_state() {
        let soup_obj = crate::model::RoomObject {
            id: "m3_027_hot_soup_stall".to_string(),
            name: "熱湯攤".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "BuySoup".to_string(),
                "QueueBroth".to_string(),
            ],
            responses: [("Look".to_string(), "湯桶冒著熱氣。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (available, _) = object_followup_buttons(&soup_obj, "", "Look", &[], &[]);
        assert_eq!(
            available,
            vec!["BuySoup".to_string(), "QueueBroth".to_string()]
        );
        let (consumed, _) = object_followup_buttons(
            &soup_obj,
            "",
            "Look",
            &[],
            &[M3_027_FREE_BROTH_STATE.to_string()],
        );
        assert!(consumed.is_empty());

        let awning_obj = crate::model::RoomObject {
            id: "m3_027_river_awning".to_string(),
            name: "避風棚".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "DryOff".to_string()],
            responses: [("Look".to_string(), "舊帆布壓著木桿。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (already_dried, _) = object_followup_buttons(
            &awning_obj,
            "",
            "Look",
            &[],
            &[M3_027_AWNING_DRY_STATE.to_string()],
        );
        assert!(already_dried.is_empty());

        let queue_obj = crate::model::RoomObject {
            id: "m3_027_waiting_line_chat".to_string(),
            name: "候渡隊列".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "TalkQueue".to_string(),
                "QueueFerry".to_string(),
            ],
            responses: [("Look".to_string(), "候渡隊列折在踏板前。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (queue_available, _) = object_followup_buttons(&queue_obj, "", "Look", &[], &[]);
        assert_eq!(
            queue_available,
            vec!["TalkQueue".to_string(), "QueueFerry".to_string()]
        );
        let (already_talked, _) = object_followup_buttons(
            &queue_obj,
            "",
            "Look",
            &[],
            &[M3_027_QUEUE_TALK_STATE.to_string()],
        );
        assert_eq!(already_talked, vec!["QueueFerry".to_string()]);
        let (already_queued, _) = object_followup_buttons(
            &queue_obj,
            "",
            "Look",
            &[],
            &[
                M3_027_QUEUE_TALK_STATE.to_string(),
                M3_027_QUEUE_FERRY_STATE.to_string(),
            ],
        );
        assert!(already_queued.is_empty());

        let plank_obj = crate::model::RoomObject {
            id: "m3_027_plank_boarding_marker".to_string(),
            name: "踏板木牌".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "BoardFerry".to_string()],
            responses: [("Look".to_string(), "踏板木牌立在水線邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_queue, _) = object_followup_buttons(&plank_obj, "", "Look", &[], &[]);
        assert!(without_queue.is_empty());
        let (with_queue, _) = object_followup_buttons(
            &plank_obj,
            "",
            "Look",
            &[],
            &[M3_027_QUEUE_FERRY_STATE.to_string()],
        );
        assert_eq!(with_queue, vec!["BoardFerry".to_string()]);

        let warehouse_obj = crate::model::RoomObject {
            id: "m3_027_warehouse_tally_board".to_string(),
            name: "倉棚點貨牌".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "CheckCargo".to_string()],
            responses: [("Look".to_string(), "點貨牌掛在倉門邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_cargo, _) = object_followup_buttons(&warehouse_obj, "", "Look", &[], &[]);
        assert!(without_cargo.is_empty());
        let (with_cargo, _) = object_followup_buttons(
            &warehouse_obj,
            "",
            "Look",
            &[],
            &[M3_027_CARGO_HELP_STATE.to_string()],
        );
        assert_eq!(with_cargo, vec!["CheckCargo".to_string()]);
        let (already_checked, _) = object_followup_buttons(
            &warehouse_obj,
            "",
            "Look",
            &[],
            &[
                M3_027_CARGO_HELP_STATE.to_string(),
                M3_027_WAREHOUSE_TALLY_STATE.to_string(),
            ],
        );
        assert!(already_checked.is_empty());
    }

    #[test]
    fn m3_020_queue_button_is_contextual() {
        let obj = crate::model::RoomObject {
            id: "m3_020_mint_half_open_door".to_string(),
            name: "半開鑄幣門".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Open".to_string(), "Queue".to_string()],
            responses: [("Look".to_string(), "半開的門縫。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };

        let (without_waiting, _) = object_followup_buttons(&obj, "", "Look", &[], &[]);
        assert_eq!(without_waiting, vec!["Open".to_string()]);

        let (with_waiting, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[],
        );
        assert_eq!(with_waiting, vec!["Open".to_string(), "Queue".to_string()]);

        let (already_queued, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[M3_020_MINT_QUEUE_STATE.to_string()],
        );
        assert_eq!(already_queued, vec!["Open".to_string()]);
    }

    #[test]
    fn m3_020_accept_job_button_is_contextual() {
        let obj = crate::model::RoomObject {
            id: "m3_020_hire_short_work_boards".to_string(),
            name: "短工牌".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "AcceptJob".to_string(),
            ],
            responses: [("Look".to_string(), "短工牌掛在棚柱外側。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };

        let (without_read, _) = object_followup_buttons(&obj, "", "Look", &[], &[]);
        assert_eq!(without_read, vec!["Read".to_string()]);

        let (with_read, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_hire_short_work_boards|Read".to_string()],
            &[],
        );
        assert_eq!(with_read, vec!["Read".to_string(), "AcceptJob".to_string()]);

        let (already_accepted, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_hire_short_work_boards|Read".to_string()],
            &[M3_020_SHORT_WORK_STATE.to_string()],
        );
        assert_eq!(already_accepted, vec!["Read".to_string()]);
    }

    #[test]
    fn m3_020_pass_and_council_buttons_are_contextual() {
        let pass_obj = crate::model::RoomObject {
            id: "m3_020_stone_gate_pass_tags".to_string(),
            name: "通行牌".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "JoinCheck".to_string(),
            ],
            responses: [("Look".to_string(), "通行牌壓在石案邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_pass_read, _) = object_followup_buttons(&pass_obj, "", "Look", &[], &[]);
        assert_eq!(without_pass_read, vec!["Read".to_string()]);
        let (with_pass_read, _) = object_followup_buttons(
            &pass_obj,
            "",
            "Look",
            &["m3_020_stone_gate_pass_tags|Read".to_string()],
            &[],
        );
        assert_eq!(
            with_pass_read,
            vec!["Read".to_string(), "JoinCheck".to_string()]
        );
        let (pass_already_checked, _) = object_followup_buttons(
            &pass_obj,
            "",
            "Look",
            &["m3_020_stone_gate_pass_tags|Read".to_string()],
            &[M3_020_PASS_CHECK_STATE.to_string()],
        );
        assert_eq!(pass_already_checked, vec!["Read".to_string()]);

        let notice_obj = crate::model::RoomObject {
            id: "m3_020_council_notice_strip".to_string(),
            name: "議棚告示".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "RegisterComplaint".to_string(),
            ],
            responses: [("Look".to_string(), "告示貼在階邊木板上。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_notice_read, _) = object_followup_buttons(&notice_obj, "", "Look", &[], &[]);
        assert_eq!(without_notice_read, vec!["Read".to_string()]);
        let (with_notice_read, _) = object_followup_buttons(
            &notice_obj,
            "",
            "Look",
            &["m3_020_council_notice_strip|Read".to_string()],
            &[],
        );
        assert_eq!(
            with_notice_read,
            vec!["Read".to_string(), "RegisterComplaint".to_string()]
        );
        let (notice_already_registered, _) = object_followup_buttons(
            &notice_obj,
            "",
            "Look",
            &["m3_020_council_notice_strip|Read".to_string()],
            &[M3_020_COUNCIL_COMPLAINT_STATE.to_string()],
        );
        assert_eq!(notice_already_registered, vec!["Read".to_string()]);
    }

    #[test]
    fn m3_020_queue_state_echo_takes_priority() {
        let echo = m3_020_scene_object_look_echo(
            "m3_020_mint_half_open_door",
            &["m3_020_mint_half_open_door|Open".to_string()],
            &[M3_020_MINT_QUEUE_STATE.to_string()],
        )
        .unwrap();
        assert!(echo.contains("側線已被兌牌人接住"));

        let flagstone_echo = m3_020_scene_object_look_echo(
            "m3_020_flagstone_waiting_lines",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[
                M3_020_MINT_QUEUE_STATE.to_string(),
                M3_020_SHORT_WORK_STATE.to_string(),
            ],
        )
        .unwrap();
        assert!(flagstone_echo.contains("通往棚口"));

        let mint_echo_after_short_work = m3_020_scene_object_look_echo(
            "m3_020_mint_half_open_door",
            &["m3_020_mint_half_open_door|Open".to_string()],
            &[
                M3_020_MINT_QUEUE_STATE.to_string(),
                M3_020_SHORT_WORK_STATE.to_string(),
            ],
        )
        .unwrap();
        assert!(mint_echo_after_short_work.contains("側線已被兌牌人接住"));

        let flagstone_after_council = m3_020_scene_object_look_echo(
            "m3_020_flagstone_waiting_lines",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[
                M3_020_MINT_QUEUE_STATE.to_string(),
                M3_020_SHORT_WORK_STATE.to_string(),
                M3_020_PASS_CHECK_STATE.to_string(),
                M3_020_COUNCIL_COMPLAINT_STATE.to_string(),
            ],
        )
        .unwrap();
        assert!(flagstone_after_council.contains("往坡街那側"));
    }

    #[test]
    fn m3_020_followup_echo_requires_matching_prior_payload() {
        assert!(
            m3_020_scene_object_followup_echo(
                "m3_020_mint_half_open_door",
                &["m3_020_mint_half_open_door|Open".to_string()]
            )
            .unwrap()
            .contains("門縫比方才更窄")
        );
        assert!(
            m3_020_scene_object_followup_echo(
                "m3_020_mint_half_open_door",
                &["m3_020_flagstone_waiting_lines|Use".to_string()]
            )
            .unwrap()
            .contains("側線延伸到巷口")
        );
        assert_eq!(
            m3_020_scene_object_followup_echo(
                "m3_020_mint_half_open_door",
                &["bad-payload".to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_020_followup_echo_uses_latest_relevant_event() {
        let waiting_then_mint = vec![
            "m3_020_flagstone_waiting_lines|Use".to_string(),
            "m3_020_mint_half_open_door|Open".to_string(),
        ];
        assert!(
            m3_020_scene_object_followup_echo("m3_020_mint_half_open_door", &waiting_then_mint)
                .unwrap()
                .contains("門縫比方才更窄")
        );

        let mint_then_waiting = vec![
            "m3_020_mint_half_open_door|Open".to_string(),
            "m3_020_flagstone_waiting_lines|Use".to_string(),
        ];
        assert!(
            m3_020_scene_object_followup_echo("m3_020_mint_half_open_door", &mint_then_waiting)
                .unwrap()
                .contains("側線延伸到巷口")
        );
    }
}
