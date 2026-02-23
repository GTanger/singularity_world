// Package economy 負責經濟引擎 goroutine 與交易、鎂流轉。本檔為經濟 tick，對齊第一版可做清單 §1.1.6。
package economy

import (
	"time"
)

// Run 在獨立 goroutine 中以固定間隔執行 onTick（例：產出事件、更新價格）；第一版 onTick 可為空函數。
// 參數：interval 為經濟 tick 間隔；onTick 為每 tick 執行的函數。
// 回傳：無。副作用：啟動一 goroutine，每 interval 呼叫 onTick()。
func Run(interval time.Duration, onTick func()) {
	go func() {
		ticker := time.NewTicker(interval)
		defer ticker.Stop()
		for range ticker.C {
			onTick()
		}
	}()
}
