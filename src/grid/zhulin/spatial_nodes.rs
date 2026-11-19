//! 沙鹿竹林與西麓街鎮之空間節點拓撲投影（Route B 空間圖層）。
//!
//! 將底層 `arcade_street` 門牆連通規則、`hydrology` 漲水阻隔與 `social` 320 活人動態，
//! 投影為具真實名稱、空間類型（街道／店面／住宅／林區／水脈）、區域群組與明確出入口連線的
//! 空間節點拓撲圖（`SpatialGraphView`），嚴禁外露原始座標或內部代碼。

use std::collections::{HashMap, HashSet, VecDeque};

use serde::Serialize;

use crate::grid::substrate::arcade_street::{available_floor_exits, can_step_horizontal};
use crate::grid::substrate::facade::CellFacade;
use crate::grid::substrate::hydrology::WorldClockSnapshot;
use crate::store::sql::DbPool;

/// 單一空間節點語義中繼資料（玩家可見，完全去座標與去內部代碼）。
#[derive(Debug, Clone, Serialize)]
pub struct SpatialNodeMeta {
    pub id: String,
    pub name: String,
    pub short_sub: String,
    /// `street` | `shop` | `residence` | `forest` | `water`
    pub kind: &'static str,
    pub kind_zh: &'static str,
    pub region_id: &'static str,
    pub region_name: &'static str,
    /// `public` | `semi_private` | `private`
    pub privacy: &'static str,
    pub privacy_zh: &'static str,
}

/// 當前空間的單一直連出入口。
#[derive(Debug, Clone, Serialize)]
pub struct SpatialExitView {
    pub target_id: String,
    pub target_name: String,
    pub target_kind: &'static str,
    pub target_kind_zh: &'static str,
    pub dir_label: String,
    /// 對應 WebSocket `move` 的 `direction` 字串（`東`/`西`/`南`/`北`/`進`/`出`/`上`/`下`）。
    pub direction: &'static str,
    /// `step` | `floor`
    pub move_kind: &'static str,
    pub target_gx: i32,
    pub target_gy: i32,
    pub target_gz: i16,
    pub pop_count: u16,
    pub live_badge: String,
    pub needs_knock: bool,
}

/// 拓撲圖中的節點（含相對佈局座標供 SVG 無重疊繪製）。
#[derive(Debug, Clone, Serialize)]
pub struct TopoNodeView {
    pub id: String,
    pub name: String,
    pub short_sub: String,
    pub kind: &'static str,
    pub kind_zh: &'static str,
    pub region_id: &'static str,
    pub region_name: &'static str,
    pub gx: i32,
    pub gy: i32,
    pub gz: i16,
    /// 以當前節點為 `(0.0, 0.0)` 的相對視覺座標（東正西負、北負南正）。
    pub rx: f32,
    pub ry: f32,
    pub is_current: bool,
    pub is_direct_exit: bool,
    pub direction: String,
    pub move_kind: &'static str,
    pub pop_count: u16,
    pub live_badge: String,
    pub needs_knock: bool,
}

/// 拓撲圖中的出入口連線。
#[derive(Debug, Clone, Serialize)]
pub struct TopoEdgeView {
    pub from_id: String,
    pub to_id: String,
    pub label: String,
    pub is_direct: bool,
    pub dashed: bool,
}

/// 玩家視角近處街坊風聞（已將座標轉譯為真實空間節點名）。
#[derive(Debug, Clone, Serialize)]
pub struct SpatialEchoView {
    pub space_name: String,
    pub summary: String,
    pub hot: bool,
}

/// 完整空間節點拓撲視圖（附加於 `room_enter` 的 `spatial_graph` 欄位）。
#[derive(Debug, Clone, Serialize)]
pub struct SpatialGraphView {
    pub current: SpatialNodeMeta,
    pub exits: Vec<SpatialExitView>,
    pub nodes: Vec<TopoNodeView>,
    pub edges: Vec<TopoEdgeView>,
    pub echoes: Vec<SpatialEchoView>,
}

pub fn node_id(gx: i32, gy: i32, gz: i16) -> String {
    format!("sp_{gx}_{gy}_{gz}")
}

/// 依空間座標與樓層推導具名空間節點（零寫死主角 NPC，自然呈現街／店／宅／林／水脈）。
pub fn resolve_spatial_node_meta(
    gx: i32,
    gy: i32,
    gz: i16,
    fallback_name: &str,
) -> SpatialNodeMeta {
    let id = node_id(gx, gy, gz);

    // 1. 西麓主石板街（gy == 0, gx = -6..=-1）
    if gy == 0 && (-6..=-1).contains(&gx) && gz == 0 {
        let (name, sub) = match gx {
            -6 => ("正街西口", "街市西端"),
            -5 => ("正街西段", "秤台前街"),
            -4 => ("正街中西段", "鍛坊街口"),
            -3 => ("正街中段", "鬧市正街"),
            -2 => ("正街東段", "茶寮街口"),
            _ => ("街東石坪", "出街林口"),
        };
        return SpatialNodeMeta {
            id,
            name: name.to_string(),
            short_sub: sub.to_string(),
            kind: "street",
            kind_zh: "街道",
            region_id: "west_foothill_town",
            region_name: "西麓街市",
            privacy: "public",
            privacy_zh: "公開街面・旁人可聽見",
        };
    }

    // 2. 北排騎樓街屋（gy == 1, gx = -6..=-2）
    if gy == 1 && (-6..=-2).contains(&gx) {
        let base_shop = match gx {
            -6 => ("驗晶秤鋪", "秤貨估價"),
            -5 => ("青鹽雜貨行", "日用採買"),
            -4 => ("老陳鍛鐵坊", "爐火校刃"),
            -3 => ("鮮筍熱食灶攤", "熱湯早食"),
            _ => ("東口茶寮", "歇腳聽風"),
        };
        return shophouse_meta_for_floor(id, base_shop.0, base_shop.1, gz, false);
    }

    // 3. 南排騎樓街屋（gy == -1, gx = -6..=-2）
    if gy == -1 && (-6..=-2).contains(&gx) {
        let base_shop = match gx {
            -6 => ("山貨收購行", "收貨清帳"),
            -5 => ("竹器編織坊", "削篾編簍"),
            -4 => ("南排乾貨鋪", "乾筍鹽漬"),
            -3 => ("麻繩修補坊", "繕具補囊"),
            _ => ("南角涼茶鋪", "解渴閒談"),
        };
        return shophouse_meta_for_floor(id, base_shop.0, base_shop.1, gz, false);
    }

    // 4. 南背巷住宅街屋（gy == -2, gx = -5..=-3）
    if gy == -2 && (-5..=-3).contains(&gx) {
        let base_home = match gx {
            -5 => ("後巷竹舍", "街坊民宅"),
            -4 => ("靜巷民居", "內院住家"),
            _ => ("巷尾小院", "清幽宅院"),
        };
        return shophouse_meta_for_floor(id, base_home.0, base_home.1, gz, true);
    }

    // 5. 南背石巷（gy == -3, gx = -6..=-2）
    if gy == -3 && (-6..=-2).contains(&gx) && gz == 0 {
        let (name, sub) = match gx {
            -6 => ("南巷西角", "背街轉角"),
            -5 => ("南背石巷西", "安靜後巷"),
            -4 => ("南背石巷", "民宅巷道"),
            -3 => ("南背石巷東", "巷尾靜道"),
            _ => ("南巷東口", "通往林坡"),
        };
        return SpatialNodeMeta {
            id,
            name: name.to_string(),
            short_sub: sub.to_string(),
            kind: "street",
            kind_zh: "街道",
            region_id: "west_foothill_town",
            region_name: "西麓街市",
            privacy: "public",
            privacy_zh: "靜僻後巷・少人經過",
        };
    }

    // 6. 四大原始自然地標錨點與竹林區域
    match (gx, gy) {
        (0, 0) => SpatialNodeMeta {
            id,
            name: "伏流卵灘".to_string(),
            short_sub: "水岸淺灘".to_string(),
            kind: "water",
            kind_zh: "水脈",
            region_id: "spring_bank",
            region_name: "伏流溪岸",
            privacy: "public",
            privacy_zh: "開闊水岸",
        },
        (0, 1) => SpatialNodeMeta {
            id,
            name: "東口刺竹林".to_string(),
            short_sub: "林緣入口".to_string(),
            kind: "forest",
            kind_zh: "林區",
            region_id: "zhulin_grove",
            region_name: "刺竹林區",
            privacy: "public",
            privacy_zh: "竹影稀疏",
        },
        (1, 0) => SpatialNodeMeta {
            id,
            name: "伏流冷泉".to_string(),
            short_sub: "湧泉清涼".to_string(),
            kind: "water",
            kind_zh: "水脈",
            region_id: "spring_bank",
            region_name: "伏流溪岸",
            privacy: "public",
            privacy_zh: "泉聲清幽",
        },
        (1, -1) => SpatialNodeMeta {
            id,
            name: "古樟林坡".to_string(),
            short_sub: "樟蔭緩坡".to_string(),
            kind: "forest",
            kind_zh: "林區",
            region_id: "zhulin_grove",
            region_name: "刺竹林區",
            privacy: "public",
            privacy_zh: "林坡清幽",
        },
        _ => {
            let clean_fallback = fallback_name.trim();
            let is_bamboo = clean_fallback.contains("竹")
                || (gx >= 0 && (-2..=3).contains(&gy));
            let is_water = clean_fallback.contains("泉")
                || clean_fallback.contains("溪")
                || clean_fallback.contains("灘");
            if is_water {
                let name = if clean_fallback.is_empty() || clean_fallback == "伏流卵石灘" {
                    "伏流淺溪"
                } else {
                    clean_fallback
                };
                SpatialNodeMeta {
                    id,
                    name: name.to_string(),
                    short_sub: "溪岸水路".to_string(),
                    kind: "water",
                    kind_zh: "水脈",
                    region_id: "spring_bank",
                    region_name: "伏流溪岸",
                    privacy: "public",
                    privacy_zh: "開闊溪岸",
                }
            } else if is_bamboo {
                let name = match (gx, gy) {
                    (1, 1) => "竹林深處",
                    (0, 2) | (1, 2) => "竹林北徑",
                    (2, 0) | (2, 1) => "高坡竹林",
                    (0, -1) => "竹林南緣",
                    _ if !clean_fallback.is_empty() && clean_fallback != "刺竹林" => clean_fallback,
                    _ if gy > 0 => "北坡竹徑",
                    _ if gy < 0 => "南坡竹徑",
                    _ => "刺竹林間",
                };
                SpatialNodeMeta {
                    id,
                    name: name.to_string(),
                    short_sub: "竹影幽徑".to_string(),
                    kind: "forest",
                    kind_zh: "林區",
                    region_id: "zhulin_grove",
                    region_name: "刺竹林區",
                    privacy: "public",
                    privacy_zh: "竹林幽徑",
                }
            } else {
                let name = if clean_fallback.is_empty() {
                    "西麓坡道"
                } else {
                    clean_fallback
                };
                SpatialNodeMeta {
                    id,
                    name: name.to_string(),
                    short_sub: "城郊外緣".to_string(),
                    kind: "street",
                    kind_zh: "坡道",
                    region_id: "west_slope",
                    region_name: "大肚西坡",
                    privacy: "public",
                    privacy_zh: "開闊坡地",
                }
            }
        }
    }
}

fn shophouse_meta_for_floor(
    id: String,
    base_name: &str,
    sub: &str,
    gz: i16,
    is_residence_plot: bool,
) -> SpatialNodeMeta {
    match gz {
        0 => SpatialNodeMeta {
            id,
            name: base_name.to_string(),
            short_sub: if is_residence_plot {
                "門廊前院".to_string()
            } else {
                sub.to_string()
            },
            kind: if is_residence_plot { "residence" } else { "shop" },
            kind_zh: if is_residence_plot { "住宅" } else { "店面" },
            region_id: "west_foothill_town",
            region_name: "西麓街市",
            privacy: "public",
            privacy_zh: "騎樓前堂・旁人可聽見",
        },
        1 => SpatialNodeMeta {
            id,
            name: format!("{base_name}・內室"),
            short_sub: if is_residence_plot {
                "廳堂內屋".to_string()
            } else {
                "店內後室".to_string()
            },
            kind: if is_residence_plot { "residence" } else { "shop" },
            kind_zh: if is_residence_plot { "住宅" } else { "內室" },
            region_id: "west_foothill_town",
            region_name: "西麓街市",
            privacy: "semi_private",
            privacy_zh: "室內空間・隔絕街聲",
        },
        2 => SpatialNodeMeta {
            id,
            name: format!("{base_name}・二樓寓所"),
            short_sub: "二樓住處".to_string(),
            kind: "residence",
            kind_zh: "住宅",
            region_id: "west_foothill_town",
            region_name: "西麓街市",
            privacy: "private",
            privacy_zh: "私密寓所・外人無從旁聽",
        },
        _ => SpatialNodeMeta {
            id,
            name: format!("{base_name}・屋頂露台"),
            short_sub: "高處平台".to_string(),
            kind: "residence",
            kind_zh: "露台",
            region_id: "west_foothill_town",
            region_name: "西麓街市",
            privacy: "private",
            privacy_zh: "高處露台・清風無人擾",
        },
    }
}

/// 依當前座標、樓層、即時水文與活態社會建立完整空間節點拓撲視圖。
pub fn build_spatial_graph_view(
    pool: Option<&DbPool>,
    gx: i32,
    gy: i32,
    gz: i16,
    live_clock: WorldClockSnapshot,
    fallback_name: &str,
) -> SpatialGraphView {
    let current = resolve_spatial_node_meta(gx, gy, gz, fallback_name);
    let pop_counts = super::social::ground_pop_counts();
    let live_badges = super::social::spatial_node_live_badges();
    let has_substrate = crate::grid::substrate::runtime::global().is_some();

    let mut exits: Vec<SpatialExitView> = Vec::new();
    let mut nodes: Vec<TopoNodeView> = Vec::new();
    let mut edges: Vec<TopoEdgeView> = Vec::new();
    let mut seen_nodes: HashSet<(i32, i32, i16)> = HashSet::new();
    let mut seen_edges: HashSet<(String, String)> = HashSet::new();

    // 1. 加入當前中心節點
    seen_nodes.insert((gx, gy, gz));
    let cur_pop = if gz == 0 {
        pop_counts.get(&(gx, gy)).copied().unwrap_or(0)
    } else {
        super::social::pop_count_at_exact(gx, gy, gz)
    };
    let cur_badge = live_badges.get(&(gx, gy)).cloned().unwrap_or_default();
    nodes.push(TopoNodeView {
        id: current.id.clone(),
        name: current.name.clone(),
        short_sub: current.short_sub.clone(),
        kind: current.kind,
        kind_zh: current.kind_zh,
        region_id: current.region_id,
        region_name: current.region_name,
        gx,
        gy,
        gz,
        rx: 0.0,
        ry: 0.0,
        is_current: true,
        is_direct_exit: false,
        direction: String::new(),
        move_kind: "stay",
        pop_count: cur_pop,
        live_badge: cur_badge,
        needs_knock: false,
    });

    if !has_substrate {
        return SpatialGraphView {
            current,
            exits,
            nodes,
            edges,
            echoes: build_spatial_echoes(),
        };
    }

    let cur_snap = CellFacade::snapshot_live_clock(pool, gx, gy, live_clock);

    // 2. 垂直／室內出入口（進、出、上、下，嚴格對齊 arcade_street::resolve_floor_move）
    let floor_dirs = available_floor_exits(&cur_snap, gz);
    for fdir in &floor_dirs {
        let target_gz = match (gz, *fdir) {
            (0, "進") => 1,
            (1, "出") => 0,
            (0, "上") => 2,
            (2, "上") => 3,
            (2, "下") => 0,
            (3, "下") => 2,
            _ => continue,
        };
        let target_meta = resolve_spatial_node_meta(gx, gy, target_gz, cur_snap.synth.micro_name);
        let dir_label = match *fdir {
            "進" => "進內室",
            "出" => "步出前堂",
            "上" => "上樓",
            "下" => "下樓",
            _ => *fdir,
        };
        let t_pop = super::social::pop_count_at_exact(gx, gy, target_gz);
        let needs_knock = target_gz >= 2;
        exits.push(SpatialExitView {
            target_id: target_meta.id.clone(),
            target_name: target_meta.name.clone(),
            target_kind: target_meta.kind,
            target_kind_zh: target_meta.kind_zh,
            dir_label: dir_label.to_string(),
            direction: fdir,
            move_kind: "floor",
            target_gx: gx,
            target_gy: gy,
            target_gz,
            pop_count: t_pop,
            live_badge: String::new(),
            needs_knock,
        });

        // 垂直相連節點置於拓撲圖上方／右上方偏角，確保與「上樓」及平面四向零重疊
        let (rx, ry) = match *fdir {
            "進" => (0.98, -0.82),
            "上" => (0.0, -1.15),
            "出" => (-0.98, 0.82),
            "下" => (0.0, 1.15),
            _ => (0.98, -0.82),
        };
        if seen_nodes.insert((gx, gy, target_gz)) {
            nodes.push(TopoNodeView {
                id: target_meta.id.clone(),
                name: target_meta.name.clone(),
                short_sub: target_meta.short_sub.clone(),
                kind: target_meta.kind,
                kind_zh: target_meta.kind_zh,
                region_id: target_meta.region_id,
                region_name: target_meta.region_name,
                gx,
                gy,
                gz: target_gz,
                rx,
                ry,
                is_current: false,
                is_direct_exit: true,
                direction: (*fdir).to_string(),
                move_kind: "floor",
                pop_count: t_pop,
                live_badge: String::new(),
                needs_knock,
            });
        }
        push_unique_edge(
            &mut edges,
            &mut seen_edges,
            &current.id,
            &target_meta.id,
            dir_label,
            true,
            needs_knock || target_gz > 0,
        );
    }

    // 3. 平面出入口與 2 跳連通拓撲 BFS（僅在 z == 0 層展開平面連通，室內 z > 0 聚焦本棟出入口）
    if gz == 0 {
        const DIRS: [(i32, i32, &str); 4] = [
            (0, 1, "北"),
            (1, 0, "東"),
            (0, -1, "南"),
            (-1, 0, "西"),
        ];
        let mut snap_cache: HashMap<(i32, i32), crate::grid::substrate::facade::CellSnapshot> =
            HashMap::new();
        snap_cache.insert((gx, gy), cur_snap);

        let mut queue: VecDeque<(i32, i32, u8)> = VecDeque::new();
        queue.push_back((gx, gy, 0));

        while let Some((cx, cy, depth)) = queue.pop_front() {
            if depth >= 2 || nodes.len() >= 14 {
                continue;
            }
            let from_snap = snap_cache
                .entry((cx, cy))
                .or_insert_with(|| CellFacade::snapshot_live_clock(pool, cx, cy, live_clock))
                .clone();
            let from_id = node_id(cx, cy, 0);

            for &(dx, dy, dir_str) in &DIRS {
                let nx = cx + dx;
                let ny = cy + dy;
                // 限制視覺半徑在 2 格內，確保拓撲佈局簡潔無雜亂重疊
                if (nx - gx).abs() > 2 || (ny - gy).abs() > 2 {
                    continue;
                }
                let to_snap = snap_cache
                    .entry((nx, ny))
                    .or_insert_with(|| CellFacade::snapshot_live_clock(pool, nx, ny, live_clock))
                    .clone();
                if !can_step_horizontal(&from_snap, &to_snap) {
                    continue;
                }
                let to_meta = resolve_spatial_node_meta(nx, ny, 0, to_snap.synth.micro_name);
                let pop = pop_counts.get(&(nx, ny)).copied().unwrap_or(0);
                let badge = live_badges.get(&(nx, ny)).cloned().unwrap_or_default();
                let is_direct = depth == 0;

                if is_direct {
                    let dir_label = if to_meta.kind == "shop" {
                        format!("{dir_str}門")
                    } else if to_meta.kind == "residence" {
                        format!("{dir_str}宅")
                    } else {
                        dir_str.to_string()
                    };
                    exits.push(SpatialExitView {
                        target_id: to_meta.id.clone(),
                        target_name: to_meta.name.clone(),
                        target_kind: to_meta.kind,
                        target_kind_zh: to_meta.kind_zh,
                        dir_label,
                        direction: dir_str,
                        move_kind: "step",
                        target_gx: nx,
                        target_gy: ny,
                        target_gz: 0,
                        pop_count: pop,
                        live_badge: badge.clone(),
                        needs_knock: false,
                    });
                }

                if seen_nodes.insert((nx, ny, 0)) && nodes.len() < 14 {
                    nodes.push(TopoNodeView {
                        id: to_meta.id.clone(),
                        name: to_meta.name.clone(),
                        short_sub: to_meta.short_sub.clone(),
                        kind: to_meta.kind,
                        kind_zh: to_meta.kind_zh,
                        region_id: to_meta.region_id,
                        region_name: to_meta.region_name,
                        gx: nx,
                        gy: ny,
                        gz: 0,
                        rx: (nx - gx) as f32,
                        // SVG 螢幕 y 軸向下，故北(ny > gy) 為負值
                        ry: -(ny - gy) as f32,
                        is_current: false,
                        is_direct_exit: is_direct,
                        direction: if is_direct {
                            dir_str.to_string()
                        } else {
                            String::new()
                        },
                        move_kind: "step",
                        pop_count: pop,
                        live_badge: badge,
                        needs_knock: false,
                    });
                    if depth + 1 < 2 {
                        queue.push_back((nx, ny, depth + 1));
                    }
                }

                if seen_nodes.contains(&(nx, ny, 0)) {
                    push_unique_edge(
                        &mut edges,
                        &mut seen_edges,
                        &from_id,
                        &to_meta.id,
                        if is_direct { dir_str } else { "" },
                        is_direct,
                        to_meta.kind == "residence",
                    );
                }
            }
        }
    }

    SpatialGraphView {
        current,
        exits,
        nodes,
        edges,
        echoes: build_spatial_echoes(),
    }
}

fn push_unique_edge(
    edges: &mut Vec<TopoEdgeView>,
    seen: &mut HashSet<(String, String)>,
    a: &str,
    b: &str,
    label: &str,
    is_direct: bool,
    dashed: bool,
) {
    let key = if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    };
    if seen.insert(key) {
        edges.push(TopoEdgeView {
            from_id: a.to_string(),
            to_id: b.to_string(),
            label: label.to_string(),
            is_direct,
            dashed,
        });
    }
}

fn build_spatial_echoes() -> Vec<SpatialEchoView> {
    super::social::recent_street_echoes(4)
        .into_iter()
        .map(|e| {
            let meta = resolve_spatial_node_meta(e.gx, e.gy, 0, "");
            let hot = e.summary.contains("口角")
                || e.summary.contains("爭")
                || e.summary.contains("欠")
                || e.summary.contains("債");
            SpatialEchoView {
                space_name: meta.name,
                summary: e.summary,
                hot,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_node_names_cover_street_shop_residence_and_forest() {
        let street = resolve_spatial_node_meta(-3, 0, 0, "");
        assert_eq!(street.name, "正街中段");
        assert_eq!(street.kind, "street");

        let shop = resolve_spatial_node_meta(-3, 1, 0, "");
        assert_eq!(shop.kind, "shop");

        let upstairs = resolve_spatial_node_meta(-3, 1, 2, "");
        assert_eq!(upstairs.kind, "residence");
        assert_eq!(upstairs.privacy, "private");

        let back_home = resolve_spatial_node_meta(-4, -2, 0, "");
        assert_eq!(back_home.kind, "residence");

        let grove = resolve_spatial_node_meta(0, 1, 0, "");
        assert_eq!(grove.kind, "forest");
        assert_eq!(grove.region_name, "刺竹林區");
    }
}
