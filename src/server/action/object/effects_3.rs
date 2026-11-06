fn apply_catalog_effects_part3(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now: i64,
    narrative: &mut String,
) -> bool {
    // Pillar 2: Nanshao Miasma Preparation & Shuiweiliao Bamboo Raft Marsh Salvage
    // 1. 備妥避瘴器物 (wt_hunt_* / wt_clinic_herb_cabinet)
    if (obj.id.starts_with("wt_hunt_")
        || obj.id == "wt_clinic_herb_cabinet"
        || obj.name.contains("牛角破甲弩")
        || obj.name.contains("帶齒鋼獸夾")
        || obj.name.contains("百草丸藥櫃"))
        && (action == "PrepareMiasmaGear" || action == "Acquire")
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 4 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -4);
            let _ = db::add_to_inventory(pid, "wt_cat_1055", 1);
            let _ = db::add_to_inventory(pid, "wt_cat_1014", 1);
            crate::pg::living_world::clear_entity_afflictions(pid, true, now);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 10).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "prepare_miasma_gear");
            narrative.push_str("【備妥避瘴器物】你向聚落公庫交付 4 鎂採買避瘴防護物資，配齊了清冽提神的「白茫避瘴清心丸」與指引濃霧方位的「紅磷十丈引路信號筒」。服下一枚清心丸後，周身經絡微涼舒暢，驅散一切瘴癘毒濁，內息充盈穩固。");
        } else {
            narrative.push_str("【資財不足】隨身資財不足，備辦避瘴器物需 4 鎂存入聚落公庫以補物資消耗。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // 2. 水尾寮竹筏撐筏入澤 (wt_dock_riverbank_raft)
    if (obj.id == "wt_dock_riverbank_raft" || obj.name.contains("平底竹筏"))
        && (action == "RowSwampRaft" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let has_miasma_pill = if let Ok(Some(ch)) = db::get_entity(pid) {
            let inv = db::get_inventory(&ch.inventory, ch.vit);
            inv.items.iter().any(|it| (it.item_id == "wt_cat_1055" || it.item_id == "wt_cat_1014") && it.qty >= 1)
        } else {
            false
        };
        let _ = db::add_to_inventory(pid, "wt_cat_0058", 1);
        let _ = db::add_to_inventory(pid, "wt_cat_0113", 1);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "row_swamp_raft_salvage");

        if has_miasma_pill {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 8).min(100);
                });
            }
            narrative.push_str("【水尾寮撐筏入澤】你解開桐油麻繩點篙撐筏，輕舟輕巧破開水尾寮蘆葦與黑沼迷霧。你在深澤暗流中打撈起堅硬不腐的「沉水烏木原材」，並採集到新鮮柔韌的「淺灘止血水苔」。懷中「白茫避瘴清心丸」藥氣清冽，逼退周遭陰濕穢霧，呼吸清爽通透。");
        } else {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi.saturating_sub(3)).max(10);
                });
            }
            narrative.push_str("【水尾寮撐筏入澤】你解開桐油麻繩點篙撐筏，深入蘆葦水網與黑沼澤畔，打撈起珍貴的「沉水烏木原材」與「淺灘止血水苔」。然而澤中暗霧濕冷透骨，因未備「白茫避瘴清心丸」護體，胸口微感陰濕滯悶。建議前往哨所或診所【備妥避瘴】。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // Pillar 3: Calendar-Coupled Seasonal Festival Banquets & Tea-House Ebony-Shell Chess Fellowship
    // 1. 歲時圍爐 (wt_dining_iron_cauldron / wt_reg_plains_specialty / wt_reg_mining_specialty)
    if (obj.id == "wt_dining_iron_cauldron"
        || obj.id == "wt_reg_plains_specialty"
        || obj.id == "wt_reg_mining_specialty"
        || obj.name.contains("紅鐵燉肉釜")
        || obj.name.contains("陳年米酒甕")
        || obj.name.contains("烤番薯砂鍋"))
        && (action == "SeasonalBanquet" || action == "Taste" || action == "Use")
    {
        narrative.clear();
        let (_, _, _, raw_days) = crate::game::game_time_now(now, conn.cfg.game_time_epoch_unix, conn.cfg.game_time_scale);
        let season_name = crate::game::game_season_label(raw_days);
        let dish_name = match season_name {
            "春融" | "春霖" => "艾草青糰與鮮筍春捲席",
            "夏漲" | "夏炎" => "寒水玉冰鎮仙草凍與鮮魚羹席",
            "秋狩" => "大圳頭新米團圓宴與桂花新米糕席",
            "冬凝" => "半嶺黑芝麻湯圓與紫銅鴛鴦鍋守歲席",
            _ => "大圳頭新米團圓宴與桂花新米糕席",
        };

        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 20).min(100);
                    e.qi = (e.qi + 15).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "seasonal_banquet_feasted");

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
                    "同席街坊".to_string()
                };
                let summary = format!("圍爐共享「{}」，相談甚歡", dish_name);
                crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, &summary, now);
                let _ = db::adjust_favorability(pid, &npc.id, 5);
                narrative.push_str(&format!(
                    "【歲時圍爐・時令節慶】當前時節正逢「{}」。你投入 3 鎂資財置辦節慶圍爐，熱氣騰騰的「{}」香氣撲鼻。席間你與同處一堂的{}圍坐共食、把盞相談，彼此情誼漸篤，四肢百骸俱生暖意。",
                    season_name, dish_name, npc_label
                ));
            } else {
                narrative.push_str(&format!(
                    "【歲時圍爐・時令節慶】當前時節正逢「{}」。你投入 3 鎂資財置辦節慶圍爐，品嚐熱氣蒸騰的時令佳餚「{}」，甘醇美味入喉，驅散長途跋涉風寒，周身元氣充沛舒泰。",
                    season_name, dish_name
                ));
            }
        } else {
            narrative.push_str("【資財不足】隨身資財不足，歲時圍爐入席需 3 鎂存入公庫置辦時令食材。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // 2. 茶屋烏木棋秤對弈 (wt_inn_ebony_shell_chess)
    if (obj.id == "wt_inn_ebony_shell_chess" || obj.id == "wt_cat_0795" || obj.name.contains("烏木棋盤") || obj.name.contains("圍棋") || obj.name.contains("白貝棋"))
        && (action == "PlayChess" || action == "Operate" || action == "Use")
    {
        narrative.clear();
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
                "對坐棋客".to_string()
            };
            let _ = db::adjust_favorability(pid, &npc.id, 5);
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在茶屋烏木棋秤前對弈手談，棋路相契", now);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 12).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "chess_fellowship_played");
            narrative.push_str(&format!(
                "【茶屋手談・落子同參】你端坐於烏木棋盤前，與席間{}分執黑白。盤上烏木黑子與厚海貝白子起落有致，推演虛實進退，彼此以棋會友相得益彰。局罷相視一笑，心神空明篤定，街坊情誼益發投契。",
                npc_label
            ));
        } else {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 8).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "chess_solo_studied");
            narrative.push_str("【茶屋獨弈・打譜推演】棋案空幽，你獨對烏木棋秤擺下殘局古譜。一人分飾黑白參悟攻防變化，篤篤落子之聲清脆迴響於茶舍之中，胸中雜慮盡消，內息澄澈流轉。");
        }
        push_refresh(conn, pid);
        return true;
    }
    // v0.38.53 Pillar 1: 觀星拓印・周天命格星盤 (ChartAstrolabe)
    if (obj.id == "wt_cat_0734" || obj.id.starts_with("wt_astro_") || obj.id == "wt_obj_armillary_almanac" || obj.name.contains("星軌儀") || obj.name.contains("推步儀") || obj.name.contains("定緯銅儀"))
        && action == "ChartAstrolabe"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            let _ = db::add_to_inventory(pid, "item_personal_astrolabe_scroll", 1);
            let _ = db::add_to_inventory(pid, "wt_cat_0783", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 12).min(100);
                });
            }
            let (_, _, _, raw_days) = crate::game::game_time_now(now, conn.cfg.game_time_epoch_unix, conn.cfg.game_time_scale);
            let season_name = crate::game::game_season_label(raw_days);
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "astrolabe_chart_rubbed");
            narrative.push_str(&format!(
                "【觀星拓印・周天命格星盤】你交付 3 鎂工本資財存入公庫，備齊竹綿宣紙與松煙精墨，細緻拓印黃銅星軌儀上的周天宿度。當前歲時正臨「{}」，星軌盤面澄澈顯映個人筋骨與山海氣脈的共鳴分野。你收穫一份「先天周天命格星盤拓卷」與「北辰歲差推步星圖」，內息亦隨天行周天而生生不息。",
                season_name
            ));
        } else {
            narrative.push_str("【資財不足】拓印個人周天命格星盤需 3 鎂工本資財存入公庫以備竹綿宣紙與松煙精墨。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.53 Pillar 1: 分家測紋・器脈校準 (TestEngraving)
    if (obj.id == "wt_martial_force_stone" || obj.id == "wt_forge_quench_trough" || obj.name.contains("測勁玄武碑") || obj.name.contains("導能測紋石板"))
        && action == "TestEngraving"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "test_engraving_calibrated");
        narrative.push_str("【分家測紋・器脈校準】你運轉周身氣息按向嵌有暗金紋理的測勁玄武碑，將隨身器物輕觸碑面導能凹槽。石板發出低沉清越的共鳴回響，器物重心與刃口紋路隨之校準歸正。工坊由聚落公庫撥付 4 鎂測紋津貼，內息運轉愈發圓融。");
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.53 Pillar 1: 試招拆勁・四序演武 (SparPillar)
    if (obj.id == "wt_arena_testing_pillar" || obj.id == "wt_martial_padded_wooden_blade" || obj.id == "wt_arena_wooden_weapon_rack" || obj.name.contains("玄武試勁柱"))
        && action == "SparPillar"
    {
        narrative.clear();
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 8).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "spar_pillar_trained");
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
                "同場武伴".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在試招場玄武試勁柱前切磋拆招，勁路相印", now);
            let _ = db::adjust_favorability(pid, &npc.id, 4);
            narrative.push_str(&format!(
                "【試招拆勁・四序演武】你繞著烏石鼻玄武試勁柱沉腰落馬，依序演練「起勢・蓄勁、進步・破鋒、轉樞・卸力、收勢・歸元」四段勁路，掌風與木刃擊打石柱激起沉雄悶響。同處試武場的{}凝神觀摩印證，彼此切磋拆招頗多得益，同道情誼漸深。",
                npc_label
            ));
        } else {
            narrative.push_str("【試招拆勁・四序演武】你繞著烏石鼻玄武試勁柱沉腰落馬，依序演練「起勢・蓄勁、進步・破鋒、轉樞・卸力、收勢・歸元」四段勁路，掌風與木刃擊打石柱激起沉雄悶響，周身筋骨舒張、內息順暢通達。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.53 Pillar 2: 泉香湯浴・祛寒舒筋 (SoakHotSpring)
    if (obj.id == "wt_bath_jade_pool" || obj.id == "wt_home_hotspring_tub" || obj.name.contains("恆溫熱湯池") || obj.name.contains("杉木沐浴桶"))
        && action == "SoakHotSpring"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            crate::pg::living_world::clear_entity_afflictions(pid, true, now);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 25).min(100);
                    e.qi = (e.qi + 20).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "soak_hot_spring_refreshed");
            narrative.push_str("【泉香湯浴・祛寒舒筋】你向浴場公庫交付 3 鎂浴資，浸入半嶺寒水玉鋪底、溫氏祛寒草木湯包浸泡的恆溫熱湯池中。溫潤泉水滲透肌膚經絡，拔除周身寒濕與跋涉勞損，四肢百骸暖意融融。");
        } else {
            narrative.push_str("【資財不足】進入熱湯池需 3 鎂存入浴場公庫以備溫泉引水與草木湯包。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.53 Pillar 2: 市井修面・煥然一新 (GroomAppearance)
    if (obj.id.starts_with("wt_barber_") || obj.id == "wt_bath_mirror_washstand" || obj.name.contains("摺疊剃刀架") || obj.name.contains("洗頭木躺椅") || obj.name.contains("銀鏡盥洗台"))
        && action == "GroomAppearance"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 2 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -2);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 14).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "groom_appearance_trimmed");
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
                    "鋪內師傅".to_string()
                };
                crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在理髮修面鋪修整儀容，談吐清爽投契", now);
                let _ = db::adjust_favorability(pid, &npc.id, 4);
                narrative.push_str(&format!(
                    "【市井修面・煥然一新】你向聚落公庫交付 2 鎂修面資費，敷上蒸騰著薄荷清香的熱毛巾，由{}以精鋼摺疊剃刀與松香木梳修整鬢角鬚髮，談吐相談甚歡，神采頓時清爽端正。",
                    npc_label
                ));
            } else {
                narrative.push_str("【市井修面・煥然一新】你向聚落公庫交付 2 鎂修面資費，敷上蒸騰著薄荷清香的熱毛巾，以精鋼摺疊剃刀與松香木梳修整鬢角鬚髮，神采頓時清爽端正。");
            }
        } else {
            narrative.push_str("【資財不足】修面整容需 2 鎂資費存入聚落公庫以補毛巾草皂消耗。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.53 Pillar 2: 街角補綴・行裝牢固 (MendTravelGear)
    if (obj.id.starts_with("wt_repair_") || obj.id == "wt_cloth_tailor_sewing_bench" || obj.name.contains("修傘木砧檯") || obj.name.contains("補靴硬木楦") || obj.name.contains("裁縫量身几"))
        && action == "MendTravelGear"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 2 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -2);
            let _ = db::add_to_inventory(pid, "item_oiled_bamboo_umbrella", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 8).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mend_travel_gear_repaired");
            narrative.push_str("【街角補綴・行裝牢固】你交付 2 鎂工料費存入聚落公庫，在木砧檯與硬木楦前為隨身行靴敲緊生鐵底釘、抹勻厚魚油，並領取一把重新刷過熟桐油的「雙層桐油防雨竹傘」，足底與行囊俱皆穩妥。");
        } else {
            narrative.push_str("【資財不足】補綴行裝需 2 鎂工料資費存入聚落公庫以備生鐵釘與熟桐油。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.53 Pillar 3: 半嶺測候・山海風潮 (ReadWindVane)
    if (obj.id == "wt_observatory_anemometer" || obj.id == "wt_obj_wind_vane_gauge" || obj.id == "wt_obs_tide_weather_table" || obj.name.contains("黃銅風向儀") || obj.name.contains("相風銅烏"))
        && action == "ReadWindVane"
    {
        narrative.clear();
        let _ = current_game_hour(&conn.cfg);
        let (_, _, _, raw_days) = crate::game::game_time_now(now, conn.cfg.game_time_epoch_unix, conn.cfg.game_time_scale);
        let season_name = crate::game::game_season_label(raw_days);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 8).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "read_wind_vane_logged");
        narrative.push_str(&format!(
            "【半嶺測候・山海風潮】你仰觀八方黃銅風向儀與相風銅烏轉速，比對滴漏水鐘與潮汐曲線：當前正值「{}」，山海氣流穩定，外海潮位正處平順航期，十九獵場視野清明。觀測所由聚落公庫撥付 4 鎂測候記錄津貼。",
            season_name
        ));
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.53 Pillar 3: 溫氏百草・艾火溫灸 (ApplyMoxibustion)
    if (obj.id == "wt_clinic_acupoint_bed" || obj.id == "wt_clinic_decoction_stove" || obj.id == "wt_obj_acupuncture_pulse_pillow" || obj.name.contains("竹席診脈榻") || obj.name.contains("溫灸盒"))
        && action == "ApplyMoxibustion"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            crate::pg::living_world::clear_entity_afflictions(pid, true, now);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 22).min(100);
                    e.qi = (e.qi + 16).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "apply_moxibustion_healed");
            narrative.push_str("【溫氏百草・艾火溫灸】你向診所公庫交付 3 鎂診金，臥於細編竹席診脈榻上，由司藥點燃陳年艾絨置於溫灸盒溫熨關元與曲池穴。溫厚藥力透入經絡，將筋骨扭挫與陰寒鬱結盡數化開。");
        } else {
            narrative.push_str("【資財不足】百草溫灸需 3 鎂診金存入診所公庫以置辦陳艾與藥油。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.53 Pillar 3: 北嶺古杉・懸掛銘牌 (HangVotivePlaque)
    if (obj.id == "wt_obj_mountain_milestone" || obj.id == "wt_shrine_votive_plaque" || obj.name.contains("山道指路碑") || obj.name.contains("祈福木牌"))
        && action == "HangVotivePlaque"
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "item_spruce_votive_plaque", 1);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "hang_votive_plaque_inscribed");
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
                "同行山客".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在北嶺古杉指路碑前一同懸掛祈福木牌，互道平安", now);
            let _ = db::adjust_favorability(pid, &npc.id, 4);
            narrative.push_str(&format!(
                "【北嶺古杉・懸掛銘牌】你取過散發天然松脂清香的百年雲杉木牌，提筆題寫行旅平安祈願並懸於山道指路碑旁，同時收起一枚隨身「北嶺雲杉祈福木牌」。身旁的{}亦拱手致意、互道平安，山風拂過林梢，萬枚木牌發出清越柔和的共鳴。",
                npc_label
            ));
        } else {
            narrative.push_str("【北嶺古杉・懸掛銘牌】你取過散發天然松脂清香的百年雲杉木牌，提筆題寫行旅平安祈願並懸於山道指路碑旁，同時收起一枚隨身「北嶺雲杉祈福木牌」。山風拂過林梢，萬枚木牌發出清越柔和的共鳴。");
        }
        push_refresh(conn, pid);
        return true;
    }
    // v0.38.54 Pillar 1: 鈔錄孤本 (TranscribeRareTome)
    if (obj.id == "wt_book_printing_shelf"
        || obj.id == "wt_book_pen_notebook_desk"
        || obj.name.contains("風物誌書架")
        || obj.name.contains("蘸水硬筆書案"))
        && action == "TranscribeRareTome"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            let _ = db::add_to_inventory(pid, "item_white_tower_gazetteer_scroll", 1);
            let _ = db::add_to_inventory(pid, "wt_cat_0727", 1);
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
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "rare_tome_transcribed");
            narrative.push_str("【藏書鈔錄・風物孤本】你向公庫交付 3 鎂工本資財，引點「守歲松脂蜂蠟紅燭」，以鋼尖蘸水硬筆在竹綿宣紙上謹飭抄錄《白塔風物誌》地方孤本。燭火微明、墨香沉靜，你收穫「白塔風物孤本鈔卷」與一挺「守歲松脂蜂蠟紅燭」，胸臆神思朗潤澄澈，靜室清息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】抄錄地方孤本需 3 鎂工本資財存入公庫以備竹綿宣紙、精墨與紅燭。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.54 Pillar 1: 旁聽策論 (AttendLecture)
    if (obj.id == "wt_academy_fountain_pen"
        || obj.id == "wt_book_pen_notebook_desk"
        || obj.name.contains("黃銅儲墨筆")
        || obj.name.contains("蘸水硬筆書案"))
        && action == "AttendLecture"
    {
        narrative.clear();
        let (_, _, _, raw_days) = crate::game::game_time_now(now, conn.cfg.game_time_epoch_unix, conn.cfg.game_time_scale);
        let season_name = crate::game::game_season_label(raw_days);
        let season_topic = match season_name {
            "春融" | "春霖" => "春融農事興修與堰道疏濬",
            "夏漲" | "夏炎" => "夏漲水運防汎與海防汛期",
            "秋狩" => "秋狩倉廩積穀與四坊財賦",
            "冬凝" => "冬凝工坊溫度與民生炭薪",
            _ => "春融農事興修與堰道疏濬",
        };

        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 15).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "academy_lecture_attended");

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
                "同堂學子".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在講堂一同旁聽策論，研討經世時務", now);
            let _ = db::adjust_favorability(pid, &npc.id, 4);
            narrative.push_str(&format!(
                "【講堂策論・同窗共學】講席正逢「{}」歲時，教習與諸生正就「{}」各陳策論。你端坐旁聽、手錄精要，身旁的{}亦頷首會心，彼此就民生物用各抒己見、獲益良多，靜室清息凝練而同窗情誼益篤。",
                season_name, season_topic, npc_label
            ));
        } else {
            narrative.push_str(&format!(
                "【講堂策論・同窗共學】講席正逢「{}」歲時，教習正就「{}」深論經世之要。你端坐側席靜聆論辯，執筆詳錄策論節略，心神融會貫通，靜室清息隨之凝練。",
                season_name, season_topic
            ));
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.54 Pillar 1: 校時報刻 (CalibrateClock)
    if (obj.id == "wt_clocktower_bronze_bell"
        || obj.id == "wt_messenger_lookout_perch"
        || obj.name.contains("報時青銅鐘")
        || obj.name.contains("青銅旋轉晷"))
        && action == "CalibrateClock"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
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
        let gh = current_game_hour(&conn.cfg);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "clock_calibrated_chimed");
        narrative.push_str(&format!(
            "【鐘樓校時・日晷報刻】你依循日晷影針與漏刻核驗天行度數，精準校正鐘樓發條，合力牽動粗麻繩敲響報時青銅鐘。清越浩蕩的鐘聲沉渾播揚全境，正是「{}時整」。司時司自公庫撥付 4 鎂津貼發放入你手中。",
            gh
        ));
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.54 Pillar 2: 榫卯校修 (FitMortiseTenon)
    if (obj.id == "wt_wood_planing_bench" || obj.name.contains("刨木工作台"))
        && action == "FitMortiseTenon"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 5);
        let _ = db::add_to_inventory(pid, "item_ebony_mortise_tea_tray", 1);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 8).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ForgeHeavy,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mortise_tenon_fitted");
        narrative.push_str("【巧木開榫・嚴絲合縫】你在刨木工作台前引鋸推刨，手持角尺細細鑿削沉水烏木榫卯。反覆嵌合校修之下，公母榫卯嚴絲合縫、扣合如一，製成一件精巧厚重的「沉水烏木榫卯茶盤」。木坊掌事依件計酬自公庫給付 5 鎂匠工辛勞工錢，鍛爐沉息隨之凝練。");
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.54 Pillar 2: 精鋼回火 (QuenchBlade)
    if (obj.id == "wt_forge_quench_trough"
        || obj.id == "wt_forge_furnace_anvil"
        || obj.name.contains("精鋼淬火槽")
        || obj.name.contains("耐火鍛造台"))
        && matches!(action, "QuenchBlade" | "精鋼回火")
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
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

            let repair_note = if let Some((v_id, v_name)) = vessel_target {
                let st = crate::pg::living_world::temper_vessel_at_forge(pid, &v_id, now);
                format!("隨身兵刃「{}」受爐火回火與淬火油滋養，器況重歸「{}」（人器磨合：{} —— {}）！", v_name, st.condition.label(), st.familiarity().label(), st.familiarity().feel_prose())
            } else {
                "隨身鐵料在槽中冷淬定型、消弭內應力，器物鋼性堅韌大增。".to_string()
            };

            let _ = db::add_to_inventory(pid, "item_quenched_steel_whetstone", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 10).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::ForgeHeavy,
                3,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "blade_quenched_tempered");
            narrative.push_str(&format!(
                "【精鋼回火・寒芒重煥】你交付 3 鎂工錢將熾紅鐵坯探入淬火油槽，白霧激盪翻騰、嗤啦大響，精鋼重煥寒芒、寒光奪目！{}\n你收得一枚「雙面精鋼油石」妥存於囊，鍛爐沉息隨之凝練。",
                repair_note
            ));
        } else {
            narrative.push_str("【資財不足】進行淬火回火需 3 鎂用料工錢存入工坊公庫以備淬火油與鍛爐精炭。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.54 Pillar 2: 藍染浸布 (DyeIndigoCloth)
    if (obj.id == "wt_cloth_garment_rack"
        || obj.id == "wt_obj_loom_dye_vats"
        || obj.name.contains("防雨長衫架")
        || obj.name.contains("踏板高織機"))
        && action == "DyeIndigoCloth"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 2 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -2);
            let _ = db::add_to_inventory(pid, "item_indigo_rain_cloak", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 12).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                2,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "indigo_cloth_dyed");
            narrative.push_str("【山藍浸染・桐油防雨】你向織造公庫交付 2 鎂料資，取過山藍發酵染液，將新織厚麻布反覆浸缸搥打、透風發色出深邃沉靜的靛青色澤，復刷上一層薄透熟桐油防雨。你收妥一件結實輕便的「山藍防雨斗篷」，海潮綿息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】浸染防雨布匹需 2 鎂用料資財存入公庫以備山藍染料與熟桐油。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.54 Pillar 3: 飼喂草獸 (FeedDraftBeast)
    if (obj.id == "wt_caravan_water_trough"
        || obj.id == "wt_obj_caravan_hitch_trough"
        || obj.name.contains("青石飲馬槽")
        || obj.name.contains("拴馬長木欄"))
        && action == "FeedDraftBeast"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 10).min(100);
                e.qi = (e.qi + 8).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ForestLong,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "draft_beast_fed");

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
                "同行車夫".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "協力飼獸", now);
            let _ = db::adjust_favorability(pid, &npc.id, 4);
            narrative.push_str(&format!(
                "【車馬添草・飲馬活泉】你抱來金黃清香的乾燕麥草料添滿木欄食槽，拔開活水木栓注滿青石槽。奔馳長途的原牛與挽馬低頭歡暢嚼食、痛飲清泉，發出滿足舒坦的響鼻。身旁的{}拱手致謝，驛站照看依例自公庫給付 4 鎂草料津貼，山林長息隨之凝練。",
                npc_label
            ));
        } else {
            narrative.push_str("【車馬添草・飲馬活泉】你抱來金黃清香的乾燕麥草料添滿木欄食槽，拔開活水木栓注滿青石槽。奔馳長途的原牛與挽馬低頭歡暢嚼食、痛飲清泉，發出滿足舒坦的響鼻。驛站照看依例自公庫給付 4 鎂草料津貼，山林長息隨之凝練。");
        }
        push_refresh(conn, pid);
        return true;
    }

    // v0.38.54 Pillar 3: 登船遠航備用 (BoardCoastalShip on wt_dock_cargo_gangplank / 雲杉卸貨板)
    if (obj.id == "wt_dock_cargo_gangplank" || obj.name.contains("雲杉卸貨板"))
        && action == "BoardCoastalShip"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 4 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -4);
            let _ = db::add_to_inventory(pid, "item_coastal_sea_chart", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 14).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                3,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "coastal_ship_boarded_gangplank");
            narrative.push_str("【順流巡航・黑潮海圖】你向公庫交付 4 鎂船資，順著厚實雲杉卸貨板穩步登上海船甲板。商船解纜順潮遠航，海風獵獵、白浪滔滔，你於航行途中參核黑潮走勢，收下一份「黑潮順時針海圖摺頁」，海潮綿息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】登船出航需交付 4 鎂船資存入公庫。");
        }
        push_refresh(conn, pid);
        return true;
    }

    false
}
