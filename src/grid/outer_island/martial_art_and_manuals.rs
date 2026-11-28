//! 第三刀核心器官：個人之相（4 武相＋4 藝相自動自然生成）× 《功法手札》著譜傳授 × 全自動田忌賽馬連續文字交鋒。
//!
//! 嚴守 `DESIGN_KEYWORDS.md` L133-135 / L167 / L183-184 / L190-206：
//! 1. 後台直用 `crate::combat::human_phase` 五大數理模組（先天骨相雙層守恆、Viterbi 同法不同相、
//!    連貫性×變化性自然成相、一力降十會／專克拆招／庖丁解牛三相判準、2-Step Residual OMP 幕後定名）；
//! 2. 前台絕不外露回合數與數值加減，只呈現真實碎招起承轉合、成相顯化／潰散與換氣對峙空檔；
//! 3. 支援將個人武相著譜為實體《功法手札》供交易與傳授，他人研讀後仍依自身骨相演化出「同法不同相」的個人套路。

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
    let manual_boost = (a.mind.ltm.mastery.studied_manuals.len() as f64) * 0.14;
    let base = (f64::from(prac) * 0.04 + manual_boost).min(0.90);
    if a.mind.ltm.mastery.private_martial_phase.is_some() {
        base.max(0.55)
    } else {
        base
    }
}

/// 自然生成角色背板「4 臨陣武相」與「4 百工藝相」（完全由骨相×所學×所熟×手持器物決定，零手動自訂）
pub fn derive_actor_eight_phases(
    a: &UnifiedMindBody,
) -> (Vec<EmergentRoutine>, Vec<EmergentRoutine>) {
    let bone = actor_bone_phase(a);
    let carrier = tool_carrier_of(&a.bound_weapon.slug, &a.bound_weapon.name);
    let vessel = a.bound_weapon.name.as_str();
    let learned = actor_learned_bonus(a);
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
            derive_personal_martial_routine(&bone, slot, carrier, vessel, learned, martial_fam, reg)
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

/// 將自身一式武相著譜成實體《功法手札》（物理守恆物件，可供交易、贈與或留存傳徒）
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

    let (node_id, author_name, routine) = {
        let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
        if a.fatigue >= 85 {
            return Err("此刻氣力已近枯竭，無力靜心凝神著譜，先歇息緩氣再動筆吧。".to_string());
        }
        let day_key = format!("manual_write:d{}", clock.game_days);
        if a.explored_windows.contains(&day_key) {
            return Err("今日已耗神撰寫過一冊功法手札，明日靜氣後再續寫新卷。".to_string());
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
            actor_learned_bonus(a),
            prac,
            WordRegistry::global(),
        );
        (a.node_id.clone(), a.display_name.clone(), r)
    };

    let book_name = format!("《{}・手札》", routine.formal_name);
    let slug = {
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        a.mind.ltm.mastery.vessel_serial = a.mind.ltm.mastery.vessel_serial.saturating_add(1);
        let s = format!("manual_{}_{}", slot_code.trim(), a.mind.ltm.mastery.vessel_serial);
        let day_key = format!("manual_write:d{}", clock.game_days);
        a.explored_windows.insert(day_key);
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: s.clone(),
                name: book_name.clone(),
                count: 1,
                weight_units: 1,
                ..CarryItem::default()
            },
        );
        a.fatigue = (a.fatigue + 10).min(95);
        a.last_active_unix = clock.real_unix;
        let msg = format!(
            "你靜心回味掌中發力與起承轉合，將【{}】（{}）寫成實體{}收入行囊，可留贈同道、傳徒或於市集議價流轉。",
            routine.formal_name,
            routine.main_motions.join("→"),
            book_name
        );
        push_breath(a, msg);
        a.mark_mutated();
        s
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
    let _ = slug;
    Ok(format!(
        "你已著成實體{}收入行囊（五常碎招：{}），他人研讀後亦會依自身骨相走出同法不同相的新路。",
        book_name,
        routine.main_motions.join(" → ")
    ))
}

/// 研讀行囊內之實體《功法手札》（不銷毀書冊，提升個人所學並依自身骨相走出「同法不同相」！）
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
    let book_name = a
        .backpack
        .iter()
        .find(|i| i.slug == slug && i.count > 0 && i.slug.starts_with("manual_"))
        .map(|i| i.name.clone())
        .ok_or("行囊裡找不到這冊功法手札。")?;
    if a.mind.ltm.mastery.studied_manuals.contains(&book_name) {
        return Ok(format!(
            "你重新翻閱了一遍{book_name}，其中的起承轉合要訣早已融進你的身法裡；此冊仍完好留在行囊，可轉贈或售予他人。"
        ));
    }
    if a.mind.ltm.mastery.studied_manuals.len() < 8 {
        a.mind.ltm.mastery.studied_manuals.push(book_name.clone());
    }
    a.bound_weapon.grip_familiarity = a.bound_weapon.grip_familiarity.saturating_add(4);
    a.fatigue = (a.fatigue + 6).min(95);
    a.last_active_unix = clock.real_unix;

    // 依本人骨相立即重算演化後的個人連攜武相，展現「同法不同相」
    let bone = actor_bone_phase(a);
    let carrier = tool_carrier_of(&a.bound_weapon.slug, &a.bound_weapon.name);
    let prac = u32::from(a.mind.ltm.mastery.martial_bouts)
        + u32::from(a.bound_weapon.grip_familiarity / 2);
    let evolved = derive_personal_martial_routine(
        &bone,
        TacticalSlot::Midgame,
        carrier,
        &a.bound_weapon.name,
        actor_learned_bonus(a),
        prac,
        WordRegistry::global(),
    );
    let msg = format!(
        "你依著{book_name}反覆比劃呼吸與重心，順著自身骨相自然代償消化，掌中【{}】漸漸流轉出屬於你自己的「{}」（{}）；書冊仍完好留在行囊。",
        a.bound_weapon.name,
        evolved.formal_name,
        evolved.main_motions.join("→")
    );
    push_breath(a, msg.clone());
    a.mark_mutated();
    Ok(msg)
}

/// 推算敵手物理模型（骨相種子、載體、硬度與起手傾向，供 `evaluate_three_phase_bout_ext` 統一演算）
fn foe_combat_profile(foe_slug: &str) -> (i64, CarrierAffinity, f64, u32) {
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
        // grey_ear_goblin / squatter / bandit
        (4_112_773, CarrierAffinity::Blade, 56.0, 8)
    }
}

/// 全自動田忌賽馬三段連續演繹（單次下令 → 連續演算起手／中盤／決勝三相物理，直抵勝負或換氣對峙空檔！）
///
/// `plan` 三策：
/// - `"yield_then_strike"`（下馬兌上馬・後發制人）：守勢 → 連攜 → 破綻
/// - `"press_burst"`（上馬壓中馬・一力降十會）：起手 → 破綻 → 連攜
/// - `"seam_harvest"`（中馬尋細縫・庖丁解牛）：連攜 → 守勢 → 破綻
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
                "後發制人（守勢卸銳→中盤截隙→破綻定局）",
            ),
            "seam_harvest" => (
                [TacticalSlot::Midgame, TacticalSlot::Defense, TacticalSlot::Finisher],
                "入微尋縫（周旋誘隙→卸力貼近→穿縫破甲）",
            ),
            _ => (
                [TacticalSlot::Opening, TacticalSlot::Finisher, TacticalSlot::Midgame],
                "搶勢壓制（起手先聲→重勢壓架→連攜收勢）",
            ),
        };
        (sk, a.node_id.clone(), slots, title)
    };

    let (
        a_bone,
        a_carrier,
        a_weapon_name,
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
        let rune_bonus = if a.bound_weapon.rune_mark.is_some() { 8.0 } else { 0.0 };
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
                    learned,
                    prac,
                    reg,
                )
            })
            .collect();
        (
            bone,
            carrier,
            a.bound_weapon.name.clone(),
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

    let (foe_seed, foe_carrier, foe_power, foe_prac) = foe_combat_profile(&sk.foe_slug);
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

    for idx in 0..3 {
        let slot = plan_slots[idx];
        let r = &routines[idx];
        // 田忌賽馬時序加成：後發制人首段穩守、入微尋縫末段穿隙、搶勢壓制首段搶攻
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
            format!(
                "起手「{motion_lead}」欲連「{motion_turn}」，中途受對手鋒芒壓逼潰散為碎招"
            )
        };
        let verdict_phrase = match res.verdict {
            ThreePhaseVerdict::Phase1ImpulseCrush { winner_is_a: true, .. } => {
                format!("以一力降十會之勢正面壓退【{}】", sk.foe_name)
            }
            ThreePhaseVerdict::Phase1ImpulseCrush { winner_is_a: false, .. } => {
                format!("被【{}】憑絕對蠻力硬生生震開半步", sk.foe_name)
            }
            ThreePhaseVerdict::Phase2KinematicIntercept { interceptor_is_a: true, .. } => {
                format!("精準搶入【{}】換氣空檔，當場拆斷對方後續來勢", sk.foe_name)
            }
            ThreePhaseVerdict::Phase2KinematicIntercept { interceptor_is_a: false, .. } => {
                format!("被【{}】抓到換招空隙搶攻逼退", sk.foe_name)
            }
            ThreePhaseVerdict::Phase3SeamThreading { threader_is_a: true, .. } => {
                format!("順著關節甲縫游刃切入（庖丁解牛），避開硬殼重創要害")
            }
            ThreePhaseVerdict::Phase3SeamThreading { threader_is_a: false, .. } => {
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
    let auto_prose = format!(
        "〔{}〕{}{}",
        plan_title,
        narr_segments.join(""),
        gear_tail
    );

    let is_tough_boss = matches!(sk.foe_slug.as_str(), "stone_spider" | "cave_lizard")
        || sk.foe_slug.starts_with("apex:")
        || sk.foe_slug.starts_with("goblin_camp_chief:")
        || sk.foe_slug.starts_with("mutated_boss:");

    // 結算勝負或拉開換氣對峙空檔：完全複用現有守恆／戰利品／成相／落敗管線！
    if total_a + 1.5 >= total_b && (!is_tough_boss || sk.foe_stagger >= 1 || manifested_any_skill) {
        if let Some(a) = delta.actors.get_mut(&aid) {
            push_breath(a, auto_prose.clone());
            if let Some(ref mut s) = a.active_skirmish {
                s.foe_stagger = s.foe_stagger.max(2);
                s.player_guard = true;
            }
        }
        let settle_msg =
            super::explore_and_combat::resolve_skirmish_turn(doc, delta, clock, &aid, "skirmish_wrist")?;
        return Ok(format!("{auto_prose}\n{settle_msg}"));
    }

    // 明顯落於下風且已重傷／力竭：觸發真實非致死落敗救回閉環
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

    // 雙方拉開「換氣對峙空檔」：更新重心破綻與疲勞，讓玩家做關鍵戰術轉折（零回合數、零勞作連點！）
    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    let fat_add: u8 = if arms_shielded { 7 } else { 11 };
    a.fatigue = (a.fatigue + fat_add).min(95);
    if is_tough_boss && !arms_shielded && total_b > total_a {
        a.wound_part = Some("左腕瘀腫".to_string());
    }
    if let Some(ref mut s) = a.active_skirmish {
        s.foe_stagger = s.foe_stagger.saturating_add(1);
        s.player_guard = plan.trim() == "yield_then_strike";
    }
    let pause_msg = format!(
        "{auto_prose} 雙方一陣疾風驟雨般交錯後各自退開兩步，進入換氣對峙空檔——【{}】氣息已顯浮動，正是調整田忌策或抽身退走的時機。",
        sk.foe_name
    );
    push_breath(a, pause_msg.clone());
    a.mark_mutated();
    Ok(pause_msg)
}
