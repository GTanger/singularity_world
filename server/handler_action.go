// 008 P5：do_action 主幹；entity/object 見 handler_action_*_dispatch.go，樹葉見 action_entity_*、action_object_*、action_narrative_*。
package server

import (
	"log"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/gametext"
)

// ── 插頭插座：do_action ──

func handleDoAction(c *Client, msg *ClientMsg, cfg config.Server, store *SessionStore, hub *Hub) {
	defer func() {
		if r := recover(); r != nil {
			log.Printf("[do_action] panic: %v", r)
			sendError(c, gametext.Client("action_error"))
		}
	}()
	if c.PlayerID == "" {
		sendError(c, gametext.Client("need_login"))
		return
	}
	targetID := msg.EntityID
	action := msg.Action
	if action == "Attack" {
		action = "Slay" // 向後相容：舊「攻擊」＝送行（致死意圖）
	}
	if targetID == "" || action == "" {
		sendError(c, gametext.Client("missing_target_or_action"))
		return
	}
	if targetID == c.PlayerID {
		sendError(c, gametext.Client("cannot_self_action"))
		return
	}
	playerRoom, _ := db.GetEntityRoom(c.PlayerID)
	if playerRoom == "" {
		sendError(c, gametext.Client("no_current_room"))
		return
	}

	if doActionEntityBranch(c, msg, cfg, store, hub, playerRoom, targetID, action) {
		return
	}
	doActionObjectBranch(c, msg, cfg, store, hub, playerRoom, targetID, action)
}
