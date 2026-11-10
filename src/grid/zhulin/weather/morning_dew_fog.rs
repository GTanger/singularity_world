use crate::grid::zhulin::objects::ZhulinTagTriad;
use super::ZhulinWeatherDef;

pub const ID: &str = "zl_weather_morning_dew_fog";
pub const LEGACY_KEY: &str = "fog";
pub const NAME: &str = "晨霧凝露";
pub const SHORT_LABEL: &str = "晨霧";
pub const AMBIENT_PROSE: &str = "乳白晨霧在林間與溪谷低緩流轉，竹葉尖端與草叢皆凝滿冰涼露珠。";

pub const TAGS: ZhulinTagTriad = ZhulinTagTriad {
    category_tags: &["weather", "condensation", "mist_veil"],
    attribute_tags: &["dew_condensing", "fungus_blooming", "cool_damp"],
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
