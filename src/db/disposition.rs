//! 心境值調整（對齊 Go `db/disposition.go`）。

use crate::store;

use super::ErrNoStore;

/// 鎂歸零時心境修正。
pub const DISP_BROKE: i32 = -20;
/// 每日自然衰減（回歸中性）。
pub const DISP_DAILY: i32 = -2;

/// 調整心境並 clamp 於 [-100, 100]（對齊 Go `AdjustDisposition`）。
pub fn adjust_disposition(entity_id: &str, delta: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.update_entity(entity_id, |e| {
        e.disposition += delta;
        e.disposition = e.disposition.clamp(-100, 100);
    })
}

/// 取得心境；無實體時 0（對齊 Go `GetDisposition`）。
#[must_use]
pub fn get_disposition(entity_id: &str) -> i32 {
    let Some(arc) = store::get_store() else {
        return 0;
    };
    let s = arc.read().unwrap();
    s.get_entity(entity_id).map(|e| e.disposition).unwrap_or(0)
}
