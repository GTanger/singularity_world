//! 銀鷗島第四刀：居民真實生老病死、遺產屋契／落款兵刃傳承、鋪位扶正、往生名錄與新世代接棒引擎。

use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::combat::wordplate::splitmix64;

use super::causal_arcs::solar_year_zh;
use super::clock_and_lang::{SolarClockSnapshot, solar_year_and_season_of_day};
use super::harbor_lineage::CURRENT_SOLAR_YEAR;
use super::household_economy::{count_household_larder_rations, is_food_producer_role};
use super::live_tide::derive_tide_clock_details;
use super::living_chronicler::{
    ChronicleFactPacket, atmosphere_clause, is_node_coastal, num_zh, town_slug_of_node,
    town_zh_of_slug,
};
use super::mind::{EventPublicity, EvidenceTier, tie_flags};
use super::movement::{add_carry_item, push_breath};
use super::persona::ValueAxis;
use super::sim_tick::{broadcast_scene_event, default_tool_for_role, spawn_dynamic_actors};
use super::types::{
    CarryItem, IslandWorldDelta, NewActorSpawnSpec, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc,
};
use super::world_weave_bio::racial_adult_age;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ObituaryRecord {
    pub deceased_id: String,
    pub deceased_name: String,
    pub race: String,
    pub role: String,
    pub town_slug: String,
    pub solar_year: i16,
    pub season_zh: String,
    pub game_day: i64,
    pub age_at_death: i16,
    pub cause_zh: String,
    pub heir_name: Option<String>,
    pub heirloom_zh: Option<String>,
    pub house_note_zh: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveLifecycleState {
    pub last_lifecycle_day: i64,
    pub obituaries: VecDeque<ObituaryRecord>,
    pub spawned_youth_seq: u32,
}

pub fn current_solar_year(game_days: i64) -> i16 {
    solar_year_and_season_of_day(game_days)
        .0
        .max(CURRENT_SOLAR_YEAR)
}

pub fn actor_lived_years(a: &UnifiedMindBody, game_days: i64) -> i16 {
    (current_solar_year(game_days) - a.birth_year).max(1)
}

fn race_lifespan_bounds(race: &str, soul_seed: u64) -> (i16, i16) {
    let h = splitmix64(soul_seed ^ 0xA6E0_11FE);
    match race {
        "精靈" => (305, 305 + (h % 106) as i16),
        "矮人" => (202, 202 + (h % 47) as i16),
        "獸人" => (68, 68 + (h % 21) as i16),
        _ => (74, 74 + (h % 21) as i16),
    }
}

/// 依戶籍至親 -> 同鋪搭檔學徒 -> 具親眷師徒或高溫厚熟人選出繼承人（絕不納入真人玩家）。
pub fn select_estate_heir(
    delta: &IslandWorldDelta,
    deceased_id: &str,
    game_days: i64,
) -> Option<String> {
    let dec = delta.actors.get(deceased_id)?;
    let dec_hh = dec.household_id.as_str();
    let dec_work = dec.work_node.as_str();

    if !dec_hh.is_empty() {
        let mut hh_cands: Vec<(bool, i16, &String)> = delta
            .actors
            .iter()
            .filter(|(aid, a)| {
                *aid != deceased_id
                    && a.alive
                    && a.controller_id.is_none()
                    && !aid.starts_with("pl:")
                    && a.household_id == dec_hh
            })
            .map(|(aid, a)| {
                let age = actor_lived_years(a, game_days);
                let adult = age >= racial_adult_age(&a.race);
                (adult, age, aid)
            })
            .collect();
        hh_cands.sort_by(|x, y| y.0.cmp(&x.0).then(y.1.cmp(&x.1)).then(x.2.cmp(y.2)));
        if let Some((_, _, aid)) = hh_cands.first() {
            return Some((*aid).clone());
        }
    }

    if !dec_work.is_empty() {
        let mut work_cands: Vec<(bool, &String)> = delta
            .actors
            .iter()
            .filter(|(aid, a)| {
                *aid != deceased_id
                    && a.alive
                    && a.controller_id.is_none()
                    && !aid.starts_with("pl:")
                    && a.work_node == dec_work
            })
            .map(|(aid, a)| (a.role.contains("學徒"), aid))
            .collect();
        work_cands.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(y.1)));
        if let Some((_, aid)) = work_cands.first() {
            return Some((*aid).clone());
        }
    }

    let mut soc_cands: Vec<(i16, &String)> = dec
        .mind
        .ltm
        .social
        .iter()
        .filter_map(|(oid, soc)| {
            let o = delta.actors.get(oid)?;
            if oid != deceased_id
                && o.alive
                && o.controller_id.is_none()
                && !oid.starts_with("pl:")
                && ((soc.tie_mask
                    & (tie_flags::BLOOD_KIN | tie_flags::IN_LAW | tie_flags::MASTER_DISCIPLE))
                    != 0
                    || soc.warmth >= 30)
            {
                let bonus = if (soc.tie_mask
                    & (tie_flags::BLOOD_KIN | tie_flags::IN_LAW | tie_flags::MASTER_DISCIPLE))
                    != 0
                {
                    40
                } else {
                    0
                };
                Some((i16::from(soc.warmth) + bonus, oid))
            } else {
                None
            }
        })
        .collect();
    soc_cands.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(y.1)));
    soc_cands.first().map(|(_, aid)| (*aid).clone())
}

fn is_heirloom_worthy(tool: &super::types::BoundTool) -> bool {
    !super::phase_and_vessels::is_starter_loan_weapon(&tool.slug)
        && (tool.rune_mark.is_some()
            || !tool.uid.is_empty()
            || tool.grip_familiarity >= 22
            || tool.provenance.contains("鍛")
            || tool.provenance.contains("擊潰"))
}

fn upgraded_master_role(deceased_role: &str) -> Option<&'static str> {
    if deceased_role.contains("鍛鐵") || deceased_role == "鍛鐵匠" {
        Some("鍛鐵匠")
    } else if deceased_role.contains("草藥") {
        Some("草藥師")
    } else if deceased_role.contains("酒館") {
        Some("酒館掌櫃")
    } else if deceased_role.contains("車行") {
        Some("車行掌櫃")
    } else if deceased_role == "木匠" {
        Some("木匠")
    } else if deceased_role == "皮匠" {
        Some("皮匠")
    } else {
        None
    }
}

fn record_npc_passing_chronicle(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    ob: &ObituaryRecord,
    place_node: &str,
) {
    let yr_zh = solar_year_zh(ob.solar_year);
    let age_num_zh = num_zh(i64::from(ob.age_at_death));
    let age_zh = format!("{age_num_zh}歲");
    let town_zh = town_zh_of_slug(&ob.town_slug).to_string();
    let coastal = is_node_coastal(doc, place_node, &ob.town_slug);
    let td = derive_tide_clock_details(clock);
    let weather_zh = if coastal {
        td.weather_zh.to_string()
    } else {
        "山風清勁".to_string()
    };
    let tide_zh = if coastal {
        "晚潮退水".to_string()
    } else {
        String::new()
    };
    let atmos = atmosphere_clause(coastal, &weather_zh, &tide_zh, &clock.bell_label);
    let heir_clause = match (&ob.heir_name, &ob.heirloom_zh) {
        (Some(hn), Some(hl)) => format!("名下積蓄與【{hl}】交由{hn}承接"),
        (Some(hn), None) => format!("身後家當已由{hn}整理收領"),
        (None, _) => "因無在島近親，餘財依例撥歸港務公庫安葬收斂".to_string(),
    };
    let mut bullets = vec![
        format!(
            "逝者：{}（{}{}，享壽{age_zh}）",
            ob.deceased_name, ob.race, ob.role
        ),
        format!("緣由與地界：於{town_zh}{}，{heir_clause}", ob.cause_zh),
    ];
    if let Some(ref hnote) = ob.house_note_zh {
        bullets.push(format!("門戶處置：{hnote}"));
    }
    let prose = format!(
        "太陽曆{yr_zh}年{}，{atmos}。在{town_zh}安身多年的{}{}（享壽{age_zh}）{}，{}。",
        ob.season_zh, ob.race, ob.deceased_name, ob.cause_zh, heir_clause
    );
    let deed_id = format!("deed:obituary:{}", ob.deceased_id);
    let fact = ChronicleFactPacket {
        deed_id: deed_id.clone(),
        kind: "resident_passing".to_string(),
        game_day: clock.game_days,
        solar_year: ob.solar_year,
        season_zh: ob.season_zh.clone(),
        bell_zh: clock.bell_label.clone(),
        weather_zh,
        tide_zh,
        town_slug: ob.town_slug.clone(),
        town_zh: town_zh.clone(),
        place_zh: doc.nodes.get(place_node).map_or(town_zh.clone(), |n| {
            n.name.rsplit('・').next().unwrap_or(&n.name).to_string()
        }),
        agent_id: ob.deceased_id.clone(),
        agent_public_label: ob.deceased_name.clone(),
        agent_role_zh: format!("{}{}", ob.race, ob.role),
        patient_id: String::new(),
        patient_public_label: ob.heir_name.clone().unwrap_or_default(),
        patient_role_zh: String::new(),
        forbidden_raw_names: Vec::new(),
        amount_copper: 0,
        vessel_zh: ob.heirloom_zh.clone().unwrap_or_default(),
        phase_zh: String::new(),
        extra_context_zh: ob.cause_zh.clone(),
        bullet_facts_zh: bullets,
        allowed_numbers_zh: vec![yr_zh, age_num_zh],
        witness_names: Vec::new(),
    };
    let fact_json = serde_json::to_string(&fact).unwrap_or_else(|_| "{}".to_string());
    delta
        .living_chronicle
        .push_new_projection(super::living_chronicler::ChronicleProjectionRow {
            deed_id,
            scope: "actor".to_string(),
            target_id: ob.deceased_id.clone(),
            kind: "resident_passing".to_string(),
            latch_key: String::new(),
            title_zh: format!("{}生平終章", ob.deceased_name),
            subtitle_zh: String::new(),
            era_zh: format!("太陽曆{}年{}", solar_year_zh(ob.solar_year), ob.season_zh),
            carrier_zh: "各鎮廣場往生木牌".to_string(),
            draft_zh: prose.clone(),
            prose_zh: prose,
            fact_json,
            solar_year: ob.solar_year,
            game_day: clock.game_days,
            status: "draft".to_string(),
            updated_seq: 0,
        });
}

/// 執行居民離世後的複式債權清結、銅幣遺產、鋪倉扶正、落款兵刃繼承、符質復歸與屋契移轉。
pub fn execute_npc_passing_and_inheritance(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    deceased_id: &str,
    cause_zh: &str,
) -> Option<ObituaryRecord> {
    let (
        dec_name,
        dec_race,
        dec_role,
        dec_work,
        dec_node,
        dec_home,
        dec_owned,
        dec_weapon,
        dec_copper_init,
        dec_age,
        dec_hh,
        dec_socials,
    ) = {
        let dec = delta.actors.get(deceased_id)?;
        if !dec.alive || dec.controller_id.is_some() || deceased_id.starts_with("pl:") {
            return None;
        }
        (
            dec.display_name.clone(),
            dec.race.clone(),
            dec.role.clone(),
            dec.work_node.clone(),
            dec.node_id.clone(),
            dec.home_node.clone(),
            dec.owned_house_node.clone(),
            dec.bound_weapon.clone(),
            dec.copper,
            actor_lived_years(dec, clock.game_days),
            dec.household_id.clone(),
            dec.mind.ltm.social.keys().cloned().collect::<Vec<_>>(),
        )
    };

    let heir_opt = select_estate_heir(delta, deceased_id, clock.game_days);
    let heir_name = heir_opt
        .as_ref()
        .and_then(|hid| delta.actors.get(hid).map(|h| h.display_name.clone()));
    let town_slug = town_slug_of_node(&dec_home).to_string();

    // 1. 先自空間佔據、車船載客與活態欄位卸除，寫入墓碑標記（保留 `oi_actors` 列與唯一 `soul_seed`）
    for v in &mut delta.transit_vehicles {
        v.passengers.remove(deceased_id);
    }
    if let Some(occ) = delta.node_occupants.get_mut(&dec_node) {
        occ.remove(deceased_id);
        if occ.is_empty() {
            delta.node_occupants.remove(&dec_node);
        }
    }
    if let Some(dec) = delta.actors.get_mut(deceased_id) {
        dec.alive = false;
        dec.active_skirmish = None;
        dec.active_transit = None;
        dec.ongoing_activity = None;
        dec.route_plan.clear();
        dec.owned_house_node = None;
        dec.mind.push_turning_point(
            &format!("life:passing:{}", deceased_id),
            clock.game_days,
            cause_zh,
            "life_end",
        );
        dec.mark_mutated();
    }

    // 清理未取件鐵匠訂單與進行中託管契（押金退還原主）
    for orders in delta.forge_orders.values_mut() {
        orders.retain(|o| o.customer_id != deceased_id && o.smith_id != deceased_id);
    }
    super::pacts_and_peers::settle_expired_escrow_pacts_as_world_events(doc, delta, clock);

    // 2. 清償逝者現存欠帳／飯帳並轉讓應收債權
    let related_cids: Vec<String> = delta
        .contracts
        .iter()
        .filter(|(_, c)| {
            (c.status == "open" || c.status == "defaulted")
                && (c.debtor_id == deceased_id || c.creditor_id == deceased_id)
        })
        .map(|(k, _)| k.clone())
        .collect();
    for cid in related_cids {
        let Some(c) = delta.contracts.get(&cid).cloned() else {
            continue;
        };
        if c.debtor_id == deceased_id {
            let avail = delta.actors.get(deceased_id).map_or(0, |d| d.copper);
            let pay = avail.min(c.amount_copper).max(0);
            let cred_alive = delta.actors.get(&c.creditor_id).is_some_and(|cr| cr.alive);
            let paid_ok = pay > 0
                && cred_alive
                && delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "estate_debt_settle",
                    deceased_id,
                    &c.creditor_id,
                    pay,
                    format!("{dec_name}遺產清償舊欠帳款"),
                );
            if let Some(cm) = delta.contracts.get_mut(&cid) {
                cm.status = if paid_ok && pay >= c.amount_copper {
                    "settled".to_string()
                } else {
                    "written_off".to_string()
                };
                delta.dirty_contracts.insert(cid);
            }
        } else if c.creditor_id == deceased_id {
            if let Some(ref hid) = heir_opt {
                if let Some(cm) = delta.contracts.get_mut(&cid) {
                    cm.creditor_id = hid.clone();
                    delta.dirty_contracts.insert(cid);
                }
            } else if let Some(cm) = delta.contracts.get_mut(&cid) {
                cm.status = "written_off".to_string();
                delta.dirty_contracts.insert(cid);
            }
        }
    }

    // 3. 剩餘銅幣 100% 經複式帳本過戶繼承人或港務公庫
    let rem_copper = delta.actors.get(deceased_id).map_or(0, |d| d.copper);
    if rem_copper > 0 {
        let payee = heir_opt.as_deref().unwrap_or("municipal_treasury");
        let note = match heir_name.as_deref() {
            Some(hn) => format!("{dec_name}離世遺產過戶予{hn}"),
            None => format!("{dec_name}身後絕戶餘財歸入港務公庫"),
        };
        let _ = delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "estate_inheritance",
            deceased_id,
            payee,
            rem_copper,
            note,
        );
    }
    let _ = dec_copper_init;

    // 4. 鋪面物料、家戶餘糧轉交接班繼承人，並於店鋪懸缺時扶正學徒
    let mut moved_items: Vec<(String, Vec<CarryItem>)> = Vec::new();
    let mut returned_runes: u16 = 0;
    if let Some(dec) = delta.actors.get_mut(deceased_id) {
        if heir_opt.is_some() {
            for it in dec.backpack.drain(..) {
                if it.bound_owner.is_none() && it.stolen_from.is_none() && it.count > 0 {
                    moved_items.push((String::new(), vec![it]));
                }
            }
            for (k, v) in std::mem::take(&mut dec.inn_lockers) {
                let clean: Vec<CarryItem> = v
                    .into_iter()
                    .filter(|i| i.bound_owner.is_none() && i.stolen_from.is_none() && i.count > 0)
                    .collect();
                if !clean.is_empty() {
                    moved_items.push((k, clean));
                }
            }
        } else {
            for it in dec.backpack.drain(..).chain(
                std::mem::take(&mut dec.inn_lockers)
                    .into_values()
                    .flatten()
                    .chain(dec.inn_locker.drain(..)),
            ) {
                if it.slug == "rune_stone" {
                    returned_runes = returned_runes.saturating_add(it.count as u16);
                } else if it
                    .vessel_meta
                    .as_ref()
                    .is_some_and(|m| m.rune_mark.as_deref() == Some("穩固鐵紋"))
                {
                    returned_runes = returned_runes.saturating_add(it.count as u16);
                }
            }
            if dec.bound_weapon.rune_mark.as_deref() == Some("穩固鐵紋") {
                dec.bound_weapon.rune_mark = None;
                returned_runes = returned_runes.saturating_add(1);
            }
        }
        dec.mark_mutated();
    }

    if returned_runes > 0 {
        delta.chaos_rune_pool = delta.chaos_rune_pool.saturating_add(returned_runes).min(12);
    }

    let mut heirloom_zh: Option<String> = None;
    let is_shop_work = doc.nodes.get(&dec_work).is_some_and(|n| n.kind == "shop");
    let promoted_role = if is_shop_work {
        upgraded_master_role(&dec_role)
    } else {
        None
    };

    if let Some(ref hid) = heir_opt {
        let legacy_vessel = is_heirloom_worthy(&dec_weapon).then(|| {
            let mut w = dec_weapon.clone();
            if w.uid.is_empty() {
                w.uid = format!("heirloom:{}:{}", deceased_id, clock.game_days);
            }
            w.provenance = if w.provenance.is_empty() {
                format!("{dec_name}生前傳家遺物")
            } else {
                format!("{}・{dec_name}傳家遺物", w.provenance)
            };
            w
        });

        let heir_needs_shop_post = is_shop_work
            && promoted_role.is_some()
            && !delta.actors.values().any(|o| {
                o.alive
                    && o.actor_id != deceased_id
                    && o.work_node == dec_work
                    && Some(o.role.as_str()) == promoted_role
            });

        if let Some(heir) = delta.actors.get_mut(hid) {
            for (lk, items) in moved_items {
                if lk.is_empty() {
                    for it in items {
                        add_carry_item(&mut heir.backpack, it);
                    }
                } else {
                    let target_key = if lk == dec_work {
                        dec_work.clone()
                    } else {
                        heir.owned_house_node
                            .as_deref()
                            .unwrap_or(heir.home_node.as_str())
                            .to_string()
                    };
                    let box_v = heir.inn_lockers.entry(target_key).or_default();
                    for it in items {
                        add_carry_item(box_v, it);
                    }
                }
            }
            if let Some(mut v) = legacy_vessel {
                let vname = v.name.clone();
                heirloom_zh = Some(vname.clone());
                if heir.bound_weapon.rune_mark.is_none() && v.rune_mark.is_some() {
                    let old = heir.bound_weapon.clone();
                    v.grip_familiarity = v.grip_familiarity.max(12);
                    heir.bound_weapon = v;
                    if !super::phase_and_vessels::is_starter_loan_weapon(&old.slug) {
                        add_carry_item(
                            &mut heir.backpack,
                            CarryItem {
                                slug: old.slug.clone(),
                                name: old.name.clone(),
                                count: 1,
                                weight_units: 2,
                                vessel_meta: Some(old),
                                ..CarryItem::default()
                            },
                        );
                    }
                } else {
                    add_carry_item(
                        &mut heir.backpack,
                        CarryItem {
                            slug: v.slug.clone(),
                            name: vname,
                            count: 1,
                            weight_units: 2,
                            vessel_meta: Some(v),
                            ..CarryItem::default()
                        },
                    );
                }
            }
            if heir_needs_shop_post && let Some(new_r) = promoted_role {
                heir.role = new_r.to_string();
                heir.work_node = dec_work.clone();
                heir.mind.push_turning_point(
                    &format!("succession:{hid}:{}", clock.game_days),
                    clock.game_days,
                    &format!("送別{dec_name}後接過鋪面鎖鑰與爐火，升任本鎮{new_r}"),
                    "work_steady",
                );
            } else {
                heir.mind.push_turning_point(
                    &format!("inherit:{hid}:{}", clock.game_days),
                    clock.game_days,
                    &format!("送別{dec_name}後收領其生前積蓄與行囊舊物"),
                    "honour_kin",
                );
            }
            let _ = super::world_weave_bio::refresh_actor_biography_if_needed(
                doc,
                heir,
                clock.game_days,
            );
            heir.mark_mutated();
        }
    }

    // 5. 自有屋舍／待售房契移轉或釋回待售
    let mut house_note_zh: Option<String> = None;
    if let Some(owned_nid) = dec_owned {
        let heir_homeless = heir_opt.as_ref().is_some_and(|hid| {
            delta
                .actors
                .get(hid)
                .is_some_and(|h| h.owned_house_node.is_none())
        });
        if heir_homeless && let Some(ref hid) = heir_opt {
            if let Some(h) = delta.actors.get_mut(hid) {
                h.owned_house_node = Some(owned_nid.clone());
                h.home_node = owned_nid.clone();
                h.mark_mutated();
            }
            if delta.sold_houses.contains_key(&owned_nid) {
                delta.sold_houses.insert(owned_nid.clone(), hid.clone());
            }
            if let Some(bh) = delta.built_houses.get_mut(&owned_nid) {
                bh.owner_id = hid.clone();
            }
            let hname = doc
                .nodes
                .get(&owned_nid)
                .map_or("自有舊屋", |n| n.name.as_str());
            house_note_zh = Some(format!(
                "【{hname}】房契由{}繼承入住",
                heir_name.as_deref().unwrap_or("至親")
            ));
        } else if delta.sold_houses.contains_key(&owned_nid) {
            delta.sold_houses.remove(&owned_nid);
            delta
                .live_land
                .house_integrity
                .insert(owned_nid.clone(), 72);
            let hname = doc
                .nodes
                .get(&owned_nid)
                .map_or("巷內石屋", |n| n.name.as_str());
            house_note_zh = Some(format!("【{hname}】已無親繼領，釋回市政掛牌待售"));
        }
    }

    // 6. 親友收到真切哀訊（高顯著度死亡衝擊＋LTM 留痕）
    let mut mourners = dec_socials;
    for (aid, a) in &delta.actors {
        if a.alive
            && a.controller_id.is_none()
            && !dec_hh.is_empty()
            && a.household_id == dec_hh
            && !mourners.contains(aid)
        {
            mourners.push(aid.clone());
        }
    }
    let grief_text = format!(
        "聞悉同在島上討生活的{dec_name}（{dec_role}）於太陽曆{}年{}離世",
        solar_year_zh(current_solar_year(clock.game_days)),
        solar_year_and_season_of_day(clock.game_days).1
    );
    for mid in mourners.into_iter().take(18) {
        if mid == deceased_id {
            continue;
        }
        if let Some(m) = delta.actors.get_mut(&mid)
            && m.alive
        {
            let langs = m.languages.clone();
            let _ = m.mind.ingest_sensory_event(
                &langs,
                EventPublicity::PublicLoud,
                None,
                false,
                &format!("grief:{deceased_id}:{}", clock.game_days),
                clock.game_days,
                clock.bell_index,
                &dec_node,
                deceased_id,
                &dec_name,
                EvidenceTier::DirectWitness,
                82,
                75,
                ValueAxis::KinshipLoyalty,
                &grief_text,
            );
            m.mind.stm.affect.grief = m.mind.stm.affect.grief.saturating_add(24).min(95);
            m.mark_mutated();
        }
    }

    let (_, season_zh) = solar_year_and_season_of_day(clock.game_days);
    let ob = ObituaryRecord {
        deceased_id: deceased_id.to_string(),
        deceased_name: dec_name.clone(),
        race: dec_race,
        role: dec_role,
        town_slug,
        solar_year: current_solar_year(clock.game_days),
        season_zh: season_zh.to_string(),
        game_day: clock.game_days,
        age_at_death: dec_age,
        cause_zh: cause_zh.to_string(),
        heir_name,
        heirloom_zh,
        house_note_zh,
    };
    delta.live_lifecycle.obituaries.push_front(ob.clone());
    while delta.live_lifecycle.obituaries.len() > 32 {
        delta.live_lifecycle.obituaries.pop_back();
    }
    delta.extras_dirty = true;
    record_npc_passing_chronicle(doc, delta, clock, &ob, &dec_home);

    if let Some(plazas) = doc
        .nodes
        .iter()
        .find(|(_, n)| n.kind == "plaza" && town_slug_of_node(&n.id) == ob.town_slug)
        .map(|(nid, _)| nid.clone())
    {
        broadcast_scene_event(
            doc,
            delta,
            clock,
            &plazas,
            deceased_id,
            &dec_name,
            EventPublicity::PublicLoud,
            Some("harbor_trade"),
            82,
            60,
            ValueAxis::KinshipLoyalty,
            "resident_passing",
            &grief_text,
        );
    }

    Some(ob)
}

/// 重傷失救判定：NPC 若已帶嚴重傷勢警告（`severe_wrist_warning`）、無搜救契且身旁無清醒旁人救護，則傷重殞命。
pub fn should_trigger_npc_mortal_peril(
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node_id: &str,
) -> bool {
    let Some(a) = delta.actors.get(aid) else {
        return false;
    };
    if a.controller_id.is_some()
        || aid.starts_with("pl:")
        || !a.alive
        || !a.severe_wrist_warning
        || a.rescue_pact_active
    {
        return false;
    }
    let helper_near = super::phase_and_vessels::witnesses_at_node(delta, node_id, aid)
        .into_iter()
        .any(|(wid, _)| {
            delta
                .actors
                .get(&wid)
                .is_some_and(|w| w.alive && w.fatigue < 65 && w.wound_part.is_none())
        });
    if helper_near {
        return false;
    }
    let roll = splitmix64(a.soul_seed ^ (clock.game_days as u64) ^ u64::from(clock.bell_index));
    roll % 3 == 0
}

fn try_spawn_coming_of_age_youth(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    let alive_npcs = delta
        .actors
        .iter()
        .filter(|(id, a)| a.alive && a.controller_id.is_none() && !id.starts_with("pl:"))
        .count();
    if alive_npcs >= 994 {
        return;
    }

    let mut hh_counts: BTreeMap<String, usize> = BTreeMap::new();
    for (id, a) in &delta.actors {
        if a.alive
            && a.controller_id.is_none()
            && !id.starts_with("pl:")
            && !a.household_id.is_empty()
        {
            *hh_counts.entry(a.household_id.clone()).or_insert(0) += 1;
        }
    }

    let mut parent_cands: Vec<String> = delta
        .actors
        .iter()
        .filter(|(id, a)| {
            a.alive
                && a.controller_id.is_none()
                && !id.starts_with("pl:")
                && !a.household_id.is_empty()
                && a.copper >= 36
                && hh_counts.get(&a.household_id).copied().unwrap_or(0) <= 4
                && count_household_larder_rations(delta, a) >= 3
        })
        .map(|(id, _)| id.clone())
        .collect();
    parent_cands.sort();
    if parent_cands.is_empty() {
        return;
    }

    let idx = (splitmix64((clock.game_days as u64) ^ 0xBEEF_9001) as usize) % parent_cands.len();
    let parent_id = parent_cands[idx].clone();
    let (p_name, p_race, p_role, p_hh, p_home, p_work, p_langs) = {
        let Some(p) = delta.actors.get(&parent_id) else {
            return;
        };
        (
            p.display_name.clone(),
            p.race.clone(),
            p.role.clone(),
            p.household_id.clone(),
            p.owned_house_node
                .clone()
                .unwrap_or_else(|| p.home_node.clone()),
            p.work_node.clone(),
            p.languages.clone(),
        )
    };

    delta.live_lifecycle.spawned_youth_seq =
        delta.live_lifecycle.spawned_youth_seq.saturating_add(1);
    let seq = delta.live_lifecycle.spawned_youth_seq;
    const YOUTH_GIVEN: &[&str] = &[
        "阿誠", "春生", "鐵柱", "青枝", "石仔", "阿桐", "海生", "松仔",
    ];
    let gname = YOUTH_GIVEN[(seq as usize) % YOUTH_GIVEN.len()];
    let p_prefix: String = p_name.chars().take(2).collect();
    let display_name = format!("{p_prefix}{gname}");
    let new_aid = format!("res:gen_youth:{seq}");
    let new_role = if doc.nodes.get(&p_work).is_some_and(|n| n.kind == "shop") {
        if p_role.contains("鍛鐵") {
            "鍛鐵匠學徒"
        } else if p_role.contains("草藥") {
            "草藥師學徒"
        } else if p_role.contains("酒館") {
            "酒館掌櫃學徒"
        } else {
            p_role.as_str()
        }
    } else if is_food_producer_role(&p_role) {
        p_role.as_str()
    } else {
        "雜工學徒"
    };

    let spec = NewActorSpawnSpec {
        actor_id: new_aid.clone(),
        controller_id: None,
        preferred_soul_seed: None,
        display_name: display_name.clone(),
        race: p_race.clone(),
        role: new_role.to_string(),
        appearance: format!("隨{}長大成人開始幫手家業", p_name),
        household_id: p_hh,
        home_node: p_home.clone(),
        work_node: p_work.clone(),
        start_node: p_home,
        initial_copper_from_mainland: 0,
        bound_weapon: default_tool_for_role(new_role, &display_name),
        languages: p_langs,
    };
    let created = spawn_dynamic_actors(doc, delta, vec![spec], clock.game_days, clock.bell_index);
    if !created.is_empty() {
        let adult_age = racial_adult_age(&p_race);
        let birth_y = current_solar_year(clock.game_days) - adult_age;
        if let Some(youth) = delta.actors.get_mut(&new_aid) {
            youth.birth_year = birth_y;
            youth.household_bond = "新生接班晚輩".to_string();
            youth.origin_pack = "island_born".to_string();
            let _ = youth.mind.fold_social_memory_from_episode(
                &format!("kin:youth:{seq}"),
                clock.game_days,
                &parent_id,
                &p_name,
                &town_zh_of_slug(town_slug_of_node(&youth.home_node)),
                tie_flags::BLOOD_KIN,
                "同屋長輩",
                true,
                true,
                72,
                52,
                &format!("由{p_name}撫養成年並授以手藝"),
                "",
            );
            youth.mark_mutated();
        }
        if let Some(parent) = delta.actors.get_mut(&parent_id) {
            let _ = parent.mind.fold_social_memory_from_episode(
                &format!("kin:youth:{seq}"),
                clock.game_days,
                &new_aid,
                &display_name,
                &town_zh_of_slug(town_slug_of_node(&parent.home_node)),
                tie_flags::BLOOD_KIN,
                "成年子弟",
                true,
                true,
                72,
                55,
                &format!("看著{display_name}滿歲接手家業幫工"),
                "",
            );
            parent.mark_mutated();
        }
        let _ = delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "youth_allowance",
            &parent_id,
            &new_aid,
            12,
            format!("{p_name}撥給成年接班的{display_name}十二枚銅幣立業盤纏"),
        );
        delta.extras_dirty = true;
    }
}

/// 按每 7 遊戲日執行至多 1 名壽終正寢檢核與 1 名新世代成年接班（嚴格隔離玩家並嚴守守恆）。
pub fn tick_daily_lifecycle(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    if delta.live_lifecycle.last_lifecycle_day == 0 {
        delta.live_lifecycle.last_lifecycle_day = clock.game_days;
        delta.extras_dirty = true;
        return;
    }
    if clock.game_days < delta.live_lifecycle.last_lifecycle_day + 7 {
        return;
    }

    delta.live_lifecycle.last_lifecycle_day = clock.game_days;
    delta.extras_dirty = true;

    let mut oldest_due: Vec<(i16, u64, String)> = delta
        .actors
        .iter()
        .filter_map(|(aid, a)| {
            if !a.alive || a.controller_id.is_some() || aid.starts_with("pl:") {
                return None;
            }
            let age = actor_lived_years(a, clock.game_days);
            let (min_age, cap_age) = race_lifespan_bounds(&a.race, a.soul_seed);
            if age < min_age {
                return None;
            }
            let over = (age - min_age).max(0) as u64;
            let span = (cap_age - min_age).max(1) as u64;
            let roll = splitmix64(a.soul_seed ^ (clock.game_days as u64)) % span;
            if age >= cap_age || roll <= over {
                Some((age, a.soul_seed, aid.clone()))
            } else {
                None
            }
        })
        .collect();
    oldest_due.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));

    if let Some((_age, _, deceased_id)) = oldest_due.first() {
        let cause = "年壽已高，於夜裡在屋中安詳壽終".to_string();
        if execute_npc_passing_and_inheritance(doc, delta, clock, deceased_id, &cause).is_some() {
            delta.extras_dirty = true;
        }
    }

    try_spawn_coming_of_age_youth(doc, delta, clock);
}

pub fn append_live_lifecycle_scene_objects<F>(
    delta: &IslandWorldDelta,
    node: &SpatialNodeDef,
    objects: &mut Vec<Value>,
    mut push_audit: F,
) where
    F: FnMut(&str, &str, &str, &str, &str, &str),
{
    if node.kind != "plaza" || delta.live_lifecycle.obituaries.is_empty() {
        return;
    }
    let ts = town_slug_of_node(&node.id);
    let board_name = if ts == "greypine_post" {
        "山驛往生告示木牌"
    } else {
        "港鎮往生告示木牌"
    };
    let latest = &delta.live_lifecycle.obituaries[0];
    objects.push(json!({
        "object_id": "obj:memorial_roll",
        "name": board_name,
        "cue": format!("近記【{}】等 {} 人壽終與家業交接冊錄", latest.deceased_name, delta.live_lifecycle.obituaries.len()),
        "desc": "鎮公櫃釘在碑旁的榆木板，逐筆寫著島上離世居民的年歲、後嗣承繼與空屋盤出情形。",
        "direct_action": { "id": "read_memorial_roll", "param": "" },
        "options": [],
    }));
    push_audit(
        "read_memorial_roll",
        "",
        "展讀往生告示木牌",
        "察看島上生老更替與家業承繼",
        "object",
        "obj:memorial_roll",
    );
}

pub fn try_apply_live_lifecycle_action(
    delta: &mut IslandWorldDelta,
    aid: &str,
    node: &SpatialNodeDef,
    action: &str,
) -> Option<Result<String, String>> {
    if action != "read_memorial_roll" {
        return None;
    }
    if node.kind != "plaza" {
        return Some(Err("需在城鎮廣場牌欄旁方能翻看往生與承繼名錄。".to_string()));
    }
    if delta.live_lifecycle.obituaries.is_empty() {
        return Some(Ok(
            "榆木牌面上乾乾淨淨，近幾季島上尚無鎮民故去、家業異動之記載。".to_string(),
        ));
    }
    let lines: Vec<String> = delta
        .live_lifecycle
        .obituaries
        .iter()
        .take(5)
        .map(|ob| {
            let yr = solar_year_zh(ob.solar_year);
            let heir_part = ob
                .heir_name
                .as_deref()
                .map_or("無在島親眷承領（餘財歸公）".to_string(), |hn| {
                    format!("由 {hn} 接承遺產")
                });
            let tool_part = ob
                .heirloom_zh
                .as_deref()
                .map_or(String::new(), |t| format!("・傳下【{t}】"));
            let house_part = ob
                .house_note_zh
                .as_deref()
                .map_or(String::new(), |h| format!("・{h}"));
            format!(
                "【太陽曆{yr}年{}】{}（{}{}，{}歲）：{}；{}{tool_part}{house_part}",
                ob.season_zh,
                ob.deceased_name,
                ob.race,
                ob.role,
                ob.age_at_death,
                ob.cause_zh,
                heir_part
            )
        })
        .collect();
    let msg = format!("往生與家業名錄近筆：\n{}", lines.join("\n"));
    if let Some(a) = delta.actors.get_mut(aid) {
        push_breath(a, msg.clone());
    }
    Some(Ok(msg))
}
