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

// GetNpcNpcThread 取得兩 NPC 的 thread 狀態；不存在回傳 nil。
func GetNpcNpcThread(entityIDA, entityIDB string) *store.NpcThread {
	if store.Default == nil {
		return nil
	}
	return store.Default.GetNpcThread(entityIDA, entityIDB)
}

// SetNpcNpcThread 寫入兩 NPC 的 thread 狀態。
func SetNpcNpcThread(entityIDA, entityIDB string, t *store.NpcThread) error {
	if store.Default == nil {
		return nil
	}
	return store.Default.SetNpcThread(entityIDA, entityIDB, t)
}

// DeleteNpcNpcThread 刪除兩 NPC 的 thread 狀態。
func DeleteNpcNpcThread(entityIDA, entityIDB string) error {
	if store.Default == nil {
		return nil
	}
	return store.Default.DeleteNpcThread(entityIDA, entityIDB)
}

// GetNpcNpcDyad 取得兩 NPC 的關係狀態；不存在回傳預設值。
func GetNpcNpcDyad(entityIDA, entityIDB string) *store.NpcDyad {
	if store.Default == nil {
		return nil
	}
	return store.Default.GetNpcDyad(entityIDA, entityIDB)
}

// SetNpcNpcDyad 寫入兩 NPC 的關係狀態。
func SetNpcNpcDyad(entityIDA, entityIDB string, d *store.NpcDyad) error {
	if store.Default == nil {
		return nil
	}
	return store.Default.SetNpcDyad(entityIDA, entityIDB, d)
}

// UpsertNpcRumor 寫入/更新一條傳聞（P4）。
func UpsertNpcRumor(r *store.NpcRumor) error {
	if store.Default == nil {
		return nil
	}
	return store.Default.UpsertNpcRumor(r)
}

// TopNpcRumors 取 room/zone 最相關 topK 傳聞（P4）。
func TopNpcRumors(roomID, zone string, nowUnix int64, topK int) []store.NpcRumor {
	if store.Default == nil {
		return nil
	}
	return store.Default.TopNpcRumors(roomID, zone, nowUnix, topK)
}

// DecayNpcRumors 衰減並移除過期傳聞（P4）。
func DecayNpcRumors(nowUnix int64) error {
	if store.Default == nil {
		return nil
	}
	return store.Default.DecayNpcRumors(nowUnix)
}

// MarkRumorUsedByText 標記傳聞被引用（P8）。
func MarkRumorUsedByText(text string, nowUnix int64) error {
	if store.Default == nil {
		return nil
	}
	return store.Default.MarkRumorUsedByText(text, nowUnix)
}

// PenalizeRumorByText 對衝突傳聞做降權與短期封鎖（P9）。
func PenalizeRumorByText(text string, nowUnix int64, reason string) error {
	if store.Default == nil {
		return nil
	}
	return store.Default.PenalizeRumorByText(text, nowUnix, reason)
}

// DebugNpcRumors 回傳 room/zone 相關傳聞（含封鎖中）供 debug 使用（P10）。
func DebugNpcRumors(roomID, zone string, nowUnix int64, limit int) []store.NpcRumor {
	if store.Default == nil {
		return nil
	}
	return store.Default.DebugNpcRumors(roomID, zone, nowUnix, limit)
}

// ResetNpcRumorSignals 清空傳聞的動態訊號（P11）。
func ResetNpcRumorSignals() error {
	if store.Default == nil {
		return nil
	}
	return store.Default.ResetNpcRumorSignals()
}

// BuildNpcRumorDigest 以目前傳聞池建立摘要（P6）。
func BuildNpcRumorDigest(nowUnix int64) error {
	if store.Default == nil {
		return nil
	}
	return store.Default.BuildNpcRumorDigest(nowUnix)
}

// GetNpcRumorDigest 取得最近一次傳聞摘要（P6）。
func GetNpcRumorDigest() *store.NpcRumorDigest {
	if store.Default == nil {
		return nil
	}
	return store.Default.GetNpcRumorDigest()
}
