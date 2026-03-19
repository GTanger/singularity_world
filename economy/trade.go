// Package economy 負責交易與鎂流轉，對齊經濟彙整 §四 鎂產消閉環。
package economy

import (
	"database/sql"

	"singularity_world/db"
)

// TransferMagnesium 將 fromID 的 amount 鎂轉給 toID；實作於 db（store／SQL 雙路徑原子扣加）。
func TransferMagnesium(database *sql.DB, fromID, toID string, amount int) error {
	return db.TransferMagnesium(database, fromID, toID, amount)
}
