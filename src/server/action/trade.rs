//! `do_action` — Trade 分支：NPC 報價、玩家議價、鎂轉帳 + inventory 轉移。

use std::collections::HashMap;
use std::sync::Mutex;
use crate::db::{
    self, TradePending, default_trade_ask_mg, get_entity_room, get_item_def, inventory_has_qty,
    mirror_market_trade_settlement, pick_npc_trade_offer, settle_inventory_trade,
    trade_floor_from_ask, trade_offer_clear, trade_offer_get, trade_offer_set,
};
use crate::entity::Character;
use crate::event::{self, types};
use crate::gametext;

use super::super::handler::{WsConnection, push_refresh};
use super::super::protocol::{ActionResultMsg, ClientMsg};
use super::display_target_name;

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
            Some(("wt_cat_0442", "三水食堂酥皮鮮肉胡椒餅", 3, "wt_vendor_pepper_bun"))
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
            Some(("wt_cat_0388", "老堰頭現磨溫米漿", 2, "wt_vendor_soy_milk"))
        }
        "wt_vendor_morning_produce" | "wt_stall_morning_produce" => {
            Some(("wt_cat_0129", "老堰頭水芹菜脆嫩莖", 3, "wt_vendor_morning_produce"))
        }
        "wt_vendor_morning_fish" | "wt_stall_morning_fish" => {
            Some(("wt_cat_0226", "水尾寮淺澤青殼河蝦", 4, "wt_vendor_morning_fish"))
        }
        "wt_npc_ma_buxue" | "wt_stall_umbrella_boot_repair" => {
            Some(("wt_cat_0896", "韌竹骨桐油晴雨摺疊傘", 8, "wt_npc_ma_buxue"))
        }
        _ => None,
    }
}

pub fn settle_live_vendor_npc_purchase(
    vendor_id: &str,
    customer_id: &str,
    room_id: &str,
    now_unix: i64,
) -> Option<VendorNpcPurchaseOutcome> {
    let (item_id, item_name, price_mg, canonical_vendor_id) =
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

    db::log_npc_event(
        now_unix,
        canonical_vendor_id,
        db::EVT_TRADE,
        &format!("在{room_id}把【{item_name}】賣給{customer_id}，實收{price_mg}鎂"),
    );

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

fn is_trade_reject_input(s: &str) -> bool {
    matches!(
        s.trim().to_lowercase().as_str(),
        "拒絕" | "取消" | "算了" | "不要" | "no" | "n"
    )
}
fn apply_player_sale_payout_with_debt_settlement(pid: &str, payout: i32, now: i64) -> (i32, i32, String) {
    let active_debt = crate::npc::debt_interaction::active_debt_amount(pid);
    if active_debt > 0 && payout > 0 {
        let max_deduct = (payout / 2).max(1).min(payout);
        let repay_amount = max_deduct.min(active_debt);
        let cleared = crate::npc::debt_interaction::settle_debt_for_actor(pid, repay_amount);
        let actual_payout = (payout - cleared).max(0);
        let _ = db::add_magnesium(pid, actual_payout);
        if let Some(pool) = crate::pg::pool() {
            if crate::npc::debt_interaction::active_debt_amount(pid) == 0 {
                let _ = crate::pg::living_world::resolve_obligations_by_kind(&pool, pid, "repay_debt", now);
            }
        }
        let note = if cleared > 0 {
            format!("「順帶清償了先前記在帳上的 {cleared} 鎂賒欠。」")
        } else {
            String::new()
        };
        (actual_payout, cleared, note)
    } else {
        let _ = db::add_magnesium(pid, payout);
        (payout, 0, String::new())
    }
}

/// 回傳 true 表示已 sendError，呼叫方應停止。
pub(super) fn do_entity_trade(
    conn: &WsConnection,
    msg: &ClientMsg,
    pid: &str,
    target_id: &str,
    target: &Character,
    now: i64,
) -> bool {
    let trade_target_name = super::player_visible_target_name(pid, target);
    let player_trade_input = msg.player_input.trim();
    if is_trade_reject_input(player_trade_input) {
        trade_offer_clear(pid, target_id);
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: format!("你中止了與【{trade_target_name}】的交易。"),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
    }
    let is_craft_intent = player_trade_input.starts_with("煉製")
        || player_trade_input.starts_with("加工")
        || player_trade_input.to_lowercase().starts_with("craft");
    if is_craft_intent {
        let raw_query = player_trade_input
            .trim_start_matches(|c: char| c == '煉' || c == '製' || c == '加' || c == '工' || c == 'c' || c == 'C' || c == 'r' || c == 'R' || c == 'a' || c == 'A' || c == 'f' || c == 'F' || c == 't' || c == 'T')
            .trim();
        let room_name = db::get_entity_room(pid)
            .ok()
            .and_then(|r| db::get_room_name(&r).ok())
            .unwrap_or_else(|| "白塔商市".to_string());
        match crate::game::object_ecology::craft_from_raw_material(pid, if raw_query.is_empty() { "auto" } else { raw_query }, &room_name, now) {
            Ok((_item_id, _item_name, _is_mut, craft_narrative)) => {
                push_refresh(conn, pid);
                let _ = db::adjust_favorability(target_id, pid, 1);
                let _ = event::append(now, pid, "craft_trade", target_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("你請【{trade_target_name}】協同掌爐加工：\n{craft_narrative}"),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
            }
            Err(err) => {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("【{trade_target_name}】看過你遞上的原料清單後道：「{err}」"),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
            }
        }
        return false;
    }

    let is_mutate_intent = player_trade_input.starts_with("異變")
        || player_trade_input.to_lowercase().starts_with("mutate");
    if is_mutate_intent {
        let item_query = player_trade_input
            .trim_start_matches(|c: char| c == '異' || c == '變' || c == 'm' || c == 'M' || c == 'u' || c == 'U' || c == 't' || c == 'T' || c == 'a' || c == 'A' || c == 'e' || c == 'E')
            .trim();
        let room_name = db::get_entity_room(pid)
            .ok()
            .and_then(|r| db::get_room_name(&r).ok())
            .unwrap_or_else(|| "白塔商市".to_string());
        match crate::game::object_ecology::mutate_finished_item(pid, if item_query.is_empty() { "auto" } else { item_query }, &room_name, now) {
            Ok((_mut_id, _mut_name, mutate_narrative)) => {
                push_refresh(conn, pid);
                let _ = db::adjust_favorability(target_id, pid, 2);
                let _ = event::append(now, pid, "mutate_trade", target_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("你請【{trade_target_name}】共同護持氣脈淬火：\n{mutate_narrative}"),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
            }
            Err(err) => {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("【{trade_target_name}】凝神審視你的隨身器物與氣脈後道：「{err}」"),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
            }
        }
        return false;
    }
    let is_selling_intent = player_trade_input.starts_with("賣")
        || player_trade_input.starts_with("售")
        || player_trade_input.to_lowercase().starts_with("sell");
    if is_selling_intent {
        let raw_query = player_trade_input
            .trim_start_matches(|c: char| c == '賣' || c == '售' || c == 's' || c == 'S' || c == 'e' || c == 'E' || c == 'l' || c == 'L')
            .trim();
        let Ok(Some(player_ent)) = db::get_entity(pid) else {
            conn.send_error("玩家實體不存在");
            return false;
        };
        let inv = db::get_inventory(&player_ent.inventory, player_ent.vit);
        let candidate = if !raw_query.is_empty() {
            inv.items.into_iter().find(|it| {
                it.item_id.contains(raw_query)
                    || get_item_def(&it.item_id).is_some_and(|d| d.name.contains(raw_query))
                    || crate::game::peninsula_catalog::find_catalog_entry(&it.item_id).is_some_and(|c| c.name.contains(raw_query))
            })
        } else {
            inv.items.into_iter().find(|it| {
                it.item_id.starts_with("mat_")
                    || it.item_id.starts_with("harvest_")
                    || it.item_id.starts_with("wt_cat_")
                    || crate::world::materials::trade_price_anchor_mg(&it.item_id).is_some()
            })
        };

        if let Some(item) = candidate {
            let item_name = get_item_def(&item.item_id)
                .map(|d| d.name)
                .or_else(|| {
                    crate::world::materials::material_by_item_id(&item.item_id)
                        .map(|m| m.name.to_string())
                })
                .or_else(|| {
                    crate::game::peninsula_catalog::find_catalog_entry(&item.item_id)
                        .map(|c| c.name.to_string())
                })
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| {
                    if item.item_id.is_ascii() {
                        "隨身器物".to_string()
                    } else {
                        item.item_id.clone()
                    }
                });

            let base_price = crate::world::materials::trade_price_anchor_mg(&item.item_id)
                .or_else(|| {
                    crate::game::peninsula_catalog::find_catalog_entry(&item.item_id)
                        .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
                })
                .or_else(|| {
                    for fg in &crate::game::object_ecology::STANDARD_FINISHED_GOODS {
                        if fg.id == item.item_id {
                            return Some(fg.base_price);
                        }
                    }
                    get_item_def(&item.item_id).map(|d| (d.weight * 15.0).round() as i32)
                })
                .unwrap_or(3)
                .max(3);

            let player_room = db::get_entity_room(pid)
                .ok()
                .and_then(|r| db::get_room_name(&r).ok())
                .unwrap_or_else(|| "文曜".to_string());

            let quote = crate::game::object_ecology::dynamic_settlement_market_quote(
                &player_room,
                &item.item_id,
                base_price,
            );
            let unit_price = quote.unit_price;

            if db::remove_from_inventory(pid, &item.item_id, 1).is_ok() {
                let (_actual_payout, _cleared, debt_note) =
                    apply_player_sale_payout_with_debt_settlement(pid, unit_price, now);
                let _ = db::add_to_inventory(target_id, &item.item_id, 1);
                let after_quote = crate::game::object_ecology::record_settlement_market_trade(
                    &player_room,
                    &item.item_id,
                    base_price,
                    1,
                    "玩家售出",
                );
                push_refresh(conn, pid);
                let _ = db::adjust_favorability(target_id, pid, 1);
                let _ = event::append(now, pid, "trade", target_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: {
                        let debt_suffix = if debt_note.is_empty() {
                            String::new()
                        } else {
                            format!(" {}", debt_note)
                        };
                        format!(
                            "【{trade_target_name}】端詳你遞上的【{item_name}】（{}），依當地即時供需行情以 {} 鎂收下。成交後本地市集庫存變為 {}/{}（下一件收購行情調整為 {} 鎂）。{debt_suffix}",
                            quote.market_narrative,
                            quote.unit_price,
                            after_quote.local_stock,
                            after_quote.target_stock,
                            after_quote.unit_price
                        )
                    },
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return false;
            }
        } else {
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Trade".into(),
                target_id: target.id.clone(),
                target_name: trade_target_name.clone(),
                narrative: format!("你向【{trade_target_name}】提出售賣，但行囊中目前沒有可供收購的物資。"),
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return false;
        }
    }
    let pending = match trade_offer_get(pid, target_id) {
        Some(p) => p,
        None => {
            if !player_trade_input.is_empty() {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: "對方尚未開價。請先點【交易】取得報價，再在輸入欄填寫出價（鎂）。"
                        .into(),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return false;
            }
            let personal_item = pick_npc_trade_offer(&target.inventory);
            let market_offer = if personal_item.is_none() {
                crate::pg::pool().and_then(
                    |pool| match crate::pg::world_market::offer_for_operator(
                        &pool, target_id, pid, now,
                    ) {
                        Ok(offer) => offer,
                        Err(error) => {
                            tracing::warn!("market offer failed {target_id}/{pid}: {error}");
                            None
                        }
                    },
                )
            } else {
                None
            };
            let room_name = super::resolve_action_room_name(pid, target);
            let canonical_offer = if personal_item.is_none() && market_offer.is_none() {
                canonical_shopkeeper_trade_offer(target, &room_name, now)
            } else {
                None
            };
            if personal_item.is_none() && market_offer.is_none() && canonical_offer.is_none() {
                let player_sell_mat = db::get_entity(pid).ok().flatten().and_then(|player_ent| {
                    let inv = db::get_inventory(&player_ent.inventory, player_ent.vit);
                    inv.items.into_iter().find(|it| {
                        it.item_id.starts_with("mat_")
                            || it.item_id.starts_with("harvest_")
                            || crate::world::materials::trade_price_anchor_mg(&it.item_id).is_some()
                    })
                });

                if let Some(mat_item) = player_sell_mat {
                    let resolve_item_display_name = |item_id: &str| -> String {
                        get_item_def(item_id)
                            .map(|d| d.name)
                            .or_else(|| {
                                crate::world::materials::material_by_item_id(item_id)
                                    .map(|m| m.name.to_string())
                            })
                            .or_else(|| {
                                crate::game::peninsula_catalog::find_catalog_entry(item_id)
                                    .map(|c| c.name.to_string())
                            })
                            .filter(|n| !n.trim().is_empty())
                            .unwrap_or_else(|| {
                                if item_id.is_ascii() {
                                    "隨身器物".to_string()
                                } else {
                                    item_id.to_string()
                                }
                            })
                    };
                    let item_name = resolve_item_display_name(&mat_item.item_id);

                    let base_price = crate::world::materials::trade_price_anchor_mg(&mat_item.item_id)
                        .or_else(|| {
                            crate::game::peninsula_catalog::find_catalog_entry(&mat_item.item_id)
                                .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
                        })
                        .unwrap_or(3)
                        .max(3);
                    let player_room = {
                        let raw = if !room_name.is_empty() {
                            room_name.clone()
                        } else {
                            db::get_entity_room(pid)
                                .ok()
                                .and_then(|r| db::get_room_name(&r).ok())
                                .unwrap_or_default()
                        };
                        if raw.is_empty() {
                            "文曜".to_string()
                        } else if raw.contains('・') {
                            raw.split('・').next().unwrap_or(&raw).trim().to_string()
                        } else {
                            raw
                        }
                    };

                    let quote = crate::game::object_ecology::dynamic_settlement_market_quote(
                        &player_room,
                        &mat_item.item_id,
                        base_price,
                    );
                    let unit_price = quote.unit_price;

                    if db::remove_from_inventory(pid, &mat_item.item_id, 1).is_ok() {
                        let (_actual_payout, _cleared, debt_note) =
                            apply_player_sale_payout_with_debt_settlement(pid, unit_price, now);
                        let _ = db::add_to_inventory(target_id, &mat_item.item_id, 1);
                        let after_quote = crate::game::object_ecology::record_settlement_market_trade(
                            &player_room,
                            &mat_item.item_id,
                            base_price,
                            1,
                            "玩家售出",
                        );
                        push_refresh(conn, pid);
                        let _ = db::adjust_favorability(target_id, pid, 1);
                        let _ = event::append(now, pid, "trade", target_id);
                        conn.send_json(&ActionResultMsg {
                            msg_type: "action_result".into(),
                            action: "Trade".into(),
                            target_id: target.id.clone(),
                            target_name: trade_target_name.clone(),
                            narrative: {
                                let debt_suffix = if debt_note.is_empty() {
                                    String::new()
                                } else {
                                    format!(" {}", debt_note)
                                };
                                format!(
                                    "【{trade_target_name}】端詳你遞上的【{item_name}】（{}），依當地即時供需行情以 {} 鎂收下。成交後本地市集庫存變為 {}/{}（下一件收購行情調整為 {} 鎂）。{debt_suffix}",
                                    quote.market_narrative,
                                    quote.unit_price,
                                    after_quote.local_stock,
                                    after_quote.target_stock,
                                    after_quote.unit_price
                                )
                            },
                            success: true,
                            actions: vec![],
                            move_target_id: String::new(),
                            open_trade: false,
                            suggested_action: String::new(),
                            tactical_choices: vec![],
                        });
                        return false;
                    }
                }

                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: {
                        let mut narr = format!(
                            "你向【{trade_target_name}】提出交易，對方表示目前暫無可交易之物。"
                        );
                        let report = crate::game::object_ecology::format_settlement_supply_demand_report(&room_name);
                        if !report.is_empty() {
                            narr.push('\n');
                            narr.push_str(&report);
                        }
                        narr
                    },
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return false;
            }
            let is_canonical_offer = canonical_offer.is_some();
            let (item_id, item_name, ask, market_id, stock_note) = if let Some(offer) = market_offer {
                (
                    offer.item_id,
                    offer.display_name,
                    offer.ask_mg,
                    Some(offer.source_market_id),
                    String::new(),
                )
            } else if let Some((cat_id, cat_name, mut cat_ask)) = canonical_offer {
                let active_ob = crate::store::get_active_npc_obligation(&target.id);
                let has_active_restock = active_ob
                    .as_ref()
                    .map(|(k, _)| k == "restock_goods")
                    .unwrap_or(false);
                let current_stock = get_shop_item_stock(&target.id, &cat_id);
                if !has_active_restock && current_stock <= 0 {
                    restock_shop_items_for_seller(&target.id);
                }
                let stock = get_shop_item_stock(&target.id, &cat_id);
                if stock <= 0 {
                    crate::store::open_active_npc_obligation(
                        &target.id,
                        "restock_goods",
                        &format!("店內【{}】已售罄急需提貨補倉", cat_name),
                    );
                    conn.send_json(&ActionResultMsg {
                        msg_type: "action_result".into(),
                        action: "Trade".into(),
                        target_id: target.id.clone(),
                        target_name: trade_target_name.clone(),
                        narrative: format!("【{trade_target_name}】看了一眼空蕩的貨架，向你抱歉地笑說：「實在不巧，店裡的【{}】現貨全被買空了，我正準備去總商行與聯運站提領新貨，勞駕稍候再來。」", cat_name),
                        success: true,
                        actions: vec![],
                        move_target_id: String::new(),
                        open_trade: false,
                        suggested_action: String::new(),
                        tactical_choices: vec![],
                    });
                    return false;
                }
                let note = if stock == 1 {
                    let bump = ((cat_ask as f64 * 0.15).round() as i32).max(1);
                    cat_ask += bump;
                    "（架上現貨僅餘最後 1 件，物稀價微揚）".to_string()
                } else {
                    format!("（店內現貨尚餘 {stock} 件）")
                };
                let quote = crate::game::object_ecology::dynamic_settlement_market_quote(&room_name, &cat_id, cat_ask);
                cat_ask = quote.unit_price;
                (cat_id, cat_name, cat_ask, None, note)
            } else {
                let item_id = personal_item.expect("personal or market offer checked");
                let item_name = get_item_def(&item_id)
                    .map(|definition| definition.name)
                    .or_else(|| {
                        crate::world::materials::material_by_item_id(&item_id)
                            .map(|m| m.name.to_string())
                    })
                    .or_else(|| {
                        crate::game::peninsula_catalog::find_catalog_entry(&item_id)
                            .map(|c| c.name.to_string())
                    })
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| {
                        if item_id.is_ascii() {
                            "隨身器物".to_string()
                        } else {
                            item_id.clone()
                        }
                    });
                let ask = default_trade_ask_mg(&item_id);
                let quote = crate::game::object_ecology::dynamic_settlement_market_quote(&room_name, &item_id, ask);
                (item_id, item_name, quote.unit_price, None, String::new())
            };
            let (floor, _trust, rel_note) = compute_relationship_trade_terms(pid, target_id, ask);
            trade_offer_set(
                pid,
                TradePending {
                    npc_id: target_id.to_string(),
                    item_id: item_id.clone(),
                    item_qty: 1,
                    ask_mg: ask,
                    floor_mg: floor,
                    market_id,
                    expires_unix: 0,
                },
            );
            let narrative = if market_id.is_some() {
                format!(
                    "【{trade_target_name}】查過此地市場現有的貨，願意交付「{item_name}」一份，開價 {ask} 鎂（可議至約 {floor} 鎂）{rel_note}。請輸入你願付的鎂數，或輸入「拒絕」取消。"
                )
            } else if is_canonical_offer {
                let stock_place = if target.id.starts_with("wt_vendor_") {
                    "攤車上的現貨"
                } else {
                    "店內架上的存貨"
                };
                format!(
                    "【{trade_target_name}】看了一下{stock_place}，願意賣出「{item_name}」一份，開價 {ask} 鎂（可議至約 {floor} 鎂）{stock_note}{rel_note}。請輸入你願付的鎂數，或輸入「拒絕」取消。"
                )
            } else {
                format!(
                    "【{trade_target_name}】願意從隨身備品中勻出「{item_name}」一份，開價 {ask} 鎂（可議至約 {floor} 鎂）{rel_note}。請輸入你願付的鎂數，或輸入「拒絕」取消。"
                )
            };
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Trade".into(),
                target_id: target.id.clone(),
                target_name: trade_target_name,
                narrative: {
                    let report = crate::game::object_ecology::format_settlement_supply_demand_report(&room_name);
                    if report.is_empty() {
                        narrative
                    } else {
                        format!("{narrative}\n{report}")
                    }
                },
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return false;
        }
    };
    if player_trade_input.is_empty() {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: "請輸入你願付的鎂數（整數），或輸入「拒絕」取消交易。".into(),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
    }
    let Ok(offer) = player_trade_input.parse::<i32>() else {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: "請輸入有效的鎂數（非負整數），或「拒絕」。".into(),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
    };
    if offer < 0 {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: "請輸入有效的鎂數（非負整數），或「拒絕」。".into(),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
    }
    let Ok(Some(buyer)) = db::get_entity(pid) else {
        conn.send_error(gametext::client("get_self_failed"));
        return true;
    };
    let paid = if offer >= pending.ask_mg {
        pending.ask_mg
    } else if offer >= pending.floor_mg {
        offer
    } else {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: format!(
                "【{trade_target_name}】搖頭：至少要 {} 鎂才肯點頭。",
                pending.floor_mg
            ),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
    };
    if buyer.magnesium < paid {
        let seller_id = &pending.npc_id;
        let (_floor, trust_score, _note) = compute_relationship_trade_terms(pid, seller_id, paid);
        let credit_outcome = crate::npc::debt_interaction::try_credit_purchase(
            pid,
            seller_id,
            &pending.item_id,
            paid,
            buyer.magnesium,
            trust_score,
            now / 60,
        );
        match credit_outcome {
            crate::npc::debt_interaction::CreditOutcome::PurchasedOnCredit {
                cash_paid,
                debt_created,
                ..
            }
            | crate::npc::debt_interaction::CreditOutcome::DebtCreated {
                upfront_paid: cash_paid,
                deferred_amount: debt_created,
                ..
            } => {
                if cash_paid > 0 {
                    let _ = db::add_magnesium(pid, -cash_paid);
                    let _ = db::add_magnesium(seller_id, cash_paid);
                }
                if pending.item_id.starts_with("wt_cat_") {
                    let _ = consume_shop_item_stock(seller_id, &pending.item_id, pending.item_qty);
                    let _ = db::add_to_inventory(pid, &pending.item_id, pending.item_qty);
                } else {
                    let _ = db::remove_from_inventory(seller_id, &pending.item_id, pending.item_qty);
                    let _ = db::add_to_inventory(pid, &pending.item_id, pending.item_qty);
                }
                let base_price = crate::world::materials::trade_price_anchor_mg(&pending.item_id)
                    .or_else(|| {
                        crate::game::peninsula_catalog::find_catalog_entry(&pending.item_id)
                            .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
                    })
                    .unwrap_or(paid)
                    .max(2);
                let player_room = super::resolve_action_room_name(pid, target);
                let _ = crate::game::object_ecology::record_settlement_market_trade(
                    &player_room,
                    &pending.item_id,
                    base_price,
                    -pending.item_qty,
                    "賒帳記契採買",
                );
                if let Some(pool) = crate::pg::pool() {
                    let _ = crate::pg::living_world::open_obligation(
                        &pool,
                        pid,
                        "repay_debt",
                        &format!("{}:{}mg", seller_id, debt_created),
                        None,
                        "credit_trade",
                        now,
                    );
                    let _ = crate::pg::living_world::record_relationship_delta(
                        &pool,
                        seller_id,
                        pid,
                        "traded",
                        1,
                        "player_trade_credit",
                        now,
                    );
                }
                let item_name = get_item_def(&pending.item_id)
                    .map(|d| d.name)
                    .filter(|n| !n.is_empty())
                    .or_else(|| {
                        crate::game::peninsula_catalog::find_catalog_entry(&pending.item_id)
                            .map(|c| c.name.to_string())
                    })
                    .unwrap_or_else(|| pending.item_id.clone());
                trade_offer_clear(pid, target_id);
                push_refresh(conn, pid);
                let _ = event::append(now, pid, types::TRADE, target_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!(
                        "你手頭現鎂僅有 {cash_paid} 鎂，【{trade_target_name}】念在彼此交情，允你先付現鎂、將差額 {debt_created} 鎂立契記帳賒欠，把【{item_name}】交予你手中。"
                    ),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return false;
            }
            _ => {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: "你的鎂不足，且交情尚淺未能立契賒欠，無法成交。".into(),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return false;
            }
        }
    }
    let settlement_result = if let Some(market_id) = pending.market_id {
        let Some(pool) = crate::pg::pool() else {
            trade_offer_clear(pid, target_id);
            conn.send_error("市場暫時無法結算，請稍後再試。");
            return true;
        };
        crate::pg::world_trade::settle_market_sale(
            &pool,
            pid,
            target_id,
            market_id,
            &pending.item_id,
            pending.item_qty,
            paid,
            now,
            "player_market_trade",
            None,
        )
        .map(|settlement| {
            mirror_market_trade_settlement(&settlement);
        })
    } else {
        let Ok(Some(seller)) = db::get_entity(target_id) else {
            trade_offer_clear(pid, target_id);
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Trade".into(),
                target_id: target.id.clone(),
                target_name: trade_target_name.clone(),
                narrative: format!("【{trade_target_name}】已不在可交易狀態，這筆交易作廢。"),
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return false;
        };
        if !inventory_has_qty(&seller.inventory, &pending.item_id, pending.item_qty) {
            if pending.item_id.starts_with("wt_cat_") {
                match consume_shop_item_stock(target_id, &pending.item_id, pending.item_qty) {
                    Ok(rem) => {
                        let item_name = get_item_def(&pending.item_id)
                            .map(|d| d.name)
                            .filter(|n| !n.is_empty())
                            .or_else(|| {
                                crate::game::peninsula_catalog::find_catalog_entry(&pending.item_id)
                                    .map(|c| c.name.to_string())
                            })
                            .unwrap_or_else(|| pending.item_id.clone());
                        if rem <= 1 {
                            crate::store::open_active_npc_obligation(
                                target_id,
                                "restock_goods",
                                &format!("店內【{}】現貨見底待補", item_name),
                            );
                        }
                        db::transfer_magnesium(pid, target_id, paid)
                            .or_else(|_| {
                                db::add_magnesium(pid, -paid)?;
                                let _ = db::add_magnesium(target_id, paid);
                                Ok(())
                            })
                            .and_then(|_| db::add_to_inventory(pid, &pending.item_id, pending.item_qty))
                            .map(|_| ())
                    }
                    Err(_) => {
                        trade_offer_clear(pid, target_id);
                        conn.send_json(&ActionResultMsg {
                            msg_type: "action_result".into(),
                            action: "Trade".into(),
                            target_id: target.id.clone(),
                            target_name: trade_target_name.clone(),
                            narrative: format!("【{trade_target_name}】抱歉地表示，該貨物方才已售罄無存，這筆交易作廢。"),
                            success: false,
                            actions: vec![],
                            move_target_id: String::new(),
                            open_trade: false,
                            suggested_action: String::new(),
                            tactical_choices: vec![],
                        });
                        return false;
                    }
                }
            } else {
                trade_offer_clear(pid, target_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("【{trade_target_name}】手上的貨已不在，這筆交易作廢。"),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return false;
            }
        } else {
            settle_inventory_trade(
                pid,
                target_id,
                &pending.item_id,
                pending.item_qty,
                paid,
                now,
                "player_trade",
            )
            .map(|_| ())
            .or_else(|_| {
                db::transfer_magnesium(pid, target_id, paid)
                    .or_else(|_| {
                        db::add_magnesium(pid, -paid)?;
                        let _ = db::add_magnesium(target_id, paid);
                        Ok(())
                    })
                    .and_then(|_| db::remove_from_inventory(target_id, &pending.item_id, pending.item_qty))
                    .and_then(|_| db::add_to_inventory(pid, &pending.item_id, pending.item_qty))
            })
        }
    };
    if settlement_result.is_err() {
        trade_offer_clear(pid, target_id);
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: "成交時交易結算失敗，請重新確認對方貨物與你的鎂。".into(),
            success: false,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
    }
    if let Ok(Some(buyer_after)) = db::get_entity(pid) {
        let expected_max_mg = (buyer.magnesium - paid).max(0);
        if buyer_after.magnesium > expected_max_mg {
            let diff = buyer_after.magnesium - expected_max_mg;
            let _ = db::add_magnesium(pid, -diff);
        }
    }
    let base_price = crate::world::materials::trade_price_anchor_mg(&pending.item_id)
        .or_else(|| {
            crate::game::peninsula_catalog::find_catalog_entry(&pending.item_id)
                .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
        })
        .or_else(|| {
            for fg in &crate::game::object_ecology::STANDARD_FINISHED_GOODS {
                if fg.id == pending.item_id {
                    return Some(fg.base_price);
                }
            }
            None
        })
        .unwrap_or(pending.ask_mg)
        .max(2);
    let player_room = super::resolve_action_room_name(pid, target);
    let _ = crate::game::object_ecology::record_settlement_market_trade(
        &player_room,
        &pending.item_id,
        base_price,
        -pending.item_qty,
        "玩家採買",
    );
    trade_offer_clear(pid, target_id);
    let _ = db::adjust_favorability(target_id, pid, 2);
    let _ = db::add_thought(target_id, crate::db::ThoughtSource::Trade);
    if let Some(pool) = &crate::pg::pool() {
        let _ = crate::pg::living_world::record_relationship_delta(
            pool,
            target_id,
            pid,
            "traded",
            1,
            "player_trade",
            now,
        );
        let _ = crate::pg::living_world::resolve_obligations_by_kind(
            pool,
            target_id,
            "stranded_cargo",
            now,
        );
        if crate::npc::debt_interaction::active_debt_amount(pid) == 0 {
            let _ = crate::pg::living_world::resolve_obligations_by_kind(
                pool,
                pid,
                "repay_debt",
                now,
            );
        }
        let player_room = get_entity_room(pid).unwrap_or_default();
        let _ = crate::pg::living_world::record_player_world_action(
            pool,
            pid,
            "trade",
            target_id,
            &player_room,
            "success",
            serde_json::json!({
                "item_id": pending.item_id,
                "paid_mg": paid,
                "ask_mg": pending.ask_mg,
                "offer_mg": offer,
            }),
            now,
        );
    }
    let item_name = get_item_def(&pending.item_id)
        .map(|d| d.name)
        .or_else(|| {
            crate::world::materials::material_by_item_id(&pending.item_id)
                .map(|m| m.name.to_string())
        })
        .or_else(|| {
            crate::game::peninsula_catalog::find_catalog_entry(&pending.item_id)
                .map(|c| c.name.to_string())
        })
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            if pending.item_id.is_ascii() {
                "隨身器物".to_string()
            } else {
                pending.item_id.clone()
            }
        });
    let stock_suffix = if pending.item_id.starts_with("wt_cat_") {
        let rem = get_shop_item_stock(target_id, &pending.item_id);
        if rem <= 0 {
            "（這是店裡最後一件現貨，店家隨即準備動身去補貨）".to_string()
        } else {
            if target_id.starts_with("wt_vendor_") {
                format!("（成交後攤上現貨尚餘 {rem} 件）")
            } else {
                format!("（成交後店內該貨尚餘 {rem} 件）")
            }
        }
    } else {
        String::new()
    };
    let narrative = if offer >= pending.ask_mg {
        format!("你以開價 {paid} 鎂買下「{item_name}」。{stock_suffix}")
    } else {
        format!("一番議價後，你以 {paid} 鎂買下「{item_name}」。{stock_suffix}")
    };
    let _ = event::append(now, pid, types::TRADE, target_id);
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "Trade".into(),
        target_id: target.id.clone(),
        target_name: trade_target_name,
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
    push_refresh(conn, pid);
    false
}

pub(super) fn is_eligible_shop_staff(target: &Character, room_name: &str) -> bool {
    let t_id = target.id.as_str();
    let title = target.display_title.as_str();

    // 衝突外單位職稱或訪客身份，嚴禁代表商鋪賣店貨
    const CONFLICTING_KEYWORDS: &[&str] = &[
        "學生", "學員", "學子", "童生", "生員",
        "編輯助理", "編輯", "校勘", "紀錄員", "記錄員", "抄寫員", "書僮", "書童",
        "訪客", "遊客", "香客", "旅人", "食客", "讀者", "信眾", "遊俠", "客商", "外來者",
    ];
    if CONFLICTING_KEYWORDS.iter().any(|k| title.contains(k) || t_id.contains(k)) {
        return false;
    }

    // 檢查若有指定外單位機構前綴且與當前場所衝突
    const ORG_PREFIXES: &[&str] = &[
        "藏書閣", "文曜學院", "公議堂", "東橋旅社", "三水大食堂",
        "三水總商行", "泉香溫水浴場", "溫氏診所", "岑氏鍛造", "大圳織造",
        "風舶聯運", "水尾寮早市", "算學舍", "鐘樓", "蒙學堂", "觀星台", "習武堂", "剪剃鋪", "柑仔店", "商務會計教室", "文曜國小",
    ];
    for org in ORG_PREFIXES {
        if (title.contains(org) || t_id.contains(org)) && !room_name.contains(org) {
            return false;
        }
    }

    // 檢查居民 household 排班之工作場所是否為衝突外單位
    let slot = crate::store::wenyao_household_slot_for(&target.id);
    if !slot.work_name.is_empty() {
        let work = slot.work_name;
        let is_same_venue = room_name.contains(work) || work.contains(room_name)
            || (room_name.contains("商行") && work.contains("商行"))
            || (room_name.contains("旅社") && (work.contains("旅社") || work.contains("茶屋") || work.contains("客棧") || work.contains("歇舍") || work.contains("臨水")))
            || (room_name.contains("食堂") && work.contains("食堂"))
            || (room_name.contains("工坊") && work.contains("工坊"))
            || (room_name.contains("診所") && work.contains("診所"))
            || (room_name.contains("浴場") && work.contains("浴場"));
        if !is_same_venue && (work.contains("學院") || work.contains("藏書閣") || work.contains("公議堂")
            || work.contains("學堂") || work.contains("診所") || work.contains("官署") || work.contains("巡防")) {
            return false;
        }
    }

    // 正向判定：具備店長、店員、幫工、大廚等商鋪職銜，或工作場所相符
    const STAFF_KEYWORDS: &[&str] = &[
        "掌事", "店主", "店長", "店員", "店助", "助理", "幫工", "學徒", "師傅",
        "匠", "主事", "管事", "大廚", "廚工", "跑堂", "侍者", "侍應", "服務生", "販", "商",
        "茶師", "藥師", "工", "役",
    ];
    if STAFF_KEYWORDS.iter().any(|k| title.contains(k) || t_id.contains(k)) {
        return true;
    }
    if !slot.work_name.is_empty() && (room_name.contains(slot.work_name) || slot.work_name.contains(room_name)) {
        return true;
    }
    let pers_role = crate::store::npc_persistent_role_label(&target.id, &target.display_title);
    if STAFF_KEYWORDS.iter().any(|k| pers_role.contains(k)) {
        return true;
    }

    // 若未被判定為衝突職稱，且無其他工作衝突
    if slot.work_name.is_empty() && !super::is_canonical_named_npc(target) {
        return true;
    }

    false
}

pub(super) fn canonical_shopkeeper_trade_offer(
    target: &Character,
    room_name: &str,
    now: i64,
) -> Option<(String, String, i32)> {
    let t_id = target.id.as_str();
    let title = target.display_title.as_str();
    let title_short = title.rsplit(['|', '｜']).next().unwrap_or(title).trim();

    let hour = crate::store::current_game_hour();
    let is_always_open = t_id.contains("沈清河")
        || title.contains("沈清河")
        || room_name.contains("客棧")
        || room_name.contains("旅社")
        || room_name.contains("風舶臨水旅社")
        || room_name.contains("北灣臨水旅社")
        || room_name.contains("臨水旅社")
        || room_name.contains("歇舍");
    let in_hours = if is_always_open {
        true
    } else if t_id.starts_with("wt_vendor_") {
        !target.current_activity.contains("安歇")
    } else {
        (6..=21).contains(&hour)
    };
    if !in_hours {
        return None;
    }

    if t_id != "wt_npc_ma_buxue" {
        if let Some((item_id, item_name, price_mg, _)) = vendor_stall_offer_for_npc(t_id) {
            return Some((item_id.to_string(), item_name.to_string(), price_mg));
        }
    }

    let has = |key: &str| -> bool {
        t_id == key
            || t_id.contains(key)
            || title.contains(key)
            || title_short == key
            || title_short.contains(key)
    };
    // 1. 陸承禮 / 契書局
    if t_id == "wt_npc_lu_chengli"
        || t_id.contains("陸承禮")
        || title.contains("陸承禮")
        || (room_name.contains("契書") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0793", "公議堂紅印屋契與租約摺", 25),
            ("wt_cat_0821", "玻璃螺紋瓶裝黑墨水", 6),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 2. 陸長貴 / 公議堂
    if t_id == "wt_npc_lu_changgui"
        || t_id.contains("陸長貴")
        || title.contains("陸長貴")
        || (room_name.contains("公議堂") && !room_name.contains("契書") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0793", "公議堂紅印屋契與租約摺", 25),
            ("wt_cat_0786", "每旬發行《半島旬報》", 3),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 3. 陸秉衡 / 三水總商行
    if t_id.contains("陸秉衡")
        || title.contains("陸秉衡")
        || (room_name.contains("總商行") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0790", "三水總商行鎂幣分格存摺", 12),
            ("wt_cat_0789", "拉鍊帆布零錢包與鎂幣夾", 8),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 4. 陳萬福 / 三水大食堂
    if t_id.contains("陳萬福")
        || title.contains("陳萬福")
        || (room_name.contains("大食堂") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0493", "三水食堂紅鐵大釜熬大角原牛骨湯", 5),
            ("wt_cat_0481", "老堰頭石磨現磨溫米漿", 3),
            ("wt_cat_0482", "木桶鹽滷嫩豆花", 4),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 5. 沈清河 / 東橋旅社與茶屋
    if t_id.contains("沈清河")
        || title.contains("沈清河")
        || (room_name.contains("東橋旅社") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0500", "東橋茶屋山風草配霜林烘青茶", 5),
            ("wt_cat_0501", "穩火慢煮牛埔鮮奶茶", 5),
            ("wt_cat_0723", "旋蓋夜光貝母調光台燈", 15),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 6. 白泉生 / 泉香溫水浴場
    if t_id.contains("白泉生")
        || title.contains("白泉生")
        || (room_name.contains("浴場") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0618", "北山茶籽配松脂香皂", 4),
            ("wt_cat_0619", "大圳長絨蜂格棉面巾", 5),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 7. 溫晴、溫懷瑾 / 溫氏脈絡診所
    if t_id.contains("溫晴")
        || title.contains("溫晴")
        || t_id.contains("溫懷瑾")
        || title.contains("溫懷瑾")
        || (room_name.contains("診所") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0742", "溫氏山風草舒筋活絡藥膏", 8),
            ("wt_cat_0745", "隨身防潮急救木匣", 12),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 8. 岑守岳 / 岑氏第一鍛造工坊
    if t_id.contains("岑守岳")
        || title.contains("岑守岳")
        || (room_name.contains("鍛造工坊") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0831", "岑氏精鋼打磨護手獵刀", 18),
            ("wt_cat_0801", "三號坑精鍛羊角錘", 10),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 9. 岑拓 / 岑氏第一鍛造工坊青年匠師
    if has("岑拓") || t_id == "wt_npc_cen_tuo" {
        let items = [
            ("wt_cat_0612", "岑氏合金黃銅自來水栓", 12),
            ("wt_cat_0957", "柯氏松木家用工具箱", 14),
            ("wt_cat_1011", "分家巷卡式導能便攜爐", 16),
            ("wt_cat_0736", "分家巷導能測紋石板", 15),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 10. 柯成器 / 木作傢俱工坊
    if has("柯成器")
        || t_id == "wt_npc_ke_chengqi"
        || (room_name.contains("木作") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0560", "寒水玉雙層保鮮石櫃", 35),
            ("wt_cat_0724", "旋蓋夜光貝母調光台燈", 15),
            ("wt_cat_0796", "烏木黑子白貝圍棋盤", 20),
            ("wt_cat_0623", "防靜電松香木梳", 5),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 11. 蘇錦心 / 織造服飾坊
    if has("蘇錦心")
        || t_id == "wt_npc_su_jinxin"
        || (room_name.contains("織造") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0561", "大圳細棉麻透氣常服", 14),
            ("wt_cat_0572", "黑沼樹膠防雨連帽風衣", 18),
            ("wt_cat_0590", "文曜素面玄武岩扣束腰長袍", 16),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 12. 秦明誠 / 百工商算學舍
    if has("秦明誠")
        || t_id == "wt_npc_qin_mingcheng"
        || ((room_name.contains("算學") || room_name.contains("會計")) && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0788", "烏木邊框便攜計數板", 14),
            ("wt_cat_0782", "竹綿紙牛皮記事手札", 8),
            ("wt_cat_0790", "三水總商行鎂幣分格存摺", 12),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 13. 林時中 / 傳訊鐘樓司鐘長
    if has("林時中")
        || t_id == "wt_npc_lin_shizhong"
        || (room_name.contains("鐘樓") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0730", "三水口校準隨身黃銅懷錶", 18),
            ("wt_cat_0729", "岑氏黃銅發條座鐘", 24),
            ("wt_cat_0795", "紅松脂封蠟行會委託單", 6),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 14. 沈硯舟、陸知微 / 書局、藏書閣
    if has("沈硯舟")
        || t_id == "wt_npc_shen_yanzhou"
        || has("陸知微")
        || t_id == "wt_npc_lu_zhiwei"
        || ((room_name.contains("書局") || room_name.contains("藏書閣"))
            && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0785", "《白塔風物誌》竹綿紙精印本", 8),
            ("wt_cat_0786", "每旬發行《半島旬報》", 3),
            ("wt_cat_0781", "彈性精鋼尖沾水筆", 10),
            ("wt_cat_0782", "竹綿紙牛皮記事手札", 8),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 15. 陸敬軒 / 學院蒙學堂講習舍
    if has("陸敬軒")
        || t_id == "wt_npc_lu_jingxuan"
        || ((room_name.contains("蒙學") || room_name.contains("國小")) && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0783", "國小基礎國語課本", 6),
            ("wt_cat_0782", "竹綿紙牛皮記事手札", 8),
            ("wt_cat_0781", "彈性精鋼尖沾水筆", 10),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 13. 趙觀星 / 觀星台
    if t_id == "wt_npc_zhao_guanxing"
        || t_id.contains("趙觀星")
        || title.contains("趙觀星")
        || (room_name.contains("觀星台") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0735", "黃銅隨身星象指北羅盤", 15),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 14. 岑雲野 / 商算學舍 / 習武堂
    if t_id == "wt_npc_cen_yunye"
        || t_id.contains("岑雲野")
        || title.contains("岑雲野")
        || (room_name.contains("習武堂") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_1015", "包革練習重木刀", 14),
            ("wt_cat_0846", "熟銅卸力護腕", 12),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 15. 周一刀 / 精鋼剪剃鋪
    if t_id == "wt_npc_zhou_yidao"
        || t_id.contains("周一刀")
        || title.contains("周一刀")
        || (room_name.contains("剪剃鋪") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0630", "精鋼摺疊修容剃刀", 12),
            ("wt_cat_0628", "牛骨柄豬鬃軟毛牙刷", 4),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 16. 許阿甘 / 柑仔店
    if t_id == "wt_npc_xu_agan"
        || t_id.contains("許阿甘")
        || title.contains("許阿甘")
        || (room_name.contains("柑仔店") && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0512", "廣口玻璃罐裝金桔酸梅糖", 3),
            ("wt_cat_0645", "防潮紅磷火柴盒與艾草蚊香", 3),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 17. 馬補靴 / 修傘補靴攤
    if t_id == "wt_npc_ma_buxue"
        || t_id.contains("馬補靴")
        || title.contains("馬補靴")
        || room_name.contains("修傘")
        || room_name.contains("補靴")
    {
        let items = [
            ("wt_cat_0608", "韌竹骨桐油棉紙摺疊晴雨傘", 8),
            ("wt_cat_0585", "厚魚油防浸牛皮行路靴", 15),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // 18. 韓鐵山 / 聯運總站、韓氏貨運行
    if has("韓鐵山")
        || t_id == "wt_npc_han_tieshan"
        || ((room_name.contains("聯運") || room_name.contains("貨運") || room_name.contains("車隊"))
            && is_eligible_shop_staff(target, room_name))
    {
        let items = [
            ("wt_cat_0605", "雙肩厚帆布遠行背囊", 12),
            ("wt_cat_0535", "雙層烏木內膽隨身保溫瓶", 10),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }
    if !is_shop_person(target, room_name) {
        return None;
    }

    // ── Food, Tea, Wine & Groceries ──
    if room_name.contains("鮮魚")
        || room_name.contains("魚市")
        || room_name.contains("三水河鮮")
        || room_name.contains("早市鮮魚拍賣場")
        || room_name.contains("風舶早市菜場")
        || room_name.contains("北灣早市菜場")
    {
        let items = [
            ("wt_cat_0224", "銀鱗土魠魚", 8),
            ("wt_cat_0226", "淺澤青殼河蝦", 6),
            ("wt_cat_0227", "活水肥美草魚", 7),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("乾貨")
        || room_name.contains("海味")
        || room_name.contains("鹽漬")
        || room_name.contains("水產乾貨")
    {
        let items = [
            ("wt_cat_0319", "海風脫水小蝦米", 6),
            ("wt_cat_0320", "乾製海生干貝柱", 10),
            ("wt_cat_0228", "礁石野生紫菜", 5),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("醬園")
        || room_name.contains("醋坊")
        || room_name.contains("陳記醬園")
    {
        let items = [
            ("wt_cat_0346", "熟成野山果醋", 5),
            ("wt_cat_0347", "老罈烏醋香料汁", 6),
            ("wt_cat_0338", "日曬白霜細海鹽", 4),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("酒肆")
        || room_name.contains("紅爐烈酒")
        || room_name.contains("烈酒大飯鋪")
        || room_name.contains("沈家老酒")
        || room_name.contains("老酒坊")
        || room_name.contains("盤查酒肆")
    {
        let items = [
            ("wt_cat_0506", "封缸陳年醇米酒", 12),
            ("wt_cat_0507", "蜜漬山果甜釀", 8),
            ("wt_cat_0444", "厚釜慢燉原牛肉湯", 10),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("羊肉湯餅")
        || room_name.contains("麵食")
        || room_name.contains("西門麵食鋪")
        || room_name.contains("乾糧")
        || room_name.contains("乾糧備製作坊")
    {
        let items = [
            ("wt_cat_0311", "鹽醃草飼羊肉條", 10),
            ("wt_cat_0284", "竹篩日曬摺疊麵", 4),
            ("wt_cat_0285", "手工日曬粗麵線", 5),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("茶棚")
        || room_name.contains("茶飯館")
        || room_name.contains("水手茶飯館")
        || room_name.contains("工匠大食堂")
        || room_name.contains("大食堂")
        || room_name.contains("食堂")
    {
        let items = [
            ("wt_cat_0394", "切仔湯麵配滷蛋", 5),
            ("wt_cat_0392", "鮮肉大包子", 4),
            ("wt_cat_0500", "山風草炭焙烘青茶", 4),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("鹽糧")
        || room_name.contains("糧倉")
        || room_name.contains("通濟河運總倉")
        || room_name.contains("戰備糧草")
        || room_name.contains("儲倉")
        || room_name.contains("草料行")
        || room_name.contains("鹽巴草料行")
        || room_name.contains("粗鹽")
    {
        let items = [
            ("wt_cat_0338", "日曬白霜細海鹽", 4),
            ("wt_cat_0284", "竹篩日曬摺疊麵", 4),
            ("wt_cat_0277", "秋收蓬萊新米", 6),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // ── Paper, Ink, Books, Maps & Stationery ──
    if room_name.contains("宣紙")
        || room_name.contains("青檀宣紙")
        || room_name.contains("造紙")
        || room_name.contains("竹綿造紙精廬")
        || room_name.contains("紙料儲庫")
        || room_name.contains("善本紙料")
    {
        let items = [
            ("wt_cat_0779", "竹綿印刷紙", 6),
            ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("徽墨")
        || room_name.contains("松煙徽墨")
        || room_name.contains("端溪石硯")
        || room_name.contains("石硯")
        || room_name.contains("宣筆")
        || room_name.contains("制毫")
        || room_name.contains("筆墨文具")
    {
        let items = [
            ("wt_cat_0821", "玻璃螺紋瓶裝黑墨水", 6),
            ("wt_cat_0780", "彈性精鋼尖沾水筆", 10),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("活字")
        || room_name.contains("印書")
        || room_name.contains("印行")
        || room_name.contains("裝幀")
        || room_name.contains("古籍裝幀")
        || room_name.contains("裱褙")
        || room_name.contains("文房四寶")
        || room_name.contains("藏書")
        || room_name.contains("書局")
        || room_name.contains("書肆")
        || room_name.contains("典藏庫")
        || room_name.contains("圖書典藏庫")
    {
        let items = [
            ("wt_cat_0784", "《白塔風物誌》", 8),
            ("wt_cat_0786", "每旬發行《半島旬報》", 3),
            ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("輿圖")
        || room_name.contains("格物輿圖")
        || room_name.contains("測風")
        || room_name.contains("北辰測風")
        || room_name.contains("會計")
        || room_name.contains("國小")
        || room_name.contains("算學")
        || room_name.contains("曆算")
        || room_name.contains("觀星台")
        || room_name.contains("學院")
        || room_name.contains("蒙學堂")
    {
        let items = [
            ("wt_cat_0785", "航海圖冊《風舶航海錄》", 15),
            ("wt_cat_0731", "高空青銅測風儀", 18),
            ("wt_cat_0788", "烏木邊框便攜計數板", 14),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("斫琴")
        || room_name.contains("古琴")
        || room_name.contains("棋院")
        || room_name.contains("文曜棋院")
        || room_name.contains("鑑藏")
        || room_name.contains("古物鑑藏齋")
    {
        let items = [
            ("wt_cat_0795", "烏木黑子白貝圍棋盤", 20),
            ("wt_cat_0796", "松香木雕刻象棋組", 16),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // ── Medicine, Herbs & Incense ──
    if room_name.contains("杏林百草醫館")
        || room_name.contains("百草醫館")
        || room_name.contains("醫館")
        || room_name.contains("診所")
        || room_name.contains("採藥棧")
        || room_name.contains("採藥")
        || room_name.contains("藥材行")
        || room_name.contains("藥材中轉站")
        || room_name.contains("百草藥居")
        || room_name.contains("脈絡診所")
    {
        let items = [
            ("wt_cat_1054", "拔毒止血水苔敷藥", 10),
            ("wt_cat_1055", "白茫避瘴清心丸", 12),
            ("wt_cat_1060", "溫氏九寸脈絡銀針囊", 16),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("烘藥房")
        || room_name.contains("百草烘藥房")
        || room_name.contains("香燭")
        || room_name.contains("山風草香燭鋪")
        || room_name.contains("浴場")
        || room_name.contains("溫水浴場")
    {
        let items = [
            ("wt_cat_1057", "溫氏導能溫石百草溫灸盒", 14),
            ("wt_cat_1058", "驅蚊艾草香盤", 6),
            ("wt_cat_0119", "山野清香薄荷葉", 4),
            ("wt_cat_1056", "祛寒草木湯包", 6),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // ── Forge, Hardware, Tools, Timber, Stone, Kiln & Craft ──
    if room_name.contains("重鐵水錘")
        || room_name.contains("大鍛坊")
        || room_name.contains("鍛造")
        || room_name.contains("鍛修")
        || room_name.contains("車軸鐵箍")
        || room_name.contains("磨刀")
        || room_name.contains("磨刀利刃")
        || room_name.contains("鐵蹄釘掌")
        || room_name.contains("釘掌鋪")
    {
        let items = [
            ("wt_cat_0945", "岑氏省力導能鍛錘", 18),
            ("wt_cat_0831", "岑氏精鋼打磨護手獵刀", 18),
            ("wt_cat_0801", "精鍛羊角錘", 10),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("農具")
        || room_name.contains("大型農具工具行")
        || room_name.contains("五金")
        || room_name.contains("銅鐵五金")
        || room_name.contains("鎖鑰")
        || room_name.contains("鎖簧")
        || room_name.contains("器物交易場")
        || room_name.contains("交易場")
    {
        let items = [
            ("wt_cat_0950", "省力導能翻土犁", 16),
            ("wt_cat_0685", "熟銅門把帶重鐵碰鎖", 10),
            ("wt_cat_0707", "青銅插銷暗扣雙重鎖", 12),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("武備")
        || room_name.contains("護商武備鋪")
        || room_name.contains("防具")
        || room_name.contains("火浣布防具坊")
        || room_name.contains("火政")
        || room_name.contains("東門火政巡防所")
        || room_name.contains("習武堂")
    {
        let items = [
            ("wt_cat_0846", "熟銅卸力護腕", 14),
            ("wt_cat_0840", "鍛造坊厚牛皮防燙圍裙", 16),
            ("wt_cat_0572", "樹膠防雨連帽風衣", 18),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("鐘錶")
        || room_name.contains("儀器坊")
        || room_name.contains("精銅鐘錶儀器坊")
        || room_name.contains("琉璃燈")
        || room_name.contains("白塔琉璃燈坊")
        || room_name.contains("鐘樓")
        || room_name.contains("傳訊鐘樓")
    {
        let items = [
            ("wt_cat_0730", "校準隨身黃銅懷錶", 18),
            ("wt_cat_0729", "岑氏黃銅發條座鐘", 24),
            ("wt_cat_0724", "旋蓋夜光貝母調光台燈", 15),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("鋸木廠")
        || room_name.contains("連軸鋸木廠")
        || room_name.contains("竹木行")
        || room_name.contains("三水竹木行")
        || room_name.contains("竹木場")
        || room_name.contains("蘆葦竹木場")
        || room_name.contains("竹編")
        || room_name.contains("藤器")
        || room_name.contains("竹編藤器作坊")
        || room_name.contains("木作")
        || room_name.contains("傢俱")
        || room_name.contains("巧木居所")
        || room_name.contains("榫眼")
        || room_name.contains("木器榫眼作坊")
        || room_name.contains("大木作")
        || room_name.contains("車轅大木作")
        || room_name.contains("大木車船坊")
    {
        let items = [
            ("wt_cat_0947", "水力原木大鋸", 20),
            ("wt_cat_0895", "雙層竹編提籃", 8),
            ("wt_cat_0623", "防靜電松香木梳", 5),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("石材切割場")
        || room_name.contains("切石")
        || room_name.contains("磚瓦窯")
        || room_name.contains("灰漿磚瓦窯")
        || room_name.contains("木炭窯")
        || room_name.contains("高山木炭窯")
        || room_name.contains("官窯")
        || room_name.contains("青釉官窯")
        || room_name.contains("銅錫")
        || room_name.contains("器皿精煉廠")
        || room_name.contains("生鐵大棧房")
        || room_name.contains("焦炭生鐵大棧房")
        || room_name.contains("生鐵庫")
        || room_name.contains("粗鹽生鐵庫")
    {
        let items = [
            ("wt_cat_0949", "銀砂水力切石機", 22),
            ("wt_cat_0944", "紅土耐火磚高溫鐵爐", 20),
            ("wt_cat_0097", "相思木硬炭", 6),
            ("wt_cat_0023", "粗陶紅黏土", 5),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("染布坊")
        || room_name.contains("三水染布坊")
        || room_name.contains("機杼大染坊")
        || room_name.contains("染纈")
        || room_name.contains("織造")
        || room_name.contains("雲錦織造與服飾坊")
        || room_name.contains("服飾坊")
        || room_name.contains("剪剃鋪")
        || room_name.contains("修傘補靴")
        || room_name.contains("修傘")
        || room_name.contains("補靴")
    {
        let items = [
            ("wt_cat_0835", "細棉麻透氣常服", 14),
            ("wt_cat_0561", "細棉麻透氣常服", 14),
            ("wt_cat_0590", "素面玄武岩扣束腰長袍", 16),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("造船廠")
        || room_name.contains("內河造船廠")
        || room_name.contains("修船塢")
        || room_name.contains("渡口修船塢")
        || room_name.contains("桅帆")
        || room_name.contains("大船桅帆作坊")
        || room_name.contains("帆布麻繩織造坊")
        || room_name.contains("漁網繩索工坊")
        || room_name.contains("修網結繩工坊")
        || room_name.contains("漁具")
        || room_name.contains("簑笠")
        || room_name.contains("漁具簑笠商號")
        || room_name.contains("桐油作坊")
        || room_name.contains("船用桐油作坊")
        || room_name.contains("浮標竹笛作坊")
        || room_name.contains("浮標")
    {
        let items = [
            ("wt_cat_1000", "深海巨藻船纜", 18),
            ("wt_cat_1002", "桐油海貝灰捻縫灰膏", 8),
            ("wt_cat_1003", "極淺吃水平底竹筏", 25),
            ("wt_cat_1006", "老青竹黃銅輪捲線釣竿", 10),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    // ── Caravan, Leather, Livestock, Salt/Iron & Inns / Docks ──
    if room_name.contains("駝馬大驛站")
        || room_name.contains("萬里駝隊大驛站")
        || room_name.contains("駝馬大驛館")
        || room_name.contains("駝馬大客棧")
        || room_name.contains("長途行商旅社")
        || room_name.contains("長途行商客棧")
        || room_name.contains("客棧")
        || room_name.contains("旅社")
        || room_name.contains("風塵客商大旅社")
        || room_name.contains("東橋旅社")
        || room_name.contains("客舍")
        || room_name.contains("青雲求學客舍")
        || room_name.contains("歇舍")
        || room_name.contains("行商長途歇舍")
        || (room_name.contains("行館") && !room_name.contains("印行館"))
        || room_name.contains("四方匠師行館")
        || room_name.contains("泊客棧舍")
        || room_name.contains("風舶臨水旅社")
        || room_name.contains("北灣臨水旅社")
        || room_name.contains("臨水旅社")
        || room_name.contains("茶屋")
    {
        let items = [
            ("wt_cat_0891", "黃銅雙扣厚帆布斜背包", 12),
            ("wt_cat_0897", "薄層重礦烏木膽隨身保溫瓶", 10),
            ("wt_cat_1037", "防倒風野營生鐵柴火爐", 15),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("皮毛大市")
        || room_name.contains("西南原皮毛大市")
        || room_name.contains("皮毛大貨棧")
        || room_name.contains("皮貨行")
        || room_name.contains("西門皮貨行")
        || room_name.contains("皮帶鞍套")
        || room_name.contains("皮鞘鞍具")
        || room_name.contains("鞍轡")
        || room_name.contains("遠行鞍轡帳篷行")
        || room_name.contains("帳篷")
        || room_name.contains("行囊修補行")
        || room_name.contains("行囊")
        || room_name.contains("行車雨布作坊")
        || room_name.contains("雨布")
    {
        let items = [
            ("wt_cat_0836", "飛鼠厚絨禦寒冬衣", 22),
            ("wt_cat_0837", "雲豹毛領禦寒大氅", 28),
            ("wt_cat_0892", "軟牛皮分格零錢夾", 8),
            ("wt_cat_1040", "平底木桶式雙槳划艇", 20),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("雙輪大車廠")
        || room_name.contains("重載雙輪大車廠")
        || room_name.contains("大車修造場")
        || room_name.contains("車馬輪軸修造場")
        || room_name.contains("長途車行")
        || room_name.contains("西道長途車行")
        || room_name.contains("車馬聯運總站")
        || room_name.contains("聯運貨運行")
        || room_name.contains("聯運")
        || room_name.contains("貨運行")
        || room_name.contains("油膏滑脂鋪")
        || room_name.contains("滑脂")
    {
        let items = [
            ("wt_cat_1004", "韓氏雙輪駝獸大貨車", 30),
            ("wt_cat_0988", "重載輪軸手動液壓千斤頂", 18),
            ("wt_cat_0985", "駝獸生鐵打蹄鉗與平口銼", 12),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("牲畜交易拍賣行")
        || room_name.contains("牲畜拍賣")
        || room_name.contains("獸醫館")
        || room_name.contains("車馬獸醫館")
    {
        let items = [
            ("wt_cat_1020", "手搖螺旋壓實乾草料壓捆機", 16),
            ("wt_cat_0969", "重型生鐵熟牛皮防撞砧", 15),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    if room_name.contains("貨船大埠")
        || room_name.contains("川流貨船大埠")
        || room_name.contains("北灣貨船大埠")
        || room_name.contains("碼頭匯兌分號")
        || room_name.contains("碼頭匯兌")
        || room_name.contains("巡河哨所")
        || room_name.contains("河工巡防哨居")
        || room_name.contains("哨所")
        || room_name.contains("長途保鏢哨所")
        || room_name.contains("四方護運行總堂")
        || room_name.contains("四方鏢局總堂")
        || room_name.contains("護運行")
        || room_name.contains("鏢局")
        || room_name.contains("關防驗契所")
        || room_name.contains("驗契")
        || room_name.contains("公議堂")
        || room_name.contains("契書")
        || room_name.contains("總商行")
        || room_name.contains("柑仔店")
    {
        let items = [
            ("wt_cat_0790", "三水總商行鎂幣分格存摺", 12),
            ("wt_cat_0789", "拉鍊帆布零錢包與鎂幣夾", 8),
            ("wt_cat_0605", "雙肩厚帆布遠行背囊", 12),
        ];
        let idx = ((now.unsigned_abs() as usize) / 60) % items.len();
        let (id, name, price) = items[idx];
        return Some((id.to_string(), name.to_string(), price));
    }

    None
}

pub(super) fn is_shop_person(target: &Character, room_name: &str) -> bool {
    if super::is_canonical_named_npc(target) {
        return true;
    }
    is_eligible_shop_staff(target, room_name)
}

pub(super) fn target_has_shop_offer(target: &Character, room_name: &str) -> bool {
    let hour = crate::store::current_game_hour();
    let is_always_open = target.id.contains("沈清河")
        || target.display_title.contains("沈清河")
        || room_name.contains("客棧")
        || room_name.contains("旅社")
        || room_name.contains("風舶臨水旅社")
        || room_name.contains("北灣臨水旅社")
        || room_name.contains("臨水旅社")
        || room_name.contains("歇舍");
    let in_hours = if is_always_open {
        true
    } else if target.id.starts_with("wt_vendor_") {
        (6..=22).contains(&hour)
    } else {
        (6..=21).contains(&hour)
    };
    if !in_hours {
        return false;
    }
    canonical_shopkeeper_trade_offer(target, room_name, 0).is_some()
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
}
