//! 第一刀核心器官：人物真活外顯、同場活群戲、NPC 主動搭話與「嘴動→身動」對話世界後果引擎。
//!
//! 嚴守 `DESIGN_KEYWORDS.md` 鐵律：
//! 1. 不捏造後端不存在的事實——所有外顯舉止、現場群戲與主動招呼皆從真實債務、缺料、關係網、行囊與處境派生；
//! 2. 靜態文字佐圖片世界裡，文字即現場鏡頭——把同場人物的摩擦、商議與對玩家的主動反應直接浮現於主舞台並可當場插話；
//! 3. 對話與身體接通——玩家開口說服、報信、問路、請客致歉或挑釁，皆由 Rust 依真實性格與守恆規則裁決並落地後果。

use serde_json::{Value, json};

use super::appraisal::ConcernCause;
use super::clock_and_lang::SolarClockSnapshot;
use super::mind::{EventPublicity, EvidenceTier};
use super::movement::{add_language_exposure, push_breath};
use super::persona::{InnatePersona, ValueAxis};
use super::shop_leases::infer_shop_kind;
use super::sim_tick::broadcast_scene_event;
use super::types::{IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};

fn short_node_label(name: &str) -> &str {
    name.split('・').next_back().unwrap_or(name)
}

/// 取代早期 `soul_seed % 3` 三句輪播貼紙：依此刻真實處境、正在執行的活計、鋪倉危機、真實債契、
/// 同場關係張力與對眼前觀察者的反應，動態合成非讀心術外顯舉止。
pub fn build_living_npc_cue(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    viewer: &UnifiedMindBody,
    other: &UnifiedMindBody,
    innate: &InnatePersona,
) -> String {
    let aff = &other.mind.stm.affect;
    let eff_fatigue = aff.fatigue.max(other.fatigue);
    let node_opt = doc.nodes.get(&other.node_id);
    let node_kind = node_opt.map_or("street", |n| n.kind.as_str());
    let is_indoor = node_opt.is_some_and(|n| n.indoor);

    // 1. 正在執行實體工序（扛貨／打鐵／短歇）
    if let Some(ref act) = other.ongoing_activity {
        if act.kind == "dock_haul" {
            let dst = doc
                .nodes
                .get(&act.target_node)
                .map_or("城內店鋪", |n| short_node_label(&n.name));
            return format!("肩扛沉甸木箱，步履穩健地正往{}送貨", dst);
        }
        return format!("正埋首專注於「{}」", act.label_zh);
    }

    // 2. 正在趕路（依真實意圖反映腳步神色）
    if let Some(next_nid) = other.route_plan.front()
        && let Some(nd) = doc.nodes.get(next_nid)
    {
        let short_dst = short_node_label(&nd.name);
        let last_intent = other.mind.stm.intentions.back().map(|i| i.kind.as_str());
        return match last_intent {
            Some("detour_avoid") => format!("壓低帽簷刻意避開人群，快步繞往{}", short_dst),
            Some("forage_wild") => format!("挎著粗麻布袋與{}，正趕往{}採料", other.bound_weapon.name, short_dst),
            Some("sell_foraged") => format!("背囊鼓脹沉手，正趕往{}脫手山貨", short_dst),
            Some("visit_confidant") => format!("眉頭微鎖似有心事，朝{}方向尋人", short_dst),
            Some("confront_chase") => format!("面色鐵青大步流星，直奔{}似要找人理論", short_dst),
            Some("plead_extension") | Some("restock_larder") => {
                format!("神色焦急、步子邁得極快，正趕往{}", short_dst)
            }
            Some("go_home_sleep") | Some("shelter_hearth") => {
                format!("收妥手邊{}，沿街往{}歇息", other.bound_weapon.name, short_dst)
            }
            _ => {
                if aff.anger >= 38 {
                    format!("臉色沉重、腳步落地有聲地走向{}", short_dst)
                } else if aff.fear >= 36 {
                    format!("不時回望街角，緊按衣袋趕往{}", short_dst)
                } else {
                    format!("手提{}，正不疾不徐往{}走去", other.bound_weapon.name, short_dst)
                }
            }
        };
    }

    // 3. 入夜作息
    if clock.is_night {
        let sleep_home = other
            .owned_house_node
            .as_deref()
            .unwrap_or(other.home_node.as_str());
        let at_sleep_spot =
            other.node_id == sleep_home || (node_kind == "inn_room" && other.inn_nights_left > 0);
        if at_sleep_spot {
            if node_kind == "shop" && other.role.contains("酒館") {
                return if clock.hour < 6 {
                    "守著壁爐餘炭在櫃台後小憩打盹".to_string()
                } else {
                    "在暖黃油燈下擦拭木杯，抬眼招呼夜裡進門的熟客".to_string()
                };
            }
            if node_kind == "shop" {
                return "鋪門已上栓落鎖，退入後堂熄燈歇息".to_string();
            }
            if is_indoor {
                return "已在屋內吹熄油燈安歇".to_string();
            }
        }
        if other.role.contains("衛兵") {
            return format!("提著防風夜燈、手按{}沿街口石階緩步巡守", other.bound_weapon.name);
        }
        if node_kind == "shop" && other.role.contains("酒館") {
            return "倚在暖爐光影旁小酌暖身，不時聽著鄰桌閒談".to_string();
        }
        return "裹緊衣襟站在避風牆角下，借著街燈餘光緩口氣".to_string();
    }

    // 4. 鋪面真實供需危機（冷爐、缺藥、斷糧、待卸貨箱）
    if node_kind == "shop"
        && other.work_node == other.node_id
        && let Some(nd) = node_opt
    {
        match infer_shop_kind(nd) {
            "smithy" if super::live_commodities::is_smithy_forge_cold(delta, &nd.id) => {
                return "對著徹底冷透的石爐皺眉翻檢炭簍，顯然急缺精煤木炭".to_string();
            }
            "apothecary" if super::live_commodities::is_apothecary_out_of_herbs(delta, &nd.id) => {
                return "望著空了大半的藥屜輕嘆，正把剩餘研缽收進木匣".to_string();
            }
            "tavern"
                if !super::live_tide::tavern_has_ration_stock(doc, delta, nd.cell_id, &nd.id) =>
            {
                return "站在灶台邊清點見底的麥袋，嘴裡嘀咕著糧車怎麼還沒進城".to_string();
            }
            _ => {}
        }
    }
    if node_kind == "dock"
        && other.work_node == other.node_id
        && delta
            .dock_cargo_piles
            .get(&other.node_id)
            .is_some_and(|p| !p.is_empty())
    {
        return "正站在堆起的鋪貨木箱旁核對木籌，四處張望找肯扛貨的幫手".to_string();
    }

    // 5. 同場真實人際互動（與在場另一位具名人物的嫌隙／債契／親故）
    if let Some(occ) = delta.node_occupants.get(&other.node_id) {
        // 5a. 同場遇見欠債或債主
        for c in delta.contracts.values() {
            if !matches!(c.status.as_str(), "open" | "defaulted") {
                continue;
            }
            if c.creditor_id == other.actor_id && occ.contains(&c.debtor_id) {
                if let Some(debtor) = delta.actors.get(&c.debtor_id) {
                    return format!(
                        "目光緊鎖場中另一側的{}，指節在袖口上敲得作響",
                        debtor.display_name
                    );
                }
            }
            if c.debtor_id == other.actor_id && occ.contains(&c.creditor_id) {
                if let Some(cred) = delta.actors.get(&c.creditor_id) {
                    return format!(
                        "餘光瞟見{}在場，刻意側身縮在人群邊緣避開視線",
                        cred.display_name
                    );
                }
            }
        }
        // 5b. 同場遇見嫌隙對象或同家熟人
        let mut kin_peer: Option<&str> = None;
        for oid in occ {
            if oid == &other.actor_id || oid == &viewer.actor_id {
                continue;
            }
            let Some(peer) = delta.actors.get(oid) else {
                continue;
            };
            if !peer.alive {
                continue;
            }
            if let Some(soc) = other.mind.ltm.social.get(oid) {
                if soc.warmth <= -22 {
                    return if innate.temperament.approach_avoid >= 0 {
                        format!("抱臂冷眼盯著不遠處的{}，下顎微微繃緊", peer.display_name)
                    } else {
                        format!("刻意背過身避開{}的視線，低頭擺弄手邊雜物", peer.display_name)
                    };
                }
                if soc.warmth >= 45 && kin_peer.is_none() {
                    kin_peer = Some(peer.display_name.as_str());
                }
            }
            if !other.household_id.is_empty()
                && peer.household_id == other.household_id
                && kin_peer.is_none()
            {
                kin_peer = Some(peer.display_name.as_str());
            }
        }
        if let Some(kname) = kin_peer {
            return format!("正與身旁的{}湊在一處低聲商量著今日營生", kname);
        }
    }

    // 6. 對玩家（觀察者）的真實當場反應
    if let Some(soc_v) = other.mind.ltm.social.get(&viewer.actor_id) {
        if soc_v.warmth <= -18 || soc_v.pressure_tactic_count >= 2 {
            return "瞥見你走近，臉色微沉、手下意識按向腰間物件".to_string();
        }
        if soc_v.acquainted && soc_v.warmth >= 25 {
            return "一眼認出你來，神色舒緩地朝你微微點頭".to_string();
        }
    }
    if viewer.wound_part.is_some() && other.role.contains("草藥") {
        return "一眼瞥見你衣袖血痕，目光停在傷處眉頭微皺".to_string();
    }

    // 7. 依真實掛心事（Concern）、器物、負傷與情緒呈現鮮明個體姿態
    if let Some(ref wp) = other.wound_part {
        return format!("護著尚在隱痛的{}，動作比平日放慢了幾分", wp);
    }
    if let Some(top) = other.mind.top_concern()
        && top.intensity >= 34
    {
        return match top.cause {
            ConcernCause::DebtDefaulted | ConcernCause::DebtPressured => {
                "眉心擰成川字，不時低頭盤算懷中銅錢與到期帳日".to_string()
            }
            ConcernCause::LivelihoodShortfall => {
                "盯著街面攤檔價目暗自盤算，神色透著幾分拮據".to_string()
            }
            ConcernCause::ToolDamaged => {
                format!("端詳著已有磨痕的{}，用拇指摩挲刃口邊緣", other.bound_weapon.name)
            }
            ConcernCause::CrossTownYearning => {
                "望著通往鎮外的大路出神，似乎在打聽遠方熟人的音訊".to_string()
            }
            ConcernCause::RivalEncounter | ConcernCause::KinshipRift | ConcernCause::PactBetrayed => {
                "指節攥得微白、唇線抿緊，像是剛吞下一口悶氣".to_string()
            }
            ConcernCause::CraftBreakthrough => {
                format!("神采透亮，正反覆掂量手中{}的分寸與重心", other.bound_weapon.name)
            }
            ConcernCause::KinshipWarmth | ConcernCause::PactHonored => {
                "眼角帶著難得的笑意，手腳俐落地整理著隨身行囊".to_string()
            }
        };
    }

    // 8. 仍帶有真實手頭器物／場所細節的日常舉止（不再使用 3 句空泛貼紙）
    if eff_fatigue >= 58 || aff.hunger >= 58 {
        return format!(
            "將{}擱在身旁石階上，抹去額角汗珠緩口氣",
            other.bound_weapon.name
        );
    }
    let tool = other.bound_weapon.name.as_str();
    match node_kind {
        "shop" if other.role.contains("鍛鐵") => {
            format!("守在熾熱鍛爐旁，手裡掂著{}察看鐵胚火色", tool)
        }
        "shop" if other.role.contains("酒館") => {
            "在橡木櫃台後燙洗粗陶杯，順耳聽著堂內旅人談天".to_string()
        }
        "shop" if other.role.contains("草藥") => {
            format!("在藥案前低頭揀選乾苦艾與草葉，手邊擺著{}", tool)
        }
        "shop" if other.role.contains("車行") => {
            "檢點篷車軸栓與馬具皮扣，偶爾抬眼望向南城門路況".to_string()
        }
        "dock" => format!("迎著鹹濕海風站在石棧邊，提著{}留心泊岸帆駁", tool),
        "gate" | "citadel" => format!("立在石門影下、手按{}，逐一打量過往行旅與貨車", tool),
        "bridge" => "扶著被海風磨亮的橋欄稍作歇腳，望著橋下水色流向港池".to_string(),
        "plaza" => format!("在市集石板間駐足端詳各攤成色，隨身帶著{}", tool),
        "wild" | "dungeon" => format!("壓低重心握緊{}，耳聽四周風聲與林間異動", tool),
        _ => format!("隨身佩著{}，在{}邊放緩腳步打量街坊動靜", tool, node_opt.map_or("街口", |n| short_node_label(&n.name))),
    }
}

/// 生成當前節點正在發生的「可旁聽、可插話、或人物主動向你搭話」現場活戲（上限 2 幕，100% 源自真實狀態）。
pub fn build_live_scene_happenings(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
) -> Vec<Value> {
    if viewer.active_skirmish.is_some() || viewer.active_transit.is_some() {
        return Vec::new();
    }
    let Some(occ) = delta.node_occupants.get(&node.id) else {
        return Vec::new();
    };
    let present: Vec<&UnifiedMindBody> = occ
        .iter()
        .filter(|&id| id != &viewer.actor_id)
        .filter_map(|id| delta.actors.get(id))
        .filter(|a| a.alive && a.setup_done && a.controller_id.is_none() && a.node_id == node.id)
        .collect();
    if present.is_empty() {
        return Vec::new();
    }

    let mut scenes: Vec<Value> = Vec::new();

    // 幕 1：NPC 主動向玩家出聲（帶傷、帶稀缺貨／拓片、夜巡盤查、熟人招呼、初抵港埠招呼）
    let mut callout: Option<Value> = None;

    // 1a. 玩家負傷且草藥師在場
    if let Some(ref wp) = viewer.wound_part
        && let Some(herb) = present.iter().find(|a| a.role.contains("草藥"))
    {
        let tok = herb.public_actor_token();
        callout = Some(json!({
            "id": format!("callout:wound:{}", tok),
            "kind": "npc_callout",
            "speaker": herb.display_name,
            "speaker_token": tok,
            "text": format!("{}抬眼瞥見你護著{}，主動朝你招手：「傷口別拖到進海風發炎，過來讓我瞧瞧！」", herb.display_name, wp),
            "options": [
                { "label": "上前讓他瞧看傷處", "act": "open_talk", "param": tok },
                { "label": "點頭謝過表示無礙", "act": "intervene_vignette", "param": format!("greet_back|{}", tok) }
            ]
        }));
    }

    // 1b. 玩家身懷鍛爐急需精煤／木炭且冷爐鐵匠在場
    let viewer_has_coal = viewer
        .backpack
        .iter()
        .any(|i| matches!(i.slug.as_str(), "black_coal" | "charcoal_sack" | "iron_ore") && i.count > 0);
    if callout.is_none()
        && viewer_has_coal
        && !clock.is_night
        && node.kind == "shop"
        && infer_shop_kind(node) == "smithy"
        && let Some(smith) = present.iter().find(|a| a.role.contains("鍛鐵"))
    {
        let tok = smith.public_actor_token();
        let urgent = super::live_commodities::is_smithy_forge_cold(delta, &node.id);
        let line = if urgent {
            format!(
                "{}目光一下子鎖定你背囊裡的炭石，快步迎上來：「這位朋友！我這爐剛斷炭熄火，你袋裡那批料肯不肯讓出來？我按行市多添價！」",
                smith.display_name
            )
        } else {
            format!(
                "{}瞄見你袋角露出的礦炭色澤，敲了敲鐵砧喊道：「剛從北林回來？好鐵好炭我這兒天天收，拿來過過秤！」",
                smith.display_name
            )
        };
        callout = Some(json!({
            "id": format!("callout:coal:{}", tok),
            "kind": "npc_callout",
            "speaker": smith.display_name,
            "speaker_token": tok,
            "text": line,
            "options": [
                { "label": "走上前亮貨談價", "act": "open_talk", "param": tok },
                { "label": "搖頭稱貨另有他用", "act": "intervene_vignette", "param": format!("greet_back|{}", tok) }
            ]
        }));
    }

    // 1c. 玩家身懷矮人古碑拓片且矮人長輩／工匠在場
    let viewer_has_rubbing = viewer
        .backpack
        .iter()
        .any(|i| i.slug == "stele_rubbing" && i.count > 0);
    if callout.is_none()
        && viewer_has_rubbing
        && let Some(dwarf) = present.iter().find(|a| a.race == "矮人")
    {
        let tok = dwarf.public_actor_token();
        callout = Some(json!({
            "id": format!("callout:rubbing:{}", tok),
            "kind": "npc_callout",
            "speaker": dwarf.display_name,
            "speaker_token": tok,
            "text": format!(
                "{}注意到你懷中露出半角炭拓麻紙，眉頭一挑低聲道：「慢著——那紙上拓的是先祖山嶽古文吧？從哪處石縫拓來的，拿給我瞧一眼！」",
                dwarf.display_name
            ),
            "options": [
                { "label": "取出古碑拓片請教", "act": "intervene_vignette", "param": format!("show_rubbing|{}", tok) },
                { "label": "先與他細聊", "act": "open_talk", "param": tok }
            ]
        }));
    }

    // 1d. 夜間街頭衛兵盤查生面孔
    if callout.is_none()
        && clock.is_night
        && !node.indoor
        && let Some(guard) = present.iter().find(|a| a.role.contains("衛兵"))
    {
        let tok = guard.public_actor_token();
        let known = viewer.is_acquainted_with(&guard.actor_id);
        let text = if known {
            format!(
                "{}提著防風夜燈認出是你，壓低嗓門道：「{}，夜深海風寒重，早些回屋或去鏽錨酒館烤烤火吧。」",
                guard.display_name, viewer.display_name
            )
        } else {
            format!(
                "{}提燈往你臉上一照，手按腰間短刃沉聲問道：「這麼晚還在{}走動？報個名號，你是哪條船下來的？」",
                guard.display_name,
                short_node_label(&node.name)
            )
        };
        callout = Some(json!({
            "id": format!("callout:nightguard:{}", tok),
            "kind": "npc_callout",
            "speaker": guard.display_name,
            "speaker_token": tok,
            "text": text,
            "options": [
                { "label": "拱手報上名號打聲招呼", "act": "intervene_vignette", "param": format!("greet_back|{}", tok) },
                { "label": "順道打聽夜間城中動靜", "act": "open_talk", "param": tok }
            ]
        }));
    }

    // 1e. 熟人重逢主動招呼，或初抵碼頭／酒館／市集有人熱情（或精明）招攬
    if callout.is_none() {
        if let Some(friend) = present.iter().find(|a| {
            a.mind
                .ltm
                .social
                .get(&viewer.actor_id)
                .is_some_and(|s| s.acquainted && s.warmth >= 15)
        }) {
            let tok = friend.public_actor_token();
            let recent_hint = friend
                .mind
                .ltm
                .episodic
                .iter()
                .rev()
                .map(|e| e.subjective_zh.as_str())
                .find(|s| !s.contains('〔') && !s.contains("你只勉強") && !s.contains("（暗記") && !s.contains("（牢記"))
                .unwrap_or("今兒港上人來人往，挺熱鬧的");
            callout = Some(json!({
                "id": format!("callout:reunion:{}", tok),
                "kind": "npc_callout",
                "speaker": friend.display_name,
                "speaker_token": tok,
                "text": format!(
                    "{}在人群裡認出你，笑著抬手招呼：「{}！正好碰上你——說起近況，{}。」",
                    friend.display_name,
                    viewer.display_name,
                    recent_hint.trim_end_matches('。')
                ),
                "options": [
                    { "label": "走過去寒暄敘舊", "act": "open_talk", "param": tok },
                    { "label": "頷首回禮記個照面", "act": "intervene_vignette", "param": format!("greet_back|{}", tok) }
                ]
            }));
        } else if matches!(node.kind.as_str(), "dock" | "shop" | "plaza")
            && let Some(host) = present.first()
        {
            let tok = host.public_actor_token();
            let shown_name = if viewer.is_acquainted_with(&host.actor_id)
                || (host.work_node == node.id && matches!(node.kind.as_str(), "shop" | "dock" | "gate"))
            {
                host.display_name.clone()
            } else {
                format!("一位{}", host.role)
            };
            let line = if node.kind == "dock" {
                format!(
                    "{}剛繫緊纜繩，瞥見你拎著{}，隨口朝你搭腔：「剛靠岸的生面孔？往西過白鷗市集館就是廣場，想賺點銅板先問問碼頭記籌的！」",
                    shown_name, viewer.bound_weapon.name
                )
            } else if node.kind == "shop" && node.name.contains("酒館") {
                format!(
                    "{}擦著木杯抬頭望向門口：「外頭海風涼，進門先喝口熱湯暖胃吧！要住店還是打聽北林、渡口的風聲？」",
                    shown_name
                )
            } else {
                format!(
                    "{}見你在{}四下張望，順嘴搭了句話：「找人還是看貨？這會兒街坊都在各自忙活，有要事只管開口問。」",
                    shown_name,
                    short_node_label(&node.name)
                )
            };
            callout = Some(json!({
                "id": format!("callout:welcome:{}", tok),
                "kind": "npc_callout",
                "speaker": shown_name,
                "speaker_token": tok,
                "text": line,
                "options": [
                    { "label": "順勢上前搭話結識", "act": "open_talk", "param": tok },
                    { "label": "客氣點頭致意", "act": "intervene_vignette", "param": format!("greet_back|{}", tok) }
                ]
            }));
        }
    }

    if let Some(c) = callout {
        scenes.push(c);
    }

    // 幕 2：同場兩名真實居民之間的活群戲（真實債契催討 / 真實嫌隙對峙 / 同家或街坊現場商議）
    if present.len() >= 2 {
        let mut duo_scene: Option<Value> = None;

        // 2a. 同場有真實債契雙方
        for c in delta.contracts.values() {
            if !matches!(c.status.as_str(), "open" | "defaulted") {
                continue;
            }
            if let (Some(cred), Some(debtor)) = (
                present.iter().find(|a| a.actor_id == c.creditor_id),
                present.iter().find(|a| a.actor_id == c.debtor_id),
            ) {
                let ctok = cred.public_actor_token();
                let dtok = debtor.public_actor_token();
                let urgent_word = if c.status == "defaulted" {
                    "都已經過了約定的還款日"
                } else {
                    "再過沒幾天就要到期"
                };
                duo_scene = Some(json!({
                    "id": format!("duo:debt:{}:{}", ctok, dtok),
                    "kind": "npc_duo_tension",
                    "speaker": format!("{} × {}", cred.display_name, debtor.display_name),
                    "text": format!(
                        "不遠處，{}攔住{}壓低嗓音爭執：「那筆 {} 枚銅幣的帳{}了，今兒總得給句準話！」{}漲紅著臉支吾：「這幾日海況差、手頭實在緊，寬限我兩日行不行？」",
                        cred.display_name, debtor.display_name, c.amount_copper, urgent_word, debtor.display_name
                    ),
                    "options": [
                        { "label": format!("上前替{}說句緩頰話", debtor.display_name), "act": "intervene_vignette", "param": format!("mediate|{}|{}", ctok, dtok) },
                        { "label": format!("幫{}講句公道話", cred.display_name), "act": "intervene_vignette", "param": format!("side_with|{}|{}", ctok, dtok) }
                    ]
                }));
                break;
            }
        }

        // 2b. 同場有真實嫌隙對峙（warmth <= -22）
        if duo_scene.is_none() {
            'pair_search: for (idx, a) in present.iter().enumerate() {
                for b in present.iter().skip(idx + 1) {
                    let w_ab = a.mind.ltm.social.get(&b.actor_id).map_or(0, |s| s.warmth);
                    let w_ba = b.mind.ltm.social.get(&a.actor_id).map_or(0, |s| s.warmth);
                    if w_ab <= -22 || w_ba <= -22 {
                        let atok = a.public_actor_token();
                        let btok = b.public_actor_token();
                        duo_scene = Some(json!({
                            "id": format!("duo:rift:{}:{}", atok, btok),
                            "kind": "npc_duo_tension",
                            "speaker": format!("{} × {}", a.display_name, b.display_name),
                            "text": format!(
                                "場中氣氛忽地一緊——{}冷冷掃過{}：「上次那樁事我可還記在帳上，少在這兒裝沒事人！」{}鼻間輕哼一聲：「話別說得太難聽，街坊四鄰可都在這兒看著呢。」",
                                a.display_name, b.display_name, b.display_name
                            ),
                            "options": [
                                { "label": "上前打圓場勸開兩人", "act": "intervene_vignette", "param": format!("mediate|{}|{}", atok, btok) },
                                { "label": "在旁冷眼記下兩人結怨", "act": "intervene_vignette", "param": format!("observe_duo|{}|{}", atok, btok) }
                            ]
                        }));
                        break 'pair_search;
                    }
                }
            }
        }

        // 2c. 同場街坊針對真實世界現況（斷橋／冷爐／潮汐／市集行情）交談
        if duo_scene.is_none() {
            let a = present[0];
            let b = present[1];
            let atok = a.public_actor_token();
            let btok = b.public_actor_token();
            let bridge_broken = delta
                .live_land
                .cells
                .values()
                .find(|cl| cl.has_river_crossing && cl.bridge_integrity < 70);
            let cold_smithy = doc
                .nodes
                .values()
                .find(|nd| {
                    nd.cell_id == node.cell_id
                        && nd.kind == "shop"
                        && infer_shop_kind(nd) == "smithy"
                        && super::live_commodities::is_smithy_forge_cold(delta, &nd.id)
                });
            let prior_real_chat = a
                .mind
                .stm
                .recent_talks
                .iter()
                .rev()
                .find(|t| t.partner_id == b.actor_id);
            let banter = if let Some(sm_nd) = cold_smithy {
                format!(
                    "{}正跟身旁的{}低聲議論：「聽說{}的鍛爐缺精煤熄火了，這會兒誰背炭過去肯定能賣個好價錢。」{}點點頭：「北林那頭炭脈多，就是得留神野獸跟灰耳斥候。」",
                    a.display_name, b.display_name, short_node_label(&sm_nd.name), b.display_name
                )
            } else if bridge_broken.is_some() {
                format!(
                    "{}壓低聲音對{}說：「這季雨水急，官道河橋有處被沖鬆了，商隊貨車都得慢下來等木料修繕。」{}嘆口氣：「貨一慢，酒館乾糧跟鐵料八成又要漲價。」",
                    a.display_name, b.display_name, b.display_name
                )
            } else if let Some(rt) = prior_real_chat {
                format!(
                    "{}正接著方才與{}的話頭低聲聊著：「{}」{}點頭應道：「{}」",
                    a.display_name, b.display_name, rt.self_replied, b.display_name, rt.partner_said
                )
            } else {
                let topic_anchor = a
                    .mind
                    .ltm
                    .episodic
                    .iter()
                    .rev()
                    .map(|e| e.subjective_zh.trim_end_matches('。').to_string())
                    .find(|s| {
                        s.chars().count() >= 8
                            && s.chars().count() <= 46
                            && !s.contains('〔')
                            && !s.contains("你只勉強")
                            && !s.contains("（暗記")
                            && !s.contains("（牢記")
                    })
                    .unwrap_or_else(|| {
                        format!(
                            "今天這{}裡往來貨船與行商比前兩日多出不少",
                            short_node_label(&node.name)
                        )
                    });
                format!(
                    "{}靠在路旁跟{}閒聊：「說起近來鎮上事，{}。」{}順口接話：「可不是嘛，大夥兒各自顧緊手頭營生才是正經。」",
                    a.display_name, b.display_name, topic_anchor, b.display_name
                )
            };
            duo_scene = Some(json!({
                "id": format!("duo:banter:{}:{}", atok, btok),
                "kind": "npc_duo_banter",
                "speaker": format!("{} ＆ {}", a.display_name, b.display_name),
                "text": banter,
                "options": [
                    { "label": format!("順話頭向{}插嘴打聽", a.display_name), "act": "open_talk", "param": atok },
                    { "label": "記下這段街坊口風", "act": "intervene_vignette", "param": format!("observe_duo|{}|{}", atok, btok) }
                ]
            }));
        }

        if let Some(ds) = duo_scene {
            scenes.push(ds);
        }
    }

    scenes
}

/// 玩家當場插話／回應現場群戲的真實結算（改寫雙方好感、結識、長期記憶與全場見證）。
pub fn apply_vignette_intervene(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    param: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let parts: Vec<&str> = param.split('|').collect();
    let mode = parts.first().copied().unwrap_or("");
    let tok1 = parts.get(1).copied().unwrap_or("");
    let tok2 = parts.get(2).copied().unwrap_or("");

    let (node_id, p_name) = {
        let p = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
        (p.node_id.clone(), p.display_name.clone())
    };
    let t1_id = delta
        .resolve_public_target(tok1)
        .ok_or("對方已經轉過街角走遠了。")?;
    let t1_name = {
        let t1 = delta.actors.get(&t1_id).ok_or("對方已不在場")?;
        if !t1.alive || t1.node_id != node_id {
            return Err("對方此刻已不在你眼前。".to_string());
        }
        t1.display_name.clone()
    };

    match mode {
        "greet_back" => {
            if let Some(t1) = delta.actors.get_mut(&t1_id) {
                t1.mind.record_talk_exchange(
                    &aid,
                    &p_name,
                    clock.game_days,
                    &clock.bell_label,
                    "禮貌點頭報上名號",
                    "頷首致意記下這個照面",
                    false,
                );
                let soc = t1.mind.ltm.social.entry(aid.clone()).or_default();
                soc.acquainted = true;
                soc.familiarity = soc.familiarity.saturating_add(8).min(100);
                soc.warmth = (soc.warmth + 6).clamp(-100, 100);
                t1.mark_mutated();
            }
            if let Some(p) = delta.actors.get_mut(&aid) {
                let soc = p.mind.ltm.social.entry(t1_id.clone()).or_default();
                soc.acquainted = true;
                soc.familiarity = soc.familiarity.saturating_add(8).min(100);
                soc.warmth = (soc.warmth + 6).clamp(-100, 100);
                let msg = format!("你朝{}拱手點頭報上名號，對方笑著記下了你的面孔。", t1_name);
                push_breath(p, msg.clone());
                p.mark_mutated();
                return Ok(msg);
            }
            Ok(format!("你向{}點頭致意。", t1_name))
        }
        "show_rubbing" => {
            let has_rub = delta.actors.get(&aid).is_some_and(|p| {
                p.backpack
                    .iter()
                    .any(|i| i.slug == "stele_rubbing" && i.count > 0)
            });
            if !has_rub {
                return Err("你背囊裡目前沒有矮人古碑拓片。".to_string());
            }
            let mut extra_lang = String::new();
            if let Some(p) = delta.actors.get_mut(&aid) {
                p.known_rumors.insert("pit_rumor".to_string());
                if let Some(ln) = add_language_exposure(p, clock.game_days, "dwarven", 18) {
                    extra_lang = ln;
                }
                let soc = p.mind.ltm.social.entry(t1_id.clone()).or_default();
                soc.acquainted = true;
                soc.warmth = (soc.warmth + 10).clamp(-100, 100);
                p.mark_mutated();
            }
            if let Some(t1) = delta.actors.get_mut(&t1_id) {
                let soc = t1.mind.ltm.social.entry(aid.clone()).or_default();
                soc.acquainted = true;
                soc.warmth = (soc.warmth + 10).clamp(-100, 100);
                t1.mark_mutated();
            }
            let msg = format!(
                "{}湊近細看你遞上的拓片，壓低聲音點破：「這是北林斷橋下舊試煉坑的機關警語，左旋楔石三寸方能進前廳，深處盤踞著吞石巨蛛！」{}",
                t1_name, extra_lang
            );
            if let Some(p) = delta.actors.get_mut(&aid) {
                push_breath(p, msg.clone());
            }
            Ok(msg)
        }
        "mediate" | "side_with" | "observe_duo" => {
            let t2_id = delta
                .resolve_public_target(tok2)
                .ok_or("另一位街坊已經走開了。")?;
            let t2_name = {
                let t2 = delta.actors.get(&t2_id).ok_or("另一位街坊已不在場")?;
                if !t2.alive || t2.node_id != node_id {
                    return Err("另一位街坊此刻已不在場。".to_string());
                }
                t2.display_name.clone()
            };
            if mode == "observe_duo" {
                let msg = format!(
                    "你不動聲色站在一旁，把{}與{}的對話口風與眉目情勢暗自記在心裡。",
                    t1_name, t2_name
                );
                if let Some(p) = delta.actors.get_mut(&aid) {
                    p.mind.ingest_historical_episode(
                        &format!("obs:{}:{}:{}", clock.game_days, t1_id, t2_id),
                        clock.game_days,
                        &node_id,
                        &t1_id,
                        &t1_name,
                        EvidenceTier::DirectWitness,
                        95,
                        92,
                        22,
                        1,
                        format!("在場聽見{}與{}的交談與恩怨口風", t1_name, t2_name),
                    );
                    push_breath(p, msg.clone());
                    p.mark_mutated();
                }
                return Ok(msg);
            }

            let (dw1, dw2, summary) = if mode == "mediate" {
                (
                    8i8,
                    12i8,
                    format!(
                        "{}上前替{}與{}居中說和，兩人臉色稍緩，暫且壓下當街爭執",
                        p_name, t1_name, t2_name
                    ),
                )
            } else {
                (
                    12i8,
                    -10i8,
                    format!(
                        "{}當場站到{}這邊替其幫腔，{}面色難看地記住了這筆人情帳",
                        p_name, t1_name, t2_name
                    ),
                )
            };

            if let Some(t1) = delta.actors.get_mut(&t1_id) {
                let s = t1.mind.ltm.social.entry(aid.clone()).or_default();
                s.acquainted = true;
                s.familiarity = s.familiarity.saturating_add(12).min(100);
                s.warmth = (s.warmth + dw1).clamp(-100, 100);
                if mode == "mediate" {
                    let sb = t1.mind.ltm.social.entry(t2_id.clone()).or_default();
                    sb.warmth = (sb.warmth + 6).clamp(-100, 100);
                }
                t1.mind.stm.affect.anger = t1.mind.stm.affect.anger.saturating_sub(14);
                t1.mark_mutated();
            }
            if let Some(t2) = delta.actors.get_mut(&t2_id) {
                let s = t2.mind.ltm.social.entry(aid.clone()).or_default();
                s.acquainted = true;
                s.familiarity = s.familiarity.saturating_add(12).min(100);
                s.warmth = (s.warmth + dw2).clamp(-100, 100);
                if mode == "mediate" {
                    let sa = t2.mind.ltm.social.entry(t1_id.clone()).or_default();
                    sa.warmth = (sa.warmth + 6).clamp(-100, 100);
                }
                t2.mark_mutated();
            }
            if let Some(p) = delta.actors.get_mut(&aid) {
                for (tid, dw) in [(&t1_id, dw1), (&t2_id, dw2)] {
                    let s = p.mind.ltm.social.entry(tid.clone()).or_default();
                    s.acquainted = true;
                    s.familiarity = s.familiarity.saturating_add(12).min(100);
                    s.warmth = (s.warmth + dw).clamp(-100, 100);
                }
                push_breath(p, summary.clone());
                p.mark_mutated();
            }
            broadcast_scene_event(
                doc,
                delta,
                clock,
                &node_id,
                &aid,
                &p_name,
                EventPublicity::PublicLoud,
                Some("harbor_trade"),
                36,
                16,
                ValueAxis::PactIntegrity,
                "scene_intervene",
                &summary,
            );
            Ok(summary)
        }
        _ => Err("尚未聽清你想如何介入現場。".to_string()),
    }
}

/// 人物實體碰觸推擠與屢次碰撞引發口角／衝突（`DESIGN_KEYWORDS.md` L132：實體不可穿透，輕則推擠、重則撞開，屢次觸發衝突）
pub fn apply_physical_crowd_bump(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    mover_id: &str,
    dst_node: &SpatialNodeDef,
    rushed_step: bool,
) -> Option<String> {
    // 僅在狹窄或人潮密集之街道／棧橋／店鋪／市集／拱橋觸發實體擦撞（寬闊廣場、荒野與私室不誤觸）
    if !matches!(
        dst_node.kind.as_str(),
        "street" | "alley" | "dock" | "shop" | "bridge" | "market" | "civic"
    ) {
        return None;
    }
    let (is_player, mover_name, carry_weight, already_fighting) = {
        let m = delta.actors.get(mover_id)?;
        let w: u32 = m
            .backpack
            .iter()
            .map(|i| u32::from(i.weight_units) * i.count)
            .sum();
        (
            m.controller_id.is_some(),
            m.display_name.clone(),
            w,
            m.active_skirmish.is_some(),
        )
    };
    if already_fighting {
        return None;
    }
    let others: Vec<String> = delta
        .node_occupants
        .get(&dst_node.id)
        .map(|occ| {
            occ.iter()
                .filter(|&oid| {
                    oid != mover_id
                        && delta.actors.get(oid).is_some_and(|o| {
                            o.alive && o.active_skirmish.is_none() && o.active_transit.is_none()
                        })
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let crowd_n = others.len();
    // 擁擠門檻：真人急行或負重穿行 >= 2 人即可能擦撞；一般步履須 >= 4 人；背景 NPC 須 >= 6 人避免和平城鎮無故群毆
    let triggers = if is_player {
        (rushed_step && crowd_n >= 2) || (carry_weight >= 8 && crowd_n >= 3) || crowd_n >= 4
    } else {
        rushed_step && crowd_n >= 6
    };
    if !triggers {
        return None;
    }
    let pick_idx = ((clock.real_unix.unsigned_abs() as usize)
        .wrapping_add(mover_id.len() * 7)
        .wrapping_add(crowd_n))
        % crowd_n;
    let target_id = others[pick_idx].clone();
    let (target_name, prior_warmth, target_is_npc_staff) = {
        let t = delta.actors.get(&target_id)?;
        let w = t
            .mind
            .ltm
            .social
            .get(mover_id)
            .map_or(0, |s| s.warmth);
        let staff = t.controller_id.is_none()
            && (t.role.contains("掌櫃") || t.role.contains("學徒") || t.role.contains("草藥師"));
        (t.display_name.clone(), w, staff)
    };

    let key1 = format!("bump:d{}:{target_id}:1", clock.game_days);
    let key2 = format!("bump:d{}:{target_id}:2", clock.game_days);
    let prior_bumps: u8 = delta.actors.get(mover_id).map_or(0, |m| {
        if m.explored_windows.contains(&key2) {
            2
        } else if m.explored_windows.contains(&key1) {
            1
        } else {
            0
        }
    });

    if let Some(m) = delta.actors.get_mut(mover_id) {
        if prior_bumps == 0 {
            m.explored_windows.insert(key1);
        } else {
            m.explored_windows.insert(key2);
        }
        let s = m.mind.ltm.social.entry(target_id.clone()).or_default();
        s.warmth = (s.warmth - if prior_bumps == 0 { 4 } else { 10 }).clamp(-100, 100);
        m.mark_mutated();
    }
    let new_target_warmth = if let Some(t) = delta.actors.get_mut(&target_id) {
        let updated_warmth = {
            let s = t.mind.ltm.social.entry(mover_id.to_string()).or_default();
            s.warmth = (s.warmth - if prior_bumps == 0 { 4 } else { 10 }).clamp(-100, 100);
            s.warmth
        };
        t.mind.stm.affect.anger = t
            .mind
            .stm
            .affect
            .anger
            .saturating_add(if prior_bumps == 0 { 6 } else { 16 })
            .min(100);
        t.mark_mutated();
        updated_warmth
    } else {
        prior_warmth - 4
    };

    if rushed_step && prior_bumps >= 2 && is_player && !target_is_npc_staff && new_target_warmth <= -18 {
        if let Some(m) = delta.actors.get_mut(mover_id) {
            m.active_skirmish = Some(super::types::SkirmishState {
                foe_slug: target_id.clone(),
                foe_name: target_name.clone(),
                can_speak_goblin: false,
                foe_stagger: 0,
                player_guard: false,
                round_idx: 1,
            });
            m.mark_mutated();
        }
        broadcast_scene_event(
            doc,
            delta,
            clock,
            &dst_node.id,
            mover_id,
            &mover_name,
            EventPublicity::PublicLoud,
            Some("harbor_trade"),
            55,
            28,
            ValueAxis::PactIntegrity,
            "crowd_scuffle",
            &format!("{mover_name}屢次硬撞{target_name}，雙方當街推搡動了真火！"),
        );
        return Some(format!(
            " 你再次迎面撞開【{target_name}】，對方忍無可忍反手推胸喝罵，街頭眾目睽睽下雙方當場進入短兵對峙！"
        ));
    }

    if prior_bumps >= 1 || prior_warmth <= -20 {
        broadcast_scene_event(
            doc,
            delta,
            clock,
            &dst_node.id,
            mover_id,
            &mover_name,
            EventPublicity::PublicLoud,
            Some("harbor_trade"),
            36,
            16,
            ValueAxis::PactIntegrity,
            "crowd_shove",
            &format!("{mover_name}在擁擠通道裡重重撞開了{target_name}，惹來一陣怒瞪呵斥"),
        );
        return Some(format!(
            " 巷道人潮擁擠，你步伐太急重重撞開了【{target_name}】，對方踉蹌半步回頭冷聲呵斥：「走路長點眼睛！」"
        ));
    }

    Some(format!(
        " 通道往來人密，你肩頭不小心擦撞到一旁的【{target_name}】，彼此側身讓過時對方面露微慍。"
    ))
}
