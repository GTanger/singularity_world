// roomcheck — 房間 JSON 契約檢查（對齊 Go internal/roomcheck）。
// 驗證：objects 的 Move/Look 觸發字是否出現在 description/responses；
// 〔…〕括號內容是否對應 objects[].name。

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// 掃描選項。
pub struct Options {
    pub trigger_sockets: HashSet<String>,
    pub check_brackets: bool,
    pub strict_brackets: bool,
}

/// 掃描結果。
pub struct CheckResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// 解析逗號分隔的 socket 列表，如 "Move,Look"。
pub fn parse_socket_list(s: &str) -> HashSet<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// 預設觸發 socket：Move + Look。
pub fn default_trigger_sockets() -> HashSet<String> {
    let mut s = HashSet::new();
    s.insert("Move".to_string());
    s.insert("Look".to_string());
    s
}

/// 遞迴收集目錄下的 .json 檔（跳過 _ 開頭）。
fn collect_json_files(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let meta = fs::metadata(root)?;
    if !meta.is_dir() {
        let name = root.file_name().unwrap_or_default().to_string_lossy();
        if name.ends_with(".json") && !name.starts_with('_') {
            return Ok(vec![root.to_path_buf()]);
        }
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    collect_recursive(root, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, out)?;
        } else {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".json") && !name.starts_with('_') {
                out.push(path);
            }
        }
    }
    Ok(())
}

fn object_sockets(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let Some(serde_json::Value::Array(arr)) = obj.get("sockets") else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

fn room_response_corpus(objects: &[serde_json::Value]) -> String {
    let mut parts = Vec::new();
    for o in objects {
        let Some(obj) = o.as_object() else { continue };
        let Some(serde_json::Value::Object(resp)) = obj.get("responses") else { continue };
        for v in resp.values() {
            if let Some(s) = v.as_str()
                && !s.is_empty()
            {
                parts.push(s.to_string());
            }
        }
    }
    parts.join("\n")
}

/// 檢查單一房間 JSON。
pub fn check_file(path: &str, data: &[u8], opts: &Options, all_rooms: &HashSet<String>) -> CheckResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let top: HashMap<String, serde_json::Value> = match serde_json::from_slice(data) {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("{path}: JSON 解析失敗: {e}"));
            return CheckResult { errors, warnings };
        }
    };

    let room_id = top
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    let desc = top
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let Some(serde_json::Value::Array(raw_objs)) = top.get("objects") else {
        return CheckResult { errors, warnings };
    };

    // 收集所有 object name
    let mut names_in_room: HashSet<String> = HashSet::new();
    for o in raw_objs {
        if let Some(obj) = o.as_object()
            && let Some(n) = obj.get("name").and_then(|v| v.as_str())
        {
            let n = n.trim();
            if !n.is_empty() {
                names_in_room.insert(n.to_string());
            }
        }
    }

    let corpus = format!("{desc}\n{}", room_response_corpus(raw_objs));

    for (i, o) in raw_objs.iter().enumerate() {
        let Some(obj) = o.as_object() else { continue };
        let socks = object_sockets(obj);
        let sock_set: HashSet<&str> = socks.iter().map(|s| s.as_str()).collect();

        let mut trig: Vec<&str> = sock_set
            .iter()
            .filter(|s| opts.trigger_sockets.contains(**s))
            .copied()
            .collect();
        if trig.is_empty() {
            continue;
        }
        trig.sort();

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .unwrap_or("");
        let oid = obj
            .get("id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("objects[{i}]"));

        if name.is_empty() {
            errors.push(format!(
                "{path} [{room_id}] {oid}: 具觸發動作 {trig:?} 但 name 為空"
            ));
            continue;
        }

        let has_move = sock_set.contains("Move");
        let has_look = sock_set.contains("Look");
        let move_trig = opts.trigger_sockets.contains("Move");
        let look_trig = opts.trigger_sockets.contains("Look");

        let move_target = obj
            .get("move_to_room_id")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .unwrap_or("");
            
        if !move_target.is_empty() && !all_rooms.contains(move_target) {
            errors.push(format!(
                "{path} [{room_id}] {oid}: 檢查 Move，move_to_room_id「{move_target}」指向不存在的房間（死連結）"
            ));
        }

        if move_trig && has_move && !desc.contains(name) {
            errors.push(format!(
                "{path} [{room_id}] {oid}: 檢查 Move，name「{name}」未在 description 出現（無法從主描述點擊移動）"
            ));
        }

        if look_trig && has_look {
            if move_trig && has_move {
                continue;
            }
            if !corpus.contains(name) {
                errors.push(format!(
                    "{path} [{room_id}] {oid}: 檢查 Look，name「{name}」未在 description 亦未在同房任一 responses 文字出現（無法點擊 Look）"
                ));
            }
        }
    }

    // 括號檢查：手動掃描〔…〕（U+3014 / U+3015）
    if opts.check_brackets {
        for inner in extract_bracket_contents(desc) {
            if !names_in_room.contains(inner) {
                let msg = format!(
                    "{path} [{room_id}]: description 有〔{inner}〕但無 objects[].name 完全一致"
                );
                if opts.strict_brackets {
                    errors.push(msg);
                } else {
                    warnings.push(msg);
                }
            }
        }
    }

    CheckResult { errors, warnings }
}

/// 掃描多個根目錄，回傳匯總結果。
pub fn run(roots: &[String], work_dir: &Path, opts: &Options) -> CheckResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let roots: Vec<String> = if roots.is_empty() {
        vec!["data/rooms/editor".to_string()]
    } else {
        roots.to_vec()
    };

    let mut files_data = Vec::new();

    for rel in &roots {
        let root = if Path::new(rel).is_absolute() {
            PathBuf::from(rel)
        } else {
            work_dir.join(rel)
        };
        if !root.exists() {
            errors.push(format!("路徑不存在: {}", root.display()));
            continue;
        }
        let files = match collect_json_files(&root) {
            Ok(f) => f,
            Err(e) => {
                errors.push(format!("{}: {e}", root.display()));
                continue;
            }
        };
        for fp in files {
            let data = match fs::read(fp.clone()) {
                Ok(d) => d,
                Err(e) => {
                    errors.push(format!("{}: 讀取失敗: {e}", fp.display()));
                    continue;
                }
            };
            files_data.push((fp, data));
        }
    }

    // Pass 1: Collect ALL room IDs
    let mut all_rooms = HashSet::new();
    for (_fp, data) in &files_data {
        if let Ok(top) = serde_json::from_slice::<HashMap<String, serde_json::Value>>(data) {
            if let Some(id) = top.get("id").and_then(|v| v.as_str()) {
                all_rooms.insert(id.to_string());
            }
        }
    }

    // Pass 2: Validate against global IDs
    for (fp, data) in &files_data {
        let r = check_file(&fp.display().to_string(), data, opts, &all_rooms);
        errors.extend(r.errors);
        warnings.extend(r.warnings);
    }

    CheckResult { errors, warnings }
}

/// 從字串中取出所有〔…〕內的內容（trim 後）。
fn extract_bracket_contents(s: &str) -> Vec<&str> {
    let mut results = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find('\u{3014}') {
        let after_open = &rest[start + '\u{3014}'.len_utf8()..];
        if let Some(end) = after_open.find('\u{3015}') {
            let inner = after_open[..end].trim();
            if !inner.is_empty() {
                results.push(inner);
            }
            rest = &after_open[end + '\u{3015}'.len_utf8()..];
        } else {
            break;
        }
    }
    results
}

/// 印出結果並回傳是否通過（無錯誤）。
pub fn print_result(result: &CheckResult, trigger: &HashSet<String>) -> bool {
    if !result.warnings.is_empty() {
        eprintln!("--- 警告 ---");
        for w in &result.warnings {
            eprintln!("{w}");
        }
    }
    if !result.errors.is_empty() {
        eprintln!("--- 錯誤 ---");
        for e in &result.errors {
            eprintln!("{e}");
        }
        eprintln!("--- 房間檢查失敗：{} 筆錯誤 ---", result.errors.len());
        return false;
    }
    let keys: BTreeSet<&str> = trigger.iter().map(|s| s.as_str()).collect();
    print!("OK：房間檢查通過 trigger_sockets={keys:?}");
    if !result.warnings.is_empty() {
        print!("（警告 {} 筆已列於 stderr）", result.warnings.len());
    }
    println!();
    true
}
