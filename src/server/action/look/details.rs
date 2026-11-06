fn character_to_store_entity_probe(target: &Character) -> crate::store::Entity {
    let mut probe = crate::store::Entity::new_npc(&target.id, &target.display_char);
    probe.display_title = target.display_title.clone();
    probe.current_activity = target.current_activity.clone();
    probe.equipment_slots = target.equipment_slots.clone();
    probe.inventory = target.inventory.clone();
    probe.grid_x = target.grid_x;
    probe.grid_y = target.grid_y;
    probe.magnesium = target.magnesium;
    probe
}


fn followup_actions(player_id: &str, target: &Character) -> Vec<String> {
    if target.id.starts_with("shanhai_")
        || (target.id.starts_with("ambient_")
            && crate::game::ambient_creature_loot(&target.display_title).is_some())
    {
        return vec!["Look".into(), "Subdue".into(), "Slay".into()];
    }
    let room_name = super::resolve_action_room_name(player_id, target);
    let named = db::get_acquaintance(player_id, &target.id)
        .ok()
        .flatten()
        .map(|a| a.named)
        .unwrap_or(false);
    if named {
        vec![
            "Talk".into(),
            "Trade".into(),
            "Borrow".into(),
            "Subdue".into(),
            "Slay".into(),
        ]
    } else if super::trade::target_has_shop_offer(target, &room_name)
        || target_is_available_market_operator(player_id, &target.id)
        || {
            let probe = character_to_store_entity_probe(target);
            let coord = (target.grid_x.unwrap_or(0), target.grid_y.unwrap_or(0));
            let (u, c) = crate::pg::living_world::is_recognizable_room_staff_or_counter_holder_entity(&probe, coord, "");
            (u || c) && crate::db::get_inventory(&target.inventory, target.vit).items.iter().any(|i| i.qty > 0)
        }
    {
        vec!["Talk".into(), "Trade".into()]
    } else {
        vec!["Talk".into()]
    }
}

fn resident_appearance_and_demeanor(target: &Character, room_name: &str) -> Option<String> {
    if crate::grid::grid::is_street_or_gate_room_name(room_name.trim())
        || crate::grid::grid::is_private_residence_room_name(room_name.trim())
    {
        return None;
    }
    let probe = character_to_store_entity_probe(target);
    let coord = (target.grid_x.unwrap_or(0), target.grid_y.unwrap_or(0));
    let (has_uniform, in_counter) =
        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder_entity(&probe, coord, "");
    let (is_staff_by_title, _, _) =
        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            room_name,
            &target.id,
            &target.display_title,
            &target.current_activity,
            &target.equipment_slots,
        );
    if !(has_uniform || in_counter || is_staff_by_title) {
        return None;
    }
    let hint = format!("{} {}", target.display_title, target.id);
    if let Some(venue) = crate::grid::resolve_room_venue_object(room_name, &hint)
        .or_else(|| crate::grid::resolve_market_or_specialty_room_object(room_name, &hint))
        .filter(|v| v.venue_kind != "街巷橋樑公共空地" && v.default_host_title != "街坊居民")
    {
        if !venue.on_duty_demeanor.is_empty() {
            return Some(venue.on_duty_demeanor.to_string());
        }
    }
    None
}

/// 市場交易是場所角色，不應要求玩家先知道經手人的私名才看得到。
/// 只有同地、受僱且此刻能從真實市場庫存提出報價的人才通過。
fn target_is_available_market_operator(player_id: &str, target_id: &str) -> bool {
    let Some(pool) = crate::pg::pool() else {
        return false;
    };
    crate::pg::world_market::offer_for_operator(
        &pool,
        target_id,
        player_id,
        crate::game::now_unix(),
    )
    .ok()
    .flatten()
    .is_some()
}

#[cfg(test)]
fn observed_label_for_room(_raw_name: &str, _room_name: &str) -> String {
    "陌生人".to_string()
}


fn scene_observation_line(room_name: &str, pronoun: &str) -> Option<String> {
    let hour = crate::store::current_game_hour();
    let is_night = !cfg!(test) && (hour >= 22 || hour < 5);
    if is_night {
        let is_watch = room_name.contains("巡防")
            || room_name.contains("哨")
            || room_name.contains("關口")
            || room_name.contains("道口")
            || room_name.contains("水口")
            || room_name.contains("渡口")
            || room_name.contains("門坊")
            || room_name.contains("旅社")
            || room_name.contains("歇舍");
        if is_watch {
            return Some(format!("{pronoun}正提著防風燈籠，在夜色中值守巡看周圍門戶與動靜。"));
        } else {
            return Some(format!("{pronoun}已在內室床鋪上熄燈安歇，四周萬籟俱寂。"));
        }
    }
    if let Some(template) = final_map_observation_template_for_room(room_name) {
        Some(template.replace("{pronoun}", pronoun))
    } else if room_name.contains("半嶺水岸渡口") || room_name.contains("水岸渡口") || room_name.contains("北岸") || room_name.contains("等候") {
        Some(format!(
            "{pronoun}沒有主動搭話，只不時望向渡船踏板，又把身邊的包袱往腳邊收了收。"
        ))
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") || room_name.contains("市集") {
        Some(format!(
            "{pronoun}在價牌前停了停，指尖掂著一只小包貨物，像是在估量要不要趁船來前換些東西。"
        ))
    } else if room_name.contains("歇腳") {
        Some(format!(
            "{pronoun}靠近長凳坐著，茶碗握在手裡，包袱收在腳邊，聽見船繩聲便抬頭看一眼。"
        ))
    } else if room_name.contains("踏板") {
        Some(format!(
            "{pronoun}一手扶著濕木扶手，等船身在水流裡穩下來，腳尖一直避著踏板邊緣的積水。"
        ))
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some(format!(
            "{pronoun}把包袱提高些，鞋底還沾著濕泥，走兩步便回頭看一眼渡船是否還靠在岸邊。"
        ))
    } else if room_name.contains("東南船塢口") {
        Some(format!(
            "{pronoun}把半修的船板靠回木架旁，手上還留著瀝青和木屑的氣味。"
        ))
    } else if room_name.contains("修船架") {
        Some(format!(
            "{pronoun}沿著船肋敲了幾下木楔，又側耳聽船身裡回出的空響。"
        ))
    } else if room_name.contains("補給倉") {
        Some(format!(
            "{pronoun}把淡水桶和乾糧箱重新分成幾堆，低聲核對哪艘船先裝。"
        ))
    } else if room_name.contains("碼頭木階") {
        Some(format!(
            "{pronoun}扶著濕木階等船身晃穩，腳邊的水痕被來往腳步踩亮。"
        ))
    } else if room_name.contains("帆布曬場") {
        Some(format!(
            "{pronoun}用膝蓋壓住被風吹起的舊帆，沿著裂口把針線慢慢收緊。"
        ))
    } else if room_name.contains("船戶街口") {
        Some(format!(
            "{pronoun}在招工牌下比了比工錢，又朝飯攤旁等活的人低聲交代船期。"
        ))
    } else if room_name.contains("外貿碼頭") {
        Some(format!(
            "{pronoun}扶住新卸下的外地貨箱，抬頭看港旗和舶司前棚那邊的手勢。"
        ))
    } else if room_name.contains("舶司前棚") {
        Some(format!(
            "{pronoun}按著濕貨單和封條，等翻譯桌那邊把貨名重新說清。"
        ))
    } else if room_name.contains("商館街口") {
        Some(format!(
            "{pronoun}把貨樣匣打開一角，低聲把新問到的價碼送往匯兌小屋。"
        ))
    } else if room_name.contains("風舶角外洋棧倉") || room_name.contains("外洋棧倉") {
        Some(format!(
            "{pronoun}把異地木箱推到雨布內側，分出已查和待查的箱包。"
        ))
    } else if room_name.contains("翻譯桌") {
        Some(format!(
            "{pronoun}指著合約上並排的兩種文字，把陌生貨名轉成在地話。"
        ))
    } else if room_name.contains("匯兌小屋") {
        Some(format!(
            "{pronoun}撥了撥銅秤，將錢票和商館街口報來的價碼寫入帳冊。"
        ))
    } else if room_name.contains("風舶角避風港") || room_name.contains("避風港") {
        Some(format!(
            "{pronoun}把結霜纜繩重新繞上木樁，又抬頭看避風船是否貼穩碼頭。"
        ))
    } else if room_name.contains("港灣貨棚") {
        Some(format!(
            "{pronoun}把濕魚籃推到鹽箱旁，鞋底在棚門下拖出一道水痕。"
        ))
    } else if room_name.contains("漁網曬場") {
        Some(format!(
            "{pronoun}沿著濕網破口慢慢穿線，膝邊的木架被海風吹得輕響。"
        ))
    } else if room_name.contains("冷霧魚市") {
        Some(format!(
            "{pronoun}掀開濕布讓買貨人看魚色，又把木秤往身邊拉近。"
        ))
    } else if room_name.contains("船戶屋排") {
        Some(format!(
            "{pronoun}在輪值板前換下一枚濕木牌，火盆煙氣貼著袖口散開。"
        ))
    } else if room_name.contains("烏石鼻守望石階") || room_name.contains("守望石階") {
        Some(format!(
            "{pronoun}扶著濕石欄看向港外海面，指尖停在警鈴繩旁。"
        ))
    } else if room_name.contains("礦坑外棚") {
        Some(format!(
            "{pronoun}把沾泥的鎬靠上木架，抬頭看坑口吊燈是否還亮著。"
        ))
    } else if room_name.contains("洗礦溝") {
        Some(format!(
            "{pronoun}把石籃壓進冷水槽，從混濁水裡分出還能送往粗料倉的礦料。"
        ))
    } else if room_name.contains("粗料倉") {
        Some(format!(
            "{pronoun}在濕礦袋旁補上一道炭線，讓坑口、重量和去向都留在木牌上。"
        ))
    } else if room_name.contains("工具修棚") {
        Some(format!(
            "{pronoun}把鬆掉的鐵圈敲回車輪邊，火星在裂柄鎬旁短短一亮。"
        ))
    } else if room_name.contains("礦渣坡") {
        Some(format!(
            "{pronoun}壓住推車把手，把廢料倒在安全線內，避開新滑坡痕。"
        ))
    } else if room_name.contains("紅土崁選礦車場") || room_name.contains("選礦車場") {
        Some(format!(
            "{pronoun}把繩索掛上礦車車架，又伸手抹開稱重木牌上的灰粉。"
        ))
    } else if room_name.contains("船戶作業點") || room_name.contains("作業") {
        Some(format!(
            "{pronoun}低頭撥著纜繩與木楔，偶爾抬眼看水勢，像是在等下一個靠船的手勢。"
        ))
    } else if room_name.contains("倉棚") {
        Some(format!(
            "{pronoun}在麻袋與木箱旁停下，手掌拍去袖口的泥點，視線一直留意著棚門和河邊的動靜。"
        ))
    } else if let Some(venue) = crate::grid::resolve_any_room_venue_object(room_name, "")
        .filter(|v| v.venue_kind != "街巷橋樑公共空地" && v.default_host_title != "街坊居民")
    {
        Some(format!("{pronoun}{}。", venue.on_duty_fixture_activity))
    } else {
        None
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    fn test_character() -> Character {
        Character {
            id: "npc_test".to_string(),
            kind: crate::entity::EntityKind::Npc,
            display_char: String::new(),
            x: 0,
            y: 0,
            move_state: crate::entity::MoveState::Idle,
            target_x: None,
            target_y: None,
            walk_or_run: crate::entity::WalkOrRun::Walk,
            move_started_at: None,
            vit: 10,
            qi: 10,
            dex: 10,
            magnesium: 0,
            last_observed_at: None,
            created_at: 0,
            gender: Some(crate::entity::Gender::M),
            soul_seed: None,
            display_title: String::new(),
            activated_nodes: "[]".to_string(),
            equipment_slots: String::new(),
            inventory: "[]".to_string(),
            disposition: 0,
            thoughts: vec![],
            current_activity: String::new(),
            grid_x: None,
            grid_y: None,
        }
    }

    #[test]
    fn look_narrative_has_no_ascii_space_after_sentence_break() {
        let mut ch = test_character();
        ch.current_activity = "整理包袱".to_string();
        let line = build_look_narrative(&ch, "候渡者", "半嶺水岸渡口", None);
        assert!(!line.contains("。 "));
        assert!(line.contains("。他目前整理包袱。"));
    }
    #[test]
    fn generated_private_name_falls_back_to_stranger_label() {
        assert_eq!(observed_label_for_room("王小明", "半嶺水岸渡口"), "陌生人");
    }

    #[test]
    fn public_role_label_is_kept() {
        assert_eq!(observed_label_for_room("船戶", "半嶺水岸渡口"), "陌生人");
    }

    #[test]
    fn market_private_name_falls_back_to_scene_label() {
        assert_eq!(observed_label_for_room("王小明", "水尾寮渡頭市集"), "陌生人");
    }

    #[test]
    fn south_bank_private_name_falls_back_to_scene_label() {
        assert_eq!(observed_label_for_room("王小明", "南岸泥坡"), "陌生人");
    }

    #[test]
    fn warehouse_private_name_falls_back_to_scene_label() {
        assert_eq!(observed_label_for_room("王小明", "倉棚前場"), "陌生人");
    }

    #[test]
    fn m3_050_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("船匠", "東南船塢口"), "陌生人");
        assert_eq!(observed_label_for_room("補帆人", "帆布曬場"), "陌生人");
        assert_eq!(observed_label_for_room("王小明", "船戶街口"), "陌生人");
    }
    #[test]
    fn boathouse_street_look_uses_street_work_not_ferry_work_point() {
        let line = scene_observation_line("船戶街口", "他").expect("船戶街口應有觀察句");

        assert!(line.contains("招工牌"));
        assert!(line.contains("船期"));
        assert!(!line.contains("木樁"));
        assert!(!line.contains("纜繩"));
    }

    #[test]
    fn m3_053_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("舶司吏", "舶司前棚"), "陌生人");
        assert_eq!(observed_label_for_room("商館人", "商館街口"), "陌生人");
        assert_eq!(observed_label_for_room("王小明", "匯兌小屋"), "陌生人");
    }
    #[test]
    fn foreign_trade_look_uses_trade_work_not_generic_passerby() {
        let customs = scene_observation_line("舶司前棚", "他").expect("舶司前棚應有觀察句");
        let exchange = scene_observation_line("匯兌小屋", "她").expect("匯兌小屋應有觀察句");

        assert!(customs.contains("貨單"));
        assert!(customs.contains("封條"));
        assert!(exchange.contains("銅秤"));
        assert!(exchange.contains("帳冊"));
        assert!(!customs.contains("包袱"));
        assert!(!exchange.contains("包袱"));
    }

    #[test]
    fn m3_006_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("搬魚人", "港灣貨棚"), "陌生人");
        assert_eq!(observed_label_for_room("補網人", "漁網曬場"), "陌生人");
        assert_eq!(observed_label_for_room("王小明", "烏石鼻守望石階"), "陌生人");
    }
    #[test]
    fn north_harbor_look_uses_harbor_work_not_generic_passerby() {
        let net = scene_observation_line("漁網曬場", "她").expect("漁網曬場應有觀察句");
        let watch = scene_observation_line("烏石鼻守望石階", "他").expect("烏石鼻守望石階應有觀察句");

        assert!(net.contains("濕網"));
        assert!(net.contains("穿線"));
        assert!(watch.contains("海面"));
        assert!(watch.contains("警鈴"));
        assert!(!net.contains("包袱"));
        assert!(!watch.contains("包袱"));
    }

    #[test]
    fn m3_009_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("礦工", "礦坑外棚"), "陌生人");
        assert_eq!(observed_label_for_room("洗礦工", "洗礦溝"), "陌生人");
        assert_eq!(observed_label_for_room("王小明", "工具修棚"), "陌生人");
        assert_eq!(observed_label_for_room("李小明", "礦渣坡"), "陌生人");
    }
    #[test]
    fn northwest_mine_look_uses_mine_work_not_generic_passerby() {
        let wash = scene_observation_line("洗礦溝", "她").expect("洗礦溝應有觀察句");
        let load = scene_observation_line("紅土崁選礦車場", "他").expect("紅土崁選礦車場應有觀察句");

        assert!(wash.contains("冷水槽"));
        assert!(wash.contains("粗料倉"));
        assert!(load.contains("礦車"));
        assert!(load.contains("稱重木牌"));
        assert!(!wash.contains("包袱"));
        assert!(!load.contains("包袱"));
    }

    #[test]
    fn final_map_public_roles_are_kept_when_looking() {
        for (raw, room, _label) in [
            ("守門人", "邊城外門", "守門人"),
            ("王小明", "糧倉前場", "倉管"),
            ("鐵匠", "鍛造棚", "鐵匠"),
            ("李小明", "牧地柵門", "牧人"),
            ("獸醫", "獸醫棚", "獸醫"),
            ("趙小明", "商隊泊位", "商隊人"),
            ("店伙", "客棧前廊", "店伙"),
            ("巡哨", "巡哨棚", "巡哨"),
        ] {
            assert_eq!(observed_label_for_room(raw, room), "陌生人", "{room}");
        }
    }
    #[test]
    fn final_map_look_uses_scene_work_not_generic_passerby() {
        let border = scene_observation_line("邊城外門", "他").expect("邊城外門應有觀察句");
        let forge = scene_observation_line("鍛造棚", "她").expect("鍛造棚應有觀察句");
        let pasture = scene_observation_line("牧地柵門", "他").expect("牧地柵門應有觀察句");
        let town = scene_observation_line("邊鎮井口", "她").expect("邊鎮井口應有觀察句");

        assert!(border.contains("通行牌"));
        assert!(forge.contains("鐵胚"));
        assert!(pasture.contains("畜牌"));
        assert!(town.contains("井繩"));
        assert!(!border.contains("包袱"));
        assert!(!forge.contains("包袱"));
        assert!(!pasture.contains("包袱"));
        assert!(!town.contains("包袱"));
    }

    #[test]
    fn no_double_outfit_conflict() {
        let mut ch = test_character();
        ch.id = "wt_npc_chen_wanfu".to_string();
        ch.display_title = "三水食堂當家大廚|陳萬福".to_string();
        ch.equipment_slots = r#"{"body":"starter_body_m","legs":"starter_legs_m","feet":"starter_feet_m"}"#.to_string();
        let line = build_look_narrative(&ch, "陳萬福", "文曜・三水口・三水食堂", None);
        assert!(line.contains("圍裙") || line.contains("大廚") || line.contains("主廚"));
        assert!(!line.contains("身上穿戴著"));
        assert!(!line.contains("粗布短褂"));
    }

    #[test]
    fn look_shows_equipped_tools_even_with_custom_outfit() {
        let mut ch = test_character();
        ch.id = "wt_npc_chen_wanfu".to_string();
        ch.display_title = "三水食堂當家大廚|陳萬福".to_string();
        ch.equipment_slots = r#"{"body":"starter_body_m","hold_r":"craft_steel_hunting_knife"}"#.to_string();
        let line = build_look_narrative(&ch, "陳萬福", "文曜・三水口・三水食堂", None);
        assert!(line.contains("手裡握著【防身獵刀"));
    }

    #[test]
    fn test_look_favorability_rel_phrase() {
        let ch = test_character();
        let line = build_look_narrative(&ch, "某甲", "客棧前廊", Some("player_test_viewer"));
        // Without store/pg memory, favorability defaults to 0 -> no special rel phrase
        assert!(!line.contains("看向你時神色帶著明顯戒備與舊怨"));
    }

    #[test]
    fn look_narrative_includes_breath_habit_vessel_and_affliction_cleanly() {
        let mut ch = test_character();
        ch.id = "npc_look_test_living".to_string();
        ch.display_title = "西市打鐵鋪匠師|鐵掌老張".to_string();
        ch.soul_seed = Some(888);
        ch.equipment_slots = r#"{"hold_r":"craft_steel_hunting_knife"}"#.to_string();

        // 植入創傷暗疾
        crate::pg::living_world::record_entity_affliction(
            &ch.id,
            crate::pg::living_world::AfflictionKind::ArmSprain,
            "forge_work",
            crate::game::now_unix(),
        );

        let narrative = build_look_narrative(&ch, "鐵掌老張", "文曜・白塔西市", None);

        // 1. 內息底色不應出現在外部打量中
        assert!(!narrative.contains("內息底色"));
        assert!(!narrative.contains("思忖著"));

        // 2. 外部觀察器物（非自身觀測時，Sharp 狀態保持乾淨器名，無內部熟練度標籤）
        assert!(narrative.contains("手裡握著【防身獵刀】"));
        assert!(!narrative.contains("如臂使指"));
        assert!(!narrative.contains("漸趨順手"));
        assert!(!narrative.contains("沉穩趁手"));

        // 3. 創傷觀測
        assert!(narrative.contains("似帶有腕肘挫傷之態。"), "narrative was: {narrative}");
        // 4. 嚴格檢查無內部 ID 洩漏與違禁詞
        assert!(!narrative.contains("npc_"));
        assert!(!narrative.contains("wt_"));
        assert!(!narrative.contains("mat_"));
        assert!(!narrative.contains("craft_"));
        assert!(!narrative.contains("database"));
        assert!(!narrative.contains("資料庫"));
        assert!(!narrative.contains("還有"));
        assert!(!narrative.contains(concat!("\u{525b}", "\u{624d}")));
        assert!(!narrative.contains(concat!("\u{525b}", "\u{525b}")));
        assert!(!narrative.contains(concat!("\u{65b9}", "\u{624d}")));
        assert!(!narrative.contains("頃刻"));
        assert!(!narrative.contains("110V"));
        assert!(!narrative.contains("念場"));
        assert!(!narrative.contains("聖塔"));
        assert!(!narrative.contains("小市"));
        assert!(!narrative.contains("短市"));
        assert!(!narrative.contains("青籠關"));

        // 清理創傷
        crate::pg::living_world::clear_entity_afflictions(&ch.id, true, crate::game::now_unix());
        crate::store::clear_active_npc_obligation(&ch.id, "treat_injury");
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut conn) = pool.get()
        {
            let _ = conn.execute(
                "DELETE FROM resident_obligations WHERE entity_id = 'npc_look_test_living'",
                &[],
            );
        }
    }

    #[test]
    fn test_look_qin_shuisheng_real_time_inscriptions_and_state() {
        let now = 1_800_000_000i64;
        crate::pg::living_world::seed_flagship_specimen_qin_shuisheng_once(now);
        let mut qin = test_character();
        qin.id = "秦水生".to_string();
        qin.display_title = "老堰頭水輪石磨首座|秦水生".to_string();
        qin.soul_seed = Some(4_088_606_620_229_995_941);
        qin.equipment_slots = r#"{"head":"wt_cat_0875","body":"wt_cat_0869","legs":"starter_legs_m","feet":"starter_feet_m","wrist":"wt_cat_0852","waist":"wt_cat_0897","hold_r":"wt_cat_0984","hold_l":"wt_cat_0973"}"#.to_string();
        qin.inventory = r#"[{"item_id":"wt_cat_0388","qty":5},{"item_id":"wt_cat_0279","qty":7},{"item_id":"wt-good-009","qty":9}]"#.to_string();
        qin.current_activity = "背負薄層重礦烏木膽溫存桶沿山道前往文曜三水大食堂送交早市米漿".to_string();

        let narrative = build_look_narrative(&qin, "秦水生", "老堰頭・山道", Some("葉卅"));
        assert!(
            narrative.contains("羊角鐵錘打石手鏨 [巧][堅]"),
            "expected inscribed chisel in look narrative: {narrative}"
        );
        assert!(
            narrative.contains("平底石磨分粉篩架 [純][流]"),
            "expected inscribed sieve in look narrative: {narrative}"
        );
        assert!(
            narrative.contains("潮汐綿長息"),
            "expected breath habit in look narrative: {narrative}"
        );
        assert!(
            narrative.contains("老堰頭水輪分動與雙疊石磨聽音辨距法"),
            "expected lineage mastery in look narrative: {narrative}"
        );
        assert!(
            narrative.contains("現磨溫米漿 5 桶") && narrative.contains("特級在來米粉 7 袋") && narrative.contains("待磨精碾白米 9 袋"),
            "expected real-time inventory counts in look narrative: {narrative}"
        );
        assert!(
            narrative.contains("背負薄層重礦烏木膽溫存桶沿山道前往文曜三水大食堂送交早市米漿"),
            "expected specific transit activity preserved in look narrative: {narrative}"
        );
    }

    #[test]
    fn test_look_universal_living_entity_details_on_offline_player_and_npc() {
        let mut player = test_character();
        player.id = "player_offline_test_look".to_string();
        player.kind = crate::entity::EntityKind::Player;
        player.inventory = r#"[{"item_id":"craft_wooden_tool_handle","qty":2},{"item_id":"wt-good-012","qty":3}]"#.to_string();
        player.current_activity = "沿著石板步道漫步穿行前往工坊".to_string();

        let (_rec, _) = crate::pg::living_world::record_mentor_imprint_progress(
            &player.id,
            "溫掌事",
            "百工調息衡準訣",
            "神思澄明",
            crate::game::now_unix(),
        );

        let narrative = build_look_narrative(&player, "某玩家", "工坊前廊", Some("viewer_p"));
        assert!(narrative.contains("身負「百工調息衡準訣」師承造詣。"), "expected lineage: {narrative}");
        assert!(narrative.contains("隨身攜有"), "expected carried items: {narrative}");
        assert!(narrative.contains("平日多留心於「"), "expected in-world offline directive posture: {narrative}");
        assert!(narrative.contains("沿著石板步道漫步穿行前往工坊"), "expected generalized transit preserved: {narrative}");
    }
    #[test]
    fn test_look_and_followup_actions_recognize_uniform_and_plain_clothes_counter_manager() {
        let viewer_id = "player_viewer_test";

        // 1. 穿便服但站在非夥計禁止入內櫃檯區的掌事（有貨物，但無制服、未結識為陌生人）
        let mut manager = test_character();
        manager.id = "npc_counter_manager_plain".to_string();
        manager.display_title = "穿便服但站在非夥計禁止入內櫃檯區的掌事".to_string();
        manager.grid_x = Some(2);
        manager.grid_y = Some(0);
        manager.inventory = r#"[{"item_id":"wt-good-009","qty":5}]"#.to_string();

        let manager_narrative = build_look_narrative(&manager, "陌生人", "沙鹿商鋪", Some(viewer_id));
        assert!(
            manager_narrative.contains("雖著尋常便服，卻能自由出入非外客可入之櫃檯與內場禁區，顯係此處主事之人。"),
            "expected plain-clothes counter manager identity in narrative: {manager_narrative}"
        );
        let manager_actions = followup_actions(viewer_id, &manager);
        assert!(
            manager_actions.contains(&"Trade".to_string()),
            "unacquainted counter manager with stock MUST expose Trade socket in followup_actions: {manager_actions:?}"
        );

        // 2. 同一房間內的路人顧客（無制服、非櫃檯內場、未結識為陌生人）
        let mut passerby = test_character();
        passerby.id = "npc_passerby_customer".to_string();
        passerby.display_title = "店內喝茶閒聊的茶客顧客".to_string();
        passerby.grid_x = Some(2);
        passerby.grid_y = Some(0);
        passerby.inventory = r#"[{"item_id":"wt-good-009","qty":5}]"#.to_string();

        let passerby_narrative = build_look_narrative(&passerby, "陌生人", "沙鹿商鋪", Some(viewer_id));
        assert!(
            !passerby_narrative.contains("櫃檯與內場禁區"),
            "passerby customer narrative must NOT contain counter zone text: {passerby_narrative}"
        );
        let passerby_actions = followup_actions(viewer_id, &passerby);
        assert_eq!(
            passerby_actions,
            vec!["Talk".to_string()],
            "passerby customer followup_actions must only be Talk: {passerby_actions:?}"
        );
    }

    #[test]
    fn test_look_any_resident_shows_inscribed_vocational_vessel_and_event_pack_duty() {
        let now = 1_800_000_000i64;
        crate::pg::living_world::seed_flagship_specimen_qin_shuisheng_once(now);

        let mut resident = test_character();
        resident.id = "秦水生".to_string();
        resident.kind = crate::entity::EntityKind::Npc;
        resident.display_title = "老堰頭水輪石磨首座|秦水生".to_string();
        resident.soul_seed = Some(4_088_606_620_229_995_941);
        resident.equipment_slots = r#"{"head":"wt_cat_0875","body":"wt_cat_0840","legs":"starter_legs_m","feet":"starter_feet_m","waist":"wt_cat_0897"}"#.to_string();
        resident.current_activity = "於水輪石磨工坊料理店務".to_string();

        let room_name = "老堰頭・水輪石磨工坊";
        let narrative = build_look_narrative(&resident, "秦水生", room_name, Some("viewer_p"));

        assert!(
            narrative.contains("平底石磨分粉篩架 [純][流]"),
            "expected inscribed vocational vessel for resident without held tools: {narrative}"
        );
        assert!(
            narrative.contains("手邊常備本職器物【平底石磨分粉篩架 [純][流]】"),
            "expected vocational vessel format in narrative: {narrative}"
        );
        assert!(
            narrative.contains("此刻正當值料理「") && !narrative.contains("事件包") && !narrative.contains("劇本包"),
            "expected active duty title and primary object in diegetic narrative: {narrative}"
        );

        let mut staff = test_character();
        staff.id = "npc_staff_floor_worker".to_string();
        staff.kind = crate::entity::EntityKind::Npc;
        staff.display_title = "石磨研磨學徒".to_string();
        staff.equipment_slots = r#"{"body":"wt_cat_0840"}"#.to_string();
        staff.current_activity = "在場中搬運麻袋".to_string();
        let floor_narrative = build_look_narrative(&staff, "學徒", room_name, Some("viewer_p"));
        assert!(
            floor_narrative.contains("身著明顯的當值工作制服，正於場內料理店務與百工差事。"),
            "expected floor uniform line: {floor_narrative}"
        );
        assert!(
            floor_narrative.contains("此刻正當值料理「") && !floor_narrative.contains("事件包") && !floor_narrative.contains("劇本包"),
            "floor staff with uniform must also show clean duty: {floor_narrative}"
        );
        assert!(
            floor_narrative.contains("手邊常備本職器物【百工學徒手用小木尺】"),
            "apprentice should show vocational vessel: {floor_narrative}"
        );

        let mut visitor = test_character();
        visitor.id = "npc_visiting_hunter".to_string();
        visitor.kind = crate::entity::EntityKind::Npc;
        visitor.display_title = "合社獵手・林山海".to_string();
        visitor.equipment_slots = r#"{"body":"starter_body_m"}"#.to_string();
        visitor.current_activity = "入內歇腳避風".to_string();
        let visitor_narrative = build_look_narrative(&visitor, "林山海", "文曜・百草診所", Some("viewer_p"));
        assert!(
            !visitor_narrative.contains("百草溫灸盒") && !visitor_narrative.contains("分格藥囊"),
            "visiting hunter in clinic must NOT be described with the clinic doctor's on_duty_demeanor: {visitor_narrative}"
        );
    }
}
