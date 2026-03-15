// Package ai 提供 NPC 對話 LLM 呼叫；階段 I 先 stub，後續接 Gemini/GPT。
package ai

import "errors"

// CallAITalk 呼叫 LLM 回覆。playerInput 為玩家輸入；npcBackstory 為 BuildIdentity 組出的 identity；npcMemorySnippets 為 SearchArchival 取回的記憶；styleExamples 為對話池口吻範例。失敗或未實作時回傳 err，由 caller fallback 模板。
func CallAITalk(playerInput, npcBackstory string, npcMemorySnippets, styleExamples []string) (string, error) {
	_ = playerInput
	_ = npcBackstory
	_ = npcMemorySnippets
	_ = styleExamples
	return "", errors.New("not implemented")
}
