//! 方格地圖容器（SquareGrid）與格子單元（GridCell）。
//!
//! - `SquareGrid` 持有 world_seed 及所有已揭露格子
//! - `GridCell` 為格子單元，含 `Terrain` 與 `GridObject`
//! - 揭露邏輯由 `reveal` 模組負責，此模組只管容器

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::cell::{Freshness, GridObject, Terrain};

use super::coord::{Direction, SquareCoord};

// ─── 顯式出口 ───

/// 顯式出口：方向 + 目標座標。村落格用這個取代座標鄰接自動計算。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExplicitExit {
    pub dir: Direction,
    pub target: SquareCoord,
}

// ─── 格子單元 ───

/// 方格地圖單元（格上資源與地形）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCell {
    pub coord: SquareCoord,
    pub terrain: Terrain,
    pub name: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zone: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// 格子內的地上物（資源、地標）
    #[serde(default)]
    pub objects: Vec<GridObject>,

    /// 是否已被玩家探索（影響前端渲染）
    #[serde(default)]
    pub explored: bool,

    /// 探索新鮮度（連續探索同一格的資源豐厚度衰減）
    #[serde(default)]
    pub freshness: Freshness,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explicit_exits: Vec<ExplicitExit>,
}

/// 判斷房間名稱是否為私人住宅、內室或宿舍（不應具備商業櫃檯或非本店人員禁止進入區）
pub fn is_private_residence_room_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains("・內室")
        || trimmed.ends_with("內室")
        || trimmed.contains("臥房")
        || trimmed.contains("寢室")
    {
        return true;
    }
    let base = trimmed.rsplit('・').next().unwrap_or(trimmed).trim();
    const RESIDENCE_SUFFIXES: &[&str] = &[
        "宅邸", "宅院", "家宅", "宗宅", "老宅", "青石宅", "石宅", "民宅", "民居", "民舍",
        "民家", "家屋", "宿所", "宿舍", "四合院", "石砌院", "鐵匠院", "小院", "書齋",
        "庖廚居", "湯屋居", "藥圃居", "算學居", "木作居", "繡坊居", "臨水居", "鍛藝居",
        "武勇舍", "老水夫舍", "總管居", "先生舍", "校勘齋", "墨香居", "木舍",
        "遠行行商棧舍", "行道車馬行舍", "聯運公秤舍", "草料客商居", "草料商旅居",
        "皮靴油布匠居", "刻書雕版匠居", "星曆舍", "推步居", "鋪居", "坊舍", "工匠居",
        "圃居", "員宅", "家舍", "儲糧宅", "織造家", "工寮",
    ];
    RESIDENCE_SUFFIXES.iter().any(|suf| base.ends_with(suf))
}

/// 判斷房間名稱是否為純公共街道、城門或街口（不應具備商業櫃檯或店鋪標籤）
pub fn is_street_or_gate_room_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    let base = trimmed.rsplit('・').next().unwrap_or(trimmed).trim();
    if matches!(base, "分家巷" | "西門街" | "講堂街" | "三水口") {
        return true;
    }
    const STREET_SUFFIXES: &[&str] = &[
        "北街", "南街", "東街", "西街", "前街", "大街", "商號街", "貨棧街", "石板街",
        "大道", "石道", "環北道", "環南道", "街口", "南口", "北口", "東口", "西口",
        "中段", "北段", "南段", "東段", "西段", "學院巷", "西北隅", "東北隅", "東南隅",
        "西南隅", "內門坊", "城門", "石徑", "石階",
    ];
    STREET_SUFFIXES.iter().any(|suf| base.ends_with(suf))
}

/// 清理分區字串中的純聚落／地名，避免聚落名稱讓全鎮所有道路與無關建築全部沾染單一資源標籤
fn strip_settlement_zone_names(zone: &str) -> String {
    let mut s = zone.to_string();
    for settlement in [
        "文曜", "風舶角", "風舶嶼", "大圳頭", "半嶺鎮", "半嶺", "南哨", "老堰頭",
        "北嶺腳", "北嶺", "霜林口", "風櫃尾", "烏石鼻", "牛埔", "紅土崁", "三號坑口",
        "三號坑", "水尾寮", "山坳鎮", "築港頭", "南灣",
    ] {
        s = s.replace(settlement, "");
    }
    s
}

/// 依據地形、房間名稱與分區推導房間語意標籤
pub fn derive_room_tags(terrain: Terrain, name: &str, zone: &str) -> Vec<String> {
    if terrain == Terrain::House || is_private_residence_room_name(name) {
        return vec!["住宅".to_string(), "民居".to_string()];
    }
    if is_street_or_gate_room_name(name) {
        return vec!["街道".to_string(), "道路".to_string()];
    }
    let mut tags = Vec::new();
    let mut add_tag = |t: &str| {
        if !tags.iter().any(|existing: &String| existing == t) {
            tags.push(t.to_string());
        }
    };

    let n = name;
    let z_clean = strip_settlement_zone_names(zone);
    let z = z_clean.as_str();
    // 糧倉、穀物、糧食、米、大圳、農田
    if matches!(
        terrain,
        Terrain::Granary
            | Terrain::FarmField
            | Terrain::Farmhouse
            | Terrain::DryField
            | Terrain::EmptyFurrow
    ) || n.contains('糧')
        || n.contains('穀')
        || n.contains('倉')
        || n.contains('米')
        || n.contains("大圳")
        || n.contains('農')
        || n.contains('田')
        || z.contains('糧')
        || z.contains('穀')
        || z.contains('米')
        || z.contains("大圳")
        || z.contains('農')
    {
        add_tag("穀物");
        if matches!(terrain, Terrain::Granary) || n.contains('倉') || n.contains('糧') {
            add_tag("糧倉");
            add_tag("櫃檯");
            add_tag("非本店人員禁止進入區");
        }
        if matches!(
            terrain,
            Terrain::FarmField | Terrain::Farmhouse | Terrain::DryField | Terrain::EmptyFurrow
        ) || n.contains('田')
            || n.contains('耕')
        {
            add_tag("農田");
        }
    }

    // 礦場、礦石、西嶺礦谷
    if matches!(terrain, Terrain::MinePass)
        || n.contains('礦')
        || n.contains("西嶺")
        || z.contains('礦')
        || z.contains("西嶺")
    {
        add_tag("礦場");
        add_tag("礦石");
        add_tag("櫃檯");
        add_tag("非本店人員禁止進入區");
    }

    // 林場、木材、霜林口
    if matches!(
        terrain,
        Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight
    ) || n.contains('林')
        || n.contains('木')
        || n.contains('杉')
        || n.contains("霜林")
        || z.contains('林')
        || z.contains('木')
        || z.contains("霜林")
    {
        add_tag("林場");
        add_tag("木材");
    }

    // 藥圃、藥材、醫館、百草、北山藥圃
    if matches!(
        terrain,
        Terrain::Clinic | Terrain::HerbGarden | Terrain::Alchemist
    ) || n.contains('藥')
        || n.contains('醫')
        || n.contains('診')
        || n.contains('草')
        || z.contains('藥')
        || z.contains('醫')
        || z.contains('草')
    {
        add_tag("藥圃");
        add_tag("藥材");
        add_tag("櫃檯");
        add_tag("非本店人員禁止進入區");
    }

    // 獵場、皮革、鹿皮、長風草甸
    if matches!(terrain, Terrain::Jungle)
        || n.contains('獵')
        || n.contains('皮')
        || n.contains('鹿')
        || n.contains("草甸")
        || z.contains('獵')
        || z.contains('皮')
        || z.contains("草甸")
    {
        add_tag("獵場");
        add_tag("皮革");
    }

    // 漁港、海鹽、鮮魚、南灣
    if matches!(
        terrain,
        Terrain::Dock | Terrain::Beach | Terrain::Water | Terrain::WaterDeep
    ) || n.contains('漁')
        || n.contains('港')
        || n.contains('鹽')
        || n.contains('海')
        || n.contains("南灣")
        || z.contains('漁')
        || z.contains('港')
        || z.contains('鹽')
        || z.contains('海')
        || z.contains("南灣")
    {
        add_tag("漁港");
        add_tag("海鹽");
        add_tag("櫃檯");
        add_tag("非本店人員禁止進入區");
    }

    // 織坊、布料、裁縫、文曜織坊
    if n.contains('織')
        || n.contains('布')
        || n.contains('染')
        || n.contains('縫')
        || z.contains('織')
        || z.contains('布')
        || z.contains('染')
    {
        add_tag("織坊");
        add_tag("布料");
        add_tag("工坊");
        add_tag("櫃檯");
        add_tag("非本店人員禁止進入區");
    }

    // 鐵匠鋪、鍛造、工坊、磨石、老堰頭水輪石磨
    if matches!(terrain, Terrain::Blacksmith | Terrain::Workshop)
        || n.contains('鐵')
        || n.contains('鍛')
        || n.contains("工坊")
        || n.contains("石磨")
        || n.contains("水輪")
        || n.contains("老堰頭")
        || z.contains('鐵')
        || z.contains('鍛')
        || z.contains("工坊")
        || z.contains("老堰頭")
    {
        if matches!(terrain, Terrain::Blacksmith) || n.contains('鐵') || n.contains('鍛') {
            add_tag("鐵匠鋪");
        }
        add_tag("工坊");
        add_tag("磨石");
        add_tag("櫃檯");
        add_tag("非本店人員禁止進入區");
    }

    // 食堂、熱食、現煮飲品、客棧、茶居、三水大食堂
    if matches!(terrain, Terrain::Tavern | Terrain::Inn)
        || n.contains("食堂")
        || n.contains('餐')
        || n.contains('飯')
        || n.contains('茶')
        || n.contains('酒')
        || n.contains("客棧")
        || n.contains('飲')
        || z.contains("食堂")
        || z.contains('餐')
        || z.contains('茶')
        || z.contains('酒')
    {
        add_tag("食堂");
        add_tag("熱食");
        add_tag("現煮飲品");
        add_tag("櫃檯");
        add_tag("非本店人員禁止進入區");
    }

    // 商鋪、市集
    if matches!(
        terrain,
        Terrain::GeneralStore | Terrain::Market | Terrain::Caravanserai | Terrain::Bank
    ) || n.contains('商')
        || n.contains('鋪')
        || n.contains("市集")
        || n.contains('店')
        || n.contains('行')
        || z.contains('商')
        || z.contains("市集")
    {
        add_tag("商鋪");
        add_tag("市集");
        add_tag("櫃檯");
        add_tag("非本店人員禁止進入區");
    }

    // 水脈、分洪閘、清泉、堰渠
    if n.contains("水脈")
        || n.contains("清泉")
        || n.contains('閘')
        || n.contains('渠')
        || n.contains('堰')
        || z.contains("水脈")
        || z.contains("清泉")
        || z.contains('閘')
        || z.contains('渠')
        || z.contains('堰')
    {
        add_tag("水脈");
        add_tag("分洪閘");
    }

    // 浴場、湯屋、泉香
    if n.contains('浴') || n.contains('湯') || n.contains("泉香") || z.contains('浴') {
        add_tag("浴場");
    }

    // 茶屋
    if matches!(terrain, Terrain::Tavern) || n.contains('茶') || z.contains('茶') {
        add_tag("茶屋");
    }

    // 旅社、驛站、歇舍
    if matches!(terrain, Terrain::Inn)
        || n.contains("旅社")
        || n.contains("驛")
        || n.contains("通鋪")
        || n.contains("歇舍")
    {
        add_tag("旅社");
    }

    // 水井、山泉、清泉
    if n.contains('井') || n.contains("山泉") || n.contains("清泉") {
        add_tag("水井");
    }

    // 聯運、碼頭、渡口
    if matches!(terrain, Terrain::Dock | Terrain::Caravanserai)
        || n.contains("聯運")
        || n.contains("貨運")
        || n.contains("碼頭")
        || n.contains("渡口")
        || n.contains("船塢")
        || n.contains("車馬")
    {
        add_tag("聯運");
        add_tag("碼頭");
    }

    // 石橋、廣場、樞紐
    if matches!(terrain, Terrain::Bridge)
        || n.contains("石橋")
        || n.contains("樞紐")
        || n.contains("石坪")
        || n.contains("廣場")
    {
        add_tag("石橋");
        add_tag("廣場");
    }

    tags
}

/// 房間物件化契約（RoomVenueObject）：
/// 將每個房間視為具備自身語意標籤、固定設施狀態、常備外借器物插座與當值職司的一等物件，
/// 避免各子系統（借物、移動視野、事件劇本）各自散落字串猜測。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomVenueObject {
    pub venue_kind: &'static str,
    pub default_host_title: &'static str,
    pub has_counter_restricted_zone: bool,
    pub on_duty_fixture_activity: &'static str,
    pub on_duty_demeanor: &'static str,
    pub lendable_tool: Option<(&'static str, &'static str, &'static str)>,
    pub counter_trade_offers: &'static [(&'static str, &'static str, i32)],
    pub trade_talk_pitch: &'static str,
    pub on_duty_talk_lines: &'static [&'static str],
}

impl RoomVenueObject {
    /// 提取場域物件之核心語意標籤
    pub fn semantic_tags(&self) -> Vec<&'static str> {
        let mut tags = Vec::new();
        let k = self.venue_kind;
        if k.contains("工坊") || k.contains("作坊") || k.contains("坊") || k.contains("窯") || k.contains("鍛") || k.contains("木作") || k.contains("造") {
            tags.push("工坊");
        }
        if k.contains("鐵") || k.contains("鍛") {
            tags.push("鐵匠鋪");
        }
        if k.contains("織") || k.contains("染") {
            tags.push("染坊");
        }
        if k.contains("磨") {
            tags.push("磨石");
        }
        if k.contains("礦") || k.contains("石") {
            tags.push("礦場");
        }
        if k.contains("木") || k.contains("林") {
            tags.push("林場");
        }
        if k.contains("鋪") || k.contains("行") || k.contains("店") || k.contains("棧") || k.contains("商") {
            tags.push("商鋪");
        }
        if k.contains("市") || k.contains("集") || k.contains("拍賣") {
            tags.push("市集");
        }
        if k.contains("食") || k.contains("飯") || k.contains("湯餅") {
            tags.push("食堂");
        }
        if k.contains("茶") {
            tags.push("茶屋");
        }
        if k.contains("旅") || k.contains("客") || k.contains("驛") {
            tags.push("旅社");
        }
        if k.contains("浴") || k.contains("溫泉") {
            tags.push("浴場");
        }
        tags
    }
}

/// 依據房間名稱與補充提示（如分區），確定性解析房間物件契約
pub fn resolve_room_venue_object(room_name: &str, extra_hint: &str) -> Option<RoomVenueObject> {
    if is_private_residence_room_name(room_name) || is_street_or_gate_room_name(room_name) {
        return None;
    }
    let combined = format!("{room_name} {extra_hint}");
    let room_tags = derive_room_tags(Terrain::Road, room_name, extra_hint);
    if (room_tags.iter().any(|t| t == "醫館" || t == "藥圃")
        || ["診所", "醫館", "醫師"].iter().any(|k| combined.contains(k)))
        && !combined.contains("教務長")
    {
        return Some(RoomVenueObject {
            venue_kind: "醫館藥局",
            default_host_title: "當值醫師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於青石藥案前調配青鹽薄荷潔牙粉與水苔敷藥",
            on_duty_demeanor: "身形纖細而步履輕快，神情溫柔專注，隨身配掛分格藥囊與百草溫灸盒，衣袍間常帶著淡淡的山風草與清涼薄荷香氣。",
            lendable_tool: Some((
                "wt_cat_1057",
                "溫氏導能溫石百草溫灸盒",
                "診所常備的「溫石百草溫灸盒」借你隨身攜帶溫熨穴位，出入風露寒濕之地可保氣血安和，用畢記得歸還。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1065", "雪蓮活絡跌打膏", 8),
                ("wt_cat_1064", "巡山獵人隨身急救皮囊", 12),
            ],
            trade_talk_pitch: "「隨身防潮急救皮囊裡配齊了止血水苔、止痛敷散與辟穢草油，出門必備，這兒隨時可替你備上一副。」",
            on_duty_talk_lines: &[
                "「請慢些坐下。容我先看一眼舌苔與脈息……氣血略有虧虛，可是在外奔波受了山風涼氣？」",
                "「山風草配北山清泉熬成的舒筋藥膏最能拔除關節寒濕。凡進山涉水之人，隨身備上一盒總有益處。」",
                "「採藥要懂天時，雨前收葉、霜後挖根。診所裡的藥材皆是在北嶺和黑沼外緣採集精選的。」",
            ],
        });
    }

    // 2. 木作/巧木/傢俱
    if ["巧木", "木作", "傢俱", "木匠"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "木作工坊",
            default_host_title: "當值木匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正用省力導能刨刀推平松香木板",
            on_duty_demeanor: "身型精瘦的老木工師傅，穿著沾有細碎木屑的耐磨工作服，神情專注親切，隨身工具皮帶上插著各式精巧的直角尺與導能木刨。",
            lendable_tool: Some((
                "wt_cat_0956",
                "柯氏松木家用工具箱",
                "這口「松木家用工具箱」裡頭鑿子、羊角錘與木尺齊全，你先提去修繕器物，用完擦淨送回工坊便是。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0559", "寒水玉雙層保鮮石櫃", 35),
                ("wt_cat_0723", "旋蓋夜光貝母調光台燈", 15),
                ("wt_cat_0795", "烏木黑子白貝圍棋盤", 20),
                ("wt_cat_0622", "防靜電松香木梳", 5),
            ],
            trade_talk_pitch: "「寒水玉雙層保鮮石櫃、旋蓋夜光貝母台燈、烏木白貝圍棋盤與松香木梳都在架上，防潮防蠹，看中自可帶走。」",
            on_duty_talk_lines: &[
                "「咔嗒！聽這榫卯合榫的一聲脆響，嚴絲合縫，不著一釘一鉚，這便是百年木器的底氣！」",
                "「北山雲杉木性柔韌、松香良木氣味避蟲，打成箱櫃百年生香。挑木料如相人，木紋正，器型才立得正。」",
                "「做傢俱急不得，木頭需陰乾三年才可下刨。摸一摸就知含水幾分。」",
            ],
        });
    }

    // 3. 火磚/測紋/器脈工坊（不得以「分家巷」街名通吃整條街的武館、雜貨與染坊）
    if ["測紋", "火磚", "器脈工坊"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "導能器脈工坊",
            default_host_title: "當值匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正用游標卡尺測量導能火磚嵌合縫隙",
            on_duty_demeanor: "年輕幹練的器脈技師，穿著耐磨工作短衫，體態勻稱結實，神情專注認真，腰間別著黃銅測尺與導能紋板。",
            lendable_tool: Some((
                "wt_cat_0735",
                "導能測紋石板",
                "這塊「導能測紋石板」嵌有微光標準刻度，借你拿去比對器物與火磚紋路，切記輕拿輕放。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0611", "岑氏合金黃銅自來水栓", 12),
                ("wt_cat_0956", "柯氏松木家用工具箱", 14),
                ("wt_cat_1010", "卡式導能便攜爐", 16),
                ("wt_cat_0735", "導能測紋石板", 15),
            ],
            trade_talk_pitch: "「前案擺著新造的導能測紋石板、卡式便攜爐、黃銅水栓與松木家用工具箱，皆是民生百工合用器物，隨意挑選。」",
            on_duty_talk_lines: &[
                "「器物脈絡精密入微。卡尺多移一絲、刻紋深淺錯一毫，導能火磚的蓄熱便差之千里。」",
                "「民生所用器具最講究耐用可靠。每一組水栓與便攜爐在出坊前都需經過三道冷熱測壓。」",
                "「鑽研器物手藝看似枯燥，實則處處玄機。若是對器脈結構有興趣，多看看這塊測紋石板自能心領神會。」",
            ],
        });
    }

    // 4. 鍛造/鐵匠鋪
    if room_tags.iter().any(|t| t == "鐵匠鋪" || t == "鍛造")
        || ["第一鐵坊", "百鍛", "鍛造", "鐵匠", "打鐵"].iter().any(|k| combined.contains(k))
    {
        let is_apprentice_or_tech = combined.contains("青年") || combined.contains("技師") || combined.contains("學徒");
        let offers: &'static [(&'static str, &'static str, i32)] = if is_apprentice_or_tech {
            &[
                ("wt_cat_0611", "岑氏合金黃銅自來水栓", 12),
                ("wt_cat_0956", "柯氏松木家用工具箱", 14),
                ("wt_cat_1010", "卡式導能便攜爐", 16),
                ("wt_cat_0735", "導能測紋石板", 15),
            ]
        } else {
            &[
                ("wt_cat_1021", "岩羊角手柄折疊獵刀", 18),
                ("wt_cat_0957", "鍛鋼小羊角錘", 10),
            ]
        };
        let demeanor = if is_apprentice_or_tech {
            "年輕幹練的鍛造技師，穿著耐磨工作短衫，體態勻稱結實，神情專注認真，眉宇間帶著鑽研新器物的朝氣與執著，指節滿是敲打金屬的厚繭。"
        } else {
            "滿面虯髯、粗獷豪邁的大匠師，露出結實粗壯的手臂，繫著厚牛皮防燙圍裙，眼神如爐火般熾烈堅定，渾身散發著炭火與金屬的沉雄氣息。"
        };
        let pitch = if is_apprentice_or_tech {
            "「前案擺著新造的導能測紋石板、卡式便攜爐、黃銅水栓與松木家用工具箱，皆是民生百工合用器物，隨意挑選。」"
        } else {
            "「精鍛護手獵刀與羊角鍛錘就擺在木架上，刀身冷光如霜，拿在手裡沉實合手，相中哪件儘管開口！」"
        };
        let lines: &'static [&'static str] = if is_apprentice_or_tech {
            &[
                "「這柄獵刀特意加了雙道放血槽，護手用黃銅配重，劈砍時手腕極省力。你覺得握感如何？」",
                "「做鐵匠看似粗笨，實則心細如髮。淬火時水溫多燙一度、火色淺淡一分，成品的硬度與韌性便差之千里。」",
                "「前街新運來的合金黃銅水栓已按標準修磨完成，出水順暢不漏滴，是每戶家屋修繕必備之物。」",
            ]
        } else {
            &[
                "「呼！火爐炭氣嗆人，退後兩步！三號坑精鐵質地堅硬，需借萬斤水錘反覆折疊鍛打，才能逼出雜質。」",
                "「這座鍛造爐打過百工翻土犁，也鍛過各鎮獵隊的斬木刀。手藝不欺人，鐵砧上見真章！」",
                "「打鐵講求火候精準，新式水槽淬火鍛出的獵刀韌性倍增，百煉精鋼才經得起山林考驗。」",
            ]
        };
        return Some(RoomVenueObject {
            venue_kind: "百鍛鐵坊",
            default_host_title: "當值鐵匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正指導學徒查看高溫紅鐵爐的火色",
            on_duty_demeanor: demeanor,
            lendable_tool: Some((
                "wt_cat_0957",
                "鍛鋼小羊角錘",
                "這柄「鍛鋼小羊角錘」錘頭厚實趁手，你先借去敲打修整，莫讓雨水鏽了錘面。",
            )),
            counter_trade_offers: offers,
            trade_talk_pitch: pitch,
            on_duty_talk_lines: lines,
        });
    }

    // 5. 鐘樓/司鐘
    if ["鐘樓", "司鐘"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "傳訊鐘樓",
            default_host_title: "當值司鐘",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正校對旋轉日規與報時青銅巨鐘並核驗快騎郵袋",
            on_duty_demeanor: "年過半百的鐘樓司鐘長，穿著深褐色耐磨工作衫，身形挺拔，斑白短髮梳理得一絲不苟，神情嚴謹，口袋裡放著一枚校時極準的黃銅發條懷錶。",
            lendable_tool: Some((
                "wt_cat_0729",
                "校準隨身黃銅懷錶",
                "這枚「校準隨身黃銅懷錶」已對準鐘樓正午日影，借你隨身掌握時辰，莫誤了潮汐與車班。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0729", "校準隨身黃銅懷錶", 18),
                ("wt_cat_0728", "岑氏黃銅發條座鐘", 24),
                ("wt_cat_0794", "紅松脂封蠟行會委託單", 6),
            ],
            trade_talk_pitch: "「案頭備有三水口校時黃銅懷錶、岑氏黃銅發條座鐘與紅松脂封蠟委託單，皆走時極準，任君挑選。」",
            on_duty_talk_lines: &[
                "「鐘樓一日鳴鐘六次，辰正開市、戌正收更，半刻不容差池。全城作息信賴此音，守這口銅鐘已歷三十載。」",
                "「樓下快騎驛使日夜馳騁，一袋緊急手信關係百里外農桑商訊。凡入鐘樓者，須知惜時如金。」",
                "「發條齒輪與旋轉日晷乃鐘樓之骨髓，日日需添山茶油養護，才能轉動無聲、報時準確。」",
            ],
        });
    }

    // 6. 藏書閣/印書/書局
    if !combined.contains("契書")
        && ["藏書閣", "印書", "旬報", "書局", "主編"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "藏書印書館",
            default_host_title: "當值編修",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正校勘新印出的《白塔風物誌》與《半島旬報》",
            on_duty_demeanor: "清麗素雅的年輕編修，身穿立領長版長衫，神情專注沉靜，眉眼溫婉明慧，袖口微挽，指尖帶著校對書稿與排印圖冊留下的淡淡油墨香。",
            lendable_tool: Some((
                "wt_cat_0784",
                "《白塔風物誌》",
                "這冊《白塔風物誌》詳載半島四鎮十九獵場山川風物，借你展讀參照，切勿折損書頁。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0784", "《白塔風物誌》", 8),
                ("wt_cat_0786", "每旬發行《半島旬報》", 3),
                ("wt_cat_0780", "彈性精鋼尖蘸水硬筆", 10),
                ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
            ],
            trade_talk_pitch: "「門前書案備有竹綿紙精印的《白塔風物誌》與每旬發行之《半島旬報》，亦有黃銅蘸水硬筆，皆可按公價結算。」",
            on_duty_talk_lines: &[
                "「你也是來查閱古卷的麼？閣內收羅了先祖渡海以來的四方碑拓與各地採風錄，請輕聲慢步，莫驚動滿室書香。」",
                "「新一期《白塔風物誌》新編纂完稿，記載了北嶺地脈與白茫洲水文，讀來頗多意趣。」",
                "「字墨有靈，能載山川興衰。若你在外鄉探尋到奇聞異草或古碑拓片，隨時可送來藏書閣一同校勘。」",
            ],
        });
    }

    // 7. 算學/商算/會計
    if ["算學", "商算", "會計"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "商算學館",
            default_host_title: "當值算師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正操作隨身計數板核算大宗行單",
            on_duty_demeanor: "精明幹練的百工核算師，穿著剪裁合身的深灰棉布外套，面容清瘦，眼神銳利，腰間掛著硬革資料袋，隨身攜帶一具精準計數板。",
            lendable_tool: Some((
                "wt_cat_0788",
                "烏木便攜計數板",
                "這塊「烏木便攜計數板」算珠滑順分明，借你外出盤點貨數與帳目，核完記得歸還。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0788", "烏木便攜計數板", 14),
                ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
                ("wt_cat_0790", "三水總商行鎂幣分格存摺", 12),
            ],
            trade_talk_pitch: "「工坊精製的計數板與帳本清冊皆在架上，手感極佳，算師學徒皆好此物，需要便取去。」",
            on_duty_talk_lines: &[
                "「精準計算分毫不爽，百工商算學舍講求的是實數實利。左手出海、右手歸倉，帳目清爽，心神始得安定。」",
                "「各工坊每季器物耗料與出貨工錢皆在此匯總。數字不會騙人，經營之道全在精細籌謀。」",
                "「若想研習行商簿記要覽，先學會心口如一、落筆有節。核算嚴謹，始見功底深厚。」",
            ],
        });
    }

    // 8. 契書局 / 房地契 / 戶政 / 屋契
    if ["契書", "房地契", "戶政", "屋契"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "地籍契書司",
            default_host_title: "當值錄事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於案前核對《半島全戶總譜》與紅印屋契",
            on_duty_demeanor: "端正嚴謹的青年地政登記員，身穿深藍色當值夾袍，腰板挺直，行事一絲不苟，袖口整齊扣好，桌邊整齊擺放著紅印屋契與吸墨器。",
            lendable_tool: Some((
                "wt_cat_0816",
                "雙面雕刻銅製驗金天平",
                "這具「雙面雕刻銅製驗金天平」分毫不差，借你核驗大宗交割成色，用畢務必歸還公案。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0793", "公議堂紅印屋契與租約摺", 25),
                ("wt_cat_0821", "玻璃螺紋瓶裝黑墨水", 6),
            ],
            trade_talk_pitch: "「案前備有標準公議堂紅印屋契與租約摺，亦有專用螺紋瓶黑墨水，字跡歷數十年不退色，隨時可辦理開具。」",
            on_duty_talk_lines: &[
                "「請出示身份憑信與定居條目。凡落籍文曜坊市者，必先登載於《半島全戶總譜》，朱墨畫押，留底備查。」",
                "「屋契乃立身安居之憑券。加蓋公議堂硃砂鋼印與騎縫花押，受全城通事保護，不可偽冒，不可毀損。」",
                "「契約條款皆白紙黑字寫得明白。田土面積、界石坐標、租期稅賦，一筆不差，此乃文曜立城之基石。」",
            ],
        });
    }

    // 9. 公議堂（不含契書）
    if combined.contains("公議堂") {
        return Some(RoomVenueObject {
            venue_kind: "公議堂民政處",
            default_host_title: "當值調解員",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正手持木牌巡看街坊水渠與公議告示",
            on_duty_demeanor: "辦事幹練圓融的公議堂民務通事，身穿體面的深赭色排扣外套，留著整齊短鬚，笑容可掬，口袋裡常備著幾份街區公告與隨身記事本。",
            lendable_tool: Some((
                "wt_cat_0816",
                "雙面雕刻銅製驗金天平",
                "這具「雙面雕刻銅製驗金天平」分毫不差，借你核驗大宗交割成色，用畢務必歸還公案。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0793", "公議堂紅印屋契與租約摺", 25),
                ("wt_cat_0786", "每旬發行《半島旬報》", 3),
            ],
            trade_talk_pitch: "「公議堂紅印屋契與租約摺、每旬發行之《半島旬報》皆在案頭，手續合規即可開印簽發。」",
            on_duty_talk_lines: &[
                "「哎呀你好！在文曜安家落籍或有公事調解，找公議堂就對了！四坊巷陌每塊界石、每張房契，我都熟記於心。」",
                "「若是看中了西門或分家巷的石砌家屋，先簽了紅印屋契，領取黃銅門牌鑰匙，隨時便能生火入厝！」",
                "「各坊若有修路引水或市集攤位爭執，皆在公議堂擺平調和。和氣生財，安居樂業最重要。」",
            ],
        });
    }

    // 10. 總商行 / 商行 / 通兌
    if ["總商行", "商行", "通兌"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "商行通兌總處",
            default_host_title: "當值掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於樟木掌事台前核驗跨鎮通兌匯票",
            on_duty_demeanor: "老掌事年事已高卻精神矍鑠，穿著整潔挺括的正裝外套，神色和藹沉穩，眉宇間透著數十年商界歷練的從容大氣，手握一枚黃銅印章。",
            lendable_tool: Some((
                "wt_cat_0816",
                "雙面雕刻銅製驗金天平",
                "這具「雙面雕刻銅製驗金天平」分毫不差，借你核驗大宗交割成色，用畢務必歸還公案。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0790", "三水總商行鎂幣分格存摺", 12),
                ("wt_cat_0892", "軟牛皮分格零錢夾", 8),
            ],
            trade_talk_pitch: "「櫃房新印了一批加厚牛皮存摺與防水鎂幣夾，皆蓋有總商行鋼印，經久耐磨，隨身攜帶最是妥帖。」",
            on_duty_talk_lines: &[
                "「年輕人有幹勁。三水口匯聚半島四方客商，總商行運轉數十年，最看重的便是『信義』二字。坐下喝杯茶吧。」",
                "「總商行發行之分格存摺與防偽匯票，在半島四鎮分號皆通存通兌。手頭有餘錢不妨在此開戶，出門在外才能踏實。」",
                "「商貿如春水，流通則活，淤塞則弊。你若在各鎮往來，多留意各地物價物候，處處皆是學問。」",
            ],
        });
    }

    // 11. 蒙學/講舍/講習
    if ["蒙學", "講舍", "講習"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "蒙學學舍",
            default_host_title: "當值教習",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正敲響響板帶領學生朗讀基礎教材",
            on_duty_demeanor: "面相斯文溫和的塾館先生，穿著深青色剪裁外套，短髮梳理整齊，身形清瘦挺拔，語調溫厚平和，看著學舍裡的學童時目光充滿期許。",
            lendable_tool: Some((
                "wt_cat_0782",
                "蒙學基礎識字課本",
                "這本「蒙學基礎識字課本」字跡工整清晰，借你拿去溫習識字，讀完記得放回講堂書架。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0782", "蒙學基礎識字課本", 6),
                ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
                ("wt_cat_0780", "彈性精鋼尖蘸水硬筆", 10),
            ],
            trade_talk_pitch: "「堂內備有蒙學基礎識字課本、竹綿紙牛皮記事手札與精鋼尖蘸水硬筆，字跡端正明晰，文友若需自可兌購。」",
            on_duty_talk_lines: &[
                "「基礎教育，乃立世之基。引導孩童端正執筆、朗讀讀本，看著那一張張純真面龐漸知明理，便是我平生最大快事。」",
                "「文曜乃半島文脈所繫，學童自幼誦讀渡海先烈開闢山川之誌，知曉粒米皆出稼穡之辛，長大後才能成為社會棟樑。」",
                "「你看起來見多識廣，若願在堂前為學童講述幾句外鄉山川險奇、行路見聞，必能開拓孩子們的視野。」",
            ],
        });
    }

    // 12. 文曜學院/講堂/教務長（非蒙學/講舍）
    if ["學院", "講堂", "文曜學院", "教務長"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "文曜講堂",
            default_host_title: "當值教習",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正於講堂案前批閱學生呈交的脈絡札記",
            on_duty_demeanor: "面容清瘦斯文的長者，鬢髮微霜，穿著水墨月白色剪裁外套，步履沉穩優雅，目光溫和睿智，展現出主持學院教務的學者氣度。",
            lendable_tool: Some((
                "wt_cat_1057",
                "溫氏導能溫石百草溫灸盒",
                "講堂常備的「溫石百草溫灸盒」可供長坐讀書者溫熨穴位，舒緩筋絡，閱畢記得歸還原處。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0784", "《白塔風物誌》", 8),
                ("wt_cat_0780", "彈性精鋼尖蘸水硬筆", 10),
            ],
            trade_talk_pitch: "「學院墨寶皆用竹綿紙印刷，亦有學生常備之黃銅蘸水硬筆與溫潤墨水，文雅精美，可隨緣取閱。」",
            on_duty_talk_lines: &[
                "「學院講究的是把原理弄懂、落實到生活裡。半島立城以來，正是靠著公開教學與技術累積，社會才能一直進步。」",
                "「學生如果只會背書而不懂實際產銷與實務，終究只是空談。在藏書閣編纂風物誌、在診所看診，都是學以致用。」",
                "「初來文曜嗎？如果對學院的基礎課程或各地人文有興趣，隨時可以詢問。」",
            ],
        });
    }

    // 13. 修傘/補靴
    if ["修傘", "補靴"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "修傘補靴鋪",
            default_host_title: "當值鞋傘匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正在硬木楦頭上釘補防浸皮靴並為竹骨傘刷上熟桐油",
            on_duty_demeanor: "寡言沉穩的修配老匠師，圍著浸過蜂蠟與桐油的厚帆布工作圍裙，雙手厚實有力，腿上墊著修鞋木楦頭，專注地修補每一雙皮靴與晴雨摺疊傘。",
            lendable_tool: Some((
                "wt_cat_0896",
                "韌竹骨桐油晴雨摺疊傘",
                "外頭山海天候多變，這把刷足桐油的「韌竹骨桐油晴雨摺疊傘」借你遮風擋雨，回城時順路還來即可。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0896", "韌竹骨桐油晴雨摺疊傘", 8),
                ("wt_cat_0843", "熟成防水牛皮短靴", 15),
            ],
            trade_talk_pitch: "「厚魚油防浸牛皮行路靴與韌竹骨桐油摺疊晴雨傘皆是現貨，尺碼齊備，過客穿上即可啟程，防風防雨保平安。」",
            on_duty_talk_lines: &[
                "「鞋底磨薄了可不行。西門出去走長路，碎石灘和泥濘路最傷腳骨，加釘一層熟牛皮掌，穩健耐磨能多走千里路。」",
                "「傘骨要選北山老竹，浸透桐油的厚棉紙經得起狂風掀扯。晴可遮烈日、雨可擋暴注，這才叫晴雨傘。」",
                "「這蜂蠟松脂防水膏是老配方，靴縫抹上一圈，踩進積水窪半點不滲。手藝人不做虛活。」",
            ],
        });
    }

    // 14. 柑仔店/雜貨
    if ["柑仔店", "雜貨"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "街角雜貨鋪",
            default_host_title: "當值掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正擦拭廣口玻璃柑仔糖罐並清點紅磷火籤與蚊香",
            on_duty_demeanor: "慈眉善目的白髮阿婆，穿著寬鬆舒適的碎花短衫，坐在藤椅上微笑，眼神溫和，手裡拿著蒲扇，身邊散發著陳皮、麥芽糖與曬乾艾草的熟悉香氣。",
            lendable_tool: Some((
                "wt_cat_0898",
                "防潮紅磷引火籤盒",
                "這盒「防潮紅磷引火籤盒」封蠟完好，出門在外升火照明最要緊，先拿去應急吧，店裡備得多得是！",
            )),
            counter_trade_offers: &[
                ("wt_cat_0514", "柑仔店古早黑糖麥芽糖", 3),
                ("wt_cat_0898", "防潮紅磷引火籤盒", 3),
            ],
            trade_talk_pitch: "「廣口玻璃罐裡的酸梅糖、三號坑防潮紅磷引火籤盒、夏季驅蚊艾草蚊香盤都在這，零錢隨意付，都是街坊價。」",
            on_duty_talk_lines: &[
                "「哎唷，年輕人肚子餓了沒？來含一顆金桔酸梅糖，酸甜生津，解渴又提神，自己用土冰糖醃的哩！」",
                "「這條巷子看了幾十年囉，誰家添丁、誰家置產過契，心裡像明鏡似的。歇歇腳再走吧！」",
                "「出遠門隨身裝一盒引火籤，夜裡露營不怕潮氣。缺零碎什物儘管來找阿婆。」",
            ],
        });
    }

    // 15. 旅社/茶屋/茶館
    if ["旅社", "茶屋", "茶館"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "茶屋旅社",
            default_host_title: "當值茶博士",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於臨水茶席沏起頭道山風草茶並擺設棋局",
            on_duty_demeanor: "眉目清俊、氣質溫和的旅社負責人，穿著雪青色休閒外套，神態安詳從容，說話不疾不徐，泡茶手法俐落熟練。",
            lendable_tool: Some((
                "wt_cat_0723",
                "旋蓋夜光貝母調光台燈",
                "夜路與房內照明少不了燈火，這盞「旋蓋夜光貝母調光台燈」柔潤不刺眼，借你掌燈夜讀或巡行。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0500", "山風草炭焙烘青茶", 5),
                ("wt_cat_0501", "穩火慢煮醇厚鮮奶茶", 5),
                ("wt_cat_0723", "旋蓋夜光貝母調光台燈", 15),
            ],
            trade_talk_pitch: "「東橋茶屋特選山風草烘青茶葉包、熱煮鮮奶茶皆可裝壺帶走，客房旋蓋夜光貝母調光台燈亦有全新行裝版可售。」",
            on_duty_talk_lines: &[
                "「請入座。這壺鍋煮鮮奶茶用了牛埔原牛乳與霜林烘青茶，火候慢煨，茶香醇厚奶香濃郁，最宜撫慰羈旅之情。」",
                "「東橋乃文曜咽喉，南北商旅皆在此歇腳。一局圍棋、半盞清茶，天下山海風物盡在言笑之中。」",
                "「夜深若要展讀或寫信，櫃檯備有旋蓋夜光台燈，光線柔和，不擾同歇客旅。」",
            ],
        });
    }

    // 16. 大食堂/食堂/廚房
    if room_tags.iter().any(|t| t == "食堂" || t == "熱食")
        || ["大食堂", "食堂", "廚房", "大廚", "主廚"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "大食堂",
            default_host_title: "食堂主廚",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正守在紅鐵大釜前慢火熬煮原牛肉湯",
            on_duty_demeanor: "體態圓潤的食堂主廚，臉上總是掛著熱情的笑容，繫著寬大的白色廚師圍裙，捲起袖子手持大湯勺，周身帶著原牛高湯與蔥香熱油的濃郁香氣。",
            lendable_tool: Some((
                "wt_cat_0897",
                "薄層重礦烏木膽隨身保溫瓶",
                "這只「薄層重礦烏木膽隨身保溫瓶」能蓄熱六個時辰，借你裝熱湯熱水帶在路上喝，回來再洗淨還我。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0444", "厚釜慢燉原牛肉湯", 5),
                ("wt_cat_0388", "現磨溫米漿", 3),
                ("wt_cat_0390", "黑糖薑汁熱豆花", 4),
            ],
            trade_talk_pitch: "「大釜熬原牛骨高湯、現磨溫米漿、木桶鹽滷嫩豆花皆滾燙備妥，亦可打入保溫瓶帶走，幾枚鎂幣管飽管暖！」",
            on_duty_talk_lines: &[
                "「哈哈！快往裡坐！聞到這鑊氣沒有？紅鐵厚底大釜慢火煨燉的大角原牛骨高湯，整整滾了一晝夜！」",
                "「開食堂四十年，本店就信一條：做飯要捨得下料，米要新米、肉要鮮肉，街坊行商吃得滿意，生意自然紅火！」",
                "「清晨出發前喝一碗現磨溫米漿，胃裡踏實暖和，頂風冒雪走幾十里路都不打哆嗦！」",
            ],
        });
    }

    // 17. 觀星台/司天
    if ["觀星台", "司天"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "觀星測候台",
            default_host_title: "當值司辰",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正撥轉三百六十星軌銅儀測算星度交角與節氣日影",
            on_duty_demeanor: "身形修長的觀星台司天掌度，身穿星青色觀測大衣，神色沉靜專注，雙眼明亮有神，手裡拿著一具摺疊式黃銅量測儀，專注於記錄天象與氣象變化。",
            lendable_tool: Some((
                "wt_cat_0732",
                "外澳夜光貝領航羅盤",
                "這枚「外澳夜光貝領航羅盤」指針穩定、夜裡亦能辨位，借你入山出海辨識北辰度數。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0732", "外澳夜光貝領航羅盤", 15),
            ],
            trade_talk_pitch: "「台上校準的黃銅隨身星象指北羅盤尚有餘珍，嵌夜光貝母刻度，長途遠行不迷北辰度數，可為你結緣。」",
            on_duty_talk_lines: &[
                "「三百六十星軌銅儀測天地度數，觀星望氣，始知四時循環。星宿雖遠，其度可推；人事雖繁，其理如一。」",
                "「白塔雪巔雲霧晨聚暮散，其間流光往往映照半島風候潮汐。細推其規律，自能趨吉避凶。」",
                "「登高望遠，胸襟自朗。若在此台仰望星河，心中紛擾雜念自會消解無形。」",
            ],
        });
    }

    // 18. 習武堂/武館
    if ["習武堂", "武館"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "演武堂",
            default_host_title: "當值武師",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正手持包革木刀指點學徒格擋防身步法",
            on_duty_demeanor: "體格健碩沉著的武術教練，身穿黑色訓練服與厚牛皮護腰，濃眉有神，動作俐落流暢，雙腕戴著防扭傷護腕，神態沉穩自持。",
            lendable_tool: Some((
                "wt_cat_1015",
                "包革練習木刀",
                "這柄「包革練習木刀」重心沉穩又不傷筋骨，借你演練步法拆招，練完放回兵架即可。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1015", "包革練習木刀", 14),
                ("wt_cat_0846", "卸力厚牛皮護腕", 12),
            ],
            trade_talk_pitch: "「架上備有熟牛皮裹革的練習重木刀與熟銅卸力護腕，耐打耐磨，初習身法必不可少，作價皆公道。」",
            on_duty_talk_lines: &[
                "「筋骨舒展、沉肩墜肘！習武並非好勇鬥狠，而在於調和周身氣脈、遇力卸力，此乃半島先民安身之本。」",
                "「分家巷習武堂重在根基紮實。一柄重木刀每日揮擊五百回，手穩心定，才能在山林曠野中應對萬全。」",
                "「看你步履沉穩，是個可造之材。若有餘暇，不妨換上軟革護腕，下場對拆幾招。」",
            ],
        });
    }

    // 19. 剪剃鋪/理容
    if ["剪剃鋪", "理容"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "理容剪剃鋪",
            default_host_title: "當值剃頭師傅",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正推磨精鋼摺疊剃刀並擰乾熱毛巾替客修面理容",
            on_duty_demeanor: "年逾六旬的理髮鋪老匠師，穿著一塵不染的白色工作衫，精瘦結實，笑容爽朗手腕極穩，工具袋裡插著磨刀布與牛角細齒梳。",
            lendable_tool: Some((
                "wt_cat_0630",
                "理髮鋪精鋼摺疊剃刀",
                "這把「理髮鋪精鋼摺疊剃刀」刃口推磨得吹毛斷髮，借你修整儀容，用時當心手下分寸。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0630", "理髮鋪精鋼摺疊剃刀", 12),
                ("wt_cat_0617", "牛尾軟毛竹柄潔齒刷", 4),
            ],
            trade_talk_pitch: "「特製的精鋼摺疊修容剃刀與牛骨柄豬鬃潔齒刷，皆是隨身洗漱佳品，鋼口鋒利耐用，買上一把受用多年。」",
            on_duty_talk_lines: &[
                "「哈哈，快請坐上紫銅躺椅！熱毛巾敷透鬚角，店裡這把精鋼摺疊剃刀保證又快又順，絕不刮破半點皮！」",
                "「講堂街的書院生員、公議堂的辦事員，哪一個修剪打理不是在店裡拾掇的？清清爽爽才見神采！」",
                "「修面要腕穩，洗頭要水溫恰好。紫銅水管裡的井水常年溫熱，躺下歇息片刻，包你神清氣爽。」",
            ],
        });
    }

    // 20. 浴場/溫泉
    if ["浴場", "溫泉"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "溫泉浴場",
            default_host_title: "當值湯番",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正往寒水玉湯池投放祛寒草木湯包",
            on_duty_demeanor: "氣色紅潤、神情悠閒的浴場掌事，穿著寬鬆透氣的灰白亞麻休閒衫，腳穿防滑木底便履，腰間掛著竹牌，周身帶著淡淡的松脂香胰與溫水蒸氣清香。",
            lendable_tool: Some((
                "wt_cat_0619",
                "長絨蜂格棉面巾",
                "這條新烘乾的「長絨蜂格棉面巾」鬆軟吸水，借你擦拭風塵汗水，洗淨晾乾再還便可。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0620", "茶籽松脂香胰", 4),
                ("wt_cat_0619", "長絨蜂格棉面巾", 5),
            ],
            trade_talk_pitch: "「架上有北山茶籽松脂香胰與大圳長絨蜂格棉面巾，吸水柔潤，買上一份進池，包你洗得通體舒暢。」",
            on_duty_talk_lines: &[
                "「舟車勞頓，身心俱疲了吧？快進熱湯池泡上一刻鐘！這泉水引自老堰頭活脈，底層鋪滿寒水玉，最能活絡筋骨。」",
                "「浴場用的是北山茶籽加松脂熬煉的手工香胰，去垢不傷膚，洗完身上泛著淡淡山林松香，清爽怡人。」",
                "「城中醫師常說，泡湯乃平民溫養第一妙法。發一身透汗、飲半盞清茶，什麼鬱結勞碌全給融化了。」",
            ],
        });
    }

    // 21. 織造/成衣/染坊/服飾/裁縫
    if room_tags.iter().any(|t| t == "織坊" || t == "布料")
        || ["織造", "成衣", "染坊", "服飾", "裁縫"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "織造染坊",
            default_host_title: "當值織匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正為遠行採購商量裁黑沼防雨樹膠長衫",
            on_duty_demeanor: "儀態端莊溫婉的服飾坊掌事，身穿親手剪裁的煙青色棉麻套裝長裙，腰間配著軟皮布尺，長髮整齊挽起，目光敏銳俐落。",
            lendable_tool: Some((
                "wt_cat_0835",
                "細棉麻透氣常服",
                "這件「細棉麻透氣常服」針腳細密、吸汗透氣，借你替換沾濕的衣衫，穿畢洗淨歸還即可。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0835", "細棉麻透氣常服", 14),
                ("wt_cat_0838", "樹膠防水防雨長衫", 18),
                ("wt_cat_0868", "素面玄武岩扣束腰長袍", 16),
            ],
            trade_talk_pitch: "「素面玄武岩扣剪裁外套、黑沼防雨連帽長衫皆有現貨，各色尺碼齊全，試穿合身便可直接交割。」",
            on_duty_talk_lines: &[
                "「請進。撫摸一下這匹大圳細棉麻，質地柔軟透氣，貼身裁件常服，盛暑天行路也不生悶汗。」",
                "「黑沼樹膠防雨長衫是坊裡的招牌，外塗三層密膠、內襯透氣薄棉，狂風驟雨打在身上順水滑落，滴水不沾。」",
                "「衣裝乃行者門面。無論治學士還是走南闖北的商旅，換上一身得體耐磨的剪裁外套，整個人便精神三分。」",
            ],
        });
    }

    // 22. 聯運/貨運/車隊/車馬
    if ["聯運", "貨運", "車隊", "車馬"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "車馬聯運站",
            default_host_title: "當值車頭",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正調度雙輪駝獸大車裝卸四方貨件",
            on_duty_demeanor: "膚色黝黑、嗓音洪亮的聯運站長，身材魁梧結實，身穿厚實毛氈翻領背心，臂膀結實有力，眼神沉穩敏銳，手腕戴著防磨牛皮護腕。",
            lendable_tool: Some((
                "wt_cat_0891",
                "黃銅雙扣厚帆布斜背包",
                "這只「黃銅雙扣厚帆布斜背包」耐磨防潑水、負重省力，借你奔走四方裝載什物。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0891", "黃銅雙扣厚帆布斜背包", 12),
                ("wt_cat_0897", "薄層重礦烏木膽隨身保溫瓶", 10),
            ],
            trade_talk_pitch: "「雙肩厚帆布遠行背囊與雙層烏木內膽保溫瓶皆是長途必備硬貨，容量大、保溫久，旅人獵戶必帶一具。」",
            on_duty_talk_lines: &[
                "「嘿！聯運車隊風雨無阻！無論是走風舶角的大宗海貨，還是運往大圳頭的新麥礦石，我們車隊從不誤期！」",
                "「出遠門行囊要緊湊。隨身背囊要用雙肩加厚帆布，防雨布套扣緊，遇上海嵐濃霧或顛簸山徑才不會散架。」",
                "「草獸性子溫順但耐力驚人，日行百里不喘粗氣。若想搭車去外鎮，先到票檯開好路單，鐘點一到準時發車！」",
            ],
        });
    }

    // 23. 水輪/石磨
    if room_tags.iter().any(|t| t == "磨石")
        || ["水輪", "石磨"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "水輪石磨工坊",
            default_host_title: "當值石磨匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正引水推動玄武岩大石磨細磨新收稻穀與米漿",
            on_duty_demeanor: "樸實沉穩的水輪石磨匠師，穿著沾有細白米粉的粗麻雙帶圍胸短褂，專注傾聽著水輪與雙疊石磨運轉的節奏。",
            lendable_tool: None,
            counter_trade_offers: &[
                ("wt_cat_0388", "現磨溫米漿", 3),
                ("wt_cat_0277", "秋收蓬萊新米", 6),
            ],
            trade_talk_pitch: "「水輪石磨現磨的新米漿與精選糙米裝袋整齊，米香純正，買上兩袋帶回煮粥最是養人。」",
            on_duty_talk_lines: &[
                "「聽這石磨旋轉的水輪聲，水流勻稱，磨出的米粉才細滑如脂。」",
                "「老堰頭引來的高山活水衝力十足，帶動三座玄武岩雙疊大磨日夜不歇，供全鎮每日炊食之需。」",
                "「磨新稻要分兩道工，頭道脫殼、次道出漿，火候全在掌磨人這雙耳朵和掌心溫度裡。」",
            ],
        });
    }

    // ── 外圍聚落工坊與特定職司原型 ──

    // 24. 海事商盟
    if ["海事", "商盟"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "海事商盟",
            default_host_title: "商盟會長",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正檢視碼頭泊位進港貨單與海潮預報",
            on_duty_demeanor: "面容堅毅、鬢角斑白的海事商盟老會長，穿著厚實防風毛呢大衣，目光沉穩如礁石，歷經數十年海上風浪依然從容不迫。",
            lendable_tool: Some((
                "wt_cat_0732",
                "外澳夜光貝領航羅盤",
                "出海靠天也靠盤，這枚「外澳夜光貝領航羅盤」借你隨船導航，切記防潮妥存。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0785", "航海圖冊《風舶航海錄》", 15),
                ("wt_cat_0732", "外澳夜光貝領航羅盤", 15),
                ("wt_cat_1000", "深海巨藻船纜", 18),
            ],
            trade_talk_pitch: "「風舶海事商盟專備遠航海圖、夜光羅盤與深海船纜，經得起狂濤巨浪，隨船必備。」",
            on_duty_talk_lines: &[
                "「看海潮如閱人面，潮頭平緩則水下暗流湧動，行船半點大意不得。」",
                "「商盟立足港灣數十年，靠的就是每艘入港大船平安歸來，船貨互通、信義長存。」",
                "「港外近來風浪稍歇，若要啟航出海，先去碼頭驗核通關路單。」",
            ],
        });
    }

    // 25. 遠洋航務
    if ["遠洋", "船長"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "遠洋航務社",
            default_host_title: "遠洋船長",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於航海圖案前量測外海航線與補給點",
            on_duty_demeanor: "英氣十足的遠洋大船女船長，身穿合身防水航海短襖，長髮高束，隨身帶著海圖筒，眼神銳利自信。",
            lendable_tool: Some((
                "wt_cat_0732",
                "外澳夜光貝領航羅盤",
                "這具夜光領航羅盤走針靈敏，借你出洋辨識水道，歸航時送回船署即可。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0785", "航海圖冊《風舶航海錄》", 15),
                ("wt_cat_1000", "深海巨藻船纜", 18),
            ],
            trade_talk_pitch: "「遠洋航務處備有精準海圖冊與深海巨藻船纜，遠航必不可少。」",
            on_duty_talk_lines: &[
                "「外海雲湧雷鳴之際，船長的手握緊舵輪，全船三十條性命便全在這掌心沉穩之中。」",
                "「遠洋航線不比內河，暗礁密布、水文莫測，每一筆海圖標註都是前人用命換來的。」",
                "「風向已轉向偏東，趁著潮頭正好，補給齊備的大船便要解纜啟航。」",
            ],
        });
    }

    // 26. 造船塢 / 船塢
    if ["造船", "船塢"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "造船修船塢",
            default_host_title: "總造船師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以木工曲尺審視龍骨平直度與捻縫填膏",
            on_duty_demeanor: "體格魁梧的總造船師，穿著耐磨牛皮工作服，手持木工曲尺，身上帶著百年雲杉與防水桐油的氣息。",
            lendable_tool: Some((
                "wt_cat_0956",
                "柯氏松木家用工具箱",
                "這口木匠箱裡有修造船板的平鑿與曲尺，借你維修棧道船體，用完歸還。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1000", "深海巨藻船纜", 18),
                ("wt_cat_1002", "桐油海貝灰捻縫灰膏", 8),
                ("wt_cat_1003", "極淺吃水平底竹筏", 25),
                ("wt_cat_1006", "老青竹黃銅輪捲線釣竿", 10),
            ],
            trade_talk_pitch: "「船塢特製海藻船纜、捻縫桐油灰膏與平底竹筏皆有現貨，水上營生最講究器具精實。」",
            on_duty_talk_lines: &[
                "「一艘好船能載萬石行千里，龍骨若有一絲偏差，到了驚濤駭浪中便會生生扭斷。」",
                "「捻縫灰膏需用百年桐油調合細研貝灰，抹進船縫凝固後堅逾鐵石，滴水不漏。」",
                "「港裡那艘吃水深平底大船正在鋪設外板，缺熟練木工幫忙拉緊夾板繩索。」",
            ],
        });
    }

    // 27. 農牧總社 / 糧倉
    if ["農牧", "穀倉", "糧倉"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "農牧總社",
            default_host_title: "農牧理事長",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正翻查各墾區秋收稻穀入倉清單與草料儲備",
            on_duty_demeanor: "面容溫和、膚色古銅的農牧總社理事長，捲起麻布短衫袖口，雙手布滿厚繭，神情踏實從容。",
            lendable_tool: Some((
                "wt_cat_0895",
                "雙層竹編提籃",
                "這只雙層竹編提籃通風透氣，借你裝運田間蔬果糧米，輕便合用。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0277", "秋收蓬萊新米", 6),
                ("wt_cat_0950", "省力導能翻土犁", 16),
                ("wt_cat_1020", "手搖螺旋壓實乾草料壓捆機", 16),
            ],
            trade_talk_pitch: "「大圳頭新收蓬萊米、省力翻土犁與草料壓捆機都在倉裡，農忙備耕正是時候。」",
            on_duty_talk_lines: &[
                "「稼穡艱辛，春耕夏耘秋收冬藏，每一粒米都凝聚著泥土與汗水。」",
                "「今年大圳頭灌溉水路通暢，各莊稻穗顆粒飽滿，糧倉充實，四鎮百姓心裡才踏實。」",
                "「草料儲備要搶在霜降前曬乾壓捆，冬日牲口吃得飽足，開春才有氣力耕田拉車。」",
            ],
        });
    }

    // 28. 礦場坑口
    if room_tags.iter().any(|t| t == "礦場")
        || ["礦務", "坑口"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "礦務坑口司",
            default_host_title: "礦務掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正督導礦車出坑並秤量赤鐵礦石品位",
            on_duty_demeanor: "體格結實的礦務掌事，穿著耐火皮革背心，眼神專注銳利，指間常帶著赤鐵礦粉的痕跡。",
            lendable_tool: Some((
                "wt_cat_0957",
                "鍛鋼小羊角錘",
                "這柄鍛鋼小錘硬度極高，借你探礦敲石取樣，莫落入深坑之中。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0097", "相思木硬炭", 6),
                ("wt_cat_0957", "鍛鋼小羊角錘", 10),
                ("wt_cat_0846", "卸力厚牛皮護腕", 14),
            ],
            trade_talk_pitch: "「坑口精選重礦相思木硬炭、採礦鍛鋼錘與護腕現貨齊全，下坑進山最需穩實裝備。」",
            on_duty_talk_lines: &[
                "「地底百尺坑道裡，岩層聲響便是警兆，凡下坑者務必眼明心細、嚴守通風。」",
                "「三號坑出的赤鐵礦品位極高，送進第一鍛造工坊的高溫爐，能出上等精鋼。」",
                "「坑口軌道日夜穿梭，每一車礦石拉上來，全島的鐵工器脈便又多了一分底氣。」",
            ],
        });
    }

    // 29. 燈塔燻坊
    if ["燈塔", "燻"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "燈塔燻坊",
            default_host_title: "燈塔巡守匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正維護燈塔透鏡燃芯並翻動松木燻魚架",
            on_duty_demeanor: "守望燈塔的資深匠師，穿著防風厚帆布短襖，目光望向海面，身上帶著冷燻松木屑與海風的乾爽氣息。",
            lendable_tool: Some((
                "wt_cat_0898",
                "防潮紅磷引火籤盒",
                "海邊潮氣重，這盒封蠟引火籤借你點火照明，遇風不滅。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0319", "海風脫水小蝦米", 6),
                ("wt_cat_0320", "乾製海生干貝柱", 10),
                ("wt_cat_0723", "旋蓋夜光貝母調光台燈", 15),
            ],
            trade_talk_pitch: "「燈塔特製冷燻乾貝、脫水海味與防風貝母夜燈皆在架上，海客行船最愛這口乾香。」",
            on_duty_talk_lines: &[
                "「燈塔之光是夜航者的眼睛，三十年風雨狂嘯，這盞主燈從未熄滅過一刻。」",
                "「底層燻坊用的是老松木冷燻，海風慢慢吹透，魚肉干貝緊實生香，能存上半年不壞。」",
                "「潮汐變化與天邊雲團相生相伴，看懂了燈塔外的大海，便懂了天道運行。」",
            ],
        });
    }

    // 30. 鑿石切玉坊
    if ["鑿石", "切石", "寒水玉"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "切石玉作坊",
            default_host_title: "切石匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正引銀砂流水切石機細剖寒水玉原石",
            on_duty_demeanor: "寒水玉切石匠師，神情沉靜專注，繫著防濺工作圍裙，仔細檢查切石水槽中的切面平整度。",
            lendable_tool: Some((
                "wt_cat_0735",
                "導能測紋石板",
                "這塊測紋石板刻度精準，借你比對玉石平整度與導能紋路，小心輕放。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0559", "寒水玉雙層保鮮石櫃", 35),
                ("wt_cat_0949", "銀砂水力切石機", 22),
            ],
            trade_talk_pitch: "「水槽剖出的寒水玉質地清透、天然蓄寒，打造成雙層保鮮石櫃，盛夏蔬果久放如新。」",
            on_duty_talk_lines: &[
                "「切玉講究順應石脈，硬取硬鑿只會碎成亂石。銀砂細磨、以柔克堅，才見玉石內蘊靈性。」",
                "「北嶺山溪沖刷而下的寒水玉，蘊含冰川冷氣，凡醫館藏藥、食堂保鮮皆賴此物。」",
                "「下刀如運筆，水流衝力與砂粒粗細絲絲入扣，每一片薄板都需經三遍拋光。」",
            ],
        });
    }

    // 31. 水動鋸木坊
    if room_tags.iter().any(|t| t == "林場")
        || ["鋸木"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "水動鋸木坊",
            default_host_title: "水車鋸木匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正操控水車傳動大鋸剖開巨幅原木",
            on_duty_demeanor: "水車鋸木匠師，身材挺拔精悍，工作服上沾著細碎木屑，檢視木材紋理時眼神極為專注。",
            lendable_tool: Some((
                "wt_cat_0956",
                "柯氏松木家用工具箱",
                "這口工具箱備有木尺與推刨，借你修整木料，用完送回鋸木坊。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0947", "水力原木大鋸", 20),
                ("wt_cat_0956", "柯氏松木家用工具箱", 14),
            ],
            trade_talk_pitch: "「鋸木坊精剖雲杉厚木料、水力原木大鋸與家用木工箱皆備妥，建屋打傢俱皆用得上。」",
            on_duty_talk_lines: &[
                "「巨木排山倒海而來，水車運轉如雷鳴。看木紋走向下鋸，木板平直不翹曲。」",
                "「北山優質老雲杉，生長極緩，質地細密柔韌，是全城營造屋樑與大船龍骨的首選。」",
                "「水動大鋸全憑老堰頭活水驅動，省工省力，一日能剖數十根巨材。」",
            ],
        });
    }

    // 32. 牛埔乳酪毛氈牧坊
    if ["乳酪", "毛氈", "牧場"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "乳酪毛氈牧坊",
            default_host_title: "牧坊大匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正壓濾新鮮乳酪並梳理熟羊毛壓製厚毛氈",
            on_duty_demeanor: "牛埔的乳酪與毛氈匠師，穿著厚毛氈背心，神態溫和開朗，雙手熟練地處理羊毛與乳製品。",
            lendable_tool: Some((
                "wt_cat_0891",
                "黃銅雙扣厚帆布斜背包",
                "這口雙扣背囊容量大、結實耐用，借你裝盛行路物資。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0501", "穩火慢煮醇厚鮮奶茶", 5),
                ("wt_cat_0836", "飛鼠厚絨禦寒冬衣", 22),
                ("wt_cat_0837", "雲豹毛領禦寒大氅", 28),
            ],
            trade_talk_pitch: "「新鮮乳酪乾、現煮醇厚鮮奶茶與厚絨禦寒毛氈衣物現貨充足，過冬遠行溫暖隨行。」",
            on_duty_talk_lines: &[
                "「牛埔草質肥美，牧群食百草甘泉，產出的乳汁脂香四溢，熬成濃縮乳酪能存數月。」",
                "「秋季羊毛剪下後需經三道水洗去脂，再以溫水重壓成氈，防風禦寒天下無雙。」",
                "「喝一碗現煮熱鮮奶茶，滿口醇香，身上寒氣頓時全消。」",
            ],
        });
    }

    // 33. 耐火磚窯
    if ["磚窯", "耐火磚"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "耐火磚窯坊",
            default_host_title: "磚窯匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正監看高溫窯爐燒製耐火磚並檢視冷卻磚色",
            on_duty_demeanor: "耐火磚窯資深匠師，膚色帶著窯火烘烤的紅潤，戴著厚牛皮隔熱袖套，對高溫燒製的耐火磚品質把關嚴格。",
            lendable_tool: Some((
                "wt_cat_0735",
                "導能測紋石板",
                "借你這塊測紋石板量測磚體規格與熱脹係數，切莫磕碰缺角。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0944", "紅土耐火磚高溫鐵爐", 20),
                ("wt_cat_0023", "粗陶紅黏土", 5),
                ("wt_cat_0097", "相思木硬炭", 6),
            ],
            trade_talk_pitch: "「窯裡新出爐一批耐火磚與高溫耐火鐵爐，能抗千度烈火，各工坊爐台修造不可或缺。」",
            on_duty_talk_lines: &[
                "「燒耐火磚全看黏土成色與窯火溫度，火候不足則鬆散易裂，火過則釉化失其蓄熱之能。」",
                "「半島各坊的鍛造爐、食堂大灶皆用我們窯裡出的紅土火磚，百年不塌不裂。」",
                "「出窯需等足三天自然冷卻，急著開窯吹了冷風，滿窯好磚便前功盡棄。」",
            ],
        });
    }

    // 34. 重型獵具坊
    if combined.contains("獵具") {
        return Some(RoomVenueObject {
            venue_kind: "重型獵具坊",
            default_host_title: "獵具大匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正組裝彈簧複合獵網與鍛造獵叉配件",
            on_duty_demeanor: "重型獵具大匠，身材魁梧結實，穿著防刮厚皮護具，雙手沉穩有力，組裝大型獵網與複合獵具十分俐落。",
            lendable_tool: Some((
                "wt_cat_1021",
                "岩羊角手柄折疊獵刀",
                "這柄折疊獵刀鋒利耐用，借你山林防身或破開荊棘，回城時擦拭乾淨交還。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1021", "岩羊角手柄折疊獵刀", 18),
                ("wt_cat_0846", "卸力厚牛皮護腕", 14),
                ("wt_cat_0840", "鍛造坊厚牛皮防燙圍裙", 16),
            ],
            trade_talk_pitch: "「岩羊角折疊獵刀、耐磨皮護腕與獵隊防護具齊備，出入深山大澤保你安然無虞。」",
            on_duty_talk_lines: &[
                "「進深山會巨獸，靠的不僅是力氣，更是稱手的獵具與冷靜心性。」",
                "「大型獵網用的是巨藻海纜與熟銅絞線合編，再狡黠兇猛的野獸也掙脫不得。」",
                "「出山打獵前，先仔細檢查刀刃開口與機簧卡榫，手頭器物可靠，命才保得住。」",
            ],
        });
    }

    // 35. 水尾寮竹筏渡口
    if ["高腳屋", "竹筏", "水尾寮"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "竹筏渡口坊",
            default_host_title: "竹筏匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正火烤老青竹彎折筏頭並編紮防水浮筒",
            on_duty_demeanor: "水尾寮的高腳屋與竹筏匠師，身形精瘦靈活，腰間繫著防滑繩索，在淺澤木棧道上行走自如。",
            lendable_tool: Some((
                "wt_cat_1006",
                "老青竹黃銅輪捲線釣竿",
                "這根老青竹釣竿彈性十足，借你在渡口淺灘垂釣魚蝦，順便消磨時光。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1003", "極淺吃水平底竹筏", 25),
                ("wt_cat_1006", "老青竹黃銅輪捲線釣竿", 10),
            ],
            trade_talk_pitch: "「水尾寮特造平底竹筏與老竹捲線釣竿，吃水極淺，穿梭蘆葦淺澤平穩自如。」",
            on_duty_talk_lines: &[
                "「水尾寮水淺灘多，尋常大船進不來，只有這種平底竹筏才能如履平地。」",
                "「竹子要挑三年生老青竹，火烤出青油後結實耐泡，在水裡泡幾年都不發爛。」",
                "「坐筏渡河順水漂，遇急流橫篙一點，水勢再急也翻不了筏。」",
            ],
        });
    }

    // 36. 山徑補給行
    if ["山徑", "補給"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "山徑補給行",
            default_host_title: "補給掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正清點防倒風生鐵柴火爐與保溫乾糧行囊",
            on_duty_demeanor: "健談熱心的山徑補給行負責人，穿著防風厚背心，腰間工具帶插著修鞋工具，笑著招呼往來山路的採集員與旅人。",
            lendable_tool: Some((
                "wt_cat_0897",
                "薄層重礦烏木膽隨身保溫瓶",
                "山道風寒露重，這只保溫瓶借你隨身裝盛熱湯熱茶，溫暖驅寒。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0891", "黃銅雙扣厚帆布斜背包", 12),
                ("wt_cat_0897", "薄層重礦烏木膽隨身保溫瓶", 10),
                ("wt_cat_1037", "防倒風野營生鐵柴火爐", 15),
            ],
            trade_talk_pitch: "「防倒風生鐵柴火爐、雙扣斜背包與烏木保溫瓶，山路行路必備三大件，備齊出發免受風露寒侵。」",
            on_duty_talk_lines: &[
                "「翻過這座埡口就是黑沼與深山，外頭氣候多變，行囊打理得當才能走得安心。」",
                "「野營過夜最怕生火受潮，生鐵柴火爐擋風防雨，幾根枯枝就能煮出一鍋熱湯。」",
                "「往來山徑三十年，這間補給行就是行山人的家，進來坐坐喝口熱水歇歇腳。」",
            ],
        });
    }

    // 37. 巡防哨所
    if ["巡防", "防務"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "巡防哨所",
            default_host_title: "哨所隊長",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正登台瞭望外圍水澤動靜並校閱防務巡行排班",
            on_duty_demeanor: "身材修長矯健的防務隊長，身穿深綠色防刮獵裝，眼神如鷹隼般敏銳，時刻留意著林間與水澤的動靜。",
            lendable_tool: Some((
                "wt_cat_0732",
                "外澳夜光貝領航羅盤",
                "巡林辨位不可短少，這枚夜光羅盤借你巡視野外辨明北辰度數。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1064", "巡山獵人隨身急救皮囊", 12),
                ("wt_cat_0846", "卸力厚牛皮護腕", 12),
                ("wt_cat_0891", "黃銅雙扣厚帆布斜背包", 12),
            ],
            trade_talk_pitch: "「哨所常備巡山急救皮囊、卸力護腕與厚帆布巡山背包，野外巡行有備無患。」",
            on_duty_talk_lines: &[
                "「哨所日夜警惕，外圍沼澤與山林若有異常動靜，烽火快騎隨時出動。」",
                "「巡行野外最忌落單冒進，隨身帶好急救皮囊與指北羅盤，安全返回哨所為要。」",
                "「凡途經此處的過路行商，皆需在哨所登記簿留下行蹤與目的地，以備策應。」",
            ],
        });
    }

    None
}

/// 依據市集攤位、專門店鋪、倉庫、碼頭與街巷解析一等房間物件契約（RoomVenueObject）
///
/// 涵蓋 28 種市集攤販、專業作坊、商棧倉庫、驛站碼頭與公共街巷類型，
/// 統一封裝常備交易物件（counter_trade_offers）與當值對話語錄（on_duty_talk_lines）。
pub fn resolve_market_or_specialty_room_object(
    room_name: &str,
    extra_hint: &str,
) -> Option<RoomVenueObject> {
    if is_private_residence_room_name(room_name) {
        return None;
    }
    if is_street_or_gate_room_name(room_name) {
        return Some(RoomVenueObject {
            venue_kind: "街巷橋樑公共空地",
            default_host_title: "街坊居民",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正於石橋欄杆旁駐足歇息，觀看街面人流往來",
            on_duty_demeanor: "神情安詳從容的街坊居民，安居樂業，對城中各坊動向頗為熟絡。",
            lendable_tool: None,
            counter_trade_offers: &[],
            trade_talk_pitch: "",
            on_duty_talk_lines: &[
                "「文曜城裡的街道很平整，作息也很規律。白天各行各業正常營業，到了晚上鐘樓報時收班，走在街上很安心。」",
                "「三水口連通外港，分家巷聚集各類工坊，講堂街則是學院與書局的所在地，城裡機能很齊全。」",
                "「初來文曜嗎？如果要找各處場所，順著三水匯流大石橋往四面看，各家招牌都很清楚。」",
            ],
        });
    }
    let combined = format!("{room_name} {extra_hint}");
    // 1. 鮮魚水產市集
    if combined.contains("鮮魚") || combined.contains("魚市") || combined.contains("海產") || combined.contains("水產") || combined.contains("活魚") || combined.contains("蝦蟹") || combined.contains("漁販") || combined.contains("魚販") || combined.contains("三水河鮮") || combined.contains("早市鮮魚拍賣場") || combined.contains("風舶早市菜場") || combined.contains("北灣早市菜場") {
        return Some(RoomVenueObject {
            venue_kind: "鮮魚水產市集",
            default_host_title: "當值魚販",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於碎冰魚槽前分揀鮮魚並以柳條穿鰓",
            on_duty_demeanor: "腰繫皮圍裙、手持刮鱗剖魚小刀的精明魚販，說話響亮俐落，周身帶著清爽海風與冰鮮氣息。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0224", "銀鱗土魠魚", 8),
            ("wt_cat_0226", "淺澤青殼河蝦", 6),
            ("wt_cat_0227", "活水肥美草魚", 7),
        ],
            trade_talk_pitch: "「清晨出海新靠岸的活水魚蝦，鱗光透亮，挑幾尾回去清蒸煲湯最是鮮甜。」",
            on_duty_talk_lines: &[
            "「晨光初透，風舶外海的銀鱗土魠魚與老堰頭肥草魚新卸船！冰鮮競價，手慢可就搶不到了。」",
            "「三水河網直通西海，這水尾寮青殼蝦活蹦亂跳，一進早市就引來各大食堂掌勺競價。」",
            "「泊位邊水汽重，買魚要看魚鰓成色。只要是我們風舶拍賣場出具的魚簍木牌，尾尾保鮮！」",
        ],
        });
    }

    // 2. 南北乾貨棧
    if combined.contains("乾貨") || combined.contains("海味") || combined.contains("南北雜貨") || combined.contains("臘味") || combined.contains("山珍") || combined.contains("山貨") || combined.contains("獵戶") || combined.contains("鹽漬") || combined.contains("水產乾貨") {
        return Some(RoomVenueObject {
            venue_kind: "南北乾貨棧",
            default_host_title: "當值乾貨商",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正翻動竹篩上的脫水蝦米與乾貝柱檢查乾燥度",
            on_duty_demeanor: "面容精瘦、熟諳水陸物產的老商賈，指尖微帶海鹽粉霜，待客細緻而健談。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0319", "海風脫水小蝦米", 6),
            ("wt_cat_0320", "乾製海生干貝柱", 10),
            ("wt_cat_0228", "礁石野生紫菜", 5),
        ],
            trade_talk_pitch: "「風吹日曬熟成的海味乾貨與山珍臘味，耐儲耐運，煲湯增鮮最為醇美。」",
            on_duty_talk_lines: &[
            "「烏石鼻乾製海貝與風櫃尾野生紫菜，曬足整整七日海風，煲湯入味最是鮮美醇厚。」",
            "「外海風急浪大，船隊遠航必備鹽漬白帶魚與脫水蝦米，耐放半年不壞，吃時用溫水泡發即可。」",
            "「乾貨儲庫通風防潮全靠頂層木百葉，若要大宗調撥發往西境，憑三水商行驗單立提。」",
        ],
        });
    }

    // 3. 醬園調味坊
    if combined.contains("醬園") || combined.contains("醋坊") || combined.contains("腐乳") || combined.contains("醬菜") || combined.contains("油坊") || combined.contains("麻油") || combined.contains("陳記醬園") {
        return Some(RoomVenueObject {
            venue_kind: "醬園調味坊",
            default_host_title: "當值釀造東家",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以竹筒長杓自老陶缸舀取烏醋過濾入瓶",
            on_duty_demeanor: "身著粗布深藍長衫的調味釀師，掌心常染豆醬濃香，眼神溫和沉穩。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0346", "熟成野山果醋", 5),
            ("wt_cat_0347", "老罈烏醋香料汁", 6),
            ("wt_cat_0338", "日曬白霜細海鹽", 4),
        ],
            trade_talk_pitch: "「老罈慢釀黑豆醬與野山果醋，酸甘醇厚，備上一壺入菜提鮮勝過千味。」",
            on_duty_talk_lines: &[
            "「老罈黑豆醬油慢釀三年，揭開陶缸蓋滿街飄香；半嶺烏醋入菜生津，是文曜老店看家滋味。」",
            "「紅麴米醋酸香柔潤，配羊肉湯餅或海鮮最解腥膩。你若帶乾糧，不妨打一小竹筒醋汁備用。」",
        ],
        });
    }

    // 4. 酒坊烈酒鋪
    if combined.contains("酒坊") || combined.contains("紅爐烈酒") || combined.contains("酒鋪") || combined.contains("釀酒") || combined.contains("酒保") || combined.contains("烈酒大飯鋪") || combined.contains("沈家老酒") || combined.contains("老酒坊") || combined.contains("盤查酒鋪") {
        return Some(RoomVenueObject {
            venue_kind: "酒坊烈酒鋪",
            default_host_title: "當值酒保",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於通紅爐火旁溫熱封缸米酒並擦拭大碗",
            on_duty_demeanor: "性情爽直的酒家漢子，挽著衣袖，迎客熱絡，說話透著四方豪氣。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0506", "封缸陳年醇米酒", 12),
            ("wt_cat_0507", "蜜漬山果甜釀", 8),
            ("wt_cat_0444", "厚釜慢燉原牛肉湯", 10),
        ],
            trade_talk_pitch: "「陳年老酒滾燙下喉，配一大碗慢燉牛肉湯，驅寒生暖，最宜歇腳暖身。」",
            on_duty_talk_lines: &[
            "「大釜紅爐烈火正旺，封缸陳年米酒滾燙下喉！走長途的行商與打鐵師傅，進門先來兩大碗！」",
            "「北嶺山果甜釀配大塊燉牛肉，酒香夾著肉香，任憑門外寒風呼嘯，屋裡熱氣熏得人面紅耳赤。」",
            "「入城盤查先歇歇腳，喝碗溫酒定定神。西關卡哨查得嚴，但只要身家清白，飲飽便好上路。」",
        ],
        });
    }

    // 5. 麵食乾糧鋪
    if combined.contains("羊肉湯餅") || combined.contains("麵食") || combined.contains("餅鋪") || combined.contains("燒餅") || combined.contains("炊餅") || combined.contains("乾糧") || combined.contains("大食堂") || combined.contains("熱食") || combined.contains("飯館") || combined.contains("廚師") || combined.contains("伙夫") || combined.contains("西門麵食鋪") || combined.contains("乾糧備製作坊") {
        return Some(RoomVenueObject {
            venue_kind: "麵食乾糧鋪",
            default_host_title: "當值麵點掌勺",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正揉擀高筋白麵團，於沸湯鐵鍋前拉切刀削麵片",
            on_duty_demeanor: "圍著雪白圍裙的掌勺麵師，臂力沉勁，動作俐落，額角沁著熱氣汗珠。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0311", "鹽醃草飼羊肉條", 10),
            ("wt_cat_0284", "竹篩日曬摺疊麵", 4),
            ("wt_cat_0285", "手工日曬粗麵線", 5),
        ],
            trade_talk_pitch: "「滾湯羊肉湯餅與日曬乾糧麵條，香醇吸汁，隨身帶上幾扎長路不愁餓肚子。」",
            on_duty_talk_lines: &[
            "「大碗羊肉湯餅熱氣騰騰！北嶺鹽醃草飼羊肉慢燉化渣，手擀麵片筋道吸汁，撒一把野薄荷碎，痛快！」",
            "「大圳頭高筋白麵切得細勻，下鍋滾三滾便撈起，配上一勺熬透的熟牛油辣子，行客吃了腳下生風！」",
            "「出西門走戈壁駝道，隨身必帶乾烙饃與脫水麵筋。耐嚼扛餓，在沙漠裡揣在懷裡十天不餿。」",
        ],
        });
    }

    // 6. 茶棚茶飯館
    if combined.contains("茶棚") || combined.contains("茶飯館") || combined.contains("涼茶") || combined.contains("甜水") || combined.contains("水手茶飯館") || combined.contains("工匠大食堂") || combined.contains("食堂") || combined.contains("跑堂") || combined.contains("大廚") {
        return Some(RoomVenueObject {
            venue_kind: "茶棚茶飯館",
            default_host_title: "當值茶東家",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正提著大銅壺向粗瓷茶碗沖注翻滾的熱茶湯",
            on_duty_demeanor: "隨和熱心的大茶棚東家，笑瞇瞇地迎送四方茶客，耳聽八方、見聞極廣。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0394", "切仔湯麵配滷蛋", 5),
            ("wt_cat_0392", "鮮肉大包子", 4),
            ("wt_cat_0500", "山風草炭焙烘青茶", 4),
        ],
            trade_talk_pitch: "「大碗熱茶配鮮肉包子與切仔麵，價廉物美，坐下歇歇腳喝碗茶再走。」",
            on_duty_talk_lines: &[
            "「大食堂飯鐘一響，千號工匠齊奔食案！厚釜原牛骨湯配十八摺肉包，熱氣騰騰管飽有力氣！」",
            "「碼頭茶棚倚江而建，粗瓷碗泡著高山濃茶。聽水手擺談外海鯨蹤與河道暗礁，最是長見識。」",
            "「切仔湯麵配滷蛋，一勺油蔥酥香飄半條街。靠岸水手吃飽抹嘴，轉身就去船塢扛大包。」",
        ],
        });
    }

    // 7. 鹽糧米行倉
    if combined.contains("鹽糧") || combined.contains("糧倉") || combined.contains("米行") || combined.contains("穀行") || combined.contains("草料行") || combined.contains("豆餅") || combined.contains("精鹽") || combined.contains("鹽棧") || combined.contains("糧商") || combined.contains("通濟河運總倉") || combined.contains("戰備糧草") || combined.contains("儲倉") || combined.contains("鹽巴草料行") || combined.contains("粗鹽") {
        return Some(RoomVenueObject {
            venue_kind: "鹽糧米行倉",
            default_host_title: "當值糧商掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以量斗平升核對白霜海鹽與新米袋數",
            on_duty_demeanor: "著青布夾襖、手握提秤與算盤的糧行司事，帳目分明，神色穩健。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0338", "日曬白霜細海鹽", 4),
            ("wt_cat_0284", "竹篩日曬摺疊麵", 4),
            ("wt_cat_0277", "秋收蓬萊新米", 6),
        ],
            trade_talk_pitch: "「半島精選新米與日曬細海鹽，囤糧防歉，戶戶少不得這些根本生計之物。」",
            on_duty_talk_lines: &[
            "「風舶白霜海鹽堆垛如雪山，大圳新米一袋袋入庫。通濟總倉連通內河，一日進出糧船不下百艘。」",
            "「戰備糧草倉四面石垣厚重，通風隔潮滴水不漏。巡防營備與各鎮應急，全指望這片地下防潮窖。」",
            "「草料壓捆機手搖絞緊，壓出的乾草磚緊實耐運。駝獸沿途嚼著草磚喝鹽水，走千里也不乏力。」",
        ],
        });
    }

    // 8. 紙料造紙坊
    if combined.contains("宣紙") || combined.contains("造紙") || combined.contains("竹綿紙") || combined.contains("青檀宣紙") || combined.contains("竹綿造紙精廬") || combined.contains("紙料儲庫") || combined.contains("善本紙料") {
        return Some(RoomVenueObject {
            venue_kind: "紙料造紙坊",
            default_host_title: "當值造紙匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正檢查簾床抄造出的竹綿濕紙平整度與烘紙壁火溫",
            on_duty_demeanor: "掌心帶著草木紙漿纖維痕跡的文質匠人，神情專注雅緻，舉止溫文。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0779", "竹綿印刷紙", 6),
            ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
        ],
            trade_talk_pitch: "「青檀與竹綿精工造就的良紙，落墨如雲、抗蛀防脆，著書立說與繪製圖冊首選。」",
            on_duty_talk_lines: &[
            "「青檀表皮經三年水浸日曬，木臼舂爛後配竹綿抄造。這紙張細膩勻整、落墨如雲，百年不脆！」",
            "「簾床入水輕搖，紙漿如蟬翼鋪展。烘紙壁火溫要求極嚴，過熱則裂，微冷則澀，全在手感掌握。」",
            "「藏書庫善本紙料皆用花椒防蛀，青檀宣紙整捆入樟木箱封存，專供學院修撰與輿圖抄繪之用。」",
        ],
        });
    }

    // 9. 文房筆墨行
    if combined.contains("徽墨") || combined.contains("石硯") || combined.contains("筆墨") || combined.contains("文房") || combined.contains("松煙徽墨") || combined.contains("端溪石硯") || combined.contains("宣筆") || combined.contains("製毫") || combined.contains("筆墨文具") {
        return Some(RoomVenueObject {
            venue_kind: "文房筆墨行",
            default_host_title: "當值筆墨東家",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以細鹿皮擦拭端溪石硯與鋼尖蘸水筆",
            on_duty_demeanor: "長袍儒雅的老文房商，眼光銳利，隨身散發淡雅松煙與沉香氣息。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0821", "玻璃螺紋瓶裝黑墨水", 6),
            ("wt_cat_0780", "彈性精鋼尖蘸水硬筆", 10),
        ],
            trade_talk_pitch: "「端溪水岩老硯與耐磨精鋼蘸水筆，下墨均勻順手，文墨行家自知妙處。」",
            on_duty_talk_lines: &[
            "「古松點煙、鹿膠輕膠慢揉，千錘百鍊始成徽墨。研於石硯之上，墨色黝黑沉厚，發出淡淡松香。」",
            "「端溪水岩老坑石硯，石質溫潤如玉，呵氣即可研墨。磨刀不損石，落墨發清響，文人求之若渴。」",
            "「宣筆製毫挑選秋兔脊毛與野狼青毫，兼毫剛柔並濟。無論寫蠅頭小楷還是大字擘窠，提按皆隨心意。」",
        ],
        });
    }

    // 10. 印書裝幀書坊
    if combined.contains("活字") || combined.contains("印書") || combined.contains("裝幀") || combined.contains("書坊") || combined.contains("藏書閣") || combined.contains("講堂") || combined.contains("書生") || combined.contains("教習") || combined.contains("刻工") || combined.contains("印行") || combined.contains("古籍裝幀") || combined.contains("裱褙") || combined.contains("文房四寶") || combined.contains("藏書") || combined.contains("書局") || combined.contains("書舍") || combined.contains("典藏庫") || combined.contains("圖書典藏庫") {
        return Some(RoomVenueObject {
            venue_kind: "印書裝幀書坊",
            default_host_title: "當值書坊刻匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正檢視活字字盤排版並塗刷均勻印墨",
            on_duty_demeanor: "穿著沾有墨漬的靛藍長衫，目光篤定認真，指節因常年排字雕刻而堅實有力。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0784", "《白塔風物誌》", 8),
            ("wt_cat_0786", "每旬發行《半島旬報》", 3),
            ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
        ],
            trade_talk_pitch: "「活字印製《白塔風物誌》與每旬旬報新鮮出爐，買幾冊手札筆記，記錄行路見聞。」",
            on_duty_talk_lines: &[
            "「活字銅模逐字揀排，壓印在濕潤竹綿紙上。新一期《半島旬報》排版初定，墨香四溢，晌午即售！」",
            "「藏書別院卷帙浩繁，從前朝航海錄到當今農水格物皆有編目。借閱抄錄需淨手焚香，切莫折角。」",
            "「名家書畫裱褙講究『漿清如水，托如蟬翼』。經熟絹包邊、花綾隔水，殘破舊卷立時重獲生機。」",
        ],
        });
    }

    // 11. 格物輿圖算學館
    if combined.contains("輿圖") || combined.contains("測風") || combined.contains("曆算") || combined.contains("星象") || combined.contains("算學") || combined.contains("觀星") || combined.contains("格物輿圖") || combined.contains("北辰測風") || combined.contains("會計") || combined.contains("講舍") || combined.contains("觀星台") || combined.contains("學院") || combined.contains("蒙學堂") || combined.contains("學生") {
        return Some(RoomVenueObject {
            venue_kind: "格物輿圖算學館",
            default_host_title: "當值曆算教習",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以銅製分度圓規在海圖上量算水深與航道經緯",
            on_duty_demeanor: "戴著黃銅助視鏡、身形修長的格物學者，嚴謹沉靜，談及天文山川便神采奕奕。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0785", "航海圖冊《風舶航海錄》", 15),
            ("wt_cat_0731", "高空青銅測風儀", 18),
            ("wt_cat_0788", "烏木便攜計數板", 14),
        ],
            trade_talk_pitch: "「精密航海圖、高空測風儀與便攜計數板，遠洋出海與築堤興修之必備圭臬。」",
            on_duty_talk_lines: &[
            "「格物輿圖院測量文曜內外地脈，雙筒青銅測量鏡校準星位，繪就的水道深淺圖是各船長保命符。」",
            "「北辰高台上銅測風儀旋轉不休，候風官推算明日信風流向，旗號一掛，整座港口的船隊便知進退。」",
            "「計算精準快速。會計與曆算講堂正推演防洪渠堤壩受力與糧稅分帳，一絲一毫差錯都逃不過算尺。」",
        ],
        });
    }

    // 12. 琴棋金石雅集齋
    if combined.contains("斫琴") || combined.contains("棋院") || combined.contains("鑑藏") || combined.contains("古玩") || combined.contains("雅集") || combined.contains("古琴") || combined.contains("文曜棋院") || combined.contains("古物鑑藏齋") {
        return Some(RoomVenueObject {
            venue_kind: "琴棋金石雅集齋",
            default_host_title: "當值鑑藏師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以紫光燈細細檢視古銅鏡紋理與古琴漆灰斷紋",
            on_duty_demeanor: "寬袍大袖、仙風道骨的古物雅士，舉止風雅超脫，鑑別器物明察秋毫。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0795", "烏木黑子白貝圍棋盤", 20),
            ("wt_cat_0796", "松香木雕刻象棋組", 16),
        ],
            trade_talk_pitch: "「烏木黑子白貝棋盤與松香木雕刻象棋，質地厚重溫潤，坐隱忘憂，陶冶性情。」",
            on_duty_talk_lines: &[
            "「老杉良桐陰乾五載，掏膛配金石之聲。指尖撫過絲弦，清越幽遠，如高山空谷流水潺潺。」",
            "「黑子玄武岩，白子老海貝。文曜棋院黑白交錯間，爭的不是一城一池，是天元推衍的大格局。」",
            "「古物鑑藏齋明窗淨几，青銅古鏡與殘卷古玉皆需紫光過目。是真古蹟還是後仿，眼力分毫不差。」",
        ],
        });
    }

    // 13. 杏林百草藥局
    if combined.contains("杏林百草醫館") || combined.contains("採藥棧") || combined.contains("藥材行") || combined.contains("藥鋪") || combined.contains("診所") || combined.contains("醫者") || combined.contains("大夫") || combined.contains("藥師") || combined.contains("百草醫館") || combined.contains("採藥") || combined.contains("藥材中轉站") || combined.contains("百草藥居") || combined.contains("脈絡診所") || combined.contains("醫師") || combined.contains("學徒") {
        return Some(RoomVenueObject {
            venue_kind: "杏林百草藥局",
            default_host_title: "當值醫師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正研磨拔毒水苔敷散並整飭銀針囊",
            on_duty_demeanor: "白袍溫潤的百草醫者，神情親切专注，隨身自帶淡淡草木芳冽香氣。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_1054", "拔毒止血水苔敷藥", 10),
            ("wt_cat_1055", "白茫避瘴清心丸", 12),
            ("wt_cat_1060", "溫氏九寸脈絡銀針囊", 16),
        ],
            trade_talk_pitch: "「黑沼拔毒水苔、避瘴清心丸與脈絡銀針囊，遠涉山林防身辟瘴必不可少。」",
            on_duty_talk_lines: &[
            "「黑沼拔毒止血水苔敷於傷口，三息止血；南哨白茫避瘴清心丸含在舌下，入深林不染瘴氣。」",
            "「醫館診脈辨虛實，九寸脈絡銀針順行氣血。無論跌打骨折還是積勞內傷，照藥方抓藥調理自能康復。」",
            "「採藥客翻越北山絕壁帶回新鮮山草，藥棧天井正分揀陰乾。山氣芬芳，聞一口便覺神清氣爽。」",
        ],
        });
    }

    // 14. 百草烘藥溫浴鋪
    if combined.contains("烘藥房") || combined.contains("百草烘藥房") || combined.contains("香燭") || combined.contains("山風草香燭鋪") || combined.contains("浴場") || combined.contains("溫水浴場") {
        return Some(RoomVenueObject {
            venue_kind: "百草烘藥溫浴鋪",
            default_host_title: "當值烘藥師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於火道前翻焙艾草香盤並調和清涼草木湯包",
            on_duty_demeanor: "衣衫清爽俐落的藥鋪管事，手腳勤快敏捷，周身環繞著草木溫熏香氣。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_1057", "溫氏導能溫石百草溫灸盒", 14),
            ("wt_cat_1058", "驅蚊艾草香盤", 6),
            ("wt_cat_0119", "山野清香薄荷葉", 4),
            ("wt_cat_1056", "祛寒草木湯包", 6),
        ],
            trade_talk_pitch: "「導能溫石灸盒、祛寒湯包與驅蚊艾草香盤，通經活血拔除寒濕，行路露營必備。」",
            on_duty_talk_lines: &[
            "「烘藥房火道均勻，溫石百草溫灸盒借火性引藥力通透骨髓，最去行商老客筋骨陰濕寒氣。」",
            "「艾草香盤佐以野薄荷與松脂，點上一盤，蚊蚋退避、室內清爽，常客長途露宿行囊裡必備數捆。」",
            "「引地熱溫泉水入池，祛寒草木湯包浸透熱浪。泡過盞茶工夫，周身毛孔舒張，筋骨疲乏盡消。」",
        ],
        });
    }

    // 15. 大鍛鐵匠鍛造坊
    if combined.contains("重鐵水錘") || combined.contains("大鍛坊") || combined.contains("鍛造") || combined.contains("鍛修") || combined.contains("車軸鐵箍") || combined.contains("磨刀") || combined.contains("磨刀利刃") || combined.contains("鐵蹄釘掌") || combined.contains("釘掌鋪") || combined.contains("鐵匠") || combined.contains("鍛徒") {
        return Some(RoomVenueObject {
            venue_kind: "大鍛鐵匠鍛造坊",
            default_host_title: "當值鍛造大匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正揮動導能鍛錘將燒紅鐵條重擊成型，火星四濺",
            on_duty_demeanor: "膀大腰圓、肌肉虯結的重甲鐵匠，穿著厚牛皮防燙圍裙，聲若洪鐘、沉穩如山。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0945", "岑氏省力導能鍛錘", 18),
            ("wt_cat_1021", "岩羊角手柄折疊獵刀", 18),
            ("wt_cat_0957", "鍛鋼小羊角錘", 10),
        ],
            trade_talk_pitch: "「千錘百鍊精鋼刀錘與車軸耐磨鐵件，剛柔兼備，經久耐用。」",
            on_duty_talk_lines: &[
            "「水錘借水力轟鳴砸下，震得地面微顫！紅鐵出爐火星四濺，工坊大匠千錘百鍊，造就精鋼刃口。」",
            "「車軸鐵箍經大爐淬火熱套，收緊後與硬木車輪合為一體，任憑山路坎坷，萬里馳騁決不崩裂。」",
            "「打鐵蹄修磨掌，生鐵蹄鐵貼合馬蹄弧度，釘入三寸不傷蹄肉。換上新掌，馱獸奔走如踏平地。」",
        ],
        });
    }

    // 16. 農具五金交易場
    if combined.contains("農具") || combined.contains("大型農具工具行") || combined.contains("五金") || combined.contains("銅鐵五金") || combined.contains("鎖鑰") || combined.contains("鎖簧") || combined.contains("器物交易場") || combined.contains("交易場") {
        return Some(RoomVenueObject {
            venue_kind: "農具五金交易場",
            default_host_title: "當值五金東家",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正檢驗省力導能翻土犁鋼口與熟銅門把碰鎖",
            on_duty_demeanor: "精打細算、熟悉各行工具結構的老司事，手中常拿著黃銅卡尺與樣板。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0950", "省力導能翻土犁", 16),
            ("wt_cat_0685", "熟銅門把帶重鐵碰鎖", 10),
            ("wt_cat_0707", "青銅插銷暗扣雙重鎖", 12),
        ],
            trade_talk_pitch: "「導能翻土犁與熟銅雙重暗扣門鎖，開荒治業必備利器，牢固防撬防鏽。」",
            on_duty_talk_lines: &[
            "「省力導能翻土犁鋼口鋒利，入土三分泥浪翻滾；水力大石磨齒槽精修，磨豆磨面細滑如雪。」",
            "「熟銅碰鎖與青銅插銷雙扣暗簧，防撬防鏽經久耐用。門戶要道安上這把鎖，夜不閉戶也心安。」",
            "「器物交易場上各坊名匠齊聚，從袖珍羅盤到重型千斤頂一應俱全，現鎂過秤、憑單交割。」",
        ],
        });
    }

    // 17. 武備防具巡防所
    if combined.contains("武備") || combined.contains("護商武備鋪") || combined.contains("防具") || combined.contains("火浣布防具坊") || combined.contains("火政") || combined.contains("東門火政巡防所") || combined.contains("習武堂") {
        return Some(RoomVenueObject {
            venue_kind: "武備防具巡防所",
            default_host_title: "當值教頭",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正擦拭厚牛皮卸力護腕並整理火浣布防水長衫",
            on_duty_demeanor: "目光炯炯、身手矯健的武備教官，腰板筆直，言談間充滿行伍警覺。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0846", "卸力厚牛皮護腕", 14),
            ("wt_cat_0840", "鍛造坊厚牛皮防燙圍裙", 16),
            ("wt_cat_0838", "樹膠防水防雨長衫", 18),
        ],
            trade_talk_pitch: "「厚牛皮護腕與防水防燙長衫，耐磨防刺，戈壁涉水野外護身第一選。」",
            on_duty_talk_lines: &[
            "「行商過戈壁山林，護身軟甲與卸力銅護腕是保命行頭。刀刃劈在熟銅上滑開三分，便多一條生路。」",
            "「厚牛皮防燙圍裙防油防刺，火浣布經烈火焚燒不焦。火政巡守手持大鉤水桶，防患於未然。」",
            "「習武堂試招木人被劈得斑駁，拳風破空呼呼作響。出遠門護運隊招募好手，先在場子裡過上三招！」",
        ],
        });
    }

    // 18. 精銅鐘錶儀器坊
    if combined.contains("鐘錶") || combined.contains("儀器坊") || combined.contains("精銅鐘錶儀器坊") || combined.contains("琉璃燈") || combined.contains("白塔琉璃燈坊") || combined.contains("鐘樓") || combined.contains("傳訊鐘樓") || combined.contains("司鐘") {
        return Some(RoomVenueObject {
            venue_kind: "精銅鐘錶儀器坊",
            default_host_title: "當值儀器師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正戴著單目助視鏡微調黃銅座鐘擒縱機構與懷錶發條",
            on_duty_demeanor: "精細微工匠師，雙手沉穩無一絲顫抖，舉手投足恪守規律。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0729", "校準隨身黃銅懷錶", 18),
            ("wt_cat_0728", "岑氏黃銅發條座鐘", 24),
            ("wt_cat_0723", "旋蓋夜光貝母調光台燈", 15),
        ],
            trade_talk_pitch: "「校準黃銅懷錶與夜光貝母調光台燈，走時分毫不差，風吹雨淋光線柔和。」",
            on_duty_talk_lines: &[
            "「發條齒輪精密咬合，黃銅擺錘分毫不差。調準懷錶時刻，各路車船驛使才能嚴絲合縫交接。」",
            "「夜光貝母薄片鑲嵌於旋蓋台燈，光線柔和護眼，狂風吹拂亦不搖晃，藏書閣與船艙必備。」",
            "「信使總站信鴿起落、飛騎待發。八方急件過印登記，晨發文曜，暮達半島各哨，號令通行。」",
        ],
        });
    }

    // 19. 鋸木竹木作坊
    if combined.contains("鋸木坊") || combined.contains("連軸鋸木坊") || combined.contains("竹木行") || combined.contains("三水竹木行") || combined.contains("竹木場") || combined.contains("蘆葦竹木場") || combined.contains("竹編") || combined.contains("藤器") || combined.contains("竹編藤器作坊") || combined.contains("木作") || combined.contains("傢俱") || combined.contains("巧木居所") || combined.contains("榫眼") || combined.contains("木器榫眼作坊") || combined.contains("大木作") || combined.contains("車轅大木作") || combined.contains("大木車船坊") || combined.contains("木匠") {
        return Some(RoomVenueObject {
            venue_kind: "鋸木竹木作坊",
            default_host_title: "當值木作大匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正用水力原木大鋸剖切雲杉樑柱並校準燕尾榫眼",
            on_duty_demeanor: "衣履染著松柏碎木屑的老匠人，沉厚篤實，對木材紋理瞭如指掌。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0947", "水力原木大鋸", 20),
            ("wt_cat_0895", "雙層竹編提籃", 8),
            ("wt_cat_0622", "防靜電松香木梳", 5),
        ],
            trade_talk_pitch: "「水力切剖優質木材、雙層密編竹籃與松香木梳，榫卯精妙，堅固耐用。」",
            on_duty_talk_lines: &[
            "「水車連軸帶動原木大鋸，巨木如剖瓜切菜般破成整齊板材，木香撲鼻，是建房造船的骨幹良料。」",
            "「竹編雙層提籃密不漏水，輕巧韌實耐磨耐摔。漁戶裝魚、當家娘子買菜，用上十年依舊齊整。」",
            "「巧木傢俱純用古法榫卯，燕尾榫、抱肩榫咬合嚴密，不著一顆鐵釘，越是吃重越見緊實。」",
        ],
        });
    }

    // 20. 石材磚瓦窯坊
    if combined.contains("石材切割場") || combined.contains("切石") || combined.contains("磚瓦窯") || combined.contains("灰漿磚瓦窯") || combined.contains("木炭窯") || combined.contains("高山木炭窯") || combined.contains("官窯") || combined.contains("青釉官窯") || combined.contains("銅錫") || combined.contains("器皿精煉坊") || combined.contains("生鐵大棧房") || combined.contains("焦炭生鐵大棧房") || combined.contains("生鐵庫") || combined.contains("粗鹽生鐵庫") {
        return Some(RoomVenueObject {
            venue_kind: "石材磚瓦窯坊",
            default_host_title: "當值窯坊掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正操作水力切石機剖平玄武岩板並點驗窯燒青磚",
            on_duty_demeanor: "膚色黝黑、面容敦厚剛毅的窯坊老工長，雙手粗糙厚實如鐵石。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0949", "銀砂水力切石機", 22),
            ("wt_cat_0944", "紅土耐火磚高溫鐵爐", 20),
            ("wt_cat_0097", "相思木硬炭", 6),
            ("wt_cat_0023", "粗陶紅黏土", 5),
        ],
            trade_talk_pitch: "「高溫耐火爐磚、相思木硬炭與精切玄武石料，百工營造爐竈不可或缺。」",
            on_duty_talk_lines: &[
            "「半嶺水力切石機水霧濛濛，整塊玄武岩巨石剖切平整，鋪在街面百年車輾馬踏不凹陷。」",
            "「青石磚瓦窯烈焰熊熊，出窯青磚敲之有金石之聲。焦炭生鐵大棧房生鐵錠堆積如山，專供鍛坊。」",
            "「青櫟硬木高溫炭化，燃燒無煙且火力持久，是百工冶煉與城中富戶圍爐取暖的上品燃料。」",
        ],
        });
    }

    // 21. 染纈織造服飾坊
    if combined.contains("染布坊") || combined.contains("三水染布坊") || combined.contains("機杼大染坊") || combined.contains("染纈") || combined.contains("織造") || combined.contains("雲錦織造與服飾坊") || combined.contains("服飾坊") || combined.contains("剪剃鋪") || combined.contains("修傘補靴") || combined.contains("修傘") || combined.contains("補靴") || combined.contains("繡娘") {
        return Some(RoomVenueObject {
            venue_kind: "染纈織造服飾坊",
            default_host_title: "當值織染師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於靛藍染缸起布翻曬，丈量棉麻衣袍尺幅",
            on_duty_demeanor: "身段窈窕、十指染著淡淡靛青色的巧手織娘，氣質嫻雅溫和。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0835", "細棉麻透氣常服", 14),
            ("wt_cat_0838", "樹膠防水防雨長衫", 18),
            ("wt_cat_0868", "素面玄武岩扣束腰長袍", 16),
        ],
            trade_talk_pitch: "「細棉麻常服、防水長衫與玄武岩扣長袍，貼身透氣防雨耐磨，四時舒合。」",
            on_duty_talk_lines: &[
            "「機杼聲聲穿梭如流，大圳細棉織就柔軟布匹。紮染絞纈沉入靛藍大缸，出水展曬，雲紋天成！」",
            "「坊內師傅親自量體裁衣，玄武岩扣剪裁外套文質彬彬，厚棉麻常服吸汗耐磨，四時得體。」",
        ],
        });
    }

    // 22. 造船漁具修船塢
    if combined.contains("造船塢") || combined.contains("內河造船塢") || combined.contains("修船塢") || combined.contains("渡口修船塢") || combined.contains("桅帆") || combined.contains("大船桅帆作坊") || combined.contains("帆布麻繩織造坊") || combined.contains("漁網繩索工坊") || combined.contains("修網結繩工坊") || combined.contains("漁具") || combined.contains("簑笠") || combined.contains("漁具簑笠商號") || combined.contains("桐油作坊") || combined.contains("船用桐油作坊") || combined.contains("浮標竹笛作坊") || combined.contains("浮標") || combined.contains("船夫") {
        return Some(RoomVenueObject {
            venue_kind: "造船漁具修船塢",
            default_host_title: "當值船匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正調配桐油海貝灰膏捻縫船板，試拉巨藻船纜",
            on_duty_demeanor: "常年在風浪中曬得黑紅的老水手兼船木師傅，豪邁粗獷，嗓門洪亮。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_1000", "深海巨藻船纜", 18),
            ("wt_cat_1002", "桐油海貝灰捻縫灰膏", 8),
            ("wt_cat_1003", "極淺吃水平底竹筏", 25),
            ("wt_cat_1006", "老青竹黃銅輪捲線釣竿", 10),
        ],
            trade_talk_pitch: "「巨藻船纜、防漏捻縫膏、平底竹筏與老竹捲線釣竿，江海行船下網必備。」",
            on_duty_talk_lines: &[
            "「造船塢內雲杉大龍骨架設完畢，桐油海貝灰捻縫密不透水。大船入水之日，半城人都來看熱鬧。」",
            "「深海巨藻絞成粗大船纜，韌性勝似牛筋，驚濤駭浪中扣住巨錨，是遠洋客船全船人的定心骨。」",
            "「修網結繩需心明手巧，麻線經魚油浸泡耐爛抗腐。老漁翁織出的密目網，連蝦苗都漏不出一隻。」",
        ],
        });
    }

    // 23. 客舍大驛館
    if combined.contains("駝馬大驛站") || combined.contains("萬里駝隊大驛站") || combined.contains("駝馬大驛館") || combined.contains("駝馬大旅社") || combined.contains("長途行商旅社") || combined.contains("旅社") || combined.contains("風塵客商大旅社") || combined.contains("東橋旅社") || combined.contains("客舍") || combined.contains("青雲求學客舍") || combined.contains("歇舍") || combined.contains("行商長途歇舍") || combined.contains("四方匠師行館") || combined.contains("泊客旅舍") || combined.contains("風舶臨水旅社") || combined.contains("北灣臨水旅社") || combined.contains("臨水旅社") || combined.contains("茶屋") || (room_name.contains("行館") && !room_name.contains("印行館")) || extra_hint.contains("茶客") {
        return Some(RoomVenueObject {
            venue_kind: "客舍大驛館",
            default_host_title: "當值驛丞",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正於櫃檯登記入住路引，為火爐添柴煮熱奶茶",
            on_duty_demeanor: "待客周到熱忱的旅社東家，熟悉八方商路與各地風土人情。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0891", "黃銅雙扣厚帆布斜背包", 12),
            ("wt_cat_0897", "薄層重礦烏木膽隨身保溫瓶", 10),
            ("wt_cat_1037", "防倒風野營生鐵柴火爐", 15),
        ],
            trade_talk_pitch: "「雙扣厚帆布背包、隨身保溫瓶與野營防風柴火爐，長途跋涉溫暖隨行。」",
            on_duty_talk_lines: &[
            "「萬里駝隊在此卸鞍歇馬！驛館後院草料溫水管夠，前堂火爐煮著濃奶茶，洗去一身風塵。」",
            "「客舍床榻整潔乾爽，每間備有防倒風野營柴火爐與保溫瓶。行商過客推杯換盞，互通關山道路。」",
            "「四方匠師齊聚行館交流技藝，南腔北調熱鬧非凡。在此歇宿一晚，各地行情掌故全在耳中。」",
        ],
        });
    }

    // 24. 皮毛鞍具帳篷行
    if combined.contains("皮毛大市") || combined.contains("西門皮毛大市") || combined.contains("皮毛大貨棧") || combined.contains("皮貨行") || combined.contains("西門皮貨行") || combined.contains("皮帶鞍套") || combined.contains("皮鞘鞍具") || combined.contains("鞍轡") || combined.contains("遠行鞍轡帳篷行") || combined.contains("帳篷") || combined.contains("行囊修補行") || combined.contains("行囊") || combined.contains("行車雨布作坊") || combined.contains("雨布") {
        return Some(RoomVenueObject {
            venue_kind: "皮毛鞍具帳篷行",
            default_host_title: "當值皮貨東家",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正給熟牛皮馬鞍上蠟保養，縫補雙層防風行商帳篷",
            on_duty_demeanor: "指掌粗糙、腰間插著骨針與刮皮刀的皮貨老東家，性情敦厚老練。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0836", "飛鼠厚絨禦寒冬衣", 22),
            ("wt_cat_0837", "雲豹毛領禦寒大氅", 28),
            ("wt_cat_0892", "軟牛皮分格零錢夾", 8),
            ("wt_cat_1040", "平底木桶式雙槳划艇", 20),
        ],
            trade_talk_pitch: "「雪狼毛領大氅、厚絨禦寒冬衣與牛皮錢夾，荒野防寒抗風不可缺。」",
            on_duty_talk_lines: &[
            "「牛埔與大圳頭送來的雪狼毛領大氅與熟牛皮鞍具，皮質油潤柔軟，披在身上能擋關外徹骨白毛風。」",
            "「雙層防風防水行商帳篷配厚木撐桿，風沙再大也能在戈壁撐起安穩避難所，行商長途必帶。」",
            "「牛皮馬鞍與護膝皆用雙股蠟線縫製，行囊修補師傅手藝精湛，崩了線的背囊片刻修整如新。」",
        ],
        });
    }

    // 25. 大車修造聯運總站
    if combined.contains("雙輪大車坊") || combined.contains("重載雙輪大車坊") || combined.contains("大車修造場") || combined.contains("車馬輪軸修造場") || combined.contains("長途車行") || combined.contains("西道長途車行") || combined.contains("車馬聯運總站") || combined.contains("聯運貨運行") || combined.contains("聯運") || combined.contains("貨運行") || combined.contains("油膏滑脂鋪") || combined.contains("滑脂") || combined.contains("搬運工") || combined.contains("車夫") {
        return Some(RoomVenueObject {
            venue_kind: "大車修造聯運總站",
            default_host_title: "當值車行把式",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正扳動液壓千斤頂頂起重載大車，塗抹輪軸滑脂",
            on_duty_demeanor: "滿手油污但精神奕奕的車馬把式，談吐爽快，深諳車馬運輸規矩。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_1004", "韓氏雙輪駝獸大貨車", 30),
            ("wt_cat_0988", "重載輪軸手動液壓千斤頂", 18),
            ("wt_cat_0985", "駝獸生鐵打蹄鉗與平口銼", 12),
        ],
            trade_talk_pitch: "「雙輪重載貨車、液壓千斤頂與打蹄工具，車隊幹道行馳無礙。」",
            on_duty_talk_lines: &[
            "「重載雙輪大車採用硬木車轅與雙軸滾珠，馱載三千斤生鐵亦行若游水，是通濟大道上的巨無霸。」",
            "「車軸千斤頂輕輕一扳，重車騰空！抹上厚厚一層動物滑脂油膏，輪軸轉動無聲，能多跑千里路。」",
            "「長途車隊每日定時啟程發往半島各鎮。託運箱籠皆掛防水油布，路途再顛簸，貨物完好無損。」",
        ],
        });
    }

    // 26. 牲畜交易獸醫館
    if combined.contains("牲畜交易拍賣行") || combined.contains("牲畜拍賣") || combined.contains("獸醫館") || combined.contains("車馬獸醫館") {
        return Some(RoomVenueObject {
            venue_kind: "牲畜交易獸醫館",
            default_host_title: "當值獸醫館長",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正調配料豆草藥，操作手搖壓捆機壓實草料厚磚",
            on_duty_demeanor: "蓄著灰白短鬚的老獸醫，神色慈和，對各類牲畜馱獸習性了然於心。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_1020", "手搖螺旋壓實乾草料壓捆機", 16),
            ("wt_cat_0969", "重型生鐵熟牛皮防撞砧", 15),
        ],
            trade_talk_pitch: "「螺旋乾草料壓捆機與防撞砧，馱獸調養料草紮實，行路千里的好幫手。」",
            on_duty_talk_lines: &[
            "「拍賣場內號子嘹亮！牛埔的高頭健馬與耐旱駝獸正當場過眼驗齒，體格健碩，嘶鳴如雷。」",
            "「獸醫老館長專治走獸蹄疾與積熱，草藥調拌料豆灌服，針灸兩穴，立時消腫去乏、胃口大開。」",
            "「螺旋壓捆機壓出的乾草料厚磚碼在槽邊，健馬吃飽打個響鼻，渾身皮毛油光發亮。」",
        ],
        });
    }

    // 27. 碼頭商行匯兌護運所
    if combined.contains("貨船大埠") || combined.contains("川流貨船大埠") || combined.contains("北灣貨船大埠") || combined.contains("碼頭匯兌分號") || combined.contains("碼頭匯兌") || combined.contains("巡河哨所") || combined.contains("河工巡防哨居") || combined.contains("哨所") || combined.contains("長途護運哨所") || combined.contains("四方護運行總堂") || combined.contains("四方護運行") || combined.contains("護運行") || combined.contains("關防驗契所") || combined.contains("驗契") || combined.contains("公議堂") || combined.contains("契書") || combined.contains("總商行") || combined.contains("柑仔店") || combined.contains("帳房") || combined.contains("書辦") || combined.contains("通事") {
        return Some(RoomVenueObject {
            venue_kind: "碼頭商行匯兌護運所",
            default_host_title: "當值商行司事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正核驗過關貨單商印，撥算盤匯兌鎂幣存摺",
            on_duty_demeanor: "穿著齊整圓領綢面短衫的帳房先生，算盤珠子撥得飛快，接人待物沉穩精準。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0790", "三水總商行鎂幣分格存摺", 12),
            ("wt_cat_0892", "軟牛皮分格零錢夾", 8),
            ("wt_cat_0891", "黃銅雙扣厚帆布斜背包", 12),
        ],
            trade_talk_pitch: "「總商行鎂幣存摺、分格零錢夾與斜背包，結算便利安全，流通全島無阻。」",
            on_duty_talk_lines: &[
            "「川流大埠貨船一艘接一艘靠岸，起重吊臂沒停過。碼頭匯兌分處可以直接換取鎂幣票據，全半島都能流通。」",
            "「四方護運行總部的護運員經驗豐富，長途車隊隨時注意幹道狀況，有他們隨行押車安穩得多。」",
            "「港務驗單處核對貨物清單與商號印章很仔細，手續辦妥蓋上封章，出城運輸就順暢了。」",
        ],
        });
    }

    // 28. 街巷橋樑公共空地
    if combined.contains("分家巷") || combined.contains("西門街") || combined.contains("講堂街") || combined.contains("石橋") {
        return Some(RoomVenueObject {
            venue_kind: "街巷橋樑公共空地",
            default_host_title: "街坊居民",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正於石橋欄杆旁駐足歇息，觀看街面人流往來",
            on_duty_demeanor: "神情安詳從容的街坊居民，安居樂業，對城中各坊動向頗為熟絡。",
            lendable_tool: None,
            counter_trade_offers: &[],
            trade_talk_pitch: "",
            on_duty_talk_lines: &[
            "「文曜城裡的街道很平整，作息也很規律。白天各行各業正常營業，到了晚上鐘樓報時收班，走在街上很安心。」",
            "「三水口連通外港，分家巷聚集各類工坊，講堂街則是學院與書局的所在地，城裡機能很齊全。」",
            "「初來文曜嗎？如果要找各處場所，順著三水匯流大石橋往四面看，各家招牌都很清楚。」",
        ],
        });
    }

    None
}

/// 統一查詢房間一等物件（涵蓋 37 類作坊政務場域與 28 類市集專賣攤位）
pub fn resolve_any_room_venue_object(room_name: &str, hint: &str) -> Option<RoomVenueObject> {
    resolve_room_venue_object(room_name, hint)
        .or_else(|| resolve_market_or_specialty_room_object(room_name, hint))
}

/// 統一解析房間訪客／顧客／路人等候與參訪環境行為描述（涵蓋全半島文曜及各鄉村據點特色場域）
pub fn resolve_room_ambient_visitor_activity(room_name: &str, seed_idx: usize) -> Option<&'static str> {
    let r = room_name;
    let variants: &[&'static str] = if r.contains("樞紐") || r.contains("石橋") {
        &[
            "憑著雕花青石欄杆遠眺三水川往來貨船",
            "在橋頭日晷石柱旁仰看刻度核對時間",
            "挑著兩擔山海乾貨穩步走過青石拱橋",
        ]
    } else if r.contains("學院") || r.contains("蒙學") {
        &[
            "端坐講堂木案前用鋼尖蘸水硬筆抄錄課卷",
            "手捧新印的《白塔風物誌》與同學低聲切磋",
            "在杉木書架前翻檢四時物候與山海圖譜",
        ]
    } else if r.contains("觀星") {
        &[
            "圍在三百六十星軌銅儀旁記錄星度刻度",
            "在評議廳長案前整理本季的氣象與潮汐觀測表",
        ]
    } else if r.contains("藏書") || r.contains("印行") || (r.contains("書局") && !r.contains("契書")) || r.contains("書舍") {
        &[
            "手持竹骨助視鏡仔細校勘新排印的《半島旬報》",
            "將裁切整齊的講義冊頁穿線裝訂成冊",
            "在多層杉木書架間輕步搜尋地方風物誌卷宗",
        ]
    } else if r.contains("會計") || r.contains("商算") {
        &[
            "專注操作計數板演練帳務核算公式",
            "對照漆板上的出海貨單核算跨鎮運費與匯差",
        ]
    } else if r.contains("剪剃") {
        &[
            "半躺在紫銅溫水管木椅上敷著熱毛巾閉目養神",
            "在亮銀明水鏡前用香柏木梳整理先前修剪好的髮鬢",
        ]
    } else if r.contains("診所") {
        &[
            "在藥櫃前依藥方稱量烘乾的光葉藥草與止血水苔",
            "用青石藥碾將薄荷與青鹽細細研成潔牙粉",
        ]
    } else if r.contains("總商行") {
        &[
            "在厚樟木櫃台後核對防偽鋼印與通兌匯票存根",
            "將收進的實體鎂幣按面額分格歸入樟木錢箱",
            "捧著四方貨單向掌事稟報今日大宗物產行情",
        ]
    } else if r.contains("鐘樓") || r.contains("信使") {
        &[
            "將加蓋火漆印戳的書信分揀裝入雙扣防水郵袋",
            "在信使站廊下替疾行草獸快騎檢查鞍轡與水囊",
        ]
    } else if r.contains("食堂") {
        &[
            "端著熱騰騰出鍋的原牛骨湯與蔥香米烙餅穿梭送餐",
            "在紅鐵大釜旁添柴看火並撇去高湯浮沫",
            "圍在木桌旁大口品嚐熱豆花與切仔湯麵",
        ]
    } else if r.contains("浴場") {
        &[
            "抱著烘得乾爽蓬鬆的蜂格棉面巾替浴客更換",
            "在寒水玉湯池邊泡著溫水洗去一身奔波疲乏",
        ]
    } else if r.contains("公議") || r.contains("契據") || r.contains("契書") {
        &[
            "在長條厚木公議案桌前謄錄今日新簽的屋契地籍",
            "站在水土民務告示立木前審視各街坊修繕委託單",
        ]
    } else if r.contains("鍛造") {
        &[
            "拉動風箱助燃紅土耐火高爐並觀察赤鐵火色",
            "舉起熟鐵鍛錘在鐵砧上反覆鍛打農具與船釘",
            "將燒紅的精鋼刃口浸入冷卻水槽激起一團白霧",
        ]
    } else if r.contains("巧木") || r.contains("傢俱") {
        &[
            "順著松香木紋理推刨木板，刨花如雪片般捲落",
            "拿著木槌與竹釘試拼寒水玉保鮮石櫃的榫卯框體",
        ]
    } else if r.contains("雲錦") || r.contains("服飾") {
        &[
            "在木機旁飛梭走線織造細密挺括的防風棉麻布",
            "拿著軟尺與粉線為客人量裁黑沼防雨樹膠長衫",
        ]
    } else if r.contains("旅社") || r.contains("茶屋") {
        &[
            "提著紅銅長嘴壺為對弈茶客添上熱騰騰的山風草茶",
            "坐在臨水窗邊一邊品嚐鍋煮鮮奶茶一邊翻看旬報",
        ]
    } else if r.contains("習武") || r.contains("試招") {
        &[
            "手握包革練習木刀在青石院中演練卸力步法",
            "站在導能測紋石板前調勻呼吸測試掌力勁道",
        ]
    } else if r.contains("柑仔店") || r.contains("雜貨") {
        &[
            "在貨架前挑選紅磷防風火籤與艾草蚊香盤",
            "拈起一塊古早黑糖麥芽糖笑咪咪地與鄰里寒暄",
        ]
    } else if r.contains("聯運") || r.contains("貨運") || r.contains("西門") {
        &[
            "推著獨輪木車將捆紮結實的貨箱推上生鐵地秤過磅",
            "在青石水槽旁替遠道歸來的雙輪駝獸刷洗毛皮添草料",
            "對照聯運班次木板核對發往大圳頭與風舶角的車次",
        ]
    } else if r.contains("修傘") || r.contains("補靴") {
        &[
            "將淋溼的桐油竹骨傘撐開晾在木架上檢視傘骨",
            "拿著麻線錐子替採購商縫補磨穿的牛皮靴底",
        ]
    } else if r.contains("講堂街") || r.contains("學院巷") || r.contains("書院巷") || r.contains("曬書") || r.contains("竹苑") {
        &[
            "手捧裝訂講義沿著石造拱廊的石道慢步交談",
            "駐足在《半島旬報》閱報木欄前細讀最新一期刊物",
        ]
    } else if r.contains("分家巷") || r.contains("百工") || r.contains("晾木") {
        &[
            "在工坊門前青石水槽邊用天然油磨石打磨隨身工具",
            "扛著新裁好的北山松香木板穿過百工石板巷",
        ]
    } else if r.contains("烘藥") || r.contains("百草") {
        &[
            "在紅泥焙藥爐旁照看火候，藥香濃郁沁鼻",
            "手持切藥鍘刀仔細修整晾乾的藥草根莖",
        ]
    } else if r.contains("造紙") || r.contains("紙廬") || r.contains("精廬") {
        &[
            "在石槽旁蕩料抄紙，水花均勻泛起",
            "拿著毛刷將濕紙平整貼於熱牆焙乾",
        ]
    } else if r.contains("書齋") || r.contains("墨香") || r.contains("居舍") || r.contains("民宅") {
        &[
            "在沉水烏木書案前研墨習字，神情安恬專注",
            "推開雕花木窗迎入微風，靜心翻閱案上線裝冊頁",
        ]
    } else if r.contains("河灣") || r.contains("北渠") || r.contains("水脈") || r.contains("東灣") || r.contains("竹筏") {
        &[
            "在青石埠階旁繫穩竹筏繩纜，掬水洗面",
            "整理汲水木桶與竹編魚簍，聽水聲潺潺流淌",
        ]
    } else if r.contains("水輪") || r.contains("石磨") || r.contains("磨坊") {
        &[
            "背著一袋新收稻穀在石磨槽旁等候過篩出粉",
            "站在引水石渠邊端詳巨型木製水輪平穩運轉",
        ]
    } else if r.contains("鋸木") || r.contains("木場") || r.contains("林場") {
        &[
            "在堆木坪旁丈量新鋸開的樟木與杉木大料",
            "拂去衣袖上的松木鋸屑，清點綑紮妥當的板材",
        ]
    } else if r.contains("切石") || r.contains("石坊") {
        &[
            "在導水冷卻槽旁端詳切整平滑的青石板材紋理",
            "用木尺比對柱礎石料的邊角尺寸",
        ]
    } else if r.contains("乳酪") || r.contains("牧場") {
        &[
            "提著木桶在乳酪壓榨架旁等候新鮮凝乳分裝",
            "在牧草棚下餵食溫馴的乳牛與草獸",
        ]
    } else if r.contains("選礦") || r.contains("磚窯") || r.contains("礦") {
        &[
            "推著裝滿赤鐵原礦的木輪車沿軌道緩步前行",
            "在磚窯外棚檢視出窯紅磚的成色與敲擊聲響",
        ]
    } else if r.contains("鹽") || r.contains("漁") || r.contains("船塢") || r.contains("碼頭") || r.contains("渡口") || r.contains("港") {
        &[
            "在岸邊石坪整理晾乾的麻繩漁網與竹編魚簍",
            "站在棧橋邊核對靠岸貨船卸下的海鹽與魚乾",
        ]
    } else {
        &[
            "提著竹編提籃在三水口商號間採辦日用雜貨",
            "沿著平整的青石板街與相熟的街坊點頭寒暄",
        ]
    };
    Some(variants[seed_idx % variants.len()])
}

impl GridCell {
    pub fn new(coord: SquareCoord, terrain: Terrain, name: impl Into<String>) -> Self {
        let name_str = name.into();
        let tags = derive_room_tags(terrain, &name_str, "");
        Self {
            coord,
            terrain,
            name: name_str,
            zone: String::new(),
            tags,
            description: String::new(),
            objects: Vec::new(),
            explored: false,
            freshness: Freshness::Fresh,
            explicit_exits: Vec::new(),
        }
    }

    /// 取得格子目前所有生效的語意標籤（包含持久化標籤與動態推導標籤）
    pub fn effective_tags(&self) -> Vec<String> {
        let mut all = self.tags.clone();
        for t in derive_room_tags(self.terrain, &self.name, &self.zone) {
            if !all.contains(&t) {
                all.push(t);
            }
        }
        all
    }

    /// 檢查格子是否具備指定的語意標籤
    pub fn has_room_tag(&self, wanted: &str) -> bool {
        if wanted.is_empty() {
            return false;
        }
        if self.tags.iter().any(|t| t == wanted || t.contains(wanted)) {
            return true;
        }
        let derived = derive_room_tags(self.terrain, &self.name, &self.zone);
        if derived.iter().any(|t| t == wanted || t.contains(wanted)) {
            return true;
        }
        if self.terrain == Terrain::House
            || is_private_residence_room_name(&self.name)
            || is_street_or_gate_room_name(&self.name)
        {
            return false;
        }
        self.name.contains(wanted)
    }

    /// 依房間名稱與分區推導結構化房間設施物件
    pub fn venue_object(&self) -> Option<RoomVenueObject> {
        if self.terrain == Terrain::House || is_private_residence_room_name(&self.name) {
            return None;
        }
        resolve_any_room_venue_object(&self.name, &self.zone)
    }
}

// ─── 自訂 Serialize / Deserialize（cells 轉 Vec 以相容 JSON）───

/// 方格地圖：world_seed + 已揭露格子集合。
///
/// 格子以 `SquareCoord` 為鍵存入 `HashMap`；序列化時轉成 `Vec`
/// 與 `HexGrid` 慣例一致。未插入 = 黑格（未揭露）。
///
/// 相鄰格之間預設八方皆通；只有不可行走地形（Water、Mountain 等）阻斷移動。
#[derive(Debug, Clone, Default)]
pub struct SquareGrid {
    /// 揭露用 PRNG 種子（與座標混合；見 `reveal` 模組）
    world_seed: u64,
    cells: Arc<HashMap<SquareCoord, GridCell>>,
    /// 資料是否被修改（格數不變但內容變動時設為 true）
    dirty: bool,
}

/// 序列化用的中間結構（cells 轉 Vec，符合 JSON 慣例）
#[derive(Serialize, Deserialize)]
struct SquareGridSer {
    #[serde(default)]
    world_seed: u64,
    cells: Vec<GridCell>,
}

impl Serialize for SquareGrid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let ser = SquareGridSer {
            world_seed: self.world_seed,
            cells: self.cells.values().cloned().collect(),
        };
        ser.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SquareGrid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ser = SquareGridSer::deserialize(deserializer)?;
        let cells = ser.cells.into_iter().map(|c| (c.coord, c)).collect();
        Ok(SquareGrid {
            world_seed: ser.world_seed,
            cells: Arc::new(cells),
            dirty: false,
        })
    }
}

// ─── Cell CRUD ───────────────────────────────────────────────────

impl SquareGrid {
    /// 建立空地圖（world_seed 預設 0，可用 `set_world_seed` 覆蓋）
    pub fn new() -> Self {
        Self::default()
    }

    pub fn world_seed(&self) -> u64 {
        self.world_seed
    }

    pub fn set_world_seed(&mut self, seed: u64) {
        self.world_seed = seed;
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// 插入格子；若座標已存在則覆蓋並回傳舊值。
    pub fn insert(&mut self, cell: GridCell) -> Option<GridCell> {
        let coord = cell.coord;
        self.dirty = true;
        Arc::make_mut(&mut self.cells).insert(coord, cell)
    }

    /// 移除格子；不存在時回傳 `None`。
    pub fn remove(&mut self, coord: SquareCoord) -> Option<GridCell> {
        let r = Arc::make_mut(&mut self.cells).remove(&coord);
        if r.is_some() {
            self.dirty = true;
        }
        r
    }
    /// 是否為顯式儲存的格子（而非依據種子即時生成的荒野格）
    pub fn has_explicit_cell(&self, coord: SquareCoord) -> bool {
        self.cells.contains_key(&coord)
    }

    /// 是否啟用白塔半島 800×1,000 無縫荒野基底層
    #[inline]
    pub fn has_peninsula_wilderness(&self) -> bool {
        self.world_seed == 2026_07_03 || self.cells.len() > 2000
    }

    /// 取得格子（未揭露回傳 `None`）
    pub fn get(&self, coord: SquareCoord) -> Option<&GridCell> {
        if let Some(c) = self.cells.get(&coord) {
            return Some(c);
        }
        if self.has_peninsula_wilderness() && super::r#gen::in_peninsula_bounds(coord) {
            return super::r#gen::procedural_wilderness_cell_ref(coord);
        }
        None
    }

    /// 取得格子（可寫）——呼叫者修改內容後應呼叫 `mark_dirty()`。
    pub fn get_mut(&mut self, coord: SquareCoord) -> Option<&mut GridCell> {
        if !self.cells.contains_key(&coord)
            && self.has_peninsula_wilderness()
            && super::r#gen::in_peninsula_bounds(coord)
        {
            let cell = super::r#gen::materialize_wilderness_cell(coord);
            Arc::make_mut(&mut self.cells).insert(coord, cell);
            self.dirty = true;
        }
        Arc::make_mut(&mut self.cells).get_mut(&coord)
    }

    /// 查詢格子是否已揭露
    pub fn contains(&self, coord: SquareCoord) -> bool {
        self.cells.contains_key(&coord)
            || (self.has_peninsula_wilderness() && super::r#gen::in_peninsula_bounds(coord))
    }

    /// 已揭露格子總數
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// 虛擬全圖格數（半島正典為 800,000 格）
    pub fn total_virtual_cells(&self) -> usize {
        if self.has_peninsula_wilderness() {
            super::r#gen::TOTAL_CELLS
        } else {
            self.cells.len()
        }
    }


    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// 遍歷所有已揭露格子
    pub fn cells(&self) -> impl Iterator<Item = &GridCell> {
        self.cells.values()
    }

    /// 遍歷所有已揭露格子（可寫；修改後應標記 dirty）
    pub fn cells_mut(&mut self) -> impl Iterator<Item = &mut GridCell> {
        Arc::make_mut(&mut self.cells).values_mut()
    }

    /// 遍歷所有已揭露座標
    pub fn coords(&self) -> impl Iterator<Item = &SquareCoord> {
        self.cells.keys()
    }

    /// 依當前座標 `(cx, cy)` 由近而遠（曼哈頓距離環 `0..=max_radius`）搜尋具備任一指定語意標籤的房間格。
    /// 僅檢查顯式建築／城鎮格（`self.cells`），避免觸發荒野程序生成；同距離環內以 `seed` 決定性挑選。
    pub fn find_nearest_room_by_tags(
        &self,
        center: (i32, i32),
        max_radius: i32,
        required_tags: &[&str],
        seed: u64,
    ) -> Option<((i32, i32), String)> {
        if required_tags.is_empty() || max_radius < 0 {
            return None;
        }
        let (cx, cy) = center;
        for dist in 0..=max_radius {
            let mut matches_in_ring: Vec<((i32, i32), String)> = Vec::new();
            if dist == 0 {
                let coord = SquareCoord::new(cx, cy);
                if let Some(cell) = self.cells.get(&coord)
                    && !cell.name.is_empty()
                    && !cell.name.contains("內室")
                    && required_tags.iter().any(|t| cell.has_room_tag(t))
                {
                    return Some(((cx, cy), cell.name.clone()));
                }
                continue;
            }
            for dx in -dist..=dist {
                let rem = dist - dx.abs();
                let dy_candidates = if rem == 0 { [0, 0] } else { [-rem, rem] };
                let count = if rem == 0 { 1 } else { 2 };
                for &dy in &dy_candidates[..count] {
                    let coord = SquareCoord::new(cx + dx, cy + dy);
                    if let Some(cell) = self.cells.get(&coord)
                        && !cell.name.is_empty()
                        && !cell.name.contains("內室")
                        && required_tags.iter().any(|t| cell.has_room_tag(t))
                    {
                        matches_in_ring.push(((coord.x, coord.y), cell.name.clone()));
                    }
                }
            }
            if !matches_in_ring.is_empty() {
                matches_in_ring.sort_by(|a, b| a.0.cmp(&b.0));
                let idx = (seed as usize) % matches_in_ring.len();
                return matches_in_ring.into_iter().nth(idx);
            }
        }
        None
    }
}

// ─── 鄰接 & 移動 ─────────────────────────────────────────────────

impl SquareGrid {
    /// 回傳已存在的相鄰格（八方向，不考慮地形通行性）
    pub fn neighbors(&self, coord: SquareCoord) -> Vec<&GridCell> {
        coord
            .neighbors()
            .into_iter()
            .filter_map(|n| self.get(n))
            .collect()
    }

    /// 能否從 `from` 步行到相鄰的 `to`？（別名方法）
    pub fn can_move_between(&self, from: SquareCoord, to: SquareCoord) -> bool {
        self.can_walk(from, to)
    }

    pub fn has_direct_non_building_walkable_exit(&self, coord: SquareCoord) -> bool {
        for &(dx, dy) in &[(0, 1), (0, -1), (1, 0), (-1, 0)] {
            let nb = SquareCoord::new(coord.x + dx, coord.y + dy);
            if let Some(c) = self.get(nb) {
                if c.terrain.walkable()
                    && c.terrain.category() != "landmark"
                    && c.terrain != Terrain::Water
                {
                    return true;
                }
            }
        }
        false
    }

    /// 能否從 `from` 步行到相鄰的 `to`？
    ///
    /// 若 `from` 有顯式出口（explicit_exits），以出口表為準；
    /// 否則回退到座標鄰接邏輯（Manhattan 距離 1、地形可走、街道與公開建築格暢通、建築隔離法則）。
    pub fn can_walk(&self, from: SquareCoord, to: SquareCoord) -> bool {
        let Some(from_cell) = self.get(from) else {
            return false;
        };
        let Some(to_cell) = self.get(to) else {
            return false;
        };

        // 顯式出口覆蓋座標邏輯
        if !from_cell.explicit_exits.is_empty() {
            return from_cell.explicit_exits.iter().any(|e| e.target == to);
        }

        // Fallback: 座標鄰接邏輯
        if from.manhattan_distance(to) != 1 {
            return false;
        }
        if !to_cell.terrain.walkable() {
            return false;
        }

        // 街道 :: 與相鄰公開建築格（$ ⚒︎ + ⌂ ⚑ 𖠿 ∆）互相開放通行
        let is_bldg = |t: Terrain| -> bool {
            t.category() == "landmark"
        };
        let is_road = |t: Terrain| -> bool {
            t == Terrain::Road
        };

        if is_road(from_cell.terrain) && is_bldg(to_cell.terrain) {
            return true;
        }
        if is_bldg(from_cell.terrain) && is_road(to_cell.terrain) {
            return true;
        }

        // 城鎮內河水道隔離：禁止由建築或城鎮街道直接踏入深渠水體（須經石橋 ≍ 通行）
        if (from_cell.terrain == Terrain::Water || to_cell.terrain == Terrain::Water)
            && (!from_cell.zone.is_empty() || !to_cell.zone.is_empty() || is_bldg(from_cell.terrain) || is_bldg(to_cell.terrain))
        {
            return false;
        }

        // 建築隔離法則：禁止建築物 -> 建築物 的直接穿牆（同名複合建築格除外）
        if is_bldg(from_cell.terrain) && is_bldg(to_cell.terrain) {
            if from_cell.name != to_cell.name
                && self.has_direct_non_building_walkable_exit(from)
                && self.has_direct_non_building_walkable_exit(to)
            {
                return false;
            }
        }
        true
    }

    /// 動態改變網格地形（10 條動態地形演變鏈）。
    ///
    /// 成功時回傳 `Ok((old_terrain, new_terrain, 描述字串))`，並將網格標記為 dirty。
    pub fn mutate_cell_terrain(
        &mut self,
        coord: SquareCoord,
        action: &str,
    ) -> Result<(Terrain, Terrain, String), String> {
        let cell = self.get_mut(coord).ok_or_else(|| {
            format!("座標 ({}, {}) 處無有效網格", coord.x, coord.y)
        })?;
        let old = cell.terrain;
        let (new_terrain, desc) = match (old, action) {
            // 1. 開山挖礦 / 坍方還山
            (Terrain::Mountain, "mine" | "quarry" | "excavate") => (
                Terrain::MinePass,
                "劈石開道，山體岩壁被鑿出可供通行的礦道山隘。".to_string(),
            ),
            (Terrain::MinePass, "collapse" | "restore_mountain") => (
                Terrain::Mountain,
                "岩層坍塌封死礦道，重歸高聳嶙峋的實心山脊。".to_string(),
            ),

            // 2. 伐林清地
            (
                Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle,
                "deforest" | "clear_trees" | "logging",
            ) => (
                Terrain::Plain,
                "伐倒巨木灌叢、清理樹樁，林地化為一片開闊平野。".to_string(),
            ),

            // 3. 開墾整地
            (Terrain::Plain | Terrain::Grassland, "plow" | "till" | "cultivate") => (
                Terrain::EmptyFurrow,
                "犁開熟土、起壟除草，平野闢為待播種之空田壟。".to_string(),
            ),

            // 4. 依栽種變化、收成與休耕復育
            (Terrain::EmptyFurrow, "plant_paddy" | "irrigate" | "plant_rice") => (
                Terrain::FarmField,
                "引清流灌渠入壟，阡陌縱橫，注滿水光映照之水田。".to_string(),
            ),
            (Terrain::EmptyFurrow, "plant_dry_crop" | "plant_wheat" | "dry_field") => (
                Terrain::DryField,
                "耙土覆麥、起溝避澇，田壟化作耐旱麥浪之旱田。".to_string(),
            ),
            (Terrain::EmptyFurrow, "plant_herbs" | "herb_garden") => (
                Terrain::HerbGarden,
                "移栽靈草仙葩、精細分畦，泥壟闢成藥香瀰漫之藥圃。".to_string(),
            ),
            (Terrain::EmptyFurrow, "afforest" | "plant_trees") => (
                Terrain::Forest,
                "栽植樹苗防風固土，田壟漸次鬱閉為青蔥林木。".to_string(),
            ),
            (
                Terrain::FarmField | Terrain::DryField | Terrain::HerbGarden,
                "harvest" | "clear_field",
            ) => (
                Terrain::EmptyFurrow,
                "收割豐實作物，田畦重歸平整空壟，靜候下季播種。".to_string(),
            ),
            (Terrain::EmptyFurrow, "fallow" | "abandon_field") => (
                Terrain::Plain,
                "停歇耕作任野草蔓延，田壟風化還原為原野平地。".to_string(),
            ),

            // 5. 踩踏鋪路 / 道路廢弛
            (Terrain::Plain | Terrain::Grassland, "pave_road" | "tread_path" | "build_road") => (
                Terrain::Road,
                "夯實路基鋪設平整石板，踩出一條四通八達之道途。".to_string(),
            ),
            (Terrain::Road, "break_road" | "abandon_road" | "remove_road") => (
                Terrain::Plain,
                "掘起石板任草木復生，道路風化還原為尋常土地。".to_string(),
            ),

            // 6. 百工造橋 / 橋樑毀損
            (Terrain::Water, "build_bridge") => (
                Terrain::Bridge,
                "立墩架樑、鋪設石木橋板，清波之上升起連通兩岸之飛橋。".to_string(),
            ),
            (Terrain::Bridge, "demolish_bridge" | "collapse_bridge") => (
                Terrain::Water,
                "橋樑拆除倒塌，河川水流暢通，復還為一泓碧水。".to_string(),
            ),

            // 7. 水文漲退
            (Terrain::Plain | Terrain::Grassland, "flood" | "inundate") => (
                Terrain::Swamp,
                "潮汛上漲、地勢凹陷聚水，土地浸軟化作蘆荻泥澤。".to_string(),
            ),
            (Terrain::Swamp, "drain" | "reclaim_swamp") => (
                Terrain::Plain,
                "疏浚導流、填石排澇，泥濘澤地重現為堅實乾爽之平原。".to_string(),
            ),

            // 8. 聚落營造與拆除
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_house" | "build_home",
            ) => (
                Terrain::House,
                "起基立柱、覆瓦築垣，落成一棟溫暖樸實之民宅。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_market" | "build_shop",
            ) => (
                Terrain::Market,
                "搭建貨架攤棚、懸掛商號錦旗，闢出一處繁華商貿之市集。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_workshop" | "build_blacksmith",
            ) => (
                Terrain::Workshop,
                "置爐安砧、排設風箱機關，落成一座百工鍛造之工坊。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_clinic" | "build_apothecary",
            ) => (
                Terrain::Clinic,
                "配置診療床、草木調理台與自然脈絡感應儀，落成一間脈絡診所。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_inn" | "build_tavern",
            ) => (
                Terrain::Inn,
                "結綵挑燈、陳設几榻佳釀，營造一處迎送行旅之現代行館。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_fence",
            ) => (
                Terrain::Fence,
                "深埋排樁、橫綁堅木防禦，立起阻絕侵擾之木柵圍欄。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_wall",
            ) => (
                Terrain::Wall,
                "條石錯縫、灌漿版築，壘砌起厚重巍峨之實心城牆。".to_string(),
            ),
            (
                Terrain::House
                | Terrain::Market
                | Terrain::Workshop
                | Terrain::Clinic
                | Terrain::Inn
                | Terrain::Fence
                | Terrain::Wall
                | Terrain::Blacksmith
                | Terrain::GeneralStore
                | Terrain::Farmhouse
                | Terrain::Warehouse
                | Terrain::Granary,
                "demolish" | "tear_down",
            ) => (
                Terrain::Plain,
                "棟樑傾覆、磚石清運，建物平毀為一片乾淨平地。".to_string(),
            ),

            // 9. 荒野復育 / 植被演替
            (Terrain::Plain | Terrain::Grassland, "grow_forest" | "wild_growth") => (
                Terrain::Forest,
                "草木滋長、嘉木成蔭，日光穿透繁枝形成幽謐林境。".to_string(),
            ),

            // 10. 沙石沉積 / 海蝕沖刷
            (Terrain::Water, "sediment" | "deposit_sand") => (
                Terrain::Beach,
                "潮沙堆積、水勢退讓，湧浪邊緣沉澱出細密沙礫之海灘。".to_string(),
            ),
            (Terrain::Beach, "erode" | "sea_wash") => (
                Terrain::Water,
                "波濤沖刷侵蝕岸線，沙灘隨浪散逸復沒入清澈淺水。".to_string(),
            ),
            (Terrain::Beach, "solidify_land") => (
                Terrain::Plain,
                "耐鹽草根固著泥沙，海灘化作堅實平整之沿岸草原。".to_string(),
            ),

            _ => {
                return Err(format!(
                    "地形 {:?} 不支援演變動作 '{}'",
                    old, action
                ));
            }
        };

        cell.terrain = new_terrain;
        self.dirty = true;
        Ok((old, new_terrain, desc))
    }

    /// BFS 最短路徑（跳數）；回傳包含起點與終點的座標序列。
    ///
    /// - `from == to`：回傳 `Some(vec![from])`
    /// - 起點或終點不在地圖中：回傳 `None`
    /// - 不可達：回傳 `None`
    pub fn find_path(&self, from: SquareCoord, to: SquareCoord) -> Option<Vec<SquareCoord>> {
        if from == to {
            return Some(vec![from]);
        }
        if !self.contains(from) || !self.contains(to) {
            return None;
        }

        let mut visited: HashSet<SquareCoord> = HashSet::new();
        let mut parent: HashMap<SquareCoord, SquareCoord> = HashMap::new();
        let mut queue: VecDeque<SquareCoord> = VecDeque::new();

        visited.insert(from);
        queue.push_back(from);

        while let Some(current) = queue.pop_front() {
            for next in current.neighbors() {
                if !self.can_walk(current, next) || visited.contains(&next) {
                    continue;
                }
                visited.insert(next);
                parent.insert(next, current);

                if next == to {
                    // 回溯路徑：from … to
                    let mut path = vec![to];
                    let mut c = to;
                    while let Some(&p) = parent.get(&c) {
                        path.push(p);
                        c = p;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(next);
            }
        }
        None
    }
}

// ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn plain(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Plain, format!("({x},{y})"))
    }

    fn water(x: i32, y: i32) -> GridCell {
        GridCell::new(
            SquareCoord::new(x, y),
            Terrain::WaterDeep,
            format!("深水({x},{y})"),
        )
    }

    fn road(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Road, format!("街道({x},{y})"))
    }

    fn shop(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Market, format!("商號({x},{y})"))
    }

    fn workshop(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Workshop, format!("工坊({x},{y})"))
    }

    // ── 基本 CRUD ───────────────────────────────────────────────

    #[test]
    fn insert_and_get() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(1, 0));
        assert_eq!(g.len(), 2);
        assert!(g.contains(SquareCoord::new(0, 0)));
        assert!(!g.contains(SquareCoord::new(5, 5)));
        assert!(g.get(SquareCoord::new(1, 0)).is_some());
        assert_eq!(
            g.get(SquareCoord::new(1, 0)).unwrap().terrain,
            Terrain::Plain
        );
    }

    #[test]
    fn insert_overwrites_and_returns_old() {
        let mut g = SquareGrid::new();
        // 第一次插入，無舊值
        let old = g.insert(plain(0, 0));
        assert!(old.is_none());

        // 第二次插入同座標，應覆蓋並回傳舊格子
        let mut overwrite = plain(0, 0);
        overwrite.terrain = Terrain::Forest;
        let old2 = g.insert(overwrite);
        assert!(old2.is_some());
        assert_eq!(old2.unwrap().terrain, Terrain::Plain);
        assert_eq!(
            g.get(SquareCoord::new(0, 0)).unwrap().terrain,
            Terrain::Forest
        );
    }

    #[test]
    fn remove_cell() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        let removed = g.remove(SquareCoord::new(0, 0));
        assert!(removed.is_some());
        assert!(g.is_empty());
        assert!(g.remove(SquareCoord::new(0, 0)).is_none());
    }

    #[test]
    fn get_mut_modifies_in_place() {
        let mut g = SquareGrid::new();
        g.insert(plain(2, 3));
        g.get_mut(SquareCoord::new(2, 3)).unwrap().explored = true;
        assert!(g.get(SquareCoord::new(2, 3)).unwrap().explored);
    }

    #[test]
    fn world_seed_roundtrip() {
        let mut g = SquareGrid::new();
        g.set_world_seed(0xDEAD_BEEF_1234_5678);
        assert_eq!(g.world_seed(), 0xDEAD_BEEF_1234_5678);
    }

    // ── 序列化 ──────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let mut g = SquareGrid::new();
        g.set_world_seed(0xABCD_EF01_2345_6789);
        g.insert(plain(0, 0));
        g.insert(water(1, 0));

        let json = serde_json::to_string(&g).unwrap();
        let back: SquareGrid = serde_json::from_str(&json).unwrap();

        assert_eq!(back.world_seed(), 0xABCD_EF01_2345_6789);
        assert_eq!(back.len(), 2);
        assert!(back.contains(SquareCoord::new(0, 0)));
        assert_eq!(
            back.get(SquareCoord::new(1, 0)).unwrap().terrain,
            Terrain::WaterDeep
        );
    }

    #[test]
    fn serde_zone_and_tags() {
        let mut cell = plain(5, 5);
        cell.zone = "野外".into();
        cell.tags = vec!["危險".into(), "資源豐富".into()];

        let mut g = SquareGrid::new();
        g.insert(cell);
        let json = serde_json::to_string(&g).unwrap();
        let back: SquareGrid = serde_json::from_str(&json).unwrap();
        let c = back.get(SquareCoord::new(5, 5)).unwrap();
        assert_eq!(c.zone, "野外");
        assert_eq!(c.tags, vec!["危險", "資源豐富"]);
    }

    // ── 尋路：直線 ──────────────────────────────────────────────

    #[test]
    fn find_path_straight_line() {
        let mut g = SquareGrid::new();
        // 水平直線：(0,0) → (4,0)
        for x in 0..=4_i32 {
            g.insert(plain(x, 0));
        }
        let path = g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(4, 0))
            .unwrap();
        assert_eq!(path.first(), Some(&SquareCoord::new(0, 0)));
        assert_eq!(path.last(), Some(&SquareCoord::new(4, 0)));
        // BFS 直線最短路：5 格（含起終點）
        assert_eq!(path.len(), 5);
    }

    #[test]
    fn find_path_diagonal() {
        // (0,0) → (3,3)：四向制下 Manhattan 距離 6，路徑 7 格（含起終點）。
        // 需要插入完整 4x4 可走區塊。
        let mut g = SquareGrid::new();
        for x in 0..=3_i32 {
            for y in 0..=3_i32 {
                g.insert(plain(x, y));
            }
        }
        let path = g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(3, 3))
            .unwrap();
        assert_eq!(path.len(), 7);
        assert_eq!(path.first(), Some(&SquareCoord::new(0, 0)));
        assert_eq!(path.last(), Some(&SquareCoord::new(3, 3)));
    }

    #[test]
    fn find_path_same_cell() {
        let mut g = SquareGrid::new();
        g.insert(plain(3, 3));
        let path = g
            .find_path(SquareCoord::new(3, 3), SquareCoord::new(3, 3))
            .unwrap();
        assert_eq!(path, vec![SquareCoord::new(3, 3)]);
    }

    // ── 尋路：繞過障礙 ──────────────────────────────────────────

    #[test]
    fn find_path_around_water_obstacle() {
        // 佈局（y 軸向北為正）：
        //   (0,0) — 水(1,0) — (2,0)
        //   (0,-1) — (1,-1) — (2,-1)
        //
        // 直路被水擋，BFS 應繞南方一行（y = -1）
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        g.insert(water(1, 0)); // 障礙
        g.insert(plain(2, 0));
        g.insert(plain(0, -1));
        g.insert(plain(1, -1));
        g.insert(plain(2, -1));

        let path = g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(2, 0))
            .unwrap();

        // 路徑不得踩水格
        assert!(!path.contains(&SquareCoord::new(1, 0)));
        // 起點與終點正確
        assert_eq!(path.first(), Some(&SquareCoord::new(0, 0)));
        assert_eq!(path.last(), Some(&SquareCoord::new(2, 0)));
    }

    // ── 尋路：不可達 ────────────────────────────────────────────

    #[test]
    fn find_path_unreachable() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(10, 10)); // 孤立格，中間無路

        assert!(
            g.find_path(SquareCoord::new(0, 0), SquareCoord::new(10, 10))
                .is_none()
        );
    }

    #[test]
    fn find_path_blocked_by_water_ring() {
        // 目標格 (3,3) 被水格完整包圍，從 (0,0) 不可達
        //
        // 水格：(2,2)(3,2)(4,2)
        //       (2,3)     (4,3)
        //       (2,4)(3,4)(4,4)
        // 目標：(3,3)
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0)); // 起點（離目標有段距離）

        // 圍牆（全水）
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                if dx != 0 || dy != 0 {
                    g.insert(water(3 + dx, 3 + dy));
                }
            }
        }
        g.insert(plain(3, 3)); // 目標，被水圍死

        assert!(
            g.find_path(SquareCoord::new(0, 0), SquareCoord::new(3, 3))
                .is_none()
        );
    }

    #[test]
    fn find_path_missing_endpoint_is_none() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        // 終點不在地圖中
        assert!(
            g.find_path(SquareCoord::new(0, 0), SquareCoord::new(1, 0))
                .is_none()
        );
    }

    #[test]
    fn road_and_building_traversal() {
        let mut g = SquareGrid::new();
        g.insert(road(0, 0));
        g.insert(shop(1, 0));
        g.insert(workshop(2, 0));
        g.insert(road(3, 0));

        // 街道 :: 踏入商貿 $
        assert!(g.can_move_between(SquareCoord::new(0, 0), SquareCoord::new(1, 0)));
        // 商貿 $ 踏回街道 ::
        assert!(g.can_move_between(SquareCoord::new(1, 0), SquareCoord::new(0, 0)));
        // 建築隔離法則：兩座皆有街道出口的建築，商貿 $ 直接穿牆到工坊 ⚒︎ 應被禁止
        assert!(!g.can_move_between(SquareCoord::new(1, 0), SquareCoord::new(2, 0)));
    }

    #[test]
    fn dynamic_mutation_chains() {
        let mut g = SquareGrid::new();
        let c = SquareCoord::new(0, 0);

        // 1. 開山挖礦 Mountain -> MinePass -> Mountain
        g.insert(GridCell::new(c, Terrain::Mountain, "山峰".to_string()));
        let (old, new_t, _) = g.mutate_cell_terrain(c, "mine").unwrap();
        assert_eq!(old, Terrain::Mountain);
        assert_eq!(new_t, Terrain::MinePass);
        let (_, new_t2, _) = g.mutate_cell_terrain(c, "collapse").unwrap();
        assert_eq!(new_t2, Terrain::Mountain);

        // 2. 伐林清地 Forest -> Plain
        g.insert(GridCell::new(c, Terrain::Forest, "樹林".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "deforest").unwrap();
        assert_eq!(new_t, Terrain::Plain);

        // 3. 開墾整地 Plain -> EmptyFurrow
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plow").unwrap();
        assert_eq!(new_t, Terrain::EmptyFurrow);

        // 4. 栽種變化 EmptyFurrow -> FarmField -> EmptyFurrow -> DryField
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plant_paddy").unwrap();
        assert_eq!(new_t, Terrain::FarmField);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "harvest").unwrap();
        assert_eq!(new_t, Terrain::EmptyFurrow);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plant_dry_crop").unwrap();
        assert_eq!(new_t, Terrain::DryField);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "harvest").unwrap();
        assert_eq!(new_t, Terrain::EmptyFurrow);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plant_herbs").unwrap();
        assert_eq!(new_t, Terrain::HerbGarden);

        // 5. 鋪路 Plain <-> Road
        g.insert(GridCell::new(c, Terrain::Plain, "草野".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "pave_road").unwrap();
        assert_eq!(new_t, Terrain::Road);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "break_road").unwrap();
        assert_eq!(new_t, Terrain::Plain);

        // 6. 造橋 Water <-> Bridge
        g.insert(GridCell::new(c, Terrain::Water, "溪流".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "build_bridge").unwrap();
        assert_eq!(new_t, Terrain::Bridge);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "demolish_bridge").unwrap();
        assert_eq!(new_t, Terrain::Water);

        // 7. 水文漲退 Plain <-> Swamp
        g.insert(GridCell::new(c, Terrain::Plain, "荒野".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "flood").unwrap();
        assert_eq!(new_t, Terrain::Swamp);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "drain").unwrap();
        assert_eq!(new_t, Terrain::Plain);

        // 8. 聚落營造 Plain -> House -> demolish -> Plain
        let (_, new_t, _) = g.mutate_cell_terrain(c, "build_house").unwrap();
        assert_eq!(new_t, Terrain::House);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "demolish").unwrap();
        assert_eq!(new_t, Terrain::Plain);
    }

    #[test]
    fn derive_room_tags_and_cell_has_room_tag() {
        let granary = GridCell::new(SquareCoord::new(0, 0), Terrain::Granary, "大圳頭公有糧倉");
        assert!(granary.has_room_tag("穀物"));
        assert!(granary.has_room_tag("糧倉"));
        assert!(granary.has_room_tag("櫃檯"));
        assert!(granary.has_room_tag("非本店人員禁止進入區"));

        let smithy = GridCell::new(SquareCoord::new(1, 0), Terrain::Blacksmith, "西嶺百鍛鐵工坊");
        assert!(smithy.has_room_tag("鐵匠鋪"));
        assert!(smithy.has_room_tag("工坊"));
        assert!(smithy.has_room_tag("磨石"));
        assert!(smithy.has_room_tag("礦石"));

        let canteen = GridCell::new(SquareCoord::new(2, 0), Terrain::Tavern, "文曜・三水大食堂");
        assert!(canteen.has_room_tag("食堂"));
        assert!(canteen.has_room_tag("熱食"));
        assert!(canteen.has_room_tag("現煮飲品"));
        assert!(canteen.has_room_tag("櫃檯"));

        let sluice = GridCell::new(SquareCoord::new(3, 0), Terrain::Water, "老堰頭・清泉水脈一號分洪閘");
        assert!(sluice.has_room_tag("水脈"));
        assert!(sluice.has_room_tag("分洪閘"));

        let eff = canteen.effective_tags();
        assert!(eff.contains(&"食堂".to_string()));
        assert!(eff.contains(&"熱食".to_string()));
    }

    #[test]
    fn test_room_venue_object_resolution() {
        let clinic = resolve_room_venue_object("半島百草診所", "").expect("clinic venue");
        assert_eq!(clinic.venue_kind, "醫館藥局");
        assert_eq!(clinic.default_host_title, "當值醫師");
        assert!(clinic.has_counter_restricted_zone);
        assert_eq!(clinic.lendable_tool.unwrap().0, "wt_cat_1057");
        assert!(!clinic.on_duty_demeanor.is_empty());
        assert!(!clinic.on_duty_talk_lines.is_empty());
        assert!(!clinic.counter_trade_offers.is_empty());

        let wood = resolve_room_venue_object("巧木工坊", "").expect("woodwork venue");
        assert_eq!(wood.venue_kind, "木作工坊");
        assert_eq!(wood.default_host_title, "當值木匠");
        assert_eq!(wood.lendable_tool.unwrap().0, "wt_cat_0956");
        assert!(!wood.on_duty_demeanor.is_empty());
        assert!(!wood.on_duty_talk_lines.is_empty());

        let forge = resolve_room_venue_object("西嶺百鍛鐵工坊", "").expect("forge venue");
        assert_eq!(forge.venue_kind, "百鍛鐵坊");
        assert_eq!(forge.default_host_title, "當值鐵匠");
        assert_eq!(forge.lendable_tool.unwrap().0, "wt_cat_0957");
        assert!(!forge.on_duty_demeanor.is_empty());
        assert!(!forge.on_duty_talk_lines.is_empty());

        let canteen_cell = GridCell::new(SquareCoord::new(2, 0), Terrain::Tavern, "文曜・三水大食堂");
        let canteen = canteen_cell.venue_object().expect("canteen venue");
        assert_eq!(canteen.venue_kind, "大食堂");
        assert_eq!(canteen.default_host_title, "食堂主廚");
        assert_eq!(canteen.lendable_tool.unwrap().0, "wt_cat_0897");
        assert!(!canteen.on_duty_demeanor.is_empty());
        assert!(!canteen.on_duty_talk_lines.is_empty());

        let mill = resolve_room_venue_object("老堰頭水輪石磨", "").expect("mill venue");
        assert_eq!(mill.venue_kind, "水輪石磨工坊");
        assert_eq!(mill.default_host_title, "當值石磨匠師");
        assert!(mill.lendable_tool.is_none());
        assert!(!mill.on_duty_demeanor.is_empty());
        assert!(!mill.on_duty_talk_lines.is_empty());

        // 驗證 resolve_market_or_specialty_room_object 解析
        let fish = resolve_market_or_specialty_room_object("早市鮮魚拍賣場", "魚販").expect("fish venue");
        assert_eq!(fish.venue_kind, "鮮魚水產市集");
        assert_eq!(fish.counter_trade_offers.len(), 3);
        assert_eq!(fish.counter_trade_offers[0].0, "wt_cat_0224");
        assert_eq!(fish.on_duty_talk_lines.len(), 3);

        let forge_stall = resolve_market_or_specialty_room_object("重鐵水錘鍛造坊", "").expect("forge stall venue");
        assert_eq!(forge_stall.venue_kind, "大鍛鐵匠鍛造坊");
        assert_eq!(forge_stall.counter_trade_offers[0].0, "wt_cat_0945");
        assert!(forge_stall.on_duty_talk_lines[0].contains("工坊大匠千錘百鍊"));

        let kiln = resolve_market_or_specialty_room_object("高山木炭窯", "").expect("kiln stall venue");
        assert_eq!(kiln.venue_kind, "石材磚瓦窯坊");
        assert!(kiln.on_duty_talk_lines[2].contains("青櫟硬木"));

        let livestock = resolve_market_or_specialty_room_object("車馬獸醫館", "").expect("livestock venue");
        assert_eq!(livestock.venue_kind, "牲畜交易獸醫館");
        assert!(livestock.on_duty_talk_lines[2].contains("乾草料厚磚"));

        let lane = resolve_market_or_specialty_room_object("西門街", "").expect("street venue");
        assert_eq!(lane.venue_kind, "街巷橋樑公共空地");
        assert!(lane.counter_trade_offers.is_empty());
        assert_eq!(lane.on_duty_talk_lines.len(), 3);

        // 驗證 resolve_any_room_venue_object 與 GridCell::venue_object
        let any_clinic = resolve_any_room_venue_object("半島百草診所", "").expect("any clinic");
        assert_eq!(any_clinic.venue_kind, "醫館藥局");
        let any_fish = resolve_any_room_venue_object("早市鮮魚拍賣場", "魚販").expect("any fish");
        assert_eq!(any_fish.venue_kind, "鮮魚水產市集");

        let fish_cell = GridCell::new(SquareCoord::new(10, 10), Terrain::Market, "早市鮮魚拍賣場");
        assert_eq!(fish_cell.venue_object().unwrap().venue_kind, "鮮魚水產市集");
    }

    #[test]
    fn test_find_nearest_room_by_tags_and_new_tags() {
        let mut grid = SquareGrid::new();
        let bath_cell = GridCell::new(SquareCoord::new(2, 2), Terrain::Plain, "溫泉香湯浴場");
        assert!(bath_cell.has_room_tag("浴場"));

        let tea_cell = GridCell::new(SquareCoord::new(0, 1), Terrain::Tavern, "路口奉茶歇舍");
        assert!(tea_cell.has_room_tag("茶屋"));
        assert!(tea_cell.has_room_tag("旅社"));

        let well_cell = GridCell::new(SquareCoord::new(-1, 0), Terrain::Plain, "村口青石水井");
        assert!(well_cell.has_room_tag("水井"));

        let dock_cell = GridCell::new(SquareCoord::new(0, 3), Terrain::Dock, "半島聯運渡口");
        assert!(dock_cell.has_room_tag("聯運"));
        assert!(dock_cell.has_room_tag("碼頭"));

        let bridge_cell = GridCell::new(SquareCoord::new(1, 0), Terrain::Bridge, "中心石橋廣場");
        assert!(bridge_cell.has_room_tag("石橋"));
        assert!(bridge_cell.has_room_tag("廣場"));

        grid.insert(bath_cell);
        grid.insert(tea_cell);
        grid.insert(well_cell);
        grid.insert(dock_cell);
        grid.insert(bridge_cell);

        // 從 (0, 0) 搜尋最近的水井 (曼哈頓距離 1: (-1, 0))
        let nearest_well = grid.find_nearest_room_by_tags((0, 0), 5, &["水井"], 0);
        assert_eq!(nearest_well, Some(((-1, 0), "村口青石水井".to_string())));

        // 從 (0, 0) 搜尋最近的廣場 (曼哈頓距離 1: (1, 0))
        let nearest_square = grid.find_nearest_room_by_tags((0, 0), 5, &["廣場"], 0);
        assert_eq!(nearest_square, Some(((1, 0), "中心石橋廣場".to_string())));

        // 距離超過半徑則為 None
        assert_eq!(grid.find_nearest_room_by_tags((0, 0), 0, &["浴場"], 0), None);
        assert_eq!(
            grid.find_nearest_room_by_tags((0, 0), 5, &["浴場"], 0),
            Some(((2, 2), "溫泉香湯浴場".to_string()))
        );
}

    #[test]
    fn test_resolve_room_ambient_visitor_activity() {
        // 文曜核心場域驗證
        let bridge_act = resolve_room_ambient_visitor_activity("文曜石橋", 0);
        assert_eq!(bridge_act, Some("憑著雕花青石欄杆遠眺三水川往來貨船"));

        let clinic_act = resolve_room_ambient_visitor_activity("文曜草藥診所", 0);
        assert_eq!(clinic_act, Some("在藥櫃前依藥方稱量烘乾的光葉藥草與止血水苔"));

        let council_act = resolve_room_ambient_visitor_activity("文曜公議堂", 1);
        assert_eq!(council_act, Some("站在水土民務告示立木前審視各街坊修繕委託單"));

        // 半島外圍聚落特色場域驗證
        let mill_act = resolve_room_ambient_visitor_activity("大圳頭水輪石磨坊", 0);
        assert_eq!(mill_act, Some("背著一袋新收稻穀在石磨槽旁等候過篩出粉"));

        let lumber_act = resolve_room_ambient_visitor_activity("北山鋸木場", 1);
        assert_eq!(lumber_act, Some("拂去衣袖上的松木鋸屑，清點綑紮妥當的板材"));

        let stone_act = resolve_room_ambient_visitor_activity("山坳切石坊", 0);
        assert_eq!(stone_act, Some("在導水冷卻槽旁端詳切整平滑的青石板材紋理"));

        let pasture_act = resolve_room_ambient_visitor_activity("牛埔乳酪工坊", 0);
        assert_eq!(pasture_act, Some("提著木桶在乳酪壓榨架旁等候新鮮凝乳分裝"));

        let mine_act = resolve_room_ambient_visitor_activity("赤嶺選礦場", 0);
        assert_eq!(mine_act, Some("推著裝滿赤鐵原礦的木輪車沿軌道緩步前行"));

        let dock_act = resolve_room_ambient_visitor_activity("風舶角漁業碼頭", 1);
        assert_eq!(dock_act, Some("站在棧橋邊核對靠岸貨船卸下的海鹽與魚乾"));

        // 預設通用場域
        let default_act = resolve_room_ambient_visitor_activity("尋常林間小道", 0);
        assert_eq!(default_act, Some("提著竹編提籃在三水口商號間採辦日用雜貨"));
    }
}
