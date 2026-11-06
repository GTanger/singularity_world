fn apply_catalog_effects_part4(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now: i64,
    narrative: &mut String,
) -> bool {
    // v0.38.54 Pillar 3: 查閱地籍 (InspectLandRegistry)
    if (obj.id == "wt_deed_cadastral_map_desk"
        || obj.id == "wt_deed_household_registry"
        || obj.name.contains("四坊地籍案")
        || obj.name.contains("樟木戶籍櫃"))
        && action == "InspectLandRegistry"
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "item_wenyao_cadastral_rubbing", 1);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            2,
            now,
        );
        let room_name = db::get_room_name(player_room).unwrap_or_default();
        let inferred = crate::world::materials::infer_origin_settlement(&room_name);
        let settlement = if inferred.is_empty() { "文曜" } else { inferred };
        let treasury = crate::npc::causal_core::settlement_treasury_balance(settlement);
        let prosperity_desc = if treasury >= 2800 {
            "城坊倉廩充盈，四民熙攘安居，公產界石井然有序"
        } else if treasury >= 1800 {
            "四坊市井百工殷實，田宅契據分明，物資流轉平穩有序"
        } else {
            "百業休養生息，街屋界址精細標核，公庫調配量入為出"
        };
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "cadastral_registry_inspected");
        narrative.push_str(&format!(
            "【四坊地籍・民生疆界】你端凝撫按長案上的地籍總圖與樟木戶籍檔案，朱砂經緯與戶號界石標註井然。當前聚落風貌正逢「{}」。你細閱{}坊巷格局與公產脈絡，拓印下一頁「{}地籍圖鈔頁」留念，靜室清息隨之凝練。",
            prosperity_desc, settlement, settlement
        ));
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.55 Pillar 1: 幫廚備膳 (AssistKitchen)
    if (obj.id == "wt_dining_iron_cauldron"
        || obj.id == "wt_dining_rice_milk_stall"
        || obj.name.contains("紅鐵燉肉釜")
        || obj.name.contains("溫米漿桶"))
        && action == "AssistKitchen"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        let _ = db::add_to_inventory(pid, "item_braised_beef_broth_jar", 1);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 15).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ForgeHeavy,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "kitchen_assisted");
        narrative.push_str("【幫廚備膳・市井炊煙】你挽起衣袖在紅鐵燉肉大釜與溫米漿桶前添柴調羹、分盛熱湯，熱氣騰騰的香氣溢滿三水大食堂。掌廚師傅依例自文曜公庫支領 4 鎂幫廚津貼交予你，並盛了一盅熱氣滾滾的「慢燉原牛肉湯陶盅」，爐火重息隨之凝練。");
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.55 Pillar 1: 焙茶品茗 (RoastMountainTea)
    if (obj.id == "wt_inn_milk_tea_stove"
        || obj.name.contains("煮茶泥爐"))
        && action == "RoastMountainTea"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 2 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -2);
            let _ = db::add_to_inventory(pid, "item_roasted_frost_tea_tin", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 16).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::ChamberClear,
                2,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mountain_tea_roasted");
            let nearby_resident = {
                let gh = current_game_hour(&conn.cfg);
                let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
                ents.into_iter().find(|e| {
                    e.id != pid
                        && e.kind == crate::entity::EntityKind::Npc
                        && e.vit > 0
                        && !e.id.starts_with("shanhai_")
                        && !(e.id.starts_with("ambient_")
                            && crate::game::ambient_creature_loot(&e.display_title).is_some())
                })
            };
            if let Some(npc) = nearby_resident {
                let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                    format!("住民【{}】", npc.display_title.replace('|', "・"))
                } else {
                    "同席茶客".to_string()
                };
                crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在山風茶屋圍爐焙茶品茗，閒話風土時務", now);
                let _ = db::adjust_favorability(pid, &npc.id, 4);
                narrative.push_str(&format!(
                    "【山風茶屋・焙茶品茗】你投入 2 鎂炭火茶資存入公庫，於泥爐上輕夾松木炭翻焙高山青茶。炭香微發、茶韻清幽，你將一罐「炭焙霜林烘青茶罐」妥善收妥入囊。身旁的{}共品甘茗、閒話風土，喉舌甘潤心曠神怡，靜室清息隨之凝練。",
                    npc_label
                ));
            } else {
                narrative.push_str("【山風茶屋・焙茶品茗】你投入 2 鎂炭火茶資存入公庫，於泥爐上輕夾松木炭翻焙高山青茶。炭香微發、茶韻清幽，你將一罐「炭焙霜林烘青茶罐」妥善收妥入囊，自斟自品甘醇生津，頓覺塵囂盡散，靜室清息隨之凝練。");
            }
        } else {
            narrative.push_str("【資財不足】山風茶屋圍爐焙茶需交付 2 鎂茶資存入聚落公庫。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.55 Pillar 1: 曝書防蠹 (SunBooks)
    if (obj.id == "wt_book_printing_shelf"
        || obj.id == "wt_study_bamboo_bookshelf"
        || obj.name.contains("風物誌書架")
        || obj.name.contains("線裝古籍架"))
        && action == "SunBooks"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "books_sun_dried");
        narrative.push_str("【石坪曬書・古籍辟蠹】你自架上搬出線裝古籍與風物圖冊，於向陽通風的曬書石坪上依序攤晾翻曬，逐卷拂拭夾入辟蠹芸香。閣中依護書常例自公庫支領 4 鎂曝書津貼交予你，滿坪墨香悠遠，靜室清息隨之凝練。");
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.55 Pillar 2: 炮製藥材 (RoastHerbs)
    if (obj.id == "wt_herb_drying_kiln"
        || obj.id == "wt_herb_bamboo_sieve"
        || obj.name.contains("紅泥焙藥爐")
        || obj.name.contains("切藥鍘刀案"))
        && action == "RoastHerbs"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            let _ = db::add_to_inventory(pid, "item_seven_star_moxa_pack", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 14).min(100);
                    e.qi = (e.qi + 12).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::ForestLong,
                2,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "herbs_roasted_prepared");
            narrative.push_str("【百草炮製・九蒸溫養】你向公庫交付 3 鎂草藥材料費，坐守焙藥爐前掌按文火炭灰，並以鍘刀細切陳艾與黃精。藥材經烘焙透散甘醇香氣，你將精製的一份「九蒸七星艾絨藥包」妥善收妥入囊，山林長息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】炮製草藥需交付 3 鎂藥材料費存入公庫。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.55 Pillar 2: 校勘碑帖 (CollateInscriptions)
    if (obj.id == "wt_study_desk_inkstone"
        || obj.id == "wt_study_bamboo_bookshelf"
        || obj.id == "wt_obj_stone_rubbing_cabinet"
        || obj.name.contains("烏木書案")
        || obj.name.contains("線裝古籍架")
        || obj.name.contains("碑拓石櫥"))
        && action == "CollateInscriptions"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        let _ = db::add_to_inventory(pid, "item_collated_stele_rubbing", 1);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 14).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "inscriptions_collated");
        narrative.push_str("【金石校勘・字句審定】你在烏木書案前對照碑拓石櫥原拓與線裝古籍，細細辨校漫漶殘缺金石字句，硃筆點定異同。學堂依校勘常例由公庫給付 4 鎂勞酬，並將一份「精校金石碑帖拓本」交付於你，靜室清息隨之凝練。");
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.55 Pillar 2: 研磨鏡片 (GrindOpticLens)
    if (obj.id == "wt_obs_brass_telescope_rack"
        || obj.id == "wt_book_map_reading_table"
        || obj.name.contains("青銅窺管架")
        || obj.name.contains("全圖閱覽桌"))
        && action == "GrindOpticLens"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            let _ = db::add_to_inventory(pid, "item_polished_quartz_lens", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 12).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::ChamberClear,
                3,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "optic_lens_ground");
            narrative.push_str("【光學精磨・水晶鏡片】你向公庫交付 3 鎂水晶礦材費，蘸取金剛微砂在旋轉圓盤上細細精磨紅土水晶。冷水潺潺洗去砂痕，鏡面光潔澄澈、纖毫畢現，你將一枚「精磨紅土水晶鏡片」妥善收妥入囊，靜室清息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】研磨光學鏡片需交付 3 鎂水晶耗材費存入公庫。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.55 Pillar 3: 揀選海味 (SelectDriedSeafood)
    if (obj.id == "wt_market_produce_stand"
        || obj.id == "wt_market_scale_counter"
        || obj.name.contains("山海乾貨攤")
        || obj.name.contains("黃銅雙盤秤"))
        && action == "SelectDriedSeafood"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            let _ = db::add_to_inventory(pid, "item_smoked_mullet_roe_pack", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 12).min(100);
                    e.qi = (e.qi + 8).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                2,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "dried_seafood_selected");
            narrative.push_str("【三水口揀選・海味精禮】你交付 3 鎂海產資財入公庫，在攤前細細辨選金黃透亮的冷燻烏魚子，以黃銅雙盤秤精確秤平份量，購得一份「風櫃尾冷燻烏魚子禮包」，海潮綿息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】揀選精裝海味乾貨需交付 3 鎂資財存入公庫。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.55 Pillar 3: 參議鄉約 (DeliberateCivicPact)
    if (obj.id == "wt_hall_council_table"
        || obj.id == "wt_hall_bulletin_board"
        || obj.name.contains("厚木公議桌")
        || obj.name.contains("民務告示欄"))
        && action == "DeliberateCivicPact"
    {
        narrative.clear();
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 14).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "civic_pact_deliberated");
        let nearby_resident = {
            let gh = current_game_hour(&conn.cfg);
            let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
            ents.into_iter().find(|e| {
                e.id != pid
                    && e.kind == crate::entity::EntityKind::Npc
                    && e.vit > 0
                    && !e.id.starts_with("shanhai_")
                    && !(e.id.starts_with("ambient_")
                        && crate::game::ambient_creature_loot(&e.display_title).is_some())
            })
        };
        if let Some(npc) = nearby_resident {
            let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                format!("住民【{}】", npc.display_title.replace('|', "・"))
            } else {
                "與會街坊".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在公議堂同席參議鄉約，共商水土民生大計", now);
            let _ = db::adjust_favorability(pid, &npc.id, 4);
            narrative.push_str(&format!(
                "【公議堂・參議鄉約】你在厚木公議桌前端坐參核鄉約草條，與同席的{}共商水土防護與市井民生，彼此切磋見解、共定保甲章則，靜室清息隨之凝練。",
                npc_label
            ));
        } else {
            narrative.push_str("【公議堂・參議鄉約】你在厚木公議桌前端坐參核鄉約草條，細閱水土修防與四坊民務法度，思慮明晰周詳，靜室清息隨之凝練。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.55 Pillar 3: 汲水潤嗓 (DrawSpringWell)
    if (obj.id == "wt_river_stone_steps"
        || obj.id == "wt_alley_stone_well"
        || obj.name.contains("汲水青石階")
        || obj.name.contains("青石古井"))
        && action == "DrawSpringWell"
    {
        narrative.clear();
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 12).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ForestLong,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "spring_well_drawn");
        let nearby_resident = {
            let gh = current_game_hour(&conn.cfg);
            let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
            ents.into_iter().find(|e| {
                e.id != pid
                    && e.kind == crate::entity::EntityKind::Npc
                    && e.vit > 0
                    && !e.id.starts_with("shanhai_")
                    && !(e.id.starts_with("ambient_")
                        && crate::game::ambient_creature_loot(&e.display_title).is_some())
            })
        };
        if let Some(npc) = nearby_resident {
            let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                format!("住民【{}】", npc.display_title.replace('|', "・"))
            } else {
                "汲水街坊".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在水岸公井旁汲水寒暄，互道街坊安好", now);
            let _ = db::adjust_favorability(pid, &npc.id, 3);
            narrative.push_str(&format!(
                "你立於汲水青石階旁，搖動轆轤打上一桶沁涼甘泉，舀起清冽泉水痛飲潤嗓。身旁的{}提桶寒暄、互道街坊安好，清涼直透腑臟，山林長息凝練，鄰里情誼漸厚。",
                npc_label
            ));
        } else {
            narrative.push_str("你立於汲水青石階旁，搖動轆轤打上一桶沁涼甘泉，舀起清冽泉水痛飲潤嗓，甘澈清泉直透腑臟，胸中燥熱頓消，山林長息凝練。");
        }
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_street_grindstone_trough" || obj.name.contains("磨刀石") || obj.name.contains("水槽與磨刀石") || obj.name.contains("磨刀槽"))
        && (action == "Use" || action == "Operate")
    {
        let vessel_target = if let Ok(Some(ch)) = db::get_entity(pid) {
            let mut found = None;
            if !ch.equipment_slots.is_empty() {
                if let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(&ch.equipment_slots) {
                    let check_slots = ["weapon", "hold_r", "main_hand", "hold", "tool", "hold_l"];
                    for s in check_slots {
                        if let Some(id) = slots.get(s).filter(|id| !id.is_empty()) {
                            let name = db::get_item_def(id).map(|d| d.name).unwrap_or_else(|| id.clone());
                            if crate::pg::living_world::is_bladed_vessel_item(id, &name) {
                                found = Some((id.clone(), name));
                                break;
                            }
                        }
                    }
                }
            }
            if found.is_none() {
                let inv_items: Vec<serde_json::Value> = serde_json::from_str(&ch.inventory).unwrap_or_default();
                for it in inv_items {
                    let id = it.get("item_id").or_else(|| it.get("id")).and_then(|v| v.as_str()).unwrap_or_default();
                    if id.is_empty() { continue; }
                    let name = db::get_item_def(id).map(|d| d.name).unwrap_or_else(|| id.to_string());
                    if crate::pg::living_world::is_bladed_vessel_item(id, &name) {
                        found = Some((id.to_string(), name));
                        break;
                    }
                }
            }
            found
        } else {
            None
        };

        if let Some((item_id, item_name)) = vessel_target {
            match crate::pg::living_world::sharpen_vessel_at_grindstone(pid, &item_id, now) {
                Ok(state) => {
                    let _ = event::append(now, pid, types::PLAYER_BODY, "blade_sharpened");
                    narrative.push_str(&format!(
                        "\n【利刃磨礪・鋒芒重現】你舀起青石水槽中的清冽井水淋在磨刀石上，將「{}」伏貼石面細細推磨。水珠混著青灰石漿滑落，刃口寒芒流轉、重歸「{}」！（人器磨合：{} —— {}）",
                        item_name,
                        state.condition.label(),
                        state.familiarity().label(),
                        state.familiarity().feel_prose()
                    ));
                }
                Err(msg) => {
                    let clean_msg = msg.trim_end_matches('。');
                    narrative.push_str(&format!(
                        "\n【利刃磨礪】推磨數下後，你發現「{}」{}。此類捲刃待修重損，街邊尋常青石磨刀槽難以推平，須赴鍛造工坊藉由鍛爐⚒︎火候與鐵砧輕敲校直才能修整復原。",
                        item_name, clean_msg
                    ));
                }
            }
        } else {
            narrative.push_str("\n【利刃磨礪】你舀起青石水槽中的清冽井水淋在磨刀石上，但隨身並未持握或攜帶刀劍鋒刃器具，只得任清水自青石表面滑落。");
        }
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_street_platform_scale" || obj.name.contains("地秤") || obj.name.contains("過磅"))
        && (action == "Operate" || action == "Use")
    {
        let (item_count, total_weight_jin) = if let Ok(Some(ch)) = db::get_entity(pid) {
            let items: Vec<serde_json::Value> = serde_json::from_str(&ch.inventory).unwrap_or_default();
            let count: usize = items.iter().filter_map(|v| v.get("qty").and_then(|q| q.as_u64())).sum::<u64>() as usize;
            let mg_weight = ch.magnesium / 50;
            let base_jin = 110 + (count as i32 * 3) + mg_weight;
            (count, base_jin)
        } else {
            (0, 110)
        };
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "scale_weighed");
        narrative.push_str(&format!(
            "\n【地秤過磅】你邁步踏上西門過磅生鐵地秤，沉重秤桿在熟鐵秤砣滑動間發出沉實金屬聲。\n過磅司秤報數：隨身攜帶物件共 {item_count} 件，合算隨身行囊與衣裝，過磅總重為【{total_weight_jin} 斤四兩】。秤台承重沉穩，符合西門聯運車隊標準裝載規範。"
        ));
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_alley_stone_well"
        || (obj.name.contains("水槽")
            && !obj.name.contains("熱水槽")
            && !obj.name.contains("溫石")
            && !obj.name.contains("淬火")
            && !obj.name.contains("飲馬槽")
            && obj.id != "wt_inn_bath_Preview"
            && obj.id != "wt_caravan_water_trough")
        || obj.name.contains("公井")
        || obj.name.contains("汲水木桶"))
        && (action == "Use" || action == "Wash" || action == "Drink" || action == "Operate")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 8).min(100);
                e.qi = (e.qi + 8).min(100);
            });
        }
        let _ = event::append(now, pid, types::PLAYER_BODY, "well_washed");
        narrative.push_str("\n【清泉洗漱】你搖動轆轤打起半桶深井甘泉，清冽泉水撲面而來，洗去了一身風塵與倦意，渾身毛孔透徹舒泰。");
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_deed_cadastral_map_desk"
        || obj.id == "wt_deed_household_registry"
        || obj.name.contains("地籍")
        || obj.name.contains("總譜")
        || obj.name.contains("戶籍"))
        && (action == "Read" || action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "registry_checked");
        narrative.push_str("\n【查閱地籍戶譜】你仔細翻閱公議堂案几上的地籍總圖與落籍戶譜。各坊街屋坐標界石標註分明，田畝產權清晰可稽，令人對文曜建置井然肅然起敬。");
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_obs_tide_weather_table"
        || obj.id == "wt_obs_brass_telescope_rack"
        || obj.name.contains("潮汐")
        || obj.name.contains("望遠銅鏡"))
        && (action == "Read" || action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "observation_studied");
        narrative.push_str("\n【觀測天象潮汐】你凝神查閱四時潮汐氣象推算案並透過雙筒望遠銅鏡俯瞰海天交界。風雲變幻與水運起落盡收眼底，胸懷頓時為之開闊。");
        push_refresh(conn, pid);
        return true;
    }
    // 2.6) Wenyao Interactive Riverbank, Farming & Lighting
    if (obj.id == "wt_reg_wenyao_riverbank" || obj.name.contains("平底竹筏") || obj.name.contains("繫船青石樁"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "bamboo_raft_poled");
        let room_name = resolve_room_name(player_room);
        if room_name.contains("風舶角") || room_name.contains("海") || room_name.contains("港") {
            narrative.push_str("\n【乘筏觀潮】你解開繫在石樁上的粗纜，平底木筏隨著港灣潮汐輕輕起伏，海風獵獵拂面，遠眺海天一色，心神頓覺開闊澄明。");
        } else {
            narrative.push_str("\n【乘筏信步】你解開繫在青石樁上的桐油麻繩，手持長竹篙在石階輕撐借力，平底竹筏滑入三水川清澈碧流。沿岸水草豐美、水鳥掠波，心神隨流水漸入澄澈。");
        }
        append_somatic_awareness_note(pid, now, narrative);
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_reg_wenyao_garden"
        || obj.id == "wt_reg_wenyao_courtyard"
        || obj.name.contains("藥圃")
        || obj.name.contains("盆栽")
        || obj.name.contains("引水筧")
        || obj.name.contains("山茶"))
        && (action == "Water" || action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "plants_watered");
        narrative.push_str("\n【細心灌溉】清涼甘泉緩緩注入泥土，泥土與草木散發出質樸而清新的生機芬芳。靜心照拂草木之間，心神漸趨安寧。");
        if obj.name.contains("藥圃") || obj.id == "wt_reg_wenyao_garden" {
            let _ = db::add_to_inventory(pid, "mat_foraged_herb", 1);
            narrative.push_str("\n【草木採收】你在藥圃中細心摘取了一份清香四溢的「野生採集草藥」收入行囊。");
        }
        push_refresh(conn, pid);
        return true;
    }

    if (action == "Use" || action == "Operate")
        && (obj.id == "wt_home_shell_lamp" || obj.name.contains("台燈") || obj.name.contains("檯燈"))
        && !obj.id.contains("signpost")
        && !obj.name.contains("路燈")
        && !obj.name.contains("路標")
        && obj.id != "wt_reg_wenyao_home"
        && !obj.name.contains("大床")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 5).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "lamp_dimmed");
        narrative.push_str("\n【旋燈調光】你輕輕旋轉床頭夜光貝母檯燈的烏木旋蓋，柔和溫潤的珠白光芒隨之微調，滿室皆是安恬暖意，心神漸入安恬。");
        push_refresh(conn, pid);
        return true;
    }

    // 2.7) Wenyao Academy, Study, Printing, Medicine & Riverbank Interactivity
    if (obj.id == "wt_study_desk_inkstone" || obj.name.contains("沉水烏木書案") || obj.name.contains("端硯"))
        && (action == "Operate" || action == "Use" || action == "Read")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "study_desk_inkstone_used");
        narrative.push_str("\n【研墨展卷】你取墨錠在端硯清泉中徐徐研磨，墨香四溢。隨後提筆蘸墨在宣紙上信手書寫，筆意沉著，心神豁然澄澈。");
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_study_bamboo_bookshelf" || obj.name.contains("竹編書簍") || obj.name.contains("古籍架"))
        && (action == "Operate" || action == "Use" || action == "Read")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "study_bamboo_bookshelf_used");
        narrative.push_str("\n【檢閱典籍】你自架上抽出一冊線裝古籍翻閱，紙質柔韌吸墨，墨香沁人心脾。先賢微言大義躍然紙上，令人心曠神怡。");
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_astro_brass_armillary" || obj.name.contains("渾天星軌") || obj.name.contains("推步儀"))
        && (action == "Operate" || action == "Use" || action == "Read")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "astro_brass_armillary_used");
        narrative.push_str("\n【星軌推步】你推動星軌儀的精銅外環，齒輪與軸承咬合平順，星軌刻度隨之流轉對齊今日天象，心中豁然開朗。");
        if let Ok((_mut_id, _mut_name, mut_narrative)) =
            crate::game::object_ecology::mutate_finished_item(pid, "auto", &obj.name, now)
        {
            narrative.push_str(&format!("\n【星軌氣脈共鳴異變】在星環轉動與星光垂照之下，{}", mut_narrative));
        }
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_math_abacus_desk" || obj.name.contains("算盤") || obj.name.contains("商算習題案"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "math_abacus_used");
        narrative.push_str("\n【撥珠速算】你信手撥動黑檀算珠，珠落聲清脆利落如落玉盤。心中速算推演，思維條理頓時無比清晰。");
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_print_movable_type" || obj.name.contains("活字排版") || obj.name.contains("墨刷案"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "movable_type_printed");
        narrative.push_str("\n【排版試印】你拿起棕櫚墨刷在排版活字上勻勻刷上一層松煙墨，覆上宣紙輕輕滾壓，字跡頓時端正顯映於紙上。");
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_paper_bamboo_vat" || obj.name.contains("抄紙石槽") || obj.name.contains("焙紙熱牆"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "paper_bamboo_vat_used");
        narrative.push_str("\n【蕩料抄紙】你雙手執竹簾浸入石槽，平順蕩料抄起一層細密纖維，揭紙貼於熱牆烘乾，紙面瑩白平滑，工藝之妙令人讚歎。");
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_herb_drying_kiln" || obj.name.contains("百草焙藥爐") || obj.name.contains("焙藥爐"))
        && (action == "Operate" || action == "Use" || action == "WarmHands")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 10).min(100);
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "herb_kiln_tended");
        narrative.push_str("\n【焙藥調溫】你撥動紅泥爐膛的通氣瓦片微調火候，熱氣蒸騰間草木本質被徐徐焙透，濃郁藥香透體而入，體魄與精神皆為之一振。");
        if let Ok((_item_id, _item_name, _is_mut, craft_narrative)) =
            crate::game::object_ecology::craft_from_raw_material(pid, "auto", &obj.name, now)
        {
            narrative.push_str(&format!("\n【工造煉製】在伺火焙藥之際，{}", craft_narrative));
        }
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_herb_bamboo_sieve" || obj.name.contains("切藥鍘刀") || obj.name.contains("晾藥篩架"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "herb_sieve_cut");
        narrative.push_str("\n【操刀切藥】你手握切藥鍘刀木柄壓下，利刃切開晾乾的草藥根莖，發出清脆嚓響，藥香濃烈撲面，筋骨隨之舒展。");
        if let Ok((_item_id, _item_name, _is_mut, craft_narrative)) =
            crate::game::object_ecology::craft_from_raw_material(pid, "auto", &obj.name, now)
        {
            narrative.push_str(&format!("\n【工造煉製】在操刀切藥之際，{}", craft_narrative));
        }
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_river_stone_steps" || obj.name.contains("汲水埠階") || obj.name.contains("竹編魚簍"))
        && (action == "Operate" || action == "Use" || action == "Wash" || action == "Rest" || action == "Meditate")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 5).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "river_steps_washed");
        if narrative.is_empty() {
            narrative.push_str("【埠階掬泉】你踏上青石汲水埠階，俯身掬起清涼甘洌的川流活水洗滌面容，暑氣盡散，神清氣爽。");
        }
        if action == "Gather" || action == "Operate" {
            let _ = db::add_to_inventory(pid, "mat_iron_scrap", 1);
            narrative.push_str("\n【岸畔淘料】你在埠階水邊清淤淘洗，順道自泥沙礫石中淘獲一塊堅實的【回收熟鐵料塊】收入行囊。");
        }
        if action == "Meditate" {
            append_somatic_awareness_note(pid, now, narrative);
        }
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_bamboo_tea_table" || obj.name.contains("竹林青石茶案") || obj.name.contains("石凳"))
        && (action == "Operate" || action == "Use" || action == "Rest" || action == "Taste" || action == "Meditate")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "bamboo_tea_rested");
        narrative.push_str("\n【竹苑啜茗】你在微涼青石凳上端坐歇息，以竹筧甘泉瀹茶，修竹沙沙入盞，頓覺身心放鬆，塵慮盡消。");
        if action == "Meditate" {
            append_somatic_awareness_note(pid, now, narrative);
        }
        push_refresh(conn, pid);
        return true;
    }

    if obj.id == "wt_sawmill_woodchip_bin" && action == "Gather" {
        let _ = db::add_to_inventory(pid, "mat_kindling_bundle", 1);
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "sawmill_woodchip_gathered");
        narrative.push_str("\n\n【拾取刨花引柴】你順手將一束乾燥易燃的【乾燥柴枝束】妥善收入行囊。");
        push_refresh(conn, pid);
        return true;
    }

    if (obj.id == "wt_clinic_herb_cabinet" && action == "Use")
        || (obj.id == "wt_clinic_decoction_stove" && action == "WarmHands")
        || ((obj.id == "wt_clinic_acupoint_bed" || obj.id == "wt_arena_rest_bench") && (action == "Rest" || action == "Meditate"))
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 15).min(100);
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "outer_settlement_rested");
        narrative.push_str("\n\n【調息溫養】草木溫潤之氣順著經絡緩緩化開，周身疲乏消散，體魄與心神皆獲調養。");
        if action == "Meditate" {
            append_somatic_awareness_note(pid, now, narrative);
        }
        push_refresh(conn, pid);
        return true;
    }
    false
}
