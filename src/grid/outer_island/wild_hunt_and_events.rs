//! 第四刀（下・拆骨重構版）：真實一次性世界危機協作 × 野外巨獸新鮮蹤跡追獵（徹底拔除每日打卡公庫提款機與設計黑話）。
//!
//! 嚴守 `2026-11-20` 總綱核心試金石與 §4-2 鐵律：
//! 1. 絕不設「每天重置白領公庫錢」的假任務！
//!    - 棧橋搶貨：只在真實白晝開潮（`!is_night && is_dock_cargo_tide_open`）且棧橋有貨時浮現，與殘夜敘述零衝突；
//!    - 護橋搶修：只在真實某處渡口橋樑受損（`bridge_integrity < 90`）時浮現，必耗實木將橋修回完好（修好危機即解除）；
//!    - 鍛爐助燃：只在城內鍛鐵鋪真實冷爐缺煤（`is_smithy_forge_cold` 或燃料 `<= 1`）時浮現，必交實炭並真正補回鍛爐庫存重燃爐火；
//!    - 巡防哨報：零免費印鈔，僅分享區域風聲傳聞，須親赴野外循跡方能逐步鎖定太古巨物去路。
//! 2. 全程使用世界內自然白話，零幕後設計術語與原始數字外洩。

use serde_json::{Value, json};

use super::clock_and_lang::SolarClockSnapshot;
use super::live_commodities::{
    count_smithy_fuel, is_smithy_forge_cold, quote_specialty_buyback_copper, restore_shop_stock_one,
};
use super::live_land::ensure_live_land_seeded;
use super::martial_art_and_manuals::invoke_craft_phase_effect;
use super::mind::EventPublicity;
use super::movement::{push_breath, shortest_node_path, take_clean_carry_items};
use super::persona::ValueAxis;
use super::shop_leases::infer_shop_kind;
use super::sim_tick::broadcast_scene_event;
use super::types::{
    ApexBeastDef, IslandWorldDelta, SkirmishState, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc,
};

#[derive(Debug, Clone)]
pub struct ApexTrailHint {
    pub beast: ApexBeastDef,
    pub target_cell_id: u32,
    pub target_cell_title: String,
    pub target_node_id: String,
    pub target_node_name: String,
    pub next_hop_name: String,
    pub next_hop_dir: String,
    pub distance_sense_zh: &'static str,
    pub at_lair: bool,
}

fn dir_zh(dir: &str) -> &'static str {
    match dir {
        "north" | "n" => "北",
        "south" | "s" => "南",
        "east" | "e" => "東",
        "west" | "w" => "西",
        "up" | "u" => "上",
        "down" | "d" => "下",
        "in" => "內",
        "out" => "外",
        _ => "前",
    }
}

/// 依真實世界節點拓撲，找出距離當前位置最近且尚未伏誅的太古巨物真實蹤跡方向。
pub fn find_nearest_living_apex_trail(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    from_node_id: &str,
) -> Option<ApexTrailHint> {
    let mut best: Option<(usize, ApexTrailHint)> = None;
    for cell in doc.cells.values() {
        let Some(ref ab) = cell.apex_beast else {
            continue;
        };
        if delta.slain_apex_beasts.contains_key(&cell.cell_id) {
            continue;
        }
        let Some(target_nd) = doc
            .nodes
            .values()
            .find(|n| n.cell_id == cell.cell_id && !n.indoor && !n.dungeon)
        else {
            continue;
        };
        if target_nd.id == from_node_id {
            return Some(ApexTrailHint {
                beast: ab.clone(),
                target_cell_id: cell.cell_id,
                target_cell_title: cell.title.clone(),
                target_node_id: target_nd.id.clone(),
                target_node_name: target_nd.name.clone(),
                next_hop_name: target_nd.name.clone(),
                next_hop_dir: "眼前".to_string(),
                distance_sense_zh: "氣息就在眼前巢穴深處",
                at_lair: true,
            });
        }
        let Some(path) = shortest_node_path(doc, from_node_id, &target_nd.id) else {
            continue;
        };
        if path.len() < 2 {
            continue;
        }
        let hops = path.len() - 1;
        let next_id = &path[1];
        let next_name = doc
            .nodes
            .get(next_id)
            .map_or_else(|| next_id.clone(), |n| n.name.clone());
        let next_dir = doc
            .nodes
            .get(from_node_id)
            .and_then(|cur| cur.exits.iter().find(|e| e.to == *next_id))
            .map_or("前", |e| dir_zh(&e.dir))
            .to_string();
        let dist_sense = if hops <= 2 {
            "足印仍帶著濕熱地氣，距巢穴近在咫尺"
        } else if hops <= 6 {
            "新鮮刮痕一路延伸，約莫半日腳程內"
        } else {
            "遠方山海間隱有重壓傳來，沿此去路可一路循跡深入"
        };
        let cand = ApexTrailHint {
            beast: ab.clone(),
            target_cell_id: cell.cell_id,
            target_cell_title: cell.title.clone(),
            target_node_id: target_nd.id.clone(),
            target_node_name: target_nd.name.clone(),
            next_hop_name: next_name,
            next_hop_dir: next_dir,
            distance_sense_zh: dist_sense,
            at_lair: false,
        };
        if best.as_ref().is_none_or(|(bh, _)| hops < *bh) {
            best = Some((hops, cand));
        }
    }
    best.map(|(_, h)| h)
}

pub fn apply_track_wild_beast(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    _clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
) -> Result<String, String> {
    if node.indoor
        || node.dungeon
        || doc
            .cells
            .get(&node.cell_id.to_string())
            .is_some_and(|c| c.kind == "settlement")
    {
        return Err("城鎮石板街區裡沒有荒野巨獸足跡可辨，請出城踏入山野林地再循跡查探。".to_string());
    }
    let Some(trail) = find_nearest_living_apex_trail(doc, delta, &node.id) else {
        return Ok("你俯身細察泥土與斷枝，島上三大太古巨物皆已伏誅，山海荒野如今一片平靜。".to_string());
    };
    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    let (craft_prose, save) = invoke_craft_phase_effect(a, "wayfare");
    a.known_rumors.insert(format!("apex_trail:{}", trail.beast.slug));
    a.mind.stm.today_goal_progressed = true;

    if trail.at_lair {
        a.active_skirmish = Some(SkirmishState {
            foe_slug: trail.beast.slug.clone(),
            foe_name: trail.beast.name_zh.clone(),
            can_speak_goblin: false,
            foe_stagger: 0,
            player_guard: false,
            round_idx: 1,
        });
        a.mark_mutated();
        let msg = format!(
            "你順著新鮮刮痕撥開岩影（{craft_prose}）——{}【{}】驀然抬首低吼，沉重威壓封住退路，當場進入臨陣對峙！",
            trail.beast.title_zh, trail.beast.name_zh
        );
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    a.fatigue = a.fatigue.saturating_add(4u8.saturating_sub(save).max(2)).min(95);
    a.mark_mutated();
    let msg = format!(
        "你俯身辨識草泥間的巨獸新鮮蹤跡（{craft_prose}）：這是{}【{}】留下的壓痕（{}），往【{}方・{}】延伸而去——{}。",
        trail.beast.title_zh,
        trail.beast.name_zh,
        trail.distance_sense_zh,
        trail.next_hop_dir,
        trail.next_hop_name,
        trail.beast.lair_hint_zh
    );
    push_breath(a, msg.clone());
    Ok(msg)
}

#[derive(Debug, Clone)]
pub struct ActiveTownCrisisSpec {
    pub kind: &'static str,
    pub title_zh: &'static str,
    pub cue_zh: String,
    pub desc_zh: String,
    pub action_label_zh: &'static str,
    pub action_sub_zh: &'static str,
}

fn find_settlement_low_fuel_smithy(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    cell_id: u32,
) -> Option<(String, String)> {
    doc.nodes
        .values()
        .find(|n| {
            n.cell_id == cell_id
                && n.kind == "shop"
                && (infer_shop_kind(n) == "smithy" || n.name.contains("鍛鐵"))
                && (is_smithy_forge_cold(delta, &n.id) || count_smithy_fuel(delta, &n.id) <= 1)
        })
        .map(|n| (n.id.clone(), n.name.clone()))
}

fn find_damaged_bridge_cell(delta: &IslandWorldDelta) -> Option<u32> {
    delta
        .live_land
        .cells
        .iter()
        .find(|(_, cl)| cl.has_river_crossing && cl.bridge_integrity < 90)
        .map(|(cid, _)| *cid)
}

pub fn build_active_town_events(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
) -> Vec<ActiveTownCrisisSpec> {
    let mut out: Vec<ActiveTownCrisisSpec> = Vec::new();

    // 危機 A：僅於全島真實有受損斷橋（integrity < 90）時才出現（修完即消失，絕不每日重置！）
    if matches!(node.kind.as_str(), "plaza" | "civic" | "gate" | "bridge")
        && let Some(cid) = find_damaged_bridge_cell(delta)
    {
        let bridge_title = doc
            .cells
            .get(&cid.to_string())
            .map_or("官道渡口", |c| c.title.as_str());
        out.push(ActiveTownCrisisSpec {
            kind: "bridge_relief_drive",
            title_zh: "官道斷橋・工務急收修橋木料",
            cue_zh: format!("【{bridge_title}】木橋遭急水沖鬆，急需原木加固橋墩"),
            desc_zh: "這處官道橋樑受損未修前，商隊與酒館糧價都會受牽連；繳交 1 份原木或漂流木即可實地修牢這座斷橋並領工料賞銀。".to_string(),
            action_label_zh: "繳交木料搶修受損官道橋 · 6 枚銅幣",
            action_sub_zh: "須自備灰松原木或漂流木 1 份，修妥後即恢復通行",
        });
    }

    // 危機 C：僅於本城鍛鐵鋪真實冷爐或燃料 <= 1 時才出現（必交真煤並補進鍛爐庫存重燃，重燃即消失！）
    if (node.name.contains("鐵匠") || node.name.contains("鍛鐵"))
        && let Some((_, sm_name)) = find_settlement_low_fuel_smithy(doc, delta, node.cell_id)
    {
        out.push(ActiveTownCrisisSpec {
            kind: "forge_ember_relay",
            title_zh: "鍛爐斷炭・急收精煤木炭重燃",
            cue_zh: format!("【{sm_name}】爐膛炭火告急，高價急收燃煤木炭"),
            desc_zh: "鍛爐一旦熄火，整條鐵匠街都無法開爐修刃與打造；把隨身北林精煤、硬木炭或漂流木送進爐倉即可立刻重燃爐火並按高價結算。".to_string(),
            action_label_zh: "交入燃煤木料重燃鍛爐",
            action_sub_zh: "須自備精煤、木炭或漂流木 1 份，實補鍛爐庫存",
        });
    }

    // 風聲 D：城鎮巡防巨獸傳聞（零免費發錢！只給區域風聞，真追獵須出城進山辨識足跡）
    let rumor_key = format!("watch_rumor@d{}", clock.game_days);
    if matches!(node.kind.as_str(), "gate" | "citadel" | "plaza")
        && find_nearest_living_apex_trail(doc, delta, &node.id).is_some()
        && !viewer.explored_windows.contains(&rumor_key)
    {
        out.push(ActiveTownCrisisSpec {
            kind: "watch_bounty_brief",
            title_zh: "城門巡哨木牌・山海異獸風聞",
            cue_zh: "巡哨貼出近日入山獵戶與漁船帶回的荒野巨獸傳聞".to_string(),
            desc_zh: "牌上記著近日山海間流傳的巨獸出沒地界，細讀可記下大概區域，想追上牠還得出城入野細查足跡。".to_string(),
            action_label_zh: "細讀巡哨記下的巨獸傳聞",
            action_sub_zh: "記下出沒大略地界（出城入野後可俯身循跡追索）",
        });
    }

    out
}

pub fn apply_resolve_town_event(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    event_kind: &str,
) -> Result<String, String> {
    ensure_live_land_seeded(doc, delta, clock.game_days);
    let kind = event_kind.trim();

    let (msg, witness_line) = match kind {
        "bridge_relief_drive" => {
            if !matches!(node.kind.as_str(), "plaza" | "civic" | "gate" | "bridge") {
                return Err("需在廣場、市政廳、城門或橋頭繳交修橋木料。".to_string());
            }
            let Some(cid) = find_damaged_bridge_cell(delta) else {
                return Err("島上各處官道橋樑目前皆完好無損，並無待修斷橋。".to_string());
            };
            let wood_item = {
                let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
                let Some(it) = take_clean_carry_items(&mut a.backpack, "timber_log", 1)
                    .or_else(|| take_clean_carry_items(&mut a.backpack, "driftwood", 1))
                else {
                    return Err("修繕受損官道橋須實繳 1 份【灰松厚原木】或【異國刻紋漂流木】。".to_string());
                };
                it
            };
            if let Some(cl) = delta.live_land.cells.get_mut(&cid) {
                cl.bridge_integrity = 100;
                cl.bridge_down_days = 0;
                delta.extras_dirty = true;
            }
            let paid = delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "town_event_bridge",
                "municipal_treasury",
                aid,
                6,
                format!("繳交【{}】搶修第 {cid} 區斷橋賞銀", wood_item.name),
            );
            let bridge_title = doc
                .cells
                .get(&cid.to_string())
                .map_or("官道木橋", |c| c.title.as_str());
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            a.fatigue = a.fatigue.saturating_add(8).min(95);
            a.mind.stm.today_goal_progressed = true;
            a.mark_mutated();
            let pay_note = if paid {
                "工務依例撥付 6 枚銅幣工料費"
            } else {
                "工務鄭重記下了你的功勞"
            };
            let m = format!(
                "你交出【{}】協助工務打樁換樑，【{bridge_title}】橋基重歸穩固恢復車馬通行，{pay_note}。",
                wood_item.name
            );
            push_breath(a, m.clone());
            (m, format!("繳料搶修了【{bridge_title}】受損官道橋"))
        }
        "forge_ember_relay" => {
            if !node.name.contains("鐵匠") && !node.name.contains("鍛鐵") {
                return Err("需在鐵匠街或鍛鐵鋪旁才能交入爐炭助燃。".to_string());
            }
            let Some((smithy_nid, smithy_name)) =
                find_settlement_low_fuel_smithy(doc, delta, node.cell_id)
            else {
                return Err("附近鍛爐炭火正旺、存煤充足，此刻並不缺燃煤。".to_string());
            };
            let fuel_item = {
                let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
                let Some(it) = take_clean_carry_items(&mut a.backpack, "black_coal", 1)
                    .or_else(|| take_clean_carry_items(&mut a.backpack, "charcoal_sack", 1))
                    .or_else(|| take_clean_carry_items(&mut a.backpack, "timber_log", 1))
                    .or_else(|| take_clean_carry_items(&mut a.backpack, "driftwood", 1))
                else {
                    return Err(
                        "助燃重開鍛爐須實交 1 份【北林精煤塊】、【耐燒硬木炭袋】或木料。".to_string(),
                    );
                };
                it
            };
            // 真正補進鍛爐持有者庫存，讓冷爐實打實重燃！
            let smith_holder = delta
                .actors
                .values()
                .find(|o| {
                    o.alive
                        && o.controller_id.is_none()
                        && (o.work_node == smithy_nid || o.node_id == smithy_nid)
                        && (o.role.contains("鍛鐵") || o.role.contains("鐵匠"))
                })
                .map(|o| o.actor_id.clone());
            let unit_price = quote_specialty_buyback_copper(
                doc,
                delta,
                node,
                smith_holder.as_deref(),
                &fuel_item,
            )
            .unwrap_or(5);
            let fuel_name = fuel_item.name.clone();
            if let Some(ref sid) = smith_holder {
                restore_shop_stock_one(delta, sid, &smithy_nid, fuel_item);
            } else {
                // 若鋪內暫無常駐鐵匠，至少保留在鋪面地面物資箱以免蒸發
                delta
                    .dropped_loot_by_node
                    .entry(smithy_nid.clone())
                    .or_default()
                    .push(super::types::DroppedLootBag {
                        owner_id: aid.to_string(),
                        copper: 0,
                        items: vec![fuel_item],
                        promoted_boss_name: None,
                    });
            }
            let payer = smith_holder.as_deref().unwrap_or("municipal_treasury");
            let paid = delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "town_event_forge",
                payer,
                aid,
                unit_price,
                format!("急收【{fuel_name}】重燃【{smithy_name}】鍛爐"),
            );
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            a.fatigue = a.fatigue.saturating_add(5).min(95);
            a.mind.stm.today_goal_progressed = true;
            a.mark_mutated();
            let pay_note = if paid {
                format!("按急收市價領得 {unit_price} 枚銅幣")
            } else {
                "師傅記下了這筆炭帳並連聲道謝".to_string()
            };
            let m = format!(
                "你將【{fuel_name}】實實在在送進【{smithy_name}】炭倉重燃爐火，爐膛重新泛起亮橘火光，{pay_note}。"
            );
            push_breath(a, m.clone());
            (m, format!("送入【{fuel_name}】重燃了【{smithy_name}】熄火的鍛爐"))
        }
        "watch_bounty_brief" => {
            if !matches!(node.kind.as_str(), "gate" | "citadel" | "plaza") {
                return Err("需在城門、軍堡或廣場巡哨木牌前查閱巨獸風聞。".to_string());
            }
            let Some(tr) = find_nearest_living_apex_trail(doc, delta, &node.id) else {
                return Ok("木牌上貼著近日四郊安靖的簡報，島上三大太古巨物皆已伏誅。".to_string());
            };
            let rumor_key = format!("watch_rumor@d{}", clock.game_days);
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            a.explored_windows.insert(rumor_key);
            a.known_rumors.insert(format!("apex_trail:{}", tr.beast.slug));
            a.mark_mutated();
            let m = format!(
                "你細讀城門巡哨記下的荒野風聞：近日跑船與進山獵戶傳回消息，{}【{}】確在【{}】一帶出沒——「{}」想找著牠的去處，還得出城進山野俯身查認新鮮足跡。",
                tr.beast.title_zh, tr.beast.name_zh, tr.target_cell_title, tr.beast.lair_hint_zh
            );
            push_breath(a, m.clone());
            (m, "查閱了巡哨木牌上的荒野巨獸風聞".to_string())
        }
        _ => return Err("眼前並無這項城鎮急務。".to_string()),
    };

    let actor_name = delta
        .actors
        .get(aid)
        .map_or_else(|| "旅人".to_string(), |a| a.display_name.clone());
    broadcast_scene_event(
        doc,
        delta,
        clock,
        &node.id,
        aid,
        &actor_name,
        EventPublicity::PublicLoud,
        None,
        50,
        30,
        ValueAxis::PactIntegrity,
        "resolve_town_event",
        &format!("{actor_name}{witness_line}"),
    );
    Ok(msg)
}

pub fn try_apply_wild_hunt_event_action(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    node: &SpatialNodeDef,
    action: &str,
    param: &str,
) -> Option<Result<String, String>> {
    match action {
        "track_wild_beast" => Some(apply_track_wild_beast(doc, delta, clock, aid, node)),
        "resolve_town_event" => Some(apply_resolve_town_event(doc, delta, clock, aid, node, param)),
        _ => None,
    }
}

pub fn build_hunt_clue_view_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
) -> Value {
    let known_any = viewer.known_rumors.iter().any(|r| r.starts_with("apex_trail:"));
    if !known_any {
        return json!({ "active": false });
    }
    let Some(tr) = find_nearest_living_apex_trail(doc, delta, &viewer.node_id) else {
        return json!({ "active": false });
    };
    let in_settlement = doc
        .nodes
        .get(&viewer.node_id)
        .and_then(|n| doc.cells.get(&n.cell_id.to_string()))
        .is_some_and(|c| c.kind == "settlement");
    let next_hint = if tr.at_lair {
        "就在眼前巢穴深處，隨時可循跡正面迎擊".to_string()
    } else if in_settlement {
        format!("傳聞盤踞於【{}】一帶（出城入野後可查認足跡辨明去路）", tr.target_cell_title)
    } else {
        format!(
            "足跡朝【{}方・{}】延伸（{}）",
            tr.next_hop_dir, tr.next_hop_name, tr.distance_sense_zh
        )
    };
    json!({
        "active": true,
        "beast_name": tr.beast.name_zh,
        "beast_title": tr.beast.title_zh,
        "next_hint": next_hint,
    })
}

pub fn append_wild_hunt_and_town_event_objects(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
    objects: &mut Vec<Value>,
    push_audit: &mut impl FnMut(&str, &str, &str, &str, &str, &str),
) {
    let is_wild_outdoor = !node.indoor
        && !node.dungeon
        && doc
            .cells
            .get(&node.cell_id.to_string())
            .is_some_and(|c| c.kind != "settlement");
    if is_wild_outdoor
        && let Some(tr) = find_nearest_living_apex_trail(doc, delta, &node.id)
    {
        let (name, cue, lbl, sub) = if tr.at_lair {
            (
                format!("【{}】新鮮巢痕", tr.beast.name_zh),
                "岩土碎裂、壓痕仍帶著餘溫，巨獸就在眼前不遠處！".to_string(),
                format!("循跡驚動【{}】正面迎擊", tr.beast.name_zh),
                "直面太古巨物展開臨陣交鋒",
            )
        } else {
            (
                "新鮮巨獸足印與樹皮刮痕".to_string(),
                format!("泥地留著沉重獸印，隱約指向【{}方】", tr.next_hop_dir),
                "俯身辨識巨獸新鮮蹤跡".to_string(),
                "順著泥痕辨明太古巨物去路",
            )
        };
        objects.push(json!({
            "object_id": "obj:apex_beast_trail",
            "name": name,
            "cue": cue,
            "desc": format!("{}。細查泥痕與斷枝折向，便能順著去路一路追索至其盤踞之地。", tr.beast.lair_hint_zh),
            "direct_action": Value::Null,
            "options": [{
                "id": "track_wild_beast",
                "param": "",
                "label": lbl,
                "sub": sub,
            }],
        }));
        push_audit("track_wild_beast", "", &lbl, sub, "object", "obj:apex_beast_trail");
    }

    let evs = build_active_town_events(doc, delta, viewer, node, clock);
    if !evs.is_empty() {
        let mut opts: Vec<Value> = Vec::new();
        for ev in &evs {
            opts.push(json!({
                "id": "resolve_town_event",
                "param": ev.kind,
                "label": ev.action_label_zh,
                "sub": ev.action_sub_zh,
            }));
            push_audit(
                "resolve_town_event",
                ev.kind,
                ev.action_label_zh,
                ev.action_sub_zh,
                "object",
                "obj:town_live_event",
            );
        }
        let first = &evs[0];
        objects.push(json!({
            "object_id": "obj:town_live_event",
            "name": first.title_zh,
            "cue": first.cue_zh,
            "desc": first.desc_zh,
            "direct_action": Value::Null,
            "options": opts,
        }));
    }
}

/// 玩家 ≡ NPC 同構日常推演（由 `sim_tick` 每鐘點對少量清醒 NPC 觸發，嚴守銅幣守恆與零無中生有）：
/// 1. 持藤枝原材且無護具者，依同一套 `apply_field_bind_craft`＋`apply_equip_body_gear` 自綁護腕／涉水履；
/// 2. 在可耕作野外／鄉村節點且行囊富餘穀種／草束（`>=2`）者，依同一套 `apply_sow_farm_plot`／`tend`／`harvest` 真實耕作；
/// 3. 疲累且身處開門酒館、銅幣充裕者，依同一套 `apply_take_warm_bath`／`apply_enjoy_hot_meal`（必扣真庫存、真付掌櫃）；
/// 4. 同場兩名清醒 NPC 於背景真實交換見聞並雙向寫入記憶（讓現場群戲源自真實背景互動而非僅渲染模板）。
pub fn tick_npc_unified_world_habits(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    abs_bell: i64,
) {
    let (node_id, fatigue, hunger, copper, has_vine, has_branch, no_arms, has_spare_seed, my_name, idle_at_node) = {
        let Some(a) = delta.actors.get(aid) else {
            return;
        };
        if !a.alive
            || a.controller_id.is_some()
            || a.active_skirmish.is_some()
            || a.active_transit.is_some()
            || a.ongoing_activity.is_some()
        {
            return;
        }
        // 以確定性輪轉節流（每人每 4 鐘輪到一次），兼顧 994 人全島效能與真實活感
        if ((a.soul_seed ^ (abs_bell.unsigned_abs())) % 4) != 0 {
            return;
        }
        let vine_n: u32 = a
            .backpack
            .iter()
            .filter(|i| i.slug == "tough_vine" && i.bound_owner.is_none() && i.stolen_from.is_none())
            .map(|i| i.count)
            .sum();
        let branch_n: u32 = a
            .backpack
            .iter()
            .filter(|i| i.slug == "dry_branch" && i.bound_owner.is_none() && i.stolen_from.is_none())
            .map(|i| i.count)
            .sum();
        let spare_seed = a
            .backpack
            .iter()
            .find(|i| {
                i.count >= 3
                    && i.bound_owner.is_none()
                    && i.stolen_from.is_none()
                    && matches!(i.slug.as_str(), "wheat_grain" | "herb_bundle")
            })
            .map(|i| i.slug.clone());
        (
            a.node_id.clone(),
            a.fatigue,
            a.mind.stm.affect.hunger,
            a.copper,
            vine_n >= 2,
            branch_n >= 1 && vine_n >= 1,
            a.body_gear.arms.is_none(),
            spare_seed,
            a.display_name.clone(),
            a.route_plan.is_empty(),
        )
    };
    let Some(node) = doc.nodes.get(&node_id) else {
        return;
    };

    // 1. 自綁並穿戴多部位護具（與玩家共用同一函式！）
    if no_arms && has_vine {
        if super::body_gear_craft::apply_field_bind_craft(delta, clock, aid, "vine_bracers").is_ok() {
            let _ = super::body_gear_craft::apply_equip_body_gear(delta, clock, aid, "gear_vine_bracers");
        }
        return;
    } else if has_branch
        && delta
            .actors
            .get(aid)
            .is_some_and(|a| a.body_gear.feet.is_none())
    {
        if super::body_gear_craft::apply_field_bind_craft(delta, clock, aid, "reed_boots").is_ok() {
            let _ = super::body_gear_craft::apply_equip_body_gear(delta, clock, aid, "gear_reed_boots");
        }
        return;
    }

    // 飢餓急迫者先顧覓食生計，不因閒聊或付費泡澡耽誤飯碗
    if hunger >= 55 {
        return;
    }

    // 2. 農田照料／收割／播種（與玩家共用同一套真實種子與 `farm_plots`！）
    let plot_key = format!("plot:{node_id}:{aid}");
    if let Some(plot) = delta.farm_plots.get(&plot_key).cloned() {
        if clock.game_days >= plot.mature_day {
            let _ = super::living_sim_and_farm::apply_harvest_farm_plot(
                doc, delta, clock, aid, node, &plot.plot_id,
            );
            return;
        } else if plot.tended_day != clock.game_days {
            let _ = super::living_sim_and_farm::apply_tend_farm_plot(
                delta, clock, aid, node, &plot.plot_id,
            );
            return;
        }
    } else if !clock.is_night
        && super::living_sim_and_farm::is_farmable_node(doc, node)
        && delta.farm_plots.len() < 48
        && let Some(seed_slug) = has_spare_seed
    {
        let crop_kind = if seed_slug == "wheat_grain" {
            "dry_wheat"
        } else {
            "herb_garden"
        };
        if super::living_sim_and_farm::apply_sow_farm_plot(doc, delta, clock, aid, node, crop_kind)
            .is_ok()
        {
            return;
        }
    }

    // 3. 在開門酒館泡溫水澡（限手頭寬裕 copper >= 30，絕不抽乾勞工餐費）或自家免費沐浴
    if !clock.is_night && fatigue >= 56 {
        let in_owned = delta
            .actors
            .get(aid)
            .is_some_and(|a| a.owned_house_node.as_deref() == Some(node_id.as_str()));
        if (in_owned || (node.kind == "shop" && node.name.contains("酒館") && copper >= 30))
            && super::living_sim_and_farm::apply_take_warm_bath(delta, clock, aid, node).is_ok()
        {
            return;
        }
    }

    // 4. 背景真實雙向社交交換記憶（限從容空閒且低頻與既有熟人街坊寒暄，絕不擠掉親族師友槽位！）
    if !clock.is_night
        && idle_at_node
        && fatigue < 60
        && delta
            .actors
            .get(aid)
            .is_some_and(|a| ((a.soul_seed ^ abs_bell.unsigned_abs()) % 24) == 0)
        && let Some(peer_id) = delta.node_occupants.get(&node_id).and_then(|occ| {
            let my_social = delta.actors.get(aid).map(|a| &a.mind.ltm.social);
            occ.iter()
                .find(|&oid| {
                    oid != aid
                        && my_social.is_some_and(|s| s.contains_key(oid.as_str()))
                        && delta.actors.get(oid).is_some_and(|o| {
                            o.alive && o.controller_id.is_none() && o.active_skirmish.is_none()
                        })
                })
                .cloned()
        })
    {
        let peer_name = delta
            .actors
            .get(&peer_id)
            .map_or_else(|| "街坊".to_string(), |o| o.display_name.clone());
        let short_place = node.name.split('・').next_back().unwrap_or(&node.name);
        let said_a = format!("最近{short_place}這邊的行情與路況如何？");
        let reply_b = format!("一切還算平順，大夥兒各自顧緊手頭活計便是。");
        if let Some(a) = delta.actors.get_mut(aid) {
            a.mind.record_talk_exchange(
                &peer_id,
                &peer_name,
                clock.game_days,
                &clock.bell_label,
                &reply_b,
                &said_a,
                false,
            );
            a.mark_mutated();
        }
        if let Some(b) = delta.actors.get_mut(&peer_id) {
            b.mind.record_talk_exchange(
                aid,
                &my_name,
                clock.game_days,
                &clock.bell_label,
                &said_a,
                &reply_b,
                false,
            );
            b.mark_mutated();
        }
    }
}
