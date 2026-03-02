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
	// 既有 DB 補上 soul_seed 欄位（創角必存，規格 §2.0）
	if _, err := db.Exec("ALTER TABLE entities ADD COLUMN soul_seed INTEGER"); err != nil && !strings.Contains(err.Error(), "duplicate column name") {
		_ = db.Close()
		return nil, fmt.Errorf("migrate entities.soul_seed: %w", err)
	}
	// 命途稱謂，空則前端顯示「無名之輩」（邏輯閉環 §4.4）
	if _, err := db.Exec("ALTER TABLE entities ADD COLUMN display_title TEXT"); err != nil && !strings.Contains(err.Error(), "duplicate column name") {
		_ = db.Close()
		return nil, fmt.Errorf("migrate entities.display_title: %w", err)
	}
	// 星盤已貫通節點 ID 清單，預設僅 N000（邏輯閉環 §4.2）
	if _, err := db.Exec("ALTER TABLE entities ADD COLUMN activated_nodes TEXT DEFAULT '[\"N000\"]'"); err != nil && !strings.Contains(err.Error(), "duplicate column name") {
		_ = db.Close()
		return nil, fmt.Errorf("migrate entities.activated_nodes: %w", err)
	}
	// 裝備槽位 JSON（裝備分頁規格 §一）
	if _, err := db.Exec("ALTER TABLE entities ADD COLUMN equipment_slots TEXT"); err != nil && !strings.Contains(err.Error(), "duplicate column name") {
		_ = db.Close()
		return nil, fmt.Errorf("migrate entities.equipment_slots: %w", err)
	}
	// 背包物品 JSON 陣列（背包規格 §六）
	if _, err := db.Exec("ALTER TABLE entities ADD COLUMN inventory TEXT DEFAULT '[]'"); err != nil && !strings.Contains(err.Error(), "duplicate column name") {
		_ = db.Close()
		return nil, fmt.Errorf("migrate entities.inventory: %w", err)
	}
	// items 表擴充：weight, item_type, stackable, denomination（背包規格 §四、§6.1）
	for _, mig := range []string{
		"ALTER TABLE items ADD COLUMN item_type TEXT NOT NULL DEFAULT 'equipment'",
		"ALTER TABLE items ADD COLUMN weight REAL NOT NULL DEFAULT 0",
		"ALTER TABLE items ADD COLUMN stackable INTEGER NOT NULL DEFAULT 0",
		"ALTER TABLE items ADD COLUMN denomination INTEGER NOT NULL DEFAULT 0",
		"ALTER TABLE items ADD COLUMN description TEXT NOT NULL DEFAULT ''",
	} {
		if _, err := db.Exec(mig); err != nil && !strings.Contains(err.Error(), "duplicate column name") {
			_ = db.Close()
			return nil, fmt.Errorf("migrate items: %w", err)
		}
	}
	if err := SeedRooms(db); err != nil {
		_ = db.Close()
		return nil, fmt.Errorf("seed rooms: %w", err)
	}
	if err := SeedItems(db); err != nil {
		_ = db.Close()
		return nil, fmt.Errorf("seed items: %w", err)
	}
	return db, nil
}
