// 同房實體插座／職場 gate；已 sendError 則回傳 false（呼叫方應結束 entity 分支）。
package server

import (
	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/gametext"
)

func entitySocketOK(c *Client, target *entity.Character, targetID, playerRoom, action string) bool {
	var targetSockets []string
	if target.Kind == "npc" {
		targetSockets = db.GetSocketsForNPC(targetID, playerRoom)
		if !entity.HasSocket(targetSockets, action) {
			sendError(c, gametext.Clientf("cannot_action_fmt", action))
			return false
		}
		if !db.IsDefaultSocket(action) {
			inVenue, _ := db.EntityInVenueAtRoom(targetID, playerRoom)
			if !inVenue {
				sendError(c, gametext.Clientf("target_not_at_workplace_fmt", action))
				return false
			}
		}
	} else {
		targetSockets = (&entity.Character{}).Sockets()
		if !entity.HasSocket(targetSockets, action) {
			sendError(c, gametext.Clientf("cannot_action_fmt", action))
			return false
		}
	}
	return true
}
