// Package db 負責場所（venues）與指派（assignments），對齊討論 001 身份與職業分離。
package db

import (
	"database/sql"
	"encoding/json"

	"singularity_world/store"
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

// SeedVenues 確保預設場所存在（浮生客棧）。store 啟用時改由 data/venues.json 提供，不再寫 DB。
func SeedVenues(db *sql.DB) error {
	if store.Default != nil {
		return nil
	}
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

// InsertAssignment 新增一筆指派；已存在則忽略。store 啟用時寫入 store 並持久化 data/assignments.json。
func InsertAssignment(db *sql.DB, entityID, occupationID, venueID, assignedBy string) error {
	if store.Default != nil {
		return store.Default.InsertAssignment(entityID, occupationID, venueID, assignedBy)
	}
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

// GetAssignmentsForEntity 取得某實體的全部指派（職業＋場所）；store 啟用時從 store 讀取。
func GetAssignmentsForEntity(db *sql.DB, entityID string) ([]Assignment, error) {
	if store.Default != nil {
		slist := store.Default.GetAssignmentsForEntity(entityID)
		list := make([]Assignment, len(slist))
		for i, a := range slist {
			list[i] = Assignment{
				EntityID: a.EntityID, OccupationID: a.OccupationID, VenueID: a.VenueID,
				AssignedBy: sql.NullString{String: a.AssignedBy, Valid: a.AssignedBy != ""},
			}
		}
		return list, nil
	}
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

// RemoveAssignmentsForEntity 移除某實體的全部指派（死亡除名用）。store 啟用時寫入 store 並持久化。
func RemoveAssignmentsForEntity(db *sql.DB, entityID string) error {
	if store.Default != nil {
		return store.Default.RemoveAssignmentsForEntity(entityID)
	}
	_, err := db.Exec("DELETE FROM assignments WHERE entity_id = ?", entityID)
	return err
}

// GetVenueMaxStaff 回傳該場所職缺上限（10.15 細目 2）；store 啟用時從 Venue.MaxStaff 讀取，0 用 defaultMax；無 store 時回傳 defaultMax。
func GetVenueMaxStaff(db *sql.DB, venueID string, defaultMax int) int {
	if store.Default != nil {
		return store.Default.GetVenueMaxStaff(venueID, defaultMax)
	}
	return defaultMax
}

// GetFirstOccupationIDForVenue 回傳該場所既有指派中的第一個職業 ID（10.15 撮合時「場所對應職業」）；無則空字串，呼叫方 fallback DefaultOccupationForHire。
func GetFirstOccupationIDForVenue(db *sql.DB, venueID string) string {
	if store.Default != nil {
		return store.Default.GetFirstOccupationIDForVenue(venueID)
	}
	var occ string
	if err := db.QueryRow("SELECT occupation_id FROM assignments WHERE venue_id = ? LIMIT 1", venueID).Scan(&occ); err != nil {
		return ""
	}
	return occ
}

// GetRoomIDsForVenue 回傳該場所涵蓋的房間 ID 列表；供死亡區域事件廣播用。store 啟用時從 store 讀取。
func GetRoomIDsForVenue(db *sql.DB, venueID string) ([]string, error) {
	if store.Default != nil {
		v := store.Default.GetVenue(venueID)
		if v == nil {
			return nil, nil
		}
		return v.RoomIDs, nil
	}
	var raw string
	if err := db.QueryRow("SELECT room_ids FROM venues WHERE id = ?", venueID).Scan(&raw); err != nil {
		if err == sql.ErrNoRows {
			return nil, nil
		}
		return nil, err
	}
	var ids []string
	if err := json.Unmarshal([]byte(raw), &ids); err != nil {
		return nil, err
	}
	return ids, nil
}

// IsRoomInVenue 判斷房間是否在該場所的 room_ids 內；store 啟用時從 store 讀取。
func IsRoomInVenue(db *sql.DB, roomID, venueID string) (bool, error) {
	if store.Default != nil {
		return store.Default.IsRoomInVenue(roomID, venueID), nil
	}
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

// GetVenueIDsForRoom 回傳包含該房間的所有場所 ID；store 啟用時從 store 讀取。
func GetVenueIDsForRoom(db *sql.DB, roomID string) ([]string, error) {
	if store.Default != nil {
		return store.Default.GetVenueIDsForRoom(roomID), nil
	}
	rows, err := db.Query("SELECT id, room_ids FROM venues")
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []string
	for rows.Next() {
		var id, raw string
		if err := rows.Scan(&id, &raw); err != nil {
			return nil, err
		}
		var ids []string
		if err := json.Unmarshal([]byte(raw), &ids); err != nil {
			continue
		}
		for _, rid := range ids {
			if rid == roomID {
				out = append(out, id)
				break
			}
		}
	}
	return out, rows.Err()
}

// GetAssignmentCountByVenue 回傳該場所目前的指派數量；store 啟用時從 store 讀取。
func GetAssignmentCountByVenue(db *sql.DB, venueID string) (int, error) {
	if store.Default != nil {
		return store.Default.GetAssignmentCountByVenue(venueID), nil
	}
	var n int
	err := db.QueryRow("SELECT COUNT(*) FROM assignments WHERE venue_id = ?", venueID).Scan(&n)
	return n, err
}

// GetAllVenueIDs 回傳所有場所 ID；store 啟用時從 store 讀取。
func GetAllVenueIDs(db *sql.DB) ([]string, error) {
	if store.Default != nil {
		return store.Default.GetAllVenueIDs(), nil
	}
	rows, err := db.Query("SELECT id FROM venues")
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []string
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return nil, err
		}
		out = append(out, id)
	}
	return out, rows.Err()
}

// GetAllVenueRoomIDs 回傳所有場所涵蓋的房間 ID（供決策引擎 SeekJob 意圖尋路用）。store 啟用時從 store 讀取。
func GetAllVenueRoomIDs(db *sql.DB) ([]string, error) {
	if store.Default != nil {
		return store.Default.GetAllVenueRoomIDs(), nil
	}
	rows, err := db.Query("SELECT room_ids FROM venues")
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []string
	seen := make(map[string]bool)
	for rows.Next() {
		var raw string
		if err := rows.Scan(&raw); err != nil {
			return nil, err
		}
		var ids []string
		if err := json.Unmarshal([]byte(raw), &ids); err != nil {
			continue
		}
		for _, id := range ids {
			if !seen[id] {
				seen[id] = true
				out = append(out, id)
			}
		}
	}
	return out, rows.Err()
}
