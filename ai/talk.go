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
// playerInput 為玩家輸入；npcBackstory 為 BuildIdentity 組出的 identity；npcMemorySnippets 為 SearchArchival 取回的記憶；styleExamples 為對話池口吻範例；sensitivityHint 為口吻與長度提示（如「此角色較冷淡，回覆簡短」），可為空。
func CallAITalk(baseURL, model, playerInput, npcBackstory string, npcMemorySnippets, styleExamples []string, sensitivityHint string) (string, error) {
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
	if sensitivityHint != "" {
		sb.WriteString("口吻與長度：")
		sb.WriteString(sensitivityHint)
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

// CallAITalkNPCToNPC 產生「A 對 B 說一句、B 回一句」的 NPC 間對話（NPC＝玩家：同一套 AI）。
// 參數為兩人顯示名、背版、房間名、時段；npcNpcMemory 為兩人過往交談摘要（長碰面不重複），topicHint 為主題提示（如「交班」）。回傳 lineA、lineB 為純台詞，失敗回傳 err。
func CallAITalkNPCToNPC(baseURL, model, speakerName, listenerName, speakerBackstory, listenerBackstory, roomName, timeLabel, npcNpcMemory, topicHint string) (lineA, lineB string, err error) {
	if baseURL == "" || model == "" {
		return "", "", errors.New("ollama not configured")
	}
	baseURL = strings.TrimSuffix(baseURL, "/")

	sb := strings.Builder{}
	sb.WriteString("你是修真市井世界中的旁白，要生成「兩名 NPC 之間的一來一往對話」。\n")
	sb.WriteString("請根據以下兩人身份與所在情境，生成：第一句是「說話者」對「聽者」說的一句話，第二句是「聽者」的回應。\n")
	sb.WriteString("要求：口吻像真人隨口交談，一兩句即可；不要加「他說」「她說」或引號外的說明。\n")
	sb.WriteString("輸出格式：嚴格兩行。第一行只有說話者的台詞，第二行只有聽者的台詞。每行不要包「」\n\n")
	sb.WriteString("說話者：" + speakerName + "\n")
	if speakerBackstory != "" {
		sb.WriteString("其身份與背景：" + speakerBackstory + "\n")
	}
	sb.WriteString("聽者：" + listenerName + "\n")
	if listenerBackstory != "" {
		sb.WriteString("其身份與背景：" + listenerBackstory + "\n")
	}
	sb.WriteString("情境：在「" + roomName + "」，" + timeLabel + "。兩人正在同處，自然交談一句。\n")
	if topicHint != "" {
		sb.WriteString("主題或情境提示：" + topicHint + "（可依此調整內容方向）。\n")
	}
	if npcNpcMemory != "" {
		sb.WriteString("兩人過往交談摘要（可接續或換話題，勿重複同一句）：" + npcNpcMemory + "\n")
	}

	reqBody := map[string]interface{}{
		"model":   model,
		"think":   false,
		"stream":  false,
		"messages": []map[string]string{
			{"role": "system", "content": sb.String()},
			{"role": "user", "content": "請輸出兩行：第一行是說話者對聽者說的那句話，第二行是聽者的回覆。只輸出這兩行台詞，不要其他說明。"},
		},
		"options": map[string]interface{}{
			"num_predict": 120,
			"temperature": 0.85,
		},
	}
	body, _ := json.Marshal(reqBody)

	client := &http.Client{Timeout: 60 * time.Second}
	resp, err := client.Post(baseURL+"/api/chat", "application/json", bytes.NewReader(body))
	if err != nil {
		return "", "", err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return "", "", fmt.Errorf("ollama %s", resp.Status)
	}

	var out struct {
		Message struct {
			Content string `json:"content"`
		} `json:"message"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return "", "", err
	}
	raw := strings.TrimSpace(out.Message.Content)
	if raw == "" {
		return "", "", errors.New("empty reply")
	}

	lines := strings.SplitN(raw, "\n", 2)
	for i := range lines {
		lines[i] = strings.TrimSpace(lines[i])
		// 去掉可能出現的「」包裝
		if strings.HasPrefix(lines[i], "「") && strings.Contains(lines[i], "」") {
			if j := strings.Index(lines[i], "」"); j > 0 {
				lines[i] = lines[i][1:j]
			}
		}
	}
	if len(lines) < 2 || lines[0] == "" || lines[1] == "" {
		// 只回一行時：當成 A 的台詞，B 回一句簡短
		if len(lines) >= 1 && lines[0] != "" {
			return lines[0], "嗯。", nil
		}
		return "", "", errors.New("could not parse two lines")
	}
	return lines[0], lines[1], nil
}
