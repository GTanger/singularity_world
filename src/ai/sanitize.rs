// NPC 台詞清理，對齊既有 ai/sanitize。

use crate::gametext;

/// 移除行首「Name:」或「Name：」前綴。
pub fn strip_npc_line_speaker_prefix(line: &str, names: &[&str]) -> String {
    let mut line = line.trim().to_string();
    for name in names {
        if name.is_empty() {
            continue;
        }
        let colon = format!("{name}:");
        if line.starts_with(&colon) {
            line = line[colon.len()..].trim().to_string();
            break;
        }
        let fullwidth = format!("{name}：");
        if line.starts_with(&fullwidth) {
            line = line[fullwidth.len()..].trim().to_string();
            break;
        }
    }
    line
}

/// 清理後台詞；若應丟棄則回傳空字串。
pub fn sanitize_npc_dialogue_line(line: &str) -> String {
    let mut s = line.trim().to_string();
    if s.is_empty() {
        return String::new();
    }
    let low = s.to_lowercase();
    let san = gametext::sanitize();
    for t in &san.think_tokens {
        if !t.is_empty() && low.contains(&t.to_lowercase()) {
            return String::new();
        }
    }
    if !san.code_fence.is_empty() && s.contains(&san.code_fence) {
        return String::new();
    }
    for m in &san.self_correct_markers {
        if !m.is_empty() && s.contains(m.as_str()) {
            return String::new();
        }
    }
    for m in &san.assistant_markers {
        if !m.is_empty() && s.contains(m.as_str()) {
            return String::new();
        }
    }
    let gopen = '\u{300c}';
    let gclose = '\u{300d}';
    while s.starts_with(gopen) && s.ends_with(gclose) {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() < 2 {
            break;
        }
        s = chars[1..chars.len() - 1]
            .iter()
            .collect::<String>()
            .trim()
            .to_string();
    }
    let gc = "\u{300d}";
    let g2 = format!("{gc}{gc}");
    while s.ends_with(&g2) {
        let Some(without) = s.strip_suffix(gc) else {
            break;
        };
        s = without.trim_end().to_string();
    }
    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_prefix_colon() {
        assert_eq!(
            strip_npc_line_speaker_prefix("阿明：你好", &["阿明", "小華"]),
            "你好"
        );
    }

    #[test]
    fn strip_prefix_ascii_colon() {
        assert_eq!(
            strip_npc_line_speaker_prefix("Bob: hi", &["Bob"]),
            "hi"
        );
    }

    #[test]
    fn sanitize_trims_corner_quotes() {
        let s = sanitize_npc_dialogue_line("\u{300c}一句話\u{300d}");
        assert_eq!(s, "一句話");
    }
}
