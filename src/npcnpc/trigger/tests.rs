#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Character;

    #[test]
    fn player_layer_pair_labels_use_scene_identity() {
        assert_eq!(
            player_layer_pair_labels("半嶺水岸渡口"),
            ("候渡客".to_string(), "舟子".to_string())
        );
        assert_eq!(
            player_layer_pair_labels("水尾寮渡頭市集"),
            ("候渡客".to_string(), "舟子".to_string())
        );
        assert_eq!(
            player_layer_pair_labels("臨河歇腳處"),
            ("候渡客".to_string(), "舟子".to_string())
        );
    }

    #[test]
    fn player_layer_dialogue_replaces_private_names() {
        let text = replace_private_names_for_player(
            "歐陽秀建說尹淑秀快看船。",
            &[("歐陽秀建", "候渡客"), ("尹淑秀", "舟子")],
        );
        assert_eq!(text, "候渡客說舟子快看船。");
    }

    #[test]
    fn test_format_staged_npc_dialogue_never_wraps_transit_or_leaks_destination() {
        let activity_a = "正動身前往【三水外坊・水巷搬運員宿所】回屋舍吹燈安穩睡一覺";
        let res = format_staged_npc_dialogue(
            0,
            "老張",
            "小李",
            "今日收工了。",
            "是啊，回去歇歇。",
            activity_a,
            "",
        );
        assert!(
            !res.contains("停下手中"),
            "Should not contain '停下手中', got: {}",
            res
        );
        assert!(
            !res.contains("【三水外坊・水巷搬運員宿所】"),
            "Should not leak bracketed destination, got: {}",
            res
        );
    }

    #[test]
    fn test_resolve_viewer_npc_label_recognizes_uniform_and_counter_zone() {
        let c = Character {
            id: "ambient_mill_staff_01".to_string(),
            kind: crate::entity::EntityKind::Npc,
            display_char: "匠".to_string(),
            x: 0,
            y: 0,
            move_state: crate::entity::MoveState::Idle,
            target_x: None,
            target_y: None,
            walk_or_run: crate::entity::WalkOrRun::Walk,
            move_started_at: None,
            vit: 10,
            qi: 10,
            dex: 10,
            magnesium: 0,
            last_observed_at: None,
            created_at: 0,
            gender: Some(crate::entity::Gender::M),
            soul_seed: None,
            display_title: String::new(),
            activated_nodes: "[]".to_string(),
            equipment_slots: r#"{"body":"wt_cat_0840"}"#.to_string(),
            inventory: "[]".to_string(),
            disposition: 0,
            thoughts: vec![],
            current_activity: "正在櫃檯內側照料石磨".to_string(),
            grid_x: None,
            grid_y: None,
        };
        let label = resolve_viewer_npc_label("player_unknown_1", &c, "老堰頭・水輪石磨工坊");
        assert!(
            label.contains("身著制服站於櫃檯內側的"),
            "Expected label containing '身著制服站於櫃檯內側的', got: {label}"
        );
        assert_eq!(label, "身著制服站於櫃檯內側的當值石磨匠師");
    }

    #[test]
    fn test_player_layer_pair_labels_uses_room_venue_object() {
        assert_eq!(
            player_layer_pair_labels("老堰頭・水輪石磨工坊"),
            (
                "當值石磨匠師".to_string(),
                "來水輪石磨工坊辦事的街坊".to_string()
            )
        );
    }
}
