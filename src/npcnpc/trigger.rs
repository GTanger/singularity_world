//! 同房 NPC↔NPC 觸發（對齊既有 `npcnpc/trigger`）。

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rand::Rng;

use crate::ai::{
    call_ai_talk_npc_to_npc, raw_events_for_dialogue_score, sanitize_npc_dialogue_line,
    score_npc_dialogue, strip_npc_line_speaker_prefix,
};
use crate::config::{Server, sim};
use crate::db::{
    self, build_identity, get_npc_npc_conversation_summary, insert_npc_npc_dialogue_archival,
    mark_rumor_used_by_text, penalize_rumor_by_text, recent_npc_npc_archival_lines_for_entity,
    set_npc_npc_conversation_summary, set_npc_npc_dyad, set_npc_npc_thread, top_npc_rumors,
    upsert_lexicon_term, upsert_npc_rumor,
};
use crate::entity::EntityKind;
use crate::game::public_person_label_for_room;
use crate::gametext;
use crate::npc::{
    find_npc_npc_topic_id_by_hint, get_npc_npc_topic_by_id, pick_random_npc_npc_topic_for_pair,
};
use crate::server::{SessionStore, send_narrate_to_room};
use crate::store::{NpcRumor, NpcThread};

use super::helpers::{
    add_tag, anchor_consistency_check, dedupe_social_context_lines, dyad_relation_hint,
    merge_thread_anchors, npc_pair_same_venue, quality_gate_npc_line, remove_tag,
    sentiment_delta_from_dialogue, should_skip_npc_npc_summary_update, topic_mask_for_room,
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

fn player_layer_room_name(room_id: &str) -> String {
    if let Some(coord) = crate::grid::parse_grid_room_id(room_id)
        && let Some(cell) = crate::server::grid_manager::get_runtime_square_grid()
            .and_then(|g| g.get(coord).cloned())
    {
        if cell.name.trim().is_empty() {
            return crate::grid::terrain_name_zh(cell.terrain).to_string();
        }
        return cell.name;
    }
    db::get_room_name(room_id).unwrap_or_default()
}

pub fn resolve_viewer_npc_label(viewer_pid: &str, npc: &crate::entity::Character, room_name: &str) -> String {
    if crate::store::player_knows_npc(viewer_pid, &npc.id) {
        return crate::store::resolve_npc_person_name(&npc.id, &npc.display_title, None);
    }
    let tag = crate::store::npc_brief_appearance_tag(&npc.id, &npc.display_title);
    let role = if room_name.contains("工坊") || room_name.contains("鍛造") || room_name.contains("木作") || room_name.contains("鐵坊") {
        "匠人"
    } else if room_name.contains("藥") || room_name.contains("診") || room_name.contains("脈") {
        "醫工"
    } else if room_name.contains("學") || room_name.contains("講堂") || room_name.contains("書") || room_name.contains("印行") || room_name.contains("觀星") {
        "學者"
    } else if room_name.contains("商") || room_name.contains("鋪") || room_name.contains("市集") || room_name.contains("行") {
        "行商"
    } else if room_name.contains("渡") || room_name.contains("津") || room_name.contains("水尾") || room_name.contains("河") {
        "候渡客"
    } else if room_name.contains("茶") || room_name.contains("食") || room_name.contains("泉") || room_name.contains("旅社") || room_name.contains("驛館") {
        "茶客"
    } else {
        "街坊"
    };
    if !tag.is_empty() {
        format!("{tag}的{role}")
    } else {
        format!("一名{role}")
    }
}

pub fn format_staged_npc_dialogue(
    pattern_idx: usize,
    label_a: &str,
    label_b: &str,
    line_a: &str,
    line_b: &str,
    activity_a: &str,
    activity_b: &str,
) -> String {
    let act_clause = if !activity_a.is_empty() && !activity_a.contains("無所事事") && !activity_a.contains("安歇") {
        let clean = activity_a.trim_start_matches("正在").trim_start_matches("正");
        format!("停下手中{clean}的活計，")
    } else {
        String::new()
    };
    let _ = activity_b;
    match pattern_idx % 8 {
        0 => format!("{label_a}{act_clause}側過身對{label_b}言道：「{line_a}」\n{label_b}微微點頭，隨口應道：「{line_b}」"),
        1 => format!("{label_a}將手頭物事略作打理，向身側的{label_b}開口：「{line_a}」\n{label_b}抬眼笑應：「{line_b}」"),
        2 => format!("只聽{label_a}若有所思地望向身旁，同{label_b}搭話：「{line_a}」\n{label_b}順著話頭接道：「{line_b}」"),
        3 => format!("{label_a}朝{label_b}低聲說道：「{line_a}」\n{label_b}神情認真地思忖片刻，答道：「{line_b}」"),
        4 => format!("{label_a}放緩步調，轉頭對{label_b}說道：「{line_a}」\n{label_b}側耳聽完，溫聲應著：「{line_b}」"),
        5 => format!("案几旁，{label_a}手上動作未停，對{label_b}言道：「{line_a}」\n{label_b}語調平穩地回道：「{line_b}」"),
        6 => format!("{label_a}見{label_b}迎面而立，便啟齒笑道：「{line_a}」\n{label_b}會心一笑，朗聲回道：「{line_b}」"),
        _ => format!("{label_a}壓低嗓音，向{label_b}問起：「{line_a}」\n{label_b}略一沉吟，輕聲答話：「{line_b}」"),
    }
}

fn player_layer_pair_labels(room_name: &str) -> (String, String) {
    if room_name.contains("渡") || room_name.contains("津") || room_name.contains("水岸") || room_name.contains("水尾") || room_name.contains("臨河") {
        ("候渡客".to_string(), "舟子".to_string())
    } else if room_name.contains("坊") || room_name.contains("匠") {
        ("掌爐匠人".to_string(), "工坊幫工".to_string())
    } else if room_name.contains("商") || room_name.contains("市") {
        ("採買行商".to_string(), "掌案帳房".to_string())
    } else {
        ("行路街客".to_string(), "鄰坊居民".to_string())
    }
}

fn replace_private_names_for_player(text: &str, names: &[(&str, &str)]) -> String {
    let mut out = text.to_string();
    for (private, public) in names {
        let private = private.trim();
        if !private.is_empty() {
            out = out.replace(private, public);
        }
    }
    out
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
    if cfg.ollama_disable {
        inc_stat("fail_disabled");
        return false;
    }
    let cd = Duration::from_secs(
        sim()
            .npc_npc_social
            .room_recent_player_talk_cooldown_sec
            .max(0) as u64,
    );
    if session_store.room_has_player_with_recent_talk(room_id, cd) {
        inc_stat("fail_recent_player_talk");
        return false;
    }
    let Ok(entities) = db::get_entities_in_room(room_id, game_hour) else {
        inc_stat("fail_not_enough_npc");
        return false;
    };
    let is_night = game_hour >= 22 || game_hour < 5;
    let npcs: Vec<_> = entities
        .into_iter()
        .filter(|e| {
            if !matches!(e.kind, EntityKind::Npc) {
                return false;
            }
            let act = &e.current_activity;
            if act.contains("安歇")
                || act.contains("熟睡")
                || act.contains("就寢")
                || act.contains("熄燈")
            {
                return false;
            }
            if is_night {
                let slot = crate::store::wenyao_household_slot_for(&e.id);
                if slot.home_name == room_id
                    || room_id.contains("居")
                    || room_id.contains("宅")
                    || room_id.contains("舍")
                    || room_id.contains("齋")
                {
                    return false;
                }
            }
            true
        })
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
    let room_name = player_layer_room_name(room_id);
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
    if let Ok(Some(room)) = db::get_room(room_id) {
        room_zone = room.zone.clone();
        room_desc = room.description.trim().to_string();
        if !room.tags.is_empty() {
            room_tags_joined = room.tags.join("、");
        }
        let rumors = top_npc_rumors(room_id, &room.zone, now_unix, rumor_top_k);
        let has_rumors = !rumors.is_empty();
        for rr in rumors {
            recent_events.push(format!("{}{}", ns.rumor_prefix, rr.text));
            let _ = mark_rumor_used_by_text(&rr.text, now_unix);
        }
        if !has_rumors {
            let act_a = if a.current_activity.trim().is_empty() { "手頭雜務" } else { a.current_activity.trim() };
            let act_b = if b.current_activity.trim().is_empty() { "手頭差事" } else { b.current_activity.trim() };
            let thought_a = a.thoughts.last().and_then(|t| t.flavor.as_deref()).unwrap_or("");
            let thought_b = b.thoughts.last().and_then(|t| t.flavor.as_deref()).unwrap_or("");
            let dialogue_anchor = if !thought_a.is_empty() || !thought_b.is_empty() {
                format!(
                    "雙方正分別忙於「{}」與「{}」，就手頭差事、器物成色或半島民生自然寒暄交流（心緒參考：{} / {}）",
                    act_a, act_b, thought_a, thought_b
                )
            } else {
                format!("雙方正分別忙於「{}」與「{}」，就手頭差事、器物成色或半島民生自然寒暄交流", act_a, act_b)
            };
            recent_events.push(dialogue_anchor);
        }
    }
    if player_in_room {
        recent_events.insert(0, ns.player_present_context_line.clone());
    }
    if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
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
        obligations_a.push(format!("{}:{}", kind, subj));
    }
    let stance_decision = crate::ai::decision_scorer::score_npc_interaction_stance(
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
    if stance_decision.choice == crate::ai::decision_scorer::NpcStanceChoice::Refuse
        && stance_decision.confidence >= 0.35
    {
        // 第一棒拍板「不欲搭話／保持防備」，直接擦肩而過，不浪費第三棒對話模型 Token
        inc_stat("decision_refuse_skip");
        return false;
    }
    recent_events.push(stance_decision.prompt_directive.clone());
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
    let arch_a = sprintf_s(&ns.archival_with_fmt, &[&name_b, &trunc_rune(&line_a, 40)]);
    let arch_b = sprintf_s(&ns.archival_with_fmt, &[&name_a, &trunc_rune(&line_b, 40)]);
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
