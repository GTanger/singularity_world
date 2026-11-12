//! 能元台灣 `L0` 底板（6-Byte 物理針腳）、TWD97 × `k = 2` 座標定位與內政部 20m DEM 母圖模組。
//!
//! 微步 1 核心隔離邊界：本模組提供純物理底板與稀疏分塊 `SWL0` 讀寫，不改寫線上既有 `GridCell` 實作。

pub mod coords;
pub mod l0_cell;
pub mod moi_dem;
pub mod render_synth;
pub mod runtime;
pub mod slots;

pub use coords::{
    CELL_SIZE_M, CHUNK_SIDE, DerivedSlope, EXPANSION_K, ORIGIN_TWD97_E, ORIGIN_TWD97_N,
    REAL_METERS_PER_CELL, SUBSTRATE_FILE_MAGIC, SUBSTRATE_FILE_VERSION, SensoryEdgeCue,
    SubstrateRegion, grid_to_twd97_center, twd97_to_grid, wgs84_to_twd97,
};
pub use l0_cell::{
    ELEV_OFFSET_M, ELEV_RAW_MAX_VALID, ELEV_RAW_NODATA, GROUNDWATER_BUCKET_UPPER_M, L0Cell,
    SubstrateMaterial, SurfaceWaterKind, encode_groundwater_depth_bucket,
    groundwater_bucket_lower_bound_m, landmass, pack_water_regime,
};
pub use moi_dem::{MOI_DEM_MIN_VALID_M, Moi20mDemWindow, TfwGeoTransform, rasterize_4conn_thick_segment};
