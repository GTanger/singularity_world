//! `do_action` — 物件分支：房內非實體目標（RoomObject）的互動與 Move 穿梭。

use crate::db::{
    self, get_object_and_room, get_object_by_id_in_room, get_object_by_name_in_room,
    object_has_socket, object_response, set_entity_grid,
};
use crate::event;

mod types {
    pub use crate::event::types::*;
    pub const PLAYER_BODY: &str = "player_body";
}
use crate::game;
use crate::gametext;
use crate::grid::parse_grid_room_id;
use crate::server::m3_029;
use crate::world::materials;
use std::collections::HashMap;

use super::super::broadcast::{refresh_room_views_for_room, send_room_view_to_session};
use super::super::handler::{WsConnection, push_refresh};
use super::super::protocol::{ActionResultMsg, MovedMsg};
use super::{action_label_zh, client_err_fmt, current_game_hour};

const M3_020_MINT_QUEUE_STATE: &str = "m3_020_mint_queue|queued_after_waiting_line";
const M3_020_SHORT_WORK_STATE: &str = "m3_020_short_work|accepted_loading_shift";
const M3_020_PASS_CHECK_STATE: &str = "m3_020_pass_check|joined_gate_review";
const M3_020_COUNCIL_COMPLAINT_STATE: &str = "m3_020_council_complaint|registered_notice_name";
const M3_027_REED_ITEM_ID: &str = "mat_reed_bundle";
const M3_027_SACKCLOTH_ITEM_ID: &str = "mat_sackcloth_bolt";
const M3_027_ROPE_ITEM_ID: &str = "mat_hemp_rope_roll";
const M3_027_REED_MARKET_DEMAND_PER_ROUND: usize = 2;
const M3_027_SOUP_COST: i32 = 2;
const M3_027_CARGO_PAY: i32 = 3;
const M3_027_REED_SELL_PAY: i32 = 4;
const M3_027_SACKCLOTH_SELL_PAY: i32 = 5;
const M3_027_ROPE_SELL_PAY: i32 = 6;
const M3_027_CARGO_TALLY_PAY: i32 = 1;
const M3_027_REED_GATHERED_STATE: &str = "m3_027_material_flow|gathered_north_bank_reed";
const M3_027_SACKCLOTH_GATHERED_STATE: &str =
    "m3_027_material_flow|gathered_short_market_sackcloth";
const M3_027_ROPE_GATHERED_STATE: &str = "m3_027_material_flow|gathered_ferry_plank_rope";
const M3_027_BENCH_REST_STATE: &str = "m3_027_player_body|rested_on_wet_bench";
const M3_027_SOUP_WARM_STATE: &str = "m3_027_player_body|warmed_by_hot_soup";
const M3_027_FREE_BROTH_STATE: &str = "m3_027_player_body|waited_for_shared_broth";
const M3_027_CARGO_HELP_STATE: &str = "m3_027_ferry_credit|carried_cargo_to_plank";
const M3_027_ROPE_HELP_STATE: &str = "m3_027_ferry_credit|helped_boat_rope";
const M3_027_REED_SOLD_STATE: &str = "m3_027_short_market|sold_reed_bundle";
const M3_027_REED_DEMAND_FILLED_STATE: &str = "m3_027_short_market|reed_demand_filled";
const M3_027_SACKCLOTH_DEMAND_FILLED_STATE: &str = "m3_027_short_market|sackcloth_demand_filled";
const M3_027_ROPE_DEMAND_FILLED_STATE: &str = "m3_027_short_market|rope_demand_filled";
const M3_027_BOARD_FERRY_STATE: &str = "m3_027_ferry_access|boarded_from_plank";
const M3_027_LANDED_SOUTH_STATE: &str = "m3_027_ferry_access|stepped_ashore_south";
const M3_027_WAREHOUSE_TALLY_STATE: &str = "m3_027_cargo_loop|checked_cargo_at_warehouse";
const M3_027_AWNING_DRY_STATE: &str = "m3_027_player_body|dried_under_river_awning";
const M3_027_BRAZIER_WARM_STATE: &str = "m3_027_player_body|warmed_hands_at_brazier";
const M3_027_QUEUE_TALK_STATE: &str = "m3_027_player_social|talked_in_waiting_line";
const M3_027_QUEUE_FERRY_STATE: &str = "m3_027_ferry_access|queued_for_ferry";

fn resolve_room_object(room_id: &str, target_id: &str) -> Option<crate::model::RoomObject> {
    let target = target_id.trim();
    if target == "room_interior"
        || target == "wt_reg_wenyao_home"
        || target.contains("大床")
        || target.contains("羽絨暖被")
        || target.contains("臥榻")
    {
        return Some(crate::model::RoomObject {
            id: "wt_reg_wenyao_home".to_string(),
            name: "松香木暖床".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Use".to_string(),
                "Rest".to_string(),
                "Sleep".to_string(),
                "Operate".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "溫暖乾燥的松香木大床上鋪著蓬鬆羽絨暖被與山風草安神枕，床旁小几上立著一盞旋蓋夜光貝母台燈，光暈柔和溫潤。".to_string()),
                ("Use".to_string(), "你在松香木大床上舒展四肢，蓬鬆羽絨暖被帶來融融暖意，周身疲勞與肌肉酸脹頓時消散大半，身心漸漸平復放鬆。".to_string()),
                ("Rest".to_string(), "你合衣在柔軟的羽絨大床上躺下歇息，安神草的清香讓緊繃的心神徹底放鬆下來，周身氣血漸復充盈。".to_string()),
                ("Sleep".to_string(), "你鑽入溫暖蓬鬆的羽絨暖被，旋暗夜光貝母台燈。在靜謐祥和的屋內，你安然入眠，體魄與心神盡皆恢復。".to_string()),
                ("Operate".to_string(), "你輕輕旋轉床頭夜光貝母台燈的烏木旋蓋，珠白光芒柔和微調，滿室皆是安恬的暖意。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }

    if target == "wt_inn_bath_Preview" || target.contains("自來活水黃銅水栓") || target.contains("溫石熱水槽") {
        return Some(crate::model::RoomObject {
            id: "wt_inn_bath_Preview".to_string(),
            name: "黃銅活水栓".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Bath".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "打磨光滑的黃銅水栓連接著暗渠活水管道，下方是紅土耐火磚砌成的溫石蓄熱水槽，槽邊備有山茶松脂皂與棉長絨擦巾。".to_string()),
                ("Use".to_string(), "你擰開黃銅水栓，清冽純淨的活水流淌而出，隨後轉入溫水檔，溫暖的水流洗去塵泥，草木皂香格外清新。".to_string()),
                ("Bath".to_string(), "你引入溫石槽加熱的熱水浸沐洗面，熱氣騰騰驅散了旅途風塵與肌肉酸脹，倍感舒泰。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }

    if target == "wt_home_shell_lamp" || target.contains("旋蓋夜光貝母") || target.contains("樟木衣箱") {
        return Some(crate::model::RoomObject {
            id: "wt_home_shell_lamp".to_string(),
            name: "夜光貝台燈".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Operate".to_string(), "Open".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "松木几案上立著旋蓋夜光貝母調光台燈，旁側設有一只包銅角的防蛀樟木衣箱，箱蓋微敞散發幽淡樟腦木香。".to_string()),
                ("Use".to_string(), "你旋亮夜光貝母台燈，珠白光暈柔和灑滿几案與床鋪，室內一片溫馨安寧。".to_string()),
                ("Operate".to_string(), "你旋轉烏木外罩調節進光弧度，光影隨心明暗，極具生活雅趣。".to_string()),
                ("Open".to_string(), "你揭開樟木衣箱銅扣，箱內分層鋪著吸濕防潮的竹綿細布，可妥善存放換洗衣物與貴重行囊。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_academy_fountain_pen"
        || target.contains("鋼筆")
        || target.contains("黃銅鋼筆")
        || target.contains("黃銅儲墨筆")
    {
        return Some(crate::model::RoomObject {
            id: "wt_academy_fountain_pen".to_string(),
            name: "黃銅儲墨筆".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Operate".to_string()],
            responses: std::collections::HashMap::from([
                (
                    "Look".to_string(),
                    "烏木教案的正中央擱著一支溫潤沉甸的黃銅桿儲墨鋼筆，筆尖以精煉銥金打磨得細膩微光，旁側襯著吸墨氈布與寫滿批註的竹綿紙講義。".to_string(),
                ),
                (
                    "Use".to_string(),
                    "你執起教案上的黃銅鋼筆，銥金筆尖觸及竹綿紙面，黑墨勻稱流暢地順著筆鋒沁入紙紋，留下一行端勁清晰的手跡。".to_string(),
                ),
                (
                    "Operate".to_string(),
                    "你輕旋黃銅筆桿檢視儲墨囊，墨色飽滿澄澈，筆帽卡榫嚴絲合縫，足見文曜百工製筆之精巧。".to_string(),
                ),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "dyn_campfire" || target.contains("野地營火堆") || target.contains("營火") {
        return Some(crate::model::RoomObject {
            id: "dyn_campfire".to_string(),
            name: "野地營火堆".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "WarmHands".to_string(),
                "RoastFood".to_string(),
                "CraftTorch".to_string(),
                "AddFuel".to_string(),
                "CollectCharcoal".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "粗壯硬柴交錯架起的野地營火堆正熊熊燃燒，金紅火苗隨風輕躍，散發著令人心神安定的融融暖意與松木焦香。".to_string()),
                ("WarmHands".to_string(), "你湊近營火伸出雙手，融融暖流自掌心順著經脈蔓延至周身四肢，驅散了荒野寒濕（體魄與元氣獲得恢復）。".to_string()),
                ("RoastFood".to_string(), "你取出行囊中的乾糧肉脯架於熾熱柴炭上方炙烤，油脂滋滋作響、肉香四溢，趁熱嚼食腹中溫暖充實（深幅恢復體魄與元氣）。".to_string()),
                ("CraftTorch".to_string(), "你自營火堆中抽出一根引燃的松脂硬木，細心纏裹封固，製成一柄劈啪作響的照明松明火把。".to_string()),
                ("AddFuel".to_string(), "你拾起乾柴添入火堆中央，劈啪脆響中火勢陡然騰起，騰躍的金紅火舌將營地周遭照耀得一片透亮。".to_string()),
                ("CollectCharcoal".to_string(), "你以長枝撥開外層浮灰，將底層悶燒透紅的硬木火炭小心夾出、封土隔氧冷卻，收整出成色極佳的無煙白炭。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    let now_fac = crate::game::now_unix();
    if let Some(fac) = crate::game::object_ecology::active_field_facility_objects(room_id, now_fac)
        .into_iter()
        .find(|obj| {
            obj.id == target
                || obj.name == target
                || obj.name.contains(target)
                || target.contains(&obj.name)
        })
    {
        return Some(fac);
    }
    if target == "wt_multi_firewood_pile" || target.contains("風乾柴木與良木料垛") || target.contains("木料垛") || target.contains("柴木垛") {
        return Some(crate::model::RoomObject {
            id: "wt_multi_firewood_pile".to_string(),
            name: "風乾柴木垛".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Gather".to_string(),
                "MakeFire".to_string(),
                "SplitWood".to_string(),
                "RefinePlank".to_string(),
                "BurnCharcoal".to_string(),
                "CraftTorch".to_string(),
                "RepairStructure".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "整齊碼放於簷下或角隅的風乾柴木與硬木料垛，木質乾燥密實，既是用以生火禦寒之良材，亦能順紋劈柴、裁鋸木板、進爐悶燒精炭或車削長柄。".to_string()),
                ("Gather".to_string(), "你自木料垛中細細翻揀，抱出一根紋理密實的伐採原木與兩束乾燥柴枝收整妥當。".to_string()),
                ("MakeFire".to_string(), "你取出乾燥柴枝引燃火星，在空地中央架起通風柴堆，迅速生起一堆溫暖明亮的野地營火。".to_string()),
                ("SplitWood".to_string(), "你運力順著木紋利索剖劈，將原木細細剖分劈成易於引火的乾燥柴枝束。".to_string()),
                ("RefinePlank".to_string(), "你推動長鋸順紋切削，將沉實原木裁鋸打磨為平整規整的樟松木板料。".to_string()),
                ("BurnCharcoal".to_string(), "你將截切硬木置入坑窯泥封悶燒，凝煉出無煙透亮的硬木白炭。".to_string()),
                ("CraftTorch".to_string(), "你挑選油脂豐富的松木枝條塗抹脂膏緊密紮裹，製成一柄照明驅寒的浸脂松明火把。".to_string()),
                ("RepairStructure".to_string(), "你挑選合適的木板料與工具長柄，加固修整破損的欄杆踏板與設施構件。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }


    get_object_by_id_in_room(room_id, target_id)
        .or_else(|| get_object_by_name_in_room(room_id, target_id))
        .or_else(|| get_object_and_room(target_id).and_then(|(o, r)| (r == room_id).then_some(o)))
        .or_else(|| resolve_grid_scene_object(room_id, target_id))
        .or_else(|| resolve_runtime_resource(room_id, target_id))
        .or_else(|| {
            game::player_layer_scene_objects_for_room(room_id)
                .into_iter()
                .find(|obj| obj.id == target_id || obj.name == target_id)
        })
        .or_else(|| {
            crate::game::peninsula_catalog::find_catalog_entry(target_id)
                .map(|e| e.to_room_object())
        })
}
fn resolve_runtime_resource(room_id: &str, target_id: &str) -> Option<crate::model::RoomObject> {
    let pool = crate::pg::pool()?;
    let source = crate::pg::world_resource::get_at_place(&pool, room_id)
        .ok()
        .flatten()?;
    let id = format!("resource:{}:{}", source.place_id, source.item_id);
    if target_id != id {
        return None;
    }
    let state = if source.current_stock > 0 {
        format!("目前仍可取得約{}份。", source.current_stock)
    } else {
        "眼前能取用的部分已空，必須等它慢慢恢復。".to_string()
    };
    Some(crate::model::RoomObject {
        id,
        name: format!("可取用的{}", source.name),
        owner: String::new(),
        sockets: vec!["Look".to_string(), "Gather".to_string()],
        responses: std::collections::HashMap::from([(
            "Look".to_string(),
            format!("{}；{}", source.origin_label, state),
        )]),
        move_to_room_id: String::new(),
    })
}

fn resolve_grid_scene_object(room_id: &str, target_id: &str) -> Option<crate::model::RoomObject> {
    let target = target_id.trim();
    if target.is_empty() {
        return None;
    }
    let coord = parse_grid_room_id(room_id)?;
    let grid = crate::server::grid_manager::get_runtime_square_grid()?;
    let cell = grid.get(coord)?;

    // 0.8) 檢查動態野地設施（dyn_field_anvil, dyn_herb_stove 等）
    let now_fac = crate::game::now_unix();
    if let Some(fac) = crate::game::object_ecology::active_field_facility_objects(room_id, now_fac)
        .into_iter()
        .find(|obj| {
            obj.id == target
                || obj.name == target
                || obj.name.contains(target)
                || target.contains(&obj.name)
        })
    {
        return Some(fac);
    }
    // 1) 先查 game::player_layer_scene_objects_for_room
    if let Some(obj) = game::player_layer_scene_objects_for_room(&cell.name)
        .into_iter()
        .find(|obj| {
            obj.id == target
                || obj.name == target
                || obj.name.contains(target)
                || target.contains(&obj.name)
        })
    {
        return Some(obj);
    }

    // 1.5) 查詢同格 NPC 並匹配動態攤位物件
    let gh = crate::store::current_game_hour();
    if let Ok(entities) = crate::db::get_entities_in_room(room_id, gh)
        .or_else(|_| crate::db::get_entities_at_grid(coord.x, coord.y, gh))
    {
        if let Some(obj) = crate::game::player_scene::dynamic_vendor_objects_for_entities(&entities)
            .into_iter()
            .find(|obj| {
                obj.id == target
                    || obj.name == target
                    || obj.name.contains(target)
                    || target.contains(&obj.name)
            })
        {
            return Some(obj);
        }
    }
    // 2) 檢查 cell.objects（GridObject）
    if let Some(obj) = cell.objects.iter().find(|o| {
        o.id == target
            || o.name == target
            || o.name.contains(target)
            || target.contains(&o.name)
    }) {
        let sockets = vec!["Look".to_string(), "Gather".to_string()];
        let look_desc = if obj.quantity > 0 {
            format!("你湊近端詳「{}」，此處物產尚存、生機完好，可順手採集取用。", obj.name)
        } else {
            format!("眼前的「{}」已被採擷一空，尚待時序更迭自然復生。", obj.name)
        };
        return Some(crate::model::RoomObject {
            id: obj.id.clone(),
            name: obj.name.clone(),
            owner: String::new(),
            sockets,
            responses: std::collections::HashMap::from([("Look".to_string(), look_desc)]),
            move_to_room_id: String::new(),
        });
    }

    // 3) 檢查是否為環境地景名稱或沉浸式兜底
    let scenery_desc = match target {
        "平整道石" => Some("青灰道石經年累月受車馬行旅踏磨，表面溫潤平實，石縫間嵌著細碎砂土與微濕青苔。".to_string()),
        "沿途界碑" => Some("半人高的風化石碑立於道旁，碑面刻著辨識方位的古舊字跡，底座覆著一層淺淺塵土。".to_string()),
        "白塔盤旋石階" => Some("素白石階循著塔身緩緩盤旋而上，階面潔淨微涼，隱約可見歷代巡塔人留下的足跡。".to_string()),
        "引靈拱券石基" => Some("厚重的拱券石基承托著高聳廊道，石紋間似有微弱的溫熱氣息緩緩流轉。".to_string()),
        "素白玉壁" => Some("壁面由整塊素白石材打磨砌成，觸手生涼，映著天光泛起柔和的光澤。".to_string()),
        "潮打青石階" => Some("石階下半截常年受潮水拍打，布滿深色水痕與細小貝殼，踏上去略帶濕滑。".to_string()),
        "泊舟木樁" => Some("粗壯的硬木樁深釘入岸邊泥石，樁身纏繞著磨出毛邊的粗麻纜繩，散發淡淡鹹腥水氣。".to_string()),
        "水畔蘆葦叢" => Some("密實的蘆葦隨風起伏搖曳，根部泥水清淺，偶有水蟲與飛鳥掠過。".to_string()),
        "青苔怪石" => Some("嶙峋山石表面覆滿厚實翠綠的青苔，背陰處凝著冰涼露珠。".to_string()),
        "盤根古木" => Some("蒼老古木的粗壯根系如虯龍般盤繞地表，枝葉繁茂，遮蔽天日。".to_string()),
        "林間落葉" => Some("層層疊疊的枯葉鋪滿林地，踩踏其上發出細碎聲響，散發著泥土與草木腐殖的清香。".to_string()),
        _ => {
            if !target.is_empty()
                && target.chars().count() <= 32
                && !target.chars().any(|c| c.is_control() || c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                Some(format!(
                    "你駐足細看{}一帶的「{}」，其肌理與周遭地勢自然相融，別有一番風物韻致。",
                    if cell.name.is_empty() { "此處" } else { &cell.name },
                    target
                ))
            } else {
                None
            }
        }
    };

    if let Some(desc) = scenery_desc {
        return Some(crate::model::RoomObject {
            id: target.to_string(),
            name: target.to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string()],
            responses: std::collections::HashMap::from([("Look".to_string(), desc)]),
            move_to_room_id: String::new(),
        });
    }

    None
}

pub(super) fn do_object_branch(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    target_id: &str,
    action: &str,
) {
    let Some(obj) = resolve_room_object(player_room, target_id) else {
        conn.send_error(gametext::client("target_not_found"));
        return;
    };
    let allow_rest_meditate = (action == "Rest" || action == "Meditate")
        && (obj.name.contains("床")
            || obj.name.contains("榻")
            || obj.name.contains("席")
            || obj.name.contains("凳")
            || obj.name.contains("椅")
            || obj.name.contains("案")
            || obj.name.contains("蒲團")
            || obj.name.contains("坐墊")
            || obj.name.contains("石階")
            || obj.name.contains("埠階")
            || obj.id.contains("bed")
            || obj.id.contains("bench")
            || obj.id.contains("chair")
            || obj.id.contains("tea")
            || obj.id.contains("clinic"));
    if !object_has_socket(&obj, action) && !allow_rest_meditate {
        conn.send_error(client_err_fmt("cannot_action_fmt", action));
        return;
    }
    let now = game::now_unix();
    if action == "MakeFire" {
        match crate::game::object_ecology::ignite_or_stoke_campfire(pid, player_room, None, now) {
            Ok((msg, _is_new)) => {
                push_refresh(conn, pid);
                refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative: msg,
                    success: true,
                    actions: vec![
                        "WarmHands".into(),
                        "RoastFood".into(),
                        "CraftTorch".into(),
                        "AddFuel".into(),
                        "CollectCharcoal".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    }
    if obj.id == "dyn_campfire" && matches!(action, "WarmHands" | "RoastFood" | "CollectCharcoal") {
        match crate::game::object_ecology::interact_with_campfire(pid, player_room, action, now) {
            Ok(narrative) => {
                push_refresh(conn, pid);
                refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                let actions = if action == "CollectCharcoal" {
                    vec!["Look".into(), "MakeFire".into(), "Gather".into()]
                } else {
                    vec![
                        "WarmHands".into(),
                        "RoastFood".into(),
                        "CraftTorch".into(),
                        "AddFuel".into(),
                        "CollectCharcoal".into(),
                    ]
                };
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions,
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    }
    if obj.id == "dyn_field_anvil" || obj.id == "dyn_herb_stove" {
        match crate::game::object_ecology::interact_with_field_facility(pid, player_room, &obj.id, action, now) {
            Ok(narrative) => {
                push_refresh(conn, pid);
                refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                let actions = if obj.id == "dyn_field_anvil" {
                    vec!["Look".into(), "ForgeRepairStrip".into(), "SharpenBlade".into()]
                } else {
                    vec!["Look".into(), "BoilDecoction".into(), "DryHerb".into(), "CrushHerb".into()]
                };
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions,
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    }
    if obj.id == "wt_multi_firewood_pile" && action == "Gather" {
        let _ = db::add_to_inventory(pid, "mat_timber_log", 1);
        let _ = db::add_to_inventory(pid, "mat_kindling_bundle", 2);
        push_refresh(conn, pid);
        let narrative = "你走至簷角木料垛前細緻翻檢，從中挑選並抽取出一根質地堅實的「伐採原木」與兩束乾燥易燃的「乾燥柴枝束」妥善收入行囊。".to_string();
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative,
            success: true,
            actions: vec![
                "MakeFire".into(),
                "SplitWood".into(),
                "RefinePlank".into(),
                "BurnCharcoal".into(),
                "CraftTorch".into(),
                "RepairStructure".into(),
            ],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }
    if action == "AddFuel" {
        if obj.id == "dyn_campfire" || obj.name.contains("營火") {
            match crate::game::object_ecology::interact_with_campfire(pid, player_room, "AddFuel", now) {
                Ok(narrative) => {
                    push_refresh(conn, pid);
                    refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                    conn.send_json(&ActionResultMsg {
                        msg_type: "action_result".into(),
                        action: action.into(),
                        target_id: obj.id.clone(),
                        target_name: obj.name.clone(),
                        narrative,
                        success: true,
                        actions: vec![
                            "WarmHands".into(),
                            "RoastFood".into(),
                            "CraftTorch".into(),
                            "AddFuel".into(),
                            "CollectCharcoal".into(),
                        ],
                        move_target_id: String::new(),
                        open_trade: false,
                        suggested_action: String::new(),
                        tactical_choices: vec![],
                    });
                    return;
                }
                Err(e) => {
                    conn.send_error(e);
                    return;
                }
            }
        } else {
            match crate::game::object_ecology::stoke_workshop_furnace(pid, player_room, now) {
                Ok(narrative) => {
                    push_refresh(conn, pid);
                    conn.send_json(&ActionResultMsg {
                        msg_type: "action_result".into(),
                        action: action.into(),
                        target_id: obj.id.clone(),
                        target_name: obj.name.clone(),
                        narrative,
                        success: true,
                        actions: vec![
                            "Operate".into(),
                            "AddFuel".into(),
                            "RefinePlank".into(),
                            "BurnCharcoal".into(),
                            "CraftHandle".into(),
                            "RepairStructure".into(),
                        ],
                        move_target_id: String::new(),
                        open_trade: false,
                        suggested_action: String::new(),
                        tactical_choices: vec![],
                    });
                    return;
                }
                Err(e) => {
                    conn.send_error(e);
                    return;
                }
            }
        }
    }
    if action == "RepairStructure" {
        match crate::game::object_ecology::repair_room_structure(pid, player_room, &obj.name, now) {
            Ok(narrative) => {
                push_refresh(conn, pid);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions: vec![
                        "Look".into(),
                        "MakeFire".into(),
                        "SplitWood".into(),
                        "RefinePlank".into(),
                        "BurnCharcoal".into(),
                        "CraftTorch".into(),
                        "RepairStructure".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    }
    if matches!(
        action,
        "SplitWood" | "RefinePlank" | "BurnCharcoal" | "CraftTorch" | "CraftHandle"
    ) {
        let res = if obj.id == "dyn_campfire" && action == "CraftTorch" {
            refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
            crate::game::object_ecology::interact_with_campfire(pid, player_room, "CraftTorch", now)
                .map(|narr| ("craft_pine_torch".to_string(), "浸脂松明火把".to_string(), narr))
        } else {
            crate::game::object_ecology::refine_wood_multi_socket(pid, action, player_room, now)
        };
        match res {
            Ok((_prod_id, _prod_name, narrative)) => {
                push_refresh(conn, pid);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions: vec![
                        "MakeFire".into(),
                        "AddFuel".into(),
                        "RefinePlank".into(),
                        "BurnCharcoal".into(),
                        "CraftTorch".into(),
                        "RepairStructure".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    }
    if matches!(
        action,
        "WashOre" | "SmeltBloom" | "ForgeRepairStrip" | "SharpenBlade" | "BuildFieldAnvil"
    ) {
        let res = crate::game::object_ecology::refine_mineral_multi_socket(pid, action, player_room, now);
        match res {
            Ok((_prod_id, _prod_name, narrative)) => {
                push_refresh(conn, pid);
                if action == "BuildFieldAnvil" {
                    refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                }
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions: vec![
                        "WashOre".into(),
                        "SmeltBloom".into(),
                        "ForgeRepairStrip".into(),
                        "SharpenBlade".into(),
                        "BuildFieldAnvil".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    }
    if matches!(
        action,
        "WashHerb" | "DryHerb" | "CrushHerb" | "BoilDecoction" | "SetupHerbStove"
    ) {
        let res = crate::game::object_ecology::refine_herb_multi_socket(pid, action, player_room, now);
        match res {
            Ok((_prod_id, _prod_name, narrative)) => {
                push_refresh(conn, pid);
                if action == "SetupHerbStove" {
                    refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                }
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions: vec![
                        "WashHerb".into(),
                        "DryHerb".into(),
                        "CrushHerb".into(),
                        "BoilDecoction".into(),
                        "SetupHerbStove".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    }
    if matches!(
        action,
        "TwistRope" | "WeaveSackcloth" | "OilCanvas" | "PressPaper"
    ) {
        let res = crate::game::object_ecology::refine_fiber_multi_socket(pid, action, player_room, now);
        match res {
            Ok((_prod_id, _prod_name, narrative)) => {
                push_refresh(conn, pid);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions: vec![
                        "TwistRope".into(),
                        "WeaveSackcloth".into(),
                        "OilCanvas".into(),
                        "PressPaper".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    }
    if matches!(
        action,
        "ScrapeHide" | "BoilFishGlue" | "CalcifyShell" | "CobbleBoots"
    ) {
        let res = crate::game::object_ecology::refine_hide_fish_multi_socket(pid, action, player_room, now);
        match res {
            Ok((_prod_id, _prod_name, narrative)) => {
                push_refresh(conn, pid);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions: vec![
                        "ScrapeHide".into(),
                        "BoilFishGlue".into(),
                        "CalcifyShell".into(),
                        "CobbleBoots".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
            Err(e) => {
                conn.send_error(e);
                return;
            }
        }
    }
    if try_grid_cell_object_action(conn, pid, player_room, &obj, action) {
        return;
    }
    if try_runtime_resource_action(conn, pid, player_room, &obj, action, now) {
        return;
    }
    if try_m3_027_material_gather(conn, pid, player_room, &obj, action, now) {
        return;
    }
    let needs_scene_history = m3_020_scene_object_uses_history(&obj.id, action)
        || m3_027_scene_object_uses_history(&obj.id, action);
    let mut prior_scene_actions = if needs_scene_history {
        recent_scene_object_action_payloads(pid, now)
    } else {
        Vec::new()
    };
    let mut prior_scene_states = if needs_scene_history {
        recent_scene_object_state_payloads(pid, now)
    } else {
        Vec::new()
    };
    if needs_scene_history {
        m3_029::merge_shared_market_history(&mut prior_scene_actions, &mut prior_scene_states, now);
    }
    if matches!(
        (obj.id.as_str(), action),
        ("m3_020_hire_short_work_boards", "AcceptJob")
            | ("m3_020_stone_gate_pass_tags", "JoinCheck")
            | ("m3_020_council_notice_strip", "RegisterComplaint")
    ) {
        let read_action = format!("{}|Read", obj.id);
        if !prior_scene_actions.iter().any(|a| a == &read_action) {
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &read_action);
            prior_scene_actions.push(read_action);
        }
    }
    if let Some(denial) = m3_027_scene_object_action_denial(
        pid,
        &obj.id,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    ) {
        let (others, move_target_id) = object_followup_buttons(
            &obj,
            player_room,
            action,
            &prior_scene_actions,
            &prior_scene_states,
        );
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative: denial.to_string(),
            success: false,
            actions: others,
            move_target_id,
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }
    if obj.id == "m3_027_market_reed_buyer" && action == "SellReeds" {
        let available = player_inventory_item_quantity(pid, M3_027_REED_ITEM_ID);
        let batch_count =
            m3_027_reed_batch_sale_count(available, &prior_scene_actions, &prior_scene_states);
        if batch_count >= 2 {
            let _ = event::append(now, pid, types::OBSERVED, &obj.id);
            for _ in 0..batch_count {
                let _ = event::append(
                    now,
                    pid,
                    types::SCENE_OBJECT_ACTION,
                    "m3_027_market_reed_buyer|SellReeds",
                );
                prior_scene_actions.push("m3_027_market_reed_buyer|SellReeds".to_string());
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, M3_027_REED_SOLD_STATE);
            let _ = event::append(
                now,
                pid,
                types::SCENE_OBJECT_STATE,
                M3_027_REED_DEMAND_FILLED_STATE,
            );
            prior_scene_states.push(M3_027_REED_SOLD_STATE.to_string());
            prior_scene_states.push(M3_027_REED_DEMAND_FILLED_STATE.to_string());

            let _ = db::remove_from_inventory(pid, M3_027_REED_ITEM_ID, batch_count as i32);
            let total_pay = (batch_count as i32) * M3_027_REED_SELL_PAY;
            let _ = db::add_magnesium(pid, total_pay);
            push_refresh(conn, pid);

            let (others, move_target_id) = object_followup_buttons(
                &obj,
                player_room,
                action,
                &prior_scene_actions,
                &prior_scene_states,
            );
            let narrative = format!(
                "你將剛割下的 {} 束蘆草一次性齊齊交到秤邊。收牌人俐落地連續起秤、掂過濕重，旁邊搬貨人隨即抱起草束，一口氣將底層貨籃的四角墊得嚴嚴實實。收牌人點出 {} 鎂交到你手中，順手將價牌翻至背面，示意這一輪水尾寮渡頭市集的蘆草需求已徹底結清。",
                batch_count, total_pay
            );
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.into(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative,
                success: true,
                actions: others,
                move_target_id,
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return;
        }
    }
    let mut narrative = m3_027_scene_object_state_action_response(
        &obj.id,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    )
    .or_else(|| {
        m3_020_scene_object_state_action_response(
            &obj.id,
            action,
            &prior_scene_actions,
            &prior_scene_states,
        )
    })
    .map(str::to_string)
    .unwrap_or_else(|| object_response(&obj, action));

    let is_street_cart = obj.id == "wt_street_roast_snack_cart"
        || obj.name.contains("炭烤")
        || obj.name.contains("豆花攤")
        || obj.name.contains("茶水攤");
    let gh = current_game_hour(&conn.cfg);
    let is_cart_night_closed = !(6..22).contains(&gh);

    if is_street_cart && is_cart_night_closed {
        match action {
            "Look" => {
                narrative = "夜深休市，街角小推車的炭火早已熄滅收攏，鐵網擦拭乾淨並蓋上了防露帆布。".to_string();
            }
            "Taste" => {
                narrative = "攤車已在深夜收攤熄火，待天亮開市後再來品嚐熱食吧。".to_string();
            }
            "Smell" => {
                narrative = "夜風微涼，熄火的推車旁僅餘一絲淡淡的木炭餘味。".to_string();
            }
            _ => {}
        }
    }

    if narrative.is_empty() {
        let act_label = action_label_zh(action);
        narrative = format!(
            "你對「{}」執行了「{act_label}」，但似乎沒有什麼特別的反應。",
            obj.name
        );
    }
    if action == "Look"
        && let Some(echo) = m3_027_scene_object_look_echo(
            player_room,
            &obj.id,
            now,
            &prior_scene_actions,
            &prior_scene_states,
        )
        .or_else(|| {
            m3_020_scene_object_look_echo(&obj.id, &prior_scene_actions, &prior_scene_states)
                .map(str::to_string)
        })
    {
        narrative.push('\n');
        narrative.push_str(&echo);
    }
    if obj.id.starts_with("hunt_kill_site_") {
        let (gx, gy) = if let Some(rest) = obj.id.strip_prefix("hunt_kill_site_") {
            let mut parts = rest.split('_');
            if let (Some(xs), Some(ys)) = (parts.next(), parts.next())
                && let (Ok(x), Ok(y)) = (xs.parse::<i32>(), ys.parse::<i32>())
            {
                (x, y)
            } else {
                db::get_entity(pid).ok().flatten().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0))
            }
        } else {
            db::get_entity(pid).ok().flatten().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0))
        };

        if action == "Look" {
            let site_opt = crate::game::peek_recent_hunt_kill_site(gx, gy, now);
            let narrative = if let Some(site) = site_opt {
                format!(
                    "領隊【{}】率合社獵團於此處四面合圍搏殺【{}】後留下的新鮮現場，草地上尚留有餘溫獸跡與散落骨角，可執行【採集 (Harvest)】拾取遺留獸材【{}】。",
                    site.hunter_label, site.prey_name, site.loot_name
                )
            } else {
                "此處圍獵痕跡已漸次風化冷卻，獵團成員早已分鹿割肉、各自返程，僅餘些許雜亂足印。".to_string()
            };
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.to_string(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative,
                success: true,
                actions: vec!["Harvest".into()],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return;
        }

        if matches!(action, "Harvest" | "Gather" | "Take") {
            if let Some(site) = crate::game::take_recent_hunt_kill_site(gx, gy, now) {
                let _ = db::add_to_inventory(pid, &site.loot_id, 1);
                let narrative = format!(
                    "你俯身清理合社圍獵【{}】後留下的現場，細心從殘骨與草叢間剝取收納了新鮮獸材【{}】（已入行囊）！現場餘溫漸漸散去。",
                    site.prey_name, site.loot_name
                );
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.to_string(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                push_refresh(conn, pid);
                return;
            } else {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.to_string(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative: "此處遺留的獸材殘軀已被取走或已隨風化散，未尋得更多材料。".to_string(),
                    success: false,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
        }
    }
    if action == "Look" {
        let item_id_candidate = if let Some(stripped) = obj.id.strip_prefix("resource:") {
            stripped.split(':').nth(1).unwrap_or(stripped)
        } else {
            obj.id.as_str()
        };
        let origin = crate::game::object_ecology::lookup_item_origin(item_id_candidate);
        if !origin.is_empty() && !narrative.contains("【產地溯源：") {
            narrative.push_str(&format!("\n【產地溯源：{}】", origin));
        }
        if (obj.id == "shanhai_apex_trace"
            || obj.name.contains("霸主獸跡")
            || obj.name.contains("獸徑足印")
            || obj.name.contains("避風岩龕")
            || obj.name.contains("山澗清泉眼")
            || obj.name.contains("巡山木樁")
            || obj.id == "dyn_campfire")
            && !narrative.contains("【山海異兆】")
        {
            let room_label = resolve_room_name(player_room);
            if let Some(omen) = crate::game::hunting_zone_apex_omen(&room_label) {
                narrative.push('\n');
                narrative.push_str(omen);
            }
            if (obj.id == "shanhai_apex_trace"
                || obj.name.contains("霸主獸跡")
                || obj.name.contains("獸徑足印"))
                && let Some((gx, gy)) = db::get_entity(pid).ok().flatten().and_then(|e| e.grid_x.zip(e.grid_y))
            {
                crate::game::mark_apex_beast_tracked_at(gx, gy);
                narrative.push_str(
                    "\n【尋跡破霧】順著地上深陷的爪印與殘留靈氣撥開深林草莽，暗處一雙威凜獸瞳驟然亮起——潛伏此地的鎮場霸主已破霧顯形！",
                );
            }
            if obj.name.contains("山澗清泉眼") && !db::inventory_has_item(pid, "crystal_water_bing") {
                let _ = db::add_to_inventory(pid, "crystal_water_bing", 1);
                narrative.push_str("\n【觀境凝晶】清泉噴湧翻滾間水汽蒸蔚，一滴靈泉受靈台照徹，在掌中結為【水源・丙品詞元結晶】。");
            } else if obj.name.contains("避風岩龕") && !db::inventory_has_item(pid, "crystal_qi_bing") {
                let _ = db::add_to_inventory(pid, "crystal_qi_bing", 1);
                narrative.push_str("\n【觀境凝晶】岩龕崖隙罡風低迴盤旋，至清至純的一縷清氣在掌中結為【氣源・丙品詞元結晶】。");
            }
        }
    }
    let _ = event::append(now, pid, types::OBSERVED, &obj.id);
    if !apply_object_move(conn, pid, player_room, &obj, action) {
        return;
    }
    if let Some(payload) = m3_020_scene_object_followup_event_payload(&obj.id, action)
        .or_else(|| m3_027_scene_object_followup_event_payload(&obj.id, action))
    {
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &payload);
    }
    let state_payload = m3_020_scene_object_state_event_payload(
        &obj.id,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    )
    .or_else(|| {
        m3_027_scene_object_state_event_payload(
            &obj.id,
            action,
            &prior_scene_actions,
            &prior_scene_states,
        )
    });
    let state_written = if let Some(payload) = state_payload {
        event::append(now, pid, types::SCENE_OBJECT_STATE, payload).is_ok()
    } else {
        false
    };
    if state_written && m3_027_scene_object_apply_side_effects(pid, &obj.id, action, state_payload)
    {
        push_refresh(conn, pid);
    }
    if (!is_street_cart || !is_cart_night_closed)
        && (action != "Look"
            || obj.id.starts_with("wt_stall_")
            || obj.id == "wt_street_roast_snack_cart"
            || obj.id == "wt_bridge_sundial_pillar"
            || obj.name.contains("日晷")
            || obj.name.contains("石柱")
            || obj.id == "wt_street_news_board"
            || obj.name.contains("閱報")
            || obj.name.contains("旬報"))
    {
        apply_peninsula_world_object_effects(
            conn,
            pid,
            player_room,
            &obj,
            action,
            now,
            &mut narrative,
        );
    }
    let (others, move_target_id) = object_followup_buttons(
        &obj,
        player_room,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    );
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: action.into(),
        target_id: obj.id.clone(),
        target_name: obj.name.clone(),
        narrative,
        success: true,
        actions: others,
        move_target_id,
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}

fn resolve_room_name(player_room: &str) -> String {
    if let Some(coord) = parse_grid_room_id(player_room) {
        if let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() {
            if let Some(cell) = grid.get(coord) {
                return cell.name.clone();
            }
        }
    }
    player_room.to_string()
}

fn apply_peninsula_world_object_effects(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now: i64,
    narrative: &mut String,
) {
    let is_medical_or_rest = obj.name.contains("診")
        || obj.name.contains("醫")
        || obj.name.contains("病床")
        || obj.name.contains("藥")
        || obj.name.contains("草")
        || obj.name.contains("榻")
        || obj.name.contains("床")
        || obj.name.contains("蒲團")
        || obj.name.contains("坐墊");

    if matches!(action, "Use" | "Operate" | "Rest" | "Meditate" | "Acquire") && is_medical_or_rest {
        let include_miasma = obj.name.contains("藥")
            || obj.name.contains("草")
            || obj.id.contains("herb");
        let cured = crate::pg::living_world::clear_entity_afflictions(pid, include_miasma, now);
        if !cured.is_empty() {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = 100;
                    e.qi = 100;
                });
            }
            let _ = db::update_vit(pid, 100);
            let cured_labels: Vec<&'static str> = cured.iter().map(|c| c.label()).collect();
            narrative.push_str(&format!(
                "\n【調理痊癒】經一番悉心敷藥與調息，你身上的「{}」已然痊癒，周身氣機重歸暢通。",
                cured_labels.join("、")
            ));
            push_refresh(conn, pid);
        }
    }

    if !obj.id.starts_with("wt_") && !obj.name.contains("紅印屋契") && !obj.name.contains("委託") && !is_medical_or_rest && action != "Rest" && action != "Meditate" {
        return;
    }
    if (obj.id.starts_with("wt_stall_") || obj.id == "wt_street_roast_snack_cart") && action == "Look" {
        if let Some((_item_id, item_name, price_mg, stock)) =
            crate::server::action::trade::get_vendor_stall_stock(&obj.id)
        {
            narrative.push_str(&format!(
                "\n（攤上正販售【{item_name}】，每份售價 {price_mg} 鎂，目前現貨尚餘 {stock} 份。）"
            ));
        }
        return;
    }
    if matches!(action, "Use" | "Operate" | "Taste" | "Wear")
        && let Some((tag, vit_gain, qi_gain, _consume, effect_narr)) = crate::game::shanhai_item_use_effect(&obj.id)
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + vit_gain).min(100);
                e.qi = (e.qi + qi_gain).min(100);
            });
        }
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        crate::pg::living_world::clear_entity_afflictions(pid, true, now);
        narrative.push_str(&format!(
            "\n【{tag}】{effect_narr}（山海本草化入肺腑，氣血隨之充盈、精神一振）"
        ));
        push_refresh(conn, pid);
        return;
    }
    if let Ok(Some(ch)) = db::get_entity(pid)
        && ch.magnesium == 0
        && (ch.inventory.is_empty() || ch.inventory == "[]")
    {
        let _ = db::add_magnesium(pid, 100);
    }
    // 1) Property Deeds (obj.id == "wt_cat_0793" or obj.id == "wt_reg_capital_specialty" or obj.name.contains("紅印屋契"))
    if (obj.id == "wt_cat_0793"
        || obj.id == "wt_reg_capital_specialty"
        || obj.name.contains("紅印屋契"))
        && (action == "Acquire" || action == "Use" || action == "Operate")
    {
        let magnesium = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| e.magnesium)
            .unwrap_or(0);
        let has_deed = db::inventory_has_item(pid, "wt_cat_0793");
        if !has_deed {
            if magnesium >= 25 {
                let _ = db::add_magnesium(pid, -25);
                let _ = db::add_to_inventory(pid, "wt_cat_0793", 1);
                let _ = db::add_to_inventory(pid, "key_brass_home_lock", 1);
                let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "wt_deed_signed");
                narrative.push_str("\n【置產落籍】你交付 25 鎂契稅定金，正式簽立「公議堂紅印屋契」並領取一把「黃銅彈子門鎖鑰匙」收入行囊，在半島擁有了屬於自己的安身家屋。");
                push_refresh(conn, pid);
            } else {
                narrative.push_str("\n【置產提示】簽立正式紅印屋契需備妥 25 鎂契稅定金，你目前已先行翻閱契書條款並完成意向登記。");
            }
        } else {
            narrative.push_str("\n【置產落籍】你已持有「公議堂紅印屋契」，隨時可憑契文與門鎖鑰匙出入並安居於半島家屋。");
        }
        return;
    }

    // 1.5) 文曜特色器物實質互動回饋
    // 講堂街三百六十星軌銅儀 (wt_cat_0734) - 觀星測韻
    if (obj.id == "wt_cat_0734" || obj.id == "wt_astronomical_armillary" || obj.name.contains("星軌銅儀"))
        && (action == "Operate" || action == "Read" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::PLAYER_BODY, "astronomical_calibrated");
        narrative.push_str("\n【觀星測韻】你輕輕轉動三層黃銅星環，黃道規線與子午環在齒輪咬合間微調校準。望筒深處映現星軌明滅，清冷星光透體而過，你只覺靈台澄澈、元氣湧動，獲得星軌校準領悟（元氣獲得恢復）。");
        if !db::inventory_has_item(pid, "crystal_qi_bing") {
            let _ = db::add_to_inventory(pid, "crystal_qi_bing", 1);
            narrative.push_str("\n【觀境凝晶】星軌齒輪咬合轉動之際，清曜星輝於儀軌心軸凝為澄澈明淨的【氣源・丙品詞元結晶】，被你妥善收存入囊。");
        }
        if let Ok((_mut_id, _mut_name, mut_narrative)) =
            crate::game::object_ecology::mutate_finished_item(pid, "auto", &obj.name, now)
        {
            narrative.push_str(&format!("\n【星軌氣脈共鳴異變】在星環轉動與星光垂照之下，{}", mut_narrative));
        }
        push_refresh(conn, pid);
        return;
    }

    // 三水總商行鎂幣分格存摺櫃 / 存摺 (wt_cat_0790) - 商行票號
    if (obj.id == "wt_cat_0790" || obj.id == "wt_bank_ledger_drawer" || obj.name.contains("存摺") || obj.name.contains("票號"))
        && (action == "Operate" || action == "Read" || action == "Open" || action == "Use")
    {
        let magnesium = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| e.magnesium)
            .unwrap_or(0);
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "bank_ledger_checked");
        narrative.push_str(&format!(
            "\n【商行票號】你翻開三水總商行分格存摺櫃內的流水簿與驗印摺，各鎮分號防偽硃印歷歷在目。\n三水總商行通存通兌狀態：正常營運。\n你目前隨身持有資財：{} 鎂幣，憑商行信譽可在半島四鎮隨時交割結算。",
            magnesium
        ));
        push_refresh(conn, pid);
        return;
    }

    // 剪剃鋪精鋼摺疊剃刀 / 溫水洗頭木躺椅 (wt_barber_folding_razor / wt_barber_copper_chair / wt_cat_0630 / wt_cat_0631)
    if (obj.id == "wt_barber_folding_razor"
        || obj.id == "wt_barber_copper_chair"
        || obj.id == "wt_cat_0630"
        || obj.id == "wt_cat_0631"
        || obj.name.contains("剃刀")
        || obj.name.contains("洗頭")
        || obj.name.contains("理髮"))
        && (action == "Operate" || action == "Use" || action == "Sit" || action == "WarmHands")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 10).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::PLAYER_BODY, "barber_groomed");
        narrative.push_str("\n【修容舒爽】熱毛巾敷面溫潤身心，精鋼剃刀輕柔修整鬚角與碎髮，再經紫銅管溫水沖洗，整個人容光煥發、精神抖擻（體魄與元氣獲得恢復）。");
        push_refresh(conn, pid);
        return;
    }

    // 商算學舍烏木案與十三檔算盤 (wt_abacus_ebony_desk / wt_cat_0788)
    if (obj.id == "wt_abacus_ebony_desk"
        || obj.id == "wt_cat_0788"
        || obj.name.contains("算盤")
        || obj.name.contains("商算"))
        && (action == "Operate" || action == "Use" || action == "Read")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::PLAYER_BODY, "abacus_practiced");
        narrative.push_str("\n【珠算實習】你拂去烏木檔間微塵，凝神運指撥動竹骨算珠，隨口訣起落核算進出貨單，珠聲清脆如落玉，心神一片專注沉靜（元氣獲得恢復）。");
        push_refresh(conn, pid);
        return;
    }

    // 習武堂包革練習木刀 / 測紋卸力石 (wt_martial_padded_wooden_blade / wt_martial_force_stone / wt_cat_1015 / wt_cat_0846)
    if (obj.id == "wt_martial_padded_wooden_blade"
        || obj.id == "wt_martial_force_stone"
        || obj.id == "wt_cat_1015"
        || obj.id == "wt_cat_0846"
        || obj.name.contains("練習木刀")
        || obj.name.contains("卸力")
        || obj.name.contains("試招"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::PLAYER_BODY, "martial_drilled");
        narrative.push_str("\n【試招切磋】你緊縛牛皮護腕，手執裹軟革的練習木刀擺開架式，依口令與石靶連續對拆劈刺卸力，步法靈動沉穩，筋骨徹底舒展（體魄獲得增益）。");
        push_refresh(conn, pid);
        return;
    }

    // 攤位真交易（NPC↔攤販 / 玩家↔攤位物件）
    if (obj.id.starts_with("wt_stall_") || obj.id == "wt_street_roast_snack_cart")
        && matches!(action, "Taste" | "Use" | "Operate")
    {
        if let Some(outcome) = crate::server::action::trade::settle_live_vendor_npc_purchase(
            &obj.id,
            pid,
            player_room,
            now,
        ) {
            match outcome {
                crate::server::action::trade::VendorNpcPurchaseOutcome::Purchased {
                    item_name,
                    price_mg,
                    remaining_stock,
                    seller_mg_after,
                    ..
                } => {
                    if let Some(arc) = crate::store::get_store() {
                        let _ = arc.write().update_entity(pid, |e| {
                            e.vit = (e.vit + 2).min(100);
                            e.qi = (e.qi + 2).min(100);
                        });
                    }
                    if action == "Taste" {
                        narrative.push_str(&format!(
                            "\n【攤位真交易】你支付 {price_mg} 鎂向攤主買下「{item_name}」，當場品嚐溫養氣血並將餘份收妥入袋（攤主實收 {price_mg} 鎂、手頭共 {seller_mg_after} 鎂，攤上現貨尚餘 {remaining_stock} 份）。"
                        ));
                    } else {
                        *narrative = format!(
                            "你走向「{}」，支付 {price_mg} 鎂向攤主買下一份「{item_name}」收進背包，順道溫養氣血（攤主實收 {price_mg} 鎂、手頭共 {seller_mg_after} 鎂，攤上現貨尚餘 {remaining_stock} 份）。",
                            obj.name
                        );
                    }
                    push_refresh(conn, pid);
                    return;
                }
                crate::server::action::trade::VendorNpcPurchaseOutcome::OutOfStock { item_name } => {
                    *narrative = format!(
                        "你走向「{}」準備購買「{item_name}」，攤主抱歉地擦了擦汗道：「真不巧，最後一份「{item_name}」賣完啦，鍋裡見底，我正準備去補料，勞駕稍候再來！」",
                        obj.name
                    );
                    push_refresh(conn, pid);
                    return;
                }
                crate::server::action::trade::VendorNpcPurchaseOutcome::InsufficientFunds {
                    item_name,
                    price_mg,
                    buyer_mg,
                } => {
                    *narrative = format!(
                        "你走向「{}」想買一份「{item_name}」（每份 {price_mg} 鎂），摸了摸錢袋發現手頭僅有 {buyer_mg} 鎂，只好先作罷。",
                        obj.name
                    );
                    return;
                }
            }
        }
    }

    // 修傘補靴攤長凳與修具 (wt_repair_umbrella_bench / wt_cat_0896 / wt_cat_0888 / wt_cat_0890)
    if (obj.id == "wt_repair_umbrella_bench"
        || obj.id == "wt_cat_0896"
        || obj.id == "wt_cat_0888"
        || obj.id == "wt_cat_0890"
        || obj.name.contains("修傘")
        || obj.name.contains("補靴")
        || obj.name.contains("修補攤"))
        && (action == "Operate" || action == "Use")
    {
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "gear_repaired");
        narrative.push_str("\n【街坊巧藝】你坐在修造長凳旁，借熟鐵大剪與桐油膠漆仔細加固隨身防雨傘骨與行靴縫線，行裝完好紮實，不懼長途風雨。");
        push_refresh(conn, pid);
        return;
    }

    // 2.1) Merchant Ledger Counter (厚樟木商契掌事櫃台：商契蓋印與核銷)
    if (obj.id == "wt_merchant_ledger_counter" || obj.name.contains("商契掌事櫃台"))
        && (action == "Use" || action == "Operate")
    {
        let _ = db::add_magnesium(pid, 10);
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        narrative.push_str("\n【商契蓋印】你在厚樟木掌事櫃台前出示商行契據，值班執事仔細核對無誤後在契本上重重蓋下硃砂印鑑，完成商契核銷與物資調撥紀錄（領取 10 鎂核銷酬勞）。");
        push_refresh(conn, pid);
        return;
    }

    // 2.15) Caravan Transit Hub (商隊聯運票務櫃台：跨聚落快速聯運)
    if (obj.id == "wt_caravan_ticket_counter"
        || obj.name.contains("聯運班次")
        || obj.name.contains("車票"))
        && (action == "Use" || action == "Operate" || action == "Move")
    {
        const TRANSIT_STOPS: &[(&str, i32, i32, &str)] = &[
            ("文曜", 0, -1, "三水交匯的學風重鎮，車馬喧囂，書香與市井氣息交融"),
            ("老堰頭", 0, 125, "白塔北境清泉奔湧的堰頭樞紐，水汽氤氳，農人引水灌田"),
            ("半嶺鎮", 160, 210, "半島東北地勢起伏的丘陵山鎮，茶香撲鼻，山徑蜿蜒"),
            ("大圳頭", -180, -140, "西南灌溉渠網縱橫的沃土農莊，桑麻遍野，機杼聲聲"),
            ("南哨", 0, -220, "半島最南端風蝕礁岩林立的哨所防線，海風呼嘯，崗樓巍峨"),
            ("風舶角", -260, 160, "西北面臨怒海狂濤的深水海港，桅檣如林，鹽霧瀰漫"),
        ];

        let ent = db::get_entity(pid).ok().flatten();
        let (cur_x, cur_y) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0));
        let magnesium = ent.as_ref().map(|e| e.magnesium).unwrap_or(0);

        if magnesium < 5 {
            narrative.push_str("\n【商隊聯運客運】值守票務執事翻看名冊後溫言道：「跨聚落商隊聯運每程需 5 鎂乘車費，手頭盤纏尚且不夠，待備妥路費再來搭車吧。」");
            push_refresh(conn, pid);
            return;
        }

        // 尋找下一個目的地（若在某一聚落附近則循環前往下一站）
        let current_stop_idx = TRANSIT_STOPS.iter().position(|(_, x, y, _)| {
            (cur_x - *x).abs() <= 30 && (cur_y - *y).abs() <= 30
        });
        let next_idx = match current_stop_idx {
            Some(idx) => (idx + 1) % TRANSIT_STOPS.len(),
            None => 0,
        };
        let (dest_name, dest_x, dest_y, dest_desc) = TRANSIT_STOPS[next_idx];

        let _ = db::add_magnesium(pid, -5);
        let _ = db::set_entity_grid(pid, dest_x, dest_y);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().move_entity(pid, &format!("grid:{dest_x}:{dest_y}"));
        }
        super::super::handler::send_place_view(conn, pid);
        push_refresh(conn, pid);

        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("caravan_transit|{}|{},{}", dest_name, dest_x, dest_y),
        );

        narrative.push_str(&format!(
            "\n【商隊聯運客運】你在聯運櫃台點付 5 鎂車資，執事為你取來竹編乘輿牌。你登上厚棚雙輪大車，車伕一揚皮鞭，健碩的挽獸邁開大步，車輪在鋪石道與土徑上轆轆作響。行過半島層疊丘陵與原野，半個時辰後順利抵達【{}】（{}）！",
            dest_name, dest_desc
        ));
        return;
    }

fn clean_catalog_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 3 <= chars.len() {
            let prefix: String = chars[i..i + 3].iter().collect();
            if prefix == "WT-" || prefix == "WS-" {
                let mut j = i + 3;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                if j > i + 3 {
                    while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
                        j += 1;
                    }
                    i = j;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

    if obj.id == "wt_guild_commission_board"
        || obj.id == "wt_hall_bulletin_board"
        || obj.id == "wt_cat_0795"
        || obj.id == "wt_cat_0794"
        || obj.name.contains("委託")
        || obj.name.contains("告示")
        || obj.name.contains("公佈欄")
    {
        let room_name = resolve_room_name(player_room);
        let scarce_opt = crate::game::object_ecology::scarcest_item_for_settlement(&room_name);
        let supply_demand_report = crate::game::object_ecology::format_settlement_supply_demand_report(&room_name);

        let player_inv = db::get_entity(pid).ok().flatten().map(|player_ent| {
            db::get_inventory(&player_ent.inventory, player_ent.vit)
        });

        let scripts = crate::game::peninsula_scripts::scripts_for_room(&room_name);
        let script_idx = ((now.unsigned_abs() as usize) / 60) % scripts.len().max(1);
        let active_script = scripts.get(script_idx);

        // 檢查玩家行囊是否備有本地急缺物資（僅在玩家主動交付/操作時觸發）
        let delivered_item = if action == "Use" || action == "Operate" || action == "Deliver" {
            player_inv.as_ref().and_then(|inv| {
                // 優先比對：本地最緊缺的供需短缺物產
                if let Some((s_id, s_name, _cur_stock, _tgt_stock, price)) = &scarce_opt {
                    if let Some(_it) = inv.items.iter().find(|i| &i.item_id == s_id && i.qty > 0) {
                        return Some((s_id.clone(), s_name.clone(), *price));
                    }
                }
                // 次要比對：當前街坊公議腳本之關鍵物產（key_objects 明確提及者）
                if let Some(sc) = active_script {
                    let clean_keys = clean_catalog_codes(sc.key_objects);
                    for it in &inv.items {
                        if it.qty <= 0 { continue; }
                        let item_name = db::get_item_def(&it.item_id)
                            .map(|d| d.name)
                            .or_else(|| {
                                crate::game::peninsula_catalog::find_catalog_entry(&it.item_id)
                                    .map(|c| c.name.to_string())
                            })
                            .filter(|n| !n.trim().is_empty())
                            .unwrap_or_else(|| it.item_id.clone());
                        if clean_keys.contains(&item_name) || sc.key_objects.contains(&it.item_id) {
                            let base_price = crate::world::materials::trade_price_anchor_mg(&it.item_id)
                                .or_else(|| {
                                    crate::game::peninsula_catalog::find_catalog_entry(&it.item_id)
                                        .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
                                })
                                .unwrap_or(20);
                            return Some((it.item_id.clone(), item_name, base_price));
                        }
                    }
                }
                None
            })
        } else {
            None
        };

        if let Some((item_id, item_name, unit_price)) = delivered_item {
            if db::remove_from_inventory(pid, &item_id, 1).is_ok() {
                let bounty_mg = unit_price + 10;
                let _ = db::add_magnesium(pid, bounty_mg);
                let _ = crate::game::object_ecology::record_settlement_market_trade(
                    &room_name,
                    &item_id,
                    unit_price,
                    1,
                    "委託交割",
                );
                let _ = event::append(
                    now,
                    pid,
                    types::SCENE_OBJECT_ACTION,
                    &format!("{}|commission_delivery|{}", obj.id, item_id),
                );
                let (imprint, newly_acquired) = crate::pg::living_world::record_mentor_imprint_progress(
                    pid,
                    "分家巷溫掌事",
                    "百工衡準訣",
                    "熟稔半島各坊物材成色與發力分寸，運器施作更為穩健",
                    now,
                );
                let imprint_note = if imprint.interaction_count == 1 {
                    format!(
                        "\n【百工指點】溫掌事見你交割物資成色地道，順口指點了你幾句《{}》（{}）的發力分寸。",
                        imprint.imprint_title,
                        imprint.imprint_prose
                    )
                } else if newly_acquired {
                    let _ = db::add_to_inventory(pid, "blank_manual", 1);
                    let mut hasher = 0xcbf29ce484222325u64;
                    for b in format!("{}_{}", imprint.mentor_name, imprint.imprint_title).bytes() {
                        hasher ^= b as u64;
                        hasher = hasher.wrapping_mul(0x100000001b3);
                    }
                    let manual_hash = format!("{:016x}", hasher);
                    let formula_word_ids = vec!["zhun".to_string(), "ding".to_string(), "he_tai".to_string()];
                    let formula_word_chars = vec!["準".to_string(), "定".to_string(), "合".to_string()];
                    let manual_rec = crate::pg::living_world::TranscribedManualRecord {
                        manual_hash: manual_hash.clone(),
                        author_id: imprint.mentor_name.clone(),
                        author_name: "溫掌事".to_string(),
                        formal_name: imprint.imprint_title.clone(),
                        slot_label: "守勢".to_string(),
                        carrier_code: "tool".to_string(),
                        word_ids: formula_word_ids.clone(),
                        word_chars: formula_word_chars.clone(),
                        origin_basis: imprint.imprint_prose.clone(),
                        created_at: now,
                    };
                    crate::pg::living_world::save_transcribed_manual(&manual_rec);
                    let manual_item_id = format!("manual_{manual_hash}");
                    let _ = db::add_to_inventory(pid, &manual_item_id, 1);

                    let word_id_refs: Vec<&str> = formula_word_ids.iter().map(|s| s.as_str()).collect();
                    crate::server::handler::inventory::grant_words_to_player_casket(pid, &word_id_refs);

                    format!(
                        "\n【百工師承・心領神會】溫掌事端詳物材成色，讚許點頭，贈予你一卷親筆手書的《{}》手札與空白手札，並手把手點撥你辨識各坊物性與運力分寸——領悟師承烙印【{}】！（{}）",
                        imprint.imprint_title,
                        imprint.imprint_title,
                        imprint.imprint_prose
                    )
                } else {
                    format!(
                        "\n【百工掌事指點】溫掌事撫鬚稱善：「物材質地極佳，解了工坊燃眉之急！」其言簡意賅的指點令你對【{}】的體會又深了一層。",
                        imprint.imprint_title
                    )
                };
                narrative.push_str(&format!(
                    "\n【本坊急缺物資交割完成】你依行會告示板之急徵令，自隨身行囊交付一份急缺的【{item_name}】供本地工坊應急！\n溫掌事點收後蓋印銷單，當場核發交割懸賞 {bounty_mg} 鎂酬勞，本坊物資緊缺之勢隨之舒緩。{imprint_note}"
                ));
                push_refresh(conn, pid);
                return;
            }
        }

        // 未攜帶急缺物資：展示本坊街坊公議與即時供需短缺報告
        let script_section = if let Some(sc) = active_script {
            let clean_objects = clean_catalog_codes(sc.key_objects);
            format!(
                "【街坊公議與物資徵集・{}】\n・主事當家：{}\n・徵集物產：{}\n・事由始末：{}\n・協力途徑：{}",
                sc.title, sc.location_and_npc, clean_objects, sc.trigger_and_causal_chain, sc.player_actions
            )
        } else {
            "【街區公務告示】目前各坊市當家正按各自班表出工，暫無加急公務告示。".to_string()
        };

        let shortage_notice = if let Some((_, scarce_name, _cur_stock, _tgt_stock, _unit_price)) = &scarce_opt {
            format!("【本坊急徵物資】目前急徵「{}」，行會特設專項酬勞廣納應急物產。\n{}", scarce_name, supply_demand_report)
        } else {
            format!("【本坊急徵物資】本地物產暫無嚴重缺口，常備物資平穩運轉中。\n{}", supply_demand_report)
        };

        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        narrative.push_str(&format!("\n{}\n\n{}", script_section, shortage_notice));
        push_refresh(conn, pid);
        return;
    }
    // 2.5) Wenyao Special World Objects
    if (obj.id == "wt_bridge_sundial_pillar" || obj.name.contains("日晷") || obj.name.contains("指路石柱"))
        && (action == "Read" || action == "Use")
    {
        let hour = current_game_hour(&conn.cfg);
        let (period, shadow_desc) = match hour {
            23 | 0..=3 => ("深宵", "此時夜闌更深，日晷無日影，青石底座刻痕藉鐘樓燈火依稀可辨"),
            4..=6 => ("晨曉", "旭日初昇，晨霞薄染，細長晷影斜斜投向正西方「西門聯運」刻度"),
            7..=10 => ("晝前", "朝陽高照，長街人聲初起，晷影清晰轉向西北方「三水總商行」刻度"),
            11..=14 => ("正晝", "日輪當空正中，晷針短影端正直指正北「三水口北街」刻度"),
            15..=18 => ("黃昏", "午後陽光漸暖，車馬絡繹，晷影斜長投向東南「東橋旅社與分家巷」刻度"),
            _ => ("入夜", "暮色四合，燈火初上，晷影隨殘陽斂入石台，正對夜市人流"),
        };
        narrative.push_str(&format!(
            "\n【日晷報時】當前半島時間為【{} {:02}:00 左右】。{}。\n石柱四面銘文刻印分明：北指「三水口北街、三水總商行與大食堂」、東指「講堂街文曜學院與東橋旅社」、南指「三水口南街、公議堂與分家巷百工」、西指「傳訊鐘樓與西門聯運總站」。",
            period, hour, shadow_desc
        ));
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_street_news_board" || obj.name.contains("閱報") || obj.name.contains("旬報"))
        && (action == "Read" || action == "Use")
    {
        let room_name = resolve_room_name(player_room);
        let scripts = crate::game::peninsula_scripts::scripts_for_room(&room_name);
        let script_idx = ((now.unsigned_abs() as usize) / 60) % scripts.len().max(1);
        let script_preview = if let Some(sc) = scripts.get(script_idx) {
            format!("\n【街區見聞・{}】{}", sc.title, sc.trigger_and_causal_chain)
        } else {
            String::new()
        };
        narrative.push_str(&format!(
            "\n【《半島旬報》當期要聞】\n一、【商貿要聞】三水口三水總商行全面發行新一期分格存摺，各鎮分號匯兌暢通，商旅皆便。\n二、【民生物候】老堰頭水輪石磨引水石堰順利通水，大圳頭新麥開磨，各坊食堂米漿香氣四溢。\n三、【百工巧藝】分家巷岑氏鍛造與柯氏木作工坊聯袂發表新式精鍛獵刀與防潮儲物匣，廣獲各鎮獵隊好評。{script_preview}"
        ));
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_street_grindstone_trough" || obj.name.contains("磨刀石") || obj.name.contains("水槽與磨刀石") || obj.name.contains("磨刀槽"))
        && (action == "Use" || action == "Operate")
    {
        let vessel_target = if let Ok(Some(ch)) = db::get_entity(pid) {
            let mut found = None;
            if !ch.equipment_slots.is_empty() {
                if let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(&ch.equipment_slots) {
                    let check_slots = ["weapon", "hold_r", "main_hand", "hold", "tool", "hold_l"];
                    for s in check_slots {
                        if let Some(id) = slots.get(s).filter(|id| !id.is_empty()) {
                            let name = db::get_item_def(id).map(|d| d.name).unwrap_or_else(|| id.clone());
                            if crate::pg::living_world::is_bladed_vessel_item(id, &name) {
                                found = Some((id.clone(), name));
                                break;
                            }
                        }
                    }
                }
            }
            if found.is_none() {
                let inv_items: Vec<serde_json::Value> = serde_json::from_str(&ch.inventory).unwrap_or_default();
                for it in inv_items {
                    let id = it.get("item_id").or_else(|| it.get("id")).and_then(|v| v.as_str()).unwrap_or_default();
                    if id.is_empty() { continue; }
                    let name = db::get_item_def(id).map(|d| d.name).unwrap_or_else(|| id.to_string());
                    if crate::pg::living_world::is_bladed_vessel_item(id, &name) {
                        found = Some((id.to_string(), name));
                        break;
                    }
                }
            }
            found
        } else {
            None
        };

        if let Some((item_id, item_name)) = vessel_target {
            match crate::pg::living_world::sharpen_vessel_at_grindstone(pid, &item_id, now) {
                Ok(state) => {
                    let _ = event::append(now, pid, types::PLAYER_BODY, "blade_sharpened");
                    narrative.push_str(&format!(
                        "\n【利刃磨礪・鋒芒重現】你舀起青石水槽中的清冽井水淋在磨刀石上，將「{}」伏貼石面細細推磨。水珠混著青灰石漿滑落，刃口寒芒流轉、重歸「{}」！（人器磨合：{} —— {}）",
                        item_name,
                        state.condition.label(),
                        state.familiarity().label(),
                        state.familiarity().feel_prose()
                    ));
                }
                Err(msg) => {
                    let clean_msg = msg.trim_end_matches('。');
                    narrative.push_str(&format!(
                        "\n【利刃磨礪】推磨數下後，你發現「{}」{}。此類捲刃待修重損，街邊尋常青石磨刀槽難以推平，須赴鍛造工坊藉由鍛爐⚒︎火候與鐵砧輕敲校直方能修整復原。",
                        item_name, clean_msg
                    ));
                }
            }
        } else {
            narrative.push_str("\n【利刃磨礪】你舀起青石水槽中的清冽井水淋在磨刀石上，但隨身並未持握或攜帶刀劍鋒刃器具，只得任清水自青石表面滑落。");
        }
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_street_platform_scale" || obj.name.contains("地秤") || obj.name.contains("過磅"))
        && (action == "Operate" || action == "Use")
    {
        let (item_count, total_weight_jin) = if let Ok(Some(ch)) = db::get_entity(pid) {
            let items: Vec<serde_json::Value> = serde_json::from_str(&ch.inventory).unwrap_or_default();
            let count: usize = items.iter().filter_map(|v| v.get("qty").and_then(|q| q.as_u64())).sum::<u64>() as usize;
            let mg_weight = ch.magnesium / 50;
            let base_jin = 110 + (count as i32 * 3) + mg_weight;
            (count, base_jin)
        } else {
            (0, 110)
        };
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "scale_weighed");
        narrative.push_str(&format!(
            "\n【地秤過磅】你邁步踏上西門過磅生鐵地秤，沉重秤桿在熟鐵秤砣滑動間發出沉實金屬聲。\n過磅司秤報數：隨身攜帶物件共 {item_count} 件，合算隨身行囊與衣裝，過磅總重為【{total_weight_jin} 斤四兩】。秤台承重沉穩，符合西門聯運車隊標準裝載規範。"
        ));
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_caravan_water_trough"
        || obj.id == "wt_alley_stone_well"
        || (obj.name.contains("水槽")
            && !obj.name.contains("熱水槽")
            && !obj.name.contains("溫石")
            && !obj.name.contains("淬火")
            && obj.id != "wt_inn_bath_Preview")
        || obj.name.contains("公井")
        || obj.name.contains("汲水木桶"))
        && (action == "Use" || action == "Wash" || action == "Drink" || action == "Operate")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 8).min(100);
                e.qi = (e.qi + 8).min(100);
            });
        }
        let _ = event::append(now, pid, types::PLAYER_BODY, "well_washed");
        narrative.push_str("\n【清泉洗漱】你搖動轆轤打起半桶深井甘泉，清冽泉水撲面而來，洗去了一身風塵與倦意，渾身毛孔透徹舒泰（體魄與元氣獲得恢復）。");
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_deed_cadastral_map_desk"
        || obj.id == "wt_deed_household_registry"
        || obj.name.contains("地籍")
        || obj.name.contains("總譜")
        || obj.name.contains("戶籍"))
        && (action == "Read" || action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "registry_checked");
        narrative.push_str("\n【查閱地籍戶譜】你仔細翻閱公議堂案几上的地籍總圖與落籍戶譜。各坊街屋坐標界石標註分明，田畝產權清晰可稽，令人對文曜建置井然肅然起敬（元氣獲得恢復）。");
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_obs_tide_weather_table"
        || obj.id == "wt_obs_brass_telescope_rack"
        || obj.name.contains("潮汐")
        || obj.name.contains("望遠銅鏡"))
        && (action == "Read" || action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "observation_studied");
        narrative.push_str("\n【觀測天象潮汐】你凝神查閱四時潮汐氣象推算案並透過雙筒望遠銅鏡俯瞰海天交界。風雲變幻與水運起落盡收眼底，胸懷頓時為之開闊（元氣獲得恢復）。");
        push_refresh(conn, pid);
        return;
    }
    // 2.6) Wenyao Interactive Riverbank, Farming & Lighting
    if (obj.id == "wt_reg_wenyao_riverbank" || obj.name.contains("平底竹筏") || obj.name.contains("繫船青石樁"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "bamboo_raft_poled");
        let room_name = resolve_room_name(player_room);
        if room_name.contains("風舶角") || room_name.contains("海") || room_name.contains("港") {
            narrative.push_str("\n【乘筏觀潮】你解開繫在石樁上的粗纜，平底木筏隨著港灣潮汐輕輕起伏，海風獵獵拂面，遠眺海天一色，心神頓覺開闊澄明（元氣獲得恢復）。");
        } else {
            narrative.push_str("\n【乘筏信步】你解開繫在青石樁上的桐油麻繩，手持長竹篙在石階輕撐借力，平底竹筏滑入三水川清澈碧流。沿岸水草豐美、水鳥掠波，心神隨流水漸入澄澈（元氣獲得恢復）。");
        }
        if !db::inventory_has_item(pid, "crystal_water_bing") {
            let _ = db::add_to_inventory(pid, "crystal_water_bing", 1);
            narrative.push_str("\n【觀境凝晶】竹篙撥開澄澈碧流之際，水波映日，一滴蘊含水靈意蘊的精華於掌心凝化為【水源・丙品詞元結晶】。");
        }
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_reg_wenyao_garden"
        || obj.id == "wt_reg_wenyao_courtyard"
        || obj.name.contains("藥圃")
        || obj.name.contains("盆栽")
        || obj.name.contains("引水筧")
        || obj.name.contains("山茶"))
        && (action == "Water" || action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "plants_watered");
        narrative.push_str("\n【細心灌溉】清涼甘泉緩緩注入泥土，泥土與草木散發出質樸而清新的生機芬芳。靜心照拂草木之間，心神漸趨安寧（元氣獲得恢復）。");
        if obj.name.contains("藥圃") || obj.id == "wt_reg_wenyao_garden" {
            let _ = db::add_to_inventory(pid, "mat_foraged_herb", 1);
            narrative.push_str("\n【草木採收】你在藥圃中細心摘取了一份清香四溢的「野生採集草藥」收入行囊。");
        }
        push_refresh(conn, pid);
        return;
    }

    if (action == "Use" || action == "Operate")
        && (obj.id == "wt_home_shell_lamp" || obj.name.contains("台燈"))
        && !obj.id.contains("signpost")
        && !obj.name.contains("路燈")
        && !obj.name.contains("路標")
        && obj.id != "wt_reg_wenyao_home"
        && !obj.name.contains("大床")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 5).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "lamp_dimmed");
        narrative.push_str("\n【旋燈調光】你輕輕旋轉床頭夜光貝母台燈的烏木旋蓋，柔和溫潤的珠白光芒隨之微調，滿室皆是安恬暖意（心神漸入安恬）。");
        push_refresh(conn, pid);
        return;
    }

    // 2.7) Wenyao Academy, Study, Printing, Medicine & Riverbank Interactivity
    if (obj.id == "wt_study_desk_inkstone" || obj.name.contains("沉水烏木書案") || obj.name.contains("端硯"))
        && (action == "Operate" || action == "Use" || action == "Read")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "study_desk_inkstone_used");
        narrative.push_str("\n【研墨展卷】你取墨錠在端硯清泉中徐徐研磨，墨香四溢。隨後提筆蘸墨在宣紙上信手書寫，筆意沉著，心神豁然澄澈（元氣獲得恢復）。");
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_study_bamboo_bookshelf" || obj.name.contains("竹編書簍") || obj.name.contains("古籍架"))
        && (action == "Operate" || action == "Use" || action == "Read")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "study_bamboo_bookshelf_used");
        narrative.push_str("\n【檢閱典籍】你自架上抽出一冊線裝古籍翻閱，紙質柔韌吸墨，墨香沁人心脾。先賢微言大義躍然紙上，令人心曠神怡（元氣獲得恢復）。");
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_astro_brass_armillary" || obj.name.contains("渾天星軌") || obj.name.contains("推步儀"))
        && (action == "Operate" || action == "Use" || action == "Read")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "astro_brass_armillary_used");
        narrative.push_str("\n【星軌推步】你推動星軌儀的精銅外環，齒輪與軸承咬合平順，星軌刻度隨之流轉對齊今日天象，心中豁然開朗（元氣獲得恢復）。");
        if let Ok((_mut_id, _mut_name, mut_narrative)) =
            crate::game::object_ecology::mutate_finished_item(pid, "auto", &obj.name, now)
        {
            narrative.push_str(&format!("\n【星軌氣脈共鳴異變】在星環轉動與星光垂照之下，{}", mut_narrative));
        }
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_math_abacus_desk" || obj.name.contains("算盤") || obj.name.contains("商算習題案"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "math_abacus_used");
        narrative.push_str("\n【撥珠速算】你信手撥動黑檀算珠，珠落聲清脆利落如落玉盤。心中速算推演，思維條理頓時無比清晰（元氣獲得恢復）。");
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_print_movable_type" || obj.name.contains("活字排版") || obj.name.contains("墨刷案"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "movable_type_printed");
        narrative.push_str("\n【排版試印】你拿起棕櫚墨刷在排版活字上勻勻刷上一層松煙墨，覆上宣紙輕輕滾壓，字跡頓時端正顯映於紙上（元氣獲得恢復）。");
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_paper_bamboo_vat" || obj.name.contains("抄紙石槽") || obj.name.contains("焙紙熱牆"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "paper_bamboo_vat_used");
        narrative.push_str("\n【蕩料抄紙】你雙手執竹簾浸入石槽，平順蕩料抄起一層細密纖維，揭紙貼於熱牆烘乾，紙面瑩白平滑，工藝之妙令人讚歎（元氣獲得恢復）。");
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_herb_drying_kiln" || obj.name.contains("百草焙藥爐") || obj.name.contains("焙藥爐"))
        && (action == "Operate" || action == "Use" || action == "WarmHands")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 10).min(100);
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "herb_kiln_tended");
        narrative.push_str("\n【焙藥調溫】你撥動紅泥爐膛的通氣瓦片微調火候，熱氣蒸騰間草木本質被徐徐焙透，濃郁藥香透體而入，體魄與精神皆為之一振（體魄與元氣獲得恢復）。");
        if let Ok((_item_id, _item_name, _is_mut, craft_narrative)) =
            crate::game::object_ecology::craft_from_raw_material(pid, "auto", &obj.name, now)
        {
            narrative.push_str(&format!("\n【工造煉製】在伺火焙藥之際，{}", craft_narrative));
        }
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_herb_bamboo_sieve" || obj.name.contains("切藥鍘刀") || obj.name.contains("晾藥篩架"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "herb_sieve_cut");
        narrative.push_str("\n【操刀切藥】你手握切藥鍘刀木柄壓下，利刃切開晾乾的草藥根莖，發出清脆嚓響，藥香濃烈撲面，筋骨隨之舒展（元氣獲得恢復）。");
        if let Ok((_item_id, _item_name, _is_mut, craft_narrative)) =
            crate::game::object_ecology::craft_from_raw_material(pid, "auto", &obj.name, now)
        {
            narrative.push_str(&format!("\n【工造煉製】在操刀切藥之際，{}", craft_narrative));
        }
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_river_stone_steps" || obj.name.contains("汲水埠階") || obj.name.contains("竹編魚簍"))
        && (action == "Operate" || action == "Use" || action == "Wash" || action == "Rest" || action == "Meditate")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 5).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "river_steps_washed");
        narrative.push_str("\n【埠階掬泉】你踏上青石汲水埠階，俯身掬起清涼甘洌的川流活水洗滌面容，暑氣盡散，神清氣爽（體魄與元氣獲得恢復）。");
        let _ = db::add_to_inventory(pid, "mat_iron_scrap", 1);
        narrative.push_str("\n【岸畔淘料】你在埠階水邊清淤淘洗，順道自泥沙礫石中淘獲一塊堅實的【回收熟鐵料塊】收入行囊。");
        if !db::inventory_has_item(pid, "crystal_water_bing") {
            let _ = db::add_to_inventory(pid, "crystal_water_bing", 1);
            narrative.push_str("\n【觀境凝晶】掬泉浣洗之時，碧波漣漪激盪出一絲極純粹的水靈精萃，於指尖凝結為【水源・丙品詞元結晶】。");
        }
        if action == "Rest" || action == "Meditate" {
            append_somatic_awareness_note(pid, now, narrative);
        }
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_bamboo_tea_table" || obj.name.contains("竹林青石茶案") || obj.name.contains("石凳"))
        && (action == "Operate" || action == "Use" || action == "Rest" || action == "Taste" || action == "Meditate")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "bamboo_tea_rested");
        narrative.push_str("\n【竹苑啜茗】你在微涼青石凳上端坐歇息，以竹筧甘泉瀹茶，修竹沙沙入盞，頓覺身心放鬆，塵慮盡消（元氣獲得恢復）。");
        if action == "Rest" || action == "Taste" || action == "Meditate" {
            append_somatic_awareness_note(pid, now, narrative);
        }
        push_refresh(conn, pid);
        return;
    }

    if obj.id == "wt_sawmill_woodchip_bin" && action == "Gather" {
        let _ = db::add_to_inventory(pid, "mat_kindling_bundle", 1);
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "sawmill_woodchip_gathered");
        narrative.push_str("\n\n【拾取刨花引柴】你順手將一束乾燥易燃的【乾燥柴枝束】妥善收入行囊。");
        push_refresh(conn, pid);
        return;
    }

    if (obj.id == "wt_clinic_herb_cabinet" && action == "Use")
        || (obj.id == "wt_clinic_decoction_stove" && action == "WarmHands")
        || ((obj.id == "wt_clinic_acupoint_bed" || obj.id == "wt_arena_rest_bench") && (action == "Rest" || action == "Meditate"))
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 15).min(100);
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "outer_settlement_rested");
        narrative.push_str("\n\n【調息溫養】草木溫潤之氣順著經絡緩緩化開，周身疲乏消散，體魄與心神皆獲調養。");
        append_somatic_awareness_note(pid, now, narrative);
        push_refresh(conn, pid);
        return;
    }
    // --- 文曜外環工坊、碼頭、窯廠、紙墨印造、倉庫等重工勞動器物 ---
    if matches!(
        obj.id.as_str(),
        "wt_outer_wharf_crane"
            | "wt_obj_wharf_crane"
            | "wt_outer_ship_ribs"
            | "wt_obj_ship_ribs_pitch"
            | "wt_outer_net_rack"
            | "wt_obj_drying_nets_trough"
            | "wt_obj_rope_twist_wheel"
            | "wt_outer_dock_caulk"
            | "wt_outer_dredge_winch"
            | "wt_obj_patrol_skiff_gear"
            | "wt_outer_reed_rack"
            | "wt_obj_timber_reed_stacks"
            | "wt_outer_salt_vat"
            | "wt_obj_iron_salt_stacks"
            | "wt_outer_paper_vat"
            | "wt_obj_paper_making_screen"
            | "wt_obj_pulp_vat"
            | "wt_outer_ink_mortar"
            | "wt_obj_ink_soot_mortar"
            | "wt_obj_ink_mould_drying"
            | "wt_outer_type_tray"
            | "wt_obj_movable_type_case"
            | "wt_outer_bind_press"
            | "wt_obj_printing_press"
            | "wt_obj_book_binding_press"
            | "wt_outer_trip_hammer"
            | "wt_obj_trip_hammer_trough"
            | "wt_obj_bellows_furnace"
            | "wt_forge_furnace_anvil"
            | "wt_forge_quench_trough"
            | "wt_wood_planing_bench"
            | "wt_outer_water_saw"
            | "wt_obj_waterwheel_sawframe"
            | "wt_outer_stone_saw"
            | "wt_obj_stone_gang_saw"
            | "wt_outer_bronze_crucible"
            | "wt_obj_bronze_crucible"
            | "wt_outer_charcoal_kiln"
            | "wt_obj_charcoal_kiln"
            | "wt_outer_brick_kiln"
            | "wt_obj_long_kiln_firebox"
            | "wt_outer_canvas_loom"
            | "wt_obj_sail_canvas_bench"
            | "wt_obj_loom_dye_vats"
            | "wt_outer_fur_press"
            | "wt_obj_fur_baling_press"
            | "wt_outer_wagon_jack"
            | "wt_obj_wagon_jack_wheel"
            | "wt_outer_salt_iron_scale"
            | "wt_obj_steelyard_cart"
    ) || (action == "Operate" || action == "Use" || action == "Caulking" || action == "Pound" || action == "PickType" || action == "RollInk" || action == "Stitch" || action == "Quench") && (
        obj.name.contains("滑輪吊架")
            || obj.name.contains("龍骨肋架")
            || obj.name.contains("曬網木架")
            || obj.name.contains("手搖木輪")
            || obj.name.contains("長柄鐵斗")
            || obj.name.contains("紮排毛竹")
            || obj.name.contains("抄紙青竹簾")
            || obj.name.contains("搗墨鐵臼")
            || obj.name.contains("字盤排架")
            || obj.name.contains("壓印機")
            || obj.name.contains("木夾壓平機")
            || obj.name.contains("水力聯動重鐵錘")
            || obj.name.contains("活塞木風箱")
            || obj.name.contains("鍛台")
            || obj.name.contains("鍛造")
            || obj.name.contains("鐵砧")
            || obj.name.contains("鍛爐")
            || obj.name.contains("淬火槽")
            || obj.name.contains("木作台")
            || obj.name.contains("大水車連軸排鋸")
            || obj.name.contains("框鋸")
            || obj.name.contains("悶炭窯")
            || obj.name.contains("斜坡龍窯")
            || obj.name.contains("帆布高織機")
            || obj.name.contains("壓捆木機")
            || obj.name.contains("千斤頂")
            || obj.name.contains("大磅秤")
    ) {
        let ent = db::get_entity(pid).ok().flatten();
        let vit = ent.as_ref().map(|e| e.vit).unwrap_or(10);
        if vit <= 1 {
            narrative.push_str("\n【體力不支】你氣喘吁吁，四肢酸痛無力，需稍作停歇恢復體力。");
            return;
        }
        let (cost_vit, wage, labor_desc) = match obj.id.as_str() {
            "wt_outer_wharf_crane" | "wt_obj_wharf_crane" => (
                4,
                10,
                "你緊握滑輪牽引粗麻纜繩，踩穩跳板協同碼頭苦力將重型貨箱自駁船起吊至條石駁岸，臂膀筋骨緊繃，動作沉穩洗練。",
            ),
            "wt_outer_ship_ribs" | "wt_obj_ship_ribs_pitch" | "wt_outer_dock_caulk" => (
                5,
                12,
                "你手持嵌縫刀蘸取滾沸的熟桐油松香瀝青，專注填塞船底龍骨與水密隔艙縫隙，膠泥密實嚴整，滴水不漏。",
            ),
            "wt_outer_net_rack" | "wt_obj_drying_nets_trough" | "wt_obj_rope_twist_wheel" => (
                3,
                8,
                "你搖動三聯絞繩輪，三股浸油苧麻絲在均勻轉動中緊密咬合絞成粗壯縴繩，手掌磨出厚繭，繩身堅韌沉實。",
            ),
            "wt_outer_dredge_winch" | "wt_obj_patrol_skiff_gear" => (
                4,
                9,
                "你推動長柄鋼齒鐵斗探入碼頭下沿清除河道淤泥暗沙，將沈積泥沙整車推卸至駁岸外，航道重獲通暢清澈。",
            ),
            "wt_outer_reed_rack" | "wt_obj_timber_reed_stacks" => (
                4,
                8,
                "你搬運剛下排筏的三年生毛竹與重型巨杉圓木，按營造尺寸整齊排垛並蓋好防潮油布，通風堆垛井然有序。",
            ),
            "wt_outer_salt_vat" | "wt_obj_iron_salt_stacks" => (
                4,
                9,
                "你協助鹽倉掌事挑運粗鹽大包並分層以十字交錯法壓緊防潮油布包，粗重勞作下汗如雨下，倉底乾燥踏實。",
            ),
            "wt_outer_paper_vat" | "wt_obj_paper_making_screen" | "wt_obj_pulp_vat" => (
                3,
                8,
                "你雙手平托青竹抄紙簾深入石槽，腕力輕巧一蕩，均勻濾去白漿殘水，簾面結出一層柔韌勻薄的純淨宣紙纖維。",
            ),
            "wt_outer_ink_mortar" | "wt_obj_ink_soot_mortar" | "wt_obj_ink_mould_drying" => (
                5,
                11,
                "你提起重鐵杵在大臼中千百次奮力舂搗皮膠與上等松煙墨坯，筋道十足的墨泥在反覆捶打下油黑鋥亮、泛出紫光。",
            ),
            "wt_outer_type_tray" | "wt_obj_movable_type_case" => (
                3,
                7,
                "你手持字夾按韻律偏旁在字盤架中快速揀字入盤，推動羊毛油墨滾筒在字版上反覆滾勻松煙油墨，字跡黑潤分明。",
            ),
            "wt_outer_bind_press" | "wt_obj_printing_press" | "wt_obj_book_binding_press" => (
                4,
                9,
                "你奮力扳動螺旋壓印機手柄，重板均勻受力壓覆紙面，揭開時字跡黑亮深透；隨後以拋光骨針鎖定線裝書脊，整飭端莊。",
            ),
            "wt_outer_trip_hammer" | "wt_obj_trip_hammer_trough" | "wt_obj_bellows_furnace" | "wt_forge_furnace_anvil" => (
                5,
                12,
                "你手持長鍛鉗將燒紅的重型鋼錠送入鍛台之下，隨著重錘轟然夯擊，火星四射間精鋼雜質盡去，器型雄厚。",
            ),
            "wt_forge_quench_trough" => (
                4,
                11,
                "你夾起通紅熾熱的刀坯猛然浸入導能精鋼冷卻淬火槽，白霧蒸騰翻湧、嗤啦作響，寒泉冷卻間器紋緊密收束。",
            ),
            "wt_outer_water_saw" | "wt_obj_waterwheel_sawframe" | "wt_wood_planing_bench" => (
                4,
                10,
                "你操持導能木作器械推刨剖木，木屑紛飛間平直厚重的良木板材與榫卯構件依次成型。",
            ),
            "wt_outer_stone_saw" | "wt_obj_stone_gang_saw" => (
                5,
                11,
                "你引導細水與金剛砂均勻注入玄武岩石縫，握緊鋼鑿重錘修平開山條石斷面，石面平直如砥，嚴絲合縫。",
            ),
            "wt_outer_bronze_crucible" | "wt_obj_bronze_crucible" => (
                5,
                12,
                "你戴上防護牛皮手套平穩夾持石墨坩堝，將金紅透亮的銅錫合金熔液順著澆口緩緩注入翻砂模具，成型精準大氣。",
            ),
            "wt_outer_charcoal_kiln" | "wt_obj_charcoal_kiln" => (
                4,
                9,
                "你協助封堵黏土炭窯窯門排煙口，掌控悶燒火候，出窯後將敲擊如金石的堅密無煙硬木白炭裝箱碼垛。",
            ),
            "wt_outer_brick_kiln" | "wt_obj_long_kiln_firebox" => (
                5,
                10,
                "你頂著炙熱火浪向斜坡龍窯火膛添送耐火薪柴，密切觀察觀火孔火焰呈白熾色澤，磚瓦坯體在高溫下逐步瓷化堅固。",
            ),
            "wt_outer_canvas_loom" | "wt_obj_sail_canvas_bench" | "wt_obj_loom_dye_vats" => (
                4,
                8,
                "你腳踏高織機踏板投梭引緯，重密粗麻經緯緊扣交織，隨後將新織防暴帆布下缸浸染群青，布料抗撕拉力極佳。",
            ),
            "wt_outer_fur_press" | "wt_obj_fur_baling_press" => (
                4,
                9,
                "你轉動木製壓捆機螺桿，將大批熟化好的優等原牛皮平整壓緊並以浸油麻繩打結封固，捆包緊實方正。",
            ),
            "wt_outer_wagon_jack" | "wt_obj_wagon_jack_wheel" => (
                4,
                10,
                "你旋動重型千斤頂將雙輪貨車大架頂升懸空，手持大鐵錘校正包鐵車軸並塗抹潤滑桐油脂，車輪轉動平穩無阻。",
            ),
            _ => (
                4,
                8,
                "你熟練操縱磅秤與搬運墊木，清點度量進出大宗貨物斤兩，動作麻利嚴謹，帳物分明。",
            ),
        };
        let actual_cost = if vit <= 10 { 1 } else { cost_vit };
        let _ = db::update_vit(pid, (vit - actual_cost).max(2));
        let _ = db::transfer_magnesium("world_market", pid, wage).or_else(|_| db::add_magnesium(pid, wage));
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        narrative.push_str(&format!(
            "\n【工坊營造・實質勞動】{}\n你投入體魄勞作，自市場營造司領取當班工造津貼收入行囊（勞動微耗體魄，補給資財結算）。",
            labor_desc
        ));
        if obj.name.contains("毛竹") || obj.id == "wt_outer_reed_rack" || obj.id == "wt_obj_timber_reed_stacks" {
            let _ = db::add_to_inventory(pid, "mat_timber_log", 1);
            narrative.push_str("\n【料場採集】你在搬運整理排筏竹木之餘，順道自料垛獲取了一根韌性上乘的「伐採原木」收入行囊。");
        } else if obj.name.contains("鹽倉") || obj.id == "wt_outer_salt_vat" || obj.id == "wt_obj_iron_salt_stacks" {
            let _ = db::add_to_inventory(pid, "mat_iron_scrap", 1);
            narrative.push_str("\n【倉儲結存】你在協助鹽倉搬運壓包之際，獲贈了一塊用以加固木架的「回收熟鐵料塊」收入行囊。");
        } else if obj.name.contains("蘆葦") {
            let _ = db::add_to_inventory(pid, "mat_reed_bundle", 1);
            narrative.push_str("\n【草澤收穫】你順手割取了一捆青翠韌實的「水澤老蘆草綑」收入行囊。");
        } else if obj.id == "wt_forge_furnace_anvil"
            || obj.name.contains("鍛造")
            || obj.name.contains("鐵砧")
            || obj.name.contains("鍛爐")
            || obj.name.contains("鍛台")
        {
            let _ = db::add_to_inventory(pid, "mat_iron_scrap", 2);
            narrative.push_str("\n【爐邊拾料】你在高爐鍛台旁清整爐渣時，順手拾得兩塊成色純良的「回收熟鐵料塊」（原料）收入行囊。");

            // 檢查持握兵刃或行囊工具是否需要回火修整
            let vessel_to_temper = if let Ok(Some(ch)) = db::get_entity(pid) {
                let mut cand = None;
                if !ch.equipment_slots.is_empty() {
                    if let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(&ch.equipment_slots) {
                        let check_slots = ["weapon", "hold_r", "main_hand", "hold", "tool", "hold_l"];
                        for s in check_slots {
                            if let Some(id) = slots.get(s).filter(|id| !id.is_empty()) {
                                let name = db::get_item_def(id).map(|d| d.name).unwrap_or_else(|| id.clone());
                                if crate::pg::living_world::is_weapon_or_tool_item(id, &name) {
                                    let st = crate::pg::living_world::get_vessel_state(pid, id);
                                    if st.condition != crate::pg::living_world::VesselCondition::Sharp {
                                        cand = Some((id.clone(), name));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                if cand.is_none() {
                    let inv_items: Vec<serde_json::Value> = serde_json::from_str(&ch.inventory).unwrap_or_default();
                    for it in inv_items {
                        let id = it.get("item_id").or_else(|| it.get("id")).and_then(|v| v.as_str()).unwrap_or_default();
                        if id.is_empty() { continue; }
                        let name = db::get_item_def(id).map(|d| d.name).unwrap_or_else(|| id.to_string());
                        if crate::pg::living_world::is_weapon_or_tool_item(id, &name) {
                            let st = crate::pg::living_world::get_vessel_state(pid, id);
                            if st.condition != crate::pg::living_world::VesselCondition::Sharp {
                                cand = Some((id.to_string(), name));
                                break;
                            }
                        }
                    }
                }
                cand
            } else {
                None
            };

            if let Some((item_id, item_name)) = vessel_to_temper {
                let prev_st = crate::pg::living_world::get_vessel_state(pid, &item_id);
                let st = crate::pg::living_world::temper_vessel_at_forge(pid, &item_id, now);
                let fix_desc = match prev_st.condition {
                    crate::pg::living_world::VesselCondition::SlightlyDull => "微鈍之刃口重歸「鋒芒畢露」",
                    crate::pg::living_world::VesselCondition::RolledEdge => "捲曲崩缺之刃口重歸「鋒芒畢露」",
                    _ => "刃口受創處重歸「鋒芒畢露」",
                };
                narrative.push_str(&format!(
                    "\n【爐火回火・器況復原】你順手將受損的「{}」探入炭火回火、於鐵砧輕敲校直，{}（人器磨合：{}）！",
                    item_name,
                    fix_desc,
                    st.status_badge()
                ));
                crate::pg::living_world::record_breath_activity(
                    pid,
                    crate::pg::living_world::BreathHabitKind::ForgeHeavy,
                    2,
                    now,
                );
            }
        } else if obj.id == "wt_wood_planing_bench" || obj.name.contains("木作台") {
            let _ = db::add_to_inventory(pid, "mat_timber_log", 2);
            narrative.push_str("\n【木坊拾料】你在木作台下整理邊角良木，順手收起兩截乾燥結實的「伐採原木」（原料）收入行囊。");
        }
        if obj.id == "wt_forge_quench_trough" || obj.name.contains("淬火槽") {
            if let Ok((_mut_id, _mut_name, mut_narrative)) =
                crate::game::object_ecology::mutate_finished_item(pid, "auto", &obj.name, now)
            {
                narrative.push_str(&format!("\n【淬火異變】藉著導能淬火槽的極溫冷熱激盪，{}", mut_narrative));
            } else if let Ok((crafted_id, _cname, is_mut, craft_narr)) =
                crate::game::object_ecology::craft_from_raw_material(pid, "auto", &obj.name, now)
            {
                narrative.push_str(&format!("\n【工造煉製】{}", craft_narr));
                if !is_mut {
                    if let Ok((_mid, _mname, mut_narr)) =
                        crate::game::object_ecology::mutate_finished_item(pid, &crafted_id, &obj.name, now)
                    {
                        narrative.push_str(&format!("\n【淬火異變】藉著導能淬火槽的極溫冷熱激盪，{}", mut_narr));
                    }
                }
            }
        }
        let is_machinery = obj.name.contains("水力聯動重鐵錘")
            || obj.name.contains("活塞木風箱")
            || obj.name.contains("鍛台")
            || obj.name.contains("木作台")
            || obj.name.contains("石墨坩堝")
            || obj.name.contains("大水車連軸排鋸")
            || obj.name.contains("抄紙青竹簾")
            || obj.name.contains("搗墨鐵臼")
            || obj.name.contains("壓印機")
            || obj.name.contains("帆布高織機");
        if is_machinery {
            if let Ok((_item_id, _item_name, _is_mut, craft_narrative)) =
                crate::game::object_ecology::craft_from_raw_material(pid, "auto", &obj.name, now)
            {
                narrative.push_str(&format!("\n【工造煉製】在操作工坊重器勞動之餘，{}", craft_narrative));
            }
        }
        push_refresh(conn, pid);
        return;
    }

    // --- 文曜外環餐飲、茶攤、酒肆器物 ---
    if matches!(
        obj.id.as_str(),
        "wt_outer_sailor_tea"
            | "wt_obj_river_tea_kettle"
            | "wt_outer_worker_canteen"
            | "wt_obj_forge_mess_cauldron"
            | "wt_outer_gate_tavern"
            | "wt_obj_mutton_broth_pot"
    ) || (action == "Taste" || action == "WarmHands") && (
        obj.name.contains("大銅茶壺")
            || obj.name.contains("水手粗陶大碗")
            || obj.name.contains("厚鐵炒鑊")
            || obj.name.contains("老白乾")
            || obj.name.contains("羊肉骨頭滾湯")
    ) {
        let magnesium = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| e.magnesium)
            .unwrap_or(0);
        let cost = if obj.id.contains("tavern") || obj.name.contains("羊肉") { 3 } else { 2 };
        if magnesium < cost {
            narrative.push_str(&format!(
                "\n【茶飯小憩】隨身鎂幣不足（需 {} 鎂），店家熱情招呼後遞上一盞溫涼清水解渴。",
                cost
            ));
            return;
        }
        let _ = db::transfer_magnesium(pid, "world_market", cost).or_else(|_| db::add_magnesium(pid, -cost));
        let (vit_gain, qi_gain, desc) = match obj.id.as_str() {
            "wt_outer_sailor_tea" | "wt_obj_river_tea_kettle" => (
                10,
                8,
                "你倒了一大碗熱氣蒸騰的濃釅臨江磚茶仰頭飲盡，微苦甘醇驅散了江風濕寒，通體暖意融融。",
            ),
            "wt_outer_worker_canteen" | "wt_obj_forge_mess_cauldron" => (
                14,
                8,
                "你盛了一大碗熱油爆炒的濃油赤醬燴原牛肉與大塊雜糧饃，肉香濃郁，大口朵頤間四肢力量漸復充盈。",
            ),
            _ => (
                16,
                10,
                "你端起粗瓷大碗痛飲奶白滾燙的原羊骨高湯，大蔥花與白胡椒粉直透百骸，驅盡荒原風塵寒意，渾身舒泰無比。",
            ),
        };
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + vit_gain).min(100);
                e.qi = (e.qi + qi_gain).min(100);
            });
        }
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        narrative.push_str(&format!(
            "\n【飲食溫養】{}\n支付 {} 鎂，體魄與元氣皆獲調理舒暢。",
            desc, cost
        ));
        if let Some((shanhai_cook_narr, _v, _q)) = crate::game::object_ecology::try_cook_shanhai_delicacy(pid, &obj.name) {
            narrative.push('\n');
            narrative.push_str(&shanhai_cook_narr);
        }
        push_refresh(conn, pid);
        return;
    }

    // --- 文曜草藥、醫館、獸醫器物 ---
    if matches!(
        obj.id.as_str(),
        "wt_outer_herb_cabinet"
            | "wt_obj_herb_cabinet_mortar"
            | "wt_obj_acupuncture_pulse_pillow"
            | "wt_outer_herb_drying"
            | "wt_obj_herb_drying_rack"
            | "wt_outer_vet_stocks"
            | "wt_obj_veterinary_box"
    ) || (action == "Weigh" || action == "Pound" || action == "Smell") && (
        obj.name.contains("百眼藥櫃")
            || obj.name.contains("紫銅搗藥臼")
            || obj.name.contains("採藥背簍竹架")
            || obj.name.contains("修蹄鐵鉗")
            || obj.name.contains("獸用草藥丸")
    ) {
        if obj.id == "wt_outer_herb_cabinet" || obj.id == "wt_obj_herb_cabinet_mortar" || obj.id == "wt_obj_acupuncture_pulse_pillow" || obj.name.contains("藥櫃") || obj.name.contains("脈枕") {
            if action == "Smell" {
                narrative.push_str("\n【藥香沁脾】百眼藥櫃中散發出清苦回甘的草木藥香，令人頭目清朗。");
                push_refresh(conn, pid);
                return;
            }
            if action == "Weigh" {
                // Free weighing interaction, already handled by base look/object interaction or returns without charge
                push_refresh(conn, pid);
                return;
            }
            if action == "Use" || action == "Operate" || action == "Buy" || action == "Acquire" {
                let magnesium = db::get_entity(pid)
                    .ok()
                    .flatten()
                    .map(|e| e.magnesium)
                    .unwrap_or(0);
                if magnesium >= 3 {
                    let _ = db::transfer_magnesium(pid, "world_market", 3).or_else(|_| db::add_magnesium(pid, -3));
                    if let Some(arc) = crate::store::get_store() {
                        let _ = arc.write().update_entity(pid, |e| {
                            e.vit = (e.vit + 18).min(100);
                            e.qi = (e.qi + 10).min(100);
                        });
                    }
                    let _ = event::append(
                        now,
                        pid,
                        types::SCENE_OBJECT_ACTION,
                        &format!("{}|{}", obj.id, action),
                    );
                    narrative.push_str("\n【本草調養】百草堂醫師為你開具清心避瘴草藥湯劑，微苦回甘的藥力入喉，經絡暢達、靈台清朗（心神與周身氣血隨之充實）。");
                    push_refresh(conn, pid);
                } else {
                    narrative.push_str("\n【草藥提示】診金與草藥調劑需 3 鎂，隨身錢資不足，醫師先行贈與一盞潤喉清熱草藥茶稍作調理。");
                }
                return;
            }
            return;
        } else {
            // wt_outer_herb_drying / wt_outer_vet_stocks 協助分藥與照料牲口
            let ent = db::get_entity(pid).ok().flatten();
            let vit = ent.as_ref().map(|e| e.vit).unwrap_or(0);
            if vit > 5 {
                let _ = db::update_vit(pid, (vit - 2).max(1));
                let _ = db::transfer_magnesium("world_market", pid, 5).or_else(|_| db::add_magnesium(pid, 5));
                let _ = event::append(
                    now,
                    pid,
                    types::SCENE_OBJECT_ACTION,
                    &format!("{}|{}", obj.id, action),
                );
                narrative.push_str("\n【醫坊襄助】你協助翻曬篾席上的新鮮草藥並修整駝獸硬蹄，動作細緻沉穩，獲贈 5 鎂勞動津貼（稍耗體力，獲得 5 鎂）。");
                push_refresh(conn, pid);
            } else {
                narrative.push_str("\n【襄助提示】你神色疲憊，醫館掌事勸你先在長凳上坐下歇息片刻再行幫忙。");
            }
            return;
        }
    }

    // --- 文曜外環學術、測繪、天象、市集、城門坊器物 ---
    if matches!(
        obj.id.as_str(),
        "wt_outer_map_table"
            | "wt_obj_cartography_table"
            | "wt_obj_surveyor_theodolite"
            | "wt_outer_abacus_slate"
            | "wt_obj_abacus_counters"
            | "wt_outer_armillary"
            | "wt_obj_armillary_almanac"
            | "wt_obj_wind_vane_gauge"
            | "wt_outer_brush_rack"
            | "wt_obj_brush_inkstone_rack"
            | "wt_outer_fish_auction"
            | "wt_obj_fish_auction_trough"
            | "wt_obj_fish_baskets"
            | "wt_outer_dried_fish_scale"
            | "wt_outer_tool_rack"
            | "wt_obj_farm_tool_rack"
            | "wt_outer_hardware_bins"
            | "wt_obj_hardware_bins"
            | "wt_outer_cattle_ring"
            | "wt_obj_livestock_auction_ring"
            | "wt_outer_weapon_rack"
            | "wt_obj_weapon_rack_stone"
            | "wt_outer_dispatch_board"
            | "wt_obj_weighbridge_milepost"
            | "wt_outer_caravan_trough"
            | "wt_obj_caravan_hitch_trough"
            | "wt_obj_caravan_bell_rack"
            | "wt_inner_gate_arch"
            | "wt_gate_stone_archway_board"
            | "wt_inner_gate_trough"
            | "wt_gate_water_culvert_drum"
            | "wt_inner_gate_gong"
            | "wt_gate_patrol_gong_stone"
    ) || (action == "Measure" || action == "Calculate" || action == "Observe" || action == "TestBrush" || action == "GrindInk" || action == "Weigh" || action == "Sharpen" || action == "Feed" || action == "Shake" || action == "Ring") && (
        obj.name.contains("測繪大畫案")
            || obj.name.contains("九章算學")
            || obj.name.contains("北辰定緯銅儀")
            || obj.name.contains("湖羊長鋒筆架")
            || obj.name.contains("鮮魚競價冰槽")
            || obj.name.contains("農具")
            || obj.name.contains("五金分格鐵匣")
            || obj.name.contains("牲口競價木欄")
            || obj.name.contains("護行厚背獵刀架")
            || obj.name.contains("過磅生鐵地台")
            || obj.name.contains("青石牌坊")
            || obj.name.contains("引水石渠")
            || obj.name.contains("巡坊銅鑼")
    ) {
        let (vit_delta, qi_delta, feedback) = match obj.id.as_str() {
            "wt_outer_map_table" | "wt_obj_cartography_table" | "wt_obj_surveyor_theodolite" => (
                0,
                6,
                "你以黃銅兩腳分規在長案輿圖上比量半島水道與陸路走勢，山川形勝與關防隘口瞭然於胸，心胸豁然開朗。",
            ),
            "wt_outer_abacus_slate" | "wt_obj_abacus_counters" => (
                0,
                6,
                "你拈起紅木二十三檔算盤輕撥珠子，清脆劈啪聲中推演九章勾股受力算題，心神專注澄澈。",
            ),
            "wt_outer_armillary" | "wt_obj_armillary_almanac" | "wt_obj_wind_vane_gauge" => (
                0,
                8,
                "你透過定緯儀銅環仰觀極星子午走向，黑曜石日晷刻影分明，推步四時節氣，頓悟天行有常之道。",
            ),
            "wt_outer_brush_rack" | "wt_obj_brush_inkstone_rack" => (
                0,
                6,
                "你挑選兼毫長鋒在端溪老坑硯台中飽蘸墨汁，落筆行雲流水，滿室墨香令人意氣平和。",
            ),
            "wt_outer_fish_auction" | "wt_obj_fish_auction_trough" | "wt_obj_fish_baskets" | "wt_outer_dried_fish_scale" => (
                0,
                5,
                "你俯身查驗冰槽中銀鱗閃爍的生鮮大魚，撥動黃銅懸臂秤核算批發斤兩，江海活水清鮮之氣撲面而來。",
            ),
            "wt_outer_tool_rack" | "wt_obj_farm_tool_rack" | "wt_outer_hardware_bins" | "wt_obj_hardware_bins" => (
                0,
                5,
                "你取下剛淬火開刃的鋼鋤鐮刀細看刃線，翻檢鐵匣鉚釘規格，百工利器質樸精準，倍添營造踏實之感。",
            ),
            "wt_outer_cattle_ring" | "wt_obj_livestock_auction_ring" => (
                0,
                5,
                "你駐足木圍欄觀看大角原牛矯健踱步，行商比劃手勢叫價，邊地大市的蓬勃生機撲面而來。",
            ),
            "wt_outer_weapon_rack" | "wt_obj_weapon_rack_stone" => (
                0,
                6,
                "你取厚背獵刀在油石槽上緩緩推磨，冷冽刃光倒映雙眸，隨身戒備與沉穩之氣油然而生。",
            ),
            "wt_outer_dispatch_board" | "wt_obj_weighbridge_milepost" => (
                0,
                5,
                "你凝神端讀西道花崗岩路石里程：城西五里為平川，十里避風坡取水點標示詳盡，遠行前程井然在胸。",
            ),
            "wt_outer_caravan_trough" | "wt_obj_caravan_hitch_trough" | "wt_obj_caravan_bell_rack" => (
                0,
                5,
                "你輕輕搖響廊下成排鍛銅駝鈴，渾厚鈴音在開闊驛場迴盪數里，駝獸溫順抬頭，心神平靜祥和。",
            ),
            "wt_inner_gate_trough" | "wt_gate_water_culvert_drum" => (
                5,
                4,
                "你走近坊口石渠掬捧清冽活水洗面並在歇腳石鼓上暫卸行囊，清泉甘涼滌盡暑燥風塵，精神大振。",
            ),
            "wt_inner_gate_gong" | "wt_gate_patrol_gong_stone" => (
                0,
                6,
                "你輕叩坊壁巡更厚銅鑼，沉渾鑼聲悠悠蕩蕩聲震四街，驚起飛簷宿鳥，城郭晨昏秩序肅然在目。",
            ),
            _ => (
                0,
                6,
                "你仰頭瞻仰巍峨挺拔的青石牌坊，拂拭指路刻石楷書文字，各坊街巷交匯處的繁華氣象盡收眼底。",
            ),
        };
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                if vit_delta > 0 {
                    e.vit = (e.vit + vit_delta).min(100);
                }
                if qi_delta > 0 {
                    e.qi = (e.qi + qi_delta).min(100);
                }
            });
        }
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        if vit_delta > 0 {
            narrative.push_str(&format!(
                "\n【城郭風物・格物研察】{}\n（體魄增益，心神元氣充盈）。",
                feedback
            ));
        } else {
            narrative.push_str(&format!(
                "\n【城郭風物・格物研察】{}\n（心神獲益，元氣獲得恢復）。",
                feedback
            ));
        }
        push_refresh(conn, pid);
        return;
    }



    // 3) Food, Drink & Tea (action == "Taste" or obj.id == "wt_canteen_red_iron_cauldron" or obj.id == "wt_canteen_fermented_tofu_jar" or obj.id == "wt_inn_hearth_tea_table" or catalog category "11")
    let cat_opt = crate::game::peninsula_catalog::find_catalog_entry(&obj.id);
    let is_food_item = cat_opt.is_some_and(|c| {
        c.category_num == "11" || matches!(c.category_num, "08" | "09" | "10")
    });
    let is_food_interaction = action == "Taste"
        || ((obj.id == "wt_canteen_red_iron_cauldron"
            || obj.id == "wt_canteen_fermented_tofu_jar"
            || obj.id == "wt_inn_hearth_tea_table"
            || is_food_item)
            && matches!(action, "Use" | "Eat" | "Drink" | "BuySoup"));

    if is_food_interaction {
        let room_name = resolve_room_name(player_room);
        let is_residential = room_name.contains("住宅")
            || room_name.contains("民宅")
            || room_name.contains("居所")
            || room_name.contains("家屋")
            || room_name.contains("公寓")
            || room_name.contains("街屋")
            || room_name.contains("農舍")
            || room_name.contains("石屋")
            || room_name.contains("漁舍")
            || room_name.contains("暖屋")
            || room_name.contains("宿舍")
            || room_name.contains("高腳木屋")
            || room_name.contains("宅邸")
            || room_name.contains("宅院")
            || room_name.contains("莊園")
            || room_name.contains("別院")
            || room_name.contains("民舍");
        let has_deed = db::inventory_has_item(pid, "wt_cat_0793");
        let cost = if is_residential && has_deed { 0 } else { 2 };
        let magnesium = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| e.magnesium)
            .unwrap_or(0);
        if magnesium >= cost {
            if cost > 0 {
                let _ = db::add_magnesium(pid, -cost);
            }
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 8).min(100);
                    e.qi = (e.qi + 6).min(100);
                });
            }
            if cost > 0 {
                narrative.push_str(&format!(
                    "\n【飲食溫養】花費 {} 鎂，溫熱鮮美的飲食入腹，你感到氣血充盈、精神一振（體魄與元氣獲得恢復）。",
                    cost
                ));
            } else {
                narrative.push_str(
                    "\n【飲食溫養】溫熱鮮美的飲食入腹，你感到氣血充盈、精神一振（體魄與元氣獲得恢復）。",
                );
            }
            push_refresh(conn, pid);
        } else {
            narrative.push_str("\n【飲食提示】手頭的鎂不足以支付這份餐食，暫且歇手。");
        }
        return;
    }

    // 4) Bathing, Sleeping, Warming & Medical Care
    let is_bed_or_bath_or_clinic = obj.name.contains("大床")
        || obj.name.contains("臥榻")
        || obj.name.contains("暖被")
        || obj.name.contains("床鋪")
        || obj.name.contains("熱水槽")
        || obj.name.contains("溫石")
        || obj.name.contains("浴桶")
        || obj.name.contains("蒲團")
        || obj.name.contains("坐墊")
        || obj.name.contains("茶案")
        || obj.name.contains("草蓆")
        || obj.id.contains("bed")
        || obj.id.contains("bath")
        || obj.id.contains("clinic")
        || obj.id == "wt_reg_wenyao_amenities"
        || obj.id == "wt_reg_wenyao_home";
    let is_recovery_action = if matches!(action, "Use" | "Operate") {
        is_bed_or_bath_or_clinic
    } else {
        matches!(action, "Sit" | "Rest" | "Meditate" | "Sleep" | "Bath" | "Wash" | "WarmHands" | "DryOff")
    };

    if is_recovery_action && obj.id != "wt_hall_council_table" && !obj.name.contains("公議案桌") {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = 100;
                e.qi = 100;
            });
        }
        let _ = event::append(
            now,
            pid,
            types::PLAYER_BODY,
            &format!("rested_via_{}", obj.id),
        );
        let rest_note = if action == "Sleep" || obj.id.contains("bed") || obj.name.contains("大床") || obj.name.contains("羽絨暖被") || obj.name.contains("臥榻") || obj.name.contains("床鋪") {
            "\n【臥床安歇】你鑽入溫暖蓬鬆的羽絨暖被安穩入眠，滿室松香與安神草清香令人心神寧靜，周身疲憊盡數散去，體魄與心神皆充盈圓滿。"
        } else if action == "Bath" || action == "Wash" || obj.id.contains("bath") || action == "DryOff" || obj.name.contains("熱水槽") || obj.name.contains("溫石") {
            "\n【溫石洗沐】熱氣騰騰的溫石活水漫過身軀，洗去了一路風塵與肌理酸脹，身心倍感舒泰通透，體魄與心神皆充盈圓滿。"
        } else if obj.id.contains("clinic") {
            "\n【調養舒筋】經脈絡調理與草木藥香溫養，周身酸痛與疲憊盡數散去，體魄與心神皆充盈圓滿。"
        } else if action == "Meditate" {
            "\n【凝神靜坐】你盤膝端坐閉目凝神，調勻呼吸，漸入物我兩忘之境，周身真氣漸趨充沛圓融。"
        } else {
            "\n【駐足歇息】你在清風中稍作歇息，呼吸漸趨平緩，周身疲憊隨之散去，體魄與心神皆充盈圓滿。"
        };
        narrative.push_str(rest_note);
        append_somatic_awareness_note(pid, now, narrative);
        push_refresh(conn, pid);
        return;
    }

    // 5) Gathering & Portable Item Acquisition (action == "Gather" || action == "Acquire" or portable wt_cat_xxxx item in shops/workshops/wilderness when action == "Use" || action == "Operate")
    if let Some(cat) = cat_opt {
        if action == "Gather" {
            let _ = db::add_to_inventory(pid, cat.id, 1);
            narrative.push_str(&format!(
                "\n【採集入袋】你將採收所得的「{}」妥善收入隨身行囊。",
                cat.name
            ));
            push_refresh(conn, pid);
            return;
        } else if crate::game::peninsula_catalog::is_portable_catalog_entry(cat)
            && !db::inventory_has_item(pid, cat.id)
            && (action == "Acquire" || action == "Use" || action == "Operate")
        {
            let price = crate::game::peninsula_catalog::catalog_entry_price_mg(cat);
            let magnesium = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.magnesium)
                .unwrap_or(0);
            if magnesium >= price {
                let _ = db::add_magnesium(pid, -price);
                let _ = db::add_to_inventory(pid, cat.id, 1);
                if action == "Acquire" {
                    *narrative = format!(
                        "你支付 {} 鎂，將這件「{}」（由{}取{}精製）妥善收入隨身行囊。",
                        price, cat.name, cat.workshop, cat.origin_material
                    );
                } else {
                    narrative.push_str(&format!(
                        "\n【器物入手】你支付 {} 鎂，將這件「{}」納入隨身行囊。",
                        price, cat.name
                    ));
                }
                push_refresh(conn, pid);
            } else if action == "Acquire" {
                *narrative = format!(
                    "【結算提示】採買「{}」需支付 {} 鎂，你隨身錢資不足，未能購入。",
                    cat.name, price
                );
            }
            return;
        } else if action == "Acquire" {
            *narrative = format!(
                "【隨身收存】你的行囊中已收有「{}」，毋需重複採買。",
                cat.name
            );
            return;
        }
    }
    if (action == "Rest" || action == "Meditate") && !narrative.contains("【動中體察・內息與氣脈】") {
        append_somatic_awareness_note(pid, now, narrative);
    }
}

fn append_somatic_awareness_note(pid: &str, now: i64, narrative: &mut String) {
    let habit = crate::pg::living_world::record_breath_activity(
        pid,
        crate::pg::living_world::BreathHabitKind::ChamberClear,
        2,
        now,
    );
    let soul_seed = db::get_entity(pid).ok().flatten().and_then(|e| e.soul_seed).unwrap_or(0);
    let meridian_prose = crate::combat::wordplate::somatic_meridian_awareness(soul_seed);
    let somatic_desc = habit.dominant.somatic_description();
    let somatic_trimmed = somatic_desc.trim_end_matches('。');
    narrative.push_str(&format!(
        "\n【動中體察・內息與氣脈】此際斂氣澄心，體內「{}」隨呼吸自如流轉——{}。\n內觀周天經絡：{}",
        habit.dominant.label(),
        somatic_trimmed,
        meridian_prose
    ));
}

const PLAYER_GATHER_BATCH: i32 = 3;

fn format_runtime_gather_batch_narrative(
    gathered: &crate::pg::world_resource::GatherResult,
) -> String {
    if gathered.quantity <= 1 {
        gathered.narrative.clone()
    } else {
        format!(
            "{}\n你順著手勢一氣呵成連取了 {} 份「{}」，直到手頭分量紮實才收手。",
            gathered.narrative, gathered.quantity, gathered.name
        )
    }
}

fn try_grid_cell_object_action(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
) -> bool {
    let Some(coord) = parse_grid_room_id(player_room) else {
        return false;
    };
    if action != "Gather" && action != "Harvest" && action != "Take" {
        return false;
    }

    let has_target_obj = crate::server::grid_manager::get_runtime_square_grid()
        .and_then(|g| g.get(coord).cloned())
        .map(|c| c.objects.iter().any(|o| o.id == obj.id || o.name == obj.name))
        .unwrap_or(false);
    if !has_target_obj {
        return false;
    }

    let mut updated_state = None;
    let mut zero_quantity = false;
    crate::server::grid_manager::with_square_grid_mut(|grid| {
        if let Some(cell) = grid.get_mut(coord) {
            if let Some(target_obj) = cell.objects.iter_mut().find(|o| o.id == obj.id || o.name == obj.name) {
                if target_obj.quantity > 0 {
                    target_obj.quantity -= 1;
                    let remaining_qty = target_obj.quantity;
                    let obj_name = target_obj.name.clone();
                    let obj_id = target_obj.id.clone();
                    updated_state = Some((
                        cell.explored,
                        cell.freshness,
                        cell.objects.clone(),
                        remaining_qty,
                        obj_name,
                        obj_id,
                    ));
                } else {
                    zero_quantity = true;
                }
            }
        }
    });

    if zero_quantity {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative: format!("眼前的「{}」已被採擷殆盡，需待時序流轉自然復生。", obj.name),
            success: false,
            actions: vec!["Look".into()],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return true;
    }

    let Some((explored, freshness, updated_objects, remaining_qty, obj_name, obj_id)) = updated_state else {
        return false;
    };

    let _ = db::update_cell_state(coord.x, coord.y, explored, freshness, &updated_objects);

    let item_id = if obj_id.starts_with("wild_fruit")
        || obj_name.contains("漿果")
        || obj_name.contains("果")
        || obj_name.contains("山茶籽")
        || obj_name.contains("燕窩")
    {
        "harvest_berries"
    } else if obj_id.starts_with("wood")
        || obj_name.contains("木材")
        || obj_name.contains("原木")
        || obj_name.contains("枯木")
        || obj_name.contains("倒木")
        || obj_name.contains("心材")
        || obj_name.contains("木材")
        || obj_name.contains("良材")
        || obj_name.contains("雲杉")
        || obj_name.contains("烏木")
        || obj_name.contains("韌竹")
        || obj_name.contains("鐵杉")
        || obj_name.contains("穀木")
        || obj_name.contains("樹脂")
    {
        if obj_name.contains("溫木") || obj_name.contains("烏木") || obj_name.contains("心材") || obj_name.contains("樹脂") {
            "harvest_rare_wood"
        } else if obj_name.contains("雲杉") || obj_name.contains("原木") {
            "harvest_wood"
        } else {
            "harvest_deadwood"
        }
    } else if obj_id.starts_with("herb")
        || obj_name.contains("草")
        || obj_name.contains("苔")
        || obj_name.contains("蓮")
        || obj_name.contains("藤")
        || obj_name.contains("葦")
        || obj_name.contains("百草")
    {
        if obj_name.contains("藤") {
            "harvest_vine"
        } else {
            "harvest_herb"
        }
    } else if obj_id.starts_with("ore")
        || obj_name.contains("礦")
        || obj_name.contains("玉")
        || obj_name.contains("石")
        || obj_name.contains("岩")
        || obj_name.contains("晶")
        || obj_name.contains("髓")
        || obj_name.contains("貝")
        || obj_name.contains("砂")
        || obj_name.contains("黏土")
        || obj_name.contains("鹽")
    {
        if obj_name.contains("玉") || obj_name.contains("晶") || obj_name.contains("髓") {
            "harvest_crystal"
        } else if obj_name.contains("鐵") || obj_name.contains("銅") || obj_name.contains("礦") || obj_name.contains("煤") {
            "harvest_ore"
        } else {
            "harvest_hardstone"
        }
    } else if obj_id.starts_with("water_source") || obj_name.contains("水") {
        "fresh_water"
    } else if obj_id.starts_with("grain") || obj_name.contains("稻") {
        "grain_rice"
    } else if obj_id.starts_with("wheat") || obj_name.contains("麥") {
        "grain_wheat"
    } else {
        "harvest_herb"
    };
    let _ = db::add_to_inventory(pid, item_id, 1);
    let now_unix = crate::game::now_unix();
    let (_extra_qty, harvest_note) = crate::game::object_ecology::harvest_from_world_object(
        pid,
        &obj.id,
        &obj.name,
        item_id,
        now_unix,
    );

    let actions = if remaining_qty > 0 {
        vec!["Look".into(), "Gather".into()]
    } else {
        vec!["Look".into()]
    };

    let mut narrative = format!("你俯身細心採擷，將一份「{}」收入行囊之中。", obj.name);
    if !harvest_note.is_empty() {
        narrative.push(' ');
        narrative.push_str(&harvest_note);
    }

    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: action.into(),
        target_id: obj.id.clone(),
        target_name: obj.name.clone(),
        narrative,
        success: true,
        actions,
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });

    push_refresh(conn, pid);
    refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
    true
}

fn try_runtime_resource_action(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now_unix: i64,
) -> bool {
    if !obj.id.starts_with("resource:") || action != "Gather" {
        return false;
    }
    let (_, _, _, game_day) = game::game_time_now(
        now_unix,
        conn.cfg.game_time_epoch_unix,
        conn.cfg.game_time_scale,
    );
    let Some(pool) = crate::pg::pool() else {
        conn.send_error("世界物質主資料暫時不可用");
        return true;
    };
    match crate::pg::world_resource::gather(
        &pool,
        pid,
        player_room,
        PLAYER_GATHER_BATCH,
        game_day,
        now_unix,
        "player_gather",
    ) {
        Ok(Some(gathered)) => {
            if let Some(store) = crate::store::get_store()
                && let Err(e) = store
                    .write()
                    .mirror_inventory_from_authority(pid, &gathered.inventory)
            {
                tracing::warn!("resource gather cache mirror: {e}");
            }
            let _ = event::append(
                now_unix,
                pid,
                types::RESOURCE_GATHER,
                &format!("{}|{}|{}", player_room, gathered.item_id, gathered.quantity),
            );
            let narrative = format_runtime_gather_batch_narrative(&gathered);
            let actions = if gathered.source_stock_after <= 0 {
                vec!["Look".into()]
            } else {
                vec!["Look".into(), "Gather".into()]
            };
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.into(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative,
                success: true,
                actions,
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
            refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
        }
        Ok(None) => {
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.into(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative: "眼前能取用的部分已空；它不會憑空再出現。".into(),
                success: false,
                actions: vec!["Look".into()],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
        }
        Err(e) => {
            tracing::warn!("runtime resource gather failed: {e}");
            conn.send_error("這次沒有取到東西");
        }
    }
    true
}

fn m3_020_scene_object_uses_history(object_id: &str, action: &str) -> bool {
    object_id.starts_with("m3_020_")
        && matches!(
            action,
            "Look" | "Queue" | "AcceptJob" | "JoinCheck" | "RegisterComplaint"
        )
}

fn m3_020_scene_object_followup_event_payload(object_id: &str, action: &str) -> Option<String> {
    match (object_id, action) {
        ("m3_020_stone_gate_pass_tags", "Read")
        | ("m3_020_flagstone_waiting_lines", "Use")
        | ("m3_020_mint_half_open_door", "Open")
        | ("m3_020_hire_short_work_boards", "Read")
        | ("m3_020_council_notice_strip", "Read") => Some(format!("{object_id}|{action}")),
        _ => None,
    }
}

fn m3_020_scene_object_state_event_payload(
    object_id: &str,
    action: &str,
    prior_payloads: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    match (object_id, action) {
        ("m3_020_mint_half_open_door", "Queue")
            if m3_020_waiting_line_has_been_used(prior_payloads)
                && !m3_020_mint_queue_state_active(prior_states) =>
        {
            Some(M3_020_MINT_QUEUE_STATE)
        }
        ("m3_020_hire_short_work_boards", "AcceptJob")
            if m3_020_short_work_board_has_been_read(prior_payloads)
                && !m3_020_short_work_state_active(prior_states) =>
        {
            Some(M3_020_SHORT_WORK_STATE)
        }
        ("m3_020_stone_gate_pass_tags", "JoinCheck")
            if m3_020_pass_tags_have_been_read(prior_payloads)
                && !m3_020_pass_check_state_active(prior_states) =>
        {
            Some(M3_020_PASS_CHECK_STATE)
        }
        ("m3_020_council_notice_strip", "RegisterComplaint")
            if m3_020_council_notice_has_been_read(prior_payloads)
                && !m3_020_council_complaint_state_active(prior_states) =>
        {
            Some(M3_020_COUNCIL_COMPLAINT_STATE)
        }
        _ => None,
    }
}

fn m3_020_scene_object_state_action_response(
    object_id: &str,
    action: &str,
    prior_payloads: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    match (object_id, action) {
        ("m3_020_mint_half_open_door", "Queue") if m3_020_mint_queue_state_active(prior_states) => {
            Some("你已在剛被接住的側線裡，兌牌人抬手要你留在門檻外，不讓後面的人重新擠上來。")
        }
        ("m3_020_mint_half_open_door", "Queue")
            if m3_020_waiting_line_has_been_used(prior_payloads) =>
        {
            Some(
                "你按剛分出的側線站到門檻外，兌牌人沒有放你進門，只把下一輪要核的牌號往石案邊推近；身後的人跟著把腳步收成一線。",
            )
        }
        ("m3_020_mint_half_open_door", "Queue") => {
            Some("廣場側線還沒分清，兌牌人抬手要你先退回人群後面，等旗手把去向喊明。")
        }
        ("m3_020_hire_short_work_boards", "AcceptJob")
            if m3_020_short_work_state_active(prior_states) =>
        {
            Some("棚頭人看見你已經應下活計，只把你壓回棚柱旁那條短線，等下一個小隊齊人。")
        }
        ("m3_020_hire_short_work_boards", "AcceptJob")
            if m3_020_short_work_board_has_been_read(prior_payloads) =>
        {
            Some(
                "你指向最外側那片短工牌，棚頭人把半日繩結往你手邊一拋，讓你站到商隊卸貨的小隊末尾。",
            )
        }
        ("m3_020_hire_short_work_boards", "AcceptJob") => {
            Some("棚頭人把短工牌往內側一翻，示意你先看清活計方向和時段，別擠在棚柱前亂應。")
        }
        ("m3_020_stone_gate_pass_tags", "JoinCheck")
            if m3_020_pass_check_state_active(prior_states) =>
        {
            Some("門衛看見你已經候檢，只把你壓回石案側邊那條短線，讓後面的人不要越過木牌。")
        }
        ("m3_020_stone_gate_pass_tags", "JoinCheck")
            if m3_020_pass_tags_have_been_read(prior_payloads) =>
        {
            Some(
                "你照著剛看清的牌序站到石案側邊，門衛把你的去向壓進短冊，讓商隊和工坊送件的人各退半步。",
            )
        }
        ("m3_020_stone_gate_pass_tags", "JoinCheck") => {
            Some("門衛把通行牌往石案內側一收，示意你先看清牌序，別直接擠進候檢線。")
        }
        ("m3_020_council_notice_strip", "RegisterComplaint")
            if m3_020_council_complaint_state_active(prior_states) =>
        {
            Some("記牌人看見你已經押名，只把短冊往階邊再壓一寸，讓你等棚內叫到再上前。")
        }
        ("m3_020_council_notice_strip", "RegisterComplaint")
            if m3_020_council_notice_has_been_read(prior_payloads) =>
        {
            Some(
                "你照告示把名字押在階側短冊上，記牌人用指節敲了敲通行牌申訴那一列，讓後面的人沿坡街邊等候。",
            )
        }
        ("m3_020_council_notice_strip", "RegisterComplaint") => {
            Some("記牌人按住告示邊角，要你先看清欠糧、傭工和通行牌申訴分列，再決定押到哪一行。")
        }
        _ => None,
    }
}

fn m3_020_scene_object_look_echo(
    object_id: &str,
    prior_payloads: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    m3_020_scene_object_state_echo(object_id, prior_states)
        .or_else(|| m3_020_scene_object_followup_echo(object_id, prior_payloads))
}

fn m3_020_scene_object_state_echo(
    object_id: &str,
    prior_states: &[String],
) -> Option<&'static str> {
    for state in prior_states.iter().rev() {
        let echo = match state.as_str() {
            M3_020_MINT_QUEUE_STATE => m3_020_mint_queue_state_echo(object_id),
            M3_020_SHORT_WORK_STATE => m3_020_short_work_state_echo(object_id),
            M3_020_PASS_CHECK_STATE => m3_020_pass_check_state_echo(object_id),
            M3_020_COUNCIL_COMPLAINT_STATE => m3_020_council_complaint_state_echo(object_id),
            _ => None,
        };
        if echo.is_some() {
            return echo;
        }
    }
    None
}

fn m3_020_mint_queue_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_020_mint_half_open_door" => Some(
            "你剛排進去的側線已被兌牌人接住，門檻外留下一條窄窄空位；他偶爾把下一張牌號往石案邊推近。",
        ),
        "m3_020_flagstone_waiting_lines" => {
            Some("鑄幣巷口那條側線被接住後，廣場中間少了一股橫穿的人潮，旗手的手勢比先前短。")
        }
        "m3_020_stone_gate_pass_tags" => {
            Some("門衛看見鑄幣巷口的側線已接住人，商隊那疊通行牌被往石案邊推近一點。")
        }
        _ => None,
    }
}

fn m3_020_short_work_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_020_hire_short_work_boards" => {
            Some("你剛應下的那片短工牌被棚頭人翻到繩結下，商隊卸貨的小隊在棚柱旁收成一列。")
        }
        "m3_020_flagstone_waiting_lines" => {
            Some("傭工棚邊收走一個半日活計後，旗石廣場通往棚口的那股人流短短薄了一層。")
        }
        _ => None,
    }
}

fn m3_020_pass_check_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_020_stone_gate_pass_tags" => {
            Some("你剛候檢的那道牌序被門衛壓在石案側邊，商隊和工坊送件的人各讓出半步。")
        }
        "m3_020_flagstone_waiting_lines" => {
            Some("石壘門那邊的候檢線被收進石案側邊後，旗石廣場入口少了一段反覆回頭的人聲。")
        }
        _ => None,
    }
}

fn m3_020_council_complaint_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_020_council_notice_strip" => {
            Some("你剛押名的短冊被記牌人壓在告示下方，通行牌申訴那一列多了一道折角。")
        }
        "m3_020_flagstone_waiting_lines" => {
            Some("議棚前階收走一列押名的人後，旗石廣場往坡街那側少了幾次逆向推擠。")
        }
        _ => None,
    }
}

fn m3_020_scene_object_followup_echo(
    object_id: &str,
    prior_payloads: &[String],
) -> Option<&'static str> {
    for payload in prior_payloads.iter().rev() {
        let Some((prior_object, prior_action)) = payload.split_once('|') else {
            continue;
        };
        if prior_object == object_id
            && let Some(echo) = m3_020_same_object_echo(object_id, prior_action)
        {
            return Some(echo);
        }
        if let Some(echo) = m3_020_cross_object_echo(object_id, prior_object, prior_action) {
            return Some(echo);
        }
    }
    None
}

fn m3_020_same_object_echo(object_id: &str, prior_action: &str) -> Option<&'static str> {
    match (object_id, prior_action) {
        ("m3_020_stone_gate_pass_tags", "Read") => {
            Some("剛讀過的幾張通行牌仍壓在石案邊，你再看時能更快分清商隊和工坊送件的隊列。")
        }
        ("m3_020_flagstone_waiting_lines", "Use") => {
            Some("你剛讓出的那一步還留在隊列邊，旗手順手把後面的人壓到同一側。")
        }
        ("m3_020_mint_half_open_door", "Open") => Some("門縫比先前更窄，兌牌人的手還壓在門板上。"),
        ("m3_020_hire_short_work_boards", "Read") => {
            Some("最急的那片短工牌仍翻在外側，棚頭人正按那幾個活計點人。")
        }
        ("m3_020_council_notice_strip", "Read") => {
            Some("你剛看過的幾行告示還在階邊，記牌人按著短冊不讓申訴人擠到坡街中間。")
        }
        _ => None,
    }
}

fn m3_020_cross_object_echo(
    object_id: &str,
    prior_object: &str,
    prior_action: &str,
) -> Option<&'static str> {
    match (object_id, prior_object, prior_action) {
        ("m3_020_mint_half_open_door", "m3_020_flagstone_waiting_lines", "Use") => {
            Some("廣場那邊剛理出的側線延伸到巷口，等換票的人比先前收得更窄。")
        }
        ("m3_020_hire_short_work_boards", "m3_020_flagstone_waiting_lines", "Use") => {
            Some("旗石廣場的隊列分清後，等工的人也被棚頭人壓到棚柱旁。")
        }
        _ => None,
    }
}

fn m3_020_waiting_line_has_been_used(prior_payloads: &[String]) -> bool {
    prior_payloads
        .iter()
        .any(|payload| payload == "m3_020_flagstone_waiting_lines|Use")
}

fn m3_020_mint_queue_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_020_MINT_QUEUE_STATE)
}

fn m3_020_short_work_board_has_been_read(prior_payloads: &[String]) -> bool {
    prior_payloads
        .iter()
        .any(|payload| payload == "m3_020_hire_short_work_boards|Read")
}

fn m3_020_short_work_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_020_SHORT_WORK_STATE)
}

fn m3_020_pass_tags_have_been_read(prior_payloads: &[String]) -> bool {
    prior_payloads
        .iter()
        .any(|payload| payload == "m3_020_stone_gate_pass_tags|Read")
}

fn m3_020_pass_check_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_020_PASS_CHECK_STATE)
}

fn m3_020_council_notice_has_been_read(prior_payloads: &[String]) -> bool {
    prior_payloads
        .iter()
        .any(|payload| payload == "m3_020_council_notice_strip|Read")
}

fn m3_020_council_complaint_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_020_COUNCIL_COMPLAINT_STATE)
}

const M3_027_GATHER_BATCH: usize = 3;

fn try_m3_027_material_gather(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now: i64,
) -> bool {
    if action != "Gather" {
        return false;
    }
    let Some(expected_item_id) = m3_027_material_source_item_for_object(&obj.id) else {
        return false;
    };
    let source_matches_object = materials::material_for_room_id(player_room)
        .is_some_and(|material| material.id == expected_item_id);
    if !source_matches_object {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative:
                "你伸手去理那堆料，卻只摸到不成束的濕碎邊；這裡暫時沒有能收進背包的可用材料。"
                    .to_string(),
            success: false,
            actions: Vec::new(),
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return true;
    }

    let mut gathered_count = 0;
    let mut last_gathered = None;
    for _ in 0..M3_027_GATHER_BATCH {
        if let Some(gathered) = materials::try_gather_material_for_room(player_room, now) {
            let stop = gathered.stock_remaining <= 0;
            last_gathered = Some(gathered);
            gathered_count += 1;
            if stop {
                break;
            }
        } else {
            break;
        }
    }

    let (narrative, success, stock_empty) = if gathered_count > 0 {
        let gathered = last_gathered.as_ref().unwrap();
        let Some(state_payload) =
            m3_027_material_gather_state_for_object(&obj.id, &gathered.item_id)
        else {
            return false;
        };
        let _ = db::add_to_inventory(pid, &gathered.item_id, gathered_count as i32);
        for _ in 0..gathered_count {
            let _ = event::append(
                now,
                pid,
                types::SCENE_OBJECT_ACTION,
                &format!("{}|Gather|{}", obj.id, gathered.item_id),
            );
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, state_payload);
        push_refresh(conn, pid);
        (
            m3_027_material_gather_batch_success_text(&obj.id, gathered, gathered_count),
            true,
            gathered.stock_remaining <= 0,
        )
    } else {
        (
            m3_027_material_gather_empty_text(&obj.id).to_string(),
            false,
            true,
        )
    };

    let (mut actions, move_target_id) = object_followup_buttons(obj, player_room, action, &[], &[]);
    if stock_empty {
        actions.retain(|a| a != "Gather");
    }
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: action.into(),
        target_id: obj.id.clone(),
        target_name: obj.name.clone(),
        narrative,
        success,
        actions,
        move_target_id,
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
    true
}

pub(crate) fn m3_027_material_gather_batch_success_text(
    object_id: &str,
    gathered: &materials::GatheredMaterial,
    count: usize,
) -> String {
    if count <= 1 {
        return m3_027_material_gather_success_text(object_id, gathered);
    }
    match object_id {
        "m3_027_north_bank_reeds" => {
            let trace = if gathered.stock_remaining <= 0 {
                "整片靠近水線的蘆叢都被割低了，水霧漫過斷莖，暫時只剩浮在泥面上的碎葉。"
            } else {
                "蘆叢順著你的手勢矮下一大片，斷口處滲出草汁，霧水沿著整齊的切口滴入泥潭。"
            };
            format!(
                "你撥開貼岸的河霧，順著草莖根部一氣呵成連割了數把；你抖落泥水，將它們俐落地扎成 {} 束「{}」妥善收進背包；{trace}",
                count, gathered.name
            )
        }
        "m3_027_market_sackcloth_pile" => {
            let trace = if gathered.stock_remaining <= 0 {
                "棚腳那堆舊麻袋已被裁割殆盡，只剩幾塊沾滿濕泥的爛邊，搬貨人乾脆把空木箱倒扣過來壓住空地。"
            } else {
                "舊麻袋堆肉眼可見地薄了一大截，旁邊的搬貨人見你動作俐落，也跟著挑揀剩下能包貨的布角。"
            };
            format!(
                "你半蹲在水尾寮渡頭市集棚腳，手起刀落連番裁剪，理出邊角完整、足堪包貨的布料，捲成 {} 卷「{}」收進背包；{trace}",
                count, gathered.name
            )
        }
        "m3_027_plank_wet_rope_coil" => {
            let trace = if gathered.stock_remaining <= 0 {
                "木樁內側只剩幾截浸水發脹的斷頭，船戶見狀把空出的繩釘用力敲緊。"
            } else {
                "木樁上原本層疊的繩圈頓時疏落開來，船戶趁空把剩下還吃得住力的繩頭順向樁後。"
            };
            format!(
                "你踩在微晃的踏板上，探身從木樁內側連續解下並盤好 {} 卷韌度結實的「{}」收進背包；{trace}",
                count, gathered.name
            )
        }
        _ => format!(
            "你動作俐落地從【{}】連續理出 {} 份可用材料收進背包。",
            gathered.origin_label, count
        ),
    }
}

fn m3_027_material_source_item_for_object(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_north_bank_reeds" => Some(M3_027_REED_ITEM_ID),
        "m3_027_market_sackcloth_pile" => Some(M3_027_SACKCLOTH_ITEM_ID),
        "m3_027_plank_wet_rope_coil" => Some(M3_027_ROPE_ITEM_ID),
        _ => None,
    }
}

fn m3_027_material_gather_state_for_object(object_id: &str, item_id: &str) -> Option<&'static str> {
    match (object_id, item_id) {
        ("m3_027_north_bank_reeds", M3_027_REED_ITEM_ID) => Some(M3_027_REED_GATHERED_STATE),
        ("m3_027_market_sackcloth_pile", M3_027_SACKCLOTH_ITEM_ID) => {
            Some(M3_027_SACKCLOTH_GATHERED_STATE)
        }
        ("m3_027_plank_wet_rope_coil", M3_027_ROPE_ITEM_ID) => Some(M3_027_ROPE_GATHERED_STATE),
        _ => None,
    }
}

fn m3_027_material_gather_success_text(
    object_id: &str,
    gathered: &materials::GatheredMaterial,
) -> String {
    match object_id {
        "m3_027_north_bank_reeds" => {
            let trace = if gathered.stock_remaining <= 0 {
                "這一側的蘆叢被割矮了，水線旁暫時只剩碎葉和濕莖。"
            } else {
                "你身後的蘆叢矮下去一小片，霧水順著新斷口往泥裡滴。"
            };
            format!(
                "你撥開貼岸的河霧，從濕蘆裡割下一束「{}」收進背包；{trace}",
                gathered.name
            )
        }
        "m3_027_market_sackcloth_pile" => {
            let trace = if gathered.stock_remaining <= 0 {
                "棚腳那堆舊麻袋被剪到只剩爛邊，搬貨人把空出的木箱往內側推。"
            } else {
                "舊麻袋堆薄了一層，旁邊搬貨人也照同一堆料挑能再包貨的布片。"
            };
            format!("你從水尾寮渡頭市集棚腳剪下一卷「{}」收進背包；{trace}", gathered.name)
        }
        "m3_027_plank_wet_rope_coil" => {
            let trace = if gathered.stock_remaining <= 0 {
                "木樁內側只剩被水泡軟的斷繩，船戶把空出的繩釘重新壓緊。"
            } else {
                "濕繩圈少了一道，船戶把剩下能用的繩頭往樁後收。"
            };
            format!(
                "你從踏板木樁內側理出一卷「{}」收進背包；{trace}",
                gathered.name
            )
        }
        _ => format!(
            "你從【{}】理出一份可用材料收進背包。",
            gathered.origin_label
        ),
    }
}

fn m3_027_material_gather_empty_text(object_id: &str) -> &'static str {
    match object_id {
        "m3_027_north_bank_reeds" => {
            "你撥開蘆叢，只摸到被割過後倒伏的濕草；這一帶能束起來的蘆草暫時被人取走，只剩貼著水線的碎葉。"
        }
        "m3_027_market_sackcloth_pile" => {
            "你翻開棚腳舊麻袋，只剩沾泥爛邊和散線；能剪成包貨布的部分已被搬貨人和看貨人取走。"
        }
        "m3_027_plank_wet_rope_coil" => {
            "你摸到木樁內側的繩圈，只剩泡軟斷股；能重新捆箱的麻繩暫時被船戶和搬貨人取走。"
        }
        _ => "你伸手去理那堆料，卻只摸到不成束的濕碎邊。",
    }
}

fn m3_027_scene_object_uses_history(object_id: &str, action: &str) -> bool {
    object_id.starts_with("m3_027_")
        && matches!(
            action,
            "Look"
                | "Gather"
                | "Rest"
                | "BuySoup"
                | "QueueBroth"
                | "CarryCargo"
                | "HelpRope"
                | "DryOff"
                | "WarmHands"
                | "TalkQueue"
                | "QueueFerry"
                | "SellReeds"
                | "SellSackcloth"
                | "SellRope"
                | "BoardFerry"
                | "StepAshore"
                | "CheckCargo"
        )
}

fn m3_027_scene_object_action_denial(
    pid: &str,
    object_id: &str,
    action: &str,
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    match (object_id, action) {
        ("m3_027_hot_soup_stall", "BuySoup") if !m3_027_soup_taken_state_active(prior_states) => {
            let magnesium = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|ch| ch.magnesium)
                .unwrap_or(0);
            if magnesium < M3_027_SOUP_COST {
                Some(
                    "你摸到錢角時停了一下，手邊鎂數不夠買滿碗熱湯；攤主把勺子往桶側一偏，示意想省錢就去等分湯那條短線。",
                )
            } else {
                None
            }
        }
        ("m3_027_market_reed_buyer", "SellReeds")
            if m3_027_reed_market_demand_filled(prior_actions, prior_states) =>
        {
            Some("收牌人把價牌翻到背面，秤邊的貨籃已墊滿；這一輪水尾寮渡頭市集不再收新的蘆草束。")
        }
        ("m3_027_market_reed_buyer", "SellReeds") => {
            let has_reeds = db::get_entity(pid)
                .ok()
                .flatten()
                .is_some_and(|ch| db::inventory_has_item(&ch.inventory, M3_027_REED_ITEM_ID));
            if has_reeds {
                None
            } else {
                Some("收牌人看了看你空著的手，沒有伸秤；水尾寮渡頭市集這一輪只收能立刻墊箱、束貨的蘆草束。")
            }
        }
        ("m3_027_market_reed_buyer", "SellSackcloth")
            if m3_027_sackcloth_demand_filled_state_active(prior_states) =>
        {
            Some("看貨人把包貨單上麻袋布那行按下濕指印；這一輪要包的箱已裹好，不再收新的麻袋布。")
        }
        ("m3_027_market_reed_buyer", "SellSackcloth") => {
            let has_sackcloth = db::get_entity(pid)
                .ok()
                .flatten()
                .is_some_and(|ch| db::inventory_has_item(&ch.inventory, M3_027_SACKCLOTH_ITEM_ID));
            if has_sackcloth {
                None
            } else {
                Some("看貨人掀了掀包貨單，沒有看見能裹箱的麻袋布；空手站在秤邊，只會擋住搬貨人。")
            }
        }
        ("m3_027_market_reed_buyer", "SellRope")
            if m3_027_rope_demand_filled_state_active(prior_states) =>
        {
            Some("收牌人把麻繩那行劃過，踏板口那批箱包已經重新捆緊；這一輪不再收新的麻繩卷。")
        }
        ("m3_027_market_reed_buyer", "SellRope") => {
            let has_rope = db::get_entity(pid)
                .ok()
                .flatten()
                .is_some_and(|ch| db::inventory_has_item(&ch.inventory, M3_027_ROPE_ITEM_ID));
            if has_rope {
                None
            } else {
                Some("收牌人看了看你空著的手，沒有接繩頭；水尾寮渡頭市集這一輪只收能立刻捆箱的麻繩卷。")
            }
        }
        _ => None,
    }
}

fn m3_027_scene_object_followup_event_payload(object_id: &str, action: &str) -> Option<String> {
    match (object_id, action) {
        ("m3_027_wet_bench", "Rest")
        | ("m3_027_hot_soup_stall", "BuySoup")
        | ("m3_027_hot_soup_stall", "QueueBroth")
        | ("m3_027_cargo_pole", "CarryCargo")
        | ("m3_027_boat_rope", "HelpRope")
        | ("m3_027_river_awning", "DryOff")
        | ("m3_027_charcoal_brazier", "WarmHands")
        | ("m3_027_waiting_line_chat", "TalkQueue")
        | ("m3_027_waiting_line_chat", "QueueFerry")
        | ("m3_027_market_reed_buyer", "SellReeds")
        | ("m3_027_market_reed_buyer", "SellSackcloth")
        | ("m3_027_market_reed_buyer", "SellRope")
        | ("m3_027_plank_boarding_marker", "BoardFerry")
        | ("m3_027_south_landing_rope", "StepAshore")
        | ("m3_027_warehouse_tally_board", "CheckCargo") => Some(format!("{object_id}|{action}")),
        _ => None,
    }
}

fn m3_027_scene_object_state_event_payload(
    object_id: &str,
    action: &str,
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    match (object_id, action) {
        ("m3_027_wet_bench", "Rest") if !m3_027_bench_rest_state_active(prior_states) => {
            Some(M3_027_BENCH_REST_STATE)
        }
        ("m3_027_hot_soup_stall", "BuySoup") if !m3_027_soup_taken_state_active(prior_states) => {
            Some(M3_027_SOUP_WARM_STATE)
        }
        ("m3_027_hot_soup_stall", "QueueBroth")
            if !m3_027_soup_taken_state_active(prior_states) =>
        {
            Some(M3_027_FREE_BROTH_STATE)
        }
        ("m3_027_cargo_pole", "CarryCargo") if !m3_027_cargo_help_state_active(prior_states) => {
            Some(M3_027_CARGO_HELP_STATE)
        }
        ("m3_027_boat_rope", "HelpRope")
            if m3_027_cargo_help_state_active(prior_states)
                && !m3_027_rope_help_state_active(prior_states) =>
        {
            Some(M3_027_ROPE_HELP_STATE)
        }
        ("m3_027_river_awning", "DryOff") if !m3_027_awning_dry_state_active(prior_states) => {
            Some(M3_027_AWNING_DRY_STATE)
        }
        ("m3_027_charcoal_brazier", "WarmHands")
            if !m3_027_brazier_warm_state_active(prior_states) =>
        {
            Some(M3_027_BRAZIER_WARM_STATE)
        }
        ("m3_027_waiting_line_chat", "TalkQueue")
            if !m3_027_queue_talk_state_active(prior_states) =>
        {
            Some(M3_027_QUEUE_TALK_STATE)
        }
        ("m3_027_waiting_line_chat", "QueueFerry")
            if !m3_027_queue_ferry_state_active(prior_states) =>
        {
            Some(M3_027_QUEUE_FERRY_STATE)
        }
        ("m3_027_market_reed_buyer", "SellReeds")
            if m3_027_reed_sales_count(prior_actions) + 1
                >= M3_027_REED_MARKET_DEMAND_PER_ROUND
                && !m3_027_reed_demand_filled_state_active(prior_states) =>
        {
            Some(M3_027_REED_DEMAND_FILLED_STATE)
        }
        ("m3_027_market_reed_buyer", "SellReeds")
            if !m3_027_reed_sold_state_active(prior_states) =>
        {
            Some(M3_027_REED_SOLD_STATE)
        }
        ("m3_027_market_reed_buyer", "SellSackcloth")
            if !m3_027_sackcloth_demand_filled_state_active(prior_states) =>
        {
            Some(M3_027_SACKCLOTH_DEMAND_FILLED_STATE)
        }
        ("m3_027_market_reed_buyer", "SellRope")
            if !m3_027_rope_demand_filled_state_active(prior_states) =>
        {
            Some(M3_027_ROPE_DEMAND_FILLED_STATE)
        }
        ("m3_027_plank_boarding_marker", "BoardFerry")
            if m3_027_queue_ferry_state_active(prior_states)
                && !m3_027_board_ferry_state_active(prior_states) =>
        {
            Some(M3_027_BOARD_FERRY_STATE)
        }
        ("m3_027_south_landing_rope", "StepAshore")
            if m3_027_board_ferry_state_active(prior_states)
                && !m3_027_landed_south_state_active(prior_states) =>
        {
            Some(M3_027_LANDED_SOUTH_STATE)
        }
        ("m3_027_warehouse_tally_board", "CheckCargo")
            if m3_027_cargo_help_state_active(prior_states)
                && !m3_027_warehouse_tally_state_active(prior_states) =>
        {
            Some(M3_027_WAREHOUSE_TALLY_STATE)
        }
        _ => None,
    }
}

fn m3_027_scene_object_apply_side_effects(
    pid: &str,
    object_id: &str,
    action: &str,
    state_payload: Option<&str>,
) -> bool {
    match (object_id, action, state_payload) {
        ("m3_027_hot_soup_stall", "BuySoup", Some(M3_027_SOUP_WARM_STATE)) => {
            let _ = db::add_magnesium(pid, -M3_027_SOUP_COST);
            true
        }
        ("m3_027_cargo_pole", "CarryCargo", Some(M3_027_CARGO_HELP_STATE)) => {
            let _ = db::add_magnesium(pid, M3_027_CARGO_PAY);
            true
        }
        ("m3_027_market_reed_buyer", "SellReeds", Some(state))
            if state == M3_027_REED_SOLD_STATE || state == M3_027_REED_DEMAND_FILLED_STATE =>
        {
            let _ = db::remove_from_inventory(pid, M3_027_REED_ITEM_ID, 1);
            let _ = db::add_magnesium(pid, M3_027_REED_SELL_PAY);
            true
        }
        (
            "m3_027_market_reed_buyer",
            "SellSackcloth",
            Some(M3_027_SACKCLOTH_DEMAND_FILLED_STATE),
        ) => {
            let _ = db::remove_from_inventory(pid, M3_027_SACKCLOTH_ITEM_ID, 1);
            let _ = db::add_magnesium(pid, M3_027_SACKCLOTH_SELL_PAY);
            true
        }
        ("m3_027_market_reed_buyer", "SellRope", Some(M3_027_ROPE_DEMAND_FILLED_STATE)) => {
            let _ = db::remove_from_inventory(pid, M3_027_ROPE_ITEM_ID, 1);
            let _ = db::add_magnesium(pid, M3_027_ROPE_SELL_PAY);
            true
        }
        ("m3_027_warehouse_tally_board", "CheckCargo", Some(M3_027_WAREHOUSE_TALLY_STATE)) => {
            let _ = db::add_magnesium(pid, M3_027_CARGO_TALLY_PAY);
            true
        }
        _ => false,
    }
}

fn m3_027_scene_object_state_action_response(
    object_id: &str,
    action: &str,
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<&'static str> {
    match (object_id, action) {
        ("m3_027_wet_bench", "Rest") if m3_027_bench_rest_state_active(prior_states) => {
            Some("剛讓出的乾位已被別人的包袱佔回去；你再靠近時，長凳只剩濕冷那一段。")
        }
        ("m3_027_hot_soup_stall", "BuySoup" | "QueueBroth")
            if m3_027_soup_taken_state_active(prior_states) =>
        {
            Some("攤主把勺子靠回鍋沿，說這一輪已經輪過你；鍋底的熱氣留給下一個被河風吹透的人。")
        }
        ("m3_027_cargo_pole", "CarryCargo") if m3_027_cargo_help_state_active(prior_states) => {
            Some("你剛扛過的那副貨擔已經停在踏板邊，市集的人把下一擔交給熟手，沒有再把肩位讓出來。")
        }
        ("m3_027_boat_rope", "HelpRope") if m3_027_rope_help_state_active(prior_states) => {
            Some("船戶看見你已經搭過一手，只讓你留在踏板旁的窄位等船靠穩；繩尾這回交給後面的人。")
        }
        ("m3_027_boat_rope", "HelpRope") if !m3_027_cargo_help_state_active(prior_states) => {
            Some("船戶把繩尾往掌心一收，先指向踏板邊的貨擔；船還沒靠穩，空手擠上去只會擋路。")
        }
        ("m3_027_river_awning", "DryOff") if m3_027_awning_dry_state_active(prior_states) => {
            Some("棚下剛被你晾過的位置已經換成別人的濕包袱；再擠進去，只會把衣角重新蹭濕。")
        }
        ("m3_027_charcoal_brazier", "WarmHands")
            if m3_027_brazier_warm_state_active(prior_states) =>
        {
            Some("炭火盆邊的人看見你剛烘過手，便把下一個冷得發抖的候渡者推近火口。")
        }
        ("m3_027_waiting_line_chat", "TalkQueue")
            if m3_027_queue_talk_state_active(prior_states) =>
        {
            Some("隊列裡能問的船期和湯價你已聽過一輪；再追著問，只會讓前後的人避開你的眼神。")
        }
        ("m3_027_waiting_line_chat", "QueueFerry")
            if m3_027_queue_ferry_state_active(prior_states) =>
        {
            Some("你已在隊列折角裡，前後的人都按這條線往踏板等；再擠一次，只會把剛認住的位置弄亂。")
        }
        ("m3_027_market_reed_buyer", "SellReeds")
            if m3_027_reed_demand_filled_state_active(prior_states) =>
        {
            Some("收牌人把價牌翻到背面，秤邊的貨籃已墊滿；這一輪水尾寮渡頭市集不再收新的蘆草束。")
        }
        ("m3_027_market_reed_buyer", "SellReeds")
            if m3_027_reed_sales_count(prior_actions) + 1
                >= M3_027_REED_MARKET_DEMAND_PER_ROUND =>
        {
            Some(
                "你把蘆草交到秤邊，收牌人掂過濕重後數出幾枚鎂；這一束被抽去墊住最後一只貨籃，價牌隨即翻到背面。",
            )
        }
        ("m3_027_market_reed_buyer", "SellSackcloth")
            if m3_027_sackcloth_demand_filled_state_active(prior_states) =>
        {
            Some("看貨人把包貨單上麻袋布那行按下濕指印；這一輪要包的箱已裹好，不再收新的麻袋布。")
        }
        ("m3_027_market_reed_buyer", "SellSackcloth") => Some(
            "你把麻袋布交到包貨單旁，看貨人沿布邊摸過一圈後數出幾枚鎂；搬貨人立刻拿它裹住一只磨角木箱，麻袋布那行被按下濕指印。",
        ),
        ("m3_027_market_reed_buyer", "SellRope")
            if m3_027_rope_demand_filled_state_active(prior_states) =>
        {
            Some("收牌人把麻繩那行劃過，踏板口那批箱包已經重新捆緊；這一輪不再收新的麻繩卷。")
        }
        ("m3_027_market_reed_buyer", "SellRope") => Some(
            "你把麻繩卷放到濕木板上，收牌人拉開繩頭試了試韌度後數出幾枚鎂；踏板口那排箱包很快被重新捆緊，麻繩那行被劃掉。",
        ),
        ("m3_027_plank_boarding_marker", "BoardFerry")
            if m3_027_board_ferry_state_active(prior_states) =>
        {
            Some("船戶已把你放進踏板內側，再回頭擠木牌只會擋住下一個被點到的人。")
        }
        ("m3_027_plank_boarding_marker", "BoardFerry")
            if !m3_027_queue_ferry_state_active(prior_states) =>
        {
            Some("船戶把手掌壓在木牌上，示意你先回北岸隊列；沒被隊列認住的人不能直接擠上踏板。")
        }
        ("m3_027_south_landing_rope", "StepAshore")
            if m3_027_landed_south_state_active(prior_states) =>
        {
            Some("你已踩過那道濕泥繩位，後面的人沿同一條痕上坡；再退回去只會把上岸線堵住。")
        }
        ("m3_027_south_landing_rope", "StepAshore")
            if !m3_027_board_ferry_state_active(prior_states) =>
        {
            Some("上岸繩那頭還沒輪到你，南岸的人只讓剛從踏板內側下來的人先踩穩。")
        }
        ("m3_027_warehouse_tally_board", "CheckCargo")
            if m3_027_warehouse_tally_state_active(prior_states) =>
        {
            Some("記牌人已把那副貨擔劃進倉棚短冊，尾款也結過；牌邊只剩下一輪貨號。")
        }
        ("m3_027_warehouse_tally_board", "CheckCargo")
            if !m3_027_cargo_help_state_active(prior_states) =>
        {
            Some("記牌人翻了翻短冊，沒有找到你剛送到踏板邊的貨擔痕；沒清過貨的人領不到這行尾款。")
        }
        _ => None,
    }
}

fn m3_027_scene_object_look_echo(
    player_room: &str,
    object_id: &str,
    now: i64,
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<String> {
    if let Some(echo) =
        m3_027_material_object_look_echo(player_room, object_id, now, prior_actions, prior_states)
    {
        return Some(echo);
    }
    if let Some(echo) = m3_027_scene_object_priority_echo(object_id, prior_states) {
        return Some(echo);
    }
    for state in prior_states.iter().rev() {
        let echo = match state.as_str() {
            M3_027_REED_GATHERED_STATE => m3_027_reed_gathered_state_echo(object_id),
            M3_027_SACKCLOTH_GATHERED_STATE => m3_027_sackcloth_gathered_state_echo(object_id),
            M3_027_ROPE_GATHERED_STATE => m3_027_rope_gathered_state_echo(object_id),
            M3_027_BENCH_REST_STATE => m3_027_bench_rest_state_echo(object_id),
            M3_027_SOUP_WARM_STATE => m3_027_soup_warm_state_echo(object_id),
            M3_027_FREE_BROTH_STATE => m3_027_free_broth_state_echo(object_id),
            M3_027_CARGO_HELP_STATE => m3_027_cargo_help_state_echo(object_id),
            M3_027_ROPE_HELP_STATE => m3_027_rope_help_state_echo(object_id),
            M3_027_REED_SOLD_STATE => m3_027_reed_sold_state_echo(object_id),
            M3_027_REED_DEMAND_FILLED_STATE => m3_027_reed_demand_filled_state_echo(object_id),
            M3_027_SACKCLOTH_DEMAND_FILLED_STATE => {
                m3_027_sackcloth_demand_filled_state_echo(object_id)
            }
            M3_027_ROPE_DEMAND_FILLED_STATE => m3_027_rope_demand_filled_state_echo(object_id),
            M3_027_BOARD_FERRY_STATE => m3_027_board_ferry_state_echo(object_id),
            M3_027_LANDED_SOUTH_STATE => m3_027_landed_south_state_echo(object_id),
            M3_027_WAREHOUSE_TALLY_STATE => m3_027_warehouse_tally_state_echo(object_id),
            M3_027_AWNING_DRY_STATE => m3_027_awning_dry_state_echo(object_id),
            M3_027_BRAZIER_WARM_STATE => m3_027_brazier_warm_state_echo(object_id),
            M3_027_QUEUE_TALK_STATE => m3_027_queue_talk_state_echo(object_id),
            M3_027_QUEUE_FERRY_STATE => m3_027_queue_ferry_state_echo(object_id),
            _ => None,
        };
        if echo.is_some() {
            return echo.map(str::to_string);
        }
    }
    None
}

fn m3_027_material_object_look_echo(
    player_room: &str,
    object_id: &str,
    now: i64,
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<String> {
    match object_id {
        "m3_027_north_bank_reeds" => {
            let source = materials::material_source_stock_for_room(player_room, now)?;
            if source.stock <= 0 {
                Some(format!(
                    "{}被割到只剩貼水線的短葉，能束成「{}」的長莖要等下一輪水氣和人手重新補上。",
                    source.origin_label, source.material_name
                ))
            } else if m3_027_reed_gathered_state_active(prior_states)
                || source.stock * 3 <= source.stock_max
            {
                Some(format!(
                    "{}被割矮了一片，還看得到能束成「{}」的長莖；前面採過的人也只沿著同一條水線取草。",
                    source.origin_label, source.material_name
                ))
            } else {
                None
            }
        }
        "m3_027_market_sackcloth_pile" => {
            let source = materials::material_source_stock_for_room(player_room, now)?;
            if source.stock <= 0 {
                Some(format!(
                    "{}只剩沾泥爛邊，能剪成「{}」的布片暫時被搬貨人取走。",
                    source.origin_label, source.material_name
                ))
            } else if m3_027_sackcloth_gathered_state_active(prior_states)
                || source.stock * 3 <= source.stock_max
            {
                Some(format!(
                    "{}薄了一層，仍能翻出幾片「{}」；前面的搬貨人也只挑能再包貨的布。",
                    source.origin_label, source.material_name
                ))
            } else {
                None
            }
        }
        "m3_027_plank_wet_rope_coil" => {
            let source = materials::material_source_stock_for_room(player_room, now)?;
            if source.stock <= 0 {
                Some(format!(
                    "{}只剩泡軟斷股，能捲成「{}」的繩材暫時被船戶和搬貨人取走。",
                    source.origin_label, source.material_name
                ))
            } else if m3_027_rope_gathered_state_active(prior_states)
                || source.stock * 3 <= source.stock_max
            {
                Some(format!(
                    "{}少了一圈，仍看得到能捲成「{}」的繩材；船戶也照同一堆料補踏板邊的捆貨繩。",
                    source.origin_label, source.material_name
                ))
            } else {
                None
            }
        }
        "m3_027_market_reed_buyer" => m3_027_pack_order_look_echo(prior_actions, prior_states),
        _ => None,
    }
}

fn m3_027_pack_order_look_echo(
    prior_actions: &[String],
    prior_states: &[String],
) -> Option<String> {
    let reed_filled = m3_027_reed_market_demand_filled(prior_actions, prior_states);
    let sackcloth_filled = m3_027_sackcloth_demand_filled_state_active(prior_states);
    let rope_filled = m3_027_rope_demand_filled_state_active(prior_states);
    let npc_delivered_reeds = m3_029::pack_runner_delivered_reeds(prior_states);
    if reed_filled && sackcloth_filled && rope_filled {
        return Some("水尾寮渡頭市集包貨單三行都被按下濕指印：蘆草已墊箱，麻袋布已包貨，麻繩也把踏板口那批箱包重新捆緊；看貨人把秤邊讓給下一輪零散貨。".to_string());
    }
    let mut filled = Vec::new();
    let mut waiting = Vec::new();
    if reed_filled {
        filled.push("蘆草墊箱");
    } else if m3_027_reed_sales_count(prior_actions) > 0
        || m3_027_reed_sold_state_active(prior_states)
    {
        if npc_delivered_reeds {
            filled.push("搬貨人先交的一束蘆草已墊箱");
        } else {
            filled.push("一部分蘆草已墊箱");
        }
        waiting.push("蘆草還未收滿");
    } else {
        waiting.push("蘆草墊箱");
    }
    if sackcloth_filled {
        filled.push("麻袋布包貨");
    } else {
        waiting.push("麻袋布包貨");
    }
    if rope_filled {
        filled.push("麻繩捆箱");
    } else {
        waiting.push("麻繩捆箱");
    }
    if filled.is_empty() {
        return None;
    }
    if npc_delivered_reeds {
        Some(format!(
            "水尾寮渡頭市集包貨單上{}已被收進秤邊，{}仍壓在單子外側；搬貨人和玩家都照同一張單交料，誰先補上就會改掉這行需求。",
            filled.join("、"),
            waiting.join("、")
        ))
    } else {
        Some(format!(
            "水尾寮渡頭市集包貨單上{}已被收進秤邊，{}仍壓在單子外側；看貨人和搬貨人都照這張單抽料。",
            filled.join("、"),
            waiting.join("、")
        ))
    }
}

fn m3_027_scene_object_priority_echo(object_id: &str, prior_states: &[String]) -> Option<String> {
    match object_id {
        "m3_027_plank_boarding_marker" => {
            if m3_027_board_ferry_state_active(prior_states) {
                m3_027_board_ferry_state_echo(object_id).map(str::to_string)
            } else if m3_027_queue_ferry_state_active(prior_states) {
                m3_027_queue_ferry_state_echo(object_id).map(str::to_string)
            } else if m3_027_cargo_help_state_active(prior_states) {
                m3_027_cargo_help_state_echo(object_id).map(str::to_string)
            } else {
                None
            }
        }
        "m3_027_south_landing_rope" => {
            if m3_027_landed_south_state_active(prior_states) {
                m3_027_landed_south_state_echo(object_id).map(str::to_string)
            } else if m3_027_board_ferry_state_active(prior_states) {
                m3_027_board_ferry_state_echo(object_id).map(str::to_string)
            } else {
                None
            }
        }
        "m3_027_warehouse_tally_board" => {
            if let Some(echo) = m3_027_pack_order_state_echo(object_id, prior_states) {
                Some(echo)
            } else if let Some(echo) = m3_029::shared_economy_echo(object_id, prior_states) {
                Some(echo)
            } else if m3_027_warehouse_tally_state_active(prior_states) {
                m3_027_warehouse_tally_state_echo(object_id).map(str::to_string)
            } else if m3_027_cargo_help_state_active(prior_states) {
                m3_027_cargo_help_state_echo(object_id).map(str::to_string)
            } else if m3_027_landed_south_state_active(prior_states) {
                m3_027_landed_south_state_echo(object_id).map(str::to_string)
            } else if m3_027_reed_demand_filled_state_active(prior_states) {
                m3_027_reed_demand_filled_state_echo(object_id).map(str::to_string)
            } else if m3_027_reed_sold_state_active(prior_states) {
                m3_027_reed_sold_state_echo(object_id).map(str::to_string)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn m3_027_pack_order_state_echo(object_id: &str, prior_states: &[String]) -> Option<String> {
    if !matches!(
        object_id,
        "m3_027_market_reed_buyer" | "m3_027_warehouse_tally_board"
    ) {
        return None;
    }
    let reed_filled = m3_027_reed_demand_filled_state_active(prior_states);
    let sackcloth_filled = m3_027_sackcloth_demand_filled_state_active(prior_states);
    let rope_filled = m3_027_rope_demand_filled_state_active(prior_states);
    if !(reed_filled || sackcloth_filled || rope_filled) {
        return None;
    }
    if object_id == "m3_027_warehouse_tally_board" {
        if reed_filled && sackcloth_filled && rope_filled {
            return Some("水尾寮渡頭市集送來的箱包已在倉棚門邊分好：濕蘆草墊住箱角，麻袋布裹住磨邊，麻繩把同一批零散貨重新捆成一排。".to_string());
        }
        let mut parts = Vec::new();
        if reed_filled {
            parts.push("蘆草墊箱");
        }
        if sackcloth_filled {
            parts.push("麻袋布包貨");
        }
        if rope_filled {
            parts.push("麻繩捆箱");
        }
        return Some(format!(
            "水尾寮渡頭市集那邊已把{}送到倉棚門前，記牌人把這批零散貨壓進同一行，不再像散貨那樣堆在泥邊。",
            parts.join("、")
        ));
    }
    if reed_filled && sackcloth_filled && rope_filled {
        Some("水尾寮渡頭市集包貨單三行都被按下濕指印，秤邊只剩下一輪零散貨要等。".to_string())
    } else {
        None
    }
}

fn m3_027_reed_gathered_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_north_bank_reeds" => {
            Some("你剛割過的蘆叢矮下一片，斷口旁另有水珠順著葉脈往泥裡落。")
        }
        "m3_027_market_reed_buyer" => {
            Some("收牌人看見你手上的濕蘆草，先把秤邊那塊木板空出來，等能墊箱的草束上秤。")
        }
        _ => None,
    }
}

fn m3_027_sackcloth_gathered_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_sackcloth_pile" => {
            Some("你剛剪過的舊麻袋堆薄了一層，棚腳露出被踩濕的爛邊和散線。")
        }
        "m3_027_market_reed_buyer" => {
            Some("看貨人看見你背包裡露出的麻袋布邊，先把包貨單那行壓到秤邊。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("倉棚前場有人指向水尾寮渡頭市集棚腳，說那邊剛剪出的麻袋布能先裹磨角木箱。")
        }
        _ => None,
    }
}

fn m3_027_rope_gathered_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_plank_wet_rope_coil" => {
            Some("你剛理過的濕繩圈少了一道，木樁內側只剩船戶重新壓緊的繩頭。")
        }
        "m3_027_market_reed_buyer" => {
            Some("收牌人看見你手邊的麻繩卷，先把捆箱那行從包貨單下抽出來。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("倉棚門邊有人等踏板口補來麻繩，幾只箱包暫時只靠舊結頂著。")
        }
        _ => None,
    }
}

fn m3_027_bench_rest_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_wet_bench" => {
            Some("你剛坐過的乾位已被包袱佔回去，凳腳旁還留著一點被鞋底蹭開的濕泥。")
        }
        _ => None,
    }
}

fn m3_027_soup_warm_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_hot_soup_stall" => {
            Some("你的空碗還倒扣在攤邊，鍋裡熱氣薄了一層，攤主把收過的錢角壓進濕布底下。")
        }
        "m3_027_wet_bench" => Some("掌心的熱湯氣還沒散，你再看長凳時，河風不像先前那麼割手。"),
        "m3_027_boat_rope" => Some("湯碗留下的熱意還在指節上，你抓繩時不至於立刻被濕繩冰住。"),
        _ => None,
    }
}

fn m3_027_free_broth_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_hot_soup_stall" => {
            Some("你剛等過的桶側短線又接上一小截人，攤主把半碗淡湯留給沒錢買滿碗的候渡者。")
        }
        "m3_027_waiting_line_chat" => Some("隊列裡有人看見你拿過分湯，說話時把湯價壓低了一點。"),
        _ => None,
    }
}

fn m3_027_cargo_help_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_cargo_pole" => {
            Some("你剛扛走的一副貨擔讓泥地空出一道深痕，剩下的麻袋被水尾寮渡頭市集的人往棚腳收。")
        }
        "m3_027_plank_boarding_marker" => {
            Some("你剛清出的貨擔痕停在踏板木牌旁，船戶不用再把上船線和搬貨線分開喊。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("踏板那頭清出的貨擔已被記牌人看見，短冊空出一行等人來對貨。")
        }
        "m3_027_boat_rope" => {
            Some("船戶看見你清過踏板邊的貨擔，手掌在繩上停了一下，像是在等你靠近搭手。")
        }
        _ => None,
    }
}

fn m3_027_rope_help_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_boat_rope" => {
            Some("你剛收過的繩圈被壓在木樁內側，船戶把那段窄位留著，等下一次船腹貼岸。")
        }
        "m3_027_cargo_pole" => {
            Some("貨擔旁的人看見你幫過船繩，說話時把踏板邊那條窄路給你留出半步。")
        }
        _ => None,
    }
}

fn m3_027_awning_dry_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_river_awning" => {
            Some("你剛站過的棚下位置被濕包袱填上，舊帆布邊還滴著慢慢變細的水線。")
        }
        "m3_027_wet_bench" => Some("衣角乾了一些，你再看長凳時，濕痕不再像先前那樣貼著膝蓋。"),
        _ => None,
    }
}

fn m3_027_brazier_warm_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_charcoal_brazier" => {
            Some("炭火盆旁你剛讓出的半掌空位被下一個人補上，煙氣貼著他的袖口往外散。")
        }
        "m3_027_boat_rope" => Some("指節被炭火烘過後，再碰濕繩時不至於立刻僵住。"),
        _ => None,
    }
}

fn m3_027_queue_talk_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_waiting_line_chat" => {
            Some("你剛問過的那幾個候渡者換成低聲閒談，船期和湯價已在隊列裡傳開。")
        }
        "m3_027_boat_rope" => {
            Some("隊列裡聽來的船期讓你看船戶手勢時更快分清哪一次靠岸能上前搭手。")
        }
        _ => None,
    }
}

fn m3_027_queue_ferry_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_waiting_line_chat" => {
            Some("你排進去的那段隊列被後面的人補緊，候渡者也按同一條木牌線往踏板口慢慢挪。")
        }
        "m3_027_boat_rope" => {
            Some("船戶看見你已排進隊列，手掌在繩上停了一下，只等下一次踏板前清出縫。")
        }
        "m3_027_hot_soup_stall" => {
            Some("你在隊列裡的位置被人認住後，湯攤那側的人不再把你擠回桶邊短線。")
        }
        "m3_027_plank_boarding_marker" => {
            Some("船戶已在木牌內側認住你的隊列位置，踏板口留出一道等你上前的窄縫。")
        }
        _ => None,
    }
}

fn m3_027_reed_sold_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_reed_buyer" => {
            Some("你交出的蘆草已被抽去墊在貨籃角落，收牌人把剛數過的鎂收進木牌下，價牌還沒有翻面。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("水尾寮渡頭市集那邊新墊的蘆草跟著貨籃進到棚門旁，木箱角不再直接磨在泥地上。")
        }
        _ => None,
    }
}

fn m3_027_reed_demand_filled_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_reed_buyer" => {
            Some("價牌已被翻到背面，秤邊貨籃都墊上濕蘆草；收牌人把秤砣壓住，不再接新的草束。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("水尾寮渡頭市集收滿的濕蘆草已墊進貨籃和木箱角，倉棚前場少了一點箱底磨泥的聲音。")
        }
        _ => None,
    }
}

fn m3_027_sackcloth_demand_filled_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_reed_buyer" => {
            Some("包貨單上麻袋布那行已按下濕指印，磨角木箱被粗布裹住，搬貨人不再催新的布卷。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("市集送來的麻袋布已裹住磨角木箱，倉棚門邊少了木角刮袋的聲音。")
        }
        "m3_027_market_sackcloth_pile" => {
            Some("包貨那批布已被收走，舊麻袋堆旁只剩等下一輪挑布的人。")
        }
        _ => None,
    }
}

fn m3_027_rope_demand_filled_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_reed_buyer" => {
            Some("包貨單上麻繩那行已被劃過，踏板口那批箱包重新捆緊，收牌人不再接新的繩卷。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("踏板口補來的麻繩把零散貨重新捆成一排，倉棚門邊的箱包不再散開。")
        }
        "m3_027_plank_wet_rope_coil" => {
            Some("剛被收走的麻繩已用在踏板口那批箱包上，木樁內側只留給船戶的繩尾。")
        }
        _ => None,
    }
}

fn m3_027_board_ferry_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_plank_boarding_marker" => {
            Some("你剛被放進踏板內側，木牌前的人群收回半步，等下一個被船戶點到。")
        }
        "m3_027_south_landing_rope" => {
            Some("對岸繩位已有人替你讓出上岸腳點，泥坡邊的包袱先往內側挪開。")
        }
        _ => None,
    }
}

fn m3_027_landed_south_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_south_landing_rope" => {
            Some("你剛踩出的泥痕被後面的人沿著走，南岸上坡那條線沒有再散開。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("南岸剛上來的人流分出一股往倉棚前場，記牌人把短冊往棚門邊推近。")
        }
        _ => None,
    }
}

fn m3_027_warehouse_tally_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_warehouse_tally_board" => {
            Some("你對過的那行貨擔已被劃進短冊，牌角留下剛按過的濕指印。")
        }
        "m3_027_market_reed_buyer" => {
            Some("倉棚那邊收走一行貨後，市集價牌旁少了一次催貨聲，收牌人把秤砣壓得更穩。")
        }
        _ => None,
    }
}

fn m3_027_bench_rest_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_BENCH_REST_STATE)
}

fn m3_027_reed_gathered_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_REED_GATHERED_STATE)
}

fn m3_027_sackcloth_gathered_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_SACKCLOTH_GATHERED_STATE)
}

fn m3_027_rope_gathered_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_ROPE_GATHERED_STATE)
}

fn m3_027_soup_warm_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_SOUP_WARM_STATE)
}

fn m3_027_free_broth_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_FREE_BROTH_STATE)
}

fn m3_027_soup_taken_state_active(prior_states: &[String]) -> bool {
    m3_027_soup_warm_state_active(prior_states) || m3_027_free_broth_state_active(prior_states)
}

fn m3_027_cargo_help_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_CARGO_HELP_STATE)
}

fn m3_027_rope_help_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_ROPE_HELP_STATE)
}

fn m3_027_reed_sold_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_REED_SOLD_STATE)
}

fn m3_027_reed_demand_filled_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_REED_DEMAND_FILLED_STATE)
}

fn m3_027_sackcloth_demand_filled_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_SACKCLOTH_DEMAND_FILLED_STATE)
}

fn m3_027_rope_demand_filled_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_ROPE_DEMAND_FILLED_STATE)
}

fn m3_027_reed_sales_count(prior_actions: &[String]) -> usize {
    prior_actions
        .iter()
        .filter(|payload| payload.as_str() == "m3_027_market_reed_buyer|SellReeds")
        .count()
}

fn m3_027_reed_market_demand_filled(prior_actions: &[String], prior_states: &[String]) -> bool {
    m3_027_reed_demand_filled_state_active(prior_states)
        || m3_027_reed_sales_count(prior_actions) >= M3_027_REED_MARKET_DEMAND_PER_ROUND
}

pub(crate) fn m3_027_reed_batch_sale_count(
    available_reeds: i32,
    prior_actions: &[String],
    prior_states: &[String],
) -> usize {
    if m3_027_reed_market_demand_filled(prior_actions, prior_states) {
        return 0;
    }
    let sold = m3_027_reed_sales_count(prior_actions);
    let remaining = M3_027_REED_MARKET_DEMAND_PER_ROUND.saturating_sub(sold);
    (available_reeds.max(0) as usize).min(remaining)
}

fn player_inventory_item_quantity(pid: &str, item_id: &str) -> i32 {
    let Ok(Some(ch)) = db::get_entity(pid) else {
        return 0;
    };
    #[derive(serde::Deserialize)]
    struct Entry {
        item_id: String,
        #[serde(default)]
        qty: i32,
    }
    let entries: Vec<Entry> = serde_json::from_str(&ch.inventory).unwrap_or_default();
    entries
        .into_iter()
        .filter(|e| e.item_id == item_id)
        .map(|e| e.qty.max(0))
        .sum()
}

fn m3_027_board_ferry_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_BOARD_FERRY_STATE)
}

fn m3_027_landed_south_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_LANDED_SOUTH_STATE)
}

fn m3_027_warehouse_tally_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_WAREHOUSE_TALLY_STATE)
}

fn m3_027_awning_dry_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_AWNING_DRY_STATE)
}

fn m3_027_brazier_warm_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_BRAZIER_WARM_STATE)
}

fn m3_027_queue_talk_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_QUEUE_TALK_STATE)
}

fn m3_027_queue_ferry_state_active(prior_states: &[String]) -> bool {
    prior_states
        .iter()
        .any(|payload| payload == M3_027_QUEUE_FERRY_STATE)
}

fn recent_scene_object_action_payloads(pid: &str, now: i64) -> Vec<String> {
    event::events_in_range(pid, now.saturating_sub(6 * 60 * 60), now)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.event_type == types::SCENE_OBJECT_ACTION)
        .map(|row| row.payload)
        .collect()
}

fn recent_scene_object_state_payloads(pid: &str, now: i64) -> Vec<String> {
    event::events_in_range(pid, now.saturating_sub(6 * 60 * 60), now)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.event_type == types::SCENE_OBJECT_STATE)
        .map(|row| row.payload)
        .collect()
}

fn object_followup_buttons(
    obj: &crate::model::RoomObject,
    player_room: &str,
    action: &str,
    prior_scene_actions: &[String],
    prior_scene_states: &[String],
) -> (Vec<String>, String) {
    let mut others: Vec<String> = obj
        .sockets
        .iter()
        .filter(|s| *s != action)
        .cloned()
        .collect();
    m3_020_filter_scene_object_buttons(
        &obj.id,
        &mut others,
        prior_scene_actions,
        prior_scene_states,
    );
    m3_027_filter_scene_object_buttons(&obj.id, &mut others, prior_scene_states);
    let mut move_target_id = String::new();
    if action == "Look" && others.is_empty() && object_response(obj, "Look").is_empty() {
        let Ok(objs) = db::get_objects_in_room(player_room) else {
            return (others, move_target_id);
        };
        let idx = objs.iter().position(|o| o.id == obj.id);
        if let Some(i) = idx {
            for range in [(i + 1..objs.len()), (0..i)] {
                for j in range {
                    let o = &objs[j];
                    if !o.move_to_room_id.is_empty() && object_has_socket(o, "Move") {
                        move_target_id.clone_from(&o.id);
                        break;
                    }
                }
                if !move_target_id.is_empty() {
                    break;
                }
            }
        } else {
            for o in &objs {
                if !o.move_to_room_id.is_empty() && object_has_socket(o, "Move") {
                    move_target_id.clone_from(&o.id);
                    break;
                }
            }
        }
        if !move_target_id.is_empty() {
            others = vec!["Move".into()];
        }
    }
    (others, move_target_id)
}

fn m3_020_filter_scene_object_buttons(
    object_id: &str,
    actions: &mut Vec<String>,
    prior_scene_actions: &[String],
    prior_scene_states: &[String],
) {
    match object_id {
        "m3_020_mint_half_open_door" => {
            let can_queue = m3_020_waiting_line_has_been_used(prior_scene_actions)
                && !m3_020_mint_queue_state_active(prior_scene_states);
            if !can_queue {
                actions.retain(|action| action != "Queue");
            }
        }
        "m3_020_hire_short_work_boards" => {
            let can_accept_job = m3_020_short_work_board_has_been_read(prior_scene_actions)
                && !m3_020_short_work_state_active(prior_scene_states);
            if !can_accept_job {
                actions.retain(|action| action != "AcceptJob");
            }
        }
        "m3_020_stone_gate_pass_tags" => {
            let can_join_check = m3_020_pass_tags_have_been_read(prior_scene_actions)
                && !m3_020_pass_check_state_active(prior_scene_states);
            if !can_join_check {
                actions.retain(|action| action != "JoinCheck");
            }
        }
        "m3_020_council_notice_strip" => {
            let can_register_complaint = m3_020_council_notice_has_been_read(prior_scene_actions)
                && !m3_020_council_complaint_state_active(prior_scene_states);
            if !can_register_complaint {
                actions.retain(|action| action != "RegisterComplaint");
            }
        }
        _ => {}
    }
}

fn m3_027_filter_scene_object_buttons(
    object_id: &str,
    actions: &mut Vec<String>,
    prior_scene_states: &[String],
) {
    match object_id {
        "m3_027_wet_bench" if m3_027_bench_rest_state_active(prior_scene_states) => {
            actions.retain(|action| action != "Rest");
        }
        "m3_027_hot_soup_stall" if m3_027_soup_taken_state_active(prior_scene_states) => {
            actions.retain(|action| action != "BuySoup" && action != "QueueBroth");
        }
        "m3_027_cargo_pole" if m3_027_cargo_help_state_active(prior_scene_states) => {
            actions.retain(|action| action != "CarryCargo");
        }
        "m3_027_boat_rope" if m3_027_rope_help_state_active(prior_scene_states) => {
            actions.retain(|action| action != "HelpRope");
        }
        "m3_027_river_awning" if m3_027_awning_dry_state_active(prior_scene_states) => {
            actions.retain(|action| action != "DryOff");
        }
        "m3_027_charcoal_brazier" if m3_027_brazier_warm_state_active(prior_scene_states) => {
            actions.retain(|action| action != "WarmHands");
        }
        "m3_027_waiting_line_chat" => {
            if m3_027_queue_talk_state_active(prior_scene_states) {
                actions.retain(|action| action != "TalkQueue");
            }
            if m3_027_queue_ferry_state_active(prior_scene_states) {
                actions.retain(|action| action != "QueueFerry");
            }
        }
        "m3_027_market_reed_buyer" => {
            if m3_027_reed_demand_filled_state_active(prior_scene_states) {
                actions.retain(|action| action != "SellReeds");
            }
            if m3_027_sackcloth_demand_filled_state_active(prior_scene_states) {
                actions.retain(|action| action != "SellSackcloth");
            }
            if m3_027_rope_demand_filled_state_active(prior_scene_states) {
                actions.retain(|action| action != "SellRope");
            }
        }
        "m3_027_plank_boarding_marker" => {
            let can_board = m3_027_queue_ferry_state_active(prior_scene_states)
                && !m3_027_board_ferry_state_active(prior_scene_states);
            if !can_board {
                actions.retain(|action| action != "BoardFerry");
            }
        }
        "m3_027_south_landing_rope" => {
            let can_step_ashore = m3_027_board_ferry_state_active(prior_scene_states)
                && !m3_027_landed_south_state_active(prior_scene_states);
            if !can_step_ashore {
                actions.retain(|action| action != "StepAshore");
            }
        }
        "m3_027_warehouse_tally_board" => {
            let can_check_cargo = m3_027_cargo_help_state_active(prior_scene_states)
                && !m3_027_warehouse_tally_state_active(prior_scene_states);
            if !can_check_cargo {
                actions.retain(|action| action != "CheckCargo");
            }
        }
        _ => {}
    }
}

fn apply_object_move(
    conn: &WsConnection,
    pid: &str,
    _player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
) -> bool {
    if action != "Move" || obj.move_to_room_id.is_empty() {
        return true;
    }
    let Some(coord) = parse_grid_room_id(&obj.move_to_room_id) else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    let gh = current_game_hour(&conn.cfg);
    let Ok(view_opt) = game::get_grid_room_view(pid, coord.x, coord.y, gh) else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    let Some(view) = view_opt else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    if set_entity_grid(pid, coord.x, coord.y).is_err() {
        conn.send_error(gametext::client("move_failed"));
        return false;
    }
    let Some(session) = conn.sessions.get(pid) else {
        conn.send_error(gametext::client("move_failed"));
        return false;
    };
    send_room_view_to_session(&session, &view, pid, &conn.cfg);
    let rid = crate::grid::grid_room_id_from_coord(coord.x, coord.y);
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: pid.to_string(),
        room_id: rid.clone(),
        room_name: view.room.name.clone(),
    };
    if let Ok(bytes) = serde_json::to_vec(&moved) {
        conn.hub.broadcast(bytes);
    }
    refresh_room_views_for_room(&conn.sessions, &conn.cfg, &rid);
    crate::server::handler::emit_presence_reflex_on_arrival(conn, pid, &rid);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m3_020_followup_event_payloads_are_specific() {
        assert_eq!(
            m3_020_scene_object_followup_event_payload("m3_020_flagstone_waiting_lines", "Use"),
            Some("m3_020_flagstone_waiting_lines|Use".to_string())
        );
        assert_eq!(
            m3_020_scene_object_followup_event_payload("m3_020_flagstone_waiting_lines", "Look"),
            None
        );
        assert_eq!(
            m3_020_scene_object_followup_event_payload("ordinary_door", "Open"),
            None
        );
    }

    #[test]
    fn m3_020_queue_state_requires_waiting_line() {
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_mint_half_open_door",
                "Queue",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_mint_half_open_door",
                "Queue",
                &["m3_020_flagstone_waiting_lines|Use".to_string()],
                &[]
            ),
            Some(M3_020_MINT_QUEUE_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_mint_half_open_door",
                "Queue",
                &["m3_020_flagstone_waiting_lines|Use".to_string()],
                &[M3_020_MINT_QUEUE_STATE.to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_020_short_work_state_requires_board_read() {
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_hire_short_work_boards",
                "AcceptJob",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_hire_short_work_boards",
                "AcceptJob",
                &["m3_020_hire_short_work_boards|Read".to_string()],
                &[]
            ),
            Some(M3_020_SHORT_WORK_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_hire_short_work_boards",
                "AcceptJob",
                &["m3_020_hire_short_work_boards|Read".to_string()],
                &[M3_020_SHORT_WORK_STATE.to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_020_pass_and_council_states_require_prior_read() {
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_stone_gate_pass_tags",
                "JoinCheck",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_stone_gate_pass_tags",
                "JoinCheck",
                &["m3_020_stone_gate_pass_tags|Read".to_string()],
                &[]
            ),
            Some(M3_020_PASS_CHECK_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_council_notice_strip",
                "RegisterComplaint",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_council_notice_strip",
                "RegisterComplaint",
                &["m3_020_council_notice_strip|Read".to_string()],
                &[]
            ),
            Some(M3_020_COUNCIL_COMPLAINT_STATE)
        );
        assert_eq!(
            m3_020_scene_object_state_event_payload(
                "m3_020_council_notice_strip",
                "RegisterComplaint",
                &["m3_020_council_notice_strip|Read".to_string()],
                &[M3_020_COUNCIL_COMPLAINT_STATE.to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_027_object_states_remain_concrete_scene_states() {
        assert_eq!(
            m3_027_scene_object_state_event_payload("m3_027_wet_bench", "Rest", &[], &[]),
            Some(M3_027_BENCH_REST_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_wet_bench",
                "Rest",
                &[],
                &[M3_027_BENCH_REST_STATE.to_string()]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_wet_bench",
                "Rest",
                &[],
                &[M3_027_BENCH_REST_STATE.to_string()]
            )
            .unwrap()
            .contains("包袱佔回去")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_hot_soup_stall",
                0,
                &[],
                &[M3_027_SOUP_WARM_STATE.to_string()]
            )
            .unwrap()
            .contains("空碗")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_hot_soup_stall",
                "BuySoup",
                &[],
                &[M3_027_FREE_BROTH_STATE.to_string()]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_hot_soup_stall",
                "QueueBroth",
                &[],
                &[M3_027_SOUP_WARM_STATE.to_string()]
            )
            .unwrap()
            .contains("輪過你")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload("m3_027_river_awning", "DryOff", &[], &[]),
            Some(M3_027_AWNING_DRY_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_charcoal_brazier",
                "WarmHands",
                &[],
                &[]
            ),
            Some(M3_027_BRAZIER_WARM_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_waiting_line_chat",
                "TalkQueue",
                &[],
                &[]
            ),
            Some(M3_027_QUEUE_TALK_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_waiting_line_chat",
                "QueueFerry",
                &[],
                &[]
            ),
            Some(M3_027_QUEUE_FERRY_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_waiting_line_chat",
                "QueueFerry",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_waiting_line_chat",
                "QueueFerry",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            )
            .unwrap()
            .contains("你已在隊列")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_waiting_line_chat",
                0,
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            )
            .unwrap()
            .contains("同一條木牌線")
        );
    }

    #[test]
    fn m3_027_rope_help_requires_prior_cargo_help() {
        assert_eq!(
            m3_027_scene_object_state_event_payload("m3_027_boat_rope", "HelpRope", &[], &[]),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response("m3_027_boat_rope", "HelpRope", &[], &[])
                .unwrap()
                .contains("貨擔")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_boat_rope",
                "HelpRope",
                &[],
                &[M3_027_CARGO_HELP_STATE.to_string()]
            ),
            Some(M3_027_ROPE_HELP_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_boat_rope",
                "HelpRope",
                &[],
                &[
                    M3_027_CARGO_HELP_STATE.to_string(),
                    M3_027_ROPE_HELP_STATE.to_string()
                ]
            ),
            None
        );
    }

    #[test]
    fn m3_027_cross_scene_loop_requires_prior_scene_states() {
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_plank_boarding_marker",
                "BoardFerry",
                &[],
                &[]
            ),
            None
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_plank_boarding_marker",
                "BoardFerry",
                &[],
                &[]
            )
            .unwrap()
            .contains("先回北岸隊列")
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_plank_boarding_marker",
                "BoardFerry",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            ),
            Some(M3_027_BOARD_FERRY_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_south_landing_rope",
                "StepAshore",
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            ),
            None
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_south_landing_rope",
                "StepAshore",
                &[],
                &[M3_027_BOARD_FERRY_STATE.to_string()]
            ),
            Some(M3_027_LANDED_SOUTH_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_warehouse_tally_board",
                "CheckCargo",
                &[],
                &[]
            ),
            None
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_warehouse_tally_board",
                "CheckCargo",
                &[],
                &[M3_027_CARGO_HELP_STATE.to_string()]
            ),
            Some(M3_027_WAREHOUSE_TALLY_STATE)
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_plank_boarding_marker",
                0,
                &[],
                &[M3_027_QUEUE_FERRY_STATE.to_string()]
            )
            .unwrap()
            .contains("隊列位置")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_warehouse_tally_board",
                0,
                &[],
                &[M3_027_CARGO_HELP_STATE.to_string()]
            )
            .unwrap()
            .contains("短冊")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_warehouse_tally_board",
                0,
                &[],
                &[
                    M3_027_CARGO_HELP_STATE.to_string(),
                    M3_027_BOARD_FERRY_STATE.to_string(),
                    M3_027_LANDED_SOUTH_STATE.to_string(),
                ]
            )
            .unwrap()
            .contains("短冊空出一行")
        );
    }

    #[test]
    fn m3_027_reed_market_uses_recent_sales_as_round_demand() {
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &[],
                &[]
            ),
            Some(M3_027_REED_SOLD_STATE)
        );

        let one_sale = vec!["m3_027_market_reed_buyer|SellReeds".to_string()];
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &one_sale,
                &[M3_027_REED_SOLD_STATE.to_string()]
            ),
            Some(M3_027_REED_DEMAND_FILLED_STATE)
        );
        assert!(
            m3_027_scene_object_state_action_response(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &one_sale,
                &[M3_027_REED_SOLD_STATE.to_string()]
            )
            .unwrap()
            .contains("價牌")
        );
        assert!(m3_027_reed_market_demand_filled(
            &[
                "m3_027_market_reed_buyer|SellReeds".to_string(),
                "m3_027_market_reed_buyer|SellReeds".to_string(),
            ],
            &[]
        ));

        let buyer_obj = crate::model::RoomObject {
            id: "m3_027_market_reed_buyer".to_string(),
            name: "蘆草收牌".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "SellReeds".to_string()],
            responses: [("Look".to_string(), "價牌壓在秤邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (after_one_sale, _) = object_followup_buttons(
            &buyer_obj,
            "",
            "Look",
            &one_sale,
            &[M3_027_REED_SOLD_STATE.to_string()],
        );
        assert_eq!(after_one_sale, vec!["SellReeds".to_string()]);
        let (after_demand_filled, _) = object_followup_buttons(
            &buyer_obj,
            "",
            "Look",
            &one_sale,
            &[
                M3_027_REED_SOLD_STATE.to_string(),
                M3_027_REED_DEMAND_FILLED_STATE.to_string(),
            ],
        );
        assert!(after_demand_filled.is_empty());
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_market_reed_buyer",
                0,
                &one_sale,
                &[M3_027_REED_SOLD_STATE.to_string()]
            )
            .unwrap()
            .contains("蘆草還未收滿")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_market_reed_buyer",
                0,
                &one_sale,
                &[M3_027_REED_DEMAND_FILLED_STATE.to_string()]
            )
            .unwrap()
            .contains("蘆草墊箱")
        );
    }

    #[test]
    fn m3_027_pack_order_multi_material_demands_are_independent() {
        assert_eq!(
            m3_027_material_source_item_for_object("m3_027_market_sackcloth_pile"),
            Some(M3_027_SACKCLOTH_ITEM_ID)
        );
        assert_eq!(
            m3_027_material_source_item_for_object("m3_027_plank_wet_rope_coil"),
            Some(M3_027_ROPE_ITEM_ID)
        );
        assert_eq!(
            m3_027_material_gather_state_for_object(
                "m3_027_market_sackcloth_pile",
                M3_027_SACKCLOTH_ITEM_ID
            ),
            Some(M3_027_SACKCLOTH_GATHERED_STATE)
        );
        assert_eq!(
            m3_027_material_gather_state_for_object(
                "m3_027_plank_wet_rope_coil",
                M3_027_ROPE_ITEM_ID
            ),
            Some(M3_027_ROPE_GATHERED_STATE)
        );

        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellSackcloth",
                &[],
                &[]
            ),
            Some(M3_027_SACKCLOTH_DEMAND_FILLED_STATE)
        );
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellRope",
                &[],
                &[]
            ),
            Some(M3_027_ROPE_DEMAND_FILLED_STATE)
        );

        let pack_order = crate::model::RoomObject {
            id: "m3_027_market_reed_buyer".to_string(),
            name: "市集包貨單".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "SellReeds".to_string(),
                "SellSackcloth".to_string(),
                "SellRope".to_string(),
            ],
            responses: [("Look".to_string(), "包貨單壓在秤邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (after_sackcloth, _) = object_followup_buttons(
            &pack_order,
            "",
            "Look",
            &[],
            &[M3_027_SACKCLOTH_DEMAND_FILLED_STATE.to_string()],
        );
        assert_eq!(
            after_sackcloth,
            vec!["SellReeds".to_string(), "SellRope".to_string()]
        );
        let all_filled_states = vec![
            M3_027_REED_DEMAND_FILLED_STATE.to_string(),
            M3_027_SACKCLOTH_DEMAND_FILLED_STATE.to_string(),
            M3_027_ROPE_DEMAND_FILLED_STATE.to_string(),
        ];
        let (after_all_filled, _) =
            object_followup_buttons(&pack_order, "", "Look", &[], &all_filled_states);
        assert!(after_all_filled.is_empty());
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_market_reed_buyer",
                0,
                &[],
                &all_filled_states
            )
            .unwrap()
            .contains("三行")
        );
        assert!(
            m3_027_scene_object_look_echo(
                "",
                "m3_027_warehouse_tally_board",
                0,
                &[],
                &all_filled_states
            )
            .unwrap()
            .contains("麻繩")
        );
    }

    #[test]
    fn m3_029_pack_runner_delivery_counts_toward_shared_reed_demand() {
        let npc_reed_action = vec!["m3_027_market_reed_buyer|SellReeds".to_string()];
        let npc_reed_states = vec![
            "m3_029_pack_runner|reed_delivered_to_short_market".to_string(),
            M3_027_REED_SOLD_STATE.to_string(),
        ];
        assert_eq!(
            m3_027_scene_object_state_event_payload(
                "m3_027_market_reed_buyer",
                "SellReeds",
                &npc_reed_action,
                &npc_reed_states
            ),
            Some(M3_027_REED_DEMAND_FILLED_STATE)
        );
        let market_echo = m3_027_scene_object_look_echo(
            "",
            "m3_027_market_reed_buyer",
            0,
            &npc_reed_action,
            &npc_reed_states,
        )
        .unwrap();
        assert!(market_echo.contains("搬貨人"));
        assert!(market_echo.contains("同一張單"));

        let warehouse_echo = m3_027_scene_object_look_echo(
            "",
            "m3_027_warehouse_tally_board",
            0,
            &npc_reed_action,
            &npc_reed_states,
        )
        .unwrap();
        assert!(warehouse_echo.contains("市集"));
        assert!(warehouse_echo.contains("倉棚"));
    }

    #[test]
    fn m3_027_object_buttons_reflect_consumed_state() {
        let soup_obj = crate::model::RoomObject {
            id: "m3_027_hot_soup_stall".to_string(),
            name: "熱湯攤".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "BuySoup".to_string(),
                "QueueBroth".to_string(),
            ],
            responses: [("Look".to_string(), "湯桶冒著熱氣。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (available, _) = object_followup_buttons(&soup_obj, "", "Look", &[], &[]);
        assert_eq!(
            available,
            vec!["BuySoup".to_string(), "QueueBroth".to_string()]
        );
        let (consumed, _) = object_followup_buttons(
            &soup_obj,
            "",
            "Look",
            &[],
            &[M3_027_FREE_BROTH_STATE.to_string()],
        );
        assert!(consumed.is_empty());

        let awning_obj = crate::model::RoomObject {
            id: "m3_027_river_awning".to_string(),
            name: "避風棚".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "DryOff".to_string()],
            responses: [("Look".to_string(), "舊帆布壓著木桿。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (already_dried, _) = object_followup_buttons(
            &awning_obj,
            "",
            "Look",
            &[],
            &[M3_027_AWNING_DRY_STATE.to_string()],
        );
        assert!(already_dried.is_empty());

        let queue_obj = crate::model::RoomObject {
            id: "m3_027_waiting_line_chat".to_string(),
            name: "候渡隊列".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "TalkQueue".to_string(),
                "QueueFerry".to_string(),
            ],
            responses: [("Look".to_string(), "候渡隊列折在踏板前。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (queue_available, _) = object_followup_buttons(&queue_obj, "", "Look", &[], &[]);
        assert_eq!(
            queue_available,
            vec!["TalkQueue".to_string(), "QueueFerry".to_string()]
        );
        let (already_talked, _) = object_followup_buttons(
            &queue_obj,
            "",
            "Look",
            &[],
            &[M3_027_QUEUE_TALK_STATE.to_string()],
        );
        assert_eq!(already_talked, vec!["QueueFerry".to_string()]);
        let (already_queued, _) = object_followup_buttons(
            &queue_obj,
            "",
            "Look",
            &[],
            &[
                M3_027_QUEUE_TALK_STATE.to_string(),
                M3_027_QUEUE_FERRY_STATE.to_string(),
            ],
        );
        assert!(already_queued.is_empty());

        let plank_obj = crate::model::RoomObject {
            id: "m3_027_plank_boarding_marker".to_string(),
            name: "踏板木牌".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "BoardFerry".to_string()],
            responses: [("Look".to_string(), "踏板木牌立在水線邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_queue, _) = object_followup_buttons(&plank_obj, "", "Look", &[], &[]);
        assert!(without_queue.is_empty());
        let (with_queue, _) = object_followup_buttons(
            &plank_obj,
            "",
            "Look",
            &[],
            &[M3_027_QUEUE_FERRY_STATE.to_string()],
        );
        assert_eq!(with_queue, vec!["BoardFerry".to_string()]);

        let warehouse_obj = crate::model::RoomObject {
            id: "m3_027_warehouse_tally_board".to_string(),
            name: "倉棚點貨牌".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "CheckCargo".to_string()],
            responses: [("Look".to_string(), "點貨牌掛在倉門邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_cargo, _) = object_followup_buttons(&warehouse_obj, "", "Look", &[], &[]);
        assert!(without_cargo.is_empty());
        let (with_cargo, _) = object_followup_buttons(
            &warehouse_obj,
            "",
            "Look",
            &[],
            &[M3_027_CARGO_HELP_STATE.to_string()],
        );
        assert_eq!(with_cargo, vec!["CheckCargo".to_string()]);
        let (already_checked, _) = object_followup_buttons(
            &warehouse_obj,
            "",
            "Look",
            &[],
            &[
                M3_027_CARGO_HELP_STATE.to_string(),
                M3_027_WAREHOUSE_TALLY_STATE.to_string(),
            ],
        );
        assert!(already_checked.is_empty());
    }

    #[test]
    fn m3_020_queue_button_is_contextual() {
        let obj = crate::model::RoomObject {
            id: "m3_020_mint_half_open_door".to_string(),
            name: "半開鑄幣門".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Open".to_string(), "Queue".to_string()],
            responses: [("Look".to_string(), "半開的門縫。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };

        let (without_waiting, _) = object_followup_buttons(&obj, "", "Look", &[], &[]);
        assert_eq!(without_waiting, vec!["Open".to_string()]);

        let (with_waiting, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[],
        );
        assert_eq!(with_waiting, vec!["Open".to_string(), "Queue".to_string()]);

        let (already_queued, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[M3_020_MINT_QUEUE_STATE.to_string()],
        );
        assert_eq!(already_queued, vec!["Open".to_string()]);
    }

    #[test]
    fn m3_020_accept_job_button_is_contextual() {
        let obj = crate::model::RoomObject {
            id: "m3_020_hire_short_work_boards".to_string(),
            name: "短工牌".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "AcceptJob".to_string(),
            ],
            responses: [("Look".to_string(), "短工牌掛在棚柱外側。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };

        let (without_read, _) = object_followup_buttons(&obj, "", "Look", &[], &[]);
        assert_eq!(without_read, vec!["Read".to_string()]);

        let (with_read, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_hire_short_work_boards|Read".to_string()],
            &[],
        );
        assert_eq!(with_read, vec!["Read".to_string(), "AcceptJob".to_string()]);

        let (already_accepted, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_hire_short_work_boards|Read".to_string()],
            &[M3_020_SHORT_WORK_STATE.to_string()],
        );
        assert_eq!(already_accepted, vec!["Read".to_string()]);
    }

    #[test]
    fn m3_020_pass_and_council_buttons_are_contextual() {
        let pass_obj = crate::model::RoomObject {
            id: "m3_020_stone_gate_pass_tags".to_string(),
            name: "通行牌".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "JoinCheck".to_string(),
            ],
            responses: [("Look".to_string(), "通行牌壓在石案邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_pass_read, _) = object_followup_buttons(&pass_obj, "", "Look", &[], &[]);
        assert_eq!(without_pass_read, vec!["Read".to_string()]);
        let (with_pass_read, _) = object_followup_buttons(
            &pass_obj,
            "",
            "Look",
            &["m3_020_stone_gate_pass_tags|Read".to_string()],
            &[],
        );
        assert_eq!(
            with_pass_read,
            vec!["Read".to_string(), "JoinCheck".to_string()]
        );
        let (pass_already_checked, _) = object_followup_buttons(
            &pass_obj,
            "",
            "Look",
            &["m3_020_stone_gate_pass_tags|Read".to_string()],
            &[M3_020_PASS_CHECK_STATE.to_string()],
        );
        assert_eq!(pass_already_checked, vec!["Read".to_string()]);

        let notice_obj = crate::model::RoomObject {
            id: "m3_020_council_notice_strip".to_string(),
            name: "議棚告示".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "RegisterComplaint".to_string(),
            ],
            responses: [("Look".to_string(), "告示貼在階邊木板上。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_notice_read, _) = object_followup_buttons(&notice_obj, "", "Look", &[], &[]);
        assert_eq!(without_notice_read, vec!["Read".to_string()]);
        let (with_notice_read, _) = object_followup_buttons(
            &notice_obj,
            "",
            "Look",
            &["m3_020_council_notice_strip|Read".to_string()],
            &[],
        );
        assert_eq!(
            with_notice_read,
            vec!["Read".to_string(), "RegisterComplaint".to_string()]
        );
        let (notice_already_registered, _) = object_followup_buttons(
            &notice_obj,
            "",
            "Look",
            &["m3_020_council_notice_strip|Read".to_string()],
            &[M3_020_COUNCIL_COMPLAINT_STATE.to_string()],
        );
        assert_eq!(notice_already_registered, vec!["Read".to_string()]);
    }

    #[test]
    fn m3_020_queue_state_echo_takes_priority() {
        let echo = m3_020_scene_object_look_echo(
            "m3_020_mint_half_open_door",
            &["m3_020_mint_half_open_door|Open".to_string()],
            &[M3_020_MINT_QUEUE_STATE.to_string()],
        )
        .unwrap();
        assert!(echo.contains("側線已被兌牌人接住"));

        let flagstone_echo = m3_020_scene_object_look_echo(
            "m3_020_flagstone_waiting_lines",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[
                M3_020_MINT_QUEUE_STATE.to_string(),
                M3_020_SHORT_WORK_STATE.to_string(),
            ],
        )
        .unwrap();
        assert!(flagstone_echo.contains("通往棚口"));

        let mint_echo_after_short_work = m3_020_scene_object_look_echo(
            "m3_020_mint_half_open_door",
            &["m3_020_mint_half_open_door|Open".to_string()],
            &[
                M3_020_MINT_QUEUE_STATE.to_string(),
                M3_020_SHORT_WORK_STATE.to_string(),
            ],
        )
        .unwrap();
        assert!(mint_echo_after_short_work.contains("側線已被兌牌人接住"));

        let flagstone_after_council = m3_020_scene_object_look_echo(
            "m3_020_flagstone_waiting_lines",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[
                M3_020_MINT_QUEUE_STATE.to_string(),
                M3_020_SHORT_WORK_STATE.to_string(),
                M3_020_PASS_CHECK_STATE.to_string(),
                M3_020_COUNCIL_COMPLAINT_STATE.to_string(),
            ],
        )
        .unwrap();
        assert!(flagstone_after_council.contains("往坡街那側"));
    }

    #[test]
    fn m3_020_followup_echo_requires_matching_prior_payload() {
        assert!(
            m3_020_scene_object_followup_echo(
                "m3_020_mint_half_open_door",
                &["m3_020_mint_half_open_door|Open".to_string()]
            )
            .unwrap()
            .contains("門縫比先前更窄")
        );
        assert!(
            m3_020_scene_object_followup_echo(
                "m3_020_mint_half_open_door",
                &["m3_020_flagstone_waiting_lines|Use".to_string()]
            )
            .unwrap()
            .contains("側線延伸到巷口")
        );
        assert_eq!(
            m3_020_scene_object_followup_echo(
                "m3_020_mint_half_open_door",
                &["bad-payload".to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_020_followup_echo_uses_latest_relevant_event() {
        let waiting_then_mint = vec![
            "m3_020_flagstone_waiting_lines|Use".to_string(),
            "m3_020_mint_half_open_door|Open".to_string(),
        ];
        assert!(
            m3_020_scene_object_followup_echo("m3_020_mint_half_open_door", &waiting_then_mint)
                .unwrap()
                .contains("門縫比先前更窄")
        );

        let mint_then_waiting = vec![
            "m3_020_mint_half_open_door|Open".to_string(),
            "m3_020_flagstone_waiting_lines|Use".to_string(),
        ];
        assert!(
            m3_020_scene_object_followup_echo("m3_020_mint_half_open_door", &mint_then_waiting)
                .unwrap()
                .contains("側線延伸到巷口")
        );
    }

    #[test]
    fn m3_027_material_gather_batch_success_text_formatting() {
        let single_reed = materials::GatheredMaterial {
            item_id: M3_027_REED_ITEM_ID.to_string(),
            name: "河霧蘆草".to_string(),
            source_node_id: "north_bank_reeds".to_string(),
            source_room_name: "半嶺水岸渡口".to_string(),
            origin_label: "河霧蘆叢".to_string(),
            stock_remaining: 3,
        };
        let single_text = m3_027_material_gather_batch_success_text(
            "m3_027_north_bank_reeds",
            &single_reed,
            1,
        );
        assert!(single_text.contains("割下一束「河霧蘆草」"));

        let batch_reed = materials::GatheredMaterial {
            item_id: M3_027_REED_ITEM_ID.to_string(),
            name: "河霧蘆草".to_string(),
            source_node_id: "north_bank_reeds".to_string(),
            source_room_name: "半嶺水岸渡口".to_string(),
            origin_label: "河霧蘆叢".to_string(),
            stock_remaining: 0,
        };
        let batch_text = m3_027_material_gather_batch_success_text(
            "m3_027_north_bank_reeds",
            &batch_reed,
            3,
        );
        assert!(batch_text.contains("扎成 3 束「河霧蘆草」"));
        assert!(batch_text.contains("整片靠近水線的蘆叢都被割低了"));

        let batch_sackcloth = materials::GatheredMaterial {
            item_id: M3_027_SACKCLOTH_ITEM_ID.to_string(),
            name: "粗麻袋布".to_string(),
            source_node_id: "market_sackcloth_pile".to_string(),
            source_room_name: "水尾寮渡頭市集".to_string(),
            origin_label: "棚腳麻袋堆".to_string(),
            stock_remaining: 2,
        };
        let sackcloth_text = m3_027_material_gather_batch_success_text(
            "m3_027_market_sackcloth_pile",
            &batch_sackcloth,
            2,
        );
        assert!(sackcloth_text.contains("捲成 2 卷「粗麻袋布」"));

        let batch_rope = materials::GatheredMaterial {
            item_id: M3_027_ROPE_ITEM_ID.to_string(),
            name: "浸水麻繩".to_string(),
            source_node_id: "plank_wet_rope_coil".to_string(),
            source_room_name: "渡船踏板".to_string(),
            origin_label: "踏板木樁繩圈".to_string(),
            stock_remaining: 1,
        };
        let rope_text = m3_027_material_gather_batch_success_text(
            "m3_027_plank_wet_rope_coil",
            &batch_rope,
            2,
        );
        assert!(rope_text.contains("解下並盤好 2 卷韌度結實的「浸水麻繩」"));
    }

    #[test]
    fn m3_027_reed_batch_sale_count_calculation() {
        // 當手頭有 2 束蘆草且市場無人交貨時，一次交齊 2 束
        assert_eq!(m3_027_reed_batch_sale_count(2, &[], &[]), 2);
        // 當手頭有 5 束蘆草，受限於每輪需求上限 2 束，一次交齊 2 束
        assert_eq!(m3_027_reed_batch_sale_count(5, &[], &[]), 2);
        // 當手頭僅有 1 束，交出 1 束
        assert_eq!(m3_027_reed_batch_sale_count(1, &[], &[]), 1);
        // 當市場已有人交了 1 束，手頭有 2 束，只交出剩餘的 1 束
        let one_prior_sale = vec!["m3_027_market_reed_buyer|SellReeds".to_string()];
        assert_eq!(m3_027_reed_batch_sale_count(2, &one_prior_sale, &[]), 1);
        // 當市場需求已滿（2 束），交出 0 束
        let two_prior_sales = vec![
            "m3_027_market_reed_buyer|SellReeds".to_string(),
            "m3_027_market_reed_buyer|SellReeds".to_string(),
        ];
        assert_eq!(m3_027_reed_batch_sale_count(2, &two_prior_sales, &[]), 0);
        // 當市場需求狀態已標記填滿，交出 0 束
        let filled_state = vec![M3_027_REED_DEMAND_FILLED_STATE.to_string()];
        assert_eq!(m3_027_reed_batch_sale_count(2, &[], &filled_state), 0);
        // 手頭無蘆草時交出 0 束
        assert_eq!(m3_027_reed_batch_sale_count(0, &[], &[]), 0);
    }

    #[test]
    fn format_runtime_gather_batch_narrative_test() {
        let single = crate::pg::world_resource::GatherResult {
            action_id: "act_1".into(),
            place_id: "place_1".into(),
            entity_id: "ent_1".into(),
            item_id: "mat_ore".into(),
            name: "粗礦石".into(),
            quantity: 1,
            source_stock_after: 5,
            entity_quantity_after: 1,
            inventory: "[]".into(),
            narrative: "你敲下了一塊粗礦石。".into(),
        };
        assert_eq!(
            format_runtime_gather_batch_narrative(&single),
            "你敲下了一塊粗礦石。"
        );

        let batch = crate::pg::world_resource::GatherResult {
            quantity: 3,
            narrative: "你敲下了一塊粗礦石。".into(),
            name: "粗礦石".into(),
            ..single
        };
        let batch_text = format_runtime_gather_batch_narrative(&batch);
        assert!(batch_text.contains("你順著手勢一氣呵成連取了 3 份「粗礦石」"));
    }

    #[test]
    fn resolve_room_object_handles_catalog_and_peninsula_entries() {
        let obj = resolve_room_object("nonexistent_room", "wt_cat_0001");
        assert!(obj.is_some());
        let obj = obj.unwrap();
        assert_eq!(obj.name, "赤鐵重礦");
        assert!(obj.sockets.contains(&"Look".to_string()));

        let by_name = resolve_room_object("nonexistent_room", "三號坑赤鐵重礦");
        assert!(by_name.is_some());
        let by_short_name = resolve_room_object("nonexistent_room", "赤鐵重礦");
        assert!(by_short_name.is_some());
    }

    #[test]
    fn resolve_room_object_handles_fountain_pen() {
        let by_id = resolve_room_object("文曜・文曜學院", "wt_academy_fountain_pen");
        assert!(by_id.is_some());
        let obj = by_id.unwrap();
        assert_eq!(obj.id, "wt_academy_fountain_pen");
        assert_eq!(obj.name, "黃銅儲墨筆");
        assert!(obj.sockets.contains(&"Look".to_string()));
        assert!(obj.sockets.contains(&"Use".to_string()));
        assert!(obj.sockets.contains(&"Operate".to_string()));
        assert!(obj.responses.get("Look").unwrap().contains("黃銅桿儲墨鋼筆"));
        assert!(obj.responses.get("Use").unwrap().contains("銥金筆尖觸及竹綿紙面"));
        assert!(obj.responses.get("Operate").unwrap().contains("輕旋黃銅筆桿檢視儲墨囊"));

        let by_name = resolve_room_object("any_room", "教案上的黃銅鋼筆");
        assert!(by_name.is_some());
    }

    #[test]
    fn resolve_room_name_handles_grid_and_plain_names() {
        assert_eq!(resolve_room_name("白塔公議堂"), "白塔公議堂");
    }
}
