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
    let line = lines[rng.random_range(0..lines.len())].to_string();
    line.replace("客棧打尖", "旅社歇腳用餐")
        .replace("客棧", "旅社")
        .replace("把酒言歡", "聚餐暢談")
        .replace("打尖", "歇腳用餐")
}

fn player_layer_room_name(room_id: &str) -> String {
    if let Some(coord) = crate::grid::parse_grid_room_id(room_id)
        && let Some(cell) = crate::server::grid_manager::get_runtime_square_grid()
            .and_then(|g| g.get(coord).cloned())
    {
        if cell.name.trim().is_empty() {
            return crate::grid::terrain_name_zh(cell.terrain).to_string();
        }
        return cell.name;
    }
    get_room_name(room_id).unwrap_or_default()
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
}
