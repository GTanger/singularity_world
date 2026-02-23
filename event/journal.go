// Package event 負責事件日誌寫入與查詢。本檔為寫入與依 entity_id/時間查詢，對齊決策 004 延遲坍縮回推。
package event

import (
	"database/sql"

	"singularity_world/db"
)

// Append 將一筆事件寫入 event_log 表。
// 參數：db 為 *sql.DB；at 為時間戳；entityID、eventType、payload 為事件內容。
// 回傳：error。副作用：INSERT 一筆。
func Append(db *sql.DB, at int64, entityID, eventType, payload string) error {
	_, err := db.Exec(
		"INSERT INTO event_log (at, entity_id, event_type, payload) VALUES (?, ?, ?, ?)",
		at, entityID, eventType, payload,
	)
	return err
}

// LastByEntity 回傳該實體在 at 之前最近一筆事件的 payload；若無則回傳空字串與 nil error。
func LastByEntity(database *sql.DB, entityID, eventType string, at int64) (string, error) {
	var payload string
	err := database.QueryRow(
		"SELECT payload FROM event_log WHERE entity_id = ? AND event_type = ? AND at <= ? ORDER BY at DESC LIMIT 1",
		entityID, eventType, at,
	).Scan(&payload)
	if err == sql.ErrNoRows {
		return "", nil
	}
	if err != nil {
		return "", err
	}
	return payload, nil
}

// MarkObserved 觀測觸發時呼叫：寫入一筆 observed 事件並更新該實體的 last_observed_at。
// 參數：database 為 *sql.DB；entityID 為被觀測的 NPC/實體 id；observerID 為觀測者 id（可空）；at 為時間戳。
// 回傳：error。副作用：INSERT event_log 一筆、UPDATE entities.last_observed_at。
func MarkObserved(database *sql.DB, entityID, observerID string, at int64) error {
	if err := Append(database, at, entityID, TypeObserved, observerID); err != nil {
		return err
	}
	return db.UpdateLastObserved(database, entityID, at)
}

// EventRow 為事件日誌一筆，供回推時依時間序套用。
type EventRow struct {
	At      int64
	Type    string
	Payload string
}

// EventsInRange 回傳該實體在 [fromAt, toAt] 區間內的事件，依 at 升序；供坍縮回推時重放。
func EventsInRange(database *sql.DB, entityID string, fromAt, toAt int64) ([]EventRow, error) {
	rows, err := database.Query(
		"SELECT at, event_type, payload FROM event_log WHERE entity_id = ? AND at >= ? AND at <= ? ORDER BY at ASC",
		entityID, fromAt, toAt,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var out []EventRow
	for rows.Next() {
		var r EventRow
		if err := rows.Scan(&r.At, &r.Type, &r.Payload); err != nil {
			return nil, err
		}
		out = append(out, r)
	}
	return out, rows.Err()
}
