// Package db NPC 長期記憶（archival）薄封裝；型別 store.ArchivalEntry 定義在 store 包。
package db

import (
	"sort"
	"strings"
	"time"

	"singularity_world/store"
)

const (
	// ArchivalThrottleWindowSec 同一 NPC 在此時間窗內最多寫入 ArchivalThrottleMax 條，避免單場轟炸。
	ArchivalThrottleWindowSec = 600
	// ArchivalThrottleMax 時間窗內最多寫入條數。
	ArchivalThrottleMax = 3
	// ArchivalMaxPerEntity 單一 NPC 最多保留條數，超過則刪最舊。
	ArchivalMaxPerEntity = 100
)

// CountArchivalSince 回傳該 entity 在 sinceUnix 之後的條目數（用於節流）。
func CountArchivalSince(entityID string, sinceUnix int64) int {
	if store.Default == nil {
		return 0
	}
	entries := store.Default.GetArchivalByEntity(entityID)
	n := 0
	for _, e := range entries {
		if e.CreatedAt >= sinceUnix {
			n++
		}
	}
	return n
}

// InsertArchival 寫入一條長期記憶。若該 NPC 在時間窗內已達 ArchivalThrottleMax 條則略過（節流）。
func InsertArchival(entityID, content, tag string) error {
	if store.Default == nil {
		return nil
	}
	now := time.Now().Unix()
	if CountArchivalSince(entityID, now-ArchivalThrottleWindowSec) >= ArchivalThrottleMax {
		return nil
	}
	entry := store.ArchivalEntry{
		EntityID:  entityID,
		Content:   content,
		Tag:       tag,
		CreatedAt: now,
	}
	return store.Default.AppendArchival(entry)
}

// SearchArchival 依 entity_id 取 topK 條：多關鍵字評分（query 拆詞，命中越多越前），無 query 取最新。回傳 Content 字串 slice。
func SearchArchival(entityID, query string, topK int) []string {
	if store.Default == nil {
		return nil
	}
	entries := store.Default.GetArchivalByEntity(entityID)
	if len(entries) == 0 {
		return nil
	}
	query = strings.TrimSpace(query)
	if query == "" {
		out := make([]string, 0, topK)
		for i := 0; i < len(entries) && len(out) < topK; i++ {
			out = append(out, entries[i].Content)
		}
		return out
	}
	// 拆成關鍵字：空白分開 + 整句當一詞，讓「不付錢」與「付錢」都能帶出含「錢」的記憶
	terms := splitQueryTerms(query)
	type scored struct {
		content string
		score   int
		at      int64
	}
	scoredList := make([]scored, 0, len(entries))
	for _, e := range entries {
		s := 0
		for _, t := range terms {
			if t != "" && strings.Contains(e.Content, t) {
				s++
			}
		}
		scoredList = append(scoredList, scored{e.Content, s, e.CreatedAt})
	}
	// 分數高優先，同分則新的優先
	sort.Slice(scoredList, func(i, j int) bool {
		if scoredList[i].score != scoredList[j].score {
			return scoredList[i].score > scoredList[j].score
		}
		return scoredList[i].at > scoredList[j].at
	})
	out := make([]string, 0, topK)
	for i := 0; i < len(scoredList) && len(out) < topK; i++ {
		if scoredList[i].score > 0 {
			out = append(out, scoredList[i].content)
		}
	}
	if len(out) == 0 {
		for i := 0; i < len(entries) && len(out) < topK; i++ {
			out = append(out, entries[i].Content)
		}
	}
	return out
}

func splitQueryTerms(q string) []string {
	q = strings.TrimSpace(q)
	if q == "" {
		return nil
	}
	var terms []string
	for _, s := range strings.Fields(q) {
		s = strings.TrimSpace(s)
		if s != "" {
			terms = append(terms, s)
		}
	}
	if len(terms) == 0 {
		terms = []string{q}
	}
	return terms
}

// GetNpcSummary 回傳該 NPC 的「與玩家最近印象」；背版用。
func GetNpcSummary(entityID string) string {
	if store.Default == nil {
		return ""
	}
	return store.Default.GetNpcSummary(entityID)
}

// SetNpcSummary 設定該 NPC 的最近印象並持久化（consolidation 時由 server 呼叫）。
func SetNpcSummary(entityID, summary string) error {
	if store.Default == nil {
		return nil
	}
	return store.Default.SetNpcSummary(entityID, summary)
}

// GetNpcNpcConversationSummary 回傳兩 NPC 的最近交談摘要，供下次觸發時帶入 AI context、避免長碰面重複。
func GetNpcNpcConversationSummary(entityIDA, entityIDB string) string {
	if store.Default == nil {
		return ""
	}
	return store.Default.GetNpcNpcSummary(entityIDA, entityIDB)
}

// SetNpcNpcConversationSummary 寫入兩 NPC 的最近交談摘要並持久化（NPC 間對話完成後呼叫）。
func SetNpcNpcConversationSummary(entityIDA, entityIDB, summary string) error {
	if store.Default == nil {
		return nil
	}
	return store.Default.SetNpcNpcSummary(entityIDA, entityIDB, summary)
}
