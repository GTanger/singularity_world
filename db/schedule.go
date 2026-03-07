package db

import "database/sql"

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

// GetAllSchedules 取得所有 NPC 排班。
func GetAllSchedules(db *sql.DB) ([]NPCSchedule, error) {
	rows, err := db.Query("SELECT entity_id, work_room, rest_room, shift_start, shift_end FROM npc_schedules")
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var list []NPCSchedule
	for rows.Next() {
		var s NPCSchedule
		if err := rows.Scan(&s.EntityID, &s.WorkRoom, &s.RestRoom, &s.ShiftStart, &s.ShiftEnd); err != nil {
			return nil, err
		}
		list = append(list, s)
	}
	return list, rows.Err()
}

// ScheduleMove 記錄一次排班移動：誰從哪到哪、職稱，供外部推送敘事用。
type ScheduleMove struct {
	EntityID string
	Title    string
	OldRoom  string
	NewRoom  string
}

// GetNPCTitle 依討論 001：先自指派（assignments）推導職稱，無指派時 fallback 查 entities.display_title。
func GetNPCTitle(db *sql.DB, entityID string) string {
	if t := GetNPCTitleFromAssignments(db, entityID); t != "" {
		return t
	}
	var title sql.NullString
	_ = db.QueryRow("SELECT display_title FROM entities WHERE id = ?", entityID).Scan(&title)
	return title.String
}

// ScheduleTarget 排班目標房間與是否為上班地（供敘事用）。
type ScheduleTarget struct {
	Room   string
	IsWork bool
}

// GetScheduleTargetRoom 依排班與遊戲小時回傳該 NPC 應前往的房間（在班→work_room，下班→rest_room）。
// 若無排班則 ok=false。供排班型移動尋路用，家可遠在十格外也逐格走。
func GetScheduleTargetRoom(database *sql.DB, entityID string, gameHour int) (targetRoom string, ok bool) {
	t, ok := GetScheduleTarget(database, entityID, gameHour)
	if !ok {
		return "", false
	}
	return t.Room, true
}

// GetScheduleTarget 回傳排班目標與是否為上班地（IsWork 供抵達敘事用）。
// 僅查 npc_schedules 表，不寫入 DB；與 ApplySchedules「只回傳不傳送」一致，實際移動由 TravelerManager 排班型 Tick 執行。
func GetScheduleTarget(database *sql.DB, entityID string, gameHour int) (t ScheduleTarget, ok bool) {
	rows, err := database.Query(
		"SELECT work_room, rest_room, shift_start, shift_end FROM npc_schedules WHERE entity_id = ?",
		entityID,
	)
	if err != nil {
		return ScheduleTarget{}, false
	}
	defer rows.Close()
	if !rows.Next() {
		return ScheduleTarget{}, false
	}
	var workRoom, restRoom string
	var shiftStart, shiftEnd int
	if err := rows.Scan(&workRoom, &restRoom, &shiftStart, &shiftEnd); err != nil {
		return ScheduleTarget{}, false
	}
	s := NPCSchedule{EntityID: entityID, WorkRoom: workRoom, RestRoom: restRoom, ShiftStart: shiftStart, ShiftEnd: shiftEnd}
	if s.IsOnDuty(gameHour) {
		return ScheduleTarget{Room: workRoom, IsWork: true}, true
	}
	return ScheduleTarget{Room: restRoom, IsWork: false}, true
}

// ApplySchedules 根據當前遊戲時間回傳「應前往的房間與敘事用清單」，不傳送（不呼叫 SetEntityRoom）。
// 實際移動由 main 迴圈內 TravelerManager.Tick 對排班型 NPC 逐格尋路並寫回 entity_room；家可遠在十格外。
func ApplySchedules(database *sql.DB, gameHour int) ([]ScheduleMove, error) {
	schedules, err := GetAllSchedules(database)
	if err != nil {
		return nil, err
	}
	var moves []ScheduleMove
	for _, s := range schedules {
		targetRoom := s.RestRoom
		if s.IsOnDuty(gameHour) {
			targetRoom = s.WorkRoom
		}
		currentRoom, _ := GetEntityRoom(database, s.EntityID)
		if currentRoom != targetRoom {
			moves = append(moves, ScheduleMove{
				EntityID: s.EntityID,
				Title:    GetNPCTitle(database, s.EntityID),
				OldRoom:  currentRoom,
				NewRoom:  targetRoom,
			})
		}
	}
	return moves, nil
}
