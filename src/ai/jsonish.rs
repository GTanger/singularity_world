//! `jsonish` — 容錯 LLM 結構化輸出解析模組。
//!
//! 本模組內化 `BoundaryML/baml` 的 `jsonish` 與 `bex_sap` 容錯核心機制，
//! 針對大型語言模型常見之輸出瑕疵（Markdown 區塊、未轉義引號、未轉義換行、
//! 尾隨逗號、單引號、未引號鍵名、註解與截斷等）提供多階段抽取、修復與候選評分。

use serde::de::DeserializeOwned;
use std::collections::HashSet;

/// JSON 修復操作型別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonishFix {
    /// 從 Markdown 程式碼區塊提取
    MarkdownFenceExtracted,
    /// 修復字串內部未轉義的雙引號
    UnescapedQuoteFixed,
    /// 轉義字串字面量中的未轉義換行與跳格符號
    UnescapedNewlineEscaped,
    /// 移除物件或陣列結尾的多餘逗號
    TrailingCommaRemoved,
    /// 將單引號字串轉換為合規雙引號字串
    SingleQuoteConverted,
    /// 為無引號的物件鍵名補充雙引號
    UnquotedKeyQuoted,
    /// 剝除單行與多行註解
    CommentStripped,
    /// 在結尾自動補齊未閉合的字串、陣列或物件括號
    UnclosedDelimiterClosed,
}

/// JSON 解析元資料
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonishMeta {
    /// 本次修復套用的修復操作清單
    pub fixes: Vec<JsonishFix>,
    /// 命中的候選片段索引
    pub candidate_index: usize,
    /// 候選評分（越低越佳，包含修復懲罰與內容品質評定）
    pub score: i32,
}

/// 容器型別堆疊節點
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContainerKind {
    Object,
    Array,
}

/// 物件內部期待之欄位語法狀態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectExpect {
    KeyOrEnd,
    Colon,
    Value,
    CommaOrEnd,
}

/// 內部候選節點
struct CandidateItem {
    text: String,
    initial_fixes: Vec<JsonishFix>,
}

/// 計算單項修復之基礎懲罰分數
fn fix_penalty(fix: JsonishFix) -> i32 {
    match fix {
        JsonishFix::MarkdownFenceExtracted => 1,
        JsonishFix::CommentStripped => 2,
        JsonishFix::TrailingCommaRemoved => 3,
        JsonishFix::SingleQuoteConverted => 4,
        JsonishFix::UnquotedKeyQuoted => 5,
        JsonishFix::UnescapedNewlineEscaped => 6,
        JsonishFix::UnescapedQuoteFixed => 8,
        JsonishFix::UnclosedDelimiterClosed => 12,
    }
}

/// 評估反序列化結果之資料飽滿度品質評分（越低越佳，非空字串有獎勵負分）
fn compute_quality_score(val: &serde_json::Value) -> i32 {
    let mut penalty: i32 = 0;
    match val {
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                penalty += 100;
            } else {
                for (_k, v) in map {
                    match v {
                        serde_json::Value::String(s) => {
                            if s.trim().is_empty() {
                                penalty += 20;
                            } else {
                                penalty -= 25;
                            }
                        }
                        serde_json::Value::Array(arr) => {
                            if arr.is_empty() {
                                penalty += 10;
                            } else {
                                penalty -= 15;
                            }
                        }
                        serde_json::Value::Null => {
                            penalty += 10;
                        }
                        _ => {
                            penalty -= 10;
                        }
                    }
                }
            }
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                penalty += 50;
            } else {
                let len_penalty = arr.len().min(10) as i32 * 15;
                penalty -= len_penalty;
            }
        }
        serde_json::Value::String(s) => {
            if s.trim().is_empty() {
                penalty += 50;
            } else {
                penalty -= 30;
            }
        }
        _ => {}
    }
    penalty
}

/// 窺探下一個非空白且非註解之字元
fn peek_next_token(chars: &[char], from: usize) -> Option<(usize, char)> {
    let mut i = from;
    let len = chars.len();
    while i < len {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < len {
            if chars[i + 1] == '/' {
                i += 2;
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            } else if chars[i + 1] == '*' {
                i += 2;
                while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                if i + 1 < len {
                    i += 2;
                } else {
                    i = len;
                }
                continue;
            }
        }
        return Some((i, c));
    }
    None
}

/// 階段 2：提取 Markdown 程式碼區塊（支援 ```json ... ``` 或 ``` ... ```）
fn extract_markdown_code_fences(raw: &str) -> Vec<String> {
    let mut fences = Vec::new();
    let mut remaining = raw;
    while let Some(start_idx) = remaining.find("```") {
        let after_ticks = &remaining[start_idx + 3..];
        let after_ticks = after_ticks.strip_prefix('`').unwrap_or(after_ticks);
        let trimmed_leading = after_ticks.trim_start();
        if trimmed_leading.starts_with('{') || trimmed_leading.starts_with('[') {
            if let Some(end_idx) = after_ticks.find("```") {
                let content = after_ticks[..end_idx].trim();
                if !content.is_empty() {
                    fences.push(content.to_string());
                }
                remaining = &after_ticks[end_idx + 3..];
            } else {
                let content = after_ticks.trim();
                if !content.is_empty() {
                    fences.push(content.to_string());
                }
                break;
            }
            continue;
        }

        if let Some(newline_idx) = after_ticks.find('\n') {
            let body = &after_ticks[newline_idx + 1..];
            if let Some(end_idx) = body.find("```") {
                let content = body[..end_idx].trim();
                if !content.is_empty() {
                    fences.push(content.to_string());
                }
                remaining = &body[end_idx + 3..];
            } else {
                let content = body.trim();
                if !content.is_empty() {
                    fences.push(content.to_string());
                }
                break;
            }
        } else {
            break;
        }
    }
    fences
}

/// 階段 3：字串感知之括號深度掃描器（掃描所有頂層物件與陣列，含結尾截斷部分）
fn scan_json_candidates(raw: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    let chars: Vec<char> = raw.chars().collect();
    let len = chars.len();
    let mut i = 0;

    let mut in_str: Option<char> = None;
    let mut stack: Vec<char> = Vec::new();
    let mut start_idx: Option<usize> = None;

    while i < len {
        let c = chars[i];

        if in_str.is_some() && c == '\\' {
            i += 2;
            continue;
        }

        if let Some(quote) = in_str {
            if c == quote {
                in_str = None;
            }
            i += 1;
            continue;
        } else if c == '"' || c == '\'' {
            in_str = Some(c);
            i += 1;
            continue;
        }

        if c == '/' && i + 1 < len {
            if chars[i + 1] == '/' {
                i += 2;
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            } else if chars[i + 1] == '*' {
                i += 2;
                while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                if i + 1 < len {
                    i += 2;
                } else {
                    i = len;
                }
                continue;
            }
        }

        if c == '{' || c == '[' {
            if stack.is_empty() {
                start_idx = Some(i);
            }
            stack.push(c);
        } else if c == '}' {
            if stack.last() == Some(&'{') {
                stack.pop();
                if stack.is_empty() {
                    if let Some(start) = start_idx.take() {
                        let candidate: String = chars[start..=i].iter().collect();
                        candidates.push(candidate);
                    }
                }
            }
        } else if c == ']' {
            if stack.last() == Some(&'[') {
                stack.pop();
                if stack.is_empty() {
                    if let Some(start) = start_idx.take() {
                        let candidate: String = chars[start..=i].iter().collect();
                        candidates.push(candidate);
                    }
                }
            }
        }

        i += 1;
    }

    if let Some(start) = start_idx {
        if start < len {
            let candidate: String = chars[start..].iter().collect();
            candidates.push(candidate);
        }
    }

    candidates
}

/// 階段 4：狀態機修復解析器，針對各類 LLM 語法異常進行修復
fn repair_jsonish_candidate(raw: &str) -> (String, Vec<JsonishFix>) {
    let chars: Vec<char> = raw.chars().collect();
    let len = chars.len();
    let mut out = String::with_capacity(raw.len() + 32);
    let mut fixes = Vec::new();
    let mut i = 0;

    let mut container_stack: Vec<ContainerKind> = Vec::new();
    let mut object_state_stack: Vec<ObjectExpect> = Vec::new();

    while i < len {
        let c = chars[i];

        // 1. 剝除註解（字串外部）
        if c == '/' && i + 1 < len {
            if chars[i + 1] == '/' {
                fixes.push(JsonishFix::CommentStripped);
                i += 2;
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            } else if chars[i + 1] == '*' {
                fixes.push(JsonishFix::CommentStripped);
                i += 2;
                while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                if i + 1 < len {
                    i += 2;
                } else {
                    i = len;
                }
                continue;
            }
        }

        // 2. 雙引號字串掃描與未轉義雙引號前瞻啟發修復
        if c == '"' {
            out.push('"');
            i += 1;
            let in_object = container_stack.last() == Some(&ContainerKind::Object);
            let is_key = in_object && object_state_stack.last() == Some(&ObjectExpect::KeyOrEnd);
            let mut closed = false;

            while i < len {
                let sc = chars[i];
                if sc == '\\' {
                    if i + 1 < len {
                        let next_sc = chars[i + 1];
                        out.push('\\');
                        out.push(next_sc);
                        i += 2;
                    } else {
                        out.push('\\');
                        i += 1;
                    }
                    continue;
                }

                if sc == '\n' {
                    out.push_str(r#"\n"#);
                    fixes.push(JsonishFix::UnescapedNewlineEscaped);
                    i += 1;
                    continue;
                }
                if sc == '\r' {
                    out.push_str(r#"\r"#);
                    fixes.push(JsonishFix::UnescapedNewlineEscaped);
                    i += 1;
                    continue;
                }
                if sc == '\t' {
                    out.push_str(r#"\t"#);
                    fixes.push(JsonishFix::UnescapedNewlineEscaped);
                    i += 1;
                    continue;
                }

                if sc == '"' {
                    let should_close = if is_key {
                        let next_tok = peek_next_token(&chars, i + 1);
                        next_tok.map(|(_, ch)| ch) == Some(':')
                    } else {
                        let next1 = peek_next_token(&chars, i + 1);
                        match next1 {
                            None => true,
                            Some((_, ',' | '}' | ']')) => true,
                            Some((idx1, '"')) => {
                                let next2 = peek_next_token(&chars, idx1 + 1);
                                match next2 {
                                    Some((_, ',' | '}' | ']')) | None => false,
                                    Some((_, ':')) => true,
                                    _ => false,
                                }
                            }
                            Some((_, ':')) => false,
                            Some((_, _)) => false,
                        }
                    };

                    if should_close {
                        out.push('"');
                        closed = true;
                        i += 1;
                        if in_object {
                            if let Some(state) = object_state_stack.last_mut() {
                                *state = if is_key {
                                    ObjectExpect::Colon
                                } else {
                                    ObjectExpect::CommaOrEnd
                                };
                            }
                        }
                        break;
                    } else {
                        out.push_str(r#"\""#);
                        fixes.push(JsonishFix::UnescapedQuoteFixed);
                        i += 1;
                        continue;
                    }
                }

                out.push(sc);
                i += 1;
            }

            if !closed {
                out.push('"');
                fixes.push(JsonishFix::UnclosedDelimiterClosed);
                if in_object {
                    if let Some(state) = object_state_stack.last_mut() {
                        *state = if is_key {
                            ObjectExpect::Colon
                        } else {
                            ObjectExpect::CommaOrEnd
                        };
                    }
                }
            }
            continue;
        }

        // 3. 單引號字串轉換為合規雙引號字串
        if c == '\'' {
            out.push('"');
            i += 1;
            fixes.push(JsonishFix::SingleQuoteConverted);
            let in_object = container_stack.last() == Some(&ContainerKind::Object);
            let is_key = in_object && object_state_stack.last() == Some(&ObjectExpect::KeyOrEnd);
            let mut closed = false;

            while i < len {
                let sc = chars[i];
                if sc == '\\' {
                    if i + 1 < len {
                        let next_sc = chars[i + 1];
                        if next_sc == '\'' {
                            out.push('\'');
                            i += 2;
                            continue;
                        } else if next_sc == '"' {
                            out.push_str(r#"\""#);
                            i += 2;
                            continue;
                        } else {
                            out.push('\\');
                            out.push(next_sc);
                            i += 2;
                            continue;
                        }
                    } else {
                        out.push('\\');
                        i += 1;
                        continue;
                    }
                }

                if sc == '"' {
                    out.push_str(r#"\""#);
                    i += 1;
                    continue;
                }

                if sc == '\n' {
                    out.push_str(r#"\n"#);
                    fixes.push(JsonishFix::UnescapedNewlineEscaped);
                    i += 1;
                    continue;
                }
                if sc == '\r' {
                    out.push_str(r#"\r"#);
                    fixes.push(JsonishFix::UnescapedNewlineEscaped);
                    i += 1;
                    continue;
                }
                if sc == '\t' {
                    out.push_str(r#"\t"#);
                    fixes.push(JsonishFix::UnescapedNewlineEscaped);
                    i += 1;
                    continue;
                }

                if sc == '\'' {
                    let should_close = if is_key {
                        let next_tok = peek_next_token(&chars, i + 1);
                        next_tok.map(|(_, ch)| ch) == Some(':')
                    } else {
                        let next1 = peek_next_token(&chars, i + 1);
                        match next1 {
                            None => true,
                            Some((_, ',' | '}' | ']')) => true,
                            Some((idx1, '\'')) => {
                                let next2 = peek_next_token(&chars, idx1 + 1);
                                match next2 {
                                    Some((_, ',' | '}' | ']')) | None => false,
                                    Some((_, ':')) => true,
                                    _ => false,
                                }
                            }
                            Some((_, ':')) => false,
                            Some((_, _)) => false,
                        }
                    };

                    if should_close {
                        out.push('"');
                        closed = true;
                        i += 1;
                        if in_object {
                            if let Some(state) = object_state_stack.last_mut() {
                                *state = if is_key {
                                    ObjectExpect::Colon
                                } else {
                                    ObjectExpect::CommaOrEnd
                                };
                            }
                        }
                        break;
                    } else {
                        out.push('\'');
                        i += 1;
                        continue;
                    }
                }

                out.push(sc);
                i += 1;
            }

            if !closed {
                out.push('"');
                fixes.push(JsonishFix::UnclosedDelimiterClosed);
                if in_object {
                    if let Some(state) = object_state_stack.last_mut() {
                        *state = if is_key {
                            ObjectExpect::Colon
                        } else {
                            ObjectExpect::CommaOrEnd
                        };
                    }
                }
            }
            continue;
        }

        // 4. 物件鍵位無引號標識符補充引號
        if container_stack.last() == Some(&ContainerKind::Object)
            && object_state_stack.last() == Some(&ObjectExpect::KeyOrEnd)
            && !c.is_whitespace()
            && c != '}'
            && c != '{'
            && c != '['
            && c != ']'
            && c != ','
            && c != ':'
            && c != '/'
        {
            let mut j = i;
            while j < len
                && !chars[j].is_whitespace()
                && chars[j] != ':'
                && chars[j] != '{'
                && chars[j] != '}'
                && chars[j] != '['
                && chars[j] != ']'
                && chars[j] != ','
                && chars[j] != '"'
                && chars[j] != '\''
                && chars[j] != '/'
            {
                j += 1;
            }
            if j > i {
                let next_tok = peek_next_token(&chars, j);
                if next_tok.map(|(_, ch)| ch) == Some(':') {
                    let key_str: String = chars[i..j].iter().collect();
                    out.push('"');
                    out.push_str(&key_str);
                    out.push('"');
                    fixes.push(JsonishFix::UnquotedKeyQuoted);
                    i = j;
                    if let Some(state) = object_state_stack.last_mut() {
                        *state = ObjectExpect::Colon;
                    }
                    continue;
                }
            }
        }

        // 5. 尾隨逗號檢查與移除
        if c == ',' {
            let next_tok = peek_next_token(&chars, i + 1);
            let is_trailing = match next_tok.map(|(_, ch)| ch) {
                Some('}') | Some(']') | Some(',') => true,
                None => true,
                _ => false,
            };

            if is_trailing {
                fixes.push(JsonishFix::TrailingCommaRemoved);
                i += 1;
                continue;
            } else {
                out.push(',');
                if container_stack.last() == Some(&ContainerKind::Object) {
                    if let Some(state) = object_state_stack.last_mut() {
                        *state = ObjectExpect::KeyOrEnd;
                    }
                }
                i += 1;
                continue;
            }
        }

        // 6. 容器開閉括號維護
        if c == '{' {
            container_stack.push(ContainerKind::Object);
            object_state_stack.push(ObjectExpect::KeyOrEnd);
            out.push('{');
            i += 1;
            continue;
        }
        if c == '}' {
            if container_stack.last() == Some(&ContainerKind::Object) {
                container_stack.pop();
                object_state_stack.pop();
                if container_stack.last() == Some(&ContainerKind::Object) {
                    if let Some(state) = object_state_stack.last_mut() {
                        *state = ObjectExpect::CommaOrEnd;
                    }
                }
            }
            out.push('}');
            i += 1;
            continue;
        }
        if c == '[' {
            container_stack.push(ContainerKind::Array);
            out.push('[');
            i += 1;
            continue;
        }
        if c == ']' {
            if container_stack.last() == Some(&ContainerKind::Array) {
                container_stack.pop();
                if container_stack.last() == Some(&ContainerKind::Object) {
                    if let Some(state) = object_state_stack.last_mut() {
                        *state = ObjectExpect::CommaOrEnd;
                    }
                }
            }
            out.push(']');
            i += 1;
            continue;
        }
        if c == ':' {
            if container_stack.last() == Some(&ContainerKind::Object) {
                if let Some(state) = object_state_stack.last_mut() {
                    *state = ObjectExpect::Value;
                }
            }
            out.push(':');
            i += 1;
            continue;
        }

        out.push(c);
        i += 1;
    }

    // 7. 結尾處未閉合容器與尾隨逗號收尾修復
    let trimmed_end = out.trim_end();
    if trimmed_end.ends_with(',') {
        let trimmed_len = trimmed_end.len() - 1;
        out.truncate(trimmed_len);
        fixes.push(JsonishFix::TrailingCommaRemoved);
    }

    if let Some(state) = object_state_stack.last() {
        match state {
            ObjectExpect::Colon => {
                out.push_str(": null");
                fixes.push(JsonishFix::UnclosedDelimiterClosed);
            }
            ObjectExpect::Value => {
                out.push_str("null");
                fixes.push(JsonishFix::UnclosedDelimiterClosed);
            }
            _ => {}
        }
    }

    while let Some(container) = container_stack.pop() {
        match container {
            ContainerKind::Object => {
                out.push('}');
                fixes.push(JsonishFix::UnclosedDelimiterClosed);
            }
            ContainerKind::Array => {
                out.push(']');
                fixes.push(JsonishFix::UnclosedDelimiterClosed);
            }
        }
    }

    (out, fixes)
}

/// 比較更新最佳候選反序列化結果
fn update_best_match<T>(
    best: &mut Option<(T, JsonishMeta)>,
    val: T,
    fixes: Vec<JsonishFix>,
    candidate_index: usize,
    score: i32,
) {
    let is_better = match best {
        None => true,
        Some((_, current_meta)) => {
            if score < current_meta.score {
                true
            } else if score == current_meta.score {
                fixes.len() < current_meta.fixes.len()
            } else {
                false
            }
        }
    };
    if is_better {
        *best = Some((
            val,
            JsonishMeta {
                fixes,
                candidate_index,
                score,
            },
        ));
    }
}

/// 容錯解析 JSON 字串並產出修復與品質元資料
pub fn parse_jsonish_with_meta<T: DeserializeOwned>(raw: &str) -> Option<(T, JsonishMeta)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 階段 1：直接快速路徑
    if let Ok(val) = serde_json::from_str::<T>(trimmed) {
        return Some((
            val,
            JsonishMeta {
                fixes: Vec::new(),
                candidate_index: 0,
                score: 0,
            },
        ));
    }

    // 彙整候選集合
    let mut candidates: Vec<CandidateItem> = Vec::new();
    let mut seen_texts = HashSet::new();

    let mut add_candidate = |text: String, fixes: Vec<JsonishFix>| {
        let t = text.trim();
        if !t.is_empty() && seen_texts.insert(t.to_string()) {
            candidates.push(CandidateItem {
                text: t.to_string(),
                initial_fixes: fixes,
            });
        }
    };

    // 階段 2：Markdown 程式碼區塊候選
    let fences = extract_markdown_code_fences(raw);
    for fence in fences {
        add_candidate(fence, vec![JsonishFix::MarkdownFenceExtracted]);
    }

    // 階段 3：頂層物件／陣列掃描候選
    let scanned = scan_json_candidates(raw);
    for s in scanned {
        add_candidate(s, Vec::new());
    }

    // 加入去除首尾空白之原文自身
    add_candidate(trimmed.to_string(), Vec::new());

    let mut best_match: Option<(T, JsonishMeta)> = None;

    for (idx, cand) in candidates.into_iter().enumerate() {
        // 先嘗試原始片段直接反序列化
        if let Ok(val) = serde_json::from_str::<T>(&cand.text) {
            let quality = serde_json::from_str::<serde_json::Value>(&cand.text)
                .map(|v| compute_quality_score(&v))
                .unwrap_or(0);
            let fix_score: i32 = cand.initial_fixes.iter().map(|f| fix_penalty(*f)).sum();
            let score = fix_score + quality;
            update_best_match(&mut best_match, val, cand.initial_fixes.clone(), idx, score);
        }

        // 階段 4：狀態機修復後再反序列化
        let (repaired, repair_fixes) = repair_jsonish_candidate(&cand.text);
        let mut all_fixes = cand.initial_fixes;
        all_fixes.extend(repair_fixes);

        if let Ok(val) = serde_json::from_str::<T>(&repaired) {
            let quality = serde_json::from_str::<serde_json::Value>(&repaired)
                .map(|v| compute_quality_score(&v))
                .unwrap_or(0);
            let fix_score: i32 = all_fixes.iter().map(|f| fix_penalty(*f)).sum();
            let score = fix_score + quality;
            update_best_match(&mut best_match, val, all_fixes, idx, score);
        }
    }

    best_match
}

/// 容錯解析 JSON 字串
pub fn parse_jsonish<T: DeserializeOwned>(raw: &str) -> Option<T> {
    parse_jsonish_with_meta::<T>(raw).map(|(val, _)| val)
}

/// 修復 JSON-like 字串使其成為合規 JSON 字串
pub fn repair_jsonish_str(raw: &str) -> String {
    repair_jsonish_candidate(raw).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct SkillDataTest {
        name: String,
        origin_basis: String,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct DialogueTest {
        a: String,
        b: String,
        #[serde(default)]
        anchors: Vec<String>,
    }

    #[test]
    fn test_clean_json_fast_path() {
        let raw = r#"{"name": "春風拂面", "origin_basis": "長劍輕舞，身法飄逸如風。"}"#;
        let res = parse_jsonish_with_meta::<SkillDataTest>(raw);
        assert!(res.is_some());
        let (val, meta) = res.expect("解析成功");
        assert_eq!(val.name, "春風拂面");
        assert_eq!(val.origin_basis, "長劍輕舞，身法飄逸如風。");
        assert_eq!(meta.candidate_index, 0);
        assert_eq!(meta.score, 0);
        assert!(meta.fixes.is_empty());
    }

    #[test]
    fn test_markdown_fenced_json_with_surrounding_brackets() {
        let raw = r#"
思考過程中提到一些名詞 {流風與破浪}，以下為輸出：
```json
{
  "name": "風捲殘雲",
  "origin_basis": "身形急轉如旋風，長劍橫掃千軍。"
}
```
額外備註 {請務必保證格式正確}
"#;
        let res = parse_jsonish_with_meta::<SkillDataTest>(raw);
        assert!(res.is_some());
        let (val, meta) = res.expect("解析成功");
        assert_eq!(val.name, "風捲殘雲");
        assert_eq!(val.origin_basis, "身形急轉如旋風，長劍橫掃千軍。");
        assert!(meta.fixes.contains(&JsonishFix::MarkdownFenceExtracted));
    }

    #[test]
    fn test_multiple_json_objects_selection() {
        let raw = r#"
範例格式如下：
{"example": true, "note": "format"}
實際產出結果：
{"name": "白浪掀天", "origin_basis": "步法如風，雙手如潮"}
尾部附註：{"status": "completed"}
"#;
        let res = parse_jsonish::<SkillDataTest>(raw);
        assert!(res.is_some());
        let val = res.expect("解析成功");
        assert_eq!(val.name, "白浪掀天");
        assert_eq!(val.origin_basis, "步法如風，雙手如潮");
    }

    #[test]
    fn test_unescaped_inner_quotes_chinese() {
        let raw = r#"{"a": "老張嘆道："這風雪怕是停不了了。"", "b": "先喝口熱湯吧。"}"#;
        let res = parse_jsonish_with_meta::<DialogueTest>(raw);
        assert!(res.is_some());
        let (val, meta) = res.expect("解析成功");
        assert_eq!(val.a, "老張嘆道：\"這風雪怕是停不了了。\"");
        assert_eq!(val.b, "先喝口熱湯吧。");
        assert!(meta.fixes.contains(&JsonishFix::UnescapedQuoteFixed));
    }

    #[test]
    fn test_unescaped_literal_newlines() {
        let raw = "{\n  \"name\": \"破空斬\",\n  \"origin_basis\": \"第一行動作\n第二行身法\r\n第三行銘文\"\n}";
        let res = parse_jsonish_with_meta::<SkillDataTest>(raw);
        assert!(res.is_some());
        let (val, meta) = res.expect("解析成功");
        assert_eq!(val.name, "破空斬");
        assert!(val.origin_basis.contains("第一行動作\n第二行身法\r\n第三行銘文"));
        assert!(meta.fixes.contains(&JsonishFix::UnescapedNewlineEscaped));
    }

    #[test]
    fn test_trailing_commas() {
        let raw = r#"{"a": "你好", "b": "早安", "anchors": ["客棧", "風雪",],}"#;
        let res = parse_jsonish_with_meta::<DialogueTest>(raw);
        assert!(res.is_some());
        let (val, meta) = res.expect("解析成功");
        assert_eq!(val.a, "你好");
        assert_eq!(val.b, "早安");
        assert_eq!(val.anchors, vec!["客棧".to_string(), "風雪".to_string()]);
        assert!(meta.fixes.contains(&JsonishFix::TrailingCommaRemoved));
    }

    #[test]
    fn test_unquoted_keys_and_single_quotes() {
        let raw = r#"{a: '對話甲', b: '對話乙', anchors: ['風雪',]}"#;
        let res = parse_jsonish_with_meta::<DialogueTest>(raw);
        assert!(res.is_some());
        let (val, meta) = res.expect("解析成功");
        assert_eq!(val.a, "對話甲");
        assert_eq!(val.b, "對話乙");
        assert_eq!(val.anchors, vec!["風雪".to_string()]);
        assert!(meta.fixes.contains(&JsonishFix::UnquotedKeyQuoted));
        assert!(meta.fixes.contains(&JsonishFix::SingleQuoteConverted));
        assert!(meta.fixes.contains(&JsonishFix::TrailingCommaRemoved));
    }

    #[test]
    fn test_truncated_json_at_eof() {
        let raw = r#"{"a": "對話甲", "b": "乙回覆中途中斷"#;
        let res = parse_jsonish_with_meta::<DialogueTest>(raw);
        assert!(res.is_some());
        let (val, meta) = res.expect("解析成功");
        assert_eq!(val.a, "對話甲");
        assert_eq!(val.b, "乙回覆中途中斷");
        assert!(meta.fixes.contains(&JsonishFix::UnclosedDelimiterClosed));
    }

    #[test]
    fn test_comments_stripped() {
        let raw = r#"
{
    // 招式名稱說明
    "name": "回風落雁",
    /* 演繹真意說明 */
    "origin_basis": "長劍飄渺，身法宛如落雁歸巢。"
}
"#;
        let res = parse_jsonish_with_meta::<SkillDataTest>(raw);
        assert!(res.is_some());
        let (val, meta) = res.expect("解析成功");
        assert_eq!(val.name, "回風落雁");
        assert_eq!(val.origin_basis, "長劍飄渺，身法宛如落雁歸巢。");
        assert!(meta.fixes.contains(&JsonishFix::CommentStripped));
    }

    #[test]
    fn test_repair_jsonish_str() {
        let raw = r#"{ a: '測試', b: "內含"引號"對話", }"#;
        let repaired = repair_jsonish_str(raw);
        let parsed: serde_json::Value = serde_json::from_str(&repaired).expect("修復後為合規 JSON");
        assert_eq!(parsed["a"], "測試");
        assert_eq!(parsed["b"], "內含\"引號\"對話");
    }
}
