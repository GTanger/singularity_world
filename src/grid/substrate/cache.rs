//! `32×32` 分片正負雙快取與 `pave_heat` 合併寫入緩衝（微步 3a′）。
//!
//! - **正負雙快取**：單次查詢整塊 `32×32` 區塊的 `cell_overlay`，查不到的格即負快取（原生未覆寫格），
//!   避免每次移動對數百萬原生格重複敲擊 PostgreSQL。
//! - **`pave_heat` 合併寫入**：活人實際踏入時寫入緩衝，累積達 `256` 格或間隔逾 `2` 秒時批次合併 CAS 落盤；
//!   純 Look 觀察絕不刷新 `pave_heat`。

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant};

use super::l0_cell::{L0Cell, SurfaceWaterKind, landmass};
use super::slots::{S0Pavement, SlotStack, V1Plugin};
use crate::pg::cell_overlay::{CellOverlayRecord, load_window_records};
use crate::store::sql::DbPool;

/// 分片邊長（32×32 格）。
pub const OVERLAY_CHUNK_SIDE: i32 = 32;
/// 記憶體常駐分片上限（防無界膨脹）。
const MAX_CACHED_CHUNKS: usize = 256;
/// `pave_heat` 合併刷寫批量閾值。
pub const PAVE_HEAT_FLUSH_BATCH: usize = 256;
/// `pave_heat` 合併刷寫時間閾值（2 秒）。
pub const PAVE_HEAT_FLUSH_INTERVAL: Duration = Duration::from_secs(2);
/// 單次活人踏入增加之踏熱量。
const FOOTSTEP_HEAT_DELTA: u8 = 8;

/// 將世界格座標對應至 `32×32` 分片鍵（歐幾里得除法，負座標不偏移）。
#[inline]
pub fn chunk_key(gx: i32, gy: i32) -> (i32, i32) {
    (
        gx.div_euclid(OVERLAY_CHUNK_SIDE),
        gy.div_euclid(OVERLAY_CHUNK_SIDE),
    )
}

#[derive(Debug, Clone)]
struct ChunkEntry {
    records: HashMap<(i32, i32), CellOverlayRecord>,
    last_touch: u64,
}

#[derive(Debug, Default)]
struct ChunkCacheState {
    chunks: HashMap<(i32, i32), ChunkEntry>,
    seq: u64,
}

impl ChunkCacheState {
    fn touch_seq(&mut self) -> u64 {
        self.seq = self.seq.wrapping_add(1);
        self.seq
    }

    fn evict_if_full(&mut self) {
        while self.chunks.len() >= MAX_CACHED_CHUNKS {
            let oldest = self
                .chunks
                .iter()
                .min_by_key(|(_, v)| v.last_touch)
                .map(|(k, _)| *k);
            match oldest {
                Some(k) => {
                    self.chunks.remove(&k);
                }
                None => break,
            }
        }
    }
}

static OVERLAY_CACHE: LazyLock<RwLock<ChunkCacheState>> =
    LazyLock::new(|| RwLock::new(ChunkCacheState::default()));

/// 從 `32×32` 正負雙快取讀取單格髒格紀錄；未命中分片時整片自 PG 載入一次。
pub fn get_cached_overlay(pool: &DbPool, gx: i32, gy: i32) -> Option<CellOverlayRecord> {
    let ck = chunk_key(gx, gy);
    if let Ok(guard) = OVERLAY_CACHE.read()
        && let Some(entry) = guard.chunks.get(&ck)
    {
        return entry.records.get(&(gx, gy)).cloned();
    }

    let min_x = ck.0 * OVERLAY_CHUNK_SIDE;
    let max_x = min_x + OVERLAY_CHUNK_SIDE - 1;
    let min_y = ck.1 * OVERLAY_CHUNK_SIDE;
    let max_y = min_y + OVERLAY_CHUNK_SIDE - 1;
    let rows = load_window_records(pool, min_x, max_x, min_y, max_y);
    let mut map = HashMap::with_capacity(rows.len());
    for rec in rows {
        map.insert((rec.gx, rec.gy), rec);
    }
    let hit = map.get(&(gx, gy)).cloned();

    if let Ok(mut guard) = OVERLAY_CACHE.write() {
        let seq = guard.touch_seq();
        guard.evict_if_full();
        guard.chunks.insert(
            ck,
            ChunkEntry {
                records: map,
                last_touch: seq,
            },
        );
    }
    hit
}

/// 寫穿更新分片快取中的單格紀錄（若該分片已載入）。
pub fn put_cached_overlay(record: CellOverlayRecord) {
    let ck = chunk_key(record.gx, record.gy);
    if let Ok(mut guard) = OVERLAY_CACHE.write() {
        let seq = guard.touch_seq();
        if let Some(entry) = guard.chunks.get_mut(&ck) {
            entry.records.insert((record.gx, record.gy), record);
            entry.last_touch = seq;
        }
    }
}

/// 從分片快取中移除已回歸原生並 `DELETE` 的單格紀錄（維持負快取）。
pub fn remove_cached_overlay(gx: i32, gy: i32) {
    let ck = chunk_key(gx, gy);
    if let Ok(mut guard) = OVERLAY_CACHE.write() {
        let seq = guard.touch_seq();
        if let Some(entry) = guard.chunks.get_mut(&ck) {
            entry.records.remove(&(gx, gy));
            entry.last_touch = seq;
        }
    }
}

/// 使指定分片失效（例如 CAS 衝突重讀前強制失效）。
pub fn invalidate_chunk(gx: i32, gy: i32) {
    let ck = chunk_key(gx, gy);
    if let Ok(mut guard) = OVERLAY_CACHE.write() {
        guard.chunks.remove(&ck);
    }
}

/// 清空全部快取（供測試或重置使用）。
#[cfg(test)]
pub fn clear_overlay_cache() {
    if let Ok(mut guard) = OVERLAY_CACHE.write() {
        guard.chunks.clear();
    }
}

/// 判定單格是否適合累積活人踩踏熱度（排除深水、石板街、窄軌、騎樓街屋與未開陸塊）。
pub fn supports_pave_heat(l0: L0Cell, stack: &SlotStack) -> bool {
    if l0.landmass_id != landmass::DAJIA_WU_TABLELAND
        || l0.surface_water().is_deep_water()
        || l0.surface_water() == SurfaceWaterKind::NoData
    {
        return false;
    }
    if matches!(stack.v1, V1Plugin::ArcadeShophouse { .. }) {
        return false;
    }
    matches!(
        stack.s0_pave,
        S0Pavement::BareSoil | S0Pavement::BambooTrampledPath { .. }
    )
}

/// 將踏熱增量套用至槽位堆疊；若實際改變則回傳 `true`。
pub fn apply_pave_heat_delta(l0: L0Cell, stack: &mut SlotStack, delta: u8) -> bool {
    if delta == 0 || !supports_pave_heat(l0, stack) {
        return false;
    }
    let next_heat = match stack.s0_pave {
        S0Pavement::BareSoil => delta,
        S0Pavement::BambooTrampledPath { pave_heat } => pave_heat.saturating_add(delta),
        S0Pavement::StoneStreet | S0Pavement::NarrowRailBed => return false,
    };
    let next_pave = S0Pavement::BambooTrampledPath {
        pave_heat: next_heat,
    };
    if stack.s0_pave == next_pave {
        return false;
    }
    stack.s0_pave = next_pave;
    true
}

struct PaveHeatBuffer {
    pending: HashMap<(i32, i32), u8>,
    last_flush: Instant,
}

impl Default for PaveHeatBuffer {
    fn default() -> Self {
        Self {
            pending: HashMap::new(),
            last_flush: Instant::now(),
        }
    }
}

static PAVE_BUFFER: LazyLock<Mutex<PaveHeatBuffer>> =
    LazyLock::new(|| Mutex::new(PaveHeatBuffer::default()));

/// 記錄活人實際踏入一格，回傳若已達 2s 或 256 格閾值時待刷寫的批次。
pub fn enqueue_footstep(gx: i32, gy: i32) -> Option<Vec<((i32, i32), u8)>> {
    let mut guard = PAVE_BUFFER.lock().ok()?;
    let entry = guard.pending.entry((gx, gy)).or_insert(0);
    *entry = entry.saturating_add(FOOTSTEP_HEAT_DELTA);
    let due = guard.pending.len() >= PAVE_HEAT_FLUSH_BATCH
        || guard.last_flush.elapsed() >= PAVE_HEAT_FLUSH_INTERVAL;
    if due {
        guard.last_flush = Instant::now();
        Some(guard.pending.drain().collect())
    } else {
        None
    }
}

/// 強制抽出全部待刷寫踏熱（供週期 tick 或收尾刷盤使用）。
pub fn drain_pending_footsteps(force: bool) -> Vec<((i32, i32), u8)> {
    let Ok(mut guard) = PAVE_BUFFER.lock() else {
        return Vec::new();
    };
    if !force
        && guard.pending.len() < PAVE_HEAT_FLUSH_BATCH
        && guard.last_flush.elapsed() < PAVE_HEAT_FLUSH_INTERVAL
    {
        return Vec::new();
    }
    guard.last_flush = Instant::now();
    guard.pending.drain().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::l0_cell::SubstrateMaterial;

    #[test]
    fn chunk_key_uses_euclidean_division() {
        assert_eq!(chunk_key(0, 0), (0, 0));
        assert_eq!(chunk_key(31, 31), (0, 0));
        assert_eq!(chunk_key(32, 0), (1, 0));
        assert_eq!(chunk_key(-1, -1), (-1, -1));
        assert_eq!(chunk_key(-32, -32), (-1, -1));
        assert_eq!(chunk_key(-33, 0), (-2, 0));
    }

    #[test]
    fn pave_heat_only_updates_soil_and_bamboo_path() {
        let land = L0Cell::new(
            Some(30.0),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::AlluvialSandyLoam,
            SurfaceWaterKind::None,
            30.0,
            80,
        );
        let mut bare = SlotStack::default();
        assert!(apply_pave_heat_delta(land, &mut bare, 8));
        assert_eq!(
            bare.s0_pave,
            S0Pavement::BambooTrampledPath { pave_heat: 8 }
        );
        assert!(apply_pave_heat_delta(land, &mut bare, 250));
        assert_eq!(
            bare.s0_pave,
            S0Pavement::BambooTrampledPath { pave_heat: 255 }
        );
        assert!(!apply_pave_heat_delta(land, &mut bare, 8));

        let mut street = SlotStack {
            s0_pave: S0Pavement::StoneStreet,
            ..SlotStack::default()
        };
        assert!(!apply_pave_heat_delta(land, &mut street, 8));
    }
}
