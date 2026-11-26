//! 實體店鋪主權與經營權頂讓、封爐出走機制與去職業鎖之物理工具能力判定。

use serde::{Deserialize, Serialize};

use super::clock_and_lang::SolarClockSnapshot;
use super::household_economy::{count_box_home_rations_only, is_craft_or_shop_master_role};
use super::live_commodities::{count_apothecary_herbs, count_smithy_fuel};
use super::movement::push_breath;
use super::persona::{ValueAxis, derive_innate_persona};
use super::types::{CarryItem, IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShopLeaseState {
    #[default]
    Active,
    SealedForLease,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShopLeaseRecord {
    pub shop_node_id: String,
    pub shop_kind: String,
    pub owner_actor_id: String,
    #[serde(default)]
    pub operator_actor_id: Option<String>,
    #[serde(default)]
    pub state: ShopLeaseState,
    #[serde(default)]
    pub transfer_fee_copper: i64,
    #[serde(default)]
    pub rent_per_day: i64,
    #[serde(default)]
    pub seal_reason_zh: String,
    #[serde(default)]
    pub sealed_day: i64,
}

/// 推導店鋪實體類型（僅依節點物理空間與招牌，不寫死特定人物姓名）。
pub fn infer_shop_kind(node: &SpatialNodeDef) -> &'static str {
    if node.name.contains("鍛鐵") || node.name.contains("鐵匠") {
        "smithy"
    } else if node.name.contains("酒館") || node.name.contains("客棧") {
        "tavern"
    } else if node.name.contains("草藥") {
        "apothecary"
    } else if node.name.contains("木匠") {
        "carpentry"
    } else if node.name.contains("車行") {
        "carriage"
    } else {
        "market"
    }
}

/// 判斷店鋪是否處於「封爐／掛牌待頂讓」狀態（此時無人承接櫃檯）。
pub fn is_shop_sealed(delta: &IslandWorldDelta, shop_node_id: &str) -> bool {
    delta
        .shop_leases
        .get(shop_node_id)
        .is_some_and(|l| l.state == ShopLeaseState::SealedForLease && l.operator_actor_id.is_none())
}

/// 查詢指定實體店鋪目前具名主事經營者；若已封爐待租則嚴格回傳 `None`。
pub fn shop_active_operator_id(
    delta: &IslandWorldDelta,
    shop_node_id: &str,
    preferred_role_hints: &[&str],
) -> Option<String> {
    if let Some(lease) = delta.shop_leases.get(shop_node_id) {
        if lease.state == ShopLeaseState::SealedForLease {
            return None;
        }
        return lease.operator_actor_id.as_ref().and_then(|op_id| {
            delta
                .actors
                .get(op_id)
                .is_some_and(|a| a.alive && !op_id.starts_with("pl:"))
                .then(|| op_id.clone())
        });
    }
    let mut candidates: Vec<&UnifiedMindBody> = delta
        .actors
        .values()
        .filter(|a| {
            a.alive
                && a.controller_id.is_none()
                && !a.actor_id.starts_with("pl:")
                && a.work_node == shop_node_id
        })
        .collect();
    candidates.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
    if !preferred_role_hints.is_empty()
        && let Some(hit) = candidates
            .iter()
            .find(|a| preferred_role_hints.iter().any(|kw| a.role.contains(kw)))
    {
        return Some(hit.actor_id.clone());
    }
    candidates.first().map(|a| a.actor_id.clone())
}

/// 查詢某個角色（含人類玩家）當下是否為指定店鋪之在位主事經營者。
pub fn actor_operates_shop(delta: &IslandWorldDelta, aid: &str, shop_node_id: &str) -> bool {
    if let Some(lease) = delta.shop_leases.get(shop_node_id) {
        return lease.state == ShopLeaseState::Active
            && lease.operator_actor_id.as_deref() == Some(aid);
    }
    delta.actors.get(aid).is_some_and(|a| {
        a.alive
            && a.work_node == shop_node_id
            && (is_craft_or_shop_master_role(&a.role) || a.home_node == shop_node_id)
    })
}

/// 檢查角色是否目前正實際背負某間仍在營業的店鋪經營責任（封爐離店後即解鎖）。
pub fn actor_currently_operates_any_shop(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
) -> Option<String> {
    if let Some((sid, _)) = delta.shop_leases.iter().find(|(_, l)| {
        l.state == ShopLeaseState::Active && l.operator_actor_id.as_deref() == Some(&a.actor_id)
    }) {
        return Some(sid.clone());
    }
    if doc
        .nodes
        .get(&a.work_node)
        .is_some_and(|n| n.kind == "shop")
        && !delta.shop_leases.contains_key(&a.work_node)
        && is_craft_or_shop_master_role(&a.role)
    {
        return Some(a.work_node.clone());
    }
    None
}

fn str_matches_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|kw| haystack.contains(kw))
}

fn actor_has_tool_with(a: &UnifiedMindBody, slug_kws: &[&str], name_kws: &[&str]) -> bool {
    if str_matches_any(&a.bound_weapon.slug, slug_kws)
        || str_matches_any(&a.bound_weapon.name, name_kws)
    {
        return true;
    }
    a.backpack.iter().any(|i| {
        i.count > 0
            && (str_matches_any(&i.slug, slug_kws)
                || str_matches_any(&i.name, name_kws)
                || i.vessel_meta.as_ref().is_some_and(|vm| {
                    str_matches_any(&vm.slug, slug_kws) || str_matches_any(&vm.name, name_kws)
                }))
    })
}

/// 物理器具檢定：具備開採／錘鍛器具（不靠角色職稱字串）。
pub fn has_mining_or_forging_tool(a: &UnifiedMindBody) -> bool {
    actor_has_tool_with(
        a,
        &["hammer", "pick", "chisel", "axe", "smith"],
        &[
            "鐵鎚", "鍛錘", "礦鎬", "鐵鎬", "石錘", "鑿", "鎚", "錘", "斧",
        ],
    )
}

/// 物理器具檢定：具備伐木與修橋木石器具（僅涵蓋斧、鋸、鑿、鉋、石錘，不誤納鐵匠鍛錘與礦鎬）。
pub fn has_forestry_or_bridge_tool(a: &UnifiedMindBody) -> bool {
    actor_has_tool_with(
        a,
        &["axe", "saw", "chisel", "carpenter", "mason"],
        &["斧", "鋸", "鑿", "鉋", "石錘", "柴刀"],
    ) || a.role.contains("木")
        || a.role.contains("石")
}

/// 物理武備檢定：持有可用於巡防、清剿或探坑防身之戰具與重錘（不含純農漁鋤鐮與日常木斧，避免把平穩農匠拉去打營寨）。
pub fn has_combat_or_guard_arm(a: &UnifiedMindBody) -> bool {
    actor_has_tool_with(
        a,
        &["sword", "blade", "spear", "bow", "hammer", "club"],
        &["刀", "劍", "矛", "槍", "弓", "弩", "戰斧", "錘", "鎚", "棍"],
    )
}

/// 物理器具檢定：具備野外採集／獵捕／農作能力（任何具趁手工具或健康體魄之住民皆可）。
pub fn has_wild_forage_capability(a: &UnifiedMindBody) -> bool {
    a.wound_part.is_none()
        && (actor_has_tool_with(
            a,
            &["sickle", "hoe", "net", "bow", "spear", "knife", "axe"],
            &["鐮", "鋤", "網", "叉", "刀", "斧", "弓", "矛", "籃"],
        ) || a.fatigue < 65)
}

fn find_outdoor_venture_post(doc: &WorldStaticDoc, cell_id: u32) -> String {
    doc.nodes
        .values()
        .filter(|n| {
            n.cell_id == cell_id
                && !n.indoor
                && !n.dungeon
                && matches!(
                    n.kind.as_str(),
                    "plaza" | "gate" | "village_gate" | "wild" | "dock"
                )
        })
        .min_by_key(|n| (n.kind != "plaza", n.id.clone()))
        .map_or_else(
            || "oi:white_gull_harbor:plaza".to_string(),
            |n| n.id.clone(),
        )
}

/// 原店主／主事者主動「封爐／掛牌頂讓」：保留所有權（`owner_actor_id`），卸下櫃檯值守改赴野外尋礦探險或另謀生計，且絕不更動原兵器。
pub fn seal_shop_for_venture(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    shop_node_id: &str,
    operator_id: &str,
    reason_zh: &str,
    transfer_fee_copper: i64,
) -> Result<String, String> {
    let shop_node = doc.nodes.get(shop_node_id).ok_or("找不到店鋪位置")?;
    if shop_node.kind != "shop" {
        return Err("唯有實體店鋪方能封爐掛牌頂讓。".to_string());
    }
    if !actor_operates_shop(delta, operator_id, shop_node_id) {
        return Err("只有現任當家主事者才能決定封爐或頂讓店鋪。".to_string());
    }

    let owner_id = delta
        .shop_leases
        .get(shop_node_id)
        .map(|l| l.owner_actor_id.clone())
        .unwrap_or_else(|| operator_id.to_string());
    let fee = transfer_fee_copper.max(4);
    let shop_kind = infer_shop_kind(shop_node).to_string();
    delta.shop_leases.insert(
        shop_node_id.to_string(),
        ShopLeaseRecord {
            shop_node_id: shop_node_id.to_string(),
            shop_kind,
            owner_actor_id: owner_id,
            operator_actor_id: None,
            state: ShopLeaseState::SealedForLease,
            transfer_fee_copper: fee,
            rent_per_day: 2,
            seal_reason_zh: reason_zh.to_string(),
            sealed_day: clock.game_days,
        },
    );
    delta.extras_dirty = true;

    let outdoor_post = find_outdoor_venture_post(doc, shop_node.cell_id);
    let shop_name = shop_node.name.clone();
    if let Some(a) = delta.actors.get_mut(operator_id) {
        // 絕不呼叫 default_tool_for_role：原本的鐵匠鍛錘／趁手器具原封不動留在手上！
        if a.work_node == shop_node_id {
            a.work_node = outdoor_post.clone();
        }
        if a.home_node == shop_node_id {
            a.home_node = outdoor_post;
        }
        a.mind.stm.active_goal_tag = format!("封鋪出走：{reason_zh}");
        a.mind.stm.today_goal_progressed = true;
        a.mind.push_turning_point(
            &format!("shop_seal:{}:{}", shop_node_id, clock.game_days),
            clock.game_days,
            &format!(
                "因「{reason_zh}」暫封【{shop_name}】掛牌頂讓，拎著【{}】出門闖路",
                a.bound_weapon.name
            ),
            "forage_supplement",
        );
        push_breath(
            a,
            format!(
                "在【{shop_name}】門前掛上「暫封尋礦・待價頂讓（頂讓金 {fee} 銅）」木牌，拎著手上的【{}】出門闖路。",
                a.bound_weapon.name
            ),
        );
        a.mark_mutated();
    }
    Ok(format!(
        "已在【{shop_name}】門口掛出待頂讓木牌（頂讓費 {fee} 枚銅幣），現任主事卸下櫃檯離鋪。"
    ))
}

/// 玩家或有餘裕之居民出資承租已掛牌頂讓的店鋪，成為新任經營者；頂讓金 100% 轉交原東家（絕不發新工具）。
pub fn lease_vacant_shop(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    lessee_id: &str,
    shop_node_id: &str,
) -> Result<String, String> {
    let shop_node = doc.nodes.get(shop_node_id).ok_or("找不到欲承租之店鋪")?;
    if shop_node.kind != "shop" {
        return Err("此處並非可頂讓營業之店鋪。".to_string());
    }
    let lease_snapshot = delta
        .shop_leases
        .get(shop_node_id)
        .cloned()
        .ok_or_else(|| "這家鋪子並未掛出頂讓木牌。".to_string())?;
    if lease_snapshot.state != ShopLeaseState::SealedForLease
        || lease_snapshot.operator_actor_id.is_some()
    {
        return Err("這家鋪子此刻已有主事者當值營業。".to_string());
    }
    if let Some(lessee) = delta.actors.get(lessee_id)
        && actor_currently_operates_any_shop(doc, delta, lessee)
            .is_some_and(|cur_sid| cur_sid != shop_node_id)
    {
        return Err("你名下尚在經營另一間鋪子，先封爐交代再來頂下這間吧。".to_string());
    }
    let fee = lease_snapshot.transfer_fee_copper.max(0);
    if fee > 0 {
        let owner_alive = delta
            .actors
            .get(&lease_snapshot.owner_actor_id)
            .is_some_and(|o| o.alive);
        if !owner_alive {
            return Err("原東家已不在島上，無人可過戶鋪面契據。".to_string());
        }
        if !delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "shop_lease_transfer",
            lessee_id,
            &lease_snapshot.owner_actor_id,
            fee,
            format!("頂讓承租【{}】契據費", shop_node.name),
        ) {
            return Err(format!(
                "承租【{}】需先付清 {} 枚銅幣頂讓金。",
                shop_node.name, fee
            ));
        }
    }

    if let Some(lease) = delta.shop_leases.get_mut(shop_node_id) {
        lease.state = ShopLeaseState::Active;
        lease.operator_actor_id = Some(lessee_id.to_string());
        lease.seal_reason_zh.clear();
    }
    delta.extras_dirty = true;

    let shop_name = shop_node.name.clone();
    if let Some(lessee) = delta.actors.get_mut(lessee_id) {
        if !lessee_id.starts_with("pl:") {
            lessee.work_node = shop_node_id.to_string();
        } else {
            lessee.owned_house_node = Some(shop_node_id.to_string());
        }
        lessee.mind.ltm.known_nodes.insert(shop_node_id.to_string());
        lessee.mind.push_turning_point(
            &format!("shop_lease:{}:{}", shop_node_id, clock.game_days),
            clock.game_days,
            &format!("出資 {fee} 枚銅幣頂下【{shop_name}】經營權重開門面"),
            "work_steady",
        );
        push_breath(
            lessee,
            format!("付清 {fee} 枚銅幣頂下【{shop_name}】鑰匙與牌照，開門生火待客。"),
        );
        lessee.mark_mutated();
    }
    Ok(format!(
        "已付清 {fee} 枚銅幣頂讓金，正式接手【{shop_name}】經營權！"
    ))
}

/// 評估現任店主是否因缺煤虧空或冒險欲而「封爐／頂讓出走」，或自由居民「付費頂下空鋪」；回傳欲執行之 `shop_node_id`。
pub fn evaluate_autonomous_shop_venture_or_lease(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
) -> Option<(&'static str, String)> {
    let a = delta.actors.get(aid)?;
    if !a.alive || a.controller_id.is_some() || aid.starts_with("pl:") || a.wound_part.is_some() {
        return None;
    }
    let innate = derive_innate_persona(a.soul_seed.max(1));

    // 1. 現任店長檢視自家爐火／庫存／帳面：煤盡爐冷或虧空或厭倦守鋪 -> 主動封爐尋活路
    if let Some(shop_nid) = actor_currently_operates_any_shop(doc, delta, a)
        && let Some(snode) = doc.nodes.get(&shop_nid)
    {
        let skind = infer_shop_kind(snode);
        let stock_empty = match skind {
            "smithy" => count_smithy_fuel(delta, &shop_nid) == 0,
            "apothecary" => count_apothecary_herbs(delta, &shop_nid) == 0,
            "tavern" => count_box_home_rations_only(a, &shop_nid) == 0,
            _ => false,
        };
        let venture_urge =
            (stock_empty && a.copper <= 18 && (skind != "tavern" || clock.game_days >= 10))
                || (innate.temperament.approach_avoid >= 28
                    && a.mind.stm.affect.hope <= 20
                    && a.copper <= 45
                    && clock.game_days >= 18);
        if venture_urge && !is_shop_sealed(delta, &shop_nid) {
            return Some(("seal_shop_venture", shop_nid));
        }
    }

    // 2. 尚未經營鋪面、手頭寬裕且有對應手藝／經商傾向的居民 -> 主動承租同鎮（或已知）掛牌待讓店鋪
    if actor_currently_operates_any_shop(doc, delta, a).is_none() && a.fatigue <= 55 {
        let top_val = innate.top_value_axis(&a.drift);
        let cur_cid = doc.nodes.get(&a.node_id)?.cell_id;
        for (sid, lease) in &delta.shop_leases {
            if lease.state != ShopLeaseState::SealedForLease || lease.operator_actor_id.is_some() {
                continue;
            }
            if lease.owner_actor_id == aid || a.copper < lease.transfer_fee_copper + 8 {
                continue;
            }
            let Some(snode) = doc.nodes.get(sid) else {
                continue;
            };
            let knows_or_same_town =
                snode.cell_id == cur_cid || a.mind.ltm.known_nodes.contains(sid);
            if !knows_or_same_town {
                continue;
            }
            let tool_fit = match lease.shop_kind.as_str() {
                "smithy" | "carpentry" => {
                    has_mining_or_forging_tool(a) || has_forestry_or_bridge_tool(a)
                }
                _ => matches!(
                    top_val,
                    ValueAxis::TangibleGain | ValueAxis::OrderHierarchy | ValueAxis::KinshipLoyalty
                ),
            };
            if tool_fit {
                return Some(("lease_vacant_shop", sid.clone()));
            }
        }
    }

    None
}

/// 盤點指定店員之貨架／箱櫃是否含有任何待售商品或原料（用於判斷鋪位狀態）。
pub fn carry_items_match_slugs(items: &[CarryItem], slugs: &[&str]) -> u32 {
    items
        .iter()
        .filter(|i| {
            slugs.contains(&i.slug.as_str())
                && i.bound_owner.is_none()
                && i.stolen_from.is_none()
                && i.count > 0
        })
        .map(|i| i.count)
        .sum()
}
