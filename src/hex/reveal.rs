//! Hex 格「揭露驅動」生成（對齊 Hex 探索規格：種子＋邊界、契約釘死、可重現）。
//!
//! 未在 `HexGrid.cells` 內之座標視為**黑格**（無契約）；`generate_wild_cell` 產出首次彩格內容。

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use super::cell::{HexCell, Terrain};
use super::coord::HexCoord;
use super::grid::HexGrid;

/// 與 `world_seed`、座標混合成 `StdRng` 種子（跨平台決定性）
pub fn mix_coord_seed(world_seed: u64, coord: HexCoord) -> u64 {
    let mut x = world_seed;
    let q = coord.q as i64 as u64;
    let r = coord.r as i64 as u64;
    x ^= q.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= r.rotate_left(21).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x
}

/// 非城內 POI：自動揭露用權重表
const WILD_WEIGHTS: &[(Terrain, u32)] = &[
    (Terrain::Plain, 14),
    (Terrain::Grassland, 10),
    (Terrain::Forest, 12),
    (Terrain::ForestLight, 8),
    (Terrain::ForestHeavy, 6),
    (Terrain::Hills, 10),
    (Terrain::Mountain, 5),
    (Terrain::Water, 8),
    (Terrain::WaterDeep, 3),
    (Terrain::Desert, 5),
    (Terrain::Swamp, 6),
    (Terrain::Tundra, 4),
    (Terrain::Jungle, 5),
    (Terrain::Road, 2),
    (Terrain::Bridge, 1),
];

fn weighted_wild_terrain(rng: &mut impl Rng) -> Terrain {
    let total: u32 = WILD_WEIGHTS.iter().map(|(_, w)| w).sum();
    let mut roll = rng.random_range(0..total);
    for &(t, w) in WILD_WEIGHTS {
        if roll < w {
            return t;
        }
        roll -= w;
    }
    Terrain::Plain
}

fn terrain_title_zh(t: Terrain) -> &'static str {
    match t {
        Terrain::Plain => "平原",
        Terrain::Forest => "森林",
        Terrain::ForestHeavy => "密林",
        Terrain::ForestLight => "疏林",
        Terrain::Mountain => "山地",
        Terrain::Hills => "丘陵",
        Terrain::Water => "水域",
        Terrain::WaterDeep => "深水",
        Terrain::Desert => "沙漠",
        Terrain::Swamp => "沼澤",
        Terrain::Tundra => "凍原",
        Terrain::Grassland => "草原",
        Terrain::Jungle => "叢林",
        Terrain::Urban => "城區",
        Terrain::Road => "道路",
        Terrain::Bridge => "橋樑",
        Terrain::Wall => "牆體",
        Terrain::FarmField => "農田",
        Terrain::Farmhouse => "農舍",
        Terrain::Inn => "旅店",
        Terrain::Tavern => "酒館",
        Terrain::Blacksmith => "鐵匠鋪",
        Terrain::GeneralStore => "雜貨店",
        Terrain::Clinic => "醫館",
        Terrain::Workshop => "工坊",
        Terrain::Market => "市集",
        Terrain::GuildHall => "公會大廳",
        Terrain::Temple => "神殿",
        Terrain::Academy => "學院",
        Terrain::Library => "圖書館",
        Terrain::Barracks => "兵營",
        Terrain::GuardPost => "衛所",
        Terrain::Warehouse => "倉庫",
        Terrain::Granary => "糧倉",
        Terrain::Dock => "碼頭",
        Terrain::Bathhouse => "浴場",
        Terrain::Courthouse => "法院",
        Terrain::Jail => "監所",
        Terrain::TownHall => "市政廳",
        Terrain::Bank => "銀行",
        Terrain::Mint => "鑄幣所",
        Terrain::Stables => "馬廄",
        Terrain::Caravanserai => "商旅驛站",
        Terrain::Theater => "劇院",
        Terrain::Arena => "競技場",
        Terrain::Observatory => "觀測台",
        Terrain::Alchemist => "鍊金工房",
        Terrain::MageTower => "法師塔",
        Terrain::Embassy => "使館",
        Terrain::PrisonYard => "囚院",
    }
}

/// 依 `world_seed`、已揭露鄰格（若有）決定性生成一筆彩格（未寫入 grid）。
pub fn generate_wild_cell(grid: &HexGrid, coord: HexCoord) -> HexCell {
    let mut rng = StdRng::seed_from_u64(mix_coord_seed(grid.world_seed(), coord));

    let neighbor_terrains: Vec<Terrain> = coord
        .neighbors()
        .into_iter()
        .filter_map(|n| grid.get(n).map(|c| c.terrain))
        .collect();

    let terrain = if !neighbor_terrains.is_empty() && rng.random_bool(0.42) {
        let i = rng.random_range(0..neighbor_terrains.len());
        neighbor_terrains[i]
    } else {
        weighted_wild_terrain(&mut rng)
    };

    let name = format!("{}·{}", terrain_title_zh(terrain), coord.to_cell_id());
    HexCell::new(coord, terrain, name)
        .with_zone("wild")
        .with_tags(vec!["proc_reveal".to_string()])
        .with_description("觀測揭露生成（契約層，單調精煉）。")
}

/// 以六角距離 `radius` 內、由近到遠順序揭露；已存在之格不覆寫。回傳**新插入**格數。
pub fn reveal_hex_disk(grid: &mut HexGrid, center: HexCoord, radius: i32) -> usize {
    if radius < 0 {
        return 0;
    }
    let r = radius as u32;
    let mut coords = Vec::new();
    for dq in -radius..=radius {
        for dr in -radius..=radius {
            let c = HexCoord::new(center.q + dq, center.r + dr);
            if center.distance(c) <= r {
                coords.push(c);
            }
        }
    }
    coords.sort_by_key(|c| (center.distance(*c), c.q, c.r));

    let mut new_count = 0usize;
    for c in coords {
        if grid.contains(c) {
            continue;
        }
        let cell = generate_wild_cell(grid, c);
        grid.insert(cell);
        new_count += 1;
    }
    new_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::cell::Terrain;

    #[test]
    fn deterministic_per_coord() {
        let mut g = HexGrid::new();
        g.set_world_seed(0xC0FFEE);
        let a = generate_wild_cell(&g, HexCoord::new(2, -5));
        let b = generate_wild_cell(&g, HexCoord::new(2, -5));
        assert_eq!(a.terrain, b.terrain);
        assert_eq!(a.name, b.name);
    }

    #[test]
    fn different_coords_differ_usually() {
        let g = HexGrid::new();
        let a = generate_wild_cell(&g, HexCoord::ORIGIN);
        let b = generate_wild_cell(&g, HexCoord::new(100, 200));
        assert_ne!(a.coord, b.coord);
    }

    #[test]
    fn neighbor_continuity() {
        let mut g = HexGrid::new();
        g.set_world_seed(42);
        g.insert(HexCell::new(
            HexCoord::ORIGIN,
            Terrain::Forest,
            "種子",
        ));
        let n = HexCoord::new(1, 0);
        let cell = generate_wild_cell(&g, n);
        assert!(cell.tags.contains(&"proc_reveal".to_string()));
        assert_eq!(cell.zone, "wild");
        assert_eq!(cell.coord, n);
    }

    #[test]
    fn reveal_disk_counts() {
        let mut g = HexGrid::new();
        g.set_world_seed(1);
        let n = reveal_hex_disk(&mut g, HexCoord::ORIGIN, 1);
        assert_eq!(n, 7);
        assert_eq!(g.len(), 7);
        let n2 = reveal_hex_disk(&mut g, HexCoord::ORIGIN, 2);
        assert!(n2 > 0);
        assert_eq!(reveal_hex_disk(&mut g, HexCoord::ORIGIN, 2), 0);
    }
}
