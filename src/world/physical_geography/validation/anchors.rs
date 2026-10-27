//! 白塔半島正典節點錨點與主要地理語義驗證。

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow, bail, ensure};

use crate::world::physical_geography::{AnchorMapping, PhysicalGeography};

const REQUIRED_ANCHORS: [&str; 13] = [
    "wt-n-sacred-mountain",
    "wt-n-north-crown-sea",
    "wt-w-windship-island",
    "wt-w-natural-harbor-channel",
    "wt-w-windship-cape",
    "wt-c-wenyao",
    "wt-c-sacred-tower",
    "wt-e-green-ridge",
    "wt-e-east-sea",
    "wt-c-southwest-plain",
    "wt-s-south-sentinel",
    "wt-s-black-marsh",
    "wt-s-qinglong-pass",
];

/// 驗證 13 個地理錨點、表面種類與語義關係；輸入只讀，沒有外部副作用。
pub(super) fn validate_anchors(
    geography: &PhysicalGeography,
    topology_ids: &BTreeSet<&str>,
    area_ids: &BTreeSet<&str>,
) -> Result<()> {
    // 錨點把程序計算綁回已接受節點，不讓生成器自行發明大地理。
    let mut anchors = BTreeMap::new();
    let area_topology = geography
        .areas
        .iter()
        .map(|area| (area.id.as_str(), area.topology_node_ref.as_str()))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        area_topology.len() == area_ids.len(),
        "anchor area 索引不完整"
    );
    for anchor in &geography.anchor_mappings {
        ensure!(
            topology_ids.contains(anchor.topology_node_ref.as_str()),
            "anchor topologyNodeRef 不存在"
        );
        ensure!(
            anchors
                .insert(anchor.topology_node_ref.as_str(), anchor)
                .is_none(),
            "重複 anchor：{}",
            anchor.topology_node_ref
        );
        validate_anchor(anchor, &area_topology)?;
    }
    ensure!(anchors.len() == REQUIRED_ANCHORS.len(), "anchor 數量不符");
    ensure!(
        REQUIRED_ANCHORS.iter().all(|id| anchors.contains_key(id)),
        "必要 anchor 不完整"
    );
    validate_anchor_semantics(&anchors)
}

/// 驗證單一錨點距離、陸水表面與 areaRef；輸入只讀，沒有副作用。
fn validate_anchor(anchor: &AnchorMapping, area_topology: &BTreeMap<&str, &str>) -> Result<()> {
    // 陸地必須落在聚合 area，水域則不能偽造可走場所。
    ensure!(!anchor.label.trim().is_empty(), "anchor.label 不可為空白");
    ensure!(
        anchor.normalized_distance <= 0.12,
        "{} anchor 距離過遠",
        anchor.topology_node_ref
    );
    match anchor.expected_surface.as_str() {
        "land" => validate_land_anchor(anchor, area_topology),
        "water" => validate_water_anchor(anchor),
        _ => bail!("{} expectedSurface 無效", anchor.topology_node_ref),
    }
}

/// 驗證陸地錨點高度與 areaRef；輸入只讀，沒有副作用。
fn validate_land_anchor(
    anchor: &AnchorMapping,
    area_topology: &BTreeMap<&str, &str>,
) -> Result<()> {
    // Azgaar 高度 20 是本來源的海陸界線，僅作作者資料契約。
    ensure!(anchor.height >= 20, "{} 應為陸地", anchor.topology_node_ref);
    let area_ref = anchor
        .area_ref
        .as_deref()
        .ok_or_else(|| anyhow!("{} 陸地 anchor 缺 areaRef", anchor.topology_node_ref))?;
    let actual_topology = area_topology
        .get(area_ref)
        .ok_or_else(|| anyhow!("{} areaRef 不存在", anchor.topology_node_ref))?;
    ensure!(
        *actual_topology == anchor.topology_node_ref,
        "{} areaRef 掛在不同拓撲節點",
        anchor.topology_node_ref
    );
    Ok(())
}

/// 驗證水域錨點低於海陸界線且沒有 areaRef；輸入只讀，沒有副作用。
fn validate_water_anchor(anchor: &AnchorMapping) -> Result<()> {
    // 水域保留為地理關係，不直接成為可通行陸地場所。
    ensure!(anchor.height < 20, "{} 應為水域", anchor.topology_node_ref);
    ensure!(
        anchor.area_ref.is_none(),
        "{} 水域不可指向陸地 area",
        anchor.topology_node_ref
    );
    Ok(())
}

/// 驗證白塔、青脊山、西南原與黑沼林錨點的必要物理語義；輸入只讀。
fn validate_anchor_semantics(anchors: &BTreeMap<&str, &AnchorMapping>) -> Result<()> {
    // 河道是否穿過黑沼林由完整主要水系路徑驗證，不以錨點單格的 river id 代替。
    ensure!(anchors["wt-n-sacred-mountain"].height >= 70, "白塔必須居高");
    ensure!(anchors["wt-e-green-ridge"].height >= 50, "青脊山高度不足");
    ensure!(anchors["wt-c-wenyao"].height >= 38, "文曜古河階高度不足");
    ensure!(anchors["wt-c-wenyao"].river == 0, "文曜不可壓在河道單格上");
    ensure!(
        anchors["wt-c-southwest-plain"].biome_name == "Grassland",
        "西南原應為草地"
    );
    ensure!(
        anchors["wt-s-black-marsh"].biome_name == "Wetland",
        "黑沼林應為濕地"
    );
    let south_sentinel = anchors["wt-s-south-sentinel"];
    ensure!(south_sentinel.height >= 34, "南哨穩定乾地高度不足");
    ensure!(south_sentinel.river == 0, "南哨不可壓在河道單格上");
    ensure!(
        south_sentinel.biome_name != "Wetland",
        "南哨不可落在濕地單格"
    );
    let qinglong_pass = anchors["wt-s-qinglong-pass"];
    ensure!(qinglong_pass.height >= 45, "青籠關地形收束高度不足");
    ensure!(
        qinglong_pass.biome_name != "Wetland",
        "青籠關不可落在濕地單格"
    );
    Ok(())
}
