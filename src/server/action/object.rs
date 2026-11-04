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
#[cfg(test)]
use super::normalize_action_key;

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
            name: "床榻".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Use".to_string(),
                "Rest".to_string(),
                "Sleep".to_string(),
                "Operate".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "溫暖乾燥的松香木大床上鋪著蓬鬆羽絨暖被與山風草安神枕，床旁小几上立著一盞旋蓋夜光貝母檯燈，光暈柔和溫潤。".to_string()),
                ("Use".to_string(), "你在松香木大床上舒展四肢，蓬鬆羽絨暖被帶來融融暖意，周身疲勞與肌肉酸脹頓時消散大半，身心漸漸平復放鬆。".to_string()),
                ("Rest".to_string(), "你合衣在柔軟的羽絨大床上躺下歇息，安神草的清香讓緊繃的心神徹底放鬆下來，周身氣血漸復充盈。".to_string()),
                ("Sleep".to_string(), "你鑽入溫暖蓬鬆的羽絨暖被，旋暗夜光貝母檯燈。在靜謐祥和的屋內，你安然入眠，體魄與心神盡皆恢復。".to_string()),
                ("Operate".to_string(), "你輕輕旋轉床頭夜光貝母檯燈的烏木旋蓋，珠白光芒柔和微調，滿室皆是安恬的暖意。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }

    if target == "wt_inn_bath_Preview" || target.contains("自來活水黃銅水栓") || target.contains("溫石熱水槽") {
        return Some(crate::model::RoomObject {
            id: "wt_inn_bath_Preview".to_string(),
            name: "沐浴桶".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Wash".to_string(), "Use".to_string(), "Bath".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "打磨光滑的黃銅水栓連接著暗渠活水管道，下方是紅土耐火磚砌成的溫石蓄熱水槽，槽邊備有山茶松脂皂與棉長絨擦巾。".to_string()),
                ("Wash".to_string(), "你踏入溫熱的杉木浴桶中浸泡洗沐，溫潤泉水洗去滿身風塵與汗漬，四肢百骸說不出的舒暢通透。".to_string()),
                ("Use".to_string(), "你擰開黃銅水栓，清冽純淨的活水流淌而出，隨後轉入溫水檔，溫暖的水流洗去塵泥，草木皂香格外清新。".to_string()),
                ("Bath".to_string(), "你引入溫石槽加熱的熱水浸沐洗面，熱氣騰騰驅散了旅途風塵與肌肉酸脹，倍感舒泰。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }

    if target == "wt_home_shell_lamp" || target.contains("旋蓋夜光貝母") || target.contains("樟木衣箱") || target.contains("貝母檯燈") || target.contains("貝母台燈") {
        return Some(crate::model::RoomObject {
            id: "wt_home_shell_lamp".to_string(),
            name: "檯燈".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Operate".to_string(), "Open".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "松木几案上立著旋蓋夜光貝母調光檯燈，旁側設有一只包銅角的防蛀樟木衣箱，箱蓋微敞散發幽淡樟腦木香。".to_string()),
                ("Use".to_string(), "你旋亮夜光貝母檯燈，珠白光暈柔和灑滿几案與床鋪，室內一片溫馨安寧。".to_string()),
                ("Operate".to_string(), "你旋轉烏木外罩調節進光弧度，光影隨心明暗，極具生活雅趣。".to_string()),
                ("Open".to_string(), "你揭開樟木衣箱銅扣，箱內分層鋪著吸濕防潮的竹綿細布，可妥善存放換洗衣物與貴重行囊。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_academy_fountain_pen"
        || target.contains("硬筆")
        || target.contains("黃銅硬筆")
        || target.contains("黃銅儲墨筆")
    {
        return Some(crate::model::RoomObject {
            id: "wt_academy_fountain_pen".to_string(),
            name: "黃銅儲墨筆".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Operate".to_string(), "AttendLecture".to_string()],
            responses: std::collections::HashMap::from([
                (
                    "Look".to_string(),
                    "烏木教案的正中央擱著一支溫潤沉甸的黃銅桿儲墨硬筆，筆尖以精煉銥金打磨得細膩微光，旁側襯著吸墨氈布與寫滿批註的竹綿紙講義。".to_string(),
                ),
                (
                    "Use".to_string(),
                    "你執起教案上的黃銅硬筆，銥金筆尖觸及竹綿紙面，黑墨勻稱流暢地順著筆鋒沁入紙紋，留下一行端勁清晰的手跡。".to_string(),
                ),
                (
                    "Operate".to_string(),
                    "你輕旋黃銅筆桿檢視儲墨囊，墨色飽滿澄澈，筆帽卡榫嚴絲合縫，足見文曜百工製筆之精巧。".to_string(),
                ),
                (
                    "AttendLecture".to_string(),
                    "你端坐於講席側畔，執筆旁聽教習策論，凝神研討時務經世之道。".to_string(),
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
                ("WarmHands".to_string(), "你湊近營火伸出雙手，融融暖流自掌心順著經脈蔓延至周身四肢，驅散了荒野寒濕。".to_string()),
                ("RoastFood".to_string(), "你取出行囊中的乾糧肉脯架於熾熱柴炭上方炙烤，油脂滋滋作響、肉香四溢，趁熱嚼食腹中溫暖充實。".to_string()),
                ("CraftTorch".to_string(), "你自營火堆中抽出一根引燃的松脂硬木，細心纏裹封固，製成一柄劈啪作響的照明松明火把。".to_string()),
                ("AddFuel".to_string(), "你拾起乾柴添入火堆中央，劈啪脆響中火勢陡然騰起，騰躍的金紅火舌將營地周遭照耀得一片透亮。".to_string()),
                ("CollectCharcoal".to_string(), "你以長枝撥開外層浮灰，將底層悶燒透紅的硬木火炭小心夾出、封土隔氧冷卻，收整出成色極佳的無煙白炭。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_home_bamboo_mailbox" || target.contains("竹編信箱") || target.contains("防雨信箱") {
        return Some(crate::model::RoomObject {
            id: "wt_home_bamboo_mailbox".to_string(),
            name: "信箱".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "Open".to_string(),
                "DispatchCourier".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "掛在門扉旁以青竹細編、外塗熟桐油防雨的立體竹筒信箱，開口設有銅扣翻蓋，專供半島旬報投遞與鄰里熟人託遞家書。".to_string()),
                ("Read".to_string(), "你掀開信箱防雨翻蓋，取出摺疊整齊的最新一期《半島旬報》細細閱覽。".to_string()),
                ("Open".to_string(), "你打開信箱銅扣，探手檢視內底，查閱是否有街坊鄰里或行腳客商留下的書信與心意回禮。".to_string()),
                ("DispatchCourier".to_string(), "你將備好的書信封妥投入信箱附設的快騎取件格，交付三枚鎂幣委請疾行草獸信使飛騎送出。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_courier_post_desk" || target.contains("信使櫃檯") || target.contains("信使總站") || target.contains("草獸信使") {
        return Some(crate::model::RoomObject {
            id: "wt_courier_post_desk".to_string(),
            name: "疾行草獸信使櫃檯".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "DispatchCourier".to_string(),
                "Use".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "疾行草獸信使櫃檯前陳列著往返四鎮的驛道封冊、火漆印章與加急竹筒，案旁栓著數頭蓄勢待發的健碩草獸。".to_string()),
                ("Read".to_string(), "你細讀櫃檯告示：草獸信使日夜兼程往來四鎮，承辦跨聚落加急包裹與重金文卷託運，每件資費 4 鎂。".to_string()),
                ("DispatchCourier".to_string(), "你向信使櫃檯交付 4 鎂託運規費，將重要急件交付草獸信使快騎飛遞。".to_string()),
                ("Use".to_string(), "你走近信使櫃檯，向值守掌事探詢各鎮驛道最新路況與班次安排。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_cat_0790" || target == "wt_bank_ledger_drawer" || target.contains("存摺櫃") || target.contains("分格存摺") {
        return Some(crate::model::RoomObject {
            id: "wt_cat_0790".to_string(),
            name: "分格存摺櫃".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "DepositBank".to_string(),
                "WithdrawBank".to_string(),
                "Use".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "厚重鐵木所造的分格存摺櫃，抽屜銅扣皆刻有三水口流水徽印，收存著各大商隊與鎮民的通兌簿冊與印鑑。".to_string()),
                ("Read".to_string(), "你細讀櫃頂張貼的三水總商行通兌章程，載明各港口通兌匯水與信用票號兌領條款。".to_string()),
                ("DepositBank".to_string(), "你在掌事案前出示存摺與信印，將隨身鎂幣妥善存入商行票號帳頭。".to_string()),
                ("WithdrawBank".to_string(), "你憑分格存摺防偽硃印與掌事核對數目，自票號結存中支取現鎂。".to_string()),
                ("Use".to_string(), "你走近分格存摺櫃，向值守執事出示存摺辦理商行通兌手續。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_home_jade_cabinet" || target.contains("寒水玉保鮮石櫃") || target.contains("寒水玉雙層保鮮石櫃") || target.contains("保鮮石櫃") {
        return Some(crate::model::RoomObject {
            id: "wt_home_jade_cabinet".to_string(),
            name: "冰鑑櫃".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Open".to_string(),
                "StoreStash".to_string(),
                "RetrieveStash".to_string(),
                "Use".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "屋角立著一座由柯氏木作坊榫接外框、內嵌北山寒水玉薄板的雙層保鮮石櫃。櫃門縫隙間隱隱透出天然清冽的涼意，存於其中的鮮食與獵獲經宿不腐。".to_string()),
                ("Open".to_string(), "你拉開寒水玉雙層保鮮石櫃的黃銅插銷，一股沁人心脾的天然玉石涼氣撲面而來，裡頭分格擺放的茶點、鮮果與肉乾皆維持著初入櫃時的鮮潤色澤。".to_string()),
                ("StoreStash".to_string(), "你將隨身物資妥善安放於寒水玉石板上，玉面寒氣瞬時凝固鮮氣，封存妥當。".to_string()),
                ("RetrieveStash".to_string(), "你拉開石櫃底層玉匣，將封存保鮮的物資取回隨身行囊之中。".to_string()),
                ("Use".to_string(), "你將隨身乾糧與果品整理妥當，置入寒水玉保鮮石櫃的冰涼玉板上鎖住鮮氣，並取出一盞冰鎮泉露飲下，暑氣與焦渴頓時全消。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_coastal_angling_spot" || target == "wt_cat_1006" || target.contains("老青竹黃銅輪捲線釣竿") || target.contains("老青竹釣竿") || target.contains("捲線釣竿") {
        return Some(crate::model::RoomObject {
            id: "wt_coastal_angling_spot".to_string(),
            name: "釣竿".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "AngleFish".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "架設在水岸青石旁的精工釣竿，竿身採老山青竹精製，堅韌修長；尾端配有岑氏工坊手工打磨的黃銅滾輪與牛角旋柄，絲線入水隨波蕩漾。".to_string()),
                ("AngleFish".to_string(), "你手持老青竹釣竿揚竿甩線，誘餌破水而入，隨波靜候水底大物咬鉤。".to_string()),
                ("Use".to_string(), "你檢視青竹釣竿的黃銅線輪與釣線張力，整理浮標隨時準備垂釣。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_lookout_telescope" || target == "wt_cat_0733" || target.contains("紅土光學黃銅單筒望遠鏡") || target.contains("黃銅單筒望遠鏡") || target.contains("單筒望遠鏡") {
        return Some(crate::model::RoomObject {
            id: "wt_lookout_telescope".to_string(),
            name: "望遠鏡".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "架設在堅實石座上的黃銅單筒望遠鏡，筒身銘刻光學工坊刻度，配備雙層紅土消色差透鏡，能極目遠眺數里外的荒野動向。".to_string()),
                ("Operate".to_string(), "你手扶黃銅鏡身旋動對焦螺環，鏡中視野漸漸清澄，將遠方山川風物與生靈行跡盡收眼底。".to_string()),
                ("Use".to_string(), "你湊近望遠鏡目鏡凝神端詳，穿透遠方山嵐薄霧，探查四方地貌與周遭行蹤。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_field_camp_stove" || target == "wt_cat_1010" || target.contains("卡式導能便攜爐") || target.contains("導能便攜爐") || target.contains("便攜爐") {
        return Some(crate::model::RoomObject {
            id: "wt_field_camp_stove".to_string(),
            name: "卡式導能便攜爐".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Cook".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "緊湊輕便的行旅便攜導能爐，配備防風耐熱精鋼環形爐圈與精準出火的紫銅火嘴，可隨時在野外安營起火烹煮。".to_string()),
                ("Cook".to_string(), "你旋開導能便攜爐的銅旋鈕點燃青焰，架起野炊銅鍋，投入食材烹煮熱氣騰騰的野地熱羹。".to_string()),
                ("Use".to_string(), "你調節導能爐盤的火焰大小，煮沸一壺清冽山泉暖手調息。".to_string()),
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


    if target.starts_with("hunt_kill_site_") || target.ends_with("・圍獵遺骸") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("hunt_kill_site_") {
                target.to_string()
            } else {
                "hunt_kill_site_0_0".to_string()
            },
            name: if target.ends_with("・圍獵遺骸") {
                target.to_string()
            } else {
                "圍獵遺骸".to_string()
            },
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Harvest".to_string(),
                "Gather".to_string(),
                "Take".to_string(),
            ],
            responses: std::collections::HashMap::new(),
            move_to_room_id: String::new(),
        });
    }

    if target.starts_with("detritus_fungi_") || target.contains("腐木牛樟靈菇叢") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("detritus_fungi_") {
                target.to_string()
            } else {
                "detritus_fungi_0_0".to_string()
            },
            name: "腐木牛樟靈菇叢".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Harvest".to_string(),
                "Gather".to_string(),
                "Take".to_string(),
            ],
            responses: std::collections::HashMap::new(),
            move_to_room_id: String::new(),
        });
    }

    if target == "shanhai_apex_trace" || target.contains("新鮮霸主獸跡") {
        return Some(crate::model::RoomObject {
            id: "shanhai_apex_trace".to_string(),
            name: "新鮮霸主獸跡".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string()],
            responses: std::collections::HashMap::new(),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_dock_riverbank_raft" || target.contains("平底竹筏") {
        return Some(crate::model::RoomObject {
            id: "wt_dock_riverbank_raft".to_string(),
            name: "平底竹筏".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "Use".to_string(), "RowSwampRaft".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "水岸石砌埠頭邊立著長年受流水沖刷的繫船青石樁，石孔穿過粗大桐油麻繩，平穩繫泊著數艘雙層防浸平底竹筏，筏上置有推水長竹篙與青竹魚簍。".to_string()),
                ("Operate".to_string(), "你解開繫在青石樁上的桐油麻繩，平底竹筏在川流水波間輕輕蕩漾，吃水甚穩。".to_string()),
                ("Use".to_string(), "你手持長竹篙在石階輕撐借力，竹筏滑入川流清澈碧流，沿岸水草豐美、水鳥掠波。".to_string()),
                ("RowSwampRaft".to_string(), "你解開纜繩點篙撐起竹筏，輕舟破開水尾寮蘆葦與黑沼迷霧，順流探尋澤中沉水烏木與止血水苔。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_inn_ebony_shell_chess" || target.contains("烏木棋盤") {
        return Some(crate::model::RoomObject {
            id: "wt_inn_ebony_shell_chess".to_string(),
            name: "烏木棋盤".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "PlayChess".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "黑沼沉水烏木雕琢的棋盤線道深沉分明，兩只棋罐各盛著溫潤厚重的烏木黑子與烏石鼻厚海貝白子，茶客對弈殺局難解難分。".to_string()),
                ("Operate".to_string(), "你拈起一枚沉甸甸的白貝棋子扣在棋盤交叉處，發出清脆沉實的篤響，盤中局勢頓時生出新意。".to_string()),
                ("PlayChess".to_string(), "你端坐於烏木棋盤前拈子佈局，黑白交錯推演天地之數，心神凝定，雜念俱泯。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_dining_iron_cauldron" || target.contains("紅鐵燉肉釜") {
        return Some(crate::model::RoomObject {
            id: "wt_dining_iron_cauldron".to_string(),
            name: "紅鐵燉肉釜".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Taste".to_string(), "WarmHands".to_string(), "SeasonalBanquet".to_string(), "AssistKitchen".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "紅鐵厚底大釜慢燉著大角原牛肉與甜白蘿蔔，肉香四溢；旁邊紫銅鴛鴦鍋滾水翻騰，備有切仔湯麵配滷蛋與大圳頭溫米酒。".to_string()),
                ("Taste".to_string(), "你舀起一勺慢燉的原牛肉湯送入口中，肉塊軟爛透味，湯頭鮮濃醇厚，整個人精神一振。".to_string()),
                ("WarmHands".to_string(), "你把雙手靠近大釜底座的耐火磚緣，溫熱厚重的火氣烘退了手掌上的水氣與寒意。".to_string()),
                ("SeasonalBanquet".to_string(), "你在紅鐵釜前添薪圍坐，鍋內歲時時令熱氣翻騰，同席之人舉杯共飲，暖意直透肺腑。".to_string()),
                ("AssistKitchen".to_string(), "你持長柄木杓在紅鐵燉肉釜前幫廚添薪撇沫，慢火細燉大角原牛肉湯。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_mill_") || target.contains("水輪玄武磨") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_mill_waterwheel_stone".to_string() },
            name: "水輪玄武磨".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "MillGrain".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "老堰頭引水渠衝擊巨木水輪帶動齒輪轉動，兩合巨大玄武岩磨盤正飛速研磨新熟米穀。".to_string()),
                ("Operate".to_string(), "你扳動水流分水閘柄調節水勢，水輪轉速隨之放緩，磨盤研磨出的白米粉更顯細密勻淨。".to_string()),
                ("MillGrain".to_string(), "你合力推開水磨料門，巨輪轉動帶動石磨隆隆轟鳴，白米碎成細漿，研磨出老堰頭溫熱糙米漿。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_sawmill_") || target.contains("水動排鋸架") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_sawmill_hydraulic_saw".to_string() },
            name: "水動排鋸架".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "RefinePlank".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "水流帶動的長排精鋼鋸條轟鳴起伏，平穩將整根百年雲杉原木剖切成均勻厚板。".to_string()),
                ("Operate".to_string(), "你推動送木台車向前咬緊鋸齒，鋼鋸飛速下切，鋸路筆直順暢，木屑紛飛。".to_string()),
                ("RefinePlank".to_string(), "你推動水動排鋸咬合雲杉巨木，精鋼鋸齒飛速剖切，裁出筆直平整的樟松木板料。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_stonecut_") || target.contains("金剛砂拉鋸") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_stonecut_wire_saw".to_string() },
            name: "金剛砂拉鋸".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "CutJade".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "工匠引細密合金絲拉鋸帶著濕潤金剛砂緩緩剖開巨大的北山寒水玉原石，石皮下透出溫潤玉光。".to_string()),
                ("Operate".to_string(), "你輕轉注水輪維持金剛砂泥漿順暢流入切割線，金屬絲勻速拉動，石料緩緩剖為兩半。".to_string()),
                ("CutJade".to_string(), "你拉動金剛砂鋼絲鋸緊貼寒水玉原石，冷泉注淋鋸槽，細細切剖出溫潤剔透的寒水玉料。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_cheese_") || target.contains("螺旋壓酪架") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_cheese_curd_press".to_string() },
            name: "螺旋壓酪架".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "Taste".to_string(), "PressCheese".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "大木架上設有黃銅螺旋加壓手柄，正將粗濾布中的凝乳塊瀝出金黃乳清，壓製硬乳酪輪。".to_string()),
                ("Operate".to_string(), "你握住螺旋桿加力旋轉半圈，木板沉重下壓，澄澈的乳清順著凹槽汩汩流出。".to_string()),
                ("Taste".to_string(), "你嚐了指尖沾上的一滴新鮮乳清，滋味微酸清冽，帶著濃醇的天然奶香。".to_string()),
                ("PressCheese".to_string(), "你旋緊螺旋木桿壓榨酪包，黃亮乳清緩緩瀝出，壓製出緊實濃郁的熟成硬乳酪。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_mine_") || target.starts_with("wt_trolley_") || target.contains("雙軸重台車") || target.contains("木製洗礦槽") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_trolley_rail_carriage".to_string() },
            name: "雙軸重台車".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "HaulMineCart".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "坑道軌道上停著厚重雙軸台車，精鐵鑄造的車輪扣住木軌鐵皮，裝滿粗礦依然平穩。".to_string()),
                ("Operate".to_string(), "你握住車身旁的機械制動閘手柄扳動，鐵閘緊緊咬住輪緣，發出沉實摩擦聲。".to_string()),
                ("HaulMineCart".to_string(), "你合力推動雙軸重台車滑入坑口鐵軌，伴隨鋼輪咬軌轟鳴，將滿車赤鐵重礦運出坑道。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_smokehouse_") || target.contains("慢燻紅陶灶") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_smokehouse_kiln".to_string() },
            name: "慢燻紅陶灶".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Smell".to_string(), "SmokeFish".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "地面砌出長排低溫煙道陶灶，正以受控的乾燥松木鋸屑慢火逼出濃郁木香冷煙。".to_string()),
                ("Smell".to_string(), "你深吸一口自陶灶溢出的煙氣，醇厚的松木煙燻香氣純淨甘醇，全無嗆鼻焦味。".to_string()),
                ("SmokeFish".to_string(), "你添入濕柏木與香草慢燻，灶內青煙繚繞，低溫透骨冷燻海鹽魚乾。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_lime_") || target.contains("海貝燃灰窯") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_lime_kiln_chamber".to_string() },
            name: "海貝燃灰窯".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "WarmHands".to_string(), "Operate".to_string(), "BurnLime".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "烏石鼻以玄武岩壘砌的圓形灰窯，烈火正將堆積的海貝殼高溫煅燒成雪白的建材灰粉。".to_string()),
                ("WarmHands".to_string(), "你將身子貼近灰窯厚實的外牆，石壁蓄積的渾厚餘熱烘散了海邊帶來的陰冷濕氣。".to_string()),
                ("Operate".to_string(), "你使用長柄鐵扒撥動窯底通風孔讓空氣流通，爐膛火勢更加旺盛純青。".to_string()),
                ("BurnLime".to_string(), "你添入焦炭鼓風引燃灰窯，高溫煅燒海貝碎殼，出窯白灰熱氣騰騰，細密凝實。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_hunt_") || target.contains("牛角破甲弩") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_hunt_heavy_crossbow".to_string() },
            name: "牛角破甲弩".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "PrepareMiasmaGear".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "工坊以大角原牛角片與堅木多層膠合的守林重弩，弩弦以原牛筋絞就，張力沉雄。".to_string()),
                ("Operate".to_string(), "你扳動腳蹬絞盤為重弩上弦，機括發出清脆卡響，鎖定弩機扣板，蓄勢待發。".to_string()),
                ("PrepareMiasmaGear".to_string(), "你校準破甲弩機，配齊紅磷引路信號筒與避瘴藥劑，整飭行囊以防黑沼瘴毒。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_cat_0734" || target.starts_with("wt_astro_") || target == "wt_obj_armillary_almanac" || target.contains("星軌儀") || target.contains("推步儀") || target.contains("定緯銅儀") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_cat_0734".to_string() },
            name: "渾天星軌銅儀".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "ChartAstrolabe".to_string(), "Operate".to_string(), "Read".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "半嶺觀星台上架設的黃銅渾天星軌儀，多層精密同心銅環鐫刻著周天星宿赤道與黃道度數，齒輪咬合滴水運轉，推步歲差與日月運行。".to_string()),
                ("ChartAstrolabe".to_string(), "你備妥竹綿宣紙與松煙精墨，輕敷於星軌儀外圈經緯刻度上細緻拓印，將四時天象與個人本命周天星度完整摹拓成卷。".to_string()),
                ("Operate".to_string(), "你輕輕旋轉渾天儀的赤道環微調齒輪，銅環咬合發出清脆鳴響，極軸準星穩穩鎖定天頂北辰。".to_string()),
                ("Read".to_string(), "你凝神細讀銅儀基座刻劃的《北辰歲差推步曆訣》，推演當前歲時節氣與星野分野之奧妙。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_martial_force_stone" || target.contains("測勁玄武碑") || target.contains("導能測紋石板") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_martial_force_stone".to_string() },
            name: "測勁玄武碑".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "TestEngraving".to_string(), "Operate".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "工坊內以烏石鼻玄武岩厚板琢磨而成的測勁試刃石碑，表面嵌有暗金導能測紋凹槽，可供武者運息灌注測試勁力深淺與兵刃平衡。".to_string()),
                ("TestEngraving".to_string(), "你提聚周身真氣貫注掌心按向測勁玄武碑，金線紋理隨內息深淺明滅流轉，嗡然共鳴間將隨身兵刃的重心與鋒芒校準至分毫不差。".to_string()),
                ("Operate".to_string(), "你手持短刃順著玄武碑表面的導能凹槽徐徐劃過，檢視刃紋深淺與鋼材淬火應力。".to_string()),
                ("Use".to_string(), "你以指節叩擊玄武碑身，聽音辨析石質密度與導能靈敏度，平復吐納氣息。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_arena_testing_pillar" || target == "wt_martial_padded_wooden_blade" || target == "wt_arena_wooden_weapon_rack" || target.contains("玄武試勁柱") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_arena_testing_pillar".to_string() },
            name: "玄武試勁柱".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "SparPillar".to_string(), "Operate".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "演武場中央矗立的玄武試勁巨柱，表面包裹著多層油鞣生牛皮與厚草繩，周遭擺放著包革木刀與兵器架，專供弟子磨礪拳腳招法與發勁沉雄之勢。".to_string()),
                ("SparPillar".to_string(), "你沉腰紮馬、抱元守一，繞柱演練「起勢・蓄勁、進步・破鋒、轉樞・卸力、收勢・歸元」四序武訣，拳風木刃撞擊石柱激起沉渾迴音。".to_string()),
                ("Operate".to_string(), "你調整試勁柱外裹的生牛皮護層繫繩，加固麻索結扣以承受更沉厚的重擊。".to_string()),
                ("Use".to_string(), "你取下一柄包革木刀掂量重心，貼近試勁柱比劃出刀角度與步法挪移。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_bath_jade_pool" || target == "wt_home_hotspring_tub" || target.contains("恆溫熱湯池") || target.contains("杉木沐浴桶") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_bath_jade_pool".to_string() },
            name: "恆溫熱湯池".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "SoakHotSpring".to_string(), "Rest".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "以半嶺寒水玉打底壘砌的天然熱湯池，溫潤白霧蒸騰繚繞，池邊放著溫氏百草祛寒湯包，地脈溫泉汨汨注滿池中，散發著舒心草木幽香。".to_string()),
                ("SoakHotSpring".to_string(), "你寬衣踏入熱湯池中，融融暖意順著肌膚毛孔滲透周身四肢，拔除骨髓深處的陰寒潮氣與跋涉勞損，經絡血脈暢達通順。".to_string()),
                ("Rest".to_string(), "你斜倚池畔光潔的寒水玉石階閉目小憩，聆聽溫泉活水潺潺流響，心神俱皆鬆弛放空。".to_string()),
                ("Use".to_string(), "你舀起一木桶溫熱泉湯澆淋雙肩，取過山茶草本皂細細洗滌塵泥，神清氣爽。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_barber_") || target == "wt_bath_mirror_washstand" || target.contains("摺疊剃刀架") || target.contains("洗頭木躺椅") || target.contains("銀鏡盥洗台") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_barber_folding_razor".to_string() },
            name: "摺疊剃刀架".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "GroomAppearance".to_string(), "Use".to_string(), "Rest".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "市井理髮鋪內的烏木梳妝几案，案上陳列著精鋼摺疊剃刀、牛角細齒梳、薄荷清香面膏與熱氣蒸騰的白布毛巾，透著幹練清爽的街坊人情味。".to_string()),
                ("GroomAppearance".to_string(), "你端坐於木躺椅上，覆上熱毛巾軟化鬍鬚，剃刀遊走間刮修鬢角、梳理鬚髮，清爽薄荷香氣沁人心脾，頓時容光煥發、神采奕奕。".to_string()),
                ("Use".to_string(), "你對著几前照影明晰的銀鏡整理衣冠領口，以細齒牛角梳順理髮髻。".to_string()),
                ("Rest".to_string(), "你靠在鋪著柔軟棉墊的洗頭躺椅上放鬆肩頸，享受溫水洗髮時的片刻安閒。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_repair_") || target == "wt_cloth_tailor_sewing_bench" || target.contains("修傘木砧檯") || target.contains("補靴硬木楦") || target.contains("裁縫量身几") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_repair_umbrella_bench".to_string() },
            name: "修傘木砧檯".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "MendTravelGear".to_string(), "Operate".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "街角匠人鋪的補綴木砧檯，檯面上散放著修補雨傘竹骨的細銅絲、生鐵底釘、補靴硬木楦與一罐透亮的熟桐油，專事修護旅者雨具與行腳革靴。".to_string()),
                ("MendTravelGear".to_string(), "你在砧檯前為行靴換上防滑耐磨的生鐵釘，抹勻厚魚油防水，並重新加固傘骨、刷上雙層熟桐油，整飭隨身行裝以備風雨長途。".to_string()),
                ("Operate".to_string(), "你使用木楦與小皮槌敲整行靴受壓變形的鞋頭，使靴筒重新貼合腳型。".to_string()),
                ("Use".to_string(), "你借用砧檯上的粗麻線與骨針穿引縫合行囊接縫，加固帶扣邊角。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_observatory_anemometer" || target == "wt_obj_wind_vane_gauge" || target == "wt_obs_tide_weather_table" || target.contains("黃銅風向儀") || target.contains("相風銅烏") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_observatory_anemometer".to_string() },
            name: "黃銅風向儀".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "ReadWindVane".to_string(), "Operate".to_string(), "Read".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "山巔測候台高聳的相風黃銅烏與旋轉風速杯，下連精密齒輪傳動指針，案几上鋪開十九獵場風向圖與近海潮汐盈縮水經推步表。".to_string()),
                ("ReadWindVane".to_string(), "你仰觀風向儀指向與銅烏旋動疾徐，記錄高空雲氣走向與近海潮位漲落，對照滴漏水鐘推算十九獵場天候變化與安全航道。".to_string()),
                ("Operate".to_string(), "你撥動風速儀轉軸潤滑銅套，清除齒輪軸承上的山嵐露水，確保量測靈敏精準。".to_string()),
                ("Read".to_string(), "你查閱觀測案上連續多月的潮汐升降水經註記，參照節氣推衍海潮汛期。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_clinic_acupoint_bed" || target == "wt_clinic_decoction_stove" || target == "wt_obj_acupuncture_pulse_pillow" || target.contains("竹席診脈榻") || target.contains("溫灸盒") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_clinic_acupoint_bed".to_string() },
            name: "竹席診脈榻".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "ApplyMoxibustion".to_string(), "Rest".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "醫館內鋪設細編青竹涼席的診脈榻，榻頭擱著青玉診脈枕與紫銅溫灸盒，爐膛內文火慢煨著百草祛寒透骨湯，藥香幽微甘醇。".to_string()),
                ("ApplyMoxibustion".to_string(), "你伏臥於竹席榻上，司藥點燃陳年蘄艾置於溫灸盒中，溫熨關元與命門穴，艾火純陽之熱循經入髓，透散筋骨陰寒鬱結與扭傷痠楚。".to_string()),
                ("Rest".to_string(), "你枕著青玉脈枕合目靜躺，伴隨滿室淡淡中草藥香調勻呼吸，緊繃的四肢百骸逐漸放鬆舒緩。".to_string()),
                ("Use".to_string(), "你端起司藥盛出的溫熱百草煎劑趁熱啜飲，苦甘藥力化開胸腹鬱結之氣。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_obj_mountain_milestone" || target == "wt_shrine_votive_plaque" || target.contains("山道指路碑") || target.contains("祈福木牌") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_obj_mountain_milestone".to_string() },
            name: "山道指路碑".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "HangVotivePlaque".to_string(), "Read".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "北嶺古山道旁矗立的深色花崗岩指路里程碑，碑旁橫架著古雲杉原木搭成的祈福木牌架，繫滿過往山客祈求行旅平安的厚實木牌，隨風輕叩。".to_string()),
                ("HangVotivePlaque".to_string(), "你取下一枚散發天然松脂清香的北嶺雲杉祈福木牌，提筆題寫行路平安字樣，虔誠懸於牌架之上，並收妥一枚隨身木牌作保平安之印記。".to_string()),
                ("Read".to_string(), "你拂去石碑青苔辨讀里程刻字，探明通往北嶺各處隘口、獵場與山泉古道的分岔方向。".to_string()),
                ("Use".to_string(), "你在指路石碑旁的青石歇腳坐下，飲泉歇息，整飭登山竹杖與綁腿。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_book_printing_shelf" || target.contains("風物誌書架") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_book_printing_shelf".to_string() },
            name: "風物誌書架".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Read".to_string(), "TranscribeRareTome".to_string(), "SunBooks".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "高大書架上整齊排著竹綿紙印製的《白塔風物誌》、每旬《半島旬報》與白話故事小冊，滿室紙墨芬芳。".to_string()),
                ("Read".to_string(), "你取下一卷《白塔風物誌》閱讀，書中記載著老堰頭水網修築沿革與寒水玉開採考證。".to_string()),
                ("TranscribeRareTome".to_string(), "你攤開竹綿宣紙與紅燭墨水，對著書架上的風物孤本恭謹鈔錄，筆端生風。".to_string()),
                ("SunBooks".to_string(), "你將書架上的竹綿紙古籍移至向陽通風石坪翻曬防蠹，夾入辟蠹芸香。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_clocktower_bronze_bell" || target.contains("報時青銅鐘") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_clocktower_bronze_bell".to_string() },
            name: "報時青銅鐘".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "CalibrateClock".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "高懸塔頂的厚重青銅巨鐘，垂下粗大的麻繩，每逢正點敲響，鐘聲清越浩蕩傳遍全境。".to_string()),
                ("Operate".to_string(), "你雙手握住粗麻繩合力一拉，青銅鐘錘重重撞上鐘體，發出一聲渾厚深沉的悠揚巨響。".to_string()),
                ("CalibrateClock".to_string(), "你參照青銅日晷刻度與滴漏校準鐘樓發條，登樓鳴鐘報刻，鐘聲悠揚傳遍四野。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_wood_planing_bench" || target.contains("刨木工作台") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_wood_planing_bench".to_string() },
            name: "刨木工作台".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "FitMortiseTenon".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "厚重工匠台上架著導能刨刀、拉鋸與木鑿，台面落滿薄如蟬翼的松木刨花，木香撲鼻。".to_string()),
                ("Operate".to_string(), "你握起導能推刨在松木料上平推而過，木片應手捲起均勻刨花，木面光滑平整。".to_string()),
                ("FitMortiseTenon".to_string(), "你手持角尺木鑿細細修校榫卯接榫，反覆推刨嵌合，嚴絲合縫、滴水不漏。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_forge_quench_trough" || target.contains("精鋼淬火槽") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_forge_quench_trough".to_string() },
            name: "精鋼淬火槽".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "TestEngraving".to_string(), "QuenchBlade".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "盛滿深井泉水與淬火油的大石槽，槽邊備有打磨精鋼刀具與門鎖配件的雙面細磨石。".to_string()),
                ("Use".to_string(), "你舀水淋在細磨石上推磨鐵件，金屬毛刺漸去，刃口泛起雪亮微光。".to_string()),
                ("TestEngraving".to_string(), "你運轉內息按上測紋石板並取隨身器物輕觸碑面，石紋微鳴回饋人器共鳴度。".to_string()),
                ("QuenchBlade".to_string(), "你將熾紅鋼刀猛然探入淬火油槽，白霧激盪翻騰，精鋼重煥寒芒。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_cloth_garment_rack" || target.contains("防雨長衫架") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_cloth_garment_rack".to_string() },
            name: "防雨長衫架".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "DyeIndigoCloth".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "松木衣架上整齊掛著塗有黑沼樹膠的防雨長衫、透氣棉麻短衫與北山雪狼絨冬衣，做工紮實。".to_string()),
                ("Use".to_string(), "你取下一件防雨長衫披在身上試穿，衣料厚韌輕便，領口銅扣封得嚴實，能擋狂風急雨。".to_string()),
                ("DyeIndigoCloth".to_string(), "你取過山藍染液浸潤粗麻料，反覆搓洗固色，為衣架上的防雨長衫添上一襲防雨藍靛。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_caravan_water_trough" || target.contains("青石飲馬槽") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_caravan_water_trough".to_string() },
            name: "青石飲馬槽".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "FeedDraftBeast".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "連通活水渠的深鑿青石長水槽，供來往跋涉的原牛與健壯駝獸低頭痛飲，水質清澈。".to_string()),
                ("Use".to_string(), "你拔開進水木栓注入新鮮井泉活水，奔波而來的原牛探頭大口痛飲，發出滿足低哼。".to_string()),
                ("FeedDraftBeast".to_string(), "你將新鮮甜燕麥與鮮嫩草料添入食槽，引來健壯馱獸垂首痛嚼，水槽泉水汨汨清澈。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_deed_cadastral_map_desk" || target.contains("四坊地籍案") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_deed_cadastral_map_desk".to_string() },
            name: "四坊地籍案".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Read".to_string(), "Use".to_string(), "InspectLandRegistry".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "寬闊平整的核契楠木長案上平鋪著文曜四坊地籍疆界全圖，一旁陳設著玻璃螺紋墨水瓶與弧形吸墨器。".to_string()),
                ("Read".to_string(), "你俯身查看四坊地籍圖冊，文曜城中各街各巷街屋的界址、戶號與公產邊界皆以硃砂與濃墨標記分明。".to_string()),
                ("Use".to_string(), "你執起木柄弧形吸墨器在草擬文契上輕輕滾過，新寫就的黑墨油光瞬間被乾淨吸附，字跡烏黑硬朗。".to_string()),
                ("InspectLandRegistry".to_string(), "你鋪開文曜四坊全圖，詳核街屋界標與公庫財賦，墨香撲鼻間取下一頁拓印地籍圖鈔。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_harbor_clockwise_ship" || target.contains("順時針環島海運商船") || target.contains("順時針環島海船") {
        return Some(crate::model::RoomObject {
            id: "wt_harbor_clockwise_ship".to_string(),
            name: "順時針環島海運商船".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "BoardCoastalShip".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "港灣深水泊位旁停靠著一艘雙桅硬帆的順時針環島海運商船，船首繪有破浪海燕紋章，水手們正忙碌清點壓艙石與淡水木桶。".to_string()),
                ("Use".to_string(), "你向船長交付船資，踏過跳板登上順時針環島海運商船，隨黑潮洋流啟程前往下一處海岸良港。".to_string()),
                ("BoardCoastalShip".to_string(), "你踏上順時針環島海運商船甲板，隨老舵手核對黑潮洋流與潮汐海圖，揚帆出海巡航。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_dock_cargo_gangplank" || target.contains("雲杉卸貨板") {
        return Some(crate::model::RoomObject {
            id: "wt_dock_cargo_gangplank".to_string(),
            name: "雲杉卸貨板".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "BoardCoastalShip".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "石砌碼頭與貨船舷側之間搭著寬闊厚實的北嶺雲杉卸貨跳板，板面釘有防滑橫木條。".to_string()),
                ("Use".to_string(), "你穩步踏過微幅起伏的雲杉跳板，腳下木板發出沉穩紮實的吱呀聲響。".to_string()),
                ("BoardCoastalShip".to_string(), "你踏過厚實防滑的雲杉跳板登上環島商船，隨水手核對黑潮航線與潮汐水文。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_dining_iron_cauldron" || target == "wt_dining_rice_milk_stall" || target.contains("紅鐵燉肉釜") || target.contains("溫米漿桶") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_dining_iron_cauldron".to_string() },
            name: if target.contains("米漿") { "溫米漿桶".to_string() } else { "紅鐵燉肉釜".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Taste".to_string(), "WarmHands".to_string(), "SeasonalBanquet".to_string(), "AssistKitchen".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "紅鐵厚底大釜慢燉著大角原牛肉與甜白蘿蔔，肉香四溢；旁邊紫銅熱桶溫著香濃米漿，熱氣騰騰。".to_string()),
                ("Taste".to_string(), "你舀起一勺慢燉的原牛肉湯送入口中，肉塊軟爛透味，湯頭鮮濃醇厚，整個人精神一振。".to_string()),
                ("WarmHands".to_string(), "你把雙手靠近大釜底座的耐火磚緣，溫熱厚重的火氣烘退了手掌上的水氣與寒意。".to_string()),
                ("SeasonalBanquet".to_string(), "你在紅鐵釜前添薪圍坐，鍋內歲時時令熱氣翻騰，同席之人舉杯共飲，暖意直透肺腑。".to_string()),
                ("AssistKitchen".to_string(), "你挽起衣袖在灶前添柴看火、揉麵分湯，協助大食堂備辦全城百工膳食。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_inn_milk_tea_stove" || target.contains("煮茶泥爐") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_inn_milk_tea_stove".to_string() },
            name: "煮茶泥爐".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Taste".to_string(), "RoastMountainTea".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "紅泥小火爐上坐著青砂茶吊，炭火微熾，正煨焙著北嶺霜林採擷的高山青茶，茶韻幽長甘醇。".to_string()),
                ("Taste".to_string(), "你端起粗陶茶盞抿了一口熱茶，炭焙香氣在喉間化開，微苦回甘。".to_string()),
                ("RoastMountainTea".to_string(), "你夾起松木炭添入泥爐，輕搖竹扇翻焙鐵碾上的高山青茶，炭焙茶香四溢。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_herb_drying_kiln" || target == "wt_herb_bamboo_sieve" || target.contains("紅泥焙藥爐") || target.contains("切藥鍘刀案") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_herb_drying_kiln".to_string() },
            name: if target.contains("鍘刀") || target.contains("竹篩") { "切藥鍘刀案".to_string() } else { "紅泥焙藥爐".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Operate".to_string(), "RoastHerbs".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "百草烘藥房中整齊排設著紅泥焙藥爐與切藥鍘刀案，竹篩上鋪滿風乾的九蒸艾草、黃精與北嶺草藥，藥香撲鼻。".to_string()),
                ("Use".to_string(), "你手持切藥鍘刀切分曬透的草根藥材，刀口起落均勻，切出平整藥片。".to_string()),
                ("Operate".to_string(), "你撥勻焙藥爐中的文火炭灰，控制烘乾火候，使百草藥性均勻透發。".to_string()),
                ("RoastHerbs".to_string(), "你依古法炮製百草，文火慢焙陳艾與黃精，封入綿紙藥包妥善製成艾絨。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_study_desk_inkstone" || target == "wt_study_bamboo_bookshelf" || target == "wt_obj_stone_rubbing_cabinet" || target.contains("烏木書案") || target.contains("線裝古籍架") || target.contains("碑拓石櫥") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_study_desk_inkstone".to_string() },
            name: if target.contains("古籍") || target.contains("書架") { "線裝古籍架".to_string() } else if target.contains("石櫥") || target.contains("碑拓") { "碑拓石櫥".to_string() } else { "烏木書案".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Read".to_string(), "SunBooks".to_string(), "CollateInscriptions".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "古雅書齋中陳設著沉水烏木書案、線裝古籍架與碑拓石櫥，整齊收藏著歷代金石拓片與水土誌略。".to_string()),
                ("Use".to_string(), "你端坐烏木書案前研開松煙墨汁，提筆抄錄卷冊上的警句註釋。".to_string()),
                ("Read".to_string(), "你自架上取下一卷線裝古籍細讀，考證半島水文地脈與前賢遺制。".to_string()),
                ("SunBooks".to_string(), "你搬動書架卷冊置於向陽石坪翻曬，通風驅蠹，拂拭古籍紙灰。".to_string()),
                ("CollateInscriptions".to_string(), "你對照石櫥碑拓與古籍印本，逐字核勘金石異同與漫漶殘字，審定拓本。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_obs_brass_telescope_rack" || target == "wt_book_map_reading_table" || target.contains("青銅窺管架") || target.contains("全圖閱覽桌") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_obs_brass_telescope_rack".to_string() },
            name: if target.contains("閱覽") || target.contains("地圖") { "全圖閱覽桌".to_string() } else { "青銅窺管架".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "Read".to_string(), "GrindOpticLens".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "觀星台與閱覽室內設有青銅窺管架與全圖閱覽桌，案旁備有細水研磨盤、金剛砂與紅土水晶鏡片毛胚。".to_string()),
                ("Operate".to_string(), "你微調青銅窺管架的仰角齒輪，對準遠方天際星位與海平線地標。".to_string()),
                ("Read".to_string(), "你在全圖閱覽桌上展閱半島星圖與等高線地勢總圖，參核各處隘口地標。".to_string()),
                ("GrindOpticLens".to_string(), "你蘸取細密金剛砂與冷泉活水，在旋轉磨盤上細細精磨紅土水晶鏡片，透光純淨明晰。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_market_produce_stand" || target == "wt_market_scale_counter" || target.contains("山海乾貨攤") || target.contains("黃銅雙盤秤") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_market_produce_stand".to_string() },
            name: if target.contains("秤") { "黃銅雙盤秤".to_string() } else { "山海乾貨攤".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Taste".to_string(), "Operate".to_string(), "SelectDriedSeafood".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "三水口海味乾貨行攤位上懸掛著金黃透亮的冷燻烏魚子、紫菜乾與乾貝，雙盤秤與精確砝碼擦拭得雪亮。".to_string()),
                ("Taste".to_string(), "你試嚐了一小片老頭家切下的薄烤烏魚子，醇濃黏齒、海味鮮香久久不散。".to_string()),
                ("Operate".to_string(), "你撥動黃銅雙盤秤上的滑碼與砝碼，稱量海味乾貨的斤兩份量，秤桿平準無差。".to_string()),
                ("SelectDriedSeafood".to_string(), "你迎著日光對光檢視烏魚子油潤成色，稱重包裝妥當，購下精美海味禮包。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_hall_council_table" || target == "wt_hall_bulletin_board" || target.contains("厚木公議桌") || target.contains("民務告示欄") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_hall_council_table".to_string() },
            name: if target.contains("告示") || target.contains("公佈") { "民務告示欄".to_string() } else { "厚木公議桌".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Sit".to_string(), "Read".to_string(), "DeliberateCivicPact".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "公議堂正中陳設著厚木公議桌，牆側立著民務告示欄，陳列著四坊公約草案、水利灌溉議程與公庫收支錄。".to_string()),
                ("Sit".to_string(), "你在公議桌旁厚木長椅落座，翻開桌上條陳，靜候街坊鄰里共商民事。".to_string()),
                ("Read".to_string(), "你閱覽告示欄與桌案上的鄉約條款，詳讀城坊水利分流與物料修防規定。".to_string()),
                ("DeliberateCivicPact".to_string(), "你與與會鄉紳街坊同席參議鄉約，各陳水土民生見解，共簽條約固本安民。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_river_stone_steps" || target == "wt_alley_stone_well" || target.contains("汲水青石階") || target.contains("青石古井") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_river_stone_steps".to_string() },
            name: if target.contains("井") { "青石古井".to_string() } else { "汲水青石階".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Wash".to_string(), "DrawSpringWell".to_string(), "Rest".to_string(), "Sit".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "沿岸石階浸入清冽山溪泉流，井台旁轆轤與青木水桶齊全，石隙間苔色青潤，水波盪漾。".to_string()),
                ("Use".to_string(), "你搖動轆轤打上一桶清澈泉水，水珠飛濺，清涼氣息拂面而來。".to_string()),
                ("Wash".to_string(), "你以冰涼甘冽的泉水清洗雙手與行裝泥塵，神清氣爽。".to_string()),
                ("DrawSpringWell".to_string(), "你自水岸公井汲取一竹筒清冽泉水飲用潤嗓，甘泉入口沁人心脾，頓消燥氣。".to_string()),
                ("Rest".to_string(), "你在沁涼的水岸青石階旁落座歇腳，聽著潺潺溪流拍岸，微風拂過面龐，行路疲憊頓消。".to_string()),
                ("Sit".to_string(), "你在沁涼的水岸青石階旁落座歇腳，聽著潺潺溪流拍岸，微風拂過面龐，行路疲憊頓消。".to_string()),
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
                .find(|obj| obj.id == target_id || obj.name == target_id || obj.name.contains(target_id) || target_id.contains(&obj.name))
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
    let cell = if let Some(coord) = parse_grid_room_id(room_id) {
        crate::server::grid_manager::get_runtime_grid_cell(coord)?
    } else {
        crate::server::grid_manager::get_runtime_grid_cell_by_name(room_id)?
    };
    let coord = cell.coord;

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
    // 0.9) 檢查私人內室／客房五大陳設器物（wt_reg_wenyao_home, wt_inn_bath_Preview, wt_home_shell_lamp, wt_home_jade_cabinet, wt_home_charcoal_wardrobe）
    let indoor_home_objs = [
        crate::model::RoomObject {
            id: "wt_reg_wenyao_home".to_string(),
            name: "床榻".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Rest".to_string(), "Sleep".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "寬大厚實的松香木大床上鋪著潔白乾爽的棉麻床單與蓬鬆的羽絨暖被，枕邊散發著淡淡的山野松香與陽光曝曬後的暖意。".to_string()),
                ("Rest".to_string(), "你在松香木大床邊坐下歇息，卸下隨身行囊，緊繃的肩背與小腿肌肉漸漸放鬆下來。".to_string()),
                ("Sleep".to_string(), "你吹熄床頭小燈，鑽入溫暖蓬鬆的羽絨暖被中沉沉睡去，一覺到天明，周身疲憊盡消。".to_string()),
                ("Use".to_string(), "你躺在鬆軟的羽絨暖被上閉目養神，呼吸著淡淡的松木清香，心神安寧。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
        crate::model::RoomObject {
            id: "wt_inn_bath_Preview".to_string(),
            name: "沐浴桶".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Wash".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "厚實的紅杉木浴桶內盛滿了冒著騰騰熱氣的清澈山泉溫水，水面上漂浮著幾片乾香草葉，散發著舒緩筋骨的溫潤水汽。".to_string()),
                ("Wash".to_string(), "你踏入溫熱的杉木浴桶中浸泡洗沐，溫潤泉水洗去滿身風塵與汗漬，四肢百骸說不出的舒暢通透。".to_string()),
                ("Use".to_string(), "你掬起溫熱的山泉水洗淨面龐與雙手，水汽氤氳間精神為之一振。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
        crate::model::RoomObject {
            id: "wt_home_shell_lamp".to_string(),
            name: "檯燈".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "書案上擺著一盞鑲嵌打磨夜光貝母薄片的黃銅旋蓋檯燈，柔和的珠光透過貝母散射開來，映照得室內溫馨靜謐。".to_string()),
                ("Operate".to_string(), "你輕輕旋動黃銅燈蓋調節光隙，室內的貝母柔光隨之流轉明滅，恰到好處地照亮案頭而不刺眼。".to_string()),
                ("Use".to_string(), "你湊近夜光貝母檯燈溫潤的光暈下靜心凝神，思緒格外澄澈清明。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
        crate::model::RoomObject {
            id: "wt_home_jade_cabinet".to_string(),
            name: "冰鑑櫃".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Open".to_string(),
                "StoreStash".to_string(),
                "RetrieveStash".to_string(),
                "Use".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "屋角立著一座由柯氏木作坊榫接外框、內嵌北山寒水玉薄板的雙層保鮮石櫃。櫃門縫隙間隱隱透出天然清冽的涼意，存於其中的鮮食與獵獲經宿不腐。".to_string()),
                ("Open".to_string(), "你拉開寒水玉雙層保鮮石櫃的黃銅插銷，一股沁人心脾的天然玉石涼氣撲面而來，裡頭分格擺放的茶點、鮮果與肉乾皆維持著初入櫃時的鮮潤色澤。".to_string()),
                ("StoreStash".to_string(), "你將隨身物資妥善安放於寒水玉石板上，玉面寒氣瞬時凝固鮮氣，封存妥當。".to_string()),
                ("RetrieveStash".to_string(), "你拉開石櫃底層玉匣，將封存保鮮的物資取回隨身行囊之中。".to_string()),
                ("Use".to_string(), "你將隨身乾糧與果品整理妥當，置入寒水玉保鮮石櫃的冰涼玉板上鎖住鮮氣，並取出一盞冰鎮泉露飲下，暑氣與焦渴頓時全消。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
        crate::model::RoomObject {
            id: "wt_home_bamboo_mailbox".to_string(),
            name: "信箱".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "Open".to_string(),
                "DispatchCourier".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "掛在門扉旁以青竹細編、外塗熟桐油防雨的立體竹筒信箱，開口設有銅扣翻蓋，專供半島旬報投遞與鄰里熟人託遞家書。".to_string()),
                ("Read".to_string(), "你掀開信箱防雨翻蓋，取出摺疊整齊的最新一期《半島旬報》細細閱覽。".to_string()),
                ("Open".to_string(), "你打開信箱銅扣，探手檢視內底，查閱是否有街坊鄰里或行腳客商留下的書信與心意回禮。".to_string()),
                ("DispatchCourier".to_string(), "你將備好的書信封妥投入信箱附設的快騎取件格，交付三枚鎂幣委請疾行草獸信使飛騎送出。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
        crate::model::RoomObject {
            id: "wt_home_charcoal_wardrobe".to_string(),
            name: "衣櫥".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "WarmHands".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "以耐潮樟木打造的立式衣櫥，底層設有岑氏工坊鍛造的百葉抽屜與白瓷炭缽，點燃無煙松炭後溫風徐徐上升，能徹底烘乾沾染山霧海風的濕重衣物。".to_string()),
                ("WarmHands".to_string(), "你旋開烘衣櫥底部的百葉氣孔，將微涼的雙手湊近溫潤帶香的松炭暖風旁烘烤，暖意順著指尖流遍全身。".to_string()),
                ("Use".to_string(), "你將略帶潮氣的外袍掛入松炭溫風烘衣櫥中烘烤片刻，再穿上時已變得蓬鬆乾爽、帶著淡淡的松木暖香，周身濕寒一掃而空。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
    ];
    if let Some(obj) = indoor_home_objs.into_iter().find(|obj| {
        obj.id == target
            || obj.name == target
            || obj.name.contains(target)
            || target.contains(&obj.name)
    }) {
        return Some(obj);
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
        if obj.name.contains("足印") || obj.name.contains("獸跡") || obj.id.contains("tracks") {
            let sockets = vec!["Look".to_string(), "Inspect".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "泥地石隙間留有一串清晰而深陷的野獸足印，蹄爪痕跡新鮮，周遭草葉微伏，顯然不久前曾有龐然巨物或敏捷靈獸由此踏過。".to_string()),
                ("Inspect".to_string(), "你伏身湊近仔細審視足印的爪痕開合、受力深淺與間距步幅，辨認出蹄印周邊殘留的泥痕與氣息，荒野行獸的蹤跡去向瞭然於胸。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.name.contains("岩龕") || obj.id.contains("niche") {
            let sockets = vec!["Look".to_string(), "Rest".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "厚實背風的山體天然凹陷處，石壁乾燥內斂，形成一座足以容身遮蔽風雨的山石岩龕，地面被往來旅人打掃得平整乾燥。".to_string()),
                ("Rest".to_string(), "你移步退入避風岩龕之內背石而坐，四周凜冽山風頓時被厚壁阻隔，呼吸沉緩調勻，渾身百骸漸次溫熱舒暢。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.name.contains("泉眼") || obj.name.contains("清泉") || obj.id.contains("spring") {
            let sockets = vec!["Look".to_string(), "Drink".to_string(), "Wash".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "清冽山泉自岩壁石隙間汩汩湧出，積聚成一泓澄澈見底的冷玉水窪，泉水清甘微涼，映著天光泛出粼粼水波。".to_string()),
                ("Drink".to_string(), "你俯身以掌心掬起一捧甘美冰涼的清泉痛飲，泉水清冽入喉直透臟腑，滿腹焦渴頓消，神清氣爽。".to_string()),
                ("Wash".to_string(), "你探手入冰涼的泉水中揉搓洗漱淨手，滌去一路攀跋的塵垢汗漬，指節肌理清涼微麻，神采倍感澄明。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.name.contains("木樁") || obj.id.contains("hunter_post") {
            let sockets = vec!["Look".to_string(), "Inspect".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "一截半人高的厚實硬木樁深釘於地，樁頭削成斜面，刻有防風雨的溝槽與巡山記號，周遭雜草已被過往獵手清整乾淨。".to_string()),
                ("Inspect".to_string(), "你湊近仔細審視木樁側面的刻痕與刮擦印記，上面留有獵團互通獵況的新舊刀痕與風雨標記，此方獵場的物候動靜一目了然。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.name.contains("木墩") || obj.id.contains("stepping_logs") {
            let sockets = vec!["Look".to_string(), "Rest".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "數截粗壯平整的硬木墩穩穩嵌在泥濘濕地中，表面斧鑿防滑刻痕，可供行人墊步落足、避開深沼。".to_string()),
                ("Rest".to_string(), "你踏上乾燥平穩的木墩坐下稍歇，甩去履底沾附的濕泥，迎著野徑清風調勻呼吸，腳力漸趨恢復。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.name.contains("石碑") || obj.name.contains("界碑") || obj.id.contains("signpost") || obj.id.contains("stele") {
            let sockets = vec!["Look".to_string(), "Read".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "大道旁矗立著風化古樸的四稜指路石碑，基座是一方厚重石礎，碑面經受風雨洗刷依然字跡深邃，指引著前路方隅。".to_string()),
                ("Read".to_string(), "你俯身撫平碑面石紋辨讀刻字，通往周遭市鎮、要隘與主幹古道的方向里數工整分明，路途去向瞭然於心。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.name.contains("營火") || obj.id.contains("campfire") {
            let sockets = vec![
                "Look".to_string(),
                "WarmHands".to_string(),
                "RoastFood".to_string(),
                "CraftTorch".to_string(),
                "AddFuel".to_string(),
                "CollectCharcoal".to_string(),
            ];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "粗壯硬柴交錯架起的野地營火堆正熊熊燃燒，金紅火苗隨風輕躍，散發著令人心神安定的融融暖意與松木焦香。".to_string()),
                ("WarmHands".to_string(), "你湊近營火伸出雙手，融融暖流自掌心順著經脈蔓延至周身四肢，驅散了荒野寒濕。".to_string()),
                ("RoastFood".to_string(), "你取出行囊中的乾糧肉脯架於熾熱柴炭上方炙烤，油脂滋滋作響、肉香四溢，趁熱嚼食腹中溫暖充實。".to_string()),
                ("CraftTorch".to_string(), "你自營火堆中抽出一根引燃的松脂硬木，細心纏裹封固，製成一柄劈啪作響的照明松明火把。".to_string()),
                ("AddFuel".to_string(), "你拾起乾柴添入火堆中央，劈啪脆響中火勢陡然騰起，騰躍的金紅火舌將營地周遭照耀得一片透亮。".to_string()),
                ("CollectCharcoal".to_string(), "你以長枝撥開外層浮灰，將底層悶燒透紅的硬木火炭小心夾出、封土隔氧冷卻，收整出成色極佳的無煙白炭。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        }

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

    // 2.5) 房間一等物件（RoomVenueObject）之櫃檯禁區與外借器物插座，以及活躍事件包（EventPack）關鍵器物插座
    if let Some(venue) = cell.venue_object() {
        if venue.has_counter_restricted_zone
            && (target == "櫃檯"
                || target == "櫃台"
                || target == "內場"
                || target == "wt_venue_counter"
                || target.contains("櫃檯")
                || target.contains("櫃台")
                || target.contains("非本店人員禁止進入區"))
        {
            let offers_str = if venue.counter_trade_offers.is_empty() {
                "各色常備器物與號內差事簿冊".to_string()
            } else {
                venue
                    .counter_trade_offers
                    .iter()
                    .map(|(_, name, price)| format!("{}（{}鎂）", name, price))
                    .collect::<Vec<_>>()
                    .join("、")
            };
            let sockets = vec![
                "Look".to_string(),
                "Inspect".to_string(),
                "Operate".to_string(),
            ];
            let active_pack_opt = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(&cell.name, None, now_fac);
            let mut inspect_desc = format!(
                "你隔著櫃檯端詳內場陳設，{}。若需採買或洽公，可直接向櫃檯內場或身著制服的{}開口交涉。",
                venue.trade_talk_pitch, venue.default_host_title
            );
            let mut operate_desc = format!(
                "櫃檯內側屬於【{}】當值作業管制區（非本店人員禁止進入區），你禮貌地在櫃檯外止步候詢。",
                venue.venue_kind
            );
            if let Some((ev_pack, sc_pack)) = &active_pack_opt {
                inspect_desc.push_str(&format!(
                    " 此外櫃檯案頭正擺放著事件包【{}】待辦簿冊（關鍵器物：{}；劇本包第 {}/3 幕）。",
                    ev_pack.event_title, ev_pack.key_objects, sc_pack.current_step.max(1)
                ));
                operate_desc.push_str(&format!(
                    " 你隔著櫃檯向內場{}示意協辦「{}」（關鍵器物：{}）。",
                    venue.default_host_title, ev_pack.event_title, ev_pack.key_objects
                ));
            }
            let responses = std::collections::HashMap::from([
                (
                     "Look".to_string(),
                     format!(
                         "這是【{}】的當值櫃檯與內場分界處，立有「非本店當值人員止步」木牌。{}平常便在此處{}。櫃上陳列著：{}。",
                         cell.name, venue.default_host_title, venue.on_duty_fixture_activity, offers_str
                     ),
                 ),
                 ("Inspect".to_string(), inspect_desc),
                 ("Operate".to_string(), operate_desc),
            ]);
            return Some(crate::model::RoomObject {
                id: "wt_venue_counter".to_string(),
                name: format!("{}・當值櫃檯", venue.venue_kind),
                owner: venue.default_host_title.to_string(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        }

        if let Some((tool_id, tool_name, lend_line)) = venue.lendable_tool
            && (target == tool_id
                || target == tool_name
                || (target.chars().count() >= 2 && (tool_name.contains(target) || target.contains(tool_name))))
        {
            let sockets = vec![
                "Look".to_string(),
                "Inspect".to_string(),
                "Use".to_string(),
            ];
            let responses = std::collections::HashMap::from([
                (
                     "Look".to_string(),
                     format!(
                         "【{}】常備於號內的「{}」，保養得宜、隨時可供{}與往來熟客借用。",
                         cell.name, tool_name, venue.default_host_title
                     ),
                 ),
                 (
                     "Inspect".to_string(),
                     format!(
                         "你細看這件「{}」，器身烙有號內印記。若需借用，可向{}開口（{}）。",
                         tool_name, venue.default_host_title, lend_line
                     ),
                 ),
                 (
                     "Use".to_string(),
                     format!(
                         "你順手試了試號內備置的「{}」，手感沉穩趁手（亦可透過借物向{}正式借取隨身攜行）。",
                         tool_name, venue.default_host_title
                     ),
                 ),
            ]);
            return Some(crate::model::RoomObject {
                id: tool_id.to_string(),
                name: tool_name.to_string(),
                owner: venue.default_host_title.to_string(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        }
    }

    if let Some((ev_pack, sc_pack)) =
        crate::game::peninsula_scripts::active_event_and_script_pack_for_room(&cell.name, None, now_fac)
    {
        let expected_event_obj_id = format!("wt_event_obj_{}", ev_pack.id.to_lowercase());
        let key_candidates: Vec<&str> = ev_pack
            .key_objects
            .split('、')
            .map(|seg| {
                seg.split('（')
                    .next()
                    .unwrap_or(seg)
                    .split('(')
                    .next()
                    .unwrap_or(seg)
                    .trim()
            })
            .filter(|k| !k.is_empty())
            .collect();
        let matched_key = if target.eq_ignore_ascii_case(&expected_event_obj_id) || target.starts_with("wt_event_obj_") {
            key_candidates.first().copied()
        } else {
            key_candidates
                .into_iter()
                .find(|k| *k == target || (target.chars().count() >= 2 && (k.contains(target) || target.contains(*k))))
        };
        if let Some(matched_key) = matched_key {
            let sockets = vec![
                "Look".to_string(),
                "Inspect".to_string(),
                "Operate".to_string(),
                "Use".to_string(),
            ];
            let responses = std::collections::HashMap::from([
                (
                     "Look".to_string(),
                     format!(
                         "【事件包・{}】眼前的「{}」正是場上差事的關鍵器物：{}。",
                         ev_pack.event_title, matched_key, ev_pack.trigger_and_causal_chain
                     ),
                 ),
                 (
                     "Inspect".to_string(),
                     format!(
                         "你湊近檢視「{}」，只見現場正待推進劇本包「{}」：{}。",
                         matched_key, ev_pack.event_title, sc_pack.player_actions
                     ),
                 ),
                 (
                     "Operate".to_string(),
                     format!(
                         "你動手操作「{}」配合現場差事：{}（{}）。",
                         matched_key, sc_pack.player_actions, sc_pack.outcome_and_world_change
                     ),
                 ),
                 (
                     "Use".to_string(),
                     format!(
                         "你運用「{}」協助推進「{}」：{}。",
                         matched_key, ev_pack.event_title, sc_pack.outcome_and_world_change
                     ),
                 ),
            ]);
            return Some(crate::model::RoomObject {
                id: format!("wt_event_obj_{}", ev_pack.id.to_lowercase()),
                name: matched_key.to_string(),
                owner: sc_pack.host_slot_actor,
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        }
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
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, total_pay);
            push_refresh(conn, pid);

            let (others, move_target_id) = object_followup_buttons(
                &obj,
                player_room,
                action,
                &prior_scene_actions,
                &prior_scene_states,
            );
            let narrative = format!(
                "你將收割整齊的 {} 束蘆草一次性齊齊交到秤邊，秤手熟練撥動秤錘：「正合分量！這是結算給你的 {} 鎂工錢。」",
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
    if action == "Look" {
        if let Some(read_text) = obj.responses.get("Read") {
            let read_clean = read_text.trim();
            if !read_clean.is_empty() && !narrative.contains(read_clean) {
                let mut cleaned = read_clean;
                const PREFIXES: &[&str] = &[
                    "你走近告示欄細讀通告，",
                    "你走近公佈欄細閱，",
                    "你細讀櫃檯告示：",
                    "你細讀櫃頂張貼的三水總商行通兌章程，",
                    "你走近石柱研讀鐫刻的方位路引：",
                    "你走近立柱端讀鐫刻的商貿契稅規章與每日各港口船班、車隊發貨時序。",
                    "你細讀帳冊上的公道行情：",
                    "你細閱屋契條款，",
                    "你逐條細讀告示內容，",
                    "你逐行研讀告示板上的急件委託，",
                ];
                for &pfx in PREFIXES {
                    if let Some(rest) = cleaned.strip_prefix(pfx) {
                        cleaned = rest.trim();
                        break;
                    }
                }
                if let Some(rest) = cleaned.strip_prefix("你走近") {
                    if let Some(pos) = rest.find("細閱，") {
                        cleaned = rest[pos + "細閱，".len()..].trim();
                    } else if let Some(pos) = rest.find("細讀，") {
                        cleaned = rest[pos + "細讀，".len()..].trim();
                    } else if let Some(pos) = rest.find("細閱：") {
                        cleaned = rest[pos + "細閱：".len()..].trim();
                    } else if let Some(pos) = rest.find("細讀：") {
                        cleaned = rest[pos + "細讀：".len()..].trim();
                    }
                } else if let Some(rest) = cleaned.strip_prefix("你細讀") {
                    if let Some(pos) = rest.find('：') {
                        cleaned = rest[pos + '：'.len_utf8()..].trim();
                    }
                } else if let Some(rest) = cleaned.strip_prefix("你細閱") {
                    if let Some(pos) = rest.find('：') {
                        cleaned = rest[pos + '：'.len_utf8()..].trim();
                    }
                }
                if !narrative.contains(cleaned) {
                    if !narrative.ends_with('。') && !narrative.ends_with('！') && !narrative.ends_with('？') {
                        narrative.push('。');
                    }
                    narrative.push_str(cleaned);
                }
            }
        }
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
            "你俯身清理合社圍獵【{}】後留下的現場，細心從殘骨與草叢間剝取收納了新鮮獸材【{}】放入行囊妥善收整，現場餘溫漸漸散去。",
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
    if obj.id.starts_with("detritus_fungi_") {
        let (gx, gy) = if let Some(rest) = obj.id.strip_prefix("detritus_fungi_") {
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
            let narrative = "此處曾有獸類殘骸與倒伏古木深埋於腐殖土壤中，歷經天地清氣與地底菌絲長久分解滋養，如今已催生出一簇生機盎然的【腐木牛樟靈菇叢】。赤紅如珊瑚般的靈芝菇蓋微吐芬芳，可執行【採集 (Harvest)】採摘收納。".to_string();
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
            if let Some((item_id, item_name)) = crate::game::harvest_zone_detritus_fungi(gx, gy) {
                let _ = db::add_to_inventory(pid, item_id, 1);
                let narrative = format!(
                    "你蹲下身軀小心撥開腐土與厚實苔蘚，運勁將生長於朽木根部的【{}】整株完整採摘起，收入隨身藥簍——泥土中殘留的腐殖清氣漸隨風散入山林土壤。",
                    item_name
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
                    narrative: "此處靈菇已被採摘一空，僅餘濕潤腐土與散落菌絲，需待山林水土再度滋養才能重新萌發。".to_string(),
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
            && !narrative.contains("原野清氣")
        {
            let room_label = resolve_room_name(player_room);
            if let Some(omen) = crate::game::dynamic_hunting_zone_ecology_omen(&room_label) {
                narrative.push('\n');
                narrative.push_str(&omen);
            }
            if (obj.id == "shanhai_apex_trace"
                || obj.name.contains("霸主獸跡")
                || obj.name.contains("獸徑足印"))
                && let Some((gx, gy)) = db::get_entity(pid).ok().flatten().and_then(|e| e.grid_x.zip(e.grid_y))
            {
                crate::game::mark_apex_beast_tracked_at(gx, gy);
                narrative.push_str(
                    "\n順著地上深陷的爪印與殘留氣息撥開深林草莽，暗處一雙威凜獸瞳驟然亮起——潛伏此地的山海巨獸已現身！",
                );
                refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                push_refresh(conn, pid);
            }
            if obj.name.contains("山澗清泉眼") || obj.name.contains("避風岩龕") {
                append_somatic_awareness_note(pid, now, &mut narrative);
            }
        }
    }
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
    if state_written && m3_027_scene_object_apply_side_effects(pid, player_room, &obj.id, action, state_payload)
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
            || obj.id == "wt_cat_0786"
            || obj.name.contains("閱報")
            || obj.name.contains("旬報")
            || obj.id == "wt_home_bamboo_mailbox"
            || obj.name.contains("竹編信箱")
            || obj.id == "wt_courier_post_desk"
            || obj.name.contains("信使櫃檯")
            || obj.id == "wt_cat_0790"
            || obj.id == "wt_bank_ledger_drawer"
            || obj.name.contains("存摺")
            || obj.name.contains("票號")
            || obj.id == "wt_guild_commission_board"
            || obj.id == "wt_hall_bulletin_board"
            || obj.id == "wt_cat_0794"
            || obj.name.contains("委託")
            || obj.name.contains("告示")
            || obj.name.contains("公佈欄")
            || obj.id == "wt_harbor_insurance_desk"
            || obj.name.contains("海事承保")
            || obj.name.contains("海事保單")
            || obj.id == "wt_harbor_clockwise_ship"
            || obj.name.contains("順時針環島海船")
            || obj.name.contains("順時針環島海運商船")
            || obj.name.contains("遠洋順流商船")
            || obj.id == "wt_home_jade_cabinet"
            || obj.name.contains("冰鑑櫃")
            || obj.name.contains("寒水玉保鮮石櫃")
            || obj.name.contains("寒水玉雙層保鮮石櫃")
            || obj.name.contains("松炭溫風烘衣櫥")
            || obj.id == "wt_coastal_angling_spot"
            || obj.id == "wt_cat_1006"
            || obj.name.contains("釣竿")
            || obj.name.contains("垂釣")
            || obj.id == "wt_lookout_telescope"
            || obj.id == "wt_cat_0733"
            || obj.name.contains("望遠鏡")
            || obj.id == "wt_field_camp_stove"
            || obj.id == "wt_cat_1010"
            || obj.name.contains("便攜爐")
            || obj.id == "wt_field_crop_plot"
            || obj.name.contains("田壟作物")
            || obj.name.contains("當季田壟"))
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
        if !narrative.is_empty() && !narrative.contains("【事件包與劇本包聯動・") {
            let room_name = resolve_room_name(player_room);
            let full_advance_opt =
                crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object_full(
                    &room_name,
                    &obj.id,
                    &obj.name,
                    action,
                    now,
                );
            if let Some((_ev, _sp, pack_note)) = full_advance_opt {
                narrative.push_str(&pack_note);
            } else if (obj.id == "wt_venue_counter" || obj.id.starts_with("wt_event_obj_"))
                && action != "Look"
            {
                if let Some((ev, _sp)) =
                    crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
                        &room_name,
                        None,
                        now,
                    )
                {
                    let first_key = ev
                        .key_objects
                        .split('、')
                        .next()
                        .map(|s| s.trim())
                        .unwrap_or("");
                    if !first_key.is_empty() {
                        if let Some((_ev, _sp, pack_note)) =
                            crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
                                &room_name,
                                first_key,
                            )
                        {
                            if !pack_note.starts_with('\n') {
                                narrative.push('\n');
                            }
                            narrative.push_str(&pack_note);
                        }
                    }
                }
            }
        }
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
        if let Some(cell) = crate::server::grid_manager::get_runtime_grid_cell(coord) {
            return cell.name.clone();
        }
    }
    player_room.to_string()
}

fn current_zone_weather(room_name: &str) -> crate::game::ZoneWeatherPhase {
    let z_idx = crate::game::resolve_zone_idx_from_name_or_settlement(room_name);
    let tick = (crate::game::now_unix() / 180) as i64;
    crate::game::current_zone_weather_and_tide(z_idx, tick).0
}

fn current_zone_tide(room_name: &str) -> crate::game::ZoneTidePhase {
    let z_idx = crate::game::resolve_zone_idx_from_name_or_settlement(room_name);
    let tick = (crate::game::now_unix() / 180) as i64;
    crate::game::current_zone_weather_and_tide(z_idx, tick).1
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
        || obj.name.contains("病榻")
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

    if !obj.id.starts_with("wt_")
        && !obj.name.contains("紅印屋契")
        && !obj.name.contains("委託")
        && !obj.name.contains("告示")
        && !obj.name.contains("公佈欄")
        && !obj.name.contains("海事")
        && !obj.name.contains("海船")
        && !obj.name.contains("石櫃")
        && !obj.name.contains("烘衣櫥")
        && !obj.name.contains("田壟")
        && !obj.name.contains("苗畦")
        && !is_medical_or_rest
        && action != "Rest"
        && action != "Meditate"
    {
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
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -25);
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
        narrative.push_str("\n【觀星測韻】你輕輕轉動三層黃銅星環，黃道規線與子午環在齒輪咬合間微調校準。望筒深處映現星軌明滅，清冷星光透體而過，你只覺靈台澄澈、元氣湧動，獲得星軌校準領悟。");
        append_somatic_awareness_note(pid, now, narrative);
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
        && (action == "Operate" || action == "Read" || action == "Open" || action == "Use" || action == "Look" || action == "DepositBank" || action == "WithdrawBank")
    {
        let magnesium = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| e.magnesium)
            .unwrap_or(0);
        let bank_balance = crate::npc::causal_core::get_player_bank_balance(pid);

        if action == "DepositBank" || action == "Operate" {
            let (actual, new_pocket, new_bank) = crate::npc::causal_core::deposit_player_bank_magnesium(pid, 10);
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "bank_deposit");
            if actual > 0 {
                narrative.push_str(&format!(
                    "\n【存入票號妥當】商行掌事核對成色後將 {} 鎂幣入帳歸檔，並在你的分格存摺上用印簽章。\n本次存入：{} 鎂幣\n隨身攜帶資財：{} 鎂幣\n票號存摺存餘：{} 鎂幣",
                    actual, actual, new_pocket, new_bank
                ));
            } else {
                narrative.push_str(&format!(
                    "\n【存入未成】隨身囊中未有足額鎂幣可供存入。目前隨身持有：{} 鎂幣，票號存摺存餘：{} 鎂幣。",
                    new_pocket, new_bank
                ));
            }
        } else if action == "WithdrawBank" || action == "Use" {
            let (actual, new_pocket, new_bank) = crate::npc::causal_core::withdraw_player_bank_magnesium(pid, 10);
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "bank_withdraw");
            if actual > 0 {
                narrative.push_str(&format!(
                    "\n【憑摺支取完成】商行掌事驗看存摺印信後，自保險庫櫃中點出 {} 鎂幣如數交到你手中。\n本次支取：{} 鎂幣\n隨身攜帶資財：{} 鎂幣\n票號存摺存餘：{} 鎂幣",
                    actual, actual, new_pocket, new_bank
                ));
            } else {
                narrative.push_str(&format!(
                    "\n【支取未成】你在票號存摺中目前尚無結存資財。隨身持有：{} 鎂幣，票號存摺存餘：{} 鎂幣。",
                    new_pocket, new_bank
                ));
            }
        } else {
            // Look / Read / Open
            let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "bank_ledger_checked");
            narrative.push_str(&format!(
                "\n【三水商行・分格存摺簿】你翻開三水總商行分格存摺櫃內的通兌流水簿與防偽印摺，各鎮分號硃印嚴謹無訛。\n三水總商行通存通兌狀態：正常營運。\n你目前隨身攜帶資財：{} 鎂幣\n票號存摺累計存餘：{} 鎂幣\n憑此存摺印信，可在半島各重鎮商行隨時辦理【存入票號】或【憑摺支取】。",
                magnesium, bank_balance
            ));
        }
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
        narrative.push_str("\n【修容舒爽】熱毛巾敷面溫潤身心，精鋼剃刀輕柔修整鬚角與碎髮，再經紫銅管溫水沖洗，整個人容光煥發、精神抖擻。");
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
        narrative.push_str("\n【珠算實習】你拂去烏木檔間微塵，凝神運指撥動竹骨算珠，隨口訣起落核算進出貨單，珠聲清脆如落玉，心神一片專注沉靜。");
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
        narrative.push_str("\n【試招切磋】你緊縛牛皮護腕，手執裹軟革的練習木刀擺開架式，依口令與石靶連續對拆劈刺卸力，步法靈動沉穩，筋骨徹底舒展，體魄更顯堅實。");
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
                    remaining_stock: _,
                    seller_mg_after: _,
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
                            "\n你交付 {price_mg} 枚鎂幣向攤主買下「{item_name}」，當場品嚐幾口並將餘份妥善收入行囊。"
                        ));
                    } else {
                        *narrative = format!(
                            "你走向「{}」，支付 {price_mg} 鎂向攤主買下一份「{item_name}」收進背包，順道溫養氣血，熱騰騰的香氣令人精神一振。",
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

    // 2.1) Merchant Ledger Counter (厚樟木商契掌事櫃檯：商契蓋印與核銷)
    if (obj.id == "wt_merchant_ledger_counter" || obj.name.contains("商契掌事櫃台") || obj.name.contains("商契掌事櫃檯"))
        && (action == "Use" || action == "Operate")
    {
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 10);
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        narrative.push_str("\n【商契蓋印】你在厚樟木掌事櫃檯前出示商行契據，值班執事仔細核對無誤後在契本上重重蓋下硃砂印鑑，完成商契核銷與物資調撥紀錄（領取 10 鎂核銷酬勞）。");
        push_refresh(conn, pid);
        return;
    }

    // 2.15) Caravan Transit Hub (商隊聯運票務櫃檯：跨聚落快速聯運)
    if (obj.id == "wt_caravan_ticket_counter"
        || obj.name.contains("聯運班次")
        || obj.name.contains("車票")
        || obj.name.contains("票務櫃檯")
        || obj.name.contains("票務櫃台"))
        && (action == "Use" || action == "Operate" || action == "Move")
    {
        const TRANSIT_STOPS: &[(&str, i32, i32, &str)] = &[
            ("文曜", 0, -1, "三水交匯的學風重鎮，車馬喧囂，書香與市井氣息交融"),
            ("水尾寮", 0, 125, "白塔北境清泉奔湧的堰頭樞紐，水汽氤氳，農人引水灌田"),
            ("半嶺鎮", 160, 210, "半島東北地勢起伏的丘陵山鎮，茶香撲鼻，山徑蜿蜒"),
            ("紅土崁", -180, -140, "西南灌溉渠網縱橫的沃土農莊，桑麻遍野，機杼聲聲"),
            ("南哨", 0, -220, "半島最南端風蝕礁岩林立的哨所防線，海風呼嘯，崗樓巍峨"),
            ("風舶角", -260, 160, "西北面臨怒海狂濤的深水海港，桅檣如林，鹽霧瀰漫"),
            ("霜林口", 190, 290, "北坡高聳入雲的雲杉林場聚落，水車排鋸轟鳴，松脂木香四溢"),
            ("下營", 0, -450, "半島極南險隘要衝，扼守山川門戶，商旅車隊往來絡繹"),
            ("烏石鼻", -260, 35, "黑曜玄武岩壁立的迎浪漁港，石階泊位井然，海風鹹鮮"),
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

        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -5);
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
            "\n【商隊聯運客運】你在聯運櫃檯點付 5 鎂車資，執事為你取來竹編乘輿牌。你登上厚棚雙輪大車，車伕一揚皮鞭，健碩的挽獸邁開大步，車輪在鋪石道與土徑上轆轆作響。行過半島層疊丘陵與原野，約莫半小時後順利抵達【{}】（{}）！",
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

        let active_pack = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
            &room_name,
            None,
            now,
        );
        let player_name = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| if !e.display_title.trim().is_empty() { e.display_title } else { e.id })
            .unwrap_or_else(|| "外來行客".to_string());

        let active_orders = crate::npc::causal_core::list_active_stranded_cargo_orders(now);
        let has_unclaimed = active_orders.iter().any(|o| o.claimed_by.is_none());
        let player_holds_stranded = active_orders
            .iter()
            .any(|o| db::inventory_has_item(pid, &o.item_id));

        // 1. 揭榜受滯貨單
        if action == "ClaimStrandedCargo"
            || (action == "Operate" && has_unclaimed && !player_holds_stranded)
        {
            if let Some(order) = crate::npc::causal_core::claim_stranded_cargo_for_player(
                pid,
                &player_name,
                "",
                &room_name,
                now,
            ) {
                let _ = db::add_to_inventory(pid, &order.item_id, order.qty.max(1));
                let _ = event::append(
                    now,
                    pid,
                    types::SCENE_OBJECT_ACTION,
                    &format!("{}|claim_stranded_cargo|{}", obj.id, order.order_id),
                );
                narrative.push_str(&format!(
                    "\n【封蠟行會受滯貨單・揭榜管運】\n你揭下告示板上的「{}」急遞單，接管了由行商【{}】託運的 {} 份【{}】！\n因途經險阻，貨物急需送達目的地【{}】公議堂／行會驗收。\n你將物資與封蠟單據妥善收入行囊，承諾護送前往交割；送達後將由目的聚落公庫足額核撥 {} 鎂託管賞格！",
                    order.order_id,
                    order.carrier_name,
                    order.qty.max(1),
                    order.item_name,
                    order.dest_settlement,
                    order.escrow_bounty_mg
                ));
                push_refresh(conn, pid);
                return;
            } else if action == "ClaimStrandedCargo" {
                narrative.push_str("\n【行會提示】目前全島各驛站受滯貨單皆已由巡守隊或行客接洽接管，暫無待揭榜之貨單。");
                push_refresh(conn, pid);
                return;
            }
        }

        // 2. 交割受滯貨單
        if action == "DeliverStrandedCargo"
            || ((action == "Deliver" || action == "Use") && player_holds_stranded)
        {
            if player_holds_stranded
                && let Some((order, bounty)) = crate::npc::causal_core::deliver_stranded_cargo_for_player(
                pid,
                &player_name,
                "",
                &room_name,
                now,
            ) {
                let _ = db::remove_from_inventory(pid, &order.item_id, order.qty.max(1));
                let _ = event::append(
                    now,
                    pid,
                    types::SCENE_OBJECT_ACTION,
                    &format!("{}|deliver_stranded_cargo|{}", obj.id, order.order_id),
                );
                narrative.push_str(&format!(
                    "\n【封蠟行會受滯貨單・交割銷單】\n你向告示板掌事書吏出示受滯貨單，將護送抵埠的 {} 份【{}】清點交付！\n目的地【{}】之物資緊缺之勢隨之大為舒緩，行商【{}】得訊後感激不盡，連聲道謝。\n公庫託管賞格 {} 鎂已足額核撥結算至你的隨身行囊！",
                    order.qty.max(1),
                    order.item_name,
                    order.dest_settlement,
                    order.carrier_name,
                    bounty
                ));
                push_refresh(conn, pid);
                return;
            } else if action == "DeliverStrandedCargo" {
                narrative.push_str("\n【交割提示】你目前隨身未持有需在此聚落交割之受滯貨物，或尚未抵達貨單指定之目的聚落。");
                push_refresh(conn, pid);
                return;
            }
        }

        // 3. 檢查玩家行囊是否備有本地急缺物資（僅在玩家主動交付/操作時觸發）
        let delivered_item = if action == "Use" || action == "Operate" || action == "Deliver" {
            player_inv.as_ref().and_then(|inv| {
                // 優先比對：本地最緊缺的供需短缺物產
                if let Some((s_id, s_name, _cur_stock, _tgt_stock, price)) = &scarce_opt {
                    if let Some(_it) = inv.items.iter().find(|i| &i.item_id == s_id && i.qty > 0) {
                        return Some((s_id.clone(), s_name.clone(), *price));
                    }
                }
                // 次要比對：當前街坊公議腳本之關鍵物產（key_objects 明確提及者）
                if let Some((ev_pack, _sc_pack)) = &active_pack {
                    let clean_keys = clean_catalog_codes(ev_pack.key_objects);
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
                        if clean_keys.contains(&item_name) || ev_pack.key_objects.contains(&it.item_id) {
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
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, &room_name, bounty_mg);
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
                let (host_opt, _) = crate::game::peninsula_scripts::resolve_on_duty_actors_for_room(&room_name);
                let host_label = host_opt.unwrap_or_else(|| "本坊當值掌事".to_string());
                let mentor_key = format!("{room_name}{host_label}");
                let (imprint, newly_acquired) = crate::pg::living_world::record_mentor_imprint_progress(
                    pid,
                    &mentor_key,
                    "百工衡準訣",
                    "熟稔半島各坊物材成色與發力分寸，運器施作更為穩健",
                    now,
                );
                let imprint_note = if imprint.interaction_count == 1 {
                    format!(
                        "\n【百工指點】{host_label}見你交割物資成色地道，順口指點了你幾句《{}》（{}）的發力分寸。",
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
                        author_name: host_label.clone(),
                        formal_name: imprint.imprint_title.clone(),
                        slot_label: "百工衡準".to_string(),
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
                        "\n【百工師承・心領神會】{host_label}端詳物材成色，讚許點頭，贈予你一卷親筆手書的《{}》手札與空白手札，並手把手點撥你辨識各坊物性與運力分寸——領悟師承烙印【{}】！（{}）",
                        imprint.imprint_title,
                        imprint.imprint_title,
                        imprint.imprint_prose
                    )
                } else {
                    format!(
                        "\n【百工掌事指點】{host_label}稱善：「物材質地極佳，解了工坊燃眉之急！」其言簡意賅的指點令你對【{}】的體會又深了一層。",
                        imprint.imprint_title
                    )
                };
                narrative.push_str(&format!(
                    "\n【本坊急缺物資交割完成】你依行會告示板之急徵令，自隨身行囊交付一份急缺的【{item_name}】供本地工坊應急！\n{host_label}點收後蓋印銷單，當場核發交割懸賞 {bounty_mg} 鎂酬勞，本坊物資緊缺之勢隨之舒緩。{imprint_note}"
                ));
                push_refresh(conn, pid);
                return;
            }
        }

        // 4. 未攜帶急缺物資：展示本坊街坊公議、即時供需短缺報告、受滯貨單與山道靖安通報
        let script_section = if let Some((ev, sp)) = &active_pack {
            let clean_objects = clean_catalog_codes(ev.key_objects);
            format!(
                "【街坊公議與物資徵集・{}】\n・主事場所：{}（{}）\n・徵集物產：{}\n・事由始末：{}\n・協力途徑：{}",
                ev.event_title, sp.location_context, sp.host_slot_actor, clean_objects, ev.trigger_and_causal_chain, sp.player_actions
            )
        } else {
            "【街區公務告示】目前各坊市當家正按各自班表出工，暫無加急公務告示。".to_string()
        };

        let shortage_notice = if let Some((_, scarce_name, _cur_stock, _tgt_stock, _unit_price)) = &scarce_opt {
            format!("【本坊急徵物資】目前急徵「{}」，行會特設專項酬勞廣納應急物產。\n{}", scarce_name, supply_demand_report)
        } else {
            format!("【本坊急徵物資】本地物產暫無嚴重缺口，常備物資平穩運轉中。\n{}", supply_demand_report)
        };

        let stranded_section = if active_orders.is_empty() {
            "【封蠟行會急遞・受滯貨單】\n目前全島商路平穩，各驛站暫無滯留貨單。".to_string()
        } else {
            let mut lines = vec!["【封蠟行會急遞・受滯貨單】".to_string()];
            for ord in active_orders.iter().take(3) {
                let claim_tag = match &ord.claimed_by {
                    Some(claimer) if claimer == pid => "【你管運中】",
                    Some(_) => "【已有行客管運】",
                    None => "【待揭榜承領】",
                };
                lines.push(format!(
                    "・{} 行商「{}」滯留於「{}」：急需送達「{}」之「{}」，公庫託管賞格 {} 鎂",
                    claim_tag,
                    ord.carrier_name,
                    ord.waystation_name,
                    ord.dest_settlement,
                    ord.item_name,
                    ord.escrow_bounty_mg
                ));
            }
            lines.join("\n")
        };

        const MAJOR_PASS_ZONES: &[&str] = &["赤石坡", "西麓林道", "清泉溪谷", "長風草甸", "古河巨木林"];
        let mut spillover_lines = Vec::new();
        for &zone_name in MAJOR_PASS_ZONES {
            let z_idx = crate::game::resolve_zone_idx_from_name_or_settlement(zone_name);
            if crate::game::is_zone_predator_spillover_active(z_idx) {
                spillover_lines.push(format!("・【{}】猛獸受驚越界伏行，山道行旅受阻，請往來商隊與巡行者多加戒備！", zone_name));
            }
        }
        let spillover_section = if spillover_lines.is_empty() {
            "【山道靖安通報】\n各主要關隘與山道巡防井然，暫無猛獸越界侵擾通報。".to_string()
        } else {
            format!("【山道靖安通報】\n{}", spillover_lines.join("\n"))
        };

        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        narrative.push_str(&format!(
            "\n{}\n\n{}\n\n{}\n\n{}",
            script_section, shortage_notice, stranded_section, spillover_section
        ));
        push_refresh(conn, pid);
        return;
    }

    // 海事承保櫃檯 (築港頭分級海事保單)
    if obj.id == "wt_harbor_insurance_desk"
        || obj.name.contains("海事承保")
        || obj.name.contains("海事保單")
    {
        let room_name = resolve_room_name(player_room);
        let weather = current_zone_weather(&room_name);
        let tide = current_zone_tide(&room_name);
        let (host_opt, _) = crate::game::peninsula_scripts::resolve_on_duty_actors_for_room(&room_name);
        let host_label = host_opt.unwrap_or_else(|| "海事櫃檯當值掌事".to_string());

        if action == "Look" || action == "Read" {
            let weather_desc = match weather {
                crate::game::ZoneWeatherPhase::RainMist => "煙雨空濛（風浪驟起、海潮翻湧）",
                crate::game::ZoneWeatherPhase::ClearBreeze => "微風拂面（碧波蕩漾、順風好行）",
                crate::game::ZoneWeatherPhase::DryWarm => "天青氣暖（海平浪靜、航路平穩）",
                crate::game::ZoneWeatherPhase::ColdFrost => "寒霜凜冽（洋流偏冷、海霧低垂）",
            };
            let tide_desc = match tide {
                crate::game::ZoneTidePhase::FloodTide => "滿潮（潮位高漲、順流起航）",
                crate::game::ZoneTidePhase::EbbTide => "退潮（落潮回流、航道平緩）",
                crate::game::ZoneTidePhase::SlackTide => "平潮（水勢止息、泊港安穩）",
            };
            narrative.push_str(&format!(
                "\n【築港頭海事承保櫃檯】\n碼頭棧橋旁的厚木櫃檯前擺放著分級海事保單與氣象籌板，由掌事{}親自主持水險承保。\n凡出海客商繳納 6 鎂契費，即可簽立一份防水桐油麻布「築港頭分級海事保單」；若航程遇狂風驟雨致使貨裝海損，抵埠憑單可自風舶角公庫足額領取 12 鎂理賠撫卹。\n當前外海水相氣候：{}\n當前港灣潮汐相位：{}",
                host_label, weather_desc, tide_desc
            ));
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|{}", obj.id, action));
            push_refresh(conn, pid);
            return;
        }

        if action == "Use" || action == "Operate" || action == "Buy" {
            let magnesium = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.magnesium)
                .unwrap_or(0);
            if magnesium < 6 {
                narrative.push_str(&format!(
                    "\n【承保提示】簽立「築港頭分級海事保單」需 6 鎂契費，隨身錢資不足，承保掌事{}請你備妥盤纏再來投保。",
                    host_label
                ));
                push_refresh(conn, pid);
                return;
            }
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, "風舶角", -6);
            let _ = db::add_to_inventory(pid, "wt_cat_0792", 1);
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|buy_marine_insurance", obj.id));
            narrative.push_str(&format!(
                "\n【簽立海事水險】你在掌事{}面前交付 6 鎂契費，於防水桐油麻布保單上簽章畫押，將一份「築港頭分級海事保單」妥善收妥入囊。\n{}蓋上硃砂大印叮嚀道：「若出海途中遭遇狂風暴雨造成海損，抵港憑單可自風舶角公庫領取 12 鎂理賠撫卹。」",
                host_label, host_label
            ));
            push_refresh(conn, pid);
            return;
        }
    }

    // 順時針環島海運商船
    if obj.id == "wt_harbor_clockwise_ship"
        || obj.name.contains("順時針環島海船")
        || obj.name.contains("順時針環島海運商船")
        || obj.name.contains("遠洋順流商船")
    {
        const CLOCKWISE_PORTS: &[(&str, i32, i32, &str)] = &[
            ("烏石鼻", -260, 35, "西南玄武岩黑石岬角，黑潮分流繞行此處"),
            ("風櫃尾", -360, 95, "西海岸風櫃海潮怒吼之險峻海岬，水深流急"),
            ("風舶嶼", -330, 160, "西北離岸島嶼，海面開闊，商船泊靠候潮"),
            ("風舶角", -260, 160, "半島西北深水良港與海運樞紐，萬船雲集"),
            ("南哨", 0, -220, "正南邊陲海疆汛防海港，暖風習習"),
            ("水尾寮", -75, -280, "南端水岸渡口與水運集散良澳，舟楫往來"),
        ];

        let room_name = resolve_room_name(player_room);
        let weather = current_zone_weather(&room_name);
        let tide = current_zone_tide(&room_name);

        let ent = db::get_entity(pid).ok().flatten();
        let (cur_x, cur_y) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((-260, 160));

        let cur_port_idx = CLOCKWISE_PORTS.iter().position(|(name, x, y, _)| {
            room_name.contains(name) || ((cur_x - *x).abs() <= 40 && (cur_y - *y).abs() <= 40)
        });
        let next_idx = match cur_port_idx {
            Some(idx) => (idx + 1) % CLOCKWISE_PORTS.len(),
            None => 0,
        };
        let (next_name, next_x, next_y, next_desc) = CLOCKWISE_PORTS[next_idx];

        if action == "Look" || action == "Read" {
            let weather_desc = match weather {
                crate::game::ZoneWeatherPhase::RainMist => "煙雨空濛（風浪驟起、海潮翻湧）",
                crate::game::ZoneWeatherPhase::ClearBreeze => "微風拂面（碧波蕩漾、順風好行）",
                crate::game::ZoneWeatherPhase::DryWarm => "天青氣暖（海平浪靜、航路平穩）",
                crate::game::ZoneWeatherPhase::ColdFrost => "寒霜凜冽（洋流偏冷、海霧低垂）",
            };
            let tide_desc = match tide {
                crate::game::ZoneTidePhase::FloodTide => "滿潮（潮位高漲、順流起航）",
                crate::game::ZoneTidePhase::EbbTide => "退潮（落潮回流、航道平緩）",
                crate::game::ZoneTidePhase::SlackTide => "平潮（水勢止息、泊港安穩）",
            };
            narrative.push_str(&format!(
                "\n【順時針環島海運商船】\n高大結實的雙桅商船泊於碼頭石階旁，順著西岸暖流、北境白塔海角與東部黑潮，依序順時針巡航於全島各海港。\n下一停靠港口：【{}】（{}）\n當前海象氣候：{}\n當前潮汐相位：{}\n船資單程 8 鎂，順流平穩，客貨兩便。",
                next_name, next_desc, weather_desc, tide_desc
            ));
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|{}", obj.id, action));
            push_refresh(conn, pid);
            return;
        }

        if action == "Use" || action == "Operate" || action == "Board" || action == "BoardCoastalShip" {
            let magnesium = ent.as_ref().map(|e| e.magnesium).unwrap_or(0);
            if action == "BoardCoastalShip" && (room_name.contains("文曜") || room_name.contains("三水")) {
                if magnesium < 4 {
                    narrative.push_str("\n【盤纏不足】於三水口渡埠登船巡航需 4 鎂船資，隨身盤纏不足，舟子溫言請你備妥船資再來登船。");
                    push_refresh(conn, pid);
                    return;
                }
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, &room_name, -4);
                let _ = db::add_to_inventory(pid, "item_coastal_sea_chart", 1);
                if let Some(arc) = crate::store::get_store() {
                    let _ = arc.write().update_entity(pid, |e| {
                        e.qi = (e.qi + 14).min(100);
                    });
                }
                crate::pg::living_world::record_breath_activity(
                    pid,
                    crate::pg::living_world::BreathHabitKind::TideContinuous,
                    3,
                    now,
                );
                let weather_desc = match weather {
                    crate::game::ZoneWeatherPhase::RainMist => "煙雨空濛、江海翻湧",
                    crate::game::ZoneWeatherPhase::ClearBreeze => "微風拂面、碧波蕩漾",
                    crate::game::ZoneWeatherPhase::DryWarm => "天青氣暖、江流平緩",
                    crate::game::ZoneWeatherPhase::ColdFrost => "寒霜凜冽、江霧低垂",
                };
                let tide_desc = match tide {
                    crate::game::ZoneTidePhase::FloodTide => "滿潮湧入、江水充沛",
                    crate::game::ZoneTidePhase::EbbTide => "退潮奔流、順流疾行",
                    crate::game::ZoneTidePhase::SlackTide => "平潮定波、水勢平穩",
                };
                narrative.push_str(&format!(
                    "\n【江海客船・黑潮巡航】你在三水口渡埠交付 4 鎂船資登上客船，隨江水巡航出海。舷外天候正逢「{}」，江海潮位正值「{}」。你手持航海木尺與羅盤，參照黑潮水流細細校核航道水深，收妥一份「黑潮順時針海圖摺頁」後乘順水穩步回泊渡埠，海潮綿息隨之凝練。",
                    weather_desc, tide_desc
                ));
                let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "wenyao_coastal_cruise");
                push_refresh(conn, pid);
                return;
            }
            if magnesium < 8 {
                narrative.push_str("\n【登船提示】搭乘順時針環島海運商船需 8 鎂船資，隨身盤纏不足，水手溫言請你備妥船資再來登船啟程。");
                push_refresh(conn, pid);
                return;
            }
            if action == "BoardCoastalShip" {
                let _ = db::add_to_inventory(pid, "item_coastal_sea_chart", 1);
                crate::pg::living_world::record_breath_activity(
                    pid,
                    crate::pg::living_world::BreathHabitKind::TideContinuous,
                    3,
                    now,
                );
                if let Some(arc) = crate::store::get_store() {
                    let _ = arc.write().update_entity(pid, |e| {
                        e.qi = (e.qi + 14).min(100);
                    });
                }
            }

            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, &room_name, -8);
            let _ = db::set_entity_grid(pid, next_x, next_y);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().move_entity(pid, &format!("grid:{next_x}:{next_y}"));
            }

            let is_stormy = weather == crate::game::ZoneWeatherPhase::RainMist
                || weather == crate::game::ZoneWeatherPhase::ColdFrost
                || crate::db::get_region_climate("bai_ta_peninsula")
                    .ok()
                    .flatten()
                    .map(|c| matches!(c.event_type.as_str(), "rain" | "plum_rain" | "storm" | "heavy_rain" | "snow"))
                    .unwrap_or(false);

            let has_insurance = db::inventory_has_item(pid, "wt_cat_0792");

            let insurance_note = if is_stormy && has_insurance {
                let _ = db::remove_from_inventory(pid, "wt_cat_0792", 1);
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, "風舶角", 12);
                "\n【海事水險理賠】航程中突遇狂浪顛簸，部分隨身貨裝受潮海損；所幸你備有「築港頭分級海事保單」，抵埠時掌事核銷單據，由風舶角公庫如數給付 12 鎂海事水險理賠撫卹！"
            } else if is_stormy {
                "\n【風浪顛簸】途中遭遇驟雨海霧，風浪顛簸，所幸掌舵老水手經驗老道，商船化險為夷安抵新港。"
            } else {
                "\n【順流航行】商船揚帆順流破浪，一路碧波萬頃，海風徐徐，十分順遂平穩。"
            };

            narrative.push_str(&format!(
                "\n【海運航行抵達】你支付 8 鎂船資登上商船，隨順時針黑潮與暖流航向【{}】（{}）。\n商船穩穩靠泊碼頭棧橋，你順著跳板踏上海港石階。{}",
                next_name, next_desc, insurance_note
            ));

            let _ = event::append(
                now,
                pid,
                types::SCENE_OBJECT_ACTION,
                &format!("clockwise_ship|{}|{},{}", next_name, next_x, next_y),
            );
            super::super::handler::send_place_view(conn, pid);
            push_refresh(conn, pid);
            return;
        }
    }

    // 寒水玉保鮮石櫃 (wt_home_jade_cabinet)
    if (obj.id == "wt_home_jade_cabinet"
        || obj.name.contains("冰鑑櫃")
        || obj.name.contains("寒水玉保鮮石櫃")
        || obj.name.contains("寒水玉雙層保鮮石櫃"))
        && (action == "Look" || action == "Open" || action == "StoreStash" || action == "RetrieveStash" || action == "Use" || action == "Operate")
    {
        if action == "StoreStash" {
            let ent_opt = db::get_entity(pid).ok().flatten();
            let storable = ent_opt.as_ref().and_then(|ch| {
                let inv = db::get_inventory(&ch.inventory, ch.vit);
                inv.items.into_iter().find(|it| {
                    it.qty >= 1
                        && it.item_id != "wt_red_seal_property_deed"
                        && it.item_id != "wt_cat_0793"
                        && it.item_id != "wt_brass_pin_tumbler_key"
                        && it.item_id != "key_brass_home_lock"
                        && !it.item_id.contains("deed")
                        && !it.item_id.contains("key")
                })
            });

            if let Some(item) = storable {
                let item_id = item.item_id;
                let item_name = if !item.name.is_empty() { item.name } else { item_id.clone() };
                let _ = db::remove_from_inventory(pid, &item_id, 1);
                match crate::npc::causal_core::store_item_in_home_stash(pid, &item_id, &item_name) {
                    Ok(total_stored) => {
                        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "home_stash_store");
                        narrative.push_str(&format!(
                            "\n【存入石櫃】你將隨身攜帶的一份「{}」妥善安放於寒水玉雙層保鮮石櫃的冰涼玉板上。天然冷冽玉氣瞬間凝結鎖住鮮味，目前石櫃內共封存保管 {} 件物資。",
                            item_name, total_stored
                        ));
                    }
                    Err(_) => {
                        let _ = db::add_to_inventory(pid, &item_id, 1);
                        narrative.push_str("\n【石櫃已滿】石櫃層架已擺滿物資（上限 24 件），無法再放入更多物資。");
                    }
                }
            } else {
                narrative.push_str("\n【行囊空空】隨身行囊中沒有可供封存的物資（貴重屋契與鑰匙不予存入）。");
            }
        } else if action == "RetrieveStash" {
            match crate::npc::causal_core::retrieve_item_from_home_stash(pid) {
                Some((item_id, item_name, rem)) => {
                    let _ = db::add_to_inventory(pid, &item_id, 1);
                    let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "home_stash_retrieve");
                    narrative.push_str(&format!(
                        "\n【取出物資】你輕輕拉開寒水玉石櫃的冷玉匣，取出保鮮完好的「{}」妥善收入隨身行囊，石櫃內尚存 {} 件物資。",
                        item_name, rem
                    ));
                }
                None => {
                    narrative.push_str("\n【石櫃空無一物】寒水玉石櫃的玉石架上空空如也，散發著天然清冷的玉石幽香，目前尚未封存任何物資。");
                }
            }
        } else {
            // Look / Open / Use / Operate
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 6).min(100);
                    e.qi = (e.qi + 6).min(100);
                });
            }
            let _ = event::append(now, pid, types::PLAYER_BODY, "home_jade_cabinet_refreshed");
            let items = crate::npc::causal_core::list_player_home_stash(pid);
            if items.is_empty() {
                narrative.push_str("\n【寒水玉雙層保鮮石櫃】北山寒水玉薄板整潔清爽，縫隙間透出天然冷冽的幽涼清氣，層架平整空曠，隨時可供封存保鮮鮮食、獵獲與各類物資。\n沁涼玉氣拂面，你頓覺神清氣爽，氣血與內息漸趨調和。");
            } else {
                let item_names: Vec<String> = items.iter().map(|(_id, name)| format!("「{}」", name)).collect();
                narrative.push_str(&format!(
                    "\n【寒水玉雙層保鮮石櫃・封存清冊】櫃內玉石架上冷氣森森，目前共保鮮儲放 {} 件物資：\n{}\n清涼玉氣沁人心脾，周身舒爽通透。",
                    items.len(),
                    item_names.join("、")
                ));
            }
        }
        push_refresh(conn, pid);
        return;
    }

    // 松炭溫風烘衣櫥
    if (obj.id == "wt_home_charcoal_wardrobe"
        || obj.name.contains("松炭溫風烘衣櫥"))
        && (action == "Use" || action == "Operate" || action == "WarmHands")
    {
        crate::pg::living_world::clear_entity_afflictions(pid, true, now);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
                e.dex = (e.dex + 5).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|{}", obj.id, action));
        narrative.push_str("\n【松炭烘暖・驅寒除濕】你拉開松香木大立櫥，底部無煙松炭與導能溫石散發著溫純乾燥的暖風。你將衣物與隨身裝備置入烘暖，周身潮氣與荒野寒熱沉痾盡數拔除消散，筋骨輕靈舒泰、精神充實！");
        push_refresh(conn, pid);
        return;
    }

    // 當季田壟作物
    if obj.id == "wt_field_crop_plot"
        || obj.name.contains("田壟作物")
        || obj.name.contains("當季田壟")
    {
        let ent = db::get_entity(pid).ok().flatten();
        let (gx, gy) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0));
        let cur_terrain = crate::server::grid_manager::get_runtime_grid_cell(crate::grid::SquareCoord::new(gx, gy))
            .map(|c| c.terrain);

        let is_empty_furrow = cur_terrain == Some(crate::grid::Terrain::EmptyFurrow);
        let field_kind = match cur_terrain {
            Some(crate::grid::Terrain::DryField) => "dry_field",
            Some(crate::grid::Terrain::HerbGarden) => "herb_garden",
            _ => "water_field",
        };

        if is_empty_furrow {
            if action == "Use" || action == "Operate" || action == "Sow" {
                crate::server::grid_manager::with_square_grid_mut(|grid| {
                    if let Some(cell) = grid.get_mut(crate::grid::SquareCoord::new(gx, gy)) {
                        cell.terrain = crate::grid::Terrain::FarmField;
                        cell.name = "水田苗畦".to_string();
                    }
                });
                let plot = crate::npc::causal_core::register_crop_plot_sown(gx, gy, "water_field", now);
                let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|sow", obj.id));
                *narrative = format!(
                    "【整壟播種】你沿著翻鬆的壟溝引水浸潤，均勻撒下當季種籽（{}）。無需反覆枯燥勞作，泥土與清泉已自發孕育生機，靜待歲時抽穗熟成。",
                    plot.crop_name
                );
                push_refresh(conn, pid);
                super::super::handler::send_place_view(conn, pid);
                return;
            } else if action == "Harvest" || action == "Gather" {
                *narrative = "【休耕空壟】此處田壟尚未播種作物，請先點選「使用／播種」撒下當季種籽。".to_string();
                push_refresh(conn, pid);
                return;
            } else if action == "Look" {
                *narrative = "【休耕空壟】這是一壟已起好壟溝但尚未播種的鬆軟田壟，土質肥沃深厚、引水便利，隨時可進行播種。".to_string();
                push_refresh(conn, pid);
                return;
            }
        } else {
            if action == "Look" {
                let plot = crate::npc::causal_core::inspect_or_advance_crop_plot(gx, gy, field_kind, now);
                let stage_desc = match plot.stage {
                    crate::npc::causal_core::CropGrowthStage::Seedling => "幼苗青翠，壟間嫩綠新芽在晨露中舒展，根系深紮土壤汲取養分。",
                    crate::npc::causal_core::CropGrowthStage::Growing => "抽穗飽滿，禾穗迎風搖曳，沉甸甸的籽實已初具成色，靜待最後成熟。",
                    crate::npc::causal_core::CropGrowthStage::Mature => "熟成待收！金黃飽滿的沉重穗實低垂在壟上，稻香撲鼻，正適及時採收！",
                };
                *narrative = format!(
                    "【田壟生息・{}】當前栽種作物：{}。\n生長氣候：{}\n當前階段：{}",
                    match field_kind {
                        "dry_field" => "旱田壟畦",
                        "herb_garden" => "藥圃苗畦",
                        _ => "水田壟畦",
                    },
                    plot.crop_name,
                    if plot.water_nourished { "清泉潤養充足" } else { "日照溫煦" },
                    stage_desc
                );
                let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|{}", obj.id, action));
                push_refresh(conn, pid);
                return;
            }

            if action == "Harvest" || action == "Gather" || action == "Use" || action == "Operate" {
                match crate::npc::causal_core::harvest_crop_plot_once(gx, gy, field_kind, now) {
                    Ok((item_id, item_name, qty, harvest_prose)) => {
                        let _ = db::add_to_inventory(pid, &item_id, qty.max(1));
                        crate::server::grid_manager::with_square_grid_mut(|grid| {
                            if let Some(cell) = grid.get_mut(crate::grid::SquareCoord::new(gx, gy)) {
                                cell.terrain = crate::grid::Terrain::EmptyFurrow;
                                cell.name = "休耕空壟".to_string();
                            }
                        });
                        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|harvest|{}", obj.id, item_id));
                        *narrative = format!(
                            "【順時收穫】{}\n你將採收得來的 {} 份「{}」整齊收納入隨身行囊，壟溝復歸清爽整潔的休耕狀態。",
                            harvest_prose, qty, item_name
                        );
                        push_refresh(conn, pid);
                        super::super::handler::send_place_view(conn, pid);
                        return;
                    }
                    Err(wait_prose) => {
                        *narrative = format!("【尚未熟成】{}", wait_prose);
                        push_refresh(conn, pid);
                        return;
                    }
                }
            }
        }
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

    // 《半島旬報》閱報欄 (wt_street_news_board / wt_cat_0786)
    if (obj.id == "wt_street_news_board" || obj.id == "wt_cat_0786" || obj.name.contains("閱報") || obj.name.contains("旬報"))
        && (action == "Look" || action == "Read" || action == "Use")
    {
        let room_name = resolve_room_name(player_room);
        let gazette = crate::npc::causal_core::compile_live_peninsula_gazette(&room_name, now);
        let gazette_clean = gazette.lines().map(|line| {
            if line.contains("現存") && line.contains("目標需求") && line.contains("缺口約") {
                let re_prefix = line.split("：").next().unwrap_or("");
                let price_part = line.split("，坊間公道收購價").nth(1).unwrap_or("");
                if !price_part.is_empty() {
                    format!("{}：坊間存貨緊俏，坊間公道收購價{}", re_prefix, price_part)
                } else {
                    format!("{}：坊間存貨緊俏。", re_prefix)
                }
            } else {
                line.to_string()
            }
        }).collect::<Vec<_>>().join("\n");
        narrative.push('\n');
        narrative.push_str(&gazette_clean);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "peninsula_gazette_read");
        push_refresh(conn, pid);
        return;
    }

    // 防雨竹編信箱 (wt_home_bamboo_mailbox)
    if (obj.id == "wt_home_bamboo_mailbox" || obj.name.contains("竹編信箱"))
        && (action == "Look" || action == "Read" || action == "Open" || action == "Use" || action == "DispatchCourier")
    {
        if action == "Look" || action == "Read" {
            let room_name = resolve_room_name(player_room);
            let gazette = crate::npc::causal_core::compile_live_peninsula_gazette(&room_name, now);
            let gazette_clean = gazette.lines().map(|line| {
                if line.contains("現存") && line.contains("目標需求") && line.contains("缺口約") {
                    // 將現存 {} 份（目標需求 {} 份，缺口約 {}%）改寫為坊間存貨緊俏
                    let re_prefix = line.split("：").next().unwrap_or("");
                    let price_part = line.split("，坊間公道收購價").nth(1).unwrap_or("");
                    if !price_part.is_empty() {
                        format!("{}：坊間存貨緊俏，坊間公道收購價{}", re_prefix, price_part)
                    } else {
                        format!("{}：坊間存貨緊俏。", re_prefix)
                    }
                } else {
                    line.to_string()
                }
            }).collect::<Vec<_>>().join("\n");
            *narrative = format!("你掀開竹編信箱的防水翻蓋，取出最新一期《半島旬報》：\n{}", gazette_clean);
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mailbox_read_gazette");
        } else if action == "Open" || action == "Use" {
            let player_name = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| if !e.display_title.trim().is_empty() { e.display_title } else { e.id })
                .unwrap_or_else(|| pid.to_string());
            let (letter, gift_opt) = crate::npc::causal_core::claim_acquaintance_mailbox_letter(pid, &player_name, now);
            *narrative = format!("你撥開竹箱銅扣探手檢視，取出封泥完好的信件：\n{}", letter);
            if let Some((gift_id, gift_name)) = gift_opt {
                let _ = db::add_to_inventory(pid, &gift_id, 1);
                narrative.push_str(&format!("\n隨信附有一份心意回禮：「{}」，已妥善收整入行囊。", gift_name));
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mailbox_opened");
        } else if action == "DispatchCourier" {
            let magnesium = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.magnesium)
                .unwrap_or(0);
            if magnesium < 3 {
                narrative.push_str("\n【資財不足】託付疾行草獸信使飛騎遞信需付 3 鎂郵資，隨身資財不足，信使禮貌婉拒。");
            } else {
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
                let room_name = resolve_room_name(player_room);
                let _ = crate::game::object_ecology::record_settlement_market_trade(&room_name, "craft", 1, 3, pid);
                let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mailbox_courier_dispatched");
                narrative.push_str("\n【快騎託運・飛騎出發】你將家書與物資封妥投入信箱快信格，支付 3 鎂郵資。信使吹響木哨，健步如飛的草獸揚起一陣輕塵，循著驛道朝鄰近重鎮疾馳而去！");
            }
        }
        push_refresh(conn, pid);
        return;
    }

    // 疾行草獸信使櫃檯 (wt_courier_post_desk)
    if (obj.id == "wt_courier_post_desk" || obj.name.contains("信使櫃檯"))
        && (action == "Look" || action == "Read" || action == "DispatchCourier" || action == "Use")
    {
        if action == "Look" || action == "Read" {
            let room_name = resolve_room_name(player_room);
            let gazette = crate::npc::causal_core::compile_live_peninsula_gazette(&room_name, now);
            let gazette_clean = gazette.lines().map(|line| {
                if line.contains("現存") && line.contains("目標需求") && line.contains("缺口約") {
                    let re_prefix = line.split("：").next().unwrap_or("");
                    let price_part = line.split("，坊間公道收購價").nth(1).unwrap_or("");
                    if !price_part.is_empty() {
                        format!("{}：坊間存貨緊俏，坊間公道收購價{}", re_prefix, price_part)
                    } else {
                        format!("{}：坊間存貨緊俏。", re_prefix)
                    }
                } else {
                    line.to_string()
                }
            }).collect::<Vec<_>>().join("\n");
            narrative.push_str(&format!(
                "\n【疾行草獸信使總站櫃檯】案頭整齊陳列著往返四鎮的驛道封冊、火漆印章與加急竹筒，案旁栓著數頭蓄勢待發的健碩草獸。壁上木牌張貼著半島各驛站聯運路況：\n{}\n草獸信使櫃檯提供跨聚落加急包裹託運服務，每件資費 4 鎂。",
                gazette_clean
            ));
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "courier_post_read");
        } else if action == "DispatchCourier" || action == "Use" {
            let magnesium = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.magnesium)
                .unwrap_or(0);
            if magnesium < 4 {
                narrative.push_str("\n【資財不足】辦理跨聚落加急件託運需付 4 鎂資費，隨身資財不足，櫃檯掌事客氣提醒。");
            } else {
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -4);
                let room_name = resolve_room_name(player_room);
                let _ = crate::game::object_ecology::record_settlement_market_trade(&room_name, "craft", 1, 4, pid);
                let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "courier_post_dispatched");
                narrative.push_str("\n【跨聚落急件託運】你向掌事交付 4 鎂託運規費，並在封皮蓋上加急硃印。當值信使將包裹紮緊繫於疾行草獸鞍袋，健碩草獸揚蹄嘶鳴，破風奔向遠方聚落！");
            }
        }
        push_refresh(conn, pid);
        return;
    }

    // 老青竹黃銅輪捲線釣竿 (wt_coastal_angling_spot / wt_cat_1006)
    if (obj.id == "wt_coastal_angling_spot"
        || obj.id == "wt_cat_1006"
        || obj.name.contains("老青竹黃銅輪捲線釣竿")
        || obj.name.contains("釣竿")
        || obj.name.contains("垂釣"))
        && (action == "AngleFish" || action == "Use" || action == "Harvest" || action == "Operate" || action == "Look")
    {
        let room_name = resolve_room_name(player_room);
        let z_idx = crate::npc::causal_core::settlement_to_zone_idx(&room_name);
        let weather = crate::npc::causal_core::zone_weather_phase(z_idx, now);
        let tide = crate::npc::causal_core::zone_tide_phase(z_idx, now);

        let weather_prose = match weather {
            crate::game::ZoneWeatherPhase::RainMist => "煙雨迷濛，水面泛起粼粼漣漪，魚群紛紛浮頭索餌",
            crate::game::ZoneWeatherPhase::ClearBreeze => "微風拂面，水波不興，水底浮標靈敏輕晃",
            crate::game::ZoneWeatherPhase::DryWarm => "天青水暖，陽光直透清澈水底，游鱗穿梭石隙之間",
            crate::game::ZoneWeatherPhase::ColdFrost => "寒霜初凝，水汽蒸騰，沉底大物沿著溫流緩緩巡游",
        };
        let tide_prose = match tide {
            crate::game::ZoneTidePhase::FloodTide => "正值滿潮大水，海流頂托河口，水產豐腴",
            crate::game::ZoneTidePhase::EbbTide => "正值退潮水落，灘頭礁縫露出，獵食活潑",
            crate::game::ZoneTidePhase::SlackTide => "正值平潮之際，水面澄澈如鏡，靜候大物咬鉤",
        };

        if action == "Look" {
            let hr = crate::store::current_game_hour();
            let water_surface = if !(6..19).contains(&hr) {
                "夜色映水，微風拂過河面泛起細碎波光，水底游鱗偶爾掠過暗流"
            } else {
                "天光映照水面，微風拂過泛起粼粼漣漪，可見游鱗穿梭石隙之間"
            };
            narrative.push_str(&format!(
                "\n{}；{}。水面下波紋微動，浮標隨暗流輕輕起伏。",
                water_surface, tide_prose
            ));
        } else {
            let is_coast = room_name.contains("港")
                || room_name.contains("海")
                || room_name.contains("澳")
                || room_name.contains("鼻")
                || room_name.contains("尾")
                || room_name.contains("風舶")
                || room_name.contains("黑石灣")
                || room_name.contains("礁");
            let is_high_or_spring_tide = tide == crate::game::ZoneTidePhase::FloodTide;
            let (catch_id, catch_name) = if is_coast && is_high_or_spring_tide {
                ("wt_cat_0225", "礁岩大龍蝦")
            } else if is_coast {
                ("shanhai_fresh_fish", "深海鮮魚")
            } else {
                ("mat_fresh_shrimp", "水尾寮鮮沼蝦")
            };

            let _ = db::add_to_inventory(pid, catch_id, 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 5).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "coastal_angling_catch");
            narrative.push_str(&format!(
                "\n【甩竿垂釣・豐收起竿】你揚起老青竹釣竿，黃銅滾輪發出清脆的收線鳴響，釣線劃破水面在空中劃出一道銀弧！\n{}，{}。\n伴隨一陣沉甸甸的水花拍打聲，你利落抄網，將一尾活蹦亂跳的「{}」收獲入魚簍，呼吸吐納愈發沉靜悠長！",
                weather_prose, tide_prose, catch_name
            ));
        }
        push_refresh(conn, pid);
        return;
    }

    // 紅土光學黃銅單筒望遠鏡 (wt_lookout_telescope / wt_cat_0733)
    if (obj.id == "wt_lookout_telescope"
        || obj.id == "wt_cat_0733"
        || obj.name.contains("望遠鏡")
        || obj.name.contains("單筒望遠鏡"))
        && (action == "Operate" || action == "Use" || action == "Look")
    {
        let room_name = resolve_room_name(player_room);
        let z_idx = crate::npc::causal_core::settlement_to_zone_idx(&room_name);
        let climate_evt = crate::db::get_region_climate("bai_ta_peninsula")
            .ok()
            .flatten()
            .map(|c| c.event_type)
            .unwrap_or_else(|| "clear".to_string());
        let tide = crate::npc::causal_core::zone_tide_phase(z_idx, now);
        let hr = crate::store::current_game_hour();
        let weather_label = match climate_evt.as_str() {
            "rain" | "plum_rain" | "storm" => "煙雨空濛、水氣瀰漫（遠景稍暗）",
            "snow" | "fog" => "寒霧微凝、高嶺薄靄（遠岫隱約可見）",
            "wind" => "長風颯爽、雲影疾馳（視野開闊明朗）",
            _ => {
                if !(6..19).contains(&hr) {
                    "天朗星垂、夜色澄明（兩岸燈火與遠影歷歷可見）"
                } else {
                    "天朗氣清、惠風和暢（視野開闊極佳）"
                }
            }
        };
        let tide_label = match tide {
            crate::game::ZoneTidePhase::FloodTide => "滿潮湧漲",
            crate::game::ZoneTidePhase::EbbTide => "退潮落水",
            crate::game::ZoneTidePhase::SlackTide => "平潮停滯",
        };

        let ent = db::get_entity(pid).ok().flatten();
        let (px, py) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0));

        let live_coords: std::collections::HashMap<String, ((i32, i32), String)> = {
            let states = crate::store::LIVE_NPC_GRID_STATES.read();
            states
                .iter()
                .map(|(k, v)| (k.clone(), (v.current, v.current_activity())))
                .collect()
        };

        let mut nearby_npcs: Vec<String> = Vec::new();
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            for (id, e) in &s.entities {
                if id != pid && (e.kind == "npc" || e.kind == "Npc") && e.vit > 0 {
                    let coord = live_coords
                        .get(id)
                        .map(|(c, _)| *c)
                        .or_else(|| e.grid_x.zip(e.grid_y));
                    if let Some((ex, ey)) = coord {
                        let d = (ex - px).abs().max((ey - py).abs());
                        if d <= 3 {
                            let name = if crate::store::player_knows_npc(pid, id) && !e.display_title.trim().is_empty() {
                                crate::store::resolve_npc_person_name(id, &e.display_title, None)
                            } else {
                                "行旅身影".to_string()
                            };
                            let dir_str = match ((ex - px).cmp(&0), (ey - py).cmp(&0)) {
                                (std::cmp::Ordering::Equal, std::cmp::Ordering::Greater) => "北面",
                                 (std::cmp::Ordering::Equal, std::cmp::Ordering::Less) => "南面",
                                 (std::cmp::Ordering::Greater, std::cmp::Ordering::Equal) => "東面",
                                 (std::cmp::Ordering::Less, std::cmp::Ordering::Equal) => "西面",
                                 (std::cmp::Ordering::Greater, std::cmp::Ordering::Greater) => "東北面",
                                 (std::cmp::Ordering::Greater, std::cmp::Ordering::Less) => "東南面",
                                 (std::cmp::Ordering::Less, std::cmp::Ordering::Greater) => "西北面",
                                 (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => "西南面",
                                 (std::cmp::Ordering::Equal, std::cmp::Ordering::Equal) => "近處",
                            };
                            let act_suffix = if let Some((_, act)) = live_coords.get(id) {
                                let raw_act = act.trim();
                                let visual_distant_act = if raw_act.contains("動身前往")
                                    || raw_act.contains("步行前往")
                                    || raw_act.contains("沿街步行")
                                    || raw_act.contains("趕往")
                                    || raw_act.contains("前往【")
                                    || raw_act.contains("回返【")
                                {
                                    "沿著街巷步道緩步穿行"
                                } else if raw_act.contains("歇")
                                    || raw_act.contains("喘")
                                    || raw_act.contains("睡")
                                    || raw_act.contains("寢")
                                {
                                    "在廊簷案頭旁駐足歇息"
                                } else if raw_act.contains("飲")
                                    || raw_act.contains("喝")
                                    || raw_act.contains("茶")
                                    || raw_act.contains("湯")
                                    || raw_act.contains("吃")
                                    || raw_act.contains("果腹")
                                {
                                    "在街邊攤肆與茶座間歇腳用茶"
                                } else {
                                    "在街坊鋪面間往來操持"
                                };
                                format!("，正{}", visual_distant_act)
                            } else {
                                String::new()
                            };
                            nearby_npcs.push(format!("「{}」（位於{} {} 里處{}）", name, dir_str, d, act_suffix));
                            if nearby_npcs.len() >= 3 {
                                break;
                            }
                        }
                    }
                }
            }
        }

        let time_tick = now / 180;
        let habitat_slots = crate::game::live_zone_shanhai_slots(z_idx, time_tick);
        let mut beast_reports: Vec<String> = Vec::new();
        for sl in habitat_slots.iter().take(3) {
            let dx = sl.x - px;
            let dy = sl.y - py;
            let dist = dx.abs().max(dy.abs());
            let dir_str = match (dx.cmp(&0), dy.cmp(&0)) {
                (std::cmp::Ordering::Equal, std::cmp::Ordering::Greater) => "北面",
                (std::cmp::Ordering::Equal, std::cmp::Ordering::Less) => "南面",
                (std::cmp::Ordering::Greater, std::cmp::Ordering::Equal) => "東面",
                (std::cmp::Ordering::Less, std::cmp::Ordering::Equal) => "西面",
                (std::cmp::Ordering::Greater, std::cmp::Ordering::Greater) => "東北面",
                (std::cmp::Ordering::Greater, std::cmp::Ordering::Less) => "東南面",
                (std::cmp::Ordering::Less, std::cmp::Ordering::Greater) => "西北面",
                (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => "西南面",
                (std::cmp::Ordering::Equal, std::cmp::Ordering::Equal) => "近處",
            };
            let creature_name = crate::game::ensure_ambient_npc_at_grid(sl.x, sl.y, sl.tier_tag)
                .map(|amb| amb.display_title)
                .unwrap_or_else(|| sl.tier_label.to_string());
            let clean_prefix = sl
                .act_prefix
                .trim()
                .trim_end_matches('正')
                .trim_end_matches('，')
                .trim_end_matches('、');
            let act = if !clean_prefix.is_empty() {
                format!("，{}出沒。", clean_prefix)
            } else {
                "一帶活動。".to_string()
            };
            beast_reports.push(format!(
                "【{}・{}】位於{}約 {} 里地勢「{}」{}",
                sl.tier_label, creature_name, dir_str, dist, sl.sub_geo, act
            ));
        }

        let npcs_text = if nearby_npcs.is_empty() {
            "周遭三里之內山徑空曠幽靜，未見其他行旅駐足。".to_string()
        } else {
            nearby_npcs.join("、")
        };
        let beasts_text = if beast_reports.is_empty() {
            "望遠鏡視野所及之處，生靈隱匿於林莽深谷之中，暫無醒目大型獸群現身。".to_string()
        } else {
            beast_reports.join("\n")
        };

        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "lookout_telescope_scouted");
        narrative.push_str(&format!(
            "\n【單筒望遠鏡・四方瞭望】當前物候能見度：{}（潮位：{}）\n近處行旅觀察（周遭三里）：{}\n遠方生靈蹤跡（白塔山海動向）：\n{}\n山川草木歷歷在目，極目遠眺令胸懷大暢！",
            weather_label, tide_label, npcs_text, beasts_text
        ));
        push_refresh(conn, pid);
        return;
    }

    // 卡式導能便攜爐 (wt_field_camp_stove / wt_cat_1010)
    if (obj.id == "wt_field_camp_stove"
        || obj.id == "wt_cat_1010"
        || obj.name.contains("卡式導能便攜爐")
        || obj.name.contains("導能便攜爐")
        || obj.name.contains("便攜爐"))
        && (action == "Cook" || action == "Use" || action == "Operate" || action == "Look")
    {
        if action == "Look" {
            narrative.push_str("\n爐內導能晶匣充盈，旋鈕與防風環完備，隨時可起火溫食。");
        } else {
            let raw_candidates = [
                ("wt_cat_0225", "礁岩大龍蝦"),
                ("shanhai_fresh_fish", "深海鮮魚"),
                ("mat_fresh_shrimp", "水尾寮鮮沼蝦"),
                ("shanhai_beast_meat", "山海鮮獸肉"),
                ("mat_grain_sack", "農家精選穀糧袋"),
                ("mat_pasture_seed_sack", "優質牧草種籽袋"),
                ("harvest_herb", "野地草藥"),
            ];

            let ent = db::get_entity(pid).ok().flatten();
            let found_ingredient = ent.as_ref().and_then(|ch| {
                let inv = db::get_inventory(&ch.inventory, ch.vit);
                for (id, default_name) in raw_candidates {
                    if let Some(item) = inv.items.iter().find(|it| it.item_id == id && it.qty >= 1) {
                        let name = if !item.name.is_empty() { item.name.clone() } else { default_name.to_string() };
                        return Some((id.to_string(), name));
                    }
                }
                None
            });

            if let Some((ing_id, ing_name)) = found_ingredient {
                let _ = db::remove_from_inventory(pid, &ing_id, 1);
                crate::pg::living_world::clear_entity_afflictions(pid, true, now);
                if let Some(arc) = crate::store::get_store() {
                    let _ = arc.write().update_entity(pid, |e| {
                        e.vit = (e.vit + 20).min(100);
                        e.qi = (e.qi + 20).min(100);
                    });
                }
                let _ = event::append(now, pid, types::PLAYER_BODY, "camp_stove_cooked_meal");
                narrative.push_str(&format!(
                    "\n【野炊烹煮・熱羹出鍋】你旋開卡式導能便攜爐的銅旋鈕，微晶導能爐盤噴吐出純青勻淨的火焰。你將行囊中的「{}」下鍋，搭配山澗清泉慢火煨煮。不多時濃郁熱氣升騰，一鍋鮮香撲鼻的野地熱羹熱氣騰騰！\n你趁熱細細啜飲享用，通體舒暢、寒濕盡拔，周身氣血與內息復歸飽滿。",
                    ing_name
                ));
            } else {
                crate::pg::living_world::clear_entity_afflictions(pid, true, now);
                if let Some(arc) = crate::store::get_store() {
                    let _ = arc.write().update_entity(pid, |e| {
                        e.vit = (e.vit + 10).min(100);
                        e.qi = (e.qi + 12).min(100);
                    });
                }
                let _ = event::append(now, pid, types::PLAYER_BODY, "camp_stove_boiled_water");
                narrative.push_str("\n【溫水暖胃・行氣調息】你旋開導能便攜爐點燃微晶青焰，架上小銅壺汲取清泉煮沸。你捧著熱水烘暖雙掌並細細啜飲，融融暖意順著咽喉化入經絡，驅散了周遭風露寒涼，身心沉痾頓時舒解。");
            }
        }
        push_refresh(conn, pid);
        return;
    }

    // ==========================================
    // v0.38.52 Three-Pillar Living-World Expansion
    // ==========================================

    // Pillar 1: Outer 13 Settlements' Signature Workshops & Pit #3 Rail Trolley
    // 1. 老堰頭水磨 (wt_mill_*)
    if (obj.id.starts_with("wt_mill_")
        || obj.name.contains("水輪玄武磨")
        || obj.name.contains("篩粉斗")
        || obj.name.contains("儲糧斗"))
        && (action == "MillGrain" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "wt_cat_0388", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 5);
        crate::npc::causal_core::record_settlement_market_production("水尾寮", "wt_cat_0388", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_mill_grain");
        narrative.push_str("【老堰頭水磨碾米】你合力推開水磨料門，巨輪轉動帶動兩合巨大玄武岩磨盤隆隆運轉，顆粒飽滿的稻穀化為細密熱漿。工坊按工計酬結算勞務，由聚落公庫支領 5 鎂酬勞，並盛了一碗現磨的「老堰頭現磨溫米漿」交予你作為手作所得。市集糧棧的新米漿儲備隨之充盈！");
        push_refresh(conn, pid);
        return;
    }

    // 2. 霜林口鋸木排鋸 (wt_sawmill_*)
    if (obj.id.starts_with("wt_sawmill_")
        || obj.name.contains("水動排鋸架")
        || obj.name.contains("原木檢寸台"))
        && (action == "RefinePlank" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "mat_plank_board", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 6);
        crate::npc::causal_core::record_settlement_market_production("霜林口", "mat_plank_board", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_sawmill_plank");
        narrative.push_str("【霜林口裁鋸木板】你推動水動排鋸咬合百年雲杉原木，精鋼鋸齒上下飛速下切，鋸路筆直順暢，木屑紛飛間剖出規整厚板。工坊由霜林口公庫支領 6 鎂工酬，並將裁切精良的「樟松木板料」交付於你。工坊向周遭商行補充了建材木料！");
        push_refresh(conn, pid);
        return;
    }

    // 3. 半嶺切石金剛砂拉鋸 (wt_stonecut_*)
    if (obj.id.starts_with("wt_stonecut_")
        || obj.name.contains("金剛砂拉鋸")
        || obj.name.contains("寒水玉料架")
        || obj.name.contains("拋光琢石盆"))
        && (action == "CutJade" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "wt_cat_0002", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 6);
        crate::npc::causal_core::record_settlement_market_production("半嶺鎮", "wt_cat_0002", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_cut_jade");
        narrative.push_str("【半嶺切磨寒玉】你拉動金剛砂合金絲拉鋸緊貼原石，引冷泉注淋鋸槽，細細剖開北山寒水玉毛料，石皮下透出溫潤玉光，經琢石盆細心修整稜邊。工坊由半嶺鎮公庫支領 6 鎂酬勞，並贈予一枚質地縝密的「寒水玉原石」。");
        push_refresh(conn, pid);
        return;
    }

    // 4. 牛埔壓酪架與牧耕鐵犁 (wt_cheese_* / wt_wool_* / wt_farm_tool_iron_plow)
    if (obj.id.starts_with("wt_cheese_")
        || obj.id == "wt_wool_spinning_wheel"
        || obj.id == "wt_farm_tool_iron_plow"
        || obj.name.contains("螺旋壓酪架")
        || obj.name.contains("手搖木紡車")
        || obj.name.contains("乳酪熟成架")
        || obj.name.contains("雙齒精鐵犁"))
        && (action == "PressCheese" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "wt_cat_0300", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 5);
        crate::npc::causal_core::record_settlement_market_production("牛埔", "wt_cat_0300", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_press_cheese");
        narrative.push_str("【牛埔壓製乳酪】你握住螺旋桿加力旋轉下壓，粗濾布中的凝乳塊瀝出澄澈微酸金黃乳清，壓製成緊實濃醇的硬乳酪磚。牧場按勞作由牛埔公庫撥付 5 鎂勞酬，並取出熟成成色極佳的一輪「窖藏熟成硬乳酪」贈予你。");
        push_refresh(conn, pid);
        return;
    }

    // 5. 三號坑口選礦槽與雙軸滑車 (wt_mine_* / wt_trolley_*)
    if (obj.id.starts_with("wt_mine_")
        || obj.id.starts_with("wt_trolley_")
        || obj.name.contains("木製洗礦槽")
        || obj.name.contains("分級重鐵篩")
        || obj.name.contains("雙軸重台車")
        || obj.name.contains("鋼索絞盤"))
        && (action == "HaulMineCart" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "wt_cat_0001", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 7);
        crate::npc::causal_core::record_settlement_market_production("三號坑口", "wt_cat_0001", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_haul_mine_cart");
        narrative.push_str("【三號坑口滑車運礦】你合力推動雙軸重台車滑入坑口鐵軌，伴隨鋼輪咬軌轟鳴與絞盤鋼索轉動，將滿車洗選完成的沉甸精礦運出坑道。礦務由三號坑口公庫支領 7 鎂運礦重資，並分得一塊高品位「赤鐵重礦」。");
        push_refresh(conn, pid);
        return;
    }

    // 6. 風櫃尾慢燻紅陶灶 (wt_smokehouse_*)
    if (obj.id.starts_with("wt_smokehouse_")
        || obj.name.contains("慢燻紅陶灶")
        || obj.name.contains("冷燻海魚架"))
        && (action == "SmokeFish" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "mat_salted_fish", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 6);
        crate::npc::causal_core::record_settlement_market_production("風櫃尾", "mat_salted_fish", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_smoke_fish");
        narrative.push_str("【風櫃尾慢火冷燻】你添入受控乾燥柏木與香草，青煙繚繞慢火透骨，將醃好的海魚燻至金紅油亮、緊實鹹香。燻坊依勞作由風櫃尾公庫支領 6 鎂工錢，並收穫一份風味醇厚的「海鹽醃魚乾」。市集魚貨儲備隨之提升！");
        push_refresh(conn, pid);
        return;
    }

    // 7. 風舶角燃灰窯與海藻搓繩 (wt_lime_* / wt_kelp_* / wt_shipyard_dry_dock)
    if (obj.id.starts_with("wt_lime_")
        || obj.id.starts_with("wt_kelp_")
        || obj.id == "wt_shipyard_dry_dock"
        || obj.name.contains("海貝燃灰窯")
        || obj.name.contains("高架曬藻棚")
        || obj.name.contains("海纜搓繩輪")
        || obj.name.contains("雲杉龍骨架"))
        && (action == "BurnLime" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "mat_hemp_rope_roll", 1);
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 6);
        crate::npc::causal_core::record_settlement_market_production("風舶角", "mat_hemp_rope_roll", 2);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "workshop_burn_lime");
        narrative.push_str("【風舶角入窯煅灰】你鼓風引燃灰窯高溫煅燒海貝，調製防腐熟石灰並協同轉動絞輪搓絞耐磨纜繩，加固船架榫卯。港坊依勞務由風舶角公庫發放 6 鎂工酬，並獲取一捆結實耐磨的「麻繩卷」。船塢物料得以充實！");
        push_refresh(conn, pid);
        return;
    }

    // Pillar 2: Nanshao Miasma Preparation & Shuiweiliao Bamboo Raft Marsh Salvage
    // 1. 備妥避瘴器物 (wt_hunt_* / wt_clinic_herb_cabinet)
    if (obj.id.starts_with("wt_hunt_")
        || obj.id == "wt_clinic_herb_cabinet"
        || obj.name.contains("牛角破甲弩")
        || obj.name.contains("帶齒鋼獸夾")
        || obj.name.contains("百草丸藥櫃"))
        && (action == "PrepareMiasmaGear" || action == "Acquire")
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 4 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -4);
            let _ = db::add_to_inventory(pid, "wt_cat_1055", 1);
            let _ = db::add_to_inventory(pid, "wt_cat_1014", 1);
            crate::pg::living_world::clear_entity_afflictions(pid, true, now);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 10).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "prepare_miasma_gear");
            narrative.push_str("【備妥避瘴器物】你向聚落公庫交付 4 鎂採買避瘴防護物資，配齊了清冽提神的「白茫避瘴清心丸」與指引濃霧方位的「紅磷十丈引路信號筒」。服下一枚清心丸後，周身經絡微涼舒暢，驅散一切瘴癘毒濁，內息充盈穩固。");
        } else {
            narrative.push_str("【資財不足】隨身資財不足，備辦避瘴器物需 4 鎂存入聚落公庫以補物資消耗。");
        }
        push_refresh(conn, pid);
        return;
    }

    // 2. 水尾寮竹筏撐筏入澤 (wt_dock_riverbank_raft)
    if (obj.id == "wt_dock_riverbank_raft" || obj.name.contains("平底竹筏"))
        && (action == "RowSwampRaft" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let has_miasma_pill = if let Ok(Some(ch)) = db::get_entity(pid) {
            let inv = db::get_inventory(&ch.inventory, ch.vit);
            inv.items.iter().any(|it| (it.item_id == "wt_cat_1055" || it.item_id == "wt_cat_1014") && it.qty >= 1)
        } else {
            false
        };
        let _ = db::add_to_inventory(pid, "wt_cat_0058", 1);
        let _ = db::add_to_inventory(pid, "wt_cat_0113", 1);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "row_swamp_raft_salvage");

        if has_miasma_pill {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 8).min(100);
                });
            }
            narrative.push_str("【水尾寮撐筏入澤】你解開桐油麻繩點篙撐筏，輕舟輕巧破開水尾寮蘆葦與黑沼迷霧。你在深澤暗流中打撈起堅硬不腐的「沉水烏木原材」，並採集到新鮮柔韌的「淺灘止血水苔」。懷中「白茫避瘴清心丸」藥氣清冽，逼退周遭陰濕穢霧，呼吸清爽通透。");
        } else {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi.saturating_sub(3)).max(10);
                });
            }
            narrative.push_str("【水尾寮撐筏入澤】你解開桐油麻繩點篙撐筏，深入蘆葦水網與黑沼澤畔，打撈起珍貴的「沉水烏木原材」與「淺灘止血水苔」。然而澤中暗霧濕冷透骨，因未備「白茫避瘴清心丸」護體，胸口微感陰濕滯悶。建議前往哨所或診所【備妥避瘴】。");
        }
        push_refresh(conn, pid);
        return;
    }

    // Pillar 3: Calendar-Coupled Seasonal Festival Banquets & Tea-House Ebony-Shell Chess Fellowship
    // 1. 歲時圍爐 (wt_dining_iron_cauldron / wt_reg_plains_specialty / wt_reg_mining_specialty)
    if (obj.id == "wt_dining_iron_cauldron"
        || obj.id == "wt_reg_plains_specialty"
        || obj.id == "wt_reg_mining_specialty"
        || obj.name.contains("紅鐵燉肉釜")
        || obj.name.contains("陳年米酒甕")
        || obj.name.contains("烤番薯砂鍋"))
        && (action == "SeasonalBanquet" || action == "Taste" || action == "Use")
    {
        narrative.clear();
        let (_, _, _, raw_days) = crate::game::game_time_now(now, conn.cfg.game_time_epoch_unix, conn.cfg.game_time_scale);
        let day_in_year = raw_days.max(0) % 365;
        let season_name = if day_in_year < 91 {
            "春融"
        } else if day_in_year < 182 {
            "夏漲"
        } else if day_in_year < 273 {
            "秋狩"
        } else {
            "冬凝"
        };
        let dish_name = match season_name {
            "春融" | "春霖" => "艾草青糰與鮮筍春捲席",
            "夏漲" | "夏炎" => "寒水玉冰鎮仙草凍與鮮魚羹席",
            "秋狩" => "大圳頭新米團圓宴與桂花新米糕席",
            "冬凝" => "半嶺黑芝麻湯圓與紫銅鴛鴦鍋守歲席",
            _ => "大圳頭新米團圓宴與桂花新米糕席",
        };

        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 20).min(100);
                    e.qi = (e.qi + 15).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "seasonal_banquet_feasted");

            let nearby_resident = {
                let gh = current_game_hour(&conn.cfg);
                let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
                ents.into_iter().find(|e| e.id != pid && e.kind == crate::entity::EntityKind::Npc && e.vit > 0)
            };

            if let Some(npc) = nearby_resident {
                let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                    format!("住民【{}】", npc.display_title.replace('|', "・"))
                } else {
                    "同席街坊".to_string()
                };
                let summary = format!("圍爐共享「{}」，相談甚歡", dish_name);
                crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, &summary, now);
                let _ = db::adjust_favorability(pid, &npc.id, 5);
                narrative.push_str(&format!(
                    "【歲時圍爐・時令節慶】當前時節正逢「{}」。你投入 3 鎂資財置辦節慶圍爐，熱氣騰騰的「{}」香氣撲鼻。席間你與同處一堂的{}圍坐共食、把盞相談，彼此情誼漸篤，四肢百骸俱生暖意。",
                    season_name, dish_name, npc_label
                ));
            } else {
                narrative.push_str(&format!(
                    "【歲時圍爐・時令節慶】當前時節正逢「{}」。你投入 3 鎂資財置辦節慶圍爐，品嚐熱氣蒸騰的時令佳餚「{}」，甘醇美味入喉，驅散長途跋涉風寒，周身元氣充沛舒泰。",
                    season_name, dish_name
                ));
            }
        } else {
            narrative.push_str("【資財不足】隨身資財不足，歲時圍爐入席需 3 鎂存入公庫置辦時令食材。");
        }
        push_refresh(conn, pid);
        return;
    }

    // 2. 茶屋烏木棋秤對弈 (wt_inn_ebony_shell_chess)
    if (obj.id == "wt_inn_ebony_shell_chess" || obj.id == "wt_cat_0795" || obj.name.contains("烏木棋盤") || obj.name.contains("圍棋") || obj.name.contains("白貝棋"))
        && (action == "PlayChess" || action == "Operate" || action == "Use")
    {
        narrative.clear();
        let nearby_resident = {
            let gh = current_game_hour(&conn.cfg);
            let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
            ents.into_iter().find(|e| e.id != pid && e.kind == crate::entity::EntityKind::Npc && e.vit > 0)
        };

        if let Some(npc) = nearby_resident {
            let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                format!("住民【{}】", npc.display_title.replace('|', "・"))
            } else {
                "對坐棋客".to_string()
            };
            let _ = db::adjust_favorability(pid, &npc.id, 5);
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在茶屋烏木棋秤前對弈手談，棋路相契", now);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 12).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "chess_fellowship_played");
            narrative.push_str(&format!(
                "【茶屋手談・落子同參】你端坐於烏木棋盤前，與席間{}分執黑白。盤上烏木黑子與厚海貝白子起落有致，推演虛實進退，彼此以棋會友相得益彰。局罷相視一笑，心神空明篤定，街坊情誼益發投契。",
                npc_label
            ));
        } else {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 8).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "chess_solo_studied");
            narrative.push_str("【茶屋獨弈・打譜推演】棋案空幽，你獨對烏木棋秤擺下殘局古譜。一人分飾黑白參悟攻防變化，篤篤落子之聲清脆迴響於茶舍之中，胸中雜慮盡消，內息澄澈流轉。");
        }
        push_refresh(conn, pid);
        return;
    }
    // v0.38.53 Pillar 1: 觀星拓印・周天命格星盤 (ChartAstrolabe)
    if (obj.id == "wt_cat_0734" || obj.id.starts_with("wt_astro_") || obj.id == "wt_obj_armillary_almanac" || obj.name.contains("星軌儀") || obj.name.contains("推步儀") || obj.name.contains("定緯銅儀"))
        && action == "ChartAstrolabe"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            let _ = db::add_to_inventory(pid, "item_personal_astrolabe_scroll", 1);
            let _ = db::add_to_inventory(pid, "wt_cat_0783", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 12).min(100);
                });
            }
            let (_, _, _, raw_days) = crate::game::game_time_now(now, conn.cfg.game_time_epoch_unix, conn.cfg.game_time_scale);
            let day_in_year = raw_days.max(0) % 365;
            let season_name = if day_in_year < 91 {
                "春融"
            } else if day_in_year < 182 {
                "夏漲"
            } else if day_in_year < 273 {
                "秋狩"
            } else {
                "冬凝"
            };
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "astrolabe_chart_rubbed");
            narrative.push_str(&format!(
                "【觀星拓印・周天命格星盤】你交付 3 鎂工本資財存入公庫，備齊竹綿宣紙與松煙精墨，細緻拓印黃銅星軌儀上的周天宿度。當前歲時正臨「{}」，星軌盤面澄澈顯映個人筋骨與山海氣脈的共鳴分野。你收穫一份「先天周天命格星盤拓卷」與「北辰歲差推步星圖」，內息亦隨天行周天而生生不息。",
                season_name
            ));
        } else {
            narrative.push_str("【資財不足】拓印個人周天命格星盤需 3 鎂工本資財存入公庫以備竹綿宣紙與松煙精墨。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.53 Pillar 1: 分家測紋・器脈校準 (TestEngraving)
    if (obj.id == "wt_martial_force_stone" || obj.id == "wt_forge_quench_trough" || obj.name.contains("測勁玄武碑") || obj.name.contains("導能測紋石板"))
        && action == "TestEngraving"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "test_engraving_calibrated");
        narrative.push_str("【分家測紋・器脈校準】你運轉周身氣息按向嵌有暗金紋理的測勁玄武碑，將隨身器物輕觸碑面導能凹槽。石板發出低沉清越的共鳴回響，器物重心與刃口紋路隨之校準歸正。工坊由聚落公庫撥付 4 鎂測紋津貼，內息運轉愈發圓融。");
        push_refresh(conn, pid);
        return;
    }

    // v0.38.53 Pillar 1: 試招拆勁・四序演武 (SparPillar)
    if (obj.id == "wt_arena_testing_pillar" || obj.id == "wt_martial_padded_wooden_blade" || obj.id == "wt_arena_wooden_weapon_rack" || obj.name.contains("玄武試勁柱"))
        && action == "SparPillar"
    {
        narrative.clear();
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 8).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "spar_pillar_trained");
        let nearby_resident = {
            let gh = current_game_hour(&conn.cfg);
            let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
            ents.into_iter().find(|e| e.id != pid && e.kind == crate::entity::EntityKind::Npc && e.vit > 0)
        };
        if let Some(npc) = nearby_resident {
            let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                format!("住民【{}】", npc.display_title.replace('|', "・"))
            } else {
                "同場武伴".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在試招場玄武試勁柱前切磋拆招，勁路相印", now);
            let _ = db::adjust_favorability(pid, &npc.id, 4);
            narrative.push_str(&format!(
                "【試招拆勁・四序演武】你繞著烏石鼻玄武試勁柱沉腰落馬，依序演練「起勢・蓄勁、進步・破鋒、轉樞・卸力、收勢・歸元」四段勁路，掌風與木刃擊打石柱激起沉雄悶響。同處試武場的{}凝神觀摩印證，彼此切磋拆招頗多得益，同道情誼漸深。",
                npc_label
            ));
        } else {
            narrative.push_str("【試招拆勁・四序演武】你繞著烏石鼻玄武試勁柱沉腰落馬，依序演練「起勢・蓄勁、進步・破鋒、轉樞・卸力、收勢・歸元」四段勁路，掌風與木刃擊打石柱激起沉雄悶響，周身筋骨舒張、內息順暢通達。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.53 Pillar 2: 泉香湯浴・祛寒舒筋 (SoakHotSpring)
    if (obj.id == "wt_bath_jade_pool" || obj.id == "wt_home_hotspring_tub" || obj.name.contains("恆溫熱湯池") || obj.name.contains("杉木沐浴桶"))
        && action == "SoakHotSpring"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            crate::pg::living_world::clear_entity_afflictions(pid, true, now);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 25).min(100);
                    e.qi = (e.qi + 20).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "soak_hot_spring_refreshed");
            narrative.push_str("【泉香湯浴・祛寒舒筋】你向浴場公庫交付 3 鎂浴資，浸入半嶺寒水玉鋪底、溫氏祛寒草木湯包浸泡的恆溫熱湯池中。溫潤泉水滲透肌膚經絡，拔除周身寒濕與跋涉勞損，四肢百骸暖意融融。");
        } else {
            narrative.push_str("【資財不足】進入熱湯池需 3 鎂存入浴場公庫以備溫泉引水與草木湯包。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.53 Pillar 2: 市井修面・煥然一新 (GroomAppearance)
    if (obj.id.starts_with("wt_barber_") || obj.id == "wt_bath_mirror_washstand" || obj.name.contains("摺疊剃刀架") || obj.name.contains("洗頭木躺椅") || obj.name.contains("銀鏡盥洗台"))
        && action == "GroomAppearance"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 2 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -2);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 14).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "groom_appearance_trimmed");
            let nearby_resident = {
                let gh = current_game_hour(&conn.cfg);
                let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
                ents.into_iter().find(|e| e.id != pid && e.kind == crate::entity::EntityKind::Npc && e.vit > 0)
            };
            if let Some(npc) = nearby_resident {
                let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                    format!("住民【{}】", npc.display_title.replace('|', "・"))
                } else {
                    "鋪內師傅".to_string()
                };
                crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在理髮修面鋪修整儀容，談吐清爽投契", now);
                let _ = db::adjust_favorability(pid, &npc.id, 4);
                narrative.push_str(&format!(
                    "【市井修面・煥然一新】你向聚落公庫交付 2 鎂修面資費，敷上蒸騰著薄荷清香的熱毛巾，由{}以精鋼摺疊剃刀與松香木梳修整鬢角鬚髮，談吐相談甚歡，神采頓時清爽端正。",
                    npc_label
                ));
            } else {
                narrative.push_str("【市井修面・煥然一新】你向聚落公庫交付 2 鎂修面資費，敷上蒸騰著薄荷清香的熱毛巾，以精鋼摺疊剃刀與松香木梳修整鬢角鬚髮，神采頓時清爽端正。");
            }
        } else {
            narrative.push_str("【資財不足】修面整容需 2 鎂資費存入聚落公庫以補毛巾草皂消耗。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.53 Pillar 2: 街角補綴・行裝牢固 (MendTravelGear)
    if (obj.id.starts_with("wt_repair_") || obj.id == "wt_cloth_tailor_sewing_bench" || obj.name.contains("修傘木砧檯") || obj.name.contains("補靴硬木楦") || obj.name.contains("裁縫量身几"))
        && action == "MendTravelGear"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 2 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -2);
            let _ = db::add_to_inventory(pid, "item_oiled_bamboo_umbrella", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 8).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mend_travel_gear_repaired");
            narrative.push_str("【街角補綴・行裝牢固】你交付 2 鎂工料費存入聚落公庫，在木砧檯與硬木楦前為隨身行靴敲緊生鐵底釘、抹勻厚魚油，並領取一把重新刷過熟桐油的「雙層桐油防雨竹傘」，足底與行囊俱皆穩妥。");
        } else {
            narrative.push_str("【資財不足】補綴行裝需 2 鎂工料資費存入聚落公庫以備生鐵釘與熟桐油。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.53 Pillar 3: 半嶺測候・山海風潮 (ReadWindVane)
    if (obj.id == "wt_observatory_anemometer" || obj.id == "wt_obj_wind_vane_gauge" || obj.id == "wt_obs_tide_weather_table" || obj.name.contains("黃銅風向儀") || obj.name.contains("相風銅烏"))
        && action == "ReadWindVane"
    {
        narrative.clear();
        let _ = current_game_hour(&conn.cfg);
        let (_, _, _, raw_days) = crate::game::game_time_now(now, conn.cfg.game_time_epoch_unix, conn.cfg.game_time_scale);
        let day_in_year = raw_days.max(0) % 365;
        let season_name = if day_in_year < 91 {
            "春融"
        } else if day_in_year < 182 {
            "夏漲"
        } else if day_in_year < 273 {
            "秋狩"
        } else {
            "冬凝"
        };
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 8).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "read_wind_vane_logged");
        narrative.push_str(&format!(
            "【半嶺測候・山海風潮】你仰觀八方黃銅風向儀與相風銅烏轉速，比對滴漏水鐘與潮汐曲線：當前正值「{}」，山海氣流穩定，外海潮位正處平順航期，十九獵場視野清明。觀測所由聚落公庫撥付 4 鎂測候記錄津貼。",
            season_name
        ));
        push_refresh(conn, pid);
        return;
    }

    // v0.38.53 Pillar 3: 溫氏百草・艾火溫灸 (ApplyMoxibustion)
    if (obj.id == "wt_clinic_acupoint_bed" || obj.id == "wt_clinic_decoction_stove" || obj.id == "wt_obj_acupuncture_pulse_pillow" || obj.name.contains("竹席診脈榻") || obj.name.contains("溫灸盒"))
        && action == "ApplyMoxibustion"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            crate::pg::living_world::clear_entity_afflictions(pid, true, now);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 22).min(100);
                    e.qi = (e.qi + 16).min(100);
                });
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "apply_moxibustion_healed");
            narrative.push_str("【溫氏百草・艾火溫灸】你向診所公庫交付 3 鎂診金，臥於細編竹席診脈榻上，由司藥點燃陳年艾絨置於溫灸盒溫熨關元與曲池穴。溫厚藥力透入經絡，將筋骨扭挫與陰寒鬱結盡數化開。");
        } else {
            narrative.push_str("【資財不足】百草溫灸需 3 鎂診金存入診所公庫以置辦陳艾與藥油。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.53 Pillar 3: 北嶺古杉・懸掛銘牌 (HangVotivePlaque)
    if (obj.id == "wt_obj_mountain_milestone" || obj.id == "wt_shrine_votive_plaque" || obj.name.contains("山道指路碑") || obj.name.contains("祈福木牌"))
        && action == "HangVotivePlaque"
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "item_spruce_votive_plaque", 1);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "hang_votive_plaque_inscribed");
        let nearby_resident = {
            let gh = current_game_hour(&conn.cfg);
            let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
            ents.into_iter().find(|e| e.id != pid && e.kind == crate::entity::EntityKind::Npc && e.vit > 0)
        };
        if let Some(npc) = nearby_resident {
            let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                format!("住民【{}】", npc.display_title.replace('|', "・"))
            } else {
                "同行山客".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在北嶺古杉指路碑前一同懸掛祈福木牌，互道平安", now);
            let _ = db::adjust_favorability(pid, &npc.id, 4);
            narrative.push_str(&format!(
                "【北嶺古杉・懸掛銘牌】你取過散發天然松脂清香的百年雲杉木牌，提筆題寫行旅平安祈願並懸於山道指路碑旁，同時收起一枚隨身「北嶺雲杉祈福木牌」。身旁的{}亦拱手致意、互道平安，山風拂過林梢，萬枚木牌發出清越柔和的共鳴。",
                npc_label
            ));
        } else {
            narrative.push_str("【北嶺古杉・懸掛銘牌】你取過散發天然松脂清香的百年雲杉木牌，提筆題寫行旅平安祈願並懸於山道指路碑旁，同時收起一枚隨身「北嶺雲杉祈福木牌」。山風拂過林梢，萬枚木牌發出清越柔和的共鳴。");
        }
        push_refresh(conn, pid);
        return;
    }
    // v0.38.54 Pillar 1: 鈔錄孤本 (TranscribeRareTome)
    if (obj.id == "wt_book_printing_shelf"
        || obj.id == "wt_book_pen_notebook_desk"
        || obj.name.contains("風物誌書架")
        || obj.name.contains("蘸水硬筆書案"))
        && action == "TranscribeRareTome"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            let _ = db::add_to_inventory(pid, "item_white_tower_gazetteer_scroll", 1);
            let _ = db::add_to_inventory(pid, "wt_cat_0727", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 14).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::ChamberClear,
                2,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "rare_tome_transcribed");
            narrative.push_str("【藏書鈔錄・風物孤本】你向公庫交付 3 鎂工本資財，引點「守歲松脂蜂蠟紅燭」，以鋼尖蘸水硬筆在竹綿宣紙上謹飭抄錄《白塔風物誌》地方孤本。燭火微明、墨香沉靜，你收穫「白塔風物孤本鈔卷」與一挺「守歲松脂蜂蠟紅燭」，胸臆神思朗潤澄澈，靜室清息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】抄錄地方孤本需 3 鎂工本資財存入公庫以備竹綿宣紙、精墨與紅燭。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.54 Pillar 1: 旁聽策論 (AttendLecture)
    if (obj.id == "wt_academy_fountain_pen"
        || obj.id == "wt_book_pen_notebook_desk"
        || obj.name.contains("黃銅儲墨筆")
        || obj.name.contains("蘸水硬筆書案"))
        && action == "AttendLecture"
    {
        narrative.clear();
        let (_, _, _, raw_days) = crate::game::game_time_now(now, conn.cfg.game_time_epoch_unix, conn.cfg.game_time_scale);
        let day_in_year = raw_days.max(0) % 365;
        let (season_topic, season_name) = if day_in_year < 91 {
            ("春融農事興修與堰道疏濬", "春融")
        } else if day_in_year < 182 {
            ("夏漲水運防汎與海防汛期", "夏漲")
        } else if day_in_year < 273 {
            ("秋狩倉廩積穀與四坊財賦", "秋狩")
        } else {
            ("冬凝工坊溫度與民生炭薪", "冬凝")
        };

        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 15).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "academy_lecture_attended");

        let nearby_resident = {
            let gh = current_game_hour(&conn.cfg);
            let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
            ents.into_iter().find(|e| e.id != pid && e.kind == crate::entity::EntityKind::Npc && e.vit > 0)
        };
        if let Some(npc) = nearby_resident {
            let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                format!("住民【{}】", npc.display_title.replace('|', "・"))
            } else {
                "同堂學子".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在講堂一同旁聽策論，研討經世時務", now);
            let _ = db::adjust_favorability(pid, &npc.id, 4);
            narrative.push_str(&format!(
                "【講堂策論・同窗共學】講席正逢「{}」歲時，教習與諸生正就「{}」各陳策論。你端坐旁聽、手錄精要，身旁的{}亦頷首會心，彼此就民生物用各抒己見、獲益良多，靜室清息凝練而同窗情誼益篤。",
                season_name, season_topic, npc_label
            ));
        } else {
            narrative.push_str(&format!(
                "【講堂策論・同窗共學】講席正逢「{}」歲時，教習正就「{}」深論經世之要。你端坐側席靜聆論辯，執筆詳錄策論節略，心神融會貫通，靜室清息隨之凝練。",
                season_name, season_topic
            ));
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.54 Pillar 1: 校時報刻 (CalibrateClock)
    if (obj.id == "wt_clocktower_bronze_bell"
        || obj.id == "wt_messenger_lookout_perch"
        || obj.name.contains("報時青銅鐘")
        || obj.name.contains("青銅旋轉晷"))
        && action == "CalibrateClock"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            2,
            now,
        );
        let gh = current_game_hour(&conn.cfg);
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "clock_calibrated_chimed");
        narrative.push_str(&format!(
            "【鐘樓校時・日晷報刻】你依循日晷影針與漏刻核驗天行度數，精準校正鐘樓發條，合力牽動粗麻繩敲響報時青銅鐘。清越浩蕩的鐘聲沉渾播揚全境，正是「{}時整」。司時司自公庫撥付 4 鎂津貼發放入你手中。",
            gh
        ));
        push_refresh(conn, pid);
        return;
    }

    // v0.38.54 Pillar 2: 榫卯校修 (FitMortiseTenon)
    if (obj.id == "wt_wood_planing_bench" || obj.name.contains("刨木工作台"))
        && action == "FitMortiseTenon"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 5);
        let _ = db::add_to_inventory(pid, "item_ebony_mortise_tea_tray", 1);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 8).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ForgeHeavy,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mortise_tenon_fitted");
        narrative.push_str("【巧木開榫・嚴絲合縫】你在刨木工作台前引鋸推刨，手持角尺細細鑿削沉水烏木榫卯。反覆嵌合校修之下，公母榫卯嚴絲合縫、扣合如一，製成一件精巧厚重的「沉水烏木榫卯茶盤」。木坊掌事依件計酬自公庫給付 5 鎂匠工辛勞工錢，鍛爐沉息隨之凝練。");
        push_refresh(conn, pid);
        return;
    }

    // v0.38.54 Pillar 2: 精鋼回火 (QuenchBlade)
    if (obj.id == "wt_forge_quench_trough"
        || obj.id == "wt_forge_furnace_anvil"
        || obj.name.contains("精鋼淬火槽")
        || obj.name.contains("耐火鍛造台"))
        && matches!(action, "QuenchBlade" | "精鋼回火")
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
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

            let repair_note = if let Some((v_id, v_name)) = vessel_target {
                let st = crate::pg::living_world::temper_vessel_at_forge(pid, &v_id, now);
                format!("隨身兵刃「{}」受爐火回火與淬火油滋養，器況重歸「{}」（人器磨合：{} —— {}）！", v_name, st.condition.label(), st.familiarity().label(), st.familiarity().feel_prose())
            } else {
                "隨身鐵料在槽中冷淬定型、消弭內應力，器物鋼性堅韌大增。".to_string()
            };

            let _ = db::add_to_inventory(pid, "item_quenched_steel_whetstone", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 10).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::ForgeHeavy,
                3,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "blade_quenched_tempered");
            narrative.push_str(&format!(
                "【精鋼回火・寒芒重煥】你交付 3 鎂工錢將熾紅鐵坯探入淬火油槽，白霧激盪翻騰、嗤啦大響，精鋼重煥寒芒、寒光奪目！{}\n你收得一枚「雙面精鋼油石」妥存於囊，鍛爐沉息隨之凝練。",
                repair_note
            ));
        } else {
            narrative.push_str("【資財不足】進行淬火回火需 3 鎂用料工錢存入工坊公庫以備淬火油與鍛爐精炭。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.54 Pillar 2: 藍染浸布 (DyeIndigoCloth)
    if (obj.id == "wt_cloth_garment_rack"
        || obj.id == "wt_obj_loom_dye_vats"
        || obj.name.contains("防雨長衫架")
        || obj.name.contains("踏板高織機"))
        && action == "DyeIndigoCloth"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 2 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -2);
            let _ = db::add_to_inventory(pid, "item_indigo_rain_cloak", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 12).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                2,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "indigo_cloth_dyed");
            narrative.push_str("【山藍浸染・桐油防雨】你向織造公庫交付 2 鎂料資，取過山藍發酵染液，將新織厚麻布反覆浸缸搥打、透風發色出深邃沉靜的靛青色澤，復刷上一層薄透熟桐油防雨。你收妥一件結實輕便的「山藍防雨斗篷」，海潮綿息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】浸染防雨布匹需 2 鎂用料資財存入公庫以備山藍染料與熟桐油。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.54 Pillar 3: 飼喂草獸 (FeedDraftBeast)
    if (obj.id == "wt_caravan_water_trough"
        || obj.id == "wt_obj_caravan_hitch_trough"
        || obj.name.contains("青石飲馬槽")
        || obj.name.contains("拴馬長木欄"))
        && action == "FeedDraftBeast"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 10).min(100);
                e.qi = (e.qi + 8).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ForestLong,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "draft_beast_fed");

        let nearby_resident = {
            let gh = current_game_hour(&conn.cfg);
            let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
            ents.into_iter().find(|e| e.id != pid && e.kind == crate::entity::EntityKind::Npc && e.vit > 0)
        };
        if let Some(npc) = nearby_resident {
            let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                format!("住民【{}】", npc.display_title.replace('|', "・"))
            } else {
                "同行車夫".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "協力飼獸", now);
            let _ = db::adjust_favorability(pid, &npc.id, 4);
            narrative.push_str(&format!(
                "【車馬添草・飲馬活泉】你抱來金黃清香的乾燕麥草料添滿木欄食槽，拔開活水木栓注滿青石槽。奔馳長途的原牛與挽馬低頭歡暢嚼食、痛飲清泉，發出滿足舒坦的響鼻。身旁的{}拱手致謝，驛站照看依例自公庫給付 4 鎂草料津貼，山林長息隨之凝練。",
                npc_label
            ));
        } else {
            narrative.push_str("【車馬添草・飲馬活泉】你抱來金黃清香的乾燕麥草料添滿木欄食槽，拔開活水木栓注滿青石槽。奔馳長途的原牛與挽馬低頭歡暢嚼食、痛飲清泉，發出滿足舒坦的響鼻。驛站照看依例自公庫給付 4 鎂草料津貼，山林長息隨之凝練。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.54 Pillar 3: 登船遠航備用 (BoardCoastalShip on wt_dock_cargo_gangplank / 雲杉卸貨板)
    if (obj.id == "wt_dock_cargo_gangplank" || obj.name.contains("雲杉卸貨板"))
        && action == "BoardCoastalShip"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 4 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -4);
            let _ = db::add_to_inventory(pid, "item_coastal_sea_chart", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 14).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                3,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "coastal_ship_boarded_gangplank");
            narrative.push_str("【順流巡航・黑潮海圖】你向公庫交付 4 鎂船資，順著厚實雲杉卸貨板穩步登上海船甲板。商船解纜順潮遠航，海風獵獵、白浪滔滔，你於航行途中參核黑潮走勢，收下一份「黑潮順時針海圖摺頁」，海潮綿息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】登船出航需交付 4 鎂船資存入公庫。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.54 Pillar 3: 查閱地籍 (InspectLandRegistry)
    if (obj.id == "wt_deed_cadastral_map_desk"
        || obj.id == "wt_deed_household_registry"
        || obj.name.contains("四坊地籍案")
        || obj.name.contains("樟木戶籍櫃"))
        && action == "InspectLandRegistry"
    {
        narrative.clear();
        let _ = db::add_to_inventory(pid, "item_wenyao_cadastral_rubbing", 1);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            2,
            now,
        );
        let treasury = crate::npc::causal_core::settlement_treasury_balance("文曜");
        let prosperity_desc = if treasury >= 2800 {
            "城坊倉廩充盈，四民熙攘安居，公產界石井然有序"
        } else if treasury >= 1800 {
            "四坊市井百工殷實，田宅契據分明，物資流轉平穩有序"
        } else {
            "百業休養生息，街屋界址精細標核，公庫調配量入為出"
        };
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "cadastral_registry_inspected");
        narrative.push_str(&format!(
            "【四坊地籍・民生疆界】你端凝撫按長案上的四坊地籍總圖與樟木戶籍檔案，朱砂經緯與戶號界石標註井然。當前聚落風貌正逢「{}」。你細閱文曜坊巷格局與公產脈絡，拓印下一頁「文曜四坊地籍圖鈔頁」留念，靜室清息隨之凝練。",
            prosperity_desc
        ));
        push_refresh(conn, pid);
        return;
    }

    // v0.38.55 Pillar 1: 幫廚備膳 (AssistKitchen)
    if (obj.id == "wt_dining_iron_cauldron"
        || obj.id == "wt_dining_rice_milk_stall"
        || obj.name.contains("紅鐵燉肉釜")
        || obj.name.contains("溫米漿桶"))
        && action == "AssistKitchen"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        let _ = db::add_to_inventory(pid, "item_braised_beef_broth_jar", 1);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 15).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ForgeHeavy,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "kitchen_assisted");
        narrative.push_str("【幫廚備膳・市井炊煙】你挽起衣袖在紅鐵燉肉大釜與溫米漿桶前添柴調羹、分盛熱湯，熱氣騰騰的香氣溢滿三水大食堂。掌廚師傅依例自文曜公庫支領 4 鎂幫廚津貼交予你，並盛了一盅熱氣滾滾的「慢燉原牛肉湯陶盅」，爐火重息隨之凝練。");
        push_refresh(conn, pid);
        return;
    }

    // v0.38.55 Pillar 1: 焙茶品茗 (RoastMountainTea)
    if (obj.id == "wt_inn_milk_tea_stove"
        || obj.name.contains("煮茶泥爐"))
        && action == "RoastMountainTea"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 2 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -2);
            let _ = db::add_to_inventory(pid, "item_roasted_frost_tea_tin", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 16).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::ChamberClear,
                2,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "mountain_tea_roasted");
            let nearby_resident = {
                let gh = current_game_hour(&conn.cfg);
                let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
                ents.into_iter().find(|e| e.id != pid && e.kind == crate::entity::EntityKind::Npc && e.vit > 0)
            };
            if let Some(npc) = nearby_resident {
                let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                    format!("住民【{}】", npc.display_title.replace('|', "・"))
                } else {
                    "同席茶客".to_string()
                };
                crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在山風茶屋圍爐焙茶品茗，閒話風土時務", now);
                let _ = db::adjust_favorability(pid, &npc.id, 4);
                narrative.push_str(&format!(
                    "【山風茶屋・焙茶品茗】你投入 2 鎂炭火茶資存入公庫，於泥爐上輕夾松木炭翻焙高山青茶。炭香微發、茶韻清幽，你將一罐「炭焙霜林烘青茶罐」妥善收妥入囊。身旁的{}共品甘茗、閒話風土，喉舌甘潤心曠神怡，靜室清息隨之凝練。",
                    npc_label
                ));
            } else {
                narrative.push_str("【山風茶屋・焙茶品茗】你投入 2 鎂炭火茶資存入公庫，於泥爐上輕夾松木炭翻焙高山青茶。炭香微發、茶韻清幽，你將一罐「炭焙霜林烘青茶罐」妥善收妥入囊，自斟自品甘醇生津，頓覺塵囂盡散，靜室清息隨之凝練。");
            }
        } else {
            narrative.push_str("【資財不足】山風茶屋圍爐焙茶需交付 2 鎂茶資存入聚落公庫。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.55 Pillar 1: 曝書防蠹 (SunBooks)
    if (obj.id == "wt_book_printing_shelf"
        || obj.id == "wt_study_bamboo_bookshelf"
        || obj.name.contains("風物誌書架")
        || obj.name.contains("線裝古籍架"))
        && action == "SunBooks"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 12).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "books_sun_dried");
        narrative.push_str("【石坪曬書・古籍辟蠹】你自架上搬出線裝古籍與風物圖冊，於向陽通風的曬書石坪上依序攤晾翻曬，逐卷拂拭夾入辟蠹芸香。閣中依護書常例自公庫支領 4 鎂曝書津貼交予你，滿坪墨香悠遠，靜室清息隨之凝練。");
        push_refresh(conn, pid);
        return;
    }

    // v0.38.55 Pillar 2: 炮製藥材 (RoastHerbs)
    if (obj.id == "wt_herb_drying_kiln"
        || obj.id == "wt_herb_bamboo_sieve"
        || obj.name.contains("紅泥焙藥爐")
        || obj.name.contains("切藥鍘刀案"))
        && action == "RoastHerbs"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            let _ = db::add_to_inventory(pid, "item_seven_star_moxa_pack", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 14).min(100);
                    e.qi = (e.qi + 12).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::ForestLong,
                2,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "herbs_roasted_prepared");
            narrative.push_str("【百草炮製・九蒸溫養】你向公庫交付 3 鎂草藥材料費，坐守焙藥爐前掌按文火炭灰，並以鍘刀細切陳艾與黃精。藥材經烘焙透散甘醇香氣，你將精製的一份「九蒸七星艾絨藥包」妥善收妥入囊，山林長息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】炮製草藥需交付 3 鎂藥材料費存入公庫。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.55 Pillar 2: 校勘碑帖 (CollateInscriptions)
    if (obj.id == "wt_study_desk_inkstone"
        || obj.id == "wt_study_bamboo_bookshelf"
        || obj.id == "wt_obj_stone_rubbing_cabinet"
        || obj.name.contains("烏木書案")
        || obj.name.contains("線裝古籍架")
        || obj.name.contains("碑拓石櫥"))
        && action == "CollateInscriptions"
    {
        narrative.clear();
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 4);
        let _ = db::add_to_inventory(pid, "item_collated_stele_rubbing", 1);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 14).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "inscriptions_collated");
        narrative.push_str("【金石校勘・字句審定】你在烏木書案前對照碑拓石櫥原拓與線裝古籍，細細辨校漫漶殘缺金石字句，硃筆點定異同。學堂依校勘常例由公庫給付 4 鎂勞酬，並將一份「精校金石碑帖拓本」交付於你，靜室清息隨之凝練。");
        push_refresh(conn, pid);
        return;
    }

    // v0.38.55 Pillar 2: 研磨鏡片 (GrindOpticLens)
    if (obj.id == "wt_obs_brass_telescope_rack"
        || obj.id == "wt_book_map_reading_table"
        || obj.name.contains("青銅窺管架")
        || obj.name.contains("全圖閱覽桌"))
        && action == "GrindOpticLens"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            let _ = db::add_to_inventory(pid, "item_polished_quartz_lens", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.qi = (e.qi + 12).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::ChamberClear,
                3,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "optic_lens_ground");
            narrative.push_str("【光學精磨・水晶鏡片】你向公庫交付 3 鎂水晶礦材費，蘸取金剛微砂在旋轉圓盤上細細精磨紅土水晶。冷水潺潺洗去砂痕，鏡面光潔澄澈、纖毫畢現，你將一枚「精磨紅土水晶鏡片」妥善收妥入囊，靜室清息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】研磨光學鏡片需交付 3 鎂水晶耗材費存入公庫。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.55 Pillar 3: 揀選海味 (SelectDriedSeafood)
    if (obj.id == "wt_market_produce_stand"
        || obj.id == "wt_market_scale_counter"
        || obj.name.contains("山海乾貨攤")
        || obj.name.contains("黃銅雙盤秤"))
        && action == "SelectDriedSeafood"
    {
        narrative.clear();
        let player_money = db::get_entity(pid).ok().flatten().map(|e| e.magnesium).unwrap_or(0);
        if player_money >= 3 {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
            let _ = db::add_to_inventory(pid, "item_smoked_mullet_roe_pack", 1);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 12).min(100);
                    e.qi = (e.qi + 8).min(100);
                });
            }
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                2,
                now,
            );
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "dried_seafood_selected");
            narrative.push_str("【三水口揀選・海味精禮】你交付 3 鎂海產資財入公庫，在攤前細細辨選金黃透亮的冷燻烏魚子，以黃銅雙盤秤精確秤平份量，購得一份「風櫃尾冷燻烏魚子禮包」，海潮綿息隨之凝練。");
        } else {
            narrative.push_str("【資財不足】揀選精裝海味乾貨需交付 3 鎂資財存入公庫。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.55 Pillar 3: 參議鄉約 (DeliberateCivicPact)
    if (obj.id == "wt_hall_council_table"
        || obj.id == "wt_hall_bulletin_board"
        || obj.name.contains("厚木公議桌")
        || obj.name.contains("民務告示欄"))
        && action == "DeliberateCivicPact"
    {
        narrative.clear();
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 14).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "civic_pact_deliberated");
        let nearby_resident = {
            let gh = current_game_hour(&conn.cfg);
            let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
            ents.into_iter().find(|e| e.id != pid && e.kind == crate::entity::EntityKind::Npc && e.vit > 0)
        };
        if let Some(npc) = nearby_resident {
            let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                format!("住民【{}】", npc.display_title.replace('|', "・"))
            } else {
                "與會街坊".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在公議堂同席參議鄉約，共商水土民生大計", now);
            let _ = db::adjust_favorability(pid, &npc.id, 4);
            narrative.push_str(&format!(
                "【公議堂・參議鄉約】你在厚木公議桌前端坐參核鄉約草條，與同席的{}共商水土防護與市井民生，彼此切磋見解、共定保甲章則，靜室清息隨之凝練。",
                npc_label
            ));
        } else {
            narrative.push_str("【公議堂・參議鄉約】你在厚木公議桌前端坐參核鄉約草條，細閱水土修防與四坊民務法度，思慮明晰周詳，靜室清息隨之凝練。");
        }
        push_refresh(conn, pid);
        return;
    }

    // v0.38.55 Pillar 3: 汲水潤嗓 (DrawSpringWell)
    if (obj.id == "wt_river_stone_steps"
        || obj.id == "wt_alley_stone_well"
        || obj.name.contains("汲水青石階")
        || obj.name.contains("青石古井"))
        && action == "DrawSpringWell"
    {
        narrative.clear();
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 12).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        crate::pg::living_world::record_breath_activity(
            pid,
            crate::pg::living_world::BreathHabitKind::ForestLong,
            2,
            now,
        );
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "spring_well_drawn");
        let nearby_resident = {
            let gh = current_game_hour(&conn.cfg);
            let ents = db::get_entities_in_room(player_room, gh).unwrap_or_default();
            ents.into_iter().find(|e| e.id != pid && e.kind == crate::entity::EntityKind::Npc && e.vit > 0)
        };
        if let Some(npc) = nearby_resident {
            let npc_label = if crate::store::player_knows_npc(pid, &npc.id) && !npc.display_title.trim().is_empty() {
                format!("住民【{}】", npc.display_title.replace('|', "・"))
            } else {
                "汲水街坊".to_string()
            };
            crate::npc::causal_core::record_player_npc_fellowship_hook(&npc.id, pid, "曾在水岸公井旁汲水寒暄，互道街坊安好", now);
            let _ = db::adjust_favorability(pid, &npc.id, 3);
            narrative.push_str(&format!(
                "你立於汲水青石階旁，搖動轆轤打上一桶沁涼甘泉，舀起清冽泉水痛飲潤嗓。身旁的{}提桶寒暄、互道街坊安好，清涼直透腑臟，山林長息凝練，鄰里情誼漸厚。",
                npc_label
            ));
        } else {
            narrative.push_str("你立於汲水青石階旁，搖動轆轤打上一桶沁涼甘泉，舀起清冽泉水痛飲潤嗓，甘澈清泉直透腑臟，胸中燥熱頓消，山林長息凝練。");
        }
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
                        "\n【利刃磨礪】推磨數下後，你發現「{}」{}。此類捲刃待修重損，街邊尋常青石磨刀槽難以推平，須赴鍛造工坊藉由鍛爐⚒︎火候與鐵砧輕敲校直才能修整復原。",
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

    if (obj.id == "wt_alley_stone_well"
        || (obj.name.contains("水槽")
            && !obj.name.contains("熱水槽")
            && !obj.name.contains("溫石")
            && !obj.name.contains("淬火")
            && !obj.name.contains("飲馬槽")
            && obj.id != "wt_inn_bath_Preview"
            && obj.id != "wt_caravan_water_trough")
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
        narrative.push_str("\n【清泉洗漱】你搖動轆轤打起半桶深井甘泉，清冽泉水撲面而來，洗去了一身風塵與倦意，渾身毛孔透徹舒泰。");
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
        narrative.push_str("\n【查閱地籍戶譜】你仔細翻閱公議堂案几上的地籍總圖與落籍戶譜。各坊街屋坐標界石標註分明，田畝產權清晰可稽，令人對文曜建置井然肅然起敬。");
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
        narrative.push_str("\n【觀測天象潮汐】你凝神查閱四時潮汐氣象推算案並透過雙筒望遠銅鏡俯瞰海天交界。風雲變幻與水運起落盡收眼底，胸懷頓時為之開闊。");
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
            narrative.push_str("\n【乘筏觀潮】你解開繫在石樁上的粗纜，平底木筏隨著港灣潮汐輕輕起伏，海風獵獵拂面，遠眺海天一色，心神頓覺開闊澄明。");
        } else {
            narrative.push_str("\n【乘筏信步】你解開繫在青石樁上的桐油麻繩，手持長竹篙在石階輕撐借力，平底竹筏滑入三水川清澈碧流。沿岸水草豐美、水鳥掠波，心神隨流水漸入澄澈。");
        }
        append_somatic_awareness_note(pid, now, narrative);
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
        narrative.push_str("\n【細心灌溉】清涼甘泉緩緩注入泥土，泥土與草木散發出質樸而清新的生機芬芳。靜心照拂草木之間，心神漸趨安寧。");
        if obj.name.contains("藥圃") || obj.id == "wt_reg_wenyao_garden" {
            let _ = db::add_to_inventory(pid, "mat_foraged_herb", 1);
            narrative.push_str("\n【草木採收】你在藥圃中細心摘取了一份清香四溢的「野生採集草藥」收入行囊。");
        }
        push_refresh(conn, pid);
        return;
    }

    if (action == "Use" || action == "Operate")
        && (obj.id == "wt_home_shell_lamp" || obj.name.contains("台燈") || obj.name.contains("檯燈"))
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
        narrative.push_str("\n【旋燈調光】你輕輕旋轉床頭夜光貝母檯燈的烏木旋蓋，柔和溫潤的珠白光芒隨之微調，滿室皆是安恬暖意，心神漸入安恬。");
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
        narrative.push_str("\n【研墨展卷】你取墨錠在端硯清泉中徐徐研磨，墨香四溢。隨後提筆蘸墨在宣紙上信手書寫，筆意沉著，心神豁然澄澈。");
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
        narrative.push_str("\n【檢閱典籍】你自架上抽出一冊線裝古籍翻閱，紙質柔韌吸墨，墨香沁人心脾。先賢微言大義躍然紙上，令人心曠神怡。");
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
        narrative.push_str("\n【星軌推步】你推動星軌儀的精銅外環，齒輪與軸承咬合平順，星軌刻度隨之流轉對齊今日天象，心中豁然開朗。");
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
        narrative.push_str("\n【撥珠速算】你信手撥動黑檀算珠，珠落聲清脆利落如落玉盤。心中速算推演，思維條理頓時無比清晰。");
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
        narrative.push_str("\n【排版試印】你拿起棕櫚墨刷在排版活字上勻勻刷上一層松煙墨，覆上宣紙輕輕滾壓，字跡頓時端正顯映於紙上。");
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
        narrative.push_str("\n【蕩料抄紙】你雙手執竹簾浸入石槽，平順蕩料抄起一層細密纖維，揭紙貼於熱牆烘乾，紙面瑩白平滑，工藝之妙令人讚歎。");
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
        narrative.push_str("\n【焙藥調溫】你撥動紅泥爐膛的通氣瓦片微調火候，熱氣蒸騰間草木本質被徐徐焙透，濃郁藥香透體而入，體魄與精神皆為之一振。");
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
        narrative.push_str("\n【操刀切藥】你手握切藥鍘刀木柄壓下，利刃切開晾乾的草藥根莖，發出清脆嚓響，藥香濃烈撲面，筋骨隨之舒展。");
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
        if narrative.is_empty() {
            narrative.push_str("【埠階掬泉】你踏上青石汲水埠階，俯身掬起清涼甘洌的川流活水洗滌面容，暑氣盡散，神清氣爽。");
        }
        if action == "Gather" || action == "Operate" {
            let _ = db::add_to_inventory(pid, "mat_iron_scrap", 1);
            narrative.push_str("\n【岸畔淘料】你在埠階水邊清淤淘洗，順道自泥沙礫石中淘獲一塊堅實的【回收熟鐵料塊】收入行囊。");
        }
        if action == "Meditate" {
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
        narrative.push_str("\n【竹苑啜茗】你在微涼青石凳上端坐歇息，以竹筧甘泉瀹茶，修竹沙沙入盞，頓覺身心放鬆，塵慮盡消。");
        if action == "Meditate" {
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
        if action == "Meditate" {
            append_somatic_awareness_note(pid, now, narrative);
        }
        push_refresh(conn, pid);
        return;
    }
    // --- 文曜外環工坊、碼頭、窯廠、紙墨印造、倉庫等重工勞動器物 ---
    if action != "WarmHands" && (matches!(
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
    )) {
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
                "你推動長柄鋼齒鐵斗探入碼頭下沿清除河道淤泥暗沙，將沉積泥沙整車推卸至駁岸外，航道重獲通暢清澈。",
            ),
            "wt_outer_reed_rack" | "wt_obj_timber_reed_stacks" => (
                4,
                8,
                "你搬運運下排筏的三年生毛竹與重型巨杉圓木，按營造尺寸整齊排垛並蓋好防潮油布，通風堆垛井然有序。",
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
                "你腳踏高織機踏板投梭引緯，重密粗麻經緯緊扣交織，隨後將新織防風帆布下缸浸染群青，布料抗撕拉力極佳。",
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
                "你熟練操縱磅秤與搬運墊木，清點度量進出大宗貨物斤兩，動作俐落嚴謹，帳物分明。",
            ),
        };
        let actual_cost = if vit <= 10 { 1 } else { cost_vit };
        let _ = db::update_vit(pid, (vit - actual_cost).max(2));
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, wage);
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        narrative.push_str(&format!(
            "\n{}\n你投入體魄勞作，自市場營造司領取當班工造津貼收入行囊，筋骨雖感微乏，補給資財已然妥善結算。",
            labor_desc
        ));
        let inv_qty = |item_id: &str| -> i32 {
            ent.as_ref()
                .map(|e| {
                    crate::db::get_inventory(&e.inventory, e.vit)
                        .items
                        .iter()
                        .find(|it| it.item_id == item_id)
                        .map(|it| it.qty)
                        .unwrap_or(0)
                })
                .unwrap_or(0)
        };
        if (obj.name.contains("毛竹") || obj.id == "wt_outer_reed_rack" || obj.id == "wt_obj_timber_reed_stacks") && inv_qty("mat_timber_log") < 4 {
            let _ = db::add_to_inventory(pid, "mat_timber_log", 1);
            narrative.push_str("\n你在搬運整理排筏竹木之餘，順道自料垛獲贈了一根韌性上乘的「伐採原木」收入行囊。");
        } else if (obj.name.contains("鹽倉") || obj.id == "wt_outer_salt_vat" || obj.id == "wt_obj_iron_salt_stacks") && inv_qty("mat_iron_scrap") < 4 {
            let _ = db::add_to_inventory(pid, "mat_iron_scrap", 1);
            narrative.push_str("\n你在協助鹽倉搬運壓包之際，獲贈了一塊用以加固木架的「回收熟鐵料塊」收入行囊。");
        } else if obj.name.contains("蘆葦") && inv_qty("mat_reed_bundle") < 4 {
            let _ = db::add_to_inventory(pid, "mat_reed_bundle", 1);
            narrative.push_str("\n你順手割取了一捆青翠韌實的「水澤老蘆草綑」收入行囊。");
        } else if obj.id == "wt_forge_furnace_anvil"
            || obj.name.contains("鍛造")
            || obj.name.contains("鐵砧")
            || obj.name.contains("鍛爐")
            || obj.name.contains("鍛台")
        {
            if inv_qty("mat_iron_scrap") < 4 {
                let _ = db::add_to_inventory(pid, "mat_iron_scrap", 1);
                narrative.push_str("\n你在高爐鍛台旁清整爐渣時，順手拾得一塊成色純良的「回收熟鐵料塊」收入行囊。");
            }

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
                    "\n你順手將受損的「{}」探入炭火回火、於鐵砧輕敲校直，{}（{}）！",
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
        } else if (obj.id == "wt_wood_planing_bench" || obj.name.contains("木作台")) && inv_qty("mat_timber_log") < 4 {
            let _ = db::add_to_inventory(pid, "mat_timber_log", 1);
            narrative.push_str("\n你在木作台下整理邊角良木，順手收起一截乾燥結實的「伐採原木」收入行囊。");
        }
        if (obj.id == "wt_forge_quench_trough" || obj.name.contains("淬火槽")) && action == "Quench" {
            if let Ok((_mut_id, _mut_name, mut_narrative)) =
                crate::game::object_ecology::mutate_finished_item(pid, "auto", &obj.name, now)
            {
                narrative.push_str(&format!("\n藉著導能淬火槽的極溫冷熱激盪，{}", mut_narrative));
            }
        }
        push_refresh(conn, pid);
        return;
    }

    // --- 文曜外環餐飲、茶攤、食肆器物 ---
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
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -cost);
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
                    let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -3);
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
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 5);
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
                "你取下淬火開刃的鋼鋤鐮刀細看刃線，翻檢鐵匣鉚釘規格，百工利器質樸精準，倍添營造踏實之感。"
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
                "\n【城郭風物・格物研察】{}\n體魄更顯健旺，心神元氣充盈。",
                feedback
            ));
        } else {
            narrative.push_str(&format!(
                "\n【城郭風物・格物研察】{}\n心神寧靜豁然，周身元氣充盈。",
                feedback
            ));
        }
        push_refresh(conn, pid);
        return;
    }



    // 3) Food, Drink & Tea (action == "Taste" or obj.id == "wt_canteen_red_iron_cauldron" or obj.id == "wt_canteen_fermented_tofu_jar" or obj.id == "wt_inn_hearth_tea_table" or catalog category "11")
    let cat_opt = crate::game::peninsula_catalog::find_catalog_entry(&obj.id);
    let is_food_item = cat_opt.is_some_and(|c| {
        matches!(c.category_num, "08" | "09" | "10") || c.socket == "Taste"
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
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -cost);
            }
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 8).min(100);
                    e.qi = (e.qi + 6).min(100);
                });
            }
            if cost > 0 {
                narrative.push_str(&format!(
                    "\n【飲食溫養】花費 {} 鎂，溫熱鮮美的飲食入腹，你感到氣血充盈、精神一振。",
                    cost
                ));
            } else {
                narrative.push_str(
                    "\n【飲食溫養】溫熱鮮美的飲食入腹，你感到氣血充盈、精神一振。",
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
        let rest_note = if !narrative.trim().is_empty() {
            ""
        } else if action == "Sleep" || obj.id.contains("bed") || obj.name.contains("大床") || obj.name.contains("羽絨暖被") || obj.name.contains("臥榻") || obj.name.contains("床鋪") {
            "你鑽入溫暖蓬鬆的羽絨暖被安穩入眠，滿室松香與安神草清香令人心神寧靜，周身疲憊盡數散去。"
        } else if action == "Bath" || action == "Wash" || obj.id.contains("bath") || action == "DryOff" || obj.name.contains("熱水槽") || obj.name.contains("溫石") {
            "熱氣騰騰的溫石活水漫過身軀，洗去了一路風塵與肌理酸脹，身心倍感舒泰通透。"
        } else if obj.id.contains("clinic") {
            "經脈絡調理與草木藥香溫養，周身酸痛與疲憊盡數散去，體魄與心神皆充盈圓滿。"
        } else if action == "Meditate" {
            "你盤膝端坐閉目凝神，調勻呼吸，漸入物我兩忘之境，周身真氣漸趨充沛圓融。"
        } else {
            "你在清風中稍作歇息，呼吸漸趨平緩，周身疲憊隨之散去，體魄與心神皆充盈圓滿。"
        };
        if !rest_note.is_empty() {
            narrative.push_str(rest_note);
        }
        if matches!(action, "Meditate" | "Practice") {
            append_somatic_awareness_note(pid, now, narrative);
        }
        push_refresh(conn, pid);
        return;
    }

    // 5) Gathering & Portable Item Acquisition (action == "Gather" || action == "Acquire" or portable wt_cat_xxxx item in shops/workshops/wilderness when action == "Use" || action == "Operate")
    if let Some(cat) = cat_opt {
        if action == "Gather" {
            if cat.category_num.starts_with("09")
                || cat.category_num.starts_with("10")
                || cat.category_num.starts_with("11")
                || cat.category_num.starts_with("12")
                || cat.category_num.starts_with("13")
                || cat.category_name.starts_with("09_")
                || cat.category_name.starts_with("10_")
                || cat.category_name.starts_with("11_")
                || cat.category_name.starts_with("12_")
                || cat.category_name.starts_with("13_")
            {
                narrative.push_str(&format!(
                    "\n【櫃檯提示】「{}」為工坊堂食或市集現售物品，非野外採集之物，請於櫃檯照價購買或細細品嚐。",
                    cat.name
                ));
                push_refresh(conn, pid);
                return;
            }
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
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -price);
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
    if matches!(action, "Meditate" | "Practice") {
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
    let meridian_prose = meridian_prose.replace("其人", "你");
    narrative.push_str(&format!(
        "\n此際斂氣澄心，體內「{}」隨呼吸自如流轉——{}。\n內觀周天經絡：{}",
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

    let has_target_obj = crate::server::grid_manager::get_runtime_grid_cell(coord)
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
            Some("你已在已排定的側線裡，兌牌人抬手要你留在門檻外，不讓後面的人重新擠上來。")
        }
        ("m3_020_mint_half_open_door", "Queue")
            if m3_020_waiting_line_has_been_used(prior_payloads) =>
        {
            Some(
                "你按分出的側線站到門檻外，兌牌人沒有放你進門，只把下一輪要核的牌號往石案邊推近；身後的人跟著把腳步收成一線。",
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
                "你照著看清的牌序站到石案側邊，門衛把你的去向壓進短冊，讓商隊和工坊送件的人各退半步。",
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
            "你排進去的側線已被兌牌人接住，門檻外留下一條窄窄空位；他偶爾把下一張牌號往石案邊推近。",
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
            Some("你應下的那片短工牌被棚頭人翻到繩結下，商隊卸貨的小隊在棚柱旁收成一列。")
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
            Some("你候檢的那道牌序被門衛壓在石案側邊，商隊和工坊送件的人各讓出半步。")
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
            Some("你押名的短冊被記牌人壓在告示下方，通行牌申訴那一列多了一道折角。")
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
            Some("讀過的幾張通行牌仍壓在石案邊，你再看時能更快分清商隊和工坊送件的候列。")
        }
        ("m3_020_flagstone_waiting_lines", "Use") => {
            Some("你讓出的那一步還留在候列邊，旗手順手把後面的人壓到同一側。")
        }
        ("m3_020_mint_half_open_door", "Open") => Some("門縫比先前更窄，兌牌人的手還壓在門板上。"),
        ("m3_020_hire_short_work_boards", "Read") => {
            Some("最急的那片短工牌仍翻在外側，棚頭人正按那幾個活計點人。")
        }
        ("m3_020_council_notice_strip", "Read") => {
            Some("你看過的幾行告示還在階邊，記牌人按著短冊不讓申訴人擠到坡街中間。")
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
            Some("廣場那邊理出的側線延伸到巷口，等換票的人比先前收得更窄。")
        }
        ("m3_020_hire_short_work_boards", "m3_020_flagstone_waiting_lines", "Use") => {
            Some("旗石廣場的候列分清後，等工的人也被棚頭人壓到棚柱旁。")
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
                "你撥開貼岸的河霧，順著草莖根部一氣呵成連割了數把；你抖落泥水，將它們俐落地紮成 {} 束「{}」妥善收進背包；{trace}",
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
    player_room: &str,
    object_id: &str,
    action: &str,
    state_payload: Option<&str>,
) -> bool {
    match (object_id, action, state_payload) {
        ("m3_027_hot_soup_stall", "BuySoup", Some(M3_027_SOUP_WARM_STATE)) => {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -M3_027_SOUP_COST);
            true
        }
        ("m3_027_cargo_pole", "CarryCargo", Some(M3_027_CARGO_HELP_STATE)) => {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, M3_027_CARGO_PAY);
            true
        }
        ("m3_027_market_reed_buyer", "SellReeds", Some(state))
            if state == M3_027_REED_SOLD_STATE || state == M3_027_REED_DEMAND_FILLED_STATE =>
        {
            let _ = db::remove_from_inventory(pid, M3_027_REED_ITEM_ID, 1);
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, M3_027_REED_SELL_PAY);
            true
        }
        (
            "m3_027_market_reed_buyer",
            "SellSackcloth",
            Some(M3_027_SACKCLOTH_DEMAND_FILLED_STATE),
        ) => {
            let _ = db::remove_from_inventory(pid, M3_027_SACKCLOTH_ITEM_ID, 1);
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, M3_027_SACKCLOTH_SELL_PAY);
            true
        }
        ("m3_027_market_reed_buyer", "SellRope", Some(M3_027_ROPE_DEMAND_FILLED_STATE)) => {
            let _ = db::remove_from_inventory(pid, M3_027_ROPE_ITEM_ID, 1);
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, M3_027_ROPE_SELL_PAY);
            true
        }
        ("m3_027_warehouse_tally_board", "CheckCargo", Some(M3_027_WAREHOUSE_TALLY_STATE)) => {
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, M3_027_CARGO_TALLY_PAY);
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
            Some("原先讓出的乾位已被別人的包袱佔回去；你再靠近時，長凳只剩濕冷那一段。")
        }
        ("m3_027_hot_soup_stall", "BuySoup" | "QueueBroth")
            if m3_027_soup_taken_state_active(prior_states) =>
        {
            Some("攤主把勺子靠回鍋沿，說這一輪已經輪過你；鍋底的熱氣留給下一個被河風吹透的人。")
        }
        ("m3_027_cargo_pole", "CarryCargo") if m3_027_cargo_help_state_active(prior_states) => {
            Some("你扛過的那副貨擔已經停在踏板邊，市集的人把下一擔交給熟手，沒有再把肩位讓出來。")
        }
        ("m3_027_boat_rope", "HelpRope") if m3_027_rope_help_state_active(prior_states) => {
            Some("船戶看見你已經搭過一手，只讓你留在踏板旁的窄位等船靠穩；繩尾這回交給後面的人。")
        }
        ("m3_027_boat_rope", "HelpRope") if !m3_027_cargo_help_state_active(prior_states) => {
            Some("船戶把繩尾往掌心一收，先指向踏板邊的貨擔；船還沒靠穩，空手擠上去只會擋路。")
        }
        ("m3_027_river_awning", "DryOff") if m3_027_awning_dry_state_active(prior_states) => {
            Some("棚下你晾過的位置已經換成別人的濕包袱；再擠進去，只會把衣角重新蹭濕。")
        }
        ("m3_027_charcoal_brazier", "WarmHands")
            if m3_027_brazier_warm_state_active(prior_states) =>
        {
            Some("炭火盆邊的人看見你烘過手，便把下一個冷得發抖的候渡者推近火口。")
        }
        ("m3_027_waiting_line_chat", "TalkQueue")
            if m3_027_queue_talk_state_active(prior_states) =>
        {
            Some("人龍裡能問的船期和湯價你已聽過一輪；再追著問，只會讓前後的人避開你的眼神。")
        }
        ("m3_027_waiting_line_chat", "QueueFerry")
            if m3_027_queue_ferry_state_active(prior_states) =>
        {
            Some("你已在人龍折角裡，前後的人都按這條線往踏板等；再擠一次，只會把認住的位置弄亂。")
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
            Some("船戶把手掌壓在木牌上，示意你先回北岸候列；沒被人龍認住的人不能直接擠上踏板。")
        }
        ("m3_027_south_landing_rope", "StepAshore")
            if m3_027_landed_south_state_active(prior_states) =>
        {
            Some("你已踩過那道濕泥繩位，後面的人沿同一條痕上坡；再退回去只會把上岸線堵住。")
        }
        ("m3_027_south_landing_rope", "StepAshore")
            if !m3_027_board_ferry_state_active(prior_states) =>
        {
            Some("上岸繩那頭還沒輪到你，南岸的人只讓從踏板內側下來的人先踩穩。")
        }
        ("m3_027_warehouse_tally_board", "CheckCargo")
            if m3_027_warehouse_tally_state_active(prior_states) =>
        {
            Some("記牌人已把那副貨擔劃進倉棚短冊，尾款也結過；牌邊只剩下一輪貨號。")
        }
        ("m3_027_warehouse_tally_board", "CheckCargo")
            if !m3_027_cargo_help_state_active(prior_states) =>
        {
            Some("記牌人翻了翻短冊，沒有找到你送到踏板邊的貨擔痕；沒清過貨的人領不到這行尾款。")
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
            Some("新割過的蘆叢矮下一片，斷口旁另有水珠順著葉脈往泥裡落。")
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
            Some("已剪過的舊麻袋堆薄了一層，棚腳露出被踩濕的爛邊和散線。")
        }
        "m3_027_market_reed_buyer" => {
            Some("看貨人看見你背包裡露出的麻袋布邊，先把包貨單那行壓到秤邊。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("倉棚前場有人指向水尾寮渡頭市集棚腳，說那邊新剪出的麻袋布能先裹磨角木箱。")
        }
        _ => None,
    }
}

fn m3_027_rope_gathered_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_plank_wet_rope_coil" => {
            Some("已理過的濕繩圈少了一道，木樁內側只剩船戶重新壓緊的繩頭。")
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
            Some("你坐過的乾位已被包袱佔回去，凳腳旁還留著一點被鞋底蹭開的濕泥。")
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
            Some("你等過的桶側短線又接上一小截人，攤主把半碗淡湯留給沒錢買滿碗的候渡者。")
        }
        "m3_027_waiting_line_chat" => Some("人龍裡有人看見你拿過分湯，說話時把湯價壓低了一點。"),
        _ => None,
    }
}

fn m3_027_cargo_help_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_cargo_pole" => {
            Some("扛走一副貨擔讓泥地空出一道深痕，剩下的麻袋被水尾寮渡頭市集的人往棚腳收。")
        }
        "m3_027_plank_boarding_marker" => {
            Some("清出的貨擔痕停在踏板木牌旁，船戶不用再把上船線和搬貨線分開喊。")
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
            Some("已收過的繩圈被壓在木樁內側，船戶把那段窄位留著，等下一次船腹貼岸。")
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
            Some("你站過的棚下位置被濕包袱填上，舊帆布邊還滴著慢慢變細的水線。")
        }
        "m3_027_wet_bench" => Some("衣角乾了一些，你再看長凳時，濕痕不再像先前那樣貼著膝蓋。"),
        _ => None,
    }
}

fn m3_027_brazier_warm_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_charcoal_brazier" => {
            Some("炭火盆旁你讓出的半掌空位被下一個人補上，煙氣貼著他的袖口往外散。")
        }
        "m3_027_boat_rope" => Some("指節被炭火烘過後，再碰濕繩時不至於立刻僵住。"),
        _ => None,
    }
}

fn m3_027_queue_talk_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_waiting_line_chat" => {
            Some("你問過的那幾個候渡者換成低聲閒談，船期和湯價已在人龍裡傳開。")
        }
        "m3_027_boat_rope" => {
            Some("人龍裡聽來的船期讓你看船戶手勢時更快分清哪一次靠岸能上前搭手。")
        }
        _ => None,
    }
}

fn m3_027_queue_ferry_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_waiting_line_chat" => {
            Some("你排進去的那段人龍被後面的人補緊，候渡者也按同一條木牌線往踏板口慢慢挪。")
        }
        "m3_027_boat_rope" => {
            Some("船戶看見你已排進人龍，手掌在繩上停了一下，只等下一次踏板前清出縫。")
        }
        "m3_027_hot_soup_stall" => {
            Some("你在人龍裡的位置被人認住後，湯攤那側的人不再把你擠回桶邊短線。")
        }
        "m3_027_plank_boarding_marker" => {
            Some("船戶已在木牌內側認住你的人龍位置，踏板口留出一道等你上前的窄縫。")
        }
        _ => None,
    }
}

fn m3_027_reed_sold_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_market_reed_buyer" => {
            Some("你交出的蘆草已被抽去墊在貨籃角落，收牌人把點數完的鎂收進木牌下，價牌還沒有翻面。")
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
            Some("被收走的麻繩已用在踏板口那批箱包上，木樁內側只留給船戶的繩尾。")
        }
        _ => None,
    }
}

fn m3_027_board_ferry_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_plank_boarding_marker" => {
            Some("你進到踏板內側後，木牌前的人群收回半步，等下一個被船戶點到。")
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
            Some("踩出的泥痕被後面的人沿著走，南岸上坡那條線沒有再散開。")
        }
        "m3_027_warehouse_tally_board" => {
            Some("南岸上來的人流分出一股往倉棚前場，記牌人把短冊往棚門邊推近。")
        }
        _ => None,
    }
}

fn m3_027_warehouse_tally_state_echo(object_id: &str) -> Option<&'static str> {
    match object_id {
        "m3_027_warehouse_tally_board" => {
            Some("你對過的那行貨擔已被劃進短冊，牌角留下按過未乾的濕指印。")
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
    if !obj.id.starts_with("m3_")
        && (obj.id == "wt_guild_commission_board"
            || obj.id == "wt_hall_bulletin_board"
            || obj.id == "wt_cat_0794"
            || obj.name.contains("委託")
            || obj.name.contains("告示")
            || obj.name.contains("公佈欄"))
    {
        let mut custom = vec![
            "Look".to_string(),
            "Read".to_string(),
            "Use".to_string(),
            "Deliver".to_string(),
        ];
        if obj.id == "wt_hall_bulletin_board" || obj.name.contains("民務告示") {
            custom.push("DeliberateCivicPact".to_string());
        }
        let now_unix = crate::game::now_unix();
        let active_orders = crate::npc::causal_core::list_active_stranded_cargo_orders(now_unix);
        if !active_orders.is_empty() {
            custom.push("ClaimStrandedCargo".to_string());
            custom.push("DeliverStrandedCargo".to_string());
        }
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_harbor_insurance_desk" || obj.name.contains("海事承保") || obj.name.contains("海事保單") {
        let custom = vec!["Look".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_harbor_clockwise_ship" || obj.name.contains("順時針環島海船") || obj.name.contains("順時針環島海運商船") || obj.name.contains("遠洋順流商船") {
        let custom = vec!["Look".to_string(), "Use".to_string(), "BoardCoastalShip".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_field_crop_plot" || obj.name.contains("田壟作物") || obj.name.contains("當季田壟") {
        let custom = vec!["Look".to_string(), "Harvest".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_home_bamboo_mailbox" || obj.name.contains("竹編信箱") {
        let custom = vec!["Look".to_string(), "Read".to_string(), "Open".to_string(), "DispatchCourier".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_courier_post_desk" || obj.name.contains("信使櫃檯") {
        let custom = vec!["Look".to_string(), "Read".to_string(), "DispatchCourier".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_cat_0790" || obj.id == "wt_bank_ledger_drawer" || obj.name.contains("分格存摺櫃") || obj.name.contains("存摺") {
        let custom = vec!["Look".to_string(), "Read".to_string(), "DepositBank".to_string(), "WithdrawBank".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_home_jade_cabinet" || obj.name.contains("冰鑑櫃") || obj.name.contains("寒水玉保鮮石櫃") || obj.name.contains("寒水玉雙層保鮮石櫃") {
        let custom = vec!["Look".to_string(), "Open".to_string(), "StoreStash".to_string(), "RetrieveStash".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_coastal_angling_spot" || obj.id == "wt_cat_1006" || obj.name.contains("垂釣") || obj.name.contains("釣竿") {
        let custom = vec!["Look".to_string(), "AngleFish".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_lookout_telescope" || obj.id == "wt_cat_0733" || obj.name.contains("望遠鏡") {
        let custom = vec!["Look".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_field_camp_stove" || obj.id == "wt_cat_1010" || obj.name.contains("便攜爐") {
        let custom = vec!["Look".to_string(), "Cook".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_mill_") || obj.name.contains("水輪玄武磨") || obj.name.contains("篩粉斗") || obj.name.contains("儲糧斗") {
        let custom = vec!["Look".to_string(), "MillGrain".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_sawmill_") || obj.name.contains("排鋸架") || obj.name.contains("檢寸台") {
        let custom = vec!["Look".to_string(), "RefinePlank".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_stonecut_") || obj.name.contains("拉鋸") || obj.name.contains("寒水玉料架") || obj.name.contains("琢石盆") {
        let custom = vec!["Look".to_string(), "CutJade".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_cheese_") || obj.id == "wt_wool_spinning_wheel" || obj.id == "wt_farm_tool_iron_plow" || obj.name.contains("壓酪架") || obj.name.contains("熟成架") || obj.name.contains("精鐵犁") || obj.name.contains("手搖木紡車") {
        let custom = vec!["Look".to_string(), "PressCheese".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_mine_") || obj.id.starts_with("wt_trolley_") || obj.name.contains("洗礦槽") || obj.name.contains("重鐵篩") || obj.name.contains("重台車") || obj.name.contains("絞盤") {
        let custom = vec!["Look".to_string(), "HaulMineCart".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_smokehouse_") || obj.name.contains("紅陶灶") || obj.name.contains("冷燻") {
        let custom = vec!["Look".to_string(), "SmokeFish".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_lime_") || obj.id.starts_with("wt_kelp_") || obj.id == "wt_shipyard_dry_dock" || obj.name.contains("灰窯") || obj.name.contains("曬藻棚") || obj.name.contains("搓繩輪") || obj.name.contains("龍骨架") {
        let custom = vec!["Look".to_string(), "BurnLime".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_hunt_") || obj.id == "wt_clinic_herb_cabinet" || obj.name.contains("破甲弩") || obj.name.contains("鋼獸夾") || obj.name.contains("百草丸藥櫃") {
        let custom = vec!["Look".to_string(), "PrepareMiasmaGear".to_string(), "Use".to_string(), "Operate".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_dock_riverbank_raft" || obj.name.contains("平底竹筏") {
        let custom = vec!["Look".to_string(), "RowSwampRaft".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_dining_iron_cauldron" || obj.id == "wt_reg_plains_specialty" || obj.id == "wt_reg_mining_specialty" || obj.name.contains("燉肉釜") || obj.name.contains("陳年米酒甕") || obj.name.contains("烤番薯砂鍋") {
        let custom = vec!["Look".to_string(), "SeasonalBanquet".to_string(), "Taste".to_string(), "Use".to_string(), "AssistKitchen".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_inn_ebony_shell_chess" || obj.name.contains("烏木棋盤") {
        let custom = vec!["Look".to_string(), "PlayChess".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if matches!(
        action,
        "ChartAstrolabe"
            | "TestEngraving"
            | "SparPillar"
            | "SoakHotSpring"
            | "GroomAppearance"
            | "MendTravelGear"
            | "ReadWindVane"
            | "ApplyMoxibustion"
            | "HangVotivePlaque"
            | "TranscribeRareTome"
            | "AttendLecture"
            | "CalibrateClock"
            | "FitMortiseTenon"
            | "QuenchBlade"
            | "DyeIndigoCloth"
            | "FeedDraftBeast"
            | "BoardCoastalShip"
            | "InspectLandRegistry"
            | "AssistKitchen"
            | "RoastMountainTea"
            | "SunBooks"
            | "RoastHerbs"
            | "CollateInscriptions"
            | "GrindOpticLens"
            | "SelectDriedSeafood"
            | "DeliberateCivicPact"
            | "DrawSpringWell"
    ) {
        others = obj.sockets.iter().filter(|s| s.as_str() != "Look").cloned().collect();
    }
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
            .contains("你已在人龍")
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
            .contains("先回北岸候列")
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
            .contains("人龍位置")
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
            name: "候渡人龍".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "TalkQueue".to_string(),
                "QueueFerry".to_string(),
            ],
            responses: [("Look".to_string(), "候渡人龍折在踏板前。".to_string())]
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
        assert!(batch_text.contains("紮成 3 束「河霧蘆草」"));
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
        assert!(obj.responses.get("Look").unwrap().contains("黃銅桿儲墨硬筆"));
        assert!(obj.responses.get("Use").unwrap().contains("銥金筆尖觸及竹綿紙面"));
        assert!(obj.responses.get("Operate").unwrap().contains("輕旋黃銅筆桿檢視儲墨囊"));

        let by_name = resolve_room_object("any_room", "教案上的黃銅硬筆");
        assert!(by_name.is_some());
    }

    #[test]
    fn resolve_room_name_handles_grid_and_plain_names() {
        assert_eq!(resolve_room_name("白塔公議堂"), "白塔公議堂");
    }

    #[test]
    fn test_v03852_living_world_expansion_object_resolution_and_followup() {
        let mill = resolve_room_object("老堰頭水磨坊", "wt_mill_laoyan_flour_mill")
            .expect("wt_mill_laoyan_flour_mill must resolve");
        assert!(mill.sockets.contains(&"MillGrain".to_string()));

        let chess = resolve_room_object("雙星鎮茶寮客堂", "wt_inn_ebony_shell_chess")
            .expect("wt_inn_ebony_shell_chess must resolve");
        assert!(chess.sockets.contains(&"PlayChess".to_string()));

        let raft = resolve_room_object("水尾寮蘆葦碼頭", "wt_dock_riverbank_raft")
            .expect("wt_dock_riverbank_raft must resolve");
        assert!(raft.sockets.contains(&"RowSwampRaft".to_string()));

        // Verify object_followup_buttons keeps repeatable actions when clicked
        let (buttons_after_mill, _) = object_followup_buttons(&mill, "老堰頭水磨坊", "MillGrain", &[], &[]);
        assert!(buttons_after_mill.contains(&"MillGrain".to_string()), "Repeatable MillGrain action must not disappear after clicking");

        let angling_obj = crate::model::RoomObject {
            id: "wt_coastal_angling_spot".to_string(),
            name: "岸邊垂釣位".to_string(),
            sockets: vec!["Look".into(), "AngleFish".into(), "Use".into()],
            responses: std::collections::HashMap::new(),
            owner: String::new(),
            move_to_room_id: String::new(),
        };
        let (buttons_after_angle, _) = object_followup_buttons(&angling_obj, "黑石灣小碼頭", "AngleFish", &[], &[]);
        assert!(buttons_after_angle.contains(&"AngleFish".to_string()), "Repeatable AngleFish must not disappear after clicking");
    }

    #[test]
    fn test_v03853_three_pillars_actions_and_labels() {
        use super::action_label_zh;

        // 1. Verify action_label_zh mappings
        assert_eq!(action_label_zh("ChartAstrolabe"), "拓印星盤");
        assert_eq!(action_label_zh("TestEngraving"), "測紋校準");
        assert_eq!(action_label_zh("SparPillar"), "試勁拆招");
        assert_eq!(action_label_zh("SoakHotSpring"), "祛寒湯浴");
        assert_eq!(action_label_zh("GroomAppearance"), "修面整容");
        assert_eq!(action_label_zh("MendTravelGear"), "補綴行裝");
        assert_eq!(action_label_zh("ReadWindVane"), "觀測風候");
        assert_eq!(action_label_zh("ApplyMoxibustion"), "百草溫灸");
        assert_eq!(action_label_zh("HangVotivePlaque"), "懸掛銘牌");

        // 2. Verify resolve_room_object for all 9 objects
        let astrolabe = resolve_room_object("半嶺觀星台", "wt_cat_0734")
            .expect("wt_cat_0734 must resolve");
        assert_eq!(astrolabe.name, "渾天星軌銅儀");
        assert!(astrolabe.sockets.contains(&"ChartAstrolabe".to_string()));

        let engraving = resolve_room_object("烏石鼻匠坊", "wt_martial_force_stone")
            .expect("wt_martial_force_stone must resolve");
        assert_eq!(engraving.name, "測勁玄武碑");
        assert!(engraving.sockets.contains(&"TestEngraving".to_string()));

        let pillar = resolve_room_object("烏石鼻試武場", "wt_arena_testing_pillar")
            .expect("wt_arena_testing_pillar must resolve");
        assert_eq!(pillar.name, "玄武試勁柱");
        assert!(pillar.sockets.contains(&"SparPillar".to_string()));

        let hotspring = resolve_room_object("半嶺溫泉池", "wt_bath_jade_pool")
            .expect("wt_bath_jade_pool must resolve");
        assert_eq!(hotspring.name, "恆溫熱湯池");
        assert!(hotspring.sockets.contains(&"SoakHotSpring".to_string()));

        let barber = resolve_room_object("雙星鎮修面鋪", "wt_barber_folding_razor")
            .expect("wt_barber_folding_razor must resolve");
        assert_eq!(barber.name, "摺疊剃刀架");
        assert!(barber.sockets.contains(&"GroomAppearance".to_string()));

        let repair = resolve_room_object("霜林口街角", "wt_repair_umbrella_bench")
            .expect("wt_repair_umbrella_bench must resolve");
        assert_eq!(repair.name, "修傘木砧檯");
        assert!(repair.sockets.contains(&"MendTravelGear".to_string()));

        let wind_vane = resolve_room_object("半嶺測候所", "wt_observatory_anemometer")
            .expect("wt_observatory_anemometer must resolve");
        assert_eq!(wind_vane.name, "黃銅風向儀");
        assert!(wind_vane.sockets.contains(&"ReadWindVane".to_string()));

        let clinic = resolve_room_object("溫氏百草堂", "wt_clinic_acupoint_bed")
            .expect("wt_clinic_acupoint_bed must resolve");
        assert_eq!(clinic.name, "竹席診脈榻");
        assert!(clinic.sockets.contains(&"ApplyMoxibustion".to_string()));

        let votive = resolve_room_object("北嶺山道古杉前", "wt_obj_mountain_milestone")
            .expect("wt_obj_mountain_milestone must resolve");
        assert_eq!(votive.name, "山道指路碑");
        assert!(votive.sockets.contains(&"HangVotivePlaque".to_string()));

        // 3. Verify object_followup_buttons keeps repeatable action buttons for all 9 actions
        let (buttons_astrolabe, _) = object_followup_buttons(&astrolabe, "半嶺觀星台", "ChartAstrolabe", &[], &[]);
        assert!(buttons_astrolabe.contains(&"ChartAstrolabe".to_string()));

        let (buttons_engraving, _) = object_followup_buttons(&engraving, "烏石鼻匠坊", "TestEngraving", &[], &[]);
        assert!(buttons_engraving.contains(&"TestEngraving".to_string()));

        let (buttons_pillar, _) = object_followup_buttons(&pillar, "烏石鼻試武場", "SparPillar", &[], &[]);
        assert!(buttons_pillar.contains(&"SparPillar".to_string()));

        let (buttons_hotspring, _) = object_followup_buttons(&hotspring, "半嶺溫泉池", "SoakHotSpring", &[], &[]);
        assert!(buttons_hotspring.contains(&"SoakHotSpring".to_string()));

        let (buttons_barber, _) = object_followup_buttons(&barber, "雙星鎮修面鋪", "GroomAppearance", &[], &[]);
        assert!(buttons_barber.contains(&"GroomAppearance".to_string()));

        let (buttons_repair, _) = object_followup_buttons(&repair, "霜林口街角", "MendTravelGear", &[], &[]);
        assert!(buttons_repair.contains(&"MendTravelGear".to_string()));

        let (buttons_wind_vane, _) = object_followup_buttons(&wind_vane, "半嶺測候所", "ReadWindVane", &[], &[]);
        assert!(buttons_wind_vane.contains(&"ReadWindVane".to_string()));

        let (buttons_clinic, _) = object_followup_buttons(&clinic, "溫氏百草堂", "ApplyMoxibustion", &[], &[]);
        assert!(buttons_clinic.contains(&"ApplyMoxibustion".to_string()));

        let (buttons_votive, _) = object_followup_buttons(&votive, "北嶺山道古杉前", "HangVotivePlaque", &[], &[]);
        assert!(buttons_votive.contains(&"HangVotivePlaque".to_string()));
    }
    #[test]
    fn test_v03854_three_pillar_actions_and_labels() {
        use super::{action_label_zh, normalize_action_key};

        // 1. Verify action_label_zh and normalize_action_key mappings
        let actions = [
            ("TranscribeRareTome", "鈔錄孤本"),
            ("AttendLecture", "旁聽策論"),
            ("CalibrateClock", "校時報刻"),
            ("FitMortiseTenon", "榫卯校修"),
            ("QuenchBlade", "精鋼回火"),
            ("DyeIndigoCloth", "藍染浸布"),
            ("FeedDraftBeast", "飼餵草獸"),
            ("BoardCoastalShip", "登船遠航"),
            ("InspectLandRegistry", "查閱地籍"),
        ];
        for (key, zh) in actions {
            assert_eq!(action_label_zh(key), zh);
            assert_eq!(action_label_zh(zh), zh);
            assert_eq!(normalize_action_key(key), key);
            assert_eq!(normalize_action_key(zh), key);
        }

        // 2. Verify resolve_room_object for all 8 fallback objects
        let shelf = resolve_room_object("藏書閣", "wt_book_printing_shelf")
            .expect("wt_book_printing_shelf must resolve");
        assert_eq!(shelf.name, "風物誌書架");
        assert!(shelf.sockets.contains(&"TranscribeRareTome".to_string()));

        let pen = resolve_room_object("講堂", "wt_academy_fountain_pen")
            .expect("wt_academy_fountain_pen must resolve");
        assert_eq!(pen.name, "黃銅儲墨筆");
        assert!(pen.sockets.contains(&"AttendLecture".to_string()));

        let bell = resolve_room_object("鐘樓", "wt_clocktower_bronze_bell")
            .expect("wt_clocktower_bronze_bell must resolve");
        assert_eq!(bell.name, "報時青銅鐘");
        assert!(bell.sockets.contains(&"CalibrateClock".to_string()));

        let bench = resolve_room_object("工坊", "wt_wood_planing_bench")
            .expect("wt_wood_planing_bench must resolve");
        assert_eq!(bench.name, "刨木工作台");
        assert!(bench.sockets.contains(&"FitMortiseTenon".to_string()));

        let trough = resolve_room_object("鐵匠鋪", "wt_forge_quench_trough")
            .expect("wt_forge_quench_trough must resolve");
        assert_eq!(trough.name, "精鋼淬火槽");
        assert!(trough.sockets.contains(&"QuenchBlade".to_string()));

        let rack = resolve_room_object("布莊", "wt_cloth_garment_rack")
            .expect("wt_cloth_garment_rack must resolve");
        assert_eq!(rack.name, "防雨長衫架");
        assert!(rack.sockets.contains(&"DyeIndigoCloth".to_string()));

        let water = resolve_room_object("車馬行", "wt_caravan_water_trough")
            .expect("wt_caravan_water_trough must resolve");
        assert_eq!(water.name, "青石飲馬槽");
        assert!(water.sockets.contains(&"FeedDraftBeast".to_string()));

        let cadastral = resolve_room_object("房地契局", "wt_deed_cadastral_map_desk")
            .expect("wt_deed_cadastral_map_desk must resolve");
        assert_eq!(cadastral.name, "四坊地籍案");
        assert!(cadastral.sockets.contains(&"InspectLandRegistry".to_string()));

        // 3. Verify object_followup_buttons keeps repeatable action buttons for all 9 actions
        let (b_shelf, _) = object_followup_buttons(&shelf, "藏書閣", "TranscribeRareTome", &[], &[]);
        assert!(b_shelf.contains(&"TranscribeRareTome".to_string()));

        let (b_pen, _) = object_followup_buttons(&pen, "講堂", "AttendLecture", &[], &[]);
        assert!(b_pen.contains(&"AttendLecture".to_string()));

        let (b_bell, _) = object_followup_buttons(&bell, "鐘樓", "CalibrateClock", &[], &[]);
        assert!(b_bell.contains(&"CalibrateClock".to_string()));

        let (b_bench, _) = object_followup_buttons(&bench, "工坊", "FitMortiseTenon", &[], &[]);
        assert!(b_bench.contains(&"FitMortiseTenon".to_string()));

        let (b_trough, _) = object_followup_buttons(&trough, "鐵匠鋪", "QuenchBlade", &[], &[]);
        assert!(b_trough.contains(&"QuenchBlade".to_string()));

        let (b_rack, _) = object_followup_buttons(&rack, "布莊", "DyeIndigoCloth", &[], &[]);
        assert!(b_rack.contains(&"DyeIndigoCloth".to_string()));

        let (b_water, _) = object_followup_buttons(&water, "車馬行", "FeedDraftBeast", &[], &[]);
        assert!(b_water.contains(&"FeedDraftBeast".to_string()));

        let (b_cadastral, _) = object_followup_buttons(&cadastral, "房地契局", "InspectLandRegistry", &[], &[]);
        assert!(b_cadastral.contains(&"InspectLandRegistry".to_string()));

        let ship = resolve_room_object("風舶角碼頭", "wt_harbor_clockwise_ship")
            .expect("wt_harbor_clockwise_ship must resolve");
        assert!(ship.sockets.contains(&"BoardCoastalShip".to_string()));
        let (b_ship, _) = object_followup_buttons(&ship, "風舶角碼頭", "BoardCoastalShip", &[], &[]);
        assert!(b_ship.contains(&"BoardCoastalShip".to_string()));
        let gangplank = resolve_room_object("風舶角碼頭", "wt_dock_cargo_gangplank")
            .expect("wt_dock_cargo_gangplank must resolve");
        assert!(gangplank.sockets.contains(&"BoardCoastalShip".to_string()));
    }

    #[test]
    fn test_v03855_three_pillar_actions_and_contracts() {
        use super::{action_label_zh, normalize_action_key};

        // 1. Verify action_label_zh and normalize_action_key mappings for all 9 v0.38.55 actions
        let actions = [
            ("AssistKitchen", "幫廚備膳"),
            ("RoastMountainTea", "焙茶品茗"),
            ("SunBooks", "曝書防蠹"),
            ("RoastHerbs", "炮製藥材"),
            ("CollateInscriptions", "校勘碑帖"),
            ("GrindOpticLens", "研磨鏡片"),
            ("SelectDriedSeafood", "揀選海味"),
            ("DeliberateCivicPact", "參議鄉約"),
            ("DrawSpringWell", "汲水潤嗓"),
        ];
        for (key, zh) in actions {
            assert_eq!(action_label_zh(key), zh);
            assert_eq!(action_label_zh(zh), zh);
            assert_eq!(normalize_action_key(key), key);
            assert_eq!(normalize_action_key(zh), key);
        }

        // 2. Verify resolve_room_object for fallback objects
        let cauldron = resolve_room_object("食堂", "wt_dining_iron_cauldron")
            .expect("wt_dining_iron_cauldron must resolve");
        assert!(cauldron.sockets.contains(&"AssistKitchen".to_string()));

        let milk_stall = resolve_room_object("食堂", "wt_dining_rice_milk_stall")
            .expect("wt_dining_rice_milk_stall must resolve");
        assert!(milk_stall.sockets.contains(&"AssistKitchen".to_string()));

        let tea_stove = resolve_room_object("旅社茶屋", "wt_inn_milk_tea_stove")
            .expect("wt_inn_milk_tea_stove must resolve");
        assert!(tea_stove.sockets.contains(&"RoastMountainTea".to_string()));

        let book_shelf = resolve_room_object("藏書閣", "wt_book_printing_shelf")
            .expect("wt_book_printing_shelf must resolve");
        assert!(book_shelf.sockets.contains(&"SunBooks".to_string()));

        let herb_kiln = resolve_room_object("百草烘藥房", "wt_herb_drying_kiln")
            .expect("wt_herb_drying_kiln must resolve");
        assert!(herb_kiln.sockets.contains(&"RoastHerbs".to_string()));

        let herb_sieve = resolve_room_object("百草烘藥房", "wt_herb_bamboo_sieve")
            .expect("wt_herb_bamboo_sieve must resolve");
        assert!(herb_sieve.sockets.contains(&"RoastHerbs".to_string()));

        let desk_ink = resolve_room_object("校勘齋", "wt_study_desk_inkstone")
            .expect("wt_study_desk_inkstone must resolve");
        assert!(desk_ink.sockets.contains(&"CollateInscriptions".to_string()));
        assert!(desk_ink.sockets.contains(&"SunBooks".to_string()));

        let stele_cab = resolve_room_object("校勘齋", "wt_obj_stone_rubbing_cabinet")
            .expect("wt_obj_stone_rubbing_cabinet must resolve");
        assert!(stele_cab.sockets.contains(&"CollateInscriptions".to_string()));

        let tele_rack = resolve_room_object("觀星台", "wt_obs_brass_telescope_rack")
            .expect("wt_obs_brass_telescope_rack must resolve");
        assert!(tele_rack.sockets.contains(&"GrindOpticLens".to_string()));

        let map_table = resolve_room_object("閱覽室", "wt_book_map_reading_table")
            .expect("wt_book_map_reading_table must resolve");
        assert!(map_table.sockets.contains(&"GrindOpticLens".to_string()));

        let market_stand = resolve_room_object("海味行", "wt_market_produce_stand")
            .expect("wt_market_produce_stand must resolve");
        assert!(market_stand.sockets.contains(&"SelectDriedSeafood".to_string()));

        let market_scale = resolve_room_object("海味行", "wt_market_scale_counter")
            .expect("wt_market_scale_counter must resolve");
        assert!(market_scale.sockets.contains(&"SelectDriedSeafood".to_string()));

        let council_table = resolve_room_object("公議堂", "wt_hall_council_table")
            .expect("wt_hall_council_table must resolve");
        assert!(council_table.sockets.contains(&"DeliberateCivicPact".to_string()));

        let bulletin = resolve_room_object("公議堂", "wt_hall_bulletin_board")
            .expect("wt_hall_bulletin_board must resolve");
        assert!(bulletin.sockets.contains(&"DeliberateCivicPact".to_string()));

        let river_steps = resolve_room_object("水岸北街", "wt_river_stone_steps")
            .expect("wt_river_stone_steps must resolve");
        assert!(river_steps.sockets.contains(&"DrawSpringWell".to_string()));

        let stone_well = resolve_room_object("巷弄天井", "wt_alley_stone_well")
            .expect("wt_alley_stone_well must resolve");
        assert!(stone_well.sockets.contains(&"DrawSpringWell".to_string()));

        // 3. Verify object_followup_buttons keeps repeatable action buttons for all 9 actions
        let (b_cauldron, _) = object_followup_buttons(&cauldron, "食堂", "AssistKitchen", &[], &[]);
        assert!(b_cauldron.contains(&"AssistKitchen".to_string()));

        let (b_tea, _) = object_followup_buttons(&tea_stove, "旅社茶屋", "RoastMountainTea", &[], &[]);
        assert!(b_tea.contains(&"RoastMountainTea".to_string()));

        let (b_shelf, _) = object_followup_buttons(&book_shelf, "藏書閣", "SunBooks", &[], &[]);
        assert!(b_shelf.contains(&"SunBooks".to_string()));

        let (b_herb, _) = object_followup_buttons(&herb_kiln, "百草烘藥房", "RoastHerbs", &[], &[]);
        assert!(b_herb.contains(&"RoastHerbs".to_string()));

        let (b_desk, _) = object_followup_buttons(&desk_ink, "校勘齋", "CollateInscriptions", &[], &[]);
        assert!(b_desk.contains(&"CollateInscriptions".to_string()));

        let (b_tele, _) = object_followup_buttons(&tele_rack, "觀星台", "GrindOpticLens", &[], &[]);
        assert!(b_tele.contains(&"GrindOpticLens".to_string()));

        let (b_market, _) = object_followup_buttons(&market_stand, "海味行", "SelectDriedSeafood", &[], &[]);
        assert!(b_market.contains(&"SelectDriedSeafood".to_string()));

        let (b_council, _) = object_followup_buttons(&council_table, "公議堂", "DeliberateCivicPact", &[], &[]);
        assert!(b_council.contains(&"DeliberateCivicPact".to_string()));

        let (b_river, _) = object_followup_buttons(&river_steps, "水岸北街", "DrawSpringWell", &[], &[]);
        assert!(b_river.contains(&"DrawSpringWell".to_string()));

        // Also verify bulletin board followup contains DeliberateCivicPact
        let (b_bulletin, _) = object_followup_buttons(&bulletin, "公議堂", "DeliberateCivicPact", &[], &[]);
        assert!(b_bulletin.contains(&"DeliberateCivicPact".to_string()));

        // 4. Verify no design jargon in strings
        let prohibited_terms = ["SFC", "Stock-Flow", "RelationalHook", "WorldRippleFact", "Chebyshev", "\u{525b}\u{624d}", "\u{525b}\u{525b}", "\u{65b9}\u{624d}"];
        for &(key, zh) in &actions {
            for term in &prohibited_terms {
                assert!(!key.contains(term), "Key {} contains jargon {}", key, term);
                assert!(!zh.contains(term), "Label {} contains jargon {}", zh, term);
            }
        }
    }

    #[test]
    fn test_resolve_grid_scene_object_venue_counter_and_event_obj() {
        let coord_canteen = crate::grid::SquareCoord::new(990, 992);
        let canteen_room_id = format!("grid:{}:{}", coord_canteen.x, coord_canteen.y);
        let cell_canteen = crate::grid::GridCell::new(coord_canteen, crate::grid::Terrain::Workshop, "三水大食堂");

        let coord_mill = crate::grid::SquareCoord::new(991, 992);
        let mill_room_id = format!("grid:{}:{}", coord_mill.x, coord_mill.y);
        let mut cell_mill = crate::grid::GridCell::new(coord_mill, crate::grid::Terrain::Workshop, "老堰頭・水輪石磨工坊");
        cell_mill.tags.push("石磨".to_string());
        cell_mill.tags.push("水輪".to_string());

        let mut test_grid = crate::grid::SquareGrid::new();
        test_grid.insert(cell_canteen);
        test_grid.insert(cell_mill);
        crate::server::grid_manager::set_runtime_square_grid_for_testing(test_grid);

        // 1. Resolve wt_venue_counter on 三水大食堂 (has_counter_restricted_zone == true)
        let counter_obj = resolve_grid_scene_object(&canteen_room_id, "wt_venue_counter")
            .expect("wt_venue_counter must resolve for venue with counter restricted zone");
        assert_eq!(counter_obj.id, "wt_venue_counter");
        assert!(counter_obj.name.contains("當值櫃檯"));
        assert!(counter_obj.sockets.contains(&"Look".to_string()));
        assert!(counter_obj.sockets.contains(&"Inspect".to_string()));
        assert!(counter_obj.sockets.contains(&"Operate".to_string()));

        let inspect_res = counter_obj.responses.get("Inspect").cloned().unwrap_or_default();
        let operate_res = counter_obj.responses.get("Operate").cloned().unwrap_or_default();
        assert!(inspect_res.contains("事件包"));
        assert!(inspect_res.contains("待辦簿冊"));
        assert!(operate_res.contains("示意協辦"));

        // 2. Resolve wt_event_obj_ws-001 by ID directly on 老堰頭・水輪石磨工坊
        let event_obj = resolve_grid_scene_object(&mill_room_id, "wt_event_obj_ws-001")
            .expect("wt_event_obj_ws-001 must resolve by event object id");
        assert!(event_obj.id.starts_with("wt_event_obj_ws-"));
        assert!(!event_obj.name.is_empty());
        assert!(event_obj.sockets.contains(&"Look".to_string()));
        assert!(event_obj.sockets.contains(&"Inspect".to_string()));
        assert!(event_obj.sockets.contains(&"Operate".to_string()));
        assert!(event_obj.sockets.contains(&"Use".to_string()));

        let ev_look = event_obj.responses.get("Look").cloned().unwrap_or_default();
        assert!(ev_look.contains("【事件包・"));
    }
}
