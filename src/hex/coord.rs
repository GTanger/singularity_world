use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 平頂六角格的六個方向（axial 座標系）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HexDir {
    E,
    Ne,
    Nw,
    W,
    Sw,
    Se,
}

impl HexDir {
    pub const ALL: [HexDir; 6] = [
        HexDir::E,
        HexDir::Ne,
        HexDir::Nw,
        HexDir::W,
        HexDir::Sw,
        HexDir::Se,
    ];

    /// axial 座標增量 (dq, dr)
    pub fn delta(self) -> (i32, i32) {
        match self {
            HexDir::E => (1, 0),
            HexDir::Ne => (1, -1),
            HexDir::Nw => (0, -1),
            HexDir::W => (-1, 0),
            HexDir::Sw => (-1, 1),
            HexDir::Se => (0, 1),
        }
    }

    pub fn opposite(self) -> HexDir {
        match self {
            HexDir::E => HexDir::W,
            HexDir::Ne => HexDir::Sw,
            HexDir::Nw => HexDir::Se,
            HexDir::W => HexDir::E,
            HexDir::Sw => HexDir::Ne,
            HexDir::Se => HexDir::Nw,
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            HexDir::E => "東",
            HexDir::Ne => "東北",
            HexDir::Nw => "西北",
            HexDir::W => "西",
            HexDir::Sw => "西南",
            HexDir::Se => "東南",
        }
    }
}

// ─────────────────────────────────────────────────────────────────

/// [Hex格座標與cell_id規約](../../docs/reference/Hex格座標與cell_id規約.md)：`q,r` 無空白
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CellIdParseError {
    #[error("cell_id 須為「q,r」兩個整數")]
    InvalidFormat,
    #[error("整數解析失敗")]
    ParseInt(#[from] std::num::ParseIntError),
}

// ─────────────────────────────────────────────────────────────────

/// Axial 六角格座標（平頂佈局）
///
/// q 軸向右，r 軸向右下。cube 第三軸 s = -q - r。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

/// 與 `Room.id` 銜接的六角「虛擬房間」id：`hex:{q}:{r}`。
#[must_use]
pub fn hex_room_id_from_coord(q: i32, r: i32) -> String {
    format!("hex:{q}:{r}")
}

/// 解析 [`hex_room_id_from_coord`] 產生的 id；非此格式則 `None`。
pub fn parse_hex_room_id(id: &str) -> Option<(i32, i32)> {
    let rest = id.strip_prefix("hex:")?;
    let mut parts = rest.splitn(3, ':');
    let q = parts.next()?.parse().ok()?;
    let r = parts.next()?.parse().ok()?;
    Some((q, r))
}

impl HexCoord {
    pub const ORIGIN: Self = Self { q: 0, r: 0 };

    pub fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// cube 第三軸
    pub fn s(self) -> i32 {
        -self.q - self.r
    }

    /// 兩格之間的六角距離
    pub fn distance(self, other: Self) -> u32 {
        let dq = (self.q - other.q).unsigned_abs();
        let dr = (self.r - other.r).unsigned_abs();
        let ds = (self.s() - other.s()).unsigned_abs();
        dq.max(dr).max(ds)
    }

    pub fn is_adjacent(self, other: Self) -> bool {
        self.distance(other) == 1
    }

    pub fn neighbor(self, dir: HexDir) -> Self {
        let (dq, dr) = dir.delta();
        Self {
            q: self.q + dq,
            r: self.r + dr,
        }
    }

    pub fn neighbors(self) -> [HexCoord; 6] {
        HexDir::ALL.map(|d| self.neighbor(d))
    }

    /// 若 other 與 self 相鄰，回傳從 self 看過去的方向
    pub fn direction_to(self, other: Self) -> Option<HexDir> {
        let dq = other.q - self.q;
        let dr = other.r - self.r;
        HexDir::ALL.into_iter().find(|d| d.delta() == (dq, dr))
    }

    /// 轉成像素座標（hex 外接圓半徑 = hex_r）
    pub fn to_pixel(self, hex_r: f64) -> (f64, f64) {
        let sqrt3 = 3.0_f64.sqrt();
        let x = hex_r * sqrt3 * (self.q as f64 + self.r as f64 / 2.0);
        let y = hex_r * 1.5 * self.r as f64;
        (x, y)
    }

    /// 從像素座標反推最近的 hex 座標
    pub fn from_pixel(x: f64, y: f64, hex_r: f64) -> Self {
        let sqrt3 = 3.0_f64.sqrt();
        let qf = (sqrt3 / 3.0 * x - y / 3.0) / hex_r;
        let rf = (2.0 / 3.0 * y) / hex_r;
        cube_round(qf, rf)
    }

    /// axial → even-q offset (col, row)
    pub fn to_even_q(self) -> (i32, i32) {
        let col = self.q;
        let row = self.r + (self.q + (self.q & 1)) / 2;
        (col, row)
    }

    /// even-q offset (col, row) → axial
    pub fn from_even_q(col: i32, row: i32) -> Self {
        let q = col;
        let r = row - (col + (col & 1)) / 2;
        Self { q, r }
    }

    /// 正規 `cell_id` 字串（`{q},{r}`，無括號）
    pub fn to_cell_id(self) -> String {
        format!("{},{}", self.q, self.r)
    }
}

impl FromStr for HexCoord {
    type Err = CellIdParseError;

    /// 解析 `cell_id`（`{q},{r}`，可含前後空白）
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (a, b) = s.split_once(',').ok_or(CellIdParseError::InvalidFormat)?;
        let q: i32 = a.trim().parse()?;
        let r: i32 = b.trim().parse()?;
        Ok(Self { q, r })
    }
}

fn cube_round(qf: f64, rf: f64) -> HexCoord {
    let sf = -qf - rf;
    let mut rq = qf.round();
    let mut rr = rf.round();
    let rs = sf.round();
    let dq = (rq - qf).abs();
    let dr = (rr - rf).abs();
    let ds = (rs - sf).abs();
    if dq > dr && dq > ds {
        rq = -rr - rs;
    } else if dr > ds {
        rr = -rq - rs;
    }
    HexCoord {
        q: rq as i32,
        r: rr as i32,
    }
}

impl std::fmt::Display for HexCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.q, self.r)
    }
}

impl std::fmt::Display for HexDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label_zh())
    }
}

// ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_distance_zero() {
        let a = HexCoord::ORIGIN;
        assert_eq!(a.distance(a), 0);
    }

    #[test]
    fn adjacent_distance_one() {
        let a = HexCoord::ORIGIN;
        for n in a.neighbors() {
            assert_eq!(a.distance(n), 1, "neighbor {n} should be distance 1");
            assert!(a.is_adjacent(n));
        }
    }

    #[test]
    fn direction_roundtrip() {
        let a = HexCoord::new(3, -2);
        for dir in HexDir::ALL {
            let b = a.neighbor(dir);
            assert_eq!(a.direction_to(b), Some(dir));
            assert_eq!(b.direction_to(a), Some(dir.opposite()));
        }
    }

    #[test]
    fn non_adjacent_returns_none() {
        let a = HexCoord::ORIGIN;
        let far = HexCoord::new(2, 0);
        assert_eq!(a.direction_to(far), None);
        assert!(!a.is_adjacent(far));
    }

    #[test]
    fn even_q_roundtrip() {
        for q in -5..=5 {
            for r in -5..=5 {
                let c = HexCoord::new(q, r);
                let (col, row) = c.to_even_q();
                let back = HexCoord::from_even_q(col, row);
                assert_eq!(c, back, "even-q roundtrip failed for {c}");
            }
        }
    }

    #[test]
    fn pixel_roundtrip() {
        let hex_r = 28.0;
        for q in -10..=10 {
            for r in -10..=10 {
                let c = HexCoord::new(q, r);
                let (px, py) = c.to_pixel(hex_r);
                let back = HexCoord::from_pixel(px, py, hex_r);
                assert_eq!(c, back, "pixel roundtrip failed for {c}");
            }
        }
    }

    #[test]
    fn opposite_is_involution() {
        for dir in HexDir::ALL {
            assert_eq!(dir.opposite().opposite(), dir);
        }
    }

    #[test]
    fn distance_two_hops() {
        let a = HexCoord::ORIGIN;
        let b = a.neighbor(HexDir::E).neighbor(HexDir::E);
        assert_eq!(a.distance(b), 2);
    }

    #[test]
    fn cell_id_roundtrip() {
        let c = HexCoord::new(-3, 7);
        let s = c.to_cell_id();
        assert_eq!(s, "-3,7");
        assert_eq!(HexCoord::from_str(&s).unwrap(), c);
        assert_eq!(" -3 , 7 ".parse::<HexCoord>().unwrap(), c);
    }

    #[test]
    fn cell_id_invalid() {
        assert!(HexCoord::from_str("").is_err());
        assert!(HexCoord::from_str("3").is_err());
        assert!(HexCoord::from_str("a,b").is_err());
    }
}
