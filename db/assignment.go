// Package db 負責場所（venues）與指派（assignments），對齊討論 001 身份與職業分離。
package db

import (
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
	AssignedBy   string
}

// SeedVenues 確保預設場所存在；現由 data/venues.json 於 store.Init 載入，此函式為 no-op。
func SeedVenues() error {
	return nil
}

// InsertAssignment 新增一筆指派；已存在則忽略。
func InsertAssignment(entityID, occupationID, venueID, assignedBy string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.InsertAssignment(entityID, occupationID, venueID, assignedBy)
}

// GetAssignmentsForEntity 取得某實體的全部指派（職業＋場所）。
func GetAssignmentsForEntity(entityID string) ([]Assignment, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	slist := store.Default.GetAssignmentsForEntity(entityID)
	list := make([]Assignment, len(slist))
	for i, a := range slist {
		list[i] = Assignment{
			EntityID: a.EntityID, OccupationID: a.OccupationID, VenueID: a.VenueID,
			AssignedBy: a.AssignedBy,
		}
	}
	return list, nil
}

// GetNPCTitleFromAssignments 依指派推導職稱；無指派則回傳空字串。
func GetNPCTitleFromAssignments(entityID string) string {
	list, err := GetAssignmentsForEntity(entityID)
	if err != nil || len(list) == 0 {
		return ""
	}
	return list[0].OccupationID
}

// RemoveAssignmentsForEntity 移除某實體的全部指派（死亡除名用）。
func RemoveAssignmentsForEntity(entityID string) error {
	if store.Default == nil {
		return ErrNoStore
	}
	return store.Default.RemoveAssignmentsForEntity(entityID)
}

// GetVenueMaxStaff 回傳該場所職缺上限；0 用 defaultMax。
func GetVenueMaxStaff(venueID string, defaultMax int) int {
	if store.Default == nil {
		return defaultMax
	}
	return store.Default.GetVenueMaxStaff(venueID, defaultMax)
}

// GetFirstOccupationIDForVenue 回傳該場所既有指派中的第一個職業 ID。
func GetFirstOccupationIDForVenue(venueID string) string {
	if store.Default == nil {
		return ""
	}
	return store.Default.GetFirstOccupationIDForVenue(venueID)
}

// GetRoomIDsForVenue 回傳該場所涵蓋的房間 ID 列表。
func GetRoomIDsForVenue(venueID string) ([]string, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	v := store.Default.GetVenue(venueID)
	if v == nil {
		return nil, nil
	}
	return v.RoomIDs, nil
}

// IsRoomInVenue 判斷房間是否在該場所的 room_ids 內。
func IsRoomInVenue(roomID, venueID string) (bool, error) {
	if store.Default == nil {
		return false, ErrNoStore
	}
	return store.Default.IsRoomInVenue(roomID, venueID), nil
}

// EntityInVenueAtRoom 回傳該實體在指定房間時，是否處於任一指派場所內。
func EntityInVenueAtRoom(entityID, roomID string) (bool, error) {
	list, err := GetAssignmentsForEntity(entityID)
	if err != nil {
		return false, err
	}
	for _, a := range list {
		ok, err := IsRoomInVenue(roomID, a.VenueID)
		if err != nil {
			return false, err
		}
		if ok {
			return true, nil
		}
	}
	return false, nil
}

// GetVenueIDsForRoom 回傳包含該房間的所有場所 ID。
func GetVenueIDsForRoom(roomID string) ([]string, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	return store.Default.GetVenueIDsForRoom(roomID), nil
}

// GetAssignmentCountByVenue 回傳該場所目前的指派數量。
func GetAssignmentCountByVenue(venueID string) (int, error) {
	if store.Default == nil {
		return 0, ErrNoStore
	}
	return store.Default.GetAssignmentCountByVenue(venueID), nil
}

// GetAllVenueIDs 回傳所有場所 ID。
func GetAllVenueIDs() ([]string, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	return store.Default.GetAllVenueIDs(), nil
}

// GetAllVenueRoomIDs 回傳所有場所涵蓋的房間 ID。
func GetAllVenueRoomIDs() ([]string, error) {
	if store.Default == nil {
		return nil, ErrNoStore
	}
	return store.Default.GetAllVenueRoomIDs(), nil
}
