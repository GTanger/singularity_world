package main

import (
	"log"
	"net/http"
	"os"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/economy"
	"singularity_world/gametext"
	"singularity_world/server"
	"singularity_world/store"

	"github.com/gorilla/websocket"
)

func main() {
	cfg := config.DefaultServer()

	if err := os.MkdirAll("data", 0755); err != nil {
		log.Fatalf("mkdir data: %v", err)
	}
	if err := os.MkdirAll("data/runtime", 0755); err != nil {
		log.Fatalf("mkdir data/runtime: %v", err)
	}
	if err := os.MkdirAll("data/config", 0755); err != nil {
		log.Fatalf("mkdir data/config: %v", err)
	}
	gametext.MustLoad()
	if err := config.LoadSimulation(""); err != nil {
		log.Printf("config: simulation.json: %v", err)
	}

	if err := store.Init("data/rooms", "data/runtime", "data"); err != nil {
		log.Fatalf("store init: %v", err)
	}
	if err := db.SeedNPCsForStore(); err != nil {
		log.Printf("seed npcs for store: %v", err)
	}
	if fixed, err := db.EnsureAllNPCsHaveSoulSeed(); err != nil {
		log.Printf("EnsureAllNPCsHaveSoulSeed: %v", err)
	} else if fixed > 0 {
		log.Printf("[npc] soul_seed backfilled for %d NPCs", fixed)
	}
	defer func() {
		if store.Default != nil {
			if err := store.Default.FlushEntities(); err != nil {
				log.Printf("flush entities on exit: %v", err)
			}
		}
	}()

	hub := server.NewHub(cfg.MaxWebSocketConn)
	sessionStore := server.NewSessionStore()
	upgrader := &websocket.Upgrader{CheckOrigin: func(r *http.Request) bool { return true }}

	registerHTTPRoutes(cfg, hub, sessionStore, upgrader)
	startSimulationMainLoop(cfg, sessionStore)

	economy.Run(cfg.EconomyTickInterval, func() {
		// 第一版留空；之後接鎂產消、交易、event.Append 事件流等。
	})

	log.Printf("listening :%s (max ws: %d, tick: %v, economy: %v)", cfg.Port, cfg.MaxWebSocketConn, cfg.TickInterval, cfg.EconomyTickInterval)
	if err := http.ListenAndServe(":"+cfg.Port, nil); err != nil {
		log.Fatalf("listen: %v", err)
	}
}
