//! 活世界 NPC 四核實用因果與體感引擎（Causal & Somatic Core）
//!
//! 1. **十八職司器物與修整場所對齊（Vocational Vessel Alignment）**：解決全島 1,772 名住民混用獵刀的錯位。
//! 2. **無鎖批次抵達佇列（Lock-Free Arrival Effect Queue）**：取代每次抵達 `std::thread::spawn` 搶寫鎖。
//! 3. **性格錨點（Cornerstone）＋壓力閾值（Stress）＋三步因果計畫鏈（MicroPlanQueue）＋信念驚奇（Belief ΔE）＋人情帳本（Relational Hooks）**。
//! 4. **按需十層體感微動作合成器（On-Demand 10-Layer Somatic Renderer）**：僅於觀測或 API 查詢時即時渲染，背景零字串浪費。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, RwLock};


#[inline]
pub fn is_recognizable_room_staff_or_counter_holder_entity(
    ent: &crate::store::Entity,
    room_coord: (i32, i32),
    needed_item_id: &str,
) -> (bool, bool) {
    crate::pg::living_world::is_recognizable_room_staff_or_counter_holder_entity(ent, room_coord, needed_item_id)
}

#[inline]
pub fn resolve_nearest_room_by_tag(
    origin: (i32, i32),
    wanted_tags: &[&str],
    grid_opt: Option<&crate::grid::SquareGrid>,
    fallback_coord: (i32, i32),
    fallback_bldg: &str,
) -> ((i32, i32), String) {
    crate::pg::living_world::resolve_nearest_room_by_tag(origin, wanted_tags, grid_opt, fallback_coord, fallback_bldg)
}

include!("causal_core/vessel_and_arrival.rs");
include!("causal_core/personality_and_types.rs");
include!("causal_core/action_catalog.rs");
include!("causal_core/decision_and_economy.rs");
include!("causal_core/planning_and_state.rs");
include!("causal_core/arrival_and_somatic_ripple.rs");
include!("causal_core/stranded_cargo_and_hooks.rs");
include!("causal_core/dyadic_kotodama_and_mutation.rs");
include!("causal_core/offline_autonomy.rs");
include!("causal_core/bank_and_gazette.rs");

#[cfg(test)]
mod tests {
    use super::*;

    include!("causal_core/tests_part1.rs");
    include!("causal_core/tests_part2.rs");
    include!("causal_core/tests_part3.rs");
    #[test]
    fn test_round35_web_cache_version_sync_and_no_raw_socket_exposure() {
        test_frontend_sw_and_index_version_sync_v0_39_57();
    }

    #[test]
    fn test_frontend_sw_and_index_version_sync_v0_39_57() {
        let index_html = include_str!("../../web/index.html");
        let sw_js = include_str!("../../web/sw.js");
        let sw_obj_list = include_str!("../../web/sw-object-list.js");

        let sw_ver = sw_js
            .split("sw-shell-v")
            .nth(1)
            .and_then(|s| s.split(['\'', '"']).next())
            .expect("sw.js should contain sw-shell-v<ver>");
        assert!(index_html.contains(&format!("?v={sw_ver}")), "index.html should use v={sw_ver}");
        assert!(index_html.contains(&format!("sw_reloaded_v{sw_ver}")), "index.html should use sw_reloaded_v{sw_ver}");
        assert!(sw_js.contains(&format!("sw-shell-v{sw_ver}")), "sw.js should use sw-shell-v{sw_ver}");
        // Verify all 10 new custom sockets have Chinese labels in sw-object-list.js
        let expected_sockets = [
            ("MillGrain", "水磨碾米"),
            ("PressCheese", "壓製乳酪"),
            ("SmokeFish", "慢火冷燻"),
            ("CutJade", "切磨寒玉"),
            ("HaulMineCart", "滑車運礦"),
            ("BurnLime", "入窯煅灰"),
            ("RowSwampRaft", "撐筏入澤"),
            ("PrepareMiasmaGear", "備妥避瘴"),
            ("PlayChess", "對弈手談"),
            ("SeasonalBanquet", "歲時圍爐"),
        ];

        for (sock, label) in expected_sockets {
            assert!(
                sw_obj_list.contains(sock),
                "sw-object-list.js missing socket: {sock}"
            );
            assert!(
                sw_obj_list.contains(label),
                "sw-object-list.js missing Chinese label: {label}"
            );
        }
    }

    #[test]
    fn test_sw_version_sync_and_custom_socket_chinese_labels() {
        test_frontend_sw_and_index_version_sync_v0_39_57();
    }

    #[test]
    fn test_round41_households_and_templates_guards() {
        let files = [
            ("src/store/households.rs", include_str!("../store/households.rs")),
            ("src/store/outer_households.rs", include_str!("../store/outer_households.rs")),
            ("src/game/peninsula_catalog.rs", include_str!("../game/peninsula_catalog.rs")),
            ("src/game/peninsula_scripts.rs", include_str!("../game/peninsula_scripts.rs")),
            (
                "src/game/player_scene.rs",
                include_str!("../game/player_scene.rs"),
            ),
            (
                "data/world/singularity_world_skeleton.json",
                include_str!("../../data/world/singularity_world_skeleton.json"),
            ),
            (
                "data/templates/dialogues/herbalist.json",
                include_str!("../../data/templates/dialogues/herbalist.json"),
            ),
            (
                "data/templates/dialogues/martial_artist.json",
                include_str!("../../data/templates/dialogues/martial_artist.json"),
            ),
            (
                "data/templates/dialogues/blacksmith.json",
                include_str!("../../data/templates/dialogues/blacksmith.json"),
            ),
            (
                "data/templates/dialogues/farmer.json",
                include_str!("../../data/templates/dialogues/farmer.json"),
            ),
            (
                "data/templates/dialogues/merchant.json",
                include_str!("../../data/templates/dialogues/merchant.json"),
            ),
        ];

        let forbidden = [
            "報備窯爐",
            "活版印書局",
            "正在長風草甸一帶正",
            "發霉",
            "防霉",
            "老少咸宜",
            "山里採集組",
            "衙門",
            "體適能夏令營",
            "仰臥起坐板",
            "防暴叉",
            "排班",
            "丹田",
            "神識",
            "百金",
            "十銀",
            "五十銅",
            "玉幣",
            "水泥",
            "防暴",
            "外帶",
            "數據",
            "系統",
        ];

        for (filename, content) in files {
            for word in forbidden {
                assert!(
                    !content.contains(word),
                    "{filename} contains forbidden word: {word}"
                );
            }
        }
    }

    #[test]
    fn test_round42_room_and_templates_no_xianxia_or_scifi_guards() {
        let files = [
            ("src/game/room.rs", include_str!("../game/room.rs")),
            ("src/store/rumor.rs", include_str!("../store/rumor.rs")),
            (
                "src/server/handler/movement.rs",
                include_str!("../server/handler/movement.rs"),
            ),
            ("web/world_atlas.html", include_str!("../../web/world_atlas.html")),
            (
                "web/behavior_mindmap.html",
                include_str!("../../web/behavior_mindmap.html"),
            ),
            (
                "data/config/gametext.json",
                include_str!("../../data/config/gametext.json"),
            ),
            (
                "data/config/word_elements.json",
                include_str!("../../data/config/word_elements.json"),
            ),
            (
                "data/npc_behaviors.json",
                include_str!("../../data/npc_behaviors.json"),
            ),
        ];

        let forbidden = [
            "神識",
            "丹田",
            "靈氣",
            "靈力",
            "排班",
            "鏢資",
            "江湖恩怨",
            "江湖宿怨",
            "門派捲入",
            "伏行巡獵、",
        ];

        for (filename, content) in files {
            for word in forbidden {
                assert!(
                    !content.contains(word),
                    "{filename} contains forbidden word: {word}"
                );
            }
        }
    }

    #[test]
    fn test_round43_disposition_catalog_and_atlas_guards() {
        let files = [
            (
                "src/db/disposition.rs",
                include_str!("../db/disposition.rs"),
            ),
            (
                "src/game/peninsula_catalog.rs",
                include_str!("../game/peninsula_catalog.rs"),
            ),
            (
                "src/store/households.rs",
                include_str!("../store/households.rs"),
            ),
            (
                "web/world_atlas.html",
                include_str!("../../web/world_atlas.html"),
            ),
            (
                "data/templates/dialogues/blacksmith.json",
                include_str!("../../data/templates/dialogues/blacksmith.json"),
            ),
            (
                "data/templates/dialogues/broker.json",
                include_str!("../../data/templates/dialogues/broker.json"),
            ),
            (
                "data/templates/dialogues/performer.json",
                include_str!("../../data/templates/dialogues/performer.json"),
            ),
            (
                "data/templates/archetypes.json",
                include_str!("../../data/templates/archetypes.json"),
            ),
            (
                "data/items.json",
                include_str!("../../data/items.json"),
            ),
            (
                "data/npc_behaviors.json",
                include_str!("../../data/npc_behaviors.json"),
            ),
        ];

        let forbidden = [
            "跟人說了會兒話",
            "神仙難擋",
            "重晶石晶片",
            "黑曜石晶片",
            "霜林鋸木大廠",
            "平價板料廠",
            "纜繩工務廠",
            "木藝器械廠",
            "尋常小怪",
            "次級掠食",
            "法術",
            "廢棄晶片",
            "采風",
        ];

        for (filename, content) in files {
            for word in forbidden {
                assert!(
                    !content.contains(word),
                    "{filename} contains forbidden word: {word}"
                );
            }
        }
    }


    include!("causal_core/guard_tests_gang.rs");
    include!("causal_core/guard_forbidden_words.rs");
    include!("causal_core/guard_tests_meta.rs");
    include!("causal_core/tests_part4.rs");
}
