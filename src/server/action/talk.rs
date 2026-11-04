//! `do_action` — Talk 分支：LLM 對話（走 Ollama 或 OpenAI-compatible），fallback pool。

use crate::ai::{
    call_ai_talk, call_openai_compatible_chat,
    decision_scorer::{NpcDecisionInput, NpcStanceChoice, score_npc_interaction_stance},
    player_npc_talk_build_prompts,
};
use crate::db::{
    self, FAV_TALK, adjust_favorability, get_disposition, get_room_name, pick_style_examples,
    search_archival_for_player_talk,
};
use crate::entity::{Character, EntityKind, Verb};
use crate::event;
use crate::game;

use super::super::conversation::{flush_conversation_and_append, get_recent_turns};
use super::super::handler::{WsConnection, push_refresh};
use super::super::protocol::{ActionResultMsg, ClientMsg};
use super::display_target_name;

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
        parts.push(format!(
            "你平時在【{}】值守活動，職務為【{}】，當前手頭正在忙碌：{}。",
            room_name, target.display_title, target.current_activity
        ));
    }

    if let Some((kind, subj)) = crate::store::get_active_npc_obligation(&target.id) {
        parts.push(format!("手頭牽掛的雜務差事：{}（{}）。", kind, subj));
    }

    parts.join(" ")
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

    let reply_trimmed = reply.trim();

    // 若回覆已包含動作神態與「...」對白
    if reply_trimmed.contains('「') && !reply_trimmed.starts_with('「') {
        if reply_trimmed.starts_with(&format!("【{clean_name}】")) {
            return reply_trimmed.to_string();
        }
        if let Some(rest) = reply_trimmed.strip_prefix(clean_name) {
            return format!("【{clean_name}】{rest}");
        }
        return format!("【{clean_name}】{reply_trimmed}");
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

    let action_desc = if act.contains("磨刀") || act.contains("廚刀") || act.contains("刃") {
        let p = [
            "按住腰間的磨刀皮蕩帶，停下腳步抬眼望向你道：",
            "試了試手裡青灰油石的刃面，神色爽利地開口道：",
            "把隨身磨刀工具袋往肩上托了托，轉過身笑道：",
        ];
        p[(seed.unsigned_abs() as usize) % p.len()]
    } else if act.contains("炭火") || act.contains("烤") || act.contains("肉") {
        "撥了撥炭爐的旺火，擦了擦額邊微汗轉頭道："
    } else if act.contains("麵") || act.contains("湯") || act.contains("鍋") {
        "掀開滾沸的大湯鍋蓋，白汽蒸騰中側過身笑道："
    } else if act.contains("餅") || act.contains("烤爐") {
        "用鐵夾翻動著熱氣騰騰的烤爐，拍了拍手上的麵粉道："
    } else if act.contains("沏茶") || act.contains("茶盞") || act.contains("品茗") {
        "端起微溫的粗陶茶盞啜了一口，神色沉靜地抬眼道："
    } else if act.contains("豆花") || act.contains("米漿") || act.contains("石磨") {
        "用木勺勻了勻桶裡的熱甜汁，語氣和藹地迎上前道："
    } else if act.contains("菜") || act.contains("農") {
        "在竹簍裡理了理水靈靈的鮮菜，抖了抖葉上的露水笑道："
    } else if act.contains("蝦") || act.contains("魚") || act.contains("漁網") {
        "往水盆裡添了勺清水，看著水花撲騰笑吟吟地道："
    } else if act.contains("書") || act.contains("校勘") || act.contains("講義") {
        "從攤開的書冊上移開視線，扶了扶書卷神色溫和道："
    } else if act.contains("鐵") || act.contains("錘") || act.contains("鍛造") {
        "停下手中敲打構件的錘子，擦拭著掌心老繭打量你道："
    } else if act.contains("帳") || act.contains("算盤") || act.contains("核算") {
        "擱下手邊的竹管毛筆，核對了帳目抬頭道："
    } else if act.contains("推車")
        || act.contains("趕往")
        || act.contains("沿街前往")
        || act.contains("動身前往")
        || act.contains("前往")
        || act.contains("漫步")
        || act.contains("穿行")
        || act.contains("行走")
        || act.contains("運送")
        || act.contains("用膳")
        || act.contains("用餐")
        || act.contains("吃")
        || act.contains("飲")
        || act.contains("茶")
        || act.contains("麵")
        || act.contains("湯")
        || act.contains("粥")
        || act.contains("餅")
        || act.contains("休憩")
        || act.contains("返回")
        || act.contains("踏著夜色")
        || act.contains("睡")
        || act.contains("歇")
    {
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
    if is_free_text {
        format!("【{clean_name}】{action_desc}「{quote_text}」")
    } else {
        format!("你走近向【{clean_name}】搭話。【{clean_name}】{action_desc}「{quote_text}」")
    }
}
fn format_talk_narrative(target: &Character, talk_name: &str, reply: &str, player_input: &str) -> String {
    format_npc_talk_line(target, talk_name, reply, player_input, "")
}


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
        } else if is_greeting {
            let role_intro = if role.ends_with("人") || role.ends_with("徒") || role.ends_with("計") || role.ends_with("家") || role.ends_with("傅") || role.ends_with("事") || role.ends_with("夫") || role.ends_with("生") || role.ends_with("匠") || role.ends_with("手") || role.ends_with("員") || role.ends_with("客") { format!("我是這兒的{role}") } else { format!("我是負責{role}的") };
            Some(format!("「你好！{role_intro}，現在正{act}。今天天氣不錯，有什麼需要幫忙或打聽的嗎？」"))
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
    if kind == "employment" {
        return None;
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
    let act_phrase_owned = if raw_act_trimmed.contains("賒碗清湯") || raw_act_trimmed.contains("啃乾糧") {
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
                    "SELECT obligation_kind, subject_ref FROM resident_obligations
                     WHERE entity_id = $1 AND status IN ('open', 'due', 'active')
                     ORDER BY priority DESC, updated_at DESC LIMIT 1",
                    &[&target.id],
                )
                && let Some(r) = rows.first()
            {
                let kind: String = r.get(0);
                let subj: String = r.get(1);
                let desc = format_obligation_subject_zh(&kind, &subj).unwrap_or(subj);
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
                let display_name = if i.name == "精碾白米" {
                    "待磨精碾白米"
                } else {
                    &i.name
                };
                let unit = if display_name.contains("米漿")
                    || display_name.contains("湯")
                    || display_name.contains("飲")
                    || display_name.contains("油")
                    || display_name.contains("水")
                {
                    "桶"
                } else if display_name.contains("粉")
                    || display_name.contains("米")
                    || display_name.contains("麵")
                    || display_name.contains("穀")
                    || display_name.contains("鹽")
                {
                    "袋"
                } else {
                    ""
                };
                if !unit.is_empty() {
                    format!("{display_name} {} {unit}", i.qty)
                } else {
                    format!("{display_name} ×{}", i.qty)
                }
            })
            .collect();

        let (t_id, t_name) = {
            let held_id = slots_map.as_ref().and_then(|m| {
                m.get("hold_r").or_else(|| m.get("hold_l")).filter(|s| !s.is_empty())
            });
            if let Some(tid) = held_id {
                let tname = crate::db::get_item_name(tid);
                (tid.clone(), tname)
            } else {
                let vspec = crate::npc::causal_core::resolve_vocational_vessel(&target.display_title);
                (vspec.item_id.to_string(), vspec.display_name.to_string())
            }
        };

        let (v_opt, worn) = if !t_id.is_empty() {
            let v = crate::pg::living_world::get_vessel_state(&target.id, &t_id);
            let is_worn = v.wear_points >= 2;
            (Some((t_name, v.wear_points, v.condition.label().to_string())), is_worn)
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
                        format!(
                            "「我現下手頭這件【{}】磨損達 {}/3（器況{}），正忙著料理活計，沒工夫多聊。」",
                            t_name, wear, cond
                        )
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
                        if has_worn_tool {
                            parts.push(format!(
                                "手頭這件【{}】磨損已至 {}/3（器況{}），需抽空打磨修整",
                                t_name, wear, cond
                            ));
                        } else {
                            parts.push(format!(
                                "手頭這件【{}】磨損 {}/3（器況{}）",
                                t_name, wear, cond
                            ));
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
                        format!(
                            "「手頭【{}】磨耗 {}/3（器況{}）我自己料理便成，不勞外人掛心。」",
                            t_name, wear, cond
                        )
                    } else if let Some((_, subj)) = &ob_opt {
                        format!("「【{subj}】是我自己的差事，不便假手於人，請回吧。」")
                    } else {
                        format!("「我手頭活計自會料理，暫無外人可插手之處。」")
                    }
                }
                _ => {
                    if has_worn_tool {
                        let (t_name, wear, cond) = live_tool_info.as_ref().unwrap();
                        format!(
                            "「要說難處，手頭這件【{}】磨耗已至 {}/3（器況{}），若你得空代為送往鐵坊整修，或是覓得砥石打磨，當真幫了大忙。」",
                            t_name, wear, cond
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
                format!(
                    "「我手頭這件【{}】磨損達 {}/3（器況{}），若有工坊物材或修整門道，倒真想向你討教一二。」",
                    t_name, wear, cond
                )
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
    } else {
        // 一般搭話或問候
        match stance {
            NpcStanceChoice::Accept => {
                if let Some(t) = latest_thought.as_deref() {
                    let is_comfort = ["肚裡暖和", "舒坦", "精神", "暖意", "歇過"]
                        .iter()
                        .any(|k| t.contains(k));
                    if is_comfort {
                        format!("「眼下正{act_phrase}，前刻{t}，此時精神正好，相逢便是有緣。」")
                    } else {
                        format!("「眼下正{act_phrase}，前刻心裡還在盤算『{t}』呢，相逢便是有緣。」")
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
                format!("「我正忙著手頭{act_phrase}的營生，尋常度日罷了。」")
            }
            NpcStanceChoice::Refuse => {
                format!("「請自便吧，我這頭正{act_phrase}忙碌著，照應不周。」")
            }
        }
    };

    let narrative = format_talk_narrative(target, talk_name, &line, player_input);
    Some((narrative, line))
}

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
    "給", "交", "幫", "修", "柴", "木", "炭", "板", "柄", "火把",
    "鐵", "礦", "草", "藥", "蘆", "繩", "布", "紙", "膠", "皮",
    "拿去", "收下", "帶來", "完成",
];

/// 玩家明確主動贈予或交付之強意圖關鍵詞
pub const EXPLICIT_OFFER_KEYWORDS: &[&str] = &[
    "給你", "交給", "幫你修", "這束柴", "這塊木", "拿去用",
];

/// 請求物資支援關鍵詞（Accept 觸發）
pub const ACCEPT_MATERIAL_KEYWORDS: &[&str] = &[
    "柴", "木", "火", "料", "借", "討", "炭", "板",
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
        ("repair_tool", "水密熟船膠與浸油麻繩修固船具")
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

/// 對話世界效果回傳結果
#[derive(Default, Clone)]
pub struct TalkWorldEffectOutcome {
    pub narrative_suffix: Option<String>,
    pub prompt_hint: Option<String>,
}

/// A. 透過對話交割物資以滿足 NPC 待辦需求
fn try_fulfill_npc_need_via_talk(
    conn: &WsConnection,
    pid: &str,
    target_id: &str,
    player_input: &str,
    _decision_choice: NpcStanceChoice,
    obligation_descs: &[String],
    now: i64,
    pool: Option<&crate::store::sql::DbPool>,
) -> Option<TalkWorldEffectOutcome> {
    let active_ob = crate::store::get_active_npc_obligation(target_id);
    let has_obligation = active_ob.is_some() || !obligation_descs.is_empty();
    let obligation_hint = active_ob.as_ref().map(|(_, subj)| subj.as_str());

    let (item_id, item_name) = find_deliverable_item(pid, player_input, obligation_hint)?;

    let mentions_delivery = is_delivery_intent_utterance(player_input);
    let explicit_offer = is_explicit_offer_utterance(player_input);

    let is_passive_greeting_or_query = matches!(
        player_input.trim(),
        "寒暄問候" | "打聽近況" | "請教差事" | "（搭話）"
    );
    let triggered = !is_passive_greeting_or_query
        && ((has_obligation && mentions_delivery) || explicit_offer);

    if !triggered {
        return None;
    }

    let _ = db::remove_from_inventory(pid, item_id, 1);
    let _ = db::add_to_inventory(target_id, item_id, 1);
    let resolved_ob_kind = active_ob
        .as_ref()
        .map(|(k, _)| k.clone())
        .or_else(|| {
            pool.and_then(|p| {
                crate::pg::living_world::list_open_obligations_for_entity(p, target_id)
                    .ok()
                    .and_then(|obs| {
                        obs.into_iter()
                            .find(|o| o.obligation_kind != "employment")
                            .map(|o| o.obligation_kind)
                    })
            })
        })
        .unwrap_or_default();
    crate::store::clear_active_npc_obligation(target_id, &resolved_ob_kind);
    let _ = db::transfer_magnesium(target_id, pid, 8).or_else(|_| db::add_magnesium(pid, 8));
    let _ = db::adjust_favorability(target_id, pid, 5);
    if let Some(p) = pool {
        let _ = crate::pg::living_world::record_relationship_delta(
            p,
            target_id,
            pid,
            "helped",
            2,
            "talk_obligation_fulfilled",
            now,
        );
    }
    let player_name = db::get_entity(pid)
        .ok()
        .flatten()
        .map(|e| e.display_title)
        .unwrap_or_else(|| "遊歷客友".to_string());
    let player_room = db::get_entity_room(pid).unwrap_or_default();
    let player_room_name = db::get_room_name(&player_room).unwrap_or_else(|_| player_room.clone());
    let delivered_order = if let Some((order, bounty)) = crate::npc::causal_core::deliver_stranded_cargo_for_player(pid, &player_name, target_id, &player_room_name, now) {
        crate::npc::causal_core::mutate_npc_from_player_action(
            target_id,
            pid,
            &player_name,
            &player_room_name,
            "rescue_cargo",
            &order.item_name,
            bounty,
            now,
        );
        Some((order, bounty))
    } else {
        None
    };

    push_refresh(conn, pid);

    let origin_str = crate::game::object_ecology::lookup_item_origin(item_id);
    let origin_note = if !origin_str.is_empty() {
        format!("（產自【{origin_str}】）")
    } else {
        String::new()
    };
    let cargo_extra_note = if let Some((order, bounty)) = &delivered_order {
        format!("（滯留貨單「{}」解送交付，公庫撥付賞格 {} 鎂）", order.item_name, bounty)
    } else {
        String::new()
    };

    Some(TalkWorldEffectOutcome {
        narrative_suffix: Some(format!(
            "\n【言出事成・器物交割】你從行囊取出【{item_name}】{origin_note}遞交對方，當場解決了對方手頭緊迫的待辦要務；對方神色舒展，取出八枚沉甸甸的鎂幣塞入你手中酬謝。{cargo_extra_note}"
        )),
        prompt_hint: Some(if !origin_str.is_empty() {
            format!("【物資交割】來客當面遞交了產自【{origin_str}】的【{item_name}】，解決了你手頭緊迫的待辦事項。請在回話中由衷感謝並收下該物件。")
        } else {
            format!("【物資交割】來客當面遞交了【{item_name}】，解決了你手頭緊迫的待辦事項。請在回話中由衷感謝並收下該物件。")
        }),
    })
}

/// B. 在 Bargain 立場下具現化待辦義務
fn ensure_bargain_obligation(
    target_id: &str,
    talk_name: &str,
    current_activity: &str,
) -> TalkWorldEffectOutcome {
    let ob_subject = if let Some((_, subj)) = crate::store::get_active_npc_obligation(target_id) {
        subj
    } else {
        let (ob_kind, ob_subject) = infer_npc_obligation(talk_name, current_activity);
        crate::store::open_active_npc_obligation(target_id, ob_kind, ob_subject);
        ob_subject.to_string()
    };

    TalkWorldEffectOutcome {
        narrative_suffix: Some(format!(
            "\n【立約待辦・條件交換】對方提出了明確的交換條件（{ob_subject}）——看來只要替他備齊所需的料件，這樁事便好商量。"
        )),
        prompt_hint: None,
    }
}

/// C. 在 Accept 立場下給予實質互助支援（柴木物資或溫言款待調養）
fn apply_accept_world_effect(
    conn: &WsConnection,
    pid: &str,
    player_input: &str,
    _familiarity_score: i32,
) -> Option<TalkWorldEffectOutcome> {
    if is_accept_material_utterance(player_input) {
        let _ = db::add_to_inventory(pid, "mat_kindling_bundle", 1);
        push_refresh(conn, pid);
        Some(TalkWorldEffectOutcome {
            narrative_suffix: Some(
                "\n【同道相助・物資支援】對方欣然應允，隨手從身側取出一束乾燥耐燃的【乾燥柴枝束】遞進你手裡，可供野外生火、添柴入爐或劈削精製。".to_string(),
            ),
            prompt_hint: Some(
                "【物資支援】你見來客有所需求，便欣然應允並從身側取出一束【乾燥柴枝束】相贈。請在回話中展現友善並提及交付柴枝。".to_string(),
            ),
        })
    } else if is_accept_comfort_utterance(player_input) {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 12).min(100);
                e.qi = (e.qi + 12).min(100);
            });
        }
        push_refresh(conn, pid);
        Some(TalkWorldEffectOutcome {
            narrative_suffix: Some(
                "\n【同道相助・溫言款待】對方熱情招呼你在近旁歇腳暖手，分予溫熱茶飲，你周身的疲乏與寒意隨之消散不少。".to_string(),
            ),
            prompt_hint: Some(
                "【溫言款待】你熱情招呼來客在近旁歇腳暖手並分予溫熱茶飲。請在回話中展現溫和關照之意。".to_string(),
            ),
        })
    } else {
        None
    }
}

/// 綜合執行對話世界效果（A 優先，未觸發則依 Stance 派發 B 或 C）
fn apply_talk_world_effects(
    conn: &WsConnection,
    pid: &str,
    target_id: &str,
    talk_name: &str,
    current_activity: &str,
    player_input: &str,
    decision_choice: NpcStanceChoice,
    obligation_descs: &[String],
    familiarity_score: i32,
    now: i64,
    pool: Option<&crate::store::sql::DbPool>,
) -> TalkWorldEffectOutcome {
    if let Some(outcome) = try_fulfill_npc_need_via_talk(
        conn,
        pid,
        target_id,
        player_input,
        decision_choice,
        obligation_descs,
        now,
        pool,
    ) {
        return outcome;
    }

    match decision_choice {
        NpcStanceChoice::Bargain => {
            if !matches!(player_input.trim(), "寒暄問候" | "打聽近況" | "（搭話）") {
                ensure_bargain_obligation(target_id, talk_name, current_activity)
            } else {
                TalkWorldEffectOutcome::default()
            }
        }
        NpcStanceChoice::Accept => {
            apply_accept_world_effect(conn, pid, player_input, familiarity_score)
                .unwrap_or_default()
        }
        _ => TalkWorldEffectOutcome::default(),
    }
}
fn resolve_mentor_word_id(
    registry: &crate::combat::wordplate::WordRegistry,
    candidate_id: &str,
    ch: &str,
) -> String {
    if registry.get_by_id(candidate_id).is_some() {
        return candidate_id.to_string();
    }
    if let Some(ids) = registry.get_by_char(ch) {
        if let Some(first) = ids.first() {
            return first.clone();
        }
    }
    if ch == "疾" {
        if let Some(ids) = registry.get_by_char("速") {
            if let Some(first) = ids.first() {
                return first.clone();
            }
        }
        return "su".to_string();
    }
    registry
        .ordered_elements()
        .next()
        .map(|w| w.id.clone())
        .unwrap_or_else(|| "wu".to_string())
}

/// 3 棒師承請益與交涉中繼（請益／學藝／指點／借物／議價）
pub fn handle_three_baton_mentorship_and_obligation(
    pid: &str,
    target: &Character,
    player_text: &str,
    stance: NegotiationStance,
    now: i64,
) -> Option<String> {
    // Baton 1 (Intent & Stance Gate):
    // 1. 永遠於 PostgreSQL living_world 記錄對話互動交情
    if let Some(pool) = crate::pg::pool() {
        let _ = crate::pg::living_world::upsert_relationship(
            &pool,
            &target.id,
            pid,
            "acquaintance",
            1,
            1,
            "talk_interaction",
            now,
        );
    }

    // 2. 檢測玩家是否提出百工或武藝指點請求，且 NPC 立場並非拒絕
    const GUIDANCE_KEYWORDS: &[&str] = &[
        "請益", "指點", "學藝", "手札", "刀法", "錘法", "火候", "木作", "要訣", "師傅", "教我",
    ];
    let asks_guidance = GUIDANCE_KEYWORDS.iter().any(|&kw| player_text.contains(kw));
    if !asks_guidance || stance == NegotiationStance::Refuse {
        return None;
    }

    // Baton 2 (Immediate Backend State Mutation):
    // 根據目標姓名、命途稱謂與當前活動推論師承領域
    let real_name = resolve_npc_real_name(target);
    let domain_source = format!("{}:{}:{}:{}", target.id, real_name, target.display_title, target.current_activity);

    let held_imprints = crate::pg::living_world::get_entity_lineage_imprints(&target.id);
    let has_watermill_imprint = held_imprints.iter().any(|imp| {
        imp.imprint_title.contains("水輪") || imp.imprint_title.contains("石磨")
    });
    let has_watermill_tool = target.equipment_slots.contains("wt_cat_0973")
        || target.equipment_slots.contains("wt_cat_0984");
    let is_watermill = has_watermill_imprint
        || has_watermill_tool
        || domain_source.contains("水輪")
        || domain_source.contains("石磨");
    let is_smithing = ["鐵", "鍛", "岑", "礦", "石"].iter().any(|k| domain_source.contains(k));
    let is_culinary = ["廚", "食", "大娘", "麵", "茶", "漁", "海"].iter().any(|k| domain_source.contains(k));
    let is_hunting = ["獵", "衛", "哨", "武", "刀", "巡", "羅", "趙"].iter().any(|k| domain_source.contains(k));

    let mastered_imprint = held_imprints
        .into_iter()
        .find(|imp| imp.manual_gifted || imp.interaction_count >= 3);

    let (mentor_label_str, imprint_title_str, imprint_prose_str, habit_kind, posture, raw_word_ids, raw_word_chars): (
        String,
        String,
        String,
        crate::pg::living_world::BreathHabitKind,
        &str,
        &[&str],
        &[&str],
    ) = if let Some(mastered) = mastered_imprint {
        let m_label = if is_watermill {
            if real_name.contains('秦') || target.id.contains('秦') {
                "秦氏水輪石磨真傳".to_string()
            } else if !real_name.is_empty() && real_name != "陌生人" {
                format!("{real_name}・水輪石磨真傳")
            } else {
                "水輪石磨真傳".to_string()
            }
        } else if target.kind == crate::entity::EntityKind::Player {
            real_name.clone()
        } else if !mastered.mentor_name.is_empty() {
            mastered.mentor_name.clone()
        } else {
            real_name.clone()
        };
        let m_title = mastered.imprint_title;
        let m_prose = if is_watermill
            && (mastered.imprint_prose.is_empty() || mastered.imprint_prose == "聽水辨速、循紋落鏨")
        {
            "順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性".to_string()
        } else {
            mastered.imprint_prose
        };

        let (hk, post, w_ids, w_chs): (
            crate::pg::living_world::BreathHabitKind,
            &str,
            &[&str],
            &[&str],
        ) = if m_title.contains("石磨") || m_title.contains("水輪") || is_watermill {
            (
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                "雙疊聽音",
                &["mo", "liu", "chun"],
                &["磨", "流", "純"],
            )
        } else if m_title.contains("鍛") || m_title.contains("落錘") || is_smithing {
            (
                crate::pg::living_world::BreathHabitKind::ForgeHeavy,
                "重手",
                &["gang", "chen_tai", "zhong"],
                &["\u{525b}", "沉", "重"],
            )
        } else if m_title.contains("收汁") || m_title.contains("庖廚") || is_culinary {
            (
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                "庖廚百工",
                &["shui", "huo_shou", "he_tai"],
                &["水", "火", "合"],
            )
        } else if m_title.contains("破隙") || m_title.contains("走刀") || is_hunting {
            (
                crate::pg::living_world::BreathHabitKind::ForestLong,
                "起手",
                &["ji_su", "ci", "chuan"],
                &["疾", "刺", "穿"],
            )
        } else {
            (
                crate::pg::living_world::BreathHabitKind::ChamberClear,
                "百工衡準",
                &["zhun", "ding", "he_tai"],
                &["準", "定", "合"],
            )
        };
        (m_label, m_title, m_prose, hk, post, w_ids, w_chs)
    } else if is_watermill {
        let mill_label = if real_name.contains('秦') || target.id.contains('秦') {
            "秦氏水輪石磨真傳".to_string()
        } else if !real_name.is_empty() && real_name != "陌生人" {
            format!("{real_name}・水輪石磨真傳")
        } else {
            "水輪石磨真傳".to_string()
        };
        (
            mill_label,
            "老堰頭水輪分動與雙疊石磨聽音辨距法".to_string(),
            "順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性".to_string(),
            crate::pg::living_world::BreathHabitKind::TideContinuous,
            "雙疊聽音",
            &["mo", "liu", "chun"],
            &["磨", "流", "純"],
        )
    } else if is_smithing {
        (
            "岑氏鍛坊真傳".to_string(),
            "岑氏沉腕落錘法".to_string(),
            "沉腕蓄勁、順著鐵胚紋理落錘，使器物與內息渾然相合".to_string(),
            crate::pg::living_world::BreathHabitKind::ForgeHeavy,
            "重手",
            &["gang", "chen_tai", "zhong"],
            &["\u{525b}", "沉", "重"],
        )
    } else if is_culinary {
        (
            "三水庖廚真傳".to_string(),
            "三水火候收汁訣".to_string(),
            "以綿長內息感應鼎釜冷熱交替，鎖住山海靈材本味".to_string(),
            crate::pg::living_world::BreathHabitKind::TideContinuous,
            "庖廚百工",
            &["shui", "huo_shou", "he_tai"],
            &["水", "火", "合"],
        )
    } else if is_hunting {
        (
            "山海尋隙真傳".to_string(),
            "順紋走刀破隙訣".to_string(),
            "運刃順骨縫游走、避實擊虛，手眼身步一氣呵成".to_string(),
            crate::pg::living_world::BreathHabitKind::ForestLong,
            "起手",
            &["ji_su", "ci", "chuan"],
            &["疾", "刺", "穿"],
        )
    } else {
        (
            "半島百工衡準".to_string(),
            "百工調息衡準訣".to_string(),
            "神思澄明、呼吸勻淨，於分寸之間拿捏器物重心".to_string(),
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            "百工衡準",
            &["zhun", "ding", "he_tai"],
            &["準", "定", "合"],
        )
    };
    let mentor_label = mentor_label_str.as_str();
    let imprint_title = imprint_title_str.as_str();
    let imprint_prose = imprint_prose_str.as_str();

    crate::pg::living_world::record_breath_activity(pid, habit_kind, 2, now);
    let (_imprint_rec, newly_gifted) = crate::pg::living_world::record_mentor_imprint_progress(
        pid,
        mentor_label,
        imprint_title,
        imprint_prose,
        now,
    );

    let registry = crate::combat::wordplate::WordRegistry::global();
    let mut resolved_word_ids = Vec::with_capacity(3);
    for (&wid, &ch) in raw_word_ids.iter().zip(raw_word_chars.iter()) {
        resolved_word_ids.push(resolve_mentor_word_id(registry, wid, ch));
    }

    if newly_gifted {
        let word_id_refs: Vec<&str> = resolved_word_ids.iter().map(|s| s.as_str()).collect();
        crate::server::handler::inventory::grant_words_to_player_casket(pid, &word_id_refs);

        let mut hasher = 0xcbf29ce484222325u64;
        for b in format!("{}_{}", mentor_label, imprint_title).bytes() {
            hasher ^= b as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
        }
        let manual_hash = format!("{:016x}", hasher);

        let manual_rec = crate::pg::living_world::TranscribedManualRecord {
            manual_hash: manual_hash.clone(),
            author_id: target.id.clone(),
            author_name: mentor_label.to_string(),
            formal_name: imprint_title.to_string(),
            slot_label: posture.to_string(),
            carrier_code: "tool".to_string(),
            word_ids: resolved_word_ids.clone(),
            word_chars: raw_word_chars.iter().map(|s| s.to_string()).collect(),
            origin_basis: imprint_prose.to_string(),
            created_at: now,
        };
        crate::pg::living_world::save_transcribed_manual(&manual_rec);
        let _ = crate::db::inventory::add_to_inventory(pid, &format!("manual_{manual_hash}"), 1);
    }

    // Baton 3 (Voice & Narrative Note):
    let note = if newly_gifted {
        format!(
            "\n【行家指點・師承烙印】{}傾囊相授，為你細說【{}】之奧妙，體會到：「{}」。你依循提點調整吐納，氣息周天漸入佳境。\n【親筆授譜】{}見你領悟精進、根基漸穩，當場將一冊親筆《{}手札》與其中要訣心法傳授於你！",
            mentor_label, imprint_title, imprint_prose, mentor_label, imprint_title
        )
    } else {
        format!(
            "\n【行家指點・師承烙印】{}傾囊相授，為你細說【{}】之奧妙，體會到：「{}」。你依循提點調整吐納，氣息周天漸入佳境。",
            mentor_label, imprint_title, imprint_prose
        )
    };

    Some(note)
}

trait KeyObjectsJoinExt {
    fn join(&self, sep: &str) -> String;
}

impl KeyObjectsJoinExt for str {
    fn join(&self, sep: &str) -> String {
        clean_catalog_codes(self)
            .split('、')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(sep)
    }
}

pub fn build_universal_entity_live_talk_context(target: &Character) -> String {
    // 1. 背包中真實庫存（取最多 4 項 qty > 0，自然量詞：桶、袋、×N）
    let equipped_ids: std::collections::HashSet<String> =
        serde_json::from_str::<std::collections::HashMap<String, String>>(&target.equipment_slots)
            .unwrap_or_default()
            .into_values()
            .filter(|v| !v.trim().is_empty())
            .collect();
    let inv = crate::db::get_inventory(&target.inventory, target.vit);
    let stock_parts: Vec<String> = inv
        .items
        .iter()
        .filter(|i| i.qty > 0 && !equipped_ids.contains(&i.item_id) && !i.item_id.starts_with("crystal_"))
        .take(4)
        .map(|i| {
            let display_name = if i.name == "精碾白米" {
                "待磨精碾白米"
            } else {
                &i.name
            };
            let unit = if display_name.contains("米漿")
                || display_name.contains("湯")
                || display_name.contains("飲")
                || display_name.contains("油")
                || display_name.contains("水")
            {
                "桶"
            } else if display_name.contains("粉")
                || display_name.contains("米")
                || display_name.contains("麵")
                || display_name.contains("穀")
                || display_name.contains("鹽")
            {
                "袋"
            } else {
                ""
            };
            if !unit.is_empty() {
                format!("{display_name} {} {unit}", i.qty)
            } else {
                format!("{display_name} ×{}", i.qty)
            }
        })
        .collect();
    let stock_desc = if stock_parts.is_empty() {
        "行囊空無一物".to_string()
    } else {
        format!("隨身攜有{}", stock_parts.join("、"))
    };

    // 2. 裝備中器物／隨身工藝器具之磨損與磨合狀態
    let (tool_id, tool_name) = {
        let slots_map: Option<std::collections::HashMap<String, String>> =
            serde_json::from_str(&target.equipment_slots).ok();
        let held_id = slots_map.as_ref().and_then(|m| {
            m.get("hold_r").or_else(|| m.get("hold_l")).filter(|s| !s.is_empty())
        });
        if let Some(tid) = held_id {
            let tname = crate::db::get_item_name(tid);
            (tid.clone(), tname)
        } else {
            let vspec = crate::npc::causal_core::resolve_vocational_vessel(&target.display_title);
            (vspec.item_id.to_string(), vspec.display_name.to_string())
        }
    };
    let tool_desc = if !tool_id.is_empty() {
        let vessel = crate::pg::living_world::get_vessel_state(&target.id, &tool_id);
        format!(
            "；手持【{}】磨合度【{}】、磨損值 {}/3（器況：{}）",
            tool_name,
            vessel.familiarity().label(),
            vessel.wear_points,
            vessel.condition.label()
        )
    } else {
        String::new()
    };

    // 3. 當前活躍因果義務
    let active_ob = crate::store::get_active_npc_obligation(&target.id)
        .and_then(|(k, s)| format_obligation_subject_zh(&k, &s))
        .or_else(|| {
            if let Some(pool) = crate::pg::pool()
                && let Ok(mut client) = pool.get()
            {
                if let Ok(rows) = client.query(
                    "SELECT obligation_kind, subject_ref FROM resident_obligations WHERE assignee_id = $1 AND status IN ('open', 'active') LIMIT 1",
                    &[&target.id],
                ) {
                    if let Some(row) = rows.first() {
                        let k: String = row.get(0);
                        let s: String = row.get(1);
                        return format_obligation_subject_zh(&k, &s);
                    }
                }
            }
            None
        });
    let ob_desc = if let Some(ob) = active_ob {
        format!("。當前掛念差事：【{}】。", ob)
    } else {
        "。".to_string()
    };

    let mut script_desc = String::new();
    let room_name = if let (Some(gx), Some(gy)) = (target.grid_x, target.grid_y) {
        crate::server::grid_manager::get_square_grid()
            .and_then(|g| g.get(crate::grid::SquareCoord::new(gx, gy)).map(|c| c.name.clone()))
            .unwrap_or_else(|| target.display_title.clone())
    } else {
        target.display_title.clone()
    };
    let real_name = resolve_npc_real_name(target);
    let (is_staff, has_uniform, in_counter_zone) =
        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            &room_name,
            &target.id,
            &target.display_title,
            &target.current_activity,
            &target.equipment_slots,
        );
    let venue_opt = crate::grid::resolve_any_room_venue_object(&room_name, &target.display_title);
    let venue_staff_desc = if is_staff {
        let host_title = venue_opt
            .as_ref()
            .map(|v| v.default_host_title)
            .unwrap_or("當值人員");
        let venue_kind = venue_opt
            .as_ref()
            .map(|v| v.venue_kind)
            .unwrap_or("場館");
        if has_uniform && in_counter_zone {
            format!("。當值狀態：身著工作制服、站立於【{venue_kind}】當值櫃檯（非本店人員禁止進入區）內側，職司【{host_title}】")
        } else if in_counter_zone {
            format!("。當值狀態：站立於【{venue_kind}】當值櫃檯（非本店人員禁止進入區）內側，職司【{host_title}】")
        } else if has_uniform {
            format!("。當值狀態：身著工作制服於【{venue_kind}】當值，職司【{host_title}】")
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    if !room_name.is_empty() {
        if let Some((ev, sp)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
            &room_name,
            Some(&real_name),
            crate::game::now_unix(),
        ) {
            script_desc = format!(
                "。場上事件包：【{}】{}（{}；關鍵器物：{}；劇本包【{}】第 {}/3 幕：{}）",
                ev.pack_id,
                ev.event_title,
                ev.trigger_and_causal_chain,
                ev.key_objects.join("、"),
                sp.pack_id,
                sp.current_step.max(1),
                sp.player_actions
            );
        }
    }

    format!(
        "\n【隨身器物與行囊實況】：{}{}{}{}{}",
        stock_desc, tool_desc, ob_desc, venue_staff_desc, script_desc
    )
}

pub(super) fn do_entity_talk(
    conn: &WsConnection,
    msg: &ClientMsg,
    pid: &str,
    player_room: &str,
    target_id: &str,
    target: &Character,
    now: i64,
) {
    crate::store::pause_npc_for_interaction(target_id, 45);
    let mut player_input = msg.player_input.clone();
    if player_input.trim().is_empty() {
        player_input = "（搭話）".into();
    }
    let was_named = if target.kind == EntityKind::Npc {
        db::get_acquaintance(pid, target_id)
            .ok()
            .flatten()
            .map(|a| a.named)
            .unwrap_or(false)
    } else {
        true
    };
    let mut talk_name = super::player_visible_target_name(pid, target);
    let hour = crate::store::current_game_hour();
    let is_night = hour >= 22 || hour < 5;
    let is_sleeping = target.current_activity.contains("安歇")
        || target.current_activity.contains("熟睡")
        || target.current_activity.contains("就寢")
        || target.current_activity.contains("熄燈");
    if is_night && is_sleeping {
        let narrative = "夜色已深，對方已在內室熄燈安歇，不便驚擾。".to_string();
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Talk".into(),
            target_id: target.id.clone(),
            target_name: talk_name,
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
    if is_open_trade_input(&player_input) && target_can_open_trade(target, target_id, player_room) {
        let narrative = if player_input.trim() != "（搭話）" && !player_input.trim().is_empty() {
            format!("【{talk_name}】示意你可以先看貨、再出價。")
        } else {
            format!("你向【{talk_name}】問起交易。{talk_name}示意你可以先看貨、再出價。")
        };
        let npc_reply = "可以先看貨、再出價。".to_string();
        let _ = event::append(now, pid, "talk", target_id);
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Talk".into(),
            target_id: target.id.clone(),
            target_name: talk_name.clone(),
            narrative,
            success: true,
            actions: vec!["Trade".into()],
            move_target_id: String::new(),
            open_trade: true,
            suggested_action: "Trade".into(),
            tactical_choices: vec![],
        });
        conn.sessions
            .record_player_talk_for_room_echo(pid, player_room, &msg.player_input);
        flush_conversation_and_append(pid, target_id, &player_input, &npc_reply, now);
        let _ = adjust_favorability(target_id, pid, FAV_TALK);
        return;
    }
    let is_quick_chip_talk = matches!(player_input.trim(), "寒暄問候" | "打聽近況" | "請教差事" | "（搭話）");
    let (backstory, style_examples, mut sensitivity) = if is_quick_chip_talk {
        (String::new(), Vec::new(), String::new())
    } else {
        let raw_backstory = db::build_identity(target_id);
        let bs = if target.kind == EntityKind::Npc {
            let real_name = resolve_npc_real_name(target);
            let patched = raw_backstory.replace(target_id, &real_name);
            if !patched.contains(&real_name) {
                format!("你的真實身份與姓名是：{real_name}。{patched}")
            } else {
                patched
            }
        } else {
            raw_backstory
        };
        let mut se = pick_style_examples(target_id, 3);
        se.retain(|s| {
            !s.contains("錢袋")
                && !s.contains("再誇")
                && !s.contains("花錢")
                && !s.contains("有啥")
                && !s.contains("啥事")
                && !s.contains("\u{5ba2}\u{5b98}")
                && !s.contains("客倌")
                && !s.contains("小二")
                && !s.contains("銀兩")
                && !s.contains("銅錢")
                && !s.contains("\u{638c}\u{6ac3}")
        });
        let mut sens = talk_sensitivity_hint(target);
        sens.push_str(&talk_disposition_hint(target_id, pid));
        (bs, se, sens)
    };
    let personality = crate::db::get_personality_for_entity(target_id)
        .or_else(|| target.soul_seed.map(crate::db::expand_soul_seed_to_personality))
        .unwrap_or(crate::db::Personality {
            boldness: 0.5,
            sensitivity: 0.5,
            orderliness: 0.5,
        });
    let pool_opt = crate::pg::pool();
    let (rel, obligations) = if let Some(pool) = &pool_opt {
        let r = crate::pg::living_world::get_relationship_summary(pool, target_id, pid)
            .unwrap_or_default();
        let o = crate::pg::living_world::list_open_obligations_for_entity(pool, target_id)
            .unwrap_or_default();
        (r, o)
    } else {
        (crate::pg::living_world::RelationshipSummary::default(), Vec::new())
    };
    let mut obligation_descs: Vec<String> = obligations
        .iter()
        .filter(|o| o.obligation_kind != "employment")
        .filter_map(|o| format_obligation_subject_zh(&o.obligation_kind, &o.subject_ref))
        .collect();
    if let Some((kind, subj)) = crate::store::get_active_npc_obligation(target_id) {
        if let Some(desc) = format_obligation_subject_zh(&kind, &subj) {
            if !obligation_descs.contains(&desc) {
                obligation_descs.push(desc);
            }
        }
    }
    let current_activity = if target.current_activity.is_empty() {
        "佇立觀察四周"
    } else {
        &target.current_activity
    };

    // Step 1: 拍板 NPC 互動立場
    let decision_input = NpcDecisionInput {
        npc_name: &talk_name,
        boldness: personality.boldness,
        sensitivity: personality.sensitivity,
        orderliness: personality.orderliness,
        familiarity_score: rel.familiarity_score,
        balance_score: rel.balance_score,
        open_obligations: &obligation_descs,
        current_activity,
        action_kind: "Talk",
        player_utterance: &player_input,
    };
    let mut decision = score_npc_interaction_stance(&decision_input);
    let resolved_room_for_stance = db::get_room_name(player_room).unwrap_or_else(|_| player_room.to_string());
    let (eval_stance, eval_stance_desc) = if let Ok(Some(player_ent)) = db::get_entity(pid) {
        let inv = db::get_inventory(&player_ent.inventory, player_ent.vit);
        evaluate_npc_negotiation_stance(pid, target_id, &resolved_room_for_stance, &inv)
    } else {
        let empty_inv = crate::db::inventory::InventorySnapshot {
            items: vec![],
            current_weight: 0.0,
            max_weight: 50.0,
        };
        evaluate_npc_negotiation_stance(pid, target_id, &resolved_room_for_stance, &empty_inv)
    };
    if eval_stance == NegotiationStance::Refuse {
        decision.choice = NpcStanceChoice::Refuse;
    }
    let mentorship_note = handle_three_baton_mentorship_and_obligation(
        pid,
        target,
        &player_input,
        eval_stance,
        now,
    );
    if mentorship_note.is_some() {
        push_refresh(conn, pid);
    }

    // Step 2.5: 實行對話世界效果（義務交割、立約交換、同道相助）
    let world_effect = apply_talk_world_effects(
        conn,
        pid,
        target_id,
        &talk_name,
        current_activity,
        &player_input,
        decision.choice,
        &obligation_descs,
        rel.familiarity_score,
        now,
        pool_opt.as_deref(),
    );
    let hunting_pledge_note = if decision.choice != NpcStanceChoice::Refuse {
        let party_opt = crate::store::LIVE_HUNTING_PARTIES
            .read()
            .values()
            .find(|p| p.member_ids.contains(&target.id))
            .cloned();
        if let Some(party) = party_opt {
            crate::store::pledge_player_to_hunting_party(pid, &party.zone_name);
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::ForestLong,
                1,
                now,
            );
            let (px, py) = party.prey_coord;
            let prey_place = crate::game::CANONICAL_HUNTING_ZONES
                .get(crate::game::nearest_hunting_zone_idx(px, py))
                .map(|z| z.0.to_string())
                .unwrap_or_else(|| {
                    let (z, s, _) = crate::grid::r#gen::classify_wilderness_cell(px, py);
                    format!("{z}・{s}")
                });
            Some(format!(
                "\n【合社入團・見者有份】你已應募加入領隊【{}】率領之【{}】合社獵團（現共 {} 名獵手）！當前鎖定獵物為【{prey_place}】一帶之【{}】——無論由你親手擊殺或隨獵團合圍收網，皆可依半島古規「見者有份」分得【{}】與交割鎂幣紅利！",
                party.leader_name,
                party.zone_name,
                party.member_ids.len(),
                party.prey_name,
                party.prey_loot_name
            ))
        } else {
            None
        }
    } else {
        None
    };

    // Step 3: 將拍板指令注入 prompt
    if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
        sensitivity.push('\n');
    }
    sensitivity.push_str(&decision.prompt_directive);
    if !sensitivity.ends_with('\n') {
        sensitivity.push('\n');
    }
    sensitivity.push_str(&eval_stance_desc);
    if let Some((kind, subj)) = crate::store::get_active_npc_obligation(target_id) {
        if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
            sensitivity.push('\n');
        }
        sensitivity.push_str(&format!(
            "【當前待辦義務】NPC目前身負待辦事務（{}: {}），處境緊迫或分心，請在對話神態與回應中自然體現。",
            kind, subj
        ));
    }
    if let Some(prompt_hint) = &world_effect.prompt_hint {
        if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
            sensitivity.push('\n');
        }
        sensitivity.push_str(prompt_hint);
    }
    if mentorship_note.is_some() {
        if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
            sensitivity.push('\n');
        }
        sensitivity.push_str("【行家指點・師承授受】你正在就百工修行與身法要訣指點來客，請在回覆中具體流露行家指點與傳授奧妙的長者風範。");
    }

    let trimmed_input = player_input.trim();
    let recent_turns = get_recent_turns(pid, target_id, 5);
    let effective_user_prompt = if trimmed_input.is_empty() || trimmed_input == "（搭話）" {
        if !recent_turns.is_empty() {
            let last_topic = recent_turns.last().map(|t| t.0.as_str()).unwrap_or("");
            if !last_topic.is_empty() && last_topic != "（搭話）" {
                format!(
                    "（來客再次向你開口搭話，請延續你們先前的話題「{}」或根據你手頭正在做的事自然回應）",
                    last_topic
                )
            } else {
                "（來客再次向你開口搭話，請延續你們先前交談的氣氛或根據你手頭正在做的事自然回應）".to_string()
            }
        } else if was_named || rel.familiarity_score >= 2 {
            "（熟識的來客走近向你打招呼，請以熟絡口吻結合你手頭的活計、心情或近期市集供需動靜主動開口）".to_string()
        } else {
            "（一名來客走近向你搭話，請展現你正在忙碌的動作神態，並以符合你職業與性格的半島自然口語開口回應）".to_string()
        }
    } else {
        trimmed_input.to_string()
    };

    let is_quick_chip_talk = matches!(trimmed_input, "寒暄問候" | "打聽近況" | "（搭話）");
    let mut snippets = Vec::new();
    if !is_quick_chip_talk {
        let session_turns = get_recent_turns(pid, target_id, 6);
        for (p_in, n_rep) in &session_turns {
            let p_in = p_in.trim();
            let n_rep = n_rep.trim();
            if !p_in.is_empty() && !n_rep.is_empty() {
                snippets.push(format!("【先前對話】來客說：「{}」 → 你回答：「{}」", p_in, n_rep));
            }
        }
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            let archs = s.get_archival_by_entity(target_id);
            for a in archs.into_iter().rev() {
                if (a.tag == "player_talk" || a.content.contains(pid) || a.content.contains("來客"))
                    && !a.content.contains("青籠關")
                    && !a.content.contains("林緣聚地")
                {
                    snippets.push(format!("【往日記憶】{}", a.content));
                    if snippets.len() >= 8 {
                        break;
                    }
                }
            }
        }
        let extra_archival = search_archival_for_player_talk(target_id, &effective_user_prompt, 4);
        for s in extra_archival {
            if !s.contains("青籠關")
                && !s.contains("林緣聚地")
                && !s.contains("草原聚地")
                && !s.contains("錢袋")
                && !s.contains("有啥")
                && !snippets.iter().any(|existing| existing.contains(&s))
            {
                snippets.push(s);
            }
        }
    }

    let mut room_display_name = super::resolve_action_room_name(pid, target);
    if room_display_name.trim().is_empty() {
        room_display_name = get_room_name(player_room).unwrap_or_default();
    }
    if room_display_name.trim().is_empty() {
        room_display_name = player_room.to_string();
    }
    let backstory = if is_quick_chip_talk {
        String::new()
    } else {
        let mut bs = format!(
            "你目前所在的當前地點是：【{room_display_name}】（對話時請嚴格以【{room_display_name}】與你當前的具體活動為準，切勿提及不在場的其他關隘或異地）。{}",
            backstory.replace("青籠關", "文曜")
        );
        let venue_ctx = extract_venue_and_role_talk_context(target, &room_display_name);
        if !venue_ctx.is_empty() {
            bs.push_str(&format!("\n【坊市職責與攤位現況】：{}", venue_ctx));
        }
        let live_talk_ctx = build_universal_entity_live_talk_context(target);
        if !live_talk_ctx.is_empty() {
            bs.push_str(&live_talk_ctx);
        }
        if let Ok(Some(player_ent)) = db::get_entity(pid) {
            let inv = db::get_inventory(&player_ent.inventory, player_ent.vit);
            let mat_ctx = build_player_material_context(&inv, &room_display_name);
            if !mat_ctx.is_empty() {
                bs.push_str(&mat_ctx);
            }
            let (_stance, stance_desc) = evaluate_npc_negotiation_stance(pid, target_id, &room_display_name, &inv);
            bs.push_str(&format!("\n{}", stance_desc));
        }
        bs
    };
    let mut instant_cognitive_pair: Option<(String, String)> = None;
    let is_craft_artisan = target.equipment_slots.contains("wt_cat_0973")
        || target.equipment_slots.contains("wt_cat_0984")
        || target.equipment_slots.contains("craft_");
    let is_asking_what_input = ["在做甚麼", "在做什麼", "做什麼", "做甚麼", "忙什麼", "忙甚麼", "在幹嘛"]
        .iter()
        .any(|k| trimmed_input.contains(k));
    let is_asking_troubles_input = ["困難", "麻煩", "煩惱", "難處", "差事", "委託", "要不要幫"]
        .iter()
        .any(|k| trimmed_input.contains(k));
    let is_trade_help_input = ["收購", "交易", "買賣", "換", "出讓", "有什麼賣"]
        .iter()
        .any(|k| trimmed_input.contains(k));
    if matches!(trimmed_input, "寒暄問候" | "打聽近況" | "請教差事" | "（搭話）")
        || is_craft_artisan
        || is_asking_what_input
        || is_asking_troubles_input
        || is_trade_help_input
    {
        if let Some((narr, reply)) = build_cognitive_talk_reply(
            target,
            &talk_name,
            &room_display_name,
            trimmed_input,
            current_activity,
            decision.choice,
            &obligation_descs,
        ) {
            instant_cognitive_pair = Some((narr, reply));
        }
    }

    let (narrative, npc_reply, ai_succeeded) = if let Some((n, r)) = instant_cognitive_pair {
        (n, r, true)
    } else {
        let sanitize_instant_time_words = |text: String| -> String {
            text.replace("\u{525b}裝好", "前刻裝好")
                .replace("\u{525b}從", "前刻從")
                .replace("\u{525b}聽", "前刻聽")
                .replace("\u{525b}到", "初到")
                .replace("\u{525b}\u{624d}", "前刻")
                .replace("\u{525b}\u{525b}", "前刻")
                .replace("\u{65b9}\u{624d}", "前刻")
        };
        let ai_result = if conn.cfg.player_talk_uses_web_api() {
            let (system_prompt, user_msg) = player_npc_talk_build_prompts(
                &effective_user_prompt,
                &backstory,
                &snippets,
                &style_examples,
                &sensitivity,
                &room_display_name,
            );
            match call_openai_compatible_chat(
                &conn.cfg.player_talk_api_base_url,
                &conn.cfg.player_talk_api_key,
                &conn.cfg.player_talk_api_model,
                &system_prompt,
                &user_msg,
                110,
            ) {
                Ok(reply) => Ok(sanitize_instant_time_words(reply)),
                Err(e) => {
                    tracing::warn!("web api chat error: {e}, falling back to local fast GPU / call_ai_talk");
                    match call_openai_compatible_chat(
                        crate::ai::LOCAL_FAST_GPU_URL,
                        "",
                        crate::ai::LOCAL_FAST_GPU_MODEL,
                        &system_prompt,
                        &user_msg,
                        350,
                    ) {
                        Ok(reply) => Ok(sanitize_instant_time_words(reply)),
                        Err(e2) => {
                            tracing::warn!("local fast GPU chat error: {e2}, falling back to call_ai_talk");
                            call_ai_talk(
                                &conn.cfg.ollama_base_url,
                                &conn.cfg.ollama_model,
                                &effective_user_prompt,
                                &backstory,
                                &snippets,
                                &style_examples,
                                &sensitivity,
                                &room_display_name,
                            )
                            .map(sanitize_instant_time_words)
                        }
                    }
                }
            }
        } else {
            call_ai_talk(
                &conn.cfg.ollama_base_url,
                &conn.cfg.ollama_model,
                &effective_user_prompt,
                &backstory,
                &snippets,
                &style_examples,
                &sensitivity,
                &room_display_name,
            )
            .map(sanitize_instant_time_words)
        };

        match ai_result {
            Ok(reply) => {
                let narr = format_npc_talk_line(target, &talk_name, &reply, &player_input, pid);
                (narr, reply, true)
            }
            Err(err) => {
                tracing::warn!("all AI calls failed ({err}), falling back to cognitive / canonical or rule reply");
                let (n, r) = if let Some(cog_pair) = build_cognitive_talk_reply(
                    target,
                    &talk_name,
                    &room_display_name,
                    &player_input,
                    current_activity,
                    decision.choice,
                    &obligation_descs,
                ) {
                    cog_pair
                } else if let Some(canon_pair) = canonical_or_venue_talk_reply(
                    target,
                    &talk_name,
                    &room_display_name,
                    &player_input,
                    decision.choice,
                    false,
                ) {
                    canon_pair
                } else {
                    build_talk_fallback(
                        target,
                        &talk_name,
                        &room_display_name,
                        &player_input,
                        decision.choice,
                    )
                };
                (n, r, false)
            }
        }
    };
    let gender_str = match target.gender {
        Some(crate::entity::Gender::F) => Some("F"),
        Some(crate::entity::Gender::M) => Some("M"),
        None => None,
    };
    let short_real_name =
        crate::store::resolve_npc_person_name(&target.id, &target.display_title, gender_str);
    let real_name = resolve_npc_real_name(target);
    let asking_name = is_asking_name(&player_input);
    let self_intro = npc_reply.contains("在下")
        || npc_reply.contains("我叫")
        || npc_reply.contains("我是")
        || npc_reply.contains("叫我")
        || npc_reply.contains("老夫")
        || npc_reply.contains("老身")
        || npc_reply.contains("免貴姓")
        || npc_reply.contains("賤名");
    let name_revealed_in_text = (!short_real_name.is_empty()
        && (narrative.contains(&short_real_name) || npc_reply.contains(&short_real_name)))
        || (!real_name.is_empty()
            && (narrative.contains(&real_name) || npc_reply.contains(&real_name)));
    let relationship_met = rel.familiarity_score >= 2
        || rel.balance_score >= 1
        || world_effect.narrative_suffix.is_some();
    let quick_greeting_met = matches!(trimmed_input, "寒暄問候" | "打聽近況")
        && decision.choice != NpcStanceChoice::Refuse;
    let should_unlock_name = target.kind == EntityKind::Npc
        && !was_named
        && (asking_name || self_intro || relationship_met || quick_greeting_met || name_revealed_in_text);
    let mut final_narrative = narrative;
    if should_unlock_name {
        talk_name = short_real_name.clone();
        let already_mentioned = npc_reply.contains(&real_name) || npc_reply.contains(&short_real_name);
        final_narrative = final_narrative
            .replace("【陌生人】", &format!("【{talk_name}】"))
            .replace("陌生人說道：", &format!("【{talk_name}】說道："));
        conn.send_json(&serde_json::json!({
            "type": "interact_result",
            "observation_update": {
                "target_id": target.id,
                "new_state": "named",
                "tag": short_real_name
            }
        }));
        let role_raw = crate::store::npc_persistent_role_label(&target.id, &target.display_title);
        let role_desc = role_raw
            .split(['|', '｜'])
            .next()
            .unwrap_or(&role_raw)
            .trim()
            .to_string();
        if !already_mentioned {
            final_narrative.push_str(&format!("\n交談間，你得知對方名叫【{talk_name}】（{role_desc}）。"));
        }
    }
    if let Some(suffix) = &world_effect.narrative_suffix {
        final_narrative.push_str(suffix);
    }
    if let Some(m_note) = &mentorship_note {
        final_narrative.push_str(m_note);
    }
    if let Some(h_note) = &hunting_pledge_note {
        final_narrative.push_str(h_note);
    }
    if decision.choice != NpcStanceChoice::Refuse && !room_display_name.trim().is_empty() {
        let ep_res = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
            &room_display_name,
            trimmed_input,
        )
        .or_else(|| {
            if let Some((ev, _sp)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
                &room_display_name,
                Some(&real_name),
                now,
            ) {
                let mut candidate_keys: Vec<String> = Vec::new();
                for seg in ev.key_objects.split('、') {
                    let s = seg.trim();
                    if !s.is_empty() {
                        candidate_keys.push(s.to_string());
                    }
                }
                for clean in clean_catalog_codes(ev.key_objects).split('、') {
                    let s = clean.trim();
                    if !s.is_empty() && !candidate_keys.iter().any(|k| k == s) {
                        candidate_keys.push(s.to_string());
                    }
                }
                for k in &candidate_keys {
                    if trimmed_input.contains(k.as_str()) || npc_reply.contains(k.as_str()) {
                        if let Some(res) =
                            crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
                                &room_display_name,
                                k,
                            )
                        {
                            return Some(res);
                        }
                    }
                }
            }
            None
        });
        if let Some((_pack_id, _step, ep_line)) = ep_res {
            final_narrative.push_str(&format!("\n{ep_line}"));
        }
    }
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "Talk".into(),
        target_id: target.id.clone(),
        target_name: talk_name.clone(),
        narrative: final_narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
    if should_unlock_name {
        let _ = db::set_named(pid, target_id);
        let _ = db::set_observed(pid, target_id);
        super::super::handler::send_place_view(conn, pid);
    }
    if let Some(pool) = &pool_opt {
        let _ = crate::pg::living_world::record_relationship_delta(
            pool,
            target_id,
            pid,
            "talked",
            1,
            "player_talk",
            now,
        );
        let _ = crate::pg::living_world::record_relationship_delta(
            pool,
            pid,
            target_id,
            "talked",
            1,
            "player_talk",
            now,
        );
        let stance_str = match decision.choice {
            NpcStanceChoice::Accept => "Accept",
            NpcStanceChoice::Refuse => "Refuse",
            NpcStanceChoice::Deflect => "Deflect",
            NpcStanceChoice::Bargain => "Bargain",
        };
        let effects = serde_json::json!({
            "stance": stance_str,
            "confidence": decision.confidence,
            "latency_ms": decision.latency_ms,
        });
        let _ = crate::pg::living_world::record_player_world_action(
            pool,
            pid,
            "talk",
            target_id,
            player_room,
            "success",
            effects,
            now,
        );
    }
    let player_name = db::get_entity(pid)
        .ok()
        .flatten()
        .map(|e| e.display_title)
        .unwrap_or_else(|| "遊歷客友".to_string());
    crate::npc::causal_core::mutate_npc_from_player_action(
        target_id,
        pid,
        &player_name,
        player_room,
        "talk_exchange",
        player_room,
        0,
        now,
    );
    let _ = event::append(now, pid, "talk", target_id);
    conn.sessions
        .record_player_talk_for_room_echo(pid, player_room, &msg.player_input);
    if ai_succeeded {
        flush_conversation_and_append(pid, target_id, &player_input, &npc_reply, now);
    }
    let _ = adjust_favorability(target_id, pid, FAV_TALK);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_open_trade_input() {
        for input in ["想看看你有什麼", "有什麼可以買？", "我要買東西", "trade"]
        {
            assert!(is_open_trade_input(input), "{input}");
        }
    }

    #[test]
    fn leaves_plain_talk_as_talk() {
        for input in ["有什麼事嗎", "你好", "（搭話）", ""] {
            assert!(!is_open_trade_input(input), "{input}");
        }
    }

    #[test]
    fn test_is_asking_name() {
        for input in [
            "你叫什麼名字",
            "請問貴姓？",
            "不知如何稱呼？",
            "你是誰",
            "敢問尊姓大名",
            "請問芳名",
        ] {
            assert!(is_asking_name(input), "{input}");
        }
        for input in ["你好", "今天天氣真好", "差事", "買東西"] {
            assert!(!is_asking_name(input), "{input}");
        }
    }

    #[test]
    fn test_resolve_npc_real_name_fallback() {
        let ch: Character =
            serde_json::from_str(r#"{"id":"npc_dk1wcrabdr6f","kind":"npc"}"#).unwrap();
        let name = resolve_npc_real_name(&ch);
        assert!(!name.is_empty());
        assert!(!name.starts_with("npc_"));
    }

    #[test]
    fn test_infer_npc_obligation() {
        let (kind1, subj1) = infer_npc_obligation("鐵匠阿茂", "正揮錘淬鍊劍胚");
        assert_eq!(kind1, "fuel_shortage");
        assert!(subj1.contains("硬木白炭袋"));

        let (kind2, subj2) = infer_npc_obligation("溫郎中", "在百草診所細心切脈");
        assert_eq!(kind2, "deliver_goods");
        assert!(subj2.contains("山野草藥"));

        let (kind3, subj3) = infer_npc_obligation("老漁夫", "在渡口泊舟修補漁網");
        assert_eq!(kind3, "repair_tool");
        assert!(subj3.contains("水密熟船膠"));

        let (kind4, subj4) = infer_npc_obligation("林邊採集者", "漫步林間");
        assert_eq!(kind4, "deliver_goods");
        assert!(subj4.contains("乾燥柴枝束"));
    }

    #[test]
    fn test_delivery_and_offer_utterance_matching() {
        assert!(is_delivery_intent_utterance("我帶來了木板"));
        assert!(is_delivery_intent_utterance("交給你這束柴"));
        assert!(is_delivery_intent_utterance("拿去用吧"));
        assert!(is_delivery_intent_utterance("幫你修修"));
        assert!(is_delivery_intent_utterance("這包藥材給你"));
        assert!(!is_delivery_intent_utterance("今天天氣真好"));
        assert!(!is_delivery_intent_utterance("你好啊"));

        assert!(is_explicit_offer_utterance("這些給你"));
        assert!(is_explicit_offer_utterance("交給你收著"));
        assert!(is_explicit_offer_utterance("我來幫你修"));
        assert!(is_explicit_offer_utterance("這束柴拿去"));
        assert!(is_explicit_offer_utterance("這塊木料合用嗎"));
        assert!(is_explicit_offer_utterance("這把柴拿去用"));
        assert!(!is_explicit_offer_utterance("請問這裡是哪裡"));
    }

    #[test]
    fn test_accept_utterance_matching() {
        assert!(is_accept_material_utterance("能借我一束柴嗎？"));
        assert!(is_accept_material_utterance("借一塊木板用用"));
        assert!(is_accept_material_utterance("討點柴火"));
        assert!(!is_accept_material_utterance("你好"));

        assert!(is_accept_comfort_utterance("身上好冷走得好累"));
        assert!(is_accept_comfort_utterance("受了傷好渴"));
        assert!(is_accept_comfort_utterance("救命啊"));
        assert!(!is_accept_comfort_utterance("今天好熱鬧"));
    }

    #[test]
    fn test_select_deliverable_item() {
        // 1. 玩家持有柴與板，但言談指定木板
        let item1 = select_deliverable_item(
            "這塊木板給你",
            None,
            |id| matches!(id, "mat_kindling_bundle" | "mat_plank_board"),
        );
        assert_eq!(item1, Some(("mat_plank_board", "樟松木板料")));

        // 2. 言談未指明，但待辦提示急需白炭
        let item2 = select_deliverable_item(
            "我來幫手",
            Some("急需硬木白炭袋添爐"),
            |id| matches!(id, "mat_charcoal_sack" | "mat_iron_scrap"),
        );
        assert_eq!(item2, Some(("mat_charcoal_sack", "硬木白炭袋")));

        // 3. 言談與待辦均未指定，依優先順序回傳
        let item3 = select_deliverable_item(
            "給你",
            None,
            |id| matches!(id, "mat_kindling_bundle" | "craft_wooden_tool_handle"),
        );
        assert_eq!(item3, Some(("craft_wooden_tool_handle", "堅韌硬木工具長柄")));

        // 4. 無任何持有物
        let item4 = select_deliverable_item("給你", None, |_| false);
        assert_eq!(item4, None);
    }

    #[test]
    fn test_forbidden_words_list_includes_legacy_market_terms() {
        assert!(FORBIDDEN_WORDS.contains(&"短市"));
        assert!(FORBIDDEN_WORDS.contains(&"小市"));
        assert!(FORBIDDEN_WORDS.contains(&"\u{65b9}\u{624d}"));
        assert!(FORBIDDEN_WORDS.contains(&"頃刻"));
        assert!(FORBIDDEN_WORDS.contains(&"青籠關"));
        assert!(FORBIDDEN_WORDS.contains(&"110V"));
        assert!(FORBIDDEN_WORDS.contains(&"念場"));
        assert!(FORBIDDEN_WORDS.contains(&"富態"));
    }

    #[test]
    fn test_effective_npc_disposition_and_trust_defaults() {
        let (disp, trust) = effective_npc_disposition_and_trust("test_npc_default_1", "test_player_default_1");
        assert_eq!(disp, 0);
        assert_eq!(trust, 0);
    }

    #[test]
    fn test_three_baton_mentorship_refuse_and_guidance() {
        let pid = format!(
            "p_test_baton_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM entity_lineage_imprints WHERE entity_id = $1", &[&pid]);
        }

        let ch: Character = serde_json::from_str(r#"{"id":"npc_smith_test","kind":"npc","display_title":"鍛造鐵匠","current_activity":"正揮錘打鐵"}"#).unwrap();
        // 1. 非請益文字 -> 回傳 None
        let res1 = handle_three_baton_mentorship_and_obligation(&pid, &ch, "今天天氣不錯", NegotiationStance::Accept, 1000);
        assert!(res1.is_none());

        // 2. 請益文字但在 Refuse 立場下 -> 回傳 None
        let res2 = handle_three_baton_mentorship_and_obligation(&pid, &ch, "請師傅指點錘法", NegotiationStance::Refuse, 1001);
        assert!(res2.is_none());

        // 3. 請益文字且在 Accept 立場下 -> 進入第 1 次師承指點（Baton 1/2/3）
        let res3 = handle_three_baton_mentorship_and_obligation(&pid, &ch, "請師傅指點錘法", NegotiationStance::Accept, 1002);
        assert!(res3.is_some());
        let note3 = res3.unwrap();
        assert!(note3.contains("【行家指點・師承烙印】"));
        assert!(note3.contains("岑氏鍛坊真傳"));
        assert!(note3.contains("岑氏沉腕落錘法"));

        // 4. 連續第 2 次請益 -> 觸發 newly_gifted 親筆授譜
        let res4 = handle_three_baton_mentorship_and_obligation(&pid, &ch, "在下再次請益要訣", NegotiationStance::Accept, 1003);
        assert!(res4.is_some());
        let note4 = res4.unwrap();
        assert!(note4.contains("【親筆授譜】"));
        assert!(note4.contains("《岑氏沉腕落錘法手札》"));

        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM entity_lineage_imprints WHERE entity_id = $1", &[&pid]);
        }
    }
    #[test]
    fn test_qin_shuisheng_watermill_mentorship() {
        let pid = "test_player_qin_mentor";
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM entity_lineage_imprints WHERE entity_id = $1", &[&pid]);
        }

        let ch: Character = serde_json::from_str(r#"{"id":"秦水生","kind":"npc","display_title":"老堰頭水輪石磨首座|秦水生","current_activity":"巡看老堰頭水輪石磨"}"#).unwrap();
        let res = handle_three_baton_mentorship_and_obligation(&pid, &ch, "請師傅指點石磨要訣", NegotiationStance::Accept, 2000);
        assert!(res.is_some());
        let note = res.unwrap();
        assert!(note.contains("【行家指點・師承烙印】"));
        assert!(note.contains("秦氏水輪石磨真傳"));
        assert!(note.contains("老堰頭水輪分動與雙疊石磨聽音辨距法"));
        assert!(note.contains("順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性"));

        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM entity_lineage_imprints WHERE entity_id = $1", &[&pid]);
        }
    }


    #[test]
    fn test_build_player_material_context_queries_dynamic_settlement_quote() {
        let inv = crate::db::inventory::InventorySnapshot {
            items: vec![
                crate::db::inventory::InventoryLine {
                    item_id: "mat_rough_ore".to_string(),
                    name: "粗礦".to_string(),
                    item_type: "material".to_string(),
                    qty: 2,
                    weight: 1.0,
                    sub_total: 2.0,
                    description: String::new(),
                    slot: String::new(),
                    origin: "紅土崁".to_string(),
                },
            ],
            current_weight: 2.0,
            max_weight: 50.0,
        };
        let ctx = build_player_material_context(&inv, "文曜");
        assert!(ctx.contains("粗礦"));
        assert!(ctx.contains("行情每份"));
        assert!(!ctx.contains("庫存 "), "must not expose raw inventory fraction: {ctx}");
        assert!(ctx.contains("鎂"));
    }

    #[test]
    fn test_evaluate_npc_negotiation_stance_quadrants() {
        let empty_inv = crate::db::inventory::InventorySnapshot {
            items: vec![],
            current_weight: 0.0,
            max_weight: 50.0,
        };
        // 1. Debt > 80 leads to Refuse
        crate::npc::debt_interaction::try_credit_purchase("test_debtor_stance_1", "test_creditor_1", "item_debt_test", 100, 0, 50, 10);
        let (stance, desc) = evaluate_npc_negotiation_stance("test_debtor_stance_1", "test_npc_1", "文曜", &empty_inv);
        assert_eq!(stance, NegotiationStance::Refuse);
        assert!(desc.contains("冷淡婉拒"));

        // Clear debt
        crate::npc::debt_interaction::settle_debt_for_actor("test_debtor_stance_1", 200);

        // 2. Settlement has script or shortage leads to Bargain
        let (stance, desc) = evaluate_npc_negotiation_stance("test_neutral_player_1", "test_neutral_npc_1", "文曜", &empty_inv);
        assert!(matches!(stance, NegotiationStance::Bargain | NegotiationStance::Deflect));
        assert!(desc.contains("交涉立場："));
    }

    #[test]
    fn test_build_cognitive_talk_reply() {
        let mut ch: Character =
            serde_json::from_str(r#"{"id":"npc_test_cog","kind":"npc"}"#).unwrap();
        ch.current_activity = "正揮錘淬鍊劍胚".to_string();
        ch.thoughts.push(crate::entity::Thought {
            source: "Work".to_string(),
            delta: 5,
            created_at: crate::game::now_unix(),
            decay_minutes: 180,
            flavor: Some("這柄長劍開刃得格外均勻".to_string()),
        });

        // 1. 詢問姓名回傳 None 由 canonical 接手
        assert!(build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "你叫什麼名字", &ch.current_activity, NpcStanceChoice::Accept, &[]).is_none());

        // 2. 詢問在做什麼／想什麼，結合活動與心思
        let (_, rep1) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "你在做什麼？", &ch.current_activity, NpcStanceChoice::Accept, &[]).unwrap();
        assert!(rep1.contains("揮錘淬鍊劍胚") || rep1.contains("這柄長劍開刃得格外均勻"));

        // 3. 詢問難處且身負義務
        let obs = vec!["急需硬木白炭袋以供鍛爐".to_string()];
        let (_, rep2) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "有何難處？", &ch.current_activity, NpcStanceChoice::Accept, &obs).unwrap();
        assert!(rep2.contains("急需硬木白炭袋以供鍛爐"));

        // 4. 寒暄問候與打聽近況
        let (_, rep3) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "寒暄問候", &ch.current_activity, NpcStanceChoice::Accept, &[]).unwrap();
        assert!(!rep3.is_empty());
        let (_, rep4) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "打聽近況", &ch.current_activity, NpcStanceChoice::Accept, &[]).unwrap();
        assert!(!rep4.is_empty());

        // 5. clean_act 過濾過度贅字
        let transit_act = "正動身前往【文曜・百鍛鐵工坊】，手腳已是熟門熟路地照料爐火";
        let (_, rep5) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "你在做什麼？", transit_act, NpcStanceChoice::Accept, &[]).unwrap();
        assert!(rep5.contains("照料爐火"));
        let transit_road = "正動身前往【南哨・食堂】賒碗清湯";
        let (_, rep6) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "你在做什麼？", transit_road, NpcStanceChoice::Accept, &[]).unwrap();
        assert!(rep6.contains("趕路辦點差事"));
    }

    #[test]
    fn test_universal_entity_live_talk_context_and_cognitive_reply() {
        let mut ch: Character =
            serde_json::from_str(r#"{"id":"npc_tailor_test","kind":"npc","display_title":"布莊掌事"}"#).unwrap();
        ch.current_activity = "正在布莊整理綢緞".to_string();
        ch.inventory = r#"[{"item_id":"craft_wooden_tool_handle","qty":2},{"item_id":"wt-good-012","qty":4}]"#.to_string();
        ch.equipment_slots = r#"{"hold_r":"craft_wooden_tool_handle"}"#.to_string();

        // 設置器物磨耗與磨損
        let _ = crate::pg::living_world::sharpen_vessel_at_grindstone(&ch.id, "craft_wooden_tool_handle", crate::game::now_unix());
        crate::pg::living_world::record_vessel_use(&ch.id, "craft_wooden_tool_handle", 2, 5, crate::game::now_unix());

        // 1. 驗證 build_universal_entity_live_talk_context 產出隨身器物與行囊實況
        let ctx = build_universal_entity_live_talk_context(&ch);
        assert!(ctx.contains("【隨身器物與行囊實況】"), "ctx: {ctx}");
        assert!(ctx.contains("隨身攜有") && !ctx.contains("wt-good-012"), "ctx: {ctx}");
        assert!(ctx.contains("手持【堅韌硬木工具長柄】"), "ctx: {ctx}");
        assert!(ctx.contains("磨損值 2/3"), "ctx: {ctx}");

        // 2. 驗證 build_cognitive_talk_reply 引用真實持有庫存與磨損
        let (_, rep_what) = build_cognitive_talk_reply(
            &ch,
            "布莊掌事",
            "布莊",
            "你在做什麼？",
            &ch.current_activity,
            NpcStanceChoice::Accept,
            &[],
        ).unwrap();
        assert!(rep_what.contains("隨身收納有") || rep_what.contains("磨損已至 2/3"), "rep: {rep_what}");

        let (_, rep_trouble) = build_cognitive_talk_reply(
            &ch,
            "布莊掌事",
            "布莊",
            "有何難處？",
            &ch.current_activity,
            NpcStanceChoice::Accept,
            &[],
        ).unwrap();
        assert!(rep_trouble.contains("磨耗已至 2/3") || rep_trouble.contains("隨身這批"), "rep: {rep_trouble}");

        let (_, rep_trade) = build_cognitive_talk_reply(
            &ch,
            "布莊掌事",
            "布莊",
            "收購",
            &ch.current_activity,
            NpcStanceChoice::Accept,
            &[],
        ).unwrap();
        assert!(rep_trade.contains("隨身正收存有") || rep_trade.contains("磨損達 2/3"), "rep: {rep_trade}");
    }

    #[test]
    fn test_mastered_lineage_imprint_passed_to_other_entities() {
        let pid = format!(
            "p_test_imprint_pass_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let mentor_id = format!("mentor_{pid}");

        // 該實體已習得並精通特定師承
        let (_rec, _) = crate::pg::living_world::record_mentor_imprint_progress(
            &mentor_id,
            "老獵戶長老",
            "順紋走刀破隙訣",
            "運刃順骨縫游走",
            crate::game::now_unix(),
        );
        // 設定進度 >= 3
        let (_rec2, _) = crate::pg::living_world::record_mentor_imprint_progress(
            &mentor_id,
            "老獵戶長老",
            "順紋走刀破隙訣",
            "運刃順骨縫游走",
            crate::game::now_unix(),
        );
        let (_rec3, _) = crate::pg::living_world::record_mentor_imprint_progress(
            &mentor_id,
            "老獵戶長老",
            "順紋走刀破隙訣",
            "運刃順骨縫游走",
            crate::game::now_unix(),
        );

        let mentor_ch: Character = serde_json::from_str(&format!(
            r#"{{"id":"{mentor_id}","kind":"npc","display_title":"獵戶領隊","current_activity":"巡山"}}"#
        )).unwrap();

        let res = handle_three_baton_mentorship_and_obligation(&pid, &mentor_ch, "請師傅指點走刀要訣", NegotiationStance::Accept, 5000);
        assert!(res.is_some());
        let note = res.unwrap();
        assert!(note.contains("順紋走刀破隙訣"), "note: {note}");
    }

    #[test]
    fn test_build_universal_entity_live_talk_context_venue_and_scripts() {
        let ch: Character = serde_json::from_str(
            r#"{
                "id": "npc_mill_clerk",
                "kind": "npc",
                "display_title": "老堰頭・水輪石磨工坊掌事",
                "current_activity": "在石磨前當值櫃檯內側磨米",
                "equipment_slots": "{\"body\":\"店員制服\"}"
            }"#,
        )
        .unwrap();
        let ctx = build_universal_entity_live_talk_context(&ch);
        assert!(
            ctx.contains("當值狀態：身著工作制服、站立於【水輪石磨工坊】當值櫃檯（非本店人員禁止進入區）內側，職司【當值石磨匠師】"),
            "context must include on-duty uniform and counter zone status: {ctx}"
        );
        assert!(
            ctx.contains("場上事件包：【WS-"),
            "context must include event pack id: {ctx}"
        );
        assert!(
            ctx.contains("劇本包【WS-") && ctx.contains("/3 幕"),
            "context must include script pack id and step: {ctx}"
        );
        assert!(
            ctx.contains("關鍵器物："),
            "context must include key objects: {ctx}"
        );
    }
}
