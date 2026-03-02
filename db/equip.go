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

// GetItemDescs 依 equipment_slots JSON 查 items 表，回傳 slot→description 對照。
func GetItemDescs(db *sql.DB, equipmentSlots string) map[string]string {
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
		var desc string
		if err := db.QueryRow("SELECT description FROM items WHERE id = ?", itemID).Scan(&desc); err == nil {
			result[slot] = desc
		}
	}
	return result
}

// SeedItems 初始化 items 表的種子資料（初始裝備＋鎂面額）。使用 REPLACE 以更新既有資料。
func SeedItems(db *sql.DB) error {
	type seedItem struct {
		id, name, slot, itemType, description string
		weight                                float64
		stackable, denomination               int
	}
	all := []seedItem{
		{"starter_body_m", "粗布短褂", "body", "equipment", "一件簡陋的短褂，聊勝於無。", 0.8, 0, 0},
		{"starter_legs_m", "粗布長褲", "legs", "equipment", "粗糙的麻布長褲，勉強遮體。", 0.6, 0, 0},
		{"starter_feet_m", "草鞋", "feet", "equipment", "稻草編成的簡單鞋履，走久了會磨腳。", 0.3, 0, 0},
		{"starter_body_f", "素布衣裙", "body", "equipment", "素色布料裁成的衣裙，樸素但整潔。", 0.7, 0, 0},
		{"starter_legs_f", "布裳", "legs", "equipment", "普通的布裳，行動尚算方便。", 0.5, 0, 0},
		{"starter_feet_f", "布鞋", "feet", "equipment", "軟底布鞋，輕便耐走。", 0.2, 0, 0},
		// 鎂面額（背包規格 §四）
		{"mg_coin", "鎂幣", "", "currency", "一枚小巧的鎂質錢幣，面值一鎂。", 0.01, 1, 1},
		{"mg_note", "鎂鈔", "", "currency", "以特殊纖維印製的鈔票，面值十鎂。", 0.005, 1, 10},
		{"mg_ingot", "鎂錠", "", "currency", "沉甸甸的鎂金屬錠，面值一萬鎂。", 1.0, 1, 10000},
	}
	for _, it := range all {
		if _, err := db.Exec(
			`INSERT OR REPLACE INTO items (id, name, slot, item_type, weight, stackable, denomination, description)
			 VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
			it.id, it.name, it.slot, it.itemType, it.weight, it.stackable, it.denomination, it.description,
		); err != nil {
			return err
		}
	}
	return nil
}
