//! 個人之相與自然成相（`human_phase`）橋接層：
//! 1. 依每位角色的先天骨相種子、持物載體與實地歷練，推導獨一無二的個人武相與百工藝相；
//! 2. 將人獸交鋒與解牛解剖接入三相動力學判準（一力降十會／專克拆招／庖丁解牛）；
//! 3. 嚴守去數值化台灣白話轉譯與完整前後因果（所學 × 所熟 × 所歷）。

use serde::{Deserialize, Serialize};

use super::rim_sim::{ActorFaction, PendingTask, RimActor, RimWorldState};
use super::rim_world::{
    NaturalProp, RimItemKind, StructureKind, derive_micro_pristine, find_micro_path,
    find_micro_path_crowded,
};
use super::runtime::SubstrateRuntime;
use crate::combat::human_phase::{
    BeastAnatomy, InnateBonePhase, ThreePhaseVerdict, derive_personal_craft_routine,
    derive_personal_martial_routine, evaluate_seam_threading, evaluate_three_phase_bout_ext,
};
use crate::combat::wordplate::{CarrierAffinity, PostureState, TacticalSlot, WordRegistry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillChipDto {
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct ExchangeOutcome {
    pub loot_meat: u16,
    pub kill_event: Option<String>,
    pub phase_note: String,
}

/// 由 `soul_seed` 或角色姓名推導確定性非零先天骨相種子。
pub fn effective_seed(actor: &RimActor) -> i64 {
    if actor.soul_seed != 0 {
        return actor.soul_seed;
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ (actor.id as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    for b in actor.name.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    let signed = (h & 0x7fff_ffff_ffff_ffff) as i64;
    if signed == 0 { 108_421 } else { signed }
}

fn tool_to_carrier(tool: Option<RimItemKind>) -> (CarrierAffinity, &'static str, u32) {
    match tool {
        Some(RimItemKind::BambooBlade) => (CarrierAffinity::Blade, "烤刃竹刀", 1),
        Some(RimItemKind::StonePick) => (CarrierAffinity::Blunt, "石刃木鎬", 2),
        _ => (CarrierAffinity::Unarmed, "徒手", 0),
    }
}

fn actor_practice_for(actor: &RimActor, carrier: CarrierAffinity) -> u32 {
    match carrier {
        CarrierAffinity::Blade => actor.practice_blade,
        CarrierAffinity::Blunt => actor.practice_blunt,
        _ => actor.practice_unarmed,
    }
}

fn role_craft_affinity(role_tag: &str) -> (&'static str, &'static str) {
    if role_tag.contains("木匠") || role_tag.contains("匠") || role_tag.contains("百工") {
        ("forge", "harvest")
    } else if role_tag.contains("貨郎") || role_tag.contains("商") {
        ("wayfare", "harvest")
    } else if role_tag.contains("茶") || role_tag.contains("廚") {
        ("culinary", "harvest")
    } else {
        ("harvest", "wayfare")
    }
}

fn domain_count_for(actor: &RimActor, code: &str) -> u32 {
    match code {
        "harvest" => actor.craft_harvest,
        "forge" => actor.craft_forge,
        "culinary" => actor.craft_culinary,
        "wayfare" => actor.craft_wayfare,
        _ => 0,
    }
}

fn mastery_label(count: u32) -> &'static str {
    if count >= 24 {
        "信手拈來"
    } else if count >= 8 {
        "熟能生巧"
    } else {
        "初具法度"
    }
}

/// 推導角色當下兩項臨陣武相與兩項百工藝相（全去數值化，供 `[自身]` 與接觸卡呈現）。
pub fn synthesize_actor_skill_chips(actor: &RimActor) -> Vec<SkillChipDto> {
    let bone = InnateBonePhase::from_seed(effective_seed(actor));
    let registry = WordRegistry::global();
    let (carrier, vessel_name, _) = tool_to_carrier(actor.equipped_tool);
    let m_practice = actor_practice_for(actor, carrier);
    let learned_bonus = (m_practice as f64 * 0.04).min(0.95);
    let total_anatomy = actor.anatomy_hoofed.saturating_add(actor.anatomy_winged);

    let open_routine = derive_personal_martial_routine(
        &bone,
        TacticalSlot::Opening,
        carrier,
        vessel_name,
        learned_bonus,
        m_practice,
        registry,
    );
    let follow_slot = if actor.directive == "rest" || actor.injury_ticks > 0 {
        TacticalSlot::Defense
    } else {
        TacticalSlot::Midgame
    };
    let second_routine = derive_personal_martial_routine(
        &bone,
        follow_slot,
        carrier,
        vessel_name,
        learned_bonus,
        m_practice,
        registry,
    );

    let spec_short = open_routine
        .phase_specialty_zh
        .split('（')
        .next()
        .unwrap_or(open_routine.phase_specialty_zh);
    let mut chips = vec![
        SkillChipDto {
            label: format!("武相・{}（{}）", open_routine.formal_name, spec_short),
            detail: format!(
                "{} {}（境界：{}）",
                open_routine.origin_basis_zh,
                open_routine.learned_and_familiar_zh,
                mastery_label(m_practice)
            ),
        },
        SkillChipDto {
            label: format!("{}・{}", second_routine.slot_label_zh, second_routine.formal_name),
            detail: format!(
                "{} {}",
                second_routine.origin_basis_zh, second_routine.auto_trigger_desc_zh
            ),
        },
    ];

    let (pref_a, pref_b) = role_craft_affinity(&actor.role_tag);
    let mut domains = ["harvest", "forge", "culinary", "wayfare"];
    domains.sort_by(|a, b| {
        let score = |d: &str| {
            let cnt = domain_count_for(actor, d);
            let role_bias = if d == pref_a {
                2
            } else if d == pref_b {
                1
            } else {
                0
            };
            (cnt, role_bias)
        };
        score(b).cmp(&score(a))
    });

    for craft_code in domains.into_iter().take(2) {
        let cnt = domain_count_for(actor, craft_code);
        let craft = derive_personal_craft_routine(
            &bone,
            craft_code,
            carrier,
            vessel_name,
            cnt,
            total_anatomy,
            registry,
        );
        chips.push(SkillChipDto {
            label: format!("{}・{}", craft.slot_label_zh, craft.formal_name),
            detail: format!(
                "{} {}（火候：{}）",
                craft.origin_basis_zh,
                craft.auto_trigger_desc_zh,
                mastery_label(cnt)
            ),
        });
    }

    chips
}

fn beast_weight_and_carrier(beast: &RimActor) -> (f64, CarrierAffinity, &'static str) {
    match BeastAnatomy::from_species_title(&beast.name) {
        BeastAnatomy::HoofedHorned => (85.0, CarrierAffinity::Blunt, "蠻衝頂撞"),
        BeastAnatomy::Winged => (6.5, CarrierAffinity::Footwork, "振羽疾撲"),
        _ => (42.0, CarrierAffinity::Unarmed, "獸爪撲襲"),
    }
}

/// 依 `human_phase` 三相判準結算人類與野生山獸交鋒及戰後解牛剔骨。
pub fn resolve_actor_vs_beast_exchange(
    settler: &mut RimActor,
    beast: &mut RimActor,
    tick: u64,
) -> Option<ExchangeOutcome> {
    if settler.faction != ActorFaction::Settler
        || beast.faction != ActorFaction::WildBeast
        || settler.hp <= 0
        || settler.downed_ticks > 0
        || beast.hp <= 0
    {
        return None;
    }

    let bone_s = InnateBonePhase::from_seed(effective_seed(settler));
    let bone_b = InnateBonePhase::from_seed(effective_seed(beast));
    let (carrier_s, vessel_s, kind_tag) = tool_to_carrier(settler.equipped_tool);
    let (weight_b, carrier_b, beast_move_zh) = beast_weight_and_carrier(beast);

    let anatomy_fam = if beast.name.contains('雞') || beast.name.contains('雉') {
        settler.anatomy_winged
    } else {
        settler.anatomy_hoofed
    };
    let s_practice = actor_practice_for(settler, carrier_s);
    let bout_idx = (tick % 4) as usize;
    let scar_s = if settler.injury_ticks > 0 { 0.45 } else { 0.0 };
    let posture_s = if (settler.hp * 100) / settler.max_hp.max(1) < 45 {
        PostureState::Shaken
    } else {
        PostureState::Steady
    };
    let posture_b = if (beast.hp * 100) / beast.max_hp.max(1) < 45 {
        PostureState::Shaken
    } else {
        PostureState::Steady
    };

    let res = evaluate_three_phase_bout_ext(
        TacticalSlot::Opening,
        &bone_s,
        &bone_b,
        62.0,
        weight_b,
        carrier_s,
        carrier_b,
        posture_s,
        posture_b,
        5,
        4,
        bout_idx,
        s_practice,
        2,
        anatomy_fam,
        0,
        (s_practice as f64 * 0.04).min(1.0),
        0.1,
        scar_s,
        0.0,
    );

    let mut loot_meat: u16 = 0;
    let mut kill_event: Option<String> = None;
    let mut phase_note = String::new();

    if settler.cooldown_ticks == 0 {
        let base_dmg: f64 = match settler.equipped_tool {
            Some(RimItemKind::BambooBlade) => 22.0,
            Some(RimItemKind::StonePick) => 21.0,
            _ => 12.0,
        };
        let phase_bonus = match res.verdict {
            ThreePhaseVerdict::Phase1ImpulseCrush {
                winner_is_a: true,
                lambda_ratio,
            } => (lambda_ratio * 4.5).clamp(4.0, 12.0),
            ThreePhaseVerdict::Phase2KinematicIntercept {
                interceptor_is_a: true,
                ..
            } => 7.0,
            ThreePhaseVerdict::Phase3SeamThreading {
                threader_is_a: true,
                xi_index,
                ..
            } => (xi_index * 5.0).clamp(5.0, 11.0),
            _ => res.score_delta_a.clamp(0.0, 5.0),
        };
        let dmg = (base_dmg * (0.88 + 0.12 * bone_s.alpha.clamp(0.6, 1.6)) + phase_bonus)
            .round()
            .clamp(10.0, 38.0) as i32;

        beast.hp -= dmg;
        settler.cooldown_ticks = 3;
        settler.activity_tag = "拆招交鋒".into();

        match kind_tag {
            1 => settler.practice_blade = settler.practice_blade.saturating_add(1),
            2 => settler.practice_blunt = settler.practice_blunt.saturating_add(1),
            _ => settler.practice_unarmed = settler.practice_unarmed.saturating_add(1),
        }

        let routine = derive_personal_martial_routine(
            &bone_s,
            TacticalSlot::Opening,
            carrier_s,
            vessel_s,
            (s_practice as f64 * 0.04).min(0.95),
            s_practice,
            WordRegistry::global(),
        );
        let verdict_tag = match res.verdict {
            ThreePhaseVerdict::Phase1ImpulseCrush { winner_is_a: true, .. } => {
                format!("{}・重勢硬撼", routine.formal_name)
            }
            ThreePhaseVerdict::Phase2KinematicIntercept {
                interceptor_is_a: true,
                ..
            } => format!("{}・截隙拆招", routine.formal_name),
            ThreePhaseVerdict::Phase3SeamThreading {
                threader_is_a: true,
                ..
            } => format!("{}・游刃穿隙", routine.formal_name),
            _ => routine.formal_name,
        };
        settler.pop_text = Some((verdict_tag.clone(), tick + 6));
        phase_note = verdict_tag;

        if beast.hp <= 0 {
            let seam = evaluate_seam_threading(
                &bone_s,
                carrier_s,
                bone_s.dex_0,
                if beast.name.contains('豬') { 145.0 } else { 115.0 },
                anatomy_fam,
            );
            let bonus_cut = carrier_s == CarrierAffinity::Blade && seam.xi_index >= 0.96;
            loot_meat = if bonus_cut { 3 } else { 2 };

            if beast.name.contains('雞') || beast.name.contains('雉') {
                settler.anatomy_winged = settler.anatomy_winged.saturating_add(2);
            } else {
                settler.anatomy_hoofed = settler.anatomy_hoofed.saturating_add(2);
            }
            settler.craft_harvest = settler.craft_harvest.saturating_add(1);

            let diary_line = if bonus_cut {
                format!("以「{phase_note}」放倒{}並順骨縫多剔出鮮肉", beast.name)
            } else {
                format!("在野徑遇上{}，使出「{phase_note}」將之擊退", beast.name)
            };
            settler.push_diary(diary_line);
            kill_event = Some(if bonus_cut {
                format!("{} 游刃解下{}厚肉", settler.glyph, beast.name)
            } else {
                format!("擊退{}・遺落骨肉", beast.name)
            });
            return Some(ExchangeOutcome {
                loot_meat,
                kill_event,
                phase_note,
            });
        }
    }

    if beast.hp > 0 && beast.cooldown_ticks == 0 {
        let dodged = matches!(
            res.verdict,
            ThreePhaseVerdict::Phase2KinematicIntercept {
                interceptor_is_a: true,
                ..
            }
        ) || (settler.equipped_tool == Some(RimItemKind::BambooBlade)
            && tick.is_multiple_of(2));
        if dodged {
            beast.pop_text = Some(("撲勢被卸開".into(), tick + 5));
        } else {
            let hit_dmg = if beast.name.contains('豬') { 11 } else { 7 };
            settler.hp -= hit_dmg;
            beast.pop_text = Some((beast_move_zh.into(), tick + 5));
        }
        beast.cooldown_ticks = 4;
    }

    Some(ExchangeOutcome {
        loot_meat,
        kill_event,
        phase_note,
    })
}

fn find_first_path_near(
    world: &RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    start: (i32, i32),
    anchor: (i32, i32),
) -> Option<Vec<(i32, i32)>> {
    let occ = world.occupied_cells(Some(idx));
    for (ox, oy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
        let g = (anchor.0 + ox, anchor.1 + oy);
        if let Some(p) =
            find_micro_path_crowded(rt, &world.overlays, start, g, false, false, &occ)
        {
            return Some(p);
        }
    }
    None
}

/// 雙聚落行腳貨郎依 `caravan_leg` 往返「水尾寮・溪畔茶寮 `(32,-7)` ↔ 西麓湧泉前哨 `(1,-1)`」，嚴格物質守恆互通有無。
pub fn try_assign_caravan_route(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
) -> bool {
    let role = world.actors[idx].role_tag.as_str();
    let dir = world.actors[idx].directive.as_str();
    if (!role.contains("貨郎") && !role.contains("商"))
        || dir == "follow"
        || dir == "rest"
        || world.actors[idx].downed_ticks > 0
    {
        return false;
    }
    let start = world.actors[idx].micro_pos();
    let leg = world.actors[idx].caravan_leg;
    let (target_anchor, next_leg, act_label) = if leg == 0 {
        ((1, -1), 0_u8, "挑貨往西麓")
    } else {
        ((32, -7), 1_u8, "返程水尾寮")
    };

    let dist = (start.0 - target_anchor.0).abs() + (start.1 - target_anchor.1).abs();
    if dist <= 4 {
        complete_caravan_arrival(world, rt, idx, next_leg);
        return world.actors[idx].task != PendingTask::None;
    }

    if let Some(p) = find_first_path_near(world, rt, idx, start, target_anchor) {
        world.actors[idx].path = p.into();
        world.actors[idx].task = PendingTask::CaravanMove {
            tx: target_anchor.0,
            ty: target_anchor.1,
            leg: next_leg,
        };
        world.actors[idx].activity_tag = act_label.into();
        return true;
    }
    false
}

/// 貨郎抵達目標聚落時卸貨、裝載餘缺物資並切換下一趟行商航段。
pub fn complete_caravan_arrival(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    leg: u8,
) {
    let start = world.actors[idx].micro_pos();
    let west_anchor = (1, -1);
    let east_anchor = (32, -7);

    if leg == 0 {
        for dish in [RimItemKind::RoastedShoot, RimItemKind::BambooShoot] {
            if world.actors[idx].take_inv(dish, 1) > 0 {
                if let Some((kx, ky)) = world.find_available_stockpile_slot((-3, -1), dish) {
                    world.add_ground_item(kx, ky, dish, 1);
                } else {
                    world.add_ground_item(start.0, start.1, dish, 1);
                }
            }
        }
        if let Some((bx, by)) =
            world.find_nearest_ground_item((-3, -1), RimItemKind::BambooPole, false)
            && (bx + 3).abs() + (by + 1).abs() <= 6
            && world
                .overlays
                .get(&(bx, by))
                .and_then(|o| o.ground_item)
                .is_some_and(|g| g.count >= 4)
            && world.take_ground_item(bx, by, RimItemKind::BambooPole, 1) > 0
        {
            world.actors[idx].add_inv(RimItemKind::BambooPole, 1);
        }
        world.actors[idx].craft_wayfare = world.actors[idx].craft_wayfare.saturating_add(2);
        world.actors[idx].caravan_leg = 1;
        world.actors[idx].push_diary("挑水尾寮山筍抵西麓湧泉互通有無");
        world.actors[idx].pop_text = Some(("抵西麓換貨".into(), world.tick + 10));
        world.push_event(format!("{} 挑貨抵西麓湧泉換妥竹材", world.actors[idx].glyph));

        if let Some(p) = find_first_path_near(world, rt, idx, start, east_anchor) {
            world.actors[idx].path = p.into();
            world.actors[idx].task = PendingTask::CaravanMove {
                tx: east_anchor.0,
                ty: east_anchor.1,
                leg: 1,
            };
            world.actors[idx].activity_tag = "返程水尾寮".into();
        } else {
            world.actors[idx].task = PendingTask::None;
        }
    } else {
        if world.actors[idx].take_inv(RimItemKind::BambooPole, 1) > 0 {
            if let Some((kx, ky)) =
                world.find_available_stockpile_slot((34, -8), RimItemKind::BambooPole)
            {
                world.add_ground_item(kx, ky, RimItemKind::BambooPole, 1);
            } else {
                world.add_ground_item(start.0, start.1, RimItemKind::BambooPole, 1);
            }
        }
        for dish in [RimItemKind::RoastedShoot, RimItemKind::BambooShoot] {
            if let Some((sx, sy)) = world.find_nearest_ground_item((34, -8), dish, false)
                && (sx - 34).abs() + (sy + 8).abs() <= 6
                && world
                    .overlays
                    .get(&(sx, sy))
                    .and_then(|o| o.ground_item)
                    .is_some_and(|g| g.count >= 2)
                && world.take_ground_item(sx, sy, dish, 1) > 0
            {
                world.actors[idx].add_inv(dish, 1);
                break;
            }
        }
        world.actors[idx].craft_wayfare = world.actors[idx].craft_wayfare.saturating_add(2);
        world.actors[idx].caravan_leg = 0;
        world.actors[idx].push_diary("返抵水尾寮茶寮清點竹材與山筍");
        world.actors[idx].pop_text = Some(("挑貨赴西麓".into(), world.tick + 10));

        if let Some(p) = find_first_path_near(world, rt, idx, start, west_anchor) {
            world.actors[idx].path = p.into();
            world.actors[idx].task = PendingTask::CaravanMove {
                tx: west_anchor.0,
                ty: west_anchor.1,
                leg: 0,
            };
            world.actors[idx].activity_tag = "挑貨往西麓".into();
        } else {
            world.actors[idx].task = PendingTask::None;
        }
    }
}

/// 依角色現行駐世方針（`craft`／`forage`）於無待建藍圖時執行自主造具或林緣採筍。
pub fn try_proactive_directive_work(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    start: (i32, i32),
    home: (i32, i32),
    claimed: &[(i32, i32)],
    occ: &[(i32, i32)],
) -> bool {
    if world.actors[idx].directive == "craft"
        && world.actors[idx].equipped_tool.is_none()
        && let Some((&(wx, wy), _)) = world
            .overlays
            .iter()
            .filter(|(_, ov)| ov.structure == Some(StructureKind::Workbench))
            .min_by_key(|&(&(cx, cy), _)| (cx - start.0).abs() + (cy - start.1).abs())
        && (wx - start.0).abs() + (wy - start.1).abs() <= 24
        && let Some(p) =
            find_micro_path_crowded(rt, &world.overlays, start, (wx, wy), true, false, occ)
    {
        world.actors[idx].path = p.into();
        world.actors[idx].task = PendingTask::CraftAt {
            tx: wx,
            ty: wy,
            item: RimItemKind::BambooBlade,
        };
        world.actors[idx].activity_tag = "赴台製刀".into();
        return true;
    }

    if matches!(world.actors[idx].directive.as_str(), "forage" | "自主巡採")
        && world.tick.is_multiple_of(5)
    {
        let bag_stock: u16 = world.actors[idx]
            .inventory
            .iter()
            .filter(|s| matches!(s.item, RimItemKind::BambooShoot | RimItemKind::BambooPole))
            .map(|s| s.count)
            .sum();
        if bag_stock < 8
            && let Some(pos) = (-18..=18)
                .flat_map(|dy| (-18..=18).map(move |dx| (home.0 + dx, home.1 + dy)))
                .filter(|pos| !claimed.contains(pos))
                .find(|&(cx, cy)| {
                    let cleared = world
                        .overlays
                        .get(&(cx, cy))
                        .is_some_and(|o| o.cleared_natural || o.structure.is_some());
                    !cleared
                        && matches!(
                            derive_micro_pristine(rt, cx, cy).1,
                            Some(NaturalProp::BambooShoot)
                        )
                })
            && let Some(p) = find_micro_path(rt, &world.overlays, start, pos, true, false)
        {
            world.actors[idx].path = p.into();
            world.actors[idx].task = PendingTask::ChopAt {
                tx: pos.0,
                ty: pos.1,
            };
            world.actors[idx].activity_tag = "林緣巡採".into();
            return true;
        }
    }
    false
}

/// 結算場上所有人獸近身碰撞交鋒並落地戰利品與見聞事件。
pub fn resolve_world_combat_round(world: &mut RimWorldState) {
    let len = world.actors.len();
    let tick = world.tick;
    for i in 0..len {
        for j in (i + 1)..len {
            if world.actors[i].hp <= 0 || world.actors[j].hp <= 0 {
                continue;
            }
            if world.actors[i].faction == world.actors[j].faction {
                continue;
            }
            let dist = (world.actors[j].px - world.actors[i].px)
                .hypot(world.actors[j].py - world.actors[i].py);
            if dist > 1.45 {
                continue;
            }
            let (settler_idx, beast_idx) = if world.actors[i].faction == ActorFaction::Settler {
                (i, j)
            } else {
                (j, i)
            };
            if world.actors[settler_idx].downed_ticks > 0 {
                continue;
            }
            let outcome = {
                let (s_mut, b_mut) = if settler_idx < beast_idx {
                    let (left, right) = world.actors.split_at_mut(beast_idx);
                    (&mut left[settler_idx], &mut right[0])
                } else {
                    let (left, right) = world.actors.split_at_mut(settler_idx);
                    (&mut right[0], &mut left[beast_idx])
                };
                resolve_actor_vs_beast_exchange(s_mut, b_mut, tick)
            };
            if let Some(out) = outcome
                && out.loot_meat > 0
            {
                let bag_meat: u16 = world.actors[settler_idx]
                    .inventory
                    .iter()
                    .filter(|s| s.item == RimItemKind::BeastMeat)
                    .map(|s| s.count)
                    .sum();
                let to_bag = out.loot_meat.min(4_u16.saturating_sub(bag_meat));
                if to_bag > 0 {
                    world.actors[settler_idx].add_inv(RimItemKind::BeastMeat, to_bag);
                }
                let rem = out.loot_meat - to_bag;
                if rem > 0 {
                    let (bx, by) = world.actors[beast_idx].micro_pos();
                    world.add_ground_item(bx, by, RimItemKind::BeastMeat, rem);
                }
                if let Some(ev) = out.kill_event {
                    world.push_event(ev);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::rim_world::{GroundStack, MicroCellOverlay, StructureKind};
    use std::collections::{HashMap, VecDeque};

    fn sample_actor(id: usize, name: &str, seed: i64, role: &str) -> RimActor {
        RimActor {
            id,
            code: format!("s_{id}"),
            actor_key: format!("s_{id}"),
            entity_id: None,
            glyph: name.chars().next().unwrap_or('人').to_string(),
            name: name.into(),
            gender: "M".into(),
            soul_seed: seed,
            role_tag: role.into(),
            faction: ActorFaction::Settler,
            is_player: false,
            autopilot: true,
            running: false,
            px: 0.0,
            py: 0.0,
            facing_rad: 0.0,
            hp: 100,
            max_hp: 100,
            equipped_tool: Some(RimItemKind::BambooBlade),
            carrying: None,
            inventory: Vec::new(),
            directive: "forage".into(),
            path: VecDeque::new(),
            task: PendingTask::None,
            activity_tag: String::new(),
            pop_text: None,
            cooldown_ticks: 0,
            last_active_unix: 0,
            hunger: 90,
            thirst: 90,
            stamina: 90,
            magnesium: 20,
            equipped_hat: true,
            equipped_pack: false,
            known_by: Vec::new(),
            practice_unarmed: 0,
            practice_blade: 0,
            practice_blunt: 0,
            craft_harvest: 0,
            craft_forge: 0,
            craft_culinary: 0,
            craft_wayfare: 0,
            anatomy_winged: 0,
            anatomy_hoofed: 0,
            injury_part: None,
            injury_ticks: 0,
            downed_ticks: 0,
            caravan_leg: 0,
            wet_ticks: 0,
            recent_diary: VecDeque::new(),
        }
    }

    #[test]
    fn distinct_soul_seeds_emerge_distinct_formal_skills() {
        let a = sample_actor(1, "林阿木", 108_421, "百工木匠");
        let b = sample_actor(2, "陳青", 294_715, "巡山獵戶");
        let c = sample_actor(3, "周阿茶", 642_817, "茶寮庖廚");

        let skills_a = synthesize_actor_skill_chips(&a);
        let skills_b = synthesize_actor_skill_chips(&b);
        let skills_c = synthesize_actor_skill_chips(&c);

        assert_eq!(skills_a.len(), 4);
        assert_eq!(skills_b.len(), 4);
        assert_eq!(skills_c.len(), 4);
        let names = [
            skills_a[0].label.as_str(),
            skills_b[0].label.as_str(),
            skills_c[0].label.as_str(),
        ];
        let uniq: std::collections::HashSet<&str> = names.into_iter().collect();
        assert!(uniq.len() >= 2, "expected varied emergent martial names across seeds: {skills_a:?} / {skills_b:?} / {skills_c:?}");
        assert_ne!(skills_a[2].label, skills_c[2].label);
        for chip in skills_a.iter().chain(skills_b.iter()) {
            assert!(!chip.detail.contains('%'));
        }
    }

    #[test]
    fn experienced_hunter_threads_boar_seams_for_extra_meat() {
        let mut hunter = sample_actor(1, "陳青", 294_715, "巡山獵戶");
        hunter.anatomy_hoofed = 24;
        hunter.practice_blade = 16;

        let mut boar = sample_actor(2, "紅土野山豬", 70_001, "山海野獸");
        boar.faction = ActorFaction::WildBeast;
        boar.equipped_tool = None;
        boar.hp = 12;
        boar.max_hp = 80;

        let out = resolve_actor_vs_beast_exchange(&mut hunter, &mut boar, 12).expect("exchange");
        assert!(boar.hp <= 0);
        assert_eq!(out.loot_meat, 3);
        assert!(!hunter.recent_diary.is_empty());
        assert!(hunter.practice_blade >= 1);
    }

    #[test]
    fn downed_settler_survives_and_ally_rescues_back_to_fire() {
        let mut overlays = HashMap::new();
        for y in -4..=4 {
            for x in -4..=4 {
                overlays.insert(
                    (x, y),
                    MicroCellOverlay {
                        cleared_natural: true,
                        structure: (x == 0 && y == 0).then_some(StructureKind::Campfire),
                        ..Default::default()
                    },
                );
            }
        }
        let mut rescuer = sample_actor(2, "林阿木", 108_421, "百工木匠");
        rescuer.hp = 95;
        rescuer.px = -3.0;

        let mut victim = sample_actor(3, "陳青", 294_715, "巡山獵戶");
        victim.hp = 1;
        victim.downed_ticks = 18;
        victim.injury_ticks = 90;
        victim.injury_part = Some("右臂筋絡震傷".into());
        victim.px = 2.0;

        let mut world = RimWorldState {
            tick: 1,
            overlays,
            actors: vec![rescuer, victim],
            events: VecDeque::new(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        };
        for _ in 0..140 {
            world.step_physics_50ms();
            if world.actors.len() == 2 && world.actors[1].downed_ticks == 0 && world.actors[1].hp >= 30 {
                break;
            }
        }
        assert_eq!(world.actors.len(), 2, "settler must never be deleted on defeat");
        assert_eq!(world.actors[1].downed_ticks, 0);
        assert!(world.actors[1].hp >= 30, "victim hp={}", world.actors[1].hp);
        assert!(!world.actors[0].recent_diary.is_empty());
        assert!(!world.actors[1].recent_diary.is_empty());
    }

    #[test]
    fn caravan_barter_between_outposts_strictly_conserves_items() {
        let mut overlays = HashMap::new();
        for y in -11..=3 {
            for x in -5..=36 {
                let st = if (x, y) == (-3, -1) || (x, y) == (-4, -1) || (x, y) == (34, -8) {
                    Some(StructureKind::Stockpile)
                } else {
                    None
                };
                let gi = if (x, y) == (-3, -1) {
                    Some(GroundStack { item: RimItemKind::BambooPole, count: 5 })
                } else if (x, y) == (34, -8) {
                    Some(GroundStack { item: RimItemKind::RoastedShoot, count: 3 })
                } else {
                    None
                };
                overlays.insert(
                    (x, y),
                    MicroCellOverlay {
                        cleared_natural: true,
                        structure: st,
                        blueprint: None,
                        ground_item: gi,
                        eco_ticks: 0,
                    },
                );
            }
        }
        let mut merchant = sample_actor(10, "鄭石頭", 781_903, "行腳貨郎");
        merchant.caravan_leg = 1;
        merchant.px = 32.0;
        merchant.py = -7.0;

        let mut world = RimWorldState {
            tick: 1,
            overlays,
            actors: vec![merchant],
            events: VecDeque::new(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        };

        let count_all = |w: &RimWorldState, kind: RimItemKind| -> u32 {
            let g: u32 = w
                .overlays
                .values()
                .filter_map(|o| o.ground_item)
                .filter(|s| s.item == kind)
                .map(|s| u32::from(s.count))
                .sum();
            let inv: u32 = w.actors[0]
                .inventory
                .iter()
                .filter(|s| s.item == kind)
                .map(|s| u32::from(s.count))
                .sum();
            g + inv
        };

        assert_eq!(count_all(&world, RimItemKind::RoastedShoot), 3);
        assert_eq!(count_all(&world, RimItemKind::BambooPole), 5);
        complete_caravan_arrival(&mut world, None, 0, 1);
        assert_eq!(count_all(&world, RimItemKind::RoastedShoot), 3);
        assert_eq!(count_all(&world, RimItemKind::BambooPole), 5);
        assert_eq!(world.actors[0].caravan_leg, 0);

        world.actors[0].px = 1.0;
        world.actors[0].py = -1.0;
        complete_caravan_arrival(&mut world, None, 0, 0);
        assert_eq!(count_all(&world, RimItemKind::RoastedShoot), 3);
        assert_eq!(count_all(&world, RimItemKind::BambooPole), 5);
        assert_eq!(world.actors[0].caravan_leg, 1);
        assert!(world.actors[0].craft_wayfare >= 4);
    }
}
