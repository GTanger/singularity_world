// Package game 傳統 MUD 房間視野與依出口移動，節點連接節點。
package game

import (
	"strings"

	"singularity_world/db"
	"singularity_world/entity"
)

// RoomView 當前房間的視野：房間資訊、出口列表、同房實體、同房可互動物件。
type RoomView struct {
	Room     db.Room
	Exits    []db.Exit
	Entities []*entity.Character
	Objects  []db.RoomObject
}

// GetRoomView 依房間 id 載入房間描述、出口、同房實體與同房物件。
// gameHour 為當前遊戲小時 0–23（NPC 列表「職稱|真名」與下班規則）；未知時傳 -1。
func GetRoomView(roomID string, gameHour int) (*RoomView, error) {
	room, err := db.GetRoom(roomID)
	if err != nil || room == nil {
		return nil, err
	}
	exits, err := db.GetExitsForRoom(roomID)
	if err != nil {
		return nil, err
	}
	entities, err := db.GetEntitiesInRoom(roomID, gameHour)
	if err != nil {
		return nil, err
	}
	objects := db.GetObjectsInRoom(roomID)
	return &RoomView{Room: *room, Exits: exits, Entities: entities, Objects: objects}, nil
}

// MoveByExit 將實體依出口方向移動到相鄰房間。回傳新房間 id 與 ok；若出口不存在或錯誤則 ok=false。
// 比對順序：1) exits（direction 與客端字串皆 TrimSpace）；2) 同房可 Move 物件，object.Name 與 direction 相同且具 move_to_room_id
// （供民居客廳內門與大廳戶碼等：即使 exits 未載入或與記憶體不同步，仍與出口按鈕／物件名一致時可移動）。
func MoveByExit(entityID, direction string) (newRoomID string, ok bool, err error) {
	dir := strings.TrimSpace(direction)
	if dir == "" {
		return "", false, nil
	}
	roomID, err := db.GetEntityRoom(entityID)
	if err != nil || roomID == "" {
		return "", false, err
	}
	exits, err := db.GetExitsForRoom(roomID)
	if err != nil {
		return "", false, err
	}
	for _, ex := range exits {
		if strings.TrimSpace(ex.Direction) == dir {
			if err := db.SetEntityRoom(entityID, ex.ToRoomID); err != nil {
				return "", false, err
			}
			return ex.ToRoomID, true, nil
		}
	}
	objs := db.GetObjectsInRoom(roomID)
	for i := range objs {
		o := &objs[i]
		if strings.TrimSpace(o.Name) != dir {
			continue
		}
		if o.MoveToRoomID == "" || !db.ObjectHasSocket(o, "Move") {
			continue
		}
		if err := db.SetEntityRoom(entityID, o.MoveToRoomID); err != nil {
			return "", false, err
		}
		return o.MoveToRoomID, true, nil
	}
	return "", false, nil
}

// EnsureEntityInRoom 若實體尚無房間則設為預設房間，並回傳其房間 id。
func EnsureEntityInRoom(entityID, defaultRoomID string) (roomID string, err error) {
	roomID, err = db.GetEntityRoom(entityID)
	if err != nil {
		return "", err
	}
	if roomID == "" {
		if err := db.SetEntityRoom(entityID, defaultRoomID); err != nil {
			return "", err
		}
		roomID = defaultRoomID
	}
	return roomID, nil
}
