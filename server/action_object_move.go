// do_action：物件具 move_to_room_id 時執行 Move 切房；失敗則 sendError 並回傳 false。
package server

import (
	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/game"
	"singularity_world/gametext"
)

func applyObjectMoveThrough(c *Client, cfg config.Server, hub *Hub, obj *db.RoomObject, playerRoom, action string) bool {
	if action != "Move" || obj.MoveToRoomID == "" {
		return true
	}
	view, err := game.GetRoomView(obj.MoveToRoomID, currentGameHour(cfg))
	if err != nil || view == nil {
		sendError(c, gametext.Client("move_cannot_go"))
		return false
	}
	if err := db.SetEntityRoom(c.PlayerID, obj.MoveToRoomID); err != nil {
		sendError(c, gametext.Client("move_failed"))
		return false
	}
	sendRoomView(c, view, cfg)
	hub.Broadcast(mustJSON(MovedMsg{Type: "moved", PlayerID: c.PlayerID, RoomID: obj.MoveToRoomID, RoomName: view.Room.Name}))
	return true
}
