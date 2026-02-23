// Package game 傳統 MUD 房間視野與依出口移動，節點連接節點。
package game

import (
	"database/sql"

	"singularity_world/db"
	"singularity_world/entity"
)

// RoomView 當前房間的視野：房間資訊、出口列表、同房實體。
type RoomView struct {
	Room     db.Room
	Exits    []db.Exit
	Entities []*entity.Character
}

// GetRoomView 依房間 id 載入房間描述、出口與同房實體。
func GetRoomView(database *sql.DB, roomID string) (*RoomView, error) {
	room, err := db.GetRoom(database, roomID)
	if err != nil || room == nil {
		return nil, err
	}
	exits, err := db.GetExitsForRoom(database, roomID)
	if err != nil {
		return nil, err
	}
	entities, err := db.GetEntitiesInRoom(database, roomID)
	if err != nil {
		return nil, err
	}
	return &RoomView{Room: *room, Exits: exits, Entities: entities}, nil
}

// MoveByExit 將實體依出口方向移動到相鄰房間。回傳新房間 id 與 ok；若出口不存在或錯誤則 ok=false。
func MoveByExit(database *sql.DB, entityID, direction string) (newRoomID string, ok bool, err error) {
	roomID, err := db.GetEntityRoom(database, entityID)
	if err != nil || roomID == "" {
		return "", false, err
	}
	exits, err := db.GetExitsForRoom(database, roomID)
	if err != nil {
		return "", false, err
	}
	for _, ex := range exits {
		if ex.Direction == direction {
			if err := db.SetEntityRoom(database, entityID, ex.ToRoomID); err != nil {
				return "", false, err
			}
			return ex.ToRoomID, true, nil
		}
	}
	return "", false, nil
}

// EnsureEntityInRoom 若實體尚無房間則設為預設房間，並回傳其房間 id。
func EnsureEntityInRoom(database *sql.DB, entityID, defaultRoomID string) (roomID string, err error) {
	roomID, err = db.GetEntityRoom(database, entityID)
	if err != nil {
		return "", err
	}
	if roomID == "" {
		if err := db.SetEntityRoom(database, entityID, defaultRoomID); err != nil {
			return "", err
		}
		roomID = defaultRoomID
	}
	return roomID, nil
}
