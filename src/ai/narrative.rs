// narrative 模組 — LLM 數據轉描述與嚴格事實保真度（Fact-Fidelity）驗證管線。
// 嚴守台灣正體中文，徹底拔除第四面牆詞彙與即時性虛詞。後台數據嚴謹，前台體感去數值化。

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

/// 招式數據封包（後台結構化詞元與動作資料）
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

/// 單節交鋒事實封包（後台物理判定與招式碰擊斷點數據）
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
    "剛才", "剛剛", "方才", "頃刻", "還有", "系統", "生成", "玩家", "序位", "回合",
    "資料庫", "database", "grid", "debug", "scene_stub", "聖塔", "110V", "念場",
    "富態", "小市", "短市", "青籠關", "HP", "點數",
];

const FORBIDDEN_WORDS_ORIGIN_BASIS: &[&str] = &[
    "剛才", "剛剛", "方才", "頃刻", "還有", "資料庫", "系統", "生成", "玩家", "序位",
    "回合", "第", "database", "grid", "debug", "scene_stub", "聖塔", "110V", "念場",
    "富態", "小市", "短市", "青籠關", "HP", "點數",
];

const FORBIDDEN_WORDS_COMBAT_PROSE: &[&str] = &[
    "第", "回合", "序位", "剛才", "剛剛", "方才", "頃刻", "還有", "系統", "生成", "HP",
    "資料庫", "database", "grid", "debug", "scene_stub", "聖塔", "110V", "念場",
    "富態", "小市", "短市", "青籠關", "玩家", "點數",
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
    let clean = trimmed
        .split(|c| c == '[' || c == '【' || c == '(' || c == '（')
        .next()
        .unwrap_or(trimmed)
        .trim();
    clean.to_string()
}

/// 驗證招式命名與真意描述是否嚴格契合數據
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

    // 4. 武學動作與數據錨定檢查
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
        // 詞元字元檢查
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

fn extract_json_object(s: &str) -> Option<&str> {
    let trimmed = s.trim();
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end > start {
        Some(&trimmed[start..=end])
    } else {
        None
    }
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

/// 同步呼叫本地 LLM 提煉招式真意（供調用方與測試使用）
pub fn call_llm_for_skill_data(packet: &SkillDataPacket) -> Option<SynthesizedSkillProse> {
    let base_url = std::env::var("SW_NARRATIVE_LLM_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".to_string());
    let model = "google_gemma-4-26B-A4B-it-Q4_1";

    let system_prompt = "你是一位武俠與百工世界的招式命名與真意演繹者。請依據提供的動作序列、詞元字元與載體器物，提煉招式名稱與演繹真意。\
嚴格要求：\
1. 嚴格使用台灣正體中文。\
2. 絕對不可出現任何阿拉伯數字（0-9），不可出現「剛才」、「剛剛」、「方才」、「頃刻」、「還有」、「資料庫」、「系統」、「生成」、「玩家」、「序位」、「回合」、「第」等字眼。\
3. 招式名稱必須為2至7字繁體中文，不可有標點符號或括號。\
4. 演繹真意（origin_basis）必須為12至90字，必須切實反映提供的動作序列與手持載體。\
5. 必須僅以標準 JSON 格式輸出：{\"name\":\"招式名\",\"origin_basis\":\"演繹真意\"}，不可包含其他文字。";

    let user_prompt = format!(
        "領域：{}\n境界或主樞：{}\n施展路數：{}\n手持器物：{}\n器物類別：{}\n所學字印：{}\n分解碎招動作：{}\n原名參考：{}",
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

    let json_str = extract_json_object(&raw_reply)?;
    #[derive(Deserialize)]
    struct SkillLlmJson {
        name: String,
        origin_basis: String,
    }
    let parsed: SkillLlmJson = serde_json::from_str(json_str).ok()?;
    let name = parsed.name.trim().to_string();
    let origin_basis = parsed.origin_basis.trim().to_string();

    if verify_skill_output(&name, &origin_basis, packet) {
        Some(SynthesizedSkillProse { name, origin_basis })
    } else {
        None
    }
}

/// 綜合招式數據生成或檢索招式真意文采
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

    // 4. 正式運行背景生成（防抖防重複調用）
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

/// 驗證單節交鋒敘事段落是否嚴格契合數據
pub fn verify_combat_bout_output(prose: &str, packet: &CombatBoutFactPacket) -> bool {
    let char_count = prose.chars().count();
    if !(20..=180).contains(&char_count) {
        return false;
    }

    // 禁用 ASCII 數字與全形數字 ('0'..='9', '０'..='９')
    if prose
        .chars()
        .any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c))
    {
        return false;
    }

    // 殘留氣機驗證：
    // 若 active_residue 為 None，禁止捏造殘留特徵詞彙
    // 若 active_residue 為 Some(res)，要求至少有一非通用字元與 res 重合
    if let Some(res) = &packet.active_residue {
        let meaningful_chars: Vec<char> = res.chars().filter(|&c| c != '氣').collect();
        let has_overlap = if meaningful_chars.is_empty() {
            res.chars().any(|c| prose.contains(c))
        } else {
            meaningful_chars.into_iter().any(|c| prose.contains(c))
        };
        if !has_overlap {
            return false;
        }
    } else if FORBIDDEN_FABRICATED_RESIDUE.iter().any(|w| prose.contains(w)) {
        return false;
    }
    // 禁用第四面牆與即時性禁忌詞
    if FORBIDDEN_WORDS_COMBAT_PROSE.iter().any(|w| prose.contains(w)) {
        return false;
    }

    // 攻守雙方主詞檢查：必須同時出現進攻方與防守方稱呼
    if !packet.attacker_name.is_empty() && !prose.contains(&packet.attacker_name) {
        return false;
    }
    if !packet.defender_name.is_empty() && !prose.contains(&packet.defender_name) {
        return false;
    }

    // 勝負顯化檢查
    if packet.winner_side == "attacker" || packet.winner_side == "defender" {
        // 必須包含勝方顯化之招式名稱
        if !prose.contains(&packet.winner_skill_name) {
            return false;
        }
        // 敗方招式被打散為碎招，絕不可出現敗方招式名稱
        if !packet.loser_skill_name.is_empty()
            && packet.loser_skill_name != packet.winner_skill_name
            && prose.contains(&packet.loser_skill_name)
        {
            return false;
        }
    }

    // 動作數據錨定檢查：必須包含勝方或敗方動作的關鍵詞或雙字子串
    let mut motion_grounded = false;
    for m in packet
        .winner_motions
        .iter()
        .chain(packet.loser_attempted_motions.iter())
    {
        let trimmed = m.trim();
        if trimmed.is_empty() {
            continue;
        }
        if prose.contains(trimmed) {
            motion_grounded = true;
            break;
        }
        let chars: Vec<char> = trimmed.chars().collect();
        if chars.len() >= 2 {
            for w in chars.windows(2) {
                let sub: String = w.iter().collect();
                if prose.contains(&sub) {
                    motion_grounded = true;
                    break;
                }
            }
        } else if chars.len() == 1 && prose.contains(chars[0]) {
            motion_grounded = true;
            break;
        }
        if motion_grounded {
            break;
        }
    }
    if !motion_grounded {
        return false;
    }

    // 兵刃白名單檢查：
    // 若雙方皆非 Blade（且雙方兵刃名稱皆無 刀/劍/刃），不可出現 劍、刀鋒、刀光
    let has_blade = packet.attacker_carrier == CarrierAffinity::Blade
        || packet.defender_carrier == CarrierAffinity::Blade
        || packet.attacker_weapon.contains('刀')
        || packet.attacker_weapon.contains('劍')
        || packet.attacker_weapon.contains('刃')
        || packet.defender_weapon.contains('刀')
        || packet.defender_weapon.contains('劍')
        || packet.defender_weapon.contains('刃');

    if !has_blade && (prose.contains('劍') || prose.contains("刀鋒") || prose.contains("刀光")) {
        return false;
    }

    // 若雙方皆非 Pole，不可出現 長槍、槍尖
    let has_pole = packet.attacker_carrier == CarrierAffinity::Pole
        || packet.defender_carrier == CarrierAffinity::Pole
        || packet.attacker_weapon.contains('槍')
        || packet.attacker_weapon.contains('矛')
        || packet.attacker_weapon.contains('戟')
        || packet.defender_weapon.contains('槍')
        || packet.defender_weapon.contains('矛')
        || packet.defender_weapon.contains('戟');

    if !has_pole && (prose.contains("長槍") || prose.contains("槍尖")) {
        return false;
    }

    true
}

/// 確定性渲染交鋒段落（純數據組裝，無第四面牆與違禁字）
pub fn render_deterministic_bout_from_data(packet: &CombatBoutFactPacket) -> String {
    let clean_atk_w = clean_vessel_name(&packet.attacker_weapon);
    let clean_def_w = clean_vessel_name(&packet.defender_weapon);

    let winner_is_attacker = packet.winner_side == "attacker";
    let winner_is_defender = packet.winner_side == "defender";

    let w_motions = if packet.winner_motions.is_empty() {
        "連貫變招".to_string()
    } else {
        packet.winner_motions.join("、")
    };

    let l_motions = if packet.loser_attempted_motions.is_empty() {
        "起勢應對".to_string()
    } else {
        packet.loser_attempted_motions.join("、")
    };

    let counter = packet.counter_reason.as_deref().unwrap_or("搶佔空門");
    let hit_part = packet.hit_part.label();
    let severity = &packet.severity_label;

    let residue_prefix = match (&packet.active_residue, &packet.residue_interaction) {
        (Some(res), Some(inter)) => format!("場中{}未散，{}，", res, inter),
        (Some(res), None) => format!("場中{}未散，", res),
        (None, Some(inter)) => format!("{}，", inter),
        (None, None) => String::new(),
    };

    let impact_tail = match (&packet.inflicted_injury, &packet.loser_posture_after) {
        (Some(inj), Some(pos)) => format!("致{}，陷入{}，{}", inj, severity, pos),
        (Some(inj), None) => format!("致{}，陷入{}", inj, severity),
        (None, Some(pos)) => format!("陷入{}，{}", severity, pos),
        (None, None) => format!("陷入{}", severity),
    };

    if winner_is_attacker {
        let atk_w_desc = if clean_atk_w.is_empty() || clean_atk_w == "徒手" {
            "徒手運勁".to_string()
        } else {
            format!("手執{}", clean_atk_w)
        };
        format!(
            "{}{}以{}施展「{}」，由{}順勢貫通；{}雖試圖以{}應招，卻遭{}中途截斷化為碎招，{}受創，{}。",
            residue_prefix,
            packet.attacker_name,
            atk_w_desc,
            packet.winner_skill_name,
            w_motions,
            packet.defender_name,
            l_motions,
            counter,
            hit_part,
            impact_tail,
        )
    } else if winner_is_defender {
        let def_w_desc = if clean_def_w.is_empty() || clean_def_w == "徒手" {
            "徒手運勁".to_string()
        } else {
            format!("手執{}", clean_def_w)
        };
        format!(
            "{}{}以{}迎擊施展「{}」，經{}一氣呵成；{}進襲的{}被{}化為碎招，{}遭挫，{}。",
            residue_prefix,
            packet.defender_name,
            def_w_desc,
            packet.winner_skill_name,
            w_motions,
            packet.attacker_name,
            l_motions,
            counter,
            hit_part,
            impact_tail,
        )
    } else {
        // Draw / 不分勝負
        let w_name = if packet.winner_skill_name.is_empty() {
            "招式".to_string()
        } else {
            format!("「{}」", packet.winner_skill_name)
        };
        format!(
            "{}{}施展{}經由{}搶攻，{}亦以{}寸步不讓，雙方氣機交錯散為碎招，各退半步暫歛鋒芒。",
            residue_prefix,
            packet.attacker_name,
            w_name,
            w_motions,
            packet.defender_name,
            l_motions,
        )
    }
}

// ---------------------------------------------------------------------------
// 交鋒敘事快取與 LLM 即時生成
// ---------------------------------------------------------------------------

fn compute_combat_bout_cache_key(packet: &CombatBoutFactPacket) -> u64 {
    let mut hasher = DefaultHasher::new();
    packet.slot.hash(&mut hasher);
    packet.attacker_name.hash(&mut hasher);
    packet.defender_name.hash(&mut hasher);
    packet.attacker_weapon.hash(&mut hasher);
    packet.defender_weapon.hash(&mut hasher);
    packet.attacker_carrier.hash(&mut hasher);
    packet.defender_carrier.hash(&mut hasher);
    packet.winner_side.hash(&mut hasher);
    packet.winner_skill_name.hash(&mut hasher);
    packet.loser_skill_name.hash(&mut hasher);
    packet.winner_motions.hash(&mut hasher);
    packet.loser_attempted_motions.hash(&mut hasher);
    packet.counter_reason.hash(&mut hasher);
    packet.hit_part.hash(&mut hasher);
    packet.severity_label.hash(&mut hasher);
    packet.active_residue.hash(&mut hasher);
    packet.residue_interaction.hash(&mut hasher);
    packet.loser_posture_after.hash(&mut hasher);
    packet.inflicted_injury.hash(&mut hasher);
    hasher.finish()
}

static COMBAT_BOUT_CACHE: LazyLock<RwLock<HashMap<u64, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn get_combat_bout_from_cache(key: u64) -> Option<String> {
    COMBAT_BOUT_CACHE.read().get(&key).cloned()
}

fn store_combat_bout_in_cache(key: u64, prose: String) {
    let mut cache = COMBAT_BOUT_CACHE.write();
    if cache.len() > 1000 {
        cache.clear();
    }
    cache.insert(key, prose);
}

/// 綜合交鋒事實數據生成精緻敘事段落
pub fn compose_combat_bout_from_data(packet: &CombatBoutFactPacket) -> String {
    let key = compute_combat_bout_cache_key(packet);

    if let Some(cached) = get_combat_bout_from_cache(key) {
        if verify_combat_bout_output(&cached, packet) {
            return cached;
        }
    }

    if cfg!(test) || std::env::var("SW_DISABLE_NARRATIVE_LLM").unwrap_or_default() == "1" {
        return render_deterministic_bout_from_data(packet);
    }

    let base_url = std::env::var("SW_NARRATIVE_LLM_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".to_string());
    let model = "google_gemma-4-26B-A4B-it-Q4_1";

    let system_prompt = "你是一位武俠與百工世界的戰鬥敘事演繹者。請依據提供的戰鬥事實數據，編撰一段精準、連貫、富有武俠節奏的交鋒描述（25至150字）。\
嚴格要求：\
1. 嚴格使用台灣正體中文。\
2. 絕對不可出現任何阿拉伯數字（0-9）或全形數字（０-９），不可出現「第」、「回合」、「序位」、「剛才」、「剛剛」、「方才」、「頃刻」、「還有」、「系統」、「生成」、「HP」等字眼。\
3. 必須明確寫出進攻方與防守方的稱呼，並描寫勝方如何將分解碎招動作一氣呵成連貫使出「勝方技能名稱」（必須以「」括起招式名自然融入句中，不可單獨掛在句尾）。\
4. 敗方僅做出前半截碎招動作便被中途截斷潰散，絕對不可替敗方編造招式名稱。\
5. 必須依據提供的器物、相剋原因、受創部位與受創程度描寫結果，不可憑空捏造未持有的兵器。\
6. 僅輸出最終敘事段落，不可有任何額外說明或前綴。";

    let (winner_display, loser_display) = if packet.winner_side == "attacker" {
        (packet.attacker_name.as_str(), packet.defender_name.as_str())
    } else if packet.winner_side == "defender" {
        (packet.defender_name.as_str(), packet.attacker_name.as_str())
    } else {
        ("雙方平分秋色", "雙方")
    };
    // 絕對不將 loser_skill_name 傳入 prompt！
    let mut user_prompt = format!(
        "交鋒階段：{}\n進攻方：{}（手持：{}，路數：{}）\n防守方：{}（手持：{}，路數：{}）\n成招勝方：{}\n勝方完整連貫之碎招動作：{}\n勝方顯化之技能名稱：「{}」\n受阻敗方：{}\n敗方中途被打斷之碎招動作：{}\n破招與相剋原因：{}\n敗方受創部位：{}\n受創程度：{}",
        packet.slot.label(),
        packet.attacker_name,
        packet.attacker_weapon,
        packet.attacker_carrier.label(),
        packet.defender_name,
        packet.defender_weapon,
        packet.defender_carrier.label(),
        winner_display,
        packet.winner_motions.join(" → "),
        packet.winner_skill_name,
        loser_display,
        packet.loser_attempted_motions.join(" → "),
        packet.counter_reason.as_deref().unwrap_or("搶佔空門截斷氣機"),
        packet.hit_part.label(),
        packet.severity_label,
    );
    if let Some(res) = &packet.active_residue {
        user_prompt.push_str(&format!("\n現場殘留氣機：{}", res));
    }
    if let Some(inter) = &packet.residue_interaction {
        user_prompt.push_str(&format!("\n氣機交互狀態：{}", inter));
    }
    if let Some(posture) = &packet.loser_posture_after {
        user_prompt.push_str(&format!("\n敗方受創後架勢：{}", posture));
    }
    if let Some(injury) = &packet.inflicted_injury {
        user_prompt.push_str(&format!("\n敗方具體傷勢：{}", injury));
    }
    match call_local_chat_llm(
        &base_url,
        model,
        Duration::from_millis(3800),
        system_prompt,
        &user_prompt,
        false,
    ) {
        Ok(reply) => {
            let cleaned = reply.trim();
            let stripped = cleaned
                .trim_matches(|c| c == '"' || c == '\'' || c == '「' || c == '」' || c == '`');
            if verify_combat_bout_output(stripped, packet) {
                let prose = stripped.to_string();
                store_combat_bout_in_cache(key, prose.clone());
                return prose;
            }
            render_deterministic_bout_from_data(packet)
        }
        Err(_) => render_deterministic_bout_from_data(packet),
    }
}

/// 並發組裝多節交鋒敘事段落（保持原本序位，以 std::thread::scope 同步並發）
pub fn compose_combat_bouts_concurrent(packets: &[CombatBoutFactPacket]) -> Vec<String> {
    if packets.is_empty() {
        return Vec::new();
    }
    std::thread::scope(|s| {
        let handles: Vec<_> = packets
            .iter()
            .map(|packet| s.spawn(|| compose_combat_bout_from_data(packet)))
            .collect();
        handles
            .into_iter()
            .zip(packets.iter())
            .map(|(handle, packet)| {
                handle
                    .join()
                    .unwrap_or_else(|_| render_deterministic_bout_from_data(packet))
            })
            .collect()
    })
}

// ---------------------------------------------------------------------------
// 百工匠造文采生成與 Fact-Fidelity 保真度檢驗
// ---------------------------------------------------------------------------

/// 驗證百工產出文采是否嚴守無數值化與事實錨定
pub fn verify_craft_output(text: &str, packet: &CraftActionFactPacket) -> Result<(), String> {
    let trimmed = text.trim();
    let char_count = trimmed.chars().count();
    if char_count < 25 {
        return Err("百工文采長度不足二十五字".to_string());
    }
    if char_count > 260 {
        return Err("百工文采長度超過二百六十字".to_string());
    }

    if trimmed.chars().any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c)) {
        return Err("百工文采嚴禁包含阿拉伯數字或全形數字".to_string());
    }
    if trimmed.contains('%') || trimmed.contains('％') || trimmed.contains('+') || trimmed.contains('＋') {
        return Err("百工文采嚴禁包含百分比或加號符號".to_string());
    }

    const FORBIDDEN_WORDS_CRAFT: &[&str] = &[
        "系統", "玩家", "生成", "剛才", "剛剛", "方才", "頃刻", "還有", "資料庫", "database", "grid", "debug",
        "scene_stub", "聖塔", "110v", "念場", "富態", "小市", "短市", "青籠關", "wt_", "mat_", "npc_", "hp", "數值", "點數",
    ];
    let lower = trimmed.to_lowercase();
    for &fw in FORBIDDEN_WORDS_CRAFT {
        if lower.contains(fw) {
            return Err(format!("百工文采包含禁用詞語：{fw}"));
        }
    }

    // 必須提及產出物名稱（或其二字字尾）
    let clean_output = clean_vessel_name(&packet.output_name);
    let output_chars: Vec<char> = clean_output.chars().collect();
    let output_suffix_2: Option<String> = if output_chars.len() >= 2 {
        Some(output_chars[output_chars.len() - 2..].iter().collect())
    } else {
        None
    };

    let output_matched = (!packet.output_name.is_empty() && trimmed.contains(&packet.output_name))
        || (!clean_output.is_empty() && trimmed.contains(&clean_output))
        || output_suffix_2.as_deref().map(|s| trimmed.contains(s)).unwrap_or(false);

    if !output_matched {
        return Err("百工文采未提及產出物名稱或其字尾".to_string());
    }

    // 必須提及至少一種所用材料或工器具
    let mut tool_or_mat_matched = false;
    let clean_tool = clean_vessel_name(&packet.tool_name);
    if !packet.tool_name.is_empty() && trimmed.contains(&packet.tool_name) {
        tool_or_mat_matched = true;
    } else if !clean_tool.is_empty() && trimmed.contains(&clean_tool) {
        tool_or_mat_matched = true;
    } else {
        let tool_chars: Vec<char> = clean_tool.chars().collect();
        if tool_chars.len() >= 2 {
            let suffix: String = tool_chars[tool_chars.len() - 2..].iter().collect();
            if trimmed.contains(&suffix) {
                tool_or_mat_matched = true;
            }
        }
    }

    if !tool_or_mat_matched {
        for mat in &packet.material_names {
            let clean_mat = clean_vessel_name(mat);
            if !mat.is_empty() && trimmed.contains(mat) {
                tool_or_mat_matched = true;
                break;
            }
            if !clean_mat.is_empty() && trimmed.contains(&clean_mat) {
                tool_or_mat_matched = true;
                break;
            }
            let mat_chars: Vec<char> = clean_mat.chars().collect();
            if mat_chars.len() >= 2 {
                let suffix: String = mat_chars[mat_chars.len() - 2..].iter().collect();
                if trimmed.contains(&suffix) {
                    tool_or_mat_matched = true;
                    break;
                }
            }
        }
    }

    if !tool_or_mat_matched {
        return Err("百工文采未提及任何所用材料或工器具".to_string());
    }

    Ok(())
}

/// 確定性百工匠造文采保底（無 LLM 或驗證失敗時之純文字兜底）
pub fn deterministic_craft_highlight_fallback(packet: &CraftActionFactPacket) -> String {
    let actor = if packet.actor_name.trim().is_empty() {
        "工匠"
    } else {
        packet.actor_name.trim()
    };
    let tool = if packet.tool_name.trim().is_empty() {
        "器具"
    } else {
        packet.tool_name.trim()
    };
    let mat = packet
        .material_names
        .first()
        .map(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("素材");
    let out = if packet.output_name.trim().is_empty() {
        "器物"
    } else {
        packet.output_name.trim()
    };
    let habit = if packet.breath_habit.trim().is_empty() {
        "沉凝調息"
    } else {
        packet.breath_habit.trim()
    };
    let env = if packet.environment_hint.trim().is_empty() {
        "案前工坊"
    } else {
        packet.environment_hint.trim()
    };
    let fam = if packet.tool_familiarity.trim().is_empty() {
        "熟稔"
    } else {
        packet.tool_familiarity.trim()
    };
    let highlight = if packet.highlight_kind.trim().is_empty() {
        "火候純熟"
    } else {
        packet.highlight_kind.trim()
    };

    format!(
        "身處{}，{}運轉{}之息，手持{}更顯{}，凝神調理{}，處處顯露{}，終成{}，鋒芒神彩沉凝內斂。",
        env, actor, habit, tool, fam, mat, highlight, out
    )
}

/// 呼叫本地 LLM 或回退確定性管線生成百工匠造高光文采
pub fn compose_craft_highlight_from_data(packet: &CraftActionFactPacket) -> String {
    let enable_llm = std::env::var("SW_ENABLE_CRAFT_LLM").unwrap_or_else(|_| {
        if cfg!(test) {
            "0".to_string()
        } else {
            "1".to_string()
        }
    });

    if enable_llm == "1" {
        let base_url = std::env::var("SW_NARRATIVE_LLM_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".to_string());
        let model = "google_gemma-4-26B-A4B-it-Q4_1";

        let system_prompt = "你是一位百工匠造與山林採擷的文采演繹者。請依據工匠身手、持器心境、內息節奏與所用材料，描摹一段百工產出過程的光彩體感。\
嚴格要求：\
1. 嚴格使用台灣正體中文。\
2. 絕對不可出現任何阿拉伯數字（0-9）或全形數字（０-９），不可出現百分比符號或加號。\
3. 絕對不可出現「系統」、「玩家」、「生成」、「剛才」、「剛剛」、「方才」、「頃刻」、「還有」、「資料庫」、「database」、「HP」、「數值」等字眼。\
4. 描述長度必須在三十至二百二十字之間。\
5. 必須明確提及最終產出物名稱與所用材料或工具器具。\
6. 直接輸出這段純文字描述，切勿包含任何前後引言、標籤或額外格式。";

        let materials_str = if packet.material_names.is_empty() {
            "各色素材".to_string()
        } else {
            packet.material_names.join("、")
        };

        let user_prompt = format!(
            "工匠姓名：{}\n手持工具：{}（熟悉度：{}）\n內息底色：{}\n所用材料：{}\n產出成品：{}\n工藝特徵：{}\n環境氛圍：{}",
            packet.actor_name,
            packet.tool_name,
            packet.tool_familiarity,
            packet.breath_habit,
            materials_str,
            packet.output_name,
            packet.highlight_kind,
            packet.environment_hint,
        );

        if let Ok(raw_reply) = call_local_chat_llm(
            &base_url,
            model,
            Duration::from_secs(6),
            system_prompt,
            &user_prompt,
            false,
        ) {
            let candidate = raw_reply.trim().to_string();
            if verify_craft_output(&candidate, packet).is_ok() {
                return candidate;
            }
        }
    }

    deterministic_craft_highlight_fallback(packet)
}

// ---------------------------------------------------------------------------
// 單元測試
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_deterministic_skill_martial_blade() {
        let packet = SkillDataPacket {
            entity_id: "test_entity".to_string(),
            role_title: "劍客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "太陰肺樞".to_string(),
            word_chars: vec!["斷".to_string(), "浪".to_string()],
            vessel_name: "分骨彎刀 [銳]".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["弓步前突".to_string(), "旋腕斜撩".to_string(), "回身橫削".to_string()],
            fallback_name: "斷浪".to_string(),
        };

        let prose = render_deterministic_skill_from_data(&packet);
        assert_eq!(prose.name, "斷浪刀式");
        assert!(prose.origin_basis.contains("分骨彎刀"));
        assert!(prose.origin_basis.contains("弓步前突"));
        assert!(verify_skill_output(&prose.name, &prose.origin_basis, &packet));
    }

    #[test]
    fn test_render_deterministic_skill_unarmed() {
        let packet = SkillDataPacket {
            entity_id: "test_entity".to_string(),
            role_title: "行者".to_string(),
            domain: "martial".to_string(),
            slot_label: "連攜".to_string(),
            hub_or_tier: "少陽膽樞".to_string(),
            word_chars: vec!["崩".to_string(), "拳".to_string()],
            vessel_name: "徒手".to_string(),
            carrier: CarrierAffinity::Unarmed,
            motion_sequence: vec!["踏步進身".to_string(), "側身擺肘".to_string(), "直拳破空".to_string()],
            fallback_name: "崩拳".to_string(),
        };

        let prose = render_deterministic_skill_from_data(&packet);
        assert_eq!(prose.name, "崩拳拳勢");
        assert!(prose.origin_basis.contains("徒手運勁"));
        assert!(!prose.origin_basis.contains("刀"));
        assert!(!prose.origin_basis.contains("劍"));
        assert!(verify_skill_output(&prose.name, &prose.origin_basis, &packet));
    }

    #[test]
    fn test_render_deterministic_skill_production() {
        let packet = SkillDataPacket {
            entity_id: "artisan_1".to_string(),
            role_title: "工匠".to_string(),
            domain: "production".to_string(),
            slot_label: "百工".to_string(),
            hub_or_tier: "熟稔".to_string(),
            word_chars: vec!["巧".to_string(), "工".to_string()],
            vessel_name: "雕花刻刀 [良]".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["審度紋理".to_string(), "斜刃修邊".to_string(), "細磨拋光".to_string()],
            fallback_name: "細琢巧工".to_string(),
        };

        let prose = render_deterministic_skill_from_data(&packet);
        assert_eq!(prose.name, "細琢巧工");
        assert!(prose.origin_basis.contains("雕花刻刀"));
        assert!(verify_skill_output(&prose.name, &prose.origin_basis, &packet));
    }

    #[test]
    fn test_verify_skill_output_reject_forbidden_words() {
        let packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["斬".to_string()],
            vessel_name: "鋼刀".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["提刃起步".to_string(), "橫揮斷水".to_string(), "收鋒定步".to_string()],
            fallback_name: "斬風".to_string(),
        };

        // 禁忌詞：系統
        assert!(!verify_skill_output("系統刀式", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        // 禁忌詞：剛才
        assert!(!verify_skill_output("斬風刀式", "剛才手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        // 禁忌詞：第
        assert!(!verify_skill_output("斬風刀式", "手執鋼刀，第一序位由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        // 禁忌詞：資料庫
        assert!(!verify_skill_output("斬風刀式", "資料庫記載手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
    }

    #[test]
    fn test_verify_skill_output_reject_ascii_digits() {
        let packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["斬".to_string()],
            vessel_name: "鋼刀".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["提刃起步".to_string(), "橫揮斷水".to_string(), "收鋒定步".to_string()],
            fallback_name: "斬風".to_string(),
        };

        assert!(!verify_skill_output("斬風刀1", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        assert!(!verify_skill_output("斬風刀式", "手執鋼刀，由1「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        // 全形數字
        assert!(!verify_skill_output("斬風刀１", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        assert!(!verify_skill_output("斬風刀式", "手執鋼刀，由２「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
    }

    #[test]
    fn test_verify_skill_output_reject_punctuation_in_name() {
        let packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["斬".to_string()],
            vessel_name: "鋼刀".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["提刃起步".to_string(), "橫揮斷水".to_string(), "收鋒定步".to_string()],
            fallback_name: "斬風".to_string(),
        };

        assert!(!verify_skill_output("[斬風刀]", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        assert!(!verify_skill_output("斬風·刀", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        assert!(!verify_skill_output("斬風刀！", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
    }

    #[test]
    fn test_verify_skill_output_reject_weapon_mismatch() {
        // 徒手不能包含兵刃名
        let unarmed_packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "拳手".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["拳".to_string()],
            vessel_name: "徒手".to_string(),
            carrier: CarrierAffinity::Unarmed,
            motion_sequence: vec!["推掌向前".to_string(), "抱元守一".to_string(), "收步凝神".to_string()],
            fallback_name: "推掌".to_string(),
        };
        assert!(!verify_skill_output("推掌劍式", "徒手運勁，由「推掌向前」起勢，順接「抱元守一」，再以「收步凝神」連貫成招。", &unarmed_packet));
        assert!(!verify_skill_output("推掌拳勢", "手握寶劍，由「推掌向前」起勢，順接「抱元守一」，再以「收步凝神」連貫成招。", &unarmed_packet));

        // 鈍器 Blunt 不可含 劍、槍、矛
        let blunt_packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "力士".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["破".to_string()],
            vessel_name: "玄鐵重錘".to_string(),
            carrier: CarrierAffinity::Blunt,
            motion_sequence: vec!["引錘蓄力".to_string(), "砸地碎石".to_string(), "回步收力".to_string()],
            fallback_name: "破岩".to_string(),
        };
        assert!(!verify_skill_output("破岩劍式", "手執玄鐵重錘，由「引錘蓄力」起勢，順接「砸地碎石」，再以「回步收力」一氣呵成。", &blunt_packet));
        assert!(!verify_skill_output("破岩重式", "如長槍直刺，由「引錘蓄力」起勢，順接「砸地碎石」，再以「回步收力」一氣呵成。", &blunt_packet));

        // 刀刃 Blade 不可含 錘、盾、槍、矛
        let blade_packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "刀客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["斬".to_string()],
            vessel_name: "青鋼長刀".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["拔刀出鞘".to_string(), "斜斬空門".to_string(), "納刀回鞘".to_string()],
            fallback_name: "破空".to_string(),
        };
        assert!(!verify_skill_output("破空錘法", "手執青鋼長刀，由「拔刀出鞘」起勢，順接「斜斬空門」，再以「納刀回鞘」一氣呵成。", &blade_packet));
        assert!(!verify_skill_output("破空刀式", "如堅盾立防，由「拔刀出鞘」起勢，順接「斜斬空門」，再以「納刀回鞘」一氣呵成。", &blade_packet));
    }

    #[test]
    fn test_verify_skill_output_reject_ungrounded_motion() {
        let packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "劍客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["刺".to_string()],
            vessel_name: "龍泉劍".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["點星尋隙".to_string(), "穿雲刺喉".to_string(), "飄身落定".to_string()],
            fallback_name: "追星".to_string(),
        };

        // origin_basis 完全未包含動作、兵器或字印
        let ungrounded_basis = "天地玄黃宇宙洪荒日月盈昃辰宿列張寒來暑往秋收冬藏。";
        assert!(!verify_skill_output("追星劍式", ungrounded_basis, &packet));
    }

    #[test]
    fn test_render_deterministic_bout_and_verify() {
        let packet = CombatBoutFactPacket {
            slot: TacticalSlot::Opening,
            attacker_name: "李俠".to_string(),
            defender_name: "王客".to_string(),
            attacker_weapon: "分骨彎刀 [銳]".to_string(),
            defender_weapon: "徒手".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Unarmed,
            winner_side: "attacker".to_string(),
            winner_skill_name: "斷浪刀式".to_string(),
            loser_skill_name: "破甲拳".to_string(),
            winner_motions: vec!["弓步前突".to_string(), "旋腕斜撩".to_string()],
            loser_attempted_motions: vec!["沉身立肘".to_string()],
            counter_reason: Some("搶佔先機".to_string()),
            hit_part: TargetPart::Arm,
            severity_label: "氣血翻湧".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        let prose = render_deterministic_bout_from_data(&packet);
        assert!(prose.contains("斷浪刀式"));
        assert!(!prose.contains("破甲拳")); // 敗方招式名稱絕不可出現
        assert!(prose.contains("碎招"));
        assert!(prose.contains("雙臂關節"));
        assert!(prose.contains("氣血翻湧"));
        assert!(verify_combat_bout_output(&prose, &packet));
    }

    #[test]
    fn test_verify_combat_bout_reject_leaked_loser_skill() {
        let packet = CombatBoutFactPacket {
            slot: TacticalSlot::Finisher,
            attacker_name: "李俠".to_string(),
            defender_name: "王客".to_string(),
            attacker_weapon: "分骨彎刀".to_string(),
            defender_weapon: "精鋼鐵棍".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Pole,
            winner_side: "attacker".to_string(),
            winner_skill_name: "斷空絕斬".to_string(),
            loser_skill_name: "千鈞覆地".to_string(),
            winner_motions: vec!["欺身進步".to_string(), "反手揮刃".to_string()],
            loser_attempted_motions: vec!["提棍蓄力".to_string()],
            counter_reason: Some("截其破綻".to_string()),
            hit_part: TargetPart::Torso,
            severity_label: "重創失據".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        // 包含敗方招式名稱 "千鈞覆地"，必須被拒絕！
        let invalid_prose = "李俠手執分骨彎刀施展「斷空絕斬」，由欺身進步順勢貫通；王客以「千鈞覆地」應對遭化為碎招，軀幹心脈受創，重創失據。";
        assert!(!verify_combat_bout_output(invalid_prose, &packet));

        // 僅包含敗方動作 "提棍蓄力" 且無敗方招式名，驗證通過
        let valid_prose = "李俠手執分骨彎刀施展「斷空絕斬」，由欺身進步順勢貫通；王客嘗試提棍蓄力遭化為碎招，軀幹心脈受創，陷入重創失據。";
        assert!(verify_combat_bout_output(valid_prose, &packet));
    }

    #[test]
    fn test_verify_combat_bout_reject_forbidden_words_and_digits() {
        let packet = CombatBoutFactPacket {
            slot: TacticalSlot::Opening,
            attacker_name: "李俠".to_string(),
            defender_name: "王客".to_string(),
            attacker_weapon: "分骨彎刀".to_string(),
            defender_weapon: "徒手".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Unarmed,
            winner_side: "attacker".to_string(),
            winner_skill_name: "斷空絕斬".to_string(),
            loser_skill_name: "碎星拳".to_string(),
            winner_motions: vec!["欺身進步".to_string()],
            loser_attempted_motions: vec!["起手格擋".to_string()],
            counter_reason: None,
            hit_part: TargetPart::Torso,
            severity_label: "氣血翻湧".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        // 出現數字
        let digit_prose = "李俠以分骨彎刀施展「斷空絕斬」，欺身進步命中，扣除30點傷害，王客陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(digit_prose, &packet));

        // 出現全形數字
        let fullwidth_digit_prose = "李俠以分骨彎刀施展「斷空絕斬」，欺身進步命中，扣除１２點傷害，王客陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(fullwidth_digit_prose, &packet));

        // 出現「第」、「回合」
        let round_prose = "第一回合交手，李俠以分骨彎刀施展「斷空絕斬」，欺身進步破開起手格擋，王客受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(round_prose, &packet));

        // 出現「剛才」
        let instant_prose = "剛才李俠以分骨彎刀施展「斷空絕斬」，欺身進步破開起手格擋，王客受創散為碎招，陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(instant_prose, &packet));
    }

    #[test]
    fn test_verify_combat_bout_reject_weapon_mismatch() {
        // 雙方皆為徒手，不可出現 劍、刀鋒、刀光、長槍、槍尖
        let packet = CombatBoutFactPacket {
            slot: TacticalSlot::Midgame,
            attacker_name: "拳師甲".to_string(),
            defender_name: "拳師乙".to_string(),
            attacker_weapon: "徒手".to_string(),
            defender_weapon: "徒手".to_string(),
            attacker_carrier: CarrierAffinity::Unarmed,
            defender_carrier: CarrierAffinity::Unarmed,
            winner_side: "attacker".to_string(),
            winner_skill_name: "大伏魔拳".to_string(),
            loser_skill_name: "八卦掌".to_string(),
            winner_motions: vec!["進步衝拳".to_string()],
            loser_attempted_motions: vec!["側閃游走".to_string()],
            counter_reason: None,
            hit_part: TargetPart::Torso,
            severity_label: "氣血翻湧".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        let sword_leak = "拳師甲以雙拳貫勁施展「大伏魔拳」，劍氣森森由進步衝拳貫通，側閃游走化為碎招，軀幹心脈受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(sword_leak, &packet));

        let spear_leak = "拳師甲以雙拳貫勁施展「大伏魔拳」，身如長槍由進步衝拳貫通，側閃游走化為碎招，軀幹心脈受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(spear_leak, &packet));
    }

    #[test]
    fn test_render_deterministic_bout_defender_win_and_draw() {
        let def_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Defense,
            attacker_name: "刀客".to_string(),
            defender_name: "槍客".to_string(),
            attacker_weapon: "短刀".to_string(),
            defender_weapon: "精鐵長槍".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Pole,
            winner_side: "defender".to_string(),
            winner_skill_name: "回馬突刺".to_string(),
            loser_skill_name: "撲面橫斬".to_string(),
            winner_motions: vec!["撤步引鋒".to_string(), "旋桿探心".to_string()],
            loser_attempted_motions: vec!["提刃猛撲".to_string()],
            counter_reason: Some("以長制短".to_string()),
            hit_part: TargetPart::Head,
            severity_label: "重創失據".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        let def_prose = render_deterministic_bout_from_data(&def_packet);
        assert!(def_prose.contains("回馬突刺"));
        assert!(!def_prose.contains("撲面橫斬"));
        assert!(def_prose.contains("頭顱要害"));
        assert!(verify_combat_bout_output(&def_prose, &def_packet));

        let draw_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Midgame,
            attacker_name: "甲".to_string(),
            defender_name: "乙".to_string(),
            attacker_weapon: "青鋼長刀".to_string(),
            defender_weapon: "精鋼短劍".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Blade,
            winner_side: "draw".to_string(),
            winner_skill_name: "平分秋色".to_string(),
            loser_skill_name: String::new(),
            winner_motions: vec!["起步截擊".to_string()],
            loser_attempted_motions: vec!["橫刃格拒".to_string()],
            counter_reason: None,
            hit_part: TargetPart::Torso,
            severity_label: "輕微擦震".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        let draw_prose = render_deterministic_bout_from_data(&draw_packet);
        assert!(verify_combat_bout_output(&draw_prose, &draw_packet));
    }

    #[test]
    fn test_synthesize_skill_and_compose_bout_fallbacks() {
        let skill_packet = SkillDataPacket {
            entity_id: "hero".to_string(),
            role_title: "少俠".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "少陽膽樞".to_string(),
            word_chars: vec!["追".to_string(), "風".to_string()],
            vessel_name: "青鋼長刀 [銳]".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["拔刀出鞘".to_string(), "斜斬空門".to_string(), "納刀回鞘".to_string()],
            fallback_name: "追風".to_string(),
        };

        let synthesized = synthesize_skill_from_data(&skill_packet);
        assert!(verify_skill_output(&synthesized.name, &synthesized.origin_basis, &skill_packet));

        let bout_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Opening,
            attacker_name: "少俠".to_string(),
            defender_name: "強梁".to_string(),
            attacker_weapon: "青鋼長刀".to_string(),
            defender_weapon: "粗木棒".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Blunt,
            winner_side: "attacker".to_string(),
            winner_skill_name: synthesized.name,
            loser_skill_name: "亂披風棒".to_string(),
            winner_motions: vec!["拔刀出鞘".to_string(), "斜斬空門".to_string()],
            loser_attempted_motions: vec!["掄棒下砸".to_string()],
            counter_reason: Some("刀勢先至".to_string()),
            hit_part: TargetPart::Arm,
            severity_label: "氣血翻湧".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        let bout_prose = compose_combat_bout_from_data(&bout_packet);
        assert!(verify_combat_bout_output(&bout_prose, &bout_packet));
    }

    #[test]
    fn test_verify_combat_bout_residue_and_concurrent() {
        // 1. active_residue 為 None 時，禁止捏造殘留特徵詞彙
        let base_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Opening,
            attacker_name: "李俠".to_string(),
            defender_name: "王客".to_string(),
            attacker_weapon: "分骨彎刀".to_string(),
            defender_weapon: "徒手".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Unarmed,
            winner_side: "attacker".to_string(),
            winner_skill_name: "斷空絕斬".to_string(),
            loser_skill_name: "碎星拳".to_string(),
            winner_motions: vec!["欺身進步".to_string()],
            loser_attempted_motions: vec!["起手格擋".to_string()],
            counter_reason: None,
            hit_part: TargetPart::Torso,
            severity_label: "氣血翻湧".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };

        let valid_clean = "李俠手執分骨彎刀施展「斷空絕斬」，由欺身進步順勢貫通；王客雖試圖以起手格擋應招卻散為碎招，軀幹心脈受創，陷入氣血翻湧。";
        assert!(verify_combat_bout_output(valid_clean, &base_packet));

        let fabricated_heat = "李俠手執分骨彎刀施展「斷空絕斬」，場中燥氣四溢，由欺身進步順勢貫通，王客受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(fabricated_heat, &base_packet));

        let fabricated_frost = "李俠手執分骨彎刀施展「斷空絕斬」，霜氣凝結，由欺身進步順勢貫通，王客受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(fabricated_frost, &base_packet));

        let fabricated_tremor = "李俠手執分骨彎刀施展「斷空絕斬」，暗震湧動，由欺身進步順勢貫通，王客受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(fabricated_tremor, &base_packet));

        let fabricated_vortex = "李俠手執分骨彎刀施展「斷空絕斬」，旋流不散，由欺身進步順勢貫通，王客受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(fabricated_vortex, &base_packet));

        // 2. active_residue 為 Some("熾烈燥氣") 時，要求重合關鍵字（例如 燥/熾/烈）
        let heat_packet = CombatBoutFactPacket {
            active_residue: Some("熾烈燥氣".to_string()),
            residue_interaction: Some("借勢共鳴".to_string()),
            loser_posture_after: Some("架勢潰散".to_string()),
            inflicted_injury: Some("雙肋挫傷".to_string()),
            ..base_packet.clone()
        };

        // 未提及殘留氣機字元之段落被拒絕
        assert!(!verify_combat_bout_output(valid_clean, &heat_packet));

        // 包含 "燥" 之段落驗證通過
        let heat_prose = render_deterministic_bout_from_data(&heat_packet);
        assert!(heat_prose.contains("熾烈燥氣"));
        assert!(heat_prose.contains("借勢共鳴"));
        assert!(heat_prose.contains("雙肋挫傷"));
        assert!(heat_prose.contains("架勢潰散"));
        assert!(verify_combat_bout_output(&heat_prose, &heat_packet));

        // 3. 測試並發組裝 compose_combat_bouts_concurrent
        let mid_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Midgame,
            active_residue: Some("凝寒霜氣".to_string()),
            residue_interaction: Some("氣脈受激".to_string()),
            ..base_packet.clone()
        };
        let fin_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Finisher,
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: Some("步履微晃".to_string()),
            ..base_packet
        };

        let packets = vec![heat_packet, mid_packet, fin_packet];
        let results = compose_combat_bouts_concurrent(&packets);
        assert_eq!(results.len(), 3);
        assert!(verify_combat_bout_output(&results[0], &packets[0]));
        assert!(verify_combat_bout_output(&results[1], &packets[1]));
        assert!(verify_combat_bout_output(&results[2], &packets[2]));
        assert!(results[0].contains("熾烈燥氣"));
        assert!(results[1].contains("凝寒霜氣"));
        assert!(results[2].contains("步履微晃"));
    }

    #[test]
    fn test_craft_output_verification_and_deterministic_fallback() {
        let packet = CraftActionFactPacket {
            actor_name: "鐵匠阿明".to_string(),
            tool_name: "淬火鐵錘 [銳]".to_string(),
            tool_familiarity: "得心應手".to_string(),
            breath_habit: "爐火沉重".to_string(),
            material_names: vec!["青剛赤鐵".to_string(), "松木炭".to_string()],
            output_name: "百煉青鋼匕首".to_string(),
            highlight_kind: "火候純熟".to_string(),
            environment_hint: "鐵匠鋪火爐旁".to_string(),
        };

        // 1. 確定性兜底必然通過驗證
        let fallback = deterministic_craft_highlight_fallback(&packet);
        assert!(
            verify_craft_output(&fallback, &packet).is_ok(),
            "確定性兜底必須通過驗證：{:?}",
            verify_craft_output(&fallback, &packet)
        );
        let char_count = fallback.chars().count();
        assert!((25..=260).contains(&char_count));
        assert!(!fallback.chars().any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c)));
        assert!(!fallback.contains('%') && !fallback.contains('+'));
        assert!(fallback.contains("百煉青鋼匕首") || fallback.contains("匕首"));
        assert!(fallback.contains("淬火鐵錘") || fallback.contains("鐵錘") || fallback.contains("青剛赤鐵"));

        // 2. 測試 compose_craft_highlight_from_data 離線回退
        let composed = compose_craft_highlight_from_data(&packet);
        assert!(verify_craft_output(&composed, &packet).is_ok());

        // 3. 測試驗證器邊界條件
        // 長度不足 25 字
        assert!(verify_craft_output("太短的淬火鐵錘匕首", &packet).is_err());

        // 包含阿拉伯數字
        let with_digit = format!("{} 帶有1把新器物", fallback);
        assert!(verify_craft_output(&with_digit, &packet).is_err());

        // 包含全形數字
        let with_full_digit = format!("{} 耗時３炷香", fallback);
        assert!(verify_craft_output(&with_full_digit, &packet).is_err());

        // 包含百分比或加號
        let with_percent = format!("{} 提升10%純度", fallback);
        assert!(verify_craft_output(&with_percent, &packet).is_err());
        let with_plus = format!("{} 威力+5", fallback);
        assert!(verify_craft_output(&with_plus, &packet).is_err());

        // 包含禁用詞彙（系統、玩家、剛才、等等）
        let with_forbidden1 = format!("{} 系統提示完成", fallback);
        assert!(verify_craft_output(&with_forbidden1, &packet).is_err());
        let with_forbidden2 = format!("{} 玩家凝神細看", fallback);
        assert!(verify_craft_output(&with_forbidden2, &packet).is_err());
        let with_forbidden3 = format!("{} 方才火勢熾烈", fallback);
        assert!(verify_craft_output(&with_forbidden3, &packet).is_err());
        let with_forbidden4 = format!("{} 念場震盪不休", fallback);
        assert!(verify_craft_output(&with_forbidden4, &packet).is_err());
        let with_forbidden5 = format!("{} 還有多餘雜質", fallback);
        assert!(verify_craft_output(&with_forbidden5, &packet).is_err());

        // 未提及產出物
        let no_output = "身處鐵匠鋪火爐旁，鐵匠阿明運轉爐火沉重之息，手持淬火鐵錘更顯得心應手，凝神調理青剛赤鐵，處處顯露火候純熟，神彩沉凝內斂。";
        assert!(verify_craft_output(no_output, &packet).is_err());

        // 未提及任何材料或工具
        let no_tool_or_mat = "身處鐵匠鋪火爐旁，鐵匠阿明運轉爐火沉重之息，得心應手，處處顯露火候純熟，終成百煉青鋼匕首，鋒芒神彩沉凝內斂。";
        assert!(verify_craft_output(no_tool_or_mat, &packet).is_err());
    }
}
