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
            Some("你已在已排定的側線裡，兌牌人抬手要你留在門檻外，不讓後面的人重新擠上來。")
        }
        ("m3_020_mint_half_open_door", "Queue")
            if m3_020_waiting_line_has_been_used(prior_payloads) =>
        {
            Some(
                "你按分出的側線站到門檻外，兌牌人沒有放你進門，只把下一輪要核的牌號往石案邊推近；身後的人跟著把腳步收成一線。",
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
                "你照著看清的牌序站到石案側邊，門衛把你的去向壓進短冊，讓商隊和工坊送件的人各退半步。",
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
            "你排進去的側線已被兌牌人接住，門檻外留下一條窄窄空位；他偶爾把下一張牌號往石案邊推近。",
        ),
        "m3_020_flagstone_waiting_lines" => {
            Some("鑄幣巷口那條側線被接住後，廣場中間少了一股橫穿的人潮，旗手的手勢比先前短。")
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
            Some("你應下的那片短工牌被棚頭人翻到繩結下，商隊卸貨的小隊在棚柱旁收成一列。")
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
            Some("你候檢的那道牌序被門衛壓在石案側邊，商隊和工坊送件的人各讓出半步。")
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
            Some("你押名的短冊被記牌人壓在告示下方，通行牌申訴那一列多了一道折角。")
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
            Some("讀過的幾張通行牌仍壓在石案邊，你再看時能更快分清商隊和工坊送件的候列。")
        }
        ("m3_020_flagstone_waiting_lines", "Use") => {
            Some("你讓出的那一步還留在候列邊，旗手順手把後面的人壓到同一側。")
        }
        ("m3_020_mint_half_open_door", "Open") => Some("門縫比先前更窄，兌牌人的手還壓在門板上。"),
        ("m3_020_hire_short_work_boards", "Read") => {
            Some("最急的那片短工牌仍翻在外側，棚頭人正按那幾個活計點人。")
        }
        ("m3_020_council_notice_strip", "Read") => {
            Some("你看過的幾行告示還在階邊，記牌人按著短冊不讓申訴人擠到坡街中間。")
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
            Some("廣場那邊理出的側線延伸到巷口，等換票的人比先前收得更窄。")
        }
        ("m3_020_hire_short_work_boards", "m3_020_flagstone_waiting_lines", "Use") => {
            Some("旗石廣場的候列分清後，等工的人也被棚頭人壓到棚柱旁。")
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
