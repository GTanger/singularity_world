// Package game 當前區塊與視野內實體查詢，供 1.2.3 視野 API／WS 推播使用。
package game

import (
	"database/sql"
	"math/rand"

	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/world"
)

// ChunkView 表示「當前區塊地形 ＋ 視野內實體」，供前端繪製與 RunViewSimulation 觀測者位置對接。
type ChunkView struct {
	Cx      int                 // 區塊 X 索引
	Cy      int                 // 區塊 Y 索引
	Grid    *world.Grid         // 151×151 地形，區塊內格點
	Entities []*entity.Character // 在 (observerX, observerY) 視野半徑內的實體（含玩家）
}

// GetChunkAndEntitiesInView 依觀測者世界座標 (wx, wy) 載入當前區塊地形，並回傳視野內實體。
// mapsPath 為區塊地圖目錄（例：config.MapsPath）。若區塊檔不存在則回傳整塊草。
func GetChunkAndEntitiesInView(database *sql.DB, wx, wy int, mapsPath string) (*ChunkView, error) {
	cx, cy := ChunkIndex(wx, wy)
	grid, err := world.LoadChunk(cx, cy, mapsPath)
	if err != nil {
		return nil, err
	}
	x0, y0, x1, y1 := ChunkBounds(cx, cy)
	// 視野可能超出當前區塊，用擴大一格的範圍取實體再篩選
	xMin, xMax := x0-ViewRadius, x1-1+ViewRadius
	yMin, yMax := y0-ViewRadius, y1-1+ViewRadius
	list, err := db.GetEntitiesInBox(database, xMin, xMax, yMin, yMax, "")
	if err != nil {
		return nil, err
	}
	var inView []*entity.Character
	for _, e := range list {
		if InView(wx, wy, e.X, e.Y) {
			inView = append(inView, e)
		}
	}
	return &ChunkView{Cx: cx, Cy: cy, Grid: grid, Entities: inView}, nil
}

// ChunkRows 將區塊 Grid 轉成扁平 []string，長度 151×151，索引 = ly*151+lx；供 WS ViewMsg 使用。
func (v *ChunkView) ChunkRows() []string {
	rows, _ := v.ChunkRowsWithColors()
	return rows
}

// ChunkRowsWithColors 依 map_terrain_world「顯示時從 chars 亂序取字與對應 colors」回傳顯示用字與色。
// 每格用 world.Display 依區塊+格位種子取候選字與對應色，同一區塊同格每次相同（不閃爍）。
func (v *ChunkView) ChunkRowsWithColors() (rows []string, colors []string) {
	const size = 151 * 151
	rows = make([]string, 0, size)
	colors = make([]string, 0, size)
	for y := 0; y < 151 && y < v.Grid.Height; y++ {
		for x := 0; x < 151 && x < v.Grid.Width; x++ {
			t := v.Grid.Cells[y*v.Grid.Width+x]
			seed := int64(v.Cx)*1000000 + int64(v.Cy)*1000 + int64(y)*151 + int64(x)
			rng := rand.New(rand.NewSource(seed))
			ch, col := world.Display(t, rng)
			rows = append(rows, ch)
			colors = append(colors, col)
		}
	}
	for len(rows) < size {
		ch, col := world.Display(world.TerrainGrass, nil)
		rows = append(rows, ch)
		colors = append(colors, col)
	}
	return rows, colors
}

// CanMoveToWorld 檢查世界座標 (wx, wy) 是否在當前區塊內且可通行（非牆/關/川/水）。
func (v *ChunkView) CanMoveToWorld(wx, wy int) bool {
	if !InChunk(wx, wy, v.Cx, v.Cy) {
		return false
	}
	x0, y0, _, _ := ChunkBounds(v.Cx, v.Cy)
	lx, ly := wx-x0, wy-y0
	return !v.Grid.At(lx, ly).Blocking()
}
