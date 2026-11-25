//! 銀鷗島五鎮地景生態自洽、四族年齡行為資格硬閘與雙邊對帳 `CanonicalArcFact` 引擎（`gazetteer_v3`）。
//!
//! 嚴格落實審辯室 Room `3bd3f070` 實證裁決：
//! 1. 廢除 `harbor_history.rs` 五模板海灣迴圈與無腦「那場陣仗／沒砸過手藝招牌」複製貼上；
//! 2. 依五鎮真實地景（海港石堤／外海漁場／海峽渡輪／深山驛道／北岬燈塔）與四族年齡資格生成雙邊唯一事實帳，鎖定施受方向；
//! 3. 事件 ID 級無損合併（僅替換 `hist:micro:*` 舊罐頭前史，100% 保留執行期記憶、`TavernHearsay`、契約與 `120,805` 銅幣守恆）；
//! 4. 內建雙重機械守門員 `validate_biography_prose`（零簡體字、零文言腔、零 AI 雞湯詞、零地景穿幫、零 `pl:` 洩漏）與高水準白話列傳保底。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::combat::wordplate::splitmix64;

use super::causal_arc_catalog::build_town_ecological_arc;
pub use super::causal_arc_catalog::{CanonicalArcFact, juvenile_witness_age, solar_year_zh};
use super::dialogue_llm::validate_clean_in_world_reply;
use super::harbor_lineage::CURRENT_SOLAR_YEAR;
use super::mind::{EvidenceTier, tie_flags};
use super::persona::{ValueAxis, derive_innate_persona};
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};
use super::world_weave::apply_world_lineage_and_memory_weave;
use super::world_weave_bio::{racial_adult_age, settlement_name_zh, settlement_slug_of};

pub const BAKED_BIOGRAPHIES_V3_PATH: &str = "data/world/island_biographies_v3.json";

const WENYAN_BANNED: &[&str] = &[
    "者也",
    "乃一",
    "其人",
    "未嘗",
    "遂於",
    "遂將",
    "誠可謂",
    "嗚呼",
    "太史公曰",
];

const AI_CLICHE_BANNED: &[&str] = &[
    "風裡的光",
    "命運的齒輪",
    "歲月靜好",
    "無聲地訴說",
    "無聲的訴說",
    "人際網絡",
    "溫暖而靈通的連結",
    "溫暖的連結",
    "見證了歲月",
    "詮釋了",
    "不僅是",
    "更是她",
    "更是他",
    "在時光的長河",
    "並不孤單",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BakedBiographyFile {
    pub version: u32,
    pub biographies: BTreeMap<String, String>,
}

/// 檢驗單篇人物列傳正文是否嚴格合規（零簡體、零英文、零文言虛詞、零 AI 雞湯詞、零鎮別直接地景穿幫、必含主角與據點名）
pub fn validate_biography_prose(
    actor_name: &str,
    town_slug: &str,
    town_zh: &str,
    prose: &str,
) -> Result<(), String> {
    let trimmed = prose.trim();
    let char_cnt = trimmed.chars().count();
    if !(260..=1_400).contains(&char_cnt) {
        return Err(format!("字數越界 ({char_cnt} 字)"));
    }
    if validate_clean_in_world_reply(trimmed).is_none() {
        return Err("含簡體字、英文字母或系統禁詞".to_string());
    }
    if !trimmed.contains(actor_name) {
        return Err(format!("缺少主角全名 {actor_name}"));
    }
    if !trimmed.contains(town_zh) {
        return Err(format!("缺少所在據點名 {town_zh}"));
    }
    if trimmed.contains("（") && trimmed.chars().any(|c| c.is_ascii_digit()) {
        return Err("含括號阿拉伯數字".to_string());
    }
    if trimmed.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("- ") || t.starts_with("* ") || t.starts_with("1.") || t.starts_with("【")
    }) {
        return Err("含條列符號或小標題方括號".to_string());
    }
    for bad in WENYAN_BANNED {
        if trimmed.contains(bad) {
            return Err(format!("含文言腔禁詞：{bad}"));
        }
    }
    for bad in AI_CLICHE_BANNED {
        if trimmed.contains(bad) {
            return Err(format!("含 AI 雞湯腔禁詞：{bad}"));
        }
    }
    for bad in ["咱們", "喝西北風", "一碼歸一碼", "抽冷子"] {
        if trimmed.contains(bad) {
            return Err(format!("含非台灣正體口語禁詞：{bad}"));
        }
    }
    if town_slug == "greypine_post" {
        for coastal_bad in [
            "封凍海灣",
            "出海與入林",
            "出海捕魚",
            "自家漁船",
            "封港買不到",
            "海港濕鹹",
            "百餘年潮汐",
        ] {
            if trimmed.contains(coastal_bad) {
                return Err(format!("內陸山林驛站地景穿幫：{coastal_bad}"));
            }
        }
    }
    Ok(())
}

/// 生成全銀鷗島五鎮雙邊對帳之 `CanonicalArcFact` 總表（純函數，零隨機漂移、零經濟變動）
pub fn generate_island_canonical_arcs(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
) -> (
    Vec<CanonicalArcFact>,
    BTreeMap<String, Vec<CanonicalArcFact>>,
) {
    let mut by_town_heads: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut hh_seen = BTreeSet::new();
    for a in delta.actors.values() {
        if a.controller_id.is_some()
            || a.actor_id.starts_with("pl:")
            || !a.alive
            || a.household_id.is_empty()
        {
            continue;
        }
        if hh_seen.insert(a.household_id.clone()) {
            let slug = settlement_slug_of(&a.household_id).to_string();
            by_town_heads
                .entry(slug)
                .or_default()
                .push(a.actor_id.clone());
        }
    }

    let mut all_arcs = Vec::new();
    let mut arcs_by_actor: BTreeMap<String, Vec<CanonicalArcFact>> = BTreeMap::new();

    for (town_slug, heads) in &by_town_heads {
        let n = heads.len();
        if n < 2 {
            continue;
        }
        for i in 0..n {
            for slot in 0..2usize {
                let step = if slot == 0 { 1 } else { (n / 3).max(2) };
                let j = (i + step) % n;
                if i == j {
                    continue;
                }
                let (Some(ha), Some(hb)) =
                    (delta.actors.get(&heads[i]), delta.actors.get(&heads[j]))
                else {
                    continue;
                };
                let min_adult_a = ha.birth_year + racial_adult_age(&ha.race);
                let min_adult_b = hb.birth_year + racial_adult_age(&hb.race);
                let earliest_valid = min_adult_a.max(min_adult_b).max(304);
                let latest_valid = CURRENT_SOLAR_YEAR - 2;
                let pair_seed = splitmix64(
                    ha.soul_seed
                        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                        .wrapping_add(hb.soul_seed)
                        .wrapping_add(slot as u64 * 131),
                );
                let ev_year = if earliest_valid <= latest_valid {
                    earliest_valid
                        + (pair_seed % ((latest_valid - earliest_valid + 1) as u64)) as i16
                } else {
                    CURRENT_SOLAR_YEAR - 1
                };
                let town_zh = settlement_name_zh(doc, ha);
                let arc_id = format!("arc:v3:{town_slug}:{i}:{j}:{slot}");
                let arc = build_town_ecological_arc(
                    arc_id,
                    town_slug,
                    &town_zh,
                    ev_year,
                    pair_seed >> 8,
                    &ha.actor_id,
                    &ha.display_name,
                    &ha.role,
                    &hb.actor_id,
                    &hb.display_name,
                    &hb.role,
                    &hb.bound_weapon.name,
                );
                arcs_by_actor
                    .entry(ha.actor_id.clone())
                    .or_default()
                    .push(arc.clone());
                arcs_by_actor
                    .entry(hb.actor_id.clone())
                    .or_default()
                    .push(arc.clone());
                all_arcs.push(arc);
            }
        }
    }

    // 為同屋非戶長成員同步掛入本戶最具代表性之 1~2 樁本末（依年齡區分親歷與幼年耳聞）
    let mut hh_head_map: BTreeMap<String, String> = BTreeMap::new();
    for heads in by_town_heads.values() {
        for hid in heads {
            if let Some(ha) = delta.actors.get(hid) {
                hh_head_map.insert(ha.household_id.clone(), hid.clone());
            }
        }
    }
    for a in delta.actors.values() {
        if a.controller_id.is_some()
            || a.actor_id.starts_with("pl:")
            || !a.alive
            || arcs_by_actor.contains_key(&a.actor_id)
        {
            continue;
        }
        if let Some(hid) = hh_head_map.get(&a.household_id)
            && let Some(h_arcs) = arcs_by_actor.get(hid).cloned()
        {
            arcs_by_actor.insert(a.actor_id.clone(), h_arcs.into_iter().take(2).collect());
        }
    }

    (all_arcs, arcs_by_actor)
}

/// 高水準確定性現代台灣正體白話《史記》列傳合成器（4 種開篇切入鏡頭 × 雙邊事實帳 × 今日人際收束，零罐頭接尾句）
pub fn render_deterministic_shiji_bio(
    doc: &WorldStaticDoc,
    a: &UnifiedMindBody,
    actor_arcs: &[CanonicalArcFact],
) -> String {
    let town_slug = settlement_slug_of(&a.household_id);
    let town_zh = settlement_name_zh(doc, a);
    let active_home = a.owned_house_node.as_deref().unwrap_or(&a.home_node);
    let raw_door_zh = doc
        .nodes
        .get(active_home)
        .map_or_else(|| town_zh.clone(), |n| n.name.replace('・', "．"));
    let door_zh: String = raw_door_zh
        .chars()
        .map(|c| {
            c.to_digit(10)
                .and_then(|d| {
                    ['〇', '一', '二', '三', '四', '五', '六', '七', '八', '九']
                        .get(d as usize)
                        .copied()
                })
                .unwrap_or(c)
        })
        .collect();
    let birth_zh = solar_year_zh(a.birth_year);
    let innate = derive_innate_persona(a.soul_seed.max(1));
    let top_ax = innate.top_value_axis(&a.drift);

    let ethos_clause = match top_ax {
        ValueAxis::CraftPride => {
            "平素把自家手藝招牌看得比銅板還重，應下的活計寧可貼料也絕不草率交差"
        }
        ValueAxis::PactIntegrity => "為人極重口頭然諾，當面應承的事就算頂著風雪也會辦妥",
        ValueAxis::TangibleGain => "持家算帳向來分明，人情歸人情、工本數目從不含糊",
        ValueAxis::Autonomy => "性子硬朗主見極深，吃軟不吃硬，最忌旁人對自家門戶指手畫腳",
        ValueAxis::KinshipLoyalty => "極為護短念舊，凡涉及屋簷下自家人與老親故的安危總是第一個出頭",
        ValueAxis::OrderHierarchy => "篤信街巷行規與長幼分寸，看不慣壞了老規矩的毛躁舉動",
        ValueAxis::CuriosityVenture => {
            "對山海異聞與外邦來貨總存著幾分留心，閒暇愛向過路行商打探風向"
        }
        ValueAxis::SurvivalSafety => "在風浪與荒氣邊緣討生活，凡事講究先留退路、穩妥持家",
    };

    let clean_app = a
        .appearance
        .split('（')
        .next()
        .unwrap_or(&a.appearance)
        .trim_end_matches('。');
    let clean_prov = a.bound_weapon.provenance.trim_end_matches('。');
    let lens = (splitmix64(a.soul_seed ^ 0x7719) % 4) as usize;
    let p1 = match lens {
        0 => format!(
            "走進{door_zh}，常能看見{name}操持{role}的身影。這位太陽曆{birth_zh}年出生的{race}{bond}，{clean_app}；手邊常年帶著那件{wname}，{clean_prov}。在{town_zh}街坊眼裡，{name}{ethos_clause}。",
            name = a.display_name,
            role = a.role,
            race = a.race,
            bond = a.household_bond,
            wname = a.bound_weapon.name,
        ),
        1 => format!(
            "{name}手裡那件{wname}，{clean_prov}，平日總隨身帶在{door_zh}的屋簷內外。{name}生於太陽曆{birth_zh}年，身為屋裡的{bond}，日常以{role}營生，{clean_app}；{ethos_clause}，在{town_zh}一帶算得上知根知底的熟面孔。",
            name = a.display_name,
            wname = a.bound_weapon.name,
            bond = a.household_bond,
            role = a.role,
        ),
        2 => format!(
            "在{town_zh}提起{door_zh}的{name}，熟識的鄰里多半先想到那件不離身的{wname}，{clean_prov}。{name}於太陽曆{birth_zh}年出生，{clean_app}，在門戶裡擔著{bond}與{role}的擔子，{ethos_clause}。",
            name = a.display_name,
            wname = a.bound_weapon.name,
            bond = a.household_bond,
            role = a.role,
        ),
        _ => format!(
            "{name}是{door_zh}的{bond}，太陽曆{birth_zh}年生，身為{race}{role}，{clean_app}。平日幹活總握著那把{wname}，{clean_prov}；在{town_zh}過日子，{ethos_clause}。",
            name = a.display_name,
            bond = a.household_bond,
            race = a.race,
            role = a.role,
            wname = a.bound_weapon.name,
        ),
    };

    let mut sorted_arcs: Vec<&CanonicalArcFact> = actor_arcs.iter().collect();
    sorted_arcs.sort_by_key(|arc| arc.solar_year);
    let mut arc_sentences = Vec::new();
    for arc in sorted_arcs.into_iter().take(2) {
        let age_then = arc.solar_year - a.birth_year;
        let is_actor_involved = a.actor_id == arc.benefactor_id || a.actor_id == arc.recipient_id;
        if age_then < juvenile_witness_age(&a.race) && !is_actor_involved {
            arc_sentences.push(format!(
                "早年{name}年紀尚幼時，家門曾遇上一樁往事：{setup}，當時{climax}，留下一句「{quote}」，長輩日後在飯桌上每每提起，總教晚輩莫忘那段前塵。",
                name = a.display_name,
                setup = arc.crisis_setup_zh,
                climax = arc.action_climax_zh,
                quote = arc.spoken_line_zh,
            ));
        } else {
            arc_sentences.push(format!(
                "{setup}；緊要關頭，{climax}，當場留下一句「{quote}」，{mark}。",
                setup = arc.crisis_setup_zh,
                climax = arc.action_climax_zh,
                quote = arc.spoken_line_zh,
                mark = arc.lasting_mark_zh,
            ));
        }
    }
    if arc_sentences.is_empty() {
        for ep in a
            .mind
            .ltm
            .episodic
            .iter()
            .filter(|e| e.tier != EvidenceTier::TavernHearsay)
            .take(2)
        {
            arc_sentences.push(ep.subjective_zh.trim_end_matches('。').to_string() + "。");
        }
    }
    if town_slug == "white_gull_harbor" {
        if a.race == "矮人" && a.birth_year <= 220 {
            arc_sentences.insert(
                0,
                format!(
                    "早在太陽曆二三八年北林老礦坑鑿穿荒氣地隙驚出吞石巨蛛之際，正值壯年的{}曾隨族中長老打入重鐵楔封死灰松斷橋暗門，背著精鐵遷入白鷗港合築北段石牆。",
                    a.display_name
                ),
            );
            arc_sentences.truncate(2);
        } else if a.race == "精靈" && a.birth_year <= 115 {
            arc_sentences.insert(
                0,
                format!(
                    "{}年歲悠長，曾親眼見過白鷗港自早年木柵小埠歷經拋石築堤與關稅洗牌一步步長成今日百戶海港的模樣。",
                    a.display_name
                ),
            );
            arc_sentences.truncate(2);
        }
    }
    let p2 = arc_sentences.join("其後歲月流轉，");

    let mut warm_ties: Vec<_> = a
        .mind
        .ltm
        .social
        .values()
        .filter(|s| !s.target_id.starts_with("pl:") && s.warmth >= 30)
        .collect();
    warm_ties.sort_by_key(|s| std::cmp::Reverse((s.warmth, s.familiarity)));

    let mut rift_ties: Vec<_> = a
        .mind
        .ltm
        .social
        .values()
        .filter(|s| !s.target_id.starts_with("pl:") && s.warmth <= -16)
        .collect();
    rift_ties.sort_by_key(|s| s.warmth);

    let cross_tie = a.mind.ltm.social.values().find(|s| {
        !s.target_id.starts_with("pl:")
            && !s.target_region_zh.is_empty()
            && s.target_region_zh != town_zh
            && (s.tie_mask & (tie_flags::BLOOD_KIN | tie_flags::IN_LAW | tie_flags::SWORN_COMRADE))
                != 0
    });

    let mut p3_parts = Vec::new();
    if let Some(w) = warm_ties.first() {
        let lbl = if w.tie_label_zh.is_empty() {
            "故舊"
        } else {
            w.tie_label_zh.as_str()
        };
        let reg = if !w.target_region_zh.is_empty() && w.target_region_zh != town_zh {
            format!("身在{}的", w.target_region_zh)
        } else if lbl.starts_with("同鎮") || lbl.starts_with("同屋") || lbl.starts_with("同鋪")
        {
            String::new()
        } else {
            "同鎮的".to_string()
        };
        p3_parts.push(format!(
            "如今{}在日常生計之外，與{}{}{}最為親厚信賴，遇上緊要事總能彼此照應",
            a.display_name, reg, lbl, w.target_name
        ));
    } else {
        p3_parts.push(format!(
            "如今{}守著{}的日常活計，與街坊往來進退有度",
            a.display_name, door_zh
        ));
    }
    if let Some(r) = rift_ties.first() {
        let lbl = if r.tie_label_zh.is_empty() {
            "舊識"
        } else {
            r.tie_label_zh.as_str()
        };
        p3_parts.push(format!(
            "唯獨與{}{}因往年舊隙心頭還隔著一層，碰面多半點頭即止",
            lbl, r.target_name
        ));
    }
    if let Some(ct) = cross_tie
        && warm_ties
            .first()
            .is_none_or(|w| w.target_id != ct.target_id)
    {
        p3_parts.push(format!(
            "另有{}的{}時常託往返車船捎信通氣，山海消息常相往還",
            ct.target_region_zh, ct.target_name
        ));
    }
    let p3 = p3_parts.join("；") + "。";

    format!("{p1}\n\n{p2}\n\n{p3}")
}

pub fn load_baked_biographies_map() -> BTreeMap<String, String> {
    let path = Path::new(BAKED_BIOGRAPHIES_V3_PATH);
    let Ok(raw) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    serde_json::from_str::<BakedBiographyFile>(&raw)
        .map(|f| f.biographies)
        .unwrap_or_default()
}

/// 執行 `chronicle_gazetteer_v3` 事件 ID 級無損升級：
/// 1. 清理舊罐頭 `hist:micro:*` 與重複口號，保留執行期記憶、`TavernHearsay`、契約與 `120,805` 銅幣；
/// 2. 先寫入雙邊對帳 `arc:v3:*` 記憶與社交關係，再執行 `apply_world_lineage_and_memory_weave` 讓三角閉合一次收斂至不動點（確保連續重跑 100% 冪等）；
/// 3. 將經驗證之成篇白話列傳寫入 `bio_core_zh` 並同步 `personal_bio_zh`。
pub fn apply_gazetteer_v3_upgrade(doc: &WorldStaticDoc, delta: &mut IslandWorldDelta) -> usize {
    let needs_backfill = delta.actors.values().any(|a| {
        a.controller_id.is_none() && a.alive && !a.household_id.is_empty() && a.birth_year == 0
    });
    if needs_backfill {
        let _ = super::harbor_history::apply_island_history_backfill(doc, delta);
    }
    let (all_arcs, arcs_by_actor) = generate_island_canonical_arcs(doc, delta);
    let baked = load_baked_biographies_map();

    let mut hh_members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for a in delta.actors.values_mut() {
        if a.controller_id.is_some() || a.actor_id.starts_with("pl:") || !a.alive {
            continue;
        }
        if a.household_id.starts_with("hh:greypine_post:") {
            a.appearance = a
                .appearance
                .replace("海港濕鹹泥印", "山徑濕冷松泥")
                .replace("百餘年潮汐", "百餘年山嵐雪線");
        }
        a.appearance = a.appearance.replace("乾淨利落", "乾淨俐落");
        if !a.household_id.is_empty() {
            hh_members
                .entry(a.household_id.clone())
                .or_default()
                .push(a.actor_id.clone());
        }
        a.mind.ltm.episodic.retain(|ep| {
            !ep.event_id.starts_with("hist:micro:") && !ep.subjective_zh.contains("那場陣仗")
        });
        for soc in a.mind.ltm.social.values_mut() {
            soc.folded_events
                .retain(|ev| !ev.starts_with("hist:micro:"));
        }
        a.mind.ltm.social.retain(|_, soc| {
            !(soc.from_genesis
                && !soc.cohabiting
                && soc.tie_mask == 0
                && soc.folded_events.is_empty())
        });
        a.mind.ltm.turning_points.retain(|tp| {
            !tp.anchor_event_id.starts_with("rust_slow")
                && !tp.lesson_zh.contains("手藝不能減分")
                && !tp.lesson_zh.contains("生計連日受阻（手頭剩")
        });
    }

    for arc in &all_arcs {
        let origin_day = i64::from(arc.solar_year - 312) * 364;
        let b_town = delta
            .actors
            .get(&arc.recipient_id)
            .map(|x| settlement_name_zh(doc, x))
            .unwrap_or_default();
        let a_town = delta
            .actors
            .get(&arc.benefactor_id)
            .map(|x| settlement_name_zh(doc, x))
            .unwrap_or_default();
        let (a_hh, a_head_by) = delta
            .actors
            .get(&arc.benefactor_id)
            .map(|x| (x.household_id.clone(), x.birth_year))
            .unwrap_or_default();
        let (b_hh, b_head_by) = delta
            .actors
            .get(&arc.recipient_id)
            .map(|x| (x.household_id.clone(), x.birth_year))
            .unwrap_or_default();

        if let Some(mems_a) = hh_members.get(&a_hh) {
            for aid in mems_a {
                let Some(act_m) = delta.actors.get_mut(aid) else {
                    continue;
                };
                if act_m.controller_id.is_some() {
                    continue;
                }
                let is_head = aid == &arc.benefactor_id;
                let old_enough =
                    arc.solar_year - act_m.birth_year >= juvenile_witness_age(&act_m.race);
                let (tier, cred, fid) = if is_head || old_enough {
                    (EvidenceTier::DirectWitness, 94, 92)
                } else {
                    (EvidenceTier::FamilyOral, 88, 68)
                };
                let head_ref = if act_m.birth_year < a_head_by {
                    "家裡當家"
                } else if act_m.household_bond == "同堂伴侶" {
                    "同堂伴侶"
                } else {
                    "家裡長輩"
                };
                let text = if is_head {
                    arc.ep_benefactor_zh.clone()
                } else if arc.warmth_delta < 0 {
                    format!(
                        "太陽曆 {} 年前後，{}{}與同鎮{}家合辦活計分帳生隙：{}",
                        arc.solar_year,
                        head_ref,
                        arc.benefactor_name,
                        arc.recipient_name,
                        arc.lasting_mark_zh
                    )
                } else {
                    format!(
                        "太陽曆 {} 年前後，{}{}曾出手相助同鎮{}家：{}",
                        arc.solar_year,
                        head_ref,
                        arc.benefactor_name,
                        arc.recipient_name,
                        arc.lasting_mark_zh
                    )
                };
                act_m.mind.ingest_historical_episode(
                    &arc.arc_id,
                    origin_day,
                    &act_m.home_node.clone(),
                    &arc.recipient_id,
                    &arc.recipient_name,
                    tier,
                    cred,
                    fid,
                    82,
                    6,
                    text,
                );
                if is_head {
                    let _ = act_m.mind.fold_social_memory_from_episode(
                        &arc.arc_id,
                        origin_day,
                        &arc.recipient_id,
                        &arc.recipient_name,
                        &b_town,
                        tie_flags::PEER_COHORT,
                        "同鎮街坊",
                        true,
                        false,
                        68,
                        arc.warmth_delta,
                        &arc.note_benefactor_zh,
                        "",
                    );
                }
            }
        }

        if let Some(mems_b) = hh_members.get(&b_hh) {
            for bid in mems_b {
                let Some(act_m) = delta.actors.get_mut(bid) else {
                    continue;
                };
                if act_m.controller_id.is_some() {
                    continue;
                }
                let is_head = bid == &arc.recipient_id;
                if is_head && let Some(ref prov) = arc.vessel_provenance_for_recipient {
                    act_m.bound_weapon.provenance = prov.clone();
                }
                let old_enough =
                    arc.solar_year - act_m.birth_year >= juvenile_witness_age(&act_m.race);
                let (tier, cred, fid) = if is_head || old_enough {
                    (EvidenceTier::DirectWitness, 94, 92)
                } else {
                    (EvidenceTier::FamilyOral, 88, 68)
                };
                let head_ref = if act_m.birth_year < b_head_by {
                    "家裡當家"
                } else if act_m.household_bond == "同堂伴侶" {
                    "同堂伴侶"
                } else {
                    "家裡長輩"
                };
                let text = if is_head {
                    arc.ep_recipient_zh.clone()
                } else if arc.warmth_delta < 0 {
                    format!(
                        "太陽曆 {} 年前後，{}{}與同鎮{}家合辦活計結下芥蒂：{}",
                        arc.solar_year,
                        head_ref,
                        arc.recipient_name,
                        arc.benefactor_name,
                        arc.lasting_mark_zh
                    )
                } else {
                    format!(
                        "太陽曆 {} 年前後，家裡曾承同鎮{}家一樁重情：{}",
                        arc.solar_year, arc.benefactor_name, arc.lasting_mark_zh
                    )
                };
                act_m.mind.ingest_historical_episode(
                    &arc.arc_id,
                    origin_day,
                    &act_m.home_node.clone(),
                    &arc.benefactor_id,
                    &arc.benefactor_name,
                    tier,
                    cred,
                    fid,
                    82,
                    6,
                    text,
                );
                if is_head {
                    let _ = act_m.mind.fold_social_memory_from_episode(
                        &arc.arc_id,
                        origin_day,
                        &arc.benefactor_id,
                        &arc.benefactor_name,
                        &a_town,
                        tie_flags::PEER_COHORT,
                        "同鎮街坊",
                        true,
                        false,
                        68,
                        arc.warmth_delta,
                        &arc.note_recipient_zh,
                        "",
                    );
                }
            }
        }
    }

    // 在 `arc:v3:*` 注入後執行全島譜系與二跳三角閉合，使其一次收斂至不動點
    let _ = apply_world_lineage_and_memory_weave(doc, delta);

    let mut upgraded = 0usize;
    let aids: Vec<String> = delta.actors.keys().cloned().collect();
    for aid in aids {
        let Some(ro) = delta.actors.get(&aid) else {
            continue;
        };
        if ro.controller_id.is_some()
            || aid.starts_with("pl:")
            || !ro.alive
            || ro.household_id.is_empty()
        {
            continue;
        }
        let t_slug = settlement_slug_of(&ro.household_id).to_string();
        let t_zh = settlement_name_zh(doc, ro);
        let my_arcs = arcs_by_actor.get(&aid).map_or(&[][..], |v| v.as_slice());

        let chosen_core = baked
            .get(&aid)
            .filter(|txt| validate_biography_prose(&ro.display_name, &t_slug, &t_zh, txt).is_ok())
            .cloned()
            .unwrap_or_else(|| render_deterministic_shiji_bio(doc, ro, my_arcs));

        if let Some(act) = delta.actors.get_mut(&aid) {
            act.bio_core_zh = chosen_core;
            let _ = super::world_weave_bio::refresh_actor_biography_if_needed(
                doc,
                act,
                super::clock_and_lang::WORLD_OPEN_DAY,
            );
            if act.personal_bio_zh.is_empty() {
                act.personal_bio_zh = act.bio_core_zh.clone();
            }
            act.rev = act.rev.saturating_add(1);
            act.dirty = true;
            upgraded += 1;
        }
    }

    upgraded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::outer_island::chronicle_export::{
        build_chronicle_actor_json, build_chronicle_index_json,
    };
    use crate::grid::outer_island::engine::load_static_world_doc;
    use crate::grid::outer_island::gazetteer::build_in_world_scroll_bundle_json;
    use crate::grid::outer_island::sim_tick::seed_genesis_actors_once;

    #[test]
    fn gazetteer_v3_biographies_and_three_volumes_pass_all_literary_and_ecology_gates() {
        let doc = load_static_world_doc();
        let mut delta = IslandWorldDelta {
            genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
            municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
            silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
            ..IslandWorldDelta::default()
        };
        seed_genesis_actors_once(&doc, &mut delta, false);
        assert_eq!(delta.conservation_diff(&doc), 0);

        // 1. 全島 674 人零罐頭口號、零內陸穿幫、零幼童親歷穿幫，且 100% 通過文學守門員
        let mut checked = 0usize;
        for a in delta.actors.values() {
            if a.controller_id.is_some() || !a.alive || a.household_id.is_empty() {
                continue;
            }
            checked += 1;
            let slug = settlement_slug_of(&a.household_id);
            let tzh = settlement_name_zh(&doc, a);
            validate_biography_prose(&a.display_name, slug, &tzh, &a.personal_bio_zh)
                .unwrap_or_else(|e| panic!("bio gate failed for {}: {e}", a.actor_id));
            assert!(!a.personal_bio_zh.contains("那場陣仗"));
            assert!(!a.personal_bio_zh.contains("沒砸過自家手藝招牌"));
            for ep in &a.mind.ltm.episodic {
                assert!(!ep.subjective_zh.contains("那場陣仗"));
                assert!(!ep.subjective_zh.contains("出海與入林"));
            }
        }
        assert_eq!(checked, doc.residents.len());

        // 2. 點名檢驗灰松林驛「止血黛安」：純文學書卷輸出零除錯數值、零海灣穿幫
        let diane_json =
            build_chronicle_actor_json(&doc, &delta, "res:greypine_post:moss_well_ln_1_0")
                .expect("diane chronicle exists");
        let raw_diane = diane_json.to_string();
        for forbidden_debug in [
            "credence", "fidelity", "fatigue", "hunger", "信 95", "真 92",
        ] {
            assert!(
                !raw_diane.contains(forbidden_debug),
                "leaked debug field in /chronicle: {forbidden_debug}"
            );
        }

        // 3. 驗證三卷全誌總目與遊戲內石碑／兵燹抄本零私密列傳洩漏
        let idx = build_chronicle_index_json(&doc, &delta);
        assert_eq!(
            idx["total_residents"].as_u64(),
            Some(doc.residents.len() as u64)
        );
        assert_eq!(idx["total_regions"].as_u64(), Some(7));
        assert_eq!(idx["total_annals"].as_u64(), Some(12));

        let stele = build_in_world_scroll_bundle_json(&doc, &delta, "greypine_post", "stele", None);
        assert_eq!(stele["ok"].as_bool(), Some(true));
        assert!(!stele.to_string().contains("\"kind\":\"actor\""));
    }
}
