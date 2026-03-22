package npcnpc

import (
	"fmt"
	"strings"
	"sync"
	"time"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/gametext"
	"singularity_world/store"
)

type roomEvent struct {
	At      int64
	Kind    string
	Subject string
	Detail  string
}

var (
	roomEventMu     sync.RWMutex
	roomEventsByRID = make(map[string][]roomEvent)

	pairTalkMu sync.RWMutex
	pairTalkAt = make(map[string]int64)

	lastChoiceMu sync.RWMutex
	lastChoice   *LastSocialChoice

	statsMu      sync.RWMutex
	statsCounter map[string]int64
)

func init() {
	statsCounter = map[string]int64{
		"attempt":                 0,
		"success":                 0,
		"fail_no_model":           0,
		"fail_recent_player_talk": 0,
		"fail_not_enough_npc":     0,
		"fail_no_pair":            0,
		"fail_llm_empty":          0,
		"fail_quality_gate":       0,
		"fail_anchor_conflict":    0,
		"fail_score_too_low":      0,
		"score_pass":              0,
		"score_disabled_skip":     0,
		"fail_quality_unicode_garbage":      0,
		"fail_quality_leaked_id":            0,
		"fail_quality_markdown_residue":     0,
		"fail_quality_too_short":            0,
		"fail_quality_suspicious_narration": 0,
		"fail_quality_json_residue":         0,
		"skip_npc_npc_summary_dup":          0,
		"skip_npc_npc_archival_dup":         0,
	}
}

func incStat(key string) {
	statsMu.Lock()
	statsCounter[key]++
	statsMu.Unlock()
}

// StatsSnapshot 回傳統計計數副本（除錯用）。
func StatsSnapshot() map[string]int64 {
	statsMu.RLock()
	defer statsMu.RUnlock()
	out := make(map[string]int64, len(statsCounter))
	for k, v := range statsCounter {
		out[k] = v
	}
	return out
}

// ResetStats 將社交統計清零。
func ResetStats() {
	statsMu.Lock()
	for k := range statsCounter {
		statsCounter[k] = 0
	}
	statsMu.Unlock()
}

// LastChoiceSnapshot 回傳最近一次選擇的副本（若無則 nil）。
func LastChoiceSnapshot() *LastSocialChoice {
	lastChoiceMu.RLock()
	defer lastChoiceMu.RUnlock()
	if lastChoice == nil {
		return nil
	}
	cp := *lastChoice
	return &cp
}

func setLastChoice(c *LastSocialChoice) {
	lastChoiceMu.Lock()
	lastChoice = c
	lastChoiceMu.Unlock()
}

// ClearLastChoice 清除最近一次選擇記錄。
func ClearLastChoice() {
	lastChoiceMu.Lock()
	lastChoice = nil
	lastChoiceMu.Unlock()
}

func canonicalPairKey(idA, idB string) string {
	if idA > idB {
		return idB + "|" + idA
	}
	return idA + "|" + idB
}

// GetPairLastTalk 回傳兩 NPC 上次對話時間戳（unix）。
func GetPairLastTalk(idA, idB string) int64 {
	key := canonicalPairKey(idA, idB)
	pairTalkMu.RLock()
	defer pairTalkMu.RUnlock()
	return pairTalkAt[key]
}

// SetPairLastTalk 記錄兩 NPC 上次對話時間。
func SetPairLastTalk(idA, idB string, at int64) {
	key := canonicalPairKey(idA, idB)
	pairTalkMu.Lock()
	pairTalkAt[key] = at
	pairTalkMu.Unlock()
}

// PushRoomEvent 推入房間滑動視窗事件並可能寫入傳聞。
func PushRoomEvent(roomID, kind, subject, detail string) {
	if roomID == "" {
		return
	}
	now := time.Now().Unix()
	roomEventMu.Lock()
	list := roomEventsByRID[roomID]
	list = append(list, roomEvent{At: now, Kind: kind, Subject: subject, Detail: detail})
	re := config.Sim().RoomEvent
	cutoff := now - re.TTLSec
	filtered := make([]roomEvent, 0, len(list))
	for _, e := range list {
		if e.At >= cutoff {
			filtered = append(filtered, e)
		}
	}
	if re.MaxEvents > 0 && len(filtered) > re.MaxEvents {
		filtered = filtered[len(filtered)-re.MaxEvents:]
	}
	roomEventsByRID[roomID] = filtered
	roomEventMu.Unlock()
	pushRumorFromRoomEvent(roomID, kind, subject, detail, now)
}

func pushRumorFromRoomEvent(roomID, kind, subject, detail string, now int64) {
	if roomID == "" || strings.TrimSpace(detail) == "" {
		return
	}
	zone := ""
	if room, _ := db.GetRoom(roomID); room != nil {
		zone = room.Zone
	}
	text := strings.TrimSpace(subject + "：" + detail)
	if text == "" {
		return
	}
	id := fmt.Sprintf("evt|%s|%s|%s", roomID, kind, gametext.TruncRune(text, 24))
	_ = db.UpsertNpcRumor(&store.NpcRumor{
		ID:          id,
		Text:        gametext.TruncRune(text, 28),
		RoomID:      roomID,
		Zone:        zone,
		Source:      "room_event",
		SourceScore: 2,
		Weight:      1,
		UpdatedAt:   now,
		ExpiresAt:   now + config.Sim().RoomEvent.RumorTTLSec,
	})
}

// RecentRoomEvents 回傳房間近期事件敘述（最多 maxCount 筆）。
func RecentRoomEvents(roomID string, maxCount int) []string {
	if roomID == "" || maxCount <= 0 {
		return nil
	}
	roomEventMu.RLock()
	list := roomEventsByRID[roomID]
	roomEventMu.RUnlock()
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
