/// 依據劇本之場所資訊、標題與關鍵物件推導房間語意標籤
pub fn infer_script_room_tags(entry: &PeninsulaScriptEntry) -> Vec<String> {
    let text = format!("{} {} {} {} {}", entry.title, entry.location_and_npc, entry.key_objects, entry.category_name, entry.trigger_and_causal_chain);
    let loc = entry.location_and_npc;
    let mut tags = Vec::new();

    // 1. 工坊
    if text.contains("工坊") || text.contains("作坊") || text.contains("磨坊") || text.contains("大鋸") {
        tags.push("workshop".to_string());
    }
    // 2. 磨石
    if text.contains("磨石") || text.contains("石磨") || text.contains("水磨") || text.contains("磨刀石") {
        tags.push("grindstone".to_string());
    }
    // 3. 穀物
    if text.contains("穀物") || text.contains('穀') || text.contains('米') || text.contains('麥') || text.contains('糧') {
        tags.push("grain".to_string());
    }
    // 4. 糧倉
    if text.contains("糧倉") || text.contains("公倉") || text.contains("穀倉") {
        tags.push("granary".to_string());
    }
    // 5. 食堂
    if text.contains("食堂") || text.contains("大食堂") || text.contains("餐館") || text.contains("飯館") {
        tags.push("canteen".to_string());
    }
    // 6. 熱食
    if text.contains("熱食") || text.contains("熟食") || text.contains("米漿") || text.contains("早點")
        || text.contains("牛肉湯") || text.contains("豆花") || text.contains("烤肉") || text.contains("番薯")
        || text.contains("烙餅") || text.contains("碗粿")
    {
        tags.push("hot_food".to_string());
    }
    // 7. 現煮飲品
    if text.contains("現煮飲品") || text.contains("飲品") || text.contains("煮茶") || text.contains("煎茶")
        || text.contains("茶飲") || text.contains("鮮奶茶") || text.contains("手沖")
    {
        tags.push("fresh_drink".to_string());
    }
    // 8. 礦場
    if text.contains("礦場") || text.contains("坑口") || text.contains("礦坑") || text.contains("礦務") || text.contains("採礦") {
        tags.push("mine".to_string());
    }
    // 9. 礦石
    if text.contains("礦石") || text.contains("原礦") || text.contains("赤鐵") || text.contains("鐵錠")
        || text.contains("選礦") || loc.contains("重礦")
    {
        tags.push("ore".to_string());
    }
    // 10. 鐵匠鋪
    if text.contains("鐵匠鋪") || text.contains("鐵匠") || text.contains("打鐵") || text.contains("鐵坊")
        || text.contains("羊角錘") || text.contains("鍛錘")
    {
        tags.push("blacksmith".to_string());
    }
    // 11. 鍛造
    if text.contains("鍛造") || text.contains("鍛鋼") || text.contains("冶煉") || text.contains("鑄造") {
        tags.push("forge".to_string());
    }
    // 12. 林場
    if text.contains("林場") || text.contains("伐木") || text.contains("霜林") || text.contains("林口") || text.contains("護林") {
        tags.push("lumber".to_string());
    }
    // 13. 木材
    if text.contains("木材") || text.contains("原木") || text.contains("雲杉") || text.contains("樑材")
        || text.contains("木料") || text.contains("松木")
    {
        tags.push("timber".to_string());
    }
    // 14. 漁港
    if text.contains("漁港") || text.contains("船塢") || text.contains("漁船") || text.contains("碼頭")
        || text.contains("風舶") || text.contains("港灣")
    {
        tags.push("fishing_port".to_string());
    }
    // 15. 海鹽
    if text.contains("海鹽") || text.contains("鹽田") || text.contains("鹽場") || text.contains("粗鹽") || text.contains("海貝灰") {
        tags.push("sea_salt".to_string());
    }
    // 16. 藥圃
    if text.contains("藥圃") || text.contains("草藥圃") || text.contains("百草") {
        tags.push("herb".to_string());
    }
    // 17. 藥材
    if text.contains("藥材") || text.contains("草藥") || text.contains("靈芝") || text.contains("金創")
        || text.contains("草果") || text.contains("艾草")
    {
        tags.push("medicine".to_string());
    }
    // 18. 醫館
    if text.contains("醫館") || text.contains("診所") || text.contains("診堂") || text.contains("郎中") || text.contains("女醫") {
        tags.push("clinic".to_string());
    }
    // 19. 織坊
    if text.contains("織坊") || text.contains("織造") || text.contains("紡織") || text.contains("染坊") {
        tags.push("textile".to_string());
    }
    // 20. 布料
    if text.contains("布料") || text.contains("棉布") || text.contains("絲綢") || text.contains("麻布")
        || text.contains("長衫") || text.contains("帆布")
    {
        tags.push("fabric".to_string());
    }
    // 21. 商鋪
    if text.contains("商鋪") || text.contains("店鋪") || text.contains("商號") || text.contains("雜貨")
        || text.contains("商盟") || text.contains("票號") || text.contains("柑仔店")
    {
        tags.push("shop".to_string());
    }
    if text.contains("總商行") || text.contains("信匯") || text.contains("票號") || text.contains("匯兌")
        || text.contains("存摺") || text.contains("匯票")
    {
        tags.push("bank".to_string());
    }
    // 22. 市集
    if text.contains("市集") || text.contains("市場") || text.contains("墟市") || text.contains("夜市") {
        tags.push("market".to_string());
    }
    // 23. 水脈
    if text.contains("水脈") || text.contains("清泉") || text.contains("地下水") || text.contains("水渠")
        || text.contains("水車") || text.contains("水泉")
    {
        tags.push("water".to_string());
    }
    // 24. 分洪閘
    if text.contains("分洪閘") || text.contains("分洪") || text.contains("節制閘") || text.contains("排洪") {
        tags.push("sluice".to_string());
    }
    // 25. 歇宿
    if text.contains("歇宿") || text.contains("客棧") || text.contains("旅社") || text.contains("旅店")
        || text.contains("客房") || text.contains("起居")
    {
        tags.push("inn".to_string());
    }
    if text.contains("牧場") || text.contains("駝獸") || text.contains("牛乳") || text.contains("乳酪") || text.contains("草場") || text.contains("馬靴") {
        tags.push("ranch".to_string());
    }
    if text.contains("石場") || text.contains("石材") || text.contains("青石") || text.contains("玉石") || text.contains("灰窯") || text.contains("石灰") {
        tags.push("masonry".to_string());
    }
    if text.contains("茶") || text.contains("山茶") || text.contains("焙籠") {
        tags.push("teahouse".to_string());
    }
    if text.contains("書院") || text.contains("學堂") || text.contains("公議堂") || text.contains("藏書") || text.contains("星盤") || text.contains("觀星") || text.contains("鐘樓") {
        tags.push("academy".to_string());
    }
    if text.contains("哨") || text.contains("巡防") || text.contains("武館") || text.contains("演武") || text.contains("關隘") {
        tags.push("defense".to_string());
    }
    if tags.is_empty() {
        tags.push("settlement".to_string());
    }
    tags
}

/// 依據場所名稱或聚落名稱查詢關聯的日常劇本切面（包含地名子字串與房間語意標籤匹配）
pub fn scripts_for_room(room_name: &str) -> Vec<&'static PeninsulaScriptEntry> {
    let raw_settlement = room_name.split('・').next().unwrap_or(room_name).trim();
    let settlement = match raw_settlement {
        "西麓林道" => "北嶺",
        "清泉溪谷" => "半嶺",
        "赤石坡" => "三號坑",
        "長風草甸" => "牛埔",
        "落日岬" => "外澳",
        "淺澤泥口" => "水尾寮",
        other => other,
    };
    let room_tags = crate::grid::derive_room_tags(crate::grid::Terrain::Road, room_name, "");
    let has_exchange_tag = room_tags.iter().any(|t| crate::grid::canonical_room_tag(t) == "bank");
    let is_umbrella_tag = |t: &str| {
        matches!(
            crate::grid::canonical_room_tag(t),
            "settlement"
                | "workshop"
                | "shop"
                | "counter"
                | "restricted_staff_only"
                | "civic_facility"
                | "supply_hub"
                | "social_hub"
                | "road"
                | "bridge"
        ) || (has_exchange_tag && crate::grid::canonical_room_tag(t) == "market")
    };
    let specific_room_tags: Vec<&String> = room_tags
        .iter()
        .filter(|t| !is_umbrella_tag(t.as_str()))
        .collect();
    let tail_room = room_name.rsplit('・').next().unwrap_or(room_name).trim();
    let loc_matched: Vec<&'static PeninsulaScriptEntry> = PENINSULA_DAILY_SCRIPTS
        .iter()
        .filter(|s| {
            (!settlement.is_empty() && s.location_and_npc.contains(settlement))
                || s.location_and_npc.contains(room_name)
                || (!tail_room.is_empty() && s.location_and_npc.contains(tail_room))
        })
        .collect();
    if !loc_matched.is_empty() {
        let exact_match: Vec<&'static PeninsulaScriptEntry> = loc_matched
            .iter()
            .copied()
            .filter(|s| {
                s.location_and_npc.contains(room_name)
                    || (!tail_room.is_empty() && s.location_and_npc.contains(tail_room))
            })
            .collect();
        if !exact_match.is_empty() {
            return exact_match;
        }
        let domain_matched: Vec<&'static PeninsulaScriptEntry> = loc_matched
            .iter()
            .copied()
            .filter(|s| {
                !specific_room_tags.is_empty()
                    && infer_script_room_tags(s)
                        .iter()
                        .any(|st| specific_room_tags.iter().any(|rt| *rt == st))
            })
            .collect();
        if !domain_matched.is_empty() {
            return domain_matched;
        }
    }
    let mut matched: Vec<&'static PeninsulaScriptEntry> = PENINSULA_DAILY_SCRIPTS
        .iter()
        .filter(|s| {
            if specific_room_tags.is_empty() {
                false
            } else {
                let s_tags = infer_script_room_tags(s);
                s_tags.iter().any(|st| specific_room_tags.iter().any(|rt| *rt == st))
            }
        })
        .collect();
    if matched.is_empty() {
        let hash = room_name.bytes().fold(17usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize));
        if !loc_matched.is_empty() {
            let total = loc_matched.len();
            matched.push(loc_matched[hash % total]);
            matched.push(loc_matched[(hash + 37) % total]);
        } else {
            let total = PENINSULA_DAILY_SCRIPTS.len();
            matched.push(&PENINSULA_DAILY_SCRIPTS[hash % total]);
            matched.push(&PENINSULA_DAILY_SCRIPTS[(hash + 37) % total]);
        }
    }
    matched
}

/// 依據場所名稱解析當值主事（host）與協力鄰坊主事（partner）
pub fn resolve_on_duty_actors_for_room(room_name: &str) -> (Option<String>, Option<String>) {
    if room_name.trim().is_empty() {
        return (None, None);
    }
    let Some(store_arc) = crate::store::get_store() else {
        return (None, None);
    };
    let s = store_arc.read();

    let room_id_opt = s.rooms.iter().find(|(_, r)| {
        r.name == room_name || room_name.contains(&r.name) || r.name.contains(room_name)
    }).map(|(id, _)| id.clone());

    let mut room_entities = Vec::new();

    for e in s.entities.values() {
        if e.kind != "npc" {
            continue;
        }
        let in_room = match &room_id_opt {
            Some(rid) if s.entity_rooms.get(&e.id) == Some(rid) => true,
            _ => e.place_id.as_deref().is_some_and(|p| !p.is_empty() && (p.contains(room_name) || room_name.contains(p))),
        };
        if !in_room {
            continue;
        }
        let coord = (e.grid_x.unwrap_or(0), e.grid_y.unwrap_or(0));
        let (has_uniform, in_counter) =
            crate::pg::living_world::is_recognizable_room_staff_or_counter_holder_entity(e, coord, "");
        let is_staff = has_uniform || in_counter;

        let name = crate::store::resolve_npc_person_name(&e.id, &e.display_title, Some(&e.gender));
        let actor_name = if !name.is_empty() && name != "陌生人" {
            name
        } else if !e.display_title.is_empty() {
            e.display_title.clone()
        } else {
            e.id.clone()
        };

        room_entities.push((actor_name, is_staff));
    }

    let mut host_opt = None;
    let mut host_idx = None;
    for (i, (name, is_staff)) in room_entities.iter().enumerate() {
        if *is_staff {
            host_opt = Some(name.clone());
            host_idx = Some(i);
            break;
        }
    }
    if host_opt.is_none() && !room_entities.is_empty() {
        host_opt = Some(room_entities[0].0.clone());
        host_idx = Some(0);
    }
    if host_opt.is_none() {
        host_opt = crate::grid::resolve_any_room_venue_object(room_name, "")
            .map(|v| v.default_host_title.to_string());
    }

    let mut partner_opt = None;
    if let Some(h_idx) = host_idx {
        for (i, (name, _)) in room_entities.iter().enumerate() {
            if i != h_idx {
                partner_opt = Some(name.clone());
                break;
            }
        }
    }
    if partner_opt.is_none() {
        partner_opt = Some("鄰坊當值主事".to_string());
    }

    (host_opt, partner_opt)
}

/// 移除劇本中物件或動作文字帶有的 `WT-XXXX ` 編號前綴
pub fn strip_wt_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == 'W' && chars.peek() == Some(&'T') {
            let rest: String = chars.clone().collect();
            if rest.starts_with("T-") {
                let after_prefix = &rest[2..];
                let digits_len = after_prefix.chars().take_while(|ch| ch.is_ascii_digit()).count();
                if digits_len == 4 {
                    // Consume 'T', '-', and the 4 digits
                    for _ in 0..6 {
                        chars.next();
                    }
                    // Consume optional trailing whitespace
                    if chars.peek() == Some(&' ') {
                        chars.next();
                    }
                    continue;
                }
            }
        }
        result.push(c);
    }
    result
}

/// 將故事母體切面拆解為客觀環境條件觸發的「事件包」（EventPack）與角色欄位動態綁定的「劇本包」（ScriptPack）
pub fn decompose_into_event_and_script_pack(
    entry: &PeninsulaScriptEntry,
    room_name: &str,
    host_override: Option<&str>,
    partner_override: Option<&str>,
) -> (EventPack, ScriptPack) {
    let (resolved_host, resolved_partner) = resolve_on_duty_actors_for_room(room_name);
    let host_actor = host_override
        .map(|s| s.to_string())
        .or(resolved_host)
        .unwrap_or_else(|| "當值主事".to_string());
    let partner_actor = partner_override
        .map(|s| s.to_string())
        .or(resolved_partner)
        .unwrap_or_else(|| "鄰坊當值主事".to_string());

    // 擷取原始典範姓名清單
    let mut primary_exemplar = String::new();
    let mut secondary_exemplar = String::new();

    let npc_segment = if let Some((_, after_semi)) = entry.location_and_npc.split_once('；') {
        after_semi
    } else if let Some(inside_paren) = entry.location_and_npc.split('（').nth(1).and_then(|s| s.split('）').next()) {
        inside_paren
    } else if let Some((_, after_comma)) = entry.location_and_npc.split_once('、') {
        after_comma
    } else {
        ""
    };

    let mut found_exemplars: Vec<String> = Vec::new();
    for sub in npc_segment.split(['、', '，', '與', '和', ' ']) {
        let trimmed = sub.trim();
        if trimmed.is_empty() {
            continue;
        }
        for &name in EXEMPLAR_NAMES {
            if trimmed.contains(name) && !found_exemplars.iter().any(|e| e == name) {
                found_exemplars.push(name.to_string());
            }
        }
    }

    for &name in EXEMPLAR_NAMES {
        if (entry.location_and_npc.contains(name) || entry.title.contains(name))
            && !found_exemplars.iter().any(|e| e == name)
        {
            found_exemplars.push(name.to_string());
        }
    }

    if let Some(first) = found_exemplars.first() {
        primary_exemplar = first.clone();
    }
    if let Some(second) = found_exemplars.get(1) {
        secondary_exemplar = second.clone();
    }

    // 處理 event_title（去除典範人物姓名）
    let mut event_title = entry.title.to_string();
    if !primary_exemplar.is_empty() && !secondary_exemplar.is_empty() {
        event_title = event_title.replace(&format!("{primary_exemplar}與{secondary_exemplar}"), "");
        event_title = event_title.replace(&format!("{primary_exemplar}和{secondary_exemplar}"), "");
        event_title = event_title.replace(&format!("{primary_exemplar}、{secondary_exemplar}"), "");
    }
    if !primary_exemplar.is_empty() {
        event_title = event_title.replace(&primary_exemplar, "");
    }
    if !secondary_exemplar.is_empty() {
        event_title = event_title.replace(&secondary_exemplar, "");
    }
    for &name in EXEMPLAR_NAMES {
        event_title = event_title.replace(name, "");
    }
    event_title = event_title.trim_matches(|c: char| {
        c == '・' || c == '與' || c == '和' || c == '的' || c == '之' || c == ' ' || c == '、' || c == '，'
    }).to_string();
    if event_title.is_empty() {
        event_title = entry.title.to_string();
    }

    // 處理 location_context（取場所純地名並剝離人物姓名）
    let loc_base = if let Some((loc, _)) = entry.location_and_npc.split_once('；') {
        loc
    } else if let Some((loc, _)) = entry.location_and_npc.split_once('（') {
        loc
    } else if let Some((loc, _)) = entry.location_and_npc.split_once('、') {
        loc
    } else {
        entry.location_and_npc
    };
    let mut location_context = loc_base.to_string();
    if !primary_exemplar.is_empty() {
        location_context = location_context.replace(&primary_exemplar, "");
    }
    if !secondary_exemplar.is_empty() {
        location_context = location_context.replace(&secondary_exemplar, "");
    }
    for &name in EXEMPLAR_NAMES {
        location_context = location_context.replace(name, "");
    }
    location_context = location_context.trim_matches(|c: char| {
        c == '・' || c == ' ' || c == '；' || c == '、' || c == '，'
    }).to_string();

    // 動態替換函式：將主副典範人物替換為 live actor
    let bind_actors = |s: &str| -> String {
        let mut res = s.to_string();
        if !primary_exemplar.is_empty() {
            res = res.replace(&primary_exemplar, &host_actor);
        }
        if !secondary_exemplar.is_empty() {
            res = res.replace(&secondary_exemplar, &partner_actor);
        }
        for &name in EXEMPLAR_NAMES {
            if res.contains(name) {
                res = res.replace(name, &partner_actor);
            }
        }
        res = res.replace("玩家", "你");
        res
    };

    let trigger_and_causal_chain = strip_wt_codes(&bind_actors(entry.trigger_and_causal_chain));
    let player_actions = strip_wt_codes(&bind_actors(entry.player_actions));
    let outcome_and_world_change = strip_wt_codes(&bind_actors(entry.outcome_and_world_change));

    let room_tags = infer_script_room_tags(entry);

    let event_pack = EventPack {
        id: entry.id,
        pack_id: entry.id,
        status: "active",
        category_num: entry.category_num,
        category_tag: entry.category_tag,
        category_name: entry.category_name,
        room_tags,
        key_objects: entry.key_objects,
        event_title,
        trigger_and_causal_chain,
    };

    let script_pack = ScriptPack {
        id: entry.id,
        pack_id: entry.id,
        current_step: 1,
        title: entry.title.to_string(),
        host_slot_actor: host_actor,
        partner_slot_actor: partner_actor,
        location_context,
        player_actions,
        outcome_and_world_change,
    };
    (event_pack, script_pack)
}

pub fn script_category_tag_for_num(category_num: &str) -> &'static str {
    match category_num.trim() {
        "01" | "1" => "craft_supply_and_delivery",
        "02" | "2" => "neighborhood_life_and_hospitality",
        "03" | "3" => "scholarship_astrolabe_and_apprenticeship",
        "04" | "4" => "homestead_finance_and_commerce",
        "05" | "5" => "hunting_angling_camping_and_foraging",
        "06" | "6" => "seasonal_festivals_and_rites",
        _ => "general_script",
    }
}

/// 依劇本編號查詢單一劇本切面
pub fn find_script_by_id(script_id: &str) -> Option<&'static PeninsulaScriptEntry> {
    PENINSULA_DAILY_SCRIPTS.iter().find(|s| s.id.eq_ignore_ascii_case(script_id))
}

/// 依據場所名稱與時間解析當前活躍的事件包與劇本包（以四季節氣篩選並依時序輪轉）
pub fn active_event_and_script_pack_for_room(
    room_name: &str,
    host_override: Option<&str>,
    now_unix: i64,
) -> Option<(EventPack, ScriptPack)> {
    active_dyadic_event_and_script_pack_for_room(room_name, host_override, None, now_unix)
}

/// 依據場所名稱、當值主事、協力對口與時間，解析雙人同場演繹的事件包與劇本包
pub fn active_dyadic_event_and_script_pack_for_room(
    room_name: &str,
    host_override: Option<&str>,
    partner_override: Option<&str>,
    now_unix: i64,
) -> Option<(EventPack, ScriptPack)> {
    if crate::grid::grid::is_street_or_gate_room_name(room_name)
        || crate::grid::grid::is_private_residence_room_name(room_name)
    {
        return None;
    }
    let all_scripts = scripts_for_room(room_name);
    if all_scripts.is_empty() {
        return None;
    }
    let raw_days = now_unix / 86400;
    let day_in_year = raw_days.max(0) % 365;
    let cur_season = if day_in_year < 91 {
        "春融"
    } else if day_in_year < 182 {
        "夏漲"
    } else if day_in_year < 273 {
        "秋狩"
    } else {
        "冬凝"
    };
    let seasonal_scripts: Vec<&'static PeninsulaScriptEntry> = all_scripts
        .iter()
        .copied()
        .filter(|sc| {
            let combined = format!("{} {}", sc.title, sc.trigger_and_causal_chain);
            let is_winter = combined.contains("冬凝") || combined.contains("冬至") || combined.contains("守歲") || combined.contains("除夕");
            let is_summer = combined.contains("夏炎") || combined.contains("夏漲") || combined.contains("盛夏");
            let is_autumn = combined.contains("秋狩") || combined.contains("秋收") || combined.contains("仲秋");
            let is_spring = combined.contains("春融") || combined.contains("春耕");
            match cur_season {
                "春融" => !is_winter && !is_summer && !is_autumn,
                "夏漲" => !is_winter && !is_spring && !is_autumn,
                "秋狩" => !is_winter && !is_spring && !is_summer,
                "冬凝" => !is_spring && !is_summer && !is_autumn,
                _ => true,
            }
        })
        .collect();
    let scripts = if seasonal_scripts.is_empty() {
        all_scripts
    } else {
        seasonal_scripts
    };
    let idx = ((now_unix.unsigned_abs() as usize) / 60) % scripts.len();
    let sc = scripts[idx];
    Some(decompose_into_event_and_script_pack(
        sc,
        room_name,
        host_override,
        partner_override,
    ))
}

/// 依據場所名稱解析所有拆解後的事件包與劇本包清單
pub fn all_decomposed_event_and_script_packs_for_room(
    room_name: &str,
    host_override: Option<&str>,
) -> Vec<(EventPack, ScriptPack)> {
    scripts_for_room(room_name)
        .into_iter()
        .map(|sc| decompose_into_event_and_script_pack(sc, room_name, host_override, None))
        .collect()
}

/// 依據場域、操作器物與互動動作，嘗試推進當前場域活躍的事件包與劇本包（完整參數版本）
pub fn try_advance_room_event_and_script_pack_via_object_full(
    room_name: &str,
    obj_id: &str,
    obj_name: &str,
    action: &str,
    now_unix: i64,
) -> Option<(EventPack, ScriptPack, String)> {
    if !matches!(
        action,
        "Use"
            | "Operate"
            | "MillGrain"
            | "RefinePlank"
            | "CutJade"
            | "PressCheese"
            | "HaulMineCart"
            | "SmokeFish"
            | "BurnLime"
            | "AssistKitchen"
            | "RoastMountainTea"
            | "FitMortiseTenon"
            | "QuenchBlade"
            | "DyeIndigoCloth"
            | "FeedDraftBeast"
            | "CalibrateClock"
            | "SeasonalBanquet"
            | "ApplyMoxibustion"
            | "TranscribeRareTome"
            | "AttendLecture"
            | "ReadWindVane"
            | "ChartAstrolabe"
            | "RowSwampRaft"
    ) {
        return None;
    }

    let candidates = scripts_for_room(room_name);
    if candidates.is_empty() {
        return None;
    }
    let room_tags = crate::grid::derive_room_tags(crate::grid::Terrain::Road, room_name, "");
    let obj_tokens: Vec<String> = {
        let chars: Vec<char> = obj_name.chars().collect();
        if chars.len() >= 2 {
            chars.windows(2).map(|w| w.iter().collect()).collect()
        } else {
            vec![]
        }
    };

    let matched_entry = candidates
        .iter()
        .copied()
        .find(|sc| {
            (!obj_name.is_empty() && sc.key_objects.contains(obj_name))
                || (!obj_id.is_empty() && sc.key_objects.contains(obj_id))
                || obj_tokens.iter().any(|tok| sc.key_objects.contains(tok.as_str()) || sc.title.contains(tok.as_str()))
        })
        .or_else(|| {
            candidates.iter().copied().find(|sc| {
                let sc_tags = infer_script_room_tags(sc);
                room_tags.iter().any(|t| sc_tags.contains(t) || (!crate::grid::canonical_room_tag(t).is_empty() && sc_tags.iter().any(|st| crate::grid::canonical_room_tag(st) == crate::grid::canonical_room_tag(t))))
            })
        });

    let (ev, sp) = if let Some(sc) = matched_entry {
        decompose_into_event_and_script_pack(sc, room_name, None, None)
    } else {
        active_event_and_script_pack_for_room(room_name, None, now_unix)?
    };
    let first_outcome = sp
        .outcome_and_world_change
        .split(['。', '；'])
        .next()
        .unwrap_or(sp.outcome_and_world_change.as_str())
        .trim_end_matches(['。', '；']);
    let note = format!(
        "\n你上前打理「{}」，與{}、{}一同落實手頭要務——{}。",
        obj_name, sp.host_slot_actor, sp.partner_slot_actor, first_outcome
    );
    Some((ev, sp, note))
}

/// 依據場域與器物或物件名稱，嘗試推進當前場域活躍的事件包與劇本包（交易、借還與物件互動簡化入口）
pub fn try_advance_room_event_and_script_pack_via_object(
    room_name: &str,
    obj_name: &str,
) -> Option<(EventPack, ScriptPack, String)> {
    try_advance_room_event_and_script_pack_via_object_full(
        room_name,
        "",
        obj_name,
        "Operate",
        crate::game::now_unix(),
    )
}

/// 依據場所名稱與主事標記解析當前活躍的事件包與劇本包（以目前伺服器時間為準）
pub fn active_event_pack_for_room(
    room_name: &str,
    host_override: Option<&str>,
) -> Option<(EventPack, ScriptPack)> {
    active_event_and_script_pack_for_room(room_name, host_override, crate::game::now_unix())
}

/// 雙人同場演繹場域活躍劇本包（傳回 (劇本包ID, 階段步數, 演繹文本)）
pub fn co_enact_script_pack(
    room_name: &str,
    caller_a: &str,
    caller_b: &str,
    now_unix: i64,
) -> Option<(String, usize, String)> {
    let (ev, sp) = active_dyadic_event_and_script_pack_for_room(room_name, Some(caller_a), Some(caller_b), now_unix)?;
    let step = (((now_unix.unsigned_abs() as usize) / 20) % 3) + 1;
    let line = match step {
        1 => ev.trigger_and_causal_chain.to_string(),
        2 => {
            let pa = sp.player_actions.trim();
            if pa.starts_with("協助")
                || pa.starts_with("向")
                || pa.starts_with("與")
                || pa.starts_with("幫")
                || pa.starts_with("請")
                || pa.starts_with("替")
            {
                format!(
                    "{caller_a}與{caller_b}圍在{}旁協同料理「{}」，一人掌器調校、一人覆核成色，手腳俐落。",
                    ev.key_objects, sp.title
                )
            } else {
                pa.to_string()
            }
        }
        _ => sp.outcome_and_world_change.to_string(),
    };
    Some((ev.event_title, step, line))
}

