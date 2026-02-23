// Package world 負責地圖格點、地形、阻擋與移動碰撞。本檔為地圖與地形，對齊 spec_移動與地圖。
// 第一版可做清單 1.2.1：大地圖格點資料（Grid）、地形字（Terrain）、阻擋表（Blocking）；經 1.2.3 區塊載入與 ChunkView 使用。
package world

// Terrain 代表單格地形類型，大地圖只存此一字；對齊 map_terrain_world、地形字對照表。
type Terrain string

const (
	TerrainWall       Terrain = "牆"
	TerrainDoor       Terrain = "門"
	TerrainDoorClosed Terrain = "關"
	TerrainRoad       Terrain = "道"
	TerrainAlley      Terrain = "巷" // 房屋間距小成巷，與道／地同色系（城市底色）
	TerrainGrass    Terrain = "草"
	TerrainTree     Terrain = "木"
	TerrainMountain Terrain = "山"
	TerrainStone    Terrain = "石"
	TerrainSwamp    Terrain = "沼"
	TerrainRiver    Terrain = "川"
	TerrainWater    Terrain = "水"
	TerrainWasteland Terrain = "荒"
	TerrainLava     Terrain = "火"
	TerrainIce      Terrain = "冰"
	TerrainFarm     Terrain = "田"
	TerrainValley   Terrain = "谷"
	TerrainFog      Terrain = "霧"
	TerrainFloor    Terrain = "地" // 房屋內部地板，單一字，顏色與當前城市底色相同
)

// Blocking 回傳該地形是否阻擋移動。牆、關、川、水阻擋；門（開門）、道、草等可通行。
func (t Terrain) Blocking() bool {
	return t == TerrainWall || t == TerrainDoorClosed || t == TerrainRiver || t == TerrainWater
}

// Grid 代表大地圖格點資料，每格一個地形；人物尺寸＝一格。
type Grid struct {
	Width   int
	Height  int
	Cells   []Terrain
}

// At 回傳 (x,y) 格的地形；越界視為牆（阻擋）。
func (g *Grid) At(x, y int) Terrain {
	if x < 0 || x >= g.Width || y < 0 || y >= g.Height {
		return TerrainWall
	}
	return g.Cells[y*g.Width+x]
}

// NewGrid 建立寬高為 w,h 的空地圖，預設為草。
func NewGrid(w, h int) *Grid {
	cells := make([]Terrain, w*h)
	for i := range cells {
		cells[i] = TerrainGrass
	}
	return &Grid{Width: w, Height: h, Cells: cells}
}
