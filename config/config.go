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
	Port                 string
	DBPath               string // 已不使用；保留欄位相容，執行期數據源為 store（JSON）
	MaxWebSocketConn     int
	TickInterval         time.Duration // 遊戲主迴圈 tick 間隔，與前端同步用
	EconomyTickInterval  time.Duration // 經濟引擎獨立 goroutine 的 tick 間隔（§1.1.6）
	ChunkSize            int           // 單區塊邊長（格數）；預設 151（22,801 格），當前區塊常亮、區塊外黑、越界換區
	MapsPath             string        // 區塊地圖 .txt 目錄（選用）；預設 data/maps，目錄可不存在則俯視為預設草
	SessionRetainMinutes int           // 斷線後同角色／同房間可恢復的觀念時長（分鐘）；實際位置由 DB 持久，重連登入即恢復
	GameTimeEpochUnix    int64         // 遊戲 0:00 對應的真實 Unix 秒；0＝以 1970-01-01 為起點
	GameTimeScale        float64       // 1 真實秒 ＝ GameTimeScale 遊戲秒；24 ＝ 1 真實小時 ＝ 1 遊戲日
	NPCPoolSize          int           // NPC 池總量（有房間的 NPC 數上限）；0＝不自動補
	NPCSpawnIntervalSec  int           // 每隔幾秒檢查一次並在未滿時生成一名 NPC；0＝不自動生成
	OllamaBaseURL        string        // NPC 對話 LLM（Ollama）位址；空＝不呼叫，走 fallback 模板
	OllamaModel          string        // Ollama 模型 tag（可改，非鎖死）；DefaultServer 預設見程式內；若 BaseURL 非空且 Model 空則 fallback qwen-4b-slim；覆寫用環境變數 OLLAMA_MODEL
	// 10.15 求職撮合
	SeekJobMgThreshold int  // 鎂低於此值才參與撮合；0＝沿用程式常數 50
	JobMatchWhenStable bool // 為 true 時，無職且鎂≥閾值者也有機率參與撮合（安定需求）
	// NPC↔NPC 對話（可選；0 表示用程式內建預設，不必設定）
	NpcNpcQualityMaxRunes           int // 單句品質閘門最大字數（rune）；0＝52
	NpcNpcSocialTickMinWithPlayer   int // 有玩家同房時，隨機閒聊計時器重置下限（tick）；0＝115
	NpcNpcSocialTickExtraWithPlayer int // 加在 min 上的亂數上界（不含）；0＝75 → 實際 min~min+extra-1
	NpcNpcSocialTickMinNoPlayer     int // 無玩家在线時計時器下限；0＝80
	NpcNpcSocialTickExtraNoPlayer   int // 無玩家時亂數上界（不含）；0＝40
	// NPC↔NPC 在線品質分：通過 qualityGate 後再評分，低於門檻不寫入摘要／archival。
	// 0＝預設門檻 35；正數＝自訂門檻；-1＝關閉評分（等同僅用 qualityGate）。環境變數 NPC_DIALOGUE_SCORE_THRESHOLD。
	NpcNpcDialogueScoreThreshold int
}

// Design 為第一版可做清單 1.2.2 設計常數：1 格＝1m＝30px、角色圓 24px、地形字 30px、格線隱藏。供前端對齊。
type Design struct {
	CellSizePx    int `json:"cell_size_px"`    // 地圖格 30 px ＝ 1m×1m
	RoleCirclePx  int `json:"role_circle_px"`  // 角色圓直徑 24 px ≈ 0.8m
	TerrainFontPx int `json:"terrain_font_px"` // 地形字 30 px
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

// DefaultServer 載入 data/config/server_defaults.json（失敗則用內建預設），再套用環境變數覆寫。
func DefaultServer() Server {
	cfg, ok := loadServerDefaultsFromJSON("")
	if !ok {
		cfg = Server{
			Port:                 "8080",
			DBPath:               "data/world.db",
			MaxWebSocketConn:     10,
			TickInterval:         500 * time.Millisecond,
			EconomyTickInterval:  time.Second,
			ChunkSize:            151,
			MapsPath:             "data/maps",
			SessionRetainMinutes: 10,
			GameTimeScale:        24,
			NPCPoolSize:          10,
			NPCSpawnIntervalSec:  120,
			OllamaBaseURL:        "http://127.0.0.1:11434",
			OllamaModel:          "sorc/qwen3.5-claude-4.6-opus:2b",
		}
	}
	cfg.GameTimeEpochUnix = resolveGameTimeEpoch()
	if p := os.Getenv("PORT"); p != "" {
		cfg.Port = p
	}
	if s := os.Getenv("CHUNK_SIZE"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n > 0 {
			cfg.ChunkSize = n
		}
	}
	if p := os.Getenv("MAPS_PATH"); p != "" {
		cfg.MapsPath = p
	}
	if os.Getenv("OLLAMA_DISABLE") == "1" || os.Getenv("OLLAMA_DISABLE") == "true" {
		cfg.OllamaBaseURL = ""
		cfg.OllamaModel = ""
	}
	if s := os.Getenv("NPC_POOL_SIZE"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n >= 0 {
			cfg.NPCPoolSize = n
		}
	}
	if s := os.Getenv("NPC_SPAWN_INTERVAL_SEC"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n >= 0 {
			cfg.NPCSpawnIntervalSec = n
		}
	}
	if s := os.Getenv("OLLAMA_BASE_URL"); s != "" {
		cfg.OllamaBaseURL = strings.TrimSuffix(s, "/")
	}
	if s := os.Getenv("OLLAMA_MODEL"); s != "" {
		cfg.OllamaModel = s
	}
	if cfg.OllamaBaseURL != "" && cfg.OllamaModel == "" {
		cfg.OllamaModel = "qwen-4b-slim"
	}
	if s := os.Getenv("SEEK_JOB_MG_THRESHOLD"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n >= 0 {
			cfg.SeekJobMgThreshold = n
		}
	}
	if os.Getenv("JOB_MATCH_WHEN_STABLE") == "1" || strings.ToLower(os.Getenv("JOB_MATCH_WHEN_STABLE")) == "true" {
		cfg.JobMatchWhenStable = true
	}
	if s := os.Getenv("NPC_NPC_QUALITY_MAX_RUNES"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n > 0 {
			cfg.NpcNpcQualityMaxRunes = n
		}
	}
	if s := os.Getenv("NPC_NPC_SOCIAL_TICK_MIN_WITH_PLAYER"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n > 0 {
			cfg.NpcNpcSocialTickMinWithPlayer = n
		}
	}
	if s := os.Getenv("NPC_NPC_SOCIAL_TICK_EXTRA_WITH_PLAYER"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n > 0 {
			cfg.NpcNpcSocialTickExtraWithPlayer = n
		}
	}
	if s := os.Getenv("NPC_NPC_SOCIAL_TICK_MIN_NO_PLAYER"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n > 0 {
			cfg.NpcNpcSocialTickMinNoPlayer = n
		}
	}
	if s := os.Getenv("NPC_NPC_SOCIAL_TICK_EXTRA_NO_PLAYER"); s != "" {
		if n, err := strconv.Atoi(s); err == nil && n > 0 {
			cfg.NpcNpcSocialTickExtraNoPlayer = n
		}
	}
	if s := os.Getenv("NPC_DIALOGUE_SCORE_THRESHOLD"); s != "" {
		if strings.EqualFold(s, "off") || s == "-1" {
			cfg.NpcNpcDialogueScoreThreshold = -1
		} else if n, err := strconv.Atoi(s); err == nil {
			cfg.NpcNpcDialogueScoreThreshold = n
		}
	}
	return cfg
}
