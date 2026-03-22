package npcnpc

import (
	"fmt"
	"math/rand"
	"strconv"
	"strings"
	"time"

	"singularity_world/ai"
	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/gametext"
	"singularity_world/npc"
	"singularity_world/server"
	"singularity_world/store"
)

// TryTriggerNpcNpcInRoom 在指定房嘗試觸發一次 NPC 間 AI 對話；topicHint 可為「交班」等或空。成功則寫入記憶並回傳 true。
// gameHour 為當前遊戲小時 0–23；無時間軸時傳 12。
func TryTriggerNpcNpcInRoom(sessionStore *server.SessionStore, cfg config.Server, roomID, topicHint string, gameHour int) bool {
	incStat("attempt")
	if cfg.OllamaBaseURL == "" || cfg.OllamaModel == "" {
		incStat("fail_no_model")
		return false
	}
	cd := time.Duration(config.Sim().NpcNpcSocial.RoomRecentPlayerTalkCooldownSec) * time.Second
	if sessionStore.RoomHasPlayerWithRecentTalk(roomID, cd) {
		incStat("fail_recent_player_talk")
		return false
	}
	entities, _ := db.GetEntitiesInRoom(roomID, gameHour)
	var npcs []*entity.Character
	for _, e := range entities {
		if e == nil || e.Kind != "npc" {
			continue
		}
		npcs = append(npcs, e)
	}
	if len(npcs) < 2 {
		incStat("fail_not_enough_npc")
		return false
	}
	nowUnix := time.Now().Unix()
	type pairPick struct {
		A      *entity.Character
		B      *entity.Character
		Score  int
		Thread *store.NpcThread
	}
	best := pairPick{Score: -1 << 30}
	pp := config.Sim().NpcNpcPairPick
	baseN := pp.ScoreBaseNoise
	if baseN <= 0 {
		baseN = 4
	}
	famDiv := pp.FamiliarityScoreDivisor
	if famDiv <= 0 {
		famDiv = 20
	}
	for i := 0; i < len(npcs); i++ {
		for j := i + 1; j < len(npcs); j++ {
			A, B := npcs[i], npcs[j]
			if A == nil || B == nil || A.ID == "" || B.ID == "" {
				continue
			}
			thread := db.GetNpcNpcThread(A.ID, B.ID)
			if thread != nil && thread.Phase != "cooling" && thread.TurnCount > 0 && nowUnix-thread.UpdatedAt > int64(pp.ThreadStaleSec) {
				thread.Phase = "cooling"
				thread.CooldownUntil = nowUnix + int64(pp.ThreadCooldownSec)
				thread.UpdatedAt = nowUnix
				_ = db.SetNpcNpcThread(A.ID, B.ID, thread)
			}
			if thread != nil && thread.Phase == "cooling" && thread.CooldownUntil > nowUnix {
				continue
			}
			if thread != nil && thread.Phase == "cooling" && thread.CooldownUntil > 0 && thread.CooldownUntil <= nowUnix {
				_ = db.DeleteNpcNpcThread(A.ID, B.ID)
				thread = nil
			}
			score := rand.Intn(baseN)
			if thread != nil && thread.Phase != "cooling" {
				score += 100
			}
			if dyad := db.GetNpcNpcDyad(A.ID, B.ID); dyad != nil {
				score += dyad.Familiarity / famDiv
				if dyad.Sentiment <= pp.DyadSentimentPenaltyThreshold {
					score -= pp.DyadSentimentPenalty
				}
				if HasTag(dyad.Tags, gametext.DyadTag("same_venue")) {
					score += pp.SameDyadVenueTagBonus
				}
			}
			if NpcPairSameVenue(A.ID, B.ID) {
				score += pp.SameVenueScoreBonus
			}
			if last := GetPairLastTalk(A.ID, B.ID); last > 0 && nowUnix-last < int64(pp.PairLastTalkPenaltySec) {
				score -= pp.PairLastTalkPenalty
			}
			pA, pB := db.PersonNameFromNPCListLabel(A.DisplayTitle), db.PersonNameFromNPCListLabel(B.DisplayTitle)
			if pA != "" && pA == pB {
				score -= pp.SameDisplayNamePenalty
			}
			if score > best.Score {
				best = pairPick{A: A, B: B, Score: score, Thread: thread}
			}
		}
	}
	if best.A == nil || best.B == nil {
		incStat("fail_no_pair")
		return false
	}
	A, B := best.A, best.B
	nameA, nameB := db.PersonNameFromNPCListLabel(A.DisplayTitle), db.PersonNameFromNPCListLabel(B.DisplayTitle)
	if nameA == "" {
		nameA = A.ID
	}
	if nameB == "" {
		nameB = B.ID
	}
	roomName, _ := db.GetRoomName(roomID)
	timeLabel := gametext.TimeOfDayLabel(gameHour)
	backstoryA := db.BuildIdentity(A.ID)
	backstoryB := db.BuildIdentity(B.ID)
	npcNpcMemory := db.GetNpcNpcConversationSummary(A.ID, B.ID)
	dyad := db.GetNpcNpcDyad(A.ID, B.ID)
	topicSource := "input_or_thread"
	chosenTopicID := ""
	if topicHint == "" && best.Thread != nil && best.Thread.TopicType != "" {
		topicHint = best.Thread.TopicType
		topicSource = "thread_topic"
	}
	if topicHint == "" {
		mask := TopicMaskForRoom(roomID, gameHour)
		fam, sen := 0, 0
		var tags []string
		if dyad != nil {
			fam = dyad.Familiarity
			sen = dyad.Sentiment
			tags = dyad.Tags
		}
		if t := npc.PickRandomNpcNpcTopicForPair(mask, "", fam, sen, tags); t != nil {
			topicHint = t.Hint
			chosenTopicID = t.ID
			topicSource = "weighted_mask_pick"
		}
	}
	if chosenTopicID == "" && topicHint != "" {
		if id := npc.FindNpcNpcTopicIDByHint(topicHint); id != "" {
			chosenTopicID = id
		}
	}
	playerInRoom := sessionStore.RoomHasPlayer(roomID)
	ns := gametext.NpcSocial()
	soc := config.Sim().NpcNpcSocial
	gossipPct := soc.PlayerGossipBiasPercent
	if gossipPct > 100 {
		gossipPct = 100
	}
	if playerInRoom && topicSource == "weighted_mask_pick" && rand.Intn(100) < gossipPct {
		if t := npc.GetNpcNpcTopicByID(gametext.TopicID("gossip")); t != nil {
			topicHint = t.Hint + ns.PlayerGossipSuffix
			chosenTopicID = gametext.TopicID("gossip")
			topicSource = "player_room_gossip_bias"
		}
	}
	slideMax := soc.RecentRoomEventsSlidingMax
	if slideMax <= 0 {
		slideMax = 3
	}
	recentEvents := RecentRoomEvents(roomID, slideMax)
	roomZone := ""
	roomDesc := ""
	rumorTopK := soc.RumorTopKDefault
	if rumorTopK <= 0 {
		rumorTopK = 2
	}
	if playerInRoom {
		if k := soc.RumorTopKPlayerInRoom; k > 0 {
			rumorTopK = k
		} else {
			rumorTopK = 3
		}
	}
	roomTagsJoined := ""
	if room, _ := db.GetRoom(roomID); room != nil {
		roomZone = room.Zone
		roomDesc = strings.TrimSpace(room.Description)
		if len(room.Tags) > 0 {
			roomTagsJoined = strings.Join(room.Tags, "、")
		}
		for _, rr := range db.TopNpcRumors(roomID, room.Zone, nowUnix, rumorTopK) {
			recentEvents = append(recentEvents, ns.RumorPrefix+rr.Text)
			_ = db.MarkRumorUsedByText(rr.Text, nowUnix)
		}
	}
	if playerInRoom {
		recentEvents = append([]string{ns.PlayerPresentLine}, recentEvents...)
	}
	if d := db.GetNpcRumorDigest(); d != nil && strings.TrimSpace(d.Text) != "" {
		recentEvents = append(recentEvents, d.Text)
	}
	anchorsUsed := []string{}
	if best.Thread != nil && len(best.Thread.Anchors) > 0 {
		anchorsUsed = append(anchorsUsed, best.Thread.Anchors...)
		recentEvents = append(recentEvents, ns.KnownAnchorsPrefix+strings.Join(best.Thread.Anchors, "；"))
	}
	if chosenTopicID != "" {
		if top := npc.GetNpcNpcTopicByID(chosenTopicID); top != nil && len(top.Lines) > 0 {
			seed := strings.TrimSpace(top.Lines[rand.Intn(len(top.Lines))])
			if seed != "" {
				recentEvents = append(recentEvents, ns.ToneSeedPrefix+seed)
			}
		}
	}
	if echo := sessionStore.PlayerTalkEchoForNpcSocial(roomID); echo != "" {
		recentEvents = append(recentEvents, fmt.Sprintf(ns.RumorEchoLineFmt, echo))
	}
	recentEvents = dedupeSocialContextLines(recentEvents)
	maxRunes := cfg.NpcNpcQualityMaxRunes
	lineA, lineB, anchors, err := ai.CallAITalkNPCToNPC(cfg.OllamaBaseURL, cfg.OllamaModel, nameA, nameB, backstoryA, backstoryB, roomName, timeLabel, recentEvents, dyadRelationHint(dyad), npcNpcMemory, topicHint, roomDesc, roomTagsJoined, playerInRoom)
	if err != nil || lineA == "" || lineB == "" {
		incStat("fail_llm_empty")
		return false
	}
	lineA = ai.StripNpcLineSpeakerPrefix(lineA, nameA, nameB)
	lineB = ai.StripNpcLineSpeakerPrefix(lineB, nameB, nameA)
	lineA = ai.SanitizeNPCDialogueLine(lineA)
	lineB = ai.SanitizeNPCDialogueLine(lineB)
	okA, reasonA := qualityGateNpcLine(lineA, maxRunes)
	okB, reasonB := qualityGateNpcLine(lineB, maxRunes)
	if lineA == "" || lineB == "" || !okA || !okB {
		incStat("fail_quality_gate")
		if !okA && reasonA != "" {
			incStat("fail_quality_" + reasonA)
		}
		if !okB && reasonB != "" {
			incStat("fail_quality_" + reasonB)
		}
		return false
	}
	conflictOK, conflictReason := anchorConsistencyCheck(anchorsUsed, lineA, lineB)
	if !conflictOK {
		for _, a := range anchorsUsed {
			_ = db.PenalizeRumorByText(a, nowUnix, conflictReason)
		}
		setLastChoice(&LastSocialChoice{
			At:                   nowUnix,
			RoomID:               roomID,
			AID:                  A.ID,
			BID:                  B.ID,
			ScoreTotal:           best.Score,
			TopicHint:            topicHint,
			TopicID:              chosenTopicID,
			TopicReason:          topicSource,
			AnchorsUsed:          anchorsUsed,
			AnchorConflict:       true,
			AnchorConflictReason: conflictReason,
		})
		incStat("fail_anchor_conflict")
		return false
	}

	scoreTh := soc.DefaultDialogueScoreThreshold
	if scoreTh <= 0 {
		scoreTh = 35
	}
	if cfg.NpcNpcDialogueScoreThreshold > 0 {
		scoreTh = cfg.NpcNpcDialogueScoreThreshold
	}
	scoreDisabled := cfg.NpcNpcDialogueScoreThreshold < 0
	var dialogueScorePtr *ai.DialogueScoreDetail
	if !scoreDisabled {
		recentLinesA := db.RecentNpcNpcArchivalLinesForEntity(A.ID, 10)
		recentLinesB := db.RecentNpcNpcArchivalLinesForEntity(B.ID, 10)
		recentLines := append(append([]string{}, recentLinesA...), recentLinesB...)
		rawEv := ai.RawEventsForDialogueScore(recentEvents)
		sc := ai.ScoreNpcDialogue(lineA, lineB, roomName, rawEv, topicHint, dyadRelationHint(dyad), npcNpcMemory, nameA, nameB, recentLines)
		dialogueScorePtr = &sc
		bucket := (sc.Total / 10) * 10
		if bucket < 0 {
			bucket = 0
		}
		if bucket > 100 {
			bucket = 100
		}
		incStat("score_total_" + strconv.Itoa(bucket))
		if sc.Total < scoreTh {
			incStat("fail_score_too_low")
			setLastChoice(&LastSocialChoice{
				At:             nowUnix,
				RoomID:         roomID,
				AID:            A.ID,
				BID:            B.ID,
				ScoreTotal:     best.Score,
				TopicHint:      topicHint,
				TopicID:        chosenTopicID,
				TopicReason:    topicSource,
				AnchorsUsed:    anchorsUsed,
				AnchorConflict: false,
				DialogueScore:  dialogueScorePtr,
			})
			return false
		}
		incStat("score_pass")
	} else {
		incStat("score_disabled_skip")
	}

	summary := fmt.Sprintf(ns.SummaryFmt, roomName, nameA, gametext.TruncRune(lineA, 25), nameB, gametext.TruncRune(lineB, 25))
	if shouldSkipNpcNpcSummaryUpdate(npcNpcMemory, summary, lineA, lineB) {
		incStat("skip_npc_npc_summary_dup")
	} else {
		_ = db.SetNpcNpcConversationSummary(A.ID, B.ID, summary)
	}
	if _, dup := db.InsertNpcNpcDialogueArchival(A.ID, fmt.Sprintf(ns.ArchivalWithFmt, nameB, gametext.TruncRune(lineA, 40))); dup {
		incStat("skip_npc_npc_archival_dup")
	}
	if _, dup := db.InsertNpcNpcDialogueArchival(B.ID, fmt.Sprintf(ns.ArchivalWithFmt, nameA, gametext.TruncRune(lineB, 40))); dup {
		incStat("skip_npc_npc_archival_dup")
	}
	thread := best.Thread
	if thread == nil {
		thread = &store.NpcThread{
			TopicType: topicHint,
			Phase:     "opening",
		}
	}
	thread.TurnCount++
	thread.UpdatedAt = nowUnix
	if thread.TopicType == "" {
		thread.TopicType = topicHint
	}
	if thread.TurnCount >= 1 {
		thread.Phase = "elaborate"
	}
	if thread.TurnCount >= 3 {
		thread.Phase = "cooling"
		thread.CooldownUntil = nowUnix + 300
	}
	thread.Anchors = mergeThreadAnchors(thread.Anchors, anchors)
	_ = db.SetNpcNpcThread(A.ID, B.ID, thread)
	for _, a := range anchors {
		a = strings.TrimSpace(a)
		if a == "" {
			continue
		}
		aLow := strings.ToLower(a)
		dar := config.Sim().DialogueAnchorRumor
		minR := dar.MinAnchorRunes
		if minR <= 0 {
			minR = 4
		}
		if len([]rune(a)) < minR ||
			strings.Contains(aLow, "npc_") || strings.Contains(aLow, "entity_") ||
			strings.Contains(aLow, "room_") || strings.Contains(aLow, "street") ||
			strings.Contains(aLow, "zone") {
			continue
		}
		rid := fmt.Sprintf("%s|%s|%s", roomID, roomZone, a)
		ss := dar.SourceScore
		if ss <= 0 {
			ss = 1
		}
		w := dar.Weight
		if w <= 0 {
			w = 2
		}
		ttl := int64(dar.TTLSec)
		if ttl <= 0 {
			ttl = 3600
		}
		_ = db.UpsertNpcRumor(&store.NpcRumor{
			ID:          rid,
			Text:        a,
			RoomID:      roomID,
			Zone:        roomZone,
			Source:      "dialogue_anchor",
			SourceScore: ss,
			Weight:      w,
			UpdatedAt:   nowUnix,
			ExpiresAt:   nowUnix + ttl,
		})
	}
	SetPairLastTalk(A.ID, B.ID, nowUnix)
	if dyad != nil {
		du := config.Sim().DyadUpdate
		cap := du.SentimentCap
		if cap <= 0 {
			cap = 100
		}
		dyad.Familiarity += du.FamiliarityDeltaPerExchange
		if dyad.Familiarity > cap {
			dyad.Familiarity = cap
		}
		dyad.Sentiment += SentimentDeltaFromDialogue(topicHint, lineA, lineB, dyad.Familiarity, chosenTopicID)
		if dyad.Sentiment > cap {
			dyad.Sentiment = cap
		}
		if dyad.Sentiment < -cap {
			dyad.Sentiment = -cap
		}
		if NpcPairSameVenue(A.ID, B.ID) {
			dyad.Tags = addTag(dyad.Tags, gametext.DyadTag("same_venue"))
		}
		qmax := du.QuarrelTagSentimentMax
		if qmax == 0 {
			qmax = -30
		}
		if dyad.Sentiment <= qmax {
			dyad.Tags = addTag(dyad.Tags, gametext.DyadTag("had_quarrel"))
		}
		cmin := du.ClearQuarrelSentimentMin
		if cmin == 0 {
			cmin = 25
		}
		if dyad.Sentiment >= cmin {
			dyad.Tags = removeTag(dyad.Tags, gametext.DyadTag("had_quarrel"))
		}
		dyad.UpdatedAt = nowUnix
		_ = db.SetNpcNpcDyad(A.ID, B.ID, dyad)
	}
	setLastChoice(&LastSocialChoice{
		At:             nowUnix,
		RoomID:         roomID,
		AID:            A.ID,
		BID:            B.ID,
		ScoreTotal:     best.Score,
		TopicHint:      topicHint,
		TopicID:        chosenTopicID,
		TopicReason:    topicSource,
		AnchorsUsed:    anchorsUsed,
		AnchorsWritten: anchors,
		AnchorConflict: false,
		DialogueScore:  dialogueScorePtr,
	})
	skipSec := time.Duration(soc.BroadcastSkipRecentTalkSec) * time.Second
	if sessionStore.RoomHasPlayerWithRecentTalk(roomID, skipSec) {
		return true
	}
	text := fmt.Sprintf(ns.BroadcastFmt, nameA, nameB, lineA, nameB, lineB)
	server.SendNarrateToRoom(sessionStore, roomID, text)
	incStat("success")
	return true
}
