//! `do_action` — Look 分支：打量目標，輸出外觀敘述。

use crate::db;
use crate::entity::Character;
use crate::event::{self, types};
use crate::game::{
    final_map_observation_template_for_room, is_final_map_public_role, public_person_label_for_room,
};

use super::super::handler::WsConnection;
use super::super::protocol::ActionResultMsg;
use super::display_target_name;

pub(super) fn do_entity_look(
    conn: &WsConnection,
    pid: &str,
    now: i64,
    target_id: &str,
    target: &Character,
) {
    let room_name = target_grid_room_name(target);
    let target_name = player_visible_target_name(pid, target, &room_name);
    let narrative = build_look_narrative(target, &target_name, &room_name);
    let actions = followup_actions(pid, target);
    let _ = event::append(now, pid, types::OBSERVED, target_id);
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "Look".into(),
        target_id: target.id.clone(),
        target_name,
        narrative,
        success: true,
        actions,
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
    });
}

fn build_look_narrative(target: &Character, name: &str, room_name: &str) -> String {
    let pronoun = match target.gender {
        Some(crate::entity::Gender::F) => "她",
        _ => "他",
    };
    let physique = match target.vit {
        v if v >= 20 => "體格異常魁梧",
        v if v >= 15 => "體格健壯",
        v if v >= 10 => "身材勻稱",
        _ => "身形消瘦",
    };
    let agility = match target.dex {
        v if v >= 20 => "舉止間透著驚人的敏捷",
        v if v >= 15 => "動作輕靈",
        v if v >= 10 => "步履平穩",
        _ => "行動略顯遲緩",
    };
    let qi_presence = match target.qi {
        v if v >= 20 => "，周身隱隱有氣勁流轉",
        v if v >= 15 => "，氣息沉穩",
        v if v >= 10 => "",
        _ => "，氣息微弱",
    };
    let mut desc = format!("你仔細打量了【{name}】。{pronoun}{physique}，{agility}{qi_presence}。");
    if !target.equipment_slots.is_empty() {
        let names = db::get_item_names(&target.equipment_slots);
        let pieces: Vec<String> = names.into_values().take(3).collect();
        if !pieces.is_empty() {
            desc.push_str("身上穿戴著");
            desc.push_str(&pieces.join("、"));
            desc.push('。');
        }
    }
    if !target.current_activity.is_empty() {
        desc.push_str(&format!(
            "{pronoun}目前{act}。",
            act = target.current_activity
        ));
    } else if !is_named_to_player(name, target)
        && let Some(line) = scene_observation_line(room_name, pronoun)
    {
        desc.push_str(&line);
    }
    desc
}

fn player_visible_target_name(player_id: &str, target: &Character, room_name: &str) -> String {
    let acq = db::get_acquaintance(player_id, &target.id).ok().flatten();
    if acq.as_ref().map(|a| a.named).unwrap_or(false) {
        return display_target_name(target);
    }

    if acq.as_ref().map(|a| a.observed).unwrap_or(false) {
        return observed_label_for_room(&display_target_name(target), room_name);
    }

    "行人".to_string()
}

fn followup_actions(player_id: &str, target: &Character) -> Vec<String> {
    let named = db::get_acquaintance(player_id, &target.id)
        .ok()
        .flatten()
        .map(|a| a.named)
        .unwrap_or(false);
    if named {
        ["Talk", "Borrow", "Trade", "Subdue", "Slay"]
            .into_iter()
            .map(str::to_string)
            .collect()
    } else {
        vec!["Talk".to_string()]
    }
}

fn observed_label_for_room(raw_name: &str, room_name: &str) -> String {
    let title = raw_name.split('|').next().unwrap_or(raw_name).trim();
    if !title.is_empty() && !looks_like_private_name(title) {
        return title.to_string();
    }
    public_person_label_for_room(room_name)
        .unwrap_or("行人")
        .to_string()
}

fn target_grid_room_name(target: &Character) -> String {
    let (Some(x), Some(y)) = (target.grid_x, target.grid_y) else {
        return String::new();
    };
    crate::server::grid_manager::get_runtime_square_grid()
        .and_then(|g| {
            g.get(crate::grid::SquareCoord::new(x, y)).map(|c| {
                if c.name.trim().is_empty() {
                    crate::grid::terrain_name_zh(c.terrain).to_string()
                } else {
                    c.name.clone()
                }
            })
        })
        .unwrap_or_default()
}

fn scene_observation_line(room_name: &str, pronoun: &str) -> Option<String> {
    if let Some(template) = final_map_observation_template_for_room(room_name) {
        Some(template.replace("{pronoun}", pronoun))
    } else if room_name.contains("北岸") || room_name.contains("等候") {
        Some(format!(
            "{pronoun}沒有主動搭話，只不時望向渡船踏板，又把身邊的包袱往腳邊收了收。"
        ))
    } else if room_name.contains("短市") {
        Some(format!(
            "{pronoun}在價牌前停了停，指尖掂著一只小包短貨，像是在估量要不要趁船來前換些東西。"
        ))
    } else if room_name.contains("歇腳") {
        Some(format!(
            "{pronoun}靠近長凳坐著，茶碗握在手裡，包袱收在腳邊，聽見船繩聲便抬頭看一眼。"
        ))
    } else if room_name.contains("踏板") {
        Some(format!(
            "{pronoun}一手扶著濕木扶手，等船身在水流裡穩下來，腳尖一直避著踏板邊緣的積水。"
        ))
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some(format!(
            "{pronoun}把包袱提高些，鞋底還沾著濕泥，走兩步便回頭看一眼渡船是否還靠在岸邊。"
        ))
    } else if room_name.contains("東南船塢口") {
        Some(format!(
            "{pronoun}把半修的船板靠回木架旁，手上還留著瀝青和木屑的氣味。"
        ))
    } else if room_name.contains("修船架") {
        Some(format!(
            "{pronoun}沿著船肋敲了幾下木楔，又側耳聽船身裡回出的空響。"
        ))
    } else if room_name.contains("補給倉") {
        Some(format!(
            "{pronoun}把淡水桶和乾糧箱重新分成幾堆，低聲核對哪艘船先裝。"
        ))
    } else if room_name.contains("碼頭木階") {
        Some(format!(
            "{pronoun}扶著濕木階等船身晃穩，腳邊的水痕被來往腳步踩亮。"
        ))
    } else if room_name.contains("帆布曬場") {
        Some(format!(
            "{pronoun}用膝蓋壓住被風吹起的舊帆，沿著裂口把針線慢慢收緊。"
        ))
    } else if room_name.contains("船戶街口") {
        Some(format!(
            "{pronoun}在招工牌下比了比工錢，又朝飯攤旁等活的人低聲交代船期。"
        ))
    } else if room_name.contains("外貿碼頭") {
        Some(format!(
            "{pronoun}扶住剛落下的外地貨箱，抬頭看港旗和海關前棚那邊的手勢。"
        ))
    } else if room_name.contains("海關前棚") {
        Some(format!(
            "{pronoun}按著濕貨單和封條，等翻譯桌那邊把貨名重新說清。"
        ))
    } else if room_name.contains("商館街口") {
        Some(format!(
            "{pronoun}把貨樣匣打開一角，低聲把剛問到的價碼送往匯兌小屋。"
        ))
    } else if room_name.contains("遠洋倉門") {
        Some(format!(
            "{pronoun}把異地木箱推到雨布內側，分出已查和待查的箱包。"
        ))
    } else if room_name.contains("翻譯桌") {
        Some(format!(
            "{pronoun}指著合約上並排的兩種文字，把陌生貨名轉成在地話。"
        ))
    } else if room_name.contains("匯兌小屋") {
        Some(format!(
            "{pronoun}撥了撥銅秤，將錢票和商館街口報來的價碼寫入賬冊。"
        ))
    } else if room_name.contains("北灣避風碼頭") {
        Some(format!(
            "{pronoun}把結霜纜繩重新繞上木樁，又抬頭看避風船是否貼穩碼頭。"
        ))
    } else if room_name.contains("港灣貨棚") {
        Some(format!(
            "{pronoun}把濕魚籃推到鹽箱旁，鞋底在棚門下拖出一道水痕。"
        ))
    } else if room_name.contains("漁網曬場") {
        Some(format!(
            "{pronoun}沿著濕網破口慢慢穿線，膝邊的木架被海風吹得輕響。"
        ))
    } else if room_name.contains("冷霧魚市") {
        Some(format!(
            "{pronoun}掀開濕布讓買貨人看魚色，又把木秤往身邊拉近。"
        ))
    } else if room_name.contains("船戶屋排") {
        Some(format!(
            "{pronoun}在排班板前換下一枚濕木牌，火盆煙氣貼著袖口散開。"
        ))
    } else if room_name.contains("北灣守望石階") {
        Some(format!(
            "{pronoun}扶著濕石欄看向港外海面，指尖停在警鈴繩旁。"
        ))
    } else if room_name.contains("礦坑外棚") {
        Some(format!(
            "{pronoun}把沾泥的鎬靠上木架，抬頭看坑口吊燈是否還亮著。"
        ))
    } else if room_name.contains("洗礦溝") {
        Some(format!(
            "{pronoun}把石籃壓進冷水槽，從混濁水裡分出還能送往粗料倉的礦料。"
        ))
    } else if room_name.contains("粗料倉") {
        Some(format!(
            "{pronoun}在濕礦袋旁補上一道炭線，讓坑口、重量和去向都留在木牌上。"
        ))
    } else if room_name.contains("工具修棚") {
        Some(format!(
            "{pronoun}把鬆掉的鐵圈敲回車輪邊，火星在裂柄鎬旁短短一亮。"
        ))
    } else if room_name.contains("礦渣坡") {
        Some(format!(
            "{pronoun}壓住推車把手，把廢料倒在安全線內，避開新滑坡痕。"
        ))
    } else if room_name.contains("舊道裝車點") {
        Some(format!(
            "{pronoun}把繩索掛上礦車車架，又伸手抹開稱重木牌上的灰粉。"
        ))
    } else if room_name.contains("船戶作業點") || room_name.contains("作業") {
        Some(format!(
            "{pronoun}低頭撥著纜繩與木楔，偶爾抬眼看水勢，像是在等下一個靠船的手勢。"
        ))
    } else if room_name.contains("倉棚") {
        Some(format!(
            "{pronoun}在麻袋與木箱旁停下，手掌拍去袖口的泥點，視線一直留意著棚門和河邊的動靜。"
        ))
    } else {
        None
    }
}

fn is_named_to_player(visible_name: &str, target: &Character) -> bool {
    visible_name == display_target_name(target)
}

fn looks_like_private_name(s: &str) -> bool {
    let mut count = 0usize;
    for ch in s.chars() {
        if !('\u{4e00}'..='\u{9fff}').contains(&ch) {
            return false;
        }
        count += 1;
    }
    (2..=4).contains(&count) && !looks_like_public_role(s)
}

fn looks_like_public_role(s: &str) -> bool {
    if is_final_map_public_role(s) {
        return true;
    }
    let roles = [
        "船戶",
        "渡工",
        "挑夫",
        "旅人",
        "商人",
        "攤販",
        "茶客",
        "行人",
        "獵戶",
        "道士",
        "守衛",
        "工匠",
        "學徒",
        "修船者",
        "船匠",
        "修船工",
        "倉管",
        "船客",
        "補帆人",
        "船行人",
        "船員",
        "稅吏",
        "海關吏",
        "商館人",
        "翻譯",
        "兌票人",
        "搬魚人",
        "補網人",
        "魚販",
        "船戶家屬",
        "守望人",
        "礦工",
        "洗礦工",
        "修具工",
        "推車人",
        "候渡者",
        "看貨人",
        "歇腳旅人",
        "候船者",
        "上岸客",
        "搬貨人",
    ];
    roles.iter().any(|role| s.contains(role))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_character() -> Character {
        Character {
            id: "npc_test".to_string(),
            kind: crate::entity::EntityKind::Npc,
            display_char: String::new(),
            x: 0,
            y: 0,
            move_state: crate::entity::MoveState::Idle,
            target_x: None,
            target_y: None,
            walk_or_run: crate::entity::WalkOrRun::Walk,
            move_started_at: None,
            vit: 10,
            qi: 10,
            dex: 10,
            magnesium: 0,
            last_observed_at: None,
            created_at: 0,
            gender: Some(crate::entity::Gender::M),
            soul_seed: None,
            display_title: String::new(),
            activated_nodes: "[]".to_string(),
            equipment_slots: String::new(),
            inventory: "[]".to_string(),
            disposition: 0,
            thoughts: vec![],
            current_activity: String::new(),
            grid_x: None,
            grid_y: None,
        }
    }

    #[test]
    fn look_narrative_has_no_ascii_space_after_sentence_break() {
        let mut ch = test_character();
        ch.current_activity = "整理包袱".to_string();
        let line = build_look_narrative(&ch, "候渡者", "北岸渡口");
        assert!(!line.contains("。 "));
        assert!(line.contains("。他目前整理包袱。"));
    }

    #[test]
    fn generated_private_name_falls_back_to_scene_label() {
        assert_eq!(observed_label_for_room("王小明", "北岸渡口"), "候渡者");
    }

    #[test]
    fn public_role_label_is_kept() {
        assert_eq!(observed_label_for_room("船戶", "北岸渡口"), "船戶");
    }

    #[test]
    fn market_private_name_falls_back_to_scene_label() {
        assert_eq!(observed_label_for_room("王小明", "岸邊短市"), "看貨人");
    }

    #[test]
    fn south_bank_private_name_falls_back_to_scene_label() {
        assert_eq!(observed_label_for_room("王小明", "南岸泥坡"), "上岸客");
    }

    #[test]
    fn warehouse_private_name_falls_back_to_scene_label() {
        assert_eq!(observed_label_for_room("王小明", "倉棚前場"), "搬貨人");
    }

    #[test]
    fn m3_050_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("船匠", "東南船塢口"), "船匠");
        assert_eq!(observed_label_for_room("補帆人", "帆布曬場"), "補帆人");
        assert_eq!(observed_label_for_room("王小明", "船戶街口"), "船戶");
    }

    #[test]
    fn boathouse_street_look_uses_street_work_not_ferry_work_point() {
        let line = scene_observation_line("船戶街口", "他").expect("船戶街口應有觀察句");

        assert!(line.contains("招工牌"));
        assert!(line.contains("船期"));
        assert!(!line.contains("木樁"));
        assert!(!line.contains("纜繩"));
    }

    #[test]
    fn m3_053_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("海關吏", "海關前棚"), "海關吏");
        assert_eq!(observed_label_for_room("商館人", "商館街口"), "商館人");
        assert_eq!(observed_label_for_room("王小明", "匯兌小屋"), "兌票人");
    }

    #[test]
    fn foreign_trade_look_uses_trade_work_not_generic_passerby() {
        let customs = scene_observation_line("海關前棚", "他").expect("海關前棚應有觀察句");
        let exchange = scene_observation_line("匯兌小屋", "她").expect("匯兌小屋應有觀察句");

        assert!(customs.contains("貨單"));
        assert!(customs.contains("封條"));
        assert!(exchange.contains("銅秤"));
        assert!(exchange.contains("賬冊"));
        assert!(!customs.contains("包袱"));
        assert!(!exchange.contains("包袱"));
    }

    #[test]
    fn m3_006_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("搬魚人", "港灣貨棚"), "搬魚人");
        assert_eq!(observed_label_for_room("補網人", "漁網曬場"), "補網人");
        assert_eq!(observed_label_for_room("王小明", "北灣守望石階"), "守望人");
    }

    #[test]
    fn north_harbor_look_uses_harbor_work_not_generic_passerby() {
        let net = scene_observation_line("漁網曬場", "她").expect("漁網曬場應有觀察句");
        let watch = scene_observation_line("北灣守望石階", "他").expect("北灣守望石階應有觀察句");

        assert!(net.contains("濕網"));
        assert!(net.contains("穿線"));
        assert!(watch.contains("海面"));
        assert!(watch.contains("警鈴"));
        assert!(!net.contains("包袱"));
        assert!(!watch.contains("包袱"));
    }

    #[test]
    fn m3_009_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("礦工", "礦坑外棚"), "礦工");
        assert_eq!(observed_label_for_room("洗礦工", "洗礦溝"), "洗礦工");
        assert_eq!(observed_label_for_room("王小明", "工具修棚"), "修具工");
        assert_eq!(observed_label_for_room("李小明", "礦渣坡"), "推車人");
    }

    #[test]
    fn northwest_mine_look_uses_mine_work_not_generic_passerby() {
        let wash = scene_observation_line("洗礦溝", "她").expect("洗礦溝應有觀察句");
        let load = scene_observation_line("舊道裝車點", "他").expect("舊道裝車點應有觀察句");

        assert!(wash.contains("冷水槽"));
        assert!(wash.contains("粗料倉"));
        assert!(load.contains("礦車"));
        assert!(load.contains("稱重木牌"));
        assert!(!wash.contains("包袱"));
        assert!(!load.contains("包袱"));
    }

    #[test]
    fn final_map_public_roles_are_kept_when_looking() {
        for (raw, room, label) in [
            ("守門人", "邊城外門", "守門人"),
            ("王小明", "糧倉前場", "倉管"),
            ("鐵匠", "鍛造棚", "鐵匠"),
            ("李小明", "牧地柵門", "牧人"),
            ("獸醫", "獸醫棚", "獸醫"),
            ("趙小明", "商隊泊位", "商隊人"),
            ("店伙", "客棧前廊", "店伙"),
            ("巡哨", "巡哨棚", "巡哨"),
        ] {
            assert_eq!(observed_label_for_room(raw, room), label, "{room}");
        }
    }

    #[test]
    fn final_map_look_uses_scene_work_not_generic_passerby() {
        let border = scene_observation_line("邊城外門", "他").expect("邊城外門應有觀察句");
        let forge = scene_observation_line("鍛造棚", "她").expect("鍛造棚應有觀察句");
        let pasture = scene_observation_line("牧地柵門", "他").expect("牧地柵門應有觀察句");
        let town = scene_observation_line("邊鎮井口", "她").expect("邊鎮井口應有觀察句");

        assert!(border.contains("通行牌"));
        assert!(forge.contains("鐵胚"));
        assert!(pasture.contains("畜牌"));
        assert!(town.contains("井繩"));
        assert!(!border.contains("包袱"));
        assert!(!forge.contains("包袱"));
        assert!(!pasture.contains("包袱"));
        assert!(!town.contains("包袱"));
    }
}
