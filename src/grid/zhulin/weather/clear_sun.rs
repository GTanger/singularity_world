use crate::grid::zhulin::objects::ZhulinTagTriad;
use super::ZhulinWeatherDef;

pub const ID: &str = "zl_weather_clear_sun";
pub const LEGACY_KEY: &str = "clear";
pub const NAME: &str = "朗日晴空";
pub const SHORT_LABEL: &str = "晴朗";
pub const AMBIENT_PROSE: &str = "天色澄明朗日當空，暖陽穿過林隙灑落，微風乾燥清爽。";

pub const TAGS: ZhulinTagTriad = ZhulinTagTriad {
    category_tags: &["weather", "clear_sky", "solar_phase"],
    attribute_tags: &["solar_warming", "air_drying", "clear_visibility"],
    exclusive_tag: ID,
};

pub const WEATHER_DEF: ZhulinWeatherDef = ZhulinWeatherDef {
    id: ID,
    legacy_key: LEGACY_KEY,
    name: NAME,
    short_label: SHORT_LABEL,
    ambient_prose: AMBIENT_PROSE,
    tags: TAGS,
};
