// Package event 負責事件日誌的寫入與查詢、事件類型定義。本檔為事件類型常數，對齊決策 004 事件日誌。
package event

const (
	TypeObserved  = "observed"  // 觀測觸發
	TypeMove      = "move"      // 移動
	TypeArrived   = "arrived"   // 抵達目標（spec_移動與地圖 §3.2）
	TypeBlocked   = "blocked"   // 撞牆／阻擋
	TypeContact   = "contact"   // 與實體接觸（payload 為對方 entity_id）
	TypeCombat    = "combat"    // 戰鬥
	TypeTrade     = "trade"     // 交易
)
