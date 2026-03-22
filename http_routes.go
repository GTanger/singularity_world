// 主程式：HTTP／WebSocket 路由註冊。
package main

import (
	"log"
	"net/http"
	"net/http/httputil"
	"net/url"
	"path/filepath"
	"strings"

	"singularity_world/config"
	"singularity_world/db"
	"singularity_world/gametext"
	"singularity_world/npcnpc"
	"singularity_world/server"

	"github.com/gorilla/websocket"
)

func registerHTTPRoutes(cfg config.Server, hub *server.Hub, sessionStore *server.SessionStore, upgrader *websocket.Upgrader) {
	http.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		conn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			log.Printf("upgrade: %v", err)
			if strings.Contains(err.Error(), "upgrade") && strings.Contains(err.Error(), "Connection") {
				log.Printf("[ws] behind reverse proxy: ensure Connection: Upgrade and Upgrade: websocket are forwarded")
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
		go server.ReadLoop(client, onClose, cfg, sessionStore, hub)
	})

	http.HandleFunc("/api/design-constants", config.ServeDesignConstants)
	roomsAPI := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { server.HandleRoomsAPI(w, r) })
	http.Handle("/api/rooms/", roomsAPI)
	http.HandleFunc("/api/rooms", roomsAPI.ServeHTTP)
	http.HandleFunc("/api/admin/wipe-entities", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "POST only", http.StatusMethodNotAllowed)
			return
		}
		if err := db.DeleteAllEntities(); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(gametext.AdminWipeResponse()))
	})
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
	http.HandleFunc("/room_editor", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/room_editor" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		w.Header().Set("Cache-Control", "no-cache, max-age=0, must-revalidate")
		http.ServeFile(w, r, filepath.Join("web", "room_editor.html"))
	})
	roomEditorAPI := http.HandlerFunc(server.HandleRoomEditorAPI)
	http.Handle("/api/room-editor/", roomEditorAPI)
	http.HandleFunc("/api/room-editor", roomEditorAPI.ServeHTTP)
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
		server.HandleTopologyAPI(w, r)
	})
	http.HandleFunc("/api/player-room", func(w http.ResponseWriter, r *http.Request) {
		server.HandlePlayerRoomAPI(w, r)
	})
	http.HandleFunc("/api/debug/npc-social", func(w http.ResponseWriter, r *http.Request) {
		npcnpc.HandleSocialDebug(w, r, cfg)
	})
	http.HandleFunc("/api/debug/npc-social/reset", func(w http.ResponseWriter, r *http.Request) {
		npcnpc.HandleSocialDebugReset(w, r)
	})

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
}
