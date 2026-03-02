// 程式入口：啟動 HTTP 伺服器、WebSocket、靜態檔與 DB，對齊第一版可做清單 §1.1。
package main

import (
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
)

func main() {
	cfg := config.DefaultServer()

	dir := filepath.Dir(cfg.DBPath)
	if err := os.MkdirAll(dir, 0755); err != nil {
		log.Fatalf("mkdir %s: %v", dir, err)
	}

	database, err := db.OpenDB(cfg.DBPath)
	if err != nil {
		log.Fatalf("open db: %v", err)
	}
	defer database.Close()

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
	var idleTickCount int
	nextIdleTrigger := 25 + rand.Intn(35) // 首次觸發：5-12 秒 ÷ 200ms tick = 25-60 ticks

	go game.Loop(cfg.TickInterval, func() {
		game.RunViewSimulation(database, func() []game.Pos { return server.GetObserverPositions(sessionStore, database) }, obs)

		now := time.Now().Unix()
		_, hour, _, _ := game.GameTimeNow(now, cfg.GameTimeEpochUnix, cfg.GameTimeScale)

		// NPC 排班：每遊戲小時檢查一次，上下班移動 + 換班敘事
		if hour != lastScheduleHour {
			lastScheduleHour = hour
			moves, err := db.ApplySchedules(database, hour)
			if err == nil {
				for _, m := range moves {
					leaveText := db.GetShiftFlavor(m.Title, m.EntityID, false)
					arriveText := db.GetShiftFlavor(m.Title, m.EntityID, true)
					server.SendNarrateToRoom(sessionStore, database, m.OldRoom, leaveText)
					server.SendNarrateToRoom(sessionStore, database, m.NewRoom, arriveText)
				}
				if len(moves) > 0 {
					server.BroadcastRoomViews(sessionStore, database, cfg)
				}
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
						// 刷新來源和目的房間的視野
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
					break // 每輪最多一個 NPC 發閒置動作，避免洗版
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
