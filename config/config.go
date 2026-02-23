// Package config 負責可調參數集中管理，避免硬編碼遊戲數值與伺服器設定。
package config

import (
	"encoding/json"
	"net/http"
	"os"
	"strconv"
	"time"
)

// Server 存放伺服器與連線可調參數。
type Server struct {
	Port                string
	DBPath              string
	MaxWebSocketConn    int
	TickInterval        time.Duration // 遊戲主迴圈 tick 間隔，與前端同步用
	EconomyTickInterval time.Duration // 經濟引擎獨立 goroutine 的 tick 間隔（§1.1.6）
	ChunkSize           int           // 單區塊邊長（格數）；預設 151（22,801 格），當前區塊常亮、區塊外黑、越界換區
	MapsPath            string        // 區塊地圖 .txt 目錄，預設 data/maps；檔名 {cx}_{cy}.txt
}

// Design 為第一版可做清單 1.2.2 設計常數：1 格＝1m＝30px、角色圓 24px、地形字 30px、格線隱藏。供前端對齊。
type Design struct {
	CellSizePx    int `json:"cell_size_px"`    // 地圖格 30 px ＝ 1m×1m
	RoleCirclePx  int `json:"role_circle_px"`  // 角色圓直徑 24 px ≈ 0.8m
	TerrainFontPx int `json:"terrain_font_px"` // 地形字 30 px
}

// DesignConstants 回傳第一版設計常數（區塊、視野、尺度等）。
func DesignConstants() Design {
	return Design{CellSizePx: 30, RoleCirclePx: 24, TerrainFontPx: 30}
}

// ServeDesignConstants 處理 GET /api/design-constants，回傳 JSON。供前端讀取常數、避免硬編碼。
func ServeDesignConstants(w http.ResponseWriter, _ *http.Request) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	_ = json.NewEncoder(w).Encode(DesignConstants())
}

// DefaultServer 回傳第一版預設值；若環境變數 PORT 已設則使用該埠（例：Cloudflare Tunnel 用 PORT=1721）。
func DefaultServer() Server {
	port := "8080"
	if p := os.Getenv("PORT"); p != "" {
		port = p
	}
	chunkSize := 151
	if s := os.Getenv("CHUNK_SIZE"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n > 0 {
			chunkSize = n
		}
	}
	mapsPath := "data/maps"
	if p := os.Getenv("MAPS_PATH"); p != "" {
		mapsPath = p
	}
	return Server{
		Port:                port,
		DBPath:              "data/world.db",
		MaxWebSocketConn:    10,
		TickInterval:        200 * time.Millisecond,
		EconomyTickInterval: time.Second,
		ChunkSize:           chunkSize,
		MapsPath:            mapsPath,
	}
}
