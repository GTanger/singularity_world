
fn canonical_or_venue_talk_reply(
    target: &Character,
    talk_name: &str,
    room_name: &str,
    player_input: &str,
    stance: NpcStanceChoice,
    has_ai_model: bool,
) -> Option<(String, String)> {
    if is_asking_name(player_input) {
        let gender_str = match target.gender {
            Some(crate::entity::Gender::F) => Some("F"),
            Some(crate::entity::Gender::M) => Some("M"),
            None => None,
        };
        let short_real_name = crate::store::resolve_npc_person_name(
            &target.id,
            &target.display_title,
            gender_str,
        );
        let role_raw = crate::store::npc_persistent_role_label(&target.id, &target.display_title);
        let role_desc = role_raw
            .split(['|', '｜'])
            .next()
            .unwrap_or(&role_raw)
            .trim();
        let line = if !target.display_title.is_empty()
            && (target.display_title.contains('|') || target.display_title.contains('｜'))
        {
            format!("「你好，我是{short_real_name}，在這邊擔任{role_desc}，有什麼事直接問我就好。」")
        } else if room_name.contains("學院")
            || room_name.contains("講堂")
            || room_name.contains("藏書閣")
            || room_name.contains("書局")
        {
            format!("「我叫{short_real_name}，平時是城裡的{role_desc}，常來{room_name}這邊看書查資料，之後遇到叫我{short_real_name}就好。」")
        } else {
            format!("「我叫{short_real_name}，平時是城裡的{role_desc}，常在{room_name}這一帶活動，之後遇到直接叫我{short_real_name}就好。」")
        };
        let narrative = if player_input.trim() != "（搭話）" && !player_input.trim().is_empty() {
            format!("【{short_real_name}】（{role_desc}）說道：{line}")
        } else {
            format!("你向【{short_real_name}】詢問姓名。【{short_real_name}】（{role_desc}）說道：{line}")
        };
        let npc_reply = line;
        return Some((narrative, npc_reply));
    }

    let trimmed_in = player_input.trim();
    let is_asking_wares = ["賣什麼", "多少錢", "有什麼賣", "吃什麼", concat!("菜", "單"), "價錢", "售價"]
        .iter()
        .any(|k| trimmed_in.contains(k));
    if has_ai_model
        && !trimmed_in.is_empty()
        && trimmed_in != "（搭話）"
        && !is_asking_name(trimmed_in)
        && !is_asking_wares
    {
        return None;
    }
    if target.id.starts_with("wt_vendor_") {
        let is_prepping_or_moving = target.current_activity.contains("準備")
            || target.current_activity.contains("推車")
            || target.current_activity.contains("步行前往")
            || target.current_activity.contains("動身前往")
            || target.current_activity.contains("返回【")
            || target.current_activity.contains("趕往");
        let is_resting = target.current_activity.contains("收攤")
            || target.current_activity.contains("安歇")
            || target.current_activity.contains("歇息")
            || target.current_activity.contains("熟睡")
            || target.current_activity.contains("熄燈");
        let is_actively_vending = !is_prepping_or_moving && !is_resting;
        let (item_id, item_name, default_price, _) =
            crate::server::action::trade::resolve_vendor_specialty(&target.id)
                .or_else(|| crate::server::action::trade::vendor_stall_offer_for_npc(&target.id))
                .unwrap_or(("wt_cat_0395", "招牌特色小吃", 3, ""));
        let cat_obj = crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS
            .iter()
            .find(|o| o.id == item_id);

        let sensory_clause = cat_obj
            .map(|o| {
                let raw = o.look_text.trim();
                let after_quote = if let Some(stripped) = raw.strip_prefix('「') {
                    if let Some((_, rest)) = stripped.split_once('」') {
                        rest.trim()
                    } else {
                        raw
                    }
                } else {
                    raw
                };
                after_quote
                    .split(['，', '。'])
                    .find(|seg| !seg.trim().is_empty())
                    .unwrap_or("真材實料")
                    .trim()
            })
            .unwrap_or("真材實料");

        let action_clause = cat_obj
            .map(|o| {
                let raw = o.action_text.trim();
                raw.split(['，', '。'])
                    .find(|seg| !seg.trim().is_empty())
                    .unwrap_or("滋味醇正道地")
                    .trim()
            })
            .unwrap_or("滋味醇正道地");

        let act = &target.current_activity;

        let line = if !is_actively_vending {
            if is_prepping_or_moving {
                format!("「這批【{item_name}】（{sensory_clause}）才備妥，我眼下{act}，等到了攤位開張再來嚐鮮選購吧！」")
            } else {
                format!("「今日的【{item_name}】已經收攤歇息了（眼下{act}），待開市時段再來光顧吧！」")
            }
        } else {
            let mut h: i64 = 0;
            for c in target.id.chars() {
                h = h.wrapping_add(c as i64);
            }
            let now_idx = game::now_unix().saturating_add(h).unsigned_abs() as usize / 15;

            let settlement = crate::world::materials::infer_origin_settlement(room_name);
            let (stock, price_mg) = crate::game::object_ecology::get_market_item_stock_and_price(
                settlement,
                item_id,
                default_price,
            );

            match now_idx % 3 {
                0 => format!(
                    "「走過路過別錯過！這【{item_name}】（{sensory_clause}），每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，趁熱品味最是地道！」"
                ),
                1 => format!(
                    "「客官請看，招牌【{item_name}】現貨新鮮（{action_clause}），每份 {price_mg} 鎂，攤頭僅餘 {stock} 份，隨時為您備好！」"
                ),
                _ => format!(
                    "「今日這批【{item_name}】用料實在（{sensory_clause}），每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，要帶一份嚐嚐嗎？」"
                ),
            }
        };

        let narrative = format_talk_narrative(target, talk_name, &line, player_input);
        let npc_reply = if player_input.trim().is_empty() || player_input == "（搭話）" {
            line
        } else {
            narrative.clone()
        };
        return Some((narrative, npc_reply));
    }
    let is_on_the_road = target.current_activity.contains("步行前往")
        || target.current_activity.contains("動身前往")
        || target.current_activity.contains("返回【")
        || target.current_activity.contains("踏著夜色")
        || target.current_activity.contains("趕往");

    if is_on_the_road && !is_asking_name(player_input) {
        let ob_suffix = if let Some((kind, _)) = crate::store::get_active_npc_obligation(&target.id) {
            match kind.as_str() {
                "restock_goods" => "，得趕緊去補齊鋪子裡的現貨",
                "repair_tool" => "，得先去鐵坊把傢伙修整妥當",
                "treat_injury" => "，身子有些微恙得先找醫工瞧瞧",
                "repay_debt" => "，手頭票單正趕著去核銷",
                _ => "，手頭正趕著辦事",
            }
        } else {
            "，手頭正趕著辦事"
        };
        let act_phrase = if is_on_the_road { "正要趕路" } else { "手頭正忙著" };
        let line = match stance {
            NpcStanceChoice::Refuse => format!(
                "「我眼下{act_phrase}{ob_suffix}，實在分身乏術，恕不能奉陪了。」"
            ),
            NpcStanceChoice::Bargain => format!(
                "「我眼下{act_phrase}{ob_suffix}，若有要緊事，長話短說隨我一併走一段吧。」"
            ),
            NpcStanceChoice::Deflect => format!(
                "「我眼下{act_phrase}{ob_suffix}，前路正忙，你若不急，待我回鋪子再說不遲。」"
            ),
            NpcStanceChoice::Accept => format!(
                "「我眼下{act_phrase}{ob_suffix}，你若有要事，不妨隨我同行或待我回鋪子再細談。」"
            ),
        };
        let narrative = format_talk_narrative(target, talk_name, &line, player_input);
        return Some((narrative, line));
    }

    if has_ai_model
        && !player_input.trim().is_empty()
        && player_input.trim() != "（搭話）"
        && !is_asking_name(player_input)
    {
        return None;
    }

    let title = target.display_title.as_str();
    let mut h: i64 = 0;
    for c in target.id.chars() {
        h = h.wrapping_add(c as i64);
    }
    let now_idx = game::now_unix().saturating_add(h).unsigned_abs() as usize / 15;

    let is_task = player_input.contains("差事")
        || player_input.contains("委託")
        || player_input.contains("幫忙")
        || player_input.contains("手頭活")
        || player_input.contains("活計")
        || player_input.contains("工作")
        || player_input.contains("工錢");

    let is_trade = player_input.contains("買")
        || player_input.contains("賣")
        || player_input.contains("貨")
        || player_input.contains("器物")
        || player_input.contains("東西")
        || player_input.contains("交易");

    if player_input.trim() != "" && player_input != "（搭話）" && !is_task && !is_trade {
        return None;
    }
    let venue_task_dynamic_line: String;
    let line: Option<&str> = if let Some(venue) = if super::trade::is_eligible_shop_staff(target, &room_name) {
        crate::grid::resolve_room_venue_object(&room_name, title)
    } else {
        None
    } {
        if is_task {
            let now = crate::game::now_unix();
            if let Some((ev, sp)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
                &room_name,
                Some(talk_name),
                now,
            ) {
                venue_task_dynamic_line = format!(
                    "{}正忙著手頭活計，見你問起差事便頷首道：「場上正逢【{}】——{}。你若願搭手：{}」",
                    talk_name, ev.event_title, ev.trigger_and_causal_chain, sp.player_actions
                );
                Some(venue_task_dynamic_line.as_str())
            } else {
                venue_task_dynamic_line = format!(
                    "「我手頭{}。你若得空願在此搭手相助，鋪裡依行規記你一份酬勞。」",
                    venue.on_duty_fixture_activity
                );
                Some(venue_task_dynamic_line.as_str())
            }
        } else if is_trade && !venue.trade_talk_pitch.is_empty() {
            Some(venue.trade_talk_pitch)
        } else if !venue.on_duty_talk_lines.is_empty() {
            Some(venue.on_duty_talk_lines[now_idx % venue.on_duty_talk_lines.len()])
        } else {
            None
        }
    } else if let Some(venue) = crate::grid::resolve_market_or_specialty_room_object(&room_name, &talk_name) {
        if !venue.on_duty_talk_lines.is_empty() {
            Some(venue.on_duty_talk_lines[now_idx % venue.on_duty_talk_lines.len()])
        } else {
            None
        }
    } else {
        None
    };

    line.map(|l| {
        let active_ob = crate::store::get_active_npc_obligation(&target.id);
        let synthesized_line = synthesize_talk_reply_with_stance_and_obligation(
            l,
            stance,
            active_ob.as_ref(),
        );
        let narrative = format_talk_narrative(target, talk_name, &synthesized_line, player_input);
        let npc_reply = if player_input.trim().is_empty() || player_input == "（搭話）" {
            synthesized_line
        } else {
            narrative.clone()
        };
        (narrative, npc_reply)
    })
}

fn is_open_trade_input(input: &str) -> bool {
    let text = input.trim().to_lowercase();
    if text.is_empty() || text == "（搭話）" {
        return false;
    }
    const NEEDLES: &[&str] = &[
        "想看看你有什麼",
        "有什麼可以買",
        "有什麼賣",
        "看看貨",
        "看貨",
        "買東西",
        "我要買",
        "賣什麼",
        "賣些什麼",
        "有貨嗎",
        "交易",
        "trade",
        "buy",
    ];
    NEEDLES.iter().any(|needle| text.contains(needle))
}

fn target_can_open_trade(target: &Character, target_id: &str, player_room: &str) -> bool {
    let room_name = super::target_grid_room_name(target);
    let has_live_stock = crate::db::get_inventory(&target.inventory, target.vit)
        .items
        .iter()
        .any(|i| i.qty > 0);
    if super::trade::target_has_shop_offer(target, &room_name) || has_live_stock {
        return true;
    }
    if target.kind == EntityKind::Npc {
        return db::get_sockets_for_npc(target_id, player_room)
            .iter()
            .any(|socket| socket == "Trade");
    }
    Character::sockets().contains(&Verb::Trade)
}

// ============================================================================
// 對話實體世界效果：義務交割、立約待辦、同道相助
// ============================================================================

/// 可交割物資清單（優先順序、識別碼、顯示名、相關語義關鍵詞）
pub const DELIVERABLE_ITEMS: [(&str, &str, &[&str]); 17] = [
    ("craft_wooden_tool_handle", "堅韌硬木工具長柄", &["工具長柄", "木柄", "長柄", "工具柄", "柄"]),
    ("mat_plank_board", "樟松木板料", &["樟松木板料", "木板料", "木板", "板料", "板"]),
    ("mat_charcoal_sack", "硬木白炭袋", &["硬木白炭袋", "白炭袋", "白炭", "木炭", "炭袋", "炭"]),
    ("mat_kindling_bundle", "乾燥柴枝束", &["乾燥柴枝束", "柴枝束", "柴枝", "柴", "薪"]),
    ("mat_timber_log", "伐採原木", &["伐採原木", "原木", "木頭", "木料"]),
    ("craft_pine_torch", "浸脂松明火把", &["浸脂松明火把", "松明火把", "火把", "松明"]),
    ("mat_iron_scrap", "回收熟鐵料塊", &["回收熟鐵料塊", "熟鐵料塊", "精製鐵料", "熟鐵", "鐵料", "鐵塊", "鐵", "礦"]),
    ("mat_repair_strip", "修補熟鐵薄片", &["修補熟鐵薄片", "修補鐵片", "鐵片", "修補木條", "修補條"]),
    ("craft_meridian_salve", "溫經通絡通痹膏", &["溫經通絡通痹膏", "通痹膏", "藥膏", "膏藥", "草藥膏", "藥"]),
    ("craft_calming_tea_pack", "山風安神草茶包", &["山風安神草茶包", "安神茶包", "茶包", "草茶包", "茶"]),
    ("mat_foraged_herb", "山野草藥", &["山野草藥", "鮮藥草", "草藥", "山野草", "草", "藥"]),
    ("mat_hemp_rope_roll", "浸油麻繩卷", &["浸油麻繩卷", "麻繩卷", "麻繩", "繩卷", "繩"]),
    ("mat_canvas_roll", "防雨油浸帆布卷", &["防雨油浸帆布卷", "油浸帆布卷", "帆布卷", "帆布", "布卷", "布"]),
    ("mat_coarse_paper_ream", "毛邊竹棉宣紙刀", &["毛邊竹棉宣紙刀", "竹棉宣紙刀", "宣紙刀", "宣紙", "粗紙", "紙刀", "紙"]),
    ("mat_sealing_pitch", "水密煮製熟船膠", &["水密煮製熟船膠", "熟船膠", "船膠", "水密膠", "瀝青", "松膠", "膠"]),
    ("mat_leather_strip", "熟皮削製護具帶", &["熟皮削製護具帶", "護具帶", "熟皮帶", "皮帶", "熟皮", "生皮", "皮"]),
    ("mat_reed_bundle", "水澤老蘆草綑", &["水澤老蘆草綑", "老蘆草綑", "老蘆草", "老蘆", "蘆草", "蘆"]),
];

/// 玩家表達交付、協助、修繕或補給意圖之關鍵詞
pub const DELIVERY_INTENT_KEYWORDS: &[&str] = &[
    "交付", "交割", "交差", "帶了", "帶來", "送來", "給你", "幫你", "修好", "補齊",
    "拿去", "收下", "完成",
];

/// 玩家明確主動贈予或交付之強意圖關鍵詞
pub const EXPLICIT_OFFER_KEYWORDS: &[&str] = &[
    "給你", "交給", "幫你修", "這束柴", "這塊木", "拿去用",
];

/// 請求物資支援關鍵詞（Accept 觸發）
pub const ACCEPT_MATERIAL_KEYWORDS: &[&str] = &[
    "借我", "借一", "借點", "借些", "討點", "討些", "勻些", "周轉", "缺料", "柴火", "生火", "借個火",
];

/// 尋求安歇或求助關鍵詞（Accept 觸發）
pub const ACCEPT_COMFORT_KEYWORDS: &[&str] = &[
    "冷", "寒", "累", "倦", "傷", "餓", "渴", "幫", "救",
];

/// 判斷玩家輸入是否包含交付/協助意圖
pub fn is_delivery_intent_utterance(input: &str) -> bool {
    DELIVERY_INTENT_KEYWORDS.iter().any(|kw| input.contains(kw))
}

/// 判斷玩家輸入是否包含明確主動交割語句
pub fn is_explicit_offer_utterance(input: &str) -> bool {
    EXPLICIT_OFFER_KEYWORDS.iter().any(|kw| input.contains(kw))
}

/// 判斷玩家輸入是否請求柴木物資支援
pub fn is_accept_material_utterance(input: &str) -> bool {
    if input.contains("借問") || input.contains("借過") {
        return false;
    }
    ACCEPT_MATERIAL_KEYWORDS.iter().any(|kw| input.contains(kw))
}

/// 判斷玩家輸入是否尋求歇息調養
pub fn is_accept_comfort_utterance(input: &str) -> bool {
    let cleaned = input.replace("寒暄", "");
    ACCEPT_COMFORT_KEYWORDS.iter().any(|kw| cleaned.contains(kw))
}

/// 根據對象稱謂與當前活動推論 NPC 待辦義務
pub fn infer_npc_obligation(talk_name: &str, current_activity: &str) -> (&'static str, &'static str) {
    let combined = format!("{talk_name}:{current_activity}");
    if combined.contains('鍛')
        || combined.contains('鐵')
        || combined.contains('爐')
        || combined.contains('錘')
        || combined.contains("工坊")
    {
        ("fuel_shortage", "熟鐵料塊與硬木白炭袋添爐精鍛")
    } else if combined.contains('醫')
        || combined.contains('診')
        || combined.contains('藥')
        || combined.contains('茶')
    {
        ("deliver_goods", "山野草藥與安神草茶包調配")
    } else if combined.contains('漁')
        || combined.contains('網')
        || combined.contains('船')
        || combined.contains('渡')
        || combined.contains('埠')
    {
        ("repair_tool", "水密熟船膠與強韌繩纜修固船具")
    } else if combined.contains('織')
        || combined.contains('布')
        || combined.contains('紙')
        || combined.contains('皮')
    {
        ("deliver_goods", "防雨帆布卷與熟皮護具帶備料")
    } else {
        ("deliver_goods", "乾燥柴枝束與伐採原木備料")
    }
}

/// 依玩家輸入、待辦提示與持有物過濾選取最相符的可交割物資
pub fn select_deliverable_item(
    player_input: &str,
    obligation_hint: Option<&str>,
    has_item: impl Fn(&str) -> bool,
) -> Option<(&'static str, &'static str)> {
    // 1. 若玩家語句直接提及特定器物/原料，且身上持有，優先交割該物件
    for (item_id, item_name, keywords) in DELIVERABLE_ITEMS {
        if keywords.iter().any(|k| player_input.contains(k)) && has_item(item_id) {
            return Some((item_id, item_name));
        }
    }
    // 2. 若待辦事項提示中提及特定原料/器物，且身上持有，優先交割能解困者
    if let Some(hint) = obligation_hint {
        for (item_id, item_name, keywords) in DELIVERABLE_ITEMS {
            if keywords.iter().any(|k| hint.contains(k)) && has_item(item_id) {
                return Some((item_id, item_name));
            }
        }
    }
    // 3. 全總典外鍵兜底：依玩家語句比對 PENINSULA_WORLD_OBJECTS（名稱、或截取後綴／末尾漢字）
    for entry in crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS {
        let matches_input = if player_input.contains(entry.name) {
            true
        } else if entry.name.len() >= 6 {
            let suffix = if let Some((_, after_dot)) = entry.name.split_once('・') {
                after_dot
            } else {
                ""
            };
            if !suffix.is_empty() && player_input.contains(suffix) {
                true
            } else {
                let chars: Vec<char> = entry.name.chars().collect();
                if chars.len() >= 3 {
                    let last3: String = chars[chars.len() - 3..].iter().collect();
                    let last2: String = chars[chars.len() - 2..].iter().collect();
                    player_input.contains(&last3) || player_input.contains(&last2)
                } else if chars.len() == 2 {
                    let last2: String = chars.iter().collect();
                    player_input.contains(&last2)
                } else {
                    false
                }
            }
        } else {
            false
        };

        if matches_input && has_item(entry.id) {
            return Some((entry.id, entry.name));
        }
    }
    // 4. 全總典外鍵兜底：依待辦提示比對 PENINSULA_WORLD_OBJECTS
    if let Some(hint) = obligation_hint {
        for entry in crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS {
            if (hint.contains(entry.id) || hint.contains(entry.name)) && has_item(entry.id) {
                return Some((entry.id, entry.name));
            }
        }
    }
    // 5. 依標準優先順序檢查身上持有的可交割物件
    for (item_id, item_name, _) in DELIVERABLE_ITEMS {
        if has_item(item_id) {
            return Some((item_id, item_name));
        }
    }
    None
}

/// 尋找玩家行囊中可交割的物資
pub fn find_deliverable_item(
    pid: &str,
    player_input: &str,
    obligation_hint: Option<&str>,
) -> Option<(&'static str, &'static str)> {
    select_deliverable_item(player_input, obligation_hint, |item_id| {
        db::inventory_has_item(pid, item_id)
    })
}

