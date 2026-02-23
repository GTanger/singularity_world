// Package world 區塊地圖載入：從 data/maps/{cx}_{cy}.txt 讀取 151×151 地形字。對齊 data/maps/README.txt。
package world

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"unicode"
)

// runeToTerrain 將檔案中的單字 rune 對應到 Terrain；未對應者視為草。
var runeToTerrain map[rune]Terrain

func init() {
	runeToTerrain = make(map[rune]Terrain)
	for _, t := range []Terrain{
		TerrainWall, TerrainDoor, TerrainDoorClosed, TerrainRoad, TerrainAlley,
		TerrainGrass, TerrainTree, TerrainMountain, TerrainStone, TerrainSwamp,
		TerrainRiver, TerrainWater, TerrainWasteland, TerrainLava, TerrainIce,
		TerrainFarm, TerrainValley, TerrainFog, TerrainFloor,
	} {
		runes := []rune(string(t))
		if len(runes) >= 1 {
			runeToTerrain[runes[0]] = t
		}
	}
	// map_terrain_world 候選字：檔案若用候選字也對應到同一地形鍵，避免誤判為草
	for _, r := range []rune("艸芔茻") { runeToTerrain[r] = TerrainGrass }
	for _, r := range []rune("林森") { runeToTerrain[r] = TerrainTree }
	for _, r := range []rune("岳巒") { runeToTerrain[r] = TerrainMountain }
	for _, r := range []rune("磊岩") { runeToTerrain[r] = TerrainStone }
	for _, r := range []rune("澤泥") { runeToTerrain[r] = TerrainSwamp }
	for _, r := range []rune("巜巛") { runeToTerrain[r] = TerrainRiver }
	for _, r := range []rune("沝淼") { runeToTerrain[r] = TerrainWater }
	for _, r := range []rune("旱焦") { runeToTerrain[r] = TerrainWasteland }
	for _, r := range []rune("路徑") { runeToTerrain[r] = TerrainRoad }
	for _, r := range []rune("炎焱燚") { runeToTerrain[r] = TerrainLava }
	for _, r := range []rune("凍冽") { runeToTerrain[r] = TerrainIce }
	for _, r := range []rune("畓畕") { runeToTerrain[r] = TerrainFarm }
	for _, r := range []rune("豀豁") { runeToTerrain[r] = TerrainValley }
	for _, r := range []rune("靄霙") { runeToTerrain[r] = TerrainFog }
	runeToTerrain['ㄑ'] = TerrainRiver
}

// TerrainFromRune 回傳該 rune 對應的地形；無對應則回傳草。
func TerrainFromRune(r rune) Terrain {
	if t, ok := runeToTerrain[r]; ok {
		return t
	}
	return TerrainGrass
}

// LoadChunk 從 basePath 讀取區塊 (cx, cy) 的 151×151 地形檔，檔名 {cx}_{cy}.txt。
// 格式：151 行、每行 151 個 UTF-8 字；第 L 行第 C 字 ＝ 區塊內 (x=C, y=L)。
// 若檔案不存在或行/字不足，不足格以草填滿。回傳 *Grid 與 error。
func LoadChunk(cx, cy int, basePath string) (*Grid, error) {
	path := filepath.Join(basePath, fmt.Sprintf("%d_%d.txt", cx, cy))
	f, err := os.Open(path)
	if err != nil {
		// 檔案不存在時回傳整塊草，方便未製圖的區塊仍可遊玩
		g := NewGrid(151, 151)
		return g, nil
	}
	defer f.Close()

	g := NewGrid(151, 151)
	sc := bufio.NewScanner(f)
	y := 0
	for sc.Scan() && y < 151 {
		line := sc.Text()
		x := 0
		for _, r := range line {
			if x >= 151 {
				break
			}
			if unicode.IsSpace(r) {
				continue
			}
			g.Cells[y*151+x] = TerrainFromRune(r)
			x++
		}
		y++
	}
	if err := sc.Err(); err != nil {
		return nil, err
	}
	return g, nil
}
