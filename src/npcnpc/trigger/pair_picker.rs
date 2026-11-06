
struct SelectedPair<'a> {
    a: &'a crate::entity::Character,
    b: &'a crate::entity::Character,
    name_a: String,
    name_b: String,
    best_score: i32,
    best_thread: Option<NpcThread>,
}

fn select_best_npc_pair<'a>(
    npcs: &'a [crate::entity::Character],
    now_unix: i64,
) -> Option<SelectedPair<'a>> {
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
                t.anchors.retain(|x| {
                    !x.contains('・')
                        && !x.starts_with("wt-")
                        && !x.starts_with("grid:")
                        && !matches!(
                            x.as_str(),
                            "清晨" | "正午" | "傍晚" | "夜裡" | "深夜" | "正晝" | "早晨" | "午後"
                        )
                });
                let _ = set_npc_npc_thread(&a.id, &b.id, t.clone());
            }
            if let Some(t) = &thread
                && (t.phase == "cooling" || t.phase == "awaiting_fulfillment")
                && t.cooldown_until > now_unix
            {
                continue;
            }
            if let Some(t) = &mut thread {
                if t.phase == "awaiting_fulfillment"
                    && t.cooldown_until > 0
                    && t.cooldown_until <= now_unix
                {
                    t.phase = "followup".into();
                    t.cooldown_until = 0;
                    let _ = set_npc_npc_thread(&a.id, &b.id, t.clone());
                } else if t.phase == "cooling"
                    && t.cooldown_until > 0
                    && t.cooldown_until <= now_unix
                {
                    let _ = db::delete_npc_npc_thread(&a.id, &b.id);
                    thread = None;
                }
            }
            let mut score = rng.random_range(0..base_n);
            if let Some(ref t) = thread
                && t.phase != "cooling"
            {
                score += 100;
                if t.phase == "awaiting_fulfillment" || t.phase == "elaborate" {
                    score += 25;
                }
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
            let has_active_thread = thread.as_ref().is_some_and(|t| {
                t.phase == "opening" || t.phase == "elaborate" || t.phase == "awaiting_fulfillment"
            });
            let talk_cd = if has_active_thread { 90 } else { 180 };
            if last > 0 && now_unix - last < talk_cd {
                continue;
            }
            if last > 0 && now_unix - last < pp.pair_last_talk_penalty_sec as i64 {
                score -= pp.pair_last_talk_penalty;
            }
            let p_a = {
                let res = crate::store::resolve_npc_person_name(&a.id, &a.display_title, None);
                if !res.is_empty() && res != "陌生人" {
                    res
                } else {
                    db::person_name_from_npc_list_label(&a.display_title)
                }
            };
            let p_b = {
                let res = crate::store::resolve_npc_person_name(&b.id, &b.display_title, None);
                if !res.is_empty() && res != "陌生人" {
                    res
                } else {
                    db::person_name_from_npc_list_label(&b.display_title)
                }
            };
            let probe_a = if !p_a.is_empty() { &p_a } else { &a.id };
            let probe_b = if !p_b.is_empty() { &p_b } else { &b.id };
            if probe_a == probe_b {
                continue;
            }
            let chars_a: Vec<char> = probe_a.chars().collect();
            let chars_b: Vec<char> = probe_b.chars().collect();
            if chars_a.len() >= 3 && chars_b.len() >= 3 && chars_a[chars_a.len() - 2..] == chars_b[chars_b.len() - 2..] {
                continue;
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
        return None;
    };
    let mut name_a = {
        let res = crate::store::resolve_npc_person_name(&a.id, &a.display_title, None);
        if !res.is_empty() && res != "陌生人" {
            res
        } else {
            db::person_name_from_npc_list_label(&a.display_title)
        }
    };
    let mut name_b = {
        let res = crate::store::resolve_npc_person_name(&b.id, &b.display_title, None);
        if !res.is_empty() && res != "陌生人" {
            res
        } else {
            db::person_name_from_npc_list_label(&b.display_title)
        }
    };
    if name_a.is_empty() {
        name_a.clone_from(&a.id);
    }
    if name_b.is_empty() {
        name_b.clone_from(&b.id);
    }
    let chars_a: Vec<char> = name_a.chars().collect();
    let chars_b: Vec<char> = name_b.chars().collect();
    let same_given = chars_a.len() >= 3 && chars_b.len() >= 3 && chars_a[chars_a.len() - 2..] == chars_b[chars_b.len() - 2..];
    if name_a == name_b || same_given {
        inc_stat("fail_same_name_npc");
        return None;
    }
    Some(SelectedPair {
        a,
        b,
        name_a,
        name_b,
        best_score,
        best_thread,
    })
}
