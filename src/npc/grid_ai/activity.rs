use crate::npc::IntentType;
use crate::world::place::PlaceGraph;

pub(super) fn staying_activity(intent: IntentType) -> &'static str {
    match intent {
        IntentType::Gather => "正在採集",
        IntentType::Socialize => "正在與身邊的人往來",
        IntentType::Work => "正在工作",
        IntentType::SeekJob => "正在找工作",
        IntentType::Trade => "正在交易",
        IntentType::Beg => "正在向人求助",
        IntentType::Wander => "停下來觀察四周",
        IntentType::Idle => "正在休息",
    }
}

pub(super) fn synthesize_moving_activity(
    intent: IntentType,
    dest_name: &str,
    blocked_by: Option<&str>,
    obligation_kind: Option<&str>,
) -> String {
    let blocked_suffix = match blocked_by {
        Some("route_weather_blocked") => "（因風雨路滑受阻）",
        Some("route_invalidated") => "（因前方路徑受阻）",
        Some(_) => "（途中受阻）",
        None => "",
    };
    let ob_prefix = match obligation_kind {
        Some("repair_tool") => "忙於修整器物，",
        Some("repay_debt") => "設法籌措欠款，",
        Some("stranded_cargo") => "設法轉運滯留貨物，",
        Some("treat_injury") => "調息養傷，",
        Some("craft_material") => "忙於籌備工坊料件，",
        Some("fuel_shortage") | Some("supply_shortage") | Some("restock_goods") => "忙於採買添補物資，",
        Some(_) | None => "",
    };

    let dest = if dest_name.trim().is_empty() {
        "目的地"
    } else {
        crate::pg::living_world::peninsula_market_settlement_name(dest_name.trim())
    };

    let base = match intent {
        IntentType::Gather => format!("正前往{}採集資源", dest),
        IntentType::Socialize => format!("正前往{}尋訪故友", dest),
        IntentType::Work | IntentType::SeekJob => format!("正前往{}工作", dest),
        IntentType::Trade => format!("正帶著物品前往{}交易", dest),
        IntentType::Beg => format!("正前往{}尋求幫助", dest),
        IntentType::Wander => format!("正漫步前往{}", dest),
        IntentType::Idle => format!("正前往{}歇息", dest),
    };

    format!("{}{}{}", ob_prefix, base, blocked_suffix)
}

pub(super) fn synthesize_staying_activity(
    intent: IntentType,
    place_name: &str,
    blocked_by: Option<&str>,
    obligation_kind: Option<&str>,
) -> String {
    let blocked_suffix = match blocked_by {
        Some("route_weather_blocked") => "（因風雨路滑受阻）",
        Some("route_invalidated") => "（因路徑中斷暫留）",
        Some(_) => "（行程受阻）",
        None => "",
    };
    let ob_prefix = match obligation_kind {
        Some("repair_tool") => "忙於修整器物，",
        Some("repay_debt") => "設法籌措欠款，",
        Some("stranded_cargo") => "設法轉運滯留貨物，",
        Some("treat_injury") => "調息養傷，",
        Some("craft_material") => "忙於籌備工坊料件，",
        Some("fuel_shortage") | Some("supply_shortage") | Some("restock_goods") => "忙於採買添補物資，",
        Some(_) | None => "",
    };

    let place = if place_name.trim().is_empty() {
        "此處"
    } else {
        crate::pg::living_world::peninsula_market_settlement_name(place_name.trim())
    };

    let base = match intent {
        IntentType::Gather => format!("在{}採集", place),
        IntentType::Socialize => format!("在{}與身邊的人往來交談", place),
        IntentType::Work => {
            if place.contains("講堂街") || place.contains("蒙學") || place.contains("書院") || place.contains("學院") {
                format!("在{}翻閱新印的書冊與經卷", place)
            } else if place.contains("分家巷") || place.contains("百工") || place.contains("鍛造") || place.contains("鐵器") {
                format!("在{}端詳新打好的鐵器與磨治刃口", place)
            } else if place.contains("三水口") || place.contains("商行") || place.contains("商算") {
                format!("在{}核對商號貨單與撥打算盤", place)
            } else if place.contains("西門") || place.contains("聯運") || place.contains("車馬") {
                format!("在{}查看聯運駝獸車班與核對地秤", place)
            } else if place.contains("食堂") || place.contains("廚房") {
                format!("在{}幫廚備菜與照看大釜火候", place)
            } else if place.contains("浴場") || place.contains("石湯") {
                format!("在{}搬運草木湯包並調節水槽水溫", place)
            } else if place.contains("書閣") || place.contains("藏書") || place.contains("風物") {
                format!("在{}整理修補古卷與裝訂風物冊頁", place)
            } else if place.contains("巧木") || place.contains("傢俱") {
                format!("在{}推刨整木與試拼榫卯", place)
            } else if place.contains("雲錦") || place.contains("織造") || place.contains("服飾") {
                format!("在{}理線穿梭與比對布料染色", place)
            } else if let Some(v) = crate::grid::resolve_room_venue_object(place, "")
                .or_else(|| crate::grid::resolve_market_or_specialty_room_object(place, ""))
            {
                format!("在{}（{}）", place, v.on_duty_fixture_activity)
            } else {
                format!("在{}專注勞作", place)
            }
        }
        IntentType::SeekJob => format!("在{}打聽差事", place),
        IntentType::Trade => format!("在{}盤點貨物並交易", place),
        IntentType::Beg => format!("在{}向過路人求助", place),
        IntentType::Wander => {
            if place.contains("講堂街") || place.contains("蒙學") || place.contains("書院") || place.contains("學院") {
                format!("在{}翻閱新印的書冊並信步漫步", place)
            } else if place.contains("分家巷") || place.contains("百工") || place.contains("鐵器") {
                format!("在{}端詳新打好的鐵器並流連各家工坊", place)
            } else if place.contains("三水口") || place.contains("商行") {
                format!("在{}核對商號貨單並觀看往來商隊", place)
            } else if place.contains("西門") || place.contains("聯運") || place.contains("車馬") {
                format!("在{}查看聯運駝獸車班並向車伕打聽路況", place)
            } else if place.contains("食堂") {
                format!("在{}聞著熱騰騰飯香挑選餐點", place)
            } else if place.contains("浴場") {
                format!("在{}感受溫暖水氣並準備沐浴", place)
            } else if place.contains("書閣") || place.contains("藏書") {
                format!("在{}輕步穿梭書架翻閱古籍圖錄", place)
            } else if place.contains("公議堂") {
                format!("在{}細閱告示欄所張貼的規章布告", place)
            } else if place.contains("石橋") || place.contains('橋') {
                format!("在{}憑欄遠眺川流水色", place)
            } else if let Some(ambient) = crate::grid::resolve_room_ambient_visitor_activity(place, 0) {
                format!("在{}（{}）", place, ambient)
            } else {
                format!("在{}漫步巡看四周市井", place)
            }
        }
        IntentType::Idle => format!("在{}安靜休息", place),
    };

    format!("{}{}{}", ob_prefix, base, blocked_suffix)
}

pub(super) fn action_delay_seconds(intent: IntentType) -> i64 {
    match intent {
        IntentType::Work => 90,
        IntentType::Gather | IntentType::Trade => 35,
        IntentType::Socialize => 50,
        IntentType::Idle => 75,
        IntentType::SeekJob | IntentType::Beg | IntentType::Wander => 30,
    }
}

pub(super) fn place_name(graph: &PlaceGraph, place_id: &str) -> String {
    graph
        .place(place_id)
        .map(|place| crate::pg::living_world::peninsula_market_settlement_name(place.player_title().trim()))
        .filter(|name| !name.is_empty())
        .unwrap_or("相鄰地點")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obligation_prefixes_and_ambient_activity() {
        let moving = synthesize_moving_activity(
            IntentType::Work,
            "大圳頭水輪石磨坊",
            None,
            Some("craft_material"),
        );
        assert!(moving.contains("忙於籌備工坊料件，"));

        let moving_shortage = synthesize_moving_activity(
            IntentType::Trade,
            "三水口總商行",
            None,
            Some("fuel_shortage"),
        );
        assert!(moving_shortage.contains("忙於採買添補物資，"));

        let staying_wander = synthesize_staying_activity(
            IntentType::Wander,
            "大圳頭水輪石磨坊",
            None,
            None,
        );
        assert!(staying_wander.contains("大圳頭水輪石磨坊"));
        assert!(staying_wander.contains('（') || staying_wander.contains("漫步"));
    }
}
