
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

        let line = if is_prepping_or_moving || is_resting {
            match target.id.as_str() {
                "wt_vendor_roast_beef" => {
                    if is_prepping_or_moving {
                        "「炭火跟原牛肉串才備妥，我這正要推車前往三水口南街，稍後開火了再來嚐鮮啊！」"
                    } else {
                        "「今天火候差不多啦，炭爐封好了，肉串也都賣清收攤囉！明日午後南街見！」"
                    }
                }
                "wt_vendor_noodle_soup" => {
                    if is_prepping_or_moving {
                        "「大骨高湯才熬出濃白香氣，我正張羅著推車去南街開張，稍後見啊！」"
                    } else {
                        "「大湯鍋跟碗筷都洗刷乾淨收攤啦，眼下歇歇腿，想吃麵明日傍晚再來坐！」"
                    }
                }
                "wt_vendor_pepper_bun" => {
                    if is_prepping_or_moving {
                        "「老麵醒好了、青蔥鮮肉餡也調足了黑胡椒，我這就推車前往南街開爐，稍等我生火啊！」"
                    } else {
                        "「今日的胡椒餅全貼爐賣光啦，炭桶餘燼封妥收攤了，想吃香酥胡椒餅明天請早！」"
                    }
                }
                "wt_vendor_herbal_tea" => {
                    if is_prepping_or_moving {
                        "「仙草凍與黑糖才用井泉鎮涼裝甕，我正推著攤車過去南街，待會擺妥攤位再為您盛一盞。」"
                    } else {
                        "「冰甕與銅盞皆已洗淨收妥歇息了，夜涼了你也早些安歇，明日午後再來喝茶。」"
                    }
                }
                "wt_vendor_tofu_pudding" => {
                    if is_prepping_or_moving {
                        "「豆漿才點好鹽滷成凍、黑糖薑汁正溫著呢，我正推車出門去南街，稍候就開賣囉。」"
                    } else {
                        "「桶裡的熱豆花賣完囉，木桶洗刷好在屋裡歇腳泡黃豆呢，想吃熱豆花明日再來找阿伯！」"
                    }
                }
                "wt_vendor_rice_roll" => {
                    if is_prepping_or_moving {
                        "「荷葉糯米飯正悶著香氣呢，我這就推車前往北街早市，想吃早點稍後過去找我！」"
                    } else {
                        "「早市收攤啦，眼下挑選完明日的配料正歇著呢，想吃炊飯明日清早來北街！」"
                    }
                }
                "wt_vendor_soy_milk" => {
                    if is_prepping_or_moving {
                        "「石磨才磨出兩大桶濃米漿，我正要推去北街架攤呢，稍後見！」"
                    } else {
                        "「銅壺跟石磨都擦拭妥當收攤休息啦，想喝香濃溫米漿明日趕早啊！」"
                    }
                }
                "wt_vendor_morning_produce" => {
                    if is_prepping_or_moving {
                        "「鮮菜已從田裡引泉水沖洗乾淨裝簍，正準備推去早市，待會來挑鮮菜啊！」"
                    } else {
                        "「今天摘的鮮菜全都賣完收攤啦，眼下歇歇腳準備去理田，明早請早囉！」"
                    }
                }
                "wt_vendor_morning_fish" => {
                    if is_prepping_or_moving {
                        "「清晨網到的青殼河蝦才分盆裝妥，正趕著推去早市，稍後來看生猛活蝦！」"
                    } else {
                        "「今日的生猛河蝦早已搶購一空收攤啦，木盆都沖乾淨了，想吃河鮮明日天亮早點來！」"
                    }
                }
                _ => {
                    if is_prepping_or_moving {
                        "「手頭正張羅著攤車備料出門，待會攤位擺好歡迎來捧場啊！」"
                    } else {
                        "「今日攤頭上的東西都賣清收攤啦，眼下正歇腳呢，明日請早！」"
                    }
                }
            }
            .to_string()
        } else {
            let mut h: i64 = 0;
            for c in target.id.chars() {
                h = h.wrapping_add(c as i64);
            }
            let now_idx = game::now_unix().saturating_add(h).unsigned_abs() as usize / 15;

            let (_item_id, item_name, price_mg, stock) =
                crate::server::action::trade::get_vendor_stall_stock(&target.id)
                    .unwrap_or(("", "招牌小吃", 3, 5));

            match target.id.as_str() {
                "wt_vendor_roast_beef" => match now_idx % 3 {
                    0 => format!(
                        "「來來來，走過路過別錯過！炭火正旺，這【{item_name}】烤得外焦裡嫩、肉汁直冒，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，趁熱吃最過癮！」"
                    ),
                    1 => format!(
                        "「肚子餓了吧？我鄭火旺的烤肉串都是挑最新鮮的原肉現烤，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，來一串暖暖胃！」"
                    ),
                    _ => format!(
                        "「聞到炭火香氣了嗎？才撒上粗鹽與香料，油脂滋滋作響！每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，要帶幾串趁手啊？」"
                    ),
                },
                "wt_vendor_noodle_soup" => match now_idx % 3 {
                    0 => format!(
                        "「進來坐喔！大骨高湯熬了整整大半日，這【{item_name}】熱騰騰又順口，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，暖胃最踏實！」"
                    ),
                    1 => format!(
                        "「吹著風餓了吧？來罔市這喝碗熱湯，現煮的【{item_name}】麵條Ｑ彈，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，現點現煮為你下鍋！」"
                    ),
                    _ => format!(
                        "「在外奔波辛苦了！切仔麵配顆入味的滷蛋最對味，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，趁熱吃，湯不夠隨時跟我說！」"
                    ),
                },
                "wt_vendor_pepper_bun" => match now_idx % 3 {
                    0 => format!(
                        "「小心燙手啊！初用鐵鏟從壁爐裡鏟起這【{item_name}】，酥皮脆、肉汁燙，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，趁熱咬一口滿嘴蔥香！」"
                    ),
                    1 => format!(
                        "「我張阿發做餅講究老麵發酵、厚肉重黑胡椒，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，吃過就知道真材實料！」"
                    ),
                    _ => format!(
                        "「新出爐這爐火候正頂！外皮烤得金黃酥脆，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，小心咬開燙嘴爆汁啊！」"
                    ),
                },
                "wt_vendor_herbal_tea" => match now_idx % 3 {
                    0 => format!(
                        "「天氣燥熱心神難定吧？來嚐一盞【{item_name}】，滑嫩仙草配手炒黑糖，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，生津降火最是舒暢。」"
                    ),
                    1 => format!(
                        "「客人請慢走，我這陶甕冰鎮的【{item_name}】清涼沁脾，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，喝一口解油膩又潤喉呢。」"
                    ),
                    _ => format!(
                        "「以古法提煉的甘美仙草，沁涼滑順，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，喝一盅消消暑氣吧。」"
                    ),
                },
                "wt_vendor_tofu_pudding" => match now_idx % 3 {
                    0 => format!(
                        "「慢慢來不著急。阿伯用老井泉沖的鹽滷豆花細緻滑嫩，這【{item_name}】每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，熱熱吃暖胃又補氣。」"
                    ),
                    1 => format!(
                        "「老薑汁文火煨透黑糖，辛香帶甜，配上現舀的【{item_name}】，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，來一碗坐著歇歇腳吧。」"
                    ),
                    _ => format!(
                        "「古法點出來的豆香最是純粹。這碗【{item_name}】每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，入口即化，嚐一碗定定心神。」"
                    ),
                },
                "wt_vendor_rice_roll" => match now_idx % 3 {
                    0 => format!(
                        "「早啊年輕人！掀開荷葉撲鼻香，這【{item_name}】粒粒糯米透著竹香油亮，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，吃飽才有力氣幹活！」"
                    ),
                    1 => format!(
                        "「我吳秀琴的炊飯用料最實在，荷葉裹著香菇蛋黃與油蔥，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，帶一份路上趁熱吃！」"
                    ),
                    _ => format!(
                        "「清晨現出鍋的熱炊飯最頂飽！這【{item_name}】每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，要帶一顆路上墊肚子嗎？」"
                    ),
                },
                "wt_vendor_soy_milk" => match now_idx % 3 {
                    0 => format!(
                        "「來碗濃淳溫熱的米漿潤潤嗓！老堰頭石磨磨出來的【{item_name}】，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，配油條最是享受！」"
                    ),
                    1 => format!(
                        "「炒花生香氣全融進糙米漿裡了，香甜滑順，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，大清早喝一碗神清氣爽！」"
                    ),
                    _ => format!(
                        "「聞香就知是好米漿！這【{item_name}】正在銅鍋文火溫熱著，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，買一碗暖胃吧！」"
                    ),
                },
                "wt_vendor_morning_produce" => match now_idx % 3 {
                    0 => format!(
                        "「瞧瞧！透早從堰頭清泉畔採下的【{item_name}】，脆嫩帶露，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，清炒或煮湯都甜得很！」"
                    ),
                    1 => format!(
                        "「自己地裡引山泉種的水芹菜，沒噴半點藥，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，抓一把回去加菜最健康！」"
                    ),
                    _ => format!(
                        "「透早新拔的鮮蔬最嫩口！這【{item_name}】梗脆葉嫩，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，隨便挑隨便選，包你滿意！」"
                    ),
                },
                "wt_vendor_morning_fish" => match now_idx % 3 {
                    0 => format!(
                        "「看過來！天未亮在水尾寮淺澤網到的【{item_name}】，隻隻青殼鮮活跳動，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，白灼鮮甜無比！」"
                    ),
                    1 => format!(
                        "「活水養出來的河蝦最青！蝦殼薄肉質脆，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，買回去加點薑絲炒一下，下酒最對味！」"
                    ),
                    _ => format!(
                        "「這盆河蝦生猛得很，水花直濺！每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，數量有限，晚來就只能看空盆囉！」"
                    ),
                },
                _ => match now_idx % 3 {
                    0 => format!(
                        "「走過路過別錯過！這【{item_name}】現貨新鮮，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，來嚐嚐看吧！」"
                    ),
                    1 => format!(
                        "「來看看喔，真材實料的【{item_name}】，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，保證新鮮！」"
                    ),
                    _ => format!(
                        "「熱騰騰現備齊的【{item_name}】，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，要帶一份嗎？」"
                    ),
                },
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
    // 3. 依標準優先順序檢查身上持有的可交割物件
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

