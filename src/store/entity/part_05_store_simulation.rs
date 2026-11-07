impl Store {
    pub fn npc_counts_by_grid(&self) -> std::collections::HashMap<(i32, i32), u16> {
        let mut counts = std::collections::HashMap::new();
        for e in self.entities.values() {
            if e.kind == "npc" && e.vit > 0 && !is_deprecated_npc_id(&e.id) {
                if let Some(coord) = self.effective_entity_grid(e) {
                    *counts.entry(coord).or_insert(0) += 1;
                }
            }
        }
        let time_tick = (crate::game::now_unix() / 180) as i64;
        for z_idx in 0..crate::game::CANONICAL_HUNTING_ZONES.len() {
            for sl in crate::game::live_zone_shanhai_slots(z_idx, time_tick) {
                if sl.tier >= 2 || crate::game::is_apex_beast_tracked_at(sl.x, sl.y) {
                    *counts.entry((sl.x, sl.y)).or_insert(0) += 1;
                }
            }
        }
        counts
    }

    /// 推進所有定居居民與山海遠征人員在正方格地圖上的思考決策與步進（每 tick 沿路徑邁進 1 格或於目標點完成作業後自主決策下一目標）。
    pub fn advance_household_npcs_one_step(&self) -> Vec<(String, String, String)> {
        static SEEDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !SEEDED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            self.ensure_household_assignments();
        }
        let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() else {
            return Vec::new();
        };
        self.sync_and_advance_hunting_parties(&grid);
        let ids: Vec<String> = LIVE_NPC_GRID_STATES.read().keys().cloned().collect();
        let mut moves = Vec::new();
        for id in ids {
            if let Some(((ox, oy), (nx, ny))) = self.step_npc_grid_coord_with_grid(&id, &grid)
                && (ox, oy) != (nx, ny)
            {
                moves.push((
                    id,
                    crate::grid::grid_room_id_from_coord(ox, oy),
                    crate::grid::grid_room_id_from_coord(nx, ny),
                ));
            }
        }
        moves
    }
    /// 同步並推進全島 19 處山海獵場的「合社圍獵小隊共識板（`LIVE_HUNTING_PARTIES`）」：
    /// 涵蓋吹角揪團集結 -> 鎖定活體山海獵物座標四面戰術包抄合圍 -> 合擊獵殺與見者有份分鹿割皮 -> 結伴赴市交割分成。
    pub fn sync_and_advance_hunting_parties(&self, grid: &crate::grid::SquareGrid) {
        let now_unix = crate::game::now_unix();
        let _hour = current_game_hour();
        self.ensure_household_assignments();

        let mut parties = LIVE_HUNTING_PARTIES.write();
        if parties.is_empty() {
            let assignments = HOUSEHOLD_MEMBER_ASSIGNMENTS.read();
            let mut slot_to_id: std::collections::HashMap<usize, String> =
                std::collections::HashMap::with_capacity(assignments.len());
            for (id, &s_idx) in assignments.iter() {
                slot_to_id.insert(s_idx, id.clone());
            }
            drop(assignments);

            for (z_idx, &(zone_name, cx, cy)) in HUNTING_PARTY_ZONES.iter().enumerate() {
                let mut core_ids = Vec::new();
                let mut rally_coord = (cx - 5, cy - 4);
                let (default_m_coord, default_m_name) = grid
                    .find_nearest_room_by_tags((cx, cy), 60, &["market", "shop", "canteen"], (z_idx as u64).wrapping_mul(37))
                    .unwrap_or(((0, 0), "文曜・西門車馬聯運總站".to_string()));
                let mut market_coord = default_m_coord;
                let mut market_name = default_m_name;

                for s_idx in 0..total_household_slots_len() {
                    let slot = household_slot_at(s_idx);
                    if slot.brief_tag.starts_with("合社獵團・")
                        && slot.work_name.starts_with(zone_name)
                        && let Some(ent_id) = slot_to_id.get(&s_idx)
                    {
                        if core_ids.is_empty() {
                            rally_coord = crate::game::pick_zone_artisan_waypoint(
                                zone_name,
                                (z_idx as u64).wrapping_mul(53).wrapping_add(11),
                                (cx, cy),
                            )
                            .0;
                            market_coord = (slot.errand_x, slot.errand_y);
                            market_name = slot.errand_name.to_string();
                        } else {
                            // 過磅交割均分鎂幣紅利場地挑選：
                            // 優先選擇山貨、皮毛、藥、商行、市集、雜貨、大食堂、棧等相稱場所，
                            // 避開診所、浴堂，以及鐵器鍛造坊、木材行、樟腦、抄紙、磚窯、石材等不相干工業作坊。
                            let is_preferred = |n: &str| {
                                (n.contains("山貨")
                                    || n.contains("皮毛")
                                    || n.contains("皮貨")
                                    || n.contains("藥")
                                    || n.contains("商行")
                                    || n.contains("市集")
                                    || n.contains("雜貨")
                                    || n.contains("大食堂")
                                    || n.contains("物產")
                                    || n.contains("棧"))
                                    && !n.contains("診所")
                                    && !n.contains("浴堂")
                                    && !n.contains("鐵器")
                                    && !n.contains("鍛造")
                                    && !n.contains("木材")
                                    && !n.contains("樟腦")
                                    && !n.contains("抄紙")
                                    && !n.contains("磚窯")
                                    && !n.contains("石材")
                            };
                            let cur_pref = is_preferred(&market_name);
                            let cand_pref = is_preferred(slot.errand_name);
                            if (!cur_pref && cand_pref)
                                || ((market_name.contains("診所") || market_name.contains("浴堂") || market_name.contains("木材") || market_name.contains("鍛造") || market_name.contains("鐵器"))
                                    && !slot.errand_name.contains("診所")
                                    && !slot.errand_name.contains("浴堂")
                                    && !slot.errand_name.contains("木材")
                                    && !slot.errand_name.contains("鍛造")
                                    && !slot.errand_name.contains("鐵器"))
                            {
                                market_coord = (slot.errand_x, slot.errand_y);
                                market_name = slot.errand_name.to_string();
                            }
                        }
                        core_ids.push(ent_id.clone());
                    }
                }
                if core_ids.is_empty() {
                    continue;
                }
                // 若最終市場仍帶有不相干工業作坊（鐵器鍛造坊、木材行、樟腦、抄紙、磚窯、石材）或診所浴堂，
                // 替換為該聚落之山海物產收購行或山產皮貨交割場，絕不將獵物送去鐵匠鋪或木材行！
                let is_unrelated_venue = market_name.contains("鐵器")
                    || market_name.contains("鍛造")
                    || market_name.contains("木材")
                    || market_name.contains("樟腦")
                    || market_name.contains("抄紙")
                    || market_name.contains("磚窯")
                    || market_name.contains("石材")
                    || market_name.contains("診所")
                    || market_name.contains("浴堂");
                if is_unrelated_venue {
                    let settlement = market_name
                        .split_once('・')
                        .map(|(s, _)| s)
                        .unwrap_or("半嶺鎮");
                    market_name = format!("{settlement}・山海物產收購行");
                    market_coord = resolve_bldg_coord_or_fallback(&market_name, market_coord);
                }
                let leader_id = core_ids[0].clone();
                let mut full_member_ids = core_ids.clone();
                // 領隊老獵頭吹角號召：自動從鄰近村鎮招募 1~2 名膽識過人（boldness > 0.52）的居民加入東北／西北側翼助陣合圍！
                for s_idx in ((z_idx * 37) % 1047..1047).chain(0..((z_idx * 37) % 1047)) {
                    if full_member_ids.len() >= 8 {
                        break;
                    }
                    let slot_idx_global = WENYAO_HOUSEHOLD_SLOTS.len() + s_idx;
                    let cand_slot = household_slot_at(slot_idx_global);
                    if !cand_slot.brief_tag.starts_with("合社獵團・")
                        && !cand_slot.brief_tag.starts_with("入山百工・")
                        && !cand_slot.brief_tag.starts_with("古道行旅・")
                        && let Some(cand_id) = slot_to_id.get(&slot_idx_global)
                        && let Some(cand_ent) = self.entities.get(cand_id)
                    {
                        let p = crate::db::expand_soul_seed_to_personality(
                            cand_ent.soul_seed.unwrap_or(slot_idx_global as i64),
                        );
                        let already_in_other_party = parties
                            .values()
                            .any(|p| p.member_ids.contains(cand_id));
                        if p.boldness > 0.54
                            && !full_member_ids.contains(cand_id)
                            && !already_in_other_party
                        {
                            full_member_ids.push(cand_id.clone());
                        }
                    }
                }
                let leader_name = self
                    .entities
                    .get(&leader_id)
                    .map(|e| {
                        resolve_npc_person_name(
                            &e.id,
                            if e.display_title.trim().is_empty() {
                                "老獵頭"
                            } else {
                                e.display_title.as_str()
                            },
                            Some(e.gender.as_str()),
                        )
                    })
                    .unwrap_or_else(|| "老獵頭".to_string());

                let init_round = z_idx as u32;
                let (prey_coord, prey_slot_idx, prey_name, prey_tier, prey_loot_id, prey_loot_name) =
                    pick_live_shanhai_prey_for_zone(cx, cy, init_round, now_unix);
                let init_phase = if z_idx % 3 == 0 {
                    HuntingPartyPhase::Rallying
                } else {
                    HuntingPartyPhase::SurroundingPrey
                };
                let party = HuntingPartyState {
                    zone_name: zone_name.to_string(),
                    zone_idx: z_idx,
                    center: (cx, cy),
                    rally_coord,
                    market_coord,
                    market_name,
                    leader_id,
                    leader_name,
                    core_member_ids: core_ids.clone(),
                    member_ids: full_member_ids,
                    phase: init_phase,
                    prey_slot_idx,
                    prey_coord,
                    prey_name,
                    prey_tier,
                    prey_loot_id,
                    prey_loot_name,
                    hunt_round: init_round,
                    last_phase_change_unix: now_unix,
                };
                parties.insert(zone_name.to_string(), party);
            }

            let mut states = LIVE_NPC_GRID_STATES.write();
            for party in parties.values() {
                for (m_idx, mid) in party.member_ids.iter().enumerate() {
                    if let Some(st) = states.get_mut(mid) {
                        apply_party_directive_to_state(party, m_idx, grid, st);
                        // 初始直接就位四面合圍圈或獵場前哨集結營地，避免塞在單一中心點
                        st.current = st.dest;
                    }
                }
            }
            return;
        }


        let mut all_party_members: std::collections::HashSet<String> = parties
            .values()
            .flat_map(|p| p.member_ids.iter().cloned())
            .collect();
        let mut states = LIVE_NPC_GRID_STATES.write();
        for party in parties.values_mut() {
            let elapsed = now_unix.saturating_sub(party.last_phase_change_unix);
            match party.phase {
                HuntingPartyPhase::Rallying => {
                    let gathered = party
                        .member_ids
                        .iter()
                        .filter(|id| {
                            states
                                .get(*id)
                                .map(|s| {
                                    (s.current.0 - party.rally_coord.0).abs()
                                        + (s.current.1 - party.rally_coord.1).abs()
                                        <= 6
                                })
                                .unwrap_or(false)
                        })
                        .count();
                    if gathered >= 3 || elapsed >= 14 {
                        if party.member_ids.len() < 8 {
                            let assignments = HOUSEHOLD_MEMBER_ASSIGNMENTS.read();
                            let mut slot_to_id: std::collections::HashMap<usize, String> =
                                std::collections::HashMap::with_capacity(assignments.len());
                            for (id, &s_idx) in assignments.iter() {
                                slot_to_id.insert(s_idx, id.clone());
                            }
                            drop(assignments);

                            let start_off = (party.zone_idx * 37 + party.hunt_round as usize * 19) % 1047;
                            let mut recruited_count = 0;
                            for s_idx in (start_off..1047).chain(0..start_off) {
                                if party.member_ids.len() >= 8 || recruited_count >= 2 {
                                    break;
                                }
                                let slot_idx_global = WENYAO_HOUSEHOLD_SLOTS.len() + s_idx;
                                let cand_slot = household_slot_at(slot_idx_global);
                                if !cand_slot.brief_tag.starts_with("合社獵團・")
                                    && !cand_slot.brief_tag.starts_with("入山百工・")
                                    && !cand_slot.brief_tag.starts_with("古道行旅・")
                                    && let Some(cand_id) = slot_to_id.get(&slot_idx_global)
                                    && let Some(cand_ent) = self.entities.get(cand_id)
                                {
                                    let p = crate::db::expand_soul_seed_to_personality(
                                        cand_ent.soul_seed.unwrap_or(slot_idx_global as i64),
                                    );
                                    let already_in_other_party =
                                        all_party_members.contains(cand_id);
                                    if p.boldness > 0.52
                                        && !party.member_ids.contains(cand_id)
                                        && !already_in_other_party
                                    {
                                        party.member_ids.push(cand_id.clone());
                                        all_party_members.insert(cand_id.clone());
                                        recruited_count += 1;
                                        let cid_c = cand_id.clone();
                                        let l_name = party.leader_name.clone();
                                        let z_name = party.zone_name.clone();
                                        crate::npc::causal_core::enqueue_arrival_effect(
                                            crate::npc::causal_core::ArrivalEffect::CustomThoughtAndMg {
                                                entity_id: cid_c,
                                                delta_mg: 0,
                                                delta_disp: 8,
                                                now_unix,
                                                thought: format!(
                                                    "聽聞領隊【{}】吹響山豬牙號角揪團前往【{}】圍獵，欣然提刀入團助陣",
                                                    l_name, z_name
                                                ),
                                            },
                                        );
                                    }
                                }
                            }
                        }

                        party.hunt_round = party.hunt_round.wrapping_add(1);
                        let (p_coord, p_slot, p_name, p_tier, l_id, l_name) =
                            pick_live_shanhai_prey_for_zone(
                                party.center.0,
                                party.center.1,
                                party.hunt_round,
                                now_unix,
                            );
                        party.prey_coord = p_coord;
                        party.prey_slot_idx = p_slot;
                        party.prey_name = p_name;
                        party.prey_tier = p_tier;
                        party.prey_loot_id = l_id;
                        party.prey_loot_name = l_name;
                        party.phase = HuntingPartyPhase::SurroundingPrey;
                        party.last_phase_change_unix = now_unix;
                        for (m_idx, mid) in party.member_ids.iter().enumerate() {
                            if let Some(st) = states.get_mut(mid) {
                                apply_party_directive_to_state(party, m_idx, grid, st);
                            }
                        }
                    }
                }
                HuntingPartyPhase::SurroundingPrey => {
                    // 即時追蹤目標獵物當前微幅游走座標並檢查合圍就位人數
                    let (p_coord, p_slot, p_name, p_tier, l_id, l_name) = pick_live_shanhai_prey_for_zone(
                        party.center.0,
                        party.center.1,
                        party.hunt_round,
                        now_unix,
                    );
                    party.prey_coord = p_coord;
                    party.prey_slot_idx = p_slot;
                    party.prey_name = p_name;
                    party.prey_tier = p_tier;
                    party.prey_loot_id = l_id;
                    party.prey_loot_name = l_name;
                    let in_surround_ring = party
                        .member_ids
                        .iter()
                        .filter(|id| {
                            states
                                .get(*id)
                                .map(|s| {
                                    (s.current.0 - party.prey_coord.0).abs()
                                        + (s.current.1 - party.prey_coord.1).abs()
                                        <= 3
                                })
                                .unwrap_or(false)
                        })
                        .count();
                    if in_surround_ring >= 3 || elapsed >= 28 {
                        if party.prey_tier == 1 {
                            crate::game::mark_apex_beast_tracked_at(
                                party.prey_coord.0,
                                party.prey_coord.1,
                            );
                        }
                        party.phase = HuntingPartyPhase::TakedownAndSplit;
                        party.last_phase_change_unix = now_unix;
                    }
                    for (m_idx, mid) in party.member_ids.iter().enumerate() {
                        if let Some(st) = states.get_mut(mid) {
                            apply_party_directive_to_state(party, m_idx, grid, st);
                        }
                    }
                }
                HuntingPartyPhase::TakedownAndSplit => {
                    if elapsed >= 8 {
                        crate::game::record_shanhai_slot_hunted(
                            party.zone_idx,
                            party.prey_slot_idx,
                            party.prey_coord,
                            &party.prey_name,
                            &party.prey_loot_id,
                            &party.prey_loot_name,
                            &party.leader_name,
                            now_unix,
                        );
                        let members_clone = party.member_ids.clone();
                        let leader_name = party.leader_name.clone();
                        let z_name = party.zone_name.clone();
                        let p_name = party.prey_name.clone();
                        let _l_id = party.prey_loot_id.clone();
                        let l_name = party.prey_loot_name.clone();
                        let mkt_name = party.market_name.clone();
                        let total_n = members_clone.len();
                        let prey_xy = party.prey_coord;
                        for mid in &members_clone {
                            crate::npc::causal_core::enqueue_arrival_effect(
                                crate::npc::causal_core::ArrivalEffect::CustomThoughtAndMg {
                                    entity_id: mid.clone(),
                                    delta_mg: 0,
                                    delta_disp: 9,
                                    now_unix,
                                    thought: format!(
                                        "與領隊【{}】等 {} 名合社夥伴在【{}】四面合圍獵獲【{}】，按見者有份古禮分得【{}】，正結伴運往{}交割分成",
                                        leader_name, total_n, z_name, p_name, l_name, mkt_name
                                    ),
                                },
                            );
                        }
                        let eligible_players: Vec<String> = self
                            .entities
                            .values()
                            .filter(|e| e.kind == "player")
                            .filter(|e| {
                                let pledged = player_pledged_hunting_zone(&e.id).as_deref()
                                    == Some(z_name.as_str());
                                let near =
                                    (e.x - prey_xy.0).abs().max((e.y - prey_xy.1).abs()) <= 8;
                                pledged || near
                            })
                            .map(|e| e.id.clone())
                            .collect();
                        for pid in eligible_players {
                            crate::npc::causal_core::enqueue_arrival_effect(
                                crate::npc::causal_core::ArrivalEffect::CustomThoughtAndMg {
                                    entity_id: pid,
                                    delta_mg: 28,
                                    delta_disp: 9,
                                    now_unix,
                                    thought: format!(
                                        "隨領隊【{}】率領之【{}】合社獵團合圍【{}】，依「見者有份」古規分得一份【{}】與 28 枚鎂幣紅利",
                                        leader_name, z_name, p_name, l_name
                                    ),
                                },
                            );
                        }
                        for member_id in &party.member_ids {
                            crate::npc::causal_core::upsert_relational_hook(
                                member_id,
                                &party.leader_id,
                                &party.leader_name,
                                "合社並肩",
                                format!(
                                    "曾與【{}】在{}並肩合圍「{}」、同分獵獲",
                                    party.leader_name, party.zone_name, party.prey_name
                                ),
                                3,
                                now_unix,
                            );
                        }
                        let apex_ripple = crate::npc::causal_core::WorldRippleFact {
                            fact_id: format!("apex_slain_{}_{}", party.zone_idx, now_unix),
                            kind: crate::npc::causal_core::RippleFactKind::ApexSlainOrRescued,
                            subject_zone_or_settlement: party.zone_name.clone(),
                            item_or_actor: party.prey_name.clone(),
                            order_id: None,
                            severity: 1.0,
                            origin_tick: now_unix,
                            hop_count: 0,
                            max_hops: 4,
                            prose_summary: format!(
                                "{}的合社獵團已在{}合圍獵得「{}」，山道威脅大減，正押運「{}」返鎮交割",
                                party.leader_name, party.zone_name, party.prey_name, party.prey_loot_name
                            ),
                        };
                        for member_id in &party.member_ids {
                            crate::npc::causal_core::inject_ripple_fact_into_npc(
                                member_id,
                                apex_ripple.clone(),
                                now_unix,
                            );
                        }
                        party.phase = HuntingPartyPhase::ReturningToMarket;
                        party.last_phase_change_unix = now_unix;
                        for (m_idx, mid) in party.member_ids.iter().enumerate() {
                            if let Some(st) = states.get_mut(mid) {
                                apply_party_directive_to_state(party, m_idx, grid, st);
                            }
                        }
                    }
                }
                HuntingPartyPhase::ReturningToMarket => {
                    let at_market = party
                        .member_ids
                        .iter()
                        .filter(|id| {
                            states
                                .get(*id)
                                .map(|s| {
                                    (s.current.0 - party.market_coord.0).abs()
                                        + (s.current.1 - party.market_coord.1).abs()
                                        <= 6
                                })
                                .unwrap_or(false)
                        })
                        .count();
                    if at_market >= 2 || elapsed >= 36 {
                        let members_clone = party.member_ids.clone();
                        let leader_name = party.leader_name.clone();
                        let l_id = party.prey_loot_id.clone();
                        let l_name = party.prey_loot_name.clone();
                        let mkt_name = party.market_name.clone();
                        let base_price = match party.prey_tier {
                            1 => 36,
                            2 => 24,
                            _ => 16,
                        };
                        let bounty = match party.prey_tier {
                            1 => 52,
                            2 => 36,
                            _ => 26,
                        };
                        let actor_tag = format!("【{}】合社獵團", leader_name);
                        let _ = crate::game::object_ecology::record_settlement_market_trade(
                            &mkt_name,
                            &l_id,
                            base_price,
                            2,
                            &actor_tag,
                        );
                        for mid in &members_clone {
                            crate::npc::causal_core::enqueue_arrival_effect(
                                crate::npc::causal_core::ArrivalEffect::CustomThoughtAndMg {
                                    entity_id: mid.clone(),
                                    delta_mg: bounty,
                                    delta_disp: 8,
                                    now_unix,
                                    thought: format!(
                                        "隨領隊【{}】合社獵團將圍獵所獲之【{}】運抵{}過磅交割，分得紅利 {} 枚鎂幣，眾人士氣大振",
                                        leader_name, l_name, mkt_name, bounty
                                    ),
                                },
                            );
                        }
                        // 交割分成完畢：臨時響應號召之聚落居民帶著分成與獵獲返回各自生活，核心獵團重返駐點準備下一輪號召
                        for mid in &party.member_ids {
                            if !party.core_member_ids.contains(mid)
                                && let Some(st) = states.get_mut(mid)
                            {
                                st.dest = st.current;
                                st.stay_ticks_remaining = 0;
                            }
                        }
                        party.member_ids = party.core_member_ids.clone();
                        party.phase = HuntingPartyPhase::Rallying;
                        party.last_phase_change_unix = now_unix;
                        for (m_idx, mid) in party.member_ids.iter().enumerate() {
                            if let Some(st) = states.get_mut(mid) {
                                apply_party_directive_to_state(party, m_idx, grid, st);
                            }
                        }
                    }
                }
            }
        }
    }

    /// 沿道路相鄰可通行方格 BFS 步進 NPC 座標（每次最多移動 1 格）。
    pub fn step_npc_grid_coord(
        &self,
        entity_id: &str,
        _route_dir: &str,
        _target_zone: &str,
        _seed: u64,
    ) -> Option<((i32, i32), (i32, i32))> {
        let grid = crate::server::grid_manager::get_runtime_square_grid()?;
        self.step_npc_grid_coord_with_grid(entity_id, &grid)
    }

}
