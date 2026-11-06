#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peninsula_daily_scripts_count_and_no_forbidden_tokens() {
        assert_eq!(PENINSULA_DAILY_SCRIPTS.len(), 108);
        let forbidden = ["debug", "資料庫", "database", "grid", "scene_stub", "還有"];
        for sc in PENINSULA_DAILY_SCRIPTS {
            for token in forbidden {
                assert!(
                    !sc.title.to_lowercase().contains(token)
                        && !sc.trigger_and_causal_chain.to_lowercase().contains(token)
                        && !sc.player_actions.to_lowercase().contains(token)
                        && !sc.outcome_and_world_change.to_lowercase().contains(token),
                    "script {} contains forbidden token {}",
                    sc.id,
                    token
                );
            }
        }
        let wen_yao_scripts = scripts_for_room("文曜・公議堂");
        assert!(!wen_yao_scripts.is_empty());
        assert!(find_script_by_id("WS-001").is_some());
        assert!(find_script_by_id("WS-108").is_some());
        for sc in PENINSULA_DAILY_SCRIPTS {
            assert!(!sc.category_tag.is_empty(), "script {} has empty category_tag", sc.id);
            assert_eq!(sc.category_tag, script_category_tag_for_num(sc.category_num));
        }
    }

    #[test]
    fn test_decompose_story_into_event_pack_and_script_pack_dynamic_slots() {
        let ws001 = find_script_by_id("WS-001").expect("WS-001 must exist");

        // Case 1: host_override = None, partner_override = None -> neutral role slots
        let (ev1, sp1) = decompose_into_event_and_script_pack(ws001, "老堰頭・水輪石磨工坊", None, None);
        assert_eq!(ev1.event_title, "老堰頭水磨米漿趕送文曜早市");
        assert!(!ev1.event_title.contains("秦水生"));
        assert!(!ev1.event_title.contains("陳萬福"));
        assert!(!ev1.trigger_and_causal_chain.contains("秦水生"));
        assert!(!ev1.trigger_and_causal_chain.contains("陳萬福"));
        assert!(ev1.trigger_and_causal_chain.contains("當值主事"));

        assert!(!sp1.player_actions.contains("秦水生"));
        assert!(!sp1.player_actions.contains("陳萬福"));
        assert!(sp1.player_actions.contains("當值主事"));

        assert!(!sp1.outcome_and_world_change.contains("秦水生"));
        assert!(!sp1.outcome_and_world_change.contains("陳萬福"));
        assert!(sp1.outcome_and_world_change.contains("當值主事"));
        assert!(sp1.outcome_and_world_change.contains("鄰坊當值主事"));

        assert_eq!(sp1.host_slot_actor, "當值主事");
        assert_eq!(sp1.partner_slot_actor, "鄰坊當值主事");
        assert_eq!(
            ev1.room_tags,
            vec!["workshop", "grindstone", "grain", "canteen", "hot_food"]
        );

        // Case 2: host_override = Some("林阿伯"), partner_override = Some("張大嫂") -> dynamic actors
        let (ev2, sp2) = decompose_into_event_and_script_pack(
            ws001,
            "老堰頭・水輪石磨工坊",
            Some("林阿伯"),
            Some("張大嫂"),
        );
        assert_eq!(ev2.event_title, "老堰頭水磨米漿趕送文曜早市");
        assert!(!ev2.event_title.contains("秦水生"));
        assert!(!ev2.event_title.contains("陳萬福"));
        assert!(!ev2.trigger_and_causal_chain.contains("秦水生"));
        assert!(!ev2.trigger_and_causal_chain.contains("陳萬福"));
        assert!(ev2.trigger_and_causal_chain.contains("林阿伯"));

        assert!(!sp2.player_actions.contains("秦水生"));
        assert!(!sp2.player_actions.contains("陳萬福"));
        assert!(sp2.player_actions.contains("林阿伯"));

        assert!(!sp2.outcome_and_world_change.contains("秦水生"));
        assert!(!sp2.outcome_and_world_change.contains("陳萬福"));
        assert!(sp2.outcome_and_world_change.contains("林阿伯"));
        assert!(sp2.outcome_and_world_change.contains("張大嫂"));

        assert_eq!(sp2.host_slot_actor, "林阿伯");
        assert_eq!(sp2.partner_slot_actor, "張大嫂");

        // Case 3: active_event_and_script_pack_for_room dynamic resolution
        let active_opt = active_event_and_script_pack_for_room("老堰頭・水輪石磨工坊", Some("林阿伯"), 1_700_000_000);
        assert!(active_opt.is_some());
        let (ev3, sp3) = active_opt.unwrap();
        assert_eq!(sp3.host_slot_actor, "林阿伯");
        for &name in EXEMPLAR_NAMES {
            assert!(!ev3.event_title.contains(name), "ev3 title contains exemplar {name}");
            assert!(!sp3.player_actions.contains(name), "sp3 player_actions contains exemplar {name}");
        }
    }

    #[test]
    fn test_object_socket_advances_room_event_and_script_pack() {
        let res = try_advance_room_event_and_script_pack_via_object_full(
            "老堰頭・水輪石磨工坊",
            "wt_mill_001",
            "水輪大石磨",
            "MillGrain",
            1_700_000_000,
        );
        assert!(res.is_some(), "operating mill grain must advance script pack");
        let (ev, sp, note) = res.unwrap();
        assert!(
            note.contains("一同落實手頭要務"),
            "note must contain diegetic linkage description: {note}"
        );
        assert!(
            !note.contains("事件包") && !note.contains("劇本包") && !note.contains("劇本演繹"),
            "note must not contain meta-design terms: {note}"
        );
        for &name in EXEMPLAR_NAMES {
            assert!(
                !note.contains(name),
                "linkage note must not contain exemplar {name}: {note}"
            );
            assert!(
                !ev.event_title.contains(name),
                "event title must not contain exemplar {name}: {}",
                ev.event_title
            );
            assert!(
                !sp.host_slot_actor.contains(name),
                "host actor must not contain exemplar {name}: {}",
                sp.host_slot_actor
            );
            assert!(
                !sp.partner_slot_actor.contains(name),
                "partner actor must not contain exemplar {name}: {}",
                sp.partner_slot_actor
            );
            assert!(
                !sp.outcome_and_world_change.contains(name),
                "outcome must not contain exemplar {name}: {}",
                sp.outcome_and_world_change
            );
        }

        let look_res = try_advance_room_event_and_script_pack_via_object_full(
            "老堰頭・水輪石磨工坊",
            "wt_mill_001",
            "水輪大石磨",
            "Look",
            1_700_000_000,
        );
        assert!(look_res.is_none(), "passive Look action must not trigger socket advance");

        let short_res = try_advance_room_event_and_script_pack_via_object(
            "老堰頭・水輪石磨工坊",
            "水輪大石磨",
        );
        assert!(short_res.is_some(), "2-arg helper must advance script pack");
    }
}
