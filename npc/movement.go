package npc

import (
	"math/rand"
	"sync"

	"singularity_world/db"
)

// MovementType NPC 移動模式。
type MovementType string

const (
	MoveRegional MovementType = "regional" // 區域型：在 wander_rooms 內隨機跳
	MoveRoute    MovementType = "route"    // 路線型：沿 waypoints 巡迴
	MovePathfind MovementType = "pathfind" // 尋路型：給定目標，BFS 自動尋路
	MoveSchedule MovementType = "schedule" // 排班型：依 gameHour 目標為 work_room 或 rest_room，尋路逐格移動
	MoveBrain    MovementType = "brain"    // 腦驅動：每當 path 空時呼叫 Decide(state, context) 產生意圖，再依意圖尋路
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
	EntityID          string
	MoveDef           MovementDef
	PathQueue         []string // 剩餘要走的房間 ID 序列
	WaypointIdx       int      // route 模式：當前目標 waypoint index
	RouteForward      bool     // route bounce 模式：正向 or 反向
	StayUntilHour     int      // 到達後停留到此遊戲小時（-1 = 不停留）
	Active            bool
	LastIntent        Intent // 腦驅動型：產生當前 path 的意圖，到達目標房時供「到達後行為」用
	DepartureNarrative string // 腦驅動型：決定意圖時生成的出發敘事，首次移動時帶出並清空
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

// NPCStep 一次移動步驟的結果：誰從哪到哪；腦驅動抵達目標時帶 ArrivalIntent 供主迴圈觸發敘事／插座。
type NPCStep struct {
	EntityID          string
	OldRoom           string
	NewRoom           string
	NpcName           string
	ArrivalIntent     IntentType // 非空表示此步為腦驅動「抵達意圖目標」，主迴圈可發敘事或執行對應行為
	DecisionNarrative string     // 非空表示腦驅動決定新意圖時的出發敘事，主迴圈發送到 OldRoom
}

// Tick 每次呼叫推進 traveler 一步。回傳實際發生移動的列表。
// gameHour 用於判斷停留期限和選擇下一個目標。
// activeRoomIDs 若非 nil：僅驅動「當前房間在此集合內」的 NPC（觀測驅動）；空集合表示無人觀測、不驅動任何人；nil 表示驅動全部（相容舊行為）。
func (tm *TravelerManager) Tick(g *db.RoomGraph, gameHour int, activeRoomIDs map[string]bool) []NPCStep {
	tm.mu.Lock()
	defer tm.mu.Unlock()

	var steps []NPCStep
	for _, t := range tm.travelers {
		if !t.Active {
			continue
		}

		currentRoom, _ := db.GetEntityRoom(t.EntityID)
		// 觀測驅動：只驅動在「活躍房」內的 NPC；無人觀測時不跑任何腦／移動
		if activeRoomIDs != nil {
			if len(activeRoomIDs) == 0 {
				continue
			}
			if !activeRoomIDs[currentRoom] {
				continue
			}
		}

		// 停留中
		if t.StayUntilHour >= 0 {
			if gameHour != t.StayUntilHour {
				continue
			}
			t.StayUntilHour = -1
		}

		// 如果 path queue 為空，需要決定下一個目標
		if len(t.PathQueue) == 0 {
			t.PathQueue = tm.computeNextPath(t, currentRoom, g, gameHour)
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
			_ = db.SetEntityRoom(t.EntityID, nextRoom)
			currentRoom = nextRoom
		}

		if currentRoom != oldRoom {
			npcName := db.GetNPCDisplayLabelAtHour(t.EntityID, gameHour)
			if npcName == "" {
				npcName = t.EntityID
			}
			step := NPCStep{
				EntityID: t.EntityID,
				OldRoom:  oldRoom,
				NewRoom:  currentRoom,
				NpcName:  npcName,
			}
			// 腦驅動抵達：先算停留（此時 LastIntent 尚在），再寫 step 與清空，否則 computeStay 的 MoveBrain 分支只會進 default
			if len(t.PathQueue) == 0 && t.MoveDef.Type == MoveBrain && t.LastIntent.Type != "" {
				stay := tm.computeStay(t, gameHour)
				if stay > 0 {
					t.StayUntilHour = (gameHour + stay) % 24
				}
				step.ArrivalIntent = t.LastIntent.Type
				t.LastIntent = Intent{}
			}
			// 出發敘事（大腦可見化）：首次移動時帶出，清空避免重複
			if t.DepartureNarrative != "" {
				step.DecisionNarrative = t.DepartureNarrative
				t.DepartureNarrative = ""
			}
			steps = append(steps, step)
		}

		// 到達 waypoint / 目的地 → 計算停留（非 MoveBrain；MoveBrain 已在上方處理）
		if len(t.PathQueue) == 0 && t.MoveDef.Type != MoveBrain {
			stay := tm.computeStay(t, gameHour)
			if stay > 0 {
				t.StayUntilHour = (gameHour + stay) % 24
			}
		}
	}
	return steps
}

func (tm *TravelerManager) computeNextPath(t *NPCTraveler, currentRoom string, g *db.RoomGraph, gameHour int) []string {
	switch t.MoveDef.Type {
	case MoveRoute:
		return tm.nextRoutePath(t, currentRoom, g)
	case MovePathfind:
		return tm.nextPathfindPath(t, currentRoom, g)
	// 排班型：依 gameHour 目標為 work_room（在班）或 rest_room（下班），BFS 尋路；家可十格外逐格走
	case MoveSchedule:
		target, ok := db.GetScheduleTargetRoom(t.EntityID, gameHour)
		if !ok || target == "" || target == currentRoom {
			return nil
		}
		return g.FindPath(currentRoom, target)
	// 腦驅動：Decide(state, context) -> Intent，再 ResolveBrainPath 得到 path；存 LastIntent 供到達後行為
	case MoveBrain:
		state, err := BuildDecisionState(t.EntityID)
		if err != nil {
			return nil
		}
		ctx := BuildDecisionContext(g, currentRoom, 20)
		intent := Decide(state, ctx)
		t.LastIntent = intent
		// 20% 機率生成出發敘事（大腦可見化），隨首步移動帶到出發房間
		if rand.Float64() < 0.20 {
			npcName := db.GetNPCDisplayLabelAtHour(t.EntityID, gameHour)
			if npcName == "" {
				npcName = t.EntityID
			}
			t.DepartureNarrative = buildDepartureNarrative(npcName, intent.Type)
		}
		return ResolveBrainPath(g, intent, currentRoom, &ctx, 25)
	default:
		return nil
	}
}

func (tm *TravelerManager) nextRoutePath(t *NPCTraveler, currentRoom string, g *db.RoomGraph) []string {
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

func (tm *TravelerManager) nextPathfindPath(t *NPCTraveler, currentRoom string, g *db.RoomGraph) []string {
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
	case MoveBrain:
		switch t.LastIntent.Type {
		case IntentBeg:
			stayRange = [2]int{2, 4}
		case IntentGather:
			stayRange = [2]int{1, 3}
		case IntentSeekJob:
			stayRange = [2]int{1, 2}
		case IntentTrade:
			stayRange = [2]int{2, 5}
		case IntentSocialize:
			stayRange = [2]int{1, 3}
		case IntentWander:
			stayRange = [2]int{1, 4}
		case IntentWork:
			stayRange = [2]int{3, 6}
		default:
			stayRange = [2]int{1, 2}
		}
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

// buildDepartureNarrative 腦驅動 NPC 決定新意圖時的出發敘事（大腦可見化）。
// 僅在部分意圖下回傳非空字串；Wander / Idle 等低資訊意圖不敘事。
func buildDepartureNarrative(npcName string, intentType IntentType) string {
	switch intentType {
	case IntentSeekJob:
		return "【" + npcName + "】望著荷包，拍拍衣袖，打算出門碰碰運氣。"
	case IntentBeg:
		return "【" + npcName + "】長嘆一聲，朝熱鬧處走去。"
	case IntentGather:
		return "【" + npcName + "】拿起背簍，悄悄往郊外去了。"
	case IntentTrade:
		return "【" + npcName + "】把包袱繫緊，往人多的地方找買家去了。"
	case IntentSocialize:
		return "【" + npcName + "】閒不住，往人多的地方晃去。"
	}
	return ""
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
