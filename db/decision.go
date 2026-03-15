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

// candidate 候選行為與權重（供性格加權選擇）。
type candidate struct {
	intent Intent
	weight float64
}

// personalityWeightedSelect 依性格加權隨機選一意圖。Boldness 高→Gather/Trade↑ Beg↓；Orderliness 高→SeekJob↑；Sensitivity 高→Beg↑。
func personalityWeightedSelect(candidates []candidate, p Personality, hasPers bool) Intent {
	if len(candidates) == 0 {
		return Intent{Type: IntentWander}
	}
	if len(candidates) == 1 || !hasPers {
		return candidates[0].intent
	}
	for i := range candidates {
		switch candidates[i].intent.Type {
		case IntentSeekJob:
			candidates[i].weight *= 0.5 + p.Orderliness
		case IntentBeg:
			candidates[i].weight *= 1.5 - p.Boldness
			candidates[i].weight *= 0.5 + p.Sensitivity
		case IntentGather:
			candidates[i].weight *= 0.5 + p.Boldness
			candidates[i].weight *= 1.5 - p.Orderliness
		case IntentTrade:
			candidates[i].weight *= 0.5 + p.Boldness
		case IntentWander:
			candidates[i].weight *= 1.5 - p.Orderliness
		}
	}
	total := 0.0
	for _, c := range candidates {
		total += c.weight
	}
	if total <= 0 {
		return candidates[0].intent
	}
	r := rand.Float64() * total
	for _, c := range candidates {
		r -= c.weight
		if r <= 0 {
			return c.intent
		}
	}
	return candidates[len(candidates)-1].intent
}

// Decide 產出單一意圖。生存 > 安定 > 閒逛；生存/安定分支依性格加權選擇。
func Decide(state DecisionState, ctx DecisionContext) Intent {
	survivalUrgency := urgencySurvival(state.Magnesium)
	if survivalUrgency > 0 {
		var candidates []candidate
		if len(ctx.VenueRoomIDsInRange) > 0 {
			candidates = append(candidates, candidate{Intent{Type: IntentSeekJob, TargetRoomID: pickRandom(ctx.VenueRoomIDsInRange)}, 1.0})
		}
		if hasTag(ctx.RoomTags, "gate", "social") {
			candidates = append(candidates, candidate{Intent{Type: IntentBeg}, 1.0})
		} else if len(ctx.SocialOrGateInRange) > 0 {
			candidates = append(candidates, candidate{Intent{Type: IntentBeg, TargetRoomID: pickRandom(ctx.SocialOrGateInRange)}, 1.0})
		}
		if len(ctx.WildernessOutdoorInRange) > 0 {
			candidates = append(candidates, candidate{Intent{Type: IntentGather, TargetRoomID: pickRandom(ctx.WildernessOutdoorInRange)}, 1.0})
		}
		if len(ctx.VenueRoomIDsInRange) > 0 {
			candidates = append(candidates, candidate{Intent{Type: IntentSeekJob, TargetRoomID: pickRandom(ctx.VenueRoomIDsInRange)}, 1.0})
		}
		candidates = append(candidates, candidate{Intent{Type: IntentWander}, 1.0})
		return personalityWeightedSelect(candidates, state.Personality, state.HasPersonality)
	}
	if !state.HasAssignment {
		var candidates []candidate
		if len(ctx.VenueRoomIDsInRange) > 0 {
			candidates = append(candidates, candidate{Intent{Type: IntentSeekJob, TargetRoomID: pickRandom(ctx.VenueRoomIDsInRange)}, 1.0})
		}
		candidates = append(candidates, candidate{Intent{Type: IntentWander}, 1.0})
		return personalityWeightedSelect(candidates, state.Personality, state.HasPersonality)
	}
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

