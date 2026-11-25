//! 背景活史官（Living Chronicler）：五維現場切片型別、讀多寫少 `Arc` 狀態機與共用輔助算子。
//!
//! 依據審辯室 Room `f75af676` 與 `Q4-ColdFusion-27B` 事實卡直寫架構實作：
//! 1. **徹底解耦 `turning_points`**：AI 慢思考內部決策口號（`slow:*`）留於 `turning_points` 驅動行為，100% 撤出人物列傳；
//! 2. **同筆交易 Outbox ＋ 事實卡直寫（Fact-Card Synthesis）**：重大里程碑發生當下捕捉 5D 現場快照與保底白話草稿，隨 `execute_commit_batch_pg` 同一 PostgreSQL 交易落盤至 `oi_chronicle_projections`，再由離鎖 `Q4-ColdFusion-27B` 依純事實卡撰寫自然白話散文；
//! 3. **方輿志單段就地彙整（零疊段洗版）**：每個城鎮之當代近況於 `reg:<town_slug>` 維持單一彙整段落（`deed:region_live:<town_slug>`），隨新事蹟就地更新覆寫，絕不重複堆疊「已有二戶／已有三戶」；
//! 4. **嚴守 M4 玩家隱私遮罩與濱海／內陸地形分流**：凡涉及真人玩家（`pl:*`）於共享表面一律鎖死為「外來旅人」；依真實 `VoronoiCellDef.is_coast` 區分濱海潮汐與內陸山谷天候。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::causal_arcs::solar_year_zh;
use super::clock_and_lang::{SolarClockSnapshot, solar_year_and_season_of_day};
use super::gazetteer::town_region_article_id;
use super::harbor_lineage::CURRENT_SOLAR_YEAR;
use super::live_tide::derive_tide_clock_details;
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

pub use super::living_chronicler_deeds::{
    record_career_shift_dock, record_crisis_rescue, record_cross_sea_arrival,
    record_homestead_built, record_pact_default, record_primordial_spider_slain,
    record_vacant_house_bought, record_vessel_forged, record_witnessed_phase,
};
pub use super::living_chronicler_gate::{
    normalize_chronicler_output, validate_polished_deed_prose,
};

/// 五維現場事實切片（5D Scene-Snapshot Fact Packet，直接驅動 `Q4-ColdFusion-27B` 事實卡寫作）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChronicleFactPacket {
    pub deed_id: String,
    pub kind: String,
    pub game_day: i64,
    pub solar_year: i16,
    pub season_zh: String,
    pub bell_zh: String,
    pub weather_zh: String,
    pub tide_zh: String,
    pub town_slug: String,
    pub town_zh: String,
    pub place_zh: String,
    pub agent_id: String,
    pub agent_public_label: String,
    pub agent_role_zh: String,
    pub patient_id: String,
    pub patient_public_label: String,
    pub patient_role_zh: String,
    pub forbidden_raw_names: Vec<String>,
    pub amount_copper: i64,
    pub vessel_zh: String,
    pub phase_zh: String,
    pub extra_context_zh: String,
    pub bullet_facts_zh: Vec<String>,
    pub allowed_numbers_zh: Vec<String>,
    pub witness_names: Vec<String>,
}

/// 正規化編年投射列（對應 PostgreSQL `oi_chronicle_projections` 單列）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChronicleProjectionRow {
    pub deed_id: String,
    pub scope: String, // "actor" | "region" | "annals"
    pub target_id: String,
    pub kind: String,
    pub latch_key: String,
    pub title_zh: String,
    pub subtitle_zh: String,
    pub era_zh: String,
    pub carrier_zh: String,
    pub prose_zh: String,
    pub draft_zh: String,
    pub fact_json: String,
    pub solar_year: i16,
    pub game_day: i64,
    pub status: String, // "draft" | "polished" | "settled_draft"
    pub updated_seq: i64,
}

#[derive(Debug, Clone, Default)]
pub struct LivingChronicleCommitted {
    pub actor_chapters: BTreeMap<String, Vec<ChronicleProjectionRow>>,
    pub region_notes: BTreeMap<String, Vec<ChronicleProjectionRow>>,
    pub dynamic_annals: BTreeMap<String, ChronicleProjectionRow>,
    pub actor_cooldowns: BTreeMap<(String, String), i64>,
    pub fired_latches: BTreeSet<String>,
    pub town_default_days: BTreeMap<String, BTreeSet<(i64, String)>>,
    pub max_updated_seq: i64,
    pub polished_seq: i64,
    pub draft_gen: u64,
}

#[derive(Debug, Clone, Default)]
pub struct LivingChronicleState {
    pub committed: Arc<LivingChronicleCommitted>,
    pub pending_outbox: Vec<ChronicleProjectionRow>,
}

impl LivingChronicleState {
    pub fn export_watermark(&self) -> (i64, u64) {
        (self.committed.max_updated_seq, self.committed.draft_gen)
    }

    pub fn apply_row_in_memory(&mut self, row: ChronicleProjectionRow) {
        let store = Arc::make_mut(&mut self.committed);
        if row.updated_seq > store.max_updated_seq {
            store.max_updated_seq = row.updated_seq;
        }
        if row.status != "draft" && row.updated_seq > store.polished_seq {
            store.polished_seq = row.updated_seq;
        }
        if !row.latch_key.is_empty() {
            store.fired_latches.insert(row.latch_key.clone());
        }
        match row.scope.as_str() {
            "actor" => {
                store
                    .actor_cooldowns
                    .entry((row.target_id.clone(), row.kind.clone()))
                    .and_modify(|d| *d = (*d).max(row.game_day))
                    .or_insert(row.game_day);
                if row.kind == "pact_default"
                    && let Ok(fp) = serde_json::from_str::<ChronicleFactPacket>(&row.fact_json)
                    && !fp.town_slug.is_empty()
                {
                    store
                        .town_default_days
                        .entry(fp.town_slug)
                        .or_default()
                        .insert((row.game_day, row.deed_id.clone()));
                }
                let vec = store
                    .actor_chapters
                    .entry(row.target_id.clone())
                    .or_default();
                if let Some(existing) = vec.iter_mut().find(|r| r.deed_id == row.deed_id) {
                    if row.draft_zh != existing.draft_zh
                        || (row.status != "draft" && existing.status == "draft")
                        || row.updated_seq > existing.updated_seq
                    {
                        *existing = row;
                    }
                } else {
                    vec.push(row);
                }
            }
            "region" => {
                let vec = store.region_notes.entry(row.target_id.clone()).or_default();
                if let Some(existing) = vec.iter_mut().find(|r| r.deed_id == row.deed_id) {
                    if row.draft_zh != existing.draft_zh
                        || (row.status != "draft" && existing.status == "draft")
                        || row.updated_seq > existing.updated_seq
                    {
                        *existing = row;
                    }
                } else {
                    vec.push(row);
                }
            }
            "annals" => match store.dynamic_annals.get_mut(&row.target_id) {
                Some(existing) if existing.deed_id == row.deed_id => {
                    if row.draft_zh != existing.draft_zh
                        || (row.status != "draft" && existing.status == "draft")
                        || row.updated_seq > existing.updated_seq
                    {
                        *existing = row;
                    }
                }
                Some(existing) => {
                    if (row.game_day, &row.deed_id) > (existing.game_day, &existing.deed_id) {
                        *existing = row;
                    }
                }
                None => {
                    store.dynamic_annals.insert(row.target_id.clone(), row);
                }
            },
            _ => {}
        }
    }

    pub fn push_new_projection(&mut self, mut row: ChronicleProjectionRow) {
        let same_draft_exists = match row.scope.as_str() {
            "actor" => self
                .committed
                .actor_chapters
                .get(&row.target_id)
                .is_some_and(|v| {
                    v.iter()
                        .any(|x| x.deed_id == row.deed_id && x.draft_zh == row.draft_zh)
                }),
            "region" => self
                .committed
                .region_notes
                .get(&row.target_id)
                .is_some_and(|v| {
                    v.iter()
                        .any(|x| x.deed_id == row.deed_id && x.draft_zh == row.draft_zh)
                }),
            "annals" => self
                .committed
                .dynamic_annals
                .get(&row.target_id)
                .is_some_and(|x| x.deed_id == row.deed_id && x.draft_zh == row.draft_zh),
            _ => false,
        };
        if same_draft_exists {
            return;
        }
        row.updated_seq = 0;
        {
            let store = Arc::make_mut(&mut self.committed);
            store.draft_gen = store.draft_gen.saturating_add(1);
        }
        self.pending_outbox.retain(|x| {
            !(x.deed_id == row.deed_id && x.scope == row.scope && x.target_id == row.target_id)
        });
        self.pending_outbox.push(row.clone());
        self.apply_row_in_memory(row);
    }

    pub fn actor_paragraphs(&self, actor_id: &str) -> Vec<String> {
        self.committed
            .actor_chapters
            .get(actor_id)
            .map(|v| v.iter().map(|r| r.prose_zh.clone()).collect())
            .unwrap_or_default()
    }

    pub fn region_paragraphs(&self, region_article_id: &str) -> Vec<String> {
        self.committed
            .region_notes
            .get(region_article_id)
            .map(|v| v.iter().map(|r| r.prose_zh.clone()).collect())
            .unwrap_or_default()
    }

    pub fn dynamic_annals_sorted(&self) -> Vec<&ChronicleProjectionRow> {
        let mut list: Vec<&ChronicleProjectionRow> =
            self.committed.dynamic_annals.values().collect();
        list.sort_by(|a, b| {
            a.game_day
                .cmp(&b.game_day)
                .then_with(|| a.target_id.cmp(&b.target_id))
        });
        list
    }
}

pub fn preserve_polished_projections(
    live: &LivingChronicleState,
    working: &mut LivingChronicleState,
) {
    if live.committed.polished_seq <= working.committed.polished_seq {
        return;
    }
    for rows in live.committed.actor_chapters.values() {
        for r in rows {
            if r.status != "draft" {
                if working
                    .committed
                    .actor_chapters
                    .get(&r.target_id)
                    .and_then(|v| v.iter().find(|w| w.deed_id == r.deed_id))
                    .is_some_and(|w| w.draft_zh == r.draft_zh)
                {
                    working.apply_row_in_memory(r.clone());
                }
            }
        }
    }
    for rows in live.committed.region_notes.values() {
        for r in rows {
            if r.status != "draft" {
                if working
                    .committed
                    .region_notes
                    .get(&r.target_id)
                    .and_then(|v| v.iter().find(|w| w.deed_id == r.deed_id))
                    .is_some_and(|w| w.draft_zh == r.draft_zh)
                {
                    working.apply_row_in_memory(r.clone());
                }
            }
        }
    }
    for r in live.committed.dynamic_annals.values() {
        if r.status != "draft" {
            if working
                .committed
                .dynamic_annals
                .get(&r.target_id)
                .is_some_and(|w| w.deed_id == r.deed_id && w.draft_zh == r.draft_zh)
            {
                working.apply_row_in_memory(r.clone());
            }
        }
    }
}

pub fn num_zh(n: i64) -> String {
    let v = n.max(0);
    let digits = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
    if v < 10 {
        return digits[v as usize].to_string();
    }
    if v < 20 {
        return if v == 10 {
            "十".to_string()
        } else {
            format!("十{}", digits[(v % 10) as usize])
        };
    }
    if v < 100 {
        let tens = digits[(v / 10) as usize];
        let rem = v % 10;
        return if rem == 0 {
            format!("{tens}十")
        } else {
            format!("{tens}十{}", digits[rem as usize])
        };
    }
    if v < 1_000 {
        let h = digits[(v / 100) as usize];
        let rem = v % 100;
        return if rem == 0 {
            format!("{h}百")
        } else if rem < 10 {
            format!("{h}百零{}", digits[rem as usize])
        } else if rem < 20 {
            format!("{h}百一{}", num_zh(rem).trim_start_matches('十'))
        } else {
            format!("{h}百{}", num_zh(rem))
        };
    }
    if v < 10_000 {
        let th = digits[(v / 1_000) as usize];
        let rem = v % 1_000;
        return if rem == 0 {
            format!("{th}千")
        } else if rem < 100 {
            format!("{th}千零{}", num_zh(rem))
        } else {
            format!("{th}千{}", num_zh(rem))
        };
    }
    let wan = v / 10_000;
    let rem = v % 10_000;
    if rem == 0 {
        format!("{}萬", num_zh(wan))
    } else if rem < 1_000 {
        format!("{}萬零{}", num_zh(wan), num_zh(rem))
    } else {
        format!("{}萬{}", num_zh(wan), num_zh(rem))
    }
}

pub(super) fn clean_place_zh(raw: &str) -> String {
    let s = raw.replace('・', "．");
    let mut out = String::with_capacity(s.len() + 8);
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            let mut buf = String::from(c);
            while let Some(&nc) = chars.peek() {
                if nc.is_ascii_digit() {
                    buf.push(nc);
                    let _ = chars.next();
                } else {
                    break;
                }
            }
            if let Ok(parsed) = buf.parse::<i64>() {
                out.push_str(&num_zh(parsed));
            } else {
                out.push_str(&buf);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// 剝除節點名稱尾綴之「空屋」機制字眼（如 `"苔井巷9號空屋"` -> `"苔井巷九號"`）
pub(super) fn clean_house_door_zh(raw: &str) -> String {
    let stripped = raw.trim_end_matches("空屋").trim();
    clean_place_zh(stripped)
}

pub fn town_slug_of_node(node_id: &str) -> &'static str {
    if node_id.contains("gull_tail_port") {
        "gull_tail_port"
    } else if node_id.contains("west_strait_ferry") {
        "west_strait_ferry"
    } else if node_id.contains("greypine_post")
        || node_id.contains("dng_pit")
        || node_id.contains("north_woods")
        || node_id.contains("mine")
        || node_id.contains("spider")
        || node_id.contains("bridge")
    {
        "greypine_post"
    } else if node_id.contains("north_cape_beacon") {
        "north_cape_beacon"
    } else if node_id.contains("burg_") {
        "island_rural"
    } else if node_id.contains("cell") {
        "island_wilds"
    } else {
        "white_gull_harbor"
    }
}

pub fn town_zh_of_slug(slug: &str) -> &'static str {
    match slug {
        "gull_tail_port" => "鷗尾漁港",
        "west_strait_ferry" => "西峽渡口",
        "greypine_post" => "灰松林驛",
        "north_cape_beacon" => "海風北塢",
        "island_rural" => "銀鷗鄉野",
        "island_wilds" => "銀鷗山野",
        _ => "白鷗港",
    }
}

pub fn is_node_coastal(doc: &WorldStaticDoc, node_id: &str, town_slug: &str) -> bool {
    if town_slug == "greypine_post" {
        return false;
    }
    if matches!(
        town_slug,
        "white_gull_harbor" | "gull_tail_port" | "west_strait_ferry" | "north_cape_beacon"
    ) {
        return true;
    }
    if let Some(nd) = doc.nodes.get(node_id) {
        const UPLAND_MARKERS: &[&str] = &[
            "山腰",
            "燒炭",
            "礦溪",
            "洗煤",
            "幽林",
            "深林",
            "試煉坑",
            "山谷",
            "林莊",
            "石堡",
            "草坡",
            "水磨",
            "曬穀",
        ];
        if UPLAND_MARKERS.iter().any(|m| nd.name.contains(m)) {
            return false;
        }
        if let Some(cell) = doc.cells.get(&nd.cell_id.to_string()) {
            return cell.is_coast;
        }
    }
    false
}

pub(super) fn coastal_sea_tide_zh(bell_index: u8) -> &'static str {
    match bell_index {
        3..=4 => "早潮漲水",
        5..=6 => "退潮平浪",
        7..=8 => "午潮拍岸",
        9..=10 => "晚潮退水",
        _ => "夜潮輕拍堤岸",
    }
}

pub(super) fn inland_safe_weather_zh(raw_weather: &str) -> &'static str {
    match raw_weather {
        "海風" => "山風清勁",
        "微雨" => "微雨",
        "薄霧" => "薄霧",
        _ => "晴朗",
    }
}

pub fn is_player_actor(delta: &IslandWorldDelta, actor_id: &str) -> bool {
    actor_id.starts_with("pl:")
        || delta
            .actors
            .get(actor_id)
            .is_some_and(|a| a.controller_id.is_some() || a.actor_id.starts_with("pl:"))
}

pub fn public_actor_label(delta: &IslandWorldDelta, actor_id: &str) -> (String, Option<String>) {
    if actor_id.is_empty() {
        return (String::new(), None);
    }
    if is_player_actor(delta, actor_id) {
        let raw_name = delta
            .actors
            .get(actor_id)
            .map(|a| a.display_name.clone())
            .filter(|s| !s.is_empty() && s != "外來旅人");
        return ("外來旅人".to_string(), raw_name);
    }
    let name = delta
        .actors
        .get(actor_id)
        .map_or_else(|| "島上居民".to_string(), |a| a.display_name.clone());
    (name, None)
}

fn non_player_witness_names(
    delta: &IslandWorldDelta,
    node_id: &str,
    exclude_a: &str,
    exclude_b: &str,
) -> Vec<String> {
    let Some(occ) = delta.node_occupants.get(node_id) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for aid in occ {
        if aid == exclude_a || aid == exclude_b || is_player_actor(delta, aid) {
            continue;
        }
        if let Some(a) = delta.actors.get(aid)
            && a.alive
            && !a.display_name.is_empty()
        {
            out.push(a.display_name.clone());
            if out.len() >= 2 {
                break;
            }
        }
    }
    out
}

pub(super) fn atmosphere_clause(
    is_coastal: bool,
    weather_zh: &str,
    tide_zh: &str,
    bell_zh: &str,
) -> String {
    if !is_coastal {
        let w = match weather_zh {
            "微雨" => "細密山雨打濕了松針與碎石路面",
            "薄霧" => "微涼山嵐漫過林間與石階",
            _ => "山谷涼風掠過屋簷與林梢",
        };
        format!("{bell_zh}前後，{w}")
    } else {
        let w = match weather_zh {
            "微雨" => format!("細雨打濕了岸邊石板，港外正逢{tide_zh}"),
            "薄霧" => format!("微涼薄霧籠在岸際，水面正逢{tide_zh}"),
            "海風" => format!("帶著鹹味的海風吹過屋脊，岸邊正值{tide_zh}"),
            _ => format!("天色清朗，岸邊正值{tide_zh}"),
        };
        format!("{bell_zh}前後，{w}")
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "5D 現場事實切片需完整接收世界、時鐘與雙邊角色錨點"
)]
pub(super) fn build_fact_packet(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    deed_id: String,
    kind: &str,
    node_id: &str,
    agent_id: &str,
    patient_id: &str,
) -> ChronicleFactPacket {
    let (yr, season) = solar_year_and_season_of_day(clock.game_days);
    let solar_year = yr.max(CURRENT_SOLAR_YEAR);
    let td = derive_tide_clock_details(clock);
    let town_slug = town_slug_of_node(node_id);
    let coastal = is_node_coastal(doc, node_id, town_slug);
    let place_zh = doc.nodes.get(node_id).map_or_else(
        || town_zh_of_slug(town_slug).to_string(),
        |n| clean_place_zh(&n.name),
    );
    let town_zh = if matches!(town_slug, "island_rural" | "island_wilds") {
        place_zh
            .split('．')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| town_zh_of_slug(town_slug))
            .to_string()
    } else {
        town_zh_of_slug(town_slug).to_string()
    };
    let (agent_public_label, forbid_a) = public_actor_label(delta, agent_id);
    let (patient_public_label, forbid_b) = public_actor_label(delta, patient_id);
    let agent_role_zh = if is_player_actor(delta, agent_id) {
        String::new()
    } else {
        delta
            .actors
            .get(agent_id)
            .map(|a| format!("{}{}", a.race, a.role))
            .unwrap_or_default()
    };
    let patient_role_zh = if is_player_actor(delta, patient_id) {
        String::new()
    } else {
        delta
            .actors
            .get(patient_id)
            .map(|a| format!("{}{}", a.race, a.role))
            .unwrap_or_default()
    };
    let mut forbidden_raw_names = Vec::new();
    if let Some(na) = forbid_a {
        forbidden_raw_names.push(na);
    }
    if let Some(nb) = forbid_b
        && !forbidden_raw_names.contains(&nb)
    {
        forbidden_raw_names.push(nb);
    }
    let weather_zh = if coastal {
        td.weather_zh.to_string()
    } else {
        inland_safe_weather_zh(td.weather_zh).to_string()
    };
    let tide_zh = if coastal {
        coastal_sea_tide_zh(clock.bell_index).to_string()
    } else {
        String::new()
    };
    ChronicleFactPacket {
        deed_id,
        kind: kind.to_string(),
        game_day: clock.game_days,
        solar_year,
        season_zh: season.to_string(),
        bell_zh: clock.bell_label.clone(),
        weather_zh,
        tide_zh,
        town_slug: town_slug.to_string(),
        town_zh,
        place_zh,
        agent_id: agent_id.to_string(),
        agent_public_label,
        agent_role_zh,
        patient_id: patient_id.to_string(),
        patient_public_label,
        patient_role_zh,
        forbidden_raw_names,
        witness_names: non_player_witness_names(delta, node_id, agent_id, patient_id),
        ..ChronicleFactPacket::default()
    }
}

pub(super) fn push_bounded_actor_summary(a: &mut UnifiedMindBody, summary: String) {
    if a.bio_post_chapters_zh.iter().any(|s| s == &summary) {
        return;
    }
    a.bio_post_chapters_zh.push(summary);
    if a.bio_post_chapters_zh.len() > 4 {
        let drop_n = a.bio_post_chapters_zh.len() - 4;
        a.bio_post_chapters_zh.drain(0..drop_n);
    }
    a.mark_mutated();
}

pub(super) fn cooldown_ready(
    delta: &IslandWorldDelta,
    actor_id: &str,
    kind: &str,
    today: i64,
    min_gap_days: i64,
) -> bool {
    delta
        .living_chronicle
        .committed
        .actor_cooldowns
        .get(&(actor_id.to_string(), kind.to_string()))
        .is_none_or(|&last_day| today.saturating_sub(last_day) >= min_gap_days)
}

#[expect(
    clippy::too_many_arguments,
    reason = "封裝單筆投射列欄位避免重複結構字面量"
)]
pub(super) fn make_row(
    fact: &ChronicleFactPacket,
    scope: &str,
    target_id: &str,
    latch_key: &str,
    title_zh: &str,
    subtitle_zh: &str,
    carrier_zh: &str,
    prose_zh: String,
) -> ChronicleProjectionRow {
    let era_zh = format!(
        "太陽曆{}年{}",
        solar_year_zh(fact.solar_year),
        fact.season_zh
    );
    let fact_json = serde_json::to_string(fact).unwrap_or_else(|_| "{}".to_string());
    ChronicleProjectionRow {
        deed_id: fact.deed_id.clone(),
        scope: scope.to_string(),
        target_id: target_id.to_string(),
        kind: fact.kind.clone(),
        latch_key: latch_key.to_string(),
        title_zh: title_zh.to_string(),
        subtitle_zh: subtitle_zh.to_string(),
        era_zh,
        carrier_zh: carrier_zh.to_string(),
        draft_zh: prose_zh.clone(),
        prose_zh,
        fact_json,
        solar_year: fact.solar_year,
        game_day: fact.game_day,
        status: "draft".to_string(),
        updated_seq: 0,
    }
}

/// 重新彙整指定五鎮之**單一當代近況段落**（`deed:region_live:<town_slug>`，就地覆寫絕不重複疊段！）
pub fn refresh_town_live_region_note(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    town_slug: &str,
) {
    if !matches!(
        town_slug,
        "white_gull_harbor"
            | "gull_tail_port"
            | "west_strait_ferry"
            | "greypine_post"
            | "north_cape_beacon"
    ) {
        return;
    }
    let reg_id = town_region_article_id(town_slug);
    let town_zh = town_zh_of_slug(town_slug).to_string();
    let (yr, season) = solar_year_and_season_of_day(clock.game_days);
    let solar_year = yr.max(CURRENT_SOLAR_YEAR);
    let yr_zh = solar_year_zh(solar_year);
    let td = derive_tide_clock_details(clock);
    let coastal = town_slug != "greypine_post";

    let mut bullets = Vec::new();
    let mut sentences = Vec::new();
    let mut allowed_nums = vec![yr_zh.clone()];
    let mut forbidden_raw = Vec::new();
    let mut primary_agent = String::new();

    if town_slug == "greypine_post"
        && let Some((ref slayer_id, ref raw_label, _)) = delta.first_spider_slain
    {
        let (s_lbl, f_a) = public_actor_label(delta, slayer_id);
        if let Some(f) = f_a {
            forbidden_raw.push(f);
        }
        let hero_zh = if let Some((_, rhs)) = raw_label.split_once('與') {
            if let Some(co) = delta.actors.values().find(|a| {
                a.controller_id.is_none() && !a.actor_id.starts_with("pl:") && a.display_name == rhs
            }) {
                format!("{s_lbl}與{}", co.display_name)
            } else {
                if !rhs.is_empty() && rhs != "外來旅人" {
                    forbidden_raw.push(rhs.to_string());
                }
                format!("{s_lbl}與外來旅人")
            }
        } else {
            s_lbl.clone()
        };
        if primary_agent.is_empty() {
            primary_agent = s_lbl;
        }
        allowed_nums.push("二三八".to_string());
        allowed_nums.push("九十".to_string());
        bullets.push(format!(
            "深坑開道：{hero_zh}踏過北林斷橋深入試煉坑．吞石巨蛛巢，合力擊潰盤踞九十餘年的吞石巨蛛，打通自太陽曆二三八年封坑以來阻斷的南面地脈凝石台石道"
        ));
        sentences.push(format!(
            "{hero_zh}深入北林斷橋後的試煉坑擊潰盤踞九十餘年的吞石巨蛛，讓封閉多年的南面地脈凝石台石道重新暢通，往來獵戶與石匠不必再繞遠路"
        ));
    }

    let mut town_sold: Vec<(String, String)> = delta
        .sold_houses
        .iter()
        .filter(|(nid, _)| town_slug_of_node(nid) == town_slug)
        .map(|(nid, aid)| (nid.clone(), aid.clone()))
        .collect();
    town_sold.sort_by(|a, b| a.0.cmp(&b.0));
    if !town_sold.is_empty() {
        let total_vacant = doc
            .nodes
            .iter()
            .filter(|(nid, n)| n.kind == "house_vacant" && town_slug_of_node(nid) == town_slug)
            .count();
        let cnt = town_sold.len();
        let cnt_zh = num_zh(cnt as i64);
        allowed_nums.push(cnt_zh.clone());
        let mut door_list = Vec::new();
        let mut buyer_list = Vec::new();
        for (nid, aid) in &town_sold {
            let door = doc
                .nodes
                .get(nid)
                .map_or_else(|| "巷內舊屋".to_string(), |n| clean_house_door_zh(&n.name));
            door_list.push(door);
            let (blbl, f_opt) = public_actor_label(delta, aid);
            if let Some(f) = f_opt
                && !forbidden_raw.contains(&f)
            {
                forbidden_raw.push(f);
            }
            if primary_agent.is_empty() {
                primary_agent = blbl.clone();
            }
            if !buyer_list.contains(&blbl) {
                buyer_list.push(blbl);
            }
        }
        let doors_joined = door_list.join("、");
        let buyers_joined = buyer_list.join("與");
        let bg_reason = if town_slug == "white_gull_harbor" {
            "早年因海峽關稅與南岸船疫落鎖空置的舊石屋"
        } else if town_slug == "greypine_post" {
            "早年開路石工下山後空置多年的舊石屋"
        } else {
            "巷內閒置多時的舊石木屋"
        };
        if total_vacant > 0 && cnt >= total_vacant {
            bullets.push(format!(
                "舊屋全數點燈：{town_zh}{bg_reason}（{doors_joined}，共{cnt_zh}戶）已由{buyers_joined}全數領契入住、重新點起石砌壁爐柴火"
            ));
            sentences.push(format!(
                "鎮裡{bg_reason}，包含{doors_joined}在內的{cnt_zh}處門戶已由{buyers_joined}全數搬入打掃、重新點起壁爐柴火"
            ));
        } else {
            bullets.push(format!(
                "舊屋重燃炊煙：{town_zh}{bg_reason}中的{doors_joined}（共{cnt_zh}戶），已由{buyers_joined}領契打掃積塵、重新生起石砌壁爐柴火入住"
            ));
            sentences.push(format!(
                "巷裡{bg_reason}中的{doors_joined}，也先後由{buyers_joined}搬入清出積塵，重新在石砌壁爐點起柴火安頓下來"
            ));
        }
    }

    let town_built: Vec<_> = delta
        .built_houses
        .values()
        .filter(|bh| town_slug_of_node(&bh.anchor_node_id) == town_slug)
        .collect();
    if !town_built.is_empty() {
        let b_cnt_zh = num_zh(town_built.len() as i64);
        allowed_nums.push(b_cnt_zh.clone());
        let mut builders = Vec::new();
        for bh in &town_built {
            let (blbl, f_opt) = public_actor_label(delta, &bh.owner_id);
            if let Some(f) = f_opt
                && !forbidden_raw.contains(&f)
            {
                forbidden_raw.push(f);
            }
            if !builders.contains(&blbl) && builders.len() < 3 {
                builders.push(blbl);
            }
        }
        let b_joined = builders.join("、");
        bullets.push(format!(
            "新宅起樑：{b_joined}等人在{town_zh}外圍空地立柱架樑，鎮邊已新添{b_cnt_zh}處石木新居"
        ));
        sentences.push(format!(
            "外圍空地上另有{b_joined}動手立柱架樑，新添了{b_cnt_zh}處遮風擋雨的石木新居"
        ));
    }

    let recent_defaults = delta
        .living_chronicle
        .committed
        .town_default_days
        .get(town_slug)
        .map_or(0, |s| {
            s.iter()
                .filter(|(d, _)| clock.game_days.saturating_sub(*d) <= 10)
                .count()
        });
    if recent_defaults >= 3 {
        bullets.push(format!(
            "坊間銀根吃緊：{town_zh}近期接連出現多筆銅幣木契逾期未清的催討糾紛，街坊立契交割轉趨當面點清現錢"
        ));
        sentences.push(
            "不過近期坊間也接連傳出幾樁銅幣木契逾期未清的催討糾紛，各鋪賒帳比往日謹慎許多，買賣多改為當面點清銅幣".to_string(),
        );
    }

    if bullets.is_empty() {
        return;
    }

    let draft_prose = format!(
        "到了太陽曆{yr_zh}年{season}，{town_zh}裡又添了幾樁街坊津津樂道的新變化：{}。",
        sentences.join("；")
    );
    let fact = ChronicleFactPacket {
        deed_id: format!("deed:region_live:{town_slug}"),
        kind: "region_live_summary".to_string(),
        game_day: clock.game_days,
        solar_year,
        season_zh: season.to_string(),
        bell_zh: clock.bell_label.clone(),
        weather_zh: if coastal {
            td.weather_zh.to_string()
        } else {
            inland_safe_weather_zh(td.weather_zh).to_string()
        },
        tide_zh: if coastal {
            coastal_sea_tide_zh(clock.bell_index).to_string()
        } else {
            String::new()
        },
        town_slug: town_slug.to_string(),
        town_zh,
        place_zh: town_zh_of_slug(town_slug).to_string(),
        agent_public_label: primary_agent,
        forbidden_raw_names: forbidden_raw,
        bullet_facts_zh: bullets,
        allowed_numbers_zh: allowed_nums,
        ..ChronicleFactPacket::default()
    };
    delta.living_chronicle.push_new_projection(make_row(
        &fact,
        "region",
        reg_id,
        "",
        "",
        "",
        "",
        draft_prose,
    ));
}
