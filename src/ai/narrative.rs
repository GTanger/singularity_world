// narrative 模組 — LLM 資料轉描述與嚴格事實保真度（Fact-Fidelity）驗證管線。
// 嚴守台灣正體中文，徹底拔除第四面牆詞彙與即時性虛詞。後台資料嚴謹，前台體感去數值化。

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::combat::wordplate::{
    derive_elementary_motions_from_chars, CarrierAffinity, TacticalSlot, TargetPart,
};

/// 招式資料封包（後台結構化字牌與動作資料）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDataPacket {
    pub entity_id: String,
    pub role_title: String,
    pub domain: String,         // "martial" | "production"
    pub slot_label: String,     // e.g. "起手", "連攜", "破綻", "守勢", "採獵", "百工", "庖廚", "行旅"
    pub hub_or_tier: String,    // e.g. "太陰肺樞" or "熟稔"
    pub word_chars: Vec<String>,
    pub vessel_name: String,    // e.g. "分骨彎刀 [銳]" or "徒手"
    pub carrier: CarrierAffinity,
    pub motion_sequence: Vec<String>,
    pub fallback_name: String,
}

/// 綜合演繹招式文采成果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynthesizedSkillProse {
    pub name: String,
    pub origin_basis: String,
}

/// 單節交鋒事實封包（後台物理判定與招式碰擊斷點資料）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CombatBoutFactPacket {
    pub slot: TacticalSlot,
    pub attacker_name: String,
    pub defender_name: String,
    pub attacker_weapon: String,
    pub defender_weapon: String,
    pub attacker_carrier: CarrierAffinity,
    pub defender_carrier: CarrierAffinity,
    pub winner_side: String, // "attacker", "defender", "draw"
    pub winner_skill_name: String,
    pub loser_skill_name: String, // 絕不傳入 LLM prompt；僅用於驗證器確保未遭洩漏
    pub winner_motions: Vec<String>,
    pub loser_attempted_motions: Vec<String>,
    pub counter_reason: Option<String>,
    pub hit_part: TargetPart,
    pub severity_label: String, // "輕微擦震" | "氣血翻湧" | "重創失據"
    #[serde(default)]
    pub active_residue: Option<String>,
    #[serde(default)]
    pub residue_interaction: Option<String>,
    #[serde(default)]
    pub loser_posture_after: Option<String>,
    #[serde(default)]
    pub inflicted_injury: Option<String>,
}

/// 百工產出文采事實封包（後台結構化材料、器具與內息資料）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftActionFactPacket {
    pub actor_name: String,
    pub tool_name: String,
    pub tool_familiarity: String,
    pub breath_habit: String,
    pub material_names: Vec<String>,
    pub output_name: String,
    pub highlight_kind: String,
    pub environment_hint: String,
}


const FORBIDDEN_WORDS_SKILL_NAME: &[&str] = &[
    concat!("\u{525b}", "\u{624d}"), concat!("\u{525b}", "\u{525b}"), concat!("\u{65b9}", "\u{624d}"), "頃刻", "還有", "系統", "生成", "玩家", "序位", "回合",
    "資料庫", "database", "grid", "debug", "scene_stub", concat!("聖", "塔"), concat!("110", "V"), concat!("念", "場"),
    concat!("富", "態"), concat!("小", "市"), concat!("短", "市"), concat!("青籠", "關"), "HP", "點數",
];

const FORBIDDEN_WORDS_ORIGIN_BASIS: &[&str] = &[
    concat!("\u{525b}", "\u{624d}"), concat!("\u{525b}", "\u{525b}"), concat!("\u{65b9}", "\u{624d}"), "頃刻", "還有", "資料庫", "系統", "生成", "玩家", "序位",
    "回合", "第", "database", "grid", "debug", "scene_stub", concat!("聖", "塔"), concat!("110", "V"), concat!("念", "場"),
    concat!("富", "態"), concat!("小", "市"), concat!("短", "市"), concat!("青籠", "關"), "HP", "點數",
];

const FORBIDDEN_WORDS_COMBAT_PROSE: &[&str] = &[
    "第", "回合", "序位", concat!("\u{525b}", "\u{624d}"), concat!("\u{525b}", "\u{525b}"), concat!("\u{65b9}", "\u{624d}"), "頃刻", "還有", "系統", "生成", "HP",
    "資料庫", "database", "grid", "debug", "scene_stub", concat!("聖", "塔"), concat!("110", "V"), concat!("念", "場"),
    concat!("富", "態"), concat!("小", "市"), concat!("短", "市"), concat!("青籠", "關"), "玩家", "點數",
];

const FORBIDDEN_FABRICATED_RESIDUE: &[&str] = &["燥氣", "霜氣", "暗震", "旋流"];

/// 判定字元是否為台灣正體常用中文字（CJK Ideographs）
#[inline]
fn is_cjk_ideograph(c: char) -> bool {
    matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}' | '\u{f900}'..='\u{faff}')
}

/// 清理器物標籤（去除後贅 `[銳]`、`【凡】` 等）
pub fn clean_vessel_name(vessel: &str) -> String {
    let trimmed = vessel.trim();
    if trimmed.is_empty() || trimmed == "徒手" {
        return String::new();
    }
    let after_prefix = if trimmed.starts_with('【') {
        if let Some((_, rest)) = trimmed.split_once("】・") {
            rest.trim()
        } else if let Some((_, rest)) = trimmed.split_once('】') {
            rest.trim()
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    let clean = after_prefix
        .split(|c| c == '[' || c == '【' || c == '(' || c == '（')
        .next()
        .unwrap_or(after_prefix)
        .trim();
    clean.to_string()
}

/// 驗證招式命名與真意描述是否嚴格契合資料
pub fn verify_skill_output(name: &str, origin_basis: &str, packet: &SkillDataPacket) -> bool {
    // 1. name 長度必須為 2..=7 字元，純繁體中文，無 ASCII 數字、無標點符號與括號
    let name_char_count = name.chars().count();
    if !(2..=7).contains(&name_char_count) {
        return false;
    }
    if name
        .chars()
        .any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c))
    {
        return false;
    }
    if !name.chars().all(is_cjk_ideograph) {
        return false;
    }
    if FORBIDDEN_WORDS_SKILL_NAME.iter().any(|w| name.contains(w)) {
        return false;
    }

    // 2. origin_basis 必須為 12..=90 字元，無 ASCII 數字，無禁忌詞
    let origin_char_count = origin_basis.chars().count();
    if !(12..=90).contains(&origin_char_count) {
        return false;
    }
    if origin_basis
        .chars()
        .any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c))
    {
        return false;
    }
    if FORBIDDEN_WORDS_ORIGIN_BASIS.iter().any(|w| origin_basis.contains(w)) {
        return false;
    }

    // 3. 兵刃與載體保真度檢查
    let is_unarmed = packet.carrier == CarrierAffinity::Unarmed
        && (packet.vessel_name.is_empty() || packet.vessel_name.contains("徒手"));
    if is_unarmed {
        const UNARMED_FORBIDDEN: &[&str] = &[
            "劍", "刀", "刃", "槍", "矛", "戟", "斧", "錘", "鎬", "盾", "棍", "棒",
        ];
        if UNARMED_FORBIDDEN.iter().any(|w| name.contains(w) || origin_basis.contains(w)) {
            return false;
        }
    }
    if packet.carrier == CarrierAffinity::Blunt {
        const BLUNT_FORBIDDEN: &[&str] = &["劍", "槍", "矛"];
        if BLUNT_FORBIDDEN.iter().any(|w| name.contains(w) || origin_basis.contains(w)) {
            return false;
        }
    }
    if packet.carrier == CarrierAffinity::Blade {
        const BLADE_FORBIDDEN: &[&str] = &["錘", "盾", "槍", "矛"];
        if BLADE_FORBIDDEN.iter().any(|w| name.contains(w) || origin_basis.contains(w)) {
            return false;
        }
    }
    if packet.carrier == CarrierAffinity::Pole {
        const POLE_FORBIDDEN: &[&str] = &["劍", "錘", "盾"];
        if POLE_FORBIDDEN.iter().any(|w| name.contains(w) || origin_basis.contains(w)) {
            return false;
        }
    }

    // 4. 武學動作與資料錨定檢查
    if packet.domain == "martial" {
        let mut grounded = false;
        // 動作序列檢查（整詞、雙字子串或單字）
        for motion in &packet.motion_sequence {
            let trimmed = motion.trim();
            if trimmed.is_empty() {
                continue;
            }
            if origin_basis.contains(trimmed) {
                grounded = true;
                break;
            }
            let chars: Vec<char> = trimmed.chars().collect();
            if chars.len() >= 2 {
                for w in chars.windows(2) {
                    let sub: String = w.iter().collect();
                    if origin_basis.contains(&sub) {
                        grounded = true;
                        break;
                    }
                }
            } else if chars.len() == 1 && origin_basis.contains(chars[0]) {
                grounded = true;
                break;
            }
            if grounded {
                break;
            }
        }
        // 器物名稱檢查
        if !grounded {
            let clean_vessel = clean_vessel_name(&packet.vessel_name);
            if !clean_vessel.is_empty() {
                if origin_basis.contains(&clean_vessel) {
                    grounded = true;
                } else {
                    let chars: Vec<char> = clean_vessel.chars().collect();
                    if chars.len() >= 2 {
                        for w in chars.windows(2) {
                            let sub: String = w.iter().collect();
                            if origin_basis.contains(&sub) {
                                grounded = true;
                                break;
                            }
                        }
                    } else if chars.len() == 1 && origin_basis.contains(chars[0]) {
                        grounded = true;
                    }
                }
            }
        }
        // 銘文字元檢查
        if !grounded {
            for wc in &packet.word_chars {
                let trimmed = wc.trim();
                if !trimmed.is_empty() && origin_basis.contains(trimmed) {
                    grounded = true;
                    break;
                }
            }
        }
        if !grounded {
            return false;
        }
    }

    true
}

/// 確定性產生招式名稱與真意描述（無 LLM 時之百分之百安全兜底）
pub fn render_deterministic_skill_from_data(packet: &SkillDataPacket) -> SynthesizedSkillProse {
    let clean_vessel = clean_vessel_name(&packet.vessel_name);
    let is_unarmed = packet.carrier == CarrierAffinity::Unarmed
        || (clean_vessel.is_empty() || clean_vessel.contains("徒手"));

    // 確定動作序列
    let motions = if !packet.motion_sequence.is_empty() {
        packet.motion_sequence.clone()
    } else if !packet.word_chars.is_empty() {
        derive_elementary_motions_from_chars(&packet.word_chars, packet.carrier, TargetPart::Torso)
    } else {
        match packet.carrier {
            CarrierAffinity::Blade => vec!["引刃出鞘".to_string(), "旋腕斜撩".to_string(), "振鋒落定".to_string()],
            CarrierAffinity::Pole => vec!["抖槍探刺".to_string(), "轉桿橫掃".to_string(), "回馬撤步".to_string()],
            CarrierAffinity::Blunt => vec!["引錘蓄力".to_string(), "裂地重砸".to_string(), "收勢回防".to_string()],
            CarrierAffinity::Shield => vec!["沉肘立盾".to_string(), "迎擊化勁".to_string(), "頓地固守".to_string()],
            CarrierAffinity::Footwork => vec!["踩星踏斗".to_string(), "斜身錯位".to_string(), "飄忽回身".to_string()],
            CarrierAffinity::Unarmed => vec!["提步運勁".to_string(), "崩拳破空".to_string(), "收勢凝神".to_string()],
        }
    };

    if packet.domain == "martial" {
        // 招式名稱生成
        let clean_fallback: String = packet
            .fallback_name
            .chars()
            .filter(|c| is_cjk_ideograph(*c))
            .take(7)
            .collect();
        let fallback_len = clean_fallback.chars().count();

        let name = if fallback_len >= 3 {
            // 已有 3~7 字自然名稱，檢查是否犯兵刃衝突
            if is_unarmed {
                let has_weapon_noun = ["劍", "刀", "刃", "槍", "矛", "戟", "斧", "錘", "鎬", "盾", "棍", "棒"]
                    .iter()
                    .any(|w| clean_fallback.contains(w));
                if has_weapon_noun {
                    "行雲拳勢".to_string()
                } else {
                    clean_fallback
                }
            } else {
                clean_fallback
            }
        } else {
            // <= 2 字：根據載體與手持器物補齊風味後綴
            let base = if fallback_len > 0 {
                if is_unarmed
                    && ["劍", "刀", "刃", "槍", "矛", "戟", "斧", "錘", "鎬", "盾", "棍", "棒"]
                        .iter()
                        .any(|w| clean_fallback.contains(w))
                {
                    "行雲".to_string()
                } else {
                    clean_fallback
                }
            } else if !packet.word_chars.is_empty() {
                let filtered: String = packet
                    .word_chars
                    .iter()
                    .filter_map(|s| s.chars().find(|c| is_cjk_ideograph(*c)))
                    .take(2)
                    .collect();
                if is_unarmed
                    && ["劍", "刀", "刃", "槍", "矛", "戟", "斧", "錘", "鎬", "盾", "棍", "棒"]
                        .iter()
                        .any(|w| filtered.contains(w))
                {
                    "凝勢".to_string()
                } else if filtered.is_empty() {
                    "凝勢".to_string()
                } else {
                    filtered
                }
            } else {
                match packet.slot_label.as_str() {
                    "起手" => "凝勢".to_string(),
                    "連攜" => "流轉".to_string(),
                    "破綻" => "絕殺".to_string(),
                    "守勢" => "固元".to_string(),
                    _ => "聚勁".to_string(),
                }
            };

            let suffix = match packet.carrier {
                CarrierAffinity::Blade => {
                    if clean_vessel.contains("劍") {
                        "劍式"
                    } else {
                        "刀式"
                    }
                }
                CarrierAffinity::Blunt => {
                    if clean_vessel.contains("錘") {
                        "錘法"
                    } else {
                        "重式"
                    }
                }
                CarrierAffinity::Pole => {
                    if clean_vessel.contains("槍") {
                        "槍式"
                    } else {
                        "桿法"
                    }
                }
                CarrierAffinity::Shield => "架式",
                CarrierAffinity::Footwork => "步法",
                CarrierAffinity::Unarmed => "拳勢",
            };
            format!("{}{}", base, suffix)
        };

        // 真意演繹（origin_basis）生成
        let m1 = motions.first().map(|s| s.as_str()).unwrap_or("運勁");
        let m2 = motions.get(1).map(|s| s.as_str()).unwrap_or("承接");
        let m3 = motions.get(2).map(|s| s.as_str()).unwrap_or("收合");

        let origin_basis = if !clean_vessel.is_empty() && !is_unarmed {
            format!("手執{}，由「{}」起勢，順接「{}」，再以「{}」一氣呵成。", clean_vessel, m1, m2, m3)
        } else {
            format!("徒手運勁，由「{}」起勢，順接「{}」，再以「{}」連貫成招。", m1, m2, m3)
        };

        SynthesizedSkillProse { name, origin_basis }
    } else {
        // production 百工領域
        let clean_fallback: String = packet
            .fallback_name
            .chars()
            .filter(|c| is_cjk_ideograph(*c))
            .take(7)
            .collect();
        let name = if (2..=7).contains(&clean_fallback.chars().count()) {
            clean_fallback
        } else {
            format!("{}工法", packet.slot_label)
        };

        let motion_str = motions.join("、");
        let origin_basis = if !clean_vessel.is_empty() && clean_vessel != "徒手" {
            format!("手持{}，經由{}之序，工法細緻，熟能生巧化為技藝。", clean_vessel, motion_str)
        } else {
            format!("專注身手，經由{}之序，工法細緻，熟能生巧化為技藝。", motion_str)
        };

        SynthesizedSkillProse { name, origin_basis }
    }
}

// ---------------------------------------------------------------------------
// 招式文采快取與 LLM 背景生成
// ---------------------------------------------------------------------------

const SKILL_CACHE_FILE_PATH: &str = "data/runtime/llm_skill_cache.json";

fn compute_skill_cache_key(packet: &SkillDataPacket) -> u64 {
    let mut hasher = DefaultHasher::new();
    packet.domain.hash(&mut hasher);
    packet.slot_label.hash(&mut hasher);
    packet.word_chars.hash(&mut hasher);
    packet.vessel_name.hash(&mut hasher);
    packet.carrier.hash(&mut hasher);
    packet.motion_sequence.hash(&mut hasher);
    packet.fallback_name.hash(&mut hasher);
    hasher.finish()
}

fn load_skill_disk_cache() -> HashMap<u64, SynthesizedSkillProse> {
    let path = Path::new(SKILL_CACHE_FILE_PATH);
    if !path.exists() {
        return HashMap::new();
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    if let Ok(map) = serde_json::from_str::<HashMap<String, SynthesizedSkillProse>>(&content) {
        let mut res = HashMap::new();
        for (k, v) in map {
            if let Ok(key_u64) = k.parse::<u64>() {
                res.insert(key_u64, v);
            }
        }
        return res;
    }
    HashMap::new()
}

fn save_skill_disk_cache(cache: &HashMap<u64, SynthesizedSkillProse>) {
    let path = Path::new(SKILL_CACHE_FILE_PATH);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let string_map: HashMap<String, &SynthesizedSkillProse> =
        cache.iter().map(|(k, v)| (k.to_string(), v)).collect();
    if let Ok(json_str) = serde_json::to_string_pretty(&string_map) {
        let _ = std::fs::write(path, json_str);
    }
}

static SKILL_PROSE_CACHE: LazyLock<RwLock<HashMap<u64, SynthesizedSkillProse>>> =
    LazyLock::new(|| RwLock::new(load_skill_disk_cache()));

static IN_FLIGHT_SKILL_KEYS: LazyLock<RwLock<HashSet<u64>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

fn get_skill_from_cache(key: u64) -> Option<SynthesizedSkillProse> {
    SKILL_PROSE_CACHE.read().get(&key).cloned()
}

fn store_skill_in_cache(key: u64, prose: SynthesizedSkillProse) {
    let mut cache = SKILL_PROSE_CACHE.write();
    cache.insert(key, prose);
    save_skill_disk_cache(&cache);
}

fn mark_skill_in_flight(key: u64) -> bool {
    let mut set = IN_FLIGHT_SKILL_KEYS.write();
    set.insert(key)
}

fn clear_skill_in_flight(key: u64) {
    let mut set = IN_FLIGHT_SKILL_KEYS.write();
    set.remove(&key);
}


fn call_local_chat_llm(
    base_url: &str,
    model: &str,
    timeout: Duration,
    system_prompt: &str,
    user_prompt: &str,
    json_mode: bool,
) -> anyhow::Result<String> {
    let raw = base_url.trim().trim_end_matches('/');
    if raw.is_empty() || model.trim().is_empty() {
        anyhow::bail!("LLM base_url or model is empty");
    }
    let endpoint = if raw.ends_with("/chat/completions") {
        raw.to_string()
    } else {
        format!("{raw}/chat/completions")
    };

    let mut body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ],
        "temperature": 0.35,
        "max_tokens": 180,
        "cache_prompt": true,
        "chat_template_kwargs": {
            "enable_thinking": false
        }
    });

    if json_mode {
        if let Some(obj) = body.as_object_mut() {
            obj.insert(
                "response_format".to_string(),
                serde_json::json!({"type": "json_object"}),
            );
        }
    }

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_millis(600).min(timeout))
        .timeout(timeout)
        .build()?;

    let resp = client.post(&endpoint).json(&body).send()?;
    if !resp.status().is_success() {
        anyhow::bail!("LLM HTTP status: {}", resp.status());
    }

    #[derive(Deserialize)]
    struct OpenAiChoice {
        message: OpenAiMsg,
    }
    #[derive(Deserialize)]
    struct OpenAiMsg {
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        reasoning_content: Option<String>,
    }
    #[derive(Deserialize)]
    struct OpenAiResponse {
        choices: Vec<OpenAiChoice>,
    }

    let out: OpenAiResponse = resp.json()?;
    let choice = out
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty choices from LLM"))?;

    let raw_text = choice
        .message
        .content
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| choice.message.reasoning_content.as_deref())
        .unwrap_or("");

    let reply = crate::ai::talk::strip_think_blocks(raw_text.trim());
    if reply.is_empty() {
        anyhow::bail!("empty reply from LLM");
    }
    Ok(reply)
}

/// 同步呼叫本地 LLM 提煉招式真意（供呼叫方與測試使用）
pub fn call_llm_for_skill_data(packet: &SkillDataPacket) -> Option<SynthesizedSkillProse> {
    let base_url = std::env::var("SW_NARRATIVE_LLM_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".to_string());
    let model = "google_gemma-4-26B-A4B-it-Q4_1";

    let system_prompt = concat!(
        "你是一位武俠與百工世界的招式命名與真意演繹者。請依據提供的動作序列、銘文字元與載體器物，提煉招式名稱與演繹真意。\
嚴格要求：\
1. 嚴格使用台灣正體中文。\
2. 絕對不可出現任何阿拉伯數字（0-9），不可出現「",
        concat!("\u{525b}", "\u{624d}"),
        "」、「",
        concat!("\u{525b}", "\u{525b}"),
        "」、「",
        concat!("\u{65b9}", "\u{624d}"),
        "」、「頃刻」、「還有」、「資料庫」、「系統」、「生成」、「玩家」、「序位」、「回合」、「第」、「主樞」、「詞元」、「剝名」、「識海」、「字譜」等字眼。\
3. 招式名稱必須為2至7字繁體中文，不可有標點符號或括號。\
4. 演繹真意（origin_basis）必須為12至90字，必須切實反映提供的動作序列與手持載體。\
5. 必須僅以標準 JSON 格式輸出：{\"name\":\"招式名\",\"origin_basis\":\"演繹真意\"}，不可包含其他文字。"
    );

    let user_prompt = format!(
        "領域：{}\n運動鏈階段：{}\n施展路數：{}\n手持器物：{}\n器物類別：{}\n研習銘文：{}\n分解碎招動作：{}\n原名參考：{}",
        packet.domain,
        packet.hub_or_tier,
        packet.slot_label,
        packet.vessel_name,
        packet.carrier.label(),
        packet.word_chars.join("、"),
        packet.motion_sequence.join(" → "),
        packet.fallback_name,
    );

    let raw_reply = call_local_chat_llm(
        &base_url,
        model,
        Duration::from_secs(6),
        system_prompt,
        &user_prompt,
        true,
    )
    .ok()?;

    #[derive(Deserialize)]
    struct SkillLlmJson {
        name: String,
        origin_basis: String,
    }
    let parsed: SkillLlmJson = crate::ai::jsonish::parse_jsonish(&raw_reply)?;
    let name = parsed.name.trim().to_string();
    let origin_basis = parsed.origin_basis.trim().to_string();

    if verify_skill_output(&name, &origin_basis, packet) {
        Some(SynthesizedSkillProse { name, origin_basis })
    } else {
        None
    }
}

/// 綜合招式資料生成或檢索招式真意文采
pub fn synthesize_skill_from_data(packet: &SkillDataPacket) -> SynthesizedSkillProse {
    let key = compute_skill_cache_key(packet);

    // 1. 快取命中且通過保真度驗證
    if let Some(cached) = get_skill_from_cache(key) {
        if verify_skill_output(&cached.name, &cached.origin_basis, packet) {
            return cached;
        }
    }

    // 2. 測試環境或明確關閉 LLM 時直接確定性產出
    let enable_llm = std::env::var("SW_ENABLE_SKILL_LLM").unwrap_or_else(|_| {
        if cfg!(test) {
            "0".to_string()
        } else {
            "1".to_string()
        }
    });

    if enable_llm == "0" && std::env::var("SW_SYNC_SKILL_LLM").as_deref() != Ok("1") {
        return render_deterministic_skill_from_data(packet);
    }

    // 3. 同步要求模式
    if std::env::var("SW_SYNC_SKILL_LLM").as_deref() == Ok("1") {
        if let Some(prose) = call_llm_for_skill_data(packet) {
            store_skill_in_cache(key, prose.clone());
            return prose;
        }
        return render_deterministic_skill_from_data(packet);
    }

    // 4. 正式運行背景生成（防抖防重複呼叫）
    if mark_skill_in_flight(key) {
        let packet_clone = packet.clone();
        std::thread::spawn(move || {
            if let Some(prose) = call_llm_for_skill_data(&packet_clone) {
                store_skill_in_cache(key, prose);
            }
            clear_skill_in_flight(key);
        });
    }

    render_deterministic_skill_from_data(packet)
}

// ---------------------------------------------------------------------------
// 交鋒文字敘事驗證與生成
// ---------------------------------------------------------------------------


include!("narrative/combat.rs");
include!("narrative/craft_tests.rs");
