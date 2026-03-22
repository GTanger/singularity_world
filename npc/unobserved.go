// Package npc：未觀測時的輕量背景模擬（事實仍持續），不發敘事。
// 觀測只影響玩家接收到的敘事；被觀測角色的「事實」不因無人看而停止或亂來。
package npc

import (
	"math/rand"

	"singularity_world/db"
)

// 未觀測背景模擬：每輪抽樣最多處理的 NPC 數，避免單次負載過大。
const UnobservedMaxNPCsPerTick = 50

// 單一無排班 NPC 被抽到後，施加任一種效果的概率（四分支：加鎂／採集／求職／隨機鄰房移動）。
const unobservedEffectDenom = 4

// RunUnobservedWorldTick 在無人觀測時呼叫：抽樣最多 maxNPCs 個 NPC，讓其「事實」持續——有排班者依 gameHour 朝 work_room/rest_room 走一步；無排班者施加一項抽象效果（加鎂／採集／求職／隨機鄰房）。不發敘事。
func RunUnobservedWorldTick(g *db.RoomGraph, gameHour int, maxNPCs int) {
	if maxNPCs <= 0 {
		maxNPCs = UnobservedMaxNPCsPerTick
	}
	ids, err := db.GetNPCIDsWithRoom()
	if err != nil || len(ids) == 0 {
		return
	}
	rand.Shuffle(len(ids), func(i, j int) { ids[i], ids[j] = ids[j], ids[i] })
	if len(ids) > maxNPCs {
		ids = ids[:maxNPCs]
	}
	venueIDs, _ := db.GetAllVenueIDs()
	for _, entityID := range ids {
		currentRoom, _ := db.GetEntityRoom(entityID)
		targetRoom, hasSchedule := db.GetScheduleTargetRoom(entityID, gameHour)
		if hasSchedule && targetRoom != "" && currentRoom != targetRoom && g != nil {
			path := g.FindPath(currentRoom, targetRoom)
			if len(path) > 0 {
				_ = db.SetEntityRoom(entityID, path[0])
			}
			continue
		}
		effect := rand.Intn(unobservedEffectDenom)
		switch effect {
		case 0:
			c, _ := db.GetEntity(entityID)
			if c != nil && c.Magnesium < SurvivalLineMg {
				_ = db.AddMagnesium(entityID, 3+rand.Intn(8))
			}
		case 1:
			_ = db.AddToInventory(entityID, "wild_herb", 1)
		case 2:
			assignments, _ := db.GetAssignmentsForEntity(entityID)
			if len(assignments) > 0 || len(venueIDs) == 0 {
				continue
			}
			vids := make([]string, len(venueIDs))
			copy(vids, venueIDs)
			rand.Shuffle(len(vids), func(i, j int) { vids[i], vids[j] = vids[j], vids[i] })
			for _, vid := range vids {
				n, _ := db.GetAssignmentCountByVenue(vid)
				if n < 2 {
					_ = db.InsertAssignment(entityID, "服務生", vid, "")
					break
				}
			}
		case 3:
			if g == nil {
				continue
			}
			if currentRoom == "" {
				continue
			}
			neighbors := g.Neighbors(currentRoom)
			if len(neighbors) == 0 {
				continue
			}
			next := neighbors[rand.Intn(len(neighbors))]
			_ = db.SetEntityRoom(entityID, next)
		}
	}
}
