//! `CellFacade` 雙讀骨架：把「底板 + 髒格覆寫 + 槽位推導 + 即時合成」收斂為單一快照入口，
//! 並提供舊 `SquareGrid` 與新底板的四錨點雙讀核驗。

use super::coords::{DerivedSlope, SensoryEdgeCue};
use super::l0_cell::L0Cell;
use super::render_synth::{SynthesizedCellView, synthesize_cell};
use super::runtime;
use super::slots::{FieldState, S0Pavement, SlotStack, V1Plugin, derive_pristine_slots};
use crate::grid::SquareCoord;
use crate::pg::cell_overlay::{CellOverlayRecord, load_overlay_record};
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
        CellSnapshot { gx, gy, l0, slope, edge_cue, slots, overlay, synth, max_z, step_ms }
    }

    /// 以全域底板與（可選）資料庫覆寫即時取得快照；底板未載入或越界時回退為海洋無資料格。
    pub fn snapshot_live(pool: Option<&DbPool>, gx: i32, gy: i32, field: FieldState) -> CellSnapshot {
        let rt = runtime::global();
        let l0 = rt.and_then(|r| r.l0(gx, gy)).unwrap_or_else(L0Cell::sea_nodata);
        let slope = rt.and_then(|r| r.slope(gx, gy));
        let edge_cue = rt.and_then(|r| r.edge_cue(gx, gy));
        let overlay = pool.and_then(|p| load_overlay_record(p, gx, gy));
        Self::snapshot_from_parts(gx, gy, l0, slope, edge_cue, overlay, field, runtime::WORLD_SEED)
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
pub fn verify_four_anchors(square_grid: &crate::grid::SquareGrid) -> Result<Vec<AnchorDualRead>, String> {
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
            grid.insert(GridCell::new(SquareCoord::new(x, y), Terrain::Plain, "測試格"));
        }
        grid
    }

    #[test]
    fn step_delay_and_max_z_rules() {
        let mut s = SlotStack::default();
        assert_eq!(step_delay_ms(&s), 280);
        s.v1 = V1Plugin::ThornyBambooClump;
        assert_eq!(step_delay_ms(&s), 420);
        s.v1 = V1Plugin::ArcadeShophouse { arcade_mask: 1, shop_open: true, max_z: 9 };
        assert_eq!(step_delay_ms(&s), 220);
        let lat = land(SubstrateMaterial::LateriteGravel, SurfaceWaterKind::None);
        assert_eq!(effective_max_z(lat, &s), 3);
        let loam = land(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::None);
        assert_eq!(effective_max_z(loam, &s), 2);
        assert_eq!(effective_max_z(loam, &SlotStack::default()), 0);
        let road = SlotStack { s0_pave: S0Pavement::StoneStreet, ..SlotStack::default() };
        assert_eq!(step_delay_ms(&road), 220);
    }

    #[test]
    fn overlay_takes_precedence_over_pristine() {
        let l0 = land(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::None);
        let stack = SlotStack { v1: V1Plugin::AncientCamphorCanopy, ..SlotStack::default() };
        let rec = CellOverlayRecord::from_stack(1, 0, stack.clone());
        let snap = CellFacade::snapshot_from_parts(
            1, 0, l0, None, None, Some(rec), FieldState::default(), runtime::WORLD_SEED,
        );
        assert_eq!(snap.slots, stack);
        let pristine = CellFacade::snapshot_from_parts(
            1, 0, l0, None, None, None, FieldState::default(), runtime::WORLD_SEED,
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
            CellFacade::snapshot_from_parts(gx, gy, l0, None, None, None, FieldState::default(), runtime::WORLD_SEED)
        });
        let reads = reads.unwrap_or_default();
        assert_eq!(reads.len(), 4);
        assert!(reads.iter().all(|r| r.legacy_walkable && r.substrate_walkable));
    }

    #[test]
    fn four_anchors_reject_unwalkable_legacy() {
        let grid = SquareGrid::new();
        let result = verify_with(&grid, |gx, gy| {
            let l0 = land(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::None);
            CellFacade::snapshot_from_parts(gx, gy, l0, None, None, None, FieldState::default(), runtime::WORLD_SEED)
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
}
