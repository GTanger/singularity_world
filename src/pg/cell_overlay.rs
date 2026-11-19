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
    charge SMALLINT NOT NULL DEFAULT -1,
    stamp INTEGER NOT NULL DEFAULT 0,
    ver INTEGER NOT NULL DEFAULT 1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (gx, gy)
)";

const ADD_GRID_Z: &str =
    "ALTER TABLE entities ADD COLUMN IF NOT EXISTS grid_z SMALLINT NOT NULL DEFAULT 0";
const ADD_INVENTORY_SEQ: &str =
    "ALTER TABLE entities ADD COLUMN IF NOT EXISTS inventory_seq BIGINT NOT NULL DEFAULT 0";
const ALTER_OVERLAY: &str = "
ALTER TABLE cell_overlay ADD COLUMN IF NOT EXISTS charge SMALLINT NOT NULL DEFAULT -1;
ALTER TABLE cell_overlay ADD COLUMN IF NOT EXISTS stamp INTEGER NOT NULL DEFAULT 0;
ALTER TABLE cell_overlay ADD COLUMN IF NOT EXISTS ver INTEGER NOT NULL DEFAULT 1";

/// 單格髒格完整覆寫紀錄（含建築、擲骰日、天氣世代、耗盡計數與版本戳）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellOverlayRecord {
    pub gx: i32,
    pub gy: i32,
    pub stack: SlotStack,
    pub building_id: Option<String>,
    pub roll_day: i32,
    pub weather_epoch: i32,
    pub explored_day: i32,
    pub flags: i32,
    /// `-1` 未動預設、`0` 耗竭、`>0` 剩餘次數。
    pub charge: i16,
    pub stamp: i32,
    pub ver: i32,
}

impl CellOverlayRecord {
    /// 以槽位堆疊建立預設紀錄。
    pub fn from_stack(gx: i32, gy: i32, stack: SlotStack) -> Self {
        Self {
            gx,
            gy,
            stack,
            building_id: None,
            roll_day: 0,
            weather_epoch: 0,
            explored_day: 0,
            flags: 0,
            charge: -1,
            stamp: 0,
            ver: 1,
        }
    }
}

const COLS: &str = "u2, u1, v1, arcade_mask, shop_open, s0_pave, pave_heat, s0_0, s0_1, s0_2, \
 flags, gx, gy, building_id, roll_day, weather_epoch, explored_day, charge, stamp, ver";

const UPSERT_SQL: &str = "
INSERT INTO cell_overlay (gx, gy, u2, u1, v1, arcade_mask, shop_open, s0_pave, pave_heat,
 s0_0, s0_1, s0_2, building_id, roll_day, weather_epoch, explored_day, flags,
 charge, stamp, ver, updated_at)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20, now())
ON CONFLICT (gx, gy) DO UPDATE SET
 u2=EXCLUDED.u2, u1=EXCLUDED.u1, v1=EXCLUDED.v1, arcade_mask=EXCLUDED.arcade_mask,
 shop_open=EXCLUDED.shop_open, s0_pave=EXCLUDED.s0_pave, pave_heat=EXCLUDED.pave_heat,
 s0_0=EXCLUDED.s0_0, s0_1=EXCLUDED.s0_1, s0_2=EXCLUDED.s0_2,
 building_id=EXCLUDED.building_id, roll_day=EXCLUDED.roll_day,
 weather_epoch=EXCLUDED.weather_epoch, explored_day=EXCLUDED.explored_day,
 flags=EXCLUDED.flags, charge=EXCLUDED.charge, stamp=EXCLUDED.stamp, ver=EXCLUDED.ver,
 updated_at=now()";

const CAS_INSERT_SQL: &str = "
INSERT INTO cell_overlay (gx, gy, u2, u1, v1, arcade_mask, shop_open, s0_pave, pave_heat,
 s0_0, s0_1, s0_2, building_id, roll_day, weather_epoch, explored_day, flags,
 charge, stamp, ver, updated_at)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19, 1, now())
ON CONFLICT (gx, gy) DO NOTHING";

const CAS_UPDATE_SQL: &str = "
UPDATE cell_overlay SET
 u2=$3, u1=$4, v1=$5, arcade_mask=$6, shop_open=$7, s0_pave=$8, pave_heat=$9,
 s0_0=$10, s0_1=$11, s0_2=$12, building_id=$13, roll_day=$14, weather_epoch=$15,
 explored_day=$16, flags=$17, charge=$18, stamp=$19, ver=$20 + 1, updated_at=now()
WHERE gx=$1 AND gy=$2 AND ver=$20";

const CAS_UPDATE_POSITIVE_CHARGE_SQL: &str = "
UPDATE cell_overlay SET
 u2=$3, u1=$4, v1=$5, arcade_mask=$6, shop_open=$7, s0_pave=$8, pave_heat=$9,
 s0_0=$10, s0_1=$11, s0_2=$12, building_id=$13, roll_day=$14, weather_epoch=$15,
 explored_day=$16, flags=$17, charge=$18, stamp=$19, ver=$20 + 1, updated_at=now()
WHERE gx=$1 AND gy=$2 AND ver=$20 AND (charge = -1 OR charge > 0)";

/// `cell_overlay` × `entities.inventory` 同交易原子提交結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayInvTxResult {
    Committed {
        new_ver: i32,
        new_inventory: String,
        new_inventory_seq: i64,
    },
    /// 版本不符或儲量已耗竭（`charge == 0`），整筆交易已回滾，呼叫端可重試。
    Conflict,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TxInvEntry {
    item_id: String,
    qty: i32,
}

fn apply_inventory_deltas(raw_json: &str, deltas: &[(&str, i32)]) -> Result<String, String> {
    let mut entries: Vec<TxInvEntry> = if raw_json.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(raw_json).unwrap_or_default()
    };
    for &(item_id, delta) in deltas {
        if delta == 0 {
            continue;
        }
        if let Some(slot) = entries.iter_mut().find(|e| e.item_id == item_id) {
            let next_qty = slot.qty + delta;
            if next_qty < 0 {
                return Err(format!("行囊「{item_id}」數量不足"));
            }
            slot.qty = next_qty;
        } else if delta > 0 {
            entries.push(TxInvEntry {
                item_id: item_id.to_string(),
                qty: delta,
            });
        } else {
            return Err(format!("行囊查無「{item_id}」可扣除"));
        }
    }
    entries.retain(|e| e.qty > 0);
    serde_json::to_string(&entries).map_err(|e| e.to_string())
}

/// 建立 `cell_overlay` 與 `entities` 擴充欄位（冪等）。
pub fn ensure_schema(pool: &DbPool) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    conn.batch_execute(CREATE_OVERLAY)
        .map_err(|e| e.to_string())?;
    conn.batch_execute(ALTER_OVERLAY)
        .map_err(|e| e.to_string())?;
    // entities 表若尚未建立，不阻擋 overlay 表就緒。
    let _ = conn.batch_execute(ADD_GRID_Z);
    let _ = conn.batch_execute(ADD_INVENTORY_SEQ);
    Ok(())
}

fn row_to_record(row: &r2d2_postgres::postgres::Row) -> CellOverlayRecord {
    let c8 = |i: usize| row.get::<_, i16>(i).clamp(0, 255) as u8;
    let flags: i32 = row.get(10);
    let stack = SlotStack {
        u2: U2Plugin::from_code_i16(row.get(0)),
        u1: U1Plugin::from_code_i16(row.get(1)),
        s0_pave: S0Pavement::from_codes(c8(5), c8(6)),
        s0_companions: (7..=9)
            .filter_map(|i| S0Companion::from_code_u8(c8(i)))
            .collect(),
        v1: V1Plugin::from_codes(c8(2), c8(3), row.get(4), (flags & 0x3) as u8),
    };
    CellOverlayRecord {
        gx: row.get(11),
        gy: row.get(12),
        stack,
        building_id: row.get(13),
        roll_day: row.get(14),
        weather_epoch: row.get(15),
        explored_day: row.get(16),
        flags,
        charge: row.get(17),
        stamp: row.get(18),
        ver: row.get(19),
    }
}

/// 讀取單格完整髒格紀錄；無列代表該格仍為惰性推導之原生狀態。
pub fn load_overlay_record(pool: &DbPool, gx: i32, gy: i32) -> Option<CellOverlayRecord> {
    let mut conn = pool.get().ok()?;
    let sql = format!("SELECT {COLS} FROM cell_overlay WHERE gx=$1 AND gy=$2");
    let row = conn.query_opt(sql.as_str(), &[&gx, &gy]).ok()??;
    Some(row_to_record(&row))
}

/// 一次讀取矩形視窗內全部髒格紀錄（含邊界）。
pub fn load_window_records(
    pool: &DbPool,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
) -> Vec<CellOverlayRecord> {
    let Ok(mut conn) = pool.get() else {
        return Vec::new();
    };
    let sql = format!(
        "SELECT {COLS} FROM cell_overlay WHERE gx BETWEEN $1 AND $2 AND gy BETWEEN $3 AND $4"
    );
    let Ok(rows) = conn.query(sql.as_str(), &[&min_x, &max_x, &min_y, &max_y]) else {
        return Vec::new();
    };
    rows.iter().map(row_to_record).collect()
}

/// 讀取單格髒格覆寫（相容包裝）。
pub fn load_slot_overlay(pool: &DbPool, gx: i32, gy: i32) -> Option<SlotStack> {
    load_overlay_record(pool, gx, gy).map(|r| r.stack)
}

/// 一次讀取矩形視窗內全部髒格（相容包裝）。
pub fn load_window_overlays(
    pool: &DbPool,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
) -> Vec<((i32, i32), SlotStack)> {
    load_window_records(pool, min_x, max_x, min_y, max_y)
        .into_iter()
        .map(|r| ((r.gx, r.gy), r.stack))
        .collect()
}

/// 寫入或更新單格完整髒格紀錄；`flags` 低 2 bits 固定存 `max_z.min(3)`。
pub fn upsert_overlay_record(pool: &DbPool, record: &CellOverlayRecord) -> Result<(), String> {
    let stack = &record.stack;
    let (mask, open, max_z) = match stack.v1 {
        V1Plugin::ArcadeShophouse {
            arcade_mask,
            shop_open,
            max_z,
        } => (arcade_mask, shop_open, max_z),
        _ => (0, false, 0),
    };
    let heat = match stack.s0_pave {
        S0Pavement::BambooTrampledPath { pave_heat } => pave_heat,
        _ => 0,
    };
    let slot = |i: usize| i16::from(stack.s0_companions.get(i).map_or(0, |c| c.code_u8()));
    let flags = (record.flags & !0x3) | i32::from(max_z.min(3));
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        UPSERT_SQL,
        &[
            &record.gx,
            &record.gy,
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
            &record.building_id,
            &record.roll_day,
            &record.weather_epoch,
            &record.explored_day,
            &flags,
            &record.charge,
            &record.stamp,
            &record.ver,
        ],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// 單列原子樂觀鎖寫入：`expected_ver == 0` 代表原生格首次落盤（`ver = 1`），
/// `expected_ver > 0` 代表比對版本號成功才覆寫並將 `ver` 遞增 `1`；版本不符回傳 `Ok(false)`。
pub fn cas_write_overlay_record(
    pool: &DbPool,
    record: &CellOverlayRecord,
    expected_ver: i32,
) -> Result<bool, String> {
    if expected_ver < 0 {
        return Err(format!("非法樂觀鎖版本 expected_ver={expected_ver}"));
    }
    let stack = &record.stack;
    let (mask, open, max_z) = match stack.v1 {
        V1Plugin::ArcadeShophouse {
            arcade_mask,
            shop_open,
            max_z,
        } => (arcade_mask, shop_open, max_z),
        _ => (0, false, 0),
    };
    let heat = match stack.s0_pave {
        S0Pavement::BambooTrampledPath { pave_heat } => pave_heat,
        _ => 0,
    };
    let slot = |i: usize| i16::from(stack.s0_companions.get(i).map_or(0, |c| c.code_u8()));
    let flags = (record.flags & !0x3) | i32::from(max_z.min(3));
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let u2 = stack.u2.code_i16();
    let u1 = stack.u1.code_i16();
    let v1 = i16::from(stack.v1.code_u8());
    let mask_i16 = i16::from(mask);
    let pave = i16::from(stack.s0_pave.code_u8());
    let heat_i16 = i16::from(heat);
    let s0 = slot(0);
    let s1 = slot(1);
    let s2 = slot(2);
    let rows = if expected_ver == 0 {
        conn.execute(
            CAS_INSERT_SQL,
            &[
                &record.gx,
                &record.gy,
                &u2,
                &u1,
                &v1,
                &mask_i16,
                &open,
                &pave,
                &heat_i16,
                &s0,
                &s1,
                &s2,
                &record.building_id,
                &record.roll_day,
                &record.weather_epoch,
                &record.explored_day,
                &flags,
                &record.charge,
                &record.stamp,
            ],
        )
        .map_err(|e| e.to_string())?
    } else {
        conn.execute(
            CAS_UPDATE_SQL,
            &[
                &record.gx,
                &record.gy,
                &u2,
                &u1,
                &v1,
                &mask_i16,
                &open,
                &pave,
                &heat_i16,
                &s0,
                &s1,
                &s2,
                &record.building_id,
                &record.roll_day,
                &record.weather_epoch,
                &record.explored_day,
                &flags,
                &record.charge,
                &record.stamp,
                &expected_ver,
            ],
        )
        .map_err(|e| e.to_string())?
    };
    Ok(rows == 1)
}

/// 在單一 PostgreSQL Transaction 內同步提交 `cell_overlay` CAS 寫入（或回歸原生刪除）與
/// `entities.inventory`（`SELECT ... FOR UPDATE` ＋ `inventory_seq` 遞增），嚴格保證資源守恆。
pub fn commit_overlay_and_inventory_tx(
    pool: &DbPool,
    record: &CellOverlayRecord,
    expected_ver: i32,
    require_positive_charge: bool,
    delete_if_pristine: bool,
    actor_id: &str,
    inv_deltas: &[(&str, i32)],
) -> Result<OverlayInvTxResult, String> {
    if expected_ver < 0 {
        return Err(format!("非法樂觀鎖版本 expected_ver={expected_ver}"));
    }
    let stack = &record.stack;
    let (mask, open, max_z) = match stack.v1 {
        V1Plugin::ArcadeShophouse {
            arcade_mask,
            shop_open,
            max_z,
        } => (arcade_mask, shop_open, max_z),
        _ => (0, false, 0),
    };
    let heat = match stack.s0_pave {
        S0Pavement::BambooTrampledPath { pave_heat } => pave_heat,
        _ => 0,
    };
    let slot = |i: usize| i16::from(stack.s0_companions.get(i).map_or(0, |c| c.code_u8()));
    let flags = (record.flags & !0x3) | i32::from(max_z.min(3));
    let u2 = stack.u2.code_i16();
    let u1 = stack.u1.code_i16();
    let v1 = i16::from(stack.v1.code_u8());
    let mask_i16 = i16::from(mask);
    let pave = i16::from(stack.s0_pave.code_u8());
    let heat_i16 = i16::from(heat);
    let s0 = slot(0);
    let s1 = slot(1);
    let s2 = slot(2);

    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let mut tx = conn.transaction().map_err(|e| e.to_string())?;

    let (overlay_ok, new_ver) = if delete_if_pristine {
        if expected_ver == 0 {
            (true, 0)
        } else {
            let n = tx
                .execute(
                    "DELETE FROM cell_overlay WHERE gx=$1 AND gy=$2 AND ver=$3",
                    &[&record.gx, &record.gy, &expected_ver],
                )
                .map_err(|e| e.to_string())?;
            (n == 1, 0)
        }
    } else if expected_ver == 0 {
        let n = tx
            .execute(
                CAS_INSERT_SQL,
                &[
                    &record.gx,
                    &record.gy,
                    &u2,
                    &u1,
                    &v1,
                    &mask_i16,
                    &open,
                    &pave,
                    &heat_i16,
                    &s0,
                    &s1,
                    &s2,
                    &record.building_id,
                    &record.roll_day,
                    &record.weather_epoch,
                    &record.explored_day,
                    &flags,
                    &record.charge,
                    &record.stamp,
                ],
            )
            .map_err(|e| e.to_string())?;
        (n == 1, 1)
    } else {
        let sql = if require_positive_charge {
            CAS_UPDATE_POSITIVE_CHARGE_SQL
        } else {
            CAS_UPDATE_SQL
        };
        let n = tx
            .execute(
                sql,
                &[
                    &record.gx,
                    &record.gy,
                    &u2,
                    &u1,
                    &v1,
                    &mask_i16,
                    &open,
                    &pave,
                    &heat_i16,
                    &s0,
                    &s1,
                    &s2,
                    &record.building_id,
                    &record.roll_day,
                    &record.weather_epoch,
                    &record.explored_day,
                    &flags,
                    &record.charge,
                    &record.stamp,
                    &expected_ver,
                ],
            )
            .map_err(|e| e.to_string())?;
        (n == 1, expected_ver + 1)
    };

    if !overlay_ok {
        return Ok(OverlayInvTxResult::Conflict);
    }

    let row = tx
        .query_opt(
            "SELECT inventory, inventory_seq FROM entities WHERE id=$1 FOR UPDATE",
            &[&actor_id],
        )
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("實體不存在：{actor_id}"))?;
    let cur_inv: String = row.get(0);
    let cur_seq: i64 = row.get(1);

    let (new_inventory, new_inventory_seq) = if inv_deltas.is_empty() {
        (cur_inv, cur_seq)
    } else {
        let next_inv = apply_inventory_deltas(&cur_inv, inv_deltas)?;
        let next_seq = cur_seq.saturating_add(1);
        tx.execute(
            "UPDATE entities SET inventory=$1, inventory_seq=$2 WHERE id=$3",
            &[&next_inv, &next_seq, &actor_id],
        )
        .map_err(|e| e.to_string())?;
        (next_inv, next_seq)
    };

    tx.commit().map_err(|e| e.to_string())?;
    Ok(OverlayInvTxResult::Committed {
        new_ver,
        new_inventory,
        new_inventory_seq,
    })
}

/// 回收已回歸原生推導狀態之髒格列。
pub fn delete_overlay_record(pool: &DbPool, gx: i32, gy: i32) -> Result<bool, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM cell_overlay WHERE gx=$1 AND gy=$2",
        &[&gx, &gy],
    )
    .map(|n| n == 1)
    .map_err(|e| e.to_string())
}

/// 寫入或更新單格髒格覆寫（相容包裝）。
pub fn upsert_slot_overlay(
    pool: &DbPool,
    gx: i32,
    gy: i32,
    stack: &SlotStack,
    explored_day: i32,
) -> Result<(), String> {
    let mut rec = CellOverlayRecord::from_stack(gx, gy, stack.clone());
    rec.explored_day = explored_day;
    upsert_overlay_record(pool, &rec)
}

/// 髒格總數（`/api/substrate/meta` 使用）。
pub fn count_dirty_cells(pool: &DbPool) -> i64 {
    let Ok(mut conn) = pool.get() else { return 0 };
    conn.query_one("SELECT COUNT(*) FROM cell_overlay", &[])
        .map(|r| r.get::<_, i64>(0))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::sql::init_pool;

    #[test]
    fn overlay_record_roundtrip_and_cas_on_pg_when_available() {
        let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:singularity@localhost:5432/singularity".to_string()
        });
        let Ok(pool) = init_pool(&url) else {
            return;
        };
        if pool.get().is_err() {
            return;
        }
        assert!(ensure_schema(&pool).is_ok());
        let (gx, gy) = (-2_000_042, -2_000_042);
        let _ = delete_overlay_record(&pool, gx, gy);

        let stack = SlotStack {
            u2: U2Plugin::DeepPile { depth_dm: 60 },
            u1: U1Plugin::ThornyBambooRhizome,
            s0_pave: S0Pavement::BambooTrampledPath { pave_heat: 77 },
            s0_companions: vec![S0Companion::BambooSheath, S0Companion::DryBambooLeaf],
            v1: V1Plugin::ArcadeShophouse {
                arcade_mask: 0b0101,
                shop_open: true,
                max_z: 2,
            },
        };
        let mut rec = CellOverlayRecord::from_stack(gx, gy, stack.clone());
        rec.building_id = Some("bld_test_3_0".to_string());
        rec.roll_day = 12;
        rec.weather_epoch = 34;
        rec.charge = 3;
        rec.stamp = 560;

        assert_eq!(cas_write_overlay_record(&pool, &rec, 0), Ok(true));
        assert_eq!(cas_write_overlay_record(&pool, &rec, 0), Ok(false));

        let loaded = load_overlay_record(&pool, gx, gy);
        assert!(loaded.is_some());
        if let Some(got) = loaded {
            assert_eq!(got.stack, stack);
            assert_eq!(got.building_id.as_deref(), Some("bld_test_3_0"));
            assert_eq!(got.roll_day, 12);
            assert_eq!(got.weather_epoch, 34);
            assert_eq!(got.charge, 3);
            assert_eq!(got.stamp, 560);
            assert_eq!(got.ver, 1);
        }

        rec.charge = 2;
        rec.stamp = 565;
        assert_eq!(cas_write_overlay_record(&pool, &rec, 1), Ok(true));
        assert_eq!(cas_write_overlay_record(&pool, &rec, 1), Ok(false));

        rec.charge = 0;
        rec.stamp = 570;
        assert_eq!(cas_write_overlay_record(&pool, &rec, 2), Ok(true));

        let win = load_window_records(&pool, gx, gx, gy, gy);
        assert_eq!(win.len(), 1);
        assert_eq!(win[0].charge, 0);
        assert_eq!(win[0].ver, 3);

        assert_eq!(delete_overlay_record(&pool, gx, gy), Ok(true));
        assert!(load_overlay_record(&pool, gx, gy).is_none());
    }

    #[test]
    fn overlay_and_inventory_tx_is_atomic_and_enforces_cas_on_pg() {
        let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:singularity@localhost:5432/singularity".to_string()
        });
        let Ok(pool) = init_pool(&url) else {
            return;
        };
        if pool.get().is_err() {
            return;
        }
        assert!(ensure_schema(&pool).is_ok());
        let (gx, gy) = (-2_000_088, -2_000_088);
        let test_actor = "__tx_test_actor_3b__";
        let _ = delete_overlay_record(&pool, gx, gy);
        if let Ok(mut conn) = pool.get() {
            let _ = conn.execute("DELETE FROM entities WHERE id=$1", &[&test_actor]);
            let ins = conn.execute(
                "INSERT INTO entities (id, kind, display_char, x, y, move_state, walk_or_run, vit, qi, dex, magnesium, created_at, gender, display_title, activated_nodes, equipment_slots, inventory, disposition, current_activity, grid_z, inventory_seq)
                 VALUES ($1, 'npc', '測', 0, 0, 'idle', 'walk', 100, 100, 100, 0, 0, 'M', '測', '[]', '{}', '[]', 0, '', 0, 0)",
                &[&test_actor],
            );
            if ins.is_err() {
                return;
            }
        }

        let mut rec = CellOverlayRecord::from_stack(gx, gy, SlotStack::default());
        rec.charge = 2;
        rec.roll_day = 5;

        // 1. 首次提交：cell_overlay ver 0->1 且背包加 2 根刺竹稈、inventory_seq 0->1
        let r1 = commit_overlay_and_inventory_tx(
            &pool,
            &rec,
            0,
            false,
            false,
            test_actor,
            &[("zl_item_bamboo_culm", 2)],
        );
        assert!(matches!(
            r1,
            Ok(OverlayInvTxResult::Committed {
                new_ver: 1,
                new_inventory_seq: 1,
                ..
            })
        ));

        // 2. CAS 版本過舊衝突 -> Conflict，背包不得重複進貨
        let r_conflict = commit_overlay_and_inventory_tx(
            &pool,
            &rec,
            0,
            false,
            false,
            test_actor,
            &[("zl_item_bamboo_culm", 2)],
        );
        assert_eq!(r_conflict, Ok(OverlayInvTxResult::Conflict));

        // 3. 扣減背包不足 -> Err，且整筆交易回滾，cell_overlay ver 維持 1
        rec.charge = 1;
        let r_underflow = commit_overlay_and_inventory_tx(
            &pool,
            &rec,
            1,
            true,
            false,
            test_actor,
            &[("zl_item_dry_leaves", -1)],
        );
        assert!(r_underflow.is_err());
        assert_eq!(load_overlay_record(&pool, gx, gy).map(|o| o.ver), Some(1));

        // 清理測試資料
        let _ = delete_overlay_record(&pool, gx, gy);
        if let Ok(mut conn) = pool.get() {
            let _ = conn.execute("DELETE FROM entities WHERE id=$1", &[&test_actor]);
        }
    }
}
