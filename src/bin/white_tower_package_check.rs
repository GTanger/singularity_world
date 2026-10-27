#![forbid(unsafe_code)]
//! 驗證白塔半島 Azgaar 作者資料包的完整性、原生層數量與非 runtime 邊界。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde_json::Value;
use sha2::{Digest, Sha256};
use singularity_world::world::physical_geography::{
    PhysicalGeography, validate_physical_geography,
};

const REQUIRED_FILES: [&str; 15] = [
    "white_tower_peninsula_v1.map",
    "white_tower_peninsula_v1_full.json",
    "white_tower_peninsula_v1_political.svg",
    "white_tower_peninsula_v1_political.png",
    "white_tower_peninsula_v1_biomes.svg",
    "white_tower_peninsula_v1_biomes.png",
    "white_tower_peninsula_v1_heightmap.svg",
    "white_tower_peninsula_v1_heightmap.png",
    "white_tower_peninsula_v1_canon_overlay.svg",
    "white_tower_peninsula_v1_canon_overlay.png",
    "white_tower_peninsula_v1_cells.geojson",
    "white_tower_peninsula_v1_routes.geojson",
    "white_tower_peninsula_v1_rivers.geojson",
    "white_tower_peninsula_v1_markers.geojson",
    "white_tower_peninsula_v1_zones.geojson",
];

fn main() -> Result<()> {
    // 允許生成器先驗 staging；不帶參數時驗正式資料包。
    let mut arguments = std::env::args_os().skip(1);
    let package_dir = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/world/white_tower_azgaar_v1"));
    let geography_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/world/white_tower_physical_geography_v1.json"));
    ensure!(arguments.next().is_none(), "參數過多");
    let manifest = read_json(&package_dir.join("manifest.json"))?;
    validate_boundaries(&manifest)?;
    validate_receipts(&package_dir, &geography_path, &manifest)?;
    validate_geography(&geography_path)?;
    validate_native_data(&package_dir, &manifest)?;
    validate_visual_exports(&package_dir)?;
    println!(
        "白塔半島 Azgaar 完整作者資料包通過：15 個原生輸出、雜湊、完整 JSON、GeoJSON 與 1600×1000 視圖均有效；canon/runtime/playerFacing 均為 false。"
    );
    Ok(())
}

/// 直接驗證 staging 的物理橋接內容，避免安裝後才發現舊邊界規則衝突。
fn validate_geography(path: &Path) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("無法讀取 {}", path.display()))?;
    let geography: PhysicalGeography = serde_json::from_slice(&bytes)?;
    validate_physical_geography(&geography)
}

/// 讀取 JSON；只讀指定檔案，不改動作者資料。
fn read_json(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).with_context(|| format!("無法讀取 {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("JSON 無效：{}", path.display()))
}

/// 驗證作者資料沒有冒充正典、runtime 或玩家畫面。
fn validate_boundaries(manifest: &Value) -> Result<()> {
    ensure!(
        manifest["schemaVersion"] == 1,
        "manifest schemaVersion 錯誤"
    );
    ensure!(manifest["canon"] == false, "作者資料不可冒充正典");
    ensure!(
        manifest["runtimeAuthority"] == false,
        "作者資料不可成為 runtime 權威"
    );
    ensure!(
        manifest["playerFacingMap"] == false,
        "Azgaar 圖不可成為玩家畫面"
    );
    ensure!(
        manifest["source"]["commit"] == "73e43d2b44b9b85b42c5394a0476d90bb22d688e",
        "Azgaar commit 未固定"
    );
    ensure!(
        manifest["dispositions"]["nativeAzgaarData"] == "retained_as_non_canon_candidates",
        "原生資料沒有完整保留為候選"
    );
    Ok(())
}

/// 驗證每個資料包檔案與外部物理橋接檔的大小及 SHA-256。
fn validate_receipts(package_dir: &Path, geography_path: &Path, manifest: &Value) -> Result<()> {
    let receipts = manifest["files"]
        .as_array()
        .context("manifest.files 缺失")?;
    ensure!(receipts.len() == REQUIRED_FILES.len(), "原生輸出數量不符");
    for required in REQUIRED_FILES {
        let receipt = receipts
            .iter()
            .find(|item| item["path"] == required)
            .with_context(|| format!("缺少原生輸出收據：{required}"))?;
        validate_receipt(&package_dir.join(required), receipt)?;
    }
    validate_receipt(geography_path, &manifest["physicalGeography"])
}

/// 驗證單一收據；讀取檔案計算雜湊，不相信只有存在性的檢查。
fn validate_receipt(path: &Path, receipt: &Value) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("缺少 {}", path.display()))?;
    let expected_bytes = receipt["bytes"].as_u64().context("收據缺 bytes")?;
    ensure!(
        bytes.len() as u64 == expected_bytes,
        "{} 大小不符",
        path.display()
    );
    let actual_hash = format!("{:x}", Sha256::digest(&bytes));
    ensure!(
        receipt["sha256"] == actual_hash,
        "{} SHA-256 不符",
        path.display()
    );
    Ok(())
}

/// 驗證 Azgaar Full JSON 的主要原生層與 manifest 數量一致。
fn validate_native_data(package_dir: &Path, manifest: &Value) -> Result<()> {
    let full = read_json(&package_dir.join("white_tower_peninsula_v1_full.json"))?;
    ensure!(
        full["info"]["mapName"] == "白塔半島",
        "Full JSON 地圖名稱錯誤"
    );
    ensure!(
        full["info"]["seed"] == "white-tower-peninsula-v1",
        "Full JSON seed 錯誤"
    );
    ensure!(
        full["info"]["width"] == 1600 && full["info"]["height"] == 1000,
        "畫布尺寸錯誤"
    );
    let pack = &full["pack"];
    compare_len(&pack["cells"], &manifest["source"]["packedCells"], "cells")?;
    compare_len(&pack["rivers"], &manifest["generated"]["rivers"], "rivers")?;
    compare_len(
        &pack["routes"],
        &manifest["generated"]["routeCandidates"],
        "routes",
    )?;
    compare_len(&pack["goods"], &manifest["generated"]["goods"], "goods")?;
    compare_len(
        &pack["markets"],
        &manifest["generated"]["markets"],
        "markets",
    )?;
    compare_len(&pack["deals"], &manifest["generated"]["deals"], "deals")?;
    compare_active(pack, manifest, "burgs", "burgCandidates", false)?;
    compare_active(pack, manifest, "states", "stateCandidates", false)?;
    compare_active(pack, manifest, "provinces", "provinceCandidates", false)?;
    compare_active(pack, manifest, "cultures", "cultureCandidates", false)?;
    compare_active(pack, manifest, "religions", "religionCandidates", false)?;
    compare_active(pack, manifest, "markers", "markerCandidates", true)?;
    compare_active(pack, manifest, "zones", "zoneCandidates", true)?;
    validate_map_header(package_dir)
}

/// 比對陣列長度；manifest 是生成收據而非另一套資料來源。
fn compare_len(array: &Value, expected: &Value, label: &str) -> Result<()> {
    let length = array
        .as_array()
        .with_context(|| format!("{label} 不是陣列"))?
        .len() as u64;
    ensure!(expected.as_u64() == Some(length), "{label} 數量不符");
    Ok(())
}

/// 比對具有 i 且未 removed 的 Azgaar 候選物件數量。
fn compare_active(
    pack: &Value,
    manifest: &Value,
    source: &str,
    target: &str,
    zero_is_valid: bool,
) -> Result<()> {
    let values = pack[source]
        .as_array()
        .with_context(|| format!("{source} 不是陣列"))?;
    let active = values
        .iter()
        .filter(|item| {
            item["i"].as_u64().is_some_and(|id| zero_is_valid || id > 0) && item["removed"] != true
        })
        .count() as u64;
    ensure!(
        manifest["generated"][target].as_u64() == Some(active),
        "{source} 候選數量不符"
    );
    Ok(())
}

/// 驗證可回載 .map 的版本提示與固定 seed，避免空殼匯出。
fn validate_map_header(package_dir: &Path) -> Result<()> {
    let map = fs::read(package_dir.join("white_tower_peninsula_v1.map"))?;
    let prefix = String::from_utf8_lossy(&map[..map.len().min(512)]);
    ensure!(prefix.starts_with("1.137.6|"), ".map 版本標頭錯誤");
    ensure!(
        prefix.contains("white-tower-peninsula-v1"),
        ".map 缺固定 seed"
    );
    Ok(())
}

/// 驗證 SVG、PNG 與 GeoJSON 確實可解析且具有預期畫布。
fn validate_visual_exports(package_dir: &Path) -> Result<()> {
    for name in REQUIRED_FILES.iter().filter(|name| name.ends_with(".svg")) {
        let text = fs::read_to_string(package_dir.join(name))?;
        ensure!(text.contains("<svg"), "{name} 不是 SVG");
    }
    for name in REQUIRED_FILES.iter().filter(|name| name.ends_with(".png")) {
        validate_png(&package_dir.join(name))?;
    }
    for name in REQUIRED_FILES
        .iter()
        .filter(|name| name.ends_with(".geojson"))
    {
        let value = read_json(&package_dir.join(name))?;
        ensure!(
            value["type"] == "FeatureCollection",
            "{name} 不是 FeatureCollection"
        );
        ensure!(
            !value["features"]
                .as_array()
                .context("GeoJSON 缺 features")?
                .is_empty(),
            "{name} 沒有 features"
        );
    }
    Ok(())
}

/// 讀取 PNG IHDR；確認輸出不是損壞縮圖或錯誤頁面。
fn validate_png(path: &Path) -> Result<()> {
    let bytes = fs::read(path)?;
    ensure!(
        bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n",
        "{} 不是 PNG",
        path.display()
    );
    let width = u32::from_be_bytes(bytes[16..20].try_into()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into()?);
    ensure!(
        (width, height) == (1600, 1000),
        "{} 尺寸不是 1600×1000",
        path.display()
    );
    Ok(())
}
