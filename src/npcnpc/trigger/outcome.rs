
fn commit_and_broadcast_dialogue(
    session_store: &SessionStore,
    room_id: &str,
    now_unix: i64,
    pair: SelectedPair<'_>,
    dialogue: GeneratedDialogue,
) -> bool {
    let SelectedPair {
        a,
        b,
        name_a,
        name_b,
        best_score,
        best_thread,
        ..
    } = pair;
    let GeneratedDialogue {
        line_a,
        line_b,
        anchors,
        topic_hint,
        chosen_topic_id,
        topic_source,
        anchors_used,
        dialogue_score_ptr,
        kotodama_plan,
        room_name,
        room_zone,
        dyad,
        npc_npc_memory,
    } = dialogue;
    let ns = gametext::npc_social();
    let soc = &sim().npc_npc_social;

    if let Some(plan) = &kotodama_plan {
        crate::npc::causal_core::commit_dyadic_kotodama_plan(&a.id, &name_a, &b.id, &name_b, plan, now_unix);
    }
    let summary = sprintf_s(
        &ns.summary_fmt,
        &[
            &room_name,
            &name_a,
            &trunc_rune(&line_a, 120),
            &name_b,
            &trunc_rune(&line_b, 120),
        ],
    );
    if !should_skip_npc_npc_summary_update(&npc_npc_memory, &summary, &line_a, &line_b) {
        let _ = set_npc_npc_conversation_summary(&a.id, &b.id, &summary);
    } else {
        inc_stat("skip_npc_npc_summary_dup");
    }
    let arch_a = sprintf_s(&ns.archival_with_fmt, &[&name_b, &trunc_rune(&line_a, 120)]);
    let arch_b = sprintf_s(&ns.archival_with_fmt, &[&name_a, &trunc_rune(&line_b, 120)]);
    if let Ok((_, true)) = insert_npc_npc_dialogue_archival(&a.id, &arch_a) {
        inc_stat("skip_npc_npc_archival_dup");
    }
    if let Ok((_, true)) = insert_npc_npc_dialogue_archival(&b.id, &arch_b) {
        inc_stat("skip_npc_npc_archival_dup");
    }
    let mut thread = best_thread.unwrap_or_else(|| NpcThread {
        thread_key: format!("{}|{}", a.id.as_str().min(b.id.as_str()), a.id.as_str().max(b.id.as_str())),
        topic_type: topic_hint.clone(),
        phase: "opening".into(),
        anchors: vec![],
        turn_count: 0,
        cooldown_until: 0,
        updated_at: 0,
    });
    thread.turn_count += 1;
    thread.updated_at = now_unix;
    if thread.topic_type.is_empty() {
        thread.topic_type.clone_from(&topic_hint);
    }
    if thread.topic_type.contains("低飽和微光") || thread.topic_type.contains("沃土風") || thread.topic_type.contains("非核冬天") {
        thread.topic_type = "天候與環境：海霧、陣雨、山風微寒或晴暖水氣等半島日常天氣變化".to_string();
    } else if thread.topic_type.contains("城外惡地") || thread.topic_type.contains("非荒蕪末世") {
        thread.topic_type = "鎮上傳聞山海古道、霜林口、山野異獸與豐茂林地動靜".to_string();
    }
    if thread.turn_count >= 1 {
        thread.phase = "elaborate".into();
    }
    if thread.turn_count >= 2 {
        thread.phase = "cooling".into();
        thread.cooldown_until = now_unix + 600;
    }
    if kotodama_plan.as_ref().is_some_and(|p| p.keep_thread_open) {
        thread.phase = "awaiting_fulfillment".into();
        thread.cooldown_until = now_unix + 300;
    }
    let is_valid_thread_anchor = |x: &String| {
        !x.contains('・')
            && x != &room_name
            && !x.starts_with("wt-")
            && !x.starts_with("grid:")
            && !matches!(
                x.as_str(),
                "清晨" | "正午" | "傍晚" | "夜裡" | "深夜" | "正晝" | "早晨" | "午後"
            )
    };
    thread.anchors = thread
        .anchors
        .into_iter()
        .map(|a| super::helpers::clean_npc_line(&sanitize_npc_dialogue_line(&a)))
        .filter(|a| !a.is_empty())
        .collect();
    thread.anchors.retain(&is_valid_thread_anchor);
    let anchors: Vec<String> = anchors.into_iter().filter(is_valid_thread_anchor).collect();
    thread.anchors = merge_thread_anchors(&thread.anchors, &anchors);
    let _ = set_npc_npc_thread(&a.id, &b.id, thread);
    let dar = &sim().dialogue_anchor_rumor;
    let mut min_r = dar.min_anchor_runes;
    if min_r <= 0 {
        min_r = 4;
    }
    let mut ss = dar.source_score;
    if ss <= 0 {
        ss = 1;
    }
    let mut w = dar.weight;
    if w <= 0 {
        w = 2;
    }
    let mut ttl = dar.ttl_sec;
    if ttl <= 0 {
        ttl = 3600;
    }
    // 迴圈會消耗 anchors；LastSocialChoice 需保留一份原始錨點清單
    let anchors_written_record = anchors.clone();
    for mut anchor in anchors {
        anchor = anchor.trim().to_string();
        if anchor.is_empty() {
            continue;
        }
        let a_low = anchor.to_lowercase();
        if anchor.chars().count() < min_r as usize
            || a_low.contains("npc_")
            || a_low.contains("entity_")
            || a_low.contains("room_")
            || a_low.contains("street")
            || a_low.contains("zone")
            || anchor.contains("飄起陣陣飛雪")
            || anchor.contains("連綿梅雨")
            || anchor.contains("開始降下陣雨")
            || anchor.contains("寒氣侵襲")
            || anchor.contains("道路變得濕滑")
            || anchor.contains("黑澤")
            || anchor.contains("青龍關")
            || anchor.contains("北冠")
            || anchor.contains("青嶺")
            || anchor.contains("白塔高地")
            || anchor.contains("西南平原")
            || anchor.contains("風船")
            || anchor.contains("天塹海峽")
            || anchor.contains("東洋海面")
        {
            continue;
        }
        let is_valid_rumor_sentence = anchor.chars().count() >= 12
            && !anchor.contains('・')
            && anchor != room_name
            && !anchor.starts_with("wt-")
            && !anchor.starts_with("grid:");
        if is_valid_rumor_sentence {
            let rid = format!("{room_id}|{room_zone}|{anchor}");
            let _ = upsert_npc_rumor(NpcRumor {
                id: rid,
                text: anchor.clone(),
                room_id: room_id.to_string(),
                zone: room_zone.clone(),
                source: "dialogue_anchor".into(),
                source_score: ss,
                weight: w,
                mention_count: 0,
                last_used_at: 0,
                blocked_until: 0,
                penalty_count: 0,
                last_penalty_at: 0,
                last_penalty_reason: String::new(),
                updated_at: now_unix,
                expires_at: now_unix + ttl,
            });
            // 寫入世界詞典
            let pair_key = format!("{}|{}", a.id, b.id);
            upsert_lexicon_term(&anchor, room_id, &pair_key);
        }
    }
    set_pair_last_talk(&a.id, &b.id, now_unix);
    let mut d = dyad.unwrap_or_else(|| {
        let (first_id, second_id) = if a.id <= b.id {
            (a.id.clone(), b.id.clone())
        } else {
            (b.id.clone(), a.id.clone())
        };
        crate::store::types::NpcDyad {
            a_id: first_id,
            b_id: second_id,
            familiarity: 0,
            sentiment: 0,
            tags: Vec::new(),
            updated_at: now_unix,
        }
    });
    let du = &sim().dyad_update;
    let mut cap = du.sentiment_cap;
    if cap <= 0 {
        cap = 100;
    }
    let delta_trust = du.familiarity_delta_per_exchange;
    d.familiarity += delta_trust;
    if d.familiarity > cap {
        d.familiarity = cap;
    }
    let delta_favor = sentiment_delta_from_dialogue(
        &topic_hint,
        &line_a,
        &line_b,
        d.familiarity,
        &chosen_topic_id,
    );
    d.sentiment += delta_favor;
    d.sentiment = d.sentiment.clamp(-cap, cap);
    if npc_pair_same_venue(&a.id, &b.id) {
        d.tags = add_tag(&d.tags, &gametext::dyad_tag("same_venue"));
    }
    let mut qmax = du.quarrel_tag_sentiment_max;
    if qmax == 0 {
        qmax = -30;
    }
    if d.sentiment <= qmax {
        d.tags = add_tag(&d.tags, &gametext::dyad_tag("had_quarrel"));
    }
    let mut cmin = du.clear_quarrel_sentiment_min;
    if cmin == 0 {
        cmin = 25;
    }
    if d.sentiment >= cmin {
        d.tags = remove_tag(&d.tags, &gametext::dyad_tag("had_quarrel"));
    }
    d.updated_at = now_unix;
    let _ = set_npc_npc_dyad(&a.id, &b.id, d);
    if let Some(pool) = crate::pg::pool() {
        let _ = crate::pg::living_world::record_relationship_delta(
            &pool,
            &a.id,
            &b.id,
            "talked",
            1,
            "npc_npc_dialogue",
            now_unix,
        );
        let _ = crate::pg::living_world::record_relationship_delta(
            &pool,
            &b.id,
            &a.id,
            "talked",
            1,
            "npc_npc_dialogue",
            now_unix,
        );
        let _ = crate::pg::living_world::adjust_relationship(
            &pool,
            &a.id,
            &b.id,
            "colleague",
            delta_trust,
            delta_favor,
        );
    }
    set_last_choice(LastSocialChoice {
        at: now_unix,
        room_id: room_id.to_string(),
        a_id: a.id.clone(),
        b_id: b.id.clone(),
        score_total: best_score,
        topic_hint: topic_hint.clone(),
        topic_id: chosen_topic_id.clone(),
        topic_reason: topic_source.clone(),
        anchors_used: anchors_used.clone(),
        anchors_written: anchors_written_record,
        anchor_conflict: false,
        anchor_conflict_reason: String::new(),
        dialogue_score: dialogue_score_ptr,
    });
    let skip_sec = Duration::from_secs(soc.broadcast_skip_recent_talk_sec.max(0) as u64);
    if session_store.room_has_player_with_recent_talk(room_id, skip_sec) {
        return true;
    }
    let want_loc = crate::db::canonical_location_key(room_id);
    let pattern_idx = (now_unix as usize / 13 + a.id.len() + b.id.len()) % 8;
    let mut sent_any = false;
    for s in session_store.all_sessions() {
        if crate::server::session::player_location_key(&s.player_id) == want_loc {
            let label_a = resolve_viewer_npc_label(&s.player_id, a, &room_name);
            let label_b = resolve_viewer_npc_label(&s.player_id, b, &room_name);
            let player_line_a = replace_private_names_for_player(
                &line_a,
                &[(&name_a, &label_a), (&name_b, &label_b)],
            );
            let player_line_b = replace_private_names_for_player(
                &line_b,
                &[(&name_a, &label_a), (&name_b, &label_b)],
            );
            let text = format_staged_npc_dialogue(
                pattern_idx,
                &label_a,
                &label_b,
                &player_line_a,
                &player_line_b,
                &a.current_activity,
                &b.current_activity,
            );
            crate::store::record_active_npc_dialogue(room_id, &a.id, &b.id, &text);
            crate::server::send_npc_dialogue_to_player(&s, &a.id, &b.id, &text);
            sent_any = true;
        }
    }
    if !sent_any {
        let (player_name_a, player_name_b) = player_layer_pair_labels(&room_name);
        let player_line_a = replace_private_names_for_player(
            &line_a,
            &[(&name_a, &player_name_a), (&name_b, &player_name_b)],
        );
        let player_line_b = replace_private_names_for_player(
            &line_b,
            &[(&name_a, &player_name_a), (&name_b, &player_name_b)],
        );
        let text = format_staged_npc_dialogue(
            pattern_idx,
            &player_name_a,
            &player_name_b,
            &player_line_a,
            &player_line_b,
            &a.current_activity,
            &b.current_activity,
        );
        crate::store::record_active_npc_dialogue(room_id, &a.id, &b.id, &text);
    }
    inc_stat("success");
    true
}
