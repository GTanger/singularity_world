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
	ID    string   `json:"id"`
	Hint  string   `json:"hint"`
	Lines []string `json:"lines"`
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
