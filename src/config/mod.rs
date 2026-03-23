// config 模組 — 伺服器與設計常數，對齊 Go config/config.go + config/server_defaults.go。
// 載入優先順序：data/config/server_defaults.json → 環境變數覆寫 → 內建預設值。

use std::env;
use std::fs;
use std::path::Path;
use serde::Deserialize;

/// 伺服器與連線可調參數。
#[derive(Debug, Clone)]
pub struct Server {
    pub port: String,
    pub max_websocket_conn: i32,
    pub tick_interval_ms: u64,
    pub economy_tick_interval_ms: u64,
    pub chunk_size: i32,
    pub maps_path: String,
    pub session_retain_minutes: i32,
    pub game_time_epoch_unix: i64,
    pub game_time_scale: f64,
    pub npc_pool_size: i32,
    pub npc_spawn_interval_sec: i32,
    pub ollama_base_url: String,
    pub ollama_model: String,
    pub ollama_disable: bool,
    pub player_talk_api_base_url: String,
    pub player_talk_api_model: String,
    pub player_talk_api_key: String,
    pub seek_job_mg_threshold: i32,
    pub job_match_when_stable: bool,
    pub npc_npc_quality_max_runes: i32,
    pub npc_npc_social_tick_min_with_player: i32,
    pub npc_npc_social_tick_extra_with_player: i32,
    pub npc_npc_social_tick_min_no_player: i32,
    pub npc_npc_social_tick_extra_no_player: i32,
    pub npc_npc_dialogue_score_threshold: i32,
}

/// 設計常數：1 格＝1m＝30px、角色圓 24px、地形字 30px。
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct Design {
    #[serde(default)]
    pub cell_size_px: i32,
    #[serde(default)]
    pub role_circle_px: i32,
    #[serde(default)]
    pub terrain_font_px: i32,
}

impl Default for Design {
    fn default() -> Self {
        Self {
            cell_size_px: 30,
            role_circle_px: 24,
            terrain_font_px: 30,
        }
    }
}

// ── JSON 反序列化輔助結構 ──

const DEFAULT_SERVER_JSON_PATH: &str = "data/config/server_defaults.json";
const GAME_EPOCH_PATH: &str = "data/game_epoch.unix";

#[derive(Deserialize, Default)]
struct ServerDefaultsFile {
    #[serde(default)]
    design: Design,
    #[serde(default)]
    server: ServerJson,
}

#[derive(Deserialize, Default)]
struct ServerJson {
    #[serde(default)]
    port: String,
    #[serde(default)]
    max_websocket_conn: i32,
    #[serde(default)]
    tick_interval_ms: u64,
    #[serde(default)]
    economy_tick_interval_ms: u64,
    #[serde(default)]
    chunk_size: i32,
    #[serde(default)]
    maps_path: String,
    #[serde(default)]
    session_retain_minutes: i32,
    #[serde(default)]
    game_time_scale: f64,
    #[serde(default)]
    npc_pool_size: i32,
    #[serde(default)]
    npc_spawn_interval_sec: i32,
    #[serde(default)]
    ollama_base_url: String,
    #[serde(default)]
    ollama_model: String,
    #[serde(default)]
    ollama_disable: bool,
    #[serde(default)]
    player_talk_api_base_url: String,
    #[serde(default)]
    player_talk_api_model: String,
    #[serde(default)]
    seek_job_mg_threshold: i32,
    #[serde(default)]
    job_match_when_stable: bool,
    #[serde(default)]
    npc_npc_quality_max_runes: i32,
    #[serde(default)]
    npc_npc_social_tick_min_with_player: i32,
    #[serde(default)]
    npc_npc_social_tick_extra_with_player: i32,
    #[serde(default)]
    npc_npc_social_tick_min_no_player: i32,
    #[serde(default)]
    npc_npc_social_tick_extra_no_player: i32,
    #[serde(default)]
    npc_npc_dialogue_score_threshold: i32,
}

/// 從 JSON 檔讀取 server_defaults。
fn load_server_defaults_from_json() -> Option<(ServerJson, Design)> {
    let raw = fs::read_to_string(DEFAULT_SERVER_JSON_PATH)
        .or_else(|_| fs::read_to_string(Path::new("..").join(DEFAULT_SERVER_JSON_PATH)))
        .ok()?;
    let root: ServerDefaultsFile = serde_json::from_str(&raw).ok()?;
    Some((root.server, root.design))
}

/// 決定奇點曆起點：環境變數 > 持久檔 > 現在並寫檔。
fn resolve_game_time_epoch() -> i64 {
    if let Ok(s) = env::var("GAME_TIME_EPOCH_UNIX")
        && let Ok(n) = s.parse::<i64>()
        && n >= 0
    {
        return n;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    if env::var("GAME_TIME_EPOCH_ROLLBACK").is_ok() {
        write_epoch_file(now);
        return now;
    }
    if let Ok(content) = fs::read_to_string(GAME_EPOCH_PATH)
        && let Ok(n) = content.trim().parse::<i64>()
        && n >= 0
    {
        return n;
    }
    write_epoch_file(now);
    now
}

fn write_epoch_file(epoch: i64) {
    if let Some(dir) = Path::new(GAME_EPOCH_PATH).parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(GAME_EPOCH_PATH, epoch.to_string());
}

fn env_i32(key: &str) -> Option<i32> {
    env::var(key).ok()?.parse().ok()
}

fn env_bool(key: &str) -> bool {
    matches!(env::var(key).ok().as_deref(), Some("1") | Some("true") | Some("TRUE") | Some("True"))
}

impl Server {
    /// 載入預設值（JSON + 環境變數覆寫），對齊 Go DefaultServer()。
    pub fn load_default() -> Self {
        let (sj, _design) = load_server_defaults_from_json()
            .unwrap_or_default();

        let mut cfg = Self {
            port: if sj.port.is_empty() { "8080".into() } else { sj.port },
            max_websocket_conn: if sj.max_websocket_conn > 0 { sj.max_websocket_conn } else { 10 },
            tick_interval_ms: if sj.tick_interval_ms > 0 { sj.tick_interval_ms } else { 500 },
            economy_tick_interval_ms: if sj.economy_tick_interval_ms > 0 { sj.economy_tick_interval_ms } else { 1000 },
            chunk_size: if sj.chunk_size > 0 { sj.chunk_size } else { 151 },
            maps_path: if sj.maps_path.is_empty() { "data/maps".into() } else { sj.maps_path },
            session_retain_minutes: if sj.session_retain_minutes > 0 { sj.session_retain_minutes } else { 10 },
            game_time_epoch_unix: resolve_game_time_epoch(),
            game_time_scale: if sj.game_time_scale > 0.0 { sj.game_time_scale } else { 24.0 },
            npc_pool_size: sj.npc_pool_size,
            npc_spawn_interval_sec: sj.npc_spawn_interval_sec,
            ollama_base_url: if sj.ollama_base_url.is_empty() { "http://127.0.0.1:11434".into() } else { sj.ollama_base_url },
            ollama_model: sj.ollama_model,
            ollama_disable: sj.ollama_disable,
            player_talk_api_base_url: sj.player_talk_api_base_url,
            player_talk_api_model: sj.player_talk_api_model,
            player_talk_api_key: String::new(),
            seek_job_mg_threshold: sj.seek_job_mg_threshold,
            job_match_when_stable: sj.job_match_when_stable,
            npc_npc_quality_max_runes: sj.npc_npc_quality_max_runes,
            npc_npc_social_tick_min_with_player: sj.npc_npc_social_tick_min_with_player,
            npc_npc_social_tick_extra_with_player: sj.npc_npc_social_tick_extra_with_player,
            npc_npc_social_tick_min_no_player: sj.npc_npc_social_tick_min_no_player,
            npc_npc_social_tick_extra_no_player: sj.npc_npc_social_tick_extra_no_player,
            npc_npc_dialogue_score_threshold: sj.npc_npc_dialogue_score_threshold,
        };

        // 環境變數覆寫
        if let Ok(p) = env::var("PORT") {
            cfg.port = p;
        }
        if let Some(n) = env_i32("CHUNK_SIZE")
            && n > 0
        {
            cfg.chunk_size = n;
        }
        if let Ok(p) = env::var("MAPS_PATH") {
            cfg.maps_path = p;
        }
        if let Some(n) = env_i32("NPC_POOL_SIZE")
            && n >= 0
        {
            cfg.npc_pool_size = n;
        }
        if let Some(n) = env_i32("NPC_SPAWN_INTERVAL_SEC")
            && n >= 0
        {
            cfg.npc_spawn_interval_sec = n;
        }
        if let Ok(s) = env::var("OLLAMA_BASE_URL") {
            cfg.ollama_base_url = s.trim_end_matches('/').to_string();
        }
        if let Ok(s) = env::var("OLLAMA_MODEL") {
            cfg.ollama_model = s;
        }
        if cfg.ollama_base_url.is_empty() && cfg.ollama_model.is_empty() {
            // 保持空
        } else if !cfg.ollama_base_url.is_empty() && cfg.ollama_model.is_empty() {
            cfg.ollama_model = "qwen-4b-slim".into();
        }
        if env_bool("OLLAMA_DISABLE") {
            cfg.ollama_disable = true;
        }
        if cfg.ollama_disable {
            cfg.ollama_base_url.clear();
            cfg.ollama_model.clear();
        }
        if let Ok(s) = env::var("PLAYER_TALK_API_BASE_URL") {
            cfg.player_talk_api_base_url = s.trim().trim_end_matches('/').to_string();
        }
        if let Ok(s) = env::var("PLAYER_TALK_API_MODEL") {
            cfg.player_talk_api_model = s.trim().to_string();
        }
        if let Ok(k) = env::var("PLAYER_TALK_API_KEY") {
            cfg.player_talk_api_key = k.trim().to_string();
        } else if let Ok(k) = env::var("OPENAI_API_KEY") {
            cfg.player_talk_api_key = k.trim().to_string();
        }
        if let Some(n) = env_i32("SEEK_JOB_MG_THRESHOLD")
            && n >= 0
        {
            cfg.seek_job_mg_threshold = n;
        }
        if env_bool("JOB_MATCH_WHEN_STABLE") {
            cfg.job_match_when_stable = true;
        }
        if let Some(n) = env_i32("NPC_NPC_QUALITY_MAX_RUNES")
            && n > 0
        {
            cfg.npc_npc_quality_max_runes = n;
        }
        if let Some(n) = env_i32("NPC_NPC_SOCIAL_TICK_MIN_WITH_PLAYER")
            && n > 0
        {
            cfg.npc_npc_social_tick_min_with_player = n;
        }
        if let Some(n) = env_i32("NPC_NPC_SOCIAL_TICK_EXTRA_WITH_PLAYER")
            && n > 0
        {
            cfg.npc_npc_social_tick_extra_with_player = n;
        }
        if let Some(n) = env_i32("NPC_NPC_SOCIAL_TICK_MIN_NO_PLAYER")
            && n > 0
        {
            cfg.npc_npc_social_tick_min_no_player = n;
        }
        if let Some(n) = env_i32("NPC_NPC_SOCIAL_TICK_EXTRA_NO_PLAYER")
            && n > 0
        {
            cfg.npc_npc_social_tick_extra_no_player = n;
        }
        if let Ok(s) = env::var("NPC_DIALOGUE_SCORE_THRESHOLD") {
            if s.eq_ignore_ascii_case("off") || s == "-1" {
                cfg.npc_npc_dialogue_score_threshold = -1;
            } else if let Ok(n) = s.parse::<i32>() {
                cfg.npc_npc_dialogue_score_threshold = n;
            }
        }

        cfg
    }

    /// 玩家 Talk 是否走 OpenAI 相容 Web API。
    pub fn player_talk_uses_web_api(&self) -> bool {
        !self.player_talk_api_base_url.trim().is_empty()
            && !self.player_talk_api_model.trim().is_empty()
    }
}

/// 取得設計常數（優先 JSON 檔，fallback 內建預設）。
pub fn design_constants() -> Design {
    load_server_defaults_from_json()
        .and_then(|(_, d)| {
            if d.cell_size_px > 0 { Some(d) } else { None }
        })
        .unwrap_or_default()
}
