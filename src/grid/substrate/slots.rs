//! 單格立體四層槽（`U2 / U1 / S0 / V1`）與 `Field` 場域調變契約（規格書第 4 章）。
//!
//! 未改動格 0 資料庫列：初始槽位由 `L0` 底板與世界種子確定性惰性推導。

use super::l0_cell::{L0Cell, SubstrateMaterial, SurfaceWaterKind, landmass};

/// `U2` 深層地基槽（地下 `-1.5m ～ -8m`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum U2Plugin {
    #[default]
    None,
    /// 深樁，`depth_dm` 為樁深（`0.1m` 單位）。
    DeepPile { depth_dm: u8 },
}

impl U2Plugin {
    /// 深樁遇淺地下水脈（桶碼 `<= 9`，水深 `<= 8m`）即失穩。
    pub fn is_destabilized_by_shallow_water(self, l0: L0Cell) -> bool {
        matches!(self, Self::DeepPile { .. }) && l0.groundwater_bucket() <= 9
    }
    pub fn code_i16(self) -> i16 {
        match self {
            Self::None => 0,
            Self::DeepPile { depth_dm } => i16::from(depth_dm).max(1),
        }
    }
    pub fn from_code_i16(code: i16) -> Self {
        if code <= 0 { Self::None } else { Self::DeepPile { depth_dm: code.min(255) as u8 } }
    }
}

/// `U1` 淺層根基槽（互斥主槽）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum U1Plugin {
    #[default]
    None,
    ThornyBambooRhizome,
    AncientCamphorRoot,
    MiscanthusRoot,
    TilledFarmRidge,
    StoneRoadbed,
    ShallowRaftFoundation,
    RuinedBridgePier,
}

impl U1Plugin {
    pub fn code_i16(self) -> i16 {
        match self {
            Self::None => 0,
            Self::ThornyBambooRhizome => 1,
            Self::AncientCamphorRoot => 2,
            Self::MiscanthusRoot => 3,
            Self::TilledFarmRidge => 4,
            Self::StoneRoadbed => 5,
            Self::ShallowRaftFoundation => 6,
            Self::RuinedBridgePier => 7,
        }
    }
    pub fn from_code_i16(code: i16) -> Self {
        match code {
            1 => Self::ThornyBambooRhizome,
            2 => Self::AncientCamphorRoot,
            3 => Self::MiscanthusRoot,
            4 => Self::TilledFarmRidge,
            5 => Self::StoneRoadbed,
            6 => Self::ShallowRaftFoundation,
            7 => Self::RuinedBridgePier,
            _ => Self::None,
        }
    }
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::None => "無根基",
            Self::ThornyBambooRhizome => "盤錯莿竹鞭",
            Self::AncientCamphorRoot => "百年樟樹根",
            Self::MiscanthusRoot => "菅芒宿根",
            Self::TilledFarmRidge => "翻耕熟地農壟",
            Self::StoneRoadbed => "夯實石板路基",
            Self::ShallowRaftFoundation => "淺寬筏式基座",
            Self::RuinedBridgePier => "舊世斷橋墩",
        }
    }
}

/// `S0` 鋪面槽（1 槽）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum S0Pavement {
    #[default]
    BareSoil,
    BambooTrampledPath { pave_heat: u8 },
    StoneStreet,
    NarrowRailBed,
}

impl S0Pavement {
    pub fn code_u8(self) -> u8 {
        match self {
            Self::BareSoil => 0,
            Self::BambooTrampledPath { .. } => 1,
            Self::StoneStreet => 2,
            Self::NarrowRailBed => 3,
        }
    }
    pub fn from_codes(code: u8, pave_heat: u8) -> Self {
        match code {
            1 => Self::BambooTrampledPath { pave_heat },
            2 => Self::StoneStreet,
            3 => Self::NarrowRailBed,
            _ => Self::BareSoil,
        }
    }
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::BareSoil => "裸土",
            Self::BambooTrampledPath { .. } => "疏竹踩踏徑",
            Self::StoneStreet => "石板街面",
            Self::NarrowRailBed => "窄軌台車道",
        }
    }
}

/// `S0` 伴生物件槽（最多 3 槽）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S0Companion {
    DryBambooLeaf,
    BambooSheath,
    EmergingBambooShoot,
    BambooRootEar,
    SweetPotatoRidge,
    ShadeLeafyRidge,
    IrrigationFurrow,
    StreetCart,
    BambooCampfire,
}

impl S0Companion {
    pub fn code_u8(self) -> u8 {
        match self {
            Self::DryBambooLeaf => 1,
            Self::BambooSheath => 2,
            Self::EmergingBambooShoot => 3,
            Self::BambooRootEar => 4,
            Self::SweetPotatoRidge => 5,
            Self::ShadeLeafyRidge => 6,
            Self::IrrigationFurrow => 7,
            Self::StreetCart => 8,
            Self::BambooCampfire => 9,
        }
    }
    pub fn from_code_u8(code: u8) -> Option<Self> {
        Some(match code {
            1 => Self::DryBambooLeaf,
            2 => Self::BambooSheath,
            3 => Self::EmergingBambooShoot,
            4 => Self::BambooRootEar,
            5 => Self::SweetPotatoRidge,
            6 => Self::ShadeLeafyRidge,
            7 => Self::IrrigationFurrow,
            8 => Self::StreetCart,
            9 => Self::BambooCampfire,
            _ => return None,
        })
    }
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::DryBambooLeaf => "乾竹葉",
            Self::BambooSheath => "枯黃竹籜",
            Self::EmergingBambooShoot => "破土莿竹筍",
            Self::BambooRootEar => "竹根木耳",
            Self::SweetPotatoRidge => "番薯藤畦",
            Self::ShadeLeafyRidge => "耐陰葉菜畦",
            Self::IrrigationFurrow => "引水田溝",
            Self::StreetCart => "早夜市攤車",
            Self::BambooCampfire => "竹葉營火",
        }
    }
    /// 活體共生項（掘除竹鞭時隨之彈出）。
    pub fn is_living_bamboo_symbiont(self) -> bool {
        matches!(self, Self::EmergingBambooShoot | Self::BambooRootEar)
    }
}

/// `V1` 立面主構／林冠槽。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum V1Plugin {
    #[default]
    None,
    ThornyBambooClump,
    AncientCamphorCanopy,
    ArcadeShophouse { arcade_mask: u8, shop_open: bool, max_z: u8 },
}

impl V1Plugin {
    /// 通用遮雨契約：濃密竹冠與騎樓頂皆可遮雨護火。
    pub fn provides_rain_cover(self) -> bool {
        matches!(self, Self::ThornyBambooClump | Self::ArcadeShophouse { .. })
    }
    pub fn code_u8(self) -> u8 {
        match self {
            Self::None => 0,
            Self::ThornyBambooClump => 1,
            Self::AncientCamphorCanopy => 2,
            Self::ArcadeShophouse { .. } => 3,
        }
    }
    pub fn from_codes(code: u8, arcade_mask: u8, shop_open: bool, max_z: u8) -> Self {
        match code {
            1 => Self::ThornyBambooClump,
            2 => Self::AncientCamphorCanopy,
            3 => Self::ArcadeShophouse { arcade_mask, shop_open, max_z },
            _ => Self::None,
        }
    }
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::None => "無地上主構",
            Self::ThornyBambooClump => "合抱莿竹叢",
            Self::AncientCamphorCanopy => "老樟樹冠",
            Self::ArcadeShophouse { .. } => "綠覆騎樓街屋",
        }
    }
}

/// 營火雨滅條件：`wetting && !v1.provides_rain_cover()`。
pub fn is_campfire_extinguished_by_rain(wetting: bool, v1: V1Plugin) -> bool {
    wetting && !v1.provides_rain_cover()
}

/// `Field` 場域調變層（不落盤，由天氣、洪潮與人氣即時疊加）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FieldState {
    pub weather_wetting: bool,
    pub flood_active: bool,
    pub tide_high: bool,
    pub human_presence: u8,
}

/// 單格四層槽堆疊。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SlotStack {
    pub u2: U2Plugin,
    pub u1: U1Plugin,
    pub s0_pave: S0Pavement,
    pub s0_companions: Vec<S0Companion>,
    pub v1: V1Plugin,
}

impl SlotStack {
    /// 劈竹開路：僅移除地上竹叢，竹鞭保留。
    pub fn clear_v1_bamboo(&mut self) -> bool {
        if self.v1 == V1Plugin::ThornyBambooClump {
            self.v1 = V1Plugin::None;
            true
        } else {
            false
        }
    }

    /// 露土老竹鞭插座：竹鞭在位、地上無主構且鋪面不是石板街面。
    pub fn exposes_bamboo_rhizome_socket(&self) -> bool {
        self.u1 == U1Plugin::ThornyBambooRhizome
            && self.v1 == V1Plugin::None
            && !matches!(self.s0_pave, S0Pavement::StoneStreet)
    }

    /// 掘鞭：須先清空 `V1`；活體共生項彈出，乾竹葉、竹籜與營火保留。
    pub fn uproot_u1_rhizome(&mut self) -> bool {
        if self.v1 != V1Plugin::None || self.u1 != U1Plugin::ThornyBambooRhizome {
            return false;
        }
        self.u1 = U1Plugin::None;
        self.s0_companions.retain(|c| !c.is_living_bamboo_symbiont());
        true
    }

    /// 伴生槽上限 3；槽滿回傳 `false`。
    pub fn add_companion(&mut self, c: S0Companion) -> bool {
        if self.s0_companions.len() >= 3 || self.s0_companions.contains(&c) {
            return false;
        }
        self.s0_companions.push(c);
        true
    }

    pub fn has_campfire(&self) -> bool {
        self.s0_companions.contains(&S0Companion::BambooCampfire)
    }

    /// 淺基承重樓層上限：紅土 3、砂壤 2、其他無常態深水陸地 1、深水／泥灘／外海 0。
    pub fn max_supported_z(l0: L0Cell) -> u8 {
        if l0.surface_water().is_deep_water()
            || l0.surface_water() == SurfaceWaterKind::NoData
            || matches!(l0.material(), SubstrateMaterial::IntertidalMudflat | SubstrateMaterial::Unknown)
        {
            return 0;
        }
        match l0.material() {
            SubstrateMaterial::LateriteGravel => 3,
            SubstrateMaterial::AlluvialSandyLoam => 2,
            _ => 1,
        }
    }
}

fn mix(gx: i32, gy: i32, seed: u64) -> u64 {
    let mut z = seed
        ^ (gx as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (gy as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn bamboo_stack(h: u64, shoot: bool) -> SlotStack {
    let mut s = SlotStack {
        u1: U1Plugin::ThornyBambooRhizome,
        v1: V1Plugin::ThornyBambooClump,
        ..SlotStack::default()
    };
    const POOL: [S0Companion; 4] = [
        S0Companion::DryBambooLeaf,
        S0Companion::BambooSheath,
        S0Companion::EmergingBambooShoot,
        S0Companion::BambooRootEar,
    ];
    if shoot {
        s.add_companion(S0Companion::EmergingBambooShoot);
    }
    let want = 1 + ((h >> 8) % 3) as usize;
    let mut i = (h >> 16) as usize;
    while s.s0_companions.len() < want {
        s.add_companion(POOL[i % 4]);
        i += 1;
    }
    s
}

/// 100% 確定性惰性推導未開發格初始槽位。
pub fn derive_pristine_slots(gx: i32, gy: i32, l0: L0Cell, world_seed: u64) -> SlotStack {
    let mut s = SlotStack::default();
    let water = l0.surface_water();
    match (gx, gy) {
        (0, 0) | (0, 1) => return s,
        (1, 0) => {
            let mut b = bamboo_stack(mix(gx, gy, world_seed), true);
            b.s0_companions.truncate(3);
            return b;
        }
        (1, -1) => {
            s.u1 = U1Plugin::AncientCamphorRoot;
            s.v1 = V1Plugin::AncientCamphorCanopy;
            return s;
        }
        (894..=895, 0) if !water.is_deep_water() && water != SurfaceWaterKind::NoData => {
            s.u1 = U1Plugin::RuinedBridgePier;
            return s;
        }
        _ => {}
    }
    if l0.landmass_id != landmass::DAJIA_WU_TABLELAND
        || water != SurfaceWaterKind::None
        || !matches!(l0.material(), SubstrateMaterial::AlluvialSandyLoam | SubstrateMaterial::LateriteGravel)
    {
        return s;
    }
    let Some(elev) = l0.elevation_m() else { return s };
    let h = mix(gx, gy, world_seed);
    let roll = (h % 100) as u32;
    if (18.0..95.0).contains(&elev) && roll < if elev >= 75.0 { 40 } else { 55 } {
        return bamboo_stack(h, false);
    }
    if elev >= 75.0 && l0.material() == SubstrateMaterial::LateriteGravel {
        if (60..85).contains(&roll) {
            s.u1 = U1Plugin::AncientCamphorRoot;
            s.v1 = V1Plugin::AncientCamphorCanopy;
        } else {
            s.u1 = U1Plugin::MiscanthusRoot;
        }
    } else if elev < 25.0 && l0.material() == SubstrateMaterial::AlluvialSandyLoam {
        s.u1 = U1Plugin::MiscanthusRoot;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(m: SubstrateMaterial, w: SurfaceWaterKind, gw: f32, elev: f32) -> L0Cell {
        L0Cell::new(Some(elev), 1, m, w, gw, 90)
    }

    #[test]
    fn split_bamboo_keeps_rhizome_and_exposes_socket() {
        let mut s = bamboo_stack(7, false);
        assert!(!s.exposes_bamboo_rhizome_socket());
        assert!(!s.uproot_u1_rhizome());
        assert!(s.clear_v1_bamboo());
        assert_eq!(s.u1, U1Plugin::ThornyBambooRhizome);
        assert!(s.exposes_bamboo_rhizome_socket());
        s.s0_pave = S0Pavement::StoneStreet;
        assert!(!s.exposes_bamboo_rhizome_socket());
    }

    #[test]
    fn uproot_pops_living_items_keeps_campfire() {
        let mut s = SlotStack { u1: U1Plugin::ThornyBambooRhizome, ..SlotStack::default() };
        for c in [S0Companion::EmergingBambooShoot, S0Companion::BambooCampfire, S0Companion::DryBambooLeaf] {
            assert!(s.add_companion(c));
        }
        assert!(s.uproot_u1_rhizome());
        assert_eq!(s.u1, U1Plugin::None);
        assert_eq!(s.s0_companions, vec![S0Companion::BambooCampfire, S0Companion::DryBambooLeaf]);
    }

    #[test]
    fn rain_cover_protects_campfire() {
        assert!(is_campfire_extinguished_by_rain(true, V1Plugin::None));
        assert!(!is_campfire_extinguished_by_rain(true, V1Plugin::ThornyBambooClump));
        let arcade = V1Plugin::ArcadeShophouse { arcade_mask: 3, shop_open: false, max_z: 2 };
        assert!(!is_campfire_extinguished_by_rain(true, arcade));
        assert!(!is_campfire_extinguished_by_rain(false, V1Plugin::None));
        assert!(is_campfire_extinguished_by_rain(true, V1Plugin::AncientCamphorCanopy));
    }

    #[test]
    fn deep_pile_destabilized_in_shallow_water() {
        let pile = U2Plugin::DeepPile { depth_dm: 50 };
        let wet = cell(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::None, 2.0, 10.0);
        let dry = cell(SubstrateMaterial::LateriteGravel, SurfaceWaterKind::None, 90.0, 100.0);
        assert!(pile.is_destabilized_by_shallow_water(wet));
        assert!(!pile.is_destabilized_by_shallow_water(dry));
        assert!(!U2Plugin::None.is_destabilized_by_shallow_water(wet));
    }

    #[test]
    fn support_floor_limits() {
        let l = cell(SubstrateMaterial::LateriteGravel, SurfaceWaterKind::None, 30.0, 100.0);
        let a = cell(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::None, 5.0, 10.0);
        let q = cell(SubstrateMaterial::RiverbedQuartzCobble, SurfaceWaterKind::None, 5.0, 10.0);
        let m = cell(SubstrateMaterial::IntertidalMudflat, SurfaceWaterKind::None, 1.0, 0.0);
        let d = cell(SubstrateMaterial::LateriteGravel, SurfaceWaterKind::DeepRapidRiver, 1.0, 5.0);
        assert_eq!(SlotStack::max_supported_z(l), 3);
        assert_eq!(SlotStack::max_supported_z(a), 2);
        assert_eq!(SlotStack::max_supported_z(q), 1);
        assert_eq!(SlotStack::max_supported_z(m), 0);
        assert_eq!(SlotStack::max_supported_z(d), 0);
        assert_eq!(SlotStack::max_supported_z(L0Cell::sea_nodata()), 0);
    }

    #[test]
    fn pristine_landmarks_and_determinism() {
        let a = cell(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::None, 5.0, 40.0);
        assert_eq!(derive_pristine_slots(0, 0, a, 1), SlotStack::default());
        let b = derive_pristine_slots(1, 0, a, 1);
        assert_eq!(b.v1, V1Plugin::ThornyBambooClump);
        assert!(b.s0_companions.contains(&S0Companion::EmergingBambooShoot));
        assert!(b.s0_companions.len() <= 3);
        let c = derive_pristine_slots(1, -1, a, 1);
        assert_eq!(c.v1, V1Plugin::AncientCamphorCanopy);
        let p = derive_pristine_slots(894, 0, cell(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::PerennialStream, 0.5, 5.0), 1);
        assert_eq!(p.u1, U1Plugin::RuinedBridgePier);
        let sea = derive_pristine_slots(-700, 0, L0Cell::sea_nodata(), 1);
        assert_eq!(sea, SlotStack::default());
        let low = cell(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::None, 5.0, 10.0);
        assert!(matches!(derive_pristine_slots(-320, 0, low, 9).u1, U1Plugin::MiscanthusRoot));
        assert_eq!(derive_pristine_slots(50, 50, a, 3), derive_pristine_slots(50, 50, a, 3));
    }

    #[test]
    fn code_roundtrip() {
        for c in 0..=7 {
            assert_eq!(U1Plugin::from_code_i16(c).code_i16(), c);
        }
        for c in 1..=9 {
            assert_eq!(S0Companion::from_code_u8(c).map(S0Companion::code_u8), Some(c));
        }
    }
}
