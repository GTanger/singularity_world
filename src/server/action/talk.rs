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
pub const FORBIDDEN_WORDS: &[&str] = &[
    "短市", "小市", "還有", "剛才", "剛剛", "方才", "頃刻", "青籠關", "110V", "念場", "富態", "資料庫", "database", "grid", "debug", "scene_stub", "聖塔",
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

    let room_scripts = crate::game::peninsula_scripts::scripts_for_room(room_name);
    let has_script = !room_scripts.is_empty();

    if scarce_opt.is_some() || has_script {
        let scarce_name = if let Some((_, name, _, _, _)) = &scarce_opt {
            name.clone()
        } else if let Some(sc) = room_scripts.first() {
            let sanitized = clean_catalog_codes(sc.key_objects);
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
                    "「實不相瞞，店裡現貨剛好斷檔見底，我正愁著去總商行提領新貨補倉，眼下實在抽不開身，還請見諒。」".to_string()
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
                    "「手頭工具刃口崩磨得厲害，不進鐵坊重淬根本使不上力，這會兒委實難以接下委任。」".to_string()
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
            crate::store::HuntingPartyPhase::ReturningToMarket => "正滿載獵獲結伴趕往城鎮商行過磅交割分紅",
        };
        parts.push(format!(
            "你隸屬於領隊【{}】率領之【{}】合社獵團（全隊共 {} 人），當前{}，鎖定座標 ({}, {}) 之山海生靈【{}】（預計採獲【{}】運往【{}】交割）。歡迎來客應募入團並肩圍獵、見者有份。",
            party.leader_name,
            party.zone_name,
            party.member_ids.len(),
            phase_desc,
            party.prey_coord.0,
            party.prey_coord.1,
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
        || target.display_title.contains("郎中")
    {
        parts.push("藥房長年熬煮溫脈舒筋油膏與百草清心茶包，常年收購各類山澤草藥，對外供應行旅急救與調養之物。".to_string());
    } else if room_name.contains("藏書")
        || room_name.contains("學院")
        || room_name.contains("印行")
        || target.display_title.contains("書生")
        || target.display_title.contains("學士")
    {
        parts.push("館內典藏半島山川水文志與考工手札，常年收購精製竹麻紙與松煙墨，並供往來學者查考地誌。".to_string());
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
    } else if act.contains("推車") || act.contains("趕往") || act.contains("沿街前往") {
        let p = [
            "稍稍放緩了趕路的步伐，側過身望向你道：",
            "停下腳步抹了一把額角的微汗，和氣地開口道：",
            "整了整隨身攜帶的物件，駐足看向你道：",
        ];
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
            "將手邊剛理好的物料收整入盒，側過身來開口道：",
            "手上的活計未停，只側過身斜睨了你一眼，神情淡定道：",
            "端起桌旁的涼茶潤了潤喉，隨即好整以暇地望向你道：",
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
        let raw_act = target
            .current_activity
            .trim()
            .trim_start_matches("正在")
            .trim_start_matches('正')
            .trim_end_matches(['。', '，', ' ']);
        let act = if raw_act.is_empty() {
            "打理手邊的事項"
        } else {
            raw_act
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
                    Some(format!("「多謝誇獎。不過我這會兒正忙著{act}，若是真想結識，不妨在手頭活計上幫個忙？」"))
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
                    "repair_tool" => Some(format!("「我手頭有件要緊工具（{subj}）磨損得厲害，得送去鐵坊整修，若是有人能代跑一趟就好了。」")),
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
            Some(format!("「你好！我是負責{role}的，現在正{act}。今天天氣不錯，有什麼需要幫忙或打聽的嗎？」"))
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
                "「路過就路過，別套近乎。」",
                "「這事免談，你找別人吧。」",
            ],
            NpcStanceChoice::Bargain => &[
                "「想談可以，先看看你能幫上什麼忙。」",
                "「天下沒有白幫的忙，我們得講究交換。」",
                "「我手頭剛好缺人搭手，你若肯出力，凡事好商量。」",
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
        let reply = match party.phase {
            crate::store::HuntingPartyPhase::Rallying => format!(
                "「好漢來得正巧！我們領隊【{}】正在號召好手（現已聚齊 {} 人），準備前往座標 ({px}, {py}) 合圍【{}】。你既開口相詢，便算入我們合社獵團一份子，待獵得【{}】運回【{}】，按古規見者有份！」",
                party.leader_name,
                party.member_ids.len(),
                party.prey_name,
                party.prey_loot_name,
                party.market_name
            ),
            crate::store::HuntingPartyPhase::SurroundingPrey => format!(
                "「噓——放輕腳步！我們【{}】合社獵團共 {} 人已鎖定座標 ({px}, {py}) 的【{}】，正從八方收攏合圍網。你隨我們一同包抄上去，獵下此獸後【{}】與賞格見者有份！」",
                party.leader_name,
                party.member_ids.len(),
                party.prey_name,
                party.prey_loot_name
            ),
            crate::store::HuntingPartyPhase::TakedownAndSplit => format!(
                "「少廢話，快亮傢伙搭把手！我們正與領隊【{}】在 ({px}, {py}) 合力搏殺【{}】，拿下之後【{}】跟鎂幣紅利當場分你一份！」",
                party.leader_name, party.prey_name, party.prey_loot_name
            ),
            crate::store::HuntingPartyPhase::ReturningToMarket => format!(
                "「哈哈，這趟我們【{}】合社獵團在山裡順利圍下【{}】，割得上好的【{}】，正要趕往【{}】過磅分紅！下輪吹角集結時，你記得一塊兒來！」",
                party.leader_name, party.prey_name, party.prey_loot_name, party.market_name
            ),
        };
        let narr = format_npc_talk_line(target, talk_name, &reply, player_input, "");
        return Some((narr, reply));
    }

    let clean_act = current_activity
        .trim()
        .trim_start_matches("正在")
        .trim_start_matches('正');
    let act_phrase = if clean_act.is_empty() {
        "照應手頭差事"
    } else {
        clean_act
    };

    let ob_opt = crate::store::get_active_npc_obligation(&target.id)
        .map(|(kind, subj)| {
            let clean_subj = if subj.is_ascii() {
                crate::world::materials::material_by_item_id(&subj)
                    .map(|m| m.name.clone())
                    .or_else(|| db::get_item_def(&subj).map(|d| d.name))
                    .unwrap_or_else(|| "手頭差事".to_string())
            } else {
                subj
            };
            (kind, clean_subj)
        })
        .or_else(|| {
            obligation_descs.first().map(|d| ("obligation".to_string(), d.clone()))
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
                    && !s.is_ascii()
                {
                    return Some(s);
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

    let line = if is_asking_what {
        match stance {
            NpcStanceChoice::Accept => {
                if let Some(t) = latest_thought {
                    format!("「我現下正{act_phrase}，心裡正琢磨著『{t}』呢。客官若是好奇，倒可同你細說兩句。」")
                } else if let Some((_, subj)) = &ob_opt {
                    format!("「手頭正忙著{act_phrase}，心裡還掛記著【{subj}】這檔差事。客官這時過來，也是湊巧。」")
                } else {
                    format!("「我這頭正踏實料理著{act_phrase}，趁著精神頭正好，把該辦的辦妥，心裡便舒坦。」")
                }
            }
            NpcStanceChoice::Refuse => {
                format!("「我現下正專心{act_phrase}，手頭活計緊得很，沒工夫閒扯別的，客官自便吧。」")
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
                    format!("「街坊鄰里討生活，誰手頭沒幾件煩難事？我自個兒料理這{act_phrase}便成，不敢勞動客官。」")
                }
                NpcStanceChoice::Refuse => {
                    format!("「這是我自個兒的差事，由我自己擔著，不勞外人插手。」")
                }
            }
        } else {
            match stance {
                NpcStanceChoice::Accept => {
                    format!("「眼下好在手頭這份{act_phrase}還算順暢，客官若真有心，往後多照應著點便足夠了。」")
                }
                NpcStanceChoice::Bargain => {
                    format!("「手頭活計瑣碎，若你能幫著照看一二，我這頭也能輕鬆不少，少不了你的好處。」")
                }
                NpcStanceChoice::Deflect => {
                    format!("「眼下倒也談不上什麼大難處，把這{act_phrase}做紮實就是了。」")
                }
                NpcStanceChoice::Refuse => {
                    format!("「我手腳俐落，沒什麼需要幫忙的，客官請回吧。」")
                }
            }
        }
    } else if is_asking_news {
        match stance {
            NpcStanceChoice::Accept => {
                format!("「客官問起這事？這{room_name}一帶動靜可不小，我適才還聽過路行商私下議論呢。」")
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
                    format!("「客官有心相助，那是再好不過！若是能幫我周全【{subj}】，當真是幫了大忙。」")
                } else {
                    format!("「有差事或物事可談自然好，我這頭正料理著{act_phrase}，客官請直說無妨。」")
                }
            }
            NpcStanceChoice::Bargain => {
                format!("「交換講究各取所需，你若能拿出合適的物事或代勞些雜務，咱們萬事好商量。」")
            }
            NpcStanceChoice::Deflect => {
                format!("「手頭這{act_phrase}正吃緊，商量物事容後再議，眼下暫且顧不上。」")
            }
            NpcStanceChoice::Refuse => {
                format!("「我這頭自顧不暇，暫無這等心思與盤算，客官還是另尋高就吧。」")
            }
        }
    } else {
        // 一般搭話或問候
        match stance {
            NpcStanceChoice::Accept => {
                if let Some(t) = latest_thought {
                    format!("「客官請了。我這會兒正{act_phrase}，適才還在想『{t}』呢，相逢便是有緣。」")
                } else {
                    format!("「客官請了。我這廂正{act_phrase}，得空聊上兩句倒也無妨。」")
                }
            }
            NpcStanceChoice::Bargain => {
                if let Some((_, subj)) = &ob_opt {
                    format!("「有禮了。客官既來了，可願替我探探【{subj}】的眉目？若有著落定有酬謝。」")
                } else {
                    format!("「客官有禮。若想深入結識，往後在本地得多互通有無才是。」")
                }
            }
            NpcStanceChoice::Deflect => {
                format!("「客官請了，我正忙著手頭{act_phrase}的營生，尋常度日罷了。」")
            }
            NpcStanceChoice::Refuse => {
                format!("「客官自便吧，我這頭正{act_phrase}忙碌著，照應不周。」")
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
    let is_asking_wares = ["賣什麼", "多少錢", "有什麼賣", "吃什麼", "菜單", "價錢", "售價"]
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
                        "「炭火跟原牛肉串剛備妥，我這正要推車前往三水口南街，待會兒開火了再來嚐鮮啊！」"
                    } else {
                        "「今天火候差不多啦，炭爐封好了，肉串也都賣清收攤囉！明日午後南街見！」"
                    }
                }
                "wt_vendor_noodle_soup" => {
                    if is_prepping_or_moving {
                        "「大骨高湯剛熬出濃白香氣，我正張羅著推車去南街開張，待會兒見啊！」"
                    } else {
                        "「大湯鍋跟碗筷都洗刷乾淨收攤啦，這會兒歇歇腿，想吃麵明兒個傍晚再來坐！」"
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
                        "「仙草凍與黑糖剛用井泉鎮涼裝甕，我正推著攤車過去南街，待會兒擺妥攤位再為您盛一盞。」"
                    } else {
                        "「冰甕與銅盞皆已洗淨收妥歇息了，夜涼了你也早些安歇，明日午後再來喝茶。」"
                    }
                }
                "wt_vendor_tofu_pudding" => {
                    if is_prepping_or_moving {
                        "「豆漿剛點好鹽滷成凍、黑糖薑汁正溫著呢，我正推車出門去南街，稍候就開賣囉。」"
                    } else {
                        "「桶裡的熱豆花賣完囉，木桶洗刷好在屋裡歇腳泡黃豆呢，想吃熱豆花明日再來找阿伯！」"
                    }
                }
                "wt_vendor_rice_roll" => {
                    if is_prepping_or_moving {
                        "「荷葉糯米飯正悶著香氣呢，我這就推車前往北街早市，想吃早點待會兒過去找我！」"
                    } else {
                        "「早市收攤啦，這會兒挑選完明日的配料正歇著呢，想吃炊飯明兒個清早來北街！」"
                    }
                }
                "wt_vendor_soy_milk" => {
                    if is_prepping_or_moving {
                        "「石磨剛磨出兩大桶濃米漿，我正要推去北街架攤呢，待會兒見！」"
                    } else {
                        "「銅壺跟石磨都擦拭妥當收攤休息啦，想喝香濃溫米漿明日趕早啊！」"
                    }
                }
                "wt_vendor_morning_produce" => {
                    if is_prepping_or_moving {
                        "「菜剛從田裡引泉水沖洗乾淨裝簍，正準備推去早市，待會兒來挑鮮菜啊！」"
                    } else {
                        "「今天摘的鮮菜全都賣完收攤啦，這會兒歇歇腳準備去理田，明早請早囉！」"
                    }
                }
                "wt_vendor_morning_fish" => {
                    if is_prepping_or_moving {
                        "「清晨網到的青殼河蝦剛分盆裝妥，正趕著推去早市，待會兒來看生猛活蝦！」"
                    } else {
                        "「今日的生猛河蝦早已搶購一空收攤啦，木盆都沖乾淨了，想吃河鮮明兒個天亮早點來！」"
                    }
                }
                _ => {
                    if is_prepping_or_moving {
                        "「手頭正張羅著攤車備料出門，待會兒攤位擺好歡迎來捧場啊！」"
                    } else {
                        "「今日攤頭上的東西都賣清收攤啦，這會兒正歇腳呢，明日請早！」"
                    }
                }
            }
            .to_string()
        } else {
            let mut h: i64 = 0;
            for c in target.id.chars() {
                h = h.wrapping_add(c as i64);
            }
            let now_idx = ((game::now_unix().saturating_add(h)).unsigned_abs() as usize / 15);

            let (_item_id, item_name, price_mg, stock) =
                crate::server::action::trade::get_vendor_stall_stock(&target.id)
                    .unwrap_or(("", "招牌小吃", 3, 5));

            match target.id.as_str() {
                "wt_vendor_roast_beef" => match now_idx % 3 {
                    0 => format!(
                        "「來來來，走過路過別錯過！炭火剛起，這【{item_name}】烤得外焦裡嫩、肉汁直冒，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，趁熱吃最過癮！」"
                    ),
                    1 => format!(
                        "「肚子餓了吧？我鄭火旺的烤肉串都是挑最新鮮的原肉現烤，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，來一串暖暖胃！」"
                    ),
                    _ => format!(
                        "「聞到炭火香氣了嗎？剛撒上粗鹽與香料，油脂滋滋作響！每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，要帶幾串趁手啊？」"
                    ),
                },
                "wt_vendor_noodle_soup" => match now_idx % 3 {
                    0 => format!(
                        "「進來坐喔！大骨高湯熬了整整三個時辰，這【{item_name}】熱騰騰又順口，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，暖胃最踏實！」"
                    ),
                    1 => format!(
                        "「吹著風餓了吧？來罔市這喝碗熱湯，現煮的【{item_name}】麵條Ｑ彈，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，馬上為你下鍋！」"
                    ),
                    _ => format!(
                        "「在外奔波辛苦了！切仔麵配顆入味的滷蛋最對味，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，趁熱吃，湯不夠隨時跟我說！」"
                    ),
                },
                "wt_vendor_pepper_bun" => match now_idx % 3 {
                    0 => format!(
                        "「小心燙手啊！剛用鐵鏟從壁爐裡鏟起這【{item_name}】，酥皮脆、肉汁燙，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，趁熱咬一口滿嘴蔥香！」"
                    ),
                    1 => format!(
                        "「我張阿發做餅講究老麵發酵、厚肉重黑胡椒，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，吃過就知道真材實料！」"
                    ),
                    _ => format!(
                        "「剛出爐這爐火候正頂！外皮烤得金黃酥脆，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，小心咬開燙嘴爆汁啊！」"
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
                        "「剛用古法提煉的甘美仙草，沁涼滑順，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，喝一盅消消暑氣吧。」"
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
                        "「清晨剛出鍋的熱炊飯最頂飽！這【{item_name}】每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，要帶一顆路上墊肚子嗎？」"
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
                        "「聞香就知是好米漿！這【{item_name}】剛在銅鍋文火溫熱著，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，買一碗暖胃吧！」"
                    ),
                },
                "wt_vendor_morning_produce" => match now_idx % 3 {
                    0 => format!(
                        "「瞧瞧！剛從堰頭清泉畔採下的【{item_name}】，脆嫩帶露，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，清炒或煮湯都甜得很！」"
                    ),
                    1 => format!(
                        "「自己地裡引山泉種的水芹菜，沒噴半點藥，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，抓一把回去加菜最健康！」"
                    ),
                    _ => format!(
                        "「透早剛拔的鮮蔬最嫩口！這【{item_name}】梗脆葉嫩，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，隨便挑隨便選，包你滿意！」"
                    ),
                },
                "wt_vendor_morning_fish" => match now_idx % 3 {
                    0 => format!(
                        "「看過來！天未亮在水尾寮淺澤網到的【{item_name}】，隻隻青殼鮮活跳動，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，白灼鮮甜無比！」"
                    ),
                    1 => format!(
                        "「活水養出來的河蝦最青！蝦殼薄肉質脆，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，買回去加點薑絲炒一下，下酒最正點！」"
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
                        "「熱騰騰剛備齊的【{item_name}】，每份 {price_mg} 鎂，目前攤上尚餘 {stock} 份，要帶一份嗎？」"
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
        || target.current_activity.contains("踏著夜色")
        || target.current_activity.contains("趕往");

    if is_on_the_road && !is_asking_name(player_input) {
        let ob_suffix = if let Some((kind, _)) = crate::store::get_active_npc_obligation(&target.id) {
            match kind.as_str() {
                "restock_goods" => "，得趕緊去補齊鋪子裡的現貨",
                "repair_tool" => "，得先去鐵坊把傢伙修整妥當",
                "treat_injury" => "，身子有些微恙得先找醫生瞧瞧",
                "repay_debt" => "，手頭票單正趕著去核銷",
                _ => "，手頭正趕著辦事",
            }
        } else {
            "，手頭正趕著辦事"
        };
        let line = match stance {
            NpcStanceChoice::Refuse => format!(
                "「我這會兒{}{ob_suffix}，實在分身乏術，恕不能奉陪了。」",
                target.current_activity
            ),
            NpcStanceChoice::Bargain => format!(
                "「我這會兒{}{ob_suffix}，若有要緊事，長話短說隨我一併走一段吧。」",
                target.current_activity
            ),
            NpcStanceChoice::Deflect => format!(
                "「我這會兒{}{ob_suffix}，前路正忙，你若不急，待我回鋪子再說不遲。」",
                target.current_activity
            ),
            NpcStanceChoice::Accept => format!(
                "「我這會兒{}{ob_suffix}，你若有要事，不妨隨我同行或待我回鋪子再細談。」",
                target.current_activity
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

    let t_id = target.id.as_str();
    let title = target.display_title.as_str();
    let title_short = title.rsplit(['|', '｜']).next().unwrap_or(title).trim();
    let has = |key: &str| -> bool {
        t_id == key
            || t_id.contains(key)
            || title.contains(key)
            || title_short == key
            || title_short.contains(key)
    };
    let mut h: i64 = 0;
    for c in target.id.chars() {
        h = h.wrapping_add(c as i64);
    }
    let now_idx = ((game::now_unix().saturating_add(h)).unsigned_abs() as usize / 15);

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
    let line: Option<&str> = if t_id.contains("陸秉衡") || title.contains("陸秉衡") {
        if is_task {
            Some("「商行今日正需核驗一批發往風舶角的棉麻布匹與鹽票，你若願搭手核對清冊，我讓帳房依行規記你一份酬勞。」")
        } else if is_trade {
            Some("「櫃房新印了一批加厚牛皮存摺與防水鎂幣夾，皆蓋有總商行鋼印，經久耐磨，隨身攜帶最是妥帖。」")
        } else {
            const LINES: &[&str] = &[
                "「年輕人有幹勁。三水口匯聚半島四方客商，我掌商行數十年，最看重的便是『信義』二字。坐下喝杯茶吧。」",
                "「總商行發行之分格存摺與防偽銀票，在半島四鎮分號皆通存通兌。手頭有餘錢不妨在此開戶，出門在外方能踏實。」",
                "「商貿如春水，流通則活，淤塞則弊。你若在各鎮往來，多留意各地物價物候，處處皆是學問。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id.contains("陸知微") || title.contains("陸知微") {
        if is_task {
            Some("「書架高處有幾箱剛從舊倉移來的航海殘卷急需分卷抄錄，你若筆力端正，不妨幫我謄寫幾頁，閣中自備筆墨潤例。」")
        } else if is_trade {
            Some("「門前書案備有竹綿紙精印的《白塔風物誌》與每旬發行之《半島旬報》，亦有黃銅沾水鋼筆，皆可按公價結算。」")
        } else {
            const LINES: &[&str] = &[
                "「你也是來查閱古卷的麼？閣內收羅了先祖渡海以來的四方碑拓與各地採風錄，請輕聲慢步，莫驚動滿室書香。」",
                "「新一期《白塔風物誌》剛編纂完稿，記載了北嶺地脈與白茫洲水文，讀來頗多意趣。」",
                "「字墨有靈，能載山川興衰。若你在外鄉探尋到奇聞異草或古碑拓片，隨時可送來藏書閣與我一同校勘。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if has("沈硯舟") || t_id == "wt_npc_shen_yanzhou" {
        if is_task {
            Some("「剛進的一批活字版《半島旬報》需要折頁壓平，手頭正缺個細心人。你若得閒幫忙整理，書局贈你一卷精裝新刊。」")
        } else if is_trade {
            Some("「架上皆是新印的《白塔風物誌》、每旬發行之《半島旬報》與鋼尖沾水筆，字跡清晰雋永，隨便翻閱看看吧。」")
        } else {
            const LINES: &[&str] = &[
                "「請進。滿架書籍紙墨常駐，剛校印的《白塔風物誌》墨跡剛乾，正是翻閱的好時機。」",
                "「閱讀能明理篤行。城中學者凡有新著，皆在四海書局排印流傳，你可有感興趣的作品？」",
                "「挑本書坐在窗下慢慢讀吧，外頭長街喧囂，唯此間紙墨最能撫平心緒。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id == "wt_npc_qin_mingcheng" || t_id.contains("秦明誠") || title.contains("秦明誠") {
        if is_task {
            Some("「手頭剛送來大圳頭各農莊的水閘分水帳目與糧草消耗表，急需覆核總數。你若通曉計算，隨時可來幫忙平帳。」")
        } else if is_trade {
            Some("「工坊精製的計數板與帳本報表皆在架上，手感極佳，算師學徒皆好此物，需要便取去。」")
        } else {
            const LINES: &[&str] = &[
                "「精準計算分毫不爽，商務會計教室講求的是實數實利。左手出海、右手歸倉，帳目清爽，心神方得安定。」",
                "「各工坊每季器物耗料與出貨工資皆在此匯總。數字不會騙人，經營之道全在精細籌謀。」",
                "「若想研習商務會計實務，先學會心口如一、落筆有節。核算嚴謹，方顯功底深厚。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id == "wt_npc_zhao_guanxing" || t_id.contains("趙觀星") || title.contains("趙觀星") {
        if is_task {
            Some("「觀星台需記錄連續三日的午正日影與子夜星交角，若你願在此值守半日記錄刻度，學院評議廳定有酬謝。」")
        } else if is_trade {
            Some("「我親手校準的黃銅隨身星象指北羅盤尚有餘珍，嵌夜光貝母刻度，長途遠行不迷方向，可為你結緣。」")
        } else {
            const LINES: &[&str] = &[
                "「三百六十星軌銅儀測天地度數，觀星望氣，方知四時循環。星宿雖遠，其度可推；人事雖繁，其理如一。」",
                "「白塔神山之巔雲霧晨聚暮散，其間流光往往映照半島風候潮汐。細推其規律，自能趨吉避凶。」",
                "「登高望遠，胸襟自朗。若在此台仰望星河，心中紛擾雜念自會消解無形。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id == "wt_npc_lin_shizhong" || t_id.contains("林時中") || title.contains("林時中") {
        if is_task {
            Some("「正有一批蓋好急印的公文需送交西門韓氏貨運行配給早班快騎，你若腳步俐落，煩請替我跑這一趟。」")
        } else if is_trade {
            Some("「案頭備有三水口校時黃銅懷錶、岑氏黃銅發條座鐘與紅松脂封蠟委託單，皆走時極準，任君挑選。」")
        } else {
            const LINES: &[&str] = &[
                "「鐘樓一日鳴鐘六次，辰正開市、戌正收更，半刻不容差池。全城作息信賴此音，我守這口銅鐘已歷三十載。」",
                "「樓下快騎驛使日夜馳騁，一袋緊急手信關係百里外農桑商訊。凡入鐘樓者，須知惜時如金。」",
                "「發條齒輪與旋轉日晷乃鐘樓之骨髓，日日需添山茶油養護，方能轉動無聲、報時準確。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id == "wt_npc_cen_yunye" || t_id.contains("岑雲野") || title.contains("岑雲野") {
        if is_task {
            Some("「堂內幾面卸力石靶受力頗深需要換綁新鞣厚牛皮，你若身手敏捷，搭手幫忙綁紮加固，我教你一套防身卸力架勢。」")
        } else if is_trade {
            Some("「架上備有熟牛皮裹革的練習重木刀與熟銅卸力護腕，耐打耐磨，初習身法必不可少，作價皆公道。」")
        } else {
            const LINES: &[&str] = &[
                "「筋骨舒展、沉肩墜肘！習武並非好勇鬥狠，而在於調和周身氣脈、遇力卸力，此乃半島先民安身之本。」",
                "「分家巷習武堂重在根基扎實。一柄重木刀每日揮擊五百回，手穩心定，方能在山林曠野中應對萬全。」",
                "「看你步履沉穩，是個可造之材。若有餘暇，不妨換上軟革護腕，隨我對拆幾招。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id.contains("岑守岳") || title.contains("岑守岳") {
        if is_task {
            Some("「新運到的二十塊生鐵錠急需搬至炭砧旁碼整齊，學徒手腳慢，你若肯出一把子力氣，我按斤付你現鎂！」")
        } else if is_trade {
            Some("「岑氏精鍛護手獵刀與羊角鍛錘就擺在木架上，刀身冷光如霜，拿在手裡沉實合手，相中哪件儘管開口！」")
        } else {
            const LINES: &[&str] = &[
                "「呼！火爐炭氣嗆人，退後兩步！三號坑精鐵質地堅硬，需借萬斤水錘反覆折疊鍛打，方能逼出雜質。」",
                "「分家巷大房這座鍛造爐，打過百工翻土犁，也鍛過各鎮獵隊的斬木刀。手藝不欺人，鐵砧上見真章！」",
                "「山上二房那些倔脾氣總說老法子最好，我偏要用新式水槽淬火，鍛出的獵刀韌性翻倍，看他們服是不服！」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id.contains("岑拓") || title.contains("岑拓") {
        if is_task {
            Some("「我正需三匣細目研磨石漿粉來拋光刀柄，百工水槽那邊有剛沉澱好的石泥，能幫我舀一桶送來麼？」")
        } else if is_trade {
            Some("「前台擺著我新造的導能測紋石板、卡式便攜爐、黃銅水栓與松木家用工具箱，皆是民生百工合用器物，隨意挑選。」")
        } else {
            const LINES: &[&str] = &[
                "「這柄獵刀我特意加了雙道放血槽，護手用黃銅配重，劈砍時手腕極省力。你覺得握感如何？」",
                "「我打算下月抽空上趟半嶺鎮，把新畫的翻土犁草圖拿給二叔公瞧瞧，兩房手藝若能合壁，何愁器物不精？」",
                "「做鐵匠看似粗笨，實則心細如髮。淬火時水溫多燙一度、火色淺淡一分，成品的剛韌度便差之千里。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id.contains("溫懷瑾") || title.contains("溫懷瑾") {
        if is_task {
            Some("「明日開講需分發三十份《白塔千字文》學童讀本，案几前尚欠裝訂線繩，你若有閒暇，可助一臂之力。」")
        } else if is_trade {
            Some("「學院墨寶皆用竹綿紙印刷，亦有學生常備之黃銅沾水鋼筆與溫潤墨水，文雅精美，可隨緣取閱。」")
        } else {
            const LINES: &[&str] = &[
                "「學院講究的是把原理弄懂、落實到生活裡。半島立城以來，正是靠著公開教學與技術累積，社會才能一直進步。」",
                "「學生如果只會背書而不懂實際產銷與實務，終究只是空談。知微在藏書閣編纂風物誌，晴兒在診所看診，都是學以致用。」",
                "「剛來文曜嗎？如果對學院的基礎課程或各地人文有興趣，隨時可以問我。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id.contains("溫晴") || title.contains("溫晴") {
        if is_task {
            Some("「剛烘乾了一批北山光葉草，需借石碾搗成細末以備熬膏。你若手頭得空，幫我推幾圈碾子可好？」")
        } else if is_trade {
            Some("「隨身防潮急救木匣裡配齊了止血水苔、止痛敷散與消毒草油，出門必備，我這隨時可替你備上一副。」")
        } else {
            const LINES: &[&str] = &[
                "「請慢些坐下。容我先看一眼舌苔與脈息……氣血略有虧虛，可是在外奔波受了山風涼氣？」",
                "「山風草配北山清泉熬成的舒筋藥膏最能拔除關節寒濕。凡進山涉水之人，隨身備上一盒總有益處。」",
                "「採藥要懂天時，雨前收葉、霜後挖根。診所裡的藥材皆是我親手在北嶺和黑沼外緣採集精選的。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id.contains("柯成器") || title.contains("柯成器") {
        if is_task {
            Some("「門前幾根新裁的雲杉方料需抹上熟桐油防裂，學徒隨車隊送貨去了，你若能搭把手刷勻，工錢少不了你的。」")
        } else if is_trade {
            Some("「寒水玉雙層保鮮石櫃、旋蓋夜光貝母台燈、烏木白貝圍棋盤與松香木梳都在架上，防潮防蠹，看中自可帶走。」")
        } else {
            const LINES: &[&str] = &[
                "「咔嗒！聽這榫卯合榫的一聲脆響，嚴絲合縫，不著一釘一鉚，這便是百年木器的底氣！」",
                "「北山雲杉木性柔韌、松香良木氣味避蟲，打成箱櫃百年生香。挑木料如相人，木紋正，器型才立得正。」",
                "「我這雙手刨了五十年的木板，摸一摸就知含水幾分。做傢俱急不得，木頭需陰乾三年方能下刨。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id.contains("蘇錦心") || title.contains("蘇錦心") {
        if is_task {
            Some("「剛裁製好的幾件外套急需以手工縫綴玄武岩石扣，你若手巧心細，替我綴好這幾副排扣，折抵工費或給現鎂皆可。」")
        } else if is_trade {
            Some("「素面玄武岩扣剪裁外套、黑沼防雨連帽風衣皆有現貨，各色尺碼齊全，試穿合身便可直接交割。」")
        } else {
            const LINES: &[&str] = &[
                "「請進。撫摸一下這匹大圳細棉麻，質地柔軟透氣，貼身裁件常服，盛暑天行路也不生悶汗。」",
                "「黑沼樹膠防雨風衣是我坊裡的招牌，外塗三層密膠、內襯透氣薄棉，狂風驟雨打在身上順水滑落，滴水不沾。」",
                "「衣裝乃行者門面。無論研究員還是走南闖北的商旅，換上一身得體耐磨的剪裁外套，整個人便精神三分。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id == "wt_npc_zhou_yidao" || t_id.contains("周一刀") || title.contains("周一刀") {
        if is_task {
            Some("「柴火灶那邊溫水大桶需添滿清冽井水，我這手頭正替客人修面走不開，幫忙挑兩桶水進來，免費替你修面一次！」")
        } else if is_trade {
            Some("「特製的精鋼摺疊修容剃刀與牛骨柄豬鬃牙刷，皆是隨身洗漱佳品，鋼口鋒利耐用，買上一把受用多年。」")
        } else {
            const LINES: &[&str] = &[
                "「哈哈，快請坐上紫銅躺椅！熱毛巾敷透鬚角，老周這把精鋼摺疊剃刀保證又快又順，絕不刮破半點皮！」",
                "「講堂街的學生研究員、公議堂的辦事員，哪一個修剪打理不是在我手底下拾掇的？清清爽爽才見神采！」",
                "「修面要腕穩，洗頭要水溫恰好。紫銅水管裡的井水常年溫熱，躺下歇息片刻，包你神清氣爽。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id == "wt_npc_xu_agan" || t_id.contains("許阿甘") || title.contains("許阿甘") {
        if is_task {
            Some("「高木架頂上那兩大罐黑糖麥芽糖，阿婆手腳矮夠不著，好心年輕人幫我抱下來放在櫃檯上，阿婆請你吃甜餅！」")
        } else if is_trade {
            Some("「廣口玻璃罐裡的酸梅糖、三號坑防潮紅磷火柴盒、夏季驅蚊艾草蚊香盤都在這，零錢隨意付，都是街坊價。」")
        } else {
            const LINES: &[&str] = &[
                "「哎唷，年輕人肚子餓了沒？來含一顆金桔酸梅糖，酸甜生津，解渴又提神，阿婆自己用土冰糖醃的哩！」",
                "「分家巷這條巷子我看了六十年囉，誰家添丁、誰家置產過契，阿婆心裡像明鏡似的。歇歇腳再走吧！」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id == "wt_npc_ma_buxue" || t_id.contains("馬補靴") || title.contains("馬補靴") {
        if is_task {
            Some("「爐邊新熬的桐油皮膠剛好火候，幫我用大蒲扇扇風散散煙氣，待會兒替你把隨身行靴的開膠邊角黏補牢實。」")
        } else if is_trade {
            Some("「厚魚油防浸牛皮行路靴與韌竹骨桐油摺疊晴雨傘皆是現貨，尺碼齊備，過客穿上即可啟程，防風防雨保平安。」")
        } else {
            const LINES: &[&str] = &[
                "「鞋底磨薄了可不行。西門出去走長路，碎石灘和泥濘路最傷腳骨，加釘一層熟牛皮掌，穩健耐磨能多走千里路。」",
                "「傘骨要選北山老竹，浸透桐油的厚棉紙經得起狂風掀扯。晴可遮烈日、雨可擋暴注，這才叫晴雨傘。」",
                "「我手裡的蜂蠟松脂防水膏是祖傳配方，靴縫抹上一圈，踩進積水窪半點不滲。手藝人不做虛活。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id.contains("陳萬福") || title.contains("陳萬福") {
        if is_task {
            Some("「後廚幾籮筐剛送來的水洗脆瓜需切條醃醬菜，廚工忙著和麵，你若刀法純熟，幫我切上一案板，這碗牛肉湯算我請客！」")
        } else if is_trade {
            Some("「大釜熬原牛骨高湯、現磨溫米漿、木桶鹽滷嫩豆花皆滾燙備妥，亦可打入保溫瓶帶走，幾枚鎂幣管飽管暖！」")
        } else {
            const LINES: &[&str] = &[
                "「哈哈！快往裡坐！聞到這鑊氣沒有？紅鐵厚底大釜慢火煨燉的大角原牛骨高湯，整整滾了八個時辰！」",
                "「開食堂四十年，老陳就信一條：做飯要捨得下料，米要新米、肉要鮮肉，街坊行商吃得滿意，生意自然紅火！」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id.contains("白泉生") || title.contains("白泉生") {
        if is_task {
            Some("「西側石池的暗管過濾竹網到了清理時辰，你若不怕水熱，幫我撈出池底殘留的松脂浮沫，這輪早湯免你浴資！」")
        } else if is_trade {
            Some("「架上有北山茶籽松脂香皂與大圳長絨蜂格棉面巾，吸水柔潤，買上一份進池，包你洗得通體舒暢。」")
        } else {
            const LINES: &[&str] = &[
                "「舟車勞頓，身心俱疲了吧？快進熱湯池泡上一刻鐘！這泉水引自老堰頭活脈，底層鋪滿寒水玉，最能活絡筋骨。」",
                "「浴場用的是北山茶籽加松脂熬煉的手工香皂，去垢不傷膚，洗完身上泛著淡淡山林松香，清爽怡人。」",
                "「溫晴醫生常說，泡湯乃平民溫養第一良方。發一身透汗、飲半盞清茶，什麼鬱結勞碌全給融化了。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id.contains("沈清河") || title.contains("沈清河") {
        if is_task {
            Some("「二樓東字號上房的客人剛結帳退房，手頭助手正忙著烹茶，若能幫我將乾淨枕蓆換好鋪平，茶水點心算我奉送。」")
        } else if is_trade {
            Some("「東橋茶屋特選山風草烘青茶葉包、熱煮鮮奶茶皆可外帶，客房旋蓋夜光貝母調光台燈亦有全新行裝版可售。」")
        } else {
            const LINES: &[&str] = &[
                "「請入座。這壺鍋煮鮮奶茶用了牛埔原牛乳與霜林烘青茶，火候慢煨，茶香醇厚奶香濃郁，最宜撫慰羈旅之情。」",
                "「東橋乃文曜咽喉，南北商旅皆在此歇腳。一局圍棋、半盞清茶，天下山海風物盡在言笑之中。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id.contains("韓鐵山") || title.contains("韓鐵山") {
        if is_task {
            Some("「一輛開往半嶺鎮的大車正要封索，尚缺一人幫忙拉緊壓車大繩扣，你膀子力氣若足，搭把手立領十鎂辛苦錢！」")
        } else if is_trade {
            Some("「雙肩厚帆布遠行背囊與雙層烏木內膽保溫瓶皆是長途必備硬貨，容量大、保溫久，旅人獵戶必帶一具。」")
        } else {
            const LINES: &[&str] = &[
                "「嘿！西門聯運車隊，風雨無阻！無論是走風舶角的大宗海貨，還是運往大圳頭的新麥礦石，咱韓家車隊從不誤期！」",
                "「出遠門行囊要緊湊。隨身背囊要用雙肩加厚帆布，防雨布套扣緊，遇上海嵐濃霧或顛簸山徑才不會散架。」",
                "「草獸性子溫順但耐力驚人，日行百里不喘粗氣。若想搭車去外鎮，先到票檯開好路單，時辰一到準時發車！」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id == "wt_npc_lu_jingxuan" || t_id.contains("陸敬軒") || title.contains("陸敬軒") {
        if is_task {
            Some("「書匣內新到了一批拓印石經，需按順序編入課架。你若有閒心，幫我整理歸位，我會備清茶點心相待。」")
        } else if is_trade {
            Some("「堂內備有國小基礎國語課本、竹綿紙牛皮記事手札與精鋼尖沾水筆，字跡端正明晰，文友若需自可兌購。」")
        } else {
            const LINES: &[&str] = &[
                "「基礎教育，乃立世之基。引導孩童端正執筆、朗讀讀本，看著那一張張純真面龐漸知明理，便是我平生最大快事。」",
                "「文曜乃半島文脈所繫，學童自幼誦讀渡海先烈開闢山川之誌，知曉粒米皆出稼穡之辛，長大後方為社會棟樑。」",
                "「你看起來見多識廣，若願在堂前為學童講述幾句外鄉山川險奇、行路見聞，必能開拓孩子們的視野。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id == "wt_npc_lu_changgui" || t_id.contains("陸長貴") || title.contains("陸長貴") {
        if is_task {
            Some("「正需將三份審定好的坊市衛生公約張貼至西門告示欄與食堂門首，你若順路，幫我跑腿張貼一番，公議堂發放津貼！」")
        } else if is_trade {
            Some("「公議堂紅印屋契與租約摺、每旬發行之《半島旬報》皆在案頭，手續合規即可開印簽發。」")
        } else {
            const LINES: &[&str] = &[
                "「哎呀你好！在文曜安家落籍或有公事調解，找我老陸就對了！四坊巷陌每塊界石、每張房契，我都熟記於心。」",
                "「若是看中了西門或分家巷的石砌家屋，先簽了紅印屋契，領取黃銅門牌鑰匙，隨時便能生火入厝！」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if t_id == "wt_npc_lu_chengli" || t_id.contains("陸承禮") || title.contains("陸承禮") {
        if is_task {
            Some("「櫃內《全戶總譜》第十七卷有五頁邊角受潮起皺，需以重木壓平並謄抄存根，你若字跡端正，可受聘繕寫。」")
        } else if is_trade {
            Some("「案前備有標準公議堂紅印屋契與租約摺，亦有專用螺紋瓶黑墨水，字跡歷數十年不退色，隨時可辦理開具。」")
        } else {
            const LINES: &[&str] = &[
                "「請出示身份憑信與定居條目。凡落籍文曜坊市者，必先登載於《半島全戶總譜》，朱墨畫押，留底備查。」",
                "「屋契乃立身安居之憑券。加蓋公議堂硃砂鋼印與騎縫花押，受全城通事保護，不可偽冒，不可毀損。」",
                "「契約條款皆白紙黑字寫得明白。田土面積、界石坐標、租期稅賦，一筆不差，此乃文曜立城之基石。」",
            ];
            Some(LINES[now_idx % LINES.len()])
        }
    } else if room_name.contains("鮮魚")
        || room_name.contains("魚市")
        || room_name.contains("三水河鮮")
        || room_name.contains("早市鮮魚拍賣場")
        || room_name.contains("風舶早市菜場")
        || room_name.contains("北灣早市菜場")
        || talk_name.contains("魚販")
    {
        const LINES: &[&str] = &[
            "「晨光剛透，風舶外海的銀鱗土魠魚與老堰頭肥草魚剛卸船！冰鮮競價，手慢可就搶不到了。」",
            "「三水河網直通西海，這水尾寮青殼蝦活蹦亂跳，剛進早市就引來各大食堂掌勺競標。」",
            "「泊位邊水汽重，買魚要看魚鰓成色。只要是我們風舶拍賣場出具的魚簍木牌，尾尾保鮮！」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("乾貨")
        || room_name.contains("海味")
        || room_name.contains("鹽漬")
        || room_name.contains("水產乾貨")
    {
        const LINES: &[&str] = &[
            "「烏石鼻乾製海貝與風櫃尾野生紫菜，曬足整整七日海風，煲湯入味最是鮮美醇厚。」",
            "「外海風急浪大，船隊遠航必備鹽漬白帶魚與脫水蝦米，耐放半年不壞，吃時用溫水泡發即可。」",
            "「乾貨儲庫通風防潮全靠頂層木百葉，若要大宗調撥發往西境，憑三水商行驗單立提。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("醬園")
        || room_name.contains("醋坊")
        || room_name.contains("陳記醬園")
    {
        const LINES: &[&str] = &[
            "「老罈黑豆醬油慢釀三年，揭開陶缸蓋滿街飄香；半嶺烏醋入菜生津，是文曜老店看家滋味。」",
            "「紅麴米醋酸香柔潤，配羊肉湯餅或海鮮最解腥膩。你若帶乾糧，不妨打一小竹筒醋汁備用。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("酒肆")
        || room_name.contains("紅爐烈酒")
        || room_name.contains("烈酒大飯鋪")
        || room_name.contains("沈家老酒")
        || room_name.contains("老酒坊")
        || room_name.contains("盤查酒肆")
    {
        const LINES: &[&str] = &[
            "「大釜紅爐烈火正旺，封缸陳年米酒滾燙下喉！走長途的行商與打鐵師傅，進門先來兩大碗！」",
            "「北嶺山果甜釀配大塊燉牛肉，酒香夾著肉香，任憑門外寒風呼嘯，屋裡熱氣熏得人面紅耳赤。」",
            "「入城盤查先歇歇腳，喝碗溫酒定定神。西關卡哨查得嚴，但只要身家清白，飲飽便好上路。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("羊肉湯餅")
        || room_name.contains("麵食")
        || room_name.contains("西門麵食鋪")
        || room_name.contains("乾糧")
        || room_name.contains("乾糧備製作坊")
    {
        const LINES: &[&str] = &[
            "「大碗羊肉湯餅熱氣騰騰！北嶺鹽醃草飼羊肉慢燉化渣，手擀麵片筋道吸汁，撒一把野薄荷碎，痛快！」",
            "「大圳頭高筋白麵切得細勻，下鍋滾三滾便撈起，配上一勺熬透的熟牛油辣子，行客吃了腳下生風！」",
            "「出西門走戈壁駝道，隨身必帶乾烙饃與脫水麵筋。耐嚼扛餓，在沙漠裡揣在懷裡十天不餿。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("茶棚")
        || room_name.contains("茶飯館")
        || room_name.contains("水手茶飯館")
        || room_name.contains("工匠大食堂")
        || room_name.contains("大食堂")
        || room_name.contains("食堂")
        || talk_name.contains("跑堂")
        || talk_name.contains("大廚")
        || talk_name.contains("廚師")
    {
        const LINES: &[&str] = &[
            "「大食堂飯鐘一響，千號工匠齊奔食案！厚釜原牛骨湯配十八摺肉包，熱氣騰騰管飽有力氣！」",
            "「碼頭茶棚倚江而建，粗瓷碗泡著高山濃茶。聽水手擺談外海鯨蹤與河道暗礁，最是長見識。」",
            "「切仔湯麵配滷蛋，一勺油蔥酥香飄半條街。靠岸水手吃飽抹嘴，轉身就去船塢扛大包。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("鹽糧")
        || room_name.contains("糧倉")
        || room_name.contains("通濟河運總倉")
        || room_name.contains("戰備糧草")
        || room_name.contains("儲倉")
        || room_name.contains("草料行")
        || room_name.contains("鹽巴草料行")
        || room_name.contains("粗鹽")
    {
        const LINES: &[&str] = &[
            "「風舶白霜海鹽堆垛如雪山，大圳新米一袋袋入庫。通濟總倉連通內河，一日進出糧船不下百艘。」",
            "「戰備糧草倉四面石垣厚重，通風隔潮滴水不漏。城防軍備與各鎮應急，全指望這片地下防潮窖。」",
            "「草料壓捆機手搖絞緊，壓出的乾草磚緊實耐運。駝獸沿途嚼著草磚喝鹽水，走千里也不乏力。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("宣紙")
        || room_name.contains("青檀宣紙")
        || room_name.contains("造紙")
        || room_name.contains("竹綿造紙精廬")
        || room_name.contains("紙料儲庫")
        || room_name.contains("善本紙料")
    {
        const LINES: &[&str] = &[
            "「青檀表皮經三年水浸日曬，木臼舂爛後配竹綿抄造。這紙張細膩勻整、落墨如雲，百年不脆！」",
            "「簾床入水輕搖，紙漿如蟬翼鋪展。烘紙壁火溫要求極嚴，過熱則裂，微冷則澀，全在手感掌握。」",
            "「藏書庫善本紙料皆用花椒防蛀，青檀宣紙整捆入樟木箱封存，專供學院修撰與輿圖抄繪之用。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("徽墨")
        || room_name.contains("松煙徽墨")
        || room_name.contains("端溪石硯")
        || room_name.contains("石硯")
        || room_name.contains("宣筆")
        || room_name.contains("制毫")
        || room_name.contains("筆墨文具")
    {
        const LINES: &[&str] = &[
            "「古松點煙、鹿膠輕膠慢揉，千錘百鍊方成徽墨。研於石硯之上，墨色黝黑沉厚，發出淡淡松香。」",
            "「端溪水岩老坑石硯，石質溫潤如玉，呵氣即可研墨。磨刀不損石，落墨發清響，文人求之若渴。」",
            "「宣筆制毫挑選秋兔脊毛與野狼青毫，兼毫剛柔並濟。無論寫蠅頭小楷還是大字擘窠，提按皆隨心意。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("活字")
        || room_name.contains("印書")
        || room_name.contains("印行")
        || room_name.contains("裝幀")
        || room_name.contains("古籍裝幀")
        || room_name.contains("裱褙")
        || room_name.contains("文房四寶")
        || room_name.contains("藏書")
        || room_name.contains("書局")
        || room_name.contains("書肆")
        || room_name.contains("典藏庫")
        || room_name.contains("圖書典藏庫")
    {
        const LINES: &[&str] = &[
            "「活字銅模逐字揀排，壓印在濕潤竹綿紙上。新一期《半島旬報》排版方定，墨香四溢，晌午即售！」",
            "「藏書別院卷帙浩繁，從前朝航海錄到當今農水格物皆有編目。借閱抄錄需淨手焚香，切莫折角。」",
            "「名家書畫裱褙講究『漿清如水，托如蟬翼』。經熟絹包邊、花綾隔水，殘破舊卷立時重獲生機。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("輿圖")
        || room_name.contains("格物輿圖")
        || room_name.contains("測風")
        || room_name.contains("北辰測風")
        || room_name.contains("會計")
        || room_name.contains("國小")
        || room_name.contains("算學")
        || room_name.contains("曆算")
        || room_name.contains("觀星台")
        || room_name.contains("學院")
        || room_name.contains("蒙學堂")
        || talk_name.contains("學生")
    {
        const LINES: &[&str] = &[
            "「格物輿圖院測量文曜內外地脈，雙筒青銅測量鏡校準星位，繪就的水道深淺圖是各船長保命符。」",
            "「北辰高台上銅測風儀旋轉不休，候風官推算明日信風流向，旗號一掛，整座港口的船隊便知進退。」",
            "「計算精準快速。會計與曆算講堂正推演防洪渠堤壩受力與糧稅分賬，一絲一毫差錯都逃不過算尺。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("斫琴")
        || room_name.contains("古琴")
        || room_name.contains("棋院")
        || room_name.contains("文曜棋院")
        || room_name.contains("鑑藏")
        || room_name.contains("古物鑑藏齋")
    {
        const LINES: &[&str] = &[
            "「老杉良桐陰乾五載，掏膛配金石之聲。指尖撫過絲弦，清越幽遠，如高山空谷流水潺潺。」",
            "「黑子玄武岩，白子老海貝。文曜棋院黑白交錯間，爭的不是一城一池，是天元推衍的大格局。」",
            "「古物鑑藏齋明窗淨几，青銅古鏡與殘卷古玉皆需紫光過目。是真古蹟還是後仿，眼力分毫不差。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("杏林百草醫館")
        || room_name.contains("百草醫館")
        || room_name.contains("醫館")
        || room_name.contains("診所")
        || room_name.contains("採藥棧")
        || room_name.contains("採藥")
        || room_name.contains("藥材行")
        || room_name.contains("藥材中轉站")
        || room_name.contains("百草藥居")
        || room_name.contains("脈絡診所")
        || talk_name.contains("醫師")
        || talk_name.contains("藥師")
        || talk_name.contains("學徒")
    {
        const LINES: &[&str] = &[
            "「黑沼拔毒止血水苔敷於傷口，三息止血；南哨白茫避瘴清心丸含在舌下，入深林不染瘴氣。」",
            "「醫館診脉辨虛實，九寸脈絡銀針順行氣血。無論跌打骨折還是積勞內傷，照方抓藥調理自能康復。」",
            "「採藥客翻越北山絕壁帶回新鮮山草，藥棧天井正分揀陰乾。山氣芬芳，聞一口便覺神清氣爽。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("烘藥房")
        || room_name.contains("百草烘藥房")
        || room_name.contains("香燭")
        || room_name.contains("山風草香燭鋪")
        || room_name.contains("浴場")
        || room_name.contains("溫水浴場")
    {
        const LINES: &[&str] = &[
            "「烘藥房火道均勻，溫石百草溫灸盒借火性引藥力通透骨髓，最去行商老客筋骨陰濕寒氣。」",
            "「艾草香盤佐以野薄荷與松脂，點上一盤，蚊蚋退避、室內清爽，常客長途露宿行囊裡必備數捆。」",
            "「引地熱溫泉水入池，祛寒草木湯包浸透熱浪。泡過半個時辰，周身毛孔舒張，筋骨疲乏盡消。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("重鐵水錘")
        || room_name.contains("大鍛坊")
        || room_name.contains("鍛造")
        || room_name.contains("鍛修")
        || room_name.contains("車軸鐵箍")
        || room_name.contains("磨刀")
        || room_name.contains("磨刀利刃")
        || room_name.contains("鐵蹄釘掌")
        || room_name.contains("釘掌鋪")
        || talk_name.contains("鐵匠")
        || talk_name.contains("鍛徒")
    {
        const LINES: &[&str] = &[
            "「水錘借水力轟鳴砸下，震得地面微顫！紅鐵出爐火星四濺，岑氏大匠千錘百鍊，造就精鋼刃口。」",
            "「車軸鐵箍經大爐淬火熱套，收緊後與硬木車輪合為一體，任憑山路坎坷，萬里馳騁決不崩裂。」",
            "「打鐵蹄修磨掌，生鐵蹄鐵貼合馬蹄弧度，釘入三寸不傷蹄肉。換上新掌，馱獸奔走如踏平地。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("農具")
        || room_name.contains("大型農具工具行")
        || room_name.contains("五金")
        || room_name.contains("銅鐵五金")
        || room_name.contains("鎖鑰")
        || room_name.contains("鎖簧")
        || room_name.contains("器物交易場")
        || room_name.contains("交易場")
    {
        const LINES: &[&str] = &[
            "「省力導能翻土犁鋼口鋒利，入土三分泥浪翻滾；水力大石磨齒槽精修，磨豆磨面細滑如雪。」",
            "「熟銅碰鎖與青銅插銷雙扣暗簧，防撬防鏽經久耐用。門戶要道安上這把鎖，夜不閉戶也心安。」",
            "「器物交易場上各坊名匠齊聚，從袖珍指南針到重型千斤頂一應俱全，現銀過秤、憑單交割。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("武備")
        || room_name.contains("護商武備鋪")
        || room_name.contains("防具")
        || room_name.contains("火浣布防具坊")
        || room_name.contains("火政")
        || room_name.contains("東門火政巡防所")
        || room_name.contains("習武堂")
    {
        const LINES: &[&str] = &[
            "「行商過戈壁山林，護身軟甲與卸力銅護腕是保命行頭。刀刃劈在熟銅上滑開三分，便多一條生路。」",
            "「厚牛皮防燙圍裙防油防刺，火浣布經烈火焚燒不焦。火政巡守手持大鉤水桶，防患於未然。」",
            "「習武堂試招木人被劈得斑駁，拳風破空呼呼作響。出遠門鏢隊招募好手，先在場子裡過上三招！」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("鐘錶")
        || room_name.contains("儀器坊")
        || room_name.contains("精銅鐘錶儀器坊")
        || room_name.contains("琉璃燈")
        || room_name.contains("白塔琉璃燈坊")
        || room_name.contains("鐘樓")
        || room_name.contains("傳訊鐘樓")
        || talk_name.contains("司鐘")
    {
        const LINES: &[&str] = &[
            "「發條齒輪精密咬合，黃銅擺錘秒秒不差。調準懷錶時辰，各路車船驛使方能嚴絲合縫交接。」",
            "「夜光貝母薄片鑲嵌於旋蓋台燈，光線柔和護眼，狂風吹拂亦不搖晃，藏書閣與船艙必備。」",
            "「信使總站信鴿起落、飛騎待發。八方急件過印登記，晨發文曜，暮達半島各哨，號令通行。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("鋸木廠")
        || room_name.contains("連軸鋸木廠")
        || room_name.contains("竹木行")
        || room_name.contains("三水竹木行")
        || room_name.contains("竹木場")
        || room_name.contains("蘆葦竹木場")
        || room_name.contains("竹編")
        || room_name.contains("藤器")
        || room_name.contains("竹編藤器作坊")
        || room_name.contains("木作")
        || room_name.contains("傢俱")
        || room_name.contains("巧木居所")
        || room_name.contains("榫眼")
        || room_name.contains("木器榫眼作坊")
        || room_name.contains("大木作")
        || room_name.contains("車轅大木作")
        || room_name.contains("大木車船坊")
        || talk_name.contains("木匠")
    {
        const LINES: &[&str] = &[
            "「水車連軸帶動原木大鋸，巨木如剖瓜切菜般破成整齊板材，木香撲鼻，是建房造船的骨幹良料。」",
            "「竹編雙層提籃密不漏水，輕巧韌實耐磨耐摔。漁戶裝魚、主婦買菜，用上十年依舊齊整。」",
            "「巧木傢俱純用古法榫卯，燕尾榫、抱肩榫咬合嚴密，不著一顆鐵釘，越是吃重越見緊實。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("石材切割場")
        || room_name.contains("切石")
        || room_name.contains("磚瓦窯")
        || room_name.contains("灰漿磚瓦窯")
        || room_name.contains("木炭窯")
        || room_name.contains("高山木炭窯")
        || room_name.contains("官窯")
        || room_name.contains("青釉官窯")
        || room_name.contains("銅錫")
        || room_name.contains("器皿精煉廠")
        || room_name.contains("生鐵大棧房")
        || room_name.contains("焦炭生鐵大棧房")
        || room_name.contains("生鐵庫")
        || room_name.contains("粗鹽生鐵庫")
    {
        const LINES: &[&str] = &[
            "「半嶺水力切石機水霧濛濛，整塊玄武岩巨石剖切平整，鋪在街面百年車輾馬踏不凹陷。」",
            "「青石磚瓦窯烈焰熊熊，出窯青磚敲之有金石之聲。焦炭生鐵大棧房生鐵錠堆積如山，專供鍛坊。」",
            "「青剛櫟木高溫炭化，燃燒無煙且火力持久，是百工冶煉與城中富戶圍爐取暖的上品燃料。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("染布坊")
        || room_name.contains("三水染布坊")
        || room_name.contains("機杼大染坊")
        || room_name.contains("染纈")
        || room_name.contains("織造")
        || room_name.contains("雲錦織造與服飾坊")
        || room_name.contains("服飾坊")
        || room_name.contains("剪剃鋪")
        || room_name.contains("修傘補靴")
        || room_name.contains("修傘")
        || room_name.contains("補靴")
        || talk_name.contains("繡娘")
    {
        const LINES: &[&str] = &[
            "「機杼聲聲穿梭如流，大圳細棉織就柔軟布匹。扎染絞纈沉入靛藍大缸，出水展曬，雲紋天成！」",
            "「蘇錦心老師親自量體裁衣，玄武岩扣剪裁外套文質彬彬，厚棉麻常服吸汗耐磨，四時得體。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("造船廠")
        || room_name.contains("內河造船廠")
        || room_name.contains("修船塢")
        || room_name.contains("渡口修船塢")
        || room_name.contains("桅帆")
        || room_name.contains("大船桅帆作坊")
        || room_name.contains("帆布麻繩織造坊")
        || room_name.contains("漁網繩索工坊")
        || room_name.contains("修網結繩工坊")
        || room_name.contains("漁具")
        || room_name.contains("簑笠")
        || room_name.contains("漁具簑笠商號")
        || room_name.contains("桐油作坊")
        || room_name.contains("船用桐油作坊")
        || room_name.contains("浮標竹笛作坊")
        || room_name.contains("浮標")
        || talk_name.contains("船夫")
    {
        const LINES: &[&str] = &[
            "「造船塢內雲杉大龍骨架設完畢，桐油海貝灰捻縫密不透水。大船入水之日，半城人都來看熱鬧。」",
            "「深海巨藻絞成粗大船纜，韌性勝似牛筋，驚濤駭浪中扣住巨錨，是遠洋客船全船人的定心骨。」",
            "「修網結繩需心明手巧，麻線經魚油浸泡耐爛抗腐。老漁翁織出的密目網，連蝦苗都漏不出一隻。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("駝馬大驛站")
        || room_name.contains("萬里駝隊大驛站")
        || room_name.contains("駝馬大客棧")
        || room_name.contains("長途行商客棧")
        || room_name.contains("客棧")
        || room_name.contains("旅社")
        || room_name.contains("風塵客商大旅社")
        || room_name.contains("東橋旅社")
        || room_name.contains("客舍")
        || room_name.contains("青雲求學客舍")
        || room_name.contains("歇舍")
        || room_name.contains("行商長途歇舍")
        || (room_name.contains("行館") && !room_name.contains("印行館"))
        || room_name.contains("四方匠師行館")
        || room_name.contains("泊客棧舍")
        || room_name.contains("茶屋")
        || talk_name.contains("茶客")
    {
        const LINES: &[&str] = &[
            "「萬里駝隊在此卸鞍歇馬！驛館後院草料溫水管夠，前堂火爐煮著濃奶茶，洗去一身風塵。」",
            "「客舍床榻整潔乾爽，每間備有防倒風野營柴火爐與保溫瓶。行商過客推杯換盞，互通關山道路。」",
            "「四方匠師齊聚行館交流技藝，南腔北調熱鬧非凡。在此歇宿一晚，各地行情掌故全在耳中。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("皮毛大市")
        || room_name.contains("西南原皮毛大市")
        || room_name.contains("皮毛大貨棧")
        || room_name.contains("皮貨行")
        || room_name.contains("西門皮貨行")
        || room_name.contains("皮帶鞍套")
        || room_name.contains("皮鞘鞍具")
        || room_name.contains("鞍轡")
        || room_name.contains("遠行鞍轡帳篷行")
        || room_name.contains("帳篷")
        || room_name.contains("行囊修補行")
        || room_name.contains("行囊")
        || room_name.contains("行車雨布作坊")
        || room_name.contains("雨布")
    {
        const LINES: &[&str] = &[
            "「西南原送來的雪狼毛領大氅與熟牛皮鞍具，皮質油潤柔軟，披在身上能擋關外徹骨白毛風。」",
            "「雙層防風防水行商帳篷配厚木撐桿，風沙再大也能在戈壁撐起安穩避難所，行商長途必帶。」",
            "「牛皮馬鞍與護膝皆用雙股蠟線縫製，行囊修補師傅手藝精湛，崩了線的背囊片刻修整如新。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("雙輪大車廠")
        || room_name.contains("重載雙輪大車廠")
        || room_name.contains("大車修造場")
        || room_name.contains("車馬輪軸修造場")
        || room_name.contains("長途車行")
        || room_name.contains("西道長途車行")
        || room_name.contains("車馬聯運總站")
        || room_name.contains("聯運貨運行")
        || room_name.contains("聯運")
        || room_name.contains("貨運行")
        || room_name.contains("油膏滑脂鋪")
        || room_name.contains("滑脂")
        || talk_name.contains("搬運工")
        || talk_name.contains("車夫")
    {
        const LINES: &[&str] = &[
            "「重載雙輪大車採用硬木車轅與雙軸滾珠，馱載三千斤生鐵亦行若游水，是通濟大道上的巨無霸。」",
            "「車軸千斤頂輕輕一扳，重車騰空！抹上厚厚一層動物滑脂油膏，輪軸轉動無聲，能多跑千里路。」",
            "「長途車隊每日定時啟程發往半島各鎮。託運箱籠皆掛防水油布，路途再顛簸，貨物完好無損。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("牲畜交易拍賣行")
        || room_name.contains("牲畜拍賣")
        || room_name.contains("獸醫館")
        || room_name.contains("車馬獸醫館")
    {
        const LINES: &[&str] = &[
            "「拍賣場內號子嘹亮！西南原的高頭健馬與耐旱駝獸正當場過眼驗齒，體格健碩，嘶鳴如雷。」",
            "「獸醫老館長專治走獸蹄疾與積熱，草藥調拌料豆灌服，針灸兩穴，立時消腫去乏、胃口大開。」",
            "「螺旋壓捆機壓出的乾草料方磚碼在槽邊，健馬吃飽打個響鼻，渾身皮毛油光發亮。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("貨船大埠")
        || room_name.contains("川流貨船大埠")
        || room_name.contains("北灣貨船大埠")
        || room_name.contains("碼頭匯兌分號")
        || room_name.contains("碼頭匯兌")
        || room_name.contains("巡河哨所")
        || room_name.contains("河工巡防哨居")
        || room_name.contains("哨所")
        || room_name.contains("長途保鏢哨所")
        || room_name.contains("四方護運行總堂")
        || room_name.contains("四方鏢局總堂")
        || room_name.contains("護運行")
        || room_name.contains("鏢局")
        || room_name.contains("關防驗契所")
        || room_name.contains("驗契")
        || room_name.contains("公議堂")
        || room_name.contains("契書")
        || room_name.contains("總商行")
        || room_name.contains("柑仔店")
        || talk_name.contains("帳房")
        || talk_name.contains("書辦")
        || talk_name.contains("通事")
    {
        const LINES: &[&str] = &[
            "「川流大埠貨船一艘接一艘靠岸，起重吊臂沒停過。碼頭匯兌分處可以直接換取鎂幣票據，全半島都能流通。」",
            "「四方護運行總部的護運員經驗豐富，長途車隊隨時注意幹道狀況，有他們隨行押車安穩得多。」",
            "「港務驗單處核對貨物清單與商號印章很仔細，手續辦妥蓋上封章，出城運輸就順暢了。」",
        ];
        Some(LINES[now_idx % LINES.len()])
    } else if room_name.contains("分家巷")
        || room_name.contains("西門街")
        || room_name.contains("講堂街")
        || room_name.contains("石橋")
    {
        const LINES: &[&str] = &[
            "「文曜城裡的街道很平整，作息也很規律。白天各行各業正常營業，到了晚上鐘樓報時收班，走在街上很安心。」",
            "「三水口連通外港，分家巷聚集各類工坊，講堂街則是學院與書局的所在地，城裡機能很齊全。」",
            "「剛來文曜嗎？如果要找各處場所，順著中心樞紐石橋往四面看，各家招牌都很清楚。」",
        ];
        Some(LINES[now_idx % LINES.len()])
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
    if super::is_canonical_named_npc(target)
        || super::trade::target_has_shop_offer(target, &room_name)
    {
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
    ACCEPT_COMFORT_KEYWORDS.iter().any(|kw| input.contains(kw))
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
        ("fuel_shortage", "急需回收熟鐵料塊、修補熟鐵薄片或硬木白炭袋添爐精鍛")
    } else if combined.contains('醫')
        || combined.contains('診')
        || combined.contains('藥')
        || combined.contains('茶')
    {
        ("restock_goods", "急需山野草藥、通痹膏或山風安神草茶包調配療傷")
    } else if combined.contains('漁')
        || combined.contains('網')
        || combined.contains('船')
        || combined.contains('渡')
        || combined.contains('埠')
    {
        ("repair_tool", "急需水密熟船膠、浸油麻繩卷或樟松木板料修固船具")
    } else if combined.contains('織')
        || combined.contains('布')
        || combined.contains('紙')
        || combined.contains('皮')
    {
        ("craft_material", "急需防雨油浸帆布卷、宣紙刀或熟皮護具帶補備物料")
    } else {
        ("supply_shortage", "急需乾燥柴枝束或伐採原木備用")
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
    decision_choice: NpcStanceChoice,
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

    let triggered = (has_obligation
        && (mentions_delivery
            || matches!(
                decision_choice,
                NpcStanceChoice::Accept | NpcStanceChoice::Bargain
            )))
        || explicit_offer;

    if !triggered {
        return None;
    }

    let _ = db::remove_from_inventory(pid, item_id, 1);
    let _ = db::add_to_inventory(target_id, item_id, 1);
    let ob_kind = active_ob.as_ref().map(|(k, _)| k.as_str()).unwrap_or("");
    crate::store::clear_active_npc_obligation(target_id, ob_kind);
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

    push_refresh(conn, pid);

    let origin_str = crate::game::object_ecology::lookup_item_origin(item_id);
    let origin_note = if !origin_str.is_empty() {
        format!("（產自【{origin_str}】）")
    } else {
        String::new()
    };

    Some(TalkWorldEffectOutcome {
        narrative_suffix: Some(format!(
            "\n【言出事成・器物交割】你從行囊取出【{item_name}】{origin_note}遞交對方，當場解決了對方手頭緊迫的待辦要務；對方神色舒展，取出一小袋沉甸甸的導能鎂晶塞入你手中酬謝。"
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
            "\n【立約待辦・條件交換】對方提出了明確的交換條件（{ob_subject}）——若你行囊備有對應的柴木、木板、白炭、鐵料、藥草或料件再次與其交談，即可當場完成交割。"
        )),
        prompt_hint: None,
    }
}

/// C. 在 Accept 立場下給予實質互助支援（柴木物資或溫言款待調養）
fn apply_accept_world_effect(
    conn: &WsConnection,
    pid: &str,
    player_input: &str,
    familiarity_score: i32,
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
    } else if is_accept_comfort_utterance(player_input) || familiarity_score >= 3 {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 12).min(100);
                e.qi = (e.qi + 12).min(100);
            });
        }
        push_refresh(conn, pid);
        Some(TalkWorldEffectOutcome {
            narrative_suffix: Some(
                "\n【同道相助・溫言款待】對方熱情招呼你在近旁歇腳暖手，分予溫熱茶飲，你周身的疲乏與寒意頓時消散不少。".to_string(),
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
            ensure_bargain_obligation(target_id, talk_name, current_activity)
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

    let is_smithing = ["鐵", "鍛", "岑", "礦", "石"].iter().any(|k| domain_source.contains(k));
    let is_culinary = ["廚", "食", "大娘", "麵", "茶", "漁", "海"].iter().any(|k| domain_source.contains(k));
    let is_hunting = ["獵", "衛", "哨", "武", "刀", "巡", "羅", "趙"].iter().any(|k| domain_source.contains(k));

    let (mentor_label, imprint_title, imprint_prose, habit_kind, posture, raw_word_ids, raw_word_chars): (
        &str, &str, &str, crate::pg::living_world::BreathHabitKind, &str, &[&str], &[&str]
    ) = if is_smithing {
        (
            "岑氏鍛坊真傳",
            "岑氏沉腕落錘法",
            "沉腕蓄勁、順著鐵胚紋理落錘，使器物與內息渾然相合",
            crate::pg::living_world::BreathHabitKind::ForgeHeavy,
            "重手",
            &["gang", "chen_tai", "zhong"],
            &["剛", "沉", "重"],
        )
    } else if is_culinary {
        (
            "三水庖廚真傳",
            "三水火候收汁訣",
            "以綿長內息感應鼎釜冷熱交替，鎖住山海靈材本味",
            crate::pg::living_world::BreathHabitKind::TideContinuous,
            "連攜",
            &["shui", "huo_shou", "he_tai"],
            &["水", "火", "合"],
        )
    } else if is_hunting {
        (
            "山海尋隙真傳",
            "順紋走刀破隙訣",
            "運刃順骨縫游走、避實擊虛，手眼身步一氣呵成",
            crate::pg::living_world::BreathHabitKind::ForestLong,
            "起手",
            &["ji_su", "ci", "chuan"],
            &["疾", "刺", "穿"],
        )
    } else {
        (
            "半島百工衡準",
            "百工調息衡準訣",
            "神思澄明、呼吸勻淨，於分寸之間拿捏器物重心",
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            "守勢",
            &["zhun", "ding", "he_tai"],
            &["準", "定", "合"],
        )
    };

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
            "\n【行家指點・師承烙印】{}傾囊相授，為你細說【{}】之奧妙，體會到：「{}」。你依循提點調整吐納，氣息周天漸入佳境。\n【親筆授譜】{}見你領悟精進、根基漸穩，當場將一冊親筆《{}手札》與三枚字印要訣傳授於你！",
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
    let raw_backstory = db::build_identity(target_id);
    let backstory = if target.kind == EntityKind::Npc {
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
    let mut style_examples = pick_style_examples(target_id, 3);
    style_examples.retain(|s| {
        !s.contains("錢袋")
            && !s.contains("再誇")
            && !s.contains("花錢")
            && !s.contains("有啥")
            && !s.contains("啥事")
            && !s.contains("客官")
            && !s.contains("客倌")
            && !s.contains("小二")
            && !s.contains("銀兩")
            && !s.contains("銅錢")
            && !s.contains("掌櫃")
    });
    let mut sensitivity = talk_sensitivity_hint(target);
    sensitivity.push_str(&talk_disposition_hint(target_id, pid));
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
        .map(|o| o.obligation_kind.clone())
        .collect();
    if let Some((kind, subj)) = crate::store::get_active_npc_obligation(target_id) {
        let desc = format!("{}:{}", kind, subj);
        if !obligation_descs.contains(&desc) && !obligation_descs.contains(&kind) {
            obligation_descs.push(desc);
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

    // Step 2: 寫入 PostgreSQL living_world
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
            Some(format!(
                "\n【合社入團・見者有份】你已應募加入領隊【{}】率領之【{}】合社獵團（現共 {} 名獵手）！當前鎖定獵物為座標 ({}, {}) 之【{}】——無論由你親手擊殺或隨獵團合圍收網，皆可依半島古規「見者有份」分得【{}】與交割鎂幣紅利！",
                party.leader_name,
                party.zone_name,
                party.member_ids.len(),
                party.prey_coord.0,
                party.prey_coord.1,
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

    let mut snippets = Vec::new();
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

    let mut room_display_name = super::resolve_action_room_name(pid, target);
    if room_display_name.trim().is_empty() {
        room_display_name = get_room_name(player_room).unwrap_or_default();
    }
    if room_display_name.trim().is_empty() {
        room_display_name = player_room.to_string();
    }
    let mut backstory = format!(
        "你目前所在的當前地點是：【{room_display_name}】（對話時請嚴格以【{room_display_name}】與你當前的具體活動為準，切勿提及不在場的其他關隘或異地）。{}",
        backstory.replace("青籠關", "文曜")
    );
    let venue_ctx = extract_venue_and_role_talk_context(target, &room_display_name);
    if !venue_ctx.is_empty() {
        backstory.push_str(&format!("\n【坊市職責與攤位現況】：{}", venue_ctx));
    }
    if let Ok(Some(player_ent)) = db::get_entity(pid) {
        let inv = db::get_inventory(&player_ent.inventory, player_ent.vit);
        let mat_ctx = build_player_material_context(&inv, &room_display_name);
        if !mat_ctx.is_empty() {
            backstory.push_str(&mat_ctx);
        }
        let (_stance, stance_desc) = evaluate_npc_negotiation_stance(pid, target_id, &room_display_name, &inv);
        backstory.push_str(&format!("\n{}", stance_desc));
    }

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
            Ok(reply) => Ok(reply),
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
                    Ok(reply) => Ok(reply),
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
    };

    let mut ai_succeeded = false;
    let (narrative, npc_reply) = match ai_result {
        Ok(reply) => {
            ai_succeeded = true;
            let narr = format_npc_talk_line(target, &talk_name, &reply, &player_input, pid);
            (narr, reply)
        }
        Err(err) => {
            tracing::warn!("all AI calls failed ({err}), falling back to cognitive / canonical or rule reply");
            if let Some(cog_pair) = build_cognitive_talk_reply(
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
    let should_unlock_name = target.kind == EntityKind::Npc
        && !was_named
        && (asking_name || self_intro || relationship_met || name_revealed_in_text);

    let mut final_narrative = narrative;
    if should_unlock_name {
        let _ = db::set_named(pid, target_id);
        let _ = db::set_observed(pid, target_id);
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
    let _ = event::append(now, pid, "talk", target_id);
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
        super::super::handler::send_place_view(conn, pid);
    }
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
        assert_eq!(kind2, "restock_goods");
        assert!(subj2.contains("山野草藥"));

        let (kind3, subj3) = infer_npc_obligation("老漁夫", "在渡口泊舟修補漁網");
        assert_eq!(kind3, "repair_tool");
        assert!(subj3.contains("樟松木板料"));

        let (kind4, subj4) = infer_npc_obligation("林邊採集者", "漫步林間");
        assert_eq!(kind4, "supply_shortage");
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
        assert!(FORBIDDEN_WORDS.contains(&"方才"));
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
        let ch: Character = serde_json::from_str(r#"{"id":"npc_smith_test","kind":"npc","display_title":"鍛造鐵匠","current_activity":"正揮錘打鐵"}"#).unwrap();
        // 1. 非請益文字 -> 回傳 None
        let res1 = handle_three_baton_mentorship_and_obligation("p_test_1", &ch, "今天天氣不錯", NegotiationStance::Accept, 1000);
        assert!(res1.is_none());

        // 2. 請益文字但在 Refuse 立場下 -> 回傳 None
        let res2 = handle_three_baton_mentorship_and_obligation("p_test_1", &ch, "請師傅指點錘法", NegotiationStance::Refuse, 1001);
        assert!(res2.is_none());

        // 3. 請益文字且在 Accept 立場下 -> 進入第 1 次師承指點（Baton 1/2/3）
        let res3 = handle_three_baton_mentorship_and_obligation("p_test_1", &ch, "請師傅指點錘法", NegotiationStance::Accept, 1002);
        assert!(res3.is_some());
        let note3 = res3.unwrap();
        assert!(note3.contains("【行家指點・師承烙印】"));
        assert!(note3.contains("岑氏鍛坊真傳"));
        assert!(note3.contains("岑氏沉腕落錘法"));

        // 4. 連續第 2 次請益 -> 觸發 newly_gifted 親筆授譜
        let res4 = handle_three_baton_mentorship_and_obligation("p_test_1", &ch, "在下再次請益要訣", NegotiationStance::Accept, 1003);
        assert!(res4.is_some());
        let note4 = res4.unwrap();
        assert!(note4.contains("【親筆授譜】"));
        assert!(note4.contains("《岑氏沉腕落錘法手札》"));
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
    }
}
