// Package npc：求職撮合（10.15），無職且鎂低於閾值的 NPC 與有職缺的場所匹配，寫入 assignment 與排班。對齊討論 002。
package npc

import (
	"math/rand"

	"singularity_world/db"
)

// SeekJobMgThresholdDefault 鎂閾值預設值（config 為 0 時使用）。
const SeekJobMgThresholdDefault = 50

// JobMatchStableProbability 安定需求：無職且鎂≥閾值者參與撮合的機率（細目 3）。
const JobMatchStableProbability = 0.3

// DefaultOccupationForHire 撮合時寫入的職業；場所若有既有指派則取該場所第一種職業（GetFirstOccupationIDForVenue），無則用此常數。
const DefaultOccupationForHire = "服務生"

// 撮合後排班預設班次（日班 06–19）。
const defaultShiftStart, defaultShiftEnd = 6, 19

// JobMatchParams 求職撮合參數（由 main 從 config 填入）。
type JobMatchParams struct {
	MaxPerVenue        int  // 全局限額；單一場所可用 GetVenueMaxStaff 覆寫
	MgThreshold        int  // 鎂低於此才參與；0 用 SeekJobMgThresholdDefault
	JobMatchWhenStable bool // 為 true 時鎂≥閾值者以機率參與
}

// RunJobMatchingTick 執行一輪求職撮合。細目 2：每場所用 GetVenueMaxStaff 取上限；細目 3：threshold、JobMatchWhenStable；細目 5：g 非 nil 時依 NPC 與場所距離優先匹配。回傳本輪新入職的 entity ID 列表。
func RunJobMatchingTick(g *db.RoomGraph, p JobMatchParams) []string {
	if p.MaxPerVenue <= 0 {
		p.MaxPerVenue = 2
	}
	threshold := p.MgThreshold
	if threshold <= 0 {
		threshold = SeekJobMgThresholdDefault
	}
	var added []string
	ids, err := db.GetNPCIDsWithRoom()
	if err != nil || len(ids) == 0 {
		return added
	}
	var unassigned []string
	for _, entityID := range ids {
		list, _ := db.GetAssignmentsForEntity(entityID)
		if len(list) > 0 {
			continue
		}
		c, _ := db.GetEntity(entityID)
		if c == nil {
			continue
		}
		if c.Magnesium < threshold {
			unassigned = append(unassigned, entityID)
		} else if p.JobMatchWhenStable && rand.Float32() < JobMatchStableProbability {
			unassigned = append(unassigned, entityID)
		}
	}
	if len(unassigned) == 0 {
		return added
	}
	venueIDs, err := db.GetAllVenueIDs()
	if err != nil || len(venueIDs) == 0 {
		return added
	}
	var slots []string
	for _, vid := range venueIDs {
		n, _ := db.GetAssignmentCountByVenue(vid)
		limit := db.GetVenueMaxStaff(vid, p.MaxPerVenue)
		for k := 0; k < limit-n; k++ {
			slots = append(slots, vid)
		}
	}
	if len(slots) == 0 {
		return added
	}
	spawnRoom := db.GetSpawnRoomID()
	if g != nil {
		rand.Shuffle(len(slots), func(i, j int) { slots[i], slots[j] = slots[j], slots[i] })
		for _, venueID := range slots {
			roomIDs, _ := db.GetRoomIDsForVenue(venueID)
			if len(roomIDs) == 0 {
				continue
			}
			venueFirst := roomIDs[0]
			bestIdx := -1
			bestDist := 9999
			for i, entityID := range unassigned {
				current, _ := db.GetEntityRoom(entityID)
				if current == "" {
					continue
				}
				path := g.FindPath(current, venueFirst)
				dist := len(path)
				if current == venueFirst {
					dist = 0
				}
				if dist < bestDist {
					bestDist = dist
					bestIdx = i
				}
			}
			if bestIdx < 0 {
				continue
			}
			entityID := unassigned[bestIdx]
			unassigned = append(unassigned[:bestIdx], unassigned[bestIdx+1:]...)
			if doAssign(entityID, venueID, spawnRoom) {
				added = append(added, entityID)
			}
			if len(unassigned) == 0 {
				break
			}
		}
	} else {
		rand.Shuffle(len(unassigned), func(i, j int) { unassigned[i], unassigned[j] = unassigned[j], unassigned[i] })
		rand.Shuffle(len(slots), func(i, j int) { slots[i], slots[j] = slots[j], slots[i] })
		n := len(unassigned)
		if len(slots) < n {
			n = len(slots)
		}
		for i := 0; i < n; i++ {
			if doAssign(unassigned[i], slots[i], spawnRoom) {
				added = append(added, unassigned[i])
			}
		}
	}
	return added
}

func doAssign(entityID, venueID, spawnRoom string) bool {
	occupation := db.GetFirstOccupationIDForVenue(venueID)
	if occupation == "" {
		occupation = DefaultOccupationForHire
	}
	if db.InsertAssignment(entityID, occupation, venueID, "job_matching") != nil {
		return false
	}
	roomIDs, _ := db.GetRoomIDsForVenue(venueID)
	var workRoom string
	if len(roomIDs) > 0 {
		workRoom = roomIDs[0]
	}
	if workRoom != "" && spawnRoom != "" {
		_ = db.InsertSchedule(entityID, workRoom, spawnRoom, defaultShiftStart, defaultShiftEnd)
	}
	return true
}
