package db

import "encoding/json"

// PickNPCTradeOffer 從 NPC 背包 JSON 取第一個數量 > 0 的物品作為本輪賣品。
func PickNPCTradeOffer(inventoryJSON string) (itemID string, ok bool) {
	if inventoryJSON == "" || inventoryJSON == "[]" {
		return "", false
	}
	var entries []InventoryEntry
	if err := json.Unmarshal([]byte(inventoryJSON), &entries); err != nil {
		return "", false
	}
	for _, e := range entries {
		if e.ItemID != "" && e.Qty > 0 {
			return e.ItemID, true
		}
	}
	return "", false
}

// DefaultTradeAskMg 第一版預設喊價（可依物品擴充）。
func DefaultTradeAskMg(itemID string) int {
	switch itemID {
	case "wild_herb":
		return 5
	default:
		return 8
	}
}

// TradeFloorFromAsk 底價＝喊價七成（至少 1），議價達此即可成交。
func TradeFloorFromAsk(ask int) int {
	if ask <= 0 {
		return 1
	}
	f := ask * 7 / 10
	if f < 1 {
		return 1
	}
	return f
}
