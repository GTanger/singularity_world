package server

import (
	"database/sql"
	"encoding/json"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/game"
)

// SendNarrateToRoom 對指定房間內所有在線玩家發送 NarrateMsg。
func SendNarrateToRoom(store *SessionStore, database *sql.DB, roomID, text string) {
	if text == "" {
		return
	}
	msg, _ := json.Marshal(NarrateMsg{Type: "narrate", Text: text})
	for _, s := range store.AllSessions() {
		rid, _ := db.GetEntityRoom(database, s.PlayerID)
		if rid == roomID {
			s.Client.Send <- msg
		}
	}
}

// GetPlayerRoomMap 回傳 roomID → true 的 map，表示哪些房間有玩家在場。
func GetPlayerRoomMap(store *SessionStore, database *sql.DB) map[string]bool {
	m := make(map[string]bool)
	for _, s := range store.AllSessions() {
		rid, _ := db.GetEntityRoom(database, s.PlayerID)
		if rid != "" {
			m[rid] = true
		}
	}
	return m
}

// RefreshRoomViews 對指定房間內所有在線玩家推送最新房間視野。
func RefreshRoomViews(store *SessionStore, database *sql.DB, cfg config.Server, roomID string) {
	view, err := game.GetRoomView(database, roomID)
	if err != nil || view == nil {
		return
	}
	for _, s := range store.AllSessions() {
		rid, _ := db.GetEntityRoom(database, s.PlayerID)
		if rid == roomID {
			sendRoomView(s.Client, view, cfg)
		}
	}
}
