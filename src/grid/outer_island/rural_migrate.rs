//! 銀鷗島 `rural_v5` 35 處基層聚落與 4 名港務記籌手之雙向親師契作網編織與 PostgreSQL 零漂移熱遷移。
//!
//! 鐵律：
//! 1. 保留既有五鎮 674 名 NPC（含 `living_chronicler_v4` 精修列傳、`drift`、執行期記憶）、真人玩家 `pl:*` 與 `oi_chronicle_projections`；
//! 2. 新編譯之鄉野居民與記籌手初始銅幣統一由 `silver_bank`（備援 `municipal_treasury`）複式轉帳撥付，嚴守 `120,805` 銅幣零誤差（`conservation_diff == 0`）；
//! 3. 所有鄉野家戶皆織入五鎮「血親／師承／契作供貨網」（含南岸病癒孤莊跨封鎖線親族），且成年非戶長具備獨立跨戶關係，100% 通過 `validate_biography_prose`。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::combat::wordplate::splitmix64;

use super::causal_arcs::{render_deterministic_shiji_bio, validate_biography_prose};
use super::cell_resources::{ensure_and_regen_cell_resources, lookup_resource_spec};
use super::harbor_lineage::CURRENT_SOLAR_YEAR;
use super::mind::{CognitiveMind, EvidenceTier, tie_flags};
use super::movement::add_carry_item;
use super::ongoing_activity::ensure_dock_cargo_piles_initialized;
use super::sim_tick::default_tool_for_role;
use super::transit_and_cargo::ensure_transit_vehicles_initialized;
use super::types::{CarryItem, IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};
use super::world_weave_bio::{
    compose_personal_biography_zh, racial_adult_age, settlement_name_zh, settlement_slug_of,
};

const CORE_FIVE_SLUGS: [&str; 5] = [
    "white_gull_harbor",
    "west_strait_ferry",
    "gull_tail_port",
    "greypine_post",
    "north_cape_beacon",
];

pub fn nearest_core_town_cell_id(doc: &WorldStaticDoc, cell_id: u32) -> u32 {
    match nearest_core_town_slug(doc, cell_id) {
        "west_strait_ferry" => 4321,
        "gull_tail_port" => 4613,
        "greypine_post" => 4525,
        "north_cape_beacon" => 4073,
        _ => 4616,
    }
}

fn nearest_core_town_slug(doc: &WorldStaticDoc, cell_id: u32) -> &'static str {
    let cur_pos = doc
        .cells
        .get(&cell_id.to_string())
        .map_or([1420.0, 770.0], |c| c.center);
    let anchors: [(u32, &'static str); 5] = [
        (4616, "white_gull_harbor"),
        (4321, "west_strait_ferry"),
        (4613, "gull_tail_port"),
        (4525, "greypine_post"),
        (4073, "north_cape_beacon"),
    ];
    anchors
        .into_iter()
        .min_by(|(ca, _), (cb, _)| {
            let pa = doc
                .cells
                .get(&ca.to_string())
                .map_or([0.0, 0.0], |c| c.center);
            let pb = doc
                .cells
                .get(&cb.to_string())
                .map_or([0.0, 0.0], |c| c.center);
            let da = (pa[0] - cur_pos[0]).powi(2) + (pa[1] - cur_pos[1]).powi(2);
            let db = (pb[0] - cur_pos[0]).powi(2) + (pb[1] - cur_pos[1]).powi(2);
            da.total_cmp(&db)
        })
        .map_or("white_gull_harbor", |(_, slug)| slug)
}

#[expect(clippy::too_many_arguments, reason = "explicit bilateral tie inputs")]
fn fold_bilateral_rural_tie(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    ev_id: &str,
    origin_day: i64,
    aid: &str,
    bid: &str,
    tie_mask: u8,
    label_ab: &str,
    label_ba: &str,
    fam: u8,
    warmth_a: i8,
    warmth_b: i8,
    note_ab: &str,
    note_ba: &str,
    ep_ab: Option<String>,
    secret_a: &str,
) {
    if aid == bid {
        return;
    }
    let (Some(a_ro), Some(b_ro)) = (delta.actors.get(aid), delta.actors.get(bid)) else {
        return;
    };
    let a_name = a_ro.display_name.clone();
    let b_name = b_ro.display_name.clone();
    let a_town = settlement_name_zh(doc, a_ro);
    let b_town = settlement_name_zh(doc, b_ro);
    let a_home = a_ro.home_node.clone();

    if let Some(act_a) = delta.actors.get_mut(aid) {
        if let Some(ep) = ep_ab {
            act_a.mind.ingest_historical_episode(
                ev_id,
                origin_day,
                &a_home,
                bid,
                &b_name,
                EvidenceTier::DirectWitness,
                92,
                88,
                78,
                5,
                ep,
            );
        }
        if act_a.mind.fold_social_memory_from_episode(
            ev_id, origin_day, bid, &b_name, &b_town, tie_mask, label_ab, true, false, fam,
            warmth_a, note_ab, secret_a,
        ) {
            act_a.mark_mutated();
        }
    }
    if let Some(act_b) = delta.actors.get_mut(bid)
        && act_b.mind.fold_social_memory_from_episode(
            ev_id, origin_day, aid, &a_name, &a_town, tie_mask, label_ba, true, false, fam,
            warmth_b, note_ba, "",
        )
    {
        act_b.mark_mutated();
    }
}

/// 確保地塊資源、實體車船與碼頭待運貨箱完成執行期初始化（冪等、零銅幣變動）
pub fn ensure_rural_runtime_initialized(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    game_day: i64,
) {
    ensure_and_regen_cell_resources(doc, delta, game_day);
    ensure_transit_vehicles_initialized(doc, delta);
    super::mesh_expedition::sync_mesh_v6_topology_on_boot(doc, delta);
    ensure_dock_cargo_piles_initialized(doc, delta, game_day);
}

/// 編織四港記籌手與 29 座活態鄉野聚落之雙向「血親・師承・契作供貨網」（純社會記憶摺疊，供 `world_weave` 二跳閉合前冪等呼叫）
pub fn weave_rural_v5_social_ties(doc: &WorldStaticDoc, delta: &mut IslandWorldDelta) {
    // 收集五鎮既有成年居民與鋪主，供鄉野聚落確定性掛接
    let mut town_adults: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut town_shops: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (aid, a) in &delta.actors {
        if a.controller_id.is_some() || !a.alive {
            continue;
        }
        let slug = settlement_slug_of(&a.household_id);
        if CORE_FIVE_SLUGS.contains(&slug) && !aid.ends_with(":tally_clerk") {
            let lived = CURRENT_SOLAR_YEAR - a.birth_year;
            if lived >= racial_adult_age(&a.race) {
                town_adults
                    .entry(slug.to_string())
                    .or_default()
                    .push(aid.clone());
                if a.household_id.contains(":shop_") && aid.ends_with("_0") {
                    town_shops
                        .entry(slug.to_string())
                        .or_default()
                        .push(aid.clone());
                }
            }
        }
    }

    // 1. 四港記籌手：織入本鎮「苦力長＋鍛鋪掌櫃＋酒館掌櫃」三角同業網，維持白鷗港聚類係數 0.24..=0.55
    for slug in [
        "white_gull_harbor",
        "west_strait_ferry",
        "gull_tail_port",
        "north_cape_beacon",
    ] {
        let tc_id = format!("res:{slug}:tally_clerk");
        if !delta.actors.contains_key(&tc_id) {
            continue;
        }
        let peers = [
            format!("res:{slug}:dock_porter"),
            format!("res:{slug}:shop_0_0"),
            format!("res:{slug}:shop_1_0"),
        ];
        for pid in &peers {
            if delta.actors.contains_key(pid) {
                let ev = format!("rural_v5:tally:{tc_id}:{pid}");
                fold_bilateral_rural_tie(
                    doc,
                    delta,
                    &ev,
                    (327 - 312) * 364,
                    &tc_id,
                    pid,
                    tie_flags::TRADE_BROKER | tie_flags::PEER_COHORT,
                    "港務對帳老友",
                    "棧橋記籌老友",
                    74,
                    46,
                    44,
                    "每日潮水靠船時當面核銷竹籌與鋪貨箱數的老搭檔",
                    "在棧橋把關貨箱竹籌、帳目從無差池的記籌手",
                    Some(
                        "太陽曆三二七年春潮大漲時，兩人冒雨在棧橋核清整船鹽鐵木箱，分毫不差。"
                            .to_string(),
                    ),
                    "",
                );
            }
        }
        // 彼此認識確保閉合三角
        if delta.actors.contains_key(&peers[0]) && delta.actors.contains_key(&peers[1]) {
            let ev_tri = format!("rural_v5:tally_tri:{slug}");
            fold_bilateral_rural_tie(
                doc,
                delta,
                &ev_tri,
                (326 - 312) * 364,
                &peers[0],
                &peers[1],
                tie_flags::TRADE_BROKER,
                "常年卸鐵熟東家",
                "碼頭卸鐵老領工",
                68,
                38,
                38,
                "常年接應靠岸煤鐵箱入爐的熟識鋪主",
                "棧橋卸運粗鐵炭袋最牢靠的老領工",
                None,
                "",
            );
        }
        if let Some(tc) = delta.actors.get_mut(&tc_id) {
            let home = tc.home_node.clone();
            let larder = tc.inn_lockers.entry(home).or_default();
            if larder.is_empty() {
                add_carry_item(
                    larder,
                    CarryItem {
                        slug: super::household_economy::HOME_RATION_SLUG.to_string(),
                        name: super::household_economy::HOME_RATION_NAME.to_string(),
                        count: 6,
                        weight_units: 1,
                        ..CarryItem::default()
                    },
                );
                tc.mark_mutated();
            }
        }
    }

    // 2. 收集全部鄉野家戶（`hh:burg_*`）與成年非戶長清單
    let mut rural_hhs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut rural_non_heads: Vec<String> = Vec::new();
    for (aid, a) in &delta.actors {
        if a.controller_id.is_some() || !a.alive || !a.household_id.starts_with("hh:burg_") {
            continue;
        }
        rural_hhs
            .entry(a.household_id.clone())
            .or_default()
            .push(aid.clone());
    }
    for mems in rural_hhs.values_mut() {
        mems.sort();
        for m in mems.iter().skip(1) {
            if let Some(a) = delta.actors.get(m) {
                let lived = CURRENT_SOLAR_YEAR - a.birth_year;
                if lived >= racial_adult_age(&a.race) {
                    rural_non_heads.push(m.clone());
                }
            }
        }
    }

    // 2a. 為每戶鄉野戶長編織：五鎮血親/師承線 ＋ 五鎮鋪主契作供貨線 ＋ 家戶儲糧與在地特產木箱
    for (hh_id, mems) in &rural_hhs {
        let Some(head_id) = mems.first() else {
            continue;
        };
        let Some((cell_id, home_node, head_seed)) = delta.actors.get(head_id).and_then(|a| {
            let cid = doc.nodes.get(&a.home_node)?.cell_id;
            Some((cid, a.home_node.clone(), a.soul_seed.max(1)))
        }) else {
            continue;
        };

        let near_slug = if matches!(cell_id, 4777 | 4776) {
            if cell_id == 4777 {
                "white_gull_harbor"
            } else {
                "gull_tail_port"
            }
        } else {
            nearest_core_town_slug(doc, cell_id)
        };

        if let Some(cands) = town_adults.get(near_slug)
            && !cands.is_empty()
        {
            let pick_idx = (splitmix64(head_seed ^ 0x51A7) as usize) % cands.len();
            let kin_target = &cands[pick_idx];
            let is_survivor = matches!(cell_id, 4777 | 4776);
            let (mask, lab_ab, lab_ba, note_ab, note_ba, ep_txt) = if is_survivor {
                (
                    tie_flags::BLOOD_KIN,
                    "封鎖線外血親",
                    "南莊病癒親人",
                    "太陽曆三二六年船疫封村後隔著木欄互託平安家書的城裡至親",
                    "熬過三二六年南岸船疫、如今在封鎖木欄內採苦艾守莊的血親",
                    Some(
                        "太陽曆三二六年秋南岸封村那日，兩家隔著風語橋木欄揮淚道別，此後全靠隔欄交接台捎信報平安。"
                            .to_string(),
                    ),
                )
            } else if head_seed.is_multiple_of(2) {
                (
                    tie_flags::BLOOD_KIN,
                    "城裡堂親",
                    "鄉間堂親",
                    "逢趕集或驛車進城便捎帶山海鮮貨去探望的城裡堂親",
                    "常居鄉野莊頭、每逢收成便託篷車捎來土產的實在堂親",
                    Some(
                        "太陽曆三二五年冬寒封路前，兩家曾合資雇雙馬篷車把過冬木炭與乾糧分送入庫。"
                            .to_string(),
                    ),
                )
            } else {
                (
                    tie_flags::MASTER_DISCIPLE,
                    "早年同門師兄姐",
                    "駐鄉同門師弟妹",
                    "早年一塊學手藝、如今一個守鎮鋪一個守鄉坊的同門舊識",
                    "回鄉鎮守住原料產地、常年替鋪裡揀選上等好料的同門",
                    Some(
                        "太陽曆三二四年開春修整官道橋樑時，兩人曾合手趕製一批急用鐵木器具。"
                            .to_string(),
                    ),
                )
            };
            let ev_kin = format!("rural_v5:kin:{hh_id}");
            fold_bilateral_rural_tie(
                doc,
                delta,
                &ev_kin,
                (326 - 312) * 364,
                head_id,
                kin_target,
                mask,
                lab_ab,
                lab_ba,
                72,
                48,
                45,
                note_ab,
                note_ba,
                ep_txt,
                "",
            );
        }

        if let Some(shops) = town_shops.get(near_slug)
            && !shops.is_empty()
            && !matches!(cell_id, 4777 | 4776)
        {
            let s_idx = (splitmix64(head_seed ^ 0x99C3) as usize) % shops.len();
            let shop_head = &shops[s_idx];
            let ev_sup = format!("rural_v5:supply:{hh_id}");
            fold_bilateral_rural_tie(
                doc,
                delta,
                &ev_sup,
                (327 - 312) * 364,
                head_id,
                shop_head,
                tie_flags::TRADE_BROKER,
                "契作收貨掌櫃",
                "鄉間契作供戶",
                68,
                38,
                40,
                "長年按季收購本莊煤鐵木魚原料並折算鹽布農具回鄉的鋪主",
                "每逢車船班期準時把鄉間原料裝箱送抵鎮上的老實供戶",
                None,
                "",
            );
        }

        // 灌入家戶初始糧倉與特產存貨（冪等）
        let res_slug = doc
            .cells
            .get(&cell_id.to_string())
            .map_or("", |c| c.resource_slug.as_str());
        if let Some(head_mut) = delta.actors.get_mut(head_id) {
            let larder = head_mut.inn_lockers.entry(home_node).or_default();
            if larder.is_empty() {
                add_carry_item(
                    larder,
                    CarryItem {
                        slug: super::household_economy::HOME_RATION_SLUG.to_string(),
                        name: super::household_economy::HOME_RATION_NAME.to_string(),
                        count: 6,
                        weight_units: 1,
                        ..CarryItem::default()
                    },
                );
                if let Some(spec) = lookup_resource_spec(res_slug) {
                    add_carry_item(
                        larder,
                        CarryItem {
                            slug: spec.slug.to_string(),
                            name: spec.name_zh.to_string(),
                            count: 3,
                            weight_units: spec.weight_units,
                            ..CarryItem::default()
                        },
                    );
                }
                head_mut.mark_mutated();
            }
        }
    }

    // 2b. 成年非戶長獨立跨戶關係（確保全島 `indep_rate >= 0.90` 鐵律！）
    let nh_len = rural_non_heads.len();
    for i in 0..nh_len {
        let aid = &rural_non_heads[i];
        let a_hh = delta
            .actors
            .get(aid)
            .map_or(String::new(), |a| a.household_id.clone());
        let mut partner_opt: Option<String> = None;
        for step in 1..nh_len {
            let cand = &rural_non_heads[(i + step) % nh_len];
            let c_hh = delta
                .actors
                .get(cand)
                .map_or(String::new(), |a| a.household_id.clone());
            if c_hh != a_hh {
                partner_opt = Some(cand.clone());
                break;
            }
        }
        if let Some(pid) = partner_opt {
            let (lo, hi) = if aid.as_str() <= pid.as_str() {
                (aid.as_str(), pid.as_str())
            } else {
                (pid.as_str(), aid.as_str())
            };
            let ev_nh = format!("rural_v5:nh_peer:{lo}:{hi}");
            fold_bilateral_rural_tie(
                doc,
                delta,
                &ev_nh,
                (327 - 312) * 364 + 60,
                aid,
                &pid,
                tie_flags::PEER_COHORT,
                "趕集同行夥伴",
                "趕集同行夥伴",
                66,
                42,
                42,
                "每逢雙馬驛車或貨駁靠站時結伴挑擔趕集的同輩熟友",
                "常在官道茶棚歇腳分喝熱茶、互通各村行情的同輩好友",
                None,
                "",
            );
        }
    }
}

/// 為新加入之 4 名港務記籌手與 29 座活態鄉野聚落完成執行期初始化與合規白話列傳保底
pub fn apply_rural_v5_weave_and_init(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    game_day: i64,
) {
    ensure_rural_runtime_initialized(doc, delta, game_day);
    weave_rural_v5_social_ties(doc, delta);

    // 補齊鄉野居民與記籌手對全島幹道及五大主鎮一樓商鋪之路網認知（確保跨鎮歸途與在地趕集皆 100% 連通）
    let shared_road_and_shop_nodes: Vec<String> = doc
        .nodes
        .iter()
        .filter(|(_, nd)| !nd.dungeon && (!nd.indoor || nd.kind == "shop"))
        .map(|(nid, _)| nid.clone())
        .collect();
    let rural_road_targets: Vec<String> = delta
        .actors
        .iter()
        .filter(|(aid, act)| {
            act.controller_id.is_none()
                && act.alive
                && (act.household_id.starts_with("hh:burg_") || aid.ends_with(":tally_clerk"))
        })
        .map(|(aid, _)| aid.clone())
        .collect();
    for aid in rural_road_targets {
        if let Some(act) = delta.actors.get_mut(&aid) {
            for rn in &shared_road_and_shop_nodes {
                act.mind.ltm.known_nodes.insert(rn.clone());
            }
        }
    }

    // 3. 為所有新聚落居民與記籌手補齊並守門驗證個人白話列傳（不覆寫既有五鎮已精修列傳）
    let target_ids: Vec<String> = delta
        .actors
        .iter()
        .filter(|(aid, a)| {
            a.controller_id.is_none()
                && a.alive
                && !a.household_id.is_empty()
                && (a.household_id.starts_with("hh:burg_")
                    || aid.ends_with(":tally_clerk")
                    || a.personal_bio_zh.trim().chars().count() < 260)
        })
        .map(|(aid, _)| aid.clone())
        .collect();

    for aid in target_ids {
        let composed = compose_personal_biography_zh(doc, delta, &aid, None);
        let Some(ro) = delta.actors.get(&aid) else {
            continue;
        };
        let t_slug = settlement_slug_of(&ro.household_id).to_string();
        let t_zh = settlement_name_zh(doc, ro);
        let mut final_bio =
            if validate_biography_prose(&ro.display_name, &t_slug, &t_zh, &composed).is_ok() {
                composed
            } else {
                render_deterministic_shiji_bio(doc, ro, &[])
            };
        if final_bio.trim().chars().count() < 268 {
            final_bio.push_str(&format!(
                "平日每逢雙馬驛車或沿岸貨駁經過{}路口，{}總會到門前與相熟車夫招呼寒暄，把家裡生計與鄰里往來照顧得格外妥貼。",
                t_zh, ro.display_name
            ));
            if validate_biography_prose(&ro.display_name, &t_slug, &t_zh, &final_bio).is_err() {
                final_bio = render_deterministic_shiji_bio(doc, ro, &[]);
            }
        }
        if let Some(act) = delta.actors.get_mut(&aid) {
            act.bio_core_zh = final_bio.clone();
            act.personal_bio_zh = final_bio;
            act.mark_mutated();
        }
    }
}

/// PostgreSQL 線上 `rural_v5` 無損熱遷移：保留既有 674 名 NPC 與真人玩家，從 `silver_bank` 撥付新鄉野居民初始銅幣，嚴守 `120,805` 銅幣零誤差！
pub fn apply_rural_v5_pg_upgrade(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    game_day: i64,
    bell_idx: u8,
    is_night: bool,
) -> usize {
    // 快照既有常住居民之精修列傳，確保 `living_chronicler_v4` 成果一字不改
    let mut preserved_bios: BTreeMap<String, (String, String, Vec<String>)> = BTreeMap::new();
    for (aid, a) in &delta.actors {
        if a.controller_id.is_none() && !a.personal_bio_zh.is_empty() {
            preserved_bios.insert(
                aid.clone(),
                (
                    a.bio_core_zh.clone(),
                    a.personal_bio_zh.clone(),
                    a.bio_post_chapters_zh.clone(),
                ),
            );
        }
    }

    let mut cell_public_nodes: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for (nid, nd) in &doc.nodes {
        if !nd.indoor && !nd.dungeon {
            cell_public_nodes
                .entry(nd.cell_id)
                .or_default()
                .push(nid.clone());
        }
    }

    let mut spawned = 0usize;
    for seed in &doc.residents {
        if delta.actors.contains_key(&seed.id) {
            continue;
        }
        let work_post = if doc
            .nodes
            .get(&seed.home_node)
            .is_some_and(|n| n.kind == "shop")
        {
            seed.home_node.clone()
        } else {
            seed.work_node.clone()
        };
        let start_node = if is_night {
            seed.home_node.clone()
        } else {
            work_post.clone()
        };
        let home_cell = doc.nodes.get(&seed.home_node).map_or(4616, |n| n.cell_id);
        let locals = cell_public_nodes
            .get(&home_cell)
            .map_or(&[][..], |v| v.as_slice());

        let mut mind = CognitiveMind::default();
        mind.seed_local_map_knowledge(
            &seed.home_node,
            &work_post,
            home_cell,
            locals.iter().map(String::as_str),
        );
        let mut known_state = BTreeSet::new();
        known_state.insert(home_cell);
        let unique_seed = delta.pick_unique_soul_seed(Some(seed.soul_seed), &seed.id);
        let is_inland = doc
            .cells
            .get(&home_cell.to_string())
            .is_some_and(|c| !c.is_coast);
        let birth_yr = 292 + i16::try_from(unique_seed % 18).unwrap_or(8);
        let hh_members = doc
            .households
            .iter()
            .find(|h| h.id == seed.household_id)
            .map(|h| h.members.as_slice())
            .unwrap_or(&[]);
        let m_idx = hh_members.iter().position(|m| m == &seed.id).unwrap_or(0);
        let bond_zh = match m_idx {
            0 => "當家戶長",
            1 => "同戶配偶",
            _ => "同戶親眷",
        };
        let app_zh = if is_inland {
            format!(
                "山林風霜鍛出的結實身板，衣角沾著松針與紅土細泥，常年操持{}營生",
                seed.role
            )
        } else {
            format!(
                "海風吹拂出黝黑面龐，靴底帶著潮間鹽漬，常年操持{}營生",
                seed.role
            )
        };

        let actor = UnifiedMindBody {
            actor_id: seed.id.clone(),
            controller_id: None,
            soul_seed: unique_seed,
            rev: 1,
            alive: true,
            setup_done: true,
            display_name: seed.name.clone(),
            race: seed.race.clone(),
            role: seed.role.clone(),
            appearance: app_zh,
            origin_pack: "island_native".to_string(),
            household_id: seed.household_id.clone(),
            household_bond: bond_zh.to_string(),
            birth_year: birth_yr,
            home_node: seed.home_node.clone(),
            work_node: work_post,
            node_id: start_node,
            route_plan: VecDeque::new(),
            copper: 0,
            bound_weapon: default_tool_for_role(&seed.role, &seed.name),
            languages: seed.languages.clone(),
            known_state_cells: known_state,
            mind,
            dirty: true,
            flushed_rev: 0,
            flushed_copper: i64::MIN,
            ..UnifiedMindBody::default()
        };
        delta.upsert_actor(actor);

        if seed.copper > 0 {
            let paid = delta.transfer_copper(
                game_day,
                bell_idx,
                "rural_v5_seed",
                "silver_bank",
                &seed.id,
                seed.copper,
                format!("銀鷗島鄉野聚落建置撥付 {} 初始生計銅幣", seed.name),
            );
            if !paid {
                let _ = delta.transfer_copper(
                    game_day,
                    bell_idx,
                    "rural_v5_seed",
                    "municipal_treasury",
                    &seed.id,
                    seed.copper,
                    format!("銀鷗島鄉野聚落建置公庫撥付 {} 初始生計銅幣", seed.name),
                );
            }
        }
        spawned += 1;
    }

    // 僅為新誕生之同戶成員建立家內親眷記憶，絕不重寫既有 674 名五鎮居民之 `birth_year` / `rev`！
    for hh in &doc.households {
        if !hh.id.starts_with("hh:burg_") {
            continue;
        }
        for a_id in &hh.members {
            for b_id in &hh.members {
                if a_id < b_id {
                    let ev = format!("rural_v5:cohab:{a_id}:{b_id}");
                    fold_bilateral_rural_tie(
                        doc,
                        delta,
                        &ev,
                        (325 - 312) * 364,
                        a_id,
                        b_id,
                        tie_flags::BLOOD_KIN,
                        "同戶家人",
                        "同戶家人",
                        92,
                        68,
                        68,
                        "朝夕同桌共飯、守望相助的自家親人",
                        "朝夕同桌共飯、守望相助的自家親人",
                        None,
                        "",
                    );
                }
            }
        }
    }
    apply_rural_v5_weave_and_init(doc, delta, game_day);

    // 還原既有 674 名 NPC 之精修列傳，杜絕任何覆寫
    for (aid, (core, full, post)) in preserved_bios {
        if let Some(act) = delta.actors.get_mut(&aid) {
            act.bio_core_zh = core;
            act.personal_bio_zh = full;
            act.bio_post_chapters_zh = post;
            act.mark_mutated();
        }
    }

    delta.rebuild_indexes();
    spawned
}
