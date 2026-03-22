// Package game 負責觀測觸發與延遲坍縮回推。本檔為觀測時寫入事件日誌與回推介面，對齊決策 004。
package game

import (
	"strconv"
	"time"

	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/event"
)

// Observer 在觀測發生時由呼叫方（如視野查詢、房間視野）呼叫，負責寫入事件日誌與更新 last_observed_at。
// observerID 為觀測者（如玩家 ID）；格點視野無觀測者時可傳空字串。
type Observer interface {
	OnObserve(entityID, observerID string, at int64)
}

// Observed 為 Observer 的具體實作；觀測時寫入 event_log 並更新 entities.last_observed_at（經 store）。
type Observed struct{}

// OnObserve 實作 Observer：寫入 observed 事件並更新該實體的 last_observed_at。
func (o *Observed) OnObserve(entityID, observerID string, at int64) {
	_ = event.MarkObserved(entityID, observerID, at)
}

// ObserveRoom 進入房間觸發觀測：對該房內所有 NPC 寫入 observed 事件並更新 last_observed_at（observerID 為觀測者，如玩家 ID）。
func ObserveRoom(roomID, observerID string, at int64) {
	entities, err := db.GetEntitiesInRoom(roomID, -1)
	if err != nil {
		return
	}
	for _, e := range entities {
		if e.Kind == "npc" {
			_ = event.MarkObserved(e.ID, observerID, at)
		}
	}
}

// Collapse 從 store 與事件日誌回推該實體在 asOf 時點的狀態；依 event.EventsInRange 重放 vit／move 等事件。
func Collapse(entityID string, asOf int64) (*entity.Character, string, error) {
	c, err := db.GetEntity(entityID)
	if err != nil || c == nil {
		return nil, "", err
	}
	roomID, _ := db.GetEntityRoom(entityID)
	now := NowUnix()
	if asOf >= now {
		return c, roomID, nil
	}
	rows, err := event.EventsInRange(entityID, asOf, now)
	if err != nil {
		return c, roomID, nil
	}
	for _, r := range rows {
		switch r.Type {
		case event.TypeVit:
			if v, e := strconv.Atoi(r.Payload); e == nil && v >= 0 {
				c.Vit = v
			}
		case event.TypeMove:
			if r.Payload != "" {
				roomID = r.Payload
			}
		}
	}
	return c, roomID, nil
}

// NowUnix 回傳當前時間的 Unix 秒數，供觀測／坍縮時戳使用；可改為遊戲 tick 計數依專案約定。
func NowUnix() int64 {
	return time.Now().Unix()
}
