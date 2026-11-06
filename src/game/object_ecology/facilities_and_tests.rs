/// 山野草藥多插座精煉派發：支援 WashHerb, DryHerb, CrushHerb, BoilDecoction, SetupHerbStove
pub fn refine_herb_multi_socket(
    pid: &str,
    mode: &str,
    room: &str,
    now: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    let inv = db::get_inventory(&ent.inventory, ent.vit);
    let consume_herb = |candidates: &[&str]| -> bool {
        for &c in candidates {
            if db::inventory_has_item(&ent.inventory, c) {
                let _ = db::remove_from_inventory(pid, c, 1);
                return true;
            }
        }
        if let Some(it) = inv.items.iter().find(|it| is_herb_item(&it.item_id)) {
            let _ = db::remove_from_inventory(pid, &it.item_id, 1);
            return true;
        }
        false
    };

    match mode {
        "WashHerb" => {
            if !consume_herb(&["mat_foraged_herb", "harvest_herb", "mat_wet_grass_bundle", "mat_water_moss_sheet"]) {
                return Err("行囊中尚無可供濯洗的山野草藥。".to_string());
            }
            let prod_id = "mat_foraged_herb".to_string();
            let prod_name = "山野鮮藥草".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_herb", &prod_name);
            Ok((prod_id, prod_name, "你引清泉細心濯洗藥草根葉，洗淨泥沙雜屑，鮮嫩莖葉翠綠如洗、散發清幽草木芬芳。".to_string()))
        }
        "DryHerb" => {
            if !consume_herb(&["mat_foraged_herb", "harvest_herb", "mat_wet_grass_bundle"]) {
                return Err("行囊中尚無可供烘焙陰乾的藥草。".to_string());
            }
            let prod_id = "craft_calming_tea_pack".to_string();
            let prod_name = "清心安神百草茶包".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_herb", &prod_name);
            Ok((prod_id, prod_name, "你將洗淨的藥草薄攤在竹簾上慢火烘焙陰乾，鎖住草木菁華，揉捻分裝成一包芳香澄澈的【清心安神百草茶包】收入行囊。".to_string()))
        }
        "CrushHerb" => {
            if !consume_herb(&["mat_foraged_herb", "harvest_herb", "craft_calming_tea_pack"]) {
                return Err("行囊中尚無可供研磨的乾燥草藥。".to_string());
            }
            let prod_id = "mat_foraged_herb".to_string();
            let prod_name = "百草細研藥末".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_herb", &prod_name);
            Ok((prod_id, prod_name, "你取出石臼研杵，將乾燥草藥細細研磨成細膩墨綠粉末，藥香濃郁撲鼻，適宜調和脂膏敷裹。".to_string()))
        }
        "BoilDecoction" => {
            if !consume_herb(&["mat_foraged_herb", "harvest_herb", "mat_sealing_pitch", "craft_calming_tea_pack"]) {
                return Err("行囊中尚無可供煎熬收膏的藥草料件。".to_string());
            }
            let prod_id = "craft_meridian_salve".to_string();
            let prod_name = "溫脈舒筋活血膏".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_herb", &prod_name);
            Ok((prod_id, prod_name, "你將研細的草藥粉末配以松脂文火慢熬收膏，藥汁凝為琥珀色溫潤脂膏，製成一盒【溫脈舒筋活血膏】收入行囊。".to_string()))
        }
        "SetupHerbStove" => {
            let norm_room = normalize_room_key(room);
            let state = FieldFacilityState {
                facility_id: "dyn_herb_stove".to_string(),
                name: "野地煎藥陶銚".to_string(),
                created_at: now,
                expires_at: now + 86400,
            };
            {
                let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
                ledger.active_field_facilities.entry(norm_room.clone()).or_default().push(state.clone());
                if norm_room != room {
                    ledger.active_field_facilities.entry(room.to_string()).or_default().push(state);
                }
                let rname = db::get_room_name(room)
                    .ok()
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| "郊野地界".to_string());
                let ev = format!("【{rname}】架起了一具溫火慢煨的【野地煎藥陶銚】，徐徐散發著舒心藥香。");
                if !ledger.recent_events.iter().any(|e| e == &ev) {
                    ledger.recent_events.push_back(ev);
                    if ledger.recent_events.len() > 30 {
                        ledger.recent_events.pop_front();
                    }
                }
            }
            db::log_npc_event(now, pid, "setup_herb_stove", room);
            Ok(("dyn_herb_stove".to_string(), "野地煎藥陶銚".to_string(), "你於避風處支起三足泥爐，架上帶蓋陶銚並注滿山泉活水，生起微火，架設好一具隨時可煎煮藥茶膏湯的【野地煎藥陶銚】！".to_string()))
        }
        _ => Err(format!("未知的草藥精煉模式：{mode}")),
    }
}

/// 纖維麻繩多插座精煉派發：支援 TwistRope, WeaveSackcloth, OilCanvas, PressPaper
pub fn refine_fiber_multi_socket(
    pid: &str,
    mode: &str,
    _room: &str,
    now: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    let inv = db::get_inventory(&ent.inventory, ent.vit);
    let consume_fiber = |candidates: &[&str]| -> bool {
        for &c in candidates {
            if db::inventory_has_item(&ent.inventory, c) {
                let _ = db::remove_from_inventory(pid, c, 1);
                return true;
            }
        }
        if let Some(it) = inv.items.iter().find(|it| is_fiber_item(&it.item_id)) {
            let _ = db::remove_from_inventory(pid, &it.item_id, 1);
            return true;
        }
        false
    };

    match mode {
        "TwistRope" => {
            if !consume_fiber(&["mat_reed_bundle", "mat_sackcloth_bolt", "mat_fishing_line_bundle"]) {
                return Err("行囊中尚無可供搓捻的草麻纖維。".to_string());
            }
            let prod_id = "mat_hemp_rope_roll".to_string();
            let prod_name = "浸油粗麻繩卷".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_fiber", &prod_name);
            Ok((prod_id, prod_name, "你手掌合搓順紋牽引，將柔韌草麻纖維緊密絞成股線，盤捲成一卷結實牢靠的【浸油粗麻繩卷】收入行囊。".to_string()))
        }
        "WeaveSackcloth" => {
            if !consume_fiber(&["mat_hemp_rope_roll", "mat_reed_bundle"]) {
                return Err("行囊中尚無可供編織的麻繩或蘆葦纖維。".to_string());
            }
            let prod_id = "mat_sackcloth_bolt".to_string();
            let prod_name = "粗織麻袋布匹".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_fiber", &prod_name);
            Ok((prod_id, prod_name, "你以經緯穿梭之法牽引麻絲密密交織，編排平整，織成一匹厚密結實的【粗織麻袋布匹】收入行囊。".to_string()))
        }
        "OilCanvas" => {
            if !consume_fiber(&["mat_sackcloth_bolt", "mat_canvas_roll", "mat_oilcloth_roll"]) {
                return Err("行囊中尚無可供浸油烘製的粗麻帆布。".to_string());
            }
            let prod_id = "craft_storm_canvas_cloak".to_string();
            let prod_name = "防風浸油帆布行旅斗篷".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_fiber", &prod_name);
            Ok((prod_id, prod_name, "你將密織帆布均勻塗刷熟桐油與天然膠脂，文火烘透，製成一領抗寒防浪的【防風浸油帆布行旅斗篷】收入行囊。".to_string()))
        }
        "PressPaper" => {
            if !consume_fiber(&["mat_reed_bundle", "mat_coarse_paper_ream", "mat_wrapping_paper_pack"]) {
                return Err("行囊中尚無可供抄紙壓製的植物纖維。".to_string());
            }
            let prod_id = "craft_peninsula_almanac".to_string();
            let prod_name = "白塔風物考工手札".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_fiber", &prod_name);
            Ok((prod_id, prod_name, "你將紙漿均勻抄撈濾水、覆簾壓平焙乾，裝訂為一部頁面勻整吸墨的【白塔風物考工手札】收入行囊。".to_string()))
        }
        _ => Err(format!("未知的纖維精煉模式：{mode}")),
    }
}

/// 生皮水產多插座精煉派發：支援 ScrapeHide, BoilFishGlue, CalcifyShell, CobbleBoots
pub fn refine_hide_fish_multi_socket(
    pid: &str,
    mode: &str,
    _room: &str,
    now: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };
    let inv = db::get_inventory(&ent.inventory, ent.vit);
    let consume_hide_fish = |candidates: &[&str]| -> bool {
        for &c in candidates {
            if db::inventory_has_item(&ent.inventory, c) {
                let _ = db::remove_from_inventory(pid, c, 1);
                return true;
            }
        }
        if let Some(it) = inv.items.iter().find(|it| is_hide_fish_item(&it.item_id)) {
            let _ = db::remove_from_inventory(pid, &it.item_id, 1);
            return true;
        }
        false
    };

    match mode {
        "ScrapeHide" => {
            if !consume_hide_fish(&["mat_leather_strip", "mat_mane_hair_bundle"]) {
                return Err("行囊中尚無可供刮脂鞣製的生皮獸毛。".to_string());
            }
            let prod_id = "mat_leather_strip".to_string();
            let prod_name = "修整牛皮帶料".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 2);
            db::log_npc_event(now, pid, "refine_hide_fish", &prod_name);
            Ok((prod_id, prod_name, "你將生皮平鋪板上，以弧刃刮刀細細削去殘留油脂與雜質，反覆揉搓軟化，鞣製成兩條柔軟堅韌的【修整牛皮帶料】收入行囊。".to_string()))
        }
        "BoilFishGlue" => {
            if !consume_hide_fish(&["mat_fish_bone_sack", "mat_fish_scale_sack", "mat_sealing_pitch"]) {
                return Err("行囊中尚無可供熬膠的魚骨魚鱗。".to_string());
            }
            let prod_id = "mat_sealing_pitch".to_string();
            let prod_name = "熬煉濃魚膠盒".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_hide_fish", &prod_name);
            Ok((prod_id, prod_name, "你將鮮魚皮骨下鍋文火慢熬，濾去骨渣濃縮收汁，凝煉出一盒黏合性極高、遇熱化開的【熬煉濃魚膠盒】收入行囊。".to_string()))
        }
        "CalcifyShell" => {
            if !consume_hide_fish(&["mat_shell_sack", "mat_stone_powder_sack"]) {
                return Err("行囊中尚無可供煅燒的海貝殼料。".to_string());
            }
            let prod_id = "mat_stone_powder_sack".to_string();
            let prod_name = "煅燒貝灰粉袋".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_hide_fish", &prod_name);
            Ok((prod_id, prod_name, "你將厚實海貝投入熾熱火堆高溫煅燒，出爐後研碎成白淨細膩的【煅燒貝灰粉袋】，防潮密封極佳，收入行囊。".to_string()))
        }
        "CobbleBoots" => {
            if !consume_hide_fish(&["mat_leather_strip", "craft_heavy_leather_boots"]) {
                return Err("行囊中尚無可供縫製行旅靴的熟牛皮帶。".to_string());
            }
            let prod_id = "craft_heavy_leather_boots".to_string();
            let prod_name = "厚底防滑行旅皮靴".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_hide_fish", &prod_name);
            Ok((prod_id, prod_name, "你以粗麻蠟線手工縫合熟牛皮與硬木齒底，靴幫貼合牢靠，縫製出一雙【厚底防滑行旅皮靴】收入行囊。".to_string()))
        }
        _ => Err(format!("未知的皮魚精煉模式：{mode}")),
    }
}

/// 獲取房間內當前存續的動態野地設施。
pub fn active_field_facility_objects(room_id_or_name: &str, now: i64) -> Vec<crate::model::RoomObject> {
    let mut objs = Vec::new();
    let norm_room = normalize_room_key(room_id_or_name);
    let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());

    let facilities = ledger.active_field_facilities.get(&norm_room)
        .or_else(|| ledger.active_field_facilities.get(room_id_or_name));

    if let Some(list) = facilities {
        for f in list {
            if f.expires_at > now {
                if f.facility_id == "dyn_field_anvil" {
                    let mut responses = HashMap::new();
                    responses.insert("Look".to_string(), "一座穩穩楔入泥地中的平整玄武岩石砧，石面被鐵錘敲擊得光潔如鑑，周圍散落著少許鍛屑，可供往來行者修具磨刃。".to_string());
                    responses.insert("ForgeRepairStrip".to_string(), "你將鐵料放置於石砧上，手起錘落叮噹作響，反覆鍛打修整出平整的熟鐵薄片。".to_string());
                    responses.insert("SharpenBlade".to_string(), "你取過油石在石砧邊緣浸水細磨，金鐵交鳴聲中，刃面浮現冷冽寒芒。".to_string());
                    objs.push(crate::model::RoomObject {
                        id: "dyn_field_anvil".to_string(),
                        name: "野地修具石砧".to_string(),
                        owner: String::new(),
                        sockets: vec!["Look".to_string(), "ForgeRepairStrip".to_string(), "SharpenBlade".to_string()],
                        responses,
                        move_to_room_id: String::new(),
                    });
                } else if f.facility_id == "dyn_herb_stove" {
                    let mut responses = HashMap::new();
                    responses.insert("Look".to_string(), "一座以三枚耐火石塊架起的陶質煎藥銚，下襯暗紅微炭，壺嘴徐徐飄散著提神醒腦的草木清香。".to_string());
                    responses.insert("BoilDecoction".to_string(), "你往陶銚中注入山泉並投入藥草末，蓋上陶蓋文火慢煎，濃郁藥香隨蒸氣繚繞飄散。".to_string());
                    responses.insert("DryHerb".to_string(), "你利用陶銚周圍溫熱的石面烘焙藥草，葉片水分迅速蒸散、藥性得以完整鎖存。".to_string());
                    responses.insert("CrushHerb".to_string(), "你借用銚旁的小石臼細心研磨草藥，藥草在清脆的研擊聲中化為均勻細粉。".to_string());
                    objs.push(crate::model::RoomObject {
                        id: "dyn_herb_stove".to_string(),
                        name: "野地煎藥陶銚".to_string(),
                        owner: String::new(),
                        sockets: vec!["Look".to_string(), "BoilDecoction".to_string(), "DryHerb".to_string(), "CrushHerb".to_string()],
                        responses,
                        move_to_room_id: String::new(),
                    });
                }
            }
        }
    }
    objs
}

/// 與野地動態設施互動。
pub fn interact_with_field_facility(
    pid: &str,
    room: &str,
    facility_id: &str,
    action: &str,
    now: i64,
) -> Result<String, String> {
    match facility_id {
        "dyn_field_anvil" => match action {
            "Look" => Ok("這是一座質地密實、表面平整的玄武岩野地石砧，擺放穩妥，可供過路行旅現場鍛整修補鐵條或磨礪武器鋒刃。".to_string()),
            "ForgeRepairStrip" => refine_mineral_multi_socket(pid, "ForgeRepairStrip", room, now).map(|r| r.2),
            "SharpenBlade" => refine_mineral_multi_socket(pid, "SharpenBlade", room, now).map(|r| r.2),
            _ => Err(format!("野地修具石砧不支援操作：{action}")),
        },
        "dyn_herb_stove" => match action {
            "Look" => Ok("這是一具野地架設的煎藥陶銚，微火溫煨，銚口熱氣裊裊，能供行者煎熬藥膏、烘焙藥茶或研碎藥草。".to_string()),
            "BoilDecoction" => refine_herb_multi_socket(pid, "BoilDecoction", room, now).map(|r| r.2),
            "DryHerb" => refine_herb_multi_socket(pid, "DryHerb", room, now).map(|r| r.2),
            "CrushHerb" => refine_herb_multi_socket(pid, "CrushHerb", room, now).map(|r| r.2),
            _ => Err(format!("野地煎藥陶銚不支援操作：{action}")),
        },
        _ => Err(format!("未知的野地設施：{facility_id}")),
    }
}

/// 依物品 ID 查詢其可在行囊中觸發的動作插座按鈕列表。
pub fn inventory_item_actions(item_id: &str) -> Vec<String> {
    if is_mineral_item(item_id) {
        vec![
            "WashOre".into(),
            "SmeltBloom".into(),
            "ForgeRepairStrip".into(),
            "SharpenBlade".into(),
            "BuildFieldAnvil".into(),
        ]
    } else if is_herb_item(item_id) {
        vec![
            "WashHerb".into(),
            "DryHerb".into(),
            "CrushHerb".into(),
            "BoilDecoction".into(),
            "SetupHerbStove".into(),
        ]
    } else if is_fiber_item(item_id) {
        vec![
            "TwistRope".into(),
            "WeaveSackcloth".into(),
            "OilCanvas".into(),
            "PressPaper".into(),
        ]
    } else if is_hide_fish_item(item_id) {
        vec![
            "ScrapeHide".into(),
            "BoilFishGlue".into(),
            "CalcifyShell".into(),
            "CobbleBoots".into(),
        ]
    } else if is_wood_or_firewood_item(item_id) {
        vec![
            "MakeFire".into(),
            "SplitWood".into(),
            "RefinePlank".into(),
            "BurnCharcoal".into(),
            "CraftTorch".into(),
            "AddFuel".into(),
            "RepairStructure".into(),
        ]
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_base_materials_and_finished_goods_have_canonical_white_tower_origins() {
        for item in crate::world::materials::material_item_defs() {
            assert!(
                !item.origin.is_empty(),
                "Material {} missing origin",
                item.id
            );
            let settlement = crate::world::materials::infer_origin_settlement(&item.origin);
            assert!(
                !settlement.is_empty(),
                "Material {} origin '{}' did not resolve to a canonical White Tower settlement",
                item.id,
                item.origin
            );
            for legacy in ["中鎮", "北岸渡口", "邊市口", "岸邊短市", "短市", "小市", "聖塔"] {
                assert!(
                    !item.origin.contains(legacy) && !item.description.contains(legacy),
                    "Material {} still contains legacy term '{}': origin={}, desc={}",
                    item.id,
                    legacy,
                    item.origin,
                    item.description
                );
            }
        }

        for fg in &STANDARD_FINISHED_GOODS {
            assert!(!fg.origin.is_empty(), "Finished good {} missing origin", fg.id);
            let settlement = crate::world::materials::infer_origin_settlement(fg.origin);
            assert!(
                !settlement.is_empty(),
                "Finished good {} origin '{}' did not resolve to a canonical White Tower settlement",
                fg.id,
                fg.origin
            );
        }
    }

    #[test]
    fn all_five_material_families_expose_tier1_multi_sockets() {
        let wood_actions = inventory_item_actions("mat_timber_log");
        assert!(wood_actions.contains(&"SplitWood".to_string()));
        assert!(wood_actions.contains(&"RefinePlank".to_string()));

        let mineral_actions = inventory_item_actions("mat_rough_ore");
        assert!(mineral_actions.contains(&"WashOre".to_string()));
        assert!(mineral_actions.contains(&"SmeltBloom".to_string()));
        assert!(mineral_actions.contains(&"BuildFieldAnvil".to_string()));

        let herb_actions = inventory_item_actions("mat_foraged_herb");
        assert!(herb_actions.contains(&"WashHerb".to_string()));
        assert!(herb_actions.contains(&"BoilDecoction".to_string()));
        assert!(herb_actions.contains(&"SetupHerbStove".to_string()));

        let fiber_actions = inventory_item_actions("mat_reed_bundle");
        assert!(fiber_actions.contains(&"TwistRope".to_string()));
        assert!(fiber_actions.contains(&"WeaveSackcloth".to_string()));
        assert!(fiber_actions.contains(&"PressPaper".to_string()));

        let hide_fish_actions = inventory_item_actions("mat_fish_scale_sack");
        assert!(hide_fish_actions.contains(&"BoilFishGlue".to_string()));
        assert!(hide_fish_actions.contains(&"CobbleBoots".to_string()));
    }

    #[test]
    fn free_economy_emergent_settlement_pricing_and_saturation_works() {
        // 1) 產地市集 vs 緊缺消費地市集：無硬編碼百分比，純依庫存供需自生浮動價格
        let hongtu_quote = dynamic_settlement_market_quote("紅土崁", "mat_rough_ore", 10);
        let wenyao_quote = dynamic_settlement_market_quote("文曜", "mat_rough_ore", 10);
        assert!(
            wenyao_quote.unit_price > hongtu_quote.unit_price,
            "文曜消費地價格 ({}) 應高於產地紅土崁價格 ({})",
            wenyao_quote.unit_price,
            hongtu_quote.unit_price
        );
        assert!(
            hongtu_quote.local_stock > hongtu_quote.target_stock,
            "紅土崁為產地，現貨應大於基準目標"
        );
        assert!(
            wenyao_quote.local_stock < wenyao_quote.target_stock,
            "文曜為緊缺地，現貨應小於基準目標"
        );

        // 2) 持續向文曜市集注水售料：庫存遞增，價格單調下降，直至市場飽和價格低於產地
        let mut last_price = wenyao_quote.unit_price;
        let mut prev_stock = wenyao_quote.local_stock;
        for _ in 0..20 {
            let after = record_settlement_market_trade("文曜", "mat_rough_ore", 10, 1, "玩家售出");
            assert!(
                after.local_stock > prev_stock,
                "持續售料應使本地庫存實質增加"
            );
            prev_stock = after.local_stock;
            assert!(
                after.unit_price <= last_price,
                "庫存增加應使市集收購行情單調遞減或持平：當前 {} > 前次 {}",
                after.unit_price,
                last_price
            );
            last_price = after.unit_price;
        }
        assert!(
            last_price < hongtu_quote.unit_price,
            "大量注水後文曜市集價格 ({}) 應被壓低至低於產地原始價格 ({})",
            last_price,
            hongtu_quote.unit_price
        );

        // 3) 從霜林口採買提料：庫存減少，市場稀缺度攀升，單價相應上揚
        let before_buy = dynamic_settlement_market_quote("霜林口", "mat_plank_board", 10);
        let after_buy = record_settlement_market_trade("霜林口", "mat_plank_board", 10, -5, "玩家採買");
        assert!(
            after_buy.local_stock < before_buy.local_stock,
            "採買應實質消耗本地庫存"
        );
        assert!(
            after_buy.unit_price > before_buy.unit_price,
            "物資消耗抽空後價格應上漲：採買後 {} 應大於採買前 {}",
            after_buy.unit_price,
            before_buy.unit_price
        );
    }

    #[test]
    fn workshop_halts_when_raw_material_stock_is_zero_and_all_1120_catalog_items_registered() {
        if crate::store::get_store().is_none() {
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let rooms = manifest_dir.join("data/rooms");
            let runtime = manifest_dir.join("data/runtime");
            let data = manifest_dir.join("data");
            let _ = crate::store::init(
                rooms.to_str().unwrap_or("data/rooms"),
                runtime.to_str().unwrap_or("data/runtime"),
                data.to_str().unwrap_or("data"),
            );
        }
        ensure_ecology_items_in_store();
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            assert!(s.items.len() >= 1120, "Store items 應至少包含 1,120 件物件");
            assert!(s.items.contains_key("wt_cat_0001"));
            assert!(s.items.contains_key("wt_cat_1109"));
        }

        let raw_id = "mat_iron_scrap";
        let finished_id = STANDARD_FINISHED_GOODS[0].id;
        let settlement = "文曜";
        let room_name = "文曜萬爐鐵坊棚";
        let raw_key = (settlement.to_string(), raw_id.to_string());
        let finished_key = (settlement.to_string(), finished_id.to_string());

        let initial_ema = {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            let cell = ledger
                .settlement_markets
                .entry(raw_key.clone())
                .or_insert_with(|| init_settlement_market_cell(settlement, raw_id));
            cell.stock = 0;
            cell.demand_ema = 1.0;
            cell.demand_ema
        };

        let initial_finished_stock = {
            let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
            ledger.settlement_markets.get(&finished_key).map(|c| c.stock).unwrap_or(0)
        };

        let npc_id = "test_artisan_npc";
        let result = run_npc_autonomous_production_step("test_room", room_name, &[npc_id.to_string()], 1729000000);

        assert!(result.is_some(), "工坊待料時應產出短缺敘事反饋");
        let narr = result.unwrap();
        assert!(narr.contains("已經見底") || narr.contains("暫停手頭工序"));

        let after_finished_stock = {
            let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
            ledger.settlement_markets.get(&finished_key).map(|c| c.stock).unwrap_or(0)
        };
        assert_eq!(after_finished_stock, initial_finished_stock);

        let after_ema = {
            let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
            ledger.settlement_markets.get(&raw_key).map(|c| c.demand_ema).unwrap_or(1.0)
        };
        assert!(after_ema > initial_ema);

        let report = format_settlement_supply_demand_report("文曜");
        assert!(report.contains("文曜市集"));
        assert!(report.contains("較為充裕"));
        assert!(report.contains("頗為緊俏"));

        let scarcest = scarcest_item_for_settlement("文曜");
        assert!(scarcest.is_some());
    }

    #[test]
    fn test_crystal_items_lookup_and_material_classification() {
        use crate::combat::wordplate::{WordQuality, WordSource};

        // 1. 驗證 15 款晶石之 lookup_ecology_item_def 與 lookup_item_origin
        let sources = [
            WordSource::Beast,
            WordSource::Plant,
            WordSource::Mineral,
            WordSource::Water,
            WordSource::Qi,
        ];
        let qualities = [WordQuality::Bing, WordQuality::Yi, WordQuality::Jia];

        for src in sources {
            for qual in qualities {
                let crystal_id = crate::combat::wordplate::crystal_item_id(src, qual);
                let origin = lookup_item_origin(&crystal_id);
                assert_eq!(origin, "白塔半島・天地五源");

                let def = lookup_ecology_item_def(&crystal_id);
                assert!(def.is_some(), "結晶物品 {crystal_id} 必須有定義");
                let d = def.unwrap();
                assert_eq!(d.item_type, "word_crystal");
                assert_eq!(d.weight, 0.1);
                assert!(d.name.contains(src.label_zh()));
                assert!(d.name.contains(qual.label_zh()));
                assert!(d.description.contains("天然晶石"));
            }
        }

        // 2. 驗證 classify_material_word_source 排除與分類規則
        assert!(classify_material_word_source("crystal_beast_jia", "甲品・獸源晶石").is_none());
        assert!(classify_material_word_source("starter_blade", "初心者長刀").is_none());
        assert!(classify_material_word_source("key_iron", "鐵鑰匙").is_none());

        // 獸源 (Beast)
        let (src, tier) = classify_material_word_source("wt_cat_0176", "泥甲巨鱷厚麟皮").unwrap();
        assert_eq!(src, WordSource::Beast);
        assert_eq!(tier, 4);

        let (src, tier) = classify_material_word_source("wt_cat_0171", "山林石虎斑皮").unwrap();
        assert_eq!(src, WordSource::Beast);
        assert_eq!(tier, 3);

        // 水源 (Water)
        let (src, tier) = classify_material_word_source("wt_cat_0236", "大花枝鮮黑墨囊").unwrap();
        assert_eq!(src, WordSource::Water);
        assert_eq!(tier, 4);

        let (src, tier) = classify_material_word_source("mat_salted_fish", "水尾寮醃鹹魚").unwrap();
        assert_eq!(src, WordSource::Water);
        assert_eq!(tier, 2);

        // 礦源 (Mineral)
        let (src, tier) = classify_material_word_source("mat_rough_ore", "粗鐵礦石").unwrap();
        assert_eq!(src, WordSource::Mineral);
        assert_eq!(tier, 2);

        let (src, tier) = classify_material_word_source("mat_stone", "玄武岩石板").unwrap();
        assert_eq!(src, WordSource::Mineral);
        assert_eq!(tier, 3);

        // 植源 (Plant)
        let (src, tier) = classify_material_word_source("mat_foraged_herb", "山野鮮藥草").unwrap();
        assert_eq!(src, WordSource::Plant);
        assert_eq!(tier, 2);

        let (src, tier) = classify_material_word_source("wt_cat_0370", "甕藏陳年老菜脯").unwrap();
        assert_eq!(src, WordSource::Plant);
        assert_eq!(tier, 3);

        let (src, tier) = classify_material_word_source("wt_cat_herb", "野生深山牛樟芝").unwrap();
        assert_eq!(src, WordSource::Plant);
        assert_eq!(tier, 4);

        // 氣源 (Qi fallback)
        let (src, tier) = classify_material_word_source("mat_incense_dust", "晨嵐清風散").unwrap();
        assert_eq!(src, WordSource::Qi);
        assert_eq!(tier, 2);
    }

    #[test]
    fn test_classify_catalog_wearable_slot_and_lookup() {
        // 1. 排除重型固定設施與生鮮原料
        assert!(classify_catalog_wearable_slot("水力原木大鋸", "18", "").is_none());
        assert!(classify_catalog_wearable_slot("紅土耐火磚高溫鐵爐", "18", "").is_none());
        assert!(classify_catalog_wearable_slot("萬鈞玄武岩重錨", "19", "").is_none());
        assert!(classify_catalog_wearable_slot("老樟木重型木工工作台", "18", "").is_none());
        assert!(classify_catalog_wearable_slot("平底載重八人木渡船", "19", "").is_none());
        assert!(classify_catalog_wearable_slot("赤鐵重礦", "01", "").is_none());

        // 2. 手持百工器具 / 刀具（器物無分工具與武器）
        let blade = classify_catalog_wearable_slot("大型手拉兩用剖木大鋸", "18", "");
        assert_eq!(blade, Some(("equipment", "hold_r", 0, 2, 4)));

        let shears = classify_catalog_wearable_slot("熟鐵裁布大剪刀", "16", "");
        assert_eq!(shears, Some(("equipment", "hold_r", 0, 2, 4)));

        let knife = classify_catalog_wearable_slot("岩羊角手柄折疊獵刀", "19", "");
        assert_eq!(knife, Some(("equipment", "hold_r", 0, 2, 4)));

        let pick = classify_catalog_wearable_slot("雙刃碎石手鎬", "18", "");
        assert_eq!(pick, Some(("equipment", "hold_r", 2, 0, 4)));

        let shovel = classify_catalog_wearable_slot("長把平頭挖泥鐵鍬", "18", "");
        assert_eq!(shovel, Some(("equipment", "hold_r", 1, 2, 3)));

        let torch = classify_catalog_wearable_slot("手拋式三色信號火把", "19", "");
        assert_eq!(torch, Some(("equipment", "hold_l", 2, 2, 1)));

        // 3. 隨身穿戴服飾防具與飾品
        let hat = classify_catalog_wearable_slot("雙層防雨斗笠", "16", "");
        assert_eq!(hat, Some(("equipment", "head", 2, 1, 0)));

        let goggles = classify_catalog_wearable_slot("雙扣防雪盲皮革護目風鏡", "19", "");
        assert_eq!(goggles, Some(("equipment", "face", 1, 2, 0)));

        let boots = classify_catalog_wearable_slot("熟成防水牛皮短靴", "16", "");
        assert_eq!(boots, Some(("equipment", "feet", 2, 3, 0)));

        let warmer = classify_catalog_wearable_slot("滑石隨身暖手爐", "17", "");
        assert_eq!(warmer, Some(("equipment", "trinket", 2, 1, 0)));

        // 4. lookup_ecology_item_def 火把與長柄
        let pine_torch = lookup_ecology_item_def("craft_pine_torch").unwrap();
        assert_eq!(pine_torch.slot, "hold_l");
        assert_eq!(pine_torch.item_type, "equipment");

        let handle1 = lookup_ecology_item_def("craft_wooden_tool_handle").unwrap();
        assert_eq!(handle1.slot, "hold_r");
        assert_eq!(handle1.item_type, "equipment");

        let handle2 = lookup_ecology_item_def("craft_wooden_handle").unwrap();
        assert_eq!(handle2.slot, "hold_r");
        assert_eq!(handle2.item_type, "equipment");
    }

    #[test]
    fn test_craft_dims_and_shanhai_cooking_highlight_integration() {
        let dims = crate::server::handler::entity_total_craft_dims("unit_test_actor");
        assert!(dims.forge >= 0.0);
        assert!(dims.culinary >= 0.0);
        assert!(dims.harvest >= 0.0);
        assert!(dims.wayfare >= 0.0);

        let packet = crate::ai::CraftActionFactPacket {
            actor_name: "行旅刀客".to_string(),
            tool_name: "行旅銅釜".to_string(),
            tool_familiarity: "沉穩趁手".to_string(),
            breath_habit: "潮汐連息".to_string(),
            material_names: vec!["山胡椒馬告粒".to_string(), "馬告炭炙高地水鹿腱".to_string()],
            output_name: "馬告炭炙高地水鹿腱".to_string(),
            highlight_kind: "山海營火合味現煮".to_string(),
            environment_hint: "野地營火堆".to_string(),
        };
        let highlight = crate::ai::compose_craft_highlight_from_data(&packet);
        assert!(!highlight.is_empty(), "LLM craft highlight must generate fallback or local prose");
        assert!(!highlight.contains("系統"), "Highlight cannot contain forbidden word 系統");
        assert!(!highlight.contains("玩家"), "Highlight cannot contain forbidden word 玩家");
        assert!(!highlight.chars().any(|c| c.is_ascii_digit()), "Highlight cannot contain raw digits");

        let (extra, note) = harvest_from_world_object("unit_test_actor", "obj_wild_ore", "露天鐵礦石", "harvest_ore", 1_700_000_000);
        assert!(extra >= 0);
        if extra > 0 {
            assert!(note.contains("手藝純熟") || note.contains("趁手"));
        }
    }

    #[test]
    fn test_shrimp_and_food_raw_materials_never_become_storage_box() {
        assert_eq!(get_clean_item_name("mat_fresh_shrimp"), "水尾寮鮮沼蝦");
        assert_ne!(get_clean_item_name("mat_fresh_shrimp"), "行囊物產");
        assert_eq!(get_clean_item_name("shanhai_fresh_fish"), "深海鮮魚");
        assert_eq!(get_clean_item_name("shanhai_beast_meat"), "山海鮮獸肉");
        assert_eq!(map_raw_to_finished("mat_fresh_shrimp").id, "craft_roasted_shrimp_skewer");
        assert_eq!(map_raw_to_finished("mat_fresh_shrimp").name, "炭炙水尾鮮蝦串");
        assert_eq!(map_raw_to_finished("wt_cat_0225").id, "craft_roasted_shrimp_skewer");
        assert_eq!(map_raw_to_finished("shanhai_fresh_fish").id, "craft_smoked_fish_fillet");
        assert_eq!(map_raw_to_finished("shanhai_fresh_fish").name, "相思木燻鮮魚乾");
        assert_eq!(map_raw_to_finished("harvest_fish").id, "craft_smoked_fish_fillet");
        assert_eq!(map_raw_to_finished("shanhai_beast_meat").id, "craft_spiced_beast_jerky");
        assert_eq!(map_raw_to_finished("shanhai_beast_meat").name, "馬告炭炙獸肉脯");
        assert_eq!(map_raw_to_finished("mat_grain_sack").id, "craft_ration_grain_cake");
        assert_eq!(map_raw_to_finished("mat_grain_sack").name, "行旅耐儲米糧餅");
        assert_eq!(map_raw_to_finished("mat_fishing_line_bundle").id, "craft_storm_canvas_cloak");
        assert_eq!(map_raw_to_finished("mat_charcoal_sack").id, "craft_pine_torch");
        assert_eq!(map_raw_to_finished("mat_sealing_pitch").id, "craft_bamboo_oil_umbrella");
        assert_eq!(map_raw_to_finished("mat_water_moss_sheet").id, "craft_meridian_salve");
        assert_eq!(map_raw_to_finished("mat_stone_powder_sack").id, "craft_iron_pickaxe");
        assert_eq!(map_raw_to_finished("mat_shell_sack").id, "craft_iron_pickaxe");
    }

    #[test]
    fn test_run_npc_autonomous_production_step_venue_staff_and_event_pack() {
        // 1. 驗證 RoomVenueObject 語意標籤
        let mill_venue = crate::grid::resolve_any_room_venue_object("老堰頭・水輪石磨工坊", "").expect("mill venue");
        assert_eq!(mill_venue.default_host_title, "當值石磨匠師");
        let tags = mill_venue.semantic_tags();
        assert!(tags.contains(&"workshop") || tags.contains(&"grindstone"));

        // 2. 測試當值人員辨識與物件化標題解析
        let (is_staff, has_uniform, in_counter) = crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            "老堰頭・水輪石磨工坊",
            "ambient_mill_staff_01",
            "身穿制服當值司秤掌事",
            "正於石磨櫃檯盤點貨單",
            r#"{"body":"wt_cat_0840"}"#,
        );
        assert!(is_staff);
        assert!(has_uniform);
        assert!(in_counter);

        // 3. 測試 run_npc_autonomous_production_step
        let room = "老堰頭・水輪石磨工坊";
        let step_res = run_npc_autonomous_production_step("room_mill_01", room, &["ambient_mill_worker".to_string()], 1_700_000_000);
        assert!(step_res.is_some());
        let narr = step_res.unwrap();
        assert!(narr.contains("老堰頭"));
        assert!(narr.contains("大石磨") || narr.contains("現磨溫米漿"));

        // 4. 驗證器物推進事件包與劇本包
        let advance_res = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
            room,
            "現磨溫米漿",
        );
        assert!(advance_res.is_some(), "finished good must advance room event pack");
        let (ev, sp, note) = advance_res.unwrap();
        assert_eq!(ev.pack_id, "WS-001");
        assert_eq!(sp.pack_id, "WS-001");
        assert!(!note.is_empty());
    }
}
