// OpenAI 相容 Chat Completions：供玩家對 NPC Talk 走雲端／Web API（/v1/chat/completions）。
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

// CallOpenAICompatibleChat 呼叫 baseURL/v1/chat/completions（baseURL 須為 …/v1 結尾，例如 https://api.openai.com/v1）。
// apiKey 可空（部分自建閘道不需）；非空則帶 Authorization: Bearer。
func CallOpenAICompatibleChat(baseURL, apiKey, model, systemPrompt, userMessage string) (string, error) {
	baseURL = strings.TrimSpace(strings.TrimSuffix(baseURL, "/"))
	if baseURL == "" || strings.TrimSpace(model) == "" {
		return "", errors.New("player talk api not configured")
	}
	endpoint := baseURL + "/chat/completions"

	body := map[string]interface{}{
		"model": model,
		"messages": []map[string]string{
			{"role": "system", "content": systemPrompt},
			{"role": "user", "content": userMessage},
		},
		"temperature": 0.82,
		"max_tokens":  200,
	}
	// OpenRouter：部分模型會做 reasoning／thinking，拖長首字延遲；關閉 effort 可接近「張嘴就答」的 Flash 體感（見 openrouter.ai docs reasoning-tokens）。
	if strings.Contains(strings.ToLower(baseURL), "openrouter.ai") {
		body["reasoning"] = map[string]interface{}{
			"effort": "none",
		}
	}
	raw, err := json.Marshal(body)
	if err != nil {
		return "", err
	}
	req, err := http.NewRequest(http.MethodPost, endpoint, bytes.NewReader(raw))
	if err != nil {
		return "", err
	}
	req.Header.Set("Content-Type", "application/json")
	if strings.TrimSpace(apiKey) != "" {
		req.Header.Set("Authorization", "Bearer "+strings.TrimSpace(apiKey))
	}

	client := &http.Client{Timeout: 90 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("chat api %s", resp.Status)
	}

	var out struct {
		Choices []struct {
			Message struct {
				Content string `json:"content"`
			} `json:"message"`
		} `json:"choices"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return "", err
	}
	if len(out.Choices) == 0 {
		return "", errors.New("empty choices")
	}
	reply := strings.TrimSpace(out.Choices[0].Message.Content)
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
