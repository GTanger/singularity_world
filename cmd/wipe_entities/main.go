// 刪除所有現有角色（auth、entity_room、entities、event_log）。離線執行，不需啟動伺服器。
// 以 JSON 為唯一數據源：載入 store 後清空並寫回對應 JSON，不使用 DB。
package main

import (
	"fmt"
	"log"

	"singularity_world/store"
)

func main() {
	if err := store.Init("data/rooms", "data/runtime", "data"); err != nil {
		log.Fatalf("store init: %v", err)
	}
	if err := store.ClearAllEntities(); err != nil {
		log.Fatalf("clear entities: %v", err)
	}
	fmt.Println("已刪除所有角色（已寫回 data/runtime/*.json、data/entities.json）。")
}
