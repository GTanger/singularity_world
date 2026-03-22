// 主程式：觀測圈與 NPC 腦驅動到達房間後的數值效果。
package main

import (
	"fmt"
	"math/rand"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/gametext"
	"singularity_world/npc"
	"singularity_world/server"
)

// buildActiveRoomIDs 回傳「觀測圈」：有玩家的房間＋其鄰房；僅這些房內的 NPC 會被腦／移動驅動。無人時回傳空 map。
func buildActiveRoomIDs(sessionStore *server.SessionStore, roomGraph *db.RoomGraph) map[string]bool {
	playerRooms := server.GetPlayerRoomMap(sessionStore)
	out := make(map[string]bool)
	for rid := range playerRooms {
		out[rid] = true
		for _, nb := range roomGraph.Neighbors(rid) {
			out[nb] = true
		}
	}
	return out
}

func applyBrainArrivalEffects(entityID, roomID string, intent npc.IntentType) {
	be := gametext.BrainEffects()
	switch intent {
	case npc.IntentBeg:
		bb := config.Sim().BrainBeg
		extra := bb.MagnesiumRandExclusive
		if extra <= 0 {
			extra = 8
		}
		minMg := bb.MagnesiumMin
		if minMg < 0 {
			minMg = 0
		}
		amount := minMg + rand.Intn(extra)
		_ = db.AddMagnesium(entityID, amount)
		db.LogNPCEvent(entityID, db.EvtBeg, fmt.Sprintf(be.BegEventFmt, roomID, amount))
		db.AdjustDisposition(entityID, db.DispBegSuccess)
	case npc.IntentGather:
		_ = db.AddToInventory(entityID, "wild_herb", 1)
		db.LogNPCEvent(entityID, db.EvtGather, fmt.Sprintf(be.GatherEventFmt, roomID))
		db.AdjustDisposition(entityID, db.DispGather)
	case npc.IntentSeekJob:
		assignments, _ := db.GetAssignmentsForEntity(entityID)
		if len(assignments) > 0 {
			return
		}
		venueIDs, _ := db.GetVenueIDsForRoom(roomID)
		occ := gametext.OccupationWaiter()
		for _, vid := range venueIDs {
			n, _ := db.GetAssignmentCountByVenue(vid)
			if n < config.Sim().MaxAssignmentsPerVenue {
				if err := db.InsertAssignment(entityID, occ, vid, ""); err == nil {
					db.LogNPCEvent(entityID, db.EvtHired, fmt.Sprintf(be.HiredEventFmt, roomID, occ))
					db.AdjustDisposition(entityID, db.DispHired)
					break
				}
			}
		}
	case npc.IntentSocialize:
		db.LogNPCEvent(entityID, db.EvtTalk, be.TalkEvent)
		db.AdjustDisposition(entityID, db.DispTalked)
	case npc.IntentTrade:
		if !db.NpcStreetSellOne(entityID, roomID) {
			db.LogNPCEvent(entityID, db.EvtTrade, be.TradeNoStock)
		}
	}
}
