//! 白塔半島物理地理作者資料的載入、驗證與場所候選轉換入口。
//!
//! Azgaar 只在離線作者階段產生資料；這個模組不寫資料庫、不啟用候選路線，
//! 也不把生成內容升格成玩家地圖或正典。

mod conversion;
mod model;
mod validation;

use anyhow::Result;

pub use model::*;

const WHITE_TOWER_PHYSICAL_GEOGRAPHY_JSON: &str =
    include_str!("../../data/world/white_tower_physical_geography_v1.json");

/// 載入內建白塔物理地理 JSON，完成全部邊界檢查後回傳作者資料；沒有外部副作用。
pub fn builtin_white_tower_physical_geography() -> Result<PhysicalGeography> {
    // include_str 讓服務編譯後不依賴外部 JSON 路徑，JSON 本身仍只是作者種子。
    let geography: PhysicalGeography = serde_json::from_str(WHITE_TOWER_PHYSICAL_GEOGRAPHY_JSON)?;
    validate_physical_geography(&geography)?;
    Ok(geography)
}

/// 驗證傳入的作者地理資料，成功回傳空值，失敗回傳具體原因；沒有外部副作用。
pub fn validate_physical_geography(geography: &PhysicalGeography) -> Result<()> {
    // 實作留在 validation 子模組，入口只維持穩定公共契約。
    validation::validate(geography)
}

#[cfg(test)]
#[path = "physical_geography/tests.rs"]
mod tests;
