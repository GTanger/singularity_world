/// 房間物件化契約（RoomVenueObject）：
/// 將每個房間視為具備自身語意標籤、固定設施狀態、常備外借器物插座與當值職司的一等物件，
/// 避免各子系統（借物、移動視野、事件劇本）各自散落字串猜測。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            tags.push("workshop");
        }
        if k.contains("鐵") || k.contains("鍛") {
            tags.push("blacksmith");
        }
        if k.contains("織") || k.contains("染") {
            tags.push("textile");
        }
        if k.contains("磨") {
            tags.push("grindstone");
        }
        if k.contains("礦") || k.contains("石") {
            tags.push("mine");
        }
        if k.contains("木") || k.contains("林") {
            tags.push("lumber");
        }
        if k.contains("鋪") || k.contains("行") || k.contains("店") || k.contains("棧") || k.contains("商") {
            tags.push("shop");
        }
        if k.contains("市") || k.contains("集") || k.contains("拍賣") {
            tags.push("market");
        }
        if k.contains("食") || k.contains("飯") || k.contains("湯餅") {
            tags.push("canteen");
        }
        if k.contains("茶") {
            tags.push("teahouse");
        }
        if k.contains("旅") || k.contains("客") || k.contains("驛") {
            tags.push("inn");
        }
        if k.contains("浴") || k.contains("溫泉") {
            tags.push("bathhouse");
        }
        tags
    }
}


static ROOM_VENUE_CACHE: std::sync::LazyLock<
    std::sync::RwLock<std::collections::HashMap<(String, String), Option<RoomVenueObject>>>,
> = std::sync::LazyLock::new(|| {
    std::sync::RwLock::new(std::collections::HashMap::with_capacity(2048))
});

static ANY_ROOM_VENUE_CACHE: std::sync::LazyLock<
    std::sync::RwLock<std::collections::HashMap<(String, String), Option<RoomVenueObject>>>,
> = std::sync::LazyLock::new(|| {
    std::sync::RwLock::new(std::collections::HashMap::with_capacity(2048))
});

/// 依據房間名稱與補充提示（如分區），確定性解析房間物件契約（具備 O(1) 執行期記憶化快取）
pub fn resolve_room_venue_object(room_name: &str, extra_hint: &str) -> Option<RoomVenueObject> {
    if is_private_residence_room_name(room_name) || is_street_or_gate_room_name(room_name) {
        return None;
    }
    let key = (room_name.to_string(), extra_hint.to_string());
    if let Ok(guard) = ROOM_VENUE_CACHE.read()
        && let Some(&cached) = guard.get(&key)
    {
        return cached;
    }
    let combined = format!("{room_name} {extra_hint}");
    let room_tags = derive_room_tags(Terrain::Road, room_name, extra_hint);
    let resolved = resolve_room_venue_object_p1(&combined, &room_tags)
        .or_else(|| resolve_room_venue_object_p2(&combined, &room_tags));
    if let Ok(mut guard) = ROOM_VENUE_CACHE.write() {
        if guard.len() < 16_384 {
            guard.insert(key, resolved);
        }
    }
    resolved
}

/// 統一查詢房間一等物件（涵蓋 37 類作坊政務場域與 28 類市集專賣攤位，具備 O(1) 執行期記憶化快取）
pub fn resolve_any_room_venue_object(room_name: &str, hint: &str) -> Option<RoomVenueObject> {
    let key = (room_name.to_string(), hint.to_string());
    if let Ok(guard) = ANY_ROOM_VENUE_CACHE.read()
        && let Some(&cached) = guard.get(&key)
    {
        return cached;
    }
    let resolved = resolve_room_venue_object(room_name, hint)
        .or_else(|| resolve_market_or_specialty_room_object(room_name, hint));
    if let Ok(mut guard) = ANY_ROOM_VENUE_CACHE.write() {
        if guard.len() < 16_384 {
            guard.insert(key, resolved);
        }
    }
    resolved
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
            "背著隨身行囊在附近駐足歇腳，打量四周風物",
            "沿著平整的路道與相熟的街坊點頭寒暄",
        ]
    };
    Some(variants[seed_idx % variants.len()])
}

