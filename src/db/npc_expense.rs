//! NPC 每日食宿鎂消耗（對齊既有 `db/npc_expense`）。

use crate::event;
use crate::store;

use super::disposition::{adjust_disposition, DISP_BROKE, DISP_DAILY};
use super::add_magnesium;
use super::ErrNoStore;

/// 每遊戲日基礎食宿消耗鎂數。
pub const DAILY_EXPENSE_BASE: i32 = 8;

/// 扣減所有在房 NPC 的每日消耗；鎂歸零時記 `broke` 事件並調心境（對齊既有 `DeductDailyExpense`）。
pub fn deduct_daily_expense(now_unix: i64) -> anyhow::Result<i32> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let ids = {
        let s = arc.read().unwrap_or_else(|e| e.into_inner());
        s.get_npc_ids_with_room()
    };
    let mut affected = 0i32;
    for id in ids {
        let mag = {
            let s = arc.read().unwrap_or_else(|e| e.into_inner());
            let Some(e) = s.get_entity(&id) else {
                continue;
            };
            e.magnesium
        };
        if mag > 0 {
            let new_mag = mag.saturating_sub(DAILY_EXPENSE_BASE);
            add_magnesium(&id, -DAILY_EXPENSE_BASE)?;
            affected += 1;
            if new_mag == 0 {
                let _ = event::append(now_unix, &id, "broke", "鎂歸零");
                let _ = adjust_disposition(&id, DISP_BROKE);
            }
        }
        let _ = adjust_disposition(&id, DISP_DAILY);
    }
    Ok(affected)
}
