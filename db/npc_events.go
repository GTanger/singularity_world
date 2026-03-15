// Package db NPC 個人事件日誌 API（階段 D 完整；階段 A 先使用 LogNPCEvent）。
package db

import (
	"time"

	"singularity_world/store"
)

// NPC 事件類型常數
const (
	EvtHired   = "hired"    // 獲得指派
	EvtFired   = "fired"    // 被解僱
	EvtBeg     = "beg"      // 乞討
	EvtGather  = "gather"   // 採集
	EvtTrade   = "trade"    // 交易
	EvtSeekJob = "seek_job" // 求職
	EvtTalk    = "talk"     // 與玩家交談
	EvtMoved   = "moved"    // 到達新房間（僅重要場景）
	EvtBroke   = "broke"    // 鎂歸零
)

// LogNPCEvent 記錄一筆 NPC 事件。
func LogNPCEvent(entityID, eventType, payload string) {
	if store.Default == nil {
		return
	}
	_ = store.Default.AppendEvent(time.Now().Unix(), entityID, eventType, payload)
}

// GetRecentEvents 取得某 NPC 最近 n 筆事件（供 Talk／背版引用）。由 store.RecentByEntity 提供。
func GetRecentEvents(entityID string, n int) []store.EventEntry {
	if store.Default == nil {
		return nil
	}
	return store.Default.RecentByEntity(entityID, n)
}
