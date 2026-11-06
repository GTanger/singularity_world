pub(crate) fn seed_all_residents_somatic_records(now_unix: i64) {
        for slot in crate::store::WENYAO_HOUSEHOLD_SLOTS.iter().chain(crate::store::OUTER_HOUSEHOLD_SLOTS.iter()) {
            let name = slot.name();
            let (dominant_breath, forge_s, forest_s, tide_s, chamber_s, battle_s, manual_title, fam_category, imprint_title, imprint_prose) =
                if slot.brief_tag.contains("鍛")
                    || slot.brief_tag.contains("鐵")
                    || slot.brief_tag.contains("礦")
                    || slot.brief_tag.contains("窯")
                    || slot.work_name.contains("鍛")
                    || slot.work_name.contains("鐵")
                    || slot.work_name.contains("礦")
                    || slot.work_name.contains("窯")
                {
                    (
                        BreathHabitKind::ForgeHeavy,
                        42,
                        8,
                        8,
                        8,
                        6,
                        "百鍛火候與錘路考工譜",
                        "赤鐵礦脈與百鍛鋼胚紋理",
                        "鐵坊鼓風落錘真傳",
                        "順風箱開闔沉肩落錘，聽音辨溫而百鍛成器",
                    )
                } else if slot.brief_tag.contains("水")
                    || slot.brief_tag.contains("磨")
                    || slot.brief_tag.contains("漁")
                    || slot.brief_tag.contains("船")
                    || slot.brief_tag.contains("海")
                    || slot.brief_tag.contains("鹽")
                    || slot.brief_tag.contains("渡")
                    || slot.work_name.contains("水")
                    || slot.work_name.contains("磨")
                    || slot.work_name.contains("漁")
                    || slot.work_name.contains("船")
                    || slot.work_name.contains("海")
                    || slot.work_name.contains("鹽")
                    || slot.work_name.contains("渡")
                {
                    (
                        BreathHabitKind::TideContinuous,
                        10,
                        8,
                        42,
                        8,
                        4,
                        "堰渠水輪與潮汐舟楫手札",
                        "水脈流速與舟車石磨齒距",
                        "水岸聽潮辨流心法",
                        "順潮汐水脈綿長調息，觀波紋緩急以拿捏分寸",
                    )
                } else if slot.brief_tag.contains("獵")
                    || slot.brief_tag.contains("林")
                    || slot.brief_tag.contains("木")
                    || slot.brief_tag.contains("藥")
                    || slot.brief_tag.contains("農")
                    || slot.brief_tag.contains("牧")
                    || slot.brief_tag.contains("茶")
                    || slot.work_name.contains("獵")
                    || slot.work_name.contains("林")
                    || slot.work_name.contains("木")
                    || slot.work_name.contains("藥")
                    || slot.work_name.contains("農")
                    || slot.work_name.contains("牧")
                    || slot.work_name.contains("茶")
                {
                    (
                        BreathHabitKind::ForestLong,
                        8,
                        42,
                        10,
                        8,
                        6,
                        "北山草木物候與百工採造錄",
                        "山林良木與百草筋脈肌理",
                        "山林順紋採造心傳",
                        "循四時物候與草木自然紋理，落手沉穩而不傷生機",
                    )
                } else {
                    (
                        BreathHabitKind::ChamberClear,
                        8,
                        8,
                        10,
                        42,
                        4,
                        "半島百業籌算與街坊通考",
                        "市井器物成色與百業規程",
                        "街坊掌事安頓心法",
                        "心平氣和核對往來器物與帳目，井然有序而不失分寸",
                    )
                };

            {
                let mut guard = STUDIED_MANUALS_CACHE.write().unwrap();
                let list = guard.entry(name.to_string()).or_default();
                if !list.iter().any(|m| m == manual_title) {
                    list.push(manual_title.to_string());
                }
            }

            {
                let mut guard = ANATOMY_FAMILIARITY_CACHE.write().unwrap();
                let map = guard.entry(name.to_string()).or_default();
                map.entry(fam_category.to_string()).or_insert(24);
            }

            {
                let mut guard = BREATH_HABIT_CACHE.write().unwrap();
                guard.entry(name.to_string()).or_insert_with(|| BreathHabitRecord {
                    entity_id: name.to_string(),
                    dominant: dominant_breath,
                    forge_score: forge_s,
                    forest_score: forest_s,
                    tide_score: tide_s,
                    chamber_score: chamber_s,
                    battle_score: battle_s,
                    updated_at: now_unix,
                });
            }

            {
                let mut guard = LINEAGE_IMPRINT_CACHE.write().unwrap();
                let list = guard.entry(name.to_string()).or_default();
                if list.is_empty() {
                    list.push(LineageImprintRecord {
                        entity_id: name.to_string(),
                        mentor_name: "行家老匠".to_string(),
                        imprint_title: imprint_title.to_string(),
                        imprint_prose: imprint_prose.to_string(),
                        interaction_count: 6,
                        manual_gifted: false,
                        updated_at: now_unix,
                    });
                }
            }

            let vessel = slot.held_vessel();
            if !vessel.is_empty() {
                let mut guard = VESSEL_STATE_CACHE.write().unwrap();
                guard
                    .entry((name.to_string(), vessel.to_string()))
                    .or_insert_with(|| VesselStateRecord {
                        entity_id: name.to_string(),
                        item_id: vessel.to_string(),
                        use_count: 24,
                        wear_points: 0,
                        condition: VesselCondition::Sharp,
                        updated_at: now_unix,
                    });
            }
        }
}

pub(crate) fn seed_qin_shuisheng_sql_venues_and_schedules() {
    let now_unix = crate::game::now_unix();
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "INSERT INTO venues (id, name, room_id, max_staff)
             VALUES ('wt-venue-laoyantou-watermill', '老堰頭・水輪石磨工坊', 'grid:-3:125', 6)
             ON CONFLICT (id) DO NOTHING",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO occupations (id, name, room_type)
             VALUES ('水輪石磨首座', '水輪石磨首座', 'workshop')
             ON CONFLICT (id) DO NOTHING",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO assignments (entity_id, occupation_id, venue_id, assigned_by)
             VALUES ('秦水生', '水輪石磨首座', 'wt-venue-laoyantou-watermill', 'resident_lineage')
             ON CONFLICT (entity_id, venue_id) DO NOTHING",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO schedules (entity_id, work_room, rest_room, shift_start, shift_end)
             VALUES ('秦水生', 'grid:-3:125', 'grid:-3:124', 7, 18)
             ON CONFLICT (entity_id) DO NOTHING",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO entity_vessel_states (entity_id, item_id, use_count, wear_points, condition_code, updated_at)
             VALUES
               ('秦水生', 'wt_cat_0973', 68, 0, 'sharp', $1),
               ('秦水生', 'wt_cat_0984', 42, 0, 'sharp', $1),
               ('岑守岳', 'wt_blacksmith_hammer', 56, 0, 'sharp', $1),
               ('柯成器', 'wt_carpenter_chisel', 48, 0, 'sharp', $1)
             ON CONFLICT (entity_id, item_id) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "INSERT INTO character_vessel_mastery (entity_id, item_id, use_count, wear_points, condition_code, updated_at)
             VALUES
               ('秦水生', 'wt_cat_0973', 68, 0, 'sharp', $1),
               ('秦水生', 'wt_cat_0984', 42, 0, 'sharp', $1),
               ('岑守岳', 'wt_blacksmith_hammer', 56, 0, 'sharp', $1),
               ('柯成器', 'wt_carpenter_chisel', 48, 0, 'sharp', $1)
             ON CONFLICT (entity_id, item_id) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "INSERT INTO entity_breath_habits (entity_id, dominant_code, forge_score, forest_score, tide_score, chamber_score, battle_score, updated_at)
             VALUES ('秦水生', 'tide_continuous', 14, 6, 48, 8, 2, $1)
             ON CONFLICT (entity_id) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "INSERT INTO entity_lineage_imprints (entity_id, mentor_name, imprint_title, imprint_prose, interaction_count, manual_gifted, updated_at)
             VALUES
               ('秦水生', '秦老堰主', '老堰頭水輪分動與雙疊石磨聽音辨距法', '順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性', 12, true, $1),
               ('岑守岳', '岑鐵匠', '岑氏百鍛落錘真傳', '百鍛淬鋒、分毫不差', 15, true, $1),
               ('柯成器', '柯木匠', '柯氏北山杉木榫卯真傳', '循木觀理、卯榫自合', 14, true, $1)
             ON CONFLICT (entity_id, mentor_name) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "INSERT INTO character_lineage_imprints (entity_id, mentor_name, imprint_title, imprint_prose, interaction_count, manual_gifted, updated_at)
             VALUES
               ('秦水生', '秦老堰主', '老堰頭水輪分動與雙疊石磨聽音辨距法', '順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性', 12, true, $1),
               ('岑守岳', '岑鐵匠', '岑氏百鍛落錘真傳', '百鍛淬鋒、分毫不差', 15, true, $1),
               ('柯成器', '柯木匠', '柯氏北山杉木榫卯真傳', '循木觀理、卯榫自合', 14, true, $1)
             ON CONFLICT (entity_id, mentor_name) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "UPDATE entity_lineage_imprints
             SET imprint_prose = '順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性'
             WHERE entity_id = '秦水生' AND imprint_title = '老堰頭水輪分動與雙疊石磨聽音辨距法'",
            &[],
        );
        let _ = client.execute(
            "UPDATE character_lineage_imprints
             SET imprint_prose = '順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性'
             WHERE entity_id = '秦水生' AND imprint_title = '老堰頭水輪分動與雙疊石磨聽音辨距法'",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO entity_anatomy_familiarity (entity_id, target_category, familiarity_count, updated_at)
             VALUES
               ('秦水生', '石磨鑿紋與水輪木齒', 28, $1),
               ('秦水生', '在來米與北山藥材水磨澄漿', 36, $1)
             ON CONFLICT (entity_id, target_category) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "DELETE FROM resident_knowledge WHERE entity_id = '秦水生' AND subject_ref LIKE 'npc_%'",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO resident_knowledge (entity_id, subject_ref, predicate, value, certainty, source_ref, learned_at, updated_at)
             VALUES
               ('秦水生', '老堰頭・水輪石磨工坊', 'millstone_and_sluice_state', '重礦包邊上下磨盤齒距已校準至三毫，清泉水脈二號引水木閘流速平穩，日可精碾晚造在來米十二石', 98, 'flagship_specimen_1', $1, $1),
               ('秦水生', '大圳頭・周萬生', 'upstream_grain_supply', '大圳頭周萬生本旬已備妥十袋晚造在來新穀與赤石坡抗旱麥，待明日辰時沿北郊水渠運抵老堰頭入磨', 95, 'flagship_specimen_1', $1, $1),
               ('秦水生', '文曜・陳萬福', 'morning_ricemilk_contract', '文曜三水大食堂每日寅末卯初需現磨溫米漿六桶與特級在來米粉八袋，須以薄層重礦烏木膽溫存桶封裝趁熱送達', 96, 'flagship_specimen_1', $1, $1),
               ('秦水生', '霜林口・梁千木', 'waterwheel_gear_maintenance', '水輪分動大軸的水青岡木齒輪每逢春霖漲水易受泥沙磨損，已約定霜林口梁千木備妥替換木齒與硬木楔子', 92, 'flagship_specimen_1', $1, $1),
               ('秦水生', '老堰頭・清泉水脈', 'hydrology_and_weather_omen', '每逢白塔北坡夜雨，老堰頭清泉水脈於寅時水位必漲三寸，須提前半刻落下一號分洪插板以免石磨轉速過急傷及米粉細度', 94, 'flagship_specimen_1', $1, $1)
             ON CONFLICT (entity_id, subject_ref, predicate) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "DELETE FROM resident_relationships WHERE entity_id = '秦水生' AND other_entity_id LIKE 'npc_%'",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO resident_relationships (entity_id, other_entity_id, relation_kind, evidence_count, source_ref, last_interaction_at, updated_at)
             VALUES
               ('秦水生', '周萬生', 'upstream_grain_and_canal_partner', 18, 'flagship_specimen_1', $1, $1),
               ('秦水生', '陳萬福', 'downstream_ricemilk_buyer', 24, 'flagship_specimen_1', $1, $1),
               ('秦水生', '梁千木', 'waterwheel_timber_craftsman', 11, 'flagship_specimen_1', $1, $1),
               ('秦水生', '蘇百草', 'north_mountain_herb_milling_partner', 14, 'flagship_specimen_1', $1, $1)
             ON CONFLICT (entity_id, other_entity_id, relation_kind) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "INSERT INTO npc_memories (entity_id, subject_id, meet_count, favorability)
             VALUES
               ('秦水生', '周萬生', 18, 78),
               ('秦水生', '陳萬福', 24, 82),
               ('秦水生', '梁千木', 11, 74),
               ('秦水生', '蘇百草', 14, 76)
             ON CONFLICT (entity_id, subject_id) DO NOTHING",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO npc_npc_summaries (dyad_key, summary)
             VALUES
               ('周萬生|秦水生', '秦水生與大圳頭水車農夫周萬生常年合議北郊水脈分水時刻，周萬生種出的晚造在來新穀皆交由秦水生水磨成粉與溫米漿。'),
               ('秦水生|陳萬福', '秦水生每日清晨將老堰頭水輪石磨慢熬的現磨溫米漿與特級在來米粉裝入重礦溫存桶，直供文曜三水大食堂陳萬福備早膳。'),
               ('梁千木|秦水生', '老堰頭水輪分動大軸與水青岡木齒輪皆由霜林口梁千木選材車製，秦水生則以現磨純糯米粉與溫米漿回禮。'),
               ('秦水生|蘇百草', '北山採回的乾燥藥材需借老堰頭細紋石磨慢碾成百草藥末而不傷藥性，秦水生常為蘇百草留出一座專磨藥材的副磨。')
             ON CONFLICT (dyad_key) DO NOTHING",
            &[],
        );
        let due_ts = now_unix + 14_400;
        let _ = client.execute(
            "INSERT INTO resident_obligations (obligation_id, entity_id, obligation_kind, subject_ref, status, due_at, source_ref, created_at, updated_at)
             VALUES
               ('ob_qin_shuisheng_morning_ricemilk_wenyao', '秦水生', 'restock_goods', '陳萬福・現磨溫米漿與特級在來米粉早市交割', 'open', $2, 'flagship_specimen_1', $1, $1),
               ('ob_qin_shuisheng_sluice_and_gear_check', '秦水生', 'repair_tool', '周萬生・老堰頭二號引水木閘與石磨齒距校準', 'open', $2, 'flagship_specimen_1', $1, $1),
               ('ob_cen_shouyue_morning_hardware_delivery', '岑守岳', 'restock_goods', '陳萬福・百鍛農具鐵件早市交割', 'open', $2, 'flagship_specimen_blacksmith', $1, $1),
               ('ob_ke_chengqi_morning_handle_delivery', '柯成器', 'restock_goods', '陳萬福・堅韌硬木工具長柄早市交割', 'open', $2, 'flagship_specimen_carpenter', $1, $1)
             ON CONFLICT (obligation_id) DO NOTHING",
            &[&now_unix, &due_ts],
        );
        let _ = client.execute(
            "UPDATE character_vessel_mastery SET wear_points = 0, condition_code = 'sharp'
             WHERE entity_id = '秦水生' AND item_id = 'wt_cat_0973' AND wear_points > 0",
            &[],
        );
        if let Ok(mut guard) = LINEAGE_IMPRINT_CACHE.write() {
            guard.entry("岑守岳".to_string()).or_default().push(LineageImprintRecord {
                entity_id: "岑守岳".to_string(),
                mentor_name: "岑鐵匠".to_string(),
                imprint_title: "岑氏百鍛落錘真傳".to_string(),
                imprint_prose: "百鍛淬鋒、分毫不差".to_string(),
                interaction_count: 15,
                manual_gifted: true,
                updated_at: now_unix,
            });
            guard.entry("柯成器".to_string()).or_default().push(LineageImprintRecord {
                entity_id: "柯成器".to_string(),
                mentor_name: "柯木匠".to_string(),
                imprint_title: "柯氏北山杉木榫卯真傳".to_string(),
                imprint_prose: "循木觀理、卯榫自合".to_string(),
                interaction_count: 14,
                manual_gifted: true,
                updated_at: now_unix,
            });
        }
        if let Ok(mut guard) = VESSEL_STATE_CACHE.write() {
            guard.insert(
                ("岑守岳".to_string(), "wt_blacksmith_hammer".to_string()),
                VesselStateRecord {
                    entity_id: "岑守岳".to_string(),
                    item_id: "wt_blacksmith_hammer".to_string(),
                    use_count: 56,
                    wear_points: 0,
                    condition: VesselCondition::Sharp,
                    updated_at: now_unix,
                },
            );
            guard.insert(
                ("柯成器".to_string(), "wt_carpenter_chisel".to_string()),
                VesselStateRecord {
                    entity_id: "柯成器".to_string(),
                    item_id: "wt_carpenter_chisel".to_string(),
                    use_count: 48,
                    wear_points: 0,
                    condition: VesselCondition::Sharp,
                    updated_at: now_unix,
                },
            );
        }
    }
}

/// 舊式依實體辨識現場人員輔助函數
pub fn is_recognizable_room_staff_or_counter_holder_entity(
    ent: &crate::store::Entity,
    _room_coord: (i32, i32),
    needed_item_id: &str,
) -> (bool, bool) {
    // 1. 檢查裝備槽位是否有可見制服／圍裙／工務裝束
    let mut has_visible_uniform = false;
    if let Ok(slots) =
        serde_json::from_str::<std::collections::HashMap<String, String>>(&ent.equipment_slots)
    {
        for slot in &["body", "waist", "wrist", "hold_r", "hold_l"] {
            if let Some(item_id) = slots.get(*slot).filter(|s| !s.trim().is_empty()) {
                if crate::game::peninsula_catalog::is_work_uniform_item_id_or_name(item_id) {
                    has_visible_uniform = true;
                    break;
                }
            }
        }
    }
    if !has_visible_uniform {
        if ent.display_title.contains("穿制服")
            || ent.display_title.contains("店員")
            || ent.display_title.contains("制服")
            || ent.display_title.contains("圍裙")
            || ent.display_title.contains("工長")
            || ent.display_title.contains("司秤")
            || ent.display_title.contains("主廚")
        {
            has_visible_uniform = true;
        }
    }
    if !has_visible_uniform && !ent.id.is_empty() {
        let slot = crate::store::wenyao_household_slot_for(&ent.id);
        if crate::game::peninsula_catalog::is_work_uniform_item_id_or_name(slot.attire_line) {
            has_visible_uniform = true;
        }
    }

    // 2. 檢查是否站立於該房間之「非本號夥計禁止入內區／櫃檯內場」
    // 即便身穿便服，站在內場櫃檯管制區，或持有該店舖原料庫存且具備常駐站席，即辨識為在場值守掌事／櫃檯主事
    let is_passerby_customer = ent.display_title.contains("顧客")
        || ent.display_title.contains("路人")
        || ent.display_title.contains("茶客")
        || ent.display_title.contains("食客")
        || ent.display_title.contains("香客")
        || ent.display_title.contains("過客")
        || ent.display_title.contains("外地商旅");

    let mut has_counter_zone_marker = ent.display_title.contains("櫃檯")
        || ent.display_title.contains("櫃台")
        || ent.display_title.contains("非夥計禁止入內")
        || ent.display_title.contains("非本店人員禁止進入區")
        || ent.display_title.contains("非本號夥計禁止入內區")
        || ent.display_title.contains("內場")
        || ent.display_title.contains("掌事")
        || ent.display_title.contains("主事")
        || ent.display_title.contains("當值")
        || ent.display_title.contains("首座");

    if !has_counter_zone_marker {
        if let Some(v) = crate::grid::resolve_any_room_venue_object(&ent.current_activity, &ent.display_title) {
            if v.has_counter_restricted_zone
                && (ent.current_activity.contains("櫃")
                    || ent.current_activity.contains("爐")
                    || ent.current_activity.contains("釜")
                    || ent.current_activity.contains("砧")
                    || ent.current_activity.contains("磨")
                    || ent.current_activity.contains("藥")
                    || ent.current_activity.contains("算")
                    || ent.current_activity.contains("帳")
                    || ent.current_activity.contains("攤"))
            {
                has_counter_zone_marker = true;
            }
        }
    }

    let stock_count = if !needed_item_id.is_empty() {
        inv_count_item(&ent.inventory, needed_item_id)
    } else {
        0
    };

    let in_staff_only_counter_zone = if is_passerby_customer && stock_count <= 0 && !has_counter_zone_marker {
        false
    } else if has_counter_zone_marker {
        true
    } else if (stock_count > 0 || ent.magnesium >= 8) && !is_passerby_customer {
        // 持有該店庫存或商業結算能力且非路人顧客，視為在場值守人員
        true
    } else {
        false
    };

    (has_visible_uniform, in_staff_only_counter_zone)
}

/// 依當前座標 (cx, cy) 尋找半徑內最具優勢之上游供貨商或房間設施櫃檯
pub fn resolve_need_decision_supplier(
    store: &crate::store::Store,
    entity_id: &str,
    cur_xy: (i32, i32),
    factors: &ResidentIntrinsicFactors,
    pw: &crate::npc::causal_core::PersonalityWeights,
    hooks: &[crate::npc::causal_core::RelationalHook],
    grid_opt: Option<&crate::grid::SquareGrid>,
) -> Option<(String, String, (i32, i32))> {
    let mut best: Option<(String, String, (i32, i32), i32)> = None;

    // 1. 掃描 Store 內之實體候選人（上游供貨商／掌事／店員）
    for (cand_id, cand_ent) in &store.entities {
        if cand_id == entity_id {
            continue;
        }
        let stock = inv_count_item(&cand_ent.inventory, &factors.raw_item_id);
        if stock <= 0 {
            continue;
        }

        let cand_coord = cand_ent
            .grid_x
            .zip(cand_ent.grid_y)
            .unwrap_or(factors.work_coord);

        let (has_uniform, in_counter_zone) =
            is_recognizable_room_staff_or_counter_holder_entity(cand_ent, cand_coord, &factors.raw_item_id);

        if !has_uniform
            && !in_counter_zone
            && (cand_ent.display_title.contains("顧客") || cand_ent.display_title.contains("路人"))
        {
            continue;
        }

        let mut trust = 0i32;
        let mut grudge = 0i32;
        if let Some(h) = hooks
            .iter()
            .find(|h| h.target_id == *cand_id || h.target_name == *cand_id)
        {
            if is_grudge_hook(h) {
                grudge = h.intensity as i32;
            } else if is_favor_hook(h) {
                trust = h.intensity as i32;
            }
        }
        if grudge >= 15 && pw.pride >= 95 {
            continue;
        }

        let dist = (cand_coord.0 - cur_xy.0).abs() + (cand_coord.1 - cur_xy.1).abs();

        let tag_match = grid_opt
            .and_then(|g| g.get(crate::grid::SquareCoord::new(cand_coord.0, cand_coord.1)))
            .map_or(false, |c| c.has_room_tag(factors.need_supply_tag));

        let tag_bonus = if tag_match { 200 } else { 0 };
        let uniform_bonus = if has_uniform { 50 } else { 0 };
        let counter_bonus = if in_counter_zone { 50 } else { 0 };
        let dist_penalty = dist * 15;

        let score = stock * 25
            + tag_bonus
            + uniform_bonus
            + counter_bonus
            + trust * (pw.honor / 8).max(4)
            - grudge * (pw.pride / 4).max(10)
            - dist_penalty;

        if best.as_ref().map(|(_, _, _, bs)| score > *bs).unwrap_or(true) {
            let place_label = if let Some(cell) = grid_opt
                .and_then(|g| g.get(crate::grid::SquareCoord::new(cand_coord.0, cand_coord.1)))
            {
                if !cell.zone.is_empty() {
                    cell.zone.clone()
                } else {
                    cell.name.clone()
                }
            } else if cand_ent.display_title.contains('・') {
                cand_ent
                    .display_title
                    .split('・')
                    .next()
                    .unwrap_or(&factors.supplier_place_label)
                    .to_string()
            } else if let Some(s) = crate::store::resident_macro_settlement(cand_id, &cand_ent.display_title) {
                s.to_string()
            } else {
                factors.supplier_place_label.clone()
            };
            best = Some((cand_id.clone(), place_label, cand_coord, score));
        }
    }

    // 2. 掃描方格地圖中符合需求之房間
    if let Some(grid) = grid_opt {
        for cell in grid.cells() {
            if cell.has_room_tag(factors.need_supply_tag) {
                let rcoord = (cell.coord.x, cell.coord.y);
                let dist = (rcoord.0 - cur_xy.0).abs() + (rcoord.1 - cur_xy.1).abs();

                let staff_opt = store.entities.iter().find(|(cid, ce)| {
                    *cid != entity_id && ce.grid_x.zip(ce.grid_y) == Some(rcoord) && {
                        let (u, c) = is_recognizable_room_staff_or_counter_holder_entity(
                            ce,
                            rcoord,
                            &factors.raw_item_id,
                        );
                        u || c
                    }
                });

                if let Some((staff_id, staff_ent)) = staff_opt {
                    let stock = inv_count_item(&staff_ent.inventory, &factors.raw_item_id).max(4);
                    let (has_uniform, in_counter_zone) = is_recognizable_room_staff_or_counter_holder_entity(
                        staff_ent,
                        rcoord,
                        &factors.raw_item_id,
                    );
                    let uniform_bonus = if has_uniform { 50 } else { 0 };
                    let counter_bonus = if in_counter_zone { 50 } else { 0 };
                    let score = stock * 25 + 200 + uniform_bonus + counter_bonus - dist * 15;
                    if best.as_ref().map(|(_, _, _, bs)| score > *bs).unwrap_or(true) {
                        let place_label = if !cell.zone.is_empty() {
                            cell.zone.clone()
                        } else {
                            cell.name.clone()
                        };
                        best = Some((staff_id.clone(), place_label, rcoord, score));
                    }
                } else {
                    let room_score = 150 - dist * 15;
                    if best.as_ref().map(|(_, _, _, bs)| room_score > *bs).unwrap_or(true) {
                        let place_label = if !cell.zone.is_empty() {
                            cell.zone.clone()
                        } else {
                            cell.name.clone()
                        };
                        let counter_name = format!("{}櫃檯", cell.name);
                        best = Some((counter_name, place_label, rcoord, room_score));
                    }
                }
            }
        }
    }

    best.map(|(id, place, coord, _)| (id, place, coord))
}

/// 依當前座標 (cx, cy) 尋找半徑內最具優勢之下游交割買方或房間設施櫃檯
pub fn resolve_need_decision_buyer(
    store: &crate::store::Store,
    entity_id: &str,
    cur_xy: (i32, i32),
    primary_out_id: &str,
    factors: &ResidentIntrinsicFactors,
    pw: &crate::npc::causal_core::PersonalityWeights,
    hooks: &[crate::npc::causal_core::RelationalHook],
    grid_opt: Option<&crate::grid::SquareGrid>,
) -> Option<(String, String, (i32, i32))> {
    let mut best: Option<(String, String, (i32, i32), i32)> = None;

    // 1. 掃描 Store 內之買方實體（食堂主事／商鋪管事／收購商）
    for (cand_id, cand_ent) in &store.entities {
        if cand_id == entity_id || cand_id == &factors.supplier_id {
            continue;
        }
        if cand_ent.magnesium < 8 {
            continue;
        }

        let cand_coord = cand_ent
            .grid_x
            .zip(cand_ent.grid_y)
            .unwrap_or(factors.deliver_coord);

        let (has_uniform, in_counter_zone) =
            is_recognizable_room_staff_or_counter_holder_entity(cand_ent, cand_coord, primary_out_id);

        if !has_uniform
            && !in_counter_zone
            && (cand_ent.display_title.contains("顧客") || cand_ent.display_title.contains("路人"))
        {
            continue;
        }

        let mut trust = 0i32;
        let mut grudge = 0i32;
        if let Some(h) = hooks
            .iter()
            .find(|h| h.target_id == *cand_id || h.target_name == *cand_id)
        {
            if is_grudge_hook(h) {
                grudge = h.intensity as i32;
            } else if is_favor_hook(h) {
                trust = h.intensity as i32;
            }
        }
        if grudge >= 15 && pw.pride >= 95 {
            continue;
        }

        let dist = (cand_coord.0 - cur_xy.0).abs() + (cand_coord.1 - cur_xy.1).abs();
        let buyer_stock = inv_count_item(&cand_ent.inventory, primary_out_id);
        let stock_need_bonus = if buyer_stock < 4 { 60 } else { 0 };

        let tag_match = grid_opt
            .and_then(|g| g.get(crate::grid::SquareCoord::new(cand_coord.0, cand_coord.1)))
            .map_or(false, |c| c.has_room_tag(factors.need_buyer_tag));

        let tag_bonus = if tag_match { 200 } else { 0 };
        let uniform_bonus = if has_uniform { 50 } else { 0 };
        let counter_bonus = if in_counter_zone { 50 } else { 0 };
        let dist_penalty = dist * 15;

        let score = cand_ent.magnesium.min(50) * 2
            + stock_need_bonus
            + tag_bonus
            + uniform_bonus
            + counter_bonus
            + trust * (pw.honor / 8).max(4)
            - grudge * (pw.pride / 4).max(10)
            - dist_penalty;

        if best.as_ref().map(|(_, _, _, bs)| score > *bs).unwrap_or(true) {
            let place_label = if let Some(cell) = grid_opt
                .and_then(|g| g.get(crate::grid::SquareCoord::new(cand_coord.0, cand_coord.1)))
            {
                if !cell.zone.is_empty() {
                    cell.zone.clone()
                } else {
                    cell.name.clone()
                }
            } else if cand_ent.display_title.contains('・') {
                cand_ent
                    .display_title
                    .split('・')
                    .next()
                    .unwrap_or(&factors.deliver_bldg)
                    .to_string()
            } else if let Some(s) = crate::store::resident_macro_settlement(cand_id, &cand_ent.display_title) {
                s.to_string()
            } else {
                factors.deliver_bldg.clone()
            };
            best = Some((cand_id.clone(), place_label, cand_coord, score));
        }
    }

    // 2. 掃描方格地圖中符合需求之房間
    if let Some(grid) = grid_opt {
        for cell in grid.cells() {
            if cell.has_room_tag(factors.need_buyer_tag) {
                let rcoord = (cell.coord.x, cell.coord.y);
                let dist = (rcoord.0 - cur_xy.0).abs() + (rcoord.1 - cur_xy.1).abs();

                let staff_opt = store.entities.iter().find(|(cid, ce)| {
                    *cid != entity_id
                        && *cid != &factors.supplier_id
                        && ce.magnesium >= 8
                        && ce.grid_x.zip(ce.grid_y) == Some(rcoord)
                        && {
                            let (u, c) = is_recognizable_room_staff_or_counter_holder_entity(
                                ce,
                                rcoord,
                                primary_out_id,
                            );
                            u || c
                        }
                });

                if let Some((staff_id, staff_ent)) = staff_opt {
                    let (has_uniform, in_counter_zone) = is_recognizable_room_staff_or_counter_holder_entity(
                        staff_ent,
                        rcoord,
                        primary_out_id,
                    );
                    let uniform_bonus = if has_uniform { 50 } else { 0 };
                    let counter_bonus = if in_counter_zone { 50 } else { 0 };
                    let score = staff_ent.magnesium.min(50) * 2
                        + 200
                        + uniform_bonus
                        + counter_bonus
                        - dist * 15;
                    if best.as_ref().map(|(_, _, _, bs)| score > *bs).unwrap_or(true) {
                        let place_label = if !cell.zone.is_empty() {
                            cell.zone.clone()
                        } else {
                            cell.name.clone()
                        };
                        best = Some((staff_id.clone(), place_label, rcoord, score));
                    }
                } else {
                    let room_score = 150 - dist * 15;
                    if best.as_ref().map(|(_, _, _, bs)| room_score > *bs).unwrap_or(true) {
                        let place_label = if !cell.zone.is_empty() {
                            cell.zone.clone()
                        } else {
                            cell.name.clone()
                        };
                        let counter_name = format!("{}櫃檯", cell.name);
                        best = Some((counter_name, place_label, rcoord, room_score));
                    }
                }
            }
        }
    }

    best.map(|(id, place, coord, _)| (id, place, coord))
}

