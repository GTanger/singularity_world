#[cfg(test)]
mod tests_1 {
    use super::*;

    #[test]
    fn m3_020_followup_event_payloads_are_specific() {
        assert_eq!(
            m3_020_scene_object_followup_event_payload("m3_020_flagstone_waiting_lines", "Use"),
            Some("m3_020_flagstone_waiting_lines|Use".to_string())
        );
        assert_eq!(
            m3_020_scene_object_followup_event_payload("m3_020_flagstone_waiting_lines", "Look"),
            None
        );
        assert_eq!(
            m3_020_scene_object_followup_event_payload("ordinary_door", "Open"),
            None
        );
    }

    #[test]
    fn m3_020_queue_state_requires_waiting_line() {
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_mint_half_open_door",
                "Queue",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_mint_half_open_door",
                "Queue",
                &["m3_020_flagstone_waiting_lines|Use".to_string()],
                &[]
            ),
            Some(M3_020_MINT_QUEUE_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_mint_half_open_door",
                "Queue",
                &["m3_020_flagstone_waiting_lines|Use".to_string()],
                &[M3_020_MINT_QUEUE_STATE.to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_020_short_work_state_requires_board_read() {
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_hire_short_work_boards",
                "AcceptJob",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_hire_short_work_boards",
                "AcceptJob",
                &["m3_020_hire_short_work_boards|Read".to_string()],
                &[]
            ),
            Some(M3_020_SHORT_WORK_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_hire_short_work_boards",
                "AcceptJob",
                &["m3_020_hire_short_work_boards|Read".to_string()],
                &[M3_020_SHORT_WORK_STATE.to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_020_pass_and_council_states_require_prior_read() {
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_stone_gate_pass_tags",
                "JoinCheck",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_stone_gate_pass_tags",
                "JoinCheck",
                &["m3_020_stone_gate_pass_tags|Read".to_string()],
                &[]
            ),
            Some(M3_020_PASS_CHECK_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_council_notice_strip",
                "RegisterComplaint",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_council_notice_strip",
                "RegisterComplaint",
                &["m3_020_council_notice_strip|Read".to_string()],
                &[]
            ),
            Some(M3_020_COUNCIL_COMPLAINT_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_council_notice_strip",
                "RegisterComplaint",
                &["m3_020_council_notice_strip|Read".to_string()],
                &[M3_020_COUNCIL_COMPLAINT_STATE.to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_027_object_states_remain_concrete_scene_states() {
        assert_eq!(
            m3_027_scene_object_state_event_payload("m3_027_wet_bench", "Rest", &[], &[]),
            Some(M3_027_BENCH_REST_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_wet_bench",
                "Rest",
                &[],
                &[M3_027_BENCH_REST_STATE.to_string()]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_wet_bench",
                "Rest",
                &[],
                &[M3_027_BENCH_REST_STATE.to_string()]
            )
            .unwrap()
            .contains("包袱佔回去")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_hot_soup_stall",
                0,
                &[],
                &[M3_027_SOUP_WARM_STATE.to_string()]
            )
            .unwrap()
            .contains("空碗")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_hot_soup_stall",
                "BuySoup",
                &[],
                &[M3_027_FREE_BROTH_STATE.to_string()]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_hot_soup_stall",
                "QueueBroth",
                &[],
                &[M3_027_SOUP_WARM_STATE.to_string()]
            )
            .unwrap()
            .contains("輪過你")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload("m3_027_river_awning", "DryOff", &[], &[]),
            Some(M3_027_AWNING_DRY_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_charcoal_brazier",
                "WarmHands",
                &[],
                &[]
            ),
            Some(M3_027_BRAZIER_WARM_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_waiting_line_chat",
                "TalkQueue",
                &[],
                &[]
            ),
            Some(M3_027_QUEUE_TALK_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_waiting_line_chat",
                "QueueFerry",
                &[],
                &[]
            ),
            Some(M3_027_QUEUE_FERRY_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_waiting_line_chat",
                "QueueFerry",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_waiting_line_chat",
                "QueueFerry",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            )
            .unwrap()
            .contains("你已在人龍")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_waiting_line_chat",
                0,
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            )
            .unwrap()
            .contains("同一條木牌線")
        );
    }

    #[test]
    fn m3_027_rope_help_requires_prior_cargo_help() {
        assert_eq!(
            m3_027_scene_object_state_event_payload("m3_027_boat_rope", "HelpRope", &[], &[]),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response("m3_027_boat_rope", "HelpRope", &[], &[])
                .unwrap()
                .contains("貨擔")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_boat_rope",
                "HelpRope",
                &[],
                &[M3_027_CARGO_HELP_STATE.to_string()]
            ),
            Some(M3_027_ROPE_HELP_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_boat_rope",
                "HelpRope",
                &[],
                &[
                    M3_027_CARGO_HELP_STATE.to_string(),
                    M3_027_ROPE_HELP_STATE.to_string()
                ]
            ),
            None
        );
    }

    #[test]
    fn m3_027_cross_scene_loop_requires_prior_scene_states() {
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_plank_boarding_marker",
                "BoardFerry",
                &[],
                &[]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_plank_boarding_marker",
                "BoardFerry",
                &[],
                &[]
            )
            .unwrap()
            .contains("先回北岸候列")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_plank_boarding_marker",
                "BoardFerry",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            ),
            Some(M3_027_BOARD_FERRY_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_south_landing_rope",
                "StepAshore",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            ),
            None
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_south_landing_rope",
                "StepAshore",
                &[],
                &[M3_027_BOARD_FERRY_STATE.to_string()]
            ),
            Some(M3_027_LANDED_SOUTH_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_warehouse_tally_board",
                "CheckCargo",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_warehouse_tally_board",
                "CheckCargo",
                &[],
                &[M3_027_CARGO_HELP_STATE.to_string()]
            ),
            Some(M3_027_WAREHOUSE_TALLY_STATE)
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_plank_boarding_marker",
                0,
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            )
            .unwrap()
            .contains("人龍位置")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_warehouse_tally_board",
                0,
                &[],
                &[M3_027_CARGO_HELP_STATE.to_string()]
            )
            .unwrap()
            .contains("短冊")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_warehouse_tally_board",
                0,
                &[],
                &[
                    M3_027_CARGO_HELP_STATE.to_string(),
                    M3_027_BOARD_FERRY_STATE.to_string(),
                    M3_027_LANDED_SOUTH_STATE.to_string(),
                ]
            )
            .unwrap()
            .contains("短冊空出一行")
        );
    }

    #[test]
    fn m3_027_reed_market_uses_recent_sales_as_round_demand() {
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &[],
                &[]
            ),
            Some(M3_027_REED_SOLD_STATE)
        );

        let one_sale = vec!["m3_027_market_reed_buyer|SellReeds".to_string()];
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &one_sale,
                &[M3_027_REED_SOLD_STATE.to_string()]
            ),
            Some(M3_027_REED_DEMAND_FILLED_STATE)
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &one_sale,
                &[M3_027_REED_SOLD_STATE.to_string()]
            )
            .unwrap()
            .contains("價牌")
        );
        assert!(m3_027_reed_market_demand_filled(
            &[
                "m3_027_market_reed_buyer|SellReeds".to_string(),
                "m3_027_market_reed_buyer|SellReeds".to_string(),
            ],
            &[]
        ));

        let buyer_obj = crate::model::RoomObject {
            id: "m3_027_market_reed_buyer".to_string(),
            name: "蘆草收牌".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "SellReeds".to_string()],
            responses: [("Look".to_string(), "價牌壓在秤邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (after_one_sale, _) = object_followup_buttons(
            &buyer_obj,
            "",
            "Look",
            &one_sale,
            &[M3_027_REED_SOLD_STATE.to_string()],
        );
        assert_eq!(after_one_sale, vec!["SellReeds".to_string()]);
        let (after_demand_filled, _) = object_followup_buttons(
            &buyer_obj,
            "",
            "Look",
            &one_sale,
            &[
                M3_027_REED_SOLD_STATE.to_string(),
                M3_027_REED_DEMAND_FILLED_STATE.to_string(),
            ],
        );
        assert!(after_demand_filled.is_empty());
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_market_reed_buyer",
                0,
                &one_sale,
                &[M3_027_REED_SOLD_STATE.to_string()]
            )
            .unwrap()
            .contains("蘆草還未收滿")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_market_reed_buyer",
                0,
                &one_sale,
                &[M3_027_REED_DEMAND_FILLED_STATE.to_string()]
            )
            .unwrap()
            .contains("蘆草墊箱")
        );
    }

    #[test]
    fn m3_027_pack_order_multi_material_demands_are_independent() {
        assert_eq!(
            m3_027_material_source_item_for_object("m3_027_market_sackcloth_pile"),
            Some(M3_027_SACKCLOTH_ITEM_ID)
        );
        assert_eq!(
            m3_027_material_source_item_for_object("m3_027_plank_wet_rope_coil"),
            Some(M3_027_ROPE_ITEM_ID)
        );
        assert_eq!(
            m3_027_material_gather_state_for_object(
                "m3_027_market_sackcloth_pile",
                M3_027_SACKCLOTH_ITEM_ID
            ),
            Some(M3_027_SACKCLOTH_GATHERED_STATE)
        );
        assert_eq!(
            m3_027_material_gather_state_for_object(
                "m3_027_plank_wet_rope_coil",
                M3_027_ROPE_ITEM_ID
            ),
            Some(M3_027_ROPE_GATHERED_STATE)
        );

        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellSackcloth",
                &[],
                &[]
            ),
            Some(M3_027_SACKCLOTH_DEMAND_FILLED_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellRope",
                &[],
                &[]
            ),
            Some(M3_027_ROPE_DEMAND_FILLED_STATE)
        );

        let pack_order = crate::model::RoomObject {
            id: "m3_027_market_reed_buyer".to_string(),
            name: "市集包貨單".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "SellReeds".to_string(),
                "SellSackcloth".to_string(),
                "SellRope".to_string(),
            ],
            responses: [("Look".to_string(), "包貨單壓在秤邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (after_sackcloth, _) = object_followup_buttons(
            &pack_order,
            "",
            "Look",
            &[],
            &[M3_027_SACKCLOTH_DEMAND_FILLED_STATE.to_string()],
        );
        assert_eq!(
            after_sackcloth,
            vec!["SellReeds".to_string(), "SellRope".to_string()]
        );
        let all_filled_states = vec![
            M3_027_REED_DEMAND_FILLED_STATE.to_string(),
            M3_027_SACKCLOTH_DEMAND_FILLED_STATE.to_string(),
            M3_027_ROPE_DEMAND_FILLED_STATE.to_string(),
        ];
        let (after_all_filled, _) =
            object_followup_buttons(&pack_order, "", "Look", &[], &all_filled_states);
        assert!(after_all_filled.is_empty());
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_market_reed_buyer",
                0,
                &[],
                &all_filled_states
            )
            .unwrap()
            .contains("三行")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_warehouse_tally_board",
                0,
                &[],
                &all_filled_states
            )
            .unwrap()
            .contains("麻繩")
        );
    }

    #[test]
    fn m3_029_pack_runner_delivery_counts_toward_shared_reed_demand() {
        let npc_reed_action = vec!["m3_027_market_reed_buyer|SellReeds".to_string()];
        let npc_reed_states = vec![
            "m3_029_pack_runner|reed_delivered_to_ferry_market".to_string(),
            M3_027_REED_SOLD_STATE.to_string(),
        ];
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &npc_reed_action,
                &npc_reed_states
            ),
            Some(M3_027_REED_DEMAND_FILLED_STATE)
        );
        let market_echo = m3_027_scene_object_look_echo(
            "",
            "m3_027_market_reed_buyer",
            0,
            &npc_reed_action,
            &npc_reed_states,
        )
        .unwrap();
        assert!(market_echo.contains("搬貨人"));
        assert!(market_echo.contains("同一張單"));

        let warehouse_echo = m3_027_scene_object_look_echo(
            "",
            "m3_027_warehouse_tally_board",
            0,
            &npc_reed_action,
            &npc_reed_states,
        )
        .unwrap();
        assert!(warehouse_echo.contains("市集"));
        assert!(warehouse_echo.contains("倉棚"));
    }

    #[test]
    fn m3_027_object_buttons_reflect_consumed_state() {
        let soup_obj = crate::model::RoomObject {
            id: "m3_027_hot_soup_stall".to_string(),
            name: "熱湯攤".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "BuySoup".to_string(),
                "QueueBroth".to_string(),
            ],
            responses: [("Look".to_string(), "湯桶冒著熱氣。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (available, _) = object_followup_buttons(&soup_obj, "", "Look", &[], &[]);
        assert_eq!(
            available,
            vec!["BuySoup".to_string(), "QueueBroth".to_string()]
        );
        let (consumed, _) = object_followup_buttons(
            &soup_obj,
            "",
            "Look",
            &[],
            &[M3_027_FREE_BROTH_STATE.to_string()],
        );
        assert!(consumed.is_empty());

        let awning_obj = crate::model::RoomObject {
            id: "m3_027_river_awning".to_string(),
            name: "避風棚".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "DryOff".to_string()],
            responses: [("Look".to_string(), "舊帆布壓著木桿。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (already_dried, _) = object_followup_buttons(
            &awning_obj,
            "",
            "Look",
            &[],
            &[M3_027_AWNING_DRY_STATE.to_string()],
        );
        assert!(already_dried.is_empty());

        let queue_obj = crate::model::RoomObject {
            id: "m3_027_waiting_line_chat".to_string(),
            name: "候渡人龍".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "TalkQueue".to_string(),
                "QueueFerry".to_string(),
            ],
            responses: [("Look".to_string(), "候渡人龍折在踏板前。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (queue_available, _) = object_followup_buttons(&queue_obj, "", "Look", &[], &[]);
        assert_eq!(
            queue_available,
            vec!["TalkQueue".to_string(), "QueueFerry".to_string()]
        );
        let (already_talked, _) = object_followup_buttons(
            &queue_obj,
            "",
            "Look",
            &[],
            &[M3_027_QUEUE_TALK_STATE.to_string()],
        );
        assert_eq!(already_talked, vec!["QueueFerry".to_string()]);
        let (already_queued, _) = object_followup_buttons(
            &queue_obj,
            "",
            "Look",
            &[],
            &[
                M3_027_QUEUE_TALK_STATE.to_string(),
                M3_027_QUEUE_FERRY_STATE.to_string(),
            ],
        );
        assert!(already_queued.is_empty());

        let plank_obj = crate::model::RoomObject {
            id: "m3_027_plank_boarding_marker".to_string(),
            name: "踏板木牌".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "BoardFerry".to_string()],
            responses: [("Look".to_string(), "踏板木牌立在水線邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_queue, _) = object_followup_buttons(&plank_obj, "", "Look", &[], &[]);
        assert!(without_queue.is_empty());
        let (with_queue, _) = object_followup_buttons(
            &plank_obj,
            "",
            "Look",
            &[],
            &[M3_027_QUEUE_FERRY_STATE.to_string()],
        );
        assert_eq!(with_queue, vec!["BoardFerry".to_string()]);

        let warehouse_obj = crate::model::RoomObject {
            id: "m3_027_warehouse_tally_board".to_string(),
            name: "倉棚點貨牌".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "CheckCargo".to_string()],
            responses: [("Look".to_string(), "點貨牌掛在倉門邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_cargo, _) = object_followup_buttons(&warehouse_obj, "", "Look", &[], &[]);
        assert!(without_cargo.is_empty());
        let (with_cargo, _) = object_followup_buttons(
            &warehouse_obj,
            "",
            "Look",
            &[],
            &[M3_027_CARGO_HELP_STATE.to_string()],
        );
        assert_eq!(with_cargo, vec!["CheckCargo".to_string()]);
        let (already_checked, _) = object_followup_buttons(
            &warehouse_obj,
            "",
            "Look",
            &[],
            &[
                M3_027_CARGO_HELP_STATE.to_string(),
                M3_027_WAREHOUSE_TALLY_STATE.to_string(),
            ],
        );
        assert!(already_checked.is_empty());
    }

}
