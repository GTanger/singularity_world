// Package ai 本地 LLM 壓力測試：模擬多地多玩家同時／連續對話。
package ai

import (
	"fmt"
	"os"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

func getOllamaConfig(t *testing.T) (baseURL, model string, skip bool) {
	baseURL = os.Getenv("OLLAMA_BASE_URL")
	if baseURL == "" {
		baseURL = "http://127.0.0.1:11434"
	}
	model = os.Getenv("OLLAMA_MODEL")
	if model == "" {
		model = "qwen-4b-slim"
	}
	if os.Getenv("OLLAMA_DISABLE") == "1" || os.Getenv("OLLAMA_DISABLE") == "true" {
		t.Skip("OLLAMA_DISABLE=1, skip local LLM stress test")
	}
	return baseURL, model, false
}

// 三十地短背版（供 30 玩家壓力測試用）
var places30 = []struct {
	name      string
	backstory string
}{
	{"浮生客棧", "你是浮生客棧的掌櫃，見多識廣。"},
	{"東街茶館", "你是東街茶館的夥計，熱情好客。"},
	{"西市鐵鋪", "你是西市鐵鋪的鐵匠，話不多。"},
	{"南門守衛", "你是南門的守衛，負責盤查。"},
	{"北巷藥鋪", "你是北巷藥鋪的學徒，略懂草藥。"},
	{"城中酒肆", "你是城中酒肆的酒保，愛聊八卦。"},
	{"橋頭攤販", "你是橋頭擺攤的小販，精打細算。"},
	{"書院門房", "你是書院的門房，斯文有禮。"},
	{"碼頭苦力", "你是碼頭的苦力，直來直往。"},
	{"廟口相士", "你是廟口的相士，說話玄乎。"},
	{"青石巷口", "你是青石巷口看攤的老漢，慢條斯理。"},
	{"蓮花池畔", "你是蓮花池畔的掃地僧，不愛多話。"},
	{"鐘樓腳下", "你是鐘樓腳下的更夫，熟知時辰。"},
	{"驛站馬房", "你是驛站馬房的夥計，一身草料味。"},
	{"裁縫鋪子", "你是裁縫鋪子的學徒，手巧嘴甜。"},
	{"當鋪櫃檯", "你是當鋪櫃檯的朝奉，眼光毒辣。"},
	{"戲臺後場", "你是戲臺後場的琴師，見慣冷暖。"},
	{"武館門前", "你是武館門前的護院，膀大腰圓。"},
	{"私塾講堂", "你是私塾講堂的先生，之乎者也。"},
	{"胭脂鋪內", "你是胭脂鋪內的掌櫃娘，能說會道。"},
	{"肉鋪案前", "你是肉鋪案前的刀手，刀工了得。"},
	{"茶樓雅座", "你是茶樓雅座的茶博士，沖茶一絕。"},
	{"賭坊門口", "你是賭坊門口的把風，見人說人話。"},
	{"醫館診間", "你是醫館診間的學徒，略通脈象。"},
	{"棺材鋪裡", "你是棺材鋪裡的老闆，看淡生死。"},
	{"香燭紙馬", "你是香燭紙馬店的夥計，虔誠本分。"},
	{"魚市攤位", "你是魚市攤位的魚販，嗓門大。"},
	{"菜市角落", "你是菜市角落的菜農，老實巴交。"},
	{"布莊櫃上", "你是布莊櫃上的夥計，會挑花色。"},
	{"銀樓櫃內", "你是銀樓櫃內的師傅，識得成色。"},
}

// TestLocalLLM_10Players10Places 模擬十地十玩家同時對 NPC 對話，對本地 Ollama 做並行壓力測試。
// 需本地 Ollama 運行中；設 OLLAMA_DISABLE=1 或無 OLLAMA_BASE_URL 時 Skip。
// 執行：go test ./ai/... -run TestLocalLLM_10Players10Places -v
func TestLocalLLM_10Players10Places(t *testing.T) {
	baseURL, model, _ := getOllamaConfig(t)

	// 十地：取前 10 個
	places := places30[:10]

	// 十句玩家輸入，模擬十人同時開口
	inputs := []string{
		"你好，請問這裡是哪裡？",
		"今天天氣如何？",
		"有吃的嗎？",
		"多少錢？",
		"最近有什麼新鮮事？",
		"能幫個忙嗎？",
		"趕時間，快一點。",
		"謝了。",
		"再會。",
		"（點頭致意）",
	}

	const n = 10
	var wg sync.WaitGroup
	startWall := time.Now()
	var (
		okCount   int32
		errCount  int32
		durations = make([]time.Duration, n)
	)
	for i := 0; i < n; i++ {
		wg.Add(1)
		idx := i
		go func() {
			defer wg.Done()
			place := places[idx%len(places)]
			input := inputs[idx%len(inputs)]
			reqStart := time.Now()
			reply, err := CallAITalk(baseURL, model, input, place.backstory, nil, nil, "")
			durations[idx] = time.Since(reqStart)
			if err != nil {
				t.Logf("[地%d %s] 玩家說「%s」 err=%v", idx+1, place.name, input, err)
				atomic.AddInt32(&errCount, 1)
				return
			}
			if reply == "" {
				atomic.AddInt32(&errCount, 1)
				return
			}
			atomic.AddInt32(&okCount, 1)
			t.Logf("[地%d %s] 玩家「%s」 → NPC「%s」 (耗時 %v)", idx+1, place.name, input, trunc(reply, 30), durations[idx])
		}()
	}
	wg.Wait()
	wall := time.Since(startWall)

	ok := atomic.LoadInt32(&okCount)
	err := atomic.LoadInt32(&errCount)
	var minD, maxD, sumD time.Duration
	for i := 0; i < n; i++ {
		d := durations[i]
		if d > 0 {
			if minD == 0 || d < minD {
				minD = d
			}
			if d > maxD {
				maxD = d
			}
			sumD += d
		}
	}
	avgD := time.Duration(0)
	if ok+err > 0 {
		avgD = sumD / time.Duration(n)
	}

	t.Logf("--- 十地十玩家同時對話 壓力測試結果 ---")
	t.Logf("  成功: %d, 失敗: %d", ok, err)
	t.Logf("  牆鐘總耗時: %v", wall)
	t.Logf("  單筆耗時: min=%v max=%v avg≈%v", minD, maxD, avgD)
	if ok < int32(n) && err > 0 {
		t.Logf("  (部分失敗屬預期，可檢查 Ollama 併發或逾時)")
	}
}

// 十輪對話的玩家輸入範本（每輪一句）
var roundInputs10 = []string{
	"你好。", "請問貴姓？", "這兒有什麼好吃的？", "多少錢？", "謝了。",
	"再來一點。", "最近生意如何？", "有緣再會。", "告辭。", "（拱手）",
}

// TestLocalLLM_30Players30Places_10Rounds 三十地三十玩家同時進行連續十輪對話，共 300 次 CallAITalk，高壓力測試。
// 需本地 Ollama 運行中；設 OLLAMA_DISABLE=1 時 Skip。執行：go test ./ai/... -run TestLocalLLM_30Players30Places_10Rounds -v -timeout 600s
func TestLocalLLM_30Players30Places_10Rounds(t *testing.T) {
	baseURL, model, _ := getOllamaConfig(t)

	const players, rounds = 30, 10
	total := players * rounds // 300

	durations := make([]time.Duration, total)
	var okCount, errCount int32
	var wg sync.WaitGroup
	startWall := time.Now()

	for p := 0; p < players; p++ {
		wg.Add(1)
		playerIdx := p
		go func() {
			defer wg.Done()
			place := places30[playerIdx%len(places30)]
			for r := 0; r < rounds; r++ {
				input := roundInputs10[r%len(roundInputs10)]
				if r > 0 {
					input = fmt.Sprintf("第%d輪：%s", r+1, input)
				}
				reqStart := time.Now()
				reply, err := CallAITalk(baseURL, model, input, place.backstory, nil, nil, "")
				d := time.Since(reqStart)
				durations[playerIdx*rounds+r] = d
				if err != nil || reply == "" {
					atomic.AddInt32(&errCount, 1)
					continue
				}
				atomic.AddInt32(&okCount, 1)
				if playerIdx == 0 && r == 0 {
					t.Logf("[玩家1 %s] 第1輪 「%s」→ NPC「%s」 (耗時 %v)", place.name, input, trunc(reply, 25), d)
				}
			}
		}()
	}
	wg.Wait()
	wall := time.Since(startWall)

	ok := atomic.LoadInt32(&okCount)
	err := atomic.LoadInt32(&errCount)
	var minD, maxD, sumD time.Duration
	for _, d := range durations {
		if d > 0 {
			if minD == 0 || d < minD {
				minD = d
			}
			if d > maxD {
				maxD = d
			}
			sumD += d
		}
	}
	avgD := time.Duration(0)
	if total > 0 {
		avgD = sumD / time.Duration(total)
	}

	t.Logf("--- 三十地三十玩家連續十輪對話 壓力測試結果 ---")
	t.Logf("  總請求: %d  成功: %d  失敗: %d", total, ok, err)
	t.Logf("  牆鐘總耗時: %v", wall)
	t.Logf("  單筆耗時: min=%v max=%v avg≈%v", minD, maxD, avgD)
	if err > 0 {
		t.Logf("  (部分失敗可接受，可檢查 Ollama 併發／逾時／資源)")
	}
}

func trunc(s string, max int) string {
	r := []rune(s)
	if len(r) <= max {
		return s
	}
	return string(r[:max]) + "…"
}
