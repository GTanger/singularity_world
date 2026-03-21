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
	"unicode/utf8"
)

// CallAITalk 呼叫 LLM 回覆。baseURL 與 model 為空時不呼叫、回傳 err 走 fallback。
// playerInput 為玩家輸入；npcBackstory 為 BuildIdentity 組出的 identity；npcMemorySnippets 為 SearchArchival 取回的記憶；styleExamples 為對話池口吻範例；sensitivityHint 為口吻與長度提示（如「此角色較冷淡，回覆簡短」），可為空。
func CallAITalk(baseURL, model, playerInput, npcBackstory string, npcMemorySnippets, styleExamples []string, sensitivityHint string) (string, error) {
	if baseURL == "" || model == "" {
		return "", errors.New("ollama not configured")
	}
	baseURL = strings.TrimSuffix(baseURL, "/")

	// system：強調「針對玩家剛說的話」回覆，並一律使用繁體中文
	sb := strings.Builder{}
	sb.WriteString("你是修真市井世界裡的一名 NPC。請「根據玩家剛剛說的那句話」直接回覆，不要無視玩家內容，不要每次都說「你好／有什麼事嗎」這類套話。可以反問、接話、敷衍、吐槽、簡短感嘆，語氣像真人隨口回應，一兩句即可。只輸出 NPC 會說的那句話，不要加「他說」或引號外的說明。\n")
	sb.WriteString("請一律使用繁體中文（台灣用字）回覆，輸出內容僅限繁體中文。\n")
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
		"model":  model,
		"think":  false,
		"stream": false,
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

// isNpcNpcMetaOrAssistantLine 過濾模型常見的後設／助理說明行，避免進入遊戲 Log。
func isNpcNpcMetaOrAssistantLine(s string) bool {
	t := strings.TrimSpace(s)
	if t == "" {
		return true
	}
	low := strings.ToLower(t)
	if strings.HasPrefix(t, "---") || strings.HasPrefix(t, "——") {
		return true
	}
	bad := []string{
		"結束對話", "若需繼續", "重新換行", "換行提問",
		"情境需求", "更簡短", "不同切入", "請再說", "若您", "如果您",
		"（若想", "（若需", "（結束", "作為ai", "作為 AI",
		"抱歉我無法", "無法生成", "```",
	}
	for _, b := range bad {
		if strings.Contains(t, b) || strings.Contains(low, strings.ToLower(b)) {
			return true
		}
	}
	return false
}

// stripOuterGuillemetLine 逐層剥掉行首行尾單對「」（各 3-byte UTF-8），避免再被 UI 包一層引號。
func stripOuterGuillemetLine(s string) string {
	s = strings.TrimSpace(s)
	const open, close = "「", "」"
	for strings.HasPrefix(s, open) && strings.HasSuffix(s, close) && len(s) >= len(open)+len(close) {
		next := strings.TrimSpace(s[len(open) : len(s)-len(close)])
		if next == "" || next == s {
			break
		}
		s = next
	}
	return strings.TrimSpace(s)
}

// stripLeadingSpeakerName 若台詞以「顯示名」開頭且後接動詞（非冒號），去掉重複署名（例：袁寧看看路標 → 看看路標）。
func stripLeadingSpeakerName(line, speakerName string) string {
	line = strings.TrimSpace(line)
	speakerName = strings.TrimSpace(speakerName)
	if speakerName == "" || !strings.HasPrefix(line, speakerName) {
		return line
	}
	rest := strings.TrimSpace(line[len(speakerName):])
	if rest == "" {
		return line
	}
	r, _ := utf8.DecodeRuneInString(rest)
	if r == '：' || r == ':' || r == '，' || r == ',' {
		return line
	}
	return rest
}

// parseNPCToNPCLines 從模型原始輸出取出兩行純台詞（略過後設與空行）。
func parseNPCToNPCLines(raw, speakerName, listenerName string) (lineA, lineB string, ok bool) {
	var picked []string
	for _, part := range strings.Split(raw, "\n") {
		line := strings.TrimSpace(part)
		if isNpcNpcMetaOrAssistantLine(line) {
			continue
		}
		picked = append(picked, line)
		if len(picked) >= 2 {
			break
		}
	}
	if len(picked) < 2 {
		return "", "", false
	}
	lineA = stripOuterGuillemetLine(picked[0])
	lineB = stripOuterGuillemetLine(picked[1])
	lineA = stripLeadingSpeakerName(lineA, speakerName)
	lineB = stripLeadingSpeakerName(lineB, listenerName)
	// 台詞內勿再嵌「…說：「子句」」；若仍有多層「，只保留最內層一句
	lineA = collapseNestedSayGuillemets(lineA)
	lineB = collapseNestedSayGuillemets(lineB)
	if lineA == "" || lineB == "" {
		return "", "", false
	}
	return lineA, lineB, true
}

// collapseNestedSayGuillemets 將「…說／問／道：「內文」」縮成「內文」，減少 Log 裡「」套「」。
func collapseNestedSayGuillemets(s string) string {
	s = strings.TrimSpace(s)
	seps := []string{"說：「", "問：「", "道：「", "说道：「"}
	for {
		bestIdx := -1
		bestLen := 0
		for _, sep := range seps {
			i := strings.LastIndex(s, sep)
			if i >= 0 && (bestIdx < 0 || i >= bestIdx) {
				bestIdx = i
				bestLen = len(sep)
			}
		}
		if bestIdx < 0 {
			break
		}
		rest := s[bestIdx+bestLen:]
		j := strings.Index(rest, "」")
		if j <= 0 {
			break
		}
		inner := strings.TrimSpace(rest[:j])
		if inner == "" {
			break
		}
		s = inner
	}
	for strings.HasSuffix(s, "」」") {
		s = strings.TrimSuffix(s, "」")
	}
	return strings.TrimSpace(s)
}

// NpcNpcDialogue 為 NPC↔NPC 結構化輸出。
type NpcNpcDialogue struct {
	A       string   `json:"a"`
	B       string   `json:"b"`
	Anchors []string `json:"anchors,omitempty"`
}

func parseNpcNpcDialogueJSON(raw, speakerName, listenerName string) (NpcNpcDialogue, bool) {
	txt := strings.TrimSpace(raw)
	if txt == "" {
		return NpcNpcDialogue{}, false
	}
	start := strings.Index(txt, "{")
	end := strings.LastIndex(txt, "}")
	if start < 0 || end <= start {
		return NpcNpcDialogue{}, false
	}
	txt = txt[start : end+1]
	var d NpcNpcDialogue
	if err := json.Unmarshal([]byte(txt), &d); err != nil {
		return NpcNpcDialogue{}, false
	}
	d.A = collapseNestedSayGuillemets(stripLeadingSpeakerName(stripOuterGuillemetLine(strings.TrimSpace(d.A)), speakerName))
	d.B = collapseNestedSayGuillemets(stripLeadingSpeakerName(stripOuterGuillemetLine(strings.TrimSpace(d.B)), listenerName))
	if d.A == "" || d.B == "" {
		return NpcNpcDialogue{}, false
	}
	var anchors []string
	for _, a := range d.Anchors {
		a = strings.TrimSpace(a)
		if a == "" || isNpcNpcMetaOrAssistantLine(a) {
			continue
		}
		if len([]rune(a)) > 18 {
			a = string([]rune(a)[:18])
		}
		anchors = append(anchors, a)
		if len(anchors) >= 2 {
			break
		}
	}
	d.Anchors = anchors
	return d, true
}

// CallAITalkNPCToNPC 產生「A 對 B 說一句、B 回一句」的 NPC 間對話（NPC＝玩家：同一套 AI）。
// recentRoomEvents 為當前房最近動靜（L0 滑窗）可選注入；npcNpcMemory 為兩人過往摘要；topicHint 為主題提示。
// roomDescription 為房間描述節錄（強化現場感）；roomTags 為房間標籤（逗號分隔）；playerInRoom 為玩家在旁時要求更短、更像低聲閒聊。
func CallAITalkNPCToNPC(baseURL, model, speakerName, listenerName, speakerBackstory, listenerBackstory, roomName, timeLabel string, recentRoomEvents []string, relationHint, npcNpcMemory, topicHint, roomDescription, roomTags string, playerInRoom bool) (lineA, lineB string, anchors []string, err error) {
	if baseURL == "" || model == "" {
		return "", "", nil, errors.New("ollama not configured")
	}
	baseURL = strings.TrimSuffix(baseURL, "/")

	sb := strings.Builder{}
	sb.WriteString("產生兩句 NPC 對話：第一句是說話者，第二句是聽者回應。\n")
	sb.WriteString("規則只有三條：\n")
	sb.WriteString("1) 每句 <= 28 字，口語短句。\n")
	sb.WriteString("2) 至少一句要貼合現場（地點/時段/最近動靜）。\n")
	sb.WriteString("3) 禁止後設說明、禁止旁白、禁止嵌套引號。\n")
	sb.WriteString("輸出繁體中文。\n")
	sb.WriteString("優先輸出 JSON：{\"a\":\"...\",\"b\":\"...\",\"anchors\":[\"...\"]}；若失敗再輸出兩行純台詞。\n\n")
	if playerInRoom {
		sb.WriteString("【重要】旁邊有路人在場：兩人說話要更短、更收斂，像在路人附近低聲講；不要對路人演講。可自然帶一句鎮上傳聞或眼前小事。\n\n")
	}
	sb.WriteString("說話者：" + speakerName + "\n")
	if speakerBackstory != "" {
		sb.WriteString("其身份與背景：" + speakerBackstory + "\n")
	}
	sb.WriteString("聽者：" + listenerName + "\n")
	if listenerBackstory != "" {
		sb.WriteString("其身份與背景：" + listenerBackstory + "\n")
	}
	sb.WriteString("情境：在「" + roomName + "」，" + timeLabel + "。兩人正在同處，自然交談一句。\n")
	roomDescription = strings.TrimSpace(roomDescription)
	if roomDescription != "" {
		if len([]rune(roomDescription)) > 120 {
			roomDescription = string([]rune(roomDescription)[:120]) + "…"
		}
		sb.WriteString("地點氛圍（可化用一字半句，勿照抄長段）：" + roomDescription + "\n")
	}
	roomTags = strings.TrimSpace(roomTags)
	if roomTags != "" {
		sb.WriteString("地點標籤（語氣可呼應其一，勿列舉標籤名）：" + roomTags + "\n")
	}
	if len(recentRoomEvents) > 0 {
		sb.WriteString("最近動靜（可接話，但不用硬提）：\n")
		for _, ev := range recentRoomEvents {
			ev = strings.TrimSpace(ev)
			if ev == "" {
				continue
			}
			sb.WriteString("- ")
			sb.WriteString(ev)
			sb.WriteString("\n")
		}
	}
	if topicHint != "" {
		sb.WriteString("主題或情境提示：" + topicHint + "（可依此調整內容方向）。\n")
	}
	if relationHint != "" {
		sb.WriteString("兩人關係提示：" + relationHint + "。\n")
	}
	if npcNpcMemory != "" {
		sb.WriteString("兩人過往交談摘要（可接續或換話題，勿重複同一句）：" + npcNpcMemory + "\n")
	}

	reqBody := map[string]interface{}{
		"model":  model,
		"think":  false,
		"stream": false,
		"messages": []map[string]string{
			{"role": "system", "content": sb.String()},
			{"role": "user", "content": "請輸出 JSON（首選）或恰好兩行純台詞：每行不超過 28 字，勿第三行、勿前後說明，勿嵌套「」。"},
		},
		"options": map[string]interface{}{
			"num_predict": 72,
			"temperature": 0.68,
		},
	}
	body, _ := json.Marshal(reqBody)

	client := &http.Client{Timeout: 60 * time.Second}
	resp, err := client.Post(baseURL+"/api/chat", "application/json", bytes.NewReader(body))
	if err != nil {
		return "", "", nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return "", "", nil, fmt.Errorf("ollama %s", resp.Status)
	}

	var out struct {
		Message struct {
			Content string `json:"content"`
		} `json:"message"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return "", "", nil, err
	}
	raw := strings.TrimSpace(out.Message.Content)
	if raw == "" {
		return "", "", nil, errors.New("empty reply")
	}

	if d, ok := parseNpcNpcDialogueJSON(raw, speakerName, listenerName); ok {
		return d.A, d.B, d.Anchors, nil
	}

	lineA, lineB, ok := parseNPCToNPCLines(raw, speakerName, listenerName)
	if ok {
		return lineA, lineB, nil, nil
	}
	// 退回：僅第一個非後設行當 A，B 用極短回覆
	for _, part := range strings.Split(raw, "\n") {
		line := strings.TrimSpace(part)
		if line == "" || isNpcNpcMetaOrAssistantLine(line) {
			continue
		}
		line = stripOuterGuillemetLine(line)
		line = stripLeadingSpeakerName(line, speakerName)
		line = collapseNestedSayGuillemets(line)
		if line != "" {
			return line, "嗯。", nil, nil
		}
	}
	return "", "", nil, errors.New("could not parse two lines")
}
