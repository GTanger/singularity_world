pub static SHOP_STOCK_LEDGER: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<(String, String), i32>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn get_shop_item_stock(seller_id: &str, item_id: &str) -> i32 {
    let key = (seller_id.to_string(), item_id.to_string());
    {
        let read_lock = SHOP_STOCK_LEDGER.read();
        if let Some(&stock) = read_lock.get(&key) {
            return stock;
        }
    }
    let mut write_lock = SHOP_STOCK_LEDGER.write();
    *write_lock.entry(key).or_insert_with(|| {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::new();
        seller_id.hash(&mut hasher);
        item_id.hash(&mut hasher);
        let h = hasher.finish();
        if seller_id.starts_with("wt_vendor_") {
            8 + (h % 5) as i32
        } else {
            3 + (h % 3) as i32
        }
    })
}

/// 統一雙軌關係分，計算議價底價折扣、信任分與關係備註。
pub fn compute_relationship_trade_terms(
    pid: &str,
    seller_id: &str,
    ask: i32,
) -> (i32, i32, &'static str) {
    let is_acquainted = crate::store::player_observed_npc(pid, seller_id)
        || crate::store::player_knows_npc(pid, seller_id);
    let favorability = crate::db::get_favorability(seller_id, pid);
    let rel = crate::pg::pool()
        .and_then(|p| crate::pg::living_world::get_relationship_summary(&p, seller_id, pid).ok())
        .unwrap_or_default();
    let composite_trust = (if is_acquainted { 20 } else { 0 })
        + favorability
        + rel.balance_score * 5
        + rel.familiarity_score * 2;
    let eff_balance = rel.balance_score + favorability / 8;

    let (floor, rel_note) = if eff_balance >= 4 || composite_trust >= 35 {
        ((ask * 55 / 100).max(1), "（念在往日交情深厚，願多讓幾分熟客價）")
    } else if eff_balance >= 2 || composite_trust >= 22 {
        ((ask * 62 / 100).max(1), "（見是熟識面孔，願意讓些情面價）")
    } else if eff_balance <= -3 || composite_trust <= -5 {
        ((ask * 90 / 100).max(1), "（對你神色冷淡，價碼咬得極緊）")
    } else {
        (trade_floor_from_ask(ask), "")
    };

    (floor, composite_trust, rel_note)
}

pub fn get_vendor_stall_stock(vendor_or_stall_id: &str) -> Option<(&'static str, &'static str, i32, i32)> {
    let (item_id, item_name, price_mg, canonical_vendor_id) =
        vendor_stall_offer_for_npc(vendor_or_stall_id)?;
    let stock = get_shop_item_stock(canonical_vendor_id, item_id);
    if stock > 0 {
        crate::store::clear_active_npc_obligation(canonical_vendor_id, "restock_goods");
    }
    Some((item_id, item_name, price_mg, stock))
}

pub fn consume_shop_item_stock(seller_id: &str, item_id: &str, qty: i32) -> Result<i32, i32> {
    let current = get_shop_item_stock(seller_id, item_id);
    if current < qty {
        return Err(current);
    }
    let mut lock = SHOP_STOCK_LEDGER.write();
    let key = (seller_id.to_string(), item_id.to_string());
    let entry = lock.entry(key).or_insert(current);
    *entry -= qty;
    Ok(*entry)
}

pub fn restock_shop_items_for_seller(seller_id: &str) {
    {
        let mut lock = SHOP_STOCK_LEDGER.write();
        for ((s_id, _), stock) in lock.iter_mut() {
            if s_id == seller_id {
                let target_stock = if s_id.starts_with("wt_vendor_") { 8 } else { 4 };
                *stock = (*stock).max(target_stock);
            }
        }
    }
    crate::store::clear_active_npc_obligation(seller_id, "restock_goods");
}

#[derive(Debug, Clone)]
pub enum VendorNpcPurchaseOutcome {
    #[allow(dead_code)]
    Purchased {
        item_id: &'static str,
        item_name: &'static str,
        price_mg: i32,
        remaining_stock: i32,
        buyer_mg_after: i32,
        seller_mg_after: i32,
    },
    OutOfStock {
        item_name: &'static str,
    },
    InsufficientFunds {
        item_name: &'static str,
        price_mg: i32,
        buyer_mg: i32,
    },
}

pub fn vendor_stall_offer_for_npc(
    vendor_or_stall_id: &str,
) -> Option<(&'static str, &'static str, i32, &'static str)> {
    match vendor_or_stall_id {
        "wt_vendor_roast_beef" | "wt_stall_roast_beef" | "wt_street_roast_snack_cart" => {
            Some(("wt_cat_0395", "街角炭烤原牛肉串", 3, "wt_vendor_roast_beef"))
        }
        "wt_vendor_noodle_soup" | "wt_stall_noodle_soup" => {
            Some(("wt_cat_0394", "切仔湯麵配滷蛋", 3, "wt_vendor_noodle_soup"))
        }
        "wt_vendor_pepper_bun" | "wt_stall_pepper_bun" => {
            Some(("wt_cat_0442", "酥皮鮮肉胡椒餅", 3, "wt_vendor_pepper_bun"))
        }
        "wt_vendor_herbal_tea" | "wt_stall_herbal_tea" => {
            Some(("wt_cat_0504", "寒水玉冰鎮黑糖仙草凍", 2, "wt_vendor_herbal_tea"))
        }
        "wt_vendor_tofu_pudding" | "wt_stall_tofu_pudding" => {
            Some(("wt_cat_0390", "黑糖薑汁熱豆花", 2, "wt_vendor_tofu_pudding"))
        }
        "wt_vendor_rice_roll" | "wt_stall_rice_roll" => {
            Some(("wt_cat_0399", "荷葉糯米炊飯", 3, "wt_vendor_rice_roll"))
        }
        "wt_vendor_soy_milk" | "wt_stall_soy_milk" => {
            Some(("wt_cat_0388", "現磨溫米漿", 2, "wt_vendor_soy_milk"))
        }
        "wt_vendor_morning_produce" | "wt_stall_morning_produce" => {
            Some(("wt_cat_0129", "水芹菜脆嫩莖", 3, "wt_vendor_morning_produce"))
        }
        "wt_vendor_morning_fish" | "wt_stall_morning_fish" => {
            Some(("wt_cat_0226", "淺澤青殼河蝦", 4, "wt_vendor_morning_fish"))
        }
        "wt_npc_ma_buxue" | "wt_stall_umbrella_boot_repair" => {
            Some(("wt_cat_0896", "韌竹骨桐油晴雨摺疊傘", 8, "wt_npc_ma_buxue"))
        }
        _ => None,
    }
}

pub(super) fn advance_room_event_pack_on_trade(room_name: &str, item_id: &str, item_name: &str) -> Option<String> {
    crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(room_name, item_name)
        .or_else(|| crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(room_name, item_id))
        .map(|(_pack_id, _step, line)| line)
}

pub fn settle_live_vendor_npc_purchase(
    vendor_id: &str,
    customer_id: &str,
    room_id: &str,
    now_unix: i64,
) -> Option<VendorNpcPurchaseOutcome> {
    let (item_id, item_name, mut price_mg, canonical_vendor_id) =
        vendor_stall_offer_for_npc(vendor_id)?;

    let stock = get_shop_item_stock(canonical_vendor_id, item_id);
    if stock <= 0 {
        crate::store::open_active_npc_obligation(
            canonical_vendor_id,
            "restock_goods",
            &format!("攤上【{item_name}】已售罄待補料"),
        );
        return Some(VendorNpcPurchaseOutcome::OutOfStock { item_name });
    }

    let buyer = db::get_entity(customer_id).ok().flatten()?;
    let is_player = buyer.kind == crate::entity::EntityKind::Player;
    if is_player {
        let has_lineage_or_trust = !crate::pg::living_world::get_entity_lineage_imprints(customer_id).is_empty()
            || crate::pg::pool()
                .and_then(|p| crate::pg::living_world::get_relationship_summary(&p, canonical_vendor_id, customer_id).ok())
                .map(|r| r.balance_score >= 2 || r.familiarity_score >= 3)
                .unwrap_or(false);
        if has_lineage_or_trust && price_mg > 3 {
            price_mg = (price_mg - (price_mg / 10).max(1)).max(2);
        }
    }
    if buyer.kind == crate::entity::EntityKind::Npc {
        static LAST_NPC_VENDOR_SALE_UNIX: std::sync::OnceLock<Mutex<HashMap<String, i64>>> =
            std::sync::OnceLock::new();
        let map = LAST_NPC_VENDOR_SALE_UNIX.get_or_init(|| Mutex::new(HashMap::new()));
        let mut lock = map.lock().unwrap();
        if let Some(&last_ts) = lock.get(canonical_vendor_id) {
            if now_unix - last_ts < 45 {
                return None;
            }
        }
        lock.insert(canonical_vendor_id.to_string(), now_unix);
    }
    if buyer.magnesium < price_mg {
        return Some(VendorNpcPurchaseOutcome::InsufficientFunds {
            item_name,
            price_mg,
            buyer_mg: buyer.magnesium,
        });
    }

    let rem = consume_shop_item_stock(canonical_vendor_id, item_id, 1).ok()?;
    if rem <= 0 {
        crate::store::open_active_npc_obligation(
            canonical_vendor_id,
            "restock_goods",
            &format!("攤上【{item_name}】已售罄待補料"),
        );
    }

    db::transfer_magnesium(customer_id, canonical_vendor_id, price_mg).ok()?;
    let _ = db::add_to_inventory(customer_id, item_id, 1);

    if let Some(arc) = crate::store::get_store() {
        let _ = arc.write().update_entity(customer_id, |e| {
            e.vit = (e.vit + 6).min(100);
            e.qi = (e.qi + 4).min(100);
        });
    }

    let buyer_mg_after = db::get_entity(customer_id)
        .ok()
        .flatten()
        .map(|e| e.magnesium)
        .unwrap_or(0);
    let seller_mg_after = db::get_entity(canonical_vendor_id)
        .ok()
        .flatten()
        .map(|e| e.magnesium)
        .unwrap_or(0);

    let room_name = db::get_room_name(room_id)
        .ok()
        .filter(|n| !n.is_empty() && !n.starts_with("grid:") && !n.starts_with("wt-physical-"))
        .unwrap_or_else(|| "街口攤位".to_string());
    let customer_name = crate::store::get_store()
        .and_then(|s| {
            s.read()
                .entities
                .get(customer_id)
                .map(|e| crate::store::resolve_npc_person_name(&e.id, &e.display_title, None))
        })
        .filter(|n| !n.is_empty() && n != "陌生人")
        .unwrap_or_else(|| customer_id.to_string());
    db::log_npc_event(
        now_unix,
        canonical_vendor_id,
        db::EVT_TRADE,
        &format!("在{room_name}把【{item_name}】賣給{customer_name}，實收{price_mg}鎂"),
    );
    let _ = advance_room_event_pack_on_trade(&room_name, item_id, item_name);

    if let Some(pool) = crate::pg::pool() {
        let _ = crate::pg::living_world::record_relationship_delta(
            &pool,
            canonical_vendor_id,
            customer_id,
            "traded",
            1,
            "vendor_stall_sale",
            now_unix,
        );
        let _ = crate::pg::living_world::record_player_world_action(
            &pool,
            customer_id,
            "trade",
            canonical_vendor_id,
            room_id,
            "vendor_stall_sale_settled",
            serde_json::json!({
                "role": "buyer",
                "item_id": item_id,
                "item_name": item_name,
                "paid_mg": price_mg,
                "remaining_stock": rem,
                "buyer_mg_after": buyer_mg_after,
                "seller_mg_after": seller_mg_after,
            }),
            now_unix,
        );
        let _ = crate::pg::living_world::record_player_world_action(
            &pool,
            canonical_vendor_id,
            "trade",
            customer_id,
            room_id,
            "vendor_stall_sale_settled",
            serde_json::json!({
                "role": "vendor",
                "item_id": item_id,
                "item_name": item_name,
                "received_mg": price_mg,
                "remaining_stock": rem,
                "buyer_mg_after": buyer_mg_after,
                "seller_mg_after": seller_mg_after,
            }),
            now_unix,
        );
    }

    Some(VendorNpcPurchaseOutcome::Purchased {
        item_id,
        item_name,
        price_mg,
        remaining_stock: rem,
        buyer_mg_after,
        seller_mg_after,
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shop_stock_decrements_and_blocks_when_sold_out_until_restocked() {
        let seller = "test_seller_chen_wanfu";
        let item = "wt_cat_beef_soup";
        let initial = get_shop_item_stock(seller, item);
        assert!((3..=5).contains(&initial));

        let after_one = consume_shop_item_stock(seller, item, 1).expect("first buy succeeds");
        assert_eq!(after_one, initial - 1);

        let after_rest =
            consume_shop_item_stock(seller, item, after_one).expect("buy remaining stock succeeds");
        assert_eq!(after_rest, 0);

        assert_eq!(consume_shop_item_stock(seller, item, 1), Err(0));

        restock_shop_items_for_seller(seller);
        assert_eq!(get_shop_item_stock(seller, item), 4);
    }

    #[test]
    fn test_resident_merchant_stock_matches_catalog() {
        let resident_npcs = [
            "陸承禮", "陸長貴", "陸秉衡", "陳萬福", "沈清河", "白泉生", "溫晴", "溫懷瑾",
            "岑守岳", "岑拓", "柯成器", "蘇錦心", "秦明誠", "林時中", "沈硯舟", "陸知微",
            "陸敬軒", "趙衡宇", "岑雲野", "周敬修", "許阿甘", "馬守成", "韓鐵山",
            "wt_npc_lu_chengli", "wt_npc_lu_changgui", "wt_npc_lu_bingheng", "wt_npc_chen_wanfu",
            "wt_npc_shen_qinghe", "wt_npc_bai_quansheng", "wt_npc_wen_qing", "wt_npc_wen_huaijin",
            "wt_npc_cen_shouyue", "wt_npc_cen_tuo", "wt_npc_ke_chengqi", "wt_npc_su_jinxin",
            "wt_npc_qin_mingcheng", "wt_npc_lin_shizhong", "wt_npc_shen_yanzhou", "wt_npc_lu_zhiwei",
            "wt_npc_lu_jingxuan", "wt_npc_zhao_guanxing", "wt_npc_cen_yunye", "wt_npc_zhou_yidao",
            "wt_npc_xu_agan", "wt_npc_ma_buxue", "wt_npc_han_tieshan",
        ];

        let sample_rooms = [
            "", "契書局", "公議堂", "三水總商行", "三水大食堂", "東橋旅社與茶屋",
            "泉香溫水浴場", "溫氏脈絡診所", "岑氏第一鍛造工坊", "木作傢俱工坊",
            "織造服飾坊", "百工商算學舍", "傳訊鐘樓", "藏書閣", "文曜學院蒙學堂講習舍",
            "觀星台", "習武堂", "剪剃鋪", "柑仔店", "修傘補靴攤", "韓氏貨運行",
            "河鮮大市", "調味油坊", "酒坊", "乾糧行", "麵食鋪", "茶行", "菜蔬大市",
            "筆墨文具鋪", "圖書典藏庫", "生藥棧", "成藥配劑所", "重鐵水錘鍛坊", "農具五金行",
            "護商武備鋪", "鐘錶儀器坊", "連軸鋸木坊", "竹編藤器作坊", "切石石雕作坊",
            "灰漿磚瓦窯", "炭薪大棧", "青釉官窯", "大布莊", "造船塢", "重載雙輪大車坊",
            "駝馬大驛館", "粗鹽生鐵庫",
        ];

        for &npc in &resident_npcs {
            let ch: Character = serde_json::from_value(serde_json::json!({
                "id": npc,
                "kind": "npc",
                "display_title": format!("東家|{}", npc),
                "current_activity": "正在鋪子整理貨架"
            }))
            .unwrap();
            for &room in &sample_rooms {
                for second in [0, 60, 120, 180, 240, 300] {
                    if let Some((item_id, item_name, _price)) = canonical_shopkeeper_trade_offer(&ch, room, second) {
                        if item_id.starts_with("wt_cat_") {
                            let entry = crate::game::peninsula_catalog::find_catalog_entry(&item_id)
                                .unwrap_or_else(|| panic!("Catalog entry missing for {}", item_id));
                            assert_eq!(
                                entry.name, item_name,
                                "Name mismatch for item_id: {}, expected: {}, got: {}",
                                item_id, entry.name, item_name
                            );
                        }
                    }
                }
            }
        }

        let stall_ids = [
            "wt_vendor_roast_beef", "wt_stall_roast_beef", "wt_street_roast_snack_cart",
            "wt_vendor_noodle_soup", "wt_stall_noodle_soup", "wt_vendor_pepper_bun",
            "wt_stall_pepper_bun", "wt_vendor_herbal_tea", "wt_stall_herbal_tea",
            "wt_vendor_tofu_pudding", "wt_stall_tofu_pudding", "wt_vendor_rice_roll",
            "wt_stall_rice_roll", "wt_vendor_soy_milk", "wt_stall_soy_milk",
            "wt_vendor_morning_produce", "wt_stall_morning_produce",
            "wt_vendor_morning_fish", "wt_stall_morning_fish",
            "wt_npc_ma_buxue", "wt_stall_umbrella_boot_repair",
        ];

        for &stall in &stall_ids {
            if let Some((item_id, item_name, _price, _)) = vendor_stall_offer_for_npc(stall) {
                if item_id.starts_with("wt_cat_") {
                    let entry = crate::game::peninsula_catalog::find_catalog_entry(item_id)
                        .unwrap_or_else(|| panic!("Catalog entry missing for {}", item_id));
                    assert_eq!(
                        entry.name, item_name,
                        "Vendor stall name mismatch for item_id: {}, expected: {}, got: {}",
                        item_id, entry.name, item_name
                    );
                }
            }
        }
    }

    #[test]
    fn test_shop_offer_and_talk_driven_by_room_and_counter_staff_not_hardcoded_name() {
        // 1. An unacquainted plain-clothes manager (陌生人) standing in the counter zone of "文曜・三水大食堂"
        // gets the canteen trade offer
        let manager: Character = serde_json::from_value(serde_json::json!({
            "id": "npc_shanshui_canteen_manager",
            "kind": "npc",
            "display_title": "陌生人",
            "current_activity": "站在櫃檯後方當值看顧"
        }))
        .unwrap();

        let canteen_room = "文曜・三水大食堂";
        let offer_mgr = canonical_shopkeeper_trade_offer(&manager, canteen_room, 0);
        assert!(
            offer_mgr.is_some(),
            "plain-clothes manager in counter zone must get canteen trade offer"
        );
        let (item_id, item_name, _price) = offer_mgr.unwrap();
        assert!(
            item_id == "wt_cat_0444" || item_id == "wt_cat_0388" || item_id == "wt_cat_0390",
            "must be a canteen menu item, got: {} ({})", item_id, item_name
        );

        // 2. A passerby diner (正在用餐的食客) in "文曜・三水大食堂" returns None
        let diner: Character = serde_json::from_value(serde_json::json!({
            "id": "npc_passerby_diner_01",
            "kind": "npc",
            "display_title": "陌生人",
            "current_activity": "正在用餐的食客"
        }))
        .unwrap();
        let offer_diner = canonical_shopkeeper_trade_offer(&diner, canteen_room, 0);
        assert!(
            offer_diner.is_none(),
            "passerby diner must NOT get shop trade offer"
        );

        // 3. A resident artisan (e.g. a blacksmith from 鍛造工坊) visiting "文曜・三水大食堂"
        // is NOT treated as "文曜・三水大食堂"'s clerk (is_shop_person is false in "文曜・三水大食堂")
        let blacksmith: Character = serde_json::from_value(serde_json::json!({
            "id": "wt_npc_cen_shouyue",
            "kind": "npc",
            "display_title": "岑氏第一鍛造工坊大匠師|岑守岳",
            "current_activity": "正在大食堂用餐歇息"
        }))
        .unwrap();
        assert!(
            !is_shop_person(&blacksmith, canteen_room),
            "blacksmith visiting canteen must NOT be recognized as canteen shop staff"
        );
        assert!(
            canonical_shopkeeper_trade_offer(&blacksmith, canteen_room, 0).is_none(),
            "blacksmith visiting canteen must NOT get canteen trade offer"
        );
    }

    #[test]
    fn test_trade_advances_room_event_and_script_pack_when_key_object_exchanged() {
        let active = crate::game::peninsula_scripts::active_event_pack_for_room("契書局", None)
            .or_else(|| crate::game::peninsula_scripts::active_event_and_script_pack_for_room("契書局", None, 1_700_000_000));
        assert!(active.is_some(), "must resolve active event pack for deed office");
        let (ev, _sp) = active.unwrap();
        let key_obj = ev.key_objects.split('、').next().unwrap_or("公議堂紅印屋契與租約摺");
        let line_opt = advance_room_event_pack_on_trade("契書局", "", key_obj);
        assert!(
            line_opt.is_some(),
            "exchanging key object {key_obj} must advance event pack"
        );
        let line = line_opt.unwrap();
        assert!(
            line.contains("一同落實手頭要務"),
            "advancement note must contain '一同落實手頭要務', got: {line}"
        );
        assert!(
            !line.contains("事件包") && !line.contains("劇本包") && !line.contains("劇本包推進"),
            "advancement note must not contain meta-terms: {line}"
        );
    }
}
