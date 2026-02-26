// Package config 負責可調參數集中管理，避免硬編碼遊戲數值與伺服器設定。
package config

import (
	"encoding/json"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"
)

// Server 存放伺服器與連線可調參數。
type Server struct {
	Port                  string
	DBPath                string
	MaxWebSocketConn      int
	TickInterval          time.Duration // 遊戲主迴圈 tick 間隔，與前端同步用
	EconomyTickInterval   time.Duration // 經濟引擎獨立 goroutine 的 tick 間隔（§1.1.6）
	ChunkSize             int           // 單區塊邊長（格數）；預設 151（22,801 格），當前區塊常亮、區塊外黑、越界換區
	MapsPath              string        // 區塊地圖 .txt 目錄，預設 data/maps；檔名 {cx}_{cy}.txt
	SessionRetainMinutes  int           // 斷線後同角色／同房間可恢復的觀念時長（分鐘）；實際位置由 DB 持久，重連登入即恢復
	GameTimeEpochUnix     int64         // 遊戲 0:00 對應的真實 Unix 秒；0＝以 1970-01-01 為起點
	GameTimeScale         float64       // 1 真實秒 ＝ GameTimeScale 遊戲秒；24 ＝ 1 真實小時 ＝ 1 遊戲日
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

const gameEpochPath = "data/game_epoch.unix"

// resolveGameTimeEpoch 決定奇點曆起點：環境變數 > 回滾 > 持久檔 > 現在並寫檔。
func resolveGameTimeEpoch() int64 {
	if s := os.Getenv("GAME_TIME_EPOCH_UNIX"); s != "" {
		if n, err := strconv.ParseInt(s, 10, 64); err == nil && n >= 0 {
			return n
		}
	}
	now := time.Now().Unix()
	if os.Getenv("GAME_TIME_EPOCH_ROLLBACK") != "" {
		writeEpochFile(now)
		return now
	}
	if b, err := os.ReadFile(gameEpochPath); err == nil {
		if n, err := strconv.ParseInt(strings.TrimSpace(string(b)), 10, 64); err == nil && n >= 0 {
			return n
		}
	}
	writeEpochFile(now)
	return now
}

func writeEpochFile(epoch int64) {
	dir := filepath.Dir(gameEpochPath)
	_ = os.MkdirAll(dir, 0755)
	_ = os.WriteFile(gameEpochPath, []byte(strconv.FormatInt(epoch, 10)), 0644)
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
	// 奇點曆起點：持久於 data/game_epoch.unix，重啟照算；設 GAME_TIME_EPOCH_ROLLBACK=1 才重設為「現在＝元年」。
	// 若設 GAME_TIME_EPOCH_UNIX 則以該值為準（不讀寫檔案）。
	gameTimeEpoch := resolveGameTimeEpoch()
	return Server{
		Port:                 port,
		DBPath:               "data/world.db",
		MaxWebSocketConn:     10,
		TickInterval:         200 * time.Millisecond,
		EconomyTickInterval:  time.Second,
		ChunkSize:            chunkSize,
		MapsPath:             mapsPath,
		SessionRetainMinutes: 10,
		GameTimeEpochUnix:    gameTimeEpoch,
		GameTimeScale:        24,
	}
}
