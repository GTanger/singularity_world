
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_open_trade_input() {
        for input in ["想看看你有什麼", "有什麼可以買？", "我要買東西", "trade"]
        {
            assert!(is_open_trade_input(input), "{input}");
        }
    }

    #[test]
    fn leaves_plain_talk_as_talk() {
        for input in ["有什麼事嗎", "你好", "（搭話）", ""] {
            assert!(!is_open_trade_input(input), "{input}");
        }
    }

    #[test]
    fn test_is_asking_name() {
        for input in [
            "你叫什麼名字",
            "請問貴姓？",
            "不知如何稱呼？",
            "你是誰",
            "敢問尊姓大名",
            "請問芳名",
        ] {
            assert!(is_asking_name(input), "{input}");
        }
        for input in ["你好", "今天天氣真好", "差事", "買東西"] {
            assert!(!is_asking_name(input), "{input}");
        }
    }

    #[test]
    fn test_resolve_npc_real_name_fallback() {
        let ch: Character =
            serde_json::from_str(r#"{"id":"npc_dk1wcrabdr6f","kind":"npc"}"#).unwrap();
        let name = resolve_npc_real_name(&ch);
        assert!(!name.is_empty());
        assert!(!name.starts_with("npc_"));
    }

    #[test]
    fn test_infer_npc_obligation() {
        let (kind1, subj1) = infer_npc_obligation("鐵匠阿茂", "正揮錘淬鍊劍胚");
        assert_eq!(kind1, "fuel_shortage");
        assert!(subj1.contains("硬木白炭袋"));

        let (kind2, subj2) = infer_npc_obligation("溫郎中", "在百草診所細心切脈");
        assert_eq!(kind2, "deliver_goods");
        assert!(subj2.contains("山野草藥"));

        let (kind3, subj3) = infer_npc_obligation("老漁夫", "在渡口泊舟修補漁網");
        assert_eq!(kind3, "repair_tool");
        assert!(subj3.contains("水密熟船膠"));

        let (kind4, subj4) = infer_npc_obligation("林邊採集者", "漫步林間");
        assert_eq!(kind4, "deliver_goods");
        assert!(subj4.contains("乾燥柴枝束"));
    }

    #[test]
    fn test_delivery_and_offer_utterance_matching() {
        assert!(is_delivery_intent_utterance("我帶來了木板"));
        assert!(is_delivery_intent_utterance("交給你這束柴"));
        assert!(is_delivery_intent_utterance("拿去用吧"));
        assert!(is_delivery_intent_utterance("幫你修修"));
        assert!(is_delivery_intent_utterance("這包藥材給你"));
        assert!(!is_delivery_intent_utterance("今天天氣真好"));
        assert!(!is_delivery_intent_utterance("你好啊"));

        assert!(is_explicit_offer_utterance("這些給你"));
        assert!(is_explicit_offer_utterance("交給你收著"));
        assert!(is_explicit_offer_utterance("我來幫你修"));
        assert!(is_explicit_offer_utterance("這束柴拿去"));
        assert!(is_explicit_offer_utterance("這塊木料合用嗎"));
        assert!(is_explicit_offer_utterance("這把柴拿去用"));
        assert!(!is_explicit_offer_utterance("請問這裡是哪裡"));
    }

    #[test]
    fn test_accept_utterance_matching() {
        assert!(is_accept_material_utterance("能借我一束柴嗎？"));
        assert!(is_accept_material_utterance("借一塊木板用用"));
        assert!(is_accept_material_utterance("討點柴火"));
        assert!(!is_accept_material_utterance("你好"));
        assert!(!is_accept_material_utterance("借問一下現在幾點了"));

        assert!(is_accept_comfort_utterance("身上好冷走得好累"));
        assert!(is_accept_comfort_utterance("受了傷好渴"));
        assert!(is_accept_comfort_utterance("救命啊"));
        assert!(!is_accept_comfort_utterance("今天好熱鬧"));
    }

    #[test]
    fn test_select_deliverable_item() {
        // 1. 玩家持有柴與板，但言談指定木板
        let item1 = select_deliverable_item(
            "這塊木板給你",
            None,
            |id| matches!(id, "mat_kindling_bundle" | "mat_plank_board"),
        );
        assert_eq!(item1, Some(("mat_plank_board", "樟松木板料")));

        // 2. 言談未指明，但待辦提示急需白炭
        let item2 = select_deliverable_item(
            "我來幫手",
            Some("急需硬木白炭袋添爐"),
            |id| matches!(id, "mat_charcoal_sack" | "mat_iron_scrap"),
        );
        assert_eq!(item2, Some(("mat_charcoal_sack", "硬木白炭袋")));

        // 3. 言談與待辦均未指定，依優先順序回傳
        let item3 = select_deliverable_item(
            "給你",
            None,
            |id| matches!(id, "mat_kindling_bundle" | "craft_wooden_tool_handle"),
        );
        assert_eq!(item3, Some(("craft_wooden_tool_handle", "堅韌硬木工具長柄")));

        // 4. 無任何持有物
        let item4 = select_deliverable_item("給你", None, |_| false);
        assert_eq!(item4, None);
    }

    #[test]
    fn test_forbidden_words_list_includes_legacy_market_terms() {
        assert!(FORBIDDEN_WORDS.contains(&"短市"));
        assert!(FORBIDDEN_WORDS.contains(&"小市"));
        assert!(FORBIDDEN_WORDS.contains(&"\u{65b9}\u{624d}"));
        assert!(FORBIDDEN_WORDS.contains(&"頃刻"));
        assert!(FORBIDDEN_WORDS.contains(&"青籠關"));
        assert!(FORBIDDEN_WORDS.contains(&"110V"));
        assert!(FORBIDDEN_WORDS.contains(&"念場"));
        assert!(FORBIDDEN_WORDS.contains(&"富態"));
    }

    #[test]
    fn test_effective_npc_disposition_and_trust_defaults() {
        let (disp, trust) = effective_npc_disposition_and_trust("test_npc_default_1", "test_player_default_1");
        assert_eq!(disp, 0);
        assert_eq!(trust, 0);
    }

    #[test]
    fn test_three_baton_mentorship_refuse_and_guidance() {
        let pid = format!(
            "p_test_baton_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM entity_lineage_imprints WHERE entity_id = $1", &[&pid]);
        }

        let ch: Character = serde_json::from_str(r#"{"id":"npc_smith_test","kind":"npc","display_title":"鍛造鐵匠","current_activity":"正揮錘打鐵"}"#).unwrap();
        // 1. 非請益文字 -> 回傳 None
        let res1 = handle_three_baton_mentorship_and_obligation(&pid, &ch, "今天天氣不錯", NegotiationStance::Accept, 1000);
        assert!(res1.is_none());

        // 2. 請益文字但在 Refuse 立場下 -> 回傳 None
        let res2 = handle_three_baton_mentorship_and_obligation(&pid, &ch, "請師傅指點錘法", NegotiationStance::Refuse, 1001);
        assert!(res2.is_none());

        // 3. 請益文字且在 Accept 立場下 -> 進入第 1 次師承指點（Baton 1/2/3）
        let res3 = handle_three_baton_mentorship_and_obligation(&pid, &ch, "請師傅指點錘法", NegotiationStance::Accept, 1002);
        assert!(res3.is_some());
        let note3 = res3.unwrap();
        assert!(note3.contains("【行家指點・師承烙印】"));
        assert!(note3.contains("岑氏鍛坊真傳"));
        assert!(note3.contains("岑氏沉腕落錘法"));

        // 4. 連續第 2 次請益 -> 觸發 newly_gifted 親筆授譜
        let res4 = handle_three_baton_mentorship_and_obligation(&pid, &ch, "在下再次請益要訣", NegotiationStance::Accept, 1003);
        assert!(res4.is_some());
        let note4 = res4.unwrap();
        assert!(note4.contains("【親筆授譜】"));
        assert!(note4.contains("《岑氏沉腕落錘法手札》"));

        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM entity_lineage_imprints WHERE entity_id = $1", &[&pid]);
        }
    }
    #[test]
    fn test_qin_shuisheng_watermill_mentorship() {
        let pid = "test_player_qin_mentor";
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM entity_lineage_imprints WHERE entity_id = $1", &[&pid]);
        }

        let ch: Character = serde_json::from_str(r#"{"id":"秦水生","kind":"npc","display_title":"老堰頭水輪石磨首座|秦水生","current_activity":"巡看老堰頭水輪石磨"}"#).unwrap();
        let res = handle_three_baton_mentorship_and_obligation(&pid, &ch, "請師傅指點石磨要訣", NegotiationStance::Accept, 2000);
        assert!(res.is_some());
        let note = res.unwrap();
        assert!(note.contains("【行家指點・師承烙印】"));
        assert!(note.contains("秦氏水輪石磨真傳"));
        assert!(note.contains("老堰頭水輪分動與雙疊石磨聽音辨距法"));
        assert!(note.contains("順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性"));

        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM entity_lineage_imprints WHERE entity_id = $1", &[&pid]);
        }
    }


    #[test]
    fn test_build_player_material_context_queries_dynamic_settlement_quote() {
        let inv = crate::db::inventory::InventorySnapshot {
            items: vec![
                crate::db::inventory::InventoryLine {
                    item_id: "mat_rough_ore".to_string(),
                    name: "粗礦".to_string(),
                    item_type: "material".to_string(),
                    qty: 2,
                    weight: 1.0,
                    sub_total: 2.0,
                    description: String::new(),
                    slot: String::new(),
                    origin: "紅土崁".to_string(),
                },
            ],
            current_weight: 2.0,
            max_weight: 50.0,
        };
        let ctx = build_player_material_context(&inv, "文曜");
        assert!(ctx.contains("粗礦"));
        assert!(ctx.contains("行情每份"));
        assert!(!ctx.contains("庫存 "), "must not expose raw inventory fraction: {ctx}");
        assert!(ctx.contains("鎂"));
    }

    #[test]
    fn test_evaluate_npc_negotiation_stance_quadrants() {
        let empty_inv = crate::db::inventory::InventorySnapshot {
            items: vec![],
            current_weight: 0.0,
            max_weight: 50.0,
        };
        // 1. Debt > 80 leads to Refuse
        crate::npc::debt_interaction::try_credit_purchase("test_debtor_stance_1", "test_creditor_1", "item_debt_test", 100, 0, 50, 10);
        let (stance, desc) = evaluate_npc_negotiation_stance("test_debtor_stance_1", "test_npc_1", "文曜", &empty_inv);
        assert_eq!(stance, NegotiationStance::Refuse);
        assert!(desc.contains("冷淡婉拒"));

        // Clear debt
        crate::npc::debt_interaction::settle_debt_for_actor("test_debtor_stance_1", 200);

        // 2. Settlement has script or shortage leads to Bargain
        let (stance, desc) = evaluate_npc_negotiation_stance("test_neutral_player_1", "test_neutral_npc_1", "文曜", &empty_inv);
        assert!(matches!(stance, NegotiationStance::Bargain | NegotiationStance::Deflect));
        assert!(desc.contains("交涉立場："));
    }

    #[test]
    fn test_build_cognitive_talk_reply() {
        let mut ch: Character =
            serde_json::from_str(r#"{"id":"npc_test_cog","kind":"npc"}"#).unwrap();
        ch.current_activity = "正揮錘淬鍊劍胚".to_string();
        ch.thoughts.push(crate::entity::Thought {
            source: "Work".to_string(),
            delta: 5,
            created_at: crate::game::now_unix(),
            decay_minutes: 180,
            flavor: Some("這柄長劍開刃得格外均勻".to_string()),
        });

        // 1. 詢問姓名回傳 None 由 canonical 接手
        assert!(build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "你叫什麼名字", &ch.current_activity, NpcStanceChoice::Accept, &[]).is_none());

        // 2. 詢問在做什麼／想什麼，結合活動與心思
        let (_, rep1) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "你在做什麼？", &ch.current_activity, NpcStanceChoice::Accept, &[]).unwrap();
        assert!(rep1.contains("揮錘淬鍊劍胚") || rep1.contains("這柄長劍開刃得格外均勻"));

        // 3. 詢問難處且身負義務
        let obs = vec!["急需硬木白炭袋以供鍛爐".to_string()];
        let (_, rep2) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "有何難處？", &ch.current_activity, NpcStanceChoice::Accept, &obs).unwrap();
        assert!(rep2.contains("急需硬木白炭袋以供鍛爐"));

        // 4. 寒暄問候與打聽近況
        let (_, rep3) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "寒暄問候", &ch.current_activity, NpcStanceChoice::Accept, &[]).unwrap();
        assert!(!rep3.is_empty());
        let (_, rep4) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "打聽近況", &ch.current_activity, NpcStanceChoice::Accept, &[]).unwrap();
        assert!(!rep4.is_empty());

        // 5. clean_act 過濾過度贅字
        let transit_act = "正動身前往【文曜・百鍛鐵工坊】，手腳已是熟門熟路地照料爐火";
        let (_, rep5) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "你在做什麼？", transit_act, NpcStanceChoice::Accept, &[]).unwrap();
        assert!(rep5.contains("照料爐火"));
        let transit_road = "正動身前往【南哨・食堂】賒碗清湯";
        let (_, rep6) = build_cognitive_talk_reply(&ch, "鐵匠", "鐵坊", "你在做什麼？", transit_road, NpcStanceChoice::Accept, &[]).unwrap();
        assert!(rep6.contains("趕路辦點差事"));
    }

    #[test]
    fn test_universal_entity_live_talk_context_and_cognitive_reply() {
        let mut ch: Character =
            serde_json::from_str(r#"{"id":"npc_tailor_test","kind":"npc","display_title":"布莊掌事"}"#).unwrap();
        ch.current_activity = "正在布莊整理綢緞".to_string();
        ch.inventory = r#"[{"item_id":"craft_wooden_tool_handle","qty":2},{"item_id":"wt-good-012","qty":4}]"#.to_string();
        ch.equipment_slots = r#"{"hold_r":"craft_wooden_tool_handle"}"#.to_string();

        // 設置器物磨耗與磨損
        let _ = crate::pg::living_world::sharpen_vessel_at_grindstone(&ch.id, "craft_wooden_tool_handle", crate::game::now_unix());
        crate::pg::living_world::record_vessel_use(&ch.id, "craft_wooden_tool_handle", 2, 5, crate::game::now_unix());

        // 1. 驗證 build_universal_entity_live_talk_context 產出隨身器物與行囊實況
        let ctx = build_universal_entity_live_talk_context(&ch);
        assert!(ctx.contains("【隨身器物與行囊實況】"), "ctx: {ctx}");
        assert!(!ctx.contains("wt-good-012"), "ctx: {ctx}");
        assert!(ctx.contains("手持【堅韌硬木工具長柄】"), "ctx: {ctx}");
        assert!(ctx.contains("磨損值 2/3"), "ctx: {ctx}");

        // 2. 驗證 build_cognitive_talk_reply 引用真實持有庫存與磨損
        let (_, rep_what) = build_cognitive_talk_reply(
            &ch,
            "布莊掌事",
            "布莊",
            "你在做什麼？",
            &ch.current_activity,
            NpcStanceChoice::Accept,
            &[],
        ).unwrap();
        assert!(rep_what.contains("隨身收納有") || rep_what.contains("磨損頗重"), "rep: {rep_what}");

        let (_, rep_trouble) = build_cognitive_talk_reply(
            &ch,
            "布莊掌事",
            "布莊",
            "有何難處？",
            &ch.current_activity,
            NpcStanceChoice::Accept,
            &[],
        ).unwrap();
        assert!(rep_trouble.contains("磨損頗重") || rep_trouble.contains("隨身這批"), "rep: {rep_trouble}");

        let (_, rep_trade) = build_cognitive_talk_reply(
            &ch,
            "布莊掌事",
            "布莊",
            "收購",
            &ch.current_activity,
            NpcStanceChoice::Accept,
            &[],
        ).unwrap();
        assert!(rep_trade.contains("隨身正收存有") || rep_trade.contains("磨損頗重"), "rep: {rep_trade}");
    }

    #[test]
    fn test_mastered_lineage_imprint_passed_to_other_entities() {
        let pid = format!(
            "p_test_imprint_pass_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let mentor_id = format!("mentor_{pid}");

        // 該實體已習得並精通特定師承
        let (_rec, _) = crate::pg::living_world::record_mentor_imprint_progress(
            &mentor_id,
            "老獵戶長老",
            "順紋走刀破隙訣",
            "運刃順骨縫游走",
            crate::game::now_unix(),
        );
        // 設定進度 >= 3
        let (_rec2, _) = crate::pg::living_world::record_mentor_imprint_progress(
            &mentor_id,
            "老獵戶長老",
            "順紋走刀破隙訣",
            "運刃順骨縫游走",
            crate::game::now_unix(),
        );
        let (_rec3, _) = crate::pg::living_world::record_mentor_imprint_progress(
            &mentor_id,
            "老獵戶長老",
            "順紋走刀破隙訣",
            "運刃順骨縫游走",
            crate::game::now_unix(),
        );

        let mentor_ch: Character = serde_json::from_str(&format!(
            r#"{{"id":"{mentor_id}","kind":"npc","display_title":"獵戶領隊","current_activity":"巡山"}}"#
        )).unwrap();

        let res = handle_three_baton_mentorship_and_obligation(&pid, &mentor_ch, "請師傅指點走刀要訣", NegotiationStance::Accept, 5000);
        assert!(res.is_some());
        let note = res.unwrap();
        assert!(note.contains("順紋走刀破隙訣"), "note: {note}");
    }

    #[test]
    fn test_build_universal_entity_live_talk_context_venue_and_scripts() {
        let ch: Character = serde_json::from_str(
            r#"{
                "id": "npc_mill_clerk",
                "kind": "npc",
                "display_title": "老堰頭・水輪石磨工坊掌事",
                "current_activity": "在石磨前當值櫃檯內側磨米",
                "equipment_slots": "{\"body\":\"店員制服\"}"
            }"#,
        )
        .unwrap();
        let ctx = build_universal_entity_live_talk_context(&ch);
        assert!(
            ctx.contains("當值狀態：身著工作制服、站立於【水輪石磨工坊】當值櫃檯（非本店人員禁止進入區）內側，職司【當值石磨匠師】"),
            "context must include on-duty uniform and counter zone status: {ctx}"
        );
        assert!(
            !ctx.contains("場上當務：") && !ctx.contains("現場差事"),
            "context must not duplicate room event pack which is provided by talk.rs/identity.rs: {ctx}"
        );
        assert!(
            !ctx.contains("事件包") && !ctx.contains("劇本包") && !ctx.contains("/3 幕"),
            "context must not contain meta-design terms: {ctx}"
        );
    }
}

#[cfg(test)]
mod tests_dialogue_guards {
    use super::*;

    #[test]
    fn classifies_trade_vs_casual_or_time_questions() {
        assert!(!is_trade_or_errand_talk_topic("現在幾點了"));
        assert!(!is_trade_or_errand_talk_topic("天還沒黑？"));
        assert!(!is_trade_or_errand_talk_topic("是天還沒亮吧？"));
        assert!(!is_trade_or_errand_talk_topic("你確定？現在幾點？"));
        assert!(!is_trade_or_errand_talk_topic("是，你很漂亮"));
        assert!(!is_trade_or_errand_talk_topic("你有男朋友嗎"));
        assert!(!is_trade_or_errand_talk_topic("所以單身？"));
        assert!(!is_trade_or_errand_talk_topic("沒有欸"));
        assert!(is_trade_or_errand_talk_topic("手邊有什麼貨可以換嗎？"));
        assert!(is_trade_or_errand_talk_topic("有什麼差事需要幫忙？"));
    }

    #[test]
    fn classifies_player_talk_intents_comprehensively() {
        assert_eq!(classify_player_talk_intent("寒暄問候"), PlayerTalkIntent::GreetingOrSocial);
        assert_eq!(classify_player_talk_intent("（搭話）"), PlayerTalkIntent::GreetingOrSocial);
        assert_eq!(classify_player_talk_intent("你好啊"), PlayerTalkIntent::GreetingOrSocial);
        assert_eq!(classify_player_talk_intent("打聽近況"), PlayerTalkIntent::LocalLore);
        assert_eq!(classify_player_talk_intent("請教差事"), PlayerTalkIntent::TradeOrErrand);
        assert_eq!(classify_player_talk_intent("我想買點木柴"), PlayerTalkIntent::TradeOrErrand);
        assert_eq!(classify_player_talk_intent("手邊有什麼活計或差事？"), PlayerTalkIntent::TradeOrErrand);
        assert_eq!(classify_player_talk_intent("請問現在幾點了？"), PlayerTalkIntent::TimeOrEnvironment);
        assert_eq!(classify_player_talk_intent("天還沒黑呢"), PlayerTalkIntent::TimeOrEnvironment);
        assert_eq!(classify_player_talk_intent("碼頭在哪裡？"), PlayerTalkIntent::TimeOrEnvironment);
        assert_eq!(classify_player_talk_intent("前輩可否指點一下手藝？"), PlayerTalkIntent::Mentorship);
        assert_eq!(classify_player_talk_intent("想向先生請益功法"), PlayerTalkIntent::Mentorship);
    }

    #[test]
    fn detects_and_strips_repeated_barter_tails() {
        let recent = vec![(
            "寒暄問候".to_string(),
            "「哎呀，這陣子忙得沒空見面，正好，手頭正缺浸油麻繩，你有嗎？」".to_string(),
        )];
        assert!(has_recent_barter_or_obligation_mention(&recent, &[]));
        let stripped = strip_repeated_barter_tail(
            "「哪會啊，天還沒亮透呢！只是我得趕快去補貨，不然明天鋪子就沒東西賣了。」",
            &recent,
            &[],
        );
        assert_eq!(stripped, "「哪會啊，天還沒亮透呢！」");
        let stripped2 = strip_repeated_barter_tail(
            "「這種事問我做什麼？倒是你有浸油麻繩嗎？拿來跟我換點手札吧！」",
            &recent,
            &[],
        );
        assert_eq!(stripped2, "「這種事問我做什麼？」");
        let stripped3 = strip_repeated_barter_tail_simple(
            "「哪會啊，天還沒亮透呢！只是我得趕快去補貨，不然明天鋪子就沒東西賣了。」",
        );
        assert_eq!(stripped3, "「哪會啊，天還沒亮透呢！」");

        // 單句逗號連接的完整論述或條件句（如「重典是重典，但…」「見兇器更得喊，…」「喊了若被攻擊，…」）絕不可在逗號處被截斷
        let recent_patrol = vec![
            (
                "亂世用重典".to_string(),
                "「重典是重點，但街坊互相照應，街市才真安穩。」".to_string(),
            ),
            (
                "大盜呢".to_string(),
                "「大盜？見著就喊，大夥一齊圍住，再等巡更的來人。」".to_string(),
            ),
        ];
        for full_line in [
            "「重典是重典，但還是得靠街坊們互相照應，才能讓這街市安穩。」",
            "「見兇器更得喊，大夥一齊圍住，再等巡更的來人。」",
            "「喊了若被攻擊，見兇器先躲開，喊人圍住，等巡更的來處理。」",
        ] {
            assert_eq!(
                strip_repeated_barter_tail(full_line, &recent_patrol, &[]),
                full_line
            );
        }
    }

    #[test]
    fn detects_dynamic_ngram_repetition_from_recent_turns() {
        let recent = vec![(
            "打聽近況".to_string(),
            "「今日海風甚急，得先去加固浮橋繩索。」".to_string(),
        )];
        let stripped = strip_repeated_barter_tail(
            "「時辰尚早。得先去加固浮橋繩索，免得生事。」",
            &recent,
            &[],
        );
        assert_eq!(stripped, "「時辰尚早。」");

        let with_action_prefix = strip_repeated_barter_tail(
            "提著紫竹提籃的手稍微鬆開，側過身望向你道：「正打算去大食堂把帳冊核對完。」",
            &[(
                "寒暄問候".to_string(),
                "提著紫竹提籃快步走過石橋，笑說：「忙完一陣了，正打算去大食堂吃點熱的。」".to_string(),
            )],
            &[],
        );
        assert_eq!(
            with_action_prefix,
            "提著紫竹提籃的手稍微鬆開，側過身望向你道：「正打算去大食堂把帳冊核對完。」"
        );
    }
    #[test]
    fn formats_game_time_prompt_context_accurately_for_predawn_and_day() {
        let predawn = crate::store::format_game_time_prompt_context(2, 38);
        assert!(predawn.contains("02:38"));
        assert!(predawn.contains("天還沒亮"));
        let midday = crate::store::format_game_time_prompt_context(12, 15);
        assert!(midday.contains("12:15"));
        assert!(midday.contains("大白天"));
    }

    #[test]
    fn formats_bi_temporal_memory_anchors_and_schedule_sense() {
        let (epoch, scale) = crate::store::game_time_cfg();
        let scale = if scale <= 0.0 { 24.0 } else { scale };
        let game_sec_now = 10 * 86400 + 12 * 3600;
        let base_now = epoch + (game_sec_now as f64 / scale).round() as i64;
        let now = base_now;

        assert!(crate::store::format_game_calendar_stamp(base_now).starts_with("開譜曆 "));

        // 1. intra-session (30s ago)
        let anchor_30s = crate::store::format_memory_temporal_anchor(now - 30, now);
        assert!(anchor_30s.contains("本場對話・約 30 秒前"));
        let short_30s = crate::store::format_short_relative_time(now - 30, now);
        assert!(short_30s.starts_with("[片刻前"));

        // 2. same-day (2h in game time: 7200 game seconds ago)
        let game_sec_2h_ago = game_sec_now - 7200;
        let real_2h_ago = epoch + (game_sec_2h_ago as f64 / scale).round() as i64;
        let anchor_same_day = crate::store::format_memory_temporal_anchor(real_2h_ago, now);
        assert!(anchor_same_day.contains("本場交談・約 5 分鐘前") || anchor_same_day.contains("今日稍早"));
        let short_same_day = crate::store::format_short_relative_time(real_2h_ago, now);
        assert!(short_same_day.contains("約5分鐘前") || short_same_day.contains("今日稍早"));

        let game_sec_evening = 10 * 86400 + 23 * 3600;
        let real_evening = epoch + (game_sec_evening as f64 / scale).round() as i64;
        let game_sec_morning = 10 * 86400 + 1 * 3600;
        let real_morning = epoch + (game_sec_morning as f64 / scale).round() as i64;
        let anchor_morning = crate::store::format_memory_temporal_anchor(real_morning, real_evening);
        assert!(anchor_morning.contains("今日稍早"));
        let short_morning = crate::store::format_short_relative_time(real_morning, real_evening);
        assert!(short_morning.contains("今日稍早"));

        // 3. cross-day (2 game days ago)
        let game_sec_2d_ago = game_sec_now - 2 * 86400;
        let real_2d_ago = epoch + (game_sec_2d_ago as f64 / scale).round() as i64;
        let cross_day = crate::store::format_memory_temporal_anchor(real_2d_ago, now);
        assert!(cross_day.contains("2個遊戲日前"));
        let short_2d = crate::store::format_short_relative_time(real_2d_ago, now);
        assert!(short_2d.contains("2日前"));

        // 4. schedule window sense
        let sched_sense = crate::store::format_schedule_window_sense(9, 17, 12, 0);
        assert!(sched_sense.contains("09:00–17:00"));
        assert!(sched_sense.contains("已持續約 3 小時"));
        assert!(sched_sense.contains("距 17:00 換班／收工尚有約 5 小時"));

        let sched_sense_wrap = crate::store::format_schedule_window_sense(19, 7, 21, 30);
        assert!(sched_sense_wrap.contains("19:00–07:00"));
        assert!(sched_sense_wrap.contains("已持續約 3 小時"));
    }
}
