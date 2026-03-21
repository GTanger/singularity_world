// Package db NPC 間對話主題劇本：載入與查詢，供觸發時帶入 AI 或 fallback。
package db

import (
	"encoding/json"
	"log"
	"math/rand"
	"os"
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
func PickRandomNpcNpcTopicByMask(mask NpcTopicMask, excludeID string) *NpcNpcTopic {
	npcTopicsMu.RLock()
	list := npcTopicsList
	npcTopicsMu.RUnlock()
	var candidates []int
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
		candidates = append(candidates, i)
	}
	if len(candidates) > 0 {
		return &list[candidates[rand.Intn(len(candidates))]]
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
	hasTag := func(t string) bool {
		for _, v := range tags {
			if v == t {
				return true
			}
		}
		return false
	}
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
		if familiarity >= 50 && t.ID == "閒聊" {
			w += 2
		}
		if hasTag("同職場") && t.ID == "交班" {
			w += 3
		}
		if sentiment <= -20 && t.ID == "打聽" {
			w += 2 // 偏緊張時，更可能用打聽/試探語態開場
		}
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
	hasTag := func(t string) bool {
		for _, v := range tags {
			if v == t {
				return true
			}
		}
		return false
	}
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
		if familiarity >= 50 && t.ID == "閒聊" {
			w += 2
		}
		if hasTag("同職場") && t.ID == "交班" {
			w += 3
		}
		if sentiment <= -20 && t.ID == "打聽" {
			w += 2
		}
		d.Weight = w
		out = append(out, d)
	}
	return out
}
