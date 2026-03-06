// Package db 負責場所（venues）與指派（assignments），對齊討論 001 身份與職業分離。
package db

import (
	"database/sql"
	"encoding/json"
)

// Venue 場所：id、名稱、room_ids（JSON 陣列）。
type Venue struct {
	ID      string
	Name    string
	RoomIDs []string
}

// Assignment 指派：誰、什麼職業、哪個場所。
type Assignment struct {
	EntityID     string
	OccupationID string
	VenueID      string
	AssignedBy   sql.NullString
}

// SeedVenues 確保預設場所存在（浮生客棧）。
func SeedVenues(db *sql.DB) error {
	roomIDs := lifeInnRoomIDs()
	raw, _ := json.Marshal(roomIDs)
	_, err := db.Exec(
		`INSERT OR IGNORE INTO venues (id, name, room_ids) VALUES (?, ?, ?)`,
		"venue_life_inn", "浮生客棧", string(raw),
	)
	return err
}

func lifeInnRoomIDs() []string {
	return []string{
		"life_garden", "life_hall", "life_dining", "life_kitchen", "life_backyard",
		"life_storage", "life_wine_cellar", "life_corridor_2f", "life_corridor_3f",
		"life_ri_1", "life_ri_2", "life_yue_1", "life_yue_2", "life_ying_1", "life_ying_2",
		"life_ze_1", "life_ze_2", "life_tian_1", "life_tian_2", "life_di_1", "life_di_2",
		"life_xuan_1", "life_xuan_2", "life_huang_1", "life_huang_2",
	}
}

// InsertAssignment 新增一筆指派；已存在則忽略。assignedBy 可空。
func InsertAssignment(db *sql.DB, entityID, occupationID, venueID, assignedBy string) error {
	var ab interface{}
	if assignedBy != "" {
		ab = assignedBy
	} else {
		ab = nil
	}
	_, err := db.Exec(
		`INSERT OR IGNORE INTO assignments (entity_id, occupation_id, venue_id, assigned_by) VALUES (?, ?, ?, ?)`,
		entityID, occupationID, venueID, ab,
	)
	return err
}

// GetAssignmentsForEntity 取得某實體的全部指派（職業＋場所）。
func GetAssignmentsForEntity(db *sql.DB, entityID string) ([]Assignment, error) {
	rows, err := db.Query(
		`SELECT entity_id, occupation_id, venue_id, assigned_by FROM assignments WHERE entity_id = ?`,
		entityID,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var list []Assignment
	for rows.Next() {
		var a Assignment
		var ab sql.NullString
		if err := rows.Scan(&a.EntityID, &a.OccupationID, &a.VenueID, &ab); err != nil {
			return nil, err
		}
		if ab.Valid {
			a.AssignedBy = ab
		}
		list = append(list, a)
	}
	return list, rows.Err()
}

// GetNPCTitleFromAssignments 依指派推導職稱；無指派則回傳空字串。
func GetNPCTitleFromAssignments(db *sql.DB, entityID string) string {
	list, err := GetAssignmentsForEntity(db, entityID)
	if err != nil || len(list) == 0 {
		return ""
	}
	return list[0].OccupationID
}

// IsRoomInVenue 判斷房間是否在該場所的 room_ids 內。
func IsRoomInVenue(db *sql.DB, roomID, venueID string) (bool, error) {
	var raw string
	if err := db.QueryRow("SELECT room_ids FROM venues WHERE id = ?", venueID).Scan(&raw); err != nil {
		if err == sql.ErrNoRows {
			return false, nil
		}
		return false, err
	}
	var ids []string
	if err := json.Unmarshal([]byte(raw), &ids); err != nil {
		return false, err
	}
	for _, id := range ids {
		if id == roomID {
			return true, nil
		}
	}
	return false, nil
}

// EntityInVenueAtRoom 回傳該實體在指定房間時，是否處於任一指派場所內（用於動作模板是否生效）。
func EntityInVenueAtRoom(db *sql.DB, entityID, roomID string) (bool, error) {
	list, err := GetAssignmentsForEntity(db, entityID)
	if err != nil {
		return false, err
	}
	for _, a := range list {
		ok, err := IsRoomInVenue(db, roomID, a.VenueID)
		if err != nil {
			return false, err
		}
		if ok {
			return true, nil
		}
	}
	return false, nil
}
