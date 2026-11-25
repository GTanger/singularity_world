//! 全區域通用個人列傳撰寫、跨鎮商路三角閉合與氏族／據點地名解析模組。

use std::collections::{BTreeMap, BTreeSet};

use super::clock_and_lang::solar_year_and_season_of_day;
use super::harbor_lineage::CURRENT_SOLAR_YEAR;
use super::mind::tie_flags;
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};
use super::world_weave::{ActorSnap, pair_hash, record_bilateral_life_event};

/// 自 `household_id` 或 `node_id` 解析據點代碼（通用於全島 5 鎮及未來新增聚落）
pub fn settlement_slug_of(id: &str) -> &str {
    let mut parts = id.split(':');
    let _prefix = parts.next();
    parts.next().unwrap_or("white_gull_harbor")
}

/// 將據點代碼轉為世界內中文聚落名稱（未知新聚落自動回退讀靜態節點名）
pub fn settlement_name_zh(doc: &WorldStaticDoc, actor: &UnifiedMindBody) -> String {
    match settlement_slug_of(&actor.household_id) {
        "white_gull_harbor" => "白鷗港".to_string(),
        "gull_tail_port" => "鷗尾漁港".to_string(),
        "west_strait_ferry" => "西峽渡口".to_string(),
        "greypine_post" => "灰松林驛".to_string(),
        "north_cape_beacon" => "海風北塢".to_string(),
        _ => doc
            .nodes
            .get(&actor.home_node)
            .and_then(|n| n.name.split('・').next())
            .unwrap_or("銀鷗島")
            .to_string(),
    }
}

/// 擷取雙字家號／氏族前綴（如「赤爐莉亞」→ Some("赤爐")，「羅蘭」→ None），供跨鎮同氏族手足叔舅安全配對
pub fn extract_clan_prefix(display_name: &str) -> Option<&'static str> {
    const KNOWN_CLAN_PREFIXES: &[&str] = &[
        "赤爐", "鐵砧", "石錘", "方石", "灰鑿", "鐵楔", "碎岩", "銅秤", "硬靴", "熟革", "韌線",
        "潮聲", "浪花", "鹹網", "魚叉", "船肋", "重擔", "鐵鉤", "寬肩", "粗繩", "北辰", "風向",
        "林蹤", "苦艾", "松木", "黑麥", "穀倉", "犁頭", "封蠟", "號角", "鐵矛", "疾風", "紅鬚",
    ];
    for &p in KNOWN_CLAN_PREFIXES {
        if display_name.starts_with(p) && display_name.chars().count() >= 3 {
            return Some(p);
        }
    }
    None
}

/// 四族成年門檻（未滿此歲數者僅有鄰巷玩伴，不背負成人債務、婚嫁或暗線把柄）
pub fn racial_adult_age(race: &str) -> i16 {
    match race {
        "精靈" => 55,
        "矮人" => 38,
        _ => 16,
    }
}

pub(super) fn craft_bucket(role: &str) -> &'static str {
    if role.contains("鍛")
        || role.contains("鐵")
        || role.contains("炭")
        || role.contains("錘")
        || role.contains("刃")
    {
        "smith"
    } else if role.contains("木")
        || role.contains("桶")
        || role.contains("鋸")
        || role.contains("漆")
    {
        "wood"
    } else if role.contains("石") || role.contains("鑿") {
        "stone"
    } else if role.contains("皮") || role.contains("革") {
        "leather"
    } else if role.contains("漁")
        || role.contains("魚")
        || role.contains("網")
        || role.contains("櫓")
    {
        "fish"
    } else if role.contains("藥") || role.contains("醫") {
        "herb"
    } else if role.contains("農") || role.contains("麥") || role.contains("畦") {
        "farm"
    } else if role.contains("獵") || role.contains("箭") {
        "hunt"
    } else {
        "port_trade"
    }
}

pub fn bare_given_name(display_name: &str) -> &str {
    if let Some(pref) = extract_clan_prefix(display_name) {
        display_name.strip_prefix(pref).unwrap_or(display_name)
    } else {
        display_name
    }
}

/// 為每位角色合成具因果脈絡、隨世界經歷自動續寫之個人生平列傳（`personal_bio_zh`）
pub fn compose_personal_biography_zh(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    actor_id: &str,
    _mainland_tie_opt: Option<&str>,
) -> String {
    let Some(a) = delta.actors.get(actor_id) else {
        return String::new();
    };
    if !a.bio_core_zh.trim().is_empty() {
        return a.bio_core_zh.clone();
    }
    super::causal_arcs::render_deterministic_shiji_bio(doc, a, &[])
}

/// 確保角色開服前傳基座 `bio_core_zh` 完整定錨；完全與 `turning_points`（AI 內部決策環形佇列）解耦，
/// 開服後里程碑統一由 `living_chronicler` 原子寫入 `oi_chronicle_projections`，`UnifiedMindBody` 僅保留 $O(1)$ 前傳基座與最多 4 句近期摘要。
pub fn refresh_actor_biography_if_needed(
    doc: &WorldStaticDoc,
    a: &mut UnifiedMindBody,
    _current_day: i64,
) -> bool {
    if a.controller_id.is_some() || a.actor_id.starts_with("pl:") || !a.alive {
        return false;
    }
    let mut changed = false;
    if a.bio_core_zh.trim().is_empty() {
        a.bio_core_zh = super::causal_arcs::render_deterministic_shiji_bio(doc, a, &[]);
        changed = true;
    }
    if a.bio_post_chapters_zh.len() > 4 {
        let drop_n = a.bio_post_chapters_zh.len() - 4;
        a.bio_post_chapters_zh.drain(0..drop_n);
        changed = true;
    }
    if a.personal_bio_zh != a.bio_core_zh {
        a.personal_bio_zh = a.bio_core_zh.clone();
        changed = true;
    }
    if changed {
        a.rev = a.rev.saturating_add(1);
        a.dirty = true;
    }
    changed
}

/// 為玩家角色即時合成純世界內現代白話生平小傳（供遊戲內 `[近事／生平]` 書卷檢視）
pub fn compose_player_biography_zh(
    doc: &WorldStaticDoc,
    p: &UnifiedMindBody,
    current_day: i64,
) -> String {
    let (yr, season) = solar_year_and_season_of_day(current_day);
    let yr_zh = super::causal_arcs::solar_year_zh(yr);
    let cur_place = doc
        .nodes
        .get(&p.node_id)
        .map_or_else(|| "銀鷗島".to_string(), |n| n.name.replace('・', "．"));
    let house_clause = p
        .owned_house_node
        .as_deref()
        .and_then(|nid| doc.nodes.get(nid))
        .map(|nd| {
            format!(
                "，並已在{}置下自有石木屋舍立住腳跟",
                nd.name.replace('・', "．")
            )
        })
        .unwrap_or_default();
    let mastery_clause = p
        .mind
        .ltm
        .emergent_phase_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|m| format!("，歷練間更凝出「{m}」的獨門身手"))
        .unwrap_or_default();
    let mut recent_eps: Vec<String> = p
        .bio_post_chapters_zh
        .iter()
        .map(|s| s.trim_end_matches('。').to_string())
        .collect();
    for e in p.mind.ltm.episodic.iter().rev().take(3).rev() {
        let line = e.subjective_zh.trim_end_matches('。').to_string();
        if !recent_eps.contains(&line) {
            recent_eps.push(line);
        }
    }
    let ep_para = if recent_eps.is_empty() {
        format!(
            "太陽曆{yr_zh}年{season}，{}正駐足於{}，山海行旅的新篇章方才展開。",
            p.display_name, cur_place
        )
    } else {
        format!(
            "登島以來親歷諸事：{}。至太陽曆{yr_zh}年{season}，行蹤正落於{}。",
            recent_eps.join("；"),
            cur_place
        )
    };
    let prov_clean = p.bound_weapon.provenance.trim_end_matches('。').trim();
    let prov_clause = if prov_clean.is_empty() {
        String::new()
    } else {
        format!("（{prov_clean}）")
    };
    format!(
        "{}，{}行旅之人，{}，隨身常握【{}】{}{}{}。\n\n{}",
        p.display_name,
        p.race,
        p.appearance.trim_end_matches('。'),
        p.bound_weapon.name,
        prov_clause,
        house_clause,
        mastery_clause,
        ep_para
    )
    .replace("  ", " ")
}

/// 第 5 層：織入五鎮商路樞紐、二跳鄰居三角閉合與稀疏非公開心結（嚴控成人 3%–12%，每人至多 1 樁）
pub(super) fn weave_brokers_triads_and_secrets(
    snaps: &BTreeMap<String, ActorSnap>,
    delta: &mut IslandWorldDelta,
) {
    let brokers: Vec<String> = snaps
        .values()
        .filter(|s| {
            s.hh_id.contains(":shop_")
                || s.role.contains("掌櫃")
                || s.role.contains("銀號")
                || s.role.contains("車行")
                || s.role.contains("領航")
        })
        .map(|s| s.id.clone())
        .collect();
    if brokers.len() >= 2 {
        for i in 0..brokers.len() {
            for step in [1usize, 3usize] {
                let j = (i + step) % brokers.len();
                let (Some(ba), Some(bb)) = (snaps.get(&brokers[i]), snaps.get(&brokers[j])) else {
                    continue;
                };
                if ba.hh_id == bb.hh_id || ba.id >= bb.id {
                    continue;
                }
                let ev_id = format!("weave:broker_net:{}:{}", ba.id, bb.id);
                let ep_a = format!(
                    "太陽曆 324 年前後，與{}操持{}的{}議定長年互通貨訊與熟客口風，兩邊鋪面常有往來。",
                    bb.town_zh, bb.role, bb.name
                );
                let ep_b = format!(
                    "太陽曆 324 年前後，與{}操持{}的{}議定長年互通貨訊與熟客口風，兩邊鋪面常有往來。",
                    ba.town_zh, ba.role, ba.name
                );
                record_bilateral_life_event(
                    delta,
                    &ev_id,
                    324,
                    &ba.home_node,
                    ba,
                    bb,
                    tie_flags::TRADE_BROKER,
                    "跨區商路熟識",
                    "跨區商路熟識",
                    true,
                    60,
                    28,
                    28,
                    Some(ep_a),
                    Some(ep_b),
                    &format!("{}的商路樞紐同行，常互通各鎮貨源風聲", bb.town_zh),
                    &format!("{}的商路樞紐同行，常互通各鎮貨源風聲", ba.town_zh),
                    "",
                    "",
                );
            }
        }
    }

    let mut base_adj: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let warm_hubs: BTreeMap<String, Vec<String>> = snaps
        .iter()
        .map(|(aid, sa)| {
            let mut ranked: Vec<(bool, bool, String)> = Vec::new();
            if let Some(act) = delta.actors.get(aid) {
                for (tid, soc) in &act.mind.ltm.social {
                    let is_base = soc
                        .folded_events
                        .iter()
                        .any(|e| !e.starts_with("weave:triad:") && !e.starts_with("weave:secret:"));
                    if is_base
                        && let Some(st) = snaps.get(tid)
                        && st.hh_id != sa.hh_id
                    {
                        base_adj.entry(aid.clone()).or_default().insert(tid.clone());
                        let strong_base = soc.folded_events.iter().any(|e| {
                            !e.starts_with("seed_tie:")
                                && !e.starts_with("weave:triad:")
                                && !e.starts_with("weave:secret:")
                        });
                        ranked.push((st.town_slug == sa.town_slug, strong_base, tid.clone()));
                    }
                }
            }
            ranked.sort_by(|a, b| {
                b.0.cmp(&a.0)
                    .then_with(|| b.1.cmp(&a.1))
                    .then_with(|| a.2.cmp(&b.2))
            });
            (
                aid.clone(),
                ranked.into_iter().take(6).map(|(_, _, id)| id).collect(),
            )
        })
        .collect();

    let mut closed_pairs: BTreeSet<(String, String)> = BTreeSet::new();
    let mut triad_deg: BTreeMap<String, usize> = BTreeMap::new();
    for (hub_id, friends) in &warm_hubs {
        let Some(shub) = snaps.get(hub_id) else {
            continue;
        };
        if (CURRENT_SOLAR_YEAR - shub.birth_year) < racial_adult_age(&shub.race) {
            continue;
        }
        for i in 0..friends.len() {
            for j in (i + 1)..friends.len() {
                let x_id = &friends[i];
                let y_id = &friends[j];
                let (Some(sx), Some(sy)) = (snaps.get(x_id), snaps.get(y_id)) else {
                    continue;
                };
                if sx.hh_id == sy.hh_id
                    || (CURRENT_SOLAR_YEAR - sx.birth_year) < racial_adult_age(&sx.race)
                    || (CURRENT_SOLAR_YEAR - sy.birth_year) < racial_adult_age(&sy.race)
                    || base_adj.get(x_id).is_some_and(|s| s.contains(y_id))
                    || triad_deg.get(x_id).copied().unwrap_or(0) >= 6
                    || triad_deg.get(y_id).copied().unwrap_or(0) >= 6
                {
                    continue;
                }
                let (lo, hi) = if x_id < y_id {
                    (x_id.clone(), y_id.clone())
                } else {
                    (y_id.clone(), x_id.clone())
                };
                if pair_hash(&lo, &hi, 0x505) % 10 >= 6 || !closed_pairs.insert((lo, hi)) {
                    continue;
                }
                *triad_deg.entry(x_id.clone()).or_default() += 1;
                *triad_deg.entry(y_id.clone()).or_default() += 1;
                let ev_id = format!("weave:triad:{}:{}:via:{}", sx.id, sy.id, shub.id);
                record_bilateral_life_event(
                    delta,
                    &ev_id,
                    325,
                    &shub.home_node,
                    sx,
                    sy,
                    tie_flags::PEER_COHORT,
                    "熟人引薦舊識",
                    "熟人引薦舊識",
                    true,
                    46,
                    22,
                    20,
                    None,
                    None,
                    &format!("經共同熟識{}引薦認識的{}朋友", shub.name, sy.town_zh),
                    &format!("經共同熟識{}引薦認識的{}朋友", shub.name, sx.town_zh),
                    "",
                    "",
                );
            }
        }
    }

    let adult_ids: Vec<String> = snaps
        .values()
        .filter(|s| (CURRENT_SOLAR_YEAR - s.birth_year) >= racial_adult_age(&s.race))
        .map(|s| s.id.clone())
        .collect();
    let target_secret_pairs = (adult_ids.len() / 28).max(4);
    let mut has_secret: BTreeSet<String> = BTreeSet::new();
    let mut secret_made = 0usize;
    for idx in 0..adult_ids.len() {
        if secret_made >= target_secret_pairs {
            break;
        }
        let a_id = &adult_ids[idx];
        if pair_hash(a_id, "secret_gate", 0x606) % 11 != 0 || has_secret.contains(a_id) {
            continue;
        }
        let b_idx = (idx + 17) % adult_ids.len();
        let b_id = &adult_ids[b_idx];
        let (Some(sa), Some(sb)) = (snaps.get(a_id), snaps.get(b_id)) else {
            continue;
        };
        if sa.hh_id == sb.hh_id || has_secret.contains(b_id) {
            continue;
        }
        has_secret.insert(a_id.clone());
        has_secret.insert(b_id.clone());
        secret_made += 1;
        let ev_id = format!("weave:secret:{}:{}", sa.id, sb.id);
        let sec_a = format!(
            "心底壓著一樁不可對外人道的舊事：太陽曆 326 年夜裡曾撞見{}暗中夾帶未報稅的檀香木匣過手，對方至今怕我說漏嘴。",
            sb.name
        );
        let sec_b = format!(
            "心頭始終忌憚{}：太陽曆 326 年夜裡私下夾帶避稅貨匣被對方撞見，這把柄一直捏在人家手裡。",
            sa.name
        );
        record_bilateral_life_event(
            delta,
            &ev_id,
            326,
            &sa.home_node,
            sa,
            sb,
            0,
            "握有舊事底細之人",
            "知曉自身暗事之人",
            true,
            42,
            -8,
            -28,
            None,
            None,
            "街面碰頭客客氣氣的舊識，平素各自保持幾分分寸",
            "街面碰頭點頭寒暄的舊識，往來話不多卻彼此認得",
            &sec_a,
            &sec_b,
        );
    }
}

pub(super) fn weave_minor_playmates(
    town_minors: &BTreeMap<String, Vec<String>>,
    snaps: &BTreeMap<String, ActorSnap>,
    delta: &mut IslandWorldDelta,
) {
    for list in town_minors.values() {
        let n = list.len();
        if n < 2 {
            continue;
        }
        for i in 0..n {
            let j = (i + 1) % n;
            let (Some(ma), Some(mb)) = (snaps.get(&list[i]), snaps.get(&list[j])) else {
                continue;
            };
            if ma.hh_id == mb.hh_id || ma.id >= mb.id {
                continue;
            }
            let yr = (CURRENT_SOLAR_YEAR - 1).max(ma.birth_year.max(mb.birth_year) + 6);
            let ev_id = format!("weave:playmate:{}:{}", ma.id, mb.id);
            let play_note = if ma.town_slug == "greypine_post" || ma.town_slug.starts_with("burg_")
            {
                "常在村巷口一塊追逐撿松果的同齡玩伴"
            } else {
                "常在巷口一塊追逐撿貝殼的同齡玩伴"
            };
            record_bilateral_life_event(
                delta,
                &ev_id,
                yr,
                &ma.home_node,
                ma,
                mb,
                tie_flags::PEER_COHORT,
                "鄰巷玩伴",
                "鄰巷玩伴",
                true,
                65,
                35,
                35,
                None,
                None,
                play_note,
                play_note,
                "",
                "",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::outer_island::dialogue_llm::validate_clean_in_world_reply;
    use crate::grid::outer_island::engine::load_static_world_doc;
    use crate::grid::outer_island::persona::derive_innate_persona;
    use crate::grid::outer_island::sim_tick::seed_genesis_actors_once;
    use crate::grid::outer_island::world_weave::apply_world_lineage_and_memory_weave;

    #[test]
    fn world_weave_unifies_memory_relations_closeness_and_cross_town_ties() {
        let doc = load_static_world_doc();
        let mut delta = IslandWorldDelta {
            genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
            municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
            silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
            ..IslandWorldDelta::default()
        };
        seed_genesis_actors_once(&doc, &mut delta, false);

        // 驗證連續重跑完全冪等（不疊加 warmth、不破壞 120,805 守恆）
        let snap_warmth_1: Vec<(String, Vec<(String, i8)>)> = delta
            .actors
            .values()
            .map(|a| {
                (
                    a.actor_id.clone(),
                    a.mind
                        .ltm
                        .social
                        .iter()
                        .map(|(k, v)| (k.clone(), v.warmth))
                        .collect(),
                )
            })
            .collect();
        apply_world_lineage_and_memory_weave(&doc, &mut delta);
        let snap_warmth_2: Vec<(String, Vec<(String, i8)>)> = delta
            .actors
            .values()
            .map(|a| {
                (
                    a.actor_id.clone(),
                    a.mind
                        .ltm
                        .social
                        .iter()
                        .map(|(k, v)| (k.clone(), v.warmth))
                        .collect(),
                )
            })
            .collect();
        assert_eq!(snap_warmth_1, snap_warmth_2);
        assert_eq!(delta.conservation_diff(&doc), 0);

        // 驗證睡眠整合不會把同屋至親熟稔度衰減成陌生人，且 64 槽在超量玩家湧入時仍嚴守硬上限
        if let Some(sample_id) = delta.actors.keys().find(|k| k.starts_with("res:")).cloned()
            && let Some(act) = delta.actors.get_mut(&sample_id)
        {
            let cohab_before: Vec<(String, u8)> = act
                .mind
                .ltm
                .social
                .iter()
                .filter(|(_, s)| s.cohabiting)
                .map(|(k, s)| (k.clone(), s.familiarity))
                .collect();
            let innate = derive_innate_persona(act.soul_seed.max(1));
            act.mind
                .run_sleep_consolidation(&innate, &mut act.drift, 12_000, false);
            let cohab_after: Vec<(String, u8)> = act
                .mind
                .ltm
                .social
                .iter()
                .filter(|(_, s)| s.cohabiting)
                .map(|(k, s)| (k.clone(), s.familiarity))
                .collect();
            assert_eq!(cohab_before, cohab_after);

            for idx in 0..80 {
                act.mind.record_talk_exchange(
                    &format!("pl:test_overflow_{idx}"),
                    "旅人",
                    12_001 + idx,
                    "午鐘",
                    "你好",
                    "你好",
                    false,
                );
            }
            assert!(
                act.mind.ltm.social.len() <= crate::grid::outer_island::mind::MAX_SOCIAL_JUDGMENTS
            );
        }
        // 還原重跑，並額外模擬正式庫 v2 舊槽（`notable_note_zh = "同屋家眷師徒", from_genesis = false, folded_events = []`）遷移驗證（R2-F1）
        let mut delta = IslandWorldDelta {
            genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
            municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
            silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
            ..IslandWorldDelta::default()
        };
        seed_genesis_actors_once(&doc, &mut delta, false);
        for aid in delta.actors.keys().cloned().collect::<Vec<_>>() {
            if let Some(act) = delta.actors.get_mut(&aid) {
                for soc in act.mind.ltm.social.values_mut() {
                    if soc.cohabiting {
                        soc.notable_note_zh = "同屋家眷師徒".to_string();
                        soc.from_genesis = false;
                        soc.folded_events.clear();
                        soc.warmth = 0;
                    }
                }
            }
        }
        apply_world_lineage_and_memory_weave(&doc, &mut delta);
        for a in delta.actors.values() {
            if a.controller_id.is_some() {
                continue;
            }
            for soc in a.mind.ltm.social.values() {
                assert_ne!(
                    soc.notable_note_zh, "同屋家眷師徒",
                    "legacy generic household note must be upgraded during v2->lineage_v2 migration"
                );
            }
        }

        let mut adult_non_heads = 0usize;
        let mut adult_non_heads_with_indep_tie = 0usize;
        let mut estranged_kin_cnt = 0usize;
        let mut warm_non_kin_cnt = 0usize;
        let mut cross_town_edges = 0usize;
        let mut secret_holders = 0usize;
        let mut total_adults = 0usize;
        let mut wgh_cross_adj: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for a in delta.actors.values() {
            if a.controller_id.is_some() {
                continue;
            }
            assert!(
                validate_clean_in_world_reply(&a.personal_bio_zh).is_some(),
                "unclean bio: {}",
                a.personal_bio_zh
            );
            let my_town = settlement_name_zh(&doc, a);
            let lived = CURRENT_SOLAR_YEAR - a.birth_year;
            let is_adult = lived >= racial_adult_age(&a.race);
            if is_adult {
                total_adults += 1;
            }
            let head_id_opt = delta
                .actors
                .values()
                .filter(|x| x.household_id == a.household_id)
                .map(|x| x.actor_id.clone())
                .min();
            let head_targets: BTreeSet<String> = head_id_opt
                .as_ref()
                .and_then(|hid| delta.actors.get(hid))
                .map(|ha| ha.mind.ltm.social.keys().cloned().collect())
                .unwrap_or_default();

            let is_non_head = head_id_opt.as_deref() != Some(a.actor_id.as_str());
            let mut has_indep = false;
            let mut my_secrets = 0usize;

            for (tid, soc) in &a.mind.ltm.social {
                if let Some(ta) = delta.actors.get(tid)
                    && ta.household_id != a.household_id
                {
                    if !head_targets.contains(tid) {
                        has_indep = true;
                    }
                    if a.household_id.starts_with("hh:white_gull_harbor:")
                        && ta.household_id.starts_with("hh:white_gull_harbor:")
                    {
                        wgh_cross_adj
                            .entry(a.actor_id.clone())
                            .or_default()
                            .insert(tid.clone());
                        wgh_cross_adj
                            .entry(tid.clone())
                            .or_default()
                            .insert(a.actor_id.clone());
                    }
                }
                if soc.is_kin() && soc.warmth <= -18 {
                    estranged_kin_cnt += 1;
                }
                if !soc.is_kin() && soc.warmth >= 42 {
                    warm_non_kin_cnt += 1;
                }
                if !soc.target_region_zh.is_empty() && soc.target_region_zh != my_town {
                    cross_town_edges += 1;
                }
                if !soc.secret_hook_zh.is_empty() {
                    my_secrets += 1;
                    assert!(is_adult, "minor must never hold secret hook");
                }
            }
            assert!(my_secrets <= 1, "at most 1 secret hook per actor");
            if my_secrets > 0 {
                secret_holders += 1;
            }
            if is_adult && is_non_head {
                adult_non_heads += 1;
                if has_indep {
                    adult_non_heads_with_indep_tie += 1;
                }
            }
        }

        let indep_rate = adult_non_heads_with_indep_tie as f64 / adult_non_heads.max(1) as f64;
        assert!(
            indep_rate >= 0.90,
            "expected adult non-head independent rate >= 0.90, got {indep_rate:.3} ({adult_non_heads_with_indep_tie}/{adult_non_heads})"
        );
        assert!(
            estranged_kin_cnt >= 20,
            "expected estranged blood/in-law kin cases"
        );
        assert!(
            warm_non_kin_cnt >= 50,
            "expected warm non-kin comrades/peers"
        );
        assert!(
            cross_town_edges >= 120,
            "expected rich cross-settlement ties across 5 towns"
        );

        let secret_rate = secret_holders as f64 / total_adults.max(1) as f64;
        assert!(
            (0.03..=0.12).contains(&secret_rate),
            "secret hook rate out of sparse range: {secret_rate:.3}"
        );

        let mut cc_sum = 0.0f64;
        let mut cc_cnt = 0usize;
        for nbrs in wgh_cross_adj.values() {
            let k = nbrs.len();
            if k < 2 {
                continue;
            }
            let v: Vec<&String> = nbrs.iter().collect();
            let mut links = 0usize;
            for i in 0..k {
                for j in (i + 1)..k {
                    if wgh_cross_adj.get(v[i]).is_some_and(|s| s.contains(v[j])) {
                        links += 1;
                    }
                }
            }
            cc_sum += (2.0 * links as f64) / ((k * (k - 1)) as f64);
            cc_cnt += 1;
        }
        let avg_cc = cc_sum / cc_cnt.max(1) as f64;
        assert!(
            (0.24..=0.55).contains(&avg_cc),
            "unexpected cross-household clustering coefficient: {avg_cc:.4}"
        );
    }

    #[test]
    fn biography_evolves_automatically_with_turning_points_and_pact_shifts() {
        let doc = load_static_world_doc();
        let mut delta = IslandWorldDelta {
            genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
            municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
            silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
            ..IslandWorldDelta::default()
        };
        seed_genesis_actors_once(&doc, &mut delta, false);

        let target_id = "res:west_strait_ferry:ferry_row_2_0".to_string();
        let before_bio = delta
            .actors
            .get(&target_id)
            .unwrap()
            .personal_bio_zh
            .clone();
        let origin_sentence = before_bio.split('。').next().unwrap().to_string();
        assert!(!before_bio.contains("近歲新篇："));

        // 模擬太陽曆 330 年仲夏（第 6,650 遊戲日）：驗證 `slow:*` 留在 `turning_points` 不污染列傳，而真實契約違約經 `living_chronicler` 入傳
        let day_330_summer = (330 - 312) * 364 + 100;
        let clk_330 =
            super::super::clock_and_lang::solar_clock_for_settled_bell(day_330_summer * 12 + 6, 0);
        if let Some(act) = delta.actors.get_mut(&target_id) {
            act.mind.push_turning_point(
                "slow:test:6650",
                day_330_summer,
                "連月海霧魚獲短少，決意改走近岸熟客訂單穩住家計。",
                "work_steady",
            );
            let innate = derive_innate_persona(act.soul_seed.max(1));
            act.mind
                .run_sleep_consolidation(&innate, &mut act.drift, day_330_summer, false);
            let _ = refresh_actor_biography_if_needed(&doc, act, day_330_summer);
        }
        super::super::living_chronicler::record_pact_default(
            &doc,
            &mut delta,
            &clk_330,
            "ct_bio_test_330",
            &target_id,
            "res:white_gull_harbor:shop_0_0",
            60,
        );

        let after_bio = &delta.actors.get(&target_id).unwrap().personal_bio_zh;
        assert!(
            after_bio.starts_with(&origin_sentence),
            "early origin must never be overwritten: {after_bio}"
        );
        assert!(
            !after_bio.contains("連月海霧魚獲短少"),
            "slow-think turning point must never leak into personal_bio_zh: {after_bio}"
        );
        let chr_json =
            super::super::chronicle_export::build_chronicle_actor_json(&doc, &delta, &target_id)
                .expect("chronicle actor json")
                .to_string();
        assert!(
            chr_json.contains("太陽曆三三〇年仲夏") && chr_json.contains("鐵砧巴爾"),
            "living chronicler must project real pact default chapter: {chr_json}"
        );
        assert!(validate_clean_in_world_reply(after_bio).is_some());
    }
}
