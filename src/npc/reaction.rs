//! NPC 行為反應一句話（借物／制伏等尾綴），對齊既有 `npc/reaction`。

use rand::Rng;

use crate::db::{expand_soul_seed_to_personality, get_entity, get_npc_person_display_name};
use crate::entity::EntityKind;

/// 依情境回傳一句 NPC 視角敘事尾綴；無則空字串。
/// `kind`：subdue_victim、slay_victim、lost_subdue、borrow_ok、borrow_caught。
#[must_use]
pub fn npc_behavior_reaction_line(npc_id: &str, player_id: &str, kind: &str) -> String {
    let mut bold = 0.5_f64;
    let mut sens = 0.5_f64;
    let mut ord = 0.5_f64;
    if let Ok(Some(ent)) = get_entity(npc_id)
        && let Some(seed) = ent.soul_seed
    {
        let p = expand_soul_seed_to_personality(seed);
        bold = p.boldness;
        sens = p.sensitivity;
        ord = p.orderliness;
    }
    let mut rng = rand::rng();
    let r: f64 = rng.random();
    let short = viewer_visible_name(npc_id, player_id);

    match kind {
        "subdue_victim" => {
            if ord > 0.65 {
                return format!("【{short}】咬牙道：「這筆帳我記下了，以後走著瞧！」");
            }
            if bold > 0.65 {
                return format!("【{short}】喘著氣退開：「別衝動，有話好說！」");
            }
            if sens > 0.65 && r < 0.5 {
                return format!("【{short}】捂著痛處連連後退，一時說不出話。");
            }
            format!("【{short}】失去再戰能力，暫難起身。")
        }
        "slay_victim" => String::new(),
        "lost_subdue" => {
            if bold > 0.6 {
                format!("【{short}】嚴肅斥道：「小心了！再靠近我就要動手自衛了！」")
            } else {
                format!("【{short}】點頭致意收手。")
            }
        }
        "borrow_ok" => {
            if ord > 0.55 && r < 0.4 {
                format!("【{short}】專注看著前方，並未察覺身上的物件被收進隨身包袋。")
            } else {
                String::new()
            }
        }
        "borrow_caught" => {
            if bold > 0.6 {
                format!("【{short}】眼神一凜，一把扣住你的手腕斥道：「快把東西放下，別逼我動手！」")
            } else if ord > 0.6 {
                format!("【{short}】盯著你斥責：「手腳這麼不乾淨，想偷東西？」")
            } else {
                format!("【{short}】立刻退開按住隨身包袋：「你未免太明目張膽了，想做什麼？」")
            }
        }
        "flee" | "retreat" => {
            format!("【{short}】低聲道：「情況不對，先撤！」")
        }
        "warn" | "greeting" => {
            format!("【{short}】打量著你：「請問有什麼事嗎？」")
        }
        "surrender" => {
            format!("【{short}】連忙退後擺手：「別衝動，有話好說！」")
        }
        "combat_opening" => {
            format!("【{short}】沉聲喝道：「小心了！」")
        }
        _ => String::new(),
    }
}

fn viewer_visible_name(entity_id: &str, player_id: &str) -> String {
    let Ok(Some(e)) = get_entity(entity_id) else {
        return "陌生人".to_string();
    };
    if e.kind == EntityKind::Npc {
        if !player_id.is_empty() && !crate::store::player_knows_npc(player_id, entity_id) {
            return "陌生人".to_string();
        }
        let name = get_npc_person_display_name(entity_id).unwrap_or_default();
        if name.is_empty() || name.is_ascii() {
            return "陌生人".to_string();
        }
        return name;
    }
    if !e.display_title.is_empty() && !e.display_title.is_ascii() {
        return e.display_title;
    }
    "陌生人".to_string()
}
