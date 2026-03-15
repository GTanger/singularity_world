// Package ai 提供 NPC 對話 LLM 呼叫；可接 Ollama 本地模型（如 qwen-4b-slim）。
package ai

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"strings"
	"time"
)

// CallAITalk 呼叫 LLM 回覆。baseURL 與 model 為空時不呼叫、回傳 err 走 fallback。
// playerInput 為玩家輸入；npcBackstory 為 BuildIdentity 組出的 identity；npcMemorySnippets 為 SearchArchival 取回的記憶；styleExamples 為對話池口吻範例。
func CallAITalk(baseURL, model, playerInput, npcBackstory string, npcMemorySnippets, styleExamples []string) (string, error) {
	if baseURL == "" || model == "" {
		return "", errors.New("ollama not configured")
	}
	baseURL = strings.TrimSuffix(baseURL, "/")

	// system：強調「針對玩家剛說的話」回覆，避免每次都套版問候
	sb := strings.Builder{}
	sb.WriteString("你是修真市井世界裡的一名 NPC。請「根據玩家剛剛說的那句話」直接回覆，不要無視玩家內容，不要每次都說「你好／有什麼事嗎」這類套話。可以反問、接話、敷衍、吐槽、簡短感嘆，語氣像真人隨口回應，一兩句即可。只輸出 NPC 會說的那句話，不要加「他說」或引號外的說明。\n")
	if npcBackstory != "" {
		sb.WriteString("你的身份與背景：")
		sb.WriteString(npcBackstory)
		sb.WriteString("\n")
	}
	if len(styleExamples) > 0 {
		sb.WriteString("口吻可參考（語氣與長度類似）：\n")
		for _, ex := range styleExamples {
			sb.WriteString("- ")
			sb.WriteString(strings.TrimSpace(ex))
			sb.WriteString("\n")
		}
	}

	// user：玩家說了什麼（若有記憶可附上）
	userMsg := "玩家剛剛說：" + playerInput + "\n請針對這句話回一句 NPC 會說的話。"
	if len(npcMemorySnippets) > 0 {
		userMsg = "玩家剛剛說：" + playerInput + "\n（若與以下記憶有關可略提）\n" + strings.Join(npcMemorySnippets, "\n") + "\n請針對玩家的話回一句 NPC 會說的話。"
	}

	reqBody := map[string]interface{}{
		"model":   model,
		"think":   false,
		"stream":  false,
		"messages": []map[string]string{
			{"role": "system", "content": sb.String()},
			{"role": "user", "content": userMsg},
		},
		"options": map[string]interface{}{
			"num_predict": 100,
			"temperature": 0.85,
		},
	}
	body, _ := json.Marshal(reqBody)

	client := &http.Client{Timeout: 60 * time.Second}
	resp, err := client.Post(baseURL+"/api/chat", "application/json", bytes.NewReader(body))
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("ollama %s", resp.Status)
	}

	var out struct {
		Message struct {
			Content string `json:"content"`
		} `json:"message"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return "", err
	}
	reply := strings.TrimSpace(out.Message.Content)
	if reply == "" {
		return "", errors.New("empty reply")
	}
	// 若模型回了引號或「說道」等，只取第一句或去殼
	if strings.HasPrefix(reply, "「") && strings.Contains(reply, "」") {
		if i := strings.Index(reply, "」"); i > 0 {
			reply = reply[1:i]
		}
	}
	return reply, nil
}
