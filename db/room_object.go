// Package db 房間非人物件：由各房間 JSON 的 objects 欄位載入並快取，供 GetRoomView 與 do_action 使用。
package db

import (
	"sync"

	"singularity_world/model"
)

// RoomObject 與 model.RoomObject 一致，供 db 層快取與查詢。
type RoomObject = model.RoomObject

var (
	objectsByRoom  map[string][]RoomObject // roomID -> list
	objectByID     map[string]*RoomObject  // objectID -> object
	objectRoomByID map[string]string       // objectID -> roomID
	roomObjectMu   sync.RWMutex
)

// SetObjectsForRoom 設定某房間的可互動物件（來自該房間 JSON 的 objects 欄位）。
func SetObjectsForRoom(roomID string, objects []RoomObject) {
	roomObjectMu.Lock()
	defer roomObjectMu.Unlock()
	if objectsByRoom == nil {
		objectsByRoom = make(map[string][]RoomObject)
	}
	if objectByID == nil {
		objectByID = make(map[string]*RoomObject)
	}
	if objectRoomByID == nil {
		objectRoomByID = make(map[string]string)
	}
	for _, obj := range objectsByRoom[roomID] {
		delete(objectByID, obj.ID)
		delete(objectRoomByID, obj.ID)
	}
	objectsByRoom[roomID] = objects
	for i := range objects {
		obj := &objects[i]
		objectByID[obj.ID] = obj
		objectRoomByID[obj.ID] = roomID
	}
}

// GetObjectsInRoom 回傳指定房間內的所有可互動物件（來自各房間 JSON 的 objects 欄位）。
func GetObjectsInRoom(roomID string) []RoomObject {
	roomObjectMu.RLock()
	defer roomObjectMu.RUnlock()
	if objectsByRoom == nil {
		return nil
	}
	return objectsByRoom[roomID]
}

// GetObjectAndRoom 依物件 ID 回傳物件與其所屬房間 ID；若不存在則回傳 nil, ""。
func GetObjectAndRoom(objectID string) (*RoomObject, string) {
	roomObjectMu.RLock()
	defer roomObjectMu.RUnlock()
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
	roomObjectMu.RLock()
	defer roomObjectMu.RUnlock()
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
