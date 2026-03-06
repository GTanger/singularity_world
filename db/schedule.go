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

// ApplySchedules 根據當前遊戲時間移動 NPC 到對應房間，並回傳實際發生移動的清單。
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
			_ = SetEntityRoom(database, s.EntityID, targetRoom)
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
