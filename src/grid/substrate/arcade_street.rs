//! 西麓亭仔腳騎樓街屋確定性推導、柱廊防穿牆拓撲與 `grid_z (0..=3)` 立體樓層契約。
//!
//! 本模組負責：
//! 1. 確定性推導西麓石板街與兩排（含背對背）綠覆騎樓街屋，0 資料庫髒列，嚴格避開四錨點。
//! 2. 定義四向柱廊位元 `arcade_mask` 雙向互指與臨街面開通規則，徹底防範背牆穿牆破口。
//! 3. 定義全體玩家與居民共用的 `z = 0..=3` 立體樓層進出／上下移動與每層 3 個室內插座。

use super::facade::CellSnapshot;
use super::l0_cell::{L0Cell, SurfaceWaterKind, landmass};
use super::slots::{S0Pavement, SlotStack, U1Plugin, V1Plugin};

/// 柱廊四向連通位元（與 `V1Plugin::ArcadeShophouse.arcade_mask` 對應）。
pub const ARCADE_NORTH: u8 = 0b0001;
pub const ARCADE_EAST: u8 = 0b0010;
pub const ARCADE_SOUTH: u8 = 0b0100;
pub const ARCADE_WEST: u8 = 0b1000;

/// 樓層門鎖與進出模式（預留後續活人屋主與租客掛載，玩家與 NPC 共用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubspaceAccessMode {
    /// 開放進出（公開內堂、客樓或未上鎖之樓層）。
    #[default]
    Public,
    /// 僅屋主與租客可入（後續第三步接入活人戶籍後啟用）。
    TenantsOnly,
}

/// 樓層權限描述符（對應規格書 §4.3 `subspace_access`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubspaceAccess {
    pub building_id: String,
    pub z: i16,
    pub owner: Option<String>,
    pub tenants: Vec<String>,
    pub mode: SubspaceAccessMode,
}

impl SubspaceAccess {
    /// 檢查指定實體是否可進入該樓層空間（玩家與 NPC 共用同一規則）。
    pub fn allows(&self, actor_id: &str) -> bool {
        match self.mode {
            SubspaceAccessMode::Public => true,
            SubspaceAccessMode::TenantsOnly => {
                self.owner.as_deref() == Some(actor_id)
                    || self.tenants.iter().any(|t| t == actor_id)
            }
        }
    }
}

/// 室內固定插座描述符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubspaceFixtureSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub object_type: &'static str,
    pub actions: &'static [&'static str],
}

/// 依 `(dx, dy)` 取得對應四向位元與反方向位元。
pub fn dir_bits(dx: i32, dy: i32) -> Option<(u8, u8)> {
    match (dx, dy) {
        (0, 1) => Some((ARCADE_NORTH, ARCADE_SOUTH)),
        (1, 0) => Some((ARCADE_EAST, ARCADE_WEST)),
        (0, -1) => Some((ARCADE_SOUTH, ARCADE_NORTH)),
        (-1, 0) => Some((ARCADE_WEST, ARCADE_EAST)),
        _ => None,
    }
}

fn is_buildable_land(l0: L0Cell) -> bool {
    l0.landmass_id == landmass::DAJIA_WU_TABLELAND
        && !l0.surface_water().is_deep_water()
        && l0.surface_water() != SurfaceWaterKind::NoData
        && l0.surface_water() != SurfaceWaterKind::FaultSpring
}

fn horizontal_row_mask(gx: i32, min_x: i32, max_x: i32) -> u8 {
    let mut mask = 0u8;
    if gx > min_x {
        mask |= ARCADE_WEST;
    }
    if gx < max_x {
        mask |= ARCADE_EAST;
    }
    mask
}

/// 西麓騎樓街屋的確定性臨街面位元（用來區分臨街正面與實牆屋背）。
///
/// - 北排街屋 `gy = 1, gx = -6..=-2`：臨南側石板街 `gy = 0`（面南，屋背朝北封死）。
/// - 南排街屋 `gy = -1, gx = -6..=-2`：臨北側石板街 `gy = 0`（面北，屋背朝南封死）。
/// - 南背巷街屋 `gy = -2, gx = -5..=-3`：臨南側石板巷 `gy = -3`（面南，北背牆緊貼 `gy = -1` 封死）。
pub fn street_face_mask(gx: i32, gy: i32, v1: V1Plugin) -> u8 {
    if !matches!(v1, V1Plugin::ArcadeShophouse { .. }) {
        return 0;
    }
    match (gx, gy) {
        (-6..=-2, 1) => ARCADE_SOUTH,
        (-6..=-2, -1) => ARCADE_NORTH,
        (-5..=-3, -2) => ARCADE_SOUTH,
        // 未來玩家透過 `cell_overlay` 自建之街屋若未於靜態聚落清單內，預設四向臨街（由鄰屋互指約束）。
        _ => ARCADE_NORTH | ARCADE_EAST | ARCADE_SOUTH | ARCADE_WEST,
    }
}

/// 西麓石板街與騎樓街屋之確定性原生槽位推導（嚴格避開四錨點 `(0,0),(0,1),(1,0),(1,-1)`）。
pub fn derive_arcade_street_slots(gx: i32, gy: i32, l0: L0Cell) -> Option<SlotStack> {
    if matches!((gx, gy), (0, 0) | (0, 1) | (1, 0) | (1, -1)) {
        return None;
    }
    if !is_buildable_land(l0) {
        return None;
    }

    // 1. 西麓主石板街（東西向，直通東端伏流卵石灘 (0,0)）與南背石板巷
    if (gy == 0 && (-6..=-1).contains(&gx)) || (gy == -3 && (-6..=-2).contains(&gx)) {
        return Some(SlotStack {
            u1: U1Plugin::StoneRoadbed,
            s0_pave: S0Pavement::StoneStreet,
            ..SlotStack::default()
        });
    }

    // 2. 北排與南排騎樓街屋（同排東西雙向互指，臨 gy=0 石板街）
    if (gy == 1 || gy == -1) && (-6..=-2).contains(&gx) {
        let max_z = SlotStack::max_supported_z(l0);
        if max_z >= 1 {
            return Some(SlotStack {
                u1: U1Plugin::ShallowRaftFoundation,
                s0_pave: S0Pavement::StoneStreet,
                v1: V1Plugin::ArcadeShophouse {
                    arcade_mask: horizontal_row_mask(gx, -6, -2),
                    shop_open: true,
                    max_z,
                },
                ..SlotStack::default()
            });
        }
    }

    // 3. 背對背南向騎樓街屋（gy = -2, gx = -5..=-3，北背牆緊貼 gy = -1 實牆封死，臨南側 gy = -3 石板巷）
    if gy == -2 && (-5..=-3).contains(&gx) {
        let max_z = SlotStack::max_supported_z(l0);
        if max_z >= 1 {
            return Some(SlotStack {
                u1: U1Plugin::ShallowRaftFoundation,
                s0_pave: S0Pavement::StoneStreet,
                v1: V1Plugin::ArcadeShophouse {
                    arcade_mask: horizontal_row_mask(gx, -5, -3),
                    shop_open: true,
                    max_z,
                },
                ..SlotStack::default()
            });
        }
    }

    None
}

/// 判定 `z = 0` 平面上自 `from` 單步正交移至 `to` 是否通行。
///
/// 規則：
/// 1. 必為正交一步 `|dx| + |dy| == 1` 且目標格在首發陸塊可走。
/// 2. 兩側皆為 `ArcadeShophouse` 時，必須雙向 `arcade_mask` 互指才能沿亭仔腳柱廊連走，否則實牆封死。
/// 3. 單側為 `ArcadeShophouse`（出入街屋）時，必須從街屋臨街面（或柱廊端點開口）進出，屋背實牆封死。
pub fn can_step_horizontal(from: &CellSnapshot, to: &CellSnapshot) -> bool {
    let dx = to.gx - from.gx;
    let dy = to.gy - from.gy;
    let Some((out_bit, in_bit)) = dir_bits(dx, dy) else {
        return false;
    };
    if !to.synth.walkable_on_landmass_01 {
        return false;
    }

    match (from.slots.v1, to.slots.v1) {
        (
            V1Plugin::ArcadeShophouse {
                arcade_mask: m_from,
                ..
            },
            V1Plugin::ArcadeShophouse {
                arcade_mask: m_to, ..
            },
        ) => (m_from & out_bit) != 0 && (m_to & in_bit) != 0,
        (
            V1Plugin::ArcadeShophouse {
                arcade_mask: m_from,
                ..
            },
            _,
        ) => {
            let open_from = m_from | street_face_mask(from.gx, from.gy, from.slots.v1);
            (open_from & out_bit) != 0
        }
        (
            _,
            V1Plugin::ArcadeShophouse {
                arcade_mask: m_to, ..
            },
        ) => {
            let open_to = m_to | street_face_mask(to.gx, to.gy, to.slots.v1);
            (open_to & in_bit) != 0
        }
        _ => true,
    }
}

/// 依當前格快照與所在樓層 `z`，列出所有可行的垂直／內室出口方向（順序固定：`進 / 出 / 上 / 下`）。
pub fn available_floor_exits(snap: &CellSnapshot, z: i16) -> Vec<&'static str> {
    let is_shophouse = matches!(snap.slots.v1, V1Plugin::ArcadeShophouse { .. });
    let mut exits = Vec::new();
    match z {
        0 => {
            if is_shophouse && snap.max_z >= 1 {
                exits.push("進");
            }
            if is_shophouse && snap.max_z >= 2 {
                exits.push("上");
            }
        }
        1 => {
            exits.push("出");
        }
        2 => {
            if is_shophouse && snap.max_z >= 3 {
                exits.push("上");
            }
            exits.push("下");
        }
        3 => {
            exits.push("下");
        }
        _ => {
            exits.push("出");
        }
    }
    exits
}

/// 解析玩家或居民在當前格執行的立體樓層移動（`進 / 出 / 上 / 下`）。
///
/// 嚴格執行：
/// - `z = 0`：`進 -> 1`（1F 內室）、`上 -> 2`（2F 主住家層，須 `max_z >= 2`）
/// - `z = 1`：`出 -> 0`
/// - `z = 2`：`下 -> 0`、`上 -> 3`（3F 高閣，須 `max_z >= 3`）
/// - `z = 3`：`下 -> 2`（禁跳層直下 0）
pub fn resolve_floor_move(
    snap: &CellSnapshot,
    current_z: i16,
    dir: &str,
    actor_id: &str,
) -> Result<(i16, &'static str), &'static str> {
    let is_shophouse = matches!(snap.slots.v1, V1Plugin::ArcadeShophouse { .. });
    let access = default_subspace_access(snap, current_z);
    if !access.allows(actor_id) {
        return Err("門鎖緊閉，無權推門而入。");
    }

    match (current_z, dir) {
        (0, "進") => {
            if is_shophouse && snap.max_z >= 1 {
                Ok((
                    1,
                    "你掀簾推門步入一樓內室，反手掩上木門，將騎樓外的人聲隔在門外。",
                ))
            } else {
                Err("此處無可入內室。")
            }
        }
        (1, "出") => Ok((0, "你拉開門閂邁步而出，重新回到騎樓前堂。")),
        (0, "上") => {
            if is_shophouse && snap.max_z >= 2 {
                Ok((2, "你沿著靠牆木梯拾級而上，來到安靜寬敞的二樓住家層。"))
            } else if is_shophouse {
                Err("此處土層淺軟，街屋未起二樓。")
            } else {
                Err("此處沒有通往樓上的階梯。")
            }
        }
        (2, "下") => Ok((0, "你順著木梯步下樓階，回到一樓騎樓前堂。")),
        (2, "上") => {
            if is_shophouse && snap.max_z >= 3 {
                Ok((3, "你沿著窄木梯再上一層，踏入視野開闊的三樓高閣。"))
            } else if is_shophouse {
                Err("腳下砂壤地基只承二層，這棟街屋並無三樓高閣。")
            } else {
                Err("此處沒有更上層的階梯。")
            }
        }
        (3, "下") => Ok((2, "你沿著高閣木梯往下走回二樓住家層。")),
        (1, "上") | (1, "下") => Err("內室之中未設樓梯，須先步出前堂再上下樓層。"),
        (2, "進") | (2, "出") | (3, "進") | (3, "出") => {
            Err("你在樓上空間，請沿階梯上下樓層。")
        }
        _ => Err("這個方向走不通。"),
    }
}

/// 當前格預設門鎖權限（第一步對街屋內室／樓層開放，第三步接活人戶籍屋契時無痛掛接）。
pub fn default_subspace_access(snap: &CellSnapshot, z: i16) -> SubspaceAccess {
    let building_id = snap
        .overlay
        .as_ref()
        .and_then(|o| o.building_id.clone())
        .unwrap_or_else(|| format!("shophouse:{}:{}", snap.gx, snap.gy));
    SubspaceAccess {
        building_id,
        z,
        owner: None,
        tenants: Vec::new(),
        mode: SubspaceAccessMode::Public,
    }
}

/// 樓層自然中文稱謂（不洩漏 `z=` 技術欄位）。
pub fn floor_label_zh(z: i16) -> &'static str {
    match z {
        0 => "地面前堂",
        1 => "一樓內室",
        2 => "二樓住家",
        3 => "三樓高閣",
        _ => "室內",
    }
}

/// 樓層自然場景敘述（`z > 0` 使用）。
pub fn floor_scene_description(base_name: &str, z: i16) -> String {
    match z {
        1 => format!(
            "{base_name}的一樓內室安靜避風，門扇掩去騎樓外的風聲與腳步，屋內安放著床榻、儲物木櫃與臨街書案。"
        ),
        2 => format!(
            "{base_name}的二樓住家鋪著溫潤木板，臨街木窗透進柔和天光，屋內擺著寬敞床榻、儲物木櫃與窗畔書案。"
        ),
        3 => format!(
            "{base_name}的三樓高閣風涼視野遠，推窗可望見西麓街屋連簷與遠方竹帶，屋內備有床榻、儲物木櫃與臨街書案。"
        ),
        _ => format!("{base_name}在眼前展開。"),
    }
}

/// `z = 1..=3` 每層固定配置的 3 個室內插座（規格書 §4.3，徹底脫離舊 `wt_*` 命名）。
pub const SUBSPACE_FLOOR_FIXTURES: [SubspaceFixtureSpec; 3] = [
    SubspaceFixtureSpec {
        id: "subspace_bed",
        name: "床榻",
        object_type: "bed",
        actions: &["Look", "Rest", "Sleep", "Use"],
    },
    SubspaceFixtureSpec {
        id: "subspace_cabinet",
        name: "儲物木櫃",
        object_type: "cabinet",
        actions: &["Look", "Open", "StoreStash", "RetrieveStash", "Use"],
    },
    SubspaceFixtureSpec {
        id: "subspace_window_desk",
        name: "臨街窗台書案",
        object_type: "desk",
        actions: &["Look", "Operate", "Use"],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::facade::CellFacade;
    use crate::grid::substrate::l0_cell::SubstrateMaterial;
    use crate::grid::substrate::runtime::WORLD_SEED;
    use crate::grid::substrate::slots::FieldState;

    fn loam() -> L0Cell {
        L0Cell::new(
            Some(26.5),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::AlluvialSandyLoam,
            SurfaceWaterKind::None,
            40.0,
            90,
        )
    }

    fn laterite() -> L0Cell {
        L0Cell::new(
            Some(82.0),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::LateriteGravel,
            SurfaceWaterKind::None,
            40.0,
            90,
        )
    }

    fn snap(gx: i32, gy: i32, l0: L0Cell) -> CellSnapshot {
        CellFacade::snapshot_from_parts(
            gx,
            gy,
            l0,
            None,
            None,
            None,
            FieldState::default(),
            WORLD_SEED,
        )
    }

    #[test]
    fn four_anchors_never_overwritten_by_arcade_street() {
        let l0 = loam();
        for (gx, gy) in [(0, 0), (0, 1), (1, 0), (1, -1)] {
            assert!(derive_arcade_street_slots(gx, gy, l0).is_none());
        }
    }

    #[test]
    fn arcade_row_allows_corridor_walk_and_blocks_back_to_back_walls() {
        let l0 = loam();
        let street = snap(-4, 0, l0);
        let south_mid = snap(-4, -1, l0);
        let south_east = snap(-3, -1, l0);
        let back_row_mid = snap(-4, -2, l0);
        let back_alley = snap(-4, -3, l0);
        let north_mid = snap(-4, 1, l0);
        let behind_north = snap(-4, 2, l0);

        // 街道 <-> 南排／北排亭仔腳暢通
        assert!(can_step_horizontal(&street, &south_mid));
        assert!(can_step_horizontal(&south_mid, &street));
        assert!(can_step_horizontal(&street, &north_mid));
        // 同排亭仔腳雙向互指連走暢通
        assert!(can_step_horizontal(&south_mid, &south_east));
        assert!(can_step_horizontal(&south_east, &south_mid));
        // 背對背街屋 (-4,-1) <-> (-4,-2) 嚴格封死、0 穿牆破口
        assert!(!can_step_horizontal(&south_mid, &back_row_mid));
        assert!(!can_step_horizontal(&back_row_mid, &south_mid));
        // 北排屋背 (-4,1) <-> (-4,2) 實牆封死
        assert!(!can_step_horizontal(&north_mid, &behind_north));
        assert!(!can_step_horizontal(&behind_north, &north_mid));
        // 南背巷街屋 (-4,-2) <-> 南石板巷 (-4,-3) 正門暢通
        assert!(can_step_horizontal(&back_row_mid, &back_alley));
        assert!(can_step_horizontal(&back_alley, &back_row_mid));
    }

    #[test]
    fn floor_transitions_respect_bearing_and_no_skip() {
        let s2 = snap(-4, 1, loam());
        assert_eq!(s2.max_z, 2);
        assert_eq!(available_floor_exits(&s2, 0), vec!["進", "上"]);
        assert_eq!(resolve_floor_move(&s2, 0, "進", "p1").map(|v| v.0), Ok(1));
        assert_eq!(resolve_floor_move(&s2, 1, "出", "p1").map(|v| v.0), Ok(0));
        assert_eq!(resolve_floor_move(&s2, 0, "上", "p1").map(|v| v.0), Ok(2));
        // 砂壤承重上限 2F，不可再上 3F
        assert!(resolve_floor_move(&s2, 2, "上", "p1").is_err());
        assert_eq!(resolve_floor_move(&s2, 2, "下", "p1").map(|v| v.0), Ok(0));

        // 紅土礫石承重 3F，逐層上下、禁跳層
        let s3 = snap(-4, 1, laterite());
        assert_eq!(s3.max_z, 3);
        assert_eq!(resolve_floor_move(&s3, 0, "上", "p1").map(|v| v.0), Ok(2));
        assert_eq!(resolve_floor_move(&s3, 2, "上", "p1").map(|v| v.0), Ok(3));
        assert_eq!(resolve_floor_move(&s3, 3, "下", "p1").map(|v| v.0), Ok(2));
        assert!(resolve_floor_move(&s3, 3, "出", "p1").is_err());
        assert!(resolve_floor_move(&s3, 1, "上", "p1").is_err());
    }
}
