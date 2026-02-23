// Package world 地形顯示對照：大地圖只存地形類型一字，顯示時由 chars 亂序取字與對應顏色。對齊 map_terrain_world、第一版可做清單 §1.2。
package world

import (
	"math/rand"
)

// TerrainMeta 單一地形的顯示用資料：名稱、候選字、對應顏色（同索引一對一）。
type TerrainMeta struct {
	Name   string
	Chars  []string
	Colors []string
}

// terrainMetas 各地形類型之顯示對照表；鍵為大地圖所存之一字（地形類型）。
var terrainMetas = map[Terrain]TerrainMeta{
	"木": {Name: "樹木", Chars: []string{"木", "林", "森"}, Colors: []string{"#a0d080", "#6aa050", "#2a5020"}},
	"山": {Name: "山脈", Chars: []string{"山", "岳", "巒"}, Colors: []string{"#98af9d", "#4a7c59", "#1f3a30"}},
	"石": {Name: "碎石", Chars: []string{"石", "磊", "岩"}, Colors: []string{"#d3d3d3", "#8c8c8c", "#4f4f4f"}},
	"沼": {Name: "沼澤", Chars: []string{"沼", "澤", "泥"}, Colors: []string{"#507050", "#608050", "#608060"}},
	"川": {Name: "河流", Chars: []string{"ㄑ", "巜", "巛"}, Colors: []string{"#60a0d0", "#50b0e0", "#70c0f0"}},
	"水": {Name: "水域", Chars: []string{"水", "沝", "淼"}, Colors: []string{"#b0d8f0", "#a0c8e0", "#c0e8ff"}},
	"草": {Name: "草原", Chars: []string{"艸", "芔", "茻"}, Colors: []string{"#c2d6a4", "#8fb361", "#4d7338"}},
	"荒": {Name: "荒地", Chars: []string{"荒", "旱", "焦"}, Colors: []string{"#e3d5ca", "#d4a373", "#a98467"}},
	"道": {Name: "道路", Chars: []string{"道", "路", "徑"}, Colors: []string{"#c4b8a8", "#c4b8a8", "#c4b8a8"}}, // 與當前城市底色同，路、徑同理
	"巷": {Name: "巷", Chars: []string{"巷"}, Colors: []string{"#c4b8a8"}},                           // 房屋間距小成巷，與城市底色同
	"火": {Name: "岩漿", Chars: []string{"炎", "焱", "燚"}, Colors: []string{"#faa307", "#f48c06", "#dc2f02"}},
	"冰": {Name: "寒冰", Chars: []string{"冰", "凍", "冽"}, Colors: []string{"#e0f2f1", "#b2ebf2", "#80deea"}},
	"田": {Name: "農地", Chars: []string{"田", "畓", "畕"}, Colors: []string{"#9ef01a", "#70e000", "#38b000"}},
	"谷": {Name: "深谷", Chars: []string{"谷", "豀", "豁"}, Colors: []string{"#3d405b", "#22223b", "#0d1b2a"}},
	"霧": {Name: "迷霧", Chars: []string{"霧", "靄", "霙"}, Colors: []string{"#c8c4b8", "#b0aca0", "#989488"}},
	"牆": {Name: "牆", Chars: []string{"牆"}, Colors: []string{"#8c8c8c"}}, // 石灰色，對齊 map_terrain_world
	"門": {Name: "門", Chars: []string{"門"}, Colors: []string{"#8b7355"}},
	"關": {Name: "關", Chars: []string{"關"}, Colors: []string{"#6b5344"}},
	"地": {Name: "地", Chars: []string{"地"}, Colors: []string{"#c4b8a8"}}, // 房屋內部地板，單一字；預設與城市底色同系，可依城市覆寫
}

// Display 依地形類型與隨機源回傳「顯示用一字」與「對應顏色」；供前端繪製用。
// 參數：t 為格點地形類型（大地圖所存一字），r 為亂數來源（可 nil 則固定取第一個）。
// 回傳：char 為候選字之一，color 為對應 hex 色碼。若無對照則回傳地形字與預設灰。
func Display(t Terrain, r *rand.Rand) (char, color string) {
	meta, ok := terrainMetas[t]
	if !ok || len(meta.Chars) == 0 {
		return string(t), "#888888"
	}
	idx := 0
	if r != nil && len(meta.Chars) > 1 {
		idx = r.Intn(len(meta.Chars))
	}
	if idx >= len(meta.Colors) {
		idx = len(meta.Colors) - 1
	}
	return meta.Chars[idx], meta.Colors[idx]
}

// TerrainMetaByType 回傳指定地形類型之顯示用資料；若無則回傳零值，供前端一次取得整表用。
func TerrainMetaByType(t Terrain) (TerrainMeta, bool) {
	meta, ok := terrainMetas[t]
	return meta, ok
}
