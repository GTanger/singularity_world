//! 玩家＝NPC 離線自主託管方針與上線去數值經歷回顧（落實 `DESIGN_KEYWORDS.md` L142/L144/L175）。
//!
//! 嚴格守則：
//! 1. 預設方針 `rest_inn` 零花費、絕不自動消耗背囊 `rye_bread` 或擅自兌換 `lodging_voucher`（守住 H1 與離線保底測試）；
//! 2. 玩家主動切換為 `work_livelihood` / `forage_patrol` / `social_tavern` 後，離線期間與 NPC 共用同一套世界規則：
//!    所有銅幣流轉走 `delta.transfer_copper`（守恆差恆為 0），採集受背囊負重與地點真實物產約束；
//! 3. 所有離線行動均寫入有界去數值化敘事日誌 `offline_diary`（最多 12 筆），重新上線後呈現自然體感與經歷回顧。

use serde_json::{Value, json};

use super::clock_and_lang::SolarClockSnapshot;
use super::mind::ActiveIntention;
use super::movement::{add_carry_item, add_language_exposure, push_breath, shortest_walk_path};
use super::types::{CarryItem, IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

pub const VALID_OFFLINE_POLICIES: &[(&str, &str, &str)] = &[
    (
        "rest_inn",
        "客房歇息養傷",
        "預設穩妥：回宿房或酒館爐畔緩氣回神，零花費、不動行囊乾糧",
    ),
    (
        "work_livelihood",
        "駐鎮幫工謀生",
        "日間前往鄰近碼頭或開爐鍛鋪出力掙銅幣，同步磨練掌中手感",
    ),
    (
        "forage_patrol",
        "照料田壟與林緣巡採",
        "優先照料收割名下田壟藥圃；若無田產則沿已知安全林緣採集實物備料",
    ),
    (
        "social_tavern",
        "市集酒館交際",
        "在廣場與酒館隨眾聽風聲、與在場街坊混臉熟並耳濡目染異族言語",
    ),
];

pub fn normalize_offline_policy(raw: &str) -> &'static str {
    match raw.trim() {
        "work_livelihood" => "work_livelihood",
        "forage_patrol" => "forage_patrol",
        "social_tavern" => "social_tavern",
        _ => "rest_inn",
    }
}

pub fn offline_policy_label_zh(policy: &str) -> &'static str {
    let norm = normalize_offline_policy(policy);
    VALID_OFFLINE_POLICIES
        .iter()
        .find(|(id, _, _)| *id == norm)
        .map_or("客房歇息養傷", |(_, lbl, _)| *lbl)
}

pub fn push_offline_diary(a: &mut UnifiedMindBody, clock: &SolarClockSnapshot, entry_zh: String) {
    let stamped = format!("〔{}・{}〕{}", clock.calendar_label, clock.bell_label, entry_zh);
    if a.offline_diary.back().map(String::as_str) == Some(stamped.as_str()) {
        return;
    }
    if a.offline_diary.len() >= 12 {
        a.offline_diary.pop_front();
    }
    a.offline_diary.push_back(stamped);
    a.offline_recap_unread = true;
    a.mark_mutated();
}

pub fn set_player_offline_policy(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    policy_raw: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let norm = normalize_offline_policy(policy_raw);
    let label = offline_policy_label_zh(norm);
    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    a.offline_policy = norm.to_string();
    a.last_active_unix = clock.real_unix;
    let msg = format!(
        "你已立下暫離時的行事方針：【{}】。往後你不在身邊照看時，角色仍會依此在島上自主過活。",
        label
    );
    push_breath(a, msg.clone());
    a.mark_mutated();
    Ok(msg)
}

pub fn dismiss_player_offline_recap(
    delta: &mut IslandWorldDelta,
    actor_key: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    a.offline_recap_unread = false;
    a.mark_mutated();
    Ok("已收妥離線期間的經歷見聞。".to_string())
}

/// 整鐘推進離線玩家之日間自主行程（嚴格僅在 `!is_actor_online_protected` 且非夜間、非戰鬥、非搭車時執行）。
pub fn tick_offline_player_day_autonomy(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    abs_bell: i64,
) {
    let (policy, node_id, fatigue, known_nodes, has_room) = {
        let Some(a) = delta.actors.get(aid) else {
            return;
        };
        if !a.alive
            || !a.setup_done
            || a.controller_id.is_none()
            || a.active_skirmish.is_some()
            || a.active_transit.is_some()
            || super::live_tide::is_actor_online_protected(a, clock.real_unix)
        {
            return;
        }
        (
            normalize_offline_policy(&a.offline_policy),
            a.node_id.clone(),
            a.fatigue,
            a.mind.ltm.known_nodes.clone(),
            a.inn_nights_left > 0 || a.owned_house_node.is_some(),
        )
    };

    // 預設 rest_inn：純緩氣歇息（0 銅花費、絕不動乾糧或抵宿券）
    if policy == "rest_inn" {
        if fatigue > 0
            && let Some(a) = delta.actors.get_mut(aid)
        {
            a.fatigue = a.fatigue.saturating_sub(14);
            a.mind.stm.affect.fatigue = a.mind.stm.affect.fatigue.saturating_sub(14);
            if a.fatigue == 0 && fatigue >= 20 {
                let place = doc
                    .nodes
                    .get(&node_id)
                    .map_or("落腳處", |n| n.name.split('・').next_back().unwrap_or(&n.name));
                let note = if has_room {
                    format!("你在{}安穩歇息了一段時辰，肩背痠脹退去大半，氣息重歸勻暢。", place)
                } else {
                    format!("你在{}避風處靜坐調息了一陣，精神緩過不少。", place)
                };
                push_offline_diary(a, clock, note);
            }
            a.mark_mutated();
        }
        return;
    }

    // 疲勞過高時自主停手緩氣，防離線過勞
    if fatigue >= 76 {
        if let Some(a) = delta.actors.get_mut(aid) {
            a.fatigue = a.fatigue.saturating_sub(18);
            a.mind.stm.affect.fatigue = a.mind.stm.affect.fatigue.saturating_sub(18);
            push_offline_diary(
                a,
                clock,
                "連日奔忙雙腿發沉，你索性在路旁茶棚石階坐下歇了口氣。".to_string(),
            );
        }
        return;
    }

    let cur_cell = doc.nodes.get(&node_id).map_or(4616, |n| n.cell_id);

    match policy {
        "work_livelihood" => {
            // 若本人此刻已站在可開爐鍛鐵鋪或碼頭，直接就地幫工；否則找同格最近工作點前往
            let cur_is_open_work = doc.nodes.get(&node_id).filter(|n| {
                n.kind == "dock"
                    || (n.kind == "shop"
                        && (n.name.contains("鍛鐵") || n.name.contains("鐵匠"))
                        && !super::live_commodities::is_smithy_forge_cold(delta, &n.id))
            });
            let target_work = cur_is_open_work
                .or_else(|| {
                    doc.nodes.values().find(|n| {
                        n.cell_id == cur_cell
                            && n.kind == "shop"
                            && (n.name.contains("鍛鐵") || n.name.contains("鐵匠"))
                            && !super::live_commodities::is_smithy_forge_cold(delta, &n.id)
                    })
                })
                .or_else(|| {
                    doc.nodes
                        .values()
                        .find(|n| n.cell_id == cur_cell && n.kind == "dock")
                })
                .map(|n| (n.id.clone(), n.name.clone(), n.kind.clone()));

            let Some((work_nid, work_name, work_kind)) = target_work else {
                return;
            };
            if node_id != work_nid {
                route_offline_actor_to(doc, delta, aid, &node_id, &work_nid, "離線前往幫工", abs_bell);
                return;
            }
            // 同格真幫工：嚴守銅幣守恆（由在場鋪主或公庫支付）
            let payer_opt = if work_kind == "shop" {
                delta.node_occupants.get(&work_nid).and_then(|occ| {
                    occ.iter().find(|&oid| {
                        oid != aid
                            && delta.actors.get(oid).is_some_and(|o| {
                                o.alive
                                    && o.controller_id.is_none()
                                    && o.copper >= 5
                                    && (o.role.contains("鍛鐵") || o.role.contains("木匠"))
                            })
                    })
                }).cloned()
            } else {
                Some("municipal_treasury".to_string())
            };
            let (wage, desc_zh, diary_zh) = if work_kind == "shop" {
                (
                    5i64,
                    "離線助鍛工錢",
                    format!(
                        "你在{}爐旁幫忙拉風箱校鐵胚，掌中握器更順手，領到一筆扎實工錢。",
                        work_name
                    ),
                )
            } else {
                (
                    4i64,
                    "離線碼頭搬運腳錢",
                    format!(
                        "你在{}跟著腳夫搬卸了一趟靠岸船貨，滿身汗氣換得幾枚腳錢。",
                        work_name
                    ),
                )
            };
            // 嚴格遵守真實開潮與爐火耗料（杜絕無中生有離線刷錢）
            if work_kind == "dock" && !super::live_tide::is_dock_cargo_tide_open(clock) {
                return;
            }
            let used_fuel = if work_kind == "shop" {
                super::live_commodities::consume_smithy_fuel_one(delta, payer_opt.as_deref(), &work_nid)
            } else {
                None
            };
            if work_kind == "shop" && used_fuel.is_none() {
                return;
            }
            if let Some(payer) = payer_opt
                && delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "offline_work",
                    &payer,
                    aid,
                    wage,
                    desc_zh.to_string(),
                )
                && let Some(a) = delta.actors.get_mut(aid)
            {
                a.fatigue = (a.fatigue + 12).min(92);
                a.bound_weapon.grip_familiarity = a.bound_weapon.grip_familiarity.saturating_add(1).min(36);
                push_offline_diary(a, clock, diary_zh);
            } else if let Some((holder, item)) = used_fuel {
                super::live_commodities::restore_shop_stock_one(delta, &holder, &work_nid, item);
            }
        }
        "forage_patrol" => {
            // A+B 合流：優先照料或收割名下實有田壟藥圃（絕非無中生有掛機刷資源！）
            if let Some(plot) = delta
                .farm_plots
                .values()
                .find(|p| p.owner_id == aid)
                .cloned()
            {
                if node_id != plot.node_id {
                    route_offline_actor_to(doc, delta, aid, &node_id, &plot.node_id, "離線前往照料名下田壟", abs_bell);
                    return;
                }
                if let Some(nd) = doc.nodes.get(&plot.node_id) {
                    let res = if clock.game_days >= plot.mature_day {
                        super::living_sim_and_farm::apply_harvest_farm_plot(doc, delta, clock, aid, nd, &plot.plot_id)
                    } else if plot.tended_day != clock.game_days {
                        super::living_sim_and_farm::apply_tend_farm_plot(delta, clock, aid, nd, &plot.plot_id)
                    } else {
                        Err(String::new())
                    };
                    if let Ok(note) = res
                        && let Some(a) = delta.actors.get_mut(aid)
                    {
                        push_offline_diary(a, clock, note);
                    }
                    return;
                }
            }
            // 尋找已知安全野外鄰格採集（不進入地城）
            let cur_is_wild = doc
                .nodes
                .get(&node_id)
                .is_some_and(|n| !n.indoor && !n.dungeon && n.kind == "wild");
            if !cur_is_wild {
                let wild_dst = known_nodes
                    .iter()
                    .filter_map(|nid| doc.nodes.get(nid))
                    .find(|n| !n.indoor && !n.dungeon && n.kind == "wild")
                    .or_else(|| {
                        // 或由目前節點出口一步踏出城外
                        doc.nodes.get(&node_id).and_then(|cur| {
                            cur.exits
                                .iter()
                                .filter_map(|e| doc.nodes.get(&e.to))
                                .find(|n| !n.indoor && !n.dungeon && n.kind == "wild")
                        })
                    })
                    .map(|n| n.id.clone());
                if let Some(dst) = wild_dst {
                    route_offline_actor_to(doc, delta, aid, &node_id, &dst, "離線林緣巡採", abs_bell);
                }
                return;
            }
            let cur_bulk = delta
                .actors
                .get(aid)
                .map_or(99, |a| super::transit_and_cargo::count_backpack_bulk_units(&a.backpack));
            if cur_bulk >= 8 {
                if let Some(a) = delta.actors.get_mut(aid) {
                    push_offline_diary(
                        a,
                        clock,
                        "肩上背囊已裝得沉甸甸，你在林緣收手歇息，等回城再行處置。".to_string(),
                    );
                }
                return;
            }
            let cell = doc.cells.get(&cur_cell.to_string());
            let (slug, name, w) = if cell.is_some_and(|c| c.is_coast) {
                ("driftwood", "異國刻紋漂流木", 2u16)
            } else if cell.is_some_and(|c| c.height >= 23) {
                ("iron_ore", "粗鐵礦石", 3u16)
            } else {
                ("herb_bundle", "山林止血草束", 1u16)
            };
            if let Some(a) = delta.actors.get_mut(aid) {
                add_carry_item(
                    &mut a.backpack,
                    CarryItem {
                        slug: slug.to_string(),
                        name: name.to_string(),
                        count: 1,
                        weight_units: w,
                        ..CarryItem::default()
                    },
                );
                a.fatigue = (a.fatigue + 10).min(92);
                let place = doc
                    .nodes
                    .get(&node_id)
                    .map_or("城外林徑", |n| n.name.as_str());
                push_offline_diary(
                    a,
                    clock,
                    format!("你在{}沿坡尋覓，順手採得一份【{}】收入背囊。", place, name),
                );
            }
        }
        "social_tavern" => {
            let social_dst = doc
                .nodes
                .values()
                .find(|n| n.cell_id == cur_cell && n.kind == "shop" && n.name.contains("酒館"))
                .or_else(|| {
                    doc.nodes
                        .values()
                        .find(|n| n.cell_id == cur_cell && n.kind == "plaza")
                })
                .map(|n| (n.id.clone(), n.name.clone()));
            let Some((dst_id, dst_name)) = social_dst else {
                return;
            };
            if node_id != dst_id {
                route_offline_actor_to(doc, delta, aid, &node_id, &dst_id, "離線市集交際", abs_bell);
                return;
            }
            let partner_opt = delta.node_occupants.get(&dst_id).and_then(|occ| {
                occ.iter()
                    .filter(|&oid| oid != aid)
                    .filter_map(|oid| delta.actors.get(oid))
                    .find(|o| o.alive && o.controller_id.is_none() && o.node_id == dst_id)
                    .map(|o| (o.actor_id.clone(), o.display_name.clone(), o.race.clone()))
            });
            if let Some((pid_npc, pname_npc, prace)) = partner_opt {
                let p_name = delta
                    .actors
                    .get(aid)
                    .map_or_else(String::new, |a| a.display_name.clone());
                if let Some(npc) = delta.actors.get_mut(&pid_npc) {
                    let s = npc.mind.ltm.social.entry(aid.to_string()).or_default();
                    s.acquainted = true;
                    s.familiarity = s.familiarity.saturating_add(6).min(100);
                    s.warmth = (s.warmth + 4).clamp(-100, 100);
                    npc.mark_mutated();
                }
                if let Some(a) = delta.actors.get_mut(aid) {
                    let s = a.mind.ltm.social.entry(pid_npc).or_default();
                    s.acquainted = true;
                    s.familiarity = s.familiarity.saturating_add(6).min(100);
                    s.warmth = (s.warmth + 4).clamp(-100, 100);
                    let mut lang_bit = String::new();
                    if let Some(rl) = super::dialogue_llm::primary_racial_lang(&prace)
                        && add_language_exposure(a, clock.game_days, rl, 4).is_some()
                    {
                        lang_bit = format!("，還順耳記了幾句{}短詞", super::clock_and_lang::language_name_zh(rl));
                    }
                    let _ = p_name;
                    push_offline_diary(
                        a,
                        clock,
                        format!(
                            "你在{}與{}閒聊了一陣街坊近況，彼此混了個臉熟{}。",
                            dst_name, pname_npc, lang_bit
                        ),
                    );
                }
            }
        }
        _ => {}
    }
}

fn route_offline_actor_to(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    aid: &str,
    from_nid: &str,
    to_nid: &str,
    reason: &str,
    abs_bell: i64,
) {
    let mut allowed = delta
        .actors
        .get(aid)
        .map(|a| a.mind.ltm.known_nodes.clone())
        .unwrap_or_default();
    allowed.insert(to_nid.to_string());
    if let Some(path) = shortest_walk_path(doc, from_nid, to_nid, &allowed)
        && path.len() >= 2
        && let Some(a) = delta.actors.get_mut(aid)
        && a.route_plan.is_empty()
    {
        a.mind.ltm.known_nodes.insert(to_nid.to_string());
        a.route_plan = path.into_iter().skip(1).collect();
        if a.mind.stm.intentions.len() >= 4 {
            a.mind.stm.intentions.pop_front();
        }
        a.mind.stm.intentions.push_back(ActiveIntention {
            kind: "offline_policy".to_string(),
            target_node: to_nid.to_string(),
            reason_zh: reason.to_string(),
            created_bell: abs_bell,
        });
        a.mark_mutated();
    }
}

pub fn build_offline_policy_view_json(viewer: &UnifiedMindBody) -> Value {
    let cur = normalize_offline_policy(&viewer.offline_policy);
    let options: Vec<Value> = VALID_OFFLINE_POLICIES
        .iter()
        .map(|(id, label, desc)| {
            json!({
                "id": id,
                "label": label,
                "desc": desc,
                "active": *id == cur,
            })
        })
        .collect();
    json!({
        "current": cur,
        "current_label": offline_policy_label_zh(cur),
        "options": options,
        "unread_recap": viewer.offline_recap_unread && !viewer.offline_diary.is_empty(),
        "diary": viewer.offline_diary.iter().cloned().collect::<Vec<_>>(),
    })
}
