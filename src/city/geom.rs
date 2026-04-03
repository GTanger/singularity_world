// 城市 GeoJSON 幾何輔助：點在多邊形內、距離、面積；不依賴外部 geo crate。

/// 兩點歐幾里得距離。
pub fn dist_xy(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = ax - bx;
    let dy = ay - by;
    (dx * dx + dy * dy).sqrt()
}

/// 多邊形面積（有號；外環逆時針為正），用於建築排序。
pub fn polygon_area_signed(ring: &[(f64, f64)]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    let mut a = 0.0;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        a += ring[i].0 * ring[j].1;
        a -= ring[j].0 * ring[i].1;
    }
    a * 0.5
}

/// 多邊形面積絕對值。
pub fn polygon_area_abs(ring: &[(f64, f64)]) -> f64 {
    polygon_area_signed(ring).abs()
}

/// 射線法：點是否在簡單多邊形內（邊界上視為內）。
pub fn point_in_polygon(px: f64, py: f64, ring: &[(f64, f64)]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let n = ring.len();
    for i in 0..n {
        let (x1, y1) = ring[i];
        let (x2, y2) = ring[(i + 1) % n];
        let dy = y2 - y1;
        if dy.abs() < f64::EPSILON {
            continue;
        }
        if (y1 > py) == (y2 > py) {
            continue;
        }
        let xinters = (x2 - x1) * (py - y1) / dy + x1;
        if px < xinters {
            inside = !inside;
        }
    }
    inside
}

/// 點到線段的最短距離。
pub fn dist_point_segment(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let abx = bx - ax;
    let aby = by - ay;
    let apx = px - ax;
    let apy = py - ay;
    let ab2 = abx * abx + aby * aby;
    if ab2 < f64::EPSILON {
        return dist_xy(px, py, ax, ay);
    }
    let mut t = (apx * abx + apy * aby) / ab2;
    t = t.clamp(0.0, 1.0);
    let cx = ax + t * abx;
    let cy = ay + t * aby;
    dist_xy(px, py, cx, cy)
}

/// 點到折線的最短距離。
pub fn dist_point_polyline(px: f64, py: f64, line: &[(f64, f64)]) -> f64 {
    if line.len() < 2 {
        return line
            .first()
            .map(|p| dist_xy(px, py, p.0, p.1))
            .unwrap_or(f64::MAX);
    }
    let mut dmin = f64::MAX;
    for w in line.windows(2) {
        let d = dist_point_segment(px, py, w[0].0, w[0].1, w[1].0, w[1].1);
        if d < dmin {
            dmin = d;
        }
    }
    dmin
}

/// 點到多邊形邊界的最短距離。
pub fn dist_point_polygon_boundary(px: f64, py: f64, ring: &[(f64, f64)]) -> f64 {
    if ring.len() < 2 {
        return f64::MAX;
    }
    let mut dmin = f64::MAX;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        let d = dist_point_segment(px, py, ring[i].0, ring[i].1, ring[j].0, ring[j].1);
        if d < dmin {
            dmin = d;
        }
    }
    dmin
}

/// 依座標差回傳八方位中文（細街網／除錯時可用；語意層出口改以「往××」為主）。
#[allow(clippy::manual_range_contains)] // 西向須含 ±180° 邊界，與 `.contains` 半開區間不易對齊
#[allow(dead_code)]
pub fn bearing_label(dx: f64, dy: f64) -> &'static str {
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return "前";
    }
    let ang = dy.atan2(dx);
    let deg = ang.to_degrees();
    if deg >= -22.5 && deg < 22.5 {
        "東"
    } else if deg >= 22.5 && deg < 67.5 {
        "東北"
    } else if deg >= 67.5 && deg < 112.5 {
        "北"
    } else if deg >= 112.5 && deg < 157.5 {
        "西北"
    } else if deg >= 157.5 || deg < -157.5 {
        "西"
    } else if deg >= -157.5 && deg < -112.5 {
        "西南"
    } else if deg >= -112.5 && deg < -67.5 {
        "南"
    } else {
        "東南"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_area() {
        let r = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert!((polygon_area_abs(&r) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn point_inside() {
        let r = vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
        assert!(point_in_polygon(1.0, 1.0, &r));
        assert!(!point_in_polygon(3.0, 1.0, &r));
    }
}
