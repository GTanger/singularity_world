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
        return Some("水尾寮渡頭市集包貨單三行都被按下濕指印：蘆草已墊箱，麻袋布已包貨，麻繩也把踏板口那批箱包重新捆緊；看貨人把秤邊讓給下一輪零散貨。".to_string());
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
            "水尾寮渡頭市集包貨單上{}已被收進秤邊，{}仍壓在單子外側；搬貨人和玩家都照同一張單交料，誰先補上就會改掉這行需求。",
            filled.join("、"),
            waiting.join("、")
        ))
    } else {
        Some(format!(
            "水尾寮渡頭市集包貨單上{}已被收進秤邊，{}仍壓在單子外側；看貨人和搬貨人都照這張單抽料。",
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
            return Some("水尾寮渡頭市集送來的箱包已在倉棚門邊分好：濕蘆草墊住箱角，麻袋布裹住磨邊，麻繩把同一批零散貨重新捆成一排。".to_string());
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
            "水尾寮渡頭市集那邊已把{}送到倉棚門前，記牌人把這批零散貨壓進同一行，不再像散貨那樣堆在泥邊。",
            parts.join("、")
        ));
    }
    if reed_filled && sackcloth_filled && rope_filled {
        Some("水尾寮渡頭市集包貨單三行都被按下濕指印，秤邊只剩下一輪零散貨要等。".to_string())
    } else {
        None
    }
}

fn m3_027_reed_gathered_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_north_bank_reeds" => {
            Some("新割過的蘆叢矮下一片，斷口旁另有水珠順著葉脈往泥裡落。")
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
            Some("已剪過的舊麻袋堆薄了一層，棚腳露出被踩濕的爛邊和散線。")
        }
        "m3_027_market_reed_buyer" => {
            Some("看貨人看見你背包裡露出的麻袋布邊，先把包貨單那行壓到秤邊。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("倉棚前場有人指向水尾寮渡頭市集棚腳，說那邊新剪出的麻袋布能先裹磨角木箱。")
        }
        _ => None,
    }
}

fn m3_027_rope_gathered_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_plank_wet_rope_coil" => {
            Some("已理過的濕繩圈少了一道，木樁內側只剩船戶重新壓緊的繩頭。")
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
            Some("你坐過的乾位已被包袱佔回去，凳腳旁還留著一點被鞋底蹭開的濕泥。")
        }
        _ => None,
    }
}

fn m3_027_soup_warm_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_hot_soup_stall" => {
            Some("你的空碗還倒扣在攤邊，鍋裡熱氣薄了一層，攤主把收過的錢角壓進濕布底下。")
        }
        "m3_027_wet_bench" => Some("掌心的熱湯氣還沒散，你再看長凳時，河風不像先前那麼割手。"),
        "m3_027_boat_rope" => Some("湯碗留下的熱意還在指節上，你抓繩時不至於立刻被濕繩冰住。"),
        _ => None,
    }
}

fn m3_027_free_broth_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_hot_soup_stall" => {
            Some("你等過的桶側短線又接上一小截人，攤主把半碗淡湯留給沒錢買滿碗的候渡者。")
        }
        "m3_027_waiting_line_chat" => Some("人龍裡有人看見你拿過分湯，說話時把湯價壓低了一點。"),
        _ => None,
    }
}

fn m3_027_cargo_help_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_cargo_pole" => {
            Some("扛走一副貨擔讓泥地空出一道深痕，剩下的麻袋被水尾寮渡頭市集的人往棚腳收。")
        }
        "m3_027_plank_boarding_marker" => {
            Some("清出的貨擔痕停在踏板木牌旁，船戶不用再把上船線和搬貨線分開喊。")
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
            Some("已收過的繩圈被壓在木樁內側，船戶把那段窄位留著，等下一次船腹貼岸。")
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
            Some("你站過的棚下位置被濕包袱填上，舊帆布邊還滴著慢慢變細的水線。")
        }
        "m3_027_wet_bench" => Some("衣角乾了一些，你再看長凳時，濕痕不再像先前那樣貼著膝蓋。"),
        _ => None,
    }
}

fn m3_027_brazier_warm_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_charcoal_brazier" => {
            Some("炭火盆旁你讓出的半掌空位被下一個人補上，煙氣貼著他的袖口往外散。")
        }
        "m3_027_boat_rope" => Some("指節被炭火烘過後，再碰濕繩時不至於立刻僵住。"),
        _ => None,
    }
}

fn m3_027_queue_talk_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_waiting_line_chat" => {
            Some("你問過的那幾個候渡者換成低聲閒談，船期和湯價已在人龍裡傳開。")
        }
        "m3_027_boat_rope" => {
            Some("人龍裡聽來的船期讓你看船戶手勢時更快分清哪一次靠岸能上前搭手。")
        }
        _ => None,
    }
}

fn m3_027_queue_ferry_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_waiting_line_chat" => {
            Some("你排進去的那段人龍被後面的人補緊，候渡者也按同一條木牌線往踏板口慢慢挪。")
        }
        "m3_027_boat_rope" => {
            Some("船戶看見你已排進人龍，手掌在繩上停了一下，只等下一次踏板前清出縫。")
        }
        "m3_027_hot_soup_stall" => {
            Some("你在人龍裡的位置被人認住後，湯攤那側的人不再把你擠回桶邊短線。")
        }
        "m3_027_plank_boarding_marker" => {
            Some("船戶已在木牌內側認住你的人龍位置，踏板口留出一道等你上前的窄縫。")
        }
        _ => None,
    }
}

fn m3_027_reed_sold_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_reed_buyer" => {
            Some("你交出的蘆草已被抽去墊在貨籃角落，收牌人把點數完的鎂收進木牌下，價牌還沒有翻面。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("水尾寮渡頭市集那邊新墊的蘆草跟著貨籃進到棚門旁，木箱角不再直接磨在泥地上。")
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
            Some("水尾寮渡頭市集收滿的濕蘆草已墊進貨籃和木箱角，倉棚前場少了一點箱底磨泥的聲音。")
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
            Some("市集送來的麻袋布已裹住磨角木箱，倉棚門邊少了木角刮袋的聲音。")
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
            Some("踏板口補來的麻繩把零散貨重新捆成一排，倉棚門邊的箱包不再散開。")
        }
        "m3_027_plank_wet_rope_coil" => {
            Some("被收走的麻繩已用在踏板口那批箱包上，木樁內側只留給船戶的繩尾。")
        }
        _ => None,
    }
}

fn m3_027_board_ferry_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_plank_boarding_marker" => {
            Some("你進到踏板內側後，木牌前的人群收回半步，等下一個被船戶點到。")
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
            Some("踩出的泥痕被後面的人沿著走，南岸上坡那條線沒有再散開。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("南岸上來的人流分出一股往倉棚前場，記牌人把短冊往棚門邊推近。")
        }
        _ => None,
    }
}

fn m3_027_warehouse_tally_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_warehouse_tally_board" => {
            Some("你對過的那行貨擔已被劃進短冊，牌角留下按過未乾的濕指印。")
        }
        "m3_027_market_reed_buyer" => {
            Some("倉棚那邊收走一行貨後，市集價牌旁少了一次催貨聲，收牌人把秤砣壓得更穩。")
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

pub(crate) fn m3_027_reed_batch_sale_count(
    available_reeds: i32,
    prior_actions: &[String],
    prior_states: &[String],
) -> usize {
    if m3_027_reed_market_demand_filled(prior_actions, prior_states) {
        return 0;
    }
    let sold = m3_027_reed_sales_count(prior_actions);
    let remaining = M3_027_REED_MARKET_DEMAND_PER_ROUND.saturating_sub(sold);
    (available_reeds.max(0) as usize).min(remaining)
}

fn player_inventory_item_quantity(pid: &str, item_id: &str) -> i32 {
    let Ok(Some(ch)) = db::get_entity(pid) else {
        return 0;
    };
    #[derive(serde::Deserialize)]
    struct Entry {
        item_id: String,
        #[serde(default)]
        qty: i32,
    }
    let entries: Vec<Entry> = serde_json::from_str(&ch.inventory).unwrap_or_default();
    entries
        .into_iter()
        .filter(|e| e.item_id == item_id)
        .map(|e| e.qty.max(0))
        .sum()
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

