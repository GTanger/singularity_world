// do_action：目標為角色（玩家／NPC）時的薄 dispatch；各動詞見 action_entity_*.go。
package server

import (
	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/game"
	"singularity_world/gametext"
)

// doActionEntityBranch 若 targetID 解析為同房實體並已處理（含錯誤回應）則回傳 true；否則 false（改試物件）。
func doActionEntityBranch(c *Client, msg *ClientMsg, cfg config.Server, store *SessionStore, hub *Hub, playerRoom, targetID, action string) bool {
	target, err := db.GetEntity(targetID)
	if err != nil || target == nil {
		return false
	}
	targetRoom, _ := db.GetEntityRoom(targetID)
	if targetRoom == "" || playerRoom != targetRoom {
		sendError(c, gametext.Client("target_not_same_room"))
		return true
	}
	if !entitySocketOK(c, target, targetID, playerRoom, action) {
		return true
	}
	now := game.NowUnix()
	switch action {
	case "Look":
		doActionEntityLook(c, now, targetID, target)
	case "Talk":
		doActionEntityTalk(c, msg, cfg, store, now, playerRoom, targetID, target)
	case "Subdue", "Slay":
		if doActionEntityCombat(c, store, now, playerRoom, targetID, action, target) {
			return true
		}
	case "Borrow":
		doActionEntityBorrow(c, now, targetID, target)
	case "Trade":
		if doActionEntityTrade(c, msg, now, targetID, target) {
			return true
		}
	default:
		sendError(c, gametext.Clientf("unknown_action_fmt", action))
	}
	return true
}
