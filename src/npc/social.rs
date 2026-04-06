//! NPC 同房微互動（對齊既有 `npc/social`）。

use rand::Rng;

use crate::db::{get_entities_in_room, person_name_from_npc_list_label};
use crate::entity::EntityKind;

const MICRO_TEMPLATES: [&str; 8] = [
    "【%s】朝【%s】點了點頭。",
    "【%s】與【%s】閒聊了幾句。",
    "【%s】看了【%s】一眼，沒有說話。",
    "【%s】對【%s】笑了笑。",
    "【%s】低聲問【%s】：「知道哪裡有活幹嗎？」",
    "【%s】向【%s】抱怨道：「唉，日子不好過啊。」",
    "【%s】和【%s】並肩站著，望向遠方。",
    "【%s】拍了拍【%s】的肩膀。",
];

/// 同房至少兩名 NPC 時，以 `chance`（0–100）機率抽一句微互動；否則回空（對齊既有 `PickMicroInteraction`）。
#[must_use]
pub fn pick_micro_interaction(room_id: &str, chance: i32, game_hour: i32) -> String {
    let Ok(entities) = get_entities_in_room(room_id, game_hour) else {
        return String::new();
    };
    let mut names: Vec<String> = Vec::new();
    for e in entities {
        if e.kind != EntityKind::Npc {
            continue;
        }
        let mut n = person_name_from_npc_list_label(&e.display_title);
        if n.is_empty() {
            n = e.id.clone();
        }
        names.push(n);
    }
    if names.len() < 2 {
        return String::new();
    }
    let mut rng = rand::rng();
    if rng.random_range(0..100) >= chance {
        return String::new();
    }
    let i = rng.random_range(0..names.len());
    let mut j = rng.random_range(0..names.len());
    while j == i {
        j = rng.random_range(0..names.len());
    }
    let tmpl = MICRO_TEMPLATES[rng.random_range(0..MICRO_TEMPLATES.len())];
    tmpl.replacen("%s", &names[i], 1).replacen("%s", &names[j], 1)
}
