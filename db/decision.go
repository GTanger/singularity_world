// Package db 決策引擎（腦）：Decide(state, context) -> Intent。
// 對齊 docs/reference/奇點決策引擎架構.md、奇點馬斯洛需求系統.md、討論 002。
// 引擎只產生意圖，不寫 DB、不發送；由主迴圈或 TravelerManager 依 Intent 驅動移動或插座。
package db

import (
	"database/sql"
	"math/rand"
	"time"
)

// 生存層觸發的鎂閾值；低於此值視為生存未滿足，優先求職／乞討／採集。
const SurvivalLineMg = 50

// socialDecaySec 社交需求衰減週期（秒）：超過此時間未交談，社交緊迫度漸升至 1.0。
const socialDecaySec = 7200 // 約兩小時真實時間

// IntentType 意圖類型：對應插座語義（決策 002）與討論 002 行為。
type IntentType string

const (
	IntentSeekJob   IntentType = "seek_job"   // 求職：前往有場所的房間，路過或撮合寫 assignment
	IntentBeg       IntentType = "beg"        // 乞討：gate/social 執行 Beg
	IntentTrade     IntentType = "trade"      // 賣貨：行腳商／帶貨兜售
	IntentGather    IntentType = "gather"     // 採集：wilderness/outdoor 執行 Gather
	IntentWander    IntentType = "wander"     // 閒逛：無明確目標，隨機移動或停留
	IntentWork      IntentType = "work"       // 上班：有排班時由 TravelerManager 處理，腦不產出
	IntentIdle      IntentType = "idle"       // 閒置：在 social 等釋放 Idle 敘事
	IntentSocialize IntentType = "socialize"  // 社交：前往 tavern/inn/social，釋放 idle 對話、調整心境
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
	LastTalkAt     int64 // Unix 時間戳；0 = 從未交談；社交需求層用
	Disposition    int   // 心境值 [-100, +100]；影響社交與乞討意圖強度
}

// DecisionContext 決策輸入：世界情境（場所、房間標籤、鄰近可達房間）。
type DecisionContext struct {
	RoomTags                 []string // 當前房間標籤
	VenueRoomIDsInRange      []string // 距離內屬於某場所的房間（求職目標）
	SocialOrGateInRange      []string // 距離內帶 social / gate 的房間（乞討）
	WildernessOutdoorInRange []string // 距離內帶 wilderness / outdoor 的房間（採集）
	TavernInRange            []string // 距離內帶 tavern / inn 的房間（社交意圖目標）
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
	s.Disposition = c.Disposition
	assignments, _ := GetAssignmentsForEntity(db, entityID)
	s.HasAssignment = len(assignments) > 0
	if c.SoulSeed != nil {
		s.Personality = ExpandSoulSeedToPersonality(*c.SoulSeed)
		s.HasPersonality = true
	}
	s.CurrentRoomID, _ = GetEntityRoom(db, entityID)
	// 最近一筆 EvtTalk 事件時間戳作為社交衰減起點
	for _, e := range GetRecentEvents(entityID, 10) {
		if e.EventType == EvtTalk {
			s.LastTalkAt = e.At
			break
		}
	}
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
	ctx.TavernInRange = g.FindRoomsWithinDist(currentRoomID, []string{"tavern", "inn"}, maxDist)
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

// Decide V2：評分引擎。各需求層同時計算緊迫度分數，性格三軸直接影響各意圖權重，
// 加權隨機選出最終意圖；取代 V1 固定 if-else 優先序。
func Decide(state DecisionState, ctx DecisionContext) Intent {
	survival := urgencySurvival(state.Magnesium)
	stability := urgencyStability(state.HasAssignment)
	social := urgencySocial(state.LastTalkAt, state.Disposition)
	p := state.Personality
	hasPers := state.HasPersonality

	var cands []candidate

	// ── 求職：安定需求 + 生存壓力共同驅動；秩序感高者更傾向找正職 ──
	if len(ctx.VenueRoomIDsInRange) > 0 {
		score := (stability + survival*0.5)
		if hasPers {
			score *= 0.5 + p.Orderliness*0.5
		}
		if score > 0.05 {
			cands = append(cands, candidate{
				intent: Intent{Type: IntentSeekJob, TargetRoomID: pickRandom(ctx.VenueRoomIDsInRange)},
				weight: score,
			})
		}
	}

	// ── 乞討：生存壓力驅動；膽量低者更願意乞討，敏感度高者更能放下面子 ──
	if hasTag(ctx.RoomTags, "gate", "social") || len(ctx.SocialOrGateInRange) > 0 {
		score := survival
		if hasPers {
			score *= (1.0 - p.Boldness*0.5) * (0.5 + p.Sensitivity*0.5)
		}
		if score > 0.05 {
			target := ""
			if !hasTag(ctx.RoomTags, "gate", "social") {
				target = pickRandom(ctx.SocialOrGateInRange)
			}
			cands = append(cands, candidate{
				intent: Intent{Type: IntentBeg, TargetRoomID: target},
				weight: score,
			})
		}
	}

	// ── 採集：生存壓力驅動；膽量高者更願意去荒野，秩序感低者更自由 ──
	if len(ctx.WildernessOutdoorInRange) > 0 {
		score := survival
		if hasPers {
			score *= (0.5 + p.Boldness*0.5) * (1.0 - p.Orderliness*0.5)
		}
		if score > 0.05 {
			cands = append(cands, candidate{
				intent: Intent{Type: IntentGather, TargetRoomID: pickRandom(ctx.WildernessOutdoorInRange)},
				weight: score,
			})
		}
	}

	// ── 社交：長時未交談 + 心境正面時驅動；敏感度高者更渴望社交 ──
	if len(ctx.TavernInRange) > 0 && social > 0 {
		score := social
		if hasPers {
			score *= 0.5 + p.Sensitivity*0.5
		}
		if score > 0.05 {
			target := ""
			if !hasTag(ctx.RoomTags, "tavern", "inn", "social") {
				target = pickRandom(ctx.TavernInRange)
			}
			cands = append(cands, candidate{
				intent: Intent{Type: IntentSocialize, TargetRoomID: target},
				weight: score,
			})
		}
	}

	// ── 自我實現：有工作且鎂足夠，依個性選額外活動 ──
	if state.HasAssignment && survival == 0 {
		if hasPers && p.Boldness > 0.6 && len(ctx.WildernessOutdoorInRange) > 0 {
			cands = append(cands, candidate{
				intent: Intent{Type: IntentGather, TargetRoomID: pickRandom(ctx.WildernessOutdoorInRange)},
				weight: 0.3 * p.Boldness,
			})
		}
		if hasPers && p.Orderliness > 0.6 && len(ctx.VenueRoomIDsInRange) > 0 {
			cands = append(cands, candidate{
				intent: Intent{Type: IntentWork, TargetRoomID: ctx.VenueRoomIDsInRange[0]},
				weight: 0.3 * p.Orderliness,
			})
		}
	}

	// ── 閒逛：底噪，始終存在；秩序感低者更常閒逛 ──
	wanderScore := 0.4
	if hasPers {
		wanderScore *= 1.5 - p.Orderliness
	}
	cands = append(cands, candidate{intent: Intent{Type: IntentWander}, weight: wanderScore})

	return weightedRandomSelect(cands)
}

// urgencySurvival 生存緊迫度：鎂越低越緊迫，>= 閾值為 0。
func urgencySurvival(mg int) float64 {
	if mg >= SurvivalLineMg {
		return 0
	}
	v := 1.0 - (float64(mg) / float64(SurvivalLineMg))
	if v < 0 {
		return 0
	}
	if v > 1 {
		return 1
	}
	return v
}

// urgencyStability 安定緊迫度：無指派者有高度不安感（0.8）；有指派則為 0。
func urgencyStability(hasAssignment bool) float64 {
	if hasAssignment {
		return 0
	}
	return 0.8
}

// urgencySocial 社交緊迫度：長時未交談漸升至 1.0；心境越低越不想出門。
// disposition [-100,+100] → 心境因子 [0, 1]；心境=0 → 0.5，心境=+100 → 1.0，心境=-100 → 0.0。
func urgencySocial(lastTalkAt int64, disposition int) float64 {
	if lastTalkAt <= 0 {
		return 0 // 從未交談：未知孤獨，不驅動社交
	}
	elapsed := time.Now().Unix() - lastTalkAt
	if elapsed < 0 {
		elapsed = 0
	}
	u := float64(elapsed) / socialDecaySec
	if u > 1 {
		u = 1
	}
	// disposition factor: [-100,+100] → [0.0, 1.0]
	dispFactor := (float64(disposition) + 100.0) / 200.0
	return u * dispFactor
}

// weightedRandomSelect 從有分數的候選清單中加權隨機選一意圖。
func weightedRandomSelect(cands []candidate) Intent {
	if len(cands) == 0 {
		return Intent{Type: IntentWander}
	}
	total := 0.0
	for _, c := range cands {
		total += c.weight
	}
	if total <= 0 {
		return cands[0].intent
	}
	r := rand.Float64() * total
	for _, c := range cands {
		r -= c.weight
		if r <= 0 {
			return c.intent
		}
	}
	return cands[len(cands)-1].intent
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
	case IntentWork:
		if ctx != nil && len(ctx.VenueRoomIDsInRange) > 0 {
			target := ctx.VenueRoomIDsInRange[0]
			if target != currentRoomID {
				if path := g.FindPath(currentRoomID, target); len(path) > 0 {
					return path
				}
			}
		}
	case IntentSocialize:
		if ctx != nil && len(ctx.TavernInRange) > 0 {
			target := pickRandom(ctx.TavernInRange)
			if target != currentRoomID {
				if path := g.FindPath(currentRoomID, target); len(path) > 0 {
					return path
				}
			}
		}
		for _, tag := range []string{"tavern", "inn", "social"} {
			room, _ := g.FindNearestByTag(currentRoomID, tag, maxDist)
			if room != "" && room != currentRoomID {
				return g.FindPath(currentRoomID, room)
			}
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

