//! 稀疏覆寫表 `cell_overlay`：僅被改動的「髒格」落盤，未變動格由底板與世界種子即時推導。

use crate::grid::substrate::slots::{
    S0Companion, S0Pavement, SlotStack, U1Plugin, U2Plugin, V1Plugin,
};
use crate::store::sql::DbPool;

const CREATE_OVERLAY: &str = "
CREATE TABLE IF NOT EXISTS cell_overlay (
    gx INTEGER NOT NULL,
    gy INTEGER NOT NULL,
    u2 SMALLINT NOT NULL DEFAULT 0,
    u1 SMALLINT NOT NULL DEFAULT 0,
    v1 SMALLINT NOT NULL DEFAULT 0,
    arcade_mask SMALLINT NOT NULL DEFAULT 0,
    shop_open BOOLEAN NOT NULL DEFAULT FALSE,
    s0_pave SMALLINT NOT NULL DEFAULT 0,
    pave_heat SMALLINT NOT NULL DEFAULT 0,
    s0_0 SMALLINT NOT NULL DEFAULT 0,
    s0_1 SMALLINT NOT NULL DEFAULT 0,
    s0_2 SMALLINT NOT NULL DEFAULT 0,
    building_id TEXT,
    roll_day INTEGER NOT NULL DEFAULT 0,
    weather_epoch INTEGER NOT NULL DEFAULT 0,
    explored_day INTEGER NOT NULL DEFAULT 0,
    flags INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (gx, gy)
)";

const ADD_GRID_Z: &str =
    "ALTER TABLE entities ADD COLUMN IF NOT EXISTS grid_z SMALLINT NOT NULL DEFAULT 0";

const UPSERT_SQL: &str = "
INSERT INTO cell_overlay (gx, gy, u2, u1, v1, arcade_mask, shop_open, s0_pave, pave_heat,
 s0_0, s0_1, s0_2, explored_day, flags, updated_at)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14, now())
ON CONFLICT (gx, gy) DO UPDATE SET
 u2=EXCLUDED.u2, u1=EXCLUDED.u1, v1=EXCLUDED.v1, arcade_mask=EXCLUDED.arcade_mask,
 shop_open=EXCLUDED.shop_open, s0_pave=EXCLUDED.s0_pave, pave_heat=EXCLUDED.pave_heat,
 s0_0=EXCLUDED.s0_0, s0_1=EXCLUDED.s0_1, s0_2=EXCLUDED.s0_2,
 explored_day=EXCLUDED.explored_day, flags=EXCLUDED.flags, updated_at=now()";

const SELECT_SQL: &str = "
SELECT u2, u1, v1, arcade_mask, shop_open, s0_pave, pave_heat, s0_0, s0_1, s0_2, flags
FROM cell_overlay WHERE gx=$1 AND gy=$2";

/// 建立 `cell_overlay` 與 `entities.grid_z`（冪等）。
pub fn ensure_schema(pool: &DbPool) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    conn.batch_execute(CREATE_OVERLAY).map_err(|e| e.to_string())?;
    // entities 表若尚未建立，不阻擋 overlay 表就緒。
    let _ = conn.batch_execute(ADD_GRID_Z);
    Ok(())
}

fn row_to_stack(row: &r2d2_postgres::postgres::Row) -> SlotStack {
    let c8 = |i: usize| row.get::<_, i16>(i).clamp(0, 255) as u8;
    let flags: i32 = row.get(10);
    SlotStack {
        u2: U2Plugin::from_code_i16(row.get(0)),
        u1: U1Plugin::from_code_i16(row.get(1)),
        s0_pave: S0Pavement::from_codes(c8(5), c8(6)),
        s0_companions: (7..=9).filter_map(|i| S0Companion::from_code_u8(c8(i))).collect(),
        v1: V1Plugin::from_codes(c8(2), c8(3), row.get(4), (flags & 0x3) as u8),
    }
}

/// 讀取單格髒格覆寫；無列代表該格仍為惰性推導之原生狀態。
pub fn load_slot_overlay(pool: &DbPool, gx: i32, gy: i32) -> Option<SlotStack> {
    let mut conn = pool.get().ok()?;
    let row = conn.query_opt(SELECT_SQL, &[&gx, &gy]).ok()??;
    Some(row_to_stack(&row))
}

/// 一次讀取矩形視窗內全部髒格（含邊界）。
pub fn load_window_overlays(
    pool: &DbPool,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
) -> Vec<((i32, i32), SlotStack)> {
    let Ok(mut conn) = pool.get() else { return Vec::new() };
    let sql = "SELECT u2, u1, v1, arcade_mask, shop_open, s0_pave, pave_heat, s0_0, s0_1, s0_2, flags, gx, gy \
               FROM cell_overlay WHERE gx BETWEEN $1 AND $2 AND gy BETWEEN $3 AND $4";
    let Ok(rows) = conn.query(sql, &[&min_x, &max_x, &min_y, &max_y]) else { return Vec::new() };
    rows.iter().map(|r| ((r.get(11), r.get(12)), row_to_stack(r))).collect()
}

/// 寫入或更新單格髒格覆寫。
pub fn upsert_slot_overlay(
    pool: &DbPool,
    gx: i32,
    gy: i32,
    stack: &SlotStack,
    explored_day: i32,
) -> Result<(), String> {
    let (mask, open, max_z) = match stack.v1 {
        V1Plugin::ArcadeShophouse { arcade_mask, shop_open, max_z } => (arcade_mask, shop_open, max_z),
        _ => (0, false, 0),
    };
    let heat = match stack.s0_pave {
        S0Pavement::BambooTrampledPath { pave_heat } => pave_heat,
        _ => 0,
    };
    let slot = |i: usize| i16::from(stack.s0_companions.get(i).map_or(0, |c| c.code_u8()));
    let flags = i32::from(max_z.min(3));
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        UPSERT_SQL,
        &[
            &gx,
            &gy,
            &stack.u2.code_i16(),
            &stack.u1.code_i16(),
            &i16::from(stack.v1.code_u8()),
            &i16::from(mask),
            &open,
            &i16::from(stack.s0_pave.code_u8()),
            &i16::from(heat),
            &slot(0),
            &slot(1),
            &slot(2),
            &explored_day,
            &flags,
        ],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// 髒格總數（`/api/substrate/meta` 使用）。
pub fn count_dirty_cells(pool: &DbPool) -> i64 {
    let Ok(mut conn) = pool.get() else { return 0 };
    conn.query_one("SELECT COUNT(*) FROM cell_overlay", &[])
        .map(|r| r.get::<_, i64>(0))
        .unwrap_or(0)
}
