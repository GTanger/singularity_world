//! `L0` 晶片底板（Bare Substrate）單格 6-Byte 物理針腳契約。
//!
//! 對應正典：`docs/design/energetic_taiwan_landmass_and_substrate_spec.md` §3。
//! 核心不變量：
//! 1. `size_of::<L0Cell>() == 6`（`u16` 置首消除對齊補位，小端序序列化固定 6 Bytes）。
//! 2. 高程採 `+500.0m` 偏移、`0.1m` 量化無號 `u16`（有效 `-500.0m ～ +6,053.4m`，`0xFFFF` 為 `NODATA`）。
//! 3. `water_regime` 高 4 位記常態地表水態、低 4 位記 16 階非線性地下水深桶；洪峰與潮汐歸上層 `Field`。
//! 4. `base_energy` 僅存無人時之天然散逸能量基底（`0..=100`），絕不覆寫聚念場壓縮值。

/// 無高程資料哨兵碼（對應海框外或原始圖資缺值）。
pub const ELEV_RAW_NODATA: u16 = 0xFFFF;
/// 高程偏移基準（公尺）：將 `-500.0m` 映射至 `0`。
pub const ELEV_OFFSET_M: f32 = 500.0;
/// 最大可編碼原始整數（`65534` 對應 `+6,053.4m`）。
pub const ELEV_RAW_MAX_VALID: u16 = 0xFFFE;

/// 自然陸塊戶籍常數（全島每格唯一歸屬）。
pub mod landmass {
    /// 外海與潮間帶外水域
    pub const SEA: u8 = 0;
    /// 首塊開放陸塊：北大甲溪—南烏溪・海線紅土台地塊
    pub const DAJIA_WU_TABLELAND: u8 = 1;
    /// 東鄰：內陸構造盆地塊（對位臺中盆地）
    pub const TAICHUNG_BASIN: u8 = 2;
    /// 北鄰：北台地與扇原塊（對位大安溪—大甲溪之后里台地與大甲扇原）
    pub const HOULI_DAJIA_FAN: u8 = 3;
    /// 南鄰：南長台地與平原塊（對位烏溪—濁水溪之八卦台地與彰化平原）
    pub const BAGUA_CHANGHUA_PLAIN: u8 = 4;
    /// 東北／東南鄰：內山丘陵與麓山帶
    pub const FOOTHILLS_EAST: u8 = 5;
}

/// 天然母岩土層（不含任何人類遺留建築或路基；前代遺構一律歸 `U1/U2` 插件）。
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubstrateMaterial {
    /// 0: 潮間淤泥灘
    IntertidalMudflat = 0,
    /// 1: 海濱風積砂
    CoastalWindSand = 1,
    /// 2: 平原沖積砂壤（肥沃易耕，淺基承重限 2 層樓）
    AlluvialSandyLoam = 2,
    /// 3: 台地紅土礫石層（透水乾旱，地盤堅實，淺基承重達 3 層樓）
    LateriteGravel = 3,
    /// 4: 河床石英卵石層（溪底石材源）
    RiverbedQuartzCobble = 4,
    /// 5: 斷層裸岩陡壁
    FaultBareRock = 5,
    /// 255: 未知／外海水底未分類
    Unknown = 255,
}

impl SubstrateMaterial {
    #[inline]
    pub const fn from_u8(code: u8) -> Self {
        match code {
            0 => Self::IntertidalMudflat,
            1 => Self::CoastalWindSand,
            2 => Self::AlluvialSandyLoam,
            3 => Self::LateriteGravel,
            4 => Self::RiverbedQuartzCobble,
            5 => Self::FaultBareRock,
            _ => Self::Unknown,
        }
    }

    /// 淺寬筏式基座（`U1`）在此天然母岩土層上的最大安全承載樓層數（`0..=3`）。
    #[inline]
    pub const fn max_shallow_foundation_floors(self) -> u8 {
        match self {
            Self::LateriteGravel | Self::FaultBareRock => 3,
            Self::AlluvialSandyLoam | Self::RiverbedQuartzCobble => 2,
            Self::CoastalWindSand => 1,
            Self::IntertidalMudflat | Self::Unknown => 0,
        }
    }

    pub const fn name_zh(self) -> &'static str {
        match self {
            Self::IntertidalMudflat => "潮間淤泥灘",
            Self::CoastalWindSand => "海濱風積砂",
            Self::AlluvialSandyLoam => "平原沖積砂壤",
            Self::LateriteGravel => "台地紅土礫石層",
            Self::RiverbedQuartzCobble => "河床石英卵石層",
            Self::FaultBareRock => "斷層裸岩陡壁",
            Self::Unknown => "未定水底層",
        }
    }
}

/// 常態地表水態（存於 `water_regime` 高 4 位；雨季洪峰與潮汐漲退由 `Field` 動態覆寫）。
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceWaterKind {
    /// 0: 無常態地表水
    None = 0,
    /// 1: 季節乾溝（旱季乾涸可通行，雨季由 Field 啟流）
    SeasonalGully = 1,
    /// 2: 常流山澗（可涉水汲水）
    PerennialStream = 2,
    /// 3: 斷層湧泉眼（恆溫冷泉）
    FaultSpring = 3,
    /// 4: 湍急深溪（無橋船不可徒步渡河）
    DeepRapidRiver = 4,
    /// 5: 潮間淺海
    IntertidalShallow = 5,
    /// 6: 外海深水
    OpenSea = 6,
    /// 15: 無資料
    NoData = 15,
}

impl SurfaceWaterKind {
    #[inline]
    pub const fn from_nibble(nibble: u8) -> Self {
        match nibble & 0x0F {
            0 => Self::None,
            1 => Self::SeasonalGully,
            2 => Self::PerennialStream,
            3 => Self::FaultSpring,
            4 => Self::DeepRapidRiver,
            5 => Self::IntertidalShallow,
            6 => Self::OpenSea,
            _ => Self::NoData,
        }
    }

    /// 常態下是否允許無橋船徒步涉過。
    #[inline]
    pub const fn is_wadeable_on_foot(self) -> bool {
        matches!(
            self,
            Self::None | Self::SeasonalGully | Self::PerennialStream | Self::FaultSpring
        )
    }

    /// 是否屬於深水體（無 `U1` 地下陸生根基槽）。
    #[inline]
    pub const fn is_deep_water(self) -> bool {
        matches!(
            self,
            Self::DeepRapidRiver | Self::IntertidalShallow | Self::OpenSea
        )
    }
}

/// 16 階非線性地下水深桶上界表（單位：公尺；索引 `0..=14` 為有限上界，`15` 代表 `> 40.0m`）。
pub const GROUNDWATER_BUCKET_UPPER_M: [f32; 16] = [
    0.0,
    0.3,
    0.5,
    1.0,
    1.5,
    2.0,
    3.0,
    4.0,
    6.0,
    8.0,
    12.0,
    15.0,
    20.0,
    30.0,
    40.0,
    f32::INFINITY,
];

/// 將地下水深（公尺）量化為低 4 位桶碼（`0..=15`）。
#[inline]
pub fn encode_groundwater_depth_bucket(depth_m: f32) -> u8 {
    if !depth_m.is_finite() || depth_m > 40.0 {
        return 15;
    }
    let clamped = depth_m.max(0.0);
    for (idx, &upper) in GROUNDWATER_BUCKET_UPPER_M.iter().take(15).enumerate() {
        if clamped <= upper {
            return idx as u8;
        }
    }
    15
}

/// 讀取低 4 位桶碼對應之保守水深下界（公尺），供深樁失穩判定使用。
#[inline]
pub fn groundwater_bucket_lower_bound_m(bucket: u8) -> f32 {
    let b = (bucket & 0x0F) as usize;
    if b == 0 {
        0.0
    } else {
        GROUNDWATER_BUCKET_UPPER_M[b - 1]
    }
}

/// 封裝高 4 位地表水態與低 4 位地下水深桶之單一位元組。
#[inline]
pub const fn pack_water_regime(surface: SurfaceWaterKind, depth_bucket: u8) -> u8 {
    ((surface as u8) << 4) | (depth_bucket & 0x0F)
}

/// 晶片底板單格 6-Byte 物理針腳結構。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct L0Cell {
    /// 偏移海拔：`round((h_m + 500.0) * 10.0)`，`0xFFFF` 代表無資料。
    pub elev_raw: u16,
    /// 自然陸塊戶籍號（`0 = 外海`，`1 = 北大甲溪—南烏溪塊`……）。
    pub landmass_id: u8,
    /// 天然母岩土層代碼（`SubstrateMaterial`）。
    pub substrate_material: u8,
    /// 高 4 位常態地表水態 ＋ 低 4 位非線性地下水深桶。
    pub water_regime: u8,
    /// 無人時之天然散逸能量基底（`0..=100`）。
    pub base_energy: u8,
}

// 編譯期鎖死單格 6 Bytes（零對齊補位膨脹）。
const _: () = assert!(std::mem::size_of::<L0Cell>() == 6);

impl L0Cell {
    /// 建立標準外海無資料底板格。
    #[inline]
    pub const fn sea_nodata() -> Self {
        Self {
            elev_raw: ELEV_RAW_NODATA,
            landmass_id: landmass::SEA,
            substrate_material: SubstrateMaterial::Unknown as u8,
            water_regime: pack_water_regime(SurfaceWaterKind::OpenSea, 0),
            base_energy: 100,
        }
    }

    /// 由物理參數建構單格底板針腳（自動夾限能量至 `0..=100`）。
    #[inline]
    pub fn new(
        elevation_m: Option<f32>,
        landmass_id: u8,
        material: SubstrateMaterial,
        surface_water: SurfaceWaterKind,
        groundwater_depth_m: f32,
        base_energy: u8,
    ) -> Self {
        let bucket = encode_groundwater_depth_bucket(groundwater_depth_m);
        Self {
            elev_raw: Self::encode_elevation_m(elevation_m),
            landmass_id,
            substrate_material: material as u8,
            water_regime: pack_water_regime(surface_water, bucket),
            base_energy: base_energy.min(100),
        }
    }

    /// 將公尺高程編碼為 `+500m` 偏移之 `0.1m` 無號 `u16`。
    #[inline]
    pub fn encode_elevation_m(elevation_m: Option<f32>) -> u16 {
        let Some(h) = elevation_m else {
            return ELEV_RAW_NODATA;
        };
        if !h.is_finite() {
            return ELEV_RAW_NODATA;
        }
        let scaled = ((h + ELEV_OFFSET_M) * 10.0).round();
        if scaled <= 0.0 {
            0
        } else if scaled >= f32::from(ELEV_RAW_MAX_VALID) {
            ELEV_RAW_MAX_VALID
        } else {
            scaled as u16
        }
    }

    /// 解碼真實海拔高程（公尺）；若為 `0xFFFF` 則回傳 `None`。
    #[inline]
    pub fn elevation_m(self) -> Option<f32> {
        if self.elev_raw == ELEV_RAW_NODATA {
            None
        } else {
            Some(f32::from(self.elev_raw) * 0.1 - ELEV_OFFSET_M)
        }
    }

    #[inline]
    pub const fn material(self) -> SubstrateMaterial {
        SubstrateMaterial::from_u8(self.substrate_material)
    }

    #[inline]
    pub const fn surface_water(self) -> SurfaceWaterKind {
        SurfaceWaterKind::from_nibble(self.water_regime >> 4)
    }

    #[inline]
    pub const fn groundwater_bucket(self) -> u8 {
        self.water_regime & 0x0F
    }

    /// 判定 `U2` 深樁（深度 `pile_depth_m`，如 `1.5m ～ 8.0m`）是否因觸及淺水脈盲區而頂推失穩。
    #[inline]
    pub fn is_deep_pile_unstable(self, pile_depth_m: f32, in_human_aura: bool) -> bool {
        if in_human_aura {
            return false;
        }
        let bucket = self.groundwater_bucket();
        bucket <= 9 && pile_depth_m >= groundwater_bucket_lower_bound_m(bucket)
    }

    /// 固定小端序 6-Byte 序列化（不依賴宿主端序）。
    #[inline]
    pub const fn to_le_bytes(self) -> [u8; 6] {
        let [e0, e1] = self.elev_raw.to_le_bytes();
        [
            e0,
            e1,
            self.landmass_id,
            self.substrate_material,
            self.water_regime,
            self.base_energy,
        ]
    }

    /// 固定小端序 6-Byte 反序列化。
    #[inline]
    pub fn from_le_bytes(b: [u8; 6]) -> Self {
        Self {
            elev_raw: u16::from_le_bytes([b[0], b[1]]),
            landmass_id: b[2],
            substrate_material: b[3],
            water_regime: b[4],
            base_energy: b[5].min(100),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_cell_size_and_roundtrip_invariants() {
        assert_eq!(std::mem::size_of::<L0Cell>(), 6);

        // 1. 西濱潮間泥灘退潮負高程 -2.5m（舊 u16 會下溢）
        let mud = L0Cell::new(
            Some(-2.5),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::IntertidalMudflat,
            SurfaceWaterKind::IntertidalShallow,
            0.0,
            100,
        );
        let mud_rt = L0Cell::from_le_bytes(mud.to_le_bytes());
        assert_eq!(mud_rt, mud);
        assert!((mud_rt.elevation_m().unwrap() - (-2.5)).abs() < 1e-4);

        // 2. 玉山主峰 +3952.4m（舊有號 i16 dm 會上溢）
        let yushan = L0Cell::new(
            Some(3952.4),
            landmass::FOOTHILLS_EAST,
            SubstrateMaterial::FaultBareRock,
            SurfaceWaterKind::None,
            45.0,
            95,
        );
        assert!((yushan.elevation_m().unwrap() - 3952.4).abs() < 1e-3);
        assert_eq!(yushan.groundwater_bucket(), 15);

        // 3. NODATA 哨兵值
        let sea = L0Cell::sea_nodata();
        assert_eq!(sea.elevation_m(), None);
    }

    #[test]
    fn foundation_bearing_and_deep_pile_physics() {
        let spring_edge = L0Cell::new(
            Some(35.0),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::AlluvialSandyLoam,
            SurfaceWaterKind::None,
            1.2, // bucket 4 (1.0..=1.5m)
            100,
        );
        assert_eq!(
            spring_edge.material().max_shallow_foundation_floors(),
            2,
            "沖積砂壤淺基限承 2F"
        );
        assert!(
            spring_edge.is_deep_pile_unstable(6.0, false),
            "無人意識盲區下 6m 深樁泡入 1.2m 淺水脈必失穩"
        );

        let laterite_top = L0Cell::new(
            Some(240.0),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::LateriteGravel,
            SurfaceWaterKind::None,
            28.0, // bucket 13 (20..=30m)
            82,
        );
        assert_eq!(
            laterite_top.material().max_shallow_foundation_floors(),
            3,
            "台地紅土礫石層淺基可承 3F"
        );
        assert!(!laterite_top.is_deep_pile_unstable(6.0, false));
    }
}
