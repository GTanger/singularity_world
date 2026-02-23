// Package game 視野內即時模擬：僅對視野內 NPC 跑邏輯，其餘僅存狀態。本檔對齊第一版可做清單 §1.1.5。
package game

import (
	"database/sql"

	"singularity_world/db"
	"singularity_world/entity"
)

// Pos 表示一個觀測者（玩家）的格點座標，用於計算視野內實體。
type Pos struct {
	X, Y int
}

// InViewEntityIDs 回傳在任一觀測者視野內的實體 ID 列表（去重）。
func InViewEntityIDs(observers []Pos, entities []*entity.Character) []string {
	seen := make(map[string]bool)
	for _, e := range entities {
		for _, o := range observers {
			if InView(o.X, o.Y, e.X, e.Y) {
				seen[e.ID] = true
				break
			}
		}
	}
	ids := make([]string, 0, len(seen))
	for id := range seen {
		ids = append(ids, id)
	}
	return ids
}

// SimulateOneTick 對單一 NPC 執行一 tick 的即時模擬；第一版為 no-op，後續可接移動／AI。
// 參數：database 為 *sql.DB；entityID 為 NPC id。
// 回傳：error。副作用：視後續實作（目前無）。
func SimulateOneTick(database *sql.DB, entityID string) error {
	_ = database
	_ = entityID
	return nil
}

// RunViewSimulation 依觀測者位置只對「視野內 NPC」跑一 tick 模擬，其餘不跑邏輯。
// 參數：database 為 *sql.DB；getObserverPositions 回傳目前所有觀測者座標（例：連線玩家位置）；obs 為觀測時寫入日誌用，可 nil。
// 副作用：對每個視野內 NPC 呼叫 OnObserve（若 obs 非 nil）並 SimulateOneTick。
func RunViewSimulation(database *sql.DB, getObserverPositions func() []Pos, obs Observer) {
	observers := getObserverPositions()
	if len(observers) == 0 {
		return
	}
	xMin, xMax := observers[0].X, observers[0].X
	yMin, yMax := observers[0].Y, observers[0].Y
	for _, p := range observers[1:] {
		if p.X < xMin {
			xMin = p.X
		}
		if p.X > xMax {
			xMax = p.X
		}
		if p.Y < yMin {
			yMin = p.Y
		}
		if p.Y > yMax {
			yMax = p.Y
		}
	}
	xMin -= ViewRadius
	xMax += ViewRadius
	yMin -= ViewRadius
	yMax += ViewRadius

	npcs, err := db.GetEntitiesInBox(database, xMin, xMax, yMin, yMax, "npc")
	if err != nil || len(npcs) == 0 {
		return
	}

	inViewIDs := InViewEntityIDs(observers, npcs)
	now := NowUnix()
	for _, id := range inViewIDs {
		if obs != nil {
			obs.OnObserve(id, now)
		}
		_ = SimulateOneTick(database, id)
	}
}
