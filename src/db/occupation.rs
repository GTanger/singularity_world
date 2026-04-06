// 職業插座（對齊既有 `db/occupation`）。

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;

use super::assignment;

#[derive(Debug, Deserialize)]
struct OccupationsFile {
    #[serde(default)]
    occupations: std::collections::HashMap<String, OccupationDef>,
}

#[derive(Debug, Clone, Deserialize)]
struct OccupationDef {
    #[serde(default)]
    action_sockets: Vec<String>,
    #[serde(default)]
    dialogue_file: String,
}

/// 快取後的職業資訊。
#[derive(Debug, Clone)]
pub(crate) struct OccCached {
    pub action_sockets: Vec<String>,
    pub dialogue_file: String,
}

static OCC_CACHE: OnceLock<std::collections::HashMap<String, OccCached>> = OnceLock::new();

pub(crate) fn load_occupation_cache() -> &'static std::collections::HashMap<String, OccCached> {
    OCC_CACHE.get_or_init(|| {
        let path = Path::new("data/templates/occupations.json");
        let raw = match fs::read_to_string(path).or_else(|_| fs::read_to_string(Path::new("..").join(path))) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("occupations.json 讀取失敗: {e}");
                return std::collections::HashMap::new();
            }
        };
        let parsed: OccupationsFile = match serde_json::from_str(&raw) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("occupations.json 解析失敗: {e}");
                return std::collections::HashMap::new();
            }
        };
        parsed
            .occupations
            .into_iter()
            .map(|(k, v)| (k, OccCached { action_sockets: v.action_sockets, dialogue_file: v.dialogue_file }))
            .collect()
    })
}

fn occupation_action_sockets(occupation_id: &str) -> Vec<String> {
    let c = load_occupation_cache();
    c.get(occupation_id).map(|o| o.action_sockets.clone()).unwrap_or_default()
}

const DEFAULT_SOCKETS: &[&str] = &["Talk", "Borrow", "Subdue", "Slay", "Look", "Trade"];

/// 該動詞是否為預設 Agent 插座（不需在工作場所即可對 NPC 執行），對齊既有 `IsDefaultSocket`。
#[must_use]
pub fn is_default_socket(action: &str) -> bool {
    DEFAULT_SOCKETS.contains(&action)
}

/// NPC 在指定房間的有效插座（對齊 `GetSocketsForNPC`）。
#[must_use]
pub fn get_sockets_for_npc(entity_id: &str, room_id: &str) -> Vec<String> {
    let mut sockets: Vec<String> = DEFAULT_SOCKETS.iter().map(|s| (*s).to_string()).collect();
    let Ok(assignments) = assignment::get_assignments_for_entity(entity_id) else {
        return sockets;
    };
    let mut seen: HashSet<String> = sockets.iter().cloned().collect();
    for a in assignments {
        let Ok(in_venue) = assignment::is_room_in_venue(room_id, &a.venue_id) else {
            continue;
        };
        if !in_venue {
            continue;
        }
        for s in occupation_action_sockets(&a.occupation_id) {
            if seen.insert(s.clone()) {
                sockets.push(s);
            }
        }
    }
    sockets
}
