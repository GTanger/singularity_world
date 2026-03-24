// npc 模組 — 與 Go `db/npc.go`、`npc/topics.go` 對齊；建立／種子實作在 `crate::db`。

mod behavior;
mod reaction;
mod brain_arrival;
mod decision;
mod job_matching;
mod social;
mod topics;
mod traveler;
mod unobserved;

pub use reaction::npc_behavior_reaction_line;
pub use brain_arrival::apply_brain_arrival_effects;
pub use decision::IntentType;
pub use behavior::{
    get_shift_flavor, get_time_period, get_wander_flavor, get_wander_rooms, movement_speed_for_title,
    pick_enter_reaction, pick_idle_emote, try_load_npc_behaviors,
};
pub use job_matching::{
    run_job_matching_tick, JobMatchParams, DEFAULT_OCCUPATION_FOR_HIRE, JOB_MATCH_STABLE_PROBABILITY,
    SEEK_JOB_MG_THRESHOLD_DEFAULT,
};
pub use crate::db::{
    ensure_all_npcs_have_soul_seed, first_rune, get_item_descs, get_item_names, get_npc_gender_counts,
    insert_npc, is_naked, seed_items, seed_npcs, seed_npcs_for_store, spawn_one_npc_from_pool,
    starter_equipment, DEFAULT_NPCS, NpcDef,
};
pub use social::pick_micro_interaction;
pub use traveler::{seed_traveler_manager, MovementDef, MovementType, NpcStep, TravelerManager};
pub use unobserved::{run_unobserved_world_tick, UNOBSERVED_MAX_NPCS_PER_TICK};
pub use topics::{
    debug_topic_weights_for_pair, find_npc_npc_topic_id_by_hint, get_npc_npc_topic_by_id,
    load_npc_npc_topics, pair_relation_weight_adj, pick_random_npc_npc_topic,
    pick_random_npc_npc_topic_by_mask, pick_random_npc_npc_topic_exclude,
    pick_random_npc_npc_topic_for_pair, room_has_tag, topic_mask_base_weight, try_load_npc_npc_topics,
    NpcNpcTopic, NpcTopicMask, TopicWeightDebug,
};
