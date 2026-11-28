//! 實體工作過程（碼頭記籌手領箱扛送 `DroppedAt`、百工 10 秒快修 vs 1 鐘精鍛委託）、
//! 酒館 1～6 號獨立客房（12 秒短歇旁聽 vs 整夜熟睡）、4 處官道避雨茶棚（0 秒熱茶活情報）與南岸隔欄捎信引擎。

use std::collections::BTreeSet;

use super::clock_and_lang::SolarClockSnapshot;
use super::live_tide::derive_tide_clock_details;
use super::movement::{add_carry_item, push_breath, shortest_walk_path, take_carry_item};
use super::types::{
    BoundTool, CarryItem, CustomForgeOrder, DockCargoOrder, DroppedLootBag, IslandWorldDelta,
    OngoingActivityState, SpatialNodeDef, WorldStaticDoc,
};

/// 自背包內 `dock_cargo_crate` 實物反推對應之港務送貨工序（確保拾起他人遺落貨箱亦可無縫送達或放下，零軟鎖！）
pub fn reconstruct_dock_haul_from_crate(
    doc: &WorldStaticDoc,
    cur_node: &str,
    crate_item: &CarryItem,
) -> Option<OngoingActivityState> {
    if crate_item.slug != "dock_cargo_crate" || crate_item.count == 0 {
        return None;
    }
    let sk_id = crate_item.bound_owner.as_deref()?;
    let hh = doc
        .households
        .iter()
        .find(|h| h.members.first().map(String::as_str) == Some(sk_id))?;
    let dest_shop_node = hh.node_id.clone();
    let shop_name_zh = doc
        .nodes
        .get(&dest_shop_node)
        .map_or_else(|| "訂貨店鋪".to_string(), |n| n.name.clone());
    let (item_slug, item_name_zh) = if crate_item.name.contains("精鐵") {
        ("iron_ore", "鍛爐補炭精鐵木箱")
    } else if crate_item.name.contains("藥簍") {
        ("herb_bundle", "草藥堂乾苦艾藥簍")
    } else {
        ("home_ration", "酒館黑麥鹽漬補給桶")
    };
    Some(OngoingActivityState {
        kind: "dock_haul".to_string(),
        label_zh: format!("扛送【{}】往{}", item_name_zh, shop_name_zh),
        started_node: cur_node.to_string(),
        target_node: dest_shop_node,
        ticks_remaining: 5,
        cargo_item: Some(CarryItem {
            slug: item_slug.to_string(),
            name: item_name_zh.to_string(),
            count: 3,
            weight_units: 2,
            ..CarryItem::default()
        }),
        payer_id: sk_id.to_string(),
        wage_copper: 5,
    })
}

/// 確保四座港口碼頭每日備有真實連通城內鋪主的待扛貨箱（`dock_cargo_piles`）
pub fn ensure_dock_cargo_piles_initialized(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    game_day: i64,
) {
    let port_defs = [
        ("white_gull_harbor", 4616u32),
        ("west_strait_ferry", 4321u32),
        ("gull_tail_port", 4613u32),
        ("north_cape_beacon", 4073u32),
    ];
    for (slug, _cid) in port_defs {
        let dock_nid = format!("oi:{slug}:dock");
        if !doc.nodes.contains_key(&dock_nid) {
            continue;
        }
        let need_refill = delta
            .dock_cargo_piles
            .get(&dock_nid)
            .is_none_or(Vec::is_empty)
            && delta
                .live_commodities
                .last_port_barge_day
                .get(&dock_nid)
                .copied()
                .is_none_or(|d| d < game_day);
        if !need_refill {
            continue;
        }
        let mut orders = Vec::new();
        for (shop_idx, item_slug, item_zh) in [
            (0usize, "iron_ore", "鍛爐補炭精鐵木箱"),
            (1usize, "home_ration", "酒館黑麥鹽漬補給桶"),
            (3usize, "herb_bundle", "草藥堂乾苦艾藥簍"),
        ] {
            let hh_target = format!("hh:{slug}:shop_{shop_idx}");
            let shop_nid = doc
                .households
                .iter()
                .find(|h| h.id == hh_target)
                .map(|h| h.node_id.clone())
                .unwrap_or_else(|| format!("oi:{slug}:shop_{shop_idx}"));
            let sk_id = format!("res:{slug}:shop_{shop_idx}_0");
            if let Some(snode) = doc.nodes.get(&shop_nid) {
                orders.push(DockCargoOrder {
                    order_id: format!("dc:{slug}:{game_day}:{shop_idx}"),
                    dock_node: dock_nid.clone(),
                    dest_shop_node: shop_nid,
                    shop_name_zh: snode.name.clone(),
                    shopkeeper_id: sk_id,
                    item_slug: item_slug.to_string(),
                    item_name_zh: item_zh.to_string(),
                    count: 3,
                    wage_copper: 5,
                });
            }
        }
        if !orders.is_empty() {
            delta
                .live_commodities
                .last_port_barge_day
                .insert(dock_nid.clone(), game_day);
            delta.dock_cargo_piles.insert(dock_nid, orders);
            delta.extras_dirty = true;
        }
    }
}

/// 每拍（`2.5s`）推進人物進行中之短工序（10s 磨刃、12.5s 酒館短歇旁聽）與到店自動交箱結算
pub fn tick_ongoing_activities(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    let aids: Vec<String> = delta
        .actors
        .iter()
        .filter(|(_, a)| a.alive && a.ongoing_activity.is_some())
        .map(|(k, _)| k.clone())
        .collect();

    let td = derive_tide_clock_details(clock);
    for aid in aids {
        let Some(act_state) = delta
            .actors
            .get(&aid)
            .and_then(|a| a.ongoing_activity.clone())
        else {
            continue;
        };
        let cur_node = delta
            .actors
            .get(&aid)
            .map_or_else(String::new, |a| a.node_id.clone());

        if act_state.kind == "dock_haul" {
            // 抵達目標店鋪即刻交箱入庫並由鋪主發放腳錢！
            if cur_node == act_state.target_node {
                let _ = settle_dock_haul_delivery(doc, delta, clock, &aid);
            }
            continue;
        }

        if act_state.ticks_remaining > 1 {
            if let Some(a) = delta.actors.get_mut(&aid)
                && let Some(ref mut st) = a.ongoing_activity
            {
                st.ticks_remaining -= 1;
                a.mark_mutated();
            }
            continue;
        }

        // 計時屆滿，完成短工序或短歇！
        match act_state.kind.as_str() {
            "quick_sharpen" => {
                if let Some(a) = delta.actors.get_mut(&aid) {
                    a.ongoing_activity = None;
                    a.bound_weapon.wear_stage = a.bound_weapon.wear_stage.saturating_sub(1);
                    push_breath(
                        a,
                        format!(
                            "〔{:02}:{:02}〕砂輪火花收歇，師傅替你的【{}】快速蹭亮刃口，順手遞還。",
                            td.hour, td.minute, a.bound_weapon.name
                        ),
                    );
                    a.mark_mutated();
                }
            }
            "short_rest" => {
                let gossip = compose_live_world_signal_zh(doc, delta);
                if let Some(a) = delta.actors.get_mut(&aid) {
                    a.ongoing_activity = None;
                    a.fatigue = a.fatigue.saturating_sub(28);
                    push_breath(
                        a,
                        format!(
                            "〔{:02}:{:02}〕你在客房靠椅歇過一口長氣，雙腿痠麻消退大半；樓下隱約傳來行旅交談：「{}」",
                            td.hour, td.minute, gossip
                        ),
                    );
                    a.mark_mutated();
                }
            }
            "overland_walk" => {
                // 相容清理任何舊版殘留之跨區行旅狀態，不阻塞零等待移動
                if let Some(a) = delta.actors.get_mut(&aid) {
                    a.ongoing_activity = None;
                    a.mark_mutated();
                }
            }
            _ => {
                if let Some(a) = delta.actors.get_mut(&aid) {
                    a.ongoing_activity = None;
                    a.mark_mutated();
                }
            }
        }
    }
}

/// 產生 100% 源自當下世界真實狀態之活情報（供 4 處官道茶棚與酒館短歇旁聽使用，零假罐頭！）
pub fn compose_live_world_signal_zh(doc: &WorldStaticDoc, delta: &IslandWorldDelta) -> String {
    if let Some(urgent) = super::live_commodities::find_urgent_commodity_rumor_zh(doc, delta) {
        return urgent;
    }
    // 1) 找最近一輛載有鄉間貨箱的在途車船
    if let Some(v) = delta
        .transit_vehicles
        .iter()
        .find(|v| !v.cargo_hold.is_empty())
    {
        let nid = v.route_nodes.get(v.route_idx).map_or("", String::as_str);
        let nname = doc.nodes.get(nid).map_or("官道途中", |n| n.name.as_str());
        let cargo_zh = &v.cargo_hold[0].name_zh;
        return format!(
            "剛見【{}】載著【{}】行經{}，鎮上鋪子正等這批料入庫呢。",
            v.name_zh, cargo_zh, nname
        );
    }
    // 2) 找碼頭尚有待運貨箱之海港
    for (dnid, orders) in &delta.dock_cargo_piles {
        if let Some(ord) = orders.first() {
            let dname = doc.nodes.get(dnid).map_or("海港碼頭", |n| n.name.as_str());
            return format!(
                "{}棧橋上還堆著要送往【{}】的{}，記籌手正招人領籌扛箱，一趟給 {} 枚銅幣。",
                dname, ord.shop_name_zh, ord.item_name_zh, ord.wage_copper
            );
        }
    }
    "風語橋哨的檢疫木欄今日依舊看守嚴密，南岸達利莊與索昂林莊的採艾人正盼著有人幫忙捎平安信回城。"
        .to_string()
}

/// 向港務記籌手領取竹籌並扛起實物貨箱送往城內訂貨店鋪（可中途放棄落地 `DroppedAt`！）
pub fn start_claim_dock_haul(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
) -> Result<String, String> {
    if node.kind != "dock" {
        return Err("需在港務碼頭記籌手處才能領籌扛送鋪貨木箱。".to_string());
    }
    if clock.is_night {
        return Err("夜間鋪門落栓打烊，記籌手已收起竹籌，待天亮再開工吧。".to_string());
    }
    if delta
        .actors
        .get(aid)
        .is_some_and(|a| a.ongoing_activity.is_some())
    {
        return Err("你手頭已有正在進行的活計，請先送達或放下手頭貨箱。".to_string());
    }
    let has_dock_clerk = delta.node_occupants.get(&node.id).is_some_and(|occ| {
        occ.iter().any(|oid| {
            oid != aid
                && delta.actors.get(oid).is_some_and(|o| {
                    o.alive
                        && o.setup_done
                        && o.node_id == node.id
                        && matches!(o.role.as_str(), "港務記籌手" | "碼頭苦力")
                })
        })
    });
    if !has_dock_clerk {
        return Err("棧橋記籌案前此刻暫無人看顧竹籌，稍候記籌手回到岸邊再領箱吧。".to_string());
    }

    ensure_dock_cargo_piles_initialized(doc, delta, clock.game_days);
    delta
        .live_commodities
        .last_port_barge_day
        .insert(node.id.clone(), clock.game_days);
    let Some(order) = delta.dock_cargo_piles.get_mut(&node.id).and_then(|v| {
        if v.is_empty() {
            None
        } else {
            Some(v.remove(0))
        }
    }) else {
        return Err("本班棧橋上的訂貨木箱剛被扛送一空，稍後船駁靠岸即有新箱。".to_string());
    };

    delta.extras_dirty = true;
    let walk_hint = shortest_walk_path(doc, &node.id, &order.dest_shop_node, &BTreeSet::new())
        .map_or(4, |p| p.len().saturating_sub(1));
    let crate_item = CarryItem {
        slug: "dock_cargo_crate".to_string(),
        name: format!("{}（送往{}）", order.item_name_zh, order.shop_name_zh),
        count: 1,
        bound_owner: Some(order.shopkeeper_id.clone()),
        weight_units: 6,
        ..CarryItem::default()
    };

    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    add_carry_item(&mut a.backpack, crate_item.clone());
    a.ongoing_activity = Some(OngoingActivityState {
        kind: "dock_haul".to_string(),
        label_zh: format!("扛送【{}】往{}", order.item_name_zh, order.shop_name_zh),
        started_node: node.id.clone(),
        target_node: order.dest_shop_node.clone(),
        ticks_remaining: u8::try_from(walk_hint.clamp(3, 8)).unwrap_or(5),
        cargo_item: Some(CarryItem {
            slug: order.item_slug,
            name: order.item_name_zh.clone(),
            count: order.count,
            weight_units: 2,
            ..CarryItem::default()
        }),
        payer_id: order.shopkeeper_id,
        wage_copper: order.wage_copper,
    });
    a.mark_mutated();
    let msg = format!(
        "港務記籌手核對竹籌，幫你把沉甸甸的【{}】架上肩頭：請沿街步送往【{}】（約 {} 步路程，送抵即付 {} 枚銅幣；若半路有急事可隨時「放下肩頭貨箱」留在原地）。",
        order.item_name_zh, order.shop_name_zh, walk_hint, order.wage_copper
    );
    push_breath(a, msg.clone());
    Ok(msg)
}

/// 抵達目標店鋪完成實物交箱入庫與腳錢結算
pub fn settle_dock_haul_delivery(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
) -> Result<String, String> {
    let (cur_node, st) = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        let resolved = a.ongoing_activity.clone().or_else(|| {
            a.backpack
                .iter()
                .find(|i| i.slug == "dock_cargo_crate" && i.count > 0)
                .and_then(|cr| reconstruct_dock_haul_from_crate(doc, &a.node_id, cr))
        });
        let Some(s) = resolved else {
            return Err("你目前並未扛送任何港務貨箱。".to_string());
        };
        if s.kind != "dock_haul" {
            return Err("手頭並非港務送貨差事。".to_string());
        }
        (a.node_id.clone(), s)
    };
    if cur_node != st.target_node {
        let dst_name = doc
            .nodes
            .get(&st.target_node)
            .map_or("指定店鋪", |n| n.name.as_str());
        return Err(format!("需走到【{}】門內才能卸箱交貨領取腳錢。", dst_name));
    }

    // 1. 原子移除背囊內封蠟貨箱（嚴格冪等：無實物貨箱即中止，杜絕自動到店與手動觸發雙重入帳！）
    {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        let took_crate = take_carry_item(&mut a.backpack, "dock_cargo_crate", 1).is_some();
        a.ongoing_activity = None;
        a.mark_mutated();
        if !took_crate {
            return Err("肩頭貨箱已點收完畢。".to_string());
        }
        a.fatigue = (a.fatigue + 8).min(95);
        a.mind.stm.today_goal_progressed = true;
    }
    if let Some(real_goods) = st.cargo_item
        && let Some(sk) = delta.actors.get_mut(&st.payer_id)
    {
        let deliver_iron = real_goods.slug == "iron_ore";
        let cnt = real_goods.count;
        let sk_box = sk.inn_lockers.entry(st.target_node.clone()).or_default();
        add_carry_item(sk_box, real_goods);
        if deliver_iron {
            add_carry_item(
                sk_box,
                CarryItem {
                    slug: "black_coal".to_string(),
                    name: "北林精煤塊".to_string(),
                    count: cnt,
                    weight_units: 3,
                    ..CarryItem::default()
                },
            );
        }
        sk.mark_mutated();
    }

    // 2. 由訂貨鋪主支付腳錢（若鋪主手頭現金不足則由港務公庫／銀號墊付，100% 銅幣守恆並核實入帳）
    let can_sk_pay = delta
        .actors
        .get(&st.payer_id)
        .is_some_and(|sk| sk.copper >= st.wage_copper + 6);
    let payer = if can_sk_pay {
        st.payer_id.as_str()
    } else if delta.municipal_treasury_copper >= st.wage_copper {
        "municipal_treasury"
    } else {
        "silver_bank"
    };
    let paid = delta.transfer_copper(
        clock.game_days,
        clock.bell_index,
        "dock_haul_wage",
        payer,
        aid,
        st.wage_copper,
        format!("實物扛箱送抵{}腳錢", st.label_zh),
    );

    let shop_name = doc
        .nodes
        .get(&st.target_node)
        .map_or("鋪內", |n| n.name.as_str());
    let msg = if paid {
        format!(
            "你把肩上貨箱穩穩卸在【{}】櫃台旁，掌櫃開封驗看無誤，收進鋪倉並點給你 {} 枚銅幣腳錢！",
            shop_name, st.wage_copper
        )
    } else {
        format!(
            "你把肩上貨箱穩穩卸在【{}】櫃台旁，掌櫃收進鋪倉並拱手致謝。",
            shop_name
        )
    };
    if let Some(a) = delta.actors.get_mut(aid) {
        push_breath(a, msg.clone());
        a.mark_mutated();
    }
    Ok(msg)
}

/// 隨時中斷手頭搬運或短工序：若肩上有實物貨箱，原物落地寫入當前節點 `dropped_loot_by_node`（絕不憑空蒸發！）
pub fn cancel_ongoing_activity(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    aid: &str,
) -> Result<String, String> {
    let (cur_node, st) = {
        let a = delta.actors.get(aid).ok_or("找不到旅人行蹤")?;
        let resolved = a.ongoing_activity.clone().or_else(|| {
            a.backpack
                .iter()
                .find(|i| i.slug == "dock_cargo_crate" && i.count > 0)
                .and_then(|cr| reconstruct_dock_haul_from_crate(doc, &a.node_id, cr))
        });
        let Some(s) = resolved else {
            return Err("你手頭並無正在進行的工序或搬運。".to_string());
        };
        (a.node_id.clone(), s)
    };
    let node_name = doc.nodes.get(&cur_node).map_or("原地", |n| n.name.as_str());

    if st.kind == "dock_haul" {
        let dropped = {
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            a.ongoing_activity = None;
            a.mark_mutated();
            take_carry_item(&mut a.backpack, "dock_cargo_crate", 1)
        };
        if let Some(crate_item) = dropped {
            delta
                .dropped_loot_by_node
                .entry(cur_node)
                .or_default()
                .push(DroppedLootBag {
                    owner_id: st.payer_id,
                    copper: 0,
                    items: vec![crate_item],
                    promoted_boss_name: None,
                });
            delta.extras_dirty = true;
        }
        let msg = format!(
            "你解開肩繩，將沉重的港務貨箱穩穩放在【{}】路邊石地上（已轉為地面實物行囊，他人或你稍後皆可拾起續送）。",
            node_name
        );
        if let Some(a) = delta.actors.get_mut(aid) {
            push_breath(a, msg.clone());
        }
        return Ok(msg);
    }

    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    a.ongoing_activity = None;
    a.mark_mutated();
    let msg = format!(
        "你收手停下了在【{}】進行中的【{}】。",
        node_name, st.label_zh
    );
    push_breath(a, msg.clone());
    Ok(msg)
}

/// 四處官道避雨茶棚：0 秒熱茶（零鎖死計時器！）、花 1 枚銅幣消解風寒輕微疲勞並聽取當下真實商路情報
pub fn apply_tea_shed_drink(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
) -> Result<String, String> {
    if node.kind != "tea_shed" {
        return Err("需在官道避雨茶棚內才能討碗熱苦艾粗茶歇腳。".to_string());
    }
    if !delta.transfer_copper(
        clock.game_days,
        clock.bell_index,
        "tea_shed",
        aid,
        "municipal_treasury",
        1,
        format!("於{}飲熱茶一碗", node.name),
    ) {
        return Err("討一碗熱苦艾粗茶需要 1 枚銅幣。".to_string());
    }
    let live_rumor = compose_live_world_signal_zh(doc, delta);
    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    a.fatigue = a.fatigue.saturating_sub(18);
    a.mark_mutated();
    let msg = format!(
        "你花 1 枚銅幣在【{}】捧起一碗滾燙的苦艾粗茶一飲而盡，寒氣與腳底痠乏頓時散去大半；同棚歇腳的車夫隨口聊起：「{}」",
        node.name, live_rumor
    );
    push_breath(a, msg.clone());
    Ok(msg)
}

/// 南岸病癒孤莊（達利莊／索昂林莊）封鎖線隔欄交接台：代傳三二六年家書與換取濃煎苦艾湯（一次性人情敘事，零每日農任務！）
pub fn apply_survivor_barrier_exchange(
    _doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
) -> Result<String, String> {
    if node.kind != "survivor_palisade" {
        return Err("需在南岸病癒孤莊的封鎖線隔欄交接台才能隔欄遞信或換湯藥。".to_string());
    }
    let has_letter = delta.actors.get(aid).is_some_and(|a| {
        a.backpack
            .iter()
            .any(|i| i.slug == "plague_family_letter" && i.count > 0)
    });
    if has_letter {
        let deliver_key = format!("{}:{}", aid, node.cell_id);
        let first_time = delta.survivor_letters_delivered.insert(deliver_key);
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        let _ = take_carry_item(&mut a.backpack, "plague_family_letter", 1);
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "wormwood_brew".to_string(),
                name: "孤莊濃煎苦艾湯囊".to_string(),
                count: 2,
                weight_units: 1,
                ..CarryItem::default()
            },
        );
        a.fatigue = a.fatigue.saturating_sub(25);
        a.wound_part = None;
        a.mark_mutated();
        let reward_note = if first_time {
            "木欄內的老莊戶雙手顫抖地接過那封三二六年未寄出的泛黃家書，紅著眼眶連聲道謝，隔著石槽推給你兩囊親手熬製的【孤莊濃煎苦艾湯囊】，並託你回白鷗港或鷗尾漁港時替他們向城裡親族報聲平安！"
        } else {
            "莊戶鄭重收下舊祠堂尋回的書信妥善供存，隔著木欄遞給你兩囊清瘟提神的【孤莊濃煎苦艾湯囊】。"
        };
        let msg = format!("在【{}】前，{}", node.name, reward_note);
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    if !delta.transfer_copper(
        clock.game_days,
        clock.bell_index,
        "wormwood_soup",
        aid,
        "municipal_treasury",
        2,
        format!("於{}隔欄換取苦艾湯", node.name),
    ) {
        return Err(
            "隔欄換取一囊孤莊濃煎苦艾湯需要 2 枚銅幣（或攜來封村遺址尋獲的未寄家書）。".to_string(),
        );
    }
    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    add_carry_item(
        &mut a.backpack,
        CarryItem {
            slug: "wormwood_brew".to_string(),
            name: "孤莊濃煎苦艾湯囊".to_string(),
            count: 1,
            weight_units: 1,
            ..CarryItem::default()
        },
    );
    a.fatigue = a.fatigue.saturating_sub(22);
    a.mark_mutated();
    let msg = format!(
        "你將 2 枚銅幣放入【{}】石槽醋水碗中，木欄內的病癒採艾人推來一囊溫熱清冽的【孤莊濃煎苦艾湯囊】，飲下後神清氣爽。",
        node.name
    );
    push_breath(a, msg.clone());
    Ok(msg)
}

/// 為投宿旅客分配該鎮二樓 1～6 號獨立客房（優先挑選當前無人佔用之房號）
pub fn assign_numbered_inn_room(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    cell_id: u32,
    soul_seed: u64,
) -> Option<(String, String)> {
    let mut rooms: Vec<(&String, &SpatialNodeDef)> = doc
        .nodes
        .iter()
        .filter(|(_, n)| n.cell_id == cell_id && n.kind == "inn_room")
        .collect();
    rooms.sort_by_key(|(id, _)| *id);
    if rooms.is_empty() {
        return None;
    }
    let start = (soul_seed as usize) % rooms.len();
    for offset in 0..rooms.len() {
        let (rid, rnode) = rooms[(start + offset) % rooms.len()];
        let occupied = delta
            .node_occupants
            .get(rid.as_str())
            .is_some_and(|s| !s.is_empty());
        if !occupied {
            return Some((rid.clone(), rnode.name.clone()));
        }
    }
    let (rid, rnode) = rooms[start];
    Some((rid.clone(), rnode.name.clone()))
}

/// 二樓客房或酒館短歇回氣（`5` 拍 = `12.5` 秒）：不扣過夜宿數，快速消減奔波疲勞並旁聽樓下活情報
pub fn start_inn_short_rest(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    aid: &str,
    node: &SpatialNodeDef,
) -> Result<String, String> {
    if node.kind != "inn_room" && !(node.kind == "shop" && node.name.contains("酒館")) {
        return Err("需在酒館大堂或二樓客房內才能靠椅短歇回氣。".to_string());
    }
    let rumor = compose_live_world_signal_zh(doc, delta);
    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    a.fatigue = a.fatigue.saturating_sub(24);
    a.ongoing_activity = Some(OngoingActivityState {
        kind: "short_rest".to_string(),
        label_zh: "靠椅短歇回氣".to_string(),
        started_node: node.id.clone(),
        target_node: node.id.clone(),
        ticks_remaining: 4,
        ..OngoingActivityState::default()
    });
    a.mark_mutated();
    let msg = format!(
        "你在【{}】鬆開行囊靠椅歇息片刻（約十餘秒完全舒緩雙腿痠麻，隨時可起身離開），耳邊聽見鄰座旅人聊起：「{}」",
        node.name, rumor
    );
    push_breath(a, msg.clone());
    Ok(msg)
}

/// 百工二階委託：下訂「1 鐘響（5 分鐘）精鍛銘器委託單」（`commission_custom_vessel`）與到鐘取件（`collect_custom_vessel`）
pub fn handle_custom_forge_commission(
    _doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    smith_id: Option<&str>,
    is_collect: bool,
) -> Result<String, String> {
    let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
    if is_collect {
        let ready_idx = delta.forge_orders.get(&node.id).and_then(|list| {
            list.iter()
                .position(|o| o.customer_id == aid && abs_bell >= o.ready_abs_bell)
        });
        let Some(idx) = ready_idx else {
            return Err(
                "你委託精鍛的器物尚在爐中淬火錘鍊（需滿一記鐘響方可出爐取件）。".to_string(),
            );
        };
        let order = delta
            .forge_orders
            .get_mut(&node.id)
            .expect("forge list exists")
            .remove(idx);
        delta.extras_dirty = true;
        let vname = order.vessel_tool.name.clone();
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: order.vessel_tool.slug.clone(),
                name: vname.clone(),
                count: 1,
                bound_owner: None,
                weight_units: 3,
                vessel_meta: Some(order.vessel_tool),
                ..CarryItem::default()
            },
        );
        a.mark_mutated();
        let msg = format!(
            "你從鍛爐取件架領回淬火完畢的【{}】！器脊落款字跡深峻。",
            vname
        );
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    if !node.name.contains("鍛鐵") && !node.name.contains("鐵匠") && !node.name.contains("集坊")
    {
        return Err("需在鍛鐵鋪或百工集坊才能下訂精鍛委託。".to_string());
    }
    let Some(payee) = smith_id else {
        return Err("石爐壓著微溫灰炭，師傅與學徒此刻不在爐邊，暫無法下訂精鍛委託。".to_string());
    };
    if super::live_commodities::is_smithy_forge_cold(delta, &node.id) {
        return Err(
            "鍛爐裡的精煤與木炭已燒盡熄火，師傅正高價收購北林精煤、木炭或乾木重開爐火！"
                .to_string(),
        );
    }
    let forge_price = super::live_commodities::quote_custom_forge_copper(delta, &node.id);
    let (quartz_cnt, iron_cnt) = delta.actors.get(aid).map_or((0u32, 0u32), |a| {
        let q: u32 = a
            .backpack
            .iter()
            .filter(|i| {
                i.slug == "quartz_sand" && i.bound_owner.is_none() && i.stolen_from.is_none()
            })
            .map(|i| i.count)
            .sum();
        let ir: u32 = a
            .backpack
            .iter()
            .filter(|i| i.slug == "iron_ore" && i.bound_owner.is_none() && i.stolen_from.is_none())
            .map(|i| i.count)
            .sum();
        (q, ir)
    });
    if quartz_cnt < 2 && iron_cnt < 2 {
        return Err(format!(
            "委託精鍛一柄落款良器需自備 2 份清白【粗鐵礦石】或【晶瑩石英白沙袋】與 {} 枚銅幣工資。",
            forge_price
        ));
    }
    let used_fuel = super::live_commodities::consume_smithy_fuel_one(delta, Some(payee), &node.id);
    if !delta.transfer_copper(
        clock.game_days,
        clock.bell_index,
        "custom_forge_order",
        aid,
        payee,
        forge_price,
        format!("於{}下訂一鐘精鍛器物委託", node.name),
    ) {
        if let Some((holder, item)) = used_fuel {
            super::live_commodities::restore_shop_stock_one(delta, &holder, &node.id, item);
        }
        return Err(format!("委託精鍛需要 {} 枚銅幣工資。", forge_price));
    }
    let smith_name = delta
        .actors
        .get(payee)
        .map_or_else(|| "匠師".to_string(), |s| s.display_name.clone());
    let customer_name = delta
        .actors
        .get(aid)
        .map_or_else(|| "旅人".to_string(), |a| a.display_name.clone());

    let used_quartz = quartz_cnt >= 2;
    {
        let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
        let target_slug = if used_quartz {
            "quartz_sand"
        } else {
            "iron_ore"
        };
        let mut need = 2u32;
        for it in &mut a.backpack {
            if need == 0 {
                break;
            }
            if it.slug == target_slug
                && it.bound_owner.is_none()
                && it.stolen_from.is_none()
                && it.count > 0
            {
                let d = it.count.min(need);
                it.count -= d;
                need -= d;
            }
        }
        a.backpack.retain(|i| i.count > 0);
        a.mark_mutated();
    }
    delta.extras_dirty = true;
    let tool_name = if used_quartz {
        format!("{}精煉白沙透光刃", smith_name)
    } else {
        format!("{}百鍛厚脊長刃", smith_name)
    };
    let order = CustomForgeOrder {
        order_id: format!("fo:{}:{abs_bell}:{aid}", node.cell_id),
        shop_node: node.id.clone(),
        customer_id: aid.to_string(),
        smith_id: payee.to_string(),
        ready_abs_bell: abs_bell + 1,
        vessel_tool: BoundTool {
            uid: format!("vessel:{aid}:{abs_bell}"),
            slug: "forged_blade".to_string(),
            name: tool_name.clone(),
            provenance: format!(
                "太陽曆第 {} 日由{}為{}開爐精鍛一鐘而成",
                clock.game_days, smith_name, customer_name
            ),
            wear_stage: 0,
            grip_familiarity: 16,
            rune_mark: used_quartz.then(|| "白沙澄明紋".to_string()),
        },
    };
    delta
        .forge_orders
        .entry(node.id.clone())
        .or_default()
        .push(order);
    if let Some(a) = delta.actors.get_mut(aid) {
        let msg = format!(
            "你交付礦料與 6 枚銅幣訂下【{}】，匠師已將毛坯送入紅爐——約一記鐘響（5 分鐘）後即可回鋪領取成器，期間可自由外出辦事！",
            tool_name
        );
        push_breath(a, msg.clone());
        a.mark_mutated();
        return Ok(msg);
    }
    Ok("已受理精鍛委託。".to_string())
}

pub fn try_dispatch_embodied_action(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    smith_id: Option<&str>,
    action: &str,
) -> Option<Result<String, String>> {
    match action {
        "claim_dock_haul" => Some(start_claim_dock_haul(doc, delta, clock, aid, node)),
        "deliver_dock_crate" => Some(settle_dock_haul_delivery(doc, delta, clock, aid)),
        "cancel_activity" => Some(cancel_ongoing_activity(doc, delta, aid)),
        "drink_tea_shed" => Some(apply_tea_shed_drink(doc, delta, clock, aid, node)),
        "survivor_barrier" => Some(apply_survivor_barrier_exchange(
            doc, delta, clock, aid, node,
        )),
        "rest_short" => Some(start_inn_short_rest(doc, delta, aid, node)),
        "commission_custom_vessel" => Some(handle_custom_forge_commission(
            doc, delta, clock, aid, node, smith_id, false,
        )),
        "collect_custom_vessel" => Some(handle_custom_forge_commission(
            doc, delta, clock, aid, node, smith_id, true,
        )),
        _ => None,
    }
}
