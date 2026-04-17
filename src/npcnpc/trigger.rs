//! 同房 NPC↔NPC 觸發（對齊既有 `npcnpc/trigger`）。

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rand::Rng;

use crate::ai::{
    call_ai_talk_npc_to_npc, raw_events_for_dialogue_score, sanitize_npc_dialogue_line,
    score_npc_dialogue, strip_npc_line_speaker_prefix,
};
use crate::config::{sim, Server};
use crate::db::{
    self, build_identity, get_npc_npc_conversation_summary, insert_npc_npc_dialogue_archival,
    mark_rumor_used_by_text, penalize_rumor_by_text, recent_npc_npc_archival_lines_for_entity,
    set_npc_npc_conversation_summary, set_npc_npc_dyad, set_npc_npc_thread, top_npc_rumors, upsert_npc_rumor,
    upsert_lexicon_term,
};
use crate::entity::EntityKind;
use crate::gametext;
use crate::npc::{
    find_npc_npc_topic_id_by_hint, get_npc_npc_topic_by_id, pick_random_npc_npc_topic_for_pair,
};
use crate::server::{send_narrate_to_room, SessionStore};
use crate::store::{NpcRumor, NpcThread};

use super::helpers::{
    add_tag, anchor_consistency_check, dedupe_social_context_lines, dyad_relation_hint, merge_thread_anchors,
    npc_pair_same_venue, quality_gate_npc_line, remove_tag, sentiment_delta_from_dialogue,
    should_skip_npc_npc_summary_update, topic_mask_for_room,
};
use super::state::{get_pair_last_talk, inc_stat, set_last_choice, set_pair_last_talk};
use super::types::LastSocialChoice;

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn trunc_rune(s: &str, max: usize) -> String {
    let ch: Vec<char> = s.chars().collect();
    if ch.len() <= max {
        return s.to_string();
    }
    ch[..max].iter().collect()
}

fn sprintf_s(template: &str, args: &[&str]) -> String {
    let mut s = template.to_string();
    for a in args {
        if let Some(pos) = s.find("%s") {
            s.replace_range(pos..pos + 2, a);
        }
    }
    s
}

/// 在指定房嘗試觸發一次 NPC 間 AI 對話（對齊 `TryTriggerNpcNpcInRoom`）。
#[allow(clippy::too_many_lines)]
pub fn try_trigger_npc_npc_in_room(
    session_store: &SessionStore,
    cfg: &Server,
    room_id: &str,
    topic_hint_in: &str,
    game_hour: i32,
) -> bool {
    inc_stat("attempt");
    if cfg.ollama_base_url.is_empty() || cfg.ollama_model.is_empty() {
        inc_stat("fail_no_model");
        return false;
    }
    let cd = Duration::from_secs(sim().npc_npc_social.room_recent_player_talk_cooldown_sec.max(0) as u64);
    if session_store.room_has_player_with_recent_talk(room_id, cd) {
        inc_stat("fail_recent_player_talk");
        return false;
    }
    let Ok(entities) = db::get_entities_in_room(room_id, game_hour) else {
        inc_stat("fail_not_enough_npc");
        return false;
    };
    let npcs: Vec<_> = entities
        .into_iter()
        .filter(|e| matches!(e.kind, EntityKind::Npc))
        .collect();
    if npcs.len() < 2 {
        inc_stat("fail_not_enough_npc");
        return false;
    }
    let now_unix = now_unix();
    let mut best_a: Option<&crate::entity::Character> = None;
    let mut best_b: Option<&crate::entity::Character> = None;
    let mut best_score = i32::MIN;
    let mut best_thread: Option<NpcThread> = None;
    let pp = &sim().npc_npc_pair_pick;
    let mut base_n = pp.score_base_noise;
    if base_n <= 0 {
        base_n = 4;
    }
    let mut fam_div = pp.familiarity_score_divisor;
    if fam_div <= 0 {
        fam_div = 20;
    }
    let mut rng = rand::rng();
    for i in 0..npcs.len() {
        for j in (i + 1)..npcs.len() {
            let a = &npcs[i];
            let b = &npcs[j];
            if a.id.is_empty() || b.id.is_empty() {
                continue;
            }
            let mut thread = db::get_npc_npc_thread(&a.id, &b.id).ok().flatten();
            if let Some(ref mut t) = thread
                && t.phase != "cooling"
                && t.turn_count > 0
                && now_unix - t.updated_at > pp.thread_stale_sec as i64
            {
                t.phase = "cooling".into();
                t.cooldown_until = now_unix + pp.thread_cooldown_sec as i64;
                t.updated_at = now_unix;
                let _ = set_npc_npc_thread(&a.id, &b.id, t.clone());
            }
            if let Some(ref t) = thread
                && t.phase == "cooling"
                && t.cooldown_until > now_unix
            {
                continue;
            }
            if let Some(ref t) = thread
                && t.phase == "cooling"
                && t.cooldown_until > 0
                && t.cooldown_until <= now_unix
            {
                let _ = db::delete_npc_npc_thread(&a.id, &b.id);
                thread = None;
            }
            let mut score = rng.random_range(0..base_n);
            if let Some(ref t) = thread
                && t.phase != "cooling"
            {
                score += 100;
            }
            if let Ok(Some(dyad)) = db::get_npc_npc_dyad(&a.id, &b.id) {
                score += dyad.familiarity / fam_div;
                if dyad.sentiment <= pp.dyad_sentiment_penalty_threshold {
                    score -= pp.dyad_sentiment_penalty;
                }
                if super::helpers::has_tag(&dyad.tags, &gametext::dyad_tag("same_venue")) {
                    score += pp.same_dyad_venue_tag_bonus;
                }
            }
            if npc_pair_same_venue(&a.id, &b.id) {
                score += pp.same_venue_score_bonus;
            }
            let last = get_pair_last_talk(&a.id, &b.id);
            if last > 0 && now_unix - last < pp.pair_last_talk_penalty_sec as i64 {
                score -= pp.pair_last_talk_penalty;
            }
            let p_a = db::person_name_from_npc_list_label(&a.display_title);
            let p_b = db::person_name_from_npc_list_label(&b.display_title);
            if !p_a.is_empty() && p_a == p_b {
                score -= pp.same_display_name_penalty;
            }
            if score > best_score {
                best_score = score;
                best_a = Some(a);
                best_b = Some(b);
                best_thread = thread;
            }
        }
    }
    let (Some(a), Some(b)) = (best_a, best_b) else {
        inc_stat("fail_no_pair");
        return false;
    };
    let mut name_a = db::person_name_from_npc_list_label(&a.display_title);
    let mut name_b = db::person_name_from_npc_list_label(&b.display_title);
    if name_a.is_empty() {
        name_a.clone_from(&a.id);
    }
    if name_b.is_empty() {
        name_b.clone_from(&b.id);
    }
    let room_name = db::get_room_name(room_id).unwrap_or_default();
    let time_label = gametext::time_of_day_label(game_hour);
    let backstory_a = build_identity(&a.id);
    let backstory_b = build_identity(&b.id);
    let npc_npc_memory = get_npc_npc_conversation_summary(&a.id, &b.id);
    let dyad = db::get_npc_npc_dyad(&a.id, &b.id).ok().flatten();
    let mut topic_source = "input_or_thread".to_string();
    let mut topic_hint = topic_hint_in.to_string();
    let mut chosen_topic_id = String::new();
    if topic_hint.is_empty()
        && let Some(ref t) = best_thread
        && !t.topic_type.is_empty()
    {
        topic_hint.clone_from(&t.topic_type);
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
    if player_in_room && topic_source == "weighted_mask_pick" && rng.random_range(0..100) < gossip_pct {
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
    if let Ok(Some(room)) = db::get_room(room_id) {
        room_zone = room.zone.clone();
        room_desc = room.description.trim().to_string();
        if !room.tags.is_empty() {
            room_tags_joined = room.tags.join("、");
        }
        for rr in top_npc_rumors(room_id, &room.zone, now_unix, rumor_top_k) {
            recent_events.push(format!("{}{}", ns.rumor_prefix, rr.text));
            let _ = mark_rumor_used_by_text(&rr.text, now_unix);
        }
    }
    if player_in_room {
        recent_events.insert(0, ns.player_present_context_line.clone());
    }
    if let Some(arc) = crate::store::get_store() {
        let s = arc.read().unwrap_or_else(|e| e.into_inner());
        if let Some(d) = s.get_npc_rumor_digest()
            && !d.text.trim().is_empty()
        {
            recent_events.push(d.text.clone());
        }
    }
    let mut anchors_used: Vec<String> = vec![];
    if let Some(ref t) = best_thread
        && !t.anchors.is_empty()
    {
        anchors_used.extend(t.anchors.clone());
        recent_events.push(format!(
            "{}{}",
            ns.known_anchors_prefix,
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
        if echo.is_empty() {
            None
        } else {
            Some(echo)
        }
    } {
        recent_events.push(sprintf_s(&ns.rumor_echo_line_fmt, &[&echo]));
    }
    recent_events = dedupe_social_context_lines(&recent_events);
    let max_runes = cfg.npc_npc_quality_max_runes;
    let relation = dyad_relation_hint(dyad.as_ref());
    let ai = match call_ai_talk_npc_to_npc(
        &cfg.ollama_base_url,
        &cfg.ollama_model,
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
            return false;
        }
    };
    let (mut line_a, mut line_b, anchors) = ai;
    let na = name_a.as_str();
    let nb = name_b.as_str();
    line_a = strip_npc_line_speaker_prefix(&line_a, &[na, nb]);
    line_b = strip_npc_line_speaker_prefix(&line_b, &[nb, na]);
    line_a = sanitize_npc_dialogue_line(&line_a);
    line_b = sanitize_npc_dialogue_line(&line_b);
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
        return false;
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
        return false;
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
    tracing::info!("[Dialogue Debug] {} vs {}: \n  -> {}: {}\n  -> {}: {}", room_name, topic_hint, name_a, line_a, name_b, line_b);
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
            return false;
        }
        inc_stat("score_pass");
    } else {
        inc_stat("score_disabled_skip");
    }
    let summary = sprintf_s(
        &ns.summary_fmt,
        &[
            &room_name,
            &name_a,
            &trunc_rune(&line_a, 25),
            &name_b,
            &trunc_rune(&line_b, 25),
        ],
    );
    if !should_skip_npc_npc_summary_update(&npc_npc_memory, &summary, &line_a, &line_b) {
        let _ = set_npc_npc_conversation_summary(&a.id, &b.id, &summary);
    } else {
        inc_stat("skip_npc_npc_summary_dup");
    }
    let arch_a = sprintf_s(
        &ns.archival_with_fmt,
        &[&name_b, &trunc_rune(&line_a, 40)],
    );
    let arch_b = sprintf_s(
        &ns.archival_with_fmt,
        &[&name_a, &trunc_rune(&line_b, 40)],
    );
    if let Ok((_, true)) = insert_npc_npc_dialogue_archival(&a.id, &arch_a) {
        inc_stat("skip_npc_npc_archival_dup");
    }
    if let Ok((_, true)) = insert_npc_npc_dialogue_archival(&b.id, &arch_b) {
        inc_stat("skip_npc_npc_archival_dup");
    }
    let mut thread = best_thread.unwrap_or_else(|| NpcThread {
        thread_key: String::new(),
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
    if thread.turn_count >= 1 {
        thread.phase = "elaborate".into();
    }
    if thread.turn_count >= 3 {
        thread.phase = "cooling".into();
        thread.cooldown_until = now_unix + 300;
    }
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
        {
            continue;
        }
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
    set_pair_last_talk(&a.id, &b.id, now_unix);
    if let Some(mut d) = dyad {
        let du = &sim().dyad_update;
        let mut cap = du.sentiment_cap;
        if cap <= 0 {
            cap = 100;
        }
        d.familiarity += du.familiarity_delta_per_exchange;
        if d.familiarity > cap {
            d.familiarity = cap;
        }
        d.sentiment += sentiment_delta_from_dialogue(
            &topic_hint,
            &line_a,
            &line_b,
            d.familiarity,
            &chosen_topic_id,
        );
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
    let text = sprintf_s(
        &ns.broadcast_fmt,
        &[&name_a, &name_b, &line_a, &name_b, &line_b],
    );
    send_narrate_to_room(session_store, room_id, &text);
    inc_stat("success");
    true
}
