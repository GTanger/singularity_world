// 模擬完整記憶流程：需專案內 data/rooms 存在，否則 Skip。
package server

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"singularity_world/db"
	"singularity_world/store"
)

// TestMemorySystemIntegration 模擬完整記憶流程：buffer → 逾時 consolidation → 寫入 archival + summary → 檢索。
// 執行：go test ./server/... -run TestMemorySystemIntegration -v
func TestMemorySystemIntegration(t *testing.T) {
	// 使用專案 data/rooms（從 server 包執行時為 ../data/rooms）
	wd, _ := os.Getwd()
	roomsPath := filepath.Join(wd, "..", "data", "rooms")
	if _, err := os.Stat(roomsPath); err != nil {
		roomsPath = filepath.Join(wd, "data", "rooms")
	}
	if _, err := os.Stat(roomsPath); err != nil {
		t.Skipf("data/rooms not found (tried %s, wd=%s), skip integration", roomsPath, wd)
	}
	tmp := t.TempDir()
	oldDefault := store.Default
	defer func() { store.Default = oldDefault }()
	if err := store.Init(roomsPath, tmp, ""); err != nil {
		t.Fatalf("store.Init: %v", err)
	}
	if store.Default == nil {
		t.Fatal("store.Default is nil after Init")
	}

	playerID := "test_player"
	npcID := "test_npc"
	t0 := int64(1000)

	// 第一輪：你好
	FlushConversationAndAppend(playerID, npcID, "你好", "你好啊，有什麼事？", t0)
	entries := store.Default.GetArchivalByEntity(npcID)
	if len(entries) != 0 {
		t.Errorf("after turn 1: want 0 archival (no flush yet), got %d", len(entries))
	}

	// 第二輪：同一場
	FlushConversationAndAppend(playerID, npcID, "天氣如何", "還行。", t0+1)
	entries = store.Default.GetArchivalByEntity(npcID)
	if len(entries) != 0 {
		t.Errorf("after turn 2: want 0 archival (no flush yet), got %d", len(entries))
	}

	// 第三輪：逾時 130 秒，觸發 consolidation，應寫入上一場 2 輪 → 2 條
	FlushConversationAndAppend(playerID, npcID, "再見", "再見。", t0+130)
	entries = store.Default.GetArchivalByEntity(npcID)
	if len(entries) < 2 {
		t.Errorf("after turn 3 (gap): want >= 2 consolidated archival entries, got %d", len(entries))
	}
	summary := store.Default.GetNpcSummary(npcID)
	if summary == "" {
		t.Error("after consolidation: want non-empty NpcSummary, got empty")
	}
	if !strings.Contains(summary, "最近") {
		t.Errorf("summary should mention 最近, got %q", summary)
	}

	// 檢索：多關鍵字應帶出相關條目（剛寫入的兩條應含「你好」「天氣」）
	snippets := db.SearchArchival(npcID, "你好 天氣", 5)
	if len(snippets) == 0 {
		t.Error("SearchArchival(你好 天氣): want some snippets, got 0")
	}
	hasRelevant := false
	for _, s := range snippets {
		if strings.Contains(s, "你好") || strings.Contains(s, "天氣") {
			hasRelevant = true
			break
		}
	}
	if !hasRelevant {
		t.Errorf("SearchArchival snippets should contain 你好 or 天氣, got %q", snippets)
	}
}
