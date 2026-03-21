// 程式入口：啟動 HTTP 伺服器、WebSocket、靜態檔與 DB，對齊第一版可做清單 §1.1。
package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"log"
	"math/rand"
	"net/http"
	"net/http/httputil"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"singularity_world/ai"
	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/economy"
	"singularity_world/entity"
	"singularity_world/game"
	"singularity_world/server"
	"singularity_world/store"

	"github.com/gorilla/websocket"
)

// brainArrivalNarrative 腦驅動 NPC 抵達意圖目標時發送的敘事。
func brainArrivalNarrative(npcName string, intent db.IntentType) string {
	switch intent {
	case db.IntentBeg:
		return "【" + npcName + "】在此地向路人乞討。"
	case db.IntentGather:
		return "【" + npcName + "】開始在附近採集。"
	case db.IntentSeekJob:
		return "【" + npcName + "】前來打聽是否有活可做。"
	case db.IntentTrade:
		return "【" + npcName + "】在此地擺開貨物。"
	case db.IntentWork:
		return "【" + npcName + "】回到崗位，繼續工作。"
	case db.IntentSocialize:
		return "【" + npcName + "】找了個角落坐下，悠閒地打量著來往的人。"
	default:
		return ""
	}
}

// maxAssignmentsPerVenue 求職撮合時，單一場所最多指派數（無 max_staff 時用此常數）。
const maxAssignmentsPerVenue = 2

// truncRune 截斷為最多 n 個 rune，超出補「…」。
func truncRune(s string, n int) string {
	r := []rune(s)
	if len(r) <= n {
		return s
	}
	return string(r[:n]) + "…"
}

// stripNpcLineSpeakerPrefix 若台詞開頭為「名字:」或「名字：」，移除之，避免顯示重複（【A】說：「A: 台詞」→「【A】說：「台詞」」）。
func stripNpcLineSpeakerPrefix(line string, names ...string) string {
	line = strings.TrimSpace(line)
	for _, name := range names {
		if name == "" {
			continue
		}
		if strings.HasPrefix(line, name+":") {
			line = strings.TrimSpace(line[len(name)+1:])
			break
		}
		if strings.HasPrefix(line, name+"：") {
			line = strings.TrimSpace(strings.TrimPrefix(line, name+"："))
			break
		}
	}
	return line
}

// sanitizeNPCDialogueLine 清洗模型殘留輸出：移除 think/code-fence/自我修正片段與外層重複引號。
func sanitizeNPCDialogueLine(line string) string {
	s := strings.TrimSpace(line)
	if s == "" {
		return s
	}
	low := strings.ToLower(s)
	// 若整句含有思考或 code fence 痕跡，直接丟棄該句，避免污染房間敘事。
	if strings.Contains(low, "</think>") || strings.Contains(low, "<think>") || strings.Contains(s, "```") {
		return ""
	}
	// 清除常見模型自我修正段落。
	if strings.Contains(s, "自我修正") || strings.Contains(s, "根據要求") {
		return ""
	}
	// NPC↔NPC 或對話模型偶發的助理／後設口吻
	if strings.Contains(s, "結束對話") || strings.Contains(s, "若需繼續") || strings.Contains(s, "換行提問") ||
		strings.Contains(s, "情境需求") || strings.Contains(s, "更簡短") || strings.Contains(s, "請再說") ||
		strings.Contains(s, "（若想") || strings.Contains(s, "（若需") {
		return ""
	}
	// 去掉外層全形引號（「...」）避免再包一層成 「「...」」。
	for len(s) >= 2 && strings.HasPrefix(s, "「") && strings.HasSuffix(s, "」") {
		s = strings.TrimSpace(s[3 : len(s)-3])
	}
	// 清掉末尾多餘引號（偶發多一個 」）。
	for strings.HasSuffix(s, "」」") {
		s = strings.TrimSuffix(s, "」")
	}
	return strings.TrimSpace(s)
}

type roomEvent struct {
	At      int64
	Kind    string
	Subject string
	Detail  string
}

var roomEventWindow = struct {
	mu    sync.RWMutex
	byRID map[string][]roomEvent
}{
	byRID: make(map[string][]roomEvent),
}

var npcPairLastTalk = struct {
	mu sync.RWMutex
	at map[string]int64
}{
	at: make(map[string]int64),
}

type lastNpcSocialChoice struct {
	At                   int64    `json:"at"`
	RoomID               string   `json:"room_id"`
	AID                  string   `json:"a_id"`
	BID                  string   `json:"b_id"`
	ScoreTotal           int      `json:"score_total"`
	TopicHint            string   `json:"topic_hint"`
	TopicID              string   `json:"topic_id,omitempty"`
	TopicReason          string   `json:"topic_reason,omitempty"`
	AnchorsUsed          []string `json:"anchors_used,omitempty"`
	AnchorsWritten       []string `json:"anchors_written,omitempty"`
	AnchorConflict       bool     `json:"anchor_conflict"`
	AnchorConflictReason string   `json:"anchor_conflict_reason,omitempty"`
}

var npcSocialLastChoice = struct {
	mu     sync.RWMutex
	choice *lastNpcSocialChoice
}{}

var npcSocialStats = struct {
	mu      sync.RWMutex
	Counter map[string]int64
}{
	Counter: map[string]int64{
		"attempt":                 0,
		"success":                 0,
		"fail_no_model":           0,
		"fail_recent_player_talk": 0,
		"fail_not_enough_npc":     0,
		"fail_no_pair":            0,
		"fail_llm_empty":          0,
		"fail_quality_gate":       0,
		"fail_anchor_conflict":    0,
	},
}

func incNpcSocialStat(key string) {
	npcSocialStats.mu.Lock()
	npcSocialStats.Counter[key]++
	npcSocialStats.mu.Unlock()
}

func canonicalNpcPairKey(idA, idB string) string {
	if idA > idB {
		return idB + "|" + idA
	}
	return idA + "|" + idB
}

func pushRoomEvent(roomID, kind, subject, detail string) {
	if roomID == "" {
		return
	}
	now := time.Now().Unix()
	roomEventWindow.mu.Lock()
	defer roomEventWindow.mu.Unlock()
	list := roomEventWindow.byRID[roomID]
	list = append(list, roomEvent{At: now, Kind: kind, Subject: subject, Detail: detail})
	const maxEvents = 5
	const ttlSec int64 = 120
	cutoff := now - ttlSec
	filtered := make([]roomEvent, 0, len(list))
	for _, e := range list {
		if e.At >= cutoff {
			filtered = append(filtered, e)
		}
	}
	if len(filtered) > maxEvents {
		filtered = filtered[len(filtered)-maxEvents:]
	}
	roomEventWindow.byRID[roomID] = filtered
	pushRumorFromRoomEvent(nil, roomID, kind, subject, detail, now)
}

func pushRumorFromRoomEvent(database *sql.DB, roomID, kind, subject, detail string, now int64) {
	if roomID == "" || strings.TrimSpace(detail) == "" {
		return
	}
	zone := ""
	if room, _ := db.GetRoom(database, roomID); room != nil {
		zone = room.Zone
	}
	text := strings.TrimSpace(subject + "：" + detail)
	if text == "" {
		return
	}
	id := fmt.Sprintf("evt|%s|%s|%s", roomID, kind, truncRune(text, 24))
	_ = db.UpsertNpcRumor(&store.NpcRumor{
		ID:          id,
		Text:        truncRune(text, 28),
		RoomID:      roomID,
		Zone:        zone,
		Source:      "room_event",
		SourceScore: 2,
		Weight:      1,
		UpdatedAt:   now,
		ExpiresAt:   now + 1800,
	})
}

func recentRoomEvents(roomID string, maxCount int) []string {
	if roomID == "" || maxCount <= 0 {
		return nil
	}
	roomEventWindow.mu.RLock()
	list := roomEventWindow.byRID[roomID]
	roomEventWindow.mu.RUnlock()
	if len(list) == 0 {
		return nil
	}
	if len(list) > maxCount {
		list = list[len(list)-maxCount:]
	}
	out := make([]string, 0, len(list))
	for _, e := range list {
		t := strings.TrimSpace(e.Subject)
		if e.Detail != "" {
			if t == "" {
				t = e.Detail
			} else {
				t = t + "，" + e.Detail
			}
		}
		if t != "" {
			out = append(out, t)
		}
	}
	return out
}

func getPairLastTalk(idA, idB string) int64 {
	key := canonicalNpcPairKey(idA, idB)
	npcPairLastTalk.mu.RLock()
	defer npcPairLastTalk.mu.RUnlock()
	return npcPairLastTalk.at[key]
}

func setPairLastTalk(idA, idB string, at int64) {
	key := canonicalNpcPairKey(idA, idB)
	npcPairLastTalk.mu.Lock()
	npcPairLastTalk.at[key] = at
	npcPairLastTalk.mu.Unlock()
}

type npcSocialPairDebug struct {
	AID          string                `json:"a_id"`
	BID          string                `json:"b_id"`
	LastTalkAt   int64                 `json:"last_talk_at"`
	Thread       *store.NpcThread      `json:"thread,omitempty"`
	Dyad         *store.NpcDyad        `json:"dyad,omitempty"`
	ScoreTotal   int                   `json:"score_total"`
	ScoreDetail  []string              `json:"score_detail,omitempty"`
	TopicWeights []db.TopicWeightDebug `json:"topic_weights,omitempty"`
}

type npcSocialDebugResp struct {
	RoomID       string               `json:"room_id"`
	RoomName     string               `json:"room_name"`
	Events       []string             `json:"events"`
	Pairs        []npcSocialPairDebug `json:"pairs"`
	NpcIDs       []string             `json:"npc_ids"`
	ServerUnix   int64                `json:"server_unix"`
	LastChoice   *lastNpcSocialChoice `json:"last_choice,omitempty"`
	Stats        map[string]int64     `json:"stats,omitempty"`
	Rumors       []string             `json:"rumors,omitempty"`
	RumorDigest  string               `json:"rumor_digest,omitempty"`
	RumorDetails []npcRumorDebugItem  `json:"rumor_details,omitempty"`
}

type npcRumorDebugItem struct {
	Text              string `json:"text"`
	Source            string `json:"source,omitempty"`
	Weight            int    `json:"weight"`
	SourceScore       int    `json:"source_score"`
	MentionCount      int    `json:"mention_count"`
	LastUsedAt        int64  `json:"last_used_at,omitempty"`
	BlockedUntil      int64  `json:"blocked_until,omitempty"`
	PenaltyCount      int    `json:"penalty_count,omitempty"`
	LastPenaltyAt     int64  `json:"last_penalty_at,omitempty"`
	LastPenaltyReason string `json:"last_penalty_reason,omitempty"`
	Status            string `json:"status"`
}

func handleNpcSocialDebug(database *sql.DB, w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, `{"error":"method not allowed"}`, http.StatusMethodNotAllowed)
		return
	}
	roomID := strings.TrimSpace(r.URL.Query().Get("room_id"))
	if roomID == "" {
		http.Error(w, `{"error":"need room_id"}`, http.StatusBadRequest)
		return
	}
	roomName, _ := db.GetRoomName(database, roomID)
	entities, _ := db.GetEntitiesInRoom(database, roomID)
	var npcs []*entity.Character
	for _, e := range entities {
		if e != nil && e.Kind == "npc" {
			npcs = append(npcs, e)
		}
	}
	resp := npcSocialDebugResp{
		RoomID:     roomID,
		RoomName:   roomName,
		Events:     recentRoomEvents(roomID, 5),
		ServerUnix: time.Now().Unix(),
	}
	npcSocialLastChoice.mu.RLock()
	if npcSocialLastChoice.choice != nil {
		cp := *npcSocialLastChoice.choice
		resp.LastChoice = &cp
	}
	npcSocialLastChoice.mu.RUnlock()
	npcSocialStats.mu.RLock()
	resp.Stats = make(map[string]int64, len(npcSocialStats.Counter))
	for k, v := range npcSocialStats.Counter {
		resp.Stats[k] = v
	}
	npcSocialStats.mu.RUnlock()
	nowUnix := time.Now().Unix()
	hour := 12
	if cfg := config.DefaultServer(); cfg.GameTimeEpochUnix != 0 {
		_, hour, _, _ = game.GameTimeNow(nowUnix, cfg.GameTimeEpochUnix, cfg.GameTimeScale)
	}
	mask := topicMaskForRoom(database, roomID, hour)
	if room, _ := db.GetRoom(database, roomID); room != nil {
		for _, r := range db.TopNpcRumors(roomID, room.Zone, nowUnix, 3) {
			resp.Rumors = append(resp.Rumors, r.Text)
		}
		for _, rr := range db.DebugNpcRumors(roomID, room.Zone, nowUnix, 12) {
			status := "active"
			if rr.BlockedUntil > nowUnix {
				status = "blocked"
			}
			resp.RumorDetails = append(resp.RumorDetails, npcRumorDebugItem{
				Text:              rr.Text,
				Source:            rr.Source,
				Weight:            rr.Weight,
				SourceScore:       rr.SourceScore,
				MentionCount:      rr.MentionCount,
				LastUsedAt:        rr.LastUsedAt,
				BlockedUntil:      rr.BlockedUntil,
				PenaltyCount:      rr.PenaltyCount,
				LastPenaltyAt:     rr.LastPenaltyAt,
				LastPenaltyReason: rr.LastPenaltyReason,
				Status:            status,
			})
		}
	}
	if d := db.GetNpcRumorDigest(); d != nil {
		resp.RumorDigest = d.Text
	}
	for _, n := range npcs {
		resp.NpcIDs = append(resp.NpcIDs, n.ID)
	}
	for i := 0; i < len(npcs); i++ {
		for j := i + 1; j < len(npcs); j++ {
			a, b := npcs[i], npcs[j]
			if a == nil || b == nil {
				continue
			}
			thread := db.GetNpcNpcThread(a.ID, b.ID)
			dyad := db.GetNpcNpcDyad(a.ID, b.ID)
			score := 0
			detail := make([]string, 0, 8)
			noise := rand.Intn(4)
			score += noise
			detail = append(detail, fmt.Sprintf("base_noise=%d", noise))
			if thread != nil && thread.Phase != "cooling" {
				score += 100
				detail = append(detail, "active_thread=+100")
			}
			if dyad != nil {
				v := dyad.Familiarity / 20
				score += v
				detail = append(detail, fmt.Sprintf("familiarity=%d/20 => +%d", dyad.Familiarity, v))
				if dyad.Sentiment <= -30 {
					score -= 2
					detail = append(detail, "sentiment<=-30 => -2")
				}
				if hasTag(dyad.Tags, "同職場") {
					score += 2
					detail = append(detail, "tag:同職場 => +2")
				}
			}
			if npcPairSameVenue(database, a.ID, b.ID) {
				score += 3
				detail = append(detail, "sameVenue => +3")
			}
			if last := getPairLastTalk(a.ID, b.ID); last > 0 && nowUnix-last < 120 {
				score -= 20
				detail = append(detail, "talked<120s => -20")
			}
			fam, sen := 0, 0
			var tags []string
			if dyad != nil {
				fam, sen, tags = dyad.Familiarity, dyad.Sentiment, dyad.Tags
			}
			resp.Pairs = append(resp.Pairs, npcSocialPairDebug{
				AID:          a.ID,
				BID:          b.ID,
				LastTalkAt:   getPairLastTalk(a.ID, b.ID),
				Thread:       thread,
				Dyad:         dyad,
				ScoreTotal:   score,
				ScoreDetail:  detail,
				TopicWeights: db.DebugTopicWeightsForPair(mask, "", fam, sen, tags),
			})
		}
	}
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	_ = json.NewEncoder(w).Encode(resp)
}

func handleNpcSocialDebugReset(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, `{"error":"method not allowed"}`, http.StatusMethodNotAllowed)
		return
	}
	npcSocialStats.mu.Lock()
	for k := range npcSocialStats.Counter {
		npcSocialStats.Counter[k] = 0
	}
	npcSocialStats.mu.Unlock()

	npcSocialLastChoice.mu.Lock()
	npcSocialLastChoice.choice = nil
	npcSocialLastChoice.mu.Unlock()

	resetRumors := r.URL.Query().Get("reset_rumors")
	rumorsReset := false
	if resetRumors == "1" || strings.EqualFold(resetRumors, "true") || strings.EqualFold(resetRumors, "yes") {
		_ = db.ResetNpcRumorSignals()
		rumorsReset = true
	}

	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	_ = json.NewEncoder(w).Encode(map[string]any{
		"ok":           true,
		"message":      "npc social debug stats reset",
		"rumors_reset": rumorsReset,
	})
}

func npcPairSameVenue(database *sql.DB, idA, idB string) bool {
	asA, _ := db.GetAssignmentsForEntity(database, idA)
	asB, _ := db.GetAssignmentsForEntity(database, idB)
	if len(asA) == 0 || len(asB) == 0 {
		return false
	}
	vid := make(map[string]bool, len(asA))
	for _, a := range asA {
		if a.VenueID != "" {
			vid[a.VenueID] = true
		}
	}
	for _, b := range asB {
		if b.VenueID != "" && vid[b.VenueID] {
			return true
		}
	}
	return false
}

func topicMaskForRoom(database *sql.DB, roomID string, hour int) db.NpcTopicMask {
	venueIDs, _ := db.GetVenueIDsForRoom(database, roomID)
	return db.NpcTopicMask{
		IsWorkVenue:  len(venueIDs) > 0,
		IsNightTime:  hour < 5 || hour >= 21,
		HasRoomEvent: len(recentRoomEvents(roomID, 1)) > 0,
	}
}

func dyadRelationHint(d *store.NpcDyad) string {
	if d == nil {
		return "兩人剛認識，語氣保守、客套"
	}
	if d.Sentiment <= -40 {
		return "兩人關係緊張，語氣克制但帶刺，避免過分友善"
	}
	if d.Sentiment >= 40 {
		return "兩人互相欣賞，語氣可更親切自然"
	}
	switch {
	case d.Familiarity >= 70:
		return "兩人是熟人，語氣自然、可簡短接話"
	case d.Familiarity >= 35:
		return "兩人算點頭之交，語氣中性偏熟"
	default:
		return "兩人不太熟，先試探、語氣留距離"
	}
}

func hasTag(tags []string, t string) bool {
	for _, v := range tags {
		if v == t {
			return true
		}
	}
	return false
}

func addTag(tags []string, t string) []string {
	if t == "" || hasTag(tags, t) {
		return tags
	}
	return append(tags, t)
}

func removeTag(tags []string, t string) []string {
	if t == "" || len(tags) == 0 {
		return tags
	}
	out := make([]string, 0, len(tags))
	for _, v := range tags {
		if v != t {
			out = append(out, v)
		}
	}
	return out
}

func sentimentDeltaFromDialogue(topicHint, lineA, lineB string, familiarity int) int {
	delta := 0
	if strings.Contains(topicHint, "交班") || strings.Contains(topicHint, "閒聊") {
		delta += 1
	}
	if strings.Contains(topicHint, "打聽") && familiarity < 20 {
		delta -= 1
	}
	joined := lineA + " " + lineB
	negWords := []string{"不必", "少管", "閉嘴", "煩", "緊張", "滾", "別再"}
	for _, w := range negWords {
		if strings.Contains(joined, w) {
			delta -= 2
			break
		}
	}
	posWords := []string{"謝謝", "多謝", "辛苦", "麻煩你", "幫我", "好啊"}
	for _, w := range posWords {
		if strings.Contains(joined, w) {
			delta += 1
			break
		}
	}
	return delta
}

func qualityGateNpcLine(line string, maxRunes int) (bool, string) {
	if maxRunes <= 0 {
		maxRunes = 52
	}
	s := strings.TrimSpace(line)
	if s == "" {
		return false, "empty"
	}
	if len([]rune(s)) > maxRunes {
		return false, "too_long"
	}
	bad := []string{"結束對話", "若需繼續", "情境需求", "換行提問", "```", "作為AI", "作為 AI"}
	for _, b := range bad {
		if strings.Contains(s, b) {
			return false, "meta_text"
		}
	}
	if strings.Count(s, "「") > 2 || strings.Count(s, "」") > 2 {
		return false, "nested_quotes"
	}
	if hasLongRepeat(s) {
		return false, "repetition"
	}
	return true, ""
}

// dedupeSocialContextLines 去掉完全重複的情境列，保留先出現者，減少 prompt 噪音。
func dedupeSocialContextLines(lines []string) []string {
	seen := make(map[string]bool, len(lines))
	out := make([]string, 0, len(lines))
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		key := strings.ToLower(line)
		if seen[key] {
			continue
		}
		seen[key] = true
		out = append(out, line)
	}
	return out
}

func hasLongRepeat(s string) bool {
	r := []rune(s)
	if len(r) < 8 {
		return false
	}
	for i := 0; i+3 < len(r); i++ {
		ch := r[i]
		if ch == ' ' {
			continue
		}
		j := i + 1
		for j < len(r) && r[j] == ch {
			j++
		}
		if j-i >= 4 {
			return true
		}
	}
	return false
}

func mergeThreadAnchors(old []string, fresh []string) []string {
	seen := map[string]bool{}
	out := make([]string, 0, 4)
	for _, v := range old {
		v = strings.TrimSpace(v)
		if v == "" || seen[v] {
			continue
		}
		seen[v] = true
		out = append(out, v)
	}
	for _, v := range fresh {
		v = strings.TrimSpace(v)
		if v == "" || seen[v] {
			continue
		}
		seen[v] = true
		out = append(out, v)
	}
	if len(out) > 4 {
		out = out[len(out)-4:]
	}
	return out
}

func anchorConsistencyCheck(existing []string, lineA, lineB string) (bool, string) {
	if len(existing) == 0 {
		return true, ""
	}
	joined := strings.TrimSpace(lineA + " " + lineB)
	if joined == "" {
		return true, ""
	}
	// 輕量規則：若既有 anchor 含關鍵詞，而新句明確否認，視為衝突
	negators := []string{"沒有", "從未", "不曾", "不是", "不找", "別提", "別再提"}
	for _, a := range existing {
		a = strings.TrimSpace(a)
		if a == "" {
			continue
		}
		anchorKeywords := []string{}
		for _, tok := range []string{"葉卅", "茶館", "交班", "東街", "浮生", "向陽", "飛霜", "梧桐"} {
			if strings.Contains(a, tok) {
				anchorKeywords = append(anchorKeywords, tok)
			}
		}
		if len(anchorKeywords) == 0 {
			continue
		}
		for _, k := range anchorKeywords {
			if !strings.Contains(joined, k) {
				continue
			}
			for _, n := range negators {
				if strings.Contains(joined, n+k) || strings.Contains(joined, k+n) || strings.Contains(joined, n) {
					return false, "anchor keyword negated: " + k
				}
			}
		}
	}
	return true, ""
}

// tryTriggerNpcNpcInRoom 在指定房嘗試觸發一次 NPC 間 AI 對話；topicHint 可為「交班」等或空。成功則寫入記憶並回傳 true。
func tryTriggerNpcNpcInRoom(database *sql.DB, sessionStore *server.SessionStore, cfg config.Server, roomID, topicHint string) bool {
	incNpcSocialStat("attempt")
	if cfg.OllamaBaseURL == "" || cfg.OllamaModel == "" {
		incNpcSocialStat("fail_no_model")
		return false
	}
	if sessionStore.RoomHasPlayerWithRecentTalk(database, roomID, 60*time.Second) {
		incNpcSocialStat("fail_recent_player_talk")
		return false
	}
	entities, _ := db.GetEntitiesInRoom(database, roomID)
	var npcs []*entity.Character
	for _, e := range entities {
		if e == nil || e.Kind != "npc" {
			continue
		}
		// 測試用 NPC 不參與同房 AI 閒聊，避免 Log 出現【試話】
		if e.ID == "試話" || strings.TrimSpace(e.DisplayTitle) == "試話" {
			continue
		}
		npcs = append(npcs, e)
	}
	if len(npcs) < 2 {
		incNpcSocialStat("fail_not_enough_npc")
		return false
	}
	nowUnix := time.Now().Unix()
	type pairPick struct {
		A      *entity.Character
		B      *entity.Character
		Score  int
		Thread *store.NpcThread
	}
	best := pairPick{Score: -1 << 30}
	for i := 0; i < len(npcs); i++ {
		for j := i + 1; j < len(npcs); j++ {
			A, B := npcs[i], npcs[j]
			if A == nil || B == nil || A.ID == "" || B.ID == "" {
				continue
			}
			thread := db.GetNpcNpcThread(A.ID, B.ID)
			if thread != nil && thread.Phase != "cooling" && thread.TurnCount > 0 && nowUnix-thread.UpdatedAt > 90 {
				thread.Phase = "cooling"
				thread.CooldownUntil = nowUnix + 300
				thread.UpdatedAt = nowUnix
				_ = db.SetNpcNpcThread(A.ID, B.ID, thread)
			}
			if thread != nil && thread.Phase == "cooling" && thread.CooldownUntil > nowUnix {
				continue // 冷卻中，不選
			}
			if thread != nil && thread.Phase == "cooling" && thread.CooldownUntil > 0 && thread.CooldownUntil <= nowUnix {
				_ = db.DeleteNpcNpcThread(A.ID, B.ID)
				thread = nil
			}
			score := rand.Intn(4) // 0~3 基礎噪音，避免永遠同組
			if thread != nil && thread.Phase != "cooling" {
				score += 100 // 延續 thread 絕對優先
			}
			if dyad := db.GetNpcNpcDyad(A.ID, B.ID); dyad != nil {
				score += dyad.Familiarity / 20 // 0~5
				if dyad.Sentiment <= -30 {
					score -= 2 // 關係過差時降低同組機率，避免過度互罵洗頻
				}
				if hasTag(dyad.Tags, "同職場") {
					score += 2
				}
			}
			if npcPairSameVenue(database, A.ID, B.ID) {
				score += 3
			}
			if last := getPairLastTalk(A.ID, B.ID); last > 0 && nowUnix-last < 120 {
				score -= 20
			}
			if score > best.Score {
				best = pairPick{A: A, B: B, Score: score, Thread: thread}
			}
		}
	}
	if best.A == nil || best.B == nil {
		incNpcSocialStat("fail_no_pair")
		return false
	}
	A, B := best.A, best.B
	nameA, nameB := A.DisplayTitle, B.DisplayTitle
	if nameA == "" {
		nameA = A.ID
	}
	if nameB == "" {
		nameB = B.ID
	}
	roomName, _ := db.GetRoomName(database, roomID)
	hour := 12
	if cfg.GameTimeEpochUnix != 0 {
		_, hour, _, _ = game.GameTimeNow(time.Now().Unix(), cfg.GameTimeEpochUnix, cfg.GameTimeScale)
	}
	timeLabel := "此時"
	if hour >= 5 && hour < 10 {
		timeLabel = "清晨"
	} else if hour >= 10 && hour < 14 {
		timeLabel = "正午"
	} else if hour >= 14 && hour < 18 {
		timeLabel = "傍晚"
	} else {
		timeLabel = "夜裡"
	}
	backstoryA := db.BuildIdentity(database, A.ID)
	backstoryB := db.BuildIdentity(database, B.ID)
	npcNpcMemory := db.GetNpcNpcConversationSummary(A.ID, B.ID)
	dyad := db.GetNpcNpcDyad(A.ID, B.ID)
	topicSource := "input_or_thread"
	chosenTopicID := ""
	if topicHint == "" && best.Thread != nil && best.Thread.TopicType != "" {
		topicHint = best.Thread.TopicType
		topicSource = "thread_topic"
	}
	if topicHint == "" {
		mask := topicMaskForRoom(database, roomID, hour)
		fam, sen := 0, 0
		var tags []string
		if dyad != nil {
			fam = dyad.Familiarity
			sen = dyad.Sentiment
			tags = dyad.Tags
		}
		if t := db.PickRandomNpcNpcTopicForPair(mask, "", fam, sen, tags); t != nil {
			topicHint = t.Hint
			chosenTopicID = t.ID
			topicSource = "weighted_mask_pick"
		}
	}
	if chosenTopicID == "" && topicHint != "" {
		if t := db.GetNpcNpcTopicByID("交班"); t != nil && t.Hint == topicHint {
			chosenTopicID = t.ID
		}
	}
	playerInRoom := sessionStore.RoomHasPlayer(database, roomID)
	// 玩家在場時略偏「短閒聊／見聞」，方便路人聽到鎮上動靜而不搶長戲
	if playerInRoom && topicSource == "weighted_mask_pick" && rand.Intn(100) < 40 {
		if t := db.GetNpcNpcTopicByID("閒聊"); t != nil {
			topicHint = t.Hint + "（路人在旁，兩句務必極短。）"
			chosenTopicID = "閒聊"
			topicSource = "player_room_gossip_bias"
		}
	}
	recentEvents := recentRoomEvents(roomID, 3)
	roomZone := ""
	roomDesc := ""
	rumorTopK := 2
	if playerInRoom {
		rumorTopK = 3
	}
	roomTagsJoined := ""
	if room, _ := db.GetRoom(database, roomID); room != nil {
		roomZone = room.Zone
		roomDesc = strings.TrimSpace(room.Description)
		if len(room.Tags) > 0 {
			roomTagsJoined = strings.Join(room.Tags, "、")
		}
		for _, rr := range db.TopNpcRumors(roomID, room.Zone, nowUnix, rumorTopK) {
			recentEvents = append(recentEvents, "傳聞："+rr.Text)
			_ = db.MarkRumorUsedByText(rr.Text, nowUnix)
		}
	}
	if playerInRoom {
		recentEvents = append([]string{"【現場】附近有路人在旁，兩人說話要短、低聲，像擦肩閒聊。"}, recentEvents...)
	}
	if d := db.GetNpcRumorDigest(); d != nil && strings.TrimSpace(d.Text) != "" {
		recentEvents = append(recentEvents, d.Text)
	}
	anchorsUsed := []string{}
	if best.Thread != nil && len(best.Thread.Anchors) > 0 {
		anchorsUsed = append(anchorsUsed, best.Thread.Anchors...)
		recentEvents = append(recentEvents, "已知："+strings.Join(best.Thread.Anchors, "；"))
	}
	if chosenTopicID != "" {
		if top := db.GetNpcNpcTopicByID(chosenTopicID); top != nil && len(top.Lines) > 0 {
			seed := strings.TrimSpace(top.Lines[rand.Intn(len(top.Lines))])
			if seed != "" {
				recentEvents = append(recentEvents, "口吻種子（勿照抄，僅參考語氣長度）："+seed)
			}
		}
	}
	if echo := sessionStore.PlayerTalkEchoForNpcSocial(database, roomID); echo != "" {
		recentEvents = append(recentEvents, "【餘音】稍早此處有路人搭過話，大意："+echo+"（兩名 NPC 可側面低聲帶過一句，勿整句複誦、勿像轉述公文）")
	}
	recentEvents = dedupeSocialContextLines(recentEvents)
	maxRunes := cfg.NpcNpcQualityMaxRunes
	lineA, lineB, anchors, err := ai.CallAITalkNPCToNPC(cfg.OllamaBaseURL, cfg.OllamaModel, nameA, nameB, backstoryA, backstoryB, roomName, timeLabel, recentEvents, dyadRelationHint(dyad), npcNpcMemory, topicHint, roomDesc, roomTagsJoined, playerInRoom)
	if err != nil || lineA == "" || lineB == "" {
		incNpcSocialStat("fail_llm_empty")
		return false
	}
	// 若模型輸出「名字: 台詞」，移除前綴以免顯示成【萬雅芬】說：「萬雅芬: 司空偉啊...」
	lineA = stripNpcLineSpeakerPrefix(lineA, nameA, nameB)
	lineB = stripNpcLineSpeakerPrefix(lineB, nameB, nameA)
	lineA = sanitizeNPCDialogueLine(lineA)
	lineB = sanitizeNPCDialogueLine(lineB)
	okA, reasonA := qualityGateNpcLine(lineA, maxRunes)
	okB, reasonB := qualityGateNpcLine(lineB, maxRunes)
	if lineA == "" || lineB == "" || !okA || !okB {
		incNpcSocialStat("fail_quality_gate")
		if !okA && reasonA != "" {
			incNpcSocialStat("fail_quality_" + reasonA)
		}
		if !okB && reasonB != "" {
			incNpcSocialStat("fail_quality_" + reasonB)
		}
		return false
	}
	conflictOK, conflictReason := anchorConsistencyCheck(anchorsUsed, lineA, lineB)
	if !conflictOK {
		// P9：反事實懲罰，衝突錨點對應傳聞降權並短期封鎖
		for _, a := range anchorsUsed {
			_ = db.PenalizeRumorByText(a, nowUnix, conflictReason)
		}
		// 衝突時不讓該輪進世界，避免 thread 自打臉
		npcSocialLastChoice.mu.Lock()
		npcSocialLastChoice.choice = &lastNpcSocialChoice{
			At:                   nowUnix,
			RoomID:               roomID,
			AID:                  A.ID,
			BID:                  B.ID,
			ScoreTotal:           best.Score,
			TopicHint:            topicHint,
			TopicID:              chosenTopicID,
			TopicReason:          topicSource,
			AnchorsUsed:          anchorsUsed,
			AnchorConflict:       true,
			AnchorConflictReason: conflictReason,
		}
		npcSocialLastChoice.mu.Unlock()
		incNpcSocialStat("fail_anchor_conflict")
		return false
	}
	summary := "在" + roomName + "：" + nameA + "說「" + truncRune(lineA, 25) + "」" + nameB + "回「" + truncRune(lineB, 25) + "」"
	_ = db.SetNpcNpcConversationSummary(A.ID, B.ID, summary)
	_ = db.InsertArchival(A.ID, "與"+nameB+"對話："+truncRune(lineA, 40), "npc_npc")
	_ = db.InsertArchival(B.ID, "與"+nameA+"對話："+truncRune(lineB, 40), "npc_npc")
	// 更新 thread（P1：最多 3 輪，超過或久未互動進 cooling）
	thread := best.Thread
	if thread == nil {
		thread = &store.NpcThread{
			TopicType: topicHint,
			Phase:     "opening",
		}
	}
	thread.TurnCount++
	thread.UpdatedAt = nowUnix
	if thread.TopicType == "" {
		thread.TopicType = topicHint
	}
	if thread.TurnCount >= 1 {
		thread.Phase = "elaborate"
	}
	if thread.TurnCount >= 3 {
		thread.Phase = "cooling"
		thread.CooldownUntil = nowUnix + 300
	}
	thread.Anchors = mergeThreadAnchors(thread.Anchors, anchors)
	_ = db.SetNpcNpcThread(A.ID, B.ID, thread)
	// P4：把可用錨點寫入傳聞池（短生命週期，避免無限汙染）
	for _, a := range anchors {
		a = strings.TrimSpace(a)
		if a == "" {
			continue
		}
		rid := fmt.Sprintf("%s|%s|%s", roomID, roomZone, a)
		_ = db.UpsertNpcRumor(&store.NpcRumor{
			ID:          rid,
			Text:        a,
			RoomID:      roomID,
			Zone:        roomZone,
			Source:      "dialogue_anchor",
			SourceScore: 1,
			Weight:      2,
			UpdatedAt:   nowUnix,
			ExpiresAt:   nowUnix + 3600,
		})
	}
	setPairLastTalk(A.ID, B.ID, nowUnix)
	// 更新 dyad（P1：先做熟悉度累積）
	if dyad != nil {
		dyad.Familiarity += 2
		if dyad.Familiarity > 100 {
			dyad.Familiarity = 100
		}
		dyad.Sentiment += sentimentDeltaFromDialogue(topicHint, lineA, lineB, dyad.Familiarity)
		if dyad.Sentiment > 100 {
			dyad.Sentiment = 100
		}
		if dyad.Sentiment < -100 {
			dyad.Sentiment = -100
		}
		if npcPairSameVenue(database, A.ID, B.ID) {
			dyad.Tags = addTag(dyad.Tags, "同職場")
		}
		if dyad.Sentiment <= -30 {
			dyad.Tags = addTag(dyad.Tags, "曾口角")
		}
		if dyad.Sentiment >= 25 {
			dyad.Tags = removeTag(dyad.Tags, "曾口角")
		}
		dyad.UpdatedAt = nowUnix
		_ = db.SetNpcNpcDyad(A.ID, B.ID, dyad)
	}
	npcSocialLastChoice.mu.Lock()
	npcSocialLastChoice.choice = &lastNpcSocialChoice{
		At:             nowUnix,
		RoomID:         roomID,
		AID:            A.ID,
		BID:            B.ID,
		ScoreTotal:     best.Score,
		TopicHint:      topicHint,
		TopicID:        chosenTopicID,
		TopicReason:    topicSource,
		AnchorsUsed:    anchorsUsed,
		AnchorsWritten: anchors,
		AnchorConflict: false,
	}
	npcSocialLastChoice.mu.Unlock()
	if sessionStore.RoomHasPlayerWithRecentTalk(database, roomID, 15*time.Second) {
		return true // 已寫記憶，不播
	}
	text := "【" + nameA + "】對【" + nameB + "】說：「" + lineA + "」【" + nameB + "】說：「" + lineB + "」"
	server.SendNarrateToRoom(sessionStore, database, roomID, text)
	incNpcSocialStat("success")
	return true
}

// buildActiveRoomIDs 回傳「觀測圈」：有玩家的房間＋其鄰房；僅這些房內的 NPC 會被腦／移動驅動。無人時回傳空 map。
func buildActiveRoomIDs(sessionStore *server.SessionStore, database *sql.DB, roomGraph *db.RoomGraph) map[string]bool {
	playerRooms := server.GetPlayerRoomMap(sessionStore, database)
	out := make(map[string]bool)
	for rid := range playerRooms {
		out[rid] = true
		for _, nb := range roomGraph.Neighbors(rid) {
			out[nb] = true
		}
	}
	return out
}

// applyBrainArrivalEffects 依意圖套用真實效果：Beg 加鎂、Gather 加物品、SeekJob 嘗試寫入指派；並記事件與心境。
func applyBrainArrivalEffects(database *sql.DB, entityID, roomID string, intent db.IntentType) {
	switch intent {
	case db.IntentBeg:
		amount := 3 + rand.Intn(8) // 3～10 鎂
		_ = db.AddMagnesium(database, entityID, amount)
		db.LogNPCEvent(entityID, db.EvtBeg, fmt.Sprintf("在%s乞討，得%d鎂", roomID, amount))
		db.AdjustDisposition(database, entityID, db.DispBegSuccess)
	case db.IntentGather:
		_ = db.AddToInventory(database, entityID, "wild_herb", 1)
		db.LogNPCEvent(entityID, db.EvtGather, fmt.Sprintf("在%s採集野草", roomID))
		db.AdjustDisposition(database, entityID, db.DispGather)
	case db.IntentSeekJob:
		assignments, _ := db.GetAssignmentsForEntity(database, entityID)
		if len(assignments) > 0 {
			return
		}
		venueIDs, _ := db.GetVenueIDsForRoom(database, roomID)
		for _, vid := range venueIDs {
			n, _ := db.GetAssignmentCountByVenue(database, vid)
			if n < maxAssignmentsPerVenue {
				if err := db.InsertAssignment(database, entityID, "服務生", vid, ""); err == nil {
					db.LogNPCEvent(entityID, db.EvtHired, fmt.Sprintf("在%s獲得%s職位", roomID, "服務生"))
					db.AdjustDisposition(database, entityID, db.DispHired)
					break
				}
			}
		}
	case db.IntentSocialize:
		db.LogNPCEvent(entityID, db.EvtTalk, "社交閒逛")
		db.AdjustDisposition(database, entityID, db.DispTalked)
	}
}

func main() {
	cfg := config.DefaultServer()

	if err := os.MkdirAll("data", 0755); err != nil {
		log.Fatalf("mkdir data: %v", err)
	}
	if err := os.MkdirAll("data/runtime", 0755); err != nil {
		log.Fatalf("mkdir data/runtime: %v", err)
	}

	// 全專案以 JSON 為唯一數據源：載入 store（data/rooms 目錄一房一檔 + runtime + data）
	if err := store.Init("data/rooms", "data/runtime", "data"); err != nil {
		log.Fatalf("store init: %v", err)
	}
	// store 模式下確保預設 NPC 存在（試話等）；若 data/entities.json 無則建立並寫入 store
	if err := db.SeedNPCsForStore(nil); err != nil {
		log.Printf("seed npcs for store: %v", err)
	}
	// 保證全部 NPC 都有 soul_seed；缺漏者補寫（僅補 seed，不改 vit/qi/dex）
	if fixed, err := db.EnsureAllNPCsHaveSoulSeed(nil); err != nil {
		log.Printf("EnsureAllNPCsHaveSoulSeed: %v", err)
	} else if fixed > 0 {
		log.Printf("[npc] 已為 %d 名 NPC 補寫 soul_seed", fixed)
	}
	defer func() {
		if store.Default != nil {
			if err := store.Default.FlushEntities(); err != nil {
				log.Printf("flush entities on exit: %v", err)
			}
		}
	}()

	var database *sql.DB // nil：不再使用 DB 檔，所有讀寫經由 store

	hub := server.NewHub(cfg.MaxWebSocketConn)
	sessionStore := server.NewSessionStore()
	upgrader := websocket.Upgrader{CheckOrigin: func(r *http.Request) bool { return true }}

	http.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		conn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			log.Printf("upgrade: %v", err)
			if strings.Contains(err.Error(), "upgrade") && strings.Contains(err.Error(), "Connection") {
				log.Printf("[ws] 提示：若經由 Cloudflare Tunnel/反向代理，請確認已轉傳 Connection: Upgrade 與 Upgrade: websocket")
			}
			return
		}
		client := server.NewClient(conn)
		if !hub.Register(client) {
			_ = conn.WriteMessage(websocket.CloseMessage,
				websocket.FormatCloseMessage(websocket.ClosePolicyViolation, "max connections reached"))
			_ = conn.Close()
			return
		}
		onClose := func(c *server.Client) {
			if c.PlayerID != "" {
				if s := sessionStore.Get(c.PlayerID); s != nil && s.Client == c {
					sessionStore.Remove(c.PlayerID)
				}
			}
			hub.Unregister(c)
		}
		go server.ReadLoop(client, onClose, database, cfg, sessionStore, hub)
	})

	http.HandleFunc("/api/design-constants", config.ServeDesignConstants)
	// 房間 API：/api/rooms（列表）與 /api/rooms/xxx（單一房間操作）皆由同一 handler 處理，避免 PUT /api/rooms/lobby 路由錯誤。
	roomsAPI := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { server.HandleRoomsAPI(database, w, r) })
	http.Handle("/api/rooms/", roomsAPI)
	http.HandleFunc("/api/rooms", roomsAPI.ServeHTTP)
	http.HandleFunc("/api/admin/wipe-entities", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "POST only", http.StatusMethodNotAllowed)
			return
		}
		if err := db.DeleteAllEntities(database); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"ok":true,"message":"已刪除所有角色"}`))
	})
	// 地圖檢視器：/map_viewer，資料由 /data/rooms.json API 從 store 彙總
	http.HandleFunc("/map_viewer", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/map_viewer" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		w.Header().Set("Cache-Control", "no-cache, max-age=0, must-revalidate")
		http.ServeFile(w, r, filepath.Join("web", "map_viewer.html"))
	})
	http.HandleFunc("/data/rooms.json", server.HandleRoomsDataAPI)
	// 房間心智圖編輯器：/room_editor + /api/room-editor/*
	http.HandleFunc("/room_editor", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/room_editor" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		w.Header().Set("Cache-Control", "no-cache, max-age=0, must-revalidate")
		http.ServeFile(w, r, filepath.Join("web", "room_editor.html"))
	})
	roomEditorAPI := http.HandlerFunc(server.HandleRoomEditorAPI)
	http.Handle("/api/room-editor/", roomEditorAPI)
	http.HandleFunc("/api/room-editor", roomEditorAPI.ServeHTTP)
	// 星盤檢視器：/star_chart，資料由 /api/topology 提供（store）
	http.HandleFunc("/star_chart", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/star_chart" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		w.Header().Set("Cache-Control", "no-cache, max-age=0, must-revalidate")
		http.ServeFile(w, r, filepath.Join("web", "star_chart.html"))
	})
	http.HandleFunc("/api/topology", func(w http.ResponseWriter, r *http.Request) {
		server.HandleTopologyAPI(database, w, r)
	})
	http.HandleFunc("/api/player-room", func(w http.ResponseWriter, r *http.Request) {
		server.HandlePlayerRoomAPI(database, w, r)
	})
	http.HandleFunc("/api/debug/npc-social", func(w http.ResponseWriter, r *http.Request) {
		handleNpcSocialDebug(database, w, r)
	})
	http.HandleFunc("/api/debug/npc-social/reset", func(w http.ResponseWriter, r *http.Request) {
		handleNpcSocialDebugReset(w, r)
	})

	// Chatmery Web：/chatmery 轉發至 localhost:1722（Tunnel 只指 1721 時由奇點代轉）
	if chatmeryURL, err := url.Parse("http://127.0.0.1:1722"); err == nil {
		chatmeryProxy := httputil.NewSingleHostReverseProxy(chatmeryURL)
		http.HandleFunc("/chatmery", func(w http.ResponseWriter, r *http.Request) {
			if r.URL.Path != "/chatmery" {
				chatmeryProxy.ServeHTTP(w, r)
				return
			}
			http.Redirect(w, r, "/chatmery/", http.StatusFound)
		})
		http.Handle("/chatmery/", chatmeryProxy)
	}

	fs := http.FileServer(http.Dir("web"))
	http.Handle("/", http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		p := r.URL.Path
		if strings.HasSuffix(p, ".js") || strings.HasSuffix(p, ".css") || strings.HasSuffix(p, ".html") || p == "/" {
			w.Header().Set("Cache-Control", "no-cache, max-age=0, must-revalidate")
		}
		fs.ServeHTTP(w, r)
	}))

	// 視野內 NPC 即時模擬 ＋ 每 tick 推進移動中實體（§1.2.3、§1.3.3）。§七 7.1：房間制觀測經 Observer。
	obs := &game.Observed{DB: database}
	server.SetDefaultObserver(obs)
	var lastScheduleHour = -1
	var lastExpenseDay = -1
	var lastSpawnCheck = time.Now()

	// NPC 活化：閒置動作 & 巡邏計時器（中頻 5-12 真實秒，即 2-5 遊戲分鐘）
	db.LoadBehaviors("data/npc_behaviors.json")
	db.LoadOccupations("data/templates/occupations.json")
	db.LoadNpcNpcTopics("data/npc_to_npc_topics.json")
	// 房間可互動物件：僅從各房間 JSON 的 objects 欄位載入
	if store.Default != nil {
		for _, id := range store.Default.RoomIDs() {
			r, _ := store.Default.GetRoom(id)
			if r != nil && len(r.Objects) > 0 {
				db.SetObjectsForRoom(id, r.Objects)
			}
		}
	}
	var idleTickCount int
	nextIdleTrigger := 25 + rand.Intn(35)
	var randomNpcDialogueTicksLeft int = 80 + rand.Intn(40) // 觸發－輕：隨機時點觸發 NPC 間對話
	var lastJobMatching time.Time                           // 10.15 求職撮合：每 30 秒跑一輪
	var lastRumorDecay time.Time
	var lastRumorDigestBuild time.Time

	// 尋路引擎：建立房間鄰接圖
	roomGraph := db.GetGraph()
	if err := roomGraph.BuildGraph(database); err != nil {
		log.Printf("[pathfind] build graph failed: %v", err)
	}

	// 地圖型 NPC 移動管理器：排班型（經理等）＋腦驅動型（無排班 NPC 依 Decide→Intent 尋路）
	travelerMgr := db.NewTravelerManager()
	schedules, _ := db.GetAllSchedules(database)
	scheduledIDs := make(map[string]bool)
	for _, s := range schedules {
		c, _ := db.GetEntity(database, s.EntityID)
		if c == nil || c.Vit <= 0 {
			continue
		}
		scheduledIDs[s.EntityID] = true
		title := db.GetNPCTitle(database, s.EntityID)
		def := db.GetMovementDefForTitle(title)
		def.Type = db.MoveSchedule
		travelerMgr.Register(s.EntityID, def)
	}
	npcIDs, _ := db.GetNPCIDsWithRoom(database)
	for _, id := range npcIDs {
		if scheduledIDs[id] {
			continue
		}
		travelerMgr.Register(id, db.MovementDef{Type: db.MoveBrain, Speed: 1})
	}
	var travelTickCount int
	travelTickInterval := 30 // 每 15 秒推進一步（30 ticks × 500ms）

	go game.Loop(cfg.TickInterval, func() {
		game.RunViewSimulation(database, func() []game.Pos { return server.GetObserverPositions(sessionStore, database) }, obs)

		now := time.Now().Unix()
		_, hour, _, gameDay := game.GameTimeNow(now, cfg.GameTimeEpochUnix, cfg.GameTimeScale)
		if time.Since(lastRumorDecay) >= 30*time.Second {
			lastRumorDecay = time.Now()
			_ = db.DecayNpcRumors(now)
		}
		if time.Since(lastRumorDigestBuild) >= 2*time.Minute {
			lastRumorDigestBuild = time.Now()
			_ = db.BuildNpcRumorDigest(now)
		}

		// 觸發－輕：隨機時點在某一有玩家的房觸發 NPC 間對話（玩家在場時間隔略拉長，減少洗頻）
		randomNpcDialogueTicksLeft--
		if randomNpcDialogueTicksLeft <= 0 && cfg.OllamaBaseURL != "" && cfg.OllamaModel != "" {
			playerRoomsForRandom := server.GetPlayerRoomMap(sessionStore, database)
			if len(playerRoomsForRandom) > 0 {
				randomNpcDialogueTicksLeft = 115 + rand.Intn(75)
				rooms := make([]string, 0, len(playerRoomsForRandom))
				for r := range playerRoomsForRandom {
					rooms = append(rooms, r)
				}
				roomID := rooms[rand.Intn(len(rooms))]
				topicHint := ""
				if t := db.PickRandomNpcNpcTopicByMask(topicMaskForRoom(database, roomID, hour), ""); t != nil {
					topicHint = t.Hint
				}
				tryTriggerNpcNpcInRoom(database, sessionStore, cfg, roomID, topicHint)
			} else {
				minE := cfg.NpcNpcSocialTickMinNoPlayer
				if minE <= 0 {
					minE = 80
				}
				extraE := cfg.NpcNpcSocialTickExtraNoPlayer
				if extraE <= 0 {
					extraE = 40
				}
				randomNpcDialogueTicksLeft = minE + rand.Intn(extraE)
			}
		}

		// NPC 每日消耗：每遊戲日扣食宿鎂（第四個回傳值為 gameDay，勿與 min 搞混）
		if gameDay != lastExpenseDay {
			lastExpenseDay = gameDay
			db.DeductDailyExpense(database)
			// 經濟傳聞降頻：避免每個遊戲日都刷到同一句「結算日」造成洗版。
			if gameDay%3 == 0 {
				ecoLines := []string{
					"這幾天米鹽又貴了一點，街坊都在精打細算。",
					"聽說租鋪成本漲了，幾家小攤開始提早收攤。",
					"近來活錢不寬，茶館裡談的多半是省錢門道。",
					"有人說東街貨價又動了，買賣人都在觀望。",
				}
				ecoText := ecoLines[rand.Intn(len(ecoLines))]
				_ = db.UpsertNpcRumor(&store.NpcRumor{
					ID:          fmt.Sprintf("eco|pulse|%d", gameDay/3),
					Text:        ecoText,
					Source:      "economy",
					SourceScore: 1,
					Weight:      1,
					UpdatedAt:   now,
					ExpiresAt:   now + 5400,
				})
			}
		}

		// NPC 池：總量＝玩家＋NPC；固定間隔檢查，未滿則生成一名並註冊腦驅動（男女數持平）
		if cfg.NPCPoolSize > 0 && cfg.NPCSpawnIntervalSec > 0 && time.Since(lastSpawnCheck) >= time.Duration(cfg.NPCSpawnIntervalSec)*time.Second {
			lastSpawnCheck = time.Now()
			npcIDs, _ := db.GetNPCIDsWithRoom(database)
			playerIDs, _ := db.GetPlayerIDsWithRoom(database)
			totalInWorld := len(npcIDs) + len(playerIDs)
			if totalInWorld < cfg.NPCPoolSize {
				spawnRoom := db.GetSpawnRoomID(database)
				if newID, err := db.SpawnOneNPCFromPool(database, spawnRoom); err == nil && newID != "" {
					travelerMgr.Register(newID, db.MovementDef{Type: db.MoveBrain, Speed: 1})
					spawnZone := ""
					if room, _ := db.GetRoom(database, spawnRoom); room != nil {
						spawnZone = room.Zone
					}
					_ = db.UpsertNpcRumor(&store.NpcRumor{
						ID:          fmt.Sprintf("spawn|%s|%s", spawnRoom, newID),
						Text:        "聽說又有新面孔進了這座城。",
						RoomID:      spawnRoom,
						Zone:        spawnZone,
						Source:      "spawn",
						SourceScore: 1,
						Weight:      1,
						UpdatedAt:   now,
						ExpiresAt:   now + 3600,
					})
				}
			}
		}

		// 10.15 求職撮合：無職且鎂低於閾值的 NPC 與有職缺場所匹配，每 30 秒一輪；新入職者註冊排班型移動
		if time.Since(lastJobMatching) >= 30*time.Second {
			lastJobMatching = time.Now()
			mgThreshold := cfg.SeekJobMgThreshold
			if mgThreshold <= 0 {
				mgThreshold = db.SeekJobMgThresholdDefault
			}
			added := db.RunJobMatchingTick(database, roomGraph, db.JobMatchParams{
				MaxPerVenue:        maxAssignmentsPerVenue,
				MgThreshold:        mgThreshold,
				JobMatchWhenStable: cfg.JobMatchWhenStable,
			})
			for _, entityID := range added {
				if _, ok := db.GetScheduleForEntity(database, entityID); ok {
					title := db.GetNPCTitle(database, entityID)
					def := db.GetMovementDefForTitle(title)
					def.Type = db.MoveSchedule
					travelerMgr.Register(entityID, def)
				}
			}
			if len(added) > 0 {
				_ = db.UpsertNpcRumor(&store.NpcRumor{
					ID:          fmt.Sprintf("job|%d", now/1800),
					Text:        fmt.Sprintf("最近鎮上又有人找到差事，約有 %d 人開始新工。", len(added)),
					Source:      "job",
					SourceScore: 4,
					Weight:      2,
					UpdatedAt:   now,
					ExpiresAt:   now + 7200,
				})
			}
		}

		// NPC 排班：每遊戲小時僅發「出發」敘事；實際移動由 TravelerManager 排班型尋路逐格執行（家可十格外）
		if hour != lastScheduleHour {
			lastScheduleHour = hour
			// 10.22 時段 disposition 微調：清晨+1、深夜-1
			{
				var dispDelta int
				if hour >= 6 && hour <= 9 {
					dispDelta = +1
				} else if hour >= 0 && hour <= 5 {
					dispDelta = -1
				}
				if dispDelta != 0 {
					if activeIDs, err := db.GetNPCIDsWithRoom(database); err == nil {
						for _, nid := range activeIDs {
							db.AdjustDisposition(database, nid, dispDelta)
						}
					}
				}
			}
			moves, err := db.ApplySchedules(database, hour)
			if err == nil {
				for _, m := range moves {
					target, _ := db.GetScheduleTarget(database, m.EntityID, hour)
					var leaveText string
					if target.Room == m.NewRoom && target.IsWork {
						leaveText = "【" + m.EntityID + "】出門往店裡去了。"
					} else {
						leaveText = db.GetShiftFlavor(m.Title, m.EntityID, false)
					}
					server.SendNarrateToRoom(sessionStore, database, m.OldRoom, leaveText)
					pushRoomEvent(m.OldRoom, "shift_leave", m.EntityID, "離開前往"+m.NewRoom)
					pushRoomEvent(m.NewRoom, "shift_arrive", m.EntityID, "排班抵達")
				}
				if len(moves) > 0 {
					server.BroadcastRoomViews(sessionStore, database, cfg)
					// 觸發－中：排班時段有動靜的房，試觸發一次 NPC 間對話（主題：交班）
					topicHint := ""
					if t := db.GetNpcNpcTopicByID("交班"); t != nil {
						topicHint = t.Hint
					}
					roomIDsWithActivity := make(map[string]bool)
					for _, m := range moves {
						roomIDsWithActivity[m.OldRoom] = true
						roomIDsWithActivity[m.NewRoom] = true
					}
					for roomID := range roomIDsWithActivity {
						hint := topicHint
						if hint == "" {
							if t := db.PickRandomNpcNpcTopicByMask(topicMaskForRoom(database, roomID, hour), ""); t != nil {
								hint = t.Hint
							}
						}
						if tryTriggerNpcNpcInRoom(database, sessionStore, cfg, roomID, hint) {
							break
						}
					}
				}
			}
		}

		// 地圖型 NPC 移動：每 travelTickInterval 推進一步；僅驅動「觀測圈」內 NPC（玩家房＋鄰房）
		travelTickCount++
		if travelTickCount >= travelTickInterval {
			travelTickCount = 0
			activeRoomIDs := buildActiveRoomIDs(sessionStore, database, roomGraph)
			if len(activeRoomIDs) == 0 {
				// 無人觀測：事實仍持續。排班者依 gameHour 朝 work/rest 走一步；無排班者抽樣加鎂／採集／求職／隨機鄰房。不發敘事。
				db.RunUnobservedWorldTick(database, roomGraph, hour, db.UnobservedMaxNPCsPerTick)
			}
			travelSteps := travelerMgr.Tick(database, roomGraph, hour, activeRoomIDs)
			for _, step := range travelSteps {
				oldName := roomGraph.RoomName(step.OldRoom)
				newName := roomGraph.RoomName(step.NewRoom)
				leaveText := "【" + step.NpcName + "】收拾行裝，往" + newName + "方向離去。"
				arriveText := "【" + step.NpcName + "】從" + oldName + "方向走了過來。"
				if target, ok := db.GetScheduleTarget(database, step.EntityID, hour); ok && target.Room == step.NewRoom {
					if target.IsWork {
						arriveText = db.GetShiftFlavor(db.GetNPCTitle(database, step.EntityID), step.EntityID, true)
					} else {
						arriveText = "【" + step.NpcName + "】回到了住處。"
					}
				}
				server.SendNarrateToRoom(sessionStore, database, step.OldRoom, leaveText)
				server.SendNarrateToRoom(sessionStore, database, step.NewRoom, arriveText)
				pushRoomEvent(step.OldRoom, "leave", step.NpcName, "離開往"+newName)
				pushRoomEvent(step.NewRoom, "arrive", step.NpcName, "從"+oldName+"方向到達")
				// 腦驅動到達後行為：敘事＋真實效果（Beg 加鎂、Gather 加物品、SeekJob 撮合）
				// 大腦可見化：NPC 決定新意圖時的出發敘事，發至出發房間
				if step.DecisionNarrative != "" {
					server.SendNarrateToRoom(sessionStore, database, step.OldRoom, step.DecisionNarrative)
				}
				if step.ArrivalIntent != "" {
					arrivalNarrative := brainArrivalNarrative(step.NpcName, step.ArrivalIntent)
					if arrivalNarrative != "" {
						server.SendNarrateToRoom(sessionStore, database, step.NewRoom, arrivalNarrative)
					}
					applyBrainArrivalEffects(database, step.EntityID, step.NewRoom, step.ArrivalIntent)
				}
				server.RefreshRoomViews(sessionStore, database, cfg, step.OldRoom)
				server.RefreshRoomViews(sessionStore, database, cfg, step.NewRoom)
			}
		}

		// NPC 閒置動作 & 巡邏：計時器到期後觸發
		idleTickCount++
		if idleTickCount >= nextIdleTrigger {
			idleTickCount = 0
			nextIdleTrigger = 60 + rand.Intn(60)
			period := db.GetTimePeriod(hour)

			playerRooms := server.GetPlayerRoomMap(sessionStore, database)
			schedules, _ := db.GetAllSchedules(database)

			// 觸發－重：閒置 tick 時同房兩 NPC 一來一往（主題劇本＋AI、寫記憶）
			npcDialogueDone := false
			if cfg.OllamaBaseURL != "" && cfg.OllamaModel != "" {
				for roomID := range playerRooms {
					topicHint := ""
					if t := db.PickRandomNpcNpcTopicByMask(topicMaskForRoom(database, roomID, hour), ""); t != nil {
						topicHint = t.Hint
					}
					if tryTriggerNpcNpcInRoom(database, sessionStore, cfg, roomID, topicHint) {
						npcDialogueDone = true
						break
					}
				}
			}
			if !npcDialogueDone {
				// Fallback：微互動（15% 機率），每輪最多一次
				for roomID := range playerRooms {
					social := db.PickMicroInteraction(database, roomID, 15)
					if social != "" {
						server.SendNarrateToRoom(sessionStore, database, roomID, social)
						break
					}
				}
			}

			for _, s := range schedules {
				if !s.IsOnDuty(hour) {
					continue
				}
				npcRoom, _ := db.GetEntityRoom(database, s.EntityID)
				title := db.GetNPCTitle(database, s.EntityID)

				// 巡邏：10% 機率移動到 wander_rooms 中的另一房間
				wanderRooms := db.GetWanderRooms(title)
				if len(wanderRooms) > 1 && rand.Intn(10) == 0 {
					var candidates []string
					for _, wr := range wanderRooms {
						if wr != npcRoom {
							candidates = append(candidates, wr)
						}
					}
					if len(candidates) > 0 {
						dest := candidates[rand.Intn(len(candidates))]
						destName, _ := db.GetRoomName(database, dest)
						srcName, _ := db.GetRoomName(database, npcRoom)
						leaveText := db.GetWanderFlavor(title, s.EntityID, destName, true)
						arriveText := db.GetWanderFlavor(title, s.EntityID, srcName, false)
						server.SendNarrateToRoom(sessionStore, database, npcRoom, leaveText)
						_ = db.SetEntityRoom(database, s.EntityID, dest)
						server.SendNarrateToRoom(sessionStore, database, dest, arriveText)
						pushRoomEvent(npcRoom, "leave", s.EntityID, "離開往"+destName)
						pushRoomEvent(dest, "arrive", s.EntityID, "從"+srcName+"方向到達")
						server.RefreshRoomViews(sessionStore, database, cfg, npcRoom)
						server.RefreshRoomViews(sessionStore, database, cfg, dest)
						continue
					}
				}

				// 閒置動作：僅對有玩家在場的房間觸發
				if _, hasPlayer := playerRooms[npcRoom]; !hasPlayer {
					continue
				}
				disp := db.GetDisposition(database, s.EntityID)
				emote := db.PickIdleEmote(title, period, s.EntityID, disp)
				if emote != "" {
					server.SendNarrateToRoom(sessionStore, database, npcRoom, emote)
					break
				}
			}
		}
	})

	// 經濟引擎：獨立 goroutine、自有 tick rate，後續可在 onTick 產出事件流／價格／任務報酬（§1.1.6）。
	economy.Run(cfg.EconomyTickInterval, func() {
		// 第一版留空；之後接鎂產消、交易、event.Append 事件流等。
		_ = database
	})

	log.Printf("listening :%s (max ws: %d, tick: %v, economy: %v)", cfg.Port, cfg.MaxWebSocketConn, cfg.TickInterval, cfg.EconomyTickInterval)
	if err := http.ListenAndServe(":"+cfg.Port, nil); err != nil {
		log.Fatalf("listen: %v", err)
	}
}
