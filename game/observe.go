// Package game 負責觀測觸發與延遲坍縮回推。本檔為觀測時寫入事件日誌與回推介面，對齊決策 004。
package game

import (
	"database/sql"
	"time"

	"singularity_world/entity"
	"singularity_world/event"
	"singularity_world/db"
)

// Observer 在觀測發生時由呼叫方（如視野查詢）呼叫，負責寫入事件日誌與更新 last_observed_at。
type Observer interface {
	OnObserve(entityID string, at int64)
}

// Observed 為 Observer 的具體實作，持有一份 *sql.DB，觀測時寫入 event_log 並更新 entities.last_observed_at。
type Observed struct {
	DB *sql.DB
}

// OnObserve 實作 Observer：寫入 observed 事件並更新該實體的 last_observed_at。
func (o *Observed) OnObserve(entityID string, at int64) {
	_ = event.MarkObserved(o.DB, entityID, "", at)
}

// Collapse 從 entities 表與事件日誌回推 NPC 在 asOf 時點的狀態；第一版最小實作為直接讀取 entity 列。
// 參數：database 為 *sql.DB；entityID 為 NPC id；asOf 為回推時點（unix 或 tick 時間戳）。
// 回傳：*entity.Character 與 error；若無該實體則 (nil, nil)。後續可依 event.EventsInRange 重放事件以精算狀態。
func Collapse(database *sql.DB, entityID string, asOf int64) (*entity.Character, error) {
	c, err := db.GetEntity(database, entityID)
	if err != nil || c == nil {
		return nil, err
	}
	_ = asOf
	return c, nil
}

// NowUnix 回傳當前時間的 Unix 秒數，供觀測／坍縮時戳使用；可改為遊戲 tick 計數依專案約定。
func NowUnix() int64 {
	return time.Now().Unix()
}
