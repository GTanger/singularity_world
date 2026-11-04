//! WebSocket `do_action` 主幹：分派到同房實體（talk/look/combat/borrow/trade）
//! 或房間物件（look/use/move）的動作。實際處理見各子模組。

mod borrow;
pub(crate) mod combat;
mod look;
mod object;
mod talk;
pub(crate) mod trade;

use crate::config::Server;
use crate::db;
use crate::entity::{self, Character, EntityKind, Verb};
use crate::game;
use crate::gametext;

use super::handler::{WsConnection, player_id, push_refresh};
use super::protocol::{ActionResultMsg, ClientMsg};

/// 與目標是否在同一可互動位置（place_id 優先，舊資料才使用相容鏡像）。
fn entities_share_playable_cell(player: &Character, target: &Character, player_room: &str) -> bool {
    if let (Some(px), Some(py), Some(tx), Some(ty)) =
        (player.grid_x, player.grid_y, target.grid_x, target.grid_y)
        && (px - tx).abs() + (py - ty).abs() <= 1
    {
        return true;
    }
    if let (Some(px), Some(py)) = (player.grid_x, player.grid_y) {
        if let Some(state) = crate::store::LIVE_NPC_GRID_STATES.read().get(&target.id)
            && (px - state.current.0).abs() + (py - state.current.1).abs() <= 1
        {
            return true;
        }
        if let Some(store) = crate::store::get_store()
            && store
                .read()
                .entity_ids_at_grid(px, py)
                .iter()
                .any(|id| id == &target.id)
        {
            return true;
        }
    }
    let player_place = db::get_entity_place_id(&player.id);
    let target_place = db::get_entity_place_id(&target.id);
    if player_place.is_some() && target_place.is_some() && player_place == target_place {
        return true;
    }
    let Ok(tr) = db::get_entity_room(&target.id) else {
        return false;
    };
    !player_room.is_empty() && db::location_keys_equivalent(player_room, &tr)
}

/// 分派 `do_action`：先試同房實體，再試房間物件。
pub fn handle_do_action(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let target_id = msg.entity_id.clone();
    let mut action = normalize_action_key(&msg.action).to_string();
    if action == "Attack" {
        action = "Slay".into();
    }
    if action == "Work" {
        do_player_work(conn, &pid, &target_id);
        return;
    }
    if target_id.is_empty() || action.is_empty() {
        conn.send_error(gametext::client("missing_target_or_action"));
        return;
    }
    if target_id == pid {
        conn.send_error(gametext::client("cannot_self_action"));
        return;
    }
    let Ok(Some(player_ch)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("get_self_failed"));
        return;
    };
    let player_room = match db::get_entity_room(&pid) {
        Ok(room) if !room.is_empty() => room,
        _ => match grid_room_key(&player_ch) {
            Some(room) => room,
            None => {
                conn.send_error(gametext::client("no_current_room"));
                return;
            }
        },
    };

    if try_entity_branch(
        conn,
        msg,
        &pid,
        &player_ch,
        &player_room,
        &target_id,
        &action,
    ) {
        return;
    }
    object::do_object_branch(conn, &pid, &player_room, &target_id, &action);
}

fn do_player_work(conn: &WsConnection, pid: &str, target_id: &str) {
    let now = game::now_unix();
    let Some(pool) = crate::pg::pool() else {
        conn.send_error("世界工作主資料暫時不可用");
        return;
    };
    match crate::pg::world_work::perform_shift(&pool, pid, now, "player_work") {
        Ok(Some(result)) => {
            if result.work_id.is_none() {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Work".into(),
                    target_id: if target_id.is_empty() {
                        result.venue_id.clone()
                    } else {
                        target_id.to_string()
                    },
                    target_name: result.occupation_id.clone(),
                    narrative: "工坊目前暫時無法結算本班次工錢，請稍後再試。".into(),
                    success: false,
                    actions: vec!["Look".into()],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            if let Some(store) = crate::store::get_store()
                && let Err(e) = store.write().mirror_economy_from_authority(
                    pid,
                    result.entity_magnesium,
                    &result.inventory,
                    &result.current_activity,
                )
            {
                tracing::warn!("player work cache mirror: {e}");
            }
            let room_name = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|ch| target_grid_room_name(&ch))
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    db::get_entity_room(pid)
                        .ok()
                        .and_then(|rid| db::get_room_name(&rid).ok())
                })
                .unwrap_or_default();
            let narrative = format_player_work_shift_narrative_and_advance_pack(
                &room_name,
                &result.occupation_id,
                result.wage_mg,
                result
                    .supply
                    .as_ref()
                    .map(|s| (s.quantity, s.display_name.as_str())),
            );
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Work".into(),
                target_id: if target_id.is_empty() {
                    result.venue_id.clone()
                } else {
                    target_id.to_string()
                },
                target_name: result.occupation_id,
                narrative,
                success: true,
                actions: vec!["Work".into(), "Look".into()],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
        }
        Ok(None) => {
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Work".into(),
                target_id: target_id.to_string(),
                target_name: "工作差事".into(),
                narrative:
                    "此處暫無你可以參與的輪值工序，或是本班次的定額勞作已經結清；請稍候下一班次或確認派工紀錄。"
                        .into(),
                success: false,
                actions: vec!["Look".into()],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
        }
        Err(e) => {
            tracing::warn!("player work perform_shift failed: {e}");
            conn.send_error(format!("執行工作時發生錯誤：{e}"));
        }
    }
}

fn try_entity_branch(
    conn: &WsConnection,
    msg: &ClientMsg,
    pid: &str,
    player: &Character,
    player_room: &str,
    target_id: &str,
    action: &str,
) -> bool {
    let is_entity_id_prefix = target_id.starts_with("ambient_")
        || target_id.starts_with("shanhai_")
        || target_id.starts_with("wt_npc_")
        || target_id.starts_with("wt_vendor_")
        || target_id.starts_with("wt_citizen_")
        || target_id.starts_with("npc_");
    let Ok(target_opt) = db::get_entity(target_id) else {
        return false;
    };
    let Some(target) = target_opt else {
        if is_entity_id_prefix {
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.to_string(),
                target_id: target_id.to_string(),
                target_name: "陌生人".into(),
                narrative: "你欲上前，對方卻已邁步離開此處，朝鄰街走遠了。".into(),
                success: false,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
            return true;
        }
        return false;
    };
    if !entities_share_playable_cell(player, &target, player_room) {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.to_string(),
            target_id: target_id.to_string(),
            target_name: player_visible_target_name(pid, &target),
            narrative: "你欲上前，對方卻已邁步離開此處，朝鄰街走遠了。".into(),
            success: false,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        push_refresh(conn, pid);
        return true;
    }
    crate::store::pause_npc_for_interaction(target_id, 45);
    if !entity_socket_ok(conn, pid, &target, target_id, player_room, action) {
        return true;
    }
    let now = game::now_unix();
    match action {
        "Look" => look::do_entity_look(conn, pid, now, target_id, &target),
        "Talk" => talk::do_entity_talk(conn, msg, pid, player_room, target_id, &target, now),
        "Subdue" | "Slay" => {
            combat::do_entity_combat(conn, pid, player_room, target_id, action, &target, now);
        }
        "Borrow" => borrow::do_entity_borrow(conn, pid, target_id, &target, now),
        "Trade" => {
            let _ = trade::do_entity_trade(conn, msg, pid, target_id, &target, now);
        }
        _ => conn.send_error(client_err_fmt("unknown_action_fmt", action)),
    }
    true
}

fn grid_room_key(ch: &Character) -> Option<String> {
    let (Some(x), Some(y)) = (ch.grid_x, ch.grid_y) else {
        return None;
    };
    Some(format!("grid:{x}:{y}"))
}

pub fn action_label_zh(action: &str) -> &str {
    match action {
        "Gather" | "Harvest" | "Take" => "採集",
        "Look" => "觀察",
        "Talk" => "交談",
        "Trade" => "交易",
        "Borrow" => "商借",
        "Subdue" => "制伏",
        "Slay" | "Attack" => "擊殺",
        "Use" => "使用",
        "Operate" => "操作",
        "Read" => "閱讀",
        "Rest" => "歇息",
        "Sleep" => "入眠",
        "Bath" => "沐浴",
        "Wash" => "洗漱淨手",
        "Drink" => "飲用",
        "Taste" => "品嚐",
        "Smell" => "聞香",
        "WarmHands" => "烘手",
        "RoastFood" => "炙烤",
        "MakeFire" => "生火",
        "AddFuel" => "添柴",
        "SplitWood" => "劈柴",
        "RefinePlank" => "裁鋸木板",
        "BurnCharcoal" => "悶燒精炭",
        "CraftTorch" => "紮製火把",
        "RepairStructure" => "修繕加固",
        "CollectCharcoal" => "取炭",
        "Work" | "差事" => "差事",
        "Move" => "移動",
        "Check" => "檢視",
        "Climb" => "登臨",
        "Inspect" => "審視",
        "Knock" => "叩門",
        "Listen" => "諦聽",
        "LookShip" => "眺望舟楫",
        "Tap" => "輕叩",
        "TestEdge" => "試刃",
        "Tie" => "繫縛",
        "Touch" => "撫觸",
        "Turn" => "旋動",
        "Watch" => "守望",
        "WeaveSackcloth" => "織造麻袋布",
        "Meditate" => "調息",
        "Water" => "澆灌",
        "Calculate" => "推算",
        "Sit" => "坐下",
        "Ring" => "叩鐘",
        "Weigh" => "秤量",
        "Caulking" => "捻縫",
        "Open" => "開啟",
        "Pound" => "舂搗",
        "PickType" => "揀字",
        "RollInk" => "滾墨",
        "Stitch" => "縫紉",
        "Measure" => "丈量",
        "Observe" => "觀測",
        "TestBrush" => "試筆",
        "GrindInk" => "研墨",
        "Quench" => "淬火",
        "Feed" => "餵食",
        "Shake" => "搖動",
        "Sharpen" => "磨利",
        "DryOff" => "避風晾衣",
        "Acquire" => "採買入袋",
        "Deliver" => "交割物資",
        "ClaimStrandedCargo" => "揭榜管運",
        "DeliverStrandedCargo" => "交割銷單",
        "Sow" => "整壟播種",
        "Board" => "登船啟航",
        "Buy" => "投保立契",
        "DepositBank" => "存入票號",
        "WithdrawBank" => "憑摺支取",
        "StoreStash" => "存入石櫃",
        "RetrieveStash" => "取出物資",
        "DispatchCourier" => "快騎託運",
        "AngleFish" => "甩竿垂釣",
        "Cook" => "野炊烹煮",
        "MillGrain" => "水磨碾米",
        "PressCheese" => "壓製乳酪",
        "SmokeFish" => "慢火冷燻",
        "CutJade" => "切磨寒玉",
        "HaulMineCart" => "滑車運礦",
        "BurnLime" => "入窯煅灰",
        "RowSwampRaft" => "撐筏入澤",
        "PrepareMiasmaGear" => "備妥避瘴",
        "PlayChess" => "對弈手談",
        "SeasonalBanquet" => "歲時圍爐",
        "ChartAstrolabe" => "拓印星盤",
        "TestEngraving" => "測紋校準",
        "SparPillar" => "試勁拆招",
        "SoakHotSpring" => "祛寒湯浴",
        "GroomAppearance" => "修面整容",
        "MendTravelGear" => "補綴行裝",
        "ReadWindVane" => "觀測風候",
        "ApplyMoxibustion" => "百草溫灸",
        "HangVotivePlaque" => "懸掛銘牌",
        "TranscribeRareTome" | "鈔錄孤本" => "鈔錄孤本",
        "AttendLecture" | "旁聽策論" => "旁聽策論",
        "CalibrateClock" | "校時報刻" => "校時報刻",
        "FitMortiseTenon" | "榫卯校修" => "榫卯校修",
        "QuenchBlade" | "精鋼回火" => "精鋼回火",
        "DyeIndigoCloth" | "藍染浸布" => "藍染浸布",
        "FeedDraftBeast" | "飼餵草獸" | "飼喂草獸" => "飼餵草獸",
        "BoardCoastalShip" | "登船遠航" => "登船遠航",
        "InspectLandRegistry" | "查閱地籍" => "查閱地籍",
        "AssistKitchen" | "幫廚備膳" => "幫廚備膳",
        "RoastMountainTea" | "焙茶品茗" => "焙茶品茗",
        "SunBooks" | "曝書防蠹" => "曝書防蠹",
        "RoastHerbs" | "炮製藥材" => "炮製藥材",
        "CollateInscriptions" | "校勘碑帖" => "校勘碑帖",
        "GrindOpticLens" | "研磨鏡片" => "研磨鏡片",
        "SelectDriedSeafood" | "揀選海味" => "揀選海味",
        "DeliberateCivicPact" | "參議鄉約" => "參議鄉約",
        "DrawSpringWell" | "汲水潤嗓" => "汲水潤嗓",
        "SmeltBloom" | "悶燒成塊" => "悶燒成塊",
        "BuildFieldAnvil" | "架設石砧" => "架設石砧",
        "SetupHerbStove" | "架設陶銚" => "架設陶銚",
        "WashOre" | "淘洗礦砂" => "淘洗礦砂",
        "ForgeRepairStrip" | "鍛修鐵條" => "鍛修鐵條",
        "SharpenBlade" | "砥石磨刃" => "砥石磨刃",
        "WashHerb" | "淘洗藥草" => "淘洗藥草",
        "DryHerb" | "DryHerbs" | "曝曬藥材" => "曝曬藥材",
        "CrushHerb" | "研碎藥草" => "研碎藥草",
        "BoilDecoction" | "DecoctPotion" | "煎煮湯劑" => "煎煮湯劑",
        "Queue" | "候號" => "候號",
        "AcceptJob" | "應工" => "應工",
        "BuySoup" | "買滿碗熱湯" => "買滿碗熱湯",
        "CarryCargo" | "搬貨收鎂" => "搬貨收鎂",
        "HelpRope" | "搭手拉繩" => "搭手拉繩",
        "TalkQueue" | "攀談船期" => "攀談船期",
        "QueueFerry" | "排隊候船" => "排隊候船",
        "SellReeds" | "賣蘆草" => "賣蘆草",
        "SellSackcloth" | "交麻袋布" => "交麻袋布",
        "SellRope" | "交麻繩卷" => "交麻繩卷",
        "BoardFerry" | "憑隊上船" => "憑隊上船",
        "StepAshore" | "踩泥上岸" => "踩泥上岸",
        "CheckCargo" | "對貨領尾款" => "對貨領尾款",
        "JoinCheck" | "候檢" => "候檢",
        "RegisterComplaint" | "押名" => "押名",
        "QueueBroth" | "等分湯" => "等分湯",
        "Explore" | "深入探索" => "深入探索",
        "CrushOre" | "粉碎礦石" => "粉碎礦石",
        "SmeltIngot" | "熔煉金屬" => "熔煉金屬",
        "ForgeTool" | "鍛打器具" => "鍛打器具",
        "CastHardware" | "澆鑄五金" => "澆鑄五金",
        "SortHerbs" | "分揀藥草" => "分揀藥草",
        "GrindPowder" | "研製藥粉" => "研製藥粉",
        "MakePoultice" | "調製敷料" => "調製敷料",
        "DistillEssence" | "萃煉純露" => "萃煉純露",
        "SoakFiber" | "漚浸纖維" => "漚浸纖維",
        "TwistCord" | "TwistRope" | "搓捻麻繩" => "搓捻麻繩",
        "WeaveCloth" | "紡織布匹" => "紡織布匹",
        "OilCanvas" | "桐油浸布" => "桐油浸布",
        "PressPaper" | "抄造紙張" => "抄造紙張",
        "ScrapeHide" | "刮治生皮" => "刮治生皮",
        "BoilFishGlue" | "熬製魚膠" => "熬製魚膠",
        "CalcifyShell" | "煅燒貝灰" => "煅燒貝灰",
        "CobbleBoots" | "縫製行靴" => "縫製行靴",
        "CraftHandle" | "車削木柄" => "車削木柄",
        "Chop" | "砍伐" => "砍伐",
        _ => action,
    }
}

/// 將前端或指令傳入之動作名標準化為 Canonical 英文鍵名
pub fn normalize_action_key(action: &str) -> &str {
    match action {
        "TranscribeRareTome" | "鈔錄孤本" => "TranscribeRareTome",
        "AttendLecture" | "旁聽策論" => "AttendLecture",
        "CalibrateClock" | "校時報刻" => "CalibrateClock",
        "FitMortiseTenon" | "榫卯校修" => "FitMortiseTenon",
        "QuenchBlade" | "精鋼回火" => "QuenchBlade",
        "DyeIndigoCloth" | "藍染浸布" => "DyeIndigoCloth",
        "FeedDraftBeast" | "飼餵草獸" | "飼喂草獸" => "FeedDraftBeast",
        "BoardCoastalShip" | "登船遠航" => "BoardCoastalShip",
        "InspectLandRegistry" | "查閱地籍" => "InspectLandRegistry",
        "AssistKitchen" | "幫廚備膳" => "AssistKitchen",
        "RoastMountainTea" | "焙茶品茗" => "RoastMountainTea",
        "SunBooks" | "曝書防蠹" => "SunBooks",
        "RoastHerbs" | "炮製藥材" => "RoastHerbs",
        "CollateInscriptions" | "校勘碑帖" => "CollateInscriptions",
        "GrindOpticLens" | "研磨鏡片" => "GrindOpticLens",
        "SelectDriedSeafood" | "揀選海味" => "SelectDriedSeafood",
        "DeliberateCivicPact" | "參議鄉約" => "DeliberateCivicPact",
        "DrawSpringWell" | "汲水潤嗓" => "DrawSpringWell",
        "SmeltBloom" | "悶燒成塊" => "SmeltBloom",
        "BuildFieldAnvil" | "架設石砧" => "BuildFieldAnvil",
        "SetupHerbStove" | "架設陶銚" => "SetupHerbStove",
        "WashOre" | "淘洗礦砂" => "WashOre",
        "ForgeRepairStrip" | "鍛修鐵條" => "ForgeRepairStrip",
        "SharpenBlade" | "砥石磨刃" => "SharpenBlade",
        "WashHerb" | "淘洗藥草" => "WashHerb",
        "DryHerb" | "DryHerbs" | "曝曬藥材" => "DryHerb",
        "CrushHerb" | "研碎藥草" => "CrushHerb",
        "BoilDecoction" | "DecoctPotion" | "煎煮湯劑" => "BoilDecoction",
        "Queue" | "候號" => "Queue",
        "AcceptJob" | "應工" => "AcceptJob",
        "BuySoup" | "買滿碗熱湯" => "BuySoup",
        "CarryCargo" | "搬貨收鎂" => "CarryCargo",
        "HelpRope" | "搭手拉繩" => "HelpRope",
        "TalkQueue" | "攀談船期" => "TalkQueue",
        "QueueFerry" | "排隊候船" => "QueueFerry",
        "SellReeds" | "賣蘆草" => "SellReeds",
        "SellSackcloth" | "交麻袋布" => "SellSackcloth",
        "SellRope" | "交麻繩卷" => "SellRope",
        "BoardFerry" | "憑隊上船" => "BoardFerry",
        "StepAshore" | "踩泥上岸" => "StepAshore",
        "CheckCargo" | "對貨領尾款" => "CheckCargo",
        "JoinCheck" | "候檢" => "JoinCheck",
        "RegisterComplaint" | "押名" => "RegisterComplaint",
        "QueueBroth" | "等分湯" => "QueueBroth",
        "Explore" | "深入探索" => "Explore",
        "CrushOre" | "粉碎礦石" => "CrushOre",
        "SmeltIngot" | "熔煉金屬" => "SmeltIngot",
        "ForgeTool" | "鍛打器具" => "ForgeTool",
        "CastHardware" | "澆鑄五金" => "CastHardware",
        "SortHerbs" | "分揀藥草" => "SortHerbs",
        "GrindPowder" | "研製藥粉" => "GrindPowder",
        "MakePoultice" | "調製敷料" => "MakePoultice",
        "DistillEssence" | "萃煉純露" => "DistillEssence",
        "SoakFiber" | "漚浸纖維" => "SoakFiber",
        "TwistCord" | "TwistRope" | "搓捻麻繩" => "TwistRope",
        "WeaveCloth" | "紡織布匹" => "WeaveCloth",
        "OilCanvas" | "桐油浸布" => "OilCanvas",
        "PressPaper" | "抄造紙張" => "PressPaper",
        "ScrapeHide" | "刮治生皮" => "ScrapeHide",
        "BoilFishGlue" | "熬製魚膠" => "BoilFishGlue",
        "CalcifyShell" | "煅燒貝灰" => "CalcifyShell",
        "CobbleBoots" | "縫製行靴" => "CobbleBoots",
        "CraftHandle" | "車削木柄" => "CraftHandle",
        "Chop" | "砍伐" => "Chop",
        "Work" | "差事" => "Work",
        _ => action,
    }
}

pub(super) fn client_err_fmt(key: &str, arg: &str) -> String {
    let label = action_label_zh(arg);
    gametext::client(key).replacen("%s", label, 1)
}

fn entity_socket_ok(
    conn: &WsConnection,
    pid: &str,
    target: &Character,
    target_id: &str,
    player_room: &str,
    action: &str,
) -> bool {
    let room_name = resolve_action_room_name(pid, target);
    let has_live_stock = crate::db::get_inventory(&target.inventory, target.vit)
        .items
        .iter()
        .any(|i| i.qty > 0);
    if (trade::target_has_shop_offer(target, &room_name)
        || has_live_stock
        || target.kind == EntityKind::Npc)
        && matches!(action, "Look" | "Talk" | "Trade" | "Borrow" | "Subdue" | "Slay")
    {
        return true;
    }
    if target.kind == EntityKind::Npc {
        let sockets = db::get_sockets_for_npc(target_id, player_room);
        if !sockets.iter().any(|s| s == action) {
            conn.send_error(client_err_fmt("cannot_action_fmt", action));
            return false;
        }
        if !db::is_default_socket(action) {
            let in_venue = db::entity_in_venue_at_room(target_id, player_room).unwrap_or(false);
            if !in_venue {
                conn.send_error(client_err_fmt("target_not_at_workplace_fmt", action));
                return false;
            }
        }
        return true;
    }
    let Some(v) = verb_from_action(action) else {
        conn.send_error(client_err_fmt("unknown_action_fmt", action));
        return false;
    };
    if !entity::has_socket(&Character::sockets(), &v) {
        conn.send_error(client_err_fmt("cannot_action_fmt", action));
        return false;
    }
    true
}


pub(super) fn target_grid_room_name(target: &Character) -> String {
    if let (Some(x), Some(y)) = (target.grid_x, target.grid_y) {
        if let Some(name) = grid_room_name_at(x, y) {
            return name;
        }
    }
    String::new()
}

pub(super) fn resolve_action_room_name(player_id: &str, target: &Character) -> String {
    let t_room = target_grid_room_name(target);
    if !t_room.is_empty() {
        return t_room;
    }
    if let Ok(Some(player)) = db::get_entity(player_id) {
        if let (Some(px), Some(py)) = (player.grid_x, player.grid_y) {
            if let Some(name) = grid_room_name_at(px, py) {
                return name;
            }
        }
    }
    String::new()
}

fn grid_room_name_at(x: i32, y: i32) -> Option<String> {
    crate::server::grid_manager::get_runtime_grid_cell(crate::grid::SquareCoord::new(x, y)).map(|c| {
        if c.name.trim().is_empty() {
            crate::grid::terrain_name_zh(c.terrain).to_string()
        } else {
            c.name.clone()
        }
    })
}

pub fn format_unacquainted_target_label(
    room_name: &str,
    display_title: &str,
    is_staff: bool,
    has_uniform: bool,
    in_counter_zone: bool,
) -> String {
    if !is_staff || (!has_uniform && !in_counter_zone) {
        return "陌生人".to_string();
    }
    let host_title = crate::grid::resolve_any_room_venue_object(room_name.trim(), display_title.trim())
        .map(|v| v.default_host_title)
        .unwrap_or("當值主事");
    if in_counter_zone && has_uniform {
        format!("櫃檯內場身著工作服的{host_title}")
    } else if in_counter_zone {
        format!("櫃檯內場的{host_title}")
    } else {
        format!("身著工作服的{host_title}")
    }
}

pub fn format_player_work_shift_narrative_and_advance_pack(
    room_name: &str,
    occupation_id: &str,
    wage_mg: i32,
    supply_opt: Option<(i32, &str)>,
) -> String {
    let object_hint = supply_opt.map(|(_, d)| d).unwrap_or(occupation_id);
    if !room_name.trim().is_empty() {
        let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
            room_name.trim(),
            object_hint.trim(),
        );
    }
    if let Some(venue) =
        crate::grid::resolve_any_room_venue_object(room_name.trim(), occupation_id.trim())
    {
        let zone_tag = if venue.has_counter_restricted_zone {
            "當值櫃檯內側"
        } else {
            "當值作業區"
        };
        if let Some((qty, disp)) = supply_opt {
            format!(
                "你在{}{}接過班牌投入「{}」（{}），順著工序一氣呵成完成整輪勞作；盤點封裝好的 {} 份「{}」隨即由專人起運。{}核對名冊後當面點清發放了 {} 鎂工錢，沉甸甸落入你的囊中。",
                venue.venue_kind,
                zone_tag,
                occupation_id,
                venue.on_duty_fixture_activity,
                qty,
                disp,
                venue.default_host_title,
                wage_mg
            )
        } else {
            format!(
                "你在{}{}挽起衣袖投入「{}」（{}），專注而連貫地完成了整整一班的勞作。隨著當值鐘聲敲響，{}核對工冊無誤，當面點清並如數發放了 {} 鎂工錢，整輪差事順利結清。",
                venue.venue_kind,
                zone_tag,
                occupation_id,
                venue.on_duty_fixture_activity,
                venue.default_host_title,
                wage_mg
            )
        }
    } else if let Some((qty, disp)) = supply_opt {
        format!(
            "你接過班牌投入「{}」，順著工序一氣呵成完成整輪勞作；盤點封裝好的 {} 份「{}」隨即由專人起運。管事核對名冊後當面點清發放了 {} 鎂工錢，沉甸甸落入你的囊中。",
            occupation_id,
            qty,
            disp,
            wage_mg
        )
    } else {
        format!(
            "你挽起衣袖投入「{}」，專注而連貫地完成了整整一班的勞作。隨著當值鐘聲敲響，管事核對工冊無誤，當面點清並如數發放了 {} 鎂工錢，整輪差事順利結清。",
            occupation_id,
            wage_mg
        )
    }
}

pub(super) fn player_visible_target_name(player_id: &str, target: &Character) -> String {
    if target.id.starts_with("ambient_") || target.id.starts_with("shanhai_") {
        return display_target_name(target);
    }
    if target.kind == EntityKind::Player {
        return display_target_name(target);
    }
    if let Ok(Some(acq)) = db::get_acquaintance(player_id, &target.id) {
        if acq.named {
            return display_target_name(target);
        }
    }
    let room_name = resolve_action_room_name(player_id, target);
    let (is_staff, has_uniform, in_counter_zone) =
        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            &room_name,
            &target.id,
            &target.display_title,
            &target.current_activity,
            &target.equipment_slots,
        );
    format_unacquainted_target_label(&room_name, &target.display_title, is_staff, has_uniform, in_counter_zone)
}

fn verb_from_action(action: &str) -> Option<Verb> {
    match action {
        "Talk" => Some(Verb::Talk),
        "Borrow" => Some(Verb::Borrow),
        "Subdue" => Some(Verb::Subdue),
        "Slay" => Some(Verb::Slay),
        "Look" => Some(Verb::Look),
        "Trade" => Some(Verb::Trade),
        _ => None,
    }
}

pub(super) fn display_target_name(target: &Character) -> String {
    if (target.id.starts_with("ambient_") || target.id.starts_with("shanhai_"))
        && !target.display_title.trim().is_empty()
    {
        return target.display_title.trim().to_string();
    }
    if target.kind == EntityKind::Npc {
        let gender_str = match target.gender {
            Some(crate::entity::Gender::F) => Some("F"),
            Some(crate::entity::Gender::M) => Some("M"),
            None => None,
        };
        return crate::store::resolve_npc_person_name(&target.id, &target.display_title, gender_str);
    }
    if !target.display_title.is_empty() {
        target.display_title.clone()
    } else {
        target.id.clone()
    }
}

pub(super) fn current_game_hour(cfg: &Server) -> i32 {
    if cfg.game_time_epoch_unix == 0 {
        return 12;
    }
    let (_, h, _, _) = game::game_time_now(
        game::now_unix(),
        cfg.game_time_epoch_unix,
        cfg.game_time_scale,
    );
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_object_and_scene_sockets_have_chinese_action_labels() {
        let sockets = [
            "SmeltBloom",
            "BuildFieldAnvil",
            "SetupHerbStove",
            "WashOre",
            "ForgeRepairStrip",
            "SharpenBlade",
            "WashHerb",
            "DryHerb",
            "DryHerbs",
            "CrushHerb",
            "BoilDecoction",
            "DecoctPotion",
            "Queue",
            "AcceptJob",
            "BuySoup",
            "CarryCargo",
            "HelpRope",
            "TalkQueue",
            "QueueFerry",
            "SellReeds",
            "SellSackcloth",
            "SellRope",
            "BoardFerry",
            "StepAshore",
            "CheckCargo",
            "JoinCheck",
            "RegisterComplaint",
            "QueueBroth",
            "Explore",
            "CrushOre",
            "SmeltIngot",
            "ForgeTool",
            "CastHardware",
            "SortHerbs",
            "GrindPowder",
            "MakePoultice",
            "DistillEssence",
            "SoakFiber",
            "TwistCord",
            "TwistRope",
            "WeaveCloth",
            "OilCanvas",
            "PressPaper",
            "ScrapeHide",
            "BoilFishGlue",
            "CalcifyShell",
            "CobbleBoots",
            "CraftHandle",
            "Chop",
            "QuenchBlade",
            "Work",
        ];
        for &sock in &sockets {
            let zh = action_label_zh(sock);
            assert_ne!(zh, sock, "Socket {} should have a Chinese label", sock);
            assert!(
                !zh.chars().any(|c| c.is_ascii_alphabetic()),
                "Chinese label '{}' for socket {} must not contain ASCII letters",
                zh,
                sock
            );
            let norm = normalize_action_key(zh);
            assert!(
                norm.chars().all(|c| c.is_ascii_alphanumeric()),
                "Normalized key '{}' for label '{}' should be canonical ASCII socket key",
                norm,
                zh
            );
        }
    }

    #[test]
    fn test_format_unacquainted_target_label_rules() {
        assert_eq!(
            format_unacquainted_target_label("老堰頭・水輪石磨工坊", "石磨工", false, true, true),
            "陌生人"
        );
        assert_eq!(
            format_unacquainted_target_label("老堰頭・水輪石磨工坊", "石磨工", true, false, false),
            "陌生人"
        );
        assert_eq!(
            format_unacquainted_target_label("老堰頭・水輪石磨工坊", "石磨匠師", true, true, true),
            "櫃檯內場身著工作服的當值石磨匠師"
        );
        assert_eq!(
            format_unacquainted_target_label("老堰頭・水輪石磨工坊", "石磨匠師", true, false, true),
            "櫃檯內場的當值石磨匠師"
        );
        assert_eq!(
            format_unacquainted_target_label("老堰頭・水輪石磨工坊", "石磨匠師", true, true, false),
            "身著工作服的當值石磨匠師"
        );
        assert_eq!(
            format_unacquainted_target_label("荒蕪礫石空地", "巡山者", true, true, false),
            "身著工作服的當值主事"
        );
    }

    #[test]
    fn test_format_player_work_shift_narrative_venue_and_fallback() {
        let venue_with_supply = format_player_work_shift_narrative_and_advance_pack(
            "老堰頭・水輪石磨工坊",
            "水輪石磨工",
            120,
            Some((5, "細磨小麥麵粉")),
        );
        assert!(venue_with_supply.contains("水輪石磨工坊"));
        assert!(venue_with_supply.contains("當值櫃檯內側"));
        assert!(venue_with_supply.contains("當值石磨匠師"));
        assert!(venue_with_supply.contains("120 鎂工錢"));
        assert!(venue_with_supply.contains("5 份「細磨小麥麵粉」"));

        let venue_no_supply = format_player_work_shift_narrative_and_advance_pack(
            "三水大食堂",
            "大食堂洗切工",
            80,
            None,
        );
        assert!(venue_no_supply.contains("大食堂"));
        assert!(venue_no_supply.contains("當值櫃檯內側"));
        assert!(venue_no_supply.contains("食堂主廚"));
        assert!(venue_no_supply.contains("80 鎂工錢"));

        let fallback_with_supply = format_player_work_shift_narrative_and_advance_pack(
            "荒蕪礫石空地",
            "採石搬運",
            60,
            Some((2, "粗礪碎石")),
        );
        assert!(fallback_with_supply.contains("你接過班牌投入「採石搬運」"));
        assert!(fallback_with_supply.contains("管事核對名冊後當面點清發放了 60 鎂工錢"));

        let fallback_no_supply = format_player_work_shift_narrative_and_advance_pack(
            "荒蕪礫石空地",
            "採石搬運",
            60,
            None,
        );
        assert!(fallback_no_supply.contains("你挽起衣袖投入「採石搬運」"));
        assert!(fallback_no_supply.contains("管事核對工冊無誤"));
    }
}
