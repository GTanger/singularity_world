//! 心境值調整（對齊既有 `db/disposition`）。

use crate::store;

use super::ErrNoStore;

/// 鎂歸零時心境修正。
pub const DISP_BROKE: i32 = -20;
/// 每日自然衰減（回歸中性）。
pub const DISP_DAILY: i32 = -2;
/// 乞討成功（對齊既有 `DispBegSuccess`）。
pub const DISP_BEG_SUCCESS: i32 = 3;
/// 採集成功。
pub const DISP_GATHER: i32 = 5;
/// 獲得指派。
pub const DISP_HIRED: i32 = 20;
/// 街頭兜售成功。
pub const DISP_TRADE: i32 = 6;
/// 與人交談／社交到達。
pub const DISP_TALKED: i32 = 5;
/// 被留人制伏倒地後的心境修正（對齊既有 `DispSubdued`）。
pub const DISP_SUBDUED: i32 = -15;

/// 調整心境並 clamp 於 [-100, 100]（對齊既有 `AdjustDisposition`）。
pub fn adjust_disposition(entity_id: &str, delta: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap_or_else(|e| e.into_inner());
    s.update_entity(entity_id, |e| {
        e.disposition += delta;
        e.disposition = e.disposition.clamp(-100, 100);
    })
}

/// 取得心境；無實體時 0（對齊既有 `GetDisposition`）。
#[must_use]
pub fn get_disposition(entity_id: &str) -> i32 {
    let Some(arc) = store::get_store() else {
        return 0;
    };
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    s.get_entity(entity_id).map(|e| e.disposition).unwrap_or(0)
}
