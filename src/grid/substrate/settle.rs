//! 觀測現場惰性結算（Pull `settle()`）與髒格到期 GC 回收（每 tick `<= 512` 格）。
//!
//! - **營火雨滅／淹滅／燃盡**：雨落無遮、高水漫流或超過 240 遊戲分鐘自動熄滅並移出 `S0` 伴生槽。
//! - **拉式（Pull）竹潮再生**：`U1 = ThornyBambooRhizome` 且 `V1 = None`（已劈開竹叢但未掘鞭），
//!   連續 3 遊戲日（雨霧天 2 日）且踩踏熱度低於閾值 `40` 時，重新頂出 `V1 = ThornyBambooClump`。
//! - **耗竭恢復**：`charge == 0` 跨日後回復為 `-1`（原生儲量）。
//! - **自動回收**：結算後若完全回歸原生推導狀態，立即走 CAS / `DELETE` 清除 `cell_overlay` 髒列。

use super::cache::{get_cached_overlay, remove_cached_overlay};
use super::hydrology::{WorldClockSnapshot, derive_cell_field};
use super::l0_cell::L0Cell;
use super::runtime;
use super::slots::{
    FieldState, S0Companion, S0Pavement, U1Plugin, V1Plugin, derive_pristine_slots,
    is_campfire_extinguished_by_rain,
};
use crate::pg::cell_overlay::{CellOverlayRecord, cas_write_overlay_record, delete_overlay_record};
use crate::store::sql::DbPool;

/// 營火自然燃盡壽命（遊戲分鐘，4 遊戲小時）。
pub const CAMPFIRE_BURN_MINUTES: i32 = 240;
/// 踩踏熱度足以壓制竹潮再生的閾值。
pub const BAMBOO_SUPPRESS_HEAT: u8 = 40;
/// 每次週期 GC 回收上限（嚴守規格書 §5.3 每 tick `<= 512` 格）。
pub const MAX_GC_PER_TICK: usize = 512;

/// 純函式：判斷單筆髒格紀錄於當下時空與場域下是否需要結算並就地更新；有變動回傳 `true`。
pub fn apply_lazy_settle(
    rec: &mut CellOverlayRecord,
    l0: L0Cell,
    world_seed: u64,
    clock: WorldClockSnapshot,
    field: FieldState,
) -> bool {
    let mut changed = false;

    // 1. 營火雨滅、高水淹滅或自然燃盡
    if rec.stack.has_campfire() {
        let rain_out = is_campfire_extinguished_by_rain(field.weather_wetting, rec.stack.v1);
        let flood_out = field.flood_active;
        let burned_out = rec.stamp > 0 && clock.world_minute - rec.stamp >= CAMPFIRE_BURN_MINUTES;
        if rain_out || flood_out || burned_out {
            rec.stack
                .s0_companions
                .retain(|c| *c != S0Companion::BambooCampfire);
            rec.stamp = 0;
            changed = true;
        }
    }

    // 2. 拉式（Pull）竹潮：有竹鞭無地上竹叢，且踩踏熱度不足並達天數門檻即重新長回
    if rec.stack.u1 == U1Plugin::ThornyBambooRhizome && rec.stack.v1 == V1Plugin::None {
        let heat = match rec.stack.s0_pave {
            S0Pavement::BambooTrampledPath { pave_heat } => pave_heat,
            _ => 0,
        };
        let elapsed_days = clock.game_day.saturating_sub(rec.roll_day);
        let regrow_days = if field.weather_wetting { 2 } else { 3 };
        if rec.roll_day > 0 && elapsed_days >= regrow_days && heat < BAMBOO_SUPPRESS_HEAT {
            let pristine = derive_pristine_slots(rec.gx, rec.gy, l0, world_seed);
            rec.stack.v1 = V1Plugin::ThornyBambooClump;
            // 若原為裸土／低熱徑，回歸原生鋪面與伴生
            if heat < 16 {
                rec.stack.s0_pave = pristine.s0_pave;
                rec.stack.s0_companions = pristine.s0_companions;
            }
            rec.roll_day = 0;
            rec.charge = -1;
            changed = true;
        }
    }

    // 3. 耗竭格跨日恢復儲量（charge == 0 -> -1）
    if rec.charge == 0 && rec.roll_day > 0 && clock.game_day > rec.roll_day {
        let pristine = derive_pristine_slots(rec.gx, rec.gy, l0, world_seed);
        rec.charge = -1;
        rec.roll_day = 0;
        if rec.stack.v1 == pristine.v1 && rec.stack.u1 == pristine.u1 {
            rec.stack.s0_companions = pristine.s0_companions;
        }
        changed = true;
    }

    changed
}

/// 判斷一筆紀錄是否已完全回歸原生推導狀態（可 `DELETE` 回收髒列）。
pub fn is_record_pristine(rec: &CellOverlayRecord, l0: L0Cell, world_seed: u64) -> bool {
    let pristine = derive_pristine_slots(rec.gx, rec.gy, l0, world_seed);
    rec.stack == pristine
        && rec.building_id.is_none()
        && rec.roll_day == 0
        && rec.weather_epoch == 0
        && rec.flags == 0
        && rec.charge == -1
        && rec.stamp == 0
}

/// 觀測現場惰性結算單格（僅於該格已是髒格時才有機會觸發寫回或回收，原生格 0 DB 開銷）。
pub fn settle_cell_on_observe(
    pool: &DbPool,
    gx: i32,
    gy: i32,
    l0: L0Cell,
    clock: WorldClockSnapshot,
    field: FieldState,
) -> Option<CellOverlayRecord> {
    let existing = get_cached_overlay(pool, gx, gy)?;
    let mut next = existing.clone();
    if !apply_lazy_settle(&mut next, l0, runtime::WORLD_SEED, clock, field) {
        return Some(existing);
    }

    if is_record_pristine(&next, l0, runtime::WORLD_SEED) {
        if delete_overlay_record(pool, gx, gy).is_ok() {
            remove_cached_overlay(gx, gy);
            return None;
        }
        return Some(existing);
    }

    match cas_write_overlay_record(pool, &next, existing.ver) {
        Ok(true) => {
            next.ver = existing.ver + 1;
            super::cache::put_cached_overlay(next.clone());
            Some(next)
        }
        _ => Some(existing),
    }
}

/// 週期性髒格到期 GC（每 tick 最多掃描 `limit.min(512)` 筆到期髒格並執行結算與 `DELETE` 回收）。
pub fn gc_expired_overlays(pool: &DbPool, clock: WorldClockSnapshot, limit: usize) -> usize {
    let Some(rt) = runtime::global() else {
        return 0;
    };
    let cap = limit.clamp(1, MAX_GC_PER_TICK) as i64;
    let campfire_cutoff = clock.world_minute.saturating_sub(CAMPFIRE_BURN_MINUTES);
    let Ok(mut conn) = pool.get() else {
        return 0;
    };
    let Ok(rows) = conn.query(
        "SELECT gx, gy FROM cell_overlay
         WHERE (stamp > 0 AND stamp <= $1)
            OR (roll_day > 0 AND roll_day < $2)
         ORDER BY updated_at ASC
         LIMIT $3",
        &[&campfire_cutoff, &clock.game_day, &cap],
    ) else {
        return 0;
    };
    drop(conn);

    let mut reclaimed = 0usize;
    for row in rows {
        let gx: i32 = row.get(0);
        let gy: i32 = row.get(1);
        let Some(l0) = rt.l0(gx, gy) else {
            continue;
        };
        let field = derive_cell_field(gx, gy, l0, clock);
        if settle_cell_on_observe(pool, gx, gy, l0, clock, field).is_none() {
            reclaimed += 1;
        }
    }
    reclaimed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::hydrology::KaipuSeason;
    use crate::grid::substrate::l0_cell::{SubstrateMaterial, SurfaceWaterKind, landmass};
    use crate::grid::substrate::slots::SlotStack;

    fn bamboo_land() -> L0Cell {
        L0Cell::new(
            Some(30.0),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::AlluvialSandyLoam,
            SurfaceWaterKind::None,
            20.0,
            90,
        )
    }

    #[test]
    fn campfire_extinguishes_on_rain_or_timeout() {
        let l0 = bamboo_land();
        let mut stack = SlotStack::default();
        stack.add_companion(S0Companion::BambooCampfire);
        let mut rec = CellOverlayRecord::from_stack(1, 0, stack);
        rec.stamp = 100;

        let dry_early = WorldClockSnapshot {
            game_day: 1,
            world_minute: 200,
            minute_of_day: 200,
            season: KaipuSeason::SpringThaw,
            weather_wetting: false,
        };
        assert!(!apply_lazy_settle(
            &mut rec,
            l0,
            runtime::WORLD_SEED,
            dry_early,
            FieldState::default()
        ));
        assert!(rec.stack.has_campfire());

        // 逾 240 遊戲分鐘自然燃盡
        let late = WorldClockSnapshot {
            world_minute: 400,
            ..dry_early
        };
        assert!(apply_lazy_settle(
            &mut rec,
            l0,
            runtime::WORLD_SEED,
            late,
            FieldState::default()
        ));
        assert!(!rec.stack.has_campfire());
    }

    #[test]
    fn bamboo_pull_regrows_after_three_dry_or_two_wet_days_unless_trampled() {
        let l0 = bamboo_land();
        let mut stack = derive_pristine_slots(1, 0, l0, runtime::WORLD_SEED);
        assert!(stack.clear_v1_bamboo());
        let mut rec = CellOverlayRecord::from_stack(1, 0, stack);
        rec.roll_day = 10;

        // 2 日晴天不再生
        let day12_dry = WorldClockSnapshot {
            game_day: 12,
            world_minute: 12 * 1440,
            minute_of_day: 0,
            season: KaipuSeason::SpringThaw,
            weather_wetting: false,
        };
        assert!(!apply_lazy_settle(
            &mut rec,
            l0,
            runtime::WORLD_SEED,
            day12_dry,
            FieldState::default()
        ));

        // 2 日雨霧天立即再生並回歸原生
        let wet_field = FieldState {
            weather_wetting: true,
            ..FieldState::default()
        };
        assert!(apply_lazy_settle(
            &mut rec,
            l0,
            runtime::WORLD_SEED,
            day12_dry,
            wet_field
        ));
        assert_eq!(rec.stack.v1, V1Plugin::ThornyBambooClump);
        assert!(is_record_pristine(&rec, l0, runtime::WORLD_SEED));
    }
}
