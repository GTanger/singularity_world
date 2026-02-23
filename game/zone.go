// Package game 負責視野區域管理與視野內即時模擬。本檔為視野範圍內實體列表，對齊決策 004 視野內才即時模擬。
package game

// ViewRadius 為以角色為中心的視野半徑（格數），可調參數建議集中於 config。
const ViewRadius = 10

// ChunkSize 為單一區塊邊長（格數）；預設 151（22,801 格）。當前區塊常亮、區塊外黑、越界瞬間載入新區塊並關閉上個區塊。
const ChunkSize = 151

// InView 檢查 (ex,ey) 是否在 (cx,cy) 的視野範圍內（曼哈頓或歐幾里得；此處用簡單方形）。
func InView(cx, cy, ex, ey int) bool {
	dx := ex - cx
	if dx < 0 {
		dx = -dx
	}
	dy := ey - cy
	if dy < 0 {
		dy = -dy
	}
	return dx <= ViewRadius && dy <= ViewRadius
}

// ChunkIndex 回傳世界座標 (wx, wy) 所屬的區塊索引 (cx, cy)。區塊邊長為 ChunkSize；負座標用 floor 除法。
func ChunkIndex(wx, wy int) (cx, cy int) {
	if wx >= 0 {
		cx = wx / ChunkSize
	} else {
		cx = (wx - ChunkSize + 1) / ChunkSize
	}
	if wy >= 0 {
		cy = wy / ChunkSize
	} else {
		cy = (wy - ChunkSize + 1) / ChunkSize
	}
	return cx, cy
}

// ChunkBounds 回傳區塊 (cx, cy) 在世界座標上的範圍 [x0, x1)、[y0, y1)，含 x0 不含 x1，格數為 ChunkSize×ChunkSize。
func ChunkBounds(cx, cy int) (x0, y0, x1, y1 int) {
	x0 = cx * ChunkSize
	y0 = cy * ChunkSize
	x1 = x0 + ChunkSize
	y1 = y0 + ChunkSize
	return x0, y0, x1, y1
}

// InChunk 回傳世界格 (wx, wy) 是否落在區塊 (cx, cy) 內。
func InChunk(wx, wy, cx, cy int) bool {
	x0, y0, x1, y1 := ChunkBounds(cx, cy)
	return wx >= x0 && wx < x1 && wy >= y0 && wy < y1
}
