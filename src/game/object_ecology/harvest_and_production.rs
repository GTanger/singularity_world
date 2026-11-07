/// 自世界物件或地景採集資源時，結合百工四維、吐納底色演進、工具磨損磨合與器況回饋。
/// 回傳 `(額外獲得份數, 質性敘事加註)`。
pub fn harvest_from_world_object(
    actor_id: &str,
    target_id: &str,
    target_name: &str,
    mat_id: &str,
    now_unix: i64,
) -> (i32, String) {
    let craft_dims = crate::server::handler::entity_total_craft_dims(actor_id);

    // 依採集物產類型演進吐納底色
    let low_mat = mat_id.to_lowercase();
    let low_tgt_id = target_id.to_lowercase();
    let is_ore = low_mat.contains("ore")
        || low_mat.contains("stone")
        || low_mat.contains("crystal")
        || low_mat.contains("clay")
        || low_mat.contains("salt")
        || low_tgt_id.contains("ore")
        || low_tgt_id.contains("stone")
        || low_tgt_id.contains("crystal")
        || target_name.contains("礦")
        || target_name.contains("石")
        || target_name.contains("岩")
        || target_name.contains("晶")
        || target_name.contains("鐵")
        || target_name.contains("銅")
        || target_name.contains("煤")
        || target_name.contains("玉")
        || target_name.contains("砂")
        || target_name.contains("鹽");

    let is_tide = low_mat.contains("reed")
        || low_mat.contains("water")
        || low_mat.contains("fish")
        || low_tgt_id.contains("water")
        || low_tgt_id.contains("fish")
        || target_name.contains("水")
        || target_name.contains("泉")
        || target_name.contains("葦")
        || target_name.contains("蘆")
        || target_name.contains("渡")
        || target_name.contains("岸")
        || target_name.contains("魚")
        || target_name.contains("海");

    let habit_kind = if is_ore {
        crate::pg::living_world::BreathHabitKind::ForgeHeavy
    } else if is_tide {
        crate::pg::living_world::BreathHabitKind::TideContinuous
    } else {
        crate::pg::living_world::BreathHabitKind::ForestLong
    };

    crate::pg::living_world::record_breath_activity(actor_id, habit_kind, 1, now_unix);

    // 檢視角色所裝備之武器或工器具
    let tool_id_opt = db::get_entity(actor_id).ok().flatten().and_then(|ent| {
        let map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&ent.equipment_slots).ok()?;
        for key in ["weapon", "hold_r", "hand", "tool", "hold_l"] {
            if let Some(s) = map.get(key).and_then(|v| v.as_str())
                && !s.is_empty()
            {
                return Some(s.to_string());
            }
        }
        None
    });

    let mut vstate_opt = None;
    if let Some(tool_id) = &tool_id_opt {
        let tool_name = get_clean_item_name(tool_id);
        if crate::pg::living_world::is_weapon_or_tool_item(tool_id, &tool_name) {
            let vs = crate::pg::living_world::record_vessel_use(actor_id, tool_id, 1, 1, now_unix);
            vstate_opt = Some(vs);
        }
    }

    let mut extra_granted = 0;
    let mut note = String::new();

    let is_rolled = vstate_opt.as_ref().map(|vs| vs.condition == crate::pg::living_world::VesselCondition::RolledEdge).unwrap_or(false);
    let is_familiar = vstate_opt.as_ref().map(|vs| {
        matches!(
            vs.familiarity(),
            crate::pg::living_world::VesselFamiliarity::WellBalanced
                | crate::pg::living_world::VesselFamiliarity::LikeOwnArm
        )
    }).unwrap_or(false);

    if is_rolled {
        note.push_str("（手頭器物刃口捲缺打滑，難以順暢切削，宜前往鐵坊以火候重新回火推磨 ⚒︎）");
    } else if craft_dims.harvest >= 1.0 || is_familiar {
        let _ = db::add_to_inventory(actor_id, mat_id, 1);
        extra_granted = 1;
        note.push_str("手頭器物趁手加上【山林採獵】手藝純熟，順著紋理多採得一份完好物材。");
    }

    let res_kind = if is_ore {
        "mineral"
    } else if is_tide {
        "fishery"
    } else if target_name.contains("木")
        || target_name.contains("樹")
        || target_name.contains("樟")
        || target_name.contains("林")
    {
        "timber"
    } else {
        "flora"
    };

    let (_h, is_abundant, is_fallow) =
        crate::game::harvest_zone_natural_resource(target_name, res_kind, 2.5);
    if is_fallow {
        note.push_str("（周遭山林水土新近採擷頗繁，順手培土護根以待來日生機）");
    } else if is_abundant && extra_granted == 0 {
        let _ = db::add_to_inventory(actor_id, mat_id, 1);
        extra_granted = 1;
        note.push_str("山林水土豐美、物產充盈，順手多採得一份上好物材。");
    }

    if let Some(ent) = db::get_entity(actor_id).ok().flatten() {
        if let (Some(gx), Some(gy)) = (ent.grid_x, ent.grid_y) {
            let room_name = crate::server::grid_manager::get_square_grid()
                .and_then(|g| g.get(crate::grid::SquareCoord::new(gx, gy)).map(|c| c.name.clone()))
                .unwrap_or_default();
            if !room_name.is_empty() {
                if let Some((_pack_id, _step, ep_line)) =
                    crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(&room_name, target_name)
                {
                    note.push(' ');
                    note.push_str(&ep_line);
                }
            }
        }
    }

    (extra_granted, note)
}

/// 運行單步 NPC 自主經濟與工造巡檢。
pub fn run_npc_autonomous_production_step(
    _room_id: &str,
    room_name: &str,
    npc_ids: &[String],
    now_unix: i64,
) -> Option<String> {
    if npc_ids.is_empty() {
        return None;
    }
    let npc_id = &npc_ids[(now_unix as usize) % npc_ids.len()];
    let (npc_name, ent_inventory, venue_opt) = if let Some(ent) = db::get_entity(npc_id).ok().flatten() {
        let tag = crate::store::npc_brief_appearance_tag(npc_id, &ent.display_title);
        let (is_staff, has_uniform, in_counter_zone) =
            crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
                room_name,
                npc_id,
                &ent.display_title,
                &ent.current_activity,
                &ent.equipment_slots,
            );
        let venue_opt = crate::grid::resolve_any_room_venue_object(room_name, &ent.display_title);
        let name = if is_staff {
            let host_title = venue_opt.as_ref().map(|v| v.default_host_title).unwrap_or("當值主事");
            if in_counter_zone {
                format!("站在櫃檯內側的{}（{}）", host_title, tag)
            } else if has_uniform {
                format!("身著工作制服的{}（{}）", host_title, tag)
            } else {
                format!("那名{}的街坊", tag)
            }
        } else {
            format!("那名{}的街坊", tag)
        };
        (name, ent.inventory, venue_opt)
    } else {
        let venue_opt = crate::grid::resolve_any_room_venue_object(room_name, "");
        ("一名街坊匠人".to_string(), "[]".to_string(), venue_opt)
    };

    // Step A: 原料採集 (渡口/水岸/河/碼頭/藥圃/竹林/山道/埠階/海/林/礦坑/採石場)
    // 排除城鎮街道、橋樑、弄堂、店鋪、公所等非採集地景
    let is_city_traffic_or_shop = room_name.contains("橋")
        || room_name.contains("街")
        || room_name.contains("巷")
        || room_name.contains("弄")
        || room_name.contains("堂")
        || room_name.contains("館")
        || room_name.contains("店")
        || room_name.contains("鋪")
        || room_name.contains("盧")
        || room_name.contains("廬")
        || room_name.contains("坪")
        || room_name.contains("道");

    let is_explicit_resource_site = room_name.contains("礦")
        || room_name.contains("坑")
        || room_name.contains("鑿石")
        || room_name.contains("採石")
        || room_name.contains("蘆")
        || room_name.contains("沼")
        || room_name.contains("草澤")
        || room_name.contains("竹林")
        || room_name.contains("深林")
        || room_name.contains("林場")
        || room_name.contains("鋸木")
        || room_name.contains("伐木");

    let is_gathering_site = (!is_city_traffic_or_shop || is_explicit_resource_site)
        && (room_name.contains("渡口")
            || room_name.contains("水岸")
            || room_name.contains("河口")
            || room_name.contains("碼頭")
            || room_name.contains("藥圃")
            || room_name.contains("竹林")
            || room_name.contains("山道")
            || room_name.contains("埠階")
            || room_name.contains("海灘")
            || room_name.contains("藥田")
            || room_name.contains("草澤")
            || room_name.contains("林")
            || room_name.contains("礦")
            || room_name.contains("坑")
            || room_name.contains("鑿石")
            || room_name.contains("採石")
            || room_name.contains("鋸木")
            || room_name.contains("伐木")
            || room_name.contains("沼")
            || room_name.contains("灘"));

    // Step B: 工坊工造與異變
    // 排除哨所、巡防、居所、橋樑等非工坊設施，並剝離外坊等行政區域名稱後判定是否為工坊
    let is_non_workshop_facility = room_name.contains("哨所")
        || room_name.contains("巡河")
        || room_name.contains("居")
        || room_name.contains("宅")
        || room_name.contains("天井")
        || room_name.contains("庭院")
        || room_name.contains("書院")
        || room_name.contains("茶館")
        || room_name.contains("飯館")
        || room_name.contains("醫館")
        || room_name.contains("會館")
        || room_name.contains("橋")
        || room_name.contains("碼頭")
        || room_name.contains("渡口");

    let clean_workshop_site_name = room_name
        .replace("三水外坊", "")
        .replace("講堂外坊", "")
        .replace("百工外坊", "")
        .replace("西門外坊", "");

    let is_workshop_site = (!is_non_workshop_facility
        && (clean_workshop_site_name.contains("鐵坊")
            || clean_workshop_site_name.contains("鍛造")
            || clean_workshop_site_name.contains("木作")
            || clean_workshop_site_name.contains("傢俱")
            || clean_workshop_site_name.contains("巧木")
            || clean_workshop_site_name.contains("印行")
            || clean_workshop_site_name.contains("造紙")
            || clean_workshop_site_name.contains("抄紙")
            || clean_workshop_site_name.contains("染坊")
            || clean_workshop_site_name.contains("織坊")
            || clean_workshop_site_name.contains("藥坊")
            || clean_workshop_site_name.contains("烘藥")
            || clean_workshop_site_name.contains("製藥")
            || clean_workshop_site_name.contains("工坊")
            || clean_workshop_site_name.contains("工寮")
            || clean_workshop_site_name.contains("石硯")
            || clean_workshop_site_name.contains("窯")
            || clean_workshop_site_name.contains("作")
            || clean_workshop_site_name.contains("匠")
            || clean_workshop_site_name.contains("爐")
            || clean_workshop_site_name.contains("印")
            || clean_workshop_site_name.contains("藥")
            || clean_workshop_site_name.contains("織")
            || clean_workshop_site_name.contains("造")
            || clean_workshop_site_name.contains("編院")
            || clean_workshop_site_name.contains("巧編")))
        || venue_opt.as_ref().is_some_and(|v| {
            v.semantic_tags().iter().any(|t| {
                matches!(
                    *t,
                    "workshop" | "blacksmith" | "textile" | "grindstone" | "mine" | "lumber"
                )
            })
        });
    if is_gathering_site && !is_workshop_site {
        let raw_opt = if room_name.contains("礦") || room_name.contains("坑") || room_name.contains("鑿石") || room_name.contains("採石") {
            Some(("mat_rough_ore", "粗生礦石"))
        } else if room_name.contains("蘆") || room_name.contains("沼") || room_name.contains("灘") || room_name.contains("岸") || room_name.contains("澤") {
            Some(("mat_reed_bundle", "水澤老蘆草綑"))
        } else if room_name.contains("藥") || room_name.contains("圃") || room_name.contains("田") {
            Some(("mat_foraged_herb", "野生採集草藥"))
        } else if room_name.contains("林") || room_name.contains("木坊") || room_name.contains("鋸木") || room_name.contains("巧木") || room_name.contains("伐木") {
            Some(("mat_timber_log", "伐採原木"))
        } else if room_name.contains("紙") {
            Some(("mat_coarse_paper_ream", "粗製竹麻紙料"))
        } else {
            None
        };

        let (raw_id, raw_name) = match raw_opt {
            Some(r) => r,
            None => return None,
        };
        let raw_s = crate::world::materials::infer_origin_settlement(room_name);
        let cur_settlement = normalize_canonical_settlement(if raw_s.is_empty() { room_name } else { raw_s });

        // 檢查 NPC 背包已持有 raw_id 數量；若 >= 4，轉為就地深加工交付市集，防止原料過度囤積
        let held_raw_qty = db::get_inventory(&ent_inventory, 10)
            .items
            .iter()
            .find(|it| it.item_id == raw_id)
            .map(|it| it.qty)
            .unwrap_or(0);

        if held_raw_qty >= 4 {
            let _ = db::remove_from_inventory(npc_id, raw_id, 2);
            let finished_def = map_raw_to_finished(raw_id);
            crate::npc::causal_core::record_settlement_market_production(cur_settlement, finished_def.id, 1);
            let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(room_name, finished_def.name);
            return Some(format!(
                "{npc_name}見行囊中所收【{raw_name}】堆疊已滿，索性就地取材修整編整，俐落打製出一件【{}】交付當地市集鋪肆。",
                finished_def.name
            ));
        }

        let _ = db::add_to_inventory(npc_id, raw_id, 1);
        {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = ledger.items.get_mut(raw_id) {
                entry.stock = (entry.stock + 1).min((entry.target_stock * 4).max(1));
            }
            let key = (cur_settlement.to_string(), raw_id.to_string());
            if let Some(cell) = ledger.settlement_markets.get_mut(&key) {
                cell.stock = (cell.stock + 1).min(cell.target_stock * 4);
            }
            save_economy_ledger_snapshot(&ledger);
        }

        let lines = [
            format!("{npc_name}手持竹編背簍在水岸草澤間巡視，俐落地割取一捆生機盎然的【{raw_name}】，仔細紮束後妥善收進行囊。"),
            format!("{npc_name}彎腰檢視埠頭旁的物料堆，將幾包質地齊整的【{raw_name}】清點分裝，額角微汗，動作甚是熟練。"),
            format!("{npc_name}在晨光水氣中採集整理【{raw_name}】，抖落泥水細心收納，準備稍後送往集市與工坊。"),
        ];
        let narr = lines[(now_unix as usize) % lines.len()].clone();
        return Some(narr);
    }

    if is_workshop_site && (room_name.contains("石磨") || room_name.contains("水輪") || room_name.contains("磨坊") || room_name.contains("碾")) {
        let (prod_id, prod_name) = if now_unix % 2 == 0 {
            ("wt_cat_0388", "現磨溫米漿")
        } else {
            ("wt_cat_0279", "特級在來米粉")
        };
        let _ = db::add_to_inventory(npc_id, prod_id, 1);
        let mill_s = crate::world::materials::infer_origin_settlement(room_name);
        let mill_settlement = if mill_s.is_empty() { "老堰頭" } else { mill_s };
        crate::npc::causal_core::record_settlement_market_production(mill_settlement, prod_id, 1);
        let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(room_name, prod_name);
        return Some(format!(
            "{npc_name}撥開{mill_settlement}引水木閘，湍急溪水帶動重礦包邊大石磨勻速旋轉，將新穀細碾分篩、慢熬成一份溫潤細膩的【{prod_name}】。"
        ));
    }

    if is_workshop_site {
        let (raw_id, raw_name, finished_def) = if room_name.contains("鐵") || room_name.contains("鍛") {
            ("mat_iron_scrap", "回收熟鐵料塊", &STANDARD_FINISHED_GOODS[0])
        } else if room_name.contains("木") || room_name.contains("竹") {
            ("mat_timber_log", "伐採原木", &STANDARD_FINISHED_GOODS[2])
        } else if room_name.contains("藥") {
            ("mat_foraged_herb", "野生採集草藥", &STANDARD_FINISHED_GOODS[4])
        } else if room_name.contains("印") || room_name.contains("紙") || room_name.contains("墨") {
            ("mat_coarse_paper_ream", "粗製竹麻紙料", &STANDARD_FINISHED_GOODS[6])
        } else {
            ("mat_hemp_rope_roll", "熟桐浸油麻繩圈", &STANDARD_FINISHED_GOODS[8])
        };

        let raw_s = crate::world::materials::infer_origin_settlement(room_name);
        let cur_settlement = normalize_canonical_settlement(if raw_s.is_empty() { room_name } else { raw_s });
        let npc_inv = db::get_inventory(&ent_inventory, 10);
        let held_finished_qty = npc_inv
            .items
            .iter()
            .find(|it| it.item_id == finished_def.id)
            .map(|it| it.qty)
            .unwrap_or(0);
        let holds_bulk_materials = npc_inv.items.iter().any(|it| {
            it.item_id == "mat_reed_bundle"
                || it.item_id == "mat_wood_offcut_bundle"
                || it.item_id == "mat_iron_scrap_box"
        });

        if held_finished_qty >= 3 || (holds_bulk_materials && held_finished_qty >= 2) {
            let deliver_qty = 2.min(held_finished_qty);
            let _ = db::remove_from_inventory(npc_id, finished_def.id, deliver_qty);
            crate::npc::causal_core::record_settlement_market_production(cur_settlement, finished_def.id, deliver_qty);
            let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(room_name, finished_def.name);
            return Some(format!(
                "{npc_name}盤點工坊案頭，見製成的【{}】已有積存，便分裝清點後挑運送往當地市集鋪肆行銷。",
                finished_def.name
            ));
        }

        let npc_has_raw = db::inventory_has_item(&ent_inventory, raw_id) || db::inventory_has_item(npc_id, raw_id);
        let raw_stock = {
            let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
            let cons_key = (cur_settlement.to_string(), raw_id.to_string());
            ledger.settlement_markets.get(&cons_key).map(|c| c.stock).unwrap_or(0)
        };

        if raw_stock <= 0 && !npc_has_raw {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            let cons_key = (cur_settlement.to_string(), raw_id.to_string());
            if let Some(cell) = ledger.settlement_markets.get_mut(&cons_key) {
                cell.demand_ema = (cell.demand_ema * 1.12).min(1.65);
            }
            let shortage_ev = format!("【工坊待料】{cur_settlement}工坊因缺乏「{raw_name}」暫緩開爐，當地收購行情急升！");
            if !ledger.recent_events.iter().any(|e| e == &shortage_ev) {
                ledger.recent_events.push_back(shortage_ev);
                if ledger.recent_events.len() > 30 {
                    ledger.recent_events.pop_front();
                }
            }
            save_economy_ledger_snapshot(&ledger);
            return Some(format!("{npc_name}檢視工坊料架，發現【{raw_name}】已經見底，只得暫停手頭工序，向門外張望是否有行商或旅人運料前來。"));
        }

        if npc_has_raw {
            let _ = db::remove_from_inventory(npc_id, raw_id, 1);
        }
        // 異變檢驗：NPC 工坊製作 25% 異變機率
        let roll = ((now_unix.wrapping_mul(1664525).wrapping_add(1013904223) / 65536) % 100).abs() as i32;
        let is_utility_or_doc = finished_def.id.contains("torch")
            || finished_def.id.contains("umbrella")
            || finished_def.id.contains("ink")
            || finished_def.id.contains("manual")
            || finished_def.id.contains("almanac")
            || finished_def.id.contains("storage_box")
            || finished_def.id.contains("tool_handle")
            || finished_def.name.contains("火把")
            || finished_def.name.contains("紙傘")
            || finished_def.name.contains("晴雨傘")
            || finished_def.name.contains("松煙墨")
            || finished_def.name.contains("墨錠")
            || finished_def.name.contains("隨身匣")
            || finished_def.name.contains("收納匣")
            || finished_def.name.contains("工具長柄")
            || finished_def.name.contains("手札");
        let is_mut = roll < 8
            && finished_def.item_type != "consumable"
            && !finished_def.slot.is_empty()
            && !is_utility_or_doc;

        if is_mut {
            let arch_idx = ((now_unix as usize) + npc_name.len()) % MUTATION_ARCHETYPES.len();
            let arch = &MUTATION_ARCHETYPES[arch_idx];
            let mut_id = format!("mut_{}_{}", finished_def.id, arch.key);
            let mut_name = format!("{}・{}", arch.name_prefix, finished_def.name);

            let desc = format!(
                "{}（{}，器物成色與質理俱臻上乘。）",
                finished_def.description, arch.desc_note
            );

            let item = Item {
                id: mut_id.clone(),
                name: mut_name.clone(),
                slot: finished_def.slot.to_string(),
                item_type: finished_def.item_type.to_string(),
                weight: finished_def.weight,
                stackable: 1,
                denomination: (finished_def.base_price as f64 * arch.price_mult).round() as i32,
                description: desc,
                vit_bonus: finished_def.vit_bonus + arch.vit_bonus,
                dex_bonus: finished_def.dex_bonus + arch.dex_bonus,
                atk_bonus: finished_def.atk_bonus + arch.atk_bonus,
                origin: finished_def.origin.to_string(),
            };
            ensure_item_in_store(item);
            let _ = db::add_to_inventory(npc_id, &mut_id, 1);
            let _ = crate::server::handler::npc_autonomous_word_and_equip_step(
                npc_id,
                &npc_name,
                &mut_id,
                &mut_name,
            );

            {
                let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
                let cons_key = (cur_settlement.to_string(), raw_id.to_string());
                if let Some(cell) = ledger.settlement_markets.get_mut(&cons_key) {
                    cell.stock = (cell.stock - 1).max(0);
                    cell.demand_ema = (cell.demand_ema * 1.05).min(1.45);
                }
                if let Some(entry) = ledger.items.get_mut(raw_id) {
                    entry.stock = (entry.stock - 1).max(0);
                }
                let prod_key = (cur_settlement.to_string(), finished_def.id.to_string());
                if let Some(cell) = ledger.settlement_markets.get_mut(&prod_key) {
                    cell.stock = (cell.stock + 1).min(cell.target_stock * 4);
                }
                if let Some(entry) = ledger.items.get_mut(finished_def.id) {
                    entry.stock = (entry.stock + 1).min((entry.target_stock * 4).max(1));
                }
                let ev = if mut_name.starts_with('【') {
                    format!("{npc_name}於工坊開爐煉製，引發氣脈異變造就{mut_name}！")
                } else {
                    format!("{npc_name}於工坊開爐煉製，引發氣脈異變造就【{mut_name}】！")
                };
                ledger.recent_events.push_back(ev);
                if ledger.recent_events.len() > 30 {
                    ledger.recent_events.pop_front();
                }
                save_economy_ledger_snapshot(&ledger);
            }
            let formatted_mut = if mut_name.starts_with('【') {
                mut_name.clone()
            } else {
                format!("【{mut_name}】")
            };
            let line = if room_name.contains("藥") {
                format!("{npc_name}守在焙藥爐前細心掌控火候，器物成形之際，濃郁靈香伴隨寒芒凝結，蛻變出罕見的{formatted_mut}！")
            } else {
                format!("{npc_name}專注拉動風箱、揮動鐵錘捶打器坯；只見火星迸散之際，氣脈流轉激盪，初成型的【{}】竟凝出奇異神彩，化為{formatted_mut}！", finished_def.name)
            };
            let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(room_name, finished_def.name);
            return Some(line);
        } else {
            let item = Item {
                id: finished_def.id.to_string(),
                name: finished_def.name.to_string(),
                slot: finished_def.slot.to_string(),
                item_type: finished_def.item_type.to_string(),
                weight: finished_def.weight,
                stackable: if finished_def.slot.is_empty() { 20 } else { 1 },
                denomination: finished_def.base_price,
                description: finished_def.description.to_string(),
                vit_bonus: finished_def.vit_bonus,
                dex_bonus: finished_def.dex_bonus,
                atk_bonus: finished_def.atk_bonus,
                origin: finished_def.origin.to_string(),
            };
            ensure_item_in_store(item);
            let _ = db::add_to_inventory(npc_id, finished_def.id, 1);

            {
                let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
                let prod_key = (cur_settlement.to_string(), finished_def.id.to_string());
                if let Some(cell) = ledger.settlement_markets.get_mut(&prod_key) {
                    cell.stock = (cell.stock + 1).min(cell.target_stock * 4);
                }
                if let Some(entry) = ledger.items.get_mut(finished_def.id) {
                    entry.stock = (entry.stock + 1).min((entry.target_stock * 4).max(1));
                }
                let cons_key = (cur_settlement.to_string(), raw_id.to_string());
                if let Some(cell) = ledger.settlement_markets.get_mut(&cons_key) {
                    cell.stock = (cell.stock - 1).max(0);
                    cell.demand_ema = (cell.demand_ema * 1.05).min(1.45);
                }
                if let Some(entry) = ledger.items.get_mut(raw_id) {
                    entry.stock = (entry.stock - 1).max(0);
                }
                save_economy_ledger_snapshot(&ledger);
            }
            let craft_narrative = if raw_id == "mat_iron_scrap"
                || finished_def.name.contains('刀')
                || finished_def.name.contains('錘')
                || finished_def.name.contains('鎬')
                || finished_def.name.contains('燈')
            {
                format!("{npc_name}揮動鍛錘在鐵砧上反覆敲打修整，火星迸落間，一件結實耐用的【{}】已然鍛打停當。", finished_def.name)
            } else if raw_id == "mat_timber_log"
                || raw_id == "mat_reed_bundle"
                || finished_def.name.contains('杖')
                || finished_def.name.contains('碗')
                || finished_def.name.contains('籃')
                || finished_def.name.contains('匣')
                || finished_def.name.contains('傘')
                || finished_def.name.contains('披')
            {
                format!("{npc_name}專心致志地推拉刨刀與竹篾刀，碎屑紛飛間，一件做工講究的【{}】已然打磨編製停當。", finished_def.name)
            } else if raw_id == "mat_foraged_herb"
                || finished_def.name.contains('茶')
                || finished_def.name.contains('膏')
            {
                format!("{npc_name}守在藥焙爐旁細心翻動百草、文火慢熬，清香瀰漫間，一份成色純正的【{}】已然焙製停當。", finished_def.name)
            } else if raw_id == "mat_coarse_paper_ream"
                || finished_def.name.contains("手札")
                || finished_def.name.contains("長卷")
                || finished_def.name.contains('墨')
            {
                format!("{npc_name}在案前細心裁紙、壓平裝幀，墨香微漾間，一冊裝訂齊整的【{}】已然整備停當。", finished_def.name)
            } else if raw_id == "mat_hemp_rope_roll"
                || finished_def.name.contains('繩')
                || finished_def.name.contains('囊')
                || finished_def.name.contains("斗篷")
                || finished_def.name.contains('靴')
            {
                format!("{npc_name}手持骨針與浸油麻線飛針走線、刷上熟桐防雨油，片刻後一件厚實耐用的【{}】已然縫製停當。", finished_def.name)
            } else {
                format!("{npc_name}按部就班操持工具，耐心地將【{raw_name}】切磋琢磨，片刻後一件結實耐用的【{}】順利完工。", finished_def.name)
            };
            let mut msg = craft_narrative;
            if let Some(note) = crate::server::handler::npc_autonomous_word_and_equip_step(
                npc_id,
                &npc_name,
                &finished_def.id,
                finished_def.name,
            ) {
                msg.push(' ');
                msg.push_str(&note);
            }
            let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(room_name, finished_def.name);
            return Some(msg);
        }
    }

    // Step C: 商市交易與動態調價 (市/街/鋪/行/商/棧/堂/口/場)
    // Step C: 自主行商跨區調撥與商市交易
    {
        let raw_s = crate::world::materials::infer_origin_settlement(room_name);
        let cur_settlement = normalize_canonical_settlement(if raw_s.is_empty() { room_name } else { raw_s });
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        execute_settlement_sinks_and_arbitrage(cur_settlement, now_unix, &mut ledger);
        save_economy_ledger_snapshot(&ledger);
    }

    let is_low_stock = {
        let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
        ledger.items.values().any(|st| st.stock < (st.target_stock * 7 / 10))
            || ledger.settlement_markets.values().any(|cell| cell.stock < (cell.target_stock * 7 / 10))
    };

    let is_shopkeeper = npc_id.starts_with("wt_vendor_")
        || (!npc_name.contains("獵")
            && !npc_name.contains("銃手")
            && !npc_name.contains("伙夫")
            && !npc_name.contains("鏢客")
            && !npc_name.contains("刀手")
            && !npc_name.contains("巡隘")
            && !npc_name.contains("苦力")
            && !npc_name.contains("採藥")
            && !npc_name.contains("淘砂")
            && !npc_name.contains("抽藤")
            && (npc_name.contains("商")
                || npc_name.contains("鋪")
                || npc_name.contains("行")
                || npc_name.contains("店")
                || npc_name.contains("販")
                || npc_name.contains("東家")
                || npc_name.contains("司庫")
                || room_name.contains("鋪")
                || room_name.contains("市")
                || room_name.contains("行")
                || room_name.contains("店")
                || room_name.contains("堂")
                || room_name.contains("棧")))
        || venue_opt.as_ref().is_some_and(|v| {
            v.has_counter_restricted_zone
                || v.semantic_tags().iter().any(|t| {
                    matches!(
                        *t,
                        "shop" | "market" | "canteen" | "teahouse" | "inn" | "bathhouse"
                    )
                })
        });

    if is_low_stock && is_shopkeeper {
        crate::store::open_active_npc_obligation(npc_id, "restock_goods", "鋪內庫存見底急需提貨補倉");
        let lines = [
            format!("{npc_name}翻閱案頭的進出貨簿子，發覺幾樣要緊物料庫存吃緊，提筆在簿上記下一筆補貨清單。"),
            format!("{npc_name}在店鋪架前來回巡視，核對今日市價行情，低聲向身旁學徒交代稍後前往工坊再添一批現貨。"),
        ];
        Some(lines[(now_unix as usize) % lines.len()].clone())
    } else if is_shopkeeper {
        let line_duty = if let Some(v) = &venue_opt {
            format!("{npc_name}（{}）熟練地核對市秤與貨品成色，當場點清貨款收妥，彼此言談爽利，交易順遂。", v.on_duty_fixture_activity)
        } else {
            format!("{npc_name}熟練地核對市秤與貨品成色，當場點清貨款收妥，彼此言談爽利，交易順遂。")
        };
        let lines = [
            line_duty,
            format!("{npc_name}向來往鄉鄰展示新到的工造物件，雙方爽朗核對價錢，乾脆俐落地結清貨款。"),
        ];
        Some(lines[(now_unix as usize) % lines.len()].clone())
    } else {
        let lines = [
            format!("{npc_name}步履從容地順著街巷前行，打量著四下往來動靜。"),
            format!("{npc_name}在街旁稍作歇步，檢視隨身攜帶的行囊與器物。"),
        ];
        Some(lines[(now_unix as usize) % lines.len()].clone())
    }
}

pub fn get_clean_item_name(item_id: &str) -> String {
    match item_id {
        "mat_rough_ore" => return "粗生礦石".to_string(),
        "mat_iron_scrap" => return "熟鐵料塊".to_string(),
        "mat_repair_strip" => return "修補鐵片".to_string(),
        "mat_timber_log" => return "伐採原木".to_string(),
        "mat_reed_bundle" => return "老蘆草綑".to_string(),
        "mat_foraged_herb" => return "野生草藥".to_string(),
        "mat_coarse_paper_ream" => return "粗竹麻紙".to_string(),
        "mat_hemp_rope_roll" => return "浸油麻繩".to_string(),
        "mat_sackcloth_bolt" => return "舊麻袋布".to_string(),
        "mat_kindling_bundle" => return "柴枝束".to_string(),
        "mat_plank_board" => return "木板料".to_string(),
        "mat_charcoal_sack" => return "白炭袋".to_string(),
        "mat_salted_fish" => return "鹽漬鹹魚".to_string(),
        "mat_coarse_salt_pack" => return "粗鹽包".to_string(),
        "craft_herbal_tea_pack" => return "百草煎茶包".to_string(),
        "craft_calming_tea_pack" => return "安神香茶包".to_string(),
        "craft_meridian_salve" => return "活血通絡膏".to_string(),
        "craft_tool_repair_strip" => return "工具修補條".to_string(),
        "craft_patched_work_boots" => return "補綴工靴".to_string(),
        "craft_tarred_hemp_rope" => return "塗焦麻繩".to_string(),
        "craft_caulking_sealant" => return "嵌縫膠泥".to_string(),
        "mat_cedar_plank" => return "杉木厚板".to_string(),
        "craft_pine_torch" => return "松明火把".to_string(),
        "craft_wooden_tool_handle" => return "工具木柄".to_string(),
        "mat_fresh_shrimp" => return "水尾寮鮮沼蝦".to_string(),
        "shanhai_fresh_fish" => return "深海鮮魚".to_string(),
        "shanhai_beast_meat" => return "山海鮮獸肉".to_string(),
        "mat_grain_sack" => return "農家精選穀糧袋".to_string(),
        "mat_pasture_seed_sack" => return "優質牧草種籽袋".to_string(),
        "wt_beast_bone" => return "堅實獸骨".to_string(),
        "wt_down_feather" => return "水鳥蓬鬆絨羽".to_string(),
        _ => {}
    }
    if let Some(mat) = crate::world::materials::material_by_item_id(item_id) {
        if !mat.name.is_empty() {
            return mat.name.clone();
        }
    }
    for def in &STANDARD_FINISHED_GOODS {
        if def.id == item_id {
            return def.name.to_string();
        }
    }
    if let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(item_id) {
        return cat.name.to_string();
    }
    if let Some(eco) = lookup_ecology_item_def(item_id) {
        if !eco.name.is_empty() {
            return eco.name;
        }
    }
    if let Some(fb) = crate::db::fallback_item_name(item_id) {
        return fb.to_string();
    }
    if item_id.is_ascii() {
        "行囊物產".to_string()
    } else {
        item_id.to_string()
    }
}

/// 聚落市場需求 EMA 溫和均值回歸衰減。
///
/// 排除無買家背景吞吐與無人瞬移調撥：真實庫存消耗、產出與跨區調撥全數交由住民因果行動驅動，
/// 此處僅保留需求 EMA 朝向 1.0 的漸進回歸衰減，維持市場信號生命力而不引發幽靈庫存異動。
pub fn execute_settlement_sinks_and_arbitrage(
    cur_settlement: &str,
    _now_unix: i64,
    ledger: &mut EconomyLedger,
) {
    let settlement = normalize_canonical_settlement(cur_settlement);
    for ((s, _item), cell) in ledger.settlement_markets.iter_mut() {
        if s == settlement || s == cur_settlement {
            cell.demand_ema = (cell.demand_ema * 0.99 + 0.01).clamp(0.75, 1.45);
        }
    }
}

/// 判斷物品是否屬於木材、柴火或相關原木料件。
pub fn is_wood_or_firewood_item(item_id: &str) -> bool {
    if matches!(
        item_id,
        "mat_timber_log"
            | "mat_kindling_bundle"
            | "mat_plank_board"
            | "mat_wood_offcut_bundle"
            | "mat_repair_strip"
            | "wt-good-001"
    ) {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            if (56..=110).contains(&num) {
                return true;
            }
        }
    }
    false
}

