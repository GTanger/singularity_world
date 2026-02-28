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
	case "create_character":
		handleCreateCharacter(c, &msg, database, cfg, store, hub)
	case "move":
		handleMove(c, &msg, database, cfg, store, hub)
	case "ping":
		c.Send <- mustJSON(PongMsg{Type: "pong"})
	case "get_entity_status":
		handleGetEntityStatus(c, &msg, database)
	default:
		sendError(c, "unknown type: "+msg.Type)
	}
}

func handleLogin(c *Client, msg *ClientMsg, database *sql.DB, cfg config.Server, store *SessionStore, hub *Hub) {
	if msg.PlayerID == "" {
		sendError(c, "請輸入角色 ID")
		return
	}
	if msg.Password == "" {
		sendError(c, "請輸入密碼")
		return
	}
	ent, err := db.GetEntity(database, msg.PlayerID)
	if err != nil || ent == nil {
		sendError(c, "角色不存在，請先創角")
		return
	}
	if ent.Kind != "player" {
		sendError(c, "此 ID 非玩家角色")
		return
	}
	ok, err := db.VerifyPassword(database, msg.PlayerID, msg.Password)
	if err != nil {
		sendError(c, "驗證失敗")
		return
	}
	if !ok {
		sendError(c, "密碼錯誤")
		return
	}
	loginSuccess(c, msg.PlayerID, database, cfg, store)
}

func handleCreateCharacter(c *Client, msg *ClientMsg, database *sql.DB, cfg config.Server, store *SessionStore, hub *Hub) {
	if msg.PlayerID == "" {
		sendError(c, "請輸入角色 ID")
		return
	}
	if msg.Password == "" {
		sendError(c, "請設定密碼")
		return
	}
	if len(msg.Password) < 6 {
		sendError(c, "密碼至少 6 個字元")
		return
	}
	if len(msg.PlayerID) < 2 || len(msg.PlayerID) > 32 {
		sendError(c, "ID 請 2～32 字元")
		return
	}
	existing, err := db.GetEntity(database, msg.PlayerID)
	if err != nil {
		sendError(c, "建立失敗")
		return
	}
	if existing != nil {
		sendError(c, "此 ID 已被使用")
		return
	}
	displayChar := msg.DisplayChar
	if displayChar == "" {
		displayChar = "我"
	}
	if len([]rune(displayChar)) > 1 {
		displayChar = string([]rune(displayChar)[:1])
	}
	gender := "M"
	if msg.Gender == "女" {
		gender = "F"
	} else if msg.Gender == "男" {
		gender = "M"
	}
	if err := db.InsertEntity(database, msg.PlayerID, displayChar, gender); err != nil {
		sendError(c, "建立角色失敗")
		return
	}
	if err := db.SetEntityRoom(database, msg.PlayerID, defaultRoomID); err != nil {
		sendError(c, "放入房間失敗")
		return
	}
	if err := db.CreateAuth(database, msg.PlayerID, msg.Password); err != nil {
		sendError(c, "設定密碼失敗")
		return
	}
	loginSuccess(c, msg.PlayerID, database, cfg, store)
}

func loginSuccess(c *Client, playerID string, database *sql.DB, cfg config.Server, store *SessionStore) {
	roomID, err := game.EnsureEntityInRoom(database, playerID, defaultRoomID)
	if err != nil {
		sendError(c, "房間載入失敗")
		return
	}
	c.PlayerID = playerID
	store.Set(playerID, &Session{Client: c, PlayerID: playerID})
	view, err := game.GetRoomView(database, roomID)
	if err != nil {
		sendError(c, "載入視野失敗")
		return
	}
	ent, _ := db.GetEntity(database, playerID)
	vit, qi, dex := 10, 10, 10
	if ent != nil {
		vit, qi, dex = ent.Vit, ent.Qi, ent.Dex
	}
	rm := db.ComputeResourceMaxes(vit, qi, dex)
	sendRoomView(c, view, cfg)
	sendMe(c, playerID, roomID, view.Room.Name, vit, qi, dex, rm)
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
	sendRoomView(c, view, cfg)
	hub.Broadcast(mustJSON(MovedMsg{Type: "moved", PlayerID: c.PlayerID, RoomID: newRoomID, RoomName: view.Room.Name}))
}

func sendRoomView(c *Client, view *game.RoomView, cfg config.Server) {
	entities := make([]ViewEntity, 0, len(view.Entities))
	for _, e := range view.Entities {
		entities = append(entities, ViewEntity{ID: e.ID, Kind: e.Kind, DisplayChar: e.DisplayChar})
	}
	exits := make([]ExitView, 0, len(view.Exits))
	for _, ex := range view.Exits {
		exits = append(exits, ExitView{Direction: ex.Direction, ToRoomID: ex.ToRoomID, ToRoomName: ex.ToRoomName})
	}
	now := game.NowUnix()
	secSinceMidnight, _, _, daysSinceEpoch := game.GameTimeNow(now, cfg.GameTimeEpochUnix, cfg.GameTimeScale)
	msg := RoomViewMsg{
		Type:                     "view",
		RoomID:                   view.Room.ID,
		RoomName:                 view.Room.Name,
		Description:              view.Room.Description,
		Exits:                    exits,
		Entities:                 entities,
		ServerUnix:               now,
		GameTimeSecSinceMidnight: secSinceMidnight,
		GameDaysSinceEpoch:       daysSinceEpoch,
	}
	c.Send <- mustJSON(msg)
}

func sendMe(c *Client, playerID, roomID, roomName string, vit, qi, dex int, rm db.ResourceMaxes) {
	c.Send <- mustJSON(MeMsg{
		Type: "me", PlayerID: playerID, RoomID: roomID, RoomName: roomName,
		Vit: vit, Qi: qi, Dex: dex,
		HpCur: int(rm.HpCur), HpMax: int(rm.HpMax),
		InnerCur: int(rm.InnerCur), InnerMax: int(rm.InnerMax),
		SpiritCur: int(rm.SpiritCur), SpiritMax: int(rm.SpiritMax),
		StaminaCur: int(rm.StaminaCur), StaminaMax: int(rm.StaminaMax),
	})
}

func sendMoved(c *Client, playerID, roomID, roomName string) {
	c.Send <- mustJSON(MovedMsg{Type: "moved", PlayerID: playerID, RoomID: roomID, RoomName: roomName})
}

func handleGetEntityStatus(c *Client, msg *ClientMsg, database *sql.DB) {
	if c.PlayerID == "" {
		sendError(c, "請先登入")
		return
	}
	entityID := msg.EntityID
	if entityID == "" {
		entityID = c.PlayerID
	}
	ent, err := db.GetEntity(database, entityID)
	if err != nil || ent == nil {
		sendError(c, "找不到該角色")
		return
	}
	isSelf := entityID == c.PlayerID
	mag := ent.Magnesium
	var magPtr *int
	if isSelf {
		magPtr = &mag
	}
	rm := db.ComputeResourceMaxes(ent.Vit, ent.Qi, ent.Dex)
	c.Send <- mustJSON(EntityStatusMsg{
		Type:        "entity_status",
		EntityID:    ent.ID,
		DisplayChar: ent.DisplayChar,
		Vit:         ent.Vit,
		Qi:          ent.Qi,
		Dex:         ent.Dex,
		HpCur:       int(rm.HpCur), HpMax: int(rm.HpMax),
		InnerCur:    int(rm.InnerCur), InnerMax: int(rm.InnerMax),
		SpiritCur:   int(rm.SpiritCur), SpiritMax: int(rm.SpiritMax),
		StaminaCur:  int(rm.StaminaCur), StaminaMax: int(rm.StaminaMax),
		Magnesium:   magPtr,
		IsSelf:      isSelf,
	})
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
