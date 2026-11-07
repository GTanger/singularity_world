
fn build_talk_fallback(
    target: &Character,
    talk_name: &str,
    room_name: &str,
    player_input: &str,
    stance: NpcStanceChoice,
) -> (String, String) {
    let name = if !talk_name.is_empty() {
        talk_name.to_string()
    } else {
        display_target_name(target)
    };

    let input = player_input.trim();
    let is_free_text = !input.is_empty() && input != "（搭話）";
    let has_craft_factors = target.equipment_slots.contains("wt_cat_0973")
        || target.equipment_slots.contains("wt_cat_0984")
        || target.equipment_slots.contains("craft_");
    if has_craft_factors {
        if let Some(cog) = build_cognitive_talk_reply(
            target,
            talk_name,
            room_name,
            player_input,
            &target.current_activity,
            stance,
            &[],
        ) {
            return cog;
        }
    }
    let intent_line: Option<String> = if is_free_text && !is_asking_name(input) {
        let is_compliment = ["漂亮", "好看", "帥", "美", "氣質", "厲害", "端莊", "標緻", "可愛", "精神"]
            .iter()
            .any(|k| input.contains(k));
        let is_asking_activity = ["忙什麼", "做什麼", "幹嘛", "這是什麼", "在忙", "在做什麼", "忙啥", "在做啥", "什麼事"]
            .iter()
            .any(|k| input.contains(k));
        let is_asking_errand = ["幫忙", "委託", "工作", "差事", "缺什麼", "手頭活", "活計", "工錢", "任務"]
            .iter()
            .any(|k| input.contains(k));
        let is_greeting = ["你好", "您好", "早安", "午安", "晚安", "嗨", "哈囉", "打擾"]
            .iter()
            .any(|k| input.contains(k));
        let is_asking_time = ["幾點", "天還沒黑", "天還沒亮", "天黑了", "天亮了", "什麼時候", "早還晚"]
            .iter()
            .any(|k| input.contains(k));
        let role_raw = crate::store::npc_persistent_role_label(&target.id, &target.display_title);
        let role = role_raw.split(['|', '｜']).next().unwrap_or(&role_raw).trim();
        let mut stripped_act = String::with_capacity(target.current_activity.len());
        let mut in_bracket = false;
        for ch in target.current_activity.chars() {
            if ch == '【' {
                in_bracket = true;
            } else if ch == '】' {
                in_bracket = false;
            } else if !in_bracket {
                stripped_act.push(ch);
            }
        }
        let mut act_s = stripped_act.as_str();
        if let Some(pos) = act_s.find("（推進目標：") {
            act_s = &act_s[..pos];
        } else if let Some(pos) = act_s.find("(推進目標：") {
            act_s = &act_s[..pos];
        }
        let mut act_clean = act_s.trim()
            .trim_start_matches("正在")
            .trim_start_matches('正')
            .trim();
        let transit_prefixes = [
            "收工之餘，動身前往",
            "收工之餘，正動身前往",
            "踏著夜色返回",
            "動身前往",
            "步行前往",
            "沿街步行",
            "沿街前往",
            "返回",
            "趕往",
            "前往",
        ];
        let mut is_transit = target.current_activity.contains("動身前往")
            || target.current_activity.contains("返回【")
            || target.current_activity.contains("步行前往")
            || target.current_activity.contains("踏著夜色返回")
            || target.current_activity.contains("趕往");
        for tp in transit_prefixes {
            if let Some(rest) = act_clean.strip_prefix(tp) {
                act_clean = rest.trim();
                is_transit = true;
                break;
            }
        }
        if let Some(rest) = act_clean.strip_prefix('，') {
            act_clean = rest.trim();
        }
        if let Some(rest) = act_clean.strip_prefix("準備") {
            act_clean = rest.trim();
        }
        let act_final = act_clean.trim_matches(['。', '，', '、', ' ']);
        let act = if act_final.is_empty() {
            if is_transit {
                "趕路辦點差事"
            } else {
                "打理手邊的事項"
            }
        } else if is_transit && (act_final.contains("前往") || act_final.contains("返回")) {
            "趕路辦點差事"
        } else {
            act_final
        };
        let p = target.soul_seed.map(crate::db::expand_soul_seed_to_personality).unwrap_or(crate::db::Personality {
            boldness: 0.5,
            sensitivity: 0.5,
            orderliness: 0.5,
        });

        if is_compliment {
            match stance {
                NpcStanceChoice::Refuse => {
                    Some("「……突然說這些做什麼？手頭正忙著，沒空同你閒聊。」".to_string())
                }
                NpcStanceChoice::Deflect => {
                    Some(format!("「您太抬舉了，街坊鄰里討生活，哪談得上這些。我正忙著{act}呢。」"))
                }
                NpcStanceChoice::Bargain => {
                    Some(format!("「多謝誇獎。不過我這陣子正忙著{act}，若是真想結識，不妨在手頭活計上幫個忙？」"))
                }
                NpcStanceChoice::Accept => {
                    if p.boldness > 0.6 {
                        Some(format!("「哈哈，多謝誇獎！身為{role}，出門在外手底下的功夫與精氣神總得打理精神些。我正{act}，有空常來坐！」"))
                    } else if p.sensitivity > 0.6 || p.boldness < 0.4 {
                        Some(format!("「您過獎了……平日裡只顧著{act}，倒沒特別留意打扮，聽您這麼說真有些不好意思。」"))
                    } else {
                        Some(format!("「多謝誇讚，您太客氣了。我平日也就是照料好這份{role}的本分，現在正{act}呢。」"))
                    }
                }
            }
        } else if is_asking_activity {
            match stance {
                NpcStanceChoice::Refuse => {
                    Some(format!("「我眼下正{act}，手頭活計容不得分心，請勿打擾。」"))
                }
                NpcStanceChoice::Bargain => {
                    Some(format!("「我正在{act}。身為{role}，手頭的事總是一樁接一樁，你若是有心搭把手，我倒能騰出些空閒細聊。」"))
                }
                NpcStanceChoice::Deflect => {
                    Some(format!("「也就是些例行的活計，正{act}呢。市井瑣事繁雜，也沒什麼特別值得大書特書的。」"))
                }
                NpcStanceChoice::Accept => {
                    Some(format!("「我眼下正{act}呢。身為{role}，日常就得把這些事務打理妥當，你若是好奇，在旁看看無妨。」"))
                }
            }
        } else if is_asking_errand {
            if let Some((kind, subj)) = crate::store::get_active_npc_obligation(&target.id) {
                match kind.as_str() {
                    "restock_goods" => Some(format!("「我這鋪子正缺一批貨料（{subj}），若你能自工坊或商鋪弄些過來，可算幫了大忙了。」")),
                    "repair_tool" => Some(format!("「我手頭器物正待整修（需{subj}），得送去工坊打理，若是有人能代跑一趟就好了。」")),
                    "treat_injury" => Some("「近來身子受了些風寒勞損，正缺些舒筋和血的草藥調理。」".to_string()),
                    "repay_debt" => Some("「手頭票單正趕著去核銷，正愁沒人幫忙跑腿遞送票契呢。」".to_string()),
                    _ => Some(format!("「眼下我正為了（{subj}）奔波，你若能搭把手，回頭少不了你的謝禮。」")),
                }
            } else {
                match stance {
                    NpcStanceChoice::Refuse => Some("「手頭暫時沒什麼要委託外人的差事，你自己四處逛逛吧。」".to_string()),
                    _ => Some("「眼下我手邊的活計尚能應付。不過工坊與商會的告示板上時常有物資收購的差事，你若想接活，不妨去那邊瞧瞧。」".to_string()),
                }
            }
        } else if is_asking_time {
            let (h, m) = crate::store::current_game_hour_and_min();
            let sky_desc = match h {
                0..=2 => "現在還是深夜，天還沒亮、外頭黑得很",
                3..=4 => "現在才剛近破曉前夕，天還沒亮透呢",
                5..=7 => "天剛破曉亮起來沒多久，正是早工時分",
                8..=14 => "現在是大白天，日頭正亮著呢",
                15..=17 => "日頭已經偏西，快到傍晚收工的時候了",
                _ => "現在天已經黑了，外頭都是夜色",
            };
            Some(format!("「眼下大概是 {h:02}:{m:02} 前後，{sky_desc}。我手頭正{act}呢。」"))
        } else if is_greeting {
            let role_intro = if role.ends_with("人") || role.ends_with("徒") || role.ends_with("計") || role.ends_with("家") || role.ends_with("傅") || role.ends_with("事") || role.ends_with("夫") || role.ends_with("生") || role.ends_with("匠") || role.ends_with("手") || role.ends_with("員") || role.ends_with("客") { format!("我是這兒的{role}") } else { format!("我是負責{role}的") };
            Some(format!("「你好！{role_intro}，現在正{act}。有什麼需要幫忙或打聽的嗎？」"))
        } else {
            None
        }
    } else {
        None
    };

    if intent_line.is_none() {
        if let Some(res) = canonical_or_venue_talk_reply(target, talk_name, room_name, player_input, stance, false) {
            return res;
        }
    }

    let line = if let Some(l) = &intent_line {
        l.as_str()
    } else {
        let pool: &[&str] = match stance {
            NpcStanceChoice::Refuse => &[
                "「沒這回事，別再問了。」",
                "「手頭正忙，沒空應付你。」",
                "「路過就路過，少套交情。」",
                "「這事免談，你找別人吧。」",
            ],
            NpcStanceChoice::Bargain => &[
                "「想談可以，先看看你能幫上什麼忙。」",
                "「天下沒有白幫的忙，我們得講究交換。」",
                "「我手頭正巧缺人搭手，你若肯出力，凡事好商量。」",
                "「條件合適的話，倒也不是不能談。」",
            ],
            NpcStanceChoice::Deflect => &[
                "「這陣子風聲緊，有些事還是少打聽為妙。」",
                "「我只管做好手邊的活，其餘的不清楚。」",
                "「天快黑了，早點找地方落腳吧。」",
                "「嗯？這事我可說不準。」",
            ],
            NpcStanceChoice::Accept => &[
                "「你好，有什麼事直說就好。」",
                "「原來是你，近來這邊動靜不少，正好聊聊。」",
                "「有話直說吧，能幫得上的我盡量。」",
                "「這附近我還算熟，你想問什麼？」",
            ],
        };
        let mut h = 0_i32;
        for c in target.id.chars() {
            h += c as i32;
        }
        let seed = game::now_unix()
            .saturating_mul(1000)
            .saturating_add(i64::from(h));
        let mut idx = (seed as usize) % pool.len();
        if let Some(seed) = target.soul_seed {
            let p = db::expand_soul_seed_to_personality(seed);
            let shift = (p.boldness * (pool.len() as f64 / 2.0)) as i32;
            idx = (idx as isize + shift as isize).rem_euclid(pool.len() as isize) as usize;
        }
        pool[idx]
    };

    let active_ob = crate::store::get_active_npc_obligation(&target.id);
    let final_line = if intent_line.is_some() {
        line.to_string()
    } else {
        synthesize_talk_reply_with_stance_and_obligation(line, stance, active_ob.as_ref())
    };
    let narrative = format_talk_narrative(target, &name, &final_line, player_input);
    let npc_reply = if player_input.trim().is_empty() || player_input == "（搭話）" {
        final_line
    } else {
        narrative.clone()
    };
    (narrative, npc_reply)
}

pub(super) fn is_asking_name(input: &str) -> bool {
    const NEEDLES: &[&str] = &[
        "名字",
        "姓名",
        "叫什麼",
        "叫甚麼",
        "叫啥",
        "什麼名",
        "甚麼名",
        "貴姓",
        "高姓",
        "尊姓",
        "大名",
        "名諱",
        "芳名",
        "名號",
        "名氏",
        "怎麼稱呼",
        "如何稱呼",
        "怎稱呼",
        "稱呼你",
        "請問稱呼",
        "稱呼",
        "你是誰",
        "你哪位",
        "你是哪位",
        "是哪位",
        "這位是",
        "請問是誰",
        "何人",
        "哪位",
        "誰呀",
        "誰啊",
        "姓什麼",
        "姓甚麼",
        "姓啥",
        "你姓",
        "認識一下",
        "請教大名",
        "報上名",
        "留個名",
        "怎麼叫",
    ];
    NEEDLES.iter().any(|&n| input.contains(n))
}

pub(super) fn resolve_npc_real_name(target: &Character) -> String {
    let gender_str = match target.gender {
        Some(crate::entity::Gender::F) => Some("F"),
        Some(crate::entity::Gender::M) => Some("M"),
        None => None,
    };
    crate::store::resolve_npc_full_label(&target.id, &target.display_title, gender_str)
}
fn format_obligation_subject_zh(kind: &str, subj: &str) -> Option<String> {
    if kind == "employment" || subj == "employment" {
        return Some("尋覓合適差事與活計".to_string());
    }
    let trimmed_subj = subj.trim();
    let trimmed_subj = trimmed_subj.trim_start_matches("急需").trim();
    if !trimmed_subj.is_empty()
        && !trimmed_subj.is_ascii()
        && !trimmed_subj.starts_with("grid:")
        && !trimmed_subj.starts_with("wt-")
        && !trimmed_subj.starts_with("npc_")
    {
        return Some(trimmed_subj.trim_end_matches('。').to_string());
    }
    if let Some(item_name) = crate::world::materials::material_by_item_id(trimmed_subj)
        .map(|m| m.name.clone())
        .or_else(|| db::get_item_def(trimmed_subj).map(|d| d.name))
    {
        return Some(format!("備齊{}", item_name));
    }
    match kind {
        "restock_goods" => Some("鋪內庫存見底急需提貨補倉".to_string()),
        "repair_tool" | "maintenance_recovery" | "tool_repair" => Some("修整受損隨身器物".to_string()),
        "stranded_cargo" | "market_delivery" | "delivery" => Some("滯留貨件調度交割".to_string()),
        "craft_material" | "material_shortage" | "MaterialShortage" | "supply_shortage" => {
            Some("籌措急需百工料件".to_string())
        }
        "fuel_shortage" => Some("添補爐火白炭與柴薪".to_string()),
        "treat_injury" | "medical_help" | "MedicalHelp" => Some("調養傷勢與備妥草藥".to_string()),
        "repay_debt" | "debt_settlement" => Some("結清手頭往來帳目".to_string()),
        _ => None,
    }
}


fn build_cognitive_talk_reply(
    target: &Character,
    talk_name: &str,
    room_name: &str,
    player_input: &str,
    current_activity: &str,
    stance: NpcStanceChoice,
    obligation_descs: &[String],
) -> Option<(String, String)> {
    if is_asking_name(player_input) {
        return None;
    }
    let trimmed = player_input.trim();
    if is_open_trade_input(trimmed) {
        return None;
    }
    if let Some(party) = crate::store::LIVE_HUNTING_PARTIES
        .read()
        .values()
        .find(|p| p.member_ids.contains(&target.id))
        .cloned()
    {
        let (px, py) = party.prey_coord;
        let prey_place = crate::game::CANONICAL_HUNTING_ZONES
            .get(crate::game::nearest_hunting_zone_idx(px, py))
            .map(|z| z.0.to_string())
            .unwrap_or_else(|| {
                let (z, s, _) = crate::grid::r#gen::classify_wilderness_cell(px, py);
                format!("{z}・{s}")
            });
        let reply = match party.phase {
            crate::store::HuntingPartyPhase::Rallying => format!(
                "「好漢來得正巧！我們領隊【{}】正在號召好手（現已聚齊 {} 人），準備前往【{prey_place}】一帶合圍【{}】。你既開口相詢，便算入我們合社獵團一份子，待獵得【{}】運回【{}】，按古規見者有份！」",
                party.leader_name,
                party.member_ids.len(),
                party.prey_name,
                party.prey_loot_name,
                party.market_name
            ),
            crate::store::HuntingPartyPhase::SurroundingPrey => format!(
                "「噓——放輕腳步！我們【{}】合社獵團共 {} 人已鎖定【{prey_place}】一帶的【{}】，正從八方收攏合圍網。你隨我們一同包抄上去，獵下此獸後【{}】與賞格見者有份！」",
                party.leader_name,
                party.member_ids.len(),
                party.prey_name,
                party.prey_loot_name
            ),
            crate::store::HuntingPartyPhase::TakedownAndSplit => format!(
                "「少廢話，快亮傢伙搭把手！我們正與領隊【{}】在【{prey_place}】一帶合力搏殺【{}】，拿下之後【{}】跟鎂幣紅利當場分你一份！」",
                party.leader_name, party.prey_name, party.prey_loot_name
            ),
            crate::store::HuntingPartyPhase::ReturningToMarket => format!(
                "「哈哈，這趟我們【{}】合社獵團在山裡順利圍下【{}】，割得上好的【{}】，正要趕往【{}】過磅分成！下輪吹角集結時，你記得一同來！」",
                party.leader_name, party.prey_name, party.prey_loot_name, party.market_name
            ),
        };
        let narr = format_npc_talk_line(target, talk_name, &reply, player_input, "");
        return Some((narr, reply));
    }

    let mut clean_act = current_activity
        .trim()
        .trim_start_matches("正在")
        .trim_start_matches('正');
    if let Some(pos) = clean_act.find("，手腳已是熟門熟路地") {
        clean_act = &clean_act[pos + "，手腳已是熟門熟路地".len()..];
    }
    let raw_act_trimmed = clean_act.trim();
    let is_narrative_stage = raw_act_trimmed.chars().count() > 10
        || ["走到", "捧起", "靠在", "站在", "擦去", "步入"]
            .iter()
            .any(|d| raw_act_trimmed.contains(d));
    let act_phrase_owned = if ["安歇", "歇宿", "歇息", "養神", "就寢", "熟睡"]
        .iter()
        .any(|k| raw_act_trimmed.contains(k))
    {
        if raw_act_trimmed.contains("走回") || raw_act_trimmed.contains("返回") || raw_act_trimmed.contains("前往") {
            "收工準備回家安歇".to_string()
        } else {
            "在屋裡歇息養神".to_string()
        }
    } else if raw_act_trimmed.contains("賒碗清湯") || raw_act_trimmed.contains("啃乾糧") {
        "趕路辦點差事".to_string()
    } else if raw_act_trimmed.contains("動身前往")
        || raw_act_trimmed.contains("步行前往")
        || raw_act_trimmed.contains("沿街步行")
        || raw_act_trimmed.contains("前往【")
        || raw_act_trimmed.contains("返回【")
    {
        "趕路辦點差事".to_string()
    } else if raw_act_trimmed.is_empty() {
        "照應手頭差事".to_string()
    } else if is_narrative_stage {
        if ["汲水", "飲水", "冷水", "水囊"].iter().any(|k| raw_act_trimmed.contains(k)) {
            "在街邊飲水歇口氣".to_string()
        } else if ["歇腳", "乘涼", "擦去", "喘口氣"].iter().any(|k| raw_act_trimmed.contains(k)) {
            "在街邊歇口氣".to_string()
        } else if ["工坊", "木架", "打磨", "鍛", "工具"].iter().any(|k| raw_act_trimmed.contains(k)) {
            "打理手頭活計".to_string()
        } else if ["食堂", "茶", "用膳", "熱食"].iter().any(|k| raw_act_trimmed.contains(k)) {
            "用過茶飯歇歇腳".to_string()
        } else if ["攤", "市集", "挑選", "採買", "採辦"].iter().any(|k| raw_act_trimmed.contains(k)) {
            "在坊間採辦日用".to_string()
        } else if ["巡", "守", "哨"].iter().any(|k| raw_act_trimmed.contains(k)) {
            "在街巷巡看".to_string()
        } else {
            "打理手頭日常差事".to_string()
        }
    } else if [
        "街邊", "橋頭", "樹蔭", "石欄", "攤前", "門前", "廊下", "岸邊", "井邊", "爐前", "窗前", "桌邊",
    ]
    .iter()
    .any(|p| raw_act_trimmed.starts_with(p))
    {
        format!("在{raw_act_trimmed}")
    } else {
        raw_act_trimmed.to_string()
    };
    let act_phrase = act_phrase_owned.as_str();

    let ob_opt = crate::store::get_active_npc_obligation(&target.id)
        .and_then(|(kind, subj)| {
            format_obligation_subject_zh(&kind, &subj).map(|desc| (kind, desc))
        })
        .or_else(|| {
            obligation_descs
                .iter()
                .find(|d| !d.is_ascii() && !d.contains("employment"))
                .map(|d| {
                    let clean_d = if let Some(idx) = d.find(':') {
                        let prefix = &d[..idx];
                        if prefix.is_ascii() {
                            d[idx + 1..].trim().to_string()
                        } else {
                            d.clone()
                        }
                    } else {
                        d.clone()
                    };
                    ("obligation".to_string(), clean_d)
                })
        })
        .or_else(|| {
            if let Some(pool) = crate::pg::pool()
                && let Ok(mut client) = pool.get()
                && let Ok(rows) = client.query(
                    "SELECT obligation_kind, subject_ref, COALESCE(created_at, 0), COALESCE(due_at, 0) FROM resident_obligations
                     WHERE entity_id = $1 AND status IN ('open', 'active')
                     ORDER BY COALESCE(due_at, 9223372036854775807) ASC, updated_at DESC LIMIT 1",
                    &[&target.id],
                )
                && let Some(r) = rows.first()
            {
                let kind: String = r.get(0);
                let subj: String = r.get(1);
                let created_at: i64 = r.get(2);
                let due_at: i64 = r.get(3);
                if format_obligation_subject_zh(&kind, &subj).is_none()
                    && subj.chars().any(|c| c.is_ascii_alphabetic())
                {
                    return None;
                }
                let base_desc = format_obligation_subject_zh(&kind, &subj).unwrap_or(subj);
                let now = crate::game::now_unix();
                let time_tag = if created_at > 0 {
                    crate::store::format_short_relative_time(created_at, now)
                } else {
                    String::new()
                };
                let due_suffix = if due_at > now {
                    format!("（距期限尚有{}）", crate::store::format_elapsed_duration_zh(due_at - now))
                } else {
                    String::new()
                };
                let desc = format!("{time_tag}{base_desc}{due_suffix}");
                Some((kind, desc))
            } else {
                None
            }
        });

    let now_ts = crate::game::now_unix();
    let latest_thought = target.thoughts.iter().rev().find_map(|t| {
        let not_expired = t.decay_minutes <= 0 || (now_ts - t.created_at) < t.decay_minutes * 60;
        if not_expired {
            if let Some(f) = &t.flavor {
                let s = f.trim();
                if !s.is_empty()
                    && !s.contains("花用又見底")
                    && !s.contains("又見底，發愁")
                    && !s.contains("完成「")
                    && !s.contains("過後，")
                    && !s.contains("舒緩了許多")
                    && !s.contains("神清氣爽")
                    && !s.contains("枚鎂幣")
                    && !s.contains("打點所需")
                    && !s.contains("花費")
                    && !s.contains("grid:")
                    && !s.is_ascii()
                    && !s.bytes().any(|b| b.is_ascii_alphabetic())
                {
                    return Some(s.replace("文曜・", ""));
                }
            }
        }
        None
    });

    let is_asking_what = [
        "在做甚麼", "在做什麼", "做什麼", "做甚麼", "忙什麼", "忙甚麼", "在幹嘛",
        "在幹甚麼", "在想什麼", "在想甚麼", "想什麼", "想甚麼", "心思", "心事",
        "發愁", "發愣", "發呆",
    ]
    .iter()
    .any(|k| trimmed.contains(k));

    let is_asking_troubles = [
        "困難", "麻煩", "煩惱", "難處", "缺", "需要", "幫忙", "要不要幫",
        "差事", "委託", "所求", "有何為難", "何事為難",
    ]
    .iter()
    .any(|k| trimmed.contains(k));

    let is_asking_news = [
        "傳聞", "聽說", "消息", "動靜", "市況", "八卦", "聽聞", "新聞",
        "動向", "近況", "最近", "風聲", "市面",
    ]
    .iter()
    .any(|k| trimmed.contains(k));

    let is_proposing_trade_help = [
        "收購", "交易", "買賣", "換", "出讓", "盤纏", "代勞", "替你", "交給我",
        "我來幫", "助你", "相助", "幫你", "有什麼賣", "價錢",
    ]
    .iter()
    .any(|k| trimmed.contains(k));

    let is_complaining_repeat = [
        "重複", "一樣的話", "說過了", "聽過了", "別再說", "不要一直",
    ]
    .iter()
    .any(|k| trimmed.contains(k));

    let is_asking_rest = [
        "好眠", "休息", "要歇", "安歇", "睡", "晚安", "早點歇", "養神",
    ]
    .iter()
    .any(|k| trimmed.contains(k));
    let (live_stock_items, live_tool_info, has_worn_tool) = {
        let slots_map: Option<std::collections::HashMap<String, String>> =
            serde_json::from_str(&target.equipment_slots).ok();
        let mut equipped_ids = std::collections::HashSet::new();
        if let Some(m) = &slots_map {
            for val in m.values() {
                if !val.trim().is_empty() {
                    equipped_ids.insert(val.clone());
                }
            }
        }
        let inv = crate::db::get_inventory(&target.inventory, target.vit);
        let items: Vec<String> = inv
            .items
            .iter()
            .filter(|i| {
                i.qty > 0
                    && !equipped_ids.contains(&i.item_id)
                    && !i.item_id.starts_with("crystal_")
                    && (crate::world::author_economy::is_runtime_good_item(&i.item_id)
                        || crate::db::item_is_tradable(&i.item_id)
                        || i.item_id.starts_with("craft_")
                        || i.item_id.starts_with("wt_cat_")
                        || i.item_id.starts_with("mat_")
                        || i.item_id.starts_with("harvest_")
                        || i.item_id.starts_with("ore_")
                        || i.item_id.starts_with("wood_"))
            })
            .take(3)
            .map(|i| {
                let display_name = if i.name == "精碾白米" && (room_name.contains("石磨") || room_name.contains("水輪")) {
                    "待磨精碾白米"
                } else {
                    &i.name
                };
                crate::game::object_ecology::ItemPhysicalClass::classify(&i.item_id, display_name)
                    .format_qty(display_name, i.qty)
            })
            .collect();

        let (t_id, t_name) = {
            let held_id = slots_map.as_ref().and_then(|m| {
                m.get("hold_r").or_else(|| m.get("hold_l")).filter(|s| !s.is_empty())
            });
            let vspec = crate::npc::causal_core::resolve_vocational_vessel(&target.display_title);
            if let Some(tid) = held_id {
                let tname = crate::db::get_item_name(tid);
                let is_vessel = crate::game::object_ecology::ItemPhysicalClass::classify(tid, &tname)
                    .is_vocational_or_combat_vessel();
                if is_vessel {
                    (tid.clone(), tname)
                } else {
                    (vspec.item_id.to_string(), vspec.display_name.to_string())
                }
            } else {
                (vspec.item_id.to_string(), vspec.display_name.to_string())
            }
        };

        let (v_opt, worn) = if !t_id.is_empty() {
            let v = crate::pg::living_world::get_vessel_state(&target.id, &t_id);
            let is_worn = v.wear_points >= 2;
            let cond_str = v.condition.label_for_item(&t_id, &t_name).to_string();
            (Some((t_name, v.wear_points, cond_str)), is_worn)
        } else {
            (None, false)
        };

        (items, v_opt, worn)
    };
    let has_carried_stock = !live_stock_items.is_empty();
    let has_obligation = ob_opt.is_some();

    let line = if (is_asking_what || is_asking_troubles || is_proposing_trade_help)
        && (has_carried_stock || has_worn_tool || has_obligation)
    {
        if is_asking_what {
            match stance {
                NpcStanceChoice::Refuse => {
                    if has_worn_tool {
                        let (t_name, wear, cond) = live_tool_info.as_ref().unwrap();
                        let wear_desc = match *wear {
                            0 => format!("手頭這件【{t_name}】器況{cond}"),
                            1 => format!("手頭這件【{t_name}】已有些許磨耗（器況{cond}）"),
                            _ => format!("手頭這件【{t_name}】磨損頗重（器況{cond}）"),
                        };
                        format!("「我現下{wear_desc}，正忙著料理活計，沒工夫多聊。」")
                    } else if let Some((_, subj)) = &ob_opt {
                        format!("「手頭正忙著{act_phrase}，還得趕緊料理【{subj}】的差事，請自便吧。」")
                    } else {
                        format!("「我正忙著{act_phrase}，隨身這批{}還得照料妥當，沒工夫閒扯。」", live_stock_items.join("、"))
                    }
                }
                _ => {
                    let mut parts = Vec::new();
                    parts.push(format!("我眼下正踏實料理著{act_phrase}"));
                    if !live_stock_items.is_empty() {
                        parts.push(format!("隨身收納有{}", live_stock_items.join("、")));
                    }
                    if let Some((t_name, wear, cond)) = live_tool_info.as_ref() {
                        let wear_desc = match *wear {
                            0 => format!("手頭這件【{t_name}】器況{cond}"),
                            1 => format!("手頭這件【{t_name}】已有些許磨耗（器況{cond}）"),
                            _ => format!("手頭這件【{t_name}】磨損頗重（器況{cond}）"),
                        };
                        if has_worn_tool {
                            parts.push(format!("{wear_desc}，需抽空打磨修整"));
                        } else {
                            parts.push(wear_desc);
                        }
                    }
                    if let Some((_, subj)) = &ob_opt {
                        parts.push(format!("心裡還掛記著【{subj}】的託付"));
                    }
                    format!("「{}。相逢若有何指教，不妨直言。」", parts.join("，"))
                }
            }
        } else if is_asking_troubles {
            match stance {
                NpcStanceChoice::Refuse => {
                    if has_worn_tool {
                        let (t_name, wear, cond) = live_tool_info.as_ref().unwrap();
                        let wear_desc = match *wear {
                            0 => format!("手頭這件【{t_name}】器況{cond}"),
                            1 => format!("手頭這件【{t_name}】已有些許磨耗（器況{cond}）"),
                            _ => format!("手頭這件【{t_name}】磨損頗重（器況{cond}）"),
                        };
                        format!("「{wear_desc}我自己料理便成，不勞外人掛心。」")
                    } else if let Some((_, subj)) = &ob_opt {
                        format!("「【{subj}】是我自己的差事，不便假手於人，請回吧。」")
                    } else {
                        format!("「我手頭活計自會料理，暫無外人可插手之處。」")
                    }
                }
                _ => {
                    if has_worn_tool {
                        let (t_name, wear, cond) = live_tool_info.as_ref().unwrap();
                        let wear_desc = match *wear {
                            0 => format!("手頭這件【{t_name}】器況{cond}"),
                            1 => format!("手頭這件【{t_name}】已有些許磨耗（器況{cond}）"),
                            _ => format!("手頭這件【{t_name}】磨損頗重（器況{cond}）"),
                        };
                        format!(
                            "「要說難處，{wear_desc}，若你得空代為送往鐵坊整修，或是覓得砥石打磨，當真幫了大忙。」"
                        )
                    } else if let Some((_, subj)) = &ob_opt {
                        format!("「實不相瞞，我眼下正為【{subj}】費神。你若肯代為周全跑一趟，酬勞自不會少。」")
                    } else if !live_stock_items.is_empty() {
                        format!("「隨身這批{}正待處置，若你能幫忙尋條銷路或搭把手，便是替我分了憂。」", live_stock_items.join("、"))
                    } else {
                        format!("「眼下踏實照料{act_phrase}，倒無甚大礙，多謝掛心。」")
                    }
                }
            }
        } else {
            // is_proposing_trade_help
            if !live_stock_items.is_empty() {
                format!("「我隨身正收存有{}，你若有意互通有無，照著市面公道規矩交割自是便宜。」", live_stock_items.join("、"))
            } else if has_worn_tool {
                let (t_name, wear, cond) = live_tool_info.as_ref().unwrap();
                let wear_desc = match *wear {
                    0 => format!("手頭這件【{t_name}】器況{cond}"),
                    1 => format!("手頭這件【{t_name}】已有些許磨耗（器況{cond}）"),
                    _ => format!("手頭這件【{t_name}】磨損頗重（器況{cond}）"),
                };
                format!("「我{wear_desc}，若有工坊物材或修整門道，倒真想向你討教一二。」")
            } else if let Some((_, subj)) = &ob_opt {
                format!("「你有心相助那是再好不過，若能在【{subj}】上一展身手，我自當備辦酬謝。」")
            } else {
                format!("「若有合適的物事往來，照市面規矩各取所需便是。」")
            }
        }
    } else if is_asking_what {
        match stance {
            NpcStanceChoice::Accept => {
                if let Some(t) = latest_thought.as_deref() {
                    format!("「我現下正{act_phrase}，心裡正琢磨著『{t}』呢。你若是好奇，倒可同你細說兩句。」")
                } else if let Some((_, subj)) = &ob_opt {
                    format!("「手頭正忙著{act_phrase}，心裡還掛記著【{subj}】這檔差事。你這時過來，也是湊巧。」")
                } else {
                    format!("「我這頭正踏實料理著{act_phrase}，趁著精神頭正好，把該辦的辦妥，心裡便舒坦。」")
                }
            }
            NpcStanceChoice::Refuse => {
                format!("「我現下正專心{act_phrase}，手頭活計緊得很，沒工夫閒扯別的，請自便吧。」")
            }
            NpcStanceChoice::Deflect => {
                format!("「無非是手頭這點{act_phrase}的尋常瑣事，日頭一天天過，哪有那麼多繁雜想頭。」")
            }
            NpcStanceChoice::Bargain => {
                if let Some((_, subj)) = &ob_opt {
                    format!("「我手上正忙著{act_phrase}，若是你能替我搭把手料理【{subj}】，我倒能騰出空同你好好聊聊。」")
                } else {
                    format!("「我正忙著{act_phrase}呢。你若真想探聽，總得先替我跑個腿或者搭把手吧？」")
                }
            }
        }
    } else if is_asking_troubles {
        if let Some((_, subj)) = &ob_opt {
            match stance {
                NpcStanceChoice::Accept => {
                    format!("「實不相瞞，我眼下正為【{subj}】掛心。你若真肯搭把手，那可是解了我的燃眉之急。」")
                }
                NpcStanceChoice::Bargain => {
                    format!("「難處自然是有的，眼下正愁【{subj}】沒人著落。你若能替我分憂，酬勞自不會虧待了你。」")
                }
                NpcStanceChoice::Deflect => {
                    format!("「街坊鄰里討生活，誰手頭沒幾件煩難事？我自己料理這{act_phrase}便成，不敢勞動你。」")
                }
                NpcStanceChoice::Refuse => {
                    format!("「這是我自己的差事，由我自己擔著，不勞外人插手。」")
                }
            }
        } else {
            match stance {
                NpcStanceChoice::Accept => {
                    "「眼下好在手頭這份日常差事還算順暢，你若真有心，往後多照應著點便足夠了。」".to_string()
                }
                NpcStanceChoice::Bargain => {
                    format!("「手頭活計瑣碎，若你能幫著照看一二，我這頭也能輕鬆不少，少不了你的好處。」")
                }
                NpcStanceChoice::Deflect => {
                    format!("「眼下倒也談不上什麼大難處，把這{act_phrase}做紮實就是了。」")
                }
                NpcStanceChoice::Refuse => {
                    format!("「我手腳俐落，沒什麼需要幫忙的，請回吧。」")
                }
            }
        }
    } else if is_asking_news {
        match stance {
            NpcStanceChoice::Accept => {
                if let Some(fact) = crate::npc::causal_core::top_active_ripple_for_npc(&target.id, now_ts)
                    && !fact.prose_summary.trim().is_empty()
                {
                    let clean_sum = fact.prose_summary.trim().trim_end_matches('。');
                    format!("「你問起近況？我前刻才聽人提起——{clean_sum}。這事在我們{room_name}一帶也傳開了。」")
                } else if let Some(rumor) = crate::db::top_npc_rumors(room_name, "", now_ts, 3)
                    .into_iter()
                    .find(|r| !r.text.trim().is_empty())
                {
                    let clean_r = rumor.text.trim().trim_end_matches('。');
                    let time_tag = crate::store::format_short_relative_time(rumor.updated_at, crate::game::now_unix());
                    let clean_r = format!("{time_tag}{clean_r}");
                    format!("「要說這{room_name}一帶的近況，前刻街面上正傳著呢——{clean_r}。」")
                } else if let Some((_, subj)) = &ob_opt {
                    format!("「這{room_name}一帶往來雖熱鬧，可眼下街坊們正為【{subj}】費神呢，市面上這路物事正緊俏。」")
                } else {
                    format!("「這{room_name}一帶今日往來行客絡繹不絕，各家鋪頭與攤位都在忙著手頭營生，倒也安穩熱鬧。」")
                }
            }
            NpcStanceChoice::Bargain => {
                format!("「市面上風吹草動我自然知曉些許，不過這等要緊消息，可不能白白給了人。」")
            }
            NpcStanceChoice::Deflect => {
                format!("「外頭傳聞真真假假的，天天都有新說法，不如踏實做好眼前這{act_phrase}來得心安。」")
            }
            NpcStanceChoice::Refuse => {
                format!("「外頭那些流言蜚語我從不過問，你打聽這些，可真是找錯人了。」")
            }
        }
    } else if is_proposing_trade_help {
        match stance {
            NpcStanceChoice::Accept => {
                if let Some((_, subj)) = &ob_opt {
                    format!("「你有心相助，那是再好不過！若是能幫我周全【{subj}】，當真是幫了大忙。」")
                } else {
                    format!("「有差事或物事可談自然好，我這頭正料理著{act_phrase}，請直說無妨。」")
                }
            }
            NpcStanceChoice::Bargain => {
                format!("「交換講究各取所需，你若能拿出合適的物事或代勞些雜務，我們萬事好商量。」")
            }
            NpcStanceChoice::Deflect => {
                format!("「手頭這{act_phrase}正吃緊，商量物事容後再議，眼下暫且顧不上。」")
            }
            NpcStanceChoice::Refuse => {
                format!("「我這頭自顧不暇，暫無這等心思與盤算，還是另尋高就吧。」")
            }
        }
    } else if is_complaining_repeat {
        format!("「哈哈，是我老惦記著{act_phrase}這點瑣事，話說得絮叨了。你若有別的事想聊，儘管直說便是。」")
    } else if is_asking_rest {
        if act_phrase.contains("安歇") || act_phrase.contains("養神") {
            "「多謝掛心，忙活了一整日，確實正準備回屋安歇養神，明早還得早起開工呢。」".to_string()
        } else {
            format!("「多謝體恤，待我把眼下這點{act_phrase}的收尾活計理順了，便回屋歇息。」")
        }
    } else {
        // 一般搭話或問候
        match stance {
            NpcStanceChoice::Accept => {
                if let Some(t) = latest_thought.as_deref() {
                    let clean_t = t.trim_start_matches("今日").trim_start_matches("前刻");
                    if t.contains("安歇") || t.contains("養神") || t.contains("明日早起") {
                        format!("「今日活計總算告一段落，正{act_phrase}，打算早些歇息養足精神，相逢便是有緣。」")
                    } else {
                        let is_comfort = ["肚裡暖和", "舒坦", "精神", "暖意", "歇過"]
                            .iter()
                            .any(|k| t.contains(k));
                        if is_comfort {
                            format!("「眼下正{act_phrase}，前刻{clean_t}，此時精神正好，相逢便是有緣。」")
                        } else {
                            format!("「眼下正{act_phrase}，心裡正盤算著『{clean_t}』呢，相逢便是有緣。」")
                        }
                    }
                } else if let Some((_, subj)) = &ob_opt {
                    format!("「在這【{room_name}】我正{act_phrase}，手頭還記掛著【{subj}】的差事，得空聊上兩句倒也無妨。」")
                } else {
                    format!("「今日【{room_name}】這頭風色尚可，我正{act_phrase}，得空歇口氣聊上兩句倒也無妨。」")
                }
            }
            NpcStanceChoice::Bargain => {
                if let Some((_, subj)) = &ob_opt {
                    format!("「有禮了。你既來了，可願替我探探【{subj}】的眉目？若有著落定有酬謝。」")
                } else {
                    format!("「有禮了。若想深入結識，往後在本地得多互通有無才是。」")
                }
            }
            NpcStanceChoice::Deflect => {
                if act_phrase.contains("歇")
                    || act_phrase.contains("養神")
                    || act_phrase.contains("睡")
                    || act_phrase.contains("納涼")
                {
                    format!("「我眼下正{act_phrase}，尋常度日罷了。」")
                } else if act_phrase.starts_with("在") || act_phrase.contains("手頭") {
                    format!("「我眼下正{act_phrase}，尋常度日罷了。」")
                } else {
                    format!("「我正忙著手頭{act_phrase}的營生，尋常度日罷了。」")
                }
            }
            NpcStanceChoice::Refuse => {
                format!("「請自便吧，我這頭正{act_phrase}忙碌著，照應不周。」")
            }
        }
    };

    let narrative = format_talk_narrative(target, talk_name, &line, player_input);
    Some((narrative, line))
}
