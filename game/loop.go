// Package game 負責遊戲主迴圈、視野區域與觀測坍縮。本檔為 tick 主迴圈，對齊第一版可做清單 §1.1.2。
package game

import (
	"time"
)

// Loop 以固定間隔 tick 呼叫 onTick；通常由 main 啟動為一 goroutine，傳入需每 tick 更新的邏輯。
// 參數：interval 為 tick 間隔；onTick 為每 tick 執行的函數。
// 回傳：無。副作用：每 interval 呼叫 onTick()，直到程式結束。
func Loop(interval time.Duration, onTick func()) {
	ticker := time.NewTicker(interval)
	defer ticker.Stop()
	for range ticker.C {
		onTick()
	}
}
