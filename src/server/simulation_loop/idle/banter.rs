fn resolve_caller_name(player_id: &str, npc_id: &str, ent: &crate::store::Entity) -> String {
    let named = crate::db::get_acquaintance(player_id, npc_id)
        .ok()
        .flatten()
        .map(|a| a.named)
        .unwrap_or(false);
    if named {
        let t = ent.display_title.trim();
        if let Some((_, n)) = t.split_once('｜').or_else(|| t.split_once('|')) {
            n.trim().to_string()
        } else if !t.is_empty() {
            t.to_string()
        } else {
            npc_id.to_string()
        }
    } else {
        let room_name = if let (Some(gx), Some(gy)) = (ent.grid_x, ent.grid_y) {
            crate::server::grid_manager::get_square_grid()
                .and_then(|g| g.get(crate::grid::SquareCoord::new(gx, gy)).map(|c| c.name.clone()))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let (is_staff, has_uniform, in_counter_zone) = crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            &room_name,
            npc_id,
            &ent.display_title,
            &ent.current_activity,
            &ent.equipment_slots,
        );
        let venue_opt = crate::grid::resolve_any_room_venue_object(&room_name, &ent.display_title)
            .filter(|v| (room_name.is_empty() || v.venue_kind != "街巷橋樑公共空地") && v.default_host_title != "街坊居民");
        if is_staff && (venue_opt.is_some() || room_name.is_empty()) {
            let host_title = venue_opt.map(|v| v.default_host_title).unwrap_or("店員");
            if in_counter_zone && has_uniform {
                return format!("身著工作制服、站在櫃檯內側的{host_title}");
            } else if in_counter_zone {
                return format!("站在櫃檯內側的{host_title}");
            } else if has_uniform {
                return format!("身著工作制服的{host_title}");
            }
        }
        let tag = crate::store::npc_brief_appearance_tag(npc_id, &ent.display_title);
        let role = if ent.display_title.contains("鐵") || ent.display_title.contains("鍛造") || ent.display_title.contains("木作") || ent.display_title.contains("工坊") {
            "匠人"
        } else if ent.display_title.contains("商") || ent.display_title.contains("鋪") || ent.display_title.contains("市集") || ent.display_title.contains("行") {
            "行商"
        } else if ent.display_title.contains("醫") || ent.display_title.contains("藥") || ent.display_title.contains("診") {
            "醫工"
        } else if ent.display_title.contains("書") || ent.display_title.contains("學") || ent.display_title.contains("講堂") || ent.display_title.contains("觀星") {
            "學者"
        } else {
            "街坊"
        };
        if !tag.is_empty() {
            format!("{tag}的{role}")
        } else {
            format!("一名{role}")
        }
    }
}

fn generate_vendor_banter(caller_a: &str, caller_b: &str, id_a: &str, id_b: &str, hour: i32) -> String {
    let specialty_a = crate::server::action::trade::resolve_vendor_specialty_with_title(id_a, caller_a);
    let specialty_b = crate::server::action::trade::resolve_vendor_specialty_with_title(id_b, caller_b);

    let resolve_offer_details = |spec: Option<(&'static str, &'static str, i32, &'static str)>| {
        spec.map(|(item_id, item_name, base_price, _)| {
            let quote = crate::game::object_ecology::dynamic_settlement_market_quote("文曜", item_id, base_price);
            let cat_obj = crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS
                .iter()
                .find(|o| o.id == item_id);
            let look_clause = cat_obj
                .map(|o| {
                    let raw = o.look_text.trim();
                    let after_quote = if let Some(stripped) = raw.strip_prefix('「') {
                        if let Some((_, rest)) = stripped.split_once('」') {
                            rest.trim()
                        } else {
                            raw
                        }
                    } else {
                        raw
                    };
                    after_quote
                        .split(['，', '。', '；'])
                        .find(|s| !s.trim().is_empty())
                        .unwrap_or("真材實料")
                        .trim()
                })
                .unwrap_or("真材實料");
            let action_clause = cat_obj
                .map(|o| {
                    o.action_text
                        .trim()
                        .split(['，', '。', '；'])
                        .find(|s| !s.trim().is_empty())
                        .unwrap_or("滋味道地")
                        .trim()
                })
                .unwrap_or("滋味道地");
            (item_name, quote.unit_price, look_clause, action_clause)
        })
    };

    let details_a = resolve_offer_details(specialty_a);
    let details_b = resolve_offer_details(specialty_b);

    if let (Some((name_a, price_a, look_a, _)), Some((name_b, price_b, look_b, _))) = (details_a, details_b) {
        format!(
            "{caller_a}抽空朝隔壁攤招呼道：「鄰攤掌事，你這【{name_b}】（每份約{price_b}鎂）{look_b}，待會替我留一份！」 {caller_b}笑著應道：「好說！你那【{name_a}】（每份約{price_a}鎂）也是{look_a}，等這陣忙完，我們換著嚐嚐！」"
        )
    } else if let Some((name_a, price_a, _, act_a)) = details_a {
        if hour >= 18 || hour < 6 {
            format!(
                "{caller_a}望向隔壁攤笑道：「今晚市面熱鬧，我這招牌【{name_a}】（每份約{price_a}鎂）備料快見底啦，{act_a}正好趁熱！」 {caller_b}點頭應道：「夜風涼爽，大夥手腳麻利些，生意正旺！」"
            )
        } else {
            format!(
                "{caller_a}望向隔壁攤笑道：「今日日頭好，這招牌【{name_a}】（每份約{price_a}鎂）{act_a}，往來行旅不斷！」 {caller_b}點頭應道：「是啊，市面興旺，大家手頭活計都俐落！」"
            )
        }
    } else if let Some((name_b, price_b, _, act_b)) = details_b {
        if hour >= 18 || hour < 6 {
            format!(
                "{caller_a}朝隔壁攤打趣道：「夜市人潮真盛，你那【{name_b}】（每份約{price_b}鎂）{act_b}，聞著就叫人嘴饞！」 {caller_b}爽朗笑應：「好說好說！趁著夜色街燈亮堂，大夥一塊把市面做旺！」"
            )
        } else {
            format!(
                "{caller_a}朝隔壁攤招呼道：「市集買賣興旺，你攤上這【{name_b}】（每份約{price_b}鎂）{act_b}，引來不少行客！」 {caller_b}點頭笑道：「鄰攤掌事客氣了，街坊鄰里互相關照，市面才熱鬧！」"
            )
        }
    } else if hour >= 18 || hour < 6 {
        format!("{caller_a}望向隔壁攤笑道：「今晚夜市人潮真旺，手頭各色活計都快忙不過來了！」 {caller_b}點頭應道：「是啊，趁著夜風涼爽、街燈亮堂，大夥手腳都俐落些！」")
    } else {
        format!("{caller_a}望向隔壁攤笑道：「今天生意不錯呀，街上往來買賣的人潮挺旺！」 {caller_b}點頭應道：「是啊，趁著日頭正好、市面熱鬧，大夥都精神！」")
    }
}
fn sanitize_thought_string(raw: &str, role_hint: &str, seed: u64) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }

    if t == concat!("跟人說了會", "兒話") || t.starts_with(concat!("跟人說了會", "兒話")) {
        let varied = [
            "與街坊聊起近日市集物價與水路貨況，心裡有了底",
            "同熟識的鄰里寒暄幾句家常，神色舒緩",
            "同過路的行商打聽各處商道物料行情，頗有收穫",
            "與街邊鄰里互道安好，說了些坊間近況，心緒平靜",
        ];
        return varied[(seed as usize) % varied.len()].to_string();
    }

    if t.contains("並結清帳目，手頭進帳") || t.contains("結清帳目") || t.contains("手頭進帳") {
        let venue = if let Some(first_part) = t.split("完成「").next() {
            let v = first_part.trim_start_matches("在").trim();
            if v.is_empty() { "坊間" } else { v }
        } else {
            "坊間"
        };

        if t.contains("浴") || t.contains("沐浴") {
            return format!("在{}泡過溫水洗去一身疲乏，筋骨格外舒坦", venue);
        } else if t.contains("食堂") || t.contains("茶") || t.contains("納涼") || t.contains("用膳") || t.contains("麵") || t.contains("飯") {
            return format!("在{}歇過腳、喝過熱茶，精神舒爽不少", venue);
        } else if t.contains("診") || t.contains("藥") || t.contains("調理") {
            return format!("在{}請醫工切脈調理氣息，身上輕鬆了許多", venue);
        } else if t.contains("歇") || t.contains("宿") || t.contains("客寮") || t.contains("旅社") {
            return format!("在{}歇息養神，精神健旺不少", venue);
        } else if t.contains("散步") || t.contains("閒談") || t.contains("寒暄") {
            return "同熟識的鄰里寒暄幾句家常，神色舒緩".to_string();
        } else {
            let is_forager = role_hint.contains("獵")
                || role_hint.contains("入山百工")
                || role_hint.contains("採")
                || role_hint.contains("樵")
                || role_hint.contains("礦工")
                || role_hint.contains("行旅");
            let manufactured = ["麻繩卷", "塗焦麻繩", "補綴工靴", "粗鹽包", "木炭袋", "鐵料盒"]
                .iter()
                .any(|m| t.contains(m));
            if !is_forager && manufactured {
                return "在市集核對物資採辦，把各色日常用度置辦妥當，心裡踏實".to_string();
            }
        }
    }

    if t.contains("記掛著「") || t.contains("記掛著『") {
        let inside = if let Some(p) = t.split("記掛著「").nth(1).and_then(|p| p.split("」").next()) {
            Some(p)
        } else if let Some(p) = t.split("記掛著『").nth(1).and_then(|p| p.split("』").next()) {
            Some(p)
        } else {
            None
        };
        if let Some(inner) = inside {
            if inner.contains("內室榻上熄燈安歇") || inner.contains("熄燈安歇") {
                if role_hint.contains("水輪") || role_hint.contains("石磨") {
                    return "檢查完老堰頭引水石堰插板，伴著窗外水輪規律的水聲安穩養神".to_string();
                }
                return "夜深熄燈安歇，養足精神以備明日早工".to_string();
            }
            if inner.contains("伴著水輪轉動的規律水聲歇息") {
                return "檢查引水石堰的水門插板，伴著水輪轉動的規律水聲安穩歇息".to_string();
            }
            if inner.chars().count() > 28 {
                let concise = if inner.contains("石磨") || inner.contains("水輪") || inner.contains("碾") || inner.contains("米漿") {
                    "水輪石磨碾米與現磨米漿工序"
                } else if inner.contains("引水入渠") || inner.contains("水門") || inner.contains("堰渠") {
                    "堰渠水門調度與明日引水時刻"
                } else if inner.contains("家常") || inner.contains("家務") || inner.contains("居") || inner.contains("宅") {
                    "家中雜務"
                } else if inner.contains("採辦") || inner.contains("採買") || inner.contains("市集") {
                    "市集採辦"
                } else if inner.contains("工坊") || inner.contains("活計") || inner.contains("打磨") || inner.contains("鐵") || inner.contains("木") {
                    "手頭活計"
                } else if inner.contains("診所") || inner.contains("抓藥") || inner.contains("切脈") {
                    "診所抓藥"
                } else {
                    "手頭差事"
                };
                return format!("心裡記掛著『{}』，正按部就班料理妥當", concise);
            }
        }
    }
    if (role_hint.contains("水輪") || role_hint.contains("石磨"))
        && (t.contains("記掛著『診所抓藥』") || t.contains("記掛著『手頭差事』"))
    {
        return "心裡記掛著『水輪石磨碾米與現磨米漿工序』，正按部就班料理妥當".to_string();
    }

    t.to_string()
}

pub(super) fn reconcile_simulation_errands_and_thoughts(_hour: i32) {
    let Some(st_arc) = crate::store::get_store() else {
        return;
    };

    // 1. Audit and correct live NPC grid states (snapshot titles first so STORE and LIVE_NPC_GRID_STATES never invert lock order)
    let npc_titles: std::collections::HashMap<String, String> = {
        let st = st_arc.read();
        st.entities
            .iter()
            .filter(|(_, e)| e.kind == "npc")
            .map(|(k, e)| (k.clone(), e.display_title.clone()))
            .collect()
    };
    {
        let mut live_states = crate::store::LIVE_NPC_GRID_STATES.write();

        for (npc_id, state) in live_states.iter_mut() {
            let Some(display_title) = npc_titles.get(npc_id) else {
                continue;
            };
            let slot = crate::store::wenyao_household_slot_for(npc_id);
            let role_full = format!(
                "{} {} {} {} {} {}",
                display_title,
                slot.brief_tag,
                slot.home_name,
                slot.errand_name,
                state.dest_name,
                state.purpose
            );
            let is_forager = [
                "獵", "入山百工", "礦脈百工", "林場百工", "古道行旅", "商號行商",
                "採", "樵", "礦", "行旅", "護送", "押送", "轉運",
            ]
            .iter()
            .any(|k| role_full.contains(k));

            let is_wilderness_dest = [
                "白塔雪巔", "赤石坡", "落日岬", "古河巨木林", "長風草甸", "西嶺礦谷", "清泉溪谷",
                "翠冠巨木林", "北坡雲杉林", "雲腰雪林", "東海峭壁", "淺澤泥口", "驚濤岩岸",
                "巨藻淺灣", "黑石灣", "白茫洲", "貼山深潭", "青籠關", "西麓林道",
            ]
            .iter()
            .any(|z| state.dest_name.contains(z));

            let home_or_errand_in_wilderness = [
                "白塔雪巔", "赤石坡", "落日岬", "古河巨木林", "長風草甸", "西嶺礦谷", "清泉溪谷",
                "翠冠巨木林", "北坡雲杉林", "雲腰雪林", "東海峭壁", "淺澤泥口", "驚濤岩岸",
                "巨藻淺灣", "黑石灣", "白茫洲", "貼山深潭", "青籠關", "西麓林道",
            ]
            .iter()
            .any(|z| slot.home_name.contains(z) || slot.errand_name.contains(z));

            let has_procure = state.purpose.contains("採辦急缺")
                || state.purpose.contains("採辦應急物產")
                || state.stay_activity.contains("急缺");

            let manufactured = ["麻繩卷", "塗焦麻繩", "補綴工靴", "粗鹽包", "木炭袋", "鐵料盒", "杉木厚板"]
                .iter()
                .any(|m| state.purpose.contains(m) || state.stay_activity.contains(m));

            if !is_forager && !home_or_errand_in_wilderness && (is_wilderness_dest || (has_procure && manufactured)) {
                let is_in_wilderness = [
                    "白塔雪巔", "赤石坡", "落日岬", "古河巨木林", "長風草甸", "西嶺礦谷", "清泉溪谷",
                    "翠冠巨木林", "北坡雲杉林", "雲腰雪林", "東海峭壁", "淺澤泥口", "驚濤岩岸",
                    "巨藻淺灣", "黑石灣", "白茫洲", "貼山深潭", "青籠關", "西麓林道",
                ]
                .iter()
                .any(|z| state.origin_name.contains(z));

                if is_in_wilderness {
                    state.dest = (slot.home_x, slot.home_y);
                    state.dest_name = slot.home_name.to_string();
                    state.purpose = "返家照看家務".to_string();
                    state.stay_activity = slot.home_stay_activity.to_string();
                    state.stay_ticks_remaining = 4;
                } else {
                    state.dest = (slot.errand_x, slot.errand_y);
                    state.dest_name = slot.errand_name.to_string();
                    state.purpose = slot.errand_purpose.to_string();
                    state.stay_activity = slot.errand_stay_activity.to_string();
                    state.stay_ticks_remaining = 4;
                }
                if let Some((ob_kind, _)) = crate::store::get_active_npc_obligation(npc_id) {
                    if ob_kind != "repay_debt" && ob_kind != "stranded_cargo" {
                        crate::store::clear_active_npc_obligation(npc_id, &ob_kind);
                    }
                }
            } else if is_forager && has_procure && manufactured {
                let raw_material = match (npc_id.len()) % 4 {
                    0 => "山野鮮藥草",
                    1 => "粗生礦石",
                    2 => "山林硬木料",
                    _ => "野味肉料",
                };
                state.purpose = format!("入山採辦{}", raw_material);
                state.stay_activity = format!("正於{}一帶尋覓並採集野生的「{}」", state.dest_name, raw_material);
            }
        }
    }

    // 2. Audit and sanitize live thoughts in entity store
    {
        let mut st = st_arc.write();
        for ent in st.entities.values_mut() {
            if ent.kind != "npc" {
                continue;
            }
            let mut mutated = false;
            let seed = ent.soul_seed.unwrap_or(0).unsigned_abs();
            let title = ent.display_title.clone();
            for t in &mut ent.thoughts {
                if let Some(flav) = &t.flavor {
                    let clean = sanitize_thought_string(flav, &title, seed);
                    if clean != *flav {
                        t.flavor = Some(clean);
                        mutated = true;
                    }
                }
            }
            if mutated {
                ent.disposition = ent.disposition.clamp(-100, 100);
                crate::pg::entity::upsert_async(ent.clone());
            }
        }
    }
}


fn resolve_ambient_caller_label(player_id: &str, npc_id: &str) -> String {
    let named = crate::store::player_knows_npc(player_id, npc_id);
    let person = get_npc_person_display_name(npc_id).unwrap_or_default();
    if named && !person.is_empty() {
        return person;
    }
    let store_opt = crate::store::get_store();
    let ent_opt = store_opt.as_ref().and_then(|s| s.read().get_entity(npc_id));
    let mut title = get_npc_title_from_assignments(npc_id);
    if title.is_empty() {
        if let Some(ent) = &ent_opt {
            title = ent.display_title.clone();
        }
    }
    if title.is_empty() {
        title = gametext::default_manager_title();
    }
    let appearance = crate::store::npc_brief_appearance_tag(npc_id, &title);
    if let Some(ent) = &ent_opt {
        let room_name = if let (Some(gx), Some(gy)) = (ent.grid_x, ent.grid_y) {
            crate::server::grid_manager::get_square_grid()
                .and_then(|g| g.get(crate::grid::SquareCoord::new(gx, gy)).map(|c| c.name.clone()))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let (is_staff, has_uniform, in_counter_zone) = crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            &room_name,
            npc_id,
            &ent.display_title,
            &ent.current_activity,
            &ent.equipment_slots,
        );
        if is_staff {
            let host_title = crate::grid::resolve_any_room_venue_object(&room_name, &ent.display_title)
                .map(|v| v.default_host_title)
                .unwrap_or("當值主事");
            if in_counter_zone {
                return if !appearance.is_empty() {
                    format!("站在櫃檯內側的{host_title}（{appearance}）")
                } else {
                    format!("站在櫃檯內側的{host_title}")
                };
            } else if has_uniform {
                return if !appearance.is_empty() {
                    format!("身著工作制服的{host_title}（{appearance}）")
                } else {
                    format!("身著工作制服的{host_title}")
                };
            }
        }
    }

    let role = if title.contains("鐵") || title.contains("鍛造") || title.contains("木作") || title.contains("工坊") {
        "匠人"
    } else if title.contains("商") || title.contains("鋪") || title.contains("市集") || title.contains("行") {
        "行商"
    } else if title.contains("醫") || title.contains("藥") || title.contains("診") {
        "醫工"
    } else if title.contains("書") || title.contains("學") || title.contains("講堂") || title.contains("觀星") {
        "學者"
    } else {
        "街坊"
    };
    if !appearance.is_empty() {
        format!("{appearance}的{role}")
    } else {
        format!("一名{role}")
    }
}

