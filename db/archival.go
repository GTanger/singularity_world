// Package db NPC 長期記憶（archival）薄封裝；型別 store.ArchivalEntry 定義在 store 包。
package db

import (
	"sort"
	"strings"
	"time"
	"unicode/utf8"

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

// RuneLCSSimilarity 以 LCS 正規化相似度：2*LCS/(len(a)+len(b))，供摘要／archival 去重。
func RuneLCSSimilarity(a, b string) float64 {
	ra, rb := []rune(a), []rune(b)
	la, lb := len(ra), len(rb)
	if la == 0 || lb == 0 {
		return 0
	}
	prev := make([]int, lb+1)
	cur := make([]int, lb+1)
	for i := 1; i <= la; i++ {
		cur[0] = 0
		for j := 1; j <= lb; j++ {
			if ra[i-1] == rb[j-1] {
				cur[j] = prev[j-1] + 1
			} else if cur[j-1] > prev[j] {
				cur[j] = cur[j-1]
			} else {
				cur[j] = prev[j]
			}
		}
		prev, cur = cur, prev
	}
	lcs := prev[lb]
	return float64(2*lcs) / float64(la+lb)
}

// stripNpcNpcArchivalLine 從 archival 原文取出對白本體（去掉「與…對話：」前綴）。
func stripNpcNpcArchivalLine(content string) string {
	content = strings.TrimSpace(content)
	if i := strings.Index(content, "對話："); i >= 0 {
		return strings.TrimSpace(content[i+len("對話："):])
	}
	return content
}

// RecentNpcNpcArchivalLinesForEntity 回傳該 entity 最近 n 條 npc_npc 對白（已由新到舊），供對話品質去重評分。
func RecentNpcNpcArchivalLinesForEntity(entityID string, n int) []string {
	if store.Default == nil || n <= 0 {
		return nil
	}
	raw := recentNpcNpcArchivalContents(entityID, n)
	out := make([]string, 0, len(raw))
	for _, c := range raw {
		line := stripNpcNpcArchivalLine(c)
		if line != "" {
			out = append(out, line)
		}
	}
	return out
}

// recentNpcNpcArchivalContents 該 entity 最近 n 條 tag=npc_npc 的 content（已由新到舊）。
func recentNpcNpcArchivalContents(entityID string, n int) []string {
	if store.Default == nil || n <= 0 {
		return nil
	}
	entries := store.Default.GetArchivalByEntity(entityID)
	out := make([]string, 0, n)
	for _, e := range entries {
		if e.Tag != "npc_npc" {
			continue
		}
		out = append(out, e.Content)
		if len(out) >= n {
			break
		}
	}
	return out
}

func shouldSkipNpcNpcArchival(entityID, content string) bool {
	content = strings.TrimSpace(content)
	if content == "" {
		return true
	}
	for _, old := range recentNpcNpcArchivalContents(entityID, 3) {
		if old == content {
			return true
		}
		if RuneLCSSimilarity(old, content) >= 0.8 {
			return true
		}
	}
	return false
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

// InsertNpcNpcDialogueArchival 寫入 tag=npc_npc；若與最近 3 條重複或相似度≥0.8 則略過。
// 回傳 (是否寫入, 是否因去重略過)；節流略過時兩者皆 false。
func InsertNpcNpcDialogueArchival(entityID, content string) (written bool, skippedDuplicate bool) {
	if store.Default == nil {
		return false, false
	}
	content = strings.TrimSpace(content)
	if content == "" {
		return false, false
	}
	if shouldSkipNpcNpcArchival(entityID, content) {
		return false, true
	}
	now := time.Now().Unix()
	if CountArchivalSince(entityID, now-ArchivalThrottleWindowSec) >= ArchivalThrottleMax {
		return false, false
	}
	entry := store.ArchivalEntry{
		EntityID:  entityID,
		Content:   content,
		Tag:       "npc_npc",
		CreatedAt: now,
	}
	if err := store.Default.AppendArchival(entry); err != nil {
		return false, false
	}
	return true, false
}

type archivalRank struct {
	content string
	score   int
	at      int64
}

// rankArchivalForQuery 依 query 拆詞對條目評分（命中越多越前，同分則較新在前）。
func rankArchivalForQuery(entries []store.ArchivalEntry, query string) []archivalRank {
	query = strings.TrimSpace(query)
	if query == "" {
		return nil
	}
	terms := splitQueryTerms(query)
	ranked := make([]archivalRank, 0, len(entries))
	for _, e := range entries {
		s := 0
		for _, t := range terms {
			if t != "" && strings.Contains(e.Content, t) {
				s++
			}
		}
		ranked = append(ranked, archivalRank{e.Content, s, e.CreatedAt})
	}
	sort.Slice(ranked, func(i, j int) bool {
		if ranked[i].score != ranked[j].score {
			return ranked[i].score > ranked[j].score
		}
		return ranked[i].at > ranked[j].at
	})
	return ranked
}

// talkOmitArchivalFallbackExact 玩家句經輕量正規化後若等於其中一項，且記憶檢索零命中時，不採「取最新幾條」fallback，避免無關節錄（如對帳）污染 Talk。
var talkOmitArchivalFallbackExact = map[string]struct{}{
	"你好": {}, "您好": {}, "嗨": {}, "哈囉": {}, "哈喽": {}, "hello": {}, "hi": {},
	"早": {}, "早安": {}, "午安": {}, "晚安": {}, "在嗎": {}, "在不在": {},
	"喂": {}, "欸": {}, "嘿": {}, "謝謝": {}, "谢谢": {}, "感恩": {},
	"再見": {}, "再见": {}, "拜拜": {}, "掰掰": {}, "掰": {},
	"嗯": {}, "嗯嗯": {}, "喔": {}, "哦": {}, "好": {}, "好啊": {}, "好喔": {}, "好的": {},
	"收到": {}, "了解": {}, "恩": {}, "噢": {},
}

// normalizePlayerTalkQueryForGreeting 供寒暄判斷：去首尾空白、ASCII 轉小寫、剝標點與常見句尾語氣字。
func normalizePlayerTalkQueryForGreeting(q string) string {
	q = strings.TrimSpace(q)
	var b strings.Builder
	b.Grow(len(q))
	for _, r := range q {
		if r >= 'A' && r <= 'Z' {
			r += 'a' - 'A'
		}
		b.WriteRune(r)
	}
	q = strings.TrimSpace(b.String())
	for _, ch := range []string{"。", "！", "？", ".", "!", "?", "~", "～", "…"} {
		q = strings.Trim(q, ch)
	}
	for {
		before := q
		for _, suf := range []string{"啊", "呀", "喔", "哦", "吧", "呢", "嘛", "啦", "唄", "哈"} {
			rs := utf8.RuneCountInString(q)
			ss := utf8.RuneCountInString(suf)
			if rs > ss && strings.HasSuffix(q, suf) {
				q = strings.TrimSpace(strings.TrimSuffix(q, suf))
			}
		}
		if q == before {
			break
		}
	}
	return strings.TrimSpace(q)
}

func shouldOmitArchivalFallbackForTalkQuery(query string) bool {
	q := strings.TrimSpace(query)
	if q == "" {
		return true
	}
	if strings.Contains(q, "搭話") {
		return true
	}
	norm := normalizePlayerTalkQueryForGreeting(q)
	if norm == "" {
		return true
	}
	_, ok := talkOmitArchivalFallbackExact[norm]
	return ok
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
	ranked := rankArchivalForQuery(entries, query)
	out := make([]string, 0, topK)
	for _, r := range ranked {
		if r.score > 0 && len(out) < topK {
			out = append(out, r.content)
		}
	}
	if len(out) == 0 {
		for i := 0; i < len(entries) && len(out) < topK; i++ {
			out = append(out, entries[i].Content)
		}
	}
	return out
}

// SearchArchivalForPlayerTalk 專供玩家↔NPC Talk：評分邏輯同 SearchArchival，但若零命中且玩家這句屬寒暄／占位等低意圖，
// 則不 fallback 到「最新幾條」，回傳 nil，避免無關對帳、換班等節錄被硬塞進 LLM。
func SearchArchivalForPlayerTalk(entityID, query string, topK int) []string {
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
	ranked := rankArchivalForQuery(entries, query)
	out := make([]string, 0, topK)
	for _, r := range ranked {
		if r.score > 0 && len(out) < topK {
			out = append(out, r.content)
		}
	}
	if len(out) == 0 && shouldOmitArchivalFallbackForTalkQuery(query) {
		return nil
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
