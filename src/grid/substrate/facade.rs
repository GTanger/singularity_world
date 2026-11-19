//! `CellFacade` 雙讀骨架：把「底板 + 髒格覆寫 + 槽位推導 + 即時合成」收斂為單一快照入口，
//! 並提供舊 `SquareGrid` 與新底板的四錨點雙讀核驗。

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use super::arcade_street::can_step_horizontal;
use super::cache::{
    apply_pave_heat_delta, drain_pending_footsteps, enqueue_footstep, get_cached_overlay,
    invalidate_chunk, put_cached_overlay, remove_cached_overlay,
};
use super::coords::{DerivedSlope, SensoryEdgeCue};
use super::l0_cell::L0Cell;
use super::render_synth::{SynthesizedCellView, synthesize_cell};
use super::runtime;
use super::slots::{FieldState, S0Pavement, SlotStack, V1Plugin, derive_pristine_slots};
use crate::grid::SquareCoord;
use crate::pg::cell_overlay::{
    CellOverlayRecord, cas_write_overlay_record, delete_overlay_record, load_overlay_record,
};
use crate::store::sql::DbPool;

/// 單格完整快照。
#[derive(Debug, Clone, PartialEq)]
pub struct CellSnapshot {
    pub gx: i32,
    pub gy: i32,
    pub l0: L0Cell,
    pub slope: Option<DerivedSlope>,
    pub edge_cue: Option<SensoryEdgeCue>,
    pub slots: SlotStack,
    pub overlay: Option<CellOverlayRecord>,
    pub synth: SynthesizedCellView,
    pub max_z: u8,
    pub step_ms: u16,
}

/// 單格步頻（毫秒）：鋪面與騎樓較快、刺竹叢較慢，其餘可走平地居中。
pub fn step_delay_ms(slots: &SlotStack) -> u16 {
    if matches!(
        slots.s0_pave,
        S0Pavement::BambooTrampledPath { .. } | S0Pavement::StoneStreet | S0Pavement::NarrowRailBed
    ) || matches!(slots.v1, V1Plugin::ArcadeShophouse { .. })
    {
        220
    } else if matches!(slots.v1, V1Plugin::ThornyBambooClump) {
        420
    } else {
        280
    }
}

/// 有效樓層上限：僅騎樓街屋可上樓，且受底板承載上限約束。
pub fn effective_max_z(l0: L0Cell, slots: &SlotStack) -> u8 {
    match slots.v1 {
        V1Plugin::ArcadeShophouse { max_z, .. } => max_z.min(SlotStack::max_supported_z(l0)),
        _ => 0,
    }
}

/// 雙讀門面。
pub struct CellFacade;

impl CellFacade {
    /// 由已備妥的各部件組裝快照（純記憶體，不碰資料庫與全域底板）。
    #[expect(clippy::too_many_arguments)]
    pub fn snapshot_from_parts(
        gx: i32,
        gy: i32,
        l0: L0Cell,
        slope: Option<DerivedSlope>,
        edge_cue: Option<SensoryEdgeCue>,
        overlay: Option<CellOverlayRecord>,
        field: FieldState,
        world_seed: u64,
    ) -> CellSnapshot {
        let slots = match &overlay {
            Some(record) => record.stack.clone(),
            None => derive_pristine_slots(gx, gy, l0, world_seed),
        };
        let synth = synthesize_cell(gx, gy, l0, slope, edge_cue.clone(), &slots, field);
        let max_z = effective_max_z(l0, &slots);
        let step_ms = step_delay_ms(&slots);
        CellSnapshot {
            gx,
            gy,
            l0,
            slope,
            edge_cue,
            slots,
            overlay,
            synth,
            max_z,
            step_ms,
        }
    }

    /// 以全域底板與（可選）`32×32` 分片正負雙快取取得單格快照；底板未載入或越界時回退為海洋無資料格。
    pub fn snapshot_live(
        pool: Option<&DbPool>,
        gx: i32,
        gy: i32,
        field: FieldState,
    ) -> CellSnapshot {
        Self::snapshot_cached(pool, gx, gy, field)
    }

    /// 走 `32×32` 分片正負雙快取讀取單格快照（單一空間讀取權威）。
    pub fn snapshot_cached(
        pool: Option<&DbPool>,
        gx: i32,
        gy: i32,
        field: FieldState,
    ) -> CellSnapshot {
        let rt = runtime::global();
        let l0 = rt
            .and_then(|r| r.l0(gx, gy))
            .unwrap_or_else(L0Cell::sea_nodata);
        let slope = rt.and_then(|r| r.slope(gx, gy));
        let edge_cue = rt.and_then(|r| r.edge_cue(gx, gy));
        let overlay = pool.and_then(|p| get_cached_overlay(p, gx, gy));
        Self::snapshot_from_parts(
            gx,
            gy,
            l0,
            slope,
            edge_cue,
            overlay,
            field,
            runtime::WORLD_SEED,
        )
    }

    /// 單一空間稀疏寫入權威：以單列樂觀鎖 CAS 寫入 `cell_overlay` 並寫穿更新 `32×32` 分片快取。
    ///
    /// - 若修改器回傳 `Ok(false)` 表示無須變動，直接回傳當前快照。
    /// - 若修改後槽位與輔助欄位完全回歸原生狀態，自動 `DELETE` 回收髒列並維持負快取。
    /// - 版本衝突時自動失效該分片並重讀重試（最多 3 次）。
    pub fn cas_mutate_cell<F>(
        pool: &DbPool,
        gx: i32,
        gy: i32,
        field: FieldState,
        mut mutator: F,
    ) -> Result<CellSnapshot, String>
    where
        F: FnMut(&mut CellOverlayRecord, L0Cell) -> Result<bool, String>,
    {
        const MAX_RETRIES: usize = 3;
        let rt = runtime::global();
        let l0 = rt
            .and_then(|r| r.l0(gx, gy))
            .unwrap_or_else(L0Cell::sea_nodata);
        let slope = rt.and_then(|r| r.slope(gx, gy));
        let edge_cue = rt.and_then(|r| r.edge_cue(gx, gy));
        let pristine = derive_pristine_slots(gx, gy, l0, runtime::WORLD_SEED);

        for attempt in 0..MAX_RETRIES {
            let existing = if attempt == 0 {
                get_cached_overlay(pool, gx, gy)
            } else {
                load_overlay_record(pool, gx, gy)
            };
            let expected_ver = existing.as_ref().map_or(0, |r| r.ver);
            let mut next = existing
                .clone()
                .unwrap_or_else(|| CellOverlayRecord::from_stack(gx, gy, pristine.clone()));

            let changed = mutator(&mut next, l0)?;
            if !changed {
                return Ok(Self::snapshot_from_parts(
                    gx,
                    gy,
                    l0,
                    slope,
                    edge_cue,
                    existing,
                    field,
                    runtime::WORLD_SEED,
                ));
            }

            let reverted_to_pristine = next.stack == pristine
                && next.building_id.is_none()
                && next.roll_day == 0
                && next.weather_epoch == 0
                && next.flags == 0
                && next.charge == -1
                && next.stamp == 0;

            if reverted_to_pristine {
                if expected_ver > 0 {
                    let _ = delete_overlay_record(pool, gx, gy)?;
                }
                remove_cached_overlay(gx, gy);
                return Ok(Self::snapshot_from_parts(
                    gx,
                    gy,
                    l0,
                    slope,
                    edge_cue,
                    None,
                    field,
                    runtime::WORLD_SEED,
                ));
            }

            if cas_write_overlay_record(pool, &next, expected_ver)? {
                next.ver = expected_ver + 1;
                put_cached_overlay(next.clone());
                return Ok(Self::snapshot_from_parts(
                    gx,
                    gy,
                    l0,
                    slope,
                    edge_cue,
                    Some(next),
                    field,
                    runtime::WORLD_SEED,
                ));
            }
            invalidate_chunk(gx, gy);
        }
        Err(format!("單格 ({gx},{gy}) 樂觀鎖寫入衝突超限"))
    }

    /// 記錄活人實踏一格（`pave_heat` 2 秒／256 格合併寫入；純 Look 嚴禁呼叫）。
    pub fn record_footstep(pool: Option<&DbPool>, gx: i32, gy: i32) {
        let Some(batch) = enqueue_footstep(gx, gy) else {
            return;
        };
        if let Some(p) = pool {
            Self::flush_pave_batch(p, batch);
        }
    }

    /// 刷寫待合併之 `pave_heat` 緩衝。
    pub fn flush_pave_heat(pool: &DbPool, force: bool) -> usize {
        let batch = drain_pending_footsteps(force);
        let n = batch.len();
        if n > 0 {
            Self::flush_pave_batch(pool, batch);
        }
        n
    }

    fn flush_pave_batch(pool: &DbPool, batch: Vec<((i32, i32), u8)>) {
        for ((gx, gy), delta) in batch {
            let _ = Self::cas_mutate_cell(pool, gx, gy, FieldState::default(), |rec, l0| {
                Ok(apply_pave_heat_delta(l0, &mut rec.stack, delta))
            });
        }
    }

    /// 判定 `z = 0` 平面上是否可從 `(from_gx, from_gy)` 單步正交移至 `(to_gx, to_gy)`。
    pub fn can_step(
        pool: Option<&DbPool>,
        from_gx: i32,
        from_gy: i32,
        to_gx: i32,
        to_gy: i32,
        field: FieldState,
    ) -> bool {
        let from = Self::snapshot_cached(pool, from_gx, from_gy, field);
        let to = Self::snapshot_cached(pool, to_gx, to_gy, field);
        can_step_horizontal(&from, &to)
    }

    /// 有界加權尋路（以 `step_delay_ms` 為權重、嚴守 `can_step_horizontal` 防穿牆與陸塊短路）。
    pub fn plan_path(
        pool: Option<&DbPool>,
        start: (i32, i32),
        goal: (i32, i32),
        field: FieldState,
        max_radius: i32,
    ) -> Option<Vec<(i32, i32)>> {
        const MAX_EXPANDED: usize = 4_096;
        if start == goal {
            return Some(vec![start]);
        }
        let dist_cheb = (goal.0 - start.0).abs().max((goal.1 - start.1).abs());
        if dist_cheb > max_radius {
            return None;
        }
        let start_snap = Self::snapshot_cached(pool, start.0, start.1, field);
        let goal_snap = Self::snapshot_cached(pool, goal.0, goal.1, field);
        if start_snap.l0.landmass_id != goal_snap.l0.landmass_id
            || !goal_snap.synth.walkable_on_landmass_01
        {
            return None;
        }

        let mut snaps: HashMap<(i32, i32), CellSnapshot> = HashMap::new();
        snaps.insert(start, start_snap);
        snaps.insert(goal, goal_snap);

        let mut dist: HashMap<(i32, i32), u32> = HashMap::new();
        let mut prev: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
        let mut heap: BinaryHeap<Reverse<(u32, i32, i32)>> = BinaryHeap::new();
        dist.insert(start, 0);
        heap.push(Reverse((0, start.0, start.1)));

        let mut expanded = 0usize;
        while let Some(Reverse((cost, cx, cy))) = heap.pop() {
            if (cx, cy) == goal {
                let mut path = vec![goal];
                let mut cur = goal;
                while let Some(&p) = prev.get(&cur) {
                    path.push(p);
                    cur = p;
                }
                path.reverse();
                return Some(path);
            }
            if cost > *dist.get(&(cx, cy)).unwrap_or(&u32::MAX) {
                continue;
            }
            expanded += 1;
            if expanded > MAX_EXPANDED {
                return None;
            }
            let cur_snap = snaps
                .entry((cx, cy))
                .or_insert_with(|| Self::snapshot_cached(pool, cx, cy, field))
                .clone();

            for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
                let nx = cx + dx;
                let ny = cy + dy;
                if (nx - start.0).abs().max((ny - start.1).abs()) > max_radius {
                    continue;
                }
                let next_snap = snaps
                    .entry((nx, ny))
                    .or_insert_with(|| Self::snapshot_cached(pool, nx, ny, field));
                if !can_step_horizontal(&cur_snap, next_snap) {
                    continue;
                }
                let next_cost = cost.saturating_add(u32::from(next_snap.step_ms));
                if next_cost < *dist.get(&(nx, ny)).unwrap_or(&u32::MAX) {
                    dist.insert((nx, ny), next_cost);
                    prev.insert((nx, ny), (cx, cy));
                    heap.push(Reverse((next_cost, nx, ny)));
                }
            }
        }
        None
    }
}

/// 單一錨點的新舊雙讀結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorDualRead {
    pub gx: i32,
    pub gy: i32,
    pub legacy_walkable: bool,
    pub substrate_walkable: bool,
    pub micro_name: &'static str,
    pub v1: V1Plugin,
}

const ANCHORS: [(i32, i32); 4] = [(0, 0), (0, 1), (1, 0), (1, -1)];

/// 核驗錨點主構是否與約定一致。
fn anchor_matches(gx: i32, gy: i32, micro_name: &str, v1: V1Plugin) -> bool {
    match (gx, gy) {
        (0, 0) => micro_name.contains("卵石灘"),
        (0, 1) => micro_name.contains("冷泉"),
        (1, 0) => v1 == V1Plugin::ThornyBambooClump,
        (1, -1) => v1 == V1Plugin::AncientCamphorCanopy,
        _ => false,
    }
}

/// 以指定快照來源核驗四錨點（`snapshot` 可注入純記憶體實作）。
fn verify_with(
    square_grid: &crate::grid::SquareGrid,
    snapshot: impl Fn(i32, i32) -> CellSnapshot,
) -> Result<Vec<AnchorDualRead>, String> {
    let mut out = Vec::with_capacity(ANCHORS.len());
    for (gx, gy) in ANCHORS {
        let legacy_walkable = square_grid
            .get(SquareCoord::new(gx, gy))
            .is_some_and(|cell| cell.terrain.walkable());
        let snap = snapshot(gx, gy);
        let read = AnchorDualRead {
            gx,
            gy,
            legacy_walkable,
            substrate_walkable: snap.synth.walkable_on_landmass_01,
            micro_name: snap.synth.micro_name,
            v1: snap.slots.v1,
        };
        if !read.legacy_walkable {
            return Err(format!("錨點 ({gx},{gy}) 舊格網不可走"));
        }
        if !read.substrate_walkable {
            return Err(format!("錨點 ({gx},{gy}) 新底板不可走"));
        }
        if !anchor_matches(gx, gy, read.micro_name, read.v1) {
            return Err(format!("錨點 ({gx},{gy}) 主構不一致：{}", read.micro_name));
        }
        out.push(read);
    }
    Ok(out)
}

/// 四錨點雙讀核驗：`(0,0)`、`(0,1)`、`(1,0)`、`(1,-1)` 新舊兩側皆可走且主構一致。
pub fn verify_four_anchors(
    square_grid: &crate::grid::SquareGrid,
) -> Result<Vec<AnchorDualRead>, String> {
    if runtime::global().is_none() {
        return Err("全域底板尚未載入".to_string());
    }
    verify_with(square_grid, |gx, gy| {
        CellFacade::snapshot_live(None, gx, gy, FieldState::default())
    })
}

/// 直接以真實竹林舊格網與全域 `SWL0` 底板執行四錨點雙讀對帳。
pub fn verify_four_anchors_shalu_zhulin() -> Result<Vec<AnchorDualRead>, String> {
    let legacy_grid = crate::grid::shalu_zhulin::generate_shalu_zhulin_grid();
    verify_four_anchors(&legacy_grid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::l0_cell::{SubstrateMaterial, SurfaceWaterKind, landmass};
    use crate::grid::{GridCell, SquareGrid, Terrain};

    fn land(m: SubstrateMaterial, w: SurfaceWaterKind) -> L0Cell {
        L0Cell::new(Some(5.0), landmass::DAJIA_WU_TABLELAND, m, w, 40.0, 90)
    }

    fn plain_grid() -> SquareGrid {
        let mut grid = SquareGrid::new();
        for (x, y) in ANCHORS {
            grid.insert(GridCell::new(
                SquareCoord::new(x, y),
                Terrain::Plain,
                "測試格",
            ));
        }
        grid
    }

    #[test]
    fn step_delay_and_max_z_rules() {
        let mut s = SlotStack::default();
        assert_eq!(step_delay_ms(&s), 280);
        s.v1 = V1Plugin::ThornyBambooClump;
        assert_eq!(step_delay_ms(&s), 420);
        s.v1 = V1Plugin::ArcadeShophouse {
            arcade_mask: 1,
            shop_open: true,
            max_z: 9,
        };
        assert_eq!(step_delay_ms(&s), 220);
        let lat = land(SubstrateMaterial::LateriteGravel, SurfaceWaterKind::None);
        assert_eq!(effective_max_z(lat, &s), 3);
        let loam = land(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::None);
        assert_eq!(effective_max_z(loam, &s), 2);
        assert_eq!(effective_max_z(loam, &SlotStack::default()), 0);
        let road = SlotStack {
            s0_pave: S0Pavement::StoneStreet,
            ..SlotStack::default()
        };
        assert_eq!(step_delay_ms(&road), 220);
    }

    #[test]
    fn overlay_takes_precedence_over_pristine() {
        let l0 = land(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::None);
        let stack = SlotStack {
            v1: V1Plugin::AncientCamphorCanopy,
            ..SlotStack::default()
        };
        let rec = CellOverlayRecord::from_stack(1, 0, stack.clone());
        let snap = CellFacade::snapshot_from_parts(
            1,
            0,
            l0,
            None,
            None,
            Some(rec),
            FieldState::default(),
            runtime::WORLD_SEED,
        );
        assert_eq!(snap.slots, stack);
        let pristine = CellFacade::snapshot_from_parts(
            1,
            0,
            l0,
            None,
            None,
            None,
            FieldState::default(),
            runtime::WORLD_SEED,
        );
        assert_eq!(pristine.slots.v1, V1Plugin::ThornyBambooClump);
        assert_eq!(pristine.step_ms, 420);
    }

    #[test]
    fn four_anchors_in_memory() {
        let grid = plain_grid();
        let reads = verify_with(&grid, |gx, gy| {
            let water = if (gx, gy) == (0, 1) {
                SurfaceWaterKind::FaultSpring
            } else {
                SurfaceWaterKind::None
            };
            let l0 = land(SubstrateMaterial::AlluvialSandyLoam, water);
            CellFacade::snapshot_from_parts(
                gx,
                gy,
                l0,
                None,
                None,
                None,
                FieldState::default(),
                runtime::WORLD_SEED,
            )
        });
        let reads = reads.unwrap_or_default();
        assert_eq!(reads.len(), 4);
        assert!(
            reads
                .iter()
                .all(|r| r.legacy_walkable && r.substrate_walkable)
        );
    }

    #[test]
    fn four_anchors_reject_unwalkable_legacy() {
        let grid = SquareGrid::new();
        let result = verify_with(&grid, |gx, gy| {
            let l0 = land(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::None);
            CellFacade::snapshot_from_parts(
                gx,
                gy,
                l0,
                None,
                None,
                None,
                FieldState::default(),
                runtime::WORLD_SEED,
            )
        });
        assert!(result.is_err());
    }

    #[test]
    fn four_anchors_real_runtime_when_available() {
        let swl0_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/runtime/landmass_01_substrate.swl0");
        if swl0_path.is_file() {
            assert!(runtime::global().is_some(), "底板檔案存在但載入失敗");
        }
        if runtime::global().is_none() {
            return;
        }
        let reads = verify_four_anchors_shalu_zhulin();
        assert!(reads.is_ok(), "{reads:?}");
        assert_eq!(reads.unwrap_or_default().len(), 4);
    }

    #[test]
    fn plan_path_routes_around_back_to_back_arcade_walls() {
        if runtime::global().is_none() {
            return;
        }
        let field = FieldState::default();
        assert!(
            !CellFacade::can_step(None, -4, -1, -4, -2, field),
            "背對背街屋 (-4,-1) -> (-4,-2) 必須實牆封死"
        );
        assert!(
            CellFacade::can_step(None, -4, -1, -3, -1, field),
            "同排亭仔腳 (-4,-1) -> (-3,-1) 必須雙向連通"
        );
        let path = CellFacade::plan_path(None, (-4, -1), (-4, -2), field, 24);
        let Some(steps) = path else {
            panic!("應能繞行石板街與南背巷抵達 (-4,-2)");
        };
        assert!(
            steps.len() > 2,
            "不得直接穿牆，實際步數={}",
            steps.len() - 1
        );
        for pair in steps.windows(2) {
            assert!(
                CellFacade::can_step(None, pair[0].0, pair[0].1, pair[1].0, pair[1].1, field),
                "路徑包含非法穿牆步：{:?} -> {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn cas_mutate_and_cache_roundtrip_on_pg_when_available() {
        let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:singularity@localhost:5432/singularity".to_string()
        });
        let Ok(pool) = crate::store::sql::init_pool(&url) else {
            return;
        };
        if pool.get().is_err() {
            return;
        }
        assert!(crate::pg::cell_overlay::ensure_schema(&pool).is_ok());
        let (gx, gy) = (-2_000_077, -2_000_077);
        let _ = delete_overlay_record(&pool, gx, gy);
        super::super::cache::clear_overlay_cache();

        let field = FieldState::default();
        // 1. 原生格負快取讀取
        let snap0 = CellFacade::snapshot_cached(Some(&pool), gx, gy, field);
        assert!(snap0.overlay.is_none());

        // 2. CAS 寫入踏熱並寫穿快取
        let snap1 = CellFacade::cas_mutate_cell(&pool, gx, gy, field, |rec, _| {
            rec.stack.s0_pave = S0Pavement::BambooTrampledPath { pave_heat: 24 };
            Ok(true)
        });
        let snap1 = snap1.expect("首次 CAS 寫入應成功");
        assert_eq!(snap1.overlay.as_ref().map(|o| o.ver), Some(1));
        assert_eq!(
            snap1.slots.s0_pave,
            S0Pavement::BambooTrampledPath { pave_heat: 24 }
        );

        // 3. 寫穿後直接命中快取
        let cached = CellFacade::snapshot_cached(Some(&pool), gx, gy, field);
        assert_eq!(cached.overlay.as_ref().map(|o| o.ver), Some(1));

        // 4. 回歸原生狀態應自動 DELETE 並清除髒格列
        let snap2 = CellFacade::cas_mutate_cell(&pool, gx, gy, field, |rec, l0| {
            rec.stack = derive_pristine_slots(gx, gy, l0, runtime::WORLD_SEED);
            Ok(true)
        });
        let snap2 = snap2.expect("回歸原生刪除應成功");
        assert!(snap2.overlay.is_none());
        assert!(load_overlay_record(&pool, gx, gy).is_none());
    }
}
