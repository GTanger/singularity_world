fn apply_catalog_effects_part5(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now: i64,
    narrative: &mut String,
) -> bool {
    // --- 文曜外環工坊、碼頭、窯廠、紙墨印造、倉庫等重工勞動器物 ---
    if action != "WarmHands" && (matches!(
        obj.id.as_str(),
        "wt_outer_wharf_crane"
            | "wt_obj_wharf_crane"
            | "wt_outer_ship_ribs"
            | "wt_obj_ship_ribs_pitch"
            | "wt_outer_net_rack"
            | "wt_obj_drying_nets_trough"
            | "wt_obj_rope_twist_wheel"
            | "wt_outer_dock_caulk"
            | "wt_outer_dredge_winch"
            | "wt_obj_patrol_skiff_gear"
            | "wt_outer_reed_rack"
            | "wt_obj_timber_reed_stacks"
            | "wt_outer_salt_vat"
            | "wt_obj_iron_salt_stacks"
            | "wt_outer_paper_vat"
            | "wt_obj_paper_making_screen"
            | "wt_obj_pulp_vat"
            | "wt_outer_ink_mortar"
            | "wt_obj_ink_soot_mortar"
            | "wt_obj_ink_mould_drying"
            | "wt_outer_type_tray"
            | "wt_obj_movable_type_case"
            | "wt_outer_bind_press"
            | "wt_obj_printing_press"
            | "wt_obj_book_binding_press"
            | "wt_outer_trip_hammer"
            | "wt_obj_trip_hammer_trough"
            | "wt_obj_bellows_furnace"
            | "wt_forge_furnace_anvil"
            | "wt_forge_quench_trough"
            | "wt_wood_planing_bench"
            | "wt_outer_water_saw"
            | "wt_obj_waterwheel_sawframe"
            | "wt_outer_stone_saw"
            | "wt_obj_stone_gang_saw"
            | "wt_outer_bronze_crucible"
            | "wt_obj_bronze_crucible"
            | "wt_outer_charcoal_kiln"
            | "wt_obj_charcoal_kiln"
            | "wt_outer_brick_kiln"
            | "wt_obj_long_kiln_firebox"
            | "wt_outer_canvas_loom"
            | "wt_obj_sail_canvas_bench"
            | "wt_obj_loom_dye_vats"
            | "wt_outer_fur_press"
            | "wt_obj_fur_baling_press"
            | "wt_outer_wagon_jack"
            | "wt_obj_wagon_jack_wheel"
            | "wt_outer_salt_iron_scale"
            | "wt_obj_steelyard_cart"
    ) || (action == "Operate" || action == "Use" || action == "Caulking" || action == "Pound" || action == "PickType" || action == "RollInk" || action == "Stitch" || action == "Quench") && (
        obj.name.contains("滑輪吊架")
            || obj.name.contains("龍骨肋架")
            || obj.name.contains("曬網木架")
            || obj.name.contains("手搖木輪")
            || obj.name.contains("長柄鐵斗")
            || obj.name.contains("紮排毛竹")
            || obj.name.contains("抄紙青竹簾")
            || obj.name.contains("搗墨鐵臼")
            || obj.name.contains("字盤排架")
            || obj.name.contains("壓印機")
            || obj.name.contains("木夾壓平機")
            || obj.name.contains("水力聯動重鐵錘")
            || obj.name.contains("活塞木風箱")
            || obj.name.contains("鍛台")
            || obj.name.contains("鍛造")
            || obj.name.contains("鐵砧")
            || obj.name.contains("鍛爐")
            || obj.name.contains("淬火槽")
            || obj.name.contains("木作台")
            || obj.name.contains("大水車連軸排鋸")
            || obj.name.contains("框鋸")
            || obj.name.contains("悶炭窯")
            || obj.name.contains("斜坡龍窯")
            || obj.name.contains("帆布高織機")
            || obj.name.contains("壓捆木機")
            || obj.name.contains("千斤頂")
            || obj.name.contains("大磅秤")
    )) {
        let ent = db::get_entity(pid).ok().flatten();
        let vit = ent.as_ref().map(|e| e.vit).unwrap_or(10);
        if vit <= 1 {
            narrative.push_str("\n【體力不支】你氣喘吁吁，四肢酸痛無力，需稍作停歇恢復體力。");
            return true;
        }
        let (cost_vit, wage, labor_desc) = match obj.id.as_str() {
            "wt_outer_wharf_crane" | "wt_obj_wharf_crane" => (
                4,
                10,
                "你緊握滑輪牽引粗麻纜繩，踩穩跳板協同碼頭苦力將重型貨箱自駁船起吊至條石駁岸，臂膀筋骨緊繃，動作沉穩洗練。",
            ),
            "wt_outer_ship_ribs" | "wt_obj_ship_ribs_pitch" | "wt_outer_dock_caulk" => (
                5,
                12,
                "你手持嵌縫刀蘸取滾沸的熟桐油松香瀝青，專注填塞船底龍骨與水密隔艙縫隙，膠泥密實嚴整，滴水不漏。",
            ),
            "wt_outer_net_rack" | "wt_obj_drying_nets_trough" | "wt_obj_rope_twist_wheel" => (
                3,
                8,
                "你搖動三聯絞繩輪，三股浸油苧麻絲在均勻轉動中緊密咬合絞成粗壯縴繩，手掌磨出厚繭，繩身堅韌沉實。",
            ),
            "wt_outer_dredge_winch" | "wt_obj_patrol_skiff_gear" => (
                4,
                9,
                "你推動長柄鋼齒鐵斗探入碼頭下沿清除河道淤泥暗沙，將沉積泥沙整車推卸至駁岸外，航道重獲通暢清澈。",
            ),
            "wt_outer_reed_rack" | "wt_obj_timber_reed_stacks" => (
                4,
                8,
                "你搬運運下排筏的三年生毛竹與重型巨杉圓木，按營造尺寸整齊排垛並蓋好防潮油布，通風堆垛井然有序。",
            ),
            "wt_outer_salt_vat" | "wt_obj_iron_salt_stacks" => (
                4,
                9,
                "你協助鹽倉掌事挑運粗鹽大包並分層以十字交錯法壓緊防潮油布包，粗重勞作下汗如雨下，倉底乾燥踏實。",
            ),
            "wt_outer_paper_vat" | "wt_obj_paper_making_screen" | "wt_obj_pulp_vat" => (
                3,
                8,
                "你雙手平托青竹抄紙簾深入石槽，腕力輕巧一蕩，均勻濾去白漿殘水，簾面結出一層柔韌勻薄的純淨宣紙纖維。",
            ),
            "wt_outer_ink_mortar" | "wt_obj_ink_soot_mortar" | "wt_obj_ink_mould_drying" => (
                5,
                11,
                "你提起重鐵杵在大臼中千百次奮力舂搗皮膠與上等松煙墨坯，筋道十足的墨泥在反覆捶打下油黑鋥亮、泛出紫光。",
            ),
            "wt_outer_type_tray" | "wt_obj_movable_type_case" => (
                3,
                7,
                "你手持字夾按韻律偏旁在字盤架中快速揀字入盤，推動羊毛油墨滾筒在字版上反覆滾勻松煙油墨，字跡黑潤分明。",
            ),
            "wt_outer_bind_press" | "wt_obj_printing_press" | "wt_obj_book_binding_press" => (
                4,
                9,
                "你奮力扳動螺旋壓印機手柄，重板均勻受力壓覆紙面，揭開時字跡黑亮深透；隨後以拋光骨針鎖定線裝書脊，整飭端莊。",
            ),
            "wt_outer_trip_hammer" | "wt_obj_trip_hammer_trough" | "wt_obj_bellows_furnace" | "wt_forge_furnace_anvil" => (
                5,
                12,
                "你手持長鍛鉗將燒紅的重型鋼錠送入鍛台之下，隨著重錘轟然夯擊，火星四射間精鋼雜質盡去，器型雄厚。",
            ),
            "wt_forge_quench_trough" => (
                4,
                11,
                "你夾起通紅熾熱的刀坯猛然浸入導能精鋼冷卻淬火槽，白霧蒸騰翻湧、嗤啦作響，寒泉冷卻間器紋緊密收束。",
            ),
            "wt_outer_water_saw" | "wt_obj_waterwheel_sawframe" | "wt_wood_planing_bench" => (
                4,
                10,
                "你操持導能木作器械推刨剖木，木屑紛飛間平直厚重的良木板材與榫卯構件依次成型。",
            ),
            "wt_outer_stone_saw" | "wt_obj_stone_gang_saw" => (
                5,
                11,
                "你引導細水與金剛砂均勻注入玄武岩石縫，握緊鋼鑿重錘修平開山條石斷面，石面平直如砥，嚴絲合縫。",
            ),
            "wt_outer_bronze_crucible" | "wt_obj_bronze_crucible" => (
                5,
                12,
                "你戴上防護牛皮手套平穩夾持石墨坩堝，將金紅透亮的銅錫合金熔液順著澆口緩緩注入翻砂模具，成型精準大氣。",
            ),
            "wt_outer_charcoal_kiln" | "wt_obj_charcoal_kiln" => (
                4,
                9,
                "你協助封堵黏土炭窯窯門排煙口，掌控悶燒火候，出窯後將敲擊如金石的堅密無煙硬木白炭裝箱碼垛。",
            ),
            "wt_outer_brick_kiln" | "wt_obj_long_kiln_firebox" => (
                5,
                10,
                "你頂著炙熱火浪向斜坡龍窯火膛添送耐火薪柴，密切觀察觀火孔火焰呈白熾色澤，磚瓦坯體在高溫下逐步瓷化堅固。",
            ),
            "wt_outer_canvas_loom" | "wt_obj_sail_canvas_bench" | "wt_obj_loom_dye_vats" => (
                4,
                8,
                "你腳踏高織機踏板投梭引緯，重密粗麻經緯緊扣交織，隨後將新織防風帆布下缸浸染群青，布料抗撕拉力極佳。",
            ),
            "wt_outer_fur_press" | "wt_obj_fur_baling_press" => (
                4,
                9,
                "你轉動木製壓捆機螺桿，將大批熟化好的優等原牛皮平整壓緊並以浸油麻繩打結封固，捆包緊實方正。",
            ),
            "wt_outer_wagon_jack" | "wt_obj_wagon_jack_wheel" => (
                4,
                10,
                "你旋動重型千斤頂將雙輪貨車大架頂升懸空，手持大鐵錘校正包鐵車軸並塗抹潤滑桐油脂，車輪轉動平穩無阻。",
            ),
            _ => (
                4,
                8,
                "你熟練操縱磅秤與搬運墊木，清點度量進出大宗貨物斤兩，動作俐落嚴謹，帳物分明。",
            ),
        };
        let actual_cost = if vit <= 10 { 1 } else { cost_vit };
        let _ = db::update_vit(pid, (vit - actual_cost).max(2));
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, wage);
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        narrative.push_str(&format!(
            "\n{}\n你投入體魄勞作，自市場營造司領取當班工造津貼收入行囊，筋骨雖感微乏，補給資財已然妥善結算。",
            labor_desc
        ));
        let inv_qty = |item_id: &str| -> i32 {
            ent.as_ref()
                .map(|e| {
                    crate::db::get_inventory(&e.inventory, e.vit)
                        .items
                        .iter()
                        .find(|it| it.item_id == item_id)
                        .map(|it| it.qty)
                        .unwrap_or(0)
                })
                .unwrap_or(0)
        };
        if (obj.name.contains("毛竹") || obj.id == "wt_outer_reed_rack" || obj.id == "wt_obj_timber_reed_stacks") && inv_qty("mat_timber_log") < 4 {
            let _ = db::add_to_inventory(pid, "mat_timber_log", 1);
            narrative.push_str("\n你在搬運整理排筏竹木之餘，順道自料垛獲贈了一根韌性上乘的「伐採原木」收入行囊。");
        } else if (obj.name.contains("鹽倉") || obj.id == "wt_outer_salt_vat" || obj.id == "wt_obj_iron_salt_stacks") && inv_qty("mat_iron_scrap") < 4 {
            let _ = db::add_to_inventory(pid, "mat_iron_scrap", 1);
            narrative.push_str("\n你在協助鹽倉搬運壓包之際，獲贈了一塊用以加固木架的「回收熟鐵料塊」收入行囊。");
        } else if obj.name.contains("蘆葦") && inv_qty("mat_reed_bundle") < 4 {
            let _ = db::add_to_inventory(pid, "mat_reed_bundle", 1);
            narrative.push_str("\n你順手割取了一捆青翠韌實的「水澤老蘆草綑」收入行囊。");
        } else if obj.id == "wt_forge_furnace_anvil"
            || obj.name.contains("鍛造")
            || obj.name.contains("鐵砧")
            || obj.name.contains("鍛爐")
            || obj.name.contains("鍛台")
        {
            if inv_qty("mat_iron_scrap") < 4 {
                let _ = db::add_to_inventory(pid, "mat_iron_scrap", 1);
                narrative.push_str("\n你在高爐鍛台旁清整爐渣時，順手拾得一塊成色純良的「回收熟鐵料塊」收入行囊。");
            }

            // 檢查持握兵刃或行囊工具是否需要回火修整
            let vessel_to_temper = if let Ok(Some(ch)) = db::get_entity(pid) {
                let mut cand = None;
                if !ch.equipment_slots.is_empty() {
                    if let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(&ch.equipment_slots) {
                        let check_slots = ["weapon", "hold_r", "main_hand", "hold", "tool", "hold_l"];
                        for s in check_slots {
                            if let Some(id) = slots.get(s).filter(|id| !id.is_empty()) {
                                let name = db::get_item_def(id).map(|d| d.name).unwrap_or_else(|| id.clone());
                                if crate::pg::living_world::is_weapon_or_tool_item(id, &name) {
                                    let st = crate::pg::living_world::get_vessel_state(pid, id);
                                    if st.condition != crate::pg::living_world::VesselCondition::Sharp {
                                        cand = Some((id.clone(), name));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                if cand.is_none() {
                    let inv_items: Vec<serde_json::Value> = serde_json::from_str(&ch.inventory).unwrap_or_default();
                    for it in inv_items {
                        let id = it.get("item_id").or_else(|| it.get("id")).and_then(|v| v.as_str()).unwrap_or_default();
                        if id.is_empty() { continue; }
                        let name = db::get_item_def(id).map(|d| d.name).unwrap_or_else(|| id.to_string());
                        if crate::pg::living_world::is_weapon_or_tool_item(id, &name) {
                            let st = crate::pg::living_world::get_vessel_state(pid, id);
                            if st.condition != crate::pg::living_world::VesselCondition::Sharp {
                                cand = Some((id.to_string(), name));
                                break;
                            }
                        }
                    }
                }
                cand
            } else {
                None
            };

            if let Some((item_id, item_name)) = vessel_to_temper {
                let prev_st = crate::pg::living_world::get_vessel_state(pid, &item_id);
                let st = crate::pg::living_world::temper_vessel_at_forge(pid, &item_id, now);
                let fix_desc = match prev_st.condition {
                    crate::pg::living_world::VesselCondition::SlightlyDull => "微鈍之刃口重歸「鋒芒畢露」",
                    crate::pg::living_world::VesselCondition::RolledEdge => "捲曲崩缺之刃口重歸「鋒芒畢露」",
                    _ => "刃口受創處重歸「鋒芒畢露」",
                };
                narrative.push_str(&format!(
                    "\n你順手將受損的「{}」探入炭火回火、於鐵砧輕敲校直，{}（{}）！",
                    item_name,
                    fix_desc,
                    st.status_badge()
                ));
                crate::pg::living_world::record_breath_activity(
                    pid,
                    crate::pg::living_world::BreathHabitKind::ForgeHeavy,
                    2,
                    now,
                );
            }
        } else if (obj.id == "wt_wood_planing_bench" || obj.name.contains("木作台")) && inv_qty("mat_timber_log") < 4 {
            let _ = db::add_to_inventory(pid, "mat_timber_log", 1);
            narrative.push_str("\n你在木作台下整理邊角良木，順手收起一截乾燥結實的「伐採原木」收入行囊。");
        }
        if (obj.id == "wt_forge_quench_trough" || obj.name.contains("淬火槽")) && action == "Quench" {
            if let Ok((_mut_id, _mut_name, mut_narrative)) =
                crate::game::object_ecology::mutate_finished_item(pid, "auto", &obj.name, now)
            {
                narrative.push_str(&format!("\n藉著導能淬火槽的極溫冷熱激盪，{}", mut_narrative));
            }
        }
        push_refresh(conn, pid);
        return true;
    }

    // --- 文曜外環餐飲、茶攤、食肆器物 ---
    if matches!(
        obj.id.as_str(),
        "wt_outer_sailor_tea"
            | "wt_obj_river_tea_kettle"
            | "wt_outer_worker_canteen"
            | "wt_obj_forge_mess_cauldron"
            | "wt_outer_gate_tavern"
            | "wt_obj_mutton_broth_pot"
    ) || (action == "Taste" || action == "WarmHands") && (
        obj.name.contains("大銅茶壺")
            || obj.name.contains("水手粗陶大碗")
            || obj.name.contains("厚鐵炒鑊")
            || obj.name.contains("老白乾")
            || obj.name.contains("羊肉骨頭滾湯")
    ) {
        let magnesium = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| e.magnesium)
            .unwrap_or(0);
        let cost = if obj.id.contains("tavern") || obj.name.contains("羊肉") { 3 } else { 2 };
        if magnesium < cost {
            narrative.push_str(&format!(
                "\n【茶飯小憩】隨身鎂幣不足（需 {} 鎂），店家熱情招呼後遞上一盞溫涼清水解渴。",
                cost
            ));
            return true;
        }
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -cost);
        let (vit_gain, qi_gain, desc) = match obj.id.as_str() {
            "wt_outer_sailor_tea" | "wt_obj_river_tea_kettle" => (
                10,
                8,
                "你倒了一大碗熱氣蒸騰的濃釅臨江磚茶仰頭飲盡，微苦甘醇驅散了江風濕寒，通體暖意融融。",
            ),
            "wt_outer_worker_canteen" | "wt_obj_forge_mess_cauldron" => (
                14,
                8,
                "你盛了一大碗熱油爆炒的濃油赤醬燴原牛肉與大塊雜糧饃，肉香濃郁，大口朵頤間四肢力量漸復充盈。",
            ),
            _ => (
                16,
                10,
                "你端起粗瓷大碗痛飲奶白滾燙的原羊骨高湯，大蔥花與白胡椒粉直透百骸，驅盡荒原風塵寒意，渾身舒泰無比。",
            ),
        };
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + vit_gain).min(100);
                e.qi = (e.qi + qi_gain).min(100);
            });
        }
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        narrative.push_str(&format!(
            "\n【飲食溫養】{}\n支付 {} 鎂，體魄與元氣皆獲調理舒暢。",
            desc, cost
        ));
        if let Some((shanhai_cook_narr, _v, _q)) = crate::game::object_ecology::try_cook_shanhai_delicacy(pid, &obj.name) {
            narrative.push('\n');
            narrative.push_str(&shanhai_cook_narr);
        }
        push_refresh(conn, pid);
        return true;
    }

    // --- 文曜草藥、醫館、獸醫器物 ---
    if matches!(
        obj.id.as_str(),
        "wt_outer_herb_cabinet"
            | "wt_obj_herb_cabinet_mortar"
            | "wt_obj_acupuncture_pulse_pillow"
            | "wt_outer_herb_drying"
            | "wt_obj_herb_drying_rack"
            | "wt_outer_vet_stocks"
            | "wt_obj_veterinary_box"
    ) || (action == "Weigh" || action == "Pound" || action == "Smell") && (
        obj.name.contains("百眼藥櫃")
            || obj.name.contains("紫銅搗藥臼")
            || obj.name.contains("採藥背簍竹架")
            || obj.name.contains("修蹄鐵鉗")
            || obj.name.contains("獸用草藥丸")
    ) {
        if obj.id == "wt_outer_herb_cabinet" || obj.id == "wt_obj_herb_cabinet_mortar" || obj.id == "wt_obj_acupuncture_pulse_pillow" || obj.name.contains("藥櫃") || obj.name.contains("脈枕") {
            if action == "Smell" {
                narrative.push_str("\n【藥香沁脾】百眼藥櫃中散發出清苦回甘的草木藥香，令人頭目清朗。");
                push_refresh(conn, pid);
                return true;
            }
            if action == "Weigh" {
                // Free weighing interaction, already handled by base look/object interaction or returns without charge
                push_refresh(conn, pid);
                return true;
            }
            if action == "Use" || action == "Operate" || action == "Buy" || action == "Acquire" {
                let magnesium = db::get_entity(pid)
                    .ok()
                    .flatten()
                    .map(|e| e.magnesium)
                    .unwrap_or(0);
                if magnesium >= 3 {
                    let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
                    if let Some(arc) = crate::store::get_store() {
                        let _ = arc.write().update_entity(pid, |e| {
                            e.vit = (e.vit + 18).min(100);
                            e.qi = (e.qi + 10).min(100);
                        });
                    }
                    let _ = event::append(
                        now,
                        pid,
                        types::SCENE_OBJECT_ACTION,
                        &format!("{}|{}", obj.id, action),
                    );
                    narrative.push_str("\n【本草調養】百草堂醫師為你開具清心避瘴草藥湯劑，微苦回甘的藥力入喉，經絡暢達、靈台清朗（心神與周身氣血隨之充實）。");
                    push_refresh(conn, pid);
                } else {
                    narrative.push_str("\n【草藥提示】診金與草藥調劑需 3 鎂，隨身錢資不足，醫師先行贈與一盞潤喉清熱草藥茶稍作調理。");
                }
                return true;
            }
            return true;
        } else {
            // wt_outer_herb_drying / wt_outer_vet_stocks 協助分藥與照料牲口
            let ent = db::get_entity(pid).ok().flatten();
            let vit = ent.as_ref().map(|e| e.vit).unwrap_or(0);
            if vit > 5 {
                let _ = db::update_vit(pid, (vit - 2).max(1));
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 5);
                let _ = event::append(
                    now,
                    pid,
                    types::SCENE_OBJECT_ACTION,
                    &format!("{}|{}", obj.id, action),
                );
                narrative.push_str("\n【醫坊襄助】你協助翻曬篾席上的新鮮草藥並修整駝獸硬蹄，動作細緻沉穩，獲贈 5 鎂勞動津貼（稍耗體力，獲得 5 鎂）。");
                push_refresh(conn, pid);
            } else {
                narrative.push_str("\n【襄助提示】你神色疲憊，醫館掌事勸你先在長凳上坐下歇息片刻再行幫忙。");
            }
            return true;
        }
    }

    // --- 文曜外環學術、測繪、天象、市集、城門坊器物 ---
    if matches!(
        obj.id.as_str(),
        "wt_outer_map_table"
            | "wt_obj_cartography_table"
            | "wt_obj_surveyor_theodolite"
            | "wt_outer_abacus_slate"
            | "wt_obj_abacus_counters"
            | "wt_outer_armillary"
            | "wt_obj_armillary_almanac"
            | "wt_obj_wind_vane_gauge"
            | "wt_outer_brush_rack"
            | "wt_obj_brush_inkstone_rack"
            | "wt_outer_fish_auction"
            | "wt_obj_fish_auction_trough"
            | "wt_obj_fish_baskets"
            | "wt_outer_dried_fish_scale"
            | "wt_outer_tool_rack"
            | "wt_obj_farm_tool_rack"
            | "wt_outer_hardware_bins"
            | "wt_obj_hardware_bins"
            | "wt_outer_cattle_ring"
            | "wt_obj_livestock_auction_ring"
            | "wt_outer_weapon_rack"
            | "wt_obj_weapon_rack_stone"
            | "wt_outer_dispatch_board"
            | "wt_obj_weighbridge_milepost"
            | "wt_outer_caravan_trough"
            | "wt_obj_caravan_hitch_trough"
            | "wt_obj_caravan_bell_rack"
            | "wt_inner_gate_arch"
            | "wt_gate_stone_archway_board"
            | "wt_inner_gate_trough"
            | "wt_gate_water_culvert_drum"
            | "wt_inner_gate_gong"
            | "wt_gate_patrol_gong_stone"
    ) || (action == "Measure" || action == "Calculate" || action == "Observe" || action == "TestBrush" || action == "GrindInk" || action == "Weigh" || action == "Sharpen" || action == "Feed" || action == "Shake" || action == "Ring") && (
        obj.name.contains("測繪大畫案")
            || obj.name.contains("九章算學")
            || obj.name.contains("北辰定緯銅儀")
            || obj.name.contains("湖羊長鋒筆架")
            || obj.name.contains("鮮魚競價冰槽")
            || obj.name.contains("農具")
            || obj.name.contains("五金分格鐵匣")
            || obj.name.contains("牲口競價木欄")
            || obj.name.contains("護行厚背獵刀架")
            || obj.name.contains("過磅生鐵地台")
            || obj.name.contains("青石牌坊")
            || obj.name.contains("引水石渠")
            || obj.name.contains("巡坊銅鑼")
    ) {
        let (vit_delta, qi_delta, feedback) = match obj.id.as_str() {
            "wt_outer_map_table" | "wt_obj_cartography_table" | "wt_obj_surveyor_theodolite" => (
                0,
                6,
                "你以黃銅兩腳分規在長案輿圖上比量半島水道與陸路走勢，山川形勝與關防隘口瞭然於胸，心胸豁然開朗。",
            ),
            "wt_outer_abacus_slate" | "wt_obj_abacus_counters" => (
                0,
                6,
                "你拈起紅木二十三檔算盤輕撥珠子，清脆劈啪聲中推演九章勾股受力算題，心神專注澄澈。",
            ),
            "wt_outer_armillary" | "wt_obj_armillary_almanac" | "wt_obj_wind_vane_gauge" => (
                0,
                8,
                "你透過定緯儀銅環仰觀極星子午走向，黑曜石日晷刻影分明，推步四時節氣，頓悟天行有常之道。",
            ),
            "wt_outer_brush_rack" | "wt_obj_brush_inkstone_rack" => (
                0,
                6,
                "你挑選兼毫長鋒在端溪老坑硯台中飽蘸墨汁，落筆行雲流水，滿室墨香令人意氣平和。",
            ),
            "wt_outer_fish_auction" | "wt_obj_fish_auction_trough" | "wt_obj_fish_baskets" | "wt_outer_dried_fish_scale" => (
                0,
                5,
                "你俯身查驗冰槽中銀鱗閃爍的生鮮大魚，撥動黃銅懸臂秤核算批發斤兩，江海活水清鮮之氣撲面而來。",
            ),
            "wt_outer_tool_rack" | "wt_obj_farm_tool_rack" | "wt_outer_hardware_bins" | "wt_obj_hardware_bins" => (
                0,
                5,
                "你取下淬火開刃的鋼鋤鐮刀細看刃線，翻檢鐵匣鉚釘規格，百工利器質樸精準，倍添營造踏實之感。"
            ),
            "wt_outer_cattle_ring" | "wt_obj_livestock_auction_ring" => (
                0,
                5,
                "你駐足木圍欄觀看大角原牛矯健踱步，行商比劃手勢叫價，邊地大市的蓬勃生機撲面而來。",
            ),
            "wt_outer_weapon_rack" | "wt_obj_weapon_rack_stone" => (
                0,
                6,
                "你取厚背獵刀在油石槽上緩緩推磨，冷冽刃光倒映雙眸，隨身戒備與沉穩之氣油然而生。",
            ),
            "wt_outer_dispatch_board" | "wt_obj_weighbridge_milepost" => (
                0,
                5,
                "你凝神端讀西道花崗岩路石里程：城西五里為平川，十里避風坡取水點標示詳盡，遠行前程井然在胸。",
            ),
            "wt_outer_caravan_trough" | "wt_obj_caravan_hitch_trough" | "wt_obj_caravan_bell_rack" => (
                0,
                5,
                "你輕輕搖響廊下成排鍛銅駝鈴，渾厚鈴音在開闊驛場迴盪數里，駝獸溫順抬頭，心神平靜祥和。",
            ),
            "wt_inner_gate_trough" | "wt_gate_water_culvert_drum" => (
                5,
                4,
                "你走近坊口石渠掬捧清冽活水洗面並在歇腳石鼓上暫卸行囊，清泉甘涼滌盡暑燥風塵，精神大振。",
            ),
            "wt_inner_gate_gong" | "wt_gate_patrol_gong_stone" => (
                0,
                6,
                "你輕叩坊壁巡更厚銅鑼，沉渾鑼聲悠悠蕩蕩聲震四街，驚起飛簷宿鳥，城郭晨昏秩序肅然在目。",
            ),
            _ => (
                0,
                6,
                "你仰頭瞻仰巍峨挺拔的青石牌坊，拂拭指路刻石楷書文字，各坊街巷交匯處的繁華氣象盡收眼底。",
            ),
        };
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                if vit_delta > 0 {
                    e.vit = (e.vit + vit_delta).min(100);
                }
                if qi_delta > 0 {
                    e.qi = (e.qi + qi_delta).min(100);
                }
            });
        }
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        if vit_delta > 0 {
            narrative.push_str(&format!(
                "\n【城郭風物・格物研察】{}\n體魄更顯健旺，心神元氣充盈。",
                feedback
            ));
        } else {
            narrative.push_str(&format!(
                "\n【城郭風物・格物研察】{}\n心神寧靜豁然，周身元氣充盈。",
                feedback
            ));
        }
        push_refresh(conn, pid);
        return true;
    }



    // 3) Food, Drink & Tea (action == "Taste" or obj.id == "wt_canteen_red_iron_cauldron" or obj.id == "wt_canteen_fermented_tofu_jar" or obj.id == "wt_inn_hearth_tea_table" or catalog category "11")
    let cat_opt = crate::game::peninsula_catalog::find_catalog_entry(&obj.id);
    let is_food_item = cat_opt.is_some_and(|c| {
        c.has_tag("food_and_drink") || c.has_tag("consumable") || c.socket == "Taste"
    });
    let is_food_interaction = action == "Taste"
        || ((obj.id == "wt_canteen_red_iron_cauldron"
            || obj.id == "wt_canteen_fermented_tofu_jar"
            || obj.id == "wt_inn_hearth_tea_table"
            || is_food_item)
            && matches!(action, "Use" | "Eat" | "Drink" | "BuySoup"));

    if is_food_interaction {
        let room_name = resolve_room_name(player_room);
        let is_residential = room_name.contains("住宅")
            || room_name.contains("民宅")
            || room_name.contains("居所")
            || room_name.contains("家屋")
            || room_name.contains("公寓")
            || room_name.contains("街屋")
            || room_name.contains("農舍")
            || room_name.contains("石屋")
            || room_name.contains("漁舍")
            || room_name.contains("暖屋")
            || room_name.contains("宿舍")
            || room_name.contains("高腳木屋")
            || room_name.contains("宅邸")
            || room_name.contains("宅院")
            || room_name.contains("莊園")
            || room_name.contains("別院")
            || room_name.contains("民舍");
        let has_deed = db::inventory_has_item(pid, "wt_cat_0793");
        let cost = if is_residential && has_deed { 0 } else { 2 };
        let magnesium = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| e.magnesium)
            .unwrap_or(0);
        if magnesium >= cost {
            if cost > 0 {
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -cost);
            }
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 8).min(100);
                    e.qi = (e.qi + 6).min(100);
                });
            }
            if cost > 0 {
                narrative.push_str(&format!(
                    "\n【飲食溫養】花費 {} 鎂，溫熱鮮美的飲食入腹，你感到氣血充盈、精神一振。",
                    cost
                ));
            } else {
                narrative.push_str(
                    "\n【飲食溫養】溫熱鮮美的飲食入腹，你感到氣血充盈、精神一振。",
                );
            }
            push_refresh(conn, pid);
        } else {
            narrative.push_str("\n【飲食提示】手頭的鎂不足以支付這份餐食，暫且歇手。");
        }
        return true;
    }

    // 4) Bathing, Sleeping, Warming & Medical Care
    let is_bed_or_bath_or_clinic = obj.name.contains("大床")
        || obj.name.contains("臥榻")
        || obj.name.contains("暖被")
        || obj.name.contains("床鋪")
        || obj.name.contains("熱水槽")
        || obj.name.contains("溫石")
        || obj.name.contains("浴桶")
        || obj.name.contains("蒲團")
        || obj.name.contains("坐墊")
        || obj.name.contains("茶案")
        || obj.name.contains("草蓆")
        || obj.id.contains("bed")
        || obj.id.contains("bath")
        || obj.id.contains("clinic")
        || obj.id == "wt_reg_wenyao_amenities"
        || obj.id == "wt_reg_wenyao_home";
    let is_recovery_action = if matches!(action, "Use" | "Operate") {
        is_bed_or_bath_or_clinic
    } else {
        matches!(action, "Sit" | "Rest" | "Meditate" | "Sleep" | "Bath" | "Wash" | "WarmHands" | "DryOff")
    };

    if is_recovery_action && obj.id != "wt_hall_council_table" && !obj.name.contains("公議案桌") {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = 100;
                e.qi = 100;
            });
        }
        let _ = event::append(
            now,
            pid,
            types::PLAYER_BODY,
            &format!("rested_via_{}", obj.id),
        );
        let rest_note = if !narrative.trim().is_empty() {
            ""
        } else if action == "Sleep" || obj.id.contains("bed") || obj.name.contains("大床") || obj.name.contains("羽絨暖被") || obj.name.contains("臥榻") || obj.name.contains("床鋪") {
            "你鑽入溫暖蓬鬆的羽絨暖被安穩入眠，滿室松香與安神草清香令人心神寧靜，周身疲憊盡數散去。"
        } else if action == "Bath" || action == "Wash" || obj.id.contains("bath") || action == "DryOff" || obj.name.contains("熱水槽") || obj.name.contains("溫石") {
            "熱氣騰騰的溫石活水漫過身軀，洗去了一路風塵與肌理酸脹，身心倍感舒泰通透。"
        } else if obj.id.contains("clinic") {
            "經脈絡調理與草木藥香溫養，周身酸痛與疲憊盡數散去，體魄與心神皆充盈圓滿。"
        } else if action == "Meditate" {
            "你盤膝端坐閉目凝神，調勻呼吸，漸入物我兩忘之境，周身真氣漸趨充沛圓融。"
        } else {
            "你在清風中稍作歇息，呼吸漸趨平緩，周身疲憊隨之散去，體魄與心神皆充盈圓滿。"
        };
        if !rest_note.is_empty() {
            narrative.push_str(rest_note);
        }
        if matches!(action, "Meditate" | "Practice") {
            append_somatic_awareness_note(pid, now, narrative);
            return true;
        }
        push_refresh(conn, pid);
        return true;
    }

    // 5) Gathering & Portable Item Acquisition (action == "Gather" || action == "Acquire" or portable wt_cat_xxxx item in shops/workshops/wilderness when action == "Use" || action == "Operate")
    if let Some(cat) = cat_opt {
        if action == "Gather" {
            if cat.has_tag("market_dish") || cat.has_tag("market_ware") {
                narrative.push_str(&format!(
                    "\n【櫃檯提示】「{}」為工坊堂食或市集現售物品，非野外採集之物，請於櫃檯照價購買或細細品嚐。",
                    cat.name
                ));
                push_refresh(conn, pid);
                return true;
            }
            let _ = db::add_to_inventory(pid, cat.id, 1);
            narrative.push_str(&format!(
                "\n【採集入袋】你將採收所得的「{}」妥善收入隨身行囊。",
                cat.name
            ));
            push_refresh(conn, pid);
            return true;
        } else if crate::game::peninsula_catalog::is_portable_catalog_entry(cat)
            && !db::inventory_has_item(pid, cat.id)
            && (action == "Acquire" || action == "Use" || action == "Operate")
        {
            let price = crate::game::peninsula_catalog::catalog_entry_price_mg(cat);
            let magnesium = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.magnesium)
                .unwrap_or(0);
            if magnesium >= price {
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -price);
                let _ = db::add_to_inventory(pid, cat.id, 1);
                if action == "Acquire" {
                    *narrative = format!(
                        "你支付 {} 鎂，將這件「{}」（由{}取{}精製）妥善收入隨身行囊。",
                        price, cat.name, cat.workshop, cat.origin_material
                    );
                } else {
                    narrative.push_str(&format!(
                        "\n【器物入手】你支付 {} 鎂，將這件「{}」納入隨身行囊。",
                        price, cat.name
                    ));
                }
                push_refresh(conn, pid);
            } else if action == "Acquire" {
                *narrative = format!(
                    "【結算提示】採買「{}」需支付 {} 鎂，你隨身錢資不足，未能購入。",
                    cat.name, price
                );
            }
            return true;
        } else if action == "Acquire" {
            *narrative = format!(
                "【隨身收存】你的行囊中已收有「{}」，毋需重複採買。",
                cat.name
            );
            return true;
        }
    }
    if matches!(action, "Meditate" | "Practice") {
        append_somatic_awareness_note(pid, now, narrative);
        return true;
    }
    false
}

fn apply_peninsula_world_object_effects(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now: i64,
    narrative: &mut String,
) {
    let is_medical_or_rest = obj.name.contains("診")
        || obj.name.contains("醫")
        || obj.name.contains("病榻")
        || obj.name.contains("藥")
        || obj.name.contains("草")
        || obj.name.contains("榻")
        || obj.name.contains("床")
        || obj.name.contains("蒲團")
        || obj.name.contains("坐墊");

    if matches!(action, "Use" | "Operate" | "Rest" | "Meditate" | "Acquire") && is_medical_or_rest {
        let include_miasma = obj.name.contains("藥")
            || obj.name.contains("草")
            || obj.id.contains("herb");
        let cured = crate::pg::living_world::clear_entity_afflictions(pid, include_miasma, now);
        if !cured.is_empty() {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = 100;
                    e.qi = 100;
                });
            }
            let _ = db::update_vit(pid, 100);
            let cured_labels: Vec<&'static str> = cured.iter().map(|c| c.label()).collect();
            narrative.push_str(&format!(
                "\n【調理痊癒】經一番悉心敷藥與調息，你身上的「{}」已然痊癒，周身氣機重歸暢通。",
                cured_labels.join("、")
            ));
            push_refresh(conn, pid);
        }
    }

    if !obj.id.starts_with("wt_")
        && !obj.name.contains("紅印屋契")
        && !obj.name.contains("委託")
        && !obj.name.contains("告示")
        && !obj.name.contains("公佈欄")
        && !obj.name.contains("海事")
        && !obj.name.contains("海船")
        && !obj.name.contains("石櫃")
        && !obj.name.contains("烘衣櫥")
        && !obj.name.contains("田壟")
        && !obj.name.contains("苗畦")
        && !is_medical_or_rest
        && action != "Rest"
        && action != "Meditate"
    {
        return;
    }

    if apply_catalog_effects_part1(conn, pid, player_room, obj, action, now, narrative) {
        return;
    }
    if apply_catalog_effects_part2(conn, pid, player_room, obj, action, now, narrative) {
        return;
    }
    if apply_catalog_effects_part3(conn, pid, player_room, obj, action, now, narrative) {
        return;
    }
    if apply_catalog_effects_part4(conn, pid, player_room, obj, action, now, narrative) {
        return;
    }
    if apply_catalog_effects_part5(conn, pid, player_room, obj, action, now, narrative) {
        return;
    }
}
