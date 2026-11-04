//! 活世界 NPC 四核實用因果與體感引擎（Causal & Somatic Core）
//!
//! 1. **十八職司器物與修整場所對齊（Vocational Vessel Alignment）**：解決全島 1,772 名住民混用獵刀的錯位。
//! 2. **無鎖批次抵達佇列（Lock-Free Arrival Effect Queue）**：取代每次抵達 `std::thread::spawn` 搶寫鎖。
//! 3. **性格錨點（Cornerstone）＋壓力閾值（Stress）＋三步因果計畫鏈（MicroPlanQueue）＋信念驚奇（Belief ΔE）＋人情帳本（Relational Hooks）**。
//! 4. **按需十層體感微動作合成器（On-Demand 10-Layer Somatic Renderer）**：僅於觀測或 API 查詢時即時渲染，背景零字串浪費。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, RwLock};

/// 職司專屬器物與修整規格
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocationalVesselSpec {
    pub item_id: &'static str,
    pub display_name: &'static str,
    pub is_bladed: bool,
    pub maintenance_venue_keyword: &'static str,
    pub maintenance_purpose_prefix: &'static str,
    pub maintenance_stay_verb: &'static str,
}

/// 依住民職銜與身分解析其真實隨身器物與保養場所（涵蓋全島 18 類職司）
pub fn resolve_vocational_vessel(role_hint: &str) -> VocationalVesselSpec {
    if role_hint.contains("學徒") {
        VocationalVesselSpec {
            item_id: "craft_wooden_tool_handle",
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
            display_name: "厚背熟鐵廚刀",
            is_bladed: true,
            maintenance_venue_keyword: "鐵",
            maintenance_purpose_prefix: "入坊磨利廚刀",
            maintenance_stay_verb: "正就著青石水磨細細推磨手中",
        }
    } else if (role_hint.contains("書")
        || role_hint.contains("書院")
        || role_hint.contains("學院")
        || role_hint.contains("講")
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
            display_name: "青銅搗藥杵",
            is_bladed: false,
            maintenance_venue_keyword: "診",
            maintenance_purpose_prefix: "入堂拭淨搗藥銅杵",
            maintenance_stay_verb: "正以山泉洗淨藥漬並擦亮手中",
        }
    } else if role_hint.contains("水輪")
        || role_hint.contains("石磨")
        || role_hint.contains("磨坊")
        || role_hint.contains("碾")
    {
        VocationalVesselSpec {
            item_id: "wt_cat_0973",
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
            display_name: "百鍛長柄鐵錘",
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
            display_name: "雙刃碎石手鎬",
            is_bladed: true,
            maintenance_venue_keyword: "鐵",
            maintenance_purpose_prefix: "入坊淬火修整手鎬",
            maintenance_stay_verb: "正就著鍛爐火色重新淬硬手中",
        }
    } else if role_hint.contains("木")
        || role_hint.contains("竹")
        || role_hint.contains("鋸")
        || role_hint.contains("船")
        || role_hint.contains("刨")
    {
        VocationalVesselSpec {
            item_id: "craft_wooden_tool_handle",
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
            display_name: "熟鐵裁布大剪刀",
            is_bladed: true,
            maintenance_venue_keyword: "鐵",
            maintenance_purpose_prefix: "入坊校緊裁布剪軸",
            maintenance_stay_verb: "正滴入茶油並合刃試剪手中",
        }
    } else if role_hint.contains("漁")
        || role_hint.contains("港")
        || role_hint.contains("渡")
        || role_hint.contains("海")
        || role_hint.contains("舵")
    {
        VocationalVesselSpec {
            item_id: "craft_pine_torch",
            display_name: "浸油麻編海釣竿",
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
            display_name: "浸油防雨行商背簍",
            is_bladed: false,
            maintenance_venue_keyword: "驛",
            maintenance_purpose_prefix: "入站綁固行商背簍",
            maintenance_stay_verb: "正重新收緊牛皮肩帶並檢查手中",
        }
    } else {
        VocationalVesselSpec {
            item_id: "craft_bamboo_basket",
            display_name: "紫竹日常提籃",
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
                entity_id, streak, ..
            } => {
                let item_choices = [
                    "mat_foraged_herb",
                    "mat_timber_log",
                    "mat_rough_ore",
                    "mat_hemp_rope_roll",
                ];
                let picked = item_choices[(*streak as usize) % item_choices.len()];
                let _ = crate::db::add_to_inventory(entity_id, picked, 1);
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
                        let thought_text = if dest_name.contains("井")
                            || dest_name.contains("泉")
                            || dest_name.contains("溪")
                            || ["籃", "簍", "袋", "桶", "網"].iter().any(|k| vessel_name.contains(k))
                        {
                            format!(
                                "在{}把隨身的「{}」洗淨擦拭、重新紮緊，器況煥然一新，用起來順手多了",
                                dest_name, vessel_name
                            )
                        } else {
                            format!(
                                "在{}把隨身的「{}」細細修整校準，器況煥然一新，幹起活來順手多了",
                                dest_name, vessel_name
                            )
                        };
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
fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// 先天心性六向單純形展開（Schwartz 圓周對偶偏置，總和恆守恆 600，各向 45..=165）
/// 順序：[Pride(傲骨), Care(護犢), Honor(重義), Profit(嗜利), Craft(匠心), Order(守序)]
pub fn innate_personality_simplex(seed: u64) -> [i32; 6] {
    let mut sm = seed ^ 0xa076_1d64_78bd_642f;
    let u1 = splitmix64_next(&mut sm);
    let u2 = splitmix64_next(&mut sm);
    let theta1 = (u1 as f64 / u64::MAX as f64) * std::f64::consts::TAU;
    let theta2 = (u2 as f64 / u64::MAX as f64) * std::f64::consts::TAU;

    let (s1, c1) = theta1.sin_cos();
    let c2 = theta2.cos();

    // Schwartz 圓周對偶偏置（Care/Honor 與 Profit/Pride 對蹠，Order 與 Craft 對蹠）
    let coupled = [
        -35.0 * c1 + 15.0 * s1, // Pride
         35.0 * c1 + 15.0 * s1, // Care
         35.0 * c1 - 15.0 * s1, // Honor
        -35.0 * c1 - 15.0 * s1, // Profit
         40.0 * c2,             // Craft
        -40.0 * c2,             // Order
    ];

    let mut vars = [0.0f64; 6];
    let mut var_sum = 0.0f64;
    for v in &mut vars {
        let uv = splitmix64_next(&mut sm);
        *v = ((uv % 1000) as f64 / 1000.0 - 0.5) * 30.0;
        var_sum += *v;
    }
    let mean_v = var_sum / 6.0;

    let mut cur = [0.0f64; 6];
    for k in 0..6 {
        cur[k] = (100.0 + coupled[k] + (vars[k] - mean_v)).clamp(45.0, 165.0);
    }

    // Bounded simplex water-filling 投影 onto 600.0
    for _ in 0..12 {
        let sum: f64 = cur.iter().sum();
        let diff = 600.0 - sum;
        if diff.abs() < 1e-9 {
            break;
        }
        let mut free_count = 0usize;
        for &v in &cur {
            if (diff > 0.0 && v < 165.0 - 1e-5) || (diff < 0.0 && v > 45.0 + 1e-5) {
                free_count += 1;
            }
        }
        if free_count == 0 {
            break;
        }
        let delta = diff / (free_count as f64);
        for v in &mut cur {
            if (diff > 0.0 && *v < 165.0 - 1e-5) || (diff < 0.0 && *v > 45.0 + 1e-5) {
                *v = (*v + delta).clamp(45.0, 165.0);
            }
        }
    }

    let mut int_b = [0i32; 6];
    for (i, v) in cur.iter().enumerate() {
        int_b[i] = (v.round() as i32).clamp(45, 165);
    }

    // 餘數校準至嚴格 600
    let mut rem = 600 - int_b.iter().sum::<i32>();
    while rem != 0 {
        let step = if rem > 0 { 1 } else { -1 };
        let mut best_i = None;
        let mut best_val = if rem > 0 { i32::MIN } else { i32::MAX };
        for (i, &val) in int_b.iter().enumerate() {
            if rem > 0 && val < 165 {
                if val > best_val {
                    best_val = val;
                    best_i = Some(i);
                }
            } else if rem < 0 && val > 45 {
                if val < best_val {
                    best_val = val;
                    best_i = Some(i);
                }
            }
        }
        let target_idx = best_i.unwrap_or_else(|| {
            int_b.iter().position(|&v| (rem > 0 && v < 165) || (rem < 0 && v > 45)).unwrap_or(0)
        });
        int_b[target_idx] += step;
        rem -= step;
    }

    int_b
}

/// 四大典型生活處境槽位（CAPS If-Then 處境簽名矩陣）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PsychSituationSlot {
    ScarcitySurvival = 0,
    VocationalDuty = 1,
    RelationalEncounter = 2,
    HighStressCrisis = 3,
}

/// 計算處境觸發相的零和位移量（各向 [-32, 32]，總和恆為 0）
pub fn situational_if_then_offset(seed: u64, slot: PsychSituationSlot) -> [i32; 6] {
    let slot_idx = slot as usize;
    // 6 facets: [Pride, Care, Honor, Profit, Craft, Order]
    let base: [i32; 6] = match slot {
        PsychSituationSlot::ScarcitySurvival => [2, 4, -4, 8, -4, -6],
        PsychSituationSlot::VocationalDuty => [2, -4, -6, -6, 8, 6],
        PsychSituationSlot::RelationalEncounter => [-4, 6, 8, -6, -2, -2],
        PsychSituationSlot::HighStressCrisis => [8, 6, -4, -2, -6, -2],
    };

    let mut sm = (seed.wrapping_mul(0x517c_c1b7_2722_0a95))
        ^ ((slot_idx as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(0xbf58_476d_1ce4_e5b9));

    let mut offsets = [0i32; 6];
    for k in 0..6 {
        let u = splitmix64_next(&mut sm);
        let var = ((u % 29) as i32) - 14;
        offsets[k] = (base[k] + var).clamp(-28, 28);
    }

    // 嚴格校準和至 0（各元素在 [-32, 32] 內）
    let mut sum: i32 = offsets.iter().sum();
    while sum != 0 {
        let step = if sum > 0 { -1 } else { 1 };
        let mut best_i = None;
        let mut best_dist = -1i32;
        for (i, &val) in offsets.iter().enumerate() {
            if step > 0 && val < 32 {
                let dist = 32 - val;
                if dist > best_dist {
                    best_dist = dist;
                    best_i = Some(i);
                }
            } else if step < 0 && val > -32 {
                let dist = val - (-32);
                if dist > best_dist {
                    best_dist = dist;
                    best_i = Some(i);
                }
            }
        }
        let target_idx = match best_i {
            Some(i) => i,
            None => break,
        };
        offsets[target_idx] += step;
        sum += step;
    }

    offsets
}

/// 計算後天「所學 × 所熟 × 所歷」性格塑造偏移量
pub fn compute_experiential_personality_shift(
    settlement: &str,
    same_task_repetitions: u32,
    hooks: &[RelationalHook],
    seed: u64,
) -> [i32; 6] {
    let mut shift = [0i32; 6];
    // 0: Pride, 1: Care, 2: Honor, 3: Profit, 4: Craft, 5: Order

    // 1. 所學（聚落風土道統，<= 8 分）
    if settlement.contains("文曜") {
        shift[5] += 6; // Order +6
        shift[4] += 4; // Craft +4
    } else if settlement.contains("聖塔") {
        shift[4] += 6; // Craft +6
        shift[0] += 4; // Pride +4
    } else if settlement.contains("南哨") {
        shift[2] += 6; // Honor +6
        shift[1] += 4; // Care +4
    } else if settlement.contains("風舶") {
        shift[3] += 6; // Profit +6
        shift[2] += 4; // Honor +4
    } else if settlement.contains("青林") {
        shift[1] += 6; // Care +6
        shift[4] += 4; // Craft +4
    } else if settlement.contains("白沙") {
        shift[0] += 6; // Pride +6
        shift[3] += 4; // Profit +4
    }

    // 2. 所熟（日課工序慣性冪律）
    let innate = innate_personality_simplex(seed);
    let mut top_idx = 0;
    let mut top_val = innate[0];
    for (i, &v) in innate.iter().enumerate().skip(1) {
        if v > top_val {
            top_val = v;
            top_idx = i;
        }
    }
    let habit_bonus = ((10.0 * ((same_task_repetitions.max(1) as f64) / 25.0).powf(0.45)) as i32).clamp(2, 28);
    shift[top_idx] += habit_bonus;

    // 3. 所歷（恩怨、償清與託付之身心刻痕）
    for h in hooks {
        let kl = &h.kind_label;
        let sm = &h.summary;
        let matches_any = |terms: &[&str]| terms.iter().any(|t| kl.contains(t) || sm.contains(t));

        if matches_any(&["恩", "守諾", "解囊", "馳援", "完修"]) {
            shift[1] += 8;  // Care += 8
            shift[2] += 10; // Honor += 10
            shift[3] -= 4;  // Profit -= 4
        }
        if matches_any(&["遭拒", "追討", "結怨", "嫌隙", "展期"]) {
            if (seed & 1) == 0 {
                shift[0] += 10; // Pride += 10
                shift[3] += 6;  // Profit += 6
                shift[1] -= 4;  // Care -= 4
            } else {
                shift[3] += 10; // Profit += 10
                shift[5] += 6;  // Order += 6
                shift[2] -= 4;  // Honor -= 4
            }
        }
        if matches_any(&["託付", "代辦", "立契"]) {
            shift[5] += 7; // Order += 7
            shift[4] += 7; // Craft += 7
        }
    }

    shift
}

/// 住民性格錨點（Cornerstone，決定高壓破防出口、不可妥協底線與行動剪枝）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cornerstone {
    /// 傲骨：寧折不彎，不肯低聲賒欠或受人冷眼，遇壓價或吃閉門羹則轉頭自謀生路
    Pride,
    /// 護犢：先顧家眷與晚輩學徒溫飽，甘願替徒弟與家人扛下損耗
    Care,
    /// 重義：欠恩必償、見同袍有難必先馳援，合社獵團與街坊有難絕不退縮
    Honor,
    /// 嗜利：精算行情價差與囤貨時機，見市集急缺必搶先跨區套利
    Profit,
    /// 匠心：對器物火候與分寸極度執拗，器況稍有磨損必先校準妥貼才肯開工
    Craft,
    /// 守序：嚴守時刻、公所條例與巡防交接，帳目與差事一絲不苟
    Order,
}

impl Cornerstone {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pride => "傲骨",
            Self::Care => "護犢",
            Self::Honor => "重義",
            Self::Profit => "嗜利",
            Self::Craft => "匠心",
            Self::Order => "守序",
        }
    }

    pub fn from_role_and_seed(role_hint: &str, seed: u64) -> Self {
        let mut b = innate_personality_simplex(seed);
        // 溫和的職業性向引導（<= 8 分，絕非鎖死宿命論，重義之商與嗜利之衛皆能自然湧現）
        if role_hint.contains("家眷") || role_hint.contains("內當家") || role_hint.contains("持家") {
            b[1] += 8; // Care
            b[5] += 4; // Order
        } else if role_hint.contains("商")
            || role_hint.contains("主事")
            || role_hint.contains("執事")
            || role_hint.contains("帳房")
            || role_hint.contains("行旅")
        {
            b[3] += 8; // Profit
            b[5] += 4; // Order
        } else if role_hint.contains("鐵")
            || role_hint.contains("鍛")
            || role_hint.contains("木工")
            || role_hint.contains("木匠")
            || role_hint.contains("木器")
            || role_hint.contains("竹")
            || role_hint.contains("書")
            || role_hint.contains("醫")
        {
            b[4] += 8; // Craft
            b[0] += 4; // Pride
        } else if role_hint.contains("巡") || role_hint.contains("衛") || role_hint.contains("哨") {
            b[2] += 8; // Honor
            b[5] += 4; // Order
        } else if role_hint.contains("獵") || role_hint.contains("採") || role_hint.contains("樵") {
            b[0] += 6; // Pride
            b[4] += 6; // Craft
        }

        let mut max_idx = 0;
        let mut max_val = b[0];
        for (i, &val) in b.iter().enumerate().skip(1) {
            if val > max_val {
                max_val = val;
                max_idx = i;
            }
        }
        match max_idx {
            0 => Self::Pride,
            1 => Self::Care,
            2 => Self::Honor,
            3 => Self::Profit,
            4 => Self::Craft,
            _ => Self::Order,
        }
    }

    /// 當壓力超過閾值（`stress >= 72`）時，依性格錨點產出截然不同的破防轉折行動
    pub fn stress_break_directive(self, settlement: &str, vessel_name: &str) -> (&'static str, String, String) {
        let s = if settlement.trim().is_empty() { "文曜" } else { settlement.trim() };
        match self {
            Self::Pride => (
                "拂袖離場獨飲散悶",
                format!("連軸奔波又逢瑣事掣肘，傲骨難耐，索性收起「{vessel_name}」赴{s}茶寮獨飲散悶"),
                "眉宇微凜、將隨身器物往案上一頓，獨自斟茶平復胸中鬱氣".to_string(),
            ),
            Self::Care => (
                "攜糧歸家照看家小",
                format!("心頭牽掛家中老小與學徒溫飽，放下手頭瑣務先返{s}家宅安頓"),
                "把早先置辦的乾糧細細包好塞入懷中，快步趕回家宅照看家人".to_string(),
            ),
            Self::Honor => (
                "赴哨所尋同袍敘話",
                format!("胸中塊壘難消，轉赴{s}街坊同袍處對坐敘話、互通聲氣"),
                "拍去肩頭風塵，與熟識的同袍並肩坐下，痛快分飲壺中熱茶".to_string(),
            ),
            Self::Profit => (
                "轉赴外埠套利翻本",
                format!("見本地利薄費時，當機立斷盤算跨區行情，準備轉赴鄰埠市集尋覓急缺差價"),
                "指尖飛快撥算荷包成色，瞇眼打量市集貨擔，盤算著下一趟翻本買賣".to_string(),
            ),
            Self::Craft => (
                "閉門精校隨身器物",
                format!("自覺手頭分寸微亂，決意暫停外務，專心閉門精校「{vessel_name}」"),
                format!("屏息凝神，反覆摩挲並校準手中的「{vessel_name}」，不許分毫差池"),
            ),
            Self::Order => (
                "赴湯屋洗沐重整作息",
                format!("察覺自身作息與氣息失序，按規矩先赴{s}溫水浴堂洗沐滌慮"),
                "整肅衣冠、緩緩吐納，藉溫水熱氣將連日積攢的勞頓規整撫平".to_string(),
            ),
        }
    }
}

/// 住民多元性格傾向權重（六大性格維度，由角色身分基底、種子微擾及當前處境動態調製）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PluralPersonalityWeights {
    pub pride: i32,
    pub care: i32,
    pub honor: i32,
    pub profit: i32,
    pub craft: i32,
    pub order: i32,
}

impl PluralPersonalityWeights {
    pub fn weight_for(self, cornerstone: Cornerstone) -> i32 {
        match cornerstone {
            Cornerstone::Pride => self.pride,
            Cornerstone::Care => self.care,
            Cornerstone::Honor => self.honor,
            Cornerstone::Profit => self.profit,
            Cornerstone::Craft => self.craft,
            Cornerstone::Order => self.order,
        }
    }
}

pub type PersonalityWeights = PluralPersonalityWeights;

/// 結合角色身分、先天單純形、處境If-Then位移與「所學×所熟×所歷」動態計算多元性格維度權重
pub fn compute_plural_personality_weights_with_experience(
    role_hint: &str,
    settlement: &str,
    seed: u64,
    stress: u8,
    magnesium: i32,
    same_task_repetitions: u32,
    hooks: &[RelationalHook],
    situation: Option<PsychSituationSlot>,
) -> PluralPersonalityWeights {
    let slot = match situation {
        Some(s) => s,
        None if stress >= 60 => PsychSituationSlot::HighStressCrisis,
        None if magnesium <= 4 => PsychSituationSlot::ScarcitySurvival,
        None => PsychSituationSlot::VocationalDuty,
    };

    let innate = innate_personality_simplex(seed);
    let sit = situational_if_then_offset(seed, slot);
    let exp = compute_experiential_personality_shift(settlement, same_task_repetitions, hooks, seed);

    // 溫和的職業性向引導（<= 8 分）
    let mut role_nudge = [0i32; 6];
    if role_hint.contains("家眷") || role_hint.contains("內當家") || role_hint.contains("持家") {
        role_nudge[1] += 8; // Care
        role_nudge[5] += 4; // Order
    } else if role_hint.contains("商")
        || role_hint.contains("主事")
        || role_hint.contains("執事")
        || role_hint.contains("帳房")
        || role_hint.contains("行旅")
    {
        role_nudge[3] += 8; // Profit
        role_nudge[5] += 4; // Order
    } else if role_hint.contains("鐵")
        || role_hint.contains("鍛")
        || role_hint.contains("磨")
        || role_hint.contains("石")
        || role_hint.contains("木工")
        || role_hint.contains("木匠")
        || role_hint.contains("木器")
        || role_hint.contains("竹")
        || role_hint.contains("書")
        || role_hint.contains("醫")
    {
        role_nudge[4] += 8; // Craft
        role_nudge[0] += 4; // Pride
    } else if role_hint.contains("巡") || role_hint.contains("衛") || role_hint.contains("哨") {
        role_nudge[2] += 8; // Honor
        role_nudge[5] += 4; // Order
    } else if role_hint.contains("獵") || role_hint.contains("採") || role_hint.contains("樵") {
        role_nudge[0] += 6; // Pride
        role_nudge[4] += 6; // Craft
    }

    // 處境情勢急迫調節
    let mut acute = [0i32; 6];
    if stress >= 60 {
        acute[0] += 15; // Pride
        acute[1] += 15; // Care
        acute[3] += 15; // Profit
    }
    if magnesium <= 4 {
        acute[3] += 20; // Profit
        acute[1] += 15; // Care
    }
    if magnesium >= 25 {
        acute[2] += 20; // Honor
        acute[4] += 15; // Craft
        acute[0] += 15; // Pride
    }

    let val = |idx: usize| -> i32 {
        (innate[idx] + sit[idx] + exp[idx] + role_nudge[idx] + acute[idx]).clamp(0, 250)
    };

    PluralPersonalityWeights {
        pride: val(0),
        care: val(1),
        honor: val(2),
        profit: val(3),
        craft: val(4),
        order: val(5),
    }
}

/// 依住民身分角色、種子及當前處境（壓力、鎂幣）計算多元性格維度權重
pub fn compute_plural_personality_weights(
    role_hint: &str,
    seed: u64,
    stress: u8,
    magnesium: i32,
) -> PluralPersonalityWeights {
    compute_plural_personality_weights_with_experience(
        role_hint,
        "",
        seed,
        stress,
        magnesium,
        1,
        &[],
        None,
    )
}

/// 依先天心性、處境觸發相與歷練成相，產出給大語言模型或身份視圖之自然純粹繁中性格相描述
pub fn describe_npc_myriad_personality_phase(
    entity_id: &str,
    _role_hint: &str,
    seed: u64,
) -> String {
    let innate = innate_personality_simplex(seed);
    let mut facet_order = [0, 1, 2, 3, 4, 5];
    facet_order.sort_by(|&a, &b| innate[b].cmp(&innate[a]));
    let top1 = facet_order[0];
    let top2 = facet_order[1];

    let innate_str = match (top1, top2) {
        (0, 1) => "先天心性自持傲骨、外冷內熱，既有不肯折腰的風骨，心底亦深藏顧念身邊人的柔情",
        (0, 2) => "天生一派孤高傲骨且極重同道情義，行事寧折不彎、恥居人後",
        (0, 3) => "骨子裡傲骨凌厲又深諳市井利害，不屑平庸苟且，遇事講求手腕與成色",
        (0, 4) => "生就一副孤傲自許的脾性，醉心琢磨技藝器物，不喜隨波逐流",
        (0, 5) => "為人自持自重且嚴於律己，傲骨隱於端方規矩之中，言出必踐",

        (1, 0) => "生性慈和體恤、念舊顧後，看似溫厚卻內藏錚錚傲骨，護持親友時分毫不讓",
        (1, 2) => "天性古道熱腸、待人赤誠，最見不得身邊親友同袍受屈受苦",
        (1, 3) => "心裡記掛家小生計溫飽，為求老小安穩既懂操持生計亦能精打細算",
        (1, 4) => "秉性溫良純厚，善於體察細微，操持手藝活計總帶著照料後進的心思",
        (1, 5) => "為人溫和敦厚而循規蹈矩，處處以安頓家宅與照顧晚輩為第一要務",

        (2, 0) => "生來一身磊落俠氣與錚錚傲骨，看重然諾名聲重於身外之物",
        (2, 1) => "天性豪邁仗義、重情重義，同道有難必定傾囊相挺、鼎力周全",
        (2, 3) => "為人處事重信講義，卻也通達商道利害，深知無本難以行義的道理",
        (2, 4) => "性情剛正重諾，敬佩真才實學與精湛造詣，遇事極講道義與原則",
        (2, 5) => "極重然諾規矩，行事光明磊落，一言既出駟馬難追",

        (3, 0) => "心思靈動務實且自持自重，善於審時度勢把握進退，不甘居於人下",
        (3, 1) => "看似算盤精明、分文必較，實則所求皆是為家小後進求得一份安穩依靠",
        (3, 2) => "在商言商講求互利，骨子裡亦知守信踐諾才能長久立足",
        (3, 4) => "眼光敏銳務實，善於將精妙手藝轉化為實在生計進項，精益求精",
        (3, 5) => "凡事精打細算、帳目清晰，極度講究契據契約與條理分寸",

        (4, 0) => "天生一顆匠心、心無旁騖，骨子裡自負手藝成色，不願粗製濫造折辱手藝",
        (4, 1) => "沉潛於器物工序之美，待晚輩徒弟耐心溫厚，樂於傳承手藝心得",
        (4, 2) => "專注於工法規矩與百工精妙，行事方正重諾，以信義與實才立身",
        (4, 3) => "精研手頭造物活計，既追求工藝極致，也通達市集行情與生計價值",
        (4, 5) => "行事嚴謹細緻、分毫不差，在案頭工序間自有一套不可移易的方圓規矩",

        (5, 0) => "秉性方正端嚴、自持沉穩，恪守綱紀分寸，內心有不可逾越的尊嚴底線",
        (5, 1) => "作風循規蹈矩且兼顧人情溫暖，處事公正不偏，顧全大局與身邊眾人",
        (5, 2) => "恪守本分職責與信約法度，以誠信規矩為立身之本",
        (5, 3) => "崇尚條理秩序與契約規範，一分一毫皆依章程交割、童叟無欺",
        (5, 4) => "為人一絲不苟、嚴謹有序，操持營生如精密卡榫般條理分明",
        _ => "心性純樸沉穩，處事端方得體",
    };

    // 處境觸發相
    let voc_off = situational_if_then_offset(seed, PsychSituationSlot::VocationalDuty);
    let mut top_voc = 0;
    for i in 1..6 {
        if voc_off[i] > voc_off[top_voc] {
            top_voc = i;
        }
    }
    let voc_part = match top_voc {
        0 => "立足案頭當值自有一股不容輕慢的傲氣",
        1 => "操持差事時格外顧念同行學徒的辛勞",
        2 => "當差辦事時極重信義託付與同袍聲氣",
        3 => "操持生計時精打細算、寸步不讓",
        4 => "在崗位案頭專注於手上分寸、精益求精",
        _ => "平日當值處事講求條理法度、按部就班",
    };

    let rel_off = situational_if_then_offset(seed, PsychSituationSlot::RelationalEncounter);
    let mut top_rel = 0;
    for i in 1..6 {
        if rel_off[i] > rel_off[top_rel] {
            top_rel = i;
        }
    }
    let rel_part = match top_rel {
        0 => "遇上街坊往來依然端著自尊風骨",
        1 => "與街坊照面時口吻溫和、善解人意",
        2 => "一旦遇上同道交涉或託付則急公好義、重諾守信",
        3 => "與人打交道時算盤分明、利害清晰",
        4 => "遇同行交談則熱衷切磋手藝與門道",
        _ => "在人際往來中進退知分寸、守禮講矩",
    };

    let scarcity_off = situational_if_then_offset(seed, PsychSituationSlot::ScarcitySurvival);
    let mut top_scarcity = 0;
    for i in 1..6 {
        if scarcity_off[i] > scarcity_off[top_scarcity] {
            top_scarcity = i;
        }
    }
    let scarcity_part = match top_scarcity {
        0 => "身處困頓匱乏時更顯一身傲骨硬氣、絕不搖尾乞憐",
        1 => "即便身處難處也先緊著身邊晚輩的溫飽",
        2 => "落入窘迫之際仍信守承諾、不欠人情",
        3 => "遭遇匱乏拮据時則全力尋求變通變現、務實求生",
        4 => "生計吃緊時靠著自身手藝咬牙硬撐",
        _ => "遇上日子難過時愈發節約度日、規矩不亂",
    };

    let mut exp_parts = Vec::new();
    if let Ok(guard) = LIVE_NPC_CAUSAL_STATES.read() {
        if let Some(st) = guard.get(entity_id) {
            if st.same_task_repetitions >= 15 {
                exp_parts.push(format!("經年累月在崗位操持磨礪（累計歷練達 {} 回），手藝與心性愈發沉穩老練", st.same_task_repetitions));
            }
            let has_positive = st.hooks.iter().any(|h| {
                ["恩", "守諾", "解囊", "馳援", "完修"].iter().any(|&k| h.summary.contains(k) || h.kind_label.contains(k))
            });
            let has_negative = st.hooks.iter().any(|h| {
                ["遭拒", "追討", "結怨", "嫌隙", "展期"].iter().any(|&k| h.summary.contains(k) || h.kind_label.contains(k))
            });
            let has_subcontract = st.hooks.iter().any(|h| {
                ["託付", "代辦", "立契"].iter().any(|&k| h.summary.contains(k) || h.kind_label.contains(k))
            });
            if has_positive {
                exp_parts.push("近來承蒙鄉鄰相助與守諾結清，待人更添三分信賴與從容".to_string());
            }
            if has_negative {
                exp_parts.push("近來歷經生計周轉與債信波折，對外行事更添幾分審慎防備".to_string());
            }
            if has_subcontract {
                exp_parts.push("近來承接街坊差事託付，行事交割格外講求分寸".to_string());
            }
        }
    }

    let mut result = format!("其{}。平日{}；{}；{}。", innate_str, voc_part, rel_part, scarcity_part);
    if !exp_parts.is_empty() {
        result.push_str(&format!(" 歷練成相：{}。", exp_parts.join("；")));
    }
    result
}

/// 跨系統因果漣漪事實類型（蝴蝶效應傳播主軸）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RippleFactKind {
    PredatorSpillover,   // 猛獸下山擾道
    WeatherRouteHazard,  // 風雨霜寒阻道
    ResourceFallow,      // 山場休養輪作
    StrandedCargo,       // 行商受阻貨滯驛站
    MarketScarcity,      // 聚落物資奇缺看漲
    ApexSlainOrRescued,  // 除獸解圍／貨單搶通
    PlayerDeed,          // 玩家義舉／借還／大宗買賣／交鋒
}

/// 跨系統因果漣漪事實物件（在全島住民間如實傳導與衰減）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldRippleFact {
    pub fact_id: String,
    pub kind: RippleFactKind,
    pub subject_zone_or_settlement: String,
    pub item_or_actor: String,
    pub order_id: Option<String>,
    pub severity: f32,
    pub origin_tick: i64,
    pub hop_count: u8,
    pub max_hops: u8,
    pub prose_summary: String,
}

/// 滯留物資貨單（商路受阻與公庫懸賞託管）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrandedCargoOrder {
    pub order_id: String,
    pub carrier_id: String,
    pub carrier_name: String,
    pub item_id: String,
    pub item_name: String,
    pub qty: i32,
    pub origin_settlement: String,
    pub dest_settlement: String,
    pub waystation_name: String,
    pub waystation_coord: (i32, i32),
    pub escrow_bounty_mg: i32,
    pub hazard_prose: String,
    pub claimed_by: Option<String>,
    pub created_at_unix: i64,
    pub expires_at_unix: i64,
}

/// 全島即時在線滯留貨單與公庫託管池
pub static LIVE_STRANDED_CARGO_ORDERS: LazyLock<RwLock<HashMap<String, StrandedCargoOrder>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 跨人物人情／恩怨／契約鉤子（Relational Hook）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalHook {
    pub kind_label: String,
    pub target_name: String,
    pub summary: String,
    pub intensity: u8,
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub updated_at_unix: i64,
}

/// 單一住民的因果大腦與體感狀態（$O(1)$ 常駐記憶體）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcCausalState {
    pub cornerstone: Cornerstone,
    pub stress: u8,
    pub stress_break_active: bool,
    #[serde(default)]
    pub hunger: u8,
    #[serde(default)]
    pub thirst: u8,
    #[serde(default)]
    pub fatigue: u8,
    pub plan_steps: [String; 3],
    pub plan_cursor: u8,
    pub belief_scarcity_item: String,
    pub last_surprise_delta: Option<String>,
    pub hooks: Vec<RelationalHook>,
    /// 不可逆之生命直線時間刻度（單調遞增，永不歸零重整）
    pub life_step_tick: u64,
    /// 同業日課與工序執行累計次數（即便每天做同樣的事，時間刻度與經驗仍持續累積）
    pub same_task_repetitions: u32,
    /// 昨夜延續至今晨之未竟狀態（每個人每天起床不是歸零重整，生命是一條直線）
    pub carryover_from_yesterday: String,
    /// 當前觸發之「決策意象」代碼（eat / drink / sleep_or_rest / heal_or_tend / mend_or_equip / work_or_trade / interact_or_unwind）
    #[serde(default)]
    pub decision_image_code: String,
    /// 當前條件（有錢沒錢、有多少錢、現況）下符合資格之預擬行為動作選項數量（體現「有餘裕的選擇多，匱乏的選擇少」）
    #[serde(default)]
    pub eligible_option_count: u8,
    /// 從符合條件之預擬選項中擇定的「行為動作」代碼
    #[serde(default)]
    pub chosen_action_code: String,
    /// 當前驅動住民之「需求」（由餓渴累傷、器損、荷包鎂幣與時刻派生）
    pub active_need: String,
    /// 「決策意象」（事先擬好選項，等待條件符合時，依有錢沒錢／有多少錢做出選擇）
    pub image_decision: String,
    /// 「行為動作」（決策意象在當前場所、器況與時間刻度下落實的具體行為動作）
    pub situated_action: String,
    /// 當前住民感知之物資供需與市價行情摘要
    #[serde(default)]
    pub belief_summary: String,
    /// 當前住民心頭縈繞之跨系統因果漣漪資訊（按突顯度遞減，至多 4 條）
    #[serde(default)]
    pub active_ripples: Vec<WorldRippleFact>,
    /// 住民自身對各漣漪 fact_id 之過濾與免疫時效清單（DAG 無環與 SIR 傳播免疫，至多 24 條）
    #[serde(default)]
    pub immune_fact_ids: Vec<(String, i64)>,
}

/// 七大日常「決策意象」（餓了就吃、渴了就喝、累了就睡、傷了就醫、器鈍就修、醒著幹活、事畢往來）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionImageKind {
    /// 餓了 ➔ 吃
    Eat,
    /// 渴了 ➔ 喝
    Drink,
    /// 累了 ➔ 睡／歇
    SleepOrRest,
    /// 傷了／病了 ➔ 醫／養
    HealOrTend,
    /// 器物鈍了／損了 ➔ 修／換
    MendOrEquip,
    /// 醒著要過日子 ➔ 幹活／交易
    WorkOrTrade,
    /// 手頭事畢／身邊有人 ➔ 往來／消遣
    InteractOrUnwind,
}

impl DecisionImageKind {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Eat => "eat",
            Self::Drink => "drink",
            Self::SleepOrRest => "sleep_or_rest",
            Self::HealOrTend => "heal_or_tend",
            Self::MendOrEquip => "mend_or_equip",
            Self::WorkOrTrade => "work_or_trade",
            Self::InteractOrUnwind => "interact_or_unwind",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Eat => "腹中飢餓，想尋口熱食果腹",
            Self::Drink => "口乾舌燥，想討碗茶水潤喉",
            Self::SleepOrRest => "筋骨困乏，想尋處歇腳養神",
            Self::HealOrTend => "身上帶傷不適，急需敷藥調理",
            Self::MendOrEquip => "隨身器物微鈍，得先修整稱手",
            Self::WorkOrTrade => "趁著精神尚可，忙活手頭生計與買賣",
            Self::InteractOrUnwind => "手頭差事告一段落，想與街坊同袍敘話散心",
        }
    }
}

/// 餘裕度分級（有錢沒錢？有錢，有多少錢？—— 有餘裕的選擇多，匱乏的選擇少）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarginBand {
    /// 匱乏（沒錢 0..=4 鎂）：僅餘極少數逼不得已的求生選項
    Scarcity,
    /// 溫飽（有小錢 5..=24 鎂）：算計荷包的務實日常選項
    Subsistence,
    /// 有餘裕（錢夠 >=25 鎂）：選擇大幅展開（自奉、請客、顧家、挑剔享受、提攜、精進）
    Abundance,
}

impl MarginBand {
    pub const fn from_magnesium(mg: i32) -> Self {
        if mg <= 4 {
            Self::Scarcity
        } else if mg <= 24 {
            Self::Subsistence
        } else {
            Self::Abundance
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Scarcity => "手頭拮据",
            Self::Subsistence => "小有進項、精打細算",
            Self::Abundance => "手頭寬裕",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VenueRequirement {
    CanteenOrStall,
    HomeOrKitchen,
    WildOrOutskirts,
    TeahouseOrTavern,
    WellOrStream,
    Bathhouse,
    InnOrHome,
    Clinic,
    ForgeOrWorkshop,
    DockOrStation,
    Market,
    OwnWorkplace,
    BridgeOrPlaza,
}

/// 事先擬好的「行為動作」選項（等待條件符合時，做出選擇）
#[derive(Debug, Clone, Copy)]
pub struct ActionOptionDef {
    pub code: &'static str,
    pub image: DecisionImageKind,
    pub margin_band: MarginBand,
    pub min_mg: i32,
    pub max_mg: Option<i32>,
    pub cost_mg: i32,
    pub earn_mg: i32,
    pub venue_req: VenueRequirement,
    pub preferred_cornerstone: Option<Cornerstone>,
    pub condition_gate: &'static str,
    pub action_title: &'static str,
    pub action_behavior: &'static str,
}

/// 預先擬好的完整「決策意象 × 條件閘門 × 行為動作」選項表：
/// 每個決策意象在「沒錢（0..=4 鎂）」時僅 2 項窄選項符合；
/// 在「有小錢（5..=24 鎂）」時有 3 項務實選項符合；
/// 在「有餘裕（>=25 鎂）」時展開為 5～6 項豐富選項（涵蓋原本被粗糙歸類為社交、尊重、自我實現的行為）。
pub fn find_predefined_action_by_title(title: &str) -> Option<&'static ActionOptionDef> {
    let t = title.trim();
    PREDEFINED_ACTION_CATALOG.iter().find(|opt| opt.action_title == t)
}

pub const PREDEFINED_ACTION_CATALOG: &[ActionOptionDef] = &[
    // =========================================================================
    // 1. 決策意象：【餓了 ➔ 吃】（Eat）
    // =========================================================================
    // 匱乏（0..=4 鎂）：僅 2 項求生選項
    ActionOptionDef {
        code: "eat_pocket_scraps_or_wild_fruit",
        image: DecisionImageKind::Eat,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::WildOrOutskirts,
        preferred_cornerstone: Some(Cornerstone::Pride),
        condition_gate: "餓了 ＋ 沒錢（0..=4 鎂）",
        action_title: "取出隨身乾糧細嚼果腹",
        action_behavior: "取出隨身攜帶的乾糧細嚼果腹，就著竹筒溫水嚥下墊肚子",
    },
    ActionOptionDef {
        code: "eat_beg_or_credit_thin_broth",
        image: DecisionImageKind::Eat,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::CanteenOrStall,
        preferred_cornerstone: Some(Cornerstone::Honor),
        condition_gate: "餓了 ＋ 沒錢（0..=4 鎂）＋ 身在食堂攤肆旁",
        action_title: "在攤前點碗清湯粗麵墊肚",
        action_behavior: "向熟識的食堂攤主點一碗清湯粗麵，坐在條凳上趁熱吃下墊肚子",
    },
    // 溫飽（>=5 鎂）：務實填肚選項（有小錢即可選，有餘裕時亦保留為節儉務實選項）
    ActionOptionDef {
        code: "eat_cheap_coarse_noodles",
        image: DecisionImageKind::Eat,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 2,
        earn_mg: 0,
        venue_req: VenueRequirement::CanteenOrStall,
        preferred_cornerstone: Some(Cornerstone::Profit),
        condition_gate: "餓了 ＋ 有小錢（≥5 鎂）",
        action_title: "買碗清湯粗麵填肚",
        action_behavior: "花幾枚鎂幣在麵攤點一碗最實惠的清湯粗麵配鹹菜，大口吃完繼續幹活",
    },
    ActionOptionDef {
        code: "eat_flatbread_on_the_move",
        image: DecisionImageKind::Eat,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: Some(24),
        cost_mg: 2,
        earn_mg: 0,
        venue_req: VenueRequirement::CanteenOrStall,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "餓了 ＋ 有小錢（5..=24 鎂）＋ 手頭有活要趕",
        action_title: "買兩個粗麥餅邊走邊啃",
        action_behavior: "在餅攤買兩個初出爐的粗麥餅揣在懷裡，邊趕路邊啃以節省坐店時間",
    },
    ActionOptionDef {
        code: "eat_buy_grain_cook_at_home",
        image: DecisionImageKind::Eat,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 3,
        earn_mg: 0,
        venue_req: VenueRequirement::HomeOrKitchen,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "餓了 ＋ 有小錢（≥5 鎂）＋ 重視日常規矩",
        action_title: "採買米菜自炊熱粥",
        action_behavior: "算計著荷包買一把青菜與糙米回屋自己生火熬粥，吃得熱乎又省錢",
    },
    // 有餘裕（>=25 鎂）：額外解鎖 4 項高自由度選項（加計溫飽保留項共 6 項可選！）
    ActionOptionDef {
        code: "eat_feast_mountain_sea_stew",
        image: DecisionImageKind::Eat,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 10,
        earn_mg: 0,
        venue_req: VenueRequirement::CanteenOrStall,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "餓了 ＋ 有餘裕（≥25 鎂）＋ 犒賞自己",
        action_title: "點一盅山海合味大菜慢用",
        action_behavior: "進食堂點一盅慢火精燉的「山海合味」鮮湯配兩碟熱菜，慢條斯理犒賞今日辛勞",
    },
    ActionOptionDef {
        code: "eat_treat_peers_at_table",
        image: DecisionImageKind::Eat,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 12,
        earn_mg: 0,
        venue_req: VenueRequirement::CanteenOrStall,
        preferred_cornerstone: Some(Cornerstone::Honor),
        condition_gate: "餓了 ＋ 有餘裕（≥25 鎂）＋ 同場有熟人同袍",
        action_title: "作東請同袍街坊吃熱食",
        action_behavior: "招呼同場熟識的街坊同袍坐下，主動掏錢加菜作東請客，席間熱絡話家常",
    },
    ActionOptionDef {
        code: "eat_pack_braised_meat_for_home",
        image: DecisionImageKind::Eat,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 8,
        earn_mg: 0,
        venue_req: VenueRequirement::HomeOrKitchen,
        preferred_cornerstone: Some(Cornerstone::Care),
        condition_gate: "餓了 ＋ 有餘裕（≥25 鎂）＋ 記掛家眷學徒",
        action_title: "切份滷肉包熱點心解饞",
        action_behavior: "自己吃飽之餘，特地請掌勺切一包滷肉、包兩份熱點心帶回家中加菜",
    },
    ActionOptionDef {
        code: "eat_medicated_nourishing_broth",
        image: DecisionImageKind::Eat,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 9,
        earn_mg: 0,
        venue_req: VenueRequirement::CanteenOrStall,
        preferred_cornerstone: Some(Cornerstone::Pride),
        condition_gate: "餓了 ＋ 有餘裕（≥25 鎂）＋ 講究養氣調身",
        action_title: "點山野藥膳湯補氣養身",
        action_behavior: "在食堂點一碗加了山野靈草的藥膳溫湯，邊進食邊調養氣血經脈",
    },

    // =========================================================================
    // 2. 決策意象：【渴了 ➔ 喝】（Drink）
    // =========================================================================
    // 匱乏（0..=4 鎂）：2 項
    ActionOptionDef {
        code: "drink_cold_well_or_stream_water",
        image: DecisionImageKind::Drink,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::WellOrStream,
        preferred_cornerstone: Some(Cornerstone::Pride),
        condition_gate: "渴了 ＋ 沒錢（0..=4 鎂）",
        action_title: "掬清泉冷水解渴",
        action_behavior: "走到街邊汲水石槽旁，捧起幾口清冽冷水灌下解渴",
    },
    ActionOptionDef {
        code: "drink_ladle_workshop_water_jar",
        image: DecisionImageKind::Drink,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::OwnWorkplace,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "渴了 ＋ 沒錢（0..=4 鎂）＋ 身在案頭或屋舍",
        action_title: "取用清冽涼水一飲解渴",
        action_behavior: "拿起木瓢從公用大水缸裡舀半瓢涼水一飲而盡",
    },
    // 溫飽（>=5 鎂）：3 項（其中 1 項僅限 5..=24 鎂，2 項延續至有餘裕）
    ActionOptionDef {
        code: "drink_roadside_coarse_tea_bowl",
        image: DecisionImageKind::Drink,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 1,
        earn_mg: 0,
        venue_req: VenueRequirement::TeahouseOrTavern,
        preferred_cornerstone: Some(Cornerstone::Profit),
        condition_gate: "渴了 ＋ 有小錢（≥5 鎂）",
        action_title: "灌大碗解暑粗茶",
        action_behavior: "在路邊茶攤點一大碗粗茶，咕咚咕咚灌下消暑解渴",
    },
    ActionOptionDef {
        code: "drink_refill_bamboo_water_tube",
        image: DecisionImageKind::Drink,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: Some(24),
        cost_mg: 1,
        earn_mg: 0,
        venue_req: VenueRequirement::TeahouseOrTavern,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "渴了 ＋ 有小錢（5..=24 鎂）＋ 準備續行",
        action_title: "灌滿隨身竹水筒備用",
        action_behavior: "在茶攤喝過茶後，順手把隨身竹水筒灌滿溫水塞回腰間備著路上喝",
    },
    ActionOptionDef {
        code: "drink_herbal_cooling_tea",
        image: DecisionImageKind::Drink,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 2,
        earn_mg: 0,
        venue_req: VenueRequirement::TeahouseOrTavern,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "渴了 ＋ 有小錢（≥5 鎂）＋ 勞作燥熱",
        action_title: "買碗草木涼茶退火",
        action_behavior: "在藥草茶鋪買一碗微苦回甘的草木涼茶喝下，壓一壓連日勞作的燥火",
    },
    // 有餘裕（>=25 鎂）：額外解鎖 3 項（共 5 項可選）
    ActionOptionDef {
        code: "drink_sip_spring_tea_at_teahouse",
        image: DecisionImageKind::Drink,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 6,
        earn_mg: 0,
        venue_req: VenueRequirement::TeahouseOrTavern,
        preferred_cornerstone: Some(Cornerstone::Pride),
        condition_gate: "渴了 ＋ 有餘裕（≥25 鎂）＋ 心神從容",
        action_title: "上茶寮慢品山泉清茗",
        action_behavior: "上茶寮雅座點一壺山泉活水沖泡的春茶，慢啜細品、聽窗外風聲歇腳",
    },
    ActionOptionDef {
        code: "drink_share_pot_of_tea_with_peers",
        image: DecisionImageKind::Drink,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 8,
        earn_mg: 0,
        venue_req: VenueRequirement::TeahouseOrTavern,
        preferred_cornerstone: Some(Cornerstone::Honor),
        condition_gate: "渴了 ＋ 有餘裕（≥25 鎂）＋ 同場有熟人",
        action_title: "叫一壺好茶邀同道共飲談天",
        action_behavior: "主動叫一壺上好焙茶與茶果，招呼鄰座同道共飲、交換各聚落奇聞與行情",
    },
    ActionOptionDef {
        code: "drink_warm_rice_wine_after_work",
        image: DecisionImageKind::Drink,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 7,
        earn_mg: 0,
        venue_req: VenueRequirement::TeahouseOrTavern,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "渴了 ＋ 有餘裕（≥25 鎂）＋ 活計告一段落",
        action_title: "溫一壺米酒配小菜舒筋活血",
        action_behavior: "在食肆溫一壺醇厚米酒配一碟鹽水花生，小酌幾杯活絡一日緊繃的筋骨",
    },

    // =========================================================================
    // 3. 決策意象：【累了 ➔ 睡／歇】（SleepOrRest）
    // =========================================================================
    // 匱乏（0..=4 鎂）：2 項
    ActionOptionDef {
        code: "rest_crouch_under_eaves_or_rock",
        image: DecisionImageKind::SleepOrRest,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::HomeOrKitchen,
        preferred_cornerstone: Some(Cornerstone::Pride),
        condition_gate: "累了 ＋ 沒錢（0..=4 鎂）",
        action_title: "於居所木榻和衣閉目歇息",
        action_behavior: "回到居所木榻和衣歇息，閉目調勻氣息養回體力",
    },
    ActionOptionDef {
        code: "rest_lean_on_workbench_catch_breath",
        image: DecisionImageKind::SleepOrRest,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::OwnWorkplace,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "累了 ＋ 沒錢（0..=4 鎂）＋ 才停下手頭活計",
        action_title: "尋處陰涼角落閉目歇腳喘口氣",
        action_behavior: "靠在工坊案頭木架旁閉目調勻急促的呼吸，歇息片刻養回體力",
    },
    // 溫飽（>=5 鎂）：3 項
    ActionOptionDef {
        code: "rest_return_home_sleep_on_cot",
        image: DecisionImageKind::SleepOrRest,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::HomeOrKitchen,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "累了 ＋ 有小錢（≥5 鎂）＋ 有固定屋舍",
        action_title: "在屋舍吹燈安穩睡一覺",
        action_behavior: "回到自己屋舍栓好門、吹熄燈火，在木榻上規規矩矩睡上一覺養精蓄銳",
    },
    ActionOptionDef {
        code: "rest_rent_cheap_common_bunk",
        image: DecisionImageKind::SleepOrRest,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: Some(24),
        cost_mg: 3,
        earn_mg: 0,
        venue_req: VenueRequirement::InnOrHome,
        preferred_cornerstone: Some(Cornerstone::Profit),
        condition_gate: "累了 ＋ 有小錢（5..=24 鎂）＋ 身在外埠",
        action_title: "在旅社通鋪床位歇宿",
        action_behavior: "在旅社通鋪床位脫鞋歇腳，避開夜裡山風寒氣安穩歇息",
    },
    ActionOptionDef {
        code: "rest_sit_under_shade_wipe_sweat",
        image: DecisionImageKind::SleepOrRest,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::BridgeOrPlaza,
        preferred_cornerstone: Some(Cornerstone::Care),
        condition_gate: "累了 ＋ 有小錢（≥5 鎂）＋ 白日歇腳",
        action_title: "坐街邊樹蔭木椅擦汗歇腿",
        action_behavior: "在街邊樹蔭木椅坐下擦乾額頭汗水，揉揉酸痛的小腿肚喘勻氣息",
    },
    // 有餘裕（>=25 鎂）：額外解鎖 3 項（共 5 項可選）
    ActionOptionDef {
        code: "rest_soak_in_hot_stone_bathhouse",
        image: DecisionImageKind::SleepOrRest,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 8,
        earn_mg: 0,
        venue_req: VenueRequirement::Bathhouse,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "累了 ＋ 有餘裕（≥25 鎂）＋ 想要徹底解乏",
        action_title: "去溫水石湯浴場泡湯解乏",
        action_behavior: "進溫水石湯浴場泡個草木熱水澡，洗去一身汗垢疲乏後再神清氣爽安歇",
    },
    ActionOptionDef {
        code: "rest_book_quiet_private_room_at_inn",
        image: DecisionImageKind::SleepOrRest,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 12,
        earn_mg: 0,
        venue_req: VenueRequirement::InnOrHome,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "累了 ＋ 有餘裕（≥25 鎂）＋ 講究睡眠品質",
        action_title: "包下旅社清靜單間點安神香熟睡",
        action_behavior: "在旅社包下一間清靜雅房，點上安神木香、換上乾爽布衫舒舒服服睡到自然醒",
    },
    ActionOptionDef {
        code: "rest_meditate_and_gaze_water_on_bridge",
        image: DecisionImageKind::SleepOrRest,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::BridgeOrPlaza,
        preferred_cornerstone: Some(Cornerstone::Pride),
        condition_gate: "累了 ＋ 有餘裕（≥25 鎂）＋ 不急著趕工",
        action_title: "憑欄觀水調息放空心神",
        action_behavior: "信步到水岸橋欄旁凝望川流水色、調勻呼吸放空思緒，讓心神徹底沉澱",
    },

    // =========================================================================
    // 4. 決策意象：【傷了 ➔ 醫／養】（HealOrTend）
    // =========================================================================
    // 匱乏（0..=4 鎂）：2 項
    ActionOptionDef {
        code: "heal_tear_cloth_bind_wound_and_endure",
        image: DecisionImageKind::HealOrTend,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::HomeOrKitchen,
        preferred_cornerstone: Some(Cornerstone::Pride),
        condition_gate: "傷了 ＋ 沒錢（0..=4 鎂）",
        action_title: "用乾淨布條綁緊傷處調息",
        action_behavior: "用乾淨布條仔細綁緊傷處，放慢手腳調息靜養",
    },
    ActionOptionDef {
        code: "heal_chew_wild_herb_poultice",
        image: DecisionImageKind::HealOrTend,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::WildOrOutskirts,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "傷了 ＋ 沒錢（0..=4 鎂）＋ 略通草木",
        action_title: "尋路邊止血草嚼碎外敷",
        action_behavior: "在林邊尋兩株野生止血草嚼碎敷在患處，靠自身底子慢慢消腫",
    },
    // 溫飽（>=5 鎂）：3 項
    ActionOptionDef {
        code: "heal_buy_cheap_bruise_plaster",
        image: DecisionImageKind::HealOrTend,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 3,
        earn_mg: 0,
        venue_req: VenueRequirement::Clinic,
        preferred_cornerstone: Some(Cornerstone::Profit),
        condition_gate: "傷了 ＋ 有小錢（≥5 鎂）",
        action_title: "去藥鋪取跌打膏藥貼敷患處",
        action_behavior: "在藥鋪取一貼跌打草藥膏貼在患處，舒緩筋骨拉扯痛楚",
    },
    ActionOptionDef {
        code: "heal_clinic_basic_clean_and_bandage",
        image: DecisionImageKind::HealOrTend,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: Some(24),
        cost_mg: 4,
        earn_mg: 0,
        venue_req: VenueRequirement::Clinic,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "傷了 ＋ 有小錢（5..=24 鎂）＋ 需儘快復工",
        action_title: "請醫工簡單清洗包紮後趕回上工",
        action_behavior: "去脈絡診所請醫工做最基礎的清創包紮，包好傷口不敢多歇便趕回案頭",
    },
    ActionOptionDef {
        code: "heal_boil_ginger_herb_soup_at_home",
        image: DecisionImageKind::HealOrTend,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 2,
        earn_mg: 0,
        venue_req: VenueRequirement::HomeOrKitchen,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "傷了 ＋ 有小錢（≥5 鎂）＋ 暗傷勞損",
        action_title: "買老薑草藥回屋煎湯熱敷",
        action_behavior: "買幾塊老薑與活血草回屋煎成熱湯，邊喝邊用熱帕敷揉酸痛關節",
    },
    // 有餘裕（>=25 鎂）：額外解鎖 3 項（共 5 項可選）
    ActionOptionDef {
        code: "heal_full_meridian_acupuncture_and_salve",
        image: DecisionImageKind::HealOrTend,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 14,
        earn_mg: 0,
        venue_req: VenueRequirement::Clinic,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "傷了 ＋ 有餘裕（≥25 鎂）＋ 尋求根治",
        action_title: "請診所主理施針推拿並敷上等膏藥",
        action_behavior: "在脈絡診所請主理施針理氣、敷上溫潤脈絡膏藥，調理體內勞損",
    },
    ActionOptionDef {
        code: "heal_take_paid_sick_leave_rest_home",
        image: DecisionImageKind::HealOrTend,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 4,
        earn_mg: 0,
        venue_req: VenueRequirement::HomeOrKitchen,
        preferred_cornerstone: Some(Cornerstone::Care),
        condition_gate: "傷了 ＋ 有餘裕（≥25 鎂）＋ 手頭存款充裕",
        action_title: "停工安心靜養調養傷勢",
        action_behavior: "放下手頭差事在居所安心靜養，緩步調勻氣息等待傷勢復原",
    },
    ActionOptionDef {
        code: "heal_buy_extra_salve_and_cover_peer_bill",
        image: DecisionImageKind::HealOrTend,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 12,
        earn_mg: 0,
        venue_req: VenueRequirement::Clinic,
        preferred_cornerstone: Some(Cornerstone::Honor),
        condition_gate: "傷了 ＋ 有餘裕（≥25 鎂）＋ 顧念同袍",
        action_title: "多備傷藥並替同袍分擔藥資",
        action_behavior: "包妥傷處後多備兩盒金瘡藥收進行囊，並順手替同來就醫的同袍結清藥資",
    },

    // =========================================================================
    // 5. 決策意象：【器鈍 ➔ 修／換】（MendOrEquip）
    // =========================================================================
    // 匱乏（0..=4 鎂）：2 項
    ActionOptionDef {
        code: "mend_make_do_with_worn_vessel",
        image: DecisionImageKind::MendOrEquip,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::OwnWorkplace,
        preferred_cornerstone: Some(Cornerstone::Profit),
        condition_gate: "器物磨損 ＋ 沒錢（0..=4 鎂）",
        action_title: "調整握持角度繼續操持器物",
        action_behavior: "避開器物磨損部位，換個順手角度繼續專注操持活計",
    },
    ActionOptionDef {
        code: "mend_self_rub_with_stream_stone_or_cord",
        image: DecisionImageKind::MendOrEquip,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::WellOrStream,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "器物磨損 ＋ 沒錢（0..=4 鎂）＋ 自己動手應急",
        action_title: "尋溪邊青石打磨修整器物",
        action_behavior: "蹲在溪邊青石旁細細打磨器物刃口，並用結實麻繩將木柄綁緊加固",
    },
    // 溫飽（>=5 鎂）：3 項
    ActionOptionDef {
        code: "mend_standard_workshop_maintenance",
        image: DecisionImageKind::MendOrEquip,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 4,
        earn_mg: 0,
        venue_req: VenueRequirement::ForgeOrWorkshop,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "器物磨損 ＋ 有小錢（≥5 鎂）",
        action_title: "在案頭做常規修整校準",
        action_behavior: "在工坊案頭用桐油細砂與校準尺規規矩矩修整隨身器物分寸",
    },
    ActionOptionDef {
        code: "mend_buy_spare_part_replace_self",
        image: DecisionImageKind::MendOrEquip,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: Some(24),
        cost_mg: 3,
        earn_mg: 0,
        venue_req: VenueRequirement::Market,
        preferred_cornerstone: Some(Cornerstone::Profit),
        condition_gate: "器物磨損 ＋ 有小錢（5..=24 鎂）＋ 精打細算",
        action_title: "去市集添購木柄麻線更換修補",
        action_behavior: "在市集挑選一截新木柄與桐油麻線，動手拆換修補隨身器物",
    },
    ActionOptionDef {
        code: "mend_oil_and_wipe_vessel_carefully",
        image: DecisionImageKind::MendOrEquip,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 2,
        earn_mg: 0,
        venue_req: VenueRequirement::OwnWorkplace,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "器物磨損 ＋ 有小錢（≥5 鎂）＋ 愛惜吃飯傢伙",
        action_title: "用保養油膏細細擦拭養護器物",
        action_behavior: "沾取防鏽桐油，在燈下一寸寸細心擦拭養護隨身器物",
    },
    // 有餘裕（>=25 鎂）：額外解鎖 3 項（共 5 項可選）
    ActionOptionDef {
        code: "mend_master_artisan_precision_overhaul",
        image: DecisionImageKind::MendOrEquip,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 12,
        earn_mg: 0,
        venue_req: VenueRequirement::ForgeOrWorkshop,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "器物磨損 ＋ 有餘裕（≥25 鎂）＋ 追求極致手感",
        action_title: "請百工老師傅精校重淬器物",
        action_behavior: "請百工坊老師傅開爐重淬精校、換上合手配件，讓隨身器物更為趁手",
    },
    ActionOptionDef {
        code: "mend_commission_backup_vessel",
        image: DecisionImageKind::MendOrEquip,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 16,
        earn_mg: 0,
        venue_req: VenueRequirement::ForgeOrWorkshop,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "器物磨損 ＋ 有餘裕（≥25 鎂）＋ 未雨綢繆",
        action_title: "向工坊添置備用器物輪換使用",
        action_behavior: "向工坊訂製一件備用器物輪換使用，仔細檢視器身卡榫與刃口成色",
    },
    ActionOptionDef {
        code: "mend_gift_repaired_vessel_to_junior",
        image: DecisionImageKind::MendOrEquip,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 10,
        earn_mg: 0,
        venue_req: VenueRequirement::ForgeOrWorkshop,
        preferred_cornerstone: Some(Cornerstone::Care),
        condition_gate: "器物磨損 ＋ 有餘裕（≥25 鎂）＋ 提攜後進",
        action_title: "換上新器並將舊器贈予後進學徒",
        action_behavior: "換上精良新器，並將修整妥當的舊器物交予年輕學徒助其操持生計",
    },

    // =========================================================================
    // 6. 決策意象：【過日子 ➔ 幹活／交易】（WorkOrTrade）
    // =========================================================================
    // 匱乏（0..=4 鎂）：2 項
    ActionOptionDef {
        code: "work_take_hard_dock_or_haul_odd_job",
        image: DecisionImageKind::WorkOrTrade,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 5,
        venue_req: VenueRequirement::DockOrStation,
        preferred_cornerstone: Some(Cornerstone::Profit),
        condition_gate: "要過日子 ＋ 沒錢（0..=4 鎂）",
        action_title: "在貨站扛運麻袋裝卸貨物",
        action_behavior: "在貨站碼頭彎腰扛起沉重的麻袋裝卸貨物，忙得額頭見汗",
    },
    ActionOptionDef {
        code: "work_scavenge_wild_firewood_and_scraps",
        image: DecisionImageKind::WorkOrTrade,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 4,
        venue_req: VenueRequirement::WildOrOutskirts,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "要過日子 ＋ 沒錢（0..=4 鎂）＋ 沒本錢進貨",
        action_title: "空手拾柴採野草換取鎂幣",
        action_behavior: "手頭沒本錢買原材，只能空手去山林近郊拾枯柴、採粗草挑去市集換幾枚糊口錢",
    },
    // 溫飽（>=5 鎂）：3 項
    ActionOptionDef {
        code: "work_perform_steady_daily_vocation",
        image: DecisionImageKind::WorkOrTrade,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 0,
        earn_mg: 7,
        venue_req: VenueRequirement::OwnWorkplace,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "要過日子 ＋ 有小錢（≥5 鎂）＋ 本職當值",
        action_title: "按部就班完成本職日課工序",
        action_behavior: "帶著隨身器物在崗位上沉穩推進今日工序，靠熟練手藝賺取安穩工錢",
    },
    ActionOptionDef {
        code: "work_peddle_gathered_goods_at_market",
        image: DecisionImageKind::WorkOrTrade,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: Some(24),
        cost_mg: 0,
        earn_mg: 8,
        venue_req: VenueRequirement::Market,
        preferred_cornerstone: Some(Cornerstone::Profit),
        condition_gate: "要過日子 ＋ 有小錢（5..=24 鎂）＋ 手頭有產出",
        action_title: "挑手頭產出赴市集擺攤交割變現",
        action_behavior: "將新整理好的一批物料挑去市集攤位叫賣交割，算清今日進帳與損耗",
    },
    ActionOptionDef {
        code: "work_restock_needed_raw_materials",
        image: DecisionImageKind::WorkOrTrade,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 4,
        earn_mg: 9,
        venue_req: VenueRequirement::Market,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "要過日子 ＋ 有小錢（≥5 鎂）＋ 補充手頭物料",
        action_title: "拿手頭周轉金去市集採買工序原材",
        action_behavior: "照著聚落今日行情，拿手頭周轉鎂幣去市集補齊下一輪工序所需的原材",
    },
    // 有餘裕（>=25 鎂）：額外解鎖 4 項（共 6 項可選）
    ActionOptionDef {
        code: "work_craft_masterpiece_at_own_pace",
        image: DecisionImageKind::WorkOrTrade,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 3,
        earn_mg: 12,
        venue_req: VenueRequirement::OwnWorkplace,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "要過日子 ＋ 有餘裕（≥25 鎂）＋ 不必趕廉價粗活",
        action_title: "在案頭花慢工夫精雕細琢器物",
        action_behavior: "屏息凝神在案頭花慢工夫打磨精修器物細節，神情專注從容",
    },
    ActionOptionDef {
        code: "work_cross_settlement_scarcity_arbitrage",
        image: DecisionImageKind::WorkOrTrade,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 6,
        earn_mg: 15,
        venue_req: VenueRequirement::Market,
        preferred_cornerstone: Some(Cornerstone::Profit),
        condition_gate: "要過日子 ＋ 有餘裕（≥25 鎂）＋ 手握充裕本錢",
        action_title: "採購緊缺物資跨區調撥交割",
        action_behavior: "在坊市攤位間清點收購緊缺物資，準備跨區調撥交割",
    },
    ActionOptionDef {
        code: "work_mentor_junior_and_delegate_routine",
        image: DecisionImageKind::WorkOrTrade,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 4,
        earn_mg: 8,
        venue_req: VenueRequirement::OwnWorkplace,
        preferred_cornerstone: Some(Cornerstone::Care),
        condition_gate: "要過日子 ＋ 有餘裕（≥25 鎂）＋ 身邊有學徒同袍",
        action_title: "指點晚輩手藝並把常規粗活交給學徒歷練",
        action_behavior: "在案頭從容指點年輕學徒手藝訣竅，把常規粗活交給晚輩練手並分予工錢",
    },
    ActionOptionDef {
        code: "work_finish_early_for_personal_cultivation",
        image: DecisionImageKind::WorkOrTrade,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 2,
        earn_mg: 0,
        venue_req: VenueRequirement::BridgeOrPlaza,
        preferred_cornerstone: Some(Cornerstone::Pride),
        condition_gate: "要過日子 ＋ 有餘裕（≥25 鎂）＋ 今日進項已足",
        action_title: "提早收工去書閣翻閱山海古卷",
        action_behavior: "提早收工步入書閣案前，靜心翻閱山海古卷與半島風物圖誌",
    },

    // =========================================================================
    // 7. 決策意象：【事畢有人 ➔ 往來／消遣】（InteractOrUnwind）
    // =========================================================================
    // 匱乏（0..=4 鎂）：2 項
    ActionOptionDef {
        code: "social_avoid_eye_contact_hurry_past",
        image: DecisionImageKind::InteractOrUnwind,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::BridgeOrPlaza,
        preferred_cornerstone: Some(Cornerstone::Pride),
        condition_gate: "身邊有人 ＋ 沒錢（0..=4 鎂）",
        action_title: "低頭快步穿行街角不多言語",
        action_behavior: "低頭快步走過街角，神色匆匆不多言語",
    },
    ActionOptionDef {
        code: "social_ask_acquaintance_for_small_loan",
        image: DecisionImageKind::InteractOrUnwind,
        margin_band: MarginBand::Scarcity,
        min_mg: 0,
        max_mg: Some(4),
        cost_mg: 0,
        earn_mg: 3,
        venue_req: VenueRequirement::BridgeOrPlaza,
        preferred_cornerstone: Some(Cornerstone::Honor),
        condition_gate: "身邊有人 ＋ 沒錢（0..=4 鎂）＋ 遇見有交情街坊",
        action_title: "向熟識同袍商議周轉應急",
        action_behavior: "向素有交情的同袍低聲訴說近況，商議周轉幾枚鎂幣應急",
    },
    // 溫飽（>=5 鎂）：3 項
    ActionOptionDef {
        code: "social_chat_weather_and_market_prices",
        image: DecisionImageKind::InteractOrUnwind,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::Market,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "身邊有人 ＋ 有小錢（≥5 鎂）＋ 日常照面",
        action_title: "與街坊寒暄今日山道天氣與市集物價",
        action_behavior: "停下腳步與迎面遇見的街坊寒暄幾句今日風雨路況與市集米糧行情",
    },
    ActionOptionDef {
        code: "social_lend_a_hand_moving_loads",
        image: DecisionImageKind::InteractOrUnwind,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: Some(24),
        cost_mg: 0,
        earn_mg: 2,
        venue_req: VenueRequirement::DockOrStation,
        preferred_cornerstone: Some(Cornerstone::Honor),
        condition_gate: "身邊有人 ＋ 有小錢（5..=24 鎂）＋ 見熟人忙碌",
        action_title: "順手幫同袍搭把手抬貨結個人情",
        action_behavior: "見熟識的街坊正吃力搬貨，順手上前搭把手抬過石階，結個實在人情",
    },
    ActionOptionDef {
        code: "social_compare_craft_tips_with_peer",
        image: DecisionImageKind::InteractOrUnwind,
        margin_band: MarginBand::Subsistence,
        min_mg: 5,
        max_mg: None,
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::OwnWorkplace,
        preferred_cornerstone: Some(Cornerstone::Craft),
        condition_gate: "身邊有人 ＋ 有小錢（≥5 鎂）＋ 遇見同行",
        action_title: "與同行交流近期器物保養與工序心得",
        action_behavior: "與同在現場的同行比劃討論近期材料成色與器物保養竅門",
    },
    // 有餘裕（>=25 鎂）：額外解鎖 3 項（共 5 項可選）
    ActionOptionDef {
        code: "social_bail_out_and_sponsor_struggling_peer",
        image: DecisionImageKind::InteractOrUnwind,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 10,
        earn_mg: 0,
        venue_req: VenueRequirement::BridgeOrPlaza,
        preferred_cornerstone: Some(Cornerstone::Honor),
        condition_gate: "身邊有人 ＋ 有餘裕（≥25 鎂）＋ 見街坊有難",
        action_title: "慷慨解囊替同袍街坊平帳解圍",
        action_behavior: "見同袍街坊手頭受困，爽快掏出錢袋替對方平帳解圍",
    },
    ActionOptionDef {
        code: "social_host_gathering_and_share_news",
        image: DecisionImageKind::InteractOrUnwind,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 10,
        earn_mg: 0,
        venue_req: VenueRequirement::TeahouseOrTavern,
        preferred_cornerstone: Some(Cornerstone::Order),
        condition_gate: "身邊有人 ＋ 有餘裕（≥25 鎂）＋ 閒暇聚會",
        action_title: "在茶屋擺開茶果邀好友品茶論事",
        action_behavior: "在茶屋廊下擺開茶果，招呼三五好友品茶暢談、商議坊間事務",
    },
    ActionOptionDef {
        code: "social_stroll_scenic_cape_and_compose_notes",
        image: DecisionImageKind::InteractOrUnwind,
        margin_band: MarginBand::Abundance,
        min_mg: 25,
        max_mg: None,
        cost_mg: 0,
        earn_mg: 0,
        venue_req: VenueRequirement::BridgeOrPlaza,
        preferred_cornerstone: Some(Cornerstone::Pride),
        condition_gate: "事畢有閒 ＋ 有餘裕（≥25 鎂）＋ 心神舒展",
        action_title: "信步巡遊石橋海岬並記錄風物手札",
        action_behavior: "俗務已了且衣食無憂，信步走到石橋海岬遠眺山海潮汐，在手札記下今日所見所感",
    },
];

/// 依住民當前處境（傷病、器損、抵達場所、壓力、時間刻度）判定觸發哪一個「決策意象」
pub fn resolve_decision_image(
    arrived_dest_name: &str,
    has_affliction: bool,
    vessel_worn: bool,
    stress: u8,
    life_step_tick: u64,
) -> DecisionImageKind {
    if has_affliction || arrived_dest_name.contains("診") || arrived_dest_name.contains("醫") {
        DecisionImageKind::HealOrTend
    } else if vessel_worn {
        DecisionImageKind::MendOrEquip
    } else if arrived_dest_name.contains("食堂")
        || arrived_dest_name.contains("廚")
        || arrived_dest_name.contains("飯")
        || arrived_dest_name.contains("麵")
    {
        DecisionImageKind::Eat
    } else if arrived_dest_name.contains("茶")
        || arrived_dest_name.contains("酒")
        || arrived_dest_name.contains("泉")
        || arrived_dest_name.contains("井")
    {
        DecisionImageKind::Drink
    } else if arrived_dest_name.contains("浴")
        || arrived_dest_name.contains("湯")
        || arrived_dest_name.contains("居")
        || arrived_dest_name.contains("宅")
        || arrived_dest_name.contains("旅")
        || arrived_dest_name.contains("宿")
        || stress >= 75
    {
        DecisionImageKind::SleepOrRest
    } else if arrived_dest_name.contains("橋")
        || arrived_dest_name.contains("廣場")
        || arrived_dest_name.contains("公議")
        || arrived_dest_name.contains("書閣")
    {
        DecisionImageKind::InteractOrUnwind
    } else if arrived_dest_name.is_empty() && life_step_tick.is_multiple_of(5) {
        DecisionImageKind::Eat
    } else {
        DecisionImageKind::WorkOrTrade
    }
}

/// 篩選某「決策意象」在當前鎂幣（有錢沒錢？有多少錢？）下所有符合條件的預擬「行為動作」選項：
/// - 沒錢（`mg <= 4`）時，僅回傳 2 個求生窄選項（匱乏的選擇少）；
/// - 有小錢（`mg: 5..=24`）時，回傳 3 個務實選項；
/// - 有餘裕（`mg >= 25`）時，回傳 5～6 個豐富選項（有餘裕的選擇多）。
pub fn eligible_actions_for_image(
    image: DecisionImageKind,
    magnesium: i32,
) -> Vec<&'static ActionOptionDef> {
    let mg = magnesium.max(0);
    PREDEFINED_ACTION_CATALOG
        .iter()
        .filter(|opt| {
            opt.image == image
                && mg >= opt.min_mg
                && opt.max_mg.is_none_or(|max| mg <= max)
                && mg >= opt.cost_mg
        })
        .collect()
}

/// 從符合條件的預擬行為動作選項池中，依住民性格基石、多元性格權重與直線時間刻度做出選擇
pub fn choose_action_from_eligible(
    eligible: &[&'static ActionOptionDef],
    cornerstone: Cornerstone,
    life_step_tick: u64,
    seed: u64,
) -> &'static ActionOptionDef {
    if eligible.is_empty() {
        return &PREDEFINED_ACTION_CATALOG[0];
    }
    if eligible.len() == 1 {
        return eligible[0];
    }
    // 計算該住民之多元性格權重（以當前 seed 及預設狀態派生次要傾向）
    let weights = compute_plural_personality_weights("", seed, 30, 10);
    // 以性格基石為主力核心（+85），同時融合多元副性格權重與時間刻度擾動，使副性格亦能真實影響選擇
    let mut best_opt = eligible[0];
    let mut best_score = i64::MIN;
    for (i, opt) in eligible.iter().enumerate() {
        let mut score: i64 = 50;
        if opt.preferred_cornerstone == Some(cornerstone) {
            // 每隔若干刻度允許副性格顯化探路，其餘時間以主要基石為重
            if (life_step_tick + seed).is_multiple_of(7) {
                score += 35;
            } else {
                score += 85;
            }
        }
        if let Some(c) = opt.preferred_cornerstone {
            score += weights.weight_for(c) as i64;
        }
        // 結合不可逆生命直線刻度與種子之確定性微擾
        let jitter = ((life_step_tick.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ seed.wrapping_add(i as u64)) % 23) as i64;
        score += jitter;

        if score > best_score {
            best_score = score;
            best_opt = opt;
        }
    }
    best_opt
}

/// 由實體 ID 與 soul_seed 計算穩定一致的非零種子
pub fn deterministic_resident_seed(entity_id: &str, soul_seed: Option<i64>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in entity_id.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    if let Some(s) = soul_seed
        && s != 0
    {
        h ^= s.unsigned_abs().wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    h.max(1)
}

const DEFAULT_SETTLEMENTS: &[&str] = &[
    "文曜", "半嶺鎮", "風舶角", "風舶嶼", "南哨", "大圳頭", "老堰頭", "北嶺腳", "霜林口",
    "風櫃尾", "烏石鼻", "牛埔", "紅土崁", "三號坑口", "水尾寮", "長風草甸", "古河巨木林",
    "赤石坡", "落日岬", "驚濤岩岸", "巨藻淺灣", "黑石灣", "西麓林道", "清泉溪谷",
    "翠冠巨木林", "北坡雲杉林", "淺澤泥口", "白茫洲", "貼山深潭", "青籠關", "西嶺礦谷",
    "東海峭壁", "雲腰雪林", "白塔雪巔",
];

/// 靜態聚落公庫（全島守恆循環經濟）：結餘儲蓄與公共派發之樞紐
pub static SETTLEMENT_TREASURIES: LazyLock<RwLock<HashMap<String, i32>>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for &s in DEFAULT_SETTLEMENTS {
        map.insert(s.to_string(), 2400);
    }
    RwLock::new(map)
});
static ECONOMY_NORMALIZED_ONCE: AtomicBool = AtomicBool::new(false);

/// 若尚未執行過全島閉環經濟初次規範化，於安全時機（未持有 Store 讀鎖時）取得寫鎖執行一次
pub fn ensure_circular_economy_normalized_if_needed() {
    if !ECONOMY_NORMALIZED_ONCE.load(Ordering::SeqCst)
        && let Some(arc) = crate::store::get_store()
        && let Some(mut s) = arc.try_write()
    {
        normalize_and_seed_circular_economy_once(&mut s);
    }
}

/// 計算單一住民在初次初始化時之寫實初始鎂幣（約 20% 匱乏 0..=4 鎂、約 35% 溫飽 5..=24 鎂、約 45% 有餘裕 25..=68 鎂）
fn compute_starting_wallet(seed: u64, role_hint: &str) -> i32 {
    let is_wealthy_role = role_hint.contains("主事")
        || role_hint.contains("店長")
        || role_hint.contains("會長")
        || role_hint.contains("大匠")
        || role_hint.contains("主理")
        || role_hint.contains("名宿")
        || role_hint.contains("東家")
        || role_hint.contains("教務長")
        || role_hint.contains("總站長")
        || role_hint.contains("大廚")
        || role_hint.contains("場主")
        || role_hint.contains("主任");
    let is_poor_role = role_hint.contains("學徒")
        || role_hint.contains("雜役")
        || role_hint.contains("腳夫")
        || role_hint.contains("苦力")
        || role_hint.contains("幫工")
        || role_hint.contains("散客");

    if is_wealthy_role {
        25 + ((seed % 44) as i32)
    } else if is_poor_role || (seed % 100) < 20 {
        (seed % 5) as i32
    } else if (seed % 100) < 55 {
        5 + ((seed % 20) as i32)
    } else {
        25 + ((seed % 44) as i32)
    }
}

/// 全島自由循環經濟單次初次規範化（修復歷史未質押暴衝鎂幣，並注入聚落公庫）
pub fn normalize_and_seed_circular_economy_once(store: &mut crate::store::Store) {
    if ECONOMY_NORMALIZED_ONCE.swap(true, Ordering::SeqCst) {
        return;
    }

    let has_hyper_inflation = store
        .entities
        .values()
        .any(|e| e.kind == "npc" && (e.magnesium > 300 || e.magnesium < 0));

    let treasuries_empty = {
        let guard = SETTLEMENT_TREASURIES.read().unwrap_or_else(|e| e.into_inner());
        guard.is_empty()
    };

    if has_hyper_inflation {
        for e in store.entities.values_mut() {
            if e.kind == "npc" {
                let seed = deterministic_resident_seed(&e.id, e.soul_seed);
                let slot = crate::store::wenyao_household_slot_for(&e.id);
                let role_hint = format!("{} {}", slot.brief_tag, e.display_title);
                let starting_wallet = compute_starting_wallet(seed, &role_hint);
                e.magnesium = starting_wallet;
                if let Some((before_tag, _)) = e.current_activity.split_once("（【") {
                    e.current_activity = before_tag.trim().to_string();
                }
                crate::pg::entity::upsert_async(e.clone());
            }
        }
    }

    if has_hyper_inflation || treasuries_empty {

        let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
        for &s in DEFAULT_SETTLEMENTS {
            treasuries.entry(s.to_string()).or_insert(2400);
        }
    }
}

/// 查詢指定聚落公庫之當前餘額（鎂幣）
pub fn settlement_treasury_balance(settlement: &str) -> i32 {
    let s = clean_settlement_name(settlement);
    let guard = SETTLEMENT_TREASURIES.read().unwrap_or_else(|e| e.into_inner());
    guard.get(s).copied().or_else(|| guard.get(settlement).copied()).unwrap_or(2400)
}

/// 將鎂幣存入指定聚落公庫（例如住民用膳消費、稅金），嚴格守恆
pub fn deposit_to_settlement_treasury(settlement: &str, amount: i32) -> i32 {
    if amount <= 0 {
        return settlement_treasury_balance(settlement);
    }
    let s = clean_settlement_name(settlement);
    let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
    let entry = treasuries.entry(s.to_string()).or_insert(2400);
    *entry = entry.saturating_add(amount);
    *entry
}

/// 計算全島閉環經濟之總守恆鎂幣量（∑ npc.magnesium + ∑ SETTLEMENT_TREASURIES + ∑ LIVE_STRANDED_CARGO_ORDERS escrow）
pub fn total_conserved_magnesium(store: &crate::store::Store) -> i64 {
    let now_unix = crate::game::now_unix();
    {
        let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
        let mut expired_refunds: Vec<(String, i32)> = Vec::new();
        for order in orders.values_mut() {
            if now_unix > order.expires_at_unix && order.escrow_bounty_mg > 0 {
                expired_refunds.push((order.dest_settlement.clone(), order.escrow_bounty_mg));
                order.escrow_bounty_mg = 0;
            }
        }
        if !expired_refunds.is_empty() {
            let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
            for (dest, refund) in expired_refunds {
                let s = clean_settlement_name(&dest);
                *treasuries.entry(s.to_string()).or_insert(2400) += refund;
            }
        }
    }
    let npc_total: i64 = store
        .entities
        .values()
        .filter(|e| e.kind == "npc" || e.kind == "player" || e.kind == "Player")
        .map(|e| i64::from(e.magnesium.max(0)))
        .sum();
    let treasuries_total: i64 = SETTLEMENT_TREASURIES
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .map(|&v| i64::from(v.max(0)))
        .sum();
    let escrow_total: i64 = LIVE_STRANDED_CARGO_ORDERS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .map(|o| i64::from(o.escrow_bounty_mg.max(0)))
        .sum();
    let bank_total: i64 = LIVE_PLAYER_BANK_LEDGER
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .map(|&v| i64::from(v.max(0)))
        .sum();
    npc_total + treasuries_total + escrow_total + bank_total
}

/// 判斷預擬行為選項所對應之生態物資分類
pub fn action_commodity_category(opt: &ActionOptionDef) -> &'static str {
    match opt.image {
        DecisionImageKind::Eat => "food",
        DecisionImageKind::Drink => "drink",
        DecisionImageKind::HealOrTend => "clinic",
        DecisionImageKind::MendOrEquip => "mend",
        DecisionImageKind::WorkOrTrade => match opt.code {
            "work_scavenge_wild_firewood_and_scraps" | "work_take_hard_dock_or_haul_odd_job" => "forage",
            "work_peddle_gathered_goods_at_market" => "peddle",
            "work_cross_settlement_scarcity_arbitrage" => "arbitrage",
            _ => "craft",
        },
        _ => "food",
    }
}

/// 依據聚落即時物資供需緊缺倍率，動態計算行為消費支出（零成本求生選項恆為 0 鎂，其餘至少 1 鎂）
pub fn dynamic_action_cost_mg(settlement: &str, opt: &ActionOptionDef) -> i32 {
    if opt.cost_mg <= 0 {
        return 0;
    }
    let (ratio, _, _) = crate::game::object_ecology::settlement_scarcity_multiplier_for_category(
        settlement,
        action_commodity_category(opt),
    );
    ((opt.cost_mg as f32 * ratio).round() as i32).max(1)
}

/// 依據聚落即時物資供需緊缺倍率，動態計算勞作或調撥所得工資（零報酬選項恆為 0 鎂，其餘至少 2 鎂）
pub fn dynamic_action_earn_mg(settlement: &str, opt: &ActionOptionDef) -> i32 {
    if opt.earn_mg <= 0 {
        return 0;
    }
    let (ratio, _, _) = crate::game::object_ecology::settlement_scarcity_multiplier_for_category(
        settlement,
        action_commodity_category(opt),
    );
    ((opt.earn_mg as f32 * ratio).round() as i32).max(2)
}

/// 從聚落公庫支付勞作／採集／交割工資（若公庫吃緊，則由在地商賈注資批發進貨，嚴格守恆）
pub fn pay_from_settlement_treasury(
    store: &mut crate::store::Store,
    worker_id: &str,
    settlement: &str,
    requested_mg: i32,
) -> i32 {
    if requested_mg <= 0 {
        return 0;
    }
    let s = clean_settlement_name(settlement);

    let current_bal = {
        let treasuries = SETTLEMENT_TREASURIES.read().unwrap_or_else(|e| e.into_inner());
        treasuries.get(s).copied().unwrap_or(0)
    };

    if current_bal < 600 || current_bal < requested_mg {
        let mut merchants: Vec<String> = store
            .entities
            .values()
            .filter(|e| {
                e.kind == "npc"
                    && e.id != worker_id
                    && e.magnesium > 45
                    && (e.display_title.contains(s) || e.current_activity.contains(s))
            })
            .map(|e| e.id.clone())
            .collect();

        if merchants.is_empty() {
            merchants = store
                .entities
                .values()
                .filter(|e| e.kind == "npc" && e.id != worker_id && e.magnesium > 55)
                .map(|e| e.id.clone())
                .collect();
        }

        merchants.truncate(3);

        for m_id in merchants {
            let (fee, m_title) = if let Some(m) = store.entities.get_mut(&m_id) {
                let restock_fee = ((m.magnesium - 40) / 3).clamp(2, 12);
                let actual_fee = restock_fee.min(m.magnesium.max(0));
                m.magnesium -= actual_fee;
                crate::pg::entity::upsert_async(m.clone());
                (actual_fee, m.display_title.clone())
            } else {
                (0, String::new())
            };

            if fee > 0 {
                {
                    let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
                    *treasuries.entry(s.to_string()).or_insert(0) += fee;
                }
                crate::game::object_ecology::supply_settlement_commodity_from_work(s, "restock", &m_title);
            }
        }
    }

    let actual_pay = {
        let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
        let bal = treasuries.entry(s.to_string()).or_insert(0);
        let take = requested_mg.min((*bal).max(0));
        *bal -= take;
        take
    };

    if actual_pay > 0 {
        if let Some(worker) = store.entities.get_mut(worker_id) {
            worker.magnesium = worker.magnesium.saturating_add(actual_pay);
            crate::pg::entity::upsert_async(worker.clone());
        } else {
            let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
            *treasuries.entry(s.to_string()).or_insert(0) += actual_pay;
            return 0;
        }
    }

    actual_pay
}

/// 玩家與聚落公庫雙向結算（嚴格符合 SFC 封閉經濟總鎂幣守恆）。
/// - delta_mg > 0: 自聚落公庫扣除相應款項並發放至玩家帳戶，回傳實際領取鎂幣（>= 0）。
/// - delta_mg < 0: 從玩家帳戶扣除款項並納入聚落公庫，回傳實際扣除之負數鎂幣（<= 0）。
pub fn settle_player_with_settlement_treasury(
    player_id: &str,
    settlement: &str,
    delta_mg: i32,
) -> i32 {
    let resolved_place = if settlement.starts_with("grid:") {
        crate::db::get_room_name(settlement).ok().unwrap_or_else(|| settlement.to_string())
    } else {
        settlement.to_string()
    };
    let Some(arc) = crate::store::get_store() else {
        return 0;
    };
    if delta_mg > 0 {
        let mut store = arc.write();
        pay_from_settlement_treasury(&mut store, player_id, &resolved_place, delta_mg)
    } else if delta_mg < 0 {
        let mut store = arc.write();
        let s = clean_settlement_name(&resolved_place);
        if let Some(ent) = store.entities.get_mut(player_id) {
            let actual_deducted = (-delta_mg).min(ent.magnesium.max(0));
            if actual_deducted > 0 {
                ent.magnesium -= actual_deducted;
                crate::pg::entity::upsert_async(ent.clone());
                let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
                *treasuries.entry(s.to_string()).or_insert(2400) += actual_deducted;
            }
            -actual_deducted
        } else {
            0
        }
    } else {
        0
    }
}

/// 住民日常生活消費支出（流向現場主事／同行／貧弱街坊，餘額納入聚落公庫，嚴格守恆）
pub fn spend_into_settlement_circulation(
    store: &mut crate::store::Store,
    payer_id: &str,
    settlement: &str,
    dest_name: &str,
    cost_mg: i32,
    peer_support: bool,
) -> i32 {
    if cost_mg <= 0 {
        return 0;
    }
    let actual_cost = match store.entities.get_mut(payer_id) {
        Some(payer) => {
            let payable = cost_mg.min(payer.magnesium.max(0));
            if payable <= 0 {
                return 0;
            }
            payer.magnesium -= payable;
            crate::pg::entity::upsert_async(payer.clone());
            payable
        }
        None => return 0,
    };

    let s = clean_settlement_name(settlement);

    if peer_support {
        let candidates: Vec<String> = store
            .entities
            .values()
            .filter(|e| e.kind == "npc" && e.id != payer_id && e.magnesium <= 8)
            .map(|e| e.id.clone())
            .collect();

        let (_to_recipient, to_treasury) = if !candidates.is_empty() {
            let idx = (deterministic_resident_seed(payer_id, None) as usize + actual_cost as usize) % candidates.len();
            let rid = candidates[idx].clone();
            let half = actual_cost / 2;
            if half > 0 {
                if let Some(rec) = store.entities.get_mut(&rid) {
                    rec.magnesium = rec.magnesium.saturating_add(half);
                    crate::pg::entity::upsert_async(rec.clone());
                }
            }
            (half, actual_cost - half)
        } else {
            (0, actual_cost)
        };

        if to_treasury > 0 {
            let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
            *treasuries.entry(s.to_string()).or_insert(0) += to_treasury;
        }
    } else {
        let dest_clean = dest_name.rsplit('・').next().unwrap_or(dest_name).trim();
        let candidates: Vec<String> = store
            .entities
            .values()
            .filter(|e| {
                e.kind == "npc"
                    && e.id != payer_id
                    && ((!dest_clean.is_empty()
                        && (e.current_activity.contains(dest_clean)
                            || e.display_title.contains(dest_clean)))
                        || (e.current_activity.contains(s) || e.display_title.contains(s)))
            })
            .map(|e| e.id.clone())
            .collect();

        let to_staff = actual_cost / 4;
        let to_treasury = actual_cost - to_staff;

        if !candidates.is_empty() && to_staff > 0 {
            let idx = (deterministic_resident_seed(payer_id, None) as usize + actual_cost as usize) % candidates.len();
            let rid = candidates[idx].clone();
            if let Some(rec) = store.entities.get_mut(&rid) {
                rec.magnesium = rec.magnesium.saturating_add(to_staff);
                crate::pg::entity::upsert_async(rec.clone());
            }
            if to_treasury > 0 {
                let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
                *treasuries.entry(s.to_string()).or_insert(0) += to_treasury;
            }
        } else {
            let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
            *treasuries.entry(s.to_string()).or_insert(0) += actual_cost;
        }
    }

    actual_cost
}

/// 執行一輪預擬動作之閉環經濟結算與體感生理需求調節
pub fn execute_conserved_action_settlement(
    store: &mut crate::store::Store,
    actor_id: &str,
    settlement: &str,
    dest_name: &str,
    action_code: &str,
    _now_unix: i64,
) -> i32 {
    let opt = lookup_action_option(action_code).unwrap_or(&PREDEFINED_ACTION_CATALOG[0]);
    let peer_support = matches!(
        action_code,
        "eat_treat_peers_at_table"
            | "heal_buy_extra_salve_and_cover_peer_bill"
            | "mend_gift_repaired_vessel_to_junior"
            | "social_bail_out_and_sponsor_struggling_peer"
            | "work_mentor_junior_and_delegate_routine"
    );

    let dyn_cost = dynamic_action_cost_mg(settlement, opt);
    let dyn_earn = dynamic_action_earn_mg(settlement, opt);

    let actor_title = store
        .entities
        .get(actor_id)
        .map(|e| e.display_title.clone())
        .unwrap_or_default();

    let spent = if dyn_cost > 0 {
        let s = spend_into_settlement_circulation(store, actor_id, settlement, dest_name, dyn_cost, peer_support);
        if s >= dyn_cost
            && matches!(
                opt.image,
                DecisionImageKind::Eat
                    | DecisionImageKind::Drink
                    | DecisionImageKind::HealOrTend
                    | DecisionImageKind::MendOrEquip
            )
        {
            crate::game::object_ecology::consume_settlement_commodity_for_action(
                settlement,
                action_commodity_category(opt),
                &actor_title,
            );
        }
        s
    } else {
        0
    };

    let earned = if dyn_earn > 0 && (dyn_cost == 0 || spent >= dyn_cost) {
        crate::game::object_ecology::supply_settlement_commodity_from_work(
            settlement,
            action_commodity_category(opt),
            &actor_title,
        );
        pay_from_settlement_treasury(store, actor_id, settlement, dyn_earn)
    } else {
        0
    };

    earned - spent
}

/// 依據住民生理與處境需求，由內向外推進決策意象並依真實有效鎂幣擇定行為動作
pub fn plan_next_causal_action(
    entity_id: &str,
    role_hint: &str,
    settlement: &str,
    seed: u64,
    prev_dest_name: &str,
    streaks: u8,
    has_affliction: bool,
    vessel_worn: bool,
    real_magnesium: i32,
    hour: u32,
    scarcity_focus: Option<&str>,
) -> (NpcCausalState, &'static ActionOptionDef) {
    let already_existed = LIVE_NPC_CAUSAL_STATES
        .read()
        .ok()
        .is_some_and(|m| m.contains_key(entity_id));
    let mut state = get_or_init_causal_state(entity_id, role_hint, settlement, seed);

    // 先結算上一輪已完成動作對生理狀態之舒緩與預計鎂幣收支，再推進下一輪時間刻度與需求
    let mut effective_mg = real_magnesium;
    if already_existed
        && let Some(prev_opt) = lookup_action_option(&state.chosen_action_code)
    {
        let dyn_cost = dynamic_action_cost_mg(settlement, prev_opt);
        let dyn_earn = dynamic_action_earn_mg(settlement, prev_opt);
        let est_spent = dyn_cost.min(real_magnesium.max(0));
        effective_mg = (real_magnesium - est_spent + dyn_earn).max(0);
        match prev_opt.image {
            DecisionImageKind::Eat => {
                let red = if dyn_cost > 0 { 55 } else { 35 };
                state.hunger = state.hunger.saturating_sub(red);
            }
            DecisionImageKind::Drink => {
                state.thirst = state.thirst.saturating_sub(55);
            }
            DecisionImageKind::SleepOrRest => {
                state.fatigue = state.fatigue.saturating_sub(60);
                state.stress = state.stress.saturating_sub(18);
                if state.stress < 72 {
                    state.stress_break_active = false;
                }
            }
            DecisionImageKind::WorkOrTrade => {
                state.hunger = state.hunger.saturating_add(8).min(100);
                state.thirst = state.thirst.saturating_add(8).min(100);
                state.fatigue = state.fatigue.saturating_add(10).min(100);
            }
            _ => {}
        }
    }

    state.life_step_tick = state.life_step_tick.saturating_add(1);
    state.same_task_repetitions = state.same_task_repetitions.saturating_add(1);
    state.hunger = state.hunger.saturating_add(12).min(100);
    state.thirst = state.thirst.saturating_add(14).min(100);
    state.fatigue = state.fatigue.saturating_add(10).min(100);
    let vessel = resolve_vocational_vessel(role_hint);
    let s_clean = if settlement.trim().is_empty() { "文曜" } else { clean_settlement_name(settlement) };
    let is_rest_or_heal = prev_dest_name.contains("浴")
        || prev_dest_name.contains("湯")
        || prev_dest_name.contains("食堂")
        || prev_dest_name.contains("茶")
        || prev_dest_name.contains("診")
        || prev_dest_name.contains("居")
        || prev_dest_name.contains("宅");
    if is_rest_or_heal {
        state.stress = state.stress.saturating_sub(16);
        if state.stress < 72 {
            state.stress_break_active = false;
        }
    } else {
        let mut stress_delta: u8 = 4 + streaks.min(6).saturating_mul(3);
        if has_affliction {
            stress_delta = stress_delta.saturating_add(12);
        }
        if vessel_worn && state.cornerstone == Cornerstone::Craft {
            stress_delta = stress_delta.saturating_add(12);
        }
        if effective_mg <= 4 {
            stress_delta = stress_delta.saturating_add(8);
        }
        state.stress = state.stress.saturating_add(stress_delta).min(100);
    }

    if hour == 6 {
        state.carryover_from_yesterday = format!(
            "延續昨日第 {} 回工序手感與生計步調，起床接續今日生計",
            state.same_task_repetitions
        );
    }

    let live_scarcity = crate::game::object_ecology::settlement_scarcity_signal(settlement);
    let (scarce_label, scarce_ratio) = if let Some((_, ref name, r)) = live_scarcity {
        (Some(name.as_str()), r)
    } else {
        (scarcity_focus, 1.0)
    };

    if vessel_worn {
        state.last_surprise_delta = Some(format!(
            "察覺手中「{}」分寸微鈍，先改道{}修整器況再續原訂差事",
            vessel.display_name, vessel.maintenance_venue_keyword
        ));
        state.plan_steps = [
            format!("先赴{}修整「{}」", vessel.maintenance_venue_keyword, vessel.display_name),
            format!("重返{s_clean}完成手頭耽擱的活計"),
            format!("收工後與【{}】核對今日人情帳", state.hooks.first().map(|h| h.target_name.as_str()).unwrap_or("街坊")),
        ];
        state.plan_cursor = 0;
    } else if let Some(actual_item) = scarce_label
        && !actual_item.is_empty()
        && actual_item != state.belief_scarcity_item
    {
        let old_item = state.belief_scarcity_item.clone();
        state.belief_scarcity_item = actual_item.to_string();
        state.last_surprise_delta = Some(format!(
            "抵埠始知{s_clean}行情已由「{old_item}」轉缺「{actual_item}」，當場改換採辦與趕製標的"
        ));
        state.plan_steps = derive_default_3step_plan(
            role_hint,
            s_clean,
            vessel.display_name,
            state.cornerstone,
            actual_item,
        );
        state.plan_cursor = 1;
    } else if !state.plan_steps.is_empty() {
        state.plan_cursor = (state.plan_cursor + 1) % (state.plan_steps.len() as u8);
        if state.plan_cursor == 0 {
            state.last_surprise_delta = None;
        }
    }

    if state.stress >= 72 {
        state.stress_break_active = true;
    }
    let image = if has_affliction {
        DecisionImageKind::HealOrTend
    } else if state.hunger >= 64 || (matches!(hour, 7 | 12 | 18) && state.hunger >= 35) {
        DecisionImageKind::Eat
    } else if state.thirst >= 62 || (matches!(hour, 10 | 15) && state.thirst >= 38) {
        DecisionImageKind::Drink
    } else if state.fatigue >= 68 || state.stress >= 76 || !(6..=21).contains(&hour) {
        DecisionImageKind::SleepOrRest
    } else if vessel_worn {
        DecisionImageKind::MendOrEquip
    } else if (scarce_ratio >= 1.25 || scarce_label.is_some())
        && state.hunger < 68
        && state.thirst < 68
        && state.fatigue < 70
        && (7..=19).contains(&hour)
    {
        DecisionImageKind::WorkOrTrade
    } else if effective_mg <= 4 || (8..=17).contains(&hour) {
        DecisionImageKind::WorkOrTrade
    } else {
        DecisionImageKind::InteractOrUnwind
    };

    let base_eligible = eligible_actions_for_image(image, effective_mg);
    let affordable_in_band: Vec<&'static ActionOptionDef> = base_eligible
        .iter()
        .copied()
        .filter(|opt| dynamic_action_cost_mg(settlement, opt) <= effective_mg.max(0))
        .collect();
    let eligible: Vec<&'static ActionOptionDef> = if !affordable_in_band.is_empty() {
        affordable_in_band
    } else {
        PREDEFINED_ACTION_CATALOG
            .iter()
            .filter(|opt| {
                opt.image == image
                    && effective_mg.max(0) >= opt.min_mg
                    && dynamic_action_cost_mg(settlement, opt) <= effective_mg.max(0)
            })
            .collect()
    };
    let margin = MarginBand::from_magnesium(effective_mg);

    let chosen = if image == DecisionImageKind::WorkOrTrade && scarce_label.is_some() {
        let preferred_code = match margin {
            MarginBand::Scarcity => "work_scavenge_wild_firewood_and_scraps",
            MarginBand::Subsistence => {
                if (state.life_step_tick + seed).is_multiple_of(2) {
                    "work_peddle_gathered_goods_at_market"
                } else {
                    "work_restock_needed_raw_materials"
                }
            }
            MarginBand::Abundance => "work_cross_settlement_scarcity_arbitrage",
        };
        eligible
            .iter()
            .copied()
            .find(|opt| opt.code == preferred_code)
            .unwrap_or_else(|| {
                choose_action_from_eligible(
                    &eligible,
                    state.cornerstone,
                    state.life_step_tick,
                    seed,
                )
            })
    } else {
        choose_action_from_eligible(
            &eligible,
            state.cornerstone,
            state.life_step_tick,
            seed,
        )
    };

    if let Some(item_name) = scarce_label {
        state.belief_scarcity_item = item_name.to_string();
        state.belief_summary = if scarce_ratio >= 1.5 {
            format!("聽聞坊間「{item_name}」頗為緊俏，價錢比往常貴上不少")
        } else {
            format!("留意到街市上「{item_name}」銷路甚快，行情隱約看漲")
        };
    } else {
        state.belief_summary = "坊間各色物料供需平穩，行情如常".to_string();
    }

    state.decision_image_code = image.code().to_string();
    state.eligible_option_count = base_eligible.len() as u8;
    state.chosen_action_code = chosen.code.to_string();
    state.active_need = format!(
        "{}（懷揣 {} 枚鎂幣，{}）",
        image.label(),
        effective_mg.max(0),
        margin.label()
    );
    state.image_decision = format!(
        "衡量{}的處境，承接第 {} 回手頭活計，決定先「{}」",
        margin.label(),
        state.same_task_repetitions,
        chosen.action_title
    );
    state.situated_action = format!(
        "承接第 {} 回熟稔手感，{}",
        state.same_task_repetitions,
        chosen.action_behavior
    );

    if let Ok(mut map) = LIVE_NPC_CAUSAL_STATES.write() {
        map.insert(entity_id.to_string(), state.clone());
    }

    (state, chosen)
}

/// 確保因果狀態中之可選動作數與擇定動作嚴格對齊當前有效鎂幣（匱乏 2 項、溫飽 3 項、有餘裕 5~6 項）
pub fn sync_causal_state_with_magnesium(
    state: &mut NpcCausalState,
    magnesium: i32,
    seed: u64,
) {
    let image = match state.decision_image_code.as_str() {
        "eat" => DecisionImageKind::Eat,
        "drink" => DecisionImageKind::Drink,
        "sleep_or_rest" => DecisionImageKind::SleepOrRest,
        "heal_or_tend" => DecisionImageKind::HealOrTend,
        "mend_or_equip" => DecisionImageKind::MendOrEquip,
        "interact_or_unwind" => DecisionImageKind::InteractOrUnwind,
        _ => DecisionImageKind::WorkOrTrade,
    };
    let margin = MarginBand::from_magnesium(magnesium);
    let eligible = eligible_actions_for_image(image, magnesium);
    let chosen = eligible
        .iter()
        .copied()
        .find(|opt| opt.code == state.chosen_action_code)
        .unwrap_or_else(|| {
            choose_action_from_eligible(
                &eligible,
                state.cornerstone,
                state.life_step_tick,
                seed,
            )
        });
    state.decision_image_code = image.code().to_string();
    state.eligible_option_count = eligible.len() as u8;
    state.chosen_action_code = chosen.code.to_string();
    state.active_need = format!(
        "{}（懷揣 {} 枚鎂幣，{}）",
        image.label(),
        magnesium.max(0),
        margin.label()
    );
    state.image_decision = format!(
        "衡量{}的處境，承接第 {} 回手頭活計，決定先「{}」",
        margin.label(),
        state.same_task_repetitions,
        chosen.action_title
    );
    state.situated_action = format!(
        "承接第 {} 回熟稔手感，{}",
        state.same_task_repetitions,
        chosen.action_behavior
    );
}

/// 依代碼查閱預擬行為選項定義
pub fn lookup_action_option(code: &str) -> Option<&'static ActionOptionDef> {
    PREDEFINED_ACTION_CATALOG.iter().find(|opt| opt.code == code)
}

pub fn coherent_place_for_venue_req(venue_req: VenueRequirement, dest_name: &str) -> String {
    let settlement = clean_settlement_name(dest_name);
    let dest_leaf = dest_name.rsplit('・').next().unwrap_or(dest_name).trim();
    match venue_req {
        VenueRequirement::HomeOrKitchen => {
            let is_home = (dest_leaf.contains('宅')
                || dest_leaf.contains('居')
                || dest_leaf.contains('屋')
                || dest_leaf.contains('家')
                || dest_leaf.contains('宿')
                || dest_leaf.contains('寮')
                || dest_leaf.contains('舍'))
                && !dest_leaf.contains("食堂")
                && !dest_leaf.contains("飯")
                && !dest_leaf.contains("茶屋")
                && !dest_leaf.contains("湯屋")
                && !dest_leaf.contains("商");
            if is_home {
                dest_leaf.to_string()
            } else {
                format!("{settlement}自家屋舍廚下")
            }
        }
        VenueRequirement::WildOrOutskirts => {
            let is_wild = (dest_leaf.contains('林')
                || dest_leaf.contains('溪')
                || dest_leaf.contains('谷')
                || dest_leaf.contains('山')
                || dest_leaf.contains('坡')
                || dest_leaf.contains('岸')
                || dest_leaf.contains('甸')
                || dest_leaf.contains('道')
                || dest_leaf.contains('嶺')
                || dest_leaf.contains('野')
                || dest_leaf.contains('灣')
                || dest_leaf.contains('潭'))
                && !dest_leaf.contains("商行")
                && !dest_leaf.contains("貨運")
                && !dest_leaf.contains("工坊")
                && !dest_leaf.contains("食堂")
                && !dest_leaf.contains("茶屋");
            if is_wild {
                dest_leaf.to_string()
            } else {
                format!("{settlement}外圍山林近郊")
            }
        }
        VenueRequirement::CanteenOrStall => {
            if dest_leaf.contains("食堂")
                || dest_leaf.contains('飯')
                || dest_leaf.contains('麵')
                || dest_leaf.contains('攤')
                || dest_leaf.contains("熱食")
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "三水大食堂".to_string()
            } else {
                format!("{settlement}街角熱食麵攤")
            }
        }
        VenueRequirement::TeahouseOrTavern => {
            if dest_leaf.contains('茶')
                || dest_leaf.contains('酒')
                || dest_leaf.contains("旅社")
                || dest_leaf.contains("茶寮")
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "東橋旅社與山風茶屋".to_string()
            } else {
                format!("{settlement}路邊茶寮")
            }
        }
        VenueRequirement::WellOrStream => {
            if dest_leaf.contains('井')
                || dest_leaf.contains('泉')
                || dest_leaf.contains('溪')
                || dest_leaf.contains('水')
                || dest_leaf.contains('圳')
            {
                dest_leaf.to_string()
            } else {
                format!("{settlement}街角山泉公井")
            }
        }
        VenueRequirement::Bathhouse => {
            if dest_leaf.contains('浴') || dest_leaf.contains('湯') {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "泉香溫水浴場".to_string()
            } else {
                format!("{settlement}溫水石湯屋")
            }
        }
        VenueRequirement::InnOrHome => {
            if dest_leaf.contains('旅')
                || dest_leaf.contains('宿')
                || dest_leaf.contains("旅社")
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "東橋旅社".to_string()
            } else {
                format!("{settlement}旅社通鋪")
            }
        }
        VenueRequirement::Clinic => {
            if dest_leaf.contains('診') || dest_leaf.contains('醫') || dest_leaf.contains('藥') {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "溫氏脈絡診所".to_string()
            } else {
                format!("{settlement}草木藥鋪診所")
            }
        }
        VenueRequirement::ForgeOrWorkshop => {
            if dest_leaf.contains('坊')
                || dest_leaf.contains('鍛')
                || dest_leaf.contains('鐵')
                || dest_leaf.contains('鋪')
                || dest_leaf.contains('室')
                || dest_leaf.contains('所')
                || dest_leaf.contains('廠')
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "岑氏第一鍛造工坊".to_string()
            } else {
                format!("{settlement}百工修繕坊")
            }
        }
        VenueRequirement::DockOrStation => {
            if dest_leaf.contains('站')
                || dest_leaf.contains('運')
                || dest_leaf.contains("碼頭")
                || dest_leaf.contains('港')
                || dest_leaf.contains('埠')
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "西門車馬聯運總站".to_string()
            } else {
                format!("{settlement}車馬聯運貨站")
            }
        }
        VenueRequirement::Market => {
            if dest_leaf.contains('市')
                || dest_leaf.contains('商')
                || dest_leaf.contains('集')
                || dest_leaf.contains('攤')
                || dest_leaf.contains('店')
                || dest_leaf.contains('行')
                || dest_leaf.contains('鋪')
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "三水總商行市集".to_string()
            } else {
                format!("{settlement}街心市集攤區")
            }
        }
        VenueRequirement::BridgeOrPlaza => {
            if dest_leaf.contains('橋')
                || dest_leaf.contains('坪')
                || dest_leaf.contains("廣場")
                || dest_leaf.contains('廊')
                || dest_leaf.contains('岬')
                || dest_leaf.contains('閣')
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "三水匯流大石橋".to_string()
            } else {
                format!("{settlement}街心石坪廊下")
            }
        }
        VenueRequirement::OwnWorkplace => {
            if dest_leaf.is_empty() {
                format!("{settlement}案頭崗位")
            } else {
                dest_leaf.to_string()
            }
        }
    }
}

/// 格式化預擬定點停留活動（在前端與狀態表中真實反應依現況與選項表擇定的自然人話活動描述）
pub fn format_predefined_stay_activity(
    _dest_name: &str,
    causal: &NpcCausalState,
    _magnesium: i32,
) -> String {
    let opt = lookup_action_option(&causal.chosen_action_code).unwrap_or(&PREDEFINED_ACTION_CATALOG[0]);
    let beh = opt.action_behavior.trim();
    if beh.starts_with('正') {
        beh.to_string()
    } else {
        format!("正{}", beh)
    }
}

pub static LIVE_NPC_CAUSAL_STATES: LazyLock<RwLock<HashMap<String, NpcCausalState>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

const HOOK_PEER_NAMES: &[&str] = &[
    "謝宇翔", "林雅筑", "許家豪", "陳冠宇", "黃詩涵", "張哲銘", "李佳穎", "王俊傑",
    "吳佩珊", "劉建宏", "蔡欣怡", "楊承翰", "鄭雅婷", "郭志豪", "洪淑芬", "邱冠廷",
];

pub fn clean_settlement_name(raw: &str) -> &'static str {
    const PLACES: &[(&str, &str)] = &[
        ("風舶角", "風舶角"),
        ("風舶嶼", "風舶嶼"),
        ("大圳頭", "大圳頭"),
        ("半嶺鎮", "半嶺鎮"),
        ("南哨", "南哨"),
        ("老堰頭", "老堰頭"),
        ("北嶺腳", "北嶺腳"),
        ("霜林口", "霜林口"),
        ("風櫃尾", "風櫃尾"),
        ("烏石鼻", "烏石鼻"),
        ("牛埔", "牛埔"),
        ("紅土崁", "紅土崁"),
        ("紅土", "紅土崁"),
        ("三號坑口", "三號坑口"),
        ("水尾寮", "水尾寮"),
        ("長風草甸", "長風草甸"),
        ("草甸", "長風草甸"),
        ("古河巨木林", "古河巨木林"),
        ("赤石坡", "赤石坡"),
        ("落日岬", "落日岬"),
        ("驚濤岩岸", "驚濤岩岸"),
        ("巨藻淺灣", "巨藻淺灣"),
        ("黑石灣", "黑石灣"),
        ("西麓林道", "西麓林道"),
        ("清泉溪谷", "清泉溪谷"),
        ("翠冠巨木林", "翠冠巨木林"),
        ("北坡雲杉林", "北坡雲杉林"),
        ("淺澤泥口", "淺澤泥口"),
        ("白茫洲", "白茫洲"),
        ("貼山深潭", "貼山深潭"),
        ("青籠關", "青籠關"),
        ("西嶺礦谷", "西嶺礦谷"),
        ("東海峭壁", "東海峭壁"),
        ("雲腰雪林", "雲腰雪林"),
        ("白塔雪巔", "白塔雪巔"),
        ("講堂街", "文曜"),
        ("三水", "文曜"),
        ("文曜", "文曜"),
    ];
    for &(kw, canonical) in PLACES {
        if raw.contains(kw) {
            return canonical;
        }
    }
    "文曜"
}

fn seed_hooks_for_resident(
    role_hint: &str,
    settlement: &str,
    seed: u64,
) -> Vec<RelationalHook> {
    let s = clean_settlement_name(settlement);
    let peer_a = HOOK_PEER_NAMES[(seed as usize) % HOOK_PEER_NAMES.len()];
    let peer_b = HOOK_PEER_NAMES[((seed as usize) + 5) % HOOK_PEER_NAMES.len()];

    let primary = if role_hint.contains("獵") || role_hint.contains("採") || role_hint.contains("巡") {
        RelationalHook {
            kind_label: "恩情".to_string(),
            target_name: peer_a.to_string(),
            summary: format!("欠【{peer_a}】一回山道解圍之恩，遇其合社召集必優先助陣"),
            intensity: 4,
            target_id: String::new(),
            updated_at_unix: 0,
        }
    } else if role_hint.contains("鐵") || role_hint.contains("木") || role_hint.contains("學徒") {
        RelationalHook {
            kind_label: "師徒代償".to_string(),
            target_name: peer_a.to_string(),
            summary: format!("與同門【{peer_a}】共擔{s}工坊損耗，見其器鈍必出手幫襯"),
            intensity: 4,
            target_id: String::new(),
            updated_at_unix: 0,
        }
    } else if role_hint.contains("商") || role_hint.contains("主事") || role_hint.contains("帳房") {
        RelationalHook {
            kind_label: "直供紅契".to_string(),
            target_name: peer_a.to_string(),
            summary: format!("與【{peer_a}】訂有{s}大宗物料直供契，貨到優先交割"),
            intensity: 3,
            target_id: String::new(),
            updated_at_unix: 0,
        }
    } else if role_hint.contains("家眷") || role_hint.contains("內當家") || role_hint.contains("食堂") {
        RelationalHook {
            kind_label: "熟客欠單".to_string(),
            target_name: peer_a.to_string(),
            summary: format!("記著街坊【{peer_a}】在{s}留下的熟客人情帳，逢集優先留貨"),
            intensity: 3,
            target_id: String::new(),
            updated_at_unix: 0,
        }
    } else {
        RelationalHook {
            kind_label: "同袍信約".to_string(),
            target_name: peer_a.to_string(),
            summary: format!("與{s}【{peer_a}】約定互通坊間行情與山海異兆"),
            intensity: 3,
            target_id: String::new(),
            updated_at_unix: 0,
        }
    };

    let secondary = match (seed / 7) % 3 {
        0 => RelationalHook {
            kind_label: "宿怨壓價".to_string(),
            target_name: peer_b.to_string(),
            summary: format!("曾遭【{peer_b}】在{s}市集壓價挑剔，交手時格外留心成色"),
            intensity: 2,
            target_id: String::new(),
            updated_at_unix: 0,
        },
        1 => RelationalHook {
            kind_label: "熟客欠單".to_string(),
            target_name: peer_b.to_string(),
            summary: format!("手頭還記著與【{peer_b}】待結的半筆物料尾款"),
            intensity: 2,
            target_id: String::new(),
            updated_at_unix: 0,
        },
        _ => RelationalHook {
            kind_label: "恩情".to_string(),
            target_name: peer_b.to_string(),
            summary: format!("雨夜曾受【{peer_b}】借火避雨之惠，心裡一直記掛著還禮"),
            intensity: 3,
            target_id: String::new(),
            updated_at_unix: 0,
        },
    };

    vec![primary, secondary]
}

pub fn derive_default_3step_plan(
    role_hint: &str,
    settlement: &str,
    vessel_name: &str,
    cornerstone: Cornerstone,
    belief_item: &str,
) -> [String; 3] {
    let s = clean_settlement_name(settlement);
    let item = if belief_item.trim().is_empty() { "山海常備物料" } else { belief_item.trim() };
    let is_blade = vessel_name.contains('刀') || vessel_name.contains('鎬') || vessel_name.contains('剪');

    if role_hint.contains("獵") || role_hint.contains("採") || role_hint.contains("樵") {
        let prep = if is_blade {
            format!("先檢視「{vessel_name}」刃口與行囊繩索")
        } else {
            format!("先收整隨身「{vessel_name}」與採集行囊")
        };
        [
            prep,
            format!("入山勘查獸徑並採集「{item}」與山林原材"),
            format!("返{s}市集交割換取鎂幣並歇腳"),
        ]
    } else if role_hint.contains("商") || role_hint.contains("主事") || role_hint.contains("執事") || role_hint.contains("帳房") {
        [
            format!("撥校「{vessel_name}」盤點{s}庫房現貨底數"),
            format!("收攏坊間急缺的「{item}」並調撥價差"),
            format!("結清今日熟客欠單後赴{s}食堂用膳"),
        ]
    } else if (role_hint.contains("書") || role_hint.contains("書院") || role_hint.contains("講") || role_hint.contains("學者") || role_hint.contains("算")) && !role_hint.contains("學徒") {
        [
            format!("展平「{vessel_name}」核對{s}今日風物與卷宗"),
            format!("於{s}講堂書齋纂錄「{item}」行情與山海考據"),
            format!("散學後巡看{s}街坊並與同道印證筆記"),
        ]
    } else if role_hint.contains("醫") || role_hint.contains("藥") || role_hint.contains("診") {
        [
            format!("拭淨「{vessel_name}」並清點{s}診所草藥存量"),
            format!("搗製應時傷藥並為{s}傷患切脈調理"),
            format!("赴{s}市集探問「{item}」與短缺藥引行情"),
        ]
    } else if role_hint.contains("水輪") || role_hint.contains("石磨") || role_hint.contains("磨坊") || role_hint.contains("碾") {
        [
            format!("開啟{s}引水閘門，以「{vessel_name}」校準上下磨盤齒距與銅網篩眼"),
            format!("藉湍急水輪分動大磨，將大圳頭新穀與「{item}」細碾分篩並熬製溫米漿"),
            format!("封妥米漿保溫桶發往文曜早市，晚間赴{s}堰渠水畔食堂與北郊農戶核對明日水門時刻"),
        ]
    } else if role_hint.contains("鐵") || role_hint.contains("鍛") || role_hint.contains("木工") || role_hint.contains("木匠") || role_hint.contains("木器") || role_hint.contains("竹") || role_hint.contains("工坊") {
        [
            format!("先於{s}工坊預熱爐火並校準「{vessel_name}」"),
            format!("趕製坊間急需的「{item}」器物批次"),
            format!("核對工坊成色後赴{s}茶寮舒緩筋骨"),
        ]
    } else if role_hint.contains("家眷") || role_hint.contains("內當家") || role_hint.contains("持家") {
        [
            format!("挽起「{vessel_name}」清點家中米糧與柴薪"),
            format!("赴{s}市集置辦「{item}」與今日鮮蔬"),
            format!("返家備妥熱飯並料理家宅內外整潔"),
        ]
    } else {
        [
            format!("依【{}】本色整頓隨身「{vessel_name}」", cornerstone.label()),
            format!("於{s}按部就班完成「{item}」相關差事"),
            format!("收工結清荷包後赴{s}街坊歇息敘話"),
        ]
    }
}

/// 格式化生命直線時間刻度標記（證明生命是一條直線，時間永不歸零）
pub fn format_time_scale_mark(state: &NpcCausalState, now_unix: i64) -> String {
    let day = if now_unix > 0 {
        ((now_unix / 3600) % 1000).max(1)
    } else {
        1
    };
    format!(
        "入島第 {} 日・歷練第 {} 步（同業累計第 {} 回）",
        day, state.life_step_tick, state.same_task_repetitions
    )
}

/// 取得或初始化單一住民的因果狀態
pub fn get_or_init_causal_state(
    entity_id: &str,
    role_hint: &str,
    settlement: &str,
    seed: u64,
) -> NpcCausalState {
    if let Ok(guard) = LIVE_NPC_CAUSAL_STATES.read()
        && let Some(existing) = guard.get(entity_id).cloned()
    {
        return existing;
    }
    let s = clean_settlement_name(settlement);
    let vessel = resolve_vocational_vessel(role_hint);
    let cornerstone = Cornerstone::from_role_and_seed(role_hint, seed);
    let belief_item = crate::game::object_ecology::scarcest_item_for_settlement(s)
        .map(|(_id, nm, _cur, _max, _pr)| nm)
        .unwrap_or_else(|| "山野鮮藥草".to_string());
    let plan_steps = derive_default_3step_plan(
        role_hint,
        s,
        vessel.display_name,
        cornerstone,
        &belief_item,
    );
    let now_unix = crate::game::now_unix();
    let life_step_tick = ((now_unix.max(0) as u64) / 60) + (seed % 900) + 120;
    let same_task_repetitions = ((seed % 180) + 24) as u32;
    let carryover_from_yesterday = format!(
        "昨日第 {} 回工序留下的器物手感與未竟尾款延續至今晨，晨起後順著昨日步調繼續忙活",
        same_task_repetitions
    );
    let initial_mg = compute_starting_wallet(seed, role_hint);
    let init_image = resolve_decision_image("", false, false, (12 + (seed % 28)) as u8, life_step_tick);
    let eligible = eligible_actions_for_image(init_image, initial_mg);
    let chosen = choose_action_from_eligible(&eligible, cornerstone, life_step_tick, seed);
    let margin = MarginBand::from_magnesium(initial_mg);
    let active_need = format!(
        "{}（懷揣 {} 枚鎂幣，{}）",
        init_image.label(),
        initial_mg,
        margin.label()
    );
    let image_decision = format!(
        "衡量{}的處境，依【{}】脾性決定先「{}」",
        margin.label(),
        cornerstone.label(),
        chosen.action_title
    );
    let situated_action = format!(
        "身在{}，{}",
        s,
        chosen.action_behavior
    );
    let state = NpcCausalState {
        cornerstone,
        stress: (12 + (seed % 28)) as u8,
        stress_break_active: false,
        hunger: ((seed % 35) + 25) as u8,
        thirst: (((seed / 3) % 40) + 15) as u8,
        fatigue: (((seed / 7) % 40) + 10) as u8,
        plan_steps,
        plan_cursor: ((seed / 3) % 3) as u8,
        belief_scarcity_item: belief_item.clone(),
        last_surprise_delta: None,
        hooks: seed_hooks_for_resident(role_hint, settlement, seed),
        life_step_tick,
        same_task_repetitions,
        carryover_from_yesterday,
        decision_image_code: init_image.code().to_string(),
        eligible_option_count: eligible.len() as u8,
        chosen_action_code: chosen.code.to_string(),
        active_need,
        image_decision,
        situated_action,
        belief_summary: format!("「{belief_item}」為當前聚落代表物資"),
        active_ripples: Vec::new(),
        immune_fact_ids: Vec::new(),
    };
    if let Ok(mut map) = LIVE_NPC_CAUSAL_STATES.write() {
        map.entry(entity_id.to_string())
            .or_insert_with(|| state.clone());
    }
    state
}

/// 住民抵達目標場所時，推進三步因果計畫鏈、信念落差（ΔE）調適與壓力閾值轉折，
/// 並將「需求產生意象決策，決策依現況產生行為」及不可逆之「時間刻度」推進至下一階段。
pub fn advance_causal_state_on_arrival(
    entity_id: &str,
    role_hint: &str,
    settlement: &str,
    seed: u64,
    arrived_dest_name: &str,
    consecutive_work_streaks: u8,
    has_affliction: bool,
    vessel_worn: bool,
    magnesium: i32,
    actual_scarcity_item: Option<&str>,
) -> NpcCausalState {
    let mut current = get_or_init_causal_state(entity_id, role_hint, settlement, seed);
    let vessel = resolve_vocational_vessel(role_hint);
    let s = if settlement.trim().is_empty() { "文曜" } else { settlement.trim() };

    // 0. 生命是一條直線：時間刻度嚴格單調遞增（永不取模歸零）
    current.life_step_tick = current.life_step_tick.saturating_add(1);

    // 1. 更新壓力（Stress 0..=100）：連軸工作、傷勢、荷包見底或器物磨損會推升壓力；洗沐、用膳、就醫則舒緩
    let is_rest_or_heal = arrived_dest_name.contains("浴")
        || arrived_dest_name.contains("湯")
        || arrived_dest_name.contains("食堂")
        || arrived_dest_name.contains("茶")
        || arrived_dest_name.contains("診")
        || arrived_dest_name.contains("居")
        || arrived_dest_name.contains("宅");

    if is_rest_or_heal {
        current.stress = current.stress.saturating_sub(24);
        current.stress_break_active = false;
        // 每個人每天起床不是歸零重整，生命是一條直線，記錄休歇與晨起未竟之延續
        current.carryover_from_yesterday = format!(
            "昨日第 {} 回工序留下的器物手感與未竟尾款延續至今晨，歇息調養後繼續推進手頭活計",
            current.same_task_repetitions
        );
    } else {
        let mut delta: u8 = 6 + consecutive_work_streaks.saturating_mul(5);
        if has_affliction {
            delta = delta.saturating_add(14);
        }
        if vessel_worn && current.cornerstone == Cornerstone::Craft {
            delta = delta.saturating_add(15);
        }
        if magnesium < 12 {
            delta = delta.saturating_add(10);
        }
        current.stress = current.stress.saturating_add(delta).min(100);

        // 同業日課與工序累計次數增加（即便每天做的都是同樣的事，時間刻度仍然是不同的）
        let is_work_or_forage = arrived_dest_name.contains("工坊")
            || arrived_dest_name.contains("獵")
            || arrived_dest_name.contains("礦")
            || arrived_dest_name.contains("林")
            || arrived_dest_name.contains("市集")
            || arrived_dest_name.contains("商")
            || arrived_dest_name.contains("書院")
            || arrived_dest_name.contains("講堂")
            || arrived_dest_name.contains("草甸")
            || arrived_dest_name.contains("道")
            || arrived_dest_name.contains("坑口")
            || consecutive_work_streaks > 0;
        if is_work_or_forage {
            current.same_task_repetitions = current.same_task_repetitions.saturating_add(1);
        }
        current.carryover_from_yesterday = format!(
            "昨日第 {} 回工序留下的器物手感與未竟尾款延續至今晨，晨起後順著昨日步調繼續忙活",
            current.same_task_repetitions
        );
    }

    // 2. 檢查信念落差（Belief Surprise ΔE）：若實際急缺物資與原先預期不同，或器物突發磨損，立即動態改寫三步計畫鏈
    if vessel_worn {
        current.last_surprise_delta = Some(format!(
            "察覺手中「{}」分寸微鈍，先改道{}修整器況再續原訂差事",
            vessel.display_name, vessel.maintenance_venue_keyword
        ));
        current.plan_steps = [
            format!("先赴{}修整「{}」", vessel.maintenance_venue_keyword, vessel.display_name),
            format!("重返{s}完成手頭耽擱的活計"),
            format!("收工後與【{}】核對今日人情帳", current.hooks.first().map(|h| h.target_name.as_str()).unwrap_or("街坊")),
        ];
        current.plan_cursor = 0;
    } else if let Some(actual_item) = actual_scarcity_item
        && !actual_item.is_empty()
        && actual_item != current.belief_scarcity_item
    {
        let old_item = current.belief_scarcity_item.clone();
        current.belief_scarcity_item = actual_item.to_string();
        current.belief_summary = format!("聽聞坊間「{actual_item}」緊缺看漲，當場改換趕製標的");
        current.last_surprise_delta = Some(format!(
            "抵埠始知{s}行情已由「{old_item}」轉缺「{actual_item}」，當場改換採辦與趕製標的"
        ));
        current.plan_steps = derive_default_3step_plan(
            role_hint,
            s,
            vessel.display_name,
            current.cornerstone,
            actual_item,
        );
        current.plan_cursor = 1;
    } else {
        current.plan_cursor = (current.plan_cursor + 1) % 3;
        if current.plan_cursor == 0 {
            current.last_surprise_delta = None;
            // 進入下一段直線刻度時，計畫鏈帶著最新累計回數向前推進，而非回到原點死循環
            let base_steps = derive_default_3step_plan(
                role_hint,
                s,
                vessel.display_name,
                current.cornerstone,
                &current.belief_scarcity_item,
            );
            current.plan_steps = [
                format!("承接第 {} 回手感，{}", current.same_task_repetitions, base_steps[0]),
                base_steps[1].clone(),
                base_steps[2].clone(),
            ];
        }
    }

    // 3. 壓力破防判定（Stress >= 72）
    if current.stress >= 72 {
        current.stress_break_active = true;
        let (break_title, break_step, _) =
            current.cornerstone.stress_break_directive(s, vessel.display_name);
        current.last_surprise_delta = Some(format!(
            "連番負荷觸發【{}】本色轉折：{}",
            current.cornerstone.label(),
            break_title
        ));
        current.plan_steps[current.plan_cursor as usize] = break_step;
    }

    // 4. 事先擬好「決策意象」與「行為動作」選項，等待條件（有錢沒錢？有多少錢？現況如何？）符合時做出選擇
    let image = resolve_decision_image(
        arrived_dest_name,
        has_affliction,
        vessel_worn,
        current.stress,
        current.life_step_tick,
    );
    let margin = MarginBand::from_magnesium(magnesium);
    let eligible = eligible_actions_for_image(image, magnesium);
    let chosen = choose_action_from_eligible(
        &eligible,
        current.cornerstone,
        current.life_step_tick,
        seed,
    );
    current.decision_image_code = image.code().to_string();
    current.eligible_option_count = eligible.len() as u8;
    current.chosen_action_code = chosen.code.to_string();

    current.active_need = format!(
        "{}（懷揣 {} 枚鎂幣，{}）",
        image.label(),
        magnesium.max(0),
        margin.label()
    );

    current.image_decision = format!(
        "衡量{}的處境，承接第 {} 回手頭活計，決定先「{}」",
        margin.label(),
        current.same_task_repetitions,
        chosen.action_title
    );

    let vessel_desc = if vessel_worn {
        format!("隨身「{}」微有磨損", vessel.display_name)
    } else {
        format!("隨身「{}」利索稱手", vessel.display_name)
    };
    current.situated_action = format!(
        "身處{}（{}），{}",
        arrived_dest_name,
        vessel_desc,
        chosen.action_behavior
    );
    if !is_rest_or_heal {
        let actor_short = crate::store::resolve_npc_person_name(entity_id, role_hint, None);
        let host_opt = if !actor_short.is_empty() && actor_short != "陌生人" {
            Some(actor_short.as_str())
        } else {
            None
        };
        if let Some((ev, sp)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
            arrived_dest_name,
            host_opt,
            crate::game::now_unix(),
        ) {
            current.situated_action = format!(
                "{}（當值事件：【{}】，與{}協力推進）",
                current.situated_action,
                ev.event_title,
                sp.partner_slot_actor,
            );
        }
    }
    if let Ok(mut map) = LIVE_NPC_CAUSAL_STATES.write() {
        map.insert(entity_id.to_string(), current.clone());
    }
    current
}

/// 按需十層體感微動作與內心念頭合成器（僅於 API 或觀測時即時合成，背景零浪費）
#[expect(
    clippy::too_many_arguments,
    reason = "10-layer somatic renderer explicitly takes the 10 live state dimensions on demand"
)]
pub fn render_somatic_10layer_thought(
    entity_id: &str,
    role_hint: &str,
    settlement: &str,
    seed: u64,
    activity: &str,
    held_vessel: &str,
    breath_habit: &str,
    afflictions: &[String],
    hunting_party: &str,
    magnesium: i32,
) -> String {
    let causal = get_or_init_causal_state(entity_id, role_hint, settlement, seed);
    let vessel_spec = resolve_vocational_vessel(role_hint);
    let vessel_short = if held_vessel.trim().is_empty() {
        vessel_spec.display_name
    } else {
        held_vessel.split('（').next().unwrap_or(vessel_spec.display_name)
    };

    // Layer 1 & 2: 若正處於性格錨點高壓破防，直接呈現鮮明的性格轉折體感
    if causal.stress_break_active {
        let (_, _, somatic_action) = causal
            .cornerstone
            .stress_break_directive(settlement, vessel_short);
        return format!(
            "{}；連日操勞下心頭頗感吃緊，承接第 {} 回累積的疲憊，心裡盤算先「{}」",
            somatic_action,
            causal.same_task_repetitions,
            causal.plan_steps[(causal.plan_cursor as usize) % 3]
        );
    }

    // Layer 3: 若正參與合社圍獵小隊，呈現小隊戰術協同與呼吸手勢
    if !hunting_party.is_empty() {
        let hook_peer = causal
            .hooks
            .first()
            .map(|h| h.target_name.as_str())
            .unwrap_or("同袍");
        return format!(
            "運轉「{}」壓住腳步聲，憑第 {} 回入山累積的老練手感扣緊「{}」，一面配合獵團合圍方位，一面留意同袍【{}】側翼動靜",
            breath_habit,
            causal.same_task_repetitions,
            vessel_short,
            hook_peer
        );
    }

    // Layer 4: 若身上帶有部位傷勢，體感微動作優先反映傷處代償與護具動作
    if let Some(first_aff) = afflictions.first() {
        return format!(
            "因「{}」隱隱作痛而微調「{}」吐納節奏，換手穩住「{}」，盤算著把第 {} 回手頭「{}」收尾便去敷藥歇息",
            first_aff,
            breath_habit,
            vessel_short,
            causal.same_task_repetitions,
            causal.plan_steps[(causal.plan_cursor as usize) % 3]
        );
    }

    // Layer 5: 若新發生信念驚奇（ΔE 預期落差），呈現隨機應變的眼神與手勢
    if let Some(surprise) = &causal.last_surprise_delta {
        return format!(
            "{}；遂一面運轉「{}」穩住心神，一面憑第 {} 回熟稔手感握緊「{}」，改按現況推進「{}」（下一步：{}）",
            surprise,
            breath_habit,
            causal.same_task_repetitions,
            vessel_short,
            causal.plan_steps[(causal.plan_cursor as usize) % 3],
            causal.plan_steps[((causal.plan_cursor as usize) + 1) % 3]
        );
    }

    // Layer 6..10: 結合器物微動作 + 呼吸節律 + 三步因果計畫鏈 + 人情鉤子 + 荷包成色 + 時間刻度
    let hand_kinematic = match (seed as usize + causal.plan_cursor as usize) % 5 {
        0 => format!("指節輕叩手中的「{vessel_short}」確核分寸"),
        1 => format!("順著「{breath_habit}」吐納節律，穩穩託住隨身「{vessel_short}」"),
        2 => format!("低頭拂去「{vessel_short}」上的浮塵，目光掃過四周動靜"),
        3 => format!("手腕微沉、將「{vessel_short}」收攏稱手位置，摸了摸荷包裡的 {magnesium} 枚鎂幣"),
        _ => format!("依【{}】習慣先將「{vessel_short}」理順，神色篤定", causal.cornerstone.label()),
    };

    let current_step = &causal.plan_steps[(causal.plan_cursor as usize) % 3];
    let next_step = &causal.plan_steps[((causal.plan_cursor as usize) + 1) % 3];
    let hook_note = causal
        .hooks
        .first()
        .map(|h| format!("心底同時記掛著「{}」", h.summary))
        .unwrap_or_default();

    if activity.contains("安歇") || activity.contains("熟睡") || activity.contains("就寢") {
        let sleep_pose = match (seed as usize) % 4 {
            0 => format!("將「{vessel_short}」妥善置於榻畔，綿長運轉「{breath_habit}」"),
            1 => format!("仔細收好「{vessel_short}」與荷包（{magnesium} 枚鎂幣），伴隨「{breath_habit}」安然歇下"),
            2 => format!("依【{}】習慣把「{vessel_short}」擺在伸手可及處，調勻「{breath_habit}」養神", causal.cornerstone.label()),
            _ => format!("吹熄燈火、撫平衣角，將「{vessel_short}」收攏枕側，在「{breath_habit}」節律中沉沉睡去"),
        };
        let continuity = format!(
            "日間第 {} 回工序的手感與帳目已收妥，明早醒來接著往下推進「{}」",
            causal.same_task_repetitions, next_step
        );
        if !hook_note.is_empty() && seed % 2 == 0 {
            format!("{sleep_pose}，{continuity}，{hook_note}")
        } else {
            format!("{sleep_pose}，{continuity}")
        }
    } else if !hook_note.is_empty() {
        format!(
            "{hand_kinematic}，承接昨日第 {} 回工序底數推進「{current_step}」（下一步：{next_step}），{hook_note}",
            causal.same_task_repetitions
        )
    } else {
        format!(
            "{hand_kinematic}，承接昨日第 {} 回工序底數專注於「{current_step}」，隨後準備「{next_step}」",
            causal.same_task_repetitions
        )
    }
}

/// 將一筆跨系統因果漣漪注入指定住民的因果大腦（SIR 傳播免疫、突顯度排序、性格錨點計畫鏈驅動）
pub fn inject_ripple_fact_into_npc(entity_id: &str, fact: WorldRippleFact, now_unix: i64) -> bool {
    let mut states = LIVE_NPC_CAUSAL_STATES.write().unwrap_or_else(|e| e.into_inner());
    let state = states.entry(entity_id.to_string()).or_insert_with(|| {
        let s = clean_settlement_name(&fact.subject_zone_or_settlement);
        NpcCausalState {
            cornerstone: Cornerstone::Pride,
            stress: 18,
            stress_break_active: false,
            hunger: 30,
            thirst: 25,
            fatigue: 20,
            plan_steps: ["先檢點隨身傢伙與行囊繩索".to_string(), "巡看周遭動靜".to_string(), "如常操持生計".to_string()],
            plan_cursor: 0,
            belief_scarcity_item: "常備物料".to_string(),
            last_surprise_delta: None,
            hooks: Vec::new(),
            life_step_tick: 100,
            same_task_repetitions: 10,
            carryover_from_yesterday: "昨日工序延續".to_string(),
            decision_image_code: "work_or_trade".to_string(),
            eligible_option_count: 3,
            chosen_action_code: "work_scavenge_wild_firewood_and_scraps".to_string(),
            active_need: "操持常業".to_string(),
            image_decision: "前往工坊".to_string(),
            situated_action: "操持常業".to_string(),
            belief_summary: format!("{s}周遭生計"),
            active_ripples: Vec::new(),
            immune_fact_ids: Vec::new(),
        }
    });

    state.immune_fact_ids.retain(|(_, exp)| *exp >= now_unix);
    if state.immune_fact_ids.iter().any(|(id, _)| id == &fact.fact_id) {
        return false;
    }
    if fact.hop_count > fact.max_hops {
        return false;
    }

    state.immune_fact_ids.push((fact.fact_id.clone(), now_unix + 5400));
    if state.immune_fact_ids.len() > 24 {
        state.immune_fact_ids.remove(0);
    }

    let eff_salience = |f: &WorldRippleFact| -> f32 { f.severity * (0.65_f32).powi(f.hop_count as i32) };
    state.active_ripples.retain(|f| {
        if f.fact_id == fact.fact_id {
            return false;
        }
        if fact.kind == RippleFactKind::ApexSlainOrRescued
            && f.kind == RippleFactKind::StrandedCargo
            && fact.order_id.is_some()
            && f.order_id == fact.order_id
        {
            return false;
        }
        true
    });
    state.active_ripples.push(fact.clone());
    state.active_ripples.sort_by(|a, b| {
        eff_salience(b)
            .partial_cmp(&eff_salience(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    state.active_ripples.truncate(4);

    state.belief_summary = fact.prose_summary.clone();
    state.last_surprise_delta = Some(fact.prose_summary.clone());

    match fact.kind {
        RippleFactKind::StrandedCargo | RippleFactKind::PredatorSpillover => {
            match state.cornerstone {
                Cornerstone::Honor | Cornerstone::Profit | Cornerstone::Pride => {
                    state.plan_steps = [
                        "先檢點隨身傢伙與行囊繩索".into(),
                        format!("趕往{}接應受阻貨單或助陣除獸", fact.subject_zone_or_settlement),
                        "護送物資直抵市集交割領賞".into(),
                    ];
                }
                Cornerstone::Order | Cornerstone::Care | Cornerstone::Craft => {
                    state.plan_steps = [
                        format!("聽聞{}風聲緊俏，先清點鋪內存貨", fact.subject_zone_or_settlement),
                        "避開險路改走穩妥熟道採辦".into(),
                        "安頓手頭器物與街坊所需".into(),
                    ];
                }
            }
            state.plan_cursor = 0;
        }
        RippleFactKind::MarketScarcity => {
            state.belief_scarcity_item = fact.item_or_actor.clone();
            state.plan_steps = [
                format!("得知{}急缺「{}」，先備齊手頭物料", fact.subject_zone_or_settlement, fact.item_or_actor),
                format!("趕製或調集「{}」運往市集", fact.item_or_actor),
                "趁行情看俏交割換取鎂幣".into(),
            ];
            state.plan_cursor = 0;
        }
        RippleFactKind::ApexSlainOrRescued => {
            state.plan_steps = [
                format!("欣聞{}險情解圍、貨途重新通暢", fact.subject_zone_or_settlement),
                "打點行囊重整商隊路引".into(),
                "如常往來交割各路物料".into(),
            ];
            state.plan_cursor = 0;
        }
        RippleFactKind::WeatherRouteHazard | RippleFactKind::ResourceFallow => {
            state.plan_steps = [
                format!("留心{}天候地勢與途程異動", fact.subject_zone_or_settlement),
                "放緩步調備妥防護用度".into(),
                "守住常業靜候路況平復".into(),
            ];
            state.plan_cursor = 0;
        }
        RippleFactKind::PlayerDeed => {
            state.plan_steps = [
                format!("得知義士在{}現身行事", fact.subject_zone_or_settlement),
                "留心坊間口碑與風聲流向".into(),
                "秉持本分相待往來豪客".into(),
            ];
            state.plan_cursor = 0;
        }
    }

    true
}

/// 兩位住民相遇時進行口耳相傳（SIR 傳播、衰減衰退、人情鉤子建立）
pub fn propagate_ripple_between_npcs(
    speaker_id: &str,
    speaker_name: &str,
    listener_id: &str,
    _listener_name: &str,
    now_unix: i64,
) -> Option<WorldRippleFact> {
    let eff_salience = |f: &WorldRippleFact| -> f32 { f.severity * (0.65_f32).powi(f.hop_count as i32) };
    let best_fact = {
        let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
        let speaker = states.get(speaker_id)?;
        let listener_opt = states.get(listener_id);
        speaker
            .active_ripples
            .iter()
            .filter(|f| {
                f.hop_count < f.max_hops
                    && (f.origin_tick + 5400 >= now_unix)
                    && listener_opt
                        .map(|l| !l.immune_fact_ids.iter().any(|(id, exp)| id == &f.fact_id && *exp >= now_unix))
                        .unwrap_or(true)
            })
            .max_by(|a, b| {
                eff_salience(a)
                    .partial_cmp(&eff_salience(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    };

    let fact = best_fact?;
    let mut next_fact = fact.clone();
    next_fact.hop_count += 1;
    next_fact.prose_summary = if fact.hop_count == 0 && !fact.prose_summary.contains(&speaker_name) {
        format!("聽{}提起，{}", speaker_name, fact.prose_summary)
    } else {
        fact.prose_summary.clone()
    };

    let injected = inject_ripple_fact_into_npc(listener_id, next_fact.clone(), now_unix);
    if injected {
        upsert_relational_hook(
            listener_id,
            speaker_id,
            speaker_name,
            "互通風聲",
            format!(
                "承蒙【{}】告知{}的實況風聲，得以提早因應",
                speaker_name, next_fact.subject_zone_or_settlement
            ),
            2,
            now_unix,
        );
        Some(next_fact)
    } else {
        None
    }
}

/// 查詢指定住民當前最突顯之因果漣漪
pub fn top_active_ripple_for_npc(entity_id: &str, now_unix: i64) -> Option<WorldRippleFact> {
    let eff_salience = |f: &WorldRippleFact| -> f32 { f.severity * (0.65_f32).powi(f.hop_count as i32) };
    let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
    let state = states.get(entity_id)?;
    state
        .active_ripples
        .iter()
        .filter(|f| f.origin_tick + 5400 >= now_unix)
        .max_by(|a, b| {
            eff_salience(a)
                .partial_cmp(&eff_salience(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}

/// 建立滯留物資貨單並自目的聚落公庫鎖定託管懸賞（SFC 嚴格守恆）
pub fn create_stranded_cargo_order(
    carrier_id: &str,
    carrier_name: &str,
    item_id: &str,
    item_name: &str,
    qty: i32,
    origin_settlement: &str,
    dest_settlement: &str,
    waystation_name: &str,
    waystation_coord: (i32, i32),
    hazard_prose: &str,
    now_unix: i64,
) -> Option<StrandedCargoOrder> {
    let clean_dest = clean_settlement_name(dest_settlement);
    let escrow_bounty_mg = {
        let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
        let bal = treasuries.entry(clean_dest.to_string()).or_insert(2400);
        if *bal >= 220 {
            *bal -= 8;
            8
        } else {
            0
        }
    };

    static ORDER_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let count = ORDER_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let order_id = format!("cargo_{}_{}_{}", carrier_id, now_unix, count);
    let final_hazard = if hazard_prose.trim().is_empty() {
        format!("山道遭遇異獸盤踞，山嵐濃霧阻道難行，行商【{carrier_name}】受困於{waystation_name}")
    } else {
        hazard_prose.trim().to_string()
    };

    let order = StrandedCargoOrder {
        order_id: order_id.clone(),
        carrier_id: carrier_id.to_string(),
        carrier_name: carrier_name.to_string(),
        item_id: item_id.to_string(),
        item_name: item_name.to_string(),
        qty,
        origin_settlement: origin_settlement.to_string(),
        dest_settlement: dest_settlement.to_string(),
        waystation_name: waystation_name.to_string(),
        waystation_coord,
        escrow_bounty_mg,
        hazard_prose: final_hazard,
        claimed_by: None,
        created_at_unix: now_unix,
        expires_at_unix: now_unix + 3600,
    };

    {
        let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
        let mut expired_refunds: Vec<(String, i32)> = Vec::new();
        for o in orders.values_mut() {
            if o.expires_at_unix < now_unix && o.escrow_bounty_mg > 0 {
                expired_refunds.push((o.dest_settlement.clone(), o.escrow_bounty_mg));
                o.escrow_bounty_mg = 0;
            }
        }
        if !expired_refunds.is_empty() {
            let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
            for (dest, refund) in expired_refunds {
                let s = clean_settlement_name(&dest);
                *treasuries.entry(s.to_string()).or_insert(2400) += refund;
            }
        }
        orders.retain(|_, o| o.expires_at_unix >= now_unix);
        if orders.len() < 32 {
            orders.insert(order_id.clone(), order.clone());
        } else {
            if escrow_bounty_mg > 0 {
                let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
                *treasuries.entry(clean_dest.to_string()).or_insert(2400) += escrow_bounty_mg;
            }
            return None;
        }
    }

    let fact = WorldRippleFact {
        fact_id: format!("rip_{}", order.order_id),
        kind: RippleFactKind::StrandedCargo,
        subject_zone_or_settlement: waystation_name.to_string(),
        item_or_actor: item_name.to_string(),
        order_id: Some(order.order_id.clone()),
        severity: 0.9,
        origin_tick: now_unix,
        hop_count: 0,
        max_hops: 3,
        prose_summary: format!(
            "行商【{}】運送「{}」行經{}受阻，貨單受滯急待接應",
            carrier_name, item_name, waystation_name
        ),
    };
    inject_ripple_fact_into_npc(carrier_id, fact, now_unix);

    Some(order)
}

/// 住民抵達或途經驛站／終點時，認領或交割受阻貨單並領取託管懸賞（SFC 守恆）
pub fn claim_or_deliver_stranded_cargo_for_npc(
    rescuer_id: &str,
    rescuer_name: &str,
    current_settlement_or_room: &str,
    current_coord: (i32, i32),
    now_unix: i64,
) -> Option<(StrandedCargoOrder, i32)> {
    let mut target_order_id = None;
    {
        let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
        for (id, o) in orders.iter() {
            if o.escrow_bounty_mg > 0 && o.carrier_id != rescuer_id && now_unix <= o.expires_at_unix {
                if o.claimed_by.as_deref().is_some_and(|c| c != rescuer_id) {
                    continue;
                }
                let near_waystation = (current_coord.0 - o.waystation_coord.0).abs()
                    + (current_coord.1 - o.waystation_coord.1).abs()
                    <= 6;
                let near_room = !current_settlement_or_room.is_empty()
                    && (current_settlement_or_room.contains(&o.waystation_name)
                        || current_settlement_or_room.contains(&o.dest_settlement)
                        || current_settlement_or_room.contains(&o.origin_settlement));
                if near_waystation || near_room {
                    target_order_id = Some(id.clone());
                    break;
                }
            }
        }
    }

    let order_id = target_order_id?;
    let mut order = {
        let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
        orders.remove(&order_id)?
    };

    let bounty = order.escrow_bounty_mg;
    order.escrow_bounty_mg = 0;

    upsert_relational_hook(
        &order.carrier_id,
        rescuer_id,
        rescuer_name,
        "山道解圍",
        format!(
            "承蒙義士【{}】在{}涉險解圍、順利起獲受阻貨物",
            rescuer_name, order.waystation_name
        ),
        4,
        now_unix,
    );

    let fact = WorldRippleFact {
        fact_id: format!("rescued_{}", order.order_id),
        kind: RippleFactKind::ApexSlainOrRescued,
        subject_zone_or_settlement: order.waystation_name.clone(),
        item_or_actor: order.item_name.clone(),
        order_id: Some(order.order_id.clone()),
        severity: 0.95,
        origin_tick: now_unix,
        hop_count: 0,
        max_hops: 3,
        prose_summary: format!(
            "滯留於{}的「{}」已蒙【{}】解圍護送，商道重新暢通",
            order.waystation_name, order.item_name, rescuer_name
        ),
    };
    inject_ripple_fact_into_npc(&order.carrier_id, fact.clone(), now_unix);
    inject_ripple_fact_into_npc(rescuer_id, fact, now_unix);

    Some((order, bounty))
}

/// 玩家承接指定滯留物資貨單（鎖定該貨單之領取人，避免被其他遊蕩者或住民誤領）
pub fn claim_stranded_cargo_for_player(
    player_id: &str,
    _player_name: &str,
    order_id_or_target: &str,
    settlement_or_room: &str,
    now_unix: i64,
) -> Option<StrandedCargoOrder> {
    let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
    let mut target_id = None;

    if !order_id_or_target.is_empty() {
        for (id, o) in orders.iter() {
            if o.escrow_bounty_mg > 0
                && now_unix <= o.expires_at_unix
                && o.claimed_by.is_none()
            {
                if o.order_id == order_id_or_target
                    || o.carrier_id == order_id_or_target
                    || o.carrier_name.contains(order_id_or_target)
                    || o.item_id == order_id_or_target
                    || o.item_name.contains(order_id_or_target)
                    || o.dest_settlement == order_id_or_target
                    || o.waystation_name.contains(order_id_or_target)
                {
                    target_id = Some(id.clone());
                    break;
                }
            }
        }
    }

    if target_id.is_none() && !settlement_or_room.is_empty() {
        for (id, o) in orders.iter() {
            if o.escrow_bounty_mg > 0
                && now_unix <= o.expires_at_unix
                && o.claimed_by.is_none()
            {
                if settlement_or_room.contains(&o.dest_settlement)
                    || settlement_or_room.contains(&o.waystation_name)
                    || settlement_or_room.contains(&o.origin_settlement)
                    || o.dest_settlement.contains(settlement_or_room)
                    || o.waystation_name.contains(settlement_or_room)
                {
                    target_id = Some(id.clone());
                    break;
                }
            }
        }
    }

    if target_id.is_none() {
        for (id, o) in orders.iter() {
            if o.escrow_bounty_mg > 0
                && now_unix <= o.expires_at_unix
                && o.claimed_by.is_none()
            {
                target_id = Some(id.clone());
                break;
            }
        }
    }

    let id = target_id?;
    if let Some(o) = orders.get_mut(&id) {
        o.claimed_by = Some(player_id.to_string());
        Some(o.clone())
    } else {
        None
    }
}

/// 玩家在特定場所交割滯留貨單並領取公庫託管懸賞（SFC 守恆）
pub fn deliver_stranded_cargo_for_player(
    player_id: &str,
    player_name: &str,
    carrier_or_target_id: &str,
    settlement_or_room: &str,
    now_unix: i64,
) -> Option<(StrandedCargoOrder, i32)> {
    let mut target_order_id = None;
    {
        let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
        for (id, o) in orders.iter() {
            if o.escrow_bounty_mg > 0
                && now_unix <= o.expires_at_unix
                && o.claimed_by.as_deref() == Some(player_id)
            {
                let matches_target = carrier_or_target_id.is_empty()
                    || o.carrier_id == carrier_or_target_id
                    || o.dest_settlement == carrier_or_target_id
                    || o.order_id == carrier_or_target_id;
                let matches_place = settlement_or_room.is_empty()
                    || settlement_or_room.contains(&o.dest_settlement)
                    || (!carrier_or_target_id.is_empty()
                        && (settlement_or_room.contains(&o.waystation_name)
                            || settlement_or_room.contains(&o.origin_settlement)));
                if matches_target && matches_place {
                    target_order_id = Some(id.clone());
                    break;
                }
            }
        }
        if target_order_id.is_none() && !carrier_or_target_id.is_empty() {
            for (id, o) in orders.iter() {
                if o.escrow_bounty_mg > 0
                    && now_unix <= o.expires_at_unix
                    && (o.claimed_by.is_none() || o.claimed_by.as_deref() == Some(player_id))
                {
                    let matches_target = carrier_or_target_id.is_empty()
                        || o.carrier_id == carrier_or_target_id
                        || o.dest_settlement == carrier_or_target_id
                        || o.order_id == carrier_or_target_id;
                    let matches_place = settlement_or_room.is_empty()
                        || settlement_or_room.contains(&o.dest_settlement)
                        || settlement_or_room.contains(&o.waystation_name)
                        || settlement_or_room.contains(&o.origin_settlement);
                    if matches_target && matches_place {
                        target_order_id = Some(id.clone());
                        break;
                    }
                }
            }
        }
    }

    let order_id = target_order_id?;
    let mut order = {
        let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
        orders.remove(&order_id)?
    };

    let bounty = order.escrow_bounty_mg;
    order.escrow_bounty_mg = 0;

    if let Some(arc) = crate::store::get_store() {
        credit_magnesium_bounty(&mut arc.write(), player_id, bounty);
    }
    crate::store::clear_active_npc_obligation(&order.carrier_id, "stranded_cargo");
    crate::game::object_ecology::supply_settlement_commodity_from_work(
        &order.dest_settlement,
        "arbitrage",
        player_name,
    );

    upsert_relational_hook(
        &order.carrier_id,
        player_id,
        player_name,
        "山道解圍",
        format!(
            "承蒙義士【{}】在{}涉險解圍、順利送達受阻貨物",
            player_name, order.waystation_name
        ),
        4,
        now_unix,
    );

    let fact = WorldRippleFact {
        fact_id: format!("rescued_{}", order.order_id),
        kind: RippleFactKind::ApexSlainOrRescued,
        subject_zone_or_settlement: order.dest_settlement.clone(),
        item_or_actor: order.item_name.clone(),
        order_id: Some(order.order_id.clone()),
        severity: 0.95,
        origin_tick: now_unix,
        hop_count: 0,
        max_hops: 3,
        prose_summary: format!(
            "滯留於{}的「{}」已蒙義士【{}】護送至{}交割，各方額手稱慶",
            order.waystation_name, order.item_name, player_name, order.dest_settlement
        ),
    };
    inject_ripple_fact_into_npc(&order.carrier_id, fact.clone(), now_unix);
    inject_ripple_fact_into_npc(player_id, fact, now_unix);

    Some((order, bounty))
}

/// 查詢指定行商或場所關聯之進行中滯留貨單
pub fn active_stranded_cargo_for_carrier_or_room(
    entity_id: &str,
    room_name: &str,
    now_unix: i64,
) -> Option<StrandedCargoOrder> {
    let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
    for o in orders.values() {
        if o.escrow_bounty_mg > 0 && now_unix <= o.expires_at_unix {
            if (!entity_id.is_empty() && o.carrier_id == entity_id)
                || (!room_name.is_empty()
                    && (room_name.contains(&o.waystation_name)
                        || room_name.contains(&o.dest_settlement)
                        || room_name.contains(&o.origin_settlement)))
            {
                return Some(o.clone());
            }
        }
    }
    None
}

/// 列出全島即時在線之未結案滯留物資貨單清單
pub fn list_active_stranded_cargo_orders(now_unix: i64) -> Vec<StrandedCargoOrder> {
    let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
    orders
        .values()
        .filter(|o| o.escrow_bounty_mg > 0 && now_unix <= o.expires_at_unix)
        .cloned()
        .collect()
}

/// 動態更新或建立住民人情／恩怨鉤子（Relational Hook）
pub fn upsert_relational_hook(
    entity_id: &str,
    target_id: &str,
    target_name: &str,
    kind_label: &str,
    summary: String,
    intensity_delta: i8,
    now_unix: i64,
) {
    let mut states = LIVE_NPC_CAUSAL_STATES.write().unwrap_or_else(|e| e.into_inner());
    let state = states.entry(entity_id.to_string()).or_insert_with(|| {
        NpcCausalState {
            cornerstone: Cornerstone::Honor,
            stress: 15,
            stress_break_active: false,
            hunger: 30,
            thirst: 25,
            fatigue: 20,
            plan_steps: ["打點隨身器物".to_string(), "巡看周遭動靜".to_string(), "如常操持生計".to_string()],
            plan_cursor: 0,
            belief_scarcity_item: "常備物料".to_string(),
            last_surprise_delta: None,
            hooks: Vec::new(),
            life_step_tick: 100,
            same_task_repetitions: 10,
            carryover_from_yesterday: "昨日工序延續".to_string(),
            decision_image_code: "work_or_trade".to_string(),
            eligible_option_count: 3,
            chosen_action_code: "work_scavenge_wild_firewood_and_scraps".to_string(),
            active_need: "操持常業".to_string(),
            image_decision: "前往工坊".to_string(),
            situated_action: "操持常業".to_string(),
            belief_summary: "常業平順".to_string(),
            active_ripples: Vec::new(),
            immune_fact_ids: Vec::new(),
        }
    });

    let mut found = false;
    for h in &mut state.hooks {
        let matches_id = !target_id.is_empty() && h.target_id == target_id;
        let matches_name = !target_name.is_empty() && h.target_name == target_name;
        if matches_id || matches_name {
            h.kind_label = kind_label.to_string();
            h.target_name = target_name.to_string();
            if !target_id.is_empty() {
                h.target_id = target_id.to_string();
            }
            h.summary = summary.clone();
            let new_int = (h.intensity as i16) + (intensity_delta as i16);
            h.intensity = new_int.clamp(1, 100) as u8;
            h.updated_at_unix = now_unix;
            found = true;
            break;
        }
    }

    if !found {
        let init_int = (5 + (intensity_delta as i16)).clamp(1, 100) as u8;
        state.hooks.push(RelationalHook {
            kind_label: kind_label.to_string(),
            target_name: target_name.to_string(),
            summary,
            intensity: init_int,
            target_id: target_id.to_string(),
            updated_at_unix: now_unix,
        });
        if state.hooks.len() > 8 {
            state.hooks.sort_by(|a, b| b.intensity.cmp(&a.intensity));
            state.hooks.truncate(8);
        }
    }
}

/// 查詢指定人物目前的關係鉤子列表
pub fn get_relational_hooks(entity_id: &str) -> Vec<RelationalHook> {
    let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
    states.get(entity_id).map(|s| s.hooks.clone()).unwrap_or_default()
}

/// 查詢指定住民對目標人物之既有關係鉤子
pub fn find_relational_hook_for_target(
    entity_id: &str,
    target_id: &str,
    target_name: &str,
) -> Option<RelationalHook> {
    let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
    let state = states.get(entity_id)?;
    state
        .hooks
        .iter()
        .find(|h| {
            (!target_id.is_empty() && h.target_id == target_id)
                || (!target_name.is_empty() && h.target_name == target_name)
        })
        .cloned()
}

/// 雙人相遇言出法隨計畫（承載雙向選擇、實物交割、關係鉤子與後續行動推進）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DyadicKotodamaPlan {
    pub plan_code: String,
    pub mg_delta_a: i32,
    pub clear_obligation_a: Option<String>,
    pub open_obligation_a: Option<(String, String)>,
    pub clear_obligation_b: Option<String>,
    pub open_obligation_b: Option<(String, String)>,
    pub hook_a: Option<(String, String, i8)>,
    pub hook_b: Option<(String, String, i8)>,
    pub next_plan_step_a: Option<String>,
    pub next_plan_step_b: Option<String>,
    pub surprise_a: Option<String>,
    pub surprise_b: Option<String>,
    pub keep_thread_open: bool,
    pub topic_override: String,
    pub prompt_directive: String,
}

/// 依雙方真實生計狀態、未完義務、多元性格與人際關係，判定言出法隨雙人實物事件
pub fn plan_dyadic_kotodama_encounter(
    a_id: &str,
    a_name: &str,
    a_role: &str,
    a_mg: i32,
    a_ob: Option<(String, String)>,
    b_id: &str,
    b_name: &str,
    b_role: &str,
    b_mg: i32,
    b_ob: Option<(String, String)>,
    room_name: &str,
    fam: i32,
    sen: i32,
    _now_unix: i64,
) -> Option<DyadicKotodamaPlan> {
    let a_seed = deterministic_resident_seed(a_id, None);
    let b_seed = deterministic_resident_seed(b_id, None);
    let s_name = clean_settlement_name(room_name);

    let (a_reps, a_hooks, a_stress) = {
        if let Ok(guard) = LIVE_NPC_CAUSAL_STATES.read() {
            if let Some(st) = guard.get(a_id) {
                (st.same_task_repetitions, st.hooks.clone(), st.stress)
            } else {
                (1, Vec::new(), 35)
            }
        } else {
            (1, Vec::new(), 35)
        }
    };
    let a_weights = compute_plural_personality_weights_with_experience(
        a_role,
        s_name,
        a_seed,
        a_stress,
        a_mg,
        a_reps,
        &a_hooks,
        Some(PsychSituationSlot::RelationalEncounter),
    );

    let (b_reps, b_hooks, b_stress) = {
        if let Ok(guard) = LIVE_NPC_CAUSAL_STATES.read() {
            if let Some(st) = guard.get(b_id) {
                (st.same_task_repetitions, st.hooks.clone(), st.stress)
            } else {
                (1, Vec::new(), 20)
            }
        } else {
            (1, Vec::new(), 20)
        }
    };
    let b_weights = compute_plural_personality_weights_with_experience(
        b_role,
        s_name,
        b_seed,
        b_stress,
        b_mg,
        b_reps,
        &b_hooks,
        Some(PsychSituationSlot::RelationalEncounter),
    );
    // (a) 雙方既有借貸債務關係 (Existing debt between A and B)
    let existing_hook_a = find_relational_hook_for_target(a_id, b_id, b_name);
    let has_debt_obligation = matches!(a_ob.as_ref(), Some((k, target)) if k == "repay_debt" && (target == b_id || target == b_name));
    let has_debt_hook = existing_hook_a.as_ref().is_some_and(|h| h.kind_label == "立契借貸" || h.kind_label == "展期欠債");

    if has_debt_obligation || has_debt_hook {
        if a_mg >= 4 {
            // 清償債務：A 支付 4 鎂幣歸還 B
            return Some(DyadicKotodamaPlan {
                plan_code: "settle_debt".to_string(),
                mg_delta_a: -4,
                clear_obligation_a: Some("repay_debt".to_string()),
                open_obligation_a: None,
                clear_obligation_b: None,
                open_obligation_b: None,
                hook_a: Some((
                    "守諾償清".to_string(),
                    format!("於{room_name}當面如數結清積欠{b_name}之四枚鎂幣"),
                    10,
                )),
                hook_b: Some((
                    "守諾償清".to_string(),
                    format!("於{room_name}如數收訖{a_name}償還之四枚鎂幣"),
                    10,
                )),
                next_plan_step_a: Some(format!("與{b_name}結清舊欠，如釋重負，重新打點生計")),
                next_plan_step_b: None,
                surprise_a: Some(format!("當面償清積欠{b_name}之欠款，心中一塊大石落地")),
                surprise_b: Some(format!("收訖{a_name}如期償還之四枚鎂幣，對其誠信頗感慰藉")),
                keep_thread_open: false,
                topic_override: "結清舊欠：當面結算歸還四枚鎂幣".to_string(),
                prompt_directive: format!(
                    "【已拍板言出法隨：當面結清欠款】{a_name}如數掏出四枚鎂幣償清對{b_name}的借款，雙方帳目兩清、如釋重負；請在對話中體現交割與守諾。"
                ),
            });
        } else {
            // 無力還款：依 B 之多元性格權重評估寬限或逼討
            let b_benevolence = b_weights.care + b_weights.honor + (a_weights.honor + a_weights.order) / 4 + fam / 2 + sen * 2;
            let b_strictness = b_weights.profit + b_weights.pride;

            let (hook_b_label, hook_b_summary, hook_b_delta, prompt_text) = if b_benevolence >= b_strictness {
                (
                    "寬限債期".to_string(),
                    format!("於{room_name}體諒{a_name}手頭拮据，寬限數日容其緩頰籌款"),
                    1,
                    format!("【已拍板言出法隨：舊欠展期寬限】{a_name}手頭匱乏難以償還舊欠，向{b_name}告急求告；{b_name}權衡之下體諒其難處，允諾展期再籌；請在對話中體現求告與寬限。"),
                )
            } else {
                (
                    "追討舊欠".to_string(),
                    format!("於{room_name}嚴詞督促{a_name}儘速設法清償舊欠"),
                    -2,
                    format!("【已拍板言出法隨：嚴詞追討舊欠】{a_name}囊空如洗無力還款，{b_name}言辭緊逼、責其不可背信，限其儘速籌措；請在對話中體現催逼與窘迫。"),
                )
            };

            return Some(DyadicKotodamaPlan {
                plan_code: "default_extend_debt".to_string(),
                mg_delta_a: 0,
                clear_obligation_a: None,
                open_obligation_a: None,
                clear_obligation_b: None,
                open_obligation_b: None,
                hook_a: Some((
                    "展期欠債".to_string(),
                    format!("於{room_name}向{b_name}告貸求寬限，舊欠展期再籌"),
                    if b_benevolence >= b_strictness { 1 } else { -4 },
                )),
                hook_b: Some((
                    hook_b_label,
                    hook_b_summary,
                    hook_b_delta,
                )),
                next_plan_step_a: Some(format!("設法籌措鎂幣償還{b_name}之欠款")),
                next_plan_step_b: None,
                surprise_a: Some(format!("因舊欠無力償還而受制於{b_name}，心急如焚籌款")),
                surprise_b: Some(format!("查覺{a_name}依舊手頭羞澀，記掛未還舊帳")),
                keep_thread_open: true,
                topic_override: "催討舊欠：就尚未償清之欠款交涉緩急".to_string(),
                prompt_directive: prompt_text,
            });
        }
    }

    // (b) 急迫匱乏 vs 餘裕 (Urgent scarcity vs surplus)
    if a_mg <= 4 && b_mg >= 12 && a_ob.is_none() {
        let benevolence = b_weights.care + b_weights.honor + (a_weights.honor + a_weights.craft) / 3 + fam + sen * 3;
        let self_interest = b_weights.profit + b_weights.pride;

        if benevolence >= self_interest {
            return Some(DyadicKotodamaPlan {
                plan_code: "lend_magnesium".to_string(),
                mg_delta_a: 4,
                clear_obligation_a: None,
                open_obligation_a: Some(("repay_debt".to_string(), b_id.to_string())),
                clear_obligation_b: None,
                open_obligation_b: None,
                hook_a: Some((
                    "立契借貸".to_string(),
                    format!("於{room_name}蒙{b_name}慷慨解囊借與四枚鎂幣應急"),
                    8,
                )),
                hook_b: Some((
                    "解囊借貸".to_string(),
                    format!("於{room_name}借與{a_name}四枚鎂幣周轉，立下還款口約"),
                    6,
                )),
                next_plan_step_a: Some(format!("持向{b_name}借得之四枚鎂幣採買急需並籌款歸還")),
                next_plan_step_b: None,
                surprise_a: Some(format!("在窘迫之際獲{b_name}借貸四枚鎂幣，解燃眉之急")),
                surprise_b: Some(format!("念及街坊情分借出四枚鎂幣予{a_name}，囑其度過難關後歸還")),
                keep_thread_open: true,
                topic_override: "借貸應急：借與四枚鎂幣周轉，立下還款口約".to_string(),
                prompt_directive: format!(
                    "【已拍板言出法隨：慷慨借貸】{a_name}生計窘迫見底，{b_name}深明其難，當面掏出四枚鎂幣借與周轉；{a_name}銘感在心立約日後必償；請在對話中體現解囊借貸與感激立契。"
                ),
            });
        } else {
            return Some(DyadicKotodamaPlan {
                plan_code: "refuse_loan".to_string(),
                mg_delta_a: 0,
                clear_obligation_a: None,
                open_obligation_a: None,
                clear_obligation_b: None,
                open_obligation_b: None,
                hook_a: Some((
                    "求借遭拒".to_string(),
                    format!("於{room_name}向{b_name}告貸無果，遭其拒絕"),
                    -3,
                )),
                hook_b: Some((
                    "婉拒借貸".to_string(),
                    format!("於{room_name}以手頭緊俏為由婉拒{a_name}告貸求助"),
                    -1,
                )),
                next_plan_step_a: Some("借貸無門，轉赴外圍自力採集變現".to_string()),
                next_plan_step_b: None,
                surprise_a: Some(format!("向{b_name}求借周轉遭拒，自尊受挫且生計更加緊迫")),
                surprise_b: Some(format!("婉拒了{a_name}借錢周轉的請求，各人自掃門前雪")),
                keep_thread_open: false,
                topic_override: "告貸遭拒：無奈求助卻遭閉門羹".to_string(),
                prompt_directive: format!(
                    "【已拍板言出法隨：婉拒借貸】{a_name}手頭窘迫試圖向{b_name}告借周轉，{b_name}算盤謹慎或各有難處婉言拒絕；請在對話中體現求借無門之尷尬與生計壓力。"
                ),
            });
        }
    }

    // (c) 差事義務轉包託付 (Task obligation subcontracting)
    if let Some((kind, subj)) = &a_ob {
        if (kind == "repair_tool" || kind == "stranded_cargo") && a_mg >= 4 && b_ob.is_none() {
            if b_weights.craft + b_weights.profit + b_weights.honor >= 110 && sen >= -10 {
                return Some(DyadicKotodamaPlan {
                    plan_code: "subcontract_obligation".to_string(),
                    mg_delta_a: -2,
                    clear_obligation_a: Some(kind.clone()),
                    open_obligation_a: None,
                    clear_obligation_b: None,
                    open_obligation_b: Some((kind.clone(), subj.clone())),
                    hook_a: Some((
                        "出資託付".to_string(),
                        format!("於{room_name}出資兩枚鎂幣託付{b_name}代辦{subj}"),
                        5,
                    )),
                    hook_b: Some((
                        "受託代辦".to_string(),
                        format!("於{room_name}收受{a_name}兩枚鎂幣定金，接下代辦{subj}之差事"),
                        6,
                    )),
                    next_plan_step_a: Some(format!("已付酬兩枚鎂幣託付{b_name}代辦{subj}，轉身操持主業")),
                    next_plan_step_b: Some(format!("前往代辦受託之差事：{subj}")),
                    surprise_a: Some(format!("成功將手頭待辦之{subj}託付予{b_name}，解了燃眉之急")),
                    surprise_b: Some(format!("受{a_name}之託承攬{subj}，賺得兩枚鎂幣酬勞")),
                    keep_thread_open: true,
                    topic_override: format!("差事託付：出資兩枚鎂幣託付代辦{subj}"),
                    prompt_directive: format!(
                        "【已拍板言出法隨：出資託付差事】{a_name}手頭分身乏術，出資兩枚鎂幣當面委託{b_name}代為奔走處理「{subj}」；{b_name}欣然接下差事與訂金；請在對話中落實此樁委託與交割。"
                    ),
                });
            }
        }
    }

    // (d) 場域事件包（EventPack）與劇本包（ScriptPack）雙人當值協作演繹
    if a_ob.is_none() && b_ob.is_none() {
        if let Some((ev_pack, sc_pack)) =
            crate::game::peninsula_scripts::active_dyadic_event_and_script_pack_for_room(
                room_name,
                Some(a_name),
                Some(b_name),
                _now_unix,
            )
        {
            let clean_key_objects: String = ev_pack
                .key_objects
                .split('、')
                .map(|part| {
                    let t = part.trim();
                    if let Some(rest) = t.strip_prefix("WT-") {
                        rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ').to_string()
                    } else {
                        t.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("、");
            return Some(DyadicKotodamaPlan {
                plan_code: "co_enact_script_pack".to_string(),
                mg_delta_a: 0,
                clear_obligation_a: None,
                open_obligation_a: None,
                clear_obligation_b: None,
                open_obligation_b: None,
                hook_a: Some((
                    "劇本協作".to_string(),
                    format!("於{room_name}與{b_name}協力演繹「{}」", ev_pack.event_title),
                    4,
                )),
                hook_b: Some((
                    "劇本協作".to_string(),
                    format!("於{room_name}同{a_name}協力完成「{}」", ev_pack.event_title),
                    4,
                )),
                next_plan_step_a: Some(format!(
                    "推進場域劇本「{}」：{}",
                    ev_pack.event_title, sc_pack.player_actions
                )),
                next_plan_step_b: Some(format!("配合{a_name}完成「{}」", ev_pack.event_title)),
                surprise_a: Some(format!(
                    "與{b_name}同場演繹事件包「{}」，{}",
                    ev_pack.event_title, sc_pack.outcome_and_world_change
                )),
                surprise_b: Some(format!("協助{a_name}演繹劇本包「{}」", ev_pack.event_title)),
                keep_thread_open: false,
                topic_override: format!("{a_name}與{b_name}協作{}：{}", ev_pack.event_title, sc_pack.player_actions),
                prompt_directive: format!(
                    "【已拍板言出法隨：雙人演繹事件包與劇本包「{}」】場域因果：{}；關鍵器物：{}；協作行動：{}；演繹成果：{}。請在對話中自然體現兩人就現場器物與差事當面配合交接。",
                    ev_pack.event_title,
                    ev_pack.trigger_and_causal_chain,
                    clean_key_objects,
                    sc_pack.player_actions,
                    sc_pack.outcome_and_world_change
                ),
            });
        }
    }

    None
}

/// 提交並落實雙人言出法隨計畫：鎂幣嚴格守恆轉移、義務狀態流轉、人情鉤子更新、NPC因果大腦推進
pub fn commit_dyadic_kotodama_plan(
    a_id: &str,
    a_name: &str,
    b_id: &str,
    b_name: &str,
    plan: &DyadicKotodamaPlan,
    now_unix: i64,
) {
    // 1. 貨幣守恆交割 (delta_a + delta_b == 0)
    if plan.mg_delta_a != 0 {
        let _ = crate::db::add_magnesium(a_id, plan.mg_delta_a);
        let _ = crate::db::add_magnesium(b_id, -plan.mg_delta_a);
    }

    // 2. 義務狀態流轉
    if let Some(kind) = &plan.clear_obligation_a {
        crate::store::clear_active_npc_obligation(a_id, kind);
    }
    if let Some((kind, subj)) = &plan.open_obligation_a {
        crate::store::open_active_npc_obligation(a_id, kind, subj);
    }
    if let Some(kind) = &plan.clear_obligation_b {
        crate::store::clear_active_npc_obligation(b_id, kind);
    }
    if let Some((kind, subj)) = &plan.open_obligation_b {
        crate::store::open_active_npc_obligation(b_id, kind, subj);
    }

    // 3. 人情／恩怨關係鉤子更新
    if let Some((kind_label, summary, delta)) = &plan.hook_a {
        upsert_relational_hook(a_id, b_id, b_name, kind_label, summary.clone(), *delta, now_unix);
    }
    if let Some((kind_label, summary, delta)) = &plan.hook_b {
        upsert_relational_hook(b_id, a_id, a_name, kind_label, summary.clone(), *delta, now_unix);
    }

    // 4. 更新因果大腦之計畫步驟與意外衝擊
    let mut states = LIVE_NPC_CAUSAL_STATES.write().unwrap_or_else(|e| e.into_inner());
    if plan.next_plan_step_a.is_some() || plan.surprise_a.is_some() {
        if let Some(st) = states.get_mut(a_id) {
            if let Some(step) = &plan.next_plan_step_a {
                st.plan_steps[0] = step.clone();
            }
            if let Some(sur) = &plan.surprise_a {
                st.last_surprise_delta = Some(sur.clone());
            }
        }
    }
    if plan.next_plan_step_b.is_some() || plan.surprise_b.is_some() {
        if let Some(st) = states.get_mut(b_id) {
            if let Some(step) = &plan.next_plan_step_b {
                st.plan_steps[0] = step.clone();
            }
            if let Some(sur) = &plan.surprise_b {
                st.last_surprise_delta = Some(sur.clone());
            }
        }
    }
    drop(states);

    // 5. 若為雙人同場演繹事件包與劇本包，同步注入世界漣漪事實
    if plan.plan_code == "co_enact_script_pack" {
        let fact = WorldRippleFact {
            fact_id: format!("script_pack_{}_{}_{}", a_id, b_id, now_unix),
            kind: RippleFactKind::ApexSlainOrRescued,
            subject_zone_or_settlement: clean_settlement_name(&plan.topic_override).to_string(),
            item_or_actor: format!("{a_name}與{b_name}"),
            order_id: None,
            severity: 0.75,
            origin_tick: now_unix,
            hop_count: 0,
            max_hops: 3,
            prose_summary: plan.topic_override.clone(),
        };
        inject_ripple_fact_into_npc(a_id, fact.clone(), now_unix);
        inject_ripple_fact_into_npc(b_id, fact, now_unix);
    }
}

/// 依據玩家之實際作為（義舉／解圍／交易／交鋒）即時改變住民因果狀態與人情鉤子
pub fn mutate_npc_from_player_action(
    npc_id: &str,
    player_id: &str,
    player_name: &str,
    settlement: &str,
    action_kind: &str,
    item_or_detail: &str,
    mg_delta: i32,
    now_unix: i64,
) {
    let s = clean_settlement_name(settlement);
    match action_kind {
        "trade" | "buy" | "sell" => {
            let lbl = if mg_delta.abs() >= 15 { "大宗交割" } else { "商貿往來" };
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                lbl,
                format!("與義士【{}】在{}交割「{}」，銀貨兩訖", player_name, s, item_or_detail),
                if mg_delta.abs() >= 15 { 4 } else { 2 },
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = st.stress.saturating_sub(4);
            }
            let fact = WorldRippleFact {
                fact_id: format!("deed_{}_{}", player_id, now_unix),
                kind: RippleFactKind::PlayerDeed,
                subject_zone_or_settlement: s.to_string(),
                item_or_actor: player_name.to_string(),
                order_id: None,
                severity: 0.65,
                origin_tick: now_unix,
                hop_count: 0,
                max_hops: 2,
                prose_summary: format!(
                    "義士【{}】在{}與商戶交割「{}」，市面行情隨之一振",
                    player_name, s, item_or_detail
                ),
            };
            inject_ripple_fact_into_npc(npc_id, fact, now_unix);
        }
        "rescue" | "help" | "rescue_cargo" => {
            let lbl = if action_kind == "rescue_cargo" { "山道解圍" } else { "救命之恩" };
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                lbl,
                format!("蒙義士【{}】在{}出手解危「{}」，此恩銘記在心", player_name, s, item_or_detail),
                6,
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = st.stress.saturating_sub(12);
            }
            let fact = WorldRippleFact {
                fact_id: format!("rescue_{}_{}", player_id, now_unix),
                kind: RippleFactKind::PlayerDeed,
                subject_zone_or_settlement: s.to_string(),
                item_or_actor: player_name.to_string(),
                order_id: None,
                severity: 0.85,
                origin_tick: now_unix,
                hop_count: 0,
                max_hops: 3,
                prose_summary: format!(
                    "義士【{}】在{}仗義馳援「{}」，街坊鄰里無不交口稱讚",
                    player_name, s, item_or_detail
                ),
            };
            inject_ripple_fact_into_npc(npc_id, fact, now_unix);
        }
        "return_sharp" => {
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                "守諾歸還",
                format!("義士【{}】守約歸還「{}」且保養得宜，令人感佩", player_name, item_or_detail),
                4,
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = st.stress.saturating_sub(6);
            }
        }
        "return_damaged" => {
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                "毀器芥蒂",
                format!("借予【{}】的「{}」歸還時有所損耗，心頭頗為疼惜", player_name, item_or_detail),
                4,
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = (st.stress + 8).min(100);
            }
        }
        "borrow_item" => {
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                "通融借器",
                format!("曾將隨身「{}」通融借予【{}】應急使用", item_or_detail, player_name),
                2,
                now_unix,
            );
        }
        "combat_subdue" => {
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                "切磋落敗",
                format!("在{}與【{}】交手切磋略遜一籌，心下暗自敬服", s, player_name),
                4,
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = (st.stress + 6).min(100);
            }
        }
        "combat_slay" | "harm" | "provoke" | "attack" => {
            let lbl = if action_kind == "combat_slay" { "結下樑子" } else { "交鋒結怨" };
            upsert_relational_hook(
                npc_id,
                player_id,
                player_name,
                lbl,
                format!("在{}遭【{}】以「{}」相逼，往後需多加提防", s, player_name, item_or_detail),
                8,
                now_unix,
            );
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                st.stress = (st.stress + 15).min(100);
            }
            let fact = WorldRippleFact {
                fact_id: format!("clash_{}_{}", player_id, now_unix),
                kind: RippleFactKind::PlayerDeed,
                subject_zone_or_settlement: s.to_string(),
                item_or_actor: player_name.to_string(),
                order_id: None,
                severity: 0.75,
                origin_tick: now_unix,
                hop_count: 0,
                max_hops: 2,
                prose_summary: format!(
                    "【{}】在{}與人爭執交鋒「{}」，市井間議論紛紛",
                    player_name, s, item_or_detail
                ),
            };
            inject_ripple_fact_into_npc(npc_id, fact, now_unix);
        }
        _ => {
            if !player_name.trim().is_empty()
                && !item_or_detail.trim().is_empty()
                && !item_or_detail.starts_with("grid:")
                && find_relational_hook_for_target(npc_id, player_id, player_name).is_none()
            {
                upsert_relational_hook(
                    npc_id,
                    player_id,
                    player_name,
                    "面會結識",
                    format!("曾於{}與【{}】打過照面", s, player_name),
                    1,
                    now_unix,
                );
            }
            if let Ok(mut states) = LIVE_NPC_CAUSAL_STATES.try_write()
                && let Some(st) = states.get_mut(npc_id)
            {
                if mg_delta > 0 {
                    st.stress = st.stress.saturating_sub(2);
                } else if mg_delta < 0 {
                    st.stress = (st.stress + 2).min(100);
                }
            }
        }
    }
}

/// 將滯留貨單之懸賞金（鎂幣）撥入解圍者帳戶，確保全島封閉經濟守恆
pub fn credit_magnesium_bounty(store: &mut crate::store::Store, entity_id: &str, bounty_mg: i32) {
    if bounty_mg <= 0 {
        return;
    }
    if store.entities.contains_key(entity_id) {
        let _ = store.update_entity(entity_id, |ent| {
            ent.magnesium = ent.magnesium.saturating_add(bounty_mg);
        });
    } else {
        let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
        *treasuries.entry("文曜".to_string()).or_insert(2400) += bounty_mg;
    }
}

/// 玩家離線自主行事準則（Offline Autonomy Directive）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfflineAutonomyDirective {
    InnRestAndHeal,         // code: "inn_rest", label: "客房歇息養傷"
    WorkshopCraftAndAssist, // code: "workshop_craft", label: "駐坊百工代工"
    FieldTendAndForage,     // code: "field_tend", label: "照料田壟與林緣巡採"
    RoadEscortAndPatrol,    // code: "road_escort", label: "古道隨行護運"
}

impl OfflineAutonomyDirective {
    pub fn from_code(code: &str) -> Self {
        match code {
            "workshop_craft" => Self::WorkshopCraftAndAssist,
            "field_tend" => Self::FieldTendAndForage,
            "road_escort" => Self::RoadEscortAndPatrol,
            _ => Self::InnRestAndHeal,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::InnRestAndHeal => "inn_rest",
            Self::WorkshopCraftAndAssist => "workshop_craft",
            Self::FieldTendAndForage => "field_tend",
            Self::RoadEscortAndPatrol => "road_escort",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::InnRestAndHeal => "客房歇息養傷",
            Self::WorkshopCraftAndAssist => "駐坊百工代工",
            Self::FieldTendAndForage => "照料田壟與林緣巡採",
            Self::RoadEscortAndPatrol => "古道隨行護運",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Self::InnRestAndHeal => "於客房靜心歇息、調勻氣血與養護體魄，舒緩舊傷疾痛",
            Self::WorkshopCraftAndAssist => "駐留工坊協助匠人推磨校準器物，按行規賺取津貼並熟習百工",
            Self::FieldTendAndForage => "巡行聚落田壟與近郊林緣，照看作物生長並採集應季野生草木",
            Self::RoadEscortAndPatrol => "隨同商隊或獵戶漫步古道，護衛行旅安寧並結識四方旅友",
        }
    }
}

pub static PLAYER_OFFLINE_DIRECTIVES: LazyLock<RwLock<HashMap<String, OfflineAutonomyDirective>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static PLAYER_OFFLINE_RECAPS: LazyLock<RwLock<HashMap<String, Vec<(String, String, String)>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static PLAYER_LAST_OFFLINE_STEP: LazyLock<RwLock<HashMap<String, i64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_player_offline_directive(player_id: &str) -> OfflineAutonomyDirective {
    if let Some(dir) = PLAYER_OFFLINE_DIRECTIVES
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(player_id)
        .copied()
    {
        return dir;
    }

    if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
        if let Some(e) = s.entities.get(player_id) {
            let combined = format!("{} {}", e.display_title, e.current_activity);
            let matches_any = |keywords: &[&str]| keywords.iter().any(|k| combined.contains(k));
            if matches_any(&["工", "匠", "鍛", "磨", "木", "窯", "織", "鐵"]) {
                return OfflineAutonomyDirective::WorkshopCraftAndAssist;
            }
            if matches_any(&["農", "田", "採", "茶", "林", "漁", "藥", "圃"]) {
                return OfflineAutonomyDirective::FieldTendAndForage;
            }
            if matches_any(&["商", "獵", "衛", "巡", "行", "渡", "哨"]) {
                return OfflineAutonomyDirective::RoadEscortAndPatrol;
            }
        }
    }

    OfflineAutonomyDirective::InnRestAndHeal
}

pub fn set_player_offline_directive(player_id: &str, directive: OfflineAutonomyDirective) {
    let mut map = PLAYER_OFFLINE_DIRECTIVES
        .write()
        .unwrap_or_else(|e| e.into_inner());
    map.insert(player_id.to_string(), directive);
}

pub fn step_offline_player_once(player_id: &str, now_unix: i64) -> Option<String> {
    let (settlement, player_name, place_id, is_exhausted, vit, magnesium, soul_seed, eq_slots, inv_json, ent_opt) =
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get(player_id) {
                let place = e.place_id.as_deref().unwrap_or("");
                let clean = clean_settlement_name(place);
                let s_name = if clean.is_empty() { "文曜" } else { clean };
                (
                    s_name.to_string(),
                    if e.display_title.is_empty() {
                        "行旅者".to_string()
                    } else {
                        e.display_title.clone()
                    },
                    e.place_id.clone(),
                    e.vit < 30,
                    e.vit,
                    e.magnesium,
                    e.soul_seed.unwrap_or(42) as u64,
                    e.equipment_slots.clone(),
                    e.inventory.clone(),
                    Some(e.clone()),
                )
            } else {
                (
                    "文曜".to_string(),
                    "行旅者".to_string(),
                    None,
                    false,
                    80,
                    20,
                    42,
                    String::new(),
                    String::new(),
                    None,
                )
            }
        } else {
            (
                "文曜".to_string(),
                "行旅者".to_string(),
                None,
                false,
                80,
                20,
                42,
                String::new(),
                String::new(),
                None,
            )
        };

    let mut dummy_ent = crate::store::Entity::default();
    dummy_ent.id = player_id.to_string();
    dummy_ent.display_title = player_name.clone();
    dummy_ent.equipment_slots = eq_slots;
    dummy_ent.inventory = inv_json;

    let (cx, cy) = if let Some(ent) = &ent_opt {
        (ent.grid_x.unwrap_or(0), ent.grid_y.unwrap_or(0))
    } else {
        (0, 0)
    };
    let cur_pos = if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
        s.entities
            .get(player_id)
            .and_then(|e| e.grid_x.zip(e.grid_y))
            .unwrap_or((cx, cy))
    } else {
        (cx, cy)
    };

    let factors = if let Some(ent) = &ent_opt {
        crate::pg::living_world::ResidentIntrinsicFactors::resolve(
            ent,
            cur_pos.0,
            cur_pos.1,
            &settlement,
            &ent.current_activity,
        )
    } else {
        crate::pg::living_world::ResidentIntrinsicFactors::resolve(
            &dummy_ent,
            0,
            0,
            &settlement,
            "",
        )
    };
    let (tool_id, tool_name, is_bladed, vstate, raw_stock_qty, finished_stock_qty) =
        crate::pg::living_world::inspect_store_entity_live_stock_and_tool(&dummy_ent);

    let (weather, tide) = crate::game::zone_weather_and_tide(&settlement, now_unix);
    let chosen_action = crate::pg::living_world::choose_resident_life_action(
        &player_name,
        &settlement,
        soul_seed,
        vit,
        magnesium,
        raw_stock_qty,
        finished_stock_qty,
        if is_bladed { vstate.wear_points } else { 0 },
        crate::store::get_active_npc_obligation(player_id).is_some(),
        crate::store::current_game_hour() as i32,
        weather,
        tide,
        vstate.use_count.max(0) as u32,
        &get_relational_hooks(player_id),
    );

    if is_bladed && vstate.wear_points >= 3 && chosen_action == crate::pg::living_world::ResidentLifeActionChoice::RepairTool {
        let _ = crate::pg::living_world::sharpen_vessel_at_grindstone(player_id, &tool_id, now_unix)
            .or_else(|_| Ok::<_, &str>(crate::pg::living_world::temper_vessel_at_forge(player_id, &tool_id, now_unix)));
        let act = format!("於{}工坊磨石前推磨修整{}，校準刃口鋒芒", settlement, tool_name);
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let _ = s.update_entity(player_id, |e| {
                e.current_activity = act.clone();
            });
            s.append_thought(
                player_id,
                crate::db::ThoughtSource::WorkDone,
                format!("於{}工坊磨石前推磨修整{}，校準刃口鋒芒，手感漸入佳境", settlement, tool_name),
                now_unix,
            );
        }
        let _ = crate::pg::living_world::record_breath_activity(
            player_id,
            crate::pg::living_world::BreathHabitKind::ForgeHeavy,
            2,
            now_unix,
        );
        let title = "駐坊磨器 校準鋒芒".to_string();
        let body = format!("於{}工坊活水磨石前推磨修整隨身{}，細緻校準刃口分寸，器物重歸鋒利。", settlement, tool_name);
        let category = "social".to_string();
        {
            let mut recaps = PLAYER_OFFLINE_RECAPS
                .write()
                .unwrap_or_else(|e| e.into_inner());
            let list = recaps.entry(player_id.to_string()).or_default();
            if let Some(existing) = list.iter_mut().find(|(_, t, _)| t == &title) {
                *existing = (category, title, body.clone());
            } else {
                list.push((category, title, body.clone()));
                if list.len() > 4 {
                    list.remove(0);
                }
            }
        }
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let next_wake = now_unix + 60;
            let _ = client.execute(
                "INSERT INTO resident_continuity (
                    entity_id, control_mode, life_status, current_intent,
                    intent_reason, destination_place_id, intent_started_at,
                    next_wake_at, updated_at, source_kind, source_ref
                ) VALUES (
                    $1, 'autonomous', 'alive', 'repair_tool',
                    $2, $3, $4,
                    $5, $4, 'offline_autonomy', 'step_offline_player_once'
                )
                ON CONFLICT (entity_id) DO UPDATE SET
                    control_mode = 'autonomous',
                    life_status = 'alive',
                    current_intent = EXCLUDED.current_intent,
                    intent_reason = EXCLUDED.intent_reason,
                    destination_place_id = EXCLUDED.destination_place_id,
                    blocked_by = NULL,
                    next_wake_at = EXCLUDED.next_wake_at,
                    source_kind = EXCLUDED.source_kind,
                    source_ref = EXCLUDED.source_ref,
                    updated_at = EXCLUDED.updated_at",
                &[
                    &player_id,
                    &format!("於{}工坊推磨修整{}", settlement, tool_name),
                    &place_id,
                    &now_unix,
                    &next_wake,
                ],
            );
        }
        return Some(body);
    }

    let mut directive = get_player_offline_directive(player_id);
    if is_exhausted {
        directive = OfflineAutonomyDirective::InnRestAndHeal;
    }

    let (title, body, (dest_x, dest_y, dest_bldg)) = match directive {
        OfflineAutonomyDirective::InnRestAndHeal => {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(player_id, |e| {
                    if is_exhausted {
                        e.vit = (e.vit + 10).min(100);
                        e.qi = (e.qi + 10).min(100);
                    } else {
                        e.vit = (e.vit + 8).min(100);
                        e.qi = (e.qi + 8).min(100);
                        e.dex = (e.dex + 4).min(100);
                    }
                    e.current_activity = "於客舍內閉門靜養、調勻氣血".to_string();
                });
            }
            crate::pg::living_world::clear_entity_afflictions(player_id, true, now_unix);
            let is_indoors = if let Some(arc) = crate::store::get_store() {
                let s = arc.read();
                s.entities
                    .get(player_id)
                    .and_then(|e| e.place_id.as_deref())
                    .is_some_and(|p| {
                        p.contains("居")
                            || p.contains("舍")
                            || p.contains("宅")
                            || p.contains("客")
                            || p.contains("旅")
                            || p.contains("宿")
                            || p.contains("房")
                    })
            } else {
                false
            };
            let (t, b) = if is_indoors || is_exhausted {
                (
                    "客舍安歇 調勻氣血".to_string(),
                    "於房舍內閉門靜養、調勻氣血，舊傷與勞頓隨之舒緩散去。".to_string(),
                )
            } else {
                (
                    "行旅小憩 調勻氣息".to_string(),
                    "行旅駐足期間靜心調勻氣息，舒緩奔波勞頓，身心漸歸安寧。".to_string(),
                )
            };
            (t, b, (factors.home_coord.0, factors.home_coord.1, factors.home_bldg.clone()))
        }
        OfflineAutonomyDirective::WorkshopCraftAndAssist => {
            let hooks = get_relational_hooks(player_id);
            let pw = PluralPersonalityWeights::default();
            let runtime_grid = crate::server::grid_manager::get_runtime_square_grid();

            let is_real_resident = ent_opt.is_some();
            if is_real_resident && raw_stock_qty <= 0 {
                let sup_opt = if let Some(arc) = crate::store::get_store() {
                    let s = arc.read();
                    crate::pg::living_world::resolve_need_decision_supplier(
                        &s,
                        player_id,
                        cur_pos,
                        &factors,
                        &pw,
                        &hooks,
                        runtime_grid.as_ref(),
                    )
                } else {
                    None
                };
                let (sup_id, sup_place, (sup_x, sup_y)) = sup_opt.unwrap_or_else(|| {
                    let coord = factors.supplier_coord.unwrap_or(factors.work_coord);
                    let place = if factors.supplier_place_label.is_empty() {
                        factors.work_bldg.clone()
                    } else {
                        factors.supplier_place_label.clone()
                    };
                    (format!("{}櫃檯", place), place, coord)
                });

                let on_site = (cur_pos.0 - sup_x).abs() + (cur_pos.1 - sup_y).abs() <= 1;
                if on_site {
                    let buy_qty = factors.restock_qty;
                    let cost_mg = factors.restock_cost_mg.min(magnesium).max(0);
                    if let Some(arc) = crate::store::get_store() {
                        let mut s = arc.write();
                        let _ = s.update_entity(player_id, |e| {
                            e.current_activity = format!(
                                "在{}向{}採買{}，補充工坊原料",
                                sup_place, sup_id, factors.raw_item_name
                            );
                            e.magnesium = e.magnesium.saturating_sub(cost_mg);
                            e.vit = (e.vit - 2).max(20);
                            let mut entries: Vec<crate::db::inventory::InventoryEntry> =
                                serde_json::from_str(&e.inventory).unwrap_or_default();
                            if let Some(entry) =
                                entries.iter_mut().find(|it| it.item_id == factors.raw_item_id)
                            {
                                entry.qty += buy_qty;
                            } else {
                                entries.push(crate::db::inventory::InventoryEntry {
                                    item_id: factors.raw_item_id.clone(),
                                    qty: buy_qty,
                                });
                            }
                            if let Ok(serialized) = serde_json::to_string(&entries) {
                                e.inventory = serialized;
                            }
                        });
                        if let Some(sup_ent) = s.entities.get_mut(&sup_id) {
                            sup_ent.magnesium = sup_ent.magnesium.saturating_add(cost_mg);
                            let mut entries: Vec<crate::db::inventory::InventoryEntry> =
                                serde_json::from_str(&sup_ent.inventory).unwrap_or_default();
                            if let Some(pos) =
                                entries.iter().position(|it| it.item_id == factors.raw_item_id)
                            {
                                if entries[pos].qty > buy_qty {
                                    entries[pos].qty -= buy_qty;
                                } else {
                                    entries.remove(pos);
                                }
                                if let Ok(serialized) = serde_json::to_string(&entries) {
                                    sup_ent.inventory = serialized;
                                }
                            }
                            crate::pg::entity::upsert_async(sup_ent.clone());
                        }
                    }
                    let _ =
                        settle_player_with_settlement_treasury(player_id, &settlement, -cost_mg);
                    let _ =
                        crate::db::add_to_inventory(player_id, &factors.raw_item_id, buy_qty);
                    (
                        "採購原料 備足工坊".to_string(),
                        format!(
                            "於{}向{}支付 {} 枚鎂幣採買 {} {}{}，備足工坊手頭用料。",
                            sup_place,
                            sup_id,
                            cost_mg,
                            buy_qty,
                            factors.raw_item_unit,
                            factors.raw_item_name
                        ),
                        (sup_x, sup_y, sup_place),
                    )
                } else {
                    if let Some(arc) = crate::store::get_store() {
                        let _ = arc.write().update_entity(player_id, |e| {
                            e.current_activity =
                                format!("前往{}採買{}", sup_place, factors.raw_item_name);
                            e.vit = (e.vit - 1).max(20);
                        });
                    }
                    (
                        "動身採買 補給原料".to_string(),
                        format!(
                            "因工坊{}存量告罄，動身前往{}向{}採買原料補足庫存。",
                            factors.raw_item_name, sup_place, sup_id
                        ),
                        (sup_x, sup_y, sup_place),
                    )
                }
            } else if is_real_resident && finished_stock_qty >= 2 {
                let primary_out_id =
                    factors.outputs.first().map(|(id, ..)| id.as_str()).unwrap_or("");
                let buy_opt = if let Some(arc) = crate::store::get_store() {
                    let s = arc.read();
                    crate::pg::living_world::resolve_need_decision_buyer(
                        &s,
                        player_id,
                        cur_pos,
                        primary_out_id,
                        &factors,
                        &pw,
                        &hooks,
                        runtime_grid.as_ref(),
                    )
                } else {
                    None
                };
                let (buyer_id, buyer_place, (buy_x, buy_y)) = buy_opt.unwrap_or_else(|| {
                    let coord = factors.deliver_coord;
                    let place = factors.deliver_bldg.clone();
                    (format!("{}櫃檯", place), place, coord)
                });

                let on_site = (cur_pos.0 - buy_x).abs() + (cur_pos.1 - buy_y).abs() <= 1;
                if on_site {
                    let payout = factors.delivery_payout_mg;
                    if let Some(arc) = crate::store::get_store() {
                        let mut s = arc.write();
                        let _ = s.update_entity(player_id, |e| {
                            e.current_activity =
                                format!("在{}向{}交割成品，結清貨款", buyer_place, buyer_id);
                            e.magnesium = e.magnesium.saturating_add(payout);
                            e.vit = (e.vit - 3).max(20);
                            let mut entries: Vec<crate::db::inventory::InventoryEntry> =
                                serde_json::from_str(&e.inventory).unwrap_or_default();
                            for (deliv_id, _, _, deliv_qty) in &factors.delivery_items {
                                if let Some(pos) =
                                    entries.iter().position(|it| &it.item_id == deliv_id)
                                {
                                    if entries[pos].qty > *deliv_qty {
                                        entries[pos].qty -= *deliv_qty;
                                    } else {
                                        entries.remove(pos);
                                    }
                                }
                            }
                            if let Ok(serialized) = serde_json::to_string(&entries) {
                                e.inventory = serialized;
                            }
                        });
                        if let Some(buyer_ent) = s.entities.get_mut(&buyer_id) {
                            buyer_ent.magnesium = buyer_ent.magnesium.saturating_sub(payout);
                            let mut entries: Vec<crate::db::inventory::InventoryEntry> =
                                serde_json::from_str(&buyer_ent.inventory).unwrap_or_default();
                            for (deliv_id, _, _, deliv_qty) in &factors.delivery_items {
                                if let Some(entry) =
                                    entries.iter_mut().find(|it| &it.item_id == deliv_id)
                                {
                                    entry.qty += *deliv_qty;
                                } else {
                                    entries.push(crate::db::inventory::InventoryEntry {
                                        item_id: deliv_id.clone(),
                                        qty: *deliv_qty,
                                    });
                                }
                            }
                            if let Ok(serialized) = serde_json::to_string(&entries) {
                                buyer_ent.inventory = serialized;
                            }
                            crate::pg::entity::upsert_async(buyer_ent.clone());
                        }
                    }
                    let _ =
                        settle_player_with_settlement_treasury(player_id, &settlement, payout);
                    for (deliv_id, _, _, deliv_qty) in &factors.delivery_items {
                        let _ = crate::db::add_to_inventory(player_id, deliv_id, -(*deliv_qty));
                    }
                    (
                        "完工交割 兌現貨款".to_string(),
                        format!(
                            "於{}將工坊製作的成品點交予{}，結算收得 {} 枚鎂幣，備料充足、工務順遂。",
                            buyer_place, buyer_id, payout
                        ),
                        (buy_x, buy_y, buyer_place),
                    )
                } else {
                    if let Some(arc) = crate::store::get_store() {
                        let _ = arc.write().update_entity(player_id, |e| {
                            e.current_activity =
                                format!("前往{}送交工坊成品", buyer_place);
                            e.vit = (e.vit - 1).max(20);
                        });
                    }
                    (
                        "整裝出發 送貨交割".to_string(),
                        format!(
                            "背負工坊製作之成品，動身前往{}向{}點交交割。",
                            buyer_place, buyer_id
                        ),
                        (buy_x, buy_y, buyer_place),
                    )
                }
            } else {
                let wage = 4;
                let earned = if let Some(arc) = crate::store::get_store() {
                    let mut s = arc.write();
                    pay_from_settlement_treasury(&mut s, player_id, &settlement, wage)
                } else {
                    0
                };
                if let Some(arc) = crate::store::get_store() {
                    let _ = arc.write().update_entity(player_id, |e| {
                        e.current_activity = "於工坊推磨校準器物，協助百工".to_string();
                        e.vit = (e.vit - 3).max(20);
                        e.qi = (e.qi - 2).max(20);
                        let mut entries: Vec<crate::db::inventory::InventoryEntry> =
                            serde_json::from_str(&e.inventory).unwrap_or_default();
                        if let Some(pos) = entries
                            .iter()
                            .position(|it| it.item_id == factors.raw_item_id && it.qty >= 1)
                        {
                            entries[pos].qty -= 1;
                            if entries[pos].qty <= 0 {
                                entries.remove(pos);
                            }
                            for (out_id, _, _, out_qty) in &factors.outputs {
                                if let Some(out_entry) =
                                    entries.iter_mut().find(|it| &it.item_id == out_id)
                                {
                                    out_entry.qty += *out_qty;
                                } else {
                                    entries.push(crate::db::inventory::InventoryEntry {
                                        item_id: out_id.clone(),
                                        qty: *out_qty,
                                    });
                                }
                            }
                            if let Ok(serialized) = serde_json::to_string(&entries) {
                                e.inventory = serialized;
                            }
                        }
                    });
                }
                crate::game::object_ecology::supply_settlement_commodity_from_work(
                    &settlement,
                    "craft",
                    &player_name,
                );
                (
                    "駐坊校器 習練百工".to_string(),
                    if earned > 0 {
                        "於工坊協助推磨校準器物，掌事按行規結給工錢並指點發力分寸，手藝與心性皆有精進。".to_string()
                    } else {
                        "於工坊細心觀摩師傅鍛打校準，熟悉各項器物分寸走向，心手相應，大有收穫。".to_string()
                    },
                    (factors.work_coord.0, factors.work_coord.1, factors.work_bldg.clone()),
                )
            }
        }
        OfflineAutonomyDirective::FieldTendAndForage => {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(player_id, |e| {
                    e.current_activity = "沿田壟巡查、採收應季草木".to_string();
                    e.vit = (e.vit - 3).max(20);
                    e.qi = (e.qi - 2).max(20);
                });
            }
            let forage_item = if (now_unix / 60) % 2 == 0 {
                "harvest_herb"
            } else {
                "mat_reed_bundle"
            };
            let cur_qty = crate::db::get_entity(player_id)
                .ok()
                .flatten()
                .map(|e| {
                    crate::db::get_inventory(&e.inventory, e.vit)
                        .items
                        .iter()
                        .find(|it| it.item_id == forage_item)
                        .map(|it| it.qty)
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            if cur_qty < 5 {
                let _ = crate::db::add_to_inventory(player_id, forage_item, 1);
            }
            crate::game::object_ecology::supply_settlement_commodity_from_work(
                &settlement,
                "forage",
                &player_name,
            );
            (
                "田壟巡查 採收草木".to_string(),
                "沿著聚落田壟與近郊林緣慢步巡看，隨手採得應季生鮮草木收入隨身行囊，泥土芬芳怡人。".to_string(),
                (factors.floodgate_coord.0, factors.floodgate_coord.1, factors.floodgate_bldg.clone()),
            )
        }
        OfflineAutonomyDirective::RoadEscortAndPatrol => {
            let (peer_id, peer_name, peer_coord) = if let Some(arc) = crate::store::get_store() {
                let s = arc.read();
                let mut candidates: Vec<(&crate::store::Entity, (i32, i32), i32)> = Vec::new();
                for e in s.entities.values() {
                    if e.kind != "npc" || e.id == player_id {
                        continue;
                    }
                    let coord = match (e.grid_x, e.grid_y) {
                        (Some(x), Some(y)) => (x, y),
                        _ => continue,
                    };
                    let dist = (coord.0 - cur_pos.0).abs() + (coord.1 - cur_pos.1).abs();
                    if dist > 30 {
                        continue;
                    }
                    let is_traveler = e.display_title.contains("商")
                        || e.display_title.contains("獵")
                        || e.display_title.contains("客")
                        || e.display_title.contains("行")
                        || e.display_title.contains("旅")
                        || e.display_title.contains("伴");
                    let (is_staff, is_counter) =
                        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder_entity(
                            e,
                            coord,
                            &factors.raw_item_id,
                        );
                    if is_traveler || is_staff || is_counter {
                        candidates.push((e, coord, dist));
                    }
                }
                candidates.sort_by(|(e1, c1, d1), (e2, c2, d2)| {
                    d1.cmp(d2).then_with(|| c1.cmp(c2)).then_with(|| e1.id.cmp(&e2.id))
                });
                if let Some((npc, coord, _)) = candidates.first() {
                    (npc.id.clone(), npc.display_title.clone(), Some(*coord))
                } else {
                    (
                        format!("escort_{}", settlement),
                        format!("{}往來行旅同伴", settlement),
                        None,
                    )
                }
            } else {
                (
                    format!("escort_{}", settlement),
                    format!("{}往來行旅同伴", settlement),
                    None,
                )
            };
            if let Some(arc) = crate::store::get_store() {
                let act = format!("與【{}】結伴巡行古道，護送行旅", peer_name);
                let _ = arc.write().update_entity(player_id, |e| {
                    e.current_activity = act;
                    e.vit = (e.vit - 3).max(20);
                    e.qi = (e.qi - 2).max(20);
                });
            }
            upsert_relational_hook(
                player_id,
                &peer_id,
                &peer_name,
                "古道同行",
                format!("與【{}】結伴於古道隨行照應，同迎山嵐晨昏", peer_name),
                3,
                now_unix,
            );
            upsert_relational_hook(
                &peer_id,
                player_id,
                &player_name,
                "古道同行",
                format!("承蒙【{}】沿途隨行護持，商道行旅倍感心安", player_name),
                3,
                now_unix,
            );
            let fact = WorldRippleFact {
                fact_id: format!("escort_{}_{}", player_id, now_unix / 300),
                kind: RippleFactKind::PlayerDeed,
                subject_zone_or_settlement: settlement,
                item_or_actor: player_name.clone(),
                order_id: None,
                severity: 0.6,
                origin_tick: now_unix,
                hop_count: 0,
                max_hops: 2,
                prose_summary: format!(
                    "【{}】與【{}】結伴巡行古道，沿途安寧無虞",
                    player_name, peer_name
                ),
            };
            inject_ripple_fact_into_npc(&peer_id, fact, now_unix);
            let (dest_x, dest_y, dest_bldg) = if let Some((px, py)) = peer_coord {
                (px, py, factors.deliver_bldg.clone())
            } else {
                (factors.deliver_coord.0, factors.deliver_coord.1, factors.deliver_bldg.clone())
            };
            (
                "古道相隨 護送行旅".to_string(),
                format!(
                    "與【{}】結伴沿古道漫步同行，彼此照應山路動靜，言談甚歡，互道珍重而別。",
                    peer_name
                ),
                (dest_x, dest_y, dest_bldg),
            )
        }
    };

    let cur_grid = if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
        s.entities.get(player_id).and_then(|e| match (e.grid_x, e.grid_y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        })
    } else {
        None
    };

    if let Some((cx, cy)) = cur_grid {
        let (target_x, target_y, dest_bldg) = (dest_x, dest_y, dest_bldg);
        if (cx - target_x).abs() + (cy - target_y).abs() > 1 {
            let (nx, ny) = crate::server::grid_manager::get_runtime_square_grid()
                .and_then(|g| crate::store::find_bfs_next_step(&g, (cx, cy), (target_x, target_y)))
                .unwrap_or((cx + (target_x - cx).signum(), cy + (target_y - cy).signum()));
            if let Some(arc) = crate::store::get_store() {
                let mut s = arc.write();
                let mut current_act = String::new();
                let _ = s.update_entity(player_id, |e| {
                    e.grid_x = Some(nx);
                    e.grid_y = Some(ny);
                    e.target_x = if (nx, ny) == (target_x, target_y) { None } else { Some(target_x) };
                    e.target_y = if (nx, ny) == (target_x, target_y) { None } else { Some(target_y) };
                    let place_id = crate::world::place::legacy_place_id(crate::grid::SquareCoord::new(nx, ny));
                    e.place_id = Some(place_id);
                    current_act = e.current_activity.clone();
                });
                let _ = s.move_entity(player_id, &format!("grid:{nx}:{ny}"));
                crate::server::grid_manager::CANONICAL_NPC_TARGETS.write().insert(
                    player_id.to_string(),
                    (target_x, target_y, dest_bldg, current_act),
                );
            }
        } else {
            crate::server::grid_manager::CANONICAL_NPC_TARGETS.write().remove(player_id);
        }
    }

    let habit_kind = match directive {
        OfflineAutonomyDirective::InnRestAndHeal => crate::pg::living_world::BreathHabitKind::ChamberClear,
        OfflineAutonomyDirective::WorkshopCraftAndAssist => crate::pg::living_world::BreathHabitKind::ForgeHeavy,
        OfflineAutonomyDirective::FieldTendAndForage => crate::pg::living_world::BreathHabitKind::ForestLong,
        OfflineAutonomyDirective::RoadEscortAndPatrol => crate::pg::living_world::BreathHabitKind::ForestLong,
    };
    let _ = crate::pg::living_world::record_breath_activity(player_id, habit_kind, 2, now_unix);

    if let Some(arc) = crate::store::get_store() {
        let mut s = arc.write();
        s.append_thought(player_id, crate::db::ThoughtSource::WorkDone, body.clone(), now_unix);
    }

    if directive != OfflineAutonomyDirective::InnRestAndHeal {
        let mut tool_ids = Vec::new();
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get(player_id) {
                if let Ok(slots) = serde_json::from_str::<std::collections::HashMap<String, String>>(&e.equipment_slots) {
                    for slot in &["hold_r", "hold_l"] {
                        if let Some(tid) = slots.get(*slot).filter(|id| !id.trim().is_empty()) {
                            tool_ids.push(tid.clone());
                        }
                    }
                }
            }
        }
        if tool_ids.is_empty() {
            if let Ok(Some(ent)) = crate::db::get_entity(player_id) {
                if let Ok(slots) = serde_json::from_str::<std::collections::HashMap<String, String>>(&ent.equipment_slots) {
                    for slot in &["hold_r", "hold_l"] {
                        if let Some(tid) = slots.get(*slot).filter(|id| !id.trim().is_empty()) {
                            tool_ids.push(tid.clone());
                        }
                    }
                }
            }
        }
        for tool_id in tool_ids {
            let is_bladed_tool = !tool_id.contains("0973") && !tool_id.contains("abacus") && !tool_id.contains("basket");
            let vstate = crate::pg::living_world::record_vessel_use(
                player_id,
                &tool_id,
                if is_bladed_tool { 1 } else { 0 },
                1,
                now_unix,
            );
            if is_bladed_tool && vstate.wear_points >= 3 {
                let _ = crate::pg::living_world::sharpen_vessel_at_grindstone(player_id, &tool_id, now_unix);
            }
        }
    }

    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let next_wake = now_unix + 60;
        let _ = client.execute(
            "INSERT INTO resident_continuity (
                entity_id, control_mode, life_status, current_intent,
                intent_reason, destination_place_id, intent_started_at,
                next_wake_at, updated_at, source_kind, source_ref
            ) VALUES (
                $1, 'autonomous', 'alive', $2,
                $3, $4, $5,
                $6, $5, 'offline_autonomy', 'step_offline_player_once'
            )
            ON CONFLICT (entity_id) DO UPDATE SET
                control_mode = 'autonomous',
                life_status = 'alive',
                current_intent = EXCLUDED.current_intent,
                intent_reason = EXCLUDED.intent_reason,
                destination_place_id = EXCLUDED.destination_place_id,
                blocked_by = NULL,
                next_wake_at = EXCLUDED.next_wake_at,
                source_kind = EXCLUDED.source_kind,
                source_ref = EXCLUDED.source_ref,
                updated_at = EXCLUDED.updated_at",
            &[
                &player_id,
                &directive.code(),
                &directive.label(),
                &place_id,
                &now_unix,
                &next_wake,
            ],
        );
    }

    let category = "social".to_string();
    {
        let mut recaps = PLAYER_OFFLINE_RECAPS
            .write()
            .unwrap_or_else(|e| e.into_inner());
        let list = recaps.entry(player_id.to_string()).or_default();
        if let Some(existing) = list.iter_mut().find(|(_, t, _)| t == &title) {
            *existing = (category, title, body.clone());
        } else {
            list.push((category, title, body.clone()));
            if list.len() > 4 {
                list.remove(0);
            }
        }
    }

    Some(body)
}

pub fn step_offline_players_autonomy(now_unix: i64) -> usize {
    let online_ids = crate::server::session::list_online_player_ids();
    let offline_player_ids: Vec<String> = if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
        s.entities
            .values()
            .filter(|e| (e.kind == "player" || e.kind == "Player") && !online_ids.contains(&e.id))
            .map(|e| e.id.clone())
            .collect()
    } else {
        let directives = PLAYER_OFFLINE_DIRECTIVES
            .read()
            .unwrap_or_else(|e| e.into_inner());
        directives
            .keys()
            .filter(|id| !online_ids.contains(*id))
            .cloned()
            .collect()
    };

    let mut stepped_count = 0;
    for pid in offline_player_ids {
        let should_step = {
            let mut last_steps = PLAYER_LAST_OFFLINE_STEP
                .write()
                .unwrap_or_else(|e| e.into_inner());
            let last = last_steps.get(&pid).copied().unwrap_or(0);
            if now_unix.saturating_sub(last) >= 60 {
                last_steps.insert(pid.clone(), now_unix);
                true
            } else {
                false
            }
        };

        if should_step {
            if step_offline_player_once(&pid, now_unix).is_some() {
                stepped_count += 1;
            }
        }
    }

    stepped_count
}

pub fn drain_player_offline_recap(
    player_id: &str,
    _now_unix: i64,
) -> Vec<(String, String, String)> {
    let mut recaps = PLAYER_OFFLINE_RECAPS
        .write()
        .unwrap_or_else(|e| e.into_inner());
    let mut list = recaps.remove(player_id).unwrap_or_default();
    if list.len() > 4 {
        list.truncate(4);
    }
    list
}

/// 田壟與藥圃生長階段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CropGrowthStage {
    Seedling, // "幼苗青翠"
    Growing,  // "抽穗飽滿"
    Mature,   // "熟成待收"
}

/// 動態田壟作物即時狀態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropPlotState {
    pub coord: (i32, i32),
    pub field_kind: String, // "water_field" | "dry_field" | "herb_garden"
    pub crop_item_id: String,
    pub crop_name: String,
    pub sown_at_unix: i64,
    pub stage: CropGrowthStage,
    pub water_nourished: bool,
}

pub static LIVE_CROP_PLOTS: LazyLock<RwLock<HashMap<(i32, i32), CropPlotState>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn register_crop_plot_sown(
    x: i32,
    y: i32,
    field_kind: &str,
    now_unix: i64,
) -> CropPlotState {
    let (crop_item_id, crop_name) = match field_kind {
        "water_field" => ("mat_grain_sack", "大圳頭飽滿新米"),
        "dry_field" => ("mat_pasture_seed_sack", "旱壟耐旱雜糧穗"),
        "herb_garden" => ("harvest_herb", "分畦清香靈藥草"),
        _ => ("mat_grain_sack", "大圳頭飽滿新米"),
    };
    let state = CropPlotState {
        coord: (x, y),
        field_kind: field_kind.to_string(),
        crop_item_id: crop_item_id.to_string(),
        crop_name: crop_name.to_string(),
        sown_at_unix: now_unix,
        stage: CropGrowthStage::Seedling,
        water_nourished: field_kind == "water_field",
    };
    let mut plots = LIVE_CROP_PLOTS.write().unwrap_or_else(|e| e.into_inner());
    plots.insert((x, y), state.clone());
    state
}

pub fn inspect_or_advance_crop_plot(
    x: i32,
    y: i32,
    field_kind: &str,
    now_unix: i64,
) -> CropPlotState {
    let mut plots = LIVE_CROP_PLOTS.write().unwrap_or_else(|e| e.into_inner());
    let entry = plots.entry((x, y)).or_insert_with(|| {
        let (crop_item_id, crop_name) = match field_kind {
            "water_field" => ("mat_grain_sack", "大圳頭飽滿新米"),
            "dry_field" => ("mat_pasture_seed_sack", "旱壟耐旱雜糧穗"),
            "herb_garden" => ("harvest_herb", "分畦清香靈藥草"),
            _ => ("mat_grain_sack", "大圳頭飽滿新米"),
        };
        CropPlotState {
            coord: (x, y),
            field_kind: field_kind.to_string(),
            crop_item_id: crop_item_id.to_string(),
            crop_name: crop_name.to_string(),
            sown_at_unix: now_unix.saturating_sub(120),
            stage: CropGrowthStage::Mature,
            water_nourished: true,
        }
    });

    if entry.stage != CropGrowthStage::Mature {
        let elapsed = now_unix.saturating_sub(entry.sown_at_unix);
        let speed_factor = if entry.water_nourished { 2 } else { 1 };
        let effective_elapsed = elapsed.saturating_mul(speed_factor);
        if effective_elapsed >= 90 {
            entry.stage = CropGrowthStage::Mature;
        } else if effective_elapsed >= 45 {
            entry.stage = CropGrowthStage::Growing;
        } else {
            entry.stage = CropGrowthStage::Seedling;
        }
    }

    entry.clone()
}

pub fn harvest_crop_plot_once(
    x: i32,
    y: i32,
    field_kind: &str,
    now_unix: i64,
) -> Result<(String, String, i32, String), String> {
    let current = inspect_or_advance_crop_plot(x, y, field_kind, now_unix);
    if current.stage == CropGrowthStage::Mature {
        let mut plots = LIVE_CROP_PLOTS.write().unwrap_or_else(|e| e.into_inner());
        plots.remove(&(x, y));
        let qty = 2;
        let prose = format!(
            "你俯身細心收割已然熟成的「{}」，沉甸甸的碩果整齊捆紮收入行囊，田地重歸齊整待耕。",
            current.crop_name
        );
        Ok((current.crop_item_id, current.crop_name, qty, prose))
    } else {
        let not_ready_prose = match current.stage {
            CropGrowthStage::Seedling => format!(
                "田壟間的「{}」正展露青翠幼苗，根系尚嫩，需待日光照耀與甘霖潤澤才能抽穗成熟。",
                current.crop_name
            ),
            CropGrowthStage::Growing => format!(
                "田壟間的「{}」正處抽穗拔節之時，籽粒漸充，尚待時日始至全熟收成。",
                current.crop_name
            ),
            CropGrowthStage::Mature => unreachable!(),
        };
        Err(not_ready_prose)
    }
}

/// 全島玩家票號存摺即時帳本
pub static LIVE_PLAYER_BANK_LEDGER: LazyLock<RwLock<HashMap<String, i32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 全島玩家家宅玉石櫃存放物產清單（每位玩家至多 24 件）
pub static LIVE_PLAYER_HOME_STASH: LazyLock<RwLock<HashMap<String, Vec<(String, String)>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 全島親善手札竹筒信箱領取時間窗口記錄（每 300 秒可領取一回親善信與隨函薄禮）
pub static LIVE_ACQUAINTANCE_MAILBOX: LazyLock<RwLock<HashMap<String, i64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 查詢指定獵場分區在給定時間刻度下之物候天色
pub fn zone_weather_phase(z_idx: usize, now_unix: i64) -> crate::game::ZoneWeatherPhase {
    crate::game::current_zone_weather_and_tide(z_idx, now_unix / 180).0
}

/// 查詢指定獵場分區在給定時間刻度下之海潮走勢
pub fn zone_tide_phase(z_idx: usize, now_unix: i64) -> crate::game::ZoneTidePhase {
    crate::game::current_zone_weather_and_tide(z_idx, now_unix / 180).1
}

/// 依聚落或場所名稱解析其所屬或鄰近之正典獵場分區編號
pub fn settlement_to_zone_idx(settlement_or_room: &str) -> usize {
    crate::game::resolve_zone_idx_from_name_or_settlement(settlement_or_room)
}

/// 檢查指定分區是否有飢餓猛獸受擾越界下山
pub fn is_zone_predator_spillover_active(z_idx: usize, _now_unix: i64) -> bool {
    crate::game::is_zone_predator_spillover_active(z_idx)
}

/// 查詢玩家在全島票號的存摺餘額（鎂幣）
pub fn get_player_bank_balance(player_id: &str) -> i32 {
    let ledger = LIVE_PLAYER_BANK_LEDGER.read().unwrap_or_else(|e| e.into_inner());
    ledger.get(player_id).copied().unwrap_or(0)
}

/// 存入鎂幣至玩家票號存摺
///
/// 傳回 `(實際存入額, 身上剩餘額, 票號新餘額)`
pub fn deposit_player_bank_magnesium(player_id: &str, requested_mg: i32) -> (i32, i32, i32) {
    let Some(arc) = crate::store::get_store() else {
        return (0, 0, get_player_bank_balance(player_id));
    };
    let mut store = arc.write();
    let Some(ent) = store.entities.get_mut(player_id) else {
        return (0, 0, get_player_bank_balance(player_id));
    };
    if !(ent.kind == "player" || ent.kind == "Player" || ent.kind == "npc") {
        return (0, 0, get_player_bank_balance(player_id));
    }
    let actual = requested_mg.max(0).min(ent.magnesium.max(0));
    if actual > 0 {
        ent.magnesium = ent.magnesium.saturating_sub(actual);
        let mut ledger = LIVE_PLAYER_BANK_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        let balance = ledger.entry(player_id.to_string()).or_insert(0);
        *balance = balance.saturating_add(actual);
        (actual, ent.magnesium, *balance)
    } else {
        let cur_bank = {
            let ledger = LIVE_PLAYER_BANK_LEDGER.read().unwrap_or_else(|e| e.into_inner());
            ledger.get(player_id).copied().unwrap_or(0)
        };
        (0, ent.magnesium, cur_bank)
    }
}

/// 從玩家票號存摺支取鎂幣
///
/// 傳回 `(實際支取額, 身上新餘額, 票號剩餘額)`
pub fn withdraw_player_bank_magnesium(player_id: &str, requested_mg: i32) -> (i32, i32, i32) {
    let Some(arc) = crate::store::get_store() else {
        return (0, 0, get_player_bank_balance(player_id));
    };
    let mut store = arc.write();
    let Some(ent) = store.entities.get_mut(player_id) else {
        return (0, 0, get_player_bank_balance(player_id));
    };
    if !(ent.kind == "player" || ent.kind == "Player" || ent.kind == "npc") {
        return (0, 0, get_player_bank_balance(player_id));
    }
    let mut ledger = LIVE_PLAYER_BANK_LEDGER.write().unwrap_or_else(|e| e.into_inner());
    let balance = ledger.entry(player_id.to_string()).or_insert(0);
    let actual = requested_mg.max(0).min((*balance).max(0));
    if actual > 0 {
        *balance = balance.saturating_sub(actual);
        ent.magnesium = ent.magnesium.saturating_add(actual);
    }
    (actual, ent.magnesium, *balance)
}

/// 列出玩家家宅玉石櫃中當前存放的物產
pub fn list_player_home_stash(player_id: &str) -> Vec<(String, String)> {
    let stash = LIVE_PLAYER_HOME_STASH.read().unwrap_or_else(|e| e.into_inner());
    stash.get(player_id).cloned().unwrap_or_default()
}

/// 存放物產至玩家家宅玉石櫃（每位玩家上限 24 件）
pub fn store_item_in_home_stash(
    player_id: &str,
    item_id: &str,
    item_name: &str,
) -> Result<usize, String> {
    let mut stash = LIVE_PLAYER_HOME_STASH.write().unwrap_or_else(|e| e.into_inner());
    let list = stash.entry(player_id.to_string()).or_default();
    if list.len() >= 24 {
        return Err("家宅玉石櫃存放物產已達二十四件上限，無法再行存入。".to_string());
    }
    list.push((item_id.to_string(), item_name.to_string()));
    Ok(list.len())
}

/// 從玩家家宅玉石櫃取出最後存入的一件物產
pub fn retrieve_item_from_home_stash(player_id: &str) -> Option<(String, String, usize)> {
    let mut stash = LIVE_PLAYER_HOME_STASH.write().unwrap_or_else(|e| e.into_inner());
    let list = stash.get_mut(player_id)?;
    let (item_id, item_name) = list.pop()?;
    Some((item_id, item_name, list.len()))
}

/// 動態彙編全島即時《半島旬報》（四段式正典風貌，零生硬術語）
pub fn compile_live_peninsula_gazette(room_name: &str, now_unix: i64) -> String {
    // 一、【四鎮商貿與緊缺行情】
    const GAZETTE_SETTLEMENTS: &[&str] = &[
        "文曜", "風舶角", "半嶺鎮", "大圳頭", "南哨", "水尾寮", "老堰頭",
    ];
    let mut scarcity_list = Vec::new();
    for &s in GAZETTE_SETTLEMENTS {
        if let Some((item_id, item_name, cur_stock, target_stock, price_mg)) =
            crate::game::object_ecology::scarcest_item_for_settlement(s)
        {
            let gap = (target_stock - cur_stock).max(0);
            let ratio = gap as f32 / target_stock.max(1) as f32;
            scarcity_list.push((ratio, s, item_id, item_name, cur_stock, target_stock, price_mg));
        }
    }
    scarcity_list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let positive_scarcity: Vec<_> = scarcity_list.into_iter().filter(|x| x.0 > 0.001).collect();
    let s1_body = if positive_scarcity.is_empty() {
        "全島各聚落物料流轉平穩充盈，各大市集暫無重大緊缺物資，工坊日課常態運轉。".to_string()
    } else {
        let mut lines = Vec::new();
        for (ratio, s, _id, name, cur, tgt, pr) in positive_scarcity.into_iter().take(3) {
            let pct = (ratio * 100.0).round() as i32;
            lines.push(format!(
                "・「{}」商市急缺「{}」：現存 {} 份（目標需求 {} 份，缺口約 {}%），坊間公道收購價 {} 鎂。",
                s, name, cur, tgt, pct, pr
            ));
        }
        lines.join("\n")
    };

    // 二、【山海聯運與受阻急貨】
    let s2_body = {
        let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
        let mut active: Vec<_> = orders
            .values()
            .filter(|o| o.escrow_bounty_mg > 0 && now_unix <= o.expires_at_unix)
            .collect();
        active.sort_by_key(|o| o.expires_at_unix);
        if active.is_empty() {
            "目前全島陸路商道與沿海舟楫往來順暢，各處驛站與渡頭暫無受阻滯留貨單。".to_string()
        } else {
            let mut lines = Vec::new();
            for o in active.into_iter().take(3) {
                lines.push(format!(
                    "・行商「{}」受阻滯留於「{}」：急需送達「{}」之「{}」，公庫託管賞格 {} 鎂。",
                    o.carrier_name, o.waystation_name, o.dest_settlement, o.item_name, o.escrow_bounty_mg
                ));
            }
            lines.join("\n")
        }
    };

    // 三、【山海物候與獵場警訊】
    let z_idx = crate::game::resolve_zone_idx_from_name_or_settlement(room_name);
    let time_tick = now_unix / 180;
    let (weather, tide) = crate::game::current_zone_weather_and_tide(z_idx, time_tick);
    let weather_str = match weather {
        crate::game::ZoneWeatherPhase::RainMist => "微雨薄霧，山林潤濕",
        crate::game::ZoneWeatherPhase::ClearBreeze => "晴和天青，清風舒爽",
        crate::game::ZoneWeatherPhase::DryWarm => "日暖乾燥，山道朗朗",
        crate::game::ZoneWeatherPhase::ColdFrost => "霜寒料峭，林霧凝冷",
    };
    let tide_str = match tide {
        crate::game::ZoneTidePhase::FloodTide => "漲潮，海浪拍岸湧潮滿水",
        crate::game::ZoneTidePhase::EbbTide => "退潮，沿海礁石潮池露灘",
        crate::game::ZoneTidePhase::SlackTide => "潮平水穩，水面浪靜",
    };
    let local_zone_name = crate::game::CANONICAL_HUNTING_ZONES
        .get(z_idx)
        .map(|(n, _, _)| *n)
        .unwrap_or("周邊獵場");
    let mut spillover_alerts = Vec::new();
    for z in 0..=5 {
        if crate::game::is_zone_predator_spillover_active(z) {
            let zn = crate::game::CANONICAL_HUNTING_ZONES.get(z).map(|(n, _, _)| *n).unwrap_or("山林獵場");
            spillover_alerts.push(format!("・【{}】猛獸受驚越界伏行，山道行旅受阻，請往來商隊與巡行者多加戒備！", zn));
        }
    }
    let spillover_section = if spillover_alerts.is_empty() {
        "各主要關隘與山道巡防井然，暫無猛獸越界侵擾通報。".to_string()
    } else {
        spillover_alerts.join("\n")
    };
    let s3_body = format!(
        "・所在水土物候（{}）：氣候呈「{}」，潮汐呈「{}」。\n{}",
        local_zone_name, weather_str, tide_str, spillover_section
    );

    // 四、【街坊新近見聞】
    let mut seen_facts = std::collections::HashSet::new();
    let mut ripples = Vec::new();
    {
        let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
        for state in states.values() {
            for rip in &state.active_ripples {
                if seen_facts.insert(rip.fact_id.clone()) {
                    let salience = rip.severity * (0.65_f32).powi(rip.hop_count as i32);
                    ripples.push((salience, rip.prose_summary.clone()));
                }
            }
        }
    }
    ripples.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let ripple_lines: Vec<String> = ripples.into_iter().take(2).map(|(_, s)| format!("・{}", s)).collect();
    let ripple_text = if ripple_lines.is_empty() {
        "・坊間鄰里生活安穩，近期未有重大異動風聲。".to_string()
    } else {
        ripple_lines.join("\n")
    };

    let script_snippet = if let Some((ev, sc)) =
        crate::game::peninsula_scripts::active_event_and_script_pack_for_room(room_name, None, now_unix)
    {
        format!(
            "・街坊風貌【{}】：{}（劇本推進：{}）",
            ev.event_title, ev.trigger_and_causal_chain, sc.outcome_and_world_change
        )
    } else {
        "・街坊風貌：街坊作坊按部就班，鄰里互通有無，各自照料門前營生。".to_string()
    };
    let s4_body = format!("{}\n{}", ripple_text, script_snippet);

    format!(
        "《半島旬報・街坊公議與山海物候》\n\n一、【四鎮商貿與緊缺行情】\n{}\n\n二、【山海聯運與受阻急貨】\n{}\n\n三、【山海物候與獵場警訊】\n{}\n\n四、【街坊新近見聞】\n{}",
        s1_body, s2_body, s3_body, s4_body
    )
}

/// 領取竹筒信箱親善信札與隨信回禮
///
/// 傳回 `(手札內文, Option<(回禮物料ID, 回禮物料名稱)>)`
pub fn claim_acquaintance_mailbox_letter(
    player_id: &str,
    player_name: &str,
    now_unix: i64,
) -> (String, Option<(String, &'static str)>) {
    let mut found_acquaintance = None;
    {
        let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
        for (npc_id, state) in states.iter() {
            for h in &state.hooks {
                let matches_player = (!player_id.is_empty() && h.target_id == player_id)
                    || (!player_name.is_empty() && (h.target_name == player_name || player_name.contains(&h.target_name)));
                let is_positive = !h.kind_label.contains("怨")
                    && (h.intensity >= 10
                        || h.kind_label.contains("恩")
                        || h.kind_label.contains("師")
                        || h.kind_label.contains("契")
                        || h.kind_label.contains("信")
                        || h.kind_label.contains("同行"));
                if matches_player && is_positive {
                    found_acquaintance = Some((npc_id.clone(), h.kind_label.clone(), h.summary.clone()));
                    break;
                }
            }
            if found_acquaintance.is_some() {
                break;
            }
        }
    }

    let (sender_name, summary) = if let Some((npc_id, kind_label, hook_summary)) = found_acquaintance {
        let name = if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            s.entities.get(&npc_id).map(|e| {
                if !e.display_title.is_empty() {
                    e.display_title.clone()
                } else {
                    npc_id.clone()
                }
            }).unwrap_or(npc_id)
        } else {
            npc_id
        };
        let sum = if hook_summary.is_empty() {
            format!("念及昔日「{}」之誼", kind_label)
        } else {
            hook_summary
        };
        (name, sum)
    } else {
        let imprints = crate::pg::living_world::get_entity_lineage_imprints(player_id);
        if let Some(imp) = imprints.first() {
            (
                imp.mentor_name.clone(),
                format!("記掛授藝指點「{}」之傳承", imp.imprint_title),
            )
        } else {
            ("三水口信使總站・韓小駒".to_string(), "街坊安居問候".to_string())
        }
    };

    let letter_text = format!(
        "【竹筒信箱・親善手札】\n展信悅。見字如晤，寄信人：「{}」。\n信中言道：「{}，近來見白塔半島風物調勻，特修書一封問候，盼安居順遂、道途平穩。」",
        sender_name, summary
    );

    let window = now_unix / 300;
    let mut mailbox = LIVE_ACQUAINTANCE_MAILBOX.write().unwrap_or_else(|e| e.into_inner());
    let unread_prefix = format!("{player_id}:unread:");
    let unread_keys: Vec<String> = mailbox
        .keys()
        .filter(|k| k.starts_with(&unread_prefix) && mailbox.get(*k).copied().unwrap_or(0) > 0)
        .cloned()
        .collect();
    for k in unread_keys {
        if let Some(cnt) = mailbox.get_mut(&k) {
            if *cnt > 0 {
                *cnt -= 1;
                break;
            }
        }
    }
    let last_window = mailbox.get(player_id).copied();
    if last_window != Some(window) {
        mailbox.insert(player_id.to_string(), window);
        const ROTATING_GIFTS: &[(&str, &'static str)] = &[
            ("wt_cat_0501", "穩火慢煮牛埔鮮奶茶"),
            ("wt_cat_0500", "東橋茶屋山風草配霜林烘青茶"),
            ("wt_cat_0723", "旋蓋夜光貝母調光台燈"),
        ];
        let gift_idx = (window.unsigned_abs() as usize) % ROTATING_GIFTS.len();
        let (gift_id, gift_name) = ROTATING_GIFTS[gift_idx];
        (letter_text, Some((gift_id.to_string(), gift_name)))
    } else {
        (letter_text, None)
    }
}

/// 記錄或更新特定住民對玩家之親善同行關係，並在信箱排入親善問候手札
pub fn record_player_npc_fellowship_hook(
    npc_id: &str,
    player_id: &str,
    summary: &str,
    now_unix: i64,
) {
    let player_name = if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
        s.entities
            .get(player_id)
            .map(|e| {
                if !e.display_title.trim().is_empty() {
                    e.display_title.clone()
                } else {
                    player_id.to_string()
                }
            })
            .unwrap_or_else(|| player_id.to_string())
    } else {
        player_id.to_string()
    };

    {
        let mut states = LIVE_NPC_CAUSAL_STATES.write().unwrap_or_else(|e| e.into_inner());
        let state = states.entry(npc_id.to_string()).or_insert_with(|| {
            NpcCausalState {
                cornerstone: Cornerstone::Honor,
                stress: 10,
                stress_break_active: false,
                hunger: 10,
                thirst: 10,
                fatigue: 10,
                plan_steps: [
                    "巡視工坊物料與庫存".to_string(),
                    "手談切磋與街坊敘舊".to_string(),
                    "回房整理行囊帳目".to_string(),
                ],
                plan_cursor: 0,
                belief_scarcity_item: "常備物料".to_string(),
                last_surprise_delta: None,
                hooks: Vec::new(),
                life_step_tick: 1,
                same_task_repetitions: 1,
                carryover_from_yesterday: "昨日工序延續".to_string(),
                decision_image_code: "interact_or_unwind".to_string(),
                eligible_option_count: 3,
                chosen_action_code: "social_chat".to_string(),
                active_need: "街坊敘舊".to_string(),
                image_decision: "同席手談".to_string(),
                situated_action: "手談切磋".to_string(),
                belief_summary: "常業平順".to_string(),
                active_ripples: Vec::new(),
                immune_fact_ids: Vec::new(),
            }
        });
        let mut found = false;
        for h in &mut state.hooks {
            let matches = (!player_id.is_empty() && h.target_id == player_id)
                || (!player_name.is_empty() && (h.target_name == player_name || player_name.contains(&h.target_name)));
            if matches {
                h.kind_label = "結識同行".to_string();
                h.target_name.clone_from(&player_name);
                h.target_id = player_id.to_string();
                h.summary = summary.to_string();
                h.intensity = 75;
                h.updated_at_unix = now_unix;
                found = true;
                break;
            }
        }
        if !found {
            state.hooks.push(RelationalHook {
                kind_label: "結識同行".to_string(),
                target_name: player_name,
                summary: summary.to_string(),
                intensity: 75,
                target_id: player_id.to_string(),
                updated_at_unix: now_unix,
            });
            if state.hooks.len() > 8 {
                state.hooks.sort_by(|a, b| b.intensity.cmp(&a.intensity));
                state.hooks.truncate(8);
            }
        }
    }

    {
        let mut mailbox = LIVE_ACQUAINTANCE_MAILBOX.write().unwrap_or_else(|e| e.into_inner());
        let act_key = format!("{player_id}:unread:{summary}");
        let unread_count = mailbox.get(&act_key).copied().unwrap_or(0);
        if unread_count < 2 {
            mailbox.insert(act_key, unread_count + 1);
            // 讓玩家能在信箱立即領取此親善手札
            mailbox.remove(player_id);
        }
    }
}

/// 記錄聚落代表工坊之物料產出，向市集補充物料庫存並平抑物價波動
pub fn record_settlement_market_production(settlement: &str, item_id: &str, quantity: i32) {
    let clean_settlement = clean_settlement_name(settlement);
    let _ = crate::game::object_ecology::record_settlement_market_trade(
        clean_settlement,
        item_id,
        10,
        quantity,
        "聚落代表工坊產出",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocational_vessel_maps_distinct_occupations_not_only_hunting_knife() {
        let scholar = resolve_vocational_vessel("文曜・書院講席學者");
        assert_eq!(scholar.item_id, "craft_peninsula_almanac");
        assert!(!scholar.is_bladed);

        let cook = resolve_vocational_vessel("南哨・街角食堂掌事");
        assert_eq!(cook.item_id, "craft_kitchen_cleaver");
        assert!(cook.is_bladed);

        let merchant = resolve_vocational_vessel("風舶角・商行帳房管事");
        assert_eq!(merchant.item_id, "craft_copper_abacus");

        let hunter = resolve_vocational_vessel("紅土崁・山林獵戶");
        assert_eq!(hunter.item_id, "craft_steel_hunting_knife");
        assert!(hunter.is_bladed);

        let apprentice = resolve_vocational_vessel("風舶角・船塢年輕學徒");
        assert_eq!(apprentice.item_id, "craft_wooden_tool_handle");
        assert_eq!(apprentice.display_name, "百工學徒手用小木尺");

        let bamboo_weaver = resolve_vocational_vessel("分家巷・竹器巧編院匠人");
        assert_eq!(bamboo_weaver.item_id, "craft_wooden_tool_handle");
        assert_eq!(bamboo_weaver.display_name, "魯班平推刨刀");

        let stone_miller = resolve_vocational_vessel("老堰頭水輪石磨首座|秦水生");
        assert_eq!(stone_miller.item_id, "wt_cat_0973");
        assert_eq!(stone_miller.display_name, "平底石磨分粉篩架");
        assert!(!stone_miller.is_bladed);
    }

    #[test]
    fn belief_surprise_and_stress_break_pivot_3step_plan() {
        let state = advance_causal_state_on_arrival(
            "test_npc_causal_01",
            "文曜・鐵匠工坊匠人",
            "文曜",
            42,
            "文曜・西門大市集",
            1,
            false,
            false,
            30,
            Some("特級精煉煤"),
        );
        assert_eq!(state.belief_scarcity_item, "特級精煉煤");
        assert!(
            state
                .last_surprise_delta
                .as_deref()
                .unwrap_or("")
                .contains("轉缺「特級精煉煤」")
        );
        assert!(state.plan_steps[1].contains("特級精煉煤"));

        // Trigger high stress break
        let stressed = advance_causal_state_on_arrival(
            "test_npc_causal_01",
            "文曜・鐵匠工坊匠人",
            "文曜",
            42,
            "文曜・百鍛鐵工坊",
            6,
            true,
            true,
            5,
            Some("特級精煉煤"),
        );
        assert!(stressed.stress >= 72);
        assert!(stressed.stress_break_active);
    }

    #[test]
    fn somatic_renderer_incorporates_vessel_plan_and_hook() {
        let text = render_somatic_10layer_thought(
            "test_npc_somatic_02",
            "大圳頭・書院講席",
            "大圳頭",
            77,
            "正在講堂整理卷宗",
            "白塔風物考工手札（微有磨損）",
            "靜室清息",
            &[],
            "",
            45,
        );
        assert!(text.contains("白塔風物考工手札"));
        assert!(text.contains("下一步") || text.contains("承接") || text.contains("盤算"));
    }

    #[test]
    fn life_line_is_monotonic_and_waking_up_does_not_reset_to_zero() {
        let entity_id = "test_npc_linear_life_01";
        let role = "文曜・工坊木匠";
        let settlement = "文曜";
        let seed = 108;

        // 1. 晨間工坊上工：初始時間刻度與同業累計
        let s1 = advance_causal_state_on_arrival(
            entity_id,
            role,
            settlement,
            seed,
            "文曜・百工木器坊",
            1,
            false,
            false,
            30,
            None,
        );
        let tick1 = s1.life_step_tick;
        let rep1 = s1.same_task_repetitions;
        assert!(tick1 >= 1);
        assert!(!s1.carryover_from_yesterday.is_empty());

        // 2. 市集交割差事：時間刻度單調推進
        let s2 = advance_causal_state_on_arrival(
            entity_id,
            role,
            settlement,
            seed,
            "文曜・西門大市集",
            2,
            false,
            false,
            35,
            None,
        );
        assert!(s2.life_step_tick > tick1);
        assert!(s2.same_task_repetitions >= rep1);

        // 3. 夜間就寢安歇：起床不是歸零重整，生命是一條直線
        let s3 = advance_causal_state_on_arrival(
            entity_id,
            role,
            settlement,
            seed,
            "文曜・分家巷宅居",
            0,
            false,
            false,
            35,
            None,
        );
        assert!(s3.life_step_tick > s2.life_step_tick);
        assert!(!s3.carryover_from_yesterday.is_empty());
        assert!(s3.carryover_from_yesterday.contains("延續至今晨"));

        // 4. 次日晨起開工：時間刻度與同業累計持續遞增，延續昨日底蘊
        let s4 = advance_causal_state_on_arrival(
            entity_id,
            role,
            settlement,
            seed,
            "文曜・百工木器坊",
            1,
            false,
            false,
            35,
            None,
        );
        assert!(s4.life_step_tick > s3.life_step_tick);
        assert!(s4.same_task_repetitions > rep1);
        assert!(!s4.carryover_from_yesterday.is_empty());
        assert!(!s4.active_need.is_empty());
        assert!(!s4.image_decision.is_empty());
        assert!(!s4.situated_action.is_empty());

        let mark = format_time_scale_mark(&s4, 1720000000);
        assert!(mark.contains("同業累計"));
        assert!(mark.contains("回"));
    }

    #[test]
    fn abundance_unlocks_more_action_options_than_scarcity_across_all_decision_images() {
        let all_images = [
            DecisionImageKind::Eat,
            DecisionImageKind::Drink,
            DecisionImageKind::SleepOrRest,
            DecisionImageKind::HealOrTend,
            DecisionImageKind::MendOrEquip,
            DecisionImageKind::WorkOrTrade,
            DecisionImageKind::InteractOrUnwind,
        ];

        for img in all_images {
            let scarcity_opts = eligible_actions_for_image(img, 0);
            let subsistence_opts = eligible_actions_for_image(img, 12);
            let abundance_opts = eligible_actions_for_image(img, 40);

            assert_eq!(
                scarcity_opts.len(),
                2,
                "匱乏（沒錢 0 鎂）時，決策意象 {:?} 應僅有 2 個求生窄選項",
                img
            );
            assert_eq!(
                subsistence_opts.len(),
                3,
                "溫飽（有小錢 12 鎂）時，決策意象 {:?} 應有 3 個務實選項",
                img
            );
            assert!(
                abundance_opts.len() >= 5,
                "有餘裕（40 鎂）時，決策意象 {:?} 選項應大幅展開（>=5），實際為 {}",
                img,
                abundance_opts.len()
            );
            assert!(
                abundance_opts.len() > subsistence_opts.len()
                    && subsistence_opts.len() > scarcity_opts.len(),
                "必須嚴格滿足「有餘裕的選擇多，匱乏的選擇少」"
            );
        }

        // 驗證同一人在「食堂」面對「餓了 ➔ 吃」時，沒錢（2 鎂）與有餘裕（50 鎂）做出的選擇截然不同
        let poor_eat = advance_causal_state_on_arrival(
            "test_npc_margin_eat_poor",
            "文曜・工坊學徒",
            "文曜",
            19,
            "文曜・街角食堂",
            1,
            false,
            false,
            2,
            None,
        );
        let rich_eat = advance_causal_state_on_arrival(
            "test_npc_margin_eat_rich",
            "文曜・商行主事",
            "文曜",
            19,
            "文曜・街角食堂",
            1,
            false,
            false,
            50,
            None,
        );
        assert_eq!(poor_eat.decision_image_code, "eat");
        assert_eq!(rich_eat.decision_image_code, "eat");
        assert!(rich_eat.eligible_option_count > poor_eat.eligible_option_count);
        assert_ne!(poor_eat.chosen_action_code, rich_eat.chosen_action_code);
    }

    static TEST_ECONOMY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn total_conserved_magnesium_remains_strictly_invariant_across_50_actions() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

        let store_arc = crate::store::get_store().expect("store available");
        let mut store = store_arc.write();

        normalize_and_seed_circular_economy_once(&mut store);

        let test_actor_id = "test_npc_conserved_actor";
        let test_peer_id = "test_npc_conserved_peer";
        let test_wealthy_id = "test_npc_conserved_wealthy";

        let actor: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":30,"display_title":"文曜・百鍛鐵工匠"}}"#,
            test_actor_id
        )).unwrap();
        let peer: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":5,"display_title":"文曜・工坊學徒"}}"#,
            test_peer_id
        )).unwrap();
        let wealthy: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":100,"display_title":"文曜・大商行大主事"}}"#,
            test_wealthy_id
        )).unwrap();

        store.entities.insert(test_actor_id.to_string(), actor);
        store.entities.insert(test_peer_id.to_string(), peer);
        store.entities.insert(test_wealthy_id.to_string(), wealthy);

        let initial_total = total_conserved_magnesium(&store);
        assert!(initial_total > 0, "Initial conserved magnesium must be positive");

        let action_codes = [
            "eat_cheap_coarse_noodles",
            "work_perform_steady_daily_vocation",
            "eat_treat_peers_at_table",
            "work_peddle_gathered_goods_at_market",
            "drink_roadside_coarse_tea_bowl",
            "work_craft_masterpiece_at_own_pace",
            "heal_clinic_basic_clean_and_bandage",
            "social_bail_out_and_sponsor_struggling_peer",
            "rest_rent_cheap_common_bunk",
            "work_scavenge_wild_firewood_and_scraps",
        ];

        let now = 1720000000i64;
        for i in 0..50 {
            let code = action_codes[i % action_codes.len()];
            let _net = execute_conserved_action_settlement(
                &mut store,
                test_actor_id,
                "文曜",
                "文曜・百鍛鐵工坊",
                code,
                now + (i as i64) * 60,
            );
            let current_total = total_conserved_magnesium(&store);
            assert_eq!(
                current_total,
                initial_total,
                "Total conserved magnesium drifted on step {}: before={}, after={}",
                i,
                initial_total,
                current_total
            );
        }
    }

    #[test]
    fn format_predefined_stay_activity_contains_zero_debug_tags() {
        let causal = NpcCausalState {
            cornerstone: Cornerstone::Craft,
            stress: 20,
            stress_break_active: false,
            hunger: 30,
            thirst: 25,
            fatigue: 20,
            plan_steps: ["".to_string(), "".to_string(), "".to_string()],
            plan_cursor: 0,
            belief_scarcity_item: "特級精煉煤".to_string(),
            last_surprise_delta: None,
            hooks: Vec::new(),
            life_step_tick: 150,
            same_task_repetitions: 45,
            carryover_from_yesterday: "昨日手感延續".to_string(),
            decision_image_code: "work_or_trade".to_string(),
            eligible_option_count: 6,
            chosen_action_code: "work_craft_masterpiece_at_own_pace".to_string(),
            active_need: "".to_string(),
            image_decision: "".to_string(),
            situated_action: "".to_string(),
            belief_summary: String::new(),
            active_ripples: Vec::new(),
            immune_fact_ids: Vec::new(),
        };

        let result = format_predefined_stay_activity("文曜・百鍛鐵工坊", &causal, 45);
        assert!(!result.contains("項符合條件選項中擇"), "Must not contain option selection debug tag");
        assert!(!result.contains('【'), "Must not contain opening bracket debug tag");
        assert!(!result.contains('】'), "Must not contain closing bracket debug tag");
    }

    #[test]
    fn plan_next_causal_action_advances_somatic_needs_and_derives_forward_causality() {
        let (state, chosen) = plan_next_causal_action(
            "test_npc_causal_plan_01",
            "文曜・百鍛鐵匠",
            "文曜",
            42,
            "文曜・分家巷宅居",
            0,
            false,
            false,
            28,
            12,
            None,
        );
        assert!(state.hunger >= 12);
        assert!(state.thirst >= 14);
        assert!(state.fatigue >= 10);
        assert_eq!(state.decision_image_code, "eat");
        assert_eq!(chosen.image, DecisionImageKind::Eat);
        assert!(chosen.cost_mg <= 28);
    }

    #[test]
    fn commodity_stock_and_dynamic_price_respond_to_npc_consumption_and_supply() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

        let store_arc = crate::store::get_store().expect("store available");
        let mut store = store_arc.write();

        normalize_and_seed_circular_economy_once(&mut store);

        let test_actor_id = "test_npc_dynamic_actor";
        let actor: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":50,"display_title":"文曜・常駐住民"}}"#,
            test_actor_id
        )).unwrap();
        store.entities.insert(test_actor_id.to_string(), actor);

        let initial_total = total_conserved_magnesium(&store);

        let opt_eat = lookup_action_option("eat_feast_mountain_sea_stew").unwrap();
        let opt_forage = lookup_action_option("work_scavenge_wild_firewood_and_scraps").unwrap();

        let initial_cost = dynamic_action_cost_mg("文曜", opt_eat);
        let (_, food_item_id, _) = crate::game::object_ecology::settlement_scarcity_multiplier_for_category("文曜", "food");
        let stock_before = crate::game::object_ecology::LIVE_ECONOMY_LEDGER
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .settlement_markets
            .get(&("文曜".to_string(), food_item_id.to_string()))
            .map(|c| c.stock)
            .unwrap_or(0);

        // 1. 住民真實消費飲食：扣減聚落現貨並推升動態物價
        for i in 0..3 {
            execute_conserved_action_settlement(
                &mut store,
                test_actor_id,
                "文曜",
                "文曜・公營食堂",
                "eat_feast_mountain_sea_stew",
                1720000000 + i * 100,
            );
        }

        let stock_after = crate::game::object_ecology::LIVE_ECONOMY_LEDGER
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .settlement_markets
            .get(&("文曜".to_string(), food_item_id.to_string()))
            .map(|c| c.stock)
            .unwrap_or(0);
        let post_eat_cost = dynamic_action_cost_mg("文曜", opt_eat);

        assert!(stock_after <= stock_before, "Stock must decrease after consumption");
        assert!(post_eat_cost >= initial_cost, "Dynamic cost must scale with consumption");

        // 2. 住民執行勞作採集補貨：物資庫存增加，並獲得動態工資
        let (_, forage_item_id, _) = crate::game::object_ecology::settlement_scarcity_multiplier_for_category("文曜", "forage");
        let forage_stock_before = crate::game::object_ecology::LIVE_ECONOMY_LEDGER
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .settlement_markets
            .get(&("文曜".to_string(), forage_item_id.to_string()))
            .map(|c| c.stock)
            .unwrap_or(0);

        for i in 0..3 {
            execute_conserved_action_settlement(
                &mut store,
                test_actor_id,
                "文曜",
                "文曜・西麓林道",
                "work_scavenge_wild_firewood_and_scraps",
                1720000500 + i * 100,
            );
        }

        let forage_stock_after = crate::game::object_ecology::LIVE_ECONOMY_LEDGER
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .settlement_markets
            .get(&("文曜".to_string(), forage_item_id.to_string()))
            .map(|c| c.stock)
            .unwrap_or(0);
        let earn_mg = dynamic_action_earn_mg("文曜", opt_forage);

        assert!(forage_stock_after >= forage_stock_before, "Stock must increase after work/forage");
        assert!(earn_mg >= 2, "Dynamic earn must be at least baseline");

        // 3. 全島總守恆鎂幣量維持恆定
        let current_total = total_conserved_magnesium(&store);
        assert_eq!(
            current_total,
            initial_total,
            "Total conserved magnesium must remain strictly invariant! initial={}, current={}",
            initial_total,
            current_total
        );
        store.entities.remove(test_actor_id);
    }

    #[test]
    fn no_design_jargon_in_frontend_strings() {
        const FORBIDDEN: &[&str] = &[
            "ΔE",
            "預擬選項",
            "決策意象",
            "觸發條件",
            "時間刻度 #",
            "壓負荷",
            "市價約",
            "項符合條件選項",
            "起床非歸零重整",
            "生命是一條直線",
            "選擇少",
            "選擇多",
            "➔",
        ];

        let sample_roles = [
            ("npc_jargon_01", "文曜・百鍛鐵工坊匠人", "文曜", "文曜・百鍛鐵工坊", 2, false, false),
            ("npc_jargon_02", "風舶角・商行帳房管事", "風舶角", "風舶角・港埠大市集", 18, false, true),
            ("npc_jargon_03", "紅土崁・山林獵戶", "紅土崁", "紅土崁・赤石坡", 45, true, false),
        ];

        for (id, role, settlement, dest, mg, aff, worn) in sample_roles {
            let (st1, _) = plan_next_causal_action(
                id,
                role,
                settlement,
                42,
                dest,
                2,
                aff,
                worn,
                mg,
                12,
                Some("特級精煉煤"),
            );
            let st2 = advance_causal_state_on_arrival(
                id,
                role,
                settlement,
                42,
                dest,
                6,
                aff,
                worn,
                mg,
                Some("山野鮮藥草"),
            );
            let aff_vec = if aff {
                vec!["左肩舊傷".to_string()]
            } else {
                Vec::new()
            };
            let thought = render_somatic_10layer_thought(
                id,
                role,
                settlement,
                42,
                "正在工坊忙活",
                "百鍛長柄鐵錘（稱手）",
                "爐火深息",
                &aff_vec,
                "",
                mg,
            );
            let act = format_predefined_stay_activity(dest, &st2, mg);
            let mark = format_time_scale_mark(&st2, 1720000000);

            for field in [
                &st1.active_need,
                &st1.image_decision,
                &st1.situated_action,
                &st1.carryover_from_yesterday,
                &st1.belief_summary,
                &st2.active_need,
                &st2.image_decision,
                &st2.situated_action,
                &st2.carryover_from_yesterday,
                &st2.belief_summary,
                &thought,
                &act,
                &mark,
            ] {
                for &bad in FORBIDDEN {
                    assert!(
                        !field.contains(bad),
                        "NPC frontend string leaked forbidden design term '{bad}': {field}"
                    );
                }
            }
        }

        let sample_ripple = WorldRippleFact {
            fact_id: "rip_sample_01".to_string(),
            kind: RippleFactKind::StrandedCargo,
            subject_zone_or_settlement: "落日岬".to_string(),
            item_or_actor: "特級精煉煤".to_string(),
            order_id: None,
            severity: 0.9,
            origin_tick: 1720000000,
            hop_count: 0,
            max_hops: 3,
            prose_summary: "行商【林雅筑】運送「特級精煉煤」行經落日岬受阻，貨單受滯急待接應".to_string(),
        };
        let sample_cargo = StrandedCargoOrder {
            order_id: "cargo_sample_01".to_string(),
            carrier_id: "carrier_01".to_string(),
            carrier_name: "林雅筑".to_string(),
            item_id: "coal_item".to_string(),
            item_name: "特級精煉煤".to_string(),
            qty: 10,
            origin_settlement: "文曜".to_string(),
            dest_settlement: "風舶角".to_string(),
            waystation_name: "落日岬".to_string(),
            waystation_coord: (10, 20),
            escrow_bounty_mg: 8,
            hazard_prose: "山道遭遇異獸盤踞，風雪阻道難行，行商【林雅筑】受困於落日岬".to_string(),
            claimed_by: None,
            created_at_unix: 1720000000,
            expires_at_unix: 1720003600,
        };

        const INSTANT_TIME_WORDS: &[&str] = &["剛才", "剛剛", "方才"];
        for field in [
            &sample_ripple.prose_summary,
            &sample_cargo.hazard_prose,
        ] {
            for &bad in FORBIDDEN {
                assert!(
                    !field.contains(bad),
                    "NPC frontend string leaked forbidden design term '{bad}': {field}"
                );
            }
            for &bad in INSTANT_TIME_WORDS {
                assert!(
                    !field.contains(bad),
                    "NPC frontend string leaked forbidden instant word '{bad}': {field}"
                );
            }
        }
    }

    #[test]
    fn test_sir_ripple_propagation_and_dag_loop_immunity() {
        let now = 1720000000;
        let fact = WorldRippleFact {
            fact_id: "fact_eco_predator_01".to_string(),
            kind: RippleFactKind::PredatorSpillover,
            subject_zone_or_settlement: "西麓林道".to_string(),
            item_or_actor: "裂谷霜豹".to_string(),
            order_id: None,
            severity: 0.9,
            origin_tick: now,
            hop_count: 0,
            max_hops: 2,
            prose_summary: "西麓林道風雪劇盛，裂谷霜豹現身擾道".to_string(),
        };

        let npc_a = "test_propagator_a";
        let npc_b = "test_propagator_b";
        let npc_c = "test_propagator_c";

        // Step 1: Inject fact into NPC A
        assert!(inject_ripple_fact_into_npc(npc_a, fact.clone(), now));
        // Re-injecting the same fact into NPC A fails due to immunity
        assert!(!inject_ripple_fact_into_npc(npc_a, fact.clone(), now));

        // NPC A's top ripple should be this fact
        let top_a = top_active_ripple_for_npc(npc_a, now).expect("top active ripple for A");
        assert_eq!(top_a.fact_id, "fact_eco_predator_01");
        assert_eq!(top_a.hop_count, 0);

        // Step 2: Propagate from A to B (hop 0 -> 1)
        let prop_b = propagate_ripple_between_npcs(npc_a, "張哲銘", npc_b, "李佳穎", now);
        assert!(prop_b.is_some());
        let fact_b = prop_b.unwrap();
        assert_eq!(fact_b.hop_count, 1);
        assert!(fact_b.prose_summary.contains("聽張哲銘提起"));

        // Check B has relational hook toward A
        let hook_b = find_relational_hook_for_target(npc_b, npc_a, "張哲銘");
        assert!(hook_b.is_some());
        assert_eq!(hook_b.unwrap().kind_label, "互通風聲");

        // Step 3: DAG loop immunity: B tries to propagate back to A -> should fail (A is immune)
        let back_prop = propagate_ripple_between_npcs(npc_b, "李佳穎", npc_a, "張哲銘", now);
        assert!(back_prop.is_none(), "DAG loop must be prevented by immunity list");

        // Step 4: Propagate from B to C (hop 1 -> 2)
        let prop_c = propagate_ripple_between_npcs(npc_b, "李佳穎", npc_c, "王俊傑", now);
        assert!(prop_c.is_some());
        let fact_c = prop_c.unwrap();
        assert_eq!(fact_c.hop_count, 2);

        // Step 5: C tries to propagate to D, but hop_count (2) >= max_hops (2), so must fail
        let npc_d = "test_propagator_d";
        let prop_d = propagate_ripple_between_npcs(npc_c, "王俊傑", npc_d, "陳冠宇", now);
        assert!(prop_d.is_none(), "Propagation must stop when hop_count reaches max_hops");
    }

    #[test]
    fn test_stranded_cargo_escrow_sfc_conservation_and_delivery_ripple() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

        let store_arc = crate::store::get_store().expect("store available");
        let mut store = store_arc.write();
        normalize_and_seed_circular_economy_once(&mut store);

        let test_carrier_id = "test_npc_cargo_carrier";
        let test_rescuer_id = "test_npc_cargo_rescuer";

        let carrier: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":40,"display_title":"風舶角・行商掌事"}}"#,
            test_carrier_id
        )).unwrap();
        let rescuer: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":25,"display_title":"風舶角・健行巡客"}}"#,
            test_rescuer_id
        )).unwrap();

        store.entities.insert(test_carrier_id.to_string(), carrier);
        store.entities.insert(test_rescuer_id.to_string(), rescuer);

        let sfc_baseline = total_conserved_magnesium(&store);
        let now = crate::game::now_unix();

        // 1. Create a stranded cargo order with dest "風舶角"
        let order = create_stranded_cargo_order(
            test_carrier_id,
            "行商阿明",
            "item_heavy_copper",
            "紫銅錠",
            20,
            "文曜",
            "風舶角",
            "落日岬驛站",
            (15, 25),
            "落日岬風急雪大，商隊車軸受損困於驛站",
            now,
        ).expect("Order should be created");

        assert_eq!(order.escrow_bounty_mg, 8);
        // Strict SFC Invariance check:
        assert_eq!(
            total_conserved_magnesium(&store),
            sfc_baseline,
            "Total magnesium must remain strictly invariant after escrow lock"
        );

        // Carrier should receive StrandedCargo ripple fact
        let carrier_rip = top_active_ripple_for_npc(test_carrier_id, now).expect("carrier ripple");
        assert_eq!(carrier_rip.kind, RippleFactKind::StrandedCargo);

        // 2. Deliver/claim the cargo by rescuer
        let (delivered, bounty) = claim_or_deliver_stranded_cargo_for_npc(
            test_rescuer_id,
            "健行客阿華",
            "落日岬驛站",
            (15, 26),
            now + 120,
        ).expect("Rescuer should be able to claim/deliver near waystation");

        assert_eq!(delivered.order_id, order.order_id);
        assert_eq!(bounty, 8);

        // Rescuer magnesium credited via credit_magnesium_bounty
        credit_magnesium_bounty(&mut store, test_rescuer_id, bounty);

        // Strict SFC Invariance check after delivery:
        assert_eq!(
            total_conserved_magnesium(&store),
            sfc_baseline,
            "Total magnesium must remain strictly invariant after cargo delivery and payout"
        );

        // Verify relational hook formed on carrier toward rescuer
        let hook = find_relational_hook_for_target(test_carrier_id, test_rescuer_id, "健行客阿華")
            .expect("carrier should have hook for rescuer");
        assert_eq!(hook.kind_label, "山道解圍");
        assert!(hook.intensity >= 4);

        // Verify rescuer received ApexSlainOrRescued ripple
        let rescuer_rip = top_active_ripple_for_npc(test_rescuer_id, now + 120).expect("rescuer ripple");
        assert_eq!(rescuer_rip.kind, RippleFactKind::ApexSlainOrRescued);

        // 3. Test escrow refund on expiration
        let order2 = create_stranded_cargo_order(
            test_carrier_id,
            "行商阿明",
            "item_herbs",
            "冷杉藥草",
            10,
            "文曜",
            "風舶角",
            "赤石坡",
            (10, 10),
            "赤石坡巨石滾落阻斷棧道",
            now + 200,
        ).expect("Order 2 should be created");

        assert_eq!(order2.escrow_bounty_mg, 8);
        assert_eq!(total_conserved_magnesium(&store), sfc_baseline);

        // Force expiration of order2
        {
            let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
            if let Some(o) = orders.get_mut(&order2.order_id) {
                o.expires_at_unix = 0;
            }
        }

        // When total_conserved_magnesium runs, it will sweep expired orders and refund escrow
        let sfc_after_sweep = total_conserved_magnesium(&store);
        assert_eq!(
            sfc_after_sweep,
            sfc_baseline,
            "Total magnesium must remain strictly invariant after escrow expiration refund"
        );

        let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
        let o2 = orders.get(&order2.order_id).expect("order2 should exist");
        assert_eq!(o2.escrow_bounty_mg, 0, "Escrow should be zeroed after refund");
    }

    #[test]
    fn test_dynamic_relational_hook_and_player_action_mutation() {
        let now = 1720100000;
        let npc_id = "test_npc_rel_mut_01";
        let player_id = "player_hero_01";
        let player_name = "青衣遊俠";

        // Initial state
        get_or_init_causal_state(npc_id, "文曜・百鍛鐵工坊匠人", "文曜", 42);

        // Test player rescue action
        mutate_npc_from_player_action(
            npc_id,
            player_id,
            player_name,
            "文曜",
            "rescue",
            "百鍛長柄鐵錘",
            0,
            now,
        );

        let hook = find_relational_hook_for_target(npc_id, player_id, player_name)
            .expect("Hook should be recorded after rescue");
        assert_eq!(hook.kind_label, "救命之恩");
        assert!(hook.intensity >= 6);
        assert!(hook.summary.contains("百鍛長柄鐵錘"));

        // Check ripple fact injected
        let rip = top_active_ripple_for_npc(npc_id, now).expect("Should have ripple fact");
        assert_eq!(rip.kind, RippleFactKind::PlayerDeed);
        assert!(rip.prose_summary.contains(player_name));

        // Test trade action updates hook
        mutate_npc_from_player_action(
            npc_id,
            player_id,
            player_name,
            "文曜",
            "trade",
            "特級精煉煤",
            10,
            now + 60,
        );

        let updated_hook = find_relational_hook_for_target(npc_id, player_id, player_name)
            .expect("Hook should be updated");
        assert_eq!(updated_hook.kind_label, "商貿往來");
        assert!(updated_hook.intensity > hook.intensity);

        // Test manual upsert_relational_hook cap at 100
        upsert_relational_hook(
            npc_id,
            player_id,
            player_name,
            "莫逆之交",
            "歷經患難，信諾千金".to_string(),
            99,
            now + 120,
        );
        let max_hook = find_relational_hook_for_target(npc_id, player_id, player_name).unwrap();
        assert_eq!(max_hook.intensity, 100);
    }
    #[test]
    fn test_settle_player_with_settlement_treasury_sfc() {
        let pid = "test_player_sfc_01";
        let res = settle_player_with_settlement_treasury(pid, "文曜", 10);
        assert!(res >= 0);
        let res_deduct = settle_player_with_settlement_treasury(pid, "文曜", -10);
        assert!(res_deduct <= 0);
    }

    #[test]
    fn test_claim_and_deliver_stranded_cargo_for_player() {
        let now = 1000000;
        let order = StrandedCargoOrder {
            order_id: "cargo_test_claim_01".to_string(),
            carrier_id: "carrier_npc_01".to_string(),
            carrier_name: "林主事".to_string(),
            item_id: "mat_grain_sack".to_string(),
            item_name: "大圳頭飽滿新米".to_string(),
            qty: 2,
            origin_settlement: "大圳頭".to_string(),
            dest_settlement: "文曜".to_string(),
            waystation_name: "落日岬驛站".to_string(),
            waystation_coord: (15, 26),
            escrow_bounty_mg: 12,
            hazard_prose: "路經落日岬遇風浪受阻".to_string(),
            claimed_by: None,
            created_at_unix: now,
            expires_at_unix: now + 3600,
        };
        {
            let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
            orders.insert(order.order_id.clone(), order);
        }

        let pid = "player_hero_01";
        let pname = "義士阿強";

        // Claim by player
        let claimed = claim_stranded_cargo_for_player(pid, pname, "cargo_test_claim_01", "文曜", now + 10)
            .expect("Should claim order");
        assert_eq!(claimed.claimed_by.as_deref(), Some(pid));

        // Another NPC should not claim this order
        let npc_claim = claim_or_deliver_stranded_cargo_for_npc(
            "npc_other_01",
            "路人NPC",
            "落日岬驛站",
            (15, 26),
            now + 20,
        );
        assert!(npc_claim.is_none(), "NPC should skip player-claimed order");

        // Player delivers cargo
        let (delivered, bounty) = deliver_stranded_cargo_for_player(
            pid,
            pname,
            "cargo_test_claim_01",
            "文曜",
            now + 30,
        ).expect("Player delivery should succeed");
        assert_eq!(delivered.order_id, "cargo_test_claim_01");
        assert_eq!(bounty, 12);
    }

    #[test]
    fn test_offline_autonomy_directives_and_recaps() {
        let pid = "player_offline_test_01";
        assert_eq!(get_player_offline_directive(pid), OfflineAutonomyDirective::InnRestAndHeal);

        set_player_offline_directive(pid, OfflineAutonomyDirective::WorkshopCraftAndAssist);
        assert_eq!(get_player_offline_directive(pid), OfflineAutonomyDirective::WorkshopCraftAndAssist);
        assert_eq!(OfflineAutonomyDirective::from_code("workshop_craft"), OfflineAutonomyDirective::WorkshopCraftAndAssist);
        assert_eq!(OfflineAutonomyDirective::from_code("field_tend"), OfflineAutonomyDirective::FieldTendAndForage);
        assert_eq!(OfflineAutonomyDirective::from_code("road_escort"), OfflineAutonomyDirective::RoadEscortAndPatrol);
        assert_eq!(OfflineAutonomyDirective::from_code("inn_rest"), OfflineAutonomyDirective::InnRestAndHeal);

        let now = 2000000;
        let step_res = step_offline_player_once(pid, now);
        assert!(step_res.is_some());

        let recaps = drain_player_offline_recap(pid, now);
        assert!(!recaps.is_empty());
        assert_eq!(recaps[0].0, "social");
        assert!(recaps[0].1.contains("駐坊"));

        // Drained second time returns empty when no new offline step occurred
        let recaps2 = drain_player_offline_recap(pid, now);
        assert!(recaps2.is_empty());
    }

    #[test]
    fn test_offline_autonomy_title_heuristics_and_physics() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut e_craft = crate::store::Entity::default();
            e_craft.id = "test_offline_crafter".to_string();
            e_craft.display_title = "水輪石磨工匠".to_string();
            e_craft.equipment_slots = r#"{"hold_r":"wt_cat_0984"}"#.to_string();
            e_craft.vit = 80;
            e_craft.qi = 60;
            e_craft.inventory = r#"[{"item_id":"wt-good-009","qty":5}]"#.to_string();
            s.entities.insert(e_craft.id.clone(), e_craft);

            let mut e_farm = crate::store::Entity::default();
            e_farm.id = "test_offline_farmer".to_string();
            e_farm.display_title = "靈田採藥圃農".to_string();
            e_farm.vit = 80;
            e_farm.qi = 60;
            s.entities.insert(e_farm.id.clone(), e_farm);

            let mut e_guard = crate::store::Entity::default();
            e_guard.id = "test_offline_guard".to_string();
            e_guard.display_title = "古道行商哨衛".to_string();
            e_guard.vit = 80;
            e_guard.qi = 60;
            s.entities.insert(e_guard.id.clone(), e_guard);

            let mut e_tired = crate::store::Entity::default();
            e_tired.id = "test_offline_tired".to_string();
            e_tired.display_title = "百工鐵匠".to_string();
            e_tired.vit = 15;
            e_tired.qi = 10;
            s.entities.insert(e_tired.id.clone(), e_tired);
        }

        assert_eq!(
            get_player_offline_directive("test_offline_crafter"),
            OfflineAutonomyDirective::WorkshopCraftAndAssist
        );
        assert_eq!(
            get_player_offline_directive("test_offline_farmer"),
            OfflineAutonomyDirective::FieldTendAndForage
        );
        assert_eq!(
            get_player_offline_directive("test_offline_guard"),
            OfflineAutonomyDirective::RoadEscortAndPatrol
        );

        let now = 2005000;
        let step_tired = step_offline_player_once("test_offline_tired", now);
        assert!(step_tired.is_some());
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get("test_offline_tired") {
                assert!(e.vit >= 25);
                assert_eq!(e.current_activity, "於客舍內閉門靜養、調勻氣血");
            }
        }

        let _ = step_offline_player_once("test_offline_crafter", now);
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get("test_offline_crafter") {
                assert_eq!(e.vit, 77);
                assert_eq!(e.qi, 58);
                assert!(e.current_activity.contains("工坊"));
            }
        }
        let vstate = crate::pg::living_world::get_vessel_state("test_offline_crafter", "wt_cat_0984");
        assert!(vstate.use_count >= 1);
    }

    #[test]
    fn test_offline_player_grid_stepping_and_material_transformation() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

        let pid = "test_offline_walker_crafter";
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut ent = crate::store::Entity::default();
            ent.id = pid.to_string();
            ent.display_title = "老堰頭水輪石磨工匠".to_string();
            ent.equipment_slots = r#"{"hold_r":"wt_cat_0984"}"#.to_string();
            ent.inventory = r#"[{"item_id":"wt-good-009","qty":5}]"#.to_string();
            ent.grid_x = Some(0);
            ent.grid_y = Some(100);
            ent.vit = 80;
            ent.qi = 60;
            s.entities.insert(ent.id.clone(), ent);
        }

        set_player_offline_directive(pid, OfflineAutonomyDirective::WorkshopCraftAndAssist);

        let now = 2010000;
        let step_res = step_offline_player_once(pid, now);
        assert!(step_res.is_some());

        // 1. 驗證實體 (grid_x, grid_y) 朝向老堰頭水輪石磨工坊目標座標 (-3, 125) 邁進
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            let ent = s.entities.get(pid).expect("player entity should exist in store");
            let nx = ent.grid_x.expect("grid_x should be Some");
            let ny = ent.grid_y.expect("grid_y should be Some");
            assert_ne!((nx, ny), (0, 100));
            assert_eq!(ent.target_x, Some(-3));
            assert_eq!(ent.target_y, Some(125));

            // 2. 驗證原料 (wt-good-009) 經工坊轉化為成品 (wt_cat_0388 溫米漿)
            let entries: Vec<crate::db::inventory::InventoryEntry> =
                serde_json::from_str(&ent.inventory).expect("inventory must be valid json");
            let raw_item = entries.iter().find(|it| it.item_id == "wt-good-009");
            assert_eq!(raw_item.map(|it| it.qty), Some(4));
            let finished_item = entries.iter().find(|it| it.item_id == "wt_cat_0388");
            assert!(finished_item.is_some());
            assert!(finished_item.unwrap().qty >= 1);
        }

        // 3. 驗證已成功登記至 CANONICAL_NPC_TARGETS
        {
            let targets = crate::server::grid_manager::CANONICAL_NPC_TARGETS.read();
            let entry = targets.get(pid);
            assert!(entry.is_some());
            let (tx, ty, bldg, act) = entry.unwrap();
            assert_eq!(*tx, -3);
            assert_eq!(*ty, 125);
            assert!(bldg.contains("石磨") || bldg.contains("老堰頭"));
            assert!(act.contains("工坊"));
        }
    }

    #[test]
    fn test_offline_player_and_canteen_spatial_need_decision() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

        // 1. 空間需求決策：resolve_nearest_room_by_tag 就近選取食堂，拒絕跨全地圖硬編碼至 (-4, 4)
        let mut test_grid = crate::grid::SquareGrid::new();
        let mut far_canteen = crate::grid::GridCell::new(
            crate::grid::SquareCoord::new(-4, 4),
            crate::grid::Terrain::Tavern,
            "文曜・三水大食堂",
        );
        far_canteen.tags.push("食堂".to_string());
        test_grid.insert(far_canteen);

        let mut near_canteen = crate::grid::GridCell::new(
            crate::grid::SquareCoord::new(20, 20),
            crate::grid::Terrain::Tavern,
            "東街・聚仙熱食館",
        );
        near_canteen.tags.push("食堂".to_string());
        near_canteen.tags.push("熱食".to_string());
        test_grid.insert(near_canteen);

        // 位於 (21, 20) 的角色查詢最近食堂，應選中 (20, 20)
        let (chosen_coord, chosen_bldg) = crate::pg::living_world::resolve_nearest_room_by_tag(
            (21, 20),
            &["食堂", "熱食", "現煮飲品"],
            Some(&test_grid),
            (-4, 4),
            "文曜・三水大食堂",
        );
        assert_eq!(chosen_coord, (20, 20));
        assert!(chosen_bldg.contains("聚仙") || chosen_bldg.contains("東街"));
        assert!(!chosen_bldg.contains("三水大食堂"));

        // 驗證若未提供 grid 且執行期 grid 未就緒時，保留既有 fallback 設施
        let (fb_coord, fb_bldg) = crate::pg::living_world::resolve_nearest_room_by_tag(
            (-3, 125),
            &["食堂", "熱食", "現煮飲品"],
            None,
            (1, 124),
            "老堰頭・堰渠水畔食堂",
        );
        assert_eq!(fb_coord, (1, 124));
        assert_eq!(fb_bldg, "老堰頭・堰渠水畔食堂");

        // 2. 離線玩家工坊缺料補料 (RestockGrain)：依 resolve_need_decision_supplier 動態採買原料
        let sup_room_coord = crate::grid::SquareCoord::new(21, 20);
        let mut sup_room = crate::grid::GridCell::new(
            sup_room_coord,
            crate::grid::Terrain::GeneralStore,
            "東街米糧行",
        );
        sup_room.tags.push("穀物".to_string());
        test_grid.insert(sup_room);

        // 注入測試地圖至執行期快照
        crate::server::grid_manager::set_runtime_square_grid_for_testing(test_grid.clone());

        let restock_pid = "test_offline_need_restocker";
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut ent = crate::store::Entity::default();
            ent.id = restock_pid.to_string();
            ent.display_title = "老堰頭水輪石磨工匠".to_string();
            ent.equipment_slots = r#"{"hold_r":"wt_cat_0984"}"#.to_string();
            ent.inventory = "[]".to_string(); // 原料存量為 0
            ent.grid_x = Some(21);
            ent.grid_y = Some(20);
            ent.vit = 80;
            ent.qi = 60;
            ent.magnesium = 50;
            s.entities.insert(ent.id.clone(), ent);
        }
        set_player_offline_directive(restock_pid, OfflineAutonomyDirective::WorkshopCraftAndAssist);

        let now = 2010000;
        let step_res = step_offline_player_once(restock_pid, now);
        assert!(step_res.is_some());
        let body = step_res.unwrap();
        assert!(body.contains("採買") || body.contains("用料"));
        let recaps = drain_player_offline_recap(restock_pid, now);
        assert!(!recaps.is_empty());
        assert!(recaps[0].1.contains("採購") || recaps[0].1.contains("採買"));

        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get(restock_pid) {
                let entries: Vec<crate::db::inventory::InventoryEntry> =
                    serde_json::from_str(&e.inventory).unwrap_or_default();
                let raw_count = entries
                    .iter()
                    .find(|it| it.item_id == "wt-good-009")
                    .map(|it| it.qty)
                    .unwrap_or(0);
                assert!(raw_count > 0, "原料應已成功採買補充至玩家行囊");
                assert!(e.magnesium < 50, "採買原料應已扣除相應鎂幣");
            }
        }

        // 3. 離線玩家完工交割 (DeliverContract)：依 resolve_need_decision_buyer 動態送交成品
        let buy_room_coord = crate::grid::SquareCoord::new(20, 21);
        let mut buy_room = crate::grid::GridCell::new(
            buy_room_coord,
            crate::grid::Terrain::Tavern,
            "東街四海茶樓",
        );
        buy_room.tags.push("食堂".to_string());
        test_grid.insert(buy_room);
        crate::server::grid_manager::set_runtime_square_grid_for_testing(test_grid);

        let deliver_pid = "test_offline_need_deliverer";
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut ent = crate::store::Entity::default();
            ent.id = deliver_pid.to_string();
            ent.display_title = "老堰頭水輪石磨工匠".to_string();
            ent.equipment_slots = r#"{"hold_r":"wt_cat_0984"}"#.to_string();
            ent.inventory = r#"[{"item_id":"wt-good-009","qty":2},{"item_id":"wt_cat_0388","qty":3}]"#.to_string();
            ent.grid_x = Some(20);
            ent.grid_y = Some(21);
            ent.vit = 80;
            ent.qi = 60;
            ent.magnesium = 10;
            s.entities.insert(ent.id.clone(), ent);
        }
        set_player_offline_directive(deliver_pid, OfflineAutonomyDirective::WorkshopCraftAndAssist);

        let step_deliv = step_offline_player_once(deliver_pid, now);
        assert!(step_deliv.is_some());
        let d_body = step_deliv.unwrap();
        assert!(d_body.contains("成品") || d_body.contains("點交") || d_body.contains("交割"));
        let d_recaps = drain_player_offline_recap(deliver_pid, now);
        assert!(!d_recaps.is_empty());
        assert!(d_recaps[0].1.contains("交割") || d_recaps[0].1.contains("貨款"));

        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get(deliver_pid) {
                let entries: Vec<crate::db::inventory::InventoryEntry> =
                    serde_json::from_str(&e.inventory).unwrap_or_default();
                let fin_count = entries
                    .iter()
                    .find(|it| it.item_id == "wt_cat_0388")
                    .map(|it| it.qty)
                    .unwrap_or(0);
                assert!(fin_count < 3, "交割後成品應已扣除");
                assert!(e.magnesium > 10, "交割後應已獲取貨款鎂幣進帳");
            }
        }

        // 4. 離線玩家古道巡護 (RoadEscortAndPatrol)：距離半徑過濾與動態同伴標籤
        let patrol_pid = "test_offline_need_patroller";
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut ent = crate::store::Entity::default();
            ent.id = patrol_pid.to_string();
            ent.display_title = "古道護衛巡客".to_string();
            ent.grid_x = Some(20);
            ent.grid_y = Some(20);
            ent.vit = 80;
            ent.qi = 60;
            s.entities.insert(ent.id.clone(), ent);

            // 設置近處同行商客 (距離 2 <= 30)
            let mut traveler = crate::store::Entity::default();
            traveler.id = "npc_traveler_east_01".to_string();
            traveler.kind = "npc".to_string();
            traveler.display_title = "東街同行商旅客".to_string();
            traveler.grid_x = Some(22);
            traveler.grid_y = Some(20);
            s.entities.insert(traveler.id.clone(), traveler);
        }
        set_player_offline_directive(patrol_pid, OfflineAutonomyDirective::RoadEscortAndPatrol);

        let step_patrol = step_offline_player_once(patrol_pid, now);
        assert!(step_patrol.is_some());
        let p_body = step_patrol.unwrap();
        assert!(p_body.contains("東街同行商旅客"), "必須選取距離近之行旅 NPC");
        assert!(!p_body.contains("古道行商阿福"), "不得使用硬編碼之特定 NPC 姓名");

        // 若四周無近處 NPC，同伴稱謂應依聚落動態生成，而非固定為古道行商阿福
        let lonely_pid = "test_offline_lonely_patroller";
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut ent = crate::store::Entity::default();
            ent.id = lonely_pid.to_string();
            ent.display_title = "荒野哨探巡行者".to_string();
            ent.grid_x = Some(900); // 孤立遠處無任何 NPC
            ent.grid_y = Some(900);
            ent.vit = 80;
            ent.qi = 60;
            s.entities.insert(ent.id.clone(), ent);
        }
        set_player_offline_directive(lonely_pid, OfflineAutonomyDirective::RoadEscortAndPatrol);

        let step_lonely = step_offline_player_once(lonely_pid, now);
        assert!(step_lonely.is_some());
        let l_body = step_lonely.unwrap();
        assert!(!l_body.contains("古道行商阿福"), "無同伴時不得出現硬編碼古道行商阿福");
        assert!(l_body.contains("往來行旅同伴"), "應使用動態同伴稱謂");
    }

    #[test]
    fn test_crop_plot_maturation_and_harvest() {
        let coord = (99, 101);
        let now = 3000000;

        // Register sown plot
        let sown = register_crop_plot_sown(coord.0, coord.1, "water_field", now);
        assert_eq!(sown.stage, CropGrowthStage::Seedling);
        assert_eq!(sown.crop_name, "大圳頭飽滿新米");

        // Harvesting now should fail (not mature)
        let early_harvest = harvest_crop_plot_once(coord.0, coord.1, "water_field", now + 10);
        assert!(early_harvest.is_err());

        // Fast forward 50s (water_nourished: 50 * 2 = 100s >= 90s -> Mature)
        let mature = inspect_or_advance_crop_plot(coord.0, coord.1, "water_field", now + 50);
        assert_eq!(mature.stage, CropGrowthStage::Mature);

        // Harvest mature crop
        let harvest = harvest_crop_plot_once(coord.0, coord.1, "water_field", now + 50);
        assert!(harvest.is_ok());
        let (item_id, item_name, qty, prose) = harvest.unwrap();
        assert_eq!(item_id, "mat_grain_sack");
        assert_eq!(item_name, "大圳頭飽滿新米");
        assert_eq!(qty, 2);
        assert!(prose.contains("熟成"));

        // Lazy-seeding native map field starts at Mature
        let native_plot = inspect_or_advance_crop_plot(88, 77, "herb_garden", now);
        assert_eq!(native_plot.stage, CropGrowthStage::Mature);
        assert_eq!(native_plot.crop_item_id, "harvest_herb");
    }

    #[test]
    fn test_player_bank_ledger_sfc_invariance() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

        let player_id = "test_player_bank_sfc_inv";
        {
            let store_arc = crate::store::get_store().expect("store available");
            let mut store = store_arc.write();
            normalize_and_seed_circular_economy_once(&mut store);
            let player: crate::store::Entity = serde_json::from_str(&format!(
                r#"{{"id":"{}","kind":"player","magnesium":500,"display_title":"白塔旅人"}}"#,
                player_id
            )).unwrap();
            store.entities.insert(player_id.to_string(), player);
        }

        {
            let mut ledger = LIVE_PLAYER_BANK_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            ledger.remove(player_id);
        }

        let store_arc = crate::store::get_store().expect("store available");
        let initial_total = total_conserved_magnesium(&store_arc.read());

        // Deposit 200
        let (actual_dep, remaining_wallet, new_bank) = deposit_player_bank_magnesium(player_id, 200);
        assert_eq!(actual_dep, 200);
        assert_eq!(remaining_wallet, 300);
        assert_eq!(new_bank, 200);
        assert_eq!(get_player_bank_balance(player_id), 200);
        let after_dep_total = total_conserved_magnesium(&store_arc.read());
        assert_eq!(initial_total, after_dep_total);

        // Withdraw 80
        let (actual_with, new_wallet, remaining_bank) = withdraw_player_bank_magnesium(player_id, 80);
        assert_eq!(actual_with, 80);
        assert_eq!(new_wallet, 380);
        assert_eq!(remaining_bank, 120);
        assert_eq!(get_player_bank_balance(player_id), 120);
        let after_with_total = total_conserved_magnesium(&store_arc.read());
        assert_eq!(initial_total, after_with_total);

        // Withdraw all remaining
        let (over_with, final_wallet, zero_bank) = withdraw_player_bank_magnesium(player_id, 9999);
        assert_eq!(over_with, 120);
        assert_eq!(final_wallet, 500);
        assert_eq!(zero_bank, 0);
        let final_total = total_conserved_magnesium(&store_arc.read());
        assert_eq!(initial_total, final_total);
    }

    #[test]
    fn test_player_home_stash_store_and_retrieve() {
        let pid = "test_player_home_stash_user";
        {
            let mut stash = LIVE_PLAYER_HOME_STASH.write().unwrap_or_else(|e| e.into_inner());
            stash.remove(pid);
        }
        assert_eq!(list_player_home_stash(pid).len(), 0);
        assert!(retrieve_item_from_home_stash(pid).is_none());

        for i in 1..=24 {
            let res = store_item_in_home_stash(pid, &format!("item_{}", i), &format!("道具_{}", i));
            assert_eq!(res, Ok(i));
        }
        assert_eq!(list_player_home_stash(pid).len(), 24);

        // 25th item should fail (cap 24)
        let overflow = store_item_in_home_stash(pid, "item_25", "道具_25");
        assert!(overflow.is_err());

        // Pop last
        let pop1 = retrieve_item_from_home_stash(pid);
        assert_eq!(pop1, Some(("item_24".to_string(), "道具_24".to_string(), 23)));
        assert_eq!(list_player_home_stash(pid).len(), 23);

        // Store again
        let store_again = store_item_in_home_stash(pid, "item_new", "新玉珮");
        assert_eq!(store_again, Ok(24));
    }

    #[test]
    fn test_compile_live_peninsula_gazette_and_mailbox_letter() {
        let now = 1700000000;
        let gazette = compile_live_peninsula_gazette("文曜・市集石坪", now);
        assert!(gazette.contains("【四鎮商貿與緊缺行情】"));
        assert!(gazette.contains("【山海聯運與受阻急貨】"));
        assert!(gazette.contains("【山海物候與獵場警訊】"));
        assert!(gazette.contains("【街坊新近見聞】"));
        assert!(!gazette.contains("SFC"));
        assert!(!gazette.contains("Stock-Flow"));
        assert!(!gazette.contains("RelationalHook"));
        assert!(!gazette.contains("WorldRippleFact"));
        assert!(!gazette.contains("LazyLock"));
        assert!(!gazette.contains("剛才"));
        assert!(!gazette.contains("剛剛"));
        assert!(!gazette.contains("方才"));

        let pid = "test_player_mailbox_user";
        {
            let mut mb = LIVE_ACQUAINTANCE_MAILBOX.write().unwrap_or_else(|e| e.into_inner());
            mb.remove(pid);
        }

        let (letter1, gift1) = claim_acquaintance_mailbox_letter(pid, "旅人小林", now);
        assert!(letter1.contains("【竹筒信箱・親善手札】"));
        assert!(gift1.is_some());
        assert!(!letter1.contains("剛才"));
        assert!(!letter1.contains("剛剛"));
        assert!(!letter1.contains("方才"));

        // Second claim in same window returns None for gift
        let (letter2, gift2) = claim_acquaintance_mailbox_letter(pid, "旅人小林", now + 10);
        assert!(letter2.contains("【竹筒信箱・親善手札】"));
        assert!(gift2.is_none());

        // Claim in next window returns Some gift
        let (_letter3, gift3) = claim_acquaintance_mailbox_letter(pid, "旅人小林", now + 301);
        assert!(gift3.is_some());
    }

    #[test]
    fn test_record_player_npc_fellowship_hook_and_mailbox() {
        let npc_id = "test_npc_chess_fellow";
        let pid = "test_player_fellow_01";
        let now = 1710000000;
        let summary = "曾在茶屋烏木棋秤前對弈手談，棋路相契";

        record_player_npc_fellowship_hook(npc_id, pid, summary, now);

        // 1. Verify in LIVE_NPC_CAUSAL_STATES
        {
            let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
            let state = states.get(npc_id).expect("npc state must exist");
            let hook = state.hooks.iter().find(|h| h.target_id == pid).expect("hook toward player must exist");
            assert_eq!(hook.intensity, 75);
            assert_eq!(hook.summary, summary);
            assert_eq!(hook.kind_label, "結識同行");
        }

        // 2. Verify letter queued in mailbox and can be claimed
        let (letter, gift) = claim_acquaintance_mailbox_letter(pid, pid, now);
        assert!(letter.contains("【竹筒信箱・親善手札】"));
        assert!(letter.contains(summary));
        assert!(gift.is_some());
        assert!(!letter.contains("剛才"));
        assert!(!letter.contains("剛剛"));
        assert!(!letter.contains("方才"));
    }

    #[test]
    fn test_round35_web_cache_version_sync_and_no_raw_socket_exposure() {
        test_frontend_sw_and_index_version_sync_v0_39_57();
    }

    #[test]
    fn test_frontend_sw_and_index_version_sync_v0_39_57() {
        let index_html = include_str!("../../web/index.html");
        let sw_js = include_str!("../../web/sw.js");
        let sw_obj_list = include_str!("../../web/sw-object-list.js");

        assert!(index_html.contains("?v=0.39.78"), "index.html should use v=0.39.78");
        assert!(index_html.contains("sw_reloaded_v0.39.78"), "index.html should use sw_reloaded_v0.39.78");
        assert!(sw_js.contains("sw-shell-v0.39.78"), "sw.js should use sw-shell-v0.39.78");
        // Verify all 10 new custom sockets have Chinese labels in sw-object-list.js
        let expected_sockets = [
            ("MillGrain", "水磨碾米"),
            ("PressCheese", "壓製乳酪"),
            ("SmokeFish", "慢火冷燻"),
            ("CutJade", "切磨寒玉"),
            ("HaulMineCart", "滑車運礦"),
            ("BurnLime", "入窯煅灰"),
            ("RowSwampRaft", "撐筏入澤"),
            ("PrepareMiasmaGear", "備妥避瘴"),
            ("PlayChess", "對弈手談"),
            ("SeasonalBanquet", "歲時圍爐"),
        ];

        for (sock, label) in expected_sockets {
            assert!(
                sw_obj_list.contains(sock),
                "sw-object-list.js missing socket: {sock}"
            );
            assert!(
                sw_obj_list.contains(label),
                "sw-object-list.js missing Chinese label: {label}"
            );
        }
    }

    #[test]
    fn test_sw_version_sync_and_custom_socket_chinese_labels() {
        test_frontend_sw_and_index_version_sync_v0_39_57();
    }

    #[test]
    fn test_round41_households_and_templates_guards() {
        let files = [
            ("src/store/households.rs", include_str!("../store/households.rs")),
            ("src/store/outer_households.rs", include_str!("../store/outer_households.rs")),
            ("src/game/peninsula_catalog.rs", include_str!("../game/peninsula_catalog.rs")),
            ("src/game/peninsula_scripts.rs", include_str!("../game/peninsula_scripts.rs")),
            (
                "src/game/player_scene.rs",
                include_str!("../game/player_scene.rs"),
            ),
            (
                "data/world/singularity_world_skeleton.json",
                include_str!("../../data/world/singularity_world_skeleton.json"),
            ),
            (
                "data/templates/dialogues/herbalist.json",
                include_str!("../../data/templates/dialogues/herbalist.json"),
            ),
            (
                "data/templates/dialogues/martial_artist.json",
                include_str!("../../data/templates/dialogues/martial_artist.json"),
            ),
            (
                "data/templates/dialogues/blacksmith.json",
                include_str!("../../data/templates/dialogues/blacksmith.json"),
            ),
            (
                "data/templates/dialogues/farmer.json",
                include_str!("../../data/templates/dialogues/farmer.json"),
            ),
            (
                "data/templates/dialogues/merchant.json",
                include_str!("../../data/templates/dialogues/merchant.json"),
            ),
        ];

        let forbidden = [
            "報備窯爐",
            "活版印書局",
            "正在長風草甸一帶正",
            "發霉",
            "防霉",
            "老少咸宜",
            "山里採集組",
            "衙門",
            "體適能夏令營",
            "仰臥起坐板",
            "防暴叉",
            "排班",
            "丹田",
            "神識",
            "百金",
            "十銀",
            "五十銅",
            "玉幣",
            "水泥",
            "防暴",
            "外帶",
            "數據",
            "系統",
        ];

        for (filename, content) in files {
            for word in forbidden {
                assert!(
                    !content.contains(word),
                    "{filename} contains forbidden word: {word}"
                );
            }
        }
    }

    #[test]
    fn test_round42_room_and_templates_no_xianxia_or_scifi_guards() {
        let files = [
            ("src/game/room.rs", include_str!("../game/room.rs")),
            ("src/store/rumor.rs", include_str!("../store/rumor.rs")),
            (
                "src/server/handler/movement.rs",
                include_str!("../server/handler/movement.rs"),
            ),
            ("web/world_atlas.html", include_str!("../../web/world_atlas.html")),
            (
                "web/behavior_mindmap.html",
                include_str!("../../web/behavior_mindmap.html"),
            ),
            (
                "data/config/gametext.json",
                include_str!("../../data/config/gametext.json"),
            ),
            (
                "data/config/word_elements.json",
                include_str!("../../data/config/word_elements.json"),
            ),
            (
                "data/npc_behaviors.json",
                include_str!("../../data/npc_behaviors.json"),
            ),
        ];

        let forbidden = [
            "神識",
            "丹田",
            "靈氣",
            "靈力",
            "排班",
            "鏢資",
            "江湖恩怨",
            "江湖宿怨",
            "門派捲入",
            "伏行巡獵、",
        ];

        for (filename, content) in files {
            for word in forbidden {
                assert!(
                    !content.contains(word),
                    "{filename} contains forbidden word: {word}"
                );
            }
        }
    }

    #[test]
    fn test_round43_disposition_catalog_and_atlas_guards() {
        let files = [
            (
                "src/db/disposition.rs",
                include_str!("../db/disposition.rs"),
            ),
            (
                "src/game/peninsula_catalog.rs",
                include_str!("../game/peninsula_catalog.rs"),
            ),
            (
                "src/store/households.rs",
                include_str!("../store/households.rs"),
            ),
            (
                "web/world_atlas.html",
                include_str!("../../web/world_atlas.html"),
            ),
            (
                "data/templates/dialogues/blacksmith.json",
                include_str!("../../data/templates/dialogues/blacksmith.json"),
            ),
            (
                "data/templates/dialogues/broker.json",
                include_str!("../../data/templates/dialogues/broker.json"),
            ),
            (
                "data/templates/dialogues/performer.json",
                include_str!("../../data/templates/dialogues/performer.json"),
            ),
            (
                "data/templates/archetypes.json",
                include_str!("../../data/templates/archetypes.json"),
            ),
            (
                "data/items.json",
                include_str!("../../data/items.json"),
            ),
            (
                "data/npc_behaviors.json",
                include_str!("../../data/npc_behaviors.json"),
            ),
        ];

        let forbidden = [
            "跟人說了會兒話",
            "神仙難擋",
            "重晶石晶片",
            "黑曜石晶片",
            "霜林鋸木大廠",
            "平價板料廠",
            "纜繩工務廠",
            "木藝器械廠",
            "尋常小怪",
            "次級掠食",
            "法術",
            "廢棄晶片",
            "采風",
        ];

        for (filename, content) in files {
            for word in forbidden {
                assert!(
                    !content.contains(word),
                    "{filename} contains forbidden word: {word}"
                );
            }
        }
    }

    #[test]
    fn test_round43_no_temporal_gang_or_fang_across_templates_and_catalog() {
        let files = [
            ("data/templates/dialogues/beggar.json", include_str!("../../data/templates/dialogues/beggar.json")),
            ("data/templates/dialogues/blacksmith.json", include_str!("../../data/templates/dialogues/blacksmith.json")),
            ("data/templates/dialogues/broker.json", include_str!("../../data/templates/dialogues/broker.json")),
            ("data/templates/dialogues/clerk.json", include_str!("../../data/templates/dialogues/clerk.json")),
            ("data/templates/dialogues/drunkard.json", include_str!("../../data/templates/dialogues/drunkard.json")),
            ("data/templates/dialogues/farmer.json", include_str!("../../data/templates/dialogues/farmer.json")),
            ("data/templates/dialogues/gatherer.json", include_str!("../../data/templates/dialogues/gatherer.json")),
            ("data/templates/dialogues/guard.json", include_str!("../../data/templates/dialogues/guard.json")),
            ("data/templates/dialogues/herbalist.json", include_str!("../../data/templates/dialogues/herbalist.json")),
            ("data/templates/dialogues/laborer.json", include_str!("../../data/templates/dialogues/laborer.json")),
            ("data/templates/dialogues/martial_artist.json", include_str!("../../data/templates/dialogues/martial_artist.json")),
            ("data/templates/dialogues/mechanic.json", include_str!("../../data/templates/dialogues/mechanic.json")),
            ("data/templates/dialogues/merchant.json", include_str!("../../data/templates/dialogues/merchant.json")),
            ("data/templates/dialogues/performer.json", include_str!("../../data/templates/dialogues/performer.json")),
            ("data/templates/dialogues/public_dialogue.json", include_str!("../../data/templates/dialogues/public_dialogue.json")),
            ("data/templates/dialogues/scholar.json", include_str!("../../data/templates/dialogues/scholar.json")),
            ("data/templates/dialogues/scout.json", include_str!("../../data/templates/dialogues/scout.json")),
            ("data/templates/dialogues/traveler.json", include_str!("../../data/templates/dialogues/traveler.json")),
            ("data/templates/dialogues/waiter.json", include_str!("../../data/templates/dialogues/waiter.json")),
            ("data/templates/behaviors/beggar.json", include_str!("../../data/templates/behaviors/beggar.json")),
            ("data/templates/behaviors/blacksmith.json", include_str!("../../data/templates/behaviors/blacksmith.json")),
            ("data/templates/behaviors/broker.json", include_str!("../../data/templates/behaviors/broker.json")),
            ("data/templates/behaviors/drunkard.json", include_str!("../../data/templates/behaviors/drunkard.json")),
            ("data/templates/behaviors/farmer.json", include_str!("../../data/templates/behaviors/farmer.json")),
            ("data/templates/behaviors/guard.json", include_str!("../../data/templates/behaviors/guard.json")),
            ("data/templates/behaviors/herbalist.json", include_str!("../../data/templates/behaviors/herbalist.json")),
            ("data/templates/behaviors/laborer.json", include_str!("../../data/templates/behaviors/laborer.json")),
            ("data/templates/behaviors/martial_artist.json", include_str!("../../data/templates/behaviors/martial_artist.json")),
            ("data/templates/behaviors/mechanic.json", include_str!("../../data/templates/behaviors/mechanic.json")),
            ("data/templates/behaviors/merchant.json", include_str!("../../data/templates/behaviors/merchant.json")),
            ("data/templates/behaviors/performer.json", include_str!("../../data/templates/behaviors/performer.json")),
            ("data/templates/behaviors/scholar.json", include_str!("../../data/templates/behaviors/scholar.json")),
            ("data/templates/behaviors/scout.json", include_str!("../../data/templates/behaviors/scout.json")),
            ("data/templates/behaviors/traveler.json", include_str!("../../data/templates/behaviors/traveler.json")),
            ("src/server/simulation_loop/idle.rs", include_str!("../server/simulation_loop/idle.rs")),
            ("src/pg/living_world.rs", include_str!("../pg/living_world.rs")),
            ("src/game/room.rs", include_str!("../game/room.rs")),
            ("src/game/peninsula_scripts.rs", include_str!("../game/peninsula_scripts.rs")),
            ("src/game/object_ecology.rs", include_str!("../game/object_ecology.rs")),
            ("src/store/entity.rs", include_str!("../store/entity.rs")),
            ("src/server/action/combat.rs", include_str!("../server/action/combat.rs")),
            ("src/server/action/talk.rs", include_str!("../server/action/talk.rs")),
            ("src/server/action/object.rs", include_str!("../server/action/object.rs")),
            ("src/server/handler/movement.rs", include_str!("../server/handler/movement.rs")),
            ("src/server/http_api/eco.rs", include_str!("../server/http_api/eco.rs")),
            ("src/server/grid_manager.rs", include_str!("../server/grid_manager.rs")),
            ("data/config/word_elements.json", include_str!("../../data/config/word_elements.json")),
            ("data/config/word_elements_gamedims.json", include_str!("../../data/config/word_elements_gamedims.json")),
            ("web/main.js", include_str!("../../web/main.js")),
            ("src/lib.rs", include_str!("../lib.rs")),
            ("src/main.rs", include_str!("../main.rs")),
            ("src/config/mod.rs", include_str!("../config/mod.rs")),
            ("src/server/action/borrow.rs", include_str!("../server/action/borrow.rs")),
            ("src/server/action/look.rs", include_str!("../server/action/look.rs")),
            ("src/server/action/mod.rs", include_str!("../server/action/mod.rs")),
            ("src/server/action/trade.rs", include_str!("../server/action/trade.rs")),
            ("src/server/handler/auth.rs", include_str!("../server/handler/auth.rs")),
            ("src/server/handler/inventory.rs", include_str!("../server/handler/inventory.rs")),
            ("src/server/handler/mod.rs", include_str!("../server/handler/mod.rs")),
            ("src/server/handler/query.rs", include_str!("../server/handler/query.rs")),
            ("src/server/simulation_loop/helpers.rs", include_str!("../server/simulation_loop/helpers.rs")),
            ("src/server/simulation_loop/market_economy.rs", include_str!("../server/simulation_loop/market_economy.rs")),
            ("src/server/simulation_loop/mod.rs", include_str!("../server/simulation_loop/mod.rs")),
            ("src/server/simulation_loop/movement.rs", include_str!("../server/simulation_loop/movement.rs")),
            ("src/server/simulation_loop/work.rs", include_str!("../server/simulation_loop/work.rs")),
            ("src/server/simulation_loop/world.rs", include_str!("../server/simulation_loop/world.rs")),
            ("src/server/http_api/admin.rs", include_str!("../server/http_api/admin.rs")),
            ("src/server/http_api/mod.rs", include_str!("../server/http_api/mod.rs")),
            ("src/server/http_api/player.rs", include_str!("../server/http_api/player.rs")),
            ("src/server/http_api/rooms.rs", include_str!("../server/http_api/rooms.rs")),
            ("src/npc/behavior.rs", include_str!("behavior.rs")),
            ("src/npc/brain_arrival.rs", include_str!("brain_arrival.rs")),
            ("src/npc/debt_interaction.rs", include_str!("debt_interaction.rs")),
            ("src/npc/decision.rs", include_str!("decision.rs")),
            ("src/npc/grid_ai.rs", include_str!("grid_ai.rs")),
            ("src/npc/job_matching.rs", include_str!("job_matching.rs")),
            ("src/npc/mod.rs", include_str!("mod.rs")),
            ("src/npc/narrative.rs", include_str!("narrative.rs")),
            ("src/npc/reaction.rs", include_str!("reaction.rs")),
            ("src/npc/social.rs", include_str!("social.rs")),
            ("src/npc/topics.rs", include_str!("topics.rs")),
            ("src/npc/traveler.rs", include_str!("traveler.rs")),
            ("src/npc/unobserved.rs", include_str!("unobserved.rs")),
            ("src/npc/grid_ai/activity.rs", include_str!("grid_ai/activity.rs")),
            ("src/npc/grid_ai/decision.rs", include_str!("grid_ai/decision.rs")),
            ("src/npc/grid_ai/journey.rs", include_str!("grid_ai/journey.rs")),
            ("src/npc/grid_ai/occupancy.rs", include_str!("grid_ai/occupancy.rs")),
            ("src/npc/grid_ai/tests.rs", include_str!("grid_ai/tests.rs")),
            ("src/npcnpc/mod.rs", include_str!("../npcnpc/mod.rs")),
            ("src/npcnpc/state.rs", include_str!("../npcnpc/state.rs")),
            ("src/npcnpc/trigger.rs", include_str!("../npcnpc/trigger.rs")),
            ("src/npcnpc/types.rs", include_str!("../npcnpc/types.rs")),
            ("src/game/peninsula_catalog.rs", include_str!("../game/peninsula_catalog.rs")),
            ("src/game/player_scene.rs", include_str!("../game/player_scene.rs")),
            ("src/game/chunk_view.rs", include_str!("../game/chunk_view.rs")),
            ("src/game/gametime.rs", include_str!("../game/gametime.rs")),
            ("src/game/mod.rs", include_str!("../game/mod.rs")),
            ("src/game/movement_tick.rs", include_str!("../game/movement_tick.rs")),
            ("src/game/observe.rs", include_str!("../game/observe.rs")),
            ("src/game/tick.rs", include_str!("../game/tick.rs")),
            ("src/game/view_sim.rs", include_str!("../game/view_sim.rs")),
            ("src/game/zone.rs", include_str!("../game/zone.rs")),
            ("src/store/households.rs", include_str!("../store/households.rs")),
            ("src/store/outer_households.rs", include_str!("../store/outer_households.rs")),
            ("src/store/archival.rs", include_str!("../store/archival.rs")),
            ("src/store/auth.rs", include_str!("../store/auth.rs")),
            ("src/store/event.rs", include_str!("../store/event.rs")),
            ("src/store/init_load.rs", include_str!("../store/init_load.rs")),
            ("src/store/item.rs", include_str!("../store/item.rs")),
            ("src/store/mod.rs", include_str!("../store/mod.rs")),
            ("src/store/npc_social.rs", include_str!("../store/npc_social.rs")),
            ("src/store/persist.rs", include_str!("../store/persist.rs")),
            ("src/store/pg_sync.rs", include_str!("../store/pg_sync.rs")),
            ("src/store/place.rs", include_str!("../store/place.rs")),
            ("src/store/room.rs", include_str!("../store/room.rs")),
            ("src/store/rumor.rs", include_str!("../store/rumor.rs")),
            ("src/store/sql.rs", include_str!("../store/sql.rs")),
            ("src/store/types.rs", include_str!("../store/types.rs")),
            ("src/store/venue.rs", include_str!("../store/venue.rs")),
            ("src/store/work.rs", include_str!("../store/work.rs")),
            ("src/combat/human_phase.rs", include_str!("../combat/human_phase.rs")),
            ("src/combat/mod.rs", include_str!("../combat/mod.rs")),
            ("src/combat/wordplate.rs", include_str!("../combat/wordplate.rs")),
            ("src/grid/cell.rs", include_str!("../grid/cell.rs")),
            ("src/grid/coord.rs", include_str!("../grid/coord.rs")),
            ("src/grid/grid.rs", include_str!("../grid/grid.rs")),
            ("src/grid/mod.rs", include_str!("../grid/mod.rs")),
            ("src/grid/village_template.rs", include_str!("../grid/village_template.rs")),
            ("src/grid/world_skeleton.rs", include_str!("../grid/world_skeleton.rs")),
            ("src/grid/gen.rs", include_str!("../grid/gen.rs")),
            ("src/grid/reveal.rs", include_str!("../grid/reveal.rs")),
            ("src/world_state.rs", include_str!("../world_state.rs")),
            ("src/world_state/sequence.rs", include_str!("../world_state/sequence.rs")),
            ("src/world_state/tests.rs", include_str!("../world_state/tests.rs")),
            ("data/config/npc_narrative.json", include_str!("../../data/config/npc_narrative.json")),
            ("data/config/terrain_ambience.json", include_str!("../../data/config/terrain_ambience.json")),
            ("data/world/singularity_world_skeleton.json", include_str!("../../data/world/singularity_world_skeleton.json")),
            ("src/world/author_economy.rs", include_str!("../world/author_economy.rs")),
            ("src/world/chunk.rs", include_str!("../world/chunk.rs")),
            ("src/world/journey.rs", include_str!("../world/journey.rs")),
            ("src/world/map.rs", include_str!("../world/map.rs")),
            ("src/world/materials.rs", include_str!("../world/materials.rs")),
            ("src/world/mod.rs", include_str!("../world/mod.rs")),
            ("src/world/movement.rs", include_str!("../world/movement.rs")),
            ("src/world/physical_geography.rs", include_str!("../world/physical_geography.rs")),
            ("src/world/physical_geography/conversion.rs", include_str!("../world/physical_geography/conversion.rs")),
            ("src/world/physical_geography/model.rs", include_str!("../world/physical_geography/model.rs")),
            ("src/world/physical_geography/model/hydrology.rs", include_str!("../world/physical_geography/model/hydrology.rs")),
            ("src/world/physical_geography/tests.rs", include_str!("../world/physical_geography/tests.rs")),
            ("src/world/physical_geography/validation.rs", include_str!("../world/physical_geography/validation.rs")),
            ("src/world/physical_geography/validation/anchors.rs", include_str!("../world/physical_geography/validation/anchors.rs")),
            ("src/world/physical_geography/validation/hydrology.rs", include_str!("../world/physical_geography/validation/hydrology.rs")),
            ("src/world/physical_geography/validation/hydrology/geometry.rs", include_str!("../world/physical_geography/validation/hydrology/geometry.rs")),
            ("src/world/physical_geography/validation/hydrology/relationships.rs", include_str!("../world/physical_geography/validation/hydrology/relationships.rs")),
            ("src/world/physical_geography/validation/land.rs", include_str!("../world/physical_geography/validation/land.rs")),
            ("src/world/physical_geography/validation/network.rs", include_str!("../world/physical_geography/validation/network.rs")),
            ("src/world/physical_geography/validation/profile.rs", include_str!("../world/physical_geography/validation/profile.rs")),
            ("src/world/place.rs", include_str!("../world/place.rs")),
            ("src/world/place/tests.rs", include_str!("../world/place/tests.rs")),
            ("src/world/runtime_world.rs", include_str!("../world/runtime_world.rs")),
            ("src/world/terrain_display.rs", include_str!("../world/terrain_display.rs")),
            ("src/world/topology.rs", include_str!("../world/topology.rs")),
            ("src/world/topology/tests.rs", include_str!("../world/topology/tests.rs")),
            ("src/ai/decision_scorer.rs", include_str!("../ai/decision_scorer.rs")),
            ("src/ai/mod.rs", include_str!("../ai/mod.rs")),
            ("src/ai/openai_chat.rs", include_str!("../ai/openai_chat.rs")),
            ("src/ai/prompts.rs", include_str!("../ai/prompts.rs")),
            ("src/ai/scorer.rs", include_str!("../ai/scorer.rs")),
            ("src/ai/talk.rs", include_str!("../ai/talk.rs")),
            ("src/economy/mod.rs", include_str!("../economy/mod.rs")),
            ("src/event/mod.rs", include_str!("../event/mod.rs")),
            ("src/model/mod.rs", include_str!("../model/mod.rs")),
            ("src/worldgen/earth.rs", include_str!("../worldgen/earth.rs")),
            ("src/worldgen/earth_places.rs", include_str!("../worldgen/earth_places.rs")),
            ("src/worldgen/mod.rs", include_str!("../worldgen/mod.rs")),
            ("src/server/broadcast.rs", include_str!("../server/broadcast.rs")),
            ("src/server/hub.rs", include_str!("../server/hub.rs")),
            ("src/server/m3_029.rs", include_str!("../server/m3_029.rs")),
            ("src/server/mod.rs", include_str!("../server/mod.rs")),
            ("src/server/protocol.rs", include_str!("../server/protocol.rs")),
            ("src/server/run.rs", include_str!("../server/run.rs")),
            ("src/server/session.rs", include_str!("../server/session.rs")),
            ("src/server/room_editor/handlers.rs", include_str!("../server/room_editor/handlers.rs")),
            ("src/server/room_editor/io.rs", include_str!("../server/room_editor/io.rs")),
            ("src/server/room_editor/mod.rs", include_str!("../server/room_editor/mod.rs")),
            ("src/server/room_editor/types.rs", include_str!("../server/room_editor/types.rs")),
            ("src/pg/auth.rs", include_str!("../pg/auth.rs")),
            ("src/pg/economy.rs", include_str!("../pg/economy.rs")),
            ("src/pg/entity.rs", include_str!("../pg/entity.rs")),
            ("src/pg/event.rs", include_str!("../pg/event.rs")),
            ("src/pg/mod.rs", include_str!("../pg/mod.rs")),
            ("src/pg/rumor.rs", include_str!("../pg/rumor.rs")),
            ("src/pg/white_tower_runtime.rs", include_str!("../pg/white_tower_runtime.rs")),
            ("src/pg/world_economy.rs", include_str!("../pg/world_economy.rs")),
            ("src/pg/world_journey.rs", include_str!("../pg/world_journey.rs")),
            ("src/pg/world_logistics.rs", include_str!("../pg/world_logistics.rs")),
            ("src/pg/world_market.rs", include_str!("../pg/world_market.rs")),
            ("src/pg/world_place.rs", include_str!("../pg/world_place.rs")),
            ("src/pg/world_place/grid_sync.rs", include_str!("../pg/world_place/grid_sync.rs")),
            ("src/pg/world_resource.rs", include_str!("../pg/world_resource.rs")),
            ("src/pg/world_supply.rs", include_str!("../pg/world_supply.rs")),
            ("src/pg/world_topology.rs", include_str!("../pg/world_topology.rs")),
            ("src/pg/world_trade.rs", include_str!("../pg/world_trade.rs")),
            ("src/pg/world_work.rs", include_str!("../pg/world_work.rs")),
            ("src/pg/writer.rs", include_str!("../pg/writer.rs")),
            ("src/db/acquaintance.rs", include_str!("../db/acquaintance.rs")),
            ("src/db/archival.rs", include_str!("../db/archival.rs")),
            ("src/db/assignment.rs", include_str!("../db/assignment.rs")),
            ("src/db/auth.rs", include_str!("../db/auth.rs")),
            ("src/db/climate.rs", include_str!("../db/climate.rs")),
            ("src/db/dialogue.rs", include_str!("../db/dialogue.rs")),
            ("src/db/disposition.rs", include_str!("../db/disposition.rs")),
            ("src/db/entity.rs", include_str!("../db/entity.rs")),
            ("src/db/equip.rs", include_str!("../db/equip.rs")),
            ("src/db/favorability.rs", include_str!("../db/favorability.rs")),
            ("src/db/grid_world.rs", include_str!("../db/grid_world.rs")),
            ("src/db/identity.rs", include_str!("../db/identity.rs")),
            ("src/db/inventory.rs", include_str!("../db/inventory.rs")),
            ("src/db/journey.rs", include_str!("../db/journey.rs")),
            ("src/db/lexicon.rs", include_str!("../db/lexicon.rs")),
            ("src/db/mod.rs", include_str!("../db/mod.rs")),
            ("src/db/npc_display.rs", include_str!("../db/npc_display.rs")),
            ("src/db/npc_events.rs", include_str!("../db/npc_events.rs")),
            ("src/db/npc_expense.rs", include_str!("../db/npc_expense.rs")),
            ("src/db/npc_names.rs", include_str!("../db/npc_names.rs")),
            ("src/db/npc_social.rs", include_str!("../db/npc_social.rs")),
            ("src/db/npc_spawn.rs", include_str!("../db/npc_spawn.rs")),
            ("src/db/npc_trade.rs", include_str!("../db/npc_trade.rs")),
            ("src/db/occupation.rs", include_str!("../db/occupation.rs")),
            ("src/db/room.rs", include_str!("../db/room.rs")),
            ("src/db/room_graph.rs", include_str!("../db/room_graph.rs")),
            ("src/db/room_object.rs", include_str!("../db/room_object.rs")),
            ("src/db/sched.rs", include_str!("../db/sched.rs")),
            ("src/db/soul_seed.rs", include_str!("../db/soul_seed.rs")),
            ("src/db/text.rs", include_str!("../db/text.rs")),
            ("src/db/trade_pending.rs", include_str!("../db/trade_pending.rs")),
            ("data/npc_behaviors.json", include_str!("../../data/npc_behaviors.json")),
            ("data/items.json", include_str!("../../data/items.json")),
            ("data/templates/archetypes.json", include_str!("../../data/templates/archetypes.json")),
            ("web/world_atlas.html", include_str!("../../web/world_atlas.html")),
            ("data/config/room_desc_templates.json", include_str!("../../data/config/room_desc_templates.json")),
            ("web/grid-map.js", include_str!("../../web/grid-map.js")),
            ("data/config/gametext.json", include_str!("../../data/config/gametext.json")),
            ("data/templates/dialogue_slots.json", include_str!("../../data/templates/dialogue_slots.json")),
            ("web/behavior_mindmap.html", include_str!("../../web/behavior_mindmap.html")),
            ("data/config/event_seeds.json", include_str!("../../data/config/event_seeds.json")),
            ("data/config/server_defaults.json", include_str!("../../data/config/server_defaults.json")),
            ("data/config/simulation.json", include_str!("../../data/config/simulation.json")),
            ("data/config/sentence_engine.json", include_str!("../../data/config/sentence_engine.json")),
            ("data/config/room_hex_overlay.json", include_str!("../../data/config/room_hex_overlay.json")),
            ("data/config/city_ambience.json", include_str!("../../data/config/city_ambience.json")),
            ("data/world/white_tower_living_world_v1.json", include_str!("../../data/world/white_tower_living_world_v1.json")),
            ("data/world/white_tower_topology_v1.json", include_str!("../../data/world/white_tower_topology_v1.json")),
            ("data/world/white_tower_peninsula_v1.json", include_str!("../../data/world/white_tower_peninsula_v1.json")),
            ("data/world/white_tower_author_economy_v1.json", include_str!("../../data/world/white_tower_author_economy_v1.json")),
            ("data/world/white_tower_east_coast_experience_v1.json", include_str!("../../data/world/white_tower_east_coast_experience_v1.json")),
            ("data/world/tradable_materials_p0.json", include_str!("../../data/world/tradable_materials_p0.json")),
            ("data/world/white_tower_runtime_v1.json", include_str!("../../data/world/white_tower_runtime_v1.json")),
            ("data/world/white_tower_physical_geography_v1.json", include_str!("../../data/world/white_tower_physical_geography_v1.json")),
            ("web/sw.js", include_str!("../../web/sw.js")),
            ("web/sw-panes.js", include_str!("../../web/sw-panes.js")),
            ("web/sw-object-list.js", include_str!("../../web/sw-object-list.js")),
            ("web/sw-room-desc.js", include_str!("../../web/sw-room-desc.js")),
            ("web/sw-clock.js", include_str!("../../web/sw-clock.js")),
            ("web/sw-interactions.js", include_str!("../../web/sw-interactions.js")),
            ("web/sw-modals.js", include_str!("../../web/sw-modals.js")),
            ("web/sw-log.js", include_str!("../../web/sw-log.js")),
            ("web/sw-narrative.js", include_str!("../../web/sw-narrative.js")),
            ("web/sw-auth.js", include_str!("../../web/sw-auth.js")),
            ("web/narrative-markdown.js", include_str!("../../web/narrative-markdown.js")),
            ("web/dashboard.js", include_str!("../../web/dashboard.js")),
            ("web/ui_mockups.js", include_str!("../../web/ui_mockups.js")),
            ("web/map_viewer.html", include_str!("../../web/map_viewer.html")),
            ("web/city_map.html", include_str!("../../web/city_map.html")),
            ("web/star_chart.html", include_str!("../../web/star_chart.html")),
            ("web/eco_viewer.html", include_str!("../../web/eco_viewer.html")),
            ("src/living_world.rs", include_str!("../living_world.rs")),
            ("src/roomcheck.rs", include_str!("../roomcheck.rs")),
            ("src/config/simulation.rs", include_str!("../config/simulation.rs")),
            ("src/server/conversation.rs", include_str!("../server/conversation.rs")),
            ("data/entities.json", include_str!("../../data/entities.json")),
            ("data/npc_to_npc_topics.json", include_str!("../../data/npc_to_npc_topics.json")),
            ("data/schedules.json", include_str!("../../data/schedules.json")),
            ("data/assignments.json", include_str!("../../data/assignments.json")),
            ("data/venues.json", include_str!("../../data/venues.json")),
            ("data/templates/dialogue_keywords.json", include_str!("../../data/templates/dialogue_keywords.json")),
            ("data/templates/occupations.json", include_str!("../../data/templates/occupations.json")),
            ("data/world/white_tower_formalization_v1.json", include_str!("../../data/world/white_tower_formalization_v1.json")),
            ("src/bin/checkrooms.rs", include_str!("../bin/checkrooms.rs")),
            ("src/bin/earth_elevation_preview.rs", include_str!("../bin/earth_elevation_preview.rs")),
            ("src/bin/earth_import_preview.rs", include_str!("../bin/earth_import_preview.rs")),
            ("src/bin/earth_import_sql.rs", include_str!("../bin/earth_import_sql.rs")),
            ("src/bin/earth_landuse_preview.rs", include_str!("../bin/earth_landuse_preview.rs")),
            ("src/bin/earth_places_preview.rs", include_str!("../bin/earth_places_preview.rs")),
            ("src/bin/generate_embeddings.rs", include_str!("../bin/generate_embeddings.rs")),
            ("src/bin/migrate.rs", include_str!("../bin/migrate.rs")),
            ("src/bin/sw-set-password.rs", include_str!("../bin/sw-set-password.rs")),
            ("src/bin/verify_embeddings.rs", include_str!("../bin/verify_embeddings.rs")),
            ("src/bin/white_tower_experience_check.rs", include_str!("../bin/white_tower_experience_check.rs")),
            ("src/bin/white_tower_formalize.rs", include_str!("../bin/white_tower_formalize.rs")),
            ("src/bin/white_tower_geography_check.rs", include_str!("../bin/white_tower_geography_check.rs")),
            ("src/bin/white_tower_living_world_check.rs", include_str!("../bin/white_tower_living_world_check.rs")),
            ("src/bin/white_tower_map_generate.rs", include_str!("../bin/white_tower_map_generate.rs")),
            ("src/bin/white_tower_package_check.rs", include_str!("../bin/white_tower_package_check.rs")),
            ("src/bin/white_tower_runtime_check.rs", include_str!("../bin/white_tower_runtime_check.rs")),
            ("src/bin/white_tower_runtime_compile.rs", include_str!("../bin/white_tower_runtime_compile.rs")),
            ("src/bin/white_tower_runtime_db.rs", include_str!("../bin/white_tower_runtime_db.rs")),
            ("src/bin/white_tower_world_check.rs", include_str!("../bin/white_tower_world_check.rs")),
            ("src/bin/worldgen_preview.rs", include_str!("../bin/worldgen_preview.rs")),
            ("src/bin/white_tower_experience_check/command.rs", include_str!("../bin/white_tower_experience_check/command.rs")),
            ("src/bin/white_tower_experience_check/evidence.rs", include_str!("../bin/white_tower_experience_check/evidence.rs")),
            ("src/bin/white_tower_experience_check/projection.rs", include_str!("../bin/white_tower_experience_check/projection.rs")),
            ("src/bin/white_tower_experience_check/receipt.rs", include_str!("../bin/white_tower_experience_check/receipt.rs")),
            ("src/bin/white_tower_experience_check/source.rs", include_str!("../bin/white_tower_experience_check/source.rs")),
            ("src/bin/white_tower_experience_check/validation.rs", include_str!("../bin/white_tower_experience_check/validation.rs")),
            ("src/bin/white_tower_experience_check/validation_tests.rs", include_str!("../bin/white_tower_experience_check/validation_tests.rs")),
            ("src/bin/white_tower_formalize/compile.rs", include_str!("../bin/white_tower_formalize/compile.rs")),
            ("src/bin/white_tower_formalize/layers.rs", include_str!("../bin/white_tower_formalize/layers.rs")),
            ("src/bin/white_tower_formalize/svg.rs", include_str!("../bin/white_tower_formalize/svg.rs")),
            ("src/bin/white_tower_map_generate/runtime.rs", include_str!("../bin/white_tower_map_generate/runtime.rs")),
            ("src/bin/white_tower_world_check/catalog_geo.rs", include_str!("../bin/white_tower_world_check/catalog_geo.rs")),
            ("src/bin/white_tower_world_check/catalog_people.rs", include_str!("../bin/white_tower_world_check/catalog_people.rs")),
            ("src/bin/white_tower_world_check/command.rs", include_str!("../bin/white_tower_world_check/command.rs")),
            ("src/bin/white_tower_world_check/policy.rs", include_str!("../bin/white_tower_world_check/policy.rs")),
            ("src/bin/white_tower_world_check/receipt.rs", include_str!("../bin/white_tower_world_check/receipt.rs")),
            ("src/bin/white_tower_world_check/refs.rs", include_str!("../bin/white_tower_world_check/refs.rs")),
            ("src/bin/white_tower_world_check/validation.rs", include_str!("../bin/white_tower_world_check/validation.rs")),
            ("src/bin/white_tower_world_check/validation_tests.rs", include_str!("../bin/white_tower_world_check/validation_tests.rs")),
            ("data/cities/32.json", include_str!("../../data/cities/32.json")),
            ("data/world/white_tower_peninsula_v1_d0_d7_receipt.json", include_str!("../../data/world/white_tower_peninsula_v1_d0_d7_receipt.json")),
            ("data/world/white_tower_east_coast_experience_v1_receipt.json", include_str!("../../data/world/white_tower_east_coast_experience_v1_receipt.json")),
            ("data/world/white_tower_azgaar_v1/manifest.json", include_str!("../../data/world/white_tower_azgaar_v1/manifest.json")),
            ("data/world/white_tower_azgaar_v1/white_tower_peninsula_v1_full.json", include_str!("../../data/world/white_tower_azgaar_v1/white_tower_peninsula_v1_full.json")),
            ("web/index.html", include_str!("../../web/index.html")),
            ("web/ui_mockups.html", include_str!("../../web/ui_mockups.html")),
            ("web/dashboard.html", include_str!("../../web/dashboard.html")),
            ("web/admin.html", include_str!("../../web/admin.html")),
            ("web/cities/32.json", include_str!("../../web/cities/32.json")),
            ("web/manifest.webmanifest", include_str!("../../web/manifest.webmanifest")),
            ("web/maps/tiles/0/manifest.json", include_str!("../../web/maps/tiles/0/manifest.json")),
            ("web/game.css", include_str!("../../web/game.css")),
            ("web/game-grid.css", include_str!("../../web/game-grid.css")),
            ("web/style.css", include_str!("../../web/style.css")),
            ("web/dashboard.css", include_str!("../../web/dashboard.css")),
            ("src/ai/narrative.rs", include_str!("../ai/narrative.rs")),
            ("src/bin/white_tower_economy_compile.rs", include_str!("../bin/white_tower_economy_compile.rs")),
            ("data/templates/README.md", include_str!("../../data/templates/README.md")),
        ];

        let allowed_gang = [
            "金剛", "剛勁", "剛強", "剛烈", "剛硬", "剛直", "剛正", "剛毅", "剛猛", "剛健", "剛柔", "剛巧", "剛好", "粗剛", "堅剛", "剛性", "剛度", "剛鬃", "剛韌度", "青剛櫟", "剛韌", "剛玉砂", "以柔化剛", "柔中帶剛", "以巧破剛", "金剛砂", "青剛赤鐵",
            "\"char\": \"剛\"", "以柔克剛",
        ];

        let allowed_fang = [
            "一方方", "配方表", "正方格", "方格網", "方解石", "地方", "方向", "官方", "方位", "方圓", "雙方",
            "對方", "藥方", "秘方", "祕方", "配方", "四方", "方寸", "方言", "方正", "方塊", "一方",
            "各方", "大方", "遠方", "方形", "丹方", "驗方", "多方", "單方", "偏方", "平方", "立方",
            "方桌", "方巾", "方舟", "方士", "賣方", "買方", "方物", "方便", "方法", "方框", "千方百計",
            "來日方長", "方圓百里", "東方", "西方", "南方", "北方", "八方", "方陣", "方臉", "方格", "方磚",
            "方案", "方志", "比方", "方量", "上方", "下方", "後方", "前方", "偏遠地方", "偏僻地方", "某方",
            "他方", "這方", "那方", "古方", "良方", "見笑大方", "大方之家", "游方", "遊方", "方丈", "方休",
            "方劑", "方程式", "方程", "方術", "方子", "塌方", "千金方", "方式", "大方地", "軍方", "湯方",
            "金創方", "方誌", "方石", "方赤土", "方小石", "開方", "男方", "女方", "何方", "端方", "方料",
            "方家", "方氏", "此方", "方隅", "按方", "照方", "關防", "敵方", "攻方", "守方", "甲方",
            "乙方", "正西方", "西北方", "呼叫方", "多方", "方針", "坍方", "方木", "漢方", "方型", "石方",
            "方池", "方碑", "方樁", "三方", "方通", "整方", "諸方", "密方", "勝出方", "厚重方", "巧勁方",
            "碾壓方", "重擊方", "百方", "東北方", "傅方白", "進攻方", "防守方", "勝方", "敗方",
            "\"char\": \"方\"", "\"display_char\": \"方\"",
            "方佳欣", "方佳穎", "方依婷", "方依晨", "方冠良", "方品妤", "方啟銘", "方妤涵",
            "方宥彤", "方庭瑄", "方廷宇", "方彥良", "方彩薇", "方心怡", "方映辰", "方映雪",
            "方晴雯", "方淑惠", "方清和", "方秀英", "方秋月", "方筱涵", "方美玲", "方芷嫣",
            "方芷瑜", "方芷蘭", "方詠晴", "方語彤", "方雅筑", "方靜雯", "方韻如",
        ];
        for (filename, content) in files {
            assert!(!content.contains("剛才"), "{filename} contains forbidden: 剛才");
            assert!(!content.contains("剛剛"), "{filename} contains forbidden: 剛剛");
            assert!(!content.contains("方才"), "{filename} contains forbidden: 方才");
            assert!(!content.contains("適才"), "{filename} contains forbidden: 適才");

            // Check any remaining 剛
            let mut remaining_gang = content.to_string();
            for word in allowed_gang {
                remaining_gang = remaining_gang.replace(word, "");
            }
            assert!(!remaining_gang.contains('剛'), "{filename} contains temporal 剛");

            // Check any remaining 方
            let mut remaining_fang = content.to_string();
            for word in allowed_fang {
                remaining_fang = remaining_fang.replace(word, "");
            }
            assert!(!remaining_fang.contains('方'), "{filename} contains temporal 方");
        }
    }

    #[test]
    fn test_round44_no_modern_tools_or_western_magic_guards() {
        let files = [
            ("src/game/peninsula_catalog.rs", include_str!("../game/peninsula_catalog.rs")),
            ("src/game/peninsula_scripts.rs", include_str!("../game/peninsula_scripts.rs")),
            ("src/game/player_scene.rs", include_str!("../game/player_scene.rs")),
            ("src/store/households.rs", include_str!("../store/households.rs")),
            ("src/store/outer_households.rs", include_str!("../store/outer_households.rs")),
            ("src/grid/gen.rs", include_str!("../grid/gen.rs")),
            ("src/grid/reveal.rs", include_str!("../grid/reveal.rs")),
            ("data/npc_behaviors.json", include_str!("../../data/npc_behaviors.json")),
            ("data/items.json", include_str!("../../data/items.json")),
            ("data/templates/archetypes.json", include_str!("../../data/templates/archetypes.json")),
            ("web/world_atlas.html", include_str!("../../web/world_atlas.html")),
            ("data/templates/dialogues/beggar.json", include_str!("../../data/templates/dialogues/beggar.json")),
            ("data/templates/dialogues/blacksmith.json", include_str!("../../data/templates/dialogues/blacksmith.json")),
            ("data/templates/dialogues/broker.json", include_str!("../../data/templates/dialogues/broker.json")),
            ("data/templates/dialogues/clerk.json", include_str!("../../data/templates/dialogues/clerk.json")),
            ("data/templates/dialogues/drunkard.json", include_str!("../../data/templates/dialogues/drunkard.json")),
            ("data/templates/dialogues/farmer.json", include_str!("../../data/templates/dialogues/farmer.json")),
            ("data/templates/dialogues/gatherer.json", include_str!("../../data/templates/dialogues/gatherer.json")),
            ("data/templates/dialogues/guard.json", include_str!("../../data/templates/dialogues/guard.json")),
            ("data/templates/dialogues/herbalist.json", include_str!("../../data/templates/dialogues/herbalist.json")),
            ("data/templates/dialogues/laborer.json", include_str!("../../data/templates/dialogues/laborer.json")),
            ("data/templates/dialogues/mechanic.json", include_str!("../../data/templates/dialogues/mechanic.json")),
            ("data/templates/dialogues/merchant.json", include_str!("../../data/templates/dialogues/merchant.json")),
            ("data/templates/dialogues/performer.json", include_str!("../../data/templates/dialogues/performer.json")),
            ("data/templates/dialogues/public_dialogue.json", include_str!("../../data/templates/dialogues/public_dialogue.json")),
            ("data/templates/dialogues/scholar.json", include_str!("../../data/templates/dialogues/scholar.json")),
            ("data/templates/dialogues/scout.json", include_str!("../../data/templates/dialogues/scout.json")),
            ("data/templates/dialogues/traveler.json", include_str!("../../data/templates/dialogues/traveler.json")),
            ("data/templates/dialogues/martial_artist.json", include_str!("../../data/templates/dialogues/martial_artist.json")),
            ("data/templates/dialogues/waiter.json", include_str!("../../data/templates/dialogues/waiter.json")),
            ("data/templates/behaviors/beggar.json", include_str!("../../data/templates/behaviors/beggar.json")),
            ("data/templates/behaviors/blacksmith.json", include_str!("../../data/templates/behaviors/blacksmith.json")),
            ("data/templates/behaviors/broker.json", include_str!("../../data/templates/behaviors/broker.json")),
            ("data/templates/behaviors/drunkard.json", include_str!("../../data/templates/behaviors/drunkard.json")),
            ("data/templates/behaviors/farmer.json", include_str!("../../data/templates/behaviors/farmer.json")),
            ("data/templates/behaviors/guard.json", include_str!("../../data/templates/behaviors/guard.json")),
            ("data/templates/behaviors/herbalist.json", include_str!("../../data/templates/behaviors/herbalist.json")),
            ("data/templates/behaviors/laborer.json", include_str!("../../data/templates/behaviors/laborer.json")),
            ("data/templates/behaviors/martial_artist.json", include_str!("../../data/templates/behaviors/martial_artist.json")),
            ("data/templates/behaviors/mechanic.json", include_str!("../../data/templates/behaviors/mechanic.json")),
            ("data/templates/behaviors/merchant.json", include_str!("../../data/templates/behaviors/merchant.json")),
            ("data/templates/behaviors/performer.json", include_str!("../../data/templates/behaviors/performer.json")),
            ("data/templates/behaviors/scholar.json", include_str!("../../data/templates/behaviors/scholar.json")),
            ("data/templates/behaviors/scout.json", include_str!("../../data/templates/behaviors/scout.json")),
            ("data/templates/behaviors/traveler.json", include_str!("../../data/templates/behaviors/traveler.json")),
            ("data/config/room_desc_templates.json", include_str!("../../data/config/room_desc_templates.json")),
            ("data/config/terrain_ambience.json", include_str!("../../data/config/terrain_ambience.json")),
            ("web/grid-map.js", include_str!("../../web/grid-map.js")),
            ("src/game/room.rs", include_str!("../game/room.rs")),
            ("src/server/simulation_loop/idle.rs", include_str!("../server/simulation_loop/idle.rs")),
            ("src/npc/brain_arrival.rs", include_str!("brain_arrival.rs")),
            ("data/config/gametext.json", include_str!("../../data/config/gametext.json")),
            ("data/templates/dialogue_slots.json", include_str!("../../data/templates/dialogue_slots.json")),
            ("src/pg/living_world.rs", include_str!("../pg/living_world.rs")),
            ("src/game/object_ecology.rs", include_str!("../game/object_ecology.rs")),
            ("src/combat/wordplate.rs", include_str!("../combat/wordplate.rs")),
            ("web/behavior_mindmap.html", include_str!("../../web/behavior_mindmap.html")),
            ("src/store/entity.rs", include_str!("../store/entity.rs")),
            ("src/server/action/combat.rs", include_str!("../server/action/combat.rs")),
            ("src/server/action/talk.rs", include_str!("../server/action/talk.rs")),
            ("src/server/action/object.rs", include_str!("../server/action/object.rs")),
            ("src/server/handler/inventory.rs", include_str!("../server/handler/inventory.rs")),
            ("src/server/handler/movement.rs", include_str!("../server/handler/movement.rs")),
            ("src/server/http_api/eco.rs", include_str!("../server/http_api/eco.rs")),
            ("src/server/http_api/player.rs", include_str!("../server/http_api/player.rs")),
            ("src/server/grid_manager.rs", include_str!("../server/grid_manager.rs")),
            ("data/config/word_elements.json", include_str!("../../data/config/word_elements.json")),
            ("data/config/word_elements_gamedims.json", include_str!("../../data/config/word_elements_gamedims.json")),
            ("data/config/event_seeds.json", include_str!("../../data/config/event_seeds.json")),
            ("data/config/npc_narrative.json", include_str!("../../data/config/npc_narrative.json")),
            ("data/config/server_defaults.json", include_str!("../../data/config/server_defaults.json")),
            ("data/config/simulation.json", include_str!("../../data/config/simulation.json")),
            ("data/config/sentence_engine.json", include_str!("../../data/config/sentence_engine.json")),
            ("data/config/room_hex_overlay.json", include_str!("../../data/config/room_hex_overlay.json")),
            ("data/config/city_ambience.json", include_str!("../../data/config/city_ambience.json")),
            ("data/world/singularity_world_skeleton.json", include_str!("../../data/world/singularity_world_skeleton.json")),
            ("data/world/white_tower_living_world_v1.json", include_str!("../../data/world/white_tower_living_world_v1.json")),
            ("data/world/white_tower_topology_v1.json", include_str!("../../data/world/white_tower_topology_v1.json")),
            ("data/world/white_tower_peninsula_v1.json", include_str!("../../data/world/white_tower_peninsula_v1.json")),
            ("data/world/white_tower_author_economy_v1.json", include_str!("../../data/world/white_tower_author_economy_v1.json")),
            ("data/world/white_tower_east_coast_experience_v1.json", include_str!("../../data/world/white_tower_east_coast_experience_v1.json")),
            ("data/world/tradable_materials_p0.json", include_str!("../../data/world/tradable_materials_p0.json")),
            ("data/world/white_tower_runtime_v1.json", include_str!("../../data/world/white_tower_runtime_v1.json")),
            ("data/world/white_tower_physical_geography_v1.json", include_str!("../../data/world/white_tower_physical_geography_v1.json")),
            ("web/sw.js", include_str!("../../web/sw.js")),
            ("web/sw-panes.js", include_str!("../../web/sw-panes.js")),
            ("web/sw-object-list.js", include_str!("../../web/sw-object-list.js")),
            ("web/main.js", include_str!("../../web/main.js")),
            ("web/sw-room-desc.js", include_str!("../../web/sw-room-desc.js")),
            ("web/sw-clock.js", include_str!("../../web/sw-clock.js")),
            ("web/sw-interactions.js", include_str!("../../web/sw-interactions.js")),
            ("web/sw-modals.js", include_str!("../../web/sw-modals.js")),
            ("web/sw-log.js", include_str!("../../web/sw-log.js")),
            ("web/sw-narrative.js", include_str!("../../web/sw-narrative.js")),
            ("web/sw-auth.js", include_str!("../../web/sw-auth.js")),
            ("web/narrative-markdown.js", include_str!("../../web/narrative-markdown.js")),
            ("web/dashboard.js", include_str!("../../web/dashboard.js")),
            ("web/ui_mockups.js", include_str!("../../web/ui_mockups.js")),
            ("web/map_viewer.html", include_str!("../../web/map_viewer.html")),
            ("web/city_map.html", include_str!("../../web/city_map.html")),
            ("web/star_chart.html", include_str!("../../web/star_chart.html")),
            ("src/lib.rs", include_str!("../lib.rs")),
            ("src/main.rs", include_str!("../main.rs")),
            ("src/config/mod.rs", include_str!("../config/mod.rs")),
            ("src/server/action/borrow.rs", include_str!("../server/action/borrow.rs")),
            ("src/server/action/look.rs", include_str!("../server/action/look.rs")),
            ("src/server/action/mod.rs", include_str!("../server/action/mod.rs")),
            ("src/server/action/trade.rs", include_str!("../server/action/trade.rs")),
            ("src/server/handler/auth.rs", include_str!("../server/handler/auth.rs")),
            ("src/server/handler/mod.rs", include_str!("../server/handler/mod.rs")),
            ("src/server/handler/query.rs", include_str!("../server/handler/query.rs")),
            ("src/server/simulation_loop/helpers.rs", include_str!("../server/simulation_loop/helpers.rs")),
            ("src/server/simulation_loop/market_economy.rs", include_str!("../server/simulation_loop/market_economy.rs")),
            ("src/server/simulation_loop/mod.rs", include_str!("../server/simulation_loop/mod.rs")),
            ("src/server/simulation_loop/movement.rs", include_str!("../server/simulation_loop/movement.rs")),
            ("src/server/simulation_loop/work.rs", include_str!("../server/simulation_loop/work.rs")),
            ("src/server/simulation_loop/world.rs", include_str!("../server/simulation_loop/world.rs")),
            ("src/server/http_api/admin.rs", include_str!("../server/http_api/admin.rs")),
            ("src/server/http_api/mod.rs", include_str!("../server/http_api/mod.rs")),
            ("src/server/http_api/rooms.rs", include_str!("../server/http_api/rooms.rs")),
            ("src/npc/behavior.rs", include_str!("behavior.rs")),
            ("src/npc/debt_interaction.rs", include_str!("debt_interaction.rs")),
            ("src/npc/decision.rs", include_str!("decision.rs")),
            ("src/npc/grid_ai.rs", include_str!("grid_ai.rs")),
            ("src/npc/job_matching.rs", include_str!("job_matching.rs")),
            ("src/npc/mod.rs", include_str!("mod.rs")),
            ("src/npc/narrative.rs", include_str!("narrative.rs")),
            ("src/npc/reaction.rs", include_str!("reaction.rs")),
            ("src/npc/social.rs", include_str!("social.rs")),
            ("src/npc/topics.rs", include_str!("topics.rs")),
            ("src/npc/traveler.rs", include_str!("traveler.rs")),
            ("src/npc/unobserved.rs", include_str!("unobserved.rs")),
            ("src/npc/grid_ai/activity.rs", include_str!("grid_ai/activity.rs")),
            ("src/npc/grid_ai/decision.rs", include_str!("grid_ai/decision.rs")),
            ("src/npc/grid_ai/journey.rs", include_str!("grid_ai/journey.rs")),
            ("src/npc/grid_ai/occupancy.rs", include_str!("grid_ai/occupancy.rs")),
            ("src/npc/grid_ai/tests.rs", include_str!("grid_ai/tests.rs")),
            ("src/npcnpc/mod.rs", include_str!("../npcnpc/mod.rs")),
            ("src/npcnpc/state.rs", include_str!("../npcnpc/state.rs")),
            ("src/npcnpc/trigger.rs", include_str!("../npcnpc/trigger.rs")),
            ("src/npcnpc/types.rs", include_str!("../npcnpc/types.rs")),
            ("src/game/chunk_view.rs", include_str!("../game/chunk_view.rs")),
            ("src/game/gametime.rs", include_str!("../game/gametime.rs")),
            ("src/game/mod.rs", include_str!("../game/mod.rs")),
            ("src/game/movement_tick.rs", include_str!("../game/movement_tick.rs")),
            ("src/game/observe.rs", include_str!("../game/observe.rs")),
            ("src/game/tick.rs", include_str!("../game/tick.rs")),
            ("src/game/view_sim.rs", include_str!("../game/view_sim.rs")),
            ("src/game/zone.rs", include_str!("../game/zone.rs")),
            ("src/store/archival.rs", include_str!("../store/archival.rs")),
            ("src/store/auth.rs", include_str!("../store/auth.rs")),
            ("src/store/event.rs", include_str!("../store/event.rs")),
            ("src/store/init_load.rs", include_str!("../store/init_load.rs")),
            ("src/store/item.rs", include_str!("../store/item.rs")),
            ("src/store/mod.rs", include_str!("../store/mod.rs")),
            ("src/store/npc_social.rs", include_str!("../store/npc_social.rs")),
            ("src/store/persist.rs", include_str!("../store/persist.rs")),
            ("src/store/pg_sync.rs", include_str!("../store/pg_sync.rs")),
            ("src/store/place.rs", include_str!("../store/place.rs")),
            ("src/store/room.rs", include_str!("../store/room.rs")),
            ("src/store/rumor.rs", include_str!("../store/rumor.rs")),
            ("src/store/sql.rs", include_str!("../store/sql.rs")),
            ("src/store/types.rs", include_str!("../store/types.rs")),
            ("src/store/venue.rs", include_str!("../store/venue.rs")),
            ("src/store/work.rs", include_str!("../store/work.rs")),
            ("src/combat/human_phase.rs", include_str!("../combat/human_phase.rs")),
            ("src/combat/mod.rs", include_str!("../combat/mod.rs")),
            ("src/grid/cell.rs", include_str!("../grid/cell.rs")),
            ("src/grid/coord.rs", include_str!("../grid/coord.rs")),
            ("src/grid/grid.rs", include_str!("../grid/grid.rs")),
            ("src/grid/mod.rs", include_str!("../grid/mod.rs")),
            ("src/grid/village_template.rs", include_str!("../grid/village_template.rs")),
            ("src/grid/world_skeleton.rs", include_str!("../grid/world_skeleton.rs")),
            ("src/world_state.rs", include_str!("../world_state.rs")),
            ("src/world_state/sequence.rs", include_str!("../world_state/sequence.rs")),
            ("src/world_state/tests.rs", include_str!("../world_state/tests.rs")),
            ("src/db/acquaintance.rs", include_str!("../db/acquaintance.rs")),
            ("src/db/archival.rs", include_str!("../db/archival.rs")),
            ("src/db/assignment.rs", include_str!("../db/assignment.rs")),
            ("src/db/auth.rs", include_str!("../db/auth.rs")),
            ("src/db/climate.rs", include_str!("../db/climate.rs")),
            ("src/db/dialogue.rs", include_str!("../db/dialogue.rs")),
            ("src/db/disposition.rs", include_str!("../db/disposition.rs")),
            ("src/db/entity.rs", include_str!("../db/entity.rs")),
            ("src/db/equip.rs", include_str!("../db/equip.rs")),
            ("src/db/favorability.rs", include_str!("../db/favorability.rs")),
            ("src/db/grid_world.rs", include_str!("../db/grid_world.rs")),
            ("src/db/identity.rs", include_str!("../db/identity.rs")),
            ("src/db/inventory.rs", include_str!("../db/inventory.rs")),
            ("src/db/journey.rs", include_str!("../db/journey.rs")),
            ("src/db/lexicon.rs", include_str!("../db/lexicon.rs")),
            ("src/db/mod.rs", include_str!("../db/mod.rs")),
            ("src/db/npc_display.rs", include_str!("../db/npc_display.rs")),
            ("src/db/npc_events.rs", include_str!("../db/npc_events.rs")),
            ("src/db/npc_expense.rs", include_str!("../db/npc_expense.rs")),
            ("src/db/npc_names.rs", include_str!("../db/npc_names.rs")),
            ("src/db/npc_social.rs", include_str!("../db/npc_social.rs")),
            ("src/db/npc_spawn.rs", include_str!("../db/npc_spawn.rs")),
            ("src/db/npc_trade.rs", include_str!("../db/npc_trade.rs")),
            ("src/db/occupation.rs", include_str!("../db/occupation.rs")),
            ("src/db/room.rs", include_str!("../db/room.rs")),
            ("src/db/room_graph.rs", include_str!("../db/room_graph.rs")),
            ("src/db/room_object.rs", include_str!("../db/room_object.rs")),
            ("src/db/sched.rs", include_str!("../db/sched.rs")),
            ("src/db/soul_seed.rs", include_str!("../db/soul_seed.rs")),
            ("src/db/text.rs", include_str!("../db/text.rs")),
            ("src/db/trade_pending.rs", include_str!("../db/trade_pending.rs")),
            ("src/pg/auth.rs", include_str!("../pg/auth.rs")),
            ("src/pg/economy.rs", include_str!("../pg/economy.rs")),
            ("src/pg/entity.rs", include_str!("../pg/entity.rs")),
            ("src/pg/event.rs", include_str!("../pg/event.rs")),
            ("src/pg/mod.rs", include_str!("../pg/mod.rs")),
            ("src/pg/rumor.rs", include_str!("../pg/rumor.rs")),
            ("src/pg/white_tower_runtime.rs", include_str!("../pg/white_tower_runtime.rs")),
            ("src/pg/world_economy.rs", include_str!("../pg/world_economy.rs")),
            ("src/pg/world_journey.rs", include_str!("../pg/world_journey.rs")),
            ("src/pg/world_logistics.rs", include_str!("../pg/world_logistics.rs")),
            ("src/pg/world_market.rs", include_str!("../pg/world_market.rs")),
            ("src/pg/world_place.rs", include_str!("../pg/world_place.rs")),
            ("src/pg/world_place/grid_sync.rs", include_str!("../pg/world_place/grid_sync.rs")),
            ("src/pg/world_resource.rs", include_str!("../pg/world_resource.rs")),
            ("src/pg/world_supply.rs", include_str!("../pg/world_supply.rs")),
            ("src/pg/world_topology.rs", include_str!("../pg/world_topology.rs")),
            ("src/pg/world_trade.rs", include_str!("../pg/world_trade.rs")),
            ("src/pg/world_work.rs", include_str!("../pg/world_work.rs")),
            ("src/pg/writer.rs", include_str!("../pg/writer.rs")),
            ("web/eco_viewer.html", include_str!("../../web/eco_viewer.html")),
            ("src/world/author_economy.rs", include_str!("../world/author_economy.rs")),
            ("src/world/chunk.rs", include_str!("../world/chunk.rs")),
            ("src/world/journey.rs", include_str!("../world/journey.rs")),
            ("src/world/map.rs", include_str!("../world/map.rs")),
            ("src/world/materials.rs", include_str!("../world/materials.rs")),
            ("src/world/mod.rs", include_str!("../world/mod.rs")),
            ("src/world/movement.rs", include_str!("../world/movement.rs")),
            ("src/world/physical_geography.rs", include_str!("../world/physical_geography.rs")),
            ("src/world/physical_geography/conversion.rs", include_str!("../world/physical_geography/conversion.rs")),
            ("src/world/physical_geography/model.rs", include_str!("../world/physical_geography/model.rs")),
            ("src/world/physical_geography/model/hydrology.rs", include_str!("../world/physical_geography/model/hydrology.rs")),
            ("src/world/physical_geography/tests.rs", include_str!("../world/physical_geography/tests.rs")),
            ("src/world/physical_geography/validation.rs", include_str!("../world/physical_geography/validation.rs")),
            ("src/world/physical_geography/validation/anchors.rs", include_str!("../world/physical_geography/validation/anchors.rs")),
            ("src/world/physical_geography/validation/hydrology.rs", include_str!("../world/physical_geography/validation/hydrology.rs")),
            ("src/world/physical_geography/validation/hydrology/geometry.rs", include_str!("../world/physical_geography/validation/hydrology/geometry.rs")),
            ("src/world/physical_geography/validation/hydrology/relationships.rs", include_str!("../world/physical_geography/validation/hydrology/relationships.rs")),
            ("src/world/physical_geography/validation/land.rs", include_str!("../world/physical_geography/validation/land.rs")),
            ("src/world/physical_geography/validation/network.rs", include_str!("../world/physical_geography/validation/network.rs")),
            ("src/world/physical_geography/validation/profile.rs", include_str!("../world/physical_geography/validation/profile.rs")),
            ("src/world/place.rs", include_str!("../world/place.rs")),
            ("src/world/place/tests.rs", include_str!("../world/place/tests.rs")),
            ("src/world/runtime_world.rs", include_str!("../world/runtime_world.rs")),
            ("src/world/terrain_display.rs", include_str!("../world/terrain_display.rs")),
            ("src/world/topology.rs", include_str!("../world/topology.rs")),
            ("src/world/topology/tests.rs", include_str!("../world/topology/tests.rs")),
            ("src/ai/decision_scorer.rs", include_str!("../ai/decision_scorer.rs")),
            ("src/ai/mod.rs", include_str!("../ai/mod.rs")),
            ("src/ai/openai_chat.rs", include_str!("../ai/openai_chat.rs")),
            ("src/ai/prompts.rs", include_str!("../ai/prompts.rs")),
            ("src/ai/scorer.rs", include_str!("../ai/scorer.rs")),
            ("src/ai/talk.rs", include_str!("../ai/talk.rs")),
            ("src/economy/mod.rs", include_str!("../economy/mod.rs")),
            ("src/event/mod.rs", include_str!("../event/mod.rs")),
            ("src/model/mod.rs", include_str!("../model/mod.rs")),
            ("src/worldgen/earth.rs", include_str!("../worldgen/earth.rs")),
            ("src/worldgen/earth_places.rs", include_str!("../worldgen/earth_places.rs")),
            ("src/worldgen/mod.rs", include_str!("../worldgen/mod.rs")),
            ("src/server/broadcast.rs", include_str!("../server/broadcast.rs")),
            ("src/server/hub.rs", include_str!("../server/hub.rs")),
            ("src/server/m3_029.rs", include_str!("../server/m3_029.rs")),
            ("src/server/mod.rs", include_str!("../server/mod.rs")),
            ("src/server/protocol.rs", include_str!("../server/protocol.rs")),
            ("src/server/run.rs", include_str!("../server/run.rs")),
            ("src/server/session.rs", include_str!("../server/session.rs")),
            ("src/server/room_editor/handlers.rs", include_str!("../server/room_editor/handlers.rs")),
            ("src/server/room_editor/io.rs", include_str!("../server/room_editor/io.rs")),
            ("src/server/room_editor/mod.rs", include_str!("../server/room_editor/mod.rs")),
            ("src/server/room_editor/types.rs", include_str!("../server/room_editor/types.rs")),
            ("src/living_world.rs", include_str!("../living_world.rs")),
            ("src/roomcheck.rs", include_str!("../roomcheck.rs")),
            ("src/config/simulation.rs", include_str!("../config/simulation.rs")),
            ("src/server/conversation.rs", include_str!("../server/conversation.rs")),
            ("data/entities.json", include_str!("../../data/entities.json")),
            ("data/npc_to_npc_topics.json", include_str!("../../data/npc_to_npc_topics.json")),
            ("data/schedules.json", include_str!("../../data/schedules.json")),
            ("data/assignments.json", include_str!("../../data/assignments.json")),
            ("data/venues.json", include_str!("../../data/venues.json")),
            ("data/templates/dialogue_keywords.json", include_str!("../../data/templates/dialogue_keywords.json")),
            ("data/templates/occupations.json", include_str!("../../data/templates/occupations.json")),
            ("data/world/white_tower_formalization_v1.json", include_str!("../../data/world/white_tower_formalization_v1.json")),
            ("src/bin/checkrooms.rs", include_str!("../bin/checkrooms.rs")),
            ("src/bin/earth_elevation_preview.rs", include_str!("../bin/earth_elevation_preview.rs")),
            ("src/bin/earth_import_preview.rs", include_str!("../bin/earth_import_preview.rs")),
            ("src/bin/earth_import_sql.rs", include_str!("../bin/earth_import_sql.rs")),
            ("src/bin/earth_landuse_preview.rs", include_str!("../bin/earth_landuse_preview.rs")),
            ("src/bin/earth_places_preview.rs", include_str!("../bin/earth_places_preview.rs")),
            ("src/bin/generate_embeddings.rs", include_str!("../bin/generate_embeddings.rs")),
            ("src/bin/migrate.rs", include_str!("../bin/migrate.rs")),
            ("src/bin/sw-set-password.rs", include_str!("../bin/sw-set-password.rs")),
            ("src/bin/verify_embeddings.rs", include_str!("../bin/verify_embeddings.rs")),
            ("src/bin/white_tower_experience_check.rs", include_str!("../bin/white_tower_experience_check.rs")),
            ("src/bin/white_tower_formalize.rs", include_str!("../bin/white_tower_formalize.rs")),
            ("src/bin/white_tower_geography_check.rs", include_str!("../bin/white_tower_geography_check.rs")),
            ("src/bin/white_tower_living_world_check.rs", include_str!("../bin/white_tower_living_world_check.rs")),
            ("src/bin/white_tower_map_generate.rs", include_str!("../bin/white_tower_map_generate.rs")),
            ("src/bin/white_tower_package_check.rs", include_str!("../bin/white_tower_package_check.rs")),
            ("src/bin/white_tower_runtime_check.rs", include_str!("../bin/white_tower_runtime_check.rs")),
            ("src/bin/white_tower_runtime_compile.rs", include_str!("../bin/white_tower_runtime_compile.rs")),
            ("src/bin/white_tower_runtime_db.rs", include_str!("../bin/white_tower_runtime_db.rs")),
            ("src/bin/white_tower_world_check.rs", include_str!("../bin/white_tower_world_check.rs")),
            ("src/bin/worldgen_preview.rs", include_str!("../bin/worldgen_preview.rs")),
            ("src/bin/white_tower_experience_check/command.rs", include_str!("../bin/white_tower_experience_check/command.rs")),
            ("src/bin/white_tower_experience_check/evidence.rs", include_str!("../bin/white_tower_experience_check/evidence.rs")),
            ("src/bin/white_tower_experience_check/projection.rs", include_str!("../bin/white_tower_experience_check/projection.rs")),
            ("src/bin/white_tower_experience_check/receipt.rs", include_str!("../bin/white_tower_experience_check/receipt.rs")),
            ("src/bin/white_tower_experience_check/source.rs", include_str!("../bin/white_tower_experience_check/source.rs")),
            ("src/bin/white_tower_experience_check/validation.rs", include_str!("../bin/white_tower_experience_check/validation.rs")),
            ("src/bin/white_tower_experience_check/validation_tests.rs", include_str!("../bin/white_tower_experience_check/validation_tests.rs")),
            ("src/bin/white_tower_formalize/compile.rs", include_str!("../bin/white_tower_formalize/compile.rs")),
            ("src/bin/white_tower_formalize/layers.rs", include_str!("../bin/white_tower_formalize/layers.rs")),
            ("src/bin/white_tower_formalize/svg.rs", include_str!("../bin/white_tower_formalize/svg.rs")),
            ("src/bin/white_tower_map_generate/runtime.rs", include_str!("../bin/white_tower_map_generate/runtime.rs")),
            ("src/bin/white_tower_world_check/catalog_geo.rs", include_str!("../bin/white_tower_world_check/catalog_geo.rs")),
            ("src/bin/white_tower_world_check/catalog_people.rs", include_str!("../bin/white_tower_world_check/catalog_people.rs")),
            ("src/bin/white_tower_world_check/command.rs", include_str!("../bin/white_tower_world_check/command.rs")),
            ("src/bin/white_tower_world_check/policy.rs", include_str!("../bin/white_tower_world_check/policy.rs")),
            ("src/bin/white_tower_world_check/receipt.rs", include_str!("../bin/white_tower_world_check/receipt.rs")),
            ("src/bin/white_tower_world_check/refs.rs", include_str!("../bin/white_tower_world_check/refs.rs")),
            ("src/bin/white_tower_world_check/validation.rs", include_str!("../bin/white_tower_world_check/validation.rs")),
            ("src/bin/white_tower_world_check/validation_tests.rs", include_str!("../bin/white_tower_world_check/validation_tests.rs")),
            ("data/cities/32.json", include_str!("../../data/cities/32.json")),
            ("data/world/white_tower_peninsula_v1_d0_d7_receipt.json", include_str!("../../data/world/white_tower_peninsula_v1_d0_d7_receipt.json")),
            ("data/world/white_tower_east_coast_experience_v1_receipt.json", include_str!("../../data/world/white_tower_east_coast_experience_v1_receipt.json")),
            ("data/world/white_tower_azgaar_v1/manifest.json", include_str!("../../data/world/white_tower_azgaar_v1/manifest.json")),
            ("data/world/white_tower_azgaar_v1/white_tower_peninsula_v1_full.json", include_str!("../../data/world/white_tower_azgaar_v1/white_tower_peninsula_v1_full.json")),
            ("web/index.html", include_str!("../../web/index.html")),
            ("web/ui_mockups.html", include_str!("../../web/ui_mockups.html")),
            ("web/dashboard.html", include_str!("../../web/dashboard.html")),
            ("web/admin.html", include_str!("../../web/admin.html")),
            ("web/cities/32.json", include_str!("../../web/cities/32.json")),
            ("web/manifest.webmanifest", include_str!("../../web/manifest.webmanifest")),
            ("web/maps/tiles/0/manifest.json", include_str!("../../web/maps/tiles/0/manifest.json")),
            ("web/game.css", include_str!("../../web/game.css")),
            ("web/game-grid.css", include_str!("../../web/game-grid.css")),
            ("web/style.css", include_str!("../../web/style.css")),
            ("web/dashboard.css", include_str!("../../web/dashboard.css")),
            ("src/ai/narrative.rs", include_str!("../ai/narrative.rs")),
            ("src/bin/white_tower_economy_compile.rs", include_str!("../bin/white_tower_economy_compile.rs")),
            ("data/templates/README.md", include_str!("../../data/templates/README.md")),
        ];

        let forbidden = [
            "// 補足",
            "（偷了乞丐的台詞）",
            "（算了，勞工還是淳樸點）",
            "【{name】",
            "【{name}」",
            "螺絲",
            "扳手",
            "化學",
            "牛里脊",
            "大木作梁架居",
            "藥香蒸汽",
            "蒸汽棒",
            "咖啡",
            "不銹鋼",
            "不鏽鋼",
            "尼龍",
            "少林寺",
            "舍利子",
            "瘋人院",
            "奇點神教",
            "奇點世界",
            "魔法",
            "法杖",
            "牧師",
            "教堂",
            "鈔票",
            "五十金",
            "錢幣",
            "官東家",
            "城東家",
            "土東家",
            "貴族東家",
            "長途跋涉半日長途",
            "刻度水位刻度",
            "瑪芬堡",
            "磨豆機",
            "鎂鈔",
            "客官",
            "江湖",
            "掌櫃",
            "老爺",
            "修煉",
            "靈石",
            "螺帽",
            "調試",
            "\u{810f}",
            "重機械油",
            "大圳頭・大圳頭望野樓",
            "導能翻土犁工坊舍",
            "安保",
            "生髮水",
            "跳蚤市場",
            "盲盒",
            "金屬拖把",
            "戶外用品店",
            "運動鞋墊",
            "行腳醫徒",
            "法師",
            "術士",
            "秘銀",
            "馬卡龍",
            "雞尾酒",
            "撲克牌",
            "塔羅牌",
            "銅瓣",
            "优质",
            "钐",
            "钎",
            "版權費",
            "蕗易沙",
            "收銀",
            "燈箱",
            "小票",
            "酒肆",
            "伙計",
            "奇點",
            "靈流",
            "靈能",
            "陣法",
            "陣紋",
            "結界",
            "白塔神山",
            "庄頭內當家",
            "廢墟",
            "絕緣導線",
            "抗毒血清",
            "操蛋",
            "妓院",
            "性價比",
            "擴音石",
            "蓄能石槽",
            "傭兵公會",
            "破勢九劍",
            "撩妹",
            "虱子",
            "麻痹",
            "白痴",
            "正骨大夫所",
            "方便大夫",
            "微型銅繞線發電機",
            "實心防刺橡膠輪",
            "鎢鋼",
            "玻璃瓶",
            "玻璃櫥窗",
            "鞋底的橡膠",
            "靈壓閥",
            "異獸晶核",
            "報廢陣盤",
            "高級法寶",
            "回氣丹",
            "氣血丹丸",
            "體育器材店",
            "速度反應球",
            "減脂丸",
            "營養標示",
            "金屬握力器",
            "運動防護員",
            "運動科學期刊",
            "重訓腰帶",
            "硬拉腰帶",
            "人面蜘蛛",
            "吸血鬼領地",
            "冰霜蜘蛛絲",
            "狂暴藥劑",
            "狂化藥劑",
            "重影防禦符",
            "能量水晶核",
            "姑孃們",
            "制毫",
            "制印",
            "風物志稿",
            "書筋活血散",
            "橡膠練習短棍",
            "重壺鈴",
            "高蛋白精力湯",
            "厚橡膠靶",
            "懸掛的輪胎",
            "防身甩棍",
            "烤雞胸肉便當",
            "槓鈴鎖扣",
            "私人司機",
            "體育場",
            "監視器",
            "獅鷲",
            "半島風物圖志",
            "虱目魚",
            "哨所手術室",
            "白內障手術",
            "熬藥夥計",
            "篩土夥計",
            "值守夥計",
            "替小姐送交稿件",
            "江湖大義",
            "銅子",
            "塊錢",
            "子兒",
            "一文",
            "分錢",
            "現錢",
            "現金",
            "酸雨",
            "實驗室",
            "飛石研究所",
            "圖書館",
            "難民營",
            "貧民窟",
            "法寶",
            "法陣",
            "符文",
            "符紋",
            "靈脈",
            "走火入魔",
            "地精",
            "巨龍",
            "國王",
            "王國",
            "官府",
            "傭兵",
            "僱傭兵",
            "盜墓",
            "古墓",
            "他媽",
            "老子",
            "滾蛋",
            "該死",
            "見鬼",
            "酒館",
            "酒精",
            "麵包",
            "紅酒",
            "雪茄",
            "罐頭",
            "違禁品",
            "紅酒般",
            "剛過篩",
            "外科學會室",
            "大圳消毒所",
            "細砂過濾消毒水缸",
            "雙重密碼旋盤",
            "旋轉金屬密碼盤",
            "黃芪",
            "鍛煉",
            "竹筏扎作",
            "扎實",
            "水文志",
            "鍛造秘笈",
            "麵攤老闆娘",
            "外場服務員",
            "茶點服務員",
            "浴場服務員",
            "通兌手續費",
            "灘岸編扎社",
            "剛在活水磨石",
            "剛運抵",
            "白塔山海志",
            "宋元地志",
            "定製",
            "深扎",
            "死扎",
            "沉扎",
            "扎馬步",
            "生之奇點",
            "一只只",
            "方自",
            "聽才從北山",
            "替才從水尾寮",
            "菜才從田裡",
            "才出西門",
            "像有人才走過去",
            "現在才被太陽碰到",
            "等船隊列",
            "隊列與佈局",
            "管路接口",
            "扎堆",
            "針扎",
            "扎手",
            "將目前內存中",
            "才走過去的胖商人",
            "竊聽器",
            "地脈晶核",
            "隱身斗篷",
            "免費廣播",
            "反偷窺神器",
            "熱力公式",
            "邏輯鏈",
            "都市規劃圖",
            "批量印刷品",
            "感應儀器",
            "擴音器",
            "啞光塗層",
            "記憶水滴",
            "致幻劑",
            "魔能信號發射器",
            "地下水道地雷",
            "財務赤字",
            "做空他們",
            "致幻孢子",
            "攻略地圖",
            "原廠貨",
            "老闆",
            "城主",
            "政客",
            "軍閥",
            "賞金獵人",
            "通緝犯",
            "暗影公會",
            "公會負責人",
            "垃圾桶",
            "下水道",
            "平民窟",
            "假證件",
            "檢查證件",
            "貼滿了傳單",
            "貼傳單",
            "破產跳樓",
            "性格參數",
            "交易邏輯",
            "光學刻紋",
            "單片鏡",
            "望遠鏡頭",
            "強力腐蝕劑",
            "手銬",
            "魔能水力",
            "啟動咒語",
            "護身咒語",
            "念咒語",
            "神啊，",
            "李太白",
            "『悟空』",
            "如來神掌",
            "觀音菩薩",
            "神農雕像",
            "神農草本抄",
            "天空之城",
            "遺忘之城",
            "極樂城",
            "鋼鐵之都",
            "不夜城",
            "絕望冰原",
            "飛霜湖",
            "候渡隊列",
            "旗石廣場的隊列",
            "隊列裡",
            "隊列折角",
            "北岸隊列",
            "被隊列認住",
            "門前隊列",
            "旗桿下的隊列",
            "隊列木牌",
            "鎮守司辰司辰",
            "地排污暗溝",
            "排污暗溝暗渠",
            "行行好行行好",
            "響箭響箭",
            "小的的",
            "店員來嘍",
            "狡猾的狡黠",
            "乾硬的乾糧硬餅",
            "乾硬的乾烙餅屑",
            "殘酒酒氣",
            "我們這這種人",
            "那只骨瘦如柴",
            "那只瘸腿",
            "那只受傷",
            "半只空了",
            "那只猴子",
            "合金鋼鑚",
            "復盤",
            "保質期",
            "蹭空調",
            "電纜",
            "擴音桶",
            "絕緣層",
            "回收站",
            "二型動力核心",
            "一次性干擾器",
            "計算單元",
            "垃圾焚化爐",
            "解瘴噴劑",
            "舊時代食品廠",
            "輕身符",
            "能量回路",
            "能量守恆定律",
            "強迫症",
            "被割的韭菜",
            "火靈符",
            "鐵匠工會",
            "銘文師",
            "盜賊工會",
            "木元素",
            "上品靈膏",
            "紅色的按鈕",
            "壓縮乾糧",
            "再生藥劑",
            "隊列",
            "大爺",
            "小朋友",
            "幫叔叔",
            "給叔叔",
            "賣給叔叔",
            "叔叔我",
            "叔叔手裡",
            "在酒店",
            "促銷",
            "折扣得大一點",
            "換季折扣",
            "打八折",
            "酒水八折",
            "八折轉讓",
            "八折束脩",
            "兩折起",
            "甩賣",
            "清倉",
            "掛在客廳",
            "客廳的大多數",
            "對著天空祈禱",
            "對它祈禱",
            "對天上祈禱",
            "口中祈禱",
            "向醫藥之神祈禱",
            "祈禱太陽",
            "祈禱的對象",
            "默默祈禱",
            "祈禱今天",
            "祈禱的方向",
            "金屬元素",
            "最神聖的",
            "神聖感",
            "防護服",
            "消炎藥",
            "退燒藥",
            "三十九度",
            "治貧血",
            "發酵果汁",
            "捲菸",
            "捲煙",
            "城裡人用的香皂",
            "自動播種機",
            "除蟲劑",
            "來塊肥皂",
            "護手霜",
            "膠鞋",
            "土豆",
            "指甲刀",
            "拆遷",
            "古堡",
            "保護費",
            "治安維護費",
            "交學費",
            "十枚鎂幣學費",
            "學費只要",
            "长官",
            "才歇下",
            "才慢條斯理",
            "才從某場",
            "才被雨水洗過",
            "像是才說過",
            "才初開始",
            "地底地脈",
            "抽一成抽成",
            "祝官司辰",
            "地頭的地氣",
            "大型重型木牛",
            "鐵甲臂甲",
            "這座城的城防",
            "巡防營長途巡防",
            "精鋼精鐵腕扣",
            "高高在上的高門",
            "草藥旱煙草卷",
            "最烈的烈火酒",
            "地底下的地脈",
            "機樞機密",
            "新運抵的新鮮",
            "異化異獸",
            "巨大的大石磨",
            "推演脈絡推理",
            "遠古古蹟",
            "衝擊攻擊",
            "窖藏藏在",
            "防瘴辟毒的辟瘴",
            "驅趕異獸的驅獸石",
            "蠍毒的解毒",
            "風乾的乾烙餅",
            "南方南林山寨",
            "秘製的『秘製",
            "猛獸獸膽",
            "穿上了華麗的法袍",
            "黃金戒指",
            "防禦力極高",
            "需專業人員操作",
            "強力麻醉劑",
            "在廣場中央廣播",
            "包月優惠價",
            "客廳",
            "香皂",
            "肥皂",
            "亮銀玻璃鏡",
            "長官",
            "大叔",
            "美女",
            "給力",
            "靠譜",
            "忽悠",
            "老頭子",
            "冰棍",
            "烤箱",
            "政府",
            "紅浪漫",
            "跳樓價",
            "職業性的微笑",
            "維度頻率",
            "儀表的槽位",
            "小姐姐",
            "小哥哥",
            "帥哥",
            "阿姨",
            "小鬼",
            "擺爛",
            "內卷",
            "躺平",
            "打工人",
            "社畜",
            "混蛋",
            "王八蛋",
            "他媽的",
            "媽的",
            "臥槽",
            "牛逼",
            "幹部",
            "同志",
            "部門",
            "分部",
            "書記",
            "局長",
            "科長",
            "總監",
            "董事長",
            "總裁",
            "客服",
            "保安",
            "物業",
            "業主",
            "房東",
            "租客",
            "開發商",
            "股票",
            "基金",
            "貸款",
            "銀行",
            "信用卡",
            "支票",
            "發票",
            "違約金",
            "賠償金",
            "醫保",
            "退休金",
            "養老金",
            "獎金",
            "提成",
            "薪水",
            "底薪",
            "時薪",
            "月薪",
            "年薪",
            "超市",
            "商場",
            "百貨",
            "專賣店",
            "連鎖店",
            "便利店",
            "外賣",
            "包郵",
            // Round 56: 61 newly cleaned modern/slang/medical/sci-fi/workplace terms
            "主婦", "代辦清單", "代金券", "保修單", "倒閉", "兄弟們", "八卦周刊", "冰淇淋",
            "前任", "剝線鉗", "十合一", "半機械", "原始碼", "周刊", "呼吸機", "啤酒",
            "失戀", "學霸", "客戶", "小弟", "急診", "情感諮詢", "打蛋器", "投資",
            "接盤", "控制模塊", "敗家", "文件夾", "暴發戶", "橡皮", "止痛藥", "汽水",
            "洗白翻身", "測試模塊", "熵增定律", "燈泡", "牛排", "特殊服務", "病房", "發動機",
            "眼鏡", "社死", "福音", "種草", "職業形象", "菜鳥", "萬能膠", "虛擬景象",
            "融資", "被甩", "調酒器材", "諮詢費", "豆腐渣工程", "辦事攻略", "退貨", "醫生",
            "鉛筆", "防割手套", "領帶", "風險自負", "香水", "齒輪箱",
            // Round 56: 101 zero-hit modern/colloquial/workplace/medical/everyday-object guards
            "搞定", "沒毛病", "哥們", "姐們", "大媽", "土豪", "大款", "剁手",
            "拔草", "奇葩", "高大上", "接地氣", "正能量", "負能量", "顏值", "人設",
            "腦洞", "甩鍋", "背鍋", "躺贏", "開掛", "大神", "學渣", "小妹",
            "打卡", "請假", "曠工", "遲到", "開除", "辭職", "跳槽", "面試",
            "簡歷", "履歷", "試用期", "消費者", "投訴", "差評", "換貨", "包退",
            "包換", "假一賠十", "零售", "加盟", "招商", "股份", "招標", "墨鏡",
            "手錶", "鬧鐘", "圓珠筆", "相片", "相框", "海報", "傳單", "廣告",
            "霓虹", "電池", "電線", "馬達", "改錐", "手電", "打火機", "香煙",
            "香菸", "煙頭", "煙灰缸", "香檳", "威士忌", "伏特加", "可樂", "雪糕",
            "巧克力", "蛋糕", "薯條", "漢堡", "三明治", "披薩", "沙拉", "西裝",
            "T恤", "牛仔褲", "運動鞋", "高跟鞋", "絲襪", "內褲", "泳衣", "毛衣",
            "口罩", "創可貼", "碘酒", "消毒水", "感冒藥", "安眠藥", "維生素", "抗生素",
            "打針", "輸液", "掛號", "門診", "護士", "手術",
            // Round 57 新增：全面封堵殘留現代日用品、大陸流行網梗、現代醫療臨床、3C家電與現代商業促銷詞（218 項）
            "拖鞋", "火柴", "起子", "鋼筆", "風衣", "黑板", "分紅", "合同", "放大鏡", "吐槽",
            "簽約", "競標", "看守所", "高鐵", "淋浴", "沐浴露", "牙刷", "毛巾架", "濕巾", "平底鍋",
            "面膜", "做大做強", "再創輝煌", "一分也是愛", "倍兒有面子", "掉鏈子", "標題黨", "防坑指南", "避坑指南", "拉練",
            "搬磚", "歡迎光臨", "熵增", "超頻運轉", "死循環", "晶圓", "物理降溫", "神經毒素", "強酸炸彈", "拳擊場",
            "調查局", "城防軍", "副司令", "雙面間諜", "透視眼", "能量代謝率", "軍工級別", "抗疲勞", "八音箱", "傻大黑粗",
            "太極拳", "職業素養", "併發症", "造雪機", "職業病", "社會階層", "空間幾何", "論文集", "專業術語", "微環境",
            "經濟概論", "下鄉調研", "學術期刊", "反偵察", "信號火箭", "量產貨", "買到就是賺到", "熱能盒飯", "氣泡水", "抵用券",
            "錦鯉特飲", "藍莓之戀", "攞貨", "防水組件", "狗血", "反套路", "轉正", "名片", "照片", "派出所",
            "警察局", "公安局", "法院", "律師", "法官", "檢察官", "監獄", "火車站", "地鐵站", "公交車",
            "計程車", "出租車", "飛機", "機場", "高速公路", "紅綠燈", "斑馬線", "加油站", "停車場", "電梯",
            "空調", "冰箱", "洗衣機", "微波爐", "電磁爐", "熱水器", "抽水馬桶", "水龍頭", "浴缸", "洗髮水",
            "牙膏", "衛生紙", "餐巾紙", "紙巾", "垃圾袋", "保鮮膜", "鋁箔紙", "不沾鍋", "電飯煲", "榨汁機",
            "攪拌機", "吹風機", "剃鬚刀", "刮鬍刀", "指甲油", "粉底", "眼影", "睫毛膏", "防曬霜", "爽膚水",
            "乳液", "精華液", "筆記本電腦", "平板電腦", "充電寶", "充電器", "數據線", "傳感器", "路由器", "攝像頭",
            "投影儀", "遙控器", "鍵盤", "鼠標", "硬盤", "液晶屏", "藍牙", "二維碼", "掃碼", "點贊",
            "朋友圈", "自媒體", "網紅", "直播間", "粉絲群", "短視頻", "互聯網", "防火牆", "防毒軟件", "點滴",
            "救護車", "血壓計", "聽診器", "核磁共振", "處方單", "輸液架", "靜脈注射", "消炎針", "麻醉藥", "止咳水",
            "眼藥水", "羽絨服", "衛衣", "西裝褲", "連衣裙", "百褶裙", "安全褲", "棒球帽", "太陽鏡", "拉鏈",
            "魔術貼", "火腿腸", "方便麵", "泡麵", "熱狗", "爆米花", "洋芋片", "果凍", "甜甜圈", "番茄醬",
            "口香糖", "立交橋", "人行天橋", "地下通道", "電話亭", "消防栓", "減速帶", "輕軌", "摩托車", "自行車",
            "滑板車", "飲水機", "淨水器", "油煙機", "洗碗機", "烘乾機", "除濕機", "加濕器", "吸塵器", "電風扇",
            "電熱毯", "卸妝油", "洗面奶", "眼霜", "身體乳", "潤膚露", "護髮素", "染髮劑", "修眉刀",
            // Round 58 新增：全面封堵迷信符咒、現代促銷折扣、軍警重械、現代載具、金融企業部門與當代市政文憑詞（225 項）
            "護身符", "平安符", "符紙", "算命", "賭場", "買一送一", "買二送一", "九折", "五折", "概不退換",
            "出門不退換", "帳單", "罰款", "舉報", "稅務官", "炸彈", "大明星", "動物明星", "休假", "少俠",
            "輻射", "老總", "奴才", "基因", "克隆", "病毒", "細菌", "網吧", "夜店", "蛋白質",
            "卡路里", "減肥", "瑜伽", "激光", "聲納", "雷達", "潛艇", "坦克", "導彈", "手雷",
            "槍支", "子彈", "毒品", "營業執照", "吊銷", "迪廳", "KTV", "蹦迪", "七折", "偶像",
            "模特", "畫報", "雜誌", "小費", "處長", "青樓", "鏢局", "毛錢", "知府",
            "縣令", "女俠", "大俠", "看相", "直升機", "戰鬥機", "轟炸機", "航母", "航空母艦", "宇宙飛船",
            "太空船", "外星人", "飛碟", "光年", "黑洞", "蟲洞", "反物質", "暗物質", "核電站", "原子彈",
            "氫彈", "中子彈", "生化武器", "衝鋒槍", "機關槍", "狙擊槍", "手槍", "步槍", "霰彈槍", "防彈衣",
            "電警棍", "催淚瓦斯", "煙霧彈", "閃光彈", "定時炸彈", "遙控炸彈", "火箭筒", "迫擊砲", "裝甲車", "越野車",
            "卡車", "貨櫃車", "推土機", "挖掘機", "起重機", "壓路機", "拖拉機", "收割機", "救生艇", "遊艇",
            "郵輪", "破冰船", "電視機", "收音機", "錄音機", "留聲機", "照相機", "攝影機", "顯微鏡", "計算機",
            "打印機", "複印機", "傳真機", "掃描儀", "碎紙機", "打卡鐘", "考勤機", "收銀機", "驗鈔機", "點鈔機",
            "自動售貨機", "提款機", "銀行卡", "儲蓄卡", "支票簿", "債券", "外匯", "匯率", "通貨膨脹", "宏觀經濟",
            "微觀經濟", "國內生產總值", "董事會", "股東大會", "執行官", "總經理", "副總經理", "財務總監", "人事部", "公關部",
            "市場部", "銷售部", "研發部", "客服部", "法務部", "行政部", "後勤部", "保安部", "物業費", "水電費",
            "煤氣費", "電話費", "網費", "房貸", "車貸", "高利貸", "信用卡債", "破產保護", "知識產權", "專利權",
            "商標權", "代言費", "廣告費", "贊助商", "冠名商", "加盟店", "旗艦店", "百貨大樓", "購物中心", "步行街",
            "遊樂園", "動物園", "植物園", "水族館", "博物館", "美術館", "電影院", "歌劇院", "體育館", "游泳池",
            "健身房", "美容院", "理髮店", "乾洗店", "洗車行", "修車廠", "火葬場", "公墓", "養老院", "孤兒院",
            "幼兒園", "高中", "大學", "研究生", "博士生", "教授", "校長", "班主任", "輔導員", "學生會",
            "高考", "中考", "期末考", "畢業證", "學位證", "學生證", "身分證", "護照", "簽證", "駕照",
            "行駛證", "車牌號", "郵政編碼", "電話號碼", "手機號碼",
            // Round 59 新增：浸泡日誌根因詞彙、報紙菜單收據摺、主任上下班、大洋走私魔術與當代網路/3C/醫療/服飾/餐飲/物流詞（207 項，累計 1,346 項）
            "氧化", "刺猬", "過個週", "縮工時", "本官", "八折", "報紙", "菜單", "夾背式三聯手寫收據摺", "工資",
            "加班", "下班", "上班", "主任", "大洋", "走私", "地雷", "火箭", "炸藥", "核能",
            "健身", "魔術", "神棍", "鬼畫符",
            "吃瓜", "腦補", "逆襲", "打臉", "氪金", "充值", "抽卡", "歐皇", "非酋", "肝帝",
            "小鮮肉", "老司機", "吃貨", "閨蜜", "基友", "渣男", "綠茶婊", "白蓮花", "高富帥", "白富美",
            "矮窮挫", "拜金", "炫富", "凡爾賽", "尷尬癌", "割韭菜", "抑鬱症", "焦慮症", "精神分裂", "心理醫生",
            "心理諮詢", "黑粉", "真愛粉", "水軍", "帶節奏", "熱搜", "流量明星", "打榜", "控評", "塌房",
            "智能手機", "觸摸屏", "內存條", "主板", "顯卡", "聲卡", "網卡", "光纖", "寬帶", "無線網",
            "局域網", "服務器", "數據庫", "雲計算", "大數據", "人工智能", "機器學習", "神經網絡", "區塊鏈", "比特幣",
            "虛擬貨幣", "加密貨幣", "元宇宙", "全息投影", "虛擬現實", "增強現實", "無人機", "機械臂", "激光器", "反應堆",
            "太陽能板", "風力發電機", "變壓器", "發電機組", "高壓電線", "電線桿", "信號塔", "衛星天線", "光學雷達",
            "手術室", "無影燈", "除顫儀", "心電圖", "超聲波", "X光機", "CT機", "化驗單", "體檢表", "病歷本",
            "醫保卡", "掛號處", "急診室", "重症監護室", "門診部", "住院部", "護士站", "輸液瓶", "注射器", "手術刀",
            "醫用酒精", "OK繃", "止痛片", "維他命", "保健品", "心肺復甦", "人工呼吸", "洗胃", "胃鏡", "腸鏡",
            "骨髓移植", "器官移植", "基因工程", "親子鑑定",
            "球鞋", "內衣褲", "比基尼", "泳褲", "蝴蝶結", "鴨舌帽", "防曬衣", "衝鋒衣", "保暖內衣", "雪紡衫",
            "迷你裙", "超短裙", "緊身褲", "打底褲", "馬丁靴", "帆布鞋", "人字拖",
            "意大利麵", "炸雞塊", "蛋黃醬", "奶昔", "聖代", "提拉米蘇", "芝士蛋糕", "慕斯蛋糕", "可頌", "白蘭地",
            "朗姆酒", "龍舌蘭酒", "純淨水", "碳酸飲料", "雪碧", "芬達", "運動飲料", "功能飲料", "速溶咖啡", "卡布奇諾",
            "摩卡", "美式咖啡", "奶茶店", "快餐店", "自助餐", "火鍋店", "燒烤攤",
            "外賣員", "快遞員", "快遞櫃", "購物車", "條形碼", "收銀台", "試衣間", "導購員", "售貨員", "推銷員",
            "保險員", "房產中介", "共享單車", "外賣箱", "快遞箱", "封箱膠帶", "防偽標籤", "防腐劑",
            "工作匯報", "年終總結", "賦能", "顆粒度", "痛點", "爆點", "轉換率", "點擊率",
            // Round 60 新增：舶司前棚/文書房/考工院/學堂/驛站/皮榻/厚褥/測熱管/羅盤/酒坊/短襖正名，與網梗黑話、企管人資、3C數碼、臨床醫藥、西式餐飲、現代載具、官僚體制及玄幻仙俠套路詞（210 項，累計 1,556 項）
            "海關前棚", "海關吏", "海關人群", "秘書處", "委員會", "學校", "郵局", "雙扣郵袋", "沙發", "床墊",
            "體溫計", "溫度計", "指南針", "酒廠", "釀酒廠", "傘骨廠", "夾克", "主席", "總統", "部長",
            "縣長", "鎮長", "村長", "警察", "公安", "工廠", "車間", "流水線", "柏油", "橡膠",
            "塑料", "化纖", "手電筒", "醫院",
            "種草官", "拔草記", "榜一大哥", "彈幕", "連麥", "點踩", "脫粉", "回踩", "拉黑", "取關",
            "互粉", "蹭熱度", "炒作", "公關文", "洗腦包", "節奏狗", "鍵盤俠", "網絡暴力", "網暴", "熱搜榜",
            "熱門話題", "吃瓜群眾",
            "復盤會", "抓手", "組合拳", "方法論", "頂層設計", "底層邏輯", "垂直領域", "生態位", "降本增效", "裁員",
            "優化名單", "全勤獎", "年假", "病假條", "事假", "產假", "陪產假", "五險一金", "勞動法", "勞動合同",
            "勞動仲裁", "辭退",
            "智能音箱", "藍牙耳機", "無線耳機", "降噪耳機", "耳麥", "對講機", "掃地機器人", "洗地機", "消毒櫃", "烤麵包機",
            "多士爐", "氣炸鍋", "空氣炸鍋", "電壓力鍋", "電熱水壺", "電熨斗", "掛燙機", "乾衣機", "中央空調", "變頻空調",
            "空氣淨化器", "掃描槍",
            "抗原檢測", "核酸檢測", "CT掃描", "心電圖機", "血糖儀", "血壓表", "水銀溫度計", "額溫槍", "耳溫槍", "口服液",
            "注射液", "點滴瓶", "生理鹽水", "葡萄糖注射液", "抗生素片", "阿司匹林", "布洛芬", "撲熱息痛", "青黴素", "頭孢",
            "紅黴素", "碘伏",
            "瑪奇朵", "濃縮咖啡", "拿鐵咖啡", "星冰樂", "冰美式", "熱美式", "冰沙", "蘇打水", "可口可樂", "百事可樂",
            "金酒", "莫吉托", "馬天尼", "血腥瑪麗", "戚風蛋糕", "黑森林蛋糕", "焦糖布丁", "泡芙", "法棍", "漢堡包",
            "吉士漢堡", "洋蔥圈",
            "公共汽車", "大客車", "雙層巴士", "觀光巴士", "私家車", "轎車", "麵包車", "警車", "消防車", "灑水車",
            "垃圾車", "油罐車", "水泥攪拌車", "磁懸浮", "有軌電車", "無軌電車", "客運索道", "客機", "民航機", "航站樓",
            "安全氣囊", "方向盤",
            "市委書記", "副市長", "省長", "副省長", "副縣長", "副鎮長", "村支書", "街道辦", "居委會", "民政局",
            "交警隊", "刑警", "緝毒警", "特警", "輔警", "城管", "工商局", "稅務局", "國稅局", "地稅局",
            "海關總署", "檢察院",
            "築基期", "金丹期", "元嬰期", "化神期", "煉虛期", "合體期", "大乘期", "渡劫期", "飛升成仙", "天劫雷罰",
            "九天玄雷", "紫霄神雷", "掌門人", "太上長老", "真傳弟子", "內門弟子", "外門弟子", "雜役弟子", "奪舍", "兵解",
            "儲物戒指", "乾坤袋",
            // Round 61 新增：襯衫/運動外套/公務外套/學生制服/皮鞋/涼拖/沾水筆/商務會計實務/天氣預報/大車廠/戰備糧草總儲倉/農業商行/兒化音/麻利正名，與北方俗語口癖、現代服飾鞋帽、衛浴家電、金融證券、勞資考核、電競網路、臨床藥劑、西點飲品及現代學位詞（243 項，累計 1,799 項）
            "襯衫", "運動外套", "公務外套", "學生制服", "皮鞋", "涼拖", "沾水筆", "商務會計實務", "天氣預報", "大車廠",
            "戰備糧草總儲倉", "農業商行", "待會兒", "一會兒", "這會兒", "那會兒", "一點兒", "有點兒", "一塊兒", "門兒清",
            "玩意兒", "今兒個", "麻利",
            "溜達兒", "遛彎兒", "找樂兒", "沒門兒", "一塊堆兒", "拔尖兒", "露餡兒", "打岔兒", "逗悶兒", "侃大山",
            "扯皮條", "耍貧嘴", "耍花招", "耍滑頭", "拍馬屁", "拍腦門", "摳門兒", "鑽空子", "打小抄", "走後門",
            "抄近道兒", "跑龍套", "挑大樑", "唱雙簧", "唱反調", "撂挑子", "穿小鞋", "碰釘子", "使絆兒", "找茬兒",
            "休閒西裝", "商務西裝", "西裝外套", "牛仔外套", "牛仔短褲", "牛仔背心", "工裝褲", "工裝連體褲", "喇叭褲", "闊腿褲",
            "平角短褲", "吊帶背心", "針織衫", "羊絨衫", "連體衣", "打底衫", "洞洞鞋", "雪地靴", "切爾西靴", "牛津鞋",
            "樂福鞋", "羅馬涼鞋", "貝雷帽", "漁夫帽", "空頂帽", "防曬面罩", "防曬袖套", "冰絲袖套", "胸針", "領帶夾",
            "智能馬桶", "智能馬桶蓋", "感應水龍頭", "恆溫花灑", "淋浴花灑", "浴霸", "暖風機", "電暖器", "電暖氣", "踢腳線暖氣",
            "擦窗機器人", "洗碗粉", "洗碗塊", "洗潔精", "潔廁靈", "管道疏通劑", "油污清洗劑", "除垢劑", "洗衣液", "柔順劑",
            "洗衣凝珠", "留香珠", "防蛀丸", "樟腦丸", "保鮮袋",
            "資產負債表", "現金流量表", "損益表", "營業利潤", "淨利潤", "毛利率", "營運資金", "流動資產", "固定資產", "無形資產",
            "股東權益", "應收賬款", "應付賬款", "壞賬準備", "審計報告", "會計師事務所", "註冊會計師", "兼併重組", "首次公開募股", "借殼上市",
            "證券交易所", "期貨合約", "期權交易", "杠桿交易", "槓桿交易",
            "保密協議", "競業協議", "競業限制", "績效考核", "績效指標", "核心指標", "季度考核", "年終考核", "十三薪", "全勤補貼",
            "餐費補貼", "交通補貼", "高溫補貼", "社保卡", "住房公積金", "人事檔案", "勞動爭議", "仲裁裁決", "背景調查", "轉正答辯",
            "定崗定薪", "職級評定", "職稱評定", "崗位說明書", "茶水間",
            "操作系統", "驅動程序", "應用程序", "服務端", "編程語言", "二進制", "十六進制", "網絡延遲", "丟包率", "網速",
            "上傳速度", "下載速度", "集線器", "交換機", "光端機", "射頻信號", "基帶芯片", "物聯網", "雲原生", "分佈式架構",
            "微服務", "容器化", "動態鏈接庫", "版本控制", "代碼倉庫",
            "心電監護儀", "氧氣面罩", "輸氧管", "血氧儀", "血糖試紙", "采血針", "留置針", "注射針筒", "消毒棉球", "酒精棉片",
            "無菌紗布", "彈力繃帶", "石膏繃帶", "醫用膠布", "抗過敏藥", "降壓藥", "降糖藥", "降脂藥", "抗凝血藥", "胃酸抑制劑",
            "質子泵抑制劑", "抗真菌藥", "鎮靜劑", "安眠片", "退燒貼",
            "曲奇餅乾", "華夫餅", "舒芙蕾", "蛋撻", "葡撻", "瑞士卷", "蘋果派", "櫻桃派", "南瓜派", "肉桂卷",
            "肉桂捲", "貝果", "可麗餅", "班尼迪克蛋", "千島醬", "番茄沙司", "果粒橙", "乳酸菌飲料", "益力多", "紅牛飲料",
            "學前班", "初級中學", "高等專科", "本科生", "碩士生", "博士後", "碩士論文", "博士論文", "開題報告", "畢業典禮",
            "學位服", "學士帽", "期中考", "摸底考", "模擬考",
            // Round 62 新增：根詞2字兒化音後綴、現代視力計時與廚衛電器、現代工業廠所及225項現代各領域禁詞（259 項，累計 2,058 項）
            // 0A. 根詞兒化音後綴與清洗收官項（19 項）
            "活兒", "大夥兒", "會兒", "事兒", "勁兒", "門兒", "味兒", "昨兒", "今兒", "塊兒", "伴兒",
            "老花鏡", "鋸木廠", "造船廠", "精煉廠", "開關", "馬桶", "國小", "國中",
            // 0B. 現代廚衛固定裝置、照明電力與現代服飾基礎詞（15 項）
            "漏電開關", "空氣開關", "配電箱", "石英錶", "機械錶", "電子錶", "高領毛衣", "真絲領帶",
            "百褶短裙", "一步裙", "衝鋒褲", "速乾衣", "速乾褲", "棒球服", "連帽衛衣",
            // 1. 北方口癖俗語與兒化音（25 項）
            "壓根兒", "透底兒", "準信兒", "挑刺兒", "兜底兒", "兜圈兒", "節骨眼兒", "肚臍眼兒", "死心眼兒", "一丁點兒",
            "打盹兒", "搭訕兒", "溜號兒", "擺譜兒", "炸鍋兒", "二把刀", "半吊子", "耍光棍", "打光棍", "打圓場",
            "走過場", "擺架子", "裝蒜", "裝糊塗", "鬧彆扭",
            // 2. 現代工業廠所、流水線與化學品（30 項）
            "木材廠", "磚瓦廠", "鐵工廠", "紡織廠", "加工廠", "修船廠", "印刷廠", "鋼鐵廠", "玻璃廠", "造紙廠",
            "煉油廠", "化肥廠", "塑料廠", "橡膠廠", "冶煉廠", "軋鋼廠", "水泥廠", "火電廠", "水電站", "化工廠",
            "農藥廠", "製藥廠", "印染廠", "製衣廠", "罐頭廠", "自來水廠", "火力發電廠", "流水線作業", "車間主任", "副廠長",
            // 3. 現代家電與廚衛日用品（30 項）
            "立式空調", "壁掛空調", "雙門冰箱", "三門冰箱", "對開門冰箱", "滾筒洗衣機", "波輪洗衣機", "洗烘一體機", "消毒碗櫃", "微波烤箱",
            "嵌入式烤箱", "電陶爐", "電磁灶", "燃氣灶", "集成灶", "即熱式熱水器", "儲水式熱水器", "空氣能熱水器", "太陽能熱水器", "無葉風扇",
            "塔扇", "吊扇", "排氣扇", "換氣扇", "熱風機", "電暖手寶", "暖手寶", "除蟎儀", "掃拖一體機", "便攜音箱",
            // 4. 現代金融、證券投資與企管考核（30 項）
            "私募基金", "公募基金", "對沖基金", "指數基金", "股票型基金", "貨幣基金", "量化基金", "天使投資", "風險投資", "私募股權",
            "牛市", "熊市", "跳空高開", "跌停板", "漲停板", "熔斷機制", "平倉", "爆倉", "強制平倉", "追加保證金",
            "市盈率", "市淨率", "每股收益", "淨資產收益率", "複利增長", "年化收益率", "理財產品", "信託產品", "大額存單", "定期存款",
            // 5. 現代通訊網絡、電競與網絡黑話（30 項）
            "基站", "5G網絡", "4G網絡", "光纖寬帶", "百兆光纖", "千兆光纖", "千兆網卡", "無線局域網", "蜂窩網絡", "內網穿透",
            "端口映射", "虛擬局域網", "動態域名", "靜態IP", "動態IP", "子網掩碼", "默認網關", "網關服務器", "主機遊戲", "掌機",
            "掌上遊戲機", "手遊", "端遊", "頁遊", "帶飛", "五殺", "團滅", "推塔", "偷塔", "補兵",
            // 6. 現代臨床檢驗、疫苗與合成西藥（30 項）
            "彩色多普勒", "胃腸鏡", "鋇餐檢查", "造影劑", "活檢", "靜脈滴注", "肌肉注射", "皮下注射", "皮內注射", "封閉針",
            "破傷風抗毒素", "破傷風疫苗", "狂犬疫苗", "乙肝疫苗", "卡介苗", "頭孢克肟", "阿莫西林", "羅紅黴素", "左氧氟沙星", "諾氟沙星",
            "甲硝唑", "替硝唑", "阿昔洛韋", "利巴韋林", "奧司他韋", "硝酸甘油", "速效救心丸", "阿托伐他汀", "氨氯地平", "卡托普利",
            // 7. 現代西式烘焙甜點、花式咖啡與調味（25 項）
            "焦糖瑪奇朵", "馥芮白", "生椰拿鐵", "燕麥拿鐵", "髒髒咖啡", "冷萃咖啡", "手沖咖啡", "意式濃縮", "冰滴咖啡", "氣泡咖啡",
            "阿芙佳朵", "波霸奶茶", "波霸奶綠", "芋圓奶茶", "黑糖珍珠鮮奶", "楊枝甘露", "滿杯紅柚", "芝士奶蓋", "奶蓋茶", "千層蛋糕",
            "巴斯克蛋糕", "歌劇院蛋糕", "拿破崙蛋糕", "閃電泡芙", "布朗尼",
            // 8. 現代鐵道列車、特種專用車輛與航運（25 項）
            "動車組", "高鐵列車", "城際鐵路", "重載鐵路", "地鐵列車", "輕軌列車", "鉸接客車", "低地板巴士", "校車", "清障車",
            "高空作業車", "道路清掃車", "垃圾轉運車", "冷藏車", "保溫車", "無人駕駛", "自動駕駛", "輔助駕駛", "車載導航", "倒車雷達",
            "倒車影像", "行車記錄儀", "定速巡航", "自適應巡航", "防抱死系統",
            // Round 63 新增：現代稱謂機構/室內保鮮櫃正名（25 項）與北方口癖/工業電力/企管人資/臨床西藥/金融數位/軟體電競/西餐汽車專詞（243 項），共 268 項（累計 2,326 項）
            // 0. Round 63 清洗收官項（25 項）
            "會計事務所", "秦氏會計事務所", "會計師", "商務會計教室", "編輯助理",
            "紀錄員", "護理員", "外務員", "民事調解員", "陸老師宿舍",
            "老師宿舍", "保鮮櫃", "顯擺", "憋屈", "扯淡",
            "八卦鏡", "發燒", "防感冒草藥", "系學生", "巡防課",
            "研究員", "得瑟", "嘚瑟", "窩囊", "磨嘰",
            // 1. 北方口癖俗語、兒化音與大陸流行語（30 項）
            "貓膩", "貓膩兒", "捅婁子", "捅簍子", "墊底兒", "兜風兒", "開小灶", "壓箱底兒", "打啞謎", "和稀泥",
            "磨洋工", "唱高調", "吹牛皮", "打馬虎眼", "踢皮球", "跑偏兒", "翹尾巴", "露馬腳", "戴高帽", "半瓶醋",
            "炒魷魚", "敲竹槓", "耍嘴皮子", "沒心沒肺", "扣帽子", "穿幫", "穿幫兒", "唱獨角戲", "唱對台戲", "唱黑臉",
            // 2. 現代工業廠所、電力與電子（30 項）
            "焦化廠", "煉焦廠", "熱電廠", "風力發電站", "光伏電站", "變電所", "配電房", "變壓房", "高壓變電站", "軋鋁廠",
            "鋁業廠", "鑄造廠", "鍛造廠", "瀝青廠", "電鍍廠", "蓄電池廠", "晶片廠", "半導體廠", "封裝廠", "組裝廠",
            "總裝車間", "塗裝車間", "沖壓車間", "焊接車間", "機加工車間", "總工程師", "車間工段長", "工段長", "班組長", "安全生產員",
            // 3. 現代企管人資、職場黑話與辦公（30 項）
            "項目經理", "產品經理", "運營總監", "技術總監", "市場總監", "運營專員", "人事專員", "招聘專員", "行政專員", "培訓專員",
            "獵頭顧問", "獵頭公司", "勞務派遣", "靈活用工", "裁員補償", "離職證明", "交接單", "考勤表", "請假條", "報銷單",
            "差旅費", "差旅補貼", "招待費", "發票抬頭", "團建活動", "企業文化", "組織架構", "職能部門", "流程審批", "扁平化管理",
            // 4. 現代臨床檢驗、醫療設備與合成西藥（30 項）
            "血液透析機", "血液透析", "人工心肺機", "體外膜肺氧合", "腦電圖機", "腦電波圖", "磁共振成像", "骨密度儀", "霧化器", "霧化吸入",
            "喉鏡", "支氣管鏡", "膀胱鏡", "腹腔鏡", "宮腔鏡", "牽引床", "擔架床", "胰島素", "胰島素筆", "葡萄糖酸鈣",
            "鹽酸腎上腺素", "硫酸阿托品", "碳酸氫鈉注射液", "氯化鈉注射液", "甘露醇注射液", "地塞米松", "氫化可的松", "潑尼松", "蒙脫石散", "奧美拉唑",
            // 5. 現代金融、銀行證券與數字貨幣（30 項）
            "以太坊", "數字人民幣", "智能合約", "去中心化金融", "數字錢包", "冷錢包", "熱錢包", "挖礦機", "算力中心", "不動產權證",
            "房屋所有權證", "土地使用權證", "預售許可證", "抵押貸款", "按揭貸款", "公積金貸款", "商業貸款", "信用貸款", "授信額度", "循環額度",
            "承兌匯票", "銀行承兌匯票", "商業承兌匯票", "信用證", "保理業務", "融資租賃", "證券開戶", "證券賬戶", "資金池", "離岸賬戶",
            // 6. 現代通訊網絡、軟體開發與電競（32 項）
            "虛擬機", "雲服務器", "負載均衡", "緩存服務器", "正向代理", "域名解析", "抓包工具", "網絡爬蟲", "內存洩漏", "堆棧溢出",
            "空指針異常", "藍屏死機", "集成測試", "自動化測試", "代碼審查", "敏捷開發", "灰度發布", "熱更新", "服務熔斷", "服務降級",
            "服務限流", "容器編排", "對象存儲", "軟路由", "光貓", "機頂盒", "電競俱樂部", "排位賽", "巔峰賽", "服務器集群",
            "數據遷移", "斷點調試",
            // 7. 現代西式快餐烘焙、飲料與包裝（30 項）
            "麥辣雞翅", "奧爾良烤翅", "麥樂雞", "上校雞塊", "勁脆雞腿堡", "香辣雞腿堡", "板燒雞腿堡", "巨無霸漢堡", "雙層吉士堡", "鱈魚堡",
            "培根披薩", "瑪格麗特披薩", "芝心披薩", "意式肉醬麵", "金槍魚沙拉", "凱撒沙拉", "吞拿魚三明治", "熔岩蛋糕", "瑪德琳蛋糕", "費南雪",
            "達克瓦茲", "司康餅", "焦糖布蕾", "抹茶拿鐵", "焙茶拿鐵", "生酪拿鐵", "髒髒包", "植脂末", "奶精球", "真空包裝袋",
            // 8. 現代交通載具、汽車工程與航空（31 項）
            "混動汽車", "純電動車", "插電式混動", "增程式電動車", "充電樁", "超級充電站", "換電站", "動力電池", "電機控制器", "變速箱",
            "雙離合變速箱", "無級變速箱", "手自一體", "渦輪增壓", "機械增壓", "剎車盤", "剎車片", "剎車油", "防凍液", "玻璃水",
            "火花塞", "點火線圈", "節氣門", "三元催化器", "輪胎防滑鏈", "胎壓監測", "盲區監測", "車道保持", "自動泊車", "全景天窗",
            "抬頭顯示",
            // Round 64 新增：現代學制職稱/場所器具與口癖正名（35 項）及北方俗語/化工冶金/企管法務/臨床器械/金融稅務/AI計算機/西式快餐/交通汽配專詞（256 項），共 291 項（累計 2,617 項）
            // 0. Round 64 清洗收官與學制職稱防禦項（35 項）
            "教室", "鍋爐房", "遣散費", "收費站", "信用額度", "破罐破摔", "店長", "保溫桶",
            "擔架", "重卡", "門都沒有", "拉倒", "吃香喝辣", "賭波", "澡堂子", "套近乎",
            "胡說八道", "瞎扯", "瞎折騰", "吹牛", "好勒",
            "系主任", "教導主任", "學分", "畢業證書", "成績單", "試卷", "收銀員",
            "瞎掰", "不靠譜", "二百五", "鬼點子", "沒門", "砸鍋", "黃了",
            // 1. 北方口癖俗語、兒化音與大陸流行語（32 項）
            "撂攤子", "甩包袱", "背包袱", "兜老底", "揭老底", "打橫炮", "放空炮", "定心丸", "碰軟釘子", "鑽牛角尖",
            "下絆子", "下套子", "攀高枝", "開倒車", "走回頭路", "窩裡橫", "窩裡鬥", "唱白臉", "打哈哈", "打掩護",
            "敲邊鼓", "吹耳邊風", "缺心眼", "死腦筋", "一根筋", "榆木疙瘩", "定盤星", "倒胃口", "丟人現眼", "犯嘀咕",
            "滿嘴跑火車", "吊兒郎當",
            // 2. 現代工業廠所、化工冶金與機械工程（32 項）
            "聚乙烯", "聚丙烯", "聚氯乙烯", "聚苯乙烯", "聚酯纖維", "丙烯酸", "合成樹脂", "合成橡膠", "石油化工廠", "乙烯裂解",
            "催化裂化", "常減壓蒸餾", "高爐煉鐵", "轉爐煉鋼", "電弧爐煉鋼", "連鑄連軋", "冷軋車間", "熱軋車間", "熱浸鍍鋅", "酸洗車間",
            "數控機床", "數控銑床", "數控車床", "激光切割機", "等離子切割機", "水刀切割機", "注塑機", "壓鑄機", "液壓折彎機", "空氣壓縮機",
            "超聲波探傷儀", "動平衡機",
            // 3. 現代企管人資、行政官僚與法務條例（32 項）
            "背景審查", "無犯罪證明", "勞動仲裁院", "調解委員會", "集體合同", "保密協議書", "競業限制協議", "績效指標庫", "關鍵績效指標", "平衡計分卡",
            "三級安全教育", "安全責任制", "安全隱患排查", "工傷認定", "工傷保險", "生育保險", "失業保險金", "待業救濟金", "裁員名單", "協商解除",
            "單方解除", "終止勞動關係", "試用期考察", "轉正申請表", "崗位職責說明書", "工作日誌", "周報匯報", "年度述職", "述職報告", "免職通知",
            "行政記大過", "破產清算組",
            // 4. 現代臨床檢驗、醫療器械與合成西藥（32 項）
            "血常規", "尿常規", "生化全套", "肝功能檢查", "腎功能檢查", "電解質檢查", "凝血功能", "心肌酶譜", "腫瘤標誌物", "甲狀腺功能",
            "動脈血氣分析", "胸腔穿刺", "腹腔穿刺", "骨髓穿刺", "腰椎穿刺", "導尿管", "鼻飼管", "氣管插管", "氣管切開套管", "中央靜脈導管",
            "脈搏血氧儀", "注射泵", "輸液泵", "呼吸氣囊", "負壓吸引器", "電動吸痰器", "除顫監護儀", "外固定支架", "人工關節", "心臟起搏器",
            "二甲雙胍", "硝苯地平",
            // 5. 現代金融、銀行稅務、保險與房地產（32 項）
            "增值稅專用發票", "進項稅額", "銷項稅額", "企業所得稅", "個人所得稅", "印花稅票", "房產稅", "土地增值稅", "車船使用稅", "關稅完稅價格",
            "出口退稅", "稅收滯納金", "稅務稽查", "稅控盤", "清稅證明", "人身意外險", "財產損失險", "機動車交強險", "車損險", "重大疾病險",
            "理賠申請書", "商品房預售", "公攤面積", "套內建築面積", "得房率", "土地使用權出讓", "不動產登記中心", "房屋產權過戶", "貼現利息", "同業拆借",
            "存款準備金率", "逆回購",
            // 6. 現代通訊網絡、AI/計算機、消費電子與電競遊戲（32 項）
            "大型語言模型", "自然語言處理", "卷積神經網絡", "循環神經網絡", "生成對抗網絡", "擴散模型", "向量數據庫", "知識圖譜", "語義分割", "目標檢測",
            "提示工程", "微調訓練", "預訓練模型", "模型蒸餾", "量子計算機", "量子比特", "固態硬盤", "機械硬盤", "顯卡交火", "水冷散熱器",
            "導熱矽脂", "雙通道內存", "圖形處理器", "指令集架構", "精簡指令集", "光線追蹤", "全局光照", "垂直同步", "防撕裂技術", "曲面顯示器",
            "體感手柄", "外設驅動",
            // 7. 現代西式快餐、烘焙甜品、飲料與超市日用（32 項）
            "麥樂雞塊", "吮指原味雞", "奧爾良烤雞腿", "深海鱈魚條", "黃金脆皮雞", "黑椒牛肉堡", "雙層牛肉芝士堡", "培根芝士蛋堡", "墨西哥雞肉捲", "美式熱狗腸",
            "芝士熱狗棒", "夏威夷披薩", "超級至尊披薩", "意式薩拉米披薩", "奶油培根蘑菇麵", "肉醬千層麵", "黑松露意麵", "法式洋蔥湯", "奶油蘑菇濃湯", "美式大曲奇",
            "焦糖瑪德琳", "法式費南雪", "覆盆子馬卡龍", "藍莓芝士蛋糕", "紐約芝士蛋糕", "抹茶慕斯", "草莓拿破崙", "榴槤千層", "紅絲絨蛋糕", "巴氏殺菌奶",
            "脫脂純牛奶", "果粒酸奶",
            // 8. 現代交通載具、航空公路、物流倉儲與汽車配件（32 項）
            "磁浮列車", "電力機車", "內燃機車", "雙層客車", "重載貨運列車", "鐵路調度中心", "自動閉塞區間", "鐵路道岔", "無縫鋼軌", "軌道交通信號",
            "高架快速路", "城市快速路", "環城高速公路", "互通式立交橋", "高速服務區", "應急車道", "ETC收費通道", "ETC電子標籤", "不停車收費系統", "輪胎動平衡",
            "四輪定位儀", "汽車舉升機", "發動機積碳", "防凍冷卻液", "玻璃清洗液", "全合成機油", "半合成機油", "汽油添加劑", "三元催化轉化器", "無級變速器",
            "電子手剎", "車載娛樂系統",
            // Round 65 清理字詞（24 項）
            "警衛", "值班室", "病床", "瓷磚", "日曆", "車廂", "煞車", "明信片", "輪胎", "井蓋", "幫浦", "藍圖",
            "糊弄", "擦屁股", "背黑鍋", "窮光蛋", "搞清楚", "清潔工", "科研", "流體力學", "發條鐵青蛙", "剪板機", "下課", "員工",
            // 0. Round 65 當代各業職稱與商務辦公設備（32 項）
            "副店長", "大堂經理", "前台接待", "保潔員", "保安隊長", "物業經理", "列車長", "空姐", "送餐員", "送水工",
            "搬家公司", "家政服務", "鐘點工", "保姆", "月嫂", "育嬰師", "業務代表", "銷售代表", "客戶代表", "客服代表",
            "話務員", "播音員", "配音員", "條碼掃描槍", "條形碼掃描槍", "塑封機", "過塑機", "裝訂機", "打孔機", "切紙機",
            "印章機", "激光刻字機",
            // 1. 北方口癖俗語、兒化音與大陸當代流行語（31 項）
            "扯後腿", "打小報告", "坐冷板凳", "打太極", "放鴿子", "開綠燈", "開紅燈", "打秋風", "咬耳朵", "嚼舌根",
            "嚼舌頭", "打水漂", "有兩下子", "耍心眼", "耍花樣", "摸門道", "找碴", "溜肩膀", "打頭陣", "捅馬蜂窩",
            "打保票", "耍賴皮", "潑髒水", "揭傷疤", "炒冷飯", "唱主角", "掉鍊子", "抹稀泥", "撂臉子", "給臉色",
            "空手套白狼",
            // 2. 現代工業重工、化工塑料、冶金機電與能源工程（31 項）
            "熱塑性塑料", "熱固性塑料", "環氧樹脂", "酚醛樹脂", "聚氨酯", "聚四氟乙烯", "特氟龍", "有機矽", "矽橡膠", "氟橡膠",
            "氯丁橡膠", "丁苯橡膠", "電弧焊機", "氬弧焊機", "二氧化碳保護焊", "氣體保護焊", "激光焊接機", "感應加熱器", "中頻淬火爐", "高頻淬火機",
            "真空退火爐", "真空浸漆箱", "無心磨床", "外圓磨床", "內圓磨床", "平面磨床", "滾齒機", "插齒機", "拋丸機", "噴砂機",
            "超聲波清洗機",
            // 3. 現代企管人資、行政司法、公務考核與政經法規（31 項）
            "公務員考試", "行政許可證", "營業執照審驗", "稅務登記證", "組織機構代碼", "法人代表證明", "開戶許可證", "特種行業許可證", "消防安全檢查合格證", "食品經營許可證",
            "衛生許可證", "排污許可證", "施工許可證", "建設用地規劃許可證", "房屋拆遷補償", "動遷協議", "土地徵收補償", "安置房分配", "經濟適用房", "廉租房補貼",
            "公共租賃住房", "住房公積金繳存", "社會保障卡", "醫保結算單", "養老金核算", "失業登記證", "就業創業證", "勞動能力鑑定", "職業病診斷證明", "工傷等級評定",
            "勞動爭議調解書",
            // 4. 現代臨床醫學、檢驗影像、醫療器械與生物製藥（31 項）
            "正電子發射斷層掃描", "冠狀動脈造影", "心血管造影", "血管造影術", "動態心電圖", "動態血壓監測", "經顱多普勒", "彩色超聲診斷儀", "纖維支氣管鏡", "電子胃鏡",
            "電子結腸鏡", "血液透析濾過", "連續性腎臟替代治療", "高壓氧艙治療", "體外衝擊波碎石", "心臟起搏器植入", "血管內支架", "人工心臟瓣膜", "人工晶狀體", "人工耳蝸",
            "骨科內固定螺釘", "骨科接骨板", "全自動生化分析儀", "化學發光免疫分析儀", "流式細胞儀", "基因測序儀", "核酸擴增儀", "聚合酶鏈式反應", "重組人胰島素", "重組促紅細胞生成素",
            "單克隆抗體藥物",
            // 5. 現代金融證券、期貨理財、稅務審計與信託保險（31 項）
            "融券賣出", "融資買入", "轉融通業務", "約定購回式證券交易", "質押式回購", "買斷式回購", "大宗交易系統", "盤後定價交易", "集合競價系統", "連續競價系統",
            "股票期權合約", "股指期貨合約", "國債期貨合約", "外匯期貨合約", "商品期貨交易所", "金融期貨交易所", "證券登記結算機構", "合格境外機構投資者", "人民幣合格境外投資者", "證券投資基金託管",
            "資產支持證券", "房地產投資信託基金", "信用違約互換", "利率互換合約", "貨幣互換協議", "金融衍生工具", "投資者適當性管理", "反洗錢監控", "可疑交易報告", "關聯交易審批",
            "控股股東股權質押",
            // 6. 現代資訊通信、雲計算、AI大模型、半導體與電競網遊（31 項）
            "光刻機", "極紫外光刻機", "深紫外光刻機", "化學氣相沉積設備", "物理氣相沉積", "等離子體刻蝕機", "化學機械拋光機", "晶圓封裝測試", "硅片切割機", "倒裝芯片封裝",
            "晶圓級封裝", "三維集成電路", "多芯片模塊", "片上系統", "現場可編程門陣列", "專用集成電路", "數字信號處理器", "微控制器單元", "隨機存取存儲器", "靜態隨機存取存儲器",
            "動態隨機存取存儲器", "閃存芯片", "固態存儲控制器", "服務器操作系統", "分佈式文檔系統", "分佈式數據庫集群", "微服務架構", "服務網格", "容器鏡像倉庫", "持續集成流水線",
            "持續交付系統",
            // 7. 現代西式料理、烘焙甜點、包裝食品與商超日用（31 項）
            "惠靈頓牛排", "西冷牛排", "肉眼牛排", "菲力牛排", "戰斧牛排", "法式鵝肝醬", "法式焗蝸牛", "意式生牛肉片", "西班牙海鮮飯", "西班牙火腿片",
            "意式帕爾瑪火腿", "意式肉醬千層餅", "法式紅酒燉牛肉", "德式烤豬膝", "德式圖林根香腸", "法蘭克福香腸", "波隆那肉醬麵", "那不勒斯披薩", "意式薄底披薩", "芝加哥深盤披薩",
            "英式炸魚薯條", "美式水桶炸雞", "香辣雞米花", "吮指雞塊", "芝士焗龍蝦", "法式酥皮濃湯", "俄式羅宋湯罐頭", "美式蛤蜊濃湯", "法式可麗餅", "比利時華夫餅",
            "焦糖舒芙蕾",
            // 8. 現代交通運輸、鐵道航空、公路物流與汽車機電（31 項）
            "電力機車頭", "內燃機車頭", "蒸汽機車頭", "重載鐵路貨車", "鐵路集裝箱平車", "鐵路罐車", "鐵路敞車", "鐵路棚車", "鐵路冷藏保溫車", "鐵路接觸網供電",
            "受電弓滑板", "鐵路自動閉塞機", "鐵路聯鎖系統", "鐵路調度集中系統", "軌道探傷車", "鐵路清篩機", "大型養路機械", "城市軌道交通列車", "懸掛式單軌列車", "跨座式單軌列車",
            "中低速磁浮列車", "高鐵動車組餐車", "商務座車廂", "一等座車廂", "二等座車廂", "硬臥車廂", "軟臥車廂", "無座車票", "自動售票機", "自動檢票閘機",
            "電子客票系統",
        ];
        let mut has_pair = vec![false; 65536];
        for (filename, content) in files {
            let cleaned: std::borrow::Cow<'_, str> = if filename.starts_with("web/") {
                std::borrow::Cow::Owned(content.replace("奇點世界", "").replace("奇點", ""))
            } else {
                std::borrow::Cow::Borrowed(content)
            };
            has_pair.fill(false);
            let bytes = cleaned.as_bytes();
            for i in 0..bytes.len().saturating_sub(2) {
                if bytes[i] >= 0xE0 {
                    has_pair[((bytes[i + 1] as usize) << 8) | (bytes[i + 2] as usize)] = true;
                }
            }
            for word in forbidden {
                let wb = word.as_bytes();
                if wb.len() >= 3
                    && wb[0] >= 0xE0
                    && !has_pair[((wb[1] as usize) << 8) | (wb[2] as usize)]
                {
                    continue;
                }
                if wb.len() >= 6
                    && wb[wb.len() - 3] >= 0xE0
                    && !has_pair[((wb[wb.len() - 2] as usize) << 8) | (wb[wb.len() - 1] as usize)]
                {
                    continue;
                }
                assert!(
                    !cleaned.contains(word),
                    "{filename} contains forbidden word: {word}"
                );
            }
        }
    }
    #[test]
    fn test_plural_personality_weights_and_cornerstone_interaction() {
        // 1. 驗證先天單純形總和恆等於 600 且各向在 45..=165
        let w_neutral = compute_plural_personality_weights("路人", 12345, 20, 10);
        let innate = innate_personality_simplex(12345);
        assert_eq!(innate.iter().sum::<i32>(), 600);
        for &v in &innate {
            assert!((45..=165).contains(&v));
        }

        // 2. 驗證職業偏置
        let w_craftsman = compute_plural_personality_weights("鐵工坊鐵匠", 12345, 20, 10);
        assert!(w_craftsman.craft > w_neutral.craft);

        let w_merchant = compute_plural_personality_weights("商行主事", 12345, 20, 10);
        assert!(w_merchant.profit > w_neutral.profit);

        let w_guard = compute_plural_personality_weights("城哨巡守", 12345, 20, 10);
        assert!(w_guard.honor > w_neutral.honor);

        let w_family = compute_plural_personality_weights("持家內當家", 12345, 20, 10);
        assert!(w_family.care > w_neutral.care);

        // 3. 驗證情境調節
        let w_stressed = compute_plural_personality_weights("路人", 12345, 75, 10);
        assert!(w_stressed.pride > w_neutral.pride);
        assert!(w_stressed.care > w_neutral.care);
        assert!(w_stressed.profit > w_neutral.profit);

        let w_poor = compute_plural_personality_weights("路人", 12345, 20, 2);
        assert!(w_poor.profit > w_neutral.profit);
        assert!(w_poor.care > w_neutral.care);

        let w_wealthy = compute_plural_personality_weights("路人", 12345, 20, 30);
        assert!(w_wealthy.honor > w_neutral.honor);
        assert!(w_wealthy.craft > w_neutral.craft);
        assert!(w_wealthy.pride > w_neutral.pride);

        // 4. 驗證 weight_for 提取
        assert_eq!(w_craftsman.weight_for(Cornerstone::Craft), w_craftsman.craft);
        assert_eq!(w_merchant.weight_for(Cornerstone::Profit), w_merchant.profit);
        assert_eq!(w_guard.weight_for(Cornerstone::Honor), w_guard.honor);
    }

    #[test]
    fn test_innate_personality_simplex_conservation() {
        for seed in 0..200 {
            let simplex = innate_personality_simplex(seed);
            let sum: i32 = simplex.iter().sum();
            assert_eq!(sum, 600, "Seed {seed} sum was {sum}, expected 600");
            for (idx, &v) in simplex.iter().enumerate() {
                assert!(
                    (45..=165).contains(&v),
                    "Seed {seed} facet {idx} was {v}, expected in 45..=165"
                );
            }
        }
    }

    #[test]
    fn test_situational_if_then_offset_zero_sum() {
        let slots = [
            PsychSituationSlot::ScarcitySurvival,
            PsychSituationSlot::VocationalDuty,
            PsychSituationSlot::RelationalEncounter,
            PsychSituationSlot::HighStressCrisis,
        ];
        for seed in 0..100 {
            for slot in slots {
                let off = situational_if_then_offset(seed, slot);
                let sum: i32 = off.iter().sum();
                assert_eq!(sum, 0, "Seed {seed}, slot {:?} sum was {sum}, expected 0", slot);
                for &v in &off {
                    assert!((-32..=32).contains(&v), "Offset {v} out of bounds [-32, 32]");
                }
            }
        }
    }

    #[test]
    fn test_occupation_cornerstone_diversity_no_stereotypes() {
        use std::collections::HashSet;
        let mut merchant_cornerstones = HashSet::new();
        let mut smith_cornerstones = HashSet::new();
        for seed in 0..100 {
            merchant_cornerstones.insert(Cornerstone::from_role_and_seed("行商", seed));
            smith_cornerstones.insert(Cornerstone::from_role_and_seed("鐵匠", seed));
        }
        assert!(
            merchant_cornerstones.len() >= 4,
            "Expected at least 4 merchant cornerstone variants, got {:?}",
            merchant_cornerstones
        );
        assert!(merchant_cornerstones.contains(&Cornerstone::Honor), "Should include Honor merchant");
        assert!(merchant_cornerstones.contains(&Cornerstone::Care), "Should include Care merchant");
        assert!(merchant_cornerstones.contains(&Cornerstone::Pride), "Should include Pride merchant");
        assert!(merchant_cornerstones.contains(&Cornerstone::Profit), "Should include Profit merchant");
        assert!(smith_cornerstones.len() >= 4, "Expected diverse smith cornerstones, got {:?}", smith_cornerstones);
    }

    #[test]
    fn test_experiential_personality_shift_scars_and_gratitude() {
        let seed = 42u64;
        let hook_gratitude = RelationalHook {
            kind_label: "守諾償清".to_string(),
            target_name: "林掌櫃".to_string(),
            summary: "當面守諾償清積欠銀錢".to_string(),
            intensity: 80,
            target_id: "npc_lin".to_string(),
            updated_at_unix: 1720500000,
        };
        let hook_grudge = RelationalHook {
            kind_label: "求借遭拒".to_string(),
            target_name: "趙管事".to_string(),
            summary: "向其借貸求助卻求借遭拒".to_string(),
            intensity: 80,
            target_id: "npc_zhao".to_string(),
            updated_at_unix: 1720500000,
        };

        let w_gratitude = compute_plural_personality_weights_with_experience(
            "居民", "文曜", seed, 20, 10, 1, &[hook_gratitude], None,
        );
        let w_grudge = compute_plural_personality_weights_with_experience(
            "居民", "文曜", seed, 20, 10, 1, &[hook_grudge], None,
        );

        assert!(
            w_gratitude.honor > w_grudge.honor,
            "Gratitude experience should elevate Honor (got gratitude={}, grudge={})",
            w_gratitude.honor, w_grudge.honor
        );
        assert!(
            w_grudge.profit > w_gratitude.profit || w_grudge.pride > w_gratitude.pride,
            "Grudge/refusal experience should elevate Profit or Pride"
        );
    }

    #[test]
    fn test_dyadic_kotodama_loan_and_repayment_chain() {
        let now = 1720500000;
        let a_id = "test_npc_kotodama_borrower";
        let a_name = "沈玉";
        let a_role = "樵夫";
        let b_id = "test_npc_kotodama_lender";
        let b_name = "林員外";
        let b_role = "富商";
        let room = "文曜・市集石坪";

        // 第一階段：匱乏與充裕碰撞 -> 觸發借貸 (lend_magnesium)
        let plan_loan = plan_dyadic_kotodama_encounter(
            a_id, a_name, a_role, 2, None,
            b_id, b_name, b_role, 20, None,
            room, 20, 15, now,
        ).expect("Borrower in scarcity and lender in surplus with high sentiment should trigger loan plan");

        assert_eq!(plan_loan.plan_code, "lend_magnesium");
        assert_eq!(plan_loan.mg_delta_a, 4);
        assert!(plan_loan.keep_thread_open);
        assert_eq!(plan_loan.open_obligation_a, Some(("repay_debt".to_string(), b_id.to_string())));

        // 執行借貸方案
        commit_dyadic_kotodama_plan(a_id, a_name, b_id, b_name, &plan_loan, now);

        // 驗證借貸義務已開立
        let a_ob = crate::store::get_active_npc_obligation(a_id);
        assert_eq!(a_ob, Some(("repay_debt".to_string(), b_id.to_string())));

        // 驗證人情鉤子
        let hook_a = find_relational_hook_for_target(a_id, b_id, b_name).expect("A should have hook toward B");
        assert_eq!(hook_a.kind_label, "立契借貸");
        let hook_b = find_relational_hook_for_target(b_id, a_id, a_name).expect("B should have hook toward A");
        assert_eq!(hook_b.kind_label, "解囊借貸");

        // 驗證 A 的下步意圖
        {
            let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
            let st_a = states.get(a_id).expect("A state must exist");
            assert!(st_a.plan_steps[0].contains("四枚鎂幣"));
        }

        // 第二階段：A 籌措到足額鎂幣 (a_mg = 6 >= 4) 再度遭遇 B -> 觸發結清 (settle_debt)
        let plan_settle = plan_dyadic_kotodama_encounter(
            a_id, a_name, a_role, 6, a_ob,
            b_id, b_name, b_role, 16, None,
            room, 25, 20, now + 600,
        ).expect("Repaying debt when magnesium >= 4 should trigger settle_debt");

        assert_eq!(plan_settle.plan_code, "settle_debt");
        assert_eq!(plan_settle.mg_delta_a, -4);
        assert!(!plan_settle.keep_thread_open);
        assert_eq!(plan_settle.clear_obligation_a, Some("repay_debt".to_string()));

        // 執行結清方案
        commit_dyadic_kotodama_plan(a_id, a_name, b_id, b_name, &plan_settle, now + 600);

        // 驗證義務已解除
        assert!(crate::store::get_active_npc_obligation(a_id).is_none());

        // 驗證關係鉤子更新為守諾償清
        let updated_hook_a = find_relational_hook_for_target(a_id, b_id, b_name).expect("A hook must exist");
        assert_eq!(updated_hook_a.kind_label, "守諾償清");
        let updated_hook_b = find_relational_hook_for_target(b_id, a_id, a_name).expect("B hook must exist");
        assert_eq!(updated_hook_b.kind_label, "守諾償清");
    }

    #[test]
    fn test_dyadic_kotodama_loan_and_default_extension_chain() {
        let now = 1720600000;
        let a_id = "test_npc_defaulter_a";
        let a_name = "阿七";
        let a_role = "散工";
        let b_id = "test_npc_creditor_b";
        let b_name = "周巡長";
        let b_role = "城哨巡守";
        let room = "南哨・望樓步道";

        // 第一階段：借貸
        let plan_loan = plan_dyadic_kotodama_encounter(
            a_id, a_name, a_role, 1, None,
            b_id, b_name, b_role, 15, None,
            room, 10, 5, now,
        ).expect("Scarcity vs surplus loan trigger");
        assert_eq!(plan_loan.plan_code, "lend_magnesium");
        commit_dyadic_kotodama_plan(a_id, a_name, b_id, b_name, &plan_loan, now);

        let a_ob = crate::store::get_active_npc_obligation(a_id);
        assert_eq!(a_ob, Some(("repay_debt".to_string(), b_id.to_string())));

        // 第二階段：A 依然匱乏 (a_mg = 1 < 4)，再度相遇 -> 觸發債務展期 (default_extend_debt)
        let plan_extend = plan_dyadic_kotodama_encounter(
            a_id, a_name, a_role, 1, a_ob,
            b_id, b_name, b_role, 11, None,
            room, 15, 5, now + 300,
        ).expect("Default when magnesium < 4 should trigger default_extend_debt");

        assert_eq!(plan_extend.plan_code, "default_extend_debt");
        assert_eq!(plan_extend.mg_delta_a, 0);
        assert!(plan_extend.keep_thread_open);

        commit_dyadic_kotodama_plan(a_id, a_name, b_id, b_name, &plan_extend, now + 300);

        // 驗證義務未被清除
        assert_eq!(crate::store::get_active_npc_obligation(a_id), Some(("repay_debt".to_string(), b_id.to_string())));

        // 驗證關係鉤子更新為展期
        let hook_a = find_relational_hook_for_target(a_id, b_id, b_name).expect("A hook must exist");
        assert_eq!(hook_a.kind_label, "展期欠債");

        // 驗證 A 推進之意圖包含償還欠款
        {
            let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
            let st_a = states.get(a_id).expect("A state must exist");
            assert!(st_a.plan_steps[0].contains("周巡長之欠款"));
        }
    }

    #[test]
    fn test_dyadic_kotodama_subcontract_task_obligation() {
        let now = 1720700000;
        let a_id = "test_npc_subcontractor_a";
        let a_name = "老張";
        let a_role = "獵戶";
        let b_id = "test_npc_worker_b";
        let b_name = "小李";
        let b_role = "鐵工坊學徒";
        let room = "文曜・百鍛鐵工坊";

        // A 手頭有 repair_tool 義務，且身懷 6 鎂幣，B 手頭清空
        let a_ob = Some(("repair_tool".to_string(), "開山短斧".to_string()));
        let plan_sub = plan_dyadic_kotodama_encounter(
            a_id, a_name, a_role, 6, a_ob.clone(),
            b_id, b_name, b_role, 5, None,
            room, 15, 10, now,
        ).expect("Task subcontracting plan should trigger");

        assert_eq!(plan_sub.plan_code, "subcontract_obligation");
        assert_eq!(plan_sub.mg_delta_a, -2);
        assert!(plan_sub.keep_thread_open);
        assert_eq!(plan_sub.clear_obligation_a, Some("repair_tool".to_string()));
        assert_eq!(plan_sub.open_obligation_b, Some(("repair_tool".to_string(), "開山短斧".to_string())));

        // 預先掛載 A 的義務至 store
        crate::store::open_active_npc_obligation(a_id, "repair_tool", "開山短斧");

        // 執行託付
        commit_dyadic_kotodama_plan(a_id, a_name, b_id, b_name, &plan_sub, now);

        // 驗證 A 的義務已轉移清除，B 承接新義務
        assert!(crate::store::get_active_npc_obligation(a_id).is_none());
        assert_eq!(
            crate::store::get_active_npc_obligation(b_id),
            Some(("repair_tool".to_string(), "開山短斧".to_string()))
        );

        // 驗證鉤子
        let hook_a = find_relational_hook_for_target(a_id, b_id, b_name).expect("A hook must exist");
        assert_eq!(hook_a.kind_label, "出資託付");
        let hook_b = find_relational_hook_for_target(b_id, a_id, a_name).expect("B hook must exist");
        assert_eq!(hook_b.kind_label, "受託代辦");
    }

    #[test]
    fn test_round50_canonical_npc_names_and_catalog_room_tags() {
        assert_eq!(
            crate::store::resolve_npc_person_name("秦水生", "老堰頭水輪石磨首座|秦水生", Some("M")),
            "秦水生"
        );
        assert_eq!(
            crate::store::resolve_npc_person_name("陳萬福", "三水食堂當家大廚|陳萬福", Some("M")),
            "陳萬福"
        );
        assert_eq!(
            crate::store::resolve_npc_person_name("蘇錦心", "織造服飾坊主理人|蘇錦心", Some("F")),
            "蘇錦心"
        );
        assert_eq!(
            crate::store::resolve_npc_person_name("wt_vendor_noodle_soup", "現煮切仔麵攤女東家|林罔市", Some("F")),
            "林罔市"
        );
        assert!(
            crate::store::wenyao_household_slot_for("秦水生").home_name.starts_with("老堰頭"),
            "Qin Shuisheng slot home_name must start with 老堰頭"
        );
        assert_eq!(
            crate::store::wenyao_household_slot_for("秦水生").brief_tag,
            "水輪石磨匠"
        );
        assert_eq!(
            crate::store::wenyao_household_slot_for("秦水生").work_name,
            "老堰頭・水輪石磨工坊"
        );

        let canonical_settlement_cases = [
            ("秦水生", "老堰頭"),
            ("陳萬福", "文曜"),
            ("陸秉衡", "文曜"),
            ("岑守岳", "文曜"),
            ("許萬川", "風舶角"),
            ("許海音", "風舶角"),
            ("林正雄", "風舶角"),
            ("顧海平", "風舶角"),
            ("林山", "老堰頭"),
            ("馬山行", "北嶺腳"),
            ("石萬鈞", "半嶺鎮"),
            ("葉青", "南哨"),
            ("田大犁", "大圳頭"),
            ("牧野", "牛埔"),
            ("wt_npc_xu_agan", "文曜"),
            ("wt_vendor_noodle_soup", "文曜"),
        ];
        for (id, expected_settlement) in canonical_settlement_cases {
            assert_eq!(
                crate::store::canonical_npc_macro_settlement(id, ""),
                Some(expected_settlement),
                "canonical_npc_macro_settlement for {id} must be Some({expected_settlement})"
            );
            let slot = crate::store::wenyao_household_slot_for(id);
            assert_eq!(
                crate::store::normalize_household_home_settlement(slot.home_name),
                expected_settlement,
                "normalize_household_home_settlement for {id} home_name ({}) must be {expected_settlement}",
                slot.home_name
            );
        }
        let cat_0150 = crate::game::peninsula_catalog::find_catalog_entry("wt_cat_0150").unwrap();
        assert_eq!(cat_0150.room_tag, "食堂");
        let cat_0212 = crate::game::peninsula_catalog::find_catalog_entry("wt_cat_0212").unwrap();
        assert_eq!(cat_0212.room_tag, "獵具");
        let cat_0745 = crate::game::peninsula_catalog::find_catalog_entry("wt_cat_0745").unwrap();
        assert_eq!(cat_0745.room_tag, "燈塔");
        let cat_0774 = crate::game::peninsula_catalog::find_catalog_entry("wt_cat_0774").unwrap();
        assert_eq!(cat_0774.room_tag, "燈塔");
    }

    #[test]
    fn test_dyadic_kotodama_co_enacts_room_event_and_script_pack() {
        let plan = plan_dyadic_kotodama_encounter(
            "wt_npc_test_miller_a",
            "秦小匠",
            "水輪匠",
            20,
            None,
            "wt_npc_test_helper_b",
            "周幫工",
            "幫工",
            20,
            None,
            "老堰頭・水輪石磨工坊",
            10,
            10,
            1_775_000_000,
        )
        .expect("should co-enact active room EventPack and ScriptPack");
        assert_eq!(plan.plan_code, "co_enact_script_pack");
        assert!(
            plan.topic_override.contains("秦小匠") && plan.topic_override.contains("周幫工"),
            "unexpected topic_override: {}",
            plan.topic_override
        );
        assert!(
            plan.prompt_directive.contains("事件包與劇本包"),
            "unexpected prompt_directive: {}",
            plan.prompt_directive
        );
    }
}
