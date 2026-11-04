//! `do_action` — Trade 分支：NPC 報價、玩家議價、鎂轉帳 + inventory 轉移。

use std::collections::HashMap;
use std::sync::Mutex;
use crate::db::{
    self, TradePending, default_trade_ask_mg, get_entity_room, get_item_def, inventory_has_qty,
    mirror_market_trade_settlement, pick_entity_trade_offer, settle_inventory_trade,
    trade_floor_from_ask, trade_offer_clear, trade_offer_get, trade_offer_set,
};
use crate::entity::Character;
use crate::event::{self, types};
use crate::gametext;

use super::super::handler::{WsConnection, push_refresh};
use super::super::protocol::{ActionResultMsg, ClientMsg};

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

/// 回傳 true 表示已 sendError，呼叫端應停止。
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
                    || crate::game::object_ecology::lookup_ecology_item_def(&it.item_id).is_some_and(|e| e.name.contains(raw_query))
                    || crate::db::fallback_item_name(&it.item_id).is_some_and(|n| n.contains(raw_query))
            })
        } else {
            inv.items.into_iter().find(|it| {
                it.item_id.starts_with("mat_")
                    || it.item_id.starts_with("harvest_")
                    || it.item_id.starts_with("craft_")
                    || it.item_id.starts_with("wt_cat_")
                    || it.item_id.starts_with("wt-good-")
                    || it.item_id.starts_with("shanhai_")
                    || it.item_id.starts_with("mut_")
                    || crate::game::object_ecology::lookup_ecology_item_def(&it.item_id).is_some()
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
                .or_else(|| {
                    crate::game::object_ecology::lookup_ecology_item_def(&item.item_id)
                        .map(|d| d.name)
                })
                .or_else(|| crate::db::fallback_item_name(&item.item_id).map(|s| s.to_string()))
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
                let npc_paid = target.magnesium.max(0).min(unit_price);
                if npc_paid > 0 {
                    let _ = db::add_magnesium(target_id, -npc_paid);
                }
                let _after_quote = crate::game::object_ecology::record_settlement_market_trade(
                    &player_room,
                    &item.item_id,
                    base_price,
                    1,
                    "玩家售出",
                );
                push_refresh(conn, pid);
                let _ = db::adjust_favorability(target_id, pid, 1);
                let _ = event::append(now, pid, "trade", target_id);
                let ep_note = advance_room_event_pack_on_trade(&player_room, &item.item_id, &item_name);
                let ep_suffix = if let Some(ep_line) = ep_note {
                    format!("\n{ep_line}")
                } else {
                    String::new()
                };
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
                            "【{trade_target_name}】端詳你遞上的【{item_name}】（{}），依當地供需行情以 {} 鎂收下。雙方款貨兩訖，妥善收訖入庫。{debt_suffix}{ep_suffix}",
                            quote.market_narrative,
                            quote.unit_price
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
            let personal_item = pick_entity_trade_offer(&target.inventory, &target.equipment_slots);
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
                            || it.item_id.starts_with("craft_")
                            || it.item_id.starts_with("wt_cat_")
                            || it.item_id.starts_with("wt-good-")
                            || it.item_id.starts_with("shanhai_")
                            || it.item_id.starts_with("mut_")
                            || crate::game::object_ecology::lookup_ecology_item_def(&it.item_id).is_some()
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
                            .or_else(|| {
                                crate::game::object_ecology::lookup_ecology_item_def(item_id)
                                    .map(|d| d.name)
                            })
                            .or_else(|| crate::db::fallback_item_name(item_id).map(|s| s.to_string()))
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
                        .or_else(|| {
                            for fg in &crate::game::object_ecology::STANDARD_FINISHED_GOODS {
                                if fg.id == mat_item.item_id {
                                    return Some(fg.base_price);
                                }
                            }
                            None
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
                    let _unit_price = quote.unit_price;

                    conn.send_json(&ActionResultMsg {
                        msg_type: "action_result".into(),
                        action: "Trade".into(),
                        target_id: target.id.clone(),
                        target_name: trade_target_name.clone(),
                        narrative: format!(
                            "【{trade_target_name}】見你行囊中攜有【{item_name}】（{}），表示願按當下行情以 {} 鎂收購。若欲售出，請輸入「賣 {item_name}」，或輸入「拒絕」取消。",
                            quote.market_narrative,
                            quote.unit_price
                        ),
                        success: true,
                        actions: vec![],
                        move_target_id: String::new(),
                        open_trade: true,
                        suggested_action: format!("賣 {item_name}"),
                        tactical_choices: vec![],
                    });
                    return false;
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
                            narr.push_str(&format!(
                                "對方順口指點道：「我手頭現下並無餘貨可讓渡；客友若想採買或出手貨物，{}」",
                                report
                            ));
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
                } else if target.id.starts_with("wt_vendor_") {
                    "（攤上尚有現貨）".to_string()
                } else {
                    "（店內尚有存貨）".to_string()
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
                    "【{trade_target_name}】查過此地市場現有的貨，願意交付「{item_name}」一份，開價 {ask} 鎂{rel_note}，若有誠意亦可略作講價。請輸入你願付的鎂數，或輸入「拒絕」取消。"
                )
            } else if is_canonical_offer {
                let stock_place = if target.id.starts_with("wt_vendor_") {
                    "攤車上的現貨"
                } else {
                    "店內架上的存貨"
                };
                format!(
                    "【{trade_target_name}】看了一下{stock_place}，願意賣出「{item_name}」一份，開價 {ask} 鎂{stock_note}{rel_note}，若有誠意亦可略作講價。請輸入你願付的鎂數，或輸入「拒絕」取消。"
                )
            } else {
                format!(
                    "【{trade_target_name}】願意從隨身備品中勻出「{item_name}」一份，開價 {ask} 鎂{rel_note}，若有誠意亦可略作講價。請輸入你願付的鎂數，或輸入「拒絕」取消。"
                )
            };
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Trade".into(),
                target_id: target.id.clone(),
                target_name: trade_target_name,
                narrative: {
                    if market_id.is_some() {
                        let report = crate::game::object_ecology::format_settlement_supply_demand_report(&room_name);
                        if report.is_empty() {
                            narrative
                        } else {
                            format!("{narrative}\n{report}")
                        }
                    } else {
                        narrative
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
    let has_lineage_or_trust = !crate::pg::living_world::get_entity_lineage_imprints(pid).is_empty()
        || crate::pg::pool()
            .and_then(|p| crate::pg::living_world::get_relationship_summary(&p, target_id, pid).ok())
            .map(|r| r.balance_score >= 2 || r.familiarity_score >= 3)
            .unwrap_or(false);
    let mut paid = paid;
    let fellowship_note = if has_lineage_or_trust && paid > 3 {
        paid = (paid - (paid / 10).max(1)).max(2);
        "（對方見你身具行家師承且素有交情，特意算了熟客同道價）"
    } else {
        ""
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
                    .or_else(|| crate::db::fallback_item_name(&pending.item_id).map(|s| s.to_string()))
                    .unwrap_or_else(|| pending.item_id.clone());
                let ep_note = advance_room_event_pack_on_trade(&player_room, &pending.item_id, &item_name);
                let ep_suffix = if let Some(ep_line) = ep_note {
                    format!("\n{ep_line}")
                } else {
                    String::new()
                };
                trade_offer_clear(pid, target_id);
                push_refresh(conn, pid);
                let _ = event::append(now, pid, types::TRADE, target_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!(
                        "你手頭現鎂僅有 {cash_paid} 鎂，【{trade_target_name}】念在彼此交情，允你先付現鎂、將差額 {debt_created} 鎂立契記帳賒欠，把【{item_name}】交予你手中。{ep_suffix}"
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
                            narrative: format!("【{trade_target_name}】抱歉地表示，該貨物眼下已售罄無存，這筆交易作廢。"),
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
    let player_name = db::get_entity(pid)
        .ok()
        .flatten()
        .map(|e| e.display_title)
        .unwrap_or_else(|| "遊歷客友".to_string());
    crate::npc::causal_core::mutate_npc_from_player_action(
        target_id,
        pid,
        &player_name,
        &player_room,
        "trade",
        &item_name,
        paid,
        now,
    );
    let delivered_cargo = crate::npc::causal_core::deliver_stranded_cargo_for_player(
        pid,
        &player_name,
        target_id,
        &player_room,
        now,
    );
    let _ = &delivered_cargo;
    let stock_suffix = if pending.item_id.starts_with("wt_cat_") {
        let rem = get_shop_item_stock(target_id, &pending.item_id);
        if rem <= 0 {
            "（這是店裡最後一件現貨，店家收拾妥當準備動身去補貨）".to_string()
        } else {
            if target_id.starts_with("wt_vendor_") {
                "（攤主俐落地將現貨包妥遞交予你）".to_string()
            } else {
                "（店家點清銀貨，將物件妥善交付予你）".to_string()
            }
        }
    } else {
        String::new()
    };
    let mut narrative = if offer >= pending.ask_mg {
        format!("你以開價 {paid} 鎂買下「{item_name}」。{stock_suffix}{fellowship_note}")
    } else {
        format!("一番議價後，你以 {paid} 鎂買下「{item_name}」。{stock_suffix}{fellowship_note}")
    };
    if let Some((order, bounty)) = delivered_cargo {
        narrative.push_str(&format!("（同行滯留貨單「{}」順利解送交付，公庫撥付賞格 {} 鎂）", order.item_name, bounty));
    }
    let ep_note = advance_room_event_pack_on_trade(&player_room, &pending.item_id, &item_name);
    if let Some(ep_line) = ep_note {
        narrative.push_str(&format!("\n{ep_line}"));
    }
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
        "編輯", "校勘", "校書生", "記錄員", "抄寫員", "抄錄書生", "書僮", "書童",
        "訪客", "遊客", "香客", "旅人", "食客", "茶客", "讀者", "信眾", "遊俠", "客商", "外來者",
    ];
    if CONFLICTING_KEYWORDS.iter().any(|k| title.contains(k) || t_id.contains(k) || target.current_activity.contains(k)) {
        return false;
    }

    // 檢查若有指定外單位機構前綴且與當前場所衝突
    const ORG_PREFIXES: &[&str] = &[
        "藏書閣", "文曜學院", "公議堂", "東橋旅社", "三水大食堂",
        "三水總商行", "泉香溫水浴場", "溫氏診所", "岑氏鍛造", "鍛造工坊", "鍛造", "木作工坊", "木作", "大圳織造", "織造",
        "風舶聯運", "水尾寮早市", "算學舍", "鐘樓", "蒙學堂", "觀星台", "習武堂", "剪剃鋪", "柑仔店", "百工商算學舍", "文曜蒙學堂",
    ];
    for org in ORG_PREFIXES {
        if (title.contains(org) || t_id.contains(org)) && !room_name.is_empty() && !room_name.contains(org) {
            return false;
        }
    }

    // 檢查居民 household 排班之工作場所是否為衝突外單位
    let slot = crate::store::wenyao_household_slot_for(&target.id);
    if !slot.work_name.is_empty() {
        let work = slot.work_name;
        let is_same_venue = room_name.is_empty()
            || room_name.contains(work) || work.contains(room_name)
            || (room_name.contains("商行") && work.contains("商行"))
            || (room_name.contains("旅社") && (work.contains("旅社") || work.contains("茶屋") || work.contains("歇舍") || work.contains("臨水")))
            || (room_name.contains("食堂") && work.contains("食堂"))
            || (room_name.contains("工坊") && work.contains("工坊"))
            || (room_name.contains("診所") && work.contains("診所"))
            || (room_name.contains("浴場") && work.contains("浴場"));
        if !is_same_venue && (work.contains("學院") || work.contains("藏書閣") || work.contains("公議堂")
            || work.contains("學堂") || work.contains("診所") || work.contains("官署") || work.contains("巡防")
            || work.contains("鍛造") || work.contains("工坊") || work.contains("商行") || work.contains("旅社")
            || work.contains("食堂") || work.contains("浴場") || work.contains("織造") || work.contains("鐘樓")
            || work.contains("車隊") || work.contains("貨運") || work.contains("鋪") || work.contains("店")) {
            return false;
        }
    }

    // 整合現場人員辨識：制服／圍裙／工務裝束或站在管制櫃檯區
    let mut probe = crate::store::Entity::new_npc(&target.id, &target.display_char);
    probe.display_title = if target.current_activity.is_empty() {
        target.display_title.clone()
    } else {
        format!("{}|{}", target.display_title, target.current_activity)
    };
    probe.current_activity = target.current_activity.clone();
    probe.equipment_slots = target.equipment_slots.clone();
    probe.inventory = target.inventory.clone();
    probe.grid_x = target.grid_x;
    probe.grid_y = target.grid_y;
    probe.magnesium = target.magnesium;

    let (has_uniform, in_counter_zone) =
        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder_entity(
            &probe,
            (target.grid_x.unwrap_or(0), target.grid_y.unwrap_or(0)),
            "",
        );
    if has_uniform || in_counter_zone {
        return true;
    }

    // 正向判定：具備店東、店員、幫工、大廚等商鋪職銜，或工作場所相符
    const STAFF_KEYWORDS: &[&str] = &[
        "掌事", "店主", concat!("店", "長"), "店員", "店助", concat!("助", "理"), "幫工", "學徒", "師傅",
        "匠", "主事", "管事", "大廚", "廚工", "跑堂", "侍者", "侍應", "服務生", "販", "商",
        "茶師", "藥師", "工", "役", "東家", concat!("掌", "櫃"), concat!("老", "闆"), "掌勺",
    ];
    if STAFF_KEYWORDS.iter().any(|k| title.contains(k) || t_id.contains(k)) {
        return true;
    }
    if !slot.work_name.is_empty() && !room_name.is_empty() && (room_name.contains(slot.work_name) || slot.work_name.contains(room_name)) {
        return true;
    }
    let pers_role = crate::store::npc_persistent_role_label(&target.id, &target.display_title);
    if STAFF_KEYWORDS.iter().any(|k| pers_role.contains(k)) {
        return true;
    }

    // 若未被判定為衝突職稱，且無其他工作衝突
    if slot.work_name.is_empty() {
        let is_commercial_venue = room_name.is_empty()
            || [
                "店", "鋪", "攤", "坊", "行", "市", "商", "堂", "閣", "館", "舍", "樓",
                "局", "所", "屋", "社", "場", "食堂", "旅社", "診所", "浴場", "工坊",
                "書局", "早市", "集市", "拍賣",
            ]
            .iter()
            .any(|k| room_name.contains(k));

        let is_passerby = target.current_activity.contains("路過")
            || target.current_activity.contains("閒逛")
            || target.current_activity.contains("茶客")
            || target.current_activity.contains("食客");

        let is_on_duty_or_active = has_uniform
            || in_counter_zone
            || target.current_activity.is_empty()
            || target.current_activity.contains("當值")
            || target.current_activity.contains("櫃檯")
            || target.current_activity.contains("看顧")
            || target.current_activity.contains("掌");

        if is_commercial_venue && !is_passerby && is_on_duty_or_active {
            return true;
        }
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

    let hour = crate::store::current_game_hour();
    let is_always_open = title.contains("旅社")
        || title.contains("客舍")
        || title.contains("夜班")
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

    if let Some((item_id, item_name, price_mg, _)) = vendor_stall_offer_for_npc(t_id) {
        if t_id.starts_with("wt_stall_") || t_id.starts_with("wt_vendor_") {
            return Some((item_id.to_string(), item_name.to_string(), price_mg));
        }
    }

    if is_eligible_shop_staff(target, room_name) {
        if let Some(venue) = crate::grid::resolve_room_venue_object(room_name, title) {
            if !venue.counter_trade_offers.is_empty() {
                let idx = ((now.unsigned_abs() as usize) / 60) % venue.counter_trade_offers.len();
                let (id, name, price) = venue.counter_trade_offers[idx];
                return Some((id.to_string(), name.to_string(), price));
            }
        }
    }
    if !is_shop_person(target, room_name) {
        return None;
    }

    if let Some(venue) = crate::grid::resolve_market_or_specialty_room_object(room_name, title) {
        if !venue.counter_trade_offers.is_empty() {
            let idx = ((now.unsigned_abs() as usize) / 60) % venue.counter_trade_offers.len();
            let (id, name, price) = venue.counter_trade_offers[idx];
            return Some((id.to_string(), name.to_string(), price));
        }
    }

    None
}

pub(super) fn is_shop_person(target: &Character, room_name: &str) -> bool {
    is_eligible_shop_staff(target, room_name)
}

pub(super) fn target_has_shop_offer(target: &Character, room_name: &str) -> bool {
    let hour = crate::store::current_game_hour();
    let is_always_open = target.display_title.contains("旅社")
        || target.display_title.contains("客舍")
        || target.display_title.contains("夜班")
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

    #[test]
    fn test_canonical_merchant_stock_matches_catalog() {
        let canonical_npcs = [
            "陸承禮", "陸長貴", "陸秉衡", "陳萬福", "沈清河", "白泉生", "溫晴", "溫懷瑾",
            "岑守岳", "岑拓", "柯成器", "蘇錦心", "秦明誠", "林時中", "沈硯舟", "陸知微",
            "陸敬軒", "趙觀星", "岑雲野", "周一刀", "許阿甘", "馬補靴", "韓鐵山",
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

        for &npc in &canonical_npcs {
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

        // 3. A canonical artisan (e.g. a blacksmith from 鍛造工坊) visiting "文曜・三水大食堂"
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
            line.contains("劇本包推進"),
            "advancement note must contain '劇本包推進', got: {line}"
        );
    }
}
