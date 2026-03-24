//! NPC 對玩家好感度常數與調整（對齊 Go `db/npc_memory.go`）。

use crate::store;

use super::ErrNoStore;

/// 對話一次。
pub const FAV_TALK: i32 = 5;
/// 被送行（致死意圖戰鬥）。
pub const FAV_SLAY: i32 = -15;
/// 被留人（制伏）。
pub const FAV_SUBDUE: i32 = -8;
/// 借物當場被喝破。
pub const FAV_BORROW_CAUGHT: i32 = -12;
/// 借物得手（較輕）。
pub const FAV_BORROW_SUCCESS: i32 = -3;

/// 調整該 NPC 對某玩家的好感度，clamp [-100, +100]（對齊 Go `AdjustFavorability`）。
pub fn adjust_favorability(entity_id: &str, subject_id: &str, delta: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.adjust_favorability(entity_id, subject_id, delta)
}

/// 回傳一句「與這位來者的短期記憶」供拼進背版；無記憶或見面 0 次回傳空字串（對齊 Go `FormatNpcMemoryForBackstory`）。
pub fn format_npc_memory_for_backstory(entity_id: &str, subject_id: &str) -> String {
    let Some(arc) = store::get_store() else {
        return String::new();
    };
    let s = arc.read().unwrap();
    let Some(m) = s.get_npc_memory(entity_id, subject_id) else {
        return String::new();
    };
    if m.meet_count <= 0 {
        return String::new();
    }
    let mut out = format!("你與這位來者見過{}次。", m.meet_count);
    if m.favorability >= 30 {
        out.push_str("你對其印象不錯。");
    } else if m.favorability <= -30 {
        out.push_str("你對其印象不佳。");
    } else {
        out.push_str("你對其印象普通。");
    }
    out
}
