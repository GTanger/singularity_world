// Watabou / MFCG 城市 GeoJSON：啟動時解析並注入記憶體 store（不寫 JSON 房間檔）。

mod ambience;
mod builder;
mod geom;
mod parser;

use std::path::Path;

use anyhow::Context;

use crate::model;
use crate::store::Store;

/// 解析後的城市幾何（不含語意，供 builder 使用）。
pub struct CityGeo {
    pub burg_id: u32,
    pub roads: Vec<Road>,
    pub buildings: Vec<Building>,
    pub districts: Vec<District>,
    pub wall_rings: Vec<Vec<(f64, f64)>>,
    pub river_lines: Vec<Vec<(f64, f64)>>,
    pub squares: Vec<Vec<(f64, f64)>>,
}

/// 道路線段（LineString）。
pub struct Road {
    pub coords: Vec<(f64, f64)>,
}

/// 建築外環。
pub struct Building {
    pub ring: Vec<(f64, f64)>,
}

/// 行政／街區多邊形。
pub struct District {
    pub name: Option<String>,
    pub ring: Vec<(f64, f64)>,
}

/// 載入統計（寫 log 用）。
#[derive(Debug, Clone)]
pub struct InjectStats {
    pub burg_id: u32,
    pub junctions: usize,
    pub buildings: usize,
}

/// 讀取 `data/cities/{burg_id}.json`，生成城市房間並注入 `store`（僅記憶體），
/// 並在世界地圖房間加上「進城」出口。失敗時回傳 Err。
pub fn load_and_inject(store: &mut Store, path: &Path) -> anyhow::Result<InjectStats> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .context("城市檔檔名無效")?;
    let burg_id: u32 = stem.parse().context("城市檔檔名須為數字 burg_id")?;

    let geo = parser::parse_city_geojson(path, burg_id)?;
    let world_id = find_world_room_id_for_burg(store, burg_id)?
        .with_context(|| format!("找不到 burg_id={} 對應的世界房間（editor JSON meta.burg_id）", burg_id))?;

    let world_room = store
        .rooms
        .get(&world_id)
        .cloned()
        .with_context(|| format!("store 缺少世界房間 {}", world_id))?;
    let world_exits = store.get_exits_for_room(&world_id);

    let ambience_path = path
        .parent()
        .and_then(|p| p.parent())
        .map(|d| d.join("config/city_ambience.json"));
    let built = builder::build_city_rooms(
        &geo,
        &world_room.tags,
        &world_id,
        &world_room.name,
        &world_exits,
        ambience_path.as_deref(),
    );
    if built.junction_count == 0 || built.gate_room_id.is_empty() {
        anyhow::bail!("城市 burg_id={} 無有效路口（道路資料可能為空）", burg_id);
    }

    let batch = built.batch;
    store.inject_ephemeral_rooms(batch.rooms, batch.exits);
    store.append_exit_unique(
        &world_id,
        model::Exit {
            direction: "進城".to_string(),
            to_room_id: built.gate_room_id.clone(),
            to_room_name: built.gate_room_name.clone(),
        },
    );

    Ok(InjectStats {
        burg_id,
        junctions: built.junction_count,
        buildings: built.enterable_count,
    })
}

/// 掃描 `rooms_path/editor/*.json`，找 `meta.burg_id` 吻合的房間 id。
fn find_world_room_id_for_burg(store: &Store, burg_id: u32) -> anyhow::Result<Option<String>> {
    let editor = Path::new(store.rooms_path()).join("editor");
    let rd = match std::fs::read_dir(&editor) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if p.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let raw = match std::fs::read_to_string(&p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(x) => x,
            Err(_) => continue,
        };
        let Some(meta) = v.get("meta") else { continue };
        let Some(src) = meta.get("source").and_then(|x| x.as_str()) else {
            continue;
        };
        if src != "azgaar" {
            continue;
        }
        let Some(b) = meta.get("burg_id").and_then(|x| x.as_u64()) else {
            continue;
        };
        if b as u32 != burg_id {
            continue;
        }
        if let Some(id) = v.get("id").and_then(|x| x.as_str()) {
            return Ok(Some(id.to_string()));
        }
    }
    Ok(None)
}
