// Package server 提供地圖檢視器用：從 store 彙總房間與出口，回傳與舊版 data/rooms.json 同格式的 JSON。
package server

import (
	"encoding/json"
	"net/http"

	"singularity_world/store"
)

// roomsDataResponse 與 map_viewer 期望的格式一致：rooms 陣列 + exits 陣列。
type roomsDataResponse struct {
	Rooms []roomDataItem  `json:"rooms"`
	Exits []exitDataItem  `json:"exits"`
}
type roomDataItem struct {
	ID          string   `json:"id"`
	Name        string   `json:"name"`
	Description string   `json:"description"`
	Tags        []string `json:"tags"`
	Zone        string   `json:"zone"`
}
type exitDataItem struct {
	From      string `json:"from"`
	Direction string `json:"direction"`
	To        string `json:"to"`
}

// HandleRoomsDataAPI 處理 GET /data/rooms.json：從 store 彙總所有房間與出口，回傳給 map_viewer 使用。
func HandleRoomsDataAPI(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.Header().Set("Cache-Control", "no-cache, max-age=0, must-revalidate")
	if r.Method != http.MethodGet {
		http.Error(w, `{"error":"GET only"}`, http.StatusMethodNotAllowed)
		return
	}
	if store.Default == nil {
		http.Error(w, `{"error":"store not initialized"}`, http.StatusServiceUnavailable)
		return
	}
	ids := store.Default.RoomIDs()
	rooms := make([]roomDataItem, 0, len(ids))
	exits := make([]exitDataItem, 0, 256)
	for _, id := range ids {
		room, err := store.Default.GetRoom(id)
		if err != nil || room == nil {
			continue
		}
		rooms = append(rooms, roomDataItem{
			ID: room.ID, Name: room.Name, Description: room.Description,
			Tags: room.Tags, Zone: room.Zone,
		})
		exList, _ := store.Default.GetExitsForRoom(id)
		for _, e := range exList {
			exits = append(exits, exitDataItem{
				From: id, Direction: e.Direction, To: e.ToRoomID,
			})
		}
	}
	resp := roomsDataResponse{Rooms: rooms, Exits: exits}
	enc := json.NewEncoder(w)
	enc.SetEscapeHTML(false)
	_ = enc.Encode(resp)
}
