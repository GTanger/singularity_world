//! 即時合成單格視圖：由 `L0 + 坡勢 + 邊緣線索 + 四層槽 + Field` 動態推得符號、微地名與處境底色。
//!
//! 無寫死格名大表；所有文字皆為世界內生語境，不含數字或百分比字樣。

use super::coords::{DerivedSlope, SensoryEdgeCue};
use super::l0_cell::{L0Cell, SubstrateMaterial, SurfaceWaterKind, landmass};
use super::slots::{
    FieldState, S0Companion, S0Pavement, SlotStack, U1Plugin, V1Plugin,
    is_campfire_extinguished_by_rain,
};

/// 陡壁判定坡度比值（`grade_ratio >= 0.85` 不可徒步）。
const STEEP_GRADE: f32 = 0.85;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthCanvasCard {
    pub title: String,
    pub hint: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SynthesizedCellView {
    pub symbol: &'static str,
    pub micro_name: &'static str,
    pub zone_name: &'static str,
    pub walkable_on_landmass_01: bool,
    pub terrain_line: String,
    pub canvas_cards: Vec<SynthCanvasCard>,
}

fn grade(slope: Option<DerivedSlope>) -> f32 {
    slope.map_or(0.0, |s| s.grade_ratio)
}

/// 世界內生自然分區名。
pub fn zone_name_of(gx: i32, l0: L0Cell) -> &'static str {
    match l0.landmass_id {
        landmass::SEA => "西側深水外海",
        landmass::TAICHUNG_BASIN => "東界斷橋溪谷",
        landmass::HOULI_DAJIA_FAN => "北界深切溪谷",
        landmass::BAGUA_CHANGHUA_PLAIN => "南界寬闊河灘",
        landmass::DAJIA_WU_TABLELAND => {
            let elev = l0.elevation_m().unwrap_or(0.0);
            match l0.material() {
                SubstrateMaterial::IntertidalMudflat => "潮間泥灘海岸",
                SubstrateMaterial::CoastalWindSand => "潮間泥灘海岸",
                SubstrateMaterial::AlluvialSandyLoam if elev < 25.0 => "西濱沖積草海",
                SubstrateMaterial::LateriteGravel if elev >= 95.0 => "紅土礫石台地",
                _ if gx < 200 => "西麓湧泉竹帶",
                _ => "紅土礫石台地",
            }
        }
        _ => "對岸未闢霧境",
    }
}

fn water_symbol_name(w: SurfaceWaterKind, flood: bool) -> Option<(&'static str, &'static str)> {
    Some(match w {
        SurfaceWaterKind::FaultSpring => ("◉", "斷層冷泉"),
        SurfaceWaterKind::DeepRapidRiver => ("≈", "湍急溪流"),
        SurfaceWaterKind::PerennialStream => ("~", "清淺山澗"),
        SurfaceWaterKind::OpenSea => ("≋", "外海深水"),
        SurfaceWaterKind::IntertidalShallow => ("≋", "潮間淺海"),
        SurfaceWaterKind::SeasonalGully if flood => ("~", "漲水乾溝"),
        _ => return None,
    })
}

/// 依優先序（營火 > 地上主構 > 水體 > 根基 > 母岩）合成符號與微地名。
fn symbol_and_name(
    gx: i32,
    gy: i32,
    l0: L0Cell,
    slope: Option<DerivedSlope>,
    slots: &SlotStack,
    field: FieldState,
) -> (&'static str, &'static str) {
    if slots.has_campfire() && !is_campfire_extinguished_by_rain(field.weather_wetting, slots.v1) {
        return ("🔥", "竹林營火");
    }
    match slots.v1 {
        V1Plugin::ArcadeShophouse { arcade_mask, .. } => {
            return if arcade_mask != 0 { ("═", "騎樓連簷") } else { ("▣", "街屋前堂") };
        }
        V1Plugin::ThornyBambooClump => return ("𓐂", "刺竹林"),
        V1Plugin::AncientCamphorCanopy => return ("♣", "古樟林坡"),
        V1Plugin::None => {}
    }
    match (gx, gy) {
        (0, 0) => return ("◦", "伏流卵石灘"),
        (0, 1) => return ("◉", "伏流冷泉"),
        _ => {}
    }
    if slots.u1 == U1Plugin::RuinedBridgePier {
        return ("╫", "舊斷橋墩");
    }
    if let Some(sn) = water_symbol_name(l0.surface_water(), field.flood_active) {
        return sn;
    }
    if matches!(slots.s0_pave, S0Pavement::BambooTrampledPath { .. }) {
        return ("∴", "疏竹小徑");
    }
    if matches!(slots.s0_pave, S0Pavement::StoneStreet) {
        return ("▤", "石板街面");
    }
    if slots.u1 == U1Plugin::MiscanthusRoot {
        return ("\"", "菅芒草海");
    }
    if grade(slope) >= STEEP_GRADE || l0.material() == SubstrateMaterial::FaultBareRock {
        return ("▲", "裸岩陡壁");
    }
    match l0.material() {
        SubstrateMaterial::IntertidalMudflat if field.tide_high => ("≈", "漲潮淺灘"),
        SubstrateMaterial::IntertidalMudflat => ("░", "潮間泥灘"),
        SubstrateMaterial::CoastalWindSand => ("·", "海濱沙地"),
        SubstrateMaterial::RiverbedQuartzCobble => ("◦", "溪畔卵石"),
        SubstrateMaterial::LateriteGravel => ("∙", "紅土礫台"),
        SubstrateMaterial::AlluvialSandyLoam => ("⁚", "沖積砂壤"),
        SubstrateMaterial::Unknown => ("≋", "外海深水"),
        SubstrateMaterial::FaultBareRock => ("▲", "裸岩陡壁"),
    }
}

/// 可否在首發開放陸塊徒步通行。
pub fn walkable_on_landmass_01(l0: L0Cell, slope: Option<DerivedSlope>) -> bool {
    l0.landmass_id == landmass::DAJIA_WU_TABLELAND
        && !matches!(
            l0.surface_water(),
            SurfaceWaterKind::OpenSea | SurfaceWaterKind::DeepRapidRiver
        )
        && grade(slope) < STEEP_GRADE
}

fn ground_phrase(l0: L0Cell) -> &'static str {
    match l0.material() {
        SubstrateMaterial::IntertidalMudflat => "腳下是黏滑的潮泥",
        SubstrateMaterial::CoastalWindSand => "腳下是鬆軟的風砂",
        SubstrateMaterial::AlluvialSandyLoam => "腳下是肥潤的砂壤",
        SubstrateMaterial::LateriteGravel => "腳下是紅褐堅實的礫土",
        SubstrateMaterial::RiverbedQuartzCobble => "腳下是圓潤的石英卵石",
        SubstrateMaterial::FaultBareRock => "腳下是冷硬的裸岩",
        SubstrateMaterial::Unknown => "腳下已無立足之地",
    }
}

fn water_phrase(l0: L0Cell, field: FieldState) -> &'static str {
    match l0.surface_water() {
        SurfaceWaterKind::FaultSpring => "泉眼湧出沁涼的水氣",
        SurfaceWaterKind::DeepRapidRiver => "急流轟響，水勢深不可涉",
        SurfaceWaterKind::PerennialStream => "山澗潺潺，水淺可涉",
        SurfaceWaterKind::OpenSea | SurfaceWaterKind::IntertidalShallow => "鹹風裹著浪聲",
        SurfaceWaterKind::SeasonalGully if field.flood_active => "乾溝漲滿了濁水",
        _ if l0.groundwater_bucket() <= 4 => "土中水氣濕重",
        _ if l0.groundwater_bucket() <= 9 => "土裡隱約透著潮氣",
        _ => "土面乾爽",
    }
}

fn slope_phrase(slope: Option<DerivedSlope>) -> &'static str {
    let g = grade(slope);
    if g >= STEEP_GRADE {
        "坡勢陡得難以落腳"
    } else if g >= 0.35 {
        "地勢明顯向下傾斜"
    } else if g >= 0.08 {
        "地勢微微起伏"
    } else {
        "地面平緩"
    }
}

fn canopy_phrase(slots: &SlotStack, field: FieldState) -> &'static str {
    match slots.v1 {
        V1Plugin::ThornyBambooClump => "頭頂莿竹密冠遮去天光",
        V1Plugin::AncientCamphorCanopy => "頭頂古樟樹冠蔭蔽",
        V1Plugin::ArcadeShophouse { .. } => "頭頂是二樓樓板挑出的簷廊",
        V1Plugin::None if field.weather_wetting => "頭頂無遮，雨絲直落",
        V1Plugin::None => "頭頂開闊見天",
    }
}

fn terrain_line(
    l0: L0Cell,
    slope: Option<DerivedSlope>,
    cue: Option<&SensoryEdgeCue>,
    slots: &SlotStack,
    field: FieldState,
) -> String {
    let mut line = format!(
        "{}，{}，{}，{}。",
        ground_phrase(l0),
        water_phrase(l0, field),
        slope_phrase(slope),
        canopy_phrase(slots, field)
    );
    if let Some(c) = cue {
        line.push_str(&format!("遠處隱約有{}之象。", c.label));
    }
    line
}

fn canvas_cards(slots: &SlotStack, field: FieldState) -> Vec<SynthCanvasCard> {
    let mut cards = Vec::new();
    let mut push = |t: &str, h: &str| {
        cards.push(SynthCanvasCard { title: t.to_string(), hint: h.to_string() });
    };
    if slots.exposes_bamboo_rhizome_socket() {
        push("露土老竹鞭", "竹叢已除，盤錯的老鞭裸露土面，可掘取");
    }
    if slots.has_campfire() {
        if is_campfire_extinguished_by_rain(field.weather_wetting, slots.v1) {
            push("竹葉營火", "雨水澆熄了火堆，只餘濕灰");
        } else {
            push("竹葉營火", "火光搖曳，可取暖或煮食");
        }
    }
    for c in &slots.s0_companions {
        if *c != S0Companion::BambooCampfire {
            push(c.label_zh(), "就在腳邊，可俯身處置");
        }
    }
    if slots.u1 == U1Plugin::RuinedBridgePier {
        push("舊斷橋墩", "舊世橋墩殘根立在水邊");
    }
    cards.truncate(3);
    cards
}

/// 即時合成單格視圖。
pub fn synthesize_cell(
    gx: i32,
    gy: i32,
    l0: L0Cell,
    slope: Option<DerivedSlope>,
    cue: Option<SensoryEdgeCue>,
    slots: &SlotStack,
    field: FieldState,
) -> SynthesizedCellView {
    let (symbol, micro_name) = symbol_and_name(gx, gy, l0, slope, slots, field);
    SynthesizedCellView {
        symbol,
        micro_name,
        zone_name: zone_name_of(gx, l0),
        walkable_on_landmass_01: walkable_on_landmass_01(l0, slope),
        terrain_line: terrain_line(l0, slope, cue.as_ref(), slots, field),
        canvas_cards: canvas_cards(slots, field),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::slots::derive_pristine_slots;

    fn lm1(m: SubstrateMaterial, w: SurfaceWaterKind, elev: f32) -> L0Cell {
        L0Cell::new(Some(elev), 1, m, w, 20.0, 90)
    }

    #[test]
    fn origin_cells_named_from_world() {
        let c = lm1(SubstrateMaterial::RiverbedQuartzCobble, SurfaceWaterKind::None, 40.0);
        let s = derive_pristine_slots(0, 0, c, 1);
        let v = synthesize_cell(0, 0, c, None, None, &s, FieldState::default());
        assert_eq!((v.symbol, v.micro_name), ("◦", "伏流卵石灘"));
        assert!(v.walkable_on_landmass_01);
        let v1 = synthesize_cell(0, 1, c, None, None, &s, FieldState::default());
        assert_eq!(v1.micro_name, "伏流冷泉");
    }

    #[test]
    fn campfire_shows_and_rain_extinguishes_without_cover() {
        let c = lm1(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::None, 40.0);
        let mut s = SlotStack::default();
        s.add_companion(S0Companion::BambooCampfire);
        let dry = synthesize_cell(5, 5, c, None, None, &s, FieldState::default());
        assert_eq!(dry.symbol, "🔥");
        let wet = FieldState { weather_wetting: true, ..FieldState::default() };
        assert_ne!(synthesize_cell(5, 5, c, None, None, &s, wet).symbol, "🔥");
        s.v1 = V1Plugin::ThornyBambooClump;
        assert_eq!(synthesize_cell(5, 5, c, None, None, &s, wet).symbol, "🔥");
    }

    #[test]
    fn rhizome_card_after_clearing_bamboo() {
        let c = lm1(SubstrateMaterial::LateriteGravel, SurfaceWaterKind::None, 60.0);
        let mut s = derive_pristine_slots(1, 0, c, 1);
        assert_eq!(synthesize_cell(1, 0, c, None, None, &s, FieldState::default()).micro_name, "刺竹林");
        s.clear_v1_bamboo();
        let v = synthesize_cell(1, 0, c, None, None, &s, FieldState::default());
        assert_eq!(v.canvas_cards[0].title, "露土老竹鞭");
        assert!(v.canvas_cards.len() <= 3);
    }

    #[test]
    fn walkability_and_no_digits() {
        let sea = L0Cell::sea_nodata();
        let steep = DerivedSlope { dz_dx: 1.0, dz_dy: 0.0, grade_ratio: 1.0 };
        let land = lm1(SubstrateMaterial::LateriteGravel, SurfaceWaterKind::None, 90.0);
        assert!(!walkable_on_landmass_01(sea, None));
        assert!(!walkable_on_landmass_01(land, Some(steep)));
        assert!(!walkable_on_landmass_01(lm1(SubstrateMaterial::AlluvialSandyLoam, SurfaceWaterKind::DeepRapidRiver, 5.0), None));
        let v = synthesize_cell(410, 9, land, Some(steep), None, &SlotStack::default(), FieldState::default());
        assert!(!v.terrain_line.chars().any(|ch| ch.is_ascii_digit() || ch == '%'));
        assert_eq!(v.zone_name, "紅土礫石台地");
    }
}
