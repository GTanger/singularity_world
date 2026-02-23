// Package server 負責 WebSocket 連線管理與玩家 session。本檔為 Hub 與連線上限邏輯。
package server

import (
	"database/sql"
	"sync"

	"github.com/gorilla/websocket"
	"singularity_world/config"
)

// Client 代表單一 WebSocket 連線，帶 Send channel 供伺服器推送；PlayerID 由 session 在登入後填入。
type Client struct {
	Conn     *websocket.Conn
	Send     chan []byte
	PlayerID string
}

// Hub 集中管理所有 WebSocket 連線並強制上限（決策 004：最多 10 人）。
type Hub struct {
	mu      sync.Mutex
	clients map[*Client]struct{}
	MaxConn int
}

// NewHub 建立 Hub，maxConn 為允許的最大連線數。回傳 *Hub，無副作用。
func NewHub(maxConn int) *Hub {
	return &Hub{
		clients: make(map[*Client]struct{}),
		MaxConn: maxConn,
	}
}

// Register 將 c 註冊到 Hub；若已達上限回傳 false，呼叫方應關閉連線。成功時會啟動寫入 goroutine。
func (h *Hub) Register(c *Client) bool {
	h.mu.Lock()
	defer h.mu.Unlock()
	if len(h.clients) >= h.MaxConn {
		return false
	}
	h.clients[c] = struct{}{}
	go h.writeLoop(c)
	return true
}

// Unregister 從 Hub 移除 c 並關閉 c.Send。
func (h *Hub) Unregister(c *Client) {
	h.mu.Lock()
	defer h.mu.Unlock()
	if _, ok := h.clients[c]; ok {
		delete(h.clients, c)
		close(c.Send)
	}
}

func (h *Hub) writeLoop(c *Client) {
	for msg := range c.Send {
		if err := c.Conn.WriteMessage(websocket.TextMessage, msg); err != nil {
			break
		}
	}
}

// SendBufferSize 為每個 client 的 Send 緩衝區大小。
const SendBufferSize = 256

// NewClient 建立 Client，conn 為已升級的 WebSocket。回傳 *Client，無副作用。
func NewClient(conn *websocket.Conn) *Client {
	return &Client{
		Conn: conn,
		Send: make(chan []byte, SendBufferSize),
	}
}

// ReadLoop 從 c.Conn 讀取 JSON 訊息並交由 HandleMessage 處理，結束時呼叫 onClose(c)。
func ReadLoop(c *Client, onClose func(*Client), database *sql.DB, cfg config.Server, store *SessionStore, hub *Hub) {
	defer onClose(c)
	for {
		_, data, err := c.Conn.ReadMessage()
		if err != nil {
			return
		}
		if len(data) > 0 {
			HandleMessage(c, data, database, cfg, store, hub)
		}
	}
}

// Broadcast 將 msg 廣播給所有已註冊連線；Send 滿的 client 會被跳過。
func (h *Hub) Broadcast(msg []byte) {
	h.mu.Lock()
	defer h.mu.Unlock()
	for c := range h.clients {
		select {
		case c.Send <- msg:
		default:
		}
	}
}
