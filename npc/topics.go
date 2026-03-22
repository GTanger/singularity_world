// Package npc：NPC 間對話主題劇本（載入與查詢），供觸發時帶入 AI 或 fallback。
package npc

import (
	"encoding/json"
	"log"
	"math/rand"
	"os"
	"strings"
	"sync"
)

// NpcNpcTopic 單一主題：id、給 AI 的 hint、候選句（可作 fallback 或種子）。
type NpcNpcTopic struct {
	ID           string   `json:"id"`
	Hint         string   `json:"hint"`
	Lines        []string `json:"lines"`
	RequiresWork bool     `json:"requires_work,omitempty"`
	NightOnly    bool     `json:"night_only,omitempty"`
	FollowUp     bool     `json:"follow_up,omitempty"`
}

type npcNpcTopicsFile struct {
	Topics []NpcNpcTopic `json:"topics"`
}

var (
	npcTopicsMu   sync.RWMutex
	npcTopicsList []NpcNpcTopic
)

// LoadNpcNpcTopics 從 path 載入主題劇本，僅執行一次或重載時呼叫。
func LoadNpcNpcTopics(path string) {
	data, err := os.ReadFile(path)
	if err != nil {
		if !os.IsNotExist(err) {
			log.Printf("[npc_topics] load %s: %v", path, err)
		}
		return
	}
	var f npcNpcTopicsFile
	if err := json.Unmarshal(data, &f); err != nil {
		log.Printf("[npc_topics] parse %s: %v", path, err)
		return
	}
	npcTopicsMu.Lock()
	npcTopicsList = f.Topics
	if npcTopicsList == nil {
		npcTopicsList = []NpcNpcTopic{}
	}
	npcTopicsMu.Unlock()
}

// GetNpcNpcTopicByID 依 id 回傳主題；無則回傳 nil。
func GetNpcNpcTopicByID(id string) *NpcNpcTopic {
	npcTopicsMu.RLock()
	defer npcTopicsMu.RUnlock()
	for i := range npcTopicsList {
		if npcTopicsList[i].ID == id {
			return &npcTopicsList[i]
		}
	}
	return nil
}

// normalizeNpcNpcTopicHint 去掉玩家在場時加在 hint 後的括注，便於與劇本 Hint 比對。
func normalizeNpcNpcTopicHint(h string) string {
	h = strings.TrimSpace(h)
	if i := strings.Index(h, "（路人在旁"); i >= 0 {
		h = strings.TrimSpace(h[:i])
	}
	return h
}

// FindNpcNpcTopicIDByHint 依目前 topicHint 反查主題 id（與 thread 存的 TopicType、加註後的 hint 對齊）。
// 先精確比對劇本 Hint，再取「hint 以前綴開頭」中最長的一筆，避免 thread 只存縮寫時無法辨識。
func FindNpcNpcTopicIDByHint(topicHint string) string {
	h := normalizeNpcNpcTopicHint(topicHint)
	if h == "" {
		return ""
	}
	npcTopicsMu.RLock()
	defer npcTopicsMu.RUnlock()
	for i := range npcTopicsList {
		th := strings.TrimSpace(npcTopicsList[i].Hint)
		if th != "" && h == th {
			return npcTopicsList[i].ID
		}
	}
	var bestID string
	var bestLen int
	for i := range npcTopicsList {
		th := strings.TrimSpace(npcTopicsList[i].Hint)
		if th == "" {
			continue
		}
		if strings.HasPrefix(h, th) && len(th) > bestLen {
			bestLen = len(th)
			bestID = npcTopicsList[i].ID
		}
	}
	return bestID
}

// PickRandomNpcNpcTopic 隨機回傳一主題；無主題時回傳 nil。
func PickRandomNpcNpcTopic() *NpcNpcTopic {
	return PickRandomNpcNpcTopicExclude("")
}

// PickRandomNpcNpcTopicExclude 隨機回傳一主題，但排除 excludeID（空字串表示不排除）。用於非職業場所不提示「交班」。
func PickRandomNpcNpcTopicExclude(excludeID string) *NpcNpcTopic {
	npcTopicsMu.RLock()
	list := npcTopicsList
	npcTopicsMu.RUnlock()
	var candidates []int
	for i := range list {
		if excludeID != "" && list[i].ID == excludeID {
			continue
		}
		candidates = append(candidates, i)
	}
	if len(candidates) == 0 {
		return nil
	}
	return &list[candidates[rand.Intn(len(candidates))]]
}

// NpcTopicMask 為主題篩選條件（P2）。
type NpcTopicMask struct {
	IsWorkVenue  bool
	IsNightTime  bool
	HasRoomEvent bool
	// RoomTags 為當前房間 tags（如 JSON 內 "outdoor"），供「城外聞」「天候」等主題加權。
	RoomTags []string
}

// roomHasTag 檢查房間是否含某 tag（與 data/rooms 內英文標籤一致，如 outdoor）。
func roomHasTag(roomTags []string, want string) bool {
	for _, v := range roomTags {
		if v == want {
			return true
		}
	}
	return false
}

// topicMaskBaseWeight 依情境與房間 tag 計算基礎權重（不含雙人關係加成）。
func topicMaskBaseWeight(mask NpcTopicMask, t *NpcNpcTopic) int {
	w := 1
	if mask.IsWorkVenue && t.RequiresWork {
		w += 4
	}
	if mask.HasRoomEvent && t.FollowUp {
		w += 3
	}
	if mask.IsNightTime && t.NightOnly {
		w += 2
	}
	// 露天／街道：較易談城外與天候（對齊沃土風題材）
	if roomHasTag(mask.RoomTags, "outdoor") {
		if t.ID == "城外聞" {
			w += 5
		}
		if t.ID == "天候" {
			w += 3
		}
	}
	// 人群聚集處：閒聊、八卦更自然
	if roomHasTag(mask.RoomTags, "social") && t.ID == "閒聊" {
		w += 2
	}
	if w < 1 {
		w = 1
	}
	return w
}

// pairRelationWeightAdj 雙人關係、同職場等額外加權。
func pairRelationWeightAdj(t *NpcNpcTopic, familiarity, sentiment int, dyadTags []string) int {
	hasTag := func(s string) bool {
		for _, v := range dyadTags {
			if v == s {
				return true
			}
		}
		return false
	}
	w := 0
	if familiarity >= 50 && t.ID == "閒聊" {
		w += 2
	}
	if hasTag("同職場") && t.ID == "交班" {
		w += 3
	}
	if sentiment <= -20 && t.ID == "打聽" {
		w += 2
	}
	return w
}

// TopicWeightDebug 為單一主題在當前條件下的權重明細（P2.6 debug）。
type TopicWeightDebug struct {
	ID       string `json:"id"`
	Hint     string `json:"hint"`
	Weight   int    `json:"weight"`
	Eligible bool   `json:"eligible"`
	Reason   string `json:"reason,omitempty"`
}

// PickRandomNpcNpcTopicByMask 依情境篩選主題，若無符合則回退為一般隨機（可排除某 id）。
// 權重含 topicMaskBaseWeight（含房間 outdoor 時對「城外聞」「天候」加成）。
func PickRandomNpcNpcTopicByMask(mask NpcTopicMask, excludeID string) *NpcNpcTopic {
	npcTopicsMu.RLock()
	list := npcTopicsList
	npcTopicsMu.RUnlock()
	type weighted struct {
		idx int
		w   int
	}
	var candidates []weighted
	var fallback []int
	for i := range list {
		t := list[i]
		if excludeID != "" && t.ID == excludeID {
			continue
		}
		fallback = append(fallback, i)
		if t.RequiresWork && !mask.IsWorkVenue {
			continue
		}
		if t.NightOnly && !mask.IsNightTime {
			continue
		}
		if t.FollowUp && !mask.HasRoomEvent {
			continue
		}
		w := topicMaskBaseWeight(mask, &t)
		candidates = append(candidates, weighted{idx: i, w: w})
	}
	if len(candidates) > 0 {
		total := 0
		for _, c := range candidates {
			total += c.w
		}
		r := rand.Intn(total)
		acc := 0
		for _, c := range candidates {
			acc += c.w
			if r < acc {
				return &list[c.idx]
			}
		}
	}
	if len(fallback) > 0 {
		return &list[fallback[rand.Intn(len(fallback))]]
	}
	return nil
}

// PickRandomNpcNpcTopicForPair 依情境+關係權重抽主題（P2.5）。
func PickRandomNpcNpcTopicForPair(mask NpcTopicMask, excludeID string, familiarity int, sentiment int, tags []string) *NpcNpcTopic {
	npcTopicsMu.RLock()
	list := npcTopicsList
	npcTopicsMu.RUnlock()
	type weighted struct {
		idx int
		w   int
	}
	var candidates []weighted
	var fallback []int
	for i := range list {
		t := list[i]
		if excludeID != "" && t.ID == excludeID {
			continue
		}
		fallback = append(fallback, i)
		if t.RequiresWork && !mask.IsWorkVenue {
			continue
		}
		if t.NightOnly && !mask.IsNightTime {
			continue
		}
		if t.FollowUp && !mask.HasRoomEvent {
			continue
		}
		w := topicMaskBaseWeight(mask, &t) + pairRelationWeightAdj(&t, familiarity, sentiment, tags)
		if w < 1 {
			w = 1
		}
		candidates = append(candidates, weighted{idx: i, w: w})
	}
	if len(candidates) > 0 {
		total := 0
		for _, c := range candidates {
			total += c.w
		}
		r := rand.Intn(total)
		acc := 0
		for _, c := range candidates {
			acc += c.w
			if r < acc {
				return &list[c.idx]
			}
		}
	}
	if len(fallback) > 0 {
		return &list[fallback[rand.Intn(len(fallback))]]
	}
	return nil
}

// DebugTopicWeightsForPair 回傳主題權重拆解，供 debug API 檢視選題依據（P2.6）。
func DebugTopicWeightsForPair(mask NpcTopicMask, excludeID string, familiarity int, sentiment int, tags []string) []TopicWeightDebug {
	npcTopicsMu.RLock()
	list := npcTopicsList
	npcTopicsMu.RUnlock()
	out := make([]TopicWeightDebug, 0, len(list))
	for _, t := range list {
		d := TopicWeightDebug{ID: t.ID, Hint: t.Hint, Eligible: true, Weight: 0}
		if excludeID != "" && t.ID == excludeID {
			d.Eligible = false
			d.Reason = "excluded by id"
			out = append(out, d)
			continue
		}
		if t.RequiresWork && !mask.IsWorkVenue {
			d.Eligible = false
			d.Reason = "requires_work"
			out = append(out, d)
			continue
		}
		if t.NightOnly && !mask.IsNightTime {
			d.Eligible = false
			d.Reason = "night_only"
			out = append(out, d)
			continue
		}
		if t.FollowUp && !mask.HasRoomEvent {
			d.Eligible = false
			d.Reason = "follow_up without room event"
			out = append(out, d)
			continue
		}
		w := topicMaskBaseWeight(mask, &t) + pairRelationWeightAdj(&t, familiarity, sentiment, tags)
		d.Weight = w
		out = append(out, d)
	}
	return out
}
