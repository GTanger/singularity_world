// Package db 裝備相關：初始穿搭、裸奔判定、物品種子。對齊裝備分頁規格 §五。
package db

import (
	"database/sql"
	"encoding/json"
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

// GetItemNames 依 equipment_slots JSON 查 items 表，回傳 slot→item_name 對照。
func GetItemNames(db *sql.DB, equipmentSlots string) (map[string]string, error) {
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
		var name string
		err := db.QueryRow("SELECT name FROM items WHERE id = ?", itemID).Scan(&name)
		if err == nil {
			result[slot] = name
		}
	}
	return result, nil
}

// SeedItems 初始化 items 表的種子資料（6 件初始裝備）。若已有資料則跳過。
func SeedItems(db *sql.DB) error {
	var n int
	if err := db.QueryRow("SELECT COUNT(*) FROM items").Scan(&n); err != nil || n > 0 {
		return err
	}
	items := []struct{ id, name, slot string }{
		{"starter_body_m", "粗布短褂", "body"},
		{"starter_legs_m", "粗布長褲", "legs"},
		{"starter_feet_m", "草鞋", "feet"},
		{"starter_body_f", "素布衣裙", "body"},
		{"starter_legs_f", "布裳", "legs"},
		{"starter_feet_f", "布鞋", "feet"},
	}
	for _, it := range items {
		if _, err := db.Exec("INSERT INTO items (id, name, slot) VALUES (?, ?, ?)", it.id, it.name, it.slot); err != nil {
			return err
		}
	}
	return nil
}
