//! NPC 行為反應一句話（借物／制伏等尾綴），對齊 Go `npc/reaction.go`。

use rand::Rng;

use crate::db::{expand_soul_seed_to_personality, get_entity, get_npc_person_display_name};
use crate::entity::EntityKind;

/// 依情境回傳一句 NPC 視角敘事尾綴；無則空字串。
/// `kind`：subdue_victim、slay_victim、lost_subdue、borrow_ok、borrow_caught。
#[must_use]
pub fn npc_behavior_reaction_line(npc_id: &str, player_id: &str, kind: &str) -> String {
    let _ = player_id;
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
    let short = short_name(npc_id);

    match kind {
        "subdue_victim" => {
            if ord > 0.65 {
                return format!("【{short}】咬牙道：「今日算你留手……」");
            }
            if bold > 0.65 {
                return format!("【{short}】雖倒地，仍怒目瞪你。");
            }
            if sens > 0.65 && r < 0.5 {
                return format!("【{short}】喘息著，一時說不出話。");
            }
            format!("【{short}】倒地不起，暫難起身。")
        }
        "slay_victim" => String::new(),
        "lost_subdue" => {
            if bold > 0.6 {
                format!("【{short}】哼了一聲：「還站得住？」")
            } else {
                format!("【{short}】收勢而立。")
            }
        }
        "borrow_ok" => {
            if ord > 0.55 && r < 0.4 {
                format!("【{short}】似乎尚未察覺身上少了一物。")
            } else {
                String::new()
            }
        }
        "borrow_caught" => {
            if bold > 0.6 {
                format!("【{short}】厲聲喝道：「住手！你這算哪門子的借！」")
            } else if ord > 0.6 {
                format!("【{short}】皺眉盯著你：「不告而取，豈是君子？」")
            } else {
                format!("【{short}】一把按住你的手：「想借？先問過我。」")
            }
        }
        _ => String::new(),
    }
}

fn short_name(entity_id: &str) -> String {
    let Ok(Some(e)) = get_entity(entity_id) else {
        return entity_id.to_string();
    };
    if e.kind == EntityKind::Npc {
        return get_npc_person_display_name(entity_id).unwrap_or_else(|_| entity_id.to_string());
    }
    if !e.display_title.is_empty() {
        return e.display_title;
    }
    e.id
}
