// Package db 背包相關：讀取背包、計算負重。對齊背包規格 §六。
package db

import (
	"encoding/json"

	"singularity_world/store"
)

// InventoryEntry 背包中一筆物品（item_id + 數量）。
type InventoryEntry struct {
	ItemID string `json:"item_id"`
	Qty    int    `json:"qty"`
}

// InventoryDisplayItem 前端顯示用：名稱、數量、單件重量、小計重量、描述、槽位。
type InventoryDisplayItem struct {
	ItemID      string  `json:"item_id"`
	Name        string  `json:"name"`
	ItemType    string  `json:"item_type"`
	Qty         int     `json:"qty"`
	Weight      float64 `json:"weight"`
	SubTotal    float64 `json:"sub_total"`
	Description string  `json:"description"`
	Slot        string  `json:"slot"`
}

// InventoryResult 背包查詢結果：物品清單 + 負重資訊。
type InventoryResult struct {
	Items         []InventoryDisplayItem `json:"items"`
	CurrentWeight float64                `json:"current_weight"`
	MaxWeight     float64                `json:"max_weight"`
}

const weightPerVit = 10.0

// GetInventory 從 entities.inventory JSON 解析物品清單，並查 items 補全名稱與重量；vit 用於計算最大負重。
func GetInventory(inventoryJSON string, vit int) InventoryResult {
	var entries []InventoryEntry
	if inventoryJSON == "" || inventoryJSON == "[]" {
		return InventoryResult{
			Items:         []InventoryDisplayItem{},
			CurrentWeight: 0,
			MaxWeight:     float64(vit) * weightPerVit,
		}
	}
	if err := json.Unmarshal([]byte(inventoryJSON), &entries); err != nil {
		return InventoryResult{
			Items:         []InventoryDisplayItem{},
			CurrentWeight: 0,
			MaxWeight:     float64(vit) * weightPerVit,
		}
	}

	var items []InventoryDisplayItem
	var totalWeight float64
	for _, e := range entries {
		if e.Qty <= 0 {
			continue
		}
		var name, itemType, description, slot string
		var weight float64
		if store.Default != nil {
			if it := store.Default.GetItem(e.ItemID); it != nil {
				name, itemType, description, slot = it.Name, it.ItemType, it.Description, it.Slot
				weight = it.Weight
			} else {
				name, itemType = e.ItemID, "misc"
			}
		} else {
			name, itemType = e.ItemID, "misc"
		}
		sub := weight * float64(e.Qty)
		items = append(items, InventoryDisplayItem{
			ItemID:      e.ItemID,
			Name:        name,
			ItemType:    itemType,
			Qty:         e.Qty,
			Weight:      weight,
			SubTotal:    sub,
			Description: description,
			Slot:        slot,
		})
		totalWeight += sub
	}
	if items == nil {
		items = []InventoryDisplayItem{}
	}

	return InventoryResult{
		Items:         items,
		CurrentWeight: totalWeight,
		MaxWeight:     float64(vit) * weightPerVit,
	}
}

// GetItemInfo 查詢單一物品的定義資訊。
func GetItemInfo(itemID string) (name, itemType, slot, description string, weight float64, err error) {
	if store.Default == nil {
		return "", "", "", "", 0, ErrNoStore
	}
	it := store.Default.GetItem(itemID)
	if it == nil {
		return "", "", "", "", 0, ErrItemNotFound
	}
	return it.Name, it.ItemType, it.Slot, it.Description, it.Weight, nil
}

// AddToInventory 將物品加入背包。
func AddToInventory(entityID, itemID string, qty int) error {
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.UpdateEntity(entityID, func(e *store.Entity) {
		raw := e.Inventory
		if raw == "" {
			raw = "[]"
		}
		var entries []InventoryEntry
		_ = json.Unmarshal([]byte(raw), &entries)
		found := false
		for i := range entries {
			if entries[i].ItemID == itemID {
				entries[i].Qty += qty
				found = true
				break
			}
		}
		if !found {
			entries = append(entries, InventoryEntry{ItemID: itemID, Qty: qty})
		}
		b, _ := json.Marshal(entries)
		e.Inventory = string(b)
	})
}

// RemoveFromInventory 從背包移除指定數量物品。
func RemoveFromInventory(entityID, itemID string, qty int) error {
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.UpdateEntity(entityID, func(e *store.Entity) {
		raw := e.Inventory
		if raw == "" {
			raw = "[]"
		}
		var entries []InventoryEntry
		_ = json.Unmarshal([]byte(raw), &entries)
		var updated []InventoryEntry
		for _, ent := range entries {
			if ent.ItemID == itemID {
				ent.Qty -= qty
				if ent.Qty > 0 {
					updated = append(updated, ent)
				}
			} else {
				updated = append(updated, ent)
			}
		}
		if updated == nil {
			updated = []InventoryEntry{}
		}
		b, _ := json.Marshal(updated)
		e.Inventory = string(b)
	})
}

// UpdateEquipmentSlot 設定單一裝備槽位的 item_id。
func UpdateEquipmentSlot(entityID, slot, itemID string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.UpdateEntity(entityID, func(e *store.Entity) {
		slots := make(map[string]string)
		if e.EquipmentSlots != "" {
			_ = json.Unmarshal([]byte(e.EquipmentSlots), &slots)
		}
		slots[slot] = itemID
		b, _ := json.Marshal(slots)
		e.EquipmentSlots = string(b)
	})
}

// ClearEquipmentSlot 清空單一裝備槽位。
func ClearEquipmentSlot(entityID, slot string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.UpdateEntity(entityID, func(e *store.Entity) {
		slots := make(map[string]string)
		if e.EquipmentSlots != "" {
			_ = json.Unmarshal([]byte(e.EquipmentSlots), &slots)
		}
		delete(slots, slot)
		b, _ := json.Marshal(slots)
		e.EquipmentSlots = string(b)
	})
}

// InventoryWeight 計算背包目前總重量。
func InventoryWeight(inventoryJSON string) float64 {
	var entries []InventoryEntry
	if inventoryJSON == "" || inventoryJSON == "[]" {
		return 0
	}
	_ = json.Unmarshal([]byte(inventoryJSON), &entries)
	var total float64
	for _, e := range entries {
		if e.Qty <= 0 {
			continue
		}
		var weight float64
		if store.Default != nil {
			if it := store.Default.GetItem(e.ItemID); it != nil {
				weight = it.Weight
			}
		}
		total += weight * float64(e.Qty)
	}
	return total
}
