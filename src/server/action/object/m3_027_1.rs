const M3_027_GATHER_BATCH: usize = 3;

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
            tactical_choices: vec![],
        });
        return true;
    }

    let mut gathered_count = 0;
    let mut last_gathered = None;
    for _ in 0..M3_027_GATHER_BATCH {
        if let Some(gathered) = materials::try_gather_material_for_room(player_room, now) {
            let stop = gathered.stock_remaining <= 0;
            last_gathered = Some(gathered);
            gathered_count += 1;
            if stop {
                break;
            }
        } else {
            break;
        }
    }

    let (narrative, success, stock_empty) = if gathered_count > 0 {
        let gathered = last_gathered.as_ref().unwrap();
        let Some(state_payload) =
            m3_027_material_gather_state_for_object(&obj.id, &gathered.item_id)
        else {
            return false;
        };
        let _ = db::add_to_inventory(pid, &gathered.item_id, gathered_count as i32);
        for _ in 0..gathered_count {
            let _ = event::append(
                now,
                pid,
                types::SCENE_OBJECT_ACTION,
                &format!("{}|Gather|{}", obj.id, gathered.item_id),
            );
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, state_payload);
        push_refresh(conn, pid);
        (
            m3_027_material_gather_batch_success_text(&obj.id, gathered, gathered_count),
            true,
            gathered.stock_remaining <= 0,
        )
    } else {
        (
            m3_027_material_gather_empty_text(&obj.id).to_string(),
            false,
            true,
        )
    };

    let (mut actions, move_target_id) = object_followup_buttons(obj, player_room, action, &[], &[]);
    if stock_empty {
        actions.retain(|a| a != "Gather");
    }
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
        tactical_choices: vec![],
    });
    true
}

pub(crate) fn m3_027_material_gather_batch_success_text(
    object_id: &str,
    gathered: &materials::GatheredMaterial,
    count: usize,
) -> String {
    if count <= 1 {
        return m3_027_material_gather_success_text(object_id, gathered);
    }
    match object_id {
        "m3_027_north_bank_reeds" => {
            let trace = if gathered.stock_remaining <= 0 {
                "整片靠近水線的蘆叢都被割低了，水霧漫過斷莖，暫時只剩浮在泥面上的碎葉。"
            } else {
                "蘆叢順著你的手勢矮下一大片，斷口處滲出草汁，霧水沿著整齊的切口滴入泥潭。"
            };
            format!(
                "你撥開貼岸的河霧，順著草莖根部一氣呵成連割了數把；你抖落泥水，將它們俐落地紮成 {} 束「{}」妥善收進背包；{trace}",
                count, gathered.name
            )
        }
        "m3_027_market_sackcloth_pile" => {
            let trace = if gathered.stock_remaining <= 0 {
                "棚腳那堆舊麻袋已被裁割殆盡，只剩幾塊沾滿濕泥的爛邊，搬貨人乾脆把空木箱倒扣過來壓住空地。"
            } else {
                "舊麻袋堆肉眼可見地薄了一大截，旁邊的搬貨人見你動作俐落，也跟著挑揀剩下能包貨的布角。"
            };
            format!(
                "你半蹲在水尾寮渡頭市集棚腳，手起刀落連番裁剪，理出邊角完整、足堪包貨的布料，捲成 {} 卷「{}」收進背包；{trace}",
                count, gathered.name
            )
        }
        "m3_027_plank_wet_rope_coil" => {
            let trace = if gathered.stock_remaining <= 0 {
                "木樁內側只剩幾截浸水發脹的斷頭，船戶見狀把空出的繩釘用力敲緊。"
            } else {
                "木樁上原本層疊的繩圈頓時疏落開來，船戶趁空把剩下還吃得住力的繩頭順向樁後。"
            };
            format!(
                "你踩在微晃的踏板上，探身從木樁內側連續解下並盤好 {} 卷韌度結實的「{}」收進背包；{trace}",
                count, gathered.name
            )
        }
        _ => format!(
            "你動作俐落地從【{}】連續理出 {} 份可用材料收進背包。",
            gathered.origin_label, count
        ),
    }
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
            format!("你從水尾寮渡頭市集棚腳剪下一卷「{}」收進背包；{trace}", gathered.name)
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
            Some("收牌人把價牌翻到背面，秤邊的貨籃已墊滿；這一輪水尾寮渡頭市集不再收新的蘆草束。")
        }
        ("m3_027_market_reed_buyer", "SellReeds") => {
            let has_reeds = db::get_entity(pid)
                .ok()
                .flatten()
                .is_some_and(|ch| db::inventory_has_item(&ch.inventory, M3_027_REED_ITEM_ID));
            if has_reeds {
                None
            } else {
                Some("收牌人看了看你空著的手，沒有伸秤；水尾寮渡頭市集這一輪只收能立刻墊箱、束貨的蘆草束。")
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
                Some("收牌人看了看你空著的手，沒有接繩頭；水尾寮渡頭市集這一輪只收能立刻捆箱的麻繩卷。")
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
    player_room: &str,
    object_id: &str,
    action: &str,
    state_payload: Option<&str>,
) -> bool {
    match (object_id, action, state_payload) {
        ("m3_027_hot_soup_stall", "BuySoup", Some(M3_027_SOUP_WARM_STATE)) => {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -M3_027_SOUP_COST);
            true
        }
        ("m3_027_cargo_pole", "CarryCargo", Some(M3_027_CARGO_HELP_STATE)) => {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, M3_027_CARGO_PAY);
            true
        }
        ("m3_027_market_reed_buyer", "SellReeds", Some(state))
            if state == M3_027_REED_SOLD_STATE || state == M3_027_REED_DEMAND_FILLED_STATE =>
        {
            let _ = db::remove_from_inventory(pid, M3_027_REED_ITEM_ID, 1);
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, M3_027_REED_SELL_PAY);
            true
        }
        (
            "m3_027_market_reed_buyer",
            "SellSackcloth",
            Some(M3_027_SACKCLOTH_DEMAND_FILLED_STATE),
        ) => {
            let _ = db::remove_from_inventory(pid, M3_027_SACKCLOTH_ITEM_ID, 1);
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, M3_027_SACKCLOTH_SELL_PAY);
            true
        }
        ("m3_027_market_reed_buyer", "SellRope", Some(M3_027_ROPE_DEMAND_FILLED_STATE)) => {
            let _ = db::remove_from_inventory(pid, M3_027_ROPE_ITEM_ID, 1);
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, M3_027_ROPE_SELL_PAY);
            true
        }
        ("m3_027_warehouse_tally_board", "CheckCargo", Some(M3_027_WAREHOUSE_TALLY_STATE)) => {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, M3_027_CARGO_TALLY_PAY);
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
            Some("原先讓出的乾位已被別人的包袱佔回去；你再靠近時，長凳只剩濕冷那一段。")
        }
        ("m3_027_hot_soup_stall", "BuySoup" | "QueueBroth")
            if m3_027_soup_taken_state_active(prior_states) =>
        {
            Some("攤主把勺子靠回鍋沿，說這一輪已經輪過你；鍋底的熱氣留給下一個被河風吹透的人。")
        }
        ("m3_027_cargo_pole", "CarryCargo") if m3_027_cargo_help_state_active(prior_states) => {
            Some("你扛過的那副貨擔已經停在踏板邊，市集的人把下一擔交給熟手，沒有再把肩位讓出來。")
        }
        ("m3_027_boat_rope", "HelpRope") if m3_027_rope_help_state_active(prior_states) => {
            Some("船戶看見你已經搭過一手，只讓你留在踏板旁的窄位等船靠穩；繩尾這回交給後面的人。")
        }
        ("m3_027_boat_rope", "HelpRope") if !m3_027_cargo_help_state_active(prior_states) => {
            Some("船戶把繩尾往掌心一收，先指向踏板邊的貨擔；船還沒靠穩，空手擠上去只會擋路。")
        }
        ("m3_027_river_awning", "DryOff") if m3_027_awning_dry_state_active(prior_states) => {
            Some("棚下你晾過的位置已經換成別人的濕包袱；再擠進去，只會把衣角重新蹭濕。")
        }
        ("m3_027_charcoal_brazier", "WarmHands")
            if m3_027_brazier_warm_state_active(prior_states) =>
        {
            Some("炭火盆邊的人看見你烘過手，便把下一個冷得發抖的候渡者推近火口。")
        }
        ("m3_027_waiting_line_chat", "TalkQueue")
            if m3_027_queue_talk_state_active(prior_states) =>
        {
            Some("人龍裡能問的船期和湯價你已聽過一輪；再追著問，只會讓前後的人避開你的眼神。")
        }
        ("m3_027_waiting_line_chat", "QueueFerry")
            if m3_027_queue_ferry_state_active(prior_states) =>
        {
            Some("你已在人龍折角裡，前後的人都按這條線往踏板等；再擠一次，只會把認住的位置弄亂。")
        }
        ("m3_027_market_reed_buyer", "SellReeds")
            if m3_027_reed_demand_filled_state_active(prior_states) =>
        {
            Some("收牌人把價牌翻到背面，秤邊的貨籃已墊滿；這一輪水尾寮渡頭市集不再收新的蘆草束。")
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
            Some("船戶把手掌壓在木牌上，示意你先回北岸候列；沒被人龍認住的人不能直接擠上踏板。")
        }
        ("m3_027_south_landing_rope", "StepAshore")
            if m3_027_landed_south_state_active(prior_states) =>
        {
            Some("你已踩過那道濕泥繩位，後面的人沿同一條痕上坡；再退回去只會把上岸線堵住。")
        }
        ("m3_027_south_landing_rope", "StepAshore")
            if !m3_027_board_ferry_state_active(prior_states) =>
        {
            Some("上岸繩那頭還沒輪到你，南岸的人只讓從踏板內側下來的人先踩穩。")
        }
        ("m3_027_warehouse_tally_board", "CheckCargo")
            if m3_027_warehouse_tally_state_active(prior_states) =>
        {
            Some("記牌人已把那副貨擔劃進倉棚短冊，尾款也結過；牌邊只剩下一輪貨號。")
        }
        ("m3_027_warehouse_tally_board", "CheckCargo")
            if !m3_027_cargo_help_state_active(prior_states) =>
        {
            Some("記牌人翻了翻短冊，沒有找到你送到踏板邊的貨擔痕；沒清過貨的人領不到這行尾款。")
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

