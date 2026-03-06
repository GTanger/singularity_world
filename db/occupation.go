// Package db 負責職業型別（occupations）載入與依指派＋在場回傳有效插座，對齊討論 001。
package db

import (
	"database/sql"
	"encoding/json"
	"log"
	"os"
	"sync"
)

// OccupationDef 單一職業定義。
type OccupationDef struct {
	Name          string   `json:"name"`
	DialogueFile  string   `json:"dialogue_file"`
	BehaviorFile  string   `json:"behavior_file"`
	ActionSockets []string `json:"action_sockets"`
}

var (
	occupationOnce   sync.Once
	occupationCache  map[string]OccupationDef
	defaultSockets   = []string{"Talk", "Attack", "Look"}
)

// LoadOccupations 讀取並快取 occupations.json；首次呼叫後即從快取返回。
func LoadOccupations(path string) map[string]OccupationDef {
	occupationOnce.Do(func() {
		data, err := os.ReadFile(path)
		if err != nil {
			log.Printf("[occupation] load %s failed: %v", path, err)
			occupationCache = make(map[string]OccupationDef)
			return
		}
		var out struct {
			Occupations map[string]OccupationDef `json:"occupations"`
		}
		if err := json.Unmarshal(data, &out); err != nil {
			log.Printf("[occupation] parse %s failed: %v", path, err)
			occupationCache = make(map[string]OccupationDef)
			return
		}
		occupationCache = out.Occupations
		if occupationCache == nil {
			occupationCache = make(map[string]OccupationDef)
		}
		log.Printf("[occupation] loaded %d occupations from %s", len(occupationCache), path)
	})
	return occupationCache
}

// GetOccupationActionSockets 回傳該職業在場時才開放的動作插座；無定義則 nil。
func GetOccupationActionSockets(occupationID string) []string {
	if occupationCache == nil {
		LoadOccupations("data/templates/occupations.json")
	}
	occ, ok := occupationCache[occupationID]
	if !ok || len(occ.ActionSockets) == 0 {
		return nil
	}
	return occ.ActionSockets
}

// GetSocketsForNPC 回傳 NPC 在指定房間內的有效插座：預設插座＋僅在綁定場所內時才加的職業動作插座。
func GetSocketsForNPC(db *sql.DB, entityID, roomID string) []string {
	sockets := make([]string, 0, len(defaultSockets)+8)
	sockets = append(sockets, defaultSockets...)
	assignments, err := GetAssignmentsForEntity(db, entityID)
	if err != nil {
		return sockets
	}
	seen := make(map[string]bool)
	for _, a := range assignments {
		inVenue, err := IsRoomInVenue(db, roomID, a.VenueID)
		if err != nil || !inVenue {
			continue
		}
		for _, s := range GetOccupationActionSockets(a.OccupationID) {
			if !seen[s] {
				seen[s] = true
				sockets = append(sockets, s)
			}
		}
	}
	return sockets
}

// IsDefaultSocket 回傳該動詞是否為預設 Agent 插座（不需在場即可執行）。
func IsDefaultSocket(action string) bool {
	for _, s := range defaultSockets {
		if s == action {
			return true
		}
	}
	return false
}
