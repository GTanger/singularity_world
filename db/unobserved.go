// Package db 未觀測時的輕量背景模擬：事實仍持續（排班依班表動、無排班者狀態／位置緩變），不發敘事。
// 觀測只影響玩家接收到的敘事；被觀測角色的「事實」不因無人看而停止或亂來（上班時間不會被隨機趕去逛、睡覺時間不會被拉出門）。
package db

import (
	"database/sql"
	"math/rand"
)

// 未觀測背景模擬：每輪抽樣最多處理的 NPC 數，避免單次負載過大。
const UnobservedMaxNPCsPerTick = 50

// 單一無排班 NPC 被抽到後，施加任一種效果的概率（四分支：加鎂／採集／求職／隨機鄰房移動）。
const unobservedEffectDenom = 4

// RunUnobservedWorldTick 在無人觀測時呼叫：抽樣最多 maxNPCs 個 NPC，讓其「事實」持續——有排班者依 gameHour 朝 work_room/rest_room 走一步；無排班者施加一項抽象效果（加鎂／採集／求職／隨機鄰房）。不發敘事。
func RunUnobservedWorldTick(database *sql.DB, g *RoomGraph, gameHour int, maxNPCs int) {
	if maxNPCs <= 0 {
		maxNPCs = UnobservedMaxNPCsPerTick
	}
	ids, err := GetNPCIDsWithRoom(database)
	if err != nil || len(ids) == 0 {
		return
	}
	rand.Shuffle(len(ids), func(i, j int) { ids[i], ids[j] = ids[j], ids[i] })
	if len(ids) > maxNPCs {
		ids = ids[:maxNPCs]
	}
	venueIDs, _ := GetAllVenueIDs(database)
	for _, entityID := range ids {
		currentRoom, _ := GetEntityRoom(database, entityID)
		targetRoom, hasSchedule := GetScheduleTargetRoom(database, entityID, gameHour)
		// 有排班：事實＝依班表動，在班往 work_room、下班往 rest_room，一步一步走
		if hasSchedule && targetRoom != "" && currentRoom != targetRoom && g != nil {
			path := g.FindPath(currentRoom, targetRoom)
			if len(path) > 0 {
				_ = SetEntityRoom(database, entityID, path[0])
			}
			continue
		}
		// 無排班：抽象效果四擇一（加鎂／採集／求職／隨機鄰房），不違背身份與時段
		effect := rand.Intn(unobservedEffectDenom)
		switch effect {
		case 0:
			c, _ := GetEntity(database, entityID)
			if c != nil && c.Magnesium < SurvivalLineMg {
				_ = AddMagnesium(database, entityID, 3+rand.Intn(8))
			}
		case 1:
			_ = AddToInventory(database, entityID, "wild_herb", 1)
		case 2:
			assignments, _ := GetAssignmentsForEntity(database, entityID)
			if len(assignments) > 0 || len(venueIDs) == 0 {
				continue
			}
			vids := make([]string, len(venueIDs))
			copy(vids, venueIDs)
			rand.Shuffle(len(vids), func(i, j int) { vids[i], vids[j] = vids[j], vids[i] })
			for _, vid := range vids {
				n, _ := GetAssignmentCountByVenue(database, vid)
				if n < 2 {
					_ = InsertAssignment(database, entityID, "服務生", vid, "")
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
			_ = SetEntityRoom(database, entityID, next)
		}
	}
}
