//! NPC 行為敘事模板載入與隨機選句。
//!
//! 從 `data/config/npc_narrative.json` 讀取五類行為模板，
//! 支援以 `npc_id + tick` 為種子的確定性隨機選句（相同參數必得相同結果）。
//!
//! 模板中的 `{name}` 佔位符會被替換為 NPC 顯示名稱。

use std::fs;
use std::path::Path;
use std::sync::RwLock;

use serde::Deserialize;

const DEFAULT_PATH: &str = "data/config/npc_narrative.json";

// ── JSON 結構 ──────────────────────────────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
struct NarrativeJson {
    #[allow(dead_code)]
    version: String,
    behaviors: BehaviorsJson,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct BehaviorsJson {
    #[serde(default)]
    gather: Vec<String>,
    #[serde(default)]
    wander: Vec<String>,
    #[serde(default)]
    idle: Vec<String>,
    #[serde(default)]
    arrive: Vec<String>,
    #[serde(default)]
    leave: Vec<String>,
}

// ── 全域快取 ───────────────────────────────────────────────

static STORE: RwLock<Option<NarrativeJson>> = RwLock::new(None);

/// 載入敘事模板 JSON，可重複呼叫（冪等）。
/// 找不到檔案時靜默略過（不 panic），使用空模板。
pub fn load() {
    let raw = fs::read_to_string(DEFAULT_PATH)
        .or_else(|_| fs::read_to_string(Path::new("..").join(DEFAULT_PATH)));
    let data = match raw {
        Ok(s) => match serde_json::from_str::<NarrativeJson>(&s) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("npc_narrative: parse error: {e}");
                NarrativeJson::default()
            }
        },
        Err(_) => NarrativeJson::default(),
    };
    let mut guard = STORE.write().unwrap();
    *guard = Some(data);
}

// ── 選句邏輯 ───────────────────────────────────────────────

/// 用 npc_id + tick 計算穩定索引，從列表選一句。
/// 同一個 NPC 在同一 tick 永遠得到相同的句子。
fn pick<'a>(templates: &'a [String], npc_id: &str, tick: u64) -> Option<&'a str> {
    if templates.is_empty() {
        return None;
    }
    // 以 npc_id 的字元碼和 + tick 做雜湊種子
    let id_hash: u64 = npc_id
        .bytes()
        .enumerate()
        .fold(0u64, |acc, (i, b)| acc.wrapping_add((b as u64).wrapping_mul(i as u64 + 31)));
    let idx = ((id_hash ^ tick.wrapping_mul(2654435761)) as usize) % templates.len();
    Some(&templates[idx])
}

/// 填入 `{name}` 佔位符。
fn fill(template: &str, name: &str) -> String {
    template.replace("{name}", name)
}

fn with_behaviors<F: FnOnce(&BehaviorsJson) -> Option<String>>(f: F) -> Option<String> {
    let guard = STORE.read().unwrap();
    guard.as_ref().and_then(|d| f(&d.behaviors))
}

// ── 公開 API ───────────────────────────────────────────────

/// 採集行為敘事句。
pub fn gather(npc_id: &str, name: &str, tick: u64) -> String {
    with_behaviors(|b| pick(&b.gather, npc_id, tick).map(|t| fill(t, name)))
        .unwrap_or_default()
}

/// 漫遊行為敘事句。
pub fn wander(npc_id: &str, name: &str, tick: u64) -> String {
    with_behaviors(|b| pick(&b.wander, npc_id, tick).map(|t| fill(t, name)))
        .unwrap_or_default()
}

/// 發呆行為敘事句。
pub fn idle(npc_id: &str, name: &str, tick: u64) -> String {
    with_behaviors(|b| pick(&b.idle, npc_id, tick).map(|t| fill(t, name)))
        .unwrap_or_default()
}

/// 抵達行為敘事句。
pub fn arrive(npc_id: &str, name: &str, tick: u64) -> String {
    with_behaviors(|b| pick(&b.arrive, npc_id, tick).map(|t| fill(t, name)))
        .unwrap_or_default()
}

/// 離開行為敘事句。
pub fn leave(npc_id: &str, name: &str, tick: u64) -> String {
    with_behaviors(|b| pick(&b.leave, npc_id, tick).map(|t| fill(t, name)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inject_test_data() {
        let data = NarrativeJson {
            version: "test".into(),
            behaviors: BehaviorsJson {
                gather: vec!["A {name} 採集".into(), "B {name} 伐木".into()],
                wander: vec!["{name} 漫遊".into()],
                idle: vec!["{name} 發呆".into()],
                arrive: vec!["{name} 抵達".into()],
                leave: vec!["{name} 離開".into()],
            },
        };
        let mut guard = STORE.write().unwrap();
        *guard = Some(data);
    }

    #[test]
    fn name_substitution() {
        inject_test_data();
        let s = wander("npc_1", "小明", 0);
        assert_eq!(s, "小明 漫遊");
    }

    #[test]
    fn deterministic_for_same_params() {
        inject_test_data();
        let a = gather("npc_42", "老王", 100);
        let b = gather("npc_42", "老王", 100);
        assert_eq!(a, b);
    }

    #[test]
    fn empty_templates_returns_empty() {
        // 注入空列表（不清空 STORE，避免並行測試競態）
        let data = NarrativeJson {
            version: "test".into(),
            behaviors: BehaviorsJson {
                gather: vec![],
                wander: vec![],
                idle: vec![],
                arrive: vec![],
                leave: vec![],
            },
        };
        {
            let mut guard = STORE.write().unwrap();
            *guard = Some(data);
        }
        let s = idle("x", "nobody", 0);
        assert!(s.is_empty());
        // 恢復為有效資料
        inject_test_data();
    }
}
