//! 快思維（System 1）個人化處境感知與 10 路行動效用仲裁器（$O(1)$ 預建索引、打破 `99.15% walk_work` 單調迴圈）。

use std::collections::{BTreeMap, BTreeSet};

use crate::combat::wordplate::splitmix64;

use super::appraisal::{ConcernCause, GoalStrategyKind};
use super::household_economy::{count_box_home_rations_only, household_bucket_key};
use super::persona::{InnatePersona, ResponseTendency, Situation16D, ValueAxis};
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

#[derive(Debug, Clone)]
pub struct DesireDecision {
    pub kind: &'static str,
    pub target_node: String,
    pub focus_actor: Option<String>,
    pub reason_zh: String,
}

/// 每響鐘預建一次之 $O(N)$ 全島家戶存糧與活躍債契查詢表（L4 修正：單人決策嚴守 $O(1)$）
pub struct BellDecisionIndex {
    pub hh_larders: BTreeMap<String, u32>,
    pub debtor_to_creditor: BTreeMap<String, (String, bool)>, // debtor -> (creditor, can_plead_ext)
    pub creditor_to_overdue_debtor: BTreeMap<String, String>,
    pub debtors_with_unpaid_record: BTreeSet<String>,
}

impl BellDecisionIndex {
    pub fn build(delta: &IslandWorldDelta) -> Self {
        let hh_larders = super::household_economy::build_household_larder_index(delta);
        let mut debtor_to_creditor = BTreeMap::new();
        let mut creditor_to_overdue_debtor = BTreeMap::new();
        let mut debtors_with_unpaid_record = BTreeSet::new();
        for c in delta.contracts.values() {
            if c.status == "open" || c.status == "defaulted" || c.status == "written_off" {
                debtors_with_unpaid_record.insert(c.debtor_id.clone());
            }
            if c.status != "open" && c.status != "defaulted" {
                continue;
            }
            let can_ext = !c.note_zh.contains("〔已寬限〕")
                && !c.note_zh.contains("〔拒寬限〕")
                && !c.creditor_id.starts_with("pl:")
                && delta
                    .actors
                    .get(&c.creditor_id)
                    .is_some_and(|cr| cr.controller_id.is_none());
            debtor_to_creditor.insert(c.debtor_id.clone(), (c.creditor_id.clone(), can_ext));
            if c.status == "defaulted" {
                creditor_to_overdue_debtor.insert(c.creditor_id.clone(), c.debtor_id.clone());
            }
        }
        Self {
            hh_larders,
            debtor_to_creditor,
            creditor_to_overdue_debtor,
            debtors_with_unpaid_record,
        }
    }

    pub fn larder_of(&self, a: &UnifiedMindBody) -> u32 {
        self.hh_larders
            .get(&household_bucket_key(a))
            .copied()
            .unwrap_or(0)
    }
}

pub fn build_live_situation_16d(
    delta: &IslandWorldDelta,
    idx: &BellDecisionIndex,
    a: &UnifiedMindBody,
    larder_count: u32,
    bell_index: u8,
    game_day: i64,
) -> (
    Situation16D,
    Option<(String, bool)>,
    Option<String>,
    Option<(String, bool)>,
    Option<String>,
) {
    let mut co_rival: Option<(i8, String, bool)> = None;
    let mut co_warm: Option<(i8, String)> = None;
    let at_own_home =
        a.node_id == a.home_node || a.owned_house_node.as_deref() == Some(a.node_id.as_str());

    if let Some(occ) = delta.node_occupants.get(&a.node_id) {
        for oid in occ {
            if oid == &a.actor_id {
                continue;
            }
            if let Some(soc) = a.mind.ltm.social.get(oid) {
                if soc.warmth <= -22 {
                    // R2-H1 + R3-H1 + R3-L6 修正：統一嫌隙門檻 `<= -22`，排除同屋家人日常同行（以冷淡體態呈現而非街頭追繞活鎖），並對街頭理論設 4 日雙向冷卻！
                    let same_hh = soc.cohabiting
                        || (!a.household_id.is_empty()
                            && delta
                                .actors
                                .get(oid)
                                .is_some_and(|o| o.household_id == a.household_id));
                    if at_own_home || same_hh {
                        continue;
                    }
                    let shared_work = a.node_id == a.work_node
                        && delta
                            .actors
                            .get(oid)
                            .is_some_and(|o| o.work_node == a.work_node);
                    let cur_stamp = (game_day + 10_000).max(1);
                    let self_cd_ok = a.mind.stm.last_self_confront_stamp == 0
                        || cur_stamp >= a.mind.stm.last_self_confront_stamp + 4;
                    let pair_cd_ok =
                        soc.last_confront_day == 0 || cur_stamp >= soc.last_confront_day + 4;
                    let can_confront = self_cd_ok && pair_cd_ok;
                    if !can_confront && shared_work {
                        continue;
                    }
                    if co_rival.as_ref().is_none_or(|(w, _, _)| soc.warmth < *w) {
                        co_rival = Some((soc.warmth, oid.clone(), can_confront));
                    }
                } else if soc.warmth >= 35 && co_warm.as_ref().is_none_or(|(w, _)| soc.warmth > *w)
                {
                    co_warm = Some((soc.warmth, oid.clone()));
                }
            }
        }
    }

    let owed_creditor = idx.debtor_to_creditor.get(&a.actor_id).cloned();
    let overdue_debtor_id = idx.creditor_to_overdue_debtor.get(&a.actor_id).cloned();

    let hunger = i32::from(a.mind.stm.affect.hunger);
    let fatigue = i32::from(a.mind.stm.affect.fatigue.max(a.fatigue));
    let max_concern = a
        .mind
        .top_concern()
        .map_or(0i32, |c| i32::from(c.intensity));
    let has_betrayal = a
        .mind
        .stm
        .focus_concerns
        .iter()
        .flatten()
        .any(|c| c.cause == ConcernCause::PactBetrayed);
    let has_curiosity = a.mind.stm.focus_concerns.iter().flatten().any(|c| {
        c.axis == ValueAxis::CuriosityVenture || c.cause == ConcernCause::CrossTownYearning
    });

    let threat = if co_rival.is_some() {
        38
    } else if owed_creditor.is_some() && a.copper < 3 {
        28
    } else {
        0
    };
    let power_gap = if a.copper < 4 {
        -20
    } else if a.copper > 120 {
        20
    } else {
        0
    };
    let familiarity = if co_warm.is_some() { 55 } else { 20 };
    let grudge_favor = if let Some((w, _, _)) = co_rival {
        i32::from(w) - 15
    } else if let Some((w, _)) = co_warm {
        i32::from(w)
    } else if overdue_debtor_id.is_some() {
        -35
    } else {
        0
    };
    let scarcity = if larder_count == 0 {
        65
    } else if larder_count < 3 {
        30
    } else {
        (hunger - 40).max(0)
    };
    let gain_loss = if a.copper < 4 || owed_creditor.is_some() {
        -32
    } else {
        12
    };
    let urgency = fatigue.max(hunger) - 30 + if owed_creditor.is_some() { 25 } else { 0 };
    let publicity_face = if co_rival.is_some() || owed_creditor.is_some() {
        42
    } else {
        10
    };
    let rule_clarity = if overdue_debtor_id.is_some() {
        45
    } else if bell_index <= 7 {
        20
    } else {
        5
    };
    let deception_cue = if has_betrayal { 45 } else { 0 };
    let weak_present = if larder_count == 0 && !a.household_id.is_empty() {
        35
    } else {
        0
    };
    let novelty = if has_curiosity { 38 } else { 5 };
    let self_strain = max_concern.max(hunger).max(fatigue);
    let kin_group = if co_warm.is_some() || !a.household_id.is_empty() {
        30
    } else {
        0
    };
    let stake_scale = if owed_creditor.is_some() || larder_count == 0 {
        40
    } else {
        12
    };

    let sit = Situation16D::new(
        threat,
        power_gap,
        familiarity,
        grudge_favor,
        gain_loss,
        scarcity,
        urgency,
        publicity_face,
        rule_clarity,
        deception_cue,
        weak_present,
        novelty,
        self_strain,
        kin_group,
        stake_scale,
        20,
    );
    (
        sit,
        co_rival.map(|(_, id, can_conf)| (id, can_conf)),
        co_warm.map(|(_, id)| id),
        owed_creditor,
        overdue_debtor_id,
    )
}

/// L1 + R3-H1 修正：繞行目標必須在本人 `known_nodes` 內，優先直赴自身當值處並以個人種子打散出口、避開對手去向，徹底根除雙人結伴繞行活鎖！
fn find_known_detour_node(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
    rival_id: &str,
    abs_bell: i64,
) -> Option<String> {
    let rival_next = delta.actors.get(rival_id).and_then(|r| {
        r.route_plan
            .front()
            .map(String::as_str)
            .or(Some(r.work_node.as_str()))
    });
    if a.work_node != a.node_id
        && a.mind.ltm.known_nodes.contains(&a.work_node)
        && rival_next != Some(a.work_node.as_str())
    {
        return Some(a.work_node.clone());
    }
    let cur = doc.nodes.get(&a.node_id)?;
    let valid_exits: Vec<&str> = cur
        .exits
        .iter()
        .filter_map(|ex| {
            (ex.to != a.node_id
                && rival_next != Some(ex.to.as_str())
                && a.mind.ltm.known_nodes.contains(&ex.to)
                && doc
                    .nodes
                    .get(&ex.to)
                    .is_some_and(|n| !n.dungeon && n.kind != "house_vacant"))
            .then_some(ex.to.as_str())
        })
        .collect();
    if !valid_exits.is_empty() {
        let pick = (splitmix64(a.soul_seed ^ abs_bell.unsigned_abs()) as usize) % valid_exits.len();
        return Some(valid_exits[pick].to_string());
    }
    if a.home_node != a.node_id && a.mind.ltm.known_nodes.contains(&a.home_node) {
        Some(a.home_node.clone())
    } else if a.work_node != a.node_id && a.mind.ltm.known_nodes.contains(&a.work_node) {
        Some(a.work_node.clone())
    } else {
        None
    }
}

pub(super) fn find_known_wild_forage_node(
    doc: &WorldStaticDoc,
    a: &UnifiedMindBody,
) -> Option<String> {
    let home_cell = doc.nodes.get(&a.home_node).map_or(4616, |n| n.cell_id);
    for nid in &a.mind.ltm.known_nodes {
        if let Some(nd) = doc.nodes.get(nid)
            && nd.cell_id == home_cell
            && matches!(nd.kind.as_str(), "gate" | "village_gate" | "wild" | "road")
        {
            if matches!(nd.kind.as_str(), "wild" | "road") && !nd.dungeon {
                return Some(nid.clone());
            }
            for ex in &nd.exits {
                if doc
                    .nodes
                    .get(&ex.to)
                    .is_some_and(|dst| matches!(dst.kind.as_str(), "wild" | "road") && !dst.dungeon)
                {
                    return Some(ex.to.clone());
                }
            }
        }
    }
    a.mind
        .ltm
        .known_nodes
        .iter()
        .find(|nid| {
            doc.nodes
                .get(*nid)
                .is_some_and(|n| matches!(n.kind.as_str(), "wild" | "road") && !n.dungeon)
        })
        .cloned()
}

pub(super) fn find_confidant_in_known_nodes(
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
) -> Option<(String, String, String)> {
    a.mind
        .ltm
        .social
        .values()
        .filter(|s| !s.target_id.starts_with("pl:") && s.warmth >= 32 && s.target_id != a.actor_id)
        .max_by_key(|s| (s.warmth, s.familiarity))
        .and_then(|best| {
            let friend = delta.actors.get(&best.target_id)?;
            if !friend.alive || friend.controller_id.is_some() {
                return None;
            }
            let dst = if a.mind.ltm.known_nodes.contains(&friend.node_id) {
                friend.node_id.clone()
            } else if a.mind.ltm.known_nodes.contains(&friend.work_node) {
                friend.work_node.clone()
            } else if a.mind.ltm.known_nodes.contains(&friend.home_node) {
                friend.home_node.clone()
            } else {
                return None;
            };
            Some((best.target_id.clone(), friend.display_name.clone(), dst))
        })
}

fn find_confidant_scoped(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
    same_cell_only: bool,
) -> Option<(String, String, String)> {
    if !same_cell_only {
        return find_confidant_in_known_nodes(delta, a);
    }
    let my_cell = doc.nodes.get(&a.home_node).map_or(4616, |n| n.cell_id);
    a.mind
        .ltm
        .social
        .values()
        .filter(|s| !s.target_id.starts_with("pl:") && s.warmth >= 32 && s.target_id != a.actor_id)
        .filter(|s| {
            delta
                .actors
                .get(&s.target_id)
                .and_then(|f| doc.nodes.get(&f.home_node))
                .is_some_and(|n| n.cell_id == my_cell)
        })
        .max_by_key(|s| (s.warmth, s.familiarity))
        .and_then(|best| {
            let friend = delta.actors.get(&best.target_id)?;
            if !friend.alive || friend.controller_id.is_some() {
                return None;
            }
            let dst = if a.mind.ltm.known_nodes.contains(&friend.node_id) {
                friend.node_id.clone()
            } else if a.mind.ltm.known_nodes.contains(&friend.work_node) {
                friend.work_node.clone()
            } else if a.mind.ltm.known_nodes.contains(&friend.home_node) {
                friend.home_node.clone()
            } else {
                return None;
            };
            Some((best.target_id.clone(), friend.display_name.clone(), dst))
        })
}

#[expect(
    clippy::too_many_arguments,
    reason = "O(1) pre-indexed fast-system utility evaluation"
)]
pub fn evaluate_personalized_desire(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    idx: &BellDecisionIndex,
    a: &UnifiedMindBody,
    innate: &InnatePersona,
    tav_opt: Option<&str>,
    keeper_opt: Option<&str>,
    bell_index: u8,
    abs_bell: i64,
) -> DesireDecision {
    let larder_count = idx.larder_of(a);
    let game_day = abs_bell.div_euclid(12);
    let (sit, co_rival_info, _co_warm_id, owed_creditor, overdue_debtor_id) =
        build_live_situation_16d(delta, idx, a, larder_count, bell_index, game_day);
    let tend = innate.choose_dominant_tendency(&sit, &a.drift);
    let top_val = innate.top_value_axis(&a.drift);
    let strat_kind = a.mind.stm.goal_strategy.as_ref().map(|g| g.kind);
    let last_kind = a.mind.stm.intentions.back().map(|i| i.kind.as_str());

    let tavern_stock = match (tav_opt, keeper_opt) {
        (Some(tn), Some(kid)) => delta
            .actors
            .get(kid)
            .map_or(0, |k| count_box_home_rations_only(k, tn)),
        _ => 0,
    };

    let hunger = i32::from(a.mind.stm.affect.hunger);
    let fatigue = i32::from(a.mind.stm.affect.fatigue.max(a.fatigue));
    let fear = i32::from(a.mind.stm.affect.fear);
    let anger = i32::from(a.mind.stm.affect.anger);
    let grief = i32::from(a.mind.stm.affect.grief);

    // 師徒與櫃位自然錯班彈性權重 ＋ 店鋪斷料主動外出尋料（撤除強制死鎖櫃台保母鎖）
    let (work_duty_adj, forage_shortage_adj) =
        super::live_agency::compute_post_duty_and_shortage_adjustment(
            doc,
            delta,
            a,
            bell_index,
            tavern_stock,
        );

    let bed_node = a
        .owned_house_node
        .clone()
        .unwrap_or_else(|| a.home_node.clone());

    let jitter = |slot: u64| -> i32 {
        let h = splitmix64(a.soul_seed ^ (abs_bell.unsigned_abs().wrapping_mul(131)) ^ slot);
        ((h % 13) as i32) - 6
    };
    let hysteresis = |k: &str| -> i32 {
        if last_kind == Some(k) {
            8 + i32::from(innate.temperament.persistence).max(0) / 8
        } else {
            0
        }
    };

    let mut candidates: Vec<(i32, DesireDecision)> = Vec::with_capacity(10);

    // 1. 本業當值 (`walk_work`)
    let mut work_sc = if (4..=7).contains(&bell_index) {
        54
    } else {
        28
    } + i32::from(innate.temperament.persistence) / 4
        + if matches!(
            top_val,
            ValueAxis::CraftPride | ValueAxis::TangibleGain | ValueAxis::OrderHierarchy
        ) {
            10
        } else {
            0
        }
        + if tend == ResponseTendency::ComplyOrder {
            10
        } else {
            0
        }
        + if strat_kind == Some(GoalStrategyKind::ClearDebt) {
            18
        } else {
            0
        }
        + hysteresis("walk_work")
        + work_duty_adj
        + jitter(1);
    if a.role == "酒館掌櫃" || a.role == "車行掌櫃" {
        work_sc += 22;
    }
    candidates.push((
        work_sc,
        DesireDecision {
            kind: "walk_work",
            target_node: a.work_node.clone(),
            focus_actor: None,
            reason_zh: "前往當值處操持本業營生".to_string(),
        },
    ));

    // 2. 返家開伙用飯／歇息 (`home_meal_rest`)
    if (hunger >= 48 && larder_count > 0) || fatigue >= 68 {
        let sc = hunger
            + fatigue / 2
            + 12
            + if bell_index == 3 || bell_index >= 8 {
                18
            } else {
                0
            }
            + hysteresis("home_meal_rest")
            + jitter(2);
        candidates.push((
            sc,
            DesireDecision {
                kind: "home_meal_rest",
                target_node: bed_node.clone(),
                focus_actor: None,
                reason_zh: "返家開伙吃口家常熱飯歇口氣".to_string(),
            },
        ));
    }

    // 3. 往酒館或本村集場買熱食 (`buy_meal`)：酒館確有存糧且目標節點在 `known_nodes` 內
    let rural_yard_opt = if a.household_id.starts_with("hh:burg_") {
        let cand = format!(
            "oi:{}:yard",
            super::world_weave_bio::settlement_slug_of(&a.household_id)
        );
        doc.nodes.contains_key(&cand).then_some(cand)
    } else {
        None
    };
    if let Some(tn) = tav_opt {
        let meal_dst = rural_yard_opt.as_deref().unwrap_or(tn);
        if tavern_stock >= 1
            && a.mind.ltm.known_nodes.contains(meal_dst)
            && hunger >= 54
            && (larder_count == 0 || a.copper >= 18)
        {
            let sc = hunger * 2 - 36 + if larder_count == 0 { 24 } else { 0 } + jitter(3);
            candidates.push((
                sc,
                DesireDecision {
                    kind: "buy_meal",
                    target_node: meal_dst.to_string(),
                    focus_actor: None,
                    reason_zh: "腹中飢餓往市集買份熱食果腹".to_string(),
                },
            ));
        }
    }

    // 4. 添購家常存糧 (`restock_larder`，家裡斷糧且無未清舊欠時亦得赴鋪賒購應急)
    if let Some(tn) = tav_opt {
        let restock_dst = rural_yard_opt.as_deref().unwrap_or(tn);
        if a.mind.ltm.known_nodes.contains(restock_dst)
            && larder_count < 3
            && (a.copper >= 2 || (larder_count == 0 && owed_creditor.is_none()))
        {
            let sc = 58
                + (3 - larder_count as i32) * 12
                + if top_val == ValueAxis::SurvivalSafety || top_val == ValueAxis::KinshipLoyalty {
                    10
                } else {
                    0
                }
                + jitter(4);
            candidates.push((
                sc,
                DesireDecision {
                    kind: "restock_larder",
                    target_node: restock_dst.to_string(),
                    focus_actor: None,
                    reason_zh: "見家裡米缸漸淺，赴集場酒館添置口糧".to_string(),
                },
            ));
        }
    }

    // 5. 同場撞見嫌隙仇親（M4 防外洩玩家真名；R2-H1：正面理論嚴守 3 日冷卻剎車，冷卻期或避忌性格改繞行）
    if let Some((ref r_id, can_confront)) = co_rival_info {
        let r_label = delta.actors.get(r_id).map_or("嫌隙對象", |x| {
            if x.controller_id.is_some() || r_id.starts_with("pl:") {
                "外來旅人"
            } else {
                x.display_name.as_str()
            }
        });
        let prefers_confront = can_confront
            && (tend == ResponseTendency::Confront
                || (innate.temperament.approach_avoid >= 8 && anger >= fear));
        if !prefers_confront || strat_kind == Some(GoalStrategyKind::AvoidRift) {
            let cur_detour_stamp = (abs_bell + 100_000).max(1);
            let detour_cd_ok = a.mind.stm.last_detour_bell == 0
                || cur_detour_stamp >= a.mind.stm.last_detour_bell + 4;
            let shared_workplace = a.node_id == a.work_node
                && delta
                    .actors
                    .get(r_id)
                    .is_some_and(|r| r.work_node == a.work_node);
            if detour_cd_ok
                && !shared_workplace
                && let Some(detour_nid) = find_known_detour_node(doc, delta, a, r_id, abs_bell)
            {
                let sc = 96 + fear / 2 + jitter(5);
                candidates.push((
                    sc,
                    DesireDecision {
                        kind: "detour_avoid",
                        target_node: detour_nid,
                        focus_actor: Some(r_id.clone()),
                        reason_zh: format!("撞見心存嫌隙的{}，刻意別過臉繞往別處", r_label),
                    },
                ));
            }
        } else {
            let sc = 94 + anger / 2 + jitter(6);
            candidates.push((
                sc,
                DesireDecision {
                    kind: "confront_chase",
                    target_node: a.node_id.clone(),
                    focus_actor: Some(r_id.clone()),
                    reason_zh: format!("撞見素有嫌隙的{}，沉著臉上前討個說法", r_label),
                },
            ));
        }
    }

    // 6. 債主上門催討逾期帳款 (`confront_chase`，R3-L3：共用 4 日雙向冷卻剎車防每響連追)
    let cur_conf_stamp = (game_day + 10_000).max(1);
    let creditor_cd_ok = a.mind.stm.last_self_confront_stamp == 0
        || cur_conf_stamp >= a.mind.stm.last_self_confront_stamp + 4;
    if creditor_cd_ok
        && let Some(ref deb_id) = overdue_debtor_id
        && a.mind
            .ltm
            .social
            .get(deb_id)
            .is_none_or(|s| s.last_confront_day == 0 || cur_conf_stamp >= s.last_confront_day + 4)
        && let Some(deb) = delta.actors.get(deb_id)
        && a.mind.ltm.known_nodes.contains(&deb.node_id)
    {
        let deb_label = if deb.controller_id.is_some() || deb_id.starts_with("pl:") {
            "欠帳旅人"
        } else {
            deb.display_name.as_str()
        };
        let sc = 64
            + anger / 2
            + if matches!(tend, ResponseTendency::Confront | ResponseTendency::Bargain) {
                18
            } else {
                0
            }
            + jitter(7);
        candidates.push((
            sc,
            DesireDecision {
                kind: "confront_chase",
                target_node: deb.node_id.clone(),
                focus_actor: Some(deb_id.clone()),
                reason_zh: format!("尋{}當面核對催討逾期欠帳", deb_label),
            },
        ));
    }

    // 7. 欠戶登門向非玩家債主懇請寬限展期 (`plead_extension`，M8 修正：已寬限過不再重試）
    if let Some((ref cred_id, true)) = owed_creditor
        && a.copper < 3
        && let Some(cred) = delta.actors.get(cred_id)
        && a.mind.ltm.known_nodes.contains(&cred.work_node)
        && matches!(
            tend,
            ResponseTendency::Bargain | ResponseTendency::ComplyOrder | ResponseTendency::SeekAlly
        )
    {
        let sc = 66 + fear / 2 + jitter(8);
        candidates.push((
            sc,
            DesireDecision {
                kind: "plead_extension",
                target_node: cred.work_node.clone(),
                focus_actor: Some(cred_id.clone()),
                reason_zh: format!("手頭吃緊，赴{}處商議寬限還帳時日", cred.display_name),
            },
        ));
    }

    // 8. 登門尋同鎮親厚師友傾訴商議或互助 (`visit_confidant`)
    let emotional_load = fear.max(anger).max(grief);
    if (emotional_load >= 28
        || bell_index >= 7
        || strat_kind == Some(GoalStrategyKind::KinMutualAid)
        || matches!(
            tend,
            ResponseTendency::SeekAlly | ResponseTendency::ShelterWeak
        ))
        && let Some((fid, fname, fnid)) = find_confidant_scoped(
            doc,
            delta,
            a,
            bell_index >= 5 || a.household_id.starts_with("hh:burg_"),
        )
    {
        let sc = 42
            + emotional_load / 2
            + if bell_index >= 7 { 16 } else { 0 }
            + if top_val == ValueAxis::KinshipLoyalty {
                14
            } else {
                0
            }
            + if strat_kind == Some(GoalStrategyKind::KinMutualAid) {
                26
            } else {
                0
            }
            + hysteresis("visit_confidant")
            + jitter(9);
        candidates.push((
            sc,
            DesireDecision {
                kind: "visit_confidant",
                target_node: fnid,
                focus_actor: Some(fid),
                reason_zh: format!("登門尋熟識親厚的{}傾談近況商議對策", fname),
            },
        ));
    }

    // 9. 赴林緣或海灘採集特產變賣或補自家鋪倉 (`forage_wild` / `sell_foraged`)
    let cur_forage_win = (abs_bell.div_euclid(12)) * 4 + i64::from(bell_index / 3);
    let has_foraged_goods = a.backpack.iter().any(|i| {
        matches!(
            i.slug.as_str(),
            "driftwood"
                | "herb_bundle"
                | "iron_ore"
                | "black_coal"
                | "charcoal_sack"
                | "timber_log"
                | "quarantine_wormwood"
                | "wheat_grain"
        ) && i.count > 0
            && i.bound_owner.is_none()
            && i.stolen_from.is_none()
    });
    let shop_work_nid = doc
        .nodes
        .get(&a.work_node)
        .is_some_and(|n| n.kind == "shop")
        .then_some(a.work_node.as_str());
    if has_foraged_goods {
        if a.mind.stm.last_forage_window <= cur_forage_win
            && let Some(sell_nid) = if forage_shortage_adj > 0 {
                shop_work_nid
            } else {
                None
            }
            .or_else(|| {
                rural_yard_opt
                    .as_deref()
                    .or(tav_opt)
                    .filter(|tn| a.mind.ltm.known_nodes.contains(*tn))
            })
            .or_else(|| {
                a.mind
                    .ltm
                    .known_nodes
                    .iter()
                    .find(|nid| {
                        doc.nodes
                            .get(*nid)
                            .is_some_and(|n| n.kind == "plaza" || n.kind == "dock")
                    })
                    .map(String::as_str)
            })
        {
            candidates.push((
                74 + (forage_shortage_adj / 2) + jitter(10),
                DesireDecision {
                    kind: "sell_foraged",
                    target_node: sell_nid.to_string(),
                    focus_actor: None,
                    reason_zh: if forage_shortage_adj > 0 && shop_work_nid == Some(sell_nid) {
                        "趕回鋪內將採得物料補入貨倉重開爐火或藥臼".to_string()
                    } else {
                        "揹著剛採得的山海物料回鎮變賣貼補家用".to_string()
                    },
                },
            ));
        }
    } else if a.mind.stm.last_forage_window < cur_forage_win
        && super::transit_and_cargo::count_backpack_bulk_units(&a.backpack) < 2
        && (a.copper < 10
            || larder_count < 3
            || forage_shortage_adj > 0
            || top_val == ValueAxis::CuriosityVenture
            || strat_kind == Some(GoalStrategyKind::ForageSupplement)
            || tend == ResponseTendency::Circumvent)
        && bell_index <= 8
        && let Some(wild_nid) = find_known_wild_forage_node(doc, a)
    {
        let sc = 44
            + if a.copper < 6 { 22 } else { 0 }
            + if strat_kind == Some(GoalStrategyKind::ForageSupplement) {
                28
            } else {
                0
            }
            + if top_val == ValueAxis::CuriosityVenture {
                14
            } else {
                0
            }
            + i32::from(innate.temperament.approach_avoid).max(0) / 4
            + forage_shortage_adj
            + hysteresis("forage_wild")
            + jitter(11);
        candidates.push((
            sc,
            DesireDecision {
                kind: "forage_wild",
                target_node: wild_nid,
                focus_actor: None,
                reason_zh: "出城往鄰近山林海灘採拾可用物料貼補生計".to_string(),
            },
        ));
    }

    // 10. 性格分流排解：閉門磨器 (`craft_hone`) vs 傍晚赴酒館打聽音訊 (`tavern_unwind`)
    if innate.temperament.approach_avoid < 0
        || top_val == ValueAxis::CraftPride
        || top_val == ValueAxis::Autonomy
        || strat_kind == Some(GoalStrategyKind::HoneCraft)
        || a.bound_weapon.wear_stage > 0
    {
        let sc = 38
            + if bell_index >= 7 { 18 } else { 0 }
            + if a.bound_weapon.wear_stage > 0 { 20 } else { 0 }
            + if strat_kind == Some(GoalStrategyKind::HoneCraft) {
                26
            } else {
                0
            }
            + emotional_load / 3
            + jitter(12);
        candidates.push((
            sc,
            DesireDecision {
                kind: "craft_hone",
                target_node: bed_node,
                focus_actor: None,
                reason_zh: format!("回屋閉門細細擦拭打磨【{}】以定心神", a.bound_weapon.name),
            },
        ));
    }
    if let Some(tn) = tav_opt {
        let unwind_dst = rural_yard_opt.as_deref().unwrap_or(tn);
        if a.mind.ltm.known_nodes.contains(unwind_dst)
            && a.copper >= 8
            && (fatigue >= 22
                || emotional_load >= 24
                || strat_kind == Some(GoalStrategyKind::SeekCrossTownNews))
            && (bell_index >= 7
                || strat_kind == Some(GoalStrategyKind::SeekCrossTownNews)
                || innate.temperament.approach_avoid > 16)
        {
            let has_cross_town_kin = a
                .mind
                .ltm
                .social
                .values()
                .any(|s| s.warmth >= 25 && !s.target_region_zh.is_empty());
            let sc = 36
                + if bell_index >= 7 { 14 } else { -10 }
                + i32::from(innate.temperament.approach_avoid) / 3
                + if has_cross_town_kin { 10 } else { 0 }
                + if strat_kind == Some(GoalStrategyKind::SeekCrossTownNews) {
                    26
                } else {
                    0
                }
                + jitter(13);
            candidates.push((
                sc,
                DesireDecision {
                    kind: "tavern_unwind",
                    target_node: unwind_dst.to_string(),
                    focus_actor: None,
                    reason_zh: "趁歇工往集場酒肆喝杯麥酒、打聽外鎮親故與商路風聲".to_string(),
                },
            ));
        }
    }

    candidates
        .into_iter()
        .max_by_key(|(sc, _)| *sc)
        .map(|(_, d)| d)
        .unwrap_or(DesireDecision {
            kind: "walk_work",
            target_node: a.work_node.clone(),
            focus_actor: None,
            reason_zh: "前往當值處幹活".to_string(),
        })
}
