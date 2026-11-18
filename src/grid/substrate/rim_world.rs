//! 輕量版 RimWorld 無縫雙尺度空間底板（`20m` 母格 ↔ `20×20` 之 `1m` 施工細格）。
//!
//! 核心契約：
//! 1. 遠景直接對應 `SubstrateRuntime` 之 `20m` 真實高程水系母格 `(gx, gy)`；
//! 2. 近景將每 1 個母格細分為 `20 × 20` 個 `1m` 細格 `(mx, my)`（`gx = mx.div_euclid(20)`）；
//! 3. 未變動細格 100% 由母格 `L0Cell` 水態、母岩與確定性雜湊即時推導（0 記憶體／0 資料庫列）；
//! 4. 僅被砍伐、鋪設藍圖、建成結構或掉落物資之細格存入稀疏 `HashMap`，並支援 `1m` 有界加權 A* 繞障尋路。

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use serde::{Deserialize, Serialize};

use super::l0_cell::{L0Cell, SubstrateMaterial, SurfaceWaterKind, landmass};
use super::runtime::{SubstrateRuntime, WORLD_SEED};

/// 每個 `20m` 母格沿單軸細分之 `1m` 細格數（`20m / 20 = 1.0m`）。
pub const SUBCELLS_PER_MACRO: i32 = 20;
/// 單次 `1m` A* 尋路最大展開節點上限（防無路可通時卡死）。
pub const MICRO_ASTAR_MAX_NODES: usize = 16_000;

/// 將 `2m` 細格座標轉為 `(母格座標, 0..10 細格偏移)`。
#[inline]
pub fn micro_to_macro(mx: i32, my: i32) -> ((i32, i32), (i32, i32)) {
    (
        (
            mx.div_euclid(SUBCELLS_PER_MACRO),
            my.div_euclid(SUBCELLS_PER_MACRO),
        ),
        (
            mx.rem_euclid(SUBCELLS_PER_MACRO),
            my.rem_euclid(SUBCELLS_PER_MACRO),
        ),
    )
}

#[inline]
fn hash_micro(mx: i32, my: i32, salt: u64) -> u64 {
    let mut z = WORLD_SEED
        ^ ((mx as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        ^ ((my as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        ^ salt;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// `2m` 細格底層地形分類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicroGround {
    LoamGrass,
    RedGravel,
    PebbleBank,
    ShallowStream,
    SpringPool,
    DeepWater,
    SteepCliff,
}

impl MicroGround {
    pub fn is_water(self) -> bool {
        matches!(
            self,
            Self::ShallowStream | Self::SpringPool | Self::DeepWater
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::LoamGrass => "沃土草坪",
            Self::RedGravel => "紅土礫地",
            Self::PebbleBank => "溪畔卵石灘",
            Self::ShallowStream => "清淺山澗",
            Self::SpringPool => "斷層冷泉潭",
            Self::DeepWater => "湍急深水",
            Self::SteepCliff => "陡峭岩壁",
        }
    }
}

/// 未開發細格上自然生長之可互動實體。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NaturalProp {
    BambooClump,
    BambooShoot,
    RiverStone,
}

impl NaturalProp {
    pub fn blocks_movement(self) -> bool {
        matches!(self, Self::BambooClump)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::BambooClump => "濃密莿竹叢",
            Self::BambooShoot => "鮮嫩春筍",
            Self::RiverStone => "河灘硬石",
        }
    }

    pub fn tags(self) -> &'static [&'static str] {
        match self {
            Self::BambooClump => &["擋路", "可砍伐得竹材"],
            Self::BambooShoot => &["可採集", "山珍"],
            Self::RiverStone => &["可拾取", "石材"],
        }
    }
}

/// 地上散落或儲物區堆放之實體物資（100% 物質守恆，禁憑空生料）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RimItemKind {
    BambooPole,
    RiverStone,
    BambooShoot,
    BeastMeat,
    RoastedShoot,
    RoastedMeat,
    BambooBlade,
    StonePick,
    BambooHat,
    BambooPack,
}

impl RimItemKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::BambooPole => "莿竹稈",
            Self::RiverStone => "硬石",
            Self::BambooShoot => "鮮筍",
            Self::BeastMeat => "山獸骨肉",
            Self::RoastedShoot => "煨冬筍",
            Self::RoastedMeat => "炙獸肉",
            Self::BambooBlade => "烤刃竹刀",
            Self::StonePick => "石刃木鎬",
            Self::BambooHat => "細編竹笠",
            Self::BambooPack => "竹編背簍",
        }
    }

    pub fn short_char(self) -> &'static str {
        match self {
            Self::BambooPole => "竹",
            Self::RiverStone => "石",
            Self::BambooShoot => "筍",
            Self::BeastMeat => "肉",
            Self::RoastedShoot => "煨",
            Self::RoastedMeat => "炙",
            Self::BambooBlade => "刀",
            Self::StonePick => "鎬",
            Self::BambooHat => "笠",
            Self::BambooPack => "簍",
        }
    }

    pub fn is_hand_tool(self) -> bool {
        matches!(self, Self::BambooBlade | Self::StonePick)
    }

    pub fn is_edible(self) -> bool {
        matches!(
            self,
            Self::RoastedShoot | Self::RoastedMeat | Self::BambooShoot
        )
    }

    pub fn is_wearable(self) -> bool {
        matches!(self, Self::BambooHat | Self::BambooPack)
    }

    pub fn base_magnesium_price(self) -> u32 {
        match self {
            Self::BambooPole | Self::RiverStone => 1,
            Self::BambooShoot => 2,
            Self::BeastMeat => 3,
            Self::RoastedShoot | Self::BambooHat => 4,
            Self::BambooBlade => 5,
            Self::RoastedMeat | Self::StonePick | Self::BambooPack => 6,
        }
    }
}

/// 細格上建成之人工設施。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructureKind {
    BambooWall,
    BambooDoor,
    WoodBridge,
    Campfire,
    Workbench,
    Stockpile,
}

impl StructureKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::BambooWall => "竹木圍牆",
            Self::BambooDoor => "竹編院門",
            Self::WoodBridge => "渡溪窄橋",
            Self::Campfire => "竹葉營火",
            Self::Workbench => "百工工作台",
            Self::Stockpile => "儲物地墊",
        }
    }

    pub fn bamboo_cost(self) -> u8 {
        match self {
            Self::Stockpile => 0,
            Self::Workbench => 2,
            Self::BambooWall | Self::BambooDoor | Self::WoodBridge | Self::Campfire => 1,
        }
    }

    pub fn work_ticks_needed(self) -> u8 {
        match self {
            Self::Stockpile => 1,
            Self::Campfire => 3,
            Self::WoodBridge | Self::BambooWall | Self::BambooDoor => 4,
            Self::Workbench => 6,
        }
    }

    pub fn blocks_actor(self, is_beast: bool) -> bool {
        match self {
            Self::BambooWall | Self::Workbench | Self::Campfire => true,
            Self::BambooDoor => is_beast,
            Self::WoodBridge | Self::Stockpile => false,
        }
    }
}

/// 玩家或同伴掛載於細格上之施工／採集藍圖。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BlueprintJob {
    Chop {
        progress: u8,
    },
    Build {
        structure: StructureKind,
        delivered_bamboo: u8,
        progress: u8,
    },
    Deconstruct {
        progress: u8,
    },
}

/// 地上單格堆疊物資。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundStack {
    pub item: RimItemKind,
    pub count: u16,
}

/// 稀疏覆寫表單格狀態（未變動格完全不存入表內）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroCellOverlay {
    pub cleared_natural: bool,
    pub structure: Option<StructureKind>,
    pub blueprint: Option<BlueprintJob>,
    pub ground_item: Option<GroundStack>,
}

impl MicroCellOverlay {
    pub fn is_pristine(&self) -> bool {
        !self.cleared_natural
            && self.structure.is_none()
            && self.blueprint.is_none()
            && self.ground_item.is_none()
    }
}

/// 推導任意 `(mx, my)` 之原生細格地貌與自然物。
pub fn derive_micro_pristine(
    rt: Option<&SubstrateRuntime>,
    mx: i32,
    my: i32,
) -> (MicroGround, Option<NaturalProp>) {
    let ((gx, gy), (sx, sy)) = micro_to_macro(mx, my);
    let l0: Option<L0Cell> = rt.and_then(|r| r.l0(gx, gy));
    let steep = rt
        .and_then(|r| r.slope(gx, gy))
        .is_some_and(|s| s.grade_ratio >= 0.85);

    if steep {
        return (MicroGround::SteepCliff, None);
    }

    if let Some(cell) = l0 {
        if cell.landmass_id == landmass::SEA {
            return (MicroGround::DeepWater, None);
        }
        match cell.surface_water() {
            SurfaceWaterKind::OpenSea
            | SurfaceWaterKind::IntertidalShallow
            | SurfaceWaterKind::DeepRapidRiver => {
                return (MicroGround::DeepWater, None);
            }
            SurfaceWaterKind::FaultSpring => {
                let dx = sx - 10;
                let dy = sy - 10;
                if dx * dx + dy * dy <= 36 {
                    return (MicroGround::SpringPool, None);
                }
                if (7..=13).contains(&sx) && sy <= 10 {
                    return (MicroGround::ShallowStream, None);
                }
                return (MicroGround::PebbleBank, None);
            }
            SurfaceWaterKind::PerennialStream => {
                let wave = ((my as f32 * 0.18).sin() * 3.5).round() as i32;
                let center = (10 + wave).clamp(4, 15);
                if (sx - center).abs() <= 2 {
                    return (MicroGround::ShallowStream, None);
                }
                if (sx - center).abs() <= 4 {
                    return (MicroGround::PebbleBank, None);
                }
            }
            _ => {}
        }
    }

    // 原點北側湧泉引出之連續蜿蜒淺澗（東西貫穿山麓帶，寬 4m［4 細格］水密無斜縫，架 4 格窄木橋即可渡溪）
    let spring_stream_y = 11 + (((mx as f32) * 0.11).sin() * 2.4).round() as i32;
    if (-1200..=1200).contains(&mx) && (-1..=2).contains(&(my - spring_stream_y)) {
        return (MicroGround::ShallowStream, None);
    }
    if (-1200..=1200).contains(&mx) && (-3..=4).contains(&(my - spring_stream_y)) {
        let h = hash_micro(mx, my, 0x11);
        let prop = if h % 16 == 0 {
            Some(NaturalProp::RiverStone)
        } else {
            None
        };
        return (MicroGround::PebbleBank, prop);
    }

    let base_ground = match l0.map(|c| c.material()) {
        Some(SubstrateMaterial::RiverbedQuartzCobble) => MicroGround::PebbleBank,
        Some(SubstrateMaterial::LateriteGravel) => MicroGround::RedGravel,
        _ => MicroGround::LoamGrass,
    };

    // 營地起手開闊坪（mx: -9..=9, my: -7..=6，約 19m × 14m）保持清爽不擋路
    if (-9..=9).contains(&mx) && (-7..=6).contains(&my) {
        return (base_ground, None);
    }

    // 外圍依碎形雜湊成簇長出濃密莿竹叢、春筍與河灘硬石
    let cluster = hash_micro(mx.div_euclid(4), my.div_euclid(4), 0xA5A5);
    let detail = hash_micro(mx, my, 0x5A5A);
    if cluster % 10 < 5 {
        if detail % 10 < 5 {
            return (base_ground, Some(NaturalProp::BambooClump));
        }
        if detail % 23 == 0 {
            return (base_ground, Some(NaturalProp::BambooShoot));
        }
    } else if detail % 37 == 0 {
        return (base_ground, Some(NaturalProp::RiverStone));
    }

    (base_ground, None)
}

/// 判定某細格對角色是否可通行，並回傳移動步進成本（`None` 為不可穿越）。
pub fn micro_step_cost(
    rt: Option<&SubstrateRuntime>,
    overlays: &HashMap<(i32, i32), MicroCellOverlay>,
    mx: i32,
    my: i32,
    is_beast: bool,
) -> Option<u32> {
    let (ground, nat) = derive_micro_pristine(rt, mx, my);
    let ov = overlays.get(&(mx, my));

    if let Some(st) = ov.and_then(|o| o.structure) {
        if st.blocks_actor(is_beast) {
            return None;
        }
        if st == StructureKind::WoodBridge {
            return Some(10);
        }
    }

    if matches!(
        ground,
        MicroGround::SteepCliff | MicroGround::DeepWater | MicroGround::SpringPool
    ) {
        return None;
    }
    if ground == MicroGround::ShallowStream {
        // 未架木橋之山澗不可直接徒步跨越，促使架橋或繞行
        return None;
    }

    let cleared = ov.is_some_and(|o| o.cleared_natural || o.structure.is_some());
    if !cleared && nat.is_some_and(NaturalProp::blocks_movement) {
        return None;
    }

    Some(match ground {
        MicroGround::PebbleBank => 12,
        MicroGround::RedGravel | MicroGround::LoamGrass => 10,
        _ => 14,
    })
}

/// 連續實心身軀包絡半徑環境檢查（防止角色重心貼邊時切進牆、營火、工作台或水岸）。
pub fn is_circle_walkable(
    rt: Option<&SubstrateRuntime>,
    overlays: &HashMap<(i32, i32), MicroCellOverlay>,
    px: f32,
    py: f32,
    r_env: f32,
    is_beast: bool,
) -> bool {
    for (ox, oy) in [
        (0.0, 0.0),
        (r_env, 0.0),
        (-r_env, 0.0),
        (0.0, r_env),
        (0.0, -r_env),
    ] {
        let cx = (px + ox).round() as i32;
        let cy = (py + oy).round() as i32;
        if micro_step_cost(rt, overlays, cx, cy, is_beast).is_none() {
            return false;
        }
    }
    true
}

#[derive(Clone, Copy)]
struct AStarNode {
    f: u32,
    g: u32,
    pos: (i32, i32),
}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f && self.g == other.g && self.pos == other.pos
    }
}
impl Eq for AStarNode {}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f
            .cmp(&self.f)
            .then_with(|| other.g.cmp(&self.g))
            .then_with(|| other.pos.cmp(&self.pos))
    }
}

/// `2m` 細格有界加權 A* 自動尋路（支援八向斜走並嚴防穿牆角 Corner-Cutting）。
/// 若目標格本身為不可穿越物件（如點竹叢去砍、點藍圖去蓋），`allow_adjacent_goal = true` 會自動停在緊鄰格。
pub fn find_micro_path(
    rt: Option<&SubstrateRuntime>,
    overlays: &HashMap<(i32, i32), MicroCellOverlay>,
    start: (i32, i32),
    goal: (i32, i32),
    allow_adjacent_goal: bool,
    is_beast: bool,
) -> Option<Vec<(i32, i32)>> {
    find_micro_path_crowded(
        rt,
        overlays,
        start,
        goal,
        allow_adjacent_goal,
        is_beast,
        &[],
    )
}

/// 附帶人潮避讓代價之 A* 尋路：開闊地優先繞開有人佔用的細格，窄橋／門道無路可繞時仍容許通過。
pub fn find_micro_path_crowded(
    rt: Option<&SubstrateRuntime>,
    overlays: &HashMap<(i32, i32), MicroCellOverlay>,
    start: (i32, i32),
    goal: (i32, i32),
    allow_adjacent_goal: bool,
    is_beast: bool,
    occupied: &[(i32, i32)],
) -> Option<Vec<(i32, i32)>> {
    if start == goal {
        return Some(Vec::new());
    }
    let goal_walkable = micro_step_cost(rt, overlays, goal.0, goal.1, is_beast).is_some();
    if !goal_walkable && !allow_adjacent_goal {
        return None;
    }

    let heuristic = |(x, y): (i32, i32)| -> u32 {
        let dx = (x - goal.0).unsigned_abs();
        let dy = (y - goal.1).unsigned_abs();
        let diag = dx.min(dy);
        let straight = dx.max(dy) - diag;
        diag * 14 + straight * 10
    };

    let mut open = BinaryHeap::new();
    let mut best_g: HashMap<(i32, i32), u32> = HashMap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

    best_g.insert(start, 0);
    open.push(AStarNode {
        f: heuristic(start),
        g: 0,
        pos: start,
    });

    let dirs: [(i32, i32, bool); 8] = [
        (0, 1, false),
        (1, 0, false),
        (0, -1, false),
        (-1, 0, false),
        (1, 1, true),
        (1, -1, true),
        (-1, -1, true),
        (-1, 1, true),
    ];

    let mut expanded = 0usize;
    while let Some(curr) = open.pop() {
        if curr.pos == goal
            || (!goal_walkable
                && allow_adjacent_goal
                && (curr.pos.0 - goal.0).abs() <= 1
                && (curr.pos.1 - goal.1).abs() <= 1
                && (curr.pos.0 == goal.0 || curr.pos.1 == goal.1))
        {
            let mut path = Vec::new();
            let mut p = curr.pos;
            while p != start {
                path.push(p);
                p = *came_from.get(&p)?;
            }
            path.reverse();
            return Some(path);
        }

        if curr.g > *best_g.get(&curr.pos).unwrap_or(&u32::MAX) {
            continue;
        }
        expanded += 1;
        if expanded > MICRO_ASTAR_MAX_NODES {
            return None;
        }

        for (dx, dy, is_diag) in dirs {
            let nx = curr.pos.0 + dx;
            let ny = curr.pos.1 + dy;
            let Some(base_cost) = micro_step_cost(rt, overlays, nx, ny, is_beast) else {
                continue;
            };
            if is_diag {
                // 斜走防穿牆角：兩側正交鄰格皆須可通行
                if micro_step_cost(rt, overlays, curr.pos.0 + dx, curr.pos.1, is_beast).is_none()
                    || micro_step_cost(rt, overlays, curr.pos.0, curr.pos.1 + dy, is_beast)
                        .is_none()
                {
                    continue;
                }
            }
            let base_step = if is_diag {
                (base_cost * 14) / 10
            } else {
                base_cost
            };
            let crowd_penalty =
                if (nx, ny) != goal && (nx, ny) != start && occupied.contains(&(nx, ny)) {
                    12
                } else {
                    0
                };
            let step = base_step.saturating_add(crowd_penalty);
            let next_g = curr.g.saturating_add(step);
            if next_g < *best_g.get(&(nx, ny)).unwrap_or(&u32::MAX) {
                best_g.insert((nx, ny), next_g);
                came_from.insert((nx, ny), curr.pos);
                open.push(AStarNode {
                    f: next_g.saturating_add(heuristic((nx, ny))),
                    g: next_g,
                    pos: (nx, ny),
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_scale_coordinate_roundtrip_handles_negative_cells() {
        assert_eq!(micro_to_macro(0, 0), ((0, 0), (0, 0)));
        assert_eq!(micro_to_macro(19, 19), ((0, 0), (19, 19)));
        assert_eq!(micro_to_macro(-1, -1), ((-1, -1), (19, 19)));
        assert_eq!(micro_to_macro(-20, 35), ((-1, 1), (0, 15)));
    }

    #[test]
    fn stream_blocks_until_wood_bridge_is_built() {
        let mut overlays = HashMap::new();
        // mx=0, my=10..=13 位於 4m 寬蜿蜒山澗帶，未架橋前不可橫越至 (0, 18)
        assert!(find_micro_path(None, &overlays, (0, 0), (0, 18), false, false).is_none());

        for y in 9..=14 {
            overlays.insert(
                (0, y),
                MicroCellOverlay {
                    cleared_natural: true,
                    structure: Some(StructureKind::WoodBridge),
                    blueprint: None,
                    ground_item: None,
                },
            );
        }
        let path = find_micro_path(None, &overlays, (0, 0), (0, 18), false, false);
        assert!(path.is_some());
    }
}
