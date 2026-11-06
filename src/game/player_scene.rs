//! 玩家層場景措辭：只描述玩家能看見的公共身份與微動作。

use std::collections::HashMap;

use crate::model::RoomObject;

include!("player_scene/scene_constants_1.rs");
include!("player_scene/scene_constants_2.rs");
include!("player_scene/room_objects_1.rs");
include!("player_scene/room_objects_2.rs");
include!("player_scene/room_objects_3.rs");
include!("player_scene/room_objects_4.rs");
include!("player_scene/room_objects_5.rs");
include!("player_scene/room_objects_6.rs");
include!("player_scene/observation_1.rs");
include!("player_scene/observation_2.rs");
include!("player_scene/vendors.rs");

pub fn white_tower_peninsula_objects_for_room(room_name: &str) -> Vec<RoomObject> {
    let clean_buf = room_name.replace("文曜・文曜", "文曜・").replace("文曜文曜", "文曜");
    let room_name = clean_buf.as_str();
    const WILDERNESS_CELL_MARKERS: &[&str] = &[
        "・草原", "・草甸", "・密林", "・山脊", "・幽沼", "・泥澤", "・冰峰", "・雪巔", "・雪林",
        "・海灣", "・深水", "・沙礫", "・巨岩", "・楓林", "・清流", "・草場", "・山隘", "・海岬",
        "・古徑", "・岩岸", "・淺灣", "・礫灣", "・溪谷", "・雲杉", "・林道", "・泥口", "・深潭",
        "・礦谷", "・礦坡", "・峭壁",
    ];
    if WILDERNESS_CELL_MARKERS.iter().any(|m| room_name.ends_with(m)) {
        return Vec::new();
    }

    if room_name == "天然港灣跨海長橋" {
        return vec![make_peninsula_object(
            "wt_bridge_harbor_causeway",
            "防風航標燈",
            &["Look", "Rest", "Operate"],
            &[
                ("Look", "跨海石橋橫臥於碧波海灣之上，厚實防風石欄抵禦著長風與浪沫，石欄轉折處設有旋蓋夜光貝母航標燈，指引往來舟楫與行旅。"),
                ("Rest", "你立於跨海長橋迎風的石欄旁稍作停歇，浩蕩海風迎面拂來，海天相接處波濤湧動，氣象萬千。"),
                ("Operate", "你伸手輕撫航標燈沉厚的銅座與夜光貝罩，防風樞紐運轉自如，散發著穿透夜霧的柔和光暈。"),
            ],
        )];
    }

    let is_trunk_road = !room_name.contains('・')
        && (room_name.ends_with("大道")
            || room_name.ends_with("幹道")
            || room_name.ends_with("山道")
            || room_name.ends_with("險徑")
            || room_name.ends_with("山徑")
            || room_name.ends_with("古徑")
            || room_name.ends_with("牧道")
            || room_name.ends_with("礦道")
            || room_name.ends_with("棧道")
            || room_name.ends_with("軌道")
            || room_name.ends_with("沿海路")
            || room_name.ends_with("海岬路")
            || room_name.ends_with("港灣道")
            || room_name.ends_with("海濱道")
            || room_name.ends_with("潮音道")
            || room_name.ends_with("巡哨路")
            || room_name.ends_with("關隘道")
            || room_name.ends_with("北坡道")
            || room_name.ends_with("天路")
            || room_name.ends_with("冰岸道")
            || room_name.ends_with("橫斷道")
            || room_name.ends_with("林徑"));
    if is_trunk_road {
        return vec![make_peninsula_object(
            "wt_trunk_road_milestone",
            "指路石碑",
            &["Look", "Read", "Rest"],
            &[
                ("Look", "大道旁矗立著風化古樸的四方指路石碑，基座是一方供往來旅客卸擔歇腳的厚重石礎，風塵僕僕中透著安穩。"),
                ("Read", "你俯身撥開碑面上附著的青苔辨讀刻字，半島城鎮間的里數與主幹道走向清晰可辨。"),
                ("Rest", "你在石碑旁的歇腳石礎上落座調勻氣息，就著清涼微風飲下一口水，片刻之後精神重新振奮。"),
            ],
        )];
    }

    let mut base_objs = peninsula_objects_chunk_1(room_name)
        .or_else(|| peninsula_objects_chunk_2(room_name))
        .or_else(|| peninsula_objects_chunk_3(room_name))
        .or_else(|| peninsula_objects_chunk_4(room_name))
        .or_else(|| peninsula_objects_chunk_5(room_name))
        .or_else(|| peninsula_objects_chunk_6(room_name))
        .unwrap_or_default();

    if !base_objs.is_empty() {
        if base_objs.len() < 3 {
            for cat_obj in super::peninsula_catalog::catalog_objects_for_room(room_name) {
                if !base_objs.iter().any(|o| o.name == cat_obj.name || o.id == cat_obj.id) {
                    base_objs.push(cat_obj);
                    if base_objs.len() >= 3 {
                        break;
                    }
                }
            }
        }
        if base_objs.len() < 3 {
            if let Some(regional_obj) = regional_specialty_object_for_room(room_name) {
                if !base_objs.iter().any(|o| o.name == regional_obj.name || o.id == regional_obj.id) {
                    base_objs.push(regional_obj);
                }
            }
        }
    }
    base_objs.truncate(3);
    base_objs
}

include!("player_scene/tests.rs");
