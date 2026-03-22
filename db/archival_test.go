package db

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"singularity_world/store"
)

func TestSplitQueryTerms(t *testing.T) {
	tests := []struct {
		q    string
		want int
	}{
		{"", 0},
		{"   ", 0},
		{"你好", 1},
		{"不付錢 就不用錢", 2},
		{"  a  b  c  ", 3},
	}
	for _, tt := range tests {
		got := splitQueryTerms(tt.q)
		if len(got) != tt.want {
			t.Errorf("splitQueryTerms(%q) len = %d, want %d", tt.q, len(got), tt.want)
		}
	}
	// SearchArchival 無 store 時回傳 nil（多數測試環境未 Init store）
	if store.Default == nil {
		out := SearchArchival("npc1", "你好", 5)
		if out != nil {
			t.Errorf("SearchArchival with nil store want nil, got %v", out)
		}
		out2 := SearchArchivalForPlayerTalk("npc1", "你好", 5)
		if out2 != nil {
			t.Errorf("SearchArchivalForPlayerTalk with nil store want nil, got %v", out2)
		}
	}
}

func TestNormalizePlayerTalkQueryForGreeting(t *testing.T) {
	if g := normalizePlayerTalkQueryForGreeting("你好啊！"); g != "你好" {
		t.Errorf("normalize …你好啊！ want 你好, got %q", g)
	}
	if g := normalizePlayerTalkQueryForGreeting("Hello啊"); g != "hello" {
		t.Errorf("normalize …Hello啊 want hello, got %q", g)
	}
}

func TestShouldOmitArchivalFallbackForTalkQuery(t *testing.T) {
	if !shouldOmitArchivalFallbackForTalkQuery("你好") {
		t.Error("你好 → omit")
	}
	if !shouldOmitArchivalFallbackForTalkQuery("（搭話）") {
		t.Error("搭話 → omit")
	}
	if shouldOmitArchivalFallbackForTalkQuery("今天帳目對了沒") {
		t.Error("帳目句 → do not omit")
	}
}

// TestSearchArchivalForPlayerTalk_SkipsFallbackOnGreeting 問候零命中時不應回傳無關最新條。
func TestSearchArchivalForPlayerTalk_SkipsFallbackOnGreeting(t *testing.T) {
	wd, _ := os.Getwd()
	roomsPath := filepath.Join(wd, "..", "data", "rooms")
	if _, err := os.Stat(roomsPath); err != nil {
		roomsPath = filepath.Join(wd, "data", "rooms")
	}
	if _, err := os.Stat(roomsPath); err != nil {
		t.Skip("data/rooms not found, skip")
	}
	tmp := t.TempDir()
	old := store.Default
	defer func() { store.Default = old }()
	if err := store.Init(roomsPath, tmp, ""); err != nil {
		t.Fatalf("Init: %v", err)
	}
	npc := "npc_archival_talk_test"
	now := time.Now().Unix()
	_ = store.Default.AppendArchival(store.ArchivalEntry{
		EntityID:  npc,
		Content:   "與鄭秀強對話：今天帳我對過了。",
		Tag:       "npc_npc",
		CreatedAt: now,
	})
	// 一般 SearchArchival：零命中仍 fallback → 會拿到對帳句
	gen := SearchArchival(npc, "你好", 3)
	if len(gen) == 0 {
		t.Fatal("SearchArchival(你好): want fallback snippet")
	}
	// Talk 專用：寒暄省略 fallback
	talk := SearchArchivalForPlayerTalk(npc, "你好", 3)
	if len(talk) != 0 {
		t.Fatalf("SearchArchivalForPlayerTalk(你好): want no irrelevant fallback, got %v", talk)
	}
	// 有命中時仍應回傳
	talk2 := SearchArchivalForPlayerTalk(npc, "帳", 3)
	if len(talk2) == 0 || !archivalTestContainsAny(talk2, "帳") {
		t.Fatalf("SearchArchivalForPlayerTalk(帳): want matching snippet, got %v", talk2)
	}
}

func archivalTestContainsAny(ss []string, sub string) bool {
	for _, s := range ss {
		if strings.Contains(s, sub) {
			return true
		}
	}
	return false
}
