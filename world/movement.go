// Package world 負責移動、碰撞與路徑。本檔為移動目標與阻擋判定，對齊 spec_移動與地圖。
package world

// CanMoveTo 檢查地圖格 (x,y) 是否可進入：不越界且地形非阻擋。
// 參數：g 為地圖；x,y 為格點。
// 回傳：true 表示可進入。無副作用。
func CanMoveTo(g *Grid, x, y int) bool {
	t := g.At(x, y)
	return !t.Blocking()
}
