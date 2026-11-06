
struct GeneratedDialogue {
    line_a: String,
    line_b: String,
    anchors: Vec<String>,
    topic_hint: String,
    chosen_topic_id: String,
    topic_source: String,
    anchors_used: Vec<String>,
    dialogue_score_ptr: Option<crate::ai::DialogueScoreDetail>,
    kotodama_plan: Option<crate::npc::causal_core::DyadicKotodamaPlan>,
    room_name: String,
    room_zone: String,
    dyad: Option<crate::store::types::NpcDyad>,
    npc_npc_memory: String,
}

#[allow(clippy::too_many_arguments)]
fn conduct_npc_dialogue(
    session_store: &SessionStore,
    cfg: &Server,
    room_id: &str,
    topic_hint_in: &str,
    game_hour: i32,
    now_unix: i64,
    pair: &mut SelectedPair<'_>,
) -> Option<GeneratedDialogue> {
    let a = pair.a;
    let b = pair.b;
    let name_a = &pair.name_a;
    let name_b = &pair.name_b;
    let best_score = pair.best_score;
    let best_thread = &mut pair.best_thread;
    let mut rng = rand::rng();

    let room_name = player_layer_room_name(room_id);
    let time_label = crate::store::format_game_calendar_stamp(now_unix);
    let backstory_a = db::build_third_person_actor_card(&a.id, "先開口角色甲", &name_b);
    let backstory_b = db::build_third_person_actor_card(&b.id, "回應角色乙", &name_a);
    let mut npc_npc_memory = get_npc_npc_conversation_summary(&a.id, &b.id);
    let dyad = db::get_npc_npc_dyad(&a.id, &b.id).ok().flatten();
    if let Some(d) = &dyad
        && d.updated_at > 0
        && !npc_npc_memory.trim().is_empty()
    {
        let last_talk_tag = crate::store::format_memory_temporal_anchor(d.updated_at, now_unix);
        npc_npc_memory = format!("【上次交談時間：{last_talk_tag}】{npc_npc_memory}");
    }
    let mut topic_source = "input_or_thread".to_string();
    let mut topic_hint = topic_hint_in.to_string();
    let mut chosen_topic_id = String::new();
    if topic_hint.is_empty()
        && let Some(t) = &mut *best_thread
        && !t.topic_type.is_empty()
    {
        if t.topic_type.contains("低飽和微光") || t.topic_type.contains("沃土風") || t.topic_type.contains("非核冬天") {
            t.topic_type = "天候與環境：海霧、陣雨、山風微寒或晴暖水氣等半島日常天氣變化".to_string();
        } else if t.topic_type.contains("城外惡地") || t.topic_type.contains("非荒蕪末世") {
            t.topic_type = "鎮上傳聞山海古道、霜林口、山野異獸與豐茂林地動靜".to_string();
        }
        let time_tag = crate::store::format_short_relative_time(t.updated_at, now_unix);
        topic_hint = format!("{time_tag}{}", t.topic_type);
        topic_source = "thread_topic".into();
    }
    if topic_hint.is_empty() {
        let mask = topic_mask_for_room(room_id, game_hour);
        let (fam, sen, tags) = dyad
            .as_ref()
            .map(|d| (d.familiarity, d.sentiment, d.tags.clone()))
            .unwrap_or((0, 0, vec![]));
        if let Some(t) = pick_random_npc_npc_topic_for_pair(&mask, "", fam, sen, &tags) {
            topic_hint.clone_from(&t.hint);
            chosen_topic_id.clone_from(&t.id);
            topic_source = "weighted_mask_pick".into();
        }
    }
    if chosen_topic_id.is_empty() && !topic_hint.is_empty() {
        let id = find_npc_npc_topic_id_by_hint(&topic_hint);
        if !id.is_empty() {
            chosen_topic_id = id;
        }
    }
    let player_in_room = session_store.room_has_player(room_id);
    let ns = gametext::npc_social();
    let soc = &sim().npc_npc_social;
    let mut gossip_pct = soc.player_gossip_bias_percent;
    if gossip_pct > 100 {
        gossip_pct = 100;
    }
    if player_in_room
        && topic_source == "weighted_mask_pick"
        && rng.random_range(0..100) < gossip_pct
    {
        let gid = gametext::topic_id("gossip");
        if let Some(t) = get_npc_npc_topic_by_id(&gid) {
            topic_hint = t.hint.clone() + &ns.player_gossip_topic_suffix;
            chosen_topic_id = gid;
            topic_source = "player_room_gossip_bias".into();
        }
    }
    let mut slide_max = soc.recent_room_events_sliding_max;
    if slide_max <= 0 {
        slide_max = 3;
    }
    let mut recent_events = super::state::recent_room_events(room_id, slide_max as usize);
    let mut room_zone = String::new();
    let mut room_desc = String::new();
    let mut rumor_top_k = soc.rumor_top_k_default;
    if rumor_top_k <= 0 {
        rumor_top_k = 2;
    }
    if player_in_room {
        rumor_top_k = if soc.rumor_top_k_player_in_room > 0 {
            soc.rumor_top_k_player_in_room
        } else {
            3
        };
    }
    let mut room_tags_joined = String::new();
    let mut dialogue_anchor = String::new();
    if let Ok(Some(room)) = db::get_room(room_id) {
        room_zone = room.zone.clone();
        room_desc = room.description.trim().to_string();
        if !room.tags.is_empty() {
            let zh_tags: Vec<&str> = room
                .tags
                .iter()
                .filter_map(|t| match t.trim() {
                    "bridge" => Some("石橋水口"),
                    "plaza" => Some("街坊廣場"),
                    "market" => Some("熱鬧市集"),
                    "street" => Some("街巷"),
                    "outdoor" => Some("露天街頭"),
                    "forest" => Some("山林野徑"),
                    "dock" | "port" | "pier" => Some("水岸碼頭"),
                    "inn" | "tavern" => Some("食肆旅社"),
                    "clinic" => Some("醫館藥鋪"),
                    "workshop" => Some("百工坊場"),
                    other if !other.chars().all(|c| c.is_ascii()) => Some(other),
                    _ => None,
                })
                .collect();
            if !zh_tags.is_empty() {
                room_tags_joined = zh_tags.join("、");
            }
        }
        let rumors = top_npc_rumors(room_id, &room.zone, now_unix, rumor_top_k);
        for rr in rumors {
            let time_tag = crate::store::format_short_relative_time(rr.updated_at, now_unix);
            recent_events.push(format!("{}{}{}", ns.rumor_prefix, time_tag, rr.text));
            let _ = mark_rumor_used_by_text(&rr.text, now_unix);
        }
        let raw_act_a = a.current_activity.trim();
        let act_a = if raw_act_a.is_empty() || db::is_routine_physiological_thought(raw_act_a) {
            crate::store::wenyao_household_slot_for(&a.id).work_stay_activity
        } else {
            raw_act_a
        };
        let raw_act_b = b.current_activity.trim();
        let act_b = if raw_act_b.is_empty() || db::is_routine_physiological_thought(raw_act_b) {
            crate::store::wenyao_household_slot_for(&b.id).work_stay_activity
        } else {
            raw_act_b
        };
        let thought_a = a
            .thoughts
            .iter()
            .rev()
            .filter_map(|t| t.flavor.as_deref())
            .find(|f| !f.trim().is_empty() && !db::is_routine_physiological_thought(f))
            .unwrap_or("");
        let thought_b = b
            .thoughts
            .iter()
            .rev()
            .filter_map(|t| t.flavor.as_deref())
            .find(|f| !f.trim().is_empty() && !db::is_routine_physiological_thought(f))
            .unwrap_or("");
        dialogue_anchor = if !thought_a.is_empty() || !thought_b.is_empty() {
            format!(
                "雙方正分別忙於「{}」與「{}」，就手頭差事、器物成色或半島民生自然寒暄交流（心緒參考：{} / {}）",
                act_a, act_b, thought_a, thought_b
            )
        } else {
            format!("雙方正分別忙於「{}」與「{}」，就手頭差事、器物成色或半島民生自然寒暄交流", act_a, act_b)
        };
        if let Some(venue) = crate::grid::resolve_any_room_venue_object(&room_name, "")
            .filter(|v| v.venue_kind != "街巷橋樑公共空地" && v.default_host_title != "街坊居民")
        {
            dialogue_anchor.push_str(&format!(
                "；場域物件：【{}】（當值主事：{}；場內活計：{}）",
                venue.venue_kind, venue.default_host_title, venue.on_duty_fixture_activity
            ));
        }
        if (a.id.len() + b.id.len()) % 4 == 0 {
            if let Some((ev, _sp)) = crate::game::peninsula_scripts::active_dyadic_event_and_script_pack_for_room(
                &room_name,
                Some(&name_a),
                Some(&name_b),
                now_unix.unsigned_abs() as i64,
            ) {
                dialogue_anchor.push_str(&format!(
                    "；現場當務背景：【{}】{}",
                    ev.pack_id,
                    ev.event_title,
                ));
            }
        }
        let (ax, ay) = (a.x, a.y);
        let place_label = &room_name;
        if !crate::game::is_inside_settlement_urban_core(ax, ay)
            && ["草甸", "岩岸", "峭壁", "林", "谷", "澤", "沼", "灘", "山", "嶺", "洲"].iter().any(|k| place_label.contains(k))
        {
            dialogue_anchor.push_str("（注意：兩人此刻身處野外荒原山林，不可謊稱正在街角麵攤或浴堂內）");
        }
    }
    if player_in_room {
        recent_events.insert(0, ns.player_present_context_line.clone());
    }
    let ripple_exchanged = crate::npc::causal_core::propagate_ripple_between_npcs(&a.id, &name_a, &b.id, &name_b, now_unix)
        .or_else(|| crate::npc::causal_core::propagate_ripple_between_npcs(&b.id, &name_b, &a.id, &name_a, now_unix));
    let has_ripple = ripple_exchanged.is_some();
    if let Some(rf) = ripple_exchanged {
        recent_events.insert(0, format!("雙方正當面交換一樁新傳開的緊要風聲：{}", rf.prose_summary));
        if topic_hint.is_empty() {
            topic_hint = rf.prose_summary.clone();
        }
    }
    if recent_events.len() < 2 && !has_ripple {
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(d) = s.get_npc_rumor_digest()
                && !d.text.trim().is_empty()
            {
                let time_tag = crate::store::format_short_relative_time(d.updated_at, now_unix);
                recent_events.push(format!("{time_tag}{}", d.text));
            }
        }
    }
    let mut anchors_used: Vec<String> = vec![];
    if let Some(t) = &*best_thread
        && !t.anchors.is_empty()
    {
        anchors_used.extend(t.anchors.clone());
        let time_tag = crate::store::format_short_relative_time(t.updated_at, now_unix);
        recent_events.push(format!(
            "{}{}{}",
            ns.known_anchors_prefix,
            time_tag,
            t.anchors.join("；")
        ));
    }
    if !chosen_topic_id.is_empty()
        && let Some(top) = get_npc_npc_topic_by_id(&chosen_topic_id)
        && !top.lines.is_empty()
    {
        let seed = top.lines[rng.random_range(0..top.lines.len())].trim();
        if !seed.is_empty() {
            recent_events.push(format!("{}{}", ns.tone_seed_prefix, seed));
        }
    }
    if let Some(echo) = {
        let echo = session_store.player_talk_echo_for_npc_social(room_id);
        if echo.is_empty() { None } else { Some(echo) }
    } {
        recent_events.push(sprintf_s(&ns.rumor_echo_line_fmt, &[&echo]));
    }
    recent_events = dedupe_social_context_lines(&recent_events);
    let max_runes = cfg.npc_npc_quality_max_runes;
    let relation = dyad_relation_hint(dyad.as_ref());
    // 【三棒接力・第一棒】先由 System 1 決策模型 (11436, n_predict=1 logit) 根據雙方關係與 SoulSeed 性格拍板互動立場
    let (fam_score, sen_score) = dyad
        .as_ref()
        .map(|d| (d.familiarity, d.sentiment))
        .unwrap_or((10, 0));
    let soul_a = crate::db::get_personality_for_entity(&a.id)
        .or_else(|| a.soul_seed.map(crate::db::expand_soul_seed_to_personality))
        .unwrap_or(crate::db::Personality {
            boldness: 0.5,
            sensitivity: 0.5,
            orderliness: 0.5,
        });
    let mut obligations_a: Vec<String> = Vec::new();
    if let Some((kind, subj)) = crate::store::get_active_npc_obligation(&a.id) {
        if kind != "employment" {
            let kind_label = match kind.as_str() {
                "repair_tool" => "器物待修",
                "stranded_cargo" => "貨擔受阻",
                "repay_debt" => "欠款待償",
                "treat_injury" => "傷病待養",
                "restock_goods" => "鋪貨待補",
                "market_delivery" => "物產待交割",
                "supply_shortage" => "物資緊缺",
                other => other,
            };
            obligations_a.push(format!("{}:{}", kind_label, subj));
        }
    }
    let decision_a = crate::ai::decision_scorer::score_npc_interaction_stance(
        &crate::ai::decision_scorer::NpcDecisionInput {
            npc_name: &name_a,
            boldness: soul_a.boldness,
            sensitivity: soul_a.sensitivity,
            orderliness: soul_a.orderliness,
            familiarity_score: fam_score,
            balance_score: sen_score,
            open_obligations: &obligations_a,
            current_activity: &a.current_activity,
            action_kind: "npc_social_initiate",
            player_utterance: &topic_hint,
        },
    );

    let soul_b = crate::db::get_personality_for_entity(&b.id)
        .or_else(|| b.soul_seed.map(crate::db::expand_soul_seed_to_personality))
        .unwrap_or(crate::db::Personality {
            boldness: 0.5,
            sensitivity: 0.5,
            orderliness: 0.5,
        });
    let mut obligations_b: Vec<String> = Vec::new();
    if let Some((kind, subj)) = crate::store::get_active_npc_obligation(&b.id) {
        if kind != "employment" {
            let kind_label = match kind.as_str() {
                "repair_tool" => "器物待修",
                "stranded_cargo" => "貨擔受阻",
                "repay_debt" => "欠款待償",
                "treat_injury" => "傷病待養",
                "restock_goods" => "鋪貨待補",
                "market_delivery" => "物產待交割",
                "supply_shortage" => "物資緊缺",
                other => other,
            };
            obligations_b.push(format!("{}:{}", kind_label, subj));
        }
    }
    let mut decision_b = crate::ai::decision_scorer::score_npc_interaction_stance(
        &crate::ai::decision_scorer::NpcDecisionInput {
            npc_name: &name_b,
            boldness: soul_b.boldness,
            sensitivity: soul_b.sensitivity,
            orderliness: soul_b.orderliness,
            familiarity_score: fam_score,
            balance_score: sen_score,
            open_obligations: &obligations_b,
            current_activity: &b.current_activity,
            action_kind: "npc_social_respond",
            player_utterance: &topic_hint,
        },
    );

    let fam = dyad.as_ref().map(|d| d.familiarity).unwrap_or(0);
    if decision_b.choice == crate::ai::decision_scorer::NpcStanceChoice::Accept
        && fam < 30
    {
        let hash_val = (now_unix.unsigned_abs() ^ (a.id.len() as u64) ^ ((b.id.len() as u64) << 8)) % 100;
        if hash_val < 45 {
            decision_b.choice = match hash_val % 3 {
                0 => crate::ai::decision_scorer::NpcStanceChoice::Bargain,
                1 => crate::ai::decision_scorer::NpcStanceChoice::Deflect,
                _ => crate::ai::decision_scorer::NpcStanceChoice::Refuse,
            };
        }
    }

    let role_a = crate::store::npc_persistent_role_label(&a.id, &a.display_title);
    let role_b = crate::store::npc_persistent_role_label(&b.id, &b.display_title);
    let kotodama_plan = if matches!(
        decision_b.choice,
        crate::ai::decision_scorer::NpcStanceChoice::Accept
            | crate::ai::decision_scorer::NpcStanceChoice::Bargain
    ) {
        crate::npc::causal_core::plan_dyadic_kotodama_encounter(
            &a.id,
            &name_a,
            &role_a,
            a.magnesium,
            crate::store::get_active_npc_obligation(&a.id),
            &b.id,
            &name_b,
            &role_b,
            b.magnesium,
            crate::store::get_active_npc_obligation(&b.id),
            &room_name,
            fam_score,
            sen_score,
            now_unix,
        )
    } else {
        None
    };

    let has_kotodama = kotodama_plan.is_some();
    let stance_b_text = match decision_b.choice {
        crate::ai::decision_scorer::NpcStanceChoice::Accept => {
            format!("【角色乙（{name_b}）回應態度：正面回應角色甲的話，爽快應允或熱絡附和】")
        }
        crate::ai::decision_scorer::NpcStanceChoice::Bargain => {
            if has_kotodama {
                format!("【角色乙（{name_b}）回應態度：針對角色甲的提議討價還價、提出交換條件或指出實際難處（嚴禁直接說「好/行/嗯」無條件答應）】")
            } else {
                format!("【角色乙（{name_b}）回應態度：正面接住角色甲前句聊到的話題，但點出自身手頭活計的實際難處或不同看法（嚴禁直接說「好/行/嗯」順從附和，亦嚴禁無故向對方索討無關器物或零食）】")
            }
        }
        crate::ai::decision_scorer::NpcStanceChoice::Deflect => {
            format!("【角色乙（{name_b}）回應態度：針對角色甲的話簡短回應，表示手頭正忙或委婉推託（嚴禁直接說「好/行/嗯」無條件答應）】")
        }
        crate::ai::decision_scorer::NpcStanceChoice::Refuse => {
            format!("【角色乙（{name_b}）回應態度：針對角色甲的話表示不以為然、直接回絕或當面吐槽反駁（嚴禁順從附和）】")
        }
    };
    let stance_a_text = match decision_a.choice {
        crate::ai::decision_scorer::NpcStanceChoice::Accept => {
            format!("【角色甲（{name_a}）發話態度：主動問候熱絡攀談】")
        }
        crate::ai::decision_scorer::NpcStanceChoice::Bargain => {
            if has_kotodama {
                format!("【角色甲（{name_a}）發話態度：試探求助或提出差事協作】")
            } else {
                format!("【角色甲（{name_a}）發話態度：聊起手頭差事或向對方打聽日常近況（無交割事件時嚴禁無故索討或借用無關器物）】")
            }
        }
        crate::ai::decision_scorer::NpcStanceChoice::Deflect => {
            format!("【角色甲（{name_a}）發話態度：平淡寒暄、就事論事】")
        }
        crate::ai::decision_scorer::NpcStanceChoice::Refuse => {
            format!("【角色甲（{name_a}）發話態度：態度戒備、略帶疏離】")
        }
    };

    if !dialogue_anchor.is_empty() {
        dialogue_anchor.push('；');
    }
    dialogue_anchor.push_str(&stance_a_text);
    dialogue_anchor.push('；');
    dialogue_anchor.push_str(&stance_b_text);

    let style_examples = db::pick_style_examples(&a.id, 1);
    if let Some(style) = style_examples.first() {
        if !style.trim().is_empty() {
            dialogue_anchor.push_str(&format!("；角色甲口吻範例：「{}」", style.trim()));
        }
    }
    recent_events.push(dialogue_anchor);

    if let Some(plan) = &kotodama_plan {
        recent_events.push(plan.prompt_directive.clone());
        topic_hint = plan.topic_override.clone();
    }
    let ai_base_url = if !cfg.player_talk_api_base_url.is_empty() {
        &cfg.player_talk_api_base_url
    } else if !cfg.ollama_base_url.is_empty() {
        &cfg.ollama_base_url
    } else {
        "http://127.0.0.1:11434"
    };
    let ai_model = if !cfg.player_talk_api_model.is_empty() {
        &cfg.player_talk_api_model
    } else if !cfg.ollama_model.is_empty() {
        &cfg.ollama_model
    } else if !cfg.player_talk_api_model.is_empty() {
        &cfg.player_talk_api_model
    } else {
        "qwen2.5:7b"
    };
    let ai = match call_ai_talk_npc_to_npc(
        ai_base_url,
        ai_model,
        &name_a,
        &name_b,
        &backstory_a,
        &backstory_b,
        &room_name,
        &time_label,
        &recent_events,
        &relation,
        &npc_npc_memory,
        &topic_hint,
        &room_desc,
        &room_tags_joined,
        player_in_room,
    ) {
        Ok(v) => v,
        Err(_) => {
            inc_stat("fail_llm_empty");
            return None;
        }
    };
    let (mut line_a, mut line_b, anchors) = ai;
    let na = name_a.as_str();
    let nb = name_b.as_str();
    line_a = strip_npc_line_speaker_prefix(&line_a, &[na, nb]);
    line_b = strip_npc_line_speaker_prefix(&line_b, &[nb, na]);
    line_a = sanitize_npc_dialogue_line(&line_a);
    line_b = sanitize_npc_dialogue_line(&line_b);
    line_a = crate::ai::sanitize::fix_self_surname_vocative_in_pair(&line_a, na, nb);
    line_b = crate::ai::sanitize::fix_self_surname_vocative_in_pair(&line_b, nb, na);
    line_a = super::helpers::clean_npc_line(&line_a);
    line_b = super::helpers::clean_npc_line(&line_b);
    for line_ref in [&mut line_a, &mut line_b] {
        *line_ref = clean_unwanted_cai_opener(line_ref);
    }
    let anchors: Vec<String> = anchors
        .into_iter()
        .map(|a| super::helpers::clean_npc_line(&sanitize_npc_dialogue_line(&a)))
        .filter(|a| !a.is_empty())
        .collect();
    let (ok_a, reason_a) = quality_gate_npc_line(&line_a, max_runes);
    let (ok_b, reason_b) = quality_gate_npc_line(&line_b, max_runes);
    if line_a.is_empty() || line_b.is_empty() || !ok_a || !ok_b {
        inc_stat("fail_quality_gate");
        if !ok_a && !reason_a.is_empty() {
            inc_stat(&format!("fail_quality_{reason_a}"));
        }
        if !ok_b && !reason_b.is_empty() {
            inc_stat(&format!("fail_quality_{reason_b}"));
        }
        return None;
    }
    let (conflict_ok, conflict_reason) = anchor_consistency_check(&anchors_used, &line_a, &line_b);
    if !conflict_ok {
        for a in &anchors_used {
            let _ = penalize_rumor_by_text(a, now_unix, &conflict_reason);
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
            anchors_written: vec![],
            anchor_conflict: true,
            anchor_conflict_reason: conflict_reason,
            dialogue_score: None,
        });
        inc_stat("fail_anchor_conflict");
        return None;
    }
    let mut score_th = soc.default_dialogue_score_threshold;
    if score_th <= 0 {
        score_th = 35;
    }
    if cfg.npc_npc_dialogue_score_threshold > 0 {
        score_th = cfg.npc_npc_dialogue_score_threshold;
    }
    let score_disabled = cfg.npc_npc_dialogue_score_threshold < 0;
    let mut dialogue_score_ptr = None;
    tracing::info!(
        "[Dialogue Debug] {} vs {}: \n  -> {}: {}\n  -> {}: {}",
        room_name,
        topic_hint,
        name_a,
        line_a,
        name_b,
        line_b
    );
    if !score_disabled {
        let recent_lines_a = recent_npc_npc_archival_lines_for_entity(&a.id, 10);
        let recent_lines_b = recent_npc_npc_archival_lines_for_entity(&b.id, 10);
        let mut recent_lines = recent_lines_a;
        recent_lines.extend(recent_lines_b);
        let raw_ev = raw_events_for_dialogue_score(&recent_events);
        let sc = score_npc_dialogue(
            &line_a,
            &line_b,
            &room_name,
            &raw_ev,
            &topic_hint,
            &relation,
            &npc_npc_memory,
            &name_a,
            &name_b,
            &recent_lines,
        );
        dialogue_score_ptr = Some(sc.clone());
        let mut bucket = (sc.total / 10) * 10;
        bucket = bucket.clamp(0, 100);
        inc_stat(&format!("score_total_{bucket}"));
        if sc.total < score_th {
            inc_stat("fail_score_too_low");
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
                anchors_written: vec![],
                anchor_conflict: false,
                anchor_conflict_reason: String::new(),
                dialogue_score: dialogue_score_ptr.clone(),
            });
            return None;
        }
        inc_stat("score_pass");
    } else {
        inc_stat("score_disabled_skip");
    }

    Some(GeneratedDialogue {
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
    })
}

pub(crate) fn clean_unwanted_cai_opener(line: &str) -> String {
    let trimmed = line.trim();
    let is_negation = trimmed.starts_with("我才不")
        || trimmed.starts_with("我才沒")
        || trimmed.starts_with("我才懶")
        || trimmed.starts_with("才不")
        || trimmed.starts_with("才沒")
        || trimmed.starts_with("才怪");
    if !is_negation {
        if trimmed.starts_with("我才從") || trimmed.starts_with("才從") {
            if let Some(comma_pos) = trimmed.find('，') {
                let prefix = &trimmed[..comma_pos];
                let rest = trimmed[comma_pos + '，'.len_utf8()..].trim();
                if prefix.chars().count() <= 18 && rest.chars().count() >= 8 {
                    return rest.to_string();
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("我這邊才") {
            return rest.trim_start().to_string();
        } else if let Some(rest) = trimmed.strip_prefix("我才") {
            return rest.trim_start().to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests_cai_opener {
    use super::*;

    #[test]
    fn test_clean_unwanted_cai_opener() {
        assert_eq!(
            clean_unwanted_cai_opener("我才扛完六袋白米，袋子還熱"),
            "扛完六袋白米，袋子還熱"
        );
        assert_eq!(
            clean_unwanted_cai_opener("我這邊才把習字筆送完，手頭鎂幣快空了"),
            "把習字筆送完，手頭鎂幣快空了"
        );
        assert_eq!(
            clean_unwanted_cai_opener("我才從東碼頭過來，那邊正卸著白米糧船呢"),
            "那邊正卸著白米糧船呢"
        );
        assert_eq!(
            clean_unwanted_cai_opener("我才不要去呢！"),
            "我才不要去呢！"
        );
        assert_eq!(
            clean_unwanted_cai_opener("才不信你的話"),
            "才不信你的話"
        );
        assert_eq!(
            clean_unwanted_cai_opener("才怪，哪有這種事"),
            "才怪，哪有這種事"
        );
    }
}
