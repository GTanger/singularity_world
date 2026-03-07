// 程式入口：啟動 HTTP 伺服器、WebSocket、靜態檔與 DB，對齊第一版可做清單 §1.1。
package main

import (
	"database/sql"
	"log"
	"math/rand"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/gorilla/websocket"
	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/economy"
	"singularity_world/game"
	"singularity_world/server"
	"singularity_world/store"
)

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

	var database *sql.DB // nil：不再使用 DB 檔，所有讀寫經由 store

	hub := server.NewHub(cfg.MaxWebSocketConn)
	sessionStore := server.NewSessionStore()
	upgrader := websocket.Upgrader{CheckOrigin: func(r *http.Request) bool { return true }}

	http.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		conn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			log.Printf("upgrade: %v", err)
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

	// NPC 活化：閒置動作 & 巡邏計時器（中頻 5-12 真實秒，即 2-5 遊戲分鐘）
	db.LoadBehaviors("data/npc_behaviors.json")
	db.LoadOccupations("data/templates/occupations.json")
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

	// 尋路引擎：建立房間鄰接圖
	roomGraph := db.GetGraph()
	if err := roomGraph.BuildGraph(database); err != nil {
		log.Printf("[pathfind] build graph failed: %v", err)
	}

	// 地圖型 NPC 移動管理器（排班型：經理等依 work_room/rest_room 尋路逐格移動，家可遠在十格外）
	travelerMgr := db.NewTravelerManager()
	schedules, _ := db.GetAllSchedules(database)
	for _, s := range schedules {
		title := db.GetNPCTitle(database, s.EntityID)
		def := db.GetMovementDefForTitle(title)
		def.Type = db.MoveSchedule
		travelerMgr.Register(s.EntityID, def)
	}
	var travelTickCount int
	travelTickInterval := 75 // 每 15 秒推進一步（75 ticks × 200ms）

	go game.Loop(cfg.TickInterval, func() {
		game.RunViewSimulation(database, func() []game.Pos { return server.GetObserverPositions(sessionStore, database) }, obs)

		now := time.Now().Unix()
		_, hour, _, _ := game.GameTimeNow(now, cfg.GameTimeEpochUnix, cfg.GameTimeScale)

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
				}
			}
		}

		// 地圖型 NPC 移動：每 travelTickInterval 推進一步（排班型會逐格走到 work/rest）
		travelTickCount++
		if travelTickCount >= travelTickInterval {
			travelTickCount = 0
			travelSteps := travelerMgr.Tick(database, roomGraph, hour)
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
				server.RefreshRoomViews(sessionStore, database, cfg, step.OldRoom)
				server.RefreshRoomViews(sessionStore, database, cfg, step.NewRoom)
			}
		}

		// NPC 閒置動作 & 巡邏：計時器到期後觸發
		idleTickCount++
		if idleTickCount >= nextIdleTrigger {
			idleTickCount = 0
			nextIdleTrigger = 25 + rand.Intn(35)
			period := db.GetTimePeriod(hour)

			playerRooms := server.GetPlayerRoomMap(sessionStore, database)
			schedules, _ := db.GetAllSchedules(database)

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
				emote := db.PickIdleEmote(title, period, s.EntityID)
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
