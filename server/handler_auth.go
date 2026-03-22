// 008 P5：登入與創角。
package server

import (
	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/game"
	"singularity_world/gametext"
)

func handleLogin(c *Client, msg *ClientMsg, cfg config.Server, store *SessionStore, hub *Hub) {
	if msg.PlayerID == "" {
		sendError(c, gametext.Client("login_need_id"))
		return
	}
	if msg.Password == "" {
		sendError(c, gametext.Client("login_need_password"))
		return
	}
	ent, err := db.GetEntity(msg.PlayerID)
	if err != nil || ent == nil {
		sendError(c, gametext.Client("login_not_found"))
		return
	}
	if ent.Kind != "player" {
		sendError(c, gametext.Client("login_not_player"))
		return
	}
	ok, err := db.VerifyPassword(msg.PlayerID, msg.Password)
	if err != nil {
		sendError(c, gametext.Client("login_verify_failed"))
		return
	}
	if !ok {
		sendError(c, gametext.Client("login_wrong_password"))
		return
	}
	loginSuccess(c, msg.PlayerID, cfg, store)
}

func handleCreateCharacter(c *Client, msg *ClientMsg, cfg config.Server, store *SessionStore, hub *Hub) {
	if msg.PlayerID == "" {
		sendError(c, gametext.Client("create_need_id"))
		return
	}
	if msg.Password == "" {
		sendError(c, gametext.Client("create_need_password"))
		return
	}
	if len(msg.Password) < 6 {
		sendError(c, gametext.Client("create_password_short"))
		return
	}
	if len(msg.PlayerID) < 2 || len(msg.PlayerID) > 32 {
		sendError(c, gametext.Client("create_id_len"))
		return
	}
	existing, err := db.GetEntity(msg.PlayerID)
	if err != nil {
		sendError(c, gametext.Client("create_failed"))
		return
	}
	if existing != nil {
		sendError(c, gametext.Client("create_id_taken"))
		return
	}
	displayChar := msg.DisplayChar
	if displayChar == "" {
		displayChar = gametext.Client("display_char_default")
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
	if err := db.InsertEntity(msg.PlayerID, displayChar, gender); err != nil {
		sendError(c, gametext.Client("create_entity_failed"))
		return
	}
	if err := db.SetEntityRoom(msg.PlayerID, db.GetSpawnRoomID()); err != nil {
		sendError(c, gametext.Client("create_room_failed"))
		return
	}
	if err := db.CreateAuth(msg.PlayerID, msg.Password); err != nil {
		sendError(c, gametext.Client("create_auth_failed"))
		return
	}
	loginSuccess(c, msg.PlayerID, cfg, store)
}

func loginSuccess(c *Client, playerID string, cfg config.Server, store *SessionStore) {
	roomID, err := game.EnsureEntityInRoom(playerID, db.GetSpawnRoomID())
	if err != nil {
		sendError(c, gametext.Client("load_room_failed"))
		return
	}
	c.PlayerID = playerID
	store.Set(playerID, &Session{Client: c, PlayerID: playerID})
	gh := currentGameHour(cfg)
	view, err := game.GetRoomView(roomID, gh)
	if err != nil {
		sendError(c, gametext.Client("load_view_failed"))
		return
	}
	// 若玩家上次所在房間已不存在（如地圖重構後 lobby 被移除），改放到創生格並重試
	if view == nil {
		spawnID := db.GetSpawnRoomID()
		_ = db.SetEntityRoom(playerID, spawnID)
		roomID = spawnID
		view, err = game.GetRoomView(roomID, gh)
		if err != nil || view == nil {
			sendError(c, gametext.Client("load_view_failed"))
			return
		}
	}
	ent, _ := db.GetEntity(playerID)
	vit, qi, dex := 10, 10, 10
	if ent != nil {
		vit, qi, dex = ent.Vit, ent.Qi, ent.Dex
	}
	rm := db.ComputeResourceMaxes(vit, qi, dex)
	sendRoomView(c, view, cfg)
	// 10.18 短期記憶：登入時與當前房內每位 NPC 計一次見面
	for _, e := range view.Entities {
		if e.Kind == "npc" && e.ID != playerID {
			_ = db.RecordMeet(e.ID, playerID)
		}
	}
	sendMeWithStatus(c, ent, playerID, roomID, view.Room.Name, vit, qi, dex, rm)
}
