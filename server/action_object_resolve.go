// do_action：解析同房物件目標與插座；已 sendError 則 abort=true。
package server

import (
	"singularity_world/db"
	"singularity_world/gametext"
)

func resolveRoomObjectForAction(c *Client, targetID, playerRoom, action string) (obj *db.RoomObject, abort bool) {
	o, objectRoom := db.GetObjectByIDInRoom(playerRoom, targetID)
	if o == nil {
		o, objectRoom = db.GetObjectByNameInRoom(playerRoom, targetID)
	}
	if o == nil {
		o, objectRoom = db.GetObjectAndRoom(targetID)
	}
	if o == nil {
		sendError(c, gametext.Client("target_not_found"))
		return nil, true
	}
	if objectRoom != playerRoom {
		sendError(c, gametext.Client("target_not_same_room"))
		return nil, true
	}
	if !db.ObjectHasSocket(o, action) {
		sendError(c, gametext.Clientf("cannot_action_fmt", action))
		return nil, true
	}
	return o, false
}
