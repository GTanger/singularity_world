//! 房間鄰接圖與 BFS 尋路（對齊既有 `db/pathfind`）。

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, OnceLock, RwLock};

use crate::store;

use super::ErrNoStore;

/// 房間圖快取（讀多寫少；`rebuild_room_graph` 於 store 就緒後更新）。
#[derive(Debug, Clone, Default)]
pub struct RoomGraph {
    adj: HashMap<String, Vec<String>>,
    tags: HashMap<String, Vec<String>>,
    #[allow(dead_code)]
    zone: HashMap<String, String>,
    name: HashMap<String, String>,
}

impl RoomGraph {
    /// 由 store 快照建圖（對齊既有 `BuildGraphFromStore`）。
    pub fn rebuild_from_store(s: &store::Store) -> Self {
        Self {
            adj: s.adjacency(),
            name: s.name_map(),
            zone: s.zone_map(),
            tags: s.room_tags_map(),
        }
    }

    /// BFS 最短路徑：不含起點、含終點之房間 id 序列；不可達回 `None`。`from == to` 回 `None`（與既有行為一致）。
    #[must_use]
    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        if from == to {
            return None;
        }
        let mut prev: HashMap<String, String> = HashMap::new();
        prev.insert(from.to_string(), String::new());
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(from.to_string());

        while let Some(cur) = queue.pop_front() {
            for nb in self.adj.get(&cur).into_iter().flatten() {
                if prev.contains_key(nb) {
                    continue;
                }
                prev.insert(nb.clone(), cur.clone());
                if nb == to {
                    return Some(reconstruct_path(&prev, from, to));
                }
                queue.push_back(nb.clone());
            }
        }
        None
    }

    /// 快取的房間顯示名。
    #[must_use]
    pub fn room_name(&self, room_id: &str) -> String {
        self.name.get(room_id).cloned().unwrap_or_default()
    }

    /// 鄰接房間 id（複本）。
    #[must_use]
    pub fn neighbors(&self, room_id: &str) -> Vec<String> {
        self.adj.get(room_id).cloned().unwrap_or_default()
    }

    /// 房間標籤（複本）；無標籤回空 vec（對齊既有 `RoomTags`）。
    #[must_use]
    pub fn room_tags(&self, room_id: &str) -> Vec<String> {
        self.tags.get(room_id).cloned().unwrap_or_default()
    }

    /// BFS 找最近帶指定 tag 的房間；`max_dist == 0` 不限深度。不可達回 `None`（對齊既有 `FindNearestByTag`）。
    #[must_use]
    pub fn find_nearest_by_tag(&self, origin: &str, tag: &str, max_dist: i32) -> Option<(String, usize)> {
        #[derive(Clone)]
        struct BfsNode {
            id: String,
            dist: usize,
        }
        let mut visited: HashMap<String, bool> = HashMap::new();
        visited.insert(origin.to_string(), true);
        let mut queue: VecDeque<BfsNode> = VecDeque::new();
        queue.push_back(BfsNode {
            id: origin.to_string(),
            dist: 0,
        });
        while let Some(cur) = queue.pop_front() {
            for nb in self.adj.get(&cur.id).into_iter().flatten() {
                if visited.contains_key(nb) {
                    continue;
                }
                visited.insert(nb.clone(), true);
                let nd = cur.dist + 1;
                if max_dist > 0 && nd > max_dist as usize {
                    continue;
                }
                if self
                    .tags
                    .get(nb)
                    .into_iter()
                    .flatten()
                    .any(|t| t == tag)
                {
                    return Some((nb.clone(), nd));
                }
                queue.push_back(BfsNode {
                    id: nb.clone(),
                    dist: nd,
                });
            }
        }
        None
    }

    /// `origin` 周圍 `max_dist` 步內、帶任一指定 tag 的房間（對齊既有 `FindRoomsWithinDist`）。
    #[must_use]
    pub fn find_rooms_within_dist(&self, origin: &str, want_tags: &[&str], max_dist: i32) -> Vec<String> {
        if want_tags.is_empty() || max_dist <= 0 {
            return Vec::new();
        }
        let mut tag_set: HashSet<&str> = HashSet::new();
        for t in want_tags {
            tag_set.insert(*t);
        }
        #[derive(Clone)]
        struct BfsNode {
            id: String,
            dist: usize,
        }
        let mut visited: HashMap<String, bool> = HashMap::new();
        visited.insert(origin.to_string(), true);
        let mut queue: VecDeque<BfsNode> = VecDeque::new();
        let mut result: Vec<String> = Vec::new();
        queue.push_back(BfsNode {
            id: origin.to_string(),
            dist: 0,
        });
        while let Some(cur) = queue.pop_front() {
            for nb in self.adj.get(&cur.id).into_iter().flatten() {
                if visited.contains_key(nb) {
                    continue;
                }
                visited.insert(nb.clone(), true);
                let nd = cur.dist + 1;
                if nd > max_dist as usize {
                    continue;
                }
                if self.tags.get(nb).into_iter().flatten().any(|t| tag_set.contains(t.as_str())) {
                    result.push(nb.clone());
                }
                queue.push_back(BfsNode {
                    id: nb.clone(),
                    dist: nd,
                });
            }
        }
        result
    }
}

fn reconstruct_path(prev: &HashMap<String, String>, from: &str, to: &str) -> Vec<String> {
    let mut path: Vec<String> = Vec::new();
    let mut cur = to.to_string();
    while cur != from {
        path.push(cur.clone());
        let Some(p) = prev.get(&cur) else {
            break;
        };
        if p.is_empty() {
            break;
        }
        cur.clone_from(p);
    }
    path.reverse();
    path
}

static GRAPH_CELL: OnceLock<Arc<RwLock<RoomGraph>>> = OnceLock::new();

fn graph_cell() -> Arc<RwLock<RoomGraph>> {
    GRAPH_CELL
        .get_or_init(|| Arc::new(RwLock::new(RoomGraph::default())))
        .clone()
}

/// 以記憶體中 `Store` 快照更新全域房間圖（不讀寫 store）。
/// 呼叫端若已持有 `Store` 內層寫鎖，必須用此函式；不可呼叫 [`rebuild_room_graph`]（會再對同一 `RwLock` 取讀鎖而死鎖）。
pub fn sync_room_graph_with_store(s: &store::Store) {
    let g = RoomGraph::rebuild_from_store(s);
    let n_rooms = g.name.len();
    let n_edges: usize = g.adj.values().map(Vec::len).sum();
    let cell = graph_cell();
    let mut w = cell.write().unwrap_or_else(|e| e.into_inner());
    *w = g;
    tracing::info!(target: "pathfind", "graph synced from store: {n_rooms} rooms, {n_edges} directed edges");
}

/// 自目前全域 store 讀鎖後重建房間圖（啟動時、或無內層寫鎖時呼叫）。
pub fn rebuild_room_graph() -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    sync_room_graph_with_store(&s);
    Ok(())
}

/// 讀鎖執行閉包（供撮合／尋路）。
pub fn with_room_graph<T>(f: impl FnOnce(&RoomGraph) -> T) -> T {
    let g = graph_cell();
    let r = g.read().unwrap_or_else(|e| e.into_inner());
    f(&r)
}
