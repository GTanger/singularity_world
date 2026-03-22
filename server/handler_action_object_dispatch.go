// do_action：目標為房間物件時的薄 dispatch；見 action_object_*.go。
package server

import (
	"singularity_world/config"
	"singularity_world/event"
	"singularity_world/game"
)

func doActionObjectBranch(c *Client, msg *ClientMsg, cfg config.Server, store *SessionStore, hub *Hub, playerRoom, targetID, action string) {
	_ = msg
	_ = store
	obj, abort := resolveRoomObjectForAction(c, targetID, playerRoom, action)
	if abort {
		return
	}
	narrative := buildObjectActionNarrative(obj, action)
	now := game.NowUnix()
	_ = event.Append(now, c.PlayerID, event.TypeObserved, obj.ID)
	if !applyObjectMoveThrough(c, cfg, hub, obj, playerRoom, action) {
		return
	}
	others, moveTargetID := objectFollowupActionButtons(obj, playerRoom, action)
	c.Send <- mustJSON(ActionResultMsg{
		Type: "action_result", Action: action,
		TargetID: obj.ID, TargetName: obj.Name,
		Narrative: narrative, Success: true,
		Actions: others, MoveTargetID: moveTargetID,
	})
}
