//! 一次性 v1 雙軌 JSON（舊玩家列／舊居民差額列）無損合流為單一 `UnifiedMindBody` 之升級轉換器。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Deserialize;

use super::mind::{CognitiveMind, SocialEvidenceSummary};
use super::persona::CharacterDriftState;
use super::sim_tick::default_tool_for_role;
use super::types::{
    BoundTool, CarryItem, ResidentSeedDef, SkirmishState, TransitState, UnifiedMindBody,
    WorldStaticDoc,
};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct LegacyV1PlayerBlob {
    player_id: String,
    soul_seed: u64,
    rev: u64,
    display_name: String,
    race: String,
    appearance: String,
    origin_pack: String,
    setup_done: bool,
    node_id: String,
    fatigue: u8,
    wound_part: Option<String>,
    severe_wrist_warning: bool,
    bound_weapon: Option<BoundTool>,
    backpack: Vec<CarryItem>,
    inn_locker: Vec<CarryItem>,
    inn_lockers: BTreeMap<String, Vec<CarryItem>>,
    inn_nights_left: u32,
    owned_house_node: Option<String>,
    rescue_pact_active: bool,
    languages: BTreeMap<String, u8>,
    lang_daily_gain: BTreeMap<String, u8>,
    lang_gain_day: i64,
    visited_cells: BTreeSet<u32>,
    known_state_cells: BTreeSet<u32>,
    visited_nodes: BTreeSet<String>,
    unlocked_secrets: BTreeSet<String>,
    known_rumors: BTreeSet<String>,
    acquaintances: BTreeSet<String>,
    explored_windows: BTreeSet<String>,
    recent_breaths: VecDeque<String>,
    active_skirmish: Option<SkirmishState>,
    active_transit: Option<TransitState>,
    last_step_unix_ms: u64,
    last_talk_unix_ms: u64,
    last_active_unix: i64,
    drift: CharacterDriftState,
    mind: CognitiveMind,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct LegacyV1ResidentBlob {
    id: String,
    soul_seed: u64,
    rev: u64,
    #[serde(default = "default_true")]
    alive: bool,
    node_id: String,
    route_plan: VecDeque<String>,
    bound_tool: Option<BoundTool>,
    carry_bag: Vec<CarryItem>,
    wound_part: Option<String>,
    conversing_until_ms: u64,
    languages: BTreeMap<String, u8>,
    drift: CharacterDriftState,
    mind: CognitiveMind,
}

fn normalize_social_keys(mind: &mut CognitiveMind) {
    let old_social = std::mem::take(&mut mind.ltm.social);
    for (k, mut v) in old_social {
        let norm_k = if k.contains(':') {
            k
        } else {
            format!("pl:{k}")
        };
        v.target_id = norm_k.clone();
        mind.ltm.social.insert(norm_k, v);
    }
    for t in &mut mind.stm.recent_talks {
        if !t.partner_id.contains(':') && !t.partner_id.is_empty() {
            t.partner_id = format!("pl:{}", t.partner_id);
        }
    }
}

pub(super) fn migrate_v1_player_json(
    doc: &WorldStaticDoc,
    seed_by_id: &BTreeMap<&str, &ResidentSeedDef>,
    actor_id_col: &str,
    soul_seed_col: i64,
    rev_col: i64,
    copper_col: i64,
    raw_json: &str,
) -> Result<UnifiedMindBody, String> {
    let p: LegacyV1PlayerBlob = serde_json::from_str(raw_json).map_err(|e| e.to_string())?;
    let clean_ctrl = actor_id_col
        .strip_prefix("pl:")
        .map(str::to_string)
        .unwrap_or_else(|| {
            if p.player_id.is_empty() {
                actor_id_col.to_string()
            } else {
                p.player_id.clone()
            }
        });
    let full_aid = if actor_id_col.starts_with("pl:") {
        actor_id_col.to_string()
    } else {
        format!("pl:{clean_ctrl}")
    };

    let mut mind = p.mind;
    normalize_social_keys(&mut mind);
    for cid in p.visited_cells {
        mind.ltm.known_cells.insert(cid);
    }
    for nid in p.visited_nodes {
        mind.ltm.known_nodes.insert(nid);
    }
    let cur_node = if p.node_id.is_empty() {
        doc.spawn_node.clone()
    } else {
        p.node_id
    };
    if let Some(nd) = doc.nodes.get(&cur_node) {
        mind.record_footstep(&cur_node, nd.cell_id);
    }
    for acq in p.acquaintances {
        let norm_acq = if acq.contains(':') {
            acq
        } else {
            format!("pl:{acq}")
        };
        let tname = seed_by_id
            .get(norm_acq.as_str())
            .map_or_else(|| norm_acq.clone(), |s| s.name.clone());
        let entry =
            mind.ltm
                .social
                .entry(norm_acq.clone())
                .or_insert_with(|| SocialEvidenceSummary {
                    target_id: norm_acq,
                    target_name: tname,
                    ..SocialEvidenceSummary::default()
                });
        entry.acquainted = true;
    }

    let role_zh = match p.origin_pack.as_str() {
        "forest_hunter" => "山林獵戶",
        "fallen_peddler" => "行腳商旅",
        "deck_sailor" => "遠航水手",
        _ => "鐵匠學徒",
    };
    let home = p
        .owned_house_node
        .clone()
        .unwrap_or_else(|| doc.spawn_node.clone());
    let seed_u64 = if soul_seed_col > 0 {
        u64::try_from(soul_seed_col).unwrap_or(1)
    } else if p.soul_seed > 0 {
        (p.soul_seed & ((1u64 << 63) - 1)).max(1)
    } else {
        1
    };
    let next_rev = p
        .rev
        .max(u64::try_from(rev_col.max(1)).unwrap_or(1))
        .saturating_add(1);

    Ok(UnifiedMindBody {
        actor_id: full_aid,
        controller_id: Some(clean_ctrl.clone()),
        soul_seed: seed_u64,
        rev: next_rev,
        alive: true,
        setup_done: p.setup_done,
        display_name: if p.display_name.is_empty() {
            clean_ctrl
        } else {
            p.display_name
        },
        race: if p.race.is_empty() {
            "人類".to_string()
        } else {
            p.race
        },
        role: role_zh.to_string(),
        appearance: p.appearance,
        origin_pack: if p.origin_pack.is_empty() {
            "smith_apprentice".to_string()
        } else {
            p.origin_pack
        },
        household_id: String::new(),
        birth_year: 306,
        household_bond: "外來旅人".to_string(),
        ancestor_note_zh: String::new(),
        personal_bio_zh: String::new(),
        bio_core_zh: String::new(),
        bio_post_chapters_zh: Vec::new(),
        home_node: home,
        work_node: doc.spawn_node.clone(),
        node_id: cur_node,
        route_plan: VecDeque::new(),
        copper: copper_col,
        fatigue: p.fatigue,
        wound_part: p.wound_part,
        severe_wrist_warning: p.severe_wrist_warning,
        bound_weapon: p.bound_weapon.unwrap_or_else(|| BoundTool {
            slug: "harbor_hammer".to_string(),
            name: "港務舊鍛錘".to_string(),
            grip_familiarity: 8,
            ..BoundTool::default()
        }),
        backpack: p.backpack,
        inn_locker: p.inn_locker,
        inn_lockers: p.inn_lockers,
        inn_nights_left: p.inn_nights_left,
        assigned_inn_room: None,
        owned_house_node: p.owned_house_node,
        rescue_pact_active: p.rescue_pact_active,
        conversing_until_ms: 0,
        languages: p.languages,
        lang_daily_gain: p.lang_daily_gain,
        lang_gain_day: p.lang_gain_day,
        known_state_cells: p.known_state_cells,
        unlocked_secrets: p.unlocked_secrets,
        known_rumors: p.known_rumors,
        explored_windows: p.explored_windows,
        recent_breaths: p.recent_breaths,
        active_skirmish: p.active_skirmish,
        active_transit: p.active_transit,
        ongoing_activity: None,
        last_step_unix_ms: p.last_step_unix_ms,
        last_talk_unix_ms: p.last_talk_unix_ms,
        last_active_unix: p.last_active_unix,
        drift: p.drift,
        mind,
        dirty: true,
        flushed_rev: 0,
        flushed_copper: i64::MIN,
    })
}

pub(super) fn migrate_v1_resident_json(
    doc: &WorldStaticDoc,
    seed_by_id: &BTreeMap<&str, &ResidentSeedDef>,
    actor_id_col: &str,
    soul_seed_col: i64,
    rev_col: i64,
    copper_delta_col: i64,
    raw_json: &str,
) -> Result<UnifiedMindBody, String> {
    let er: LegacyV1ResidentBlob = serde_json::from_str(raw_json).map_err(|e| e.to_string())?;
    let seed_opt = seed_by_id.get(actor_id_col).copied();
    let name = seed_opt.map_or_else(|| actor_id_col.to_string(), |s| s.name.clone());
    let race = seed_opt.map_or_else(|| "人類".to_string(), |s| s.race.clone());
    let role = seed_opt.map_or_else(|| "街坊居民".to_string(), |s| s.role.clone());
    let household_id = seed_opt.map_or_else(String::new, |s| s.household_id.clone());
    let home_node = seed_opt.map_or_else(|| doc.spawn_node.clone(), |s| s.home_node.clone());
    let work_node = match seed_opt {
        Some(s) => {
            if doc
                .nodes
                .get(&s.home_node)
                .is_some_and(|n| n.kind == "shop")
            {
                s.home_node.clone()
            } else {
                s.work_node.clone()
            }
        }
        None => home_node.clone(),
    };
    let cur_node = if er.node_id.is_empty() {
        work_node.clone()
    } else {
        er.node_id
    };
    let direct_copper = seed_opt.map_or(0, |s| s.copper) + copper_delta_col;

    let mut mind = er.mind;
    normalize_social_keys(&mut mind);
    let home_cell = doc.nodes.get(&home_node).map_or(4616, |n| n.cell_id);
    mind.ltm.known_cells.insert(home_cell);
    mind.ltm.known_nodes.insert(home_node.clone());
    mind.ltm.known_nodes.insert(work_node.clone());
    if let Some(nd) = doc.nodes.get(&cur_node) {
        mind.record_footstep(&cur_node, nd.cell_id);
    }
    let mut known_state = BTreeSet::new();
    known_state.insert(home_cell);

    let seed_u64 = if soul_seed_col > 0 {
        u64::try_from(soul_seed_col).unwrap_or(1)
    } else if er.soul_seed > 0 {
        (er.soul_seed & ((1u64 << 63) - 1)).max(1)
    } else {
        1
    };
    let next_rev = er
        .rev
        .max(u64::try_from(rev_col.max(1)).unwrap_or(1))
        .saturating_add(1);
    let langs = if er.languages.is_empty() {
        seed_opt.map_or_else(BTreeMap::new, |s| s.languages.clone())
    } else {
        er.languages
    };

    Ok(UnifiedMindBody {
        actor_id: actor_id_col.to_string(),
        controller_id: None,
        soul_seed: seed_u64,
        rev: next_rev,
        alive: er.alive,
        setup_done: true,
        display_name: name.clone(),
        race: race.clone(),
        role: role.clone(),
        appearance: format!("{}・{}", race, role),
        origin_pack: "island_native".to_string(),
        household_id,
        birth_year: 300,
        household_bond: String::new(),
        ancestor_note_zh: String::new(),
        personal_bio_zh: String::new(),
        bio_core_zh: String::new(),
        bio_post_chapters_zh: Vec::new(),
        home_node,
        work_node,
        node_id: cur_node,
        route_plan: er.route_plan,
        copper: direct_copper,
        fatigue: mind.stm.affect.fatigue,
        wound_part: er.wound_part,
        severe_wrist_warning: false,
        bound_weapon: er
            .bound_tool
            .unwrap_or_else(|| default_tool_for_role(&role, &name)),
        backpack: er.carry_bag,
        inn_locker: Vec::new(),
        inn_lockers: BTreeMap::new(),
        inn_nights_left: 0,
        assigned_inn_room: None,
        owned_house_node: None,
        rescue_pact_active: false,
        conversing_until_ms: er.conversing_until_ms,
        languages: langs,
        lang_daily_gain: BTreeMap::new(),
        lang_gain_day: 0,
        known_state_cells: known_state,
        unlocked_secrets: BTreeSet::new(),
        known_rumors: BTreeSet::new(),
        explored_windows: BTreeSet::new(),
        recent_breaths: VecDeque::new(),
        active_skirmish: None,
        active_transit: None,
        ongoing_activity: None,
        last_step_unix_ms: 0,
        last_talk_unix_ms: 0,
        last_active_unix: 0,
        drift: er.drift,
        mind,
        dirty: true,
        flushed_rev: 0,
        flushed_copper: i64::MIN,
    })
}
