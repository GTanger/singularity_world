/// 職司專屬器物與修整規格
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocationalVesselSpec {
    pub item_id: &'static str,
    pub catalog_id: &'static str,
    pub display_name: &'static str,
    pub is_bladed: bool,
    pub maintenance_venue_keyword: &'static str,
    pub maintenance_purpose_prefix: &'static str,
    pub maintenance_stay_verb: &'static str,
}

/// 剝離職銜字串末尾的個人姓名（如「合社獵手・林山海」→「合社獵手」），
/// 避免姓名中的「林、木、石、海、鐵、墨」字元誤觸其他職司分支。
pub fn strip_trailing_person_name_from_role(role_hint: &str) -> &str {
    if let Some((prefix, tail)) = role_hint.rsplit_once('・') {
        let p_trim = prefix.trim();
        let t_trim = tail.trim();
        let tail_len = t_trim.chars().count();
        let prefix_is_settlement = [
            "文曜", "水尾寮", "樟木驛", "風櫃尾", "烏石鼻", "老堰頭", "大圳頭", "半嶺鎮", "半嶺",
            "風舶嶼", "風舶角", "南哨", "三號坑", "鋸木溪", "竹林寨", "分家巷", "白塔",
        ]
        .iter()
        .any(|s| p_trim == *s || p_trim.ends_with(s));
        let tail_is_role = [
            "匠", "師", "夫", "手", "戶", "工", "主", "長", "事", "員", "生", "徒", "頭",
            "家", "商", "販", "醫", "廚", "衛", "哨", "卒", "客", "人", "官", "老", "婆",
            "嬸", "伯", "叔", "姑", "娘", "坊", "館", "行", "局", "堂", "所", "店", "攤",
            "站", "舍", "屋", "社", "隊", "者", "院", "書", "講", "席", "學", "閣", "齋",
            "樓", "鋪", "台", "廠", "場", "窯", "磨", "塢", "關", "港", "渡", "橋", "街",
            "巷", "司", "役", "丁", "農", "樵", "漁", "獵",
        ]
        .iter()
        .any(|kw| t_trim.contains(kw));
        if (2..=4).contains(&tail_len) && !prefix_is_settlement && !tail_is_role && !p_trim.is_empty() {
            return p_trim;
        }
    }
    role_hint
}

/// 依住民職銜與身分解析其真實隨身器物與保養場所（涵蓋全島 18 類職司，直接錨定 PENINSULA_WORLD_OBJECTS 總典外鍵）
pub fn resolve_vocational_vessel(role_hint: &str) -> VocationalVesselSpec {
    let role_hint = strip_trailing_person_name_from_role(role_hint);
    if role_hint.contains("學徒") {
        VocationalVesselSpec {
            item_id: "craft_wooden_tool_handle",
            catalog_id: "wt_cat_0989",
            display_name: "百工學徒手用小木尺",
            is_bladed: false,
            maintenance_venue_keyword: "工坊",
            maintenance_purpose_prefix: "入坊校刻木尺分寸",
            maintenance_stay_verb: "正以細砂紙打磨尺邊並核對手中",
        }
    } else if role_hint.contains("家眷")
        || role_hint.contains("內當家")
        || role_hint.contains("持家")
        || role_hint.contains("眷")
    {
        VocationalVesselSpec {
            item_id: "craft_bamboo_basket",
            catalog_id: "wt_cat_0895",
            display_name: "紫竹家常提籃",
            is_bladed: false,
            maintenance_venue_keyword: "市集",
            maintenance_purpose_prefix: "修整竹籃提耳",
            maintenance_stay_verb: "正用韌麻細繩纏緊手中",
        }
    } else if role_hint.contains("執事")
        || role_hint.contains("主事")
        || role_hint.contains("帳房")
        || role_hint.contains("商行")
        || role_hint.contains("公所")
        || role_hint.contains("簿")
        || role_hint.contains("調撥")
    {
        VocationalVesselSpec {
            item_id: "craft_copper_abacus",
            catalog_id: "wt_cat_0834",
            display_name: "紫銅商簿算盤",
            is_bladed: false,
            maintenance_venue_keyword: "商行",
            maintenance_purpose_prefix: "校訂銅算盤檔珠",
            maintenance_stay_verb: "正以軟布擦拭並撥校手中",
        }
    } else if role_hint.contains("食堂")
        || role_hint.contains("廚")
        || role_hint.contains("茶")
        || role_hint.contains("膳")
        || role_hint.contains("食")
        || role_hint.contains("掌事")
        || role_hint.contains("麵")
        || role_hint.contains("攤")
    {
        VocationalVesselSpec {
            item_id: "craft_kitchen_cleaver",
            catalog_id: "wt_cat_0564",
            display_name: "厚背熟鐵廚刀",
            is_bladed: true,
            maintenance_venue_keyword: "鐵",
            maintenance_purpose_prefix: "入坊磨利廚刀",
            maintenance_stay_verb: "正就著青石水磨細細推磨手中",
        }
    } else if (role_hint.contains("書")
        || role_hint.contains("書院")
        || role_hint.contains("學院")
        || (role_hint.contains("講") && !role_hint.contains("講堂街"))
        || role_hint.contains("印")
        || role_hint.contains("記")
        || role_hint.contains("學者")
        || role_hint.contains("學子")
        || role_hint.contains("算")
        || role_hint.contains("觀星"))
        && !role_hint.contains("學徒")
    {
        VocationalVesselSpec {
            item_id: "craft_peninsula_almanac",
            catalog_id: "wt_cat_0781",
            display_name: "白塔風物考工手札",
            is_bladed: false,
            maintenance_venue_keyword: "書",
            maintenance_purpose_prefix: "入齋補訂考工手札",
            maintenance_stay_verb: "正展卷謄錄並以棉線裝訂手中",
        }
    } else if role_hint.contains("藥")
        || role_hint.contains("醫")
        || role_hint.contains("草")
        || role_hint.contains("丹")
        || role_hint.contains("診")
    {
        VocationalVesselSpec {
            item_id: "craft_meridian_salve",
            catalog_id: "wt_cat_1057",
            display_name: "溫氏導能溫石百草溫灸盒",
            is_bladed: false,
            maintenance_venue_keyword: "診",
            maintenance_purpose_prefix: "入堂拭淨溫灸銅盒",
            maintenance_stay_verb: "正以山泉洗淨藥漬並擦亮手中",
        }
    } else if role_hint.contains("水輪")
        || role_hint.contains("石磨")
        || role_hint.contains("磨坊")
        || role_hint.contains("碾")
    {
        VocationalVesselSpec {
            item_id: "wt_cat_0973",
            catalog_id: "wt_cat_0973",
            display_name: "平底石磨分粉篩架",
            is_bladed: false,
            maintenance_venue_keyword: "石磨",
            maintenance_purpose_prefix: "入坊校準石磨齒距與分粉銅網篩架",
            maintenance_stay_verb: "正調校水輪分動木齒並繃緊手中",
        }
    } else if role_hint.contains("鐵")
        || role_hint.contains("鍛")
        || role_hint.contains("爐")
        || role_hint.contains("鑄")
    {
        VocationalVesselSpec {
            item_id: "craft_iron_pickaxe",
            catalog_id: "wt_cat_0945",
            display_name: "岑氏省力導能鍛錘",
            is_bladed: false,
            maintenance_venue_keyword: "鐵",
            maintenance_purpose_prefix: "入坊重楔鐵錘木柄",
            maintenance_stay_verb: "正於砧台校平錘面並楔緊手中",
        }
    } else if role_hint.contains("礦")
        || role_hint.contains("石")
        || role_hint.contains("坑")
        || role_hint.contains("鑿")
    {
        VocationalVesselSpec {
            item_id: "craft_iron_pickaxe",
            catalog_id: "wt_cat_0964",
            display_name: "雙刃碎石手鎬",
            is_bladed: true,
            maintenance_venue_keyword: "鐵",
            maintenance_purpose_prefix: "入坊淬火修整手鎬",
            maintenance_stay_verb: "正就著鍛爐火色重新淬硬手中",
        }
    } else if role_hint.contains("詩")
        || role_hint.contains("唱和")
        || role_hint.contains("講堂")
        || role_hint.contains("曬書")
        || role_hint.contains("書院")
        || role_hint.contains("藏書")
    {
        VocationalVesselSpec {
            item_id: "craft_peninsula_almanac",
            catalog_id: "wt_cat_0890",
            display_name: "青銅鎮紙尺",
            is_bladed: false,
            maintenance_venue_keyword: "書",
            maintenance_purpose_prefix: "入館拭淨鎮紙銅尺",
            maintenance_stay_verb: "正以細絨軟布拭淨尺面微塵並擦拭手中",
        }
    } else if role_hint.contains("木")
        || role_hint.contains("竹")
        || role_hint.contains("鋸")
        || role_hint.contains("船")
        || role_hint.contains("刨")
    {
        VocationalVesselSpec {
            item_id: "craft_wooden_tool_handle",
            catalog_id: "wt_cat_0954",
            display_name: "魯班平推刨刀",
            is_bladed: true,
            maintenance_venue_keyword: "木",
            maintenance_purpose_prefix: "入坊校準平推刨刃",
            maintenance_stay_verb: "正退下木楔、在細砥石上推磨手中",
        }
    } else if role_hint.contains("織")
        || role_hint.contains("布")
        || role_hint.contains("服")
        || role_hint.contains("染")
        || role_hint.contains("縫")
    {
        VocationalVesselSpec {
            item_id: "craft_steel_scissors",
            catalog_id: "wt_cat_0890",
            display_name: "熟鐵裁布大剪刀",
            is_bladed: true,
            maintenance_venue_keyword: "鐵",
            maintenance_purpose_prefix: "入坊校緊裁布剪軸",
            maintenance_stay_verb: "正滴入茶油並合刃試剪手中",
        }
    } else if role_hint.contains("靴")
        || role_hint.contains("鞋")
        || role_hint.contains("傘")
        || role_hint.contains("皮")
        || role_hint.contains("革")
    {
        VocationalVesselSpec {
            item_id: "wt_cat_1021",
            catalog_id: "wt_cat_1021",
            display_name: "彎頭修靴裁皮刀",
            is_bladed: true,
            maintenance_venue_keyword: "鐵",
            maintenance_purpose_prefix: "入坊細磨裁皮刀刃",
            maintenance_stay_verb: "正就著細油石推磨並試削手中",
        }
    } else if role_hint.contains("理髮")
        || role_hint.contains("剪剃")
        || role_hint.contains("剃")
    {
        VocationalVesselSpec {
            item_id: "craft_steel_scissors",
            catalog_id: "wt_cat_0642",
            display_name: "夾耳防脫修面剪刀",
            is_bladed: true,
            maintenance_venue_keyword: "鐵",
            maintenance_purpose_prefix: "入坊細磨剃刀鋒口",
            maintenance_stay_verb: "正於盪刀布上來回推亮手中",
        }
    } else if role_hint.contains("漁")
        || role_hint.contains("港")
        || role_hint.contains("渡")
        || role_hint.contains("海")
        || role_hint.contains("舵")
    {
        VocationalVesselSpec {
            item_id: "craft_pine_torch",
            catalog_id: "wt_cat_1006",
            display_name: "老青竹黃銅輪捲線釣竿",
            is_bladed: false,
            maintenance_venue_keyword: "渡",
            maintenance_purpose_prefix: "入埠重編浸油釣線",
            maintenance_stay_verb: "正以松脂塗抹竿身並換綁手中",
        }
    } else if role_hint.contains("巡")
        || role_hint.contains("衛")
        || role_hint.contains("哨")
        || role_hint.contains("武")
        || role_hint.contains("教頭")
    {
        VocationalVesselSpec {
            item_id: "craft_steel_hunting_knife",
            catalog_id: "wt_cat_1021",
            display_name: "精鍛步巡短刀",
            is_bladed: true,
            maintenance_venue_keyword: "鐵",
            maintenance_purpose_prefix: "入坊淬火磨利佩刀",
            maintenance_stay_verb: "正就著爐火與磨石修整手中",
        }
    } else if role_hint.contains("獵")
        || role_hint.contains("採")
        || role_hint.contains("樵")
    {
        VocationalVesselSpec {
            item_id: "craft_steel_hunting_knife",
            catalog_id: "wt_cat_1021",
            display_name: "精鍛防身獵刀",
            is_bladed: true,
            maintenance_venue_keyword: "鐵",
            maintenance_purpose_prefix: "入坊淬火磨利獵刀",
            maintenance_stay_verb: "正就著爐火與磨石修整手中",
        }
    } else if role_hint.contains("行旅")
        || role_hint.contains("押貨")
        || role_hint.contains("挑夫")
    {
        VocationalVesselSpec {
            item_id: "craft_bamboo_basket",
            catalog_id: "wt_cat_0894",
            display_name: "內襯寒水玉保鮮採集背簍",
            is_bladed: false,
            maintenance_venue_keyword: "驛",
            maintenance_purpose_prefix: "入站綁固行商背簍",
            maintenance_stay_verb: "正重新收緊牛皮肩帶並檢查手中",
        }
    } else {
        VocationalVesselSpec {
            item_id: "craft_bamboo_basket",
            catalog_id: "wt_cat_0895",
            display_name: "雙層竹編提籃",
            is_bladed: false,
            maintenance_venue_keyword: "市集",
            maintenance_purpose_prefix: "整理隨身提籃",
            maintenance_stay_verb: "正低頭收拾並拍淨手中",
        }
    }
}

/// 抵達場所後的延遲結算事件（取代過往每次抵達 `std::thread::spawn` 搶奪 `Store::write`）
#[derive(Debug, Clone)]
pub enum ArrivalEffect {
    ClinicRecovered {
        entity_id: String,
        dest_name: String,
        recovered_label: String,
        now_unix: i64,
    },
    VesselMaintained {
        entity_id: String,
        dest_name: String,
        vessel_name: String,
        now_unix: i64,
    },
    WildernessForaged {
        entity_id: String,
        dest_name: String,
        purpose: String,
        streak: u8,
        now_unix: i64,
    },
    DinedAtVenue {
        entity_id: String,
        dest_name: String,
        purpose: String,
        now_unix: i64,
    },
    BathedAtVenue {
        entity_id: String,
        now_unix: i64,
    },
    CompletedWorkShift {
        entity_id: String,
        dest_name: String,
        purpose: String,
        earn: i32,
        streaks: u8,
        now_unix: i64,
    },
    PruneStaleThoughts {
        entity_id: String,
        now_unix: i64,
    },
    CustomThoughtAndMg {
        entity_id: String,
        delta_mg: i32,
        delta_disp: i32,
        thought: String,
        now_unix: i64,
    },
    RestockShopItems {
        seller_id: String,
    },
    ActionSettlement {
        entity_id: String,
        settlement: String,
        dest_name: String,
        action_code: String,
        now_unix: i64,
    },
}

static PENDING_ARRIVAL_EFFECTS: LazyLock<Mutex<Vec<ArrivalEffect>>> =
    LazyLock::new(|| Mutex::new(Vec::with_capacity(256)));

/// 將抵達結算事件推入無鎖競爭佇列，由模擬迴圈單次批次寫入
pub fn enqueue_arrival_effect(effect: ArrivalEffect) {
    if let Ok(mut q) = PENDING_ARRIVAL_EFFECTS.lock() {
        q.push(effect);
    }
}

/// 批次排空並結算所有待處理的抵達事件（僅獲取一次 `Store::write`，徹底消除執行緒風暴）
pub fn drain_pending_arrival_effects() -> usize {
    let needs_boot_norm = !ECONOMY_NORMALIZED_ONCE.load(Ordering::SeqCst);
    let batch: Vec<ArrivalEffect> = {
        let Ok(mut q) = PENDING_ARRIVAL_EFFECTS.lock() else {
            return 0;
        };
        if q.is_empty() && !needs_boot_norm {
            return 0;
        }
        std::mem::take(&mut *q)
    };
    let count = batch.len();

    // 先在 Store 寫鎖之外完成 DB/生態庫存副作用
    for eff in &batch {
        match eff {
            ArrivalEffect::WildernessForaged {
                entity_id, ..
            } => {
                let work_ctx = if entity_id.starts_with("npc_") {
                    crate::store::wenyao_household_slot_for(entity_id).work_name
                } else {
                    ""
                };
                let (picked, _) =
                    crate::game::object_ecology::ItemPhysicalClass::canonical_vocation_supply_item(
                        work_ctx,
                    );
                if !crate::db::inventory_has_qty(entity_id, picked, 2) {
                    let _ = crate::db::add_to_inventory(entity_id, picked, 1);
                }
                record_settlement_market_production("文曜", picked, 1);
            }
            ArrivalEffect::CompletedWorkShift {
                entity_id,
                dest_name,
                now_unix,
                ..
            } => {
                let single_id = [entity_id.clone()];
                let _ = crate::game::object_ecology::run_npc_autonomous_production_step(
                    "",
                    dest_name,
                    &single_id,
                    *now_unix,
                );
            }
            ArrivalEffect::RestockShopItems { seller_id } => {
                crate::server::action::trade::restock_shop_items_for_seller(seller_id);
            }
            _ => {}
        }
    }

    // 單次取得 Store 寫鎖，批次更新所有抵達實體之鎂幣、念頭與心境
    if let Some(arc) = crate::store::get_store() {
        let mut s = arc.write();
        normalize_and_seed_circular_economy_once(&mut s);
        for eff in batch {
            match eff {
                ArrivalEffect::ClinicRecovered {
                    entity_id,
                    dest_name,
                    recovered_label,
                    now_unix,
                } => {
                    let _ = s.update_entity(&entity_id, |ent| {
                        crate::db::add_thought_to_entity_with_flavor(
                            ent,
                            crate::db::ThoughtSource::Rested,
                            now_unix,
                            format!(
                                "在{}給醫工敷過草藥，原本「{}」的痛楚消減大半，手腳重新利索起來",
                                dest_name, recovered_label
                            ),
                        );
                        ent.disposition = (ent.disposition + 10).clamp(-100, 100);
                    });
                }
                ArrivalEffect::VesselMaintained {
                    entity_id,
                    dest_name,
                    vessel_name,
                    now_unix,
                } => {
                    let _ = s.update_entity(&entity_id, |ent| {
                        let phys_class = crate::game::object_ecology::ItemPhysicalClass::classify(
                            "",
                            &vessel_name,
                        );
                        let action_phrase = phys_class.maintenance_action_phrase();
                        let thought_text = format!(
                            "在{dest_name}把隨身的「{vessel_name}」{action_phrase}，器況煥然一新，幹起活來順手多了"
                        );
                        crate::db::add_thought_to_entity_with_flavor(
                            ent,
                            crate::db::ThoughtSource::WorkDone,
                            now_unix,
                            thought_text,
                        );
                    });
                }
                ArrivalEffect::WildernessForaged {
                    entity_id,
                    dest_name,
                    purpose,
                    now_unix,
                    ..
                } => {
                    let _ = s.update_entity(&entity_id, |ent| {
                        crate::db::add_thought_to_entity_with_delta(
                            ent,
                            crate::db::ThoughtSource::WorkDone,
                            6,
                            now_unix,
                            format!(
                                "在{}一帶完成一輪「{}」，行囊裡添了新採獲的山海物產，繼續察看前路動靜",
                                dest_name, purpose
                            ),
                        );
                    });
                }
                ArrivalEffect::DinedAtVenue {
                    entity_id,
                    dest_name,
                    purpose,
                    now_unix,
                } => {
                    let _ = s.update_entity(&entity_id, |ent| {
                        let meal_flavor = if dest_name.contains("公井")
                            || purpose.contains("冷水")
                            || purpose.contains("冷開水")
                            || purpose.contains("水筒")
                        {
                            format!("在{}飲過清冽甘水，喉間乾渴頓消，精神一振", dest_name)
                        } else if purpose.contains("乾糧") {
                            format!("在{}就著隨身竹筒溫水細嚼乾糧果腹，肚裡有了底", dest_name)
                        } else if find_predefined_action_by_title(&purpose)
                            .is_some_and(|opt| opt.image == DecisionImageKind::Drink)
                            || purpose.contains("茶")
                            || purpose.contains("酒")
                            || purpose.contains("共飲")
                            || purpose.contains("清茗")
                        {
                            format!("在{}飲過茶酒歇腳，喉間舒爽，燥意盡消", dest_name)
                        } else {
                            format!("在{}用過熱騰騰的茶飯，肚裡暖和舒坦", dest_name)
                        };
                        crate::db::add_thought_to_entity_with_flavor(
                            ent,
                            crate::db::ThoughtSource::Ate,
                            now_unix,
                            meal_flavor,
                        );
                        ent.disposition = (ent.disposition + 8).clamp(-100, 100);
                    });
                }
                ArrivalEffect::BathedAtVenue {
                    entity_id,
                    now_unix,
                } => {
                    let _ = s.update_entity(&entity_id, |ent| {
                        crate::db::add_thought_to_entity_with_flavor(
                            ent,
                            crate::db::ThoughtSource::Rested,
                            now_unix,
                            "在溫水浴場泡過熱湯，一身疲乏盡數散去",
                        );
                        ent.disposition = (ent.disposition + 10).clamp(-100, 100);
                    });
                }
                ArrivalEffect::CompletedWorkShift {
                    entity_id,
                    dest_name,
                    purpose,
                    earn,
                    streaks,
                    now_unix,
                } => {
                    let _ = s.update_entity(&entity_id, |ent| {
                        let is_rest_purpose = find_predefined_action_by_title(&purpose)
                            .map(|opt| opt.image == DecisionImageKind::SleepOrRest)
                            .unwrap_or_else(|| {
                                purpose.contains("歇息")
                                    || purpose.contains("小憩")
                                    || purpose.contains("便當")
                                    || purpose.contains("喘口氣")
                                    || purpose.contains("閉目")
                                    || purpose.contains("泡湯")
                                    || purpose.contains("沐浴")
                                    || purpose.contains("納涼")
                                    || purpose.contains("用膳")
                                    || purpose.contains("吃熱食")
                                    || purpose.contains("睡一覺")
                                    || purpose.contains("熟睡")
                                    || purpose.contains("調息")
                                    || purpose.contains("擦汗歇腿")
                            });
                        if is_rest_purpose {
                            let clean_purpose = purpose
                                .strip_prefix("回屋舍")
                                .or_else(|| purpose.strip_prefix("回居所"))
                                .or_else(|| purpose.strip_prefix("尋處"))
                                .unwrap_or(&purpose);
                            crate::db::add_thought_to_entity_with_flavor(
                                ent,
                                crate::db::ThoughtSource::Rested,
                                now_unix,
                                format!(
                                    "在{}{}過後，緊繃的肩背舒緩了許多，神清氣爽",
                                    dest_name, clean_purpose
                                ),
                            );
                        } else if streaks >= 2 {
                            crate::db::add_thought_to_entity_with_delta(
                                ent,
                                crate::db::ThoughtSource::Daily,
                                -4,
                                now_unix,
                                format!(
                                    "在{}連軸忙完「{}」，肩背有些痠沉，想找地方歇口氣喝杯熱茶",
                                    dest_name, purpose
                                ),
                            );
                        } else if earn > 0 {
                            crate::db::add_thought_to_entity_with_delta(
                                ent,
                                crate::db::ThoughtSource::WorkDone,
                                6,
                                now_unix,
                                format!(
                                    "在{}完成「{}」並結清帳目，手頭進帳 {} 枚鎂幣，心裡踏實",
                                    dest_name, purpose, earn
                                ),
                            );
                        }
                    });
                }
                ArrivalEffect::PruneStaleThoughts {
                    entity_id,
                    now_unix,
                } => {
                    let _ = s.update_entity(&entity_id, |ent| {
                        if ent.magnesium >= 35 {
                            ent.thoughts.retain(|t| {
                                !t.flavor.as_deref().unwrap_or("").contains("鎂袋空了")
                            });
                        }
                        let mut kept_daily = 0;
                        ent.thoughts.retain(|t| {
                            if t.flavor.as_deref().unwrap_or("").contains("今日花用又見底")
                            {
                                kept_daily += 1;
                                kept_daily <= 2
                            } else {
                                true
                            }
                        });
                        crate::db::clean_and_recalc_entity_disposition(ent, now_unix);
                    });
                }
                ArrivalEffect::CustomThoughtAndMg {
                    entity_id,
                    delta_mg,
                    delta_disp,
                    thought,
                    now_unix,
                } => {
                    if delta_mg > 0 {
                        pay_from_settlement_treasury(&mut s, &entity_id, "文曜", delta_mg);
                    } else if delta_mg < 0 {
                        spend_into_settlement_circulation(
                            &mut s,
                            &entity_id,
                            "文曜",
                            "市集",
                            (-delta_mg) as i32,
                            false,
                        );
                    }
                    let _ = s.update_entity(&entity_id, |ent| {
                        if delta_disp != 0 {
                            ent.disposition = (ent.disposition + delta_disp).clamp(-100, 100);
                        }
                        if !thought.is_empty() {
                            crate::db::add_thought_to_entity_with_delta(
                                ent,
                                crate::db::ThoughtSource::Daily,
                                delta_disp,
                                now_unix,
                                thought,
                            );
                        }
                    });
                }
                ArrivalEffect::RestockShopItems { .. } => {}
                ArrivalEffect::ActionSettlement {
                    entity_id,
                    settlement,
                    dest_name,
                    action_code,
                    now_unix,
                } => {
                    let net = execute_conserved_action_settlement(
                        &mut s,
                        &entity_id,
                        &settlement,
                        &dest_name,
                        &action_code,
                        now_unix,
                    );
                    if net != 0 {
                        let title = lookup_action_option(&action_code)
                            .map(|o| o.action_title)
                            .unwrap_or("當下事務");
                        let _ = s.update_entity(&entity_id, |ent| {
                            if net > 0 {
                                crate::db::add_thought_to_entity_with_delta(
                                    ent,
                                    crate::db::ThoughtSource::WorkDone,
                                    5,
                                    now_unix,
                                    format!(
                                        "在{}完成「{}」並結清帳目，手頭進帳 {} 枚鎂幣，心裡踏實",
                                        dest_name, title, net
                                    ),
                                );
                            } else {
                                crate::db::add_thought_to_entity_with_delta(
                                    ent,
                                    crate::db::ThoughtSource::Daily,
                                    2,
                                    now_unix,
                                    format!(
                                        "在{}完成「{}」，花費 {} 枚鎂幣打點所需",
                                        dest_name, title, -net
                                    ),
                                );
                            }
                        });
                    }
                }
            }
        }
    }
    count
}
