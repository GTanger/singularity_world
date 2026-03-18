package server

import (
	"testing"
)

func TestConsolidateTurns(t *testing.T) {
	turns := []conversationTurn{
		{"你好", "你好。", 1000},
		{"天氣如何", "還行。", 1001},
		{"再見", "再見。", 1002},
	}
	out := consolidateTurns(turns)
	if len(out) != 3 {
		t.Fatalf("want 3, got %d", len(out))
	}
	if out[0] != "玩家說：你好；NPC回：你好。" {
		t.Errorf("got %q", out[0])
	}
	// 超過 3 輪取最後 3
	turns4 := append(turns, conversationTurn{"謝啦", "不客氣。", 1003})
	out4 := consolidateTurns(turns4)
	if len(out4) != maxConsolidate {
		t.Fatalf("want %d, got %d", maxConsolidate, len(out4))
	}
	if out4[0] != "玩家說：天氣如何；NPC回：還行。" {
		t.Errorf("last 3 first: got %q", out4[0])
	}
}

func TestSummaryFromTurns(t *testing.T) {
	if s := summaryFromTurns(nil); s != "" {
		t.Errorf("nil want \"\", got %q", s)
	}
	turns := []conversationTurn{{"不付錢就不用錢", "哼。", 1000}}
	s := summaryFromTurns(turns)
	if s != "最近聊過：「不付錢就不用錢」。" {
		t.Errorf("got %q", s)
	}
	turns2 := []conversationTurn{{"（搭話）", "嗯？", 1000}}
	s2 := summaryFromTurns(turns2)
	if s2 != "最近曾與玩家對話。" {
		t.Errorf("got %q", s2)
	}
}
