package server

import (
	"encoding/json"
	"time"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/game"
)

// SendNarrateToRoom 對指定房間內所有在線玩家發送 NarrateMsg。
func SendNarrateToRoom(store *SessionStore, roomID, text string) {
	if text == "" {
		return
	}
	msg, _ := json.Marshal(NarrateMsg{Type: "narrate", Text: text})
	for _, s := range store.AllSessions() {
		rid, _ := db.GetEntityRoom(s.PlayerID)
		if rid == roomID {
			s.Client.Send <- msg
		}
	}
}

// GetPlayerRoomMap 回傳 roomID → true 的 map，表示哪些房間有玩家在場。
func GetPlayerRoomMap(store *SessionStore) map[string]bool {
	m := make(map[string]bool)
	for _, s := range store.AllSessions() {
		rid, _ := db.GetEntityRoom(s.PlayerID)
		if rid != "" {
			m[rid] = true
		}
	}
	return m
}

// RefreshRoomViews 對指定房間內所有在線玩家推送最新房間視野。
func RefreshRoomViews(store *SessionStore, cfg config.Server, roomID string) {
	gh := 12
	if cfg.GameTimeEpochUnix != 0 {
		_, gh, _, _ = game.GameTimeNow(time.Now().Unix(), cfg.GameTimeEpochUnix, cfg.GameTimeScale)
	}
	view, err := game.GetRoomView(roomID, gh)
	if err != nil || view == nil {
		return
	}
	for _, s := range store.AllSessions() {
		rid, _ := db.GetEntityRoom(s.PlayerID)
		if rid == roomID {
			sendRoomView(s.Client, view, cfg)
		}
	}
}
