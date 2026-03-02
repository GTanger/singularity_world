package db

import (
	"encoding/json"
	"log"
	"math/rand"
	"os"
	"strings"
	"sync"
)

// BehaviorData 從 npc_behaviors.json 載入的全部 NPC 行為資料。
type BehaviorData struct {
	TimePeriods map[string][2]int          `json:"time_periods"`
	Roles       map[string]RoleBehavior    `json:"roles"`
}

// RoleBehavior 單一職稱的行為定義。
type RoleBehavior struct {
	Idle           map[string][]string `json:"idle"`
	EnterReactions []string            `json:"enter_reactions"`
	ShiftArrive    string              `json:"shift_arrive"`
	ShiftLeave     string              `json:"shift_leave"`
	WanderRooms    []string            `json:"wander_rooms"`
	WanderLeave    string              `json:"wander_leave"`
	WanderArrive   string              `json:"wander_arrive"`
}

var (
	behaviorOnce sync.Once
	behaviorCache *BehaviorData
)

// LoadBehaviors 讀取並快取 npc_behaviors.json；首次呼叫後即從快取返回。
func LoadBehaviors(path string) *BehaviorData {
	behaviorOnce.Do(func() {
		data, err := os.ReadFile(path)
		if err != nil {
			log.Printf("[behavior] load %s failed: %v", path, err)
			behaviorCache = &BehaviorData{
				Roles: make(map[string]RoleBehavior),
			}
			return
		}
		var bd BehaviorData
		if err := json.Unmarshal(data, &bd); err != nil {
			log.Printf("[behavior] parse %s failed: %v", path, err)
			behaviorCache = &BehaviorData{
				Roles: make(map[string]RoleBehavior),
			}
			return
		}
		behaviorCache = &bd
		log.Printf("[behavior] loaded %d roles from %s", len(bd.Roles), path)
	})
	return behaviorCache
}

// GetBehaviors 取得已快取的 BehaviorData（須先呼叫 LoadBehaviors）。
func GetBehaviors() *BehaviorData {
	if behaviorCache == nil {
		return &BehaviorData{Roles: make(map[string]RoleBehavior)}
	}
	return behaviorCache
}

// GetTimePeriod 將遊戲小時（0-23）映射為 morning/noon/evening/night。
func GetTimePeriod(gameHour int) string {
	bd := GetBehaviors()
	for name, bounds := range bd.TimePeriods {
		start, end := bounds[0], bounds[1]
		if start <= end {
			if gameHour >= start && gameHour < end {
				return name
			}
		} else {
			if gameHour >= start || gameHour < end {
				return name
			}
		}
	}
	return "night"
}

func replacePlaceholders(text, npcName string, extra map[string]string) string {
	text = strings.ReplaceAll(text, "{name}", npcName)
	for k, v := range extra {
		text = strings.ReplaceAll(text, "{"+k+"}", v)
	}
	return text
}

func pickRandom(list []string) string {
	if len(list) == 0 {
		return ""
	}
	return list[rand.Intn(len(list))]
}

// PickIdleEmote 隨機選一條指定職稱、時段的閒置動作，替換 {name}。
func PickIdleEmote(title, period, npcName string) string {
	bd := GetBehaviors()
	role, ok := bd.Roles[title]
	if !ok {
		return ""
	}
	emotes := role.Idle[period]
	if len(emotes) == 0 {
		return ""
	}
	return replacePlaceholders(pickRandom(emotes), npcName, nil)
}

// PickEnterReaction 隨機選一條進房反應，替換 {name}。
func PickEnterReaction(title, npcName string) string {
	bd := GetBehaviors()
	role, ok := bd.Roles[title]
	if !ok {
		return ""
	}
	return replacePlaceholders(pickRandom(role.EnterReactions), npcName, nil)
}

// GetShiftFlavor 取得換班敘事文本（arriving=true 為上班，false 為下班）。
func GetShiftFlavor(title, npcName string, arriving bool) string {
	bd := GetBehaviors()
	role, ok := bd.Roles[title]
	if !ok {
		return ""
	}
	if arriving {
		return replacePlaceholders(role.ShiftArrive, npcName, nil)
	}
	return replacePlaceholders(role.ShiftLeave, npcName, nil)
}

// GetWanderRooms 取得指定職稱的巡邏房間列表。
func GetWanderRooms(title string) []string {
	bd := GetBehaviors()
	role, ok := bd.Roles[title]
	if !ok {
		return nil
	}
	return role.WanderRooms
}

// GetWanderFlavor 取得巡邏敘事（leaving=true 為離開當前房間，false 為到達新房間）。
func GetWanderFlavor(title, npcName string, roomName string, leaving bool) string {
	bd := GetBehaviors()
	role, ok := bd.Roles[title]
	if !ok {
		return ""
	}
	extra := map[string]string{}
	if leaving {
		extra["dest"] = roomName
		return replacePlaceholders(role.WanderLeave, npcName, extra)
	}
	extra["from"] = roomName
	return replacePlaceholders(role.WanderArrive, npcName, extra)
}
