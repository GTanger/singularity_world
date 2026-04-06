// 對話模板載入、抽句與佔位符替換（對齊既有 `db/dialogue`）。
// 支援：關鍵字檢索、池子混用、片段組合、情境權重、佔位符詞表、微變體。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

use super::Personality;

const DEFAULT_TEMPLATES_BASE: &str = "data/templates";
const DIALOGUE_KEYWORDS_FILENAME: &str = "dialogue_keywords.json";

// ── 對話檔結構 ──

#[derive(Debug, Clone, Deserialize)]
pub struct DialogueFile {
    #[serde(default)]
    pub greet: DialogueSection,
    #[serde(default)]
    pub talk: DialogueSection,
    #[serde(default)]
    pub talk_fragments: Vec<Vec<Vec<String>>>,
    #[serde(default)]
    pub idle: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub trade_announce: TradeAnnounce,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DialogueSection {
    #[serde(default)]
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TradeAnnounce {
    #[serde(default)]
    pub buy: Vec<String>,
    #[serde(default)]
    pub sell: Vec<String>,
}

// ── 佔位符詞表 ──

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DialogueSlots {
    #[serde(default)]
    pub verb: Vec<String>,
    #[serde(default)]
    pub verb_manner: Vec<String>,
    #[serde(default)]
    pub verb_action: Vec<String>,
    #[serde(default)]
    pub mood: Vec<String>,
    #[serde(default)]
    pub time: Vec<String>,
    #[serde(default)]
    pub thing: Vec<String>,
    #[serde(default)]
    pub weather: Vec<String>,
    #[serde(default)]
    pub topic: Vec<String>,
}

// ── 關鍵字對應表 ──

#[derive(Debug, Clone, Deserialize)]
struct KeywordGroup {
    #[serde(default)]
    terms: Vec<String>,
    #[serde(default)]
    responses: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct KeywordFile {
    #[serde(default)]
    groups: Vec<KeywordGroup>,
}

// ── 全域快取 ──

static DIALOGUE_CACHE: RwLock<Option<HashMap<String, DialogueFile>>> = RwLock::new(None);
static SLOTS_CACHE: OnceLock<DialogueSlots> = OnceLock::new();
static KEYWORD_GROUPS: RwLock<Vec<KeywordGroup>> = RwLock::new(Vec::new());

// ── 簡易偽隨機（seed 可重現，對齊既有 math/rand） ──

struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: i64) -> Self {
        Self(seed as u64)
    }
    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn intn(&mut self, n: usize) -> usize {
        if n == 0 { return 0; }
        (self.next_u64() % n as u64) as usize
    }
    fn float32(&mut self) -> f32 {
        (self.next_u64() & 0xFFFFFF) as f32 / 0x100_0000 as f32
    }
    fn float64(&mut self) -> f64 {
        (self.next_u64() & 0xFFFF_FFFF_FFFF) as f64 / 0x1_0000_0000_0000_u64 as f64
    }
}

fn pick(list: &[String], rng: &mut SimpleRng) -> String {
    if list.is_empty() {
        return String::new();
    }
    list[rng.intn(list.len())].clone()
}

// ── 載入函式 ──

/// 載入關鍵字對應表（僅執行一次，懶加載）。
pub fn load_dialogue_keywords(base_path: &str) {
    {
        let guard = KEYWORD_GROUPS.read().unwrap();
        if !guard.is_empty() {
            return;
        }
    }
    let path = PathBuf::from(base_path).join(DIALOGUE_KEYWORDS_FILENAME);
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[dialogue] keywords {}: {e}", path.display());
            return;
        }
    };
    let f: KeywordFile = match serde_json::from_str(&raw) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("[dialogue] keywords parse: {e}");
            return;
        }
    };
    let mut guard = KEYWORD_GROUPS.write().unwrap();
    *guard = f.groups;
}

/// 若玩家輸入包含任一群組的任一關鍵詞，則從該組隨機回傳一句；否則回傳空字串。
pub fn try_match_keyword(player_input: &str, seed: i64) -> String {
    {
        let guard = KEYWORD_GROUPS.read().unwrap();
        if guard.is_empty() {
            drop(guard);
            load_dialogue_keywords(DEFAULT_TEMPLATES_BASE);
        }
    }
    let input = player_input.trim().to_lowercase();
    if input.is_empty() || input == "（搭話）" {
        return String::new();
    }
    let guard = KEYWORD_GROUPS.read().unwrap();
    let mut rng = SimpleRng::new(seed);
    for g in guard.iter() {
        for term in &g.terms {
            if input.contains(&term.to_lowercase()) && !g.responses.is_empty() {
                return g.responses[rng.intn(g.responses.len())].clone();
            }
        }
    }
    String::new()
}

/// 從公版對話 `public_dialogue.json` 的 `talk.lines` 抽一句，並將 `{name}` 替換為 npc_name。
pub fn pick_from_public_talk(npc_name: &str, seed: i64) -> String {
    let d = match load_dialogue(DEFAULT_TEMPLATES_BASE, "dialogues/public_dialogue.json") {
        Some(d) => d,
        None => return String::new(),
    };
    if d.talk.lines.is_empty() {
        return String::new();
    }
    let mut rng = SimpleRng::new(seed);
    let line = &d.talk.lines[rng.intn(d.talk.lines.len())];
    line.replace("{name}", npc_name)
}

/// 載入 `dialogue_slots.json`，僅執行一次。
pub fn load_dialogue_slots(path: &str) -> &'static DialogueSlots {
    SLOTS_CACHE.get_or_init(|| {
        let raw = match fs::read_to_string(path).or_else(|_| fs::read_to_string(Path::new("..").join(path))) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("[dialogue] slots {path}: {e}");
                return DialogueSlots {
                    verb: vec!["嘆了口氣".into(), "點點頭".into()],
                    mood: vec!["隨口".into(), "正色".into()],
                    time: vec!["清晨".into(), "正午".into(), "傍晚".into(), "夜裡".into()],
                    thing: vec!["這兒的規矩".into()],
                    weather: vec!["晴".into()],
                    topic: vec!["這兒的規矩".into()],
                    ..Default::default()
                };
            }
        };
        match serde_json::from_str(&raw) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("[dialogue] slots parse: {e}");
                DialogueSlots {
                    verb: vec!["嘆了口氣".into()],
                    mood: vec!["隨口".into()],
                    time: vec!["此時".into()],
                    thing: vec![String::new()],
                    ..Default::default()
                }
            }
        }
    })
}

/// 依 `base_path + dialogue_file` 載入對話檔並快取。
pub fn load_dialogue(base_path: &str, dialogue_file: &str) -> Option<DialogueFile> {
    let key = PathBuf::from(base_path).join(dialogue_file).to_string_lossy().to_string();
    {
        let guard = DIALOGUE_CACHE.read().unwrap();
        if let Some(ref cache) = *guard
            && let Some(d) = cache.get(&key)
        {
            return Some(d.clone());
        }
    }
    let raw = fs::read_to_string(&key)
        .or_else(|_| fs::read_to_string(Path::new("..").join(&key)))
        .ok()?;
    let d: DialogueFile = serde_json::from_str(&raw).ok()?;
    let mut guard = DIALOGUE_CACHE.write().unwrap();
    let cache = guard.get_or_insert_with(HashMap::new);
    cache.insert(key, d.clone());
    Some(d)
}

// ── 佔位符替換 ──

/// 佔位符替換用情境。
pub struct PlaceholderCtx<'a> {
    pub name: &'a str,
    pub room_name: &'a str,
    pub time_label: &'a str,
    pub mood: &'a str,
    pub verb: &'a str,
    pub thing: &'a str,
    pub goods: &'a str,
    pub weather: &'a str,
    pub topic: &'a str,
}

/// 將模板中的佔位符替換；空欄位從 slots 抽。子槽位：`{verb}` 有機率為 `verb_manner+verb_action`。
pub fn fill_placeholders(s: &str, ctx: &PlaceholderCtx, slots: &DialogueSlots, seed: i64) -> String {
    let mut result = s.to_string();
    let mut rng = SimpleRng::new(seed);

    let do_replace = |result: &mut String, key: &str, val: &str, slots: &DialogueSlots, rng: &mut SimpleRng| {
        let actual = if val.is_empty() {
            match key {
                "mood" => pick(&slots.mood, rng),
                "verb" => {
                    if !slots.verb_manner.is_empty() && !slots.verb_action.is_empty() && rng.float32() < 0.4 {
                        pick(&slots.verb_manner, rng) + &pick(&slots.verb_action, rng)
                    } else {
                        pick(&slots.verb, rng)
                    }
                }
                "time" => pick(&slots.time, rng),
                "thing" => pick(&slots.thing, rng),
                "weather" => pick(&slots.weather, rng),
                "topic" => pick(&slots.topic, rng),
                _ => String::new(),
            }
        } else {
            val.to_string()
        };
        let placeholder = format!("{{{key}}}");
        *result = result.replace(&placeholder, &actual);
    };

    do_replace(&mut result, "name", ctx.name, slots, &mut rng);
    do_replace(&mut result, "room", ctx.room_name, slots, &mut rng);
    do_replace(&mut result, "time", ctx.time_label, slots, &mut rng);
    do_replace(&mut result, "mood", ctx.mood, slots, &mut rng);
    do_replace(&mut result, "verb", ctx.verb, slots, &mut rng);
    do_replace(&mut result, "thing", ctx.thing, slots, &mut rng);
    do_replace(&mut result, "goods", ctx.goods, slots, &mut rng);
    do_replace(&mut result, "weather", ctx.weather, slots, &mut rng);
    do_replace(&mut result, "topic", ctx.topic, slots, &mut rng);
    result
}

/// 填完佔位符後做輕量口語變體（依 seed 可重現）。
pub fn apply_micro_variants(s: &str, seed: i64) -> String {
    let mut result = s.to_string();
    let mut rng = SimpleRng::new(seed);
    if rng.float32() < 0.15
        && let Some(idx) = result.rfind('。')
    {
        result = format!("{}啦。{}", &result[..idx], &result[idx + '。'.len_utf8()..]);
    }
    if rng.float32() < 0.2 {
        if rng.float32() < 0.5 {
            result = result.replacen("這兒", "那兒", 1);
        } else {
            result = result.replacen("那兒", "這兒", 1);
        }
    }
    result
}

// ── 情境權重抽句 ──

fn time_label_to_idle_key(time_label: &str) -> &str {
    match time_label {
        "清晨" => "morning",
        "正午" => "noon",
        "傍晚" => "evening",
        "夜裡" => "night",
        _ => "noon",
    }
}

/// 從 lines 中依 seed 與 personality 選一句；依 time_label、room_name 對含關鍵字的句加權。
pub fn pick_line_weighted(
    lines: &[String],
    seed: i64,
    personality: Option<&Personality>,
    time_label: &str,
    room_name: &str,
) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut rng = SimpleRng::new(seed);
    let mut weights: Vec<f64> = vec![1.0; lines.len()];
    for (i, line) in lines.iter().enumerate() {
        if !time_label.is_empty()
            && (line.contains(time_label)
                || (time_label == "夜裡" && line.contains("夜"))
                || (time_label == "清晨" && line.contains("晨")))
        {
            weights[i] *= 1.6;
        }
        if room_name.len() >= 2 && line.contains(room_name) {
            weights[i] *= 1.5;
        }
    }
    let sum: f64 = weights.iter().sum();
    let sum = if sum <= 0.0 { 1.0 } else { sum };
    let mut x = rng.float64() * sum;
    for (i, &w) in weights.iter().enumerate() {
        x -= w;
        if x <= 0.0 {
            return apply_personality_shift(lines, i, personality);
        }
    }
    let idx = rng.intn(lines.len());
    apply_personality_shift(lines, idx, personality)
}

fn apply_personality_shift(lines: &[String], mut idx: usize, personality: Option<&Personality>) -> String {
    if let Some(p) = personality {
        let n = lines.len() as i32;
        let mut shift = (p.boldness * (n / 2) as f64) as i32;
        if p.sensitivity > 0.6 {
            shift += n / 4;
        } else if p.sensitivity < 0.3 {
            shift -= n / 4;
        }
        idx = ((idx as i32 + shift + n) % n) as usize;
    }
    lines[idx].clone()
}

fn assemble_fragment(segments: &[Vec<String>], rng: &mut SimpleRng) -> String {
    let mut buf = String::new();
    for seg in segments {
        if !seg.is_empty() {
            buf.push_str(&seg[rng.intn(seg.len())]);
        }
    }
    buf
}

// ── 池子混用常數 ──

const PROB_GREET: f64 = 0.12;
const PROB_IDLE: f64 = 0.08;
const PROB_FRAGMENT: f32 = 0.25;

// ── 主要抽句函式 ──

/// 依職業取得對話檔、抽一句、填佔位符、微變體後回傳（對齊既有 `PickFromDialogue`）。
#[allow(clippy::too_many_arguments)]
pub fn pick_from_dialogue(
    entity_id: &str,
    room_id: &str,
    occupation_id: &str,
    key: &str,
    personality: Option<&Personality>,
    goods: &str,
    room_name: &str,
    time_label: &str,
    weather: &str,
) -> String {
    let occs = super::occupation::load_occupation_cache();
    let occ = match occs.get(occupation_id) {
        Some(o) if !o.dialogue_file.is_empty() => o,
        _ => return String::new(),
    };
    let d = match load_dialogue(DEFAULT_TEMPLATES_BASE, &occ.dialogue_file) {
        Some(d) => d,
        None => return String::new(),
    };
    let slots_path = PathBuf::from(DEFAULT_TEMPLATES_BASE).join("dialogue_slots.json");
    let slots = load_dialogue_slots(slots_path.to_str().unwrap_or("data/templates/dialogue_slots.json"));

    let mut seed: i64 = 0;
    for r in entity_id.chars() {
        seed = seed.wrapping_mul(31).wrapping_add(r as i64);
    }
    seed = seed.wrapping_add((room_id.len() as i64) << 8).wrapping_add((key.len() as i64) << 16);

    let mut line = String::new();
    let mut pool_key = key;

    // 池子混用：主 key 為 talk 時，有機率改從 greet 或 idle 抽
    if key == "talk" {
        let mut rng = SimpleRng::new(seed.wrapping_add(99));
        let p = rng.float64();
        if p < PROB_GREET && !d.greet.lines.is_empty() {
            pool_key = "greet";
        } else if p < PROB_GREET + PROB_IDLE
            && let Some(idles) = d.idle.get(time_label_to_idle_key(time_label))
            && !idles.is_empty()
        {
            line = pick_line_weighted(idles, seed.wrapping_add(101), personality, time_label, room_name);
            pool_key = "";
        }
    }

    if line.is_empty() {
        if pool_key == "greet" {
            line = pick_line_weighted(&d.greet.lines, seed.wrapping_add(1), personality, time_label, room_name);
        } else if pool_key == "talk" {
            // 片段組合
            if !d.talk_fragments.is_empty() && SimpleRng::new(seed.wrapping_add(77)).float32() < PROB_FRAGMENT {
                let mut rng = SimpleRng::new(seed.wrapping_add(17));
                let segments = &d.talk_fragments[rng.intn(d.talk_fragments.len())];
                line = assemble_fragment(segments, &mut rng);
            }
            if line.is_empty() && !d.talk.lines.is_empty() {
                line = pick_line_weighted(&d.talk.lines, seed, personality, time_label, room_name);
            }
        }
    }

    if line.is_empty() {
        return String::new();
    }

    let ctx = PlaceholderCtx {
        name: entity_id,
        room_name,
        time_label,
        mood: "",
        verb: "",
        thing: "",
        goods,
        weather,
        topic: "",
    };
    let filled = fill_placeholders(&line, &ctx, slots, seed.wrapping_add(1));
    apply_micro_variants(&filled, seed.wrapping_add(31))
}
