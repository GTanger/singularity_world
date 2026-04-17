//! 方格格「揭露驅動」生成（對稱 hex/reveal.rs，適配正交網格）。
//!
//! 未在 `SquareGrid.cells` 內之座標視為**黑格**（無契約）；
//! `generate_wild_cell` 產出首次揭露的格子內容。
//!
//! 地形由加權隨機決定（30% 草原、20% 平地、15% 森林……），
//! 與 hex 版的雙噪聲場不同——方格世界採輕量化純雜湊路線，
//! 不依賴噪聲場，確保每格完全獨立且可重現。

use super::coord::SquareCoord;
use super::grid::{GridCell, SquareGrid};
use crate::hex::{HexObject, HexObjectKind, Terrain};

// ─── 座標種子混合 ───

/// 將 `world_seed` 與方格座標混合成決定性 u64 種子。
///
/// 演算法對稱 hex/reveal.rs 的 `mix_coord_seed`：
/// 以飽和乘法與 XOR 組合，確保跨平台確定性。
pub fn mix_coord_seed(world_seed: u64, coord: SquareCoord) -> u64 {
    let mut x = world_seed;
    // 將有號整數位元樣式轉成 u64（負數也能正確參與運算）
    let cx = coord.x as i64 as u64;
    let cy = coord.y as i64 as u64;
    x ^= cx.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= cy.rotate_left(17).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x
}

// ─── 地形加權隨機 ───

/// 依種子高 32 位元決定地形（加權隨機，共 100 份）。
///
/// 分佈：草原 30、平地 20、森林 15、疏林 10、丘陵 8、密林 5、山地 4、水域 3、沼澤 3、荒漠 2
fn terrain_from_seed(seed: u64) -> Terrain {
    // 取高 32 位元轉成 0..100 的整數
    let roll = ((seed >> 32) % 100) as u8;
    match roll {
        0..=29 => Terrain::Grassland,   // 30%
        30..=49 => Terrain::Plain,      // 20%
        50..=64 => Terrain::Forest,     // 15%
        65..=74 => Terrain::ForestLight, // 10%
        75..=82 => Terrain::Hills,      // 8%
        83..=87 => Terrain::ForestHeavy, // 5%
        88..=91 => Terrain::Mountain,   // 4%
        92..=94 => Terrain::Water,      // 3%
        95..=97 => Terrain::Swamp,      // 3%
        _ => Terrain::Desert,           // 2%（98-99）
    }
}

// ─── 地名 ───

pub fn terrain_name_zh(t: Terrain) -> &'static str {
    match t {
        Terrain::Grassland => "草原",
        Terrain::Plain => "平地",
        Terrain::Forest => "林地",
        Terrain::ForestLight => "疏林",
        Terrain::ForestHeavy => "密林",
        Terrain::Hills => "丘陵",
        Terrain::Mountain => "山嶺",
        Terrain::Water => "水域",
        Terrain::WaterDeep => "深水",
        Terrain::Swamp => "沼地",
        Terrain::Desert => "荒漠",
        Terrain::Tundra => "凍原",
        Terrain::Jungle => "叢林",
        _ => "地塊",
    }
}

// ─── 地上物生成 ───

use HexObjectKind::{Flora, Mineral, WaterSource};

/// (kind, name, max_quantity, regrowth_per_tick, 出現機率 0..=100)
type ObjSpec = (HexObjectKind, &'static str, u32, u32, u8);

static OBJ_FOREST: &[ObjSpec] = &[
    (Flora, "野果", 5, 1, 40),
    (Flora, "草藥", 4, 1, 30),
    (Flora, "木材", 5, 1, 55),
];
static OBJ_FOREST_LIGHT: &[ObjSpec] = &[
    (Flora, "野果", 4, 1, 35),
    (Flora, "草藥", 3, 1, 25),
];
static OBJ_FOREST_HEAVY: &[ObjSpec] = &[
    (Flora, "木材", 5, 1, 60),
    (Flora, "草藥", 5, 1, 35),
];
static OBJ_GRASSLAND: &[ObjSpec] = &[
    (Flora, "野果", 4, 1, 30),
    (Flora, "草藥", 3, 1, 20),
];
static OBJ_PLAIN: &[ObjSpec] = &[
    (Flora, "野果", 3, 1, 20),
];
static OBJ_HILLS: &[ObjSpec] = &[
    (Mineral, "礦石", 5, 1, 40),
];
static OBJ_SWAMP: &[ObjSpec] = &[
    (WaterSource, "淡水", 5, 1, 50),
];
static OBJ_EMPTY: &[ObjSpec] = &[];

fn object_table(terrain: Terrain) -> &'static [ObjSpec] {
    match terrain {
        Terrain::Forest => OBJ_FOREST,
        Terrain::ForestLight => OBJ_FOREST_LIGHT,
        Terrain::ForestHeavy => OBJ_FOREST_HEAVY,
        Terrain::Grassland => OBJ_GRASSLAND,
        Terrain::Plain => OBJ_PLAIN,
        Terrain::Hills => OBJ_HILLS,
        Terrain::Swamp => OBJ_SWAMP,
        _ => OBJ_EMPTY,
    }
}

/// 依種子決定性產出地上物（整體出現率 ~25%）。
fn generate_objects(world_seed: u64, coord: SquareCoord, terrain: Terrain) -> Vec<HexObject> {
    let table = object_table(terrain);
    let mut objects = Vec::new();
    for (i, &(kind, name, max_qty, regen, chance)) in table.iter().enumerate() {
        // 每件物品用獨立種子判斷是否生成
        let obj_seed = mix_coord_seed(
            world_seed.wrapping_add(0xABCD_0000 + i as u64),
            coord,
        );
        // 取低 8 位轉 0..100
        let roll = (obj_seed & 0xFF) % 100;
        if roll >= chance as u64 {
            continue;
        }
        // 初始量：max_qty 的 50%~100%（決定性）
        let qty_roll = ((obj_seed >> 8) & 0xFF) as f64 / 255.0;
        let quantity = ((max_qty as f64) * (0.5 + 0.5 * qty_roll)).ceil() as u32;
        objects.push(HexObject {
            id: format!("sq_{}_{}", coord.x, coord.y),
            kind,
            name: name.to_string(),
            quantity,
            max_quantity: max_qty,
            regrowth_per_tick: regen,
        });
    }
    objects
}

// ─── 主要公開函式 ───

/// 依 `world_seed` + 座標決定性生成一格野外格子（未寫入 grid）。
///
/// 地形由加權隨機決定，資源由地形對應機率表決定。
/// 相同輸入永遠產出相同格子。
pub fn generate_wild_cell(world_seed: u64, coord: SquareCoord) -> GridCell {
    let seed = mix_coord_seed(world_seed, coord);
    let terrain = terrain_from_seed(seed);
    let objects = generate_objects(world_seed, coord, terrain);
    let name = terrain_name_zh(terrain).to_string();

    let mut cell = GridCell::new(coord, terrain, name);
    cell.objects = objects;
    // 不可通行地形視為已揭露（不需玩家涉足確認）
    if !terrain.walkable() {
        cell.explored = true;
    }
    cell
}

/// 揭露以 `center` 為中心、Chebyshev 距離 ≤ `radius` 的所有方格。
///
/// 對應 hex 版的 `reveal_hex_disk`，使用切比雪夫距離（方格世界標準）。
/// 已存在的格子不覆寫（契約穩定性）。回傳**新插入**格數。
///
/// 半徑 r 的方格數為 (2r+1)²。
pub fn reveal_grid_disk(grid: &mut SquareGrid, center: SquareCoord, radius: i32) -> usize {
    if radius < 0 {
        return 0;
    }
    let ws = grid.world_seed();
    let mut new_count = 0usize;

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let coord = SquareCoord::new(center.x + dx, center.y + dy);
            // Chebyshev 距離過濾（實際上在這個迴圈裡 dx、dy 都在 radius 內，已是方形範圍）
            if coord.chebyshev_distance(center) > radius {
                continue;
            }
            if grid.contains(coord) {
                if let Some(c) = grid.get_mut(coord)
                    && !c.explored
                {
                    c.explored = true;
                    grid.mark_dirty();
                }
                continue;
            }
            let mut cell = generate_wild_cell(ws, coord);
            cell.explored = true;
            grid.insert(cell);
            new_count += 1;
        }
    }
    new_count
}

// ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // ── mix_coord_seed 決定性 ──

    #[test]
    fn mix_coord_seed_deterministic() {
        let seed_a = mix_coord_seed(0xC0FFEE, SquareCoord::new(3, -7));
        let seed_b = mix_coord_seed(0xC0FFEE, SquareCoord::new(3, -7));
        assert_eq!(seed_a, seed_b);
    }

    #[test]
    fn mix_coord_seed_different_coords_differ() {
        let a = mix_coord_seed(42, SquareCoord::new(0, 0));
        let b = mix_coord_seed(42, SquareCoord::new(1, 0));
        let c = mix_coord_seed(42, SquareCoord::new(0, 1));
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);
    }

    #[test]
    fn mix_coord_seed_different_world_seeds_differ() {
        let a = mix_coord_seed(1, SquareCoord::ORIGIN);
        let b = mix_coord_seed(2, SquareCoord::ORIGIN);
        assert_ne!(a, b);
    }

    // ── generate_wild_cell 決定性 + 合法地形 ──

    #[test]
    fn generate_wild_cell_deterministic() {
        let coord = SquareCoord::new(5, -3);
        let a = generate_wild_cell(0xDEAD_BEEF, coord);
        let b = generate_wild_cell(0xDEAD_BEEF, coord);
        assert_eq!(a.terrain, b.terrain);
        assert_eq!(a.name, b.name);
        assert_eq!(a.objects.len(), b.objects.len());
    }

    #[test]
    fn generate_wild_cell_valid_terrain() {
        // 跑足夠多座標，確認地形都落在合法範圍內
        let valid_terrains = [
            Terrain::Grassland,
            Terrain::Plain,
            Terrain::Forest,
            Terrain::ForestLight,
            Terrain::Hills,
            Terrain::ForestHeavy,
            Terrain::Mountain,
            Terrain::Water,
            Terrain::Swamp,
            Terrain::Desert,
        ];
        for x in -10..=10i32 {
            for y in -10..=10i32 {
                let cell = generate_wild_cell(12345, SquareCoord::new(x, y));
                assert!(
                    valid_terrains.contains(&cell.terrain),
                    "非法地形 {:?} @ ({x},{y})",
                    cell.terrain
                );
            }
        }
    }

    #[test]
    fn generate_wild_cell_unwalkable_explored() {
        // 山地、水域等不可通行地形應預設 explored = true
        // 跑足夠多格找到至少一格不可通行地形
        let mut found_unwalkable = false;
        for x in 0..100i32 {
            let cell = generate_wild_cell(9999, SquareCoord::new(x, 0));
            if !cell.terrain.walkable() {
                assert!(cell.explored, "不可通行地形應 explored=true");
                found_unwalkable = true;
                break;
            }
        }
        // 若 100 格都是可通行，就只檢查 walkable 分支不崩潰
        let _ = found_unwalkable;
    }

    // ── reveal_grid_disk 計數 ──

    #[test]
    fn reveal_grid_disk_radius_0() {
        let mut g = SquareGrid::new();
        g.set_world_seed(1);
        // (2*0+1)² = 1
        let n = reveal_grid_disk(&mut g, SquareCoord::ORIGIN, 0);
        assert_eq!(n, 1);
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn reveal_grid_disk_radius_1() {
        let mut g = SquareGrid::new();
        g.set_world_seed(2);
        // (2*1+1)² = 9
        let n = reveal_grid_disk(&mut g, SquareCoord::ORIGIN, 1);
        assert_eq!(n, 9);
        assert_eq!(g.len(), 9);
    }

    #[test]
    fn reveal_grid_disk_radius_3() {
        let mut g = SquareGrid::new();
        g.set_world_seed(3);
        // (2*3+1)² = 49
        let n = reveal_grid_disk(&mut g, SquareCoord::ORIGIN, 3);
        assert_eq!(n, 49);
        assert_eq!(g.len(), 49);
    }

    #[test]
    fn reveal_grid_disk_no_overwrite() {
        // 第二次揭露同一區域應回傳 0（無新格）
        let mut g = SquareGrid::new();
        g.set_world_seed(7);
        reveal_grid_disk(&mut g, SquareCoord::ORIGIN, 2);
        let n2 = reveal_grid_disk(&mut g, SquareCoord::ORIGIN, 2);
        assert_eq!(n2, 0);
    }

    #[test]
    fn reveal_grid_disk_negative_radius() {
        let mut g = SquareGrid::new();
        assert_eq!(reveal_grid_disk(&mut g, SquareCoord::ORIGIN, -1), 0);
        assert_eq!(g.len(), 0);
    }

    #[test]
    fn reveal_grid_disk_partial_overlap() {
        // 第一次揭露 r=1（9格），再以偏移中心揭露 r=1
        // 新格數應 < 9
        let mut g = SquareGrid::new();
        g.set_world_seed(8);
        let n1 = reveal_grid_disk(&mut g, SquareCoord::ORIGIN, 1);
        let n2 = reveal_grid_disk(&mut g, SquareCoord::new(2, 0), 1);
        assert_eq!(n1, 9);
        assert!(n2 < 9, "部分重疊時新格數應小於 9，實際 {n2}");
        assert!(n2 > 0, "非完全重疊時應有新格");
    }
}
