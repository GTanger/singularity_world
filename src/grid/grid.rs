//! 方格地圖容器（SquareGrid）與格子單元（GridCell）。
//!
//! - `SquareGrid` 持有 world_seed 及所有已揭露格子
//! - `GridCell` 為格子單元，含 `Terrain` 與 `GridObject`
//! - 揭露邏輯由 `reveal` 模組負責，此模組只管容器

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::cell::{Freshness, GridObject, Terrain};

use super::coord::{Direction, SquareCoord};

// ─── 顯式出口 ───

/// 顯式出口：方向 + 目標座標。村落格用這個取代座標鄰接自動計算。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExplicitExit {
    pub dir: Direction,
    pub target: SquareCoord,
}

// ─── 格子單元 ───

/// 方格地圖單元（格上資源與地形）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCell {
    pub coord: SquareCoord,
    pub terrain: Terrain,
    pub name: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zone: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// 格子內的地上物（資源、地標）
    #[serde(default)]
    pub objects: Vec<GridObject>,

    /// 是否已被玩家探索（影響前端渲染）
    #[serde(default)]
    pub explored: bool,

    /// 探索新鮮度（連續探索同一格的資源豐厚度衰減）
    #[serde(default)]
    pub freshness: Freshness,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explicit_exits: Vec<ExplicitExit>,
}

impl GridCell {
    /// 取得格子的顯示符號：若為竹林／竹篁地貌則回傳 `𓐂`，否則回傳 `self.terrain.display_char()`
    pub fn display_symbol(&self) -> &'static str {
        if matches!(
            self.terrain,
            Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle
        ) && (self.has_room_tag("bamboo")
            || self.name.contains('竹')
            || self.name.contains('篁')
            || self.name.contains('筍'))
        {
            "𓐂"
        } else {
            self.terrain.display_char()
        }
    }
}

/// 判斷房間名稱是否為私人住宅、內室或宿舍（不應具備商業櫃檯或非本店人員禁止進入區）
pub fn is_private_residence_room_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains("火磚") || trimmed.contains("造紙") {
        return false;
    }
    if trimmed.contains("・內室")
        || trimmed.ends_with("內室")
        || trimmed.contains("臥房")
        || trimmed.contains("寢室")
        || trimmed.contains("宿所")
        || trimmed.contains("客寮")
        || trimmed.contains("工舍")
        || trimmed.contains("匠廬")
        || trimmed.contains("精廬")
        || trimmed.contains("山房")
        || trimmed.contains("小築")
    {
        return true;
    }
    let base = trimmed.rsplit('・').next().unwrap_or(trimmed).trim();
    const RESIDENCE_SUFFIXES: &[&str] = &[
        "宅邸", "宅院", "家宅", "宗宅", "老宅", "青石宅", "石宅", "民宅", "民居", "民舍",
        "民家", "家屋", "宿所", "宿舍", "四合院", "石砌院", "鐵匠院", "小院", "大院", "書齋",
        "庖廚居", "湯屋居", "藥圃居", "算學居", "木作居", "繡坊居", "臨水居", "鍛藝居",
        "武勇舍", "老水夫舍", "總管居", "先生舍", "校勘齋", "墨香居", "木舍",
        "遠行行商棧舍", "行道車馬行舍", "聯運公秤舍", "草料客商居", "草料商旅居",
        "皮靴油布匠居", "刻書雕版匠居", "星曆舍", "推步居", "鋪居", "坊舍", "工匠居",
        "圃居", "員宅", "家舍", "儲糧宅", "織造家", "工寮", "客寮", "匠舍", "人居", "人家",
        "工舍", "匠廬", "精廬", "山房", "小築",
    ];
    RESIDENCE_SUFFIXES.iter().any(|suf| base.ends_with(suf))
}

/// 判斷房間名稱是否為純公共街道、城門或街口（不應具備商業櫃檯或店鋪標籤）
pub fn is_street_or_gate_room_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    let base = trimmed.rsplit('・').next().unwrap_or(trimmed).trim();
    if matches!(base, "分家巷" | "西門街" | "講堂街" | "三水口") {
        return true;
    }
    const STREET_SUFFIXES: &[&str] = &[
        "北街", "南街", "東街", "西街", "前街", "大街", "商號街", "貨棧街", "石板街",
        "大道", "石道", "行商道", "車道", "坡道", "環北道", "環南道",
        "西巷", "東巷", "北巷", "南巷", "學院巷", "巷弄", "弄堂天井",
        "街口", "南口", "北口", "東口", "西口",
        "中段", "北段", "南段", "東段", "西段",
        "西北隅", "東北隅", "東南隅", "西南隅",
        "內門坊", "城門", "石徑", "石階", "平坪", "草坪", "轉角坪", "石坪", "花圃",
    ];
    STREET_SUFFIXES.iter().any(|suf| base.ends_with(suf))
}


#[derive(Debug, Clone, Default)]
pub struct SquareGrid {
    /// 揭露用 PRNG 種子（與座標混合；見 `reveal` 模組）
    world_seed: u64,
    cells: Arc<HashMap<SquareCoord, GridCell>>,
    /// 資料是否被修改（格數不變但內容變動時設為 true）
    dirty: bool,
}

impl GridCell {
    pub fn is_private_residence(&self) -> bool {
        self.terrain == Terrain::House
            || self.has_room_tag("residence")
            || is_private_residence_room_name(&self.name)
    }

    pub fn is_pure_street_or_courtyard(&self) -> bool {
        matches!(self.terrain, Terrain::Road | Terrain::Bridge)
            || is_street_or_gate_room_name(&self.name)
    }

    pub fn is_night_hospitality_or_watch_venue(&self) -> bool {
        self.terrain == Terrain::Inn
            || self.has_room_tag("inn")
            || self.has_room_tag("defense")
            || self.has_room_tag("bell_tower")
            || self.has_room_tag("lighthouse")
            || ["旅社", "客舍", "行館", "驛館", "通鋪", "歇舍", "哨", "巡防", "守更", "鐘樓", "燈塔"]
                .iter()
                .any(|k| self.name.contains(k))
    }

    pub fn is_evening_dining_or_bath_venue(&self) -> bool {
        self.terrain == Terrain::Tavern
            || self.has_room_tag("canteen")
            || self.has_room_tag("teahouse")
            || self.has_room_tag("bathhouse")
            || ["食堂", "茶", "飯", "麵", "浴", "湯"]
                .iter()
                .any(|k| self.name.contains(k))
    }

    pub fn is_open_at_hour(&self, hour: u32, is_night_duty_role: bool) -> bool {
        if (5..=19).contains(&hour) {
            true
        } else {
            self.is_private_residence()
                || self.is_night_hospitality_or_watch_venue()
                || (hour == 20 && self.is_evening_dining_or_bath_venue())
                || (is_night_duty_role && self.is_pure_street_or_courtyard())
        }
    }
}



impl SquareGrid {
    /// 依當前座標 `(cx, cy)` 由近而遠（曼哈頓距離環 `0..=max_radius`）搜尋具備任一指定語意標籤的房間格。
    /// 僅檢查顯式建築／城鎮格（`self.cells`），避免觸發荒野程序生成；同距離環內以 `seed` 決定性挑選。
    pub fn find_nearest_room_by_tags(
        &self,
        center: (i32, i32),
        max_radius: i32,
        required_tags: &[&str],
        seed: u64,
    ) -> Option<((i32, i32), String)> {
        if required_tags.is_empty() || max_radius < 0 {
            return None;
        }
        let allow_residence = required_tags
            .iter()
            .any(|t| matches!(*t, "residence" | "home" | "house"));
        let (cx, cy) = center;
        for dist in 0..=max_radius {
            let mut matches_in_ring: Vec<((i32, i32), String)> = Vec::new();
            if dist == 0 {
                let coord = SquareCoord::new(cx, cy);
                if let Some(cell) = self.cells.get(&coord)
                    && !cell.name.is_empty()
                    && !cell.name.contains("內室")
                    && (allow_residence || !cell.is_private_residence())
                    && required_tags.iter().any(|t| cell.has_room_tag(t))
                {
                    return Some(((cx, cy), cell.name.clone()));
                }
                continue;
            }
            for dx in -dist..=dist {
                let rem = dist - dx.abs();
                let dy_candidates = if rem == 0 { [0, 0] } else { [-rem, rem] };
                let count = if rem == 0 { 1 } else { 2 };
                for &dy in &dy_candidates[..count] {
                    let coord = SquareCoord::new(cx + dx, cy + dy);
                    if let Some(cell) = self.cells.get(&coord)
                        && !cell.name.is_empty()
                        && !cell.name.contains("內室")
                        && (allow_residence || !cell.is_private_residence())
                        && required_tags.iter().any(|t| cell.has_room_tag(t))
                    {
                        matches_in_ring.push(((coord.x, coord.y), cell.name.clone()));
                    }
                }
            }
            if !matches_in_ring.is_empty() {
                matches_in_ring.sort_by(|a, b| a.0.cmp(&b.0));
                let idx = (seed as usize) % matches_in_ring.len();
                return matches_in_ring.into_iter().nth(idx);
            }
        }
        None
    }

    /// 依中心座標查詢周邊核心機能地標，優先依據來客輸入抽取之兩字特徵詞篩選，再以生活機能分類補滿（至多 12 個）
    pub fn query_settlement_landmarks_from(
        &self,
        center: (i32, i32),
        max_radius: i32,
        query_hint: &str,
    ) -> Vec<((i32, i32), String)> {
        let (cx, cy) = center;
        const STOP_WORDS: &[&str] = &[
            "請問", "哪裡", "在哪", "怎麼", "如何", "地方", "這裡", "什麼", "過去", "走去",
        ];
        let mut cleaned = query_hint.to_string();
        for &stop in STOP_WORDS {
            cleaned = cleaned.replace(stop, " ");
        }
        let mut query_subs: Vec<String> = Vec::new();
        for segment in cleaned.split_whitespace() {
            let chars: Vec<char> = segment
                .chars()
                .filter(|c| {
                    !c.is_ascii_punctuation()
                        && !c.is_ascii_whitespace()
                        && *c != '（'
                        && *c != '）'
                        && *c != '？'
                        && *c != '！'
                        && *c != '，'
                        && *c != '。'
                })
                .collect();
            if chars.len() >= 2 {
                for i in 0..chars.len() - 1 {
                    let sub: String = [chars[i], chars[i + 1]].iter().collect();
                    if !query_subs.contains(&sub) {
                        query_subs.push(sub);
                    }
                }
            }
        }

        let mut candidates: Vec<&GridCell> = self
            .cells
            .values()
            .filter(|cell| {
                let dx = cell.coord.x - cx;
                let dy = cell.coord.y - cy;
                dx.abs() <= max_radius
                    && dy.abs() <= max_radius
                    && (dx != 0 || dy != 0)
                    && !cell.name.is_empty()
                    && !cell.is_pure_street_or_courtyard()
                    && !cell.is_private_residence()
            })
            .collect();

        candidates.sort_by_key(|c| {
            let dist = (cx - c.coord.x).abs() + (cy - c.coord.y).abs();
            (dist, c.coord.x, c.coord.y)
        });

        let mut results: Vec<((i32, i32), String)> = Vec::new();

        if !query_subs.is_empty() {
            for cell in &candidates {
                if results.len() >= 6 {
                    break;
                }
                if query_subs.iter().any(|sub| cell.name.contains(sub)) {
                    let item = ((cell.coord.x, cell.coord.y), cell.name.clone());
                    if !results.contains(&item) {
                        results.push(item);
                    }
                }
            }
        }

        const CORE_CATEGORIES: &[&[&str]] = &[
            &["inn"],
            &["canteen", "teahouse"],
            &["bathhouse"],
            &["clinic", "medicine"],
            &["bank", "shop", "market"],
            &["blacksmith", "workshop"],
            &["academy"],
            &["civic"],
            &["courier", "dock"],
        ];

        for &category in CORE_CATEGORIES {
            if results.len() >= 12 {
                break;
            }
            if let Some(cell) = candidates.iter().find(|c| {
                category.iter().any(|&tag| c.has_room_tag(tag))
                    && !results.iter().any(|(pos, _)| *pos == (c.coord.x, c.coord.y))
            }) {
                results.push(((cell.coord.x, cell.coord.y), cell.name.clone()));
            }
        }

        results
    }
}


include!("grid/tags.rs");
include!("grid/venue.rs");
include!("grid/venue_p1.rs");
include!("grid/venue_p2.rs");
include!("grid/market_venue.rs");
include!("grid/square_grid.rs");
include!("grid/tests.rs");
