//! 全世界共用之「個人生命事件鏈 × 名分與親疏冷暖正交 × 跨區域記憶交織」確定性引擎。
//!
//! 嚴守審辯室 Room `48dca12c` 與後端紅隊 Room `e8ff1ef0` 裁決：
//! 1. 關係即是記憶的一部分：每條對人關係皆由帶年份之個人生命事件單向增量摺疊進入 `ltm.social`（帶 `event_id` 去重，100% 冪等）。
//! 2. 名分與親疏冷暖徹底脫鉤：名分互逆成對寫入（`tie_mask` 支援多重重疊），熟稔度與心頭冷暖則依雙方 `soul_seed` 脾氣與逆鱗獨立折射（血親亦會反目疏遠，異族外人亦可過命親厚）。
//! 3. 不動任何人的靜態住址、戶籍與銅幣（`120,805` 守恆差額 = 0），透過既有五鎮 674 人跨鎮氏族、姻親、同門、同輩、商路樞紐與二跳三角閉合自然織出多區域立體關係網。

use std::collections::BTreeMap;

use crate::combat::wordplate::splitmix64;

use super::harbor_lineage::CURRENT_SOLAR_YEAR;
use super::mind::{EvidenceTier, tie_flags};
use super::persona::derive_innate_persona;
use super::types::{IslandWorldDelta, WorldStaticDoc};
use super::world_weave_bio::{
    bare_given_name, compose_personal_biography_zh, craft_bucket, extract_clan_prefix,
    racial_adult_age, settlement_name_zh, settlement_slug_of, weave_brokers_triads_and_secrets,
    weave_minor_playmates,
};

pub(super) fn pair_hash(id_a: &str, id_b: &str, salt: u64) -> u64 {
    let (lo, hi) = if id_a <= id_b {
        (id_a, id_b)
    } else {
        (id_b, id_a)
    };
    let mut h = 0xcbf29ce484222325u64 ^ salt;
    for b in lo.bytes().chain([0xFF]).chain(hi.bytes()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    splitmix64(h)
}

#[derive(Clone)]
pub(super) struct ActorSnap {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) race: String,
    pub(super) role: String,
    pub(super) hh_id: String,
    pub(super) home_node: String,
    pub(super) town_slug: String,
    pub(super) town_zh: String,
    pub(super) birth_year: i16,
    pub(super) bond: String,
    pub(super) seed: u64,
    pub(super) clan_opt: Option<&'static str>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "explicit bilateral life event folding inputs"
)]
pub(super) fn record_bilateral_life_event(
    delta: &mut IslandWorldDelta,
    ev_id: &str,
    ev_year: i16,
    loc_node: &str,
    a: &ActorSnap,
    b: &ActorSnap,
    tie_mask: u8,
    label_a_sees_b: &str,
    label_b_sees_a: &str,
    direct_met: bool,
    base_fam: u8,
    base_warmth_a: i8,
    base_warmth_b: i8,
    ep_a_zh: Option<String>,
    ep_b_zh: Option<String>,
    note_a_zh: &str,
    note_b_zh: &str,
    secret_a: &str,
    secret_b: &str,
) {
    if a.id == b.id {
        return;
    }
    let p_a = derive_innate_persona(a.seed);
    let p_b = derive_innate_persona(b.seed);
    let compat_ab = p_a.interpersonal_compat_delta(&p_b);
    let compat_ba = p_b.interpersonal_compat_delta(&p_a);
    let origin_day = i64::from(ev_year - 312) * 364;
    let a_old_enough = ev_year - a.birth_year >= super::causal_arcs::juvenile_witness_age(&a.race);
    let b_old_enough = ev_year - b.birth_year >= super::causal_arcs::juvenile_witness_age(&b.race);
    let tier_a = if direct_met && a_old_enough {
        EvidenceTier::DirectWitness
    } else {
        EvidenceTier::FamilyOral
    };
    let tier_b = if direct_met && b_old_enough {
        EvidenceTier::DirectWitness
    } else {
        EvidenceTier::FamilyOral
    };
    let (cred_a, fid_a) = if tier_a == EvidenceTier::DirectWitness {
        (94, 90)
    } else {
        (88, 68)
    };
    let (cred_b, fid_b) = if tier_b == EvidenceTier::DirectWitness {
        (94, 90)
    } else {
        (88, 68)
    };

    let w_a = (i16::from(base_warmth_a) + i16::from(compat_ab)).clamp(-85, 88) as i8;
    let w_b = (i16::from(base_warmth_b) + i16::from(compat_ba)).clamp(-85, 88) as i8;

    let cohabiting = a.hh_id == b.hh_id && !a.hh_id.is_empty();
    if let Some(act_a) = delta.actors.get_mut(&a.id) {
        if let Some(text_a) = ep_a_zh {
            act_a.mind.ingest_historical_episode(
                ev_id, origin_day, loc_node, &b.id, &b.name, tier_a, cred_a, fid_a, 78, 5, text_a,
            );
        }
        if act_a.mind.fold_social_memory_from_episode(
            ev_id,
            origin_day,
            &b.id,
            &b.name,
            &b.town_zh,
            tie_mask,
            label_a_sees_b,
            direct_met,
            cohabiting,
            base_fam,
            w_a,
            note_a_zh,
            secret_a,
        ) {
            act_a.rev = act_a.rev.saturating_add(1);
            act_a.dirty = true;
        }
    }

    if let Some(act_b) = delta.actors.get_mut(&b.id) {
        if let Some(text_b) = ep_b_zh {
            act_b.mind.ingest_historical_episode(
                ev_id, origin_day, loc_node, &a.id, &a.name, tier_b, cred_b, fid_b, 78, 5, text_b,
            );
        }
        if act_b.mind.fold_social_memory_from_episode(
            ev_id,
            origin_day,
            &a.id,
            &a.name,
            &a.town_zh,
            tie_mask,
            label_b_sees_a,
            direct_met,
            cohabiting,
            base_fam,
            w_b,
            note_b_zh,
            secret_b,
        ) {
            act_b.rev = act_b.rev.saturating_add(1);
            act_b.dirty = true;
        }
    }
}

/// 對全島（及未來任意新聚落）全體常住居民執行個人生命線、屋內親疏分化與跨區域人際記憶編織。
pub fn apply_world_lineage_and_memory_weave(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
) -> usize {
    let mut snaps: BTreeMap<String, ActorSnap> = BTreeMap::new();
    let mut by_hh: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (aid, a) in &delta.actors {
        if a.controller_id.is_some() || !a.alive || a.household_id.is_empty() {
            continue;
        }
        let s = ActorSnap {
            id: aid.clone(),
            name: a.display_name.clone(),
            race: a.race.clone(),
            role: a.role.clone(),
            hh_id: a.household_id.clone(),
            home_node: a.home_node.clone(),
            town_slug: settlement_slug_of(&a.household_id).to_string(),
            town_zh: settlement_name_zh(doc, a),
            birth_year: a.birth_year,
            bond: a.household_bond.clone(),
            seed: a.soul_seed.max(1),
            clan_opt: extract_clan_prefix(&a.display_name),
        };
        by_hh.entry(s.hh_id.clone()).or_default().push(aid.clone());
        snaps.insert(aid.clone(), s);
    }
    for mems in by_hh.values_mut() {
        mems.sort_by(|a, b| {
            let ha = snaps.get(a).is_some_and(|s| {
                s.bond.contains("當家") || s.bond.contains("匠師") || s.bond.contains("掌櫃")
            });
            let hb = snaps.get(b).is_some_and(|s| {
                s.bond.contains("當家") || s.bond.contains("匠師") || s.bond.contains("掌櫃")
            });
            hb.cmp(&ha).then_with(|| a.cmp(b))
        });
    }

    // 補齊既有 `ltm.social` 條目之 `target_region_zh`（無損增補）
    for (_, a_ro) in snaps.iter() {
        let updates: Vec<(String, String)> = delta
            .actors
            .get(&a_ro.id)
            .map(|act| {
                act.mind
                    .ltm
                    .social
                    .keys()
                    .filter_map(|tid| snaps.get(tid).map(|ts| (tid.clone(), ts.town_zh.clone())))
                    .collect()
            })
            .unwrap_or_default();
        if let Some(act_mut) = delta.actors.get_mut(&a_ro.id) {
            for (tid, reg) in updates {
                if let Some(soc) = act_mut.mind.ltm.social.get_mut(&tid) {
                    if soc.target_region_zh.is_empty() {
                        soc.target_region_zh = reg;
                    }
                    // R2-F1：將正式庫 v2 既有未摺疊之舊種子槽（「同屋家眷師徒」等）無損歸正為可覆寫前史槽，並消除舊種子偽造之 kept_pacts=1
                    if !tid.starts_with("pl:") && soc.folded_events.is_empty() {
                        soc.from_genesis = true;
                        if matches!(
                            soc.notable_note_zh.as_str(),
                            "同屋家眷師徒" | "日常往來街坊" | "同巷老鄰居"
                        ) {
                            soc.notable_note_zh.clear();
                        }
                        if soc.broken_pacts == 0 && soc.helped_in_crisis == 0 && soc.kept_pacts <= 1
                        {
                            soc.kept_pacts = 0;
                            soc.trade_count = 0;
                        }
                    }
                }
            }
        }
    }

    // ── 第 1 層：同屋至親／師徒「名分 × 脾氣逆鱗親疏分化」（血親亦可能同屋生分！）──
    for (hh_id, mems) in &by_hh {
        let is_shop = hh_id.contains(":shop_");
        for i in 0..mems.len() {
            for j in (i + 1)..mems.len() {
                let (Some(sa), Some(sb)) = (snaps.get(&mems[i]), snaps.get(&mems[j])) else {
                    continue;
                };
                let (mask, lab_ab, lab_ba) = if is_shop {
                    if i == 0 {
                        (tie_flags::MASTER_DISCIPLE, "入室學徒", "授業師父")
                    } else {
                        (tie_flags::MASTER_DISCIPLE, "同鋪師兄弟", "同鋪師兄弟")
                    }
                } else if sa.bond == "同堂伴侶" || sb.bond == "同堂伴侶" {
                    if i == 0 || j == 0 {
                        (tie_flags::IN_LAW, "結髮伴侶", "結髮伴侶")
                    } else if sb.bond.contains("大輩") || sb.bond.contains("小輩") {
                        (tie_flags::BLOOD_KIN, "膝下晚輩", "屋內尊長")
                    } else if sa.bond.contains("大輩") || sa.bond.contains("小輩") {
                        (tie_flags::BLOOD_KIN, "屋內尊長", "膝下晚輩")
                    } else if sb.bond.contains("長老") || sb.bond.contains("老輩") {
                        (tie_flags::IN_LAW, "屋內老輩", "入門伴侶")
                    } else {
                        (tie_flags::IN_LAW, "入門伴侶", "屋內老輩")
                    }
                } else if sb.bond.contains("長老") || sb.bond.contains("老輩") {
                    (tie_flags::BLOOD_KIN, "家中老輩", "當家晚輩")
                } else if sa.bond.contains("長老") || sa.bond.contains("老輩") {
                    (tie_flags::BLOOD_KIN, "膝下孫輩", "家中長老")
                } else if i == 0 && (sb.bond.contains("大輩") || sb.bond.contains("小輩")) {
                    (tie_flags::BLOOD_KIN, "膝下晚輩", "當家尊長")
                } else {
                    (tie_flags::BLOOD_KIN, "同屋手足", "同屋手足")
                };

                let pa = derive_innate_persona(sa.seed);
                let pb = derive_innate_persona(sb.seed);
                let c_ab = pa.interpersonal_compat_delta(&pb);
                let h = pair_hash(&sa.id, &sb.id, 0x101);
                let youngest_by = sa.birth_year.max(sb.birth_year);
                let ev_year = (youngest_by + 14)
                    .min(CURRENT_SOLAR_YEAR - 1)
                    .max(youngest_by);
                let ev_id = format!("weave:hh:{}:{}", sa.id, sb.id);

                if c_ab <= -12 && (h % 3 != 0) {
                    let ep_a = format!(
                        "太陽曆 {} 年前後，我與同屋的{}（{}）因家業規矩與出路主張相左大吵過一場，如今雖同在屋簷下吃飯，平素除了交接活計幾乎不多說半句閒話。",
                        ev_year, sb.name, lab_ab
                    );
                    let ep_b = format!(
                        "太陽曆 {} 年前後，{}（{}）總拿自家死規矩壓人，那回爭執後彼此心頭都留了疙瘩，同住一屋卻頗為生分。",
                        ev_year, sa.name, lab_ba
                    );
                    record_bilateral_life_event(
                        delta,
                        &ev_id,
                        ev_year,
                        &sa.home_node,
                        sa,
                        sb,
                        mask,
                        lab_ab,
                        lab_ba,
                        true,
                        76,
                        -18,
                        -22,
                        Some(ep_a),
                        Some(ep_b),
                        &format!("同屋{}，但因理念相沖長年寡言帶悶氣", lab_ab),
                        &format!("同屋{}，早年爭執後心頭始終隔著一層", lab_ba),
                        "",
                        "",
                    );
                } else {
                    let base_w = if c_ab >= 10 { 36 } else { 18 };
                    let note_a = if c_ab >= 10 {
                        format!("同屋{}，性情投契，遇事最先替對方擔待", lab_ab)
                    } else {
                        format!("同屋{}，日常一塊操持門戶生計", lab_ab)
                    };
                    let note_b = if c_ab >= 10 {
                        format!("同屋{}，平日裡最信得過的自家人", lab_ba)
                    } else {
                        format!("同屋{}，朝夕共撐家業", lab_ba)
                    };
                    record_bilateral_life_event(
                        delta,
                        &ev_id,
                        ev_year,
                        &sa.home_node,
                        sa,
                        sb,
                        mask,
                        lab_ab,
                        lab_ba,
                        true,
                        88,
                        base_w,
                        base_w,
                        None,
                        None,
                        &note_a,
                        &note_b,
                        "",
                        "",
                    );
                }
            }
        }
    }

    let mut safe_clan_groups: BTreeMap<(String, &'static str), Vec<String>> = BTreeMap::new();
    for (aid, s) in &snaps {
        if let Some(clan) = s.clan_opt
            && (CURRENT_SOLAR_YEAR - s.birth_year) >= racial_adult_age(&s.race)
        {
            safe_clan_groups
                .entry((s.race.clone(), clan))
                .or_default()
                .push(aid.clone());
        }
    }

    for ((_race, clan), members) in &safe_clan_groups {
        let m_len = members.len();
        if m_len < 2 {
            continue;
        }
        for idx in 0..m_len {
            let aid_a = &members[idx];
            let Some(sa) = snaps.get(aid_a) else {
                continue;
            };
            // 優先找不同聚落之同氏族親人建立跨鎮血脈！
            let Some(aid_b) = (1..m_len)
                .map(|off| &members[(idx + off) % m_len])
                .find(|cand| {
                    snaps.get(*cand).is_some_and(|sb| {
                        sb.town_slug != sa.town_slug
                            && sb.hh_id != sa.hh_id
                            && bare_given_name(&sb.name) != bare_given_name(&sa.name)
                    })
                })
            else {
                continue;
            };
            if aid_a > aid_b {
                continue;
            }
            let Some(sb) = snaps.get(aid_b) else {
                continue;
            };
            let age_gap = (sa.birth_year - sb.birth_year).abs();
            let (older, younger) = if sa.birth_year <= sb.birth_year {
                (sa, sb)
            } else {
                (sb, sa)
            };
            let (lab_old_to_yng, lab_yng_to_old) = if age_gap <= 15 {
                ("分家外遷的同宗弟妹", "留守別鎮的同宗兄姊")
            } else {
                ("分支落腳外鎮的本家晚輩", "早年分家別鎮的叔舅長輩")
            };
            let h = pair_hash(&older.id, &younger.id, 0x202);
            let split_year =
                (younger.birth_year + racial_adult_age(&younger.race)).min(CURRENT_SOLAR_YEAR - 2);
            let ev_id = format!("weave:clan_split:{}:{}", older.id, younger.id);
            let estranged = h.is_multiple_of(3);
            let (ep_o, ep_y, w_o, w_y, fam, note_o, note_y) = if estranged {
                (
                    format!(
                        "太陽曆 {} 年前後，{}一脈分家時我與{}對家產器具去向意見不合，對方後來落腳{}做{}，這幾年除了逢年託車捎句平安話，少有深談。",
                        split_year, clan, younger.name, younger.town_zh, younger.role
                    ),
                    format!(
                        "太陽曆 {} 年前後，當年不願在舊門戶受氣，與{}分家後獨自在{}憑{}立足，心裡對當年分家舊帳還留著幾分硬氣。",
                        split_year, older.name, younger.town_zh, younger.role
                    ),
                    -22i8,
                    -26i8,
                    24u8,
                    format!(
                        "分家在{}的{}，因當年分家舊隙往來疏淡",
                        younger.town_zh, lab_old_to_yng
                    ),
                    format!(
                        "住在{}的{}，分家後多年少有走動",
                        older.town_zh, lab_yng_to_old
                    ),
                )
            } else {
                (
                    format!(
                        "太陽曆 {} 年前後，同宗的{}前往{}操持{}營生，兩鎮雖隔著山海路途，每逢班車與船班總會互相捎帶特產照應。",
                        split_year, younger.name, younger.town_zh, younger.role
                    ),
                    format!(
                        "太陽曆 {} 年前後，我分支到{}做{}時，多虧在{}的{}照應安家傢伙，至今兩邊常通口信。",
                        split_year, younger.town_zh, younger.role, older.town_zh, older.name
                    ),
                    44i8,
                    48i8,
                    52u8,
                    format!(
                        "落腳{}的{}，常託車船捎信互助",
                        younger.town_zh, lab_old_to_yng
                    ),
                    format!(
                        "身在{}的{}，有難處隨時能投奔的本家親人",
                        older.town_zh, lab_yng_to_old
                    ),
                )
            };
            record_bilateral_life_event(
                delta,
                &ev_id,
                split_year,
                &older.home_node,
                older,
                younger,
                tie_flags::BLOOD_KIN,
                lab_old_to_yng,
                lab_yng_to_old,
                true,
                fam,
                w_o,
                w_y,
                Some(ep_o),
                Some(ep_y),
                &note_o,
                &note_y,
                "",
                "",
            );
        }
    }

    // ── 第 3 層：全島 234 戶「同堂伴侶娘家／外家網」（含 30% 跨鎮通婚，連通子女舅姨線！）──
    let hh_heads_by_race: BTreeMap<String, Vec<String>> = {
        let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for mems in by_hh.values() {
            if let Some(hid) = mems.first()
                && let Some(hs) = snaps.get(hid)
                && !hs.hh_id.contains(":shop_")
                && !hs.hh_id.ends_with(":porter")
            {
                m.entry(hs.race.clone()).or_default().push(hid.clone());
            }
        }
        m
    };

    for mems in by_hh.values() {
        let Some(head_id) = mems.first() else {
            continue;
        };
        let Some(spouse_id) = mems
            .iter()
            .find(|id| snaps.get(*id).is_some_and(|s| s.bond == "同堂伴侶"))
        else {
            continue;
        };
        let (Some(head_s), Some(sp_s)) = (snaps.get(head_id), snaps.get(spouse_id)) else {
            continue;
        };
        let Some(cands) = hh_heads_by_race.get(&sp_s.race) else {
            continue;
        };
        if cands.len() < 2 {
            continue;
        }
        let h = pair_hash(&sp_s.id, "natal_pick", 0x303);
        let want_cross_town = (h % 10) < 3;
        let start_idx = (h as usize) % cands.len();
        let natal_hid_opt = (0..cands.len())
            .map(|off| &cands[(start_idx + off) % cands.len()])
            .find(|cid| {
                snaps.get(*cid).is_some_and(|ns| {
                    ns.hh_id != sp_s.hh_id
                        && bare_given_name(&ns.name) != bare_given_name(&sp_s.name)
                        && (ns.birth_year - sp_s.birth_year).abs() <= 45
                        && (!want_cross_town || ns.town_slug != sp_s.town_slug)
                })
            })
            .or_else(|| {
                (0..cands.len())
                    .map(|off| &cands[(start_idx + off) % cands.len()])
                    .find(|cid| {
                        snaps.get(*cid).is_some_and(|ns| {
                            ns.hh_id != sp_s.hh_id
                                && bare_given_name(&ns.name) != bare_given_name(&sp_s.name)
                        })
                    })
            });

        let Some(natal_hid) = natal_hid_opt else {
            continue;
        };
        let Some(natal_s) = snaps.get(natal_hid) else {
            continue;
        };
        let wed_year =
            (sp_s.birth_year + racial_adult_age(&sp_s.race) + 2).min(CURRENT_SOLAR_YEAR - 2);
        let ev_id = format!("weave:natal:{}:{}", sp_s.id, natal_s.id);
        let ep_sp = format!(
            "太陽曆 {} 年前後，我自{}的{}娘家出門成家，平日裡雖忙著夫家{}生計，逢事仍常記掛娘家那頭的近況。",
            wed_year, natal_s.town_zh, natal_s.name, sp_s.role
        );
        let ep_na = format!(
            "太陽曆 {} 年前後，自家親族{}成家落腳{}，兩家自此成了常走動的實在親戚。",
            wed_year, sp_s.name, sp_s.town_zh
        );
        record_bilateral_life_event(
            delta,
            &ev_id,
            wed_year,
            &sp_s.home_node,
            sp_s,
            natal_s,
            tie_flags::BLOOD_KIN,
            "娘家本門當家",
            "出門成家的本門親族",
            true,
            64,
            42,
            40,
            Some(ep_sp),
            Some(ep_na),
            &format!("{}的娘家當家，遇上家務難處可回門商量", natal_s.town_zh),
            &format!("嫁入{}的自家親族", sp_s.town_zh),
            "",
            "",
        );
        // 屋主與大舅子／姻家當家互連
        record_bilateral_life_event(
            delta,
            &format!("{ev_id}:inlaw"),
            wed_year,
            &head_s.home_node,
            head_s,
            natal_s,
            tie_flags::IN_LAW,
            "伴侶娘家當家",
            "結親姻家屋主",
            true,
            48,
            20,
            20,
            None,
            None,
            &format!("伴侶在{}的娘家當家，逢年節禮數往來", natal_s.town_zh),
            &format!("{}的結親姻家屋主", head_s.town_zh),
            "",
            "",
        );
        // 屋內隨業大輩／小輩連向舅姨外家（讓年輕一代天然擁有跨巷／跨鎮外家親戚！）
        for child_id in mems.iter().skip(1) {
            let Some(cs) = snaps.get(child_id) else {
                continue;
            };
            if !cs.bond.contains("大輩") && !cs.bond.contains("小輩") {
                continue;
            }
            let visit_yr = (cs.birth_year + 10).min(CURRENT_SOLAR_YEAR - 1);
            let c_ev = format!("weave:maternal:{}:{}", cs.id, natal_s.id);
            let ep_c = format!(
                "自記事起便常隨家裡長輩去{}探望外家的{}，偶爾在家裡受了悶氣也會往外家跑。",
                natal_s.town_zh, natal_s.name
            );
            record_bilateral_life_event(
                delta,
                &c_ev,
                visit_yr,
                &natal_s.home_node,
                cs,
                natal_s,
                tie_flags::BLOOD_KIN,
                "外家舅姨尊長",
                "外嫁親族的晚輩",
                true,
                54,
                38,
                42,
                Some(ep_c),
                None,
                &format!("在{}的外家尊長，從小看著自己長大", natal_s.town_zh),
                &format!("住在{}的親族晚輩", cs.town_zh),
                "",
                "",
            );
        }
    }

    // ── 第 4 層：非當家成員個人生命線（老輩患難舊同袍、成年晚輩同門師兄弟與跨巷／跨鎮同輩圈、未成年玩伴）──
    let mut elders: Vec<String> = Vec::new();
    let mut spouses_by_town: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut craft_youth: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    let mut town_youth: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut town_minors: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (aid, s) in &snaps {
        let lived = CURRENT_SOLAR_YEAR - s.birth_year;
        let adult_min = racial_adult_age(&s.race);
        if s.bond.contains("長老") || s.bond.contains("老輩") {
            elders.push(aid.clone());
        } else if s.bond == "同堂伴侶" {
            spouses_by_town
                .entry(s.town_slug.clone())
                .or_default()
                .push(aid.clone());
        } else if lived < adult_min {
            town_minors
                .entry(s.town_slug.clone())
                .or_default()
                .push(aid.clone());
        } else if s.bond.contains("大輩")
            || s.bond.contains("小輩")
            || s.bond == "入室學徒"
            || !s.bond.contains("當家")
        {
            craft_youth
                .entry(craft_bucket(&s.role))
                .or_default()
                .push(aid.clone());
            town_youth
                .entry(s.town_slug.clone())
                .or_default()
                .push(aid.clone());
        }
    }

    // 4a-0. 同堂伴侶獨立交友圈：成家前後各自結識之同鎮手帕／市集摯友（不依附當家屋主！）
    for list in spouses_by_town.values() {
        let n = list.len();
        if n < 2 {
            continue;
        }
        for i in 0..n {
            let j = (i + 1) % n;
            let (Some(pa), Some(pb)) = (snaps.get(&list[i]), snaps.get(&list[j])) else {
                continue;
            };
            if pa.hh_id == pb.hh_id || pa.id >= pb.id {
                continue;
            }
            let yr = (pa.birth_year.max(pb.birth_year) + 18).min(CURRENT_SOLAR_YEAR - 1);
            let ev_id = format!("weave:spouse_peer:{}:{}", pa.id, pb.id);
            let ep = format!(
                "太陽曆 {} 年前後，平日採買與操持{}時與同在{}持家的{}結為私交密友，家裡不便對伴侶直說的煩心事多半找對方商量。",
                yr, pa.role, pa.town_zh, pb.name
            );
            record_bilateral_life_event(
                delta,
                &ev_id,
                yr,
                &pa.home_node,
                pa,
                pb,
                tie_flags::PEER_COHORT,
                "市集手帕密友",
                "市集手帕密友",
                true,
                70,
                46,
                46,
                Some(ep.clone()),
                Some(ep),
                "長年一塊趕集持家的私交密友，比屋裡人更懂彼此難處",
                "長年一塊趕集持家的私交密友，比屋裡人更懂彼此難處",
                "",
                "",
            );
        }
    }

    // 4a. 老輩：壯年過命同袍（含跨鎮老友）
    if elders.len() >= 2 {
        for i in 0..elders.len() {
            let j = (i + 1) % elders.len();
            if elders[i] >= elders[j] {
                continue;
            }
            let (Some(ea), Some(eb)) = (snaps.get(&elders[i]), snaps.get(&elders[j])) else {
                continue;
            };
            if ea.hh_id == eb.hh_id {
                continue;
            }
            let yr = (ea.birth_year.max(eb.birth_year) + 28).min(CURRENT_SOLAR_YEAR - 12);
            let ev_id = format!("weave:elder_sworn:{}:{}", ea.id, eb.id);
            let ep = format!(
                "太陽曆 {} 年前後，年輕氣盛時曾與{}的{}一塊在風暴與山道裡扛過死裡逃生的苦差，如今兩人都已退居老輩，提起來仍是過命交情。",
                yr, eb.town_zh, eb.name
            );
            let ep_rev = format!(
                "太陽曆 {} 年前後，早年曾與{}的{}並肩熬過一場大險，這份老交情比尋常親戚還鐵。",
                yr, ea.town_zh, ea.name
            );
            record_bilateral_life_event(
                delta,
                &ev_id,
                yr,
                &ea.home_node,
                ea,
                eb,
                tie_flags::SWORN_COMRADE,
                "早年過命老友",
                "早年過命老友",
                true,
                56,
                58,
                58,
                Some(ep),
                Some(ep_rev),
                &format!("身在{}的早年過命老友，年輕時曾共患難", eb.town_zh),
                &format!("身在{}的早年過命老友，年輕時曾共患難", ea.town_zh),
                "",
                "",
            );
        }
    }

    // 4b. 成年晚輩與學徒：跨戶／跨鎮「同藝切磋師兄弟」＋「同齡竹馬摯友／暗中較勁」
    for (_cb, list) in &craft_youth {
        let n = list.len();
        if n < 2 {
            continue;
        }
        for i in 0..n {
            let j = (i + 1) % n;
            let (Some(ya), Some(yb)) = (snaps.get(&list[i]), snaps.get(&list[j])) else {
                continue;
            };
            if ya.hh_id == yb.hh_id || ya.id >= yb.id {
                continue;
            }
            let yr = (ya.birth_year.max(yb.birth_year) + racial_adult_age(&ya.race))
                .min(CURRENT_SOLAR_YEAR - 1);
            let h = pair_hash(&ya.id, &yb.id, 0x404);
            let rival = h.is_multiple_of(4);
            let ev_id = format!("weave:cohort:{}:{}", ya.id, yb.id);
            let (ep_a, ep_b, w) = if rival {
                (
                    format!(
                        "太陽曆 {} 年前後，在市集交活時常撞見{}做{}的同輩{}，兩人手藝路數處處較勁，誰也不服誰。",
                        yr, yb.town_zh, yb.role, yb.name
                    ),
                    format!(
                        "太陽曆 {} 年前後，與{}的{}同輩操持手藝，每次碰面都要暗中比一比誰手裡的活更俐落。",
                        yr, ya.town_zh, ya.name
                    ),
                    -16i8,
                )
            } else {
                (
                    format!(
                        "太陽曆 {} 年前後，剛上手做{}時常與{}的同輩{}私下湊在一塊琢磨竅門、互換順手小工具。",
                        yr, ya.role, yb.town_zh, yb.name
                    ),
                    format!(
                        "太陽曆 {} 年前後，學做{}吃苦那陣子常跟{}的{}私下通氣照應，比跟家裡長輩還好說話。",
                        yr, yb.role, ya.town_zh, ya.name
                    ),
                    42i8,
                )
            };
            let lab = if rival {
                "同行較勁同輩"
            } else {
                "同藝切磋摯友"
            };
            record_bilateral_life_event(
                delta,
                &ev_id,
                yr,
                &ya.home_node,
                ya,
                yb,
                tie_flags::MASTER_DISCIPLE | tie_flags::PEER_COHORT,
                lab,
                lab,
                true,
                62,
                w,
                w,
                Some(ep_a),
                Some(ep_b),
                &format!("{}的{}，同輩習藝相熟", yb.town_zh, lab),
                &format!("{}的{}，同輩習藝相熟", ya.town_zh, lab),
                "",
                "",
            );
        }
    }

    for list in town_youth.values() {
        let n = list.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let step = (n / 3).max(2);
            let j = (i + step) % n;
            let (Some(ya), Some(yb)) = (snaps.get(&list[i]), snaps.get(&list[j])) else {
                continue;
            };
            if ya.hh_id == yb.hh_id || ya.id >= yb.id {
                continue;
            }
            let yr = (ya.birth_year.max(yb.birth_year) + 14).min(CURRENT_SOLAR_YEAR - 1);
            let ev_id = format!("weave:peer:{}:{}", ya.id, yb.id);
            let watch_target = if ya.town_slug == "greypine_post" {
                "看驛車"
            } else {
                "看船"
            };
            let ep_a = format!(
                "太陽曆 {} 年前後，打小在{}街巷裡跟{}一塊跑腿{}長大，家裡長輩不讓提的心事多半只跟對方講。",
                yr, ya.town_zh, yb.name, watch_target
            );
            let ep_b = format!(
                "太陽曆 {} 年前後，與同鎮的{}從少年時便常在巷口碰頭閒聊，是知根知底的同輩朋友。",
                yr, ya.name
            );
            record_bilateral_life_event(
                delta,
                &ev_id,
                yr,
                &ya.home_node,
                ya,
                yb,
                tie_flags::PEER_COHORT,
                "總角竹馬同輩",
                "總角竹馬同輩",
                true,
                68,
                46,
                44,
                Some(ep_a),
                Some(ep_b),
                "同鎮一塊長大的總角竹馬，私下無話不談",
                "同鎮一塊長大的總角竹馬，私下極為熟稔",
                "",
                "",
            );
        }
    }

    // 4c. 未滿成年歲數者：僅配對同鎮鄰巷玩伴（嚴禁債務、婚約與暗線！）
    weave_minor_playmates(&town_minors, &snaps, delta);

    // 4d. 鄉野 29 聚落與四港記籌手織入五鎮親師契作網（於二跳三角閉合前執行，確保重複呼叫 100% 冪等不動點！）
    super::rural_migrate::weave_rural_v5_social_ties(doc, delta);

    // ── 第 5 層：跨鎮商路樞紐 ＋ 二跳鄰居三角閉合（聚類 0.24..=0.55）＋ 稀疏暗線 ──
    weave_brokers_triads_and_secrets(&snaps, delta);

    // ── 第 6 層：合成全島 674 人連貫可讀之個人生平列傳（含少數跨海蒼淵主陸遠親記憶錨點）──
    let mut woven_count = 0usize;
    for aid in snaps.keys() {
        let bio = compose_personal_biography_zh(doc, delta, aid, None);
        if let Some(act) = delta.actors.get_mut(aid) {
            if act.bio_core_zh.is_empty() && act.personal_bio_zh != bio {
                act.personal_bio_zh = bio;
                act.rev = act.rev.saturating_add(1);
                act.dirty = true;
            }
            woven_count += 1;
        }
    }

    woven_count
}
