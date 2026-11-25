//! 離線批次為銀鷗島五鎮 674 名常住居民撰寫現代台灣正體白話《史記》列傳（`data/world/island_biographies_v3.json`）。
//!
//! - 直接自線上權威 `postgres-singularity` 唯讀讀取 NPC 快照與確定性 `CanonicalArcFact` 雙邊事實帳；
//! - 以 4 人微批次呼叫本機 `:11434` `google_gemma-4-26B-A4B-it-Q4_1`（預填 `<|thought|>\n<|thought|>` 零思考稅、每批 4 種不同開篇鏡頭防撞句）；
//! - 每篇強制通過 `validate_biography_prose` 機械守門員（零簡體、零文言虛詞、零 AI 雞湯詞、零鎮別地景穿幫、必含主角與據點名）；
//! - 支援斷點續跑（每 2 批原子寫盤 checkpoint），絕不寫入資料庫、絕不觸碰任何 `pl:` 玩家列。

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use r2d2_postgres::PostgresConnectionManager;
use r2d2_postgres::postgres::NoTls;
use serde_json::{Value, json};

use singularity_world::grid::outer_island::causal_arcs::{
    BAKED_BIOGRAPHIES_V3_PATH, BakedBiographyFile, CanonicalArcFact,
    generate_island_canonical_arcs, load_baked_biographies_map, render_deterministic_shiji_bio,
    solar_year_zh, validate_biography_prose,
};
use singularity_world::grid::outer_island::engine::load_static_world_doc;
use singularity_world::grid::outer_island::mind::tie_flags;
use singularity_world::grid::outer_island::sim_tick::seed_genesis_actors_once;
use singularity_world::grid::outer_island::types::{
    IslandWorldDelta, UnifiedMindBody, WorldStaticDoc,
};
use singularity_world::grid::outer_island::world_weave::apply_world_lineage_and_memory_weave;
use singularity_world::grid::outer_island::world_weave_bio::{
    settlement_name_zh, settlement_slug_of,
};

const LOCAL_LLM_URL: &str = "http://127.0.0.1:11434/v1/chat/completions";
const LOCAL_LLM_MODEL: &str = "google_gemma-4-26B-A4B-it-Q4_1";

const OPENING_LENSES: &[&str] = &[
    "以隨身器物的材質痕跡與日常幹活手勢切入，再自然帶出生年與門牌",
    "以所在街巷門戶的清晨或暮色聲響切入，帶出此人在屋裡的身分與行當",
    "以街坊鄰里怎麼稱呼或看待此人的處事規矩切入，再交代家世與生年",
    "以人物外貌老繭與操持活計的習慣動作切入，再點明生年與落腳處",
];

fn normalize_common_simplified(s: &str) -> String {
    const PAIRS: &[(char, char)] = &[
        ('\u{7b80}', '簡'),
        ('\u{7801}', '碼'),
        ('\u{7f51}', '網'),
        ('\u{7edc}', '絡'),
        ('\u{8c03}', '調'),
        ('\u{8ba4}', '認'),
        ('\u{5f02}', '異'),
        ('\u{7ebf}', '線'),
        ('\u{8fdb}', '進'),
        ('\u{76d8}', '盤'),
        ('\u{7f13}', '緩'),
        ('\u{7ec4}', '組'),
        ('\u{5757}', '塊'),
        ('\u{53c2}', '參'),
        ('\u{5bf9}', '對'),
        ('\u{9488}', '針'),
        ('\u{7ed3}', '結'),
        ('\u{6784}', '構'),
        ('\u{4e3e}', '舉'),
        ('\u{5b9e}', '實'),
        ('\u{73b0}', '現'),
        ('\u{7f16}', '編'),
        ('\u{8bd1}', '譯'),
        ('\u{8fd0}', '運'),
        ('\u{6d4b}', '測'),
        ('\u{8bd5}', '試'),
        ('\u{9519}', '錯'),
        ('\u{8bef}', '誤'),
        ('\u{76d1}', '監'),
        ('\u{8282}', '節'),
        ('\u{94a5}', '鑰'),
        ('\u{542f}', '啟'),
        ('\u{6001}', '態'),
        ('\u{53d1}', '發'),
        ('\u{8bbe}', '設'),
        ('\u{8bfb}', '讀'),
        ('\u{5199}', '寫'),
        ('\u{5e93}', '庫'),
        ('\u{6237}', '戶'),
        ('\u{7ea7}', '級'),
        ('\u{7c7b}', '類'),
        ('\u{5c42}', '層'),
        ('\u{5173}', '關'),
        ('\u{5e94}', '應'),
        ('\u{663e}', '顯'),
        ('\u{9690}', '隱'),
        ('\u{94b1}', '錢'),
        ('\u{94c1}', '鐵'),
        ('\u{9524}', '錘'),
        ('\u{95e8}', '門'),
        ('\u{9a6c}', '馬'),
        ('\u{8f66}', '車'),
        ('\u{9c7c}', '魚'),
        ('\u{9e1f}', '鳥'),
        ('\u{5c9b}', '島'),
        ('\u{8fd9}', '這'),
        ('\u{8bf4}', '說'),
        ('\u{4eec}', '們'),
        ('\u{4e2a}', '個'),
        ('\u{4f1a}', '會'),
        ('\u{6765}', '來'),
        ('\u{65f6}', '時'),
        ('\u{8fc7}', '過'),
        ('\u{8fd8}', '還'),
        ('\u{6ca1}', '沒'),
        ('\u{7ed9}', '給'),
        ('\u{8ba9}', '讓'),
        ('\u{5417}', '嗎'),
        ('\u{542c}', '聽'),
        ('\u{8bdd}', '話'),
        ('\u{89c1}', '見'),
        ('\u{5f00}', '開'),
        ('\u{95ee}', '問'),
        ('\u{4e1c}', '東'),
        ('\u{5934}', '頭'),
        ('\u{8fb9}', '邊'),
        ('\u{4e70}', '買'),
        ('\u{5356}', '賣'),
        ('\u{94f6}', '銀'),
        ('\u{94dc}', '銅'),
        ('\u{5e01}', '幣'),
        ('\u{77ff}', '礦'),
        ('\u{5251}', '劍'),
        ('\u{836f}', '藥'),
        ('\u{5e08}', '師'),
        ('\u{9986}', '館'),
        ('\u{94fa}', '鋪'),
        ('\u{9547}', '鎮'),
        ('\u{957f}', '長'),
        ('\u{5f20}', '張'),
        ('\u{4e24}', '兩'),
        ('\u{51e0}', '幾'),
        ('\u{513f}', '兒'),
        ('\u{503a}', '債'),
        ('\u{8d26}', '帳'),
        ('\u{5e10}', '帳'),
        ('\u{4f20}', '傳'),
        ('\u{8bb0}', '記'),
        ('\u{5fc6}', '憶'),
        ('\u{7237}', '爺'),
        ('\u{53f7}', '號'),
        ('\u{4e3a}', '為'),
        ('\u{4e48}', '麼'),
        ('\u{5f53}', '當'),
        ('\u{4ece}', '從'),
        ('\u{89c9}', '覺'),
        ('\u{6837}', '樣'),
        ('\u{5c81}', '歲'),
        ('\u{5386}', '曆'),
        ('\u{98ce}', '風'),
        ('\u{4e91}', '雲'),
        ('\u{7ef3}', '繩'),
        ('\u{76d0}', '鹽'),
        ('\u{9ea6}', '麥'),
        ('\u{7cae}', '糧'),
        ('\u{7d27}', '緊'),
        ('\u{96be}', '難'),
        ('\u{4eb2}', '親'),
        ('\u{65e7}', '舊'),
        ('\u{4f24}', '傷'),
        ('\u{62a4}', '護'),
        ('\u{7bd3}', '簍'),
        ('\u{62c5}', '擔'),
        ('\u{53f9}', '嘆'),
        ('\u{644a}', '攤'),
        ('\u{6ee9}', '灘'),
        ('\u{6e7e}', '灣'),
        ('\u{706f}', '燈'),
        ('\u{7089}', '爐'),
        ('\u{9505}', '鍋'),
        ('\u{9508}', '鏽'),
        ('\u{951a}', '錨'),
        ('\u{7f06}', '纜'),
        ('\u{6808}', '棧'),
        ('\u{6865}', '橋'),
        ('\u{5899}', '牆'),
        ('\u{9876}', '頂'),
        ('\u{6881}', '樑'),
        ('\u{9489}', '釘'),
        ('\u{952f}', '鋸'),
        ('\u{51ff}', '鑿'),
        ('\u{9570}', '鐮'),
        ('\u{9504}', '鋤'),
        ('\u{94f2}', '鏟'),
        ('\u{953b}', '鍛'),
        ('\u{70bc}', '煉'),
        ('\u{70e7}', '燒'),
        ('\u{70ed}', '熱'),
        ('\u{51bb}', '凍'),
        ('\u{9669}', '險'),
        ('\u{7a33}', '穩'),
        ('\u{5e2e}', '幫'),
        ('\u{886c}', '襯'),
        ('\u{4f19}', '夥'),
        ('\u{6512}', '攢'),
        ('\u{8d54}', '賠'),
        ('\u{8d5a}', '賺'),
        ('\u{8d34}', '貼'),
        ('\u{8865}', '補'),
        ('\u{7f1d}', '縫'),
        ('\u{7ec7}', '織'),
        ('\u{6652}', '曬'),
        ('\u{814c}', '醃'),
        ('\u{54b8}', '鹹'),
        ('\u{9c9c}', '鮮'),
        ('\u{6d53}', '濃'),
        ('\u{96fe}', '霧'),
        ('\u{6e7f}', '濕'),
        ('\u{810f}', '髒'),
        ('\u{4e71}', '亂'),
        ('\u{65ad}', '斷'),
        ('\u{788e}', '碎'),
        ('\u{88c2}', '裂'),
        ('\u{636e}', '據'),
    ];
    let mut out = String::with_capacity(s.len());
    let map: BTreeMap<char, char> = PAIRS.iter().copied().collect();
    let digits = ['〇', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
    for ch in s.chars() {
        if let Some(d) = ch.to_digit(10) {
            out.push(digits[d as usize]);
        } else {
            out.push(map.get(&ch).copied().unwrap_or(ch));
        }
    }
    out.replace("一〇號", "十號")
        .replace("咱們", "我們")
        .replace("喝西北風", "挨餓受凍")
        .replace("一碼歸一碼", "一筆歸一筆")
        .replace("乾淨利落", "乾淨俐落")
        .replace("幫襯", "照應")
        .replace("拮據", "緊絀")
        .replace("拮\u{636e}", "緊絀")
        .replace("傢夥", "傢伙")
        .replace("小夥", "小伙")
        .replace("扎根", "落腳")
        .replace("見證了歲月", "留著經年磨痕")
        .replace("人際網絡", "往來交情")
        .replace("溫暖而靈通的連結", "互通有無的照應")
        .replace("溫暖的連結", "實在的照應")
        .replace("並不孤單", "總有熟人照應")
        .replace("風裡的光", "雪夜爐火")
        .replace("歲月靜好", "日子平順")
        .replace("不僅是", "既是")
        .replace("更是她", "也是她")
        .replace("更是他", "也是他")
        .replace("——", "，")
}

fn load_world_snapshot(doc: &WorldStaticDoc) -> IslandWorldDelta {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/singularity".to_string());
    if let Ok(cfg) = db_url.parse() {
        let mgr = PostgresConnectionManager::new(cfg, NoTls);
        if let Ok(pool) = r2d2::Pool::builder().max_size(2).build(mgr)
            && let Ok(mut conn) = pool.get()
            && let Ok(rows) = conn.query(
                "SELECT state_json FROM oi_actors WHERE NOT is_player ORDER BY actor_id",
                &[],
            )
            && !rows.is_empty()
        {
            let mut delta = IslandWorldDelta {
                genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
                municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
                silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
                ..IslandWorldDelta::default()
            };
            for r in rows {
                let sjson: String = r.get(0);
                if let Ok(actor) = serde_json::from_str::<UnifiedMindBody>(&sjson) {
                    if actor.controller_id.is_none() && !actor.actor_id.starts_with("pl:") {
                        delta.actors.insert(actor.actor_id.clone(), actor);
                    }
                }
            }
            let _ = apply_world_lineage_and_memory_weave(doc, &mut delta);
            return delta;
        }
    }
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(doc, &mut delta, false);
    delta
}

fn format_actor_fact_sheet(
    doc: &WorldStaticDoc,
    a: &UnifiedMindBody,
    arcs: &[CanonicalArcFact],
    lens_idx: usize,
) -> String {
    let town_zh = settlement_name_zh(doc, a);
    let door_zh = doc
        .nodes
        .get(&a.home_node)
        .map_or_else(|| town_zh.clone(), |n| n.name.replace('・', "．"));
    let by_zh = solar_year_zh(a.birth_year);
    let lens = OPENING_LENSES[lens_idx % OPENING_LENSES.len()];

    let mut arc_lines = Vec::new();
    for (k, arc) in arcs.iter().take(2).enumerate() {
        arc_lines.push(format!(
            "本末{}：{}；{}；當場原話：「{}」；後續：{}。",
            k + 1,
            arc.crisis_setup_zh,
            arc.action_climax_zh,
            arc.spoken_line_zh,
            arc.lasting_mark_zh
        ));
    }
    for ep in a
        .mind
        .ltm
        .episodic
        .iter()
        .filter(|e| e.event_id.starts_with("hist:wgh:") || e.event_id.starts_with("weave:hh:"))
        .take(1)
    {
        arc_lines.push(format!("家門或公史脈絡：{}", ep.subjective_zh));
    }

    let mut soc_bits = Vec::new();
    let mut sorted_soc: Vec<_> = a
        .mind
        .ltm
        .social
        .values()
        .filter(|s| !s.target_id.starts_with("pl:"))
        .collect();
    sorted_soc.sort_by_key(|s| std::cmp::Reverse((s.warmth.abs(), s.familiarity)));
    for s in sorted_soc.into_iter().take(3) {
        let reg = if !s.target_region_zh.is_empty() && s.target_region_zh != town_zh {
            format!("（現居{}）", s.target_region_zh)
        } else {
            String::new()
        };
        let _ = tie_flags::BLOOD_KIN;
        soc_bits.push(format!(
            "{}{}[{}｜{}]",
            s.target_name, reg, s.tie_label_zh, s.notable_note_zh
        ));
    }

    format!(
        "===INPUT:{id}===\n[指定開篇切入法：{lens}]\n姓名：{name}｜種族：{race}｜生年：太陽曆{by_zh}年｜據點：{town_zh}｜住址：{door_zh}（{bond}）｜行當：{role}\n外貌：{app}｜隨身器物：{wname}（{wprov}）\n{arcs}\n今日人際（提及人名須保留完整氏族前綴）：{socs}\n",
        id = a.actor_id,
        name = a.display_name,
        race = a.race,
        bond = a.household_bond,
        role = a.role,
        app = a.appearance,
        wname = a.bound_weapon.name,
        wprov = a.bound_weapon.provenance,
        arcs = arc_lines.join("\n"),
        socs = soc_bits.join("；"),
    )
}

fn parse_batch_output(raw: &str, expected_first_id: &str) -> BTreeMap<String, String> {
    let mut cleaned = raw.replace("<|thought|>\n<|thought|>", "");
    if !cleaned.trim_start().starts_with("===ACTOR:") {
        cleaned = format!("===ACTOR:{expected_first_id}===\n{cleaned}");
    }
    let mut out = BTreeMap::new();
    let mut cur_id: Option<String> = None;
    let mut buf = Vec::new();
    for line in cleaned.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("===ACTOR:")
            && let Some(id_part) = rest.strip_suffix("===")
        {
            if let Some(prev) = cur_id.take() {
                let text = normalize_common_simplified(buf.join("\n").trim());
                if !text.is_empty() {
                    out.insert(prev, text);
                }
            }
            cur_id = Some(id_part.trim().to_string());
            buf.clear();
        } else if cur_id.is_some() {
            buf.push(line);
        }
    }
    if let Some(prev) = cur_id {
        let text = normalize_common_simplified(buf.join("\n").trim());
        if !text.is_empty() {
            out.insert(prev, text);
        }
    }
    out
}

fn save_checkpoint(map: &BTreeMap<String, String>) -> Result<(), String> {
    let path = Path::new(BAKED_BIOGRAPHIES_V3_PATH);
    let tmp = path.with_extension("json.tmp");
    let payload = BakedBiographyFile {
        version: 3,
        biographies: map.clone(),
    };
    let json_str = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json_str).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn main() -> Result<(), String> {
    let doc = load_static_world_doc();
    let delta = load_world_snapshot(&doc);
    let (_, arcs_by_actor) = generate_island_canonical_arcs(&doc, &delta);
    let mut baked = load_baked_biographies_map();

    let mut targets: Vec<&UnifiedMindBody> = delta
        .actors
        .values()
        .filter(|a| {
            a.controller_id.is_none()
                && !a.actor_id.starts_with("pl:")
                && a.alive
                && !a.household_id.is_empty()
        })
        .collect();
    targets.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));

    let pending: Vec<&UnifiedMindBody> = targets
        .iter()
        .copied()
        .filter(|a| {
            let slug = settlement_slug_of(&a.household_id);
            let tzh = settlement_name_zh(&doc, a);
            baked.get(&a.actor_id).is_none_or(|txt| {
                validate_biography_prose(&a.display_name, slug, &tzh, txt).is_err()
            })
        })
        .collect();

    println!(
        "[bake-biographies] 總常住居民 {} 人｜已合格快取 {} 人｜待撰寫 {} 人",
        targets.len(),
        targets.len().saturating_sub(pending.len()),
        pending.len()
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(95))
        .build()
        .map_err(|e| e.to_string())?;

    let started = Instant::now();
    let mut llm_ok = 0usize;
    let mut det_fallback = 0usize;

    for (b_idx, chunk) in pending.chunks(5).enumerate() {
        let mut inputs = String::new();
        for (k, a) in chunk.iter().enumerate() {
            let my_arcs = arcs_by_actor
                .get(&a.actor_id)
                .map_or(&[][..], |v| v.as_slice());
            inputs.push_str(&format_actor_fact_sheet(&doc, a, my_arcs, b_idx + k));
            inputs.push('\n');
        }
        let first_id = &chunk[0].actor_id;
        let prompt = format!(
            "你是《奇點世界》史官，請用「純粹現代台灣正體白話散文」依序為下列居民撰寫《人物列傳》。\n\
            【寫作鐵律】：\n\
            1. 採《史記》列傳結構但全用現代白話：第一段依指定切入法立人（必含全名、所在據點名、生年、行當與隨身器物來歷）；第二段連貫敘述兩樁本末事件的前因、當場動作、原話與後果；第三段寫今日屋簷內外與跨鎮親友的具體互動（人名須保留完整前綴如「重擔莎拉」）。\n\
            2. 嚴禁文言虛詞（禁：者也、乃一、其人、未嘗、遂於、遂將、誠可謂、嗚呼、太史公曰）。\n\
            3. 嚴禁 AI 雞湯詞（禁：風裡的光、命運的齒輪、歲月靜好、無聲的訴說、人際網絡、溫暖的連結、見證了歲月、詮釋了、不僅是、並不孤單）。\n\
            4. 嚴禁條列符號、小標題或括號阿拉伯數字（年份一律寫中文數字如太陽曆三〇六年）；灰松林驛為內陸山林，嚴禁寫封凍海灣或出海捕魚；絕不編造事實帳以外的人名或銅幣金額。\n\
            5. 每人約 360～480 字（分 3 個自然段），篇首固定獨佔一行 `===ACTOR:<actor_id>===`。\n\n{inputs}"
        );

        let body = json!({
            "model": LOCAL_LLM_MODEL,
            "messages": [
                { "role": "user", "content": prompt },
                { "role": "assistant", "content": format!("<|thought|>\n<|thought|>===ACTOR:{first_id}===\n") }
            ],
            "max_tokens": 3000,
            "temperature": 0.62
        });

        let mut parsed_map = BTreeMap::new();
        if let Ok(resp) = client.post(LOCAL_LLM_URL).json(&body).send()
            && let Ok(val) = resp.json::<Value>()
            && let Some(content) = val["choices"][0]["message"]["content"].as_str()
        {
            parsed_map = parse_batch_output(content, first_id);
        }

        for a in chunk {
            let slug = settlement_slug_of(&a.household_id);
            let tzh = settlement_name_zh(&doc, a);
            let my_arcs = arcs_by_actor
                .get(&a.actor_id)
                .map_or(&[][..], |v| v.as_slice());
            match parsed_map.get(&a.actor_id) {
                Some(cand) => match validate_biography_prose(&a.display_name, slug, &tzh, cand) {
                    Ok(()) => {
                        baked.insert(a.actor_id.clone(), cand.clone());
                        llm_ok += 1;
                    }
                    Err(reason) => {
                        eprintln!(
                            "[bake-warn] {} LLM 未過閘 ({reason})，採確定性白話列傳保底",
                            a.actor_id
                        );
                        let fb = render_deterministic_shiji_bio(&doc, a, my_arcs);
                        baked.insert(a.actor_id.clone(), fb);
                        det_fallback += 1;
                    }
                },
                None => {
                    eprintln!(
                        "[bake-warn] {} 批次漏切標記，採確定性白話列傳保底",
                        a.actor_id
                    );
                    let fb = render_deterministic_shiji_bio(&doc, a, my_arcs);
                    baked.insert(a.actor_id.clone(), fb);
                    det_fallback += 1;
                }
            }
        }

        if b_idx % 2 == 1 || (b_idx + 1) * 5 >= pending.len() {
            save_checkpoint(&baked)?;
            println!(
                "[bake-biographies] 批次 {}/{} 完成｜累計合格 {}/{}（本輪 LLM {}・保底 {}）｜耗時 {:.1}s",
                b_idx + 1,
                pending.len().div_ceil(5),
                baked.len(),
                targets.len(),
                llm_ok,
                det_fallback,
                started.elapsed().as_secs_f64()
            );
            use std::io::Write as _;
            let _ = std::io::stdout().flush();
        }
    }

    save_checkpoint(&baked)?;
    println!(
        "[bake-biographies] 全島 {} 人白話列傳定稿落盤完成（LLM {}・保底 {}）！",
        baked.len(),
        llm_ok,
        det_fallback
    );
    Ok(())
}
