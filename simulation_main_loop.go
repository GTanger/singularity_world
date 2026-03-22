// 主程式：NPC 模擬、排班、移動、閒置與 game.Loop tick（自 main 抽出）。
package main

import (
	"fmt"
	"log"
	"math/rand"
	"time"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/game"
	"singularity_world/gametext"
	"singularity_world/npc"
	"singularity_world/npcnpc"
	"singularity_world/server"
	"singularity_world/store"
)

// startSimulationMainLoop 註冊 Observer、建圖、TravelerManager，並啟動主遊戲 tick。
func startSimulationMainLoop(cfg config.Server, sessionStore *server.SessionStore) {
	// 視野內 NPC 即時模擬 ＋ 每 tick 推進移動中實體（§1.2.3、§1.3.3）。§七 7.1：房間制觀測經 Observer。
	obs := &game.Observed{}
	server.SetDefaultObserver(obs)
	var lastScheduleHour = -1
	var lastExpenseDay = -1
	var lastSpawnCheck = time.Now()

	// NPC 活化：閒置動作 & 巡邏計時器（中頻 5-12 真實秒，即 2-5 遊戲分鐘）
	npc.LoadBehaviors("data/npc_behaviors.json")
	db.LoadOccupations("data/templates/occupations.json")
	npc.LoadNpcNpcTopics("data/npc_to_npc_topics.json")
	// 房間可互動物件：僅從各房間 JSON 的 objects 欄位載入
	if store.Default != nil {
		for _, id := range store.Default.RoomIDs() {
			r, _ := store.Default.GetRoom(id)
			if r != nil && len(r.Objects) > 0 {
				db.SetObjectsForRoom(id, r.Objects)
			}
		}
	}
	var idleTickCount int
	sim := config.Sim()
	idle := sim.Idle
	idleFirstSpan := idle.FirstTriggerSpan
	if idleFirstSpan <= 0 {
		idleFirstSpan = 1
	}
	nextIdleTrigger := idle.FirstTriggerMin + rand.Intn(idleFirstSpan)
	rdt := sim.RandomNpcDialogueTicks
	rdInitSpan := rdt.InitialSpan
	if rdInitSpan <= 0 {
		rdInitSpan = 1
	}
	var randomNpcDialogueTicksLeft int = rdt.InitialMin + rand.Intn(rdInitSpan)
	var lastJobMatching time.Time
	var lastRumorDecay time.Time
	var lastRumorDigestBuild time.Time

	// 尋路引擎：建立房間鄰接圖
	roomGraph := db.GetGraph()
	if err := roomGraph.BuildGraph(); err != nil {
		log.Printf("[pathfind] build graph failed: %v", err)
	}

	// 地圖型 NPC 移動管理器：排班型（經理等）＋腦驅動型（無排班 NPC 依 Decide→Intent 尋路）
	travelerMgr := npc.NewTravelerManager()
	schedules, _ := db.GetAllSchedules()
	scheduledIDs := make(map[string]bool)
	for _, s := range schedules {
		c, _ := db.GetEntity(s.EntityID)
		if c == nil || c.Vit <= 0 {
			continue
		}
		scheduledIDs[s.EntityID] = true
		title := db.GetNPCTitleFromAssignments(s.EntityID)
		if title == "" {
			title = gametext.DefaultManagerTitle()
		}
		def := npc.GetMovementDefForTitle(title)
		def.Type = npc.MoveSchedule
		travelerMgr.Register(s.EntityID, def)
	}
	npcIDs, _ := db.GetNPCIDsWithRoom()
	for _, id := range npcIDs {
		if scheduledIDs[id] {
			continue
		}
		travelerMgr.Register(id, npc.MovementDef{Type: npc.MoveBrain, Speed: 1})
	}
	var travelTickCount int
	travelTickInterval := 30 // 每 15 秒推進一步（30 ticks × 500ms）

	go game.Loop(cfg.TickInterval, func() {
		game.RunViewSimulation(func() []game.Pos { return server.GetObserverPositions(sessionStore) }, obs)

		now := time.Now().Unix()
		_, hour, _, gameDay := game.GameTimeNow(now, cfg.GameTimeEpochUnix, cfg.GameTimeScale)
		rdSec := config.Sim().RumorDecayIntervalSec
		if rdSec <= 0 {
			rdSec = 30
		}
		if time.Since(lastRumorDecay) >= time.Duration(rdSec)*time.Second {
			lastRumorDecay = time.Now()
			_ = db.DecayNpcRumors(now)
		}
		rdMin := config.Sim().RumorDigestIntervalMin
		if rdMin <= 0 {
			rdMin = 2
		}
		if time.Since(lastRumorDigestBuild) >= time.Duration(rdMin)*time.Minute {
			lastRumorDigestBuild = time.Now()
			_ = db.BuildNpcRumorDigest(now)
		}

		// 觸發－輕：隨機時點在某一有玩家的房觸發 NPC 間對話（玩家在場時間隔略拉長，減少洗頻）
		randomNpcDialogueTicksLeft--
		if randomNpcDialogueTicksLeft <= 0 && cfg.OllamaBaseURL != "" && cfg.OllamaModel != "" {
			playerRoomsForRandom := server.GetPlayerRoomMap(sessionStore)
			if len(playerRoomsForRandom) > 0 {
				rp := config.Sim().RandomNpcDialogueTicks
				rsp := rp.ResetSpanWithPlayer
				if rsp <= 0 {
					rsp = 1
				}
				randomNpcDialogueTicksLeft = rp.ResetMinWithPlayer + rand.Intn(rsp)
				rooms := make([]string, 0, len(playerRoomsForRandom))
				for r := range playerRoomsForRandom {
					rooms = append(rooms, r)
				}
				roomID := rooms[rand.Intn(len(rooms))]
				topicHint := ""
				if t := npc.PickRandomNpcNpcTopicByMask(npcnpc.TopicMaskForRoom(roomID, hour), ""); t != nil {
					topicHint = t.Hint
				}
				npcnpc.TryTriggerNpcNpcInRoom(sessionStore, cfg, roomID, topicHint, hour)
			} else {
				minE := cfg.NpcNpcSocialTickMinNoPlayer
				if minE <= 0 {
					minE = 80
				}
				extraE := cfg.NpcNpcSocialTickExtraNoPlayer
				if extraE <= 0 {
					extraE = 40
				}
				randomNpcDialogueTicksLeft = minE + rand.Intn(extraE)
			}
		}

		// NPC 每日消耗：每遊戲日扣食宿鎂（第四個回傳值為 gameDay，勿與 min 搞混）
		if gameDay != lastExpenseDay {
			lastExpenseDay = gameDay
			db.DeductDailyExpense()
			ecoMod := config.Sim().EconomyPulse.GameDayModulo
			if ecoMod <= 0 {
				ecoMod = 3
			}
			if gameDay%ecoMod == 0 {
				ecoLines := gametext.EconomyRumorLines()
				if len(ecoLines) > 0 {
					ecoText := ecoLines[rand.Intn(len(ecoLines))]
					ecoTTL := int64(config.Sim().EconomyPulse.RumorTTLSec)
					if ecoTTL <= 0 {
						ecoTTL = 5400
					}
					_ = db.UpsertNpcRumor(&store.NpcRumor{
						ID:          fmt.Sprintf("eco|pulse|%d", gameDay/ecoMod),
						Text:        ecoText,
						Source:      "economy",
						SourceScore: 1,
						Weight:      1,
						UpdatedAt:   now,
						ExpiresAt:   now + ecoTTL,
					})
				}
			}
		}

		// NPC 池：總量＝玩家＋NPC；固定間隔檢查，未滿則生成一名並註冊腦驅動（男女數持平）
		if cfg.NPCPoolSize > 0 && cfg.NPCSpawnIntervalSec > 0 && time.Since(lastSpawnCheck) >= time.Duration(cfg.NPCSpawnIntervalSec)*time.Second {
			lastSpawnCheck = time.Now()
			npcIDs, _ := db.GetNPCIDsWithRoom()
			playerIDs, _ := db.GetPlayerIDsWithRoom()
			totalInWorld := len(npcIDs) + len(playerIDs)
			if totalInWorld < cfg.NPCPoolSize {
				spawnRoom := db.GetSpawnRoomID()
				if newID, err := db.SpawnOneNPCFromPool(spawnRoom); err == nil && newID != "" {
					travelerMgr.Register(newID, npc.MovementDef{Type: npc.MoveBrain, Speed: 1})
					spawnZone := ""
					if room, _ := db.GetRoom(spawnRoom); room != nil {
						spawnZone = room.Zone
					}
					spawnTTL := int64(config.Sim().SpawnRumorTTLSec)
					if spawnTTL <= 0 {
						spawnTTL = 3600
					}
					_ = db.UpsertNpcRumor(&store.NpcRumor{
						ID:          fmt.Sprintf("spawn|%s|%s", spawnRoom, newID),
						Text:        gametext.NpcSocial().NewcomerRumor,
						RoomID:      spawnRoom,
						Zone:        spawnZone,
						Source:      "spawn",
						SourceScore: 1,
						Weight:      1,
						UpdatedAt:   now,
						ExpiresAt:   now + spawnTTL,
					})
				}
			}
		}

		jmSec := config.Sim().JobMatchingIntervalSec
		if jmSec <= 0 {
			jmSec = 30
		}
		if time.Since(lastJobMatching) >= time.Duration(jmSec)*time.Second {
			lastJobMatching = time.Now()
			mgThreshold := cfg.SeekJobMgThreshold
			if mgThreshold <= 0 {
				mgThreshold = npc.SeekJobMgThresholdDefault
			}
			added := npc.RunJobMatchingTick(roomGraph, npc.JobMatchParams{
				MaxPerVenue:        config.Sim().MaxAssignmentsPerVenue,
				MgThreshold:        mgThreshold,
				JobMatchWhenStable: cfg.JobMatchWhenStable,
			})
			for _, entityID := range added {
				if _, ok := db.GetScheduleForEntity(entityID); ok {
					title := db.GetNPCTitleFromAssignments(entityID)
					if title == "" {
						title = gametext.DefaultManagerTitle()
					}
					def := npc.GetMovementDefForTitle(title)
					def.Type = npc.MoveSchedule
					travelerMgr.Register(entityID, def)
				}
			}
			if len(added) > 0 {
				jobTTL := int64(config.Sim().JobMatchRumorTTLSec)
				if jobTTL <= 0 {
					jobTTL = 7200
				}
				_ = db.UpsertNpcRumor(&store.NpcRumor{
					ID:          fmt.Sprintf("job|%d", now/1800),
					Text:        fmt.Sprintf(gametext.NpcSocial().JobMatchRumorFmt, len(added)),
					Source:      "job",
					SourceScore: 4,
					Weight:      2,
					UpdatedAt:   now,
					ExpiresAt:   now + jobTTL,
				})
			}
		}

		// NPC 排班：每遊戲小時僅發「出發」敘事；實際移動由 TravelerManager 排班型尋路逐格執行（家可十格外）
		if hour != lastScheduleHour {
			lastScheduleHour = hour
			{
				var dispDelta int
				tod := config.Sim().DispositionTimeOfDay
				ds, de := tod.DawnHourStart, tod.DawnHourEnd
				if ds == 0 && de == 0 {
					ds, de = 6, 9
				}
				if hour >= ds && hour <= de {
					dispDelta = tod.DawnDelta
				} else if le := tod.LateNightHourEnd; hour >= 0 && hour <= le {
					dispDelta = tod.LateNightDelta
				}
				if dispDelta != 0 {
					if activeIDs, err := db.GetNPCIDsWithRoom(); err == nil {
						for _, nid := range activeIDs {
							db.AdjustDisposition(nid, dispDelta)
						}
					}
				}
			}
			moves, err := db.ApplySchedules(hour)
			if err == nil {
				rf := gametext.RuntimeFmt()
				for _, m := range moves {
					target, _ := db.GetScheduleTarget(m.EntityID, hour)
					occ := db.GetNPCTitleFromAssignments(m.EntityID)
					person := db.GetNPCPersonDisplayName(m.EntityID)
					if person == "" {
						person = m.EntityID
					}
					var leaveText string
					if target.Room == m.NewRoom && target.IsWork {
						leaveText = fmt.Sprintf(rf.ShiftLeaveToShopFmt, person)
					} else {
						if occ != "" {
							leaveText = npc.GetShiftFlavor(occ, person, false)
						}
						if leaveText == "" {
							leaveText = fmt.Sprintf(rf.ShiftLeaveFmt, person)
						}
					}
					server.SendNarrateToRoom(sessionStore, m.OldRoom, leaveText)
					newRoomName, _ := db.GetRoomName(m.NewRoom)
					if newRoomName == "" {
						newRoomName = m.NewRoom
					}
					npcnpc.PushRoomEvent(m.OldRoom, "shift_leave", person, fmt.Sprintf(rf.PushShiftLeaveDetailFmt, newRoomName))
					npcnpc.PushRoomEvent(m.NewRoom, "shift_arrive", person, rf.ShiftArrive)
				}
				if len(moves) > 0 {
					server.BroadcastRoomViews(sessionStore, cfg)
					// 觸發－中：排班時段有動靜的房，試觸發一次 NPC 間對話（主題：交班）
					topicHint := ""
					if t := npc.GetNpcNpcTopicByID(gametext.TopicID("shift_handover")); t != nil {
						topicHint = t.Hint
					}
					roomIDsWithActivity := make(map[string]bool)
					for _, m := range moves {
						roomIDsWithActivity[m.OldRoom] = true
						roomIDsWithActivity[m.NewRoom] = true
					}
					for roomID := range roomIDsWithActivity {
						hint := topicHint
						if hint == "" {
							if t := npc.PickRandomNpcNpcTopicByMask(npcnpc.TopicMaskForRoom(roomID, hour), ""); t != nil {
								hint = t.Hint
							}
						}
						if npcnpc.TryTriggerNpcNpcInRoom(sessionStore, cfg, roomID, hint, hour) {
							break
						}
					}
				}
			}
		}

		// 地圖型 NPC 移動：每 travelTickInterval 推進一步；僅驅動「觀測圈」內 NPC（玩家房＋鄰房）
		travelTickCount++
		if travelTickCount >= travelTickInterval {
			travelTickCount = 0
			activeRoomIDs := buildActiveRoomIDs(sessionStore, roomGraph)
			if len(activeRoomIDs) == 0 {
				// 無人觀測：事實仍持續。排班者依 gameHour 朝 work/rest 走一步；無排班者抽樣加鎂／採集／求職／隨機鄰房。不發敘事。
				npc.RunUnobservedWorldTick(roomGraph, hour, npc.UnobservedMaxNPCsPerTick)
			}
			travelSteps := travelerMgr.Tick(roomGraph, hour, activeRoomIDs)
			rf := gametext.RuntimeFmt()
			for _, step := range travelSteps {
				oldName := roomGraph.RoomName(step.OldRoom)
				newName := roomGraph.RoomName(step.NewRoom)
				leaveText := fmt.Sprintf(rf.TravelLeaveFmt, step.NpcName, newName)
				arriveText := fmt.Sprintf(rf.TravelArriveFmt, step.NpcName, oldName)
				if target, ok := db.GetScheduleTarget(step.EntityID, hour); ok && target.Room == step.NewRoom {
					if target.IsWork {
						occ := db.GetNPCTitleFromAssignments(step.EntityID)
						if occ == "" {
							occ = gametext.OccupationClerk()
						}
						arriveText = npc.GetShiftFlavor(occ, db.GetNPCPersonDisplayName(step.EntityID), true)
					} else {
						arriveText = fmt.Sprintf(rf.WanderArriveHome, step.NpcName)
					}
				}
				server.SendNarrateToRoom(sessionStore, step.OldRoom, leaveText)
				server.SendNarrateToRoom(sessionStore, step.NewRoom, arriveText)
				npcnpc.PushRoomEvent(step.OldRoom, "leave", step.NpcName, fmt.Sprintf(rf.PushLeaveFmt, newName))
				npcnpc.PushRoomEvent(step.NewRoom, "arrive", step.NpcName, fmt.Sprintf(rf.PushArriveFmt, oldName))
				// 腦驅動到達後行為：敘事＋真實效果（Beg 加鎂、Gather 加物品、SeekJob 撮合）
				// 大腦可見化：NPC 決定新意圖時的出發敘事，發至出發房間
				if step.DecisionNarrative != "" {
					server.SendNarrateToRoom(sessionStore, step.OldRoom, step.DecisionNarrative)
				}
				if step.ArrivalIntent != "" {
					arrivalNarrative := gametext.BrainArrival(string(step.ArrivalIntent), step.NpcName)
					if arrivalNarrative != "" {
						server.SendNarrateToRoom(sessionStore, step.NewRoom, arrivalNarrative)
					}
					applyBrainArrivalEffects(step.EntityID, step.NewRoom, step.ArrivalIntent)
				}
				server.RefreshRoomViews(sessionStore, cfg, step.OldRoom)
				server.RefreshRoomViews(sessionStore, cfg, step.NewRoom)
			}
		}

		// NPC 閒置動作 & 巡邏：計時器到期後觸發
		idleTickCount++
		if idleTickCount >= nextIdleTrigger {
			idleTickCount = 0
			idle2 := config.Sim().Idle
			isp := idle2.IntervalSpan
			if isp <= 0 {
				isp = 1
			}
			nextIdleTrigger = idle2.IntervalMin + rand.Intn(isp)
			period := npc.GetTimePeriod(hour)

			playerRooms := server.GetPlayerRoomMap(sessionStore)
			schedules, _ := db.GetAllSchedules()

			// 觸發－重：閒置 tick 時同房兩 NPC 一來一往（主題劇本＋AI、寫記憶）
			npcDialogueDone := false
			if cfg.OllamaBaseURL != "" && cfg.OllamaModel != "" {
				for roomID := range playerRooms {
					topicHint := ""
					if t := npc.PickRandomNpcNpcTopicByMask(npcnpc.TopicMaskForRoom(roomID, hour), ""); t != nil {
						topicHint = t.Hint
					}
					if npcnpc.TryTriggerNpcNpcInRoom(sessionStore, cfg, roomID, topicHint, hour) {
						npcDialogueDone = true
						break
					}
				}
			}
			if !npcDialogueDone {
				// Fallback：微互動（15% 機率），每輪最多一次
				for roomID := range playerRooms {
					microPct := config.Sim().MicroInteractionChancePercent
					if microPct <= 0 {
						microPct = 15
					}
					social := npc.PickMicroInteraction(roomID, microPct, hour)
					if social != "" {
						server.SendNarrateToRoom(sessionStore, roomID, social)
						break
					}
				}
			}

			rfWander := gametext.RuntimeFmt()
			for _, s := range schedules {
				if !s.IsOnDuty(hour) {
					continue
				}
				npcRoom, _ := db.GetEntityRoom(s.EntityID)
				title := db.GetNPCTitleFromAssignments(s.EntityID)
				if title == "" {
					title = gametext.DefaultManagerTitle()
				}
				person := db.GetNPCPersonDisplayName(s.EntityID)

				// 巡邏：10% 機率移動到 wander_rooms 中的另一房間
				wanderRooms := npc.GetWanderRooms(title)
				wMax := config.Sim().WanderRollMax
				if wMax <= 0 {
					wMax = 10
				}
				if len(wanderRooms) > 1 && rand.Intn(wMax) == 0 {
					var candidates []string
					for _, wr := range wanderRooms {
						if wr != npcRoom {
							candidates = append(candidates, wr)
						}
					}
					if len(candidates) > 0 {
						dest := candidates[rand.Intn(len(candidates))]
						destName, _ := db.GetRoomName(dest)
						srcName, _ := db.GetRoomName(npcRoom)
						leaveText := npc.GetWanderFlavor(title, person, destName, true)
						arriveText := npc.GetWanderFlavor(title, person, srcName, false)
						server.SendNarrateToRoom(sessionStore, npcRoom, leaveText)
						_ = db.SetEntityRoom(s.EntityID, dest)
						server.SendNarrateToRoom(sessionStore, dest, arriveText)
						npcnpc.PushRoomEvent(npcRoom, "leave", person, fmt.Sprintf(rfWander.PushLeaveFmt, destName))
						npcnpc.PushRoomEvent(dest, "arrive", person, fmt.Sprintf(rfWander.PushArriveFmt, srcName))
						server.RefreshRoomViews(sessionStore, cfg, npcRoom)
						server.RefreshRoomViews(sessionStore, cfg, dest)
						continue
					}
				}

				// 閒置動作：僅對有玩家在場的房間觸發
				if _, hasPlayer := playerRooms[npcRoom]; !hasPlayer {
					continue
				}
				disp := db.GetDisposition(s.EntityID)
				emote := npc.PickIdleEmote(title, period, person, disp)
				if emote != "" {
					server.SendNarrateToRoom(sessionStore, npcRoom, emote)
					break
				}
			}
		}
	})
}
