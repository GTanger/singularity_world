//! NPC 同房微互動（對齊既有 `npc/social`）。

use rand::Rng;

use crate::db::{get_entities_in_room, get_room_name};
use crate::entity::EntityKind;
use crate::game::micro_interaction_lines_for_room;

/// 同房至少兩名 NPC 時，以 `chance`（0–100）機率抽一句微互動；否則回空（對齊既有 `PickMicroInteraction`）。
#[must_use]
pub fn pick_micro_interaction(room_id: &str, chance: i32, game_hour: i32) -> String {
    let Ok(entities) = get_entities_in_room(room_id, game_hour) else {
        return String::new();
    };
    let mut npc_count = 0usize;
    for e in entities {
        if e.kind != EntityKind::Npc {
            continue;
        }
        npc_count += 1;
    }
    if npc_count < 2 {
        return String::new();
    }
    let mut rng = rand::rng();
    if rng.random_range(0..100) >= chance {
        return String::new();
    }
    let room_name = player_layer_room_name(room_id);
    let lines = micro_interaction_lines_for_room(&room_name);
    if lines.is_empty() {
        return String::new();
    }
    let line = if lines.first().copied() == Some("近旁兩名行人低聲交談了幾句，很快又各自看向別處。") {
        if let Some(venue_line) = synthesize_venue_micro_interaction(&room_name) {
            venue_line
        } else {
            lines[rng.random_range(0..lines.len())].to_string()
        }
    } else {
        lines[rng.random_range(0..lines.len())].to_string()
    };
    line.replace("客棧打尖", "旅社歇腳用餐")
        .replace("客棧", "旅社")
        .replace("把酒言歡", "聚餐暢談")
        .replace("打尖", "歇腳用餐")
}

/// 依據房間之一等場域物件合成微互動文本（無模板符號與括號，純街坊日常舉止）
pub fn synthesize_venue_micro_interaction(room_name: &str) -> Option<String> {
    let venue = crate::grid::resolve_any_room_venue_object(room_name, "")?;
    if venue.has_counter_restricted_zone {
        Some(format!(
            "{}站在{}當值櫃檯內側，一邊{}，一邊與身旁的街坊核對交割細節。",
            venue.default_host_title, venue.venue_kind, venue.on_duty_fixture_activity
        ))
    } else {
        Some(format!(
            "{}在{}裡{}，身旁的同伴順手遞過器物默契配合。",
            venue.default_host_title, venue.venue_kind, venue.on_duty_fixture_activity
        ))
    }
}

fn player_layer_room_name(room_id: &str) -> String {
    let raw = if let Some(coord) = crate::grid::parse_grid_room_id(room_id)
        && let Some(cell) = crate::server::grid_manager::get_runtime_grid_cell(coord)
    {
        if cell.name.trim().is_empty() {
            crate::grid::terrain_name_zh(cell.terrain).to_string()
        } else {
            cell.name.clone()
        }
    } else {
        get_room_name(room_id).unwrap_or_default()
    };
    crate::store::clean_wilderness_place_label(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_layer_micro_lines_do_not_expose_template_names() {
        for scene in ["半嶺水岸渡口", "水尾寮渡頭市集", "水尾寮"] {
            let lines = micro_interaction_lines_for_room(scene);
            assert!(lines.iter().all(|line| !line.contains("%s")));
            assert!(lines.iter().all(|line| !line.contains('【')));
        }
    }

    #[test]
    fn test_synthesize_venue_micro_interaction_uses_room_venue_object() {
        let canteen_line = synthesize_venue_micro_interaction("三水大食堂");
        assert!(canteen_line.is_some());
        let c_text = canteen_line.unwrap();
        assert!(c_text.contains("食堂主廚"));
        assert!(c_text.contains("當值櫃檯內側"));
        assert!(c_text.contains("街坊"));
        assert!(!c_text.contains("%s"));
        assert!(!c_text.contains('【'));
        assert!(!c_text.contains('】'));

        let mill_line = synthesize_venue_micro_interaction("老堰頭・水輪石磨工坊");
        assert!(mill_line.is_some());
        let m_text = mill_line.unwrap();
        assert!(m_text.contains("當值石磨匠師"));
        assert!(m_text.contains("水輪石磨工坊"));
        assert!(m_text.contains("當值櫃檯內側"));
        assert!(!m_text.contains('【'));
        assert!(!m_text.contains('】'));

        assert!(synthesize_venue_micro_interaction("荒蕪礫石空地").is_none());
    }
}
