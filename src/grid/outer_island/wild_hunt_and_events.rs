//! 第四刀（下）：野外巨獸新鮮蹤跡追獵鏈 × 多條城鎮動態活事件鏈（100% 錨定真實拓撲與世界狀態）。
//!
//! 嚴守原則：
//! 1. 巨獸蹤跡指引 100% 由 `shortest_node_path` 真實圖論拓撲算出下一步出口，絕不瞎編假方向。
//! 2. 城鎮活事件 100% 讀取真實潮汐、橋樑破損、鐵爐燃煤與野外未伏誅巨獸，絕非罐頭假劇情；每日每案防刷，銅幣嚴守複式守恆。
//! 3. 前台零原始數字外洩（距離以白話腳程呈現）。

use serde_json::{Value, json};

use super::clock_and_lang::SolarClockSnapshot;
use super::live_land::ensure_live_land_seeded;
use super::mind::EventPublicity;
use super::movement::{push_breath, shortest_node_path, take_clean_carry_items};
use super::persona::ValueAxis;
use super::sim_tick::broadcast_scene_event;
use super::types::{
    ApexBeastDef, IslandWorldDelta, SkirmishState, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc,
};

#[derive(Debug, Clone)]
pub struct ApexTrailHint {
    pub beast: ApexBeastDef,
    pub target_cell_id: u32,
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
    let Some(trail) = find_nearest_living_apex_trail(doc, delta, &node.id) else {
        return Ok(
            "你俯身細察泥土與林間斷枝，島上三大太古巨物皆已伏誅，荒野山海如今一片平靜。".to_string(),
        );
    };
    let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
    a.known_rumors
        .insert(format!("apex_trail:{}", trail.beast.slug));
    a.mind.ltm.mastery.craft_works = a.mind.ltm.mastery.craft_works.saturating_add(1);
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
            "你順著新鮮巨獸刮痕撥開岩影——{}【{}】驀然抬首低吼，沉重威壓封住周遭，當場進入臨陣對峙！",
            trail.beast.title_zh, trail.beast.name_zh
        );
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    a.fatigue = a.fatigue.saturating_add(4).min(95);
    a.mark_mutated();
    let msg = format!(
        "你俯身辨識草泥間的巨獸新鮮蹤跡：這是{}【{}】留下的壓痕（{}），蹤跡朝【{}方・{}】延伸而去——{}。",
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
pub struct ActiveTownEventSpec {
    pub kind: &'static str,
    pub title_zh: &'static str,
    pub cue_zh: String,
    pub desc_zh: String,
    pub action_label_zh: &'static str,
    pub action_sub_zh: &'static str,
}

fn event_done_key(kind: &str, game_days: i64) -> String {
    format!("town_evt:{kind}@d{game_days}")
}

pub fn build_active_town_events(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
) -> Vec<ActiveTownEventSpec> {
    let mut out: Vec<ActiveTownEventSpec> = Vec::new();

    // 事件鏈 A：海潮搶卸緊急船貨（港務棧橋）
    if node.kind == "dock"
        && !viewer
            .explored_windows
            .contains(&event_done_key("harbor_tide_salvage", clock.game_days))
    {
        out.push(ActiveTownEventSpec {
            kind: "harbor_tide_salvage",
            title_zh: "海潮急湧・棧橋搶護船貨",
            cue_zh: "浪頭拍上跳板，水手長正急呼人手協搬鹽漬木箱".to_string(),
            desc_zh: "遠洋貨船遇上湧浪急潮，甲板邊幾只封蠟貨箱眼看就要浸水，搭一把手穩住絞盤就能領到港務急工賞銀並結交碼頭街坊。".to_string(),
            action_label_zh: "上前搭手穩住絞盤搶護船貨",
            action_sub_zh: "協助港務保住商貨，領取急工賞銀並增進街坊情誼",
        });
    }

    // 事件鏈 B：斷橋／高水護路物資互助（廣場／市政／城門／橋頭）
    let damaged_bridge_cell = delta
        .live_land
        .cells
        .iter()
        .find(|(_, cl)| cl.has_river_crossing && cl.bridge_integrity < 95)
        .map(|(cid, _)| *cid);
    if matches!(node.kind.as_str(), "plaza" | "civic" | "gate" | "bridge")
        && (damaged_bridge_cell.is_some() || delta.live_land.water_stage_pct >= 55)
        && !viewer
            .explored_windows
            .contains(&event_done_key("bridge_relief_drive", clock.game_days))
    {
        let cue = if damaged_bridge_cell.is_some() {
            "官道木橋遭急水沖鬆，工務正徵集木料與壯丁加固橋墩".to_string()
        } else {
            "河口水位偏高，工務正忙著堆築沙袋護堤防潮".to_string()
        };
        out.push(ActiveTownEventSpec {
            kind: "bridge_relief_drive",
            title_zh: "工務急徵・護橋防潮協作",
            cue_zh: cue,
            desc_zh: "一旦官道橋樑受損，商隊貨車就會受阻、酒館乾糧與鐵料也跟著看漲；帶上原木直接加固或出力搬沙袋，都能實打實穩住城鎮物流並領公庫賞銀。".to_string(),
            action_label_zh: "加入工務護橋與防潮協作",
            action_sub_zh: "若有原木直接修繕橋基，或出力搬沙袋領取公庫賞銀",
        });
    }

    // 事件鏈 C：寒爐急送燃煤助燃（鐵匠街／鍛鐵鋪）
    if (node.name.contains("鐵匠") || node.name.contains("鍛鐵"))
        && !viewer
            .explored_windows
            .contains(&event_done_key("forge_ember_relay", clock.game_days))
    {
        out.push(ActiveTownEventSpec {
            kind: "forge_ember_relay",
            title_zh: "爐火救急・鐵匠街保溫助燃",
            cue_zh: "鐵砧連響不停，爐邊亟需清渣添炭穩住火候".to_string(),
            desc_zh: "百工訂單堆積如山，爐溫一旦掉落就得封爐停工；送入隨身煤炭或漂流木助燃，或搭手清渣整料，都能穩住全鎮鐵器供應並磨練鍛造手感。".to_string(),
            action_label_zh: "助燃添料或清渣穩住鍛爐火候",
            action_sub_zh: "投入煤炭／木料或出力清爐，領取賞銀與鍛造手感",
        });
    }

    // 事件鏈 D：巡防警訊與巨獸懸賞線索（城門／軍堡／廣場／酒館）
    if matches!(node.kind.as_str(), "gate" | "citadel" | "plaza" | "shop")
        && find_nearest_living_apex_trail(doc, delta, &node.id).is_some()
        && !viewer
            .explored_windows
            .contains(&event_done_key("watch_bounty_brief", clock.game_days))
    {
        out.push(ActiveTownEventSpec {
            kind: "watch_bounty_brief",
            title_zh: "巡防告示・荒野巨獸最新動向",
            cue_zh: "巡哨剛帶回山海異獸足跡急報，正招人抄錄巡查".to_string(),
            desc_zh: "外海大島能量豐饒，山海深處盤踞著太古巨物；幫巡防整理這班哨報即可領取微薄津貼，並獲得最準確的巨獸出沒去路指引。".to_string(),
            action_label_zh: "抄錄巡防急報並核對巨獸方位",
            action_sub_zh: "獲得真實巨獸追獵指引與巡防協力津貼",
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
    let key = event_done_key(kind, clock.game_days);
    if delta
        .actors
        .get(aid)
        .is_some_and(|a| a.explored_windows.contains(&key))
    {
        return Ok("這樁城鎮急務今日你已盡過心力，待明日新班潮水與消息再看吧。".to_string());
    }

    let (reward_copper, msg, witness_line) = match kind {
        "harbor_tide_salvage" => {
            if node.kind != "dock" {
                return Err("需站在港務棧橋邊才能搭手搶護靠岸船貨。".to_string());
            }
            let reward = 5i64;
            let paid = delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "town_event_harbor",
                "municipal_treasury",
                aid,
                reward,
                "海潮急湧搶護船貨急工賞銀".to_string(),
            );
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            a.fatigue = a.fatigue.saturating_add(9).min(95);
            a.explored_windows.insert(key);
            a.mind.stm.today_goal_progressed = true;
            a.mark_mutated();
            let pay_note = if paid {
                "港務記籌當場數了 5 枚銅幣急工賞銀給你"
            } else {
                "水手長記下了你的情分，連聲致謝"
            };
            let m = format!(
                "你頂著撲面海浪與水手合力絞緊纜繩、將三只封蠟鹽貨箱搶搬上高臺；{pay_note}，碼頭眾人對你刮目相看。"
            );
            push_breath(a, m.clone());
            (reward, m, "合力搶護險潮船貨保住了這班商貨".to_string())
        }
        "bridge_relief_drive" => {
            if !matches!(node.kind.as_str(), "plaza" | "civic" | "gate" | "bridge") {
                return Err("需在廣場、市政廳、城門或橋頭才能參與工務護橋協作。".to_string());
            }
            let donated_wood = {
                let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
                take_clean_carry_items(&mut a.backpack, "timber_log", 1)
                    .or_else(|| take_clean_carry_items(&mut a.backpack, "driftwood", 1))
                    .map(|it| it.name)
            };
            if let Some((_, cl)) = delta
                .live_land
                .cells
                .iter_mut()
                .find(|(_, cl)| cl.has_river_crossing && cl.bridge_integrity < 100)
            {
                let boost = if donated_wood.is_some() { 25 } else { 12 };
                cl.bridge_integrity = cl.bridge_integrity.saturating_add(boost).min(100);
                if cl.bridge_integrity >= 70 {
                    cl.bridge_down_days = 0;
                }
                delta.extras_dirty = true;
            }
            let reward = if donated_wood.is_some() { 6 } else { 4 };
            let paid = delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "town_event_bridge",
                "municipal_treasury",
                aid,
                reward,
                "工務護橋防潮協作津貼".to_string(),
            );
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            a.fatigue = a.fatigue.saturating_add(8).min(95);
            a.explored_windows.insert(key);
            a.mind.stm.today_goal_progressed = true;
            a.mark_mutated();
            let pay_note = if paid {
                format!("工務管事依例撥付 {reward} 枚銅幣津貼")
            } else {
                "工務管事鄭重記下你的功勞".to_string()
            };
            let m = if let Some(wname) = donated_wood {
                format!("你將隨身【{wname}】交予工務並一同打樁加固橋基，官道橋樑頓時穩固許多，{pay_note}。")
            } else {
                format!("你跟著街坊接力搬運沙袋、疏通橋墩浮木，穩住了這波漲水衝擊，{pay_note}。")
            };
            push_breath(a, m.clone());
            (reward, m, "參與工務護橋防潮協作穩住了官道通行".to_string())
        }
        "forge_ember_relay" => {
            if !node.name.contains("鐵匠") && !node.name.contains("鍛鐵") {
                return Err("需在鐵匠街或鍛鐵鋪旁才能協助保溫助燃。".to_string());
            }
            let fuel_given = {
                let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
                take_clean_carry_items(&mut a.backpack, "black_coal", 1)
                    .or_else(|| take_clean_carry_items(&mut a.backpack, "charcoal_sack", 1))
                    .or_else(|| take_clean_carry_items(&mut a.backpack, "driftwood", 1))
                    .map(|it| it.name)
            };
            let reward = if fuel_given.is_some() { 6 } else { 4 };
            let paid = delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "town_event_forge",
                "municipal_treasury",
                aid,
                reward,
                "鐵匠街爐火保溫助燃賞銀".to_string(),
            );
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            a.fatigue = a.fatigue.saturating_add(7).min(95);
            a.bound_weapon.grip_familiarity = a.bound_weapon.grip_familiarity.saturating_add(3);
            a.mind.ltm.mastery.craft_works = a.mind.ltm.mastery.craft_works.saturating_add(1);
            a.explored_windows.insert(key);
            a.mind.stm.today_goal_progressed = true;
            a.mark_mutated();
            let pay_note = if paid {
                format!("領得 {reward} 枚銅幣工資")
            } else {
                "博得鐵匠師傅連聲稱讚".to_string()
            };
            let m = if let Some(fname) = fuel_given {
                format!("你把【{fname}】投進吃緊的鍛爐並幫著鼓風穩火，爐膛重新泛起亮橘火光，{pay_note}，掌中錘感也更熟稔了幾分。")
            } else {
                format!("你幫鐵匠街師傅清出爐底熔渣、碼齊鐵胚，保住了這班鍛造火候，{pay_note}，掌中錘感更熟稔了幾分。")
            };
            push_breath(a, m.clone());
            (reward, m, "協助鐵匠街穩住鍛爐火候".to_string())
        }
        "watch_bounty_brief" => {
            if !matches!(node.kind.as_str(), "gate" | "citadel" | "plaza" | "shop") {
                return Err("需在城門、軍堡、廣場或酒館方能查閱巡防急報。".to_string());
            }
            let trail = find_nearest_living_apex_trail(doc, delta, &node.id);
            let reward = 3i64;
            let paid = delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "town_event_watch",
                "municipal_treasury",
                aid,
                reward,
                "協助抄錄巡防巨獸急報津貼".to_string(),
            );
            let a = delta.actors.get_mut(aid).ok_or("找不到旅人行蹤")?;
            a.explored_windows.insert(key);
            a.mind.stm.today_goal_progressed = true;
            if let Some(ref tr) = trail {
                a.known_rumors
                    .insert(format!("apex_trail:{}", tr.beast.slug));
            }
            a.mark_mutated();
            let pay_note = if paid {
                "巡哨隊副塞了 3 枚銅幣辛苦費給你"
            } else {
                "巡哨隊副拍了拍你肩膀致謝"
            };
            let m = if let Some(tr) = trail {
                format!(
                    "你幫巡哨整理完本班山海異動哨報，{pay_note}；哨報指明{}【{}】近期確在【{}】一帶出沒，從此處出發宜先往【{}方・{}】一路循跡追索。",
                    tr.beast.title_zh,
                    tr.beast.name_zh,
                    tr.target_node_name,
                    tr.next_hop_dir,
                    tr.next_hop_name
                )
            } else {
                format!("你幫巡哨核對完四方來信，{pay_note}；近日海島三大巨物皆已伏誅，四郊尚稱平靖。")
            };
            push_breath(a, m.clone());
            (reward, m, "協助巡防抄錄山海巨獸動向哨報".to_string())
        }
        _ => return Err("眼前並無這項城鎮急務。".to_string()),
    };

    let _ = reward_copper;
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
        55,
        35,
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
    let trail = find_nearest_living_apex_trail(doc, delta, &viewer.node_id);
    let known_any = viewer
        .known_rumors
        .iter()
        .any(|r| r.starts_with("apex_trail:"));
    if let Some(tr) = trail
        && known_any
    {
        json!({
            "active": true,
            "beast_name": tr.beast.name_zh,
            "beast_title": tr.beast.title_zh,
            "next_hint": if tr.at_lair {
                "就在眼前巢穴深處，隨時可循跡正面迎擊".to_string()
            } else {
                format!("朝【{}方・{}】延伸（{}）", tr.next_hop_dir, tr.next_hop_name, tr.distance_sense_zh)
            },
        })
    } else {
        json!({ "active": false })
    }
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
    // 1. 野外巨獸新鮮蹤跡追獵插座（僅於非室內非城鎮或巢穴節點浮現）
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
                format!("循跡正面驚動【{}】", tr.beast.name_zh),
                "直面太古巨物並以全自動田忌賽馬交鋒",
            )
        } else {
            (
                "新鮮巨獸足印與樹皮刮痕".to_string(),
                format!("泥地留著沉重獸印，隱約指向【{}方】", tr.next_hop_dir),
                "俯身辨識巨獸新鮮蹤跡".to_string(),
                "推斷太古巨物去向並記錄追獵方位",
            )
        };
        objects.push(json!({
            "object_id": "obj:apex_beast_trail",
            "name": name,
            "cue": cue,
            "desc": format!("{}。細查泥痕與斷枝折向，便能順著真實去路一路追獵至其盤踞之地。", tr.beast.lair_hint_zh),
            "direct_action": Value::Null,
            "options": [{
                "id": "track_wild_beast",
                "param": "",
                "label": lbl,
                "sub": sub,
            }],
        }));
        push_audit(
            "track_wild_beast",
            "",
            &lbl,
            sub,
            "object",
            "obj:apex_beast_trail",
        );
    }

    // 2. 城鎮動態活事件鏈插座（每節點依實況浮現 1~2 樁正在發生的急務）
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
