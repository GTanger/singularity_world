//! 玩家主畫面「具名現場實物與車船（Object Socket）× 首行自身體感（Self Socket）」組裝器。
//!
//! - 單一無扣款閱讀／拾取／已付費取件實物：設 `direct_action`（1 指直達，嚴守死罪 B 零「細閱」廢話選單）。
//! - 涉及銅幣收付、時間推進或多單據選擇之實物：設 `options`（展開單層實物卡）。

use serde_json::{Value, json};

use super::clock_and_lang::SolarClockSnapshot;
use super::scene_sockets::has_role_at_node;
use super::types::{IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};

/// 組裝主畫面三大無標題物理插座：`scene.objects`（具名實物與車船）、`player.body_cues`（首行自身體感）、`scene.actions`（全載體對帳稽核表）
pub fn build_scene_presence_sockets(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
    all_npcs_json: &[Value],
) -> (Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut objects: Vec<Value> = Vec::new();
    let mut body_cues: Vec<Value> = Vec::new();
    let mut audit_actions: Vec<Value> = Vec::new();

    let mut push_audit =
        |id: &str, param: &str, label: &str, sub: &str, c_kind: &str, c_id: &str| {
            let dup = audit_actions.iter().any(|x| {
                x["id"].as_str() == Some(id)
                    && x["carrier_kind"].as_str() == Some(c_kind)
                    && x["param"].as_str() == Some(param)
            });
            if !dup {
                audit_actions.push(json!({
                    "id": id,
                    "param": param,
                    "label": label,
                    "sub": sub,
                    "carrier_kind": c_kind,
                    "carrier_id": c_id,
                }));
            }
        };

    // A. 彙整所有在場人物身上掛載之 `deeds` 入對帳表
    for npc_val in all_npcs_json {
        let cid = npc_val["id"].as_str().unwrap_or("");
        if let Some(arr) = npc_val["deeds"].as_array() {
            for d in arr {
                push_audit(
                    d["id"].as_str().unwrap_or(""),
                    d["param"].as_str().unwrap_or(""),
                    d["label"].as_str().unwrap_or(""),
                    d["sub"].as_str().unwrap_or(""),
                    "person",
                    cid,
                );
            }
        }
    }

    // B. 角色自身體感橫條與隨身處置（不入場景地面，不佔頂欄！）
    if let Some(ref wpart) = viewer.wound_part {
        let tail = if viewer.severe_wrist_warning {
            "未敷苦艾膏包紮前，切莫硬撼重甲異獸"
        } else {
            "可尋草藥堂醫者敷苦艾膏消腫"
        };
        body_cues.push(json!({
            "kind": "wound",
            "text": format!("{wpart}隱隱作痛（{tail}）"),
            "action": Value::Null,
        }));
    }
    if let Some(ref st) = viewer.ongoing_activity {
        let act_label = if st.kind == "dock_haul" {
            "就地卸下"
        } else {
            "停下手頭活計"
        };
        let sense_text = if st.kind == "dock_haul" {
            format!("肩頭沉甸甸地{}", st.label_zh)
        } else {
            format!("正在專心進行：{}", st.label_zh)
        };
        body_cues.push(json!({
            "kind": "activity",
            "text": sense_text,
            "action": {
                "id": "cancel_activity",
                "param": "",
                "label": act_label,
            }
        }));
        push_audit(
            "cancel_activity",
            "",
            act_label,
            "原物留於腳邊地面",
            "self",
            "self:activity",
        );
    }
    let has_ration = viewer
        .backpack
        .iter()
        .any(|i| i.slug == "rye_bread" && i.count > 0);
    if has_ration && (viewer.fatigue >= 20 || viewer.mind.stm.affect.hunger >= 35) {
        body_cues.push(json!({
            "kind": "hunger",
            "text": "腹中微飢，腳步漸覺沉重",
            "action": {
                "id": "eat_ration",
                "param": "",
                "label": "嚼塊乾糧",
            }
        }));
        push_audit(
            "eat_ration",
            "",
            "嚼塊黑麥乾糧",
            "隨身取食回氣",
            "self",
            "self:backpack",
        );
    }

    if viewer.active_skirmish.is_some() {
        return (objects, body_cues, audit_actions);
    }
    if viewer.active_transit.is_some() {
        push_audit(
            "transit_alight_next",
            "",
            "下一站停靠下車",
            "抵前方站口落腳",
            "self",
            "self:transit",
        );
        push_audit(
            "carriage_alight",
            "",
            "招呼就地停靠",
            "當場落腳步行",
            "self",
            "self:transit",
        );
        return (objects, body_cues, audit_actions);
    }

    // C1. 地面散落行囊（1 指彎腰拾起）
    if delta
        .dropped_loot_by_node
        .get(&node.id)
        .is_some_and(|v| !v.is_empty())
    {
        objects.push(json!({
            "object_id": "obj:dropped_bag",
            "name": "地面遺落行囊",
            "cue": "散落在石板上的粗布行囊與物件",
            "direct_action": { "id": "pickup_dropped_bag", "param": "" },
            "options": [],
        }));
        push_audit(
            "pickup_dropped_bag",
            "",
            "拾起地面遺落行囊",
            "彎腰拾回散落物",
            "object",
            "obj:dropped_bag",
        );
    }

    // C2. 廣場鎮志石碑（1 指直讀，零「細閱」廢話選單）
    if node.kind == "plaza" {
        objects.push(json!({
            "object_id": "obj:town_stele",
            "name": "青石鎮志碑",
            "cue": "碑面刻滿地方開埠沿革與風物紀事",
            "direct_action": { "id": "open_scroll_stele", "param": "" },
            "options": [],
        }));
        push_audit(
            "open_scroll_stele",
            "",
            "展讀青石鎮志碑",
            "地方沿革紀事",
            "object",
            "obj:town_stele",
        );
    }

    // C3. 酒館案頭兵燹抄本（1 指直讀）＋ 靠窗長椅 ＋ 掌櫃離櫃時之自助乾糧竹簍
    if node.kind == "shop" && node.name.contains("酒館") {
        objects.push(json!({
            "object_id": "obj:war_annals",
            "name": "海峽兵燹抄本",
            "cue": "橡木案頭攤開的厚皮手抄史冊",
            "direct_action": { "id": "open_scroll_annals", "param": "" },
            "options": [],
        }));
        push_audit(
            "open_scroll_annals",
            "",
            "翻閱海峽兵燹抄本",
            "諸國大戰與關稅本末",
            "object",
            "obj:war_annals",
        );

        if viewer.ongoing_activity.is_none() && viewer.fatigue >= 12 {
            objects.push(json!({
                "object_id": "obj:tavern_bench",
                "name": "靠窗橡木長椅",
                "cue": "爐火微暖，可坐下歇口氣聽鄰桌交談",
                "desc": "靠窗的厚實橡木長椅擦得乾淨，壁爐暖意烘得人雙肩放鬆。",
                "direct_action": Value::Null,
                "options": [{
                    "id": "rest_short",
                    "param": "",
                    "label": "靠椅短歇回氣",
                    "sub": "片刻消減奔波疲乏並旁聽大堂風聲",
                }],
            }));
            push_audit(
                "rest_short",
                "",
                "靠椅短歇回氣",
                "消減疲乏並旁聽交談",
                "object",
                "obj:tavern_bench",
            );
        }

        let staff_here = has_role_at_node(delta, &viewer.actor_id, &node.id, "酒館掌櫃")
            || has_role_at_node(delta, &viewer.actor_id, &node.id, "酒館掌櫃學徒");
        if !staff_here
            && super::live_tide::tavern_has_ration_stock(doc, delta, node.cell_id, &node.id)
        {
            let basket_price = super::live_commodities::quote_tavern_ration_copper(
                doc,
                delta,
                node.cell_id,
                &node.id,
            );
            let basket_param = format!("self_basket|{basket_price}");
            let basket_lbl = format!("投 {basket_price} 枚銅幣自取黑麥乾糧");
            objects.push(json!({
                "object_id": "obj:ration_basket",
                "name": "櫃頭黑麥乾糧竹簍",
                "cue": format!("旁設誠實投幣木筒・每包 {} 枚銅幣", basket_price),
                "desc": "掌櫃與學徒暫離櫃台片刻，竹簍裡擺著包好的黑麥乾糧，旁邊立著一隻帶鎖縫的投幣木筒。",
                "direct_action": Value::Null,
                "options": [{
                    "id": "buy_ration",
                    "param": basket_param,
                    "label": basket_lbl,
                    "sub": if basket_price > 2 { "存糧吃緊・隨行市調價" } else { "誠實投幣取走一包口糧" },
                }],
            }));
            push_audit(
                "buy_ration",
                &basket_param,
                &basket_lbl,
                "櫃頭竹簍自助取糧",
                "object",
                "obj:ration_basket",
            );
        }
    }

    // C4. 鍛爐出爐取件架（1 指直領已付費銘器）＆ 石砌鍛爐與鐵砧（玩家親手自鍛）
    let in_owned = viewer.owned_house_node.as_deref() == Some(node.id.as_str());
    let at_forge = node.name.contains("鍛鐵")
        || node.name.contains("鐵匠")
        || node.name.contains("木匠")
        || node.name.contains("集坊");
    if at_forge {
        let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
        let ready_vessel_name = delta.forge_orders.get(&node.id).and_then(|list| {
            list.iter()
                .find(|o| o.customer_id == viewer.actor_id && abs_bell >= o.ready_abs_bell)
                .map(|o| o.vessel_tool.name.clone())
        });
        if let Some(vname) = ready_vessel_name {
            objects.push(json!({
                "object_id": "obj:forge_rack",
                "name": "鍛爐出爐取件架",
                "cue": format!("架上擱著已淬火落款的【{vname}】"),
                "direct_action": { "id": "collect_custom_vessel", "param": "" },
                "options": [],
            }));
            push_audit(
                "collect_custom_vessel",
                "",
                "取走精鍛出爐銘器",
                "領回已付費成器",
                "object",
                "obj:forge_rack",
            );
        }

        if in_owned || !clock.is_night {
            let can_box = viewer.work_node == node.id || in_owned;
            let cnt_mat = |s: &str| -> u32 {
                let b: u32 = viewer
                    .backpack
                    .iter()
                    .filter(|i| i.slug == s && i.bound_owner.is_none() && i.stolen_from.is_none())
                    .map(|i| i.count)
                    .sum();
                let bx: u32 = if can_box {
                    viewer.inn_lockers.get(&node.id).map_or(0, |v| {
                        v.iter()
                            .filter(|i| {
                                i.slug == s && i.bound_owner.is_none() && i.stolen_from.is_none()
                            })
                            .map(|i| i.count)
                            .sum()
                    })
                } else {
                    0
                };
                b + bx
            };
            let n_iron = cnt_mat("iron_ore");
            let n_wood = cnt_mat("driftwood");
            let n_rune = cnt_mat("rune_stone");
            let n_beast = cnt_mat("spider_carapace") + cnt_mat("lizard_scale");
            if n_iron >= 2
                || n_wood >= 2
                || (n_iron >= 1 && n_wood >= 1)
                || (n_rune >= 1 && n_iron >= 1)
                || (n_beast >= 1 && (n_iron >= 1 || n_wood >= 1))
            {
                let shop_cold =
                    !in_owned && super::live_commodities::is_smithy_forge_cold(delta, &node.id);
                let has_own_coal = viewer.backpack.iter().any(|i| {
                    matches!(i.slug.as_str(), "black_coal" | "charcoal_sack")
                        && i.count > 0
                        && i.bound_owner.is_none()
                        && i.stolen_from.is_none()
                });
                if !shop_cold || has_own_coal {
                    objects.push(json!({
                        "object_id": "obj:forge_hearth",
                        "name": "石砌鍛爐與鐵砧",
                        "cue": if shop_cold { "爐炭熄火，可添入隨身精煤木炭開爐鍛打" } else { "紅炭正熾，可就著鐵砧親手鍛打器物" },
                        "desc": "沉重的黑鐵砧座立在石爐旁，手頭備有礦石、硬木或甲殼且爐內有炭時可親手開爐鍛打落款器物。",
                        "direct_action": Value::Null,
                        "options": [{
                            "id": "forge_vessel",
                            "param": "",
                            "label": "親手開爐鍛打落款器物",
                            "sub": "消耗隨身礦木甲殼鍛出新器",
                        }],
                    }));
                    push_audit(
                        "forge_vessel",
                        "",
                        "親手開爐鍛打落款器物",
                        "借爐自鍛新器",
                        "object",
                        "obj:forge_hearth",
                    );
                } else {
                    objects.push(json!({
                        "object_id": "obj:forge_hearth",
                        "name": "石砌鍛爐與鐵砧",
                        "cue": "爐火已熄・精煤木炭用罄（須先補炭方能開爐）",
                        "desc": "鐵砧旁炭筐見底、石爐只餘冷灰；不論掌錘師傅或旅人自鍛，皆須先備妥精煤或木炭生火。",
                        "direct_action": Value::Null,
                        "options": [],
                    }));
                }
            }
        }
    }

    // C5. 公櫃掛牌木欄（廣場與酒館常設：逾時搜救契、街坊託付單、懸賞收購、廣場拾遺登記）
    if node.kind == "plaza" || (node.kind == "shop" && node.name.contains("酒館")) {
        let mut board_opts: Vec<Value> = Vec::new();
        if !viewer.rescue_pact_active {
            board_opts.push(json!({
                "id": "buy_rescue_pact",
                "param": "public_board|5",
                "label": "簽立逾時搜救契 · 5 枚銅幣",
                "sub": "荒野坑道力竭時由巡林隊抬回草藥堂",
            }));
            push_audit(
                "buy_rescue_pact",
                "public_board|5",
                "簽立逾時搜救契 · 5 枚銅幣",
                "公櫃安全契約",
                "object",
                "obj:public_board",
            );
        }
        if node.kind == "plaza" && viewer.backpack.iter().any(|i| i.stolen_from.is_some()) {
            board_opts.push(json!({
                "id": "declare_found_bag",
                "param": "public_board",
                "label": "於招領簿登記拾遺物件",
                "sub": "加蓋公櫃章記轉正當留用",
            }));
            push_audit(
                "declare_found_bag",
                "public_board",
                "於招領簿登記拾遺物件",
                "公櫃拾遺報備",
                "object",
                "obj:public_board",
            );
        }
        for c in delta.contracts.values() {
            if !super::pacts_and_peers::is_escrow_pact_kind(&c.kind) {
                continue;
            }
            let (meta_t, desc) = super::pacts_and_peers::parse_pact_note(&c.note_zh);
            let active_open_or_mine = (c.status == "escrowed" && c.debtor_id.is_empty())
                || (c.status == "in_progress" && c.debtor_id == viewer.actor_id);
            let can_fulfill_here = active_open_or_mine
                && c.creditor_id != viewer.actor_id
                && match c.kind.as_str() {
                    "shop_cover" => meta_t == node.id,
                    "supply_commission" => {
                        delta.actors.get(&c.creditor_id).is_some_and(|iss| {
                            iss.alive && (iss.node_id == node.id || iss.work_node == node.id)
                        }) && viewer.backpack.iter().any(|i| {
                            i.slug == meta_t
                                && i.count > 0
                                && i.bound_owner.is_none()
                                && i.stolen_from.is_none()
                        })
                    }
                    "escort_watch" => {
                        c.status == "in_progress"
                            && c.debtor_id == viewer.actor_id
                            && meta_t == node.id
                            && delta
                                .actors
                                .get(&c.creditor_id)
                                .is_some_and(|iss| iss.alive && iss.node_id == node.id)
                    }
                    _ => false,
                };
            let short_desc: String = desc.chars().take(18).collect();
            if can_fulfill_here {
                let lbl = format!("履約交差領取 {} 枚銅幣", c.amount_copper);
                board_opts.push(json!({
                    "id": "fulfill_pact",
                    "param": c.contract_id,
                    "label": lbl,
                    "sub": short_desc,
                }));
                push_audit(
                    "fulfill_pact",
                    &c.contract_id,
                    &lbl,
                    &short_desc,
                    "object",
                    "obj:public_board",
                );
            } else if c.status == "escrowed"
                && c.creditor_id != viewer.actor_id
                && delta
                    .actors
                    .get(&c.creditor_id)
                    .and_then(|iss| doc.nodes.get(&iss.node_id).map(|n| n.cell_id))
                    == Some(node.cell_id)
            {
                let lbl = format!("揭牌接下託付（保付 {} 枚銅幣）", c.amount_copper);
                board_opts.push(json!({
                    "id": "accept_pact",
                    "param": c.contract_id,
                    "label": lbl,
                    "sub": short_desc,
                }));
                push_audit(
                    "accept_pact",
                    &c.contract_id,
                    &lbl,
                    &short_desc,
                    "object",
                    "obj:public_board",
                );
            }
        }
        if viewer.copper >= 8 {
            let has_my_supply = delta.contracts.values().any(|c| {
                c.creditor_id == viewer.actor_id
                    && c.kind == "supply_commission"
                    && (c.status == "escrowed" || c.status == "in_progress")
            });
            if !has_my_supply {
                board_opts.push(json!({
                    "id": "propose_supply_pact",
                    "param": "iron_ore",
                    "label": "掛牌懸賞收購鐵礦 · 8 枚銅幣",
                    "sub": "預存銀號保付，由街坊採礦交收",
                }));
                push_audit(
                    "propose_supply_pact",
                    "iron_ore",
                    "掛牌懸賞收購鐵礦 · 8 枚銅幣",
                    "銀號保付託付",
                    "object",
                    "obj:public_board",
                );
            }
        }
        if !board_opts.is_empty() {
            objects.push(json!({
                "object_id": "obj:public_board",
                "name": "公櫃掛牌木欄",
                "cue": format!("木欄上釘著 {} 項契約與巡防牌記", board_opts.len()),
                "desc": "市政銀號與街坊共用的樟木掛牌欄，一側設有巡林搜救契投幣匣，另一側釘著銀號保付的街坊託付木牌。",
                "direct_action": Value::Null,
                "options": board_opts,
            }));
        }
    } else {
        for c in delta.contracts.values() {
            if !super::pacts_and_peers::is_escrow_pact_kind(&c.kind) {
                continue;
            }
            let (meta_t, desc) = super::pacts_and_peers::parse_pact_note(&c.note_zh);
            let active_open_or_mine = (c.status == "escrowed" && c.debtor_id.is_empty())
                || (c.status == "in_progress" && c.debtor_id == viewer.actor_id);
            if active_open_or_mine && c.creditor_id != viewer.actor_id && meta_t == node.id {
                let short_desc: String = desc.chars().take(18).collect();
                let lbl = format!("履約交差領取 {} 枚銅幣", c.amount_copper);
                objects.push(json!({
                    "object_id": format!("obj:pact:{}", c.contract_id),
                    "name": "店堂託付木牌",
                    "cue": short_desc,
                    "desc": desc,
                    "direct_action": Value::Null,
                    "options": [{
                        "id": "fulfill_pact",
                        "param": c.contract_id,
                        "label": lbl,
                        "sub": "完成託付領取保付賞銀",
                    }],
                }));
                push_audit(
                    "fulfill_pact",
                    &c.contract_id,
                    &lbl,
                    &short_desc,
                    "object",
                    "obj:shop_pact",
                );
            }
        }
    }

    // C6. 二樓客房或自有宅院木床
    if node.kind == "inn_room" || in_owned {
        let mut bed_opts = vec![json!({
            "id": "rest_bed",
            "param": "",
            "label": if node.kind == "inn_room" { "寬衣上床安睡" } else { "步入臥榻安睡" },
            "sub": "沉睡一覺消除奔波疲乏並整理見聞",
        })];
        push_audit(
            "rest_bed",
            "",
            "寬衣上床安睡",
            "消除奔波疲乏",
            "object",
            "obj:bed",
        );
        if node.kind == "inn_room" && viewer.ongoing_activity.is_none() && viewer.fatigue >= 12 {
            bed_opts.push(json!({
                "id": "rest_short",
                "param": "",
                "label": "靠坐床沿短歇回氣",
                "sub": "片刻舒緩腿力並靜聽樓下動靜",
            }));
            push_audit(
                "rest_short",
                "",
                "靠坐床沿短歇回氣",
                "片刻回氣",
                "object",
                "obj:bed",
            );
        }
        objects.push(json!({
            "object_id": "obj:bed",
            "name": if node.kind == "inn_room" { "客房乾燥木床" } else { "自家石木臥榻" },
            "cue": "鋪著乾淨亞麻草墊，可安心歇息",
            "desc": "乾燥厚實的松木床架鋪著曬過太陽的亞麻被褥，床頭旁便是帶銅鎖的厚木箱。",
            "direct_action": Value::Null,
            "options": bed_opts,
        }));
    }

    // C7. 官道避雨茶棚粗茶桶 ＆ 南岸檢疫木欄交接台
    if node.kind == "tea_shed" {
        objects.push(json!({
            "object_id": "obj:tea_barrel",
            "name": "粗陶熱苦艾茶桶",
            "cue": "冒著微苦熱氣・每碗 1 枚銅幣",
            "desc": "避雨木棚下支著一隻保溫粗陶茶桶，過路商旅與車夫常投一枚銅幣舀碗熱苦艾茶暖身歇腳。",
            "direct_action": Value::Null,
            "options": [{
                "id": "drink_tea_shed",
                "param": "",
                "label": "舀碗熱苦艾粗茶歇腳 · 1 枚銅幣",
                "sub": "回氣暖身並聽聽官道近況",
            }],
        }));
        push_audit(
            "drink_tea_shed",
            "",
            "舀碗熱苦艾粗茶歇腳 · 1 枚銅幣",
            "回氣聽行情",
            "object",
            "obj:tea_barrel",
        );
    }
    if node.kind == "survivor_palisade" {
        let has_letter = viewer
            .backpack
            .iter()
            .any(|i| i.slug == "plague_family_letter" && i.count > 0);
        let (lbl, sub) = if has_letter {
            ("隔欄遞交三二六年家書", "換取兩囊濃煎苦艾湯")
        } else {
            ("隔欄換取濃煎苦艾湯 · 2 枚銅幣", "消乏清瘟")
        };
        objects.push(json!({
            "object_id": "obj:palisade_table",
            "name": "檢疫木欄交接台",
            "cue": "石灰木欄間設有不接觸遞物木盤",
            "desc": "灑滿白石灰的雙層木欄之間架著一塊長柄推拉木盤，供病癒孤莊居民與外人隔欄換藥捎信。",
            "direct_action": Value::Null,
            "options": [{
                "id": "survivor_barrier",
                "param": "",
                "label": lbl,
                "sub": sub,
            }],
        }));
        push_audit(
            "survivor_barrier",
            "",
            lbl,
            sub,
            "object",
            "obj:palisade_table",
        );
    }

    // C8. 待售空屋房契木牌 ＆ 街旁可營造空置宅基
    if node.kind == "house_vacant"
        && viewer.owned_house_node.is_none()
        && !delta.sold_houses.contains_key(&node.id)
    {
        let price = doc
            .households
            .iter()
            .find(|h| h.node_id == node.id)
            .map_or(360, |h| h.price_copper);
        let lbl = format!("交納 {} 枚銅幣買下房契", price);
        objects.push(json!({
            "object_id": "obj:vacant_deed",
            "name": "待售石木屋舍木牌",
            "cue": format!("市政廳掛牌契價 {} 枚銅幣", price),
            "desc": "門框旁釘著加蓋港務火漆的待售木牌，買下房契後即為島上自有安身居所，附設私人臥榻與儲物大木箱。",
            "direct_action": Value::Null,
            "options": [{
                "id": "buy_vacant_house",
                "param": "",
                "label": lbl,
                "sub": "領取市政廳正式房契落戶",
            }],
        }));
        push_audit(
            "buy_vacant_house",
            "",
            &lbl,
            "購置自有屋舍",
            "object",
            "obj:vacant_deed",
        );
    }
    if matches!(
        node.kind.as_str(),
        "plaza" | "alley" | "village_gate" | "yard" | "road"
    ) && !node.indoor
        && !node.dungeon
        && viewer.owned_house_node.is_none()
    {
        let wood_n: u32 = viewer
            .backpack
            .iter()
            .filter(|i| i.slug == "driftwood" && i.bound_owner.is_none() && i.stolen_from.is_none())
            .map(|i| i.count)
            .sum();
        let iron_n: u32 = viewer
            .backpack
            .iter()
            .filter(|i| i.slug == "iron_ore" && i.bound_owner.is_none() && i.stolen_from.is_none())
            .map(|i| i.count)
            .sum();
        if wood_n >= 2 && iron_n >= 1 {
            objects.push(json!({
                "object_id": "obj:homestead_plot",
                "name": "街旁空置石基宅地",
                "cue": "地基平整，手頭木鐵齊備可立柱造屋",
                "desc": "街巷側邊留有一方平整的石砌空基，備妥兩份漂流硬木、一份鐵礦與工料費即可立柱起造自有新宅。",
                "direct_action": Value::Null,
                "options": [{
                    "id": "build_homestead",
                    "param": "",
                    "label": "立柱營造自有石木新宅 · 120 枚銅幣",
                    "sub": "耗漂流木 2 份、鐵礦 1 份與工料費",
                }],
            }));
            push_audit(
                "build_homestead",
                "",
                "立柱營造自有石木新宅 · 120 枚銅幣",
                "立柱造屋",
                "object",
                "obj:homestead_plot",
            );
        }
    }

    // C8b. 店鋪門前封爐頂讓木牌（待頂讓時任何人可付費承租；現任經營者亦可主動封爐離鋪）
    if node.kind == "shop" {
        if let Some(lease) = delta.shop_leases.get(&node.id)
            && lease.state == super::shop_leases::ShopLeaseState::SealedForLease
            && lease.operator_actor_id.is_none()
        {
            let fee = lease.transfer_fee_copper.max(0);
            let lbl = format!("付 {fee} 枚銅幣頂下【{}】經營權", node.name);
            objects.push(json!({
                "object_id": "obj:shop_lease_board",
                "name": "鋪門封條與待價頂讓木牌",
                "cue": format!("原主事因「{}」封鋪出走・頂讓金 {} 枚銅幣", lease.seal_reason_zh, fee),
                "desc": "門板封著麻繩與木籤牌，寫明鋪面現貨箱、門牌契據與轉讓金數額；凡未另掌鋪面者皆可出資頂下開張營業。",
                "direct_action": Value::Null,
                "options": [{
                    "id": "lease_vacant_shop",
                    "param": node.id.clone(),
                    "label": lbl,
                    "sub": "成為新任鋪主並把頂讓金匯交原東家",
                }],
            }));
            push_audit(
                "lease_vacant_shop",
                &node.id,
                &lbl,
                "承租掛牌店鋪",
                "object",
                "obj:shop_lease_board",
            );
        } else if super::shop_leases::actor_operates_shop(delta, &viewer.actor_id, &node.id) {
            objects.push(json!({
                "object_id": "obj:shop_lease_board",
                "name": "掌鋪木牌與封爐門栓",
                "cue": "你現為本鋪當家主事，可隨時暫封鋪門掛牌頂讓、提原有傢伙離鋪闖路",
                "desc": "將鐵牌翻向「暫封待頂讓」，屋契所有權仍歸原東家名下，待有緣旅人或居民出資承租接手。",
                "direct_action": Value::Null,
                "options": [{
                    "id": "seal_shop_venture",
                    "param": node.id.clone(),
                    "label": "暫封鋪門掛牌頂讓（16 銅）",
                    "sub": "保住手頭傢伙卸下櫃檯離鋪闖路",
                }],
            }));
            push_audit(
                "seal_shop_venture",
                &node.id,
                "暫封鋪門掛牌頂讓（16 銅）",
                "封爐掛牌離鋪",
                "object",
                "obj:shop_lease_board",
            );
        }
    }

    // C9. 靠岸潮汐商船（早午潮停泊棧橋卸貨）＋ 跨海渡輪 ＋ 停靠班車 vs 站口時刻木牌
    if node.kind == "dock"
        && super::live_tide::is_dock_cargo_tide_open(clock)
        && !viewer
            .explored_windows
            .contains(&super::live_tide::dock_shift_window_key(
                clock.game_days,
                clock.bell_index,
            ))
    {
        objects.push(json!({
            "object_id": "obj:cargo_ship",
            "name": "靠岸雙桅商船",
            "cue": "跳板已放妥，水手長正招人搭手卸下鹽漬貨箱 · 4 枚銅幣",
            "desc": "趁著當班潮水靠妥木石棧橋的雙桅貨船，甲板水手正將鹽漬貨箱往棧橋搬卸，缺人手搭一把。",
            "direct_action": Value::Null,
            "options": [{
                "id": "work_dock",
                "param": "",
                "label": "搭手搬卸靠岸船貨 · 4 枚銅幣",
                "sub": "本班潮汐靠岸卸船工錢",
            }],
        }));
        push_audit(
            "work_dock",
            "",
            "搭手搬卸靠岸船貨 · 4 枚銅幣",
            "靠岸商船卸貨",
            "object",
            "obj:cargo_ship",
        );
    }

    if node.id.starts_with("oi:west_strait_ferry") && !clock.is_night {
        objects.push(json!({
            "object_id": "obj:cross_strait_ferry",
            "name": "跨海雙桅公營渡輪",
            "cue": "停泊長棧橋畔・跨海船資 25 枚銅幣",
            "desc": "吃水深穩的雙桅橫帆渡輪正繫在重木長棧橋旁裝卸木桶，順風潮起即可橫渡西海峽前往蒼淵主陸。",
            "direct_action": Value::Null,
            "options": [{
                "id": "inquire_ferry",
                "param": "",
                "label": "向掌纜官洽詢登船 · 25 枚銅幣",
                "sub": "核驗跨海船資與啟航潮汐",
            }],
        }));
        push_audit(
            "inquire_ferry",
            "",
            "洽詢跨海雙桅渡輪 · 25 枚銅幣",
            "跨海船班",
            "object",
            "obj:cross_strait_ferry",
        );
    }

    let signboards = super::transit_and_cargo::build_node_transit_signboards(doc, delta, &node.id);
    let mut remote_post_opts: Vec<Value> = Vec::new();
    for sb in signboards {
        let lid = sb["line_id"].as_str().unwrap_or("");
        let lname = sb["line_name"].as_str().unwrap_or("班車");
        let vname = sb["vehicle_name"].as_str().unwrap_or(lname);
        let fare = sb["fare_copper"].as_i64().unwrap_or(2);
        let at_now = sb["at_node_now"].as_bool().unwrap_or(false);
        let st = sb["status_zh"].as_str().unwrap_or("");
        let opt_label = format!("登乘【{vname}】 · {fare} 枚銅幣");
        if at_now {
            let oid = format!("obj:veh:{lid}");
            objects.push(json!({
                "object_id": oid,
                "name": vname,
                "cue": format!("{st}・票價 {fare} 枚銅幣"),
                "desc": format!("【{vname}】（{lname}）此刻正停靠在眼前上下客與裝卸貨箱。"),
                "direct_action": Value::Null,
                "options": [{
                    "id": "board_transit",
                    "param": lid,
                    "label": opt_label,
                    "sub": st,
                }],
            }));
            push_audit("board_transit", lid, &opt_label, st, "object", &oid);
        } else {
            let hail_lbl = format!("候乘／招手攔搭【{lname}】 · {fare} 枚銅幣");
            remote_post_opts.push(json!({
                "id": "board_transit",
                "param": lid,
                "label": hail_lbl,
                "sub": st,
            }));
            push_audit(
                "board_transit",
                lid,
                &hail_lbl,
                st,
                "object",
                "obj:transit_signpost",
            );
        }
    }

    if (node.name.contains("驛馬") || node.name.contains("車馬")) && !clock.is_night {
        if !node.id.starts_with("oi:west_strait_ferry") {
            remote_post_opts.push(json!({
                "id": "board_carriage_west",
                "param": "",
                "label": "搭乘幹線篷車往西峽渡口",
                "sub": "順東西官道西行",
            }));
            push_audit(
                "board_carriage_west",
                "",
                "搭乘幹線篷車往西峽渡口",
                "東西官道西行",
                "object",
                "obj:transit_signpost",
            );
        }
        if !node.id.starts_with("oi:white_gull_harbor") {
            remote_post_opts.push(json!({
                "id": "board_carriage_east",
                "param": "",
                "label": "搭乘幹線篷車回白鷗港",
                "sub": "順東西官道東行",
            }));
            push_audit(
                "board_carriage_east",
                "",
                "搭乘幹線篷車回白鷗港",
                "東西官道東行",
                "object",
                "obj:transit_signpost",
            );
        }
    }
    if !remote_post_opts.is_empty() {
        let is_sea_post = node.kind == "dock";
        objects.push(json!({
            "object_id": "obj:transit_signpost",
            "name": if is_sea_post { "棧橋船班木牌" } else { "官道車馬站牌" },
            "cue": if is_sea_post { "標示沿岸貨駁航向與靠岸動靜" } else { "標示往來篷車路線與班次動靜" },
            "desc": "立在路側的風雨木牌，寫明往來各鎮村莊的班線票價，車船鄰近時可直接招手登乘。",
            "direct_action": Value::Null,
            "options": remote_post_opts,
        }));
    }

    super::mesh_expedition::append_expedition_scene_objects(
        node,
        clock,
        &mut objects,
        &mut push_audit,
    );
    super::live_land::append_live_land_scene_objects(
        doc,
        delta,
        viewer,
        node,
        &mut objects,
        &mut push_audit,
    );
    super::live_wilds::append_live_wilds_scene_objects(
        delta,
        viewer,
        node,
        clock,
        &mut objects,
        &mut push_audit,
    );
    super::live_lifecycle::append_live_lifecycle_scene_objects(
        delta,
        node,
        &mut objects,
        &mut push_audit,
    );
    super::body_gear_craft::append_gear_and_forage_scene_objects(
        doc,
        delta,
        viewer,
        node,
        clock,
        &mut objects,
        &mut push_audit,
    );

    (objects, body_cues, audit_actions)
}
