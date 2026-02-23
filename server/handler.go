// Package server 處理 WebSocket 客戶端訊息：登入、載入房間視野、依出口移動。傳統 MUD 節點連接節點。
package server

import (
	"database/sql"
	"encoding/json"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/event"
	"singularity_world/game"
)

const defaultRoomID = "lobby"

// HandleMessage 解析客戶端 JSON 並執行 login 或 move；傳入 sessionStore 與 hub 以綁定 session 與廣播。
func HandleMessage(c *Client, raw []byte, database *sql.DB, cfg config.Server, store *SessionStore, hub *Hub) {
	var msg ClientMsg
	if err := json.Unmarshal(raw, &msg); err != nil {
		sendError(c, "invalid json")
		return
	}
	switch msg.Type {
	case "login":
		handleLogin(c, &msg, database, cfg, store, hub)
	case "move":
		handleMove(c, &msg, database, cfg, store, hub)
	default:
		sendError(c, "unknown type: "+msg.Type)
	}
}

func handleLogin(c *Client, msg *ClientMsg, database *sql.DB, cfg config.Server, store *SessionStore, hub *Hub) {
	if msg.PlayerID == "" {
		sendError(c, "player_id required")
		return
	}
	ent, err := db.GetEntity(database, msg.PlayerID)
	if err != nil || ent == nil {
		sendError(c, "player not found")
		return
	}
	_ = ent
	roomID, err := game.EnsureEntityInRoom(database, msg.PlayerID, defaultRoomID)
	if err != nil {
		sendError(c, "room failed")
		return
	}
	c.PlayerID = msg.PlayerID
	store.Set(msg.PlayerID, &Session{Client: c, PlayerID: msg.PlayerID})

	view, err := game.GetRoomView(database, roomID)
	if err != nil {
		sendError(c, "load room failed")
		return
	}
	sendRoomView(c, view)
	sendMe(c, msg.PlayerID, roomID, view.Room.Name)
}

func handleMove(c *Client, msg *ClientMsg, database *sql.DB, cfg config.Server, store *SessionStore, hub *Hub) {
	if c.PlayerID == "" {
		sendError(c, "login first")
		return
	}
	if msg.Direction == "" {
		sendError(c, "direction required")
		return
	}
	newRoomID, ok, err := game.MoveByExit(database, c.PlayerID, msg.Direction)
	if err != nil {
		sendError(c, "move failed")
		return
	}
	if !ok {
		now := game.NowUnix()
		_ = event.Append(database, now, c.PlayerID, event.TypeBlocked, msg.Direction)
		c.Send <- mustJSON(BlockedMsg{Type: "blocked", Direction: msg.Direction})
		return
	}
	view, err := game.GetRoomView(database, newRoomID)
	if err != nil {
		sendError(c, "load room failed")
		return
	}
	sendRoomView(c, view)
	hub.Broadcast(mustJSON(MovedMsg{Type: "moved", PlayerID: c.PlayerID, RoomID: newRoomID, RoomName: view.Room.Name}))
}

func sendRoomView(c *Client, view *game.RoomView) {
	entities := make([]ViewEntity, 0, len(view.Entities))
	for _, e := range view.Entities {
		entities = append(entities, ViewEntity{ID: e.ID, Kind: e.Kind, DisplayChar: e.DisplayChar})
	}
	exits := make([]ExitView, 0, len(view.Exits))
	for _, ex := range view.Exits {
		exits = append(exits, ExitView{Direction: ex.Direction, ToRoomID: ex.ToRoomID, ToRoomName: ex.ToRoomName})
	}
	msg := RoomViewMsg{
		Type:        "view",
		RoomID:      view.Room.ID,
		RoomName:    view.Room.Name,
		Description: view.Room.Description,
		Exits:       exits,
		Entities:    entities,
	}
	c.Send <- mustJSON(msg)
}

func sendMe(c *Client, playerID, roomID, roomName string) {
	c.Send <- mustJSON(MeMsg{Type: "me", PlayerID: playerID, RoomID: roomID, RoomName: roomName})
}

func sendMoved(c *Client, playerID, roomID, roomName string) {
	c.Send <- mustJSON(MovedMsg{Type: "moved", PlayerID: playerID, RoomID: roomID, RoomName: roomName})
}

func sendError(c *Client, message string) {
	c.Send <- mustJSON(ErrorMsg{Type: "error", Message: message})
}

func mustJSON(v interface{}) []byte {
	b, _ := json.Marshal(v)
	return b
}

// GetObserverPositions 回傳目前所有已登入玩家的世界座標（格點制時供 RunViewSimulation 用；房間制可留空）。
func GetObserverPositions(store *SessionStore, database *sql.DB) []game.Pos {
	return nil
}
