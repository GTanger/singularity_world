// Package event 負責事件日誌寫入與查詢。寫入/讀取 data/runtime/event_log.json（store）。
package event

import (
	"singularity_world/db"
	"singularity_world/store"
)

// Append 將一筆事件寫入 store 並持久化 event_log.json。
func Append(at int64, entityID, eventType, payload string) error {
	if store.Default == nil {
		return db.ErrNoStore
	}
	return store.Default.AppendEvent(at, entityID, eventType, payload)
}

// LastByEntity 回傳該實體在 at 之前最近一筆事件的 payload。
func LastByEntity(entityID, eventType string, at int64) (string, error) {
	if store.Default == nil {
		return "", db.ErrNoStore
	}
	return store.Default.LastByEntity(entityID, eventType, at), nil
}

// MarkObserved 觀測觸發時呼叫：寫入一筆 observed 事件並更新該實體的 last_observed_at。
func MarkObserved(entityID, observerID string, at int64) error {
	if err := Append(at, entityID, TypeObserved, observerID); err != nil {
		return err
	}
	return db.UpdateLastObserved(entityID, at)
}

// EventRow 為事件日誌一筆，供回推時依時間序套用。
type EventRow struct {
	At      int64
	Type    string
	Payload string
}

// EventsInRange 回傳該實體在 [fromAt, toAt] 區間內的事件，依 at 升序。
func EventsInRange(entityID string, fromAt, toAt int64) ([]EventRow, error) {
	if store.Default == nil {
		return nil, db.ErrNoStore
	}
	entries := store.Default.EventsInRange(entityID, fromAt, toAt)
	out := make([]EventRow, len(entries))
	for i, e := range entries {
		out[i] = EventRow{At: e.At, Type: e.EventType, Payload: e.Payload}
	}
	return out, nil
}
