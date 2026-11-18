//! 輕量版 RimWorld 平面大地圖 HTTP 端點與背景 250ms 推進器（`/api/rim/{state,cmd}`）。

use std::time::Duration;

use axum::extract::Query;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::game::now_unix;
use crate::grid::substrate::render_synth::zone_name_of;
use crate::grid::substrate::rim_life::{
    eat_item, hunger_tag, magnesium_tag, stamina_tag, thirst_tag, toggle_wearable,
};
use crate::grid::substrate::rim_sim::{ActorFaction, RimWorldState, global_rim};
use crate::grid::substrate::rim_world::{
    BlueprintJob, MicroGround, NaturalProp, RimItemKind, StructureKind, derive_micro_pristine,
    micro_to_macro,
};
use crate::grid::substrate::runtime;

use super::rim_auth::{build_sky_time_payload, resolve_viewer_pid};

const MAX_MICRO_SPAN: i32 = 84;

#[derive(Deserialize)]
pub struct RimStateQuery {
    pub min_mx: Option<i32>,
    pub max_mx: Option<i32>,
    pub min_my: Option<i32>,
    pub max_my: Option<i32>,
    pub sel_x: Option<i32>,
    pub sel_y: Option<i32>,
}

#[derive(Deserialize)]
pub struct RimCmdBody {
    pub cmd: String,
    pub mx: Option<i32>,
    pub my: Option<i32>,
    pub x0: Option<i32>,
    pub y0: Option<i32>,
    pub x1: Option<i32>,
    pub y1: Option<i32>,
    pub tool: Option<String>,
    pub action: Option<String>,
    pub target_actor: Option<usize>,
    pub run: Option<bool>,
    pub name: Option<String>,
    pub glyph: Option<String>,
    pub item_code: Option<String>,
    pub directive: Option<String>,
}

/// 啟動常駐 50ms（20Hz）輕量版 RimWorld 連續平滑物理與工單推進迴圈。
pub fn spawn_rim_tick_loop() {
    let _ = global_rim();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(50));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Ok(mut world) = global_rim().lock() {
                world.step_physics_50ms();
            }
        }
    });
}

fn ground_code(g: MicroGround) -> &'static str {
    match g {
        MicroGround::LoamGrass => "grass",
        MicroGround::RedGravel => "red",
        MicroGround::PebbleBank => "pebble",
        MicroGround::ShallowStream => "stream",
        MicroGround::SpringPool => "spring",
        MicroGround::DeepWater => "deep",
        MicroGround::SteepCliff => "cliff",
    }
}

fn prop_code(p: NaturalProp) -> &'static str {
    match p {
        NaturalProp::BambooClump => "bamboo",
        NaturalProp::BambooShoot => "shoot",
        NaturalProp::RiverStone => "stone",
        NaturalProp::WildHerb => "herb",
    }
}

fn struct_code(s: StructureKind) -> &'static str {
    match s {
        StructureKind::BambooWall => "wall",
        StructureKind::BambooDoor => "door",
        StructureKind::WoodBridge => "bridge",
        StructureKind::Campfire => "campfire",
        StructureKind::Workbench => "workbench",
        StructureKind::Stockpile => "stockpile",
        StructureKind::BambooBed => "bed",
        StructureKind::TilledRidge => "ridge",
        StructureKind::HerbSeedling => "herb_seed",
        StructureKind::HerbMature => "herb_plot",
        StructureKind::CropSeedling => "crop_seed",
        StructureKind::CropMature => "crop_plot",
    }
}

pub(super) fn item_code_str(k: RimItemKind) -> &'static str {
    match k {
        RimItemKind::BambooPole => "bamboo",
        RimItemKind::RiverStone => "stone",
        RimItemKind::BambooShoot => "shoot",
        RimItemKind::BeastMeat => "meat",
        RimItemKind::RoastedShoot => "roast_shoot",
        RimItemKind::RoastedMeat => "roast_meat",
        RimItemKind::BambooBlade => "blade",
        RimItemKind::StonePick => "pick",
        RimItemKind::BambooHat => "hat",
        RimItemKind::BambooPack => "pack",
        RimItemKind::MountainHerb => "herb",
        RimItemKind::WoundSalve => "salve",
        RimItemKind::SweetTuber => "tuber",
        RimItemKind::RoastedTuber => "roast_tuber",
    }
}

pub(super) fn parse_item_code(s: &str) -> Option<RimItemKind> {
    Some(match s {
        "bamboo" => RimItemKind::BambooPole,
        "stone" => RimItemKind::RiverStone,
        "shoot" => RimItemKind::BambooShoot,
        "meat" => RimItemKind::BeastMeat,
        "roast_shoot" => RimItemKind::RoastedShoot,
        "roast_meat" => RimItemKind::RoastedMeat,
        "blade" => RimItemKind::BambooBlade,
        "pick" => RimItemKind::StonePick,
        "hat" => RimItemKind::BambooHat,
        "pack" => RimItemKind::BambooPack,
        "herb" => RimItemKind::MountainHerb,
        "salve" => RimItemKind::WoundSalve,
        "tuber" => RimItemKind::SweetTuber,
        "roast_tuber" => RimItemKind::RoastedTuber,
        _ => return None,
    })
}

pub(super) fn zone_for_micro(
    rt: Option<&runtime::SubstrateRuntime>,
    mx: i32,
    my: i32,
) -> &'static str {
    if mx >= 22 && my <= -2 {
        return "水尾寮溪坪";
    }
    let ((gx, gy), _) = micro_to_macro(mx, my);
    rt.and_then(|r| r.l0(gx, gy).map(|l0| zone_name_of(gx, l0)))
        .unwrap_or("西麓湧泉竹帶")
}

fn build_selection_card(
    world: &RimWorldState,
    rt: Option<&runtime::SubstrateRuntime>,
    viewer_pid: Option<&str>,
    mx: i32,
    my: i32,
) -> Value {
    let ((gx, gy), _) = micro_to_macro(mx, my);
    let zone = zone_for_micro(rt, mx, my);

    // 優先檢查是否有異獸或他人站在該格附近
    if let Some(actor) = world.actors.iter().find(|a| {
        a.entity_id.as_deref() != viewer_pid
            && a.hp > 0
            && (a.px - mx as f32).hypot(a.py - my as f32) <= 1.05
    }) {
        let vis_name = actor.display_name_for(viewer_pid);
        let mut tags = vec![actor.role_tag.clone(), actor.health_state_tag()];
        if let Some(t) = actor.equipped_tool {
            tags.push(format!("持{}", t.label()));
        }
        tags.push(actor.activity_tag.clone());
        let npc_skills: Vec<_> = if actor.faction == ActorFaction::Settler {
            actor.natural_skills().into_iter().take(2).collect()
        } else {
            Vec::new()
        };
        let mut sockets = Vec::new();
        if actor.faction == ActorFaction::WildBeast {
            sockets.push(json!({
                "label": "上前拆招交鋒",
                "action": "attack_actor",
                "target_actor": actor.id
            }));
        } else {
            sockets.push(json!({
                "label": "搭話問訊",
                "cmd": "talk",
                "target_actor": actor.id
            }));
            sockets.push(json!({
                "label": "託付：跟隨同行",
                "cmd": "directive",
                "target_actor": actor.id,
                "directive": "follow"
            }));
            sockets.push(json!({
                "label": "託付：協作營造",
                "cmd": "directive",
                "target_actor": actor.id,
                "directive": "assist"
            }));
            sockets.push(json!({
                "label": "託付：自由巡走",
                "cmd": "directive",
                "target_actor": actor.id,
                "directive": "patrol"
            }));
        }
        let actor_profile = (actor.faction == ActorFaction::Settler).then(|| {
            let hp_ratio = (actor.hp as f32 / actor.max_hp.max(1) as f32).clamp(0.0, 1.0);
            json!({
                "id": actor.id,
                "name": vis_name,
                "glyph": actor.glyph,
                "gender": actor.gender,
                "soul_seed": actor.soul_seed.to_string(),
                "role": actor.role_tag,
                "tool": actor.equipped_tool.map(RimItemKind::label),
                "carry": actor.carrying.map(|c| format!("{}×{}", c.item.short_char(), c.count)),
                "wear_hat": actor.equipped_hat,
                "show_pack": actor.equipped_pack || actor.carrying.is_some(),
                "hp_ratio": hp_ratio,
                "skills": npc_skills,
            })
        });
        return json!({
            "mx": mx, "my": my, "gx": gx, "gy": gy, "zone": zone,
            "title": vis_name,
            "tags": tags,
            "sockets": sockets,
            "actor_profile": actor_profile,
        });
    }

    let (ground, nat) = derive_micro_pristine(rt, mx, my);
    let ov = world.overlays.get(&(mx, my));
    let cleared = ov.is_some_and(|o| o.cleared_natural || o.structure.is_some());

    let mut title = ground.label().to_string();
    let mut tags: Vec<String> = Vec::new();
    let mut sockets: Vec<Value> = Vec::new();

    if let Some(bp) = ov.and_then(|o| o.blueprint) {
        match bp {
            BlueprintJob::Chop { .. } => {
                title = format!("待採伐・{}", nat.map_or("林地", NaturalProp::label));
                tags.push("待採伐".into());
                sockets.push(json!({ "label": "親手砍伐", "action": "chop" }));
                sockets.push(json!({ "label": "取消採伐", "tool": "deconstruct" }));
            }
            BlueprintJob::Build {
                structure,
                delivered_bamboo,
                ..
            } => {
                title = format!("營造中・{}", structure.label());
                let need = structure.bamboo_cost();
                tags.push(if delivered_bamboo >= need {
                    "材料已齊・施工中".into()
                } else {
                    "待同伴送竹材".into()
                });
                sockets.push(json!({ "label": "親手築造", "action": "build" }));
                sockets.push(json!({ "label": "取消營造", "tool": "deconstruct" }));
            }
            BlueprintJob::Deconstruct { .. } => {
                title = "待拆除".into();
                tags.push("待拆除".into());
            }
        }
    } else if let Some(st) = ov.and_then(|o| o.structure) {
        let weather = crate::grid::substrate::rim_shelter::weather_at_unix(
            crate::grid::substrate::rim_sim::now_unix_sec(),
        );
        let sheltered = crate::grid::substrate::rim_shelter::sheltered_cell_set(&world.overlays);
        title = st.label().to_string();
        match st {
            StructureKind::Workbench => {
                tags.push("百工造物".into());
                sockets.push(json!({ "label": "削製烤刃竹刀", "action": "craft_blade" }));
                sockets.push(json!({ "label": "綁製石刃木鎬", "action": "craft_pick" }));
                sockets.push(json!({ "label": "搗製敷傷藥膏", "action": "craft_salve" }));
                sockets.push(json!({ "label": "劈篾編竹笠", "action": "craft_hat" }));
                sockets.push(json!({ "label": "編紮竹背簍", "action": "craft_pack" }));
            }
            StructureKind::Campfire => {
                if weather.is_raining() && !sheltered.contains(&(mx, my)) {
                    tags.push("受雨轉弱・冒煙".into());
                } else {
                    tags.push("溫暖照明".into());
                }
                tags.push("駐足回氣".into());
                sockets.push(json!({ "label": "籜裹煨筍", "action": "cook_shoot" }));
                sockets.push(json!({ "label": "炭火煨甘薯", "action": "cook_tuber" }));
                sockets.push(json!({ "label": "炭火炙肉", "action": "cook_meat" }));
                sockets.push(json!({ "label": "圍爐盤坐歇息", "action": "rest" }));
                sockets.push(json!({ "label": "走近烤火", "action": "move" }));
            }
            StructureKind::Stockpile => {
                let viewer_idx = viewer_pid.and_then(|vp| {
                    world.actors.iter().position(|a| a.entity_id.as_deref() == Some(vp))
                });
                title = if crate::grid::substrate::rim_economy::stockpile_is_indoor(mx, my) {
                    "倉廩木架儲位".into()
                } else {
                    "竹木石料場".into()
                };
                crate::grid::substrate::rim_economy::describe_stockpile_card(
                    world, viewer_idx, mx, my, ov.and_then(|o| o.ground_item), &mut tags, &mut sockets,
                );
            }
            StructureKind::WoodBridge => {
                tags.push("跨澗通行".into());
                sockets.push(json!({ "label": "踏橋而過", "action": "move" }));
            }
            StructureKind::BambooWall => tags.push("阻隔野獸與視線".into()),
            StructureKind::BambooDoor => {
                tags.push("同伴可通・擋野獸".into());
                sockets.push(json!({ "label": "穿門而過", "action": "move" }));
            }
            StructureKind::BambooBed => {
                tags.push("遮風安歇".into());
                tags.push("調養傷疲".into());
                if sheltered.contains(&(mx, my)) {
                    tags.push("屋簷庇護".into());
                }
                sockets.push(json!({ "label": "上榻安歇養傷", "action": "sleep_bed" }));
            }
            other if other.is_garden_plot() => {
                crate::grid::substrate::rim_ecology::describe_ecology_structure(
                    other,
                    weather.is_raining(),
                    &mut tags,
                    &mut sockets,
                );
            }
            _ => {}
        }
        sockets.push(json!({ "label": "標記拆除", "tool": "deconstruct" }));
    } else if !cleared && let Some(prop) = nat {
        title = prop.label().to_string();
        for &t in prop.tags() {
            tags.push(t.into());
        }
        sockets.push(json!({ "label": "親手採伐", "action": "chop" }));
        sockets.push(json!({ "label": "交同伴採伐", "tool": "chop" }));
    } else if matches!(ground, MicroGround::ShallowStream | MicroGround::SpringPool) {
        tags.push("清泉甘洌".into());
        sockets.push(json!({ "label": "掬泉飲水", "action": "drink" }));
        if ground == MicroGround::ShallowStream {
            sockets.push(json!({ "label": "在此架設渡溪窄橋", "tool": "bridge" }));
        }
    } else if !ground.is_water() && ground != MicroGround::SteepCliff {
        tags.push("平坦地面".into());
        if let Some(g) = ov.and_then(|o| o.ground_item) {
            tags.push(format!("散落{}×{}", g.item.label(), g.count));
            sockets.push(
                json!({ "label": format!("拾入行囊（{}）", g.item.label()), "action": "pickup" }),
            );
        }
        sockets.push(json!({ "label": "行至此處", "action": "move" }));
        if matches!(ground, MicroGround::LoamGrass | MicroGround::RedGravel) {
            sockets.push(json!({ "label": "持鎬翻土開壟", "action": "till_ridge" }));
        }
        sockets.push(json!({ "label": "在此生起營火", "tool": "campfire" }));
        sockets.push(json!({ "label": "在此立竹木牆", "tool": "wall" }));
        sockets.push(json!({ "label": "在此安竹編門", "tool": "door" }));
        sockets.push(json!({ "label": "在此設工作台", "tool": "workbench" }));
        sockets.push(json!({ "label": "在此鋪儲物墊", "tool": "stockpile" }));
        sockets.push(json!({ "label": "在此設竹編睡榻", "tool": "bed" }));
    } else {
        tags.push("天險阻絕".into());
    }

    json!({
        "mx": mx, "my": my, "gx": gx, "gy": gy, "zone": zone,
        "title": title,
        "tags": tags,
        "sockets": sockets
    })
}

pub async fn rim_state(headers: HeaderMap, Query(q): Query<RimStateQuery>) -> Response {
    let rt = runtime::global();
    let viewer_pid = resolve_viewer_pid(&headers);
    let Ok(mut world) = global_rim().lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if let Some(ref pid) = viewer_pid {
        world.touch_player_activity(pid);
    }

    let mut cells = Vec::new();
    if let (Some(x0), Some(x1), Some(y0), Some(y1)) = (q.min_mx, q.max_mx, q.min_my, q.max_my) {
        let cx = (x0 + x1) / 2;
        let cy = (y0 + y1) / 2;
        let min_x = x0.max(cx - MAX_MICRO_SPAN / 2);
        let max_x = x1.min(cx + MAX_MICRO_SPAN / 2);
        let min_y = y0.max(cy - MAX_MICRO_SPAN / 2);
        let max_y = y1.min(cy + MAX_MICRO_SPAN / 2);

        for my in min_y..=max_y {
            for mx in min_x..=max_x {
                let (ground, nat) = derive_micro_pristine(rt, mx, my);
                let ov = world.overlays.get(&(mx, my));
                let cleared = ov.is_some_and(|o| o.cleared_natural || o.structure.is_some());
                let prop = if cleared { None } else { nat.map(prop_code) };
                let st = ov.and_then(|o| o.structure).map(struct_code);
                let bp = ov.and_then(|o| o.blueprint).map(|b| match b {
                    BlueprintJob::Chop { progress } => json!({ "k": "chop", "p": progress }),
                    BlueprintJob::Build {
                        structure,
                        delivered_bamboo,
                        progress,
                    } => json!({
                        "k": struct_code(structure),
                        "mat": delivered_bamboo,
                        "need": structure.bamboo_cost(),
                        "p": progress
                    }),
                    BlueprintJob::Deconstruct { progress } => {
                        json!({ "k": "decon", "p": progress })
                    }
                });
                let item = ov.and_then(|o| o.ground_item).map(
                    |g| json!({ "c": g.item.short_char(), "n": g.count, "label": g.item.label() }),
                );

                // 僅回傳非純草地或帶有物件之細格以大幅壓縮 JSON 體積，純草地由前端底色鋪滿
                if ground != MicroGround::LoamGrass
                    || prop.is_some()
                    || st.is_some()
                    || bp.is_some()
                    || item.is_some()
                {
                    cells.push(json!({
                        "x": mx,
                        "y": my,
                        "g": ground_code(ground),
                        "p": prop,
                        "s": st,
                        "b": bp,
                        "i": item
                    }));
                }
            }
        }
    }

    let now = now_unix();
    let actors: Vec<Value> = world
        .actors
        .iter()
        .filter(|a| {
            let is_me = viewer_pid
                .as_deref()
                .is_some_and(|v| a.entity_id.as_deref() == Some(v));
            if a.entity_id.is_none() || is_me || a.autopilot {
                return true;
            }
            now.saturating_sub(a.last_active_unix) <= 120
        })
        .map(|a| {
            let is_player = viewer_pid
                .as_deref()
                .is_some_and(|v| a.entity_id.as_deref() == Some(v));
            let base_speed = if a.downed_ticks > 0 {
                0.0_f32
            } else if a.faction == ActorFaction::WildBeast {
                1.60_f32
            } else if a.running {
                3.80_f32
            } else {
                1.35_f32
            };
            let speed = if a.injury_ticks > 0 {
                base_speed * 0.86
            } else {
                base_speed
            };
            let limit = if is_player { 64 } else { 8 };
            let path: Vec<(i32, i32)> = a.path.iter().take(limit).copied().collect();
            json!({
                "id": a.id,
                "glyph": a.glyph,
                "name": a.display_name_for(viewer_pid.as_deref()),
                "gender": a.gender,
                "soul_seed": a.soul_seed.to_string(),
                "role": a.role_tag,
                "faction": a.faction,
                "is_player": is_player,
                "autopilot": a.autopilot,
                "running": a.running,
                "px": a.px,
                "py": a.py,
                "speed": speed,
                "facing": a.facing_rad,
                "hp_ratio": (a.hp as f32 / a.max_hp.max(1) as f32).clamp(0.0, 1.0),
                "health_tag": a.health_state_tag(),
                "tool": a.equipped_tool.map(RimItemKind::label),
                "wear_hat": a.equipped_hat,
                "show_pack": a.equipped_pack || a.carrying.is_some(),
                "carry": a.carrying.map(|c| format!("{}×{}", c.item.short_char(), c.count)),
                "act": a.activity_tag,
                "pop": a.pop_text.as_ref().map(|(t, _)| t.as_str()),
                "path": path,
            })
        })
        .collect();

    let weather = crate::grid::substrate::rim_shelter::weather_at_unix(
        crate::grid::substrate::rim_sim::now_unix_sec(),
    );
    let enclosed_rooms = crate::grid::substrate::rim_shelter::detect_enclosed_rooms(&world.overlays);
    let mut sheltered_cells = std::collections::HashSet::new();
    for room in &enclosed_rooms {
        for &c in &room.cells {
            sheltered_cells.insert(c);
        }
        for &d in &room.doors {
            sheltered_cells.insert(d);
        }
    }

    let me_detail = viewer_pid
        .as_deref()
        .and_then(|pid| {
            world
                .actors
                .iter()
                .find(|a| a.entity_id.as_deref() == Some(pid))
        })
        .map(|p| {
            let (pmx, pmy) = p.micro_pos();
            let player_zone = zone_for_micro(rt, pmx, pmy);
            let inv: Vec<Value> = p
                .inventory
                .iter()
                .map(|s| {
                    json!({
                        "code": item_code_str(s.item),
                        "label": s.item.label(),
                        "count": s.count,
                        "is_tool": s.item.is_hand_tool(),
                        "is_edible": s.item.is_edible(),
                        "is_medical": s.item.is_medical(),
                        "is_wearable": s.item.is_wearable(),
                    })
                })
                .collect();
            let hp_ratio = (p.hp as f32 / p.max_hp.max(1) as f32).clamp(0.0, 1.0);
            let on_bed = world
                .overlays
                .get(&(pmx, pmy))
                .and_then(|o| o.structure)
                == Some(StructureKind::BambooBed);
            let mut feels = vec![
                hunger_tag(p.hunger),
                thirst_tag(p.thirst),
                stamina_tag(p.stamina),
            ];
            if on_bed && p.path.is_empty() {
                feels.push("榻上安眠");
            } else if sheltered_cells.contains(&(pmx, pmy)) {
                feels.push("屋簷庇護");
            } else if p.wet_ticks >= 16 {
                feels.push("衣衫濕冷");
            } else if weather.is_raining() && p.equipped_hat {
                feels.push("竹笠擋雨");
            }
            json!({
                "id": p.id,
                "name": p.name,
                "glyph": p.glyph,
                "gender": p.gender,
                "soul_seed": p.soul_seed.to_string(),
                "wear_hat": p.equipped_hat,
                "show_pack": p.equipped_pack || p.carrying.is_some(),
                "equipped_hat": p.equipped_hat,
                "equipped_pack": p.equipped_pack,
                "hp_ratio": hp_ratio,
                "zone": player_zone,
                "health": p.health_state_tag(),
                "feels": feels,
                "magnesium_tag": magnesium_tag(p.magnesium),
                "act": p.activity_tag,
                "autopilot": p.autopilot,
                "directive": crate::grid::substrate::rim_cmd::normalize_directive_code(&p.directive),
                "equipped_code": p.equipped_tool.map(item_code_str),
                "equipped_label": p.equipped_tool.map_or("空手", RimItemKind::label),
                "skills": p.natural_skills(),
                "recent_diary": p.recent_diary.iter().take(6).cloned().collect::<Vec<_>>(),
                "injury_part": p.injury_part,
                "inventory": inv,
            })
        });

    let selected = q
        .sel_x
        .zip(q.sel_y)
        .map(|(sx, sy)| build_selection_card(&world, rt, viewer_pid.as_deref(), sx, sy));

    let events: Vec<&str> = world
        .events
        .iter()
        .take(4)
        .map(|e| e.text.as_str())
        .collect();
    let (world_clock, sky_time) = build_sky_time_payload();

    Json(json!({
        "tick": world.tick,
        "world_clock": world_clock,
        "sky_time": sky_time,
        "weather": {
            "code": weather.code(),
            "label": weather.label(),
            "raining": weather.is_raining(),
        },
        "rooms": enclosed_rooms,
        "me_detail": me_detail,
        "actors": actors,
        "cells": cells,
        "events": events,
        "selected": selected,
    }))
    .into_response()
}

pub async fn rim_cmd(headers: HeaderMap, Json(body): Json<RimCmdBody>) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "ok": false, "msg": "請先登入角色" })),
        )
            .into_response();
    };
    let Ok(mut world) = global_rim().lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    match body.cmd.as_str() {
        "act" => {
            let (Some(mx), Some(my)) = (body.mx, body.my) else {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "ok": false, "msg": "請先點選位置" })),
                )
                    .into_response();
            };
            let is_run = body.run.unwrap_or(false);
            match world.command_player(
                &pid,
                mx,
                my,
                body.action.as_deref(),
                body.target_actor,
                is_run,
            ) {
                Ok(msg) => {
                    let (path, speed, running, px, py) = world
                        .actors
                        .iter()
                        .find(|a| a.entity_id.as_deref() == Some(pid.as_str()))
                        .map(|p| {
                            let base_s = if p.running { 3.80_f32 } else { 1.35_f32 };
                            let s = if p.injury_ticks > 0 {
                                base_s * 0.86
                            } else {
                                base_s
                            };
                            (
                                p.path.iter().copied().collect::<Vec<_>>(),
                                s,
                                p.running,
                                p.px,
                                p.py,
                            )
                        })
                        .unwrap_or_default();
                    Json(json!({
                        "ok": true,
                        "msg": msg,
                        "path": path,
                        "speed": speed,
                        "running": running,
                        "px": px,
                        "py": py
                    }))
                    .into_response()
                }
                Err(msg) => Json(json!({ "ok": false, "msg": msg })).into_response(),
            }
        }
        "designate" => {
            let (Some(x0), Some(y0), Some(x1), Some(y1), Some(tool)) =
                (body.x0, body.y0, body.x1, body.y1, body.tool.as_deref())
            else {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "ok": false, "msg": "請先點選範圍" })),
                )
                    .into_response();
            };
            world.touch_player_activity(&pid);
            let n = world.designate_rect(x0, y0, x1, y1, tool);
            if n > 0 {
                world.persist_if_needed(true);
            }
            Json(json!({ "ok": true, "count": n })).into_response()
        }
        "toggle_autopilot" => {
            let mut now_ap = false;
            let mut manual_aid = None;
            if let Some(p) = world.actors.iter_mut().find(|a| a.entity_id.as_deref() == Some(pid.as_str())) {
                p.autopilot = !p.autopilot;
                if p.autopilot {
                    p.activity_tag = "自主行事".into();
                } else {
                    p.task = crate::grid::substrate::rim_sim::PendingTask::None;
                    p.path.clear();
                    p.activity_tag = "駐足待命".into();
                    manual_aid = Some(p.id);
                }
                now_ap = p.autopilot;
            }
            if let Some(aid) = manual_aid {
                world.stall_tracker.remove(&aid);
                world.economy.parcel_queues.remove(&aid);
            }
            world.touch_player_activity(&pid);
            world.persist_if_needed(true);
            Json(json!({ "ok": true, "autopilot": now_ap })).into_response()
        }
        "equip" => {
            if let Some(p) = world
                .actors
                .iter_mut()
                .find(|a| a.entity_id.as_deref() == Some(pid.as_str()))
            {
                let code = body.item_code.as_deref().unwrap_or("");
                if code == "none" || code.is_empty() {
                    if let Some(prev) = p.equipped_tool.take() {
                        p.add_inv(prev, 1);
                    }
                } else if let Some(want) = parse_item_code(code)
                    && want.is_hand_tool()
                    && p.equipped_tool != Some(want)
                    && p.take_inv(want, 1) > 0
                {
                    if let Some(prev) = p.equipped_tool.replace(want) {
                        p.add_inv(prev, 1);
                    }
                }
            }
            world.touch_player_activity(&pid);
            world.persist_if_needed(true);
            Json(json!({ "ok": true })).into_response()
        }
        "eat" => {
            let Some(item) = body.item_code.as_deref().and_then(parse_item_code) else {
                return Json(json!({ "ok": false, "msg": "無法辨識此食物" })).into_response();
            };
            let tick = world.tick;
            let Some(p) = world
                .actors
                .iter_mut()
                .find(|a| a.entity_id.as_deref() == Some(pid.as_str()))
            else {
                return Json(json!({ "ok": false, "msg": "尚未進入世界" })).into_response();
            };
            match eat_item(p, item) {
                Ok(msg) => {
                    p.pop_text = Some((format!("吃罷{}", item.label()), tick + 10));
                    world.touch_player_activity(&pid);
                    world.persist_if_needed(true);
                    Json(json!({ "ok": true, "msg": msg })).into_response()
                }
                Err(err) => Json(json!({ "ok": false, "msg": err })).into_response(),
            }
        }
        "apply_salve" => {
            let tick = world.tick;
            let Some(p) = world
                .actors
                .iter_mut()
                .find(|a| a.entity_id.as_deref() == Some(pid.as_str()))
            else {
                return Json(json!({ "ok": false, "msg": "尚未進入世界" })).into_response();
            };
            match crate::grid::substrate::rim_ecology::apply_wound_salve(p) {
                Ok(msg) => {
                    p.pop_text = Some(("敷妥藥膏".into(), tick + 10));
                    world.touch_player_activity(&pid);
                    world.persist_if_needed(true);
                    Json(json!({ "ok": true, "msg": msg })).into_response()
                }
                Err(err) => Json(json!({ "ok": false, "msg": err })).into_response(),
            }
        }
        "toggle_wear" => {
            let Some(item) = body.item_code.as_deref().and_then(parse_item_code) else {
                return Json(json!({ "ok": false, "msg": "無法辨識穿戴物品" })).into_response();
            };
            let Some(p) = world
                .actors
                .iter_mut()
                .find(|a| a.entity_id.as_deref() == Some(pid.as_str()))
            else {
                return Json(json!({ "ok": false, "msg": "尚未進入世界" })).into_response();
            };
            let wear = match item {
                RimItemKind::BambooHat => !p.equipped_hat,
                RimItemKind::BambooPack => !p.equipped_pack,
                _ => false,
            };
            match toggle_wearable(p, item, wear) {
                Ok(msg) => {
                    world.touch_player_activity(&pid);
                    world.persist_if_needed(true);
                    Json(json!({ "ok": true, "msg": msg })).into_response()
                }
                Err(err) => Json(json!({ "ok": false, "msg": err })).into_response(),
            }
        }
        "drop" => {
            if let Some(code) = body.item_code.as_deref().and_then(parse_item_code)
                && let Some(p_idx) = world
                    .actors
                    .iter()
                    .position(|a| a.entity_id.as_deref() == Some(pid.as_str()))
            {
                let pos = world.actors[p_idx].micro_pos();
                let taken = world.actors[p_idx].take_inv(code, 1);
                if taken > 0 {
                    world.add_ground_item(pos.0, pos.1, code, taken);
                }
            }
            world.touch_player_activity(&pid);
            world.persist_if_needed(true);
            Json(json!({ "ok": true })).into_response()
        }
        "directive" => {
            let dir = body.directive.unwrap_or_default();
            if let Some(tid) = body.target_actor {
                let tick = world.tick;
                if let Some(a) = world
                    .actors
                    .iter_mut()
                    .find(|a| a.id == tid && a.entity_id.is_none())
                {
                    a.directive = dir.clone();
                    a.task = crate::grid::substrate::rim_sim::PendingTask::None;
                    a.path.clear();
                    let label = match dir.as_str() {
                        "follow" => "領命跟隨同行",
                        "assist" => "領命協作營造",
                        _ => "轉為自由巡走",
                    };
                    a.pop_text = Some((label.into(), tick + 10));
                }
            } else if let Some(p) = world
                .actors
                .iter_mut()
                .find(|a| a.entity_id.as_deref() == Some(pid.as_str()))
            {
                let norm = crate::grid::substrate::rim_cmd::normalize_directive_code(&dir);
                p.directive = norm.into();
                let label = match norm {
                    "craft" => "百工營造",
                    "guard" => "護衛獵巡",
                    "rest" => "爐畔靜養",
                    _ => "林緣巡採",
                };
                p.push_diary(format!("將駐世行事方針改為「{label}」"));
            }
            world.touch_player_activity(&pid);
            world.persist_if_needed(true);
            Json(json!({ "ok": true, "msg": "已記下駐世方針" })).into_response()
        }
        "talk" => {
            let tick = world.tick;
            if let Some(tid) = body.target_actor
                && let Some(a) = world.actors.iter_mut().find(|a| a.id == tid)
            {
                let reply = match a.directive.as_str() {
                    "follow" => "「走哪我都跟上。」".into(),
                    "assist" | "craft" => "「有活儘管標出來。」".into(),
                    _ => a.recent_diary.front().map_or_else(
                        || "「北澗已架好窄木橋，過溪仍當心野豬。」".to_string(),
                        |d| format!("「剛才才忙著{d}。」"),
                    ),
                };
                a.pop_text = Some((reply, tick + 14));
            }
            world.touch_player_activity(&pid);
            Json(json!({ "ok": true })).into_response()
        }
        "sync_identity" => {
            if let Some(p) = world
                .actors
                .iter_mut()
                .find(|a| a.entity_id.as_deref() == Some(pid.as_str()))
            {
                if let Some(name) = body.name.filter(|s| !s.trim().is_empty()) {
                    p.name = name.trim().chars().take(16).collect();
                }
                if let Some(g) = body.glyph.filter(|s| !s.trim().is_empty()) {
                    p.glyph = g.trim().chars().take(1).collect();
                }
            }
            world.touch_player_activity(&pid);
            Json(json!({ "ok": true })).into_response()
        }
        "reset_camp" => {
            world.reset_camp_for_player(&pid);
            world.persist_if_needed(true);
            Json(json!({ "ok": true, "msg": "已回到營火旁歇息" })).into_response()
        }
        "vault_buy" | "vault_sell" => {
            let (Some(mx), Some(my), Some(code)) = (body.mx, body.my, body.item_code.as_deref()) else {
                return (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "msg": "缺少倉位或品項" }))).into_response();
            };
            let Some(p_idx) = world.actors.iter().position(|a| a.entity_id.as_deref() == Some(pid.as_str())) else {
                return StatusCode::NOT_FOUND.into_response();
            };
            let (ok, msg) = crate::grid::substrate::rim_economy::handle_vault_cmd(&mut world, p_idx, body.cmd == "vault_buy", mx, my, code);
            world.touch_player_activity(&pid);
            if ok { world.push_event(msg.clone()); world.persist_if_needed(true); }
            Json(json!({ "ok": ok, "msg": msg })).into_response()
        }
        _ => (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "msg": "無法執行此動作" }))).into_response(),
    }
}
