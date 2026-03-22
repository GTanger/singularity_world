// do_action：對同房實體 Look。
package server

import (
	"singularity_world/entity"
	"singularity_world/event"
)

func doActionEntityLook(c *Client, now int64, targetID string, target *entity.Character) {
	narrative := buildLookNarrative(target)
	_ = event.Append(now, c.PlayerID, event.TypeObserved, targetID)
	lookTargetName := target.DisplayTitle
	if lookTargetName == "" {
		lookTargetName = target.ID
	}
	c.Send <- mustJSON(ActionResultMsg{
		Type: "action_result", Action: "Look",
		TargetID: target.ID, TargetName: lookTargetName,
		Narrative: narrative, Success: true,
	})
}
