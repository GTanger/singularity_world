// Package game 每 tick 推進移動中實體的位置，對齊 1.3.3 單擊走雙擊跑、抵達時間。
package game

import (
	"database/sql"

	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/event"
)

// MovedResult 單一實體本 tick 移動後的座標，供 server 廣播。
type MovedResult struct {
	ID string
	X  int
	Y  int
}

// AdvanceMovement 對所有 move_state=moving 的實體依 walk/run 步進一或二格，寫回 DB 並回傳有變動的 (id,x,y)。
// 若中途撞牆則停止並寫入 blocked 事件；抵達目標則寫入 arrived 事件。
func AdvanceMovement(database *sql.DB, mapsPath string, now int64) ([]MovedResult, error) {
	list, err := db.GetMovingEntities(database)
	if err != nil || len(list) == 0 {
		return nil, err
	}
	var out []MovedResult
	for _, e := range list {
		if e.TargetX == nil || e.TargetY == nil {
			continue
		}
		tx, ty := *e.TargetX, *e.TargetY
		if e.X == tx && e.Y == ty {
			_ = db.UpdatePosition(database, e.ID, tx, ty)
			_ = event.Append(database, now, e.ID, event.TypeArrived, "")
			out = append(out, MovedResult{ID: e.ID, X: tx, Y: ty})
			continue
		}
		steps := 1
		if e.WalkOrRun == "run" {
			steps = 2
		}
		nx, ny := e.X, e.Y
		for s := 0; s < steps; s++ {
			view, err := GetChunkAndEntitiesInView(database, nx, ny, mapsPath)
			if err != nil {
				break
			}
			dx := sign(tx - nx)
			dy := sign(ty - ny)
			if dx == 0 && dy == 0 {
				break
			}
			nextX, nextY := nx+dx, ny+dy
			if !view.CanMoveToWorld(nextX, nextY) {
				_ = event.Append(database, now, e.ID, event.TypeBlocked, "")
				_ = db.UpdatePosition(database, e.ID, nx, ny)
				break
			}
			nx, ny = nextX, nextY
			if nx == tx && ny == ty {
				break
			}
		}
		if nx != e.X || ny != e.Y {
			_ = db.UpdatePositionOnly(database, e.ID, nx, ny)
			out = append(out, MovedResult{ID: e.ID, X: nx, Y: ny})
		}
		if nx == tx && ny == ty {
			_ = db.UpdatePosition(database, e.ID, tx, ty)
			_ = event.Append(database, now, e.ID, event.TypeArrived, "")
			if other, _ := EntityAt(database, tx, ty, e.ID); other != nil {
				_ = event.Append(database, now, e.ID, event.TypeContact, other.ID)
			}
		}
	}
	return out, nil
}

func sign(n int) int {
	if n < 0 {
		return -1
	}
	if n > 0 {
		return 1
	}
	return 0
}

// EntityAt 回傳在 (x,y) 的實體（不含 id 者）；用於接觸判定等。
func EntityAt(database *sql.DB, x, y int, excludeID string) (*entity.Character, error) {
	list, err := db.GetEntitiesInBox(database, x, x, y, y, "")
	if err != nil {
		return nil, err
	}
	for _, e := range list {
		if e.ID != excludeID {
			return e, nil
		}
	}
	return nil, nil
}
