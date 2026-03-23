// 為既有玩家 ID 寫入／覆寫登入密碼（bcrypt 寫入 data/runtime/auth.json）。
// 用法（專案根目錄，建議先關閉奇點伺服器）：
//
//	go run ./cmd/sw-set-password <player_id> <新密碼>
package main

import (
	"fmt"
	"log"
	"os"

	"singularity_world/db"
	"singularity_world/store"
)

func main() {
	log.SetFlags(0)
	if len(os.Args) != 3 {
		fmt.Fprintf(os.Stderr, "用法: %s <player_id> <新密碼>\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "需在專案根目錄執行；密碼至少 6 字元；建議先停止 ./start 再執行。\n")
		os.Exit(2)
	}
	playerID := os.Args[1]
	password := os.Args[2]
	if len(password) < 6 {
		log.Fatal("密碼至少 6 個字元")
	}
	if err := os.MkdirAll("data", 0755); err != nil {
		log.Fatal(err)
	}
	if err := os.MkdirAll("data/runtime", 0755); err != nil {
		log.Fatal(err)
	}
	if err := store.Init("data/rooms", "data/runtime", "data"); err != nil {
		log.Fatalf("store init: %v", err)
	}
	ent, err := db.GetEntity(playerID)
	if err != nil || ent == nil {
		log.Fatalf("找不到角色：%q", playerID)
	}
	if ent.Kind != "player" {
		log.Fatalf("ID %q 不是玩家（kind=%s）", playerID, ent.Kind)
	}
	if err := db.CreateAuth(playerID, password); err != nil {
		log.Fatalf("寫入密碼：%v", err)
	}
	fmt.Printf("已為玩家 %q 設定登入密碼。\n", playerID)
}
