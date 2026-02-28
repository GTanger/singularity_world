// 刪除所有現有角色（entity_auth、entity_room、entities）。離線執行，不需啟動伺服器。
package main

import (
	"fmt"
	"log"
	"os"

	"singularity_world/config"
	"singularity_world/db"
)

func main() {
	cfg := config.DefaultServer()
	if p := os.Getenv("DB_PATH"); p != "" {
		cfg.DBPath = p
	}
	database, err := db.OpenDB(cfg.DBPath)
	if err != nil {
		log.Fatalf("open db: %v", err)
	}
	defer database.Close()
	if err := db.DeleteAllEntities(database); err != nil {
		log.Fatalf("delete entities: %v", err)
	}
	fmt.Println("已刪除所有角色。")
}
