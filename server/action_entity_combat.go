// do_action：對同房實體 Subdue／Slay；若無法取得攻擊者則 sendError 並回傳 true。
package server

import (
	"strconv"

	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/event"
	"singularity_world/gametext"
	"singularity_world/npc"
)

func doActionEntityCombat(c *Client, store *SessionStore, now int64, playerRoom, targetID, action string, target *entity.Character) bool {
	attacker, _ := db.GetEntity(c.PlayerID)
	if attacker == nil {
		sendError(c, gametext.Client("get_self_failed"))
		return true
	}
	subdue := action == "Subdue"
	narrative, fightWinner, attackerFinalHP, defenderFinalHP := buildAttackNarrative(playerRoom, attacker, target, subdue)
	if target.Kind == "npc" {
		extra := ""
		if subdue {
			if fightWinner == "attacker" {
				extra = npc.NpcBehaviorReactionLine(targetID, c.PlayerID, "subdue_victim")
			} else if fightWinner == "defender" {
				extra = npc.NpcBehaviorReactionLine(targetID, c.PlayerID, "lost_subdue")
			}
		}
		if extra != "" {
			narrative += "\n" + extra
		}
	}
	_ = event.Append(now, c.PlayerID, event.TypeCombat, targetID)
	attackTargetName := target.DisplayTitle
	if attackTargetName == "" {
		attackTargetName = target.ID
	}
	outAct := "Slay"
	if subdue {
		outAct = "Subdue"
	}
	c.Send <- mustJSON(ActionResultMsg{
		Type: "action_result", Action: outAct,
		TargetID: target.ID, TargetName: attackTargetName,
		Narrative: narrative, Success: true,
	})
	_ = db.UpdateVit(c.PlayerID, attackerFinalHP)
	_ = db.UpdateVit(targetID, defenderFinalHP)
	_ = event.Append(now, c.PlayerID, event.TypeVit, strconv.Itoa(attackerFinalHP))
	_ = event.Append(now, targetID, event.TypeVit, strconv.Itoa(defenderFinalHP))
	if target.Kind == "npc" {
		if subdue {
			_ = db.AdjustFavorability(targetID, c.PlayerID, db.FavSubdue)
			if fightWinner == "attacker" {
				db.AdjustDisposition(targetID, db.DispSubdued)
			}
		} else {
			_ = db.AdjustFavorability(targetID, c.PlayerID, db.FavSlay)
		}
	}
	if target.Kind == "npc" && !subdue && defenderFinalHP <= 0 {
		deathRoom := playerRoom
		db.LogNPCEvent(targetID, db.EvtDeath, "在"+deathRoom+"倒下")
		name := attackTargetName
		narrateText := "傳來消息：【" + name + "】倒下了。"
		roomsToNarrate := make(map[string]bool)
		roomsToNarrate[deathRoom] = true
		for _, nb := range db.GetGraph().Neighbors(deathRoom) {
			roomsToNarrate[nb] = true
		}
		assignments, _ := db.GetAssignmentsForEntity(targetID)
		for _, a := range assignments {
			venueRooms, _ := db.GetRoomIDsForVenue(a.VenueID)
			for _, rid := range venueRooms {
				roomsToNarrate[rid] = true
			}
		}
		for rid := range roomsToNarrate {
			SendNarrateToRoom(store, rid, narrateText)
		}
		_ = db.RemoveAssignmentsForEntity(targetID)
		_ = db.RemoveScheduleForEntity(targetID)
	}
	return false
}
