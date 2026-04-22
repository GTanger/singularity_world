//! OSM `place=*` 聚落種子抽取。
//!
//! seed-and-grow 的 seed 來源：OSM 對現實地球聚落的官方標註。
//! 不自己發明城鎮位置，直接採收實地名點。
//!
//! Tier 映射（對齊 session 04614601/ae211257 拍板的三級粒度）：
//!   city / town        → 城核（100m scale）
//!   village / hamlet   → 鄉鎮（200m scale）
//!   （無種子覆蓋區）    → 野外（1km scale，由 grow 階段自動填）
//!
//! 輸出純資料；不產網格、不連接鄰居——那是 grow 階段的事。

use osmpbf::{Element, ElementReader};
use std::path::Path;

/// 格尺度——對齊三級粒度架構。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// 城核，~100m/格
    Urban,
    /// 鄉鎮，~200m/格
    Town,
    /// 野外，~1km/格（seed 無此類，grow 階段填）
    #[allow(dead_code)]
    Wild,
}

impl Tier {
    /// 格邊長（公尺），用於 grow 階段決定格子密度
    pub fn cell_meters(self) -> f64 {
        match self {
            Tier::Urban => 100.0,
            Tier::Town => 200.0,
            Tier::Wild => 1000.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Tier::Urban => "urban",
            Tier::Town => "town",
            Tier::Wild => "wild",
        }
    }
}

/// OSM place 節點抽出的聚落種子。
#[derive(Debug, Clone)]
pub struct PlaceSeed {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub tier: Tier,
    /// 原始 place=* 值（city/town/village/hamlet）
    pub place_kind: String,
    /// population 標籤（OSM 有標才有，否則 None）
    pub population: Option<u64>,
}

/// 將 OSM place=* tag 值映射為 Tier。
///
/// 只收 city/town/village/hamlet——其他（neighbourhood/suburb/isolated_dwelling）
/// 尺度太細，疊到 Urban/Town 即可，不獨立成種子。
pub fn classify_place(place: &str) -> Option<Tier> {
    match place {
        "city" | "town" => Some(Tier::Urban),
        "village" | "hamlet" => Some(Tier::Town),
        _ => None,
    }
}

/// 掃 PBF 抽所有合格 place 節點。
///
/// 節點型為 Node 或 DenseNode，都會收。
pub fn load_places(pbf: &Path) -> anyhow::Result<Vec<PlaceSeed>> {
    eprintln!("OSM：掃 place=* 節點…");
    let reader = ElementReader::from_path(pbf)?;
    let seeds: Vec<PlaceSeed> = reader.par_map_reduce(
        |element| {
            let mut batch = Vec::new();
            let (lat, lon, tags): (f64, f64, Vec<(String, String)>) = match element {
                Element::Node(n) => (
                    n.lat(),
                    n.lon(),
                    n.tags().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                ),
                Element::DenseNode(n) => (
                    n.lat(),
                    n.lon(),
                    n.tags().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                ),
                _ => return batch,
            };

            let mut place_val: Option<String> = None;
            let mut name_val: Option<String> = None;
            let mut pop_val: Option<u64> = None;
            for (k, v) in &tags {
                match k.as_str() {
                    "place" => place_val = Some(v.clone()),
                    "name:zh" => name_val = Some(v.clone()),
                    // name:zh 優先；沒有才退回一般 name
                    "name" => {
                        if name_val.is_none() {
                            name_val = Some(v.clone());
                        }
                    }
                    "population" => pop_val = v.replace(',', "").parse().ok(),
                    _ => {}
                }
            }

            if let Some(pv) = place_val
                && let Some(tier) = classify_place(&pv)
            {
                batch.push(PlaceSeed {
                    name: name_val.unwrap_or_else(|| "(unnamed)".to_string()),
                    lat,
                    lon,
                    tier,
                    place_kind: pv,
                    population: pop_val,
                });
            }
            batch
        },
        Vec::new,
        |mut a, b| {
            a.extend(b);
            a
        },
    )?;
    eprintln!("  找到 {} 個 place 種子", seeds.len());
    Ok(seeds)
}

/// 統計輔助：按 tier + kind 分桶計數。
pub fn summarize(seeds: &[PlaceSeed]) -> Vec<(String, usize)> {
    use std::collections::BTreeMap;
    let mut bucket: BTreeMap<String, usize> = BTreeMap::new();
    for s in seeds {
        *bucket.entry(format!("{}/{}", s.tier.name(), s.place_kind)).or_insert(0) += 1;
    }
    bucket.into_iter().collect()
}
