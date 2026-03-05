// Package server 地圖檢視器用 API：驗證玩家身份後回傳當前房間 ID。
package server

import (
	"database/sql"
	"encoding/json"
	"net/http"

	"singularity_world/db"
)

// PlayerRoomResponse 回傳結構。
type PlayerRoomResponse struct {
	PlayerID string `json:"player_id"`
	RoomID   string `json:"room_id"`
}

// HandlePlayerRoomAPI 處理 GET /api/player-room?id=xxx&pw=yyy，驗證密碼後回傳該玩家當前房間 ID。
func HandlePlayerRoomAPI(database *sql.DB, w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	if r.Method != http.MethodGet {
		http.Error(w, `{"error":"GET only"}`, http.StatusMethodNotAllowed)
		return
	}
	playerID := r.URL.Query().Get("id")
	password := r.URL.Query().Get("pw")
	if playerID == "" || password == "" {
		http.Error(w, `{"error":"需提供 id 與 pw 參數"}`, http.StatusBadRequest)
		return
	}
	ok, err := db.VerifyPassword(database, playerID, password)
	if err != nil || !ok {
		http.Error(w, `{"error":"身份驗證失敗"}`, http.StatusForbidden)
		return
	}
	roomID, err := db.GetEntityRoom(database, playerID)
	if err != nil {
		http.Error(w, `{"error":"查詢房間失敗"}`, http.StatusInternalServerError)
		return
	}
	resp := PlayerRoomResponse{
		PlayerID: playerID,
		RoomID:   roomID,
	}
	w.Header().Set("Cache-Control", "no-cache")
	json.NewEncoder(w).Encode(resp)
}
