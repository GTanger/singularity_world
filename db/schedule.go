package db

import (
	"strings"

	"singularity_world/store"
)

// NPCSchedule 描述一名 NPC 的排班：工作房間、休息房間、班次起迄（遊戲時 0-23）。
type NPCSchedule struct {
	EntityID   string
	WorkRoom   string
	RestRoom   string
	ShiftStart int // 遊戲時 0-23，班次開始
	ShiftEnd   int // 遊戲時 0-23，班次結束；可跨午夜（如 18→07）
}

// IsOnDuty 判斷在 gameHour（0-23）時該 NPC 是否在班。
func (s *NPCSchedule) IsOnDuty(gameHour int) bool {
	if s.ShiftStart <= s.ShiftEnd {
		return gameHour >= s.ShiftStart && gameHour < s.ShiftEnd
	}
	return gameHour >= s.ShiftStart || gameHour < s.ShiftEnd
}

// RemoveScheduleForEntity 移除某實體的排班（死亡除名用）。
func RemoveScheduleForEntity(entityID string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.RemoveScheduleForEntity(entityID)
}

// GetScheduleForEntity 取得單一實體的排班；供 10.15 撮合後 main 註冊 MoveSchedule 用。
func GetScheduleForEntity(entityID string) (NPCSchedule, bool) {
	if store.Default == nil {
		return NPCSchedule{}, false
	}
	sch := store.Default.GetSchedule(entityID)
	if sch == nil {
		return NPCSchedule{}, false
	}
	return NPCSchedule{
		EntityID: sch.EntityID, WorkRoom: sch.WorkRoom, RestRoom: sch.RestRoom,
		ShiftStart: sch.ShiftStart, ShiftEnd: sch.ShiftEnd,
	}, true
}

// GetAllSchedules 取得所有 NPC 排班。
func GetAllSchedules() ([]NPCSchedule, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	slist := store.Default.GetAllSchedules()
	list := make([]NPCSchedule, len(slist))
	for i, s := range slist {
		list[i] = NPCSchedule{
			EntityID: s.EntityID, WorkRoom: s.WorkRoom, RestRoom: s.RestRoom,
			ShiftStart: s.ShiftStart, ShiftEnd: s.ShiftEnd,
		}
	}
	return list, nil
}

// ScheduleMove 記錄一次排班移動：誰從哪到哪、職稱，供外部推送敘事用。
type ScheduleMove struct {
	EntityID string
	Title    string
	OldRoom  string
	NewRoom  string
}

// GetNPCPersonDisplayName 回傳 NPC 真名（entities.display_title）；無則自動產生並寫回 store。
func GetNPCPersonDisplayName(entityID string) string {
	if store.Default == nil {
		return ""
	}
	if e := store.Default.GetEntity(entityID); e != nil {
		if e.DisplayTitle != "" {
			return e.DisplayTitle
		}
		if e.Kind == "npc" {
			name := GenerateNPCName(e.Gender)
			e.DisplayTitle = name
			_ = store.Default.PutEntity(e)
			return name
		}
		return ""
	}
	return ""
}

// SplitNPCListDisplayLabel 解析同房列表顯示字串「職稱|真名」；無 | 則整段為真名，職稱為空。
func SplitNPCListDisplayLabel(label string) (occupationKey, personName string) {
	label = strings.TrimSpace(label)
	i := strings.Index(label, "|")
	if i < 0 {
		return "", label
	}
	return strings.TrimSpace(label[:i]), strings.TrimSpace(label[i+1:])
}

// PersonNameFromNPCListLabel 從可能帶「職稱|」前綴的列表顯示字串取出真名。
func PersonNameFromNPCListLabel(label string) string {
	_, p := SplitNPCListDisplayLabel(label)
	if p != "" {
		return p
	}
	return strings.TrimSpace(label)
}

// occupationForEntityAtRoom 若該房間屬於某筆指派的場所，回傳該筆職稱；否則空字串。
func occupationForEntityAtRoom(entityID, roomID string) string {
	if roomID == "" {
		return ""
	}
	list, err := GetAssignmentsForEntity(entityID)
	if err != nil {
		return ""
	}
	for _, a := range list {
		ok, err := IsRoomInVenue(roomID, a.VenueID)
		if err != nil || !ok || a.OccupationID == "" {
			continue
		}
		return a.OccupationID
	}
	return ""
}

// npcListDisplayAndBehaviorRole 同房 UI：在指派場所且（無排班或當班）時為「職稱|真名」，否則僅真名；並回傳行為模板用的職稱 key（下班或離場為空）。
// gameHour ∈ [0,23] 時依排班判斷下班；<0 則不判斷下班（在職場內一律帶職稱前綴）。
func npcListDisplayAndBehaviorRole(entityID, roomID string, gameHour int) (listDisplay string, behaviorRole string) {
	person := GetNPCPersonDisplayName(entityID)
	occ := occupationForEntityAtRoom(entityID, roomID)
	if occ == "" {
		return person, ""
	}
	offDuty := false
	if sch, ok := GetScheduleForEntity(entityID); ok {
		if gameHour >= 0 && gameHour <= 23 && !sch.IsOnDuty(gameHour) {
			offDuty = true
		}
	}
	if offDuty {
		return person, ""
	}
	return occ + "|" + person, occ
}

// GetNPCTitleInRoomAtHour 同房列表／視野用顯示字串（職稱|真名 規則 + 下班僅真名）。
func GetNPCTitleInRoomAtHour(entityID, roomID string, gameHour int) string {
	s, _ := npcListDisplayAndBehaviorRole(entityID, roomID, gameHour)
	return s
}

// GetNPCTitleInRoom 向後相容：不傳遊戲小時（不套用下班規則，在職場內一律「職稱|真名」）。
func GetNPCTitleInRoom(entityID, roomID string) string {
	return GetNPCTitleInRoomAtHour(entityID, roomID, -1)
}

// GetNPCDisplayLabelAtHour 敘事／移動用：需傳遊戲小時才能在職場內正確隱去下班前綴。
func GetNPCDisplayLabelAtHour(entityID string, gameHour int) string {
	roomID, _ := GetEntityRoom(entityID)
	return GetNPCTitleInRoomAtHour(entityID, roomID, gameHour)
}

// GetNPCTitle 無遊戲小時語境時等同 hour=-1（下班規則不生效）。
func GetNPCTitle(entityID string) string {
	roomID, _ := GetEntityRoom(entityID)
	return GetNPCTitleInRoomAtHour(entityID, roomID, -1)
}

// ScheduleTarget 排班目標房間與是否為上班地（供敘事用）。
type ScheduleTarget struct {
	Room   string
	IsWork bool
}

// GetScheduleTargetRoom 依排班與遊戲小時回傳該 NPC 應前往的房間（在班→work_room，下班→rest_room）。
// 若無排班則 ok=false。供排班型移動尋路用，家可遠在十格外也逐格走。
func GetScheduleTargetRoom(entityID string, gameHour int) (targetRoom string, ok bool) {
	t, ok := GetScheduleTarget(entityID, gameHour)
	if !ok {
		return "", false
	}
	return t.Room, true
}

// GetScheduleTarget 回傳排班目標與是否為上班地（IsWork 供抵達敘事用）。
func GetScheduleTarget(entityID string, gameHour int) (t ScheduleTarget, ok bool) {
	if store.Default == nil {
		return ScheduleTarget{}, false
	}
	sch := store.Default.GetSchedule(entityID)
	if sch == nil {
		return ScheduleTarget{}, false
	}
	s := NPCSchedule{
		EntityID: sch.EntityID, WorkRoom: sch.WorkRoom, RestRoom: sch.RestRoom,
		ShiftStart: sch.ShiftStart, ShiftEnd: sch.ShiftEnd,
	}
	if s.IsOnDuty(gameHour) {
		return ScheduleTarget{Room: sch.WorkRoom, IsWork: true}, true
	}
	return ScheduleTarget{Room: sch.RestRoom, IsWork: false}, true
}

// ApplySchedules 根據當前遊戲時間回傳「應前往的房間與敘事用清單」，不傳送（不呼叫 SetEntityRoom）。
// 實際移動由 main 迴圈內 TravelerManager.Tick 對排班型 NPC 逐格尋路並寫回 entity_room；家可遠在十格外。
func ApplySchedules(gameHour int) ([]ScheduleMove, error) {
	schedules, err := GetAllSchedules()
	if err != nil {
		return nil, err
	}
	var moves []ScheduleMove
	for _, s := range schedules {
		ent, _ := GetEntity(s.EntityID)
		if ent == nil || ent.Vit <= 0 {
			continue
		}
		targetRoom := s.RestRoom
		if s.IsOnDuty(gameHour) {
			targetRoom = s.WorkRoom
		}
		currentRoom, _ := GetEntityRoom(s.EntityID)
		if currentRoom != targetRoom {
			moves = append(moves, ScheduleMove{
				EntityID: s.EntityID,
				Title:    GetNPCDisplayLabelAtHour(s.EntityID, gameHour),
				OldRoom:  currentRoom,
				NewRoom:  targetRoom,
			})
		}
	}
	return moves, nil
}
