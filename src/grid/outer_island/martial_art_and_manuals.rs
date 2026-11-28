//! 第三刀（拆骨補實版）：個人之相（4 武相＋4 藝相實算施展）× 實體內文《功法手札》（真同法不同相）× 獨立三相連續交鋒結算。
//!
//! 嚴守正典與關鍵字鐵律：
//! 1. 4 臨陣武相於交鋒自動施展；4 百工藝相（採獵／鍛造／庖廚／行旅）透過 `invoke_craft_phase_effect` 於真實生產採作時實際參與減耗與敘事；
//! 2. 《功法手札》在 `CarryItem.manual_meta` 永久保存作者、載體、槽位、五常動作序列與變招分支；研讀時依該原譜槽位與研讀者自身骨相重算 Viterbi 代償（真「同法不同相」）；
//! 3. 連續交鋒勝出直走共用 `settle_skirmish_victory`，絕不借殼舊猜拳 `skirmish_wrist`，且前台徹底禁絕任何幕後設計黑話。

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::combat::human_phase::{
    EmergentRoutine, InnateBonePhase, ThreePhaseVerdict, derive_personal_craft_routine,
    derive_personal_martial_routine, evaluate_three_phase_bout_ext,
};
use crate::combat::wordplate::{CarrierAffinity, PostureState, TacticalSlot, WordRegistry};

use super::clock_and_lang::SolarClockSnapshot;
use super::mind::EventPublicity;
use super::movement::{add_carry_item, push_breath};
use super::persona::ValueAxis;
use super::phase_and_vessels::{actor_bone_phase, tool_carrier_of};
use super::sim_tick::broadcast_scene_event;
use super::types::{CarryItem, IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

/// 存於實體《功法手札》物件上之不可變原譜內文，讓任何研讀者皆可依原譜槽位與自身骨相走出「同法不同相」。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MartialManualMeta {
    pub author_name: String,
    pub slot_code: String,
    pub carrier_zh: String,
    pub formal_name: String,
    pub motions: Vec<String>,
    pub turn_branch_zh: String,
    pub origin_seed: u64,
}

fn slot_from_code(code: &str) -> TacticalSlot {
    match code.trim() {
        "opening" => TacticalSlot::Opening,
        "finisher" => TacticalSlot::Finisher,
        "defense" => TacticalSlot::Defense,
        _ => TacticalSlot::Midgame,
    }
}

fn carrier_label_zh(c: CarrierAffinity) -> &'static str {
    match c {
        CarrierAffinity::Blade => "刃具",
        CarrierAffinity::Blunt => "重錘",
        CarrierAffinity::Pole => "長柄",
        CarrierAffinity::Shield => "盾牌護具",
        CarrierAffinity::Footwork => "步法身法",
        CarrierAffinity::Unarmed => "徒手身法",
    }
}

/// 將先天骨相三軸 `(alpha, beta, gamma)` 轉譯為純去數值體感敘述
pub fn describe_bone_phase_zh(bone: &InnateBonePhase) -> String {
    let power_speed = if bone.alpha >= 1.18 {
        "筋骨沉雄厚重、發勁力矩極深，唯連招換氣略緩"
    } else if bone.beta >= 1.18 {
        "身段輕靈捷快、出手頻率綿密，勝在先手與變招"
    } else {
        "力道與步速相衡，進退平穩少有偏廢"
    };
    let style = if bone.gamma >= 0.25 {
        "氣息偏守正沉穩，最宜扎實架勢步步推進"
    } else if bone.gamma <= -0.25 {
        "氣息偏奇正流轉，善於臨場借勢變招破局"
    } else {
        "氣息圓融中和，可隨局勢剛柔並濟"
    };
    format!("{power_speed}；{style}。")
}

/// 推算角色當前所學加成 `learned_bonus`（隨歷練與研讀《功法手札》成長，上限 0.90）
pub fn actor_learned_bonus(a: &UnifiedMindBody) -> f64 {
    let prac = u32::from(a.mind.ltm.mastery.martial_bouts)
        + u32::from(a.bound_weapon.grip_familiarity / 2);
    let manual_boost = (a.mind.ltm.mastery.studied_manuals.len() as f64) * 0.08;
    let base = (f64::from(prac) * 0.04 + manual_boost).min(0.85);
    if a.mind.ltm.mastery.private_martial_phase.is_some() {
        base.max(0.55)
    } else {
        base
    }
}

/// 依所研讀手札之具體招式槽位（起手／連攜／破綻／守勢）疊加專項動作引導加成
pub fn actor_learned_bonus_for_slot(a: &UnifiedMindBody, slot: TacticalSlot) -> f64 {
    let slot_key = match slot {
        TacticalSlot::Opening => "opening",
        TacticalSlot::Midgame => "midgame",
        TacticalSlot::Finisher => "finisher",
        TacticalSlot::Defense => "defense",
    };
    let slot_depth = f64::from(
        a.mind
            .ltm
            .mastery
            .studied_manual_slots
            .get(slot_key)
            .copied()
            .unwrap_or(0),
    );
    (actor_learned_bonus(a) + slot_depth * 0.12).min(0.95)
}

/// 在真實採獵／鍛造／庖廚／行旅時自動施展角色專屬百工藝相，回傳 `(白話施展句, 實算省力減耗)`！
pub fn invoke_craft_phase_effect(a: &UnifiedMindBody, craft_code: &str) -> (String, u8) {
    let bone = actor_bone_phase(a);
    let carrier = tool_carrier_of(&a.bound_weapon.slug, &a.bound_weapon.name);
    let craft_fam = u32::from(a.mind.ltm.mastery.craft_works)
        + u32::from(a.bound_weapon.grip_familiarity / 3);
    let anatomy_fam = u32::try_from(a.mind.ltm.mastery.distinct_foes.len()).unwrap_or(0) * 3
        + u32::from(a.mind.ltm.mastery.wayfare_crossings / 2);
    let r = derive_personal_craft_routine(
        &bone,
        craft_code,
        carrier,
        &a.bound_weapon.name,
        craft_fam,
        anatomy_fam,
        WordRegistry::global(),
    );
    let save: u8 = if craft_fam >= 12 {
        3
    } else if craft_fam >= 4 {
        2
    } else {
        1
    };
    let lead = r.main_motions.first().map_or("順手運勁", String::as_str);
    let close = r.main_motions.last().map_or("收勢落定", String::as_str);
    (
        format!("順手使出【{}】（{lead}→{close}）", r.formal_name),
        save,
    )
}

/// 自然生成角色背板「4 臨陣武相」與「4 百工藝相」（完全由骨相×所學×所熟×手持器物決定）
pub fn derive_actor_eight_phases(
    a: &UnifiedMindBody,
) -> (Vec<EmergentRoutine>, Vec<EmergentRoutine>) {
    let bone = actor_bone_phase(a);
    let carrier = tool_carrier_of(&a.bound_weapon.slug, &a.bound_weapon.name);
    let vessel = a.bound_weapon.name.as_str();
    let martial_fam = u32::from(a.mind.ltm.mastery.martial_bouts)
        + u32::from(a.bound_weapon.grip_familiarity / 2);
    let craft_fam = u32::from(a.mind.ltm.mastery.craft_works)
        + u32::from(a.bound_weapon.grip_familiarity / 3);
    let anatomy_fam = u32::try_from(a.mind.ltm.mastery.distinct_foes.len()).unwrap_or(0) * 3
        + u32::from(a.mind.ltm.mastery.wayfare_crossings / 2);
    let reg = WordRegistry::global();

    let martial_slots = [
        TacticalSlot::Opening,
        TacticalSlot::Midgame,
        TacticalSlot::Finisher,
        TacticalSlot::Defense,
    ];
    let martial: Vec<EmergentRoutine> = martial_slots
        .iter()
        .map(|&slot| {
            let slot_learned = actor_learned_bonus_for_slot(a, slot);
            derive_personal_martial_routine(
                &bone,
                slot,
                carrier,
                vessel,
                slot_learned,
                martial_fam,
                reg,
            )
        })
        .collect();

    let craft_codes = ["harvest", "forge", "culinary", "wayfare"];
    let craft: Vec<EmergentRoutine> = craft_codes
        .iter()
        .map(|&code| {
            derive_personal_craft_routine(
                &bone,
                code,
                carrier,
                vessel,
                craft_fam,
                anatomy_fam,
                reg,
            )
        })
        .collect();

    (martial, craft)
}

/// 序列化個人之相背板（4 武相＋4 藝相＋先天骨相體感＋已研讀手札，100% 去數值化）
pub fn build_eight_phases_view_json(viewer: &UnifiedMindBody) -> Value {
    let bone = actor_bone_phase(viewer);
    let (martial, craft) = derive_actor_eight_phases(viewer);
    let fmt_routine = |r: &EmergentRoutine| {
        let chain = r.main_motions.join(" → ");
        let comp_note = if r.compensated_stages.is_empty() {
            "順骨相經脈一氣呵成"
        } else {
            "已依自身骨相代償改道"
        };
        json!({
            "slot_code": r.slot_code,
            "slot_label": r.slot_label_zh,
            "formal_name": r.formal_name,
            "specialty": r.phase_specialty_zh,
            "chain": chain,
            "turn_branch": format!("遇{}則轉「{}」", r.turn_counter_target_zh, r.turn_branch_motion),
            "comp_note": comp_note,
            "basis": r.origin_basis_zh,
        })
    };

    let can_author = viewer.mind.ltm.mastery.martial_bouts >= 1
        || viewer.bound_weapon.grip_familiarity >= 10
        || !viewer.mind.ltm.mastery.studied_manuals.is_empty();

    json!({
        "bone_sense": describe_bone_phase_zh(&bone),
        "can_author_manual": can_author,
        "studied_manuals": viewer.mind.ltm.mastery.studied_manuals,
        "martial": martial.iter().map(fmt_routine).collect::<Vec<_>>(),
        "craft": craft.iter().map(fmt_routine).collect::<Vec<_>>(),
    })
}

/// 將自身一式武相著譜成實體《功法手札》（內含真實原譜招式序列 `manual_meta`，可供交易傳授）
pub fn apply_write_martial_manual(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    slot_code: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let slot = match slot_code.trim() {
        "opening" => TacticalSlot::Opening,
        "midgame" => TacticalSlot::Midgame,
        "finisher" => TacticalSlot::Finisher,
        "defense" => TacticalSlot::Defense,
        _ => return Err("請選擇欲著譜的武相部位（起手／連攜／破綻／守勢）。".to_string()),
    };

    let (node_id, author_name, origin_seed, carrier_zh, routine) = {
        let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
        if a.fatigue >= 85 {
            return Err("此刻氣力已近枯竭，無力靜心凝神著譜，先歇息緩氣再動筆吧。".to_string());
        }
        let day_key = format!("manual_write:d{}", clock.game_days);
        if a.explored_windows.contains(&day_key) {
            return Err("今日已耗神撰寫過一冊功法手札，明日靜氣後再續寫新卷。".to_string());
        }
        let has_ink_paper = a.backpack.iter().any(|i| {
            i.count > 0
                && i.bound_owner.is_none()
                && i.stolen_from.is_none()
                && matches!(
                    i.slug.as_str(),
                    "tough_vine" | "herb_bundle" | "charcoal_sack" | "black_coal"
                )
        });
        if !has_ink_paper {
            return Err(
                "著譜成冊須備妥紙墨炭料（行囊內須有【韌藤纖維】／【草束】製藤紙，或【木炭】／【北林精煤】研墨 1 份），不可憑空變書。"
                    .to_string(),
            );
        }
        let bone = actor_bone_phase(a);
        let carrier = tool_carrier_of(&a.bound_weapon.slug, &a.bound_weapon.name);
        let prac = u32::from(a.mind.ltm.mastery.martial_bouts)
            + u32::from(a.bound_weapon.grip_familiarity / 2);
        let r = derive_personal_martial_routine(
            &bone,
            slot,
            carrier,
            &a.bound_weapon.name,
            actor_learned_bonus_for_slot(a, slot),
            prac,
            WordRegistry::global(),
        );
        (
            a.node_id.clone(),
            a.display_name.clone(),
            a.soul_seed,
            carrier_label_zh(carrier),
            r,
        )
    };

    let book_name = format!("《{}・手札》", routine.formal_name);
    let meta = MartialManualMeta {
        author_name: author_name.clone(),
        slot_code: slot_code.trim().to_string(),
        carrier_zh: carrier_zh.to_string(),
        formal_name: routine.formal_name.clone(),
        motions: routine.main_motions.to_vec(),
        turn_branch_zh: format!(
            "遇{}則轉「{}」",
            routine.turn_counter_target_zh, routine.turn_branch_motion
        ),
        origin_seed,
    };
    let consumed_mat_zh = {
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        let mut taken_label: Option<&'static str> = None;
        for (mslug, mlabel) in [
            ("tough_vine", "韌藤製紙"),
            ("herb_bundle", "草纖壓紙"),
            ("charcoal_sack", "硬木炭條"),
            ("black_coal", "精煤研墨"),
        ] {
            if super::movement::take_clean_carry_items(&mut a.backpack, mslug, 1).is_some() {
                taken_label = Some(mlabel);
                break;
            }
        }
        let Some(used_lbl) = taken_label else {
            return Err("著譜所需的藤紙／草束／炭料已不在行囊內。".to_string());
        };
        a.mind.ltm.mastery.vessel_serial = a.mind.ltm.mastery.vessel_serial.saturating_add(1);
        let s = format!("manual_{}_{}", slot_code.trim(), a.mind.ltm.mastery.vessel_serial);
        let day_key = format!("manual_write:d{}", clock.game_days);
        a.explored_windows.insert(day_key);
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: s,
                name: book_name.clone(),
                count: 1,
                weight_units: 1,
                manual_meta: Some(meta),
                ..CarryItem::default()
            },
        );
        a.fatigue = (a.fatigue + 10).min(95);
        a.last_active_unix = clock.real_unix;
        let msg = format!(
            "你耗去一份{used_lbl}，靜心回味掌中發力與起承轉合，將【{}】（{}）寫成實體{}收入行囊，可留贈同道、傳徒或於市集議價流轉。",
            routine.formal_name,
            routine.main_motions.join("→"),
            book_name
        );
        push_breath(a, msg);
        a.mark_mutated();
        used_lbl
    };

    broadcast_scene_event(
        doc,
        delta,
        clock,
        &node_id,
        &aid,
        &author_name,
        EventPublicity::PublicLoud,
        None,
        32,
        14,
        ValueAxis::CraftPride,
        "write_manual",
        &format!("{author_name}凝神著成一冊{book_name}"),
    );
    Ok(format!(
        "你已以{consumed_mat_zh}著成實體{}收入行囊（五常碎招：{}），他人研讀後亦會依自身骨相走出同法不同相的新路。",
        book_name,
        routine.main_motions.join(" → ")
    ))
}

/// 研讀行囊內之實體《功法手札》（讀取手札真實 `manual_meta` 原譜槽位與動作，依研讀者骨相走出真「同法不同相」，且書冊完好不毀！）
pub fn apply_study_martial_manual(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    slug: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    let (book_name, meta_opt) = a
        .backpack
        .iter()
        .find(|i| i.slug == slug && i.count > 0 && i.slug.starts_with("manual_"))
        .map(|i| (i.name.clone(), i.manual_meta.clone()))
        .ok_or("行囊裡找不到這冊功法手札。")?;
    if a.mind.ltm.mastery.studied_manuals.contains(&book_name) {
        return Ok(format!(
            "你重新翻閱了一遍{book_name}，其中的起承轉合要訣早已融進你的身法裡；此冊仍完好留在行囊，可轉贈或售予他人。"
        ));
    }

    // 防止原地重複讀自己寫的手札白刷熟悉度（須實戰歷練或跨人交流方得最大增益）
    let is_self_authored = meta_opt
        .as_ref()
        .is_some_and(|m| m.origin_seed != 0 && m.origin_seed == a.soul_seed);
    if a.mind.ltm.mastery.studied_manuals.len() < 8 {
        a.mind.ltm.mastery.studied_manuals.push(book_name.clone());
    }
    let target_slot_code = meta_opt
        .as_ref()
        .map_or("midgame", |m| m.slot_code.as_str())
        .to_string();
    let target_slot = slot_from_code(&target_slot_code);
    if !is_self_authored {
        a.bound_weapon.grip_familiarity =
            a.bound_weapon.grip_familiarity.saturating_add(3).min(40);
        let entry = a
            .mind
            .ltm
            .mastery
            .studied_manual_slots
            .entry(target_slot_code)
            .or_insert(0);
        *entry = entry.saturating_add(1).min(4);
    }
    a.fatigue = (a.fatigue + 6).min(95);
    a.last_active_unix = clock.real_unix;

    let bone = actor_bone_phase(a);
    let carrier = tool_carrier_of(&a.bound_weapon.slug, &a.bound_weapon.name);
    let prac = u32::from(a.mind.ltm.mastery.martial_bouts)
        + u32::from(a.bound_weapon.grip_familiarity / 2);
    let evolved = derive_personal_martial_routine(
        &bone,
        target_slot,
        carrier,
        &a.bound_weapon.name,
        actor_learned_bonus_for_slot(a, target_slot),
        prac,
        WordRegistry::global(),
    );

    let contrast_note = if let Some(ref m) = meta_opt
        && !m.motions.is_empty()
    {
        if is_self_authored {
            format!(
                "對照自己落筆的「{}」，再順一遍呼吸與重心，手路愈發清晰",
                m.motions.join("→")
            )
        } else {
            format!(
                "原譜載{}以{}行「{}」，經你依自身骨相代償化用，掌中【{}】走出屬於你自己的「{}」（{}）",
                m.author_name,
                m.carrier_zh,
                m.motions.join("→"),
                a.bound_weapon.name,
                evolved.formal_name,
                evolved.main_motions.join("→")
            )
        }
    } else {
        format!(
            "順著自身骨相自然代償消化，掌中【{}】流轉出「{}」（{}）",
            a.bound_weapon.name,
            evolved.formal_name,
            evolved.main_motions.join("→")
        )
    };
    let msg = format!("你展讀{book_name}反覆比劃呼吸重心：{contrast_note}；書冊仍完好留在行囊。");
    push_breath(a, msg.clone());
    a.mark_mutated();
    Ok(msg)
}

/// 推算敵手物理模型（對實有 NPC / 玩家直接取其真實 `soul_seed` 與器物；對荒野魔物取其棲位骨相）
fn foe_combat_profile(
    delta: &IslandWorldDelta,
    foe_slug: &str,
) -> (i64, CarrierAffinity, f64, u32) {
    if let Some(real_actor) = delta.actors.get(foe_slug) {
        let carrier = tool_carrier_of(&real_actor.bound_weapon.slug, &real_actor.bound_weapon.name);
        let prac = u32::from(real_actor.mind.ltm.mastery.martial_bouts)
            + u32::from(real_actor.bound_weapon.grip_familiarity / 2);
        let power = 62.0
            + if real_actor.bound_weapon.rune_mark.is_some() { 8.0 } else { 0.0 }
            + if super::body_gear_craft::arms_absorbs_wrist_recoil(real_actor) { 4.0 } else { 0.0 };
        return ((real_actor.soul_seed & 0x7FFF_FFFF) as i64, carrier, power, prac);
    }
    if foe_slug == "stone_spider" {
        (9_107_421, CarrierAffinity::Blunt, 74.0, 18)
    } else if foe_slug.starts_with("mutated_boss:") {
        (8_441_903, CarrierAffinity::Blunt, 78.0, 20)
    } else if foe_slug.starts_with("apex:") {
        (7_773_119, CarrierAffinity::Blade, 72.0, 16)
    } else if foe_slug.starts_with("goblin_camp_chief:") {
        (6_331_205, CarrierAffinity::Blade, 68.0, 14)
    } else if foe_slug == "cave_lizard" {
        (5_219_881, CarrierAffinity::Blunt, 66.0, 12)
    } else {
        (4_112_773, CarrierAffinity::Blade, 56.0, 8)
    }
}

/// 連續三段臨陣交鋒演繹（直走 `evaluate_three_phase_bout_ext` 與獨立 `settle_skirmish_victory`，零舊猜拳借殼與設計黑話）
pub fn resolve_auto_tianji_clash(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    plan: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let (sk, node_id, plan_slots, plan_title) = {
        let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
        let Some(sk) = a.active_skirmish.clone() else {
            return Err("眼前平靜無事，並未與敵手交鋒。".to_string());
        };
        let (slots, title) = match plan.trim() {
            "yield_then_strike" => (
                [TacticalSlot::Defense, TacticalSlot::Midgame, TacticalSlot::Finisher],
                "後發制人・守勢卸銳",
            ),
            "seam_harvest" => (
                [TacticalSlot::Midgame, TacticalSlot::Defense, TacticalSlot::Finisher],
                "入微尋縫・避實擊虛",
            ),
            _ => (
                [TacticalSlot::Opening, TacticalSlot::Finisher, TacticalSlot::Midgame],
                "搶勢壓制・正面破架",
            ),
        };
        (sk, a.node_id.clone(), slots, title)
    };

    let (
        a_bone,
        a_carrier,
        a_weapon_name,
        a_has_rune,
        a_learned,
        a_prac,
        a_foes_cnt,
        a_gear_power,
        a_scar,
        arms_shielded,
        feet_steady,
        routines,
    ) = {
        let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
        let bone = actor_bone_phase(a);
        let carrier = tool_carrier_of(&a.bound_weapon.slug, &a.bound_weapon.name);
        let learned = actor_learned_bonus(a);
        let prac = u32::from(a.mind.ltm.mastery.martial_bouts)
            + u32::from(a.bound_weapon.grip_familiarity / 2);
        let foes_cnt = u32::try_from(a.mind.ltm.mastery.distinct_foes.len()).unwrap_or(0);
        let has_rune = a.bound_weapon.rune_mark.is_some();
        let rune_bonus = if has_rune { 8.0 } else { 0.0 };
        let arms_ok = super::body_gear_craft::arms_absorbs_wrist_recoil(a);
        let feet_ok = super::body_gear_craft::feet_fatigue_reduction(a, false) > 0;
        let gear_pow = 62.0
            + rune_bonus
            + if arms_ok { 4.0 } else { 0.0 }
            + if feet_ok { 3.0 } else { 0.0 };
        let scar = if a.wound_part.is_some() { 0.30 } else { 0.0 } + f64::from(a.fatigue) * 0.003;
        let reg = WordRegistry::global();
        let rs: Vec<EmergentRoutine> = plan_slots
            .iter()
            .map(|&s| {
                derive_personal_martial_routine(
                    &bone,
                    s,
                    carrier,
                    &a.bound_weapon.name,
                    actor_learned_bonus_for_slot(a, s),
                    prac,
                    reg,
                )
            })
            .collect();
        (
            bone,
            carrier,
            a.bound_weapon.name.clone(),
            has_rune,
            learned,
            prac,
            foes_cnt,
            gear_pow,
            scar,
            arms_ok,
            feet_ok,
            rs,
        )
    };

    let (foe_seed, foe_carrier, foe_power, foe_prac) = foe_combat_profile(delta, &sk.foe_slug);
    let foe_bone = InnateBonePhase::from_seed(foe_seed);

    let mut post_a = if a_scar >= 0.45 {
        PostureState::Shaken
    } else {
        PostureState::Steady
    };
    let mut post_b = if sk.foe_stagger > 0 {
        PostureState::Shaken
    } else {
        PostureState::Steady
    };

    let mut total_a = 0.0f64;
    let mut total_b = 0.0f64;
    let mut narr_segments: Vec<String> = Vec::new();
    let mut manifested_any_skill = false;
    let mut wound_candidate = "左腕瘀腫";

    for idx in 0..3 {
        let slot = plan_slots[idx];
        let r = &routines[idx];
        let tactic_bonus = match (plan.trim(), idx) {
            ("yield_then_strike", 0) => 5.0,
            ("yield_then_strike", 2) => 4.0,
            ("seam_harvest", 2) => 6.0,
            ("press_burst", 0) => 5.0,
            _ => 1.5,
        };
        let res = evaluate_three_phase_bout_ext(
            slot,
            &a_bone,
            &foe_bone,
            a_gear_power + tactic_bonus,
            foe_power,
            a_carrier,
            foe_carrier,
            post_a,
            post_b,
            5,
            5,
            idx + usize::from(sk.round_idx),
            a_prac,
            foe_prac,
            a_foes_cnt,
            2,
            a_learned,
            0.35,
            a_scar,
            if sk.foe_stagger > 0 { 0.25 } else { 0.0 },
        );
        total_a += res.score_delta_a;
        total_b += res.score_delta_b;
        post_a = res.a_posture_after;
        post_b = res.b_posture_after;

        let motion_lead = &r.main_motions[0];
        let motion_turn = &r.main_motions[2];
        let skill_phrase = if res.a_manifested_as_skill {
            manifested_any_skill = true;
            format!(
                "以「{motion_lead}」起勢接「{motion_turn}」，五常連貫成相顯化【{}】",
                r.formal_name
            )
        } else {
            format!("起手「{motion_lead}」欲連「{motion_turn}」，中途受對手鋒芒壓逼潰散為碎招")
        };
        let verdict_phrase = match res.verdict {
            ThreePhaseVerdict::Phase1ImpulseCrush { winner_is_a: true, .. } => {
                format!("以一力降十會之勢正面壓退【{}】", sk.foe_name)
            }
            ThreePhaseVerdict::Phase1ImpulseCrush { winner_is_a: false, .. } => {
                wound_candidate = "肋側震痛";
                format!("被【{}】憑絕對蠻力硬生生震開半步", sk.foe_name)
            }
            ThreePhaseVerdict::Phase2KinematicIntercept { interceptor_is_a: true, .. } => {
                format!("精準搶入【{}】換氣空檔，當場拆斷對方來勢", sk.foe_name)
            }
            ThreePhaseVerdict::Phase2KinematicIntercept { interceptor_is_a: false, .. } => {
                wound_candidate = "右肩挫傷";
                format!("被【{}】抓到換招空隙搶攻逼退", sk.foe_name)
            }
            ThreePhaseVerdict::Phase3SeamThreading { threader_is_a: true, .. } => {
                "順著關節甲縫游刃切入（庖丁解牛），避開硬殼重創要害".to_string()
            }
            ThreePhaseVerdict::Phase3SeamThreading { threader_is_a: false, .. } => {
                wound_candidate = "小腿拉傷";
                format!("被【{}】刁鑽尋隙擦過護身門戶", sk.foe_name)
            }
            ThreePhaseVerdict::StandardExchange => {
                if res.score_delta_a >= res.score_delta_b {
                    format!("與【{}】短兵交擊互不相讓，略佔一線主動", sk.foe_name)
                } else {
                    format!("與【{}】硬拚一記，掌中【{a_weapon_name}】微微發麻", sk.foe_name)
                }
            }
        };
        narr_segments.push(format!("你{skill_phrase}，{verdict_phrase}。"));
    }

    let gear_tail = if arms_shielded && feet_steady {
        "雙臂護具卸震、足下靴履穩步，呼吸並未大亂。"
    } else if arms_shielded {
        "雙臂護具替你卸去大半反震力道，護住手腕未傷。"
    } else if feet_steady {
        "腳下涉水靴抓地穩實，騰挪換氣省了不少氣力。"
    } else {
        ""
    };
    let auto_prose = format!("〔{}〕{}{}", plan_title, narr_segments.join(""), gear_tail);

    let is_tough_boss = matches!(sk.foe_slug.as_str(), "stone_spider" | "cave_lizard")
        || sk.foe_slug.starts_with("apex:")
        || sk.foe_slug.starts_with("goblin_camp_chief:")
        || sk.foe_slug.starts_with("mutated_boss:");

    // 勝出：直接呼叫獨立 `settle_skirmish_victory`，絕不借殼舊猜拳 `skirmish_wrist`！
    if total_a + 1.5 >= total_b && (!is_tough_boss || sk.foe_stagger >= 1 || manifested_any_skill) {
        if let Some(a) = delta.actors.get_mut(&aid) {
            push_breath(a, auto_prose.clone());
        }
        let settle_msg = super::explore_and_combat::settle_skirmish_victory(
            doc, delta, clock, &aid, &node_id, &sk, a_has_rune, false,
        )?;
        return Ok(format!("{auto_prose}\n{settle_msg}"));
    }

    let critical_state = delta.actors.get(&aid).is_some_and(|a| {
        (a.wound_part.is_some() && a.fatigue >= 72) || (total_b > total_a + 12.0 && a.fatigue >= 80)
    });
    if critical_state {
        if let Some(a) = delta.actors.get_mut(&aid) {
            push_breath(a, auto_prose.clone());
        }
        let def_msg = super::explore_and_combat::trigger_non_lethal_defeat(
            doc,
            delta,
            clock,
            &aid,
            &node_id,
            &sk.foe_slug,
            &sk.foe_name,
        )?;
        return Ok(format!("{auto_prose}\n{def_msg}"));
    }

    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    let fat_add: u8 = if arms_shielded { 7 } else { 11 };
    a.fatigue = (a.fatigue + fat_add).min(95);
    if is_tough_boss && !arms_shielded && total_b > total_a {
        a.wound_part = Some(wound_candidate.to_string());
    }
    if let Some(ref mut s) = a.active_skirmish {
        s.foe_stagger = s.foe_stagger.saturating_add(1);
        s.player_guard = plan.trim() == "yield_then_strike";
    }
    let pause_msg = format!(
        "{auto_prose} 雙方急驟交錯後各自退開兩步，拉開換氣對峙空檔——【{}】氣息已顯浮動，正是變換步調或抽身退走的時機。",
        sk.foe_name
    );
    push_breath(a, pause_msg.clone());
    a.mark_mutated();
    Ok(pause_msg)
}
