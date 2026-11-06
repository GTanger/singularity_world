fn replace_numeral_copper_currency(input: &str) -> String {
    const NUM_CHARS: &[char] = &[
        '一', '二', '兩', '三', '四', '五', '六', '七', '八', '九', '十', '幾',
    ];
    const NON_CURRENCY_NEXT: &[char] = &[
        '壺', '盆', '爐', '鍋', '鎖', '鏡', '鐘', '鈴', '鼎', '盤', '碗', '杓', '勺', '罐',
        '盒', '匣', '燈', '柱', '環', '扣', '釘', '片', '塊', '板', '條', '絲', '線', '管',
        '礦', '鐵', '器', '像', '皮', '色', '臭', '活', '模', '尺', '秤',
    ];
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    for (i, &ch) in chars.iter().enumerate() {
        if ch == '銅' {
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = chars.get(i + 1).copied();
            if prev.is_some_and(|p| NUM_CHARS.contains(&p))
                && !next.is_some_and(|n| NON_CURRENCY_NEXT.contains(&n))
            {
                out.push('鎂');
                continue;
            }
        }
        out.push(ch);
    }
    out
}

pub fn replace_temporal_gang_chars(input: &str) -> String {
    const KEEP_NEXT: &[char] = &[
        '強', '烈', '硬', '直', '正', '毅', '猛', '健', '勁', '柔', '愎', '巧', '好',
        '鬃', '玉', '櫟', '赤', '鐵',
        '，', '。', '！', '？', '、', '：', '；', '」', '』', ' ',
    ];
    const KEEP_PREV: &[char] = &[
        '金', '花', '陽', '至', '內', '克', '方', '青', '含', '以', '柔', '志', '建', '永', '大',
        '阿', '偉', '國', '明', '文', '武', '振', '成', '家', '世', '宗',
    ];
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len() + 8);
    for (i, &ch) in chars.iter().enumerate() {
        if ch == '剛' {
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = chars.get(i + 1).copied();
            let keep = next.is_none_or(|n| KEEP_NEXT.contains(&n))
                || prev.is_some_and(|p| KEEP_PREV.contains(&p));
            if keep {
                out.push(ch);
            } else if !out.ends_with('才') {
                out.push('才');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn rewrite_sentence_final_bu_question(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    for (i, &ch) in chars.iter().enumerate() {
        if ch == '不' {
            let next = chars.get(i + 1).copied();
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            if matches!(next, Some('？' | '?')) && prev != Some('豈') {
                out.push('嗎');
                continue;
            }
        }
        out.push(ch);
    }
    out
}

pub fn replace_clause_initial_temporal_cai(input: &str) -> String {
    input.to_string()
}

pub fn fix_self_surname_vocative_in_pair(
    line: &str,
    speaker_name: &str,
    listener_name: &str,
) -> String {
    let sp_chars: Vec<char> = speaker_name.chars().collect();
    let ls_chars: Vec<char> = listener_name.chars().collect();
    if sp_chars.len() < 2 || ls_chars.len() < 2 {
        return line.to_string();
    }
    let speaker_surname = sp_chars[0];
    let listener_surname = ls_chars[0];
    if speaker_surname == listener_surname {
        return line.to_string();
    }

    const SUFFIXES: &[&str] = &[
        "師傅", "大姐", "大哥", "大叔", "大娘", "大嬸", "大伯", "阿姨", "老伯", "大夫", "掌櫃", "老闆", "先生", "姑娘", "婆婆", "阿伯", "阿公", "阿婆", "叔", "嬸", "伯", "姨", "哥", "姐", "老弟", "老兄", "掌事", "頭兒",
    ];
    const CLAUSE_DELIMS: &[char] = &[
        '，', '。', '！', '？', '；', '：', '、', ' ', '「', '『',
        ',', '.', '!', '?', ';', ':',
    ];

    let mut result = line.to_string();

    // 檢查行首是否以 speaker_name（或其「阿+名字第二字」）+「，」開頭，若是則替換為 listener_name（或直接移除該自呼前綴）
    let sp_trimmed = speaker_name.trim();
    let ls_trimmed = listener_name.trim();
    if !sp_trimmed.is_empty() {
        let direct_prefix = format!("{sp_trimmed}，");
        if result.starts_with(&direct_prefix) {
            let rest = result[direct_prefix.len()..].trim_start();
            if !ls_trimmed.is_empty() && !rest.starts_with(ls_trimmed) {
                result = format!("{ls_trimmed}，{rest}");
            } else {
                result = rest.to_string();
            }
        }
        if sp_chars.len() >= 2 {
            let a_alias = format!("阿{}，", sp_chars[1]);
            if result.starts_with(&a_alias) {
                let rest = result[a_alias.len()..].trim_start();
                if !ls_trimmed.is_empty() && !rest.starts_with(ls_trimmed) {
                    result = format!("{ls_trimmed}，{rest}");
                } else {
                    result = rest.to_string();
                }
            }
        }
    }

    for &suffix in SUFFIXES {
        let target = format!("{speaker_surname}{suffix}");
        let replacement = format!("{listener_surname}{suffix}");

        let target_ni = format!("{target}你");
        let repl_ni = format!("{replacement}你");
        if result.contains(&target_ni) {
            result = result.replace(&target_ni, &repl_ni);
        }

        for follow in ["，", "那邊", "這頭", "手裡"] {
            let pat = format!("{target}{follow}");
            let mut start = 0;
            while let Some(rel_idx) = result[start..].find(&pat) {
                let idx = start + rel_idx;
                let is_clause_start = if idx == 0 {
                    true
                } else {
                    let prev_ch = result[..idx].chars().last();
                    prev_ch.is_some_and(|c| CLAUSE_DELIMS.contains(&c))
                };
                if is_clause_start {
                    result = format!(
                        "{}{}{}",
                        &result[..idx],
                        replacement,
                        &result[idx + target.len()..]
                    );
                    start = idx + replacement.len();
                } else {
                    start = idx + pat.len();
                }
            }
        }
    }
    result
}

