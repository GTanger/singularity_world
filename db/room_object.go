// Package db 房間非人物件：從 room_objects.json 載入並快取，供 GetRoomView 與 do_action 使用。
package db

import (
	"encoding/json"
	"log"
	"os"
	"sync"
)

// RoomObject 單一房間物件的定義（與 data/room_objects.json 對應）。
type RoomObject struct {
	ID        string            `json:"id"`
	Name      string            `json:"name"`
	Owner     string            `json:"owner"`
	Sockets   []string          `json:"sockets"`
	Responses map[string]string `json:"responses"`
}

var (
	roomObjectOnce   sync.Once
	objectsByRoom    map[string][]RoomObject // roomID -> list
	objectByID       map[string]*RoomObject  // objectID -> object
	objectRoomByID   map[string]string       // objectID -> roomID
)

// LoadRoomObjects 讀取並快取 room_objects.json；首次呼叫後即從快取返回。啟動時呼叫一次即可。
func LoadRoomObjects(path string) {
	roomObjectOnce.Do(func() {
		data, err := os.ReadFile(path)
		if err != nil {
			log.Printf("[room_object] load %s failed: %v", path, err)
			objectsByRoom = make(map[string][]RoomObject)
			objectByID = make(map[string]*RoomObject)
			objectRoomByID = make(map[string]string)
			return
		}
		var raw map[string]json.RawMessage
		if err := json.Unmarshal(data, &raw); err != nil {
			log.Printf("[room_object] parse %s failed: %v", path, err)
			objectsByRoom = make(map[string][]RoomObject)
			objectByID = make(map[string]*RoomObject)
			objectRoomByID = make(map[string]string)
			return
		}
		objectsByRoom = make(map[string][]RoomObject)
		objectByID = make(map[string]*RoomObject)
		objectRoomByID = make(map[string]string)
		for roomID, rawVal := range raw {
			if len(roomID) > 0 && roomID[0] == '_' {
				continue
			}
			var list []RoomObject
			if err := json.Unmarshal(rawVal, &list); err != nil {
				log.Printf("[room_object] skip room %s: %v", roomID, err)
				continue
			}
			objectsByRoom[roomID] = list
			for i := range list {
				obj := &list[i]
				objectByID[obj.ID] = obj
				objectRoomByID[obj.ID] = roomID
			}
		}
		log.Printf("[room_object] loaded %d objects in %d rooms from %s", len(objectByID), len(objectsByRoom), path)
	})
}

// GetObjectsInRoom 回傳指定房間內的所有可互動物件。
func GetObjectsInRoom(roomID string) []RoomObject {
	if objectsByRoom == nil {
		return nil
	}
	return objectsByRoom[roomID]
}

// GetObjectAndRoom 依物件 ID 回傳物件與其所屬房間 ID；若不存在則回傳 nil, ""。
func GetObjectAndRoom(objectID string) (*RoomObject, string) {
	if objectByID == nil || objectRoomByID == nil {
		return nil, ""
	}
	obj := objectByID[objectID]
	roomID := objectRoomByID[objectID]
	if obj == nil {
		return nil, ""
	}
	return obj, roomID
}

// GetObjectByNameInRoom 在指定房間內依顯示名稱找物件（前端可能送名稱而非 ID 時用）。
func GetObjectByNameInRoom(roomID, name string) (*RoomObject, string) {
	if objectsByRoom == nil || name == "" {
		return nil, ""
	}
	list := objectsByRoom[roomID]
	for i := range list {
		if list[i].Name == name {
			return &list[i], roomID
		}
	}
	return nil, ""
}

// ObjectHasSocket 檢查物件是否具備指定動詞插座。
func ObjectHasSocket(obj *RoomObject, action string) bool {
	if obj == nil {
		return false
	}
	for _, s := range obj.Sockets {
		if s == action {
			return true
		}
	}
	return false
}

// ObjectResponse 回傳物件對某動詞的敘事文字；若無則回傳空字串。
func ObjectResponse(obj *RoomObject, action string) string {
	if obj == nil || obj.Responses == nil {
		return ""
	}
	return obj.Responses[action]
}
