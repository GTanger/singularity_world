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

// ApplySchedules 根據當前遊戲時間移動 NPC 到對應房間（上班→工作房間，下班→休息房間）。
func ApplySchedules(db *sql.DB, gameHour int) error {
	schedules, err := GetAllSchedules(db)
	if err != nil {
		return err
	}
	for _, s := range schedules {
		targetRoom := s.RestRoom
		if s.IsOnDuty(gameHour) {
			targetRoom = s.WorkRoom
		}
		currentRoom, _ := GetEntityRoom(db, s.EntityID)
		if currentRoom != targetRoom {
			_ = SetEntityRoom(db, s.EntityID, targetRoom)
		}
	}
	return nil
}
