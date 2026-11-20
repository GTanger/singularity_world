//! 外海大島四族百戶當面交談、源頭異語遮罩與本機／雲端 LLM 對話橋接。

use std::collections::BTreeMap;

use crate::ai::openai_chat::{
    DEFAULT_OPENROUTER_BASE_URL, DEFAULT_OPENROUTER_TALK_MODEL,
    call_openai_compatible_chat_with_history, get_player_openrouter_key,
};
use crate::ai::talk::call_openai_compatible_completions;

use super::clock_and_lang::{language_name_zh, mask_speech_by_proficiency};
use super::persona::{DialogueHardStance, StanceVerdict};

const LOCAL_FAST_TALK_URL: &str = "http://127.0.0.1:11436/v1";
const LOCAL_FAST_TALK_MODEL: &str = "/home/tanger/Models/Qwen3-4B-Instruct-2507-Q8_0.gguf";

const FORBIDDEN_META_WORDS: &[&str] = &[
    "去數值化",
    "無職業",
    "零現銀",
    "防分身",
    "獨占態",
    "高維混沌",
    "富態化",
    "聚念場",
    "降維",
    "定形紋",
    "任務看板",
    "任務",
    "等級",
    "玩家",
    "系統",
    "經驗值",
    "技能點",
    "血量",
    "副本",
    "刷新",
    "NPC",
    "HP",
    "MP",
    "遊戲設計",
    "好感度",
    "靈魂種子",
    "短期記憶",
    "長期記憶",
];

const SIMPLIFIED_CHARS: &[char] = &[
    '\u{7b80}', '\u{636e}', '\u{7801}', '\u{7f51}', '\u{7edc}', '\u{8c03}', '\u{8ba4}', '\u{5f02}',
    '\u{7ebf}', '\u{8fdb}', '\u{76d8}', '\u{7f13}', '\u{7ec4}', '\u{5757}', '\u{53c2}', '\u{5bf9}',
    '\u{9488}', '\u{7ed3}', '\u{6784}', '\u{4e3e}', '\u{5b9e}', '\u{73b0}', '\u{7f16}', '\u{8bd1}',
    '\u{8fd0}', '\u{6d4b}', '\u{8bd5}', '\u{9519}', '\u{8bef}', '\u{76d1}', '\u{8282}', '\u{94a5}',
    '\u{542f}', '\u{6001}', '\u{53d1}', '\u{8bbe}', '\u{8bfb}', '\u{5199}', '\u{5e93}', '\u{6237}',
    '\u{7ea7}', '\u{7c7b}', '\u{5c42}', '\u{5173}', '\u{5e94}', '\u{663e}', '\u{9690}', '\u{94b1}',
    '\u{94c1}', '\u{9524}', '\u{95e8}', '\u{9a6c}', '\u{8f66}', '\u{9c7c}', '\u{9e1f}', '\u{5c9b}',
    '\u{8fd9}', '\u{8bf4}', '\u{4eec}', '\u{4e2a}', '\u{4f1a}', '\u{6765}', '\u{65f6}', '\u{8fc7}',
    '\u{8fd8}', '\u{6ca1}', '\u{7ed9}', '\u{8ba9}', '\u{5417}', '\u{542c}', '\u{8bdd}', '\u{89c1}',
    '\u{5f00}', '\u{95ee}', '\u{4e1c}', '\u{5934}', '\u{8fb9}', '\u{4e70}', '\u{5356}', '\u{94f6}',
    '\u{94dc}', '\u{5e01}', '\u{77ff}', '\u{5251}', '\u{836f}', '\u{5e08}', '\u{9986}', '\u{94fa}',
    '\u{9547}', '\u{957f}', '\u{5f20}', '\u{4e24}', '\u{51e0}', '\u{513f}',
];

/// 對談對象之執行期公開摘要（自 `UnifiedMindBody` 直接投影，零 `ResidentSeedDef` 靜態表依賴）
#[derive(Debug, Clone)]
pub struct DialoguePartnerProfile {
    pub id: String,
    pub name: String,
    pub race: String,
    pub role: String,
    pub languages: BTreeMap<String, u8>,
}

#[derive(Debug, Clone)]
pub struct NpcDialogueInput {
    pub player_id: String,
    pub player_name: String,
    pub player_race: String,
    pub player_copper: i64,
    pub player_langs: BTreeMap<String, u8>,
    pub has_stele_rubbing: bool,
    pub has_lodging_voucher: bool,
    pub acquainted: bool,
    pub node_name: String,
    pub calendar_label: String,
    pub bell_label: String,
    pub npc: DialoguePartnerProfile,
    pub npc_effective_copper: i64,
    pub npc_tool_name: String,
    pub apprentice_name: Option<String>,
    pub stance_verdict: StanceVerdict,
    pub recent_talks_with_player: Vec<(String, String)>,
    pub relevant_memories_zh: Vec<String>,
    pub known_local_landmarks_zh: Vec<String>,
    pub used_pressure_tactic: bool,
    pub topic: String,
    pub player_utterance: String,
}

#[derive(Debug, Clone)]
pub struct NpcDialogueOutcome {
    pub player_said: String,
    pub used_pressure_tactic: bool,
    pub npc_reply: String,
    pub side_overheard: Option<String>,
    pub learned_lang: Option<(String, u8)>,
    pub unlocked_rumor: Option<String>,
}

pub fn primary_racial_lang(race: &str) -> Option<&'static str> {
    match race {
        "矮人" => Some("dwarven"),
        "精靈" => Some("elvish"),
        "獸人" => Some("orcish"),
        _ => None,
    }
}

pub(super) fn validate_clean_in_world_reply(raw: &str) -> Option<String> {
    let cleaned = raw.trim().to_string();
    if cleaned.chars().count() < 4 {
        return None;
    }
    if cleaned.chars().any(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    if FORBIDDEN_META_WORDS.iter().any(|bad| cleaned.contains(bad)) {
        return None;
    }
    if SIMPLIFIED_CHARS.iter().any(|&ch| cleaned.contains(ch)) {
        return None;
    }
    Some(cleaned)
}

/// 在不持有世界鎖的前提下生成四族居民交談回應，並執行源頭異語遮罩。
pub fn generate_npc_dialogue(input: &NpcDialogueInput) -> NpcDialogueOutcome {
    let mut unlocked_rumor = None;
    let mut learned_lang = None;
    let mut side_overheard = None;

    // 1. 同族私語旁聽（初上前搭話時觸發一次，避免多輪交談每句重複插入）
    if (input.topic == "greet" || input.recent_talks_with_player.is_empty())
        && let Some(r_lang) = primary_racial_lang(&input.npc.race)
    {
        let prof = input.player_langs.get(r_lang).copied().unwrap_or(0);
        if let Some(ref app_name) = input.apprentice_name {
            let secret_line = match r_lang {
                "dwarven" => format!(
                    "{}師傅，北林斷橋石縫後的試煉坑暗門鬆動了，裡頭雖有發光原石，卻盤著一頭吞石巨蛛！",
                    input.npc.name
                ),
                "elvish" => format!(
                    "{}，林間深處草木越發粗壯，採止血草時別靠近灰耳棚寨的陷阱。",
                    input.npc.name
                ),
                _ => format!(
                    "{}，西峽渡輪這幾日風浪大，扛重貨上船得留神腳下棧板。",
                    input.npc.name
                ),
            };
            let (masked, full_ok) =
                mask_speech_by_proficiency(app_name, r_lang, prof, &secret_line);
            side_overheard = Some(masked);
            if full_ok && r_lang == "dwarven" {
                unlocked_rumor = Some("pit_rumor".to_string());
            }
        }
    }

    // 2. 請教異族語詞或解讀石碑拓片
    let msg = input.player_utterance.trim();
    if input.topic == "learn_lang" || msg.contains("教我") || msg.contains("怎麼說") {
        let target_lang = if input.npc.languages.get("goblin").copied().unwrap_or(0) >= 40
            && (msg.contains("哥布林") || input.npc.role.contains("苦力"))
        {
            Some("goblin")
        } else {
            primary_racial_lang(&input.npc.race)
        };
        if let Some(lcode) = target_lang {
            learned_lang = Some((lcode.to_string(), 15));
            let lzh = language_name_zh(lcode);
            let reply = format!(
                "{}停下手邊活計，指著鐵器與乾糧一字一句示範了幾個{}常用短詞：「先記住『停手』、『糧食』和『石門』的發音，多聽幾回自然就順耳了。」",
                input.npc.name, lzh
            );
            return NpcDialogueOutcome {
                player_said: if msg.is_empty() {
                    "請教幾句日常母語".to_string()
                } else {
                    msg.to_string()
                },
                used_pressure_tactic: input.used_pressure_tactic,
                npc_reply: reply,
                side_overheard,
                learned_lang,
                unlocked_rumor,
            };
        }
    }

    if (input.topic == "show_rubbing" || msg.contains("拓片") || msg.contains("石碑"))
        && input.has_stele_rubbing
        && input.npc.languages.get("dwarven").copied().unwrap_or(0) >= 80
    {
        learned_lang = Some(("dwarven".to_string(), 25));
        unlocked_rumor = Some("pit_rumor".to_string());
        let reply = format!(
            "{}湊近炭火端詳那張石碑拓片，粗手撫過字痕沉聲道：「這是先祖工匠留下的礦坑警語——寫著『斷橋楔石左旋三寸可啟前廳，唯深處石台凝有溫潤原石，切忌驚動鐵絲網中巨蛛』。拿穩你的傢伙再去！」",
            input.npc.name
        );
        return NpcDialogueOutcome {
            player_said: if msg.is_empty() {
                "出示矮人古碑拓片請教字意".to_string()
            } else {
                msg.to_string()
            },
            used_pressure_tactic: input.used_pressure_tactic,
            npc_reply: reply,
            side_overheard,
            learned_lang,
            unlocked_rumor,
        };
    }

    // 3. 若居民海港商語極生疏且玩家不通其母語，直接在源頭遮罩（防提示詞注入代譯）
    let trade_prof = input
        .npc
        .languages
        .get("harbor_trade")
        .copied()
        .unwrap_or(80);
    if trade_prof < 50
        && let Some(r_lang) = primary_racial_lang(&input.npc.race)
    {
        let p_prof = input.player_langs.get(r_lang).copied().unwrap_or(0);
        if p_prof < 80 {
            let raw_native = "我聽不太懂海港話，要換鐵料或草藥得比手勢，北林斷橋那頭很危險。";
            let (masked, _) =
                mask_speech_by_proficiency(&input.npc.name, r_lang, p_prof, raw_native);
            return NpcDialogueOutcome {
                player_said: msg.to_string(),
                used_pressure_tactic: input.used_pressure_tactic,
                npc_reply: masked,
                side_overheard,
                learned_lang: Some((r_lang.to_string(), 6)),
                unlocked_rumor,
            };
        }
    }

    // 4. 打聽北林／斷橋風聲時，由酒館掌櫃與山林獵戶提供口風；矮人試煉坑暗記亦可透過山嶽語旁聽或拓片解讀獲知
    let clipped_msg: String = msg.chars().take(80).collect();
    let msg = clipped_msg.as_str();
    if (input.topic == "rumor"
        || msg.contains("斷橋")
        || msg.contains("北林")
        || msg.contains("試煉"))
        && (input.npc.role.contains("酒館") || input.npc.role.contains("獵戶"))
    {
        unlocked_rumor = Some("pit_rumor".to_string());
    }

    let fallback = deterministic_in_world_reply(input);
    if input.topic == "greet" && msg.is_empty() {
        return NpcDialogueOutcome {
            player_said: "上前點頭招呼".to_string(),
            used_pressure_tactic: input.used_pressure_tactic,
            npc_reply: fallback,
            side_overheard,
            learned_lang,
            unlocked_rumor,
        };
    }

    let safe_player_name: String = input
        .player_name
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '"' | '\'' | '`' | '{' | '}' | '[' | ']'))
        .take(16)
        .collect();

    let mut mem_lines = input.relevant_memories_zh.clone();
    if let Some((last_p, last_n)) = input.recent_talks_with_player.last() {
        mem_lines.insert(
            0,
            format!("剛才對方才對你說過「{}」，你當時回他「{}」", last_p, last_n),
        );
    }
    let memory_block = if mem_lines.is_empty() {
        "（今日尚無特別掛懷的鎮上異聞）".to_string()
    } else {
        mem_lines.join("；")
    };
    let map_scope_block = if input.known_local_landmarks_zh.is_empty() {
        input.node_name.clone()
    } else {
        input.known_local_landmarks_zh.join("、")
    };
    let relation_hint = if input.acquainted {
        format!(
            "熟識的街坊來客「{}」（種族：{}）",
            safe_player_name, input.player_race
        )
    } else {
        format!("初見面的{}旅人", input.player_race)
    };

    // 零性格形容詞提示（禁止餵「暴躁/傲嬌」標籤，改注入純 Rust 預判立場、語用約束、個人記憶與已知地圖迷霧邊界）
    let system_prompt = format!(
        "你是銀鷗島上的真實居民「{}」（種族：{}，日常營生：{}，隨身器物：{}）。\n\
         所在位置：{}（{}・{}），當下神態：{}。\n\
         你手頭現有 {} 枚銅幣；面前搭話者是{}。\n\
         【當下處事定見與說話方式】：{}\n\
         【你親身記得的近期見聞與往來紀錄】：{}\n\
         【你曾親自踏足知曉的地界】：{}（未列出的陌生荒野或地底深處你並未去過，絕不可裝熟指路）。\n\
         嚴格守則：\n\
         1. 只用台灣正體中文與口語化世界內對白回答（不超過 90 字），禁止加引號外旁白或舞台腔。\n\
         2. 報價與時刻一律說具體銅幣數與鐘響，絕不可違背上方【當下處事定見】私自降價、白送財物或無憑賒帳。\n\
         3. 絕對禁止出現任何現代詞彙、設定詞彙，也不可替不懂異族語言的人翻譯未學會的語言。",
        input.npc.name,
        input.npc.race,
        input.npc.role,
        input.npc_tool_name,
        input.node_name,
        input.calendar_label,
        input.bell_label,
        input.stance_verdict.posture_zh,
        input.npc_effective_copper,
        relation_hint,
        input.stance_verdict.speech_rule_zh,
        memory_block,
        map_scope_block,
    );

    let user_prompt = if !msg.is_empty() {
        msg.to_string()
    } else {
        match input.topic.as_str() {
            "work" => {
                "我想打聽店裡有沒有能出力賺幾枚銅幣的活計，順便問問修補器具怎麼算。".to_string()
            }
            "rumor" => "最近島上北林斷橋、荒野山徑或鎮上街坊有什麼該留神的消息？".to_string(),
            "carriage" => "請問驛馬車發車鐘響、票價與路線怎麼走？".to_string(),
            _ => "你好，路過貴寶地打聲招呼。".to_string(),
        }
    };

    let final_reply = call_talk_backends(
        &input.player_id,
        &system_prompt,
        &input.recent_talks_with_player,
        &user_prompt,
    )
    .and_then(|raw| validate_clean_in_world_reply(&raw))
    .and_then(|clean| verify_stance_consistency(&clean, input.stance_verdict.stance))
    .unwrap_or(fallback);

    NpcDialogueOutcome {
        player_said: user_prompt,
        used_pressure_tactic: input.used_pressure_tactic,
        npc_reply: final_reply,
        side_overheard,
        learned_lang,
        unlocked_rumor,
    }
}

/// 一階防順從立場校驗器：若 Rust 已鎖死強硬回絕或鐵價不讓，攔截 LLM 討好鬆口語句
fn verify_stance_consistency(reply: &str, stance: DialogueHardStance) -> Option<String> {
    if matches!(
        stance,
        DialogueHardStance::RefuseColdly | DialogueHardStance::HaggleFirm
    ) {
        const CAVE_WORDS: &[&str] = &[
            "好吧",
            "算你便宜",
            "免費送",
            "白送你",
            "答應你",
            "賒給你",
            "不用錢",
            "拿去吧",
        ];
        if CAVE_WORDS.iter().any(|w| reply.contains(w)) {
            return None;
        }
    }
    Some(reply.to_string())
}

fn call_talk_backends(
    player_id: &str,
    system_prompt: &str,
    history: &[(String, String)],
    user_prompt: &str,
) -> Option<String> {
    if let Some(api_key) = get_player_openrouter_key(player_id)
        && let Ok(ans) = call_openai_compatible_chat_with_history(
            DEFAULT_OPENROUTER_BASE_URL,
            &api_key,
            DEFAULT_OPENROUTER_TALK_MODEL,
            system_prompt,
            history,
            user_prompt,
            160,
        )
    {
        return Some(ans);
    }

    if let Ok(base) = std::env::var("PLAYER_TALK_API_BASE_URL")
        && !base.trim().is_empty()
    {
        let key = std::env::var("PLAYER_TALK_API_KEY").unwrap_or_default();
        let model = std::env::var("PLAYER_TALK_API_MODEL")
            .unwrap_or_else(|_| "google/gemini-2.5-flash".to_string());
        if let Ok(ans) = call_openai_compatible_completions(
            &base,
            &key,
            &model,
            system_prompt,
            user_prompt,
            6,
            180,
        ) {
            return Some(ans);
        }
    }

    call_openai_compatible_chat_with_history(
        LOCAL_FAST_TALK_URL,
        "",
        LOCAL_FAST_TALK_MODEL,
        system_prompt,
        history,
        user_prompt,
        140,
    )
    .ok()
}

fn deterministic_in_world_reply(input: &NpcDialogueInput) -> String {
    match input.stance_verdict.stance {
        DialogueHardStance::RefuseColdly => {
            return format!(
                "「咱們話不投機，鋪子裡還有活要趕，請自便吧。」{}（{}）頭也不抬地冷淡應道。",
                input.npc.name, input.stance_verdict.posture_zh
            );
        }
        DialogueHardStance::HaggleFirm if input.used_pressure_tactic => {
            return format!(
                "「港裡做營生講的是明碼實價，手藝工本一文也少不得，沒帶夠銅幣就先去扛活攢足再來。」{}按住手邊的{}沉聲道。",
                input.npc.name, input.npc_tool_name
            );
        }
        _ => {}
    }

    let role = input.npc.role.as_str();
    if let Some(mem) = input.relevant_memories_zh.first()
        && input.topic == "rumor"
    {
        return format!(
            "「這兩天鎮上大夥兒都在傳——{}。出門在外多留個心眼總沒錯。」{}（{}）低聲說道。",
            mem, input.npc.name, input.stance_verdict.posture_zh
        );
    }
    if role.contains("鍛鐵匠") {
        if input.topic == "rumor" || input.player_utterance.contains("斷橋") {
            return format!(
                "「北城門出去那座灰松斷橋底下，前陣子有獵戶看見鬆動的矮人楔石，搬開就能鑽進舊試煉坑；你要敢下去，先把傢伙修利索，別餵了裡頭的巨蛛。」{}敲了敲手中的{}。",
                input.npc.name, input.npc_tool_name
            );
        }
        return format!(
            "「爐火正旺！想出力幫拉風箱打下手，一趟算你 5 枚銅幣；修補磨損兵刃收 4 枚銅幣。若從北林坑底尋來發光原石，再添 6 枚銅幣我能替你把鐵紋鑲進錘頭裡。」{}（{}）說道。",
            input.npc.name, input.stance_verdict.posture_zh
        );
    }
    if role.contains("酒館掌櫃") {
        if input.has_lodging_voucher {
            return format!(
                "「喲，手裡攥著船東開的抵宿券吧？遞過來就能在二樓乾淨閣樓住兩宿，樓上床頭厚木箱能鎖重物。餓了買份黑麥乾糧 2 枚銅幣，出遠門前還能花 5 枚銅幣簽份逾時搜救契。」{}笑著招呼。",
                input.npc.name
            );
        }
        return format!(
            "「外頭海風涼，進來歇會兒！二樓宿房一晚 3 枚銅幣、黑麥乾糧 2 枚銅幣；要是打算往島心老林子鑽，花 5 枚銅幣留份逾時搜救契，真倒在路上也有人把你抬回來。」{}擦著橡木杯說。",
            input.npc.name
        );
    }
    if role.contains("車行掌櫃") || role.contains("馭手") {
        return format!(
            "「我們雙馬篷車專走白鷗港經灰松林驛到西峽渡口的大路，單程票價 3 枚銅幣，省腳力又不必怕林子裡的灰耳斥候。攢夠 25 枚銅幣到了西峽渡口，就能搭公營雙桅渡輪過海！」{}指著時刻木牌說。",
            input.npc.name
        );
    }
    if role.contains("草藥師") {
        return format!(
            "「山裡野獸甲殼硬，硬拚容易震傷手腕。要是不慎掛彩，花 3 枚銅幣讓我敷上苦艾止血膏就能消腫；你在海灘或林間採到草藥束也能拿來換銅幣。」{}握著{}提醒道。",
            input.npc.name, input.npc_tool_name
        );
    }
    if role.contains("苦力") {
        return format!(
            "「碼頭上天天有重活，肯流汗就餓不死！對了，北面灰松林裡的灰耳哥布林嘴饞得很，我懂幾句牠們的叫嚷，你要學個一兩句，遇上斥候拋塊乾糧就能免場惡鬥。」{}拍了拍寬厚肩膀。",
            input.npc.name
        );
    }
    let greet_head = if input.acquainted {
        format!("「又見面了，{}！", input.player_name)
    } else {
        "「".to_string()
    };
    format!(
        "{}這會兒正值{}，{}裡大夥兒各忙各的營生；我手頭正忙著{}的活計。」{}（{}）點頭致意。",
        greet_head,
        input.bell_label,
        input.node_name,
        input.npc.role,
        input.npc.name,
        input.stance_verdict.posture_zh
    )
}
