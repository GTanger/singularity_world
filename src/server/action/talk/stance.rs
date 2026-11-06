#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerTalkIntent {
    GreetingOrSocial,
    TimeOrEnvironment,
    LocalLore,
    TradeOrErrand,
    Mentorship,
}

pub fn classify_player_talk_intent(player_input: &str) -> PlayerTalkIntent {
    let t = player_input.trim();
    if t.is_empty() {
        return PlayerTalkIntent::GreetingOrSocial;
    }
    match t {
        "寒暄問候" | "（搭話）" => return PlayerTalkIntent::GreetingOrSocial,
        "打聽近況" => return PlayerTalkIntent::LocalLore,
        "請教差事" => return PlayerTalkIntent::TradeOrErrand,
        _ => {}
    }

    const MENTORSHIP_KEYWORDS: &[&str] = &[
        "請益", "指點", "拜師", "學藝", "傳授", "功法", "手藝",
    ];
    if MENTORSHIP_KEYWORDS.iter().any(|kw| t.contains(kw)) {
        return PlayerTalkIntent::Mentorship;
    }

    const TIME_ENV_KEYWORDS: &[&str] = &[
        "幾點", "天還沒黑", "天還沒亮", "天黑", "天亮", "早晚", "天氣", "哪裡", "在哪", "怎麼走",
    ];
    if TIME_ENV_KEYWORDS.iter().any(|kw| t.contains(kw)) {
        return PlayerTalkIntent::TimeOrEnvironment;
    }

    const TRADE_ERRAND_KEYWORDS: &[&str] = &[
        "買", "賣", "換", "交易", "價錢", "多少鎂", "差事", "委託", "活計", "工作", "任務",
        "幫忙", "缺什麼", "補貨", "收購",
    ];
    if TRADE_ERRAND_KEYWORDS.iter().any(|kw| t.contains(kw)) {
        return PlayerTalkIntent::TradeOrErrand;
    }

    PlayerTalkIntent::GreetingOrSocial
}

pub fn is_trade_or_errand_talk_topic(player_input: &str) -> bool {
    matches!(
        classify_player_talk_intent(player_input),
        PlayerTalkIntent::TradeOrErrand
    )
}

pub fn has_recent_barter_or_obligation_mention(
    recent_turns: &[(String, String)],
    obligation_descs: &[String],
) -> bool {
    const GENERIC_BARTER_TOKENS: &[&str] = &[
        "補貨", "庫存", "見底", "沒東西賣", "缺", "換點", "換份", "換些", "補倉", "差事", "收購", "交割",
    ];
    for (_, n_rep) in recent_turns {
        if GENERIC_BARTER_TOKENS.iter().any(|kw| n_rep.contains(kw)) {
            return true;
        }
        for desc in obligation_descs {
            let core = desc
                .split('（')
                .next()
                .unwrap_or(desc)
                .split('：')
                .last()
                .unwrap_or(desc)
                .trim();
            if core.chars().count() >= 2 && n_rep.contains(core) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
pub fn strip_repeated_barter_tail_simple(reply: &str) -> String {
    strip_repeated_barter_tail(reply, &[], &[])
}

pub fn strip_repeated_barter_tail(
    reply: &str,
    recent_turns: &[(String, String)],
    obligation_descs: &[String],
) -> String {
    let raw = reply.trim();
    if raw.is_empty() {
        return String::new();
    }
    let has_close_quote = raw.ends_with('」');

    let mut prior_ngrams: Vec<String> = Vec::new();
    for (_, prior_reply) in recent_turns {
        let spoken_part = if let Some(q_idx) = prior_reply.find('「') {
            &prior_reply[q_idx + '「'.len_utf8()..]
        } else {
            prior_reply.as_str()
        };
        let clean = spoken_part.trim_matches(|c| c == '「' || c == '」' || c == ' ' || c == '\n');
        let chars: Vec<char> = clean.chars().collect();
        if chars.len() >= 6 {
            for w in chars.windows(6) {
                let s: String = w.iter().collect();
                if !s.contains(|c| matches!(c, '，' | '。' | '！' | '？' | '；' | ' ' | '\n'))
                    && !prior_ngrams.contains(&s)
                {
                    prior_ngrams.push(s);
                }
            }
        }
    }
    let mut ob_cores: Vec<String> = Vec::new();
    for desc in obligation_descs {
        let core = desc
            .split('（')
            .next()
            .unwrap_or(desc)
            .split('：')
            .last()
            .unwrap_or(desc)
            .trim();
        if core.chars().count() >= 2 && !ob_cores.contains(&core.to_string()) {
            ob_cores.push(core.to_string());
        }
    }

    const PITCH_CONNECTIVE_PREFIXES: &[&str] = &[
        "只是我得",
        "不過我得",
        "但我得",
        "倒是你有",
        "手邊有沒有",
        "拿來跟我換",
    ];

    const GENERAL_BARTER_TERMS: &[&str] = &[
        "補貨", "沒東西賣", "庫存見底", "換點", "換份", "換些", "拿來跟我換", "手頭正缺", "趕快去補",
    ];

    const CHORE_REPEAT_MARKERS: &[&str] = &[
        "得先去", "正要去", "得趕去", "還得去", "正打算去", "手頭還有", "手邊這批", "手邊燈罩",
    ];

    let chars: Vec<char> = raw.chars().collect();
    let open_quote_idx = chars.iter().position(|&c| c == '「');

    for (i, &ch) in chars.iter().enumerate() {
        if matches!(ch, '，' | '。' | '！' | '？' | '；') {
            let spoken_slice = if let Some(oq) = open_quote_idx {
                if i <= oq {
                    continue;
                }
                &chars[oq + 1..i]
            } else {
                &chars[..i]
            };
            let spoken_prefix_len = spoken_slice
                .iter()
                .filter(|&&c| c != '「' && c != '」' && c != ' ')
                .count();
            if spoken_prefix_len < 4 {
                continue;
            }
            let prefix_chars = &chars[..i];
            let suffix_chars = &chars[i + 1..];
            let suffix_str: String = suffix_chars.iter().collect();
            let clean_suffix = suffix_str.trim_matches(|c| c == '」' || c == ' ' || c == '\n');
            if clean_suffix.is_empty() {
                continue;
            }

            let is_sentence_boundary = matches!(ch, '。' | '！' | '？');
            let starts_pitch_conn = PITCH_CONNECTIVE_PREFIXES
                .iter()
                .any(|p| clean_suffix.starts_with(p));
            let has_barter_term = GENERAL_BARTER_TERMS.iter().any(|t| clean_suffix.contains(t));
            let has_ob_core = ob_cores.iter().any(|ob| clean_suffix.contains(ob));
            let has_chore_marker = CHORE_REPEAT_MARKERS.iter().any(|m| clean_suffix.contains(m));

            let mut is_tail_to_drop = false;
            if starts_pitch_conn
                || has_barter_term
                || (clean_suffix.contains("你有")
                    && clean_suffix.contains("嗎")
                    && clean_suffix.contains("換"))
            {
                if is_sentence_boundary || spoken_prefix_len >= 8 {
                    is_tail_to_drop = true;
                }
            }
            // 義務與動態 6-gram 僅允許在完整句號／驚嘆號／問號之後切掉獨立的第二句差事複讀，絕不在逗號（，）處攔腰截斷單句！
            if !is_tail_to_drop && is_sentence_boundary {
                if has_ob_core {
                    is_tail_to_drop = true;
                } else if has_chore_marker && !prior_ngrams.is_empty() {
                    for ng in &prior_ngrams {
                        if clean_suffix.contains(ng) {
                            is_tail_to_drop = true;
                            break;
                        }
                    }
                }
            }

            if is_tail_to_drop {
                let mut prefix_str: String = prefix_chars.iter().collect();
                prefix_str = prefix_str.trim().to_string();
                let end_punct = if matches!(ch, '！' | '？' | '。') {
                    ch
                } else {
                    '。'
                };
                if !prefix_str.ends_with(['。', '！', '？']) {
                    prefix_str.push(end_punct);
                }
                if has_close_quote && !prefix_str.ends_with('」') {
                    prefix_str.push('」');
                }
                return prefix_str;
            }
        }
    }
    raw.to_string()
}

pub(super) fn talk_sensitivity_hint(target: &Character) -> String {
    let Some(seed) = target.soul_seed else {
        return String::new();
    };
    let p = db::expand_soul_seed_to_personality(seed);
    let mut s = String::new();
    if p.sensitivity > 0.6 {
        s.push_str("此角色較熱絡，可多說一兩句。");
    } else if p.sensitivity < 0.3 {
        s.push_str("此角色較冷淡，回覆簡短。");
    }
    s
}
pub fn effective_npc_disposition_and_trust(npc_id: &str, pid: &str) -> (i32, i32) {
    let base_disp = get_disposition(npc_id);
    let base_trust = crate::store::get_store()
        .and_then(|arc| arc.read().get_npc_memory(npc_id, pid))
        .map(|m| m.favorability)
        .unwrap_or(0);
    if let Some(pool) = crate::pg::pool() {
        if !npc_id.trim().is_empty() && !pid.trim().is_empty() {
            if let Ok(rel) = crate::pg::living_world::get_relationship_summary(&pool, npc_id, pid) {
                let delta = rel.balance_score * 5 + rel.familiarity_score * 3;
                return (base_disp + delta, base_trust + delta);
            }
        }
    }
    (base_disp, base_trust)
}

pub(super) fn talk_disposition_hint(target_id: &str, pid: &str) -> String {
    let (disp, _) = effective_npc_disposition_and_trust(target_id, pid);
    if disp < -30 {
        "此角色心境低落，語氣較冷漠沉鬱。".into()
    } else if disp > 30 {
        "此角色心情愉快，語氣較活潑熱情。".into()
    } else {
        String::new()
    }
}

/// 談話與市集負向禁用詞清單（包含短市、小市等歷史名詞）。
#[cfg(test)]
pub const FORBIDDEN_WORDS: &[&str] = &[
    "短市", "小市", "還有", "\u{525b}\u{624d}", "\u{525b}\u{525b}", "\u{65b9}\u{624d}", "頃刻", "青籠關", "110V", "念場", "富態", "資料庫", "database", "grid", "debug", "scene_stub", "聖塔",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NegotiationStance {
    Accept,
    Bargain,
    Refuse,
    Deflect,
}

impl From<NegotiationStance> for NpcStanceChoice {
    fn from(s: NegotiationStance) -> Self {
        match s {
            NegotiationStance::Accept => NpcStanceChoice::Accept,
            NegotiationStance::Bargain => NpcStanceChoice::Bargain,
            NegotiationStance::Refuse => NpcStanceChoice::Refuse,
            NegotiationStance::Deflect => NpcStanceChoice::Deflect,
        }
    }
}
fn clean_catalog_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 3 <= chars.len() {
            let prefix: String = chars[i..i + 3].iter().collect();
            if prefix == "WT-" || prefix == "WS-" {
                let mut j = i + 3;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                if j > i + 3 {
                    while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
                        j += 1;
                    }
                    i = j;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

pub fn evaluate_npc_negotiation_stance(
    pid: &str,
    npc_id: &str,
    room_name: &str,
    inv: &crate::db::inventory::InventorySnapshot,
) -> (NegotiationStance, String) {
    let active_debt = crate::npc::debt_interaction::active_debt_amount(pid);
    let (eff_disp, eff_trust) = effective_npc_disposition_and_trust(npc_id, pid);
    let scarce_opt = crate::game::object_ecology::scarcest_item_for_settlement(room_name);

    if eff_trust < -15 || eff_disp < -25 || active_debt > 80 {
        return (
            NegotiationStance::Refuse,
            "【交涉立場：冷淡婉拒】對方對未清舊欠或過往嫌隙心存芥蒂，語氣冷淡防備。".to_string(),
        );
    }

    let carries_scarce = if let Some((scarce_id, _, _, _, _)) = &scarce_opt {
        inv.items.iter().any(|it| &it.item_id == scarce_id && it.qty > 0)
    } else {
        false
    };

    if carries_scarce || eff_trust >= 20 || eff_disp >= 30 {
        return (
            NegotiationStance::Accept,
            "【交涉立場：熱絡接納】對方見你帶有本地急需物產或素有交情，樂意深談並給出實惠條件。".to_string(),
        );
    }

    let active_pack = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
        room_name,
        None,
        crate::game::now_unix(),
    );
    let has_script = active_pack.is_some();

    if scarce_opt.is_some() || has_script {
        let scarce_name = if let Some((_, name, _, _, _)) = &scarce_opt {
            name.clone()
        } else if let Some((ev_pack, _)) = &active_pack {
            let sanitized = clean_catalog_codes(ev_pack.key_objects);
            sanitized
                .split('、')
                .next()
                .unwrap_or("周轉物資")
                .trim()
                .to_string()
        } else {
            "周轉物資".to_string()
        };
        return (
            NegotiationStance::Bargain,
            format!(
                "【交涉立場：條件交換】對方正為本地緊缺的「{}」傷神，願以消息或折讓換取你協助籌措。",
                scarce_name
            ),
        );
    }

    (
        NegotiationStance::Deflect,
        "【交涉立場：客套轉介】對方手頭活計繁忙，僅作簡短寒暄並指引你往市集或行會探問。".to_string(),
    )
}

/// 構建玩家隨身攜帶物料的市場脈絡：查詢本地市集即時庫存與浮動價格，注入 NPC 對話脈絡中。
pub fn build_player_material_context(inv: &crate::db::inventory::InventorySnapshot, room_name: &str) -> String {
    let mut carried_info = Vec::new();
    for it in &inv.items {
        let origin_str = if !it.origin.is_empty() {
            it.origin.clone()
        } else {
            crate::game::object_ecology::lookup_item_origin(&it.item_id)
        };
        let name = if !it.name.is_empty() {
            &it.name
        } else {
            &it.item_id
        };
        let base_price = crate::world::materials::trade_price_anchor_mg(&it.item_id)
            .or_else(|| {
                for fg in &crate::game::object_ecology::STANDARD_FINISHED_GOODS {
                    if fg.id == it.item_id {
                        return Some(fg.base_price);
                    }
                }
                None
            })
            .unwrap_or(5)
            .max(2);
        let quote = crate::game::object_ecology::dynamic_settlement_market_quote(room_name, &it.item_id, base_price);
        carried_info.push(format!(
            "「{}」（產自【{}】，行情每份 {} 鎂，{}）",
            name,
            if origin_str.is_empty() { "外地" } else { &origin_str },
            quote.unit_price,
            quote.status_label
        ));
        if carried_info.len() >= 3 {
            break;
        }
    }
    let mut out = if !carried_info.is_empty() {
        format!("\n【行囊物產：對方隨身攜有{}】", carried_info.join("、"))
    } else {
        String::new()
    };
    if let Some((_, scarce_name, _, _, _)) =
        crate::game::object_ecology::scarcest_item_for_settlement(room_name)
    {
        out.push_str(&format!("\n【本地緊缺物產：目前坊間最缺「{}」（奇缺待補）】", scarce_name));
    }
    out
}

fn synthesize_talk_reply_with_stance_and_obligation(
    base_line: &str,
    stance: NpcStanceChoice,
    obligation: Option<&(String, String)>,
) -> String {
    if obligation.is_none() && stance == NpcStanceChoice::Accept {
        return base_line.to_string();
    }
    if let Some((kind, subj)) = obligation {
        match kind.as_str() {
            "restock_goods" => match stance {
                NpcStanceChoice::Refuse => {
                    "「實不相瞞，店裡現貨正巧斷檔見底，我正愁著去總商行提領新貨補倉，眼下實在抽不開身，還請見諒。」".to_string()
                }
                NpcStanceChoice::Deflect => {
                    "「鋪子裡眼下正為提貨補倉的事抽不開身，你提的事先緩一下，等貨運齊備了再來好好聊。」".to_string()
                }
                NpcStanceChoice::Bargain => {
                    "「店裡現貨正等著去提領，你若願替我搭手跑趟腿或打點清單，我便能早些抽身與你相商。」".to_string()
                }
                NpcStanceChoice::Accept => {
                    let mut s = base_line.to_string();
                    if s.ends_with('」') {
                        s.pop();
                    }
                    format!("{s}……只不過店內現貨已然售罄，我正準備往商行補倉，你要提貨恐怕得稍等片刻。」")
                }
            },
            "repair_tool" => match stance {
                NpcStanceChoice::Refuse => {
                    "「手頭工具刃口崩磨得厲害，不進鐵坊重淬根本使不上力，眼下委實難以接下委任。」".to_string()
                }
                NpcStanceChoice::Deflect => {
                    "「待我先把手頭磨損的傢伙什送去鐵坊整飭好，回頭得空再來與你細商。」".to_string()
                }
                NpcStanceChoice::Bargain => {
                    "「手頭傢伙正需送修，你若有熟識的鐵坊或能幫忙捎過去，我便能早些抽身與你相商。」".to_string()
                }
                NpcStanceChoice::Accept => {
                    let mut s = base_line.to_string();
                    if s.ends_with('」') {
                        s.pop();
                    }
                    format!("{s}……不過手頭傢伙磨損待修，待我稍後磨妥了便能動工。」")
                }
            },
            "treat_injury" => match stance {
                NpcStanceChoice::Refuse => {
                    "「連日勞作氣息微滯，胸口悶得緊，眼下急需前往診所調息，實在難以相陪。」".to_string()
                }
                NpcStanceChoice::Deflect => {
                    "「身子骨略感不適，待我去溫氏脈絡診所討兩劑舒筋活血的方子，改日再來敘話。」".to_string()
                }
                NpcStanceChoice::Bargain => {
                    "「我氣息有些不暢正需調理，你若能幫忙照看一下手頭物件，彼此好商量。」".to_string()
                }
                NpcStanceChoice::Accept => {
                    let mut s = base_line.to_string();
                    if s.ends_with('」') {
                        s.pop();
                    }
                    format!("{s}……稍後我還得赴診所調息一番，我們長話短說。」")
                }
            },
            "repay_debt" => match stance {
                NpcStanceChoice::Refuse => {
                    "「手頭票單帳目催得緊，正急著赴總商行核銷，此刻委實分身乏術。」".to_string()
                }
                NpcStanceChoice::Deflect => {
                    "「眼前帳務繁雜正需理清，待我將商行票單對齊了，再來與你細談。」".to_string()
                }
                NpcStanceChoice::Bargain => {
                    "「這批貨單帳目正待核驗，你若精通核算願搭手對對帳，我會算報酬給你。」".to_string()
                }
                NpcStanceChoice::Accept => {
                    let mut s = base_line.to_string();
                    if s.ends_with('」') {
                        s.pop();
                    }
                    format!("{s}……待我將手頭這筆票單核銷完畢，便能全心相應。」")
                }
            },
            _ => {
                let note = if !subj.is_empty() { subj.as_str() } else { "手頭要事" };
                match stance {
                    NpcStanceChoice::Refuse => {
                        format!("「眼下我正為【{note}】分身乏術，實在難以答應你，還請包涵。」")
                    }
                    NpcStanceChoice::Deflect => {
                        format!("「我這頭【{note}】尚未了結，你的事晚點再聊。」")
                    }
                    NpcStanceChoice::Bargain => {
                        format!("「我正忙著【{note}】，你若願搭手分擔，凡事皆可商量。」")
                    }
                    NpcStanceChoice::Accept => {
                        let mut s = base_line.to_string();
                        if s.ends_with('」') {
                            s.pop();
                        }
                        format!("{s}……不過手頭尚有【{note}】待辦，我們且先說要緊處。」")
                    }
                }
            }
        }
    } else {
        match stance {
            NpcStanceChoice::Refuse => {
                "「今日事情繁雜，你這番心思我心領了，只是眼下實在抽不開身，還請見諒。」".to_string()
            }
            NpcStanceChoice::Deflect => {
                "「你說的事晚點再聊。眼下手頭瑣碎甚多，不如先坐著喝杯茶，改天再細說。」".to_string()
            }
            NpcStanceChoice::Bargain => {
                let mut s = base_line.to_string();
                if s.ends_with('」') {
                    s.pop();
                }
                format!("{s}……不過凡事講求互相，你若能在手頭活計上幫忙搭把手，那就另當別論了。」")
            }
            NpcStanceChoice::Accept => base_line.to_string(),
        }
    }
}

fn extract_venue_and_role_talk_context(target: &Character, room_name: &str) -> String {
    let mut parts = Vec::new();

    if let Some(party) = crate::store::LIVE_HUNTING_PARTIES
        .read()
        .values()
        .find(|p| p.member_ids.contains(&target.id))
        .cloned()
    {
        let phase_desc = match party.phase {
            crate::store::HuntingPartyPhase::Rallying => "正於獵場前哨吹響山豬牙號角招攬好手集結",
            crate::store::HuntingPartyPhase::SurroundingPrey => "正引獵犬分從八方包抄合圍獵物",
            crate::store::HuntingPartyPhase::TakedownAndSplit => "正與隊友收網合擊獵物並準備依見者有份古規分鹿割皮",
            crate::store::HuntingPartyPhase::ReturningToMarket => "正滿載獵獲結伴趕往城鎮商行過磅交割分成",
        };
        let (px, py) = party.prey_coord;
        let prey_place = crate::game::CANONICAL_HUNTING_ZONES
            .get(crate::game::nearest_hunting_zone_idx(px, py))
            .map(|z| z.0.to_string())
            .unwrap_or_else(|| {
                let (z, s, _) = crate::grid::r#gen::classify_wilderness_cell(px, py);
                format!("{z}・{s}")
            });
        parts.push(format!(
            "你隸屬於領隊【{}】率領之【{}】合社獵團（全隊共 {} 人），當前{}，鎖定【{prey_place}】一帶之山海生靈【{}】（預計採獲【{}】運往【{}】交割）。歡迎來客應募入團並肩圍獵、見者有份。",
            party.leader_name,
            party.zone_name,
            party.member_ids.len(),
            phase_desc,
            party.prey_name,
            party.prey_loot_name,
            party.market_name
        ));
    } else if target.id.starts_with("wt_vendor_") {
        let (item_id, item_name, base_price_mg, stock) =
            crate::server::action::trade::get_vendor_stall_stock(&target.id)
                .unwrap_or(("", "招牌小吃", 3, 5));
        let quote = crate::game::object_ecology::dynamic_settlement_market_quote(
            room_name,
            item_id,
            base_price_mg,
        );
        let stock_qual = if stock <= 1 { "所剩無幾" } else { "尚有現貨" };
        parts.push(format!(
            "你是市井攤商，當前攤頭主要販售【{}】（標價每份 {} 鎂幣，行情為{}，攤上{}）。你當前的攤位動態為：{}。",
            item_name, quote.unit_price, quote.status_label, stock_qual, target.current_activity
        ));
    } else if room_name.contains("工坊")
        || room_name.contains("鐵坊")
        || room_name.contains("木坊")
        || target.display_title.contains("工匠")
        || target.display_title.contains("鐵匠")
    {
        parts.push("工坊內爐火常明，日常急需礦砂、熟鐵與木料等原料以備煉製加工，並對外出售各類防護行裝與農工器物。".to_string());
    } else if room_name.contains("藥房")
        || room_name.contains("醫")
        || target.display_title.contains("藥師")
    {
        parts.push("藥房長年熬煮溫脈舒筋油膏與百草清心茶包，常年收購各類山澤草藥，對外供應行旅急救與調養之物。".to_string());
    } else if room_name.contains("藏書")
        || room_name.contains("學院")
        || room_name.contains("印行")
        || target.display_title.contains("書生")
        || target.display_title.contains("學士")
    {
        parts.push("館內典藏半島山川水文誌與考工手札，常年收購精製竹麻紙與松煙墨，並供往來學者查考地誌。".to_string());
    } else if room_name.contains("商行")
        || room_name.contains("商會")
        || target.display_title.contains("掌事")
        || target.display_title.contains("管事")
    {
        parts.push("商行綜理半島各埠貨物流通與鎂幣兌換，時刻關注各類原料缺貨與市集價格浮動動向。".to_string());
    } else {
        let work_place = crate::store::wenyao_household_slot_for(&target.id).work_name;
        parts.push(format!(
            "你平時在【{}】任事（職務為【{}】），此刻身在【{}】，當前手頭動靜：{}。",
            work_place, target.display_title, room_name, target.current_activity
        ));
    }

    let _ = &target.id;

    parts.join(" ")
}
/// 規範化 NPC 對白內容，去除任何殘存的非正典材質（牛角燈）與幽靈店鋪／非現行街巷名。
pub fn enforce_strict_third_person_and_no_parens(s: &str) -> String {
    let mut out = s.to_string();
    out = out.replace("牛角防風燈罩", "夜光貝母防風燈罩");
    out = out.replace("牛角明瓦罩", "夜光貝母防風燈罩");
    out = out.replace("牛角燈罩", "夜光貝母防風燈罩");
    out = out.replace("牛角燈具", "夜光貝母防風燈");
    out = out.replace("牛角燈", "夜光貝母防風燈");
    out = out.replace("燈籠鋪", "銅錫器皿坊");
    out = out.replace("燈具店", "銅錫器皿坊");
    out = out.replace("聯運站前街", "三水渡埠街");
    out = out.replace("西門東街", "三水口西街");
    out = out.replace("北津石板拱橋", "講堂街南口");
    out = out.replace("西虹石板拱橋", "西門聯運東口");
    out = out.replace("講堂街西口", "三水東街");
    out = out.replace("學院前街", "東橋石板拱橋");
    out = out.replace("公議堂前街", "公議堂東街");
    out
}


pub fn format_npc_talk_line(target: &Character, talk_name: &str, reply: &str, player_input: &str, pid: &str) -> String {
    let clean_name = if talk_name.is_empty() || talk_name == "陌生人" {
        if !target.display_title.is_empty() {
            target
                .display_title
                .split(['|', '｜'])
                .next()
                .unwrap_or(&target.display_title)
                .trim()
        } else {
            "市井居民"
        }
    } else {
        talk_name
    };

    let reply_normalized = enforce_strict_third_person_and_no_parens(reply);
    let reply_trimmed = reply_normalized.trim();

    // 若回覆已包含動作神態與「...」對白
    if reply_trimmed.contains('「') && !reply_trimmed.starts_with('「') {
        if reply_trimmed.starts_with(&format!("【{clean_name}】")) {
            return enforce_strict_third_person_and_no_parens(reply_trimmed);
        }
        if let Some(rest) = reply_trimmed.strip_prefix(clean_name) {
            return enforce_strict_third_person_and_no_parens(&format!("【{clean_name}】{rest}"));
        }
        return enforce_strict_third_person_and_no_parens(&format!("【{clean_name}】{reply_trimmed}"));
    }

    // 提取對白文字
    let raw_quote = if reply_trimmed.starts_with('「') && reply_trimmed.ends_with('」') && reply_trimmed.len() >= 6 {
        &reply_trimmed[3..reply_trimmed.len() - 3]
    } else {
        reply_trimmed
    };

    // 若對白開頭已帶有（動作神態）或 (動作神態)，直接提取作為外層肢體敘事，避免內外雙重動作重複
    let trimmed_q = raw_quote.trim();
    if let Some(after_open) = trimmed_q.strip_prefix('（') {
        if let Some(close_idx) = after_open.find('）') {
            let inner_act = after_open[..close_idx].trim().trim_end_matches(['，', '。', '：']);
            let spoken_rest = after_open[close_idx + '）'.len_utf8()..].trim();
            if !inner_act.is_empty() && !spoken_rest.is_empty() {
                return format!("【{clean_name}】{inner_act}，開口道：「{spoken_rest}」");
            }
        }
    }
    if let Some(q_open) = trimmed_q.find('『') {
        if trimmed_q.ends_with('』') && q_open > 0 {
            let inner_act = trimmed_q[..q_open].trim().trim_end_matches(['：', ':', '，', ' ']);
            let spoken_rest = trimmed_q[q_open + '『'.len_utf8()..trimmed_q.len() - '』'.len_utf8()].trim();
            if !inner_act.is_empty() && !spoken_rest.is_empty() {
                return format!("【{clean_name}】{inner_act}：「{spoken_rest}」");
            }
        }
    }
    let quote_text = trimmed_q;

    let act = &target.current_activity;
    let (disp, _) = effective_npc_disposition_and_trust(&target.id, pid);
    let mut h = (reply.len() as i64).wrapping_mul(31).wrapping_add(player_input.len() as i64 * 17);
    for c in target.id.chars() {
        h = h.wrapping_add(c as i64);
    }
    let seed = crate::game::now_unix().wrapping_add(h);
    let is_cook_or_vendor = target.id.starts_with("wt_vendor_")
        || ["廚", "攤", "食堂", "茶館"].iter().any(|k| target.display_title.contains(k));
    let action_desc = if is_cook_or_vendor && (act.contains("磨刀") || act.contains("廚刀") || act.contains("刃")) {
        let p = [
            "按住腰間的磨刀皮蕩帶，停下腳步抬眼望向你道：",
            "試了試手裡青灰油石的刃面，神色爽利地開口道：",
            "把隨身磨刀工具袋往肩上托了托，轉過身笑道：",
        ];
        p[(seed.unsigned_abs() as usize) % p.len()]
    } else if is_cook_or_vendor && (act.contains("炭火") || act.contains("烤") || act.contains("肉")) {
        "撥了撥炭爐的旺火，擦了擦額邊微汗轉頭道："
    } else if is_cook_or_vendor && (act.contains("麵") || act.contains("湯") || act.contains("鍋")) {
        "掀開滾沸的大湯鍋蓋，白汽蒸騰中側過身笑道："
    } else if is_cook_or_vendor && (act.contains("餅") || act.contains("烤爐")) {
        "用鐵夾翻動著熱氣騰騰的烤爐，拍了拍手上的麵粉道："
    } else if is_cook_or_vendor && (act.contains("沏茶") || act.contains("茶盞") || act.contains("品茗")) {
        "端起微溫的粗陶茶盞啜了一口，神色沉靜地抬眼道："
    } else if is_cook_or_vendor && (act.contains("豆花") || act.contains("米漿") || act.contains("石磨")) {
        "用木勺勻了勻桶裡的熱甜汁，語氣和藹地迎上前道："
    } else if is_cook_or_vendor && (act.contains("菜") || act.contains("農")) {
        "在竹簍裡理了理水靈靈的鮮菜，抖了抖葉上的露水笑道："
    } else if is_cook_or_vendor && (act.contains("蝦") || act.contains("魚") || act.contains("漁網")) {
        "往水盆裡添了勺清水，看著水花撲騰笑吟吟地道："
    } else if act.contains("書") || act.contains("校勘") || act.contains("講義") {
        "從攤開的書冊上移開視線，扶了扶書卷神色溫和道："
    } else if act.contains("鐵") || act.contains("錘") || act.contains("鍛造") {
        "停下手中敲打構件的錘子，擦拭著掌心老繭打量你道："
    } else if act.contains("帳") || act.contains("算盤") || act.contains("核算") {
        "擱下手邊的竹管毛筆，核對了帳目抬頭道："
    } else if ["推車", "趕往", "沿街前往", "動身前往", "前往", "漫步", "穿行", "行走", "運送", "用膳", "用餐", "吃", "飲", "茶", "麵", "湯", "粥", "餅", "休憩", "返回", "踏著夜色", "睡", "歇"].iter().any(|k| act.contains(k)) {
        let p = if act.contains("睡") || act.contains("歇") || act.contains("休憩") {
            [
                "合著衣衫側過身來，略顯倦意地看向你道：",
                "揉了揉微酸的眉心，神色平和地開口道：",
                "在椅榻上稍稍挪正了身子，神情放鬆地看向你道：",
            ]
        } else {
            [
                "稍稍放緩了趕路的步伐，側過身望向你道：",
                "停下腳步抹了一把額角的微汗，和氣地開口道：",
                "整了整隨身攜帶的物件，駐足看向你道：",
            ]
        };
        p[(seed.unsigned_abs() as usize) % p.len()]
    } else if act.contains("巡") || act.contains("衛") {
        "扶了扶腰間佩具，神色嚴肅警醒地注視著你道："
    } else if disp < -20 {
        let pool = [
            "眉頭微蹙，有些心不在焉地擺弄著手頭物料道：",
            "語氣略顯煩躁地停下手中動作，轉過頭沉聲道：",
            "眼神戒備地打量了你片刻，雙手抱臂防備地道：",
            "有些疲憊地按了按眉心，長嘆了一口氣道：",
            "神情緊繃地掃視了一圈周遭，壓低嗓子道：",
        ];
        pool[(seed.unsigned_abs() as usize) % pool.len()]
    } else if disp > 25 {
        let pool = [
            "臉上堆滿熟絡熱切的笑意，熱情地招呼上前道：",
            "眉開眼笑地拍了拍掌心的灰塵，語調輕快地道：",
            "嘴角含著爽朗的笑意，活動了一下微酸的臂膀道：",
            "神清氣爽地挺直腰桿，眼中滿是熱忱地看著你道：",
            "欣然停下手中的活計，神態格外親切地開口道：",
        ];
        pool[(seed.unsigned_abs() as usize) % pool.len()]
    } else {
        let pool = [
            "停下手邊正忙著的活計，抬眼朝你微微頷首道：",
            "拍了拍衣袖上的塵屑，神色溫和從容地看著你道：",
            "將手邊才理好的物料收整入盒，側過身來開口道：",
            "手上的活計未停，只側過身斜睨了你一眼，神情淡定道：",
            "端起桌旁的涼茶潤了潤喉，從容好整以暇地望向你道：",
            "目光在你身上停留了片刻，雙手搭在膝頭平靜地道：",
            "將手頭的零件順手歸位，轉過身沉穩地看著你道：",
            "雙手在擦布上抹了抹，眼神中帶著市井特有的精明笑道：",
            "抬手拂開額前碎髮，迎著微風神情自若地道：",
            "靠在廊柱旁神色平和地望著你，唇角噙著一絲淡笑道：",
            "神情專注地打量著你，若有所思地沉吟片刻道：",
            "神態閒適地點了點頭，語調平和從容地接過話道：",
            "順手理了理衣襟，目光澄澈地看著你開口道：",
            "稍微挪動了一下身位騰出空隙，和氣地看著你道：",
            "從手頭的瑣碎活計中回過神來，禮貌地望向你道：",
        ];
        pool[(seed.unsigned_abs() as usize) % pool.len()]
    };

    let is_free_text = !player_input.trim().is_empty() && player_input.trim() != "（搭話）";
    let raw_out = if is_free_text {
        format!("【{clean_name}】{action_desc}「{quote_text}」")
    } else {
        format!("你走近向【{clean_name}】搭話。【{clean_name}】{action_desc}「{quote_text}」")
    };
    enforce_strict_third_person_and_no_parens(&raw_out)
}
fn format_talk_narrative(target: &Character, talk_name: &str, reply: &str, player_input: &str) -> String {
    format_npc_talk_line(target, talk_name, reply, player_input, "")
}

