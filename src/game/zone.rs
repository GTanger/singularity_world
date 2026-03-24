// 視野區域與區塊索引幾何，對齊 Go game/zone.go。

/// 以角色為中心的視野半徑（格數）。
pub const VIEW_RADIUS: i32 = 10;

/// 單一區塊邊長（格數）；預設 151。
pub const CHUNK_SIZE: i32 = 151;

/// 檢查 `(ex,ey)` 是否在 `(cx,cy)` 的方形視野內。
#[must_use]
pub fn in_view(cx: i32, cy: i32, ex: i32, ey: i32) -> bool {
    let dx = (ex - cx).abs();
    let dy = (ey - cy).abs();
    dx <= VIEW_RADIUS && dy <= VIEW_RADIUS
}

/// 世界座標 `(wx, wy)` 所屬區塊索引 `(cx, cy)`；負座標用 floor 除法。
#[must_use]
pub fn chunk_index(wx: i32, wy: i32) -> (i32, i32) {
    let cx = if wx >= 0 {
        wx / CHUNK_SIZE
    } else {
        (wx - CHUNK_SIZE + 1) / CHUNK_SIZE
    };
    let cy = if wy >= 0 {
        wy / CHUNK_SIZE
    } else {
        (wy - CHUNK_SIZE + 1) / CHUNK_SIZE
    };
    (cx, cy)
}

/// 區塊 `(cx,cy)` 在世界座標上的範圍 `[x0,x1)×[y0,y1)`。
#[must_use]
pub fn chunk_bounds(cx: i32, cy: i32) -> (i32, i32, i32, i32) {
    let x0 = cx * CHUNK_SIZE;
    let y0 = cy * CHUNK_SIZE;
    let x1 = x0 + CHUNK_SIZE;
    let y1 = y0 + CHUNK_SIZE;
    (x0, y0, x1, y1)
}

/// 世界格 `(wx, wy)` 是否落在區塊 `(cx, cy)` 內。
#[must_use]
pub fn in_chunk(wx: i32, wy: i32, cx: i32, cy: i32) -> bool {
    let (x0, y0, x1, y1) = chunk_bounds(cx, cy);
    wx >= x0 && wx < x1 && wy >= y0 && wy < y1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_view_center() {
        assert!(in_view(0, 0, 5, -3));
        assert!(!in_view(0, 0, 11, 0));
    }

    #[test]
    fn chunk_index_negative() {
        assert_eq!(chunk_index(-1, 0), (-1, 0));
        assert_eq!(chunk_index(-151, 0), (-1, 0));
        assert_eq!(chunk_index(-152, 0), (-2, 0));
    }

    #[test]
    fn in_chunk_bounds() {
        let (cx, cy) = (1, 2);
        let (x0, y0, x1, y1) = chunk_bounds(cx, cy);
        assert!(in_chunk(x0, y0, cx, cy));
        assert!(!in_chunk(x1, y0, cx, cy));
        assert!(!in_chunk(x0, y1, cx, cy));
    }
}
