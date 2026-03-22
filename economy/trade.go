// Package economy 負責交易與鎂流轉，對齊經濟彙整 §四 鎂產消閉環。
package economy

import "singularity_world/db"

// TransferMagnesium 將 fromID 的 amount 鎂轉給 toID；實作於 db（store）。
func TransferMagnesium(fromID, toID string, amount int) error {
	return db.TransferMagnesium(fromID, toID, amount)
}
