//! 銀鷗島第四刀：灰耳部族營寨與缺糧／破礦劫掠屯贓、破屋舊巢佔據與落敗原石異變頭目引擎。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::combat::wordplate::splitmix64;

use super::clock_and_lang::SolarClockSnapshot;
use super::movement::{add_carry_item, add_language_exposure, push_breath, take_clean_carry_items};
use super::persona::ValueAxis;
use super::sim_tick::broadcast_scene_event;
use super::types::{CarryItem, IslandWorldDelta, SkirmishState, SpatialNodeDef, WorldStaticDoc};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GoblinBandState {
    pub cell_id: u32,
    pub camp_node_id: String,
    pub band_name_zh: String,
    pub headcount: u8,
    pub food_stores: u16,
    pub raid_pressure: u8,
    pub hoard_copper: i64,
    pub hoard_items: Vec<CarryItem>,
    pub suppressed_until_day: i64,
    pub last_raid_day: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SquatterOccupant {
    pub foe_slug: String,
    pub foe_name_zh: String,
    pub can_speak_goblin: bool,
    pub occupied_since_day: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MutatedBossState {
    pub base_foe_slug: String,
    pub boss_name_zh: String,
    pub absorbed_runes: u8,
    pub victim_label_zh: String,
    pub mutated_day: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveWildsState {
    pub seeded: bool,
    pub last_wilds_day: i64,
    pub bands: BTreeMap<u32, GoblinBandState>,
    pub squatters_by_node: BTreeMap<String, SquatterOccupant>,
    pub mutated_boss_by_node: BTreeMap<String, MutatedBossState>,
    pub lair_rotation_seq: u32,
}

impl LiveWildsState {
    pub fn total_band_hoard_copper(&self) -> i64 {
        self.bands.values().map(|b| b.hoard_copper).sum()
    }

    pub fn count_active_runes(&self) -> u32 {
        let mut total = 0u32;
        for b in self.bands.values() {
            for it in &b.hoard_items {
                if it.slug == "rune_stone" {
                    total = total.saturating_add(it.count);
                } else if it
                    .vessel_meta
                    .as_ref()
                    .is_some_and(|m| m.rune_mark.as_deref() == Some("穩固鐵紋"))
                {
                    total = total.saturating_add(it.count);
                }
            }
        }
        for mb in self.mutated_boss_by_node.values() {
            total = total.saturating_add(u32::from(mb.absorbed_runes));
        }
        total
    }
}

const BAND_SPECS: &[(u32, &str, &str, u8)] = &[
    (4323, "oi:cell4323:camp", "古橡坡西・灰耳樹窟營寨", 6),
    (4325, "oi:cell4325:camp", "青苔溪西・灰耳石棚部族", 5),
    (4430, "oi:cell4430:camp", "灰松礦坡・灰耳掘鐵匪眾", 7),
    (4526, "oi:cell4526:camp", "灰松林東・灰耳狩枝部族", 6),
    (4694, "oi:cell4694:camp", "乳酪林谷・灰耳伏哨盜伙", 5),
];

/// 冪等初始化五處內陸灰耳哥布林部族營寨。
pub fn ensure_live_wilds_seeded(doc: &WorldStaticDoc, delta: &mut IslandWorldDelta, game_day: i64) {
    if delta.live_wilds.seeded {
        return;
    }
    let mut bands = BTreeMap::new();
    for &(cid, camp_nid, name_zh, init_head) in BAND_SPECS {
        if !doc.cells.contains_key(&cid.to_string()) || !doc.nodes.contains_key(camp_nid) {
            continue;
        }
        bands.insert(
            cid,
            GoblinBandState {
                cell_id: cid,
                camp_node_id: camp_nid.to_string(),
                band_name_zh: name_zh.to_string(),
                headcount: init_head,
                food_stores: 8,
                raid_pressure: 18,
                hoard_copper: 0,
                hoard_items: Vec::new(),
                suppressed_until_day: 0,
                last_raid_day: -10,
            },
        );
    }
    delta.live_wilds.seeded = true;
    delta.live_wilds.last_wilds_day = game_day;
    delta.live_wilds.bands = bands;
    delta.extras_dirty = true;
}

pub fn has_squatter_at_node<'a>(
    delta: &'a IslandWorldDelta,
    node_id: &str,
) -> Option<&'a SquatterOccupant> {
    delta.live_wilds.squatters_by_node.get(node_id)
}

fn is_edible_cargo_slug(slug: &str) -> bool {
    matches!(
        slug,
        "home_ration"
            | "rye_bread"
            | "wheat_grain"
            | "dairy_cheese"
            | "fresh_fish_basket"
            | "salt_fish_barrel"
            | "shellfish_meat"
    )
}

fn deposit_hoard_item(hoard: &mut Vec<CarryItem>, mut item: CarryItem) {
    item.bound_owner = None;
    item.stolen_from = None;
    add_carry_item(hoard, item);
}

/// 由實地礦木減產、寒冬缺糧推進灰耳部族劫掠壓力，發動真實商車搶掠與破敗空屋／舊祠佔據。
pub fn tick_daily_live_wilds(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    ensure_live_wilds_seeded(doc, delta, clock.game_days);
    if clock.game_days <= delta.live_wilds.last_wilds_day {
        return;
    }

    let start_day = delta.live_wilds.last_wilds_day + 1;
    let end_day = clock.game_days.min(delta.live_wilds.last_wilds_day + 30);

    for d in start_day..=end_day {
        let doy = d.rem_euclid(364);
        let is_winter = doy >= 273;
        let band_cids: Vec<u32> = delta.live_wilds.bands.keys().copied().collect();

        for cid in band_cids {
            let (suppressed_until, camp_nid, need_raid, band_name) = {
                let Some(b) = delta.live_wilds.bands.get_mut(&cid) else {
                    continue;
                };
                if d < b.suppressed_until_day {
                    b.raid_pressure = b.raid_pressure.saturating_sub(6);
                    continue;
                }
                if b.headcount == 0 {
                    b.headcount = 4;
                }
                let consume = if is_winter { 2u16 } else { 1u16 };
                b.food_stores = b.food_stores.saturating_sub(consume);

                let local_stock = delta.cell_stocks.get(&cid).copied().unwrap_or(6);
                let local_density = delta
                    .live_land
                    .cells
                    .get(&cid)
                    .map_or(60, |cl| cl.forest_density);
                let mut delta_p: u8 = 4;
                if b.food_stores <= 2 {
                    delta_p = delta_p.saturating_add(14);
                }
                if is_winter {
                    delta_p = delta_p.saturating_add(8);
                }
                if local_stock <= 2 {
                    delta_p = delta_p.saturating_add(16);
                }
                if local_density <= 30 {
                    delta_p = delta_p.saturating_add(8);
                }
                b.raid_pressure = b.raid_pressure.saturating_add(delta_p).min(100);
                (
                    b.suppressed_until_day,
                    b.camp_node_id.clone(),
                    b.raid_pressure >= 60 && d > b.last_raid_day,
                    b.band_name_zh.clone(),
                )
            };
            if d < suppressed_until || !need_raid {
                continue;
            }

            let adj_cells = doc
                .cells
                .get(&cid.to_string())
                .map_or_else(Vec::new, |c| c.neighbors_f6.clone());

            // A. 若部族山地或鄰格地面留有未拾取之散落行囊，斥候優先摸回營寨窖藏
            let mut scavenged_any = false;
            let loot_nodes: Vec<String> = delta
                .dropped_loot_by_node
                .iter()
                .filter(|(nid, bags)| {
                    !bags.is_empty()
                        && doc
                            .nodes
                            .get(*nid)
                            .is_some_and(|n| n.cell_id == cid || adj_cells.contains(&n.cell_id))
                })
                .map(|(nid, _)| nid.clone())
                .collect();
            for lnid in loot_nodes.into_iter().take(1) {
                if let Some(mut bag) = delta.dropped_loot_by_node.get_mut(&lnid).and_then(Vec::pop)
                {
                    if delta
                        .dropped_loot_by_node
                        .get(&lnid)
                        .is_some_and(Vec::is_empty)
                    {
                        delta.dropped_loot_by_node.remove(&lnid);
                    }
                    let taken_cp = bag.copper;
                    if taken_cp > 0 {
                        delta.goblin_hoard_copper += taken_cp;
                        delta.push_ledger_entry(
                            d,
                            clock.bell_index,
                            "goblin_scavenge_bag",
                            "dropped_loot_escrow",
                            "goblin_hoard",
                            taken_cp,
                            format!("{band_name}斥候摸走遺落行囊內 {taken_cp} 枚銅幣入窖藏"),
                        );
                    }
                    if let Some(b) = delta.live_wilds.bands.get_mut(&cid) {
                        b.hoard_copper += taken_cp;
                        for it in bag.items.drain(..) {
                            if is_edible_cargo_slug(&it.slug) {
                                b.food_stores = b.food_stores.saturating_add((it.count as u16) * 2);
                                b.headcount = b.headcount.saturating_add(1).min(12);
                            } else if it.bound_owner.is_none() {
                                deposit_hoard_item(&mut b.hoard_items, it);
                            }
                        }
                        b.last_raid_day = d;
                        b.raid_pressure = b.raid_pressure.saturating_sub(24);
                    }
                    scavenged_any = true;
                }
            }

            // B. 攔截鄰接幹道上無衛兵／獵戶護行之陸路貨車，奪糧進補並屯贓埋銀
            let mut raided_carriage = false;
            let toll_amt = 2 + i64::from((splitmix64((d as u64) ^ u64::from(cid)) % 4) as u8);
            let target_car = delta
                .transit_vehicles
                .iter()
                .enumerate()
                .find_map(|(v_idx, v)| {
                    if v.mode != "carriage" {
                        return None;
                    }
                    let cur_vnid = v.route_nodes.get(v.route_idx)?;
                    let cur_vcid = doc.nodes.get(cur_vnid).map(|n| n.cell_id)?;
                    if cur_vcid != cid && !adj_cells.contains(&cur_vcid) {
                        return None;
                    }
                    let escorted = v.passengers.iter().any(|pid| {
                        delta.actors.get(pid).is_some_and(|p| {
                            p.alive
                                && (p.role.contains("衛兵")
                                    || p.role.contains("獵")
                                    || p.bound_weapon.rune_mark.is_some())
                        })
                    });
                    if escorted {
                        return None;
                    }
                    let victim_id =
                        v.passengers
                            .iter()
                            .find(|pid| {
                                delta.actors.get(*pid).is_some_and(|p| {
                                    p.controller_id.is_none() && p.copper >= toll_amt
                                })
                            })
                            .cloned()
                            .unwrap_or_else(|| "municipal_treasury".to_string());
                    Some((v_idx, cur_vnid.clone(), victim_id))
                });
            if let Some((v_idx, stop_nid, victim_id)) = target_car {
                let crate_opt = if !delta.transit_vehicles[v_idx].cargo_hold.is_empty() {
                    Some(delta.transit_vehicles[v_idx].cargo_hold.remove(0))
                } else {
                    None
                };
                let coin_ok = delta.transfer_copper(
                    d,
                    clock.bell_index,
                    "goblin_raid_hoard",
                    &victim_id,
                    "goblin_hoard",
                    toll_amt,
                    format!("{band_name}於官道攔車奪走 {toll_amt} 枚銅幣入山窟窖藏"),
                );
                let mut loot_note = Vec::new();
                if coin_ok && let Some(b) = delta.live_wilds.bands.get_mut(&cid) {
                    b.hoard_copper += toll_amt;
                    loot_note.push(format!("{toll_amt} 枚銅幣"));
                }
                if let Some(cr) = crate_opt
                    && let Some(b) = delta.live_wilds.bands.get_mut(&cid)
                {
                    loot_note.push(format!("{}×{}", cr.name_zh, cr.count));
                    if is_edible_cargo_slug(&cr.slug) {
                        b.food_stores = b.food_stores.saturating_add((cr.count as u16) * 3);
                        b.headcount = b.headcount.saturating_add(1).min(12);
                    } else {
                        deposit_hoard_item(
                            &mut b.hoard_items,
                            CarryItem {
                                slug: cr.slug,
                                name: cr.name_zh,
                                count: cr.count,
                                weight_units: 3,
                                ..CarryItem::default()
                            },
                        );
                    }
                }
                if !loot_note.is_empty() {
                    if let Some(b) = delta.live_wilds.bands.get_mut(&cid) {
                        b.last_raid_day = d;
                        b.raid_pressure = b.raid_pressure.saturating_sub(34);
                    }
                    let note = format!(
                        "{}趁無人護車在路段邊攔截篷車，搶走{}遁回山窟！",
                        band_name,
                        loot_note.join("與")
                    );
                    broadcast_scene_event(
                        doc,
                        delta,
                        clock,
                        &stop_nid,
                        "municipal_treasury",
                        &band_name,
                        super::mind::EventPublicity::PublicLoud,
                        Some("harbor_trade"),
                        72,
                        55,
                        ValueAxis::SurvivalSafety,
                        "goblin_road_raid",
                        &note,
                    );
                    let _ = camp_nid;
                    raided_carriage = true;
                }
            }

            // C. 若周邊無車可攔，從野地／鄉莊籌措 2 枚散錢埋入營地並發出警報
            if !raided_carriage && !scavenged_any {
                let fallback_amt = 2i64;
                if delta.municipal_treasury_copper >= fallback_amt
                    && delta.transfer_copper(
                        d,
                        clock.bell_index,
                        "goblin_raid_hoard",
                        "municipal_treasury",
                        "goblin_hoard",
                        fallback_amt,
                        format!("{band_name}下山騷擾道旁攤擔掠去 2 枚銅幣"),
                    )
                {
                    if let Some(b) = delta.live_wilds.bands.get_mut(&cid) {
                        b.hoard_copper += fallback_amt;
                        b.last_raid_day = d;
                        b.raid_pressure = b.raid_pressure.saturating_sub(26);
                    }
                }
            }
        }

        // D. 失修塌落之空屋（house_integrity < 30）與散瘴舊祠廢墟：依日序自然遷入佔巢賊獸
        if d.rem_euclid(4) == 0 {
            let mut ruined_candidates: Vec<(String, bool)> = doc
                .nodes
                .iter()
                .filter_map(|(nid, nd)| {
                    if nd.kind == "house_vacant"
                        && super::live_land::house_is_ruined(delta, nid)
                        && !delta.live_wilds.squatters_by_node.contains_key(nid)
                    {
                        Some((nid.clone(), false))
                    } else if nd.kind == "plague_ruin"
                        && nd.id.ends_with(":ruin_hall")
                        && delta
                            .live_land
                            .cells
                            .get(&nd.cell_id)
                            .is_some_and(|cl| cl.plague_miasmas < 20)
                        && !delta.live_wilds.squatters_by_node.contains_key(nid)
                    {
                        Some((nid.clone(), true))
                    } else {
                        None
                    }
                })
                .collect();
            ruined_candidates.sort_by(|a, b| a.0.cmp(&b.0));
            if let Some((sq_nid, is_ruin)) = ruined_candidates.first() {
                let h = splitmix64((d as u64) ^ 0x51AA_77E1);
                let (fslug, fname, speak) = if *is_ruin || h % 2 == 0 {
                    ("cave_lizard", "潛入斷牆築巢的噬木岩蜥", false)
                } else {
                    ("grey_ear_goblin", "竄入破屋翻找鐵釘的灰耳拾荒賊", true)
                };
                delta.live_wilds.squatters_by_node.insert(
                    sq_nid.clone(),
                    SquatterOccupant {
                        foe_slug: fslug.to_string(),
                        foe_name_zh: fname.to_string(),
                        can_speak_goblin: speak,
                        occupied_since_day: d,
                    },
                );
            }
        }
    }

    delta.live_wilds.last_wilds_day = end_day;
    delta.extras_dirty = true;
}

/// 試煉坑元祖吞石巨蛛伏誅後，依周邊部族壓力與季節輪替遷入佔巢異獸／掘礦隊（絕不重複同一套罐頭怪）。
pub fn pick_dynamic_pit_lair_occupant(
    delta: &mut IslandWorldDelta,
    game_days: i64,
) -> (&'static str, String, bool, String) {
    delta.live_wilds.lair_rotation_seq = delta.live_wilds.lair_rotation_seq.saturating_add(1);
    delta.extras_dirty = true;
    let high_goblin_pressure = delta
        .live_wilds
        .bands
        .values()
        .any(|b| b.raid_pressure >= 48 && game_days >= b.suppressed_until_day);
    let pick = if high_goblin_pressure && delta.live_wilds.lair_rotation_seq % 2 == 1 {
        0
    } else {
        (delta.live_wilds.lair_rotation_seq + (game_days.rem_euclid(3) as u32)) % 3
    };
    match pick {
        0 => (
            "grey_ear_goblin",
            "潛入斷柱坑道撬礦石的灰耳掘礦隊".to_string(),
            true,
            "廢棄巨蛛巢穴裡火把搖曳，幾名【潛入斷柱坑道撬礦石的灰耳掘礦隊】正拎著短鎬與麻袋轉身盯住你！".to_string(),
        ),
        1 => (
            "cave_lizard",
            "循石縫地熱遷入的晶背岩蜥".to_string(),
            false,
            "碎裂蛛網深處傳來粗重鱗爪刮石聲，一頭體背凝著灰白礦斑的【循石縫地熱遷入的晶背岩蜥】低伏身軀猛撲而來！".to_string(),
        ),
        _ => (
            "cave_lizard",
            "佔據幽暗岩穴啃骨的鐵鬃山豺群".to_string(),
            false,
            "洞口血腥未散，幾頭順著氣味鑽進石巢的【佔據幽暗岩穴啃骨的鐵鬃山豺群】齜著利齒封住退路！".to_string(),
        ),
    }
}

/// 落敗掉包時若遺落未綁定「古符文原石」，魔物／部族當場奪石吞噬並異變為具名頭目駐守原地。
pub fn try_absorb_dropped_rune_into_mutated_boss(
    delta: &mut IslandWorldDelta,
    node_id: &str,
    foe_slug: &str,
    foe_name: &str,
    dropped_items: &mut Vec<CarryItem>,
    victim_label_zh: &str,
    game_days: i64,
) -> Option<String> {
    let idx = dropped_items
        .iter()
        .position(|i| i.slug == "rune_stone" && i.count > 0 && i.bound_owner.is_none())?;
    dropped_items[idx].count -= 1;
    if dropped_items[idx].count == 0 {
        dropped_items.remove(idx);
    }

    let boss_name = if foe_name.contains("異變") {
        foe_name.to_string()
    } else {
        format!("吞石異變的{}", foe_name)
    };
    let entry = delta
        .live_wilds
        .mutated_boss_by_node
        .entry(node_id.to_string())
        .or_insert_with(|| MutatedBossState {
            base_foe_slug: foe_slug.to_string(),
            boss_name_zh: boss_name.clone(),
            absorbed_runes: 0,
            victim_label_zh: victim_label_zh.to_string(),
            mutated_day: game_days,
        });
    entry.absorbed_runes = entry.absorbed_runes.saturating_add(1);
    entry.mutated_day = game_days;
    delta.extras_dirty = true;
    Some(entry.boss_name_zh.clone())
}

pub fn live_wilds_scene_prose_suffix(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(band) = delta
        .live_wilds
        .bands
        .values()
        .find(|b| b.camp_node_id == node.id)
    {
        if clock.game_days < band.suppressed_until_day {
            parts.push(format!(
                "火塘灰燼早被踢散，【{}】遭人連根端掉後暫時潰散避入深林",
                band.band_name_zh
            ));
        } else {
            let loot_hint = if band.hoard_copper > 0 || !band.hoard_items.is_empty() {
                "營帳後石坑新覆著浮土，隱約埋著搶掠來的銅錢與貨物箱袋"
            } else if band.raid_pressure >= 55 {
                "營地裡乾糧見底，幾名持骨刀的斥候正磨刃盯著山下官道方向"
            } else {
                "石縫火塘旁散著啃剩獸骨與木柴，幾雙暗處眼珠正打量來客"
            };
            parts.push(format!("{}。", loot_hint));
        }
    }
    if let Some(sq) = delta.live_wilds.squatters_by_node.get(&node.id) {
        parts.push(format!(
            "屋內陰影中傳來抓撓與低吼：裡頭已被【{}】盤踞作窩",
            sq.foe_name_zh
        ));
    }
    if let Some(mb) = delta.live_wilds.mutated_boss_by_node.get(&node.id) {
        parts.push(format!(
            "前方岩影裡伏著一頭體軀膨脹泛光的【{}】，顯然剛吞下{}遺落的古符文原石正躁動喘息",
            mb.boss_name_zh, mb.victim_label_zh
        ));
    }
    let _ = doc;
    if parts.is_empty() {
        String::new()
    } else {
        format!(" {}", parts.join("；"))
    }
}

pub fn append_live_wilds_scene_objects<F>(
    delta: &IslandWorldDelta,
    viewer: &super::types::UnifiedMindBody,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
    objects: &mut Vec<Value>,
    mut push_audit: F,
) where
    F: FnMut(&str, &str, &str, &str, &str, &str),
{
    if let Some(band) = delta
        .live_wilds
        .bands
        .values()
        .find(|b| b.camp_node_id == node.id)
        && clock.game_days >= band.suppressed_until_day
    {
        let mut opts = vec![json!({
            "id": "raid_goblin_camp",
            "param": band.cell_id.to_string(),
            "label": format!("拔械清剿【{}】", band.band_name_zh),
            "sub": format!("逐散 {} 名部族賊眾並奪回埋藏贓物", band.headcount),
        })];
        push_audit(
            "raid_goblin_camp",
            &band.cell_id.to_string(),
            "拔械清剿部族營寨",
            "迎戰頭目奪回山窟窖藏",
            "object",
            "obj:goblin_camp_hoard",
        );
        let has_two_bread = viewer
            .backpack
            .iter()
            .filter(|i| i.slug == "rye_bread" && i.count > 0)
            .map(|i| i.count)
            .sum::<u32>()
            >= 2;
        if has_two_bread {
            opts.push(json!({
                "id": "parley_goblin_camp",
                "param": band.cell_id.to_string(),
                "label": "拋出兩份乾糧以部族語講數",
                "sub": "壓低劫掠戾氣免戰抽身",
            }));
            push_audit(
                "parley_goblin_camp",
                &band.cell_id.to_string(),
                "以乾糧部族語與山窟講數",
                "暫消山下官道劫掠",
                "object",
                "obj:goblin_camp_hoard",
            );
        }
        objects.push(json!({
            "object_id": "obj:goblin_camp_hoard",
            "name": "灰耳部族火塘與浮土石坑",
            "cue": if band.hoard_copper > 0 || !band.hoard_items.is_empty() {
                format!("坑內埋有掠奪來的 {} 枚銅幣與貨袋", band.hoard_copper)
            } else {
                format!("營地聚眾約 {} 名，山風吹著粗木籬柵", band.headcount)
            },
            "desc": "灰耳部族圍著石灶火塘紮營，靠岩壁掘了個掩土窖坑藏放搶來的銅幣與礦木雜貨。",
            "direct_action": Value::Null,
            "options": opts,
        }));
    }

    if let Some(sq) = delta.live_wilds.squatters_by_node.get(&node.id) {
        objects.push(json!({
            "object_id": "obj:squatter_nest",
            "name": "斷牆裡的亂草舊窩",
            "cue": format!("裡面蹲伏著【{}】", sq.foe_name_zh),
            "desc": "久無人居的朽樑斷牆被叼進大批乾草碎骨築成窩巢，不先趕走裡頭佔窩的傢伙根本沒法修屋入住。",
            "direct_action": Value::Null,
            "options": [{
                "id": "evict_squatter",
                "param": node.id.clone(),
                "label": format!("持械驅逐【{}】", sq.foe_name_zh),
                "sub": "出手清出破屋／舊祠內窩巢",
            }],
        }));
        push_audit(
            "evict_squatter",
            &node.id,
            "持械驅逐盤踞窩巢者",
            "清理破屋佔據危患",
            "object",
            "obj:squatter_nest",
        );
    }

    if let Some(mb) = delta.live_wilds.mutated_boss_by_node.get(&node.id) {
        objects.push(json!({
            "object_id": "obj:mutated_threat",
            "name": mb.boss_name_zh.clone(),
            "cue": format!("脊骨縫間嵌著 {} 塊失落的古符文原石", mb.absorbed_runes),
            "desc": "這頭原本尋常的凶獸吞下他人遺落的古符文原石後體格暴漲，正守在現場虎視眈眈。",
            "direct_action": Value::Null,
            "options": [{
                "id": "challenge_mutated_boss",
                "param": node.id.clone(),
                "label": format!("正面迎戰【{}】", mb.boss_name_zh),
                "sub": "擊潰異變凶獸奪回被吞原石",
            }],
        }));
        push_audit(
            "challenge_mutated_boss",
            &node.id,
            "迎戰吞石異變凶獸",
            "擊潰並撬回古符文原石",
            "object",
            "obj:mutated_threat",
        );
    }
}

pub fn try_apply_live_wilds_action(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    action: &str,
    param: &str,
) -> Option<Result<String, String>> {
    match action {
        "raid_goblin_camp" => {
            ensure_live_wilds_seeded(doc, delta, clock.game_days);
            let target_cid = param.trim().parse::<u32>().ok().unwrap_or(node.cell_id);
            let Some(band) = delta
                .live_wilds
                .bands
                .get(&target_cid)
                .filter(|b| b.camp_node_id == node.id)
                .cloned()
            else {
                return Some(Err("眼前並無可清剿的灰耳部族營寨。".to_string()));
            };
            if clock.game_days < band.suppressed_until_day {
                return Some(Err("這處山窟營寨早已被打散，此刻空無一人。".to_string()));
            }
            let can_gob = delta
                .actors
                .get(aid)
                .is_some_and(|a| a.languages.get("goblin").copied().unwrap_or(0) >= 25);
            if let Some(a) = delta.actors.get_mut(aid) {
                a.active_skirmish = Some(SkirmishState {
                    foe_slug: format!("goblin_camp_chief:{target_cid}"),
                    foe_name: format!("{}・持盾頭目", band.band_name_zh),
                    can_speak_goblin: can_gob,
                    foe_stagger: 0,
                    player_guard: false,
                    round_idx: 1,
                });
                a.mark_mutated();
                let msg = format!(
                    "你踏破草柵直衝火塘，【{}・持盾頭目】揮動骨斧嚎叫迎上！",
                    band.band_name_zh
                );
                push_breath(a, msg.clone());
                return Some(Ok(msg));
            }
            Some(Err("找不到旅人行蹤。".to_string()))
        }
        "parley_goblin_camp" => {
            ensure_live_wilds_seeded(doc, delta, clock.game_days);
            let target_cid = param.trim().parse::<u32>().ok().unwrap_or(node.cell_id);
            let Some(band) = delta
                .live_wilds
                .bands
                .get(&target_cid)
                .filter(|b| b.camp_node_id == node.id)
                .cloned()
            else {
                return Some(Err("此處並無部族營寨可上前講數。".to_string()));
            };
            if clock.game_days < band.suppressed_until_day {
                return Some(Err("營地已散，火塘只餘冷灰。".to_string()));
            }
            let gob_prof = delta
                .actors
                .get(aid)
                .and_then(|a| a.languages.get("goblin").copied())
                .unwrap_or(0);
            if gob_prof < 25 {
                return Some(Err(
                    "你還不懂哥布林部族語，冒然遞糧過去只會被當成肥羊圍搶（須先略辨部族語詞）。"
                        .to_string(),
                ));
            }
            let swap_item = {
                let Some(a) = delta.actors.get_mut(aid) else {
                    return Some(Err("找不到旅人行蹤。".to_string()));
                };
                if take_clean_carry_items(&mut a.backpack, "rye_bread", 2).is_none() {
                    return Some(Err("講數換和需備足 2 份【黑麥乾糧】。".to_string()));
                }
                let _ = add_language_exposure(a, clock.game_days, "goblin", 10);
                a.mark_mutated();
                delta.live_wilds.bands.get_mut(&target_cid).and_then(|b| {
                    b.food_stores = b.food_stores.saturating_add(4);
                    b.raid_pressure = 0;
                    (!b.hoard_items.is_empty()).then(|| b.hoard_items.remove(0))
                })
            };
            delta.extras_dirty = true;
            let extra = if let Some(it) = swap_item {
                let iname = it.name.clone();
                if let Some(a) = delta.actors.get_mut(aid) {
                    add_carry_item(&mut a.backpack, it);
                }
                format!("並自坑邊踢出 1 件【{iname}】作回禮，")
            } else {
                String::new()
            };
            let msg = format!(
                "你拋出兩袋黑麥乾糧並用部族語低喝議和，【{}】首領嗅完麵包後點頭收兵，{}這幾日不再攔路騷擾山下官道。",
                band.band_name_zh, extra
            );
            if let Some(a) = delta.actors.get_mut(aid) {
                push_breath(a, msg.clone());
            }
            Some(Ok(msg))
        }
        "evict_squatter" => {
            let target_nid = if !param.trim().is_empty() {
                param.trim()
            } else {
                node.id.as_str()
            };
            let Some(sq) = delta.live_wilds.squatters_by_node.get(target_nid).cloned() else {
                return Some(Err("屋內已被清空，並無野獸賊人佔據。".to_string()));
            };
            if target_nid != node.id {
                return Some(Err("須走近門內才能動手驅逐裡頭的傢伙。".to_string()));
            }
            if let Some(a) = delta.actors.get_mut(aid) {
                a.active_skirmish = Some(SkirmishState {
                    foe_slug: format!("squatter:{target_nid}"),
                    foe_name: sq.foe_name_zh.clone(),
                    can_speak_goblin: sq.can_speak_goblin,
                    foe_stagger: 0,
                    player_guard: false,
                    round_idx: 1,
                });
                a.mark_mutated();
                let msg = format!(
                    "你抄起掌中傢伙逼進斷牆角落，驚動裡頭的【{}】竄起撲來！",
                    sq.foe_name_zh
                );
                push_breath(a, msg.clone());
                return Some(Ok(msg));
            }
            Some(Err("找不到旅人行蹤。".to_string()))
        }
        "challenge_mutated_boss" => {
            let Some(mb) = delta.live_wilds.mutated_boss_by_node.get(&node.id).cloned() else {
                return Some(Err("這處已無吞石異變凶獸盤踞。".to_string()));
            };
            if let Some(a) = delta.actors.get_mut(aid) {
                a.active_skirmish = Some(SkirmishState {
                    foe_slug: format!("mutated_boss:{}", node.id),
                    foe_name: mb.boss_name_zh.clone(),
                    can_speak_goblin: false,
                    foe_stagger: 0,
                    player_guard: false,
                    round_idx: 1,
                });
                a.mark_mutated();
                let msg = format!(
                    "你跨步喝破岩影，脊背泛著符文冷光的【{}】猛然轉身撲殺過來！",
                    mb.boss_name_zh
                );
                push_breath(a, msg.clone());
                return Some(Ok(msg));
            }
            Some(Err("找不到旅人行蹤。".to_string()))
        }
        _ => None,
    }
}

/// 結算擊敗部族營寨頭目、佔屋賊獸或吞石異變頭目後的追贓過帳、清場與撬回原石。
pub fn try_settle_wilds_skirmish_victory(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node_id: &str,
    foe_slug: &str,
    foe_name: &str,
) -> Option<Result<String, String>> {
    if let Some(cid_str) = foe_slug.strip_prefix("goblin_camp_chief:") {
        let cid = cid_str.parse::<u32>().unwrap_or(0);
        let (reclaimed_cp, reclaimed_items, band_name) =
            if let Some(b) = delta.live_wilds.bands.get_mut(&cid) {
                let cp = b.hoard_copper;
                let items: Vec<CarryItem> = b.hoard_items.drain(..).collect();
                b.hoard_copper = 0;
                b.raid_pressure = 0;
                b.food_stores = b.food_stores.min(2);
                b.headcount = 2;
                b.suppressed_until_day = clock.game_days + 10;
                (cp, items, b.band_name_zh.clone())
            } else {
                (0, Vec::new(), foe_name.to_string())
            };
        if reclaimed_cp > 0 {
            let _ = delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "goblin_hoard_reclaim",
                "goblin_hoard",
                aid,
                reclaimed_cp,
                format!("踏平{band_name}營寨掘回山窟窖藏 {reclaimed_cp} 枚銅幣"),
            );
        }
        let item_names: Vec<String> = reclaimed_items.iter().map(|i| i.name.clone()).collect();
        let pname = delta
            .actors
            .get(aid)
            .map_or("旅人".to_string(), |a| a.display_name.clone());
        if let Some(a) = delta.actors.get_mut(aid) {
            for it in reclaimed_items {
                add_carry_item(&mut a.backpack, it);
            }
            a.mark_mutated();
        }
        delta.extras_dirty = true;
        broadcast_scene_event(
            doc,
            delta,
            clock,
            node_id,
            aid,
            &pname,
            super::mind::EventPublicity::PublicLoud,
            Some("harbor_trade"),
            80,
            60,
            ValueAxis::SurvivalSafety,
            "raid_goblin_camp_win",
            &format!("{pname}殺散{band_name}賊眾，從浮土石坑掘回 {reclaimed_cp} 枚被掠銅幣"),
        );
        let mut loot_clauses = Vec::new();
        if reclaimed_cp > 0 {
            loot_clauses.push(format!("{reclaimed_cp} 枚藏匿銅幣"));
        }
        if !item_names.is_empty() {
            loot_clauses.push(format!("【{}】", item_names.join("、")));
        }
        let tail = if loot_clauses.is_empty() {
            "這帶官道幾日內可保太平。".to_string()
        } else {
            format!("並從火塘浮土坑裡起出{}！", loot_clauses.join("與"))
        };
        let msg = format!("你一舉擊潰【{}】令賊眾四散奔逃，{tail}", foe_name);
        if let Some(a) = delta.actors.get_mut(aid) {
            push_breath(a, msg.clone());
        }
        return Some(Ok(msg));
    }

    if let Some(sq_nid) = foe_slug.strip_prefix("squatter:") {
        delta.live_wilds.squatters_by_node.remove(sq_nid);
        delta.extras_dirty = true;
        let msg = format!(
            "你揮械將盤踞屋內的【{}】狠狠逐出斷牆，屋舍裡外終於重歸清靜，可著手修繕落戶了。",
            foe_name
        );
        if let Some(a) = delta.actors.get_mut(aid) {
            push_breath(a, msg.clone());
        }
        return Some(Ok(msg));
    }

    if let Some(mb_nid) = foe_slug.strip_prefix("mutated_boss:") {
        let recovered = delta
            .live_wilds
            .mutated_boss_by_node
            .remove(mb_nid)
            .map_or(1, |m| u32::from(m.absorbed_runes.max(1)));
        delta.extras_dirty = true;
        let pname = delta
            .actors
            .get(aid)
            .map_or("旅人".to_string(), |a| a.display_name.clone());
        if let Some(a) = delta.actors.get_mut(aid) {
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: "rune_stone".to_string(),
                    name: "古符文原石".to_string(),
                    count: recovered,
                    weight_units: 2,
                    ..CarryItem::default()
                },
            );
            a.mark_mutated();
        }
        broadcast_scene_event(
            doc,
            delta,
            clock,
            node_id,
            aid,
            &pname,
            super::mind::EventPublicity::PublicLoud,
            None,
            84,
            58,
            ValueAxis::CraftPride,
            "slay_mutated_boss",
            &format!("{pname}斬倒異變的{foe_name}並撬回 {recovered} 塊古符文原石"),
        );
        let msg = format!(
            "【{}】轟然倒地不再動彈，你從牠脊骨硬甲裂縫間撬回 {} 塊溫潤的【古符文原石】！",
            foe_name, recovered
        );
        if let Some(a) = delta.actors.get_mut(aid) {
            push_breath(a, msg.clone());
        }
        return Some(Ok(msg));
    }

    None
}
