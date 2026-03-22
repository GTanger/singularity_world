package main

import (
	"os"
	"testing"

	"singularity_world/config"
	"singularity_world/gametext"
	"singularity_world/npcnpc"
)

func TestMain(m *testing.M) {
	gametext.SetPathForTest("data/config/gametext.json")
	if err := gametext.Load(""); err != nil {
		gametext.SetPathForTest("../data/config/gametext.json")
		_ = gametext.Load("")
	}
	config.SetSimulationPathForTest("data/config/simulation.json")
	_ = config.LoadSimulation("")
	os.Exit(m.Run())
}

func TestSentimentDeltaFromDialogue_worldTopics(t *testing.T) {
	if got := npcnpc.SentimentDeltaFromDialogue("", "好", "是啊", 50, "城外聞"); got < 1 {
		t.Fatalf("城外聞 want delta>=1 (incl. pos path), got %d", got)
	}
	if got := npcnpc.SentimentDeltaFromDialogue("", "好", "是啊", 50, "天候"); got < 1 {
		t.Fatalf("天候 want delta>=1, got %d", got)
	}
	// hint 後備（無 topicID）
	h := "鎮上傳聞城外惡地、霜林、游離輻射或富態化巨獸（非荒蕪末世，而是生機過剩的危險）"
	if got := npcnpc.SentimentDeltaFromDialogue(h, "好", "是啊", 50, ""); got < 1 {
		t.Fatalf("hint 城外惡地 want delta>=1, got %d", got)
	}
}
