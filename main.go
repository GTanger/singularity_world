// 程式入口：啟動 HTTP 伺服器、WebSocket、靜態檔與 DB，對齊第一版可做清單 §1.1。
package main

import (
	"log"
	"net/http"
	"os"
	"path/filepath"
	"strings"

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
				sessionStore.Remove(c.PlayerID)
			}
			hub.Unregister(c)
		}
		go server.ReadLoop(client, onClose, database, cfg, sessionStore, hub)
	})

	http.HandleFunc("/api/design-constants", config.ServeDesignConstants)
	http.HandleFunc("/api/rooms", func(w http.ResponseWriter, r *http.Request) { server.HandleRoomsAPI(database, w, r) })
	http.Handle("/api/rooms/", http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { server.HandleRoomsAPI(database, w, r) }))
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
	// 房間制：無格點移動 tick；視野為當前房間，移動依出口即時完成。
	go game.Loop(cfg.TickInterval, func() {
		game.RunViewSimulation(database, func() []game.Pos { return server.GetObserverPositions(sessionStore, database) }, obs)
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
