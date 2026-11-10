use crate::grid::zhulin::objects::ZhulinTagTriad;
use super::ZhulinWeatherDef;

pub const ID: &str = "zl_weather_sea_gale";
pub const LEGACY_KEY: &str = "wind";
pub const NAME: &str = "九降疾風";
pub const SHORT_LABEL: &str = "勁風";
pub const AMBIENT_PROSE: &str = "西側海峽吹來強勁的九降疾風，竹梢劇烈翻湧呼嘯，枯葉與竹籜隨風簌簌震落。";

pub const TAGS: ZhulinTagTriad = ZhulinTagTriad {
    category_tags: &["weather", "air_current", "monsoon_wind"],
    attribute_tags: &["gale_force", "canopy_shedding", "air_drying"],
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
