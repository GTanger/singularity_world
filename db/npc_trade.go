// Package db NPC 行腳／兜售：與討論 002「不必 assignment 亦可賣貨換鎂」對齊。
package db

import (
	"encoding/json"
	"fmt"
	"math/rand"
)

// NpcStreetSellOne 在街頭賣出背包中第一筆數量≥1 的物品 1 件，換取少量鎂；供 IntentTrade 到達效果。
func NpcStreetSellOne(entityID, roomID string) bool {
	c, err := GetEntity(entityID)
	if err != nil || c == nil {
		return false
	}
	if c.Inventory == "" || c.Inventory == "[]" {
		return false
	}
	var entries []InventoryEntry
	if err := json.Unmarshal([]byte(c.Inventory), &entries); err != nil {
		return false
	}
	for _, e := range entries {
		if e.Qty <= 0 || e.ItemID == "" {
			continue
		}
		if err := RemoveFromInventory(entityID, e.ItemID, 1); err != nil {
			continue
		}
		mag := 4 + rand.Intn(10)
		_ = AddMagnesium(entityID, mag)
		LogNPCEvent(entityID, EvtTrade, fmt.Sprintf("在%s兜售賣出%s，得%d鎂", roomID, e.ItemID, mag))
		AdjustDisposition(entityID, DispTrade)
		return true
	}
	return false
}
