fn run_npc_npc_dialogue_attempt(
    sessions: &SessionStore,
    cfg: &Server,
    hour: i32,
    player_room_set: &std::collections::HashSet<String>,
) -> bool {
    let ollama_ready = !cfg.ollama_disable;
    let mut dialogue_done = false;
    if ollama_ready {
        for room_id in player_room_set {
            let topic_hint =
                pick_random_npc_npc_topic_by_mask(&topic_mask_for_room(room_id, hour), "")
                    .map(|t| t.hint)
                    .unwrap_or_default();
            if try_trigger_npc_npc_in_room(sessions, cfg, room_id, &topic_hint, hour) {
                dialogue_done = true;
                break;
            }
        }
        if !dialogue_done {
            let mut rng = rand::rng();
            if rng.random_range(0..100) < 25 {
                if let Some(store) = crate::store::get_store() {
                    let candidate_rooms: Vec<String> = {
                        let st = store.read();
                        let live_states = crate::store::LIVE_NPC_GRID_STATES.read();
                        let mut coord_map: HashMap<(i32, i32), usize> = HashMap::new();
                        for e in st.entities.values() {
                            if e.kind == "npc" && e.vit > 0 {
                                let (coord, act) = if let Some(state) = live_states.get(&e.id) {
                                    (Some(state.current), state.current_activity())
                                } else {
                                    let c = match (e.grid_x, e.grid_y) {
                                        (Some(x), Some(y)) => Some((x, y)),
                                        _ => None,
                                    };
                                    (c, e.current_activity.clone())
                                };
                                let is_awake = !act.contains("安歇")
                                    && !act.contains("熟睡")
                                    && !act.contains("熄燈")
                                    && !act.contains("就寢")
                                    && !crate::store::is_npc_paused_for_interaction(&e.id);
                                if is_awake {
                                    if let Some(c) = coord {
                                        *coord_map.entry(c).or_insert(0) += 1;
                                    }
                                }
                            }
                        }
                        coord_map
                            .into_iter()
                            .filter(|((x, y), count)| {
                                if *count < 2 {
                                    return false;
                                }
                                let rid = crate::grid::grid_room_id_from_coord(*x, *y);
                                !player_room_set.contains(&rid)
                            })
                            .map(|((x, y), _)| crate::grid::grid_room_id_from_coord(x, y))
                            .collect()
                    };

                    if !candidate_rooms.is_empty() {
                        let idx = rng.random_range(0..candidate_rooms.len());
                        let offscreen_room_id = &candidate_rooms[idx];
                        let topic_hint = pick_random_npc_npc_topic_by_mask(
                            &topic_mask_for_room(offscreen_room_id, hour),
                            "",
                        )
                        .map(|t| t.hint)
                        .unwrap_or_default();
                        if try_trigger_npc_npc_in_room(
                            sessions,
                            cfg,
                            offscreen_room_id,
                            &topic_hint,
                            hour,
                        ) {
                            dialogue_done = true;
                        }
                    }
                }
            }
        }
    }

    dialogue_done
}

fn run_offscreen_autonomous_production(
    player_room_set: &std::collections::HashSet<String>,
    rng: &mut impl rand::Rng,
) {
            let offscreen_rooms: Vec<(String, Vec<String>)> = if let Some(store) = crate::store::get_store() {
                let st = store.read();
                let mut room_map: HashMap<String, Vec<String>> = HashMap::new();
                for e in st.entities.values() {
                    if e.kind == "npc" && e.vit > 0 {
                        let rid = st.get_entity_room(&e.id);
                        if !rid.is_empty() && !player_room_set.contains(&rid) {
                            room_map.entry(rid).or_default().push(e.id.clone());
                        }
                    }
                }
                room_map
                    .into_iter()
                    .filter(|(_, npcs)| !npcs.is_empty())
                    .collect()
            } else {
                Vec::new()
            };

            if !offscreen_rooms.is_empty() {
                let now_ts = crate::game::now_unix();
                let mut eligible: Vec<(String, Vec<String>)> = offscreen_rooms
                    .into_iter()
                    .filter(|(rid, _)| {
                        let lock = ROOM_IDLE_CHAT_LAST_TS.read().unwrap();
                        let last = lock.get(rid).copied().unwrap_or(0);
                        now_ts.saturating_sub(last) >= 35
                    })
                    .collect();

                let count = eligible.len().min(2);
                for _ in 0..count {
                    if eligible.is_empty() {
                        break;
                    }
                    let idx = rng.random_range(0..eligible.len());
                    let (off_room_id, off_active_npcs) = eligible.swap_remove(idx);
                    let off_room_name = if let Some(coord) = crate::grid::parse_grid_room_id(&off_room_id)
                        && let Some(cell) = crate::server::grid_manager::get_runtime_grid_cell(coord)
                    {
                        if cell.name.trim().is_empty() {
                            crate::grid::terrain_name_zh(cell.terrain).to_string()
                        } else {
                            cell.name
                        }
                    } else {
                        get_room_name(&off_room_id).unwrap_or_default()
                    };
                    if crate::game::object_ecology::run_npc_autonomous_production_step(
                        &off_room_id,
                        &off_room_name,
                        &off_active_npcs,
                        now_ts,
                    )
                    .is_some()
                    {
                        ROOM_IDLE_CHAT_LAST_TS
                            .write()
                            .unwrap()
                            .insert(off_room_id, now_ts);
                    }
                }
            }
}

fn run_npc_patrol_and_wander(
    sessions: &SessionStore,
    cfg: &Server,
    hour: i32,
    period: &str,
    player_room_set: &std::collections::HashSet<String>,
) {
    let Ok(schedules) = get_all_schedules() else {
        return;
    };
    let rf = gametext::runtime_fmt();
    let mut wmax = sim().wander_roll_max;
    if wmax <= 0 {
        wmax = 10;
    }
    let mut rng = rand::rng();
    for sch in schedules {
        if !sch.is_on_duty(hour) || hour >= 22 || hour < 6 {
            continue;
        }
        if crate::db::get_entity_place_id(&sch.entity_id).is_some_and(|place_id| {
            crate::store::get_store().is_some_and(|store| {
                store
                    .read()
                    .get_place(&place_id)
                    .is_some_and(|place| place.source_kind == "azgaar_runtime")
            })
        }) {
            continue;
        }
        if crate::store::is_npc_paused_for_interaction(&sch.entity_id) {
            continue;
        }
        let Ok(npc_room) = get_entity_room(&sch.entity_id) else {
            continue;
        };
        let mut title = get_npc_title_from_assignments(&sch.entity_id);
        if title.is_empty() {
            title = gametext::default_manager_title();
        }
        let person = get_npc_person_display_name(&sch.entity_id).unwrap_or_default();
        let wander_rooms = get_wander_rooms(&title);
        if wander_rooms.len() > 1 && rng.random_range(0..wmax) == 0 {
            let npc_key = canonical_location_key(&npc_room);
            let candidates: Vec<String> = wander_rooms
                .into_iter()
                .filter(|wr| canonical_location_key(wr) != npc_key)
                .collect();
            if !candidates.is_empty() {
                let dest = candidates[rng.random_range(0..candidates.len())].clone();
                let dest_name = get_room_name(&dest).unwrap_or_else(|_| String::new());
                let src_name = get_room_name(&npc_room).unwrap_or_else(|_| String::new());
                let want_src = canonical_location_key(&npc_room);
                for s in sessions.all_sessions() {
                    if crate::server::session::player_location_key(&s.player_id) == want_src {
                        let viewer_person = resolve_ambient_caller_label(&s.player_id, &sch.entity_id);
                        let leave_text = get_wander_flavor(&title, &viewer_person, &dest_name, true);
                        if !leave_text.is_empty() {
                            crate::server::broadcast::send_narrate_to_player(&s, &leave_text);
                        }
                    }
                }
                if set_entity_room(&sch.entity_id, &dest).is_ok() {
                    let want_dest = canonical_location_key(&dest);
                    for s in sessions.all_sessions() {
                        if crate::server::session::player_location_key(&s.player_id) == want_dest {
                            let viewer_person = resolve_ambient_caller_label(&s.player_id, &sch.entity_id);
                            let arrive_text = get_wander_flavor(&title, &viewer_person, &src_name, false);
                            if !arrive_text.is_empty() {
                                crate::server::broadcast::send_narrate_to_player(&s, &arrive_text);
                            }
                        }
                    }
                }
                push_room_event(
                    &npc_room,
                    "leave",
                    &person,
                    &sprintf_s(&rf.push_leave_fmt, &[dest_name.as_str()]),
                );
                push_room_event(
                    &dest,
                    "arrive",
                    &person,
                    &sprintf_s(&rf.push_arrive_fmt, &[src_name.as_str()]),
                );
                refresh_room_views_for_room(sessions, cfg, &npc_room);
                refresh_room_views_for_room(sessions, cfg, &dest);
                continue;
            }
        }
        if !player_room_set.contains(&canonical_location_key(&npc_room)) {
            continue;
        }
        let disp = get_disposition(&sch.entity_id);
        let want_room = canonical_location_key(&npc_room);
        let mut sent_any = false;
        for s in sessions.all_sessions() {
            if crate::server::session::player_location_key(&s.player_id) == want_room {
                let viewer_person = resolve_ambient_caller_label(&s.player_id, &sch.entity_id);
                let emote = pick_idle_emote(&title, &period, &viewer_person, disp);
                if !emote.is_empty() {
                    crate::server::broadcast::send_narrate_to_player(&s, &emote);
                    sent_any = true;
                }
            }
        }
        if sent_any {
            break;
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vendor_banter_zero_hardcoded_personal_names() {
        let pairs = [
            ("roast_beef", "noodle_soup"),
            ("pepper_bun", "tofu_pudding"),
            ("herbal_tea", "roast_beef"),
            ("herbal_tea", "pepper_bun"),
            ("tofu_pudding", "herbal_tea"),
            ("rice_roll", "soy_milk"),
            ("morning_produce", "morning_fish"),
            ("other_vendor_1", "other_vendor_2"),
        ];
        let forbidden_names = [
            "罔市", "火旺", "順興伯", "阿發", "秋月", "文貴", "秀琴", "海生", "春生",
        ];
        for (a, b) in pairs {
            for hour in [8, 14, 20] {
                let banter = generate_vendor_banter("甲掌事", "乙主事", a, b, hour);
                for name in forbidden_names {
                    assert!(!banter.contains(name), "vendor banter for ({a}, {b}) contained hardcoded personal name: {name}");
                }
            }
        }
    }

    #[test]
    fn test_resolve_caller_name_uniform_and_counter_zone_recognition() {
        let mut ent_uniform = crate::store::Entity::new_npc("test_npc_clerk", "員");
        ent_uniform.display_title = "身穿制服店員".to_string();
        let caller_uniform = resolve_caller_name("p_test", &ent_uniform.id, &ent_uniform);
        assert!(caller_uniform.contains("身著工作制服"), "expected uniform caller label, got: {caller_uniform}");

        let mut ent_counter = crate::store::Entity::new_npc("test_npc_mgr", "掌");
        ent_counter.display_title = "在場值守的當值掌事".to_string();
        let caller_counter = resolve_caller_name("p_test", &ent_counter.id, &ent_counter);
        assert!(caller_counter.contains("站在櫃檯內側"), "expected counter caller label, got: {caller_counter}");

        let mut ent_both = crate::store::Entity::new_npc("test_npc_lead", "長");
        ent_both.display_title = "身穿制服店員且在場值守的當值掌事".to_string();
        let caller_both = resolve_caller_name("p_test", &ent_both.id, &ent_both);
        assert!(caller_both.contains("身著工作制服、站在櫃檯內側"), "expected both uniform and counter label, got: {caller_both}");
    }
}
