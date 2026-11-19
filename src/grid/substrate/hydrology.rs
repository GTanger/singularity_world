//! 第一條環境壓力因果：「高水期與通行地帶遷移」（單一時鐘 `game_time_scale = 24`）。
//!
//! 依開譜曆四季、多流域高水期脈衝、當前降雨與半日潮汐，動態推導單格之 `FieldState`
//! （不落盤），使低窪季節乾溝、淺水脈與潮間泥灘在高水期自然轉為漫流阻隔，迫使行旅與
//! 尋路改走高地或石板街，而不造成全島同步水災。

use super::l0_cell::{L0Cell, SubstrateMaterial, SurfaceWaterKind, landmass};
use super::slots::{FieldState, S0Pavement, SlotStack, V1Plugin};

/// 開譜曆一年 360 遊戲日（四季各 90 日）。
pub const DAYS_PER_YEAR: i32 = 360;
/// 單日遊戲分鐘數。
pub const MINUTES_PER_DAY: i32 = 1_440;

/// 開譜曆四季。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KaipuSeason {
    /// 春融（1..=90 日）
    SpringThaw,
    /// 夏漲高水期（91..=180 日）
    SummerFlood,
    /// 秋狩（181..=270 日）
    AutumnHunt,
    /// 冬凝（271..=360 日）
    WinterFreeze,
}

impl KaipuSeason {
    pub fn from_game_day(game_day: i32) -> Self {
        let day_of_year = game_day.rem_euclid(DAYS_PER_YEAR);
        match day_of_year {
            0..=89 => Self::SpringThaw,
            90..=179 => Self::SummerFlood,
            180..=269 => Self::AutumnHunt,
            _ => Self::WinterFreeze,
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::SpringThaw => "春融",
            Self::SummerFlood => "夏漲",
            Self::AutumnHunt => "秋狩",
            Self::WinterFreeze => "冬凝",
        }
    }
}

/// 單一世界時鐘快照（由 `game_time_now` 換算）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldClockSnapshot {
    pub game_day: i32,
    pub world_minute: i32,
    pub minute_of_day: u16,
    pub season: KaipuSeason,
    pub weather_wetting: bool,
}

impl WorldClockSnapshot {
    /// 由系統 Unix 時間與單一時鐘參數（`game_time_scale = 24.0`）建構。
    pub fn from_unix(
        now_unix: i64,
        epoch_unix: i64,
        game_time_scale: f64,
        weather_wetting: bool,
    ) -> Self {
        let (sec_midnight, _hour, _min, days) =
            crate::game::game_time_now(now_unix, epoch_unix, game_time_scale);
        let minute_of_day = (sec_midnight.rem_euclid(86_400) / 60) as u16;
        let world_minute = days
            .saturating_mul(MINUTES_PER_DAY)
            .saturating_add(i32::from(minute_of_day));
        Self {
            game_day: days,
            world_minute,
            minute_of_day,
            season: KaipuSeason::from_game_day(days),
            weather_wetting,
        }
    }

    /// 以系統預設單一時鐘（`scale = 24.0`）與當前天氣建構即時快照。
    pub fn current_live() -> Self {
        let (w_def, _) = crate::grid::zhulin::weather::current_world_zhulin_weather();
        let wetting = matches!(
            w_def.legacy_key,
            "rain" | "plum_rain" | "storm" | "light_rain" | "downpour"
        );
        let now_unix = crate::game::now_unix();
        Self::from_unix(now_unix, 0, 24.0, wetting)
    }

    /// 判斷指定座標流域於當日是否處於「夏漲高水脈衝期」。
    ///
    /// 多流域以錯開相位輪流漲退（非全島同步水災）：每 12 遊戲日一個脈衝窗，其中 4 日為高水期；
    /// 降雨期間高水窗自然延長至 6 日。
    pub fn is_high_water_pulse_at(&self, gx: i32, gy: i32) -> bool {
        if self.season != KaipuSeason::SummerFlood && !self.weather_wetting {
            return false;
        }
        // 以 64×64 格（約 1.28 km）流域區塊錯開來水相位，避免全島同步水災
        let basin_x = gx.div_euclid(64);
        let basin_y = gy.div_euclid(64);
        let phase = (basin_x.wrapping_mul(5) ^ basin_y.wrapping_mul(7)).rem_euclid(12);
        let cycle_day = (self.game_day + phase).rem_euclid(12);
        let window = match (self.season, self.weather_wetting) {
            (KaipuSeason::SummerFlood, true) => 6,
            (KaipuSeason::SummerFlood, false) => 4,
            (_, true) => 2,
            _ => 0,
        };
        cycle_day < window
    }

    /// 半日潮汐高潮判定（約 12 遊戲小時一輪，高潮維持 4 遊戲小時）。
    pub fn is_high_tide(&self) -> bool {
        let half_day_min = self.minute_of_day % 720;
        (180..420).contains(&half_day_min)
    }
}

/// 推導單格在當下單一時鐘與天候下的 `FieldState`。
pub fn derive_cell_field(gx: i32, gy: i32, l0: L0Cell, clock: WorldClockSnapshot) -> FieldState {
    let pulse = clock.is_high_water_pulse_at(gx, gy);
    let sw = l0.surface_water();
    let elev = l0.elevation_m().unwrap_or(100.0);
    // 僅季節乾溝與淺水低窪漫流帶（不含高坡、不含斷層冷泉）於高水脈衝活化漫流
    let low_basin_overflow = l0.landmass_id == landmass::DAJIA_WU_TABLELAND
        && sw == SurfaceWaterKind::None
        && elev <= 25.0
        && l0.groundwater_bucket() <= 4;
    let flood_active = pulse
        && (matches!(
            sw,
            SurfaceWaterKind::SeasonalGully | SurfaceWaterKind::PerennialStream
        ) || low_basin_overflow);
    let tide_high = clock.is_high_tide()
        && (sw == SurfaceWaterKind::IntertidalShallow
            || l0.material() == SubstrateMaterial::IntertidalMudflat);
    FieldState {
        weather_wetting: clock.weather_wetting,
        flood_active,
        tide_high,
        human_presence: 0,
    }
}

/// 高水期與潮汐下的單格涉水阻隔判定（石板街面、窄軌與騎樓街屋因架高／排水不受低窪漫流阻斷）。
pub fn is_blocked_by_high_water(l0: L0Cell, slots: &SlotStack, field: FieldState) -> bool {
    if matches!(slots.v1, V1Plugin::ArcadeShophouse { .. })
        || matches!(
            slots.s0_pave,
            S0Pavement::StoneStreet | S0Pavement::NarrowRailBed
        )
    {
        return false;
    }
    if field.tide_high && l0.material() == SubstrateMaterial::IntertidalMudflat {
        return true;
    }
    if field.flood_active
        && matches!(
            l0.surface_water(),
            SurfaceWaterKind::SeasonalGully | SurfaceWaterKind::PerennialStream
        )
    {
        return true;
    }
    false
}

/// 高水期低窪漫流涉水遲滯加成（毫秒）：未阻斷但積水泥濘時額外增加步時。
pub fn high_water_step_penalty_ms(l0: L0Cell, slots: &SlotStack, field: FieldState) -> u16 {
    if !field.flood_active
        || matches!(slots.v1, V1Plugin::ArcadeShophouse { .. })
        || matches!(
            slots.s0_pave,
            S0Pavement::StoneStreet | S0Pavement::NarrowRailBed
        )
    {
        return 0;
    }
    if l0.groundwater_bucket() <= 4 { 160 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gully_cell() -> L0Cell {
        L0Cell::new(
            Some(22.0),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::AlluvialSandyLoam,
            SurfaceWaterKind::SeasonalGully,
            4.0,
            80,
        )
    }

    fn mudflat_cell() -> L0Cell {
        L0Cell::new(
            Some(1.0),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::IntertidalMudflat,
            SurfaceWaterKind::IntertidalShallow,
            1.0,
            90,
        )
    }

    #[test]
    fn seasonal_gully_blocks_during_summer_high_water_and_reopens_in_dry_season() {
        let cell = gully_cell();
        let bare = SlotStack::default();
        let street = SlotStack {
            s0_pave: S0Pavement::StoneStreet,
            ..SlotStack::default()
        };
        let summer_pulse = WorldClockSnapshot {
            game_day: 96, // 夏漲，phase(0,0)=0 -> 96 % 12 = 0 < 4
            world_minute: 96 * MINUTES_PER_DAY + 600,
            minute_of_day: 600,
            season: KaipuSeason::SummerFlood,
            weather_wetting: false,
        };
        let f_flood = derive_cell_field(0, 0, cell, summer_pulse);
        assert!(f_flood.flood_active);
        assert!(is_blocked_by_high_water(cell, &bare, f_flood));
        // 石板街架高排水不受阻隔
        assert!(!is_blocked_by_high_water(cell, &street, f_flood));

        let autumn_dry = WorldClockSnapshot {
            game_day: 204, // 秋狩乾季
            world_minute: 204 * MINUTES_PER_DAY + 600,
            minute_of_day: 600,
            season: KaipuSeason::AutumnHunt,
            weather_wetting: false,
        };
        let f_dry = derive_cell_field(0, 0, cell, autumn_dry);
        assert!(!f_dry.flood_active);
        assert!(!is_blocked_by_high_water(cell, &bare, f_dry));
    }

    #[test]
    fn basins_stagger_high_water_not_island_wide_flood() {
        let summer = WorldClockSnapshot {
            game_day: 96,
            world_minute: 96 * MINUTES_PER_DAY + 600,
            minute_of_day: 600,
            season: KaipuSeason::SummerFlood,
            weather_wetting: false,
        };
        // 相鄰不同流域區塊（gx=0 vs gx=64）不應永遠同步漲水
        let p0 = summer.is_high_water_pulse_at(0, 0);
        let p1 = summer.is_high_water_pulse_at(64, 0);
        assert_ne!(p0, p1, "不同流域應錯開高水相位，非全島同步水災");
    }

    #[test]
    fn high_tide_blocks_mudflat_low_tide_opens() {
        let cell = mudflat_cell();
        let bare = SlotStack::default();
        let high = WorldClockSnapshot {
            game_day: 10,
            world_minute: 10 * MINUTES_PER_DAY + 240,
            minute_of_day: 240,
            season: KaipuSeason::SpringThaw,
            weather_wetting: false,
        };
        let low = WorldClockSnapshot {
            minute_of_day: 60,
            ..high
        };
        assert!(is_blocked_by_high_water(
            cell,
            &bare,
            derive_cell_field(-50, 0, cell, high)
        ));
        assert!(!is_blocked_by_high_water(
            cell,
            &bare,
            derive_cell_field(-50, 0, cell, low)
        ));
    }
}
