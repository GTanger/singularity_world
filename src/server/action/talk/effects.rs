/// 對話世界效果回傳結果
#[derive(Default, Clone)]
pub struct TalkWorldEffectOutcome {
    pub narrative_suffix: Option<String>,
    pub prompt_hint: Option<String>,
}

/// A. 透過對話交割物資以滿足 NPC 待辦需求
fn try_fulfill_npc_need_via_talk(
    conn: &WsConnection,
    pid: &str,
    target_id: &str,
    player_input: &str,
    _decision_choice: NpcStanceChoice,
    obligation_descs: &[String],
    now: i64,
    pool: Option<&crate::store::sql::DbPool>,
) -> Option<TalkWorldEffectOutcome> {
    let active_ob = crate::store::get_active_npc_obligation(target_id);
    let has_obligation = active_ob.is_some() || !obligation_descs.is_empty();
    let obligation_hint = active_ob.as_ref().map(|(_, subj)| subj.as_str());

    let (item_id, item_name) = find_deliverable_item(pid, player_input, obligation_hint)?;

    let mentions_delivery = is_delivery_intent_utterance(player_input);
    let explicit_offer = is_explicit_offer_utterance(player_input);

    let is_passive_greeting_or_query = matches!(
        player_input.trim(),
        "寒暄問候" | "打聽近況" | "請教差事" | "（搭話）"
    );
    let triggered = !is_passive_greeting_or_query
        && ((has_obligation && mentions_delivery) || explicit_offer);

    if !triggered {
        return None;
    }

    let _ = db::remove_from_inventory(pid, item_id, 1);
    let _ = db::add_to_inventory(target_id, item_id, 1);
    let resolved_ob_kind = active_ob
        .as_ref()
        .map(|(k, _)| k.clone())
        .or_else(|| {
            pool.and_then(|p| {
                crate::pg::living_world::list_open_obligations_for_entity(p, target_id)
                    .ok()
                    .and_then(|obs| {
                        obs.into_iter()
                            .find(|o| o.obligation_kind != "employment")
                            .map(|o| o.obligation_kind)
                    })
            })
        })
        .unwrap_or_default();
    crate::store::clear_active_npc_obligation(target_id, &resolved_ob_kind);
    let _ = db::transfer_magnesium(target_id, pid, 8).or_else(|_| db::add_magnesium(pid, 8));
    let _ = db::adjust_favorability(target_id, pid, 5);
    if let Some(p) = pool {
        let _ = crate::pg::living_world::record_relationship_delta(
            p,
            target_id,
            pid,
            "helped",
            2,
            "talk_obligation_fulfilled",
            now,
        );
    }
    let player_name = db::get_entity(pid)
        .ok()
        .flatten()
        .map(|e| e.display_title)
        .unwrap_or_else(|| "遊歷客友".to_string());
    let player_room = db::get_entity_room(pid).unwrap_or_default();
    let player_room_name = db::get_room_name(&player_room).unwrap_or_else(|_| player_room.clone());
    let delivered_order = if let Some((order, bounty)) = crate::npc::causal_core::deliver_stranded_cargo_for_player(pid, &player_name, target_id, &player_room_name, now) {
        crate::npc::causal_core::mutate_npc_from_player_action(
            target_id,
            pid,
            &player_name,
            &player_room_name,
            "rescue_cargo",
            &order.item_name,
            bounty,
            now,
        );
        Some((order, bounty))
    } else {
        None
    };

    push_refresh(conn, pid);

    let origin_str = crate::game::object_ecology::lookup_item_origin(item_id);
    let origin_note = if !origin_str.is_empty() {
        format!("（產自【{origin_str}】）")
    } else {
        String::new()
    };
    let cargo_extra_note = if let Some((order, bounty)) = &delivered_order {
        format!("（滯留貨單「{}」解送交付，公庫撥付賞格 {} 鎂）", order.item_name, bounty)
    } else {
        String::new()
    };

    Some(TalkWorldEffectOutcome {
        narrative_suffix: Some(format!(
            "\n【言出事成・器物交割】你從行囊取出【{item_name}】{origin_note}遞交對方，當場解決了對方手頭緊迫的待辦要務；對方神色舒展，取出八枚沉甸甸的鎂幣塞入你手中酬謝。{cargo_extra_note}"
        )),
        prompt_hint: Some(if !origin_str.is_empty() {
            format!("【物資交割】來客當面遞交了產自【{origin_str}】的【{item_name}】，解決了你手頭緊迫的待辦事項。請在回話中由衷感謝並收下該物件。")
        } else {
            format!("【物資交割】來客當面遞交了【{item_name}】，解決了你手頭緊迫的待辦事項。請在回話中由衷感謝並收下該物件。")
        }),
    })
}

/// B. 在 Bargain 立場下具現化待辦義務
fn ensure_bargain_obligation(
    target_id: &str,
    talk_name: &str,
    current_activity: &str,
) -> TalkWorldEffectOutcome {
    let ob_subject = if let Some((_, subj)) = crate::store::get_active_npc_obligation(target_id) {
        subj
    } else {
        let (ob_kind, ob_subject) = infer_npc_obligation(talk_name, current_activity);
        crate::store::open_active_npc_obligation(target_id, ob_kind, ob_subject);
        ob_subject.to_string()
    };

    TalkWorldEffectOutcome {
        narrative_suffix: Some(format!(
            "\n【立約待辦・條件交換】對方提出了明確的交換條件（{ob_subject}）——看來只要替他備齊所需的料件，這樁事便好商量。"
        )),
        prompt_hint: None,
    }
}

/// C. 在 Accept 立場下給予實質互助支援（柴木物資或溫言款待調養）
fn apply_accept_world_effect(
    conn: &WsConnection,
    pid: &str,
    player_input: &str,
    _familiarity_score: i32,
) -> Option<TalkWorldEffectOutcome> {
    if is_accept_material_utterance(player_input) {
        let _ = db::add_to_inventory(pid, "mat_kindling_bundle", 1);
        push_refresh(conn, pid);
        Some(TalkWorldEffectOutcome {
            narrative_suffix: Some(
                "\n【同道相助・物資支援】對方欣然應允，隨手從身側取出一束乾燥耐燃的【乾燥柴枝束】遞進你手裡，可供野外生火、添柴入爐或劈削精製。".to_string(),
            ),
            prompt_hint: Some(
                "【物資支援】你見來客有所需求，便欣然應允並從身側取出一束【乾燥柴枝束】相贈。請在回話中展現友善並提及交付柴枝。".to_string(),
            ),
        })
    } else if is_accept_comfort_utterance(player_input) {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 12).min(100);
                e.qi = (e.qi + 12).min(100);
            });
        }
        push_refresh(conn, pid);
        Some(TalkWorldEffectOutcome {
            narrative_suffix: Some(
                "\n【同道相助・溫言款待】對方熱情招呼你在近旁歇腳暖手，分予溫熱茶飲，你周身的疲乏與寒意隨之消散不少。".to_string(),
            ),
            prompt_hint: Some(
                "【溫言款待】你熱情招呼來客在近旁歇腳暖手並分予溫熱茶飲。請在回話中展現溫和關照之意。".to_string(),
            ),
        })
    } else {
        None
    }
}

/// 綜合執行對話世界效果（A 優先，未觸發則依 Stance 派發 B 或 C）
fn apply_talk_world_effects(
    conn: &WsConnection,
    pid: &str,
    target_id: &str,
    talk_name: &str,
    current_activity: &str,
    player_input: &str,
    decision_choice: NpcStanceChoice,
    obligation_descs: &[String],
    familiarity_score: i32,
    now: i64,
    pool: Option<&crate::store::sql::DbPool>,
) -> TalkWorldEffectOutcome {
    if let Some(outcome) = try_fulfill_npc_need_via_talk(
        conn,
        pid,
        target_id,
        player_input,
        decision_choice,
        obligation_descs,
        now,
        pool,
    ) {
        return outcome;
    }

    match decision_choice {
        NpcStanceChoice::Bargain => {
            if is_trade_or_errand_talk_topic(player_input) && obligation_descs.is_empty() {
                ensure_bargain_obligation(target_id, talk_name, current_activity)
            } else {
                TalkWorldEffectOutcome::default()
            }
        }
        NpcStanceChoice::Accept => {
            apply_accept_world_effect(conn, pid, player_input, familiarity_score)
                .unwrap_or_default()
        }
        _ => TalkWorldEffectOutcome::default(),
    }
}
fn resolve_mentor_word_id(
    registry: &crate::combat::wordplate::WordRegistry,
    candidate_id: &str,
    ch: &str,
) -> String {
    if registry.get_by_id(candidate_id).is_some() {
        return candidate_id.to_string();
    }
    if let Some(ids) = registry.get_by_char(ch) {
        if let Some(first) = ids.first() {
            return first.clone();
        }
    }
    if ch == "疾" {
        if let Some(ids) = registry.get_by_char("速") {
            if let Some(first) = ids.first() {
                return first.clone();
            }
        }
        return "su".to_string();
    }
    registry
        .ordered_elements()
        .next()
        .map(|w| w.id.clone())
        .unwrap_or_else(|| "wu".to_string())
}

/// 3 棒師承請益與交涉中繼（請益／學藝／指點／借物／議價）
pub fn handle_three_baton_mentorship_and_obligation(
    pid: &str,
    target: &Character,
    player_text: &str,
    stance: NegotiationStance,
    now: i64,
) -> Option<String> {
    // Baton 1 (Intent & Stance Gate):
    // 1. 永遠於 PostgreSQL living_world 記錄對話互動交情
    if let Some(pool) = crate::pg::pool() {
        let _ = crate::pg::living_world::upsert_relationship(
            &pool,
            &target.id,
            pid,
            "acquaintance",
            1,
            1,
            "talk_interaction",
            now,
        );
    }

    // 2. 檢測玩家是否提出百工或武藝指點請求，且 NPC 立場並非拒絕
    const GUIDANCE_KEYWORDS: &[&str] = &[
        "請益", "指點", "學藝", "手札", "刀法", "錘法", "火候", "木作", "要訣", "師傅", "教我",
    ];
    let asks_guidance = GUIDANCE_KEYWORDS.iter().any(|&kw| player_text.contains(kw));
    if !asks_guidance || stance == NegotiationStance::Refuse {
        return None;
    }

    // Baton 2 (Immediate Backend State Mutation):
    // 根據目標姓名、命途稱謂與當前活動推論師承領域
    let real_name = resolve_npc_real_name(target);
    let domain_source = format!("{}:{}:{}:{}", target.id, real_name, target.display_title, target.current_activity);

    let held_imprints = crate::pg::living_world::get_entity_lineage_imprints(&target.id);
    let has_watermill_imprint = held_imprints.iter().any(|imp| {
        imp.imprint_title.contains("水輪") || imp.imprint_title.contains("石磨")
    });
    let has_watermill_tool = target.equipment_slots.contains("wt_cat_0973")
        || target.equipment_slots.contains("wt_cat_0984");
    let is_watermill = has_watermill_imprint
        || has_watermill_tool
        || domain_source.contains("水輪")
        || domain_source.contains("石磨");
    let is_smithing = ["鐵", "鍛", "岑", "礦", "石"].iter().any(|k| domain_source.contains(k));
    let is_culinary = ["廚", "食", "大娘", "麵", "茶", "漁", "海"].iter().any(|k| domain_source.contains(k));
    let is_hunting = ["獵", "衛", "哨", "武", "刀", "巡", "羅", "趙"].iter().any(|k| domain_source.contains(k));

    let mastered_imprint = held_imprints
        .into_iter()
        .find(|imp| imp.manual_gifted || imp.interaction_count >= 3);

    let (mentor_label_str, imprint_title_str, imprint_prose_str, habit_kind, posture, raw_word_ids, raw_word_chars): (
        String,
        String,
        String,
        crate::pg::living_world::BreathHabitKind,
        &str,
        &[&str],
        &[&str],
    ) = if let Some(mastered) = mastered_imprint {
        let m_label = if is_watermill {
            if real_name.contains('秦') || target.id.contains('秦') {
                "秦氏水輪石磨真傳".to_string()
            } else if !real_name.is_empty() && real_name != "陌生人" {
                format!("{real_name}・水輪石磨真傳")
            } else {
                "水輪石磨真傳".to_string()
            }
        } else if target.kind == crate::entity::EntityKind::Player {
            real_name.clone()
        } else if !mastered.mentor_name.is_empty() {
            mastered.mentor_name.clone()
        } else {
            real_name.clone()
        };
        let m_title = mastered.imprint_title;
        let m_prose = if is_watermill
            && (mastered.imprint_prose.is_empty() || mastered.imprint_prose == "聽水辨速、循紋落鏨")
        {
            "順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性".to_string()
        } else {
            mastered.imprint_prose
        };

        let (hk, post, w_ids, w_chs): (
            crate::pg::living_world::BreathHabitKind,
            &str,
            &[&str],
            &[&str],
        ) = if m_title.contains("石磨") || m_title.contains("水輪") || is_watermill {
            (
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                "雙疊聽音",
                &["mo", "liu", "chun"],
                &["磨", "流", "純"],
            )
        } else if m_title.contains("鍛") || m_title.contains("落錘") || is_smithing {
            (
                crate::pg::living_world::BreathHabitKind::ForgeHeavy,
                "重手",
                &["gang", "chen_tai", "zhong"],
                &["\u{525b}", "沉", "重"],
            )
        } else if m_title.contains("收汁") || m_title.contains("庖廚") || is_culinary {
            (
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                "庖廚百工",
                &["shui", "huo_shou", "he_tai"],
                &["水", "火", "合"],
            )
        } else if m_title.contains("破隙") || m_title.contains("走刀") || is_hunting {
            (
                crate::pg::living_world::BreathHabitKind::ForestLong,
                "起手",
                &["ji_su", "ci", "chuan"],
                &["疾", "刺", "穿"],
            )
        } else {
            (
                crate::pg::living_world::BreathHabitKind::ChamberClear,
                "百工衡準",
                &["zhun", "ding", "he_tai"],
                &["準", "定", "合"],
            )
        };
        (m_label, m_title, m_prose, hk, post, w_ids, w_chs)
    } else if is_watermill {
        let mill_label = if real_name.contains('秦') || target.id.contains('秦') {
            "秦氏水輪石磨真傳".to_string()
        } else if !real_name.is_empty() && real_name != "陌生人" {
            format!("{real_name}・水輪石磨真傳")
        } else {
            "水輪石磨真傳".to_string()
        };
        (
            mill_label,
            "老堰頭水輪分動與雙疊石磨聽音辨距法".to_string(),
            "順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性".to_string(),
            crate::pg::living_world::BreathHabitKind::TideContinuous,
            "雙疊聽音",
            &["mo", "liu", "chun"],
            &["磨", "流", "純"],
        )
    } else if is_smithing {
        (
            "岑氏鍛坊真傳".to_string(),
            "岑氏沉腕落錘法".to_string(),
            "沉腕蓄勁、順著鐵胚紋理落錘，使器物與內息渾然相合".to_string(),
            crate::pg::living_world::BreathHabitKind::ForgeHeavy,
            "重手",
            &["gang", "chen_tai", "zhong"],
            &["\u{525b}", "沉", "重"],
        )
    } else if is_culinary {
        (
            "三水庖廚真傳".to_string(),
            "三水火候收汁訣".to_string(),
            "以綿長內息感應鼎釜冷熱交替，鎖住山海靈材本味".to_string(),
            crate::pg::living_world::BreathHabitKind::TideContinuous,
            "庖廚百工",
            &["shui", "huo_shou", "he_tai"],
            &["水", "火", "合"],
        )
    } else if is_hunting {
        (
            "山海尋隙真傳".to_string(),
            "順紋走刀破隙訣".to_string(),
            "運刃順骨縫游走、避實擊虛，手眼身步一氣呵成".to_string(),
            crate::pg::living_world::BreathHabitKind::ForestLong,
            "起手",
            &["ji_su", "ci", "chuan"],
            &["疾", "刺", "穿"],
        )
    } else {
        (
            "半島百工衡準".to_string(),
            "百工調息衡準訣".to_string(),
            "神思澄明、呼吸勻淨，於分寸之間拿捏器物重心".to_string(),
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            "百工衡準",
            &["zhun", "ding", "he_tai"],
            &["準", "定", "合"],
        )
    };
    let mentor_label = mentor_label_str.as_str();
    let imprint_title = imprint_title_str.as_str();
    let imprint_prose = imprint_prose_str.as_str();

    crate::pg::living_world::record_breath_activity(pid, habit_kind, 2, now);
    let (_imprint_rec, newly_gifted) = crate::pg::living_world::record_mentor_imprint_progress(
        pid,
        mentor_label,
        imprint_title,
        imprint_prose,
        now,
    );

    let registry = crate::combat::wordplate::WordRegistry::global();
    let mut resolved_word_ids = Vec::with_capacity(3);
    for (&wid, &ch) in raw_word_ids.iter().zip(raw_word_chars.iter()) {
        resolved_word_ids.push(resolve_mentor_word_id(registry, wid, ch));
    }

    if newly_gifted {
        let word_id_refs: Vec<&str> = resolved_word_ids.iter().map(|s| s.as_str()).collect();
        crate::server::handler::inventory::grant_words_to_player_casket(pid, &word_id_refs);

        let mut hasher = 0xcbf29ce484222325u64;
        for b in format!("{}_{}", mentor_label, imprint_title).bytes() {
            hasher ^= b as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
        }
        let manual_hash = format!("{:016x}", hasher);

        let manual_rec = crate::pg::living_world::TranscribedManualRecord {
            manual_hash: manual_hash.clone(),
            author_id: target.id.clone(),
            author_name: mentor_label.to_string(),
            formal_name: imprint_title.to_string(),
            slot_label: posture.to_string(),
            carrier_code: "tool".to_string(),
            word_ids: resolved_word_ids.clone(),
            word_chars: raw_word_chars.iter().map(|s| s.to_string()).collect(),
            origin_basis: imprint_prose.to_string(),
            created_at: now,
        };
        crate::pg::living_world::save_transcribed_manual(&manual_rec);
        let _ = crate::db::inventory::add_to_inventory(pid, &format!("manual_{manual_hash}"), 1);
    }

    // Baton 3 (Voice & Narrative Note):
    let note = if newly_gifted {
        format!(
            "\n【行家指點・師承烙印】{}傾囊相授，為你細說【{}】之奧妙，體會到：「{}」。你依循提點調整吐納，氣息周天漸入佳境。\n【親筆授譜】{}見你領悟精進、根基漸穩，當場將一冊親筆《{}手札》與其中要訣心法傳授於你！",
            mentor_label, imprint_title, imprint_prose, mentor_label, imprint_title
        )
    } else {
        format!(
            "\n【行家指點・師承烙印】{}傾囊相授，為你細說【{}】之奧妙，體會到：「{}」。你依循提點調整吐納，氣息周天漸入佳境。",
            mentor_label, imprint_title, imprint_prose
        )
    };

    Some(note)
}


pub fn build_universal_entity_live_talk_context(target: &Character) -> String {
    // 1. 裝備中器物／隨身工藝器具之磨損與磨合狀態
    let (tool_id, tool_name) = {
        let slots_map: Option<std::collections::HashMap<String, String>> =
            serde_json::from_str(&target.equipment_slots).ok();
        let held_id = slots_map.as_ref().and_then(|m| {
            m.get("hold_r").or_else(|| m.get("hold_l")).filter(|s| !s.is_empty())
        });
        if let Some(tid) = held_id {
            let tname = crate::db::get_item_name(tid);
            (tid.clone(), tname)
        } else {
            let vspec = crate::npc::causal_core::resolve_vocational_vessel(&target.display_title);
            (vspec.item_id.to_string(), vspec.display_name.to_string())
        }
    };
    let tool_desc = if !tool_id.is_empty() {
        let vessel = crate::pg::living_world::get_vessel_state(&target.id, &tool_id);
        format!(
            "手持【{}】磨合度【{}】、磨損值 {}/3（器況：{}）",
            tool_name,
            vessel.familiarity().label(),
            vessel.wear_points,
            vessel.condition.label()
        )
    } else {
        String::new()
    };

    let room_name = if let (Some(gx), Some(gy)) = (target.grid_x, target.grid_y) {
        crate::server::grid_manager::get_square_grid()
            .and_then(|g| g.get(crate::grid::SquareCoord::new(gx, gy)).map(|c| c.name.clone()))
            .unwrap_or_else(|| target.display_title.clone())
    } else {
        target.display_title.clone()
    };
    let (is_staff, has_uniform, in_counter_zone) =
        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            &room_name,
            &target.id,
            &target.display_title,
            &target.current_activity,
            &target.equipment_slots,
        );
    let venue_opt = crate::grid::resolve_any_room_venue_object(&room_name, &target.display_title)
        .filter(|v| v.venue_kind != "街巷橋樑公共空地" && v.default_host_title != "街坊居民");
    let venue_staff_desc = if is_staff && venue_opt.is_some() {
        let host_title = venue_opt
            .as_ref()
            .map(|v| v.default_host_title)
            .unwrap_or("當值人員");
        let venue_kind = venue_opt
            .as_ref()
            .map(|v| v.venue_kind)
            .unwrap_or("場館");
        if has_uniform && in_counter_zone {
            format!("當值狀態：身著工作制服、站立於【{venue_kind}】當值櫃檯（非本店人員禁止進入區）內側，職司【{host_title}】")
        } else if in_counter_zone {
            format!("當值狀態：站立於【{venue_kind}】當值櫃檯（非本店人員禁止進入區）內側，職司【{host_title}】")
        } else if has_uniform {
            format!("當值狀態：身著工作制服於【{venue_kind}】當值，職司【{host_title}】")
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mut parts = Vec::new();
    if !tool_desc.is_empty() {
        parts.push(tool_desc);
    }
    if !venue_staff_desc.is_empty() {
        parts.push(venue_staff_desc);
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("\n【隨身器物與行囊實況】：{}。", parts.join("；"))
    }
}
