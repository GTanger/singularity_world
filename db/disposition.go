// Package db 心境值（disposition）操作；階段 E 會補上 store.Entity.Disposition 與實際加減。
package db

import (
	"database/sql"

	"singularity_world/store"
)

// disposition 變動量常數
const (
	DispHired      = +20 // 找到工作
	DispFired      = -30 // 被解僱
	DispBroke      = -20 // 鎂歸零
	DispBegSuccess = +3  // 乞討成功
	DispGather     = +5  // 採集成功
	DispTalked     = +5  // 與人交談
	DispDaily      = -2  // 每日自然衰減（回歸中性）
	DispSubdued    = -15 // 被制伏
)

// AdjustDisposition 調整心境值，clamp [-100, +100]。
func AdjustDisposition(database *sql.DB, entityID string, delta int) {
	if store.Default == nil {
		return
	}
	_ = store.Default.UpdateEntity(entityID, func(e *store.Entity) {
		e.Disposition += delta
		if e.Disposition > 100 {
			e.Disposition = 100
		}
		if e.Disposition < -100 {
			e.Disposition = -100
		}
	})
	_ = database
}

// GetDisposition 取得心境值；無實體或無 store 時回傳 0。
func GetDisposition(database *sql.DB, entityID string) int {
	c, _ := GetEntity(database, entityID)
	if c == nil {
		return 0
	}
	return c.Disposition
}
