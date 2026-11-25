//! 三卷一體「同源雙出」全誌匯出器：自同一份權威狀態同時驅動 `/chronicle` 純文學書卷閱讀器與超連結 Markdown 全誌集。
//!
//! 嚴守使用者拍板與審辯室 Room `3bd3f070` 裁決：
//! - `/chronicle` 100% 純文學書卷（零 `信95/真92`、零疲勞飢餓銅幣存糧數值、零心事數值、零條列記憶流水帳；除錯數值全留 `/world_atlas`）；
//! - 涵蓋卷一〈方輿志〉、卷二〈紀事本末〉（戰役與歷史風變長卷）與卷三〈人物列傳〉（674 名常住居民現代白話列傳）；
//! - 採用以本人 `ltm.social` 綁定 ID 為準之精確超連結映射表（`linked_refs`），杜絕跨鎮同名誤連，並附一鍵複製純淨入樂母本全文。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{Value, json};

use super::causal_arcs::{render_deterministic_shiji_bio, solar_year_zh};
use super::clock_and_lang::{current_solar_clock, solar_year_and_season_of_day};
use super::gazetteer::{
    ChronicleLinkRef, REGIONAL_GAZETTEERS, build_gazetteer_article_json, town_region_article_id,
};
use super::gazetteer_wars::all_historical_annals;
use super::harbor_lineage::CURRENT_SOLAR_YEAR;
use super::mind::tie_flags;
use super::slow_think::mask_player_names_in_text;
use super::surface_cues::render_concern_inner_thought_zh;
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};
use super::world_weave_bio::{bare_given_name, settlement_name_zh, settlement_slug_of};

pub const ESTRANGED_WARMTH_MAX: i8 = -18;

pub const SETTLEMENT_BOOKS: &[(&str, &str, &str)] = &[
    ("white_gull_harbor", "白鷗港", "03_白鷗港人物列傳.md"),
    ("gull_tail_port", "鷗尾漁港", "04_鷗尾漁港人物列傳.md"),
    ("west_strait_ferry", "西峽渡口", "05_西峽渡口人物列傳.md"),
    ("greypine_post", "灰松林驛", "06_灰松林驛人物列傳.md"),
    ("north_cape_beacon", "海風北塢", "07_海風北塢人物列傳.md"),
];

fn book_file_for_slug(slug: &str) -> &'static str {
    SETTLEMENT_BOOKS
        .iter()
        .find(|(s, _, _)| *s == slug)
        .map_or("03_白鷗港人物列傳.md", |(_, _, f)| *f)
}

fn anchor_slug(actor_id: &str) -> String {
    actor_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn cjk_clean(s: &str) -> String {
    s.replace("——", "，")
}

fn zh_house_number(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            let mut num_str = String::from(c);
            while let Some(&nc) = chars.peek() {
                if nc.is_ascii_digit() {
                    num_str.push(nc);
                    let _ = chars.next();
                } else {
                    break;
                }
            }
            if let Ok(n) = num_str.parse::<u32>() {
                let digits = ["〇", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
                let zh = match n {
                    0..=9 => digits[n as usize].to_string(),
                    10 => "十".to_string(),
                    11..=19 => format!("十{}", digits[(n % 10) as usize]),
                    20..=99 if n % 10 == 0 => format!("{}十", digits[(n / 10) as usize]),
                    20..=99 => format!(
                        "{}十{}",
                        digits[(n / 10) as usize],
                        digits[(n % 10) as usize]
                    ),
                    _ => solar_year_zh(n as i16),
                };
                out.push_str(&zh);
            } else {
                out.push_str(&num_str);
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn zone_label_of(doc: &WorldStaticDoc, actor: &UnifiedMindBody) -> String {
    doc.nodes
        .get(&actor.home_node)
        .map(|n| {
            let parts: Vec<&str> = n.name.split('・').collect();
            let seg = if parts.len() >= 2
                && matches!(
                    parts[0],
                    "白鷗港" | "鷗尾漁港" | "西峽渡口" | "灰松林驛" | "海風北塢"
                ) {
                parts[1]
            } else {
                parts[0]
            };
            if let Some(without_hao) = seg.strip_suffix('號') {
                let street = without_hao.trim_end_matches(|c: char| c.is_ascii_digit());
                if !street.is_empty() {
                    return street.to_string();
                }
            }
            seg.to_string()
        })
        .unwrap_or_else(|| "街區".to_string())
}

fn door_label_of(doc: &WorldStaticDoc, actor: &UnifiedMindBody) -> String {
    let home = actor
        .owned_house_node
        .as_deref()
        .unwrap_or(&actor.home_node);
    doc.nodes.get(home).map_or_else(
        || home.to_string(),
        |n| zh_house_number(&n.name.replace('・', "．")),
    )
}

fn tie_flags_summary_zh(mask: u8) -> &'static str {
    if (mask & tie_flags::BLOOD_KIN) != 0 && (mask & tie_flags::MASTER_DISCIPLE) != 0 {
        "血親兼師門"
    } else if (mask & tie_flags::IN_LAW) != 0 && (mask & tie_flags::MASTER_DISCIPLE) != 0 {
        "姻親兼師門"
    } else if (mask & tie_flags::BLOOD_KIN) != 0 {
        "血親"
    } else if (mask & tie_flags::IN_LAW) != 0 {
        "姻親"
    } else if (mask & tie_flags::SWORN_COMRADE) != 0 {
        "患難至交"
    } else if (mask & tie_flags::MASTER_DISCIPLE) != 0 {
        "師門同行"
    } else if (mask & tie_flags::PEER_COHORT) != 0 {
        "同輩故舊"
    } else if (mask & tie_flags::TRADE_BROKER) != 0 {
        "商路往來"
    } else {
        "街坊舊識"
    }
}

/// 組裝角色純文學段落（優先使用已驗證之 `bio_core_zh` 前傳基座，接續 `living_chronicler` 動態生平篇章，若當下存有掛念則收束於段尾並遮罩玩家真名）
fn build_actor_literary_paragraphs(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
) -> Vec<String> {
    let raw_core = if !a.bio_core_zh.trim().is_empty() {
        a.bio_core_zh.as_str()
    } else if let Some((core, _)) = a.personal_bio_zh.split_once("\n\n近歲新篇：") {
        core
    } else {
        a.personal_bio_zh.as_str()
    };
    let base_bio = if raw_core.trim().chars().count() >= 180 {
        raw_core.to_string()
    } else {
        render_deterministic_shiji_bio(doc, a, &[])
    };

    let mut paras: Vec<String> = base_bio
        .split("\n\n")
        .map(|p| cjk_clean(&mask_player_names_in_text(delta, p.trim())))
        .filter(|p| !p.is_empty())
        .collect();

    for live_para in delta.living_chronicle.actor_paragraphs(&a.actor_id) {
        let cleaned = cjk_clean(&mask_player_names_in_text(delta, live_para.trim()));
        if !cleaned.is_empty() && !paras.iter().any(|p| p == &cleaned) {
            paras.push(cleaned);
        }
    }

    if let Some(top_c) = a.mind.top_concern() {
        let raw_thought = render_concern_inner_thought_zh(delta, top_c, true);
        let masked_thought = cjk_clean(&mask_player_names_in_text(delta, raw_thought.trim()));
        if !masked_thought.is_empty() && !paras.iter().any(|p| p.contains(&masked_thought)) {
            let clean_tail = masked_thought.trim_end_matches('。');
            if let Some(last) = paras.last_mut() {
                last.push_str(&format!("近來眉宇間亦藏著一樁心事：{clean_tail}。"));
            } else {
                paras.push(format!("{clean_tail}。"));
            }
        }
    }

    paras
}

/// 建立該角色專屬、無跨鎮同名誤連之 ID 錨定超連結表（按詞長降序排列）
fn build_actor_precise_links(
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
) -> Vec<ChronicleLinkRef> {
    let mut links: Vec<ChronicleLinkRef> = Vec::new();
    let mut seen_surfaces = BTreeSet::new();

    // 1. 優先綁定本人 `ltm.social` 中確切往來之非玩家人物
    let my_bare = bare_given_name(&a.display_name);
    let mut bare_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for soc in a.mind.ltm.social.values() {
        if soc.target_id.starts_with("pl:")
            || delta
                .actors
                .get(&soc.target_id)
                .is_none_or(|ta| ta.controller_id.is_some() || !ta.alive)
        {
            continue;
        }
        let bg = bare_given_name(&soc.target_name);
        if bg.chars().count() >= 2 && bg != my_bare {
            *bare_counts.entry(bg).or_default() += 1;
        }
    }

    for soc in a.mind.ltm.social.values() {
        if soc.target_id.starts_with("pl:")
            || delta
                .actors
                .get(&soc.target_id)
                .is_none_or(|ta| ta.controller_id.is_some() || !ta.alive)
        {
            continue;
        }
        if soc.target_name != a.display_name && seen_surfaces.insert(soc.target_name.clone()) {
            links.push(ChronicleLinkRef {
                surface: soc.target_name.clone(),
                kind: "actor",
                target_id: soc.target_id.clone(),
            });
        }
        let bg = bare_given_name(&soc.target_name);
        if bg != soc.target_name
            && bg.chars().count() >= 2
            && !a.display_name.contains(bg)
            && bare_counts.get(bg).copied() == Some(1)
            && seen_surfaces.insert(bg.to_string())
        {
            links.push(ChronicleLinkRef {
                surface: bg.to_string(),
                kind: "actor",
                target_id: soc.target_id.clone(),
            });
        }
    }

    // 2. 五鎮與主陸〈方輿志〉及關鍵〈紀事本末〉超連結
    let world_anchors: &[(&str, &str, &str)] = &[
        ("白鷗港", "region", "reg:white_gull_harbor"),
        ("鷗尾漁港", "region", "reg:gull_tail_port"),
        ("西峽渡口", "region", "reg:west_strait_ferry"),
        ("灰松林驛", "region", "reg:greypine_post"),
        ("海風北塢", "region", "reg:north_cape_beacon"),
        ("銀鷗島", "region", "reg:silver_gull_realm"),
        ("銀鷗海領", "region", "reg:silver_gull_realm"),
        ("蒼淵主陸", "region", "reg:cangyuan_mainland"),
        ("羅泉王國", "region", "reg:cangyuan_mainland"),
        ("北林老礦坑", "annals", "ann:epoch_238_mine_seal"),
        ("北林深坑", "annals", "ann:epoch_238_mine_seal"),
        ("北林封坑", "annals", "ann:epoch_238_mine_seal"),
        ("灰松斷橋", "annals", "ann:epoch_238_mine_seal"),
        ("防波石堤", "annals", "ann:epoch_179_seawall"),
        ("東西碎石驛道", "annals", "ann:epoch_274_greypine_road"),
        ("碎石驛道", "annals", "ann:epoch_274_greypine_road"),
        ("二成防禦關稅", "annals", "ann:epoch_296_strait_tariff"),
        ("二成關稅", "annals", "ann:epoch_296_strait_tariff"),
        ("古加津", "annals", "ann:epoch_326_plague_ship"),
        ("南岸船疫", "annals", "ann:epoch_326_plague_ship"),
    ];
    for &(surf, kind, tid) in world_anchors {
        if seen_surfaces.insert(surf.to_string()) {
            links.push(ChronicleLinkRef {
                surface: surf.to_string(),
                kind,
                target_id: tid.to_string(),
            });
        }
    }

    for dyn_ann in delta.living_chronicle.dynamic_annals_sorted() {
        if !dyn_ann.title_zh.is_empty() && seen_surfaces.insert(dyn_ann.title_zh.clone()) {
            links.push(ChronicleLinkRef {
                surface: dyn_ann.title_zh.clone(),
                kind: "annals",
                target_id: dyn_ann.target_id.clone(),
            });
        }
        if dyn_ann.target_id == "ann:live:north_mine_spider_slain"
            && seen_surfaces.insert("吞石巨蛛".to_string())
        {
            links.push(ChronicleLinkRef {
                surface: "吞石巨蛛".to_string(),
                kind: "annals",
                target_id: dyn_ann.target_id.clone(),
            });
        }
    }

    links.sort_by_key(|r| std::cmp::Reverse(r.surface.chars().count()));
    links
}

/// 建構 `/api/chronicle/index` 全島三卷目錄快照（〈人物列傳〉674 篇 ＋ 〈方輿志〉7 篇 ＋ 〈紀事本末〉12 篇）
pub fn build_chronicle_index_json(doc: &WorldStaticDoc, delta: &IslandWorldDelta) -> Value {
    let live_year = solar_year_and_season_of_day(current_solar_clock().game_days)
        .0
        .max(CURRENT_SOLAR_YEAR);
    let mut actors_out = Vec::new();
    let mut cross_town_total = 0usize;
    let mut estranged_kin_total = 0usize;

    for a in delta.actors.values() {
        if a.controller_id.is_some()
            || a.actor_id.starts_with("pl:")
            || !a.alive
            || a.household_id.is_empty()
        {
            continue;
        }
        let slug = settlement_slug_of(&a.household_id);
        let town_zh = settlement_name_zh(doc, a);
        let zone_zh = zone_label_of(doc, a);
        let mut has_cross_town = false;
        let mut has_estranged_kin = false;

        for soc in a.mind.ltm.social.values() {
            if soc.target_id.starts_with("pl:")
                || delta
                    .actors
                    .get(&soc.target_id)
                    .is_some_and(|t| t.controller_id.is_some())
            {
                continue;
            }
            if !soc.target_region_zh.is_empty() && soc.target_region_zh != town_zh {
                has_cross_town = true;
                cross_town_total += 1;
            }
            if soc.is_kin() && soc.warmth <= ESTRANGED_WARMTH_MAX {
                has_estranged_kin = true;
                estranged_kin_total += 1;
            }
        }

        actors_out.push(json!({
            "actor_id": a.actor_id,
            "name": a.display_name,
            "race": a.race,
            "role": a.role,
            "bond": a.household_bond,
            "town_slug": slug,
            "town_zh": town_zh,
            "zone_zh": zone_zh,
            "door_zh": door_label_of(doc, a),
            "has_cross_town": has_cross_town,
            "has_estranged_kin": has_estranged_kin,
            "has_concern": a.mind.top_concern().is_some(),
        }));
    }

    let settlements: Vec<Value> = SETTLEMENT_BOOKS
        .iter()
        .map(|(slug, zh, _)| {
            let count = actors_out
                .iter()
                .filter(|x| x["town_slug"].as_str() == Some(*slug))
                .count();
            json!({
                "slug": slug,
                "name_zh": zh,
                "resident_count": count,
                "region_article_id": town_region_article_id(slug),
            })
        })
        .collect();

    let regions: Vec<Value> = REGIONAL_GAZETTEERS
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "title_zh": r.title_zh,
                "subtitle_zh": r.subtitle_zh,
                "era_zh": r.era_zh,
            })
        })
        .collect();

    let mut annals: Vec<Value> = all_historical_annals()
        .iter()
        .map(|an| {
            json!({
                "id": an.id,
                "title_zh": an.title_zh,
                "subtitle_zh": an.subtitle_zh,
                "era_zh": an.era_zh,
                "is_mainland_war": an.id.starts_with("ann:war_") || an.id.starts_with("ann:calamity_") || an.id.starts_with("ann:smuggle_"),
                "is_live": false,
            })
        })
        .collect();
    for dyn_an in delta.living_chronicle.dynamic_annals_sorted() {
        annals.push(json!({
            "id": dyn_an.target_id,
            "title_zh": dyn_an.title_zh,
            "subtitle_zh": dyn_an.subtitle_zh,
            "era_zh": dyn_an.era_zh,
            "is_mainland_war": false,
            "is_live": true,
        }));
    }

    json!({
        "ok": true,
        "solar_year_zh": format!("太陽曆{}年", solar_year_zh(live_year)),
        "total_residents": actors_out.len(),
        "total_regions": regions.len(),
        "total_annals": annals.len(),
        "cross_town_ties": cross_town_total,
        "estranged_kin_ties": estranged_kin_total,
        "settlements": settlements,
        "regions": regions,
        "annals": annals,
        "actors": actors_out,
    })
}

/// 建構單一角色純文學書卷列傳 JSON（100% 拔除除錯數值與條列記憶表）
pub fn build_chronicle_actor_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    actor_id: &str,
) -> Option<Value> {
    let a = delta.actors.get(actor_id)?;
    if a.controller_id.is_some() || actor_id.starts_with("pl:") {
        return None;
    }
    let my_town = settlement_name_zh(doc, a);
    let my_town_slug = settlement_slug_of(&a.household_id);
    let door_zh = door_label_of(doc, a);
    let paragraphs = build_actor_literary_paragraphs(doc, delta, a);
    let linked_refs = build_actor_precise_links(delta, a);

    let mut sorted_soc: Vec<_> = a
        .mind
        .ltm
        .social
        .values()
        .filter(|s| {
            !s.target_id.starts_with("pl:")
                && delta
                    .actors
                    .get(&s.target_id)
                    .is_some_and(|ta| ta.controller_id.is_none() && ta.alive)
        })
        .collect();
    sorted_soc.sort_by_key(|s| std::cmp::Reverse((s.warmth.abs(), s.familiarity)));

    let ego_ring: Vec<Value> = sorted_soc
        .into_iter()
        .take(12)
        .map(|soc| {
            let band = if soc.warmth >= 30 {
                "warm"
            } else if soc.warmth <= ESTRANGED_WARMTH_MAX {
                "estranged"
            } else {
                "neutral"
            };
            let tie_lbl = if soc.tie_label_zh.is_empty() {
                tie_flags_summary_zh(soc.tie_mask)
            } else {
                soc.tie_label_zh.as_str()
            };
            json!({
                "target_id": soc.target_id,
                "target_name": soc.target_name,
                "tie_label_zh": tie_lbl,
                "target_town": if soc.target_region_zh.is_empty() { &my_town } else { &soc.target_region_zh },
                "band": band,
                "inner_circle": soc.familiarity >= 58,
            })
        })
        .collect();

    let title_zh = format!("{}傳", a.display_name);
    let subtitle_zh = format!(
        "{}・{}｜{}・{}（太陽曆{}年生）",
        my_town,
        door_zh,
        a.race,
        a.role,
        solar_year_zh(a.birth_year)
    );
    let plain_copy_text = format!("{title_zh}\n{subtitle_zh}\n\n{}", paragraphs.join("\n\n"));

    Some(json!({
        "ok": true,
        "actor": {
            "actor_id": a.actor_id,
            "name": a.display_name,
            "title_zh": title_zh,
            "subtitle_zh": subtitle_zh,
            "race": a.race,
            "role": a.role,
            "bond": a.household_bond,
            "town_slug": my_town_slug,
            "town_zh": my_town,
            "region_article_id": town_region_article_id(my_town_slug),
            "zone_zh": zone_label_of(doc, a),
            "door_zh": door_zh,
            "tool_name": mask_player_names_in_text(delta, &a.bound_weapon.name),
            "tool_provenance": cjk_clean(&mask_player_names_in_text(delta, &a.bound_weapon.provenance)),
        },
        "paragraphs": paragraphs,
        "linked_refs": linked_refs,
        "ego_ring": ego_ring,
        "plain_copy_text": plain_copy_text,
    }))
}

pub fn build_chronicle_article_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    article_id: &str,
) -> Option<Value> {
    build_gazetteer_article_json(doc, delta, article_id, true)
}

fn write_file_atomic(path: &Path, content: &str) -> Result<(), String> {
    let tmp = path.with_extension("md.tmp");
    std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn linkify_markdown_paragraphs(
    delta: &IslandWorldDelta,
    self_slug: &str,
    self_name: &str,
    paras: &[String],
    refs: &[ChronicleLinkRef],
) -> String {
    let mut linked_once = BTreeSet::new();
    let mut out_paras = Vec::with_capacity(paras.len());
    for para in paras {
        let mut out = String::with_capacity(para.len() + 64);
        let mut rest = para.as_str();
        while !rest.is_empty() {
            if !self_name.is_empty() && rest.starts_with(self_name) {
                out.push_str(self_name);
                rest = &rest[self_name.len()..];
                continue;
            }
            let mut matched: Option<&ChronicleLinkRef> = None;
            for r in refs {
                if !r.surface.is_empty()
                    && !linked_once.contains(r.surface.as_str())
                    && rest.starts_with(&r.surface)
                {
                    matched = Some(r);
                    break;
                }
            }
            if let Some(r) = matched {
                linked_once.insert(r.surface.as_str());
                let href = match r.kind {
                    "actor" => {
                        let t_slug = delta
                            .actors
                            .get(&r.target_id)
                            .map_or(self_slug, |ta| settlement_slug_of(&ta.household_id));
                        let target_file = book_file_for_slug(t_slug);
                        if t_slug == self_slug {
                            format!("#{}", anchor_slug(&r.target_id))
                        } else {
                            format!("{}#{}", target_file, anchor_slug(&r.target_id))
                        }
                    }
                    "region" => format!(
                        "01_卷一方輿志_山海五鎮與蒼淵輿略.md#{}",
                        anchor_slug(&r.target_id)
                    ),
                    _ => format!(
                        "02_卷二紀事本末_銀鷗風變與蒼淵兵燹戰史.md#{}",
                        anchor_slug(&r.target_id)
                    ),
                };
                out.push_str(&format!("[{}]({})", r.surface, href));
                rest = &rest[r.surface.len()..];
            } else if let Some(ch) = rest.chars().next() {
                out.push(ch);
                rest = &rest[ch.len_utf8()..];
            }
        }
        out_paras.push(out);
    }
    out_paras.join("\n\n")
}

/// 匯出全三卷 Markdown 山海全誌（〈方輿志〉、〈紀事本末〉與五鎮 674 人〈人物列傳〉）至 `out_dir`
pub fn export_markdown_chronicles_to_dir(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    out_dir: &Path,
) -> Result<usize, String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    std::fs::write(
        out_dir.join(".gbrainignore"),
        "# 自動匯出之三卷全誌鏡像（主隔離由根目錄 .gitignore 排除 docs/chronicles/ 生效）\n*\n",
    )
    .map_err(|e| e.to_string())?;

    // 1. 匯出卷一〈方輿志〉
    let mut reg_md = String::from(
        "# 《奇點世界》卷一・方輿志（銀鷗海領五鎮與蒼淵主陸輿略）\n\n返回 [全誌總目](00_銀鷗島五鎮人物列傳總目與跨區關係網.md)。\n\n---\n\n",
    );
    for r in REGIONAL_GAZETTEERS {
        let mut r_paras: Vec<String> = r.paragraphs.iter().map(|s| (*s).to_string()).collect();
        r_paras.extend(
            delta
                .living_chronicle
                .region_paragraphs(r.id)
                .into_iter()
                .map(|p| mask_player_names_in_text(delta, &p)),
        );
        reg_md.push_str(&format!(
            "<a id=\"{}\"></a>\n## {}\n\n*{}｜載體：{}*\n\n{}\n\n---\n\n",
            anchor_slug(r.id),
            r.title_zh,
            r.subtitle_zh,
            r.carrier_zh,
            r_paras.join("\n\n")
        ));
    }
    write_file_atomic(
        &out_dir.join("01_卷一方輿志_山海五鎮與蒼淵輿略.md"),
        &reg_md,
    )?;

    // 2. 匯出卷二〈紀事本末〉
    let dyn_annals = delta.living_chronicle.dynamic_annals_sorted();
    let total_annals_cnt = all_historical_annals().len() + dyn_annals.len();
    let mut ann_md = String::from(
        "# 《奇點世界》卷二・紀事本末（銀鷗六大風變與蒼淵主陸兵燹戰史）\n\n返回 [全誌總目](00_銀鷗島五鎮人物列傳總目與跨區關係網.md)。\n\n---\n\n",
    );
    for an in all_historical_annals() {
        ann_md.push_str(&format!(
            "<a id=\"{}\"></a>\n## {}（{}）\n\n*{}｜史料出處：{}*\n\n{}\n\n---\n\n",
            anchor_slug(an.id),
            an.title_zh,
            an.era_zh,
            an.subtitle_zh,
            an.carrier_zh,
            an.paragraphs.join("\n\n")
        ));
    }
    for dyn_an in &dyn_annals {
        let body = mask_player_names_in_text(delta, &dyn_an.prose_zh);
        ann_md.push_str(&format!(
            "<a id=\"{}\"></a>\n## {}（{}）\n\n*{}｜史料出處：{}*\n\n{}\n\n---\n\n",
            anchor_slug(&dyn_an.target_id),
            dyn_an.title_zh,
            dyn_an.era_zh,
            dyn_an.subtitle_zh,
            dyn_an.carrier_zh,
            body
        ));
    }
    write_file_atomic(
        &out_dir.join("02_卷二紀事本末_銀鷗風變與蒼淵兵燹戰史.md"),
        &ann_md,
    )?;

    // 3. 匯出卷三〈人物列傳〉五鎮分冊
    let known_book_slugs: BTreeSet<&str> = SETTLEMENT_BOOKS.iter().map(|(s, _, _)| *s).collect();
    let mut by_town: BTreeMap<String, Vec<&UnifiedMindBody>> = BTreeMap::new();
    for a in delta.actors.values() {
        if a.controller_id.is_none()
            && !a.actor_id.starts_with("pl:")
            && a.alive
            && !a.household_id.is_empty()
        {
            let raw_slug = settlement_slug_of(&a.household_id);
            let norm_slug = if known_book_slugs.contains(raw_slug) {
                raw_slug
            } else {
                "white_gull_harbor"
            };
            by_town.entry(norm_slug.to_string()).or_default().push(a);
        }
    }

    let mut total_written = 0usize;
    let mut index_md = format!(
        "# 《奇點世界》山海人物與風土戰史全誌總目（同源雙出定稿）\n\n\
        本全誌直接自線上權威資料庫 `postgres-singularity` 同源匯出，分〈方輿志〉、〈紀事本末〉與〈人物列傳〉三卷，全篇採用純現代台灣正體白話傳體散文撰寫，亦可於瀏覽器開啟 `/chronicle` 書卷閱讀器翻閱或複製入樂母本。\n\n\
        ## 卷一與卷二・山海方輿與戰史本末\n\n\
        - [卷一・方輿志（銀鷗五鎮風土志與蒼淵主陸百邦輿略，共 {} 篇）](01_卷一方輿志_山海五鎮與蒼淵輿略.md)\n\
        - [卷二・紀事本末（銀鷗當代風變與蒼淵主陸跨國大戰長卷，共 {} 篇）](02_卷二紀事本末_銀鷗風變與蒼淵兵燹戰史.md)\n\n\
        ## 卷三・銀鷗五鎮人物列傳分冊\n\n\
        | 據點名稱 | 入傳人數 | 分冊連結 |\n\
        | :--- | :---: | :--- |\n",
        REGIONAL_GAZETTEERS.len(),
        total_annals_cnt
    );

    for &(slug, town_zh, filename) in SETTLEMENT_BOOKS {
        let mut list = by_town.remove(slug).unwrap_or_default();
        list.sort_by(|a, b| {
            a.household_id
                .cmp(&b.household_id)
                .then(a.actor_id.cmp(&b.actor_id))
        });

        let mut book = format!(
            "# 《奇點世界》卷三・{}人物列傳（共 {} 人）\n\n返回 [全誌總目](00_銀鷗島五鎮人物列傳總目與跨區關係網.md) ｜ 參閱 [《{}風土志》](01_卷一方輿志_山海五鎮與蒼淵輿略.md#{})\n\n---\n\n",
            town_zh,
            list.len(),
            town_zh,
            anchor_slug(town_region_article_id(slug))
        );

        let mut current_zone = String::new();
        for a in &list {
            total_written += 1;
            let z = zone_label_of(doc, a);
            if z != current_zone {
                current_zone = z.clone();
                book.push_str(&format!("## {}\n\n", current_zone));
            }
            let anc_id = anchor_slug(&a.actor_id);
            let paras = build_actor_literary_paragraphs(doc, delta, a);
            let refs = build_actor_precise_links(delta, a);
            let linked_body =
                linkify_markdown_paragraphs(delta, slug, &a.display_name, &paras, &refs);
            book.push_str(&format!(
                "<a id=\"{anc_id}\"></a>\n### {}傳\n\n*{}｜{}・{}（太陽曆{}年生）｜隨身器物：{}*\n\n{}\n\n---\n\n",
                a.display_name,
                door_label_of(doc, a),
                a.race,
                a.role,
                solar_year_zh(a.birth_year),
                a.bound_weapon.name,
                linked_body
            ));
        }

        let masked_book = mask_player_names_in_text(delta, &book);
        write_file_atomic(&out_dir.join(filename), &masked_book)?;
        index_md.push_str(&format!(
            "| **{}** | {} 人 | [{}]({}) |\n",
            town_zh,
            list.len(),
            filename,
            filename
        ));
    }

    write_file_atomic(
        &out_dir.join("00_銀鷗島五鎮人物列傳總目與跨區關係網.md"),
        &index_md,
    )?;
    Ok(total_written)
}

static STAGED_EXPORT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn fsync_dir_and_children(dir: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let p = entry.path();
        if p.is_file() {
            let f = std::fs::File::open(&p).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
    }
    let df = std::fs::File::open(dir).map_err(|e| e.to_string())?;
    df.sync_all().map_err(|e| e.to_string())
}

/// 先全套寫入同磁碟區隱藏暫存目錄並逐檔 `fsync`，再以單鎖保護之目錄置換更新目標目錄（杜絕半套髒讀與崩潰破損）
pub fn export_markdown_chronicles_staged_atomic(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    target_dir: &Path,
) -> Result<usize, String> {
    let _serial = STAGED_EXPORT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let raw_parent = target_dir
        .parent()
        .ok_or_else(|| "invalid target_dir parent".to_string())?;
    let parent = if raw_parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        raw_parent
    };
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let staging = parent.join(".staging_chronicle_export");
    let prev_bak = parent.join(".prev_chronicle_bak");

    // 崩潰復原保護：若前次恰於 `rename(target_dir, prev_bak)` 後中斷，先無損還原唯一備份，絕不誤刪！
    if !target_dir.exists() && prev_bak.exists() {
        std::fs::rename(&prev_bak, target_dir).map_err(|e| e.to_string())?;
    }
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| e.to_string())?;
    }
    if prev_bak.exists() {
        std::fs::remove_dir_all(&prev_bak).map_err(|e| e.to_string())?;
    }

    let written = match export_markdown_chronicles_to_dir(doc, delta, &staging)
        .and_then(|n| fsync_dir_and_children(&staging).map(|()| n))
    {
        Ok(n) => n,
        Err(err) => {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(err);
        }
    };

    if target_dir.exists() {
        std::fs::rename(target_dir, &prev_bak).map_err(|e| e.to_string())?;
    }
    if let Err(err) = std::fs::rename(&staging, target_dir) {
        if prev_bak.exists() {
            let _ = std::fs::rename(&prev_bak, target_dir);
        }
        let _ = std::fs::remove_dir_all(&staging);
        return Err(err.to_string());
    }
    if prev_bak.exists() {
        let _ = std::fs::remove_dir_all(&prev_bak);
    }
    if let Ok(pf) = std::fs::File::open(parent) {
        let _ = pf.sync_all();
    }
    Ok(written)
}
