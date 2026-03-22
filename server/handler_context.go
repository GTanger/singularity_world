// Package server — 008 P5：WebSocket 共用脈絡（遊戲時刻、觀測者、JSON 與全服視野同步）。
package server

import (
	"encoding/json"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/game"
)

// 創生預設格：以房間名稱「界壁」解析 id，見 db.GetSpawnRoomID。

func currentGameHour(cfg config.Server) int {
	if cfg.GameTimeEpochUnix == 0 {
		return 12
	}
	_, h, _, _ := game.GameTimeNow(game.NowUnix(), cfg.GameTimeEpochUnix, cfg.GameTimeScale)
	return h
}

// defaultObserver 供 §七 7.1：房間制觀測改經 Observer 介面；main 啟動時 SetDefaultObserver(obs)。
var defaultObserver game.Observer

// SetDefaultObserver 設定全域 Observer，sendRoomView 會對同房 NPC 呼叫 OnObserve（若非 nil）；否則沿用 ObserveRoom。
func SetDefaultObserver(o game.Observer) { defaultObserver = o }

func sendError(c *Client, message string) {
	c.Send <- mustJSON(ErrorMsg{Type: "error", Message: message})
}

func mustJSON(v interface{}) []byte {
	b, _ := json.Marshal(v)
	return b
}

// GetObserverPositions 回傳目前所有已登入玩家的世界座標（格點制時供 RunViewSimulation 用；房間制可留空）。
func GetObserverPositions(store *SessionStore) []game.Pos {
	return nil
}

// BroadcastRoomViews 對所有在線玩家推送其當前房間的最新視野。
// 用於 NPC 排班移動後同步前端人物欄。
func BroadcastRoomViews(store *SessionStore, cfg config.Server) {
	for _, s := range store.AllSessions() {
		roomID, _ := db.GetEntityRoom(s.PlayerID)
		if roomID == "" {
			continue
		}
		view, err := game.GetRoomView(roomID, currentGameHour(cfg))
		if err != nil || view == nil {
			continue
		}
		sendRoomView(s.Client, view, cfg)
	}
}
