// Package db 負責 SQLite 連線與 schema 載入，對齊第一版可做清單 §1.8.3。
// 不包含 init()，由呼叫方傳入路徑並呼叫 OpenDB。
package db

import (
	"database/sql"
	_ "embed"
	"fmt"
	"strings"

	_ "modernc.org/sqlite"
)

//go:embed schema.sql
var schemaSQL string

// OpenDB 開啟 SQLite 連線並執行內嵌的 schema 建立表結構；若表已存在則不覆寫。
// 參數：path 為 SQLite 檔案路徑（例 data/world.db）。
// 回傳：*sql.DB 與 error；成功時 schema 已就緒。
// 副作用：可能建立檔案與 entities、event_log 表。
func OpenDB(path string) (*sql.DB, error) {
	db, err := sql.Open("sqlite", path)
	if err != nil {
		return nil, fmt.Errorf("sql open: %w", err)
	}
	if err := db.Ping(); err != nil {
		_ = db.Close()
		return nil, fmt.Errorf("sql ping: %w", err)
	}
	// 遇鎖定時等待最多約 5 秒，避免 SQLITE_BUSY 導致啟動失敗（多進程或 seed 多筆寫入時）
	if _, err := db.Exec("PRAGMA busy_timeout = 5000"); err != nil {
		_ = db.Close()
		return nil, fmt.Errorf("pragma busy_timeout: %w", err)
	}
	if _, err := db.Exec(schemaSQL); err != nil {
		_ = db.Close()
		return nil, fmt.Errorf("exec schema: %w", err)
	}
	// 既有 DB 補上 gender 欄位（新 DB 已含於 schema）
	if _, err := db.Exec("ALTER TABLE entities ADD COLUMN gender TEXT"); err != nil && !strings.Contains(err.Error(), "duplicate column name") {
		_ = db.Close()
		return nil, fmt.Errorf("migrate entities.gender: %w", err)
	}
	if err := SeedRooms(db); err != nil {
		_ = db.Close()
		return nil, fmt.Errorf("seed rooms: %w", err)
	}
	return db, nil
}
