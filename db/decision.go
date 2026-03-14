// Package db 決策引擎（腦）：Decide(state, context) -> Intent。
// 對齊 docs/reference/奇點決策引擎架構.md、奇點馬斯洛需求系統.md、討論 002。
// 引擎只產生意圖，不寫 DB、不發送；由主迴圈或 TravelerManager 依 Intent 驅動移動或插座。
package db

import (
	"database/sql"
	"math/rand"
)

// 生存層觸發的鎂閾值；低於此值視為生存未滿足，優先求職／乞討／採集。
const SurvivalLineMg = 50

// IntentType 意圖類型：對應插座語義（決策 002）與討論 002 行為。
type IntentType string

const (
	IntentSeekJob IntentType = "seek_job" // 求職：前往有場所的房間，路過或撮合寫 assignment
	IntentBeg     IntentType = "beg"     // 乞討：gate/social 執行 Beg
	IntentTrade   IntentType = "trade"    // 賣貨：行腳商／帶貨兜售
	IntentGather  IntentType = "gather"   // 採集：wilderness/outdoor 執行 Gather
	IntentWander  IntentType = "wander"    // 閒逛：無明確目標，隨機移動或停留
	IntentWork    IntentType = "work"     // 上班：有排班時由 TravelerManager 處理，腦不產出
	IntentIdle    IntentType = "idle"     // 閒置：在 social 等釋放 Idle 敘事
)

// Intent 決策輸出：單一意圖與可選目標房間（由主迴圈／TravelerManager 執行移動或插座）。
type Intent struct {
	Type         IntentType
	TargetRoomID string // 可為空：表示在當前房間執行插座，或由上層自行選目標
}

// DecisionState 決策輸入：NPC 自身狀態（馬斯洛掃描用）。
type DecisionState struct {
	EntityID       string
	Magnesium      int
	HasAssignment  bool
	Personality    Personality
	HasPersonality bool
	CurrentRoomID  string
}

// DecisionContext 決策輸入：世界情境（場所、房間標籤、鄰近可達房間）。
type DecisionContext struct {
	RoomTags              []string // 當前房間標籤
	VenueRoomIDsInRange   []string // 距離內屬於某場所的房間（求職目標）
	SocialOrGateInRange   []string // 距離內帶 social / gate 的房間（乞討）
	WildernessOutdoorInRange []string // 距離內帶 wilderness / outdoor 的房間（採集）
}

// BuildDecisionState 從 DB 組裝 NPC 的決策狀態。
func BuildDecisionState(db *sql.DB, entityID string) (DecisionState, error) {
	var s DecisionState
	s.EntityID = entityID
	c, err := GetEntity(db, entityID)
	if err != nil || c == nil {
		return s, err
	}
	s.Magnesium = c.Magnesium
	assignments, _ := GetAssignmentsForEntity(db, entityID)
	s.HasAssignment = len(assignments) > 0
	if c.SoulSeed != nil {
		s.Personality = ExpandSoulSeedToPersonality(*c.SoulSeed)
		s.HasPersonality = true
	}
	s.CurrentRoomID, _ = GetEntityRoom(db, entityID)
	return s, nil
}

// BuildDecisionContext 從 RoomGraph 與 DB 組裝當前房間周邊情境（maxDist 步內）。
func BuildDecisionContext(db *sql.DB, g *RoomGraph, currentRoomID string, maxDist int) DecisionContext {
	if maxDist <= 0 {
		maxDist = 20
	}
	var ctx DecisionContext
	ctx.RoomTags = g.RoomTags(currentRoomID)
	venueRooms, _ := GetAllVenueRoomIDs(db)
	ctx.VenueRoomIDsInRange = filterRoomsWithinDist(g, currentRoomID, venueRooms, maxDist)
	ctx.SocialOrGateInRange = g.FindRoomsWithinDist(currentRoomID, []string{"social", "gate"}, maxDist)
	ctx.WildernessOutdoorInRange = g.FindRoomsWithinDist(currentRoomID, []string{"wilderness", "outdoor"}, maxDist)
	return ctx
}

// filterRoomsWithinDist 從候選房間中篩出在 origin 的 maxDist 步內者。
func filterRoomsWithinDist(g *RoomGraph, origin string, candidates []string, maxDist int) []string {
	if len(candidates) == 0 || maxDist <= 0 {
		return nil
	}
	var out []string
	for _, c := range candidates {
		if c == origin {
			out = append(out, c)
			continue
		}
		path := g.FindPath(origin, c)
		if len(path) > 0 && len(path) <= maxDist {
			out = append(out, c)
		}
	}
	return out
}

// hasTag 判斷房間標籤是否包含任一給定 tag。
func hasTag(tags []string, want ...string) bool {
	for _, t := range tags {
		for _, w := range want {
			if t == w {
				return true
			}
		}
	}
	return false
}

// Decide 產出單一意圖。V1：固定優先序 生存 > 安定 > 閒逛；不產出 Attack；性格僅預留。
func Decide(state DecisionState, ctx DecisionContext) Intent {
	// 1. 生存未滿足（鎂低於閾值）
	survivalUrgency := urgencySurvival(state.Magnesium)
	if survivalUrgency > 0 {
		if !state.HasAssignment && len(ctx.VenueRoomIDsInRange) > 0 {
			return Intent{Type: IntentSeekJob, TargetRoomID: pickRandom(ctx.VenueRoomIDsInRange)}
		}
		if hasTag(ctx.RoomTags, "gate", "social") {
			return Intent{Type: IntentBeg}
		}
		if len(ctx.SocialOrGateInRange) > 0 {
			return Intent{Type: IntentBeg, TargetRoomID: pickRandom(ctx.SocialOrGateInRange)}
		}
		if len(ctx.WildernessOutdoorInRange) > 0 {
			return Intent{Type: IntentGather, TargetRoomID: pickRandom(ctx.WildernessOutdoorInRange)}
		}
		if len(ctx.VenueRoomIDsInRange) > 0 {
			return Intent{Type: IntentSeekJob, TargetRoomID: pickRandom(ctx.VenueRoomIDsInRange)}
		}
		return Intent{Type: IntentWander}
	}

	// 2. 安定未滿足（無職）
	if !state.HasAssignment {
		if len(ctx.VenueRoomIDsInRange) > 0 {
			return Intent{Type: IntentSeekJob, TargetRoomID: pickRandom(ctx.VenueRoomIDsInRange)}
		}
		return Intent{Type: IntentWander}
	}

	// 3. 生存與安定皆滿足 → 閒逛
	return Intent{Type: IntentWander}
}

func urgencySurvival(mg int) float64 {
	if mg >= SurvivalLineMg {
		return 0
	}
	// clamp(1.0 - (current_mg / survival_line), 0, 1)
	v := 1.0 - (float64(mg) / float64(SurvivalLineMg))
	if v < 0 {
		return 0
	}
	if v > 1 {
		return 1
	}
	return v
}

// ResolveBrainPath 依意圖解析出尋路目標房間（供 MoveBrain 使用）。若意圖已帶 TargetRoomID 則用該房；否則依類型用 ctx 或圖找目標。
func ResolveBrainPath(g *RoomGraph, intent Intent, currentRoomID string, ctx *DecisionContext, maxDist int) []string {
	if intent.TargetRoomID != "" && intent.TargetRoomID != currentRoomID {
		path := g.FindPath(currentRoomID, intent.TargetRoomID)
		if len(path) > 0 {
			return path
		}
	}
	if maxDist <= 0 {
		maxDist = 25
	}
	switch intent.Type {
	case IntentSeekJob:
		if ctx != nil && len(ctx.VenueRoomIDsInRange) > 0 {
			target := pickRandom(ctx.VenueRoomIDsInRange)
			if target != currentRoomID {
				if path := g.FindPath(currentRoomID, target); len(path) > 0 {
					return path
				}
			}
		}
		room, _ := g.FindNearestByTag(currentRoomID, "social", maxDist)
		if room != "" && room != currentRoomID {
			return g.FindPath(currentRoomID, room)
		}
	case IntentBeg:
		room, _ := g.FindNearestByTag(currentRoomID, "gate", maxDist)
		if room == "" {
			room, _ = g.FindNearestByTag(currentRoomID, "social", maxDist)
		}
		if room != "" && room != currentRoomID {
			return g.FindPath(currentRoomID, room)
		}
	case IntentGather:
		room, _ := g.FindNearestByTag(currentRoomID, "wilderness", maxDist)
		if room == "" {
			room, _ = g.FindNearestByTag(currentRoomID, "outdoor", maxDist)
		}
		if room != "" && room != currentRoomID {
			return g.FindPath(currentRoomID, room)
		}
	case IntentWander:
		candidates := g.FindRoomsWithinDist(currentRoomID, []string{"social", "gate", "outdoor"}, maxDist)
		var filtered []string
		for _, c := range candidates {
			if c != currentRoomID {
				filtered = append(filtered, c)
			}
		}
		if len(filtered) > 0 {
			target := filtered[rand.Intn(len(filtered))]
			return g.FindPath(currentRoomID, target)
		}
	}
	return nil
}

