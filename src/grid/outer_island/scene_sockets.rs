//! 玩家主畫面「在場對口人物營生／買賣籤（Person Socket）」與同節點原子驗真輔助。
//!
//! 審辯室 Room `cf1bfab9-16d7-4de8-b742-8076dd910468` 定案契約：
//! - 營生買賣收歸在場對口人物抽屜頂部（0ms 直出、附報價憑證、選物出售防盲賣）。
//! - 伺服器端嚴格驗證同一空間節點 `node_occupants[node_id]` 在場事實，零全城隔空交易。

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::cell_resources::{
    is_specialty_buyer_role, lookup_resource_spec, specialty_sell_unit_price,
};
use super::clock_and_lang::SolarClockSnapshot;
use super::types::{IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};

pub use super::scene_objects::build_scene_presence_sockets;

/// 嚴格於「當前同一空間節點 `node_occupants[node_id]`」原子查找在場對口人物（零全城 `cell_id` 隔空掃描）
pub fn find_same_node_counterparty(
    delta: &IslandWorldDelta,
    self_aid: &str,
    node_id: &str,
    real_unix: i64,
    accepted_roles: &[&str],
    explicit_tok: Option<&str>,
) -> Option<String> {
    let occ = delta.node_occupants.get(node_id)?;
    let can_transact = |a: &UnifiedMindBody| -> bool {
        a.alive
            && a.setup_done
            && a.node_id == node_id
            && accepted_roles.contains(&a.role.as_str())
            && !super::live_tide::is_actor_online_protected(a, real_unix)
    };
    if let Some(tok) =
        explicit_tok.filter(|s| !s.is_empty() && *s != "self_basket" && *s != "public_board")
    {
        return occ
            .iter()
            .find(|&oid| {
                oid != self_aid
                    && delta.actors.get(oid).is_some_and(|a| {
                        can_transact(a) && (a.public_actor_token() == tok || a.actor_id == tok)
                    })
            })
            .cloned();
    }
    occ.iter()
        .find(|&oid| oid != self_aid && delta.actors.get(oid).is_some_and(can_transact))
        .cloned()
}

pub fn parse_counterparty_and_quote(param: &str) -> (Option<&str>, Option<i64>) {
    let trimmed = param.trim();
    if trimmed.is_empty() {
        return (None, None);
    }
    let mut parts = trimmed.split('|');
    let first = parts.next().filter(|s| !s.is_empty());
    let quote = parts.next().and_then(|q| q.parse::<i64>().ok());
    (first, quote)
}

pub(super) fn has_role_at_node(
    delta: &IslandWorldDelta,
    viewer_id: &str,
    node_id: &str,
    role: &str,
) -> bool {
    delta.node_occupants.get(node_id).is_some_and(|occ| {
        occ.iter().any(|oid| {
            oid != viewer_id
                && delta.actors.get(oid).is_some_and(|o| {
                    o.alive
                        && o.setup_done
                        && o.controller_id.is_none()
                        && o.node_id == node_id
                        && o.role == role
                })
        })
    })
}

fn is_first_of_roles_at_node(
    delta: &IslandWorldDelta,
    viewer_id: &str,
    node_id: &str,
    target_id: &str,
    roles: &[&str],
) -> bool {
    for role in roles {
        if let Some(occ) = delta.node_occupants.get(node_id) {
            let mut matching: Vec<&str> = occ
                .iter()
                .filter_map(|oid| {
                    (oid != viewer_id
                        && delta.actors.get(oid).is_some_and(|o| {
                            o.alive
                                && o.setup_done
                                && o.controller_id.is_none()
                                && o.node_id == node_id
                                && o.role == *role
                        }))
                    .then_some(oid.as_str())
                })
                .collect();
            matching.sort_unstable();
            if let Some(&first_id) = matching.first() {
                return first_id == target_id;
            }
        }
    }
    false
}

/// 組裝單一在場 NPC 對當前觀察者可直接當面洽辦之「明碼標價營生／買賣籤」（抽屜頂部 0ms 直出，免等 LLM）
pub fn build_actor_deeds_for_viewer(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    other: &UnifiedMindBody,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
) -> Vec<Value> {
    let mut deeds = Vec::new();
    if other.controller_id.is_some()
        || !other.alive
        || other.node_id != node.id
        || viewer.active_skirmish.is_some()
        || viewer.active_transit.is_some()
    {
        return deeds;
    }

    let tok = other.public_actor_token();

    // 1. 港務碼頭：記籌手領箱扛送 ＆ 潮汐靠岸卸貨
    if node.kind == "dock" && !clock.is_night {
        let is_haul_host = is_first_of_roles_at_node(
            delta,
            &viewer.actor_id,
            &node.id,
            &other.actor_id,
            &["港務記籌手", "碼頭苦力"],
        );
        if is_haul_host && viewer.ongoing_activity.is_none() {
            let order_preview = match delta.dock_cargo_piles.get(&node.id) {
                Some(list) => list.first().map(|ord| {
                    let short_shop = ord
                        .shop_name_zh
                        .split('・')
                        .next_back()
                        .unwrap_or(&ord.shop_name_zh);
                    format!("送往{} · 腳錢 {} 枚銅幣", short_shop, ord.wage_copper)
                }),
                None => matches!(
                    node.id.as_str(),
                    "oi:white_gull_harbor:dock"
                        | "oi:west_strait_ferry:dock"
                        | "oi:gull_tail_port:dock"
                        | "oi:north_cape_beacon:dock"
                )
                .then(|| "送往城內鋪號 · 腳錢 5 枚銅幣".to_string()),
            };
            if let Some(sub_zh) = order_preview {
                deeds.push(json!({
                    "id": "claim_dock_haul",
                    "param": tok.clone(),
                    "label": "領籌扛送鋪貨木箱",
                    "sub": sub_zh,
                }));
            }
        }
    }

    // 2. 鍛鐵鋪／百工集坊：師傅或當值學徒（須在爐邊且日間開爐）
    if !clock.is_night
        && (node.name.contains("鍛鐵") || node.name.contains("鐵匠") || node.name.contains("集坊"))
        && is_first_of_roles_at_node(
            delta,
            &viewer.actor_id,
            &node.id,
            &other.actor_id,
            &["鍛鐵匠", "鍛鐵匠學徒", "木匠"],
        )
    {
        if node.name.contains("鍛鐵") || node.name.contains("鐵匠") {
            if !viewer
                .explored_windows
                .contains(&super::live_tide::forge_shift_window_key(
                    clock.game_days,
                    clock.bell_index,
                ))
            {
                deeds.push(json!({
                    "id": "forge_help",
                    "param": format!("{tok}|5"),
                    "label": "拉風箱協助打鐵 · 5 枚銅幣",
                    "sub": "本爐助鍛工錢",
                }));
            }
            if viewer.bound_weapon.wear_stage > 0 {
                deeds.push(json!({
                    "id": "repair_weapon",
                    "param": format!("{tok}|4"),
                    "label": format!("修補【{}】 · 4 枚銅幣", viewer.bound_weapon.name),
                    "sub": "燒紅校直加固木柄",
                }));
            }
            if viewer.bound_weapon.rune_mark.is_none()
                && viewer
                    .backpack
                    .iter()
                    .any(|i| i.slug == "rune_stone" && i.count > 0 && i.bound_owner.is_none())
            {
                deeds.push(json!({
                    "id": "engrave_rune",
                    "param": format!("{tok}|6"),
                    "label": "將古符文原石鍛入兵刃 · 6 枚銅幣",
                    "sub": "鏨刻沉穩鐵紋",
                }));
            }
        }
        let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
        let has_ready_order = delta.forge_orders.get(&node.id).is_some_and(|list| {
            list.iter()
                .any(|o| o.customer_id == viewer.actor_id && abs_bell >= o.ready_abs_bell)
        });
        if !has_ready_order
            && viewer
                .backpack
                .iter()
                .any(|i| matches!(i.slug.as_str(), "iron_ore" | "quartz_sand") && i.count >= 2)
        {
            deeds.push(json!({
                "id": "commission_custom_vessel",
                "param": format!("{tok}|6"),
                "label": "下訂一鐘精鍛銘器 · 6 枚銅幣",
                "sub": "交礦沙 2 份・一記鐘響後於取件架領器",
            }));
        }
    }

    // 3. 酒館一樓櫃台：掌櫃與值夜學徒錯班辦理投宿、抵宿券與現烘乾糧
    if node.kind == "shop"
        && node.name.contains("酒館")
        && is_first_of_roles_at_node(
            delta,
            &viewer.actor_id,
            &node.id,
            &other.actor_id,
            &["酒館掌櫃", "酒館掌櫃學徒"],
        )
    {
        if viewer
            .backpack
            .iter()
            .any(|i| i.slug == "lodging_voucher" && i.count > 0)
        {
            deeds.push(json!({
                "id": "redeem_voucher",
                "param": tok.clone(),
                "label": "出示船東抵宿券",
                "sub": "換領二樓獨立客房木牌兩宿",
            }));
        }
        deeds.push(json!({
            "id": "rent_inn",
            "param": format!("{tok}|3"),
            "label": "租下二樓客房一宿 · 3 枚銅幣",
            "sub": "領客房木牌與床頭木箱銅鎖",
        }));
        if super::live_tide::tavern_has_ration_stock(doc, delta, node.cell_id, &node.id) {
            deeds.push(json!({
                "id": "buy_ration",
                "param": format!("{tok}|2"),
                "label": "購買黑麥乾糧 · 2 枚銅幣",
                "sub": "現烘紮實遠行口糧",
            }));
        }
    }

    // 4. 苦艾草藥堂：醫者或當值學徒包紮傷勢
    if !clock.is_night
        && node.name.contains("草藥")
        && viewer.wound_part.is_some()
        && is_first_of_roles_at_node(
            delta,
            &viewer.actor_id,
            &node.id,
            &other.actor_id,
            &["草藥師", "草藥師學徒"],
        )
    {
        let wpart = viewer.wound_part.as_deref().unwrap_or("傷處");
        deeds.push(json!({
            "id": "bandage_wound",
            "param": format!("{tok}|3"),
            "label": format!("敷苦艾膏包紮【{}】 · 3 枚銅幣", wpart),
            "sub": "溫膏纏帶消腫",
        }));
    }

    // 5. 城門當值衛兵：主動申報拾遺物
    if node.kind == "gate"
        && viewer.backpack.iter().any(|i| i.stolen_from.is_some())
        && is_first_of_roles_at_node(
            delta,
            &viewer.actor_id,
            &node.id,
            &other.actor_id,
            &["巡城衛兵", "關哨衛兵", "檢疫衛兵"],
        )
    {
        deeds.push(json!({
            "id": "declare_found_bag",
            "param": tok.clone(),
            "label": "主動報備拾遺物件",
            "sub": "核對章記轉為正當留用",
        }));
    }

    // 6. 當面選物報價出售（每個收貨對口皆列出玩家背囊可售品項與單價，根除盲賣第一件 Bug！）
    if matches!(
        node.kind.as_str(),
        "shop" | "dock" | "plaza" | "village_gate"
    ) && is_specialty_buyer_role(&other.role)
    {
        let mut seen_slugs = BTreeSet::new();
        for it in &viewer.backpack {
            if seen_slugs.len() >= 4 {
                break;
            }
            if let Some(unit_price) = specialty_sell_unit_price(it)
                && seen_slugs.insert(it.slug.clone())
            {
                let note = if it.stolen_from.is_some() {
                    "未報備拾遺折半計價"
                } else if lookup_resource_spec(&it.slug).is_some() {
                    "當面過秤點銅"
                } else {
                    "驗看成色收購"
                };
                deeds.push(json!({
                    "id": "sell_specialty",
                    "param": format!("{tok}|{}|{unit_price}", it.slug),
                    "label": format!("出售【{}】 · {} 枚銅幣", it.name, unit_price),
                    "sub": note,
                }));
            }
        }
    }

    deeds
}
