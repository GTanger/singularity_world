use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────
// Direction — 四方向列舉（2026-04-18：八向→四向）
// ─────────────────────────────────────────────────────────────────

/// 正方格的四個移動方向（只走正交、不走對角）。
/// x 軸向東為正，y 軸向北為正。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    /// 所有四個方向，依北→東→南→西順序排列。
    pub const ALL: [Direction; 4] = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];

    /// 各方向對應的座標增量 `(dx, dy)`。
    /// x 軸向東為正，y 軸向北為正。
    /// N=(0,1), E=(1,0), S=(0,-1), W=(-1,0)
    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::North => (0, 1),
            Direction::East  => (1, 0),
            Direction::South => (0, -1),
            Direction::West  => (-1, 0),
        }
    }

    /// 方向的繁體中文標籤。
    pub fn label_zh(self) -> &'static str {
        match self {
            Direction::North => "北",
            Direction::East  => "東",
            Direction::South => "南",
            Direction::West  => "西",
        }
    }

    /// 回傳正好相反的方向。
    pub fn opposite(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East  => Direction::West,
            Direction::South => Direction::North,
            Direction::West  => Direction::East,
        }
    }

    /// 從繁體中文方向標籤解析。回傳 `None` 表示無法匹配。
    pub fn from_zh(s: &str) -> Option<Direction> {
        match s.trim() {
            "北" => Some(Direction::North),
            "東" => Some(Direction::East),
            "南" => Some(Direction::South),
            "西" => Some(Direction::West),
            _ => None,
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label_zh())
    }
}

// ─────────────────────────────────────────────────────────────────
// SquareCoord — 正方格座標
// ─────────────────────────────────────────────────────────────────

/// 正方格座標。x 軸向東為正，y 軸向北為正。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SquareCoord {
    pub x: i32,
    pub y: i32,
}

impl SquareCoord {
    /// 原點 (0, 0)。
    pub const ORIGIN: SquareCoord = SquareCoord { x: 0, y: 0 };

    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// 沿指定方向移動一格，回傳鄰格座標。
    pub fn neighbor(self, dir: Direction) -> SquareCoord {
        let (dx, dy) = dir.delta();
        SquareCoord {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    /// 回傳所有四個鄰格，順序與 `Direction::ALL` 相同。
    pub fn neighbors(self) -> [SquareCoord; 4] {
        Direction::ALL.map(|d| self.neighbor(d))
    }

    /// Manhattan 距離：`|dx| + |dy|`。
    /// 正方格四方向移動時，此即兩格之間的最短步數。
    pub fn manhattan_distance(self, other: SquareCoord) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Chebyshev 距離：`max(|dx|, |dy|)`。
    /// 四向制下仍保留此函數供揭露半徑（菱形 vs 方形視野）等用途使用。
    pub fn chebyshev_distance(self, other: SquareCoord) -> i32 {
        let dx = (self.x - other.x).abs();
        let dy = (self.y - other.y).abs();
        dx.max(dy)
    }

    /// 若 `other` 與 `self` 正交相鄰（Manhattan 距離 = 1），回傳對應方向；否則回傳 `None`。
    pub fn direction_to(self, other: SquareCoord) -> Option<Direction> {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        Direction::ALL.into_iter().find(|d| d.delta() == (dx, dy))
    }
}

impl std::fmt::Display for SquareCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}

// ─────────────────────────────────────────────────────────────────
// Room ID 工具函式
// ─────────────────────────────────────────────────────────────────

/// 從座標產生正方格虛擬房間 id，格式：`grid:{x}:{y}`。
#[must_use]
pub fn grid_room_id_from_coord(x: i32, y: i32) -> String {
    format!("grid:{x}:{y}")
}

/// 解析 [`grid_room_id_from_coord`] 產生的 id，非此格式則回傳 `None`。
pub fn parse_grid_room_id(id: &str) -> Option<SquareCoord> {
    let rest = id.strip_prefix("grid:")?;
    let (x_str, y_str) = rest.split_once(':')?;
    if y_str.contains(':') {
        return None;
    }
    let x: i32 = x_str.parse().ok()?;
    let y: i32 = y_str.parse().ok()?;
    Some(SquareCoord { x, y })
}

// ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // ── Direction ──

    #[test]
    fn opposite_is_involution() {
        for dir in Direction::ALL {
            assert_eq!(dir.opposite().opposite(), dir, "方向 {dir} 的 opposite 不是對合");
        }
    }

    #[test]
    fn delta_opposite_cancel() {
        for dir in Direction::ALL {
            let (dx, dy) = dir.delta();
            let (odx, ody) = dir.opposite().delta();
            assert_eq!(dx + odx, 0, "方向 {dir} 的 dx 與 opposite 不相消");
            assert_eq!(dy + ody, 0, "方向 {dir} 的 dy 與 opposite 不相消");
        }
    }

    // ── neighbors ──

    #[test]
    fn neighbors_count() {
        let c = SquareCoord::ORIGIN;
        assert_eq!(c.neighbors().len(), 4);
    }

    #[test]
    fn neighbors_are_adjacent() {
        let c = SquareCoord::new(3, -5);
        for n in c.neighbors() {
            assert_eq!(
                c.manhattan_distance(n),
                1,
                "鄰格 {n} 的 Manhattan 距離不為 1"
            );
        }
    }

    #[test]
    fn neighbors_unique() {
        let ns = SquareCoord::ORIGIN.neighbors();
        for i in 0..ns.len() {
            for j in (i + 1)..ns.len() {
                assert_ne!(ns[i], ns[j], "鄰格重複：{} 和 {}", ns[i], ns[j]);
            }
        }
    }

    // ── manhattan_distance ──

    #[test]
    fn manhattan_self_zero() {
        let c = SquareCoord::new(7, -3);
        assert_eq!(c.manhattan_distance(c), 0);
    }

    #[test]
    fn manhattan_symmetric() {
        let a = SquareCoord::new(0, 0);
        let b = SquareCoord::new(3, -4);
        assert_eq!(a.manhattan_distance(b), b.manhattan_distance(a));
    }

    #[test]
    fn manhattan_known_values() {
        let origin = SquareCoord::ORIGIN;
        assert_eq!(origin.manhattan_distance(SquareCoord::new(2, 2)), 4);
        assert_eq!(origin.manhattan_distance(SquareCoord::new(3, 0)), 3);
        assert_eq!(origin.manhattan_distance(SquareCoord::new(1, 5)), 6);
    }

    // ── direction_to ──

    #[test]
    fn direction_to_all_neighbors() {
        let a = SquareCoord::new(-2, 4);
        for dir in Direction::ALL {
            let b = a.neighbor(dir);
            assert_eq!(
                a.direction_to(b),
                Some(dir),
                "direction_to 在方向 {dir} 失敗"
            );
        }
    }

    #[test]
    fn direction_to_opposite() {
        let a = SquareCoord::ORIGIN;
        for dir in Direction::ALL {
            let b = a.neighbor(dir);
            assert_eq!(
                b.direction_to(a),
                Some(dir.opposite()),
                "反向 direction_to 在方向 {dir} 失敗"
            );
        }
    }

    #[test]
    fn direction_to_non_adjacent_is_none() {
        let a = SquareCoord::ORIGIN;
        // 對角方向不算相鄰（四向制）
        assert_eq!(a.direction_to(SquareCoord::new(1, 1)), None);
        assert_eq!(a.direction_to(SquareCoord::new(-1, -1)), None);
        // 距離 2 不算相鄰
        assert_eq!(a.direction_to(SquareCoord::new(2, 0)), None);
        assert_eq!(a.direction_to(SquareCoord::new(0, 2)), None);
        // 自身也不算相鄰
        assert_eq!(a.direction_to(a), None);
    }

    // ── room_id round-trip ──

    #[test]
    fn room_id_format() {
        assert_eq!(grid_room_id_from_coord(3, -7), "grid:3:-7");
        assert_eq!(grid_room_id_from_coord(0, 0), "grid:0:0");
        assert_eq!(grid_room_id_from_coord(-100, 42), "grid:-100:42");
    }

    #[test]
    fn room_id_roundtrip() {
        let cases = [
            SquareCoord::new(0, 0),
            SquareCoord::new(1, -1),
            SquareCoord::new(-999, 1234),
            SquareCoord::new(i32::MAX, i32::MIN),
        ];
        for c in cases {
            let id = grid_room_id_from_coord(c.x, c.y);
            let parsed = parse_grid_room_id(&id)
                .unwrap_or_else(|| panic!("parse_grid_room_id 無法解析 {id:?}"));
            assert_eq!(parsed, c, "round-trip 失敗：{id:?}");
        }
    }

    #[test]
    fn room_id_invalid_inputs() {
        assert_eq!(parse_grid_room_id(""), None);
        assert_eq!(parse_grid_room_id("hex:1:2"), None);
        assert_eq!(parse_grid_room_id("grid:"), None);
        assert_eq!(parse_grid_room_id("grid:1"), None);
        assert_eq!(parse_grid_room_id("grid:a:b"), None);
        assert_eq!(parse_grid_room_id("grid:1:2:3"), None);
    }
}
