use crate::grid::zhulin::objects::ZhulinTagTriad;
use super::ZhulinWeatherDef;

pub const ID: &str = "zl_weather_post_rain_mist";
pub const LEGACY_KEY: &str = "plum_rain";
pub const NAME: &str = "雨後初霽";
pub const SHORT_LABEL: &str = "雨霽";
pub const AMBIENT_PROSE: &str = "雨勢初歇、雲隙透出天光，林間水氣蒸騰，濕軟紅土散發濃郁土香與草木生機。";

pub const TAGS: ZhulinTagTriad = ZhulinTagTriad {
    category_tags: &["weather", "post_precipitation", "humid_clearing"],
    attribute_tags: &["rhizome_awakening", "fungus_blooming", "moist_soil"],
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
