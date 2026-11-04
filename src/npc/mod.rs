// npc 模組 — NPC 行為與主題劇本；建立／種子實作在 `crate::db`。

mod behavior;
pub mod brain_arrival;
pub mod causal_core;
pub mod debt_interaction;
mod decision;
mod grid_ai;
mod job_matching;
pub mod narrative;
mod reaction;
mod social;
mod topics;
mod traveler;
mod unobserved;

pub use crate::db::{
    DEFAULT_NPCS, NpcDef, ensure_all_npcs_have_soul_seed, first_rune, get_item_descs,
    get_item_names, get_npc_gender_counts, insert_npc, is_naked, seed_items, seed_npcs,
    seed_npcs_for_store, spawn_one_npc_from_pool, starter_equipment,
};
pub use behavior::{
    get_shift_flavor, get_time_period, get_wander_flavor, get_wander_rooms,
    movement_speed_for_title, pick_enter_reaction, pick_idle_emote, try_load_npc_behaviors,
};
pub use brain_arrival::apply_brain_arrival_effects;
pub use decision::IntentType;
pub use grid_ai::{GridMoveEvent, GridNpcManager};
pub use job_matching::{
    DEFAULT_OCCUPATION_FOR_HIRE, JOB_MATCH_STABLE_PROBABILITY, JobMatchParams,
    RoomCandidate, SEEK_JOB_MG_THRESHOLD_DEFAULT, find_best_work_room,
    occupation_keywords, room_matches_occupation, run_job_matching_tick,
};
pub use reaction::npc_behavior_reaction_line;
pub use social::pick_micro_interaction;
pub use topics::{
    NpcNpcTopic, NpcTopicMask, TopicWeightDebug, debug_topic_weights_for_pair,
    find_npc_npc_topic_id_by_hint, get_npc_npc_topic_by_id, load_npc_npc_topics,
    pair_relation_weight_adj, pick_random_npc_npc_topic, pick_random_npc_npc_topic_by_mask,
    pick_random_npc_npc_topic_exclude, pick_random_npc_npc_topic_for_pair, room_has_tag,
    topic_mask_base_weight, try_load_npc_npc_topics,
};
pub use traveler::{MovementDef, MovementType, NpcStep, TravelerManager, seed_traveler_manager};
pub use unobserved::{UNOBSERVED_MAX_NPCS_PER_TICK, run_unobserved_world_tick};
pub use causal_core::{
    active_stranded_cargo_for_carrier_or_room, claim_or_deliver_stranded_cargo_for_npc,
    create_stranded_cargo_order, credit_magnesium_bounty, deliver_stranded_cargo_for_player,
    find_relational_hook_for_target, inject_ripple_fact_into_npc,
    list_active_stranded_cargo_orders, mutate_npc_from_player_action,
    propagate_ripple_between_npcs, top_active_ripple_for_npc, upsert_relational_hook,
    NpcCausalState, RelationalHook, RippleFactKind, StrandedCargoOrder, WorldRippleFact,
    LIVE_NPC_CAUSAL_STATES, LIVE_STRANDED_CARGO_ORDERS,
};
