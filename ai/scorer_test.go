package ai

import "testing"

func TestScoreNpcDialogue_reasonableDialogue(t *testing.T) {
	d := ScoreNpcDialogue(
		"今天街口風大，你還出攤啊？",
		"嗯，總得討生活嘛，浮生這邊人還多。",
		"四路街口",
		[]string{"天色漸暗"},
		"閒聊",
		"兩人算點頭之交，語氣中性偏熟",
		"",
		"葉俊",
		"楊康偉",
		nil,
	)
	if d.Total < 30 {
		t.Fatalf("expected decent score, got %d detail=%+v", d.Total, d)
	}
	if d.KilledBy != "" {
		t.Fatalf("unexpected kill: %s", d.KilledBy)
	}
}

func TestScoreNpcDialogue_poison(t *testing.T) {
	d := ScoreNpcDialogue("npc_1 說", "好", "房", nil, "", "", "", "A", "B", nil)
	if d.Total != 0 || d.KilledBy == "" {
		t.Fatalf("want killed, got %+v", d)
	}
}

func TestScoreNpcDialogue_wastelandTonePenalty(t *testing.T) {
	d := ScoreNpcDialogue(
		"你聽說沒？廢土那邊又出事。",
		"唉，日子難過。",
		"街口",
		nil,
		"閒聊",
		"",
		"",
		"A",
		"B",
		nil,
	)
	if d.ToneDrift != -10 {
		t.Fatalf("want tone_drift -10, got %d detail=%+v", d.ToneDrift, d)
	}
}

func TestScoreNpcDialogue_okWithEdiAndFusui(t *testing.T) {
	d := ScoreNpcDialogue(
		"城外惡地這季別單走。",
		"嗯，聽說霜林邊緣有獸跡。",
		"城門",
		[]string{"游離輻射"},
		"城外聞",
		"",
		"",
		"A",
		"B",
		nil,
	)
	if d.ToneDrift != 0 {
		t.Fatalf("惡地/霜林不應觸發廢土漂移, tone_drift=%d %+v", d.ToneDrift, d)
	}
	if d.Total < 25 {
		t.Fatalf("expected passable score, got %d %+v", d.Total, d)
	}
}
