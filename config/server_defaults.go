package config

import (
	"encoding/json"
	"os"
	"path/filepath"
	"time"
)

const defaultServerJSONPath = "data/config/server_defaults.json"

type serverDefaultsFile struct {
	Design struct {
		CellSizePx    int `json:"cell_size_px"`
		RoleCirclePx  int `json:"role_circle_px"`
		TerrainFontPx int `json:"terrain_font_px"`
	} `json:"design"`
	Server struct {
		Port                            string  `json:"port"`
		DBPath                          string  `json:"db_path"`
		MaxWebSocketConn                int     `json:"max_websocket_conn"`
		TickIntervalMS                  int     `json:"tick_interval_ms"`
		EconomyTickIntervalMS           int     `json:"economy_tick_interval_ms"`
		ChunkSize                       int     `json:"chunk_size"`
		MapsPath                        string  `json:"maps_path"`
		SessionRetainMinutes            int     `json:"session_retain_minutes"`
		GameTimeScale                   float64 `json:"game_time_scale"`
		NPCPoolSize                     int     `json:"npc_pool_size"`
		NPCSpawnIntervalSec             int     `json:"npc_spawn_interval_sec"`
		OllamaBaseURL                   string  `json:"ollama_base_url"`
		OllamaModel                     string  `json:"ollama_model"`
		OllamaDisable                   bool    `json:"ollama_disable"` // true＝不呼叫 Ollama；玩家 Talk 仍可走 player_talk_api_* 雲端
		PlayerTalkAPIBaseURL            string  `json:"player_talk_api_base_url"`
		PlayerTalkAPIModel              string  `json:"player_talk_api_model"`
		SeekJobMgThreshold              int     `json:"seek_job_mg_threshold"`
		JobMatchWhenStable              bool    `json:"job_match_when_stable"`
		NpcNpcQualityMaxRunes           int     `json:"npc_npc_quality_max_runes"`
		NpcNpcSocialTickMinWithPlayer   int     `json:"npc_npc_social_tick_min_with_player"`
		NpcNpcSocialTickExtraWithPlayer int     `json:"npc_npc_social_tick_extra_with_player"`
		NpcNpcSocialTickMinNoPlayer     int     `json:"npc_npc_social_tick_min_no_player"`
		NpcNpcSocialTickExtraNoPlayer   int     `json:"npc_npc_social_tick_extra_no_player"`
		NpcNpcDialogueScoreThreshold    int     `json:"npc_npc_dialogue_score_threshold"`
	} `json:"server"`
}

var (
	fileDesign   Design
	designLoaded bool
)

// DesignConstants returns UI scale constants; prefers data/config/server_defaults.json design section.
func DesignConstants() Design {
	if designLoaded && fileDesign.CellSizePx > 0 {
		return fileDesign
	}
	return Design{CellSizePx: 30, RoleCirclePx: 24, TerrainFontPx: 30}
}

func loadServerDefaultsFromJSON(path string) (Server, bool) {
	if path == "" {
		path = defaultServerJSONPath
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		if raw2, err2 := os.ReadFile(filepath.Join("..", path)); err2 == nil {
			raw, err = raw2, nil
		}
	}
	if err != nil {
		return Server{}, false
	}
	var root serverDefaultsFile
	if err := json.Unmarshal(raw, &root); err != nil {
		return Server{}, false
	}
	if root.Design.CellSizePx > 0 {
		fileDesign = Design{
			CellSizePx:    root.Design.CellSizePx,
			RoleCirclePx:  root.Design.RoleCirclePx,
			TerrainFontPx: root.Design.TerrainFontPx,
		}
		designLoaded = true
	}
	s := root.Server
	cfg := Server{
		Port:                            s.Port,
		DBPath:                          s.DBPath,
		MaxWebSocketConn:                s.MaxWebSocketConn,
		TickInterval:                    time.Duration(s.TickIntervalMS) * time.Millisecond,
		EconomyTickInterval:             time.Duration(s.EconomyTickIntervalMS) * time.Millisecond,
		ChunkSize:                       s.ChunkSize,
		MapsPath:                        s.MapsPath,
		SessionRetainMinutes:            s.SessionRetainMinutes,
		GameTimeScale:                   s.GameTimeScale,
		NPCPoolSize:                     s.NPCPoolSize,
		NPCSpawnIntervalSec:             s.NPCSpawnIntervalSec,
		OllamaBaseURL:                   s.OllamaBaseURL,
		OllamaModel:                     s.OllamaModel,
		OllamaDisable:                   s.OllamaDisable,
		PlayerTalkAPIBaseURL:            s.PlayerTalkAPIBaseURL,
		PlayerTalkAPIModel:              s.PlayerTalkAPIModel,
		SeekJobMgThreshold:              s.SeekJobMgThreshold,
		JobMatchWhenStable:              s.JobMatchWhenStable,
		NpcNpcQualityMaxRunes:           s.NpcNpcQualityMaxRunes,
		NpcNpcSocialTickMinWithPlayer:   s.NpcNpcSocialTickMinWithPlayer,
		NpcNpcSocialTickExtraWithPlayer: s.NpcNpcSocialTickExtraWithPlayer,
		NpcNpcSocialTickMinNoPlayer:     s.NpcNpcSocialTickMinNoPlayer,
		NpcNpcSocialTickExtraNoPlayer:   s.NpcNpcSocialTickExtraNoPlayer,
		NpcNpcDialogueScoreThreshold:    s.NpcNpcDialogueScoreThreshold,
	}
	if cfg.Port == "" {
		cfg.Port = "8080"
	}
	if cfg.DBPath == "" {
		cfg.DBPath = "data/world.db"
	}
	if cfg.MaxWebSocketConn <= 0 {
		cfg.MaxWebSocketConn = 10
	}
	if cfg.TickInterval <= 0 {
		cfg.TickInterval = 500 * time.Millisecond
	}
	if cfg.EconomyTickInterval <= 0 {
		cfg.EconomyTickInterval = time.Second
	}
	if cfg.ChunkSize <= 0 {
		cfg.ChunkSize = 151
	}
	if cfg.MapsPath == "" {
		cfg.MapsPath = "data/maps"
	}
	if cfg.SessionRetainMinutes <= 0 {
		cfg.SessionRetainMinutes = 10
	}
	if cfg.GameTimeScale <= 0 {
		cfg.GameTimeScale = 24
	}
	return cfg, true
}
