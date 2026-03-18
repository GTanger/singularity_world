// 程式入口：啟動 HTTP 伺服器、WebSocket、靜態檔與 DB，對齊第一版可做清單 §1.1。
package main

import (
	"database/sql"
	"fmt"
	"log"
	"math/rand"
	"net/http"
	"net/http/httputil"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"time"

	"singularity_world/ai"
	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/entity"
	"singularity_world/economy"
	"singularity_world/game"
	"singularity_world/server"
	"singularity_world/store"

	"github.com/gorilla/websocket"
)

// brainArrivalNarrative 腦驅動 NPC 抵達意圖目標時發送的敘事。
func brainArrivalNarrative(npcName string, intent db.IntentType) string {
	switch intent {
	case db.IntentBeg:
		return "【" + npcName + "】在此地向路人乞討。"
	case db.IntentGather:
		return "【" + npcName + "】開始在附近採集。"
	case db.IntentSeekJob:
		return "【" + npcName + "】前來打聽是否有活可做。"
	case db.IntentTrade:
		return "【" + npcName + "】在此地擺開貨物。"
	default:
		return ""
	}
}

// maxAssignmentsPerVenue 求職撮合時，單一場所最多指派數（無 max_staff 時用此常數）。
const maxAssignmentsPerVenue = 2

// truncRune 截斷為最多 n 個 rune，超出補「…」。
func truncRune(s string, n int) string {
	r := []rune(s)
	if len(r) <= n {
		return s
	}
	return string(r[:n]) + "…"
}

// tryTriggerNpcNpcInRoom 在指定房嘗試觸發一次 NPC 間 AI 對話；topicHint 可為「交班」等或空。成功則寫入記憶並回傳 true。
func tryTriggerNpcNpcInRoom(database *sql.DB, sessionStore *server.SessionStore, cfg config.Server, roomID, topicHint string) bool {
	if cfg.OllamaBaseURL == "" || cfg.OllamaModel == "" {
		return false
	}
	if sessionStore.RoomHasPlayerWithRecentTalk(database, roomID, 60*time.Second) {
		return false
	}
	entities, _ := db.GetEntitiesInRoom(database, roomID)
	var npcs []*entity.Character
	for _, e := range entities {
		if e != nil && e.Kind == "npc" {
			npcs = append(npcs, e)
		}
	}
	if len(npcs) < 2 {
		return false
	}
	i := rand.Intn(len(npcs))
	j := i
	for j == i {
		j = rand.Intn(len(npcs))
	}
	A, B := npcs[i], npcs[j]
	nameA, nameB := A.DisplayTitle, B.DisplayTitle
	if nameA == "" {
		nameA = A.ID
	}
	if nameB == "" {
		nameB = B.ID
	}
	roomName, _ := db.GetRoomName(database, roomID)
	hour := 12
	if cfg.GameTimeEpochUnix != 0 {
		_, hour, _, _ = game.GameTimeNow(time.Now().Unix(), cfg.GameTimeEpochUnix, cfg.GameTimeScale)
	}
	timeLabel := "此時"
	if hour >= 5 && hour < 10 {
		timeLabel = "清晨"
	} else if hour >= 10 && hour < 14 {
		timeLabel = "正午"
	} else if hour >= 14 && hour < 18 {
		timeLabel = "傍晚"
	} else {
		timeLabel = "夜裡"
	}
	backstoryA := db.BuildIdentity(database, A.ID)
	backstoryB := db.BuildIdentity(database, B.ID)
	npcNpcMemory := db.GetNpcNpcConversationSummary(A.ID, B.ID)
	lineA, lineB, err := ai.CallAITalkNPCToNPC(cfg.OllamaBaseURL, cfg.OllamaModel, nameA, nameB, backstoryA, backstoryB, roomName, timeLabel, npcNpcMemory, topicHint)
	if err != nil || lineA == "" || lineB == "" {
		return false
	}
	summary := "在" + roomName + "：" + nameA + "說「" + truncRune(lineA, 25) + "」" + nameB + "回「" + truncRune(lineB, 25) + "」"
	_ = db.SetNpcNpcConversationSummary(A.ID, B.ID, summary)
	if sessionStore.RoomHasPlayerWithRecentTalk(database, roomID, 15*time.Second) {
		return true // 已寫記憶，不播
	}
	text := "【" + nameA + "】對【" + nameB + "】說：「" + lineA + "」【" + nameB + "】說：「" + lineB + "」"
	server.SendNarrateToRoom(sessionStore, database, roomID, text)
	return true
}

// buildActiveRoomIDs 回傳「觀測圈」：有玩家的房間＋其鄰房；僅這些房內的 NPC 會被腦／移動驅動。無人時回傳空 map。
func buildActiveRoomIDs(sessionStore *server.SessionStore, database *sql.DB, roomGraph *db.RoomGraph) map[string]bool {
	playerRooms := server.GetPlayerRoomMap(sessionStore, database)
	out := make(map[string]bool)
	for rid := range playerRooms {
		out[rid] = true
		for _, nb := range roomGraph.Neighbors(rid) {
			out[nb] = true
		}
	}
	return out
}

// applyBrainArrivalEffects 依意圖套用真實效果：Beg 加鎂、Gather 加物品、SeekJob 嘗試寫入指派；並記事件與心境。
func applyBrainArrivalEffects(database *sql.DB, entityID, roomID string, intent db.IntentType) {
	switch intent {
	case db.IntentBeg:
		amount := 3 + rand.Intn(8) // 3～10 鎂
		_ = db.AddMagnesium(database, entityID, amount)
		db.LogNPCEvent(entityID, db.EvtBeg, fmt.Sprintf("在%s乞討，得%d鎂", roomID, amount))
		db.AdjustDisposition(database, entityID, db.DispBegSuccess)
	case db.IntentGather:
		_ = db.AddToInventory(database, entityID, "wild_herb", 1)
		db.LogNPCEvent(entityID, db.EvtGather, fmt.Sprintf("在%s採集野草", roomID))
		db.AdjustDisposition(database, entityID, db.DispGather)
	case db.IntentSeekJob:
		assignments, _ := db.GetAssignmentsForEntity(database, entityID)
		if len(assignments) > 0 {
			return
		}
		venueIDs, _ := db.GetVenueIDsForRoom(database, roomID)
		for _, vid := range venueIDs {
			n, _ := db.GetAssignmentCountByVenue(database, vid)
			if n < maxAssignmentsPerVenue {
				if err := db.InsertAssignment(database, entityID, "服務生", vid, ""); err == nil {
					db.LogNPCEvent(entityID, db.EvtHired, fmt.Sprintf("在%s獲得%s職位", roomID, "服務生"))
					db.AdjustDisposition(database, entityID, db.DispHired)
					break
				}
			}
		}
	}
}

func main() {
	cfg := config.DefaultServer()

	if err := os.MkdirAll("data", 0755); err != nil {
		log.Fatalf("mkdir data: %v", err)
	}
	if err := os.MkdirAll("data/runtime", 0755); err != nil {
		log.Fatalf("mkdir data/runtime: %v", err)
	}

	// 全專案以 JSON 為唯一數據源：載入 store（data/rooms 目錄一房一檔 + runtime + data）
	if err := store.Init("data/rooms", "data/runtime", "data"); err != nil {
		log.Fatalf("store init: %v", err)
	}
	// store 模式下確保預設 NPC 存在（試話等）；若 data/entities.json 無則建立並寫入 store
	if err := db.SeedNPCsForStore(nil); err != nil {
		log.Printf("seed npcs for store: %v", err)
	}
	defer func() {
		if store.Default != nil {
			if err := store.Default.FlushEntities(); err != nil {
				log.Printf("flush entities on exit: %v", err)
			}
		}
	}()

	var database *sql.DB // nil：不再使用 DB 檔，所有讀寫經由 store

	hub := server.NewHub(cfg.MaxWebSocketConn)
	sessionStore := server.NewSessionStore()
	upgrader := websocket.Upgrader{CheckOrigin: func(r *http.Request) bool { return true }}

	http.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		conn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			log.Printf("upgrade: %v", err)
			if strings.Contains(err.Error(), "upgrade") && strings.Contains(err.Error(), "Connection") {
				log.Printf("[ws] 提示：若經由 Cloudflare Tunnel/反向代理，請確認已轉傳 Connection: Upgrade 與 Upgrade: websocket")
			}
			return
		}
		client := server.NewClient(conn)
		if !hub.Register(client) {
			_ = conn.WriteMessage(websocket.CloseMessage,
				websocket.FormatCloseMessage(websocket.ClosePolicyViolation, "max connections reached"))
			_ = conn.Close()
			return
		}
		onClose := func(c *server.Client) {
			if c.PlayerID != "" {
				if s := sessionStore.Get(c.PlayerID); s != nil && s.Client == c {
					sessionStore.Remove(c.PlayerID)
				}
			}
			hub.Unregister(c)
		}
		go server.ReadLoop(client, onClose, database, cfg, sessionStore, hub)
	})

	http.HandleFunc("/api/design-constants", config.ServeDesignConstants)
	// 房間 API：/api/rooms（列表）與 /api/rooms/xxx（單一房間操作）皆由同一 handler 處理，避免 PUT /api/rooms/lobby 路由錯誤。
	roomsAPI := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { server.HandleRoomsAPI(database, w, r) })
	http.Handle("/api/rooms/", roomsAPI)
	http.HandleFunc("/api/rooms", roomsAPI.ServeHTTP)
	http.HandleFunc("/api/admin/wipe-entities", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "POST only", http.StatusMethodNotAllowed)
			return
		}
		if err := db.DeleteAllEntities(database); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"ok":true,"message":"已刪除所有角色"}`))
	})
	// 地圖檢視器：/map_viewer，資料由 /data/rooms.json API 從 store 彙總
	http.HandleFunc("/map_viewer", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/map_viewer" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		w.Header().Set("Cache-Control", "no-cache, max-age=0, must-revalidate")
		http.ServeFile(w, r, filepath.Join("web", "map_viewer.html"))
	})
	http.HandleFunc("/data/rooms.json", server.HandleRoomsDataAPI)
	// 星盤檢視器：/star_chart，資料由 /api/topology 提供（store）
	http.HandleFunc("/star_chart", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/star_chart" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		w.Header().Set("Cache-Control", "no-cache, max-age=0, must-revalidate")
		http.ServeFile(w, r, filepath.Join("web", "star_chart.html"))
	})
	http.HandleFunc("/api/topology", func(w http.ResponseWriter, r *http.Request) {
		server.HandleTopologyAPI(database, w, r)
	})
	http.HandleFunc("/api/player-room", func(w http.ResponseWriter, r *http.Request) {
		server.HandlePlayerRoomAPI(database, w, r)
	})

	// Chatmery Web：/chatmery 轉發至 localhost:1722（Tunnel 只指 1721 時由奇點代轉）
	if chatmeryURL, err := url.Parse("http://127.0.0.1:1722"); err == nil {
		chatmeryProxy := httputil.NewSingleHostReverseProxy(chatmeryURL)
		http.HandleFunc("/chatmery", func(w http.ResponseWriter, r *http.Request) {
			if r.URL.Path != "/chatmery" {
				chatmeryProxy.ServeHTTP(w, r)
				return
			}
			http.Redirect(w, r, "/chatmery/", http.StatusFound)
		})
		http.Handle("/chatmery/", chatmeryProxy)
	}

	fs := http.FileServer(http.Dir("web"))
	http.Handle("/", http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		p := r.URL.Path
		if strings.HasSuffix(p, ".js") || strings.HasSuffix(p, ".css") || strings.HasSuffix(p, ".html") || p == "/" {
			w.Header().Set("Cache-Control", "no-cache, max-age=0, must-revalidate")
		}
		fs.ServeHTTP(w, r)
	}))

	// 視野內 NPC 即時模擬 ＋ 每 tick 推進移動中實體（§1.2.3、§1.3.3）。
	obs := &game.Observed{DB: database}
	var lastScheduleHour = -1
	var lastExpenseDay = -1
	var lastSpawnCheck = time.Now()

	// NPC 活化：閒置動作 & 巡邏計時器（中頻 5-12 真實秒，即 2-5 遊戲分鐘）
	db.LoadBehaviors("data/npc_behaviors.json")
	db.LoadOccupations("data/templates/occupations.json")
	db.LoadNpcNpcTopics("data/npc_to_npc_topics.json")
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
	nextIdleTrigger := 25 + rand.Intn(35)
	var randomNpcDialogueTicksLeft int = 80 + rand.Intn(40) // 觸發－輕：隨機時點觸發 NPC 間對話

	// 尋路引擎：建立房間鄰接圖
	roomGraph := db.GetGraph()
	if err := roomGraph.BuildGraph(database); err != nil {
		log.Printf("[pathfind] build graph failed: %v", err)
	}

	// 地圖型 NPC 移動管理器：排班型（經理等）＋腦驅動型（無排班 NPC 依 Decide→Intent 尋路）
	travelerMgr := db.NewTravelerManager()
	schedules, _ := db.GetAllSchedules(database)
	scheduledIDs := make(map[string]bool)
	for _, s := range schedules {
		scheduledIDs[s.EntityID] = true
		title := db.GetNPCTitle(database, s.EntityID)
		def := db.GetMovementDefForTitle(title)
		def.Type = db.MoveSchedule
		travelerMgr.Register(s.EntityID, def)
	}
	npcIDs, _ := db.GetNPCIDsWithRoom(database)
	for _, id := range npcIDs {
		if scheduledIDs[id] {
			continue
		}
		travelerMgr.Register(id, db.MovementDef{Type: db.MoveBrain, Speed: 1})
	}
	var travelTickCount int
	travelTickInterval := 30 // 每 15 秒推進一步（30 ticks × 500ms）

	go game.Loop(cfg.TickInterval, func() {
		game.RunViewSimulation(database, func() []game.Pos { return server.GetObserverPositions(sessionStore, database) }, obs)

		now := time.Now().Unix()
		_, hour, _, gameDay := game.GameTimeNow(now, cfg.GameTimeEpochUnix, cfg.GameTimeScale)

		// 觸發－輕：隨機時點在某一有玩家的房觸發 NPC 間對話
		randomNpcDialogueTicksLeft--
		if randomNpcDialogueTicksLeft <= 0 && cfg.OllamaBaseURL != "" && cfg.OllamaModel != "" {
			randomNpcDialogueTicksLeft = 80 + rand.Intn(40)
			playerRoomsForRandom := server.GetPlayerRoomMap(sessionStore, database)
			if len(playerRoomsForRandom) > 0 {
				rooms := make([]string, 0, len(playerRoomsForRandom))
				for r := range playerRoomsForRandom {
					rooms = append(rooms, r)
				}
				roomID := rooms[rand.Intn(len(rooms))]
				topicHint := ""
				if t := db.PickRandomNpcNpcTopic(); t != nil {
					topicHint = t.Hint
				}
				tryTriggerNpcNpcInRoom(database, sessionStore, cfg, roomID, topicHint)
			}
		}

		// NPC 每日消耗：每遊戲日扣食宿鎂（第四個回傳值為 gameDay，勿與 min 搞混）
		if gameDay != lastExpenseDay {
			lastExpenseDay = gameDay
			db.DeductDailyExpense(database)
		}

		// NPC 池：總量＝玩家＋NPC；固定間隔檢查，未滿則生成一名並註冊腦驅動（男女數持平）
		if cfg.NPCPoolSize > 0 && cfg.NPCSpawnIntervalSec > 0 && time.Since(lastSpawnCheck) >= time.Duration(cfg.NPCSpawnIntervalSec)*time.Second {
			lastSpawnCheck = time.Now()
			npcIDs, _ := db.GetNPCIDsWithRoom(database)
			playerIDs, _ := db.GetPlayerIDsWithRoom(database)
			totalInWorld := len(npcIDs) + len(playerIDs)
			if totalInWorld < cfg.NPCPoolSize {
				spawnRoom := db.GetSpawnRoomID(database)
				if newID, err := db.SpawnOneNPCFromPool(database, spawnRoom); err == nil && newID != "" {
					travelerMgr.Register(newID, db.MovementDef{Type: db.MoveBrain, Speed: 1})
				}
			}
		}

		// NPC 排班：每遊戲小時僅發「出發」敘事；實際移動由 TravelerManager 排班型尋路逐格執行（家可十格外）
		if hour != lastScheduleHour {
			lastScheduleHour = hour
			moves, err := db.ApplySchedules(database, hour)
			if err == nil {
				for _, m := range moves {
					target, _ := db.GetScheduleTarget(database, m.EntityID, hour)
					var leaveText string
					if target.Room == m.NewRoom && target.IsWork {
						leaveText = "【" + m.EntityID + "】出門往店裡去了。"
					} else {
						leaveText = db.GetShiftFlavor(m.Title, m.EntityID, false)
					}
					server.SendNarrateToRoom(sessionStore, database, m.OldRoom, leaveText)
				}
				if len(moves) > 0 {
					server.BroadcastRoomViews(sessionStore, database, cfg)
					// 觸發－中：排班時段有動靜的房，試觸發一次 NPC 間對話（主題：交班）
					topicHint := ""
					if t := db.GetNpcNpcTopicByID("交班"); t != nil {
						topicHint = t.Hint
					}
					roomIDsWithActivity := make(map[string]bool)
					for _, m := range moves {
						roomIDsWithActivity[m.OldRoom] = true
						roomIDsWithActivity[m.NewRoom] = true
					}
					for roomID := range roomIDsWithActivity {
						if tryTriggerNpcNpcInRoom(database, sessionStore, cfg, roomID, topicHint) {
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
			activeRoomIDs := buildActiveRoomIDs(sessionStore, database, roomGraph)
			if len(activeRoomIDs) == 0 {
				// 無人觀測：事實仍持續。排班者依 gameHour 朝 work/rest 走一步；無排班者抽樣加鎂／採集／求職／隨機鄰房。不發敘事。
				db.RunUnobservedWorldTick(database, roomGraph, hour, db.UnobservedMaxNPCsPerTick)
			}
			travelSteps := travelerMgr.Tick(database, roomGraph, hour, activeRoomIDs)
			for _, step := range travelSteps {
				oldName := roomGraph.RoomName(step.OldRoom)
				newName := roomGraph.RoomName(step.NewRoom)
				leaveText := "【" + step.NpcName + "】收拾行裝，往" + newName + "方向離去。"
				arriveText := "【" + step.NpcName + "】從" + oldName + "方向走了過來。"
				if target, ok := db.GetScheduleTarget(database, step.EntityID, hour); ok && target.Room == step.NewRoom {
					if target.IsWork {
						arriveText = db.GetShiftFlavor(db.GetNPCTitle(database, step.EntityID), step.EntityID, true)
					} else {
						arriveText = "【" + step.NpcName + "】回到了住處。"
					}
				}
				server.SendNarrateToRoom(sessionStore, database, step.OldRoom, leaveText)
				server.SendNarrateToRoom(sessionStore, database, step.NewRoom, arriveText)
				// 腦驅動到達後行為：敘事＋真實效果（Beg 加鎂、Gather 加物品、SeekJob 撮合）
				if step.ArrivalIntent != "" {
					arrivalNarrative := brainArrivalNarrative(step.NpcName, step.ArrivalIntent)
					if arrivalNarrative != "" {
						server.SendNarrateToRoom(sessionStore, database, step.NewRoom, arrivalNarrative)
					}
					applyBrainArrivalEffects(database, step.EntityID, step.NewRoom, step.ArrivalIntent)
				}
				server.RefreshRoomViews(sessionStore, database, cfg, step.OldRoom)
				server.RefreshRoomViews(sessionStore, database, cfg, step.NewRoom)
			}
		}

		// NPC 閒置動作 & 巡邏：計時器到期後觸發
		idleTickCount++
		if idleTickCount >= nextIdleTrigger {
			idleTickCount = 0
			nextIdleTrigger = 60 + rand.Intn(60)
			period := db.GetTimePeriod(hour)

			playerRooms := server.GetPlayerRoomMap(sessionStore, database)
			schedules, _ := db.GetAllSchedules(database)

			// 觸發－重：閒置 tick 時同房兩 NPC 一來一往（主題劇本＋AI、寫記憶）
			npcDialogueDone := false
			if cfg.OllamaBaseURL != "" && cfg.OllamaModel != "" {
				topicHint := ""
				if t := db.PickRandomNpcNpcTopic(); t != nil {
					topicHint = t.Hint
				}
				for roomID := range playerRooms {
					if tryTriggerNpcNpcInRoom(database, sessionStore, cfg, roomID, topicHint) {
						npcDialogueDone = true
						break
					}
				}
			}
			if !npcDialogueDone {
				// Fallback：微互動（15% 機率），每輪最多一次
				for roomID := range playerRooms {
					social := db.PickMicroInteraction(database, roomID, 15)
					if social != "" {
						server.SendNarrateToRoom(sessionStore, database, roomID, social)
						break
					}
				}
			}

			for _, s := range schedules {
				if !s.IsOnDuty(hour) {
					continue
				}
				npcRoom, _ := db.GetEntityRoom(database, s.EntityID)
				title := db.GetNPCTitle(database, s.EntityID)

				// 巡邏：10% 機率移動到 wander_rooms 中的另一房間
				wanderRooms := db.GetWanderRooms(title)
				if len(wanderRooms) > 1 && rand.Intn(10) == 0 {
					var candidates []string
					for _, wr := range wanderRooms {
						if wr != npcRoom {
							candidates = append(candidates, wr)
						}
					}
					if len(candidates) > 0 {
						dest := candidates[rand.Intn(len(candidates))]
						destName, _ := db.GetRoomName(database, dest)
						srcName, _ := db.GetRoomName(database, npcRoom)
						leaveText := db.GetWanderFlavor(title, s.EntityID, destName, true)
						arriveText := db.GetWanderFlavor(title, s.EntityID, srcName, false)
						server.SendNarrateToRoom(sessionStore, database, npcRoom, leaveText)
						_ = db.SetEntityRoom(database, s.EntityID, dest)
						server.SendNarrateToRoom(sessionStore, database, dest, arriveText)
						server.RefreshRoomViews(sessionStore, database, cfg, npcRoom)
						server.RefreshRoomViews(sessionStore, database, cfg, dest)
						continue
					}
				}

				// 閒置動作：僅對有玩家在場的房間觸發
				if _, hasPlayer := playerRooms[npcRoom]; !hasPlayer {
					continue
				}
				disp := db.GetDisposition(database, s.EntityID)
				emote := db.PickIdleEmote(title, period, s.EntityID, disp)
				if emote != "" {
					server.SendNarrateToRoom(sessionStore, database, npcRoom, emote)
					break
				}
			}
		}
	})

	// 經濟引擎：獨立 goroutine、自有 tick rate，後續可在 onTick 產出事件流／價格／任務報酬（§1.1.6）。
	economy.Run(cfg.EconomyTickInterval, func() {
		// 第一版留空；之後接鎂產消、交易、event.Append 事件流等。
		_ = database
	})

	log.Printf("listening :%s (max ws: %d, tick: %v, economy: %v)", cfg.Port, cfg.MaxWebSocketConn, cfg.TickInterval, cfg.EconomyTickInterval)
	if err := http.ListenAndServe(":"+cfg.Port, nil); err != nil {
		log.Fatalf("listen: %v", err)
	}
}
