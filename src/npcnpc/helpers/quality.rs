/// 單句品質閘（對齊既有 `qualityGateNpcLine`）；通過則 `(true, "")`。
pub fn quality_gate_npc_line(line: &str, max_runes: i32) -> (bool, &'static str) {
    let max = if max_runes <= 0 {
        QUALITY_GATE_DEFAULT_MAX_RUNES
    } else {
        max_runes
    };
    let s = line.trim();
    if s.is_empty() {
        return (false, "empty");
    }
    if s.contains('\u{fffd}') {
        return (false, "unicode_garbage");
    }
    let t = s.trim();
    let t_runes: Vec<char> = t.chars().collect();
    if t == "{" || t == "}" || (t.starts_with('{') && t_runes.len() <= 4) {
        return (false, "json_residue");
    }
    let low = s.to_lowercase();
    for leak in ["npc_", "entity_", "room_", "wt_", "mat_", "grid:", "scene_stub"] {
        if low.contains(leak) {
            return (false, "leaked_id");
        }
    }
    if low.contains("street") || low.contains("zone") {
        return (false, "leaked_id");
    }
    for forbidden in [
        "小市",
        "短市",
        "剛才",
        "剛剛",
        "資料庫",
        "聖塔",
        "110v",
        "110V",
        "念場",
        "富態",
        "城外惡地",
        "沃土風",
        "非核冬天",
        "低飽和微光",
        "玉女克全真",
        "庖丁解牛",
        "一力降十會",
        "十會解一力",
        "五常運動鏈",
        "換招破綻窗",
        "語意字根",
        "骨脈解構",
        "差官",
        "報表",
        "導能鎂晶",
        "銀票",
        "走你",
        "一趨",
        "松快",
        "回来",
        "這样",
        "晒乾",
        "采草",
        "老伴兒",
        "涼白開",
        "加班",
        "加了班",
        "加個班",
        "上班",
        "下班",
        "報備",
        "報修",
        "那伙人",
        "黑沼關隘",
        "鹽田澳",
        "半個鐘",
        "陽春麵",
        "開水",
        "防爆",
        "黑沼林",
        "西南原",
        "白塔山麓",
        "北聖山",
        "西北礦帶",
        "方才",
        "快遞",
        "外帶",
        "打卡",
        "時辰",
        "客官",
        "小二",
        "掌櫃",
        "銅錢",
        "文錢",
        "銀兩",
        "碎銀",
        "玄鐵",
        "骨相",
        "成相",
        "字印",
        "詞元",
        "書肆",
        "茶肆",
        "酒肆",
        "客棧",
        "保鏢",
        "鏢局",
        "老鏢頭",
        "排班",
        "發霉",
        "倒霉",
        "霉味",
        "老少咸宜",
        "衙門",
        "老夫",
        "哥們",
        "宗門",
        "神識",
        "適才",
        "丹田",
        "官衙",
        "差役",
        "住店",
        "銀幣",
        "銅幣",
        "百金",
        "十銀",
        "五十銅",
        "玉幣",
        "防暴",
        "效率",
        "數據",
        "系統",
        "體適能夏令營",
        "仰臥起坐板",
        "導彈",
        "水泥",
        "猎",
        "账",
        "调试",
        "靈氣",
        "靈力",
        "修煉",
        "神仙",
        "幫派",
        "門派",
        "江湖",
        "鏢資",
        "機器人",
        "瑜伽",
        "輻射",
        "法術",
        "晶片",
        "電池",
        "工廠",
        "兵工廠",
        "跟人說了會兒話",
        "螺絲",
        "扳手",
        "蒸汽鍛打機",
        "機油",
        "化學反應",
        "少林寺",
        "舍利子",
        "瘋人院",
        "訂書機",
        "奇點神教",
        "奇點世界",
        "魔法",
        "法杖",
        "牧師",
        "教堂",
        "鈔票",
        "五十金",
        "不鏽鋼",
        "不銹鋼",
        "尼龍",
        "塑膠",
        "塑料",
        "錢幣",
        "金幣",
        "鎂鈔",
        "氧化",
        "刺猬",
        "啥用",
        "過個週",
        "縮工時",
        "傘骨廠",
        "酒廠",
        "釀酒廠",
        "咱",
        "啥",
        "天氣預報",
        "待會兒",
        "一會兒",
        "這會兒",
        "那會兒",
        "玩意兒",
        "門兒清",
        "麻利",
        "車廠",
        "廠棚",
        "襯衫",
        "活兒",
        "大夥兒",
        "事兒",
        "勁兒",
        "門兒",
        "味兒",
        "昨兒",
        "今兒",
        "老花鏡",
        "眼鏡",
        "鋸木廠",
        "造船廠",
        "精煉廠",
        "會計事務所",
        "商務會計教室",
        "會計師",
        "編輯助理",
        "紀錄員",
        "護理員",
        "外務員",
        "民事調解員",
        "陸老師宿舍",
        "老師宿舍",
        "保鮮櫃",
        "顯擺",
        "憋屈",
        "扯淡",
        "閒磕牙",
        "八卦鏡",
        "染寒熱",
        "防感冒草藥",
        "系學生",
        "生員",
        "巡防課",
        "研究員",
        "治學士",
        "商算大匠",
        "校書生",
        "抄錄書生",
        "得瑟",
        "嘚瑟",
        "窩囊",
        "磨嘰",
        "教室",
        "鍋爐房",
        "遣散費",
        "收費站",
        "信用額度",
        "店長",
        "保溫桶",
        "擔架",
        "重卡",
        "不成便罷",
        "信口胡謅",
        "賭波",
        "澡堂子",
        "瞎扯",
        "瞎折騰",
        "好勒",
        "警衛",
        "值班室",
        "病榻",
        "瓷磚",
        "車廂",
        "車輿",
        "明信片",
        "輪胎",
        "井蓋",
        "幫浦",
        "藍圖",
        "糊弄",
        "擦屁股",
        "窮光蛋",
        "赤貧窮漢",
        "清潔工",
        "淨街夫",
        "科研",
        "流體力學",
        "發條鐵青蛙",
        "剪板機",
        "下課",
        "歇功",
        "員工",
        "安歇",
        "回舍",
        "行腳客",
        "眼下",
        "早先",
        "前刻",
        "片刻",
        "半晌",
        "一時半刻",
        "良久",
        "名號",
        "活計方歇",
        "詞兒",
        "這地兒",
        "那地兒",
    ] {
        if s.contains(forbidden) || low.contains(forbidden) {
            return (false, "forbidden_term");
        }
    }
    if s.chars().any(|c| c.is_ascii_alphabetic()) {
        return (false, "ascii_alpha_residue");
    }
    if s.contains("---") || s.contains("```") || s.contains("\n\n") {
        return (false, "markdown_residue");
    }
    let n = s.chars().count();
    if n <= 2 {
        return (false, "too_short");
    }
    if n > max as usize {
        return (false, "too_long");
    }
    for b in gametext::quality_gate_bad_meta() {
        if !b.is_empty() && s.contains(b.as_str()) {
            return (false, "meta_text");
        }
    }
    let gopen: char = '\u{300c}';
    let gclose: char = '\u{300d}';
    if s.matches(gopen).count() > 2 || s.matches(gclose).count() > 2 {
        return (false, "nested_quotes");
    }
    if has_long_repeat(s) {
        return (false, "repetition");
    }
    if looks_like_npc_npc_narration_line(s) {
        return (false, "suspicious_narration");
    }
    (true, "")
}

fn looks_like_npc_npc_narration_line(s: &str) -> bool {
    let r: Vec<char> = s.chars().collect();
    if r.len() < 8 || !s.ends_with('。') {
        return false;
    }
    let hints = gametext::dialogue_markers();
    for ch in r {
        if hints.contains(ch) {
            return false;
        }
    }
    true
}

#[must_use]
pub fn should_skip_npc_npc_summary_update(
    old_summary: &str,
    new_summary: &str,
    line_a: &str,
    line_b: &str,
) -> bool {
    let old_summary = old_summary.trim();
    let new_summary = new_summary.trim();
    if old_summary.is_empty() {
        return false;
    }
    if new_summary == old_summary {
        return true;
    }
    if rune_lcs_similarity(old_summary, new_summary) >= 0.7 {
        return true;
    }
    let la = line_a.trim();
    let lb = line_b.trim();
    if !la.is_empty() && rune_lcs_similarity(old_summary, la) >= 0.7 {
        return true;
    }
    if !lb.is_empty() && rune_lcs_similarity(old_summary, lb) >= 0.7 {
        return true;
    }
    false
}

#[must_use]
pub fn dedupe_social_context_lines(lines: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let key = line.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        out.push(line.to_string());
    }
    out
}

fn has_long_repeat(s: &str) -> bool {
    let r: Vec<char> = s.chars().collect();
    if r.len() < 8 {
        return false;
    }
    let mut i = 0;
    while i + 3 < r.len() {
        let ch = r[i];
        if ch == ' ' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < r.len() && r[j] == ch {
            j += 1;
        }
        if j - i >= 4 {
            return true;
        }
        i += 1;
    }
    false
}

#[must_use]
pub fn merge_thread_anchors(old: &[String], fresh: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::<String>::new();
    let mut out = Vec::new();
    for v in old.iter().chain(fresh.iter()) {
        let v = v.trim();
        if v.is_empty() || !seen.insert(v.to_string()) {
            continue;
        }
        out.push(v.to_string());
    }
    if out.len() > 4 {
        out = out[out.len() - 4..].to_vec();
    }
    out
}

/// 錨與台詞是否矛盾（對齊既有 `anchorConsistencyCheck`）。
pub fn anchor_consistency_check(existing: &[String], line_a: &str, line_b: &str) -> (bool, String) {
    if existing.is_empty() {
        return (true, String::new());
    }
    let joined = format!("{} {}", line_a.trim(), line_b.trim())
        .trim()
        .to_string();
    if joined.is_empty() {
        return (true, String::new());
    }
    let negators = gametext::anchor_negators();
    for a in existing {
        let a = a.trim();
        if a.is_empty() {
            continue;
        }
        let runes: Vec<char> = a.chars().collect();
        for i in 0..runes.len().saturating_sub(1) {
            let k: String = runes[i..i + 2].iter().collect();
            if !joined.contains(&k) {
                continue;
            }
            for n in &negators {
                let nk = format!("{n}{k}");
                let kn = format!("{k}{n}");
                if joined.contains(&nk) || joined.contains(&kn) {
                    return (false, format!("anchor negated: {k}"));
                }
            }
        }
    }
    (true, String::new())
}

