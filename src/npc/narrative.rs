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
    let id_hash: u64 = npc_id.bytes().enumerate().fold(0u64, |acc, (i, b)| {
        acc.wrapping_add((b as u64).wrapping_mul(i as u64 + 31))
    });
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
    with_behaviors(|b| pick(&b.gather, npc_id, tick).map(|t| fill(t, name))).unwrap_or_default()
}

/// 漫遊行為敘事句。
pub fn wander(npc_id: &str, name: &str, tick: u64) -> String {
    with_behaviors(|b| pick(&b.wander, npc_id, tick).map(|t| fill(t, name))).unwrap_or_default()
}

/// 發呆行為敘事句。
pub fn idle(npc_id: &str, name: &str, tick: u64) -> String {
    with_behaviors(|b| pick(&b.idle, npc_id, tick).map(|t| fill(t, name))).unwrap_or_default()
}

/// 抵達行為敘事句。
pub fn arrive(npc_id: &str, name: &str, tick: u64) -> String {
    with_behaviors(|b| pick(&b.arrive, npc_id, tick).map(|t| fill(t, name))).unwrap_or_default()
}

/// 離開行為敘事句。
pub fn leave(npc_id: &str, name: &str, tick: u64) -> String {
    with_behaviors(|b| pick(&b.leave, npc_id, tick).map(|t| fill(t, name))).unwrap_or_default()
}
/// 依場域一等物件或活躍事件／劇本包產出 NPC 閒置敘事句。
#[must_use]
pub fn venue_or_event_pack_idle_line(
    room_name: &str,
    npc_name: &str,
    title_hint: &str,
    now_unix: i64,
) -> Option<String> {
    let r = room_name.trim();
    let n = npc_name.trim();
    if r.is_empty() || n.is_empty() {
        return None;
    }
    let hint = title_hint.trim();
    if let Some((ev, _sp)) =
        crate::game::peninsula_scripts::active_event_and_script_pack_for_room(r, None, now_unix)
    {
        let first_key = ev.key_objects.split('、').next().unwrap_or(ev.key_objects).trim();
        if let Some(venue) = crate::grid::resolve_any_room_venue_object(r, hint) {
            return Some(format!(
                "{}在{}圍繞【{}】忙碌，{}。",
                n, venue.venue_kind, first_key, venue.on_duty_fixture_activity
            ));
        }
    }
    if let Some(venue) = crate::grid::resolve_any_room_venue_object(r, hint) {
        return Some(format!(
            "{}在{}當值，{}。",
            n, venue.venue_kind, venue.on_duty_fixture_activity
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

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
        let _guard = TEST_LOCK.lock().unwrap();
        inject_test_data();
        let s = wander("npc_1", "小明", 0);
        assert_eq!(s, "小明 漫遊");
    }

    #[test]
    fn deterministic_for_same_params() {
        let _guard = TEST_LOCK.lock().unwrap();
        inject_test_data();
        let a = gather("npc_42", "老王", 100);
        let b = gather("npc_42", "老王", 100);
        assert_eq!(a, b);
    }

    #[test]
    fn empty_templates_returns_empty() {
        let _guard = TEST_LOCK.lock().unwrap();
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
    #[test]
    fn test_venue_or_event_pack_idle_line() {
        let now_unix = 1_700_000_000;

        // 1. 水輪石磨工坊（在 peninsula_scripts 中有劇本包／事件包配置）
        let mill_line = venue_or_event_pack_idle_line(
            "老堰頭・水輪石磨工坊",
            "石老爹",
            "當值石磨匠師",
            now_unix,
        );
        assert!(mill_line.is_some());
        let ml = mill_line.unwrap();
        assert!(ml.contains("石老爹"));
        assert!(ml.contains("水輪石磨工坊"));

        // 2. 三水大食堂
        let canteen_line = venue_or_event_pack_idle_line(
            "三水大食堂",
            "張大廚",
            "食堂主廚",
            now_unix,
        );
        assert!(canteen_line.is_some());
        let cl = canteen_line.unwrap();
        assert!(cl.contains("張大廚"));
        assert!(cl.contains("大食堂"));

        // 3. 荒蕪礫石空地（非場域物件、無劇本包配置）
        let wasteland_line = venue_or_event_pack_idle_line(
            "荒蕪礫石空地",
            "小旅人",
            "訪客",
            now_unix,
        );
        assert!(wasteland_line.is_none());

        // 4. 空名稱保護
        assert!(venue_or_event_pack_idle_line("", "石老爹", "", now_unix).is_none());
        assert!(venue_or_event_pack_idle_line("三水大食堂", "", "", now_unix).is_none());
    }
}
