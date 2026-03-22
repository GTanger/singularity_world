// do_action：對同房實體 Borrow。
package server

import (
	"math/rand"

	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/event"
	"singularity_world/npc"
)

func doActionEntityBorrow(c *Client, now int64, targetID string, target *entity.Character) {
	borrowName := target.DisplayTitle
	if borrowName == "" {
		borrowName = target.ID
	}
	itemID, okB := db.PickNPCTradeOffer(target.Inventory)
	if !okB {
		c.Send <- mustJSON(ActionResultMsg{
			Type: "action_result", Action: "Borrow",
			TargetID: target.ID, TargetName: borrowName,
			Narrative: "【" + borrowName + "】身無長物可借，你只得作罷。", Success: true,
		})
		return
	}
	itemDisp := itemID
	if n, _, _, _, _, err := db.GetItemInfo(itemID); err == nil && n != "" {
		itemDisp = n
	}
	r := rand.Float64()
	var narrative string
	if r < 0.42 {
		_ = db.RemoveFromInventory(targetID, itemID, 1)
		_ = db.AddToInventory(c.PlayerID, itemID, 1)
		narrative = "你悄聲「借」得一物——「" + itemDisp + "」已入你手。"
		_ = event.Append(now, c.PlayerID, "borrow", targetID)
		if target.Kind == "npc" {
			_ = db.AdjustFavorability(targetID, c.PlayerID, db.FavBorrowSuccess)
			if ex := npc.NpcBehaviorReactionLine(targetID, c.PlayerID, "borrow_ok"); ex != "" {
				narrative += "\n" + ex
			}
		}
		pushRefresh(c)
	} else if r < 0.78 {
		narrative = "你伸手的瞬間被【" + borrowName + "】察覺。"
		if target.Kind == "npc" {
			_ = db.AdjustFavorability(targetID, c.PlayerID, db.FavBorrowCaught)
			if ex := npc.NpcBehaviorReactionLine(targetID, c.PlayerID, "borrow_caught"); ex != "" {
				narrative += "\n" + ex
			}
		}
		_ = event.Append(now, c.PlayerID, "borrow_fail", targetID)
	} else {
		narrative = "你試探了一番，未能得手，悻悻收回。"
		_ = event.Append(now, c.PlayerID, "borrow_fail", targetID)
	}
	c.Send <- mustJSON(ActionResultMsg{
		Type: "action_result", Action: "Borrow",
		TargetID: target.ID, TargetName: borrowName,
		Narrative: narrative, Success: true,
	})
}
