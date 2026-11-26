//! 銀鷗島 `human_phase`「碎招 ➔ 套路 ➔ 成相（物理私有 × 街頭名聲須在場目擊）」與「唯一落款活器物鍛造／換裝流轉」模組。
//!
//! 嚴格落實審辯室 Room `7eee7478-1493-4faf-b7e3-12de460402e3` 定案：
//! 1. 拒絕刷次數自動掛名：必須在「跨對手／跨配方新情境」真正奏效，碎招才連貫為個人成相（私有物理實力即時生效）；
//! 2. 物理與名聲雙層分離：唯有同節點有其他活人親眼目睹（`DirectWitness`），成相與斬獸事蹟才寫入公認街頭名號與兵刃傳世銘文；無目擊時自稱僅列 `ClaimedBySpeaker`；
//! 3. 活器物湧現與流轉：匠人（玩家或居民）消耗真實採集／收購之礦木甲殼原石，鍛出帶唯一 `uid`、匠師落款與歷練履歷之實體新器物，供買賣與換裝流轉。

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::combat::human_phase::{
    InnateBonePhase, derive_personal_craft_routine, derive_personal_martial_routine,
    evaluate_three_phase_bout_ext,
};
use crate::combat::wordplate::{CarrierAffinity, PostureState, TacticalSlot, WordRegistry};

use super::clock_and_lang::{SolarClockSnapshot, solar_year_and_season_of_day};
use super::harbor_lineage::CURRENT_SOLAR_YEAR;
use super::mind::EventPublicity;
use super::movement::{add_carry_item, push_breath};
use super::persona::ValueAxis;
use super::sim_tick::broadcast_scene_event;
use super::types::{BoundTool, CarryItem, IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

/// 個人程序記憶與成相雙層狀態（私有真本事 vs 有目擊公認名號）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProceduralMastery {
    pub martial_bouts: u16,
    pub distinct_foes: BTreeSet<String>,
    pub craft_works: u16,
    pub distinct_recipes: BTreeSet<String>,
    pub wayfare_crossings: u16,
    pub private_martial_phase: Option<String>,
    pub witnessed_martial_phase: Option<String>,
    pub private_craft_phase: Option<String>,
    pub witnessed_craft_phase: Option<String>,
    pub witnessed_boss_slayer: bool,
    pub vessel_serial: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub past_aliases: Vec<String>,
}

/// 耗費木石鐵料與木工錢整修／增建之自立門戶新屋契（破除固定 6 棟空屋上限，零玩家特權保留）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BuiltHomestead {
    pub house_id: String,
    pub owner_id: String,
    pub anchor_node_id: String,
    pub label_zh: String,
    pub built_day: i64,
}

/// 同節點當面器物／山貨議價要約（供真人對真人或真人對居民單筆確認交割）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PeerTradeOffer {
    pub seller_id: String,
    pub seller_name: String,
    pub item_slug: String,
    pub item_uid: String,
    pub item_name: String,
    pub price_copper: i64,
    pub node_id: String,
    pub created_bell: i64,
}

pub fn ensure_chaos_rune_pool_seeded(delta: &mut IslandWorldDelta) {
    if !delta.chaos_rune_seeded {
        delta.chaos_rune_seeded = true;
        delta.chaos_rune_pool = 8;
    }
}

/// 統計全島當前存世流通之古符文原石與已鑲「穩固鐵紋」之兵刃／活器物總數（與 `chaos_rune_pool` 合併守恆 `<= 12`）
pub fn count_world_active_runes(delta: &IslandWorldDelta) -> u8 {
    let mut total: u32 = delta.live_wilds.count_active_runes();
    for a in delta.actors.values() {
        if !a.alive {
            continue;
        }
        if a.bound_weapon.rune_mark.as_deref() == Some("穩固鐵紋") {
            total = total.saturating_add(1);
        }
        let iter_items = a
            .backpack
            .iter()
            .chain(a.inn_locker.iter())
            .chain(a.inn_lockers.values().flat_map(|v| v.iter()));
        for it in iter_items {
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
    for bags in delta.dropped_loot_by_node.values() {
        for bag in bags {
            for it in &bag.items {
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
    }
    u8::try_from(total.min(250)).unwrap_or(250)
}

pub fn is_starter_loan_weapon(slug: &str) -> bool {
    matches!(
        slug,
        "harbor_hammer" | "old_hunting_knife" | "iron_walking_rod" | "belaying_club"
    ) || slug.starts_with("harbor_")
}

pub fn tool_carrier_of(slug: &str, name: &str) -> CarrierAffinity {
    if slug.contains("blade")
        || slug.contains("knife")
        || slug.contains("sickle")
        || name.contains('刀')
        || name.contains('刃')
        || name.contains('劍')
        || name.contains('鐮')
        || name.contains('叉')
        || name.contains('矛')
    {
        CarrierAffinity::Blade
    } else if slug.contains("hammer")
        || slug.contains("axe")
        || slug.contains("pick")
        || name.contains('錘')
        || name.contains('斧')
        || name.contains('鎬')
        || name.contains('杵')
        || name.contains('杖')
    {
        CarrierAffinity::Blunt
    } else {
        CarrierAffinity::Unarmed
    }
}

/// 查詢同節點除本人以外之在場活人目擊者名單（`DirectWitness` 判定唯一權威）
pub fn witnesses_at_node(
    delta: &IslandWorldDelta,
    node_id: &str,
    self_aid: &str,
) -> Vec<(String, String)> {
    let Some(occ) = delta.node_occupants.get(node_id) else {
        return Vec::new();
    };
    occ.iter()
        .filter(|&oid| oid != self_aid)
        .filter_map(|oid| delta.actors.get(oid))
        .filter(|o| o.alive && o.setup_done && o.node_id == node_id)
        .map(|o| (o.actor_id.clone(), o.display_name.clone()))
        .collect()
}

pub fn actor_bone_phase(a: &UnifiedMindBody) -> InnateBonePhase {
    let seed_i64 = ((a.soul_seed & ((1u64 << 63) - 1)).max(1)) as i64;
    InnateBonePhase::from_seed(seed_i64)
}

/// 推算兩名活人過招之 `human_phase` 三相優勢（一力降十會／專克拆招／十會解一力）
pub fn evaluate_actor_bout(
    a: &UnifiedMindBody,
    b: &UnifiedMindBody,
    slot: TacticalSlot,
    bout_idx: usize,
) -> (bool, String) {
    let bone_a = actor_bone_phase(a);
    let bone_b = actor_bone_phase(b);
    let car_a = tool_carrier_of(&a.bound_weapon.slug, &a.bound_weapon.name);
    let car_b = tool_carrier_of(&b.bound_weapon.slug, &b.bound_weapon.name);
    let prac_a = u32::from(a.mind.ltm.mastery.martial_bouts)
        + u32::from(a.bound_weapon.grip_familiarity / 2);
    let prac_b = u32::from(b.mind.ltm.mastery.martial_bouts)
        + u32::from(b.bound_weapon.grip_familiarity / 2);
    let learned_a = if a.mind.ltm.mastery.private_martial_phase.is_some() {
        0.55
    } else {
        (f64::from(prac_a) * 0.03).min(0.40)
    };
    let learned_b = if b.mind.ltm.mastery.private_martial_phase.is_some() {
        0.55
    } else {
        (f64::from(prac_b) * 0.03).min(0.40)
    };
    let scar_a = if a.wound_part.is_some() { 0.35 } else { 0.0 } + f64::from(a.fatigue) * 0.003;
    let scar_b = if b.wound_part.is_some() { 0.35 } else { 0.0 } + f64::from(b.fatigue) * 0.003;
    let rune_bonus_a = if a.bound_weapon.rune_mark.is_some() {
        4.5
    } else {
        0.0
    };
    let rune_bonus_b = if b.bound_weapon.rune_mark.is_some() {
        4.5
    } else {
        0.0
    };
    let posture_a = if a.wound_part.is_some() || a.fatigue >= 65 {
        PostureState::Shaken
    } else {
        PostureState::Steady
    };
    let posture_b = if b.wound_part.is_some() || b.fatigue >= 65 {
        PostureState::Shaken
    } else {
        PostureState::Steady
    };
    let res = evaluate_three_phase_bout_ext(
        slot,
        &bone_a,
        &bone_b,
        62.0 + rune_bonus_a,
        62.0 + rune_bonus_b,
        car_a,
        car_b,
        posture_a,
        posture_b,
        5,
        5,
        bout_idx,
        prac_a,
        prac_b,
        u32::try_from(a.mind.ltm.mastery.distinct_foes.len()).unwrap_or(0),
        u32::try_from(b.mind.ltm.mastery.distinct_foes.len()).unwrap_or(0),
        learned_a,
        learned_b,
        scar_a,
        scar_b,
    );
    let a_wins = res.score_delta_a >= res.score_delta_b;
    let note = res
        .counter_reason
        .unwrap_or_else(|| "雙方兵刃交擊、步法進退分明".to_string());
    (a_wins, note)
}

/// 實戰新情境奏效記錄：跨 >=2 類對手且實戰 >=2 場方凝出私有武相；同場有目擊者才晉升街頭公認名號與斬獸銘文！
pub fn record_martial_victory_and_check_phase(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_id: &str,
    foe_key: &str,
    foe_name_zh: &str,
    node_id: &str,
    is_historic_boss: bool,
) -> Option<(String, bool)> {
    let witnesses = witnesses_at_node(delta, node_id, actor_id);
    let has_witness = !witnesses.is_empty();

    let (phase_opt, newly_witnessed, actor_name, weapon_name) = {
        let a = delta.actors.get_mut(actor_id)?;
        a.mind.ltm.mastery.martial_bouts = a.mind.ltm.mastery.martial_bouts.saturating_add(1);
        a.mind.ltm.mastery.distinct_foes.insert(foe_key.to_string());

        if is_historic_boss && has_witness {
            a.mind.ltm.mastery.witnessed_boss_slayer = true;
            let deed = format!("・曾當眾擊潰{}", foe_name_zh);
            if !a.bound_weapon.provenance.contains(&deed) {
                if a.bound_weapon.provenance.is_empty() {
                    a.bound_weapon.provenance = format!("{}隨身慣用兵刃{}", a.display_name, deed);
                } else {
                    a.bound_weapon.provenance.push_str(&deed);
                }
            }
        }

        let qualifies =
            a.mind.ltm.mastery.martial_bouts >= 2 && a.mind.ltm.mastery.distinct_foes.len() >= 2;
        if !qualifies {
            a.mark_mutated();
            return None;
        }

        let bone = actor_bone_phase(a);
        let carrier = tool_carrier_of(&a.bound_weapon.slug, &a.bound_weapon.name);
        let prac = u32::from(a.mind.ltm.mastery.martial_bouts)
            + u32::from(a.bound_weapon.grip_familiarity / 2);
        let routine = derive_personal_martial_routine(
            &bone,
            TacticalSlot::Midgame,
            carrier,
            &a.bound_weapon.name,
            (f64::from(prac) * 0.05).min(0.85),
            prac,
            WordRegistry::global(),
        );
        let pname = routine.formal_name;
        if a.mind.ltm.mastery.private_martial_phase.is_none() {
            a.mind.ltm.mastery.private_martial_phase = Some(pname.clone());
        }
        let final_phase = a
            .mind
            .ltm
            .mastery
            .private_martial_phase
            .clone()
            .unwrap_or(pname);

        let mut just_witnessed = false;
        if has_witness && a.mind.ltm.mastery.witnessed_martial_phase.is_none() {
            a.mind.ltm.mastery.witnessed_martial_phase = Some(final_phase.clone());
            a.mind.ltm.emergent_phase_name = Some(final_phase.clone());
            a.mind.push_turning_point(
                &format!("phase:martial:{}:{}", actor_id, clock.game_days),
                clock.game_days,
                &format!(
                    "在旁人親眼見證下以【{}】使出貫通勁道，街坊漸傳其「{}」身手名號",
                    a.bound_weapon.name, final_phase
                ),
                "hone_craft",
            );
            just_witnessed = true;
        }
        a.mark_mutated();
        (
            final_phase,
            just_witnessed,
            a.display_name.clone(),
            a.bound_weapon.name.clone(),
        )
    };

    if newly_witnessed {
        broadcast_scene_event(
            doc,
            delta,
            clock,
            node_id,
            actor_id,
            &actor_name,
            EventPublicity::PublicLoud,
            Some("harbor_trade"),
            86,
            62,
            ValueAxis::CraftPride,
            "witness_martial_phase",
            &format!(
                "親眼見{}揮動【{}】連貫破敵，一手「{}」成相勁道令人側目",
                actor_name, weapon_name, phase_opt
            ),
        );
        super::living_chronicler::record_witnessed_phase(
            doc,
            delta,
            clock,
            actor_id,
            node_id,
            &weapon_name,
            &phase_opt,
            true,
        );
    }
    Some((phase_opt, has_witness))
}

/// 百工新情境奏效記錄：跨 >=2 類工藝配方且累積 >=3 件實作方凝出百工藝相；有目擊者方公認傳名！
pub fn record_craft_work_and_check_phase(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_id: &str,
    recipe_key: &str,
    domain_code: &str,
    node_id: &str,
) -> Option<(String, bool)> {
    let has_witness = !witnesses_at_node(delta, node_id, actor_id).is_empty();
    let (phase_name, newly_witnessed, actor_name) = {
        let a = delta.actors.get_mut(actor_id)?;
        a.mind.ltm.mastery.craft_works = a.mind.ltm.mastery.craft_works.saturating_add(1);
        a.mind
            .ltm
            .mastery
            .distinct_recipes
            .insert(recipe_key.to_string());
        if a.mind.ltm.mastery.craft_works < 3 || a.mind.ltm.mastery.distinct_recipes.len() < 2 {
            a.mark_mutated();
            return None;
        }
        let bone = actor_bone_phase(a);
        let carrier = tool_carrier_of(&a.bound_weapon.slug, &a.bound_weapon.name);
        let cnt = u32::from(a.mind.ltm.mastery.craft_works);
        let routine = derive_personal_craft_routine(
            &bone,
            domain_code,
            carrier,
            &a.bound_weapon.name,
            cnt,
            u32::try_from(a.mind.ltm.mastery.distinct_recipes.len()).unwrap_or(2),
            WordRegistry::global(),
        );
        if a.mind.ltm.mastery.private_craft_phase.is_none() {
            a.mind.ltm.mastery.private_craft_phase = Some(routine.formal_name.clone());
        }
        let p_final = a
            .mind
            .ltm
            .mastery
            .private_craft_phase
            .clone()
            .unwrap_or(routine.formal_name);
        let mut just_wit = false;
        if has_witness && a.mind.ltm.mastery.witnessed_craft_phase.is_none() {
            a.mind.ltm.mastery.witnessed_craft_phase = Some(p_final.clone());
            if a.mind.ltm.emergent_phase_name.is_none() {
                a.mind.ltm.emergent_phase_name = Some(p_final.clone());
            }
            a.mind.push_turning_point(
                &format!("phase:craft:{}:{}", actor_id, clock.game_days),
                clock.game_days,
                &format!("爐案前手法純熟自成一格，街坊稱道其「{}」手藝火候", p_final),
                "hone_craft",
            );
            just_wit = true;
        }
        a.mark_mutated();
        (p_final, just_wit, a.display_name.clone())
    };

    if newly_witnessed {
        broadcast_scene_event(
            doc,
            delta,
            clock,
            node_id,
            actor_id,
            &actor_name,
            EventPublicity::PublicLoud,
            Some("harbor_trade"),
            78,
            52,
            ValueAxis::CraftPride,
            "witness_craft_phase",
            &format!(
                "親眼見{}施展「{}」純熟百工手路，成器俐落穩妥",
                actor_name, phase_name
            ),
        );
        let craft_wep = delta
            .actors
            .get(actor_id)
            .map_or_else(|| "匠作器具".to_string(), |a| a.bound_weapon.name.clone());
        super::living_chronicler::record_witnessed_phase(
            doc,
            delta,
            clock,
            actor_id,
            node_id,
            &craft_wep,
            &phase_name,
            false,
        );
    }
    Some((phase_name, has_witness))
}

/// 玩家與匠師居民 100% 共用之「唯一落款活器物」鍛造動詞（消耗真實背囊原料，零憑空生貨！）
pub fn forge_living_vessel(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let node_id = delta
        .actors
        .get(&aid)
        .map(|a| a.node_id.clone())
        .ok_or("找不到行蹤")?;
    let node = doc
        .nodes
        .get(&node_id)
        .ok_or("四周霧氣迷濛，辨不清工坊方位。")?;
    let at_forge = node.name.contains("鍛鐵")
        || node.name.contains("鐵匠")
        || node.name.contains("木匠")
        || delta.actors.get(&aid).is_some_and(|a| {
            a.work_node == node_id
                && (a.role.contains("鐵") || a.role.contains("木") || a.role.contains("匠"))
        });
    if !at_forge {
        return Err("需在鐵匠鋪石爐或木工案台旁才能開爐鍛製器物。".to_string());
    }
    if clock.is_night
        && delta.actors.get(&aid).is_some_and(|a| {
            a.controller_id.is_some() && a.owned_house_node.as_deref() != Some(node_id.as_str())
        })
    {
        return Err("夜深鋪面已封爐落栓打烊，待破曉後晨鐘二響再開爐鍛器吧。".to_string());
    }
    if delta.actors.get(&aid).is_some_and(|a| a.fatigue >= 82) {
        return Err("雙臂酸沉力乏，先歇口氣再握錘開爐才不會廢了胚料。".to_string());
    }

    let (in_owned_house, actor_has_coal) = delta.actors.get(&aid).map_or((false, false), |a| {
        let owned = a.owned_house_node.as_deref() == Some(node_id.as_str());
        let has_coal = a.backpack.iter().any(|i| {
            matches!(i.slug.as_str(), "black_coal" | "charcoal_sack")
                && i.count > 0
                && i.bound_owner.is_none()
                && i.stolen_from.is_none()
        });
        (owned, has_coal)
    });
    let shop_cold =
        !in_owned_house && super::live_commodities::is_smithy_forge_cold(delta, &node_id);
    if shop_cold && !actor_has_coal {
        return Err(
            "鍛爐裡的精煤與木炭已燒盡熄火，需先補煤生火（或隨身自備精煤／木炭）才能開爐鍛器！"
                .to_string(),
        );
    }

    let yr = solar_year_and_season_of_day(clock.game_days)
        .0
        .max(CURRENT_SOLAR_YEAR);
    let (vessel_name, recipe_key, rune_opt, maker_name) = {
        let a = delta.actors.get_mut(&aid).ok_or("找不到行蹤")?;
        let can_use_work_box =
            a.work_node == node_id || a.owned_house_node.as_deref() == Some(node_id.as_str());
        let cnt_of = |act: &UnifiedMindBody, s: &str| -> u32 {
            let bag_c: u32 = act
                .backpack
                .iter()
                .filter(|i| i.slug == s && i.bound_owner.is_none() && i.stolen_from.is_none())
                .map(|i| i.count)
                .sum();
            let box_c: u32 = if can_use_work_box {
                act.inn_lockers.get(&node_id).map_or(0, |v| {
                    v.iter()
                        .filter(|i| {
                            i.slug == s && i.bound_owner.is_none() && i.stolen_from.is_none()
                        })
                        .map(|i| i.count)
                        .sum()
                })
            } else {
                0
            };
            bag_c + box_c
        };
        let consume_mat = |act: &mut UnifiedMindBody, s: &str, mut need: u32| {
            while need > 0 {
                if super::movement::take_clean_carry_items(&mut act.backpack, s, 1).is_some() {
                    need -= 1;
                } else if can_use_work_box
                    && let Some(bv) = act.inn_lockers.get_mut(&node_id)
                    && super::movement::take_clean_carry_items(bv, s, 1).is_some()
                {
                    need -= 1;
                } else {
                    break;
                }
            }
        };
        let n_rune = cnt_of(a, "rune_stone");
        let n_cara = cnt_of(a, "spider_carapace");
        let n_scale = cnt_of(a, "lizard_scale");
        let n_iron = cnt_of(a, "iron_ore");
        let n_wood = cnt_of(a, "driftwood");

        let (slug, vname, rkey, rune_mark, w_units) = if n_rune >= 1 && n_iron >= 1 {
            consume_mat(a, "rune_stone", 1);
            consume_mat(a, "iron_ore", 1);
            (
                "forged_rune_hammer",
                format!("{}鍛・嵌紋破甲錘", a.display_name),
                "rune_iron_hammer",
                Some("穩固鐵紋".to_string()),
                3u16,
            )
        } else if (n_cara >= 1 || n_scale >= 1) && (n_iron >= 1 || n_wood >= 1) {
            let use_cara = n_cara >= 1;
            if use_cara {
                consume_mat(a, "spider_carapace", 1);
            } else {
                consume_mat(a, "lizard_scale", 1);
            }
            if n_iron >= 1 {
                consume_mat(a, "iron_ore", 1);
            } else {
                consume_mat(a, "driftwood", 1);
            }
            (
                "forged_carapace_blade",
                format!(
                    "{}鍛・{}",
                    a.display_name,
                    if use_cara {
                        "甲脊獵刃"
                    } else {
                        "鱗脊獵刃"
                    }
                ),
                if use_cara {
                    "carapace_blade"
                } else {
                    "lizard_scale_blade"
                },
                Some("甲脊利紋".to_string()),
                2u16,
            )
        } else if n_iron >= 1 && n_wood >= 1 {
            consume_mat(a, "iron_ore", 1);
            consume_mat(a, "driftwood", 1);
            (
                "forged_iron_blade",
                format!("{}製・堅木厚背刀", a.display_name),
                "iron_wood_blade",
                None,
                2u16,
            )
        } else if n_iron >= 2 {
            consume_mat(a, "iron_ore", 2);
            (
                "forged_heavy_hammer",
                format!("{}鍛・百煉重鐵錘", a.display_name),
                "double_iron_hammer",
                None,
                3u16,
            )
        } else if n_wood >= 2 {
            consume_mat(a, "driftwood", 2);
            (
                "forged_wood_staff",
                format!("{}製・沉木護身手杖", a.display_name),
                "double_driftwood_staff",
                None,
                2u16,
            )
        } else {
            return Err(
                "背囊或工坊木箱裡鍛料不足（至少需「粗鐵礦石＋漂流木」、「粗鐵礦石×2」、「漂流木×2」，或搭配巨蛛甲殼／古符文原石）。"
                    .to_string(),
            );
        };

        if shop_cold {
            if super::movement::take_clean_carry_items(&mut a.backpack, "black_coal", 1).is_none() {
                let _taken_charcoal =
                    super::movement::take_clean_carry_items(&mut a.backpack, "charcoal_sack", 1);
            }
        }

        a.mind.ltm.mastery.vessel_serial = a.mind.ltm.mastery.vessel_serial.saturating_add(1);
        let serial = a.mind.ltm.mastery.vessel_serial;
        let uid = format!("vsl:{}:{}:{}", a.soul_seed % 100_000, yr, serial);
        let craft_tag = a
            .mind
            .ltm
            .mastery
            .witnessed_craft_phase
            .as_deref()
            .map(|p| format!("（施展「{p}」手路）"))
            .unwrap_or_default();
        let prov = format!(
            "太陽曆 {} 年由{}於{}親手鍛造{}（編號 #{}）",
            yr, a.display_name, node.name, craft_tag, serial
        );
        let meta = BoundTool {
            uid: uid.clone(),
            slug: slug.to_string(),
            name: vname.clone(),
            provenance: prov,
            wear_stage: 0,
            grip_familiarity: 12,
            rune_mark: rune_mark.clone(),
        };
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: format!("vessel_{uid}"),
                name: vname.clone(),
                count: 1,
                bound_owner: None,
                stolen_from: None,
                weight_units: w_units,
                vessel_meta: Some(meta),
            },
        );
        a.fatigue = (a.fatigue + 16).min(95);
        a.bound_weapon.grip_familiarity = a.bound_weapon.grip_familiarity.saturating_add(4);
        a.mind.stm.today_goal_progressed = true;
        a.mark_mutated();
        (vname, rkey, rune_mark, a.display_name.clone())
    };

    let _ =
        record_craft_work_and_check_phase(doc, delta, clock, &aid, recipe_key, "forge", &node_id);
    super::living_chronicler::record_vessel_forged(
        doc,
        delta,
        clock,
        &aid,
        &node_id,
        &vessel_name,
        rune_opt.as_deref(),
    );
    if rune_opt.is_some() {
        broadcast_scene_event(
            doc,
            delta,
            clock,
            &node_id,
            &aid,
            &maker_name,
            EventPublicity::PublicLoud,
            Some("harbor_trade"),
            80,
            55,
            ValueAxis::CraftPride,
            "forge_living_vessel",
            &format!(
                "{}在爐火飛濺間親手鍛成落款銘器【{}】",
                maker_name, vessel_name
            ),
        );
    }
    let consumed_shop_fuel = if !in_owned_house && !shop_cold {
        super::live_commodities::consume_smithy_fuel_one(delta, None, &node_id).is_some()
    } else {
        false
    };
    let fuel_note = if shop_cold {
        "（投入隨身炭薪重開爐火）"
    } else if consumed_shop_fuel {
        "（添入鋪爐炭薪開鍛）"
    } else {
        ""
    };
    let msg = format!(
        "爐火淬水白霧騰起{}，你親手鍛成落款活器物【{}】並收入背囊（可隨時換握上手，或當面議價讓渡予街坊旅人）！",
        fuel_note, vessel_name
    );
    if let Some(a) = delta.actors.get_mut(&aid) {
        push_breath(a, msg.clone());
    }
    Ok(msg)
}

/// 換握背囊內之活器物（原隨身器物退回背囊保存其來歷與磨損；港務借領起手器物保留記名防洗鐵）
pub fn equip_living_vessel(
    delta: &mut IslandWorldDelta,
    actor_key: &str,
    target_slug_or_uid: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let a = delta.actors.get_mut(&aid).ok_or("找不到行蹤")?;
    let idx = a
        .backpack
        .iter()
        .position(|i| {
            i.count > 0
                && i.vessel_meta.is_some()
                && (target_slug_or_uid.is_empty()
                    || i.slug == target_slug_or_uid
                    || i.vessel_meta
                        .as_ref()
                        .is_some_and(|m| m.uid == target_slug_or_uid))
        })
        .ok_or("背囊裡沒有可換握的備用兵刃或工具。")?;

    let new_tool = if a.backpack[idx].count > 1 {
        a.backpack[idx].count -= 1;
        a.backpack[idx]
            .vessel_meta
            .clone()
            .ok_or("該物件並非可握持器物。")?
    } else {
        a.backpack
            .remove(idx)
            .vessel_meta
            .ok_or("該物件並非可握持器物。")?
    };
    let old_tool = std::mem::replace(&mut a.bound_weapon, new_tool);

    let is_harbor_loan = is_starter_loan_weapon(&old_tool.slug);
    let old_slug = if old_tool.uid.is_empty() {
        old_tool.slug.clone()
    } else {
        format!("vessel_{}", old_tool.uid)
    };
    add_carry_item(
        &mut a.backpack,
        CarryItem {
            slug: old_slug,
            name: old_tool.name.clone(),
            count: 1,
            bound_owner: is_harbor_loan.then(|| aid.clone()),
            stolen_from: None,
            weight_units: 2,
            vessel_meta: Some(old_tool),
        },
    );
    a.mark_mutated();
    let msg = format!(
        "你將【{}】換握在手（來歷：{}），原器物妥善收進背囊。",
        a.bound_weapon.name,
        if a.bound_weapon.provenance.is_empty() {
            "隨身舊器"
        } else {
            &a.bound_weapon.provenance
        }
    );
    push_breath(a, msg.clone());
    Ok(msg)
}

/// 估算一件活器物或山海特產之公平基準銅幣價（依刻紋、磨損與落款決定）
pub fn appraise_item_fair_copper(item: &CarryItem) -> i64 {
    if let Some(ref vm) = item.vessel_meta {
        let base = if vm.rune_mark.as_deref() == Some("穩固鐵紋") {
            28_i64
        } else if vm.rune_mark.is_some() {
            22_i64
        } else {
            14_i64
        };
        let wear_pen = i64::from(vm.wear_stage) * 3;
        (base - wear_pen).max(6)
    } else {
        match item.slug.as_str() {
            "rune_stone" => 15,
            "spider_carapace" => 12,
            "lizard_scale" => 8,
            "iron_ore" => 5,
            "driftwood" => 4,
            "herb_bundle" => 4,
            "rye_bread" => 2,
            _ => 4,
        }
    }
}

/// 供 `[自身]`、`/chronicle` 與 Markdown 列傳顯示之成相雙層摘要
pub fn format_mastery_status_zh(a: &UnifiedMindBody) -> String {
    let m = &a.mind.ltm.mastery;
    let mut parts = Vec::new();
    if let Some(ref wm) = m.witnessed_martial_phase {
        parts.push(format!("公認武相【{}】", wm));
    } else if let Some(ref pm) = m.private_martial_phase {
        parts.push(format!("私下練熟武路【{}】（尚無旁人目擊公認）", pm));
    }
    if let Some(ref wc) = m.witnessed_craft_phase {
        parts.push(format!("公認藝相【{}】", wc));
    } else if let Some(ref pc) = m.private_craft_phase {
        parts.push(format!("私下揣摩手路【{}】（尚無旁人目擊公認）", pc));
    }
    if parts.is_empty() {
        format!(
            "碎招磨合中（實戰 {} 合・百工 {} 件）",
            m.martial_bouts, m.craft_works
        )
    } else {
        parts.join("｜")
    }
}

/// 共用無目擊吹牛拆穿檢定（供真人傳話與對 NPC 開口雙路徑統一呼叫；僅通曉該語言 `>= 60` 之在場活人可出聲拆穿！）
pub fn check_and_expose_unwitnessed_boast(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    speaker_aid: &str,
    node_id: &str,
    lang_code: &str,
    utterance: &str,
) -> Option<String> {
    let boasts_spider = utterance.contains("我斬了巨蛛")
        || utterance.contains("親手斬殺巨蛛")
        || utterance.contains("獨自宰了巨蛛");
    let boasts_master = utterance.contains("天下無敵") || utterance.contains("練成絕世");
    if !boasts_spider && !boasts_master {
        return None;
    }
    let (sp_name, has_wit_boss, has_wit_phase) = {
        let sp = delta.actors.get(speaker_aid)?;
        (
            sp.display_name.clone(),
            sp.mind.ltm.mastery.witnessed_boss_slayer,
            sp.mind.ltm.mastery.witnessed_martial_phase.is_some(),
        )
    };
    if !(boasts_spider && !has_wit_boss) && !(boasts_master && !has_wit_phase) {
        return None;
    }
    let (_wid, wname) = witnesses_at_node(delta, node_id, speaker_aid)
        .into_iter()
        .find(|(wid, _)| {
            delta.actors.get(wid).is_some_and(|w| {
                let prof = if lang_code == "harbor_trade" {
                    w.languages.get("harbor_trade").copied().unwrap_or(80)
                } else {
                    w.languages.get(lang_code).copied().unwrap_or(0)
                };
                prof >= 60
            })
        })?;
    let callout = format!(
        "{}聽罷嗤笑一聲當場拆穿：「慢著！港裡誰也沒見過這樁戰績，少拿沒影的事吹大氣！」",
        wname
    );
    broadcast_scene_event(
        doc,
        delta,
        clock,
        node_id,
        speaker_aid,
        &sp_name,
        EventPublicity::PublicLoud,
        Some("harbor_trade"),
        68,
        45,
        ValueAxis::PactIntegrity,
        "boast_exposed",
        &format!("{}當眾誇口冒功，被在場的{}開口拆穿", sp_name, wname),
    );
    Some(callout)
}
