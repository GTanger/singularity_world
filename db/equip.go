// Package db 裝備相關：初始穿搭、裸奔判定、物品種子。對齊裝備分頁規格 §五。
package db

import (
	"encoding/json"

	"singularity_world/store"
)

// StarterEquipment 依性別回傳初始裝備 JSON（裝備分頁規格 §5.1）。
func StarterEquipment(gender string) string {
	if gender == "F" {
		return `{"body":"starter_body_f","legs":"starter_legs_f","feet":"starter_feet_f"}`
	}
	return `{"body":"starter_body_m","legs":"starter_legs_m","feet":"starter_feet_m"}`
}

// IsNaked 檢查 equipment_slots JSON，body 或 legs 任一為空即為「衣不蔽體」（裝備分頁規格 §5.2）。
func IsNaked(equipmentSlots string) bool {
	if equipmentSlots == "" {
		return true
	}
	var slots map[string]string
	if err := json.Unmarshal([]byte(equipmentSlots), &slots); err != nil {
		return true
	}
	return slots["body"] == "" || slots["legs"] == ""
}

// GetItemNames 依 equipment_slots JSON 查物品名稱。
func GetItemNames(equipmentSlots string) (map[string]string, error) {
	result := make(map[string]string)
	if equipmentSlots == "" {
		return result, nil
	}
	var slots map[string]string
	if err := json.Unmarshal([]byte(equipmentSlots), &slots); err != nil {
		return result, nil
	}
	for slot, itemID := range slots {
		if itemID == "" {
			continue
		}
		if store.Default != nil {
			if it := store.Default.GetItem(itemID); it != nil {
				result[slot] = it.Name
			}
		}
	}
	return result, nil
}

// GetItemDescs 依 equipment_slots JSON 查物品描述。
func GetItemDescs(equipmentSlots string) map[string]string {
	result := make(map[string]string)
	if equipmentSlots == "" {
		return result
	}
	var slots map[string]string
	if err := json.Unmarshal([]byte(equipmentSlots), &slots); err != nil {
		return result
	}
	for slot, itemID := range slots {
		if itemID == "" {
			continue
		}
		if store.Default != nil {
			if it := store.Default.GetItem(itemID); it != nil {
				result[slot] = it.Description
			}
		}
	}
	return result
}

// SeedItems 初始化物品種子；現由 data/items.json 於 store.Init 載入，此函式為 no-op。
func SeedItems() error {
	return nil
}
