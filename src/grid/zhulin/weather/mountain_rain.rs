use crate::grid::zhulin::objects::ZhulinTagTriad;
use super::ZhulinWeatherDef;

pub const ID: &str = "zl_weather_mountain_rain";
pub const LEGACY_KEY: &str = "rain";
pub const NAME: &str = "山雨霏微";
pub const SHORT_LABEL: &str = "微雨";
pub const AMBIENT_PROSE: &str = "山雲低垂、細雨霏微，密密雨絲打在竹葉與紅土上沙沙作響，空氣濕涼沁人。";

pub const TAGS: ZhulinTagTriad = ZhulinTagTriad {
    category_tags: &["weather", "precipitation", "rain_shower"],
    attribute_tags: &["wetting", "rain_harvestable", "rhizome_soaking"],
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
