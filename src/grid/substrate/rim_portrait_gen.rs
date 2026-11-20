//! 人物靜態立繪雙軌生成引擎（Track A: OpenRouter BYOK `google/gemini-3.1-flash-image` ＋ Track B: 本機 `FLUX.2-klein-4B`）。
//!
//! 依角色先天骨相（`soul_seed`、性別、年齡神態）與即時穿戴／手持器物結構化文字敘述，
//! 搭配「宣紙白描・簡約礦物淡彩」風格錨點生成單張靜態立繪並依狀態雜湊落盤快取；
//! 半身胸像與全身立像共用同一張快取圖，絕不切割紙娃娃圖層。

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use reqwest::blocking::multipart;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::ai::openai_chat::{DEFAULT_OPENROUTER_IMAGE_MODEL, get_player_openrouter_key};
use crate::game::now_unix;

const STYLE_REF_PATH: &str = "data/config/portrait_style_ref.png";
const CACHE_DIR: &str = "data/runtime/portraits";
const DEFAULT_LOCAL_SD_URL: &str = "http://127.0.0.1:11438";
const LOCAL_SD_MODEL_LABEL: &str = "FLUX.2-klein-4B";

const STYLE_ZH: &str = "宣紙白描・簡約礦物淡彩（均勻柔細焦墨輪廓線 #332620、低飽和礦物平塗淡彩、純淨米黃宣紙底 #f4ecdc，約 7.4 頭身寫實立像，無排線雜影與無貼紙白邊）";

static INFLIGHT_KEYS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static GEN_SEMAPHORE: LazyLock<tokio::sync::Semaphore> =
    LazyLock::new(|| tokio::sync::Semaphore::new(1));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortraitRequest {
    pub id: usize,
    pub name: String,
    pub gender: String,
    pub soul_seed: i64,
    pub role_tag: String,
    pub wear_hat: bool,
    pub show_pack: bool,
    pub tool_label: Option<String>,
    pub injured: bool,
    pub owner_player_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortraitTexts {
    pub actor_slug: String,
    pub state_hash: String,
    pub deterministic_seed: u32,
    pub appearance_zh: String,
    pub gear_zh: String,
    pub style_zh: String,
    pub gen_prompt: String,
    pub edit_prompt: String,
}

#[derive(Debug, Clone)]
pub enum PortraitLookup {
    Ready {
        path: PathBuf,
        engine: String,
        texts: PortraitTexts,
    },
    Updating {
        base_path: PathBuf,
        engine: String,
        texts: PortraitTexts,
    },
    Pending {
        texts: PortraitTexts,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CachedMeta {
    engine: String,
    updated_unix: i64,
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn seed_u64(soul_seed: i64, name: &str) -> u64 {
    if soul_seed != 0 {
        return splitmix64(soul_seed as u64);
    }
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let dig = hasher.finalize();
    let mut b = [0u8; 8];
    b.copy_from_slice(&dig[..8]);
    u64::from_le_bytes(b)
}

fn hex_sha256_16(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let dig = hasher.finalize();
    dig[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// 依先天骨相種子與即時裝備狀態組裝台灣正體文字敘述與雙語繪圖提示詞。
pub fn synthesize_portrait_texts(req: &PortraitRequest) -> PortraitTexts {
    let is_female = req.gender == "F" || req.gender == "女";
    let base_hash = seed_u64(req.soul_seed, &req.name);
    let age_idx = ((base_hash >> 2) % 4) as usize;
    let face_idx = ((base_hash >> 6) % 4) as usize;
    let skin_idx = ((base_hash >> 10) % 4) as usize;
    let vest_idx = ((base_hash >> 14) % 5) as usize;
    let trace_idx = ((base_hash >> 18) % 4) as usize;

    let (age_zh, age_en) = if is_female {
        match age_idx {
            0 => ("二十出頭的俐落女山民", "a woman in her early 20s"),
            1 => ("二十後半的沉穩女子", "a woman in her late 20s"),
            2 => ("三十餘歲精幹女行者", "a capable woman in her mid-30s"),
            _ => (
                "雙鬢微霜的中年女匠師",
                "a seasoned woman in her late 40s with subtle silver strands at temples",
            ),
        }
    } else {
        match age_idx {
            0 => ("二十出頭精瘦青年山民", "a lean young man in his early 20s"),
            1 => ("三十上下精實壯年男子", "a sturdy man in his early 30s"),
            2 => ("三十後半風霜山林行者", "a weathered man in his late 30s"),
            _ => (
                "微蓄短鬚的穩健中年漢子",
                "a composed seasoned man in his late 40s with light short stubble",
            ),
        }
    };

    let (face_zh, face_en) = if is_female {
        match face_idx {
            0 => (
                "清秀瓜子臉、細挑遠山眉、神情警醒",
                "slender oval face, crisp arched eyebrows, alert dark eyes",
            ),
            1 => (
                "端正鵝蛋臉、眉目清朗平靜",
                "balanced oval face, clear calm eyes and soft resolute mouth",
            ),
            2 => (
                "柔和方頰、眼神專注果決",
                "gentle angular cheekbones, focused steady gaze",
            ),
            _ => (
                "輪廓分明、眉尾微揚帶英氣",
                "sculpted jawline, resolute brows with quiet composure",
            ),
        }
    } else {
        match face_idx {
            0 => (
                "方正下顎、劍眉星目神態篤實",
                "square jaw, straight thick brows and calm resolute eyes",
            ),
            1 => (
                "高顴微削、眼神沉斂內穩",
                "defined cheekbones, steady thoughtful dark eyes",
            ),
            2 => (
                "闊額直鼻、敦厚端正面相",
                "broad forehead, straight nose, honest composed expression",
            ),
            _ => (
                "稜角俐落、眉心微凝顯幹練",
                "lean angular jaw, focused attentive expression",
            ),
        }
    };

    let (skin_zh, skin_en) = match skin_idx {
        0 => ("淺暖象牙膚色", "light warm ivory skin"),
        1 => ("日曬微暖麥色", "lightly sun-tanned wheat skin"),
        2 => ("柔和健康蜜黃膚", "warm natural honey-beige skin"),
        _ => ("山風清透淡暖膚", "pale warm cream skin"),
    };

    let hair_zh = if req.wear_hat {
        "鬢髮整齊束入笠簷內，雙耳與眉眼完整外露"
    } else if is_female {
        "墨黑長髮整齊盤作高髻別素木笄，露肩頸額前"
    } else {
        "烏黑長髮於頂心束成俐落丸髻，額前乾淨"
    };
    let hair_en = if req.wear_hat {
        "hair neatly tucked flat under a bamboo conical hat with bare ears and brows clearly exposed"
    } else if is_female {
        "neat dark hair tied up in a traditional topknot bun pinned with a plain wooden hairpin"
    } else {
        "neat black hair tied tightly in a clean topknot bun on crown"
    };

    let (vest_zh, vest_en) = match vest_idx {
        0 => ("茶褐帆布", "earthen ochre-brown canvas"),
        1 => ("青黛藍灰布", "slate-indigo dyed hemp"),
        2 => ("竹青艾褐布", "muted bamboo-sage olive hemp"),
        3 => ("赭石棕染布", "terracotta-rust mineral cloth"),
        _ => ("亞麻深灰布", "charcoal-flax rough cloth"),
    };

    let (trace_zh, trace_en) = if req.injured {
        (
            "右顴骨帶著一道新添細長傷痕",
            "a faint small red scratch on the right cheek",
        )
    } else {
        match trace_idx {
            0 => ("掌緣厚繭分明", "calloused steady working hands"),
            1 => ("眼角帶著淡淺風日細紋", "faint sun lines beside outer eyes"),
            2 => ("神氣內斂乾淨", "clean calm bearing"),
            _ => (
                "虎口泛著淺淡舊白痕",
                "faint weathered pale trace at thumb web",
            ),
        }
    };

    let appearance_zh = format!("{age_zh}，{face_zh}，{skin_zh}，{hair_zh}，{trace_zh}。");

    let hat_zh = if req.wear_hat {
        "圓錐細編原色竹笠（頂端光潔無墜飾）"
    } else {
        "無帽笠"
    };
    let hat_en = if req.wear_hat {
        "wearing a woven bamboo conical straw hat with clean smooth pointed apex resting neatly above eyebrows"
    } else {
        "bareheaded with no hat"
    };

    let pack_zh = if req.show_pack {
        "背後肩背原木負重木架與綑紮行囊"
    } else {
        "背後輕裝無背架"
    };
    let pack_en = if req.show_pack {
        "with a light traditional wooden backpack frame strapped behind back"
    } else {
        "with no backpack on shoulders"
    };

    let tool_raw = req.tool_label.as_deref().unwrap_or("").trim();
    let (tool_zh, tool_en) = if tool_raw.is_empty() || tool_raw == "空手" {
        (
            "雙手空手自然垂放身側".to_string(),
            "both empty hands resting naturally relaxed at sides holding nothing".to_string(),
        )
    } else if tool_raw.contains("導能") || tool_raw.contains("獵刀") {
        (
            format!(
                "舉起左手穩握直立向上的冷鋼「{tool_raw}」（桃木短柄與銅格護手，帶微青導能刃紋）"
            ),
            format!(
                "raised left hand cleanly gripping an upright forged cold-steel hunting blade ({tool_raw}) with hardwood grip, bronze guard, and subtle celadon rune fuller pointing straight up"
            ),
        )
    } else if tool_raw.contains('刀') || tool_raw.contains('劍') {
        (
            format!("舉起左手穩握直立向上的青竹「{tool_raw}」（四指實心緊包纏繩刀柄）"),
            format!(
                "raised left hand cleanly wrapping all fingers around the cord-wrapped hilt of an upright pale celadon-green bamboo machete ({tool_raw}) pointing straight up"
            ),
        )
    } else if tool_raw.contains("開山鎬") || tool_raw.contains("鋼") {
        (
            format!("舉起左手緊握直立「{tool_raw}」（堅木長柄配深鐵鍛鋼雙頭鎬刃）"),
            format!(
                "raised left hand firmly gripping the sturdy wooden shaft of a forged iron-steel double-pointed pickaxe ({tool_raw}) held upright vertically"
            ),
        )
    } else if tool_raw.contains('鎬') {
        (
            format!("舉起左手緊握直立「{tool_raw}」（素木長柄穿過拳心，頂端橫綁雙尖楔形灰岩鎬頭）"),
            format!(
                "raised left hand firmly gripping the straight wooden shaft of a primitive stone pickaxe ({tool_raw}) with a horizontal T-shaped double-pointed grey rock pick head mounted perpendicular at the top (pointed pick ends, NOT an axe blade)"
            ),
        )
    } else if tool_raw.contains('斧') {
        (
            format!("舉起左手穩握直立「{tool_raw}」（長木柄穿過拳心上下延伸，頂端寬弧單刃鐵斧）"),
            format!(
                "raised left hand firmly gripping the straight wooden handle of a forged single-bit felling axe ({tool_raw}) held upright vertically"
            ),
        )
    } else {
        (
            format!("舉起左手垂直穩握「{tool_raw}」"),
            format!(
                "raised left hand firmly gripping a traditional wooden-handled hand tool ({tool_raw}) held upright"
            ),
        )
    };

    let gear_zh = format!(
        "{hat_zh}；灰藍粗麻立領捲袖襯衫外罩無袖{vest_zh}背心、銅扣厚牛皮腰帶、褐皮護腕與露指手套；{pack_zh}；深靛青工作長褲、米白小腿綁腿與褐皮高筒繫帶工作靴；{tool_zh}。"
    );

    let frame_en = if is_female {
        "slender young Taiwanese Han-descent frontier woman (about 7.4 heads tall, narrow shoulders, trim waist and hips, gentle feminine jawline, no beard)"
    } else {
        "sturdy athletic Taiwanese Han-descent man (about 7.4 heads tall, balanced shoulders and masculine proportions)"
    };

    let face_clean_en = if req.injured {
        ""
    } else {
        ", clean uninjured face with zero scratches or wounds"
    };

    let gen_prompt = format!(
        "Flat 2D Chinese Baimiao ink line art with minimalist muted low-saturation mineral watercolor wash, uniform crisp dark sepia ink (#332620) outlines of even stroke width, zero cross-hatching, zero heavy cast shadows, plain uniform pale cream rice paper background (#f4ecdc) with NO white sticker cutout border, no frame, and no text. \
         Draw one centered full-body front-facing standing portrait from top of head to boots, {frame_en}: {age_en}, {face_en}, {skin_en}, {hair_en}, {trace_en}{face_clean_en}; {hat_en}; wearing a slate-grey collared linen shirt with rolled sleeves under a sleeveless {vest_en} vest with brown leather waist belt, leather forearm bracers and half-finger gloves, {pack_en}, dark indigo work trousers tucked into off-white leg wraps and brown lace-up boots; {tool_en}."
    );

    let edit_prompt = format!(
        "Keep the exact same character's face, age, gender, skin tone, jawline, standing full-body proportions, flat minimalist #332620 sepia ink lineweight, and flat muted mineral watercolor style on uniform pale cream #f4ecdc paper background from the reference image. \
         Update ONLY the worn gear, held item, and facial injury state to match: {hair_en}, {hat_en}, sleeveless {vest_en} vest over slate-grey linen shirt, leather bracers and fingerless gloves, {pack_en}, dark indigo trousers with off-white leg wraps and brown boots; {tool_en}. {trace_en}{face_clean_en}."
    );

    let actor_identity = format!("{}_{}_{}", req.name, req.gender, req.soul_seed);
    let actor_slug = format!("sw_{}", hex_sha256_16(&actor_identity));

    let state_sig = format!(
        "{}|{}|{}|hat={}|pack={}|tool={}|inj={}",
        req.gender,
        req.soul_seed,
        vest_idx,
        u8::from(req.wear_hat),
        u8::from(req.show_pack),
        tool_raw,
        u8::from(req.injured)
    );
    let state_hash = hex_sha256_16(&state_sig);
    let deterministic_seed = ((base_hash & 0x7FFF_FFFF) as u32).max(1);

    PortraitTexts {
        actor_slug,
        state_hash,
        deterministic_seed,
        appearance_zh,
        gear_zh,
        style_zh: STYLE_ZH.to_string(),
        gen_prompt,
        edit_prompt,
    }
}

fn cache_paths(texts: &PortraitTexts) -> (PathBuf, PathBuf, PathBuf) {
    let dir = Path::new(CACHE_DIR);
    let target = dir.join(format!("{}_{}.png", texts.actor_slug, texts.state_hash));
    let base = dir.join(format!("{}_base.png", texts.actor_slug));
    let meta = dir.join(format!(
        "{}_{}.meta.json",
        texts.actor_slug, texts.state_hash
    ));
    (target, base, meta)
}

fn read_meta_engine(meta_path: &Path) -> String {
    fs::read_to_string(meta_path)
        .ok()
        .and_then(|s| serde_json::from_str::<CachedMeta>(&s).ok())
        .map(|m| m.engine)
        .filter(|e| !e.is_empty())
        .unwrap_or_else(|| format!("local:{LOCAL_SD_MODEL_LABEL}"))
}

fn encode_base64(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        out.push(TABLE[b0 >> 2] as char);
        out.push(TABLE[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((b1 & 0x0f) << 2) | (b2 >> 6)] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[b2 & 0x3f] as char
        } else {
            '='
        });
    }
    out
}

fn decode_base64(input: &str) -> anyhow::Result<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if bytes.is_empty() || bytes.len() % 4 != 0 {
        anyhow::bail!("invalid base64 length");
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks_exact(4) {
        let a = val(chunk[0]).ok_or_else(|| anyhow::anyhow!("bad b64 char"))?;
        let b = val(chunk[1]).ok_or_else(|| anyhow::anyhow!("bad b64 char"))?;
        out.push(((a << 2) | (b >> 4)) as u8);
        if chunk[2] != b'=' {
            let c = val(chunk[2]).ok_or_else(|| anyhow::anyhow!("bad b64 char"))?;
            out.push((((b & 0x0f) << 4) | (c >> 2)) as u8);
            if chunk[3] != b'=' {
                let d = val(chunk[3]).ok_or_else(|| anyhow::anyhow!("bad b64 char"))?;
                out.push((((c & 0x03) << 6) | d) as u8);
            }
        }
    }
    Ok(out)
}

fn decode_data_url_bytes(raw_url: &str) -> anyhow::Result<Vec<u8>> {
    let trimmed = raw_url.trim();
    if let Some((_, b64)) = trimmed.split_once(";base64,") {
        return decode_base64(b64);
    }
    decode_base64(trimmed)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn call_openrouter_image(
    api_key: &str,
    prompt: &str,
    ref_img_bytes: Option<&[u8]>,
) -> anyhow::Result<Vec<u8>> {
    let mut content_parts: Vec<Value> = Vec::new();
    if let Some(bytes) = ref_img_bytes {
        let b64 = encode_base64(bytes);
        content_parts.push(json!({
            "type": "image_url",
            "image_url": { "url": format!("data:image/png;base64,{b64}") }
        }));
    }
    content_parts.push(json!({ "type": "text", "text": prompt }));

    let body = json!({
        "model": DEFAULT_OPENROUTER_IMAGE_MODEL,
        "messages": [{ "role": "user", "content": content_parts }],
        "modalities": ["image", "text"],
        "image_config": { "aspect_ratio": "2:3" }
    });

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(65))
        .build()?;
    let resp = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .header("X-Title", "Singularity World Portraits")
        .json(&body)
        .send()?;
    if !resp.status().is_success() {
        let st = resp.status();
        let err_txt = resp.text().unwrap_or_default();
        anyhow::bail!(
            "OpenRouter image HTTP {st}: {}",
            err_txt.chars().take(180).collect::<String>()
        );
    }
    let val: Value = resp.json()?;
    let data_url = val
        .pointer("/choices/0/message/images/0/image_url/url")
        .and_then(Value::as_str)
        .or_else(|| {
            val.pointer("/choices/0/message/images/0/url")
                .and_then(Value::as_str)
        })
        .ok_or_else(|| anyhow::anyhow!("OpenRouter missing message.images[0].image_url.url"))?;
    decode_data_url_bytes(data_url)
}

fn call_local_sd_image(prompt: &str, ref_path: Option<&Path>) -> anyhow::Result<Vec<u8>> {
    let base = std::env::var("SW_LOCAL_SD_URL").unwrap_or_else(|_| DEFAULT_LOCAL_SD_URL.into());
    let base = base.trim_end_matches('/');
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(90))
        .build()?;

    let val: Value = if let Some(rp) = ref_path.filter(|p| p.exists()) {
        let img_part = multipart::Part::file(rp)?
            .file_name("ref.png")
            .mime_str("image/png")?;
        let form = multipart::Form::new()
            .part("image", img_part)
            .text("prompt", prompt.to_string())
            .text("size", "512x768".to_string())
            .text("output_format", "png".to_string());
        let resp = client
            .post(format!("{base}/v1/images/edits"))
            .multipart(form)
            .send()?;
        if !resp.status().is_success() {
            anyhow::bail!("local sd-server edits HTTP {}", resp.status());
        }
        resp.json()?
    } else {
        let resp = client
            .post(format!("{base}/v1/images/generations"))
            .json(&json!({
                "prompt": prompt,
                "size": "512x768",
                "output_format": "png",
                "n": 1
            }))
            .send()?;
        if !resp.status().is_success() {
            anyhow::bail!("local sd-server gen HTTP {}", resp.status());
        }
        resp.json()?
    };

    let b64 = val
        .pointer("/data/0/b64_json")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("local sd-server missing data[0].b64_json"))?;
    decode_base64(b64)
}

/// 實際執行單次雙軌生圖（Track A BYOK -> Fallback Track B）並原子寫盤。
pub fn generate_portrait_blocking(
    req: &PortraitRequest,
    texts: &PortraitTexts,
) -> anyhow::Result<String> {
    let (target, base, meta) = cache_paths(texts);
    if target.exists() {
        return Ok(read_meta_engine(&meta));
    }
    let ref_is_char = base.exists();
    let char_base_opt: Option<&Path> = if ref_is_char {
        Some(base.as_path())
    } else {
        None
    };
    let prompt_to_use = if ref_is_char {
        &texts.edit_prompt
    } else {
        &texts.gen_prompt
    };

    let mut used_engine = format!("local:{LOCAL_SD_MODEL_LABEL}");
    let mut img_bytes: Option<Vec<u8>> = None;

    if let Some(pid) = req.owner_player_id.as_deref()
        && let Some(api_key) = get_player_openrouter_key(pid)
    {
        let (or_prompt, ref_bytes) = if let Some(bp) = char_base_opt {
            (prompt_to_use.to_string(), fs::read(bp).ok())
        } else if Path::new(STYLE_REF_PATH).exists() {
            (
                format!(
                    "Use the attached reference image ONLY for its flat #332620 ink stroke lineweight, muted mineral watercolor palette, and #f4ecdc rice paper background — DO NOT copy the reference person's gender, age, face, hair, clothing, or pose. {prompt_to_use}"
                ),
                fs::read(STYLE_REF_PATH).ok(),
            )
        } else {
            (prompt_to_use.to_string(), None)
        };
        match call_openrouter_image(&api_key, &or_prompt, ref_bytes.as_deref()) {
            Ok(bytes) => {
                used_engine = format!("openrouter:{DEFAULT_OPENROUTER_IMAGE_MODEL}");
                img_bytes = Some(bytes);
            }
            Err(e) => {
                tracing::warn!(
                    "OpenRouter portrait failed for {}, falling back to local sd-server: {e}",
                    req.name
                );
            }
        }
    }

    let final_bytes = match img_bytes {
        Some(b) => b,
        // 本機 FLUX.2-klein 僅在「同角色換裝鎖定 base.png」走 edits；新角色初生成一律走純 txt2img 保萬人萬相不複製同一男相參考圖
        None => call_local_sd_image(prompt_to_use, char_base_opt)?,
    };

    write_atomic(&target, &final_bytes)?;
    if !base.exists() && !req.injured {
        let _ = write_atomic(&base, &final_bytes);
    }
    let meta_json = serde_json::to_vec(&CachedMeta {
        engine: used_engine.clone(),
        updated_unix: now_unix(),
    })?;
    let _ = write_atomic(&meta, &meta_json);
    Ok(used_engine)
}

/// 查詢靜態立繪快取；若尚未就緒則於單工背景任務觸發生圖，絕不阻塞 HTTP 主執行緒。
pub fn lookup_or_schedule(req: &PortraitRequest) -> PortraitLookup {
    let texts = synthesize_portrait_texts(req);
    let (target, base, meta) = cache_paths(&texts);
    if target.exists() {
        return PortraitLookup::Ready {
            path: target,
            engine: read_meta_engine(&meta),
            texts,
        };
    }

    let inflight_key = format!("{}_{}", texts.actor_slug, texts.state_hash);
    let should_spawn = {
        let mut guard = INFLIGHT_KEYS.lock().unwrap_or_else(|e| e.into_inner());
        guard.insert(inflight_key.clone())
    };
    if should_spawn {
        let req_bg = req.clone();
        let texts_bg = texts.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _permit = GEN_SEMAPHORE.acquire().await;
                let res = tokio::task::spawn_blocking(move || {
                    generate_portrait_blocking(&req_bg, &texts_bg)
                })
                .await;
                if let Ok(Err(e)) = res {
                    tracing::warn!("portrait generation error ({inflight_key}): {e}");
                }
                let mut guard = INFLIGHT_KEYS.lock().unwrap_or_else(|e| e.into_inner());
                guard.remove(&inflight_key);
            });
        } else {
            let mut guard = INFLIGHT_KEYS.lock().unwrap_or_else(|e| e.into_inner());
            guard.remove(&inflight_key);
        }
    }

    if base.exists() {
        PortraitLookup::Updating {
            base_path: base,
            engine: read_meta_engine(&meta),
            texts,
        }
    } else {
        PortraitLookup::Pending { texts }
    }
}

/// 清除指定狀態（或連同基準定裝照）以供使用者手動重繪。
pub fn invalidate_actor_cache(req: &PortraitRequest, include_base: bool) -> PortraitTexts {
    let texts = synthesize_portrait_texts(req);
    let (target, base, meta) = cache_paths(&texts);
    let _ = fs::remove_file(target);
    let _ = fs::remove_file(meta);
    if include_base {
        let _ = fs::remove_file(base);
    }
    texts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_req(tool: Option<&str>) -> PortraitRequest {
        PortraitRequest {
            id: 2,
            name: "林阿木".to_string(),
            gender: "M".to_string(),
            soul_seed: 108_421,
            role_tag: "百工木匠".to_string(),
            wear_hat: false,
            show_pack: true,
            tool_label: tool.map(str::to_string),
            injured: false,
            owner_player_id: None,
        }
    }

    #[test]
    fn texts_and_hash_reflect_gear_changes() {
        let t_blade = synthesize_portrait_texts(&sample_req(Some("烤刃竹刀")));
        let t_pick = synthesize_portrait_texts(&sample_req(Some("石刃木鎬")));
        assert_ne!(t_blade.state_hash, t_pick.state_hash);
        assert!(t_blade.appearance_zh.contains("。") && t_blade.gear_zh.contains("烤刃竹刀"));
        assert!(t_pick.gear_zh.contains("石刃木鎬"));
    }

    #[test]
    fn base64_roundtrip() {
        let src = b"SingularityWorld-PNG-Header\x00\xff\x89";
        let enc = encode_base64(src);
        let dec = decode_base64(&enc).expect("decode");
        assert_eq!(dec, src);
    }
}
