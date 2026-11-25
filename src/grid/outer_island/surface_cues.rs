//! 雙表面呈現隔離器：玩家街頭「非讀心術」感官線索 × 對話「分級私密度吐露門檻」 × `/chronicle` 創作者完整心緒透視。
//!
//! 嚴守正典與審辯室 Room `eb5e806f` 定案：
//! 1. 玩家端（`views.rs` / `dialogue_llm.rs`）：絕不洩漏陌生人內心獨白或心事槽明文；街上僅見步態、神色、視線迴避與手頭舉動。
//! 2. 對話分級門檻：唯有 `熟稔度/2 + 正向冷暖 + 名分加成 >= 心事私密度` 且話題相關時，角色才向對方吐露實情；否則藏在心底並表現為語氣短促或戒備。
//! 3. 創作者端（`/chronicle` 與五鎮列傳）：完整投射同一個 4 格心事槽、六維心緒、先天脾氣底色與中程方針。

use serde_json::{Value, json};

use super::appraisal::{ConcernCause, FocusConcern, PrimaryEmotion};
use super::household_economy::count_household_larder_rations;
use super::mind::tie_flags;
use super::persona::{DialogueHardStance, InnatePersona, StanceVerdict, derive_innate_persona};
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

fn resolve_actor_label<'a>(
    delta: &'a IslandWorldDelta,
    tid_opt: Option<&str>,
    mask_players: bool,
) -> &'a str {
    let Some(tid) = tid_opt else {
        return "街坊舊識";
    };
    if mask_players && tid.starts_with("pl:") {
        return "外來旅人";
    }
    delta.actors.get(tid).map_or("街坊舊識", |a| {
        if mask_players && a.controller_id.is_some() {
            "外來旅人"
        } else {
            a.display_name.as_str()
        }
    })
}

/// 將結構化 `FocusConcern` 即時查表合成為世界內第一人稱心事語句（`mask_players = true` 時對 `/chronicle` 隱去真實玩家名號）
pub fn render_concern_inner_thought_zh(
    delta: &IslandWorldDelta,
    c: &FocusConcern,
    mask_players: bool,
) -> String {
    let who = resolve_actor_label(delta, c.target_id.as_deref(), mask_players);
    match c.cause {
        ConcernCause::LivelihoodShortfall => {
            "家裡存糧見底、手頭銅幣緊俏，正發愁這兩頓伙食怎麼張羅".to_string()
        }
        ConcernCause::DebtDefaulted => {
            format!("欠{}的帳款已到期限卻還沒湊齊銅板，心裡懸著這筆債", who)
        }
        ConcernCause::DebtPressured => {
            if c.emotion == PrimaryEmotion::Anger {
                format!("氣惱{}帳目往來言而無信，心頭窩著火", who)
            } else {
                format!("被{}當面追問帳款，臉面與手頭都備感壓力", who)
            }
        }
        ConcernCause::PactBetrayed => {
            format!("遭{}毀諾失信傷了根本，至今難以釋懷", who)
        }
        ConcernCause::RivalEncounter => {
            format!("剛才在街頭撞見素有嫌隙的{}，心緒翻湧不快", who)
        }
        ConcernCause::KinshipRift => {
            format!("與{}之間的舊日裂痕未消，想起便覺心頭沉鬱", who)
        }
        ConcernCause::KinshipWarmth => {
            format!("多虧{}近日溫言照拂相助，心頭踏實不少", who)
        }
        ConcernCause::PactHonored => {
            if c.target_id.is_some() {
                format!("與{}銀貨兩訖守信履約，手頭生計漸入正軌", who)
            } else {
                "剛把手頭山海物料換成實打實的銅幣，心裡寬裕許多".to_string()
            }
        }
        ConcernCause::ToolDamaged => "隨身吃飯傢伙有了崩損磨痕，得抽空好好打磨修整".to_string(),
        ConcernCause::CrossTownYearning => {
            format!("許久未聞外鎮{}的平安音訊，心裡不免記掛", who)
        }
        ConcernCause::CraftBreakthrough => "手頭器物打磨得愈發趁手，對往後生計頗有把握".to_string(),
    }
}

/// 正規化單條見聞語句：消除「人名：「…」人名動作」頭尾重複主詞與缺漏左引號 `「`
pub fn normalize_breath_line(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    if let Some((speaker_raw, rest_raw)) = s.split_once('：') {
        let speaker = speaker_raw.trim();
        let rest = rest_raw.trim();
        if !speaker.is_empty()
            && speaker.chars().count() <= 16
            && let Some((spoken, tail)) = rest.split_once('」')
        {
            let clean_spoken = spoken.trim().trim_start_matches('「').trim();
            let clean_tail = tail.trim();
            if !clean_tail.is_empty() && clean_tail.contains(speaker) {
                return format!("「{}」{}", clean_spoken, clean_tail);
            }
            if clean_tail.is_empty() {
                return format!("{}：「{}」", speaker, clean_spoken);
            }
            return format!("{}：「{}」{}", speaker, clean_spoken, clean_tail);
        }
    }
    if s.contains('」') && !s.contains('「') {
        return format!("「{}", s);
    }
    s.to_string()
}

/// 過濾並去重場景即時見聞：消除相鄰重複洗版、修復舊引號格式，並濾除跨鐘響過時寒暄（防正午顯示「正值黃昏鐘響」）
pub fn filter_clean_scene_breaths(
    breaths: &std::collections::VecDeque<String>,
    current_bell_label: &str,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(breaths.len());
    for raw in breaths {
        let norm = normalize_breath_line(raw);
        if norm.is_empty() || norm.starts_with("柯德斯：") || norm.contains("」柯德斯（") {
            continue;
        }
        if norm.contains("正值") && norm.contains("鐘響") && !norm.contains(current_bell_label)
        {
            continue;
        }
        if out.last().is_some_and(|prev| prev == &norm) {
            continue;
        }
        out.push(norm);
    }
    out
}

/// 產生玩家在街頭肉眼可見之「非讀心術」外顯舉止與神態線索（零錢包透視、依真實時辰場所與個人種子自然分化）
pub fn build_observable_npc_cue(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    clock: &super::clock_and_lang::SolarClockSnapshot,
    other: &UnifiedMindBody,
    innate: &InnatePersona,
) -> String {
    let aff = &other.mind.stm.affect;
    let eff_fatigue = aff.fatigue.max(other.fatigue);
    let last_intent = other.mind.stm.intentions.back().map(|i| i.kind.as_str());
    let node_opt = doc.nodes.get(&other.node_id);
    let node_kind = node_opt.map_or("street", |n| n.kind.as_str());
    let is_indoor = node_opt.is_some_and(|n| n.indoor);
    let variant = (other.soul_seed % 3) as usize;

    // 1. 行進中：依當下意圖與情緒決定腳步神色（絕不直接印出內心隱私原因）
    if let Some(next_nid) = other.route_plan.front()
        && let Some(nd) = doc.nodes.get(next_nid)
    {
        let short_dst = nd.name.split('・').next_back().unwrap_or(nd.name.as_str());
        return match last_intent {
            Some("detour_avoid") => format!("神色冷硬、刻意低頭繞往{}", short_dst),
            Some("forage_wild") => format!("肩挎粗麻袋正快步趕往{}", short_dst),
            Some("sell_foraged") => format!("背囊沉甸甸地正往{}趕路", short_dst),
            Some("visit_confidant") => format!("眉間帶著心事正往{}尋人", short_dst),
            Some("confront_chase") => format!("沉著臉大步流星直奔{}", short_dst),
            Some("plead_extension") | Some("restock_larder") => {
                format!("腳步匆匆正往{}趕去", short_dst)
            }
            Some("go_home_sleep") | Some("shelter_hearth") => {
                format!("收妥手頭器物正沿夜街往{}歇息", short_dst)
            }
            _ => {
                if aff.fear >= 38 {
                    format!("神色匆促緊按衣袋正趕往{}", short_dst)
                } else if aff.anger >= 40 {
                    format!("面色鐵青、腳步重重走向{}", short_dst)
                } else {
                    innate.body_posture_cue(aff.hunger, eff_fatigue, aff.anger, Some(short_dst))
                }
            }
        };
    }

    // 1b. 入夜駐足作息真實化：杜絕三更半夜仍在碼頭扛貨或閉門打烊後仍顯示營業張羅
    if clock.is_night {
        let sleep_home = other
            .owned_house_node
            .as_deref()
            .unwrap_or(other.home_node.as_str());
        let at_sleep_spot =
            other.node_id == sleep_home || (node_kind == "inn_room" && other.inn_nights_left > 0);
        if at_sleep_spot {
            if node_kind == "shop" {
                if other.role.contains("酒館") {
                    return if clock.hour < 6 {
                        "守著壁爐餘火在櫃後打盹歇息".to_string()
                    } else {
                        "在暖黃油燈下擦拭木杯照看夜客".to_string()
                    };
                }
                return "鋪門已落栓、退入後堂熄燈歇息".to_string();
            }
            if is_indoor {
                return "已在屋內吹燈安歇".to_string();
            }
            return "裹緊舊斗篷靠在避風處假寐歇腳".to_string();
        }
        if node_kind == "shop" && other.role.contains("酒館") {
            return if clock.hour < 6 {
                "守著壁爐餘火在櫃後打盹歇息".to_string()
            } else {
                "在暖黃油燈下擦拭木杯照看夜客".to_string()
            };
        }
        if node_kind == "shop" {
            return "靠在壁爐餘溫旁歇腳避寒".to_string();
        }
        if other.role.contains("衛兵") {
            return "提著防風夜燈沿街口石階巡守".to_string();
        }
        if !is_indoor {
            return "裹緊衣襟在夜風避風處歇腳緩氣".to_string();
        }
    }

    // 2. 同場撞見嫌隙對象：肉眼可見視線迴避或緊繃（玩家可觀察誰跟誰不對盤，但需親自打聽才知前因！）
    if let Some(occ) = delta.node_occupants.get(&other.node_id) {
        for oid in occ {
            if oid != &other.actor_id
                && other
                    .mind
                    .ltm
                    .social
                    .get(oid)
                    .is_some_and(|s| s.warmth <= -22)
            {
                return if innate.temperament.approach_avoid >= 0 {
                    "抱臂冷眼瞥向場中一角、下顎微繃".to_string()
                } else {
                    "刻意側身背對場中某人、低頭不語".to_string()
                };
            }
        }
    }

    // 3. 駐足狀態：依四維情緒 × 先天脾氣 × 所在場所投射真實肢體動作（不透視銅幣家底、不複製貼上）
    if aff.anger >= 38 {
        if innate.temperament.ignition < 0 {
            "指節攥得微白、眉頭緊鎖".to_string()
        } else {
            "面色沉冷、手裡活計落得極重".to_string()
        }
    } else if aff.grief >= 35 {
        "眼眶微黯、不時對著手邊器物出神輕嘆".to_string()
    } else if aff.fear >= 35 {
        "神色戒備、目光不時望向街口動靜".to_string()
    } else if aff.hope >= 38 {
        match node_kind {
            "dock" => "眉宇舒展、在棧橋邊俐落理著纜繩".to_string(),
            "shop" => "眉宇舒展、手腳麻利地在鋪裡張羅".to_string(),
            _ => "神采奕奕、步履輕穩地打量四周".to_string(),
        }
    } else if eff_fatigue >= 60 || aff.hunger >= 60 {
        if innate.temperament.pain_tolerance > 15 {
            "額角見汗仍沉肩撐著氣力".to_string()
        } else {
            match variant {
                0 => "氣息微促、抬手拭去額角汗水".to_string(),
                1 => "步履微沉、靠著旁側緩一口氣".to_string(),
                _ => "神色略顯倦怠、揉了揉酸沉肩頸".to_string(),
            }
        }
    } else {
        match node_kind {
            "dock" => {
                if !super::live_tide::is_dock_cargo_tide_open(clock) && other.role.contains("苦力")
                {
                    "坐在纜樁旁歇氣、等候下一班潮汐貨船".to_string()
                } else {
                    match variant {
                        0 => "在棧橋邊穩手理著粗麻纜繩".to_string(),
                        1 => "迎著海風望向泊船棧橋動靜".to_string(),
                        _ => "挽起袖口在碼頭石板間搬理貨箱".to_string(),
                    }
                }
            }
            "shop" => match variant {
                0 => "神態從容在鋪裡張羅手頭活計".to_string(),
                1 => "守在案台旁不疾不徐整理器物".to_string(),
                _ => "垂目核對著鋪內貨架與器物".to_string(),
            },
            "gate" => match variant {
                0 => "立在城門洞旁留神往來車馬".to_string(),
                _ => "手按腰間兵刃打量進出旅人".to_string(),
            },
            "plaza" => match variant {
                0 => "在市集石板間駐足打量四下動靜".to_string(),
                1 => "神色從容在廣場邊歇腳觀望".to_string(),
                _ => "眼神活絡留心著市集往來行色".to_string(),
            },
            "wild" | "dungeon" => "手按隨身兵刃、留神四周風吹草動".to_string(),
            _ => innate.body_posture_cue(aff.hunger, eff_fatigue, aff.anger, None),
        }
    }
}

fn tie_disclosure_bonus(tie_mask: u8, same_household: bool) -> i32 {
    if same_household || (tie_mask & tie_flags::SWORN_COMRADE) != 0 {
        28
    } else if (tie_mask & (tie_flags::BLOOD_KIN | tie_flags::MASTER_DISCIPLE)) != 0 {
        18
    } else if (tie_mask & (tie_flags::IN_LAW | tie_flags::PEER_COHORT)) != 0 {
        10
    } else {
        0
    }
}

/// 對話前執行「分級私密度吐露閘門」並補齊真實生理神態（修補 `persona.rs:638` 傳 `0,0` 漏洞）
#[expect(
    clippy::too_many_arguments,
    reason = "graduated disclosure gate requires speaker-target relational context"
)]
pub fn enrich_dialogue_with_graduated_disclosure(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    speaker: &UnifiedMindBody,
    target: &UnifiedMindBody,
    innate: &InnatePersona,
    topic: &str,
    utterance: &str,
    stance_verdict: &mut StanceVerdict,
    relevant_memories_zh: &mut Vec<String>,
) {
    // 1. 以真實時辰與飢疲怒憂覆寫對話當下神態（不再傳 0, 0）
    let now_clk = super::clock_and_lang::current_solar_clock();
    stance_verdict.posture_zh = build_observable_npc_cue(doc, delta, &now_clk, target, innate);

    // 2. 計算說話者對目標之信任吐露分數
    let soc_opt = target.mind.ltm.social.get(&speaker.actor_id);
    let fam = soc_opt.map_or(0, |s| s.familiarity);
    let warmth = soc_opt.map_or(0, |s| s.warmth);
    let mask = soc_opt.map_or(0, |s| s.tie_mask);
    let same_hh = !speaker.household_id.is_empty() && speaker.household_id == target.household_id;

    let trust_score = if warmth <= -15 {
        0
    } else {
        i32::from(fam) / 2 + i32::from(warmth).max(0) + tie_disclosure_bonus(mask, same_hh)
    };

    let asks_wellbeing = topic == "rumor"
        || utterance.contains("最近")
        || utterance.contains("怎麼了")
        || utterance.contains("還好")
        || utterance.contains("心事")
        || utterance.contains("臉色")
        || utterance.contains("幫忙")
        || utterance.contains("發愁");

    let mut disclosed_lines: Vec<String> = Vec::new();
    let mut withheld_any = false;
    let mut withheld_heavy = false;
    let mut withheld_names: Vec<String> = Vec::new();

    // R4-M7 修正：完整掃描全部 4 格心事槽，避免第 3~4 格高私密心事漏進未吐露保護名單！
    for c in target.mind.active_concerns_sorted() {
        let gate = if asks_wellbeing {
            i32::from(c.privacy).saturating_sub(10)
        } else {
            i32::from(c.privacy)
        };
        if trust_score >= gate {
            if disclosed_lines.len() < 2 {
                // R3-M5 修正：以純世界內第一人稱口語插入，徹底拔除「【你此刻願意…】」提示詞標籤！
                disclosed_lines.push(format!(
                    "這陣子{}",
                    render_concern_inner_thought_zh(delta, c, false)
                ));
            }
        } else {
            withheld_any = true;
            if let Some(ref tid) = c.target_id
                && let Some(ta) = delta.actors.get(tid)
                && !ta.display_name.is_empty()
            {
                withheld_names.push(ta.display_name.clone());
            }
            if c.intensity >= 38 && c.valence < 0 {
                withheld_heavy = true;
            }
        }
    }

    // R2-M3 + R3-L5 修正：凡存在未達吐露門檻之心事或信任度未達熟稔線，先剔除已撈出之私密情節，再插入允許吐露之自然語句！
    if withheld_any || trust_score < 56 {
        const PRIV_LEAK_TERMS: &[&str] = &[
            "欠",
            "債",
            "賒",
            "寬限",
            "人情帳",
            "帳款",
            "逾期",
            "催討",
            "拖欠",
            "拒賒",
            "黑冊",
            "決裂",
            "反目",
            "嫌隙",
            "毀諾",
            "背約",
            "背棄然諾",
        ];
        relevant_memories_zh.retain(|m| {
            let has_priv_term = PRIV_LEAK_TERMS.iter().any(|k| m.contains(k));
            let hits_withheld_person = withheld_names.iter().any(|nm| m.contains(nm.as_str()));
            !has_priv_term && !hits_withheld_person
        });
    }

    let disclosed_any = !disclosed_lines.is_empty();
    for line in disclosed_lines.into_iter().rev() {
        relevant_memories_zh.insert(0, line);
    }

    if !disclosed_any && withheld_heavy {
        if trust_score < 24 {
            stance_verdict.stance = DialogueHardStance::GuardedEvade;
        }
        stance_verdict.speech_rule_zh.push_str(
            "；你心裡正壓著不便對生面孔明言的煩心事，回話略顯心不在焉或語氣短促，絕不向眼前人洩露私密家務與欠債底細。",
        );
    } else if !withheld_any
        && trust_score >= 62
        && let Some(ref strat) = target.mind.stm.goal_strategy
    {
        // R4-H2 修正：僅在無隱瞞心事且高度親信（>=62）時以第一人稱自然口語提及盤算，杜絕第二人稱提示詞與欠債底細外露！
        relevant_memories_zh.push(format!("我這幾日正打算先設法{}", strat.kind.label_zh()));
    }
}

fn format_temperament_signature_zh(innate: &InnatePersona, target: &UnifiedMindBody) -> String {
    let ign = if innate.temperament.ignition < -18 {
        "快言直性一點就著"
    } else if innate.temperament.ignition > 18 {
        "沉穩慢熱不易動氣"
    } else {
        "動靜有度"
    };
    let damp = if innate.temperament.damping > 18 {
        "心事記掛極深"
    } else if innate.temperament.damping < -18 {
        "事過則散不記隔夜仇"
    } else {
        "恩怨分明"
    };
    let top_v = innate.top_value_axis(&target.drift).in_world_motive_zh();
    let tab_v = innate.taboo_axis.in_world_motive_zh();
    format!("{}・{}｜最重「{}」・逆鱗「{}」", ign, damp, top_v, tab_v)
}

fn format_affect_summary_zh(a: &UnifiedMindBody, larder: u32) -> String {
    let aff = &a.mind.stm.affect;
    let mut parts = Vec::new();
    if aff.anger >= 20 {
        parts.push(format!("憤懣({})", aff.anger));
    }
    if aff.fear >= 20 {
        parts.push(format!("憂懼({})", aff.fear));
    }
    if aff.grief >= 20 {
        parts.push(format!("悲戚({})", aff.grief));
    }
    if aff.hope >= 20 {
        parts.push(format!("期盼({})", aff.hope));
    }
    let emo_str = if parts.is_empty() {
        "心緒平穩".to_string()
    } else {
        parts.join("・")
    };
    let belly = if aff.hunger >= 65 {
        "腹中飢餓"
    } else if aff.hunger <= 28 {
        "飲食溫飽"
    } else {
        "飢飽如常"
    };
    format!(
        "{}｜{}（屋內存糧 {} 份・手頭 {} 銅）",
        emo_str, belly, larder, a.copper
    )
}

/// 專供創作者閱讀器 `/api/chronicle/actor` 輸出之完整內在心緒與思維盤算透視快照
pub fn build_chronicle_mind_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
    current_day: i64,
) -> Value {
    let innate = derive_innate_persona(a.soul_seed.max(1));
    let larder = count_household_larder_rations(delta, a);
    let safe_place = |raw: &str| -> String {
        if let Some(n) = doc.nodes.get(raw) {
            n.name.clone()
        } else if let Some(bh) = delta.built_houses.get(raw) {
            super::slow_think::mask_player_names_in_text(delta, &bh.label_zh)
        } else if raw.contains("pl:") {
            "旅人自建新宅".to_string()
        } else {
            super::slow_think::mask_player_names_in_text(delta, raw)
        }
    };
    let cur_place = safe_place(&a.node_id);
    let cur_intent = a.mind.stm.intentions.back().map_or_else(
        || format!("駐足於【{}】操持日常", cur_place),
        |it| {
            let dst = safe_place(&it.target_node);
            format!("{}（目標地點：{}）", it.reason_zh, dst)
        },
    );

    let strategy_zh = a.mind.stm.goal_strategy.as_ref().map_or_else(
        || "順應日常營生（無高張力轉向方針）".to_string(),
        |g| {
            let rem = (g.expires_day - current_day).max(1);
            format!("【{}】（尚餘 {} 日觀察期）", g.kind.label_zh(), rem)
        },
    );

    let concerns: Vec<Value> = a
        .mind
        .active_concerns_sorted()
        .into_iter()
        .map(|c| {
            json!({
                "emotion_zh": c.emotion.label_zh(),
                "intensity": c.intensity,
                "axis_zh": c.axis.in_world_motive_zh(),
                "privacy": c.privacy,
                "thought_zh": render_concern_inner_thought_zh(delta, c, true),
            })
        })
        .collect();

    let mastery_zh = super::phase_and_vessels::format_mastery_status_zh(a);
    let pacts_vec: Vec<String> = delta
        .contracts
        .values()
        .filter(|c| {
            super::pacts_and_peers::is_escrow_pact_kind(&c.kind)
                && (c.status == "escrowed" || c.status == "in_progress")
                && (c.creditor_id == a.actor_id || c.debtor_id == a.actor_id)
        })
        .take(2)
        .map(|c| {
            let (_, desc) = super::pacts_and_peers::parse_pact_note(&c.note_zh);
            super::slow_think::mask_player_names_in_text(delta, desc)
        })
        .collect();

    json!({
        "temperament_zh": format_temperament_signature_zh(&innate, a),
        "affect_summary_zh": format_affect_summary_zh(a, larder),
        "current_place_zh": cur_place,
        "current_intention_zh": super::slow_think::mask_player_names_in_text(delta, &cur_intent),
        "active_strategy_zh": super::slow_think::mask_player_names_in_text(delta, &strategy_zh),
        "mastery_zh": super::slow_think::mask_player_names_in_text(delta, &mastery_zh),
        "living_pacts_zh": pacts_vec.join("；"),
        "concerns": concerns,
    })
}

/// 供五鎮 Markdown 離線列傳集同步輸出一行精煉「性情與當下盤算」摘要
pub fn format_markdown_mind_bullet_zh(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
    current_day: i64,
) -> String {
    let innate = derive_innate_persona(a.soul_seed.max(1));
    let larder = count_household_larder_rations(delta, a);
    let temp = format_temperament_signature_zh(&innate, a);
    let aff = format_affect_summary_zh(a, larder);
    let mastery = super::phase_and_vessels::format_mastery_status_zh(a);
    let cur_intent = a
        .mind
        .stm
        .intentions
        .back()
        .map_or("守鋪操持日常", |it| it.reason_zh.as_str());
    let top_c = a
        .mind
        .top_concern()
        .map(|c| {
            format!(
                "；心頭掛念：{}",
                render_concern_inner_thought_zh(delta, c, true)
            )
        })
        .unwrap_or_default();
    let _ = (doc, current_day);
    format!(
        "- **性情底色與當下心緒**：{}｜{}｜歷練：{}｜眼下盤算：{}{}\n",
        temp, aff, mastery, cur_intent, top_c
    )
}
