fn apply_catalog_effects_part2(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now: i64,
    narrative: &mut String,
) -> bool {
    // 寒水玉保鮮石櫃 (wt_home_jade_cabinet)
    if (obj.id == "wt_home_jade_cabinet"
        || obj.name.contains("冰鑑櫃")
        || obj.name.contains("寒水玉保鮮石櫃")
        || obj.name.contains("寒水玉雙層保鮮石櫃"))
        && (action == "Look" || action == "Open" || action == "StoreStash" || action == "RetrieveStash" || action == "Use" || action == "Operate")
    {
        if action == "StoreStash" {
            let ent_opt = db::get_entity(pid).ok().flatten();
            let storable = ent_opt.as_ref().and_then(|ch| {
                let inv = db::get_inventory(&ch.inventory, ch.vit);
                inv.items.into_iter().find(|it| {
                    it.qty >= 1
                        && it.item_id != "wt_red_seal_property_deed"
                        && it.item_id != "wt_cat_0793"
                        && it.item_id != "wt_brass_pin_tumbler_key"
                        && it.item_id != "key_brass_home_lock"
                        && !it.item_id.contains("deed")
                        && !it.item_id.contains("key")
                })
            });

            if let Some(item) = storable {
                let item_id = item.item_id;
                let item_name = if !item.name.is_empty() { item.name } else { item_id.clone() };
                let _ = db::remove_from_inventory(pid, &item_id, 1);
                match crate::npc::causal_core::store_item_in_home_stash(pid, &item_id, &item_name) {
                    Ok(total_stored) => {
                        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "home_stash_store");
                        narrative.push_str(&format!(
                            "\n【存入石櫃】你將隨身攜帶的一份「{}」妥善安放於寒水玉雙層保鮮石櫃的冰涼玉板上。天然冷冽玉氣瞬間凝結鎖住鮮味，目前石櫃內共封存保管 {} 件物資。",
                            item_name, total_stored
                        ));
                    }
                    Err(_) => {
                        let _ = db::add_to_inventory(pid, &item_id, 1);
                        narrative.push_str("\n【石櫃已滿】石櫃層架已擺滿物資（上限 24 件），無法再放入更多物資。");
                    }
                }
            } else {
                narrative.push_str("\n【行囊空空】隨身行囊中沒有可供封存的物資（貴重屋契與鑰匙不予存入）。");
            }
        } else if action == "RetrieveStash" {
            match crate::npc::causal_core::retrieve_item_from_home_stash(pid) {
                Some((item_id, item_name, rem)) => {
                    let _ = db::add_to_inventory(pid, &item_id, 1);
                    let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "home_stash_retrieve");
                    narrative.push_str(&format!(
                        "\n【取出物資】你輕輕拉開寒水玉石櫃的冷玉匣，取出保鮮完好的「{}」妥善收入隨身行囊，石櫃內尚存 {} 件物資。",
                        item_name, rem
                    ));
                }
                None => {
                    narrative.push_str("\n【石櫃空無一物】寒水玉石櫃的玉石架上空空如也，散發著天然清冷的玉石幽香，目前尚未封存任何物資。");
                }
            }
        } else {
            // Look / Open / Use / Operate
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 6).min(100);
                    e.qi = (e.qi + 6).min(100);
                });
            }
            let _ = event::append(now, pid, types::PLAYER_BODY, "home_jade_cabinet_refreshed");
            let items = crate::npc::causal_core::list_player_home_stash(pid);
            if items.is_empty() {
                narrative.push_str("\n【寒水玉雙層保鮮石櫃】北山寒水玉薄板整潔清爽，縫隙間透出天然冷冽的幽涼清氣，層架平整空曠，隨時可供封存保鮮鮮食、獵獲與各類物資。\n沁涼玉氣拂面，你頓覺神清氣爽，氣血與內息漸趨調和。");
            } else {
                let item_names: Vec<String> = items.iter().map(|(_id, name)| format!("「{}」", name)).collect();
                narrative.push_str(&format!(
                    "\n【寒水玉雙層保鮮石櫃・封存清冊】櫃內玉石架上冷氣森森，目前共保鮮儲放 {} 件物資：\n{}\n清涼玉氣沁人心脾，周身舒爽通透。",
                    items.len(),
                    item_names.join("、")
                ));
            }
        }
        push_refresh(conn, pid);
        return true;
    }

    // 松炭溫風烘衣櫥
    if (obj.id == "wt_home_charcoal_wardrobe"
        || obj.name.contains("松炭溫風烘衣櫥"))
        && (action == "Use" || action == "Operate" || action == "WarmHands")
    {
        crate::pg::living_world::clear_entity_afflictions(pid, true, now);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
                e.dex = (e.dex + 5).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|{}", obj.id, action));
        narrative.push_str("\n【松炭烘暖・驅寒除濕】你拉開松香木大立櫥，底部無煙松炭與導能溫石散發著溫純乾燥的暖風。你將衣物與隨身裝備置入烘暖，周身潮氣與荒野寒熱沉痾盡數拔除消散，筋骨輕靈舒泰、精神充實！");
        push_refresh(conn, pid);
        return true;
    }

    // 當季田壟作物
    if obj.id == "wt_field_crop_plot"
        || obj.name.contains("田壟作物")
        || obj.name.contains("當季田壟")
    {
        let ent = db::get_entity(pid).ok().flatten();
        let (gx, gy) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0));
        let cur_terrain = crate::server::grid_manager::get_runtime_grid_cell(crate::grid::SquareCoord::new(gx, gy))
            .map(|c| c.terrain);

        let is_empty_furrow = cur_terrain == Some(crate::grid::Terrain::EmptyFurrow);
        let field_kind = match cur_terrain {
            Some(crate::grid::Terrain::DryField) => "dry_field",
            Some(crate::grid::Terrain::HerbGarden) => "herb_garden",
            _ => "water_field",
        };

        if is_empty_furrow {
            if action == "Use" || action == "Operate" || action == "Sow" {
                crate::server::grid_manager::with_square_grid_mut(|grid| {
                    if let Some(cell) = grid.get_mut(crate::grid::SquareCoord::new(gx, gy)) {
                        cell.terrain = crate::grid::Terrain::FarmField;
                        cell.name = "水田苗畦".to_string();
                    }
                });
                let plot = crate::npc::causal_core::register_crop_plot_sown(gx, gy, "water_field", now);
                let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|sow", obj.id));
                *narrative = format!(
                    "【整壟播種】你沿著翻鬆的壟溝引水浸潤，均勻撒下當季種籽（{}）。無需反覆枯燥勞作，泥土與清泉已自發孕育生機，靜待歲時抽穗熟成。",
                    plot.crop_name
                );
                push_refresh(conn, pid);
                super::super::handler::send_place_view(conn, pid);
                return true;
            } else if action == "Harvest" || action == "Gather" {
                *narrative = "【休耕空壟】此處田壟尚未播種作物，請先點選「使用／播種」撒下當季種籽。".to_string();
                push_refresh(conn, pid);
                return true;
            } else if action == "Look" {
                *narrative = "【休耕空壟】這是一壟已起好壟溝但尚未播種的鬆軟田壟，土質肥沃深厚、引水便利，隨時可進行播種。".to_string();
                push_refresh(conn, pid);
                return true;
            }
        } else {
            if action == "Look" {
                let plot = crate::npc::causal_core::inspect_or_advance_crop_plot(gx, gy, field_kind, now);
                let stage_desc = match plot.stage {
                    crate::npc::causal_core::CropGrowthStage::Seedling => "幼苗青翠，壟間嫩綠新芽在晨露中舒展，根系深紮土壤汲取養分。",
                    crate::npc::causal_core::CropGrowthStage::Growing => "抽穗飽滿，禾穗迎風搖曳，沉甸甸的籽實已初具成色，靜待最後成熟。",
                    crate::npc::causal_core::CropGrowthStage::Mature => "熟成待收！金黃飽滿的沉重穗實低垂在壟上，稻香撲鼻，正適及時採收！",
                };
                *narrative = format!(
                    "【田壟生息・{}】當前栽種作物：{}。\n生長氣候：{}\n當前階段：{}",
                    match field_kind {
                        "dry_field" => "旱田壟畦",
                        "herb_garden" => "藥圃苗畦",
                        _ => "水田壟畦",
                    },
                    plot.crop_name,
                    if plot.water_nourished { "清泉潤養充足" } else { "日照溫煦" },
                    stage_desc
                );
                let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|{}", obj.id, action));
                push_refresh(conn, pid);
                return true;
            }

            if action == "Harvest" || action == "Gather" || action == "Use" || action == "Operate" {
                match crate::npc::causal_core::harvest_crop_plot_once(gx, gy, field_kind, now) {
                    Ok((item_id, item_name, qty, harvest_prose)) => {
                        let _ = db::add_to_inventory(pid, &item_id, qty.max(1));
                        crate::server::grid_manager::with_square_grid_mut(|grid| {
                            if let Some(cell) = grid.get_mut(crate::grid::SquareCoord::new(gx, gy)) {
                                cell.terrain = crate::grid::Terrain::EmptyFurrow;
                                cell.name = "休耕空壟".to_string();
                            }
                        });
                        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|harvest|{}", obj.id, item_id));
                        *narrative = format!(
                            "【順時收穫】{}\n你將採收得來的 {} 份「{}」整齊收納入隨身行囊，壟溝復歸清爽整潔的休耕狀態。",
                            harvest_prose, qty, item_name
                        );
                        push_refresh(conn, pid);
                        super::super::handler::send_place_view(conn, pid);
                        return true;
                    }
                    Err(wait_prose) => {
                        *narrative = format!("【尚未熟成】{}", wait_prose);
                        push_refresh(conn, pid);
                        return true;
                    }
                }
            }
        }
    }
    // 2.5) Wenyao Special World Objects
    if (obj.id == "wt_bridge_sundial_pillar" || obj.name.contains("日晷") || obj.name.contains("指路石柱"))
        && (action == "Read" || action == "Use")
    {
        let hour = current_game_hour(&conn.cfg);
        let (period, shadow_desc) = match hour {
            23 | 0..=3 => ("深宵", "此時夜闌更深，日晷無日影，青石底座刻痕藉鐘樓燈火依稀可辨"),
            4..=6 => ("晨曉", "旭日初昇，晨霞薄染，細長晷影斜斜投向正西方「西門聯運」刻度"),
            7..=10 => ("晝前", "朝陽高照，長街人聲初起，晷影清晰轉向西北方「三水總商行」刻度"),
            11..=14 => ("正晝", "日輪當空正中，晷針短影端正直指正北「三水口北街」刻度"),
            15..=18 => ("黃昏", "午後陽光漸暖，車馬絡繹，晷影斜長投向東南「東橋旅社與分家巷」刻度"),
            _ => ("入夜", "暮色四合，燈火初上，晷影隨殘陽斂入石台，正對夜市人流"),
        };
        narrative.push_str(&format!(
            "\n【日晷報時】當前半島時間為【{} {:02}:00 左右】。{}。\n石柱四面銘文刻印分明：北指「三水口北街、三水總商行與大食堂」、東指「講堂街文曜學院與東橋旅社」、南指「三水口南街、公議堂與分家巷百工」、西指「傳訊鐘樓與西門聯運總站」。",
            period, hour, shadow_desc
        ));
        push_refresh(conn, pid);
        return true;
    }

    // 《半島旬報》閱報欄 (wt_street_news_board / wt_cat_0786)
    if (obj.id == "wt_street_news_board" || obj.id == "wt_cat_0786" || obj.name.contains("閱報") || obj.name.contains("旬報"))
        && (action == "Look" || action == "Read" || action == "Use")
    {
        let room_name = resolve_room_name(player_room);
        let gazette = crate::npc::causal_core::compile_live_peninsula_gazette(&room_name, now);
        let gazette_clean = gazette.lines().map(|line| {
            if line.contains("現存") && line.contains("目標需求") && line.contains("缺口約") {
                let re_prefix = line.split("：").next().unwrap_or("");
                let price_part = line.split("，坊間公道收購價").nth(1).unwrap_or("");
                if !price_part.is_empty() {
                    format!("{}：坊間存貨緊俏，坊間公道收購價{}", re_prefix, price_part)
                } else {
                    format!("{}：坊間存貨緊俏。", re_prefix)
                }
            } else {
                line.to_string()
            }
        }).collect::<Vec<_>>().join("\n");
        narrative.push('\n');
        narrative.push_str(&gazette_clean);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "peninsula_gazette_read");
        push_refresh(conn, pid);
        return true;
    }

    // 防雨竹編信箱 (wt_home_bamboo_mailbox)
    if (obj.id == "wt_home_bamboo_mailbox" || obj.name.contains("竹編信箱"))
        && (action == "Look" || action == "Read" || action == "Open" || action == "Use" || action == "DispatchCourier")
    {
        if action == "Look" || action == "Read" {
            let room_name = resolve_room_name(player_room);
            let gazette = crate::npc::causal_core::compile_live_peninsula_gazette(&room_name, now);
            let gazette_clean = gazette.lines().map(|line| {
                if line.contains("現存") && line.contains("目標需求") && line.contains("缺口約") {
                    // 將現存 {} 份（目標需求 {} 份，缺口約 {}%）改寫為坊間存貨緊俏
                    let re_prefix = line.split("：").next().unwrap_or("");
                    let price_part = line.split("，坊間公道收購價").nth(1).unwrap_or("");
                    if !price_part.is_empty() {
                        format!("{}：坊間存貨緊俏，坊間公道收購價{}", re_prefix, price_part)
                    } else {
                        format!("{}：坊間存貨緊俏。", re_prefix)
                    }
                } else {
                    line.to_string()
                }
            }).collect::<Vec<_>>().join("\n");
            *narrative = format!("你掀開竹編信箱的防水翻蓋，取出最新一期《半島旬報》：\n{}", gazette_clean);
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mailbox_read_gazette");
        } else if action == "Open" || action == "Use" {
            let player_name = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| if !e.display_title.trim().is_empty() { e.display_title } else { e.id })
                .unwrap_or_else(|| pid.to_string());
            let (letter, gift_opt) = crate::npc::causal_core::claim_acquaintance_mailbox_letter(pid, &player_name, now);
            *narrative = format!("你撥開竹箱銅扣探手檢視，取出封泥完好的信件：\n{}", letter);
            if let Some((gift_id, gift_name)) = gift_opt {
                let _ = db::add_to_inventory(pid, &gift_id, 1);
                narrative.push_str(&format!("\n隨信附有一份心意回禮：「{}」，已妥善收整入行囊。", gift_name));
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mailbox_opened");
        } else if action == "DispatchCourier" {
            let magnesium = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.magnesium)
                .unwrap_or(0);
            if magnesium < 3 {
                narrative.push_str("\n【資財不足】託付疾行草獸信使飛騎遞信需付 3 鎂郵資，隨身資財不足，信使禮貌婉拒。");
            } else {
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
                let room_name = resolve_room_name(player_room);
                let _ = crate::game::object_ecology::record_settlement_market_trade(&room_name, "craft", 1, 3, pid);
                let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mailbox_courier_dispatched");
                narrative.push_str("\n【快騎託運・飛騎出發】你將家書與物資封妥投入信箱快信格，支付 3 鎂郵資。信使吹響木哨，健步如飛的草獸揚起一陣輕塵，循著驛道朝鄰近重鎮疾馳而去！");
            }
        }
        push_refresh(conn, pid);
        return true;
    }

    // 疾行草獸信使櫃檯 (wt_courier_post_desk)
    if (obj.id == "wt_courier_post_desk" || obj.name.contains("信使櫃檯"))
        && (action == "Look" || action == "Read" || action == "DispatchCourier" || action == "Use")
    {
        if action == "Look" || action == "Read" {
            let room_name = resolve_room_name(player_room);
            let gazette = crate::npc::causal_core::compile_live_peninsula_gazette(&room_name, now);
            let gazette_clean = gazette.lines().map(|line| {
                if line.contains("現存") && line.contains("目標需求") && line.contains("缺口約") {
                    let re_prefix = line.split("：").next().unwrap_or("");
                    let price_part = line.split("，坊間公道收購價").nth(1).unwrap_or("");
                    if !price_part.is_empty() {
                        format!("{}：坊間存貨緊俏，坊間公道收購價{}", re_prefix, price_part)
                    } else {
                        format!("{}：坊間存貨緊俏。", re_prefix)
                    }
                } else {
                    line.to_string()
                }
            }).collect::<Vec<_>>().join("\n");
            narrative.push_str(&format!(
                "\n【疾行草獸信使總站櫃檯】案頭整齊陳列著往返四鎮的驛道封冊、火漆印章與加急竹筒，案旁栓著數頭蓄勢待發的健碩草獸。壁上木牌張貼著半島各驛站聯運路況：\n{}\n草獸信使櫃檯提供跨聚落加急包裹託運服務，每件資費 4 鎂。",
                gazette_clean
            ));
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "courier_post_read");
        } else if action == "DispatchCourier" || action == "Use" {
            let magnesium = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.magnesium)
                .unwrap_or(0);
            if magnesium < 4 {
                narrative.push_str("\n【資財不足】辦理跨聚落加急件託運需付 4 鎂資費，隨身資財不足，櫃檯掌事客氣提醒。");
            } else {
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -4);
                let room_name = resolve_room_name(player_room);
                let _ = crate::game::object_ecology::record_settlement_market_trade(&room_name, "craft", 1, 4, pid);
                let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "courier_post_dispatched");
                narrative.push_str("\n【跨聚落急件託運】你向掌事交付 4 鎂託運規費，並在封皮蓋上加急硃印。當值信使將包裹紮緊繫於疾行草獸鞍袋，健碩草獸揚蹄嘶鳴，破風奔向遠方聚落！");
            }
        }
        push_refresh(conn, pid);
        return true;
    }

    // 老青竹黃銅輪捲線釣竿 (wt_coastal_angling_spot / wt_cat_1006)
    if (obj.id == "wt_coastal_angling_spot"
        || obj.id == "wt_cat_1006"
        || obj.name.contains("老青竹黃銅輪捲線釣竿")
        || obj.name.contains("釣竿")
        || obj.name.contains("垂釣"))
        && (action == "AngleFish" || action == "Use" || action == "Harvest" || action == "Operate" || action == "Look")
    {
        let room_name = resolve_room_name(player_room);
        let z_idx = crate::npc::causal_core::settlement_to_zone_idx(&room_name);
        let weather = crate::npc::causal_core::zone_weather_phase(z_idx, now);
        let tide = crate::npc::causal_core::zone_tide_phase(z_idx, now);

        let weather_prose = match weather {
            crate::game::ZoneWeatherPhase::RainMist => "煙雨迷濛，水面泛起粼粼漣漪，魚群紛紛浮頭索餌",
            crate::game::ZoneWeatherPhase::ClearBreeze => "微風拂面，水波不興，水底浮標靈敏輕晃",
            crate::game::ZoneWeatherPhase::DryWarm => "天青水暖，陽光直透清澈水底，游鱗穿梭石隙之間",
            crate::game::ZoneWeatherPhase::ColdFrost => "寒霜初凝，水汽蒸騰，沉底大物沿著溫流緩緩巡游",
        };
        let tide_prose = match tide {
            crate::game::ZoneTidePhase::FloodTide => "正值滿潮大水，海流頂托河口，水產豐腴",
            crate::game::ZoneTidePhase::EbbTide => "正值退潮水落，灘頭礁縫露出，獵食活潑",
            crate::game::ZoneTidePhase::SlackTide => "正值平潮之際，水面澄澈如鏡，靜候大物咬鉤",
        };

        if action == "Look" {
            let hr = crate::store::current_game_hour();
            let water_surface = if !(6..19).contains(&hr) {
                "夜色映水，微風拂過河面泛起細碎波光，水底游鱗偶爾掠過暗流"
            } else {
                "天光映照水面，微風拂過泛起粼粼漣漪，可見游鱗穿梭石隙之間"
            };
            narrative.push_str(&format!(
                "\n{}；{}。水面下波紋微動，浮標隨暗流輕輕起伏。",
                water_surface, tide_prose
            ));
        } else {
            let is_coast = room_name.contains("港")
                || room_name.contains("海")
                || room_name.contains("澳")
                || room_name.contains("鼻")
                || room_name.contains("尾")
                || room_name.contains("風舶")
                || room_name.contains("黑石灣")
                || room_name.contains("礁");
            let is_high_or_spring_tide = tide == crate::game::ZoneTidePhase::FloodTide;
            let (catch_id, catch_name) = if is_coast && is_high_or_spring_tide {
                ("wt_cat_0225", "礁岩大龍蝦")
            } else if is_coast {
                ("shanhai_fresh_fish", "深海鮮魚")
            } else {
                ("mat_fresh_shrimp", "水尾寮鮮沼蝦")
            };

            let _ = db::add_to_inventory(pid, catch_id, 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 5).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "coastal_angling_catch");
            narrative.push_str(&format!(
                "\n【甩竿垂釣・豐收起竿】你揚起老青竹釣竿，黃銅滾輪發出清脆的收線鳴響，釣線劃破水面在空中劃出一道銀弧！\n{}，{}。\n伴隨一陣沉甸甸的水花拍打聲，你利落抄網，將一尾活蹦亂跳的「{}」收獲入魚簍，呼吸吐納愈發沉靜悠長！",
                weather_prose, tide_prose, catch_name
            ));
        }
        push_refresh(conn, pid);
        return true;
    }

    // 紅土光學黃銅單筒望遠鏡 (wt_lookout_telescope / wt_cat_0733)
    if (obj.id == "wt_lookout_telescope"
        || obj.id == "wt_cat_0733"
        || obj.name.contains("望遠鏡")
        || obj.name.contains("單筒望遠鏡"))
        && (action == "Operate" || action == "Use" || action == "Look")
    {
        let room_name = resolve_room_name(player_room);
        let z_idx = crate::npc::causal_core::settlement_to_zone_idx(&room_name);
        let climate_evt = crate::db::get_region_climate("bai_ta_peninsula")
            .ok()
            .flatten()
            .map(|c| c.event_type)
            .unwrap_or_else(|| "clear".to_string());
        let tide = crate::npc::causal_core::zone_tide_phase(z_idx, now);
        let hr = crate::store::current_game_hour();
        let weather_label = match climate_evt.as_str() {
            "rain" | "plum_rain" | "storm" => "煙雨空濛、水氣瀰漫（遠景稍暗）",
            "snow" | "fog" => "寒霧微凝、高嶺薄靄（遠岫隱約可見）",
            "wind" => "長風颯爽、雲影疾馳（視野開闊明朗）",
            _ => {
                if !(6..19).contains(&hr) {
                    "天朗星垂、夜色澄明（兩岸燈火與遠影歷歷可見）"
                } else {
                    "天朗氣清、惠風和暢（視野開闊極佳）"
                }
            }
        };
        let tide_label = match tide {
            crate::game::ZoneTidePhase::FloodTide => "滿潮湧漲",
            crate::game::ZoneTidePhase::EbbTide => "退潮落水",
            crate::game::ZoneTidePhase::SlackTide => "平潮停滯",
        };

        let ent = db::get_entity(pid).ok().flatten();
        let (px, py) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0));

        let live_coords: std::collections::HashMap<String, ((i32, i32), String)> = {
            let states = crate::store::LIVE_NPC_GRID_STATES.read();
            states
                .iter()
                .map(|(k, v)| (k.clone(), (v.current, v.current_activity())))
                .collect()
        };

        let mut nearby_npcs: Vec<String> = Vec::new();
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            for (id, e) in &s.entities {
                if id != pid && (e.kind == "npc" || e.kind == "Npc") && e.vit > 0 {
                    let coord = live_coords
                        .get(id)
                        .map(|(c, _)| *c)
                        .or_else(|| e.grid_x.zip(e.grid_y));
                    if let Some((ex, ey)) = coord {
                        let d = (ex - px).abs().max((ey - py).abs());
                        if d <= 3 {
                            let name = if crate::store::player_knows_npc(pid, id) && !e.display_title.trim().is_empty() {
                                crate::store::resolve_npc_person_name(id, &e.display_title, None)
                            } else {
                                "行旅身影".to_string()
                            };
                            let dir_str = match ((ex - px).cmp(&0), (ey - py).cmp(&0)) {
                                (std::cmp::Ordering::Equal, std::cmp::Ordering::Greater) => "北面",
                                 (std::cmp::Ordering::Equal, std::cmp::Ordering::Less) => "南面",
                                 (std::cmp::Ordering::Greater, std::cmp::Ordering::Equal) => "東面",
                                 (std::cmp::Ordering::Less, std::cmp::Ordering::Equal) => "西面",
                                 (std::cmp::Ordering::Greater, std::cmp::Ordering::Greater) => "東北面",
                                 (std::cmp::Ordering::Greater, std::cmp::Ordering::Less) => "東南面",
                                 (std::cmp::Ordering::Less, std::cmp::Ordering::Greater) => "西北面",
                                 (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => "西南面",
                                 (std::cmp::Ordering::Equal, std::cmp::Ordering::Equal) => "近處",
                            };
                            let act_suffix = if let Some((_, act)) = live_coords.get(id) {
                                let raw_act = act.trim();
                                let visual_distant_act = if raw_act.contains("動身前往")
                                    || raw_act.contains("步行前往")
                                    || raw_act.contains("沿街步行")
                                    || raw_act.contains("趕往")
                                    || raw_act.contains("前往【")
                                    || raw_act.contains("回返【")
                                {
                                    "沿著街巷步道緩步穿行"
                                } else if raw_act.contains("歇")
                                    || raw_act.contains("喘")
                                    || raw_act.contains("睡")
                                    || raw_act.contains("寢")
                                {
                                    "在廊簷案頭旁駐足歇息"
                                } else if raw_act.contains("飲")
                                    || raw_act.contains("喝")
                                    || raw_act.contains("茶")
                                    || raw_act.contains("湯")
                                    || raw_act.contains("吃")
                                    || raw_act.contains("果腹")
                                {
                                    "在街邊攤肆與茶座間歇腳用茶"
                                } else {
                                    "在街坊鋪面間往來操持"
                                };
                                format!("，正{}", visual_distant_act)
                            } else {
                                String::new()
                            };
                            nearby_npcs.push(format!("「{}」（位於{} {} 里處{}）", name, dir_str, d, act_suffix));
                            if nearby_npcs.len() >= 3 {
                                break;
                            }
                        }
                    }
                }
            }
        }

        let time_tick = now / 180;
        let habitat_slots = crate::game::live_zone_shanhai_slots(z_idx, time_tick);
        let mut beast_reports: Vec<String> = Vec::new();
        for sl in habitat_slots.iter().take(3) {
            let dx = sl.x - px;
            let dy = sl.y - py;
            let dist = dx.abs().max(dy.abs());
            let dir_str = match (dx.cmp(&0), dy.cmp(&0)) {
                (std::cmp::Ordering::Equal, std::cmp::Ordering::Greater) => "北面",
                (std::cmp::Ordering::Equal, std::cmp::Ordering::Less) => "南面",
                (std::cmp::Ordering::Greater, std::cmp::Ordering::Equal) => "東面",
                (std::cmp::Ordering::Less, std::cmp::Ordering::Equal) => "西面",
                (std::cmp::Ordering::Greater, std::cmp::Ordering::Greater) => "東北面",
                (std::cmp::Ordering::Greater, std::cmp::Ordering::Less) => "東南面",
                (std::cmp::Ordering::Less, std::cmp::Ordering::Greater) => "西北面",
                (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => "西南面",
                (std::cmp::Ordering::Equal, std::cmp::Ordering::Equal) => "近處",
            };
            let creature_name = crate::game::ensure_ambient_npc_at_grid(sl.x, sl.y, sl.tier_tag)
                .map(|amb| amb.display_title)
                .unwrap_or_else(|| sl.tier_label.to_string());
            let clean_prefix = sl
                .act_prefix
                .trim()
                .trim_end_matches('正')
                .trim_end_matches('，')
                .trim_end_matches('、');
            let act = if !clean_prefix.is_empty() {
                format!("，{}出沒。", clean_prefix)
            } else {
                "一帶活動。".to_string()
            };
            beast_reports.push(format!(
                "【{}・{}】位於{}約 {} 里地勢「{}」{}",
                sl.tier_label, creature_name, dir_str, dist, sl.sub_geo, act
            ));
        }

        let npcs_text = if nearby_npcs.is_empty() {
            "周遭三里之內山徑空曠幽靜，未見其他行旅駐足。".to_string()
        } else {
            nearby_npcs.join("、")
        };
        let beasts_text = if beast_reports.is_empty() {
            "望遠鏡視野所及之處，生靈隱匿於林莽深谷之中，暫無醒目大型獸群現身。".to_string()
        } else {
            beast_reports.join("\n")
        };

        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "lookout_telescope_scouted");
        narrative.push_str(&format!(
            "\n【單筒望遠鏡・四方瞭望】當前物候能見度：{}（潮位：{}）\n近處行旅觀察（周遭三里）：{}\n遠方生靈蹤跡（白塔山海動向）：\n{}\n山川草木歷歷在目，極目遠眺令胸懷大暢！",
            weather_label, tide_label, npcs_text, beasts_text
        ));
        push_refresh(conn, pid);
        return true;
    }

    // 卡式導能便攜爐 (wt_field_camp_stove / wt_cat_1010)
    if (obj.id == "wt_field_camp_stove"
        || obj.id == "wt_cat_1010"
        || obj.name.contains("卡式導能便攜爐")
        || obj.name.contains("導能便攜爐")
        || obj.name.contains("便攜爐"))
        && (action == "Cook" || action == "Use" || action == "Operate" || action == "Look")
    {
        if action == "Look" {
            narrative.push_str("\n爐內導能晶匣充盈，旋鈕與防風環完備，隨時可起火溫食。");
        } else {
            let raw_candidates = [
                ("wt_cat_0225", "礁岩大龍蝦"),
                ("shanhai_fresh_fish", "深海鮮魚"),
                ("mat_fresh_shrimp", "水尾寮鮮沼蝦"),
                ("shanhai_beast_meat", "山海鮮獸肉"),
                ("mat_grain_sack", "農家精選穀糧袋"),
                ("mat_pasture_seed_sack", "優質牧草種籽袋"),
                ("harvest_herb", "野地草藥"),
            ];

            let ent = db::get_entity(pid).ok().flatten();
            let found_ingredient = ent.as_ref().and_then(|ch| {
                let inv = db::get_inventory(&ch.inventory, ch.vit);
                for (id, default_name) in raw_candidates {
                    if let Some(item) = inv.items.iter().find(|it| it.item_id == id && it.qty >= 1) {
                        let name = if !item.name.is_empty() { item.name.clone() } else { default_name.to_string() };
                        return Some((id.to_string(), name));
                    }
                }
                None
            });

            if let Some((ing_id, ing_name)) = found_ingredient {
                let _ = db::remove_from_inventory(pid, &ing_id, 1);
                crate::pg::living_world::clear_entity_afflictions(pid, true, now);
                if let Some(arc) = crate::store::get_store() {
                    let _ = arc.write().update_entity(pid, |e| {
                        e.vit = (e.vit + 20).min(100);
                        e.qi = (e.qi + 20).min(100);
                    });
                }
                let _ = event::append(now, pid, types::PLAYER_BODY, "camp_stove_cooked_meal");
                narrative.push_str(&format!(
                    "\n【野炊烹煮・熱羹出鍋】你旋開卡式導能便攜爐的銅旋鈕，微晶導能爐盤噴吐出純青勻淨的火焰。你將行囊中的「{}」下鍋，搭配山澗清泉慢火煨煮。不多時濃郁熱氣升騰，一鍋鮮香撲鼻的野地熱羹熱氣騰騰！\n你趁熱細細啜飲享用，通體舒暢、寒濕盡拔，周身氣血與內息復歸飽滿。",
                    ing_name
                ));
            } else {
                crate::pg::living_world::clear_entity_afflictions(pid, true, now);
                if let Some(arc) = crate::store::get_store() {
                    let _ = arc.write().update_entity(pid, |e| {
                        e.vit = (e.vit + 10).min(100);
                        e.qi = (e.qi + 12).min(100);
                    });
                }
                let _ = event::append(now, pid, types::PLAYER_BODY, "camp_stove_boiled_water");
                narrative.push_str("\n【溫水暖胃・行氣調息】你旋開導能便攜爐點燃微晶青焰，架上小銅壺汲取清泉煮沸。你捧著熱水烘暖雙掌並細細啜飲，融融暖意順著咽喉化入經絡，驅散了周遭風露寒涼，身心沉痾頓時舒解。");
            }
        }
        push_refresh(conn, pid);
        return true;
    }

    // ==========================================
    // v0.38.52 Three-Pillar Living-World Expansion
    // ==========================================

    // Pillar 1: Outer 13 Settlements' Signature Workshops & Pit #3 Rail Trolley
    // 1. 老堰頭水磨 (wt_mill_*)
    if (obj.id.starts_with("wt_mill_")
        || obj.name.contains("水輪玄武磨")
        || obj.name.contains("篩粉斗")
        || obj.name.contains("儲糧斗"))
        && (action == "MillGrain" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "wt_cat_0388", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 5);
        crate::npc::causal_core::record_settlement_market_production("水尾寮", "wt_cat_0388", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_mill_grain");
        narrative.push_str("【老堰頭水磨碾米】你合力推開水磨料門，巨輪轉動帶動兩合巨大玄武岩磨盤隆隆運轉，顆粒飽滿的稻穀化為細密熱漿。工坊按工計酬結算勞務，由聚落公庫支領 5 鎂酬勞，並盛了一碗現磨的「老堰頭現磨溫米漿」交予你作為手作所得。市集糧棧的新米漿儲備隨之充盈！");
        push_refresh(conn, pid);
        return true;
    }

    // 2. 霜林口鋸木排鋸 (wt_sawmill_*)
    if (obj.id.starts_with("wt_sawmill_")
        || obj.name.contains("水動排鋸架")
        || obj.name.contains("原木檢寸台"))
        && (action == "RefinePlank" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "mat_plank_board", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 6);
        crate::npc::causal_core::record_settlement_market_production("霜林口", "mat_plank_board", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_sawmill_plank");
        narrative.push_str("【霜林口裁鋸木板】你推動水動排鋸咬合百年雲杉原木，精鋼鋸齒上下飛速下切，鋸路筆直順暢，木屑紛飛間剖出規整厚板。工坊由霜林口公庫支領 6 鎂工酬，並將裁切精良的「樟松木板料」交付於你。工坊向周遭商行補充了建材木料！");
        push_refresh(conn, pid);
        return true;
    }

    // 3. 半嶺切石金剛砂拉鋸 (wt_stonecut_*)
    if (obj.id.starts_with("wt_stonecut_")
        || obj.name.contains("金剛砂拉鋸")
        || obj.name.contains("寒水玉料架")
        || obj.name.contains("拋光琢石盆"))
        && (action == "CutJade" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "wt_cat_0002", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 6);
        crate::npc::causal_core::record_settlement_market_production("半嶺鎮", "wt_cat_0002", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_cut_jade");
        narrative.push_str("【半嶺切磨寒玉】你拉動金剛砂合金絲拉鋸緊貼原石，引冷泉注淋鋸槽，細細剖開北山寒水玉毛料，石皮下透出溫潤玉光，經琢石盆細心修整稜邊。工坊由半嶺鎮公庫支領 6 鎂酬勞，並贈予一枚質地縝密的「寒水玉原石」。");
        push_refresh(conn, pid);
        return true;
    }

    // 4. 牛埔壓酪架與牧耕鐵犁 (wt_cheese_* / wt_wool_* / wt_farm_tool_iron_plow)
    if (obj.id.starts_with("wt_cheese_")
        || obj.id == "wt_wool_spinning_wheel"
        || obj.id == "wt_farm_tool_iron_plow"
        || obj.name.contains("螺旋壓酪架")
        || obj.name.contains("手搖木紡車")
        || obj.name.contains("乳酪熟成架")
        || obj.name.contains("雙齒精鐵犁"))
        && (action == "PressCheese" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "wt_cat_0300", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 5);
        crate::npc::causal_core::record_settlement_market_production("牛埔", "wt_cat_0300", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_press_cheese");
        narrative.push_str("【牛埔壓製乳酪】你握住螺旋桿加力旋轉下壓，粗濾布中的凝乳塊瀝出澄澈微酸金黃乳清，壓製成緊實濃醇的硬乳酪磚。牧場按勞作由牛埔公庫撥付 5 鎂勞酬，並取出熟成成色極佳的一輪「窖藏熟成硬乳酪」贈予你。");
        push_refresh(conn, pid);
        return true;
    }

    // 5. 三號坑口選礦槽與雙軸滑車 (wt_mine_* / wt_trolley_*)
    if (obj.id.starts_with("wt_mine_")
        || obj.id.starts_with("wt_trolley_")
        || obj.name.contains("木製洗礦槽")
        || obj.name.contains("分級重鐵篩")
        || obj.name.contains("雙軸重台車")
        || obj.name.contains("鋼索絞盤"))
        && (action == "HaulMineCart" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "wt_cat_0001", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 7);
        crate::npc::causal_core::record_settlement_market_production("三號坑口", "wt_cat_0001", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_haul_mine_cart");
        narrative.push_str("【三號坑口滑車運礦】你合力推動雙軸重台車滑入坑口鐵軌，伴隨鋼輪咬軌轟鳴與絞盤鋼索轉動，將滿車洗選完成的沉甸精礦運出坑道。礦務由三號坑口公庫支領 7 鎂運礦重資，並分得一塊高品位「赤鐵重礦」。");
        push_refresh(conn, pid);
        return true;
    }

    // 6. 風櫃尾慢燻紅陶灶 (wt_smokehouse_*)
    if (obj.id.starts_with("wt_smokehouse_")
        || obj.name.contains("慢燻紅陶灶")
        || obj.name.contains("冷燻海魚架"))
        && (action == "SmokeFish" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "mat_salted_fish", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 6);
        crate::npc::causal_core::record_settlement_market_production("風櫃尾", "mat_salted_fish", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_smoke_fish");
        narrative.push_str("【風櫃尾慢火冷燻】你添入受控乾燥柏木與香草，青煙繚繞慢火透骨，將醃好的海魚燻至金紅油亮、緊實鹹香。燻坊依勞作由風櫃尾公庫支領 6 鎂工錢，並收穫一份風味醇厚的「海鹽醃魚乾」。市集魚貨儲備隨之提升！");
        push_refresh(conn, pid);
        return true;
    }

    // 7. 風舶角燃灰窯與海藻搓繩 (wt_lime_* / wt_kelp_* / wt_shipyard_dry_dock)
    if (obj.id.starts_with("wt_lime_")
        || obj.id.starts_with("wt_kelp_")
        || obj.id == "wt_shipyard_dry_dock"
        || obj.name.contains("海貝燃灰窯")
        || obj.name.contains("高架曬藻棚")
        || obj.name.contains("海纜搓繩輪")
        || obj.name.contains("雲杉龍骨架"))
        && (action == "BurnLime" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "mat_hemp_rope_roll", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 6);
        crate::npc::causal_core::record_settlement_market_production("風舶角", "mat_hemp_rope_roll", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_burn_lime");
        narrative.push_str("【風舶角入窯煅灰】你鼓風引燃灰窯高溫煅燒海貝，調製防腐熟石灰並協同轉動絞輪搓絞耐磨纜繩，加固船架榫卯。港坊依勞務由風舶角公庫發放 6 鎂工酬，並獲取一捆結實耐磨的「麻繩卷」。船塢物料得以充實！");
        push_refresh(conn, pid);
        return true;
    }

    false
}
