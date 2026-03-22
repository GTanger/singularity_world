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

// PlayerNPCTalkBuildPrompts 組出玩家↔NPC Talk 的 system 與 user 字串（Ollama /api/chat 與 OpenAI 相容 /v1/chat/completions 共用）。
// roomDisplayName 為玩家當前房間顯示名（可空）；非空時依 llm_prompts.json 的 room_context_fmt 注入情境。
func PlayerNPCTalkBuildPrompts(playerInput, npcBackstory string, npcMemorySnippets, styleExamples []string, sensitivityHint, roomDisplayName string) (systemPrompt, userMsg string) {
	var sb strings.Builder
	var user string
	sb.WriteString(playerNPCPersonaLine())
	sb.WriteString(WorldPhenomenaCognitionPrompt())
	roomDisplayName = strings.TrimSpace(roomDisplayName)
	if roomDisplayName != "" {
		if fmtLine := strings.TrimSpace(playerNPCRoomContextFmt()); fmtLine != "" {
			sb.WriteString(fmt.Sprintf(fmtLine, roomDisplayName))
		}
		if sc := strings.TrimSpace(playerNPCSpaceConsistencyRule()); sc != "" {
			if !strings.HasSuffix(sc, "\n") {
				sc += "\n"
			}
			sb.WriteString(sc)
		}
	}
	sb.WriteString(playerNPCBehaviorRules())
	sb.WriteString(playerNPCTraditionalRule())
	if npcBackstory != "" {
		sb.WriteString(playerNPCIdentityPrefix())
		sb.WriteString(npcBackstory)
		sb.WriteString("\n")
	}
	if sensitivityHint != "" {
		sb.WriteString(playerNPCSensitivityPrefix())
		sb.WriteString(sensitivityHint)
		sb.WriteString("\n")
	}
	if len(styleExamples) > 0 {
		sb.WriteString(playerNPCStyleExamplesHeader())
		bullet := playerNPCStyleExampleBullet()
		for _, ex := range styleExamples {
			sb.WriteString(bullet)
			sb.WriteString(strings.TrimSpace(ex))
			sb.WriteString("\n")
		}
	}
	if len(npcMemorySnippets) > 0 {
		user = fmt.Sprintf(playerNPCUserWithMemoryFmt(), playerInput, strings.Join(npcMemorySnippets, "\n"))
	} else {
		user = fmt.Sprintf(playerNPCUserPlainFmt(), playerInput)
	}
	return sb.String(), user
}

func CallAITalk(baseURL, model, playerInput, npcBackstory string, npcMemorySnippets, styleExamples []string, sensitivityHint, roomDisplayName string) (string, error) {
	if baseURL == "" || model == "" {
		return "", errors.New("ollama not configured")
	}
	baseURL = strings.TrimSuffix(baseURL, "/")

	systemPrompt, userMsg := PlayerNPCTalkBuildPrompts(playerInput, npcBackstory, npcMemorySnippets, styleExamples, sensitivityHint, roomDisplayName)

	reqBody := map[string]interface{}{
		"model":  model,
		"think":  false,
		"stream": false,
		"messages": []map[string]string{
			{"role": "system", "content": systemPrompt},
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
	gopen, gclose := string(rune(0x300c)), string(rune(0x300d))
	if strings.HasPrefix(reply, gopen) && strings.Contains(reply, gclose) {
		if i := strings.Index(reply, gclose); i > 0 {
			reply = reply[len(gopen):i]
		}
	}
	return reply, nil
}

func isNpcNpcMetaOrAssistantLine(s string) bool {
	t := strings.TrimSpace(s)
	if t == "" {
		return true
	}
	low := strings.ToLower(t)
	if strings.HasPrefix(t, "---") || strings.HasPrefix(t, "——") {
		return true
	}
	for _, b := range metaBadMarkers() {
		if b == "" {
			continue
		}
		if strings.Contains(t, b) || strings.Contains(low, strings.ToLower(b)) {
			return true
		}
	}
	return false
}

func stripOuterGuillemetLine(s string) string {
	s = strings.TrimSpace(s)
	open, close := string(rune(0x300c)), string(rune(0x300d))
	for strings.HasPrefix(s, open) && strings.HasSuffix(s, close) && len(s) >= len(open)+len(close) {
		next := strings.TrimSpace(s[len(open) : len(s)-len(close)])
		if next == "" || next == s {
			break
		}
		s = next
	}
	return strings.TrimSpace(s)
}

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
	lineA = collapseNestedSayGuillemets(lineA)
	lineB = collapseNestedSayGuillemets(lineB)
	if lineA == "" || lineB == "" {
		return "", "", false
	}
	return lineA, lineB, true
}

func collapseNestedSayGuillemets(s string) string {
	s = strings.TrimSpace(s)
	seps := npcToNpcCollapseSaySeparators()
	if len(seps) > 0 {
		for {
			bestIdx := -1
			bestLen := 0
			for _, sep := range seps {
				if sep == "" {
					continue
				}
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
			j := strings.Index(rest, string(rune(0x300d)))
			if j <= 0 {
				break
			}
			inner := strings.TrimSpace(rest[:j])
			if inner == "" {
				break
			}
			s = inner
		}
	}
	gc := string(rune(0x300d))
	for strings.HasSuffix(s, gc+gc) {
		s = strings.TrimSuffix(s, gc)
	}
	return strings.TrimSpace(s)
}

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

func CallAITalkNPCToNPC(baseURL, model, speakerName, listenerName, speakerBackstory, listenerBackstory, roomName, timeLabel string, recentRoomEvents []string, relationHint, npcNpcMemory, topicHint, roomDescription, roomTags string, playerInRoom bool) (lineA, lineB string, anchors []string, err error) {
	if baseURL == "" || model == "" {
		return "", "", nil, errors.New("ollama not configured")
	}
	baseURL = strings.TrimSuffix(baseURL, "/")

	sb := strings.Builder{}
	sb.WriteString(npcToNpcTaskIntro())
	sb.WriteString(WorldPhenomenaCognitionPrompt())
	sb.WriteString(npcToNpcToneAfterWorld())
	sb.WriteString(npcToNpcRulesBlock())
	if playerInRoom {
		sb.WriteString(npcToNpcPlayerPresentNote())
	}
	sb.WriteString(npcToNpcSpeakerLabel() + speakerName + "\n")
	if speakerBackstory != "" {
		sb.WriteString(npcToNpcIdentityPrefix() + speakerBackstory + "\n")
	}
	sb.WriteString(npcToNpcListenerLabel() + listenerName + "\n")
	if listenerBackstory != "" {
		sb.WriteString(npcToNpcIdentityPrefix() + listenerBackstory + "\n")
	}
	sb.WriteString(fmt.Sprintf(npcToNpcContextLineFmt(), roomName, timeLabel))
	roomDescription = strings.TrimSpace(roomDescription)
	if roomDescription != "" {
		if len([]rune(roomDescription)) > 120 {
			suf := npcToNpcRoomDescTruncSuffix()
			if suf == "" {
				suf = string(rune(0x2026))
			}
			roomDescription = string([]rune(roomDescription)[:120]) + suf
		}
		sb.WriteString(npcToNpcRoomAtmospherePrefix() + roomDescription + "\n")
	}
	roomTags = strings.TrimSpace(roomTags)
	if roomTags != "" {
		sb.WriteString(npcToNpcRoomTagsPrefix() + roomTags + "\n")
	}
	if len(recentRoomEvents) > 0 {
		sb.WriteString(npcToNpcRecentEventsHeader())
		bullet := npcToNpcRecentEventBullet()
		for _, ev := range recentRoomEvents {
			ev = strings.TrimSpace(ev)
			if ev == "" {
				continue
			}
			sb.WriteString(bullet)
			sb.WriteString(ev)
			sb.WriteString("\n")
		}
	}
	if topicHint != "" {
		sb.WriteString(fmt.Sprintf(npcToNpcTopicLineFmt(), topicHint))
	}
	if relationHint != "" {
		sb.WriteString(fmt.Sprintf(npcToNpcRelationLineFmt(), relationHint))
	}
	if npcNpcMemory != "" {
		sb.WriteString(fmt.Sprintf(npcToNpcMemoryLineFmt(), npcNpcMemory))
	}

	reqBody := map[string]interface{}{
		"model":  model,
		"think":  false,
		"stream": false,
		"messages": []map[string]string{
			{"role": "system", "content": sb.String()},
			{"role": "user", "content": npcToNpcUserMessage()},
		},
		"options": map[string]interface{}{
			"num_predict": 120,
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
	fallback := npcToNpcFallbackListenerReply()
	for _, part := range strings.Split(raw, "\n") {
		line := strings.TrimSpace(part)
		if line == "" || isNpcNpcMetaOrAssistantLine(line) {
			continue
		}
		line = stripOuterGuillemetLine(line)
		line = stripLeadingSpeakerName(line, speakerName)
		line = collapseNestedSayGuillemets(line)
		if line != "" {
			return line, fallback, nil, nil
		}
	}
	return "", "", nil, errors.New("could not parse two lines")
}
