use crate::grid::zhulin::objects::ZhulinTagTriad;

pub mod clear_sun;
pub mod morning_dew_fog;
pub mod mountain_rain;
pub mod post_rain_mist;
pub mod sea_gale;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZhulinWeatherDef {
    pub id: &'static str,
    pub legacy_key: &'static str,
    pub name: &'static str,
    pub short_label: &'static str,
    pub ambient_prose: &'static str,
    pub tags: ZhulinTagTriad,
}

pub const ALL_ZHULIN_WEATHERS: &[ZhulinWeatherDef; 5] = &[
    clear_sun::WEATHER_DEF,
    sea_gale::WEATHER_DEF,
    mountain_rain::WEATHER_DEF,
    post_rain_mist::WEATHER_DEF,
    morning_dew_fog::WEATHER_DEF,
];

pub fn lookup_zhulin_weather(key_or_id: &str) -> &'static ZhulinWeatherDef {
    let needle = key_or_id.trim();
    for def in ALL_ZHULIN_WEATHERS {
        if def.id == needle
            || def.legacy_key == needle
            || def.name == needle
            || def.short_label == needle
        {
            return def;
        }
    }
    &clear_sun::WEATHER_DEF
}

pub fn active_zhulin_weather_for_hour(
    stored_event_type: Option<&str>,
    game_hour: u32,
) -> &'static ZhulinWeatherDef {
    if let Some(k) = stored_event_type {
        let trimmed = k.trim();
        if trimmed == "wind"
            || trimmed == "rain"
            || trimmed == "plum_rain"
            || trimmed == "fog"
            || trimmed == sea_gale::ID
            || trimmed == mountain_rain::ID
            || trimmed == post_rain_mist::ID
            || trimmed == morning_dew_fog::ID
        {
            return lookup_zhulin_weather(trimmed);
        }
    }
    if (5..=7).contains(&game_hour) {
        &morning_dew_fog::WEATHER_DEF
    } else {
        &clear_sun::WEATHER_DEF
    }
}

pub fn next_zhulin_weather_in_cycle(
    current_key_or_id: &str,
    game_hour: u32,
) -> &'static ZhulinWeatherDef {
    let current_def = lookup_zhulin_weather(current_key_or_id);
    if current_def.id == clear_sun::ID {
        if (5..=7).contains(&game_hour) {
            &morning_dew_fog::WEATHER_DEF
        } else {
            &sea_gale::WEATHER_DEF
        }
    } else if current_def.id == morning_dew_fog::ID {
        &clear_sun::WEATHER_DEF
    } else if current_def.id == sea_gale::ID {
        &mountain_rain::WEATHER_DEF
    } else if current_def.id == mountain_rain::ID {
        &post_rain_mist::WEATHER_DEF
    } else if current_def.id == post_rain_mist::ID {
        &clear_sun::WEATHER_DEF
    } else {
        &clear_sun::WEATHER_DEF
    }
}

pub fn current_world_zhulin_weather() -> (&'static ZhulinWeatherDef, i64) {
    let climate = crate::db::get_region_climate("default")
        .ok()
        .flatten()
        .or_else(|| crate::db::get_region_climate("紅土西麓").ok().flatten());
    let game_hour = crate::store::current_game_hour() as u32;
    let def = active_zhulin_weather_for_hour(climate.as_ref().map(|c| c.event_type.as_str()), game_hour);
    let epoch = climate.as_ref().map(|c| c.started_at_unix).unwrap_or(0);
    (def, epoch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_five_weather_objects_triad_tags() {
        assert_eq!(ALL_ZHULIN_WEATHERS.len(), 5);
        for def in ALL_ZHULIN_WEATHERS {
            assert!(!def.id.is_empty());
            assert!(!def.legacy_key.is_empty());
            assert!(!def.name.is_empty());
            assert!(!def.short_label.is_empty());
            assert!(!def.ambient_prose.is_empty());
            assert!(def.tags.has_tag(def.id));
            assert!(def.tags.has_tag("weather"));
        }
        assert!(clear_sun::WEATHER_DEF.tags.has_tag("solar_warming"));
        assert!(sea_gale::WEATHER_DEF.tags.has_tag("gale_force"));
        assert!(mountain_rain::WEATHER_DEF.tags.has_tag("wetting"));
        assert!(post_rain_mist::WEATHER_DEF.tags.has_tag("moist_soil"));
        assert!(morning_dew_fog::WEATHER_DEF.tags.has_tag("dew_condensing"));
    }

    #[test]
    fn test_lookup_zhulin_weather() {
        assert_eq!(lookup_zhulin_weather("clear").id, clear_sun::ID);
        assert_eq!(lookup_zhulin_weather("wind").id, sea_gale::ID);
        assert_eq!(lookup_zhulin_weather("rain").id, mountain_rain::ID);
        assert_eq!(lookup_zhulin_weather("plum_rain").id, post_rain_mist::ID);
        assert_eq!(lookup_zhulin_weather("fog").id, morning_dew_fog::ID);
        assert_eq!(lookup_zhulin_weather("zl_weather_sea_gale").id, sea_gale::ID);
        assert_eq!(lookup_zhulin_weather("九降疾風").id, sea_gale::ID);
        assert_eq!(lookup_zhulin_weather("微雨").id, mountain_rain::ID);
        assert_eq!(lookup_zhulin_weather("unknown_fallback").id, clear_sun::ID);
    }

    #[test]
    fn test_active_and_next_weather_cycle() {
        assert_eq!(active_zhulin_weather_for_hour(Some("wind"), 12).id, sea_gale::ID);
        assert_eq!(active_zhulin_weather_for_hour(None, 6).id, morning_dew_fog::ID);
        assert_eq!(active_zhulin_weather_for_hour(None, 14).id, clear_sun::ID);

        assert_eq!(next_zhulin_weather_in_cycle("clear", 6).id, morning_dew_fog::ID);
        assert_eq!(next_zhulin_weather_in_cycle("clear", 12).id, sea_gale::ID);
        assert_eq!(next_zhulin_weather_in_cycle("wind", 12).id, mountain_rain::ID);
        assert_eq!(next_zhulin_weather_in_cycle("rain", 12).id, post_rain_mist::ID);
        assert_eq!(next_zhulin_weather_in_cycle("plum_rain", 12).id, clear_sun::ID);
        assert_eq!(next_zhulin_weather_in_cycle("fog", 6).id, clear_sun::ID);
    }
}
