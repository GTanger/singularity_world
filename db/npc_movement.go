package db

import (
	"database/sql"
	"math/rand"
	"sync"
)

// MovementType NPC 移動模式。
type MovementType string

const (
	MoveRegional MovementType = "regional" // 區域型：在 wander_rooms 內隨機跳
	MoveRoute    MovementType = "route"    // 路線型：沿 waypoints 巡迴
	MovePathfind MovementType = "pathfind" // 尋路型：給定目標，BFS 自動尋路
	MoveSchedule MovementType = "schedule" // 排班型：依 gameHour 目標為 work_room 或 rest_room，尋路逐格移動
)

// Waypoint 路線型 NPC 的途經點。
type Waypoint struct {
	RoomID    string `json:"room"`
	StayHours [2]int `json:"stay_hours"` // [min, max] 遊戲小時
	Activity  string `json:"activity"`   // trade / rest / pass_through
}

// MovementDef 定義一個 NPC 的移動行為。存在行為模板 behaviors/*.json 中。
type MovementDef struct {
	Type            MovementType `json:"type"`
	Speed           int          `json:"speed"`            // 每次移動走幾格（預設 1）
	WanderRooms     []string     `json:"wander_rooms"`     // regional 模式用
	RouteWaypoints  []Waypoint   `json:"route_waypoints"`  // route 模式用
	RouteMode       string       `json:"route_mode"`       // loop / bounce / one_way
	HomeBase        string       `json:"home_base"`        // pathfind 模式用
	DestinationTags []string     `json:"destination_tags"` // pathfind 模式用
	WanderRange     int          `json:"wander_range"`     // pathfind 模式用：最大搜尋距離
	StayHours       [2]int       `json:"stay_hours"`       // pathfind 模式：到達後停留時間
}

// NPCTraveler 管理一個正在移動中的 NPC 的即時狀態（記憶體內）。
type NPCTraveler struct {
	EntityID      string
	MoveDef       MovementDef
	PathQueue     []string // 剩餘要走的房間 ID 序列
	WaypointIdx   int      // route 模式：當前目標 waypoint index
	RouteForward  bool     // route bounce 模式：正向 or 反向
	StayUntilHour int      // 到達後停留到此遊戲小時（-1 = 不停留）
	Active        bool
}

// TravelerManager 管理所有正在進行地圖級移動的 NPC。
type TravelerManager struct {
	mu        sync.Mutex
	travelers map[string]*NPCTraveler // entityID → traveler
}

func NewTravelerManager() *TravelerManager {
	return &TravelerManager{
		travelers: make(map[string]*NPCTraveler),
	}
}

// Register 註冊一個 NPC 進入移動系統。
func (tm *TravelerManager) Register(entityID string, def MovementDef) {
	tm.mu.Lock()
	defer tm.mu.Unlock()
	if def.Speed <= 0 {
		def.Speed = 1
	}
	tm.travelers[entityID] = &NPCTraveler{
		EntityID:      entityID,
		MoveDef:       def,
		RouteForward:  true,
		StayUntilHour: -1,
		Active:        true,
	}
}

// Unregister 移除 NPC。
func (tm *TravelerManager) Unregister(entityID string) {
	tm.mu.Lock()
	defer tm.mu.Unlock()
	delete(tm.travelers, entityID)
}

// NPCStep 一次移動步驟的結果：誰從哪到哪。
type NPCStep struct {
	EntityID string
	OldRoom  string
	NewRoom  string
	NpcName  string
}

// Tick 每次呼叫推進所有 traveler 一步。回傳實際發生移動的列表。
// gameHour 用於判斷停留期限和選擇下一個目標。
func (tm *TravelerManager) Tick(database *sql.DB, g *RoomGraph, gameHour int) []NPCStep {
	tm.mu.Lock()
	defer tm.mu.Unlock()

	var steps []NPCStep
	for _, t := range tm.travelers {
		if !t.Active {
			continue
		}

		// 停留中
		if t.StayUntilHour >= 0 {
			if gameHour != t.StayUntilHour {
				continue
			}
			t.StayUntilHour = -1
		}

		currentRoom, _ := GetEntityRoom(database, t.EntityID)

		// 如果 path queue 為空，需要決定下一個目標
		if len(t.PathQueue) == 0 {
			t.PathQueue = tm.computeNextPath(t, currentRoom, g, database, gameHour)
			if len(t.PathQueue) == 0 {
				continue
			}
		}

		// 依 movement.speed 每次走若干格（房間），寫回 entity_room，排班型會多 tick 才抵達
		stepsToTake := t.MoveDef.Speed
		if stepsToTake > len(t.PathQueue) {
			stepsToTake = len(t.PathQueue)
		}

		oldRoom := currentRoom
		for i := 0; i < stepsToTake; i++ {
			nextRoom := t.PathQueue[0]
			t.PathQueue = t.PathQueue[1:]
			_ = SetEntityRoom(database, t.EntityID, nextRoom)
			currentRoom = nextRoom
		}

		if currentRoom != oldRoom {
			npcName := GetNPCTitle(database, t.EntityID)
			if npcName == "" {
				npcName = t.EntityID
			}
			steps = append(steps, NPCStep{
				EntityID: t.EntityID,
				OldRoom:  oldRoom,
				NewRoom:  currentRoom,
				NpcName:  npcName,
			})
		}

		// 到達 waypoint / 目的地 → 計算停留
		if len(t.PathQueue) == 0 {
			stay := tm.computeStay(t, gameHour)
			if stay > 0 {
				t.StayUntilHour = (gameHour + stay) % 24
			}
		}
	}
	return steps
}

func (tm *TravelerManager) computeNextPath(t *NPCTraveler, currentRoom string, g *RoomGraph, database *sql.DB, gameHour int) []string {
	switch t.MoveDef.Type {
	case MoveRoute:
		return tm.nextRoutePath(t, currentRoom, g)
	case MovePathfind:
		return tm.nextPathfindPath(t, currentRoom, g)
	// 排班型：依 gameHour 目標為 work_room（在班）或 rest_room（下班），BFS 尋路；家可十格外逐格走
	case MoveSchedule:
		target, ok := GetScheduleTargetRoom(database, t.EntityID, gameHour)
		if !ok || target == "" || target == currentRoom {
			return nil
		}
		return g.FindPath(currentRoom, target)
	default:
		return nil
	}
}

func (tm *TravelerManager) nextRoutePath(t *NPCTraveler, currentRoom string, g *RoomGraph) []string {
	wps := t.MoveDef.RouteWaypoints
	if len(wps) == 0 {
		return nil
	}

	// 前進到下一個 waypoint
	if t.RouteForward {
		t.WaypointIdx++
		if t.WaypointIdx >= len(wps) {
			switch t.MoveDef.RouteMode {
			case "bounce":
				t.RouteForward = false
				t.WaypointIdx = len(wps) - 2
				if t.WaypointIdx < 0 {
					t.WaypointIdx = 0
				}
			case "loop":
				t.WaypointIdx = 0
			default: // one_way
				t.Active = false
				return nil
			}
		}
	} else {
		t.WaypointIdx--
		if t.WaypointIdx < 0 {
			t.RouteForward = true
			t.WaypointIdx = 1
			if t.WaypointIdx >= len(wps) {
				t.WaypointIdx = 0
			}
		}
	}

	target := wps[t.WaypointIdx].RoomID
	return g.FindPath(currentRoom, target)
}

func (tm *TravelerManager) nextPathfindPath(t *NPCTraveler, currentRoom string, g *RoomGraph) []string {
	tags := t.MoveDef.DestinationTags
	maxDist := t.MoveDef.WanderRange
	if maxDist <= 0 {
		maxDist = 50
	}

	candidates := g.FindRoomsWithinDist(currentRoom, tags, maxDist)
	if len(candidates) == 0 {
		return nil
	}

	// 隨機挑一個目標（排除當前位置）
	var filtered []string
	for _, c := range candidates {
		if c != currentRoom {
			filtered = append(filtered, c)
		}
	}
	if len(filtered) == 0 {
		return nil
	}
	target := filtered[rand.Intn(len(filtered))]
	return g.FindPath(currentRoom, target)
}

func (tm *TravelerManager) computeStay(t *NPCTraveler, gameHour int) int {
	var stayRange [2]int

	switch t.MoveDef.Type {
	case MoveRoute:
		wps := t.MoveDef.RouteWaypoints
		if t.WaypointIdx >= 0 && t.WaypointIdx < len(wps) {
			stayRange = wps[t.WaypointIdx].StayHours
		}
	case MovePathfind:
		stayRange = t.MoveDef.StayHours
	}

	if stayRange[1] <= 0 {
		return 0
	}
	min, max := stayRange[0], stayRange[1]
	if min > max {
		min = max
	}
	if min == max {
		return min
	}
	return min + rand.Intn(max-min+1)
}

// Count 回傳目前管理中的 traveler 數量。
func (tm *TravelerManager) Count() int {
	tm.mu.Lock()
	defer tm.mu.Unlock()
	return len(tm.travelers)
}

// GetTraveler 取得指定 NPC 的 traveler（供外部查詢路徑等）。
func (tm *TravelerManager) GetTraveler(entityID string) *NPCTraveler {
	tm.mu.Lock()
	defer tm.mu.Unlock()
	return tm.travelers[entityID]
}
