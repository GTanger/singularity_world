// 區塊載入，對齊既有 world/chunk_load。

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use super::map::{
    Grid, TERRAIN_ALLEY, TERRAIN_DOOR, TERRAIN_DOOR_CLOSED, TERRAIN_FARM, TERRAIN_FLOOR,
    TERRAIN_FOG, TERRAIN_GRASS, TERRAIN_ICE, TERRAIN_LAVA, TERRAIN_MOUNTAIN, TERRAIN_RIVER,
    TERRAIN_ROAD, TERRAIN_STONE, TERRAIN_SWAMP, TERRAIN_TREE, TERRAIN_VALLEY, TERRAIN_WALL,
    TERRAIN_WASTELAND, TERRAIN_WATER,
};

static RUNE_TO_TERRAIN: LazyLock<HashMap<char, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    for t in [
        TERRAIN_WALL,
        TERRAIN_DOOR,
        TERRAIN_DOOR_CLOSED,
        TERRAIN_ROAD,
        TERRAIN_ALLEY,
        TERRAIN_GRASS,
        TERRAIN_TREE,
        TERRAIN_MOUNTAIN,
        TERRAIN_STONE,
        TERRAIN_SWAMP,
        TERRAIN_RIVER,
        TERRAIN_WATER,
        TERRAIN_WASTELAND,
        TERRAIN_LAVA,
        TERRAIN_ICE,
        TERRAIN_FARM,
        TERRAIN_VALLEY,
        TERRAIN_FOG,
        TERRAIN_FLOOR,
    ] {
        if let Some(c) = t.chars().next() {
            m.insert(c, t);
        }
    }
    for c in "艸芔茻".chars() {
        m.insert(c, TERRAIN_GRASS);
    }
    for c in "林森".chars() {
        m.insert(c, TERRAIN_TREE);
    }
    for c in "岳巒".chars() {
        m.insert(c, TERRAIN_MOUNTAIN);
    }
    for c in "磊岩".chars() {
        m.insert(c, TERRAIN_STONE);
    }
    for c in "澤泥".chars() {
        m.insert(c, TERRAIN_SWAMP);
    }
    for c in "巜巛".chars() {
        m.insert(c, TERRAIN_RIVER);
    }
    for c in "沝淼".chars() {
        m.insert(c, TERRAIN_WATER);
    }
    for c in "旱焦".chars() {
        m.insert(c, TERRAIN_WASTELAND);
    }
    for c in "路徑".chars() {
        m.insert(c, TERRAIN_ROAD);
    }
    for c in "炎焱燚".chars() {
        m.insert(c, TERRAIN_LAVA);
    }
    for c in "凍冽".chars() {
        m.insert(c, TERRAIN_ICE);
    }
    for c in "畓畕".chars() {
        m.insert(c, TERRAIN_FARM);
    }
    for c in "豀豁".chars() {
        m.insert(c, TERRAIN_VALLEY);
    }
    for c in "靄霙".chars() {
        m.insert(c, TERRAIN_FOG);
    }
    m.insert('ㄑ', TERRAIN_RIVER);
    m
});

/// 單一字元對應地形；無對應則草。
pub fn terrain_from_rune(r: char) -> String {
    RUNE_TO_TERRAIN
        .get(&r)
        .map(|s| (*s).to_string())
        .unwrap_or_else(|| TERRAIN_GRASS.to_string())
}

/// 讀取區塊 `(cx, cy)` 的 151×151 地形檔 `{cx}_{cy}.txt`；檔不存在則整塊草。
pub fn load_chunk(cx: i32, cy: i32, base_path: &Path) -> anyhow::Result<Grid> {
    const N: usize = 151;
    let path = base_path.join(format!("{cx}_{cy}.txt"));
    if !path.exists() {
        return Ok(Grid::new(N, N));
    }
    let content = fs::read_to_string(&path)?;
    let mut g = Grid::new(N, N);
    for (y, line) in content.lines().enumerate().take(N) {
        let mut x = 0usize;
        for r in line.chars() {
            if x >= N {
                break;
            }
            if r.is_whitespace() {
                continue;
            }
            let t = terrain_from_rune(r);
            g.set_at(x, y, t);
            x += 1;
        }
    }
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn terrain_from_rune_wall() {
        assert_eq!(terrain_from_rune('牆'), TERRAIN_WALL);
    }

    #[test]
    fn terrain_from_rune_alias_grass() {
        assert_eq!(terrain_from_rune('艸'), TERRAIN_GRASS);
    }

    #[test]
    fn load_chunk_missing_file_is_all_grass() {
        let tmp = env::temp_dir().join("sw_world_no_chunks_xyz");
        let _ = fs::create_dir_all(&tmp);
        let g = load_chunk(9999, 9999, &tmp).expect("ok");
        assert_eq!(g.width, 151);
        assert_eq!(g.at(0, 0), super::super::map::TERRAIN_GRASS);
    }
}
